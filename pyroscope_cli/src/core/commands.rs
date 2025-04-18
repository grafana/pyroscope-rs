use ctrlc;
use duct::cmd;
use std::os::unix::process::CommandExt;

use super::profiler::Profiler;
use crate::cli::{CommandArgs};
use crate::utils::error::{Error, Result};
use std::sync::mpsc::channel;

pub fn exec(command : Vec<String>, common: CommandArgs) -> Result<()> {
    if command.is_empty() {
        return Err(Error::new("command is empty"));
    }
    let exe = &command[0];
    let args = &command[1..];

    let uid = common.user_name.clone();
    let gid = common.group_name.clone();
    let handle = cmd(exe, args)
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
        .to_owned() as i32;

    let mut profiler = Profiler::default();
    profiler.init(pid, common)?;

    let child_res = handle.wait();

    //todo this waits for up to 10 seconds, fix this
    // todo this waits forever if the server is down?
    let profiler_res = profiler.stop();
    child_res?;
    profiler_res?;

    Ok(())
}

pub fn connect(pid :i32, common: CommandArgs) -> Result<()> {
    let mut profiler = Profiler::default();
    profiler.init(pid, common)?;

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
