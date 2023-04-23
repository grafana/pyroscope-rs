use std::collections::HashMap;
use pyroscope::{pyroscope::PyroscopeAgentRunning, PyroscopeAgent};
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use pyroscope_rbspy::{rbspy_backend, RbspyConfig};

use crate::utils::{
    app_config::AppConfig,
    error::{Error, Result},
    types::Spy,
};

const LOG_TAG: &str = "Pyroscope::cli";

/// Wrapper for the `pyroscope` library and the `pyroscope_pyspy` and `pyroscope_rbspy` backends.
#[derive(Debug, Default)]
pub struct Profiler {
    agent: Option<PyroscopeAgent<PyroscopeAgentRunning>>,
}

impl Profiler {
    /// Creates a new instance of the `Profiler` and initializes the `pyroscope` library and one of the backends.
    pub fn init(&mut self) -> Result<()> {
        let pid: i32 = AppConfig::get::<i32>("pid")?;

        let app_name: String = AppConfig::get::<String>("application_name")?;

        let auth_token: String = AppConfig::get::<String>("auth_token")?;
        let basic_auth_username: String = AppConfig::get::<String>("basic_auth_username")?;
        let basic_auth_password: String = AppConfig::get::<String>("basic_auth_password")?;

        let scope_org_id: String = AppConfig::get::<String>("scope_org_id").unwrap_or("".to_string());

        let http_headers = get_http_headers();

        let server_address: String = AppConfig::get::<String>("server_address")?;

        let sample_rate: u32 = AppConfig::get::<u32>("sample_rate")?;

        let blocking: bool = AppConfig::get::<bool>("blocking")?;

        let oncpu: bool = AppConfig::get::<bool>("oncpu")?;
        let pyspy_gil: bool = AppConfig::get::<bool>("pyspy_gil")?;
        let pyspy_native: bool = AppConfig::get::<bool>("pyspy_native")?;

        let detect_subprocesses: bool = AppConfig::get::<bool>("detect_subprocesses")?;

        let tag_str = &AppConfig::get::<String>("tag")?;
        let tags = tags_to_array(tag_str)?;

        let agent = match AppConfig::get::<Spy>("spy_name")? {
            Spy::Pyspy => {
                let config = PyspyConfig::new(pid)
                    .sample_rate(sample_rate)
                    .lock_process(blocking)
                    .detect_subprocesses(detect_subprocesses)
                    .oncpu(oncpu)
                    .gil_only(pyspy_gil)
                    .native(pyspy_native);

                let backend = pyspy_backend(config);

                let mut builder = PyroscopeAgent::default_builder();
                builder = builder.url(server_address);
                builder = builder.application_name(app_name);
                if scope_org_id != "" {
                    builder = builder.scope_org_id(scope_org_id);
                }
                if http_headers.len() > 0 {
                    builder = builder.http_headers(http_headers);
                }

                // There must be a better way to do this, hopefully as clap supports Option<String>
                if auth_token.len() > 0 {
                    builder = builder.auth_token(auth_token);
                } else if basic_auth_username != "" && basic_auth_password != "" {
                    builder = builder.basic_auth(basic_auth_username, basic_auth_password);
                }

                builder.backend(backend).tags(tags).build()?
            }
            Spy::Rbspy => {
                let config = RbspyConfig::new(pid)
                    .sample_rate(sample_rate)
                    .lock_process(blocking)
                    .oncpu(oncpu)
                    .detect_subprocesses(detect_subprocesses);
                let backend = rbspy_backend(config);

                let mut builder = PyroscopeAgent::default_builder();
                builder = builder.url(server_address);
                builder = builder.application_name(app_name);
                if scope_org_id != "" {
                    builder = builder.scope_org_id(scope_org_id);
                }

                // There must be a better way to do this, hopefully as clap supports Option<String>
                if auth_token.len() > 0 {
                    builder = builder.auth_token(auth_token);
                }

                builder.backend(backend).tags(tags).build()?
            }
        };

        let agent_running = agent.start()?;

        self.agent = Some(agent_running);

        Ok(())
    }

    /// Stops the `pyroscope` library agent and the backend.
    pub fn stop(self) -> Result<()> {
        if let Some(agent_running) = self.agent {
            let agent_ready = agent_running.stop()?;
            agent_ready.shutdown();
        }

        Ok(())
    }
}

/// Converts a string of semi-colon-separated tags into an array of tags.
fn tags_to_array(tags: &str) -> Result<Vec<(&str, &str)>> {
    // check if tags is empty
    if tags.is_empty() {
        return Ok(Vec::new());
    }

    let mut tags_array = Vec::new();

    for tag in tags.split(';') {
        let mut tag_array = tag.split('=');
        let key = tag_array
            .next()
            .ok_or_else(|| Error::new("failed to parse tag key"))?;
        let value = tag_array
            .next()
            .ok_or_else(|| Error::new("failed to parse tag value"))?;
        tags_array.push((key, value));
    }

    Ok(tags_array)
}

#[cfg(test)]
mod test_tags_to_array {
    use super::*;

    #[test]
    fn test_tags_to_array_empty() {
        let tags = tags_to_array("").unwrap();
        assert_eq!(tags.len(), 0);
    }

    #[test]
    fn test_tags_to_array_one_tag() {
        let tags = tags_to_array("key=value").unwrap();

        assert_eq!(tags.len(), 1);

        assert_eq!(tags, vec![("key", "value")]);
    }

    #[test]
    fn test_tags_to_array_multiple_tags() {
        let tags = tags_to_array("key1=value1;key2=value2").unwrap();
        assert_eq!(tags.len(), 2);

        assert_eq!(tags, vec![("key1", "value1"), ("key2", "value2")]);
    }
}

fn get_http_headers() -> HashMap<String, String> {
    let http_headers: String = AppConfig::get::<String>("http_headers_json")
        .unwrap_or("{}".to_string());

    let http_headers = pyroscope::pyroscope::parse_http_headers_json(http_headers)
        .unwrap_or_else(|e| {
            match e {
                pyroscope::PyroscopeError::Json(e) => {
                    log::error!(target: LOG_TAG, "parse_http_headers_json error {}", e);
                }
                pyroscope::PyroscopeError::AdHoc(e) => {
                    log::error!(target: LOG_TAG, "parse_http_headers_json {}", e);
                }
                _ => {}
            }
            HashMap::new()
        });
    return http_headers;
}