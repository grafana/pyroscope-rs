extern crate pyroscope;

use pyroscope::backend::{void_backend, VoidConfig};
use pyroscope::Result;

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
    dbg!(report);

    // Stop profiling
    backend.stop()?;

    Ok(())
}
