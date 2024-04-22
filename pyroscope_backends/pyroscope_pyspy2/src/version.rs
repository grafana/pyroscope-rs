use anyhow::Error;
use lazy_static::lazy_static;
use log::{debug, error, info};

use regex::bytes::Regex;

#[derive(Debug)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}


impl Version {
    pub fn scan_bytes(data: &[u8]) -> Result<Version, Error> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"((2|3)\.(3|4|5|6|7|8|9|10|11)\.(\d{1,2}))((a|b|c|rc)\d{1,2})?(\+(?:[0-9a-z-]+(?:[.][0-9a-z-]+)*)?)? (.{1,64})"
            )
            .unwrap();
        }

        if let Some(cap) = RE.captures_iter(data).next() {
            let release = match cap.get(5) {
                Some(x) => std::str::from_utf8(x.as_bytes())?,
                None => "",
            };
            let major = std::str::from_utf8(&cap[2])?.parse::<u64>()?;
            let minor = std::str::from_utf8(&cap[3])?.parse::<u64>()?;
            let patch = std::str::from_utf8(&cap[4])?.parse::<u64>()?;
            let build_metadata = if let Some(s) = cap.get(7) {
                Some(std::str::from_utf8(&s.as_bytes()[1..])?.to_owned())
            } else {
                None
            };

            let version = std::str::from_utf8(&cap[0])?;
            info!("Found matching version string '{}'", version);
            #[cfg(windows)]
            {
                if version.contains("32 bit") {
                    error!("32-bit python is not yet supported on windows! See https://github.com/benfred/py-spy/issues/31 for updates");
                    // we're panic'ing rather than returning an error, since we can't recover from this
                    // and returning an error would just get the calling code to fall back to other
                    // methods of trying to find the version
                    panic!("32-bit python is unsupported on windows");
                }
            }

            let v = Version {
                major,
                minor,
                patch,
                // release_flags: release.to_owned(),
                // build_metadata,
            };
            debug!("Found version: {:?} release_flags: {:?} build_metadata: {:?}", v, release, build_metadata);
            return Ok(v);
        }
        Err(format_err!("failed to find version string"))
    }
}
