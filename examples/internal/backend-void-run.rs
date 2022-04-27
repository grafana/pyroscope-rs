extern crate pyroscope;

use log::info;

use pyroscope::backend::{void_backend, VoidConfig};
use pyroscope::{PyroscopeAgent, Result};

fn main() -> Result<()> {
    // Force rustc to display the log messages in the console.
    std::env::set_var("RUST_LOG", "trace");

    // Initialize the logger.
    pretty_env_logger::init_timed();

    info!("Void Backend");

    // Create new VoidConfig
    let backend_config = VoidConfig::new().sample_rate(100);

    // Create backend
    let backend = void_backend(backend_config);

    // Create a new agent.
    let agent = PyroscopeAgent::builder("http://localhost:4040", "void.backend")
        .tags([("TagA", "ValueA"), ("TagB", "ValueB")].to_vec())
        .backend(backend)
        .build()?;

    // Show start time
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("Start Time: {}", start);

    // Start Agent
    let agent_running = agent.start()?;

    // Sleep for 1 minute
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
