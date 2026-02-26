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
        while Instant::now() < deadline {
            let v: Vec<u8> = vec![0u8; 1024 * 64];
            drop(v);
        }

        let reports = backend.report().expect("failed to dump report");
        drop(reports);
    }
}
