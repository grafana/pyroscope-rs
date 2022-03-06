extern crate pyroscope;

use pyroscope::Result;
use pyroscope_backends::void::{void_backend, VoidConfig};

fn main() -> Result<()> {
    // Create new VoidConfig
    let backend_config = VoidConfig::new().sample_rate(100);

    // Create backend
    let mut backend = void_backend(backend_config);

    // Initialize backend
    backend.initialize()?;

    // Start profiling
    backend.start()?;

    // Collect profile data
    let report = backend.report()?;

    // Print report
    println!("{}", std::str::from_utf8(&report).unwrap());

    // Stop profiling
    backend.stop()?;

    Ok(())
}
