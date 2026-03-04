//! Signal-safe eventfd and epoll-based multi-fd waiter.
//!
//! # Platform support
//! The implementation uses inline-asm syscalls and is therefore only available
//! on `x86_64`/Linux.  On other targets the public types exist but every
//! constructor returns an `Err` at runtime, keeping downstream code
//! unconditionally compilable.

/// Error type: the raw positive errno value returned by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub i32);

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "errno {}", self.0)
    }
}

// ── platform-specific syscall numbers & constants ─────────────────────────────

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
use crate::syscall_nr::x86_64::{
    SYS_CLOSE, SYS_EPOLL_CREATE1, SYS_EPOLL_CTL, SYS_EPOLL_WAIT, SYS_EVENTFD2, SYS_WRITE,
};

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod flags {
    /// `EFD_NONBLOCK | EFD_SEMAPHORE`
    pub const EFD_FLAGS: usize = 0x800 | 0x1;
    /// `EPOLL_CTL_ADD`
    pub const EPOLL_CTL_ADD: usize = 1;
    /// `EPOLLIN`
    pub const EPOLLIN: u32 = 0x0000_0001;
}

// ── epoll_event layout (x86_64, packed) ──────────────────────────────────────

/// Mirror of `struct epoll_event` on x86_64 Linux (packed, 12 bytes).
/// `data` is the full 8-byte union; we use only the `u64` interpretation.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[repr(C, packed)]
struct EpollEvent {
    events: u32,
    data: u64,
}

// ── raw fd helpers ────────────────────────────────────────────────────────────

/// Close a raw file descriptor. Errors are ignored.
fn close_fd(fd: i32) {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    unsafe {
        crate::syscall::syscall1(SYS_CLOSE, fd as usize);
    }
    #[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
    let _ = fd;
}

// ── EventFd ───────────────────────────────────────────────────────────────────

/// A non-blocking, semaphore-mode Linux eventfd.
///
/// Owns the file descriptor; closes it on `Drop`.
pub struct EventFd {
    fd: i32,
}

impl EventFd {
    /// Create a new `EventFd`.
    ///
    /// Uses `SYS_eventfd2` with `EFD_NONBLOCK | EFD_SEMAPHORE`.
    pub fn new() -> Result<Self, Error> {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            let ret = unsafe { crate::syscall::syscall2(SYS_EVENTFD2, 0, flags::EFD_FLAGS) };
            if ret >= 0 {
                Ok(Self { fd: ret as i32 })
            } else {
                Err(Error((-ret) as i32))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
        Err(Error(38)) // ENOSYS
    }

    /// Return the underlying file descriptor.
    ///
    /// The fd remains owned by this `EventFd`; do not close it externally.
    pub fn as_fd(&self) -> i32 {
        self.fd
    }

    /// Write 1 to the eventfd counter.
    ///
    /// Signal-safe: uses a direct `SYS_write` syscall; errors are ignored
    /// because there is nothing meaningful to do in a signal-handler context.
    pub fn notify(&self) {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            let val: u64 = 1;
            // SAFETY: `val` lives on the stack for the duration of the syscall.
            unsafe {
                crate::syscall::syscall3(
                    SYS_WRITE,
                    self.fd as usize,
                    &val as *const u64 as usize,
                    8,
                );
            }
        }
    }
}

impl Drop for EventFd {
    fn drop(&mut self) {
        close_fd(self.fd);
    }
}

// ── EventSet ──────────────────────────────────────────────────────────────────

/// Capacity of an `EventSet` — the maximum number of `EventFd`s it can hold.
pub const EVENT_SET_CAPACITY: usize = 64;

/// Waits on up to [`EVENT_SET_CAPACITY`] `EventFd`s simultaneously using epoll.
///
/// After construction, register individual `EventFd`s with [`EventSet::add`].
/// Call [`EventSet::wait`] to block until at least one fires; it returns the
/// **index** (0-based, in registration order) of the first notified fd.
///
/// Owns the epoll file descriptor; closes it on `Drop`.
pub struct EventSet {
    epfd: i32,
    /// fds registered in order; index into this slice == the index returned by `wait`.
    fds: [i32; EVENT_SET_CAPACITY],
    len: usize,
}

impl EventSet {
    /// Create an empty `EventSet`.
    pub fn new() -> Result<Self, Error> {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            // epoll_create1(0) — no special flags needed
            let ret = unsafe { crate::syscall::syscall1(SYS_EPOLL_CREATE1, 0) };
            if ret >= 0 {
                Ok(Self {
                    epfd: ret as i32,
                    fds: [-1; EVENT_SET_CAPACITY],
                    len: 0,
                })
            } else {
                Err(Error((-ret) as i32))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
        Err(Error(38)) // ENOSYS
    }

    /// Register an `EventFd` with this set.
    ///
    /// Returns the 0-based index assigned to this fd, which is the value
    /// [`EventSet::wait`] will return when this fd fires.
    ///
    /// Returns `Err` if the set is full or the `epoll_ctl` syscall fails.
    pub fn add(&mut self, efd: &EventFd) -> Result<usize, Error> {
        if self.len >= EVENT_SET_CAPACITY {
            return Err(Error(28)); // ENOSPC
        }
        let idx = self.len;

        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            let mut ev = EpollEvent {
                events: flags::EPOLLIN,
                // Store the registration index in the epoll data so we can
                // recover it without a secondary lookup in `wait`.
                data: idx as u64,
            };
            let ret = unsafe {
                crate::syscall::syscall4(
                    SYS_EPOLL_CTL,
                    self.epfd as usize,
                    flags::EPOLL_CTL_ADD,
                    efd.fd as usize,
                    &mut ev as *mut EpollEvent as usize,
                )
            };
            if ret < 0 {
                return Err(Error((-ret) as i32));
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
        {
            let _ = efd;
            return Err(Error(38)); // ENOSYS
        }

        self.fds[idx] = efd.fd;
        self.len += 1;
        Ok(idx)
    }

    /// Block until at least one registered `EventFd` is notified.
    ///
    /// Returns the **index** (as given by [`EventSet::add`]) of the first
    /// fd that became ready.  If multiple fds fired simultaneously only the
    /// first one (in epoll's internal ordering) is reported; the others remain
    /// pending for the next call.
    ///
    /// `timeout_ms` is passed directly to `epoll_wait`:
    /// - `-1` → block indefinitely
    /// - `0` → non-blocking poll
    /// - `n > 0` → wait up to `n` milliseconds
    pub fn wait(&self, timeout_ms: i32) -> Result<usize, Error> {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            let mut ev = EpollEvent { events: 0, data: 0 };
            loop {
                let ret = unsafe {
                    crate::syscall::syscall4(
                        SYS_EPOLL_WAIT,
                        self.epfd as usize,
                        &mut ev as *mut EpollEvent as usize,
                        1, // max_events = 1
                        timeout_ms as usize,
                    )
                };
                if ret > 0 {
                    return Ok(ev.data as usize);
                } else if ret == 0 {
                    return Err(Error(110)); // ETIMEDOUT
                } else {
                    let errno = (-ret) as i32;
                    if errno == 4 {
                        // EINTR — restart the syscall
                        continue;
                    }
                    return Err(Error(errno));
                }
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
        {
            let _ = timeout_ms;
            Err(Error(38)) // ENOSYS
        }
    }
}

impl Drop for EventSet {
    fn drop(&mut self) {
        if self.epfd >= 0 {
            close_fd(self.epfd);
        }
    }
}
