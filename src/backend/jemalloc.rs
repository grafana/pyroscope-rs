use crate::backend::{Backend, BackendImpl, BackendUninitialized, Report, ThreadTag};
use crate::error::{PyroscopeError, Result};

const LOG_TAG: &str = "Pyroscope::Jemalloc";

pub fn jemalloc_backend(config: JemallocConfig) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(Jemalloc::new(config)))
}

#[derive(Debug, Default)]
pub struct JemallocConfig {}

struct Jemalloc {
    _config: JemallocConfig,
    runtime: Option<tokio::runtime::Runtime>,
}

impl Jemalloc {
    fn new(config: JemallocConfig) -> Self {
        Self {
            _config: config,
            runtime: None,
        }
    }
}

impl std::fmt::Debug for Jemalloc {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Jemalloc Backend")
    }
}

impl Backend for Jemalloc {
    fn initialize(&mut self) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|e| PyroscopeError::new(&format!("jemalloc: failed to create tokio runtime: {}", e)))?;

        // Verify PROF_CTL is available and activated
        runtime.block_on(async {
            let prof_ctl = jemalloc_pprof::PROF_CTL
                .as_ref()
                .ok_or_else(|| PyroscopeError::new("jemalloc: PROF_CTL not available. Ensure jemalloc is configured with prof:true"))?;
            let guard = prof_ctl.lock().await;
            if !guard.activated() {
                return Err(PyroscopeError::new("jemalloc: profiling is not activated. Ensure malloc_conf includes prof:true,prof_active:true"));
            }
            Ok(())
        })?;

        self.runtime = Some(runtime);
        log::info!(target: LOG_TAG, "Jemalloc profiling backend initialized");

        Ok(())
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        log::trace!(target: LOG_TAG, "Shutting down jemalloc backend");
        // runtime is dropped here
        Ok(())
    }

    fn report(&mut self) -> Result<Vec<Report>> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("jemalloc: runtime not initialized"))?;

        let pprof_data = runtime.block_on(async {
            let prof_ctl = jemalloc_pprof::PROF_CTL
                .as_ref()
                .ok_or_else(|| PyroscopeError::new("jemalloc: PROF_CTL not available"))?;
            let mut guard = prof_ctl.lock().await;
            guard
                .dump_pprof()
                .map_err(|e| PyroscopeError::new(&format!("jemalloc: dump_pprof failed: {}", e)))
        })?;

        Ok(vec![Report::from_raw_pprof(pprof_data)])
    }

    fn add_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }

    fn remove_tag(&self, _tag: ThreadTag) -> Result<()> {
        Ok(())
    }
}
