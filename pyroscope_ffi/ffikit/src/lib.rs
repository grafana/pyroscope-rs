use bincode::{config, Decode, Encode};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use pyroscope::error::Result;
use std::io::{Read, Write};
use std::sync::{
    atomic::AtomicU32,
    mpsc::{self, Receiver, Sender},
    Mutex, Once,
};

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
    AddThreadTag(String, String),
    RemoveThreadTag(String, String),
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

        let fn_sender = merge_sender.clone();
        // Listen for signals on the main parent process.
        std::thread::spawn(move || {
            while let Ok(Signal) = receiver.recv() {
                // Send the signal to the merge channel.
                fn_sender.send(Signal).unwrap();
            }
        });

        let socket_sender = merge_sender.clone();
        // Listen for signals on local socket
        std::thread::spawn(move || {
            let listener =
                LocalSocketListener::bind(format!("/tmp/PYROSCOPE-{}", get_parent_pid())).unwrap();

            let config = config::standard();

            listener.incoming().for_each(|packet| {
                let mut read_buffer = packet.unwrap();
                let mut buffer = String::new();
                read_buffer.read_to_string(&mut buffer).unwrap();
                let (signal, len): (Signal, usize) =
                    bincode::decode_from_slice(&buffer.as_bytes(), config).unwrap();
                // Send the signal to the merge channel.
                socket_sender.send(signal).unwrap();
            });
        });
    });

    Ok(merge_receiver)
}

pub fn send(signal: Signal) -> Result<()> {
    // Check if SENDER is set.
    // Send signal through forked process.
    unsafe {
        if SENDER.is_none() {
            let mut conn =
                LocalSocketStream::connect(format!("/tmp/PYROSCOPE-{}", get_parent_pid())).unwrap();
            // encode signal
            let buffer = bincode::encode_to_vec(&signal, config::standard()).unwrap();

            conn.write_all(&buffer).unwrap();
        }
    }

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

    Ok(())
}

fn set_parent_pid() {
    PARENT_PID.store(std::process::id(), std::sync::atomic::Ordering::Relaxed);
}

/// Returns the PID of the Parent Process.
/// This can be called from forks, threads, or any other context.
pub fn get_parent_pid() -> u32 {
    PARENT_PID.load(std::sync::atomic::Ordering::Relaxed)
}
