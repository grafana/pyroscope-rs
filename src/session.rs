use std::{
    io::Write,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::Duration,
};

use libflate::gzip::Encoder;
use prost::Message;
use reqwest::Url;
use uuid::Uuid;
use crate::encode::gen::google::Profile;
use crate::encode::gen::push::{PushRequest, RawProfileSeries, RawSample};
use crate::{
    backend::Report,
    encode::pprof,
    pyroscope::PyroscopeConfig,
    utils::get_time_range,
    PyroscopeError, Result,
};
use crate::encode::gen::types::LabelPair;

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
            let client = reqwest::blocking::Client::new();
            while let Ok(signal) = rx.recv() {
                match signal {
                    SessionSignal::Session(session) => {
                        // Send the session
                        // Matching is done here (instead of ?) to avoid breaking
                        // the SessionManager thread if the server is not available.
                        match session.send_with_client(&client) {
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

#[derive(Clone, Debug)]
pub struct Session {
    pub config: PyroscopeConfig,
    pub reports: Vec<Report>,
    // unix time todo remove comment, use types
    pub from: u64,
    // unix time todo remove comment, use types
    pub until: u64,
}

impl Session {
    const PUSHER_SERVICE_PUSH: &str = "/push.v1.PusherService/Push";

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


    pub fn send_with_client(self, client: &reqwest::blocking::Client) -> Result<()> {
        if self.reports.is_empty() {
            return Ok(());
        }
        match self.config.func {
            None => {}
            Some(f) => {

            }
        }

        let profile = self.encode_reports(&self.reports);

        self.push(profile, client)
    }

    fn encode_reports(&self, reports: &Vec<Report>) -> Profile {
        pprof::encode(
            reports,
            self.config.sample_rate,
            self.from * 1_000_000_000,
            (self.until - self.from) * 1_000_000_000,
        )
    }

    fn push(self, profile: Profile, client: &reqwest::blocking::Client) -> Result<()> {
        log::info!(target: LOG_TAG, "Sending Session: {} - {}", self.from, self.until);

        let mut labels :Vec<LabelPair> = Vec::with_capacity(1 + self.config.tags.iter().len());
        labels.push(LabelPair {
            name: "service_name".to_string(),
            value: self.config.application_name.clone(),
        });
        for tag in self.config.tags {
            labels.push(LabelPair{
                name: tag.0,
                value: tag.1,
            })
        }
        let req = PushRequest {
            series: vec![RawProfileSeries {
                labels,
                samples: vec![RawSample {
                    raw_profile: profile.encode_to_vec(),
                    id: Uuid::new_v4().to_string(),
                }],
            }],
        };

        let req = Self::gzip(&req.encode_to_vec())?;


        let mut url = Url::parse(&self.config.url)?; // todo parse url once
        url.path_segments_mut()
            .map_err(|_e| PyroscopeError::new("url construction failure - cannot_be_a_base"))?
            .push(Self::PUSHER_SERVICE_PUSH);

        let mut req_builder = client
            .post(url.as_str())
            .header("User-Agent", format!("pyroscope-rs/{} reqwest", self.config.spy_name))//todo version
            .header("Content-Type", "application/proto")
            .header("Content-Encoding", "gzip");

        if let Some(auth_token) = &self.config.auth_token {
            req_builder = req_builder.bearer_auth(auth_token);
        } else if let Some(basic_auth) = &self.config.basic_auth {
            req_builder = req_builder.basic_auth(
                basic_auth.username.clone(),
                Some(basic_auth.password.clone()),
            );
        }
        if let Some(tenant_id) = &self.config.tenant_id {
            req_builder = req_builder.header("X-Scope-OrgID", tenant_id);
        }
        for (k, v) in &self.config.http_headers {
            req_builder = req_builder.header(k, v);
        }

        let mut response = req_builder
            .body(req)
            .timeout(Duration::from_secs(10)) //todo allow configuration
            .send()?;

        let status = response.status();

        if status.is_success() {
            let mut sink = std::io::sink();
            _ = response.copy_to(&mut sink);
        } else {
            let resp = response.text();
            let resp = match &resp {
                Ok(t) => t,
                Err(_) => "",
            };
            log::error!(target: LOG_TAG, "Sending Session failed {} {}", status.as_u16(), resp);
        }
        Ok(())
    }

    fn gzip(report: &Vec<u8>) -> Result<Vec<u8>> {
        let mut encoder = Encoder::new(Vec::new())?;
        encoder.write_all(report)?;
        let compressed_data = encoder.finish().into_result()?;
        Ok(compressed_data)
    }
}
