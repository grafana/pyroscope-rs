// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

//! Rust integration for [Pyroscope](https://pyroscope.io).
//!
//! # Quick Start
//!
//! ## Configure Pyroscope Agent
//!
//! ```ignore
//! let mut agent =
//!     PyroscopeAgent::builder("http://localhost:4040", "fibonacci")
//!        .sample_rate(100)
//!     .tags(&[("TagA", "ValueA"), ("TagB", "ValueB")])
//!     .build()
//!    ?;
//! ```
//!
//! ## Start/Stop profiling
//!
//! To start profiling code and sending data.
//!
//! ```ignore
//!  agent.start()?;
//! ```
//!
//! To stop profiling code. You can restart the profiling at a later point.
//!
//! ```ignore
//!  agent.stop()?;
//! ```

// Re-exports structs
pub use crate::pyroscope::PyroscopeAgent;
pub use error::{PyroscopeError, Result};

// Public modules
pub mod backends;
pub mod error;
pub mod pyroscope;
pub mod session;
pub mod timer;

// Private modules
mod utils;
