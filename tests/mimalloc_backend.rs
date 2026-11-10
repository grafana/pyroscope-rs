#[cfg(feature = "backend-mimalloc")]
#[path = "support/push_receiver.rs"]
mod push_receiver;

#[cfg(feature = "backend-mimalloc")]
mod tests {
    use pyroscope::backend::mimalloc::{mimalloc_backend, MimallocBackendConfig};
    use pyroscope::backend::{ReportData, Tag, ThreadTag};
    use rustfs_mimalloc_profiling::{mimalloc_stats, MimallocConfig, SamplingAllocator};
    use std::sync::Mutex;

    #[global_allocator]
    static ALLOC: SamplingAllocator<mimalloc::MiMalloc> =
        SamplingAllocator::with_allocator(mimalloc::MiMalloc);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn adapter_and_standalone_profiler_share_one_recorder() {
        use rustfs_mimalloc_profiling::{Error, Profiler};
        let _guard = TEST_LOCK.lock().unwrap();
        let backend = live_backend();
        let tag = ThreadTag::new(
            pyroscope::ThreadId::pthread_self(),
            Tag::new("request".into(), "test".into()),
        );
        backend.add_tag(tag.clone()).unwrap();
        backend.remove_tag(tag).unwrap();
        // Exposing both factories does not install a second global allocator
        // or acquire another recording session.
        #[cfg(feature = "backend-jemalloc")]
        drop(pyroscope::backend::jemalloc_backend());
        assert!(matches!(
            Profiler::start(MimallocConfig::default()),
            Err(Error::AlreadyActive)
        ));
        drop(backend);
        let standalone = Profiler::start(MimallocConfig::default()).unwrap();
        let Err(error) = mimalloc_backend(MimallocConfig::default()).initialize() else {
            panic!("adapter must reject an active standalone profiler");
        };
        assert_eq!(
            std::error::Error::source(&error).and_then(|source| source.downcast_ref()),
            Some(&Error::AlreadyActive)
        );
        drop(standalone);
        let mut restarted = live_backend();
        assert!(matches!(
            restarted.report().unwrap().data,
            ReportData::RawPprof(_)
        ));
        restarted.shutdown().unwrap();
        // Explicit shutdown also releases ownership without waiting for a
        // later backend to be dropped by the test harness.
        let _standalone = Profiler::start(MimallocConfig::default()).unwrap();
    }

    #[test]
    fn adapter_resets_report_stats_when_restarted() {
        let _guard = TEST_LOCK.lock().expect("lock stats test");
        // Seed a prior session with more reports than the next one so both
        // counter resets are checked without relying on test execution order.
        for report_count in [2, 1] {
            let mut backend = live_backend();
            let initial_stats = mimalloc_stats();
            assert_eq!(initial_stats.reports, 0);
            assert_eq!(initial_stats.reported_samples, 0);

            let retained = std::hint::black_box(vec![0_u8; 2 * 1024 * 1024]);
            for _ in 0..report_count {
                backend.report().expect("collect heap profile");
            }
            let report_stats = mimalloc_stats();
            assert_eq!(report_stats.reports, report_count);
            assert!(report_stats.reported_samples > 0);
            backend.shutdown().expect("shut down profiling session");
            std::hint::black_box(&retained);
        }
    }

    #[test]
    fn adapter_uploads_the_profiler_payload_without_reencoding() {
        use pyroscope::{
            pyroscope::PyroscopeConfig,
            session::{Session, SessionManager, SessionSignal},
        };
        let _guard = TEST_LOCK.lock().expect("lock upload test");
        let mut backend = live_backend();
        let retained = std::hint::black_box(vec![0_u8; 2 * 1024 * 1024]);
        let batch = backend.report().expect("collect live heap profile");
        let report_stats = mimalloc_stats();
        // Counters are reset when the session starts, even after earlier tests.
        assert_eq!(report_stats.reports, 1);
        assert!(report_stats.reported_samples > 0);
        let ReportData::RawPprof(ref bytes) = batch.data else {
            panic!("raw memory profile expected");
        };
        let expected_bytes = bytes.clone();
        backend.shutdown().unwrap();

        let receiver = super::push_receiver::PushReceiver::start();
        let config = PyroscopeConfig::new(
            &receiver.url,
            "mimalloc-upload",
            100,
            "pyroscope-rs",
            env!("CARGO_PKG_VERSION"),
        );
        let session = Session::new(1950, config, batch).unwrap();
        let manager = SessionManager::new().unwrap();
        manager
            .push(SessionSignal::Session(Box::new(session)))
            .unwrap();
        manager.push(SessionSignal::Kill).unwrap();
        manager.handle.unwrap().join().unwrap().unwrap();
        let captured = receiver.finish();
        assert_eq!(captured.path, "/push.v1.PusherService/Push");
        assert_eq!(captured.headers["content-encoding"], "gzip");
        let series = &captured.request.series[0];
        assert!(series
            .labels
            .iter()
            .any(|label| label.name == "__name__" && label.value == "memory"));
        assert_eq!(series.samples[0].raw_profile, expected_bytes);
        std::hint::black_box(&retained);
    }

    #[test]
    fn default_queue_preserves_early_and_late_allocation_sites() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut backend = mimalloc_backend(MimallocBackendConfig::default())
            .initialize()
            .unwrap();
        pressure_early_site();
        pressure_late_site();
        let report = backend.report().unwrap();
        assert_eq!(mimalloc_stats().dropped_samples, 0);
        let ReportData::RawPprof(bytes) = report.data else {
            panic!("expected raw pprof")
        };
        for marker in ["pressure_early_site", "pressure_late_site"] {
            let bytes = allocation_bytes_for_marker(&bytes, marker);
            // Each site allocates 2 GiB. Allow Poisson sampling variance but
            // reject severe loss or the disappearance of the later site.
            assert!(
                (1536 * 1024 * 1024..2560 * 1024 * 1024).contains(&bytes),
                "{marker}: {bytes}"
            );
        }
        backend.shutdown().unwrap();
    }

    #[test]
    fn standalone_defaults_reproduce_the_lost_late_site() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut backend = mimalloc_backend(MimallocConfig::default())
            .initialize()
            .unwrap();
        pressure_early_site();
        pressure_late_site();
        let report = backend.report().unwrap();
        assert!(mimalloc_stats().dropped_samples > 0);
        let ReportData::RawPprof(bytes) = report.data else {
            panic!("expected raw pprof")
        };
        assert_eq!(allocation_bytes_for_marker(&bytes, "pressure_late_site"), 0);
        assert!(allocation_bytes_for_marker(&bytes, "pressure_early_site") < 1024 * 1024 * 1024);
        backend.shutdown().unwrap();
    }

    struct WarningLog(std::sync::Mutex<Vec<String>>);
    static WARNINGS: WarningLog = WarningLog(std::sync::Mutex::new(Vec::new()));
    impl log::Log for WarningLog {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() == log::Level::Warn
        }
        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) && record.target() == "pyroscope::backend::mimalloc"
            {
                self.0.lock().unwrap().push(record.args().to_string());
            }
        }
        fn flush(&self) {}
    }

    #[test]
    fn explicit_small_queue_reports_loss_and_warns_each_affected_interval() {
        let _guard = TEST_LOCK.lock().unwrap();
        log::set_logger(&WARNINGS).unwrap();
        log::set_max_level(log::LevelFilter::Warn);
        WARNINGS.0.lock().unwrap().clear();
        let mut backend = mimalloc_backend(MimallocConfig {
            ring_capacity: 8,
            ..MimallocConfig::default()
        })
        .initialize()
        .unwrap();
        for expected_warnings in 1..=2 {
            pressure_early_site();
            pressure_late_site();
            backend.report().unwrap();
            assert!(mimalloc_stats().dropped_samples > 0);
            let logs = WARNINGS.0.lock().unwrap();
            assert_eq!(logs.len(), expected_warnings);
            assert!(logs.last().unwrap().contains("Incomplete mimalloc profile"));
        }
        backend.report().unwrap();
        assert_eq!(
            WARNINGS.0.lock().unwrap().len(),
            2,
            "old losses must not be logged again"
        );
        backend.shutdown().unwrap();
        // A new session must not reuse the previous session's loss baseline.
        let mut backend = mimalloc_backend(MimallocConfig {
            ring_capacity: 8,
            ..MimallocConfig::default()
        })
        .initialize()
        .unwrap();
        pressure_late_site();
        backend.report().unwrap();
        assert_eq!(WARNINGS.0.lock().unwrap().len(), 3);
        backend.shutdown().unwrap();
    }

    #[inline(never)]
    fn pressure_early_site() {
        for _ in 0..2048 {
            std::hint::black_box(vec![0_u8; 1024 * 1024]);
        }
    }
    #[inline(never)]
    fn pressure_late_site() {
        for _ in 0..2048 {
            std::hint::black_box(vec![0_u8; 1024 * 1024]);
        }
    }

    fn allocation_bytes_for_marker(bytes: &[u8], marker: &str) -> i64 {
        use prost::Message;
        use pyroscope::encode::gen::google::Profile;
        let profile = Profile::decode(bytes).unwrap();
        let index = profile
            .sample_type
            .iter()
            .position(|kind| profile.string_table[kind.r#type as usize] == "alloc_space")
            .unwrap();
        let functions: std::collections::HashSet<_> = profile
            .function
            .iter()
            .filter(|f| profile.string_table[f.name as usize].contains(marker))
            .map(|f| f.id)
            .collect();
        let locations: std::collections::HashSet<_> = profile
            .location
            .iter()
            .filter(|l| {
                l.line
                    .iter()
                    .any(|line| functions.contains(&line.function_id))
            })
            .map(|l| l.id)
            .collect();
        profile
            .sample
            .iter()
            .filter(|sample| sample.location_id.iter().any(|id| locations.contains(id)))
            .map(|sample| sample.value[index])
            .sum()
    }

    fn live_backend() -> pyroscope::backend::BackendImpl<pyroscope::backend::BackendReady> {
        mimalloc_backend(MimallocConfig {
            sample_interval_bytes: 4096,
            live_heap_tracking: true,
            max_live_samples: 4096,
            ring_capacity: 4096,
            ..MimallocConfig::default()
        })
        .initialize()
        .expect("initialize live heap backend")
    }
}
