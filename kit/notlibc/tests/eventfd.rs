//! Integration tests for `notlibc::eventfd`.
//!
//! libc is used only in the test harness for draining fds and verifying
//! counts; production code uses no libc.

#![cfg(all(target_arch = "x86_64", target_os = "linux"))]

use notlibc::eventfd::{EventFd, EventSet};
use std::sync::Arc;
use std::thread;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Drain one unit from a semaphore-mode eventfd.
/// Returns the value read (always 1 for semaphore mode) or the negative errno.
fn drain_one(fd: i32) -> i64 {
    let mut buf: u64 = 0;
    let ret = unsafe { libc::read(fd, &mut buf as *mut u64 as *mut libc::c_void, 8) };
    if ret < 0 {
        let errno = unsafe { *libc::__errno_location() };
        -(errno as i64)
    } else {
        buf as i64
    }
}

// ── EventFd tests ─────────────────────────────────────────────────────────────

#[test]
fn create_returns_valid_fd() {
    let efd = EventFd::new().expect("EventFd::new should succeed");
    assert!(efd.as_fd() >= 0, "fd must be non-negative");
    // Drop closes the fd automatically.
}

#[test]
fn notify_once_drain_reads_one() {
    let efd = EventFd::new().expect("EventFd::new");
    efd.notify();
    assert_eq!(drain_one(efd.as_fd()), 1);
}

#[test]
fn notify_twice_drain_twice_accumulates() {
    let efd = EventFd::new().expect("EventFd::new");
    efd.notify();
    efd.notify();
    // Semaphore mode: each read decrements by 1 and returns 1.
    assert_eq!(drain_one(efd.as_fd()), 1);
    assert_eq!(drain_one(efd.as_fd()), 1);
}

#[test]
fn non_blocking_second_read_returns_eagain() {
    let efd = EventFd::new().expect("EventFd::new");
    efd.notify();
    let _ = drain_one(efd.as_fd()); // drain the one notification
    let ret = drain_one(efd.as_fd()); // should be EAGAIN
    assert_eq!(
        ret,
        -(libc::EAGAIN as i64),
        "empty non-blocking eventfd should return EAGAIN, got {ret}"
    );
}

// ── EventSet tests ────────────────────────────────────────────────────────────

#[test]
fn event_set_single_fd_wait() {
    let efd = EventFd::new().expect("EventFd::new");
    let mut set = EventSet::new().expect("EventSet::new");
    let idx = set.add(&efd).expect("EventSet::add");
    assert_eq!(idx, 0);

    efd.notify();
    let fired = set.wait(-1).expect("EventSet::wait");
    assert_eq!(fired, 0);
}

#[test]
fn event_set_identifies_which_fd_fired() {
    // Register 4 eventfds; notify only the third one (index 2).
    let efds: Vec<EventFd> = (0..4).map(|_| EventFd::new().unwrap()).collect();
    let mut set = EventSet::new().unwrap();
    for efd in &efds {
        set.add(efd).unwrap();
    }

    efds[2].notify();
    let fired = set.wait(-1).unwrap();
    assert_eq!(fired, 2, "expected index 2 to fire");
}

#[test]
fn event_set_16_threads_one_reader() {
    const N: usize = 16;

    // Create 16 eventfds and an EventSet.
    let efds: Vec<Arc<EventFd>> = (0..N).map(|_| Arc::new(EventFd::new().unwrap())).collect();
    let mut set = EventSet::new().unwrap();
    for efd in &efds {
        set.add(efd).unwrap();
    }

    // Notify from thread 7.
    let notifier = Arc::clone(&efds[7]);
    let handle = thread::spawn(move || {
        notifier.notify();
    });

    let fired = set.wait(-1).unwrap();
    handle.join().unwrap();

    assert_eq!(fired, 7, "thread 7 should have fired index 7, got {fired}");
}

#[test]
fn event_set_all_16_threads_notify_wait_sees_at_least_one() {
    const N: usize = 16;

    let efds: Vec<Arc<EventFd>> = (0..N).map(|_| Arc::new(EventFd::new().unwrap())).collect();
    let mut set = EventSet::new().unwrap();
    for efd in &efds {
        set.add(efd).unwrap();
    }

    // All 16 threads notify simultaneously.
    let handles: Vec<_> = efds
        .iter()
        .map(|efd| {
            let efd = Arc::clone(efd);
            thread::spawn(move || efd.notify())
        })
        .collect();

    // Wait should return as soon as any one fires.
    let fired = set.wait(-1).unwrap();
    assert!(fired < N, "fired index {fired} out of range");

    for h in handles {
        h.join().unwrap();
    }
}
