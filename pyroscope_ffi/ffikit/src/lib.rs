use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use lazy_static::lazy_static;
use pyroscope::error::{PyroscopeError, Result};
use std::{
    sync::{
        atomic::AtomicU32,
        mpsc::{self, Receiver, Sender},
        Mutex, Once,
    },
    thread::JoinHandle,
};

static ONCE: Once = Once::new();

lazy_static! {
    /// Root Sender
    /// This is the sender to the main loop. It is lazily initialized inside a Mutex.
    static ref SENDER: Mutex<Option<Sender<Signal>>> = Mutex::new(None);
}

/// Signal enum.
/// This enum is used to send signals to the main loop. It is used to add/remove global or thread
/// tags and to exit the main loop.
#[derive(Debug, PartialEq, Clone)]
pub enum Signal {
    Kill,
    AddGlobalTag(String, String),
    RemoveGlobalTag(String, String),
    AddThreadTag(pyroscope::ThreadId, String, String),
    RemoveThreadTag(pyroscope::ThreadId, String, String),
}

// todo this looks like this does pretty much nothing - receive and send, consider deliting this alltogether
pub fn initialize_ffi() -> Receiver<Signal> {
    // Create another channel to merge communication.
    let (merge_sender, merge_receiver): (Sender<Signal>, Receiver<Signal>) = mpsc::channel();

    ONCE.call_once(|| {
        // Create a channel to communicate with the FFI.
        let (sender, receiver): (Sender<Signal>, Receiver<Signal>) = mpsc::channel();

        // Set the Sender.
        *SENDER.lock().unwrap() = Some(sender);

        // Listen for signals on the main parent process.
        let fn_sender = merge_sender.clone();
        let _channel_listener: JoinHandle<Result<()>> = std::thread::spawn(move || {
            log::trace!("Spawned FFI listener thread.");

            while let Ok(signal) = receiver.recv() {
                match signal {
                    Signal::Kill => {

                        // Send the signal to the merge channel.
                        fn_sender.send(signal)?;

                        break;
                    }
                    _ => {
                        fn_sender.send(signal)?;
                    }
                }
            }

            Ok(())
        });
    });

    merge_receiver
}

pub fn send(signal: Signal) -> Result<()> {
    if let Some(sender) = &*SENDER.lock()? {
        sender.send(signal)?;
    } else {
        return Err(PyroscopeError::new("FFI channel not initialized"));
    }
    Ok(())
}
