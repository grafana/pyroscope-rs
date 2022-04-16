use super::error::Result;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Tag {
    key: String,
    value: String,
}

impl Tag {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Rule {
    GlobalTag(Tag),
    ThreadTag(Tag),
}

#[derive(Debug)]
pub struct Ruleset {
    rules: Arc<Mutex<Vec<Rule>>>,
}

impl Ruleset {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn add_rule(&self, rule: Rule) {
        let mut rules = self.rules.lock().unwrap();
        rules.push(rule);
    }

    pub fn remove_rule(&self, rule: Rule) {
        let mut rules = self.rules.lock().unwrap();
        rules.retain(|r| r != &rule);
    }
}

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

    fn add_ruleset(&mut self, ruleset: Rule) -> Result<()>;
    fn remove_ruleset(&mut self, ruleset: Rule) -> Result<()>;
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

/// Stack buffer
#[derive(Debug, Default, Clone)]
pub struct StackBuffer {
    pub data: HashMap<StackTrace, usize>,
}

impl StackBuffer {
    pub fn new(data: HashMap<StackTrace, usize>) -> Self {
        Self { data }
    }

    pub fn record(&mut self, stack_trace: StackTrace) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += 1;

        Ok(())
    }

    pub fn record_with_count(&mut self, stack_trace: StackTrace, count: usize) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += count;

        Ok(())
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl From<StackBuffer> for Vec<Report> {
    fn from(stack_buffer: StackBuffer) -> Self {
        let ss: HashMap<usize, Report> =
            stack_buffer
                .data
                .iter()
                .fold(HashMap::new(), |mut acc, (k, v)| {
                    if let Some(report) = acc.get_mut(&k.metadata.get_id()) {
                        report.record_with_count(k.to_owned(), v.to_owned());
                    } else {
                        let stacktrace = k.to_owned();
                        let report = Report::new(HashMap::new());
                        let mut report = report.metadata(stacktrace.metadata.clone());
                        report.record(stacktrace);
                        acc.insert(k.metadata.get_id(), report);
                    }
                    acc
                });
        // convert ss to vector
        ss.iter().map(|(_, v)| v.clone()).collect()
    }
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct Metadata {
    pub tags: Vec<Tag>,
}

impl Metadata {
    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.push(tag);
    }
    pub fn get_id(&self) -> usize {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish() as usize
    }
}

/// Report
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub data: HashMap<StackTrace, usize>,
    pub metadata: Metadata,
}

impl Hash for Report {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.metadata.hash(state);
    }
}

impl Report {
    pub fn new(data: HashMap<StackTrace, usize>) -> Self {
        Self {
            data,
            metadata: Metadata::default(),
        }
    }

    pub fn metadata(self, metadata: Metadata) -> Self {
        Self {
            data: self.data,
            metadata,
        }
    }

    pub fn record(&mut self, stack_trace: StackTrace) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += 1;

        Ok(())
    }

    pub fn record_with_count(&mut self, stack_trace: StackTrace, count: usize) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += count;

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
    /// Metadata
    pub metadata: Metadata,
}

impl StackTrace {
    /// Create a new StackTrace
    pub fn new(
        pid: Option<u32>, thread_id: Option<u64>, thread_name: Option<String>,
        frames: Vec<StackFrame>,
    ) -> Self {
        let mut metadata = Metadata::default();
        if let Some(pid) = pid {
            metadata.add_tag(Tag::new("pid".to_owned(), pid.to_string()));
        }
        if let Some(thread_id) = thread_id {
            metadata.add_tag(Tag::new("thread_id".to_owned(), thread_id.to_string()));
        }
        if let Some(thread_name) = thread_name.clone() {
            metadata.add_tag(Tag::new("thread_name".to_owned(), thread_name));
        }
        Self {
            pid,
            thread_id,
            thread_name,
            frames,
            metadata,
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

/// Generate a dummy stack trace
fn generate_stack_trace() -> Result<Vec<StackTrace>> {
    let frames = vec![StackFrame::new(
        None,
        Some("void".to_string()),
        Some("void.rs".to_string()),
        None,
        None,
        Some(0),
    )];
    let stack_trace_1 = StackTrace::new(None, Some(1), None, frames.clone());

    let stack_trace_2 = StackTrace::new(None, Some(2), None, frames);

    Ok(vec![stack_trace_1, stack_trace_2])
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
    buffer: StackBuffer,
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
        let stack_traces = generate_stack_trace()?;

        // Add the StackTrace to the buffer
        for stack_trace in stack_traces {
            self.buffer.record(stack_trace)?;
        }

        Ok(())
    }

    fn shutdown(self) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        let reports = self.buffer.clone().into();

        Ok(reports)
    }

    fn add_ruleset(&mut self, _ruleset: Rule) -> Result<()> {
        Ok(())
    }
    fn remove_ruleset(&mut self, _ruleset: Rule) -> Result<()> {
        Ok(())
    }
}

pub fn void_backend(config: VoidConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Arc::new(Mutex::new(VoidBackend::new(config))))
}

#[cfg(test)]
mod ruleset_tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        let tag = Tag::new("key".to_string(), "value".to_string());

        assert_eq!(tag.key, "key");
        assert_eq!(tag.value, "value");
    }

    #[test]
    fn test_rule_new() {
        let rule = Rule::ThreadTag(Tag::new("key".to_string(), "value".to_string()));

        assert_eq!(
            rule,
            Rule::ThreadTag(Tag::new("key".to_string(), "value".to_string()))
        );
    }

    #[test]
    fn test_ruleset_new() {
        let ruleset = Ruleset::new();

        assert_eq!(ruleset.rules.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_ruleset_add_rule() {
        let ruleset = Ruleset::new();

        let rule = Rule::ThreadTag(Tag::new("key".to_string(), "value".to_string()));

        ruleset.add_rule(rule);

        assert_eq!(ruleset.rules.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_ruleset_remove_rule() {
        let ruleset = Ruleset::new();

        let add_rule = Rule::ThreadTag(Tag::new("key".to_string(), "value".to_string()));

        ruleset.add_rule(add_rule);

        assert_eq!(ruleset.rules.lock().unwrap().len(), 1);

        let remove_rule = Rule::ThreadTag(Tag::new("key".to_string(), "value".to_string()));

        ruleset.remove_rule(remove_rule);

        assert_eq!(ruleset.rules.lock().unwrap().len(), 0);
    }
}
pub fn merge_tags_with_app_name(application_name: String, tags: Vec<Tag>) -> Result<String> {
    let mut merged_tags = String::new();

    if tags.is_empty() {
        return Ok(application_name);
    }

    for tag in tags {
        merged_tags.push_str(&format!("{}={},", tag.key, tag.value));
    }

    Ok(format!("{}{{{}}}", application_name, merged_tags))
}
