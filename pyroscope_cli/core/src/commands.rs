use utils::app_config::AppConfig;
use utils::error::{Error, Result};
use utils::types::{LogLevel, Spy};

use pyroscope::PyroscopeAgent;
use pyroscope_pyspy::{Pyspy, PyspyConfig};
use pyroscope_rbspy::{Rbspy, RbspyConfig};

use ctrlc;
use std::sync::mpsc::channel;

use crate::executor::Executor;
use crate::profiler::Profiler;

/// exec command
pub fn exec() -> Result<()> {
    // TODO: this is hacky
    set_application_name()?;
    set_tags()?;

    // Get command to execute
    let command = AppConfig::get::<Option<String>>("command")?
        .ok_or_else(|| Error::new("command unwrap failed"))?;

    // Get UID
    let uid = AppConfig::get::<Option<u32>>("user_name").unwrap_or(None);
    // Get GID
    let gid = AppConfig::get::<Option<u32>>("group_name").unwrap_or(None);

    // Create new executor and run it
    let executor = Executor::new(command.as_ref(), "", uid, gid).run()?;

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
    // TODO: this is hacky
    set_application_name()?;
    set_tags()?;

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

fn set_application_name() -> Result<()> {
    let pre_app_name: String = AppConfig::get::<String>("application_name").unwrap_or_else(|_| {
        names::Generator::default()
            .next()
            .unwrap_or("unassigned.app".to_string())
            .replace("-", ".")
    });

    let pre = match AppConfig::get::<Spy>("spy_name")? {
        Spy::Pyspy => "pyspy",
        Spy::Rbspy => "rbspy",
        _ => "none",
    };

    // add pre to pre_app_name
    let app_name = format!("{}.{}", pre, pre_app_name);

    AppConfig::set("application_name", app_name.as_str())?;

    Ok(())
}

fn set_tags() -> Result<()> {
    let tag: String = AppConfig::get::<String>("tag").unwrap_or("".to_string());

    AppConfig::set("tag", tag.as_str())?;

    Ok(())
}
