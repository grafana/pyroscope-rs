extern crate pyroscope;

use std::env;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_rbspy::{rbspy_backend, RbspyConfig};

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    std::env::set_var("RUST_LOG", "error");

    // Initialize the logger.
    pretty_env_logger::init_timed();

    let args: Vec<String> = env::args().collect();

    let pid = args[1].parse::<i32>().unwrap();

    let config = RbspyConfig::new(pid)
        .sample_rate(100)
        .lock_process(true)
        .with_subprocesses(true);

    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rbspy.basic")
        .backend(rbspy_backend(config))
        .build()?;

    // Show start time
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Start Time: {}", start);

    // Start Agent
    agent.start()?;

    // Profile for around 1 minute
    std::thread::sleep(std::time::Duration::from_secs(120));

    agent.stop()?;

    drop(agent);

    // Show program exit time
    let exit = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("Exit Time: {}", exit);

    Ok(())
}
