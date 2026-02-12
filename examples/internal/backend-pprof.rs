extern crate pyroscope;

use pyroscope::backend::{Backend, BackendConfig};
use pyroscope::Result;
use pyroscope_pprofrs::{Pprof, PprofConfig};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    let config = PprofConfig { sample_rate: 100 };
    let backend_config = BackendConfig {
        report_thread_id: false,
        report_thread_name: false,
        report_pid: false,
    };

    let mut backend = Pprof::new(config, backend_config);

    backend.initialize()?;

    fibonacci(45);

    let report = backend.report()?;

    dbg!(report);

    Box::new(backend).shutdown()?;

    Ok(())
}
