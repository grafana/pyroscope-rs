use bincode::{config, Decode, Encode};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use lazy_static::lazy_static;
use pyroscope::error::{Result, PyroscopeError};
use std::{
    io::{BufReader, Read, Write},
    sync::{
        atomic::AtomicU32,
        mpsc::{self, Receiver, Sender},
        Mutex, Once,
    },
    thread::JoinHandle,
};

/// Logging Tag
const LOG_TAG: &str = "Pyroscope::FFIKit";

/// PID of the Root Process.
pub static PARENT_PID: AtomicU32 = AtomicU32::new(0);

/// Once is used to ensure a unique agent.
static ONCE: Once = Once::new();

lazy_static! {
    /// Root Sender
    /// This is the sender to the main loop. It is lazily initialized inside a Mutex.
    static ref SENDER: Mutex<Option<Sender<Signal>>> = Mutex::new(None);
}

/// Signal enum.
/// This enum is used to send signals to the main loop. It is used to add/remove global or thread
/// tags and to exit the main loop.
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub enum Signal {
    Kill,
    AddGlobalTag(String, String),
    RemoveGlobalTag(String, String),
    AddThreadTag(u64, String, String),
    RemoveThreadTag(u64, String, String),
}

/// pre-fork initialization.
pub fn initialize_ffi() -> Result<Receiver<Signal>> {
    // Create another channel to merge communication.
    let (merge_sender, merge_receiver): (Sender<Signal>, Receiver<Signal>) = mpsc::channel();

    ONCE.call_once(|| {
        // Set the parent PID.
        set_parent_pid();

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
                        log::info!(target: LOG_TAG, "FFI channel received kill signal.");

                        // Send the signal to the merge channel.
                        fn_sender.send(signal)?;

                        break;
                    }
                    _ => {
                        log::trace!(target: LOG_TAG, "FFI channel received signal: {:?}", signal);

                        // Send the signal to the merge channel.
                        fn_sender.send(signal)?;

                        log::trace!(target: LOG_TAG, "Sent FFI signal to merge channel");
                    }
                }
            }

            Ok(())
        });

        // Listen for signals on local socket
        let socket_sender = merge_sender.clone();
        let _socket_listener: JoinHandle<Result<()>> = std::thread::spawn(move || {
            let socket_address = format!("/tmp/PYROSCOPE-{}", get_parent_pid());

            log::trace!(
                target: LOG_TAG,
                "FFI Socket Listening on {}",
                socket_address
            );

            // Bind to the socket.
            match LocalSocketListener::bind(socket_address) {
                Ok(listener) => {
                    // Listen for connections.
                    listener
                        .incoming()
                        .map(|packet| {
                            log::trace!(target: LOG_TAG, "Received socket packet");

                            // Read the packet using a BufReader.
                            let mut conn = BufReader::new(packet?);
                            // Initialize a buffer to store the message.
                            let mut buffer = [0; 2048];
                            // Read the message.
                            conn.read(&mut buffer)?;

                            // Decode the message.
                            let (signal, _len): (Signal, usize) =
                                bincode::decode_from_slice(&buffer, config::standard()).unwrap();

                            // Send the signal to the merge channel.
                            socket_sender.send(signal.clone())?;

                            if signal == Signal::Kill {
                                log::info!(target: LOG_TAG, "FFI socket received kill signal.");
                                return Ok(());
                            }

                            log::trace!(target: LOG_TAG, "Sent Socket signal to merge channel");

                            return Ok(());
                        })
                        .collect::<Result<()>>()?;
                }
                Err(error) => {
                    log::error!(target: LOG_TAG, "Socket failed to bind {} - can't receive signals", error)
                }
            }
            Ok(())
        });
    });

    // Return the merge channel receiver.
    Ok(merge_receiver)
}

pub fn send(signal: Signal) -> Result<()> {
    if get_parent_pid() != std::process::id() {
        let socket_address = format!("/tmp/PYROSCOPE-{}", get_parent_pid());
        match LocalSocketStream::connect(socket_address) {
            Ok(mut conn) => {
                let buffer = bincode::encode_to_vec(&signal, config::standard()).unwrap();
                conn.write_all(&buffer)?;
                conn.flush()?;
            }
            Err(error) => {
                log::error!(target: LOG_TAG, "Socket failed to connect {}", error)
            }
        }
    } else {
        if let Some(sender) = &*SENDER.lock()? {
            sender.send(signal)?;
        } else {
            return Err(PyroscopeError::new( "FFI channel not initialized"));
        }
    }

    Ok(())
}

/// Set the parent PID.
fn set_parent_pid() {
    // Get the parent PID.
    let pid = std::process::id();

    log::trace!(target: LOG_TAG, "Set PARENT_PID: {}", pid);

    // Set the parent PID.
    PARENT_PID.store(pid, std::sync::atomic::Ordering::Relaxed);
}

/// Returns the PID of the Parent Process.
/// This can be called from forks, threads, or any other context.
pub fn get_parent_pid() -> u32 {
    // Retrieve the parent PID.
    PARENT_PID.load(std::sync::atomic::Ordering::Relaxed)
}
