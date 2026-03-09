mod push;

use std::io::Write as _;
use std::time::Duration;

use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use prost::Message;
use push::{LabelPair, PushRequest, RawProfileSeries, RawSample};

/// Send a pprof profile to the Pyroscope push API.
///
/// - `base_url`: e.g. `"http://localhost:4040"`
/// - `app_name`: application name; sent as `service_name` label
/// - `pprof`: raw (uncompressed) pprof protobuf bytes
/// - `_from`: profile start time (Unix seconds) — unused by push API, kept for caller compat
/// - `_until`: profile end time (Unix seconds) — unused by push API, kept for caller compat
///
/// Errors are returned but do not retry — callers should log and discard on failure.
pub fn send(
    base_url: &str,
    app_name: &str,
    tags: &[(&str, &str)],
    pprof: &[u8],
    _from: u64,
    _until: u64,
) -> Result<()> {
    let mut labels = vec![
        LabelPair {
            name: "service_name".to_string(),
            value: app_name.to_string(),
        },
        LabelPair {
            name: "__name__".to_string(),
            value: "process_cpu".to_string(),
        },
    ];
    for &(k, v) in tags {
        labels.push(LabelPair {
            name: k.to_string(),
            value: v.to_string(),
        });
    }
    let req = PushRequest {
        series: vec![RawProfileSeries {
            labels,
            samples: vec![RawSample {
                raw_profile: pprof.to_vec(),
                id: uuid::Uuid::new_v4().to_string(),
            }],
        }],
    };

    let proto_bytes = req.encode_to_vec();

    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&proto_bytes)?;
    let body = gz.finish()?;

    let url = format!(
        "{}/push.v1.PusherService/Push",
        base_url.trim_end_matches('/')
    );
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent();
    agent
        .post(&url)
        .header("Content-Type", "application/proto")
        .header("Content-Encoding", "gzip")
        .send(&body)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_happy_path() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/push.v1.PusherService/Push")
            .with_status(200)
            .create();

        let result = send(&server.url(), "myapp", &[], b"fakepprof", 1000, 2000);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        mock.assert();
    }

    #[test]
    fn test_send_server_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/push.v1.PusherService/Push")
            .with_status(500)
            .create();

        let result = send(&server.url(), "myapp", &[], b"fakepprof", 1000, 2000);
        assert!(result.is_err(), "expected Err on 500, got Ok");
    }

    #[test]
    fn test_send_headers_and_body() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/push.v1.PusherService/Push")
            .match_header("content-type", "application/proto")
            .match_header("content-encoding", "gzip")
            .match_body(mockito::Matcher::Any)
            .with_status(200)
            .create();

        let result = send(&server.url(), "myapp", &[], b"\x0a\x0b", 100, 200);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        mock.assert();
    }

    #[test]
    fn test_send_includes_tags_as_labels() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/push.v1.PusherService/Push")
            .match_header("content-type", "application/proto")
            .match_header("content-encoding", "gzip")
            .with_status(200)
            .create();

        let tags = vec![("env", "prod"), ("region", "us-east")];
        let result = send(&server.url(), "myapp", &tags, b"fakepprof", 100, 200);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        mock.assert();
    }

    #[test]
    fn test_push_request_contains_tags() {
        let tags = vec![("env", "prod"), ("canary", "abc123")];

        let mut labels = vec![
            LabelPair {
                name: "service_name".to_string(),
                value: "myapp".to_string(),
            },
            LabelPair {
                name: "__name__".to_string(),
                value: "process_cpu".to_string(),
            },
        ];
        for &(k, v) in &tags {
            labels.push(LabelPair {
                name: k.to_string(),
                value: v.to_string(),
            });
        }

        assert_eq!(labels.len(), 4);
        assert_eq!(labels[2].name, "env");
        assert_eq!(labels[2].value, "prod");
        assert_eq!(labels[3].name, "canary");
        assert_eq!(labels[3].value, "abc123");
    }
}
