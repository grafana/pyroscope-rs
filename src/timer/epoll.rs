// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::Result;

use std::sync::{mpsc::Sender, Arc, Mutex};
use std::{thread, thread::JoinHandle};

#[derive(Debug, Default)]
pub struct Timer {
    /// A vector to store listeners (mpsc::Sender)
    txs: Arc<Mutex<Vec<Sender<u64>>>>,

    /// Thread handle
    pub handle: Option<JoinHandle<Result<()>>>,
}

impl Timer {
    pub fn initialize(self) -> Self {
        self
    }

    pub fn attach_listener(&mut self, tx: Sender<u64>) -> Result<()> {
        Ok(())
    }

    pub fn drop_listeners(&mut self) -> Result<()> {
        Ok(())
    }
}
