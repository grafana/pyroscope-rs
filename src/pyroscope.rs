use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{
        mpsc::{self, Sender},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
};

use crate::{
    backend::{void_backend, BackendReady, BackendUninitialized, Rule, Tag, VoidConfig},
    error::Result,
    session::{Session, SessionManager, SessionSignal},
    timer::{Timer, TimerSignal},
    utils::get_time_range,
};

use crate::backend::BackendImpl;

const LOG_TAG: &str = "Pyroscope::Agent";

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
    /// Sample Rate
    pub sample_rate: u32,
    /// Spy Name
    pub spy_name: String,
}

impl PyroscopeConfig {
    /// Create a new PyroscopeConfig object. url and application_name are required.
    /// tags and sample_rate are optional. If sample_rate is not specified, it will default to 100.
    /// # Example
    /// ```ignore
    /// let config = PyroscopeConfig::new("http://localhost:8080", "my-app");
    /// ```
    pub fn new(url: impl AsRef<str>, application_name: impl AsRef<str>) -> Self {
        Self {
            url: url.as_ref().to_owned(), // Pyroscope Server URL
            application_name: application_name.as_ref().to_owned(), // Application Name
            tags: HashMap::new(),         // Empty tags
            sample_rate: 100u32,          // Default sample rate
            spy_name: String::from("undefined"), // Spy Name should be set by the backend
        }
    }

    /// Set the Sample rate.
    pub fn sample_rate(self, sample_rate: u32) -> Self {
        Self {
            sample_rate,
            ..self
        }
    }

    /// Set the Spy Name.
    pub fn spy_name(self, spy_name: String) -> Self {
        Self { spy_name, ..self }
    }

    /// Set the tags
    /// # Example
    /// ```ignore
    /// use pyroscope::pyroscope::PyroscopeConfig;
    /// let config = PyroscopeConfig::new("http://localhost:8080", "my-app")
    ///    .tags(vec![("env", "dev")])?;
    /// ```
    pub fn tags(self, tags: Vec<(&str, &str)>) -> Self {
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
/// let agent = builder.build()?;
/// ```
pub struct PyroscopeAgentBuilder {
    /// Profiler backend
    backend: BackendImpl<BackendUninitialized>,
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
    pub fn new(url: impl AsRef<str>, application_name: impl AsRef<str>) -> Self {
        Self {
            backend: void_backend(VoidConfig::default()), // Default Backend
            config: PyroscopeConfig::new(url, application_name),
        }
    }

    /// Set the agent backend. Default is pprof.
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .backend(Pprof::default())
    /// .build()
    /// ?;
    /// ```
    pub fn backend(self, backend: BackendImpl<BackendUninitialized>) -> Self {
        Self { backend, ..self }
    }

    /// Set tags. Default is empty.
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .tags(vec![("env", "dev")])
    /// .build()?;
    /// ```
    pub fn tags(self, tags: Vec<(&str, &str)>) -> Self {
        Self {
            config: self.config.tags(tags),
            ..self
        }
    }

    /// Initialize the backend, timer and return a PyroscopeAgent object.
    pub fn build(self) -> Result<PyroscopeAgent<PyroscopeAgentReady>> {
        // Get the backend
        //let backend = Arc::clone(&self.backend);

        // Set Spy Name and Sample Rate from the Backend
        let config = self.config.sample_rate(self.backend.sample_rate()?);
        let config = config.spy_name(self.backend.spy_name()?);

        // Set Global Tags
        for (k, v) in config.tags.iter() {
            self.backend
                .add_rule(crate::backend::Rule::GlobalTag(Tag::new(
                    k.to_owned(),
                    v.to_owned(),
                )))?;
        }

        // Initialize the backend
        let backend_ready = self.backend.initialize()?;

        log::trace!(target: LOG_TAG, "Backend initialized");

        // Start Timer
        let timer = Timer::initialize(std::time::Duration::from_secs(10))?;
        log::trace!(target: LOG_TAG, "Timer initialized");

        // Start the SessionManager
        let session_manager = SessionManager::new()?;
        log::trace!(target: LOG_TAG, "SessionManager initialized");

        // Return PyroscopeAgent
        Ok(PyroscopeAgent {
            backend: backend_ready,
            config,
            timer,
            session_manager,
            tx: None,
            handle: None,
            running: Arc::new((Mutex::new(false), Condvar::new())),
            _state: PhantomData,
        })
    }
}

pub trait PyroscopeAgentState {}
pub struct PyroscopeAgentBare;
pub struct PyroscopeAgentReady;
pub struct PyroscopeAgentRunning;
impl PyroscopeAgentState for PyroscopeAgentBare {}
impl PyroscopeAgentState for PyroscopeAgentReady {}
impl PyroscopeAgentState for PyroscopeAgentRunning {}

/// PyroscopeAgent is the main object of the library. It is used to start and stop the profiler, schedule the timer, and send the profiler data to the server.
#[derive(Debug)]
pub struct PyroscopeAgent<S: PyroscopeAgentState> {
    timer: Timer,
    session_manager: SessionManager,
    tx: Option<Sender<TimerSignal>>,
    handle: Option<JoinHandle<Result<()>>>,
    running: Arc<(Mutex<bool>, Condvar)>,

    /// Profiler backend
    pub backend: BackendImpl<BackendReady>,
    /// Configuration Object
    pub config: PyroscopeConfig,
    _state: PhantomData<S>,
}

/// Gracefully stop the profiler.
impl<S: PyroscopeAgentState> PyroscopeAgent<S> {
    /// Properly shutdown the agent.
    pub fn shutdown(mut self) {
        log::debug!(target: LOG_TAG, "PyroscopeAgent::drop()");

        // Drop Timer listeners
        match self.timer.drop_listeners() {
            Ok(_) => log::trace!(target: LOG_TAG, "Dropped timer listeners"),
            Err(_) => log::error!(target: LOG_TAG, "Error Dropping timer listeners"),
        }

        // Wait for the Timer thread to finish
        if let Some(handle) = self.timer.handle.take() {
            match handle.join() {
                Ok(_) => log::trace!(target: LOG_TAG, "Dropped timer thread"),
                Err(_) => log::error!(target: LOG_TAG, "Error Dropping timer thread"),
            }
        }

        // Stop the SessionManager
        match self.session_manager.push(SessionSignal::Kill) {
            Ok(_) => log::trace!(target: LOG_TAG, "Sent kill signal to SessionManager"),
            Err(_) => log::error!(
                target: LOG_TAG,
                "Error sending kill signal to SessionManager"
            ),
        }

        if let Some(handle) = self.session_manager.handle.take() {
            match handle.join() {
                Ok(_) => log::trace!(target: LOG_TAG, "Dropped SessionManager thread"),
                Err(_) => log::error!(target: LOG_TAG, "Error Dropping SessionManager thread"),
            }
        }

        // Wait for main thread to finish
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(_) => log::trace!(target: LOG_TAG, "Dropped main thread"),
                Err(_) => log::error!(target: LOG_TAG, "Error Dropping main thread"),
            }
        }

        log::debug!(target: LOG_TAG, "Agent Dropped");
    }
}

impl PyroscopeAgent<PyroscopeAgentBare> {
    /// Short-hand for PyroscopeAgentBuilder::build(). This is a convenience method.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// ```
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::new(url, application_name)
    }
}

impl PyroscopeAgent<PyroscopeAgentReady> {
    /// Start profiling and sending data. The agent will keep running until stopped. The agent will send data to the server every 10s secondy.
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// agent.start()?;
    /// ```
    pub fn start(mut self) -> Result<PyroscopeAgent<PyroscopeAgentRunning>> {
        log::debug!(target: LOG_TAG, "Starting");

        // Create a clone of Backend
        let backend = Arc::clone(&self.backend.backend);
        // Call start()

        // set running to true
        let pair = Arc::clone(&self.running);
        let (lock, _cvar) = &*pair;
        let mut running = lock.lock()?;
        *running = true;
        drop(running);

        let (tx, rx) = mpsc::channel();
        self.timer.attach_listener(tx.clone())?;
        self.tx = Some(tx);

        let config = self.config.clone();

        let stx = self.session_manager.tx.clone();

        self.handle = Some(std::thread::spawn(move || {
            log::trace!(target: LOG_TAG, "Main Thread started");

            while let Ok(signal) = rx.recv() {
                match signal {
                    TimerSignal::NextSnapshot(until) => {
                        log::trace!(target: LOG_TAG, "Sending session {}", until);

                        // Generate report from backend
                        let report = backend.lock()?.as_mut().unwrap().report()?;

                        // Send new Session to SessionManager
                        stx.send(SessionSignal::Session(Session::new(
                            until,
                            config.clone(),
                            report,
                        )?))?
                    }
                    TimerSignal::Terminate => {
                        log::trace!(target: LOG_TAG, "Session Killed");

                        let (lock, cvar) = &*pair;
                        let mut running = lock.lock()?;
                        *running = false;
                        cvar.notify_one();

                        return Ok(());
                    }
                }
            }
            Ok(())
        }));

        let agent_running = PyroscopeAgent {
            timer: self.timer,
            session_manager: self.session_manager,
            tx: self.tx,
            handle: self.handle,
            running: self.running,
            backend: self.backend,
            config: self.config,
            _state: PhantomData,
        };

        Ok(agent_running)
    }
}
impl PyroscopeAgent<PyroscopeAgentRunning> {
    /// Stop the agent. The agent will stop profiling and send a last report to the server.
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// agent.start()?;
    /// // Expensive operation
    /// agent.stop();
    /// ```
    pub fn stop(mut self) -> Result<PyroscopeAgent<PyroscopeAgentReady>> {
        log::debug!(target: LOG_TAG, "Stopping");
        // get tx and send termination signal
        if let Some(sender) = self.tx.take() {
            // Send last session
            let _ = sender.send(TimerSignal::NextSnapshot(get_time_range(0)?.until));
            sender.send(TimerSignal::Terminate)?;
        } else {
            log::error!("PyroscopeAgent - Missing sender")
        }

        // Wait for the Thread to finish
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        let _guard = cvar.wait_while(lock.lock()?, |running| *running)?;

        // Create a clone of Backend
        //let backend = Arc::clone(&self.backend);

        let agent_running = PyroscopeAgent {
            timer: self.timer,
            session_manager: self.session_manager,
            tx: self.tx,
            handle: self.handle,
            running: self.running,
            backend: self.backend,
            config: self.config,
            _state: PhantomData,
        };

        Ok(agent_running)
    }

    pub fn tag_wrapper(
        &self,
    ) -> (
        impl Fn(String, String) -> Result<()>,
        impl Fn(String, String) -> Result<()>,
    ) {
        let backend_add = self.backend.backend.clone();
        let backend_remove = self.backend.backend.clone();

        (
            move |key, value| {
                let thread_id = crate::utils::pthread_self()?;
                let rule = Rule::ThreadTag(thread_id, Tag::new(key, value));
                let backend = backend_add.lock()?;
                backend.as_ref().unwrap().add_rule(rule)?;

                Ok(())
            },
            move |key, value| {
                let thread_id = crate::utils::pthread_self()?;
                let rule = Rule::ThreadTag(thread_id, Tag::new(key, value));
                let backend = backend_remove.lock()?;
                backend.as_ref().unwrap().remove_rule(rule)?;

                Ok(())
            },
        )
    }

    // TODO: change &mut self to &self
    pub fn add_global_tag(&mut self, tag: Tag) -> Result<()> {
        let rule = Rule::GlobalTag(tag);
        self.backend.add_rule(rule)?;

        Ok(())
    }

    pub fn remove_global_tag(&mut self, tag: Tag) -> Result<()> {
        let rule = Rule::GlobalTag(tag);
        self.backend.remove_rule(rule)?;

        Ok(())
    }

    pub fn add_thread_tag(&mut self, thread_id: u64, tag: Tag) -> Result<()> {
        let rule = Rule::ThreadTag(thread_id, tag);
        self.backend.add_rule(rule)?;

        Ok(())
    }

    pub fn remove_thread_tag(&mut self, thread_id: u64, tag: Tag) -> Result<()> {
        let rule = Rule::ThreadTag(thread_id, tag);
        self.backend.remove_rule(rule)?;

        Ok(())
    }
}
