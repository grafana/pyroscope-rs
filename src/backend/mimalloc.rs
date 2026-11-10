//! Experimental Pyroscope adapter for the standalone `rustfs-mimalloc-profiling` crate.
//!
//! This optional backend may change or be removed independently of stable backends.
//! Install [`rustfs_mimalloc_profiling::SamplingAllocator`] from `rustfs-mimalloc-profiling` as the global allocator, then pass
//! [`mimalloc_backend`] to the agent builder. Sampling, live heap tracking,
//! symbolization and pprof generation are owned by the profiling crate.
//!
//! Applications select the provider, such as `mimalloc::MiMalloc` or
//! `rustfs_mimalloc::MiMalloc`. Install the sampling wrapper to observe allocations.

use rustfs_mimalloc_profiling::{mimalloc_stats, MimallocConfig, Profiler};

use crate::{
    backend::{Backend, BackendImpl, BackendUninitialized, ReportBatch, ReportData},
    error::{PyroscopeError, Result},
};

/// Configuration for the experimental Pyroscope mimalloc adapter.
///
/// Import allocator and profiler types directly from `rustfs-mimalloc-profiling`.
/// The adapter uses a larger bounded sample queue for the SDK's default
/// ten-second upload interval. Capacity is a sample count, not a throughput
/// guarantee: overload still drops samples and emits a warning on each affected report.
#[derive(Debug, Copy, Clone)]
pub struct MimallocBackendConfig {
    /// Sampling, capture, live-heap and queue settings for the profiling crate.
    /// Explicit settings are passed through unchanged.
    pub profiler: MimallocConfig,
}

impl Default for MimallocBackendConfig {
    fn default() -> Self {
        Self {
            profiler: MimallocConfig {
                // At a mean 1 MiB sample interval this accommodates roughly
                // 16 GiB of allocations per report, using about 8 MiB for stacks.
                ring_capacity: 16 * 1024,
                ..MimallocConfig::default()
            },
        }
    }
}

impl From<MimallocConfig> for MimallocBackendConfig {
    fn from(profiler: MimallocConfig) -> Self {
        Self { profiler }
    }
}

/// Create a mimalloc allocation memory profiling backend.
///
/// The returned backend should be passed to `PyroscopeAgentBuilder::new`, and
/// the process must install [`rustfs_mimalloc_profiling::SamplingAllocator`] as its global allocator.
///
/// # Examples
///
/// ```no_run
/// use pyroscope::backend::mimalloc::{
///     mimalloc_backend, MimallocBackendConfig,
/// };
/// use pyroscope::pyroscope::PyroscopeAgentBuilder;
/// use rustfs_mimalloc_profiling::SamplingAllocator;
///
/// #[global_allocator]
/// static ALLOC: SamplingAllocator<mimalloc::MiMalloc> =
///     SamplingAllocator::with_allocator(mimalloc::MiMalloc);
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let agent = PyroscopeAgentBuilder::new(
///     "http://localhost:4040",
///     "example.mimalloc",
///     100,
///     "pyroscope-rs",
///     env!("CARGO_PKG_VERSION"),
///     mimalloc_backend(MimallocBackendConfig::default()),
/// )
/// .build()?;
/// # let _ = agent;
/// # Ok(())
/// # }
/// ```
pub fn mimalloc_backend(
    config: impl Into<MimallocBackendConfig>,
) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(Mimalloc {
        config: config.into().profiler,
        profiler: None,
        losses: (0, 0),
    }))
}

#[derive(Debug)]
struct Mimalloc {
    config: MimallocConfig,
    profiler: Option<Profiler>,
    losses: (u64, u64),
}

impl Backend for Mimalloc {
    fn initialize(&mut self) -> Result<()> {
        self.profiler = Some(Profiler::start(self.config).map_err(|error| {
            PyroscopeError::new_with_source(
                &format!("mimalloc: failed to start profiling: {error}"),
                error,
            )
        })?);
        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        // Returning drops self and its Profiler, releasing recorder ownership.
        Ok(())
    }

    fn report(&mut self) -> Result<ReportBatch> {
        let profiler = self
            .profiler
            .as_mut()
            .ok_or_else(|| PyroscopeError::new("mimalloc: profiling backend is not initialized"))?;
        let bytes = profiler.report().map_err(|error| {
            PyroscopeError::new_with_source("mimalloc: failed to collect profile", error)
        })?;
        let stats = mimalloc_stats();
        let dropped = stats.dropped_samples.saturating_sub(self.losses.0);
        let dropped_live = stats.dropped_live_samples.saturating_sub(self.losses.1);
        self.losses = (stats.dropped_samples, stats.dropped_live_samples);
        if dropped != 0 || dropped_live != 0 {
            log::warn!(target: "pyroscope::backend::mimalloc",
                "Incomplete mimalloc profile: dropped {dropped} allocation samples and {dropped_live} live samples since the previous report; ring_capacity={}, report_drain_limit={}. Increase capacity or sample interval, or shorten the upload interval.",
                self.config.ring_capacity, self.config.report_drain_limit);
        }
        Ok(ReportBatch {
            profile_type: "memory".into(),
            data: ReportData::RawPprof(bytes),
        })
    }
}
