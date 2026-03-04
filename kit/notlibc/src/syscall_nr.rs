//! Linux syscall numbers for each supported architecture.
//!
//! Add a new `#[cfg(target_arch = "…")]` block when porting to a new arch.

#[cfg(target_arch = "x86_64")]
pub mod x86_64 {
    pub const SYS_READ: usize = 0;
    pub const SYS_WRITE: usize = 1;
    pub const SYS_CLOSE: usize = 3;
    pub const SYS_MMAP: usize = 9;
    pub const SYS_MPROTECT: usize = 10;
    pub const SYS_MUNMAP: usize = 11;
    pub const SYS_EPOLL_WAIT: usize = 232;
    pub const SYS_EPOLL_CTL: usize = 233;
    pub const SYS_OPENAT: usize = 257;
    pub const SYS_EVENTFD2: usize = 290;
    pub const SYS_EPOLL_CREATE1: usize = 291;
}
