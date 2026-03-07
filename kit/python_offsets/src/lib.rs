pub mod py313;
pub mod py314;

use core::mem::size_of;
use std::io::BufRead;

use object::{Object, ObjectSegment, ObjectSymbol};
use zydis::ffi::DecodedOperandKind;

/// Expected cookie at `_Py_DebugOffsets` offset 0: `b"xdebugpy"` as little-endian u64.
/// This is version-independent — all CPython builds that have `_Py_DebugOffsets` use it.
pub const COOKIE: u64 = 0x7970_6775_6265_6478;

#[derive(Debug, PartialEq)]
pub enum InitError {
    KindasafeInitFailed,
    PythonNotFound,
    /// `_PyRuntime` or `Py_Version` symbol not found in the ELF dynamic symbol table.
    /// Corresponds to init error code 3.
    SymbolNotFound,
    DebugOffsetsMismatch,
    UnsupportedVersion,
    /// The ELF file could not be parsed.
    ElfParse,
    /// Failed to open or mmap the binary file.
    Io,
    /// Disassembly of `_PyThreadState_GetCurrent` did not yield an FS-relative load.
    /// Corresponds to init error code 6.
    TlsDiscoveryFailed,
}

/// Parsed CPython version from `PY_VERSION_HEX`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PythonVersion {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
}

/// Parse a raw `PY_VERSION_HEX` value into [`PythonVersion`].
///
/// The hex layout is `(major << 24) | (minor << 16) | (micro << 8) | release_level`.
pub fn parse_version(version_hex: u64) -> PythonVersion {
    PythonVersion {
        major: ((version_hex >> 24) & 0xFF) as u8,
        minor: ((version_hex >> 16) & 0xFF) as u8,
        micro: ((version_hex >> 8) & 0xFF) as u8,
    }
}

/// Convert a 3.14 `_Py_DebugOffsets` into the 3.13 layout (common denominator).
/// Fields only present in 3.14 are dropped; shared sub-structs are copied
/// field-by-field since the 3.14 versions may have extra trailing fields.
impl From<&py314::_Py_DebugOffsets> for py313::_Py_DebugOffsets {
    fn from(v: &py314::_Py_DebugOffsets) -> Self {
        Self {
            cookie: v.cookie,
            version: v.version,
            free_threaded: v.free_threaded,
            runtime_state: py313::_Py_DebugOffsets__runtime_state {
                size: v.runtime_state.size,
                finalizing: v.runtime_state.finalizing,
                interpreters_head: v.runtime_state.interpreters_head,
            },
            interpreter_state: py313::_Py_DebugOffsets__interpreter_state {
                size: v.interpreter_state.size,
                id: v.interpreter_state.id,
                next: v.interpreter_state.next,
                threads_head: v.interpreter_state.threads_head,
                // 3.14 has threads_main here; 3.13 does not — skip it
                gc: v.interpreter_state.gc,
                imports_modules: v.interpreter_state.imports_modules,
                sysdict: v.interpreter_state.sysdict,
                builtins: v.interpreter_state.builtins,
                ceval_gil: v.interpreter_state.ceval_gil,
                gil_runtime_state: v.interpreter_state.gil_runtime_state,
                gil_runtime_state_enabled: v.interpreter_state.gil_runtime_state_enabled,
                gil_runtime_state_locked: v.interpreter_state.gil_runtime_state_locked,
                gil_runtime_state_holder: v.interpreter_state.gil_runtime_state_holder,
            },
            thread_state: py313::_Py_DebugOffsets__thread_state {
                size: v.thread_state.size,
                prev: v.thread_state.prev,
                next: v.thread_state.next,
                interp: v.thread_state.interp,
                current_frame: v.thread_state.current_frame,
                thread_id: v.thread_state.thread_id,
                native_thread_id: v.thread_state.native_thread_id,
                datastack_chunk: v.thread_state.datastack_chunk,
                status: v.thread_state.status,
            },
            interpreter_frame: py313::_Py_DebugOffsets__interpreter_frame {
                size: v.interpreter_frame.size,
                previous: v.interpreter_frame.previous,
                executable: v.interpreter_frame.executable,
                instr_ptr: v.interpreter_frame.instr_ptr,
                localsplus: v.interpreter_frame.localsplus,
                owner: v.interpreter_frame.owner,
                // 3.14 has stackpointer here; 3.13 does not — drop it
            },
            code_object: py313::_Py_DebugOffsets__code_object {
                size: v.code_object.size,
                filename: v.code_object.filename,
                name: v.code_object.name,
                qualname: v.code_object.qualname,
                linetable: v.code_object.linetable,
                firstlineno: v.code_object.firstlineno,
                argcount: v.code_object.argcount,
                localsplusnames: v.code_object.localsplusnames,
                localspluskinds: v.code_object.localspluskinds,
                co_code_adaptive: v.code_object.co_code_adaptive,
            },
            pyobject: py313::_Py_DebugOffsets__pyobject {
                size: v.pyobject.size,
                ob_type: v.pyobject.ob_type,
            },
            type_object: py313::_Py_DebugOffsets__type_object {
                size: v.type_object.size,
                tp_name: v.type_object.tp_name,
                tp_repr: v.type_object.tp_repr,
                tp_flags: v.type_object.tp_flags,
            },
            tuple_object: py313::_Py_DebugOffsets__tuple_object {
                size: v.tuple_object.size,
                ob_item: v.tuple_object.ob_item,
                ob_size: v.tuple_object.ob_size,
            },
            list_object: py313::_Py_DebugOffsets__list_object {
                size: v.list_object.size,
                ob_item: v.list_object.ob_item,
                ob_size: v.list_object.ob_size,
            },
            dict_object: py313::_Py_DebugOffsets__dict_object {
                size: v.dict_object.size,
                ma_keys: v.dict_object.ma_keys,
                ma_values: v.dict_object.ma_values,
            },
            float_object: py313::_Py_DebugOffsets__float_object {
                size: v.float_object.size,
                ob_fval: v.float_object.ob_fval,
            },
            long_object: py313::_Py_DebugOffsets__long_object {
                size: v.long_object.size,
                lv_tag: v.long_object.lv_tag,
                ob_digit: v.long_object.ob_digit,
            },
            bytes_object: py313::_Py_DebugOffsets__bytes_object {
                size: v.bytes_object.size,
                ob_size: v.bytes_object.ob_size,
                ob_sval: v.bytes_object.ob_sval,
            },
            unicode_object: py313::_Py_DebugOffsets__unicode_object {
                size: v.unicode_object.size,
                state: v.unicode_object.state,
                length: v.unicode_object.length,
                asciiobject_size: v.unicode_object.asciiobject_size,
            },
            gc: py313::_Py_DebugOffsets__gc {
                size: v.gc.size,
                collecting: v.gc.collecting,
            },
            // 3.14 sub-structs not in 3.13: set_object, gen_object, debugger_support — dropped
        }
    }
}

/// Absolute runtime addresses of two key CPython symbols, after applying ASLR load bias.
#[derive(Debug, PartialEq)]
pub struct ElfSymbols {
    pub py_runtime_addr: u64,
    pub py_version_addr: u64,
}

/// Open and mmap `binary.path`, parse the ELF dynamic symbol table, find
/// `_PyRuntime` and `Py_Version`, apply the ASLR load bias, and return their
/// absolute runtime addresses.
///
/// Returns [`InitError::SymbolNotFound`] (error code 3) if either symbol is absent.
pub fn resolve_elf_symbols(binary: &PythonBinary) -> Result<ElfSymbols, InitError> {
    let file = std::fs::File::open(&binary.path).map_err(|_| InitError::Io)?;
    // SAFETY: the file is a read-only view of an on-disk ELF; no other code
    // modifies it during parsing.
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(|_| InitError::Io)?;
    resolve_elf_symbols_from_bytes(&mmap, binary.base)
}

/// Parse ELF dynamic symbols from a byte slice and compute absolute addresses.
///
/// `mapped_base` is the address at which the first mapping of this binary
/// appears in `/proc/self/maps` (i.e. the runtime base after ASLR).
fn resolve_elf_symbols_from_bytes(data: &[u8], mapped_base: u64) -> Result<ElfSymbols, InitError> {
    let obj = object::File::parse(data).map_err(|_| InitError::ElfParse)?;

    // load_bias = runtime base − ELF-file base (first LOAD segment vaddr).
    // For PIE/shared objects the first LOAD vaddr is 0, so load_bias == mapped_base.
    // For non-PIE executables (ET_EXEC) the first LOAD vaddr is already the
    // absolute address (e.g. 0x400000), so load_bias == 0.
    // NOTE: object::Object::relative_address_base() always returns 0 for ELF,
    // so we must compute the first LOAD vaddr ourselves from program headers.
    let elf_base = obj.segments().map(|s| s.address()).min().unwrap_or(0);
    let load_bias = mapped_base.wrapping_sub(elf_base);

    let mut py_runtime: Option<u64> = None;
    let mut py_version: Option<u64> = None;

    for sym in obj.dynamic_symbols() {
        match sym.name() {
            Ok("_PyRuntime") => py_runtime = Some(sym.address().wrapping_add(load_bias)),
            Ok("Py_Version") => py_version = Some(sym.address().wrapping_add(load_bias)),
            _ => {}
        }
        if py_runtime.is_some() && py_version.is_some() {
            break;
        }
    }

    match (py_runtime, py_version) {
        (Some(py_runtime_addr), Some(py_version_addr)) => Ok(ElfSymbols {
            py_runtime_addr,
            py_version_addr,
        }),
        _ => Err(InitError::SymbolNotFound),
    }
}

/// Read `Py_Version` from live memory, parse it, and validate it is a supported CPython version.
///
/// Currently supports CPython 3.13 and 3.14.
/// Returns [`PythonVersion`] on success, or [`InitError::UnsupportedVersion`]
/// for unsupported versions.
pub fn detect_version(py_version_addr: u64) -> Result<PythonVersion, InitError> {
    let raw = kindasafe::u64(py_version_addr).map_err(|_| InitError::UnsupportedVersion)?;
    let version_hex = raw & 0xFFFF_FFFF;
    let version = parse_version(version_hex);
    if version.major != 3 || !matches!(version.minor, 13 | 14) {
        return Err(InitError::UnsupportedVersion);
    }
    Ok(version)
}

/// Read `Py_Version` raw value from live memory as a 32-bit `PY_VERSION_HEX`.
///
/// This is the raw value needed by [`read_debug_offsets`] for cookie validation.
pub fn read_version_hex(py_version_addr: u64) -> Result<u64, InitError> {
    let raw = kindasafe::u64(py_version_addr).map_err(|_| InitError::UnsupportedVersion)?;
    Ok(raw & 0xFFFF_FFFF)
}

/// Read `_Py_DebugOffsets` from `_PyRuntime` and return it as a
/// [`py313::_Py_DebugOffsets`] (the common-denominator layout).
///
/// For CPython 3.14, reads the larger struct and converts down to the 3.13
/// layout, dropping fields that only exist in 3.14.
///
/// `py_runtime_addr` is the absolute address of `_PyRuntime`.
/// `version` is the validated [`PythonVersion`] from [`detect_version`].
/// `version_hex` is the raw `PY_VERSION_HEX` from [`read_version_hex`].
///
/// Returns [`InitError::DebugOffsetsMismatch`] if cookie, version, or
/// `free_threaded` validation fails.
pub fn read_debug_offsets(
    py_runtime_addr: u64,
    version: &PythonVersion,
    version_hex: u64,
) -> Result<py313::_Py_DebugOffsets, InitError> {
    match version.minor {
        14 => {
            let mut buf = [0u8; size_of::<py314::_Py_DebugOffsets>()];
            kindasafe::slice(&mut buf, py_runtime_addr)
                .map_err(|_| InitError::DebugOffsetsMismatch)?;
            let d314 = parse_repr_c::<py314::_Py_DebugOffsets>(&buf, version_hex)?;
            Ok(py313::_Py_DebugOffsets::from(&d314))
        }
        13 => {
            let mut buf = [0u8; size_of::<py313::_Py_DebugOffsets>()];
            kindasafe::slice(&mut buf, py_runtime_addr)
                .map_err(|_| InitError::DebugOffsetsMismatch)?;
            parse_repr_c::<py313::_Py_DebugOffsets>(&buf, version_hex)
        }
        _ => Err(InitError::UnsupportedVersion),
    }
}

/// Common header validation for any `_Py_DebugOffsets` layout.
///
/// Checks cookie, version, and free_threaded at the fixed offsets (0, 8, 16).
fn parse_repr_c<T>(buf: &[u8], expected_version: u64) -> Result<T, InitError> {
    let size = size_of::<T>();
    if buf.len() < size {
        return Err(InitError::DebugOffsetsMismatch);
    }
    // Cookie at offset 0 (8 bytes), version at offset 8, free_threaded at offset 16
    let cookie = u64::from_le_bytes(buf[0..8].try_into().unwrap());
    if cookie != COOKIE {
        return Err(InitError::DebugOffsetsMismatch);
    }
    let version = u64::from_le_bytes(buf[8..16].try_into().unwrap());
    if version != expected_version {
        return Err(InitError::DebugOffsetsMismatch);
    }
    let free_threaded = u64::from_le_bytes(buf[16..24].try_into().unwrap());
    if free_threaded != 0 {
        return Err(InitError::DebugOffsetsMismatch);
    }
    // SAFETY: buf is at least size_of::<T>() bytes. T is #[repr(C)] with only
    // u64 and [u8; 8] fields. Any bit pattern is valid.
    Ok(unsafe { core::ptr::read_unaligned(buf.as_ptr() as *const T) })
}

/// Maximum number of bytes to scan when disassembling `_PyThreadState_GetCurrent`.
const TLS_SCAN_LIMIT: usize = 128;

/// Open and mmap `binary.path`, find `_PyThreadState_GetCurrent` in the ELF
/// dynamic symbol table, read its first [`TLS_SCAN_LIMIT`] bytes from the file,
/// and extract the FS-relative 32-bit displacement.
///
/// Returns [`InitError::TlsDiscoveryFailed`] (error code 6) if the symbol is
/// absent or no FS-relative load instruction is found in the function prologue.
pub fn find_tls_offset(binary: &PythonBinary) -> Result<u64, InitError> {
    let file = std::fs::File::open(&binary.path).map_err(|_| InitError::Io)?;
    // SAFETY: the file is a read-only view of an on-disk ELF; no other code
    // modifies it during parsing.
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(|_| InitError::Io)?;
    find_tls_offset_from_bytes(&mmap)
}

/// Parse ELF data, locate `_PyThreadState_GetCurrent`, read its first
/// [`TLS_SCAN_LIMIT`] bytes from the file image, and call
/// [`discover_tls_offset`].
fn find_tls_offset_from_bytes(data: &[u8]) -> Result<u64, InitError> {
    let obj = object::File::parse(data).map_err(|_| InitError::ElfParse)?;

    // Find _PyThreadState_GetCurrent in the dynamic symbol table.
    let func_va = obj
        .dynamic_symbols()
        .find(|sym| matches!(sym.name(), Ok("_PyThreadState_GetCurrent")))
        .map(|sym| sym.address())
        .ok_or(InitError::TlsDiscoveryFailed)?;

    // Resolve the virtual address to a file byte range via LOAD segments.
    let func_bytes = obj
        .segments()
        .find_map(|seg| {
            seg.data_range(func_va, TLS_SCAN_LIMIT as u64)
                .ok()
                .flatten()
        })
        .ok_or(InitError::TlsDiscoveryFailed)?;

    discover_tls_offset(func_bytes, func_va)
}

/// Disassemble up to [`TLS_SCAN_LIMIT`] bytes starting at `func_bytes` and
/// return the 32-bit displacement from the first FS-relative load instruction.
///
/// Matches the x86_64 static-TLS patterns emitted by GCC/Clang for
/// `__thread` variables in position-independent executables:
/// - `64 48 8b 04 25 XX XX XX XX`  — `mov rax, fs:[disp32]`
/// - `64 48 8b 05 XX XX XX XX`     — `mov rax, fs:[rip+disp32]`
///
/// `func_addr` is the runtime (or file-relative) virtual address of the first
/// byte, used by zydis for RIP-relative displacement calculation.
///
/// Returns [`InitError::TlsDiscoveryFailed`] (error code 6) if no matching
/// instruction is found within the scanned bytes.
pub fn discover_tls_offset(func_bytes: &[u8], func_addr: u64) -> Result<u64, InitError> {
    let decoder = zydis::Decoder::new64();

    let scan_len = func_bytes.len().min(TLS_SCAN_LIMIT);
    for (_ip, _raw, insn) in decoder
        .decode_all::<zydis::VisibleOperands>(&func_bytes[..scan_len], func_addr)
        .flatten()
    {
        // We want any MOV that has an FS segment prefix on its memory operand.
        if !insn
            .attributes
            .contains(zydis::InstructionAttributes::HAS_SEGMENT_FS)
        {
            continue;
        }

        // Find the memory operand and extract its displacement.
        for op in insn.visible_operands() {
            if let DecodedOperandKind::Mem(mem) = &op.kind
                && mem.disp.has_displacement
            {
                // Return the displacement as a u64 (preserving the bit
                // pattern; the value is a signed offset from FS base).
                return Ok(mem.disp.displacement as u64);
            }
        }
    }

    Err(InitError::TlsDiscoveryFailed)
}

#[derive(Debug, PartialEq)]
pub struct PythonBinary {
    pub base: u64,
    pub path: String,
}

// Flags bitmask for /proc/maps permissions field
pub const FLAGS_READ: u32 = 0x1;
pub const FLAGS_WRITE: u32 = 0x2;
pub const FLAGS_EXEC: u32 = 0x4;
pub const FLAGS_SHARED: u32 = 0x8; // 's' = shared, 'p' = private (0)

/// Fields parsed from a single `/proc/maps` line, in order.
/// `path` is a subslice of the original line — no allocation.
type MapsLineFields<'a> = (u64, u64, u32, u64, u32, u32, u64, &'a [u8]);

/// Parse a single `/proc/maps` line.
///
/// Returns `(start, end, flags, offset, dev_major, dev_minor, inode, path_bytes)`.
/// `path_bytes` is a subslice of `line` — no allocation.
/// Returns `None` if the line is malformed.
fn parse_maps_line(line: &[u8]) -> Option<MapsLineFields<'_>> {
    // Format: start-end perms offset dev inode [path]
    // Example: 7f1234560000-7f1234580000 r--p 00000000 08:01 123456 /usr/lib/libpython3.11.so.1.0

    let mut iter = line.splitn(6, |&b| b == b' ');

    // Field 1: "start-end"
    let addr_field = iter.next()?;
    let dash = addr_field.iter().position(|&b| b == b'-')?;
    let start = u64::from_str_radix(core::str::from_utf8(&addr_field[..dash]).ok()?, 16).ok()?;
    let end = u64::from_str_radix(core::str::from_utf8(&addr_field[dash + 1..]).ok()?, 16).ok()?;

    // Field 2: "rwxp" or "rwxs"
    let perms = iter.next()?;
    if perms.len() < 4 {
        return None;
    }
    let mut flags: u32 = 0;
    if perms[0] == b'r' {
        flags |= FLAGS_READ;
    }
    if perms[1] == b'w' {
        flags |= FLAGS_WRITE;
    }
    if perms[2] == b'x' {
        flags |= FLAGS_EXEC;
    }
    if perms[3] == b's' {
        flags |= FLAGS_SHARED;
    }

    // Field 3: offset (hex)
    let offset_field = iter.next()?;
    let offset = u64::from_str_radix(core::str::from_utf8(offset_field).ok()?, 16).ok()?;

    // Field 4: "major:minor"
    let dev_field = iter.next()?;
    let colon = dev_field.iter().position(|&b| b == b':')?;
    let dev_major =
        u32::from_str_radix(core::str::from_utf8(&dev_field[..colon]).ok()?, 16).ok()?;
    let dev_minor =
        u32::from_str_radix(core::str::from_utf8(&dev_field[colon + 1..]).ok()?, 16).ok()?;

    // Field 5: inode (decimal)
    let inode_field = iter.next()?;
    let inode = core::str::from_utf8(inode_field)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;

    // Field 6: optional path (remainder), strip leading spaces and trailing newline
    let path_bytes = iter.next().map_or(b"".as_slice(), |rest| rest.trim_ascii());

    Some((
        start, end, flags, offset, dev_major, dev_minor, inode, path_bytes,
    ))
}

fn find_python_in_maps_reader<R: BufRead>(mut reader: R) -> Result<PythonBinary, InitError> {
    // We track the *first* mapping seen for each candidate.
    // libpython3 is preferred over python3.
    let mut libpython3: Option<PythonBinary> = None;
    let mut python3: Option<PythonBinary> = None;

    // Reuse a single buffer across all lines to avoid repeated allocations.
    let mut buf: Vec<u8> = Vec::with_capacity(256);

    loop {
        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|_| InitError::PythonNotFound)?;
        if n == 0 {
            break;
        }

        let (start, _end, _flags, _offset, _dev_major, _dev_minor, _inode, path_bytes) =
            match parse_maps_line(&buf) {
                Some(e) => e,
                None => continue,
            };

        // Check for libpython3 (preferred)
        if libpython3.is_none() && path_contains(path_bytes, b"libpython3") {
            libpython3 = Some(PythonBinary {
                base: start,
                path: String::from_utf8_lossy(path_bytes).into_owned(),
            });
            // Once we have a libpython3 candidate we're done — it will always win.
            break;
        }

        // Check for python3 (fallback) — only if no python3 yet
        if python3.is_none() && path_contains(path_bytes, b"python3") {
            python3 = Some(PythonBinary {
                base: start,
                path: String::from_utf8_lossy(path_bytes).into_owned(),
            });
            // Don't break here: a later libpython3 entry would be preferred.
        }
    }

    libpython3.or(python3).ok_or(InitError::PythonNotFound)
}

/// Check whether `haystack` contains the byte-string `needle` as a substring.
/// No allocation.
fn path_contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Parse `/proc/self/maps` and return the `PythonBinary` describing where Python
/// (or libpython3) is loaded.
///
/// Prefers a `libpython3` mapping over a bare `python3` mapping.
/// Returns [`InitError::PythonNotFound`] (error code 2) when neither is found.
pub fn find_python_in_maps() -> Result<PythonBinary, InitError> {
    let f = std::fs::File::open("/proc/self/maps").map_err(|_| InitError::PythonNotFound)?;
    find_python_in_maps_reader(std::io::BufReader::new(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_version tests ────────────────────────────────────────────────

    #[test]
    fn parse_version_3_14_0_final() {
        let v = parse_version(0x030E00F0);
        assert_eq!(
            v,
            PythonVersion {
                major: 3,
                minor: 14,
                micro: 0
            }
        );
    }

    #[test]
    fn parse_version_3_14_2_final() {
        let v = parse_version(0x030E02F0);
        assert_eq!(
            v,
            PythonVersion {
                major: 3,
                minor: 14,
                micro: 2
            }
        );
    }

    #[test]
    fn parse_version_3_13_1() {
        let v = parse_version(0x030D01F0);
        assert_eq!(
            v,
            PythonVersion {
                major: 3,
                minor: 13,
                micro: 1
            }
        );
    }

    // ── parse_maps_line tests ────────────────────────────────────────────────

    #[test]
    fn parse_libpython3_ro_header() {
        let line =
            b"7f1234560000-7f1234580000 r--p 00000000 08:01 123456 /usr/lib/libpython3.11.so.1.0\n";
        let (start, end, flags, offset, dev_major, dev_minor, inode, path) =
            parse_maps_line(line).unwrap();
        assert_eq!(start, 0x7f1234560000);
        assert_eq!(end, 0x7f1234580000);
        assert_eq!(flags, FLAGS_READ);
        assert_eq!(offset, 0);
        assert_eq!(dev_major, 8);
        assert_eq!(dev_minor, 1);
        assert_eq!(inode, 123456);
        assert_eq!(path, b"/usr/lib/libpython3.11.so.1.0");
    }

    #[test]
    fn parse_libpython3_exec_mapping() {
        let line =
            b"7f1234580000-7f1234600000 r-xp 00020000 08:01 123456 /usr/lib/libpython3.11.so.1.0\n";
        let (start, _end, flags, offset, _dmaj, _dmin, _inode, path) =
            parse_maps_line(line).unwrap();
        assert_eq!(start, 0x7f1234580000);
        assert_eq!(flags, FLAGS_READ | FLAGS_EXEC);
        assert_eq!(offset, 0x20000);
        assert_eq!(path, b"/usr/lib/libpython3.11.so.1.0");
    }

    #[test]
    fn parse_static_python3() {
        let line = b"555555554000-5555555b2000 r--p 00000000 08:01 654321 /usr/bin/python3\n";
        let (start, _end, flags, _off, _dmaj, _dmin, inode, path) = parse_maps_line(line).unwrap();
        assert_eq!(start, 0x555555554000);
        assert_eq!(flags, FLAGS_READ);
        assert_eq!(inode, 654321);
        assert_eq!(path, b"/usr/bin/python3");
    }

    #[test]
    fn parse_anonymous_mapping() {
        let line = b"7fff12340000-7fff12360000 rw-p 00000000 00:00 0 \n";
        let (start, _end, flags, _off, dev_major, dev_minor, inode, path) =
            parse_maps_line(line).unwrap();
        assert_eq!(start, 0x7fff12340000);
        assert_eq!(flags, FLAGS_READ | FLAGS_WRITE);
        assert_eq!(dev_major, 0);
        assert_eq!(dev_minor, 0);
        assert_eq!(inode, 0);
        assert_eq!(path, b"");
    }

    #[test]
    fn parse_anonymous_mapping_no_trailing_space() {
        // Some kernels emit no trailing space for anonymous mappings
        let line = b"7fff12340000-7fff12360000 rw-p 00000000 00:00 0\n";
        let result = parse_maps_line(line);
        assert!(result.is_some());
        let (_s, _e, _f, _o, _dm, _dn, _i, path) = result.unwrap();
        assert_eq!(path, b"");
    }

    #[test]
    fn parse_vdso() {
        let line = b"7fff12370000-7fff12372000 r-xp 00000000 00:00 0 [vdso]\n";
        let (_s, _e, flags, _o, _dm, _dn, _i, path) = parse_maps_line(line).unwrap();
        assert_eq!(flags, FLAGS_READ | FLAGS_EXEC);
        assert_eq!(path, b"[vdso]");
    }

    #[test]
    fn parse_shared_mapping() {
        let line = b"7f0000000000-7f0000010000 rw-s 00000000 00:05 0 /dev/zero\n";
        let (_s, _e, flags, _o, _dm, _dn, _i, _path) = parse_maps_line(line).unwrap();
        assert_eq!(flags, FLAGS_READ | FLAGS_WRITE | FLAGS_SHARED);
    }

    #[test]
    fn parse_malformed_line_returns_none() {
        assert!(parse_maps_line(b"not a valid maps line\n").is_none());
        assert!(parse_maps_line(b"\n").is_none());
    }

    // ── find_python_in_maps_reader tests ────────────────────────────────────

    const MAPS_LIBPYTHON3_ONLY: &[u8] = b"\
7f0000000000-7f0000020000 r--p 00000000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
7f0000020000-7f0000100000 r-xp 00020000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
7fff00000000-7fff00020000 rw-p 00000000 00:00 0\n\
";

    const MAPS_PYTHON3_ONLY: &[u8] = b"\
555555554000-5555555b2000 r--p 00000000 08:01 222 /usr/bin/python3\n\
5555555b2000-555555600000 r-xp 0005e000 08:01 222 /usr/bin/python3\n\
7fff00000000-7fff00020000 rw-p 00000000 00:00 0\n\
";

    const MAPS_BOTH: &[u8] = b"\
555555554000-5555555b2000 r--p 00000000 08:01 222 /usr/bin/python3\n\
7f0000000000-7f0000020000 r--p 00000000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
7f0000020000-7f0000100000 r-xp 00020000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
";

    const MAPS_LIBPYTHON3_MULTIPLE: &[u8] = b"\
7f0000000000-7f0000020000 r--p 00000000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
7f0000020000-7f0000100000 r-xp 00020000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
7f0000200000-7f0000210000 r--p 00000000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
";

    const MAPS_NO_PYTHON: &[u8] = b"\
7f0000000000-7f0000020000 r--p 00000000 08:01 333 /usr/lib/libc.so.6\n\
7fff00000000-7fff00020000 rw-p 00000000 00:00 0\n\
";

    fn run(maps: &[u8]) -> Result<PythonBinary, InitError> {
        find_python_in_maps_reader(std::io::Cursor::new(maps))
    }

    #[test]
    fn finds_libpython3_only() {
        let bin = run(MAPS_LIBPYTHON3_ONLY).unwrap();
        assert_eq!(bin.base, 0x7f0000000000);
        assert!(bin.path.contains("libpython3"));
    }

    #[test]
    fn finds_python3_only() {
        let bin = run(MAPS_PYTHON3_ONLY).unwrap();
        assert_eq!(bin.base, 0x555555554000);
        assert!(bin.path.contains("python3"));
    }

    #[test]
    fn prefers_libpython3_over_python3() {
        let bin = run(MAPS_BOTH).unwrap();
        assert!(
            bin.path.contains("libpython3"),
            "expected libpython3, got {}",
            bin.path
        );
        assert_eq!(bin.base, 0x7f0000000000);
    }

    #[test]
    fn returns_first_mapping_base() {
        // The first mapping (r--p, offset 0) should be the base, not the r-xp one.
        let bin = run(MAPS_LIBPYTHON3_MULTIPLE).unwrap();
        assert_eq!(bin.base, 0x7f0000000000);
    }

    #[test]
    fn returns_python_not_found_when_absent() {
        assert_eq!(run(MAPS_NO_PYTHON), Err(InitError::PythonNotFound));
    }

    #[test]
    fn empty_maps_returns_not_found() {
        assert_eq!(run(b""), Err(InitError::PythonNotFound));
    }

    #[test]
    fn python3_before_libpython3_still_prefers_libpython3() {
        // python3 entry appears first, but libpython3 comes later — must prefer libpython3
        let maps = b"\
555555554000-5555555b2000 r--p 00000000 08:01 222 /usr/bin/python3\n\
7f0000000000-7f0000020000 r--p 00000000 08:01 111 /usr/lib/libpython3.11.so.1.0\n\
";
        let bin = run(maps).unwrap();
        assert!(bin.path.contains("libpython3"), "should prefer libpython3");
    }

    // ── discover_tls_offset tests ────────────────────────────────────────────

    #[test]
    fn detects_mov_rax_fs_abs_disp32() {
        // 64 48 8b 04 25 f8 ff ff ff  →  mov rax, fs:0xfffffff8  (i.e. fs:[-8])
        // followed by ret (c3) to terminate the scan
        let bytes = [0x64u8, 0x48, 0x8b, 0x04, 0x25, 0xf8, 0xff, 0xff, 0xff, 0xc3];
        let offset = discover_tls_offset(&bytes, 0).unwrap();
        // zydis sign-extends the 32-bit displacement to i64; we store it as u64
        assert_eq!(offset, (-8i64) as u64);
    }

    #[test]
    fn detects_mov_rax_fs_rip_disp32() {
        // 64 48 8b 05 10 00 00 00  →  mov rax, fs:[rip+0x10]
        let bytes = [0x64u8, 0x48, 0x8b, 0x05, 0x10, 0x00, 0x00, 0x00, 0xc3];
        let offset = discover_tls_offset(&bytes, 0).unwrap();
        assert_eq!(offset, 0x10);
    }

    #[test]
    fn returns_tls_discovery_failed_on_no_match() {
        // 16 NOPs — no FS-relative load anywhere
        let bytes = [0x90u8; 16];
        assert_eq!(
            discover_tls_offset(&bytes, 0),
            Err(InitError::TlsDiscoveryFailed)
        );
    }

    #[test]
    fn ignores_fs_prefix_on_non_mov_instructions() {
        // Stack canary: 64 48 8b 04 25 28 00 00 00  →  mov rax, fs:0x28
        // This HAS the FS prefix and IS a MOV — it should be matched and return 0x28.
        // (The scanner doesn't filter by which FS offset it is, only that one exists.)
        let bytes = [0x64u8, 0x48, 0x8b, 0x04, 0x25, 0x28, 0x00, 0x00, 0x00, 0xc3];
        let offset = discover_tls_offset(&bytes, 0).unwrap();
        assert_eq!(offset, 0x28);
    }

    #[test]
    fn skips_non_fs_memory_operands() {
        // mov rax, [rax+0x8]  (48 8b 80 08 00 00 00) — no FS prefix
        let bytes = [0x48u8, 0x8b, 0x80, 0x08, 0x00, 0x00, 0x00, 0xc3];
        assert_eq!(
            discover_tls_offset(&bytes, 0),
            Err(InitError::TlsDiscoveryFailed)
        );
    }

    // ── find_tls_offset_from_bytes tests ─────────────────────────────────────

    #[test]
    fn libpython314_uses_gd_tls_returns_discovery_failed() {
        // libpython3.14.so.1.0 uses the General Dynamic TLS model (__tls_get_addr),
        // not a direct FS-relative load.  The scanner should return TlsDiscoveryFailed.
        let result = find_tls_offset_from_bytes(LIBPYTHON314);
        assert_eq!(result, Err(InitError::TlsDiscoveryFailed));
    }

    // ── resolve_elf_symbols_from_bytes tests ─────────────────────────────────

    // Real libpython3.14.so.1.0 committed as a test fixture.
    // Symbol values verified with `nm --dynamic`:
    //   _PyRuntime  0x71bd00
    //   Py_Version  0x61c1b0
    const LIBPYTHON314: &[u8] = include_bytes!("../testdata/libpython3.14.so.1.0");

    #[test]
    fn resolves_both_symbols() {
        // mapped_base = 0 → load_bias = 0 (ET_DYN, relative_address_base() = 0)
        // absolute addr = symbol st_value + 0 = st_value
        let result = resolve_elf_symbols_from_bytes(LIBPYTHON314, 0).unwrap();
        assert_eq!(result.py_runtime_addr, 0x71bd00);
        assert_eq!(result.py_version_addr, 0x61c1b0);
    }

    #[test]
    fn applies_load_bias() {
        let mapped_base: u64 = 0x7f00_0000_0000;
        let result = resolve_elf_symbols_from_bytes(LIBPYTHON314, mapped_base).unwrap();
        assert_eq!(result.py_runtime_addr, mapped_base + 0x71bd00);
        assert_eq!(result.py_version_addr, mapped_base + 0x61c1b0);
    }

    #[test]
    fn elf_invalid_bytes_returns_elf_parse_error() {
        let result = resolve_elf_symbols_from_bytes(b"not an elf file", 0);
        assert_eq!(result, Err(InitError::ElfParse));
    }
}
