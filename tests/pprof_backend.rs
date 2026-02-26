#[cfg(feature = "backend-pprof-rs")]
mod tests {
    use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_pprof_backend_alloc_loop() {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();

        // Start the alloc loop slightly before the profiler initializes
        let alloc_thread = std::thread::spawn(move || {
            let mut seed: u64 = 0xdeadbeef_cafebabe;
            while !stop_thread.load(Ordering::Relaxed) {
                // LCG to vary allocation size between 1 B and 256 KiB
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let size = (seed >> 48) as usize % (256 * 1024) + 1;
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
}
