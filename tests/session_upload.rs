use std::{collections::HashMap, io::Read};

use prost::Message;
use pyroscope::{
    backend::{ReportBatch, ReportData, Tag},
    encode::gen::google::Profile,
    pyroscope::PyroscopeConfig,
    session::{Session, SessionManager, SessionSignal},
};

#[path = "support/profile_fixture.rs"]
mod profile_fixture;
#[path = "support/push_receiver.rs"]
mod push_receiver;

#[test]
fn raw_pprof_upload_preserves_bytes_and_http_contract() {
    for fixture in profile_fixture::profiles() {
        let receiver = push_receiver::PushReceiver::start();
        let mut config = PyroscopeConfig::new(
            format!("{}/sdk", receiver.url),
            "upload-test",
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
        config.func = Some(|_| panic!("raw pprof must bypass structured transforms"));
        let expected = fixture.bytes;
        let session = Session::new(
            1950,
            config,
            ReportBatch {
                profile_type: fixture.profile_type.into(),
                data: ReportData::RawPprof(expected.clone()),
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
        assert_eq!(captured.request.series.len(), 1);
        let series = &captured.request.series[0];
        let labels: HashMap<_, _> = series
            .labels
            .iter()
            .map(|l| (l.name.as_str(), l.value.as_str()))
            .collect();
        assert_eq!(labels["__name__"], fixture.profile_type);
        assert_eq!(labels["service_name"], "upload-test");
        assert_eq!(labels["env"], "test");
        assert_eq!(series.samples.len(), 1);
        assert_eq!(
            series.samples[0].raw_profile, expected,
            "fixture {}",
            fixture.name
        );
        assert!(!series.samples[0].id.is_empty());
        let decoded = if fixture.name == "memory-precompressed" {
            let mut bytes = Vec::new();
            libflate::gzip::Decoder::new(expected.as_slice())
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            bytes
        } else {
            expected
        };
        let profile = Profile::decode(decoded.as_slice()).unwrap();
        assert_eq!(profile.string_table[0], "");
        if profile.default_sample_type != 0 {
            assert!(profile
                .sample_type
                .iter()
                .any(|ty| ty.r#type == profile.default_sample_type));
        }
    }
}

#[test]
fn structured_upload_preserves_transform_counts_tags_and_metadata() {
    for transformed in [false, true] {
        let receiver = push_receiver::PushReceiver::start();
        let mut config = PyroscopeConfig::new(&receiver.url, "cpu-test", 100, "test-rs", "1");
        if transformed {
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
        let reports = profile_fixture::structured_reports(64);
        let count: usize = reports.iter().flat_map(|report| report.data.values()).sum();
        let session = Session::new(
            1950,
            config,
            ReportBatch {
                profile_type: "process_cpu".into(),
                data: ReportData::Reports(reports),
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
        let p =
            Profile::decode(captured.request.series[0].samples[0].raw_profile.as_slice()).unwrap();
        assert_eq!(p.sample.len(), 64);
        assert_eq!(p.function.len(), 64 * 16);
        assert_eq!(p.location.len(), 64 * 16);
        assert_eq!(p.time_nanos, 1940 * 1_000_000_000);
        assert_eq!(p.duration_nanos, 10 * 1_000_000_000);
        assert_eq!(p.period, 10_000_000);
        assert_eq!(
            p.sample.iter().map(|s| s.value[0]).sum::<i64>(),
            count as i64 * p.period * if transformed { 3 } else { 1 }
        );
        for sample in &p.sample {
            assert_eq!(sample.location_id.len(), 16);
            assert_eq!(sample.label.len(), usize::from(transformed));
            if transformed {
                let label = &sample.label[0];
                assert_eq!(p.string_table[label.key as usize], "transformed");
                assert_eq!(p.string_table[label.str as usize], "true");
            }
        }
        assert!(p
            .function
            .iter()
            .all(|f| p.string_table[f.filename as usize] == "src/workload.rs"));
        assert!(p.location.iter().all(|l| l.line[0].line >= 42));
    }
}
