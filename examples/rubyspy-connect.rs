// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_backends::rbspy::Rbspy;

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    //std::env::set_var("RUST_LOG", "trace");

    // Initialize the logger.
    //pretty_env_logger::init_timed();

    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rbspy.basic")
        .backend(Rbspy::default())
        .build()?;

    // Show start time
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Start Time: {}", start);

    // Start Agent
    agent.start();

    println!("herehere");

    // Profile for around 1 minute
    std::thread::sleep(std::time::Duration::from_secs(60));

    // Stop Agent
    agent.stop();

    drop(agent);

    // Show program exit time
    let exit = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("Exit Time: {}", exit);

    Ok(())
}
