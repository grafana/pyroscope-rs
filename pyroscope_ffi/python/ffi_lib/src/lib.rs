use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{Pyspy, PyspyConfig};

#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn initialize_agent() -> bool {
    std::thread::spawn(|| {
        let pid = std::process::id();
        let pyspy_config = PyspyConfig::new(pid.try_into().unwrap())
            .sample_rate(100)
            .lock_process(false)
            .with_subprocesses(true);

        let pyspy = Pyspy::new(pyspy_config);
        let mut agent = PyroscopeAgent::builder("http://localhost:4040", "pyspy-ffi")
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
