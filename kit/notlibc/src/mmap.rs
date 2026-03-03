/// Anonymous memory mapping via inline assembly syscalls — no libc.
///
/// Structural design follows memmap2's unix.rs (MmapInner / Mmap / MmapMut),
/// with every libc call replaced by a call into `crate::syscall`.
///
/// Only anonymous private mappings are supported (the use-case for
/// signal-handler–safe scratch buffers).  File-backed maps are out of scope.

/// Convert a raw kernel `isize` return value into `Result`.
/// Negative values encode `-errno`; non-negative values are success.
/// Not architecture-specific: the sign convention is the same on all
/// Linux targets.
#[inline]
pub(crate) fn check(ret: isize) -> Result<isize, i32> {
    if ret < 0 { Err((-ret) as i32) } else { Ok(ret) }
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod imp {
    use core::ops::{Deref, DerefMut};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::check;
    use crate::auxv::getauxval;
    use crate::syscall::{syscall2, syscall3, syscall6};

    // ── syscall numbers ────────────────────────────────────────────────────────
    const SYS_MMAP: usize = 9;
    const SYS_MUNMAP: usize = 11;
    const SYS_MPROTECT: usize = 10;

    // ── mmap prot / flags constants (Linux x86_64) ─────────────────────────────
    const PROT_READ: usize = 1;
    const PROT_WRITE: usize = 2;
    const PROT_EXEC: usize = 4;
    const MAP_PRIVATE: usize = 0x02;
    const MAP_ANONYMOUS: usize = 0x20;

    // ── ELF auxiliary vector entry type for page size ──────────────────────────
    const AT_PAGESZ: usize = 6;

    // ── page size (cached, same pattern as memmap2) ────────────────────────────

    pub fn page_size() -> usize {
        static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);
        match PAGE_SIZE.load(Ordering::Relaxed) {
            0 => {
                let ps = getauxval(AT_PAGESZ).unwrap_or(4096);
                PAGE_SIZE.store(ps, Ordering::Relaxed);
                ps
            }
            ps => ps,
        }
    }

    // ── MmapInner (mirrors memmap2's MmapInner) ───────────────────────────────
    //
    // Memory layout (same as memmap2):
    //
    //   mmap_base_ptr ──► [page-aligned kernel mapping start]
    //                          │  offset bytes (ignored prefix for file maps)
    //   self.ptr      ──►      └─► [slice start, = mmap_base_ptr for anon maps]
    //
    // For anonymous maps offset is always 0, so self.ptr == mmap_base_ptr.

    struct MmapInner {
        ptr: *mut u8, // start of the user-visible slice (page-aligned for anon)
        len: usize,   // length of the user-visible slice
    }

    impl MmapInner {
        /// Create an anonymous private mapping with the given `prot` flags.
        fn map_anon(len: usize, prot: usize) -> Result<Self, i32> {
            // Mirror memmap2: Rust slices cannot exceed isize::MAX.
            // On 64-bit this is never a practical issue, but keep the guard
            // for correctness on 32-bit (where `usize` == `u32`).
            if core::mem::size_of::<usize>() < 8 && len > isize::MAX as usize {
                return Err(22); // EINVAL
            }
            // mmap(2) rejects len=0 with EINVAL.  Map at least 1 byte so we
            // always obtain a valid kernel mapping; the public slice length
            // stays `len` (possibly 0) so the caller sees an empty slice.
            let map_len = len.max(1);
            let ptr = unsafe {
                check(syscall6(
                    SYS_MMAP,
                    0,                           // addr  = NULL → kernel chooses
                    map_len,                     // length
                    prot,                        // prot
                    MAP_PRIVATE | MAP_ANONYMOUS, // flags
                    usize::MAX,                  // fd = -1  (usize::MAX == -1 as usize)
                    0,                           // offset = 0
                ))? as usize
            };
            Ok(MmapInner {
                ptr: ptr as *mut u8,
                len,
            })
        }

        /// Returns `(page_aligned_base, map_len)` for munmap / mprotect.
        ///
        /// Identical to memmap2's `as_mmap_params()`: the kernel mapping starts
        /// at the page-aligned address below `self.ptr`; for anonymous maps the
        /// offset is always zero so this reduces to `(self.ptr, self.len.max(1))`.
        fn mmap_base_and_len(&self) -> (*mut u8, usize) {
            let offset = self.ptr as usize % page_size();
            let base = unsafe { self.ptr.sub(offset) };
            let map_len = (self.len + offset).max(1);
            (base, map_len)
        }

        fn mprotect(&mut self, prot: usize) -> Result<isize, i32> {
            let (base, map_len) = self.mmap_base_and_len();
            check(unsafe { syscall3(SYS_MPROTECT, base as usize, map_len, prot) })
        }

        #[inline]
        pub fn ptr(&self) -> *const u8 {
            self.ptr
        }

        #[inline]
        pub fn mut_ptr(&mut self) -> *mut u8 {
            self.ptr
        }

        #[inline]
        pub fn len(&self) -> usize {
            self.len
        }
    }

    impl Drop for MmapInner {
        fn drop(&mut self) {
            let (base, map_len) = self.mmap_base_and_len();
            // Errors are ignored in Drop — same rationale as memmap2:
            // there is no meaningful way to report them here.
            let _ = check(unsafe { syscall2(SYS_MUNMAP, base as usize, map_len) });
        }
    }

    // SAFETY: the mapped memory is not tied to any thread-local state.
    unsafe impl Send for MmapInner {}
    unsafe impl Sync for MmapInner {}

    // ── Public RAII types ──────────────────────────────────────────────────────

    /// An immutable (PROT_READ) anonymous memory map.
    ///
    /// Derefs to `&[u8]`.  The mapping is unmapped when dropped.
    pub struct Mmap {
        inner: MmapInner,
    }

    impl Mmap {
        /// Create a read-only anonymous mapping of `len` bytes.
        pub fn map_anon(len: usize) -> Result<Self, i32> {
            MmapInner::map_anon(len, PROT_READ).map(|inner| Mmap { inner })
        }

        /// Transition to a mutable mapping via `mprotect(PROT_READ | PROT_WRITE)`.
        pub fn make_mut(mut self) -> Result<MmapMut, i32> {
            self.inner.mprotect(PROT_READ | PROT_WRITE)?;
            Ok(MmapMut { inner: self.inner })
        }
    }

    impl Deref for Mmap {
        type Target = [u8];
        #[inline]
        fn deref(&self) -> &[u8] {
            unsafe { core::slice::from_raw_parts(self.inner.ptr(), self.inner.len()) }
        }
    }

    /// A mutable (PROT_READ | PROT_WRITE) anonymous memory map.
    ///
    /// Derefs to `&mut [u8]`.  The mapping is unmapped when dropped.
    pub struct MmapMut {
        inner: MmapInner,
    }

    impl MmapMut {
        /// Create a read-write anonymous mapping of `len` bytes.
        pub fn map_anon(len: usize) -> Result<Self, i32> {
            MmapInner::map_anon(len, PROT_READ | PROT_WRITE).map(|inner| MmapMut { inner })
        }

        /// Transition to a read-only mapping via `mprotect(PROT_READ)`.
        pub fn make_read_only(mut self) -> Result<Mmap, i32> {
            self.inner.mprotect(PROT_READ)?;
            Ok(Mmap { inner: self.inner })
        }

        /// Transition to a read+execute mapping via `mprotect(PROT_READ | PROT_EXEC)`.
        pub fn make_exec(mut self) -> Result<Mmap, i32> {
            self.inner.mprotect(PROT_READ | PROT_EXEC)?;
            Ok(Mmap { inner: self.inner })
        }

        #[inline]
        pub fn as_ptr(&self) -> *const u8 {
            self.inner.ptr()
        }

        #[inline]
        pub fn as_mut_ptr(&mut self) -> *mut u8 {
            self.inner.mut_ptr()
        }
    }

    impl Deref for MmapMut {
        type Target = [u8];
        #[inline]
        fn deref(&self) -> &[u8] {
            unsafe { core::slice::from_raw_parts(self.inner.ptr(), self.inner.len()) }
        }
    }

    impl DerefMut for MmapMut {
        #[inline]
        fn deref_mut(&mut self) -> &mut [u8] {
            unsafe { core::slice::from_raw_parts_mut(self.inner.mut_ptr(), self.inner.len()) }
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub use imp::{Mmap, MmapMut, page_size};
