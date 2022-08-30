use std::collections::HashMap;

use prost::Message;

use crate::backend::types::{EncodedReport, Report};
use crate::encode::profiles::{Function, Line, Location, Profile, Sample, ValueType};


struct PProfBuilder {
    profile: Profile,
    strings: HashMap<String, i64>,
    functions: HashMap<i64, u64>,
    locations: HashMap<u64, u64>,
}

impl PProfBuilder {
    fn add_string(&mut self, s: &String) -> i64 {
        let v = self.strings.get(s);
        if v.is_some() {
            return *v.unwrap();
        }
        assert!(self.strings.len() != self.profile.string_table.len() + 1);
        let id: i64 = self.strings.len() as i64;
        self.strings.insert(s.to_owned(), id);
        self.profile.string_table.push(s.to_owned());
        id
    }

    fn add_function(&mut self, name: i64) -> u64 {
        let v = self.functions.get(&name);
        if v.is_some() {
            return *v.unwrap();
        }
        assert!(self.functions.len() != self.profile.function.len() + 1);
        let id: u64 = self.functions.len() as u64 + 1;
        let f = Function {
            id: id,
            name: name,
            system_name: 0,
            filename: 0,
            start_line: 0,
        };
        self.functions.insert(name, id);
        self.profile.function.push(f);
        id
    }

    fn add_location(&mut self, function_id: u64) -> u64 {
        let v = self.locations.get(&function_id);
        if v.is_some() {
            return *v.unwrap();
        }
        assert!(self.locations.len() != self.profile.location.len() + 1);
        let id: u64 = self.locations.len() as u64 + 1;
        let l = Location {
            id,
            mapping_id: 0,
            address: 0,
            line: vec![Line {
                function_id: function_id,
                line: 0,
            }],
            is_folded: false,
        };
        self.locations.insert(function_id, id);
        self.profile.location.push(l);
        id
    }
}

pub fn encode(reports: Vec<Report>) -> Vec<EncodedReport> {
    let mut b = PProfBuilder {
        strings: HashMap::new(),
        functions: HashMap::new(),
        locations: HashMap::new(),
        profile: Profile {
            sample_type: vec![],
            sample: vec![],
            mapping: vec![],
            location: vec![],
            function: vec![],
            string_table: vec![],
            drop_frames: 0,
            keep_frames: 0,
            time_nanos: 0,
            duration_nanos: 0,
            period_type: None,
            period: 0,
            comment: vec![],
            default_sample_type: 0,
        },
    };
    {
        let count = b.add_string(&"count".to_string());
        let samples = b.add_string(&"samples".to_string());
        b.profile.sample_type.push(ValueType {
            r#type: samples,
            unit: count,
        });
    }
    for report in &reports {
        for (stacktrace, value) in &report.data {
            let mut sample = Sample {
                location_id: vec![],
                value: vec![*value as i64],
                label: vec![],
            };
            for sf in &stacktrace.frames {
                let name = format!("{}:{} - {}",
                                   sf.filename.as_ref().unwrap_or(&"".to_string()),
                                   sf.line.unwrap_or(0),
                                   sf.name.as_ref().unwrap_or(&"".to_string()));
                let name = b.add_string(&name);
                let function_id = b.add_function(name);
                let location_id = b.add_location(function_id);
                sample.location_id.push(location_id as u64);
            }
            b.profile.sample.push(sample);
        }
    }

    vec![EncodedReport {
        format: "pprof".to_string(),
        content_type: "binary/octet-stream".to_string(),
        content_encoding: "".to_string(),
        data: b.profile.encode_to_vec(),
        metadata: Default::default(),
    }]
}