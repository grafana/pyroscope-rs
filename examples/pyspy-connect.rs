extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_backends::pyspy::Pyspy;

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    //std::env::set_var("RUST_LOG", "trace");

    // Initialize the logger.
    //pretty_env_logger::init_timed();

    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rbspy.basic")
        .backend(Pyspy::default())
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
