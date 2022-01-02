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
//use crate::scheduler::{Event, PyroscopeScheduler};
use crate::timer::Timer;

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
            sample_rate: 100i32,
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

    pub fn frequency(self, frequency: i32) -> Self {
        Self {
            sample_rate: frequency,
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

    pub fn build(self) -> Result<PyroscopeAgent> {
        // Initiliaze the backend
        let backend = Arc::clone(&self.backend);
        backend.lock()?.initialize(self.sample_rate)?;

        // Create Tags Arc<Mutex<>>
        let tags = Arc::new(Mutex::new(self.tags));

        // Initialize Scheduler
        //let scheduler = PyroscopeScheduler::initialize(
        //self.url.to_owned(),
        //self.application_name.to_owned(),
        //Arc::clone(&tags),
        //self.sample_rate.to_owned(),
        //Arc::clone(&backend),
        //);

        // Start Timer
        let mut timer = Timer::default().initialize();

        // Return PyroscopeAgent
        Ok(PyroscopeAgent {
            backend: self.backend,
            //scheduler,
            url: self.url,
            application_name: self.application_name,
            tags: Arc::clone(&tags),
            sample_rate: self.sample_rate,
            timer,
            tx: None,
            handle: None,
            running: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }
}

pub struct PyroscopeAgent {
    pub backend: Arc<Mutex<dyn Backend>>,
    //scheduler: PyroscopeScheduler,
    timer: Timer,
    tx: Option<Sender<u64>>,
    handle: Option<JoinHandle<()>>,
    running: Arc<(Mutex<bool>, Condvar)>,

    url: String,
    application_name: String,
    tags: Arc<Mutex<HashMap<String, String>>>,
    sample_rate: i32,
}

impl PyroscopeAgent {
    pub fn builder<S: AsRef<str>>(url: S, application_name: S) -> PyroscopeAgentBuilder {
        // Build PyroscopeAgent
        PyroscopeAgentBuilder::new(url, application_name)
    }

    pub fn terminate(self) -> Result<()> {
        // Send Termination Signal
        //self.scheduler.tx.send(Event::Terminate).unwrap();

        // Drop Scheduler Transmitter
        //drop(self.scheduler.tx);

        // Wait for the main Thread to drop
        //self.scheduler.thread_handle.join();

        Ok(())
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

        //self.scheduler.tx.send(Event::Start).unwrap();
        let (tx, rx): (Sender<u64>, Receiver<u64>) = channel();
        self.timer.attach_listener(tx.clone()).unwrap();
        self.tx = Some(tx.clone());

        self.handle = Some(std::thread::spawn(move || {
            while let Ok(time) = rx.recv() {
                println!("Timer Notification: {}", time);
                let a = backend.lock().unwrap().report().unwrap();
                println!("{:?}", a);
                if time == 0 {
                    println!("Thread Terminated");

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
        self.tx.take().unwrap().send(0);

        // Wait for the Thread to finish
        let pair = Arc::clone(&self.running);
        let (lock, cvar) = &*pair;
        cvar.wait_while(lock.lock().unwrap(), |running| {*running}).unwrap();

        // Create a clone of Backend
        let backend = Arc::clone(&self.backend);
        // Call stop()
        backend.lock()?.stop()?;

        // Send Stop Event to Scheduler
        //self.scheduler.tx.send(Event::Stop).unwrap();

        Ok(())
    }

    pub fn add_tags(&mut self, tags: &[(&str, &str)]) -> Result<()> {
        // Stop Agent
        self.stop()?;

        // Restart Agent
        self.start()?;

        // Convert &[(&str, &str)] to HashMap(String, String)
        let tags_hashmap: HashMap<String, String> = tags
            .to_owned()
            .iter()
            .cloned()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        // Create a clone of tags
        let tags_arc = Arc::clone(&self.tags);
        // Extend tags with tags_hashmap
        tags_arc.lock()?.extend(tags_hashmap);

        Ok(())
    }

    pub fn remove_tags(&mut self, tags: &[&str]) -> Result<()> {
        // Stop Agent
        self.stop()?;

        // Create a clone of tags
        let tags_arc = Arc::clone(&self.tags);
        // Get a lock of tags
        let mut tags_lock = tags_arc.lock()?;

        // Iterate through every tag
        tags.iter().for_each(|key| {
            // Remove tag
            tags_lock.remove(key.to_owned());
        });

        // Drop lock
        drop(tags_lock);

        // Restart Agent
        self.start()?;

        Ok(())
    }
}
