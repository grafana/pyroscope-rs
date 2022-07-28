use ffikit::Signal;
use pyroscope::backend::{Report, StackFrame, Tag};
use pyroscope::PyroscopeAgent;
use pyroscope_rbspy::{rbspy_backend, RbspyConfig};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::ffi::CStr;
use std::hash::Hasher;
use std::os::raw::c_char;

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
    application_name: *const c_char, server_address: *const c_char, auth_token: *const c_char,
    sample_rate: u32, detect_subprocesses: bool, on_cpu: bool, report_pid: bool,
    report_thread_id: bool, tags: *const c_char,
) -> bool {
    // Initialize FFIKit
    let recv = ffikit::initialize_ffi().unwrap();

    let application_name = unsafe { CStr::from_ptr(application_name) }
        .to_str()
        .unwrap()
        .to_string();

    let server_address = unsafe { CStr::from_ptr(server_address) }
        .to_str()
        .unwrap()
        .to_string();

    let auth_token = unsafe { CStr::from_ptr(auth_token) }
        .to_str()
        .unwrap()
        .to_string();

    let tags_string = unsafe { CStr::from_ptr(tags) }
        .to_str()
        .unwrap()
        .to_string();

    let pid = std::process::id();

    let rbspy_config = RbspyConfig::new(pid.try_into().unwrap())
        .sample_rate(sample_rate)
        .lock_process(false)
        .with_subprocesses(detect_subprocesses)
        .on_cpu(on_cpu)
        .report_pid(report_pid)
        .report_thread_id(report_thread_id);

    let tags_ref = tags_string.as_str();
    let tags = string_to_tags(tags_ref);
    let rbspy = rbspy_backend(rbspy_config);

    let mut agent_builder = PyroscopeAgent::builder(server_address, application_name)
        .backend(rbspy)
        .func(transform_report)
        .tags(tags);

    if auth_token != "" {
        agent_builder = agent_builder.auth_token(auth_token);
    }

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

    true
}

#[no_mangle]
pub extern "C" fn drop_agent() -> bool {
    // Send Kill signal to the FFI merge channel.
    ffikit::send(ffikit::Signal::Kill).unwrap();

    true
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

    ffikit::send(ffikit::Signal::AddThreadTag(id, key, value)).unwrap();

    true
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

    ffikit::send(ffikit::Signal::RemoveThreadTag(id, key, value)).unwrap();

    true
}

#[no_mangle]
pub extern "C" fn add_global_tag(key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    ffikit::send(ffikit::Signal::AddGlobalTag(key, value)).unwrap();

    true
}

#[no_mangle]
pub extern "C" fn remove_global_tag(key: *const c_char, value: *const c_char) -> bool {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();

    ffikit::send(ffikit::Signal::RemoveGlobalTag(key, value)).unwrap();

    true
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
