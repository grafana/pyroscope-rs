use crate::Result;

use std::fmt::Debug;

/// Backend State
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// Backend is uninitialized.
    Uninitialized,
    /// Backend is ready to be used.
    Ready,
    /// Backend is running.
    Running,
}

impl Default for State {
    fn default() -> Self {
        State::Uninitialized
    }
}

/// Backend Trait
pub trait Backend: Send + Debug {
    /// Get the backend state.
    fn get_state(&self) -> State;
    /// Initialize the backend.
    fn initialize(&mut self, sample_rate: i32) -> Result<()>;
    /// Start the backend.
    fn start(&mut self) -> Result<()>;
    /// Stop the backend.
    fn stop(&mut self) -> Result<()>;
    /// Generate profiling report
    fn report(&mut self) -> Result<Vec<u8>>;
}

pub mod pprof;
