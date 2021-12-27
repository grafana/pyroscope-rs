// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::backends::Backend;
use crate::PyroscopeAgent;
use crate::Result;

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender ,SyncSender};
use std::sync::{Arc, Mutex};

use std::thread::{spawn, JoinHandle};
use std::time;

pub enum Event {
    Start,
    Stop,
    Report,
    Terminate,
}

pub struct Timer{
   tx: Sender<Event>,
   running: Arc<Mutex<bool>>,
   handle: Option<JoinHandle<()>>
}

impl Timer {
    pub fn start(&mut self) -> Result<()> {
        let tx = self.tx.clone();
        let running = Arc::clone(&self.running);

        let thread_handle = spawn(move || {
            let mut a = running.lock().unwrap();
            *a = true;
            drop(a);
            loop {
                let mut a = running.lock().unwrap();
                if *a == false {
                    drop(a);
                    break;
                }
                drop(a);
                std::thread::sleep(time::Duration::from_millis(500));
                tx.send(Event::Report).unwrap();
            }
            return;
        });
        self.handle = Some(thread_handle);
        Ok(())
    }
    pub fn stop(&mut self) -> Result<()> {
        let running = Arc::clone(&self.running);
        let mut a = running.lock().unwrap();
        *a = false;
        drop(a);

        self.handle.take().unwrap().join();

        Ok(())
    }
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

        // Initialize timer
        let mut timer = Timer{
            tx: tx.clone(), running: Arc::new(Mutex::new(false)), handle: None,
        };

        let backend_arc = Arc::clone(&backend);
        let tx_thread = tx.clone();

        // Execute Thread
        let thread_handle = spawn(move || loop {
            match rx.recv() {
                Ok(Event::Start) => {
                    println!("Profiling Started");
                    timer.start().unwrap();
                }
                Ok(Event::Stop) => {
                    println!("Profiling Stopped");
                    timer.stop().unwrap();
                }
                Ok(Event::Report) => {
                    println!("Send Report to Pyroscope");
                }
                Ok(Event::Terminate) => {
                    println!("Terminated");
                    return;
                }
                _ => {
                }
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
