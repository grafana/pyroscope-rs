use super::error::{BackendError, Result};
use super::types::{Backend, State};

use rbspy::{sampler::Sampler, OutputFormat, StackTrace};

use std::io::Write;

// TODO: handle errors returned from rx2
// TODO: sync_channel size
// TODO: handle unwraps

/// Rbspy Configuration
#[derive(Debug)]
pub struct RbspyConfig {
    pid: i32,
    sample_rate: u32,
    lock_process: bool,
    time_limit: Option<core::time::Duration>,
    with_subprocesses: bool,
}

impl Default for RbspyConfig {
    fn default() -> Self {
        RbspyConfig {
            pid: 0,
            sample_rate: 100,
            lock_process: false,
            time_limit: None,
            with_subprocesses: false,
        }
    }
}

impl RbspyConfig {
    /// Create a new RbspyConfig
    pub fn new(
        pid: i32, sample_rate: u32, lock_process: bool, time_limit: Option<core::time::Duration>,
        with_subprocesses: bool,
    ) -> Self {
        RbspyConfig {
            pid,
            sample_rate,
            lock_process,
            time_limit,
            with_subprocesses,
        }
    }
}

/// Rbspy Backend
#[derive(Default)]
pub struct Rbspy {
    sampler: Option<Sampler>,
    rx: Option<std::sync::mpsc::Receiver<StackTrace>>,
    rx2: Option<std::sync::mpsc::Receiver<std::result::Result<(), anyhow::Error>>>,
    state: State,

    config: RbspyConfig,
}

impl std::fmt::Debug for Rbspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rbspy Backend")
    }
}

impl Rbspy {
    pub fn new(config: RbspyConfig) -> Self {
        Rbspy {
            sampler: None,
            rx: None,
            rx2: None,
            state: State::Uninitialized,
            config,
        }
    }
}

impl Backend for Rbspy {
    fn get_state(&self) -> State {
        self.state
    }

    fn spy_name(&self) -> Result<String> {
        Ok("rbspy".to_string())
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn initialize(&mut self) -> Result<()> {
        // Check if Backend is Uninitialized
        if self.state != State::Uninitialized {
            return Err(BackendError::new("Rbspy Backend is already Initialized"));
        }

        // Create Sampler
        self.sampler = Some(Sampler::new(
            self.config.pid,
            self.config.sample_rate,
            self.config.lock_process,
            self.config.time_limit,
            self.config.with_subprocesses,
        ));

        // Set State to Ready
        self.state = State::Ready;

        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        // Check if Backend is Ready
        if self.state != State::Ready {
            return Err(BackendError::new("Rbspy Backend is not Ready"));
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let (synctx, syncrx) = std::sync::mpsc::sync_channel(1000);
        self.rx = Some(syncrx);
        self.rx2 = Some(rx);
        let a = self
            .sampler
            .as_mut()
            .unwrap_or_else(|| panic!("sampler is none"));

        println!("am here");

        let b = a.start(synctx, tx);

        match b {
            Ok(_) => println!("Worked"),
            Err(e) => {
                dbg!(e);
            }
        }

        // Set State to Running
        self.state = State::Running;

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Rbspy Backend is not Running"));
        }

        // Stop Sampler
        self.sampler.as_mut().unwrap().stop();

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        // Check if Backend is Running
        if self.state != State::Running {
            return Err(BackendError::new("Rbspy Backend is not Running"));
        }

        let col = self.rx.as_ref().unwrap().try_iter();

        let mut outputter = OutputFormat::collapsed.outputter(0.1);

        for trace in col {
            outputter.record(&trace)?;
        }

        // Create a new writer
        let mut buffer: Vec<u8> = Vec::new();
        let mut writer = RbspyWriter::new(&mut buffer);

        // Push the outputter into the writer
        outputter.complete(&mut writer)?;

        // Flush the Writer
        writer.flush()?;

        // Return the writer's buffer
        Ok(buffer)
    }
}

/// Rubyspy Writer
/// This object is used to write the output of the rbspy sampler to a data buffer
struct RbspyWriter<'a> {
    data: Vec<u8>,
    buffer: &'a mut Vec<u8>,
}

impl<'a> RbspyWriter<'a> {
    /// Create a new RbspyWriter
    pub fn new(buffer: &'a mut Vec<u8>) -> Self {
        RbspyWriter {
            data: Vec::new(),
            buffer,
        }
    }
}

/// Implement Writer for Rbspy
impl<'a> std::io::Write for RbspyWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // push the data to the buffer
        self.data.extend_from_slice(buf);

        // return the number of bytes written
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // flush the buffer
        self.buffer.extend_from_slice(&self.data);

        Ok(())
    }
}

#[cfg(test)]
mod test_rbspy_writer {
    use super::RbspyWriter;
    use std::io::Write;

    #[test]
    fn test_rbspy_writer() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut writer = RbspyWriter::new(&mut buffer);

        writer.write(b"hello").unwrap();
        writer.write(b"world").unwrap();
        writer.flush().unwrap();

        assert_eq!(buffer, b"helloworld".to_vec());
    }
}
