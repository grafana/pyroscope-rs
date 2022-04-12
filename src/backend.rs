use super::error::Result;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

/// Backend Trait
pub trait Backend: Send + Debug {
    /// Backend Spy Name
    fn spy_name(&self) -> Result<String>;
    /// Get backend configuration.
    fn sample_rate(&self) -> Result<u32>;
    /// Initialize the backend.
    fn initialize(&mut self) -> Result<()>;
    /// Drop the backend.
    fn shutdown(self) -> Result<()>;
    /// Generate profiling report
    fn report(&mut self) -> Result<Vec<Report>>;
}

#[derive(Debug)]
pub struct BackendUninitialized;
#[derive(Debug)]
pub struct BackendReady;

pub trait BackendState {}
impl BackendState for BackendUninitialized {}
impl BackendState for BackendReady {}

#[derive(Debug)]
pub struct BackendImpl<S: BackendState + ?Sized> {
    pub backend: Arc<Mutex<dyn Backend>>,
    _state: std::marker::PhantomData<S>,
}

impl<S: BackendState> BackendImpl<S> {
    pub fn spy_name(&self) -> Result<String> {
        self.backend.lock()?.spy_name()
    }
    pub fn sample_rate(&self) -> Result<u32> {
        self.backend.lock()?.sample_rate()
    }
}

impl BackendImpl<BackendUninitialized> {
    pub fn new(backend: Arc<Mutex<dyn Backend>>) -> Self {
        Self {
            backend,
            _state: std::marker::PhantomData,
        }
    }

    pub fn initialize(self) -> Result<BackendImpl<BackendReady>> {
        let backend = self.backend.clone();
        backend.lock()?.initialize()?;

        Ok(BackendImpl {
            backend,
            _state: std::marker::PhantomData,
        })
    }
}
impl BackendImpl<BackendReady> {
    pub fn shutdown(self) -> Result<()> {
        //let backend = self.backend.clone();
        //backend.lock()?.shutdown()?;
        Ok(())
    }
    pub fn report(&mut self) -> Result<Vec<Report>> {
        self.backend.lock()?.report()
    }
}

/// Report
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub data: HashMap<StackTrace, usize>,
}

impl Report {
    pub fn new(data: HashMap<StackTrace, usize>) -> Self {
        Self { data }
    }

    pub fn record(&mut self, stack_trace: StackTrace) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += 1;

        Ok(())
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let collpased = self
            .data
            .iter()
            .map(|(k, v)| format!("{} {}", k, v))
            .collect::<Vec<String>>();

        write!(f, "{}", collpased.join("\n"))
    }
}

/// StackTrace
/// A representation of a stack trace.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct StackTrace {
    /// Process ID
    pub pid: Option<u32>,
    /// Thread ID
    pub thread_id: Option<u64>,
    /// Thread Name
    pub thread_name: Option<String>,
    /// Stack Trace
    pub frames: Vec<StackFrame>,
}

impl StackTrace {
    /// Create a new StackTrace
    pub fn new(
        pid: Option<u32>, thread_id: Option<u64>, thread_name: Option<String>,
        frames: Vec<StackFrame>,
    ) -> Self {
        Self {
            pid,
            thread_id,
            thread_name,
            frames,
        }
    }
}

/// StackFrame
/// A representation of a stack frame.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct StackFrame {
    /// Module name
    pub module: Option<String>,
    /// Function name
    pub name: Option<String>,
    /// File name
    pub filename: Option<String>,
    /// File relative path
    pub relative_path: Option<String>,
    /// File absolute path
    pub absolute_path: Option<String>,
    /// Line number
    pub line: Option<u32>,
}

impl StackFrame {
    /// Create a new StackFrame.
    pub fn new(
        module: Option<String>, name: Option<String>, filename: Option<String>,
        relative_path: Option<String>, absolute_path: Option<String>, line: Option<u32>,
    ) -> Self {
        Self {
            module,
            name,
            filename,
            relative_path,
            absolute_path,
            line,
        }
    }
}

impl std::fmt::Display for StackFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} - {}",
            self.filename.as_ref().unwrap_or(&"".to_string()),
            self.line.unwrap_or(0),
            self.name.as_ref().unwrap_or(&"".to_string())
        )
    }
}

impl std::fmt::Display for StackTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &self
                .frames
                .iter()
                .rev()
                .map(|frame| format!("{}", frame))
                .collect::<Vec<_>>()
                .join(";")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_frame_display() {
        let frame = StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(1),
        );

        assert_eq!(format!("{}", frame), "filename:1 - name");
    }

    #[test]
    fn test_stack_trace_display() {
        let mut frames = Vec::new();
        frames.push(StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(1),
        ));
        frames.push(StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(2),
        ));

        let stack_trace = StackTrace::new(None, None, None, frames);

        assert_eq!(
            format!("{}", stack_trace),
            "filename:2 - name;filename:1 - name"
        );
    }

    #[test]
    fn test_report_record() {
        let mut report = Report::new(HashMap::new());

        let stack_trace = StackTrace::new(None, None, None, vec![]);

        assert!(report.record(stack_trace).is_ok());
        assert_eq!(report.data.len(), 1);
    }

    #[test]
    fn test_report_clear() {
        let mut report = Report::new(HashMap::new());

        let stack_trace = StackTrace::new(None, None, None, vec![]);

        assert!(report.record(stack_trace).is_ok());

        report.clear();

        assert_eq!(report.data.len(), 0);
    }

    #[test]
    fn test_report_display() {
        // Dummy StackTrace
        let mut frames = Vec::new();
        frames.push(StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("absolute_path".to_string()),
            Some("relative_path".to_string()),
            Some(1),
        ));
        frames.push(StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("absolute_path".to_string()),
            Some("relative_path".to_string()),
            Some(2),
        ));
        let stack_trace = StackTrace::new(None, None, None, frames);

        let mut report = Report::new(HashMap::new());

        report.record(stack_trace.clone()).unwrap();
        report.record(stack_trace).unwrap();

        assert_eq!(
            format!("{}", report),
            "filename:2 - name;filename:1 - name 2"
        );
    }
}

#[derive(Debug)]
pub struct VoidConfig {
    sample_rate: u32,
}

impl Default for VoidConfig {
    fn default() -> Self {
        Self {
            sample_rate: 100u32,
        }
    }
}
impl VoidConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample_rate(self, sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

/// Empty Backend implementation for Testing purposes
#[derive(Debug, Default)]
pub struct VoidBackend {
    config: VoidConfig,
    buffer: Report,
}

impl VoidBackend {
    pub fn new(config: VoidConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

impl Backend for VoidBackend {
    fn spy_name(&self) -> Result<String> {
        Ok("void".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        // Generate a dummy Stack Trace
        let stack_trace = generate_stack_trace()?;

        // Add the StackTrace to the buffer
        self.buffer.record(stack_trace)?;

        Ok(())
    }

    fn shutdown(self) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        let reports = vec![self.buffer.clone()];

        Ok(reports)
    }
}

pub fn void_backend(config: VoidConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Arc::new(Mutex::new(VoidBackend::new(config))))
}

/// Generate a dummy stack trace
fn generate_stack_trace() -> Result<StackTrace> {
    let frames = vec![StackFrame::new(
        None,
        Some("void".to_string()),
        Some("void.rs".to_string()),
        None,
        None,
        Some(0),
    )];
    let stack_trace = StackTrace::new(None, None, None, frames);

    Ok(stack_trace)
}
