extern crate pyroscope;

use pyroscope::Result;
use pyroscope_backends::pprof::{Pprof, PprofConfig};
use pyroscope_backends::types::Backend;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    // Create Pprof configuration
    let backend_config = PprofConfig::new(113);

    // Create backend
    let mut backend = Pprof::new(backend_config);

    // Initialize backend
    backend.initialize()?;

    // Start profiling
    backend.start()?;

    // Do some work
    fibonacci(45);

    // Collect profile data
    let report = backend.report()?;

    // Print report
    println!("{}", std::str::from_utf8(&report).unwrap());

    // Stop profiling
    backend.stop()?;

    Ok(())
}
