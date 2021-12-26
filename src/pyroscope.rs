// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::error::Result;
use crate::backends::Backend;
use crate::backends::pprof::Pprof;

pub struct PyroscopeAgentBuilder {
    backend: Arc<Mutex<dyn Backend>>,

    url: String,
    application_name: String,
    tags: HashMap<String, String>,

    sample_rate: i32,
}

impl PyroscopeAgentBuilder {
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            application_name: application_name.as_ref().to_owned(),
            tags: HashMap::new(),
            backend: Arc::new(Mutex::new(Pprof::default())), // Default Backend
            // TODO: This is set by default in pprof, probably should find a
            // way to force this to 100 at initialization.
            sample_rate: 99,
        }
    }

    pub fn backend<T: 'static>(self, backend: T) -> Self where T: Backend {
        Self {
            backend: Arc::new(Mutex::new(backend)),
            ..self
        }
    }

    pub fn frequency(self, frequency: i32) -> Self {
        Self {
            sample_rate: frequency,
            ..self
        }
    }

    pub fn blocklist<T: AsRef<str>>(self, blocklist: &[T]) -> Self {
        Self {
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
        // Initiliaze the backend
        let a = Arc::clone(&self.backend);
        let mut lock = a.lock()?;
        lock.initialize(self.sample_rate)?;

        // Return PyroscopeAgent
        Ok(PyroscopeAgent {
            backend: self.backend,
            url: self.url,
            application_name: self.application_name,
            tags: Arc::new(Mutex::new(self.tags)),
            sample_rate: self.sample_rate,
        })
    }
}

pub struct PyroscopeAgent {
    backend: Arc<Mutex<dyn Backend>>,

    url: String,
    application_name: String,
    tags: Arc<Mutex<HashMap<String, String>>>,
    sample_rate: i32,
}

impl PyroscopeAgent {
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        PyroscopeAgentBuilder::new(url, application_name)
    }

    pub fn stop(&mut self) -> Result<()> {
        // Stop Backend
        let a = Arc::clone(&self.backend);
        let mut lock = a.lock()?;
        lock.stop()?;

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
        // Start Backend
        let a = Arc::clone(&self.backend);
        let mut lock = a.lock()?;
        lock.start()?;

        Ok(())
    }
}
