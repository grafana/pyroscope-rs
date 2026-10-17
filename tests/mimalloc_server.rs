#![cfg(feature = "backend-mimalloc")]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pyroscope::{
    backend::{
        mimalloc::{mimalloc_backend, MimallocConfig, SamplingMiMalloc},
        ReportData,
    },
    pyroscope::PyroscopeConfig,
    session::{Session, SessionManager, SessionSignal},
};

#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();

const RETAINED_SIZE: usize = 2 * 1024 * 1024;

// Rust produces real allocator samples and uploads through the SDK. The Go
// harness owns profile, query and flamegraph assertions.
#[test]
#[ignore = "requires the Go harness; run make mimalloc/test/server"]
fn upload_mimalloc_fixture() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
    let url = std::env::var("PYROSCOPE_TEST_URL").expect("test server URL");
    let service = std::env::var("PYROSCOPE_TEST_SERVICE").expect("unique test service");
    let artifacts = std::env::var_os("PYROSCOPE_TEST_ARTIFACT_DIR").expect("artifact directory");
    let artifacts = std::path::Path::new(&artifacts);
    let mut allocation_backend = mimalloc_backend(MimallocConfig {
        sample_interval_bytes: 1024,
        ..MimallocConfig::default()
    })
    .initialize()
    .unwrap();
    let allocation = allocate_server_event_marker(RETAINED_SIZE);
    let events = allocation_backend.report().unwrap();
    drop(allocation);
    // Separate timestamps and a smaller second interval expose accidental
    // cumulative subtraction in the server.
    std::thread::sleep(Duration::from_millis(2));
    let allocation = allocate_server_event_marker(64 * 1024);
    let smaller_events = allocation_backend.report().unwrap();
    allocation_backend.shutdown().unwrap();
    drop(allocation);

    let mut live_backend = mimalloc_backend(MimallocConfig {
        sample_interval_bytes: 1024,
        live_heap_tracking: true,
        ..MimallocConfig::default()
    })
    .initialize()
    .unwrap();
    let allocation = allocate_server_live_marker();
    let retained = live_backend.report().unwrap();
    std::thread::spawn(move || drop(allocation)).join().unwrap();
    // A positive control lets Go distinguish an ingested freed snapshot from
    // an empty query result.
    let freed_control = allocate_server_freed_control_marker();
    let freed = live_backend.report().unwrap();
    live_backend.shutdown().unwrap();
    drop(freed_control);

    // Stop recording before networking so SDK uploads cannot become samples.
    let manager = SessionManager::new().unwrap();
    for (phase, name, batch) in [
        ("events", "events-first-uploaded.pprof", events),
        ("events", "events-second-uploaded.pprof", smaller_events),
        ("retained", "retained-uploaded.pprof", retained),
        ("freed", "freed-uploaded.pprof", freed),
    ] {
        if let ReportData::RawPprof(bytes) = &batch.data {
            std::fs::write(artifacts.join(name), bytes).unwrap();
        }
        let config = PyroscopeConfig::new(
            &url,
            &service,
            100,
            "pyroscope-rs",
            env!("CARGO_PKG_VERSION"),
        )
        .tags(vec![("phase", phase), ("backend", "mimalloc")]);
        let until = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let session = Session::new(until, config, batch).unwrap();
        manager
            .push(SessionSignal::Session(Box::new(session)))
            .unwrap();
    }
    manager.push(SessionSignal::Kill).unwrap();
    manager.handle.unwrap().join().unwrap().unwrap();
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
