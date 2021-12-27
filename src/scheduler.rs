// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::backends::Backend;
use crate::PyroscopeAgent;
use crate::Result;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};

use std::thread::{spawn, JoinHandle};

pub enum Event {
    Start,
    Stop,
    Report,
}

type A = (
    Event,
    i32,
    String,
    String,
    Arc<Mutex<HashMap<String, String>>>,
    Arc<Mutex<dyn Backend>>,
);

pub struct PyroscopeScheduler {
    pub thread_handle: JoinHandle<()>,
    pub tx: Sender<A>,
}

impl PyroscopeScheduler {
    pub fn initialize() -> Self {
        // Create streaming channel
        let (tx, rx): (Sender<A>, Receiver<A>) = std::sync::mpsc::channel();

        // Execute Thread
        let thread_handle = spawn(move || loop {
            match rx.recv() {
                Ok((Event::Start, sample_rate, url, application_name, tags, backend)) => {
                    println!("Profiling Started");
                    let report = backend.lock().unwrap().report().unwrap();
                    println!("{}", std::str::from_utf8(&report).unwrap()); 
                }
                Ok((Event::Stop, sample_rate, url, application_name, tags, backend)) => {
                    println!("Profiling Stopped");
                }
                Ok((Event::Report, sample_rate, url, application_name, tags, backend)) => {
                    println!("Send Report to Pyroscope");
                }

                _ => {}
            }
        });

        Self { thread_handle, tx }
    }
}
