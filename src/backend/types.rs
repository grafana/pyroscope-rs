use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use crate::error::Result;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl Tag {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
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
