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
