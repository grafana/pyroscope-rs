use std::ffi::OsStr;
use std::ops::Range;
use std::path::{Path, PathBuf};

use findshlibs::{IterationControl, SharedLibrary, TargetSharedLibrary};
use framehop::{Module, ModuleSectionInfo};
use memmap2::Mmap;
use object::{Object, ObjectSection};
use once_cell::sync::Lazy;

static OBJECTS: Lazy<Vec<Module<Vec<u8>>>> = Lazy::new(find_objects);

pub fn get_objects() -> &'static [Module<Vec<u8>>] {
    &OBJECTS
}

/// Where the running executable is mapped and a path that still opens it after
/// the original file was unlinked or replaced.
pub struct Executable {
    pub path: PathBuf,
    pub avma: Range<usize>,
    pub bias: usize,
}

pub fn executable() -> Option<Executable> {
    let mut executable = None;
    TargetSharedLibrary::each(|shlib| {
        let base = shlib.actual_load_addr().0;
        executable = Some(Executable {
            path: executable_path(shlib.name()),
            avma: base..base + shlib.len(),
            bias: shlib.virtual_memory_bias().0,
        });
        IterationControl::Break
    });
    executable
}

fn executable_path(name: &OsStr) -> PathBuf {
    let proc_exe = Path::new("/proc/self/exe");
    if cfg!(target_os = "linux") && proc_exe.exists() {
        proc_exe.to_path_buf()
    } else {
        PathBuf::from(name)
    }
}

pub struct ObjectInfo<'f>(object::File<'f>);

impl<'file, D> ModuleSectionInfo<D> for ObjectInfo<'file>
where
    D: From<&'file [u8]>,
{
    fn base_svma(&self) -> u64 {
        if let Some(section) = self.0.section_by_name("__TEXT") {
            // in mach-o addresses are relative to __TEXT
            section.address()
        } else {
            self.0.relative_address_base()
        }
    }

    fn section_svma_range(&mut self, name: &[u8]) -> Option<std::ops::Range<u64>> {
        if let Some(section) = self.0.section_by_name_bytes(name) {
            let start = section.address();
            let end = start + section.size();
            Some(start..end)
        } else {
            None
        }
    }

    fn section_data(&mut self, name: &[u8]) -> Option<D> {
        if let Some(section) = self.0.section_by_name_bytes(name) {
            let data = section.data().ok()?;
            Some(D::from(data))
        } else {
            None
        }
    }
}

fn open_mmap(path: &Path) -> Option<Mmap> {
    let file = std::fs::File::open(path).ok()?;
    let mmap = unsafe { Mmap::map(&file) }.ok()?;
    Some(mmap)
}

fn find_objects() -> Vec<Module<Vec<u8>>> {
    let mut objects = Vec::new();
    let mut is_executable = true;
    // objects
    TargetSharedLibrary::each(|shlib| {
        let path = PathBuf::from(shlib.name());
        let base_avma = shlib.actual_load_addr().0 as u64;
        let avma_range = base_avma..base_avma + shlib.len() as u64;
        let mmap = if is_executable {
            open_mmap(&executable_path(shlib.name()))
        } else {
            open_mmap(&path)
        };
        is_executable = false;
        if let Some(mmap) = mmap {
            if let Ok(obj) = object::File::parse(&*mmap) {
                let section_info = ObjectInfo(obj);

                objects.push(Module::new(
                    path.to_string_lossy().to_string(),
                    avma_range,
                    base_avma,
                    section_info,
                ));
            }
        }
    });

    objects
}
