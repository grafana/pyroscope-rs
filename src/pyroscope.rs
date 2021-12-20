use std::collections::HashMap;

use pprof::ProfilerGuardBuilder;

use tokio::sync::mpsc;

use libc::c_int;

use crate::utils::pyroscope_ingest;
use crate::utils::merge_tags_with_app_name;
use crate::error::Result;

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
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
       PyroscopeAgentBuilder::new(url, application_name) 
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.stopper.take().unwrap().send(()).await?;
        self.handler.take().unwrap().await??;

        Ok(())
    }
    
    pub fn start(&mut self) -> Result<()> {
        let application_name = merge_tags_with_app_name(self.application_name.clone(), self.tags.clone())?;
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
                                let report = guard.report().build()?;
                                pyroscope_ingest(report, &url_tmp, &application_name).await?;
                            }
                            _ = stop_signal.recv() => {
                                let report = guard.report().build()?;
                                pyroscope_ingest(report, &url_tmp, &application_name).await?;

                                break Ok(())
                            }
                        }
                    }
                    Err(_err) => {
                        // TODO: this error will only be caught when this
                        // handler is joined. Find way to report error earlier
                        break Err(crate::error::PyroscopeError {msg: String::from("Tokio Task Error")});
                    }
                }
            }
        });

        self.stopper = Some(stopper);
        self.handler = Some(handler);

        Ok(())
    }
}
