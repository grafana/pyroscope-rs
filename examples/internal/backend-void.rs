extern crate pyroscope;

use pyroscope::backend::{void_backend, VoidConfig};
use pyroscope::Result;

fn main() -> Result<()> {
    // Create new VoidConfig
    let backend_config = VoidConfig::new().sample_rate(100);

    // Create backend
    let backend = void_backend(backend_config);

    // Initialize backend
    let mut ready_backend = backend.initialize()?;

    // Collect profile data
    let report = ready_backend.report()?;

    // Print report
    dbg!(report);

    Ok(())
}
