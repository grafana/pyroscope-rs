#[cfg(feature = "backend-mimalloc")]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier, Mutex,
    };

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

    #[test]
    fn mimalloc_backend_reports_while_worker_threads_are_allocating() {
        let _guard = TEST_LOCK.lock().expect("lock mimalloc backend test");
        let mut backend = mimalloc_backend(MimallocConfig {
            sample_interval_bytes: 1024,
            ring_capacity: 65_536,
            report_drain_limit: 65_536,
            ..MimallocConfig::default()
        })
        .initialize()
        .expect("initialize mimalloc backend");

        let worker_count = 4;
        let start = Arc::new(Barrier::new(worker_count + 1));
        let stop = Arc::new(AtomicBool::new(false));
        let workers: Vec<_> = (0..worker_count)
            .map(|worker| {
                let start = Arc::clone(&start);
                let stop = Arc::clone(&stop);
                std::thread::spawn(move || {
                    start.wait();
                    let mut rounds = 0;
                    while !stop.load(Ordering::Acquire) || rounds < 64 {
                        let allocations: Vec<Vec<u8>> = (0..128)
                            .map(|iteration| {
                                vec![worker as u8; 256 + ((rounds + iteration) % 8) * 64]
                            })
                            .collect();
                        std::hint::black_box(&allocations);
                        rounds += 1;
                        if rounds >= 512 {
                            break;
                        }
                    }
                })
            })
            .collect();

        start.wait();
        let live_profiles: Vec<_> = (0..3).map(|_| report_profile(&mut backend)).collect();
        stop.store(true, Ordering::Release);
        for worker in workers {
            worker.join().expect("join allocation worker");
        }

        let final_profile = report_profile(&mut backend);
        let stats = mimalloc_stats();
        assert!(stats.recorded_samples > 0);
        assert!(stats.flushes > 0);
        assert!(
            live_profiles.iter().any(profile_has_alloc_space_sample)
                || profile_has_alloc_space_sample(&final_profile)
        );
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

    fn profile_has_alloc_space_sample(profile: &Profile) -> bool {
        profile
            .sample
            .iter()
            .any(|sample| matches!(sample.value.get(1), Some(value) if *value > 0))
    }
}
