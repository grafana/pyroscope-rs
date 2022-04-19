use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    backend::Report,
    pyroscope::PyroscopeConfig,
    utils::{get_time_range, merge_tags_with_app_name_v2},
    Result,
};

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
    pub config: PyroscopeConfig,
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

        // Loop through the reports and process them
        for report in &self.reports {
            dbg!(&report.metadata);
            self.process(&report)?;
        }

        Ok(())
    }

    fn process(&self, report: &Report) -> Result<()> {
        log::info!(
            target: LOG_TAG,
            "Sending Session: {} - {}",
            self.from,
            self.until
        );

        // Convert a report to a byte array
        let report_u8 = report.to_string().into_bytes();

        // Check if the report is empty
        if report_u8.is_empty() {
            return Ok(());
        }

        // Create a new client
        let client = reqwest::blocking::Client::new();

        // Clone URL
        let url = self.config.url.clone();

        // Merge application name with Tags
        //let application_name = merge_tags_with_app_name(
        //self.config.application_name.clone(),
        //self.config.tags.clone(),
        //)?;

        // New Merge
        let application_name = merge_tags_with_app_name_v2(
            self.config.application_name.clone(),
            report.metadata.tags.clone(),
        )?;

        // Merge application name with Report Tags

        // Create and send the request
        client
            .post(format!("{}/ingest", url))
            .header("Content-Type", "binary/octet-stream")
            .query(&[
                ("name", application_name.as_str()),
                ("from", &format!("{}", self.from)),
                ("until", &format!("{}", self.until)),
                ("format", "folded"),
                ("sampleRate", &format!("{}", self.config.sample_rate)),
                ("spyName", self.config.spy_name.as_str()),
            ])
            .body(report_u8)
            .timeout(Duration::from_secs(10))
            .send()?;
        Ok(())
    }
}
