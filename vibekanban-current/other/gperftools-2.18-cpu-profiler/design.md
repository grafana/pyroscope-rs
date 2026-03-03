# gperftools CPU Profiler Design Analysis (v2.18, Linux)

Source: https://github.com/gperftools/gperftools, tag `gperftools-2.18` (January 2026).

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Component Breakdown](#2-component-breakdown)
3. [OS APIs Used](#3-os-apis-used)
4. [Signal Handling and Async-Signal Safety](#4-signal-handling-and-async-signal-safety)
5. [Data Structures](#5-data-structures)
6. [Memory Allocation and Management](#6-memory-allocation-and-management)
7. [Stack Unwinding](#7-stack-unwinding)
8. [Locking Protocol](#8-locking-protocol)
9. [Output Format](#9-output-format)
10. [Per-Thread Timer Mode (Linux-specific)](#10-per-thread-timer-mode-linux-specific)
11. [Design Tradeoffs and Lessons](#11-design-tradeoffs-and-lessons)

---

## 1. Architecture Overview

The gperftools CPU profiler is a **sampling profiler** that uses OS timer signals to periodically interrupt program execution and capture stack traces. It consists of four layers:

```
+---------------------------+
|  Public C API             |  profiler.h
|  (ProfilerStart/Stop)     |
+---------------------------+
|  CpuProfiler singleton    |  profiler.cc
|  (orchestration)          |
+---------------------------+
|  ProfileHandler singleton |  profile-handler.cc
|  (timer + signal mgmt)    |
+---------------------------+
|  ProfileData              |  profiledata.cc
|  (hash table + I/O)       |
+---------------------------+
```

**Data flow during profiling:**

1. `ProfileHandler` sets up a periodic timer (ITIMER_PROF or per-thread POSIX timer)
2. On each timer tick, the kernel delivers SIGPROF (or configured signal) to the process/thread
3. `ProfileHandler::SignalHandler()` acquires `signal_lock_` and iterates registered callbacks
4. `CpuProfiler::prof_handler()` callback extracts PC from `ucontext_t`, captures stack trace, calls `ProfileData::Add()`
5. `ProfileData::Add()` inserts the stack trace into a fixed-size hash table
6. When hash table entries are evicted, they're written to a write buffer, eventually flushed to disk via `write(2)`

## 2. Component Breakdown

### 2.1 `profiler.cc` — CpuProfiler

The **orchestration layer**. `CpuProfiler` is a process-wide singleton (`CpuProfiler::instance_`), created as a global static object. Its constructor checks for `CPUPROFILE` environment variable to auto-start profiling.

Key responsibilities:
- Owns a `SpinLock lock_` for serializing control operations (Start/Stop/Flush)
- Owns a `ProfileData collector_` instance
- Registers/unregisters its `prof_handler` callback with `ProfileHandler`
- The `prof_handler` callback runs in signal context — it captures the stack and calls `collector_.Add()`

**Critical design pattern**: Before any control operation on `collector_` (Stop, FlushTable), the signal handler is **unregistered first**. `ProfileHandlerUnregisterCallback()` guarantees that the currently-running callback completes and no future invocations occur. Only then does the control path touch `collector_` state. This eliminates the need for locks in the signal-handler hot path.

### 2.2 `profile-handler.cc` — ProfileHandler

Manages the **timer and signal handler**. Also a singleton (lazily initialized via `TrivialOnce`).

Key responsibilities:
- Installs `SignalHandler` for `SIGPROF` (or `SIGALRM` if `CPUPROFILE_REALTIME` is set)
- Maintains a `std::list<ProfileHandlerToken*> callbacks_` of registered profiler callbacks
- Manages timer start/stop via `setitimer()` or per-thread POSIX `timer_create()`
- Uses a **two-lock protocol**: `control_lock_` for serializing registrations, `signal_lock_` for protecting callback list access from within the signal handler

### 2.3 `profiledata.cc` — ProfileData

The **data accumulation engine**. Stores stack traces in a set-associative hash table, periodically flushing to an output file.

Key design: `Add()` is explicitly designed to be **safe to call from async signals** (though it is not re-entrant). All other methods (`Start`, `Stop`, `FlushTable`) require external synchronization (provided by `CpuProfiler`).

### 2.4 Stack Unwinding (`stacktrace.cc`, `stacktrace_generic_fp-inl.h`)

Multiple unwinding backends are compiled in; one is selected at startup:
- Frame-pointer based (preferred on x86-64, aarch64, riscv)
- libunwind
- libgcc (`_Unwind_Backtrace`)
- `backtrace()` (glibc)

Selection is via `TCMALLOC_STACKTRACE_METHOD` env var or compile-time defaults.

### 2.5 GetPC (`getpc.h`, `getpc-inl.h`)

Extracts the interrupted program counter from `ucontext_t`. Uses C++ template SFINAE to auto-detect the correct `ucontext_t` field for each OS/architecture combination at compile time. On Linux/x86_64 this resolves to `uc->uc_mcontext.gregs[REG_RIP]`.

## 3. OS APIs Used

### 3.1 Timer APIs

| API | Usage | File |
|-----|-------|------|
| `setitimer(ITIMER_PROF, ...)` | Default: CPU-time timer, delivers SIGPROF | `profile-handler.cc:516` |
| `setitimer(ITIMER_REAL, ...)` | Wall-clock timer (via `CPUPROFILE_REALTIME`), delivers SIGALRM | `profile-handler.cc:516` |
| `timer_create(CLOCK_THREAD_CPUTIME_ID, ...)` | Per-thread CPU timer (via `CPUPROFILE_PER_THREAD_TIMERS`) | `profile-handler.cc:284` |
| `timer_create(CLOCK_MONOTONIC, ...)` | Per-thread wall-clock timer | `profile-handler.cc:282-283` |
| `timer_settime()` | Arms per-thread timer with interval | `profile-handler.cc:298` |
| `timer_delete()` | Destroys per-thread timer (in TLS destructor) | `profile-handler.cc:258` |

**Timer mode selection logic:**

```
if CPUPROFILE_PER_THREAD_TIMERS or CPUPROFILE_TIMER_SIGNAL:
    Use POSIX timer_create() with SIGEV_THREAD_ID
    (each thread gets its own timer, signal delivered to that specific thread)
    Clock: CLOCK_THREAD_CPUTIME_ID (or CLOCK_MONOTONIC if CPUPROFILE_REALTIME)
else if CPUPROFILE_REALTIME:
    Use setitimer(ITIMER_REAL) -> SIGALRM
else:
    Use setitimer(ITIMER_PROF) -> SIGPROF   [DEFAULT]
```

### 3.2 Signal APIs

| API | Usage | File |
|-----|-------|------|
| `sigaction(signal_number_, &sa, ...)` | Install signal handler with `SA_RESTART \| SA_SIGINFO` | `profile-handler.cc:380-383` |
| `sigprocmask(SIG_BLOCK, ...)` | Block profiling signal during callback list modification | `profile-handler.cc:97` |
| `sigprocmask(SIG_UNBLOCK, ...)` | Unblock signal | `profile-handler.cc:101` |
| `signal(number, handler)` | Install toggle handler for `CPUPROFILESIGNAL` | `profiler.cc:183` |

### 3.3 Synchronization Primitives

| API | Usage | File |
|-----|-------|------|
| `std::atomic<int>` with CAS | SpinLock implementation (lock-free fast path) | `base/spinlock.h:58` |
| `syscall(__NR_futex, FUTEX_WAIT, ...)` | SpinLock slow path: sleep waiting for lock | `base/spinlock_linux-inl.h:84` |
| `syscall(__NR_futex, FUTEX_WAKE, ...)` | SpinLock: wake sleeping waiters | `base/spinlock_linux-inl.h:96` |
| `pthread_key_create/setspecific` | Per-thread timer ID storage (TLS) | `base/threading.h:105` |

### 3.4 I/O and Memory APIs

| API | Usage | File |
|-----|-------|------|
| `open(fname, O_CREAT\|O_WRONLY\|O_TRUNC)` | Create output profile file | `profiledata.cc:99` |
| `write(fd, buf, len)` | Write profile data (in `FlushEvicted`) | `profiledata.cc:142` |
| `close(fd)` | Close profile output file | `profiledata.cc:191` |
| `new[]/delete[]` | Allocate hash table and eviction buffer | `profiledata.cc:114-115` |
| `strdup()/free()` | Filename storage | `profiledata.cc:106,197` |
| `mmap(PROT_NONE, MAP_PRIVATE\|MAP_ANONYMOUS)` | Create unreadable page (for address checking calibration) | `check_address-inl.h:121` |

### 3.5 Address Validity Checking (for safe frame-pointer unwinding)

| API | Usage | File |
|-----|-------|------|
| `syscall(SYS_rt_sigprocmask, ~0, addr, ...)` | Check if memory address is readable (fast path) | `check_address-inl.h:82` |
| `syscall(SYS_rt_sigprocmask, SIG_BLOCK, addr, old, ...)` | Check readability (robust fallback) | `check_address-inl.h:100` |
| `getpagesize()` | Get page size for alignment checks | `stacktrace_generic_fp-inl.h:129` |

**Clever trick**: On Linux, the profiler abuses `sigprocmask` to test if a memory address is readable. It passes the address as the `set` argument with an invalid `how` parameter (`~0`). The kernel reads the `set` before checking `how`, so:
- If the address is unreadable: `EFAULT`
- If the address is readable: `EINVAL` (because `how` is invalid)

This is async-signal-safe (it's a single syscall) and avoids the overhead of pipe-based approaches.

### 3.6 Context APIs

| API | Usage | File |
|-----|-------|------|
| `ucontext_t` / `sys/ucontext.h` | Access signal handler context (PC, frame pointer) | `profiler.cc:48-56` |
| `syscall(SYS_gettid)` | Get kernel thread ID for per-thread timers | `profile-handler.cc:278` |
| `getpid()`, `getuid()`, `geteuid()` | Security checks, unique file naming | `profiler.cc:174` |

## 4. Signal Handling and Async-Signal Safety

### 4.1 The Signal Handler Chain

```
kernel delivers SIGPROF
    |
    v
ProfileHandler::SignalHandler(sig, sinfo, ucontext)     [profile-handler.cc:533]
    |-- saves errno
    |-- acquires signal_lock_ (SpinLock)
    |-- iterates callbacks_ list
    |       |
    |       v
    |   CpuProfiler::prof_handler(sig, sinfo, ucontext, cpu_profiler)  [profiler.cc:324]
    |       |-- GetPC(*ucontext) -> extract interrupted PC
    |       |-- GetStackTraceWithContext(stack, ..., ucontext) -> unwind stack
    |       |-- collector_.Add(depth, stack) -> store in hash table
    |
    |-- releases signal_lock_
    |-- restores errno
```

### 4.2 What Makes It Async-Signal-Safe?

The signal handler path is carefully designed to avoid all async-signal-unsafe operations:

1. **No malloc/free in the hot path**: `ProfileData::Add()` operates on pre-allocated memory (hash table and eviction buffer allocated during `Start()`). The only heap operations happen in `Start()`/`Stop()` which are never called from signal context.

2. **SpinLock is async-signal-safe**: The `SpinLock` uses `std::atomic<int>` with compare-exchange (no pthread mutex, no system calls on the fast path). The comment in `spinlock.h:35-37` states:
   > "SpinLock is async signal safe. If used within a signal handler, all lock holders should block the signal even outside the signal handler."

3. **`signal_lock_` protocol**: All non-signal code that needs to modify `callbacks_` must:
   - Acquire `control_lock_`
   - Block the profiling signal via `sigprocmask(SIG_BLOCK)`
   - Acquire `signal_lock_`
   - Modify data
   - Release `signal_lock_`
   - Unblock signal

   This ensures the signal handler (which also acquires `signal_lock_`) can never deadlock with the control path.

4. **No locks in `ProfileData::Add()`**: The `Add()` method runs lock-free. Synchronization is provided by the guarantee from `ProfileHandler` that only one instance of any registered callback can run at a time (the `signal_lock_` serializes this).

5. **`write(2)` in signal context**: When the eviction buffer is full, `FlushEvicted()` calls `write()` to the output file. `write(2)` is async-signal-safe per POSIX. The `NO_INTR` macro retries on `EINTR`.

6. **errno preservation**: `SignalHandler` saves and restores `errno` at entry/exit.

7. **Address validity checking during unwinding**: The frame-pointer unwinder uses `SYS_rt_sigprocmask` (a raw syscall) to check if frame pointer addresses are readable, rather than risking a SIGSEGV.

### 4.3 What Is NOT Async-Signal-Safe (But Contained)

- `ProfileHandlerRegisterCallback()` and `ProfileHandlerUnregisterCallback()` — these use `new`/`delete` for callback tokens and `std::list` operations. They are **never called from signal context**.
- `ProfileData::Start()`/`Stop()` — use `open()`, `close()`, `new[]`/`delete[]`, `strdup()`, `free()`. Also never called from signal context.

## 5. Data Structures

### 5.1 Set-Associative Hash Table (`ProfileData`)

The core data structure is a **fixed-size, set-associative hash table** with an eviction buffer:

```
Constants:
    kMaxStackDepth = 254        // Max frames per stack trace
    kAssociativity = 4          // Ways per bucket (set-associative)
    kBuckets       = 1024       // Number of hash buckets (1 << 10)
    kBufferLength  = 262144     // Eviction buffer slots (1 << 18)

struct Entry {
    uintptr_t count;                    // Number of times this trace was seen
    uintptr_t depth;                    // Number of frames in stack trace
    uintptr_t stack[kMaxStackDepth];    // The stack trace (array of PCs)
};

struct Bucket {
    Entry entry[kAssociativity];        // 4 entries per bucket
};

Bucket hash_[kBuckets];                 // The hash table (1024 buckets)
uintptr_t evict_[kBufferLength];        // Eviction/write buffer (262144 slots)
```

**Memory layout**: Each `Entry` stores up to 254 PC values, a depth, and a count. With 4-way associativity and 1024 buckets, the table holds at most 4096 unique stack traces simultaneously. The total hash table size is:
- `Entry` = (254 + 2) * 8 bytes = 2048 bytes
- `Bucket` = 4 * 2048 = 8192 bytes
- Total hash table = 1024 * 8192 = **8 MB**
- Eviction buffer = 262144 * 8 = **2 MB**

### 5.2 Hash Function

Simple polynomial rolling hash over PC values:

```cpp
Slot h = 0;
for (int i = 0; i < depth; i++) {
    Slot slot = reinterpret_cast<Slot>(stack[i]);
    h = (h << 8) | (h >> (8*(sizeof(h)-1)));  // rotate left 8 bits
    h += (slot * 31) + (slot * 7) + (slot * 3);
}
bucket = &hash_[h % kBuckets];
```

This runs entirely in the signal handler. It's simple, fast, and uses no division (the modulo is against a power-of-2 bucket count).

### 5.3 Lookup and Eviction (LRU-like)

The `Add()` algorithm:

```
1. Compute hash h from stack trace
2. Look in bucket[h % kBuckets]:
   - Scan all 4 ways for exact match (compare depth + all PCs)
   - If match found: increment count, done
3. If no match:
   - Find entry with smallest count in the bucket
   - Evict it (write count + depth + stack to eviction buffer)
   - Replace with new stack trace (count = 1)
4. If eviction buffer is full:
   - Call FlushEvicted() -> write() to output file
```

This is a **lossy** counting scheme. Frequently-seen stack traces accumulate high counts and resist eviction. Infrequent traces get evicted to disk with smaller counts. The effect is similar to a counting Bloom filter / heavy hitters sketch — very common patterns stay in the hot table while rare ones flow through to disk.

### 5.4 Callback List (`ProfileHandler`)

```cpp
std::list<ProfileHandlerToken*> callbacks_;
```

A `std::list` of callback pointers. The list is typically very small (1-2 entries: the CPU profiler and possibly a thread module). Protected by the two-lock protocol described above.

**Mutation without malloc under signal lock**: When modifying the callback list, `ProfileHandler` builds a new list outside the signal-critical section, then under `signal_lock_` performs only a `swap()` of the list (which is a pointer swap, no allocation). Old nodes are freed **after** releasing `signal_lock_`.

### 5.5 SpinLock State Machine

```
Three states of lockword_ (std::atomic<int>):

kSpinLockFree    = 0    (unlocked)
kSpinLockHeld    = 1    (locked, no waiters)
kSpinLockSleeper = 2    (locked, has waiters)

Lock fast path:  CAS(Free -> Held)
Lock slow path:  adaptive spin + CAS(Held -> Sleeper) + futex(FUTEX_WAIT)
Unlock fast path: exchange(Free), if was Held -> done
Unlock slow path: if was Sleeper -> futex(FUTEX_WAKE)
```

## 6. Memory Allocation and Management

### 6.1 Allocation Timeline

**At `ProfileData::Start()` time** (outside signal context):
- `new Bucket[1024]` — hash table (~8 MB)
- `new Slot[262144]` — eviction buffer (~2 MB)
- `strdup(fname)` — filename string
- `open(fname, ...)` — file descriptor

**At `ProfileData::Stop()` time** (outside signal context):
- Flush remaining entries to disk
- `close(fd)`
- `delete[] hash_`
- `delete[] evict_`
- `free(fname)`

**During signal handler (`Add()`)**:
- **Zero allocations**. All work happens on the pre-allocated hash table and eviction buffer.
- The only system call that can happen is `write(2)` when the eviction buffer is full (`FlushEvicted()`).

### 6.2 Callback Token Management

`ProfileHandlerToken` objects are `new`'d during `RegisterCallback()` and `delete`'d during `UnregisterCallback()`. Both happen outside signal context. The signal handler only reads the callback list, never modifies it.

### 6.3 Per-Thread Timer Resources

When `CPUPROFILE_PER_THREAD_TIMERS` is enabled:
- `timer_create()` allocates a kernel timer per thread
- A `timer_id_holder` is `new`'d per thread and stored in TLS via `pthread_key_create()`
- The TLS destructor (`ThreadTimerDestructor`) calls `timer_delete()` and `delete holder` on thread exit

## 7. Stack Unwinding

### 7.1 Available Backends

| Backend | File | Signal-safe? | Notes |
|---------|------|-------------|-------|
| Generic frame-pointer | `stacktrace_generic_fp-inl.h` | Yes | Preferred on x86-64, aarch64, riscv. Requires `-fno-omit-frame-pointer` |
| libunwind | `stacktrace_libunwind-inl.h` | Mostly | Uses DWARF info, no frame pointers needed |
| libgcc (`_Unwind_Backtrace`) | `stacktrace_libgcc-inl.h` | Partially | May call malloc, may use internal locks |
| glibc `backtrace()` | `stacktrace_generic-inl.h` | Partially | Usually wraps libgcc |

### 7.2 Frame-Pointer Unwinding Details

The preferred backend (`stacktrace_generic_fp-inl.h`) walks the frame pointer chain:

```
Frame layout (x86-64):
    [higher addresses]
    ...
    return address   <-- frame->pc
    saved RBP        <-- frame->parent (points to caller's frame)
    ...
    [lower addresses]

struct frame {
    uintptr_t parent;   // saved frame pointer (points to parent frame)
    void* pc;           // return address
};
```

**Safety checks during unwinding:**

1. **Alignment check**: Frame pointer must be aligned to architecture-specific boundary (16 bytes on x86-64)
2. **Minimum address check**: Frame pointer must be > 16KB (`kTooSmallAddr = 16 << 10`), filtering out NULL-ish pointers
3. **Frame size check**: Distance between child and parent frame must be < 128KB (`kFrameSizeThreshold = 128 << 10`). Larger gaps indicate corruption.
4. **Stack growth direction**: Parent frame address must be > child frame address (stack grows downward)
5. **Page readability check**: Before dereferencing a frame pointer, `CheckPageIsReadable()` verifies the page is mapped (using the `SYS_rt_sigprocmask` trick described above). This can be disabled with the "unsafe" variant for speed.

### 7.3 How the Signal Handler Captures the Stack

```cpp
// In CpuProfiler::prof_handler():

// 1. Get the PC from the signal ucontext (the interrupted instruction)
stack[0] = GetPC(*reinterpret_cast<ucontext_t*>(signal_ucontext));

// 2. Unwind the rest of the stack, skipping 3 frames:
//    - prof_handler() itself
//    - ProfileHandler::SignalHandler()
//    - The kernel's signal trampoline frame
int depth = GetStackTraceWithContext(stack + 1, max - 1, /*skip=*/3, signal_ucontext);

// 3. Deduplicate: if frame-pointer unwinder already got the PC as first frame,
//    skip the manually-extracted PC to avoid double counting
if (depth > 0 && stack[1] == stack[0]) {
    used_stack = stack + 1;   // skip duplicate
} else {
    used_stack = stack;
    depth++;
}
```

The `ucontext_t` is passed to the stack unwinder (`GetStackTraceWithContext`) so it can use the signal context's frame pointer as the starting point, rather than the signal handler's own frame.

## 8. Locking Protocol

### 8.1 The Two-Lock Design in ProfileHandler

```
control_lock_ (SpinLock)
    - Acquired by: RegisterCallback, UnregisterCallback, Reset, GetState, RegisterThread, UpdateTimer
    - Purpose: Serialize all control operations
    - ACQUIRED_BEFORE(signal_lock_)  [enforced ordering annotation]

signal_lock_ (SpinLock)
    - Acquired by: SignalHandler (in signal context), and control operations (after blocking signal)
    - Purpose: Protect callbacks_ list and interrupts_ counter
    - Held briefly: just iterating callbacks or swapping list
```

**Why two locks?** The signal handler needs to access `callbacks_`, but it can't acquire `control_lock_` (which may be held by the thread being interrupted — would deadlock). So `signal_lock_` is a separate, more granular lock that the signal handler can safely acquire.

**Why it doesn't deadlock:**
- The signal handler only takes `signal_lock_`
- Non-signal code that takes `signal_lock_` first **blocks the signal** via `sigprocmask(SIG_BLOCK)`, so the signal handler cannot interrupt while `signal_lock_` is held by non-signal code

### 8.2 CpuProfiler's Lock

```
lock_ (SpinLock)
    - Acquired by: Start, Stop, FlushTable, Enabled, GetCurrentState
    - NOT acquired by: prof_handler (the signal callback)
    - Purpose: Serialize control operations and protect collector_ state
```

**Protocol for modifying shared state:**
```
1. Acquire lock_
2. Call DisableHandler() -> ProfileHandlerUnregisterCallback()
   (This blocks the signal, waits for running callback to complete)
3. Safely modify collector_ (no signal handler can run now)
4. Call EnableHandler() if continuing
5. Release lock_
```

This is the pattern the `profile-handler.h` header documents:
> When code other than the signal handler modifies the shared data it must:
> - Acquire lock
> - Unregister the callback with the ProfileHandler
> - Modify shared data
> - Re-register the callback
> - Release lock

And the callback code gets **lockless, read-write access** to the data.

### 8.3 Signal Lock Acquisition in Non-Signal Context

```cpp
// ScopedSignalBlocker blocks the signal for the current thread
class ScopedSignalBlocker {
    ScopedSignalBlocker(int signo) {
        sigaddset(&sig_set_, signo);
        sigprocmask(SIG_BLOCK, &sig_set_, nullptr);    // Block signal
    }
    ~ScopedSignalBlocker() {
        sigprocmask(SIG_UNBLOCK, &sig_set_, nullptr);  // Unblock signal
    }
};

// Usage in RegisterCallback:
SpinLockHolder cl(&control_lock_);
{
    ScopedSignalBlocker block(signal_number_);  // Block SIGPROF
    SpinLockHolder sl(&signal_lock_);           // Now safe to acquire
    callbacks_.splice(callbacks_.end(), copy);   // Modify list
}
// Signal unblocked, signal_lock_ released
```

### 8.4 Avoiding Malloc Under signal_lock_

When unregistering a callback, the code must remove a node from `std::list`. But `std::list` node deletion calls `delete` (which calls `malloc`'s `free`), and calling `malloc`/`free` while holding `signal_lock_` risks deadlock (malloc has its own locks).

The solution: **swap, don't delete under lock**.

```cpp
void ProfileHandler::UnregisterCallback(ProfileHandlerToken* token) {
    SpinLockHolder cl(&control_lock_);

    // Build new list WITHOUT the token (outside signal lock)
    CallbackList copy;
    for (auto* t : callbacks_) {
        if (t != token) copy.push_back(t);
    }

    {
        ScopedSignalBlocker block(signal_number_);
        SpinLockHolder sl(&signal_lock_);
        // Only swap pointers under signal lock (no alloc/dealloc)
        using std::swap;
        swap(copy, callbacks_);
    }
    // Old list nodes in 'copy' are destroyed here, OUTSIDE signal_lock_
    delete token;
}
```

## 9. Output Format

The profile is written as a binary stream of `uintptr_t` values:

### Header
```
0                  // count (0 = header marker)
3                  // depth (3 = header has 3 data fields)
0                  // version number
period_usec        // sampling period in microseconds (e.g., 10000 for 100Hz)
0                  // padding
```

### Sample Records
```
count              // number of times this stack trace was observed
depth              // number of PC values in trace
pc[0]              // bottom of stack (most recent call)
pc[1]
...
pc[depth-1]        // top of stack
```

### End Marker
```
0                  // count = 0
1                  // depth = 1
0                  // single zero PC = end marker
```

### Trailer
After the binary data, `/proc/self/maps` is appended as text. This allows the analysis tool (`pprof`) to map PC addresses to shared library names and offsets.

## 10. Per-Thread Timer Mode (Linux-specific)

Enabled by `CPUPROFILE_PER_THREAD_TIMERS` environment variable. Uses Linux-specific POSIX timer APIs:

```cpp
struct sigevent sevp;
sevp.sigev_notify = SIGEV_THREAD_ID;            // Deliver signal to specific thread
sevp.sigev_notify_thread_id = syscall(SYS_gettid);  // This thread's kernel TID
sevp.sigev_signo = signal_number_;               // Signal to deliver

timer_create(CLOCK_THREAD_CPUTIME_ID, &sevp, &timerid);  // Per-thread CPU clock

struct itimerspec its;
its.it_interval.tv_nsec = 1000000000 / frequency;  // e.g., 10ms for 100Hz
its.it_value = its.it_interval;
timer_settime(timerid, 0, &its, NULL);
```

**Advantages over `setitimer(ITIMER_PROF)`:**
- Each thread gets its own timer: more accurate per-thread CPU time measurement
- Signal is delivered to the specific thread, not to an arbitrary thread in the process
- Custom signal number via `CPUPROFILE_TIMER_SIGNAL` (avoids SIGPROF/SIGALRM conflicts)

**Cleanup**: Timer IDs are stored in thread-local storage (via `pthread_key_create` with destructor). On thread exit, the destructor calls `timer_delete()`.

**Limitation**: Once per-thread timers are enabled, they cannot be disabled per-thread — the `UpdateTimer()` method becomes a no-op. This is because each thread manages its own timer lifecycle.

## 11. Design Tradeoffs and Lessons

### 11.1 Strengths

1. **Minimal signal-handler overhead**: The hot path (`prof_handler` -> `Add`) does zero memory allocations, uses no locks, and performs no system calls unless the eviction buffer is full.

2. **Bounded memory usage**: The hash table is a fixed 8MB + 2MB eviction buffer. This is allocated once and never grows.

3. **Lossy but statistically sound**: The set-associative hash table with eviction is essentially a heavy-hitters algorithm. Hot call stacks (the ones you care about in profiling) accumulate high counts and resist eviction. Cold stacks flow through to disk quickly. The aggregate output is statistically faithful.

4. **Clean separation of concerns**: Timer management (ProfileHandler) is separate from data collection (ProfileData) and orchestration (CpuProfiler). Multiple consumers can register callbacks.

5. **Signal blocking as synchronization**: Rather than trying to make complex data structure operations async-signal-safe, the code blocks the signal when it needs exclusive access. Simple and correct.

### 11.2 Weaknesses / Limitations

1. **`ITIMER_PROF` is process-wide**: Only one profiling consumer can use it. This is the default mode. Per-thread timers solve this but require additional setup.

2. **Hash table size is fixed at compile time**: Not tunable without recompilation. The 8MB hash table may be wasteful for simple programs or insufficient for complex ones.

3. **`write()` in signal handler**: When the eviction buffer fills, `FlushEvicted()` calls `write()` in signal context. While `write(2)` is POSIX async-signal-safe, it can block (e.g., on a full pipe, slow NFS). This introduces latency jitter.

4. **Frame-pointer dependency**: The preferred unwinder requires `-fno-omit-frame-pointer`. Without it, stack traces are incomplete. On x86-64, GCC/Clang default to omitting frame pointers unless told otherwise.

5. **SpinLock in signal handler**: While the `signal_lock_` is held very briefly, any contention means spinning in a signal handler. The protocol prevents deadlock but not all contention scenarios (though in practice, the control path blocks the signal first).

### 11.3 Key Lessons for Our Python Profiler

1. **Pre-allocate everything**: Allocate all data structures before installing the signal handler. Zero allocations in the hot path.

2. **The "unregister-before-modify" pattern**: Instead of complex locking in the signal handler, unregister the handler, wait for completion, modify state, re-register. This is the cleanest approach to signal-handler synchronization.

3. **`sigprocmask` for address checking**: The `SYS_rt_sigprocmask` trick is a fast, async-signal-safe way to test memory readability — useful for safe frame-pointer walking.

4. **Hash table with eviction for sample aggregation**: Reduces memory usage vs. storing every raw sample, at the cost of some accuracy for rare stack traces.

5. **Per-thread timers via `timer_create` + `SIGEV_THREAD_ID`**: More accurate than `ITIMER_PROF` for multi-threaded programs. Essential for per-thread CPU profiling.

6. **Two-lock protocol with signal blocking**: A practical pattern for safely sharing data between signal handlers and normal code without deadlock.

---

## File Index

Source files copied to `src/` directory for reference:

| File | Description |
|------|-------------|
| `src/profiler.h` | Public C API |
| `src/profiler.cc` | CpuProfiler singleton, signal handler callback |
| `src/profile-handler.h` | ProfileHandler API and callback typedef |
| `src/profile-handler.cc` | Timer management, signal handler, callback registry |
| `src/profiledata.h` | ProfileData class with hash table types |
| `src/profiledata.cc` | Hash table implementation, eviction, file I/O |
| `src/getpc.h` | GetPC() - extract PC from ucontext_t |
| `src/getpc-inl.h` | Auto-generated SFINAE-based PC extraction for all OS/arch |
| `src/stacktrace.cc` | Stack unwinding dispatch (selects backend at init) |
| `src/stacktrace_generic_fp-inl.h` | Frame-pointer-based stack unwinder |
| `src/stacktrace_impl_setup-inl.h` | Macro scaffolding for stack trace backends |
| `src/check_address-inl.h` | Memory readability check (sigprocmask trick) |
| `src/base/spinlock.h` | SpinLock (async-signal-safe, futex-backed) |
| `src/base/spinlock.cc` | SpinLock slow path (adaptive spin + futex) |
| `src/base/spinlock_internal.cc` | SpinLockDelay/Wake dispatch |
| `src/base/spinlock_internal.h` | SpinLockDelay/Wake declarations |
| `src/base/spinlock_linux-inl.h` | Linux futex-based SpinLockDelay/Wake |
| `src/base/threading.h` | TLS key wrappers (pthread_key_create) |
