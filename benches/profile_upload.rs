//! Reproduce with `cargo bench --locked --bench profile_upload` (release profile).
//!
//! Fixtures are deterministic and contain no captured paths, addresses or payloads.
//! `PROFILE_UPLOAD_MODE=legacy|optimized|control` selects one implementation;
//! unset runs all. `PROFILE_UPLOAD_MODE=abba` runs A1/B1/B2/A2 for each input in
//! one process, sharing the same report HashMap layout. Use otherwise idle, fixed
//! resources and retain Criterion JSON and size lines. Reject baseline median
//! drift or candidate repeat variation above 5% before attributing gains.
//!
//! For a longer run: `PROFILE_UPLOAD_MODE=abba cargo bench --locked --bench
//! profile_upload -- --warm-up-time 3 --measurement-time 5 --noplot`.
//! Gzip times include output allocation/drop, but exclude protobuf encoding and
//! fixture construction. Transform times include transformation and pprof encoding;
//! cloning the starting input is setup outside timing. These do not measure network
//! latency, sampling overhead or application throughput. Compressed size can grow.
use std::{
    env,
    io::{self, Read, Write},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use prost::Message;
use pyroscope::{
    backend::{Report, Tag},
    encode::{
        gen::{
            push::{PushRequest, RawProfileSeries, RawSample},
            types::LabelPair,
        },
        pprof,
    },
};

// Benchmark the same private gzip implementation used by Session::push.
#[path = "../src/session/compression.rs"]
mod compression;
#[path = "../tests/support/profile_fixture.rs"]
mod profile_fixture;

type Encoder = fn(&[u8]) -> io::Result<Vec<u8>>;

fn control_gzip(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(input)?;
    e.finish()
}

fn modes() -> Vec<(&'static str, &'static str)> {
    match env::var("PROFILE_UPLOAD_MODE")
        .unwrap_or_else(|_| "all".into())
        .as_str()
    {
        "abba" => vec![
            ("A1-legacy", "legacy"),
            ("B1-optimized", "optimized"),
            ("B2-optimized", "optimized"),
            ("A2-legacy", "legacy"),
        ],
        "all" => vec![
            ("legacy", "legacy"),
            ("optimized", "optimized"),
            ("control", "control"),
        ],
        "legacy" => vec![("legacy", "legacy")],
        "optimized" => vec![("optimized", "optimized")],
        "control" => vec![("control", "control")],
        _ => panic!("invalid PROFILE_UPLOAD_MODE"),
    }
}

fn gzip(c: &mut Criterion) {
    for fixture in profile_fixture::profiles() {
        let request = PushRequest {
            series: vec![RawProfileSeries {
                labels: vec![
                    LabelPair {
                        name: "__name__".into(),
                        value: fixture.profile_type.into(),
                    },
                    LabelPair {
                        name: "service_name".into(),
                        value: "upload-benchmark".into(),
                    },
                ],
                samples: vec![RawSample {
                    raw_profile: fixture.bytes,
                    id: "00000000-0000-4000-8000-000000000000".into(),
                }],
            }],
        };
        let bytes = request.encode_to_vec();
        let mut group = c.benchmark_group(format!("gzip/{}", fixture.name));
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        for (label, mode) in modes() {
            let encode: Encoder = match mode {
                "legacy" => profile_fixture::legacy_gzip,
                "optimized" => compression::gzip,
                "control" => control_gzip,
                _ => unreachable!(),
            };
            let compressed = encode(&bytes).unwrap();
            let mut restored = Vec::new();
            libflate::gzip::Decoder::new(compressed.as_slice())
                .unwrap()
                .read_to_end(&mut restored)
                .unwrap();
            assert_eq!(restored, bytes);
            assert_eq!(PushRequest::decode(restored.as_slice()).unwrap(), request);
            eprintln!(
                "upload-size,{},{label},{},{}",
                fixture.name,
                bytes.len(),
                compressed.len()
            );
            group.bench_function(label, |b| {
                b.iter(|| black_box(encode(black_box(&bytes)).unwrap()))
            });
        }
        group.finish();
    }
}

fn transform(mut report: Report) -> Report {
    report
        .metadata
        .add_tag(Tag::new("transformed".into(), "true".into()));
    report
}

fn reports(c: &mut Criterion) {
    for count in [128, 2048] {
        let input = profile_fixture::structured_reports(count);
        let mut group = c.benchmark_group(format!("transform/{count}"));
        group.throughput(Throughput::Elements(count as u64));
        for (label, mode) in modes() {
            if mode == "control" {
                continue;
            }
            group.bench_function(label, |b| {
                b.iter_batched(
                    || input.clone(),
                    |reports| {
                        let reports: Vec<Report> = if mode == "legacy" {
                            reports.iter().map(|r| transform(r.to_owned())).collect()
                        } else {
                            reports.into_iter().map(transform).collect()
                        };
                        black_box(
                            pprof::encode(&reports, 100, 1_700_000_000_000_000_000, 10_000_000_000)
                                .encode_to_vec(),
                        )
                    },
                    BatchSize::PerIteration,
                )
            });
        }
        group.finish();
    }
    // Whole-process peak, including fixture construction; not a per-call RSS metric.
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        if let Some(peak) = status.lines().find(|line| line.starts_with("VmHWM:")) {
            eprintln!("upload-process-memory,{peak}");
        }
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3));
    targets = gzip, reports
}
criterion_main!(benches);
