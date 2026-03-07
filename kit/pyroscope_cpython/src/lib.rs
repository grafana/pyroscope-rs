use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use python_offsets_types::py313;

const STATE_UNINITIALIZED: u32 = 0;
const STATE_RUNNING: u32 = 1;

static LIFECYCLE: AtomicU32 = AtomicU32::new(STATE_UNINITIALIZED);

/// Profiler state shared between init (writer) and signal handler (reader).
///
/// Allocated once at init time via `Box::into_raw`. Published to the handler
/// via an `AtomicPtr` store with `Release` ordering; the handler loads with
/// `Acquire`. Never deallocated — the profiler runs for the process lifetime.
struct HandlerState {
    debug_offsets: py313::_Py_DebugOffsets,
    /// FS-relative displacement for the CPython TLS thread-state variable.
    /// This is the raw value from `mov rax, fs:[disp32]` in
    /// `_PyThreadState_GetCurrent`, stored as `u64` (may be a negative `i64`
    /// bit pattern for typical glibc static TLS).
    tls_offset: u64,
    /// Expected type-object addresses for runtime type checking.
    type_addrs: python_unwind::TypeAddrs,
}

static HANDLER_STATE: AtomicPtr<HandlerState> = AtomicPtr::new(core::ptr::null_mut());

/// Signal handler called on every SIGPROF tick (~10 ms of CPU time).
///
/// Reads the FS base via `kindasafe::fs_0x0()`, computes the TLS address
/// of `_Py_tss_tstate`, reads the current `PyThreadState*`, and unwinds
/// the Python frame chain. In debug builds, `python_unwind::unwind` prints
/// each frame's code object address and instruction pointer to stdout.
extern "C" fn on_sigprof(_sig: c_int, _info: *mut libc::siginfo_t, _ctx: *mut c_void) {
    // Step 1: Load global profiler state (Acquire pairs with Release in init).
    let state_ptr = HANDLER_STATE.load(Ordering::Acquire);
    if state_ptr.is_null() {
        return;
    }
    // SAFETY: state_ptr was published via Box::into_raw after full
    // initialization. It is never deallocated (profiler runs for process
    // lifetime). The handler only reads from it.
    let state = unsafe { &*state_ptr };

    // Step 2: Read FS base (thread self-pointer at fs:0x0).
    let fs_base = match kindasafe::fs_0x0() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Step 3: Read tstate from TLS.
    // tls_offset is the displacement used in `mov rax, fs:[disp32]`.
    // On glibc this is typically negative for static TLS; wrapping_add
    // handles the sign correctly since tls_offset stores the u64 bit
    // pattern of the signed displacement.
    let tstate_addr = fs_base.wrapping_add(state.tls_offset);
    let tstate = match kindasafe::u64(tstate_addr) {
        Ok(v) => v,
        Err(_) => return,
    };
    if tstate == 0 {
        return;
    }

    // Step 4: Unwind Python stack frames.
    // Stack-allocated buffer — 128 frames * 16 bytes = 2048 bytes, well
    // within the default signal stack size.
    let mut buf = [python_unwind::RawFrame {
        code_object: 0,
        instr_offset: 0,
    }; python_unwind::MAX_DEPTH];
    python_unwind::unwind(tstate, &state.debug_offsets, &state.type_addrs, &mut buf);
    // python_unwind::unwind() prints each frame via notlibc::debug
    // (raw SYS_write syscall).
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
    let debug_offsets =
        python_offsets::read_debug_offsets(symbols.py_runtime_addr, &version, version_hex)
            .map_err(|e| map_init_error(&e))?;

    // Step 6: Discover TLS offset for _PyThreadState_GetCurrent.
    let tls_offset = python_offsets::find_tls_offset(&binary).map_err(|e| map_init_error(&e))?;

    // Publish handler state before installing the signal handler.
    let type_addrs = python_unwind::TypeAddrs {
        code_type: symbols.py_code_type_addr,
    };
    let state = Box::new(HandlerState {
        debug_offsets,
        tls_offset,
        type_addrs,
    });
    HANDLER_STATE.store(Box::into_raw(state), Ordering::Release);

    // Steps 7-11 skipped: ring buffers, shards, reader thread
    // — not yet implemented. For now the handler just unwinds and prints.

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
        python_offsets::InitError::SymbolNotFound(_) => 3,
        python_offsets::InitError::ElfParse => 3,
        python_offsets::InitError::DebugOffsetsMismatch => 4,
        python_offsets::InitError::UnsupportedVersion => 5,
        python_offsets::InitError::TlsDiscoveryFailed => 6,
    }
}
