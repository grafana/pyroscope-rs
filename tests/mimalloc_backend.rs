#[cfg(feature = "backend-mimalloc")]
mod tests {
    use std::sync::Mutex;

    use prost::Message;
    use pyroscope::backend::mimalloc::{
        mimalloc_backend, mimalloc_stats, MimallocConfig, SamplingMiMalloc,
    };
    use pyroscope::backend::ReportData;
    use pyroscope::encode::gen::google::Profile;

    #[global_allocator]
    static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn mimalloc_backend_reports_raw_memory_pprof() {
        let _guard = TEST_LOCK.lock().expect("lock mimalloc backend test");
        let mut backend = mimalloc_backend(MimallocConfig {
            sample_interval_bytes: 1024,
            ..MimallocConfig::default()
        })
        .initialize()
        .expect("initialize mimalloc backend");

        let allocations: Vec<Vec<u8>> = (0..4096).map(|_| vec![0_u8; 1024]).collect();
        std::hint::black_box(&allocations);

        let profile = report_profile(&mut backend);
        assert!(profile.string_table.iter().any(|s| s == "alloc_space"));
        backend.shutdown().expect("shutdown mimalloc backend");
    }

    #[test]
    fn mimalloc_backend_reports_multithreaded_allocation_churn() {
        let _guard = TEST_LOCK.lock().expect("lock mimalloc backend test");
        let mut backend = mimalloc_backend(MimallocConfig {
            sample_interval_bytes: 4096,
            ring_capacity: 16_384,
            report_drain_limit: 16_384,
            ..MimallocConfig::default()
        })
        .initialize()
        .expect("initialize mimalloc backend");

        let workers: Vec<_> = (0..4)
            .map(|worker| {
                std::thread::spawn(move || {
                    let allocations: Vec<Vec<u8>> = (0..512)
                        .map(|iteration| vec![worker as u8; 512 + (iteration % 4) * 128])
                        .collect();
                    std::hint::black_box(&allocations);
                })
            })
            .collect();
        for worker in workers {
            worker.join().expect("join allocation worker");
        }

        let stats = mimalloc_stats();
        assert!(stats.recorded_samples > 0);
        assert!(stats.flushes > 0);
        assert!(stats.flushed_samples > 0);

        let profile = report_profile(&mut backend);
        assert!(!profile.sample.is_empty());
        assert!(profile
            .sample
            .iter()
            .any(|sample| matches!(sample.value.get(1), Some(value) if *value > 0)));
        backend.shutdown().expect("shutdown mimalloc backend");
    }

    fn report_profile(
        backend: &mut pyroscope::backend::BackendImpl<pyroscope::backend::BackendReady>,
    ) -> Profile {
        let batch = backend.report().expect("report memory profile");
        assert_eq!(batch.profile_type, "memory");

        let ReportData::RawPprof(bytes) = batch.data else {
            panic!("expected raw pprof memory profile");
        };
        Profile::decode(bytes.as_slice()).expect("decode memory pprof")
    }
}
