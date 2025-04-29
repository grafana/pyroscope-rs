use libc::{SIGPROF, c_int};

unsafe fn setitimer(
    which: libc::c_int,
    new: *const libc::itimerval,
    old: *mut libc::itimerval,
) -> libc::c_long {
    #[cfg(target_arch = "x86_64")]
    const NR_SETITIMER: libc::c_long = 38;
    #[cfg(target_arch = "aarch64")]
    const NR_SETITIMER: libc::c_long = 103;
    unsafe { libc::syscall(NR_SETITIMER, which, new, old) }
}

pub fn new_signal_handler(
    handler: fn(sig: libc::c_int, info: *const libc::siginfo_t, data: *mut libc::c_void),
) -> Result<libc::sigaction, std::io::Error> {
    let usec = 100000;
    let signal = SIGPROF;
    let tv: libc::itimerval = libc::itimerval {
        it_interval: libc::timeval {
            tv_sec: 0,
            tv_usec: usec,
        },
        it_value: libc::timeval {
            tv_sec: 0,
            tv_usec: usec,
        },
    };

    // if (setitimer(ITIMER_PROF, &tv, NULL) != 0) {
    //     return Error("ITIMER_PROF is not supported on this system");
    // }

    unsafe {
        let mut old: libc::sigaction = std::mem::zeroed();
        if libc::sigaction(signal, std::ptr::null_mut(), &mut old) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut new: libc::sigaction = old.clone();
        new.sa_sigaction = handler as usize;
        new.sa_flags |= libc::SA_RESTART | libc::SA_SIGINFO;
        if libc::sigaction(signal, &new, &mut old) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        const ITIMER_PROF: c_int = 2;
        if setitimer(ITIMER_PROF, &tv, std::ptr::null_mut()) != 0 {
            return Err(std::io::Error::last_os_error());
        };

        Ok(old)
    }
}
