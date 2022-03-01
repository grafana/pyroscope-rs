extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope_backends::pprof::{Pprof, PprofConfig};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.backend")
        .backend(Pprof::new(PprofConfig::new(113)))
        .tags(&[("TagA", "ValueA"), ("TagB", "ValueB")])
        .build()?;

    agent.start()?;
    let _result = fibonacci(45);
    agent.stop()?;

    Ok(())
}
