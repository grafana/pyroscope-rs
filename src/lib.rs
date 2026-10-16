//! Rust integration for [Pyroscope](https://grafana.com/oss/pyroscope/).
//!
//! # Quick Start
//!
//! ## Add the Pyroscope Library and the pprof-rs backend to Cargo.toml
//!
//! ```toml
//! [dependencies]
//! pyroscope = { version = "2.0.0", features = ["backend-pprof-rs"] }
//! ```
//!
//! ## Configure a Pyroscope Agent
//!
//! ```no_run
//! # #[cfg(feature = "backend-pprof-rs")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use pyroscope::pyroscope::PyroscopeAgentBuilder;
//! use pyroscope::backend::{pprof_backend, PprofConfig, BackendConfig};
//!
//! let agent = PyroscopeAgentBuilder::new(
//!     "http://localhost:4040",
//!     "myapp",
//!     100, // sample rate
//!     "pyroscope-rs",
//!     env!("CARGO_PKG_VERSION"),
//!     pprof_backend(PprofConfig::default(), BackendConfig::default()),
//! )
//! .build()?;
//! let running = agent.start()?;
//! // Run the application workload.
//! let ready = running.stop()?;
//! ready.shutdown();
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "backend-pprof-rs"))]
//! # fn main() {}
//! ```
//!
//! Start the agent before the workload, stop it to finish the current report,
//! and shut it down after profiling is complete. The example above shows the
//! complete lifecycle.

extern crate core;

// Re-exports structs
pub use crate::pyroscope::PyroscopeAgent;
pub use error::{PyroscopeError, Result};

pub mod backend;
pub mod encode;
pub mod error;
pub mod pyroscope;
pub mod session;
pub mod timer;

mod utils;
pub use utils::ThreadId;
pub mod ffikit;
