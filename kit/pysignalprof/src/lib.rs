#![cfg(all(target_arch = "x86_64", target_os = "linux"))]

use core::cell::UnsafeCell;
use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use bbqueue::framed::{FrameConsumer, FrameProducer};
use python_offsets_types::py314;
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

/// Error codes returned by `start`.
#[repr(i32)]
#[derive(Copy, Clone, Debug)]
pub enum InitError {
    KindasafeInit = 1,
    PythonNotFound = 2,
    SymbolNotFound = 3,
    DebugOffsetsMismatch = 4,
    UnsupportedVersion = 5,
    AllocFailed = 7,
    SignalHandler = 8,
    AlreadyRunning = 9,
    KindasafeSanityCheck = 10,
}

// ── Logging ──────────────────────────────────────────────────────────────────

fn log_info(msg: &str) {
    if LOG_ENABLED.load(Ordering::Relaxed) {
        eprintln!("pysignalprof: {}", msg);
    }
}

fn log_error(msg: &str) {
    if LOG_ENABLED.load(Ordering::Relaxed) {
        eprintln!("pysignalprof ERROR: {}", msg);
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
    debug_offsets: py314::_Py_DebugOffsets,
    /// Function pointer to `_PyThreadState_GetCurrent()`, resolved at init time.
    get_tstate: fn() -> u64,
    /// Expected type-object addresses for runtime type checking.
    type_addrs: python_unwind::TypeAddrs,
    /// Asyncio module debug offsets (`None` if `_asyncio` was not loaded at init).
    /// Used by the reader thread for walking suspended async tasks (future work).
    #[allow(dead_code)]
    asyncio_offsets: Option<py314::Py_AsyncioModuleDebugOffsets>,
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
    app_name: String,
    server_url: Option<String>,
    tags: Vec<(String, String)>,
}

// SAFETY: HandlerState is initialized once and then only accessed via:
// - signal handler: loads AtomicPtr(Acquire), takes shard try_lock,
//   reads debug_offsets/get_tstate (immutable), writes to producer.
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

    // Step 2: Get current thread state by calling _PyThreadState_GetCurrent.
    let tstate = (state.get_tstate)();
    if tstate == 0 {
        return;
    }

    notlibc::debug::writes("sigprof: tstate=0x");
    notlibc::debug::write_hex(tstate as usize);
    notlibc::debug::puts("");

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
        notlibc::debug::writes("sigprof: unwind depth=0 tstate=0x");
        notlibc::debug::write_hex(tstate as usize);
        notlibc::debug::puts("");
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

// ── Unicode string reading ───────────────────────────────────────────────────

/// Read a Python unicode string object into `buf` and return a UTF-8 `&str`.
///
/// Handles all CPython compact string representations:
/// - ASCII (ascii=1): data at `obj + asciiobject_size`, null-terminated
/// - Non-ASCII with cached UTF-8: reads from the `utf8` pointer field
/// - UCS1/Latin-1 (kind=1, ascii=0): 1 byte/char, encoded to UTF-8
/// - UCS2 (kind=2): 2 bytes/char (little-endian u16), encoded to UTF-8
/// - UCS4 (kind=4): 4 bytes/char (little-endian u32), encoded to UTF-8
fn read_pyunicode<'a>(
    buf: &'a mut [u8],
    obj_ptr: u64,
    unicode_offsets: &py314::_Py_DebugOffsets__unicode_object,
    free_threaded: bool,
) -> Option<&'a str> {
    if obj_ptr == 0 {
        return None;
    }

    // Read state (u32 at the state offset within the unicode object).
    let state_raw = kindasafe::u64(obj_ptr + unicode_offsets.state).ok()? as u32;

    // Extract ascii and kind bits. The layout differs between standard and
    // free-threaded builds:
    //   Standard:      [interned:2][kind:3][compact:1][ascii:1][statically_allocated:1][pad:24]
    //   Free-threaded: [interned:8][kind:3][compact:1][ascii:1][statically_allocated:1][pad:18]
    let (ascii, kind) = if free_threaded {
        (((state_raw >> 12) & 1) != 0, (state_raw >> 8) & 0x7)
    } else {
        (((state_raw >> 6) & 1) != 0, (state_raw >> 2) & 0x7)
    };

    // ASCII fast path: data is null-terminated UTF-8 right after PyASCIIObject.
    if ascii {
        return kindasafe::str(buf, obj_ptr + unicode_offsets.asciiobject_size)
            .ok()
            .filter(|s| !s.is_empty());
    }

    // Non-ASCII: try the cached utf8 pointer in PyCompactUnicodeObject.
    // Layout: PyASCIIObject | utf8_length (8 bytes) | utf8 (8 bytes) | data...
    let utf8_ptr_addr = obj_ptr + unicode_offsets.asciiobject_size + 8;
    let utf8_ptr = kindasafe::u64(utf8_ptr_addr).unwrap_or(0);
    if utf8_ptr != 0 {
        // Read the cached UTF-8 representation directly.
        return kindasafe::str(buf, utf8_ptr).ok().filter(|s| !s.is_empty());
    }

    // No cached UTF-8: read raw UCS data and convert.
    let length = kindasafe::u64(obj_ptr + unicode_offsets.length).ok()? as usize;
    if length == 0 {
        return None;
    }

    // Data for compact non-ASCII strings starts after PyCompactUnicodeObject,
    // which is PyASCIIObject + 16 bytes (utf8_length + utf8 pointer).
    let data_addr = obj_ptr + unicode_offsets.asciiobject_size + 16;

    match kind {
        1 => read_ucs1_to_utf8(buf, data_addr, length),
        2 => read_ucs2_to_utf8(buf, data_addr, length),
        4 => read_ucs4_to_utf8(buf, data_addr, length),
        _ => None,
    }
}

/// Read UCS1 (Latin-1) data and encode to UTF-8.
fn read_ucs1_to_utf8(buf: &mut [u8], data_addr: u64, length: usize) -> Option<&str> {
    let max_read = length.min(128);
    let mut raw = [0u8; 128];
    kindasafe::slice(&mut raw[..max_read], data_addr).ok()?;

    let mut out = 0;
    for &byte in &raw[..max_read] {
        if let Some(c) = char::from_u32(byte as u32) {
            let len = c.len_utf8();
            if out + len > buf.len() {
                break;
            }
            c.encode_utf8(&mut buf[out..]);
            out += len;
        }
    }
    if out == 0 {
        return None;
    }
    core::str::from_utf8(&buf[..out]).ok()
}

/// Read UCS2 data (little-endian u16) and encode to UTF-8.
fn read_ucs2_to_utf8(buf: &mut [u8], data_addr: u64, length: usize) -> Option<&str> {
    let max_read = length.min(128);
    let byte_len = max_read * 2;
    let mut raw = [0u8; 256];
    kindasafe::slice(&mut raw[..byte_len], data_addr).ok()?;

    let mut out = 0;
    for i in 0..max_read {
        let cp = u16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]) as u32;
        if let Some(c) = char::from_u32(cp) {
            let len = c.len_utf8();
            if out + len > buf.len() {
                break;
            }
            c.encode_utf8(&mut buf[out..]);
            out += len;
        }
    }
    if out == 0 {
        return None;
    }
    core::str::from_utf8(&buf[..out]).ok()
}

/// Read UCS4 data (little-endian u32) and encode to UTF-8.
fn read_ucs4_to_utf8(buf: &mut [u8], data_addr: u64, length: usize) -> Option<&str> {
    let max_read = length.min(64);
    let byte_len = max_read * 4;
    let mut raw = [0u8; 256];
    kindasafe::slice(&mut raw[..byte_len], data_addr).ok()?;

    let mut out = 0;
    for i in 0..max_read {
        let cp = u32::from_le_bytes([raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]]);
        if let Some(c) = char::from_u32(cp) {
            let len = c.len_utf8();
            if out + len > buf.len() {
                break;
            }
            c.encode_utf8(&mut buf[out..]);
            out += len;
        }
    }
    if out == 0 {
        return None;
    }
    core::str::from_utf8(&buf[..out]).ok()
}

// ── Symbolization helper ─────────────────────────────────────────────────────

/// Resolve the function name for a code object via `co_qualname` (with
/// `co_name` fallback). Returns an owned `String`, or `"<unknown>"` if
/// resolution fails.
fn resolve_function_name(code_object: u64, offsets: &py314::_Py_DebugOffsets) -> String {
    let mut name_buf = [0u8; 256];
    let free_threaded = offsets.free_threaded != 0;

    // Try co_qualname first.
    if let Ok(qualname_ptr) = kindasafe::u64(code_object + offsets.code_object.qualname)
        && let Some(name) = read_pyunicode(
            &mut name_buf,
            qualname_ptr,
            &offsets.unicode_object,
            free_threaded,
        )
    {
        return name.to_owned();
    }

    // Fallback to co_name.
    if let Ok(name_ptr) = kindasafe::u64(code_object + offsets.code_object.name)
        && let Some(name) = read_pyunicode(
            &mut name_buf,
            name_ptr,
            &offsets.unicode_object,
            free_threaded,
        )
    {
        return name.to_owned();
    }

    "<unknown>".to_owned()
}

// ── Reader thread ────────────────────────────────────────────────────────────

/// Reader thread entry point. Wakes on eventfd or timeout, drains all shard
/// consumers, symbolizes and feeds samples directly into the ProfileBuilder.
/// Every 15 seconds the builder is encoded to pprof and optionally sent to
/// Pyroscope, then reset for the next window.
fn reader_thread(state: &'static HandlerState) {
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

    let flush_interval = Duration::from_secs(15);
    let period: i64 = 10_000_000; // 10 ms
    let mut last_flush = Instant::now();
    let mut builder = pprof_enc::ProfileBuilder::new(0, flush_interval.as_nanos() as i64, period);
    // Cache: code_object address → resolved function name.
    let mut symbol_cache: HashMap<u64, String> = HashMap::new();

    loop {
        // Phase 1: Wait with dynamic timeout so flush happens on time.
        let remaining = flush_interval.saturating_sub(last_flush.elapsed());
        let timeout_ms = remaining.as_millis() as i32;
        let _ = event_set.wait(timeout_ms);

        // Phase 2: Drain all shards, symbolize, feed into builder.
        let offsets = &state.debug_offsets;
        let log = LOG_ENABLED.load(Ordering::Relaxed);

        for shard_idx in 0..state.num_shards {
            let _shard_guard = state.shards[shard_idx].lock();
            let consumer = unsafe { &mut *state.consumers[shard_idx].get() };

            while let Some(grant) = consumer.read() {
                if let Some(record) = sig_ring::parse_record(&grant) {
                    let depth = record.depth as usize;

                    // Ensure all code objects are in the cache (mutable pass).
                    for i in 0..depth {
                        let raw = record.frame(i);
                        symbol_cache
                            .entry(raw.code_object)
                            .or_insert_with(|| resolve_function_name(raw.code_object, offsets));
                    }

                    // Build frames from cached names (immutable pass).
                    let frames: Vec<pprof_enc::Frame<'_>> = (0..depth)
                        .map(|i| {
                            let raw = record.frame(i);
                            pprof_enc::Frame {
                                function_name: symbol_cache[&raw.code_object].as_str(),
                                filename: "",
                                first_line: 0,
                            }
                        })
                        .collect();

                    if log {
                        let names: Vec<&str> = frames.iter().map(|f| f.function_name).collect();
                        eprintln!(
                            "pysignalprof: reader: tid=0x{:x} depth={} [{}]",
                            record.tid,
                            depth,
                            names.join(" < "),
                        );
                    }

                    builder.add_sample(&frames, 1);
                }
                grant.release();
            }
        }

        // Phase 3: Flush when 15 seconds have elapsed.
        if last_flush.elapsed() >= flush_interval {
            flush_pprof(state, &mut builder);
            symbol_cache.clear();
            last_flush = Instant::now();
        }
    }
}

/// Encode the accumulated profile, optionally send it, then reset the builder.
fn flush_pprof(state: &'static HandlerState, builder: &mut pprof_enc::ProfileBuilder) {
    if builder.is_empty() {
        log_info("flush: no samples, skipping");
        return;
    }

    let num_stacks = builder.len();
    let pprof = builder.encode();
    log_info(&format!(
        "flush: {} unique stacks, pprof {} bytes",
        num_stacks,
        pprof.len(),
    ));

    if let Some(ref url) = state.server_url {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let from_secs = now_secs.saturating_sub(15);
        let tag_refs: Vec<(&str, &str)> = state
            .tags
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        if let Err(e) =
            pyroscope_ingest::send(url, &state.app_name, &tag_refs, &pprof, from_secs, now_secs)
        {
            log_error(&format!("ingest send failed: {}", e));
        }
    }

    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64;
    builder.reset(now_nanos, 15_000_000_000);
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Start the CPython profiler.
///
/// Runs the full init sequence: kindasafe crash recovery, Python binary
/// discovery, ELF symbol resolution, version detection, debug offsets,
/// TLS offset discovery, ring buffer allocation, reader thread spawn,
/// then installs a SIGPROF handler + 10 ms timer.
///
/// Parameters:
/// - `app_name`: application name (empty string if not specified).
/// - `server_url`: server URL (`None` to skip ingestion).
/// - `num_shards`: number of shards (0 = use default 16).
/// - `log_enabled`: if true, print diagnostic messages to stderr.
/// - `tags`: static key-value labels to attach to ingested profiles.
///
/// Returns `Ok(())` on success, `Err(InitError)` on failure.
pub fn start(
    app_name: String,
    server_url: Option<String>,
    num_shards: usize,
    log_enabled: bool,
    tags: Vec<(String, String)>,
) -> Result<(), InitError> {
    if LIFECYCLE
        .compare_exchange(
            STATE_UNINITIALIZED,
            STATE_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return Err(InitError::AlreadyRunning);
    }

    if log_enabled {
        LOG_ENABLED.store(true, Ordering::Release);
    }

    let num_shards = if num_shards == 0 {
        DEFAULT_NUM_SHARDS
    } else {
        num_shards
    };

    log_info(&format!(
        "configured num_shards={}, ring_size={}KiB",
        num_shards,
        RING_SIZE / 1024,
    ));

    match init_sequence(num_shards, app_name, server_url, tags) {
        Ok(()) => Ok(()),
        Err(code) => {
            log_error(&format!("init failed with code {}", code as c_int));
            LIFECYCLE.store(STATE_UNINITIALIZED, Ordering::Release);
            Err(code)
        }
    }
}

fn init_sequence(
    num_shards: usize,
    app_name: String,
    server_url: Option<String>,
    tags: Vec<(String, String)>,
) -> Result<(), InitError> {
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

    // Step 5b: Try to read asyncio module debug offsets (non-fatal).
    let asyncio_offsets = match python_offsets::find_asyncio_in_maps() {
        Ok(asyncio_binary) => match python_offsets::resolve_asyncio_debug_symbol(&asyncio_binary) {
            Ok(addr) => match python_offsets::read_asyncio_debug_offsets(addr) {
                Ok(offsets) => {
                    log_info("read asyncio debug offsets");
                    Some(offsets)
                }
                Err(e) => {
                    log_info(&format!("asyncio debug offsets read failed: {:?}", e));
                    None
                }
            },
            Err(e) => {
                log_info(&format!("asyncio debug symbol not found: {:?}", e));
                None
            }
        },
        Err(_) => {
            log_info("_asyncio module not loaded yet");
            None
        }
    };

    // Step 6: Resolve _PyThreadState_GetCurrent as a callable function pointer.
    let get_tstate: fn() -> u64 = unsafe { core::mem::transmute(symbols.get_tstate_addr as usize) };
    log_info(&format!(
        "_PyThreadState_GetCurrent at 0x{:x}",
        symbols.get_tstate_addr
    ));

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
        get_tstate,
        type_addrs,
        asyncio_offsets,
        shards,
        consumers,
        eventfd,
        samples_since_notify: AtomicU32::new(0),
        num_shards,
        notify_interval,
        app_name,
        server_url,
        tags,
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

    // Step 12: Install SIGPROF handler (but don't start the timer yet).
    unsafe {
        sighandler::register_sigaction(on_sigprof).map_err(|_| {
            log_error("signal handler installation failed");
            InitError::SignalHandler
        })?;
    }

    // Step 12b: Verify kindasafe crash recovery works under the SIGPROF
    // handler's signal mask (sa_mask). This catches misconfigurations like
    // SIGSEGV/SIGBUS being blocked during SIGPROF.
    kindasafe_init::sanity_check().map_err(|_| {
        log_error("kindasafe sanity check failed — crash recovery is not working");
        InitError::KindasafeSanityCheck
    })?;
    log_info("kindasafe sanity check passed");

    // Step 13: Start 10 ms ITIMER_PROF timer.
    unsafe {
        sighandler::start_timer().map_err(|_| {
            log_error("setitimer failed");
            InitError::SignalHandler
        })?;
    }

    log_info("init complete");
    notlibc::debug::puts("pysignalprof: init complete");
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
    }
}
