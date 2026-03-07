use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicU32, Ordering};

const STATE_UNINITIALIZED: u32 = 0;
const STATE_RUNNING: u32 = 1;

static LIFECYCLE: AtomicU32 = AtomicU32::new(STATE_UNINITIALIZED);

/// Signal handler called on every SIGPROF tick (~10 ms of CPU time).
///
/// Currently just prints a debug message. In debug builds this emits
/// "SIGPROF fired\n" via raw `SYS_write`; in release builds it is a no-op.
extern "C" fn on_sigprof(_sig: c_int, _info: *mut libc::siginfo_t, _ctx: *mut c_void) {
    notlibc::debug::puts("SIGPROF fired");
}

/// Start the CPython profiler.
///
/// Runs the full init sequence: kindasafe crash recovery, Python binary
/// discovery, ELF symbol resolution, version detection, debug offsets,
/// TLS offset discovery, then installs a SIGPROF handler + 10 ms timer.
///
/// Returns 0 on success, nonzero error code on failure (see design doc
/// section 17.3). There is no stop function — the profiler runs for the
/// lifetime of the process.
///
/// # Safety
///
/// `app_name` and `server_url` must be valid pointers to NUL-terminated
/// C strings, or null (which returns error code 1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pyroscope_start(
    _app_name: *const c_char,
    _server_url: *const c_char,
) -> c_int {
    if LIFECYCLE
        .compare_exchange(
            STATE_UNINITIALIZED,
            STATE_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return 9;
    }

    match init_sequence() {
        Ok(()) => 0,
        Err(code) => {
            LIFECYCLE.store(STATE_UNINITIALIZED, Ordering::Release);
            code
        }
    }
}

fn init_sequence() -> Result<(), c_int> {
    // Step 1: Install kindasafe SIGSEGV/SIGBUS recovery.
    kindasafe_init::init().map_err(|_| 1)?;

    // Step 2: Find Python binary in /proc/self/maps.
    let binary = python_offsets::find_python_in_maps().map_err(|e| map_init_error(&e))?;

    // Step 3: Resolve _PyRuntime and Py_Version ELF symbols.
    let symbols = python_offsets::resolve_elf_symbols(&binary).map_err(|e| map_init_error(&e))?;

    // Step 4: Detect and validate Python version.
    let version =
        python_offsets::detect_version(symbols.py_version_addr).map_err(|e| map_init_error(&e))?;

    // Read raw version hex (needed by read_debug_offsets).
    let version_hex = python_offsets::read_version_hex(symbols.py_version_addr)
        .map_err(|e| map_init_error(&e))?;

    // Step 5: Read _Py_DebugOffsets from _PyRuntime.
    let _debug_offsets =
        python_offsets::read_debug_offsets(symbols.py_runtime_addr, &version, version_hex)
            .map_err(|e| map_init_error(&e))?;

    // Step 6: Discover TLS offset for _PyThreadState_GetCurrent.
    let _tls_offset = python_offsets::find_tls_offset(&binary).map_err(|e| map_init_error(&e))?;

    // Steps 7-11 skipped: ring buffers, shards, global state, reader thread
    // — not yet implemented.

    // Steps 12+13: Install SIGPROF handler and start 10 ms ITIMER_PROF timer.
    sighandler::start(on_sigprof).map_err(|_| 8)?;

    notlibc::debug::puts("pyroscope_cpython: init complete");
    Ok(())
}

/// Map `python_offsets::InitError` variants to integer error codes.
fn map_init_error(err: &python_offsets::InitError) -> c_int {
    match err {
        python_offsets::InitError::KindasafeInitFailed => 1,
        python_offsets::InitError::PythonNotFound => 2,
        python_offsets::InitError::Io => 2,
        python_offsets::InitError::SymbolNotFound => 3,
        python_offsets::InitError::ElfParse => 3,
        python_offsets::InitError::DebugOffsetsMismatch => 4,
        python_offsets::InitError::UnsupportedVersion => 5,
        python_offsets::InitError::TlsDiscoveryFailed => 6,
    }
}
