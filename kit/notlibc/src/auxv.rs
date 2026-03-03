/// Read entries from the ELF auxiliary vector via `/proc/self/auxv`.
///
/// Uses only inline-assembly syscalls (SYS_openat, SYS_read, SYS_close) so
/// it is safe to call from a signal handler and requires no libc.

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod imp {
    use crate::syscall::{syscall1, syscall3, syscall4};
    use crate::mmap::check;

    // ── syscall numbers ────────────────────────────────────────────────────────
    const SYS_OPENAT: usize = 257;
    const SYS_READ: usize = 0;
    const SYS_CLOSE: usize = 3;

    // ── openat constants ───────────────────────────────────────────────────────
    const AT_FDCWD: usize = (-100_isize) as usize;
    const O_RDONLY: usize = 0;

    // ── ELF auxiliary vector entry types ──────────────────────────────────────
    const AT_NULL: usize = 0;

    /// Scan `/proc/self/auxv` for the entry with the given `tag` and return
    /// its value, or `None` if the file cannot be read or the tag is absent.
    pub fn getauxval(tag: usize) -> Option<usize> {
        const PATH: &[u8] = b"/proc/self/auxv\0";

        let fd = unsafe {
            check(syscall4(
                SYS_OPENAT,
                AT_FDCWD,
                PATH.as_ptr() as usize,
                O_RDONLY,
                0, // mode — unused without O_CREAT
            ))
            .ok()?
        };

        // Each auxv entry is a (type, value) pair of native `usize` words.
        // A stack buffer of 64 entries (1 024 bytes on 64-bit) comfortably
        // covers the ~20 entries the Linux kernel typically produces.
        const BUF_ENTRIES: usize = 64;
        const ENTRY_SIZE: usize = core::mem::size_of::<usize>() * 2;
        const BUF_BYTES: usize = BUF_ENTRIES * ENTRY_SIZE;

        let mut buf = [0u8; BUF_BYTES];
        let mut result: Option<usize> = None;

        'outer: loop {
            let n = unsafe {
                check(syscall3(
                    SYS_READ,
                    fd as usize,
                    buf.as_mut_ptr() as usize,
                    BUF_BYTES,
                ))
            };
            let n = match n {
                Ok(0) | Err(_) => break,
                Ok(n) => n as usize,
            };

            let available = &buf[..n];
            let mut i = 0usize;
            while i + ENTRY_SIZE <= available.len() {
                // Decode a (type, value) pair from little-endian bytes.
                // `usize::from_le_bytes` is used so the code compiles in
                // `no_std` without any byte-order helpers.
                const WORD: usize = core::mem::size_of::<usize>();
                let mut a_bytes = [0u8; WORD];
                let mut v_bytes = [0u8; WORD];
                a_bytes.copy_from_slice(&available[i..i + WORD]);
                v_bytes.copy_from_slice(&available[i + WORD..i + 2 * WORD]);
                let a_type = usize::from_le_bytes(a_bytes);
                let a_val  = usize::from_le_bytes(v_bytes);
                i += ENTRY_SIZE;

                if a_type == AT_NULL {
                    break 'outer;
                }
                if a_type == tag {
                    result = Some(a_val);
                    break 'outer;
                }
            }
        }

        unsafe { syscall1(SYS_CLOSE, fd as usize) };
        result
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub use imp::getauxval;
