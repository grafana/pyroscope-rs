// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::error::Result;
use crate::PyroscopeError;

use std::collections::HashMap;

// Copyright: https://github.com/cobbinma - https://github.com/YangKeao/pprof-rs/pull/14
/// Format application_name with tags.
pub fn merge_tags_with_app_name(
    application_name: String, tags: HashMap<String, String>,
) -> Result<String> {
    let mut tags_vec = tags
        .into_iter()
        .filter(|(k, _)| k != "__name__")
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>();
    tags_vec.sort();
    let tags_str = tags_vec.join(",");

    if !tags_str.is_empty() {
        Ok(format!("{}{{{}}}", application_name, tags_str,))
    } else {
        Ok(application_name)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::utils::merge_tags_with_app_name;

    #[test]
    fn merge_tags_with_app_name_with_tags() {
        let mut tags = HashMap::new();
        tags.insert("env".to_string(), "staging".to_string());
        tags.insert("region".to_string(), "us-west-1".to_string());
        tags.insert("__name__".to_string(), "reserved".to_string());
        assert_eq!(
            merge_tags_with_app_name("my.awesome.app.cpu".to_string(), tags).unwrap(),
            "my.awesome.app.cpu{env=staging,region=us-west-1}".to_string()
        )
    }

    #[test]
    fn merge_tags_with_app_name_without_tags() {
        assert_eq!(
            merge_tags_with_app_name("my.awesome.app.cpu".to_string(), HashMap::default()).unwrap(),
            "my.awesome.app.cpu".to_string()
        )
    }
}

/// Wrapper for libc functions.
///
/// Error wrapper for some libc functions used by the library. This only does
/// Error (-1 return) wrapping. Alternatively, the nix crate could be used
/// instead of expanding this wrappers (if more functions and types are used
/// from libc)

/// Error Wrapper for libc return. Only check for errors.
fn check_err<T: Ord + Default>(num: T) -> Result<T> {
    if num < T::default() {
        return Err(PyroscopeError::from(std::io::Error::last_os_error()));
    }
    Ok(num)
}

/// libc::timerfd wrapper
pub fn timerfd_create(clockid: libc::clockid_t, clock_flags: libc::c_int) -> Result<i32> {
    check_err(unsafe { libc::timerfd_create(clockid, clock_flags) }).map(|timer_fd| timer_fd as i32)
}

/// libc::timerfd_settime wrapper
pub fn timerfd_settime(
    timer_fd: i32, set_flags: libc::c_int, new_value: &mut libc::itimerspec,
    old_value: &mut libc::itimerspec,
) -> Result<()> {
    check_err(unsafe { libc::timerfd_settime(timer_fd, set_flags, new_value, old_value) })?;
    Ok(())
}

/// libc::epoll_create1 wrapper
pub fn epoll_create1(epoll_flags: libc::c_int) -> Result<i32> {
    check_err(unsafe { libc::epoll_create1(epoll_flags) }).map(|epoll_fd| epoll_fd as i32)
}

/// libc::epoll_ctl wrapper
pub fn epoll_ctl(epoll_fd: i32, epoll_flags: libc::c_int, timer_fd: i32, event: &mut libc::epoll_event) -> Result<()> {
    check_err(unsafe {
        libc::epoll_ctl(epoll_fd, epoll_flags, timer_fd, event)
    })?;
    Ok(())
}

/// libc::epoll_wait wrapper
pub fn epoll_wait(epoll_fd: i32, events: *mut libc::epoll_event, maxevents: libc::c_int, timeout: libc::c_int) -> Result<()> {
    check_err(unsafe {
        libc::epoll_wait(epoll_fd, events, maxevents, timeout)
    })?;
    Ok(())
}

/// libc::read wrapper
pub fn read(timer_fd: i32, bufptr: *mut libc::c_void, count: libc::size_t) -> Result<()> {
    check_err(unsafe {
        libc::read(timer_fd, bufptr, count)
    })?;
    Ok(())
}
