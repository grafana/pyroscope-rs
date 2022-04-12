extern crate pyroscope;

use pyroscope::backend::Backend;
use pyroscope::Result;
use pyroscope_pprofrs::{Pprof, PprofConfig};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    // Create Pprof configuration
    let backend_config = PprofConfig::new().sample_rate(100);

    // Create backend
    let mut backend = Pprof::new(backend_config);

    // Initialize backend
    backend.initialize()?;

    // Do some work
    fibonacci(45);

    // Collect profile data
    let report = backend.report()?;

    // Print report
    dbg!(report);

    // Stop profiling
    backend.shutdown()?;

    Ok(())
}
