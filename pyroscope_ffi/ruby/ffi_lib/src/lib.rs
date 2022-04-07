use pyroscope::PyroscopeAgent;
use pyroscope_rbspy::{Rbspy, RbspyConfig};

#[link(name = "pyroscope_ffi", vers = "0.1")]
#[no_mangle]
pub fn initialize_agent() -> bool {
    std::thread::spawn(|| {
        let pid = std::process::id();
        let rbspy_config = RbspyConfig::new(pid.try_into().unwrap())
            .sample_rate(100)
            .lock_process(true)
            .with_subprocesses(true);

        let rbspy = Rbspy::new(rbspy_config);
        let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rubyspy-ffi")
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
