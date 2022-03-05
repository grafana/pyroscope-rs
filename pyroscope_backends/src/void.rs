use super::error::{BackendError, Result};
use super::types::{Backend, BackendImpl, State};

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
        Ok(())
    }
    fn start(&mut self) -> Result<()> {
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }
    fn report(&mut self) -> Result<Vec<u8>> {
        let report = "void".to_string().into_bytes();

        Ok(report)
    }
}

pub fn void_backend(config: VoidConfig) -> BackendImpl<VoidBackend> {
    BackendImpl::new(VoidBackend::new(config))
}
