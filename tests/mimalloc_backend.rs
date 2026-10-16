#[cfg(feature = "backend-mimalloc")]
#[path = "support/push_receiver.rs"]
mod push_receiver;

#[cfg(feature = "backend-mimalloc")]
mod tests {
    use prost::Message;
    use pyroscope::backend::mimalloc::{
        mimalloc_backend, mimalloc_stats, MimallocConfig, SamplingMiMalloc,
    };
    use pyroscope::backend::{ReportData, Tag, ThreadTag};
    use pyroscope::encode::gen::google::Profile;
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
        assert!(mimalloc_backend(MimallocConfig::default())
            .initialize()
            .is_err());
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
    fn live_heap_report_preserves_source_metadata_through_http_upload() {
        use pyroscope::{
            pyroscope::PyroscopeConfig,
            session::{Session, SessionManager, SessionSignal},
        };
        let _guard = TEST_LOCK.lock().expect("lock upload test");
        let mut backend = live_backend();
        let retained = allocate_retained_live_test(2 * 1024 * 1024);
        let batch = backend.report().expect("collect live heap profile");
        let report_stats = mimalloc_stats();
        assert_eq!(report_stats.reports, 1);
        assert!(report_stats.reported_samples > 0);
        let ReportData::RawPprof(ref bytes) = batch.data else {
            panic!("raw memory profile expected");
        };
        let expected_bytes = bytes.clone();
        let expected = Profile::decode(bytes.as_slice()).unwrap();
        let function = expected
            .function
            .iter()
            .find(|function| {
                expected.string_table[function.name as usize]
                    .contains("allocate_retained_live_test")
            })
            .expect("retained allocation symbol");
        assert!(expected.string_table[function.filename as usize].ends_with("mimalloc_backend.rs"));
        assert!(!expected.string_table[function.system_name as usize].is_empty());
        let profiler_frames: Vec<_> = expected
            .function
            .iter()
            .map(|function| &expected.string_table[function.name as usize])
            .filter(|name| {
                let owner = name.strip_prefix('<').unwrap_or(name);
                owner.starts_with("rustfs_mimalloc_profiling::")
                    || owner.starts_with("pyroscope::backend::mimalloc::")
                    || owner.starts_with("backtrace::")
                    || name.contains("rustfs_mimalloc_profiling::profiler::SamplerState>")
                    || name
                        .contains("rustfs_mimalloc_profiling::profiler::RegisteredTlsSampleBuffer>")
                    || name.contains("pyroscope[")
            })
            .collect();
        assert!(
            profiler_frames.is_empty(),
            "unexpected profiler frames: {profiler_frames:?}"
        );
        assert!(expected.location.iter().any(|location| {
            location.address != 0
                && location
                    .line
                    .iter()
                    .any(|line| line.function_id == function.id && line.line > 0)
        }));
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
        {
            let location = expected
                .location
                .iter()
                .find(|location| {
                    location
                        .line
                        .iter()
                        .any(|line| line.function_id == function.id)
                })
                .unwrap();
            let mapping = expected
                .mapping
                .iter()
                .find(|mapping| mapping.id == location.mapping_id)
                .expect("retained allocation executable mapping");
            assert_eq!(mapping.id, expected.mapping[0].id);
            assert!(
                mapping.memory_start <= location.address && location.address < mapping.memory_limit
            );
            assert!(!expected.string_table[mapping.filename as usize].is_empty());
            assert!(!expected.string_table[mapping.build_id as usize].is_empty());
        }
        assert_eq!(
            sample_value_for_frame(&expected, "allocate_retained_live_test", "inuse_space"),
            retained.len() as i64
        );
        backend.shutdown().unwrap();

        let receiver = super::push_receiver::PushReceiver::start();
        let config = PyroscopeConfig::new(
            &receiver.url,
            "mimalloc-upload",
            100,
            "pyroscope-rs",
            env!("CARGO_PKG_VERSION"),
        )
        .tags(vec![("env", "integration")]);
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
        assert_eq!(
            Profile::decode(series.samples[0].raw_profile.as_slice()).unwrap(),
            expected
        );
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

    #[inline(never)]
    fn allocate_retained_live_test(size: usize) -> Vec<u8> {
        std::hint::black_box(vec![0; size])
    }

    fn sample_value_for_frame(profile: &Profile, frame: &str, sample_type: &str) -> i64 {
        let value_index = profile
            .sample_type
            .iter()
            .position(|ty| profile.string_table[ty.r#type as usize] == sample_type)
            .expect("profile includes requested sample type");
        profile
            .sample
            .iter()
            .filter(|sample| {
                sample.location_id.iter().any(|id| {
                    profile
                        .location
                        .iter()
                        .filter(|location| location.id == *id)
                        .any(|location| {
                            location.line.iter().any(|line| {
                                profile
                                    .function
                                    .iter()
                                    .filter(|function| function.id == line.function_id)
                                    .any(|function| {
                                        profile.string_table[function.name as usize].contains(frame)
                                    })
                            })
                        })
                })
            })
            .map(|sample| sample.value[value_index])
            .sum()
    }
}
