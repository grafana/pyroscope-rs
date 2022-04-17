use crate::{backend::Rule, error::Result};
use std::{
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Mutex},
};

use super::Report;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl Tag {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

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

    fn add_rule(&self, ruleset: Rule) -> Result<()>;
    fn remove_rule(&self, ruleset: Rule) -> Result<()>;
}

#[derive(Debug)]
pub struct BackendUninitialized;
#[derive(Debug)]
pub struct BackendReady;

pub trait BackendState {}
impl BackendState for BackendUninitialized {}
impl BackendState for BackendReady {}

#[derive(Debug)]
pub struct BackendImpl<S: BackendState + ?Sized> {
    pub backend: Arc<Mutex<dyn Backend>>,
    _state: std::marker::PhantomData<S>,
}

impl<S: BackendState> BackendImpl<S> {
    pub fn spy_name(&self) -> Result<String> {
        self.backend.lock()?.spy_name()
    }
    pub fn sample_rate(&self) -> Result<u32> {
        self.backend.lock()?.sample_rate()
    }
    pub fn add_rule(&self, rule: Rule) -> Result<()> {
        self.backend.lock()?.add_rule(rule)
    }
    pub fn remove_rule(&self, rule: Rule) -> Result<()> {
        self.backend.lock()?.remove_rule(rule)
    }
}

impl BackendImpl<BackendUninitialized> {
    pub fn new(backend: Arc<Mutex<dyn Backend>>) -> Self {
        Self {
            backend,
            _state: std::marker::PhantomData,
        }
    }

    pub fn initialize(self) -> Result<BackendImpl<BackendReady>> {
        let backend = self.backend.clone();
        backend.lock()?.initialize()?;

        Ok(BackendImpl {
            backend,
            _state: std::marker::PhantomData,
        })
    }
}
impl BackendImpl<BackendReady> {
    pub fn shutdown(self) -> Result<()> {
        //let backend = self.backend.clone();
        //backend.lock()?.shutdown()?;
        Ok(())
    }
    pub fn report(&mut self) -> Result<Vec<Report>> {
        self.backend.lock()?.report()
    }
}

pub fn merge_tags_with_app_name(application_name: String, tags: Vec<Tag>) -> Result<String> {
    let mut merged_tags = String::new();

    if tags.is_empty() {
        return Ok(application_name);
    }

    for tag in tags {
        merged_tags.push_str(&format!("{}={},", tag.key, tag.value));
    }

    Ok(format!("{}{{{}}}", application_name, merged_tags))
}
