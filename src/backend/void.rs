use super::{
    Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, Rule, Ruleset, StackBuffer,
    StackFrame, StackTrace,
};
use crate::error::Result;

/// Generate a dummy stack trace
fn generate_stack_trace() -> Result<Vec<StackTrace>> {
    let frames = vec![StackFrame::new(
        None,
        Some("void".to_string()),
        Some("void.rs".to_string()),
        None,
        None,
        Some(0),
    )];
    let stack_trace_1 = StackTrace::new(
        &BackendConfig::default(),
        None,
        Some(1),
        None,
        frames.clone(),
    );

    let stack_trace_2 = StackTrace::new(&BackendConfig::default(), None, Some(2), None, frames);

    Ok(vec![stack_trace_1, stack_trace_2])
}

#[derive(Debug)]
pub struct VoidConfig {
    /// Sample rate.
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
    /// Create a new VoidConfig
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sample rate
    pub fn sample_rate(self, sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

/// Empty Backend implementation for Testing purposes
#[derive(Debug, Default)]
pub struct VoidBackend {
    /// Backend configuration
    config: VoidConfig,
    /// Backend StackTrace Buffer
    buffer: StackBuffer,
    /// Backend RuleSet
    ruleset: Ruleset,
}

impl VoidBackend {
    /// Create a new VoidBackend
    pub fn new(config: VoidConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

impl Backend for VoidBackend {
    /// Return the Backend name.
    fn spy_name(&self) -> Result<String> {
        Ok("void".to_string())
    }

    /// Return the Backend extension.
    fn spy_extension(&self) -> Result<Option<String>> {
        Ok(Some("void".to_string()))
    }

    /// Return the set sample rate.
    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    /// Initialize the Backend.
    fn initialize(&mut self) -> Result<()> {
        // Generate a dummy Stack Trace
        let stack_traces = generate_stack_trace()?;

        // Add the StackTrace to the buffer
        for stack_trace in stack_traces {
            let stack_trace = stack_trace + &self.ruleset;
            self.buffer.record(stack_trace)?;
        }

        Ok(())
    }

    /// Shutdown the Backend.
    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }

    /// Generate a report.
    fn report(&mut self) -> Result<Vec<Report>> {
        let reports = self.buffer.clone().into();

        Ok(reports)
    }

    /// Set the configuration.
    fn set_config(&self, _config: BackendConfig) {}

    /// Return the configuration.
    fn get_config(&self) -> Result<BackendConfig> {
        Ok(BackendConfig::default())
    }

    /// Add a Rule to the Backend.
    fn add_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.add_rule(rule)?;

        Ok(())
    }

    /// Remove a Rule from the Backend.
    fn remove_rule(&self, rule: Rule) -> Result<()> {
        self.ruleset.remove_rule(rule)?;

        Ok(())
    }
}

/// A Backend implementation for testing purposes.
pub fn void_backend(config: VoidConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(VoidBackend::new(config)), None)
}
