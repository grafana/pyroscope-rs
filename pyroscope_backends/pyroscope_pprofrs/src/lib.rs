use pprof::{ProfilerGuard, ProfilerGuardBuilder};
use pyroscope::{
    backend::{Backend, Report, StackFrame, StackTrace, State},
    error::{PyroscopeError, Result},
};
use std::{collections::HashMap, ffi::OsStr};

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
    config: PprofConfig,
    inner_builder: Option<ProfilerGuardBuilder>,
    guard: Option<ProfilerGuard<'a>>,
    state: State,
}

impl std::fmt::Debug for Pprof<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Pprof Backend")
    }
}

impl<'a> Pprof<'a> {
    pub fn new(config: PprofConfig) -> Self {
        Pprof {
            config,
            inner_builder: None,
            guard: None,
            state: State::default(),
        }
    }
}

impl Backend for Pprof<'_> {
    fn get_state(&self) -> State {
        self.state
    }

    fn spy_name(&self) -> std::result::Result<String, PyroscopeError> {
        Ok("pyroscope-rs".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        // Check if Backend is Uninitialized
        if self.state != State::Uninitialized {
            return Err(PyroscopeError::new("Pprof Backend is already Initialized"));
        }

        // Construct a ProfilerGuardBuilder
        let profiler = ProfilerGuardBuilder::default().frequency(self.config.sample_rate as i32);
        // Set inner_builder field
        self.inner_builder = Some(profiler);

        // Set State to Ready
        self.state = State::Ready;

        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        // Check if Backend is Ready
        if self.state != State::Ready {
            return Err(PyroscopeError::new("Pprof Backend is not Ready"));
        }

        self.guard = Some(
            self.inner_builder
                .as_ref()
                .ok_or_else(|| PyroscopeError::new("pprof-rs: ProfilerGuardBuilder error"))?
                .clone()
                .build()
                .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?,
        );

        // Set State to Running
        self.state = State::Running;

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(PyroscopeError::new("Pprof Backend is not Running"));
        }

        // drop the guard
        drop(self.guard.take());

        // Set State to Ready
        self.state = State::Ready;

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(PyroscopeError::new("Pprof Backend is not Running"));
        }

        let report = self
            .guard
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("pprof-rs: ProfilerGuard report error"))?
            .report()
            .build()
            .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?;

        let new_report = Into::<Report>::into(Into::<ReportWrapper>::into(report))
            .to_string()
            .into_bytes();

        // Restart Profiler
        self.stop()?;
        self.start()?;

        Ok(new_report)
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
