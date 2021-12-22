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

#[tokio::main]
async fn main() -> Result<()> {
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "fibonacci")
        .frequency(100)
        .tags(
            [("Hostname".to_owned(), "pyroscope".to_owned())]
                .iter()
                .cloned()
                .collect(),
        )
        .build()?;

    agent.start()?;


    agent.add_tags(
        [("series".to_owned(), "Number 1".to_owned()), ("order".to_owned(), "first".to_owned())]
            .iter()
            .cloned()
            .collect(),
    )?;
    let result = fibonacci(47);
    println!("fibonacci {}", result);
    agent.remove_tags(vec!["order".to_owned()])?;

    agent.add_tags(
        [("series".to_owned(), "Number 2".to_owned())]
            .iter()
            .cloned()
            .collect(),
    )?;
    let result = fibonacci(47);
    println!("fibonacci {}", result);
    agent.remove_tags(vec!["series".to_owned()])?;

    agent.stop().await?;

    Ok(())
}
