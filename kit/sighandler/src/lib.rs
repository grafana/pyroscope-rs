use core::ffi::{c_int, c_void};

pub type HandlerFn = extern "C" fn(c_int, *mut libc::siginfo_t, *mut c_void);

#[derive(Debug, PartialEq)]
pub enum Error {
    SigactionFailed,
    SetitimerFailed,
}

/// Install `handler` as the SIGPROF signal handler and start a repeating
/// 10 ms ITIMER_PROF timer (100 Hz).
///
/// # Safety
/// The caller must ensure `handler` is safe to invoke from a signal context.
pub fn start(handler: HandlerFn) -> Result<(), Error> {
    unsafe {
        register_sigaction(handler)?;
        start_timer()?;
    }
    Ok(())
}

unsafe fn register_sigaction(handler: HandlerFn) -> Result<(), Error> {
    unsafe {
        let mut new_action: libc::sigaction = core::mem::zeroed();
        new_action.sa_sigaction = handler as usize;
        new_action.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
        libc::sigemptyset(&mut new_action.sa_mask);
        libc::sigaddset(&mut new_action.sa_mask, libc::SIGPROF);
        libc::sigaddset(&mut new_action.sa_mask, libc::SIGSEGV);
        libc::sigaddset(&mut new_action.sa_mask, libc::SIGBUS);
        if libc::sigaction(libc::SIGPROF, &new_action, core::ptr::null_mut()) != 0 {
            return Err(Error::SigactionFailed);
        }
    }
    Ok(())
}

unsafe fn start_timer() -> Result<(), Error> {
    unsafe {
        let interval = libc::itimerval {
            it_interval: libc::timeval {
                tv_sec: 0,
                tv_usec: 10_000,
            },
            it_value: libc::timeval {
                tv_sec: 0,
                tv_usec: 10_000,
            },
        };
        if libc::setitimer(libc::ITIMER_PROF, &interval, core::ptr::null_mut()) != 0 {
            return Err(Error::SetitimerFailed);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicI32, Ordering};

    static LAST_SIG: AtomicI32 = AtomicI32::new(0);

    extern "C" fn test_handler(sig: c_int, _info: *mut libc::siginfo_t, _ctx: *mut c_void) {
        LAST_SIG.store(sig, Ordering::Relaxed);
    }

    #[test]
    fn start_returns_ok() {
        let result = start(test_handler);
        // Clean up: disarm the timer immediately.
        unsafe {
            let zero = libc::itimerval {
                it_interval: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                it_value: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
            };
            libc::setitimer(libc::ITIMER_PROF, &zero, core::ptr::null_mut());
        }
        assert_eq!(result, Ok(()));
    }
}
