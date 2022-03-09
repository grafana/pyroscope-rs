use super::error::{BackendError, Result};
use super::types::{Backend, BackendImpl, State};
use crate::types::{Report, StackFrame, StackTrace};

#[derive(Debug)]
pub struct PhpspyConfig {
    pid: i32,
    sample_rate: u32,
}

impl Default for PhpspyConfig {
    fn default() -> Self {
        Self {
            pid: 0,
            sample_rate: 100u32,
        }
    }
}

impl PhpspyConfig {
    pub fn new(pid: i32) -> Self {
        Self {
            pid,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct PhpspyBackend {
    state: State,
    config: PhpspyConfig,
    buffer: Report,
}

impl PhpspyBackend {
    pub fn new(config: PhpspyConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

impl Backend for PhpspyBackend {
    fn get_state(&self) -> State {
        self.state
    }

    fn spy_name(&self) -> Result<String> {
        Ok("phpspy".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}
