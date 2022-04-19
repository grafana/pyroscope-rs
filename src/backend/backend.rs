use crate::{backend::Rule, error::Result};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use super::Report;

/// Backend Trait
pub trait Backend: Send + Debug {
    /// Backend Spy Name
    fn spy_name(&self) -> Result<String>;
    /// Get backend configuration.
    fn sample_rate(&self) -> Result<u32>;
    /// Initialize the backend.
    fn initialize(&mut self) -> Result<()>;
    /// Drop the backend.
    fn shutdown(self) -> Result<()>;
    /// Generate profiling report
    fn report(&mut self) -> Result<Vec<Report>>;
    /// Add a report-splitting rule to the backend.
    fn add_rule(&self, ruleset: Rule) -> Result<()>;
    /// Remove a report-splitting rule from the backend.
    fn remove_rule(&self, ruleset: Rule) -> Result<()>;
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
pub trait BackendAccessible {}
impl BackendAccessible for BackendUninitialized {}
impl BackendAccessible for BackendReady {}

/// Precursor Backend Implementation
/// This struct is used to implement the Backend trait. It serves two purposes:
/// 1. It enforces state transitions using the Type System.
/// 2. It manages the lifetime of the backend through an Arc<Mutex<T>>.
#[derive(Debug)]
pub struct BackendImpl<S: BackendState + ?Sized> {
    /// Backend
    pub backend: Arc<Mutex<dyn Backend>>,

    /// Backend State
    _state: std::marker::PhantomData<S>,
}

impl BackendImpl<BackendBare> {
    /// Create a new BackendImpl instance
    pub fn new(backend: Arc<Mutex<dyn Backend>>) -> BackendImpl<BackendUninitialized> {
        BackendImpl {
            backend,
            _state: std::marker::PhantomData,
        }
    }
}

impl BackendImpl<BackendUninitialized> {
    /// Initialize the backend
    pub fn initialize(self) -> Result<BackendImpl<BackendReady>> {
        let backend = self.backend.clone();
        backend.lock()?.initialize()?;

        Ok(BackendImpl {
            backend,
            _state: std::marker::PhantomData,
        })
    }
}

impl<S: BackendState + BackendAccessible> BackendImpl<S> {
    /// Return the backend name
    pub fn spy_name(&self) -> Result<String> {
        self.backend.lock()?.spy_name()
    }

    /// Return the backend sample rate
    pub fn sample_rate(&self) -> Result<u32> {
        self.backend.lock()?.sample_rate()
    }

    /// Add a report-splitting rule to the backend
    pub fn add_rule(&self, rule: Rule) -> Result<()> {
        self.backend.lock()?.add_rule(rule)
    }

    /// Remove a report-splitting rule from the backend
    pub fn remove_rule(&self, rule: Rule) -> Result<()> {
        self.backend.lock()?.remove_rule(rule)
    }
}

impl BackendImpl<BackendReady> {
    /// Shutdown the backend and destroy BackendImpl
    pub fn shutdown(self) -> Result<()> {
        //let backend = self.backend.clone();
        //backend.lock()?.shutdown()?;

        Ok(())
    }
    /// Generate profiling report
    pub fn report(&mut self) -> Result<Vec<Report>> {
        self.backend.lock()?.report()
    }
}
