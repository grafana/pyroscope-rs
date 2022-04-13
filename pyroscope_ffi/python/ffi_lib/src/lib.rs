use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Mutex, Once};

pub enum Signal {
    Kill,
    Tag,
}

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

#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn initialize_agent(
    application_name: *const c_char, server_address: *const c_char, sample_rate: u32,
    detect_subprocesses: bool, tags: *const c_char,
) -> bool {
    let application_name = unsafe { CStr::from_ptr(application_name) }
        .to_str()
        .unwrap();
    let server_address = unsafe { CStr::from_ptr(server_address) }.to_str().unwrap();
    let tags_string = unsafe { CStr::from_ptr(tags) }.to_str().unwrap();
    let tags = string_to_tags(tags_string);
    let pid = std::process::id();
    std::thread::spawn(move || {
        let pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
            .sample_rate(sample_rate)
            .lock_process(false)
            .with_subprocesses(detect_subprocesses);

        let pyspy = pyspy_backend(pyspy_config);
        let mut agent = PyroscopeAgent::builder(server_address, application_name)
            .backend(pyspy)
            .tags(tags)
            .build()
            .unwrap();

        agent.start().unwrap();

        let s = signalpass();

        while let Ok(signal) = s.inner_receiver.lock().unwrap().recv() {
            match signal {
                Signal::Kill => {
                    agent.stop().unwrap();
                    break;
                }
                Signal::Tag => {}
            }
        }
    });

    true
}

#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn drop_agent() -> bool {
    let s = signalpass();
    s.inner_sender.lock().unwrap().send(Signal::Kill).unwrap();
    true
}

// Convert a string of tags to a Vec<(&str, &str)>
fn string_to_tags(tags: &'static str) -> Vec<(&'static str, &'static str)> {
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
