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
    eprintln!("=== Granular memory investigation ===");
    eprintln!();
    eprintln!("[0] Baseline RSS: {:.1} MB", rss_mb());

    // Step 1: Just calling backtrace::Backtrace::new() — triggers DWARF loading
    eprintln!();
    eprintln!("--- Step 1: backtrace::Backtrace::new() ---");
    let _bt = backtrace::Backtrace::new();
    eprintln!("[1] After backtrace::Backtrace::new(): {:.1} MB", rss_mb());
    drop(_bt);
    eprintln!("[1b] After dropping backtrace: {:.1} MB", rss_mb());

    // Step 2: Build ProfilerGuard (triggers trigger_lazy + Collector + UNWINDER)
    eprintln!();
    eprintln!("--- Step 2: ProfilerGuardBuilder::default().build() ---");
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .build()
        .unwrap();
    eprintln!("[2] After ProfilerGuard build: {:.1} MB", rss_mb());

    // Step 3: Collect some samples
    eprintln!();
    eprintln!("--- Step 3: Busy work for 3s to collect samples ---");
    busy_work(Duration::from_secs(3));
    eprintln!("[3] After collecting samples: {:.1} MB", rss_mb());

    // Step 4: Build report (triggers symbol resolution via backtrace::resolve)
    eprintln!();
    eprintln!("--- Step 4: guard.report().build() (symbol resolution) ---");
    let report = guard.report().build().unwrap();
    eprintln!(
        "[4] After report().build(): {:.1} MB  (report has {} unique stacks)",
        rss_mb(),
        report.data.len()
    );
    drop(report);
    eprintln!("[4b] After dropping report: {:.1} MB", rss_mb());

    // Step 5: Drop guard, build new one, collect, report again
    eprintln!();
    eprintln!("--- Step 5: Drop guard, build new, collect, report ---");
    drop(guard);
    eprintln!("[5a] After dropping guard: {:.1} MB", rss_mb());

    let guard2 = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .build()
        .unwrap();
    eprintln!("[5b] After new guard build: {:.1} MB", rss_mb());

    busy_work(Duration::from_secs(3));
    let report2 = guard2.report().build().unwrap();
    eprintln!(
        "[5c] After second report: {:.1} MB  ({} unique stacks)",
        rss_mb(),
        report2.data.len()
    );

    drop(report2);
    drop(guard2);
    eprintln!("[5d] After dropping everything: {:.1} MB", rss_mb());

    Ok(())
}
