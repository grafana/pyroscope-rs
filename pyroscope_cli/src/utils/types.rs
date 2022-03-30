use clap::ArgEnum;
use serde::{Deserialize, Serialize};

use super::error::Result;
use std::str::FromStr;

// TODO: These definitions should be placed in the core workspace.

#[derive(Debug, Serialize, Deserialize, Copy, Clone, ArgEnum)]
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
    #[serde(rename = "critical")]
    Critical,
}

impl FromStr for LogLevel {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Ok(LogLevel::Info),
        }
    }
}

/// Output Format for Adhoc profiling
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ArgEnum)]
pub enum OutputFormat {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "html")]
    Html,
    #[serde(rename = "pprof")]
    Pprof,
    #[serde(rename = "collpased")]
    Collapsed,
}

impl FromStr for OutputFormat {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(OutputFormat::None),
            "html" => Ok(OutputFormat::Html),
            "pprof" => Ok(OutputFormat::Pprof),
            "collapsed" => Ok(OutputFormat::Collapsed),
            _ => Ok(OutputFormat::None),
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ArgEnum)]
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
            _ => Ok(Spy::Rbspy),
        }
    }
}
