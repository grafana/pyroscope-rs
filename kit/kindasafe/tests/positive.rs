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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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
