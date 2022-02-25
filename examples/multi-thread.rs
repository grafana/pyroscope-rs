extern crate pyroscope;

use pyroscope::PyroscopeAgent;

use std::thread;

fn fibonacci1(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci1(n - 1) + fibonacci1(n - 2),
    }
}

fn fibonacci2(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci2(n - 1) + fibonacci2(n - 2),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.multithread")
        .sample_rate(100)
        .build()?;

    // Start Agent
    agent.start();

    let handle_1 = thread::spawn(|| {
        fibonacci1(45);
    });

    let handle_2 = thread::spawn(|| {
        fibonacci2(45);
    });

    // Wait for the threads to complete
    handle_1.join()?;
    handle_2.join()?;

    // Stop Agent
    agent.stop();

    Ok(())
}
