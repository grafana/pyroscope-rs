use pyroscope::PyroscopeAgent;
use pyroscope_pprofrs::{Pprof, PprofConfig};
use pyroscope_pyspy::{Pyspy, PyspyConfig};
use pyroscope_rbspy::{Rbspy, RbspyConfig};
use utils::app_config::AppConfig;
use utils::error::Result;
use utils::types::Spy;

#[derive(Debug, Default)]
pub struct Profiler {
    agent: Option<PyroscopeAgent>,
}

impl Profiler {
    pub fn init(&mut self) -> Result<()> {
        let pid: i32 = AppConfig::get::<i32>("pid")?;

        let app_name: String = get_application_name()?;

        let server_address: String = AppConfig::get::<String>("server_address")?;

        let sample_rate: u32 = AppConfig::get::<u32>("sample_rate")?;

        // TODO: CLI should probably unify this into a single argument
        let pyspy_blocking: bool = AppConfig::get::<bool>("pyspy_blocking")?;
        let rbspy_blocking: bool = AppConfig::get::<bool>("rbspy_blocking")?;

        let detect_subprocesses: bool = AppConfig::get::<bool>("detect_subprocesses")?;

        let mut agent = match AppConfig::get::<Spy>("spy_name")? {
            Spy::Rustspy => {
                println!("here");
                let config = PprofConfig::new(100);
                let backend = Pprof::new(config);
                PyroscopeAgent::builder(server_address, app_name)
                    .backend(backend)
                    .build()?
            }
            Spy::Pyspy => {
                let config = PyspyConfig::new(pid)
                    .sample_rate(sample_rate)
                    .lock_process(pyspy_blocking)
                    .with_subprocesses(detect_subprocesses);
                let backend = Pyspy::new(config);
                PyroscopeAgent::builder(server_address, app_name)
                    .backend(backend)
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
                    .build()?
            }
            _ => return Ok(()),
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

fn get_application_name() -> Result<String> {
    let pre_app_name: String = AppConfig::get::<String>("application_name").unwrap_or_else(|_| {
        names::Generator::default()
            .next()
            .unwrap_or("unassigned.app".to_string())
            .replace("-", ".")
    });

    let pre = match AppConfig::get::<Spy>("spy_name")? {
        Spy::Pyspy => "pyspy",
        Spy::Rbspy => "rbspy",
        _ => "none",
    };

    // add pre to pre_app_name
    let app_name = format!("{}.{}", pre, pre_app_name);

    Ok(app_name)
}
