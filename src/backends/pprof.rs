// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use pprof::{ProfilerGuardBuilder, ProfilerGuard};

use crate::backends::Backend;
use crate::Result;

#[derive(Default)]
pub struct Pprof<'a> {
    inner_builder: Option<ProfilerGuardBuilder>,
    guard: Option<ProfilerGuard<'a>>,
}

impl Backend for Pprof<'_> {
    fn initialize(&mut self, sample_rate: i32) -> Result<()> {
        let profiler = ProfilerGuardBuilder::default()
            .frequency(sample_rate);
        self.inner_builder = Some(profiler);
        Ok(())
    }
    fn start(&mut self) -> Result<()> {
        let inner_builder = self.inner_builder.take().unwrap();
        self.guard = Some(inner_builder.clone().build()?);

        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        // drop the guard
        self.guard = None;

        Ok(())
    }
    fn report(&mut self) -> Result<()> {
        let report = self.guard.as_ref().unwrap().report().build()?;

        Ok(())
    }
}
