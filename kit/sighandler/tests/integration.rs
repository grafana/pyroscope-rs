use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

static COUNTER: AtomicU32 = AtomicU32::new(0);

extern "C" fn counting_handler(_sig: c_int, _info: *mut libc::siginfo_t, _ctx: *mut c_void) {
    COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Burn CPU for at least `dur` so ITIMER_PROF (which counts CPU time) fires.
fn burn_cpu(dur: Duration) {
    let start = Instant::now();
    let mut x: u64 = 1;
    while start.elapsed() < dur {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    }
    // Prevent the loop from being optimized away.
    let _ = x;
}

#[test]
fn sigprof_fires_repeatedly() {
    COUNTER.store(0, Ordering::Relaxed);

    sighandler::start(counting_handler).expect("start failed");

    // Burn ~200 ms of CPU time; at 100 Hz we expect ~20 signals.
    burn_cpu(Duration::from_millis(200));

    // Disarm the timer before asserting so stray signals don't interfere.
    unsafe {
        let zero = libc::itimerval {
            it_interval: libc::timeval { tv_sec: 0, tv_usec: 0 },
            it_value: libc::timeval { tv_sec: 0, tv_usec: 0 },
        };
        libc::setitimer(libc::ITIMER_PROF, &zero, core::ptr::null_mut());
    }

    let count = COUNTER.load(Ordering::Relaxed);
    assert!(count > 0, "expected SIGPROF to fire at least once, got {count}");
}
