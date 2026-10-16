use claims::assert_ok;
use pyroscope::{
    backend::{Report, ReportBatch, ReportData},
    pyroscope::PyroscopeConfig,
    session::{Session, SessionManager, SessionSignal},
};
use std::collections::HashMap;

#[path = "support/push_receiver.rs"]
mod push_receiver;

#[test]
fn test_session_manager_new() {
    let session_manager = SessionManager::new().unwrap();
    assert!(session_manager.handle.is_some());
}

#[test]
fn test_session_manager_push_kill() {
    let session_manager = SessionManager::new().unwrap();
    session_manager.push(SessionSignal::Kill).unwrap();
    assert_ok!(session_manager.handle.unwrap().join().unwrap());
}

#[test]
fn test_session_new() {
    let config = PyroscopeConfig {
        url: "http://localhost:8080".to_string(),
        application_name: "test".to_string(),
        tags: HashMap::new(),
        sample_rate: 100u32,
        spy_name: "test-rs".to_string(),
        ..Default::default()
    };

    let batch = ReportBatch {
        profile_type: "process_cpu".into(),
        data: ReportData::Reports(vec![Report::new(HashMap::new())]),
    };

    let session = Session::new(1950, config, batch).unwrap();

    assert_eq!(session.from, 1940);
    assert_eq!(session.until, 1950);
}

#[test]
fn test_session_send_error() {
    let config = PyroscopeConfig {
        url: "http://invalid_url".to_string(),
        application_name: "test".to_string(),
        tags: HashMap::new(),
        sample_rate: 100u32,
        spy_name: "test-rs".to_string(),
        ..Default::default()
    };

    let batch = ReportBatch {
        profile_type: "process_cpu".into(),
        data: ReportData::Reports(vec![Report::new(HashMap::new())]),
    };

    let _session = Session::new(1950, config, batch).unwrap();
}

#[test]
fn raw_memory_upload_preserves_pprof_auth_tenant_and_agent_labels() {
    use prost::Message;
    use pyroscope::encode::gen::google::{Profile, Sample, ValueType};
    use std::io::{Read, Write};

    for (gzipped_profile, sample_count) in [(false, 1), (true, 1), (false, 32_768), (true, 32_768)]
    {
        let receiver = push_receiver::PushReceiver::start();
        let mut config = PyroscopeConfig::new(
            format!("{}/sdk", receiver.url),
            "heap-service",
            100,
            "test-rs",
            "test-version",
        )
        .basic_auth("test".into(), "secret".into())
        .tenant_id("test-tenant".into())
        .tags(vec![("env", "test"), ("__name__", "ignored")]);
        config
            .http_headers
            .insert("X-Test-Header".into(), "upload-test".into());
        config.func = Some(|_| panic!("raw profile must bypass structured report transforms"));
        let pprof = Profile {
            string_table: [
                "",
                "alloc_objects",
                "count",
                "alloc_space",
                "bytes",
                "inuse_objects",
                "inuse_space",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            sample_type: vec![
                ValueType { r#type: 1, unit: 2 },
                ValueType { r#type: 3, unit: 4 },
                ValueType { r#type: 5, unit: 2 },
                ValueType { r#type: 6, unit: 4 },
            ],
            sample: (0..sample_count)
                .map(|i| Sample {
                    value: vec![i + 2, (i + 1) * 8192, i + 1, (i + 1) * 4096],
                    ..Sample::default()
                })
                .collect(),
            default_sample_type: 6,
            time_nanos: 1_000_000,
            duration_nanos: 10_000,
            ..Profile::default()
        }
        .encode_to_vec();
        let expected = Profile::decode(pprof.as_slice()).unwrap();
        let payload = if gzipped_profile {
            let mut encoder = libflate::gzip::Encoder::new(Vec::new()).unwrap();
            encoder.write_all(&pprof).unwrap();
            encoder.finish().into_result().unwrap()
        } else {
            pprof
        };
        let session = Session::new(
            1950,
            config,
            ReportBatch {
                profile_type: "memory".into(),
                data: ReportData::RawPprof(payload.clone()),
            },
        )
        .unwrap();
        let manager = SessionManager::new().unwrap();
        manager
            .push(SessionSignal::Session(Box::new(session)))
            .unwrap();
        manager.push(SessionSignal::Kill).unwrap();
        manager.handle.unwrap().join().unwrap().unwrap();
        let captured = receiver.finish();
        assert_eq!(captured.path, "/sdk/push.v1.PusherService/Push");
        assert_eq!(captured.headers["content-type"], "application/proto");
        assert_eq!(captured.headers["content-encoding"], "gzip");
        assert_eq!(captured.headers["authorization"], "Basic dGVzdDpzZWNyZXQ=");
        assert_eq!(captured.headers["x-scope-orgid"], "test-tenant");
        assert_eq!(captured.headers["x-test-header"], "upload-test");
        assert!(captured.headers["user-agent"].contains("test-rs/test-version"));
        let series = &captured.request.series[0];
        let labels: HashMap<_, _> = series
            .labels
            .iter()
            .map(|label| (label.name.as_str(), label.value.as_str()))
            .collect();
        assert_eq!(labels["__name__"], "memory");
        assert_eq!(labels["service_name"], "heap-service");
        assert_eq!(labels["env"], "test");
        assert_eq!(labels["process.runtime.name"], "rust");
        assert_eq!(series.samples[0].raw_profile, payload);
        assert!(!series.samples[0].id.is_empty());
        let restored = if gzipped_profile {
            let mut decoded = Vec::new();
            libflate::gzip::Decoder::new(payload.as_slice())
                .unwrap()
                .read_to_end(&mut decoded)
                .unwrap();
            decoded
        } else {
            payload
        };
        assert_eq!(Profile::decode(restored.as_slice()).unwrap(), expected);
    }
}

#[test]
fn structured_upload_preserves_transform_samples_tags_and_timestamps() {
    use prost::Message;
    use pyroscope::{
        backend::{StackFrame, StackTrace, Tag},
        encode::gen::google::Profile,
    };

    for transform in [false, true] {
        let receiver = push_receiver::PushReceiver::start();
        let mut config = PyroscopeConfig::new(&receiver.url, "cpu-service", 100, "test-rs", "1");
        if transform {
            config.func = Some(|mut report| {
                for count in report.data.values_mut() {
                    *count *= 3;
                }
                report
                    .metadata
                    .add_tag(Tag::new("transformed".into(), "true".into()));
                report
            });
        }
        let mut report = Report::default();
        report.record_with_count(
            StackTrace {
                frames: vec![StackFrame {
                    name: Some("cpu_work".into()),
                    filename: Some("work.rs".into()),
                    line: Some(42),
                    ..StackFrame::default()
                }],
                ..StackTrace::default()
            },
            2,
        );
        let session = Session::new(
            1950,
            config,
            ReportBatch {
                profile_type: "process_cpu".into(),
                data: ReportData::Reports(vec![report]),
            },
        )
        .unwrap();
        let manager = SessionManager::new().unwrap();
        manager
            .push(SessionSignal::Session(Box::new(session)))
            .unwrap();
        manager.push(SessionSignal::Kill).unwrap();
        manager.handle.unwrap().join().unwrap().unwrap();
        let captured = receiver.finish();
        let series = &captured.request.series[0];
        assert!(series
            .labels
            .iter()
            .any(|label| { label.name == "__name__" && label.value == "process_cpu" }));
        let profile = Profile::decode(series.samples[0].raw_profile.as_slice()).unwrap();
        assert_eq!(profile.time_nanos, 1940 * 1_000_000_000);
        assert_eq!(profile.duration_nanos, 10 * 1_000_000_000);
        assert_eq!(profile.period, 10_000_000);
        assert_eq!(profile.sample.len(), 1);
        assert_eq!(
            profile.sample[0].value,
            vec![if transform { 60_000_000 } else { 20_000_000 }]
        );
        assert_eq!(profile.sample[0].label.len(), usize::from(transform));
        if transform {
            let label = &profile.sample[0].label[0];
            assert_eq!(profile.string_table[label.key as usize], "transformed");
            assert_eq!(profile.string_table[label.str as usize], "true");
        }
        let function = &profile.function[0];
        assert_eq!(profile.string_table[function.name as usize], "cpu_work");
        assert_eq!(profile.string_table[function.filename as usize], "work.rs");
        assert_eq!(profile.location[0].line[0].line, 42);
    }
}
