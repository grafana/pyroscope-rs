#![cfg(target_arch = "x86_64")]

use core::ffi::{CStr, c_char, c_int};

/// Start the CPython profiler.
///
/// Parameters:
/// - `app_name`: application name (NUL-terminated C string, or null).
/// - `server_url`: server URL (NUL-terminated C string, or null).
/// - `num_shards`: number of shards (0 = use default 16). Must be >= 1.
/// - `log_enabled`: if nonzero, print diagnostic messages to stderr.
///
/// Returns 0 on success, nonzero error code on failure:
/// - 1: kindasafe init failed
/// - 2: Python binary not found
/// - 3: ELF symbol not found
/// - 4: debug offsets validation failed
/// - 5: unsupported Python version
/// - 7: memory allocation / resource creation failed
/// - 8: signal handler installation failed
/// - 9: profiler already running
/// - 10: kindasafe sanity check failed (crash recovery not working)
///
/// # Safety
///
/// `app_name` and `server_url` must be valid pointers to NUL-terminated
/// C strings, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pyroscope_start(
    app_name: *const c_char,
    server_url: *const c_char,
    num_shards: c_int,
    log_enabled: c_int,
) -> c_int {
    let app_name = if app_name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(app_name) }
            .to_string_lossy()
            .into_owned()
    };

    let server_url = if server_url.is_null() {
        None
    } else {
        let s = unsafe { CStr::from_ptr(server_url) }
            .to_string_lossy()
            .into_owned();
        if s.is_empty() { None } else { Some(s) }
    };

    let num_shards = if num_shards <= 0 {
        0
    } else {
        num_shards as usize
    };

    match pysignalprof::start(
        app_name,
        server_url,
        num_shards,
        log_enabled != 0,
        Vec::new(),
    ) {
        Ok(()) => 0,
        Err(code) => code as c_int,
    }
}
