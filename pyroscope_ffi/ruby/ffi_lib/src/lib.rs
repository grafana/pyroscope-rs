use pyroscope::PyroscopeAgent;
use pyroscope_rbspy::{Rbspy, RbspyConfig};
use rutie::{RString, VM};
use std::ffi::CStr;
use std::os::raw::c_char;

#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn initialize_agent(
    application_name: *const c_char, server_address: *const c_char, sample_rate: u32,
    detect_subprocesses: bool,
) -> bool {
    // Convert the C string to a Rust string
    let application_name = unsafe { CStr::from_ptr(application_name) }
        .to_str()
        .unwrap();
    let server_address = unsafe { CStr::from_ptr(server_address) }.to_str().unwrap();
    std::thread::spawn(move || {
        let pid = std::process::id();
        let rbspy_config = RbspyConfig::new(pid.try_into().unwrap())
            .sample_rate(sample_rate)
            .lock_process(false)
            .with_subprocesses(detect_subprocesses);

        let rbspy = Rbspy::new(rbspy_config);
        let mut agent = PyroscopeAgent::builder(server_address, application_name)
            .backend(rbspy)
            .build()
            .unwrap();

        agent.start().unwrap();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000000000));
        }
    });

    true
}
