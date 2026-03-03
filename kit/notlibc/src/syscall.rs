/// Raw inline-assembly Linux syscall helpers for x86-64.
///
/// Each function issues the `syscall` instruction with the given arguments and
/// returns the kernel's raw return value (negative → errno on error).
/// All are marked `unsafe`; callers are responsible for argument validity.

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[inline(always)]
pub(crate) unsafe fn syscall1(nr: usize, a0: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") nr => ret,
            in("rdi") a0,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags),
        );
    }
    ret
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[inline(always)]
pub(crate) unsafe fn syscall2(nr: usize, a0: usize, a1: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") nr => ret,
            in("rdi") a0,
            in("rsi") a1,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags),
        );
    }
    ret
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[inline(always)]
pub(crate) unsafe fn syscall3(nr: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") nr => ret,
            in("rdi") a0,
            in("rsi") a1,
            in("rdx") a2,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags),
        );
    }
    ret
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[inline(always)]
pub(crate) unsafe fn syscall4(nr: usize, a0: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") nr => ret,
            in("rdi") a0,
            in("rsi") a1,
            in("rdx") a2,
            in("r10") a3,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags),
        );
    }
    ret
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[inline(always)]
pub(crate) unsafe fn syscall6(
    nr: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") nr => ret,
            in("rdi") a0,
            in("rsi") a1,
            in("rdx") a2,
            in("r10") a3,
            in("r8")  a4,
            in("r9")  a5,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags),
        );
    }
    ret
}
