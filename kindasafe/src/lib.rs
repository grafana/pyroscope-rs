pub mod errors;
pub use errors::{DestroyError, InitError};
mod handlers;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Debug, PartialEq)]
pub enum ReadMemError {
    Signal { signal: u64 },
    NotInitialized,
}

pub type Ptr = u64;

pub fn u64(at: Ptr) -> Result<Ptr, ReadMemError> {
    arch::read_u64(at)
}

// todo arm64
#[cfg(target_arch = "x86_64")]
mod arch {
    use std::arch::asm;
    use std::mem;

    use super::is_initialized;

    #[inline(never)]
    pub fn read_vec(at: crate::Ptr, buf: &mut [u8]) -> Result<(), crate::ReadMemError> {
        let mut signal: u64;
        unsafe {
            if !is_initialized() {
                return Err(crate::ReadMemError::NotInitialized);
            }
            // todo maybe read qwords or owords
            asm!(
            "mov rsi, {at}",
            "mov rdi, {buf}",
            "mov rcx, {len}",
            "rep movsb",    // f3 a4     other       rep    movsb	(%rsi), %es:(%rdi)
            "xor rdx, rdx", // 48 31 d2  other       xorq   %rdx, %rdx
            at = in(reg) at,
            buf = in(reg) buf.as_ptr(),
            len = in(reg) buf.len(),
            out("rdx")  signal,
            out("rsi")  _,
            out("rdi")  _,
            out("rcx")  _,
            );
        }
        if signal == 0 {
            return Ok(());
        }
        Err(crate::ReadMemError::Signal { signal })
    }

    #[inline(never)]
    pub fn read_u64(at: crate::Ptr) -> Result<crate::Ptr, crate::ReadMemError> {
        //todo objdump the generated code in release profile
        let mut result: u64;
        let mut signal: u64;
        unsafe {
            if !is_initialized() {
                //todo think of api where we don't need this check at all
                return Err(crate::ReadMemError::NotInitialized);
            }
            asm!(
            "mov rdi, {at}",
            "mov rax, [rdi]",  // 48 8b 07
            "xor rdx, rdx",    // 48 31 d2
            out("rax")  result,
            out("rdx")  signal,
            out("rdi")  _,
            at = in(reg) at,
            );
        }
        if signal == 0 {
            return Ok(result);
        }
        Err(crate::ReadMemError::Signal { signal })
    }

    pub fn find_read_u64_insn() -> Option<usize> {
        find_insn(read_u64 as usize, 0xd2_31_48_07_8b_48, 0x0000ffffffffffff)
    }

    pub fn find_read_vec_insn() -> Option<usize> {
        find_insn(read_vec as usize, 0xd2_31_48_a4_f3, 0x000000ffffffffff)
    }

    fn find_insn(fptr: usize, marker: u64, mask: u64) -> Option<usize> {
        #[cfg(debug_assertions)]
        const SEARCH_DEPTH: usize = 200;
        #[cfg(not(debug_assertions))]
        const SEARCH_DEPTH: usize = 50;

        for i in 0..SEARCH_DEPTH {
            let insnp = ((fptr as usize) + i) as *const u64;
            let insn = unsafe { std::ptr::read_unaligned(insnp) } & mask;
            if insn == marker {
                return Some(insnp as usize);
            }
        }
        None
    }

    pub fn crash_handler(sig: libc::c_int, info: *const libc::siginfo_t, data: *mut libc::c_void) {
        unsafe {
            if !is_initialized() {
                // wtf
                let mut action: libc::sigaction = mem::zeroed();
                action.sa_sigaction = libc::SIG_DFL;
                libc::sigaction(sig, &action, std::ptr::null_mut());
                return;
            }

            let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
            if ctx.is_null() {
                super::fallback(sig, info, data);
                return;
            }
            let pc = (*ctx).uc_mcontext.gregs[libc::REG_RIP as usize] as usize;

            if pc == super::READ_U64_INSN {
                (*ctx).uc_mcontext.gregs[libc::REG_RIP as usize] = (pc + 6) as libc::greg_t;
                (*ctx).uc_mcontext.gregs[libc::REG_RAX as usize] = 0 as libc::greg_t;
                (*ctx).uc_mcontext.gregs[libc::REG_RDX as usize] = sig as u64 as libc::greg_t;
                return;
            }
            if pc == super::READ_VEC_INSN {
                (*ctx).uc_mcontext.gregs[libc::REG_RIP as usize] = (pc + 5) as libc::greg_t;
                (*ctx).uc_mcontext.gregs[libc::REG_RAX as usize] = 0 as libc::greg_t;
                (*ctx).uc_mcontext.gregs[libc::REG_RDX as usize] = sig as u64 as libc::greg_t;
                return;
            }
            super::fallback(sig, info, data);
        }
    }
}

// todo think how to have less static mut
static mut FALLBACK_SIGSEGV: libc::sigaction = unsafe { std::mem::zeroed() };
static mut FALLBACK_SIGBUS: libc::sigaction = unsafe { std::mem::zeroed() };
static mut READ_U64_INSN: usize = 0;
static mut READ_VEC_INSN: usize = 0;
static INIT: AtomicBool = AtomicBool::new(false);

fn is_initialized() -> bool {
    INIT.load(Ordering::Acquire)
}

pub struct InitOptions {
    sanity_check_ok: Vec<Ptr>,
    sanity_check_err: Vec<Ptr>,
}

impl Default for InitOptions {
    fn default() -> Self {
        InitOptions {
            sanity_check_ok: vec![arch::read_u64 as Ptr, arch::read_vec as Ptr],
            sanity_check_err: vec![0x0, 0xcafe000],
        }
    }
}

//todo create unloadable API
pub fn init() -> Result<(), InitError> {
    init_with_options(InitOptions::default())
}

pub fn init_with_options(opt: InitOptions) -> Result<(), InitError> {
    if is_initialized() {
        return Err(InitError::AlreadyInitialized);
    }
    unsafe {
        READ_U64_INSN = arch::find_read_u64_insn().ok_or(InitError::ReadInsnNotFound)?;
        READ_VEC_INSN = arch::find_read_vec_insn().ok_or(InitError::ReadVecInsnNotFound)?;
        install_handlers()?;
        INIT.store(true, Ordering::Release)
    };

    if let Err(e) = sanity_checks(opt) {
        _ = destroy();
        return Err(e);
    }

    Ok(())
}

fn install_handlers() -> Result<(), InitError> {
    let sigsegv = handlers::new_signal_handler(libc::SIGSEGV, arch::crash_handler);
    let sigbus = handlers::new_signal_handler(libc::SIGBUS, arch::crash_handler);
    match (sigsegv, sigbus) {
        (Ok(sigsegv), Ok(sigbus)) => {
            unsafe {
                FALLBACK_SIGSEGV = sigsegv;
                FALLBACK_SIGBUS = sigbus;
            }
            Ok(())
        }
        (Err(_), Ok(sigbus)) => {
            _ = handlers::restore_signal_handler(libc::SIGBUS, sigbus);
            Err(InitError::InstallSignalHandlersFailed)
        }
        (Ok(sigsegv), Err(_)) => {
            _ = handlers::restore_signal_handler(libc::SIGSEGV, sigsegv);
            Err(InitError::InstallSignalHandlersFailed)
        }
        (Err(_), Err(_)) => Err(InitError::InstallSignalHandlersFailed),
    }
}

fn sanity_checks(opt: InitOptions) -> Result<(), InitError> {
    let reads = |x: &Ptr| {
        let mut buf = [0u8; 8];
        let r1 = arch::read_u64(*x).map(|_| ());
        let r2 = arch::read_vec(*x, &mut buf);
        return vec![r1, r2];
    };

    let errs = opt.sanity_check_err.iter().flat_map(reads);
    for e in errs {
        match e {
            Err(ReadMemError::Signal { signal }) => {
                if signal != libc::SIGSEGV as u64 {
                    return Err(InitError::SanityCheckFailed);
                }
            }
            Err(ReadMemError::NotInitialized) => {
                return Err(InitError::SanityCheckFailed);
            }
            Ok(_) => {
                return Err(InitError::SanityCheckFailed);
            }
        }
    }

    let oks = opt.sanity_check_ok.iter().flat_map(reads);
    for e in oks {
        match e {
            Ok(_) => {}
            Err(_) => {
                return Err(InitError::SanityCheckFailed);
            }
        }
    }
    Ok(())
}

//todo make it private
pub fn destroy() -> Result<(), DestroyError> {
    if !is_initialized() {
        return Err(DestroyError::NotInitialized);
    }
    destroy_handlers()
}

fn destroy_handlers() -> Result<(), DestroyError> {
    unsafe {
        let sigsegv = handlers::restore_signal_handler(libc::SIGSEGV, FALLBACK_SIGSEGV);
        let sigbus = handlers::restore_signal_handler(libc::SIGBUS, FALLBACK_SIGBUS);
        INIT.store(false, Ordering::Release);
        FALLBACK_SIGSEGV = std::mem::zeroed();
        FALLBACK_SIGBUS = std::mem::zeroed();

        match (sigsegv, sigbus) {
            (Ok(_), Ok(_)) => Ok(()),
            _ => Err(DestroyError::RestoreHandlersFailed),
        }
    }
}

unsafe fn fallback(sig: libc::c_int, info: *const libc::siginfo_t, data: *mut libc::c_void) {
    fn call_fallback(
        sig: libc::c_int,
        info: *const libc::siginfo_t,
        data: *mut libc::c_void,
        fallback: libc::sigaction,
    ) {
        if fallback.sa_sigaction == 0 {
            handlers::restore_default(sig);
        } else {
            // In practice, we call rust's signal handler from stack_overflow.rs
            let handler = unsafe {
                std::mem::transmute::<
                    usize,
                    extern "C" fn(libc::c_int, *const libc::siginfo_t, *mut libc::c_void),
                >(fallback.sa_sigaction)
            };
            handler(sig, info, data);
        }
    }
    if sig == libc::SIGSEGV {
        call_fallback(sig, info, data, unsafe { FALLBACK_SIGSEGV }); // todo there may be a race if someone calls destroy concurrently

        return;
    }
    if sig == libc::SIGBUS {
        call_fallback(sig, info, data, unsafe { FALLBACK_SIGBUS }); // todo there may be a race if someone calls destroy concurrently
        return;
    }
}

pub static SERIALIZE_TESTS_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub mod tests {
    use super::{init_with_options, InitError, Ptr, ReadMemError};

    pub struct TestScopedInit {}
    impl TestScopedInit {
        pub fn new() -> Result<Self, InitError> {
            super::init()?;
            Ok(Self {})
        }
    }
    impl Drop for TestScopedInit {
        fn drop(&mut self) {
            assert_eq!(super::destroy(), Ok(()));
        }
    }

    // #[derive(Debug, PartialEq)]
    // pub enum ReadMemErrorTestMirror {
    //     // todo this feels like java LOL
    //     Signal { signal: u64 },
    //     NotInitialized,
    // }
    //
    // impl From<sigsafe::ReadMemError> for ReadMemErrorTestMirror {
    //     fn from(value: ReadMemError) -> Self {
    //         match value {
    //             ReadMemError::Signal { signal } => Signal { signal },
    //             ReadMemError::NotInitialized => NotInitialized,
    //         }
    //     }
    // }

    pub fn serialize(f: impl FnOnce() -> Result<(), anyhow::Error>) -> Result<(), anyhow::Error> {
        let _shared = super::SERIALIZE_TESTS_LOCK.lock();
        match _shared {
            Ok(_) => f(),
            Err(_) => {
                bail!("could not serialize lock")
            }
        }
    }

    #[test]
    fn test_read_u64() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef];
            let x_ptr = x.as_ptr() as Ptr;

            let i = super::arch::read_u64(x_ptr)
                .or_else(|err| Err(anyhow!("read mem error {err:?}")))?;
            assert_eq!(i, 0xefbeaddebebafeca);
            Ok(())
        })
    }

    #[test]
    fn test_read_u64_unaligned() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0x00];
            let x_ptr = x.as_ptr() as Ptr + 1;
            let i = super::arch::read_u64(x_ptr)
                .or_else(|err| Err(anyhow!("read mem error {err:?}")))?;
            assert_eq!(i, 0xefbeaddebebafe);
            Ok(())
        })
    }

    #[test]
    fn test_read_u64_fail() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            unsafe {
                let x_ptr: usize;
                x_ptr = libc::mmap(
                    0xdead000 as *mut libc::c_void,
                    8,
                    libc::PROT_NONE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                ) as usize;
                let i = super::arch::read_u64(x_ptr as Ptr);
                libc::munmap(x_ptr as *mut libc::c_void, 8);
                assert_eq!(
                    i,
                    Err(ReadMemError::Signal {
                        signal: libc::SIGSEGV as u64
                    })
                );
            }
            Ok(())
        })
    }

    #[test]
    fn test_read_vec() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            let mut buf = vec![0u8; 8];
            let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef];
            super::arch::read_vec(x.as_ptr() as Ptr, &mut buf)
                .or_else(|err| Err(anyhow!("read mem error {err:?}")))?;
            assert_eq!(buf, x.clone());
            Ok(())
        })
    }

    #[test]
    fn test_read_vec_unaligned() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            let mut buf = vec![0u8; 8];
            let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0xcc];
            let x_ptr = x.as_ptr() as Ptr + 1;
            super::arch::read_vec(x_ptr, &mut buf[0..7])
                .or_else(|err| Err(anyhow!("read mem error {err:?}")))?;
            let expected: Vec<u8> = vec![0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0];
            assert_eq!(buf, expected);
            Ok(())
        })
    }

    #[test]
    fn test_read_vec_fail() -> Result<(), anyhow::Error> {
        serialize(|| unsafe {
            let _init = TestScopedInit::new()?;
            let x_ptr = libc::mmap(
                0xdead000 as *mut libc::c_void,
                8,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) as usize;
            let mut buf = [0u8; 8];
            let i = super::arch::read_vec(x_ptr as Ptr, &mut buf);
            libc::munmap(x_ptr as *mut libc::c_void, 8);
            assert_eq!(
                i,
                Err(ReadMemError::Signal {
                    signal: libc::SIGSEGV as u64
                })
            );
            Ok(())
        })
    }

    #[test]
    fn test_read_vec_fail_page_boundary() -> Result<(), anyhow::Error> {
        serialize(|| unsafe {
            let _init = TestScopedInit::new()?;
            let x_ptr = libc::mmap(
                0xdead000 as *mut libc::c_void,
                0x2000,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) as usize;
            libc::mprotect(
                x_ptr as *mut libc::c_void,
                0x1000,
                libc::PROT_READ | libc::PROT_WRITE,
            );
            libc::memset(x_ptr as *mut libc::c_void, 0x61, 0x1000);
            let mut buf = [0u8; 16];
            let i = super::arch::read_vec((x_ptr + 0x1000 - 8) as Ptr, &mut buf);
            libc::munmap(x_ptr as *mut libc::c_void, 0x2000);
            assert_eq!(
                i,
                Err(ReadMemError::Signal {
                    signal: libc::SIGSEGV as u64
                })
            );
            assert_eq!(
                buf,
                vec![
                    0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                    0x0, 0x0
                ]
                .as_slice()
            );
            Ok(())
        })
    }

    #[test]
    fn test_fallback_sigsegv() -> Result<(), anyhow::Error> {
        serialize(|| {
            let prev = super::handlers::new_signal_handler(
                libc::SIGSEGV,
                fallback_sigsegv_sigbus_crash_handler,
            )?;
            {
                let _init = TestScopedInit::new()?;
                FALLBACK_CALLED.store(0, Ordering::SeqCst);
                trigger_sigsegv();
                trigger_sigsegv();
                assert_eq!(FALLBACK_CALLED.load(Ordering::SeqCst), 2);
            }
            super::handlers::restore_signal_handler(libc::SIGSEGV, prev)?;
            Ok(())
        })
    }

    #[test]
    fn test_fallback_sigbus() -> Result<(), anyhow::Error> {
        serialize(|| {
            let prev = super::handlers::new_signal_handler(
                libc::SIGBUS,
                fallback_sigsegv_sigbus_crash_handler,
            )?;
            {
                let _init = TestScopedInit::new()?;
                FALLBACK_CALLED.store(0, Ordering::SeqCst);
                trigger_sigbus();
                trigger_sigbus();
                assert_eq!(FALLBACK_CALLED.load(Ordering::SeqCst), 2);
            }
            super::handlers::restore_signal_handler(libc::SIGBUS, prev)?;
            Ok(())
        })
    }

    #[test]
    fn test_init_twice() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            assert_eq!(super::init(), Err(InitError::AlreadyInitialized));
            Ok(())
        })
    }

    #[test]
    fn test_read_uninitialized() -> Result<(), anyhow::Error> {
        serialize(|| {
            assert!(!super::is_initialized());
            let mut buf = [0u8; 8];
            let x: u64 = 0x123456789abcdef0;
            let res = super::arch::read_vec(&x as *const u64 as Ptr, &mut buf);
            assert_eq!(res, Err(ReadMemError::NotInitialized));
            Ok(())
        })
    }

    #[test]
    fn test_sanity_check_fai() -> Result<(), anyhow::Error> {
        serialize(|| {
            let opt = super::InitOptions {
                sanity_check_ok: vec![0x0],
                sanity_check_err: vec![0x0],
            };
            let res = init_with_options(opt);
            assert_eq!(res, Err(InitError::SanityCheckFailed));
            assert!(!super::is_initialized());
            Ok(())
        })
    }

    // use crate::tests::ReadMemErrorTestMirror::{NotInitialized, Signal};
    use anyhow::{anyhow, bail};
    use std::sync::atomic::{AtomicI32, Ordering};

    static FALLBACK_CALLED: AtomicI32 = AtomicI32::new(0);

    fn trigger_sigsegv() {
        unsafe {
            let m = libc::mmap(
                0xdead000 as *mut libc::c_void,
                4,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            let m = m as *mut i32;
            *m = 0;
            libc::munmap(m as *mut libc::c_void, 4);
        }
    }

    fn trigger_sigbus() {
        unsafe {
            let f = libc::tmpfile();
            let m = libc::mmap(
                0 as *mut libc::c_void,
                4,
                libc::PROT_WRITE,
                libc::MAP_PRIVATE,
                libc::fileno(f),
                0,
            );
            let m = m as *mut i32;
            *m = 0;

            libc::munmap(m as *mut libc::c_void, 4);
            libc::fclose(f);
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn fallback_sigsegv_sigbus_crash_handler(
        _sig: libc::c_int,
        _info: *const libc::siginfo_t,
        data: *mut libc::c_void,
    ) {
        use libc::{greg_t, REG_RIP};

        unsafe {
            let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
            let pc = (*ctx).uc_mcontext.gregs[REG_RIP as usize] as usize;
            (*ctx).uc_mcontext.gregs[REG_RIP as usize] = (pc + 6) as greg_t;
            FALLBACK_CALLED.fetch_add(1, Ordering::SeqCst);
        }
    }
}
