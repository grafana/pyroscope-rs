use serde::{Deserialize, Serialize};

use crate::error::Result;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub enum LogLevel {
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
    type Err = crate::error::Error;

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
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
    type Err = crate::error::Error;

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

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Spy {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "rbspy")]
    Rbspy,
    #[serde(rename = "dotnetspy")]
    Dotnetspy,
    #[serde(rename = "ebpfspy")]
    Ebpfspy,
    #[serde(rename = "phpspy")]
    Phpspy,
    #[serde(rename = "pyspy")]
    Pyspy,
}

impl FromStr for Spy {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "auto" => Ok(Spy::Auto),
            "rbspy" => Ok(Spy::Rbspy),
            "dotnetspy" => Ok(Spy::Dotnetspy),
            "ebpfspy" => Ok(Spy::Ebpfspy),
            "phpspy" => Ok(Spy::Phpspy),
            "pyspy" => Ok(Spy::Pyspy),
            _ => Ok(Spy::Auto),
        }
    }
}
