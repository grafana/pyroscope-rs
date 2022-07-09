use bincode::{config, Decode, Encode};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use pyroscope::error::Result;
use std::io::{BufReader, Read, Write};
use std::sync::{
    atomic::AtomicU32,
    mpsc::{self, Receiver, Sender},
    Mutex, Once,
};

/// Logging Tag
const LOG_TAG: &str = "Pyroscope::FFIKit";

/// PID of the Root Process.
pub static PARENT_PID: AtomicU32 = AtomicU32::new(0);

/// Once is used to ensure a unique agent.
static ONCE: Once = Once::new();

/// Root Sender
static mut SENDER: Option<Mutex<Sender<Signal>>> = None;

#[derive(Debug, Encode, Decode)]
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
        unsafe {
            SENDER = Some(Mutex::new(sender));
        }

        // Listen for signals on the main parent process.
        let fn_sender = merge_sender.clone();
        std::thread::spawn(move || {
            log::trace!("Spawned FFI listener thread.");

            while let Ok(signal) = receiver.recv() {
                match signal {
                    Signal::Kill => {
                        log::info!(target: LOG_TAG, "FFI channel received kill signal.");
                        break;
                    }
                    _ => {
                        log::trace!(target: LOG_TAG, "FFI channel received signal: {:?}", signal);

                        // Send the signal to the merge channel.
                        fn_sender.send(signal).unwrap();

                        log::trace!(target: LOG_TAG, "Sent FFI signal to merge channel");
                    }
                }
            }
        });

        // Listen for signals on local socket
        let socket_sender = merge_sender.clone();
        std::thread::spawn(move || {
            let socket_address = format!("/tmp/PYROSCOPE-{}", get_parent_pid());

            log::trace!(
                target: LOG_TAG,
                "FFI Socket Listening on {}",
                socket_address
            );

            // Bind to the socket.
            let listener = LocalSocketListener::bind(socket_address).unwrap();

            // Listen for connections.
            listener.incoming().for_each(|packet| {
                log::trace!(target: LOG_TAG, "Received socket packet");

                // Read the packet using a BufReader.
                let mut conn = BufReader::new(packet.unwrap());
                // Initialize a buffer to store the message.
                let mut buffer = [0; 2048];
                // Read the message.
                conn.read(&mut buffer).unwrap();

                // Decode the message.
                let (signal, _len): (Signal, usize) =
                    bincode::decode_from_slice(&buffer, config::standard()).unwrap();

                // Send the signal to the merge channel.
                socket_sender.send(signal).unwrap();

                log::trace!(target: LOG_TAG, "Sent Socket signal to merge channel");
            });
        });
    });

    // Return the merge channel receiver.
    Ok(merge_receiver)
}

pub fn send(signal: Signal) -> Result<()> {
    // Check if SENDER is set.
    // Send signal through forked process.
    if get_parent_pid() != std::process::id() {
        let socket_address = format!("/tmp/PYROSCOPE-{}", get_parent_pid());

        log::trace!(
            target: LOG_TAG,
            "Sending signal {:?} through socket {}",
            signal,
            &socket_address
        );

        // Connect to the socket.
        let mut conn = LocalSocketStream::connect(socket_address).unwrap();

        // encode signal
        let buffer = bincode::encode_to_vec(&signal, config::standard()).unwrap();

        // Write the message.
        conn.write_all(&buffer).unwrap();

        // Flush the connection.
        conn.flush().unwrap();
    } else {
        // Send signal through parent process.
        unsafe {
            SENDER
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .send(signal)
                .unwrap();
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
