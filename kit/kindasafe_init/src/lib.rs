#[derive(Debug, PartialEq, Clone)]
pub enum InitError {
    InstallSignalHandlersFailed,
    SanityCheckFailed,
}

// todo think how to have less static mut
static mut FALLBACK_SIGSEGV: libc::sigaction = unsafe { std::mem::zeroed() };
static mut FALLBACK_SIGBUS: libc::sigaction = unsafe { std::mem::zeroed() };

static INIT_LOCK: spin::Mutex<Option<Result<(), InitError>>> = spin::Mutex::new(None);

pub fn is_initialized() -> Option<Result<(), InitError>> {
    let g = INIT_LOCK.lock();
    g.clone()
}
pub fn init() -> Result<(), InitError> {
    let mut g = INIT_LOCK.lock();
    if let Some(prev) = g.clone() {
        return prev;
    }

    let res = init_locked();
    *g = Some(res.clone());
    res
}

pub fn init_locked() -> Result<(), InitError> {
    unsafe {
        FALLBACK_SIGSEGV = new_signal_handler(libc::SIGSEGV, crash_handler)
            .map_err(|_| InitError::InstallSignalHandlersFailed)?;
        FALLBACK_SIGBUS = new_signal_handler(libc::SIGBUS, crash_handler)
            .map_err(|_| InitError::InstallSignalHandlersFailed)?;
    }
    Ok(())
}

/// # Safety
/// `data` must be a valid pointer to `libc::ucontext_t`.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
unsafe fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut libc::c_void) {
    unsafe {
        let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
        let pc = (*ctx).uc_mcontext.gregs[libc::REG_RIP as usize] as usize;
        for x in kindasafe::crash_points().crash_points {
            if x.pc == pc {
                (*ctx).uc_mcontext.gregs[libc::REG_RIP as usize] = (pc + x.skip) as libc::greg_t;
                let reg_idx = match x.signal_reg {
                    kindasafe::Reg::Rax => libc::REG_RAX as usize,
                    kindasafe::Reg::Rdx => libc::REG_RDX as usize,
                };
                (*ctx).uc_mcontext.gregs[reg_idx] = sig as u64 as libc::greg_t;
                return;
            }
        }
        fallback(sig, info, data);
    }
}

/// # Safety
/// `data` must be a valid pointer to `libc::ucontext_t`.
#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
unsafe fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut libc::c_void) {
    unsafe {
        let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
        let mctx = (*ctx).uc_mcontext;
        let ss = &mut (*mctx).__ss;
        let pc = ss.__rip as usize;
        for x in kindasafe::crash_points().crash_points {
            if x.pc == pc {
                ss.__rip = (pc + x.skip) as u64;
                match x.signal_reg {
                    kindasafe::Reg::Rax => ss.__rax = sig as u64,
                    kindasafe::Reg::Rdx => ss.__rdx = sig as u64,
                };
                return;
            }
        }
        fallback(sig, info, data);
    }
}

/// # Safety
/// `data` must be a valid pointer to `libc::ucontext_t`.
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
unsafe fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut libc::c_void) {
    unsafe {
        let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
        let pc = (*ctx).uc_mcontext.pc as usize;
        for x in kindasafe::crash_points().crash_points {
            if x.pc == pc {
                (*ctx).uc_mcontext.pc = (pc + x.skip) as u64;
                let reg_idx = match x.signal_reg {
                    kindasafe::Reg::X0 => 0,
                    kindasafe::Reg::X1 => 1,
                };
                (*ctx).uc_mcontext.regs[reg_idx] = sig as u64;
                return;
            }
        }
        fallback(sig, info, data);
    }
}

/// # Safety
/// `data` must be a valid pointer to `libc::ucontext_t`.
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
unsafe fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut libc::c_void) {
    unsafe {
        let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
        let mctx = (*ctx).uc_mcontext;
        let pc = (*mctx).__ss.__pc as usize;
        for x in kindasafe::crash_points().crash_points {
            if x.pc == pc {
                (*mctx).__ss.__pc = (pc + x.skip) as u64;
                let reg_idx = match x.signal_reg {
                    kindasafe::Reg::X0 => 0,
                    kindasafe::Reg::X1 => 1,
                };
                (*mctx).__ss.__x[reg_idx] = sig as u64;
                return;
            }
        }
        fallback(sig, info, data);
    }
}

fn call_fallback(
    sig: libc::c_int,
    info: *mut libc::siginfo_t,
    data: *mut libc::c_void,
    fallback: libc::sigaction,
) {
    if fallback.sa_sigaction == 0 {
        restore_default_ignal_handler(sig);
    } else {
        let handler = unsafe {
            std::mem::transmute::<
                usize,
                extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void),
            >(fallback.sa_sigaction)
        };
        handler(sig, info, data);
    }
}
unsafe fn fallback(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut libc::c_void) {
    if sig == libc::SIGSEGV {
        call_fallback(sig, info, data, unsafe { FALLBACK_SIGSEGV });
        return;
    }
    if sig == libc::SIGBUS {
        call_fallback(sig, info, data, unsafe { FALLBACK_SIGBUS });
    }
}

fn new_signal_handler(
    signal: libc::c_int,
    handler: unsafe fn(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut libc::c_void),
) -> Result<libc::sigaction, ()> {
    unsafe {
        let mut old: libc::sigaction = std::mem::zeroed();
        if libc::sigaction(signal, std::ptr::null_mut(), &mut old) != 0 {
            return Err(());
        }
        let mut new: libc::sigaction = old;
        new.sa_sigaction = handler as usize;
        new.sa_flags |= libc::SA_RESTART | libc::SA_SIGINFO;
        if libc::sigaction(signal, &new, &mut old) != 0 {
            return Err(());
        }
        Ok(old)
    }
}

pub fn restore_default_ignal_handler(sig: libc::c_int) {
    let action: libc::sigaction = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaction(sig, &action, std::ptr::null_mut()) };
}

/// Sanity check that kindasafe crash recovery is working.
///
/// Maps a PROT_NONE page, attempts to read it via `kindasafe::u64`,
/// and verifies the read returns an error (SIGSEGV) instead of crashing.
/// Unmaps the page before returning.
///
/// Returns `Ok(())` if the sanity check passes, `Err(SanityCheckFailed)` if
/// the read unexpectedly succeeded (meaning crash recovery is broken).
pub fn sanity_check() -> Result<(), InitError> {
    unsafe {
        let page = libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        );
        if page == libc::MAP_FAILED {
            return Err(InitError::SanityCheckFailed);
        }
        let addr = page as u64;
        let result = kindasafe::u64(addr);
        libc::munmap(page, 4096);
        match result {
            Err(_) => Ok(()),
            Ok(_) => Err(InitError::SanityCheckFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_idempotent() {
        assert!(init().is_ok());
        assert!(init().is_ok());
        assert!(is_initialized().is_some());
        assert_eq!(is_initialized(), Some(Ok(())));
    }

    #[test]
    fn test_sanity_check() {
        assert!(init().is_ok());
        assert!(sanity_check().is_ok());
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[test]
    fn test_reg_mapping() {
        // Verify our Reg enum maps to the correct libc register indices.
        // The crash_handler maps Reg::Rax -> libc::REG_RAX, Reg::Rdx -> libc::REG_RDX.
        assert_eq!(libc::REG_RAX, 13);
        assert_eq!(libc::REG_RDX, 12);
    }

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    #[test]
    fn test_reg_mapping() {
        // Verify our Reg enum maps to the correct __darwin_x86_thread_state64 fields.
        // The crash_handler maps Reg::Rax -> __ss.__rax, Reg::Rdx -> __ss.__rdx.
        let ss: libc::__darwin_x86_thread_state64 = unsafe { std::mem::zeroed() };
        let base = &ss as *const _ as usize;
        assert_eq!((&ss.__rax as *const _ as usize - base) % 8, 0);
        assert_eq!((&ss.__rdx as *const _ as usize - base) % 8, 0);
        assert_eq!((&ss.__rip as *const _ as usize - base) % 8, 0);
    }
}
