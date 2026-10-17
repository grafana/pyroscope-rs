#[cfg(feature = "backend-mimalloc")]
#[path = "support/push_receiver.rs"]
mod push_receiver;

#[cfg(feature = "backend-mimalloc")]
mod tests {
    use pyroscope::backend::mimalloc::{
        mimalloc_backend, mimalloc_stats, MimallocConfig, SamplingMiMalloc,
    };
    use pyroscope::backend::{ReportData, Tag, ThreadTag};
    use std::sync::Mutex;

    #[global_allocator]
    static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();
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
