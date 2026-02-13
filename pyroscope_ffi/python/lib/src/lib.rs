mod backend;

use crate::backend::Pyspy;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyroscope::backend::{BackendConfig, BackendImpl, Tag};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;

const LOG_TAG: &str = "Pyroscope::pyspy::ffi";
const PYSPY_NAME: &str = "pyspy";
const PYSPY_VERSION: &str = "1.0.1";

#[no_mangle]
pub extern "C" fn initialize_logging(logging_level: u32) -> bool {
    // Force rustc to display the log messages in the console.
    match logging_level {
        50 => {
            unsafe { std::env::set_var("RUST_LOG", "off") };
        }
        40 => {
            unsafe { std::env::set_var("RUST_LOG", "error") };
        }
        30 => {
            unsafe { std::env::set_var("RUST_LOG", "warn") };
        }
        20 => {
            unsafe { std::env::set_var("RUST_LOG", "info") };
        }
        10 => {
            unsafe { std::env::set_var("RUST_LOG", "debug") };
        }
        _ => {
            unsafe { std::env::set_var("RUST_LOG", "debug") };
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
    basic_auth_username: *const c_char,
    basic_auth_password: *const c_char,
    sample_rate: u32,
    oncpu: bool,
    gil_only: bool,
    report_pid: bool,
    report_thread_id: bool,
    report_thread_name: bool,
    tags: *const c_char,
    tenant_id: *const c_char,
    http_headers_json: *const c_char,
    line_no: LineNo,
) -> bool {
    let application_name = unsafe { CStr::from_ptr(application_name) }
        .to_str()
        .unwrap()
        .to_string();

    let server_address = unsafe { CStr::from_ptr(server_address) }
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

    let backend_config = BackendConfig {
        report_thread_id,
        report_thread_name,
        report_pid,
    };

    let pid = pid.try_into().unwrap();

    let config = py_spy::Config {
        blocking: py_spy::config::LockingStrategy::NonBlocking,
        native: false,
        pid: Some(pid),
        sampling_rate: sample_rate.into(),
        include_idle: !oncpu,
        include_thread_ids: true,
        subprocesses: false,
        gil_only,
        lineno: line_no.into(),
        duration: py_spy::config::RecordDuration::Unlimited,
        ..py_spy::Config::default()
    };

    let tags_ref = tags_string.as_str();
    let tags = string_to_tags(tags_ref);

    let pyspy = BackendImpl::new(Box::new(Pyspy::new(config, backend_config)));

    let mut agent_builder = PyroscopeAgentBuilder::new(
        server_address,
        application_name,
        sample_rate,
        PYSPY_NAME,
        PYSPY_VERSION,
        pyspy,
    )
    .tags(tags);

    if basic_auth_username != "" && basic_auth_password != "" {
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
        Err(e) => match e {
            pyroscope::PyroscopeError::Json(e) => {
                log::error!(target: LOG_TAG, "parse_http_headers_json error {}", e);
            }
            pyroscope::PyroscopeError::AdHoc(e) => {
                log::error!(target: LOG_TAG, "parse_http_headers_json {}", e);
            }
            _ => {}
        },
    }

    pyroscope::ffikit::run(agent_builder).is_ok()
}

#[no_mangle]
pub extern "C" fn drop_agent() -> bool {
    pyroscope::ffikit::send(pyroscope::ffikit::Signal::Kill).is_ok()
}

#[no_mangle]
pub extern "C" fn add_thread_tag(key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    pyroscope::ffikit::send(pyroscope::ffikit::Signal::AddThreadTag(
        self_thread_id(),
        Tag { key, value },
    ))
    .is_ok()
}

#[no_mangle]
pub extern "C" fn remove_thread_tag(key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    pyroscope::ffikit::send(pyroscope::ffikit::Signal::RemoveThreadTag(
        self_thread_id(),
        Tag { key, value },
    ))
    .is_ok()
}

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

#[repr(C)]
#[derive(Debug)]
pub enum LineNo {
    LastInstruction = 0,
    First = 1,
    NoLine = 2,
}

impl Into<py_spy::config::LineNo> for LineNo {
    fn into(self) -> py_spy::config::LineNo {
        match self {
            LineNo::LastInstruction => py_spy::config::LineNo::LastInstruction,
            LineNo::First => py_spy::config::LineNo::First,
            LineNo::NoLine => py_spy::config::LineNo::NoLine,
        }
    }
}

pub fn self_thread_id() -> pyroscope::ThreadId {
    // https://github.com/python/cpython/blob/main/Python/thread_pthread.h#L304
    pyroscope::ThreadId::pthread_self()
}

#[pyfunction(name = "initialize_logging")]
fn py_initialize_logging(logging_level: u32) -> bool {
    initialize_logging(logging_level)
}

#[pyfunction(name = "initialize_agent")]
#[allow(clippy::too_many_arguments)]
fn py_initialize_agent(
    application_name: &str,
    server_address: &str,
    basic_auth_username: &str,
    basic_auth_password: &str,
    sample_rate: u32,
    oncpu: bool,
    gil_only: bool,
    report_pid: bool,
    report_thread_id: bool,
    report_thread_name: bool,
    tags: &str,
    tenant_id: &str,
    http_headers_json: &str,
    line_no: u8,
) -> PyResult<bool> {
    let line_no = match line_no {
        0 => LineNo::LastInstruction,
        1 => LineNo::First,
        2 => LineNo::NoLine,
        _ => return Err(PyValueError::new_err("line_no must be 0, 1, or 2")),
    };

    let application_name =
        CString::new(application_name).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let server_address =
        CString::new(server_address).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let basic_auth_username =
        CString::new(basic_auth_username).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let basic_auth_password =
        CString::new(basic_auth_password).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let tags = CString::new(tags).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let tenant_id = CString::new(tenant_id).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let http_headers_json =
        CString::new(http_headers_json).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(initialize_agent(
        application_name.as_ptr(),
        server_address.as_ptr(),
        basic_auth_username.as_ptr(),
        basic_auth_password.as_ptr(),
        sample_rate,
        oncpu,
        gil_only,
        report_pid,
        report_thread_id,
        report_thread_name,
        tags.as_ptr(),
        tenant_id.as_ptr(),
        http_headers_json.as_ptr(),
        line_no,
    ))
}

#[pyfunction(name = "drop_agent")]
fn py_drop_agent() -> bool {
    drop_agent()
}

#[pyfunction(name = "add_thread_tag")]
fn py_add_thread_tag(key: &str, value: &str) -> PyResult<bool> {
    let key = CString::new(key).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let value = CString::new(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(add_thread_tag(key.as_ptr(), value.as_ptr()))
}

#[pyfunction(name = "remove_thread_tag")]
fn py_remove_thread_tag(key: &str, value: &str) -> PyResult<bool> {
    let key = CString::new(key).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let value = CString::new(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(remove_thread_tag(key.as_ptr(), value.as_ptr()))
}

#[pymodule]
fn _native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_initialize_logging, m)?)?;
    m.add_function(wrap_pyfunction!(py_initialize_agent, m)?)?;
    m.add_function(wrap_pyfunction!(py_drop_agent, m)?)?;
    m.add_function(wrap_pyfunction!(py_add_thread_tag, m)?)?;
    m.add_function(wrap_pyfunction!(py_remove_thread_tag, m)?)?;
    Ok(())
}
