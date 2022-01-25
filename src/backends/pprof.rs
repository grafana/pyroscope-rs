// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use pprof::{ProfilerGuard, ProfilerGuardBuilder, Report};

use crate::backends::Backend;
use crate::backends::State;
use crate::PyroscopeError;
use crate::Result;

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
            return Err(PyroscopeError::new("Pprof Backend is already Initialized"));
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
            return Err(PyroscopeError::new("Pprof Backend is not Ready"));
        }

        self.guard = Some(self.inner_builder.as_ref().unwrap().clone().build()?);

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

        let mut buffer = Vec::new();
        let report = self.guard.as_ref().unwrap().report().build()?;
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

        let last_frame = key.frames.len() - 1;
        for (index, frame) in key.frames.iter().rev().enumerate() {
            let last_symbol = frame.len() - 1;
            for (index, symbol) in frame.iter().rev().enumerate() {
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
