/// Verify that every constant in `syscall_nr::x86_64` matches the value
/// exported by the `libc` crate.  This catches copy-paste errors and makes
/// future arch additions easier to validate.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod x86_64 {
    use notlibc::syscall_nr::x86_64;

    #[test]
    fn syscall_numbers_match_libc() {
        assert_eq!(x86_64::SYS_READ, libc::SYS_read as usize);
        assert_eq!(x86_64::SYS_WRITE, libc::SYS_write as usize);
        assert_eq!(x86_64::SYS_CLOSE, libc::SYS_close as usize);
        assert_eq!(x86_64::SYS_MMAP, libc::SYS_mmap as usize);
        assert_eq!(x86_64::SYS_MPROTECT, libc::SYS_mprotect as usize);
        assert_eq!(x86_64::SYS_MUNMAP, libc::SYS_munmap as usize);
        assert_eq!(x86_64::SYS_EPOLL_WAIT, libc::SYS_epoll_wait as usize);
        assert_eq!(x86_64::SYS_EPOLL_CTL, libc::SYS_epoll_ctl as usize);
        assert_eq!(x86_64::SYS_OPENAT, libc::SYS_openat as usize);
        assert_eq!(x86_64::SYS_EVENTFD2, libc::SYS_eventfd2 as usize);
        assert_eq!(x86_64::SYS_EPOLL_CREATE1, libc::SYS_epoll_create1 as usize);
    }
}
