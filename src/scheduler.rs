// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::backends::Backend;
use crate::PyroscopeAgent;
use crate::Result;

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use std::thread::{spawn, JoinHandle};

pub enum Event {
    Start,
    Stop,
    Report,
}

pub struct PyroscopeScheduler {
    pub thread_handle: JoinHandle<()>,
    pub tx: Sender<Event>,

    url: String,
    application_name: String,
    tags: Arc<Mutex<HashMap<String, String>>>,
    sample_rate: i32,
    backend: Arc<Mutex<dyn Backend>>,
}

impl PyroscopeScheduler {
    pub fn initialize(
        url: String,
        application_name: String,
        tags: Arc<Mutex<HashMap<String, String>>>,
        sample_rate: i32,
        backend: Arc<Mutex<dyn Backend>>,
    ) -> Self {
        // Create streaming channel
        let (tx, rx): (Sender<Event>, Receiver<Event>) = std::sync::mpsc::channel();

        let backend_arc = Arc::clone(&backend);

        // Execute Thread
        let thread_handle = spawn(move || loop {
            match rx.recv() {
                Ok(Event::Start) => {
                    println!("Profiling Started");
                }
                Ok(Event::Stop) => {
                    println!("Profiling Stopped");
                }
                Ok(Event::Report) => {
                    println!("Send Report to Pyroscope");
                }

                _ => {}
            }
        });

        Self {
            thread_handle,
            tx,
            url,
            application_name,
            tags,
            sample_rate,
            backend,
        }
    }
}
