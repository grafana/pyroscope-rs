use crate::backend::Tag;
use crate::{error::Result, PyroscopeError};

/// Error Wrapper for libc return. Only check for errors.
pub fn check_err<T: Ord + Default>(num: T) -> Result<T> {
    if num < T::default() {
        return Err(PyroscopeError::from(std::io::Error::last_os_error()));
    }
    Ok(num)
}

#[cfg(test)]
mod check_err_tests {
    use crate::utils::check_err;

    #[test]
    fn check_err_success() {
        assert_eq!(check_err(1).unwrap(), 1)
    }

    #[test]
    fn check_err_error() {
        assert!(check_err(-1).is_err())
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ThreadId {
    pthread: libc::pthread_t,
}

// SAFETY: pthread_t is an opaque thread identifier used as a handle,
// never dereferenced. On musl it's *mut c_void, on glibc it's c_ulong.
unsafe impl Send for ThreadId {}
unsafe impl Sync for ThreadId {}

impl From<libc::pthread_t> for ThreadId {
    fn from(value: libc::pthread_t) -> Self {
        Self { pthread: value }
    }
}
impl ThreadId {
    pub fn pthread_self() -> Self {
        Self {
            pthread: unsafe { libc::pthread_self() },
        }
    }

    pub fn to_string(&self) -> String {
        (self.pthread as usize).to_string()
    }
}

/// Return the current time in seconds.
pub fn get_current_time_secs() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

#[cfg(test)]
mod get_current_time_secs_tests {
    use crate::utils::get_current_time_secs;

    #[test]
    fn get_current_time_secs_success() {
        assert!(get_current_time_secs().is_ok())
    }
}

/// A representation of a time range. The time range is represented by a start
/// time, an end time, a current time and remaining time in seconds. The
/// remaining time is the duration in seconds until the end time.
#[derive(Debug, PartialEq)]
pub struct TimeRange {
    pub from: u64,
    pub until: u64,
    pub current: u64,
    pub rem: u64,
}

/// Return a range of timestamps in the form [start, end).
/// The range is inclusive of start and exclusive of end.
pub fn get_time_range(timestamp: u64) -> Result<TimeRange> {
    // if timestamp is 0, then get the current time
    if timestamp == 0 {
        return get_time_range(get_current_time_secs()?);
    }

    // Determine the start and end of the range
    Ok(TimeRange {
        from: timestamp / 10 * 10,
        until: timestamp / 10 * 10 + 10,
        current: timestamp,
        rem: 10 - (timestamp % 10),
    })
}

#[cfg(test)]
mod get_time_range_tests {
    use crate::utils::{get_time_range, TimeRange};

    #[test]
    fn get_time_range_verify() {
        assert_eq!(
            get_time_range(1644194479).unwrap(),
            TimeRange {
                from: 1644194470,
                until: 1644194480,
                current: 1644194479,
                rem: 1,
            }
        );
        assert_eq!(
            get_time_range(1644194470).unwrap(),
            TimeRange {
                from: 1644194470,
                until: 1644194480,
                current: 1644194470,
                rem: 10,
            }
        );
        assert_eq!(
            get_time_range(1644194476).unwrap(),
            TimeRange {
                from: 1644194470,
                until: 1644194480,
                current: 1644194476,
                rem: 4,
            }
        );
    }
}
