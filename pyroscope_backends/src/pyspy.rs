use super::error::{BackendError, Result};
use super::types::{Backend, State};
use py_spy::config::Config;
use py_spy::sampler::Sampler;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// TODO: initialize with default args and add helper functions
// TODO: state for the thread + stop function
// TODO: refactor fold function

/// Pyspy Configuration
#[derive(Debug, Clone)]
pub struct PyspyConfig {
    /// Process to monitor
    pid: Option<i32>,
    /// Sampling rate
    sample_rate: u32,
    /// Lock Process while sampling
    lock_process: bool,
    /// Profiling duration. None for infinite.
    time_limit: Option<core::time::Duration>,
    /// Include subprocesses
    with_subprocesses: bool,
    /// todo
    include_idle: bool,
    /// todo
    gil_only: bool,
    /// todo
    native: bool,
}

impl Default for PyspyConfig {
    fn default() -> Self {
        PyspyConfig {
            pid: Some(0),
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

// TODO: use helper functions
impl PyspyConfig {
    /// Create a new PyspyConfig
    pub fn new(pid: i32) -> Self {
        PyspyConfig {
            pid: Some(pid),
            ..Default::default()
        }
    }

    /// Set the sampling rate
    pub fn sample_rate(self, sample_rate: u32) -> Self {
        PyspyConfig {
            sample_rate,
            ..self
        }
    }

    /// Set the lock process flag
    pub fn lock_process(self, lock_process: bool) -> Self {
        PyspyConfig {
            lock_process,
            ..self
        }
    }

    /// Set the time limit
    pub fn time_limit(self, time_limit: Option<core::time::Duration>) -> Self {
        PyspyConfig { time_limit, ..self }
    }

    /// Include subprocesses
    pub fn with_subprocesses(self, with_subprocesses: bool) -> Self {
        PyspyConfig {
            with_subprocesses,
            ..self
        }
    }

    /// Include idle time
    pub fn include_idle(self, include_idle: bool) -> Self {
        PyspyConfig {
            include_idle,
            ..self
        }
    }

    pub fn gil_only(self, gil_only: bool) -> Self {
        PyspyConfig { gil_only, ..self }
    }

    pub fn native(self, native: bool) -> Self {
        PyspyConfig { native, ..self }
    }
}

/// Pyspy Backend
#[derive(Default)]
pub struct Pyspy {
    /// Pyspy State
    state: State,
    /// Profiling buffer
    buffer: Arc<Mutex<HashMap<String, usize>>>,
    /// Pyspy Configuration
    config: PyspyConfig,
    /// Sampler configuration
    sampler_config: Option<Config>,
    /// Sampler thread
    sampler_thread: Option<JoinHandle<Result<()>>>,
}

impl std::fmt::Debug for Pyspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pyspy Backend")
    }
}

impl Pyspy {
    /// Create a new Pyspy Backend
    pub fn new(config: PyspyConfig) -> Self {
        Pyspy {
            state: State::Uninitialized,
            buffer: Arc::new(Mutex::new(HashMap::new())),
            config,
            sampler_config: None,
            sampler_thread: None,
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
        // Check if Backend is Uninitialized
        if self.state != State::Uninitialized {
            return Err(BackendError::new("Rbspy: Backend is already Initialized"));
        }

        // Check if a process ID is set
        if self.config.pid.is_none() {
            return Err(BackendError::new("Rbspy: No Process ID Specified"));
        }

        // Create a new py-spy configuration
        self.sampler_config = Some(Config {
            //blocking: config_c.lock_process,
            native: self.config.native,
            pid: self.config.pid,
            sampling_rate: self.config.sample_rate as u64,
            include_idle: self.config.include_idle,
            include_thread_ids: true,
            subprocesses: self.config.with_subprocesses,
            gil_only: self.config.gil_only,
            ..Config::default()
        });

        // Set State to Ready
        self.state = State::Ready;

        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        // Check if Backend is Ready
        if self.state != State::Ready {
            return Err(BackendError::new("Rbspy: Backend is not Ready"));
        }

        let buffer = self.buffer.clone();

        let config_c = self.config.clone();

        self.sampler_thread = Some(std::thread::spawn(move || {
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

            let sampler = Sampler::new(config_c.pid.unwrap(), &config)?;

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
                    *buffer.lock()?.entry(frame).or_insert(0) += 1;
                }
            }

            Ok(())
        }));

        // Set State to Running
        self.state = State::Running;

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Rbspy: Backend is not Running"));
        }

        // Set State to Running
        self.state = State::Ready;

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Rbspy: Backend is not Running"));
        }

        let buffer = self.buffer.clone();

        let col: Vec<String> = buffer
            .lock()?
            .iter()
            .map(|(k, v)| format!("{} {}", k, v))
            .collect();

        let v8: Vec<u8> = col.join("\n").into_bytes();

        buffer.lock()?.clear();

        Ok(v8)
    }
}
