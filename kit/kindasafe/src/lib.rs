#![cfg(target_arch = "x86_64")]
#![no_std]

#[derive(Debug, PartialEq)]
pub struct ReadMemError {
    pub signal: u64,
}

pub type Ptr = u64;

pub fn u64(at: Ptr) -> Result<Ptr, ReadMemError> {
    let res = arch::u64(at);
    if res.signal == 0 {
        Ok(res.value)
    } else {
        Err(ReadMemError { signal: res.signal })
    }
}

pub fn slice(buf: &mut [u8], at: Ptr) -> Result<(), ReadMemError> {
    let res = arch::slice(buf.as_ptr(), at, buf.len() as u64);
    if res.signal == 0 {
        Ok(())
    } else {
        Err(ReadMemError { signal: res.signal })
    }
}

pub fn str(buf: &mut [u8], at: Ptr) -> Result<&str, ReadMemError> {
    if at == 0 {
        return Ok("");
    }
    let res = arch::slice(buf.as_ptr(), at, buf.len() as u64);
    if res.signal != 0 {
        return Err(ReadMemError { signal: res.signal });
    }
    for i in 0..buf.len() {
        if buf[i] == 0 {
            let v = &buf[..i];
            return match core::str::from_utf8(v) {
                Ok(v) => Ok(v),
                Err(_) => Err(ReadMemError { signal: 228 }), //todo
            };
        }
    }
    Err(ReadMemError { signal: 229 }) //todo
}

#[cfg(target_arch = "x86_64")]
pub fn fs_0x10() -> Result<Ptr, ReadMemError> {
    let res = arch::fs_0x10();
    if res.signal == 0 {
        return Ok(res.value);
    }
    Err(ReadMemError { signal: res.signal })
}

pub fn crash_points() -> CrashPoints {
    arch::crash_points()
}

#[derive(Copy, Clone)]
pub struct CrashPoint {
    pub pc: usize,
    pub signal_reg: usize,
    pub skip: usize,
}
#[derive(Copy, Clone)]
pub struct CrashPoints {
    pub crash_points: [CrashPoint; 3],
}

// todo arm64
#[cfg(target_arch = "x86_64")]
pub mod arch {

    #[repr(C)]
    pub struct U64Res {
        pub value: u64,
        pub signal: u64,
    }

    #[unsafe(naked)]
    pub extern "sysv64" fn u64(_at: u64) -> U64Res {
        core::arch::naked_asm!(
            "mov rax, [rdi]", // 00010000 	48 8B 07 	mov 	rax, qword ptr [rdi]
            "xor edx, edx",   // 00010003 	31 D2 	xor 	edx, edx
            "ret",            // 00010005 	C3 	ret
        )
    }

    #[repr(C)]
    pub struct VecResult {
        pub signal: u64,
    }

    #[unsafe(naked)]
    pub extern "sysv64" fn slice(
        _dst: *const u8, // rdi
        _src: u64,       // rsi
        _n: u64,         // rdx
    ) -> VecResult {
        core::arch::naked_asm!(
            "mov ecx, edx", // 00010000 	89 D1 	mov 	ecx, edx
            "rep movsb",    // 00010002 	F3 A4 	rep movsb 	byte ptr [rdi], byte ptr [rsi]
            "xor eax, eax", // 00010004 	31 C0 	xor 	eax, eax
            "ret",          // 00010006 	C3 	ret
        )
    }

    #[unsafe(naked)]
    pub extern "sysv64" fn fs_0x10() -> U64Res {
        core::arch::naked_asm!(
            "mov    rax, qword ptr fs:0x10", // 00010000 	48 64 A1 10 00 00 00 00 00 00 00 	movabs 	eax, dword ptr fs:[0x10]
            "xor    edx, edx",               // 0001000B 	31 D2 	xor 	edx, edx
            "ret",                           // 0001000D 	C3 	ret
        )
    }

    const REG_RAX: usize = 13;
    const REG_RDX: usize = 12;

    pub fn crash_points() -> crate::CrashPoints {
        crate::CrashPoints {
            crash_points: [
                crate::CrashPoint {
                    pc: u64 as *const () as usize,
                    signal_reg: REG_RDX,
                    skip: 5,
                },
                crate::CrashPoint {
                    pc: slice as *const () as usize + 2, // +2 for 89 D1 	mov 	ecx, edx
                    signal_reg: REG_RAX,
                    skip: 4,
                },
                crate::CrashPoint {
                    pc: fs_0x10 as *const () as usize,
                    signal_reg: REG_RDX,
                    skip: 13,
                },
            ],
        }
    }
}
