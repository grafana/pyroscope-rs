// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};
use pyroscope::backends::pprof::{Pprof};
use pyroscope::backends::Backend;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<()>{
    
    let mut backend = pyroscope::backends::pprof::Pprof::default();
    backend.initialize(100)?;
    backend.start()?;

    fibonacci(45);
    let report = backend.report()?;
    println!("{}", std::str::from_utf8(&report).unwrap()); 

    backend.stop()?;

    Ok(())
}
