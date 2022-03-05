use super::error::{BackendError, Result};
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
    /// Backend Spy Name
    fn spy_name(&self) -> Result<String>;
    /// Get backend configuration.
    fn sample_rate(&self) -> Result<u32>;
    /// Initialize the backend.
    fn initialize(&mut self) -> Result<()>;
    /// Start the backend.
    fn start(&mut self) -> Result<()>;
    /// Stop the backend.
    fn stop(&mut self) -> Result<()>;
    /// Generate profiling report
    fn report(&mut self) -> Result<Vec<u8>>;
}

/// Backend Holder
///
/// This is an experimental holder for the backend Trait. It's goal is to garantuee State
/// Transitions and avoid State transition implementations in the backend.
pub struct BackendImpl<T: Backend> {
    state: State,
    backend: T,
}

impl<T: Backend> BackendImpl<T> {
    /// Create a new backend factory.
    pub fn new(backend: T) -> Self {
        Self {
            state: State::Uninitialized,
            backend,
        }
    }

    /// Get the backend state.
    pub fn get_state(&self) -> State {
        self.state
    }

    /// Return the spyname of the backend.
    pub fn spy_name(&self) -> Result<String> {
        self.backend.spy_name()
    }

    /// Return the sample rate of the backend.
    pub fn sample_rate(&self) -> Result<u32> {
        self.backend.sample_rate()
    }

    /// Initialize the backend.
    pub fn initialize(&mut self) -> Result<()> {
        // Check if Backend is Uninitialized
        if self.state != State::Uninitialized {
            return Err(BackendError::new("Backend is already Initialized"));
        }

        self.backend.initialize()?;

        // Set State to Ready
        self.state = State::Ready;

        Ok(())
    }

    /// Start the backend.
    pub fn start(&mut self) -> Result<()> {
        // Check if Backend is Ready
        if self.state != State::Ready {
            return Err(BackendError::new("Backend is not Ready"));
        }

        self.backend.start()?;

        // Set State to Running
        self.state = State::Running;

        Ok(())
    }

    /// Stop the backend.
    pub fn stop(&mut self) -> Result<()> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Backend is not Running"));
        }

        self.backend.stop()?;

        // Set State to Ready
        self.state = State::Ready;

        Ok(())
    }

    /// Generate profiling report
    pub fn report(&mut self) -> Result<Vec<u8>> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Backend is not Running"));
        }

        self.backend.report()
    }
}

/// StackTrace
#[derive(Debug, Default)]
pub struct StackTrace {
    /// Process ID
    pub pid: Option<u32>,
    /// Thread ID
    pub thread_id: Option<u64>,
    /// Thread Name
    pub thread_name: Option<String>,
    /// Stack Trace
    pub frames: Vec<StackFrame>,
}

/// StackFrame
#[derive(Debug, Default)]
pub struct StackFrame {
    /// Module name
    pub module: Option<String>,
    /// Function name
    pub name: Option<String>,
    /// File name
    pub filename: Option<String>,
    /// File relative path
    pub relative_path: Option<String>,
    /// File absolute path
    pub absolute_path: Option<String>,
    /// Line number
    pub line: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_impl() {
        // Create mock TestBackend
        #[derive(Debug)]
        struct TestBackend;
        impl Backend for TestBackend {
            fn get_state(&self) -> State {
                State::Uninitialized
            }

            fn spy_name(&self) -> Result<String> {
                Ok("TestBackend".to_string())
            }

            fn sample_rate(&self) -> Result<u32> {
                Ok(100)
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

        // Create BackendImpl
        let mut backend = BackendImpl::new(TestBackend);

        // Test State Transitions
        assert_eq!(backend.get_state(), State::Uninitialized);
        assert!(backend.initialize().is_ok());
        assert_eq!(backend.get_state(), State::Ready);
        assert!(backend.start().is_ok());
        assert_eq!(backend.get_state(), State::Running);
        assert!(backend.stop().is_ok());
        assert_eq!(backend.get_state(), State::Ready);
    }
}
