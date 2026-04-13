use std::time::Duration;

fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map(|pages| pages * 4)
        .unwrap_or(0) as f64
        / 1024.0
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
    eprintln!("=== Memory investigation (issue #127) ===");
    eprintln!();
    eprintln!("[0] Baseline RSS: {:.1} MB", rss_mb());

    // ProfilerGuard::build() internally calls trigger_lazy() which runs
    // backtrace::Backtrace::new() (primes the global DWARF cache) and
    // creates the Collector<UnresolvedFrames> (~18 MB pre-allocation).
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .build()
        .unwrap();
    eprintln!("[1] After ProfilerGuard::build(): {:.1} MB", rss_mb());

    busy_work(Duration::from_secs(3));
    eprintln!("[2] After collecting samples (3s): {:.1} MB", rss_mb());

    // report().build() resolves symbols via backtrace::resolve() for each
    // frame, which may load additional DWARF sections into the global cache.
    let report = guard.report().build().unwrap();
    eprintln!(
        "[3] After report().build(): {:.1} MB  ({} unique stacks)",
        rss_mb(),
        report.data.len()
    );
    drop(report);

    // Dropping the guard calls Profiler::stop() → Profiler::init() which
    // allocates a new Collector before dropping the old one. glibc keeps
    // the freed pages mapped, so RSS increases by ~18 MB and never shrinks.
    drop(guard);
    eprintln!("[4] After dropping guard: {:.1} MB", rss_mb());

    // Second cycle to show memory is stable (no unbounded growth).
    let guard2 = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .build()
        .unwrap();
    busy_work(Duration::from_secs(3));
    let report2 = guard2.report().build().unwrap();
    eprintln!(
        "[5] After second cycle: {:.1} MB  ({} unique stacks)",
        rss_mb(),
        report2.data.len()
    );

    drop(report2);
    drop(guard2);
    eprintln!("[6] Final: {:.1} MB", rss_mb());

    Ok(())
}
