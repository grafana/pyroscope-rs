#![no_std]

pub mod auxv;
pub mod debug;
mod errno_guard;
pub mod mmap;
mod syscall;
pub mod syscall_nr;

pub use spin::Mutex;

pub type ShardMutex<T> = spin::Mutex<T>;

pub mod eventfd;
pub use eventfd::{EVENT_SET_CAPACITY, EventFd, EventSet};

/// Return the caller's Linux thread ID via raw `SYS_gettid` syscall.
///
/// Async-signal-safe: uses inline assembly, no libc.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub fn gettid() -> u32 {
    unsafe { syscall::syscall1(syscall_nr::x86_64::SYS_GETTID, 0) as u32 }
}
