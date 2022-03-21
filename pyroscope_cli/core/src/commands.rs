use utils::app_config::AppConfig;
use utils::error::{Error, Result};

use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{Pyspy, PyspyConfig};
use pyroscope_rbspy::{Rbspy, RbspyConfig};

use ctrlc;
use std::sync::mpsc::channel;

use crate::executor::Executor;
use crate::profiler::Profiler;

/// exec command
pub fn exec() -> Result<()> {
    // Get command to execute
    let command = AppConfig::get::<Option<String>>("command")?
        .ok_or_else(|| Error::new("command unwrap failed"))?;

    //dbg!(AppConfig::fetch()?);
    //return Ok(());

    // Get UID
    let uid = AppConfig::get::<Option<u32>>("user_name").unwrap_or(None);
    // Get GID
    let gid = AppConfig::get::<Option<u32>>("group_name").unwrap_or(None);

    // Create new executor and run it
    let executor = Executor::new(command.as_ref(), "", uid, gid).run()?;

    println!("stopped here?");

    // Set PID
    AppConfig::set("pid", executor.get_pid()?.to_string().as_str())?;

    // Initialize profiler
    let mut profiler = Profiler::default();

    profiler.init()?;

    let (tx, rx) = channel();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press Ctrl-C to exit.");

    rx.recv().unwrap();

    println!("Exiting.");

    executor.stop()?;
    profiler.stop()?;

    Ok(())
}

/// connect command
pub fn connect() -> Result<()> {
    // Initialize profiler
    let mut profiler = Profiler::default();

    profiler.init()?;

    let (tx, rx) = channel();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press Ctrl-C to exit.");

    rx.recv().unwrap();

    println!("Exiting.");

    profiler.stop()?;

    Ok(())
}

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}

pub fn rbspy() -> Result<()> {
    let (tx, rx) = channel();

    let pid: i32 = AppConfig::get("pid")?;
    let sample_rate: u32 = AppConfig::get("sample_rate")?;
    let lock_process: bool = AppConfig::get("rbspy_blocking")?;
    let with_subprocesses: bool = AppConfig::get("detect_subprocesses")?;

    let config = RbspyConfig::new(pid)
        .sample_rate(sample_rate)
        .lock_process(lock_process)
        .with_subprocesses(with_subprocesses);

    println!("Connecting to PID {}", pid);

    let server_address: String = AppConfig::get("server_address")?;

    let mut agent = PyroscopeAgent::builder(server_address, "rbspy.basic".to_string())
        .backend(Rbspy::new(config))
        .build()
        .unwrap();

    agent.start().unwrap();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press Ctrl-C to exit.");

    rx.recv().unwrap();

    println!("Exiting.");

    agent.stop().unwrap();

    drop(agent);

    Ok(())
}

pub fn pyspy() -> Result<()> {
    let (tx, rx) = channel();

    let pid: i32 = AppConfig::get("pid")?;
    let sample_rate: u32 = AppConfig::get("sample_rate")?;
    let lock_process: bool = AppConfig::get("pyspy_blocking")?;
    let with_subprocesses: bool = AppConfig::get("detect_subprocesses")?;

    let config = PyspyConfig::new(pid)
        .sample_rate(sample_rate)
        .lock_process(lock_process)
        .with_subprocesses(with_subprocesses);

    println!("Connecting to PID {}", pid);

    let server_address: String = AppConfig::get("server_address")?;

    let mut agent = PyroscopeAgent::builder(server_address, "pyspy.basic".to_string())
        .backend(Pyspy::new(config))
        .build()
        .unwrap();

    agent.start().unwrap();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press Ctrl-C to exit.");

    rx.recv().unwrap();

    println!("Exiting.");

    agent.stop().unwrap();

    drop(agent);

    Ok(())
}
