use std::fmt::{Debug, Display};
use std::error::Error as StdError;

//todo make initialization - not part of the crate
#[derive(Debug, PartialEq)]
pub enum InitError {
    AlreadyInitialized,
    ReadInsnNotFound,
    ReadVecInsnNotFound,
    InstallSignalHandlersFailed,
    SanityCheckFailed,
}

impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::AlreadyInitialized => write!(f, "AlreadyInitialized"),
            InitError::ReadInsnNotFound => write!(f, "ReadInsnNotFound"),
            InitError::ReadVecInsnNotFound => write!(f, "ReadVecInsnNotFound"),
            InitError::InstallSignalHandlersFailed => write!(f, "InstallSignalHandlersFailed"),
            InitError::SanityCheckFailed => write!(f, "SanityCheckFailed"),
        }
    }
}

impl StdError for InitError {}

// todo how to not expose this to users?
#[derive(Debug, PartialEq)]
pub enum DestroyError {
    NotInitialized,
    RestoreHandlersFailed,
}

impl Display for DestroyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DestroyError::NotInitialized => write!(f, "NotInitialized"),
            DestroyError::RestoreHandlersFailed => write!(f, "RestoreHandlersFailed"),
        }
    }
}

impl StdError for DestroyError {}

