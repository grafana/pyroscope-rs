use crate::backend;

use backend::types::{EncodedReport, Report};

pub fn encode(reports: &Vec<Report>) -> Vec<EncodedReport> {
    reports
        .into_iter()
        .map(|r| EncodedReport {
            format: "folded".to_string(),
            content_type: "binary/octet-stream".to_string(),
            content_encoding: "".to_string(),
            data: r.to_string().into_bytes(),
            metadata: r.metadata.to_owned(),
        })
        .collect()
}
