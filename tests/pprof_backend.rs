#[cfg(feature = "backend-pprof-rs")]
mod tests {
    use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn next_size(hasher: &mut DefaultHasher) -> usize {
        Instant::now().hash(hasher);
        (hasher.finish() as usize) % (256 * 1024) + 1
    }

    // NOTE: This test is not reliable — it only exercises the initialize/report
    // lifecycle against a live workload and makes no assertions on the output.
    // No failures have been observed so far; kept as a smoke test for the future.
    #[test]
    fn test_pprof_backend_alloc_loop() {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();

        // Start the alloc loop slightly before the profiler initializes
        let alloc_thread = std::thread::spawn(move || {
            let mut hasher = DefaultHasher::new();
            while !stop_thread.load(Ordering::Relaxed) {
                // Vary allocation size between 1 B and 256 KiB
                let size = next_size(&mut hasher);
                let v: Vec<u8> = vec![0u8; size];
                drop(v);
            }
        });

        // Brief pause so the alloc thread is running before profiling starts
        std::thread::sleep(Duration::from_millis(50));

        let mut backend = pprof_backend(PprofConfig::default(), BackendConfig::default())
            .initialize()
            .expect("failed to initialize pprof backend");

        std::thread::sleep(Duration::from_secs(5));

        let reports = backend.report().expect("failed to dump report");
        drop(reports);

        stop.store(true, Ordering::Relaxed);
        alloc_thread.join().expect("alloc thread panicked");
    }

    #[cfg(target_os = "linux")]
    mod unlinked {
        use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig, ReportData};
        use std::time::{Duration, Instant};

        #[inline(never)]
        fn hot_loop(duration: Duration) -> u64 {
            let start = Instant::now();
            let mut x = 1u64;
            while start.elapsed() < duration {
                for i in 0..10_000u64 {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(i);
                }
                std::hint::black_box(x);
            }
            x
        }

        fn worker(mode: &str) {
            let exe = std::env::current_exe().unwrap();
            if mode == "before-init" {
                std::fs::remove_file(&exe).unwrap();
            }
            let mut backend = pprof_backend(PprofConfig::default(), BackendConfig::default())
                .initialize()
                .expect("failed to initialize pprof backend");
            if mode == "after-init" {
                std::fs::remove_file(&exe).unwrap();
            }
            hot_loop(Duration::from_secs(1));

            let batch = backend.report().expect("failed to dump report");
            let reports = match batch.data {
                ReportData::Reports(reports) => reports,
                ReportData::RawPprof(_) => panic!("pprof backend returned raw pprof"),
            };
            let samples: usize = reports
                .iter()
                .flat_map(|report| report.data.iter())
                .filter(|(trace, _)| {
                    trace
                        .frames
                        .iter()
                        .any(|frame| frame.name.as_deref().unwrap_or("").contains("hot_loop"))
                })
                .map(|(_, count)| count)
                .sum();
            println!("hot_loop_samples={samples}");
        }

        // The worker is a copy of this test binary that deletes itself, so the
        // build artifacts stay in place.
        #[test]
        fn test_pprof_backend_symbolizes_unlinked_binary() {
            const MODE: &str = "PYROSCOPE_TEST_UNLINK_MODE";
            if let Ok(mode) = std::env::var(MODE) {
                worker(&mode);
                std::process::exit(0);
            }

            for mode in ["before-init", "after-init"] {
                let dir = tempfile::tempdir().unwrap();
                let worker = dir.path().join("worker");
                std::fs::copy(std::env::current_exe().unwrap(), &worker).unwrap();
                let output = std::process::Command::new(&worker)
                    .env(MODE, mode)
                    .args([
                        "--exact",
                        "tests::unlinked::test_pprof_backend_symbolizes_unlinked_binary",
                        "--nocapture",
                        "--test-threads",
                        "1",
                    ])
                    .output()
                    .unwrap();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    output.status.success(),
                    "{mode}: worker failed\n{stdout}\n{stderr}"
                );
                assert!(!worker.exists(), "{mode}: worker did not unlink itself");
                let samples: usize = stdout
                    .lines()
                    .find_map(|line| line.split("hot_loop_samples=").nth(1))
                    .unwrap_or_else(|| {
                        panic!("{mode}: worker printed no summary\n{stdout}\n{stderr}")
                    })
                    .trim()
                    .parse()
                    .unwrap();
                assert!(
                    samples > 0,
                    "{mode}: no sample symbolized to hot_loop\n{stdout}"
                );
            }
        }
    }
}
