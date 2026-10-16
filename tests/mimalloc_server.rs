#![cfg(feature = "backend-mimalloc")]

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use prost::Message;
use pyroscope::{
    backend::{
        mimalloc::{mimalloc_backend, MimallocConfig, SamplingMiMalloc},
        ReportData,
    },
    encode::gen::google::{Location, Profile},
    pyroscope::PyroscopeConfig,
    session::{Session, SessionManager, SessionSignal},
};
use reqwest::blocking::Client;
use serde_json::{json, Value};

#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();

const ALLOCATION_MARKER: &str = "allocate_server_event_marker";
const LIVE_MARKER: &str = "allocate_server_live_marker";
const FREED_CONTROL_MARKER: &str = "allocate_server_freed_control_marker";
const RETAINED_SIZE: usize = 2 * 1024 * 1024;

#[test]
#[ignore = "requires PYROSCOPE_TEST_URL; run make mimalloc/test/server"]
fn mimalloc_reports_round_trip_through_pyroscope_server() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
    let url = std::env::var("PYROSCOPE_TEST_URL").expect("set PYROSCOPE_TEST_URL to a test server");
    let url = url.trim_end_matches('/');
    let service = format!("mimalloc-e2e-{}", uuid::Uuid::new_v4());
    let mut allocation_backend = mimalloc_backend(MimallocConfig {
        sample_interval_bytes: 1024,
        ..MimallocConfig::default()
    })
    .initialize()
    .unwrap();
    let allocation = allocate_server_event_marker(RETAINED_SIZE);
    let events = allocation_backend.report().unwrap();
    drop(allocation);
    // Distinct timestamps and a smaller second interval catch accidental
    // cumulative subtraction, not just the first-profile baseline drop.
    std::thread::sleep(Duration::from_millis(2));
    let allocation = allocate_server_event_marker(64 * 1024);
    let smaller_events = allocation_backend.report().unwrap();
    allocation_backend.shutdown().unwrap();
    drop(allocation);
    let event_totals = ["alloc_objects", "alloc_space"].map(|kind| {
        [&events, &smaller_events]
            .iter()
            .map(|batch| {
                let ReportData::RawPprof(bytes) = &batch.data else {
                    unreachable!()
                };
                marker_value(
                    &Profile::decode(bytes.as_slice()).unwrap(),
                    ALLOCATION_MARKER,
                    kind,
                )
            })
            .sum::<i64>()
    });

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
    // A positive control proves the freed phase reached storage. An empty
    // query alone must not be mistaken for successful cross-thread removal.
    let freed_control = allocate_server_freed_control_marker();
    let freed = live_backend.report().unwrap();
    live_backend.shutdown().unwrap();
    drop(freed_control);

    // Capture before networking and shut down the recorder so requests and
    // query decoding cannot become profiler-generated allocation samples.
    let manager = SessionManager::new().unwrap();
    for (phase, batch) in [
        ("events", events),
        ("events", smaller_events),
        ("retained", retained),
        ("freed", freed),
    ] {
        let ReportData::RawPprof(ref bytes) = batch.data else {
            panic!("raw pprof expected");
        };
        let input = Profile::decode(bytes.as_slice()).unwrap();
        let marker = match phase {
            "events" => ALLOCATION_MARKER,
            "retained" => LIVE_MARKER,
            "freed" => FREED_CONTROL_MARKER,
            _ => unreachable!(),
        };
        assert!(assert_source_metadata(&input, marker).address > 0);
        assert!(marker_value(&input, marker, "alloc_space") > 0);
        write_artifact(
            &format!("{phase}-{}-uploaded.pprof", input.time_nanos),
            bytes,
        );
        let config = PyroscopeConfig::new(
            url,
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

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let types = query_json(
        &client,
        url,
        "ProfileTypes",
        json!({"start": now - 60_000, "end": now + 60_000}),
    );
    write_artifact(
        "profile-types.json",
        &serde_json::to_vec_pretty(&types).unwrap(),
    );
    println!("server profile types: {types}");
    for (phase, kind, marker, minimum) in [
        (
            "events",
            "alloc_objects",
            ALLOCATION_MARKER,
            event_totals[0],
        ),
        ("events", "alloc_space", ALLOCATION_MARKER, event_totals[1]),
        ("retained", "inuse_objects", LIVE_MARKER, 1),
        ("retained", "inuse_space", LIVE_MARKER, RETAINED_SIZE as i64),
    ] {
        let unit = if kind.ends_with("objects") {
            "count"
        } else {
            "bytes"
        };
        let type_id = format!("memory:{kind}:{unit}:space:bytes");
        let selector =
            format!("{{service_name=\"{service}\",phase=\"{phase}\",backend=\"mimalloc\"}}");
        let request = MergeRequest {
            profile_type_id: type_id.clone(),
            label_selector: selector.clone(),
            start: now - 60_000,
            end: now + 60_000,
            format: 4,
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        let diagnostic = query_json(
            &client,
            url,
            "SelectMergeStacktraces",
            json!({
                "profileTypeID": type_id,
                "labelSelector": selector,
                "start": request.start,
                "end": request.end,
            }),
        );
        write_artifact(
            &format!("{phase}-{kind}-diagnostic.json"),
            &serde_json::to_vec_pretty(&diagnostic).unwrap(),
        );
        let profile = loop {
            let profile = query_pprof(&client, url, &request);
            if marker_value(&profile, marker, kind) >= minimum {
                break profile;
            }
            assert!(
                Instant::now() < deadline,
                "server did not ingest {phase}/{kind}"
            );
            std::thread::sleep(Duration::from_millis(200));
        };
        assert_eq!(marker_value(&profile, marker, kind), minimum);
        assert_source_metadata(&profile, marker);
        write_artifact(
            &format!("{phase}-{kind}-queried.pprof"),
            &profile.encode_to_vec(),
        );

        let flamegraph = query_json(
            &client,
            url,
            "SelectMergeStacktraces",
            json!({
                "profileTypeID": type_id,
                "labelSelector": selector,
                "start": request.start,
                "end": request.end,
            }),
        );
        assert!(
            flamegraph["flamegraph"]["names"]
                .as_array()
                .unwrap()
                .iter()
                .any(|name| { name.as_str().is_some_and(|name| name.contains(marker)) }),
            "application marker missing from {phase}/{kind} flamegraph"
        );
        write_artifact(
            &format!("{phase}-{kind}-flamegraph.json"),
            &serde_json::to_vec_pretty(&flamegraph).unwrap(),
        );
        println!(
            "verified phase={phase} profile_type={type_id} marker_value={}",
            marker_value(&profile, marker, kind)
        );

        // Use a separate phase selector: merging retained and freed snapshots
        // would keep the earlier live object in the aggregate by design.
        if kind.starts_with("inuse") {
            let freed_request = MergeRequest {
                label_selector: format!(
                    "{{service_name=\"{service}\",phase=\"freed\",backend=\"mimalloc\"}}"
                ),
                ..request
            };
            let deadline = Instant::now() + Duration::from_secs(30);
            let freed_profile = loop {
                let profile = query_pprof(&client, url, &freed_request);
                if marker_value(&profile, FREED_CONTROL_MARKER, kind) > 0 {
                    break profile;
                }
                assert!(
                    Instant::now() < deadline,
                    "freed phase positive control was not ingested"
                );
                std::thread::sleep(Duration::from_millis(200));
            };
            assert_eq!(marker_value(&freed_profile, LIVE_MARKER, kind), 0);
            write_artifact(
                &format!("freed-{kind}-queried.pprof"),
                &freed_profile.encode_to_vec(),
            );
        }
    }
    let types = query_json(
        &client,
        url,
        "ProfileTypes",
        json!({"start": now - 60_000, "end": now + 60_000}),
    );
    let types_json = serde_json::to_vec_pretty(&types).unwrap();
    write_artifact("profile-types.json", &types_json);
    for kind in [
        "alloc_objects",
        "alloc_space",
        "inuse_objects",
        "inuse_space",
    ] {
        assert!(
            types["profileTypes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| {
                    entry["ID"]
                        .as_str()
                        .is_some_and(|id| id.starts_with(&format!("memory:{kind}:")))
                }),
            "profile type {kind} not advertised by server"
        );
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

fn query_json(client: &Client, url: &str, method: &str, body: Value) -> Value {
    let response = client
        .post(format!("{url}/querier.v1.QuerierService/{method}"))
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap();
    serde_json::from_slice(&response).unwrap()
}

fn query_pprof(client: &Client, url: &str, request: &MergeRequest) -> Profile {
    let response = client
        .post(format!(
            "{url}/querier.v1.QuerierService/SelectMergeStacktraces"
        ))
        .header("Content-Type", "application/proto")
        .header("Accept-Encoding", "identity")
        .body(request.encode_to_vec())
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap();
    MergeResponse::decode(response)
        .unwrap()
        .pprof
        .and_then(|pprof| pprof.profile)
        .unwrap_or_default()
}

fn marker_value(profile: &Profile, marker: &str, kind: &str) -> i64 {
    let Some(index) = profile
        .sample_type
        .iter()
        .position(|kind_type| profile.string_table[kind_type.r#type as usize] == kind)
    else {
        return 0;
    };
    profile
        .sample
        .iter()
        .filter(|sample| {
            sample.location_id.iter().any(|id| {
                profile
                    .location
                    .iter()
                    .filter(|location| location.id == *id)
                    .any(|location| {
                        location.line.iter().any(|line| {
                            profile.function.iter().any(|function| {
                                function.id == line.function_id
                                    && profile.string_table[function.name as usize].contains(marker)
                            })
                        })
                    })
            })
        })
        .map(|sample| sample.value[index])
        .sum()
}

fn assert_source_metadata<'a>(profile: &'a Profile, marker: &str) -> &'a Location {
    let function = profile
        .function
        .iter()
        .find(|function| profile.string_table[function.name as usize].contains(marker))
        .expect("application function in server pprof");
    assert!(profile.string_table[function.filename as usize].ends_with("mimalloc_server.rs"));
    assert!(!profile.string_table[function.system_name as usize].is_empty());
    let location = profile
        .location
        .iter()
        .find(|location| {
            location
                .line
                .iter()
                .any(|line| line.function_id == function.id && line.line > 0)
        })
        .expect("application source line in server pprof");
    // Pyroscope intentionally clears addresses for fully symbolized mappings.
    // Assert runtime addresses on the uploaded input, not after normalization.
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
    {
        let mapping = profile
            .mapping
            .iter()
            .find(|mapping| mapping.id == location.mapping_id)
            .expect("application executable mapping");
        assert!(!profile.string_table[mapping.build_id as usize].is_empty());
    }
    location
}

fn write_artifact(name: &str, bytes: &[u8]) {
    if let Some(directory) = std::env::var_os("PYROSCOPE_TEST_ARTIFACT_DIR") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join(name), bytes).unwrap();
    }
}

// Minimal wire view of the public v2.4.0 querier API. Reuse the crate's actual
// pprof schema so server-returned symbol and mapping metadata is decoded too.
#[derive(Message)]
struct MergeRequest {
    #[prost(string, tag = "1")]
    profile_type_id: String,
    #[prost(string, tag = "2")]
    label_selector: String,
    #[prost(int64, tag = "3")]
    start: i64,
    #[prost(int64, tag = "4")]
    end: i64,
    #[prost(int32, tag = "6")]
    format: i32,
}

#[derive(Message)]
struct MergeResponse {
    #[prost(message, optional, tag = "5")]
    pprof: Option<PprofResponse>,
}

#[derive(Message)]
struct PprofResponse {
    #[prost(message, optional, tag = "1")]
    profile: Option<Profile>,
}
