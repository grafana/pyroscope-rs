use serde::{Deserialize, Serialize};

use super::error::Result;
use clap::ValueEnum;
use std::str::FromStr;
// TODO: These definitions should be placed in the core workspace.

#[derive(Debug, Serialize, Deserialize, Copy, Clone, ValueEnum)]
pub enum LogLevel {
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "error")]
    Error,
}

impl FromStr for LogLevel {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(super::error::Error::new(
                format!("unknown LogLevel: {:?}", s).as_str(),
            )),
        }
    }
}
impl AsRef<str> for LogLevel {
    fn as_ref(&self) -> &str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}


#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
pub enum Spy {
    #[serde(rename = "rbspy")]
    Rbspy,
    #[serde(rename = "pyspy")]
    Pyspy,
}

impl FromStr for Spy {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "rbspy" => Ok(Spy::Rbspy),
            "pyspy" => Ok(Spy::Pyspy),
            _ => Err(super::error::Error::new(
                format!("unknown spy {:?}", s).as_str(),
            )),
        }
    }
}

impl AsRef<str> for Spy {
    fn as_ref(&self) -> &str {
        match self {
            Spy::Rbspy => "rbspy",
            Spy::Pyspy => "pyspy",
        }
    }
}
