//! Deterministic, public pprof fixtures shared by upload tests and benchmarks.
use std::io::{self, Write};

use prost::Message;
use pyroscope::{
    backend::{Report, StackFrame, StackTrace},
    encode::gen::google::{Function, Label, Line, Location, Mapping, Profile, Sample, ValueType},
};

pub struct Fixture {
    pub name: &'static str,
    pub profile_type: &'static str,
    pub bytes: Vec<u8>,
}

pub fn profiles() -> Vec<Fixture> {
    let large = profile(ProfileKind::Memory, 32_768, false).encode_to_vec();
    let precompressed = legacy_gzip(&large).unwrap();
    vec![
        Fixture {
            name: "empty",
            profile_type: "memory",
            bytes: Profile {
                string_table: vec![String::new()],
                ..Profile::default()
            }
            .encode_to_vec(),
        },
        Fixture {
            name: "memory-small",
            profile_type: "memory",
            bytes: profile(ProfileKind::Memory, 64, false).encode_to_vec(),
        },
        Fixture {
            name: "memory-large",
            profile_type: "memory",
            bytes: large,
        },
        Fixture {
            name: "memory-high-entropy",
            profile_type: "memory",
            bytes: profile(ProfileKind::Memory, 32_768, true).encode_to_vec(),
        },
        Fixture {
            name: "memory-precompressed",
            profile_type: "memory",
            bytes: precompressed,
        },
        Fixture {
            name: "cpu-small",
            profile_type: "process_cpu",
            bytes: profile(ProfileKind::Cpu, 64, false).encode_to_vec(),
        },
    ]
}

// The pre-change encoder, retained only for independent tests and comparisons.
pub fn legacy_gzip(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = libflate::gzip::Encoder::new(Vec::new())?;
    encoder.write_all(input)?;
    encoder.finish().into_result()
}

#[derive(Clone, Copy)]
enum ProfileKind {
    Memory,
    Cpu,
}

fn string(profile: &mut Profile, value: &str) -> i64 {
    if let Some(index) = profile.string_table.iter().position(|s| s == value) {
        return index as i64;
    }
    let index = profile.string_table.len() as i64;
    profile.string_table.push(value.to_owned());
    index
}

fn profile(kind: ProfileKind, count: usize, high_entropy: bool) -> Profile {
    let mut p = Profile {
        string_table: vec![String::new()],
        time_nanos: 1_700_000_000_000_000_000,
        duration_nanos: 10_000_000_000,
        ..Profile::default()
    };
    let types: &[(&str, &str)] = match kind {
        ProfileKind::Memory => &[
            ("alloc_objects", "count"),
            ("alloc_space", "bytes"),
            ("inuse_objects", "count"),
            ("inuse_space", "bytes"),
        ],
        ProfileKind::Cpu => &[("cpu", "nanoseconds")],
    };
    for &(name, unit) in types {
        let r#type = string(&mut p, name);
        let unit = string(&mut p, unit);
        p.sample_type.push(ValueType { r#type, unit });
    }
    p.default_sample_type = p.sample_type.last().unwrap().r#type;
    let (period_name, period_unit, period) = match kind {
        ProfileKind::Memory => ("space", "bytes", 524_288),
        ProfileKind::Cpu => ("cpu", "nanoseconds", 10_000_000),
    };
    let r#type = string(&mut p, period_name);
    let unit = string(&mut p, period_unit);
    p.period_type = Some(ValueType { r#type, unit });
    p.period = period;
    let filename = string(&mut p, "src/workload.rs");
    let executable = string(&mut p, "synthetic-workload");
    let build_id = string(&mut p, "0123456789abcdef");
    let delta_key = string(&mut p, "__delta__");
    let delta_value = string(&mut p, "false");
    p.mapping.push(Mapping {
        id: 1,
        memory_start: 0x1000,
        memory_limit: 0x1800,
        filename: executable,
        build_id,
        has_functions: true,
        has_filenames: true,
        has_line_numbers: true,
        ..Mapping::default()
    });
    for i in 0..128u64 {
        let name = string(&mut p, &format!("workload::function_{i}"));
        p.function.push(Function {
            id: i + 1,
            name,
            system_name: name,
            filename,
            start_line: 10,
        });
        p.location.push(Location {
            id: i + 1,
            mapping_id: 1,
            address: 0x1000 + i * 16,
            line: vec![Line {
                function_id: i + 1,
                line: 42 + i as i64,
            }],
            is_folded: false,
        });
    }
    let mut random = 0x5865727a55a891f3u64;
    for i in 0..count {
        let mut values = match kind {
            ProfileKind::Memory => vec![
                1 + (i % 7) as i64,
                64 + (i * 127 % 8192) as i64,
                (i % 3) as i64,
                (i * 61 % 4096) as i64,
            ],
            ProfileKind::Cpu => vec![(1 + (i % 7) as i64) * period],
        };
        if high_entropy {
            for value in &mut values {
                random ^= random << 13;
                random ^= random >> 7;
                random ^= random << 17;
                *value = (random >> 1) as i64;
            }
        }
        p.sample.push(Sample {
            location_id: (0..16)
                .map(|depth| ((i + depth) % 128 + 1) as u64)
                .collect(),
            value: values,
            label: vec![Label {
                key: delta_key,
                str: delta_value,
                ..Label::default()
            }],
        });
    }
    p
}

pub fn structured_reports(stacks: usize) -> Vec<Report> {
    let mut report = Report::default();
    for i in 0..stacks {
        let frames = (0..16)
            .map(|depth| StackFrame {
                name: Some(format!("workload::stack_{i}_frame_{depth}")),
                filename: Some("src/workload.rs".into()),
                line: Some(42 + depth),
                ..StackFrame::default()
            })
            .collect();
        report.record_with_count(
            StackTrace {
                frames,
                ..StackTrace::default()
            },
            1 + i % 7,
        );
    }
    vec![report]
}
