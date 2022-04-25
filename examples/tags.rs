extern crate pyroscope;

use pyroscope::{backend::Tag, PyroscopeAgent, Result};
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
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.tags")
        .backend(pprof_backend(PprofConfig::new().sample_rate(100)))
        .tags([("Hostname", "pyroscope")].to_vec())
        .build()?;

    // Start Agent
    agent.start()?;

    // Show start time
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Start Time: {}", start);

    // Add Tags
    agent.add_global_tag(Tag::new("series".to_string(), "Number 1".to_string()))?;

    // Make some calculation
    let _result = hash_rounds(300_000);

    // Add Tags
    agent.remove_global_tag(Tag::new("series".to_string(), "Number 1".to_string()))?;
    agent.add_global_tag(Tag::new("series".to_string(), "Number 2".to_string()))?;

    // Do more calculation
    let _result = hash_rounds(500_000);

    // Show stop time
    let stop = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Stop Time: {}", stop);

    // Stop Agent
    agent.stop()?;

    Ok(())
}
