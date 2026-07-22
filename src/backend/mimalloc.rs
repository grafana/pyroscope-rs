//! Allocation profiling backend for applications that install
//! [`SamplingMiMalloc`] as their global allocator.
//!
//! The allocator hot path records sampled allocation events into fixed TLS
//! rings with best-effort, non-blocking handoff to sharded global buffers.
//! `report()` is the non-hot path: it may block briefly to drain registered TLS
//! rings, aggregate samples, resolve symbols, and encode memory pprof data.

use std::{
    alloc::{GlobalAlloc, Layout},
    cell::Cell,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use once_cell::sync::Lazy;

use crate::{
    backend::{Backend, BackendImpl, BackendUninitialized, ReportBatch, ReportData, ThreadTag},
    encode::memory_pprof::{self, AllocationSample},
    error::{PyroscopeError, Result},
};

const LOG_TAG: &str = "Pyroscope::Mimalloc";
const DEFAULT_SAMPLE_INTERVAL_BYTES: u64 = 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 64;
const DEFAULT_RING_CAPACITY: usize = 512;
const DEFAULT_REPORT_DRAIN_LIMIT: usize = 1_000_000;
const MAX_CAPTURE_DEPTH: usize = 64;
const TLS_SAMPLE_RING_CAPACITY: usize = 64;
const RECORDED_SAMPLE_SHARD_COUNT: usize = 8;
const SYNTHETIC_FRAME: &str = "[mimalloc] sampled allocations (stack capture pending)";
const RNG_INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;
const RNG_INITIAL_STATE: u64 = 0xa076_1d64_78bd_642f;
// Keep worst-case work bounded inside the allocator hook. Very large
// allocations keep the first intervals stochastic, then fall back to a
// deterministic approximation instead of looping once per sampled interval.
const MAX_POISSON_INTERVALS_PER_ALLOCATION: u64 = 64;

static RECORDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static ALLOCATOR_SEEN: AtomicBool = AtomicBool::new(false);
static SAMPLE_INTERVAL_BYTES: AtomicU64 = AtomicU64::new(DEFAULT_SAMPLE_INTERVAL_BYTES);
static SAMPLING_CONFIG_GENERATION: AtomicU64 = AtomicU64::new(0);
static SAMPLING_RNG_SEED: AtomicU64 = AtomicU64::new(RNG_INITIAL_STATE);
static FLUSH_REQUEST_GENERATION: AtomicU64 = AtomicU64::new(0);
static MAX_RECORDED_SAMPLES: AtomicUsize = AtomicUsize::new(DEFAULT_RING_CAPACITY);
static GLOBAL_BUFFERED_SAMPLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static NEXT_RECORDED_SAMPLE_SHARD: AtomicUsize = AtomicUsize::new(0);
static RECORDED_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);
static FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);
static FLUSHED_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);
static DROPPED_SAMPLES: AtomicU64 = AtomicU64::new(0);
static LAST_PPROF_ENCODE_ELAPSED_MICROS: AtomicU64 = AtomicU64::new(0);

static RECORDED_SAMPLE_SHARDS: Lazy<Vec<Mutex<Vec<RecordedAllocationSample>>>> = Lazy::new(|| {
    (0..RECORDED_SAMPLE_SHARD_COUNT)
        .map(|_| Mutex::new(Vec::new()))
        .collect()
});
static TLS_SAMPLE_BUFFER_REGISTRY: Lazy<Mutex<TlsSampleBufferRegistry>> =
    Lazy::new(|| Mutex::new(TlsSampleBufferRegistry::new()));

#[derive(Debug, Copy, Clone)]
struct SamplerState {
    in_profiler: bool,
    profiler_suppressed: bool,
    remaining_bytes: u64,
    remaining_config_generation: u64,
    rng_state: u64,
    flush_generation: u64,
}

impl SamplerState {
    const fn new() -> Self {
        Self {
            in_profiler: false,
            profiler_suppressed: false,
            remaining_bytes: DEFAULT_SAMPLE_INTERVAL_BYTES,
            remaining_config_generation: 0,
            rng_state: 0,
            flush_generation: 0,
        }
    }
}

thread_local! {
    // Keep all allocation-hook state in one TLS cell so the hot path pays one
    // state lookup instead of one lookup per guard, sampler, RNG, and flush flag.
    static SAMPLER_STATE: Cell<SamplerState> = const { Cell::new(SamplerState::new()) };
    static TLS_SAMPLE_BUFFER: RegisteredTlsSampleBuffer = RegisteredTlsSampleBuffer::new();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct StackKey {
    frames: [usize; MAX_CAPTURE_DEPTH],
    depth: usize,
}

impl StackKey {
    fn capture(max_depth: usize) -> Self {
        let mut key = Self {
            frames: [0; MAX_CAPTURE_DEPTH],
            depth: 0,
        };
        let max_depth = max_depth.min(MAX_CAPTURE_DEPTH);

        backtrace::trace(|frame| {
            if key.depth >= max_depth {
                return false;
            }
            key.frames[key.depth] = frame.ip() as usize;
            key.depth += 1;
            true
        });

        key
    }

    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.frames[..self.depth].iter().copied()
    }
}

#[derive(Debug, Copy, Clone)]
struct RecordedAllocationSample {
    stack: StackKey,
    weighted_objects: u64,
    weighted_bytes: u64,
}

#[derive(Debug)]
struct TlsSampleBuffer {
    samples: [Option<RecordedAllocationSample>; TLS_SAMPLE_RING_CAPACITY],
    len: usize,
}

impl TlsSampleBuffer {
    const fn new() -> Self {
        Self {
            samples: [None; TLS_SAMPLE_RING_CAPACITY],
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn is_full(&self) -> bool {
        self.len == TLS_SAMPLE_RING_CAPACITY
    }

    fn push(&mut self, sample: RecordedAllocationSample) -> bool {
        if self.is_full() {
            return false;
        }

        self.samples[self.len] = Some(sample);
        self.len += 1;
        true
    }

    fn clear(&mut self) {
        for sample in &mut self.samples[..self.len] {
            *sample = None;
        }
        self.len = 0;
    }

    fn drain_into(&mut self, out: &mut Vec<RecordedAllocationSample>, limit: usize) -> usize {
        let drain_len = self.len.min(limit);
        for index in 0..drain_len {
            if let Some(sample) = self.samples[index].take() {
                out.push(sample);
            }
        }

        let remaining = self.len - drain_len;
        for index in 0..remaining {
            self.samples[index] = self.samples[drain_len + index].take();
        }
        for index in remaining..self.len {
            self.samples[index] = None;
        }
        self.len = remaining;

        drain_len
    }
}

#[derive(Debug)]
struct RegisteredTlsSampleBuffer {
    id: Cell<Option<usize>>,
    buffer: Arc<Mutex<TlsSampleBuffer>>,
}

impl RegisteredTlsSampleBuffer {
    fn new() -> Self {
        let buffer = Arc::new(Mutex::new(TlsSampleBuffer::new()));
        let id = register_tls_sample_buffer(buffer.clone());
        Self {
            id: Cell::new(id),
            buffer,
        }
    }

    fn ensure_registered(&self) {
        if self.id.get().is_none() {
            self.id.set(register_tls_sample_buffer(self.buffer.clone()));
        }
    }

    fn try_lock(&self) -> Option<std::sync::MutexGuard<'_, TlsSampleBuffer>> {
        self.ensure_registered();
        self.buffer.try_lock().ok()
    }
}

impl Drop for RegisteredTlsSampleBuffer {
    fn drop(&mut self) {
        with_profiler_suppressed(|| {
            if RECORDER_ACTIVE.load(Ordering::Acquire) {
                if let Some(mut buffer) = self.try_lock() {
                    flush_tls_samples_for_report(&mut buffer);
                }
            }
            // Deregister only after the final handoff attempt so a concurrent
            // report can still see this thread's ring until thread teardown.
            if let Some(id) = self.id.get() {
                deregister_tls_sample_buffer(id);
            }
        });
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct AggregatedAllocationSample {
    alloc_objects: u64,
    alloc_space: u64,
}

/// Configuration for the mimalloc allocation memory profiling backend.
///
/// The backend records sampled allocation call stacks and reports memory pprof
/// data with `alloc_objects/count` and `alloc_space/bytes` sample types. It does
/// not track live allocations or emit `inuse_*` samples. Samples whose frames
/// cannot be resolved are grouped under a synthetic fallback frame.
///
/// # Examples
///
/// ```rust
/// use pyroscope::backend::mimalloc::MimallocConfig;
///
/// let config = MimallocConfig {
///     sample_interval_bytes: 512 * 1024,
///     max_depth: 48,
///     ..MimallocConfig::default()
/// };
///
/// assert_eq!(config.sample_interval_bytes, 512 * 1024);
/// assert_eq!(config.max_depth, 48);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MimallocConfig {
    /// Average number of allocated bytes between sampled allocation events.
    ///
    /// The sampler uses a byte-based Poisson process, so this value is the mean
    /// interval rather than a fixed every-N-bytes trigger. Lower values increase
    /// profile detail and hot-path overhead.
    pub sample_interval_bytes: u64,
    /// Maximum number of stack frames captured for each sampled allocation.
    pub max_depth: usize,
    /// Maximum number of samples retained in the global recorder between reports.
    ///
    /// If the recorder is full or contended, new samples are dropped rather than
    /// blocking the allocator hot path.
    pub ring_capacity: usize,
    /// Maximum number of global samples drained by one `report()` call.
    ///
    /// A bounded drain keeps large bursts from making a single report interval do
    /// unbounded aggregation and pprof encoding work.
    pub report_drain_limit: usize,
}

/// Runtime counters for the mimalloc memory profiling backend.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MimallocStats {
    /// Number of samples accepted into the recorder since backend initialization.
    pub recorded_samples: u64,
    /// Number of successful TLS-to-global sample flushes since backend initialization.
    pub flushes: u64,
    /// Number of samples moved from TLS rings into the global buffer.
    pub flushed_samples: u64,
    /// Number of sample records dropped because the recorder was full or locked.
    pub dropped_samples: u64,
    /// Number of samples currently buffered for the next report, if the buffer lock is available.
    pub buffered_samples: Option<usize>,
    /// Duration of the most recent pprof encoding step in microseconds.
    pub last_pprof_encode_elapsed_micros: u64,
}

/// Return current mimalloc backend recorder counters.
///
/// This is mainly intended for tests, diagnostics, and benchmark reports.
pub fn mimalloc_stats() -> MimallocStats {
    let tls_buffered_samples = registered_tls_buffered_samples();

    MimallocStats {
        recorded_samples: RECORDED_SAMPLE_COUNT.load(Ordering::Relaxed),
        flushes: FLUSH_COUNT.load(Ordering::Relaxed),
        flushed_samples: FLUSHED_SAMPLE_COUNT.load(Ordering::Relaxed),
        dropped_samples: DROPPED_SAMPLES.load(Ordering::Relaxed),
        buffered_samples: tls_buffered_samples.map(|tls| {
            GLOBAL_BUFFERED_SAMPLE_COUNT
                .load(Ordering::Relaxed)
                .saturating_add(tls)
        }),
        last_pprof_encode_elapsed_micros: LAST_PPROF_ENCODE_ELAPSED_MICROS.load(Ordering::Relaxed),
    }
}

impl Default for MimallocConfig {
    fn default() -> Self {
        Self {
            sample_interval_bytes: DEFAULT_SAMPLE_INTERVAL_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
            ring_capacity: DEFAULT_RING_CAPACITY,
            report_drain_limit: DEFAULT_REPORT_DRAIN_LIMIT,
        }
    }
}

impl MimallocConfig {
    fn validate(&self) -> Result<()> {
        if self.sample_interval_bytes == 0 {
            return Err(PyroscopeError::new(
                "mimalloc: sample_interval_bytes must be greater than zero",
            ));
        }
        if self.max_depth == 0 {
            return Err(PyroscopeError::new(
                "mimalloc: max_depth must be greater than zero",
            ));
        }
        if self.ring_capacity == 0 {
            return Err(PyroscopeError::new(
                "mimalloc: ring_capacity must be greater than zero",
            ));
        }
        if self.report_drain_limit == 0 {
            return Err(PyroscopeError::new(
                "mimalloc: report_drain_limit must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// A mimalloc global allocator wrapper that records allocation samples.
///
/// Use this type as the application's global allocator when enabling
/// `backend-mimalloc`:
///
/// ```rust
/// use pyroscope::backend::mimalloc::SamplingMiMalloc;
///
/// #[global_allocator]
/// static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();
/// ```
///
/// The backend cannot record allocation call stacks when an application uses
/// `mimalloc::MiMalloc` directly.
pub struct SamplingMiMalloc {
    inner: mimalloc::MiMalloc,
}

impl SamplingMiMalloc {
    /// Create a `SamplingMiMalloc` allocator.
    ///
    /// The allocator is always safe to install, but it only records samples while
    /// a `backend-mimalloc` backend is initialized.
    pub const fn new() -> Self {
        Self {
            inner: mimalloc::MiMalloc,
        }
    }
}

impl Default for SamplingMiMalloc {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for SamplingMiMalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        mark_allocator_seen();
        // SAFETY: `SamplingMiMalloc` preserves the caller's `GlobalAlloc`
        // contract and forwards the exact layout to the wrapped mimalloc
        // allocator.
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            record_allocation(layout.size() as u64);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        mark_allocator_seen();
        // SAFETY: The caller provided a valid `GlobalAlloc` layout; this wrapper
        // only forwards it to mimalloc and records after successful allocation.
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record_allocation(layout.size() as u64);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Deallocation is forwarded unchanged; callers must pass a
        // pointer and layout that satisfy the `GlobalAlloc::dealloc` contract.
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        mark_allocator_seen();
        // SAFETY: Reallocation is forwarded unchanged to mimalloc with the
        // caller-provided pointer, old layout, and requested new size.
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let recorded_size = realloc_recorded_size(ptr, new_ptr, layout.size(), new_size) as u64;
            record_allocation(recorded_size);
        }
        new_ptr
    }
}

/// Create a mimalloc allocation memory profiling backend.
///
/// The returned backend should be passed to `PyroscopeAgentBuilder::new`, and
/// the process must install [`SamplingMiMalloc`] as its global allocator.
///
/// # Examples
///
/// ```no_run
/// use pyroscope::backend::mimalloc::{
///     mimalloc_backend, MimallocConfig, SamplingMiMalloc,
/// };
/// use pyroscope::pyroscope::PyroscopeAgentBuilder;
///
/// #[global_allocator]
/// static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let agent = PyroscopeAgentBuilder::new(
///     "http://localhost:4040",
///     "example.mimalloc",
///     100,
///     "pyroscope-rs",
///     env!("CARGO_PKG_VERSION"),
///     mimalloc_backend(MimallocConfig::default()),
/// )
/// .build()?;
/// # let _ = agent;
/// # Ok(())
/// # }
/// ```
pub fn mimalloc_backend(config: MimallocConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(Mimalloc::new(config)))
}

#[derive(Debug)]
struct Mimalloc {
    config: MimallocConfig,
    last_report: Option<Instant>,
}

impl Mimalloc {
    fn new(config: MimallocConfig) -> Self {
        Self {
            config,
            last_report: None,
        }
    }
}

impl Backend for Mimalloc {
    fn initialize(&mut self) -> Result<()> {
        self.config.validate()?;
        SAMPLE_INTERVAL_BYTES.store(self.config.sample_interval_bytes, Ordering::Relaxed);
        SAMPLING_CONFIG_GENERATION.fetch_add(1, Ordering::Relaxed);
        NEXT_RECORDED_SAMPLE_SHARD.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(self.config.ring_capacity, Ordering::Relaxed);
        RECORDED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        LAST_PPROF_ENCODE_ELAPSED_MICROS.store(0, Ordering::Relaxed);
        prepare_sample_buffer(self.config.ring_capacity);
        // A backend can be stopped and started again while worker threads keep
        // their TLS rings alive. Clear registered rings at the session boundary
        // so old samples cannot be flushed into the next profile interval.
        clear_registered_tls_samples();
        reset_current_thread_sample_buffer();
        warm_backtrace();
        RECORDER_ACTIVE.store(true, Ordering::Release);
        self.last_report = Some(Instant::now());

        if !ALLOCATOR_SEEN.load(Ordering::Relaxed) {
            log::warn!(
                target: LOG_TAG,
                "SamplingMiMalloc has not observed allocations yet; ensure it is configured as #[global_allocator]"
            );
        }

        log::info!(target: LOG_TAG, "Mimalloc profiling backend initialized");
        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        RECORDER_ACTIVE.store(false, Ordering::Release);
        // Stop new sampling first, then clear any registered TLS rings that
        // might otherwise survive until a later backend initialization.
        with_profiler_suppressed(clear_registered_tls_samples);
        log::trace!(target: LOG_TAG, "Shutting down mimalloc backend");
        Ok(())
    }

    fn report(&mut self) -> Result<ReportBatch> {
        // Reporting allocates for registry snapshots, aggregation, symbol
        // resolution, and pprof encoding. Suppressing this thread avoids
        // profiler self-sampling without disabling worker-thread sampling.
        with_profiler_suppressed(|| {
            let now = Instant::now();
            let duration_nanos = self
                .last_report
                .replace(now)
                .map(|last_report| duration_to_i64_nanos(now.duration_since(last_report)))
                .unwrap_or_default();

            request_tls_sample_flush();
            flush_registered_tls_samples();
            let recorded = drain_recorded_samples(self.config.report_drain_limit);
            let recorded_count = recorded.len();
            let dropped_count = DROPPED_SAMPLES.load(Ordering::Relaxed);
            if dropped_count > 0 {
                log::debug!(
                    target: LOG_TAG,
                    "Mimalloc report drained {recorded_count} samples; {dropped_count} samples have been dropped since initialization"
                );
            }

            let samples = build_allocation_samples(recorded, self.config.max_depth);

            let encode_start = Instant::now();
            let pprof_data = memory_pprof::encode_allocation_profile(
                &samples,
                self.config.sample_interval_bytes,
                duration_nanos,
            );
            LAST_PPROF_ENCODE_ELAPSED_MICROS.store(
                duration_to_u64_micros(encode_start.elapsed()),
                Ordering::Relaxed,
            );

            Ok(ReportBatch {
                profile_type: "memory".into(),
                data: ReportData::RawPprof(pprof_data),
            })
        })
    }

    fn add_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }

    fn remove_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }
}

fn mark_allocator_seen() {
    if !ALLOCATOR_SEEN.load(Ordering::Relaxed) {
        ALLOCATOR_SEEN.store(true, Ordering::Relaxed);
    }
}

fn with_profiler_suppressed<R>(f: impl FnOnce() -> R) -> R {
    struct SuppressionGuard<'a> {
        sampler: &'a Cell<SamplerState>,
        previous: bool,
    }

    impl Drop for SuppressionGuard<'_> {
        fn drop(&mut self) {
            let mut state = self.sampler.get();
            state.profiler_suppressed = self.previous;
            self.sampler.set(state);
        }
    }

    let mut f = Some(f);
    match SAMPLER_STATE.try_with(|sampler| {
        let mut state = sampler.get();
        let previous = state.profiler_suppressed;
        state.profiler_suppressed = true;
        sampler.set(state);

        let _guard = SuppressionGuard { sampler, previous };
        f.take().expect("suppression closure was already taken")()
    }) {
        Ok(result) => result,
        Err(_) => f
            .take()
            .expect("suppression fallback closure was already taken")(),
    }
}

fn realloc_recorded_size(
    old_ptr: *mut u8,
    new_ptr: *mut u8,
    old_size: usize,
    new_size: usize,
) -> usize {
    if old_ptr == new_ptr {
        new_size.saturating_sub(old_size)
    } else {
        new_size
    }
}

fn record_allocation(size: u64) {
    if size == 0 || !RECORDER_ACTIVE.load(Ordering::Acquire) {
        return;
    }

    struct ProfilerReentryGuard<'a> {
        sampler: &'a Cell<SamplerState>,
    }

    impl Drop for ProfilerReentryGuard<'_> {
        fn drop(&mut self) {
            let mut state = self.sampler.get();
            state.in_profiler = false;
            self.sampler.set(state);
        }
    }

    let _ = SAMPLER_STATE.try_with(|sampler| {
        let mut state = sampler.get();
        if state.profiler_suppressed || state.in_profiler {
            return;
        }

        state.in_profiler = true;
        sampler.set(state);
        let _guard = ProfilerReentryGuard { sampler };

        let mut state = sampler.get();
        flush_requested_tls_samples_with_state(&mut state);

        let interval = SAMPLE_INTERVAL_BYTES.load(Ordering::Relaxed).max(1);
        let generation = SAMPLING_CONFIG_GENERATION.load(Ordering::Relaxed);
        let mut current = state.remaining_bytes;
        if state.remaining_config_generation != generation || current == 0 {
            clear_current_thread_samples();
            state.rng_state = next_thread_rng_seed();
            current = next_poisson_interval(interval, &mut state.rng_state);
            state.remaining_config_generation = generation;
        }

        if size < current {
            state.remaining_bytes = current - size;
            sampler.set(state);
        } else {
            let weight = calculate_sample_weight(size, current, interval, &mut state.rng_state);
            state.remaining_bytes = weight.next_remaining;
            sampler.set(state);
            record_sample(weight);
        }
    });
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SampleWeight {
    weighted_objects: u64,
    weighted_bytes: u64,
    next_remaining: u64,
}

fn calculate_sample_weight(
    size: u64,
    current: u64,
    sample_interval: u64,
    rng_state: &mut u64,
) -> SampleWeight {
    let sample_interval = sample_interval.max(1);
    let mut remaining_bytes = size.saturating_sub(current.max(1));
    let mut crossed_intervals = 1_u64;
    let mut next_remaining = next_poisson_interval(sample_interval, rng_state);

    while remaining_bytes >= next_remaining
        && crossed_intervals < MAX_POISSON_INTERVALS_PER_ALLOCATION
    {
        remaining_bytes -= next_remaining;
        crossed_intervals = crossed_intervals.saturating_add(1);
        next_remaining = next_poisson_interval(sample_interval, rng_state);
    }

    if remaining_bytes >= next_remaining {
        remaining_bytes -= next_remaining;
        crossed_intervals = crossed_intervals.saturating_add(1);
        // Past the bounded stochastic prefix, approximate the rest of an
        // unusually large allocation with fixed-size intervals. This preserves
        // proportional weighting while keeping hook latency predictable.
        let deterministic_intervals = remaining_bytes / sample_interval;
        crossed_intervals = crossed_intervals.saturating_add(deterministic_intervals);
        let bytes_into_next_interval = remaining_bytes % sample_interval;
        next_remaining = if bytes_into_next_interval == 0 {
            sample_interval
        } else {
            sample_interval - bytes_into_next_interval
        };
    } else {
        next_remaining -= remaining_bytes;
    }

    let weighted_bytes = crossed_intervals.saturating_mul(sample_interval);
    let weighted_objects = weighted_bytes.checked_div(size).unwrap_or_default().max(1);

    SampleWeight {
        weighted_objects,
        weighted_bytes,
        next_remaining,
    }
}

fn next_poisson_interval(sample_interval: u64, rng_state: &mut u64) -> u64 {
    let sample_interval = sample_interval.max(1);
    let random = next_random_u64(rng_state);
    let mantissa = (random >> 11).max(1);
    let uniform = mantissa as f64 * (1.0 / ((1_u64 << 53) as f64));
    let interval = -(uniform.ln()) * sample_interval as f64;

    if interval.is_finite() && interval < u64::MAX as f64 {
        (interval.ceil() as u64).max(1)
    } else {
        u64::MAX
    }
}

fn next_random_u64(rng_state: &mut u64) -> u64 {
    if *rng_state == 0 {
        *rng_state = next_thread_rng_seed();
    }
    *rng_state = (*rng_state).wrapping_add(RNG_INCREMENT);
    splitmix64(*rng_state)
}

fn next_thread_rng_seed() -> u64 {
    let seed = SAMPLING_RNG_SEED.fetch_add(RNG_INCREMENT, Ordering::Relaxed);
    splitmix64(seed.wrapping_add(RNG_INCREMENT))
}

fn splitmix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
fn calculate_deterministic_sample_weight(size: u64, current: u64, interval: u64) -> SampleWeight {
    let interval = interval.max(1);
    let current = current.clamp(1, interval);
    let bytes_after_first_sample = size.saturating_sub(current);
    let crossed_intervals = bytes_after_first_sample
        .checked_div(interval)
        .unwrap_or_default()
        .saturating_add(1);
    let weighted_bytes = crossed_intervals.saturating_mul(interval);
    let weighted_objects = weighted_bytes.checked_div(size).unwrap_or_default().max(1);
    let bytes_into_next_interval = bytes_after_first_sample % interval;
    let next_remaining = if bytes_into_next_interval == 0 {
        interval
    } else {
        interval - bytes_into_next_interval
    };

    SampleWeight {
        weighted_objects,
        weighted_bytes,
        next_remaining,
    }
}

fn record_sample(weight: SampleWeight) {
    let stack = StackKey::capture(MAX_CAPTURE_DEPTH);
    let sample = RecordedAllocationSample {
        stack,
        weighted_objects: weight.weighted_objects,
        weighted_bytes: weight.weighted_bytes,
    };

    TLS_SAMPLE_BUFFER.with(|buffer| {
        let Some(mut buffer) = buffer.try_lock() else {
            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
            return;
        };

        if buffer.is_full() {
            flush_tls_samples(&mut buffer);
        }

        if buffer.push(sample) {
            RECORDED_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
        } else {
            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        }
    });
}

fn flush_current_thread_samples() -> bool {
    TLS_SAMPLE_BUFFER.with(|buffer| {
        let Some(mut buffer) = buffer.try_lock() else {
            return false;
        };
        flush_tls_samples(&mut buffer)
    })
}

fn request_tls_sample_flush() {
    FLUSH_REQUEST_GENERATION.fetch_add(1, Ordering::Relaxed);
}

#[cfg(test)]
fn flush_requested_tls_samples() {
    SAMPLER_STATE.with(|sampler| {
        let mut state = sampler.get();
        flush_requested_tls_samples_with_state(&mut state);
        sampler.set(state);
    });
}

fn flush_requested_tls_samples_with_state(state: &mut SamplerState) {
    let requested_generation = FLUSH_REQUEST_GENERATION.load(Ordering::Relaxed);
    if state.flush_generation == requested_generation {
        return;
    }

    if flush_current_thread_samples() {
        state.flush_generation = requested_generation;
    }
}

fn reset_current_thread_sample_buffer() {
    let generation = FLUSH_REQUEST_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    SAMPLER_STATE.with(|sampler| {
        let mut state = sampler.get();
        state.flush_generation = generation;
        sampler.set(state);
    });
    clear_current_thread_samples();
}

fn clear_current_thread_samples() {
    TLS_SAMPLE_BUFFER.with(|buffer| {
        if let Some(mut buffer) = buffer.try_lock() {
            buffer.clear();
        }
    });
}

#[derive(Debug)]
struct TlsSampleBufferRegistry {
    buffers: Vec<Option<Arc<Mutex<TlsSampleBuffer>>>>,
    free_ids: Vec<usize>,
}

impl TlsSampleBufferRegistry {
    const fn new() -> Self {
        Self {
            buffers: Vec::new(),
            free_ids: Vec::new(),
        }
    }

    fn register(&mut self, buffer: Arc<Mutex<TlsSampleBuffer>>) -> usize {
        if let Some(id) = self.free_ids.pop() {
            self.buffers[id] = Some(buffer);
            id
        } else {
            let id = self.buffers.len();
            self.buffers.push(Some(buffer));
            id
        }
    }

    fn deregister(&mut self, id: usize) {
        if id < self.buffers.len() && self.buffers[id].take().is_some() {
            self.free_ids.push(id);
        }
    }

    fn buffers(&self) -> Vec<Arc<Mutex<TlsSampleBuffer>>> {
        self.buffers.iter().filter_map(Clone::clone).collect()
    }
}

fn register_tls_sample_buffer(buffer: Arc<Mutex<TlsSampleBuffer>>) -> Option<usize> {
    // TLS registration can happen from the allocator hook on first use. Do not
    // wait for the global registry lock there; unregistered threads still keep
    // local TLS buffering and can flush on ring pressure or thread exit.
    let Ok(mut registry) = TLS_SAMPLE_BUFFER_REGISTRY.try_lock() else {
        return None;
    };

    Some(registry.register(buffer))
}

fn deregister_tls_sample_buffer(id: usize) {
    if let Ok(mut registry) = TLS_SAMPLE_BUFFER_REGISTRY.lock() {
        registry.deregister(id);
    }
}

fn registered_tls_sample_buffers() -> Vec<Arc<Mutex<TlsSampleBuffer>>> {
    let Ok(registry) = TLS_SAMPLE_BUFFER_REGISTRY.lock() else {
        DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        return Vec::new();
    };

    registry.buffers()
}

fn registered_tls_buffered_samples() -> Option<usize> {
    with_profiler_suppressed(|| {
        let buffers = registered_tls_sample_buffers();
        let mut buffered_samples = 0_usize;
        for buffer in buffers {
            let Ok(buffer) = buffer.try_lock() else {
                return None;
            };
            buffered_samples = buffered_samples.saturating_add(buffer.len());
        }
        Some(buffered_samples)
    })
}

fn count_recorded_samples() -> usize {
    RECORDED_SAMPLE_SHARDS
        .iter()
        .filter_map(|shard| match shard.lock() {
            Ok(samples) => Some(samples.len()),
            Err(_) => {
                DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
                None
            }
        })
        .sum()
}

fn flush_registered_tls_samples() {
    // `report()` is outside the allocation hot path, so it can block briefly to
    // make the profile interval deterministic instead of best-effort.
    for buffer in registered_tls_sample_buffers() {
        if let Ok(mut buffer) = buffer.lock() {
            flush_tls_samples_for_report(&mut buffer);
        }
    }
}

fn clear_registered_tls_samples() {
    // Used only at backend lifecycle boundaries; clear rather than flush so
    // stale samples from a previous session cannot leak into a new report.
    for buffer in registered_tls_sample_buffers() {
        if let Ok(mut buffer) = buffer.lock() {
            buffer.clear();
        }
    }
}

fn flush_tls_samples(buffer: &mut TlsSampleBuffer) -> bool {
    flush_tls_samples_to_global(buffer, GlobalSampleShardLock::Try)
}

fn flush_tls_samples_for_report(buffer: &mut TlsSampleBuffer) -> bool {
    flush_tls_samples_to_global(buffer, GlobalSampleShardLock::Blocking)
}

#[derive(Debug, Copy, Clone)]
enum GlobalSampleShardLock {
    Try,
    Blocking,
}

fn flush_tls_samples_to_global(
    buffer: &mut TlsSampleBuffer,
    lock_mode: GlobalSampleShardLock,
) -> bool {
    if buffer.is_empty() {
        return true;
    }

    let reserved_slots = reserve_global_sample_slots(buffer.len());
    if reserved_slots == 0 {
        return false;
    };

    let shard_index =
        NEXT_RECORDED_SAMPLE_SHARD.fetch_add(1, Ordering::Relaxed) % RECORDED_SAMPLE_SHARD_COUNT;
    let mut samples = match lock_mode {
        GlobalSampleShardLock::Try => {
            let Ok(samples) = RECORDED_SAMPLE_SHARDS[shard_index].try_lock() else {
                release_global_sample_slots(reserved_slots);
                return false;
            };
            samples
        }
        GlobalSampleShardLock::Blocking => {
            let Ok(samples) = RECORDED_SAMPLE_SHARDS[shard_index].lock() else {
                release_global_sample_slots(reserved_slots);
                return false;
            };
            samples
        }
    };

    let flushed = buffer.drain_into(&mut samples, reserved_slots);
    if flushed < reserved_slots {
        release_global_sample_slots(reserved_slots - flushed);
    }
    if flushed > 0 {
        FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.fetch_add(flushed as u64, Ordering::Relaxed);
    }
    if !buffer.is_empty() {
        return false;
    }

    flushed > 0
}

fn reserve_global_sample_slots(wanted: usize) -> usize {
    let max_samples = MAX_RECORDED_SAMPLES.load(Ordering::Relaxed);
    let mut current = GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed);

    loop {
        let available = max_samples.saturating_sub(current);
        if available == 0 {
            return 0;
        }

        let reserved = wanted.min(available);
        match GLOBAL_BUFFERED_SAMPLE_COUNT.compare_exchange_weak(
            current,
            current + reserved,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return reserved,
            Err(observed) => current = observed,
        }
    }
}

fn release_global_sample_slots(slots: usize) {
    if slots == 0 {
        return;
    }

    let mut current = GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed);
    loop {
        let release = slots.min(current);
        if release == 0 {
            return;
        }

        match GLOBAL_BUFFERED_SAMPLE_COUNT.compare_exchange_weak(
            current,
            current - release,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn prepare_sample_buffer(capacity: usize) {
    for shard in RECORDED_SAMPLE_SHARDS.iter() {
        let Ok(mut samples) = shard.lock() else {
            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        samples.clear();
        let current_capacity = samples.capacity();
        let shard_capacity = recorded_sample_shard_capacity(capacity);
        if current_capacity < shard_capacity {
            samples.reserve(shard_capacity - current_capacity);
        }
    }
    GLOBAL_BUFFERED_SAMPLE_COUNT.store(count_recorded_samples(), Ordering::Relaxed);
}

fn drain_recorded_samples(limit: usize) -> Vec<RecordedAllocationSample> {
    let mut drained = Vec::new();
    let mut remaining = limit;

    for shard in RECORDED_SAMPLE_SHARDS.iter() {
        if remaining == 0 {
            break;
        }

        let Ok(mut samples) = shard.lock() else {
            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let drain_len = samples.len().min(remaining);
        drained.extend(samples.drain(..drain_len));
        remaining -= drain_len;
    }

    if !drained.is_empty() {
        release_global_sample_slots(drained.len());
    }

    drained
}

fn recorded_sample_shard_capacity(total_capacity: usize) -> usize {
    total_capacity.saturating_add(RECORDED_SAMPLE_SHARD_COUNT - 1) / RECORDED_SAMPLE_SHARD_COUNT
}

fn build_allocation_samples(
    recorded: Vec<RecordedAllocationSample>,
    max_depth: usize,
) -> Vec<AllocationSample> {
    let mut aggregated: HashMap<StackKey, AggregatedAllocationSample> = HashMap::new();
    for sample in recorded {
        let entry = aggregated.entry(sample.stack).or_default();
        entry.alloc_objects = entry.alloc_objects.saturating_add(sample.weighted_objects);
        entry.alloc_space = entry.alloc_space.saturating_add(sample.weighted_bytes);
    }

    aggregated
        .into_iter()
        .map(|(stack, sample)| {
            AllocationSample::new(
                resolve_stack(&stack, max_depth),
                i64::try_from(sample.alloc_objects).unwrap_or(i64::MAX),
                i64::try_from(sample.alloc_space).unwrap_or(i64::MAX),
            )
        })
        .collect()
}

fn resolve_stack(stack: &StackKey, max_depth: usize) -> Vec<String> {
    resolve_stack_with(stack, max_depth, resolve_frame_names)
}

fn resolve_stack_with(
    stack: &StackKey,
    max_depth: usize,
    resolve: impl FnMut(usize) -> Vec<String>,
) -> Vec<String> {
    let frames: Vec<String> = stack
        .iter()
        .flat_map(resolve)
        .filter(|name| !is_mimalloc_profiler_frame(name))
        .take(max_depth)
        .collect();

    if frames.is_empty() {
        vec![SYNTHETIC_FRAME.to_string()]
    } else {
        frames
    }
}

fn resolve_frame_names(ip: usize) -> Vec<String> {
    let mut resolved = Vec::new();
    backtrace::resolve(ip as *mut std::ffi::c_void, |symbol| {
        if let Some(name) = symbol.name() {
            resolved.push(name.to_string());
        }
    });
    if resolved.is_empty() {
        resolved.push(format!("0x{ip:x}"));
    }
    resolved
}

fn is_mimalloc_profiler_frame(name: &str) -> bool {
    name.contains("pyroscope::backend::mimalloc")
        || name.contains("pyroscope::encode::memory_pprof")
        || name.contains("backtrace::")
}

fn warm_backtrace() {
    let mut frames = 0;
    backtrace::trace(|_frame| {
        frames += 1;
        frames < 2
    });
}

fn duration_to_i64_nanos(duration: std::time::Duration) -> i64 {
    i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX)
}

fn duration_to_u64_micros(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as TestMutex;

    static TEST_LOCK: TestMutex<()> = TestMutex::new(());

    struct RecorderActiveGuard;

    impl Drop for RecorderActiveGuard {
        fn drop(&mut self) {
            RECORDER_ACTIVE.store(false, Ordering::Release);
        }
    }

    fn test_sample(stack: StackKey) -> RecordedAllocationSample {
        RecordedAllocationSample {
            stack,
            weighted_objects: 1,
            weighted_bytes: 1024,
        }
    }

    fn clear_test_buffers() {
        for shard in RECORDED_SAMPLE_SHARDS.iter() {
            shard.lock().expect("lock samples").clear();
        }
        GLOBAL_BUFFERED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        NEXT_RECORDED_SAMPLE_SHARD.store(0, Ordering::Relaxed);
        SAMPLER_STATE.with(|sampler| sampler.set(SamplerState::new()));
        for buffer in registered_tls_sample_buffers() {
            buffer.lock().expect("lock tls samples").clear();
        }
    }

    fn push_global_test_samples(samples: impl IntoIterator<Item = RecordedAllocationSample>) {
        let mut shard = RECORDED_SAMPLE_SHARDS[0].lock().expect("lock samples");
        let initial_len = shard.len();
        shard.extend(samples);
        GLOBAL_BUFFERED_SAMPLE_COUNT.fetch_add(shard.len() - initial_len, Ordering::Relaxed);
    }

    #[test]
    fn mimalloc_config_default_is_valid() {
        assert!(MimallocConfig::default().validate().is_ok());
    }

    #[test]
    fn mimalloc_config_rejects_zero_sample_interval() {
        let config = MimallocConfig {
            sample_interval_bytes: 0,
            ..MimallocConfig::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn mimalloc_stats_reports_global_recorder_counters() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        clear_test_buffers();
        RECORDED_SAMPLE_COUNT.store(7, Ordering::Relaxed);
        DROPPED_SAMPLES.store(3, Ordering::Relaxed);
        LAST_PPROF_ENCODE_ELAPSED_MICROS.store(11, Ordering::Relaxed);

        let stats = mimalloc_stats();

        assert_eq!(
            stats,
            MimallocStats {
                recorded_samples: 7,
                flushes: 0,
                flushed_samples: 0,
                dropped_samples: 3,
                buffered_samples: Some(0),
                last_pprof_encode_elapsed_micros: 11,
            }
        );

        RECORDED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        LAST_PPROF_ENCODE_ELAPSED_MICROS.store(0, Ordering::Relaxed);
        clear_test_buffers();
    }

    #[test]
    fn mimalloc_stats_includes_current_thread_tls_samples() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        push_global_test_samples([test_sample(stack)]);
        TLS_SAMPLE_BUFFER.with(|buffer| {
            let mut buffer = buffer.try_lock().expect("lock current thread buffer");
            assert!(buffer.push(test_sample(stack)));
            assert!(buffer.push(test_sample(stack)));
        });

        assert_eq!(mimalloc_stats().buffered_samples, Some(3));

        clear_test_buffers();
    }

    #[test]
    fn profiler_suppression_skips_internal_allocations_without_drop_count() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        clear_test_buffers();
        RECORDER_ACTIVE.store(true, Ordering::Release);
        let _active_guard = RecorderActiveGuard;
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        RECORDED_SAMPLE_COUNT.store(0, Ordering::Relaxed);

        with_profiler_suppressed(|| record_allocation(1));

        assert_eq!(RECORDED_SAMPLE_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(DROPPED_SAMPLES.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn reentrant_allocation_is_ignored_without_drop_count() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        clear_test_buffers();
        RECORDER_ACTIVE.store(true, Ordering::Release);
        let _active_guard = RecorderActiveGuard;
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        RECORDED_SAMPLE_COUNT.store(0, Ordering::Relaxed);

        SAMPLER_STATE.with(|sampler| {
            let mut state = sampler.get();
            let previous = state.in_profiler;
            state.in_profiler = true;
            sampler.set(state);
            record_allocation(1);
            let mut state = sampler.get();
            state.in_profiler = previous;
            sampler.set(state);
        });

        assert_eq!(RECORDED_SAMPLE_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(DROPPED_SAMPLES.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn realloc_recorded_size_tracks_newly_allocated_bytes() {
        let mut old_byte = 0_u8;
        let mut new_byte = 0_u8;
        let old_ptr = &mut old_byte as *mut u8;
        let new_ptr = &mut new_byte as *mut u8;

        assert_eq!(realloc_recorded_size(old_ptr, old_ptr, 1024, 1536), 512);
        assert_eq!(realloc_recorded_size(old_ptr, old_ptr, 1536, 1024), 0);
        assert_eq!(realloc_recorded_size(old_ptr, new_ptr, 1024, 1536), 1536);
        assert_eq!(realloc_recorded_size(old_ptr, new_ptr, 1536, 1024), 1024);
    }

    #[test]
    fn drain_recorded_samples_respects_limit_and_keeps_remaining_samples() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        push_global_test_samples([test_sample(stack), test_sample(stack), test_sample(stack)]);

        let drained = drain_recorded_samples(2);

        assert_eq!(drained.len(), 2);
        assert_eq!(mimalloc_stats().buffered_samples, Some(1));

        clear_test_buffers();
    }

    #[test]
    fn release_global_sample_slots_saturates_at_zero() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        GLOBAL_BUFFERED_SAMPLE_COUNT.store(0, Ordering::Relaxed);

        release_global_sample_slots(3);
        assert_eq!(GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed), 0);

        GLOBAL_BUFFERED_SAMPLE_COUNT.store(2, Ordering::Relaxed);
        release_global_sample_slots(5);
        assert_eq!(GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed), 0);

        GLOBAL_BUFFERED_SAMPLE_COUNT.store(5, Ordering::Relaxed);
        release_global_sample_slots(3);
        assert_eq!(GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed), 2);

        GLOBAL_BUFFERED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
    }

    #[test]
    fn prepare_sample_buffer_recounts_global_sample_slots_after_clear() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        push_global_test_samples([test_sample(stack), test_sample(stack)]);
        GLOBAL_BUFFERED_SAMPLE_COUNT.store(usize::MAX, Ordering::Relaxed);

        prepare_sample_buffer(10);

        assert_eq!(count_recorded_samples(), 0);
        assert_eq!(GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed), 0);

        clear_test_buffers();
    }

    #[test]
    fn tls_sample_buffer_drain_into_respects_limit() {
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        let mut buffer = TlsSampleBuffer::new();
        assert!(buffer.push(test_sample(stack)));
        assert!(buffer.push(test_sample(stack)));
        assert!(buffer.push(test_sample(stack)));
        let mut out = Vec::new();

        let drained = buffer.drain_into(&mut out, 2);

        assert_eq!(drained, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn tls_sample_buffer_registry_reuses_deregistered_slots() {
        let _guard = TEST_LOCK.lock().expect("lock test");

        let first_id = register_tls_sample_buffer(Arc::new(Mutex::new(TlsSampleBuffer::new())))
            .expect("register first buffer");
        deregister_tls_sample_buffer(first_id);

        let second_id = register_tls_sample_buffer(Arc::new(Mutex::new(TlsSampleBuffer::new())))
            .expect("register second buffer");

        assert_eq!(second_id, first_id);
        deregister_tls_sample_buffer(second_id);
    }

    #[test]
    fn tls_sample_buffer_retries_registration_after_initial_contention() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let registry_guard = TLS_SAMPLE_BUFFER_REGISTRY.lock().expect("lock registry");

        let buffer = RegisteredTlsSampleBuffer::new();
        assert_eq!(buffer.id.get(), None);
        drop(buffer.try_lock().expect("lock local buffer"));
        assert_eq!(buffer.id.get(), None);

        drop(registry_guard);
        drop(buffer.try_lock().expect("lock local buffer after retry"));

        let id = buffer.id.get().expect("registration retried");
        deregister_tls_sample_buffer(id);
        buffer.id.set(None);
    }

    #[test]
    fn flush_tls_samples_moves_buffer_to_global_samples() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        MAX_RECORDED_SAMPLES.store(10, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        let mut buffer = TlsSampleBuffer::new();
        assert!(buffer.push(test_sample(stack)));
        assert!(buffer.push(test_sample(stack)));

        assert!(flush_tls_samples(&mut buffer));

        assert_eq!(buffer.len(), 0);
        assert_eq!(mimalloc_stats().flushes, 1);
        assert_eq!(mimalloc_stats().flushed_samples, 2);
        assert_eq!(mimalloc_stats().buffered_samples, Some(2));
        clear_test_buffers();
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
    }

    #[test]
    fn flush_tls_samples_keeps_tls_samples_when_global_capacity_is_full() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(0, Ordering::Relaxed);
        let mut buffer = TlsSampleBuffer::new();
        assert!(buffer.push(test_sample(stack)));
        assert!(buffer.push(test_sample(stack)));

        assert!(!flush_tls_samples(&mut buffer));

        assert_eq!(buffer.len(), 2);
        assert_eq!(DROPPED_SAMPLES.load(Ordering::Relaxed), 0);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
        clear_test_buffers();
    }

    #[test]
    fn flush_tls_samples_keeps_unflushed_tls_samples_after_partial_flush() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(1, Ordering::Relaxed);
        let mut buffer = TlsSampleBuffer::new();
        assert!(buffer.push(test_sample(stack)));
        assert!(buffer.push(test_sample(stack)));

        assert!(!flush_tls_samples(&mut buffer));

        assert_eq!(buffer.len(), 1);
        assert_eq!(count_recorded_samples(), 1);
        assert_eq!(GLOBAL_BUFFERED_SAMPLE_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(DROPPED_SAMPLES.load(Ordering::Relaxed), 0);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        clear_test_buffers();
    }

    #[test]
    fn flush_requested_tls_samples_flushes_current_thread_buffer_once() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        MAX_RECORDED_SAMPLES.store(10, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        TLS_SAMPLE_BUFFER.with(|buffer| {
            let mut buffer = buffer.try_lock().expect("lock current thread buffer");
            assert!(buffer.push(test_sample(stack)));
            assert!(buffer.push(test_sample(stack)));
        });

        request_tls_sample_flush();
        flush_requested_tls_samples();

        assert_eq!(mimalloc_stats().buffered_samples, Some(2));
        assert_eq!(mimalloc_stats().flushes, 1);
        assert_eq!(mimalloc_stats().flushed_samples, 2);

        flush_requested_tls_samples();
        assert_eq!(mimalloc_stats().buffered_samples, Some(2));
        assert_eq!(mimalloc_stats().flushes, 1);
        assert_eq!(mimalloc_stats().flushed_samples, 2);

        clear_test_buffers();
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
    }

    #[test]
    fn flush_requested_tls_samples_retries_after_lock_failure() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        MAX_RECORDED_SAMPLES.store(10, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);

        TLS_SAMPLE_BUFFER.with(|buffer| {
            let mut locked_buffer = buffer.try_lock().expect("lock current thread buffer");
            assert!(locked_buffer.push(test_sample(stack)));

            request_tls_sample_flush();
            flush_requested_tls_samples();
            assert_eq!(mimalloc_stats().flushes, 0);
            assert_eq!(locked_buffer.len(), 1);
        });

        flush_requested_tls_samples();

        assert_eq!(mimalloc_stats().flushes, 1);
        assert_eq!(mimalloc_stats().flushed_samples, 1);

        clear_test_buffers();
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
    }

    #[test]
    fn flush_registered_tls_samples_drains_live_worker_thread_buffer() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        MAX_RECORDED_SAMPLES.store(10, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();

        let worker = std::thread::spawn(move || {
            TLS_SAMPLE_BUFFER.with(|buffer| {
                let mut buffer = buffer.try_lock().expect("lock worker buffer");
                assert!(buffer.push(test_sample(stack)));
                assert!(buffer.push(test_sample(stack)));
            });
            ready_tx.send(()).expect("send ready");
            release_rx.recv().expect("wait for release");
        });

        ready_rx.recv().expect("wait for worker buffer");
        assert_eq!(registered_tls_buffered_samples(), Some(2));

        flush_registered_tls_samples();

        let stats = mimalloc_stats();
        assert_eq!(stats.flushes, 1);
        assert_eq!(stats.flushed_samples, 2);
        assert_eq!(stats.buffered_samples, Some(2));

        release_tx.send(()).expect("release worker");
        worker.join().expect("join worker");
        clear_test_buffers();
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
    }

    #[test]
    fn tls_sample_buffer_flushes_on_thread_exit_when_recorder_is_active() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        MAX_RECORDED_SAMPLES.store(10, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        RECORDER_ACTIVE.store(true, Ordering::Release);
        let _active_guard = RecorderActiveGuard;

        std::thread::spawn(move || {
            TLS_SAMPLE_BUFFER.with(|buffer| {
                let mut buffer = buffer.try_lock().expect("lock worker buffer");
                assert!(buffer.push(test_sample(stack)));
                assert!(buffer.push(test_sample(stack)));
            });
        })
        .join()
        .expect("join allocation thread");

        let stats = mimalloc_stats();
        assert!(matches!(stats.buffered_samples, Some(samples) if samples >= 2));
        assert!(stats.flushes >= 1);
        assert!(stats.flushed_samples >= 2);
        assert_eq!(stats.dropped_samples, 0);
        assert_eq!(registered_tls_buffered_samples(), Some(0));

        clear_test_buffers();
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
    }

    #[test]
    fn clear_registered_tls_samples_drains_all_live_thread_buffers() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();

        let worker = std::thread::spawn(move || {
            TLS_SAMPLE_BUFFER.with(|buffer| {
                let mut buffer = buffer.try_lock().expect("lock worker buffer");
                assert!(buffer.push(test_sample(stack)));
                assert!(buffer.push(test_sample(stack)));
            });
            ready_tx.send(()).expect("send ready");
            release_rx.recv().expect("wait for release");
        });

        ready_rx.recv().expect("wait for worker buffer");
        assert_eq!(registered_tls_buffered_samples(), Some(2));

        clear_registered_tls_samples();

        assert_eq!(registered_tls_buffered_samples(), Some(0));
        release_tx.send(()).expect("release worker");
        worker.join().expect("join worker");
        clear_test_buffers();
    }

    #[test]
    fn build_allocation_samples_groups_matching_stacks() {
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        let samples = build_allocation_samples(
            vec![
                RecordedAllocationSample {
                    stack,
                    weighted_objects: 8,
                    weighted_bytes: 1024,
                },
                RecordedAllocationSample {
                    stack,
                    weighted_objects: 4,
                    weighted_bytes: 1024,
                },
            ],
            DEFAULT_MAX_DEPTH,
        );

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].alloc_objects, 12);
        assert_eq!(samples[0].alloc_space, 2048);
    }

    #[test]
    fn resolve_stack_applies_depth_after_profiler_frame_filtering() {
        let mut frames = [0; MAX_CAPTURE_DEPTH];
        frames[..4].copy_from_slice(&[1, 2, 3, 4]);
        let stack = StackKey { frames, depth: 4 };

        let resolved = resolve_stack_with(&stack, 1, |ip| match ip {
            1 => vec!["pyroscope::backend::mimalloc::record_sample".to_string()],
            2 => vec!["backtrace::trace_unsynchronized".to_string()],
            3 => vec!["example::allocate".to_string()],
            4 => vec!["example::caller".to_string()],
            _ => Vec::new(),
        });

        assert_eq!(resolved, vec!["example::allocate"]);
    }

    #[test]
    fn resolve_stack_expands_inline_symbols_before_filtering() {
        let mut frames = [0; MAX_CAPTURE_DEPTH];
        frames[0] = 1;
        let stack = StackKey { frames, depth: 1 };

        let resolved = resolve_stack_with(&stack, 2, |ip| match ip {
            1 => vec![
                "pyroscope::backend::mimalloc::record_sample".to_string(),
                "example::inline_allocate".to_string(),
                "example::caller".to_string(),
            ],
            _ => Vec::new(),
        });

        assert_eq!(
            resolved,
            vec!["example::inline_allocate", "example::caller"]
        );
    }

    #[test]
    fn calculate_sample_weight_uses_interval_for_small_allocation() {
        let mut rng_state = 1;
        let weight = calculate_sample_weight(128, 128, 1024, &mut rng_state);

        assert_eq!(weight.weighted_objects, 8);
        assert_eq!(weight.weighted_bytes, 1024);
        assert!(weight.next_remaining > 0);
    }

    #[test]
    fn calculate_sample_weight_carries_large_allocation_overshoot_with_poisson_intervals() {
        let mut rng_state = 1;
        let weight = calculate_sample_weight(2500, 1000, 1000, &mut rng_state);

        assert!(weight.weighted_objects >= 1);
        assert!(weight.weighted_bytes >= 1000);
        assert!(weight.next_remaining > 0);
    }

    #[test]
    fn calculate_sample_weight_bounds_large_allocation_interval_work() {
        let mut rng_state = 1;
        let size = (MAX_POISSON_INTERVALS_PER_ALLOCATION + 1024) * 1024;
        let weight = calculate_sample_weight(size, 1, 1024, &mut rng_state);

        assert!(weight.weighted_objects >= 1);
        assert!(weight.weighted_bytes >= MAX_POISSON_INTERVALS_PER_ALLOCATION * 1024);
        assert!(weight.next_remaining > 0);
    }

    #[test]
    fn deterministic_sample_weight_documents_previous_interval_semantics() {
        let weight = calculate_deterministic_sample_weight(2500, 1000, 1000);

        assert_eq!(weight.weighted_objects, 1);
        assert_eq!(weight.weighted_bytes, 2000);
        assert_eq!(weight.next_remaining, 500);
    }

    #[test]
    fn next_poisson_interval_uses_thread_rng_state() {
        let mut rng_state = 1;
        let first = next_poisson_interval(1024, &mut rng_state);
        let second = next_poisson_interval(1024, &mut rng_state);

        assert!(first > 0);
        assert!(second > 0);
        assert_ne!(first, second);
    }
}
