// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

/// Result Alias with PyroscopeError
pub type Result<T> = std::result::Result<T, PyroscopeError>;

/// Error type of Pyroscope
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum PyroscopeError {
    #[error("Other: {}", &.0)]
    AdHoc(String),

    #[error("{msg}: {source:?}")]
    Compat {
        msg: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Pprof(#[from] pprof::Error),

    #[error(transparent)]
    TimeSource(#[from] std::time::SystemTimeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl PyroscopeError {
    /// Create a new instance of PyroscopeError
    pub fn new(msg: &str) -> Self {
        PyroscopeError::AdHoc(msg.to_string())
    }

    /// Create a new instance of PyroscopeError with source
    pub fn new_with_source<E>(msg: &str, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        PyroscopeError::Compat {
            msg: msg.to_string(),
            source: Box::new(source),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for PyroscopeError {
    fn from(_err: std::sync::PoisonError<T>) -> Self {
        PyroscopeError::AdHoc("Poison Error".to_owned())
    }
}

impl<T: 'static + Send + Sync> From<std::sync::mpsc::SendError<T>> for PyroscopeError {
    fn from(err: std::sync::mpsc::SendError<T>) -> Self {
        PyroscopeError::Compat {
            msg: String::from("SendError Error"),
            source: Box::new(err),
        }
    }
}
