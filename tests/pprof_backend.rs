#[cfg(feature = "backend-pprof-rs")]
mod tests {
    use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
    use std::time::{Duration, Instant};

    #[test]
    fn test_pprof_backend_alloc_loop() {
        let mut backend = pprof_backend(PprofConfig::default(), BackendConfig::default())
            .initialize()
            .expect("failed to initialize pprof backend");

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seed: u64 = 0xdeadbeef_cafebabe;
        while Instant::now() < deadline {
            // LCG to vary allocation size between 1 B and 256 KiB
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let size = (seed >> 48) as usize % (256 * 1024) + 1;
            let v: Vec<u8> = vec![0u8; size];
            drop(v);
        }

        let reports = backend.report().expect("failed to dump report");
        drop(reports);
    }
}
