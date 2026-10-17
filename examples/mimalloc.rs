//! Mimalloc memory profiling example.
//!
//! This example demonstrates `backend-mimalloc` with `SamplingAllocator` as the
//! process global allocator. The backend records sampled allocation stacks where
//! frames can be resolved and sends memory pprof data through the normal
//! Pyroscope agent pipeline.
//!
//! ```sh
//! cargo run --example mimalloc --features backend-mimalloc
//! MIMALLOC_LIVE_HEAP=1 cargo run --example mimalloc --features backend-mimalloc
//! ```

use pyroscope::backend::mimalloc::{mimalloc_backend, MimallocConfig, SamplingAllocator};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use std::time::Duration;

#[global_allocator]
static ALLOC: SamplingAllocator<mimalloc::MiMalloc> =
    SamplingAllocator::with_allocator(mimalloc::MiMalloc);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let live_heap = matches!(std::env::var("MIMALLOC_LIVE_HEAP").as_deref(), Ok("1"));

    let agent = PyroscopeAgentBuilder::new(
        "http://localhost:4040",
        "example.mimalloc",
        100,
        "pyroscope-rs",
        env!("CARGO_PKG_VERSION"),
        mimalloc_backend(MimallocConfig {
            live_heap_tracking: live_heap,
            ..MimallocConfig::default()
        }),
    )
    .tags(vec![("env", "dev")])
    .build()?;

    let agent_running = agent.start()?;
    let retained: Vec<Vec<u8>> = if live_heap {
        (0..64).map(|_| vec![0; 128 * 1024]).collect()
    } else {
        Vec::new()
    };

    let start = std::time::Instant::now();
    let mut iteration = 0u64;
    while start.elapsed() < Duration::from_secs(30) {
        for i in 0..100 {
            let size = 1024 * (1 + (iteration as usize + i) % 1024);
            let allocation: Vec<u8> = vec![0u8; size];
            std::hint::black_box(&allocation);
        }
        iteration += 1;
        std::hint::black_box(&retained);
    }
    eprintln!("Completed {iteration} iterations");

    let agent_ready = agent_running.stop()?;
    agent_ready.shutdown();

    Ok(())
}
