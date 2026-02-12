extern crate pyroscope;

use pyroscope::backend::BackendConfig;
use pyroscope::backend::{pprof_backend, PprofConfig};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use pyroscope::Result;
use std::hash::{Hash, Hasher};

fn hash_rounds1(n: u64) -> u64 {
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

fn hash_rounds2(n: u64) -> u64 {
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

#[tokio::main]
async fn main() -> Result<()> {
    let backend = pprof_backend(PprofConfig { sample_rate: 100 }, BackendConfig::default());
    let agent = PyroscopeAgentBuilder::new("http://localhost:4040", "example.async", backend)
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

    tokio::task::spawn(async {
        let n = hash_rounds1(300_000);
        println!("Thread 1: {}", n);
    })
    .await
    .unwrap();

    tokio::task::spawn(async {
        let n = hash_rounds2(300_000);
        println!("Thread 2: {}", n);
    })
    .await
    .unwrap();

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
