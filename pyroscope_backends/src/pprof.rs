use pprof::{ProfilerGuard, ProfilerGuardBuilder, Report};

use super::error::{BackendError, Result};
use super::types::{Backend, State};

#[derive(Default)]
pub struct Pprof<'a> {
    inner_builder: Option<ProfilerGuardBuilder>,
    guard: Option<ProfilerGuard<'a>>,
    state: State,
}

impl std::fmt::Debug for Pprof<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Pprof Backend")
    }
}

impl Backend for Pprof<'_> {
    fn get_state(&self) -> State {
        self.state
    }

    fn initialize(&mut self, sample_rate: i32) -> Result<()> {
        // Check if Backend is Uninitialized
        if self.state != State::Uninitialized {
            return Err(BackendError::new("Pprof Backend is already Initialized"));
        }

        // Construct a ProfilerGuardBuilder
        let profiler = ProfilerGuardBuilder::default().frequency(sample_rate);
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

        let mut buffer = Vec::new();
        let report = self
            .guard
            .as_ref()
            .ok_or_else(|| BackendError::new("pprof-rs: ProfilerGuard report error"))?
            .report()
            .build()?;
        fold(&report, true, &mut buffer)?;

        // Restart Profiler
        self.stop()?;
        self.start()?;

        Ok(buffer)
    }
}

// Copyright: https://github.com/YangKeao
fn fold<W>(report: &Report, with_thread_name: bool, mut writer: W) -> Result<()>
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
            let last_frame = key.frames.len().saturating_sub(1);
            for (index, symbol) in frame.iter().rev().enumerate() {
                let last_symbol = frame.len().saturating_sub(1);
                if index == last_symbol {
                    write!(writer, "{}", symbol)?;
                } else {
                    write!(writer, "{};", symbol)?;
                }
            }

            if index != last_frame {
                write!(writer, ";")?;
            }
        }

        writeln!(writer, " {}", value)?;
    }

    Ok(())
}
