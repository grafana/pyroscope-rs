extern crate pyroscope;

use log::info;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_backends::void::{void_backend, VoidConfig};

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
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "void.backend")
        .backend(backend.backend)
        .build()?;

    // Start Agent
    agent.start()?;

    // Sleep for 1 minute
    std::thread::sleep(std::time::Duration::from_secs(60));

    // Stop Agent
    agent.stop()?;

    Ok(())
}
