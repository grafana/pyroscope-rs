use std::io::Error;
use std::mem;

pub fn new_signal_handler(signal: libc::c_int, handler: usize) -> std::result::Result<libc::sigaction, Error> {
    let mut new: libc::sigaction = unsafe { mem::zeroed() };
    new.sa_sigaction = handler as usize;
    new.sa_flags = libc::SA_RESTART | libc::SA_SIGINFO;
    let mut old: libc::sigaction = unsafe { mem::zeroed() };
    if unsafe { libc::sigaction(signal, &new, &mut old) } != 0 {
        return Err(Error::last_os_error());
    }
    Ok(old)
}


pub fn restore_signal_handler(signal: libc::c_int, prev: libc::sigaction) -> std::result::Result<(), Error> {
    let mut old: libc::sigaction = unsafe { mem::zeroed() };
    if unsafe { libc::sigaction(signal, &prev, &mut old) } != 0 {
        return Err(Error::last_os_error());
    }
    Ok(())
}
pub fn start_timer(interval : libc::time_t) -> std::result::Result<(), Error> {
    // let interval = 10000000; //
    let sec = interval / 1000000000;
    let usec = (interval % 1000000000) / 1000;
    let mut tv: libc::itimerval = unsafe { mem::zeroed() };
    tv.it_value.tv_sec = sec;
    tv.it_value.tv_usec = usec as libc::suseconds_t;
    tv.it_interval.tv_sec = sec;
    tv.it_interval.tv_usec = usec as libc::suseconds_t;
    if unsafe { libc::setitimer(libc::ITIMER_PROF, &tv, std::ptr::null_mut()) } != 0 {
        return Err(Error::last_os_error());
    }
    return Ok(());
}
