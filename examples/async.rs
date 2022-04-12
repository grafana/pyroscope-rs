extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_pprofrs::{pprof_backend, PprofConfig};
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
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.async")
        .backend(pprof_backend(PprofConfig::new().sample_rate(100)))
        .tags([("TagA", "ValueA"), ("TagB", "ValueB")].to_vec())
        .build()?;

    // Start Agent
    agent.start()?;

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
    agent.stop()?;

    Ok(())
}
