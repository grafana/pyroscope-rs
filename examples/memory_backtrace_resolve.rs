fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map(|pages| pages * 4)
        .unwrap_or(0) as f64
        / 1024.0
}

#[inline(never)]
fn target_function() -> *mut std::ffi::c_void {
    std::hint::black_box(42);
    target_function as *mut std::ffi::c_void
}

fn main() {
    eprintln!("=== backtrace::resolve memory cost (issue #127) ===");
    eprintln!();
    eprintln!("[0] Baseline RSS: {:.1} MB", rss_mb());

    let addr = target_function();
    eprintln!("[1] Got address {:?}, RSS: {:.1} MB", addr, rss_mb());

    // A single backtrace::resolve call triggers the backtrace crate to:
    //   1. mmap the main binary and shared libraries
    //   2. Parse DWARF debug sections into addr2line::Context
    //   3. Cache everything in a process-global static that is never freed
    backtrace::resolve(addr, |symbol| {
        eprintln!(
            "    resolved: {} at {}:{}",
            symbol.name().map(|n| n.to_string()).unwrap_or_default(),
            symbol
                .filename()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            symbol.lineno().unwrap_or(0),
        );
    });
    eprintln!("[2] After backtrace::resolve(): {:.1} MB", rss_mb());

    // Resolve again in the same library — cache is already primed.
    backtrace::resolve(main as *mut std::ffi::c_void, |_| {});
    eprintln!("[3] After second resolve (cached): {:.1} MB", rss_mb());

    // Resolve an address in libc — forces loading libc's debug info too.
    let libc_addr = libc::write as *mut std::ffi::c_void;
    backtrace::resolve(libc_addr, |symbol| {
        eprintln!(
            "    resolved libc: {}",
            symbol.name().map(|n| n.to_string()).unwrap_or_default(),
        );
    });
    eprintln!("[4] After resolving libc address: {:.1} MB", rss_mb());
}
