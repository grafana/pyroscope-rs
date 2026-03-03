# Signal-Based CPU Profiler for Python — Unified Design Document

## Preface: Review of Prior Proposals

This document synthesizes the best ideas from three prior design proposals (designs/2.md, 3.md, 4.md), the hackathon-profiler prototype, and reference profiler implementations (async-profiler 3.0, gperftools, pprof-rs, Go 1.26 runtime). Each proposal was reviewed for correctness, completeness, and practicality. Key issues found:

**Proposal 2** ("The Detailed One"): Best overall structure and technical depth. Correct frame_owner handling (skips only CSTACK frames). Good sharding design. Issues: reads `co_code_adaptive` per frame in the handler unnecessarily; errno save/restore via libc calls; over-engineered TimerBackend trait.

**Proposal 3** ("The Practical One"): Good errno save/restore awareness, practical reset protocol using sigprocmask. Issues: fixed-size 4KB ring buffer entries (wasteful); wrong frame_owner handling (stops at generators); no sharding (single global SpinMutex); fragile TLS access via autoTSSkey formula; errno save/restore calls __errno_location() which is itself a libc function.

**Proposal 4** ("The Modular One"): Most decomposed crate structure; raw syscall wrappers; double-buffer idea for hash map reset. Issues: wrong frame_owner handling; redundant nanolibc vs libc; too many crates without clear benefit.

**Common errors**: Proposals 3 and 4 incorrectly stop frame walking at generator frames (owner != 0). Only `FRAME_OWNED_BY_CSTACK` (3) should be skipped — generators (1) and frame-object-backed frames (2) contain valid Python frames. None properly address the 3.14 `_PyInterpreterFrame` layout changes from 3.13.

---

## Table of Contents

1. [Goals and Non-Goals](#1-goals-and-non-goals)
2. [Architecture Overview](#2-architecture-overview)
3. [Crate Layout](#3-crate-layout)
4. [OS APIs and Syscalls](#4-os-apis-and-syscalls)
5. [Initialization and Discovery](#5-initialization-and-discovery)
6. [Signal Handler](#6-signal-handler)
7. [Python Stack Unwinding](#7-python-stack-unwinding)
8. [Async-Signal-Safe Primitives](#8-async-signal-safe-primitives)
9. [Collection: Lock-Free Ring Buffer](#9-collection-lock-free-ring-buffer)
10. [Reader Thread and Periodic Flush](#10-reader-thread-and-periodic-flush)
11. [Symbolication](#11-symbolication)
12. [Pprof Generation](#12-pprof-generation)
13. [Pyroscope Ingestion](#13-pyroscope-ingestion)
14. [Shared Library Interface](#14-shared-library-interface)
15. [Concurrency Model](#15-concurrency-model)
16. [Memory Management](#16-memory-management)
17. [Error Handling](#17-error-handling)
18. [Future Work](#18-future-work)
19. [Appendix: CPython 3.14 Structures](#appendix-cpython-314-structures)

---

## 1. Goals and Non-Goals

### Goals

- **In-process, signal-based CPU profiler** for CPython, written in Rust.
- **`setitimer(ITIMER_PROF)` + `SIGPROF`** delivering a signal every **10ms of CPU time** (100 Hz). Design allows migration to per-thread `timer_create(CLOCK_THREAD_CPUTIME_ID)` later.
- **Fully async-signal-safe signal handler**: no malloc/free, **no libc function calls whatsoever**, no /proc reads, no syscalls beyond `gettid`. All code executing in signal handler context must be `#![no_std]` and must not depend on `libc` — use raw syscall wrappers or inline assembly instead. Memory is pre-allocated via raw `mmap` syscall. Spinlocks (from the `spin` crate) are try-lock only in the handler; full lock is used by the reader thread during dump.
- **Lock-free ring buffer** (`kit/sig_ring`) for passing raw stack traces from signal handler to reader thread. The handler appends variable-length records; every N samples it notifies the reader thread via an `eventfd` to process batches. The reader also drains the full buffer on every 15s flush. All mmap-backed, pre-allocated.
- **No CPython API calls, no linking against libpython** — discover everything by inspecting process memory as a debugger would. Read `/proc/self/maps`, parse ELF, use `_Py_DebugOffsets` from `_PyRuntime`.
- **Python 3.14 initially** — all offsets in a separate structure. Use `_Py_DebugOffsets` introspection to read as many offsets as possible; hardcode only what cannot be read.
- **Minimal handler work** — only unwind Python frames and record raw `(PyCodeObject*, instr_offset)` tuples. No string reads, no symbol resolution, no filename/line extraction in the handler.
- **Periodic flush** — every 15 seconds: drain collected samples, symbolize, build a pprof protobuf, HTTP POST it to a Pyroscope instance at `localhost:4040`.
- **Loadable via `dlopen`** — the profiler is a cdylib (`.so`). A single `extern "C"` function is called from Python via `ctypes`/`cffi` to start profiling. The profiler runs for the lifetime of the process — there is no stop function.
- **Reusable crates** — new crates must not depend on any existing workspace crates except `kindasafe` and `kindasafe_init`. They should be generic enough to reuse for Ruby/dotnet profilers later.
- **Small crates, minimal dependencies** — each crate does one thing and has as few external dependencies as possible.

### Non-Goals (Explicit Deferrals)

- Native/C/C++ frame unwinding — added later.
- Line numbers and filenames in the handler — symbolized later from code object pointers.
- Multiple Python version support in this iteration — 3.14 only, but offset table is version-parameterized.
- Free-threaded (no-GIL) Python support — future work.
- ARM64 support — x86_64 only initially, but keep architecture-specific code isolated.
- Production-grade error reporting, configuration API, or Python-level API.

---

## 2. Architecture Overview

### Data Flow

```
                              KERNEL
                                │
                 setitimer(ITIMER_PROF, 10ms)
                                │
                          SIGPROF delivery
                                │
                                ▼
                    ┌───────────────────────┐
                    │   Signal Handler      │
                    │                       │
                    │  1. [debug] Save errno│
                    │  2. Try-lock shard    │
                    │  3. Read TLS → find   │
                    │     PyThreadState     │
                    │  4. Walk Python       │
                    │     frame chain       │
                    │  5. Record raw stack  │
                    │     to collector      │
                    │  6. Unlock shard      │
                    │  7. [debug] Assert    │
                    │     errno unchanged   │
                    └──────────┬────────────┘
                               │
                               ▼
                  ┌────────────────────────┐
                  │  Lock-Free Ring Buffer  │
                  │  (kit/sig_ring)         │
                  │  per-shard SPSC         │
                  │  variable-length records│
                  │  mmap-backed            │
                  └──────────┬─────────────┘
                             │
                     eventfd notify every
                     N samples + 15s timer
                             │
                             ▼
                   ┌──────────────────────┐
                   │  Reader Thread        │
                   │  (kit/profiler_core)  │
                   │                       │
                   │  Wakes on:            │
                   │  - eventfd (batch)    │
                   │  - 15s timeout (flush)│
                   │                       │
                   │  1. Drain ring bufs   │
                   │  2. Aggregate stacks  │
                   │  3. Symbolize         │
                   │  4. Build pprof proto │
                   │  5. HTTP POST to      │
                   │     Pyroscope (on 15s)│
                   └──────────────────────┘
```

### Component Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                      pyroscope-rs workspace                         │
│                                                                     │
│  kit/                                                               │
│  ├── kindasafe/         (SPLIT FROM EXISTING) `#![no_std]` SIGSEGV- │
│  │                      safe memory reads — naked asm, no libc      │
│  ├── kindasafe_init/    (SPLIT FROM EXISTING) kindasafe init —      │
│  │                      sigaction-based SIGSEGV/SIGBUS recovery     │
│  │                      setup, depends on libc, NOT `#![no_std]`    │
│  ├── notlibc/        Async-signal-safe primitives                │
│  │                      raw mmap, eventfd, raw syscall helpers       │
│  ├── sig_ring/          Lock-free SPSC ring buffer per shard        │
│  ├── sighandler/        Signal registration, setitimer, timer mgmt  │
│  ├── python_offsets/    CPython version detection, _Py_DebugOffsets, │
│  │                      ELF symbol lookup, /proc/self/maps parsing  │
│  ├── python_unwind/     Signal-safe Python frame walking            │
│  ├── profiler_core/     Orchestration: reader thread, flush, symbol │
│  ├── pprof_enc/         Minimal pprof protobuf encoder              │
│  ├── pyroscope_ingest/  HTTP POST to Pyroscope push API             │
│  └── pyroscope_cpython/ cdylib: dlopen entry point                  │
│                                                                     │
│  (existing crates — NOT depended upon by new crates)                │
│  ├── pyroscope/         Existing agent (not used)                   │
│  └── pyroscope_ffi/     Existing Python/Ruby FFI (not used)         │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Crate Layout

Each crate has minimal dependencies. The dependency graph flows in one direction:

```
                     pyroscope_cpython (cdylib)
                             │
                ┌────────────┼─────────────┐
                ▼            ▼              ▼
         profiler_core   sighandler    python_offsets
             │  │  │        │              │
   ┌────┬───┘  │  └──┐     │         ┌────┘
   ▼    ▼      ▼     ▼     ▼         ▼
pprof  │  pyroscope  python     kindasafe_init ──→ kindasafe
_enc   │  _ingest    _unwind                        (#![no_std])
       │                │
       ▼           ┌────┤
     sig_ring   kindasafe  notlibc
       │        (#![no_std])
       ▼
    notlibc
```

**Note on kindasafe split**: The existing `kindasafe` crate is a single crate that depends on `libc` and `spin`, mixing signal-handler-safe read primitives (naked asm) with init-time code (`sigaction` installation, fallback handler chaining). It must be split into two crates before other signal-handler-path crates can depend on it:

- **`kindasafe`** (after split): `#![no_std]`, zero external dependencies. Contains only the naked-asm read primitives (`u64`, `slice`, `str`, `fs_0x10`), the crash-point table, and the `crash_handler` function. The `crash_handler` currently references `libc::ucontext_t` for register access — this must be replaced with raw pointer arithmetic at known offsets (the `ucontext_t` layout is stable on Linux x86_64). No libc dependency after split.
- **`kindasafe_init`** (new crate): depends on `kindasafe`, `libc`, `spin`. Contains `init()`, `is_initialized()`, SIGSEGV/SIGBUS signal handler installation via `sigaction`, fallback handler chaining (`FALLBACK_SIGSEGV`/`FALLBACK_SIGBUS` statics), and `restore_default_signal_handler()`.

Signal-handler-path crates (`python_unwind`, `sig_ring`, etc.) depend on `kindasafe` (no_std). Init-time crates (`python_offsets`, `profiler_core`) depend on `kindasafe_init`.

### Crate Descriptions

| Crate | Type | Dependencies | Purpose |
|-------|------|-------------|---------|
| `kit/kindasafe` | lib (`#![no_std]`) | (none) | **Split from existing `kindasafe` crate.** SIGSEGV-safe memory read primitives: `u64()`, `slice()`, `str()`, `fs_0x10()` via naked assembly. Crash-point table and crash handler (manipulates `ucontext_t` via raw pointer offsets to skip faulting instructions). Currently the crash handler uses `libc::ucontext_t` types — these must be replaced with raw pointer arithmetic at fixed offsets for `#![no_std]`. **No libc dependency, no `std`** — uses only `core` and naked asm. Safe for use in signal handler context. |
| `kit/kindasafe_init` | lib | `kindasafe`, `libc`, `spin` | **Split from existing `kindasafe` crate.** Initialization for kindasafe: installs SIGSEGV/SIGBUS signal handlers via `sigaction` that delegate to `kindasafe::arch::crash_handler` for recovery. Manages fallback handler chaining (`FALLBACK_SIGSEGV`/`FALLBACK_SIGBUS` statics). Provides `init()` and `is_initialized()`. **Not `#![no_std]`** — depends on `libc` for `sigaction` and `ucontext_t` types. Called once at profiler startup, never from signal handler context. |
| `kit/notlibc` | lib (`#![no_std]`) | `spin` | Async-signal-safe primitives: raw `mmap`/`munmap` wrappers (via raw `syscall` instruction — no libc), `eventfd` wrapper. Re-exports `spin::Mutex` for shard locking. **No libc dependency.** Uses inline assembly for syscalls. |
| `kit/sig_ring` | lib (`#![no_std]`) | `notlibc` | Lock-free SPSC ring buffer with variable-length records. One per shard. Pre-allocated via mmap. Writer is signal handler, reader is background thread. Supports eventfd notification. **No libc dependency.** |
| `kit/sighandler` | lib | `libc` | Signal handler registration (`sigaction`), `setitimer` wrapper. Generic — not Python-specific. Note: this crate uses `libc` for `sigaction`/`setitimer` calls at **init time only** — it is never called from within the signal handler. |
| `kit/python_offsets` | lib | `kindasafe_init`, `libc` | CPython version detection, `_Py_DebugOffsets` reading, offset table structures. ELF symbol lookup for `_PyRuntime` and `Py_Version`. `/proc/self/maps` parsing. TLS offset discovery. All done at **init time only** — not in signal handler. Uses `kindasafe` reads (via `kindasafe_init`) for safe memory access during discovery. |
| `kit/python_unwind` | lib (`#![no_std]`) | `kindasafe`, `notlibc` | Signal-safe Python frame walking. Reads `PyThreadState.current_frame` → `_PyInterpreterFrame` chain. Outputs `(PyCodeObject*, instr_offset)` tuples. **No libc dependency** — depends on `kindasafe` (the `#![no_std]` read crate), runs in signal handler context. |
| `kit/profiler_core` | lib | `sig_ring`, `python_unwind`, `python_offsets`, `pprof_enc`, `pyroscope_ingest`, `sighandler` | Orchestrator: owns the reader thread, periodic flush, stack aggregation, symbolication, pprof building, ingestion. |
| `kit/pprof_enc` | lib | `prost` | Minimal pprof protobuf encoder. String table, Function, Location, Sample messages. Gzip output via `flate2`. |
| `kit/pyroscope_ingest` | lib | `ureq` or `minreq` | HTTP POST of gzipped pprof to `{base_url}/ingest` with query parameters. |
| `kit/pyroscope_cpython` | cdylib | `profiler_core` | Exposes `extern "C" pyroscope_start(config)`. The `.so` that gets `dlopen`'d. No stop — profiler runs for process lifetime. |

### Dependency Policy

- New crates **must not** depend on the existing `pyroscope` crate, `py-spy`, `rbspy`, or any existing workspace member except `kindasafe` and `kindasafe_init`.
- External dependencies must be minimal:
  - `spin` — `#![no_std]` spinlock (in `notlibc`, `kindasafe_init`).
  - `libc` — **only for init-time crates** (`kindasafe_init`, `sighandler`, `python_offsets`) that call `sigaction`/`setitimer`/file I/O. Never in signal-handler-path crates.
  - `prost` — protobuf encoding (in `pprof_enc` only).
  - `flate2` — gzip compression (in `pprof_enc` only).
  - `ureq` or `minreq` — HTTP client (in `pyroscope_ingest` only).
- **Signal-handler-path crates** (`kindasafe`, `notlibc`, `sig_ring`, `python_unwind`) **must be `#![no_std]` and must NOT depend on `libc`**. They use raw `syscall` instructions via inline assembly or naked asm for any OS interaction (e.g., `mmap`, `gettid`, memory reads). This ensures no libc function can ever be called from within the signal handler, regardless of interposition.

---

## 4. OS APIs and Syscalls

### 4.1 Signal Registration — `sigaction(2)`

**Crate:** `kit/sighandler`

```
sigaction(SIGPROF, &new_action, &old_action)

new_action:
  sa_sigaction = our_handler    (3-arg handler: sig, siginfo_t*, ucontext_t*)
  sa_flags     = SA_SIGINFO | SA_RESTART
  sa_mask      = {SIGPROF, SIGSEGV, SIGBUS}
```

- `SA_SIGINFO` — we need the 3-argument handler form to receive `ucontext_t*`.
- `SA_RESTART` — interrupted syscalls restart automatically so the profiler doesn't break the profiled application.
- `sa_mask = {SIGPROF, SIGSEGV, SIGBUS}` — signals blocked during handler execution:
  - `SIGPROF` — prevents re-entrant handler invocation on the same thread. If a signal is delivered while the handler is running, it would corrupt the per-shard frame buffer.
  - `SIGSEGV`, `SIGBUS` — the profiler signal handler must NOT be delivered while the kernel's SIGSEGV/SIGBUS handler is executing. `kindasafe_init` installs its own SIGSEGV/SIGBUS handlers for memory read recovery; if SIGPROF fires during that recovery, the profiler handler would run in a corrupted signal context. By masking SIGSEGV/SIGBUS, we ensure our handler never runs while a fault handler is active on the same thread.
- We do NOT use `SA_ONSTACK` initially — Python threads have sufficiently large stacks (default 8MB). Our handler uses minimal stack space (a few hundred bytes for local variables, pointers, and the MutexGuard) because the frame buffer lives in the locked shard, not on the signal handler's stack.

### 4.2 Timer Setup — `setitimer(2)`

**Crate:** `kit/sighandler`

```
setitimer(ITIMER_PROF, &interval, NULL)

interval:
  it_value    = { 0, 10000 }   // 10ms = 10000 µs
  it_interval = { 0, 10000 }   // repeating
```

- `ITIMER_PROF` counts CPU time (user + system) consumed by the **process**. The kernel delivers SIGPROF to whichever thread was executing when the timer expired.
- 10ms interval = 100 Hz sampling rate.
- The timer runs for the lifetime of the process — there is no stop.

### 4.3 Future: Per-Thread Timers — `timer_create(2)`

The design isolates timer management so that `setitimer` can be replaced with per-thread timers later. The `sighandler` crate exposes a simple `start_timer` function rather than a trait — traits add complexity without benefit when only one implementation exists. When per-thread timers are needed, refactor at that time.

Per-thread timers require:
- `timer_create(CLOCK_THREAD_CPUTIME_ID)` with `SIGEV_THREAD_ID` + `SIGEV_SIGNAL = SIGPROF`.
- Thread enumeration via `/proc/self/task/` at startup.
- Periodic poll or hook to discover new threads.
- Randomized initial delay (`random_in(0..10ms)`) to avoid synchronization artifacts (pattern from Go runtime).
- A timer registry mapping TID → timer_id.

### 4.4 Memory Allocation — Raw `mmap` Syscall

**Crate:** `kit/notlibc`

All memory used by signal-handler data structures is allocated via **raw `syscall` instruction**, NOT via libc's `mmap` wrapper (which can be interposed by jemalloc, tcmalloc, ASan, etc.):

```
// Raw syscall instruction via inline assembly — no libc involved
// Equivalent to: syscall(SYS_mmap, NULL, size, PROT_READ | PROT_WRITE,
//                        MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)
```

These are wrapped in `notlibc::safe_mmap(size) -> *mut u8` and `notlibc::safe_munmap(addr, size)`, which use inline assembly to invoke the `mmap`/`munmap` syscalls directly without any libc dependency.

The signal handler itself **never calls mmap** — all memory is pre-allocated at profiler start.

### 4.5 Process Memory Introspection

| Operation | API | Crate | Context |
|-----------|-----|-------|---------|
| Read memory safely | `kindasafe::u64()`, `kindasafe::slice()` | `kindasafe` | Signal handler + init |
| Read FS segment base (TLS) | `kindasafe::fs_0x10()` | `kindasafe` | Signal handler |
| Parse `/proc/self/maps` | `std::fs::read_to_string` | `python_offsets` | Init only |
| Read ELF headers/symbols | `kindasafe::slice()` on mapped regions | `python_offsets` | Init only |
| Read `_Py_DebugOffsets` | `kindasafe::slice()` | `python_offsets` | Init only |

### 4.6 Thread Identity

In the signal handler, we identify the current thread via `gettid()` (raw `syscall` instruction via inline assembly — not the libc `gettid()` wrapper). This gives the Linux thread ID, used for:
- Shard selection: `shard_index = tid % NUM_SHARDS`
- Thread identification in sample records

### 4.7 Syscall Summary

| Syscall | Where Called | Signal-Safe | Purpose |
|---------|-------------|-------------|---------|
| `sigaction` | Init | N/A | Register signal handler |
| `setitimer` | Init | N/A | Start process-wide timer |
| `SYS_mmap` | Init (pre-alloc) | Yes (raw) | Allocate ring buffers, shard state |
| `SYS_eventfd2` | Init | N/A | Create eventfd for reader notification |
| `SYS_write` | Signal handler (every N samples) | Yes (raw) | Write 1 to eventfd to wake reader |
| `SYS_gettid` | Signal handler | Yes | Thread identification + shard selection |
| `read` | Init only | N/A | Read /proc/self/maps |
| `open` | Init only | N/A | Open /proc/self/maps |
| `write` | Reader thread | N/A | HTTP POST (not in handler) |

---

## 5. Initialization and Discovery

### 5.1 Overview

Before the signal handler can unwind Python stacks, we must discover:
1. Where `_PyRuntime` is in memory.
2. The Python version.
3. Struct field offsets (from `_Py_DebugOffsets` + hardcoded fallbacks).
4. How to find the current thread's `PyThreadState` (TLS offset).

All discovery happens at profiler initialization time (not in the signal handler) and can use `std`, allocate freely, etc.

### 5.2 Finding Python in Memory

**Crate:** `kit/python_offsets`

Parse `/proc/self/maps` to find memory regions mapped from the Python binary or `libpython`:

```
Pattern matching on pathname:
  - "libpython3" in the filename → prefer this (library has dynamic symbols)
  - "python3" in the filename → fallback (static binary)
  - Look for the first executable mapping (r-xp) of this file
```

Record the base address and path. The base address is needed to compute ASLR-adjusted symbol addresses.

### 5.3 ELF Symbol Resolution

**Crate:** `kit/python_offsets`

From the mapped Python binary, parse ELF headers to find these symbols:

| Symbol | Purpose |
|--------|---------|
| `_PyRuntime` | Runtime state — `_Py_DebugOffsets` is at offset 0 |
| `Py_Version` | Version integer — `(major << 24) | (minor << 16) | (micro << 8) | release` |

Procedure:
1. Read the ELF header from the base address of the mapped region (the binary is already mapped by the OS loader).
2. Locate the `.dynsym` and `.dynstr` sections via the dynamic segment (`PT_DYNAMIC`).
3. Iterate dynamic symbols looking for `_PyRuntime` and `Py_Version`.
4. Compute absolute addresses: `symbol_value + load_bias` (where `load_bias = mapped_base - first_LOAD_segment_vaddr`).

We read the ELF from already-mapped memory using `kindasafe::slice()`, avoiding additional file I/O.

### 5.4 Version Detection

**Crate:** `kit/python_offsets`

```
version_hex = kindasafe::u64(py_version_addr) & 0xFFFFFFFF
major = (version_hex >> 24) & 0xFF
minor = (version_hex >> 16) & 0xFF
```

Verify: `major == 3`, `minor == 14`. Return error for unsupported versions.

### 5.5 Reading `_Py_DebugOffsets`

**Crate:** `kit/python_offsets`

`_Py_DebugOffsets` is located at the very beginning of `_PyRuntime`. It provides offsets for navigating CPython's internal structures without hardcoding them.

#### Cookie and Version Validation

```
offset 0:  cookie[8]         — must equal "xdebugpy" (0x7970677562656478 LE)
offset 8:  version (u64)     — must match Py_Version
offset 16: free_threaded(u64) — must be 0 (we don't support free-threaded yet)
```

#### Offsets We Read From `_Py_DebugOffsets`

**Runtime state** (offsets within `_PyRuntime`):

| Field | Purpose |
|-------|---------|
| `runtime_state.interpreters_head` | Offset to `interpreters.head` — first `PyInterpreterState*` |

**Interpreter state** (offsets within `PyInterpreterState`):

| Field | Purpose |
|-------|---------|
| `interpreter_state.threads_head` | Offset to `threads.head` — first `PyThreadState*` |

**Thread state** (offsets within `PyThreadState`):

| Field | Purpose |
|-------|---------|
| `thread_state.current_frame` | Active `_PyInterpreterFrame*` |
| `thread_state.native_thread_id` | OS thread ID for matching |
| `thread_state.next` | Next thread in linked list |
| `thread_state.thread_id` | Python thread ID |

**Interpreter frame** (offsets within `_PyInterpreterFrame`):

| Field | Purpose |
|-------|---------|
| `interpreter_frame.previous` | Caller frame |
| `interpreter_frame.executable` | The `PyCodeObject*` |
| `interpreter_frame.instr_ptr` | Current bytecode instruction pointer (`_Py_CODEUNIT*`) |
| `interpreter_frame.owner` | Frame owner enum (u8) |

**Code object** (offsets within `PyCodeObject`):

| Field | Purpose |
|-------|---------|
| `code_object.qualname` | `co_qualname` — qualified function name (for symbolication) |
| `code_object.name` | `co_name` — function name (fallback) |
| `code_object.filename` | `co_filename` — source file path (for symbolication) |
| `code_object.firstlineno` | `co_firstlineno` — first line number |
| `code_object.co_code_adaptive` | Offset to bytecode array start (for instruction offset) |

**Unicode object** (for symbolication):

| Field | Purpose |
|-------|---------|
| `unicode_object.asciiobject_size` | Size of `PyASCIIObject` — data follows immediately after |

#### Offset Table Structure

```
PythonOffsets {
    // Runtime navigation
    py_runtime_addr: u64,                // Absolute address of _PyRuntime
    runtime_interpreters_head: u64,      // _PyRuntime + this → interp*
    interp_threads_head: u64,            // interp + this → tstate*

    // Thread state navigation
    tstate_current_frame: u64,
    tstate_native_thread_id: u64,
    tstate_next: u64,
    tstate_thread_id: u64,

    // Frame navigation
    frame_previous: u64,
    frame_executable: u64,               // → PyCodeObject*
    frame_instr_ptr: u64,                // → _Py_CODEUNIT*
    frame_owner: u64,                    // → u8 enum

    // Code object (for symbolication — NOT used in handler)
    code_qualname: u64,
    code_name: u64,
    code_filename: u64,
    code_firstlineno: u64,
    code_co_code_adaptive: u64,          // → start of bytecode array

    // Unicode (for symbolication)
    unicode_asciiobject_size: u64,       // data starts at this offset from object

    // TLS access
    tls_offset: u64,                     // FS-relative offset for _Py_tss_tstate
    tls_method: TlsMethod,              // which method we're using
}

enum TlsMethod {
    StaticTls { fs_offset: u64 },        // Direct FS-relative access (preferred)
    ThreadListWalk,                      // Walk interpreter thread list (fallback)
}
```

This struct is populated once at init time and then shared (read-only) with the signal handler via a global atomic pointer.

### 5.6 TLS Offset Discovery (Finding PyThreadState in Signal Handler)

**Crate:** `kit/python_offsets`

The signal handler needs O(1) access to the current thread's `PyThreadState`. We use TLS-based access.

#### Primary Approach: Disassemble `_PyThreadState_GetCurrent`

CPython 3.14 stores the current thread state in a compiler-level `__thread` variable (`_Py_tss_tstate`). The function `_PyThreadState_GetCurrent` reads it:

1. Find the address of `_PyThreadState_GetCurrent` from ELF `.dynsym`.
2. Read the first N bytes of the function (typically 5-10 bytes).
3. Decode x86_64 instructions to find an FS-relative load:
   - Pattern: `64 48 8b 04 25 XX XX XX XX` (mov rax, fs:disp32)
   - Or: `64 48 8b 05 XX XX XX XX` (mov rax, fs:[rip+disp32])
   - Extract the displacement value — this is the static TLS offset.
4. At runtime in the signal handler:
   ```
   tstate = kindasafe::u64(fs_base + tls_offset)
   ```
   where `fs_base` = the thread's TLS base (obtained via `arch_prctl(ARCH_GET_FS)` or reading `fs:0x0` at init for validation).

This approach is robust because it doesn't depend on pthread implementation details or autoTSSkey.

#### Fallback Approach: Walk the Thread List

If disassembly fails, fall back to walking the interpreter's thread list:

```
interp = read_u64(py_runtime_addr + offsets.runtime_interpreters_head)
tstate = read_u64(interp + offsets.interp_threads_head)
my_tid = gettid()
while tstate != 0 {
    tid = read_u64(tstate + offsets.tstate_native_thread_id)
    if tid == my_tid { return tstate }
    tstate = read_u64(tstate + offsets.tstate_next)
}
return None
```

This is O(n) in the number of threads but uses `kindasafe::u64()` for safety.

#### Validation

At init time, verify the discovered TLS offset by reading the current thread's `PyThreadState` and checking:
- The value is non-null.
- The `native_thread_id` field matches `gettid()`.

### 5.7 Init Sequence Summary

```
pyroscope_start(config):
  1. kindasafe_init::init()                   — install SIGSEGV/SIGBUS recovery
  2. python_offsets::discover_python()       — parse /proc/self/maps
  3. python_offsets::read_elf_symbols()      — find _PyRuntime, Py_Version
  4. python_offsets::check_version()         — verify Python 3.14
  5. python_offsets::read_debug_offsets()    — read _Py_DebugOffsets from _PyRuntime
  6. python_offsets::discover_tls()          — disassemble _PyThreadState_GetCurrent
  7. Construct PythonOffsets struct
  8. Pre-allocate ring buffers (16 shards × 256 KiB via safe_mmap) + create eventfd
  9. Pre-allocate per-shard frame buffers
  10. Publish PythonOffsets + collector to global state (atomic pointer store with Release)
  11. Spawn reader thread
  12. sighandler::install_handler(SIGPROF)   — register signal handler
  13. sighandler::start_timer(10ms)          — arm setitimer
```

**Important**: The signal handler is installed AFTER all data structures are allocated and published. The timer is started AFTER the handler is installed. This ordering ensures the handler never runs against uninitialized state.

---

## 6. Signal Handler

### 6.1 Handler Signature

**Crate:** `kit/sighandler` (generic registration) + `kit/pyroscope_cpython` (Python-specific callback)

```
extern "C" fn signal_handler(
    sig: c_int,
    info: *mut siginfo_t,
    ucontext: *mut c_void,
)
```

### 6.2 Handler Flow

```
signal_handler(sig, info, ucontext):
  │
  ├─ 1. Load global profiler state (atomic Acquire load)
  │     If NULL → return
  │
  ├─ 3. Compute shard index: shard = gettid() % NUM_SHARDS
  │
  ├─ 4. Try-lock shard: spin::Mutex::try_lock(&shards[shard])
  │     On fail → try (shard+1) % N, then (shard+2) % N
  │     If all 3 fail → increment drop counter, return
  │
  ├─ 5. Find PyThreadState via TLS:
  │     [StaticTls]: tstate = kindasafe::u64(fs_base + tls_offset)
  │     [ThreadListWalk]: walk interpreter thread list matching gettid()
  │     If tstate == 0 or read failed → unlock, return
  │
  ├─ 6. Unwind Python stack:
  │     python_unwind::unwind(tstate, &offsets, &mut shard.frame_buffer)
  │     → walks _PyInterpreterFrame chain
  │     → fills frame_buffer with RawFrame structs
  │     → stops at max depth (128) or NULL/CSTACK frame
  │
  ├─ 7. Append stack to per-shard ring buffer:
  │     shard.ring_buffer.write(tid, shard.frame_buffer, depth)
  │     → writes variable-length record (header + frames)
  │     → if ring buffer full: increment overflow counter, skip
  │
  ├─ 8. Increment samples_collected counter
  │     If samples_collected % N == 0:
  │       notify reader via eventfd (raw write(eventfd, &1u64, 8))
  │
  └─ 9. Unlock shard (MutexGuard drop)
         return
```

### 6.3 Async-Signal-Safety Guarantees

Every operation in the handler is async-signal-safe. **No libc functions are called — the entire handler path is `#![no_std]`.**

| Operation | How it's safe |
|-----------|---------------|
| Atomic load of global state | `AtomicPtr::load(Acquire)` — lock-free, no libc |
| `gettid()` | Raw `syscall` instruction via inline asm — no libc |
| `spin::Mutex::try_lock()` | Single `compare_exchange` — no libc, never blocks |
| `kindasafe::u64()` / `fs_0x10()` | Naked assembly with SIGSEGV recovery — no libc |
| Ring buffer write | Atomic store on pre-allocated mmap'd memory — no libc |
| eventfd notify | Raw `write` syscall via inline asm (8 bytes to fd) — no libc |
| Atomic counter increment | `fetch_add(Relaxed)` — no libc |
| `spin::MutexGuard::drop()` | `store(Release)` — no libc |
**What the handler does NOT do:**
- No **libc function calls at all** — not `memcpy`, not `__errno_location`, not `gettid()` via libc, nothing. Every operation is either a Rust core/atomic intrinsic, inline assembly, or a raw `syscall` instruction.
- No `malloc` / `free` / `Box` / `Vec` / any Rust allocating type
- No `pthread_mutex_lock` or any blocking lock
- No `printf` / `write` / logging
- No `dlsym` / `dlopen`
- No reading `/proc` filesystem
- No CPython API calls
- No string reads or symbol resolution

**Why no libc at all?** Libc functions — even "simple" ones like `memcpy` or `errno` access — can be interposed by sanitizers (ASan, TSan), malloc replacements (jemalloc, tcmalloc), or LD_PRELOAD wrappers. Any interposed function might allocate, take a lock, or otherwise break async-signal-safety. By depending on zero libc functions in the handler path, we are immune to all such interposition.

### 6.4 Shard Design

We use **16 shards** (proven by async-profiler at scale):

```
SignalHandlerState:
  shards: [spin::Mutex<Shard>; 16]
  eventfd: i32                       // file descriptor for reader notification
  samples_since_notify: AtomicU32    // global counter for batch notification

Shard:
  frame_buffer: [RawFrame; 128]   // scratch buffer for unwinding
  ring_buffer: RingBuffer          // per-shard SPSC ring buffer
```

- Shard index: `gettid() % 16`
- 3 attempts on contention: `shard`, `(shard+1) % 16`, `(shard+2) % 16`
- With 16 shards and try-lock-only in the handler, the probability of dropping a sample is negligible even with many threads
- Each shard's `frame_buffer` is used as scratch space during unwinding (not on the signal handler's stack — the shard is pre-allocated via mmap)
- Each shard has its own `RingBuffer` instance — true SPSC since the shard lock ensures single-producer
- The `spin` crate's `Mutex` provides both `try_lock()` (used in handler — never blocks, async-signal-safe) and `lock()` (used by reader thread during drain — spins until acquired, ensures exclusive access to shard data)

**Why `spin::Mutex` and not a custom lock**: The `spin` crate is well-tested, `#![no_std]`, and provides exactly the API we need — `try_lock()` for the signal handler and `lock()` for the reader thread. No need to reimplement a spinlock.

---

## 7. Python Stack Unwinding

### 7.1 Overview

**Crate:** `kit/python_unwind`

The unwinder walks the Python interpreter frame chain to collect raw `(code_object_ptr, instruction_offset)` tuples. It does NOT read function names, filenames, or line numbers — those are resolved later by the reader thread.

### 7.2 Frame Chain Structure (CPython 3.14)

```
PyThreadState
  └── current_frame → _PyInterpreterFrame
                        ├── executable → PyCodeObject*
                        ├── instr_ptr → _Py_CODEUNIT* (current instruction)
                        ├── owner → u8 (frame ownership enum)
                        └── previous → _PyInterpreterFrame (caller)
                                        └── ... → NULL at bottom
```

### 7.3 Raw Frame Representation

```
#[repr(C)]
#[derive(Copy, Clone)]
struct RawFrame {
    code_object: u64,    // PyCodeObject* address
    instr_offset: u64,   // byte offset of instr_ptr from co_code_adaptive start
}
```

We store the instruction offset for future line number resolution (not implemented in v1, but the data is captured).

### 7.4 Unwind Algorithm

```
fn unwind(
    tstate: u64,
    offsets: &PythonOffsets,
    buf: &mut [RawFrame; 128],
) -> usize:

  frame_ptr = kindasafe::u64(tstate + offsets.tstate_current_frame)?
  if frame_ptr == 0: return 0

  depth = 0
  prev_frame = 0  // cycle detection

  while frame_ptr != 0 && frame_ptr != prev_frame && depth < 128:
    // Read frame owner to classify the frame
    owner = kindasafe::u64(frame_ptr + offsets.frame_owner)? & 0xFF

    if owner == FRAME_OWNED_BY_CSTACK:  // value 3
      // C→Python entry shim — skip this frame, continue to previous
      prev_frame = frame_ptr
      frame_ptr = kindasafe::u64(frame_ptr + offsets.frame_previous)?
      continue

    // Read code object pointer
    code_obj = kindasafe::u64(frame_ptr + offsets.frame_executable)?
    if code_obj == 0:
      break  // invalid frame — stop

    // Read instruction pointer for future line number resolution
    instr_ptr = kindasafe::u64(frame_ptr + offsets.frame_instr_ptr)?

    buf[depth] = RawFrame {
      code_object: code_obj,
      instr_offset: instr_ptr,  // store raw pointer; symbolizer computes offset
    }
    depth += 1

    // Move to previous frame
    prev_frame = frame_ptr
    frame_ptr = kindasafe::u64(frame_ptr + offsets.frame_previous)?

  return depth
```

### 7.5 Frame Owner Values (CPython 3.14)

```
FRAME_OWNED_BY_THREAD       = 0   // Normal Python frame — INCLUDE
FRAME_OWNED_BY_GENERATOR    = 1   // Generator/coroutine — INCLUDE
FRAME_OWNED_BY_FRAME_OBJECT = 2   // Has visible PyFrameObject — INCLUDE
FRAME_OWNED_BY_CSTACK       = 3   // C→Python entry shim — SKIP
```

**Critical**: We skip ONLY `FRAME_OWNED_BY_CSTACK`. Generator frames (1) and frame-object-backed frames (2) represent valid Python function calls and must be included. Proposals 3 and 4 got this wrong by stopping at any non-zero owner value.

### 7.6 Safety Considerations

- Every memory read uses `kindasafe::u64()`, recovering gracefully from invalid pointers.
- If any read fails, the unwind stops and returns whatever frames were collected so far.
- Max depth of 128 frames prevents infinite loops from corrupted pointers.
- Cycle detection: if `frame_ptr == prev_frame`, break (circular list).
- The frame buffer is pre-allocated (in the shard), not on the signal handler's stack.

### 7.7 Instruction Offset Note

In CPython 3.14, `instr_ptr` is a `_Py_CODEUNIT*`, not a byte offset. The actual byte offset from `co_code_adaptive` is:
```
byte_offset = instr_ptr - co_code_adaptive_addr
```

We store the raw `instr_ptr` in the handler and compute the actual offset during symbolication (which can read `co_code_adaptive` at that time). This avoids an extra `kindasafe::u64()` read per frame in the handler.

---

## 8. Async-Signal-Safe Primitives

**Crate:** `kit/notlibc`

### 8.1 Shard Locking via `spin::Mutex`

We use the `spin` crate's `Mutex<T>` for shard locking. It provides two lock methods:

- **`try_lock() -> Option<MutexGuard>`** — single CAS, returns immediately. Used by the signal handler. Async-signal-safe (no syscalls, no blocking).
- **`lock() -> MutexGuard`** — spins until acquired. Used by the reader thread during dump to guarantee exclusive access to shard data (ensures no handler is mid-unwind into the frame buffer).

```
// Signal handler: never block
if let Some(guard) = shards[shard_idx].try_lock() {
    // unwind + record
} else {
    // contention — try next shard
}

// Reader thread during dump: must wait for any in-flight handler to finish
let guard = shards[shard_idx].lock();
// ensure no handler is mid-unwind, then snapshot counts
```

Properties:
- `#![no_std]` — no OS dependencies.
- `try_lock()` is async-signal-safe — single CAS.
- `lock()` spins with `core::hint::spin_loop()` — used only outside signal context.
- Guard-based RAII unlock — cannot forget to unlock.
- Well-tested, widely used crate — no need to reimplement.

### 8.2 Eventfd Wrapper

```
fn eventfd_create() -> Result<i32, i32>:
  // Inline assembly: syscall(SYS_eventfd2, 0, EFD_NONBLOCK | EFD_SEMAPHORE)
  // Returns file descriptor

fn eventfd_notify(fd: i32):
  // Inline assembly: syscall(SYS_write, fd, &1u64, 8)
  // Writes 1 to the eventfd counter — wakes the reader
  // Non-blocking, async-signal-safe (raw write syscall to a kernel fd)
```

The `eventfd` is created at init time (not in the handler). The handler only calls `eventfd_notify()` which is a single raw `write` syscall — async-signal-safe on Linux.

### 8.3 Raw Mmap Wrappers

```
fn safe_mmap(size: usize) -> Result<*mut u8, i32>:
  // Inline assembly: syscall instruction with SYS_mmap number
  // No libc dependency — direct kernel call
  // MAP_PRIVATE | MAP_ANONYMOUS, PROT_READ | PROT_WRITE

fn safe_munmap(addr: *mut u8, size: usize) -> Result<(), i32>:
  // Inline assembly: syscall instruction with SYS_munmap number
```

### 8.4 Raw Syscall Helpers

```
fn raw_gettid() -> u32:
  // Inline assembly: syscall instruction with SYS_gettid number
  // Returns Linux thread ID without calling libc

```

All syscall helpers use `core::arch::asm!` — no libc dependency.

---

## 9. Collection: Lock-Free Ring Buffer (`kit/sig_ring`)

A per-shard SPSC (Single Producer Single Consumer) lock-free ring buffer with variable-length records. The signal handler appends raw stack traces; the reader thread drains them in batches.

### 9.1 Structure

```
RingBuffer:
  data: *mut u8             // mmap'd buffer
  capacity: u32             // byte capacity (power of 2)
  mask: u32                 // capacity - 1

  write_pos: AtomicU32      // writer position (monotonically increasing)
  read_pos: AtomicU32       // reader position (monotonically increasing)
  overflow_count: AtomicU64 // samples dropped due to full buffer
```

Both `write_pos` and `read_pos` are on separate cache lines (64-byte aligned) to avoid false sharing.

### 9.2 Record Format

Each sample is a variable-length record:

```
┌──────────┬───────────┬─────────┬──────────────────────────────────┐
│ total_len│ thread_id │ depth   │ frames[0..depth]                 │
│ (u32)    │ (u32)     │ (u32)   │ (code_object:u64, instr_ptr:u64) │
└──────────┴───────────┴─────────┴──────────────────────────────────┘

total_len = 12 + depth * 16   (in bytes, padded to 8-byte alignment)
```

Variable-length records are space-efficient: a typical 20-frame stack uses `12 + 20*16 = 332 bytes` (padded to 336), not the 4KB fixed entries of Proposal 3.

### 9.3 Write Path (Signal Handler)

```
fn write(&self, tid: u32, frames: &[RawFrame], depth: u32) -> bool:
  record_len = (12 + depth * 16 + 7) & !7  // 8-byte aligned

  w = self.write_pos.load(Relaxed)
  r = self.read_pos.load(Acquire)
  available = self.capacity - (w.wrapping_sub(r))

  if record_len > available:
    self.overflow_count.fetch_add(1, Relaxed)
    return false

  // Write record at w & mask
  // Handle wraparound: if record spans buffer boundary, write in two parts
  write_record_at(self.data, w & self.mask, self.capacity, tid, depth, frames)

  // Publish: make data visible before advancing write_pos
  self.write_pos.store(w.wrapping_add(record_len), Release)
  return true
```

With the shard lock, only one writer exists per ring buffer (true SPSC), so no CAS loop is needed — a simple load + store suffices.

### 9.4 Read Path (Reader Thread)

```
fn drain(&self, out: &mut Vec<RawSample>) -> usize:
  r = self.read_pos.load(Relaxed)    // only reader writes read_pos
  w = self.write_pos.load(Acquire)
  count = 0

  while r != w:
    (record_len, tid, depth, frames) = read_record_at(self.data, r & self.mask, self.capacity)
    out.push(RawSample { tid, frames, depth })
    r = r.wrapping_add(record_len)
    count += 1

  self.read_pos.store(r, Release)
  return count
```

### 9.5 Notification Mechanism

The signal handler notifies the reader thread via an `eventfd` every **N samples** (e.g., N = 32). This allows the reader to process samples in batches rather than sleeping for the full 15s interval:

```
// In signal handler, after successful ring buffer write:
total = global_sample_counter.fetch_add(1, Relaxed)
if total % NOTIFY_INTERVAL == 0:
  eventfd_notify(state.eventfd)  // raw write(fd, &1u64, 8) — async-signal-safe
```

The notification is best-effort: if the write fails (fd full, etc.), it's harmless — the reader will still wake on the 15s timer. The `eventfd` is created with `EFD_NONBLOCK` so the handler never blocks.

### 9.6 Sizing

- 16 shards × 1 ring buffer per shard.
- Each ring buffer: **256 KiB** = 262144 bytes.
- At 100 Hz with 20-frame stacks: each record ≈ 336 bytes.
- Per shard: `256 KiB / 336 ≈ 780` samples ≈ 7.8 seconds of data.
- Total across 16 shards: **4 MiB**, ~12480 samples.
- With reader draining on notification (every ~320ms at 100Hz/32), the buffers rarely fill up.
- Overflow is tracked by per-shard `overflow_count`.

### 9.7 Properties

- **True SPSC per shard**: one writer (signal handler holding shard lock), one reader (reader thread).
- **Lock-free write path**: simple atomic store of `write_pos` (no CAS needed with SPSC).
- **Variable-length records**: space-efficient for varying stack depths.
- **No allocation in handler**: buffer is pre-allocated via mmap.
- **Simple**: no hashing, no probing, no CAS races — just append and advance a position.

---

## 10. Reader Thread and Periodic Flush

### 10.1 Overview

**Crate:** `kit/profiler_core`

A background thread that:
- Wakes on **eventfd notification** (every N samples) to drain ring buffers and aggregate stacks.
- Wakes on **15-second timeout** to flush: symbolize aggregated stacks, build pprof, send to Pyroscope.

This two-trigger design ensures low-latency batch processing (ring buffers don't fill up) while still doing expensive symbolization/HTTP only every 15 seconds.

### 10.2 Thread Loop

```
fn reader_thread(state: Arc<ProfilerState>):
  let mut aggregated: HashMap<Vec<RawFrame>, u64> = HashMap::new()
  let mut last_flush = Instant::now()

  loop:
    // Wait for either: eventfd notification OR 15s timeout
    timeout = Duration::from_secs(15) - last_flush.elapsed()
    poll_result = poll([eventfd], timeout)

    // Drain all ring buffers regardless of wake reason
    drain_all_shards(&state, &mut aggregated)

    if poll_result == TIMEOUT || last_flush.elapsed() >= 15s:
      // 15s elapsed — flush to Pyroscope
      flush_and_send(&state, &mut aggregated)
      last_flush = Instant::now()

fn drain_all_shards(state: &ProfilerState, aggregated: &mut HashMap<Vec<RawFrame>, u64>):
  for shard_idx in 0..16:
    let guard = shards[shard_idx].lock()  // wait for in-flight handler
    let mut raw_samples = Vec::new()
    guard.ring_buffer.drain(&mut raw_samples)
    // guard drops → shard unlocked immediately
    for sample in raw_samples:
      *aggregated.entry(sample.frames).or_insert(0) += 1

fn flush_and_send(state: &ProfilerState, aggregated: &mut HashMap<Vec<RawFrame>, u64>):
  if aggregated.is_empty():
    return

  // 1. Symbolize
  symbolized = symbolize(aggregated, &state.offsets, &mut state.symbol_cache)

  // 2. Build pprof
  pprof_bytes = pprof_enc::build(&symbolized, 100 /* sample_rate_hz */)

  // 3. Compute time range
  now = SystemTime::now()
  from = now - Duration::from_secs(15)

  // 4. Send to Pyroscope
  if let Err(e) = pyroscope_ingest::send(
    &state.config.server_url,
    &state.config.app_name,
    &pprof_bytes,
    from, now,
  ) {
    // Log error, continue — don't crash
  }

  // 5. Clear aggregation for next cycle
  aggregated.clear()
```

### 10.3 Drain Behavior

The reader drains one shard at a time, taking a **full `spin::Mutex::lock()`** on each shard. This ensures no handler is mid-write to that shard's ring buffer. The lock is held only for the duration of the `drain()` call (reading atomic positions and copying data out), then immediately released — the shard is blocked for microseconds at most.

```
for shard_idx in 0..16:
  let guard = shards[shard_idx].lock()  // wait for in-flight handler
  guard.ring_buffer.drain(&mut raw_samples)
  // guard drops → shard unlocked, handler can resume
```

Since only one shard is locked at a time, at most 1/16 of concurrent signals are affected during drain.

### 10.4 Two-Phase Design: Drain vs Flush

| Phase | Trigger | What happens | Cost |
|-------|---------|-------------|------|
| **Drain** | eventfd (every N samples) | Read raw samples from ring buffers, aggregate into `HashMap<stack, count>` | Cheap — no symbolization, no I/O |
| **Flush** | 15s timeout | Symbolize aggregated stacks, build pprof, HTTP POST to Pyroscope | Expensive — string reads, protobuf encoding, network I/O |

This separation means the ring buffers are drained frequently (keeping them from overflowing) while the expensive flush happens only once per 15-second window. The aggregation `HashMap` lives in the reader thread and is standard Rust — no signal-safety concerns.

---

## 11. Symbolication

### 11.1 Overview

Performed by the reader thread in `kit/profiler_core`. Converts raw `(PyCodeObject*, instr_ptr)` tuples into human-readable function names.

### 11.2 Symbol Cache

```
SymbolCache:
  cache: HashMap<u64, SymbolInfo>    // PyCodeObject* → SymbolInfo

SymbolInfo:
  function_name: String,    // from co_qualname or co_name
  filename: String,         // from co_filename (for future use)
  first_line: i32,          // from co_firstlineno
```

### 11.3 Reading Code Object Fields

For each unique `PyCodeObject*` not in cache:

```
// Function name: co_qualname (preferred) or co_name (fallback)
qualname_ptr = kindasafe::u64(code_obj_ptr + offsets.code_qualname)?
function_name = read_python_ascii_string(qualname_ptr, offsets)?

// Filename
filename_ptr = kindasafe::u64(code_obj_ptr + offsets.code_filename)?
filename = read_python_ascii_string(filename_ptr, offsets)?

// First line number
firstlineno = kindasafe::u64(code_obj_ptr + offsets.code_firstlineno)? as i32
```

### 11.4 Reading Python Unicode Strings

Most Python code object names are ASCII:

```
fn read_python_ascii_string(obj_ptr: u64, offsets: &PythonOffsets) -> Option<String>:
  if obj_ptr == 0: return None
  // ASCII data starts immediately after PyASCIIObject header
  data_ptr = obj_ptr + offsets.unicode_asciiobject_size
  let mut buf = [0u8; 256]
  kindasafe::slice(&mut buf, data_ptr)?
  let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
  String::from_utf8_lossy(&buf[..len]).into_owned()
```

This runs in the reader thread (normal context), so `kindasafe` reads are safe. The SIGSEGV recovery handles stale code object pointers.

### 11.5 Cache Lifetime

For this initial implementation, the symbol cache persists for the profiler's lifetime. Code object addresses may be reused by CPython's allocator after a code object is freed, leading to stale cache entries. In practice this is rare for long-lived code (imports, class definitions) and acceptable for an initial implementation. Future improvement: validate cache entries by reading a secondary field.

---

## 12. Pprof Generation

**Crate:** `kit/pprof_enc`

### 12.1 Profile Structure

Following the [pprof format specification](https://github.com/google/pprof/blob/main/proto/profile.proto):

```
Profile:
  string_table: Vec<String>        // index 0 = "" (required by spec)
  sample_type: [ValueType]         // [{type: "cpu", unit: "nanoseconds"}]
  samples: Vec<Sample>
  locations: Vec<Location>
  functions: Vec<Function>

  period: i64                      // 10,000,000 ns (10ms)
  period_type: ValueType           // {type: "cpu", unit: "nanoseconds"}
  duration_nanos: i64              // 15,000,000,000 ns (15s)
  time_nanos: i64                  // profile start time (epoch ns)
```

### 12.2 Building Process

```
For each (symbolized_stack, count):
  sample = Sample {
    location_ids: [],
    values: [count * 10_000_000],   // count × period_ns = CPU nanoseconds
  }

  for frame in symbolized_stack:
    func = intern_function(frame.function_name, frame.filename, frame.first_line)
    loc = intern_location(func.id)
    sample.location_ids.push(loc.id)
```

### 12.3 String Table Deduplication

```
StringTable:
  strings: Vec<String>             // strings[0] = ""
  index: HashMap<String, i64>      // string → index

  fn intern(&mut self, s: &str) -> i64
```

### 12.4 Encoding

Use `prost` derive macros to define the pprof message structs inline (avoids .proto file management). Encode with `prost::Message::encode_to_vec()`, then gzip compress with `flate2`.

---

## 13. Pyroscope Ingestion

**Crate:** `kit/pyroscope_ingest`

### 13.1 API Endpoint

```
POST {base_url}/ingest

Query parameters:
  name       = {app_name}.cpu
  from       = {start_timestamp_unix_seconds}
  until      = {end_timestamp_unix_seconds}
  format     = pprof
  spyName    = pyroscope-cpython-rs
  sampleRate = 100

Headers:
  Content-Type: application/octet-stream

Body: gzipped pprof protobuf bytes
```

We use the `/ingest` endpoint for simplicity (it accepts raw gzipped pprof).

### 13.2 Implementation

Use `ureq` (minimal synchronous HTTP client, no async runtime):

```
fn send(base_url: &str, app_name: &str, pprof_gz: &[u8], from: u64, until: u64) -> Result<()>:
  let url = format!(
    "{}/ingest?name={}.cpu&from={}&until={}&format=pprof&spyName=pyroscope-cpython-rs&sampleRate=100",
    base_url, app_name, from, until
  )
  ureq::post(&url)
    .set("Content-Type", "application/octet-stream")
    .timeout(Duration::from_secs(5))
    .send_bytes(pprof_gz)?
  Ok(())
```

### 13.3 Error Handling

- Ingestion errors are logged but do not stop the profiler.
- If the Pyroscope server is down, samples are lost (not queued).
- Timeout: 5 seconds.
- No retries in this implementation.

---

## 14. Shared Library Interface

### 14.1 Overview

**Crate:** `kit/pyroscope_cpython`

Produces a `.so` (cdylib) loadable via `dlopen`:

```python
import ctypes
lib = ctypes.CDLL("./libpyroscope_cpython.so")
lib.pyroscope_start(b"my-python-app", b"http://localhost:4040")
# ... application runs, profiling continues for process lifetime ...
```

### 14.2 Exported Functions

```
#[no_mangle]
pub unsafe extern "C" fn pyroscope_start(
    app_name: *const c_char,
    server_url: *const c_char,
) -> c_int
// Returns 0 on success, nonzero error code on failure.
// There is no stop function — the profiler runs for the lifetime of the process.
```

### 14.3 Profiler Lifecycle

```
UNINITIALIZED (0) → RUNNING (1)
```

- `pyroscope_start()` transitions 0 → 1. Once started, the profiler runs until process exit.
- Calling `pyroscope_start()` when already running returns error code 9.
- The signal handler checks state == RUNNING (via the global AtomicPtr being non-null) before proceeding.
- No stop/cleanup is needed — the OS reclaims all mmap'd memory and timers on process exit.

---

## 15. Concurrency Model

### 15.1 Participants

| Participant | Runs on | What it accesses |
|-------------|---------|-----------------|
| Signal handler | Interrupted thread (any) | Shard lock, frame buffer, collector (write side), atomic counters |
| Reader thread | Dedicated background thread | Collector (read side), symbol cache (HashMap), pprof builder, HTTP |
| Init | Main thread (Python caller) | Everything — but runs before handler is installed |

### 15.2 Lock Hierarchy

| Level | Mechanism | Context |
|-------|-----------|---------|
| Signal handler | `spin::Mutex::try_lock()` on per-shard state | Async-signal-safe |
| Reader thread (dump) | `spin::Mutex::lock()` on per-shard state | Spins until acquired |
| Signal handler | Atomic store on ring buffer write_pos | Async-signal-safe |
| Reader thread (dump) | `spin::Mutex::lock()` on each shard sequentially | Normal (spins briefly) |
| Reader thread | `HashMap` for symbol cache (local to reader) | Normal |
| Init | Sequential — runs before handler is installed | Sequential |

### 15.3 No Deadlocks: Proof by Construction

1. **Signal handler** only uses `try_lock()` (never blocks) → cannot deadlock.
2. **Reader thread** uses `lock()` on shard mutexes, but this is the only lock it acquires. The signal handler on the same thread cannot interrupt and deadlock because `sa_mask = {SIGPROF, SIGSEGV, SIGBUS}` blocks SIGPROF during handler execution, and the reader thread is not a signal handler.
3. **Reader thread spin duration is bounded**: the signal handler holds the shard lock for ~1-10µs (one unwind + one collection write). The reader thread's `lock()` spin completes quickly.
4. **Init** runs before the handler is installed → no concurrent signal handler access during initialization.
5. `sa_mask = {SIGPROF, SIGSEGV, SIGBUS}` prevents re-entrant handler execution and prevents delivery during fault recovery → the shard lock is never held by the same thread trying to re-acquire it via signal.

### 15.4 Global State Management

```
static PROFILER_STATE: AtomicPtr<ProfilerState> = AtomicPtr::new(null_mut())

// Init: publish state (after all allocation is complete)
let state = Box::into_raw(Box::new(state))
PROFILER_STATE.store(state, Release)

// Signal handler: read state
let state = PROFILER_STATE.load(Acquire)
if state.is_null() { return }

// No shutdown — state and memory persist for process lifetime.
// The OS reclaims everything on process exit.
```

---

## 16. Memory Management

### 16.1 Allocation Overview

| Data Structure | Size | Allocated By | When Freed |
|---------------|------|-------------|-----------|
| Shard state (16 shards × lock + frame buf) | 16 × (4 + 128×16) ≈ 33 KiB | `safe_mmap` at init | Process exit |
| Ring buffers (16 shards × 256 KiB) | 4 MiB | `safe_mmap` at init | Process exit |
| eventfd | 1 fd | `eventfd2` syscall at init | Process exit |
| PythonOffsets struct | ~200 B | `Box::new` at init | Process exit (leaked) |
| Symbol cache | Dynamic | Reader thread heap | Process exit |
| Pprof buffer | Dynamic | Reader thread heap | Dropped after each send |

### 16.2 Signal Handler: Zero Heap Allocation

The signal handler path allocates **zero bytes from the heap**:
- Frame buffer: pre-allocated per-shard (via mmap).
- Ring buffers: pre-allocated at init (via mmap).
- Hash computation: pure computation, stack-local variables.
- All atomic operations: on pre-allocated memory.

### 16.3 mmap vs malloc Boundary

- **`safe_mmap`** (raw `syscall` instruction): all memory touched by the signal handler.
- **`malloc`** (Rust's standard allocator): everything in the reader thread and initialization.

This strict separation ensures that even if the signal interrupts the reader thread inside `malloc`, the signal handler never calls into the allocator — no deadlock possible.

### 16.4 No-libc Guarantee in Handler Path

The signal handler path uses **zero libc functions**. This is enforced structurally:
- All handler-path crates (`kindasafe`, `notlibc`, `sig_ring`, `python_unwind`) are `#![no_std]` and do not have `libc` in their dependency tree.
- `kindasafe` (the read crate) uses naked assembly — no libc. The init-time code (`kindasafe_init`) depends on libc but is never in the handler path.
- `spin` is `#![no_std]` — no libc.
- OS interactions (`gettid`, `mmap`) use raw `syscall` instructions via `core::arch::asm!`.

---

## 17. Error Handling

### 17.1 Signal Handler Error Strategy

The signal handler must never panic, abort, or call `unwrap()`. All errors result in early return with a counter increment:

| Situation | Counter Incremented |
|-----------|-------------------|
| Global state is NULL | (none — just return) |
| All 3 shard locks contended | `samples_lock_fail` |
| TLS read failed | `samples_tstate_fail` |
| PyThreadState is NULL | `samples_tstate_fail` |
| Unwind produced 0 frames | `samples_empty` |
| Ring buffer full | `samples_overflow` |
| Successful collection | `samples_collected` |

All counters are `AtomicU64` with `Relaxed` ordering — no synchronization needed for metrics.

### 17.2 Reader Thread Error Strategy

The reader thread uses standard Rust error handling:
- **Symbolication failure**: use placeholder `"<unknown>"` for the function name.
- **Pprof encoding failure**: log error, skip this flush cycle.
- **HTTP ingestion failure**: log error, drop the data, continue to next cycle.

The reader thread never crashes the host process.

### 17.3 Init Error Codes

| Return Code | Meaning |
|------------|---------|
| 0 | Success |
| 1 | kindasafe_init init failed |
| 2 | Python binary not found in /proc/self/maps |
| 3 | _PyRuntime or Py_Version symbol not found |
| 4 | _Py_DebugOffsets cookie/version validation failed |
| 5 | Unsupported Python version |
| 6 | TLS offset discovery failed |
| 7 | Memory allocation (mmap) failed |
| 8 | Signal handler installation failed |
| 9 | Profiler already running |

---

## 18. Future Work

| Feature | Where it Fits |
|---------|---------------|
| Native (C/C++) frame unwinding | Add `kit/native_unwind` crate, interleave native and Python frames |
| Line number resolution | Decode `co_linetable` in reader thread using `instr_offset` from `RawFrame` |
| Per-thread `timer_create` | Add per-thread timer in `sighandler`, thread discovery via `/proc/self/task/` |
| Multiple Python versions | Add offset tables for 3.12, 3.13 in `python_offsets`; version dispatch at init |
| Free-threaded Python (3.13t+) | Handle `free_threaded == 1` in debug offsets, adjust TLS access |
| ARM64 support | Add arch modules in `kindasafe` and `python_offsets` (FS register differs) |
| Ruby profiler | Reuse `notlibc`, `sig_ring`, `sighandler`, `pprof_enc`, `pyroscope_ingest`; create `ruby_offsets` + `ruby_unwind` |
| .NET profiler | Same reuse pattern |
| Wall-clock profiling | Use `ITIMER_REAL` + `SIGALRM` or `timer_create(CLOCK_MONOTONIC)` |
| Labels/tags | Add label fields to sample records, propagate to pprof |
| Configuration API | More `pyroscope_start` parameters or environment variable configuration |
| Auto-start via LD_PRELOAD | Constructor function (`__attribute__((constructor))`) with env var config |

---

## Appendix: CPython 3.14 Structures

### A.1 `_Py_DebugOffsets` Layout

Located at `_PyRuntime + 0`. Begins with:

```
Offset 0:   cookie[8]          "xdebugpy" (0x7970677562656478 LE)
Offset 8:   version (u64)      PY_VERSION_HEX, e.g. 0x030E00F0 for 3.14.0 final
Offset 16:  free_threaded(u64) 0 for standard build, 1 for free-threaded
```

Followed by sub-structures, each containing a `size` field and then offset fields (all u64):

- `runtime_state { size, finalizing, interpreters_head }`
- `interpreter_state { size, id, next, threads_head, threads_main, gc, ... }`
- `thread_state { size, prev, next, interp, current_frame, native_thread_id, thread_id, ... }`
- `interpreter_frame { size, previous, executable, instr_ptr, localsplus, owner, ... }`
- `code_object { size, filename, name, qualname, linetable, firstlineno, argcount, ..., co_code_adaptive, ... }`
- `pyobject { size, ob_type }`
- `type_object { size, tp_name, tp_repr, tp_flags }`
- `tuple_object { size, ob_item, ob_size }`
- `unicode_object { size, state, length, asciiobject_size }`
- `gen_object { size, gi_name, gi_iframe, gi_frame_state }`

The recommended approach to read:
1. Validate cookie (first 8 bytes == "xdebugpy") and version.
2. Read the entire `_Py_DebugOffsets` as a sequence of u64 values (it's at a known address).
3. Parse according to the known 3.14 field order (field count per sub-struct is fixed per minor version).
4. Store parsed values in `PythonOffsets`.

### A.2 Frame Owner Constants

```
FRAME_OWNED_BY_THREAD       = 0   // Normal Python frame
FRAME_OWNED_BY_GENERATOR    = 1   // Generator/coroutine frame
FRAME_OWNED_BY_FRAME_OBJECT = 2   // Has a visible PyFrameObject
FRAME_OWNED_BY_CSTACK       = 3   // C→Python entry shim — SKIP THIS
```

### A.3 Thread State Navigation

```
_PyRuntime (global, found via ELF symbol)
  + runtime_state.interpreters_head → PyInterpreterState*
    + interpreter_state.threads_head → PyThreadState* (head)
      + native_thread_id → OS TID
      + current_frame → _PyInterpreterFrame*
        + executable → PyCodeObject*
        + instr_ptr → _Py_CODEUNIT*
        + owner → u8
        + previous → next frame (or NULL)
      + next → next PyThreadState* (or NULL)
```

### A.4 TLS Access on x86_64

For `_PyThreadState_GetCurrent` using a static `__thread` variable:
```
// The compiler generates something like:
//   mov rax, fs:[known_negative_offset]
//   ret
//
// We extract known_negative_offset by disassembling the first bytes.
// At runtime in the handler:
//   tstate = *(FS_BASE + known_negative_offset)
// where FS_BASE can be read via arch_prctl or from fs:0x0.
```

For the legacy `autoTSSkey` approach (3.13 and earlier):
```
// fs_base = kindasafe::fs_0x10()  // TCB pointer
// tls_slot = fs_base + 8 + ((tss_key + 0x31) << 4)
// tstate = kindasafe::u64(tls_slot)
```

### A.5 Memory Read Patterns in Signal Handler

All memory reads in the signal handler follow this pattern:
```
let value = match kindasafe::u64(addr) {
    Ok(v) => v,
    Err(_) => return,  // read failed — abort this sample gracefully
};
```

No read failure crashes the handler. Every pointer dereference is wrapped in `kindasafe`, which uses SIGSEGV recovery to handle invalid addresses.
