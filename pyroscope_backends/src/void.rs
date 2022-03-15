use crate::types::{Report, StackFrame, StackTrace};

use super::{
    error::Result,
    types::{Backend, BackendImpl, State},
};

#[derive(Debug)]
pub struct VoidConfig {
    sample_rate: u32,
}

impl Default for VoidConfig {
    fn default() -> Self {
        Self {
            sample_rate: 100u32,
        }
    }
}
impl VoidConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample_rate(self, sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

#[derive(Debug, Default)]
pub struct VoidBackend {
    state: State,
    config: VoidConfig,
    buffer: Report,
}

impl VoidBackend {
    pub fn new(config: VoidConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

impl Backend for VoidBackend {
    fn get_state(&self) -> State {
        self.state
    }

    fn spy_name(&self) -> Result<String> {
        Ok("void".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        // Generate a dummy Stack Trace
        let stack_trace = generate_stack_trace()?;

        // Add the StackTrace to the buffer
        self.buffer.record(stack_trace)?;

        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        let report = self.buffer.to_string().into_bytes();

        Ok(report)
    }
}

pub fn void_backend(config: VoidConfig) -> BackendImpl<VoidBackend> {
    BackendImpl::new(VoidBackend::new(config))
}

/// Generate a dummy stack trace
fn generate_stack_trace() -> Result<StackTrace> {
    let frames = vec![StackFrame::new(
        None,
        Some("void".to_string()),
        Some("void.rs".to_string()),
        None,
        None,
        Some(0),
    )];
    let stack_trace = StackTrace::new(None, None, None, frames);

    Ok(stack_trace)
}
