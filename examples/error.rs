extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_pprofrs::{pprof_backend, PprofConfig};

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

    let agent = PyroscopeAgent::builder("http://invalid_url", "example.error")
        .backend(pprof_backend(PprofConfig::new().sample_rate(100)))
        .build()
        .unwrap();
    // Start Agent
    let agent_running = agent.start()?;

    let _result = fibonacci(47);

    // Stop Agent
    let agent_ready = agent_running.stop()?;

    agent_ready.shutdown();

    Ok(())
}
