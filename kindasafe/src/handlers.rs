use std::io::Error;

//todo make it not an api of the crate
pub fn new_signal_handler(
    signal: libc::c_int,
    handler: fn(sig: libc::c_int, info: *const libc::siginfo_t, data: *mut libc::c_void),
) -> Result<libc::sigaction, Error> {
    unsafe {
        let mut old: libc::sigaction = std::mem::zeroed();
        if libc::sigaction(signal, std::ptr::null_mut(), &mut old) != 0 {
            return Err(Error::last_os_error());
        }
        let mut new: libc::sigaction = old.clone();
        new.sa_sigaction = handler as usize;
        new.sa_flags |= libc::SA_RESTART | libc::SA_SIGINFO;
        if libc::sigaction(signal, &new, &mut old) != 0 {
            return Err(Error::last_os_error());
        }
        Ok(old)
    }
}

pub fn restore_signal_handler(
    signal: libc::c_int, prev: libc::sigaction,
) -> Result<(), Error> {
    let mut old: libc::sigaction = unsafe {std::mem::zeroed()};
    if unsafe {libc::sigaction(signal, &prev, &mut old)} != 0 {
        return Err(Error::last_os_error());
    }
    Ok(())
}

pub fn restore_default(sig: libc::c_int) {
    let mut action: libc::sigaction = unsafe{std::mem::zeroed()};
    action.sa_sigaction = libc::SIG_DFL;
    unsafe{libc::sigaction(sig, &action, std::ptr::null_mut())};
}
