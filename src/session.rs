use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::Duration,
    io::Write,
};

use reqwest::Url;
use libflate::gzip::Encoder;

use crate::{
    backend::Report,
    pyroscope::{PyroscopeConfig, Compression},
    utils::{get_time_range, merge_tags_with_app_name},
    Result,
    encode::{folded, pprof}
};
use crate::backend::EncodedReport;
use crate::pyroscope::ReportEncoding;

const LOG_TAG: &str = "Pyroscope::Session";

/// Session Signal
///
/// This enum is used to send data to the session thread. It can also kill the session thread.
#[derive(Debug)]
pub enum SessionSignal {
    /// Send session data to the session thread.
    Session(Session),
    /// Kill the session thread.
    Kill,
}

/// Manage sessions and send data to the server.
#[derive(Debug)]
pub struct SessionManager {
    /// The SessionManager thread.
    pub handle: Option<JoinHandle<Result<()>>>,
    /// Channel to send data to the SessionManager thread.
    pub tx: SyncSender<SessionSignal>,
}

impl SessionManager {
    /// Create a new SessionManager
    pub fn new() -> Result<Self> {
        log::info!(target: LOG_TAG, "Creating SessionManager");

        // Create a channel for sending and receiving sessions
        let (tx, rx): (SyncSender<SessionSignal>, Receiver<SessionSignal>) = sync_channel(10);

        // Create a thread for the SessionManager
        let handle = Some(thread::spawn(move || {
            log::trace!(target: LOG_TAG, "Started");
            while let Ok(signal) = rx.recv() {
                match signal {
                    SessionSignal::Session(session) => {
                        // Send the session
                        // Matching is done here (instead of ?) to avoid breaking
                        // the SessionManager thread if the server is not available.
                        match session.send() {
                            Ok(_) => log::trace!("SessionManager - Session sent"),
                            Err(e) => log::error!("SessionManager - Failed to send session: {}", e),
                        }
                    }
                    SessionSignal::Kill => {
                        // Kill the session manager
                        log::trace!(target: LOG_TAG, "Kill signal received");
                        return Ok(());
                    }
                }
            }
            Ok(())
        }));

        Ok(SessionManager { handle, tx })
    }

    /// Push a new session into the SessionManager
    pub fn push(&self, session: SessionSignal) -> Result<()> {
        // Push the session into the SessionManager
        self.tx.send(session)?;

        log::trace!(target: LOG_TAG, "SessionSignal pushed");

        Ok(())
    }
}

/// Pyroscope Session
///
/// Used to contain the session data, and send it to the server.
#[derive(Clone, Debug)]
pub struct Session {
    // Pyroscope instance configuration
    pub config: PyroscopeConfig,
    // Session data
    pub reports: Vec<Report>,
    // unix time
    pub from: u64,
    // unix time
    pub until: u64,
}

impl Session {
    /// Create a new Session
    /// # Example
    /// ```ignore
    /// let config = PyroscopeConfig::new("https://localhost:8080", "my-app");
    /// let report = vec![1, 2, 3];
    /// let until = 154065120;
    /// let session = Session::new(until, config, report)?;
    /// ```
    pub fn new(until: u64, config: PyroscopeConfig, reports: Vec<Report>) -> Result<Self> {
        log::info!(target: LOG_TAG, "Creating Session");

        // get_time_range should be used with "from". We balance this by reducing
        // 10s from the returned range.
        let time_range = get_time_range(until)?;

        Ok(Self {
            config,
            reports,
            from: time_range.from - 10,
            until: time_range.until - 10,
        })
    }

    /// Send the session to the server and consumes the session object.
    /// # Example
    /// ```ignore
    /// let config = PyroscopeConfig::new("https://localhost:8080", "my-app");
    /// let report = vec![1, 2, 3];
    /// let until = 154065120;
    /// let session = Session::new(until, config, report)?;
    /// session.send()?;
    /// ```
    pub fn send(self) -> Result<()> {
        // Check if the report is empty
        if self.reports.is_empty() {
            return Ok(());
        }


        let reports = self.process_reports(&self.reports);
        let reports = self.encode_reports(reports);
        let reports = self.compress_reports(reports);

        for report in reports {
            self.upload(report)?;
        }

        Ok(())
    }

    // Aly config.func on all reports
    fn process_reports(&self, reports: &Vec<Report>) -> Vec<Report> {
        if let Some(func) = self.config.func {
            reports.iter().map(|r| func(r.to_owned())).collect()
        } else {
            reports.to_owned()
        }
    }

    // Encode reports to folded or protobuf format
    fn encode_reports(&self, reports: Vec<Report>) -> Vec<EncodedReport> {
        match &self.config.report_encoding {
            ReportEncoding::FOLDED => folded::encode(reports),
            ReportEncoding::PPROF => pprof::encode(reports),
        }
    }

    // Optionally compress reports
    fn compress_reports(&self, reports: Vec<EncodedReport>) -> Vec<EncodedReport> {
        reports.into_iter().map(|r|
            match &self.config.compression {
                None => r,
                Some(Compression::GZIP) => {
                    let mut encoder = Encoder::new(Vec::new()).unwrap();
                    encoder.write_all(&r.data).unwrap();
                    let compressed_data = encoder.finish().into_result().unwrap();
                    EncodedReport {
                        format: r.format,
                        content_type: r.content_type,
                        metadata: r.metadata,
                        content_encoding: "gzip".to_string(),
                        data: compressed_data,
                    }
                }
            }
        ).collect()
    }


    fn upload(&self, report: EncodedReport) -> Result<()> {
        log::info!(
            target: LOG_TAG,
            "Sending Session: {} - {}",
            self.from,
            self.until
        );

        if report.data.is_empty() {
            return Ok(());
        }

        // Create a new client
        let client = reqwest::blocking::Client::new();

        // Clone URL
        let url = self.config.url.clone();

        // Merge application name with Tags
        let application_name = merge_tags_with_app_name(
            self.config.application_name.clone(),
            report.metadata.tags.clone().into_iter().collect(),
        )?;

        // Parse URL
        let joined = Url::parse(&url)?.join("ingest")?;

        // Create Reqwest builder
        let mut req_builder = client
            .post(joined.as_str())
            .header("Content-Type", report.content_type.as_str());

        // Set authentication token
        if let Some(auth_token) = self.config.auth_token.clone() {
            req_builder = req_builder.bearer_auth(auth_token);
        }
        if report.content_encoding != "" {
            req_builder = req_builder.header("Content-Encoding", report.content_encoding.as_str());
        }

        // Send the request
        req_builder
            .query(&[
                ("name", application_name.as_str()),
                ("from", &format!("{}", self.from)),
                ("until", &format!("{}", self.until)),
                ("format", report.format.as_str()),
                ("sampleRate", &format!("{}", self.config.sample_rate)),
                ("spyName", self.config.spy_name.as_str()),
            ])
            .body(report.data)
            .timeout(Duration::from_secs(10))
            .send()?;
        Ok(())
    }
}
