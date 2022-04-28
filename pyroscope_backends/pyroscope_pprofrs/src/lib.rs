use pprof::{ProfilerGuard, ProfilerGuardBuilder};
use pyroscope::{
    backend::{
        Backend, BackendImpl, BackendUninitialized, Report, Rule, Ruleset, StackBuffer, StackFrame,
        StackTrace,
    },
    error::{PyroscopeError, Result},
};
use std::{
    collections::HashMap,
    ffi::OsStr,
    ops::Deref,
    sync::{Arc, Mutex},
};

pub fn pprof_backend(config: PprofConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(Pprof::new(config)))
}

/// Pprof Configuration
#[derive(Debug)]
pub struct PprofConfig {
    sample_rate: u32,
}

impl Default for PprofConfig {
    fn default() -> Self {
        PprofConfig { sample_rate: 100 }
    }
}

impl PprofConfig {
    pub fn new() -> Self {
        PprofConfig::default()
    }

    pub fn sample_rate(self, sample_rate: u32) -> Self {
        PprofConfig { sample_rate }
    }
}

/// Pprof Backend
#[derive(Default)]
pub struct Pprof<'a> {
    /// Profling buffer
    buffer: Arc<Mutex<StackBuffer>>,
    /// pprof-rs Configuration
    config: PprofConfig,
    /// pprof-rs profiler Guard Builder
    inner_builder: Arc<Mutex<Option<ProfilerGuardBuilder>>>,
    /// pprof-rs profiler Guard
    guard: Arc<Mutex<Option<ProfilerGuard<'a>>>>,
    /// Ruleset
    ruleset: Ruleset,
}

impl std::fmt::Debug for Pprof<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Pprof Backend")
    }
}

impl<'a> Pprof<'a> {
    pub fn new(config: PprofConfig) -> Self {
        Pprof {
            buffer: Arc::new(Mutex::new(StackBuffer::default())),
            config,
            inner_builder: Arc::new(Mutex::new(None)),
            guard: Arc::new(Mutex::new(None)),
            ruleset: Ruleset::default(),
        }
    }
}

impl Backend for Pprof<'_> {
    fn spy_name(&self) -> std::result::Result<String, PyroscopeError> {
        Ok("pyroscope-rs".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        //drop(self.guard.take());

        Ok(())
    }

    fn initialize(&mut self) -> Result<()> {
        // Construct a ProfilerGuardBuilder
        let profiler = ProfilerGuardBuilder::default().frequency(self.config.sample_rate as i32);

        // Set inner_builder field
        *self.inner_builder.lock()? = Some(profiler);

        *self.guard.lock()? = Some(
            self.inner_builder
                .lock()?
                .as_ref()
                .ok_or_else(|| PyroscopeError::new("pprof-rs: ProfilerGuardBuilder error"))?
                .clone()
                .build()
                .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?,
        );

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        self.dump_report()?;

        let buffer = self.buffer.clone();

        let report: StackBuffer = buffer.lock()?.deref().to_owned();

        let reports: Vec<Report> = report.into();

        buffer.lock()?.clear();

        Ok(reports)
    }

    fn add_rule(&self, rule: Rule) -> Result<()> {
        if self.guard.lock()?.as_ref().is_some() {
            self.dump_report()?;
        }

        self.ruleset.add_rule(rule)?;

        Ok(())
    }
    fn remove_rule(&self, rule: Rule) -> Result<()> {
        if self.guard.lock()?.as_ref().is_some() {
            self.dump_report()?;
        }

        self.ruleset.remove_rule(rule)?;

        Ok(())
    }
}

impl Pprof<'_> {
    pub fn dump_report(&self) -> Result<()> {
        let report = self
            .guard
            .lock()?
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("pprof-rs: ProfilerGuard report error"))?
            .report()
            .build()
            .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?;

        let stack_buffer = Into::<StackBuffer>::into(Into::<StackBufferWrapper>::into(report));

        // apply ruleset to stack_buffer
        let data: HashMap<StackTrace, usize> = stack_buffer
            .data
            .iter()
            .map(|(stacktrace, ss)| {
                let stacktrace = stacktrace.to_owned() + &self.ruleset;
                (stacktrace, ss.to_owned())
            })
            .collect();

        let buffer = self.buffer.clone();

        for (stacktrace, count) in data {
            buffer.lock()?.record_with_count(stacktrace, count)?;
        }

        self.reset()?;

        Ok(())
    }

    pub fn reset(&self) -> Result<()> {
        drop(self.guard.lock()?.take());

        *self.guard.lock()? = Some(
            self.inner_builder
                .lock()?
                .as_ref()
                .ok_or_else(|| PyroscopeError::new("pprof-rs: ProfilerGuardBuilder error"))?
                .clone()
                .build()
                .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?,
        );

        Ok(())
    }
}

struct ReportWrapper(Report);

impl From<ReportWrapper> for Report {
    fn from(report: ReportWrapper) -> Self {
        report.0
    }
}

impl From<pprof::Report> for ReportWrapper {
    fn from(report: pprof::Report) -> Self {
        //convert report to Report
        let report_data: HashMap<StackTrace, usize> = report
            .data
            .iter()
            .map(|(key, value)| {
                (
                    Into::<StackTraceWrapper>::into(key.to_owned()).into(),
                    value.to_owned() as usize,
                )
            })
            .collect();
        ReportWrapper(Report::new(report_data))
    }
}

struct StackBufferWrapper(StackBuffer);

impl From<StackBufferWrapper> for StackBuffer {
    fn from(stackbuffer: StackBufferWrapper) -> Self {
        stackbuffer.0
    }
}

impl From<pprof::Report> for StackBufferWrapper {
    fn from(report: pprof::Report) -> Self {
        //convert report to stackbuffer
        let buffer_data: HashMap<StackTrace, usize> = report
            .data
            .iter()
            .map(|(key, value)| {
                (
                    Into::<StackTraceWrapper>::into(key.to_owned()).into(),
                    value.to_owned() as usize,
                )
            })
            .collect();
        StackBufferWrapper(StackBuffer::new(buffer_data))
    }
}

struct StackTraceWrapper(StackTrace);

impl From<StackTraceWrapper> for StackTrace {
    fn from(stack_trace: StackTraceWrapper) -> Self {
        stack_trace.0
    }
}

impl From<pprof::Frames> for StackTraceWrapper {
    fn from(frames: pprof::Frames) -> Self {
        StackTraceWrapper(StackTrace::new(
            None,
            Some(frames.thread_id),
            Some(frames.thread_name),
            frames
                .frames
                .concat()
                .iter()
                .map(|frame| Into::<StackFrameWrapper>::into(frame.to_owned()).into())
                .collect(),
        ))
    }
}

struct StackFrameWrapper(StackFrame);

impl From<StackFrameWrapper> for StackFrame {
    fn from(stack_frame: StackFrameWrapper) -> Self {
        stack_frame.0
    }
}

impl From<pprof::Symbol> for StackFrameWrapper {
    fn from(symbol: pprof::Symbol) -> Self {
        StackFrameWrapper(StackFrame::new(
            None,
            Some(symbol.name()),
            Some(
                symbol
                    .filename
                    .clone()
                    .unwrap_or_else(std::path::PathBuf::new)
                    .file_name()
                    .unwrap_or_else(|| OsStr::new(""))
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            ),
            Some(
                symbol
                    .filename
                    .unwrap_or_else(std::path::PathBuf::new)
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            ),
            None,
            symbol.lineno,
        ))
    }
}
