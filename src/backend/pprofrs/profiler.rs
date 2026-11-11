// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use std::convert::TryInto;
use std::os::raw::c_int;
use std::time::SystemTime;

use nix::sys::signal;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use spin::RwLock;

use crate::backend::pprofrs::backtrace::{Trace, TraceImpl};
use crate::backend::pprofrs::collector::Collector;
use crate::backend::pprofrs::error::{Error, Result};
use crate::backend::pprofrs::frames::UnresolvedFrames;
use crate::backend::pprofrs::report::ReportBuilder;
use crate::backend::pprofrs::timer::Timer;
use crate::backend::pprofrs::{MAX_DEPTH, MAX_THREAD_NAME};

pub(crate) static PROFILER: Lazy<RwLock<Result<Profiler>>> =
    Lazy::new(|| RwLock::new(Profiler::new()));

pub struct Profiler {
    pub(crate) data: Collector<UnresolvedFrames>,
    sample_counter: usize,
    sample_limit: Option<usize>,
    individual_samples: bool,

    running: bool,

    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "loongarch64"
    ))]
    blocklist_segments: Vec<(usize, usize)>,
}

#[derive(Clone)]
pub struct ProfilerGuardBuilder {
    frequency: c_int,
    sample_limit: Option<usize>,
    individual_samples: bool,

    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "loongarch64"
    ))]
    blocklist_segments: Vec<(usize, usize)>,
}

impl Default for ProfilerGuardBuilder {
    fn default() -> ProfilerGuardBuilder {
        ProfilerGuardBuilder {
            frequency: 99,
            sample_limit: None,
            individual_samples: false,

            #[cfg(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64",
                target_arch = "loongarch64"
            ))]
            blocklist_segments: Vec::new(),
        }
    }
}

impl ProfilerGuardBuilder {
    /// Retains separate observations for timeline export instead of combining
    /// identical stacks. Adds a platform-clock timestamp and OS thread ID to each
    /// observation.
    ///
    /// Disabled by default. Memory usage grows with the number of samples; use
    /// [`Self::sample_limit`] to bound collection.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn retain_individual_samples(self) -> Self {
        Self {
            individual_samples: true,
            ..self
        }
    }

    /// Stops accepting observations after `limit` samples, before unwinding any
    /// further stacks. By default collection is unlimited. A limit of zero
    /// disables sample collection.
    pub fn sample_limit(self, limit: usize) -> Self {
        Self {
            sample_limit: Some(limit),
            ..self
        }
    }

    pub fn frequency(self, frequency: c_int) -> Self {
        Self { frequency, ..self }
    }

    pub fn build(self) -> Result<ProfilerGuard<'static>> {
        trigger_lazy();

        match PROFILER.write().as_mut() {
            Err(err) => {
                log::error!("Error in creating profiler: {}", err);
                Err(Error::Creating)
            }
            Ok(profiler) => {
                if profiler.running {
                    return Err(Error::Running);
                }
                profiler.sample_limit = self.sample_limit;
                profiler.individual_samples = self.individual_samples;
                #[cfg(any(
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                    target_arch = "riscv64",
                    target_arch = "loongarch64"
                ))]
                {
                    profiler.blocklist_segments = self.blocklist_segments;
                }

                match profiler.start() {
                    Ok(()) => Ok(ProfilerGuard::<'static> {
                        profiler: &PROFILER,
                        timer: Some(Timer::new(self.frequency)),
                    }),
                    Err(err) => Err(err),
                }
            }
        }
    }
}

/// RAII structure used to stop profiling when dropped. It is the only interface to access profiler.
pub struct ProfilerGuard<'a> {
    profiler: &'a RwLock<Result<Profiler>>,
    timer: Option<Timer>,
}

fn trigger_lazy() {
    let _ = backtrace::Backtrace::new();
    let _profiler = PROFILER.read();
    TraceImpl::init();
}

impl ProfilerGuard<'_> {
    /// Generate a report
    pub fn report(&self) -> ReportBuilder<'_> {
        ReportBuilder::new(self.profiler)
    }
}

impl<'a> Drop for ProfilerGuard<'a> {
    fn drop(&mut self) {
        drop(self.timer.take());

        match self.profiler.write().as_mut() {
            Err(_) => {}
            Ok(profiler) => match profiler.stop() {
                Ok(()) => {}
                Err(err) => log::error!("error while stopping profiler {}", err),
            },
        }
    }
}

fn write_thread_name_fallback(current_thread: libc::pthread_t, name: &mut [libc::c_char]) {
    let mut len = 0;
    let mut base = 1;

    while current_thread as u128 > base && len < MAX_THREAD_NAME {
        base *= 10;
        len += 1;
    }

    let mut index = 0;
    while index < len && base > 1 {
        base /= 10;

        name[index] = match (48 + (current_thread as u128 / base) % 10).try_into() {
            Ok(digit) => digit,
            Err(_) => {
                log::error!("fail to convert thread_id to string");
                0
            }
        };

        index += 1;
    }
}

#[cfg(not(all(any(target_os = "linux", target_os = "macos"), target_env = "gnu")))]
fn write_thread_name(current_thread: libc::pthread_t, name: &mut [libc::c_char]) {
    write_thread_name_fallback(current_thread, name);
}

#[cfg(all(any(target_os = "linux", target_os = "macos"), target_env = "gnu"))]
fn write_thread_name(current_thread: libc::pthread_t, name: &mut [libc::c_char]) {
    let name_ptr = name as *mut [libc::c_char] as *mut libc::c_char;
    let ret = unsafe { libc::pthread_getname_np(current_thread, name_ptr, MAX_THREAD_NAME) };

    if ret != 0 {
        write_thread_name_fallback(current_thread, name);
    }
}

struct ErrnoProtector(libc::c_int);

impl ErrnoProtector {
    fn new() -> Self {
        unsafe {
            #[cfg(target_os = "android")]
            {
                let errno = *libc::__errno();
                Self(errno)
            }
            #[cfg(target_os = "linux")]
            {
                let errno = *libc::__errno_location();
                Self(errno)
            }
            #[cfg(any(target_os = "macos", target_os = "freebsd"))]
            {
                let errno = *libc::__error();
                Self(errno)
            }
        }
    }
}

impl Drop for ErrnoProtector {
    fn drop(&mut self) {
        unsafe {
            #[cfg(target_os = "android")]
            {
                *libc::__errno() = self.0;
            }
            #[cfg(target_os = "linux")]
            {
                *libc::__errno_location() = self.0;
            }
            #[cfg(any(target_os = "macos", target_os = "freebsd"))]
            {
                *libc::__error() = self.0;
            }
        }
    }
}

#[no_mangle]
#[cfg_attr(
    not(all(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "loongarch64"
    ))),
    allow(unused_variables)
)]
#[allow(clippy::unnecessary_cast)]
extern "C" fn perf_signal_handler(
    _signal: c_int,
    _siginfo: *mut libc::siginfo_t,
    ucontext: *mut libc::c_void,
) {
    let _errno = ErrnoProtector::new();

    if let Some(mut guard) = PROFILER.try_write() {
        if let Ok(profiler) = guard.as_mut() {
            if !profiler.running || profiler.sample_limit_reached() {
                return;
            }
            #[cfg(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64",
                target_arch = "loongarch64"
            ))]
            if !ucontext.is_null() {
                let ucontext: *mut libc::ucontext_t = ucontext as *mut libc::ucontext_t;

                #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                let addr =
                    unsafe { (*ucontext).uc_mcontext.gregs[libc::REG_RIP as usize] as usize };

                #[cfg(all(target_arch = "x86_64", target_os = "freebsd"))]
                let addr = unsafe { (*ucontext).uc_mcontext.mc_rip as usize };

                #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
                let addr = unsafe {
                    let mcontext = (*ucontext).uc_mcontext;
                    if mcontext.is_null() {
                        0
                    } else {
                        (*mcontext).__ss.__rip as usize
                    }
                };

                #[cfg(all(
                    target_arch = "aarch64",
                    any(target_os = "android", target_os = "linux")
                ))]
                let addr = unsafe { (*ucontext).uc_mcontext.pc as usize };

                #[cfg(all(target_arch = "aarch64", target_os = "freebsd"))]
                let addr = unsafe { (*ucontext).mc_gpregs.gp_elr as usize };

                #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
                let addr = unsafe {
                    let mcontext = (*ucontext).uc_mcontext;
                    if mcontext.is_null() {
                        0
                    } else {
                        (*mcontext).__ss.__pc as usize
                    }
                };

                #[cfg(all(target_arch = "riscv64", target_os = "linux"))]
                let addr = unsafe { (*ucontext).uc_mcontext.__gregs[libc::REG_PC] as usize };

                #[cfg(all(target_arch = "loongarch64", target_os = "linux"))]
                let addr = unsafe { (*ucontext).uc_mcontext.__pc as usize };

                if profiler.is_blocklisted(addr) {
                    return;
                }
            }

            let mut bt: SmallVec<[<TraceImpl as Trace>::Frame; MAX_DEPTH]> =
                SmallVec::with_capacity(MAX_DEPTH);
            let mut index = 0;

            let sample_timestamp: SystemTime = SystemTime::now();
            let (trace_timestamp_ns, native_thread_id) =
                sample_metadata(profiler.individual_samples);
            if profiler.individual_samples && (trace_timestamp_ns == 0 || native_thread_id == 0) {
                return;
            }
            TraceImpl::trace(ucontext, |frame| {
                if index < MAX_DEPTH {
                    bt.push(frame.clone());
                    index += 1;
                    true
                } else {
                    false
                }
            });

            let current_thread = unsafe { libc::pthread_self() };
            let mut name = [0; MAX_THREAD_NAME];
            let name_ptr = &mut name as *mut [libc::c_char] as *mut libc::c_char;

            write_thread_name(current_thread, &mut name);

            let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
            profiler.sample(
                bt,
                name.to_bytes(),
                current_thread as u64,
                sample_timestamp,
                trace_timestamp_ns,
                native_thread_id,
            );
        }
    }
}

impl Profiler {
    fn new() -> Result<Self> {
        Ok(Profiler {
            data: Collector::new()?,
            sample_counter: 0,
            sample_limit: None,
            individual_samples: false,
            running: false,

            #[cfg(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64",
                target_arch = "loongarch64"
            ))]
            blocklist_segments: Vec::new(),
        })
    }

    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "loongarch64"
    ))]
    fn is_blocklisted(&self, addr: usize) -> bool {
        for libs in &self.blocklist_segments {
            if addr > libs.0 && addr < libs.1 {
                return true;
            }
        }
        false
    }
}

impl Profiler {
    pub fn start(&mut self) -> Result<()> {
        log::info!("starting cpu profiler");
        if self.running {
            Err(Error::Running)
        } else {
            self.register_signal_handler()?;
            self.running = true;

            Ok(())
        }
    }

    fn init(&mut self) -> Result<()> {
        self.sample_counter = 0;
        self.data.clear()?;
        self.running = false;

        Ok(())
    }

    /// Clear the sample data collector without stopping profiling.
    /// Signal handler and timer remain active — only the accumulated
    /// samples are discarded.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotRunning`] if the profiler is not currently active.
    /// Returns an I/O error if truncating the overflow backing file fails.
    ///
    /// NOTE: pyroscope patch — added to support periodic report collection
    /// without recreating the ProfilerGuard. See <https://github.com/grafana/pyroscope-rs/issues/399>
    pub fn clear(&mut self) -> Result<()> {
        if self.running {
            self.sample_counter = 0;
            self.data.clear()?;
            Ok(())
        } else {
            Err(Error::NotRunning)
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        log::info!("stopping cpu profiler");
        if self.running {
            self.unregister_signal_handler()?;
            self.init()?;

            Ok(())
        } else {
            Err(Error::NotRunning)
        }
    }

    fn register_signal_handler(&mut self) -> Result<()> {
        let handler = signal::SigHandler::SigAction(perf_signal_handler);
        // SA_RESTART will only restart a syscall when it's safe to do so,
        // e.g. when it's a blocking read(2) or write(2). See man 7 signal.
        let flags = signal::SaFlags::SA_SIGINFO | signal::SaFlags::SA_RESTART;
        let sigaction = signal::SigAction::new(handler, flags, signal::SigSet::empty());
        _ = unsafe { signal::sigaction(signal::SIGPROF, &sigaction) }?;
        Ok(())
    }

    // On macOS 26 ARM64 the kernel passes the handler address directly to
    // _sigtramp even when the signal handler is SIG_IGN (value 1). _sigtramp
    // branches to it unconditionally, so address 0x1 is not instruction-aligned
    // → SIGBUS (PC alignment fault). A real function pointer is always a valid
    // aligned address.
    // https://github.com/grafana/pyroscope-pprof-rs/pull/40/changes
    #[cfg(target_os = "macos")]
    fn unregister_signal_handler(&mut self) -> Result<()> {
        extern "C" fn noop(_: c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {}
        let noop = signal::SigAction::new(
            signal::SigHandler::SigAction(noop),
            signal::SaFlags::SA_SIGINFO | signal::SaFlags::SA_RESTART,
            signal::SigSet::empty(),
        );
        unsafe { signal::sigaction(signal::SIGPROF, &noop) }?;
        Ok(())
    }

    // Never restore SIG_DFL: SIGPROF's default action terminates the process,
    // so a pending signal delivered between unregister and re-register would be
    // fatal. SIG_IGN safely discards those signals on non-macOS platforms.
    // See https://github.com/tikv/pprof-rs/issues/288
    //     https://github.com/grafana/pprof-rs/pull/8
    #[cfg(not(target_os = "macos"))]
    fn unregister_signal_handler(&mut self) -> Result<()> {
        let ignore = signal::SigAction::new(
            signal::SigHandler::SigIgn,
            signal::SaFlags::empty(),
            signal::SigSet::empty(),
        );
        unsafe { signal::sigaction(signal::SIGPROF, &ignore) }?;
        Ok(())
    }

    fn sample_limit_reached(&self) -> bool {
        self.sample_limit
            .map_or(false, |limit| self.sample_counter >= limit)
    }

    // This function has to be AS-safe
    pub fn sample(
        &mut self,
        backtrace: SmallVec<[<TraceImpl as Trace>::Frame; MAX_DEPTH]>,
        thread_name: &[u8],
        thread_id: u64,
        sample_timestamp: SystemTime,
        trace_timestamp_ns: u64,
        native_thread_id: u64,
    ) {
        if self.sample_limit_reached() {
            return;
        }
        let mut frames = UnresolvedFrames::new(backtrace, thread_name, thread_id, sample_timestamp);
        frames.trace_timestamp_ns = trace_timestamp_ns;
        frames.os_thread_id = native_thread_id;
        self.sample_counter = self.sample_counter.saturating_add(1);

        if let Ok(()) = self.data.add(frames, 1) {}
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    struct AllocDetector {
        should_count_alloc: std::sync::atomic::AtomicBool,
        alloc_count: std::sync::atomic::AtomicUsize,
    }

    unsafe impl std::alloc::GlobalAlloc for AllocDetector {
        unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
            if self
                .should_count_alloc
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                self.alloc_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }

            unsafe { std::alloc::System.alloc(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
            unsafe { std::alloc::System.dealloc(ptr, layout) }
        }
    }
    impl AllocDetector {
        fn enable_count_alloc(&self) {
            self.alloc_count
                .store(0, std::sync::atomic::Ordering::SeqCst);
            self.should_count_alloc
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        fn disable_count_alloc(&self) {
            self.should_count_alloc
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }

        fn alloc_count(&self) -> usize {
            self.alloc_count.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[global_allocator]
    static ALLOC: AllocDetector = AllocDetector {
        should_count_alloc: std::sync::atomic::AtomicBool::new(false),
        alloc_count: std::sync::atomic::AtomicUsize::new(0),
    };

    #[test]
    fn test_stop_clears_samples_without_allocating() {
        // Like the unwind test, run with --test-threads 1 for allocation counting.
        let mut profiler = Profiler::new().unwrap();
        profiler.start().unwrap();
        profiler.sample(SmallVec::new(), b"old", 1, SystemTime::now(), 0, 0);

        ALLOC.enable_count_alloc();
        let result = profiler.stop();
        ALLOC.disable_count_alloc();

        result.unwrap();
        assert_eq!(ALLOC.alloc_count(), 0);
        assert!(!profiler.running);
        assert_eq!(profiler.sample_counter, 0);
        assert_eq!(profiler.data.try_iter().unwrap().count(), 0);

        profiler.start().unwrap();
        profiler.sample(SmallVec::new(), b"new", 2, SystemTime::now(), 0, 0);
        assert_eq!(profiler.sample_counter, 1);
        {
            let mut samples = profiler.data.try_iter().unwrap();
            let sample = samples.next().unwrap();
            assert_eq!(sample.item.thread_id, 2);
            assert_eq!(sample.count, 1);
            assert!(samples.next().is_none());
        }
        profiler.stop().unwrap();
    }

    #[test]
    fn test_no_alloc_during_unwind() {
        // This test cannot run parallelly because it requires the global allocator to
        // record the allocation count.

        trigger_lazy();
        PROFILER.write().as_mut().unwrap().start().unwrap();
        let timer = Timer::new(999);
        let start = std::time::Instant::now();
        ALLOC.enable_count_alloc();

        // alloc something to make sure the ALLOC works fine.
        let _alloc = Box::new(1usize);
        // busy loop for a while to trigger some samples
        while start.elapsed().as_millis() < 500 {
            std::hint::black_box(());
        }
        ALLOC.disable_count_alloc();

        assert_eq!(ALLOC.alloc_count(), 1);

        drop(timer);
        PROFILER.write().as_mut().unwrap().stop().unwrap();
    }
}

// Use non-adjustable OS clocks so observations can be correlated with other
// timeline sources: BOOTTIME on Linux and UPTIME_RAW on macOS.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn trace_clock_ns() -> u64 {
    #[cfg(target_os = "macos")]
    let clock = libc::CLOCK_UPTIME_RAW;
    #[cfg(target_os = "linux")]
    let clock = libc::CLOCK_BOOTTIME;
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: initialized output, supported clock, async-signal-safe API.
    if unsafe { libc::clock_gettime(clock, &mut ts) } != 0 {
        return 0;
    }
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}
#[cfg(target_os = "macos")]
fn os_thread_id() -> u64 {
    let mut tid = 0;
    // SAFETY: null selects the current thread; tid is a valid output pointer.
    if unsafe { libc::pthread_threadid_np(0, &mut tid) } != 0 {
        return 0;
    }
    tid
}
#[cfg(target_os = "linux")]
fn os_thread_id() -> u64 {
    // SAFETY: gettid has no pointer arguments.
    unsafe { libc::syscall(libc::SYS_gettid) as u64 }
}

// Read metadata before unwinding so the timestamp locates the observation, not
// the end of stack collection. Other platforms retain aggregated samples only.
fn sample_metadata(individual_samples: bool) -> (u64, u64) {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if individual_samples {
        return (trace_clock_ns(), os_thread_id());
    }
    let _ = individual_samples;
    (0, 0)
}

#[cfg(test)]
mod sample_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn default_collection_aggregates_and_individual_collection_preserves_observations() {
        for individual in [false, true] {
            let mut profiler = Profiler::new().unwrap();
            for offset in 1..=2 {
                profiler.sample(
                    SmallVec::new(),
                    b"worker",
                    7,
                    SystemTime::UNIX_EPOCH + Duration::from_secs(offset),
                    if individual { offset } else { 0 },
                    if individual { 42 } else { 0 },
                );
            }
            let lock = RwLock::new(Ok(profiler));
            let report = ReportBuilder::new(&lock).build_and_clear(false).unwrap();
            assert_eq!(report.data.len(), if individual { 2 } else { 1 });
            assert_eq!(report.data.values().sum::<isize>(), 2);
            for (frames, count) in report.data {
                assert_eq!(frames.thread_id, 7);
                if individual {
                    assert_eq!(count, 1);
                    assert_eq!(frames.os_thread_id, 42);
                    assert!(frames.trace_timestamp_ns > 0);
                } else {
                    assert_eq!(count, 2);
                    assert_eq!(frames.os_thread_id, 0);
                }
            }
        }
    }

    #[test]
    fn sample_limit_bounds_collection_and_defaults_to_unlimited() {
        assert_eq!(ProfilerGuardBuilder::default().sample_limit, None);
        assert!(!ProfilerGuardBuilder::default().individual_samples);
        for limit in [0, 2] {
            let mut profiler = Profiler::new().unwrap();
            profiler.sample_limit = Some(limit);
            for _ in 0..3 {
                profiler.sample(SmallVec::new(), b"worker", 7, SystemTime::UNIX_EPOCH, 0, 0);
            }
            assert_eq!(profiler.sample_counter, limit);
            assert!(profiler.sample_limit_reached());
            let lock = RwLock::new(Ok(profiler));
            let report = ReportBuilder::new(&lock).build_and_clear(false).unwrap();
            assert_eq!(report.data.values().sum::<isize>(), limit as isize);
        }
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn individual_sample_metadata_uses_a_stable_os_thread_and_monotonic_clock() {
        assert_eq!(sample_metadata(false), (0, 0));
        let first = sample_metadata(true);
        let second = sample_metadata(true);
        assert!(first.0 > 0);
        assert!(second.0 >= first.0);
        assert!(first.1 > 0);
        assert_eq!(first.1, second.1);
    }
}
