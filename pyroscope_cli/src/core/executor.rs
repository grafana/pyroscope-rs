use duct::cmd;
use std::os::unix::process::CommandExt;

use crate::utils::error::{Error, Result};

/// Run a command with the given arguments, uid and gid.
/// This is a wrapper around the duct crate.
pub struct Executor<'a> {
    /// The command to run
    cmd: &'a str,
    /// The arguments to pass to the command
    args: Vec<String>,
    /// The user id to run the command as
    uid: Option<u32>,
    /// The group id to run the command as
    gid: Option<u32>,
    /// duct thread handle
    handle: Option<duct::Handle>,
}

impl<'a> Executor<'a> {
    /// Create a new `Executor` with the given command, arguments, uid and gid.
    pub fn new(cmd: &'a str, args: Vec<String>, uid: Option<u32>, gid: Option<u32>) -> Executor<'a> {
        Executor {
            cmd,
            args,
            uid,
            gid,
            handle: None,
        }
    }

    /// Run the command.
    pub fn run(self) -> Result<Self> {
        let handle = cmd(self.cmd, &self.args)
            .before_spawn(move |cmd| {
                if let Some(uid) = self.uid {
                    cmd.uid(uid);
                }
                if let Some(gid) = self.gid {
                    cmd.gid(gid);
                }
                Ok(())
            })
            // .stdout_capture()
            .start()?;

        Ok(Self {
            cmd: self.cmd,
            args: self.args,
            uid: self.uid,
            gid: self.gid,
            handle: Some(handle),
        })
    }

    /// Stop the command.
    pub fn stop(self) -> Result<()> {
        self.handle
            .ok_or_else(|| Error::new("handle is not initialized"))?
            .kill()?;

        Ok(())
    }

    /// Get the process id of the executed command.
    pub fn get_pid(&self) -> Result<i32> {
        let pid = self
            .handle
            .as_ref()
            .ok_or_else(|| Error::new("handle is not initialized"))?
            .pids()
            .get(0)
            .ok_or_else(|| Error::new("pid not collected"))?
            .to_owned() as i32;

        Ok(pid)
    }
}
