use std::{
    alloc::{GlobalAlloc, Layout},
    cell::Cell,
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
const SYNTHETIC_FRAME: &str = "[mimalloc] sampled allocations (stack capture pending)";

static RECORDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static ALLOCATOR_SEEN: AtomicBool = AtomicBool::new(false);
static SAMPLE_INTERVAL_BYTES: AtomicU64 = AtomicU64::new(DEFAULT_SAMPLE_INTERVAL_BYTES);
static SAMPLING_CONFIG_GENERATION: AtomicU64 = AtomicU64::new(0);
static MAX_CAPTURED_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_DEPTH);
static MAX_RECORDED_SAMPLES: AtomicUsize = AtomicUsize::new(DEFAULT_RING_CAPACITY);
static DROPPED_SAMPLES: AtomicU64 = AtomicU64::new(0);
static RECORDED_SAMPLES: Mutex<Vec<RecordedAllocationSample>> = Mutex::new(Vec::new());

thread_local! {
    static IN_ALLOC_PROFILER: Cell<bool> = const { Cell::new(false) };
    static REMAINING_BYTES: Cell<u64> = const { Cell::new(DEFAULT_SAMPLE_INTERVAL_BYTES) };
    static REMAINING_CONFIG_GENERATION: Cell<u64> = const { Cell::new(0) };
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
        DROPPED_SAMPLES.store(0, Ordering::Relaxed);
        prepare_sample_buffer(self.config.ring_capacity);
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

        let samples = build_allocation_samples(drain_recorded_samples(), self.config.max_depth);

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
        REMAINING_BYTES.with(|remaining| {
            REMAINING_CONFIG_GENERATION.with(|remaining_generation| {
                let interval = SAMPLE_INTERVAL_BYTES.load(Ordering::Relaxed).max(1);
                let generation = SAMPLING_CONFIG_GENERATION.load(Ordering::Relaxed);
                let mut current = remaining.get();
                if remaining_generation.get() != generation || current == 0 || current > interval {
                    current = interval;
                    remaining_generation.set(generation);
                }

                if size < current {
                    remaining.set(current - size);
                } else {
                    let weight = calculate_sample_weight(size, current, interval);
                    remaining.set(weight.next_remaining);
                    record_sample(weight);
                }
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

fn calculate_sample_weight(size: u64, current: u64, interval: u64) -> SampleWeight {
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
    let max_samples = MAX_RECORDED_SAMPLES.load(Ordering::Relaxed);
    let Ok(mut samples) = RECORDED_SAMPLES.try_lock() else {
        DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        return;
    };

    if samples.len() >= max_samples {
        DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    samples.push(RecordedAllocationSample {
        stack,
        weighted_objects: weight.weighted_objects,
        weighted_bytes: weight.weighted_bytes,
    });
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

fn drain_recorded_samples() -> Vec<RecordedAllocationSample> {
    let Ok(mut samples) = RECORDED_SAMPLES.lock() else {
        DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        return Vec::new();
    };
    samples.drain(..).collect()
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
        let weight = calculate_sample_weight(128, 128, 1024);

        assert_eq!(
            weight,
            SampleWeight {
                weighted_objects: 8,
                weighted_bytes: 1024,
                next_remaining: 1024,
            }
        );
    }

    #[test]
    fn calculate_sample_weight_carries_large_allocation_overshoot() {
        let weight = calculate_sample_weight(2500, 1000, 1000);

        assert_eq!(
            weight,
            SampleWeight {
                weighted_objects: 1,
                weighted_bytes: 2000,
                next_remaining: 500,
            }
        );
    }
}
