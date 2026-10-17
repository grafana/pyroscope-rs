//! Reproduce with `cargo bench --locked --bench profile_upload` (release profile).
//!
//! Fixtures are deterministic and contain no captured paths, addresses or payloads.
//! `PROFILE_UPLOAD_MODE=legacy|optimized|control` selects one implementation;
//! unset runs all. `PROFILE_UPLOAD_MODE=abba` runs A1/B1/B2/A2 for each input in
//! one process, sharing the same report HashMap layout. Use otherwise idle, fixed
//! resources and retain Criterion JSON and size lines. Reject baseline median
//! drift or candidate repeat variation above 5% before attributing gains.
//!
//! Run the paired comparison with `PROFILE_UPLOAD_MODE=abba cargo bench --locked
//! --no-default-features --bench profile_upload -- --noplot`.
//! Gzip times include output allocation/drop, but exclude protobuf encoding and
//! fixture construction. Transform times include transformation and pprof encoding;
//! cloning the starting input is setup outside timing. These do not measure network
//! latency, sampling overhead or application throughput. Compressed size can grow.
use std::{
    env,
    hint::black_box,
    io::{self, Read, Write},
    time::Duration,
};

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use prost::Message;
use pyroscope::{
    backend::{self, Report, Tag},
    encode::{
        self,
        gen::{
            push::{PushRequest, RawProfileSeries, RawSample},
            types::LabelPair,
        },
    },
};

// Both optimized measurements call the same private helpers as Session::push.
#[path = "../src/session/encoding.rs"]
mod encoding;
#[path = "../tests/support/profile_fixture.rs"]
mod profile_fixture;

type Gzip = fn(&[u8]) -> io::Result<Vec<u8>>;
type EncodeReports = fn(Vec<Report>) -> Vec<u8>;
const START: u64 = 1_700_000_000_000_000_000;
const DURATION: u64 = 10_000_000_000;

#[derive(Clone, Copy)]
enum Mode {
    Legacy,
    Optimized,
    Control,
}

fn runs() -> &'static [(&'static str, Mode)] {
    match env::var("PROFILE_UPLOAD_MODE")
        .unwrap_or_else(|_| "all".into())
        .as_str()
    {
        "abba" => &[
            ("A1-legacy", Mode::Legacy),
            ("B1-optimized", Mode::Optimized),
            ("B2-optimized", Mode::Optimized),
            ("A2-legacy", Mode::Legacy),
        ],
        "all" => &[
            ("legacy", Mode::Legacy),
            ("optimized", Mode::Optimized),
            ("control", Mode::Control),
        ],
        "legacy" => &[("legacy", Mode::Legacy)],
        "optimized" => &[("optimized", Mode::Optimized)],
        "control" => &[("control", Mode::Control)],
        _ => panic!("invalid PROFILE_UPLOAD_MODE"),
    }
}

fn control_gzip(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input)?;
    encoder.finish()
}

fn gzip(c: &mut Criterion) {
    let runs = runs();
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
        for &(label, mode) in runs {
            let compress: Gzip = match mode {
                Mode::Legacy => profile_fixture::legacy_gzip,
                Mode::Optimized => encoding::gzip,
                Mode::Control => control_gzip,
            };
            // Verification buffers are dropped before timing.
            {
                let compressed = compress(&bytes).unwrap();
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
            }
            group.bench_function(label, |b| {
                b.iter(|| black_box(compress(black_box(&bytes)).unwrap()))
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

fn legacy_reports(reports: Vec<Report>) -> Vec<u8> {
    let transformed: Vec<_> = reports.iter().map(|r| transform(r.to_owned())).collect();
    encode::pprof::encode(&transformed, 100, START, DURATION).encode_to_vec()
}

fn optimized_reports(reports: Vec<Report>) -> Vec<u8> {
    encoding::encode_reports(reports, Some(transform), 100, START, DURATION)
}

fn reports(c: &mut Criterion) {
    let runs = runs();
    for count in [128, 2048] {
        let input = profile_fixture::structured_reports(count);
        let mut group = c.benchmark_group(format!("transform/{count}"));
        group.throughput(Throughput::Elements(count as u64));
        for &(label, mode) in runs {
            let encode: EncodeReports = match mode {
                Mode::Legacy => legacy_reports,
                Mode::Optimized => optimized_reports,
                Mode::Control => continue,
            };
            group.bench_function(label, |b| {
                b.iter_batched(
                    || input.clone(),
                    |reports| black_box(encode(black_box(reports))),
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
    config = Criterion::default().sample_size(30).warm_up_time(Duration::from_secs(10)).measurement_time(Duration::from_secs(10));
    targets = gzip, reports
}
criterion_main!(benches);
