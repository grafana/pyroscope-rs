use procmaps::{get_process_maps, MapRange};
use memmap2::MmapOptions;
use object::Object;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

fn get_python_path(ms: &Vec<MapRange>) -> std::io::Result<&PathBuf> {
    let pythons: Vec<(PythonMappingType, &PathBuf)> = ms
        .iter()
        .filter(|x| x.executable())
        .filter_map(|x| is_python_mapping(x))
        .collect();
    let mut bin: Option<&PathBuf> = None;
    for x in pythons {
        match x {
            (PythonMappingType::Binary, path) => bin = Some(path),
            (PythonMappingType::Library, path) => return Ok(path),
        }
    }
    if let Some(bin) = bin {
        return Ok(bin);
    }
    Err(Error::new(ErrorKind::Other, "not found"))
}

pub fn load() -> std::io::Result<()> {
    let ms = get_process_maps()?; // maybe pass it?

    let python = get_python_path(&ms)?;
    println!("python {python:?}");

    let f = OpenOptions::new().read(true).open(python)?;

    unsafe {
        let m = MmapOptions::new().map(&f)?;
        let elf = object::File::parse(m.as_ref());
        let elf = if let Ok(elf) = elf {
            elf
        } else {
            //todo error type
            return Err(Error::new(ErrorKind::Other, "todo"));
        };
        //todo does this use gnu hash table?
        let py_runtime = elf.symbol_by_name("_PyRuntime");

        println!("_PyRuntime {py_runtime:?}")
    };
    Ok(())
}

#[derive(PartialEq, Clone)]
enum PythonMappingType {
    Binary,
    Library,
}

fn is_python_mapping(x: &MapRange) -> Option<(PythonMappingType, &PathBuf)> {
    const LIB_PYTHON_PREFIX: &'static str = "libPython3";
    const PYTHON_PREFIX: &'static str = "python3";
    match x.filename() {
        None => None,
        Some((path, filename)) => {
            if filename.contains(LIB_PYTHON_PREFIX) {
                Some((PythonMappingType::Library, path))
            } else if filename.contains(PYTHON_PREFIX) {
                Some((PythonMappingType::Binary, path))
            } else {
                None
            }
        }
    }
}
