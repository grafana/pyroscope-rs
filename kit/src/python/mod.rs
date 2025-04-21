use object::ObjectSymbol;
use memmap2::MmapOptions;
use object::{Object, Symbol};
use procmaps::{MapRange, get_process_maps};
use sigsafe::Ptr;
use std::fs::OpenOptions;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Mem(kindasafe::ReadMemError),
    Object(object::Error),
    PythonNotFound,
    PythonSymbolNotFound,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<object::Error> for Error {
    fn from(value: object::Error) -> Self {
        Self::Object(value)
    }
}

impl From<kindasafe::ReadMemError> for Error {
    fn from(value: kindasafe::ReadMemError) -> Self {
        Self::Mem(value)
    }
}


fn get_python_path(ms: &Vec<MapRange>) -> Result<MappingInfo, Error> {

    let mut bin : Option<MappingInfo> = None;
    let mut lib : Option<MappingInfo> = None;
    for x in ms {
        match is_python_mapping(x) {
            None => {}
            Some(info) => {
                match info.typ {
                    MappingType::Binary => {
                        if bin.is_none() { //todo how to make it nicer
                            bin = Some(info)
                        }
                    }
                    MappingType::Library => {
                        if lib.is_none() {
                            lib = Some(info)
                        }
                        // lib.push(info)
                    }
                }
            }
        }
    }
    if let Some(lib) = lib {
        return Ok(lib)
    }
    if let Some(bin) = bin {
        return Ok(bin)
    }
    Err(Error::PythonNotFound)
}

pub fn load() -> std::result::Result<(), Error> {
    let ms = get_process_maps()?; // maybe pass it?

    let python = get_python_path(&ms)?;

    let f = OpenOptions::new().read(true).open(python.path)?;

    unsafe {
        let m = MmapOptions::new().map(&f)?;
        let elf = object::File::parse(m.as_ref())?;

        //todo does this use gnu hash table?
        let py_runtime = elf.symbol_by_name("_PyRuntime")
            .ok_or_else(|| Error::PythonSymbolNotFound)?;
        let auto_tss_key = get_auto_tss(&python, &py_runtime)?;
        println!("tss {:?}", auto_tss_key);
    };
    Ok(())
}

#[derive(PartialEq, Clone, Debug)]
enum MappingType {
    Binary,
    Library,
}

#[derive(Debug)]
struct MappingInfo<'a> {
    typ : MappingType,
    path: &'a  PathBuf,
    range: &'a MapRange,
}

fn is_python_mapping(x: &MapRange) -> Option<MappingInfo> {
    const LIB_PYTHON_PREFIX: &'static str = "libpython3";
    const PYTHON_PREFIX: &'static str = "python3";
    match x.filename() {
        None => None,
        Some((path, filename)) => {
            if filename.contains(LIB_PYTHON_PREFIX) {
                Some(MappingInfo{
                    typ: MappingType::Library,
                    path,
                    range: x,
                })
            } else if filename.contains(PYTHON_PREFIX) {
                Some(MappingInfo{
                    typ: MappingType::Binary,
                    path,
                    range: x,
                })
            } else {
                None
            }
        }
    }
}

fn get_auto_tss(m: &MappingInfo, py_runtime: &Symbol) -> std::result::Result<u32, kindasafe::ReadMemError> {
    // todo extract this value from disasm?
    // (gdb) print &_PyRuntime.autoTSSkey
    // $4 = (Py_tss_t *) 0x7f0b6a130d10 <_PyRuntime+2160>
    //     (gdb) print &_PyRuntime
    // $5 = (_PyRuntimeState *) 0x7f0b6a1304a0 <_PyRuntime>
    let tss = kindasafe::u64((m.range.range_start as u64 + py_runtime.address() + 2160) as Ptr)?; //todo checked add
    let initialized = tss & 0xffffffff;
    assert_eq!(initialized, 1);
    let key = tss >> 32;
    Ok(key as u32)
}
