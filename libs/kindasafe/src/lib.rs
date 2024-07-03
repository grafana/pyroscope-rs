use std::mem;
use std::ffi::c_void;
use std::fmt::{Debug, Display, Formatter};
use anyhow::{Context};
use libc::{c_int, sigaction, siginfo_t};

pub use arch::read_vec;


pub fn new_signal_handler(signal: libc::c_int, handler: usize) -> std::result::Result<libc::sigaction, std::io::Error> {
    let mut new: libc::sigaction = unsafe { mem::zeroed() };
    new.sa_sigaction = handler as usize;
    new.sa_flags = libc::SA_RESTART | libc::SA_SIGINFO;
    let mut old: libc::sigaction = unsafe { mem::zeroed() };
    if unsafe { libc::sigaction(signal, &new, &mut old) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(old)
}


pub fn restore_signal_handler(signal: libc::c_int, prev: libc::sigaction) -> std::result::Result<(), std::io::Error> {
    let mut old: libc::sigaction = unsafe { mem::zeroed() };
    if unsafe { libc::sigaction(signal, &prev, &mut old) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
// pub fn start_timer(interval : libc::time_t) -> std::result::Result<(), Error> {
//     // let interval = 10000000; //
//     let sec = interval / 1000000000;
//     let usec = (interval % 1000000000) / 1000;
//     let mut tv: libc::itimerval = unsafe { mem::zeroed() };
//     tv.it_value.tv_sec = sec;
//     tv.it_value.tv_usec = usec as libc::suseconds_t;
//     tv.it_interval.tv_sec = sec;
//     tv.it_interval.tv_usec = usec as libc::suseconds_t;
//     if unsafe { libc::setitimer(libc::ITIMER_PROF, &tv, std::ptr::null_mut()) } != 0 {
//         return Err(Error::last_os_error());
//     }
//     return Ok(());
// }


#[derive(PartialEq)]
pub struct Error(u64);


impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "kindasafe error: {}", self.0)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "kindasafe error: {}", self.0)
    }
}

impl std::error::Error for Error {}


#[cfg(target_arch = "aarch64")]
mod arch_specific {
    use std::arch::asm;

    // #[inline(never)]
    // pub fn read_u64(at: usize) -> Result<u64, Error> {
    //     let mut result: u64;
    //     let mut signal: u64;
    //     unsafe {
    //         asm!(
    //         "ldr x0, [{at}]",
    //         "mov x1, #0x0",
    //         out("x0")  result,
    //         out("x1")  signal,
    //         at = in(reg) at,
    //         );
    //     }
    //     if signal == 0 {
    //         return Ok(result);
    //     }
    //     return Err(Error(signal));
    // }

    pub unsafe fn find_ret() -> anyhow::Result<usize> {
        const ret: u32 = 0xd65f03c0;
        for i in 0..40 {
            let insnp = ((read_u64 as usize) + i * 4) as *const u32;
            let insn = *insnp;
            if insn == ret {
                return Ok(i * 4);
            }
        }
        bail!("failed to find ret on arm64") //todo hexdump insns
    }

    pub fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
        unsafe {
            let ctx: *const libc::ucontext_t = data as *const libc::ucontext_t;
            if ctx.is_null() {
                fallback(sig, info, data);
                return;
            }
            let mctx = (*ctx).uc_mcontext;
            if mctx.is_null() {
                fallback(sig, info, data);
                return;
            }

            let mctx = (*ctx).uc_mcontext;
            let pc = (*mctx).__ss.__pc as usize;
            if pc >= read_begin && pc < read_end {
                (*mctx).__ss.__pc = pc as u64 + 8;
                (*mctx).__ss.__x[0] = 0;
                (*mctx).__ss.__x[1] = sig as u64;
            } else {}
        }
    }
}
pub const NOT_INITIALIZED: i64 = 239;


#[cfg(target_arch = "x86_64")]
mod arch {
    use std::arch::asm;
    use std::ffi::c_void;
    use anyhow::bail;
    use libc::{greg_t, REG_RAX, REG_RDX, REG_RIP};
    use crate::INITIALIZED;

    #[inline(never)]
    pub fn read_vec(at: usize, buf: &mut [u8]) -> Result<(), super::Error> {
        let mut signal: u64;
        unsafe {
            if !INITIALIZED {
                return Err(super::Error(super::NOT_INITIALIZED as u64));
            }
            //todo maybe read qwords or owords
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
        return Err(super::Error(signal));
    }

    // #[inline(never)]
    // pub fn read_u64(at: usize) -> Result<u64, super::Error> {
    //     let mut result: u64;
    //     let mut signal: u64;
    //     unsafe {
    //         if !INITIALIZED {
    //             return Err(super::Error(super::NOT_INITIALIZED as u64));
    //         }
    //         asm!(
    //         "mov rdi, {at}",
    //         "mov rax, [rdi]",  // 48 8b 07
    //         "xor rdx, rdx",    // 48 31 d2
    //         out("rax")  result,
    //         out("rdx")  signal,
    //         out("rdi")  _,
    //         at = in(reg) at,
    //         );
    //     }
    //     if signal == 0 {
    //         return Ok(result);
    //     }
    //     return Err(super::Error(signal));
    // }

    // pub unsafe fn find_read_u64_insn() -> anyhow::Result<usize> {
    //     return find_insn(read_u64 as usize, 0xd2_31_48_07_8b_48, 0x0000ffffffffffff)
    // }

    pub unsafe fn find_read_vec_insn() -> anyhow::Result<usize> {
        return find_insn(read_vec as usize, 0xd2_31_48_a4_f3, 0x000000ffffffffff);
    }

    pub unsafe fn find_insn(fptr: usize, marker: u64, mask: u64) -> anyhow::Result<usize> {
        for i in 0..50 {
            let insnp = ((fptr as usize) + i) as *const u64;
            let insn = std::ptr::read_unaligned(insnp) & mask;
            if insn == marker {
                return Ok(insnp as usize);
            }
        }
        bail!("failed to find insn on x86_64")  //todo hexdump insns
    }

    pub fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
        unsafe {
            let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
            if ctx.is_null() {
                super::fallback(sig, info, data);
                return;
            }
            let pc = (*ctx).uc_mcontext.gregs[REG_RIP as usize] as usize;

            // if pc == super::READ_U64_INSN {
            //     (*ctx).uc_mcontext.gregs[REG_RIP as usize] = (pc + 6) as greg_t;
            //     (*ctx).uc_mcontext.gregs[REG_RAX as usize] = 0 as greg_t;
            //     (*ctx).uc_mcontext.gregs[REG_RDX as usize] = sig as u64 as greg_t;
            //     return;
            // }
            if pc == super::READ_VEC_INSN {
                (*ctx).uc_mcontext.gregs[REG_RIP as usize] = (pc + 5) as greg_t;
                (*ctx).uc_mcontext.gregs[REG_RAX as usize] = 0 as greg_t;
                (*ctx).uc_mcontext.gregs[REG_RDX as usize] = sig as u64 as greg_t;
                return;
            }
            super::fallback(sig, info, data);
        }
    }
}


static mut FALLBACK_SIGSEGV: Option<libc::sigaction> = None;
static mut FALLBACK_SIGBUS: Option<libc::sigaction> = None;
// static mut READ_U64_INSN: usize = 0;
static mut READ_VEC_INSN: usize = 0;
static mut INITIALIZED: bool = false;


pub fn init() -> anyhow::Result<()> {
    // println!(" read_u64 at {:016x}", arch_specific::read_u64 as u64);


    unsafe {
        if INITIALIZED {
            return Err(anyhow::anyhow!("kindasafe already initialized"));
        }
        // READ_U64_INSN = arch::find_read_u64_insn()
        //     .context("failed to find read_u64 insn")?;
        READ_VEC_INSN = arch::find_read_vec_insn()
            .context("failed to find read_vec insn")?;
        let prev = new_signal_handler(libc::SIGSEGV, arch::crash_handler as usize)
            .context("kindasafe failed to install sigsegv handler")?;
        FALLBACK_SIGSEGV = Some(prev);
        let prev = new_signal_handler(libc::SIGBUS, arch::crash_handler as usize)
            .context("kindasafe failed to install sigbus handler")?;
        FALLBACK_SIGBUS = Some(prev);
    }


    // if let Ok(_) = read_u64(0xcafebabe) { // this could be a valid address
    //     return Err(anyhow::anyhow!("read_u64 failed sanity check"));
    // }

    // if let Ok(_) = arch::read_u64(0x0) {
    //     return Err(anyhow::anyhow!("read_u64 failed sanity check"));
    // }

    let mut buf = [0u8; 8];
    if let Ok(_) = arch::read_vec(0x0, &mut buf) {
        return Err(anyhow::anyhow!("read_u64 failed sanity check"));
    }
    unsafe {
        INITIALIZED = true;
    }
    Ok(())
}

pub fn destroy() -> anyhow::Result<()> {
    if let Some(fallback) = unsafe { FALLBACK_SIGSEGV } {
        restore_signal_handler(libc::SIGSEGV, fallback)
            .context("kindasafe failed to restore sigsegv handler")?;
    }
    if let Some(fallback) = unsafe { FALLBACK_SIGBUS } {
        restore_signal_handler(libc::SIGBUS, fallback)
            .context("kindasafe failed to restore sigbus handler")?;
    }
    unsafe {
        FALLBACK_SIGSEGV = None;
        FALLBACK_SIGBUS = None;
        INITIALIZED = false;
    }
    Ok(())
}

unsafe fn fallback(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
    unsafe fn call_fallback(sig: c_int, info: *mut siginfo_t, data: *mut c_void, fallback: Option<sigaction>) {
        if let Some(fallback) = fallback {
            if fallback.sa_sigaction == 0 {
                panic!("kindasafe: sigsegv fallback.sa_sigaction handler not installed (fallback.sa_sigaction == 0)")  // todo more useful message
                // todo check if panic is ok here
                // todo maybe we should just call abort() here
                // another idea is to unregister ourselves as the handler to avoid crash loops
            } else {
                let handler = std::mem::transmute::<usize, extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void)>(fallback.sa_sigaction);
                handler(sig, info, data);
            }
        } else {
            panic!("kindasafe: sigsegv fallback handler not installed") // todo more useful message
            // todo check if panic is ok here
            // todo maybe we should just call abort() here
            // another idea is to unregister ourselves as the handler to avoid crash loops
        }
    }
    if sig == libc::SIGSEGV {
        call_fallback(sig, info, data, FALLBACK_SIGSEGV.clone());
        return;
    }
    if sig == libc::SIGBUS {
        call_fallback(sig, info, data, FALLBACK_SIGBUS.clone());
        return;
    }
}


#[cfg(test)]
mod tests {
    use std::ffi::c_void;
    use once_cell::sync::Lazy;
    use std::{sync::Mutex};

    static SERIALIZE_TESTS_LOCK: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

    // #[test]
    // fn test_read_u64() {
    //     let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
    //
    //     assert!(super::init().is_ok());
    //     let x = 0x123456789abcdef0;
    //     let x_ptr = &x as *const u64 as usize;
    //     let i = super::arch::read_u64(x_ptr);
    //     assert_eq!(i, Ok(x));
    //
    //     assert!(super::destroy().is_ok())
    // }
    //
    // #[test]
    // fn test_read_u64_unaligned() {
    //     let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
    //
    //     assert!(super::init().is_ok());
    //     let x = 0x123456789abcdef0;
    //     let x_ptr = &x as *const u64 as usize + 1;
    //     let i = super::arch::read_u64(x_ptr);
    //     assert_eq!(i, Ok(x >> 8));
    //
    //     assert!(super::destroy().is_ok())
    // }
    // #[test]
    // fn test_read_u64_fail() {
    //     let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
    //
    //     assert!(super::init().is_ok());
    //     let x_ptr: usize;
    //     unsafe {
    //         x_ptr = libc::mmap(0xdead000 as *mut c_void, 8, libc::PROT_NONE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0) as usize;
    //     }
    //     let i = super::arch::read_u64(x_ptr);
    //     unsafe {
    //         libc::munmap(x_ptr as *mut c_void, 8);
    //     }
    //     assert_eq!(i, Err(super::Error(libc::SIGSEGV as u64)));
    //     assert!(super::destroy().is_ok());
    // }

    #[test]
    fn test_read_vec() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();

        assert!(super::init().is_ok());
        let mut buf = [0u8; 8];
        let x: u64 = 0x123456789abcdef0;
        let x_ptr = &x as *const u64 as usize;
        let i = super::arch::read_vec(x_ptr, &mut buf);
        assert_eq!(i, Ok(()));
        assert_eq!(buf, x.to_le_bytes());

        assert!(super::destroy().is_ok())
    }

    #[test]
    fn test_read_vec_unaligned() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();

        assert!(super::init().is_ok());
        let mut buf = [0u8; 8];
        let x: u64 = 0x123456789abcdef0;
        let x_ptr = &x as *const u64 as usize + 1;
        let i = super::arch::read_vec(x_ptr, &mut buf[0..7]);
        assert_eq!(i, Ok(()));
        assert_eq!(buf, (x >> 8).to_le_bytes());

        assert!(super::destroy().is_ok())
    }


    #[test]
    fn test_read_vec_fail() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();

        assert!(super::init().is_ok());
        let x_ptr: usize;
        unsafe {
            x_ptr = libc::mmap(0xdead000 as *mut c_void, 8, libc::PROT_NONE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0) as usize;
        }
        let mut buf = [0u8; 8];
        let i = super::arch::read_vec(x_ptr, &mut buf);
        unsafe {
            libc::munmap(x_ptr as *mut c_void, 8);
        }
        assert_eq!(i, Err(super::Error(libc::SIGSEGV as u64)));
        assert!(super::destroy().is_ok());
    }

    #[test]
    fn test_read_vec_fail_page_boundary() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();

        assert!(super::init().is_ok());
        let x_ptr: usize;
        unsafe {
            x_ptr = libc::mmap(0xdead000 as *mut c_void, 0x2000, libc::PROT_NONE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0) as usize;
            libc::mprotect(x_ptr as *mut c_void, 0x1000, libc::PROT_READ | libc::PROT_WRITE);
            libc::memset(x_ptr as *mut c_void, 0x61, 0x1000);
        }
        let mut buf = [0u8; 16];
        let i = super::arch::read_vec(x_ptr + 0x1000 - 8, &mut buf);
        unsafe {
            libc::munmap(x_ptr as *mut c_void, 0x2000);
        }
        assert_eq!(i, Err(super::Error(libc::SIGSEGV as u64)));
        assert_eq!(buf, vec![0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0].as_slice());
        assert!(super::destroy().is_ok());
    }


    #[test]
    fn test_fallback_sigsegv() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
        let prev = super::new_signal_handler(libc::SIGSEGV, fallback_sigsegv_sigbus_crash_handler as usize).unwrap();
        super::init().unwrap();
        unsafe {
            FALLBACK_CALLED = 0;
            trigger_sigsegv();
            trigger_sigsegv();
            assert_eq!(FALLBACK_CALLED, 2);
        }
        super::destroy().unwrap();
        super::restore_signal_handler(libc::SIGSEGV, prev).unwrap();
    }


    #[test]
    fn test_fallback_sigbus() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
        let prev = super::new_signal_handler(libc::SIGBUS, fallback_sigsegv_sigbus_crash_handler as usize).unwrap();
        super::init().unwrap();
        unsafe {
            FALLBACK_CALLED = 0;
            trigger_sigbus();
            trigger_sigbus();
            assert_eq!(FALLBACK_CALLED, 2);
        }

        super::destroy().unwrap();
        super::restore_signal_handler(libc::SIGBUS, prev).unwrap();
    }

    #[test]
    fn test_init_twice() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
        assert_eq!(super::init().is_ok(), true);
        assert_eq!(super::init().is_err(), true);
        assert_eq!(super::destroy().is_ok(), true);
    }


    #[test]
    fn test_read_uninitialized() {
        let _shared = SERIALIZE_TESTS_LOCK.lock().unwrap();
        unsafe {
            assert_eq!(false, super::INITIALIZED);
        }
        let mut buf = [0u8; 8];
        let x: u64 = 0x123456789abcdef0;
        let res = super::arch::read_vec(&x as *const u64 as usize, &mut buf);
        assert_eq!(res, Err(super::Error(super::NOT_INITIALIZED as u64)));
    }

    pub static mut FALLBACK_CALLED: i32 = 0;

    unsafe fn trigger_sigsegv() {
        let m = libc::mmap(0xdead000 as *mut c_void, 4, libc::PROT_NONE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0);
        let m = m as *mut i32;
        *m = 0;
        libc::munmap(m as *mut c_void, 4);
    }

    unsafe fn trigger_sigbus() {
        let f = libc::tmpfile();
        let m = libc::mmap(0 as *mut c_void, 4, libc::PROT_WRITE, libc::MAP_PRIVATE, libc::fileno(f), 0);
        let m = m as *mut i32;
        *m = 0;
        libc::munmap(m as *mut c_void, 4);
        libc::fclose(f);
    }


    #[cfg(target_arch = "x86_64")]
    unsafe fn fallback_sigsegv_sigbus_crash_handler(_sig: libc::c_int, _info: *mut libc::siginfo_t, data: *mut c_void) {
        use libc::{greg_t, REG_RIP};

        unsafe {
            let ctx: *mut libc::ucontext_t = data as *mut libc::ucontext_t;
            let pc = (*ctx).uc_mcontext.gregs[REG_RIP as usize] as usize;
            (*ctx).uc_mcontext.gregs[REG_RIP as usize] = (pc + 6) as greg_t; // c7 00 00 00 00 00        other       movl   $0x0, (%rax)
            FALLBACK_CALLED += 1;
        }
    }
}


