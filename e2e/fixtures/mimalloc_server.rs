use std::{path::PathBuf, time::Duration};

use pyroscope::{
    backend::{
        mimalloc::{mimalloc_backend, MimallocBackendConfig},
        Backend, BackendImpl, ReportBatch, ReportData, ThreadTag,
    },
    pyroscope::PyroscopeAgentBuilder,
};
use rustfs_mimalloc_profiling::MimallocConfig;

const RETAINED_SIZE: usize = 2 * 1024 * 1024;

// Capture the exact bytes produced by the real backend while exercising the
// public agent lifecycle and upload pipeline. Go owns all profile assertions.
struct CaptureBackend {
    inner: Box<dyn Backend>,
    artifacts: PathBuf,
    name: &'static str,
    reports: usize,
}

impl Backend for CaptureBackend {
    fn initialize(&mut self) -> pyroscope::Result<()> {
        self.inner.initialize()
    }
    fn shutdown(self: Box<Self>) -> pyroscope::Result<()> {
        self.inner.shutdown()
    }
    fn add_tag(&self, tag: ThreadTag) -> pyroscope::Result<()> {
        self.inner.add_tag(tag)
    }
    fn remove_tag(&self, tag: ThreadTag) -> pyroscope::Result<()> {
        self.inner.remove_tag(tag)
    }
    fn report(&mut self) -> pyroscope::Result<ReportBatch> {
        let batch = self.inner.report()?;
        if let ReportData::RawPprof(bytes) = &batch.data {
            let name = format!("{}-{}-uploaded.pprof", self.name, self.reports);
            std::fs::write(self.artifacts.join(name), bytes)?;
            self.reports += 1;
        }
        Ok(batch)
    }
}

#[test]
#[ignore = "requires the Go harness; run make mimalloc/test/server"]
fn upload_mimalloc_fixture() {
    let url = std::env::var("PYROSCOPE_TEST_URL").expect("test server URL");
    let service = std::env::var("PYROSCOPE_TEST_SERVICE").expect("unique test service");
    let artifacts =
        PathBuf::from(std::env::var_os("PYROSCOPE_TEST_ARTIFACT_DIR").expect("artifact directory"));
    for (name, phase) in [
        ("events-first", "events"),
        ("events-second", "events"),
        ("retained", "retained"),
        ("freed", "freed"),
    ] {
        let config = MimallocBackendConfig::from(MimallocConfig {
            sample_interval_bytes: 1024,
            live_heap_tracking: phase != "events",
            ..MimallocBackendConfig::default().profiler
        });
        let backend = mimalloc_backend(config);
        let inner = backend.backend.lock().unwrap().take().unwrap();
        let backend = BackendImpl::new(Box::new(CaptureBackend {
            inner,
            artifacts: artifacts.clone(),
            name,
            reports: 0,
        }));
        let agent = PyroscopeAgentBuilder::new(
            &url,
            &service,
            100,
            "pyroscope-rs",
            env!("CARGO_PKG_VERSION"),
            backend,
        )
        .tags(vec![("phase", phase), ("backend", "mimalloc")])
        .upload_interval(Duration::from_secs(1))
        .build()
        .expect("initialize real agent and profiler");
        let retained = match name {
            "events-first" => allocate_server_event_marker(RETAINED_SIZE),
            "events-second" => allocate_server_event_marker(64 * 1024),
            "retained" => allocate_server_live_marker(),
            "freed" => {
                let allocation = allocate_server_live_marker();
                std::thread::spawn(move || drop(allocation)).join().unwrap();
                allocate_server_freed_control_marker()
            }
            _ => unreachable!(),
        };
        // stop() queues the final snapshot through the actual worker, and
        // shutdown() joins the uploader. Capture every timer/final snapshot;
        // Go merges them, so timer-boundary races cannot hide/duplicate data.
        let running = agent.start().expect("start agent");
        let ready = running.stop().expect("flush final profile");
        ready.shutdown();
        std::hint::black_box(&retained);
    }
}

#[inline(never)]
fn allocate_server_event_marker(size: usize) -> Vec<u8> {
    let allocation = vec![0; size];
    std::hint::black_box(&allocation);
    allocation
}
#[inline(never)]
fn allocate_server_live_marker() -> Vec<u8> {
    let allocation = vec![0; RETAINED_SIZE];
    std::hint::black_box(&allocation);
    allocation
}
#[inline(never)]
fn allocate_server_freed_control_marker() -> Vec<u8> {
    let allocation = vec![0; 64 * 1024];
    std::hint::black_box(&allocation);
    allocation
}
