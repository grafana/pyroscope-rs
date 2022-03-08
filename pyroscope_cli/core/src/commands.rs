use pyroscope::pyroscope_backends::pyspy::{Pyspy, PyspyConfig};
use utils::app_config::AppConfig;
use utils::error::Result;
use utils::types::Spy;

use pyroscope::pyroscope_backends::rbspy::{Rbspy, RbspyConfig};
use pyroscope::PyroscopeAgent;

use ctrlc;
use std::sync::mpsc::channel;

use duct::cmd;

/// adhoc command
pub fn adhoc() -> Result<()> {
    println!("adhoc command");
    Ok(())
}

/// exec command
pub fn exec() -> Result<()> {
    let (tx, rx) = channel();

    let handle = cmd!("ruby", "./scripts/ruby.rb").stdout_capture().start()?;
    let pids = handle.pids();

    let pid = *pids.get(0).unwrap() as i32;

    let config = RbspyConfig::new(pid);

    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rbspy.basic")
        .backend(Rbspy::new(config))
        .build()
        .unwrap();

    agent.start().unwrap();

    //handle.wait()?;

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press Ctrl-C to exit.");

    rx.recv().unwrap();

    println!("Exiting.");

    agent.stop().unwrap();

    drop(agent);

    handle.kill()?;

    Ok(())
}

/// connect command
pub fn connect() -> Result<()> {
    let spy: Spy = AppConfig::get("spy_name")?;

    dbg!(spy);
    match spy {
        Spy::Rbspy => {
            rbspy()?;
        }
        Spy::Pyspy => {
            pyspy()?;
        }
        _ => {
            println!("not supported spy");
        }
    }

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
