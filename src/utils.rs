use crate::{error::Result, PyroscopeError};

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
mod merge_tags_with_app_name_tests {
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
