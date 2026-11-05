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
        drop(backend);
        check_individual_samples();
    }

    // Run after the smoke test: pprof owns one process-wide profiler.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn check_individual_samples() {
        use pyroscope::backend::ReportData;
        for limit in [0, 2] {
            let mut backend = pprof_backend(
                PprofConfig::default()
                    .retain_individual_samples()
                    .sample_limit(limit),
                BackendConfig::default(),
            )
            .initialize()
            .unwrap();
            for _ in 0..2 {
                let start = std::time::Instant::now();
                while start.elapsed() < std::time::Duration::from_millis(250) {
                    std::hint::black_box((0..1000).sum::<u64>());
                }
                let batch = backend.report().unwrap();
                let ReportData::Reports(reports) = batch.data else {
                    panic!("expected reports")
                };
                let samples: Vec<_> = reports.iter().flat_map(|r| r.data.iter()).collect();
                assert_eq!(samples.len(), limit);
                for (stack, count) in samples {
                    assert_eq!(*count, 1);
                    let observation = stack.observation.as_ref().unwrap();
                    assert!(observation.trace_timestamp_ns > 0);
                    #[cfg(target_os = "linux")]
                    assert_eq!(observation.os_thread_id, unsafe {
                        libc::syscall(libc::SYS_gettid) as u64
                    });
                    #[cfg(target_os = "macos")]
                    assert!(observation.os_thread_id > 0);
                }
            }
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn check_individual_samples() {}
}
