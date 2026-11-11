use std::ops::Range;
use std::path::{Path, PathBuf};

use addr2line::Loader;
use once_cell::sync::Lazy;
use spin::Mutex;

use super::shlib;
use crate::backend::pprofrs::frames::Symbol;

/// Symbolizes addresses of the running executable from an image opened at
/// startup, so unlinking or replacing the file later does not lose symbols.
struct Symbolizer {
    avma: Range<usize>,
    bias: usize,
    loader: Loader,
}

static SYMBOLIZER: Lazy<Mutex<Option<Symbolizer>>> = Lazy::new(|| {
    Mutex::new(shlib::executable().and_then(|executable| {
        Symbolizer::open(&executable.path, executable.avma, executable.bias)
    }))
});

pub fn init() {
    Lazy::force(&SYMBOLIZER);
}

/// Returns false when `ip` is outside the executable or its image is not loaded.
pub fn resolve<F: FnMut(&Symbol)>(ip: usize, cb: F) -> bool {
    match SYMBOLIZER.lock().as_ref() {
        Some(symbolizer) if symbolizer.avma.contains(&ip) => {
            symbolizer.resolve(ip, cb);
            true
        }
        _ => false,
    }
}

impl Symbolizer {
    fn open(path: &Path, avma: Range<usize>, bias: usize) -> Option<Self> {
        match Loader::new(path) {
            Ok(loader) => Some(Symbolizer { avma, bias, loader }),
            Err(err) => {
                log::warn!("cannot load symbols from {}: {err}", path.display());
                None
            }
        }
    }

    fn resolve<F: FnMut(&Symbol)>(&self, ip: usize, mut cb: F) {
        // Same as backtrace: look one byte back so return addresses stay in the call site.
        let addr = (ip - 1).wrapping_sub(self.bias) as u64;
        let mut found = false;
        if let Ok(mut frames) = self.loader.find_frames(addr) {
            while let Ok(Some(frame)) = frames.next() {
                found = true;
                let name = match &frame.function {
                    Some(function) => function
                        .raw_name()
                        .ok()
                        .map(|name| name.into_owned().into_bytes()),
                    None => self.symtab_name(addr),
                };
                let location = frame.location.as_ref();
                cb(&Symbol {
                    name,
                    lineno: location.and_then(|location| location.line),
                    filename: location
                        .and_then(|location| location.file)
                        .map(PathBuf::from),
                });
            }
        }
        if !found {
            if let Some(name) = self.symtab_name(addr) {
                cb(&Symbol {
                    name: Some(name),
                    lineno: None,
                    filename: None,
                });
            }
        }
    }

    fn symtab_name(&self, addr: u64) -> Option<Vec<u8>> {
        self.loader
            .find_symbol(addr)
            .map(|name| name.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline(never)]
    fn probe() {}

    #[test]
    fn resolves_through_an_image_opened_before_unlink() {
        let executable = shlib::executable().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join("executable");
        std::fs::copy(std::env::current_exe().unwrap(), &copy).unwrap();
        let symbolizer = Symbolizer::open(&copy, executable.avma, executable.bias).unwrap();
        std::fs::remove_file(&copy).unwrap();

        let ip = (probe as fn()) as usize + 1;
        let mut names = Vec::new();
        symbolizer.resolve(ip, |symbol| names.push(symbol.name()));
        assert!(
            names.iter().any(|name| name.ends_with("::tests::probe")),
            "{names:?}"
        );
    }
}
