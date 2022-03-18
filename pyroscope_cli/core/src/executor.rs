use duct::cmd;
use utils::error::{Error, Result};

pub struct Executor<'a> {
    cmd: &'a str,
    args: &'a str,
    handle: Option<duct::Handle>,
}

impl<'a> Executor<'a> {
    pub fn new(cmd: &'a str, args: &'a str) -> Executor<'a> {
        Executor {
            cmd,
            args,
            handle: None,
        }
    }

    pub fn run(self) -> Result<Self> {
        let handle = cmd!(self.cmd, self.args).start()?;

        Ok(Self {
            cmd: self.cmd,
            args: self.args,
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
