mod kindasafe;
mod signalhandlers;
pub mod offsets;

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
use std::arch::asm;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::io::Error;
use libc::{exit, itimerval, SIGBUS, SIGSEGV};

use log::info;
use remoteprocess::ProcessMemory;
use signal_hook::consts::SIGPROF;
use py_spy::python_process_info::{get_interpreter_address, get_python_version, get_threadstate_address};
use crate::kindasafe::read_u64;
use crate::offsets::validate_python_offsets;

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

    fn initialize(&mut self) -> Result<()> { //todo anyhow
        println!("pyspy2 init");

        let pid: Pid = std::process::id() as i32;

        let p = remoteprocess::Process::new(pid)
            .map_err(|_| PyroscopeError::new("Pyspy2: remoteprocess::Process::new"))?;

        let pi = PythonProcessInfo::new(&p)
            .map_err(|_| PyroscopeError::new("Pyspy2: PythonProcessInfo::new"))?;

        let version: py_spy::version::Version = get_python_version(&pi, &p)
            .map_err(|_| PyroscopeError::new("Pyspy2: get_python_version"))?;
        info!("python version {} detected", version);

        let version_string = format!("python{}.{}", version.major, version.minor);
        info!("python version {:?}", version_string);

        unsafe {
            offsets = offsets::get_python_offsets(&version);
            validate_python_offsets(&version, &offsets)
                .map_err(|_| PyroscopeError::new("faild to validate offsets"))?;
        }





        kindasafe::init()
            .map_err(|_| PyroscopeError::new("Pyspy2: kindasafe::init"))?;

        unsafe  {
            tss_key = get_tss_key(&pi, &offsets)?;
            info!("tssKey {:016x}", tss_key);
        }


        signalhandlers::new_signal_handler(SIGPROF, handler as usize).expect("failed to set up signal handler"); //todo
        // let interval = 10000000;
        let interval = 100000000;
        signalhandlers::start_timer(interval).expect("failed to start timer"); //todo

        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        Ok(vec![])
    }
}



fn get_tss_key(pi: &PythonProcessInfo, o: &offsets::Offsets) -> Result<u64> {
    let _PyRuntime = pi.get_symbol("_PyRuntime")
        .ok_or(PyroscopeError::new("get_tss_key: _PyRuntime"))?;// todo
    let _PyRuntime: usize = *_PyRuntime as usize;
    info!("_PyRuntime {:016x}", _PyRuntime);
    let initialized: u32 = read_u64(_PyRuntime + o.PyRuntimeState_gilstate as usize + o.Gilstate_runtime_state_autoTSSkey as usize + o.PyTssT_is_initialized as usize) as u32;
    let key = read_u64(_PyRuntime + o.PyRuntimeState_gilstate as usize + o.Gilstate_runtime_state_autoTSSkey as usize + o.PyTssT_key as usize);
    if initialized != 1 {
        return Err(PyroscopeError::new("get_tss_key: not initialized"));
    }
    return Ok(key);
}

fn get_thread_state() -> usize {
    unsafe {
        libc::pthread_getspecific(tss_key) as usize
    }
}

static mut tss_key: libc::pthread_key_t = 0;

static mut offsets: offsets::Offsets = offsets::Offsets{
    PyVarObject_ob_size: 0,
    PyObject_ob_type: 0,
    PyTypeObject_tp_name: 0,
    PyThreadState_frame: 0,
    PyThreadState_cframe: 0,
    PyThreadState_current_frame: 0,
    PyCFrame_current_frame: 0,
    PyFrameObject_f_back: 0,
    PyFrameObject_f_code: 0,
    PyFrameObject_f_localsplus: 0,
    PyCodeObject_co_filename: 0,
    PyCodeObject_co_name: 0,
    PyCodeObject_co_varnames: 0,
    PyCodeObject_co_localsplusnames: 0,
    PyTupleObject_ob_item: 0,
    PyInterpreterFrame_f_code: 0,
    PyInterpreterFrame_f_executable: 0,
    PyInterpreterFrame_previous: 0,
    PyInterpreterFrame_localsplus: 0,
    PyInterpreterFrame_owner: 0,
    PyRuntimeState_gilstate: 0,
    PyRuntimeState_autoTSSkey: 0,
    Gilstate_runtime_state_autoTSSkey: 0,
    PyTssT_is_initialized: 0,
    PyTssT_key: 0,
    PyTssTSize: 0,
    PyASCIIObjectSize: 0,
    PyCompactUnicodeObjectSize: 0,
};





struct pystr {
    // buf array of 256
    buf: [u8; 256],
    len: usize
}

fn pystr_read(at: usize, s : &mut pystr) {
    let o_len = 0x10;
    let len = read_u64(at + 0x10) as usize;
    let state = read_u64(at + 0x20) as usize as u32;
    //todo check if it is ascii
    // println!("str len {:016x} {:08x}", len, state);
    let mut i = 0;

    while i < 255 && i < len { // todo
        s.buf[i] = read_u64(at + 0x30 +  i) as u8;
        i += 1;
    }
    s.buf[i] = 0;
    s.len = len
}

extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
    let ts = get_thread_state();
    let o = unsafe { &offsets };
    if (ts == 0) {
        return;
    }

    let top_frame = read_u64(ts + o.PyThreadState_frame as usize) as usize;
    if (top_frame == 0) {
        return;
    }

    println!("==============");

    let mut count = 0;
    let mut frame = top_frame;
    while frame != 0 && count < 128 {
        let code =  read_u64(frame + o.PyFrameObject_f_code as usize) as usize;
        //todo owner check
        let back =  read_u64(frame + o.PyFrameObject_f_back as usize) as usize;
        let name_ptr: usize =
        if code != 0 {
            read_u64(code + o.PyCodeObject_co_name as usize) as usize
        } else {
            0
        };
        let mut name = pystr { buf: [0; 256], len:0 };
        if name_ptr != 0 {
            pystr_read(name_ptr, &mut name);
        }
        // let name = std::str::from_utf8(&name.buf).unwrap();//todo
        let name = std::str::from_utf8(&name.buf[0..name.len]).unwrap();//todo




        println!("frame {:016x} code {:016x} back {:016x} name {:016x} {:?}", frame, code, back, name_ptr, name);

        frame = back;
        count += 1;
    }
}

extern "C" fn thread_id() -> u64 {
    unsafe { libc::pthread_self() as u64 }
}



