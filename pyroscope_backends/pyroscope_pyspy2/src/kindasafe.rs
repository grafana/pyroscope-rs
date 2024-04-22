use std::arch::asm;
use std::ffi::c_void;
use std::io::{stdout, Write};
use anyhow::Context;
use libc::{exit, SIGSEGV};
use crate::signalhandlers::{new_signal_handler, restore_signal_handler};

#[inline(never)]
pub fn read_u64(at: usize) -> u64 {
    let mut result: u64 = 0;
    unsafe {
        asm!(
        "ldr x0, [{at}]",
        out("x0")  result,
        at = in(reg) at,
        );
    }
    result
}

static mut fallback_SIGSEGV: Option<libc::sigaction> = None;
static mut fallback_SIGBUS: Option<libc::sigaction> = None;
static mut read_begin: usize = 0;
static mut read_end: usize = 0;

unsafe fn find_ret() -> anyhow::Result<usize> {
    // todo amd
    const ret: u32 = 0xd65f03c0;
    for i in 0..20 {
        let insnp = ((read_u64 as usize) + i * 4) as *const u32;
        let insn = *insnp;
        if insn == ret {
            return Ok(i * 4);
        }
    }
    bail!("failed to find ret on arm64") //todo hexdump insns
}

pub fn init() -> anyhow::Result<()> {
    unsafe {
        read_begin = read_u64 as usize;
        read_end = read_begin + find_ret()
            .context("failed to find ret instruction")?;
        let prev = new_signal_handler(SIGSEGV, segv_handler as usize)
            .context("kindasafe failed to install sigsegv handler")?;
        fallback_SIGSEGV = Some(prev);
        let prev = new_signal_handler(libc::SIGBUS, segv_handler as usize)
            .context("kindasafe failed to install sigbus handler")?;
        fallback_SIGBUS = Some(prev);
    }
    println!(" read_u64 at {:016x}", read_u64 as u64);
    if read_u64(0xcafebabe) != 0 {
        return Err(anyhow::anyhow!("read_u64 failed sanity check"));
    }

    Ok(())
}

pub fn destroy() -> anyhow::Result<()> {
    if let Some(fallback) = unsafe { fallback_SIGSEGV } {
        unsafe {
            restore_signal_handler(SIGSEGV, fallback)
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
    if sig == SIGSEGV {
        unsafe {
            if let Some(fallback) = fallback_SIGSEGV {
                fallback.sa_sigaction(sig, info, data);
            }
        }
    } else {
        unsafe {
            if let Some(fallback) = fallback_SIGBUS {
                fallback.sa_sigaction(sig, info, data);
            }
        }
    }
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
            (*mctx).__ss.__pc = pc as u64 + 4;
            (*mctx).__ss.__x[0] = 0;
        } else {}
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_read_u64() {
        super::init().unwrap();

        let x = 0x123456789abcdef0;
        let x_ptr = &x as *const u64 as usize;
        let i = super::read_u64(x_ptr);
        assert_eq!(i, x);

        super::destroy().unwrap()
    }


    #[test]
    fn test_read_u64_fail() {
        println!("test_read_u64_fail");
        super::init().unwrap();
        let x = 0x123456789abcdef0;
        let x_ptr = 0xcafebabe;
        let i = super::read_u64(x_ptr);
        assert_eq!(i, x);
        super::destroy().unwrap();
    }
}
