//! Debug output helpers using raw syscalls — zero-cost no-ops in release builds.
//!
//! All functions compile away completely when `debug_assertions` is disabled
//! (i.e. in `--release` or any profile with `debug-assertions = false`).

#[cfg(all(debug_assertions, target_arch = "x86_64", target_os = "linux"))]
use crate::syscall_nr::x86_64::SYS_WRITE;
#[cfg(all(debug_assertions, target_arch = "x86_64", target_os = "linux"))]
const STDOUT: usize = 1;

/// Write a string to stdout followed by a newline (debug builds only).
#[cfg(debug_assertions)]
#[inline(always)]
pub fn puts(s: &str) {
    writes(s);
    writes("\n");
}

/// Write a string to stdout without a trailing newline (debug builds only).
#[cfg(debug_assertions)]
#[inline(always)]
pub fn writes(s: &str) {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    unsafe {
        crate::syscall::syscall3(SYS_WRITE, STDOUT, s.as_ptr() as usize, s.len());
    }
}

/// Write a `usize` value as lowercase hex digits to stdout (debug builds only).
///
/// No `0x` prefix is emitted; callers should use `writes("0x")` before this
/// if the prefix is desired.
#[cfg(debug_assertions)]
#[inline(always)]
pub fn write_hex(v: usize) {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    unsafe {
        const HEX: &[u8] = b"0123456789abcdef";
        let mut buf = [0u8; 16];
        let mut i = 16usize;
        let mut n = v;
        loop {
            i -= 1;
            buf[i] = HEX[n & 0xf];
            n >>= 4;
            if n == 0 {
                break;
            }
        }
        crate::syscall::syscall3(SYS_WRITE, STDOUT, buf.as_ptr().add(i) as usize, 16 - i);
    }
}

// --- release-build no-ops ---

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn puts(_s: &str) {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn writes(_s: &str) {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn write_hex(_v: usize) {}
