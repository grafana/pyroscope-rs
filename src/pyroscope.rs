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
    backend::{void_backend, BackendReady, BackendUninitialized, Report, Rule, Tag, VoidConfig},
    error::Result,
    session::{Session, SessionManager, SessionSignal},
    timer::{Timer, TimerSignal},
    utils::get_time_range,
    PyroscopeError,
};

use json;

use crate::backend::BackendImpl;
use crate::pyroscope::Compression::GZIP;
use crate::pyroscope::ReportEncoding::{PPROF};

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
    /// Function to apply
    pub func: Option<fn(Report) -> Report>,
    /// Pyroscope http request body compression
    pub compression: Option<Compression>,
    pub report_encoding: ReportEncoding,
    pub scope_org_id: Option<String>,
    pub http_headers: HashMap<String, String>,
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
            func: None,
            compression: None,
            report_encoding: ReportEncoding::FOLDED,
            scope_org_id: None,
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
            func: None,                   // No function
            compression: None,
            report_encoding: ReportEncoding::FOLDED,
            scope_org_id: None,
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

    /// Set the Authentication Token.
    pub fn auth_token(self, auth_token: String) -> Self {
        Self {
            auth_token: Some(auth_token),
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

    pub fn report_encoding(self, report_encoding: ReportEncoding) -> Self {
        Self {
            report_encoding: report_encoding,
            ..self
        }
    }

    pub fn scope_org_id(self, scope_org_id: String) -> Self {
        Self {
            scope_org_id: Some(scope_org_id),
            ..self
        }
    }

    pub fn http_headers(self, http_headers: HashMap<String, String>) -> Self {
        Self {
            http_headers: http_headers,
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

impl Default for PyroscopeAgentBuilder {
    fn default() -> Self {
        Self {
            backend: void_backend(VoidConfig::default()),
            config: PyroscopeConfig::default(),
        }
    }
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

    /// Set the agent backend. Default is void-backend.
    ///
    /// # Example
    /// ```ignore
    /// let builder = PyroscopeAgentBuilder::new("http://localhost:8080", "my-app")
    /// .backend(PprofConfig::new().sample_rate(100))
    /// .build()?;
    /// ```
    pub fn backend(self, backend: BackendImpl<BackendUninitialized>) -> Self {
        Self { backend, ..self }
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

    pub fn report_encoding(self, report_encoding: ReportEncoding) -> Self {
        Self {
            config: self.config.report_encoding(report_encoding),
            ..self
        }
    }

    pub fn scope_org_id(self, scope_org_id: String) -> Self {
        Self {
            config: self.config.scope_org_id(scope_org_id),
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

        // use match instead of if let to avoid the need to borrow
        let config = match self.backend.spy_extension()? {
            Some(extension) => {
                if config.report_encoding == PPROF {
                    config
                } else {
                    let application_name = config.application_name.clone();
                    config.application_name(format!("{}.{}", application_name, extension))
                }
            }
            None => config,
        };

        // Set Global Tags
        for (key, value) in config.tags.iter() {
            self.backend
                .add_rule(crate::backend::Rule::GlobalTag(Tag::new(
                    key.to_owned(),
                    value.to_owned(),
                )))?;
        }

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
    GZIP
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

#[derive(Clone, PartialEq, Debug)]
pub enum ReportEncoding {
    FOLDED,
    PPROF,
}

impl FromStr for ReportEncoding {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<ReportEncoding, Self::Err> {
        match input {
            "collapsed" => Ok(ReportEncoding::FOLDED),
            "folded" => Ok(ReportEncoding::FOLDED),
            "pprof" => Ok(ReportEncoding::PPROF),
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
#[derive(Debug)]
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

impl PyroscopeAgent<PyroscopeAgentBare> {
    /// Short-hand for PyroscopeAgentBuilder::new(url, application_name). This is a convenience method.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::builder("http://localhost:8080", "my-app").build()?;
    /// ```
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::new(url, application_name)
    }

    /// Short-hand for PyroscopeAgentBuilder::default(). This is a convenience method.
    /// Default URL is "http://localhost:4040". Default application name is randomly generated.
    ///
    /// # Example
    /// ```ignore
    /// let agent = PyroscopeAgent::default_builder().build()?;
    /// ```
    pub fn default_builder() -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::default()
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
                let thread_id = crate::utils::pthread_self()?;
                let rule = Rule::ThreadTag(thread_id, Tag::new(key, value));
                let backend = backend_add.lock()?;
                backend
                    .as_ref()
                    .ok_or_else(|| {
                        PyroscopeError::AdHoc(
                            "PyroscopeAgent - Failed to unwrap backend".to_string(),
                        )
                    })?
                    .add_rule(rule)?;

                Ok(())
            },
            move |key, value| {
                let thread_id = crate::utils::pthread_self()?;
                let rule = Rule::ThreadTag(thread_id, Tag::new(key, value));
                let backend = backend_remove.lock()?;
                backend
                    .as_ref()
                    .ok_or_else(|| {
                        PyroscopeError::AdHoc(
                            "PyroscopeAgent - Failed to unwrap backend".to_string(),
                        )
                    })?
                    .remove_rule(rule)?;

                Ok(())
            },
        )
    }

    /// Add a global Tag rule to the backend Ruleset. For tagging, it's
    /// recommended to use the `tag_wrapper` function.
    pub fn add_global_tag(&self, tag: Tag) -> Result<()> {
        let rule = Rule::GlobalTag(tag);
        self.backend.add_rule(rule)?;

        Ok(())
    }

    /// Remove a global Tag rule from the backend Ruleset. For tagging, it's
    /// recommended to use the `tag_wrapper` function.
    pub fn remove_global_tag(&self, tag: Tag) -> Result<()> {
        let rule = Rule::GlobalTag(tag);
        self.backend.remove_rule(rule)?;

        Ok(())
    }

    /// Add a thread Tag rule to the backend Ruleset. For tagging, it's
    /// recommended to use the `tag_wrapper` function.
    pub fn add_thread_tag(&self, thread_id: u64, tag: Tag) -> Result<()> {
        let rule = Rule::ThreadTag(thread_id, tag);
        self.backend.add_rule(rule)?;

        Ok(())
    }

    /// Remove a thread Tag rule from the backend Ruleset. For tagging, it's
    /// recommended to use the `tag_wrapper` function.
    pub fn remove_thread_tag(&self, thread_id: u64, tag: Tag) -> Result<()> {
        let rule = Rule::ThreadTag(thread_id, tag);
        self.backend.remove_rule(rule)?;

        Ok(())
    }
}

pub fn parse_http_headers_json(http_headers_json: String) -> Result<HashMap<String, String>> {
    let mut http_headers = HashMap::new();
    let parsed = json::parse(&http_headers_json)?;
    if !parsed.is_object() {
        return Err(PyroscopeError::AdHoc(format!("expected object, got {}", parsed)));
    }
    for (k, v) in parsed.entries() {
        if v.is_string() {
            http_headers.insert(k.to_string(), v.to_string());
        } else {
            return Err(PyroscopeError::AdHoc(format!("invalid http header value, not a string: {}", v.to_string())));
        }
    };
    return Ok(http_headers);
}