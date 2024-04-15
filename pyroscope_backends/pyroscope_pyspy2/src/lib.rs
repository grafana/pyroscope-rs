#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;


use py_spy::{config::Config, sampler::Sampler, python_process_info::PythonProcessInfo};
use pyroscope::{
    backend::{
        Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, Rule, Ruleset,
        StackBuffer, StackFrame, StackTrace,
    },
    error::{PyroscopeError, Result},
};
use proc_maps::{get_process_maps, MapRange, Pid};
use std::{mem, ops::Deref, sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
}, thread::JoinHandle};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::io::Error;
use libc::itimerval;

use log::info;
use signal_hook::consts::SIGPROF;
use py_spy::python_process_info::{get_interpreter_address, get_python_version, get_threadstate_address};

const LOG_TAG: &str = "Pyroscope::Pyspy2";

pub fn pyspy2_backend(config: Pyspy2Config) -> BackendImpl<BackendUninitialized> {
    // Clone BackendConfig to pass to the backend object.
    let backend_config = config.backend_config;

    BackendImpl::new(Box::new(Pyspy2::new(config)), Some(backend_config))
}

#[derive(Debug, Clone)]
pub struct Pyspy2Config {
    sample_rate: u32,
    backend_config: BackendConfig,

}

impl Default for Pyspy2Config {
    fn default() -> Self {
        Pyspy2Config {
            sample_rate: 100,
            backend_config: BackendConfig::default(),
        }
    }
}

impl Pyspy2Config {
    /// Create a new Pyspy2Config
    pub fn new() -> Self {
        Pyspy2Config {
            // pid: Some(pid),
            ..Default::default()
        }
    }

    pub fn sample_rate(self, sample_rate: u32) -> Self {
        Pyspy2Config {
            sample_rate,
            ..self
        }
    }
}

#[derive(Default)]
pub struct Pyspy2 {
    config: Pyspy2Config,
}

impl std::fmt::Debug for Pyspy2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pyspy2 Backend")
    }
}

impl Pyspy2 {
    pub fn new(config: Pyspy2Config) -> Self {
        Pyspy2 {
            config,
        }
    }
}

impl Backend for Pyspy2 {
    fn spy_name(&self) -> Result<String> {
        Ok("pyspy2".to_string())
    }

    fn spy_extension(&self) -> Result<Option<String>> {
        Ok(Some("cpu".to_string()))
    }

    fn sample_rate(&self) -> Result<u32> {
        Ok(self.config.sample_rate)
    }

    fn set_config(&self, _config: BackendConfig) {}

    fn add_rule(&self, rule: Rule) -> Result<()> {
        Ok(())
    }

    fn remove_rule(&self, rule: Rule) -> Result<()> {
        Ok(())
    }

    fn initialize(&mut self) -> Result<()> {
        println!("pyspy2 init");

        let pid: Pid = std::process::id() as i32;

        let p = remoteprocess::Process::new(pid)
            .map_err(|_| PyroscopeError::new("Pyspy2: remoteprocess::Process::new"))?;

        let pi = PythonProcessInfo::new(&p)
            .map_err(|_| PyroscopeError::new("Pyspy2: PythonProcessInfo::new"))?;

        let version = get_python_version(&pi, &p)
            .map_err(|_| PyroscopeError::new("Pyspy2: get_python_version"))?;
        info!("python version {} detected", version);

        let interpreter_address = get_interpreter_address(&pi, &p, &version)
            .map_err(|_| PyroscopeError::new("Pyspy2: get_interpreter_address"))?;
        info!("Found interpreter at 0x{:016x}", interpreter_address);

        let mut config: Config = Config::default();
        config.gil_only = true;
        let threadstate_address = get_threadstate_address(&pi, &version, &config)
            .map_err(|_| PyroscopeError::new("Pyspy2: get_interpreter_address"))?;
        info!("threadstate_address {:016x}", threadstate_address);
        let version_string = format!("python{}.{}", version.major, version.minor);
        info!("python version {:?}", version_string);

        new_signal_handler(SIGPROF).expect("failed to set up signal handler"); //todo

        start_timer().expect("failed to start timer"); //todo

        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        Ok(vec![])
    }
}

fn new_signal_handler(signal: libc::c_int) -> std::result::Result<(), Error> {
    let mut new: libc::sigaction = unsafe { mem::zeroed() };
    new.sa_sigaction = handler as usize;
    new.sa_flags = libc::SA_RESTART | libc::SA_SIGINFO;
    let mut old: libc::sigaction = unsafe { mem::zeroed() };
    if unsafe { libc::sigaction(signal, &new, &mut old) } != 0 {
        return Err(Error::last_os_error());
    }
    Ok(()) // todo keep prev for fallback
}

fn start_timer() -> std::result::Result<(), Error>  {
    let interval = 10000000; //
    let sec = interval / 1000000000;
    let usec = (interval % 1000000000) / 1000;
    let mut tv: libc::itimerval = unsafe { mem::zeroed() };
    tv.it_value.tv_sec = sec;
    tv.it_value.tv_usec = usec as libc::suseconds_t;
    tv.it_interval.tv_sec = sec;
    tv.it_interval.tv_usec = usec as libc::suseconds_t;
    if unsafe { libc::setitimer(libc::ITIMER_PROF, &tv, std::ptr::null_mut()) } != 0 {
        return Err(Error::last_os_error());
    }
    return Ok(());
}

#[cfg(not(windows))]
extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
    println!("handler")// this is not safe, only for debugging
}


