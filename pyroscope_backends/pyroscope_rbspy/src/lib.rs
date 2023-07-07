use pyroscope::{
    backend::{
        Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, Rule, Ruleset,
        StackBuffer, StackFrame, StackTrace,
    },
    error::{PyroscopeError, Result},
};
use pyroscope_rbspy_oncpu::sampler::Sampler;
use std::{
    ops::Deref,
    sync::{
        mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

const LOG_TAG: &str = "Pyroscope::Rbspy";

/// Short-hand function for creating a new Rbspy backend.
pub fn rbspy_backend(config: RbspyConfig) -> BackendImpl<BackendUninitialized> {
    // Clone BackendConfig to pass to the backend object.
    let backend_config = config.backend_config;

    // Create a new backend object.
    BackendImpl::new(Box::new(Rbspy::new(config)), Some(backend_config))
}

/// Rbspy Configuration
#[derive(Debug)]
pub struct RbspyConfig {
    /// Process to monitor
    pid: Option<i32>,
    /// Sampling rate
    sample_rate: u32,
    /// Backend Config
    backend_config: BackendConfig,
    /// Lock Process while sampling
    lock_process: bool,
    /// Profiling duration. None for infinite.
    time_limit: Option<core::time::Duration>,
    /// Include subprocesses
    detect_subprocesses: bool,
    /// Include Oncpu Time
    oncpu: bool,
}

impl Default for RbspyConfig {
    fn default() -> Self {
        RbspyConfig {
            pid: None,
            sample_rate: 100,
            backend_config: BackendConfig::default(),
            lock_process: false,
            time_limit: None,
            detect_subprocesses: false,
            oncpu: false,
        }
    }
}

impl RbspyConfig {
    /// Create a new RbspyConfig
    pub fn new(pid: i32) -> Self {
        RbspyConfig {
            pid: Some(pid),
            ..Default::default()
        }
    }

    /// Set the sampling rate
    pub fn sample_rate(self, sample_rate: u32) -> Self {
        RbspyConfig {
            sample_rate,
            ..self
        }
    }

    /// Tag thread id in report
    pub fn report_pid(self, report_pid: bool) -> Self {
        let backend_config = BackendConfig {
            report_pid,
            ..self.backend_config
        };

        RbspyConfig {
            backend_config,
            ..self
        }
    }

    /// Tag thread id in report
    pub fn report_thread_id(self, report_thread_id: bool) -> Self {
        let backend_config = BackendConfig {
            report_thread_id,
            ..self.backend_config
        };

        RbspyConfig {
            backend_config,
            ..self
        }
    }

    /// Set the lock process flag
    pub fn lock_process(self, lock_process: bool) -> Self {
        RbspyConfig {
            lock_process,
            ..self
        }
    }

    /// Set the time limit
    pub fn time_limit(self, time_limit: Option<core::time::Duration>) -> Self {
        RbspyConfig { time_limit, ..self }
    }

    /// Include subprocesses
    pub fn detect_subprocesses(self, detect_subprocesses: bool) -> Self {
        RbspyConfig {
            detect_subprocesses,
            ..self
        }
    }

    /// Include idle time
    pub fn oncpu(self, oncpu: bool) -> Self {
        RbspyConfig { oncpu, ..self }
    }
}

/// Rbspy Backend
#[derive(Default)]
pub struct Rbspy {
    /// Rbspy Configuration
    config: RbspyConfig,
    /// Rbspy Sampler
    sampler: Option<Sampler>,
    /// StackTrace Receiver
    //stack_receiver: Option<Receiver<pyroscope_rbspy_oncpu::StackTrace>>,
    /// Error Receiver
    error_receiver: Option<Receiver<std::result::Result<(), anyhow::Error>>>,
    /// Profiling buffer
    buffer: Arc<Mutex<StackBuffer>>,
    /// Rulset
    ruleset: Ruleset,
}

impl std::fmt::Debug for Rbspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rbspy Backend")
    }
}

impl Rbspy {
    /// Create a new Rbspy instance
    pub fn new(config: RbspyConfig) -> Self {
        Rbspy {
            sampler: None,
            //stack_receiver: None,
            error_receiver: None,
            config,
            buffer: Arc::new(Mutex::new(StackBuffer::default())),
            ruleset: Ruleset::default(),
        }
    }
}

// Type aliases
type ErrorSender = Sender<std::result::Result<(), anyhow::Error>>;
type ErrorReceiver = Receiver<std::result::Result<(), anyhow::Error>>;

impl Backend for Rbspy {
    /// Return the backend name
    fn spy_name(&self) -> Result<String> {
        Ok("rbspy".to_string())
    }

    /// Return the backend extension
    fn spy_extension(&self) -> Result<Option<String>> {
        Ok(Some("cpu".to_string()))
    }

    /// Return the sample rate
    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn set_config(&self, _config: BackendConfig) {}

    fn get_config(&self) -> Result<BackendConfig> {
        Ok(self.config.backend_config)
    }

    /// Add a rule to the ruleset
    fn add_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.add_rule(rule)?;

        Ok(())
    }

    /// Remove a rule from the ruleset
    fn remove_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.remove_rule(rule)?;

        Ok(())
    }

    /// Initialize the backend
    fn initialize(&mut self) -> Result<()> {
        // Check if a process ID is set
        if self.config.pid.is_none() {
            return Err(PyroscopeError::new("Rbspy: No Process ID Specified"));
        }

        // Create Sampler
        self.sampler = Some(Sampler::new(
            self.config.pid.unwrap(), // unwrap is safe because of check above
            self.config.sample_rate,
            self.config.lock_process,
            self.config.time_limit,
            self.config.detect_subprocesses,
            None,
            self.config.oncpu,
        ));

        // Channel for Errors generated by the RubySpy Sampler
        let (error_sender, error_receiver): (ErrorSender, ErrorReceiver) = channel();

        // This is provides enough space for 100 threads.
        // It might be a better idea to figure out how many threads are running and determine the
        // size of the channel based on that.
        let queue_size: usize = self.config.sample_rate as usize * 10 * 100;

        // Channel for StackTraces generated by the RubySpy Sampler
        let (stack_sender, stack_receiver): (
            SyncSender<pyroscope_rbspy_oncpu::StackTrace>,
            Receiver<pyroscope_rbspy_oncpu::StackTrace>,
        ) = sync_channel(queue_size);

        // Set Error and Stack Receivers
        //self.stack_receiver = Some(stack_receiver);
        self.error_receiver = Some(error_receiver);

        // Get the Sampler
        let sampler = self
            .sampler
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("Rbspy: Sampler is not set"))?;

        // Start the Sampler
        sampler
            .start(stack_sender, error_sender)
            .map_err(|e| PyroscopeError::new(&format!("Rbspy: Sampler Error: {}", e)))?;

        // Start own thread
        //
        // Get an Arc reference to the Report Buffer
        let buffer = self.buffer.clone();

        // ruleset reference
        let ruleset = self.ruleset.clone();

        let backend_config = self.config.backend_config;

        let _: JoinHandle<Result<()>> = std::thread::spawn(move || {
            // Iterate over the StackTrace
            while let Ok(stack_trace) = stack_receiver.recv() {
                // convert StackTrace
                let own_trace: StackTrace =
                    Into::<StackTraceWrapper>::into((stack_trace, &backend_config)).into();

                let stacktrace = own_trace + &ruleset;

                buffer.lock()?.record(stacktrace)?;
            }

            Ok(())
        });

        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        log::trace!(target: LOG_TAG, "Shutting down sampler thread");

        // Stop Sampler
        self.sampler
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("Rbspy: Sampler is not set"))?
            .stop();

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        let v8: StackBuffer = self.buffer.lock()?.deref().to_owned();
        let reports: Vec<Report> = v8.into();

        self.buffer.lock()?.clear();

        // Return the writer's buffer
        Ok(reports)
    }
}

struct StackFrameWrapper(StackFrame);

impl From<StackFrameWrapper> for StackFrame {
    fn from(frame: StackFrameWrapper) -> Self {
        frame.0
    }
}

impl From<pyroscope_rbspy_oncpu::StackFrame> for StackFrameWrapper {
    fn from(frame: pyroscope_rbspy_oncpu::StackFrame) -> Self {
        StackFrameWrapper(StackFrame {
            module: None,
            name: Some(frame.name),
            filename: Some(frame.relative_path.clone()),
            relative_path: Some(frame.relative_path),
            absolute_path: frame.absolute_path,
            line: frame.lineno.map(|l| l as u32),
        })
    }
}

struct StackTraceWrapper(StackTrace);

impl From<StackTraceWrapper> for StackTrace {
    fn from(trace: StackTraceWrapper) -> Self {
        trace.0
    }
}

impl From<(pyroscope_rbspy_oncpu::StackTrace, &BackendConfig)> for StackTraceWrapper {
    fn from(arg: (pyroscope_rbspy_oncpu::StackTrace, &BackendConfig)) -> Self {
        let (trace, config) = arg;

        StackTraceWrapper(StackTrace::new(
            config,
            trace.pid.map(|pid| pid as u32),
            trace.thread_id.map(|id| id as u64),
            None,
            trace
                .iter()
                .map(|frame| Into::<StackFrameWrapper>::into(frame.clone()).into())
                .collect(),
        ))
    }
}
