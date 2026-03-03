# Go 1.26 CPU Profiler Design Analysis (Linux)

This document is a deep-dive analysis of the Go runtime CPU profiler implementation,
focused on Linux/amd64. It is written as a reference for building a Python CPU profiler
using signal handlers.

Source: Go 1.26.0 (`go.googlesource.com/go`, tag `go1.26.0`)

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [OS APIs and Syscalls](#2-os-apis-and-syscalls)
3. [Signal Handler Chain](#3-signal-handler-chain)
4. [Async-Signal Safety](#4-async-signal-safety)
5. [Data Structures](#5-data-structures)
6. [Memory Management](#6-memory-management)
7. [Thread Lifecycle Integration](#7-thread-lifecycle-integration)
8. [Stack Unwinding](#8-stack-unwinding)
9. [Reader Pipeline](#9-reader-pipeline)
10. [Key Design Decisions and Lessons for Python Profiler](#10-key-design-decisions-and-lessons-for-python-profiler)

---

## 1. Architecture Overview

### Data Flow

```
                         KERNEL
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   setitimer(ITIMER_PROF)  │  timer_create(CLOCK_THREAD_CPUTIME_ID)
   (process-wide)          │  (per-thread, per-M)
        │                  │                  │
        └─────── SIGPROF ──┼── SIGPROF ───────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │  sigtramp (asm) │  sys_linux_amd64.s:347
                  │  C ABI → Go ABI │
                  └────────┬────────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │  sigtrampgo()   │  signal_unix.go:432
                  │  fetch g, set   │
                  │  gsignal stack  │
                  └────────┬────────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │  sighandler()   │  signal_unix.go:646
                  │  validSIGPROF() │  os_linux.go:590
                  └────────┬────────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │  sigprof()      │  proc.go:5748
                  │  unwind stack   │
                  │  max 64 frames  │
                  └────────┬────────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │  cpuprof.add()  │  cpuprof.go:106
                  │  CAS spinlock   │
                  └────────┬────────┘
                           │
                           ▼
              ┌──────────────────────────┐
              │  profBuf.write()          │  profbuf.go:307
              │  lock-free ring buffer    │
              │  1 MiB data + 16K tags   │
              └────────────┬─────────────┘
                           │
                    ┌──────┴──────┐
                    │  futex wait │
                    └──────┬──────┘
                           │
                           ▼
              ┌──────────────────────────┐
              │  profileWriter()          │  pprof/pprof.go:922
              │  reader goroutine         │
              │  blocking profBuf.read()  │
              └────────────┬─────────────┘
                           │
                           ▼
              ┌──────────────────────────┐
              │  profileBuilder           │  pprof/proto.go
              │  profMap deduplication    │  pprof/map.go
              │  gzip + protobuf output  │
              └──────────────────────────┘
```

### Two-Tier Timer Design

Go uses **two concurrent timer mechanisms** on Linux:

| Timer | Syscall | Clock | Scope | Signal Code |
|-------|---------|-------|-------|-------------|
| Process-wide | `setitimer(ITIMER_PROF)` | Process CPU time | All threads | `_SI_KERNEL` (0x80) |
| Per-thread | `timer_create(CLOCK_THREAD_CPUTIME_ID)` | Thread CPU time | One thread via `SIGEV_THREAD_ID` | `_SI_TIMER` (-2) |

**Why both?** The process-wide timer catches threads that don't have per-thread timers
(non-Go threads, newly created threads). The per-thread timer provides more accurate
attribution. `validSIGPROF()` prevents double-counting by checking `si_code`.

### Component Summary

| Component | File | Lines | Role |
|-----------|------|-------|------|
| `cpuProfile` | `cpuprof.go` | 258 | Profile control, signal-safe write |
| `profBuf` | `profbuf.go` | 583 | Lock-free ring buffer |
| `sighandler` | `signal_unix.go` | 1472 | Signal dispatch |
| `validSIGPROF` | `os_linux.go` | ~50 | Double-counting prevention |
| `setThreadCPUProfiler` | `os_linux.go` | ~70 | Per-thread timer management |
| `sigprof` | `proc.go` | ~120 | Stack capture in signal context |
| `setcpuprofilerate` | `proc.go` | ~35 | Rate change coordination |
| `sigtramp` | `sys_linux_amd64.s` | ~25 | Assembly trampoline |
| `profileWriter` | `pprof/pprof.go` | ~25 | Reader goroutine |
| `profileBuilder` | `pprof/proto.go` | 769 | Protobuf output |

---

## 2. OS APIs and Syscalls

### 2.1 Signal Registration

**`setsig()`** — `os_linux.go:477`

Registers the Go signal handler for a given signal number:

```go
func setsig(i uint32, fn uintptr) {
    var sa sigactiont
    sa.sa_flags = _SA_SIGINFO | _SA_ONSTACK | _SA_RESTORER | _SA_RESTART
    sigfillset(&sa.sa_mask)  // Block ALL signals during handler execution
    if GOARCH == "386" || GOARCH == "amd64" {
        sa.sa_restorer = abi.FuncPCABI0(sigreturn__sigaction)
    }
    if fn == abi.FuncPCABIInternal(sighandler) {
        if iscgo {
            fn = abi.FuncPCABI0(cgoSigtramp)  // CGO-aware trampoline
        } else {
            fn = abi.FuncPCABI0(sigtramp)      // Plain Go trampoline
        }
    }
    sa.sa_handler = fn
    sigaction(i, &sa, nil)  // → rt_sigaction syscall
}
```

**Key flags:**
- `SA_SIGINFO` — provides `siginfo_t` and `ucontext_t` (needed for `si_code`, PC, SP)
- `SA_ONSTACK` — use alternate signal stack (gsignal's 32KB stack)
- `SA_RESTART` — restart interrupted syscalls automatically
- `SA_RESTORER` — required on x86_64; kernel uses `sa_restorer` for sigreturn
- `sigfillset(&sa.sa_mask)` — **all signals blocked** during handler execution

**Underlying syscall:** `rt_sigaction()` — implemented in assembly (`sys_linux_amd64.s`).

### 2.2 Timer Setup

#### Process-Wide Timer

**`setProcessCPUProfilerTimer()`** — `signal_unix.go:280`

```go
func setProcessCPUProfilerTimer(hz int32) {
    if hz != 0 {
        // Enable Go signal handler for SIGPROF
        if atomic.Cas(&handlingSig[_SIGPROF], 0, 1) {
            h := getsig(_SIGPROF)
            if h == _SIG_DFL { h = _SIG_IGN }  // Prevent crash from pending signals
            atomic.Storeuintptr(&fwdSig[_SIGPROF], h)
            setsig(_SIGPROF, abi.FuncPCABIInternal(sighandler))
        }
        var it itimerval
        it.it_interval.set_usec(1000000 / hz)  // Period in microseconds
        it.it_value = it.it_interval
        setitimer(_ITIMER_PROF, &it, nil)
    } else {
        setitimer(_ITIMER_PROF, &itimerval{}, nil)  // Stop timer
        // Restore previous signal handler if needed
    }
}
```

`ITIMER_PROF` measures **CPU time consumed by the process** (user + system). The kernel
delivers SIGPROF to an arbitrary thread when the timer expires.

#### Per-Thread Timer

**`setThreadCPUProfiler()`** — `os_linux.go:637`

```go
func setThreadCPUProfiler(hz int32) {
    mp := getg().m
    mp.profilehz = hz

    // Destroy existing timer if any
    if mp.profileTimerValid.Load() {
        timer_delete(mp.profileTimer)
        mp.profileTimerValid.Store(false)
    }
    if hz == 0 { return }

    // Randomized initial delay to avoid thundering herd / sampling bias
    spec := new(itimerspec)
    spec.it_value.setNsec(1 + int64(cheaprandn(uint32(1e9/hz))))
    spec.it_interval.setNsec(1e9 / int64(hz))

    var timerid int32
    var sevp sigevent
    sevp.notify = _SIGEV_THREAD_ID       // Target specific thread
    sevp.signo = _SIGPROF                 // Signal number
    sevp.sigev_notify_thread_id = int32(mp.procid)  // Linux TID
    ret := timer_create(_CLOCK_THREAD_CPUTIME_ID, &sevp, &timerid)
    if ret != 0 { return }  // Fallback to process-wide timer

    timer_settime(timerid, 0, spec, nil)
    mp.profileTimer = timerid
    mp.profileTimerValid.Store(true)
}
```

**`CLOCK_THREAD_CPUTIME_ID`** measures CPU time for a specific thread only. Combined
with `SIGEV_THREAD_ID`, the signal is delivered to exactly the thread that consumed
the CPU time.

**Randomized initial delay:** `cheaprandn(uint32(1e9/hz))` ensures that threads
created at the same time don't all fire their first sample simultaneously. This
also removes bias against short-lived threads: a thread that runs for 1/10th of a
period has a 10% chance of being sampled (expected value 0.1 samples).

#### Linux-Specific Timer Syscalls

```go
// os_linux.go:429-439
func setitimer(mode int32, new, old *itimerval)
func timer_create(clockid int32, sevp *sigevent, timerid *int32) int32
func timer_settime(timerid int32, flags int32, new, old *itimerspec) int32
func timer_delete(timerid int32) int32
```

### 2.3 Double-Counting Prevention

**`validSIGPROF()`** — `os_linux.go:590`

When both timers are active, a thread may receive SIGPROF from both sources. This
function uses `si_code` from `siginfo_t` to determine which source generated the signal:

```go
func validSIGPROF(mp *m, c *sigctxt) bool {
    code := int32(c.sigcode())
    setitimer := code == _SI_KERNEL     // 0x80: from setitimer
    timer_create := code == _SI_TIMER   // -2: from timer_create

    if !(setitimer || timer_create) {
        return true  // Unknown source, accept it
    }
    if mp == nil {
        return setitimer  // Non-Go thread: only accept process-wide
    }
    if mp.profileTimerValid.Load() {
        return timer_create  // Has per-thread timer: prefer it
    }
    return setitimer  // No per-thread timer: use process-wide
}
```

**Decision Matrix:**

| Thread Type | Has Per-Thread Timer | Signal Source | Accept? | Reason |
|-------------|---------------------|--------------|---------|--------|
| Non-Go (mp=nil) | — | setitimer | Yes | Only reliable source |
| Non-Go (mp=nil) | — | timer_create | No | Would double-count |
| Go thread | Yes | setitimer | No | Per-thread is more accurate |
| Go thread | Yes | timer_create | Yes | Preferred source |
| Go thread | No | setitimer | Yes | Only available source |
| Go thread | No | timer_create | No | Shouldn't happen |

### 2.4 Other Relevant Syscalls

| Syscall | Go wrapper | Usage |
|---------|-----------|-------|
| `rt_sigaction` | `sysSigaction()` | Register signal handlers |
| `sigaltstack` | `sigaltstack()` | Query/set alternate signal stack |
| `clone` | `clone()` | Create OS threads (via `newosproc()`) |
| `gettid` | `gettid()` | Get Linux thread ID (for timer_create) |
| `tgkill` | `tgkill()` | Send signal to specific thread |
| `futex` | via `note` | Block/wake reader goroutine |
| `sched_yield` | `osyield()` | Yield CPU in CAS spinlock |
| `sigprocmask` | `sigprocmask()` | Block signals during clone |

---

## 3. Signal Handler Chain

### 3.1 Assembly Entry — `sigtramp`

**`sys_linux_amd64.s:347-369`**

```asm
TEXT runtime·sigtramp(SB),NOSPLIT|TOPFRAME|NOFRAME,$0
    // Save all host registers for C → Go ABI transition
    PUSH_REGS_HOST_TO_ABI0()

    // Load Go runtime state
    get_tls(R12)              // Thread-local storage → R12
    MOVQ    g(R12), R14       // Current goroutine → R14 (Go ABI register)
    PXOR    X15, X15          // Clear X15 (Go ABI requirement)

    ADJSP   $24               // Reserve spill slots

    // Move C ABI args to Go ABI registers
    MOVQ    DI, AX            // sig number
    MOVQ    SI, BX            // siginfo_t*
    MOVQ    DX, CX            // ucontext_t*
    CALL    ·sigtrampgo<ABIInternal>(SB)

    ADJSP   $-24
    POP_REGS_HOST_TO_ABI0()
    RET
```

This is a thin shim that transitions from the C calling convention (kernel delivers
signal using C ABI) to Go's internal calling convention. The `NOSPLIT` flag means
no stack growth check — critical for signal handlers. `NOFRAME` means no Go frame
pointer setup.

For CGO programs, `cgoSigtramp` (`sys_linux_amd64.s:398`) is used instead. It checks
if we're in a CGO call and can collect C stack frames before falling through to
`sigtramp`.

### 3.2 `sigtrampgo()` — Go Signal Entry

**`signal_unix.go:432-501`**

```go
func sigtrampgo(sig uint32, info *siginfo, ctx unsafe.Pointer) {
    // 1. Check if signal should be forwarded to a non-Go handler
    if sigfwdgo(sig, info, ctx) { return }

    c := &sigctxt{info, ctx}
    gp := sigFetchG(c)
    setg(gp)

    // 2. Handle non-Go threads (gp == nil)
    if gp == nil || (gp.m != nil && gp.m.isExtraInC) {
        if sig == _SIGPROF {
            if validSIGPROF(nil, c) {
                sigprofNonGoPC(c.sigpc())  // Record non-Go thread sample
            }
            return
        }
        badsignal(uintptr(sig), c)
        return
    }

    // 3. Switch to gsignal stack (dedicated 32KB signal stack)
    setg(gp.m.gsignal)
    var gsignalStack gsignalStack
    setStack := adjustSignalStack(sig, gp.m, &gsignalStack)

    // 4. Dispatch to sighandler
    sighandler(sig, info, ctx, gp)

    // 5. Restore original g
    setg(gp)
    if setStack { restoreGsignalStack(&gsignalStack) }
}
```

Key operations:
- **Signal forwarding**: `sigfwdgo()` checks if a non-Go handler was installed before Go and forwards if appropriate
- **Non-Go thread handling**: If `gp == nil`, the signal arrived on a thread Go doesn't own. For SIGPROF, it's recorded via `sigprofNonGoPC()` → `cpuprof.addNonGo()`
- **Stack switch**: `setg(gp.m.gsignal)` switches to the dedicated signal-handling goroutine, which has its own 32KB stack

### 3.3 `sighandler()` — Signal Dispatch

**`signal_unix.go:646-672`** (SIGPROF path only)

```go
func sighandler(sig uint32, info *siginfo, ctxt unsafe.Pointer, gp *g) {
    gsignal := getg()
    mp := gsignal.m
    c := &sigctxt{info, ctxt}

    // Detect delayed signals from cgo TSAN
    delayedSignal := *cgo_yield != nil && mp != nil && gsignal.stack == mp.g0.stack

    if sig == _SIGPROF {
        if !delayedSignal && validSIGPROF(mp, c) {
            sigprof(c.sigpc(), c.sigsp(), c.siglr(), gp, mp)
        }
        return
    }
    // ... other signal handling ...
}
```

The `sigpc()`, `sigsp()`, `siglr()` methods extract the interrupted program counter,
stack pointer, and link register from the `ucontext_t` structure. These are used to
start the stack unwinding.

### 3.4 `sigprof()` — Stack Capture

**`proc.go:5748-5864`**

This is the core profiling function, called from the signal handler. Full code path:

```go
func sigprof(pc, sp, lr uintptr, gp *g, mp *m) {
    // Guard: profiling enabled?
    if prof.hz.Load() == 0 { return }
    if mp != nil && mp.profilehz == 0 { return }

    // Guard: MIPS/ARM atomic64 deadlock prevention
    // (64-bit atomics on 32-bit archs use spinlocks that would deadlock)
    if GOARCH == "mips" || GOARCH == "mipsle" || GOARCH == "arm" {
        if f := findfunc(pc); f.valid() {
            if stringslite.HasPrefix(funcname(f), "internal/runtime/atomic") {
                cpuprof.lostAtomic++
                return
            }
        }
    }

    // Trap: detect if signal handler accidentally allocates
    getg().m.mallocing++

    var u unwinder
    var stk [maxCPUProfStack]uintptr  // maxCPUProfStack = 64
    n := 0

    // Determine stack source and initialize unwinder
    if mp.ncgo > 0 && mp.curg != nil && mp.curg.syscallpc != 0 {
        // CGO call: collect C frames from mp.cgoCallers, then Go stack
        // from mp.curg.syscallpc/sp
        // ...
        u.initAt(mp.curg.syscallpc, mp.curg.syscallsp, 0, mp.curg, unwindSilentErrors)
    } else if mp != nil && mp.vdsoSP != 0 {
        // VDSO call (e.g., nanotime1): use stored PC/SP
        u.initAt(mp.vdsoPC, mp.vdsoSP, 0, gp, unwindSilentErrors|unwindJumpStack)
    } else {
        // Normal Go code: use interrupted PC/SP directly
        u.initAt(pc, sp, lr, gp, unwindSilentErrors|unwindTrap|unwindJumpStack)
    }
    n += tracebackPCs(&u, 0, stk[n:])

    // Fallback: if unwinding fails, use synthetic frames
    if n <= 0 {
        n = 2
        if inVDSOPage(pc) {
            stk[0] = abi.FuncPCABIInternal(_VDSO) + sys.PCQuantum
        } else if pc > firstmoduledata.etext {
            stk[0] = abi.FuncPCABIInternal(_ExternalCode) + sys.PCQuantum
        } else {
            stk[0] = pc
        }
        stk[1] = mp.preemptoff != "" ?
            abi.FuncPCABIInternal(_GC) :
            abi.FuncPCABIInternal(_System)
    }

    // Write sample to profile buffer
    if prof.hz.Load() != 0 {
        var tagPtr *unsafe.Pointer
        if gp != nil && gp.m != nil && gp.m.curg != nil {
            tagPtr = &gp.m.curg.labels  // Goroutine labels
        }
        cpuprof.add(tagPtr, stk[:n])
    }

    getg().m.mallocing--
}
```

---

## 4. Async-Signal Safety

### 4.1 Constraints

Signal handlers in Go must obey strict constraints because they can interrupt the
program at **any** point, including while holding locks or during GC:

| Constraint | How Enforced | Why |
|-----------|-------------|-----|
| No memory allocation | `mallocing++` trap | malloc holds locks that may already be held |
| No OS mutexes | CAS spinlock only | Would deadlock if signal interrupts lock holder |
| No write barriers | `//go:nowritebarrierrec` | GC may be in progress, write barrier code is complex |
| Limited stack usage | 32KB gsignal stack | Signal stack is pre-allocated and fixed |
| No goroutine ops | No `schedule()`, `gopark()` | Scheduler state may be inconsistent |
| Only async-signal-safe syscalls | Careful syscall selection | POSIX requirement |

### 4.2 CAS Spinlock (`prof.signalLock`)

**`proc.go:5728-5734`**

```go
var prof struct {
    signalLock atomic.Uint32
    hz         atomic.Int32
}
```

Used in three places:
1. **`cpuprof.add()`** (cpuprof.go:108) — signal handler writing to profBuf
2. **`cpuprof.addNonGo()`** (cpuprof.go:145) — non-Go thread writing to extra[]
3. **`setcpuprofilerate()`** (proc.go:5884) — changing profiling rate

```go
// Acquire
for !prof.signalLock.CompareAndSwap(0, 1) {
    osyield()  // sched_yield — yield CPU instead of spinning
}
// ... critical section ...
// Release
prof.signalLock.Store(0)
```

**Why not a mutex?** A mutex uses `futex(FUTEX_WAIT)` which blocks the thread. If the
signal handler is called while the main code holds the mutex, the signal handler would
block forever (deadlock). CAS + yield never blocks — it just retries.

**Why `osyield()` instead of spinning?** The comment notes this is debatable
(see go.dev/issue/52672). `osyield()` calls `sched_yield` which is technically not
in the POSIX async-signal-safe list, but works on Linux.

### 4.3 Non-Go Thread Handling

Non-Go threads (C threads in CGO programs) don't have Go runtime state (no `g`, no `m`).
They can't call `profBuf.write()` because it needs runtime infrastructure.

Solution: **deferred processing** via `extra[1000]`.

**`cpuprof.addNonGo()`** (cpuprof.go:138):
```go
func (p *cpuProfile) addNonGo(stk []uintptr) {
    for !prof.signalLock.CompareAndSwap(0, 1) { osyield() }

    if cpuprof.numExtra+1+len(stk) < len(cpuprof.extra) {
        i := cpuprof.numExtra
        cpuprof.extra[i] = uintptr(1 + len(stk))  // Length marker
        copy(cpuprof.extra[i+1:], stk)
        cpuprof.numExtra += 1 + len(stk)
    } else {
        cpuprof.lostExtra++
    }

    prof.signalLock.Store(0)
}
```

The `extra[1000]` buffer stores frames in a packed format: `[length, pc1, pc2, ...]`.
When a Go thread next receives SIGPROF, `cpuprof.add()` calls `addExtra()` to drain
the buffer into the profBuf.

---

## 5. Data Structures

### 5.1 `profBuf` — Lock-Free Ring Buffer

**`profbuf.go:91-107`**

This is the most important data structure in the profiler. It enables communication
between the signal handler (writer) and the reader goroutine without locks.

```go
type profBuf struct {
    // Accessed atomically
    r, w         profAtomic     // Read/write positions (packed)
    overflow     atomic.Uint64  // Pending overflow count + generation
    overflowTime atomic.Uint64  // Time of first overflow

    eof          atomic.Uint32  // EOF flag

    // Immutable after creation
    hdrsize uintptr             // Fixed header size per record (1 for CPU prof)
    data    []uint64            // Circular data buffer (power-of-two sized)
    tags    []unsafe.Pointer    // Parallel circular tag buffer

    // Owned by reader only
    rNext       profIndex       // Next read position (not committed yet)
    overflowBuf []uint64        // Scratch buffer for overflow records
    wait        note            // Futex-based sleep/wake
}
```

#### profIndex Encoding

**`profbuf.go:112-118`**

Each read/write pointer (`r`, `w`) is a 64-bit value with packed fields:

```
Bit layout of profIndex (uint64):
┌──────────────────┬───┬───┬────────────────────────────────┐
│   tag count      │ E │ S │         data count             │
│   (30 bits)      │(1)│(1)│         (32 bits)              │
│   bits 34-63     │33 │32 │         bits 0-31              │
└──────────────────┴───┴───┴────────────────────────────────┘

S = profReaderSleeping (bit 32): reader is blocked on futex
E = profWriteExtra (bit 33): overflow or EOF pending
```

- **data count** (bits 0-31): Total number of uint64 words written, mod 2^32
- **tag count** (bits 34-63): Total number of tags written, mod 2^30
- **Flags** (bits 32-33): Only used in `w`, unused in `r`

Both counts are monotonically increasing. The actual buffer offset is
`count % len(buffer)`. Because buffer lengths are powers of two, this works correctly
across wraparound.

```go
func (x profIndex) dataCount() uint32 { return uint32(x) }
func (x profIndex) tagCount() uint32  { return uint32(x >> 34) }

// Subtract two counts, handling wraparound
func countSub(x, y uint32) int {
    return int(int32(x-y) << 2 >> 2)  // sign-extend from 30-bit or 32-bit
}
```

#### Record Format

Each sample in the data buffer:

```
┌─────────┬───────────┬──────────────────┬──────────────────────┐
│ length  │ timestamp │ header[hdrsize]  │ stack[0..nstk-1]     │
│ (1 word)│ (1 word)  │ (1 word for CPU) │ (variable, max 64)   │
└─────────┴───────────┴──────────────────┴──────────────────────┘
  length = 2 + hdrsize + nstk
  For CPU profiling: header[0] = sample count (always 1 from signal handler)
```

Corresponding tag in the parallel `tags` buffer: `unsafe.Pointer` to goroutine labels.

#### Write Path

**`profbuf.go:307-411`**

```
write(tagPtr, now, hdr, stk):
  1. If pending overflow AND room for 2 records:
     → Flush overflow record first, then write new record
  2. If pending overflow AND no room for 2:
     → incrementOverflow(now), wakeupExtra(), return
  3. If no room for 1 record:
     → incrementOverflow(now), wakeupExtra(), return
  4. Write tag (NO write barrier — see comment about GC safety)
  5. Write data record:
     a. If record doesn't fit at end of buffer:
        → Write 0 marker (rewind sentinel), skip to beginning
     b. Write [length, timestamp, header..., stack...]
  6. CAS loop to commit new w:
     a. Compute new w with updated counts
     b. If buffer <50% full: carry over profReaderSleeping flag
     c. If buffer ≥50% full AND reader sleeping: wake via notewakeup()
```

**No write barriers on tags:** The comment at line 337-352 explains why this is safe.
The tag is `&gp.labels` — since the goroutine `gp` is the one that was interrupted,
its labels are stable. The GC will keep `gp.labels` alive because `gp` is reachable.

#### Read Path

**`profbuf.go:455-583`**

```
read(mode):
  1. Commit previous read: clear consumed tags to nil, advance r
  2. Load w, compute available data
  3. If no data:
     a. If overflow pending → synthesize overflow record, return
     b. If EOF → return eof=true
     c. If profWriteExtra flag set → clear it, retry
     d. If blocking mode → CAS profReaderSleeping flag, futex wait
     e. If non-blocking → return empty
  4. Handle wraparound: if data[0] == 0, skip to buffer start
  5. Count complete records in available data
  6. Update rNext (not yet committed — committed on next read() call)
  7. Return slice of data buffer and tags
```

**Important:** `read()` returns a slice of the buffer itself, not a copy. The caller
must finish with the data before calling `read()` again (which commits the read
and allows the writer to reuse that space).

#### Overflow Tracking

**`profbuf.go:155-207`**

The `overflow` field uses a generation counter to avoid ABA problems:

```
overflow (uint64):
┌──────────────────────────────┬──────────────────────────────┐
│      generation (32 bits)    │    lost count (32 bits)      │
│      high 32 bits            │    low 32 bits               │
└──────────────────────────────┴──────────────────────────────┘
```

- `incrementOverflow(now)`: Called by writer when buffer is full.
  Sets `overflowTime` on 0→N transition, then increments count.
- `takeOverflow()`: Called by reader to consume. Uses CAS to atomically
  clear count while incrementing generation.
- Generation counter prevents: reader clears overflow, writer starts new overflow,
  reader's stale CAS succeeds on the new overflow.
- `overflowTime` stores the timestamp of the first lost sample (written before
  count transitions from 0, so always valid when count > 0).

#### Buffer Sizing

**`cpuprof.go:21-34`**

```go
const (
    maxCPUProfStack = 64
    profBufWordCount = 1 << 17  // 131,072 uint64 words = 1 MiB
    profBufTagCount  = 1 << 14  // 16,384 tag pointers
)
```

At 100 Hz with 64-frame stacks: each sample ≈ 2+1+64 = 67 words.
131,072 / 67 ≈ 1,956 samples = 19.5 seconds of single-thread profiling.

The tag buffer at 16,384 entries = 163 seconds at 100 Hz.

`newProfBuf()` rounds sizes up to the next power of two for efficient modular
arithmetic.

### 5.2 `cpuProfile` Struct

**`cpuprof.go:37-57`**

```go
type cpuProfile struct {
    lock mutex           // Protects on, log fields (NOT used in signal handler)
    on   bool            // Profiling is active
    log  *profBuf        // Ring buffer for samples

    extra      [1000]uintptr  // Deferred non-Go thread stacks
    numExtra   int            // Current fill level of extra
    lostExtra  uint64         // Frames lost because extra is full
    lostAtomic uint64         // Frames lost due to MIPS/ARM atomic64 deadlock
}
```

**`extra[1000]`** packed format: `[length1, pc1a, pc1b, length2, pc2a, pc2b, ...]`
where `length_i = 1 + nframes_i`. Assuming 2-frame stacks (common for non-Go threads),
each entry is 3 words → ~333 pending non-Go samples. At 100 Hz, this drains every
~3 seconds when a Go thread gets SIGPROF.

### 5.3 `profMap` — Reader-Side Sample Deduplication

**`pprof/map.go`**

```go
type profMap struct {
    hash    map[uintptr]*profMapEntry  // Hash table
    all     *profMapEntry              // Linked list of all entries
    last    *profMapEntry              // Tail pointer
    free    []profMapEntry             // Pre-allocated entry pool (batch 128)
    freeStk []uintptr                  // Pre-allocated stack storage (batch 1024)
}

type profMapEntry struct {
    nextHash *profMapEntry    // Hash chain
    nextAll  *profMapEntry    // All-entries chain
    stk      []uintptr        // Stack PCs (slice into freeStk)
    tag      unsafe.Pointer   // Goroutine label pointer
    count    int64            // Aggregated sample count
}
```

`lookup(stk, tag)` hashes the stack + tag, finds or creates an entry, and returns
a pointer. The caller increments `count`. Entries and stacks are pre-allocated in
batches to reduce allocation overhead.

### 5.4 `profileBuilder` — Protobuf Output

**`pprof/proto.go:25-44`**

```go
type profileBuilder struct {
    start      time.Time
    end        time.Time
    havePeriod bool
    period     int64            // 1e9/hz nanoseconds

    m          profMap           // Sample deduplication
    w          io.Writer
    zw         *gzip.Writer     // Gzip compression layer
    pb         protobuf          // Raw protobuf encoder
    strings    []string          // String table (deduplication)
    stringMap  map[string]int
    locs       map[uintptr]locInfo  // PC → location info cache
    funcs      map[string]int       // Function → ID cache
    mem        []memMap             // /proc/self/maps
    deck       pcDeck              // Inline frame tracking
}
```

Output format is pprof protobuf (gzip-compressed). The builder:
1. Reads chunks from profBuf via `readProfile()`
2. Parses records: `[length, time, count, pc1, pc2, ...]`
3. Deduplicates via profMap (aggregates count for identical stacks)
4. On `build()`: emits all unique samples, locations, functions, and string table

---

## 6. Memory Management

### 6.1 Allocation Timeline

```
Program start:
  per M (thread):
    mpreinit():  malg(32*1024)        → 32KB gsignal stack    [os_linux.go:387]

Profile start (SetCPUProfileRate):
    newProfBuf(1, 1<<17, 1<<14)       → profBuf struct         [cpuprof.go:86]
      make([]uint64, 1<<17)           → 1 MiB data buffer
      make([]unsafe.Pointer, 1<<14)   → 128 KB tag buffer
      make([]uint64, 4)               → 32 byte overflow buffer

Profile running (signal handler):
    NO ALLOCATIONS                     [enforced by mallocing++ trap]

Profile stop:
    cpuprof.log.close()               → sets EOF flag
    Reader drains remaining data
    cpuprof.log = nil                  → profBuf becomes GC-eligible
```

### 6.2 Signal Handler: Zero Allocation

The entire signal handler path from `sigtramp` through `sigprof` to `profBuf.write()`
allocates **zero bytes**. All storage is pre-allocated:

| Data | Storage | Allocated When |
|------|---------|---------------|
| Stack frames (max 64) | `stk [maxCPUProfStack]uintptr` on gsignal stack | Stack-local |
| Header | `hdr [1]uint64` on gsignal stack | Stack-local |
| Ring buffer | `profBuf.data[]` | Profile start |
| Tag pointers | `profBuf.tags[]` | Profile start |
| Non-Go stacks | `cpuProfile.extra[1000]` | Static (global) |
| Spinlock | `prof.signalLock` (uint32) | Static (global) |

The only "allocation" is `spec := new(itimerspec)` in `setThreadCPUProfiler()`, but
that runs from goroutine context, not from the signal handler.

---

## 7. Thread Lifecycle Integration

### 7.1 Thread Creation

```
newosproc(mp)                    [os_linux.go:170]
  │ clone(cloneFlags, stack, mp, mp.g0, mstart)
  │ (signals blocked during clone via sigprocmask)
  │
  ▼ (new OS thread)
mstart()                         [assembly entry]
  │
  ▼
mstart0()
  │
  ▼
mstart1()                        [proc.go:1904]
  │ asminit()
  │ minit()                      [os_linux.go:395]
  │   minitSignals()
  │   mp.procid = uint64(gettid())    ← Linux TID obtained here
  │
  │ schedule()
  │   findRunnable()
  │   execute(gp)                [proc.go:3370]
  │     hz := sched.profilehz
  │     if mp.profilehz != hz {
  │         setThreadCPUProfiler(hz)   ← Per-thread timer created here
  │     }
  │     gogo(&gp.sched)
```

**Key point:** The per-thread timer is not created during `minit()` but during the first
`execute()` call. This is when the thread first checks `sched.profilehz`. If profiling
is active, `setThreadCPUProfiler(hz)` creates the `CLOCK_THREAD_CPUTIME_ID` timer.

### 7.2 Rate Changes

**`setcpuprofilerate()`** — `proc.go:5868`

Changing the profiling rate is a delicate operation that must coordinate with signal
handlers to avoid deadlock:

```
setcpuprofilerate(hz):
  1. gp.m.locks++                          // Disable preemption
  2. setThreadCPUProfiler(0)               // Stop THIS thread's timer
     // Now safe to acquire signalLock — no SIGPROF can fire on this thread
  3. for !prof.signalLock.CompareAndSwap(0, 1) { osyield() }
  4. setProcessCPUProfiler(hz)             // Update process-wide timer
  5. prof.hz.Store(hz)                     // Publish new rate
  6. prof.signalLock.Store(0)
  7. lock(&sched.lock)
  8. sched.profilehz = hz                  // Update scheduler
  9. unlock(&sched.lock)
  10. if hz != 0 { setThreadCPUProfiler(hz) }  // Restart this thread's timer
  11. gp.m.locks--                          // Re-enable preemption
```

**Why step 2 before step 3?** If SIGPROF fires while `signalLock` is held, the signal
handler's `cpuprof.add()` will spin on the same lock → deadlock. By disabling the
per-thread timer first, we guarantee no SIGPROF arrives on this thread while we hold
the lock.

Other threads may still receive SIGPROF from the process-wide timer, but they'll try
to acquire `signalLock` and yield until we're done.

### 7.3 Thread Parking

When an M (OS thread) parks (no work to do), its per-thread timer **remains active**.
This is correct because `CLOCK_THREAD_CPUTIME_ID` only measures actual CPU time. A
parked thread consumes no CPU, so the timer doesn't fire. If the thread later unparks
and runs Go code, the timer resumes counting.

### 7.4 Thread Exit

```
mexit(osStack)                   [proc.go:1971]
  │ sigblock(true)               // Block all signals
  │ unminit()                    [os_linux.go:407]
  │   unminitSignals()
  │   mp.procid = 0
  │
  │ (timer implicitly destroyed when thread exits,
  │  or explicitly via setThreadCPUProfiler(0) during profile stop)
```

### 7.5 M Struct Profiling Fields

**`runtime2.go`** and **`os_linux.go:21-41`**

```go
type m struct {
    procid       uint64          // Linux TID (from gettid())
    gsignal      *g              // Signal-handling goroutine (32KB stack)
    curg         *g              // Currently running goroutine
    profilehz    int32           // Current profiling rate for this thread

    // Linux-specific (embedded mOS):
    profileTimer      int32      // POSIX timer ID
    profileTimerValid atomic.Bool // Timer validity flag (atomic for signal handler reads)
}
```

---

## 8. Stack Unwinding

### 8.1 Unwinder

**`traceback.go:96-117`**

```go
type unwinder struct {
    frame        stkframe    // Current physical stack frame
    g            guintptr    // Goroutine being unwound
    cgoCtxt      int         // Index into g.cgoCtxt
    calleeFuncID abi.FuncID  // For wrapper elision
    flags        unwindFlags
}
```

**Flags used in signal handler context:**
- `unwindSilentErrors` — Don't throw on unwind errors (common in signal context)
- `unwindTrap` — PC is from a trap, not a return address (signal interrupted mid-instruction)
- `unwindJumpStack` — Allow jumping from system stack to user stack

### 8.2 `tracebackPCs()`

**`traceback.go:621-654`**

```go
func tracebackPCs(u *unwinder, skip int, pcBuf []uintptr) int {
    n := 0
    for ; n < len(pcBuf) && u.valid(); u.next() {
        f := u.frame.fn
        for iu, uf := newInlineUnwinder(f, u.symPC()); uf.valid(); uf = iu.next(uf) {
            sf := iu.srcFunc(uf)
            if sf.funcID == abi.FuncIDWrapper && elideWrapperCalling(u.calleeFuncID) {
                continue  // Skip wrapper functions
            }
            if skip > 0 { skip--; continue }
            pcBuf[n] = uf.pc + 1  // Return address (PC + 1)
            n++
        }
    }
    return n
}
```

**Key details:**
- Maximum 64 frames (`maxCPUProfStack`)
- Handles **inlined functions** via `inlineUnwinder`
- Returns "return PCs" (actual PC + 1) for compatibility with tools
- Elides wrapper functions (e.g., `runtime.goexit`)

### 8.3 Special Unwinding Cases in `sigprof()`

| Case | Detection | Stack Source |
|------|-----------|-------------|
| CGO call | `mp.ncgo > 0 && mp.curg.syscallpc != 0` | C frames from `mp.cgoCallers`, Go frames from `syscallpc/sp` |
| VDSO call | `mp.vdsoSP != 0` | Go frames from `mp.vdsoPC/SP` |
| Normal Go | Default | Interrupted PC/SP from `ucontext_t` |
| Failed unwind | `n <= 0` | Synthetic: `_VDSO`/`_ExternalCode`/`_GC`/`_System` |

**VDSO handling:** Go's `nanotime()` calls into the kernel's VDSO page. When SIGPROF
fires during a VDSO call, the interrupted PC is in kernel memory. Go saves `vdsoPC`
and `vdsoSP` before entering the VDSO so the profiler can recover the Go stack.

---

## 9. Reader Pipeline

### 9.1 `profileWriter()` Goroutine

**`pprof/pprof.go:922-945`**

```go
func profileWriter(w io.Writer) {
    b := newProfileBuilder(w)
    for {
        data, tags, eof := readProfile()  // Blocks via futex
        b.addCPUData(data, tags)
        if eof { break }
    }
    b.build()        // Emit protobuf
    cpu.done <- true // Signal StopCPUProfile
}
```

### 9.2 `readProfile()`

**`cpuprof.go:243-258`**

```go
func runtime_pprof_readProfile() ([]uint64, []unsafe.Pointer, bool) {
    lock(&cpuprof.lock)
    log := cpuprof.log
    unlock(&cpuprof.lock)

    readMode := profBufBlocking
    if GOOS == "darwin" || GOOS == "ios" {
        readMode = profBufNonBlocking  // notes not async-signal-safe on Darwin
    }

    data, tags, eof := log.read(readMode)
    if len(data) == 0 && eof {
        lock(&cpuprof.lock)
        cpuprof.log = nil  // Allow GC of profBuf
        unlock(&cpuprof.lock)
    }
    return data, tags, eof
}
```

On Linux, the reader blocks via `notetsleepg(&b.wait, -1)` which uses `futex(FUTEX_WAIT)`.
The writer wakes it via `notewakeup(&b.wait)` → `futex(FUTEX_WAKE)` when the buffer
is ≥50% full, or on overflow/EOF.

### 9.3 `addCPUData()` — Record Parsing

**`pprof/proto.go:278-345`**

```
First record (header): [3, timestamp, hz]
  → b.period = 1e9 / hz nanoseconds

Sample records: [2+hdrsize+nstk, time, count, pc1, pc2, ..., pcN]
  → Deduplicate via b.m.lookup(stk, tag).count += count

Overflow records: [2+hdrsize+1, time, 0, lost_count]
  → count=0 means overflow, stk[0] is the lost count
  → Attributed to synthetic lostProfileEvent function
```

### 9.4 `build()` — Protobuf Output

**`pprof/proto.go:348-395`**

The builder iterates all deduplicated samples from `profMap.all` and emits:
- Sample values: `[count, count * period]` (sample count and CPU nanoseconds)
- Location IDs from symbolized stack PCs
- Labels from goroutine tags
- Function table, string table, memory mappings

Output is gzip-compressed protobuf matching the pprof format specification.

### 9.5 Goroutine Labels

Labels flow through the system as `unsafe.Pointer`:

```
User code:
  pprof.WithLabels(ctx, pprof.Labels("key", "value"))
    → context stores *labelMap
    → pprof.Do(ctx, ...) calls runtime_setProfLabel(labels)
    → gp.labels = labels pointer                       [proflabel.go]

Signal handler:
  sigprof() reads tagPtr = &gp.m.curg.labels
  cpuprof.add(tagPtr, stk) → profBuf.write(tagPtr, ...)
  Tag pointer stored in profBuf.tags[] WITHOUT write barrier

Reader:
  profBuf.read() returns tags[]
  profileBuilder uses tag pointer to recover label key-value pairs
```

---

## 10. Key Design Decisions and Lessons for Python Profiler

### 10.1 Two-Tier Timer System

**Decision:** Use both `setitimer(ITIMER_PROF)` and per-thread `timer_create()`.

**Rationale:** `setitimer` is simple but delivers to arbitrary threads, making attribution
inaccurate. `timer_create(CLOCK_THREAD_CPUTIME_ID)` targets specific threads but requires
tracking thread lifecycle. The combination covers all threads (including non-Go ones)
while preferring per-thread accuracy where available.

**For Python:** Python has a GIL, so process-wide `setitimer` may be sufficient. But
for free-threaded Python (PEP 703), per-thread timers would be valuable.

### 10.2 Lock-Free Ring Buffer

**Decision:** Single-writer single-reader lock-free ring buffer with power-of-two sizing.

**Rationale:** The writer (signal handler) cannot block or allocate. The reader (goroutine)
can block. A lock-free buffer with atomic CAS operations is the only safe option.

**For Python:** This pattern translates directly. The signal handler writes to a
pre-allocated ring buffer, a Python thread reads from it.

### 10.3 CAS Spinlock

**Decision:** `CompareAndSwap(0,1)` + `osyield()` instead of mutex.

**Rationale:** Mutex would deadlock if signal interrupts the lock holder. CAS + yield
is the only safe option for signal handler synchronization.

**For Python:** Use `atomic_compare_exchange_weak` + `sched_yield` in C extension code.

### 10.4 Pre-Allocation of All Resources

**Decision:** Allocate profBuf at profile start, never during signal handling.

**Rationale:** `malloc()` is not async-signal-safe (it holds internal locks). Any
allocation in the signal handler risks deadlock.

**For Python:** Pre-allocate the ring buffer, stack frame arrays, and all auxiliary
storage before enabling the signal handler.

### 10.5 Randomized Initial Delay

**Decision:** `cheaprandn(uint32(1e9/hz))` for per-thread timer initial value.

**Rationale:** Without randomization, threads created at the same time would all fire
their first sample simultaneously, creating bias. Randomization also ensures short-lived
threads have proportional representation.

**For Python:** Apply the same randomization to per-thread timer initial values.

### 10.6 Overflow Tracking with Generation Counter

**Decision:** `overflow` field uses high 32 bits as generation counter.

**Rationale:** Simple CAS on overflow count has ABA problem: reader clears count,
writer increments, reader's old CAS value matches. Generation counter makes every
state transition unique.

**For Python:** Use the same generation-counter pattern for overflow tracking.

### 10.7 Reader Decoupling via Goroutine

**Decision:** Separate reader goroutine that blocks on futex.

**Rationale:** The signal handler must return quickly. All I/O (protobuf encoding,
compression, file writing) happens in the reader goroutine. The ring buffer provides
back-pressure: if the reader can't keep up, samples are counted as overflow rather
than blocking the signal handler.

**For Python:** Use a dedicated reader thread that blocks on a futex/condition variable.
The signal handler should do minimal work: capture stack, write to ring buffer, return.

### 10.8 Write Barrier Avoidance

**Decision:** Store tag pointers without write barriers, relying on GC invariants.

**Rationale:** The GC may be in progress during the signal handler. Write barriers are
complex and may allocate or take locks. The comment in `profbuf.go:337-352` explains
why this is safe: the goroutine's labels are reachable via `gp`, and the signal handler
only reads stable data.

**For Python:** Python's GC is simpler (reference counting + cycle collector). The signal
handler should avoid touching Python objects. Store raw pointers/integers in the ring
buffer and resolve them to Python objects in the reader thread.

---

## Source Files Reference

All source files are copied to `go-profiler-src/` for quick access.

### Core Profiler

| File | Key Functions/Types |
|------|-------------------|
| `runtime/cpuprof.go` | `cpuProfile`, `SetCPUProfileRate()`, `add()`, `addNonGo()`, `addExtra()`, `readProfile()` |
| `runtime/profbuf.go` | `profBuf`, `profIndex`, `newProfBuf()`, `write()`, `read()`, `incrementOverflow()`, `takeOverflow()` |
| `runtime/signal_unix.go` | `sigtrampgo()`, `sighandler()`, `setProcessCPUProfilerTimer()` |
| `runtime/os_linux.go` | `setsig()`, `validSIGPROF()`, `setThreadCPUProfiler()`, `minit()`, `mpreinit()` |
| `runtime/proc.go` | `sigprof()`, `setcpuprofilerate()`, `execute()` (timer check at line 3370) |

### Supporting

| File | Key Contents |
|------|-------------|
| `runtime/runtime2.go` | `m` struct (profiling fields), `g` struct (labels field) |
| `runtime/traceback.go` | `unwinder`, `tracebackPCs()` |
| `runtime/proflabel.go` | `runtime_setProfLabel()`, `runtime_getProfLabel()` |
| `runtime/sys_linux_amd64.s` | `sigtramp`, `cgoSigtramp` assembly trampolines |
| `runtime/defs_linux_amd64.go` | `_SIGPROF`, `_SI_KERNEL`, `_SI_TIMER`, `_CLOCK_THREAD_CPUTIME_ID`, etc. |
| `runtime/signal_linux_amd64.go` | `sigctxt` methods: `sigpc()`, `sigsp()`, `siglr()`, `sigcode()` |

### Reader/Output

| File | Key Contents |
|------|-------------|
| `runtime/pprof/pprof.go` | `StartCPUProfile()`, `StopCPUProfile()`, `profileWriter()` |
| `runtime/pprof/proto.go` | `profileBuilder`, `addCPUData()`, `build()` |
| `runtime/pprof/map.go` | `profMap`, `profMapEntry`, `lookup()` |
| `runtime/pprof/label.go` | `WithLabels()`, `Labels()`, `Do()` |
| `runtime/pprof/protobuf.go` | Low-level protobuf encoding |
