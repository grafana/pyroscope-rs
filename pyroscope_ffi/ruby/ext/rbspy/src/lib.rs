mod backend;

use rbspy::sampler::Sampler;
use remoteprocess::Pid;
use std::env;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::backend::Rbspy;
use pyroscope;
use pyroscope::backend::{BackendConfig, BackendImpl, Report, StackFrame, Tag};
use pyroscope::pyroscope::PyroscopeAgentBuilder;

const LOG_TAG: &str = "Pyroscope::rbspy::ffi";
const RBSPY_NAME: &str = "rbspy";
const RBSPY_VERSION: &str = "0.6.9";

pub fn transform_report(report: Report) -> Report {
    let cwd = env::current_dir().unwrap();
    let cwd = cwd.to_str().unwrap_or("");

    let data = report
        .data
        .iter()
        .map(|(stacktrace, count)| {
            let new_frames = stacktrace
                .frames
                .iter()
                .map(|frame| {
                    let frame = frame.to_owned();
                    let mut s = frame.filename.unwrap();
                    match s.find(cwd) {
                        Some(i) => {
                            s = s[(i + cwd.len() + 1)..].to_string();
                        }
                        None => match s.find("/gems/") {
                            Some(i) => {
                                s = s[(i + 1)..].to_string();
                            }
                            None => match s.find("/ruby/") {
                                Some(i) => {
                                    s = s[(i + 6)..].to_string();
                                    match s.find("/") {
                                        Some(i) => {
                                            s = s[(i + 1)..].to_string();
                                        }
                                        None => {}
                                    }
                                }
                                None => {}
                            },
                        },
                    }

                    // something
                    StackFrame::new(
                        frame.module,
                        frame.name,
                        Some(s.to_string()),
                        frame.relative_path,
                        frame.absolute_path,
                        frame.line,
                    )
                })
                .collect();

            let mut mystack = stacktrace.to_owned();

            mystack.frames = new_frames;

            (mystack, count.to_owned())
        })
        .collect();

    let new_report = Report::new(data).metadata(report.metadata.clone());

    new_report
}

#[no_mangle]
pub extern "C" fn initialize_logging(logging_level: u32) -> bool {
    // Force rustc to display the log messages in the console.
    match logging_level {
        50 => {
            std::env::set_var("RUST_LOG", "error");
        }
        40 => {
            std::env::set_var("RUST_LOG", "warn");
        }
        30 => {
            std::env::set_var("RUST_LOG", "info");
        }
        20 => {
            std::env::set_var("RUST_LOG", "debug");
        }
        10 => {
            std::env::set_var("RUST_LOG", "trace");
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
    basic_auth_user: *const c_char,
    basic_auth_password: *const c_char,
    sample_rate: u32,
    oncpu: bool,
    report_pid: bool,
    report_thread_id: bool,
    tags: *const c_char,
    tenant_id: *const c_char,
    http_headers_json: *const c_char,
) -> bool {
    let application_name = unsafe { CStr::from_ptr(application_name) }
        .to_str()
        .unwrap()
        .to_string();

    let server_address = unsafe { CStr::from_ptr(server_address) }
        .to_str()
        .unwrap()
        .to_string();

    let basic_auth_user = unsafe { CStr::from_ptr(basic_auth_user) }
        .to_str()
        .unwrap()
        .to_string();

    let basic_auth_password = unsafe { CStr::from_ptr(basic_auth_password) }
        .to_str()
        .unwrap()
        .to_string();

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
        report_thread_name: false,
        report_pid,
    };

    let sampler = Sampler::new(pid as Pid, sample_rate, false, None, false, None, oncpu);

    let tags_ref = tags_string.as_str();
    let tags = string_to_tags(tags_ref);
    let rbspy = BackendImpl::new(Box::new(Rbspy::new(sampler, sample_rate, backend_config)));

    let mut agent_builder = PyroscopeAgentBuilder::new(
        server_address,
        application_name,
        sample_rate,
        RBSPY_NAME,
        RBSPY_VERSION,
        rbspy,
    )
    .func(transform_report)
    .tags(tags);

    if basic_auth_user != "" && basic_auth_password != "" {
        agent_builder = agent_builder.basic_auth(basic_auth_user, basic_auth_password);
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
        backend::self_thread_id(),
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
        backend::self_thread_id(),
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
