use std::{
    alloc::{GlobalAlloc, Layout},
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
    time::Instant,
};

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
const SYNTHETIC_FRAME: &str = "[mimalloc] sampled allocations (stack capture pending)";
const RNG_INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;
const RNG_INITIAL_STATE: u64 = 0xa076_1d64_78bd_642f;

static RECORDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static ALLOCATOR_SEEN: AtomicBool = AtomicBool::new(false);
static SAMPLE_INTERVAL_BYTES: AtomicU64 = AtomicU64::new(DEFAULT_SAMPLE_INTERVAL_BYTES);
static SAMPLING_CONFIG_GENERATION: AtomicU64 = AtomicU64::new(0);
static SAMPLING_RNG_SEED: AtomicU64 = AtomicU64::new(RNG_INITIAL_STATE);
static FLUSH_REQUEST_GENERATION: AtomicU64 = AtomicU64::new(0);
static MAX_CAPTURED_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_DEPTH);
static MAX_RECORDED_SAMPLES: AtomicUsize = AtomicUsize::new(DEFAULT_RING_CAPACITY);
static RECORDED_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);
static FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);
static FLUSHED_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);
static DROPPED_SAMPLES: AtomicU64 = AtomicU64::new(0);
static RECORDED_SAMPLES: Mutex<Vec<RecordedAllocationSample>> = Mutex::new(Vec::new());

thread_local! {
    static IN_ALLOC_PROFILER: Cell<bool> = const { Cell::new(false) };
    static REMAINING_BYTES: Cell<u64> = const { Cell::new(DEFAULT_SAMPLE_INTERVAL_BYTES) };
    static REMAINING_CONFIG_GENERATION: Cell<u64> = const { Cell::new(0) };
    static SAMPLE_RNG_STATE: Cell<u64> = const { Cell::new(0) };
    static TLS_FLUSH_GENERATION: Cell<u64> = const { Cell::new(0) };
    static TLS_SAMPLE_BUFFER: RefCell<TlsSampleBuffer> = const { RefCell::new(TlsSampleBuffer::new()) };
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

impl Drop for TlsSampleBuffer {
    fn drop(&mut self) {
        if RECORDER_ACTIVE.load(Ordering::Acquire) {
            flush_tls_samples(self);
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct AggregatedAllocationSample {
    alloc_objects: u64,
    alloc_space: u64,
}

/// Configuration for the mimalloc memory profiling backend.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MimallocConfig {
    pub sample_interval_bytes: u64,
    pub max_depth: usize,
    pub ring_capacity: usize,
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
    /// Number of samples dropped because the recorder was re-entered, full, or locked.
    pub dropped_samples: u64,
    /// Number of samples currently buffered for the next report, if the buffer lock is available.
    pub buffered_samples: Option<usize>,
}

/// Return current mimalloc backend recorder counters.
pub fn mimalloc_stats() -> MimallocStats {
    let global_buffered_samples = RECORDED_SAMPLES
        .try_lock()
        .ok()
        .map(|samples| samples.len());
    let current_thread_buffered_samples = current_thread_buffered_samples();

    MimallocStats {
        recorded_samples: RECORDED_SAMPLE_COUNT.load(Ordering::Relaxed),
        flushes: FLUSH_COUNT.load(Ordering::Relaxed),
        flushed_samples: FLUSHED_SAMPLE_COUNT.load(Ordering::Relaxed),
        dropped_samples: DROPPED_SAMPLES.load(Ordering::Relaxed),
        buffered_samples: global_buffered_samples
            .zip(current_thread_buffered_samples)
            .map(|(global, current_thread)| global.saturating_add(current_thread)),
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
/// ```ignore
/// use pyroscope::backend::mimalloc::SamplingMiMalloc;
///
/// #[global_allocator]
/// static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();
/// ```
pub struct SamplingMiMalloc {
    inner: mimalloc::MiMalloc,
}

impl SamplingMiMalloc {
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
        ALLOCATOR_SEEN.store(true, Ordering::Relaxed);
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            record_allocation(layout.size() as u64);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATOR_SEEN.store(true, Ordering::Relaxed);
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record_allocation(layout.size() as u64);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATOR_SEEN.store(true, Ordering::Relaxed);
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATOR_SEEN.store(true, Ordering::Relaxed);
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            record_allocation(new_size as u64);
        }
        new_ptr
    }
}

/// Create a mimalloc memory profiling backend.
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
        MAX_CAPTURED_DEPTH.store(
            self.config.max_depth.min(MAX_CAPTURE_DEPTH),
            Ordering::Relaxed,
        );
        MAX_RECORDED_SAMPLES.store(self.config.ring_capacity, Ordering::Relaxed);
        RECORDED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        prepare_sample_buffer(self.config.ring_capacity);
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
        log::trace!(target: LOG_TAG, "Shutting down mimalloc backend");
        Ok(())
    }

    fn report(&mut self) -> Result<ReportBatch> {
        let now = Instant::now();
        let duration_nanos = self
            .last_report
            .replace(now)
            .map(|last_report| duration_to_i64_nanos(now.duration_since(last_report)))
            .unwrap_or_default();

        request_tls_sample_flush();
        flush_current_thread_samples();
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

        let pprof_data = memory_pprof::encode_allocation_profile(
            &samples,
            self.config.sample_interval_bytes,
            duration_nanos,
        );

        Ok(ReportBatch {
            profile_type: "memory".into(),
            data: ReportData::RawPprof(pprof_data),
        })
    }

    fn add_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }

    fn remove_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }
}

fn record_allocation(size: u64) {
    if size == 0 || !RECORDER_ACTIVE.load(Ordering::Acquire) {
        return;
    }

    IN_ALLOC_PROFILER.with(|in_profiler| {
        if in_profiler.get() {
            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
            return;
        }

        in_profiler.set(true);
        flush_requested_tls_samples();
        SAMPLE_RNG_STATE.with(|rng_state| {
            REMAINING_BYTES.with(|remaining| {
                REMAINING_CONFIG_GENERATION.with(|remaining_generation| {
                    let interval = SAMPLE_INTERVAL_BYTES.load(Ordering::Relaxed).max(1);
                    let generation = SAMPLING_CONFIG_GENERATION.load(Ordering::Relaxed);
                    let mut current = remaining.get();
                    if remaining_generation.get() != generation || current == 0 {
                        clear_current_thread_samples();
                        rng_state.set(next_thread_rng_seed());
                        current = next_poisson_interval(interval, rng_state);
                        remaining_generation.set(generation);
                    }

                    if size < current {
                        remaining.set(current - size);
                    } else {
                        let weight = calculate_sample_weight(size, current, interval, rng_state);
                        remaining.set(weight.next_remaining);
                        record_sample(weight);
                    }
                });
            });
        });
        in_profiler.set(false);
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
    rng_state: &Cell<u64>,
) -> SampleWeight {
    let sample_interval = sample_interval.max(1);
    let mut remaining_bytes = size.saturating_sub(current.max(1));
    let mut crossed_intervals = 1_u64;
    let mut next_remaining = next_poisson_interval(sample_interval, rng_state);

    while remaining_bytes >= next_remaining {
        remaining_bytes -= next_remaining;
        crossed_intervals = crossed_intervals.saturating_add(1);
        next_remaining = next_poisson_interval(sample_interval, rng_state);
    }

    let weighted_bytes = crossed_intervals.saturating_mul(sample_interval);
    let weighted_objects = weighted_bytes.checked_div(size).unwrap_or_default().max(1);
    next_remaining -= remaining_bytes;

    SampleWeight {
        weighted_objects,
        weighted_bytes,
        next_remaining,
    }
}

fn next_poisson_interval(sample_interval: u64, rng_state: &Cell<u64>) -> u64 {
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

fn next_random_u64(rng_state: &Cell<u64>) -> u64 {
    let mut state = rng_state.get();
    if state == 0 {
        state = next_thread_rng_seed();
    }
    state = state.wrapping_add(RNG_INCREMENT);
    rng_state.set(state);
    splitmix64(state)
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
    let stack = StackKey::capture(MAX_CAPTURED_DEPTH.load(Ordering::Relaxed));
    let sample = RecordedAllocationSample {
        stack,
        weighted_objects: weight.weighted_objects,
        weighted_bytes: weight.weighted_bytes,
    };

    TLS_SAMPLE_BUFFER.with(|buffer| {
        let Ok(mut buffer) = buffer.try_borrow_mut() else {
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

fn flush_current_thread_samples() {
    TLS_SAMPLE_BUFFER.with(|buffer| {
        if let Ok(mut buffer) = buffer.try_borrow_mut() {
            flush_tls_samples(&mut buffer);
        }
    });
}

fn request_tls_sample_flush() {
    FLUSH_REQUEST_GENERATION.fetch_add(1, Ordering::Relaxed);
}

fn flush_requested_tls_samples() {
    let requested_generation = FLUSH_REQUEST_GENERATION.load(Ordering::Relaxed);
    TLS_FLUSH_GENERATION.with(|seen_generation| {
        if seen_generation.get() == requested_generation {
            return;
        }

        seen_generation.set(requested_generation);
        flush_current_thread_samples();
    });
}

fn current_thread_buffered_samples() -> Option<usize> {
    TLS_SAMPLE_BUFFER.with(|buffer| buffer.try_borrow().ok().map(|buffer| buffer.len()))
}

fn reset_current_thread_sample_buffer() {
    let generation = FLUSH_REQUEST_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    TLS_FLUSH_GENERATION.with(|seen_generation| seen_generation.set(generation));
    clear_current_thread_samples();
}

fn clear_current_thread_samples() {
    TLS_SAMPLE_BUFFER.with(|buffer| {
        if let Ok(mut buffer) = buffer.try_borrow_mut() {
            buffer.clear();
        }
    });
}

fn flush_tls_samples(buffer: &mut TlsSampleBuffer) -> bool {
    if buffer.is_empty() {
        return true;
    }

    let Ok(mut samples) = RECORDED_SAMPLES.try_lock() else {
        drop_tls_samples(buffer);
        return false;
    };

    let max_samples = MAX_RECORDED_SAMPLES.load(Ordering::Relaxed);
    let available = max_samples.saturating_sub(samples.len());
    if available == 0 {
        drop_tls_samples(buffer);
        return false;
    }

    let flushed = buffer.drain_into(&mut samples, available);
    if flushed > 0 {
        FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.fetch_add(flushed as u64, Ordering::Relaxed);
    }
    if !buffer.is_empty() {
        drop_tls_samples(buffer);
        return false;
    }

    flushed > 0
}

fn drop_tls_samples(buffer: &mut TlsSampleBuffer) {
    DROPPED_SAMPLES.fetch_add(buffer.len() as u64, Ordering::Relaxed);
    buffer.clear();
}

fn prepare_sample_buffer(capacity: usize) {
    if let Ok(mut samples) = RECORDED_SAMPLES.lock() {
        samples.clear();
        let current_capacity = samples.capacity();
        if current_capacity < capacity {
            samples.reserve(capacity - current_capacity);
        }
    }
}

fn drain_recorded_samples(limit: usize) -> Vec<RecordedAllocationSample> {
    let Ok(mut samples) = RECORDED_SAMPLES.lock() else {
        DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        return Vec::new();
    };
    let drain_len = samples.len().min(limit);
    samples.drain(..drain_len).collect()
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
    let frames: Vec<String> = stack
        .iter()
        .take(max_depth)
        .filter_map(resolve_frame)
        .filter(|name| !is_mimalloc_profiler_frame(name))
        .collect();

    if frames.is_empty() {
        vec![SYNTHETIC_FRAME.to_string()]
    } else {
        frames
    }
}

fn resolve_frame(ip: usize) -> Option<String> {
    let mut resolved = None;
    backtrace::resolve(ip as *mut std::ffi::c_void, |symbol| {
        if let Some(name) = symbol.name() {
            resolved = Some(name.to_string());
        }
    });
    resolved.or_else(|| Some(format!("0x{ip:x}")))
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
        RECORDED_SAMPLES.lock().expect("lock samples").clear();
        clear_current_thread_samples();
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

        let stats = mimalloc_stats();

        assert_eq!(
            stats,
            MimallocStats {
                recorded_samples: 7,
                flushes: 0,
                flushed_samples: 0,
                dropped_samples: 3,
                buffered_samples: Some(0),
            }
        );

        RECORDED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
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
        {
            let mut global_samples = RECORDED_SAMPLES.lock().expect("lock samples");
            global_samples.push(test_sample(stack));
        }
        TLS_SAMPLE_BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            assert!(buffer.push(test_sample(stack)));
            assert!(buffer.push(test_sample(stack)));
        });

        assert_eq!(mimalloc_stats().buffered_samples, Some(3));

        clear_test_buffers();
    }

    #[test]
    fn drain_recorded_samples_respects_limit_and_keeps_remaining_samples() {
        let _guard = TEST_LOCK.lock().expect("lock test");
        let stack = StackKey {
            frames: [42; MAX_CAPTURE_DEPTH],
            depth: 1,
        };
        clear_test_buffers();
        {
            let mut samples = RECORDED_SAMPLES.lock().expect("lock samples");
            samples.extend([test_sample(stack), test_sample(stack), test_sample(stack)]);
        }

        let drained = drain_recorded_samples(2);

        assert_eq!(drained.len(), 2);
        assert_eq!(mimalloc_stats().buffered_samples, Some(1));

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
    fn flush_tls_samples_drops_when_global_capacity_is_full() {
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

        assert_eq!(buffer.len(), 0);
        assert_eq!(DROPPED_SAMPLES.load(Ordering::Relaxed), 2);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
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
            let mut buffer = buffer.borrow_mut();
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
        RECORDER_ACTIVE.store(true, Ordering::Release);
        let _active_guard = RecorderActiveGuard;

        std::thread::spawn(move || {
            TLS_SAMPLE_BUFFER.with(|buffer| {
                let mut buffer = buffer.borrow_mut();
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

        clear_test_buffers();
        FLUSH_COUNT.store(0, Ordering::Relaxed);
        FLUSHED_SAMPLE_COUNT.store(0, Ordering::Relaxed);
        MAX_RECORDED_SAMPLES.store(DEFAULT_RING_CAPACITY, Ordering::Relaxed);
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
    fn calculate_sample_weight_uses_interval_for_small_allocation() {
        let rng_state = Cell::new(1);
        let weight = calculate_sample_weight(128, 128, 1024, &rng_state);

        assert_eq!(weight.weighted_objects, 8);
        assert_eq!(weight.weighted_bytes, 1024);
        assert!(weight.next_remaining > 0);
    }

    #[test]
    fn calculate_sample_weight_carries_large_allocation_overshoot_with_poisson_intervals() {
        let rng_state = Cell::new(1);
        let weight = calculate_sample_weight(2500, 1000, 1000, &rng_state);

        assert!(weight.weighted_objects >= 1);
        assert!(weight.weighted_bytes >= 1000);
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
        let rng_state = Cell::new(1);
        let first = next_poisson_interval(1024, &rng_state);
        let second = next_poisson_interval(1024, &rng_state);

        assert!(first > 0);
        assert!(second > 0);
        assert_ne!(first, second);
    }
}
