use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use prost::Message;

use crate::encode::gen::google::{Function, Line, Location, Profile, Sample, ValueType};

/// A memory allocation sample ready to be encoded into pprof.
#[derive(Debug, Clone)]
pub struct AllocationSample {
    pub frames: Vec<String>,
    pub alloc_objects: i64,
    pub alloc_space: i64,
}

impl AllocationSample {
    pub fn new(frames: Vec<String>, alloc_objects: i64, alloc_space: i64) -> Self {
        Self {
            frames,
            alloc_objects,
            alloc_space,
        }
    }
}

struct PprofMemoryBuilder {
    profile: Profile,
    strings: HashMap<String, i64>,
    functions: HashMap<String, u64>,
    locations: HashMap<String, u64>,
}

impl PprofMemoryBuilder {
    fn new(period: i64, duration_nanos: i64) -> Self {
        let mut builder = Self {
            profile: Profile {
                sample_type: vec![],
                sample: vec![],
                mapping: vec![],
                location: vec![],
                function: vec![],
                string_table: vec![],
                drop_frames: 0,
                keep_frames: 0,
                time_nanos: now_nanos(),
                duration_nanos,
                period_type: None,
                period,
                comment: vec![],
                default_sample_type: 0,
            },
            strings: HashMap::new(),
            functions: HashMap::new(),
            locations: HashMap::new(),
        };

        builder.add_string("");
        let alloc_objects = builder.add_string("alloc_objects");
        let alloc_space = builder.add_string("alloc_space");
        let count = builder.add_string("count");
        let bytes = builder.add_string("bytes");
        let space = builder.add_string("space");

        builder.profile.sample_type.push(ValueType {
            r#type: alloc_objects,
            unit: count,
        });
        builder.profile.sample_type.push(ValueType {
            r#type: alloc_space,
            unit: bytes,
        });
        builder.profile.period_type = Some(ValueType {
            r#type: space,
            unit: bytes,
        });
        builder.profile.default_sample_type = alloc_space;

        builder
    }

    fn add_string(&mut self, value: &str) -> i64 {
        if let Some(id) = self.strings.get(value) {
            return *id;
        }

        let id = self.profile.string_table.len() as i64;
        self.strings.insert(value.to_owned(), id);
        self.profile.string_table.push(value.to_owned());
        id
    }

    fn add_frame(&mut self, name: &str) -> u64 {
        if let Some(location_id) = self.locations.get(name) {
            return *location_id;
        }

        let name_id = self.add_string(name);
        let function_id = if let Some(function_id) = self.functions.get(name) {
            *function_id
        } else {
            let function_id = self.profile.function.len() as u64 + 1;
            self.profile.function.push(Function {
                id: function_id,
                name: name_id,
                system_name: 0,
                filename: 0,
                start_line: 0,
            });
            self.functions.insert(name.to_owned(), function_id);
            function_id
        };

        let location_id = self.profile.location.len() as u64 + 1;
        self.profile.location.push(Location {
            id: location_id,
            mapping_id: 0,
            address: 0,
            line: vec![Line {
                function_id,
                line: 0,
            }],
            is_folded: false,
        });
        self.locations.insert(name.to_owned(), location_id);
        location_id
    }

    fn add_sample(&mut self, sample: &AllocationSample) {
        if sample.alloc_objects <= 0 || sample.alloc_space <= 0 {
            return;
        }

        let location_id = sample
            .frames
            .iter()
            .map(|frame| self.add_frame(frame))
            .collect();

        self.profile.sample.push(Sample {
            location_id,
            value: vec![sample.alloc_objects, sample.alloc_space],
            label: vec![],
        });
    }
}

pub fn encode_allocation_profile(
    samples: &[AllocationSample],
    period: u64,
    duration_nanos: i64,
) -> Vec<u8> {
    let period = i64::try_from(period).unwrap_or(i64::MAX);
    let mut builder = PprofMemoryBuilder::new(period, duration_nanos);

    for sample in samples {
        builder.add_sample(sample);
    }

    builder.profile.encode_to_vec()
}

fn now_nanos() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;
    use crate::encode::gen::google::Profile;

    #[test]
    fn encode_allocation_profile_uses_memory_sample_types() {
        let bytes = encode_allocation_profile(
            &[AllocationSample::new(
                vec!["sampled_mimalloc_allocation".to_string()],
                7,
                4096,
            )],
            1024 * 1024,
            10_000,
        );
        let profile = Profile::decode(bytes.as_slice()).expect("decode memory pprof");

        assert!(profile.string_table.iter().any(|s| s == "alloc_objects"));
        assert!(profile.string_table.iter().any(|s| s == "alloc_space"));
        assert!(profile.string_table.iter().any(|s| s == "bytes"));
        assert!(!profile.string_table.iter().any(|s| s == "nanoseconds"));
        assert_eq!(profile.sample.len(), 1);
        assert_eq!(profile.sample[0].value, vec![7, 4096]);
        assert!(profile.time_nanos > 0);
        assert_eq!(profile.duration_nanos, 10_000);
    }

    #[test]
    fn encode_allocation_profile_allows_empty_samples() {
        let bytes = encode_allocation_profile(&[], 1024 * 1024, 0);
        let profile = Profile::decode(bytes.as_slice()).expect("decode empty memory pprof");

        assert_eq!(profile.sample.len(), 0);
        assert_eq!(profile.sample_type.len(), 2);
        assert_eq!(profile.period, 1024 * 1024);
    }
}
