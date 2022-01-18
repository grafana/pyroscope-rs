// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::utils::check_err;
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
    pub fn initialize(self) -> Result<Self> {
        let txs = Arc::clone(&self.txs);

        let kqueue = kqueue()?;

        let handle = Some(thread::spawn(move || {
            // Wait for initial expiration
            let initial_event = Timer::register_initial_expiration(kqueue)?;
            Timer::wait_event(kqueue, [initial_event].as_mut_ptr())?;

            // Register loop event
            let loop_event = Timer::register_loop_expiration(kqueue)?;

            // Loop 10s
            loop {
                // Exit thread if there are no listeners
                if txs.lock()?.len() == 0 {
                    // TODO: should close file descriptors?
                    return Ok(());
                }

                // Get current time
                let current = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();

                // Iterate through Senders
                txs.lock()?.iter().for_each(|tx| {
                    // Send event to attached Sender
                    tx.send(current).unwrap();
                });

                // Wait 10s
                Timer::wait_event(kqueue, [loop_event].as_mut_ptr())?;
            }
        }));

        Ok(Self { handle, ..self })
    }

    /// Attach an mpsc::Sender to Timer
    ///
    /// Timer will dispatch an event with the timestamp of the current instant,
    /// every 10th second to all attached senders
    pub fn attach_listener(&mut self, tx: Sender<u64>) -> Result<()> {
        // Push Sender to a Vector of Sender(s)
        let txs = Arc::clone(&self.txs);
        txs.lock()?.push(tx);

        Ok(())
    }

    /// Clear the listeners (txs) from Timer. This will shutdown the Timer thread
    pub fn drop_listeners(&mut self) -> Result<()> {
        let txs = Arc::clone(&self.txs);
        txs.lock()?.clear();

        Ok(())
    }

    fn wait_event(kqueue: i32, events: *mut libc::kevent) -> Result<()> {
        kevent(kqueue, [].as_mut_ptr(), 0, events, 1, std::ptr::null())?;
        Ok(())
    }
    fn register_initial_expiration(kqueue: i32) -> Result<libc::kevent> {
        // Get the next event time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let rem = 10u64.checked_sub(now.checked_rem(10).unwrap()).unwrap();
        let first_fire = now + rem;

        let initial_event = libc::kevent {
            ident: 1,
            filter: libc::EVFILT_TIMER,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
            fflags: libc::NOTE_ABSOLUTE | libc::NOTE_SECONDS,
            data: first_fire as isize,
            udata: 0 as *mut libc::c_void,
        };

        // add first event
        kevent(
            kqueue,
            [initial_event].as_ptr() as *const libc::kevent,
            1,
            [].as_mut_ptr(),
            0,
            std::ptr::null(),
        )?;

        Ok(initial_event)
    }
    fn register_loop_expiration(kqueue: i32) -> Result<libc::kevent> {
        let loop_event = libc::kevent {
            ident: 1,
            filter: libc::EVFILT_TIMER,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 10000,
            udata: 0 as *mut libc::c_void,
        };

        // add loop event
        let ke = kevent(
            kqueue,
            [loop_event].as_ptr() as *const libc::kevent,
            1,
            [].as_mut_ptr(),
            0,
            std::ptr::null(),
        )?;

        Ok(loop_event)
    }
}

fn kqueue() -> Result<i32> {
    check_err(unsafe { libc::kqueue() }).map(|kq| kq as i32)
}

fn kevent(
    kqueue: i32, change: *const libc::kevent, c_count: libc::c_int, events: *mut libc::kevent,
    e_count: libc::c_int, timeout: *const libc::timespec,
) -> Result<()> {
    check_err(unsafe { libc::kevent(kqueue, change, c_count, events, e_count, timeout) })?;
    Ok(())
}
