use std::io::{self, Write};

use flate2::{write::GzEncoder, Compression};

pub(super) fn gzip(report: &[u8]) -> io::Result<Vec<u8>> {
    // Favor upload CPU cost over maximum compression. Report bytes and the
    // gzip wire format stay unchanged; compressed size can increase.
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(report)?;
    encoder.finish()
}
