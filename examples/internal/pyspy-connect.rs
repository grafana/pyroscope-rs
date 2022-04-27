extern crate pyroscope;

use std::env;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    std::env::set_var("RUST_LOG", "info");

    // Initialize the logger.
    pretty_env_logger::init_timed();

    let args: Vec<String> = env::args().collect();

    let pid = args[1].parse::<i32>().unwrap();

    let config = PyspyConfig::new(pid)
        .sample_rate(100)
        .lock_process(true)
        .time_limit(None)
        .with_subprocesses(true)
        .include_idle(false)
        .native(false);

    let agent = PyroscopeAgent::builder("http://localhost:4040", "pyspy.basic")
        .tags([("Host", "Python")].to_vec())
        .backend(pyspy_backend(config))
        .build()?;

    // Show start time
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Start Time: {}", start);

    // Start Agent
    let agent_running = agent.start()?;

    // Profile for around 1 minute
    std::thread::sleep(std::time::Duration::from_secs(60));

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
