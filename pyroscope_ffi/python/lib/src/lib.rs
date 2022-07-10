use ffikit::Signal;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use pyroscope::backend::Tag;
use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use std::collections::hash_map::DefaultHasher;
use std::ffi::CStr;
use std::hash::Hasher;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Mutex, Once};

static PARENT_ID: AtomicU32 = AtomicU32::new(0);

//#[derive(Debug)]
//pub enum Signal {
//Kill,
//AddGlobalTag(String, String),
//RemoveGlobalTag(String, String),
//AddThreadTag(u64, String, String),
//RemoveThreadTag(u64, String, String),
//}

#[derive(Debug)]
pub struct SignalPass {
    inner_sender: Mutex<SyncSender<Signal>>,
    inner_receiver: Mutex<Receiver<Signal>>,
}

fn signalpass() -> &'static SignalPass {
    static mut SIGNAL_PASS: MaybeUninit<SignalPass> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    ONCE.call_once(|| unsafe {
        let (sender, receiver) = sync_channel(1);
        let singleton = SignalPass {
            inner_sender: Mutex::new(sender),
            inner_receiver: Mutex::new(receiver),
        };
        SIGNAL_PASS = MaybeUninit::new(singleton);
    });

    unsafe { SIGNAL_PASS.assume_init_ref() }
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
    sample_rate: u32, detect_subprocesses: bool, oncpu: bool, native: bool, gil_only: bool,
    report_pid: bool, report_thread_id: bool, report_thread_name: bool, tags: *const c_char,
) -> bool {
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
    std::thread::spawn(move || {
        let mut pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
            .sample_rate(sample_rate)
            .lock_process(false)
            .with_subprocesses(detect_subprocesses)
            .include_idle(!oncpu)
            .native(native)
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

        let tags_ref = tags_string.as_str();
        let tags = string_to_tags(tags_ref);
        let pyspy = pyspy_backend(pyspy_config);
        let mut agent_builder = PyroscopeAgent::builder(server_address, application_name)
            .backend(pyspy)
            .tags(tags);

        if auth_token != "" {
            agent_builder = agent_builder.auth_token(auth_token);
        }

        let agent = agent_builder.build().unwrap();

        let agent_running = agent.start().unwrap();

        //while let Ok(signal) = s.inner_receiver.lock().unwrap().recv() {
        //match signal {
        //Signal::Kill => {
        //agent_running.stop().unwrap();
        //break;
        //}
        //Signal::AddGlobalTag(name, value) => {
        //agent_running.add_global_tag(Tag::new(name, value)).unwrap();
        //}
        //Signal::RemoveGlobalTag(name, value) => {
        //agent_running
        //.remove_global_tag(Tag::new(name, value))
        //.unwrap();
        //}
        //Signal::AddThreadTag(thread_id, key, value) => {
        //let tag = Tag::new(key, value);
        //agent_running.add_thread_tag(thread_id, tag).unwrap();
        //}
        //Signal::RemoveThreadTag(thread_id, key, value) => {
        //let tag = Tag::new(key, value);
        //agent_running.remove_thread_tag(thread_id, tag).unwrap();
        //}
        //}
        //}

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
    });

    true
}

#[no_mangle]
pub extern "C" fn drop_agent() -> bool {
    let s = signalpass();
    s.inner_sender.lock().unwrap().send(Signal::Kill).unwrap();
    true
}

#[no_mangle]
pub extern "C" fn add_thread_tag(thread_id: u64, key: *const c_char, value: *const c_char) -> bool {
    //let s = signalpass();
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();
    //s.inner_sender
    //.lock()
    //.unwrap()
    //.send(Signal::AddThreadTag(thread_id, key, value))
    //.unwrap();
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
    //let s = signalpass();
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .to_owned();
    //s.inner_sender
    //.lock()
    //.unwrap()
    //.send(Signal::RemoveThreadTag(thread_id, key, value))
    //.unwrap();
    let pid = std::process::id();
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(thread_id % pid as u64);
    let id = hasher.finish();
    ffikit::send(ffikit::Signal::RemoveThreadTag(id, key, value)).unwrap();
    true
}

#[no_mangle]
pub extern "C" fn add_global_tag(key: *const c_char, value: *const c_char) -> bool {
    //let s = signalpass();
    //let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    //let value = unsafe { CStr::from_ptr(value) }
    //.to_str()
    //.unwrap()
    //.to_owned();
    //s.inner_sender
    //.lock()
    //.unwrap()
    //.send(Signal::AddGlobalTag(key, value))
    //.unwrap();
    true
}

#[no_mangle]
pub extern "C" fn remove_global_tag(key: *const c_char, value: *const c_char) -> bool {
    //let s = signalpass();
    //let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap().to_owned();
    //let value = unsafe { CStr::from_ptr(value) }
    //.to_str()
    //.unwrap()
    //.to_owned();
    //s.inner_sender
    //.lock()
    //.unwrap()
    //.send(Signal::RemoveGlobalTag(key, value))
    //.unwrap();
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
