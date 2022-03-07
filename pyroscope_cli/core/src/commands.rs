use utils::app_config::AppConfig;
use utils::error::Result;

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

    let config = RbspyConfig::new(pid, 100, true, None, true);

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
    let (tx, rx) = channel();

    println!("connect command");
    let pid: i32 = AppConfig::get("pid")?;

    let config = RbspyConfig::new(pid, 100, true, None, true);

    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rbspy.basic")
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

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}
