extern crate pyroscope;

use pyroscope::{Result};
use pyroscope_pprofrs::{pprof_backend, PprofConfig};
use std::hash::{Hash, Hasher};
use pyroscope::backend::BackendConfig;
use pyroscope::pyroscope::PyroscopeAgentBuilder;

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
    let backend = pprof_backend(PprofConfig { sample_rate: 100 }, BackendConfig::default());
    let agent = PyroscopeAgentBuilder::new("http://localhost:4040", "example.basic", backend)
        .tags([("TagA", "ValueA"), ("TagB", "ValueB")].to_vec())
        .build()?;

    // Show start time
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Start Time: {}", start);

    // Start Agent
    let agent_running = agent.start()?;

    let _result = hash_rounds(300_000);

    // Show stop time
    let stop = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Stop Time: {}", stop);

    // Stop Agent
    let agent_ready = agent_running.stop()?;

    // Shutdown the Agent
    agent_ready.shutdown();

    // Show program exit time
    let exit = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Exit Time: {}", exit);

    Ok(())
}
