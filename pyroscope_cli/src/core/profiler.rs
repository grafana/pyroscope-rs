use pyroscope::{
    pyroscope::{PyroscopeAgentReady, PyroscopeAgentRunning, PyroscopeAgentState},
    PyroscopeAgent,
};
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use pyroscope_rbspy::{rbspy_backend, RbspyConfig};

use crate::utils::{
    app_config::AppConfig,
    error::{Error, Result},
    types::Spy,
};

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

        let server_address: String = AppConfig::get::<String>("server_address")?;

        let sample_rate: u32 = AppConfig::get::<u32>("sample_rate")?;

        let blocking: bool = AppConfig::get::<bool>("blocking")?;

        let pyspy_idle: bool = AppConfig::get::<bool>("pyspy_idle")?;
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
                    .with_subprocesses(detect_subprocesses)
                    .include_idle(pyspy_idle)
                    .gil_only(pyspy_gil)
                    .native(pyspy_native);
                let backend = pyspy_backend(config);
                PyroscopeAgent::builder(server_address, app_name)
                    .backend(backend)
                    .tags(tags)
                    .build()?
            }
            Spy::Rbspy => {
                let config = RbspyConfig::new(pid)
                    .sample_rate(sample_rate)
                    .lock_process(blocking)
                    .with_subprocesses(detect_subprocesses);
                let backend = rbspy_backend(config);
                PyroscopeAgent::builder(server_address, app_name)
                    .backend(backend)
                    .tags(tags)
                    .build()?
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
