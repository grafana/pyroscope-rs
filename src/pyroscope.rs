// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};

use crate::backends::pprof::Pprof;
use crate::backends::Backend;
use crate::error::Result;
use crate::session::Session;
use crate::timer::Timer;

/// Represent PyroscopeAgent Configuration
#[derive(Clone, Debug)]
pub struct PyroscopeConfig {
    pub url: String,
    /// Application Name
    pub application_name: String,
    pub tags: HashMap<String, String>,
    /// Sample rate used in Hz
    pub sample_rate: i32,
    // TODO
    // log_level
    // auth_token
    // upstream_request_timeout = 10s
    // no_logging
}

impl PyroscopeConfig {
    /// Create a new PyroscopeConfig object. url and application_name are required.
    /// tags and sample_rate are optional.
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            application_name: application_name.as_ref().to_owned(),
            tags: HashMap::new(),
            sample_rate: 100i32,
        }
    }

    /// Set the Sample rate
    pub fn sample_rate(self, sample_rate: i32) -> Self {
        Self {
            sample_rate,
            ..self
        }
    }

    /// Set Tags
    pub fn tags(self, tags: &[(&str, &str)]) -> Self {
        // Convert &[(&str, &str)] to HashMap(String, String)
        let tags_hashmap: HashMap<String, String> = tags
            .to_owned()
            .iter()
            .cloned()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        Self {
            tags: tags_hashmap,
            ..self
        }
    }
}

/// PyroscopeAgent Builder
///
/// Alternatively, you can use PyroscopeAgent::build() which is a short-hand
/// for calling PyroscopeAgentBuilder::new()
pub struct PyroscopeAgentBuilder {
    /// Profiler backend
    backend: Arc<Mutex<dyn Backend>>,
    /// Configuration Object
    config: PyroscopeConfig,
}

impl PyroscopeAgentBuilder {
    /// Create a new PyroscopeConfig object. url and application_name are required.
    /// tags and sample_rate are optional.
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            backend: Arc::new(Mutex::new(Pprof::default())), // Default Backend
            config: PyroscopeConfig::new(url, application_name),
        }
    }

    /// Set the agent backend. Default is pprof.
    pub fn backend<T: 'static>(self, backend: T) -> Self
    where T: Backend {
        Self {
            backend: Arc::new(Mutex::new(backend)),
            ..self
        }
    }

    /// Set the Sample rate. Default value is 100.
    pub fn sample_rate(self, sample_rate: i32) -> Self {
        Self {
            config: self.config.sample_rate(sample_rate),
            ..self
        }
    }

    /// Set tags. Default is empty.
    pub fn tags(self, tags: &[(&str, &str)]) -> Self {
        Self {
            config: self.config.tags(tags),
            ..self
        }
    }

    /// Initialize the backend, timer and return a PyroscopeAgent object.
    pub fn build(self) -> Result<PyroscopeAgent> {
        // Initiliaze the backend
        let backend = Arc::clone(&self.backend);
        backend.lock()?.initialize(self.config.sample_rate)?;

        // Start Timer
        let timer = Timer::default().initialize()?;

        // Return PyroscopeAgent
        Ok(PyroscopeAgent {
            backend: self.backend,
            config: self.config,
            timer,
            tx: None,
            handle: None,
            running: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }
}

/// PyroscopeAgent
#[derive(Debug)]
pub struct PyroscopeAgent {
    pub backend: Arc<Mutex<dyn Backend>>,
    timer: Timer,
    tx: Option<Sender<u64>>,
    handle: Option<JoinHandle<Result<()>>>,
    running: Arc<(Mutex<bool>, Condvar)>,

    // Session Data
    pub config: PyroscopeConfig,
}

impl Drop for PyroscopeAgent {
    /// Properly shutdown the agent.
    fn drop(&mut self) {
        // Stop Timer
        self.timer.drop_listeners().unwrap(); // Drop listeners
        self.timer.handle.take().unwrap().join().unwrap().unwrap(); // Wait for the Timer thread to finish
    }
}

impl PyroscopeAgent {
    /// Short-hand for PyroscopeAgentBuilder::build()
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::new(url, application_name)
    }

    /// Start profiling and sending data. The agent will keep running until stopped.
    pub fn start(&mut self) -> Result<()> {
        // Create a clone of Backend
        let backend = Arc::clone(&self.backend);
        // Call start()
        backend.lock()?.start()?;

        // set running to true
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        let mut running = lock.lock()?;
        *running = true;
        drop(lock);
        drop(cvar);
        drop(running);

        let (tx, rx): (Sender<u64>, Receiver<u64>) = channel();
        self.timer.attach_listener(tx.clone())?;
        self.tx = Some(tx.clone());

        let config = self.config.clone();

        self.handle = Some(std::thread::spawn(move || {
            while let Ok(time) = rx.recv() {
                let report = backend.lock()?.report()?;
                // start a new session
                Session::new(time, config.clone(), report)?.send()?;

                if time == 0 {
                    let (lock, cvar) = &*pair;
                    let mut running = lock.lock()?;
                    *running = false;
                    cvar.notify_one();

                    return Ok(());
                }
            }

            return Ok(());
        }));

        Ok(())
    }

    /// Stop the agent.
    pub fn stop(&mut self) -> Result<()> {
        // get tx and send termination signal
        self.tx.take().unwrap().send(0)?;

        // Wait for the Thread to finish
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        let _guard = cvar.wait_while(lock.lock()?, |running| *running)?;

        // Create a clone of Backend
        let backend = Arc::clone(&self.backend);
        // Call stop()
        backend.lock()?.stop()?;

        Ok(())
    }

    /// Add tags. This will restart the agent.
    pub fn add_tags(&mut self, tags: &[(&str, &str)]) -> Result<()> {
        // Stop Agent
        self.stop()?;

        // Convert &[(&str, &str)] to HashMap(String, String)
        let tags_hashmap: HashMap<String, String> = tags
            .to_owned()
            .iter()
            .cloned()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        self.config.tags.extend(tags_hashmap);

        // Restart Agent
        self.start()?;

        Ok(())
    }

    /// Remove tags. This will restart the agent.
    pub fn remove_tags(&mut self, tags: &[&str]) -> Result<()> {
        // Stop Agent
        self.stop()?;

        // Iterate through every tag
        tags.iter().for_each(|key| {
            // Remove tag
            self.config.tags.remove(key.to_owned());
        });

        // Restart Agent
        self.start()?;

        Ok(())
    }
}
