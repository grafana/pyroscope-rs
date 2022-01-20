// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread,
    thread::JoinHandle,
};

use crate::pyroscope::PyroscopeConfig;
use crate::utils::merge_tags_with_app_name;
use crate::Result;

/// Session Signal
#[derive(Debug)]
pub enum SessionSignal {
    Session(Session),
    Kill,
}

/// SessionManager
#[derive(Debug)]
pub struct SessionManager {
    pub handle: Option<JoinHandle<Result<()>>>,
    pub tx: SyncSender<SessionSignal>,
}

impl SessionManager {
    /// Create a new SessionManager
    pub fn new() -> Result<Self> {
        // Create a channel for sending and receiving sessions
        let (tx, rx): (SyncSender<SessionSignal>, Receiver<SessionSignal>) = sync_channel(10);

        // Create a thread for the SessionManager
        let handle = Some(thread::spawn(move || {
            while let Ok(signal) = rx.recv() {
                match signal {
                    SessionSignal::Session(session) => {
                        // Send the session
                        session.send()?;
                    }
                    SessionSignal::Kill => {
                        // Kill the session manager
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
        self.tx.send(session)?;
        Ok(())
    }
}

/// Pyroscope Session
#[derive(Clone, Debug)]
pub struct Session {
    pub config: PyroscopeConfig,
    pub report: Vec<u8>,
    pub from: u64,
    pub until: u64,
}

impl Session {
    pub fn new(mut until: u64, config: PyroscopeConfig, report: Vec<u8>) -> Result<Self> {
        // Session interrupted (0 signal), determine the time
        if until == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            until = now
                .checked_add(10u64.checked_sub(now.checked_rem(10).unwrap()).unwrap())
                .unwrap();
        }

        // Start of the session
        let from = until.checked_sub(10u64).unwrap();

        Ok(Self {
            config,
            report,
            from,
            until,
        })
    }

    pub fn send(self) -> Result<()> {
        let _handle: JoinHandle<Result<()>> = thread::spawn(move || {
            if self.report.is_empty() {
                return Ok(());
            }

            let client = reqwest::blocking::Client::new();
            // TODO: handle the error of this request

            // Clone URL
            let url = self.config.url.clone();

            // Merge application name with Tags
            let application_name = merge_tags_with_app_name(
                self.config.application_name.clone(),
                self.config.tags.clone(),
            )?;

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
                .send()?;

            return Ok(());
        });

        Ok(())
    }
}
