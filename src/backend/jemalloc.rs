use crate::backend::{Backend, BackendImpl, BackendUninitialized, Report, ReportBatch, ThreadTag};
use crate::error::{PyroscopeError, Result};

const LOG_TAG: &str = "Pyroscope::Jemalloc";

/// Create a jemalloc memory profiling backend.
///
/// # Example
///
/// ```ignore
/// use pyroscope::PyroscopeAgent;
/// use pyroscope::backend::jemalloc::{jemalloc_backend, JemallocConfig};
///
/// let agent = PyroscopeAgent::builder("http://localhost:4040", "my-app")
///     .backend(jemalloc_backend(JemallocConfig::default()))
///     .build()?;
/// agent.start()?;
/// ```
pub fn jemalloc_backend(config: JemallocConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(Jemalloc::new(config)))
}

#[derive(Debug, Default)]
pub struct JemallocConfig {}

struct Jemalloc {
    _config: JemallocConfig,
}

impl Jemalloc {
    fn new(config: JemallocConfig) -> Self {
        Self { _config: config }
    }
}

impl std::fmt::Debug for Jemalloc {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Jemalloc Backend")
    }
}

impl Backend for Jemalloc {
    fn initialize(&mut self) -> Result<()> {
        let prof_ctl = jemalloc_pprof::PROF_CTL.as_ref().ok_or_else(|| {
            PyroscopeError::new(
                "jemalloc: PROF_CTL not available. Ensure jemalloc is configured with prof:true",
            )
        })?;
        let guard = prof_ctl.try_lock().map_err(|_| {
            PyroscopeError::new(
                "jemalloc: failed to acquire PROF_CTL lock during initialization. \
                 This is unexpected at startup; ensure no other code holds the lock.",
            )
        })?;
        if !guard.activated() {
            return Err(PyroscopeError::new(
                "jemalloc: profiling is not activated. Ensure malloc_conf includes prof:true,prof_active:true",
            ));
        }

        log::info!(target: LOG_TAG, "Jemalloc profiling backend initialized");

        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        log::trace!(target: LOG_TAG, "Shutting down jemalloc backend");
        Ok(())
    }

    fn report(&mut self) -> Result<ReportBatch> {
        let prof_ctl = jemalloc_pprof::PROF_CTL
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("jemalloc: PROF_CTL not available"))?;
        let mut guard = prof_ctl.blocking_lock();
        let pprof_data = guard
            .dump_pprof()
            .map_err(|e| PyroscopeError::new(&format!("jemalloc: dump_pprof failed: {}", e)))?;

        Ok(ReportBatch {
            profile_type: "memory".into(),
            reports: vec![Report::from_raw_pprof(pprof_data)],
        })
    }

    fn add_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }

    fn remove_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }
}
