use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use std::time::Duration;

fn rss_kb() -> usize {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map(|pages| pages * 4) // pages to KB
        .unwrap_or(0)
}

fn rss_mb() -> f64 {
    rss_kb() as f64 / 1024.0
}

fn busy_work(duration: Duration) {
    let start = std::time::Instant::now();
    let mut x = 0u64;
    while start.elapsed() < duration {
        for _ in 0..10_000 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
        }
        std::hint::black_box(x);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    eprintln!("=== pyroscope-rs memory investigation ===");
    eprintln!();
    eprintln!("Baseline RSS: {:.1} MB", rss_mb());

    let agent = PyroscopeAgentBuilder::new(
        "http://localhost:44040", // intentionally wrong port — we don't need a server
        "memory-test",
        100,
        "pyroscope-rs",
        env!("CARGO_PKG_VERSION"),
        pprof_backend(PprofConfig::default(), BackendConfig::default()),
    )
    .build()?;

    eprintln!("After build (backend initialized): {:.1} MB", rss_mb());

    let agent_running = agent.start()?;
    eprintln!("After start: {:.1} MB", rss_mb());

    // Generate CPU activity so the profiler has samples to collect
    eprintln!();
    eprintln!("Running busy work for 15s (one full report cycle)...");
    busy_work(Duration::from_secs(15));

    eprintln!("After first report cycle (symbol resolution triggered): {:.1} MB", rss_mb());

    eprintln!();
    eprintln!("Running busy work for another 15s...");
    busy_work(Duration::from_secs(15));

    eprintln!("After second report cycle: {:.1} MB", rss_mb());

    eprintln!();
    eprintln!("Running busy work for another 15s...");
    busy_work(Duration::from_secs(15));

    eprintln!("After third report cycle: {:.1} MB", rss_mb());

    let agent_ready = agent_running.stop()?;
    eprintln!("After stop: {:.1} MB", rss_mb());

    agent_ready.shutdown();
    eprintln!("After shutdown: {:.1} MB", rss_mb());

    Ok(())
}
