extern crate pyroscope;

use log::{debug, error, info, trace, warn};

use pyroscope::{PyroscopeAgent, Result};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    std::env::set_var("RUST_LOG", "trace");

    // Initialize the logger.
    pretty_env_logger::init_timed();

    info!("With Logger example");

    // Create a new agent.
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.logger").build()?;

    // Start Agent
    agent.start();

    let _result = fibonacci(47);

    // Stop Agent
    agent.stop();

    Ok(())
}
