//! Jemalloc memory profiling example.
//!
//! This example demonstrates how to use the jemalloc backend for memory profiling.
//! It requires jemalloc to be configured as the global allocator with profiling enabled.
//!
//! # Requirements
//!
//! Add these dependencies to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! pyroscope = { version = "0.5", features = ["backend-jemalloc"] }
//! tikv-jemallocator = "0.6"
//! ```
//!
//! # Running
//!
//! ```sh
//! # Enable jemalloc profiling via environment variable
//! _RJEM_MALLOC_CONF=prof:true,prof_active:true,lg_prof_sample:19 \
//!     cargo run --example jemalloc --features backend-jemalloc
//! ```

use pyroscope::backend::jemalloc::{jemalloc_backend, JemallocConfig};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use std::thread;
use std::time::Duration;

// Configure jemalloc as the global allocator.
// Profiling must also be enabled at runtime via MALLOC_CONF or _RJEM_MALLOC_CONF.
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let agent = PyroscopeAgentBuilder::new(
        "http://localhost:4040",
        "example.jemalloc",
        100,
        "pyroscope-rs",
        env!("CARGO_PKG_VERSION"),
        jemalloc_backend(JemallocConfig::default()),
    )
    .tags(vec![("env", "dev")])
    .build()?;

    let agent_running = agent.start()?;

    // Simulate some allocations.
    for i in 0..30 {
        let size = 1024 * (1 + i % 10);
        let _v: Vec<u8> = vec![0u8; size];
        thread::sleep(Duration::from_secs(1));
    }

    let agent_ready = agent_running.stop()?;
    agent_ready.shutdown();

    Ok(())
}
