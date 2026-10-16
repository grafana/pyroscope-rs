//! Pyroscope adapter for the standalone `rustfs-mimalloc-profiling` crate.
//!
//! Install [`SamplingMiMalloc`] as the global allocator, then pass
//! [`mimalloc_backend`] to the agent builder. Sampling, live heap tracking,
//! symbolization and pprof generation are owned by the profiling crate.

use rustfs_mimalloc_profiling::Profiler;
pub use rustfs_mimalloc_profiling::{
    mimalloc_stats, MimallocConfig, MimallocStackCapture, MimallocStats, SamplingMiMalloc,
};

use crate::{
    backend::{Backend, BackendImpl, BackendUninitialized, ReportBatch, ReportData, ThreadTag},
    error::{PyroscopeError, Result},
};

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
    BackendImpl::new(Box::new(Mimalloc {
        config,
        profiler: None,
    }))
}

#[derive(Debug)]
struct Mimalloc {
    config: MimallocConfig,
    profiler: Option<Profiler>,
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
        // Profiler::drop stops recording and releases recorder ownership.
        drop(self);
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
        Ok(ReportBatch {
            profile_type: "memory".into(),
            data: ReportData::RawPprof(bytes),
        })
    }

    fn add_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }

    fn remove_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }
}
