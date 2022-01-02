// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::backends::Backend;
use crate::PyroscopeAgent;
use crate::timer::Timer;
use crate::Result;

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};

use std::thread::{spawn, JoinHandle};
use std::time;
use std::time::Duration;

#[derive(Debug)]
pub enum Event {
    Start,
    Stop,
    Report,
    Terminate,
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
        //let mut timer = Timer::default().initialize();

        let backend_arc = Arc::clone(&backend);
        let tags_arc = Arc::clone(&tags);
        let tx_thread = tx.clone();

        // Execute Thread
        let thread_handle = spawn(move || {
            loop {
                match rx.recv() {
                    Ok(Event::Start) => {
                        println!(
                            "Start Timer: {}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                        );
                    }

                    Ok(Event::Stop) => {
                        println!(
                            "Stop Timer : {}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                        );

                        // Send Last Report
                        tx_thread.send(Event::Report).unwrap();
                    }

                    // Send Report to Pyroscope API
                    Ok(Event::Report) => {
                        println!("Send Report to Pyroscope");
                        println!("{:?}", tags_arc);
                    }

                    // Gracefully terminate the Scheduler
                    Ok(Event::Terminate) => {
                        println!("Terminate called");

                        // Drop Thread Transmitter
                        drop(tx_thread);
                        // Drop Timer
//                        drop(timer);
                        // Clear the Receiver Backlog
                        for x in rx.iter() {
                            println!("{:?}", x);
                        println!("{:?}", tags_arc);
                        }

                        // Exit the Thread
                        return;
                    }
                    _ => {}
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
