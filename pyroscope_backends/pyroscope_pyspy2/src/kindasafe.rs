use std::arch::asm;
use std::ffi::c_void;
use std::fmt::{Debug, Display, Formatter};
use anyhow::Context;
use crate::signalhandlers::{new_signal_handler, restore_signal_handler};


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

#[inline(never)]
pub fn read_u64(at: usize) -> Result<u64, Error>{
    let mut result: u64 = 0;
    let mut signal: u64 = 0;
    unsafe {
        asm!(
        "ldr x0, [{at}]",
        "mov x1, #0x0",
        out("x0")  result,
        out("x1")  signal,
        at = in(reg) at,
        );
    }
    if signal == 0 {
        return Ok(result);
    }
    return Err(Error(signal));
}

static mut fallback_SIGSEGV: Option<libc::sigaction> = None;
static mut fallback_SIGBUS: Option<libc::sigaction> = None;
static mut read_begin: usize = 0;
static mut read_end: usize = 0;

unsafe fn find_ret() -> anyhow::Result<usize> {
    // todo amd
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

pub fn init() -> anyhow::Result<()> {
    println!(" read_u64 at {:016x}", read_u64 as u64);


    unsafe {
        read_begin = read_u64 as usize;
        read_end = read_begin + find_ret()
            .context("failed to find ret instruction")?;
        let prev = new_signal_handler(libc::SIGSEGV, segv_handler as usize)
            .context("kindasafe failed to install sigsegv handler")?;
        fallback_SIGSEGV = Some(prev);
        let prev = new_signal_handler(libc::SIGBUS, segv_handler as usize)
            .context("kindasafe failed to install sigbus handler")?;
        fallback_SIGBUS = Some(prev);
    }


    if let Ok(_) = read_u64(0xcafebabe) {
        return Err(anyhow::anyhow!("read_u64 failed sanity check"));
    }
    if let Ok(_) = read_u64(0x0) {
        return Err(anyhow::anyhow!("read_u64 failed sanity check"));
    }

    Ok(())
}

pub fn destroy() -> anyhow::Result<()> {
    if let Some(fallback) = unsafe { fallback_SIGSEGV } {
        unsafe {
            restore_signal_handler(libc::SIGSEGV, fallback)
                .context("kindasafe failed to restore sigsegv handler")?;
        }
    }
    if let Some(fallback) = unsafe { fallback_SIGBUS } {
        unsafe {
            restore_signal_handler(libc::SIGBUS, fallback)
                .context("kindasafe failed to restore sigbus handler")?;
        }
    }
    Ok(())
}

fn fallback(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
    //todo
    // if sig == SIGSEGV {
    //     unsafe {
    //         if let Some(fallback) = fallback_SIGSEGV {
    //             fallback.sa_sigaction(sig, info, data);
    //         }
    //     }
    // } else {
    //     unsafe {
    //         if let Some(fallback) = fallback_SIGBUS {
    //             fallback.sa_sigaction(sig, info, data);
    //         }
    //     }
    // }
}

extern "C" fn segv_handler(sig: libc::c_int, info: *mut libc::siginfo_t, data: *mut c_void) {
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_read_u64() {
        assert!(super::init().is_ok());

        let x = 0x123456789abcdef0;
        let x_ptr = &x as *const u64 as usize;
        let i = super::read_u64(x_ptr);
        println!("i: {:?}", i);
        assert_eq!(i, Ok(x));

        assert!(super::destroy().is_ok())
    }


    #[test]
    fn test_read_u64_fail() {
        assert!(super::init().is_ok());
        let x: i64 = 0x123456789abcdef0;
        let x_ptr = 0xcafebabe;
        let i = super::read_u64(x_ptr);
        println!("i: {:?}", i);
        assert_eq!(i, Err(super::Error(libc::SIGSEGV as u64)));
        assert!(super::destroy().is_ok());
    }

    // #[test]
    // fn test_fallback() {
    //     todo!("test_fallback")
    // }
}
