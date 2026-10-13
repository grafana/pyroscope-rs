use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use prost::Message;

use crate::encode::gen::google::{
    Function, Label, Line, Location, Mapping, Profile, Sample, ValueType,
};

/// A verified executable segment in the process's runtime address space.
pub(crate) struct MemoryMapping {
    pub memory_start: u64,
    pub memory_limit: u64,
    pub file_offset: u64,
    pub filename: String,
    pub build_id: String,
}

/// A memory allocation sample ready to be encoded into pprof.
#[derive(Debug, Clone)]
pub struct AllocationSample {
    /// Physical frames in leaf-to-root order, shared across sampled stacks.
    pub frames: Vec<Arc<MemoryFrame>>,
    pub alloc_objects: i64,
    pub alloc_space: i64,
    pub inuse_objects: i64,
    pub inuse_space: i64,
}

impl AllocationSample {
    pub fn new(frames: Vec<String>, alloc_objects: i64, alloc_space: i64) -> Self {
        Self {
            frames: frames
                .into_iter()
                .map(|name| Arc::new(MemoryFrame::named(name)))
                .collect(),
            alloc_objects,
            alloc_space,
            inuse_objects: 0,
            inuse_space: 0,
        }
    }
}

/// A symbol at an instruction address, including an optional source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySymbol {
    /// Demangled function name without Rust hash/disambiguator decorations.
    pub name: String,
    /// Original symbol name when provided by the resolver, before demangling.
    pub system_name: Option<String>,
    /// Source file when debug information is available.
    pub filename: Option<String>,
    /// Source line, or zero when unknown.
    pub line: i64,
}

/// One physical instruction address and its leaf-first inline symbol chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryFrame {
    /// Runtime instruction address; zero for name-only synthetic frames.
    pub address: u64,
    /// Inline symbols at this address, innermost first; empty when unresolved.
    pub symbols: Vec<MemorySymbol>,
}

impl MemoryFrame {
    /// Create a name-only synthetic frame with no source metadata.
    pub fn named(name: String) -> Self {
        Self {
            address: 0,
            symbols: vec![MemorySymbol {
                name,
                system_name: None,
                filename: None,
                line: 0,
            }],
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct LocationKey {
    address: u64,
    lines: Vec<Line>,
}

struct PprofMemoryBuilder<'a> {
    profile: Profile,
    // Keys borrow immutable inputs; only the protobuf string table owns a copy.
    strings: HashMap<&'a str, i64>,
    functions: HashMap<(i64, i64, i64), u64>,
    locations: HashMap<LocationKey, u64>,
    // Keep an owning reference so identity-cache addresses cannot be reused.
    frames: HashMap<usize, (Arc<MemoryFrame>, u64)>,
    // Sorted independently from the pprof mappings, whose first entry is the main binary.
    mapping_ranges: Vec<(u64, u64, usize)>,
    mapping_seen: Vec<bool>,
    delta_label: Label,
    live_heap: bool,
}

impl<'a> PprofMemoryBuilder<'a> {
    fn new(period: i64, duration_nanos: i64, live_heap: bool) -> Self {
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
            delta_label: Label::default(),
            functions: HashMap::new(),
            locations: HashMap::new(),
            frames: HashMap::new(),
            mapping_ranges: Vec::new(),
            mapping_seen: Vec::new(),
            live_heap,
        };

        builder.add_string("");
        let alloc_objects = builder.add_string("alloc_objects");
        // Pyroscope promotes this sample label to the series. "false" means
        // do not difference alloc_* again: these samples are already intervals.
        builder.delta_label = Label {
            key: builder.add_string("__delta__"),
            str: builder.add_string("false"),
            num: 0,
            num_unit: 0,
        };
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
        if live_heap {
            let inuse_objects = builder.add_string("inuse_objects");
            let inuse_space = builder.add_string("inuse_space");
            builder.profile.sample_type.extend([
                ValueType {
                    r#type: inuse_objects,
                    unit: count,
                },
                ValueType {
                    r#type: inuse_space,
                    unit: bytes,
                },
            ]);
            builder.profile.default_sample_type = inuse_space;
        }

        builder
    }

    fn add_mappings(&mut self, mappings: &'a [MemoryMapping]) {
        for mapping in mappings {
            if mapping.memory_start >= mapping.memory_limit {
                continue;
            }
            let filename = self.add_string(&mapping.filename);
            let build_id = self.add_string(&mapping.build_id);
            let index = self.profile.mapping.len();
            self.profile.mapping.push(Mapping {
                id: index as u64 + 1,
                memory_start: mapping.memory_start,
                memory_limit: mapping.memory_limit,
                file_offset: mapping.file_offset,
                filename,
                build_id,
                ..Mapping::default()
            });
            self.mapping_ranges
                .push((mapping.memory_start, mapping.memory_limit, index));
            self.mapping_seen.push(false);
        }
        self.mapping_ranges.sort_unstable_by_key(|range| range.0);
    }

    fn mapping_index(&self, address: u64) -> Option<usize> {
        if address == 0 {
            return None;
        }
        let index = self
            .mapping_ranges
            .partition_point(|range| range.0 <= address)
            .checked_sub(1)?;
        let (_, limit, mapping) = self.mapping_ranges[index];
        (address < limit).then_some(mapping)
    }

    fn add_string(&mut self, value: &'a str) -> i64 {
        if let Some(id) = self.strings.get(value) {
            return *id;
        }

        let id = self.profile.string_table.len() as i64;
        self.strings.insert(value, id);
        self.profile.string_table.push(value.to_owned());
        id
    }

    fn add_frame(&mut self, frame: &'a Arc<MemoryFrame>) -> u64 {
        let identity = Arc::as_ptr(frame) as usize;
        if let Some((_, location_id)) = self.frames.get(&identity) {
            return *location_id;
        }
        let mapping_id = self.mapping_index(frame.address).map_or(0, |index| {
            let symbols = &frame.symbols;
            let has_functions =
                !symbols.is_empty() && symbols.iter().all(|symbol| !symbol.name.is_empty());
            let has_filenames = has_functions
                && symbols.iter().all(|symbol| {
                    symbol
                        .filename
                        .as_ref()
                        .is_some_and(|name| !name.is_empty())
                });
            let has_lines = has_filenames && symbols.iter().all(|symbol| symbol.line > 0);
            let mapping = &mut self.profile.mapping[index];
            // A partial mapping must remain eligible for offline symbolization.
            if self.mapping_seen[index] {
                mapping.has_functions &= has_functions;
                mapping.has_filenames &= has_filenames;
                mapping.has_line_numbers &= has_lines;
            } else {
                mapping.has_functions = has_functions;
                mapping.has_filenames = has_filenames;
                mapping.has_line_numbers = has_lines;
                self.mapping_seen[index] = true;
            }
            mapping.has_inline_frames |= symbols.len() > 1;
            mapping.id
        });
        let lines: Vec<_> = frame
            .symbols
            .iter()
            .map(|symbol| {
                let name = self.add_string(&symbol.name);
                let system_name = symbol
                    .system_name
                    .as_deref()
                    .map(|name| self.add_string(name))
                    .unwrap_or(0);
                let filename = symbol
                    .filename
                    .as_deref()
                    .map(|file| self.add_string(file))
                    .unwrap_or(0);
                let function_id = *self
                    .functions
                    .entry((name, system_name, filename))
                    .or_insert_with(|| {
                        let id = self.profile.function.len() as u64 + 1;
                        self.profile.function.push(Function {
                            id,
                            name,
                            filename,
                            system_name,
                            start_line: 0,
                        });
                        id
                    });
                Line {
                    function_id,
                    line: symbol.line,
                }
            })
            .collect();
        let key = LocationKey {
            address: frame.address,
            lines,
        };
        let location_id = *self.locations.entry(key).or_insert_with_key(|key| {
            let id = self.profile.location.len() as u64 + 1;
            self.profile.location.push(Location {
                id,
                mapping_id,
                address: key.address,
                line: key.lines.clone(),
                is_folded: false,
            });
            id
        });
        self.frames
            .insert(identity, (Arc::clone(frame), location_id));
        location_id
    }

    fn add_sample(&mut self, sample: &'a AllocationSample) {
        if sample.alloc_space <= 0 && (!self.live_heap || sample.inuse_space <= 0) {
            return;
        }

        let location_id = sample
            .frames
            .iter()
            .map(|frame| self.add_frame(frame))
            .collect();

        let mut value = Vec::with_capacity(if self.live_heap { 4 } else { 2 });
        value.extend([sample.alloc_objects, sample.alloc_space]);
        if self.live_heap {
            value.extend([sample.inuse_objects, sample.inuse_space]);
        }
        self.profile.sample.push(Sample {
            location_id,
            value,
            label: vec![self.delta_label],
        });
    }
}

pub fn encode_allocation_profile(
    samples: &[AllocationSample],
    period: u64,
    duration_nanos: i64,
) -> Vec<u8> {
    encode_memory_profile(samples, period, duration_nanos, false)
}

/// Encode interval allocation counters and, when enabled, a live heap snapshot.
pub fn encode_memory_profile(
    samples: &[AllocationSample],
    period: u64,
    duration_nanos: i64,
    live_heap: bool,
) -> Vec<u8> {
    encode_memory_profile_with_mappings(samples, period, duration_nanos, live_heap, &[])
}

pub(crate) fn encode_memory_profile_with_mappings(
    samples: &[AllocationSample],
    period: u64,
    duration_nanos: i64,
    live_heap: bool,
    mappings: &[MemoryMapping],
) -> Vec<u8> {
    let period = i64::try_from(period).unwrap_or(i64::MAX);
    let mut builder = PprofMemoryBuilder::new(period, duration_nanos, live_heap);
    builder.add_mappings(mappings);

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

    #[test]
    fn interval_memory_samples_disable_server_cumulative_differencing() {
        for live_heap in [false, true] {
            let mut allocation =
                AllocationSample::new(vec!["application::allocate".into()], 1, 4096);
            allocation.inuse_objects = 1;
            allocation.inuse_space = 4096;
            let bytes = encode_memory_profile(&[allocation], 1024, 10_000, live_heap);
            let profile = Profile::decode(bytes.as_slice()).unwrap();
            assert_eq!(profile.sample.len(), 1);
            let labels = &profile.sample[0].label;
            assert_eq!(labels.len(), 1);
            assert_eq!(profile.string_table[labels[0].key as usize], "__delta__");
            assert_eq!(profile.string_table[labels[0].str as usize], "false");
        }
    }

    #[test]
    fn live_heap_profile_keeps_live_only_samples_and_selects_inuse_space() {
        let mut sample = AllocationSample::new(vec!["retained".into()], 0, 0);
        sample.inuse_objects = 3;
        sample.inuse_space = 8192;
        let bytes = encode_memory_profile(&[sample], 4096, 42, true);
        let profile = Profile::decode(bytes.as_slice()).expect("decode live heap pprof");
        let types: Vec<_> = profile
            .sample_type
            .iter()
            .map(|ty| {
                (
                    profile.string_table[ty.r#type as usize].as_str(),
                    profile.string_table[ty.unit as usize].as_str(),
                )
            })
            .collect();
        assert_eq!(
            types,
            [
                ("alloc_objects", "count"),
                ("alloc_space", "bytes"),
                ("inuse_objects", "count"),
                ("inuse_space", "bytes")
            ]
        );
        assert_eq!(
            profile.string_table[profile.default_sample_type as usize],
            "inuse_space"
        );
        assert_eq!(profile.sample[0].value, [0, 0, 3, 8192]);
        assert_eq!(profile.period, 4096);
        assert_eq!(profile.duration_nanos, 42);
    }

    #[test]
    fn locations_preserve_addresses_inline_lines_and_distinct_source_functions() {
        let frame = Arc::new(MemoryFrame {
            address: 0x1234,
            symbols: vec![
                MemorySymbol {
                    name: "allocate".into(),
                    system_name: None,
                    filename: Some("allocator.rs".into()),
                    line: 17,
                },
                MemorySymbol {
                    name: "caller".into(),
                    system_name: None,
                    filename: Some("app.rs".into()),
                    line: 29,
                },
            ],
        });
        let other = Arc::new(MemoryFrame {
            address: 0x5678,
            symbols: vec![MemorySymbol {
                name: "allocate".into(),
                system_name: None,
                filename: Some("other.rs".into()),
                line: 5,
            }],
        });
        let same_function = Arc::new(MemoryFrame {
            address: 0x1240,
            symbols: vec![frame.symbols[0].clone()],
        });
        let mut sample = AllocationSample::new(Vec::new(), 1, 4096);
        sample.frames = vec![Arc::clone(&frame), Arc::clone(&frame), other, same_function];
        let bytes = encode_allocation_profile(&[sample], 4096, 0);
        let profile = Profile::decode(bytes.as_slice()).unwrap();
        assert_eq!(profile.location.len(), 3);
        assert_eq!(profile.function.len(), 3);
        assert_eq!(
            profile.sample[0].location_id[0],
            profile.sample[0].location_id[1]
        );
        let inline = profile
            .location
            .iter()
            .find(|location| location.address == 0x1234)
            .unwrap();
        assert_eq!(inline.line.len(), 2);
        assert_eq!(inline.line[0].line, 17);
        assert_eq!(inline.line[1].line, 29);
        let function = profile
            .function
            .iter()
            .find(|function| function.id == inline.line[0].function_id)
            .unwrap();
        assert_eq!(profile.string_table[function.name as usize], "allocate");
        assert_eq!(
            profile.string_table[function.filename as usize],
            "allocator.rs"
        );
        let same = profile
            .location
            .iter()
            .find(|location| location.address == 0x1240)
            .unwrap();
        assert_eq!(same.line[0].function_id, function.id);
    }

    #[test]
    fn equivalent_distinct_frame_objects_share_a_location() {
        let first = Arc::new(MemoryFrame::named("allocate".into()));
        let second = Arc::new((*first).clone());
        let mut sample = AllocationSample::new(Vec::new(), 1, 512);
        sample.frames = vec![first, second];
        let profile =
            Profile::decode(encode_allocation_profile(&[sample], 1024, 0).as_slice()).unwrap();
        assert_eq!(profile.location.len(), 1);
        assert_eq!(profile.function.len(), 1);
    }

    #[test]
    fn mappings_preserve_runtime_addresses_offsets_and_symbol_metadata() {
        let mappings = [
            MemoryMapping {
                memory_start: 0x8000,
                memory_limit: 0x9000,
                file_offset: 0x2000,
                filename: "app".into(),
                build_id: "abcd1234".into(),
            },
            MemoryMapping {
                memory_start: 0x1000,
                memory_limit: 0x2000,
                file_offset: 0x4000,
                filename: "library.so".into(),
                build_id: String::new(),
            },
        ];
        let inline = Arc::new(MemoryFrame {
            address: 0x8010,
            symbols: vec![
                MemorySymbol {
                    name: "allocate".into(),
                    system_name: None,
                    filename: Some("allocator.rs".into()),
                    line: 17,
                },
                MemorySymbol {
                    name: "caller".into(),
                    system_name: None,
                    filename: Some("app.rs".into()),
                    line: 29,
                },
            ],
        });
        let mut sample = AllocationSample::new(vec!["synthetic".into()], 1, 512);
        sample.frames.extend([
            inline.clone(),
            inline,
            Arc::new(MemoryFrame {
                address: 0x1000,
                symbols: Vec::new(),
            }),
            Arc::new(MemoryFrame {
                address: 0x2000,
                symbols: Vec::new(),
            }),
        ]);
        let bytes = encode_memory_profile_with_mappings(&[sample], 4096, 42, false, &mappings);
        let profile = Profile::decode(bytes.as_slice()).unwrap();
        assert_eq!(profile.mapping.len(), 2);
        let main = &profile.mapping[0];
        assert_eq!(main.id, 1);
        assert_eq!(main.file_offset, 0x2000);
        assert_eq!(profile.string_table[main.filename as usize], "app");
        assert_eq!(profile.string_table[main.build_id as usize], "abcd1234");
        assert!(main.has_functions && main.has_filenames && main.has_line_numbers);
        assert!(main.has_inline_frames);
        let mapped = profile
            .location
            .iter()
            .find(|location| location.address == 0x8010)
            .unwrap();
        assert_eq!(mapped.mapping_id, main.id);
        assert_eq!(mapped.line.len(), 2);
        assert!(profile
            .location
            .iter()
            .any(|location| location.address == 0x1000 && location.mapping_id == 2));
        assert!(profile
            .location
            .iter()
            .any(|location| location.address == 0x2000 && location.mapping_id == 0));
        assert!(profile
            .location
            .iter()
            .any(|location| location.address == 0 && location.mapping_id == 0));
        assert!(!profile.mapping[1].has_functions);
        assert_eq!(profile.mapping[1].build_id, 0);
        assert_eq!(
            profile.sample[0].location_id[1],
            profile.sample[0].location_id[2]
        );
    }

    #[test]
    fn partially_symbolized_mapping_remains_eligible_for_offline_resolution() {
        let mappings = [MemoryMapping {
            memory_start: 0x1000,
            memory_limit: 0x2000,
            file_offset: 0,
            filename: "app".into(),
            build_id: "1234".into(),
        }];
        let mut sample = AllocationSample::new(Vec::new(), 1, 512);
        sample.frames = vec![
            Arc::new(MemoryFrame {
                address: 0x1010,
                symbols: vec![MemorySymbol {
                    name: "allocate".into(),
                    system_name: None,
                    filename: Some("allocator.rs".into()),
                    line: 17,
                }],
            }),
            Arc::new(MemoryFrame {
                address: 0x1020,
                symbols: Vec::new(),
            }),
        ];
        let bytes = encode_memory_profile_with_mappings(&[sample], 4096, 0, false, &mappings);
        let profile = Profile::decode(bytes.as_slice()).unwrap();
        let mapping = &profile.mapping[0];
        assert!(!mapping.has_functions && !mapping.has_filenames && !mapping.has_line_numbers);
    }

    #[test]
    fn raw_symbol_names_disambiguate_equal_display_names() {
        let mut sample = AllocationSample::new(Vec::new(), 1, 512);
        sample.frames = [
            "_ZN4demo3run17h1111111111111111E",
            "_ZN4demo3run17h2222222222222222E",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            Arc::new(MemoryFrame {
                address: 0x1000 + index as u64,
                symbols: vec![MemorySymbol {
                    name: "demo::run".into(),
                    system_name: Some(raw.into()),
                    filename: Some("demo.rs".into()),
                    line: 17,
                }],
            })
        })
        .collect();
        let profile =
            Profile::decode(encode_allocation_profile(&[sample], 1024, 0).as_slice()).unwrap();
        assert_eq!(profile.function.len(), 2);
        assert_eq!(profile.function[0].name, profile.function[1].name);
        assert_ne!(
            profile.function[0].system_name,
            profile.function[1].system_name
        );
        assert_eq!(
            profile.string_table[profile.function[0].system_name as usize],
            "_ZN4demo3run17h1111111111111111E"
        );
    }

    #[test]
    fn unresolved_addresses_do_not_create_fake_functions_or_lines() {
        let mut sample = AllocationSample::new(Vec::new(), 1, 512);
        sample.frames = vec![Arc::new(MemoryFrame {
            address: 0x1010,
            symbols: Vec::new(),
        })];
        let mappings = [MemoryMapping {
            memory_start: 0x1000,
            memory_limit: 0x2000,
            file_offset: 0,
            filename: "app".into(),
            build_id: "1234".into(),
        }];
        let profile = Profile::decode(
            encode_memory_profile_with_mappings(&[sample], 1024, 0, false, &mappings).as_slice(),
        )
        .unwrap();
        assert!(profile.function.is_empty());
        assert!(profile.location[0].line.is_empty());
        assert_eq!(profile.location[0].address, 0x1010);
        assert_eq!(profile.location[0].mapping_id, 1);
        assert!(!profile.mapping[0].has_functions);
        assert!(!profile.string_table.iter().any(|name| name == "0x1010"));
    }

    #[test]
    fn encoded_strings_remain_owned_after_borrowed_inputs_are_dropped() {
        let bytes = {
            let mut sample = AllocationSample::new(Vec::new(), 1, 512);
            sample.frames = vec![Arc::new(MemoryFrame {
                address: 0x1010,
                symbols: vec![MemorySymbol {
                    name: "demo::run".into(),
                    system_name: Some("_ZN4demo3runE".into()),
                    filename: Some("demo.rs".into()),
                    line: 17,
                }],
            })];
            encode_allocation_profile(&[sample], 1024, 0)
        };
        let profile = Profile::decode(bytes.as_slice()).unwrap();
        let function = &profile.function[0];
        assert_eq!(profile.string_table[function.name as usize], "demo::run");
        assert_eq!(
            profile.string_table[function.system_name as usize],
            "_ZN4demo3runE"
        );
        assert_eq!(profile.string_table[function.filename as usize], "demo.rs");
    }
}
