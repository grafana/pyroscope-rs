// Copyright 2021 Developers of Pyroscope.

// Licensed under the Apache License, Version 2.0 <LICENSE or
// https://www.apache.org/licenses/LICENSE-2.0>. This file may not be copied, modified, or distributed
// except according to those terms.

// Possibly: ios, netbsd, openbsd, freebsd
#[cfg(target_os = "macos")]
pub mod kqueue;
#[cfg(target_os = "macos")]
pub use kqueue::Timer;

// Possibly: android
#[cfg(target_os = "linux")]
pub mod epoll;
#[cfg(target_os = "linux")]
pub use epoll::Timer;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub mod sleep;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub use sleep::Timer;
