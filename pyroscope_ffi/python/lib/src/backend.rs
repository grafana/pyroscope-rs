use py_spy::{sampler::Sampler};
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



#[derive(Default)]
pub struct Pyspy {
    buffer: Arc<Mutex<StackBuffer>>,
    config: py_spy::config::Config,
    backend_config: BackendConfig,
    sampler_thread: Option<JoinHandle<Result<()>>>,
    running: Arc<AtomicBool>,
    ruleset: Arc<Mutex<Ruleset>>,
}

impl std::fmt::Debug for Pyspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pyspy Backend")
    }
}

impl Pyspy {
    pub fn new(config: py_spy::config::Config, backend_config: BackendConfig) -> Self {
        Pyspy {
            buffer: Arc::new(Mutex::new(StackBuffer::default())),
            config,
            backend_config,
            sampler_thread: None,
            running: Arc::new(AtomicBool::new(false)),
            ruleset: Arc::new(Mutex::new(Ruleset::default())),
        }
    }
}

impl Backend for Pyspy {
    fn spy_name(&self) -> Result<String> {
        Ok("pyspy".to_string())
    }

    fn spy_extension(&self) -> Result<Option<String>> {
        Ok(Some("cpu".to_string()))
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sampling_rate as u32)
    }


    fn add_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.lock()?.add_rule(rule)?;

        Ok(())
    }

    fn remove_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.lock()?.remove_rule(rule)?;

        Ok(())
    }

    fn initialize(&mut self) -> Result<()> {
        if self.config.pid.is_none() {
            return Err(PyroscopeError::new("Pyspy: No Process ID Specified"));
        }


        let running = Arc::clone(&self.running);
        running.store(true, Ordering::Relaxed);

        let buffer = self.buffer.clone();

        let config = self.config.clone();


        let ruleset = self.ruleset.clone();

        let backend_config = self.backend_config;

        self.sampler_thread = Some(std::thread::spawn(move || {
            let pid = config
                .pid
                .ok_or_else(|| PyroscopeError::new("Pyspy: PID is not set"))?;

            let sampler = Sampler::new(pid, &config)
                .map_err(|e| PyroscopeError::new(&format!("Pyspy: Sampler Error: {}", e)))?;

            let sampler_output = sampler.take_while(|_x| running.load(Ordering::Relaxed));

            for sample in sampler_output {
                for trace in sample.traces {
                    if !(config.include_idle || trace.active) {
                        continue;
                    }

                    if config.gil_only && !trace.owns_gil {
                        continue;
                    }

                    let own_trace: StackTrace =
                        Into::<StackTraceWrapper>::into((trace.clone(), &backend_config)).into();

                    let stacktrace = own_trace + &ruleset.lock()?.clone();

                    buffer.lock()?.record(stacktrace)?;
                }
            }

            Ok(())
        }));

        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        log::trace!(target: LOG_TAG, "Shutting down sampler thread");

        self.running.store(false, Ordering::Relaxed);

        self.sampler_thread
            .ok_or_else(|| PyroscopeError::new("Pyspy: Failed to unwrap Sampler Thread"))?
            .join()
            .unwrap_or_else(|_| Err(PyroscopeError::new("Pyspy: Failed to join sampler thread")))?;

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        let report: StackBuffer = self.buffer.lock()?.deref().to_owned();
        let reports: Vec<Report> = report.into();

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

