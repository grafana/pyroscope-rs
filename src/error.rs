use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, PyroscopeError>;

#[derive(Error, Debug)]
pub struct PyroscopeError {
  pub msg: String,
}

impl fmt::Display for PyroscopeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<reqwest::Error> for PyroscopeError {
    fn from(_err: reqwest::Error) -> Self {
        PyroscopeError {
            msg: String::from("reqwest Error"),
        }
    }
}

impl From<pprof::error::Error> for PyroscopeError {
    fn from(_err: pprof::Error) -> Self {
        PyroscopeError {
            msg: String::from("pprof Error"),
        }
    }
}

impl From<tokio::task::JoinError> for PyroscopeError {
    fn from(_err: tokio::task::JoinError) -> Self {
        PyroscopeError {
            msg: String::from("Tokio handler join error"),
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for PyroscopeError {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        PyroscopeError {
            msg: String::from("pprof Error"),
        }
    }
}

impl From<std::time::SystemTimeError> for PyroscopeError {
    fn from(_err: std::time::SystemTimeError) -> Self {
        PyroscopeError {
            msg: String::from("SystemTime Error"),
        }
    }
}
