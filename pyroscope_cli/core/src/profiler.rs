use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{Pyspy, PyspyConfig};
use pyroscope_rbspy::{Rbspy, RbspyConfig};
use utils::app_config::AppConfig;
use utils::error::{Error, Result};
use utils::types::Spy;

#[derive(Debug, Default)]
pub struct Profiler {
    agent: Option<PyroscopeAgent>,
}

impl Profiler {
    pub fn init(&mut self) -> Result<()> {
        let pid: i32 = AppConfig::get::<i32>("pid")?;

        let app_name: String = AppConfig::get::<String>("application_name")?;

        let server_address: String = AppConfig::get::<String>("server_address")?;

        let sample_rate: u32 = AppConfig::get::<u32>("sample_rate")?;

        // TODO: CLI should probably unify this into a single argument
        let rbspy_blocking: bool = AppConfig::get::<bool>("rbspy_blocking")?;
        let pyspy_blocking: bool = AppConfig::get::<bool>("pyspy_blocking")?;
        let pyspy_idle: bool = AppConfig::get::<bool>("pyspy_idle")?;
        let pyspy_gil: bool = AppConfig::get::<bool>("pyspy_gil")?;
        let pyspy_native: bool = AppConfig::get::<bool>("pyspy_native")?;

        let detect_subprocesses: bool = AppConfig::get::<bool>("detect_subprocesses")?;

        let tag_str = &AppConfig::get::<String>("tag")?;
        let tags = tags_to_array(tag_str)?;

        let mut agent = match AppConfig::get::<Spy>("spy_name")? {
            Spy::Pyspy => {
                let config = PyspyConfig::new(pid)
                    .sample_rate(sample_rate)
                    .lock_process(pyspy_blocking)
                    .with_subprocesses(detect_subprocesses)
                    .include_idle(pyspy_idle)
                    .gil_only(pyspy_gil)
                    .native(pyspy_native);
                let backend = Pyspy::new(config);
                PyroscopeAgent::builder(server_address, app_name)
                    .backend(backend)
                    .tags(tags)
                    .build()?
            }
            Spy::Rbspy => {
                let config = RbspyConfig::new(pid)
                    .sample_rate(sample_rate)
                    .lock_process(rbspy_blocking)
                    .with_subprocesses(detect_subprocesses);
                let backend = Rbspy::new(config);
                PyroscopeAgent::builder(server_address, app_name)
                    .backend(backend)
                    .tags(tags)
                    .build()?
            }
        };

        agent.start()?;

        self.agent = Some(agent);

        Ok(())
    }

    pub fn stop(self) -> Result<()> {
        if let Some(mut agent) = self.agent {
            agent.stop()?;
        }

        Ok(())
    }
}

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
