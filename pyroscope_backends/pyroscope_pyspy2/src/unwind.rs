use std::ffi::c_void;
use log::debug;
use proc_maps::Pid;


//todo remove
use py_spy::python_process_info::{get_python_version, PythonProcessInfo};

use crate::{kindasafe, offsets, pystr};
use crate::kindasafe::{read_u64};
use crate::offsets::validate_python_offsets;

#[derive(Debug)]
pub enum PythonUnwinderError {
    NoThreadState,
    NoTopFrame,
    ReadError(kindasafe::Error),
}

impl From<kindasafe::Error> for PythonUnwinderError {
    fn from(value: kindasafe::Error) -> Self {
        return PythonUnwinderError::ReadError(value);
    }
}

#[derive(Debug)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}


#[derive(Debug)]
pub struct PythonUnwinder {
    pub offsets: offsets::Offsets,
    pub tss_key: libc::pthread_key_t,
    pub version: Version,
    pub py_runtime: usize,
}

impl PythonUnwinder {
    pub fn new() -> anyhow::Result<Self>{
        let pid: Pid = std::process::id() as i32;

        let p = remoteprocess::Process::new(pid)?;

        let pi = PythonProcessInfo::new(&p)?;

        let version: py_spy::version::Version = get_python_version(&pi, &p)?;

        let offsets = offsets::get_python_offsets(&version);
        validate_python_offsets(&version, &offsets)?;


        kindasafe::init()?;

        let py_runtime = get_py_runtime(&pi)? as usize;
        let tss_key = get_tss_key(py_runtime, &offsets)?;
        let res = Ok(Self {
            offsets,
            tss_key,
            py_runtime,
            version: Version {
                major: version.major,
                minor: version.minor,
                patch: version.patch,
            },
        });
        debug!("PythonUnwinder::new {:?}", res);
        return res;
    }


    pub fn read_python_stack(&mut self) -> std::result::Result<(), PythonUnwinderError> {
        let ts = self.get_thread_state();

        if (ts == 0) {
            return Err(PythonUnwinderError::NoThreadState);
        }

        let top_frame = read_u64(ts + self.offsets.PyThreadState_frame as usize)? as usize;
        if (top_frame == 0) {
            return Err(PythonUnwinderError::NoTopFrame);
        }

        unwind_println("==============");

        let mut count = 0;
        let mut frame = top_frame;
        while frame != 0 && count < 128 {
            let code = read_u64(frame + self.offsets.PyFrameObject_f_code as usize)? as usize;
            //todo owner check
            let back = read_u64(frame + self.offsets.PyFrameObject_f_back as usize).unwrap() as usize;
            let name_ptr: usize =
                if code != 0 {
                    read_u64(code + self.offsets.PyCodeObject_co_name as usize).unwrap() as usize
                } else {
                    0
                };

            let mut pyname = pystr::pystr { buf: [0; 256], len: 0 };
            let name = if name_ptr != 0 {
                if let Err(e) = pystr::read(name_ptr, &mut pyname) {
                    "ErrName"
                } else {
                    std::str::from_utf8(&pyname.buf[0..pyname.len]).unwrap()//todo
                }
            } else {
                "NullName"
            };



            // unwind_print_hex(frame);
            // unwind_print_hex(code);
            // unwind_print_hex(back);
            // unwind_print_hex(name_ptr);
            unwind_println(name);
            frame = back;
            count += 1;
        }
        return Ok(());
    }
    fn get_thread_state(&self) -> usize {
        unsafe {
            libc::pthread_getspecific(self.tss_key) as usize
        }
    }




}

fn get_py_runtime(pi: &PythonProcessInfo) -> anyhow::Result<u64> {
    let res = pi.get_symbol("_PyRuntime")
        .ok_or(anyhow!("no _PyRuntime found"))?;
    return Ok(*res)
}

fn get_tss_key(py_runtime :usize, o: &offsets::Offsets) -> anyhow::Result<u64> {

    let initialized: u32 = read_u64(py_runtime + o.PyRuntimeState_gilstate as usize + o.Gilstate_runtime_state_autoTSSkey as usize + o.PyTssT_is_initialized as usize)? as u32;
    let key = read_u64(py_runtime + o.PyRuntimeState_gilstate as usize + o.Gilstate_runtime_state_autoTSSkey as usize + o.PyTssT_key as usize)?;
    if initialized != 1 {
        bail!("tss key not initialized");
    }


    return Ok(key);
}




extern "C" fn thread_id() -> u64 {
    unsafe { libc::pthread_self() as u64 }
}


fn unwind_println(s: &str) {
    unsafe {
        libc::write(1, s.as_ptr() as *const c_void, s.len());
        libc::write(1, "\n".as_ptr() as *const c_void, 1);
    }
}

fn unwind_print(s: &str) {
    unsafe {
        libc::write(1, s.as_ptr() as *const c_void, s.len());
    }
}

fn unwind_print_hex(v : usize) {
    unwind_print(" ");
    let mut buf = [0u8; 16];
    let mut i = 0;
    let mut v = v;
    while v > 0 {
        let c = (v & 0xf) as u8;
        buf[i] = if c < 10 {
            c + '0' as u8
        } else {
            c - 10 + 'a' as u8
        };
        v >>= 4;
        i += 1;
    }
    if i == 0 {
        buf[i] = '0' as u8;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        unsafe {
            libc::write(1, &buf[i] as *const u8 as *const c_void, 1);
        }
    }
    unwind_print(" ");
}
