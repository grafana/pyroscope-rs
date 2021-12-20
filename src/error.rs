use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, PyroscopeError>;

#[derive(Error, Debug)]
pub struct PyroscopeError {
   msg: String,
}

impl fmt::Display for PyroscopeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
