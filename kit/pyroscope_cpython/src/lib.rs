use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use bbqueue::framed::{FrameConsumer, FrameProducer};
use python_offsets_types::py313;
use python_unwind::RawFrame;
use sig_ring::RING_SIZE;

const STATE_UNINITIALIZED: u32 = 0;
const STATE_RUNNING: u32 = 1;

/// Default number of shards for concurrent signal handler access.
const DEFAULT_NUM_SHARDS: usize = 16;

static LIFECYCLE: AtomicU32 = AtomicU32::new(STATE_UNINITIALIZED);

/// Whether to log diagnostic messages to stderr. Off by default.
static LOG_ENABLED: AtomicBool = AtomicBool::new(false);

// ── Error codes ──────────────────────────────────────────────────────────────

/// Error codes returned by `pyroscope_start`.
#[repr(i32)]
#[derive(Copy, Clone)]
enum InitError {
    KindasafeInit = 1,
    PythonNotFound = 2,
    SymbolNotFound = 3,
    DebugOffsetsMismatch = 4,
    UnsupportedVersion = 5,
    TlsDiscoveryFailed = 6,
    AllocFailed = 7,
    SignalHandler = 8,
    AlreadyRunning = 9,
}

// ── Logging ──────────────────────────────────────────────────────────────────

fn log_info(msg: &str) {
    if LOG_ENABLED.load(Ordering::Relaxed) {
        eprintln!("pyroscope_cpython: {}", msg);
    }
}

fn log_error(msg: &str) {
    if LOG_ENABLED.load(Ordering::Relaxed) {
        eprintln!("pyroscope_cpython ERROR: {}", msg);
    }
}

// ── Per-shard state ──────────────────────────────────────────────────────────

/// Per-shard mutable state protected by a spin::Mutex.
///
/// The signal handler `try_lock()`s a shard, unwinds into `frame_buffer`,
/// then writes the result into the bbqueue `producer`.
struct Shard {
    frame_buffer: [RawFrame; python_unwind::MAX_DEPTH],
    producer: FrameProducer<'static, RING_SIZE>,
}

// ── Handler state (shared between init + signal handler + reader) ────────────

/// Global profiler state accessed by the signal handler via `AtomicPtr`.
///
/// Allocated once at init time via `Box::into_raw`. Published to the handler
/// with `Release`; the handler loads with `Acquire`. Never deallocated.
struct HandlerState {
    debug_offsets: py313::_Py_DebugOffsets,
    tls_offset: u64,
    /// Expected type-object addresses for runtime type checking.
    type_addrs: python_unwind::TypeAddrs,
    /// Dynamically-sized shard array (length = num_shards).
    shards: Vec<notlibc::ShardMutex<Shard>>,
    /// Per-shard bbqueue consumers. Only accessed by the reader thread.
    /// Wrapped in UnsafeCell for interior mutability; safety is guaranteed
    /// by the single-reader-thread invariant.
    consumers: Vec<UnsafeCell<FrameConsumer<'static, RING_SIZE>>>,
    eventfd: notlibc::EventFd,
    samples_since_notify: AtomicU32,
    num_shards: usize,
    notify_interval: u32,
}

// SAFETY: HandlerState is initialized once and then only accessed via:
// - signal handler: loads AtomicPtr(Acquire), takes shard try_lock,
//   reads debug_offsets/tls_offset (immutable), writes to producer.
// - reader thread: takes shard lock, reads consumers (separate from handler).
// All accesses are properly synchronized via AtomicPtr + spin::Mutex.
unsafe impl Sync for HandlerState {}

static HANDLER_STATE: AtomicPtr<HandlerState> = AtomicPtr::new(core::ptr::null_mut());

// ── Signal handler ───────────────────────────────────────────────────────────

extern "C" fn on_sigprof(_sig: c_int, _info: *mut libc::siginfo_t, _ctx: *mut c_void) {
    // Step 1: Load global profiler state.
    let state_ptr = HANDLER_STATE.load(Ordering::Acquire);
    if state_ptr.is_null() {
        return;
    }
    let state = unsafe { &*state_ptr };

    // Step 2: Read FS base.
    let fs_base = match kindasafe::fs_0x0() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Step 3: Read tstate from TLS.
    let tstate_addr = fs_base.wrapping_add(state.tls_offset);
    let tstate = match kindasafe::u64(tstate_addr) {
        Ok(v) => v,
        Err(_) => return,
    };
    if tstate == 0 {
        return;
    }

    // Step 4: Select shard via gettid, try-lock with 3 fallback attempts.
    let tid = notlibc::gettid();
    let num_shards = state.num_shards;
    let base = tid as usize % num_shards;

    let mut guard = None;
    for attempt in 0..3 {
        let idx = (base + attempt) % num_shards;
        if let Some(g) = state.shards[idx].try_lock() {
            guard = Some(g);
            break;
        }
    }
    let mut guard = match guard {
        Some(g) => g,
        None => return, // all 3 shards contended — drop sample
    };

    // Step 5: Unwind Python stack into the shard's pre-allocated frame buffer.
    let depth = python_unwind::unwind(
        tstate,
        &state.debug_offsets,
        &state.type_addrs,
        &mut guard.frame_buffer,
    );
    if depth == 0 {
        return;
    }

    // Step 6: Write stack trace record into the shard's bbqueue producer.
    // Split the borrow: take a shared ref to the frame_buffer data, then
    // pass the producer as &mut. This is safe because write() only reads
    // from frames[..depth] and only writes to the producer.
    let shard = &mut *guard;
    sig_ring::write(&mut shard.producer, tid, &shard.frame_buffer, depth);

    // Step 7: Notify reader thread periodically.
    let total = state.samples_since_notify.fetch_add(1, Ordering::Relaxed);
    if total % state.notify_interval == 0 {
        state.eventfd.notify();
    }
}

// ── Reader thread ────────────────────────────────────────────────────────────

/// Reader thread entry point. Wakes on eventfd or 15s timeout, drains all
/// shard consumers, and debug-prints the received stacks.
fn reader_thread(state: &'static HandlerState) {
    // Set up epoll to wait on the eventfd.
    let mut event_set = match notlibc::EventSet::new() {
        Ok(es) => es,
        Err(_) => {
            log_error("reader: failed to create EventSet");
            return;
        }
    };
    if event_set.add(&state.eventfd).is_err() {
        log_error("reader: failed to add eventfd to EventSet");
        return;
    }

    log_info(&format!(
        "reader thread started, {} shards",
        state.num_shards
    ));

    loop {
        // Wait for eventfd notification or 15s timeout.
        let _ = event_set.wait(15_000);

        // Drain all shards.
        for shard_idx in 0..state.num_shards {
            // Lock the shard to ensure no signal handler is mid-write.
            let _shard_guard = state.shards[shard_idx].lock();

            // SAFETY: consumers are only accessed by the reader thread.
            let consumer = unsafe { &mut *state.consumers[shard_idx].get() };

            // Drain all available frames from this shard's consumer.
            while let Some(grant) = consumer.read() {
                if let Some(record) = sig_ring::parse_record(&grant) {
                    notlibc::debug::writes("reader: tid=");
                    notlibc::debug::write_hex(record.tid as usize);
                    notlibc::debug::writes(" depth=");
                    notlibc::debug::write_hex(record.depth as usize);
                    notlibc::debug::puts("");

                    for i in 0..record.depth as usize {
                        let frame = record.frame(i);
                        notlibc::debug::writes("  reader: [");
                        notlibc::debug::write_hex(i);
                        notlibc::debug::writes("] code=0x");
                        notlibc::debug::write_hex(frame.code_object as usize);
                        notlibc::debug::writes(" instr=0x");
                        notlibc::debug::write_hex(frame.instr_offset as usize);
                        notlibc::debug::puts("");
                    }
                }

                grant.release();
            }
        }
    }
}

// ── Public C API ─────────────────────────────────────────────────────────────

/// Start the CPython profiler.
///
/// Runs the full init sequence: kindasafe crash recovery, Python binary
/// discovery, ELF symbol resolution, version detection, debug offsets,
/// TLS offset discovery, ring buffer allocation, reader thread spawn,
/// then installs a SIGPROF handler + 10 ms timer.
///
/// Parameters:
/// - `app_name`: application name (NUL-terminated C string, or null).
/// - `server_url`: server URL (NUL-terminated C string, or null).
/// - `num_shards`: number of shards (0 = use default 16). Must be >= 1.
/// - `log_enabled`: if nonzero, print diagnostic messages to stderr.
///
/// Returns 0 on success, nonzero error code on failure:
/// - 1: kindasafe init failed
/// - 2: Python binary not found
/// - 3: ELF symbol not found
/// - 4: debug offsets validation failed
/// - 5: unsupported Python version
/// - 6: TLS discovery failed
/// - 7: memory allocation / resource creation failed
/// - 8: signal handler installation failed
/// - 9: profiler already running
///
/// # Safety
///
/// `app_name` and `server_url` must be valid pointers to NUL-terminated
/// C strings, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pyroscope_start(
    _app_name: *const c_char,
    _server_url: *const c_char,
    num_shards: c_int,
    log_enabled: c_int,
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
        return InitError::AlreadyRunning as c_int;
    }

    if log_enabled != 0 {
        LOG_ENABLED.store(true, Ordering::Release);
    }

    let num_shards = if num_shards <= 0 {
        DEFAULT_NUM_SHARDS
    } else {
        num_shards as usize
    };

    log_info(&format!(
        "configured num_shards={}, ring_size={}KiB",
        num_shards,
        RING_SIZE / 1024,
    ));

    match init_sequence(num_shards) {
        Ok(()) => 0,
        Err(code) => {
            log_error(&format!("init failed with code {}", code as c_int));
            LIFECYCLE.store(STATE_UNINITIALIZED, Ordering::Release);
            code as c_int
        }
    }
}

fn init_sequence(num_shards: usize) -> Result<(), InitError> {
    let notify_interval = sig_ring::DEFAULT_NOTIFY_INTERVAL;

    log_info(&format!(
        "starting init: num_shards={}, ring_size={}KiB, notify_interval={}",
        num_shards,
        RING_SIZE / 1024,
        notify_interval,
    ));

    // Step 1: Install kindasafe SIGSEGV/SIGBUS recovery.
    kindasafe_init::init().map_err(|_| {
        log_error("kindasafe_init failed");
        InitError::KindasafeInit
    })?;
    log_info("kindasafe_init ok");

    // Step 2: Find Python binary in /proc/self/maps.
    let binary = python_offsets::find_python_in_maps().map_err(|e| {
        log_error(&format!("find_python_in_maps: {:?}", e));
        map_init_error(&e)
    })?;
    log_info("found Python binary");

    // Step 3: Resolve _PyRuntime and Py_Version ELF symbols.
    let symbols = python_offsets::resolve_elf_symbols(&binary).map_err(|e| {
        log_error(&format!("resolve_elf_symbols: {:?}", e));
        map_init_error(&e)
    })?;
    log_info("resolved ELF symbols");

    // Step 4: Detect and validate Python version.
    let version = python_offsets::detect_version(symbols.py_version_addr).map_err(|e| {
        log_error(&format!("detect_version: {:?}", e));
        map_init_error(&e)
    })?;
    log_info(&format!("detected Python version: {:?}", version));

    // Read raw version hex (needed by read_debug_offsets).
    let version_hex = python_offsets::read_version_hex(symbols.py_version_addr).map_err(|e| {
        log_error(&format!("read_version_hex: {:?}", e));
        map_init_error(&e)
    })?;

    // Step 5: Read _Py_DebugOffsets from _PyRuntime.
    let debug_offsets =
        python_offsets::read_debug_offsets(symbols.py_runtime_addr, &version, version_hex)
            .map_err(|e| {
                log_error(&format!("read_debug_offsets: {:?}", e));
                map_init_error(&e)
            })?;
    log_info("read debug offsets");

    // Step 6: Discover TLS offset for _PyThreadState_GetCurrent.
    let tls_offset = python_offsets::find_tls_offset(&binary).map_err(|e| {
        log_error(&format!("find_tls_offset: {:?}", e));
        map_init_error(&e)
    })?;
    log_info(&format!("TLS offset: 0x{:x}", tls_offset));

    // Step 7: Allocate bbqueue buffers and split into producer/consumer pairs.
    let mut producers: Vec<Option<FrameProducer<'static, RING_SIZE>>> =
        (0..num_shards).map(|_| None).collect();
    let mut consumers: Vec<Option<FrameConsumer<'static, RING_SIZE>>> =
        (0..num_shards).map(|_| None).collect();

    for i in 0..num_shards {
        let bb = Box::new(bbqueue::BBBuffer::<RING_SIZE>::new());
        let bb: &'static bbqueue::BBBuffer<RING_SIZE> = Box::leak(bb);
        let (prod, cons) = bb.try_split_framed().map_err(|_| {
            log_error(&format!("bbqueue split failed for shard {}", i));
            InitError::AllocFailed
        })?;
        producers[i] = Some(prod);
        consumers[i] = Some(cons);
    }
    log_info(&format!("allocated {} ring buffers", num_shards));

    // Step 8: Create eventfd for reader thread notification.
    let eventfd = notlibc::EventFd::new().map_err(|_| {
        log_error("eventfd creation failed");
        InitError::AllocFailed
    })?;

    // Step 9: Build shard and consumer vecs.
    let empty_frame = RawFrame {
        code_object: 0,
        instr_offset: 0,
    };
    let shards: Vec<notlibc::ShardMutex<Shard>> = (0..num_shards)
        .map(|i| {
            notlibc::ShardMutex::new(Shard {
                frame_buffer: [empty_frame; python_unwind::MAX_DEPTH],
                producer: producers[i].take().unwrap(),
            })
        })
        .collect();

    let consumers: Vec<UnsafeCell<FrameConsumer<'static, RING_SIZE>>> = consumers
        .into_iter()
        .map(|c| UnsafeCell::new(c.unwrap()))
        .collect();

    // Step 10: Publish handler state.
    let type_addrs = python_unwind::TypeAddrs {
        code_type: symbols.py_code_type_addr,
    };
    let state = Box::new(HandlerState {
        debug_offsets,
        tls_offset,
        type_addrs,
        shards,
        consumers,
        eventfd,
        samples_since_notify: AtomicU32::new(0),
        num_shards,
        notify_interval,
    });
    let state: &'static HandlerState = unsafe { &*Box::into_raw(state) };
    HANDLER_STATE.store(
        state as *const HandlerState as *mut HandlerState,
        Ordering::Release,
    );

    // Step 11: Spawn reader thread.
    std::thread::Builder::new()
        .name("pyroscope-reader".into())
        .spawn(move || reader_thread(state))
        .map_err(|_| {
            log_error("failed to spawn reader thread");
            InitError::AllocFailed
        })?;

    // Steps 12+13: Install SIGPROF handler and start 10 ms ITIMER_PROF timer.
    sighandler::start(on_sigprof).map_err(|_| {
        log_error("signal handler installation failed");
        InitError::SignalHandler
    })?;

    log_info("init complete");
    notlibc::debug::puts("pyroscope_cpython: init complete");
    Ok(())
}

/// Map `python_offsets::InitError` variants to our `InitError` enum.
fn map_init_error(err: &python_offsets::InitError) -> InitError {
    match err {
        python_offsets::InitError::KindasafeInitFailed => InitError::KindasafeInit,
        python_offsets::InitError::PythonNotFound => InitError::PythonNotFound,
        python_offsets::InitError::Io => InitError::PythonNotFound,
        python_offsets::InitError::SymbolNotFound(_) => InitError::SymbolNotFound,
        python_offsets::InitError::ElfParse => InitError::SymbolNotFound,
        python_offsets::InitError::DebugOffsetsMismatch => InitError::DebugOffsetsMismatch,
        python_offsets::InitError::UnsupportedVersion => InitError::UnsupportedVersion,
        python_offsets::InitError::TlsDiscoveryFailed => InitError::TlsDiscoveryFailed,
    }
}
