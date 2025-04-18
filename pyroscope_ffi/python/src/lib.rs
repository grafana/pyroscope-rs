use ffikit::Signal;
use pyo3::prelude::*;
use pyroscope::backend::Tag;
use pyroscope::pyroscope::ReportEncoding;
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

#[pyfunction]
fn initialize_agent(
    application_name: String, server_address: String, auth_token: Option<String>,
    basic_auth_username: Option<String>, basic_auth_password: Option<String>, sample_rate: u32,
    detect_subprocesses: bool, oncpu: bool, gil_only: bool, report_pid: bool,
    report_thread_id: bool, report_thread_name: bool, tags: Option<Vec<(String, String)>>,
    tenant_id: Option<String>, http_headers_json: Option<String>,
    line_no: Option<LineNo>,
) -> PyResult<()> {
    let recv = ffikit::initialize_ffi().unwrap(); //todo do not unwrap

    let pid = std::process::id();

    let mut pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
        .sample_rate(sample_rate)
        .lock_process(false)
        .detect_subprocesses(detect_subprocesses)
        .oncpu(oncpu)
        .native(false)
        // .line_no(line_no.into()) //todo
        .gil_only(gil_only);

    if report_pid {
        pyspy_config = pyspy_config.report_pid();
    }

    if report_thread_id {
        pyspy_config = pyspy_config.report_thread_id();
    }

    if report_thread_name {
        pyspy_config = pyspy_config.report_thread_name();
    }

    // Convert the tags to a vector of strings.
    // let tags_ref = tags.as_str();
    // let tags = string_to_tags(tags_ref);

    let pyspy = pyspy_backend(pyspy_config);

    let mut agent_builder = PyroscopeAgent::builder(server_address, application_name)
        .report_encoding(ReportEncoding::PPROF)
        .backend(pyspy);
    // .tags(tags); //todo

    if let Some(auth_token) = auth_token {
        agent_builder = agent_builder.auth_token(auth_token);
    } else if let (Some(basic_auth_username), Some(basic_auth_password)) =
        (basic_auth_username, basic_auth_password)
    {
        agent_builder = agent_builder.basic_auth(basic_auth_username, basic_auth_password);
    }
    if let Some(tenant_id) = tenant_id {
        agent_builder = agent_builder.tenant_id(tenant_id);
    }

    // let http_headers = pyroscope::pyroscope::parse_http_headers_json(http_headers_json);
    // match http_headers {
    //     Ok(http_headers) => {
    //         agent_builder = agent_builder.http_headers(http_headers);
    //     }
    //     Err(e) => match e {
    //         pyroscope::PyroscopeError::Json(e) => {
    //             log::error!(target: LOG_TAG, "parse_http_headers_json error {}", e);
    //         }
    //         pyroscope::PyroscopeError::AdHoc(e) => {
    //             log::error!(target: LOG_TAG, "parse_http_headers_json {}", e);
    //         }
    //         _ => {}
    //     },
    // }

    let agent = agent_builder.build().unwrap();

    let agent_running = agent.start().unwrap();

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

    Ok(())
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


#[derive(Eq, PartialEq, Hash, Debug, Clone)]
#[pyclass(eq, eq_int)]
pub enum LineNo {
    LastInstruction,
    First,
    NoLine,
}

impl Into<pyroscope_pyspy::LineNo> for LineNo {
    fn into(self) -> pyroscope_pyspy::LineNo {
        match self {
            LineNo::LastInstruction => pyroscope_pyspy::LineNo::LastInstruction,
            LineNo::First => pyroscope_pyspy::LineNo::First,
            LineNo::NoLine => pyroscope_pyspy::LineNo::NoLine,
        }
    }
}

#[pymodule]
fn python_wheel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(initialize_agent, m)?)?;
    m.add_class::<LineNo>()?;
    Ok(())
}
