use std::time::Duration;

use anyhow::Result;

/// Send a gzipped pprof profile to the Pyroscope ingest endpoint.
///
/// - `base_url`: e.g. `"http://localhost:4040"`
/// - `app_name`: application name; sent as `{app_name}.cpu`
/// - `pprof_gz`: gzipped pprof protobuf bytes
/// - `from`: profile start time (Unix seconds)
/// - `until`: profile end time (Unix seconds)
///
/// Errors are returned but do not retry — callers should log and discard on failure.
pub fn send(base_url: &str, app_name: &str, pprof_gz: &[u8], from: u64, until: u64) -> Result<()> {
    let url = format!(
        "{}/ingest?name={}.cpu&from={}&until={}&format=pprof&spyName=pyroscope-cpython-rs&sampleRate=100",
        base_url, app_name, from, until
    );
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent();
    agent
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .send(pprof_gz)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_happy_path() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", mockito::Matcher::Regex(r"^/ingest\?".to_string()))
            .with_status(200)
            .create();

        let result = send(&server.url(), "myapp", b"fakegzdata", 1000, 2000);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        mock.assert();
    }

    #[test]
    fn test_send_server_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", mockito::Matcher::Regex(r"^/ingest\?".to_string()))
            .with_status(500)
            .create();

        let result = send(&server.url(), "myapp", b"fakegzdata", 1000, 2000);
        assert!(result.is_err(), "expected Err on 500, got Ok");
    }

    #[test]
    fn test_send_url_query_params() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/ingest?name=myapp.cpu&from=100&until=200&format=pprof&spyName=pyroscope-cpython-rs&sampleRate=100")
            .match_header("content-type", "application/octet-stream")
            .match_body(mockito::Matcher::Any)
            .with_status(200)
            .create();

        let result = send(&server.url(), "myapp", b"\x1f\x8b", 100, 200);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        mock.assert();
    }
}
