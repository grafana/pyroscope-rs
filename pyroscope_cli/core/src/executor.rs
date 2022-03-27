use duct::cmd;
use std::os::unix::process::CommandExt;

use utils::error::{Error, Result};

pub struct Executor<'a> {
    cmd: &'a str,
    args: &'a str,
    uid: Option<u32>,
    gid: Option<u32>,
    handle: Option<duct::Handle>,
}

impl<'a> Executor<'a> {
    pub fn new(cmd: &'a str, args: &'a str, uid: Option<u32>, gid: Option<u32>) -> Executor<'a> {
        Executor {
            cmd,
            args,
            uid,
            gid,
            handle: None,
        }
    }

    pub fn run(self) -> Result<Self> {
        let handle = cmd!(self.cmd, self.args)
            .before_spawn(move |cmd| {
                if let Some(uid) = self.uid {
                    cmd.uid(uid);
                }
                if let Some(gid) = self.gid {
                    cmd.gid(gid);
                }
                Ok(())
            })
            .stdout_capture()
            .start()?;

        Ok(Self {
            cmd: self.cmd,
            args: self.args,
            uid: self.uid,
            gid: self.gid,
            handle: Some(handle),
        })
    }

    pub fn stop(self) -> Result<()> {
        self.handle
            .ok_or_else(|| Error::new("handle is not initialized"))?
            .kill()?;

        Ok(())
    }

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
