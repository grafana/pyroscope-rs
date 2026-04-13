fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map(|pages| pages * 4)
        .unwrap_or(0) as f64
        / 1024.0
}

fn main() {
    eprintln!("=== Component-level memory investigation ===");
    eprintln!();
    eprintln!("[0] Baseline RSS: {:.1} MB", rss_mb());

    // Step 1: Collector<usize> — just the hash map + tempfile structures
    let collector = pprof::Collector::<usize>::new().expect("collector");
    eprintln!("[1] After Collector<usize>::new(): {:.1} MB", rss_mb());
    drop(collector);
    eprintln!("[1b] After drop: {:.1} MB", rss_mb());

    // Step 2: ProfilerGuard build
    // This triggers: trigger_lazy() → backtrace::Backtrace::new() + UNWINDER init + Profiler(Collector<UnresolvedFrames>)
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .build()
        .unwrap();
    eprintln!("[2] After ProfilerGuard::build(): {:.1} MB", rss_mb());

    // Step 3: Drop to see what stays resident
    drop(guard);
    eprintln!("[3] After guard drop (Profiler::stop → new Collector): {:.1} MB", rss_mb());
}
