use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{Pyspy, PyspyConfig};

#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn initialize_agent(
    application_name: String, server_address: String, sample_rate: u32, detect_subprocesses: bool,
    log_level: String, tag: Option<String>,
) -> bool {
    std::thread::spawn(move || {
        let pid = std::process::id();
        let pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
            .sample_rate(sample_rate)
            .lock_process(false)
            .with_subprocesses(detect_subprocesses);

        let pyspy = Pyspy::new(pyspy_config);
        let mut agent = PyroscopeAgent::builder(server_address, application_name)
            .backend(pyspy)
            .build()
            .unwrap();

        agent.start().unwrap();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000000000));
        }
    });

    true
}
