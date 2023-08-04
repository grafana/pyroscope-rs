use ctrlc;
use duct::cmd;
use std::os::unix::process::CommandExt;

use std::sync::mpsc::channel;
use std::thread;

use super::{profiler::Profiler};
use crate::utils::{
    app_config::AppConfig,
    error::{Error, Result},
    types::Spy,
};

/// exec command
pub fn exec() -> Result<()> {
    // TODO: this processing should probably be done along with the config parsing
    set_application_name()?;
    set_tags()?;

    let command = AppConfig::get::<Option<String>>("command")?
        .ok_or_else(|| Error::new("command unwrap failed"))?;

    let command_args = AppConfig::get::<Option<String>>("command_args")?
        .ok_or_else(|| Error::new("command unwrap failed"))?;
    let command_args: Vec<String> = pyroscope::pyroscope::parse_vec_string_json(command_args)?;

    let uid = AppConfig::get::<Option<u32>>("user_name").unwrap_or(None);
    let gid = AppConfig::get::<Option<u32>>("group_name").unwrap_or(None);

    let handle = cmd(command, command_args)
        .before_spawn(move |c| {
            if let Some(uid) = uid {
                c.uid(uid);
            }
            if let Some(gid) = gid {
                c.gid(gid);
            }
            Ok(())
        })
        .start()?;

    let pid = handle
        .pids()
        .get(0)
        .ok_or_else(|| Error::new("pid not collected"))?
        .to_owned() as i32 ;

    AppConfig::set("pid", pid.to_string().as_str())?;

    let mut profiler = Profiler::default();
    profiler.init()?;

    let child_res = handle.wait();
    let profiler_res = profiler.stop();
    child_res?;
    profiler_res?;

    Ok(())
}

/// connect command
pub fn connect() -> Result<()> {
    // TODO: this processing should probably be done along with the config parsing
    set_application_name()?;
    set_tags()?;

    // Initialize profiler
    let mut profiler = Profiler::default();
    profiler.init()?;

    // Create a channel for ctrlc
    let (tx, rx) = channel();

    // Set ctrcl handler
    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press Ctrl-C to exit.");

    // Wait for Ctrl+C signal
    rx.recv().unwrap();

    println!("Exiting.");

    // Stop exector and profiler
    profiler.stop()?;

    Ok(())
}

//
// TODO: These functions should be placed somewhere else
//
fn set_application_name() -> Result<()> {
    let pre_app_name: String = AppConfig::get::<String>("application_name").unwrap_or_else(|_| {
        log::info!("we recommend specifying application name via -application-name flag or env variable PYROSCOPE_APPLICATION_NAME");
        names::Generator::default()
            .next()
            .unwrap_or_else(|| "unassigned.app".to_string())
            .replace('-', ".")
    });

    let pre = match AppConfig::get::<Spy>("spy_name")? {
        Spy::Pyspy => "pyspy",
        Spy::Rbspy => "rbspy",
    };

    // add pre to pre_app_name
    let app_name = format!("{}.{}", pre, pre_app_name);

    log::info!(
        "Profiling with {} profiler with application name: {}",
        pre,
        app_name
    );

    AppConfig::set("application_name", app_name.as_str())?;

    Ok(())
}

fn set_tags() -> Result<()> {
    let tag: String = AppConfig::get::<String>("tag").unwrap_or_else(|_| "".to_string());

    AppConfig::set("tag", tag.as_str())?;

    Ok(())
}
