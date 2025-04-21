use sigsafe::{Ptr, ReadMem};
use std::arch::asm;
use std::marker::PhantomData;

pub struct KindaSafeMem;
impl ReadMem for KindaSafeMem {
    //todo separate mod
    fn read_u64(at: sigsafe::Ptr) -> Result<sigsafe::Ptr, sigsafe::MemError> {
        // todo look what code does this generate
        match kindasafe::u64(at) {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(sigsafe::MemError),
        }
    }
}

pub struct LibcReadTLS {}

impl sigsafe::ReadTLS for LibcReadTLS {
    fn read_tls(k: sigsafe::PthreadKey) -> Result<Ptr, sigsafe::MemError> {
        unsafe { Ok(libc::pthread_getspecific(k.v as libc::pthread_key_t) as Ptr) }
    }
}

pub struct ReadTLSMem<T: ReadMem> {
    phantom: PhantomData<T>,
}

impl<T: ReadMem> sigsafe::ReadTLS for ReadTLSMem<T> {
    // todo do we even need this? can't we just call the libc?
    #[cfg(target_arch = "x86_64")]
    fn read_tls(k: sigsafe::PthreadKey) -> Result<Ptr, sigsafe::MemError> {
        if k.v > 0x1f {
            return Err(sigsafe::MemError);
        }
        T::read_u64(((k.v + 0x31) << 4) as Ptr + fs_0x10() + 8)
        //    0x00007ffff789e5b4 <+4>:     cmp    edi,0x1f
        //    0x00007ffff789e5b7 <+7>:     ja     0x7ffff789e5f0 <___pthread_getspecific+64>
        //    0x00007ffff789e5b9 <+9>:     mov    eax,edi
        //    0x00007ffff789e5bb <+11>:    add    rax,0x31
        //    0x00007ffff789e5bf <+15>:    shl    rax,0x4
        //    0x00007ffff789e5c3 <+19>:    add    rax,QWORD PTR fs:0x10
        //    0x00007ffff789e5cc <+28>:    mov    rdx,QWORD PTR [rax+0x8]
        //    0x00007ffff789e5d0 <+32>:    test   rdx,rdx
        //    0x00007ffff789e5d3 <+35>:    je     0x7ffff789e628 <___pthread_getspecific+120>
        //    0x00007ffff789e5d5 <+37>:    mov    edi,edi
        //    0x00007ffff789e5d7 <+39>:    lea    rcx,[rip+0x1671a2]        # 0x7ffff7a05780 <__pthread_keys>
        //    0x00007ffff789e5de <+46>:    mov    rsi,QWORD PTR [rax]
        //    0x00007ffff789e5e1 <+49>:    shl    rdi,0x4
        //    0x00007ffff789e5e5 <+53>:    cmp    QWORD PTR [rcx+rdi*1],rsi
        //    0x00007ffff789e5e9 <+57>:    jne    0x7ffff789e620 <___pthread_getspecific+112>
        // => 0x00007ffff789e5eb <+59>:    mov    rax,rdx
        //    0x00007ffff789e5ee <+62>:    ret
    }
}

#[cfg(target_arch = "x86_64")]
fn fs_0x10() -> Ptr {
    unsafe {
        let res: Ptr;
        asm!(
        "mov    {ptr}, QWORD PTR fs:0x10",
        ptr = out(reg)  res
        );
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::tls::LibcReadTLS;
    use crate::tls::ReadTLSMem;
    use anyhow::{anyhow, bail};
    use libc;
    use sigsafe::{MemError, Ptr, ReadMem};
    use std::ffi::c_void;
    use std::mem::zeroed;
    use kindasafe::InitError;

    //todo do not copypaste this
    pub struct TestScopedInit {}
    impl TestScopedInit {
        pub fn new() -> Result<Self, InitError> {
            kindasafe::init()?;
            Ok(Self {})
        }
    }

    impl Drop for TestScopedInit {
        fn drop(&mut self) {
            assert_eq!(kindasafe::destroy(), Ok(()));
        }
    }
    //todo do not copypaste this
    pub fn serialize(f: impl FnOnce() -> Result<(), anyhow::Error>) -> Result<(), anyhow::Error> {
        let _shared = kindasafe::SERIALIZE_TESTS_LOCK.lock();
        match _shared {
            Ok(_) => f(),
            Err(_) => {
                bail!("could not serialize lock")
            }
        }
    }


    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_fs_16() -> Result<(), anyhow::Error> {
        serialize(|| {
            let _init = TestScopedInit::new()?;
            let ptr = super::fs_0x10();
            assert_ne!(0, ptr);
            kindasafe::u64(ptr).or_else(|err| Err(anyhow!("read mem error {err:?}")))?;
            Ok(())
        })
    }

    struct UnsafeMem;

    impl ReadMem for UnsafeMem {
        fn read_u64(at: Ptr) -> Result<Ptr, MemError> {
            let insnp = ((at as usize) + 0) as *const u64;
            let insn = unsafe { std::ptr::read_unaligned(insnp) };
            Ok(insn)
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_tls_kindasafe() -> Result<(), anyhow::Error> {
        test_tls::<ReadTLSMem<super::KindaSafeMem>>()
    }
    #[test]
    fn test_tls_unsafe() -> Result<(), anyhow::Error> {
        test_tls::<ReadTLSMem<UnsafeMem>>()
    }

    #[test]
    fn test_tls_libc() -> Result<(), anyhow::Error> {
        test_tls::<LibcReadTLS>()
    }

    fn test_tls<T: sigsafe::ReadTLS>() -> Result<(), anyhow::Error> {
        serialize(|| unsafe {
            let _init = TestScopedInit::new()?;
            let mut k: libc::pthread_key_t = zeroed();
            let ok = libc::pthread_key_create(&mut k, None);
            assert_eq!(0, ok);
            libc::pthread_setspecific(k, 0xcafe as *const c_void);
            let res = T::read_tls(sigsafe::PthreadKey { v: k as u32 })
                .or_else(|_| Err(anyhow!("read mem error ")))?;
            assert_eq!(0xcafe, res);
            Ok(())
        })
    }
}
