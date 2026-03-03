#![no_std]

mod errno_guard;
mod mmap;
mod syscall;

pub use spin::Mutex;

pub type ShardMutex<T> = spin::Mutex<T>;

pub mod eventfd;
pub use eventfd::{EventFd, EventSet, EVENT_SET_CAPACITY};
