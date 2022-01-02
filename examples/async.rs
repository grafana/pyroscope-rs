// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};

fn fibonacci1(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci1(n - 1) + fibonacci1(n - 2),
    }
}

fn fibonacci2(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci2(n - 1) + fibonacci2(n - 2),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "async")
        .tags(&[("TagA", "ValueA"), ("TagB", "ValueB")])
        .build()?;

    // Start Agent
    agent.start()?;

    tokio::task::spawn(async {
        let n = fibonacci1(45);
        println!("Thread 1: {}", n);
    })
    .await
    .unwrap();

    tokio::task::spawn(async {
        let n = fibonacci2(45);
        println!("Thread 2: {}", n);
    })
    .await
    .unwrap();

    // Stop Agent
    agent.stop()?;

    Ok(())
}
