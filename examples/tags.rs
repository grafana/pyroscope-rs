extern crate pyroscope;

use pyroscope::PyroscopeAgent;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "example.tags")
        .sample_rate(100)
        .tags(&[("Hostname", "pyroscope")])
        .build()?;

    // Start Agent
    agent.start();

    // Make some calculation
    let _result = fibonacci(47);

    // Add Tags
    agent.add_tags(&[("series", "Number 2")])?;

    // Do more calculation
    let _result = fibonacci(47);

    // Stop Agent
    agent.stop();

    Ok(())
}
