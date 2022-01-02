// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
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

#[derive(Clone, Debug)]
pub struct PyroscopeConfig {
    pub url: String,
    pub application_name: String,
    pub tags: HashMap<String, String>,
    pub sample_rate: i32,

    // TODO
    // log_level
    // auth_token
    // upstream_request_timeout = 10s
    // no_logging
}

impl PyroscopeConfig {
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            application_name: application_name.as_ref().to_owned(),
            tags: HashMap::new(),
            sample_rate: 100i32,
        }
    }

    pub fn sample_rate(self, sample_rate: i32) -> Self {
        Self {
            sample_rate,
            ..self
        }
    }
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

pub struct PyroscopeAgentBuilder {
    backend: Arc<Mutex<dyn Backend>>,
    config: PyroscopeConfig,
}

impl PyroscopeAgentBuilder {
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            backend: Arc::new(Mutex::new(Pprof::default())), // Default Backend
            config: PyroscopeConfig::new(url, application_name),
        }
    }

    pub fn backend<T: 'static>(self, backend: T) -> Self
    where
        T: Backend,
    {
        Self {
            backend: Arc::new(Mutex::new(backend)),
            ..self
        }
    }

    pub fn sample_rate(self, sample_rate: i32) -> Self {
        Self {
            config: self.config.sample_rate(sample_rate),
            ..self
        }
    }

    pub fn tags(self, tags: &[(&str, &str)]) -> Self {
        Self {
            config: self.config.tags(tags),
            ..self
        }
    }

    pub fn build(self) -> Result<PyroscopeAgent> {
        // Initiliaze the backend
        let backend = Arc::clone(&self.backend);
        backend.lock()?.initialize(self.config.sample_rate)?;

        // Start Timer
        let timer = Timer::default().initialize();

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

#[derive(Debug)]
pub struct PyroscopeAgent {
    pub backend: Arc<Mutex<dyn Backend>>,
    timer: Timer,
    tx: Option<Sender<u64>>,
    handle: Option<JoinHandle<()>>,
    running: Arc<(Mutex<bool>, Condvar)>,

    // Session Data
    pub config: PyroscopeConfig,
}

impl PyroscopeAgent {
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::new(url, application_name)
    }

    pub fn start(&mut self) -> Result<()> {
        // Create a clone of Backend
        let backend = Arc::clone(&self.backend);
        // Call start()
        backend.lock()?.start()?;

        // set running to true
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        let mut running = lock.lock().unwrap();
        *running = true;
        drop(lock);
        drop(cvar);
        drop(running);

        //self.scheduler.tx.send(Event::Start).unwrap();
        let (tx, rx): (Sender<u64>, Receiver<u64>) = channel();
        self.timer.attach_listener(tx.clone()).unwrap();
        self.tx = Some(tx.clone());

        let config = self.config.clone();

        self.handle = Some(std::thread::spawn(move || {
            while let Ok(time) = rx.recv() {
                let report = backend.lock().unwrap().report().unwrap();
                // start a new session
                Session::new(time, config.clone(), report).send().unwrap();

                if time == 0 {
                    let (lock, cvar) = &*pair;
                    let mut running = lock.lock().unwrap();
                    *running = false;
                    cvar.notify_one();

                    return;
                }
            }
        }));

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        // get tx and send termination signal
        self.tx.take().unwrap().send(0).unwrap();

        // Wait for the Thread to finish
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        cvar.wait_while(lock.lock().unwrap(), |running| *running)
            .unwrap();

        // Create a clone of Backend
        let backend = Arc::clone(&self.backend);
        // Call stop()
        backend.lock()?.stop()?;

        Ok(())
    }

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
