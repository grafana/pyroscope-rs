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
    pub backend: T,
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

impl StackTrace {
    /// Create a new StackTrace
    pub fn new(
        pid: Option<u32>, thread_id: Option<u64>, thread_name: Option<String>,
        frames: Vec<StackFrame>,
    ) -> Self {
        Self {
            pid,
            thread_id,
            thread_name,
            frames,
        }
    }
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

impl StackFrame {
    /// Create a new StackFrame.
    pub fn new(
        module: Option<String>, name: Option<String>, filename: Option<String>,
        relative_path: Option<String>, absolute_path: Option<String>, line: Option<u32>,
    ) -> Self {
        Self {
            module,
            name,
            filename,
            relative_path,
            absolute_path,
            line,
        }
    }
}

impl std::fmt::Display for StackFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} - {}",
            self.filename.as_ref().unwrap_or(&"".to_string()),
            self.line.unwrap_or(0),
            self.name.as_ref().unwrap_or(&"".to_string())
        )
    }
}

impl std::fmt::Display for StackTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &self
                .frames
                .iter()
                .rev()
                .map(|frame| format!("{}", frame))
                .collect::<Vec<_>>()
                .join(";")
        )
    }
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

    #[test]
    fn test_stack_frame_display() {
        let frame = StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(1),
        );

        assert_eq!(format!("{}", frame), "filename:1 - name");
    }

    #[test]
    fn test_stack_trace_display() {
        let mut frames = Vec::new();
        frames.push(StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(1),
        ));
        frames.push(StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(2),
        ));

        let stack_trace = StackTrace::new(None, None, None, frames);

        assert_eq!(
            format!("{}", stack_trace),
            "filename:2 - name;filename:1 - name"
        );
    }
}
