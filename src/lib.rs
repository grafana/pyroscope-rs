//! Rust integration for [Pyroscope](https://pyroscope.io).
//!
//! # Quick Start
//!
//! ## Configure Pyroscope Agent
//!
//! ```ignore
//! let mut agent =
//!     PyroscopeAgent::builder("http://localhost:4040", "myapp")
//!     .build()?;
//! ```
//!
//! ## Start/Stop profiling
//!
//! To start profiling code and sending data.
//!
//! ```ignore
//!  agent.start();
//! ```
//!
//! To stop profiling code. You can restart the profiling at a later point.
//!
//! ```ignore
//!  agent.stop();
//! ```

// Re-exports structs
pub use crate::pyroscope::PyroscopeAgent;
pub use error::{PyroscopeError, Result};

// Public modules
pub mod error;
pub mod pyroscope;
pub mod session;
pub mod timer;

// Private modules
mod utils;
