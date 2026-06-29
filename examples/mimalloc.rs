//! Mimalloc memory profiling example.
//!
//! This example demonstrates the `backend-mimalloc` API shape. The first
//! implementation phase records sampled allocation totals; later phases will
//! replace the synthetic sample frame with captured allocation stacks.
//!
//! ```sh
//! cargo run --example mimalloc --features backend-mimalloc
//! ```

use pyroscope::backend::mimalloc::{mimalloc_backend, MimallocConfig, SamplingMiMalloc};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use std::time::Duration;

#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let agent = PyroscopeAgentBuilder::new(
        "http://localhost:4040",
        "example.mimalloc",
        100,
        "pyroscope-rs",
        env!("CARGO_PKG_VERSION"),
        mimalloc_backend(MimallocConfig::default()),
    )
    .tags(vec![("env", "dev")])
    .build()?;

    let agent_running = agent.start()?;

    let start = std::time::Instant::now();
    let mut iteration = 0u64;
    while start.elapsed() < Duration::from_secs(30) {
        for i in 0..100 {
            let size = 1024 * (1 + (iteration as usize + i) % 1024);
            let allocation: Vec<u8> = vec![0u8; size];
            std::hint::black_box(&allocation);
        }
        iteration += 1;
    }
    eprintln!("Completed {iteration} iterations");

    let agent_ready = agent_running.stop()?;
    agent_ready.shutdown();

    Ok(())
}
