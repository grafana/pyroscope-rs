#![no_std]

pub mod auxv;
mod errno_guard;
pub mod mmap;
mod syscall;

pub use spin::Mutex;

pub type ShardMutex<T> = spin::Mutex<T>;

pub mod eventfd;
pub use eventfd::{EVENT_SET_CAPACITY, EventFd, EventSet};
