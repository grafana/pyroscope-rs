use py_spy::{sampler::Sampler};
use pyroscope::{
    backend::{
        Backend, BackendConfig, BackendUninitialized, Report, ThreadTag, Ruleset,
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


    fn add_tag(&self, rule: ThreadTag) -> Result<()> {
        self.ruleset.lock()?.add_rule(rule)?;

        Ok(())
    }

    fn remove_tag(&self, rule: ThreadTag) -> Result<()> {
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

                    let stacktrace = own_trace.add_tag_rules(&*ruleset.lock()?);

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
        // Format name as "module.function" when module is available,
        // otherwise just use the function name.
        let formatted_name = match &frame.module {
            Some(module) => format!("{}.{}", module, frame.name),
            None => frame.name.clone(),
        };

        StackFrameWrapper(StackFrame {
            module: frame.module.clone(),
            name: Some(formatted_name),
            filename: Some(frame.filename.clone()),
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
        // https://github.com/python/cpython/blob/main/Python/thread_pthread.h#L304
        let thread_id = stack_trace.thread_id as libc::pthread_t;
        let stacktrace = StackTrace::new(
            config,
            Some(stack_trace.pid as u32),
            Some(thread_id.into()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frame(name: &str, filename: &str, module: Option<&str>, line: i32) -> py_spy::Frame {
        py_spy::Frame {
            name: name.to_string(),
            filename: filename.to_string(),
            module: module.map(|s| s.to_string()),
            short_filename: None,
            line,
            locals: None,
            is_entry: false,
            is_shim_entry: false,
        }
    }

    #[test]
    fn test_frame_name_with_module() {
        // When module is provided (e.g., native frames or class methods),
        // name should be "module.function"
        let frame = create_test_frame(
            "find_longest_match",
            "/usr/lib/python3.12/difflib.py",
            Some("SequenceMatcher"),
            42,
        );

        let wrapper: StackFrameWrapper = frame.into();
        let stack_frame: StackFrame = wrapper.into();

        assert_eq!(
            stack_frame.name,
            Some("SequenceMatcher.find_longest_match".to_string())
        );
        assert_eq!(
            stack_frame.module,
            Some("SequenceMatcher".to_string())
        );
        // filename preserves the full absolute path
        assert_eq!(
            stack_frame.filename,
            Some("/usr/lib/python3.12/difflib.py".to_string())
        );
        assert_eq!(stack_frame.line, Some(42));
    }

    #[test]
    fn test_frame_name_without_module() {
        // When module is None, name should just be the function name
        let frame = create_test_frame(
            "my_function",
            "/home/user/app/main.py",
            None,
            10,
        );

        let wrapper: StackFrameWrapper = frame.into();
        let stack_frame: StackFrame = wrapper.into();

        assert_eq!(
            stack_frame.name,
            Some("my_function".to_string())
        );
        assert_eq!(stack_frame.module, None);
        // filename preserves the full absolute path
        assert_eq!(
            stack_frame.filename,
            Some("/home/user/app/main.py".to_string())
        );
        assert_eq!(stack_frame.line, Some(10));
    }

    #[test]
    fn test_frame_absolute_path_preserved() {
        // absolute_path should always contain the full path
        let frame = create_test_frame(
            "test_func",
            "/path/to/file.py",
            None,
            1,
        );

        let wrapper: StackFrameWrapper = frame.into();
        let stack_frame: StackFrame = wrapper.into();

        assert_eq!(
            stack_frame.absolute_path,
            Some("/path/to/file.py".to_string())
        );
        assert_eq!(
            stack_frame.filename,
            Some("/path/to/file.py".to_string())
        );
        assert_eq!(stack_frame.relative_path, None);
    }

    #[test]
    fn test_name_never_contains_path_separator() {
        // The function name field should NEVER contain path separators
        // This would indicate the filename is leaking into the name
        let test_cases = vec![
            ("func1", "/a/b/c.py", None),
            ("func2", "/very/long/path/to/module.py", Some("MyClass")),
            ("func3", "relative/path.py", None),
        ];

        for (func_name, filename, module) in test_cases {
            let frame = create_test_frame(func_name, filename, module, 1);
            let wrapper: StackFrameWrapper = frame.into();
            let stack_frame: StackFrame = wrapper.into();

            let name = stack_frame.name.unwrap();
            assert!(
                !name.contains('/'),
                "Function name '{}' should not contain '/' path separator! Input: func={}, file={}, module={:?}",
                name, func_name, filename, module
            );
        }
    }
}

