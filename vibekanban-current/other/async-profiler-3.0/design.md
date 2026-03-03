# async-profiler v3.0 — Design Deep Dive (Linux CPU Profiling)

Source: https://github.com/async-profiler/async-profiler tag v3.0
~18,000 lines of C++ in `src/`. This analysis covers only the Linux CPU profiling path.

---

## 1. Architecture Overview

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Profiler (singleton)                        │
│                                                                     │
│  State Machine: NEW → IDLE → RUNNING → IDLE → TERMINATED           │
│                                                                     │
│  ┌─────────────┐  ┌──────────────────┐  ┌────────────────────────┐ │
│  │  Engine*     │  │ CallTraceStorage │  │ FlightRecorder (JFR)   │ │
│  │ (selected)   │  │  (lock-free HT)  │  │ (output recording)    │ │
│  └──────┬───────┘  └──────────────────┘  └────────────────────────┘ │
│         │          ┌──────────────────┐  ┌────────────────────────┐ │
│         │          │ Dictionary ×2    │  │ CodeCacheArray         │ │
│         │          │ (_class_map,     │  │ (_native_libs)         │ │
│         │          │  _symbol_map)    │  │                        │ │
│         │          └──────────────────┘  └────────────────────────┘ │
│         │          ┌──────────────────┐                             │
│         │          │ SpinLock[16]     │  ← signal handlers acquire  │
│         │          │ CallTraceBuffer  │    these non-blockingly      │
│         │          │            [16]  │                             │
│         │          └──────────────────┘                             │
│         ▼                                                           │
│  ┌──────────────────────────────────────────────┐                   │
│  │              CPU Engines (one active)         │                   │
│  │                                               │                   │
│  │  PerfEvents        CTimer          ITimer     │                   │
│  │  ─────────         ──────          ──────     │                   │
│  │  perf_event_open   timer_create    setitimer  │                   │
│  │  per-thread FD     per-thread      process    │                   │
│  │  mmap ring buf     CLOCK_THREAD    ITIMER_    │                   │
│  │  SIGPROF           _CPUTIME_ID     PROF       │                   │
│  │                    SIGPROF         SIGPROF    │                   │
│  └──────────────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────────┘
```

### Engine Hierarchy

```
Engine                     (engine.h — base class, virtual start/stop)
  └─ CpuEngine             (cpuEngine.h — adds thread hook, signal handlers)
       ├─ PerfEvents        (perfEvents_linux.cpp — perf_event_open)
       ├─ CTimer            (ctimer_linux.cpp — timer_create per-thread)
       └─ ITimer            (itimer.cpp — setitimer process-wide)
```

`Engine::_enabled` is a `static volatile bool` checked at the top of every signal handler to fast-exit when profiling is disabled.

### Key Files

| File | Lines | Purpose |
|------|-------|---------|
| `profiler.cpp` | 1804 | Singleton orchestrator: start/stop lifecycle, `recordSample()` |
| `profiler.h` | 253 | State, locks[16], buffers[16], storage, dictionaries |
| `perfEvents_linux.cpp` | 953 | perf_event_open, ring buffer, signal handler |
| `cpuEngine.cpp` | 129 | Thread hook (GOT patching), base signal handler |
| `ctimer_linux.cpp` | 142 | Per-thread POSIX CPU timer |
| `itimer.cpp` | 63 | Process-wide setitimer fallback |
| `callTraceStorage.cpp` | 292 | Lock-free hash table + linear allocator |
| `linearAllocator.cpp` | 104 | CAS-based bump allocator |
| `stackWalker.cpp` | 411 | FP walking, DWARF walking |
| `os_linux.cpp` | 342 | Thread enumeration, signal install, safeAlloc |
| `dictionary.cpp` | 131 | Lock-free string interning |
| `spinLock.h` | 66 | CAS-based spin lock |

---

## 2. Profiling Lifecycle

### start() Flow — `profiler.cpp:1031-1208`

```
Profiler::start(args)
  │
  ├─ 1. checkJvmCapabilities()
  │
  ├─ 2. Reset counters, dictionaries, storage (under lockAll())
  │
  ├─ 3. Allocate CallTraceBuffer[16]
  │     size = (max_stack_depth + 128 + 4) * sizeof(CallTraceBuffer)
  │     Each buffer stores one in-progress stack trace
  │
  ├─ 4. selectEngine(event_name)
  │     "cpu"    → PerfEvents (if supported)
  │     "ctimer" → CTimer
  │     "itimer" → ITimer
  │
  ├─ 5. updateSymbols(kernel_symbols_needed)
  │     Parse /proc/self/maps, load ELF .symtab/.dynsym, DWARF .eh_frame
  │
  ├─ 6. installTraps(begin, end) — SIGTRAP breakpoints
  │
  ├─ 7. Engine::start(args) → e.g. PerfEvents::start():
  │     ├─ setupThreadHook()         — find GOT entry for pthread_setspecific
  │     ├─ installSignalHandler()    — sigaction(SIGPROF, signalHandler)
  │     ├─ enableThreadHook()        — patch GOT to intercept new threads
  │     └─ createForAllThreads()     — iterate /proc/self/task, create per-thread events
  │
  ├─ 8. switchThreadEvents(JVMTI_ENABLE) — JVMTI thread start/end callbacks
  │
  └─ 9. _state = RUNNING
```

### Signal Handler Flow (PerfEvents)

```
Kernel delivers SIGPROF to thread
  │
  ▼
PerfEvents::signalHandler(signo, siginfo, ucontext)    ← perfEvents_linux.cpp:655
  │
  ├─ if (siginfo->si_code <= 0) return   // filter external signals
  │
  ├─ if (!_enabled) { resetBuffer(tid); return; }
  │
  ├─ counter = readCounter(siginfo, ucontext)
  │
  └─ Profiler::recordSample(ucontext, counter, PERF_SAMPLE, &event)
       │
       ├─ atomicInc(_total_samples)
       │
       ├─ tid = fastThreadId()
       ├─ lock_index = tid % 16
       ├─ Try _locks[lock_index].tryLock()       // non-blocking CAS
       │   Fail → try (lock_index+1) % 16
       │   Fail → try (lock_index+2) % 16
       │   All fail → drop sample, atomicInc(failures), return 0
       │
       ├─ frames = _calltrace_buffer[lock_index]
       │
       ├─ getNativeTrace()     — walk perf ring buffer or FP/DWARF
       ├─ getJavaTraceAsync()  — call AsyncGetCallTrace (ASGCT)
       │
       ├─ call_trace_id = _call_trace_storage.put(num_frames, frames, counter)
       ├─ _jfr.recordEvent(lock_index, tid, call_trace_id, ...)
       │
       └─ _locks[lock_index].unlock()

  // Back in signal handler:
  ioctl(siginfo->si_fd, PERF_EVENT_IOC_RESET, 0)     // reset counter
  ioctl(siginfo->si_fd, PERF_EVENT_IOC_REFRESH, 1)    // re-arm for next sample
```

### stop() Flow

```
Profiler::stop()
  ├─ disableThreadHook()     — restore original pthread_setspecific in GOT
  ├─ destroyForThread(i)     — for each tid: ioctl(DISABLE), close(fd), munmap(page)
  └─ _state = IDLE
```

---

## 3. CPU Sampling Engines

### 3.1 PerfEvents (Primary) — `perfEvents_linux.cpp`

Uses Linux `perf_event_open(2)` syscall. One file descriptor per thread.

**Event types supported:**

| Name | Type | Config | Default Interval |
|------|------|--------|-----------------|
| `cpu` | SOFTWARE | CPU_CLOCK | 10,000,000 ns |
| `page-faults` | SOFTWARE | PAGE_FAULTS | 1 |
| `context-switches` | SOFTWARE | CONTEXT_SWITCHES | 2 |
| `cycles` | HARDWARE | CPU_CYCLES | 1,000,000 |
| `instructions` | HARDWARE | INSTRUCTIONS | 1,000,000 |
| `cache-misses` | HARDWARE | CACHE_MISSES | 1,000 |
| `L1-dcache-load-misses` | HW_CACHE | L1D load miss | 1,000,000 |
| `LLC-load-misses` | HW_CACHE | LL load miss | 1,000 |
| `dTLB-load-misses` | HW_CACHE | DTLB load miss | 1,000 |

Also supports raw PMU events (`rNNN`, `pmu/descriptor/`), hardware breakpoints (`mem:addr`), tracepoints (`trace:ID`, `subsys:event`), and kprobe/uprobe.

**Per-thread setup** (`createForThread`, line 514):

```c
// 1. perf_event_open syscall
struct perf_event_attr attr = {0};
attr.type = event_type->type;            // e.g. PERF_TYPE_SOFTWARE
attr.config = event_type->config;         // e.g. PERF_COUNT_SW_CPU_CLOCK
attr.sample_period = _interval;           // e.g. 10000000 (10ms)
attr.sample_type = PERF_SAMPLE_CALLCHAIN; // kernel collects call chain
attr.disabled = 1;
attr.wakeup_events = 1;
attr.precise_ip = 2;                     // for software events

fd = syscall(__NR_perf_event_open, &attr, tid, -1, -1, 0);

// 2. mmap ring buffer (1 metadata page + 1 data page)
page = mmap(NULL, 2 * page_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);

// 3. Configure signal delivery to specific thread
fcntl(fd, F_SETFL, O_ASYNC);             // async notification
fcntl(fd, F_SETSIG, SIGPROF);            // which signal
fcntl(fd, F_SETOWN_EX, {F_OWNER_TID, tid}); // deliver to this TID

// 4. Arm the event
ioctl(fd, PERF_EVENT_IOC_RESET, 0);
ioctl(fd, PERF_EVENT_IOC_REFRESH, 1);    // fire after 1 sample_period
```

**Ring buffer reading** (`walk`, line 839):

The kernel writes `PERF_RECORD_SAMPLE` records into the mmap'd ring buffer containing the kernel-collected call chain (IP addresses). The profiler reads this in the signal handler:

```c
struct perf_event_mmap_page* page = event->_page;
u64 tail = page->data_tail;
u64 head = page->data_head;
rmb();                                    // read memory barrier (lfence on x86)

RingBuffer ring(page);
while (tail < head) {
    struct perf_event_header* hdr = ring.seek(tail);
    if (hdr->type == PERF_RECORD_SAMPLE) {
        u64 nr = ring.next();             // number of IPs in chain
        while (nr-- > 0) {
            u64 ip = ring.next();
            if (ip < PERF_CONTEXT_MAX) {
                callchain[depth++] = (const void*)ip;
            }
        }
    }
    tail += hdr->size;
}
page->data_tail = head;                  // mark as consumed
```

The ring buffer access is protected by a per-event SpinLock (`PerfEvent extends SpinLock`) to prevent races with `destroyForThread` which calls `munmap`. The tryLock pattern ensures the signal handler never blocks.

### 3.2 CTimer — `ctimer_linux.cpp`

Uses POSIX per-thread CPU-time timers. One timer per thread.

```c
// Custom clock ID for per-thread CPU time (CPUCLOCK_SCHED | CPUCLOCK_PERTHREAD_MASK)
clockid_t clock = ((~tid) << 3) | 6;

// Create timer via raw syscall (libc wrapper only allows predefined clocks)
struct sigevent sev;
sev.sigev_signo = SIGPROF;
sev.sigev_notify = SIGEV_THREAD_ID;
((int*)&sev.sigev_notify)[1] = tid;      // embed TID in sigevent struct
syscall(__NR_timer_create, clock, &sev, &timer);

// Arm timer
struct itimerspec ts;
ts.it_interval = {sec, nsec};
ts.it_value = ts.it_interval;
syscall(__NR_timer_settime, timer, 0, &ts, NULL);
```

The timer fires SIGPROF when the thread has consumed `interval` nanoseconds of CPU time. Key advantage over ITimer: per-thread granularity. Key advantage over PerfEvents: works without `perf_event_paranoid` permissions.

Timer IDs stored as `_timers[tid]` (value is `timer + 1` since 0 means empty slot, and CAS used for race-free creation/deletion).

### 3.3 ITimer — `itimer.cpp`

Simplest fallback. Process-wide, not per-thread.

```c
struct itimerval tv = {{sec, usec}, {sec, usec}};
setitimer(ITIMER_PROF, &tv, NULL);        // fires SIGPROF
```

`ITIMER_PROF` counts process CPU time (user + system). The kernel delivers SIGPROF to an arbitrary running thread. No per-thread control. No setup/teardown per thread. Simplest but least accurate.

### Engine Selection Priority

```c
// profiler.cpp — selectEngine()
if event == "cpu":
    if PerfEvents::supported():    return PerfEvents   // best
    else:                          return WallClock     // fallback
if event == "ctimer":              return CTimer
if event == "itimer":              return ITimer
else:                              return PerfEvents    // raw perf event
```

---

## 4. Signal Handling

### 4.1 Signal Installation — `os_linux.cpp:227`

```c
SigAction OS::installSignalHandler(int signo, SigAction action, SigHandler handler) {
    struct sigaction sa;
    sigemptyset(&sa.sa_mask);

    if (handler != NULL) {
        sa.sa_handler = handler;
        sa.sa_flags = 0;
    } else {
        sa.sa_sigaction = action;            // 3-argument handler
        sa.sa_flags = SA_SIGINFO | SA_RESTART;
    }

    sigaction(signo, &sa, &oldsa);
    return oldsa.sa_sigaction;
}
```

Key flags:
- `SA_SIGINFO` — receive `siginfo_t*` and `ucontext*` (needed for register access)
- `SA_RESTART` — automatically restart interrupted syscalls
- Signal mask is empty — no signals blocked during handler

### 4.2 Signals Used

| Signal | Source | Purpose |
|--------|--------|---------|
| SIGPROF | perf_event / timer | CPU sampling (configurable via `_signal`) |
| SIGTRAP | Trap breakpoints | Begin/end profiling windows |
| SIGSEGV | SafeAccess faults | Crash recovery during stack walking |

### 4.3 Async-Signal-Safety Analysis

**What runs inside the signal handler:**

```
signalHandler()
  ├─ siginfo->si_code check          ✅ safe (struct field access)
  ├─ volatile bool _enabled check    ✅ safe (volatile read)
  ├─ readCounter()                   ✅ safe (register access from ucontext, or read() syscall)
  │
  └─ recordSample()
       ├─ atomicInc()                ✅ safe (__sync_fetch_and_add)
       ├─ fastThreadId()             ✅ safe (cached TLS or gettid syscall)
       ├─ SpinLock::tryLock()        ✅ safe (__sync_bool_compare_and_swap, never blocks)
       ├─ PerfEvents::walk()         ✅ safe (mmap page access + rmb)
       │    └─ SpinLock tryLock      ✅ safe
       ├─ StackWalker::walkFP()      ✅ safe (register access, pointer chasing)
       │    └─ SafeAccess::load()    ✅ safe (may SIGSEGV → handled by segvHandler)
       ├─ StackWalker::walkDwarf()   ✅ safe (same + DWARF table lookup)
       ├─ CallTraceStorage::put()    ✅ safe (CAS-based hash table + LinearAllocator)
       │    ├─ calcHash()            ✅ safe (pure computation)
       │    ├─ CAS on keys[]         ✅ safe (__sync_bool_compare_and_swap)
       │    └─ LinearAllocator::alloc() ✅ safe (CAS bump pointer + OS::safeAlloc)
       │         └─ OS::safeAlloc()  ✅ safe (raw mmap syscall, not libc mmap)
       ├─ FlightRecorder::recordEvent() ✅ safe (writes to pre-allocated buffer)
       └─ SpinLock::unlock()         ✅ safe (__sync_fetch_and_sub)

  ioctl(fd, PERF_EVENT_IOC_RESET)    ✅ safe (syscall)
  ioctl(fd, PERF_EVENT_IOC_REFRESH)  ✅ safe (syscall)
```

**What is explicitly NOT called in signal context:**
- `malloc` / `free` — uses LinearAllocator or OS::safeAlloc instead
- `pthread_mutex_lock` — uses SpinLock::tryLock (non-blocking) instead
- `printf` / `write` to log — deferred
- `dlsym` / `dlopen` — never
- `memcpy` — avoided; manual loop in `storeCallTrace()` (line 201: "Do not use memcpy inside signal handler")

### 4.4 SIGSEGV Recovery for Safe Memory Access

`safeAccess.h` defines `SafeAccess::load()` — a `NOINLINE __attribute__((aligned(16)))` function that simply dereferences a pointer. If the pointer is invalid, SIGSEGV fires. The SIGSEGV handler (`Profiler::segvHandler`) checks if the faulting PC is within the `SafeAccess::load` function (using `skipLoad()` which checks the PC offset from `load`'s address). If so, it skips the faulting instruction and returns 0/NULL, effectively turning an unsafe dereference into a safe one that returns a sentinel.

For `walkVM`, a more robust approach uses `setjmp`/`longjmp` stored in the VMThread's exception field.

---

## 5. perf_event Integration Details

### 5.1 Full Syscall Sequence Per Thread

```
1. syscall(__NR_perf_event_open, &attr, tid, -1, -1, 0)
     → Returns file descriptor
     → attr.sample_period = interval (nanoseconds for cpu clock)
     → attr.sample_type = PERF_SAMPLE_CALLCHAIN
     → attr.disabled = 1, attr.wakeup_events = 1

2. mmap(NULL, 2*page_size, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0)
     → Maps ring buffer: page[0] = perf_event_mmap_page header
                          page[1] = data page for PERF_RECORD_SAMPLE

3. fcntl(fd, F_SETFL, O_ASYNC)
     → Enable async I/O notification

4. fcntl(fd, F_SETSIG, signal_number)
     → Which signal to deliver (SIGPROF by default)

5. fcntl(fd, F_SETOWN_EX, {type=F_OWNER_TID, pid=tid})
     → Deliver signal to this specific thread

6. ioctl(fd, PERF_EVENT_IOC_RESET, 0)
     → Reset event counter to 0

7. ioctl(fd, PERF_EVENT_IOC_REFRESH, 1)
     → Enable event, fire signal after 1 overflow
     → This is a ONE-SHOT arm; must be re-armed in signal handler
```

### 5.2 Ring Buffer Protocol

```
┌──────────────────────────────────────┐
│ perf_event_mmap_page (page 0)        │
│   data_head: written by kernel       │  ← producer pointer
│   data_tail: written by userspace    │  ← consumer pointer
│   ...                                │
├──────────────────────────────────────┤
│ Data page (page 1)                   │
│   ┌─────────────────────────────┐    │
│   │ perf_event_header           │    │
│   │   type: PERF_RECORD_SAMPLE  │    │
│   │   size: ...                 │    │
│   │ nr: number_of_IPs           │    │
│   │ ip[0]: kernel frame         │    │
│   │ ip[1]: kernel frame         │    │
│   │ PERF_CONTEXT_USER           │    │ ← context switch marker
│   │ ip[2]: userspace frame      │    │
│   │ ip[3]: userspace frame      │    │
│   │ ...                         │    │
│   └─────────────────────────────┘    │
└──────────────────────────────────────┘
```

Reading protocol:
1. Read `data_head` from page
2. `rmb()` — load fence to ensure data is visible
3. Walk records from `data_tail` to `data_head`
4. Write `data_tail = data_head` to mark consumed

### 5.3 Event Lifecycle in Signal Handler

```
         perf counter overflows
                │
                ▼
        kernel writes PERF_RECORD_SAMPLE to ring buffer
        kernel sends SIGPROF to owning thread (F_SETOWN_EX)
        kernel DISABLES the event (REFRESH was 1, now 0)
                │
                ▼
        signal handler runs:
          1. Read ring buffer (walk)
          2. Collect stack trace (FP/DWARF + ASGCT)
          3. Store trace (hash table)
          4. ioctl(PERF_EVENT_IOC_RESET, 0)    ← reset counter
          5. ioctl(PERF_EVENT_IOC_REFRESH, 1)  ← re-arm for next sample
```

The one-shot `REFRESH(1)` model ensures that the event is automatically disabled while the signal handler processes it — no risk of recursive signals from the same event.

---

## 6. Stack Walking

### 6.1 Frame Pointer Walking — `stackWalker.cpp:55`

Fastest method. Requires code compiled with `-fno-omit-frame-pointer`.

```
Algorithm:
  1. Extract PC, FP, SP from ucontext_t
  2. Loop:
     a. If PC is in JVM CodeHeap → stop (Java frame boundary)
     b. Record callchain[depth++] = PC
     c. Validate FP: aligned? within stack bounds? reasonable frame size?
     d. PC = SafeAccess::load(*(FP + FRAME_PC_SLOT))
     e. SP = FP + (FRAME_PC_SLOT + 1) * sizeof(void*)
     f. FP = *FP  (previous frame pointer)
```

Validation constants:
- `MAX_WALK_SIZE = 0x100000` (1MB) — maximum stack walk distance
- `MAX_FRAME_SIZE = 0x40000` (256KB) — maximum single frame size
- `FRAME_PC_SLOT = 1` on x86-64 (return address at FP+8)

### 6.2 DWARF CFI Walking — `stackWalker.cpp:105`

Handles code compiled without frame pointers (e.g., glibc, `-O2` binaries).

```
Algorithm:
  1. Extract PC, FP, SP from ucontext_t
  2. Loop:
     a. If PC is in JVM CodeHeap → stop
     b. Record callchain[depth++] = PC
     c. Find CodeCache for PC → findFrameDesc(PC)
        → Binary search in parsed .eh_frame_hdr table
        → Returns FrameDesc {cfa, fp_off, pc_off}
     d. Compute CFA (Canonical Frame Address):
        if cfa_reg == DW_REG_SP: CFA = SP + cfa_off
        if cfa_reg == DW_REG_FP: CFA = FP + cfa_off
        if cfa_reg == DW_REG_PLT: CFA = SP + (alignment-dependent offset)
     e. SP = CFA
     f. FP = *(CFA + fp_off)  if fp_off != DW_SAME_FP
     g. PC = *(CFA + pc_off)  (return address)
```

DWARF parsing (`dwarf.cpp`) processes `.eh_frame_hdr` → binary-searchable FDE index → CFA instructions including:
- `DW_CFA_advance_loc`, `DW_CFA_offset`, `DW_CFA_def_cfa`
- `DW_CFA_remember_state` / `DW_CFA_restore_state` (state stack)
- `DW_CFA_expression` (evaluated via simple stack machine)

### 6.3 Safe Memory Access

`SafeAccess::load(ptr)` is a trivially small function (`NOINLINE`, `aligned(16)`) that simply does `return *ptr`. If the pointer is invalid, SIGSEGV fires. The SIGSEGV handler:

1. Checks if faulting PC is within the 16-byte `SafeAccess::load` function
2. If yes: identifies the MOV instruction, advances PC past it, returns 0
3. If no: this is a real crash, handle normally

This gives the stack walker safe pointer chasing without `setjmp`/`longjmp` overhead on the fast path (no fault = no overhead).

### 6.4 Register Access from Signal Context — `stackFrame_x64.cpp`

On Linux x86-64, registers are in `ucontext_t->uc_mcontext.gregs[]`:

```c
uintptr_t& pc()     → gregs[REG_RIP]
uintptr_t& sp()     → gregs[REG_RSP]
uintptr_t& fp()     → gregs[REG_RBP]
uintptr_t  arg0()   → gregs[REG_RDI]   // 1st syscall/function arg
uintptr_t  arg1()   → gregs[REG_RSI]
uintptr_t  arg2()   → gregs[REG_RDX]
uintptr_t  arg3()   → gregs[REG_RCX]
uintptr_t  method() → gregs[REG_RBX]   // HotSpot method register
```

---

## 7. Data Structures

### 7.1 CallTraceStorage — Lock-Free Hash Table

**Purpose:** Deduplicate and count stack traces. Called from signal handlers.

```
Architecture:

  CallTraceStorage
    ├─ LinearAllocator _allocator     (for CallTrace objects)
    └─ LongHashTable* _current_table  (linked list of tables)

  LongHashTable (page-aligned, allocated via OS::safeAlloc)
    ├─ LongHashTable* _prev           (previous/smaller table)
    ├─ u32 capacity                   (power of 2, starts at 65536)
    ├─ volatile u32 size              (with cache-line padding)
    ├─ u64 keys[capacity]             (MurmurHash64A of frame array)
    └─ CallTraceSample values[capacity]
         ├─ CallTrace* trace          (pointer to allocated trace)
         ├─ u64 samples               (number of times seen)
         └─ u64 counter               (event counter accumulator)
```

**put() algorithm** (line 227):

```
1. hash = MurmurHash64A(frames, num_frames * sizeof(frame))
2. slot = hash & (capacity - 1)
3. Linear probe with quadratic step:
   while keys[slot] != hash:
     if keys[slot] == 0:
       CAS(&keys[slot], 0, hash)       // claim slot
       if table.incSize() >= capacity*3/4:
         Allocate new table (2x), CAS into _current_table
       Migrate trace from prev table if exists, else allocate new
       break
     slot = (slot + (++step)) & (capacity - 1)  // improved linear probing
4. atomicInc(values[slot].samples)
5. atomicInc(values[slot].counter, counter_value)
6. Return stable trace ID: capacity - (INITIAL_CAPACITY - 1) + slot
```

Key properties:
- **Lock-free**: all operations use CAS, no mutexes
- **Append-only**: tables grow but never shrink (linked list of doubling tables)
- **Migration**: when a hash is found in `prev` table, reuses the CallTrace pointer
- **Overflow**: if probe exhausts capacity, returns `OVERFLOW_TRACE_ID` (0x7fffffff)

**Memory ordering:**
- `CallTraceSample::setTrace()` uses `__atomic_store_n(RELEASE)`
- `CallTraceSample::acquireTrace()` uses `__atomic_load_n(ACQUIRE)`
- `incSize()` uses `__sync_add_and_fetch` (full barrier)

### 7.2 LinearAllocator — Signal-Safe Bump Allocator

**Purpose:** Allocate variable-size CallTrace objects without malloc.

```
Chunk (8MB each, allocated via OS::safeAlloc = raw mmap syscall)
  ┌────────────────────────────────────────────────┐
  │ Chunk* prev        │ volatile size_t offs      │
  │ padding[56]        │ (avoid false sharing)      │
  ├────────────────────┴───────────────────────────┤
  │                                                 │
  │  Allocated objects (bump pointer)               │
  │  ┌──────┐ ┌──────────┐ ┌────────┐             │
  │  │Trace1│ │ Trace2   │ │ Trace3 │ ...         │
  │  └──────┘ └──────────┘ └────────┘             │
  │                        ↑ offs                   │
  │                                                 │
  └─────────────────────────────────────────────────┘
```

**alloc() algorithm** (line 41):

```c
void* alloc(size_t size) {
    Chunk* chunk = _tail;
    do {
        for (offs = chunk->offs; offs + size <= _chunk_size; offs = chunk->offs) {
            if (CAS(&chunk->offs, offs, offs + size)) {   // bump pointer atomically
                if (_chunk_size/2 - offs < size)           // past halfway?
                    reserveChunk(chunk);                   // pre-allocate next chunk
                return (char*)chunk + offs;
            }
        }
    } while ((chunk = getNextChunk(chunk)) != NULL);       // get reserved/new chunk
    return NULL;
}
```

Key properties:
- **No mutex**: CAS on offset counter
- **No free**: chunks are freed only on `clear()` (profiling stop)
- **Pre-allocation**: when allocation crosses chunk midpoint, reserves next chunk
- **Reserve chunk**: CAS on `_reserve` pointer, allocated via `OS::safeAlloc()`
- **Signal-safe**: `OS::safeAlloc()` uses raw `syscall(__NR_mmap, ...)`, bypasses libc

### 7.3 Dictionary — Lock-Free String Interning

**Purpose:** Map strings (class names, method names) to unique integer IDs.

```
DictTable (128 rows × 3 cells = 384 entries per level)
  ┌─────────────────────────────────────────┐
  │ DictRow[0]:  keys[0] keys[1] keys[2]   │
  │              next → DictTable (overflow) │
  │ DictRow[1]:  keys[0] keys[1] keys[2]   │
  │              next → NULL                 │
  │ ...                                      │
  │ DictRow[127]:keys[0] keys[1] keys[2]   │
  │              next → NULL                 │
  │                                          │
  │ base_index = 1                           │
  └─────────────────────────────────────────┘
```

**lookup() algorithm** (line 82):

```
1. hash = FNV-1a(key)
2. row = table->rows[hash % 128]
3. For each cell [0..2]:
   - If NULL: CAS(cell, NULL, allocateKey(key))
              If CAS wins: return unique index
              If CAS loses: free key, check if inserted key matches
   - If matches key: return index
4. If all 3 cells full: create overflow DictTable, chain from row->next
5. Rehash with rotated hash, repeat in overflow table
```

Key properties:
- **Append-only**: tables never resize (concurrent readers safe)
- **CAS on cells**: prevents duplicate insertions
- **Chaining**: overflow creates new tables linked via `row->next`
- **FNV-1a hash**: fast for short strings (class descriptors like `[B`, `()V`)
- **Note**: uses `malloc` for key strings — NOT signal-safe. Dictionary is used for method/class name recording, which happens outside signal handlers (via JVMTI callbacks).

### 7.4 SpinLock — `spinLock.h`

```c
class SpinLock {
    volatile int _lock;   // 0=free, 1=exclusive, <0=shared

    // Exclusive (used in signal handlers)
    bool tryLock()     { return __sync_bool_compare_and_swap(&_lock, 0, 1); }
    void lock()        { while (!tryLock()) spinPause(); }
    void unlock()      { __sync_fetch_and_sub(&_lock, 1); }

    // Shared (used for CodeCache traversal)
    bool tryLockShared()  { CAS(_lock, value, value-1) where value <= 0 }
    void unlockShared()   { __sync_fetch_and_add(&_lock, 1); }
};
```

Properties:
- `tryLock()` is the only variant called in signal handlers (never blocks)
- `__sync_*` builtins imply full memory barrier (acquire+release)
- Reader-writer support: negative values = shared readers, 1 = exclusive writer

### 7.5 CodeCache / CodeCacheArray — `codeCache.h`

Stores metadata for loaded libraries and JIT-compiled code:

```
CodeCache (one per native library / JVM code heap)
  ├─ CodeBlob* _blobs[]      (sorted array, binary search by address)
  │    └─ {_start, _end, _name}
  ├─ FrameDesc* _dwarf_table (parsed .eh_frame for DWARF unwinding)
  └─ void** _imports[]       (GOT entries for hook patching)
```

Binary search in sorted blob array gives O(log n) symbol lookup from any PC.

---

## 8. Memory Management

### 8.1 Allocation Strategies

| Context | Allocator | Underlying |
|---------|-----------|-----------|
| Signal handler (traces) | LinearAllocator | `OS::safeAlloc()` → raw `syscall(__NR_mmap)` |
| Signal handler (hash tables) | `LongHashTable::allocate` | `OS::safeAlloc()` → raw `syscall(__NR_mmap)` |
| Profiler startup (buffers) | libc `malloc` | Standard allocator |
| Profiler startup (events) | libc `calloc` | Standard allocator |
| String interning | libc `malloc` | Standard allocator (NOT signal-safe) |
| Symbol tables | libc `malloc` | Standard allocator |

### 8.2 OS::safeAlloc / OS::safeFree — `os_linux.cpp:257`

```c
void* OS::safeAlloc(size_t size) {
    // Raw syscall bypasses libc mmap (which may hold locks, be hooked, etc.)
    intptr_t result = syscall(__NR_mmap, NULL, size,
                              PROT_READ | PROT_WRITE,
                              MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (result < 0 && result > -4096) return NULL;  // errno check
    return (void*)result;
}

void OS::safeFree(void* addr, size_t size) {
    syscall(__NR_munmap, addr, size);
}
```

Why raw `syscall()` instead of libc `mmap()`? Two reasons, one practical and one critical.

**Reason 1: LD_PRELOAD interposition**

glibc's own `mmap()` wrapper is actually a thin function: it validates the offset, then calls the syscall. In stock glibc, calling `mmap()` from a signal handler would probably work fine.

However, `mmap` is a well-known interposition target. Memory allocators (jemalloc, tcmalloc), sanitizers (ASan, TSan), and various profiling tools commonly replace `mmap` via `LD_PRELOAD`. These replacement implementations may acquire locks, update bookkeeping data structures, or call other non-signal-safe functions. If a signal interrupts a thread that holds one of those locks, and the signal handler calls the interposed `mmap`, it can deadlock.

The raw `syscall(__NR_mmap, ...)` is immune to `LD_PRELOAD` — it goes directly to the kernel, bypassing any userspace wrappers or hooks.

**Reason 2: Infinite recursion when profiling `mmap` calls**

This is the critical reason, and the source code says it explicitly (`os_linux.cpp:258-259`): *"Naked syscall can be used inside a signal handler. Also, we don't want to catch our own calls when profiling mmap."*

async-profiler supports profiling `mmap` calls via hardware breakpoints (e.g., `-e mmap`). Here's how that works:

1. A `PERF_TYPE_BREAKPOINT` is set on the address of the libc `mmap` function (resolved via `dlsym(RTLD_DEFAULT, "mmap")` — see `perfEvents_linux.cpp:243`)
2. Every time any code calls `mmap()`, the CPU triggers the breakpoint → perf event overflow → kernel delivers SIGPROF → signal handler runs
3. The signal handler needs to allocate memory to store the captured stack trace (via `LinearAllocator::alloc()` → `OS::safeAlloc()`)

If `safeAlloc` called libc `mmap()`, it would hit the breakpoint again:

```
mmap() called by application
  → breakpoint fires → SIGPROF → signal handler
    → safeAlloc → mmap()                          ← hits breakpoint again!
      → breakpoint fires → SIGPROF → signal handler
        → safeAlloc → mmap()                       ← hits breakpoint again!
          → ... infinite recursion → stack overflow
```

The raw `syscall(__NR_mmap, ...)` bypasses the breakpointed function entirely. The breakpoint is set on the libc `mmap` function entry point address, not on the kernel `syscall` instruction. The raw syscall never executes the `mmap` function, so it never hits the breakpoint.

### 8.3 Memory Lifecycle

```
Profiler::start()
  ├─ malloc(CallTraceBuffer[16])        — pre-allocated frame buffers
  ├─ calloc(PerfEvent[pid_max])         — pre-allocated event array
  ├─ LinearAllocator(8MB chunks)        — ready for signal handler allocs
  └─ LongHashTable(65536 initial)       — ready for trace storage

Signal handlers:
  ├─ LinearAllocator::alloc()           — bump pointer (CAS)
  └─ LongHashTable grow (via safeAlloc) — rare, only when 75% full

Profiler::stop()
  ├─ LinearAllocator::clear()           — free all chunks (mmap pages)
  ├─ LongHashTable chain destruction    — free all tables
  └─ CallTraceBuffer kept               — reused across sessions
```

---

## 9. Thread Management

### 9.1 Thread Enumeration — `os_linux.cpp:35`

```c
class LinuxThreadList {
    DIR* _dir;

    LinuxThreadList() {
        _dir = opendir("/proc/self/task");     // all thread TIDs
    }

    int next() {
        struct dirent* entry;
        while ((entry = readdir(_dir)) != NULL) {
            if (entry->d_name[0] != '.')
                return atoi(entry->d_name);    // TID from directory name
        }
        return -1;
    }

    int size() {
        // Read from /proc/self/stat field 18 (thread count)
    }
};
```

**Thread state** from `/proc/self/task/[tid]/stat`:
- `R` or `D` → THREAD_RUNNING
- Anything else → THREAD_SLEEPING

**Thread name** from `/proc/self/task/[tid]/comm`

**Max thread ID** from `/proc/sys/kernel/pid_max` — used to size the per-thread arrays.

### 9.2 Automatic Thread Tracking — GOT Patching

**Problem:** New threads created after profiling starts need perf_events too.

**Solution:** Patch the JVM's GOT entry for `pthread_setspecific` (`cpuEngine.cpp:25`):

```c
// Find GOT entry in libjvm.so
_pthread_entry = libjvm->findImport(im_pthread_setspecific);

// enableThreadHook():
*_pthread_entry = (void*)pthread_setspecific_hook;  // redirect calls

// pthread_setspecific_hook():
static int pthread_setspecific_hook(pthread_key_t key, const void* value) {
    if (key != VMThread::key())
        return pthread_setspecific(key, value);  // not a VM thread, pass through

    if (value != NULL) {                         // thread starting
        int result = pthread_setspecific(key, value);
        CpuEngine::onThreadStart();              // → createForThread(gettid())
        return result;
    } else {                                     // thread ending
        CpuEngine::onThreadEnd();                // → destroyForThread(gettid())
        return pthread_setspecific(key, value);
    }
}

// disableThreadHook():
*_pthread_entry = (void*)pthread_setspecific;    // restore original
```

This intercepts HotSpot's TLS management to detect thread creation/destruction. The `_current` pointer uses `__atomic_store_n(RELEASE)` / `__atomic_load_n(ACQUIRE)` for safe publication.

Race condition prevention: `enableThreadHook()` is called BEFORE `createForAllThreads()` — any thread created between hook enable and enumeration will be caught by the hook. Any thread that already exists gets its event from enumeration. The `CAS` on `_events[tid]._fd` (from 0 to -1) prevents duplicate creation.

---

## 10. Symbol Resolution

### 10.1 Memory Map Parsing — `symbols_linux.cpp`

Reads `/proc/self/maps` to discover loaded libraries:

```
address range          perms offset  dev   inode  pathname
7f1234000000-7f1234100000 r-xp 00000000 08:01 12345 /usr/lib/libc.so.6
```

For each library:
1. Parse ELF header (validate magic `\x7fELF`)
2. Find `.dynsym` / `.symtab` sections
3. Find `.eh_frame_hdr` program header (DWARF CFI for unwinding)
4. Parse PLT relocations for import hooking

### 10.2 ELF Symbol Loading

```
ELF File
  ├─ .dynsym + .dynstr     → dynamic symbols (always available)
  ├─ .symtab + .strtab     → full symbol table (if not stripped)
  ├─ .eh_frame_hdr          → DWARF frame descriptions (for walkDwarf)
  ├─ .gnu.hash              → GNU hash table for efficient lookup
  └─ .rela.plt              → PLT relocations (for GOT patching)
```

Debug symbol search order:
1. In-binary `.symtab`
2. `/usr/lib/debug/.build-id/XX/YYYY.debug` (by Build-ID)
3. `/path/to/.debug/libname.debug` (by gnu_debuglink)
4. `/usr/lib/debug/path/to/libname.debug`

---

## 11. Concurrency Model

### 11.1 Lock Sharding

```
Profiler._locks[16]               ← SpinLock array
Profiler._calltrace_buffer[16]    ← one buffer per shard

Signal handler:
  lock_index = tid % 16
  if !_locks[lock_index].tryLock():
    try (lock_index + 1) % 16
    try (lock_index + 2) % 16
    all fail → drop sample
```

Why 16 shards: with `tryLock()` (non-blocking) and 3 attempts, the probability of all 3 being held simultaneously is extremely low. Even with 100+ threads being sampled concurrently, contention is minimal.

### 11.2 Atomic Primitives Used

| Primitive | Usage |
|-----------|-------|
| `__sync_bool_compare_and_swap` | SpinLock, hash table slots, timer IDs, GOT entries |
| `__sync_val_compare_and_swap` | LinearAllocator reserve/tail |
| `__sync_fetch_and_add` | Sample counters, table size, lock transitions |
| `__sync_fetch_and_sub` | SpinLock unlock |
| `__atomic_load_n(ACQUIRE)` | CallTrace pointer reading, `_current` engine |
| `__atomic_store_n(RELEASE)` | CallTrace pointer publishing, hook enable/disable |
| `volatile` | `Engine::_enabled`, `Chunk::offs`, `SpinLock::_lock` |

### 11.3 Memory Barriers

| Barrier | Where | Purpose |
|---------|-------|---------|
| `rmb()` = `lfence` | Ring buffer read | Ensure data visible after reading `data_head` |
| `__sync_*` (full barrier) | All CAS operations | Implicit acquire+release |
| `__atomic_*` | Trace storage | Explicit acquire/release for publishing |

---

## 12. Linux Syscall Reference

| Syscall | File | Purpose |
|---------|------|---------|
| `__NR_perf_event_open` | perfEvents_linux.cpp:573 | Create perf event counter |
| `__NR_mmap` | os_linux.cpp:260 | Signal-safe memory allocation (safeAlloc) |
| `__NR_munmap` | os_linux.cpp:268 | Signal-safe memory deallocation (safeFree) |
| `__NR_gettid` | os_linux.cpp:172 | Get thread ID |
| `__NR_tgkill` | os_linux.cpp:254 | Send signal to specific thread |
| `__NR_timer_create` | ctimer_linux.cpp:46 | Create per-thread CPU timer |
| `__NR_timer_settime` | ctimer_linux.cpp:61 | Arm timer with interval |
| `__NR_timer_delete` | ctimer_linux.cpp:53 | Destroy timer |
| `mmap` (libc) | perfEvents_linux.cpp:587 | Map perf_event ring buffer |
| `ioctl` | perfEvents_linux.cpp:605,669-670 | PERF_EVENT_IOC_{RESET,REFRESH,DISABLE} |
| `fcntl` | perfEvents_linux.cpp:602 | F_SETFL, F_SETSIG, F_SETOWN_EX |
| `sigaction` | os_linux.cpp:240 | Install signal handler |
| `setitimer` | itimer.cpp:50 | ITIMER_PROF for process CPU time |
| `clock_gettime` | os_linux.cpp:115 | CLOCK_MONOTONIC timestamps |
| `open/read/close` | os_linux.cpp, symbols_linux.cpp | /proc/self/task, /proc/self/maps, ELF files |
| `opendir/readdir` | os_linux.cpp:63 | Thread enumeration via /proc/self/task |
| `getrlimit/setrlimit` | perfEvents_linux.cpp:146 | Raise RLIMIT_NOFILE for per-thread FDs |
| `sched_getscheduler` | os_linux.cpp:176 | Thread scheduling policy |
| `sendfile` | os_linux.cpp:329 | File copying |
| `posix_fadvise` | os_linux.cpp:338 | Free page cache |

### /proc and /sys Files Read

| Path | Purpose |
|------|---------|
| `/proc/self/task/` | Enumerate threads (readdir for TIDs) |
| `/proc/self/task/[tid]/comm` | Thread name |
| `/proc/self/task/[tid]/stat` | Thread state (R/D/S) |
| `/proc/self/stat` | Thread count (field 18) |
| `/proc/self/maps` | Loaded libraries and memory regions |
| `/proc/sys/kernel/pid_max` | Maximum thread ID |
| `/proc/sys/kernel/perf_event_paranoid` | Perf event availability check |
| `/proc/cpuinfo` | CPU description |
| `/proc/stat` | Global CPU statistics |
| `/proc/kallsyms` | Kernel symbol table |
| `/sys/bus/event_source/devices/*/type` | PMU device type |
| `/sys/bus/event_source/devices/*/events/*` | PMU event descriptors |
| `/sys/bus/event_source/devices/*/format/*` | PMU config field layout |
| `/sys/kernel/debug/tracing/events/*/id` | Tracepoint IDs |

---

## 13. Lessons for Python CPU Profiler

### Directly Applicable Patterns

1. **Signal-based sampling**: Use `SIGPROF` with `perf_event_open` (best) or `timer_create` per thread (fallback) or `setitimer` (simplest). The 3-tier engine hierarchy (PerfEvents > CTimer > ITimer) provides graceful degradation.

2. **Async-signal-safety via raw syscalls**: Use `syscall(__NR_mmap, ...)` instead of `mmap()` for memory allocation inside signal handlers. Never call libc allocator functions in signal context.

3. **Non-blocking lock acquisition**: Use `tryLock()` with CAS in signal handlers. 16-shard lock array with 3-attempt retry gives excellent concurrency with zero blocking. Dropping samples under contention is acceptable — profile data is statistical.

4. **Bump allocator for traces**: LinearAllocator with CAS bump pointer is perfect for signal-safe allocation of variable-size stack traces. Pre-allocate reserve chunks to avoid allocation latency.

5. **Lock-free hash table for dedup**: MurmurHash64A + CAS-based open addressing with quadratic probing. Grow by linking new tables (never resize in place). Track sample counts with atomic increments.

6. **perf_event one-shot model**: `REFRESH(1)` ensures no recursive signals. Re-arm in signal handler after processing.

7. **Per-thread event management**: Track threads via `/proc/self/task/` enumeration at startup, plus GOT hooking for dynamic thread tracking. Use `F_SETOWN_EX` with `F_OWNER_TID` to target specific threads.

### Python-Specific Adaptations Needed

1. **No ASGCT equivalent**: Python doesn't have AsyncGetCallTrace. Instead, read `PyThreadState` → `PyFrameObject` chain directly from the signal handler. Must be careful about GIL state and frame validity.

2. **Simpler stack walking**: Python frames are a linked list (`f_back` pointers), not native frames requiring FP/DWARF unwinding. Native C extension frames would need FP walking.

3. **GIL-aware sampling**: Python's GIL means only one thread runs Python code at a time. Need to decide: sample the GIL holder, or sample all threads (wall-clock style)?

4. **No GOT hooking needed**: Python provides `threading` callbacks or `PyThread` API for tracking thread creation. Or simply enumerate `/proc/self/task/` periodically.

5. **Symbol resolution simpler**: Python function names come from code objects (`co_filename`, `co_name`, `co_firstlineno`), not ELF symbol tables. No DWARF needed for Python frames.
