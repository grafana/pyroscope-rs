use crate::backend;

use backend::types::{EncodedReport, Report};

pub fn encode(reports: &[Report]) -> Vec<EncodedReport> {
    reports
        .iter()
        .map(|r| EncodedReport {
            format: "folded".to_string(),
            content_type: "binary/octet-stream".to_string(),
            content_encoding: "".to_string(),
            data: r.to_string().into_bytes(),
            metadata: r.metadata.to_owned(),
        })
        .collect()
}
