use pyo3::prelude::*;
use pyroscope::backend::Tag;
use pyroscope::pyroscope::ReportEncoding;
use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;

#[pyfunction]
fn initialize_logging(logging_level: u32) -> bool {
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

    pretty_env_logger::try_init_timed().is_ok()
}

#[pyfunction]
fn initialize_agent(
    application_name: String, server_address: String, auth_token: Option<String>,
    basic_auth_username: Option<String>, basic_auth_password: Option<String>, sample_rate: u32,
    detect_subprocesses: bool, oncpu: bool, gil_only: bool, report_pid: bool,
    report_thread_id: bool, report_thread_name: bool, tags: Option<HashMap<String, String>>,
    tenant_id: Option<String>, http_headers: Option<HashMap<String, String>>,
    line_no: Option<LineNo>,
) -> bool {
    let recv = match ffikit::initialize_ffi() {
        Ok(recv) => recv,
        Err(_) => return false, // todo return Err(PyErr)
    };

    let pid = std::process::id();

    let line_no = match line_no {
        None => LineNo::NoLine,
        Some(line_no) => line_no,
    };
    let mut pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
        .sample_rate(sample_rate)
        .lock_process(false)
        .detect_subprocesses(detect_subprocesses)
        .oncpu(oncpu)
        .native(false)
        .line_no(line_no.into())
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

    let pyspy = pyspy_backend(pyspy_config);

    let mut agent_builder = PyroscopeAgent::builder(server_address, application_name)
        .report_encoding(ReportEncoding::PPROF)
        .backend(pyspy);

    if let Some(tags) = tags {
        agent_builder = agent_builder.tags_map(tags);
    }
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
    if let Some(headers) = http_headers {
        agent_builder = agent_builder.http_headers(headers);
    };

    let agent = agent_builder.build().unwrap();

    let agent_running = agent.start().unwrap();

    std::thread::spawn(move || {
        while let Ok(signal) = recv.recv() {
            match signal {
                ffikit::Signal::Kill => {
                    agent_running.stop().unwrap();
                    break;
                }
                ffikit::Signal::AddGlobalTag(name, value) => {
                    agent_running.add_global_tag(Tag::new(name, value)).unwrap();
                }
                ffikit::Signal::RemoveGlobalTag(name, value) => {
                    agent_running
                        .remove_global_tag(Tag::new(name, value))
                        .unwrap();
                }
                ffikit::Signal::AddThreadTag(thread_id, key, value) => {
                    let tag = Tag::new(key, value);
                    agent_running.add_thread_tag(thread_id, tag).unwrap();
                }
                ffikit::Signal::RemoveThreadTag(thread_id, key, value) => {
                    let tag = Tag::new(key, value);
                    agent_running.remove_thread_tag(thread_id, tag).unwrap();
                }
            }
        }
    });
    true
}

#[pyfunction]
fn drop_agent() -> bool {
    ffikit::send(ffikit::Signal::Kill).is_ok()
}

#[pyfunction]
fn add_thread_tag(thread_id: u64, key: String, value: String) -> bool {
    let pid = std::process::id();
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(thread_id % pid as u64);
    let id = hasher.finish();

    ffikit::send(ffikit::Signal::AddThreadTag(id, key, value)).is_ok()
}

#[pyfunction]
fn remove_thread_tag(thread_id: u64, key: String, value: String) -> bool {
    let pid = std::process::id();
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(thread_id % pid as u64);
    let id = hasher.finish();

    ffikit::send(ffikit::Signal::RemoveThreadTag(id, key, value)).is_ok()
}

#[pyfunction]
fn add_global_tag(key: String, value: String) -> bool {
    ffikit::send(ffikit::Signal::AddGlobalTag(key, value)).is_ok()
}

#[pyfunction]
fn remove_global_tag(key: String, value: String) -> bool {
    ffikit::send(ffikit::Signal::RemoveGlobalTag(key, value)).is_ok()
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
    m.add_function(wrap_pyfunction!(initialize_logging, m)?)?;
    m.add_function(wrap_pyfunction!(initialize_agent, m)?)?;
    m.add_function(wrap_pyfunction!(drop_agent, m)?)?;
    m.add_function(wrap_pyfunction!(add_thread_tag, m)?)?;
    m.add_function(wrap_pyfunction!(remove_thread_tag, m)?)?;
    m.add_function(wrap_pyfunction!(add_global_tag, m)?)?;
    m.add_function(wrap_pyfunction!(remove_global_tag, m)?)?;
    m.add_class::<LineNo>()?;
    Ok(())
}
