use pyroscope::PyroscopeAgent;
use pyroscope_rbspy::{Rbspy, RbspyConfig};
use std::ffi::CStr;
use std::os::raw::c_char;

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
    std::thread::spawn(move || {
        let pid = std::process::id();
        let rbspy_config = RbspyConfig::new(pid.try_into().unwrap())
            .sample_rate(sample_rate)
            .lock_process(false)
            .with_subprocesses(detect_subprocesses);

        let rbspy = Rbspy::new(rbspy_config);
        let mut agent = PyroscopeAgent::builder(server_address, application_name)
            .backend(rbspy)
            .tags(tags)
            .build()
            .unwrap();

        agent.start().unwrap();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000000000));
        }
    });

    true
}
#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn drop_agent() -> bool {
    true
}

// Convert a string of tags to a Vec<(&str, &str)>
fn string_to_tags(tags: &'static str) -> Vec<(&'static str, &'static str)> {
    let mut tags_vec = Vec::new();
    for tag in tags.split(',') {
        let mut tag_split = tag.split('=');
        let key = tag_split.next().unwrap();
        let value = tag_split.next().unwrap();
        tags_vec.push((key, value));
    }

    tags_vec
}
