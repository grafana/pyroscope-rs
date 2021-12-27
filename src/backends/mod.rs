// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::Result;

///! Backend Trait

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Uninitialized,
    Ready,
    Running,
}

impl Default for State {
    fn default() -> Self { State::Uninitialized }
}

pub trait Backend: Send {
    fn get_state(&self) -> State;
    fn initialize(&mut self, sample_rate: i32) -> Result<()>;
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn report(&mut self) -> Result<Vec<u8>>;
}

pub mod pprof;
