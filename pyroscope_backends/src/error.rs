// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use thiserror::Error;

/// Result Alias with BackendError
pub type Result<T> = std::result::Result<T, BackendError>;

/// Error type of Pyroscope
#[derive(Error, Debug)]
pub struct BackendError {
    pub msg: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Default for BackendError {
    fn default() -> Self {
        BackendError {
            msg: "".to_string(),
            source: None,
        }
    }
}

impl BackendError {
    /// Create a new instance of BackendError
    pub fn new(msg: &str) -> Self {
        BackendError {
            msg: msg.to_string(),
            source: None,
        }
    }

    /// Create a new instance of BackendError with source
    pub fn new_with_source(msg: &str, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        BackendError {
            msg: msg.to_string(),
            source: Some(source),
        }
    }
}

impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        BackendError {
            msg: String::from("IO Error"),
            source: Some(Box::new(err)),
        }
    }
}

impl From<pprof::Error> for BackendError {
    fn from(err: pprof::Error) -> Self {
        BackendError {
            msg: String::from("pprof Error"),
            source: Some(Box::new(err)),
        }
    }
}
