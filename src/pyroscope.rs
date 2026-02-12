use std::{
    collections::HashMap,
    marker::PhantomData,
    str::FromStr,
    sync::{
        mpsc::{self, Sender},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
};

use crate::{
    backend::{BackendReady, BackendUninitialized, Report, Tag},
    error::Result,
    session::{Session, SessionManager, SessionSignal},
    timer::{Timer, TimerSignal},
    utils::get_time_range,
    PyroscopeError,
};

use crate::backend::{BackendImpl, ThreadTag};
use crate::pyroscope::Compression::GZIP;

const LOG_TAG: &str = "Pyroscope::Agent";

/// Pyroscope Agent Configuration. This is the configuration that is passed to the agent.
///
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
    /// Authentication Token
    pub auth_token: Option<String>,
    pub basic_auth: Option<BasicAuth>,
    /// Function to apply
    pub func: Option<fn(Report) -> Report>,
    /// Pyroscope http request body compression
    pub compression: Option<Compression>,
    pub tenant_id: Option<String>,
    pub http_headers: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl Default for PyroscopeConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:4040".to_string(),
            application_name: names::Generator::default()
                .next()
                .unwrap_or_else(|| "unassigned.app".to_string())
                .replace('-', "."),
            tags: HashMap::new(),
            sample_rate: 100u32,
            spy_name: "undefined".to_string(),
            auth_token: None,
            basic_auth: None,
            func: None,
            compression: Some(GZIP),
            tenant_id: None,
            http_headers: HashMap::new(),
        }
    }
}

impl PyroscopeConfig {
    /// Create a new PyroscopeConfig object. url and application_name are required.
    /// tags and sample_rate are optional. If sample_rate is not specified, it will default to 100.
    ///
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
            auth_token: None,             // No authentication token
            basic_auth: None,
            func: None, // No function
            compression: Some(GZIP),
            tenant_id: None,
            http_headers: HashMap::new(),
        }
    }

    // Set the Pyroscope Server URL
    pub fn url(self, url: impl AsRef<str>) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            ..self
        }
    }

    // Set the Application Name
    pub fn application_name(self, application_name: impl AsRef<str>) -> Self {
        Self {
            application_name: application_name.as_ref().to_owned(),
            ..self
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

    pub fn auth_token(self, auth_token: String) -> Self {
        Self {
            auth_token: Some(auth_token),
            ..self
        }
    }

    pub fn basic_auth(self, username: String, password: String) -> Self {
        Self {
            basic_auth: Some(BasicAuth { username, password }),
            ..self
        }
    }

    /// Set the Function.
    pub fn func(self, func: fn(Report) -> Report) -> Self {
        Self {
            func: Some(func),
            ..self
        }
    }

    /// Set the tags.
    ///
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

    /// Set the http request body compression.
    ///
    /// # Example
    /// ```ignore
    /// use pyroscope::pyroscope::PyroscopeConfig;
    /// let config = PyroscopeConfig::new("http://localhost:8080", "my-app")
    ///     .compression(GZIP);
    /// ```
    pub fn compression(self, compression: Compression) -> Self {
        Self {
            compression: Some(compression),
            ..self
        }
    }

    pub fn tenant_id(self, tenant_id: String) -> Self {
        Self {
            tenant_id: Some(tenant_id),
            ..self
        }
    }

    pub fn http_headers(self, http_headers: HashMap<String, String>) -> Self {
        Self {
            http_headers,
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
    pub fn new(url: impl AsRef<str>, application_name: impl AsRef<str>, backend: BackendImpl<BackendUninitialized>) -> Self {
        Self {
            backend,
            config: PyroscopeConfig::new(url, application_name),
        }
    }

    /// Set the Pyroscope Server URL. This can be used if the Builder was initialized with the default
    /// trait. Default is "http://localhost:4040".
    ///
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::default()
    /// .url("http://localhost:8080")
    /// .build()?;
    /// ```
    pub fn url(self, url: impl AsRef<str>) -> Self {
        Self {
            config: self.config.url(url),
            ..self
        }
    }

    /// Set the Application Name. This can be used if the Builder was initialized with the default
    /// trait. Default is a randomly generated name.
    ///
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::default()
    /// .application_name("my-app")
    /// .build()?;
    /// ```
    pub fn application_name(self, application_name: impl AsRef<str>) -> Self {
        Self {
            config: self.config.application_name(application_name),
            ..self
        }
    }
    

    /// Set JWT authentication token.
    /// This is optional. If not set, the agent will not send any authentication token.
    ///
    /// #Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .auth_token("my-token")
    /// .build()
    /// ?;
    /// ```
    pub fn auth_token(self, auth_token: impl AsRef<str>) -> Self {
        Self {
            config: self.config.auth_token(auth_token.as_ref().to_owned()),
            ..self
        }
    }

    pub fn basic_auth(self, username: impl AsRef<str>, password: impl AsRef<str>) -> Self {
        Self {
            config: self
                .config
                .basic_auth(username.as_ref().to_owned(), password.as_ref().to_owned()),
            ..self
        }
    }

    /// Set the Function.
    /// This is optional. If not set, the agent will not apply any function.
    /// #Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .func(|report| {
    ///    report
    ///    })
    ///    .build()
    ///    ?;
    ///    ```
    pub fn func(self, func: fn(Report) -> Report) -> Self {
        Self {
            config: self.config.func(func),
            ..self
        }
    }

    /// Set tags. Default is empty.
    ///
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

    /// Set the http request body compression.
    ///
    /// # Example
    /// ```ignore
    /// use pyroscope::pyroscope::PyroscopeConfig;
    /// let agent = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    ///     .compression(GZIP)
    ///     .build();
    /// ```
    pub fn compression(self, compression: Compression) -> Self {
        Self {
            config: self.config.compression(compression),
            ..self
        }
    }

    pub fn tenant_id(self, tenant_id: String) -> Self {
        Self {
            config: self.config.tenant_id(tenant_id),
            ..self
        }
    }

    pub fn http_headers(self, http_headers: HashMap<String, String>) -> Self {
        Self {
            config: self.config.http_headers(http_headers),
            ..self
        }
    }

    /// Initialize the backend, timer and return a PyroscopeAgent with Ready
    /// state. While you can call this method, you should call it through the
    /// `PyroscopeAgent.build()` method.
    pub fn build(self) -> Result<PyroscopeAgent<PyroscopeAgentReady>> {
        // Set Spy Name, Spy Extension and Sample Rate from the Backend
        let config = self.config.sample_rate(self.backend.sample_rate()?);
        let config = config.spy_name(self.backend.spy_name()?);

        // Set Global Tags
        // for (key, value) in config.tags.iter() {
            // todo!("implement")
        // }

        // Initialize the Backend
        let backend_ready = self.backend.initialize()?;
        log::trace!(target: LOG_TAG, "Backend initialized");

        // Start the Timer
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
            running: Arc::new((
                #[allow(clippy::mutex_atomic)]
                Mutex::new(false),
                Condvar::new(),
            )),
            _state: PhantomData,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Compression {
    GZIP,
}

impl FromStr for Compression {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<Compression, Self::Err> {
        match input {
            "gzip" => Ok(GZIP),
            _ => Err(()),
        }
    }
}

/// This trait is used to encode the state of the agent.
pub trait PyroscopeAgentState {}

/// Marker struct for an Uninitialized state.
#[derive(Debug)]
pub struct PyroscopeAgentBare;

/// Marker struct for a Ready state.
#[derive(Debug)]
pub struct PyroscopeAgentReady;

/// Marker struct for a Running state.
#[derive(Debug)]
pub struct PyroscopeAgentRunning;

impl PyroscopeAgentState for PyroscopeAgentBare {}

impl PyroscopeAgentState for PyroscopeAgentReady {}

impl PyroscopeAgentState for PyroscopeAgentRunning {}

/// PyroscopeAgent is the main object of the library. It is used to start and stop the profiler, schedule the timer, and send the profiler data to the server.
pub struct PyroscopeAgent<S: PyroscopeAgentState> {
    /// Instance of the Timer
    timer: Timer,
    /// Instance of the SessionManager
    session_manager: SessionManager,
    /// Channel sender for the timer thread
    tx: Option<Sender<TimerSignal>>,
    /// Handle to the thread that runs the Pyroscope Agent
    handle: Option<JoinHandle<Result<()>>>,
    /// A structure to signal thread termination
    running: Arc<(Mutex<bool>, Condvar)>,
    /// Profiler backend
    pub backend: BackendImpl<BackendReady>,
    /// Configuration Object
    pub config: PyroscopeConfig,
    /// PyroscopeAgent State
    _state: PhantomData<S>,
}

impl<S: PyroscopeAgentState> PyroscopeAgent<S> {
    /// Transition the PyroscopeAgent to a new state.
    fn transition<D: PyroscopeAgentState>(self) -> PyroscopeAgent<D> {
        PyroscopeAgent {
            timer: self.timer,
            session_manager: self.session_manager,
            tx: self.tx,
            handle: self.handle,
            running: self.running,
            backend: self.backend,
            config: self.config,
            _state: PhantomData,
        }
    }
}

impl<S: PyroscopeAgentState> PyroscopeAgent<S> {
    /// Properly shutdown the agent.
    pub fn shutdown(mut self) {
        log::debug!(target: LOG_TAG, "PyroscopeAgent::drop()");

        // Shutdown Backend
        match self.backend.shutdown() {
            Ok(_) => log::debug!(target: LOG_TAG, "Backend shutdown"),
            Err(e) => log::error!(target: LOG_TAG, "Backend shutdown error: {}", e),
        }

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

        log::debug!(target: LOG_TAG, "Agent Shutdown");
    }
}

impl PyroscopeAgent<PyroscopeAgentReady> {
    /// Start profiling and sending data. The agent will keep running until stopped. The agent will send data to the server every 10s seconds.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// let agent_running = agent.start()?;
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

        // Create a channel to listen for timer signals
        let (tx, rx) = mpsc::channel();
        self.timer.attach_listener(tx.clone())?;
        self.tx = Some(tx);

        let config = self.config.clone();

        // Clone SessionManager Sender
        let stx = self.session_manager.tx.clone();

        self.handle = Some(std::thread::spawn(move || {
            log::trace!(target: LOG_TAG, "Main Thread started");

            while let Ok(signal) = rx.recv() {
                match signal {
                    TimerSignal::NextSnapshot(until) => {
                        log::trace!(target: LOG_TAG, "Sending session {}", until);

                        // Generate report from backend
                        let report = backend
                            .lock()?
                            .as_mut()
                            .ok_or_else(|| {
                                PyroscopeError::AdHoc(
                                    "PyroscopeAgent - Failed to unwrap backend".to_string(),
                                )
                            })?
                            .report()?;

                        // Send new Session to SessionManager
                        stx.send(SessionSignal::Session(Session::new(
                            until,
                            config.clone(),
                            report,
                        )?))?
                    }
                    TimerSignal::Terminate => {
                        log::trace!(target: LOG_TAG, "Session Killed");

                        // Notify the Stop function
                        let (lock, cvar) = &*pair;
                        let mut running = lock.lock()?;
                        *running = false;
                        cvar.notify_one();

                        // Kill the internal thread
                        return Ok(());
                    }
                }
            }
            Ok(())
        }));

        Ok(self.transition())
    }
}

impl PyroscopeAgent<PyroscopeAgentRunning> {
    /// Stop the agent. The agent will stop profiling and send a last report to the server.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// let agent_running = agent.start()?;
    /// // Expensive operation
    /// let agent_ready = agent_running.stop();
    /// ```
    pub fn stop(mut self) -> Result<PyroscopeAgent<PyroscopeAgentReady>> {
        log::debug!(target: LOG_TAG, "Stopping");
        // get tx and send termination signal
        if let Some(sender) = self.tx.take() {
            // Send last session
            sender.send(TimerSignal::NextSnapshot(get_time_range(0)?.until))?;
            // Terminate PyroscopeAgent internal thread
            sender.send(TimerSignal::Terminate)?;
        } else {
            log::error!("PyroscopeAgent - Missing sender")
        }

        // Wait for the Thread to finish
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        let _guard = cvar.wait_while(lock.lock()?, |running| *running)?;

        Ok(self.transition())
    }

    /// Return a tuple of functions to add and remove tags to the agent across
    /// thread boundaries. This function can be called multiple times.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// let agent_running = agent.start()?;
    /// let (add_tag, remove_tag) = agent_running.tag_wrapper();
    /// ```
    ///
    /// The functions can be later called from any thread.
    ///
    /// # Example
    /// ```ignore
    /// add_tag("key".to_string(), "value".to_string());
    /// // some computation
    /// remove_tag("key".to_string(), "value".to_string());
    /// ```
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
                // https://github.com/tikv/pprof-rs/blob/01cff82dbe6fe110a707bf2b38d8ebb1d14a18f8/src/profiler.rs#L405
                let thread_id = crate::utils::ThreadId::pthread_self();
                let rule = ThreadTag::new(thread_id, Tag::new(key, value));
                let backend = backend_add.lock()?;
                backend
                    .as_ref()
                    .ok_or_else(|| {
                        PyroscopeError::AdHoc(
                            "PyroscopeAgent - Failed to unwrap backend".to_string(),
                        )
                    })?
                    .add_tag(rule)?;

                Ok(())
            },
            move |key, value| {
                // https://github.com/tikv/pprof-rs/blob/01cff82dbe6fe110a707bf2b38d8ebb1d14a18f8/src/profiler.rs#L405
                let thread_id = crate::utils::ThreadId::pthread_self();
                let rule = ThreadTag::new(thread_id, Tag::new(key, value));
                let backend = backend_remove.lock()?;
                backend
                    .as_ref()
                    .ok_or_else(|| {
                        PyroscopeError::AdHoc(
                            "PyroscopeAgent - Failed to unwrap backend".to_string(),
                        )
                    })?
                    .remove_tag(rule)?;

                Ok(())
            },
        )
    }


    /// Add a thread Tag rule to the backend Ruleset. For tagging, it's
    /// recommended to use the `tag_wrapper` function.
    pub fn add_thread_tag(&self, thread_id: crate::utils::ThreadId, tag: Tag) -> Result<()> {
        let rule = ThreadTag::new(thread_id, tag);
        self.backend.add_tag(rule)?;

        Ok(())
    }

    /// Remove a thread Tag rule from the backend Ruleset. For tagging, it's
    /// recommended to use the `tag_wrapper` function.
    pub fn remove_thread_tag(&self, thread_id: crate::utils::ThreadId, tag: Tag) -> Result<()> {
        let rule = ThreadTag::new(thread_id, tag);
        self.backend.remove_tag(rule)?;

        Ok(())
    }
}

pub fn parse_http_headers_json(http_headers_json: String) -> Result<HashMap<String, String>> {
    let mut http_headers = HashMap::new();
    let parsed: serde_json::Value = serde_json::from_str(&http_headers_json)?;
    let parsed = parsed.as_object().ok_or_else(||
        PyroscopeError::AdHoc(format!("expected object, got {}", parsed))
    )?;
    for (k, v) in parsed {
        if let Some(value) = v.as_str() {
            http_headers.insert(k.to_string(), value.to_string());
        } else {
            return Err(PyroscopeError::AdHoc(format!(
                "invalid http header value, not a string: {}",
                v
            )));
        }
    }
    Ok(http_headers)
}

pub fn parse_vec_string_json(s: String) -> Result<Vec<String>> {
    let parsed: serde_json::Value = serde_json::from_str(&s)?;
    let parsed = parsed.as_array().ok_or_else(||
        PyroscopeError::AdHoc(format!("expected array, got {}", parsed))
    )?;
    let mut res = Vec::with_capacity(parsed.len());
    for v in parsed {
        if let Some(s) = v.as_str() {
            res.push(s.to_string());
        } else {
            return Err(PyroscopeError::AdHoc(format!(
                "invalid element value, not a string: {}",
                v
            )));
        }
    }
    Ok(res)
}
