extern crate pyroscope;

use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};

use pyroscope::timer::Timer;

fn main() {
    // Initialize the Timer
    let mut timer = Timer::default().initialize();

    // Create a streaming channel
    let (tx, rx): (Sender<u64>, Receiver<u64>) = channel();

    let (tx2, rx2): (Sender<u64>, Receiver<u64>) = channel();

    // Attach tx to Timer
    timer.attach_listener(tx).unwrap();
    timer.attach_listener(tx2).unwrap();

    // Listen to the Timer events
    std::thread::spawn(move || {
        while let Ok(time) = rx.recv() {
            println!("Thread 1 Notification: {}", time);
        }
    });

    std::thread::spawn(move || {
        while let Ok(time) = rx2.recv() {
            println!("Thread 2 Notification: {}", time);
        }
    }).join().unwrap();
}
