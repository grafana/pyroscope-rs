// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "fibonacci")
        .sample_rate(100)
        .tags(
            &[("Hostname", "pyroscope")]
        )
        .build()?;

    // Start Agent
    agent.start()?;

    // Make some calculation
    let _result = fibonacci(47);

    // Add Tags
    agent.add_tags(
        &[("series", "Number 2")]
    )?;

    // Do more calculation
    let _result = fibonacci(47);

    // Stop Agent
    agent.stop()?;

    Ok(())
}
