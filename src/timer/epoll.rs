// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::utils::{epoll_create1, epoll_ctl, epoll_wait, read, timerfd_create, timerfd_settime};
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

        let timer_fd = Timer::set_timerfd()?;
        let epoll_fd = Timer::create_epollfd(timer_fd)?;

        let handle = Some(thread::spawn(move || {
            loop {
                // Exit thread if there are no listeners
                if txs.lock()?.len() == 0 {
                    // TODO: should close file descriptors?
                    return Ok(());
                }

                // Fire @ 10th sec
                Timer::epoll_wait(timer_fd, epoll_fd)?;

                // Get current time
                let current = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();

                // Iterate through Senders
                txs.lock()?.iter().for_each(|tx| {
                    // Send event to attached Sender
                    tx.send(current).unwrap();
                });
            }
        }));

        Ok(Self { handle, ..self })
    }

    /// create and set a timer file descriptor
    fn set_timerfd() -> Result<libc::c_int> {
        // Set the timer to use the system time.
        let clockid: libc::clockid_t = libc::CLOCK_REALTIME;
        // Non-blocking file descriptor
        let clock_flags: libc::c_int = libc::TFD_NONBLOCK;

        // Create timer fd
        let tfd = timerfd_create(clockid, clock_flags)?;

        // Get the next event time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let rem = 10u64.checked_sub(now.checked_rem(10).unwrap()).unwrap();
        let first_fire = now + rem;

        // new_value sets the Timer
        let mut new_value = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 10,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: first_fire as i64,
                tv_nsec: 0,
            },
        };

        // Empty itimerspec object
        let mut old_value = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
        };

        let set_flags = libc::TFD_TIMER_ABSTIME;

        // Set the timer
        timerfd_settime(tfd, set_flags, &mut new_value, &mut old_value)?;

        // Return file descriptor
        Ok(tfd)
    }

    /// Create a new epoll file descriptor and add the timer to its interests
    fn create_epollfd(timer_fd: libc::c_int) -> Result<libc::c_int> {
        // create a new epoll fd
        let epoll_fd = epoll_create1(0)?;

        // event to pull
        let mut event = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: 1,
        };

        let epoll_flags = libc::EPOLL_CTL_ADD;

        // add event to the epoll
        epoll_ctl(epoll_fd, epoll_flags, timer_fd, &mut event)?;

        // return epoll fd
        Ok(epoll_fd)
    }

    fn epoll_wait(timer_fd: libc::c_int, epoll_fd: libc::c_int) -> Result<()> {
        // vector to store events
        let mut events = Vec::with_capacity(1);

        // wait for the timer to fire an event. This is function will block.
        epoll_wait(epoll_fd, events.as_mut_ptr(), 1, -1)?;

        // read the value from the timerfd. This is required to re-arm the timer.
        let mut buffer: u64 = 0;
        let bufptr: *mut _ = &mut buffer;
        read(timer_fd, bufptr as *mut libc::c_void, 8)?;

        Ok(())
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
}
