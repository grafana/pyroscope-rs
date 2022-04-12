extern crate pyroscope;

use log::info;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_pprofrs::{pprof_backend, PprofConfig};
use std::hash::{Hash, Hasher};

fn hash_rounds(n: u64) -> u64 {
    let hash_str = "Some string to hash";
    let mut default_hasher = std::collections::hash_map::DefaultHasher::new();

    for _ in 0..n {
        for _ in 0..1000 {
            default_hasher.write(hash_str.as_bytes());
        }
        hash_str.hash(&mut default_hasher);
    }

    n
}

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    std::env::set_var("RUST_LOG", "trace");

    // Initialize the logger.
    pretty_env_logger::init_timed();

    info!("With Logger example");

    // Create a new agent.
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.logger")
        .backend(pprof_backend(PprofConfig::new().sample_rate(100)))
        .build()?;

    // Start Agent
    agent.start()?;

    let _result = hash_rounds(300_000);

    // Stop Agent
    agent.stop()?;

    Ok(())
}
