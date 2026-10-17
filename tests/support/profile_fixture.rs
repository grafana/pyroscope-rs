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
    let large = memory_profile(32_768, false).encode_to_vec();
    let precompressed = legacy_gzip(&large).unwrap();
    let mut cpu = memory_profile(64, false);
    cpu.sample_type = vec![ValueType {
        r#type: 10,
        unit: 11,
    }];
    cpu.period_type = Some(ValueType {
        r#type: 10,
        unit: 11,
    });
    cpu.period = 10_000_000;
    cpu.default_sample_type = 10;
    for sample in &mut cpu.sample {
        sample.value = vec![sample.value[0] * cpu.period];
    }
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
            bytes: memory_profile(64, false).encode_to_vec(),
        },
        Fixture {
            name: "memory-large",
            profile_type: "memory",
            bytes: large,
        },
        Fixture {
            name: "memory-high-entropy",
            profile_type: "memory",
            bytes: memory_profile(32_768, true).encode_to_vec(),
        },
        Fixture {
            name: "memory-precompressed",
            profile_type: "memory",
            bytes: precompressed,
        },
        Fixture {
            name: "cpu-small",
            profile_type: "process_cpu",
            bytes: cpu.encode_to_vec(),
        },
    ]
}

// The pre-change encoder, retained only for independent tests and comparisons.
pub fn legacy_gzip(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = libflate::gzip::Encoder::new(Vec::new())?;
    encoder.write_all(input)?;
    encoder.finish().into_result()
}

fn memory_profile(count: usize, high_entropy: bool) -> Profile {
    let mut p = Profile {
        string_table: [
            "",
            "alloc_objects",
            "count",
            "alloc_space",
            "bytes",
            "inuse_objects",
            "inuse_space",
            "space",
            "__delta__",
            "false",
            "cpu",
            "nanoseconds",
            "src/workload.rs",
            "synthetic-workload",
            "0123456789abcdef",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        sample_type: vec![
            ValueType { r#type: 1, unit: 2 },
            ValueType { r#type: 3, unit: 4 },
            ValueType { r#type: 5, unit: 2 },
            ValueType { r#type: 6, unit: 4 },
        ],
        mapping: vec![Mapping {
            id: 1,
            memory_start: 0x1000,
            memory_limit: 0x1800,
            filename: 13,
            build_id: 14,
            has_functions: true,
            has_filenames: true,
            has_line_numbers: true,
            ..Mapping::default()
        }],
        period_type: Some(ValueType { r#type: 7, unit: 4 }),
        period: 524_288,
        time_nanos: 1_700_000_000_000_000_000,
        duration_nanos: 10_000_000_000,
        default_sample_type: 6,
        ..Profile::default()
    };
    for i in 0..128u64 {
        let name = p.string_table.len() as i64;
        p.string_table.push(format!("workload::function_{i}"));
        p.function.push(Function {
            id: i + 1,
            name,
            system_name: name,
            filename: 12,
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
        let mut values = vec![
            1 + (i % 7) as i64,
            64 + (i * 127 % 8192) as i64,
            (i % 3) as i64,
            (i * 61 % 4096) as i64,
        ];
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
                key: 8,
                str: 9,
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
