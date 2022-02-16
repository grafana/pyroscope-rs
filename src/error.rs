use std::fmt;
use thiserror::Error;

/// Result Alias with PyroscopeError
pub type Result<T> = std::result::Result<T, PyroscopeError>;

/// Error type of Pyroscope
#[derive(Error, Debug)]
pub struct PyroscopeError {
    pub msg: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Display for PyroscopeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Default for PyroscopeError {
    fn default() -> Self {
        PyroscopeError {
            msg: "".to_string(),
            source: None,
        }
    }
}

impl PyroscopeError {
    /// Create a new instance of PyroscopeError
    pub fn new(msg: &str) -> Self {
        PyroscopeError {
            msg: msg.to_string(),
            source: None,
        }
    }

    /// Create a new instance of PyroscopeError with source
    pub fn new_with_source(msg: &str, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        PyroscopeError {
            msg: msg.to_string(),
            source: Some(source),
        }
    }
}

impl From<reqwest::Error> for PyroscopeError {
    fn from(err: reqwest::Error) -> Self {
        PyroscopeError {
            msg: String::from("reqwest Error"),
            source: Some(Box::new(err)),
        }
    }
}

impl From<std::time::SystemTimeError> for PyroscopeError {
    fn from(err: std::time::SystemTimeError) -> Self {
        PyroscopeError {
            msg: String::from("SystemTime Error"),
            source: Some(Box::new(err)),
        }
    }
}

impl From<std::io::Error> for PyroscopeError {
    fn from(err: std::io::Error) -> Self {
        PyroscopeError {
            msg: String::from("IO Error"),
            source: Some(Box::new(err)),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for PyroscopeError {
    fn from(_err: std::sync::PoisonError<T>) -> Self {
        PyroscopeError {
            msg: String::from("Poison Error"),
            source: None,
        }
    }
}

impl<T: 'static + Send + Sync> From<std::sync::mpsc::SendError<T>> for PyroscopeError {
    fn from(err: std::sync::mpsc::SendError<T>) -> Self {
        PyroscopeError {
            msg: String::from("SendError Error"),
            source: Some(Box::new(err)),
        }
    }
}

impl From<pyroscope_backends::error::BackendError> for PyroscopeError {
    fn from(err: pyroscope_backends::error::BackendError) -> Self {
        PyroscopeError {
            msg: err.msg,
            source: err.source,
        }
    }
}
