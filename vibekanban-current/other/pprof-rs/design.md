# pprof-rs Design Analysis (v0.11.1)

Deep analysis of pprof-rs Linux CPU profiler internals.
Source: https://github.com/tikv/pprof-rs tag v0.11.1

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Component Map](#component-map)
- [Lifecycle: Start → Sample → Stop → Report](#lifecycle)
- [OS APIs Used (Linux)](#os-apis-used-linux)
- [Signal Handler: `perf_signal_handler`](#signal-handler-perf_signal_handler)
- [Async-Signal Safety Strategy](#async-signal-safety-strategy)
- [Data Structures](#data-structures)
- [Memory Allocation and Management](#memory-allocation-and-management)
- [Backtrace Collection](#backtrace-collection)
- [Address Validation (Pipe Trick)](#address-validation-pipe-trick)
- [Blocklist Mechanism](#blocklist-mechanism)
- [Symbol Resolution](#symbol-resolution)
- [Report Generation](#report-generation)
- [Known Limitations and Design Tradeoffs](#known-limitations-and-design-tradeoffs)
- [Relevance for Python CPU Profiler Design](#relevance-for-python-cpu-profiler-design)

---

## Architecture Overview

pprof-rs is a self-contained, in-process CPU profiler for Rust programs. The core idea:

1. Use `setitimer(ITIMER_PROF)` to generate periodic `SIGPROF` signals proportional to CPU time consumed.
2. In the signal handler, capture a backtrace (stack trace) of the interrupted thread.
3. Store the raw backtrace in a pre-allocated hash map (no malloc in signal handler).
4. On report generation, symbolicate the raw addresses into human-readable function names.

```
┌─────────────────────────────────────────────────────────────┐
│                      User Code                               │
│  ProfilerGuardBuilder::build()                               │
│       │                                                      │
│       ├─ Creates global Profiler (Lazy<RwLock<Profiler>>)    │
│       ├─ Registers SIGPROF signal handler (sigaction)        │
│       ├─ Starts setitimer(ITIMER_PROF, frequency)            │
│       └─ Returns ProfilerGuard (RAII)                        │
│                                                              │
│  ┌──────── Timer fires SIGPROF ─────────┐                    │
│  │                                      │                    │
│  │  perf_signal_handler() {             │                    │
│  │    try_write(PROFILER)               │  ← no deadlock    │
│  │    extract RIP from ucontext         │  ← blocklist chk  │
│  │    TraceImpl::trace(ucontext, cb)    │  ← backtrace      │
│  │    profiler.sample(bt, thread_name)  │  ← store sample   │
│  │  }                                   │                    │
│  └──────────────────────────────────────┘                    │
│                                                              │
│  guard.report().build()                                      │
│       │                                                      │
│       ├─ Iterates Collector (HashMap + TempFile overflow)    │
│       ├─ Symbolicates: addr → function name, file, line     │
│       └─ Returns Report { HashMap<Frames, count> }           │
│                                                              │
│  drop(guard)                                                 │
│       ├─ drop(Timer) → setitimer(0,0) stops timer            │
│       └─ profiler.stop() → sigaction(SIGPROF, SIG_IGN)       │
└─────────────────────────────────────────────────────────────┘
```

## Component Map

| File | Role |
|------|------|
| `src/profiler.rs` | Core profiler struct, signal handler, start/stop, sample collection |
| `src/timer.rs` | `setitimer(ITIMER_PROF)` wrapper, frequency configuration |
| `src/collector.rs` | Fixed-size hash map + temp file overflow storage |
| `src/frames.rs` | `UnresolvedFrames` (raw addresses) and `Frames` (symbolicated) |
| `src/addr_validate.rs` | Async-signal-safe memory address validation via pipe write trick |
| `src/backtrace/mod.rs` | Trait definitions: `Trace`, `Frame`, `Symbol` |
| `src/backtrace/frame_pointer.rs` | Manual frame-pointer-based stack walking |
| `src/backtrace/backtrace_rs.rs` | Delegation to `backtrace-rs` (libunwind-based) |
| `src/report.rs` | Report building: merge counts, symbolicate, generate flamegraph/pprof |
| `src/error.rs` | Error types |
| `src/lib.rs` | Public API, constants (`MAX_DEPTH=128`, `MAX_THREAD_NAME=16`) |

---

## Lifecycle

### 1. Start Profiling

```
ProfilerGuardBuilder::build()
  → trigger_lazy()          // force backtrace-rs init + PROFILER lazy init
  → PROFILER.write()        // acquire write lock
  → profiler.start()
      → register_signal_handler()
          → sigaction(SIGPROF, perf_signal_handler, SA_SIGINFO | SA_RESTART)
  → Timer::new(frequency)
      → setitimer(ITIMER_PROF, interval_usec)
  → return ProfilerGuard { profiler, timer }
```

Key detail: `trigger_lazy()` forces initialization of `backtrace::Backtrace::new()` and the global `PROFILER` **before** the signal handler is active. This ensures that lazy allocations (which would be unsafe in a signal handler) happen upfront.

### 2. Sampling (Signal Handler)

Each `SIGPROF` delivery invokes `perf_signal_handler`. See [detailed analysis below](#signal-handler-perf_signal_handler).

### 3. Stop Profiling

```
drop(ProfilerGuard)
  → drop(timer)             // Timer::drop() → setitimer(ITIMER_PROF, 0) stops signal delivery
  → profiler.stop()
      → unregister_signal_handler()
          → signal(SIGPROF, SIG_IGN)
      → profiler.init()     // reset collector
```

The timer is dropped **first** to stop signal delivery before unregistering the handler. This ordering prevents a window where signals could be delivered but the handler is already removed.

### 4. Report Generation

```
guard.report().build()
  → PROFILER.write()         // acquire write lock (blocks signal handler via try_write)
  → profiler.data.try_iter() // iterate over HashCounter + TempFdArray
  → for each (UnresolvedFrames, count):
      → Frames::from(unresolved)   // symbolicate via backtrace::resolve
      → aggregate into HashMap<Frames, isize>
  → return Report { data, timing }
```

---

## OS APIs Used (Linux)

### Timer: `setitimer(2)`

**File:** `src/timer.rs:21-23`

```rust
extern "C" {
    fn setitimer(which: c_int, new_value: *mut Itimerval, old_value: *mut Itimerval) -> c_int;
}
const ITIMER_PROF: c_int = 2;
```

- Uses `ITIMER_PROF` which counts **only CPU time** (user + system) spent by the process.
- When the timer expires, the kernel delivers `SIGPROF` to the thread that consumed the CPU time.
- The interval is set as `1,000,000 / frequency` microseconds (e.g., 100 Hz → 10,000 µs interval).
- **Important limitation:** `ITIMER_PROF` is per-process, not per-thread. On Linux, the kernel delivers the signal to the thread that was executing when the timer expired, which naturally distributes samples proportionally to CPU usage across threads.
- Timer is stopped by setting both `it_interval` and `it_value` to zero in `Timer::drop()`.

### Signal Handler Registration: `sigaction(2)`

**File:** `src/profiler.rs:432-444`

```rust
fn register_signal_handler(&self) -> Result<()> {
    let handler = signal::SigHandler::SigAction(perf_signal_handler);
    let sigaction = signal::SigAction::new(
        handler,
        signal::SaFlags::SA_SIGINFO | signal::SaFlags::SA_RESTART,
        signal::SigSet::empty(),
    );
    unsafe { signal::sigaction(signal::SIGPROF, &sigaction) }?;
    Ok(())
}
```

- **`SA_SIGINFO`**: The handler receives 3 arguments: `(signal, siginfo_t*, ucontext*)`. The `ucontext` is critical — it contains the register state (including the instruction pointer and frame pointer) of the interrupted code.
- **`SA_RESTART`**: Automatically restarts interrupted system calls (like `read`, `write`) so the profiler doesn't cause `EINTR` errors in the profiled program.
- **`SigSet::empty()`**: No additional signals are blocked during handler execution. This means the handler could theoretically be interrupted by another signal (but not by another `SIGPROF` since the signal being handled is blocked by default).

### Unregistration: `signal(2)`

**File:** `src/profiler.rs:446-449`

```rust
fn unregister_signal_handler(&self) -> Result<()> {
    let handler = signal::SigHandler::SigIgn;
    unsafe { signal::signal(signal::SIGPROF, handler) }?;
    Ok(())
}
```

Sets handler to `SIG_IGN` (ignore). Note: does **not** restore the original handler (acknowledged as a TODO in README).

### ucontext Register Access

**File:** `src/profiler.rs:290-328`

The signal handler receives `ucontext_t*` which contains the machine context (registers) at the point of interruption:

| Architecture | Instruction Pointer | Frame Pointer |
|---|---|---|
| x86_64 Linux | `uc_mcontext.gregs[REG_RIP]` | `uc_mcontext.gregs[REG_RBP]` |
| aarch64 Linux | `uc_mcontext.pc` | `uc_mcontext.regs[29]` |
| riscv64 Linux | `uc_mcontext.__gregs[REG_PC]` | `uc_mcontext.__gregs[REG_S0]` |
| loongarch64 Linux | `uc_mcontext.sc_pc` | `uc_mcontext.sc_regs[22]` |

The instruction pointer is used for blocklist checking. The frame pointer is used for frame-pointer-based stack walking.

### Thread Identification

**File:** `src/profiler.rs:354-361`

```rust
let current_thread = unsafe { libc::pthread_self() };
// ...
libc::pthread_getname_np(current_thread, name_ptr, MAX_THREAD_NAME)
```

- `pthread_self()` — returns the calling thread's ID (async-signal-safe).
- `pthread_getname_np()` — gets the thread name. **Note:** `pthread_getname_np` is NOT listed as async-signal-safe in POSIX. This is a pragmatic tradeoff — it works in practice on Linux/glibc because it reads from a fixed-size buffer in the thread control block.

### Errno Protection

**File:** `src/profiler.rs:231-263`

```rust
struct ErrnoProtector(libc::c_int);

impl ErrnoProtector {
    fn new() -> Self {
        unsafe {
            let errno = *libc::__errno_location();
            Self(errno)
        }
    }
}

impl Drop for ErrnoProtector {
    fn drop(&mut self) {
        unsafe { *libc::__errno_location() = self.0; }
    }
}
```

The signal handler saves and restores `errno` via `__errno_location()` (Linux-specific TLS accessor). This is critical because the signal handler might call functions that modify `errno`, and the interrupted code might be checking `errno` after a syscall.

### Pipe-based Address Validation: `pipe2(2)`, `read(2)`, `write(2)`

**File:** `src/addr_validate.rs`

Uses `pipe2(O_CLOEXEC | O_NONBLOCK)` to create a non-blocking pipe, then validates memory addresses by attempting `write(pipe_fd, addr, len)`. See [detailed section below](#address-validation-pipe-trick).

### Shared Library Enumeration: `dl_iterate_phdr(3)`

**File:** `src/profiler.rs:86-113` (via `findshlibs` crate)

```rust
TargetSharedLibrary::each(|shlib| {
    // get name, segments, virtual address ranges
});
```

The `findshlibs` crate internally uses `dl_iterate_phdr()` on Linux to iterate loaded shared libraries. This is used only during `ProfilerGuardBuilder::blocklist()` setup (not in the signal handler).

---

## Signal Handler: `perf_signal_handler`

**File:** `src/profiler.rs:265-364`

This is the most critical function. It runs in signal context on every timer expiration.

```rust
#[no_mangle]
extern "C" fn perf_signal_handler(
    _signal: c_int,
    _siginfo: *mut libc::siginfo_t,
    ucontext: *mut libc::c_void,
) {
    // 1. Save and restore errno
    let _errno = ErrnoProtector::new();

    // 2. Try to acquire the profiler lock (non-blocking)
    if let Some(mut guard) = PROFILER.try_write() {
        if let Ok(profiler) = guard.as_mut() {

            // 3. Extract instruction pointer from ucontext for blocklist check
            let addr = unsafe { (*ucontext).uc_mcontext.gregs[libc::REG_RIP as usize] as usize };
            if profiler.is_blocklisted(addr) {
                return;
            }

            // 4. Collect backtrace into stack-allocated SmallVec
            let mut bt: SmallVec<[Frame; MAX_DEPTH]> = SmallVec::with_capacity(MAX_DEPTH);
            let mut index = 0;
            let sample_timestamp: SystemTime = SystemTime::now();
            TraceImpl::trace(ucontext, |frame| {
                if index < MAX_DEPTH {
                    bt.push(frame.clone());
                    index += 1;
                    true
                } else {
                    false
                }
            });

            // 5. Get thread identity
            let current_thread = unsafe { libc::pthread_self() };
            let mut name = [0; MAX_THREAD_NAME];
            write_thread_name(current_thread, &mut name);

            // 6. Store sample
            profiler.sample(bt, name.to_bytes(), current_thread as u64, sample_timestamp);
        }
    }
}
```

### Step-by-step breakdown:

1. **Errno protection** — RAII guard saves `errno` on entry, restores on exit. Essential because functions called in the handler may modify errno.

2. **`try_write()` — non-blocking lock acquisition** — If the profiler's `RwLock` is held (e.g., by report generation), the signal handler **drops the sample** rather than deadlocking. This is the primary deadlock avoidance mechanism. Uses `parking_lot::RwLock` which provides `try_write()`.

3. **Blocklist check** — Extracts the instruction pointer (RIP on x86_64) from the `ucontext` and checks if it falls within any blocklisted library segment. If the interrupted code was in libc/libgcc/pthread, the sample is skipped. This prevents potential deadlocks from unwinding through non-signal-safe code (particularly libgcc's `_Unwind_Backtrace`).

4. **Backtrace collection** — Calls `TraceImpl::trace()` which is either:
   - **frame-pointer mode**: Manual frame pointer chain walking (signal-safe)
   - **default mode**: `backtrace::trace_unsynchronized()` → libunwind (NOT signal-safe, but works with blocklist)

   Uses `SmallVec<[Frame; 128]>` which stores up to 128 frames **inline on the stack** (no heap allocation for typical traces).

5. **Thread identification** — Gets thread ID via `pthread_self()` and thread name via `pthread_getname_np()`. Both are called inside the signal handler.

6. **Sample storage** — `profiler.sample()` creates an `UnresolvedFrames` and adds it to the `Collector`. The collector uses a pre-allocated hash map, so this is allocation-free in the common case. In the eviction case, it writes to a temp file via `write()` (which IS async-signal-safe).

---

## Async-Signal Safety Strategy

pprof-rs acknowledges that perfect signal safety is impossible with its current design (libunwind is not signal-safe). It uses a defense-in-depth approach:

### 1. Deadlock Avoidance: `try_write()` instead of `write()`

```rust
if let Some(mut guard) = PROFILER.try_write() {
```

The `parking_lot::RwLock::try_write()` is a non-blocking operation. If the lock is held (e.g., by `report().build()` which takes a write lock), the signal handler returns immediately, dropping the sample. This prevents the classic deadlock where:
- Main thread holds lock for reporting
- Signal interrupts main thread
- Signal handler tries to acquire same lock → deadlock

**Note:** `parking_lot` uses a spinlock internally (no `futex`), which is important because `futex(2)` is NOT async-signal-safe. The README explicitly mentions: *"futex is also not safe to use in signal handler. So we use a spin lock to avoid usage of futex."*

### 2. Pre-allocated Data Structures: No `malloc` in Hot Path

The `HashCounter` is pre-allocated with fixed dimensions:
- `BUCKETS = 4096` (1 << 12)
- `BUCKETS_ASSOCIATIVITY = 4`

This means the hash map can hold up to 16,384 unique stack traces without any allocation. The `SmallVec<[Frame; 128]>` also avoids allocation for traces up to 128 frames deep (they live on the signal handler's stack).

### 3. Blocklist: Skip Dangerous Libraries

By checking the instruction pointer against blocklisted library segments, pprof-rs avoids unwinding through code that holds internal locks (e.g., libgcc's exception handling, glibc's malloc).

### 4. Errno Preservation

The `ErrnoProtector` RAII guard ensures that any errno modifications by functions called in the signal handler don't leak to the interrupted code.

### 5. File I/O for Overflow

When the hash map evicts an entry, it writes to a temp file using `write(2)`, which IS async-signal-safe. However, the `TempFdArray::push()` path calls `self.file.write_all(buf)` which goes through Rust's `Write` trait and may internally retry on `EINTR`. The `flush_buffer()` function writes raw bytes.

### What is NOT signal-safe:

| Function | Issue | Mitigation |
|---|---|---|
| `backtrace::trace_unsynchronized()` | Uses libunwind which calls `dl_iterate_phdr` + `malloc` | Blocklist libc/libgcc/pthread |
| `pthread_getname_np()` | Not POSIX async-signal-safe | Works in practice on Linux glibc |
| `SystemTime::now()` | Calls `clock_gettime()` — actually IS signal-safe | No issue |
| `SmallVec::push()` if overflow | Would heap-allocate | Bounded by `MAX_DEPTH=128` inline capacity |
| `std::io::Write::write_all()` | Complex Rust I/O | Only on eviction path |
| `Hash` / `DefaultHasher` | Computation only, no syscalls | Safe |

---

## Data Structures

### `HashCounter<T>` — Fixed-Size Set-Associative Hash Map

**File:** `src/collector.rs:108-145`

```
┌──────────────────────────────────────────────────────┐
│                 HashCounter<T>                         │
│                                                        │
│  buckets: Box<[Bucket<T>; 4096]>                      │
│                                                        │
│  Each Bucket:                                          │
│  ┌────────────────────────────────────────────┐       │
│  │ length: usize                               │       │
│  │ entries: Box<[Entry<T>; 4]>                 │       │
│  │   [0]: { item: T, count: isize }           │       │
│  │   [1]: { item: T, count: isize }           │       │
│  │   [2]: { item: T, count: isize }           │       │
│  │   [3]: { item: T, count: isize }           │       │
│  └────────────────────────────────────────────┘       │
│                                                        │
│  Total capacity: 4096 × 4 = 16,384 entries            │
└──────────────────────────────────────────────────────┘
```

This is a **set-associative hash table** (similar to CPU cache organization):
- **4096 buckets** (BUCKETS = 1 << 12)
- **4-way associativity** (BUCKETS_ASSOCIATIVITY = 4)
- Hash function: `DefaultHasher` (SipHash) → `hash % 4096` selects bucket
- **Collision handling**: Linear scan within the 4-entry bucket
- **Eviction policy**: When a bucket is full, the entry with the **minimum count** is evicted

This design is chosen specifically for signal safety:
- All memory is pre-allocated at profiler creation time
- The `add()` method never allocates — it either finds an existing entry, uses a free slot, or evicts
- The evicted entry is returned to the caller (which writes it to temp file)

### `Bucket<T>` — Fixed-Size Array with Linear Scan

**File:** `src/collector.rs:33-88`

```rust
pub struct Bucket<T: 'static> {
    pub length: usize,
    entries: Box<[Entry<T>; BUCKETS_ASSOCIATIVITY]>,  // Box<[Entry<T>; 4]>
}
```

The `add()` method:
1. Linear scan `entries[0..length]` for matching key → increment count
2. If not found and `length < 4` → insert at `entries[length]`, increment length
3. If full → find minimum count entry, swap it out, return evicted entry

The eviction of minimum-count entries is a clever optimization: high-frequency stack traces stay in the hash map, while rare traces get evicted to disk. Since profiling typically has a power-law distribution (few hot paths, many cold paths), this keeps the most important data in the fast path.

### `TempFdArray<T>` — Temp File Overflow Buffer

**File:** `src/collector.rs:147-207`

```
┌──────────────────────────────────────┐
│          TempFdArray<T>               │
│                                        │
│  file: NamedTempFile (on disk)        │
│                                        │
│  buffer: Box<[T; BUFFER_LENGTH]>      │
│    BUFFER_LENGTH = (1<<18) / sizeof(Entry<UnresolvedFrames>)
│    ≈ 262144 bytes / ~2064 bytes = ~127 entries
│                                        │
│  buffer_index: usize                   │
│                                        │
│  Write path:                           │
│    push(entry) →                       │
│      if buffer full → flush to file    │
│      else → buffer[index++] = entry    │
│                                        │
│  Read path (report time):              │
│    try_iter() → chain(buffer, file)    │
└──────────────────────────────────────┘
```

Evicted entries are buffered in memory and periodically flushed to a temporary file. At report time, the iterator chains the in-memory buffer with file contents. Data is written as raw bytes (no serialization).

### `Collector<T>` — Combined Hash Map + Overflow

**File:** `src/collector.rs:236-262`

```rust
pub struct Collector<T: Hash + Eq + 'static> {
    map: HashCounter<T>,
    temp_array: TempFdArray<Entry<T>>,
}
```

The `add()` operation:
```rust
pub fn add(&mut self, key: T, count: isize) -> std::io::Result<()> {
    if let Some(evict) = self.map.add(key, count) {
        self.temp_array.push(evict)?;
    }
    Ok(())
}
```

At report time, `try_iter()` chains both sources, and the report builder aggregates all counts for matching entries.

### `UnresolvedFrames` — Raw Sample Data

**File:** `src/frames.rs:17-87`

```rust
pub struct UnresolvedFrames {
    pub frames: SmallVec<[<TraceImpl as Trace>::Frame; MAX_DEPTH]>,  // up to 128 frames inline
    pub thread_name: [u8; MAX_THREAD_NAME],  // 16 bytes, fixed
    pub thread_name_length: usize,
    pub thread_id: u64,
    pub sample_timestamp: SystemTime,
}
```

- **Equality**: Based on `thread_id` + all `symbol_address()` values (not instruction pointers).
- **Hash**: Based on all `symbol_address()` values + `thread_id`.
- The `SmallVec` stores frames inline (on stack/in struct) for up to `MAX_DEPTH=128` frames. This is critical: it avoids heap allocation during sampling.

### `Frames` — Symbolicated Sample Data

**File:** `src/frames.rs:167-224`

Created from `UnresolvedFrames` at report time by resolving each frame's address to function name, file, and line number via `backtrace::resolve()`. The signal handler frames (`perf_signal_handler` and its caller) are stripped out during conversion.

---

## Memory Allocation and Management

### What's pre-allocated (before signal handler runs):

| Component | Size | Allocated When |
|---|---|---|
| `HashCounter` buckets | `4096 × sizeof(Bucket)` ≈ 4096 × (8 + Box of 4 entries) | `Collector::new()` in `Profiler::new()` |
| Each `Bucket.entries` | `Box<[Entry<UnresolvedFrames>; 4]>` — 4 entries per bucket | `Bucket::default()` |
| `TempFdArray.buffer` | `Box<[Entry<UnresolvedFrames>; ~127]>` — ~262KB | `TempFdArray::new()` |
| `TempFdArray.file` | `NamedTempFile` — temp file descriptor | `TempFdArray::new()` |
| Blocklist segments | `Vec<(usize, usize)>` | `ProfilerGuardBuilder::blocklist()` |

### What happens during signal handler (per sample):

| Action | Allocation? | Details |
|---|---|---|
| `ErrnoProtector::new()` | None | Stack variable |
| `PROFILER.try_write()` | None | Spinlock, no futex |
| ucontext register read | None | Pointer dereference |
| `is_blocklisted()` | None | Linear scan of Vec |
| `SmallVec::with_capacity(128)` | None | Inline storage on signal stack |
| `TraceImpl::trace()` | **Depends** | frame-pointer: none; backtrace-rs: uses libunwind which may malloc |
| `frame.clone()` / `bt.push()` | None | SmallVec inline push |
| `pthread_self()` | None | TLS read |
| `pthread_getname_np()` | None | Reads from thread control block |
| `UnresolvedFrames::new()` | None | Copies into pre-existing struct |
| `self.data.add(frames, 1)` | None | Writes to pre-allocated HashCounter |
| Eviction → `temp_array.push()` | None (usually) | Writes to pre-allocated buffer; flush_buffer does `write(2)` |

### `SmallVec` Inline Storage

The key allocation avoidance trick: `SmallVec<[Frame; 128]>` stores up to 128 `Frame` objects **inline within the SmallVec struct itself** (which lives on the signal handler's stack). Since `MAX_DEPTH = 128`, the SmallVec never spills to the heap during backtrace collection.

The `Frame` type in frame-pointer mode is:
```rust
pub struct Frame {
    pub ip: usize,  // 8 bytes on 64-bit
}
```

So the SmallVec inline storage is `128 × 8 = 1024 bytes` on the signal handler's stack.

In backtrace-rs mode, `backtrace::Frame` is larger (contains more fields), but still fits in the inline SmallVec.

### Temp File Management

The `NamedTempFile` from the `tempfile` crate creates a file in the system's temp directory. It is automatically deleted when the `TempFdArray` (and thus the `Collector` / `Profiler`) is dropped. Data is written as raw byte representations (no serialization overhead).

---

## Backtrace Collection

### Default Mode: `backtrace-rs` (libunwind)

**File:** `src/backtrace/backtrace_rs.rs`

```rust
impl super::Trace for Trace {
    type Frame = backtrace::Frame;

    fn trace<F: FnMut(&Self::Frame) -> bool>(_: *mut libc::c_void, cb: F) {
        unsafe { backtrace::trace_unsynchronized(cb) }
    }
}
```

- **Ignores the ucontext entirely** — starts unwinding from the current frame (the signal handler itself).
- Uses `backtrace::trace_unsynchronized()` which calls `_Unwind_Backtrace` from libgcc/libunwind.
- **NOT async-signal-safe**: libunwind may call `malloc`, `dl_iterate_phdr`, hold internal locks.
- The signal handler frames are stripped out later during symbolication (in `Frames::from()`).

### Frame-Pointer Mode (feature `frame-pointer`)

**File:** `src/backtrace/frame_pointer.rs`

```rust
impl super::Trace for Trace {
    type Frame = Frame;

    fn trace<F: FnMut(&Self::Frame) -> bool>(ucontext: *mut libc::c_void, mut cb: F) {
        // Extract frame pointer (RBP on x86_64) from ucontext
        let frame_pointer = unsafe {
            (*ucontext).uc_mcontext.gregs[libc::REG_RBP as usize] as usize
        };

        let mut frame_pointer = frame_pointer as *mut FramePointerLayout;
        let mut last_frame_pointer: *mut FramePointerLayout = null_mut();

        loop {
            // Stack grows downward: frame_pointer should increase as we unwind
            if !last_frame_pointer.is_null() && frame_pointer < last_frame_pointer {
                break;
            }
            // Validate the memory address is readable
            if !validate(frame_pointer as *const libc::c_void) {
                break;
            }
            last_frame_pointer = frame_pointer;

            let frame = Frame { ip: unsafe { (*frame_pointer).ret } };
            if !cb(&frame) { break; }
            frame_pointer = unsafe { (*frame_pointer).frame_pointer };
        }
    }
}

#[repr(C)]
struct FramePointerLayout {
    frame_pointer: *mut FramePointerLayout,  // saved RBP (pointer to previous frame)
    ret: usize,                               // return address
}
```

This is the **signal-safe** backtrace method:

1. Reads the frame pointer (RBP) from the `ucontext` — this is the frame pointer of the **interrupted code**, not the signal handler.
2. Follows the frame pointer chain: each frame contains `(prev_frame_pointer, return_address)`.
3. For each frame, validates the memory address using the pipe trick before dereferencing.
4. Stops when: frame pointer goes backward (stack grows down), address validation fails, or `MAX_DEPTH` reached.
5. Uses `_Unwind_FindEnclosingFunction()` for `symbol_address()` resolution (only at report time).

**Stack layout being walked:**

```
High addresses (stack bottom)
┌──────────────────────┐
│ return address        │  ← frame N
│ saved RBP ──────────┐│
├──────────────────────┤│
│ local variables       ││
├──────────────────────┤│
│ return address        ││← frame N-1
│ saved RBP ←──────────┘│
├──────────────────────┤
│ ...                   │
Low addresses (stack top, current RSP)
```

---

## Address Validation (Pipe Trick)

**File:** `src/addr_validate.rs`

This is an elegant async-signal-safe technique to check if a memory address is readable:

```rust
pub fn validate(addr: *const libc::c_void) -> bool {
    // Try to write from the target address into a pipe
    // If the address is invalid, write() returns EFAULT
    let buf = unsafe { std::slice::from_raw_parts(addr as *const u8, CHECK_LENGTH) };
    match write(write_fd, buf) {
        Ok(bytes) => bytes > 0,
        Err(Errno::EINTR) => /* retry */,
        Err(_) => false,    // EFAULT or other error
    }
}
```

**How it works:**

1. A non-blocking pipe is created at startup: `pipe2(O_CLOEXEC | O_NONBLOCK)`
2. To validate an address, call `write(pipe_fd, addr, 16)`:
   - If `addr` is a valid readable address → `write()` succeeds (data goes into pipe)
   - If `addr` is invalid → kernel returns `EFAULT` without crashing
3. The pipe's read end is periodically drained to prevent it from filling up (blocking writes)
4. `CHECK_LENGTH = 2 * sizeof(pointer) = 16 bytes` on 64-bit — validates enough memory to read one `FramePointerLayout` struct

**Why this works in signal handlers:**
- `write(2)` is explicitly listed as async-signal-safe in POSIX
- `read(2)` is explicitly listed as async-signal-safe in POSIX
- The pipe is pre-created, so no allocation or complex setup happens in the signal handler
- `O_NONBLOCK` ensures `write()` never blocks

**Why not use `mincore()` or `/proc/self/maps`?** Those are not async-signal-safe or require `mmap`/file I/O.

---

## Blocklist Mechanism

**File:** `src/profiler.rs:85-119, 383-396`

### Setup (before profiling starts):

```rust
pub fn blocklist<T: AsRef<str>>(self, blocklist: &[T]) -> Self {
    let mut segments = Vec::new();
    TargetSharedLibrary::each(|shlib| {
        if name_matches_blocklist(shlib, blocklist) {
            for seg in shlib.segments() {
                let start = seg.actual_virtual_memory_address(shlib).0;
                let end = start + seg.len();
                segments.push((start, end));
            }
        }
    });
    Self { blocklist_segments: segments, ..self }
}
```

Uses `findshlibs` (which calls `dl_iterate_phdr()`) to enumerate loaded shared libraries and their memory segments. Stores matching segment address ranges as `Vec<(usize, usize)>`.

### Check (in signal handler):

```rust
fn is_blocklisted(&self, addr: usize) -> bool {
    for libs in &self.blocklist_segments {
        if addr > libs.0 && addr < libs.1 {
            return true;
        }
    }
    false
}
```

Simple linear scan of address ranges. Called with the instruction pointer from `ucontext` to skip samples where the interrupted code was in a blocklisted library.

**Purpose:** Prevents stack unwinding when the interrupted code is inside:
- `libc` — malloc/free hold internal locks
- `libgcc` — `_Unwind_Backtrace` holds internal locks
- `pthread` — thread synchronization primitives
- `vdso` — broken DWARF info in some distros

---

## Symbol Resolution

Symbol resolution happens at **report time**, NOT in the signal handler:

**File:** `src/frames.rs:186-222`

```rust
impl From<UnresolvedFrames> for Frames {
    fn from(frames: UnresolvedFrames) -> Self {
        let mut fs = Vec::new();
        let mut frame_iter = frames.frames.iter();

        while let Some(frame) = frame_iter.next() {
            let mut symbols: Vec<Symbol> = Vec::new();
            frame.resolve_symbol(|symbol| {
                symbols.push(Symbol::from(symbol));
            });

            // Strip signal handler frames
            if symbols.iter().any(|s| s.name() == "perf_signal_handler") {
                frame_iter.next(); // skip the frame after the signal handler too
                continue;
            }

            if !symbols.is_empty() {
                fs.push(symbols);
            }
        }
        // ...
    }
}
```

- Each `Frame` stores only an address (IP).
- `resolve_symbol()` calls `backtrace::resolve()` (in default mode) or `backtrace::resolve(ip)` (in frame-pointer mode) to look up DWARF/ELF debug info.
- Demangling is done via the `symbolic-demangle` crate (supports Rust and optionally C++ mangling).
- Signal handler frames (`perf_signal_handler` + one frame above it) are stripped from the output.

---

## Report Generation

**File:** `src/report.rs`

### UnresolvedReport (no symbolication)

```rust
pub fn build_unresolved(&self) -> Result<UnresolvedReport> {
    let mut hash_map = HashMap::new();
    profiler.data.try_iter()?.for_each(|entry| {
        // Merge counts for same UnresolvedFrames
    });
    Ok(UnresolvedReport { data: hash_map, timing })
}
```

### Report (with symbolication)

```rust
pub fn build(&self) -> Result<Report> {
    let mut hash_map = HashMap::new();
    profiler.data.try_iter()?.for_each(|entry| {
        let key = Frames::from(entry.item.clone());  // symbolicate
        // Merge counts for same Frames
    });
    Ok(Report { data: hash_map, timing })
}
```

The report builder acquires the profiler's write lock, which means:
- While building a report, the signal handler's `try_write()` will fail → samples are dropped.
- This is acceptable: report building is a rare operation.

### Output Formats

- **Debug format**: Human-readable `FRAME: ... -> FRAME: ... THREAD: name count`
- **Flamegraph**: Via `inferno` crate → SVG
- **pprof protobuf**: Via `prost` or `protobuf` crate → `profile.proto` format

---

## Known Limitations and Design Tradeoffs

### 1. Default backtrace (libunwind) is NOT signal-safe

The default `backtrace-rs` mode uses `_Unwind_Backtrace` from libgcc, which:
- Calls `dl_iterate_phdr()` which takes a global lock
- May call `malloc()`
- Can deadlock if the interrupted code was inside `malloc` or `dl_iterate_phdr`

**Mitigation:** The blocklist mechanism skips samples when interrupted inside these libraries. This creates a sampling bias (underrepresents time spent in libc/libgcc) but avoids deadlocks.

### 2. `ITIMER_PROF` limitations

- Per-process timer, not per-thread. On Linux, this works okay because the kernel delivers SIGPROF to the thread that was running.
- Only counts CPU time — misses I/O-blocked threads.
- Deprecated in POSIX in favor of `timer_create(CLOCK_PROCESS_CPUTIME_ID)` + `timer_settime()`.

### 3. Frame-pointer mode requires recompilation

The `frame-pointer` feature requires all code (including the standard library) to be compiled with frame pointers. This means using `cargo +nightly -Z build-std` and appropriate `-C force-frame-pointers=yes` flags.

### 4. Hash map eviction loses precision

When the `HashCounter` is full, low-count entries are evicted to the temp file. At report time, evicted entries with the same key are merged. However, entries evicted at different times may not be merged correctly if one version was in the hash map and another in the temp file.

Actually, looking at the code more carefully: `try_iter()` chains both sources, and `ReportBuilder::build()` merges them into a `HashMap`. So the counts ARE correctly aggregated at report time. The eviction only affects the signal handler's hot path — it doesn't lose data.

### 5. Single global profiler

The `PROFILER` is a `Lazy<RwLock<Result<Profiler>>>` — there can only be one profiler instance. This means you can't run multiple profilers with different configurations simultaneously.

### 6. `parking_lot::RwLock` signal safety

pprof-rs relies on `parking_lot`'s `try_write()` being safe to call from a signal handler. `parking_lot` uses atomics for the fast path (CAS on the lock state), but the slow path (when contention occurs) may use `futex` on Linux. Since `try_write()` should never enter the slow path (it returns `None` immediately on contention), this is safe.

### 7. No SA_SIGINFO handler chaining

The original `SIGPROF` handler is not saved and restored. Any pre-existing profiler or signal handler for `SIGPROF` will be overwritten.

---

## Relevance for Python CPU Profiler Design

Key takeaways for designing a Python CPU profiler:

### What to adopt:

1. **`setitimer(ITIMER_PROF)` + `SIGPROF`** — Works for CPU profiling. For wall-clock profiling, use `ITIMER_REAL` + `SIGALRM` or `timer_create(CLOCK_MONOTONIC)`.

2. **`try_lock` pattern in signal handler** — Essential for avoiding deadlocks. Never block in a signal handler.

3. **Pre-allocated fixed-size hash map** — The set-associative design with eviction to disk is clever. For Python, we might want a simpler approach since we don't need to worry about Rust's ownership model.

4. **Pipe-based address validation** — Useful if doing frame-pointer walking. For Python, we might use the interpreter's own frame chain instead.

5. **Blocklist mechanism** — For Python, this maps to skipping samples when the interpreter is inside C extension code that holds the GIL in a state where stack walking isn't safe.

6. **Errno preservation** — Must do this in any C signal handler.

7. **`SmallVec` inline storage pattern** — For Python/C, use fixed-size stack arrays in the signal handler.

### What's different for Python:

1. **Stack walking**: Python has its own frame chain (`PyFrameObject` linked list / `_PyInterpreterFrame`), so we don't need libunwind or frame pointers for Python frames. We need to walk the Python frame chain, which is signal-safe IF the interpreter state is consistent.

2. **Thread enumeration**: Python has `PyInterpreterState` which tracks all threads. We might want to sample all threads from a single signal handler using `_PyRuntimeState`.

3. **GIL interaction**: The GIL is the main concern for Python signal safety. `PyGILState_Check()` or equivalent to verify we can safely access Python state.

4. **Symbol resolution**: Python function names are available directly from `PyCodeObject` — no DWARF/ELF symbolication needed. However, we need to handle native (C extension) frames separately.

5. **`timer_create` + `SIGRTMIN`**: Modern alternative to `setitimer`. Allows per-thread timers and custom signal numbers (avoids conflicting with Python's own `SIGPROF` usage).
