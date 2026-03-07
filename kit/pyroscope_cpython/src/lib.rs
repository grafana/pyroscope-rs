use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use bbqueue::framed::{FrameConsumer, FrameProducer};
use python_offsets_types::py313;
use python_unwind::RawFrame;
use sig_ring::{NOTIFY_INTERVAL, NUM_SHARDS, RING_SIZE};

const STATE_UNINITIALIZED: u32 = 0;
const STATE_RUNNING: u32 = 1;

static LIFECYCLE: AtomicU32 = AtomicU32::new(STATE_UNINITIALIZED);

// ── Per-shard state ──────────────────────────────────────────────────────────

/// Per-shard mutable state protected by a spin::Mutex.
///
/// The signal handler `try_lock()`s a shard, unwinds into `frame_buffer`,
/// then writes the result into the bbqueue `producer`.
struct Shard {
    frame_buffer: [RawFrame; python_unwind::MAX_DEPTH],
    producer: FrameProducer<'static, RING_SIZE>,
}

// ── Handler state (shared between init + signal handler) ─────────────────────

/// Global profiler state accessed by the signal handler via `AtomicPtr`.
///
/// Allocated once at init time via `Box::into_raw`. Published to the handler
/// with `Release`; the handler loads with `Acquire`. Never deallocated.
struct HandlerState {
    debug_offsets: py313::_Py_DebugOffsets,
    tls_offset: u64,
    /// Expected type-object addresses for runtime type checking.
    type_addrs: python_unwind::TypeAddrs,
    shards: [notlibc::ShardMutex<Shard>; NUM_SHARDS],
    eventfd: notlibc::EventFd,
    samples_since_notify: AtomicU32,
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
    let base = tid as usize % NUM_SHARDS;

    let mut guard = None;
    for attempt in 0..3 {
        let idx = (base + attempt) % NUM_SHARDS;
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
    if total % NOTIFY_INTERVAL == 0 {
        state.eventfd.notify();
    }
}

// ── Reader thread ────────────────────────────────────────────────────────────

/// Reader thread entry point. Wakes on eventfd or 15s timeout, drains all
/// shard consumers, and debug-prints the received stacks.
fn reader_thread(
    state: &'static HandlerState,
    mut consumers: [FrameConsumer<'static, RING_SIZE>; NUM_SHARDS],
) {
    // Set up epoll to wait on the eventfd.
    let mut event_set = match notlibc::EventSet::new() {
        Ok(es) => es,
        Err(_) => return,
    };
    if event_set.add(&state.eventfd).is_err() {
        return;
    }

    loop {
        // Wait for eventfd notification or 15s timeout.
        let _ = event_set.wait(15_000);

        // Drain all shards.
        for (shard_idx, consumer) in consumers.iter_mut().enumerate() {
            // Lock the shard to ensure no signal handler is mid-write.
            let _guard = state.shards[shard_idx].lock();

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
/// Returns 0 on success, nonzero error code on failure.
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

    // Step 7: Allocate bbqueue buffers and split into producer/consumer pairs.
    let mut producers: [Option<FrameProducer<'static, RING_SIZE>>; NUM_SHARDS] =
        core::array::from_fn(|_| None);
    let mut consumers: [Option<FrameConsumer<'static, RING_SIZE>>; NUM_SHARDS] =
        core::array::from_fn(|_| None);

    for i in 0..NUM_SHARDS {
        let bb = Box::new(bbqueue::BBBuffer::<RING_SIZE>::new());
        let bb: &'static bbqueue::BBBuffer<RING_SIZE> = Box::leak(bb);
        let (prod, cons) = bb.try_split_framed().map_err(|_| 7)?;
        producers[i] = Some(prod);
        consumers[i] = Some(cons);
    }

    // Step 8: Create eventfd for reader thread notification.
    let eventfd = notlibc::EventFd::new().map_err(|_| 7)?;

    // Step 9: Build shard array.
    let empty_frame = RawFrame {
        code_object: 0,
        instr_offset: 0,
    };
    let shards: [notlibc::ShardMutex<Shard>; NUM_SHARDS] = core::array::from_fn(|i| {
        notlibc::ShardMutex::new(Shard {
            frame_buffer: [empty_frame; python_unwind::MAX_DEPTH],
            producer: producers[i].take().unwrap(),
        })
    });

    // Unwrap consumers into a fixed-size array.
    let consumers: [FrameConsumer<'static, RING_SIZE>; NUM_SHARDS] =
        core::array::from_fn(|i| consumers[i].take().unwrap());

    // Step 10: Publish handler state.
    let type_addrs = python_unwind::TypeAddrs {
        code_type: symbols.py_code_type_addr,
    };
    let state = Box::new(HandlerState {
        debug_offsets,
        tls_offset,
        type_addrs,
        shards,
        eventfd,
        samples_since_notify: AtomicU32::new(0),
    });
    let state: &'static HandlerState = unsafe { &*Box::into_raw(state) };
    HANDLER_STATE.store(
        state as *const HandlerState as *mut HandlerState,
        Ordering::Release,
    );

    // Step 11: Spawn reader thread.
    std::thread::Builder::new()
        .name("pyroscope-reader".into())
        .spawn(move || reader_thread(state, consumers))
        .map_err(|_| 7)?;

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
