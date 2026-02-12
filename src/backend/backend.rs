#![allow(clippy::module_inception)]

use crate::error::{PyroscopeError, Result};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use super::{Report, ThreadTag};

/// Backend Config
#[derive(Debug, Copy, Clone, Default)]
pub struct BackendConfig {
    pub report_thread_id: bool,
    pub report_thread_name: bool,
    pub report_pid: bool,
}

/// Backend Trait
pub trait Backend: Send {
    /// Backend Spy Name
    fn spy_name(&self) -> Result<String>;
    /// Backend name extension
    fn spy_extension(&self) -> Result<Option<String>>;
    /// Get backend configuration.
    fn sample_rate(&self) -> Result<u32>;
    /// Initialize the backend.
    fn initialize(&mut self) -> Result<()>;
    /// Drop the backend.
    fn shutdown(self: Box<Self>) -> Result<()>;
    /// Generate profiling report
    fn report(&mut self) -> Result<Vec<Report>>;
    fn add_tag(&self, tag: ThreadTag) -> Result<()>;
    fn remove_tag(&self, tag: ThreadTag) -> Result<()>;
}

/// Marker struct for Empty BackendImpl
#[derive(Debug)]
pub struct BackendBare;

/// Marker struct for Uninitialized Backend
#[derive(Debug)]
pub struct BackendUninitialized;

/// Marker struct for Initialized Backend
#[derive(Debug)]
pub struct BackendReady;

/// Backend State Trait
pub trait BackendState {}
impl BackendState for BackendBare {}
impl BackendState for BackendUninitialized {}
impl BackendState for BackendReady {}

/// Backend Accessibility Trait
pub trait BackendAccessible: BackendState {}
impl BackendAccessible for BackendUninitialized {}
impl BackendAccessible for BackendReady {}

/// Precursor Backend Implementation
/// This struct is used to implement the Backend trait. It serves two purposes:
/// 1. It enforces state transitions using the Type System.
/// 2. It manages the lifetime of the backend through an Arc<Mutex<T>>.
pub struct BackendImpl<S: BackendState + ?Sized> {
    /// Backend
    pub backend: Arc<Mutex<Option<Box<dyn Backend>>>>,

    /// Backend State
    _state: std::marker::PhantomData<S>,
}

impl BackendImpl<BackendBare> {
    /// Create a new BackendImpl instance
    pub fn new(backend_box: Box<dyn Backend>) -> BackendImpl<BackendUninitialized> {
        BackendImpl {
            backend: Arc::new(Mutex::new(Some(backend_box))),
            _state: std::marker::PhantomData,
        }
    }
}

impl BackendImpl<BackendUninitialized> {
    /// Initialize the backend
    pub fn initialize(self) -> Result<BackendImpl<BackendReady>> {
        let backend = self.backend.clone();

        // Initialize the backend
        backend
            .lock()?
            .as_mut()
            .ok_or(PyroscopeError::BackendImpl)?
            .initialize()?;

        // Transition to BackendReady
        Ok(BackendImpl {
            backend,
            _state: std::marker::PhantomData,
        })
    }
}

impl<S: BackendAccessible> BackendImpl<S> {
    /// Return the backend name
    pub fn spy_name(&self) -> Result<String> {
        self.backend
            .lock()?
            .as_ref()
            .ok_or(PyroscopeError::BackendImpl)?
            .spy_name()
    }

    /// Return the backend extension
    pub fn spy_extension(&self) -> Result<Option<String>> {
        self.backend
            .lock()?
            .as_ref()
            .ok_or(PyroscopeError::BackendImpl)?
            .spy_extension()
    }

    /// Return the backend sample rate
    pub fn sample_rate(&self) -> Result<u32> {
        self.backend
            .lock()?
            .as_ref()
            .ok_or(PyroscopeError::BackendImpl)?
            .sample_rate()
    }

    pub fn add_tag(&self, tag: ThreadTag) -> Result<()> {
        self.backend
            .lock()?
            .as_ref()
            .ok_or(PyroscopeError::BackendImpl)?
            .add_tag(tag)
    }

    pub fn remove_tag(&self, rule: ThreadTag) -> Result<()> {
        self.backend
            .lock()?
            .as_ref()
            .ok_or(PyroscopeError::BackendImpl)?
            .remove_tag(rule)
    }
}

impl BackendImpl<BackendReady> {
    /// Shutdown the backend and destroy BackendImpl
    pub fn shutdown(self) -> Result<()> {
        self.backend
            .lock()?
            .take()
            .ok_or(PyroscopeError::BackendImpl)?
            .shutdown()?;

        Ok(())
    }
    /// Generate profiling report
    pub fn report(&mut self) -> Result<Vec<Report>> {
        self.backend
            .lock()?
            .as_mut()
            .ok_or(PyroscopeError::BackendImpl)?
            .report()
    }
}
