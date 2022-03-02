use super::error::Result;
use super::types::{Backend, State};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use py_spy::config::Config;
use py_spy::sampler::Sampler;

// TODO: initialize with default args and add helper functions
// TODO: state for the thread + stop function
// TODO: refactor fold function
// TODO: handle errors and unwraps
// TODO: document configuration options

#[derive(Debug, Clone)]
pub struct PyspyConfig {
    pid: i32,
    sample_rate: u32,
    lock_process: bool,
    time_limit: Option<core::time::Duration>,
    with_subprocesses: bool,
    include_idle: bool,
    gil_only: bool,
    native: bool,
}

impl Default for PyspyConfig {
    fn default() -> Self {
        PyspyConfig {
            pid: 0,
            sample_rate: 100,
            lock_process: false,
            time_limit: None,
            with_subprocesses: false,
            include_idle: false,
            gil_only: false,
            native: false,
        }
    }
}

impl PyspyConfig {
    pub fn new(
        pid: i32, sample_rate: u32, lock_process: bool, time_limit: Option<core::time::Duration>,
        with_subprocesses: bool, include_idle: bool, gil_only: bool, native: bool,
    ) -> Self {
        PyspyConfig {
            pid,
            sample_rate,
            lock_process,
            time_limit,
            with_subprocesses,
            include_idle,
            gil_only,
            native,
        }
    }
}

#[derive(Default)]
pub struct Pyspy {
    state: State,
    buffer: Arc<Mutex<HashMap<String, usize>>>,
    config: PyspyConfig,
}

impl std::fmt::Debug for Pyspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pyspy Backend")
    }
}

impl Pyspy {
    pub fn new(config: PyspyConfig) -> Self {
        Pyspy {
            state: State::Uninitialized,
            buffer: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
}

impl Backend for Pyspy {
    fn get_state(&self) -> State {
        self.state
    }

    fn spy_name(&self) -> Result<String> {
        Ok("pyspy".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        //let buffer = Some(Arc::new(Mutex::new(String::new())));
        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        let buffer = self.buffer.clone();

        let config_c = self.config.clone();

        std::thread::spawn(move || {
            let mut config = Config::default();

            config.subprocesses = config_c.with_subprocesses;

            config.native = config_c.native;

            if config_c.lock_process {
                config.blocking = py_spy::config::LockingStrategy::Lock;
            } else {
                config.blocking = py_spy::config::LockingStrategy::NonBlocking;
            }

            config.gil_only = config_c.gil_only;

            config.include_idle = config_c.include_idle;

            let sampler = Sampler::new(config_c.pid, &config).unwrap();

            for sample in sampler {
                for trace in sample.traces {
                    if !(config.include_idle || trace.active) {
                        continue;
                    }

                    if config.gil_only && !trace.owns_gil {
                        continue;
                    }

                    let frame = trace
                        .frames
                        .iter()
                        .rev()
                        .map(|frame| {
                            let filename = match &frame.short_filename {
                                Some(f) => &f,
                                None => &frame.filename,
                            };
                            if frame.line != 0 {
                                format!("{} ({}:{})", frame.name, filename, frame.line)
                            } else if filename.len() > 0 {
                                format!("{} ({})", frame.name, filename)
                            } else {
                                frame.name.clone()
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(";");
                    *buffer.lock().unwrap().entry(frame).or_insert(0) += 1;
                }
            }
        });

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        let mut buffer = self.buffer.clone();

        let col: Vec<String> = buffer
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| format!("{} {}", k, v))
            .collect();

        let v8: Vec<u8> = col.join("\n").into_bytes();

        buffer.lock().unwrap().clear();

        Ok(v8)
    }
}
