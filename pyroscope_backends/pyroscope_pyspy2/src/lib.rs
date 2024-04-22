mod kindasafe;
mod signalhandlers;
pub mod offsets;
mod unwind;
mod pystr;

// copy from py-spy
pub mod python_process_info;
// copy from py-spy
pub mod binary_parser;
pub mod version;
mod print;

use log::{debug, error};



#[macro_use]
extern crate anyhow;


use pyroscope::{
    backend::{
        Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, Rule, Ruleset,
        StackBuffer, StackFrame, StackTrace,
    },
    error::{PyroscopeError, Result},
};


use std::ffi::c_void;
use libc::SIGPROF;
use remoteprocess::ProcessMemory;
use crate::kindasafe::read_u64;
use crate::offsets::validate_python_offsets;
use crate::unwind::PythonUnwinder;
use crate::version::Version;

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


impl Pyspy2 {
    fn initialize2(&mut self) -> anyhow::Result<()> {
        let res = PythonUnwinder::new();
        debug!("PythonUnwinder::new {:?}", res);
        let res = res?;

        unsafe {
            unwinder = res;
        }


        signalhandlers::new_signal_handler(SIGPROF, handler as usize)?;
        // let interval = 10000000;
        let interval = 100000000;
        signalhandlers::start_timer(interval)?;

        Ok(())
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
        let result = self.initialize2();
        debug!("pyspy2 init {:?}", result);
        if let Err(e) = result {
            error!("failed to initialize pyspy2 backend: {:?}", e);
            return Err(PyroscopeError::new("pyspy2 init"));
        }
        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        Ok(vec![])
    }
}

static mut unwinder: unwind::PythonUnwinder = unwind::PythonUnwinder {
    offsets: offsets::Offsets {
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
    },
    tss_key: 0,
    version: Version {
        major: 0,
        minor: 0,
        patch: 0,
    },
    py_runtime: 0,
};


extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
    unsafe {
        let err = unwinder.read_python_stack();
        if let Err(e) = err {
            //todo log
        }
    };
}





