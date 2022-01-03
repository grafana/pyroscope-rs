// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use thiserror::Error;

/// Result Alias with PyroscopeError
pub type Result<T> = std::result::Result<T, PyroscopeError>;

/// Error type of Pyroscope
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

impl From<pprof::Error> for PyroscopeError {
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

impl From<std::io::Error> for PyroscopeError {
    fn from(_err: std::io::Error) -> Self {
        PyroscopeError {
            msg: String::from("IO Error"),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for PyroscopeError {
    fn from(_err: std::sync::PoisonError<T>) -> Self {
        PyroscopeError {
            msg: String::from("Poison/Mutex Error"),
        }
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for PyroscopeError {
    fn from(_err: std::sync::mpsc::SendError<T>) -> Self {
        PyroscopeError {
            msg: String::from("mpsc Send Error"),
        }
    }
}
