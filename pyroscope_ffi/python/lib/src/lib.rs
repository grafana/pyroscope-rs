use ffikit::Signal;
use pyroscope::backend::Tag;
use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use std::collections::hash_map::DefaultHasher;
use std::ffi::CStr;
use std::hash::Hasher;
use std::os::raw::c_char;

const LOG_TAG: &str = "Pyroscope::pyspy::ffi";

#[no_mangle]
pub extern "C" fn initialize_logging(logging_level: u32) -> bool {
    // Force rustc to display the log messages in the console.
    match logging_level {
        50 => {
            std::env::set_var("RUST_LOG", "off");
        }
        40 => {
            std::env::set_var("RUST_LOG", "error");
        }
        30 => {
            std::env::set_var("RUST_LOG", "warn");
        }
        20 => {
            std::env::set_var("RUST_LOG", "info");
        }
        10 => {
            std::env::set_var("RUST_LOG", "debug");
        }
        _ => {
            std::env::set_var("RUST_LOG", "debug");
        }
    }

    // Initialize the logger.
    pretty_env_logger::init_timed();
    true
}

#[no_mangle]
pub extern "C" fn initialize_agent(
    application_name: *const c_char,
    server_address: *const c_char,
    auth_token: *const c_char,
    basic_auth_username: *const c_char,
    basic_auth_password: *const c_char,
    sample_rate: u32,
    detect_subprocesses: bool,
    oncpu: bool,
    native: bool,
    gil_only: bool,
    report_pid: bool,
    report_thread_id: bool,
    report_thread_name: bool,
    tags: *const c_char,
    tenant_id: *const c_char,
    http_headers_json: *const c_char,
) -> bool {
    // Initialize FFIKit
    let recv = ffikit::initialize_ffi().unwrap();

    // application_name
    let application_name = unsafe { CStr::from_ptr(application_name) }
        .to_str()
        .unwrap()
        .to_string();

    // server_address
    let server_address = unsafe { CStr::from_ptr(server_address) }
        .to_str()
        .unwrap()
        .to_string();

    let auth_token = unsafe { CStr::from_ptr(auth_token) }
        .to_str()
        .unwrap()
        .to_string();

    let basic_auth_username = unsafe { CStr::from_ptr(basic_auth_username) }
        .to_str()
        .unwrap()
        .to_string();

    let basic_auth_password = unsafe { CStr::from_ptr(basic_auth_password) }
        .to_str()
        .unwrap()
        .to_string();

    // tags
    let tags_string = unsafe { CStr::from_ptr(tags) }
        .to_str()
        .unwrap()
        .to_string();

    let tenant_id = unsafe { CStr::from_ptr(tenant_id) }
        .to_str()
        .unwrap()
        .to_string();

    let http_headers_json = unsafe { CStr::from_ptr(http_headers_json) }
        .to_str()
        .unwrap()
        .to_string();

    let pid = std::process::id();

    // Configure the Pyspy Backend.
    let mut pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
        .sample_rate(sample_rate)
        .lock_process(false)
        .detect_subprocesses(detect_subprocesses)
        .oncpu(oncpu)
        .native(native)
        .gil_only(gil_only);

    // Report the PID.
    if report_pid {
        pyspy_config = pyspy_config.report_pid();
    }

    // Report thread IDs.
    if report_thread_id {
        pyspy_config = pyspy_config.report_thread_id();
    }

    // Report thread names.
    if report_thread_name {
        pyspy_config = pyspy_config.report_thread_name();
    }

    // Convert the tags to a vector of strings.
    let tags_ref = tags_string.as_str();
    let tags = string_to_tags(tags_ref);

    // Create the Pyspy Backend.
    let pyspy = pyspy_backend(pyspy_config);

    // Create the Pyroscope Agent.
    let mut agent_builder = PyroscopeAgent::builder(server_address, application_name)
        .backend(pyspy)
        .tags(tags);

    // Add the auth token if it is not empty.
    if auth_token != "" {
        agent_builder = agent_builder.auth_token(auth_token);
    } else if basic_auth_username != "" && basic_auth_password != "" {
        agent_builder = agent_builder.basic_auth(basic_auth_username, basic_auth_password);
    }
    if tenant_id != "" {
        agent_builder = agent_builder.tenant_id(tenant_id);
    }

    let http_headers = pyroscope::pyroscope::parse_http_headers_json(http_headers_json);
    match http_headers {
        Ok(http_headers) => {
            agent_builder = agent_builder.http_headers(http_headers);
        }
        Err(e) => {
            match e {
                pyroscope::PyroscopeError::Json(e) => {
                    log::error!(target: LOG_TAG, "parse_http_headers_json error {}", e);
                }
                pyroscope::PyroscopeError::AdHoc(e) => {
                    log::error!(target: LOG_TAG, "parse_http_headers_json {}", e);
                }
                _ => {}
            }
        }
    }

    // Build the agent.
    let agent = agent_builder.build().unwrap();

    // Start the agent.
    let agent_running = agent.start().unwrap();

    // Spawn a thread to receive signals from the FFIKit merge channel.
    std::thread::spawn(move || {
        while let Ok(signal) = recv.recv() {
            match signal {
                Signal::Kill => {
                    agent_running.stop().unwrap();
                    break;
                }
                Signal::AddGlobalTag(name, value) => {
                    agent_running.add_global_tag(Tag::new(name, value)).unwrap();
                }
                Signal::RemoveGlobalTag(name, value) => {
                    agent_running
                        .remove_global_tag(Tag::new(name, value))
                        .unwrap();
                }
                Signal::AddThreadTag(thread_id, key, value) => {
                    let tag = Tag::new(key, value);
                    agent_running.add_thread_tag(thread_id, tag).unwrap();
                }
                Signal::RemoveThreadTag(thread_id, key, value) => {
                    let tag = Tag::new(key, value);
                    agent_running.remove_thread_tag(thread_id, tag).unwrap();
                }
            }
        }
    });

    true
}

#[no_mangle]
pub extern "C" fn drop_agent() -> bool {
    return ffikit::send(ffikit::Signal::Kill).is_ok();
}

#[no_mangle]
pub extern "C" fn add_thread_tag(thread_id: u64, key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    let pid = std::process::id();
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(thread_id % pid as u64);
    let id = hasher.finish();

    return ffikit::send(ffikit::Signal::AddThreadTag(id, key, value)).is_ok();
}

#[no_mangle]
pub extern "C" fn remove_thread_tag(
    thread_id: u64, key: *const c_char, value: *const c_char,
) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    let pid = std::process::id();
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(thread_id % pid as u64);
    let id = hasher.finish();

    return ffikit::send(ffikit::Signal::RemoveThreadTag(id, key, value)).is_ok();
}

#[no_mangle]
pub extern "C" fn add_global_tag(key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    return ffikit::send(ffikit::Signal::AddGlobalTag(key, value)).is_ok();
}

#[no_mangle]
pub extern "C" fn remove_global_tag(key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    return ffikit::send(ffikit::Signal::RemoveGlobalTag(key, value)).is_ok();
}

// Convert a string of tags to a Vec<(&str, &str)>
fn string_to_tags<'a>(tags: &'a str) -> Vec<(&'a str, &'a str)> {
    let mut tags_vec = Vec::new();

    // check if string is empty
    if tags.is_empty() {
        return tags_vec;
    }

    for tag in tags.split(',') {
        let mut tag_split = tag.split('=');
        let key = tag_split.next().unwrap();
        let value = tag_split.next().unwrap();
        tags_vec.push((key, value));
    }

    tags_vec
}
