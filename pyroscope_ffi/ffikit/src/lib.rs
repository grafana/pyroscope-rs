use bincode::{config, Decode, Encode};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use pyroscope::error::Result;
use std::io::{prelude::*, BufReader, Read, Write};
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
            let socket_address = format!("/tmp/PYROSCOPE-{}", get_parent_pid());
            println!("RECV == Receiving Socket address: {}", socket_address);
            let listener = LocalSocketListener::bind(socket_address).unwrap();

            listener.incoming().for_each(|packet| {
                println!("RECV == Received socket packet");
                let mut conn = BufReader::new(packet.unwrap());
                let mut buffer = [0; 2048];
                conn.read(&mut buffer).unwrap();

                println!("RECV == Client answered: {:?}", buffer);

                let (signal, len): (Signal, usize) =
                    bincode::decode_from_slice(&buffer, config::standard()).unwrap();

                socket_sender.send(signal).unwrap();
                println!("RECV == Sent signal to merge channel");
            });
        });
    });

    Ok(merge_receiver)
}

pub fn send(signal: Signal) -> Result<()> {
    // Check if SENDER is set.
    // Send signal through forked process.
    if get_parent_pid() != std::process::id() {
        let socket_address = format!("/tmp/PYROSCOPE-{}", get_parent_pid());
        println!("SEND == Socket address: {}", socket_address);

        let mut conn = LocalSocketStream::connect(socket_address).unwrap();
        //conn.set_nonblocking(true).unwrap();

        // encode signal
        let buffer = bincode::encode_to_vec(&signal, config::standard()).unwrap();
        //let buffer = b"Hello World";

        println!("SEND == Buffer to send {:?}", &buffer);

        conn.write_all(&buffer).unwrap();
        conn.flush().unwrap();

        drop(conn);

        println!("SEND == Sent buffer through socket");
    } else {
        // Send signal through parent process.
        unsafe {
            println!("SEND == Send through main function");
            SENDER
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .send(signal)
                .unwrap();
            println!("SEND == Sent through main function");
        }
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
