fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map(|pages| pages * 4)
        .unwrap_or(0) as f64
        / 1024.0
}

fn print_large_maps(label: &str) {
    eprintln!("\n--- {} (RSS: {:.1} MB) ---", label, rss_mb());
    let smaps = std::fs::read_to_string("/proc/self/smaps").unwrap_or_default();
    let mut current_name = String::new();
    let mut current_rss = 0u64;

    struct Region {
        name: String,
        rss_kb: u64,
    }
    let mut regions: Vec<Region> = Vec::new();

    for line in smaps.lines() {
        if !line.starts_with(|c: char| c.is_ascii_uppercase()) && !line.starts_with(' ') {
            if current_rss > 0 {
                regions.push(Region {
                    name: current_name.clone(),
                    rss_kb: current_rss,
                });
            }
            current_name = line.to_string();
            current_rss = 0;
        } else if line.starts_with("Rss:") {
            if let Some(kb) = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok()) {
                current_rss = kb;
            }
        }
    }
    if current_rss > 0 {
        regions.push(Region {
            name: current_name,
            rss_kb: current_rss,
        });
    }

    regions.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    for r in regions.iter().take(15) {
        if r.rss_kb >= 100 {
            eprintln!("  {:>8} KB  {}", r.rss_kb, &r.name[..r.name.len().min(100)]);
        }
    }
}

fn main() {
    print_large_maps("Baseline");

    let _bt = backtrace::Backtrace::new();
    print_large_maps("After backtrace::Backtrace::new()");
    drop(_bt);

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .build()
        .unwrap();
    print_large_maps("After ProfilerGuard::build()");

    let start = std::time::Instant::now();
    let mut x = 0u64;
    while start.elapsed() < std::time::Duration::from_secs(3) {
        for _ in 0..10_000 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
        }
        std::hint::black_box(x);
    }

    let report = guard.report().build().unwrap();
    eprintln!("\nReport: {} unique stacks", report.data.len());
    print_large_maps("After report().build()");
}
