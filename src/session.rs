use crate::pyroscope::PyroscopeConfig;
use crate::utils::merge_tags_with_app_name;
use crate::Result;

#[derive(Clone, Debug)]
pub struct Session {
    pub config: PyroscopeConfig,
    pub report: Vec<u8>,
    pub from: u64,
    pub until: u64,
}

impl Session {
    pub fn new(mut until: u64, config: PyroscopeConfig, report: Vec<u8>) -> Self {
        // Session interrupted (0 signal), determine the time
        if until == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            until = now
                .checked_add(10u64.checked_sub(now.checked_rem(10).unwrap()).unwrap())
                .unwrap();
        }

        // Start of the session
        let from = until.checked_sub(10u64).unwrap();

        println!("From: {}", from);
        println!("Until: {}", until);

        dbg!(config.clone());
        dbg!(report.len());

        Self {
            config,
            report,
            from,
            until,
        }
    }

    pub fn send(self) -> Result<()> {
        std::thread::spawn(move || {
            if self.report.is_empty() {
                return;
            }

            let client = reqwest::blocking::Client::new();
            // TODO: handle the error of this request

            // Clone URL
            let url = self.config.url.clone();

            // Merge application name with Tags
            let application_name = merge_tags_with_app_name(
                self.config.application_name.clone(),
                self.config.tags.clone(),
            )
            .unwrap();

            client
                .post(format!("{}/ingest", url))
                .header("Content-Type", "binary/octet-stream")
                .query(&[
                    ("name", application_name.as_str()),
                    ("from", &format!("{}", self.from)),
                    ("until", &format!("{}", self.until)),
                    ("format", "folded"),
                    ("sampleRate", &format!("{}", self.config.sample_rate)),
                    ("spyName", "pprof-rs"),
                ])
                .body(self.report)
                .send()
                .unwrap();

            return;
        });

        Ok(())
    }
}
