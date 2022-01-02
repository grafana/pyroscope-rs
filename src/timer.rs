// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::Result;

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration};

///
/// Custom Timer that sends a notification every 10th second
///
#[derive(Debug, Default)]
pub struct Timer {
    txs: Arc<Mutex<Vec<Sender<u64>>>>,
}

impl Timer {
    pub fn initialize(self) -> Self {
        let txs = Arc::clone(&self.txs);
        thread::spawn(move || {
            // Get the current time
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Calculate number of seconds until 10th second
            let rem = 10u64.checked_sub(now.checked_rem(10).unwrap()).unwrap();

            // Sleep for rem seconds
            thread::sleep(Duration::from_secs(rem));

            loop {
                let current = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                txs.lock().unwrap().iter().for_each(|tx| {
                    tx.send(current).unwrap();
                });
                thread::sleep(Duration::from_millis(10000));
            }
        });
        self
    }

    pub fn attach_listener(&mut self, tx: Sender<u64>) -> Result<()> {
        let txs = Arc::clone(&self.txs);
        txs.lock().unwrap().push(tx);
        Ok(())
    }
}
