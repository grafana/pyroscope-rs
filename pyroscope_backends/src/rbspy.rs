use super::error::{BackendError, Result};
use super::types::{Backend, State};

use std::sync::{Arc, Mutex};

use rbspy::{
    recorder::{RecordConfig, Recorder},
    sampler::Sampler,
    OutputFormat, StackTrace,
};

#[derive(Debug)]
pub struct RbspyConfig {
    sample_rate: u32,
}

impl Default for RbspyConfig {
    fn default() -> Self {
        RbspyConfig { sample_rate: 100 }
    }
}

impl RbspyConfig {
    pub fn new(sample_rate: u32) -> Self {
        RbspyConfig { sample_rate }
    }
}

#[derive(Default)]
pub struct Rbspy {
    record_config: Option<RecordConfig>,
    recorder: Option<Arc<Recorder>>,
    sampler: Option<Sampler>,
    rx: Option<std::sync::mpsc::Receiver<StackTrace>>,
    rx2: Option<std::sync::mpsc::Receiver<std::result::Result<(), anyhow::Error>>>,
    state: State,
    pid: i32,

    config: RbspyConfig,
}

impl std::fmt::Debug for Rbspy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rbspy Backend")
    }
}

impl Rbspy {
    pub fn new(pid: i32) -> Self {
        Rbspy {
            record_config: None,
            recorder: None,
            sampler: None,
            rx: None,
            rx2: None,
            state: State::Uninitialized,
            pid,
            config: RbspyConfig::default(),
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
        //let config = RecordConfig {
        //format: OutputFormat::flamegraph,
        //raw_path: None,
        //out_path: None,
        //pid: 371084,
        //with_subprocesses: false,
        //sample_rate: 100,
        //maybe_duration: None,
        //flame_min_width: 10.0,
        //lock_process: true,
        //};

        // Set configuration for rbspy
        //self.record_config = Some(config);

        // Create recorder
        //self.recorder = Some(Arc::new(Recorder::new(config)));
        //println!("humhum");

        // TODO: To redo
        //dbg!(self.pid);
        self.sampler = Some(Sampler::new(self.pid, 100, false, None, false));

        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        //let recorder = Recorder::new(config);

        //match recorder.record() {
        //Ok(a) => dbg!(a),
        //Err(e) => println!("Failed to record: {:?}", e),
        //}

        //let record = self.recorder.as_mut().unwrap();
        //let recorder = Arc::clone(record);
        //std::thread::spawn(move || match recorder.record() {
        //Ok(a) => dbg!(a),
        //Err(e) => println!("Failed to record: {:?}", e),
        //});
        //self.recorder.as_ref().unwrap().record().unwrap();

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

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        //self.recorder.as_ref().unwrap().stop();

        self.sampler.as_mut().unwrap().stop();

        Ok(())
    }

    fn report(&mut self) -> Result<Vec<u8>> {
        //println!("report");
        //let mut writer = MyWriter {};

        //self.recorder
        //.as_ref()
        //.unwrap()
        //.write_summary(&mut writer)
        //.unwrap();

        //for i in 0..10005 {
        //let a = self.rx.as_ref().unwrap().recv().unwrap();
        //println!("{}", a);
        //println!("{}", self.sampler.as_ref().unwrap().total_traces());
        //}

        //println!("Done");
        //
        let col = self.rx.as_ref().unwrap().try_iter();

        //let a = self.rx2.as_ref().unwrap().recv().unwrap();
        //dbg!(a);
        println!("seems to be working");

        //println!("{:?}", &col.count());
        let mut outputter = OutputFormat::collapsed.outputter(0.1);

        for trace in col {
            outputter.record(&trace).unwrap();
        }
        let mut writer = MyWriter { data: Vec::new() };
        outputter.complete(&mut writer).unwrap();
        //col.for_each(|x| println!("{:#?}", x));
        //println!("{:?}", writer.data);

        Ok(writer.data)
    }
}

struct MyWriter {
    data: Vec<u8>,
}

impl std::io::Write for MyWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        //print!("{}", std::str::from_utf8(buf).unwrap());
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
