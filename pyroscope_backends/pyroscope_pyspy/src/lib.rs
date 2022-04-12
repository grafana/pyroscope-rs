use py_spy::{config::Config, sampler::Sampler};
use pyroscope::{
    backend::{Backend, BackendImpl, BackendUninitialized, Report, StackFrame, StackTrace},
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

pub fn pyspy_backend(config: PyspyConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Arc::new(Mutex::new(Pyspy::new(config))))
}

/// Pyspy Configuration
#[derive(Debug, Clone)]
pub struct PyspyConfig {
    /// Process to monitor
    pid: Option<i32>,
    /// Sampling rate
    sample_rate: u32,
    /// Lock Process while sampling
    lock_process: py_spy::config::LockingStrategy,
    /// Profiling duration (None for infinite)
    time_limit: Option<core::time::Duration>,
    /// Include subprocesses
    with_subprocesses: bool,
    /// Include idle time
    include_idle: bool,
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
            lock_process: py_spy::config::LockingStrategy::NonBlocking,
            time_limit: None,
            with_subprocesses: false,
            include_idle: false,
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
    buffer: Arc<Mutex<Report>>,
    /// Pyspy Configuration
    config: PyspyConfig,
    /// Sampler configuration
    sampler_config: Option<Config>,
    /// Sampler thread
    sampler_thread: Option<JoinHandle<Result<()>>>,
    /// Atomic flag to stop the sampler
    running: Arc<AtomicBool>,
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
            buffer: Arc::new(Mutex::new(Report::default())),
            config,
            sampler_config: None,
            sampler_thread: None,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Backend for Pyspy {
    fn spy_name(&self) -> Result<String> {
        Ok("pyspy".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

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
            include_idle: self.config.include_idle,
            include_thread_ids: true,
            subprocesses: self.config.with_subprocesses,
            gil_only: self.config.gil_only,
            duration,
            ..Config::default()
        });

        //
        //
        //
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

        self.sampler_thread = Some(std::thread::spawn(move || {
            // Get PID
            // TODO: we are doing lots of these checks, we should probably do this once
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
                    if !(config.include_idle || trace.active) {
                        continue;
                    }

                    if config.gil_only && !trace.owns_gil {
                        continue;
                    }

                    // Convert py-spy trace to a Pyroscope trace
                    let own_trace: StackTrace =
                        Into::<StackTraceWrapper>::into(trace.clone()).into();

                    // Add the trace to the buffer
                    buffer.lock()?.record(own_trace)?;
                }
            }

            Ok(())
        }));

        Ok(())
    }

    fn shutdown(self) -> Result<()> {
        // set running to false
        //self.running.store(false, Ordering::Relaxed);

        // wait for sampler thread to finish
        self.sampler_thread
            //.take()
            .ok_or_else(|| PyroscopeError::new("Pyspy: Failed to unwrap Sampler Thread"))?
            .join()
            .unwrap_or_else(|_| Err(PyroscopeError::new("Pyspy: Failed to join sampler thread")))?;

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        // create a new buffer reference
        let buffer = self.buffer.clone();

        // convert the buffer report into a byte vector
        let report: Report = buffer.lock()?.deref().to_owned();
        let reports = vec![report];

        // Clear the buffer
        buffer.lock()?.clear();

        Ok(reports)
    }
}

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

struct StackTraceWrapper(StackTrace);

impl From<StackTraceWrapper> for StackTrace {
    fn from(stack_trace: StackTraceWrapper) -> Self {
        stack_trace.0
    }
}

impl From<py_spy::StackTrace> for StackTraceWrapper {
    fn from(trace: py_spy::StackTrace) -> Self {
        StackTraceWrapper(StackTrace {
            pid: Some(trace.pid as u32),
            thread_id: Some(trace.thread_id as u64),
            thread_name: trace.thread_name.clone(),
            frames: trace
                .frames
                .iter()
                .map(|frame| Into::<StackFrameWrapper>::into(frame.clone()).into())
                .collect(),
        })
    }
}
