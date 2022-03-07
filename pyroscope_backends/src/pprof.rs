use std::collections::HashMap;
use std::ffi::OsStr;

use pprof::{ProfilerGuard, ProfilerGuardBuilder};

use crate::types::{Report, StackFrame, StackTrace};

use super::error::{BackendError, Result};
use super::types::{Backend, State};

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
    pub fn new(sample_rate: u32) -> Self {
        PprofConfig { sample_rate }
    }
}

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

    fn spy_name(&self) -> Result<String> {
        Ok("pyroscope-rs".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        // Check if Backend is Uninitialized
        if self.state != State::Uninitialized {
            return Err(BackendError::new("Pprof Backend is already Initialized"));
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
            return Err(BackendError::new("Pprof Backend is not Ready"));
        }

        self.guard = Some(
            self.inner_builder
                .as_ref()
                .ok_or_else(|| BackendError::new("pprof-rs: ProfilerGuardBuilder error"))?
                .clone()
                .build()?,
        );

        // Set State to Running
        self.state = State::Running;

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Pprof Backend is not Running"));
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
            return Err(BackendError::new("Pprof Backend is not Running"));
        }

        let report = self
            .guard
            .as_ref()
            .ok_or_else(|| BackendError::new("pprof-rs: ProfilerGuard report error"))?
            .report()
            .build()?;

        let new_report = Into::<Report>::into(report).to_string().into_bytes();

        // Restart Profiler
        self.stop()?;
        self.start()?;

        Ok(new_report)
    }
}

// Copyright: https://github.com/YangKeao
fn fold<W>(report: &pprof::Report, with_thread_name: bool, mut writer: W) -> Result<()>
where
    W: std::io::Write,
{
    for (key, value) in report.data.iter() {
        if with_thread_name {
            if !key.thread_name.is_empty() {
                write!(writer, "{};", key.thread_name)?;
            } else {
                write!(writer, "{:?};", key.thread_id)?;
            }
        }

        for (index, frame) in key.frames.iter().rev().enumerate() {
            for (index, symbol) in frame.iter().rev().enumerate() {
                if index + 1 == frame.len() {
                    write!(writer, "{}", symbol)?;
                } else {
                    write!(writer, "{};", symbol)?;
                }
            }

            if index + 1 != key.frames.len() {
                write!(writer, ";")?;
            }
        }

        writeln!(writer, " {}", value)?;
    }

    Ok(())
}

impl From<pprof::Report> for Report {
    fn from(report: pprof::Report) -> Self {
        //convert report to Report
        let report_data: HashMap<StackTrace, usize> = report
            .data
            .iter()
            .map(|(key, value)| (key.to_owned().into(), value.to_owned() as usize))
            .collect();
        Report::new(report_data)
    }
}

impl From<pprof::Frames> for StackTrace {
    fn from(frames: pprof::Frames) -> Self {
        StackTrace::new(
            None,
            Some(frames.thread_id),
            Some(frames.thread_name),
            frames
                .frames
                .concat()
                .iter()
                .map(|frame| frame.to_owned().into())
                .collect(),
        )
    }
}

impl From<pprof::Symbol> for StackFrame {
    fn from(symbol: pprof::Symbol) -> Self {
        StackFrame::new(
            None,
            Some(symbol.name()),
            Some(
                symbol
                    .filename
                    .clone()
                    .unwrap_or(std::path::PathBuf::new())
                    .file_name()
                    .unwrap_or(OsStr::new(""))
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            ),
            Some(
                symbol
                    .filename
                    .unwrap_or(std::path::PathBuf::new())
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            ),
            None,
            symbol.lineno,
        )
    }
}
