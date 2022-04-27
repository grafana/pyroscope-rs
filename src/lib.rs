//! Rust integration for [Pyroscope](https://pyroscope.io).
//!
//! # Quick Start
//!
//! ## Add the Pyroscope Library and the pprof-rs backend to Cargo.toml
//!
//! ```toml
//! [dependencies]
//! pyroscope = "0.5"
//! pyroscope-pprofrs = "0.2"
//! ```
//!
//! ## Configure a Pyroscope Agent
//!
//! ```ignore
//! let agent =
//!     PyroscopeAgent::builder("http://localhost:4040", "myapp")
//!     .backend(Pprof::new(PprofConfig::new().sample_rate(100)))
//!     .build()?;
//! ```
//!
//! ## Start/Stop profiling
//!
//! To start profiling code and sending data.
//!
//! ```ignore
//!  let agent_running = agent.start()?;
//! ```
//!
//! To stop profiling code. You can restart the profiling at a later point.
//!
//! ```ignore
//!  let agent_ready = agent.stop()?;
//! ```
//!
//! Before you drop the variable, make sure to shutdown the agent.
//!
//! ```ignore
//! agent_ready.shutdown();
//! ```

// Re-exports structs
pub use crate::pyroscope::PyroscopeAgent;
pub use error::{PyroscopeError, Result};

// Public modules
pub mod backend;
pub mod error;
pub mod pyroscope;
pub mod session;
pub mod timer;

// Private modules
mod utils;
