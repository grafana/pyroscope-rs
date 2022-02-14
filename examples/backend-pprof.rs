extern crate pyroscope;

use pyroscope::backends::pprof::Pprof;
use pyroscope::backends::Backend;
use pyroscope::Result;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()> {
    let mut backend = Pprof::default();
    backend.initialize(100)?;
    backend.start()?;

    fibonacci(45);
    let report = backend.report()?;
    println!("{}", std::str::from_utf8(&report).unwrap());

    backend.stop()?;

    Ok(())
}
