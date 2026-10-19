use std::io::{self, Write};

use flate2::{write::GzEncoder, Compression};
use prost::Message;

use crate::{backend::Report, encode::pprof};

pub(super) fn encode_reports(
    reports: Vec<Report>,
    transform: Option<fn(Report) -> Report>,
    sample_rate: u32,
    start_time_nanos: u64,
    duration_nanos: u64,
) -> Vec<u8> {
    let reports: Vec<Report> = match transform {
        None => reports,
        Some(transform) => reports.into_iter().map(transform).collect(),
    };
    pprof::encode(&reports, sample_rate, start_time_nanos, duration_nanos).encode_to_vec()
}

pub(super) fn gzip(report: &[u8]) -> io::Result<Vec<u8>> {
    // Favor upload CPU cost over maximum compression. Report bytes and the
    // gzip wire format stay unchanged; compressed size can increase.
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(report)?;
    encoder.finish()
}
