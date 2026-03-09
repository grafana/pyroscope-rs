// Minimal pprof protobuf encoder.
//
// Defines pprof message structs inline using prost derive macros (no .proto file).
// Encodes to protobuf bytes with prost.

use std::collections::HashMap;

use prost::Message;

// ---------------------------------------------------------------------------
// Pprof protobuf message types (inline, matching google/pprof profile.proto)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Message)]
pub struct Profile {
    /// A description of the samples associated with each Sample.value.
    #[prost(message, repeated, tag = "1")]
    pub sample_type: Vec<ValueType>,
    /// The set of samples recorded in this profile.
    #[prost(message, repeated, tag = "2")]
    pub sample: Vec<Sample>,
    /// All locations referenced by this profile.
    #[prost(message, repeated, tag = "4")]
    pub location: Vec<Location>,
    /// All functions referenced by this profile.
    #[prost(message, repeated, tag = "5")]
    pub function: Vec<Function>,
    /// A common table for strings referenced by various messages.
    /// Index 0 must always be "".
    #[prost(string, repeated, tag = "6")]
    pub string_table: Vec<String>,
    /// Time of collection (approximate), as nanoseconds past the Unix epoch.
    #[prost(int64, tag = "9")]
    pub time_nanos: i64,
    /// Duration of the profile, if a duration makes sense, in nanoseconds.
    #[prost(int64, tag = "10")]
    pub duration_nanos: i64,
    /// The kind of events between sampled occurrences.
    #[prost(message, optional, tag = "11")]
    pub period_type: Option<ValueType>,
    /// The number of events between sampled occurrences.
    #[prost(int64, tag = "12")]
    pub period: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct ValueType {
    /// Index into string_table for the type name (e.g. "cpu").
    #[prost(int64, tag = "1")]
    pub r#type: i64,
    /// Index into string_table for the unit (e.g. "nanoseconds").
    #[prost(int64, tag = "2")]
    pub unit: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Sample {
    /// The IDs of the location (frame) for this sample, from leaf to root.
    #[prost(uint64, repeated, tag = "1")]
    pub location_id: Vec<u64>,
    /// The type-specific measurement values for this sample.
    #[prost(int64, repeated, tag = "2")]
    pub value: Vec<i64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Location {
    /// Unique nonzero id for this location.
    #[prost(uint64, tag = "1")]
    pub id: u64,
    /// The set of inlined functions at this location.
    #[prost(message, repeated, tag = "4")]
    pub line: Vec<Line>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Line {
    /// Index into Profile.function for the function executed at this line.
    #[prost(uint64, tag = "1")]
    pub function_id: u64,
    /// Line number in source code.
    #[prost(int64, tag = "2")]
    pub line: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Function {
    /// Unique nonzero id for this function.
    #[prost(uint64, tag = "1")]
    pub id: u64,
    /// Index into string_table for the name of the function.
    #[prost(int64, tag = "2")]
    pub name: i64,
    /// Index into string_table for the source file containing the function.
    #[prost(int64, tag = "4")]
    pub filename: i64,
    /// Line number in source file of start of function.
    #[prost(int64, tag = "5")]
    pub start_line: i64,
}

// ---------------------------------------------------------------------------
// String table with deduplication
// ---------------------------------------------------------------------------

struct StringTable {
    strings: Vec<String>,
    index: HashMap<String, i64>,
}

impl StringTable {
    fn new() -> Self {
        // Index 0 must always be "" per spec.
        let mut st = StringTable {
            strings: Vec::new(),
            index: HashMap::new(),
        };
        st.intern("");
        st
    }

    fn intern(&mut self, s: &str) -> i64 {
        if let Some(&idx) = self.index.get(s) {
            return idx;
        }
        let idx = self.strings.len() as i64;
        self.strings.push(s.to_owned());
        self.index.insert(s.to_owned(), idx);
        idx
    }
}

// ---------------------------------------------------------------------------
// Frame — a single symbolized stack frame
// ---------------------------------------------------------------------------

pub struct Frame<'a> {
    pub function_name: &'a str,
    pub filename: &'a str,
    pub first_line: i64,
}

// ---------------------------------------------------------------------------
// ProfileBuilder — accumulates samples and produces an encoded profile
// ---------------------------------------------------------------------------

pub struct ProfileBuilder {
    st: StringTable,
    functions: Vec<Function>,
    /// Map from (name_idx, filename_idx, start_line) → function id (1-based)
    func_index: HashMap<(i64, i64, i64), u64>,
    locations: Vec<Location>,
    /// Map from function id → location id (1-based); one location per function
    loc_index: HashMap<u64, u64>,
    samples: Vec<Sample>,
    /// Map from location_id sequence → index into self.samples (for merging)
    sample_index: HashMap<Vec<u64>, usize>,
    time_nanos: i64,
    duration_nanos: i64,
    period: i64,
}

impl ProfileBuilder {
    /// Create a new builder.
    ///
    /// * `time_nanos`     — profile start time (Unix epoch nanoseconds)
    /// * `duration_nanos` — duration covered by this profile in nanoseconds
    /// * `period`         — sampling period in nanoseconds (e.g. 10_000_000 for 10 ms)
    pub fn new(time_nanos: i64, duration_nanos: i64, period: i64) -> Self {
        ProfileBuilder {
            st: StringTable::new(),
            functions: Vec::new(),
            func_index: HashMap::new(),
            locations: Vec::new(),
            loc_index: HashMap::new(),
            samples: Vec::new(),
            sample_index: HashMap::new(),
            time_nanos,
            duration_nanos,
            period,
        }
    }

    /// Add a sample consisting of a symbolized stack and a hit count.
    ///
    /// Frames should be ordered leaf-first (innermost frame first).
    /// The value stored is `count * period` (CPU nanoseconds).
    /// If an identical stack (same location_id sequence) was already added,
    /// the values are merged (summed) instead of creating a duplicate sample.
    pub fn add_sample(&mut self, frames: &[Frame<'_>], count: i64) {
        let mut location_ids: Vec<u64> = Vec::with_capacity(frames.len());
        for frame in frames {
            let func_id = self.intern_function(frame);
            let loc_id = self.intern_location(func_id, frame.first_line);
            location_ids.push(loc_id);
        }
        let value = count * self.period;
        if let Some(&idx) = self.sample_index.get(&location_ids) {
            self.samples[idx].value[0] += value;
        } else {
            let idx = self.samples.len();
            self.samples.push(Sample {
                location_id: location_ids.clone(),
                value: vec![value],
            });
            self.sample_index.insert(location_ids, idx);
        }
    }

    /// Reset all accumulated state so the builder can be reused for the next
    /// profile window. Keeps the allocated capacity for efficiency.
    pub fn reset(&mut self, time_nanos: i64, duration_nanos: i64) {
        self.st = StringTable::new();
        self.functions.clear();
        self.func_index.clear();
        self.locations.clear();
        self.loc_index.clear();
        self.samples.clear();
        self.sample_index.clear();
        self.time_nanos = time_nanos;
        self.duration_nanos = duration_nanos;
    }

    /// Return the number of accumulated samples (unique stacks).
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Return true if no samples have been accumulated.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    fn intern_function(&mut self, frame: &Frame<'_>) -> u64 {
        let name_idx = self.st.intern(frame.function_name);
        let filename_idx = self.st.intern(frame.filename);
        let start_line = frame.first_line;
        let key = (name_idx, filename_idx, start_line);
        if let Some(&id) = self.func_index.get(&key) {
            return id;
        }
        let id = (self.functions.len() + 1) as u64;
        self.functions.push(Function {
            id,
            name: name_idx,
            filename: filename_idx,
            start_line,
        });
        self.func_index.insert(key, id);
        id
    }

    fn intern_location(&mut self, func_id: u64, line: i64) -> u64 {
        if let Some(&id) = self.loc_index.get(&func_id) {
            return id;
        }
        let id = (self.locations.len() + 1) as u64;
        self.locations.push(Location {
            id,
            line: vec![Line {
                function_id: func_id,
                line,
            }],
        });
        self.loc_index.insert(func_id, id);
        id
    }

    /// Encode the profile to protobuf bytes.
    pub fn encode(&mut self) -> Vec<u8> {
        let cpu_type_idx = self.st.intern("cpu");
        let nanos_idx = self.st.intern("nanoseconds");
        let value_type = ValueType {
            r#type: cpu_type_idx,
            unit: nanos_idx,
        };

        let profile = Profile {
            sample_type: vec![value_type.clone()],
            sample: core::mem::take(&mut self.samples),
            location: core::mem::take(&mut self.locations),
            function: core::mem::take(&mut self.functions),
            string_table: core::mem::take(&mut self.st.strings),
            time_nanos: self.time_nanos,
            duration_nanos: self.duration_nanos,
            period_type: Some(value_type),
            period: self.period,
        };

        profile.encode_to_vec()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame<'a>(name: &'a str, file: &'a str, line: i64) -> Frame<'a> {
        Frame {
            function_name: name,
            filename: file,
            first_line: line,
        }
    }

    #[test]
    fn string_table_starts_with_empty() {
        let mut st = StringTable::new();
        assert_eq!(st.strings[0], "");
        assert_eq!(st.intern(""), 0);
    }

    #[test]
    fn string_table_deduplicates() {
        let mut st = StringTable::new();
        let a = st.intern("hello");
        let b = st.intern("hello");
        assert_eq!(a, b);
        assert_eq!(st.strings.len(), 2); // "" and "hello"
    }

    #[test]
    fn encode_empty_profile_is_valid_protobuf() {
        let mut builder = ProfileBuilder::new(0, 15_000_000_000, 10_000_000);
        let bytes = builder.encode();
        assert!(!bytes.is_empty());
        let profile = Profile::decode(bytes.as_slice()).unwrap();
        assert_eq!(profile.string_table[0], "");
    }

    #[test]
    fn encode_single_sample() {
        let mut builder = ProfileBuilder::new(1_000_000_000, 15_000_000_000, 10_000_000);
        let frames = vec![
            make_frame("leaf_fn", "leaf.rs", 10),
            make_frame("root_fn", "root.rs", 1),
        ];
        builder.add_sample(&frames, 3);
        let bytes = builder.encode();

        let profile = Profile::decode(bytes.as_slice()).unwrap();

        // string_table[0] must be ""
        assert_eq!(profile.string_table[0], "");

        // One sample with value = count * period = 3 * 10_000_000
        assert_eq!(profile.sample.len(), 1);
        assert_eq!(profile.sample[0].value, vec![30_000_000i64]);
        assert_eq!(profile.sample[0].location_id.len(), 2);

        // Two locations, two functions
        assert_eq!(profile.location.len(), 2);
        assert_eq!(profile.function.len(), 2);

        // period and period_type
        assert_eq!(profile.period, 10_000_000);
        assert!(profile.period_type.is_some());

        // time / duration
        assert_eq!(profile.time_nanos, 1_000_000_000);
        assert_eq!(profile.duration_nanos, 15_000_000_000);
    }

    #[test]
    fn merges_identical_stacks() {
        let mut builder = ProfileBuilder::new(0, 15_000_000_000, 10_000_000);
        let frames = vec![make_frame("main", "main.rs", 1)];
        builder.add_sample(&frames, 1);
        builder.add_sample(&frames, 2);
        let bytes = builder.encode();

        let profile = Profile::decode(bytes.as_slice()).unwrap();
        assert_eq!(profile.function.len(), 1);
        assert_eq!(profile.location.len(), 1);
        // Identical stacks are merged into one sample with summed value.
        assert_eq!(profile.sample.len(), 1);
        assert_eq!(profile.sample[0].value, vec![(1 + 2) * 10_000_000i64]);
    }
}
