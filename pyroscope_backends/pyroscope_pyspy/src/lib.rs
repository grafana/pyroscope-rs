use py_spy::{config::Config, sampler::Sampler};
use pyroscope::{
    backend::{
        Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, Rule, Ruleset,
        StackBuffer, StackFrame, StackTrace,
    },
    error::{PyroscopeError, Result},
};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

const LOG_TAG: &str = "Pyroscope::Pyspy";

/// Short-hand function for creating a new Pyspy backend.
pub fn pyspy_backend(config: PyspyConfig) -> BackendImpl<BackendUninitialized> {
    // Clone BackendConfig to pass to the backend object.
    let backend_config = config.backend_config;

    BackendImpl::new(Box::new(Pyspy::new(config)), Some(backend_config))
}

/// Pyspy Configuration
#[derive(Debug, Clone)]
pub struct PyspyConfig {
    /// Process to monitor
    pid: Option<i32>,
    /// Sampling rate
    sample_rate: u32,
    /// Backend Config
    backend_config: BackendConfig,
    /// Lock Process while sampling
    lock_process: py_spy::config::LockingStrategy,
    /// Profiling duration (None for infinite)
    time_limit: Option<core::time::Duration>,
    /// Include subprocesses
    detect_subprocesses: bool,
    /// Include idle time
    oncpu: bool,
    /// Detect Python GIL
    gil_only: bool,
    /// Profile native C extensions
    native: bool,
}

impl Default for PyspyConfig {
    fn default() -> Self {
        PyspyConfig {
            pid: Some(0),
            sample_rate: 100,
            backend_config: BackendConfig::default(),
            lock_process: py_spy::config::LockingStrategy::NonBlocking,
            time_limit: None,
            detect_subprocesses: false,
            oncpu: false,
            gil_only: false,
            native: false,
        }
    }
}

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

    /// Tag thread id in report
    pub fn report_pid(self) -> Self {
        let backend_config = BackendConfig {
            report_pid: true,
            ..self.backend_config
        };

        PyspyConfig {
            backend_config,
            ..self
        }
    }

    /// Tag thread id in report
    pub fn report_thread_id(self) -> Self {
        let backend_config = BackendConfig {
            report_thread_id: true,
            ..self.backend_config
        };

        PyspyConfig {
            backend_config,
            ..self
        }
    }

    /// Tag thread name in report
    pub fn report_thread_name(self) -> Self {
        let backend_config = BackendConfig {
            report_thread_name: true,
            ..self.backend_config
        };

        PyspyConfig {
            backend_config,
            ..self
        }
    }

    /// Set the lock process flag
    pub fn lock_process(self, lock_process: bool) -> Self {
        PyspyConfig {
            lock_process: if lock_process {
                py_spy::config::LockingStrategy::Lock
            } else {
                py_spy::config::LockingStrategy::NonBlocking
            },
            ..self
        }
    }

    /// Set the time limit
    pub fn time_limit(self, time_limit: Option<core::time::Duration>) -> Self {
        PyspyConfig { time_limit, ..self }
    }

    /// Include subprocesses
    pub fn detect_subprocesses(self, detect_subprocesses: bool) -> Self {
        PyspyConfig {
            detect_subprocesses,
            ..self
        }
    }

    /// Include idle time
    pub fn oncpu(self, oncpu: bool) -> Self {
        PyspyConfig { oncpu, ..self }
    }

    /// Detect Python GIL
    pub fn gil_only(self, gil_only: bool) -> Self {
        PyspyConfig { gil_only, ..self }
    }

    /// Profile native C extensions
    pub fn native(self, native: bool) -> Self {
        PyspyConfig { native, ..self }
    }
}

/// Pyspy Backend
#[derive(Default)]
pub struct Pyspy {
    /// Profiling buffer
    buffer: Arc<Mutex<StackBuffer>>,
    /// Pyspy Configuration
    config: PyspyConfig,
    /// Sampler configuration
    sampler_config: Option<Config>,
    /// Sampler thread
    sampler_thread: Option<JoinHandle<Result<()>>>,
    /// Atomic flag to stop the sampler
    running: Arc<AtomicBool>,
    /// Ruleset
    ruleset: Arc<Mutex<Ruleset>>,
}

impl std::fmt::Debug for Pyspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pyspy Backend")
    }
}

impl Pyspy {
    /// Create a new Pyspy Backend.
    pub fn new(config: PyspyConfig) -> Self {
        Pyspy {
            buffer: Arc::new(Mutex::new(StackBuffer::default())),
            config,
            sampler_config: None,
            sampler_thread: None,
            running: Arc::new(AtomicBool::new(false)),
            ruleset: Arc::new(Mutex::new(Ruleset::default())),
        }
    }
}

impl Backend for Pyspy {
    /// Return the name of the backend.
    fn spy_name(&self) -> Result<String> {
        Ok("pyspy".to_string())
    }

    /// Return the extension of the backend.
    fn spy_extension(&self) -> Result<Option<String>> {
        Ok(Some("cpu".to_string()))
    }

    /// Return the sample rate.
    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn set_config(&self, _config: BackendConfig) {}

    fn get_config(&self) -> Result<BackendConfig> {
        Ok(self.config.backend_config)
    }

    /// Add a rule to the ruleset.
    fn add_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.lock()?.add_rule(rule)?;

        Ok(())
    }

    /// Remove a rule from the ruleset.
    fn remove_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.lock()?.remove_rule(rule)?;

        Ok(())
    }

    /// Initialize the backend.
    fn initialize(&mut self) -> Result<()> {
        // Check if a process ID is set
        if self.config.pid.is_none() {
            return Err(PyroscopeError::new("Pyspy: No Process ID Specified"));
        }

        // Set duration for py-spy
        let duration = match self.config.time_limit {
            Some(duration) => py_spy::config::RecordDuration::Seconds(duration.as_secs()),
            None => py_spy::config::RecordDuration::Unlimited,
        };

        // Create a new py-spy configuration
        self.sampler_config = Some(Config {
            blocking: self.config.lock_process.clone(),
            native: self.config.native,
            pid: self.config.pid,
            sampling_rate: self.config.sample_rate as u64,
            include_idle: self.config.oncpu,
            include_thread_ids: true,
            subprocesses: self.config.detect_subprocesses,
            gil_only: self.config.gil_only,
            duration,
            ..Config::default()
        });

        // set sampler state to running
        let running = Arc::clone(&self.running);
        running.store(true, Ordering::Relaxed);

        // create a new buffer reference
        let buffer = self.buffer.clone();

        // create a new sampler_config reference
        let config = self
            .sampler_config
            .clone()
            .ok_or_else(|| PyroscopeError::new("Pyspy: Sampler configuration is not set"))?;

        // create a new ruleset reference
        let ruleset = self.ruleset.clone();

        let backend_config = self.config.backend_config;

        self.sampler_thread = Some(std::thread::spawn(move || {
            // Get PID
            let pid = config
                .pid
                .ok_or_else(|| PyroscopeError::new("Pyspy: PID is not set"))?;

            // Create a new pyspy sampler
            let sampler = Sampler::new(pid, &config)
                .map_err(|e| PyroscopeError::new(&format!("Pyspy: Sampler Error: {}", e)))?;

            // Keep the sampler running until the running flag is set to false
            let sampler_output = sampler.take_while(|_x| running.load(Ordering::Relaxed));

            // Collect the sampler output
            for sample in sampler_output {
                for trace in sample.traces {
                    // idle config
                    if !(config.include_idle || trace.active) {
                        continue;
                    }

                    // gil config
                    if config.gil_only && !trace.owns_gil {
                        continue;
                    }

                    // Convert py-spy trace to a Pyroscope trace
                    let own_trace: StackTrace =
                        Into::<StackTraceWrapper>::into((trace.clone(), &backend_config)).into();

                    // apply ruleset
                    let stacktrace = own_trace + &ruleset.lock()?.clone();

                    // Add the trace to the buffer
                    buffer.lock()?.record(stacktrace)?;
                }
            }

            Ok(())
        }));

        Ok(())
    }

    /// Shutdown the backend.
    fn shutdown(self: Box<Self>) -> Result<()> {
        log::trace!(target: LOG_TAG, "Shutting down sampler thread");

        // set running to false, terminate sampler thread
        self.running.store(false, Ordering::Relaxed);

        // wait for sampler thread to finish
        self.sampler_thread
            .ok_or_else(|| PyroscopeError::new("Pyspy: Failed to unwrap Sampler Thread"))?
            .join()
            .unwrap_or_else(|_| Err(PyroscopeError::new("Pyspy: Failed to join sampler thread")))?;

        Ok(())
    }

    /// Report buffer
    fn report(&mut self) -> Result<Vec<Report>> {
        // convert the buffer report into a byte vector
        let report: StackBuffer = self.buffer.lock()?.deref().to_owned();
        let reports: Vec<Report> = report.into();

        // Clear the buffer
        self.buffer.lock()?.clear();

        Ok(reports)
    }
}

/// Wrapper for StackFrame. This is needed because both StackFrame and
/// py_spy::Frame are not defined in the same module.
struct StackFrameWrapper(StackFrame);

impl From<StackFrameWrapper> for StackFrame {
    fn from(stack_frame: StackFrameWrapper) -> Self {
        stack_frame.0
    }
}

impl From<py_spy::Frame> for StackFrameWrapper {
    fn from(frame: py_spy::Frame) -> Self {
        StackFrameWrapper(StackFrame {
            module: frame.module.clone(),
            name: Some(frame.name.clone()),
            filename: frame.short_filename.clone(),
            relative_path: None,
            absolute_path: Some(frame.filename.clone()),
            line: Some(frame.line as u32),
        })
    }
}

/// Wrapper for StackTrace. This is needed because both StackTrace and
/// py_spy::StackTrace are not defined in the same module.
struct StackTraceWrapper(StackTrace);

impl From<StackTraceWrapper> for StackTrace {
    fn from(stack_trace: StackTraceWrapper) -> Self {
        stack_trace.0
    }
}

impl From<(py_spy::StackTrace, &BackendConfig)> for StackTraceWrapper {
    fn from(arg: (py_spy::StackTrace, &BackendConfig)) -> Self {
        let (stack_trace, config) = arg;
        let stacktrace = StackTrace::new(
            config,
            Some(stack_trace.pid as u32),
            Some(stack_trace.thread_id as u64),
            stack_trace.thread_name.clone(),
            stack_trace
                .frames
                .iter()
                .map(|frame| Into::<StackFrameWrapper>::into(frame.clone()).into())
                .collect(),
        );
        StackTraceWrapper(stacktrace)
    }
}
