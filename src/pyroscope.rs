// Copyright 2021 TiKV Project Authors. Licensed under Apache-2.0.

//! this mod could help you to upload profiler data to the pyroscope
//!
//! To enable this mod, you need to enable the features: "pyroscope" and
//! "default-tls" (or "rustls-tls"). To start profiling, you can create a
//! `PyroscopeAgent`:
//!
//! ```ignore
//! let guard =  
//!   PyroscopeAgentBuilder::new("http://localhost:4040".to_owned(), "fibonacci".to_owned())
//!     .frequency(99)
//!     .tags([
//!         ("TagA".to_owned(), "ValueA".to_owned()),
//!         ("TagB".to_owned(), "ValueB".to_owned()),
//!     ]
//!     .iter()
//!     .cloned()
//!     .collect())
//!     .build().unwrap();
//! ```
//!
//! This guard will collect profiling data and send profiling data to the
//! pyroscope server every 10 seconds. This interval is not configurable now
//! (both server side and client side).
//!
//! If you need to stop the profiling, you can call `stop()` on the guard:
//!
//! ```ignore
//! guard.stop().await
//! ```
//!
//! It will return the error if error occurs while profiling.

use std::collections::HashMap;

use pprof::ProfilerGuardBuilder;
use pprof::Result;

use tokio::sync::mpsc;

use libc::c_int;
use crate::utils::pyroscope_ingest;
use crate::utils::merge_tags_with_app_name;

pub struct PyroscopeAgentBuilder {
    inner_builder: ProfilerGuardBuilder,

    url: String,
    application_name: String,
    tags: HashMap<String, String>,
}

impl PyroscopeAgentBuilder {
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            inner_builder: ProfilerGuardBuilder::default(),
            url: url.as_ref().to_owned(),
            application_name: application_name.as_ref().to_owned(),
            tags: HashMap::new(),
        }
    }

    pub fn frequency(self, frequency: c_int) -> Self {
        Self {
            inner_builder: self.inner_builder.frequency(frequency),
            ..self
        }
    }

    pub fn blocklist<T: AsRef<str>>(self, blocklist: &[T]) -> Self {
        Self {
            inner_builder: self.inner_builder.blocklist(blocklist),
            ..self
        }
    }

    pub fn tags(self, tags: HashMap<String, String>) -> Self {
        Self { tags, ..self }
    }

    pub fn build(self) -> Result<PyroscopeAgent> {
        Ok(PyroscopeAgent { 
            inner_builder: self.inner_builder,
            url: self.url,
            application_name: self.application_name,
            tags: self.tags,
            stopper: None,
            handler: None,
        })
    }
}

pub struct PyroscopeAgent {
    inner_builder: ProfilerGuardBuilder,

    url: String,
    application_name: String,
    tags: HashMap<String, String>,

    stopper: Option<mpsc::Sender<()>>,
    handler: Option<tokio::task::JoinHandle<Result<()>>>,
}

impl PyroscopeAgent {
    pub async fn stop(&mut self) -> Result<()> {
        self.stopper.take().unwrap().send(()).await.unwrap();
        self.handler.take().unwrap().await.unwrap()?;

        Ok(())
    }
    
    pub fn start(&mut self) -> Result<()> {
        let application_name = merge_tags_with_app_name(self.application_name.clone(), self.tags.clone());
        let (stopper, mut stop_signal) = mpsc::channel::<()>(1);

        // Since Pyroscope only allow 10s intervals, it might not be necessary
        // to make this customizable at this point
        let upload_interval = std::time::Duration::from_secs(10);
        let mut interval = tokio::time::interval(upload_interval);

        let tmp = self.inner_builder.clone();
        let url_tmp = self.url.clone();
        let handler = tokio::spawn(async move {
            loop {
                match tmp.clone().build() {
                    Ok(guard) => {
                        tokio::select! {
                            _ = interval.tick() => {
                                pyroscope_ingest(guard.report().build()?, &url_tmp, &application_name).await?;
                            }
                            _ = stop_signal.recv() => {
                                pyroscope_ingest(guard.report().build()?, &url_tmp, &application_name).await?;

                                break Ok(())
                            }
                        }
                    }
                    Err(err) => {
                        // TODO: this error will only be caught when this
                        // handler is joined. Find way to report error earlier
                        break Err(err);
                    }
                }
            }
        });

        self.stopper = Some(stopper);
        self.handler = Some(handler);

        Ok(())
    }
}
