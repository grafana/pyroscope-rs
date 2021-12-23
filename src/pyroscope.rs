// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use pprof::ProfilerGuardBuilder;

use tokio::sync::mpsc;

use libc::c_int;

use crate::error::Result;
use crate::utils::fold;
use crate::utils::merge_tags_with_app_name;
use crate::utils::pyroscope_ingest;
use crate::backends::backend::Backend;
use crate::backends::pprof::Pprof;

pub struct PyroscopeAgentBuilder {
    inner_builder: ProfilerGuardBuilder,
    backend: Box<dyn Backend>,

    url: String,
    application_name: String,
    tags: HashMap<String, String>,
    sample_rate: libc::c_int,
}

impl PyroscopeAgentBuilder {
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            inner_builder: ProfilerGuardBuilder::default(),
            url: url.as_ref().to_owned(),
            application_name: application_name.as_ref().to_owned(),
            tags: HashMap::new(),
            backend: Box::new(Pprof::new()), // Default Backend
            // TODO: This is set by default in pprof, probably should find a
            // way to force this to 100 at initialization.
            sample_rate: 99,
        }
    }

    pub fn backend(self, backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            ..self
        }
    }

    pub fn frequency(self, frequency: c_int) -> Self {
        Self {
            inner_builder: self.inner_builder.frequency(frequency),
            sample_rate: frequency,
            ..self
        }
    }

    pub fn blocklist<T: AsRef<str>>(self, blocklist: &[T]) -> Self {
        Self {
            inner_builder: self.inner_builder.blocklist(blocklist),
            ..self
        }
    }

    pub fn tags(self, tags: &[(&str, &str)]) -> Self {
        let ntags: HashMap<String, String> = tags
            .to_owned()
            .iter()
            .cloned()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();
        Self {
            tags: ntags,
            ..self
        }
    }

    pub fn build(self) -> Result<PyroscopeAgent> {
        Ok(PyroscopeAgent {
            inner_builder: self.inner_builder,
            url: self.url,
            application_name: self.application_name,
            tags: Arc::new(Mutex::new(self.tags)),
            sample_rate: self.sample_rate,
            stopper: None,
            handler: None,
            timer: None,
        })
    }
}

pub struct Timer {
    start_time: SystemTime,
    duration: Duration,
}

pub struct PyroscopeAgent {
    inner_builder: ProfilerGuardBuilder,

    url: String,
    application_name: String,
    tags: Arc<Mutex<HashMap<String, String>>>,
    sample_rate: libc::c_int,

    stopper: Option<mpsc::Sender<()>>,
    handler: Option<tokio::task::JoinHandle<Result<()>>>,

    timer: Option<Timer>,
}

impl PyroscopeAgent {
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        PyroscopeAgentBuilder::new(url, application_name)
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.stopper.take().unwrap().send(()).await?;
        self.handler.take().unwrap().await??;

        Ok(())
    }

    pub fn add_tags(&mut self, tags: &[(&str, &str)]) -> Result<()> {
        let ntags: HashMap<String, String> = tags
            .to_owned()
            .iter()
            .cloned()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();
        let itags = Arc::clone(&self.tags);
        let mut lock = itags.lock()?;
        lock.extend(ntags);

        Ok(())
    }

    pub fn remove_tags(&mut self, tags: &[&str]) -> Result<()> {
        let itags = Arc::clone(&self.tags);
        let mut lock = itags.lock()?;
        tags.iter().for_each(|key| {
            lock.remove(key.to_owned());
        });

        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        let application_name = self.application_name.clone();
        let new_tags = Arc::clone(&self.tags);
        let (stopper, mut stop_signal) = mpsc::channel::<()>(1);

        // Since Pyroscope only allow 10s intervals, it might not be necessary
        // to make this customizable at this point
        let upload_interval = std::time::Duration::from_secs(10);
        let mut interval = tokio::time::interval(upload_interval);

        let tmp = self.inner_builder.clone();
        let url_tmp = self.url.clone();
        let sample_rate = self.sample_rate.clone();

        let handler = tokio::spawn(async move {
            loop {
                match tmp.clone().build() {
                    Ok(guard) => {
                        tokio::select! {
                            _ = interval.tick() => {
                                let mut buffer = Vec::new();
                                let report = guard.report().build()?;
                                fold(&report, true, &mut buffer)?;
                                let start = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    ?
                                    .as_secs() - 10u64;
                                let t = new_tags.lock()?.clone();
                                let merged = merge_tags_with_app_name(application_name.clone(), t)?;
                                pyroscope_ingest(start, sample_rate, buffer, &url_tmp, merged).await?;
                            }
                            _ = stop_signal.recv() => {
                                let mut buffer = Vec::new();
                                let report = guard.report().build()?;
                                fold(&report, true, &mut buffer)?;
                                let start = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    ?
                                    .as_secs() - 10u64;
                                let t = new_tags.lock()?.clone();
                                let merged = merge_tags_with_app_name(application_name.clone(), t)?;
                                pyroscope_ingest(start, sample_rate, buffer, &url_tmp, merged).await?;

                                break Ok(())
                            }
                        }
                    }
                    Err(_err) => {
                        // TODO: this error will only be caught when this
                        // handler is joined. Find way to report error earlier
                        break Err(crate::error::PyroscopeError {
                            msg: String::from("Tokio Task Error"),
                        });
                    }
                }
            }
        });

        self.stopper = Some(stopper);
        self.handler = Some(handler);

        Ok(())
    }
}
