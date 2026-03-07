use anyhow::anyhow;
use kindasafe::ReadMemError;

use anyhow::Result;
use kindasafe::{Ptr, slice, u64};

#[test]
fn test_init() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    Ok(())
}

#[test]
fn u64_aligned() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;

    let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef];
    let x_ptr = x.as_ptr() as Ptr;

    let i = u64(x_ptr).map_err(|err| anyhow!("read mem error {err:?}"))?;
    assert_eq!(i, 0xefbeaddebebafeca);
    Ok(())
}

#[test]
fn u64_unaligned() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;

    let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0x00];
    let x_ptr = x.as_ptr() as Ptr + 1;
    let i = u64(x_ptr).map_err(|err| anyhow!("read mem error {err:?}"))?;
    assert_eq!(i, 0xefbeaddebebafe);
    Ok(())
}

#[test]
fn u64_sigsegv() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    trigger_sigsegv(|p| {
        assert_eq!(
            u64(p),
            Err(ReadMemError {
                signal: libc::SIGSEGV as u64
            })
        );
    });
    Ok(())
}

#[test]
fn u64_sigbus() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    trigger_sigbus(|p| {
        assert_eq!(
            u64(p),
            Err(ReadMemError {
                signal: libc::SIGBUS as u64
            })
        );
    });
    Ok(())
}

#[test]
fn u64_unaligned_page_boundary() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;

    trigger_sigsegv_page_boundary(|p| {
        assert_eq!(u64(p), Ok(0x6161616161616161));
        assert_eq!(u64(p + 0x1000 - 0x8), Ok(0x6161616161616161));
        assert_eq!(
            u64(p + 0x1000 - 0x7),
            Err(ReadMemError {
                signal: libc::SIGSEGV as u64
            })
        );
        assert_eq!(
            u64(p + 0x1000),
            Err(ReadMemError {
                signal: libc::SIGSEGV as u64
            })
        );
    });
    Ok(())
}

#[test]
fn vec_aligned() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    let mut buf = vec![0u8; 8];
    let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef];
    slice(&mut buf, x.as_ptr() as Ptr).map_err(|err| anyhow!("read mem error {err:?}"))?;
    assert_eq!(buf, x.clone());
    Ok(())
}

#[test]
fn vec_unaligned() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    let mut buf = vec![0u8; 8];
    let x: Vec<u8> = vec![0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0xcc];
    let x_ptr = x.as_ptr() as Ptr + 1;
    slice(&mut buf[0..7], x_ptr).map_err(|err| anyhow!("read mem error {err:?}"))?;
    let expected: Vec<u8> = vec![0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0];
    assert_eq!(buf, expected);
    Ok(())
}

#[test]
fn vec_sigsegv() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    trigger_sigsegv(|p| {
        let mut buf = [0u8; 8];
        let res = slice(&mut buf, p as Ptr);
        assert_eq!(
            res,
            Err(ReadMemError {
                signal: libc::SIGSEGV as u64
            })
        );
    });
    Ok(())
}

#[test]
fn vec_sigbus() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    trigger_sigbus(|p| {
        let mut buf = [0u8; 8];
        let res = slice(&mut buf, p as Ptr);
        assert_eq!(
            res,
            Err(ReadMemError {
                signal: libc::SIGBUS as u64
            })
        );
    });
    Ok(())
}
#[test]
fn vec_sigsegv_page_boundary() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;

    trigger_sigsegv_page_boundary(|p| {
        let mut buf = [0u8; 16];
        let i = slice(&mut buf, (p + 0x1000 - 8) as Ptr);
        assert_eq!(
            i,
            Err(ReadMemError {
                signal: libc::SIGSEGV as u64
            })
        );
        assert_eq!(
            buf,
            vec![
                0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x61, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                0x0
            ]
            .as_slice()
        );
    });
    Ok(())
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn fs_0x0() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    let res = kindasafe::arch::fs_0x0();
    assert_eq!(0, res.signal);
    assert_ne!(0, res.value);

    // fs:0x0 is the self-pointer (TCB), which equals the FS base.
    let fs_base = get_fs_base();
    assert_ne!(0, fs_base);
    assert_eq!(fs_base, res.value);

    Ok(())
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn fs_0x10() -> Result<(), anyhow::Error> {
    kindasafe_init::init().map_err(|err| anyhow!("{:?}", err))?;
    let res = kindasafe::arch::fs_0x10();
    assert_eq!(0, res.signal);
    assert_ne!(0, res.value);

    let fs_base = get_fs_base();
    assert_ne!(0, fs_base);

    //todo test SIGSEGV failure
    // todo this crashes twice, once on the PROT_NONE, then next in the crash handler
    // unsafe {
    //     libc::mprotect((fs_base & 0xfffffffffffff000) as *mut libc::c_void, 0x1000, libc::PROT_NONE);
    //     let res = kindasafenostd::arch::fs_0x10();
    //     assert_eq!(0, res.signal);
    //     assert_ne!(0, res.value);
    //     libc::mprotect((fs_base & 0xfffffffffffff000) as *mut libc::c_void, 0x1000, libc::PROT_READ | libc::PROT_WRITE);
    // }

    Ok(())
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
fn trigger_sigsegv_page_boundary<F>(mut cb: F)
where
    F: FnMut(Ptr),
{
    unsafe {
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

        cb(x_ptr as Ptr);

        libc::munmap(x_ptr as *mut libc::c_void, 0x2000);
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub fn trigger_sigbus<F>(mut cb: F)
where
    F: FnMut(u64),
{
    unsafe {
        let f = libc::tmpfile();
        let m = libc::mmap(
            std::ptr::null_mut::<libc::c_void>(),
            4,
            libc::PROT_WRITE,
            libc::MAP_PRIVATE,
            libc::fileno(f),
            0,
        );
        let m = m as *mut i32;
        cb(m as u64);

        libc::munmap(m as *mut libc::c_void, 4);
        libc::fclose(f);
    };
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub fn trigger_sigsegv<F>(mut cb: F)
where
    F: FnMut(u64),
{
    unsafe {
        let m = libc::mmap(
            std::ptr::null_mut::<libc::c_void>(),
            4,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(libc::MAP_FAILED, m);
        let m = m as *mut i32;
        cb(m as u64);

        libc::munmap(m as *mut libc::c_void, 4);
    };
}

fn get_fs_base() -> u64 {
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
            libc::raise(libc::SIGTRAP);
            std::process::exit(0);
        }
        let mut status: libc::c_int = 0;
        let mut regs: libc::user_regs_struct = std::mem::zeroed();
        libc::waitpid(pid, &mut status, 0);
        libc::ptrace(libc::PTRACE_GETREGS, pid, 0, &mut regs);
        libc::ptrace(libc::PTRACE_CONT, pid, 0, 0);
        libc::waitpid(pid, &mut status, 0);
        println!("ptrace_getregs {:x}", regs.fs_base);
        regs.fs_base
    }
}
