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
use crate::session::SessionManager;
use crate::session::SessionSignal;
use crate::timer::Timer;

/// Pyroscope Agent Configuration. This is the configuration that is passed to the agent.
/// # Example
/// ```
/// use pyroscope::pyroscope::PyroscopeConfig;
/// let config = PyroscopeConfig::new("http://localhost:8080", "my-app");
/// ```
#[derive(Clone, Debug)]
pub struct PyroscopeConfig {
    /// Pyroscope Server Address
    pub url: String,
    /// Application Name
    pub application_name: String,
    /// Tags
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
    /// tags and sample_rate are optional. If sample_rate is not specified, it will default to 100.
    /// # Example
    /// ```ignore
    /// let config = PyroscopeConfig::new("http://localhost:8080", "my-app");
    /// ```
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            application_name: application_name.as_ref().to_owned(),
            tags: HashMap::new(),
            sample_rate: 100i32,
        }
    }

    /// Set the Sample rate
    /// # Example
    /// ```ignore
    /// let mut config = PyroscopeConfig::new("http://localhost:8080", "my-app");
    /// config.set_sample_rate(10)
    /// .unwrap();
    /// ```
    pub fn sample_rate(self, sample_rate: i32) -> Self {
        Self {
            sample_rate,
            ..self
        }
    }

    /// Set the tags
    /// # Example
    /// ```ignore
    /// use pyroscope::pyroscope::PyroscopeConfig;
    /// let config = PyroscopeConfig::new("http://localhost:8080", "my-app")
    ///    .tags(vec![("env", "dev")])
    ///    .unwrap();
    /// ```
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
///
/// # Example
/// ```ignore
/// use pyroscope::pyroscope::PyroscopeAgentBuilder;
/// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app");
/// let agent = builder.build().unwrap();
/// ```
pub struct PyroscopeAgentBuilder {
    /// Profiler backend
    backend: Arc<Mutex<dyn Backend>>,
    /// Configuration Object
    config: PyroscopeConfig,
}

impl PyroscopeAgentBuilder {
    /// Create a new PyroscopeAgentBuilder object. url and application_name are required.
    /// tags and sample_rate are optional.
    ///
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app");
    /// ```
    pub fn new<S: AsRef<str>>(url: S, application_name: S) -> Self {
        Self {
            backend: Arc::new(Mutex::new(Pprof::default())), // Default Backend
            config: PyroscopeConfig::new(url, application_name),
        }
    }

    /// Set the agent backend. Default is pprof.
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .backend(Pprof::default())
    /// .build()
    /// .unwrap();
    /// ```
    pub fn backend<T: 'static>(self, backend: T) -> Self
    where
        T: Backend,
    {
        Self {
            backend: Arc::new(Mutex::new(backend)),
            ..self
        }
    }

    /// Set the Sample rate. Default value is 100.
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .sample_rate(99)
    /// .build()
    /// .unwrap();
    /// ```
    pub fn sample_rate(self, sample_rate: i32) -> Self {
        Self {
            config: self.config.sample_rate(sample_rate),
            ..self
        }
    }

    /// Set tags. Default is empty.
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .tags(vec![("env", "dev")])
    /// .build()
    /// .unwrap();
    /// ```
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
        log::trace!("PyroscopeAgent - Backend initialized");

        // Start Timer
        let timer = Timer::default().initialize()?;
        log::trace!("PyroscopeAgent - Timer initialized");

        // Start the SessionManager
        let session_manager = SessionManager::new()?;
        log::trace!("PyroscopeAgent - SessionManager initialized");

        // Return PyroscopeAgent
        Ok(PyroscopeAgent {
            backend: self.backend,
            config: self.config,
            timer,
            session_manager,
            tx: None,
            handle: None,
            running: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }
}

/// PyroscopeAgent is the main object of the library. It is used to start and stop the profiler, schedule the timer, and send the profiler data to the server.
#[derive(Debug)]
pub struct PyroscopeAgent {
    timer: Timer,
    session_manager: SessionManager,
    tx: Option<Sender<u64>>,
    handle: Option<JoinHandle<Result<()>>>,
    running: Arc<(Mutex<bool>, Condvar)>,

    /// Profiler backend
    pub backend: Arc<Mutex<dyn Backend>>,
    /// Configuration Object
    pub config: PyroscopeConfig,
}

impl Drop for PyroscopeAgent {
    /// Properly shutdown the agent.
    fn drop(&mut self) {
        log::debug!("PyroscopeAgent::drop()");

        // Drop Timer listeners
        match self.timer.drop_listeners() {
            Ok(_) => log::trace!("PyroscopeAgent - Dropped timer listeners"),
            Err(_) => log::error!("PyroscopeAgent - Error Dropping timer listeners"),
        }

        // Wait for the Timer thread to finish
        match self.timer.handle.take().unwrap().join() {
            Ok(_) => log::trace!("PyroscopeAgent - Dropped timer thread"),
            Err(_) => log::error!("PyroscopeAgent - Error Dropping timer thread"),
        }

        // Stop the SessionManager
        match self.session_manager.push(SessionSignal::Kill) {
            Ok(_) => log::trace!("PyroscopeAgent - Sent kill signal to SessionManager"),
            Err(_) => log::error!("PyroscopeAgent - Error sending kill signal to SessionManager"),
        }

        // Stop SessionManager
        match self.session_manager.handle.take().unwrap().join() {
            Ok(_) => log::trace!("PyroscopeAgent - Dropped SessionManager thread"),
            Err(_) => log::error!("PyroscopeAgent - Error Dropping SessionManager thread"),
        }

        // Wait for main thread to finish
        match self.handle.take().unwrap().join() {
            Ok(_) => log::trace!("PyroscopeAgent - Dropped main thread"),
            Err(_) => log::error!("PyroscopeAgent - Error Dropping main thread"),
        }
    }
}

impl PyroscopeAgent {
    /// Short-hand for PyroscopeAgentBuilder::build(). This is a convenience method.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build().unwrap();
    /// ```
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::new(url, application_name)
    }

    /// Start profiling and sending data. The agent will keep running until stopped. The agent will send data to the server every 10s secondy.
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build().unwrap();
    /// agent.start().unwrap();
    /// ```
    fn _start(&mut self) -> Result<()> {
        log::debug!("PyroscopeAgent - Starting");

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

        let stx = self.session_manager.tx.clone();

        self.handle = Some(std::thread::spawn(move || {
            log::trace!("PyroscopeAgent - Main Thread started");

            while let Ok(time) = rx.recv() {
                log::trace!("PyroscopeAgent - Sending session {}", time);

                // Generate report from backend
                let report = backend.lock()?.report()?;

                // Send new Session to SessionManager
                stx.send(SessionSignal::Session(Session::new(
                    time,
                    config.clone(),
                    report,
                )?))?;

                if time == 0 {
                    log::trace!("PyroscopeAgent - Session Killed");

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

    pub fn start(&mut self) {
        match self._start() {
            Ok(_) => log::trace!("PyroscopeAgent - Agent started"),
            Err(_) => log::error!("PyroscopeAgent - Error starting agent"),
        }
    }

    /// Stop the agent. The agent will stop profiling and send a last report to the server.
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build().unwrap();
    /// agent.start().unwrap();
    /// // Expensive operation
    /// agent.stop().unwrap();
    /// ```
    fn _stop(&mut self) -> Result<()> {
        log::debug!("PyroscopeAgent - Stopping");
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

    pub fn stop(&mut self) {
        match self._stop() {
            Ok(_) => log::trace!("PyroscopeAgent - Agent stopped"),
            Err(_) => log::error!("PyroscopeAgent - Error stopping agent"),
        }
    }

    /// Add tags. This will restart the agent.
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build().unwrap();
    /// agent.start().unwrap();
    /// // Expensive operation
    /// agent.add_tags(vec!["tag", "value"]).unwrap();
    /// // Tagged operation
    /// agent.stop().unwrap();
    /// ```
    pub fn add_tags(&mut self, tags: &[(&str, &str)]) -> Result<()> {
        log::debug!("PyroscopeAgent - Adding tags");
        // Check that tags are not empty
        if tags.is_empty() {
            return Ok(());
        }

        // Stop Agent
        self.stop();

        // Convert &[(&str, &str)] to HashMap(String, String)
        let tags_hashmap: HashMap<String, String> = tags
            .to_owned()
            .iter()
            .cloned()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        self.config.tags.extend(tags_hashmap);

        // Restart Agent
        self.start();

        Ok(())
    }

    /// Remove tags. This will restart the agent.
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app")
    /// .tags(vec![("tag", "value")])
    /// .build().unwrap();
    /// agent.start().unwrap();
    /// // Expensive operation
    /// agent.remove_tags(vec!["tag"]).unwrap();
    /// // Un-Tagged operation
    /// agent.stop().unwrap();
    pub fn remove_tags(&mut self, tags: &[&str]) -> Result<()> {
        log::debug!("PyroscopeAgent - Removing tags");

        // Check that tags are not empty
        if tags.is_empty() {
            return Ok(());
        }

        // Stop Agent
        self.stop();

        // Iterate through every tag
        tags.iter().for_each(|key| {
            // Remove tag
            self.config.tags.remove(key.to_owned());
        });

        // Restart Agent
        self.start();

        Ok(())
    }
}
