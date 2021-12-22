// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};

use std::thread;
use std::time::Duration;

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
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "MultiThreadGlobal")
        .frequency(100)
        .build()
        ?;

    agent.start()?;

    let handle_1 = thread::spawn(|| {
        fibonacci1(44);
    });

    let handle_2 = thread::spawn(|| {
        fibonacci2(44);
    });

    handle_1.join().unwrap();
    handle_2.join().unwrap();

    agent.stop().await
}
