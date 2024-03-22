use std::{
    collections::{hash_map::DefaultHasher, BTreeSet, HashMap},
    hash::{Hash, Hasher},
};

use crate::error::Result;

use super::BackendConfig;

/// Pyroscope Tag
#[derive(Debug, PartialOrd, Ord, Eq, PartialEq, Hash, Clone)]
pub struct Tag {
    /// Tag key
    pub key: String,
    /// Tag value
    pub value: String,
}

impl Tag {
    /// Create a new Tag
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

/// Stack buffer
#[derive(Debug, Default, Clone)]
pub struct StackBuffer {
    /// Buffer data bucket
    pub data: HashMap<StackTrace, usize>,
}

impl StackBuffer {
    /// Create a new StackBuffer with the given data
    pub fn new(data: HashMap<StackTrace, usize>) -> Self {
        Self { data }
    }

    /// Record a new stack trace
    pub fn record(&mut self, stack_trace: StackTrace) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += 1;

        Ok(())
    }

    /// Record a new stack trace with count
    pub fn record_with_count(&mut self, stack_trace: StackTrace, count: usize) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += count;

        Ok(())
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// Split a Stack Buffer into Reports
impl From<StackBuffer> for Vec<Report> {
    fn from(stack_buffer: StackBuffer) -> Self {
        stack_buffer
            .data
            .into_iter()
            .fold(
                Ok(HashMap::new()),
                |acc: Result<HashMap<usize, Report>>, (stacktrace, count): (StackTrace, usize)| {
                    let mut acc = acc?;
                    // if a report exists for this stacktrace, add the count to it
                    if let Some(report) = acc.get_mut(&stacktrace.metadata.get_id()) {
                        // record the count
                        report.record_with_count(stacktrace, count)?;
                    // if no report exists,
                    } else {
                        // create a new report
                        let report = Report::new(HashMap::new());
                        let report_id = stacktrace.metadata.get_id();
                        // set the metadata of the report, from the stacktrace own metadata.
                        let mut report = report.metadata(stacktrace.metadata.clone());
                        // record the stacktrace. The count should be 1.
                        report.record_with_count(stacktrace, count)?;
                        // add the report to the accumulator.
                        acc.insert(report_id, report);
                    }
                    // return the accumulator
                    Ok(acc)
                },
            )
            .unwrap_or_default()
            .into_values()
            .collect()
    }
}

/// Metdata
/// Metadata attached to a StackTrace or a Report. For now, this is just tags.
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct Metadata {
    /// Tags
    pub tags: BTreeSet<Tag>,
}

impl Metadata {
    /// Add a tag to the metadata
    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.insert(tag);
    }

    /// Get the id of the metadata. This uses the hash of the Metadata type.
    pub fn get_id(&self) -> usize {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish() as usize
    }
}

/// Report
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// Report StackTraces
    pub data: HashMap<StackTrace, usize>,
    /// Metadata
    pub metadata: Metadata,
}

#[derive(Debug)]
pub struct EncodedReport {
    pub format: String,
    pub content_type: String,
    pub content_encoding: String,
    pub data: Vec<u8>,
    pub metadata: Metadata,
}

/// Custom implementation of the Hash trait for Report.
/// Only the metadata is hashed.
impl Hash for Report {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.metadata.hash(state);
    }
}

impl Report {
    /// Create a new Report.
    pub fn new(data: HashMap<StackTrace, usize>) -> Self {
        Self {
            data,
            metadata: Metadata::default(),
        }
    }

    /// Return an iterator over the StackTraces of the Report.
    pub fn iter(&self) -> impl Iterator<Item = (&StackTrace, &usize)> {
        self.data.iter()
    }

    /// Set the metadata of the report.
    pub fn metadata(self, metadata: Metadata) -> Self {
        Self {
            data: self.data,
            metadata,
        }
    }

    /// Record a new stack trace.
    pub fn record(&mut self, stack_trace: StackTrace) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += 1;

        Ok(())
    }

    /// Record a new stack trace with count.
    pub fn record_with_count(&mut self, stack_trace: StackTrace, count: usize) -> Result<()> {
        *self.data.entry(stack_trace).or_insert(0) += count;

        Ok(())
    }

    /// Clear the report data buffer.
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

impl StackTrace {
    /// Create a new StackTrace
    pub fn new(
        config: &BackendConfig, pid: Option<u32>, thread_id: Option<u64>,
        thread_name: Option<String>, frames: Vec<StackFrame>,
    ) -> Self {
        // Set StackTrace specific tags
        let mut metadata = Metadata::default();

        if config.report_pid {
            if let Some(pid) = pid {
                metadata.add_tag(Tag::new("pid".to_owned(), pid.to_string()));
            }
        }

        if config.report_thread_id {
            if let Some(thread_id) = thread_id {
                metadata.add_tag(Tag::new("thread_id".to_owned(), thread_id.to_string()));
            }
        }

        if config.report_thread_name {
            if let Some(thread_name) = thread_name.clone() {
                metadata.add_tag(Tag::new("thread_name".to_owned(), thread_name));
            }
        }

        Self {
            pid,
            thread_id,
            thread_name,
            frames,
            metadata,
        }
    }

    /// Return an iterator over the frames of the stacktrace.
    pub fn iter(&self) -> impl Iterator<Item = &StackFrame> {
        self.frames.iter()
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
