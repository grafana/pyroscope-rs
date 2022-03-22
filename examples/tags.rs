extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_pprofrs::{Pprof, PprofConfig};
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
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.tags")
        .backend(Pprof::new(PprofConfig::new().sample_rate(100)))
        .tags([("Hostname", "pyroscope")].to_vec())
        .build()?;

    // Start Agent
    agent.start()?;

    // Make some calculation
    let _result = hash_rounds(300_000);

    // Add Tags
    agent.add_tags(&[("series", "Number 2")])?;

    // Do more calculation
    let _result = hash_rounds(500_000);

    // Stop Agent
    agent.stop()?;

    Ok(())
}
