#![no_std]

mod mmap;
mod eventfd;
mod syscall;
mod errno_guard;

pub use spin::Mutex;

pub type ShardMutex<T> = spin::Mutex<T>;
