// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::backends::backend::Backend;
use crate::Result;

pub struct Pprof {}

impl Pprof {
    pub fn new() -> Self {
        Self {}
    }
}

impl Backend for Pprof {
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    fn start(&mut self) -> Result<()> {
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }
    fn report(&mut self) -> Result<()> {
        Ok(())
    }
}
