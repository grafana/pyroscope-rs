# V8 CPU Profiler Design Analysis

**V8 version**: 14.7.134 (latest stable tag, March 2026)
**Focus**: Linux CPU profiling via signal-based sampling only

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Thread Model](#thread-model)
3. [OS APIs Used (Linux)](#os-apis-used-linux)
4. [Signal Handler & Async-Signal Safety](#signal-handler--async-signal-safety)
5. [Data Structures](#data-structures)
6. [Memory Allocation & Management](#memory-allocation--management)
7. [The Sampling Pipeline (End-to-End)](#the-sampling-pipeline-end-to-end)
8. [Stack Walking in Signal Context](#stack-walking-in-signal-context)
9. [Code Event Tracking & Symbolization](#code-event-tracking--symbolization)
10. [Key Design Decisions & Tradeoffs](#key-design-decisions--tradeoffs)
11. [Source File Map](#source-file-map)

---

## Architecture Overview

V8's CPU profiler is a **sampling profiler** that periodically interrupts the JS thread to capture stack traces. The high-level architecture follows a **producer-consumer** pattern with three distinct roles:

```
┌──────────────────────────────────────────────────────────┐
│                    Profiler Thread                        │
│  (SamplingEventsProcessor::Run)                          │
│                                                          │
│  1. Sleep for sampling interval                          │
│  2. Call sampler_->DoSample()                            │
│     └─> sends SIGPROF to JS thread via pthread_kill()    │
│  3. Consume samples from circular buffer                 │
│  4. Consume code events from locked queue                │
│  5. Symbolize samples (address -> function name)         │
│  6. Add symbolized samples to CpuProfile tree            │
│                                                          │
└──────────────────────────────────────────────────────────┘
         │ SIGPROF                  ▲ samples via
         ▼                         │ SamplingCircularQueue
┌──────────────────────────────────────────────────────────┐
│                    JS/VM Thread                           │
│                                                          │
│  Signal Handler (HandleProfilerSignal):                  │
│  1. Extract PC/SP/FP from ucontext_t                     │
│  2. Walk the stack (StackFrameIteratorForProfiler)        │
│  3. Write TickSample into circular buffer                │
│                                                          │
│  Normal execution:                                       │
│  - Emits code events (create/move/delete) to             │
│    LockedQueue when profiling is active                   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Thread Model

### 1. JS/VM Thread (the profiled thread)
- Executes JavaScript and native code
- Receives SIGPROF signals
- Runs the signal handler which collects stack samples
- Emits code events (code creation, movement, deletion) during JIT compilation and GC

### 2. Profiler Thread (`v8:ProfEvntProc`)
- A dedicated `base::Thread` (see `ProfilerEventsProcessor`)
- Stack size: 256 KB (`kProfilerStackSize`)
- Responsible for:
  - Timing and triggering samples via `Sampler::DoSample()`
  - Consuming tick samples from the circular buffer
  - Consuming code events from the locked queue
  - Symbolizing raw addresses into function names
  - Building the profile tree

### Key Insight: Signal Delivery Model
V8 does **not** use `setitimer()` or `timer_create()`. Instead:
1. The profiler thread **sleeps** for the sampling interval using `ConditionVariable::WaitFor()`
2. When the interval expires, it calls `sampler_->DoSample()`
3. `DoSample()` calls `pthread_kill(target_thread, SIGPROF)` to deliver the signal to the specific JS thread

This is a deliberate design: the profiler thread controls the timing, and `pthread_kill()` targets a specific thread (not the whole process), avoiding the ambiguity of `setitimer()` which delivers to an arbitrary thread.

```
// sampler.cc:603-608
void Sampler::DoSample() {
  base::RecursiveMutexGuard lock_guard(SignalHandler::mutex());
  if (!SignalHandler::Installed()) return;
  SetShouldRecordSample();
  pthread_kill(platform_data()->vm_tself(), SIGPROF);
}
```

## OS APIs Used (Linux)

### Signal Management
| API | Usage | Location |
|-----|-------|----------|
| `sigaction(SIGPROF, ...)` | Install signal handler with `SA_SIGINFO \| SA_RESTART \| SA_ONSTACK` flags | `sampler.cc:348-358` |
| `pthread_kill(tid, SIGPROF)` | Send SIGPROF to specific thread | `sampler.cc:607` |
| `sigemptyset()` | Initialize empty signal mask for handler | `sampler.cc:352` |

### Thread Identification
| API | Usage | Location |
|-----|-------|----------|
| `pthread_self()` | Store thread handle for `pthread_kill` targeting | `sampler.cc:208` |
| `base::OS::GetCurrentThreadId()` | Get numeric thread ID for sampler map lookup | `sampler.cc:208` |
| `syscall(SYS_gettid)` | (underlying impl) Get Linux thread ID | via platform abstraction |

### Thread Synchronization (Profiler Thread)
| API | Usage | Location |
|-----|-------|----------|
| `pthread_create` | Create profiler thread | via `base::Thread::Start()` |
| `pthread_join` | Wait for profiler thread on stop | via `base::Thread::Join()` |
| `pthread_mutex_*` | ConditionVariable and Mutex internals | `cpu-profiler.cc:280-329` |
| `pthread_cond_timedwait` | Sleep with interruptible wait for sampling interval | `cpu-profiler.cc:319-328` |

### Context Extraction (Signal Handler)
| API | Usage | Location |
|-----|-------|----------|
| `ucontext_t` / `mcontext_t` | Extract register state (PC, SP, FP) from signal context | `sampler.cc:405-476` |

Register extraction for Linux x86_64:
```c
// sampler.cc:419-422
state->pc = reinterpret_cast<void*>(mcontext.gregs[REG_RIP]);
state->sp = reinterpret_cast<void*>(mcontext.gregs[REG_RSP]);
state->fp = reinterpret_cast<void*>(mcontext.gregs[REG_RBP]);
```

### Signal Handler Flags
```c
sa.sa_flags = SA_RESTART | SA_SIGINFO | SA_ONSTACK;
```
- **`SA_SIGINFO`**: Provides `siginfo_t` and `ucontext_t` to the handler (needed for register state)
- **`SA_RESTART`**: Automatically restart interrupted system calls
- **`SA_ONSTACK`**: Use alternate signal stack if one is set up (avoids stack overflow on small stacks)

## Signal Handler & Async-Signal Safety

### The Central Problem
The signal handler interrupts the JS thread at an **arbitrary point**, including in the middle of:
- Heap allocations
- GC
- JIT compilation
- Stack frame setup/teardown

V8 must collect a stack trace without calling any async-signal-unsafe functions (e.g., `malloc`, `printf`, `pthread_mutex_lock`).

### V8's Async-Signal Safety Strategy

#### 1. Pre-allocated Circular Buffer (No malloc in signal handler)
The `SamplingCircularQueue` is **pre-allocated** at profiler start. The signal handler writes directly into a pre-allocated slot:

```cpp
// cpu-profiler-inl.h:73-79
TickSample* SamplingEventsProcessor::StartTickSample() {
  void* address = ticks_buffer_.StartEnqueue();  // lock-free, no alloc
  if (address == nullptr) return nullptr;         // buffer full, drop sample
  TickSampleEventRecord* evt =
      new (address) TickSampleEventRecord(last_code_event_id_);
  return &evt->sample;
}
```

The `new (address)` is **placement new** — no heap allocation, just in-place construction.

#### 2. AtomicGuard Instead of Mutexes
The `SamplerManager::DoSample()` is called from the signal handler. It cannot use mutexes. Instead, V8 uses a **non-blocking atomic spinlock**:

```cpp
// sampler.cc:246-261
void SamplerManager::DoSample(const v8::RegisterState& state) {
  AtomicGuard atomic_guard(&samplers_access_counter_, false);  // non-blocking!
  if (!atomic_guard.is_success()) return;  // bail if can't acquire
  int thread_id = base::OS::GetCurrentThreadId();
  auto it = sampler_map_.find(thread_id);
  if (it == sampler_map_.end()) return;
  SamplerList& samplers = it->second;
  for (Sampler* sampler : samplers) {
    if (!sampler->ShouldRecordSample()) continue;
    Isolate* isolate = sampler->isolate();
    if (isolate == nullptr || !isolate->IsInUse()) continue;
    sampler->SampleStack(state);
  }
}
```

The `AtomicGuard` with `is_blocking=false` uses `compare_exchange_strong` — if it fails to acquire, it **immediately returns** and drops the sample. This prevents deadlock if the signal fires while `AddSampler`/`RemoveSampler` holds the lock.

```cpp
// sampler.cc:188-196
AtomicGuard::AtomicGuard(AtomicMutex* atomic, bool is_blocking)
    : atomic_(atomic), is_success_(false) {
  do {
    bool expected = false;
    is_success_ = atomic->compare_exchange_strong(expected, true);
  } while (is_blocking && !is_success_);
}
```

#### 3. Atomic Record-Sample Flag
`Sampler::DoSample()` sets an atomic flag before sending the signal:
```cpp
void Sampler::DoSample() {
  ...
  SetShouldRecordSample();     // atomic store
  pthread_kill(..., SIGPROF);
}
```
The signal handler checks and consumes this flag:
```cpp
bool ShouldRecordSample() {
  return record_sample_.exchange(false, std::memory_order_relaxed);
}
```
This ensures only one sample is produced per `DoSample()` call.

#### 4. GC Bailout
Stack walking during GC is unsafe because objects are being moved:
```cpp
// tick-sample.cc:233-242
if (sample_info->vm_state == GC || v8_isolate->heap()->IsInGC()) {
  return true;  // Skip this sample entirely
}
```

#### 5. Frame Setup Detection (IsNoFrameRegion)
If the signal arrives while a function prologue/epilogue is executing (e.g., `push %rbp; mov %rsp, %rbp`), the frame pointer is invalid. V8 detects this by pattern-matching instruction bytes at PC:

```cpp
// tick-sample.cc:32-82
bool IsNoFrameRegion(i::Address address) {
  // Checks for x86_64:
  //   push %rbp; mov %rsp,%rbp  (prologue)
  //   pop %rbp; ret             (epilogue)
  // If PC is within these patterns, bail out
}
```

This check only examines bytes **on the same page** as PC to avoid SEGFAULT on unmapped memory.

#### 6. ASAN/MSAN Integration
Stack pointer values may point to "poisoned" memory under sanitizers:
```cpp
// tick-sample.cc:197-198
ASAN_UNPOISON_MEMORY_REGION(regs.sp, sizeof(void*));
MSAN_MEMORY_IS_INITIALIZED(regs.sp, sizeof(void*));
```
The `TickSample::Init` function itself is marked `DISABLE_ASAN`.

#### 7. No V8_EXPORT_PRIVATE Functions in Signal Path
```cpp
// tick-sample.cc:215-218 (comment)
// IMPORTANT: 'GetStackSample' is sensitive to stack overflows. For this reason
// we try not to use any function/method marked as V8_EXPORT_PRIVATE with their
// only use-site in 'GetStackSample': The resulting linker stub needs quite
// a bit of stack space and has caused stack overflow crashes in the past.
```

#### 8. Abseil Deadlock Detection Disabled
```cpp
// sampler.cc:578
Sampler::Sampler(Isolate* isolate)
    : isolate_(isolate), data_(std::make_unique<PlatformData>()) {
  SetMutexDeadlockDetectionMode(absl::OnDeadlockCycle::kIgnore);
}
```
If the signal handler interrupts Abseil's internal lock, deadlock detection could itself deadlock.

### Summary: What Runs in Signal Context

| Operation | Async-Signal Safe? | How? |
|-----------|-------------------|------|
| Register extraction from `ucontext_t` | Yes | Pure struct reads |
| `AtomicGuard` (try-lock) | Yes | `atomic::compare_exchange_strong`, no mutex |
| `ShouldRecordSample()` | Yes | `atomic::exchange` |
| `base::OS::GetCurrentThreadId()` | Yes | Uses `gettid()` syscall (async-signal safe) |
| `sampler_map_.find()` | **Not strictly** | Relies on `AtomicGuard` preventing concurrent modification |
| `StartTickSample()` / `FinishEnqueue()` | Yes | Lock-free atomic operations on pre-allocated buffer |
| `TickSample::Init` / `GetStackSample` | **Carefully safe** | No allocations, no locks; reads stack memory defensively |
| `StackFrameIteratorForProfiler` | **Carefully safe** | Validates addresses before dereferencing; no allocations |

## Data Structures

### 1. SamplingCircularQueue (Lock-Free SPSC Ring Buffer)
**File**: `circular-queue.h`, `circular-queue-inl.h`
**Purpose**: Transfer tick samples from signal handler (producer) to profiler thread (consumer)

```
┌────────┬────────┬────────┬────────┬────────┬────────┐
│ Entry0 │ Entry1 │ Entry2 │ Entry3 │ ...    │ EntryN │
│ marker │ marker │ marker │ marker │        │ marker │
│ record │ record │ record │ record │        │ record │
└────────┴────────┴────────┴────────┴────────┴────────┘
     ▲                          ▲
     │                          │
  dequeue_pos_              enqueue_pos_
  (consumer)                (producer/signal handler)
```

**Key properties**:
- **Single producer, single consumer (SPSC)** — signal handler writes, profiler thread reads
- **Lock-free**: Uses `base::Acquire_Load` / `base::Release_Store` atomic ops on per-entry `marker` field
- **Cache-line aligned entries**: `alignas(PROCESSOR_CACHE_LINE_SIZE)` to prevent false sharing
- **Fixed size, pre-allocated**: 512 KB buffer, ~`512KB / sizeof(TickSampleEventRecord)` entries
- **No-allocation enqueue**: Signal handler calls `StartEnqueue()` to get a pointer, writes in-place, then `FinishEnqueue()`
- **If full, drops the sample** (returns nullptr from `StartEnqueue()`)

Entry marker states:
- `kEmpty` (0): Slot available for producer
- `kFull` (1): Slot has data ready for consumer

Protocol:
```
Producer (signal handler):
  1. SeqCst fence
  2. if (Acquire_Load(enqueue_pos_->marker) == kEmpty) → get pointer to record
  3. Write sample data into record
  4. Release_Store(enqueue_pos_->marker, kFull)
  5. Advance enqueue_pos_ (wrap around)

Consumer (profiler thread):
  1. SeqCst fence
  2. if (Acquire_Load(dequeue_pos_->marker) == kFull) → return pointer to record
  3. Read sample data
  4. Release_Store(dequeue_pos_->marker, kEmpty)
  5. Advance dequeue_pos_ (wrap around)
```

### 2. LockedQueue (Lock-Based MPMC Queue)
**File**: `locked-queue.h`
**Purpose**: Transfer code events (create/move/delete) from VM thread to profiler thread

Based on Michael & Scott's concurrent queue algorithm. Uses separate `head_mutex_` and `tail_mutex_` for fine-grained locking. This is **not** used in signal context — only from the normal VM thread.

### 3. TickSample (Fixed-Size Stack Snapshot)
**File**: `tick-sample.h`
**Purpose**: Capture one sample of the program's state

```cpp
struct TickSample {
  void* pc;                               // Instruction pointer
  void* tos / external_callback_entry;    // Top of stack or external callback
  void* context;                          // Native context address
  base::TimeTicks timestamp;
  base::TimeDelta sampling_interval_;
  StateTag state;                         // JS, GC, COMPILER, etc.
  uint16_t frames_count;
  void* stack[kMaxFramesCount];           // Call stack (up to 255 frames)
};
```

`kMaxFramesCount = 255` (2^8 - 1). The entire structure is of **fixed width** to work with the circular buffer.

### 4. TickSampleEventRecord (Sequenced Sample)
```cpp
class TickSampleEventRecord {
  unsigned order;        // Monotonic sequence number from last_code_event_id_
  TickSample sample;
};
```
The `order` field links samples to code events — a sample with `order=N` was taken when the latest code event had ID N. The profiler thread processes code events and samples **in order**, ensuring it symbolizes against the correct code map state.

### 5. InstructionStreamMap (Address → CodeEntry)
**File**: `profile-generator.h`
```cpp
class InstructionStreamMap {
  std::multimap<Address, CodeEntryMapInfo> code_map_;
};
```
Maps instruction addresses to `CodeEntry` objects. Used by the **profiler thread only** (not signal-safe). Maintained by processing code events (create/move/delete).

Uses `std::multimap` to handle overlapping code regions. Lookup is `O(log n)`.

### 6. ProfileTree (Calling Context Tree)
**File**: `profile-generator.h`
```cpp
class ProfileTree {
  ProfileNode* root_;
  // Each node: CodeEntry* + children map + self_ticks_
};

class ProfileNode {
  CodeEntry* entry_;
  unsigned self_ticks_;
  std::unordered_map<CodeEntryAndPosition, ProfileNode*, Hasher, Equals> children_;
  std::vector<ProfileNode*> children_list_;
  ProfileNode* parent_;
};
```
A **calling context tree (CCT)** where each path from root to leaf represents a unique call stack observed during profiling. Nodes accumulate tick counts.

### 7. SamplerManager (Global Sampler Registry)
**File**: `sampler.h`
```cpp
class SamplerManager {
  std::unordered_map<int, SamplerList> sampler_map_;  // thread_id -> samplers
  AtomicMutex samplers_access_counter_;
};
```
Global singleton (via `base::LeakyObject`) that maps thread IDs to active samplers. Protected by non-blocking atomic lock since it's accessed from signal handler.

## Memory Allocation & Management

### Allocation Strategy by Component

| Component | Allocation | Lifetime | Thread |
|-----------|-----------|----------|--------|
| `SamplingCircularQueue` buffer | Aligned alloc at profiler start | Profiler session | Created on profiler thread |
| `TickSample` in circular queue | Placement new in pre-allocated slot | Consumed then slot reused | Written in signal handler |
| `TickSampleEventRecord` in locked queue | Node-based `new` | Dequeued and deleted by profiler thread | Allocated on VM thread |
| `CodeEntry` objects | `new` via `CodeEntryStorage::Create()` | Ref-counted, freed when unused | VM thread creates, profiler thread reads |
| `InstructionStreamMap` | Standard heap (`std::multimap`) | Profiler session | Profiler thread only |
| `ProfileTree` / `ProfileNode` | Standard heap (`new`) | Profiler session | Profiler thread only |
| `StringsStorage` | Heap-allocated copies of strings | Profiler session, ref-counted | Protected by mutex |
| `SamplerManager` | `base::LeakyObject` (leaked singleton) | Process lifetime | Global |
| `SignalHandler` state | Static globals | Process lifetime | Protected by `RecursiveMutex` |

### SamplingEventsProcessor Custom Allocator
The `SamplingEventsProcessor` overrides `operator new`/`delete` because `SamplingCircularQueue` requires stricter alignment (cache-line aligned entries):

```cpp
// cpu-profiler.cc:355-359
void* SamplingEventsProcessor::operator new(size_t size) {
  return AlignedAllocWithRetry(size, alignof(SamplingEventsProcessor));
}
void SamplingEventsProcessor::operator delete(void* ptr) { AlignedFree(ptr); }
```

### CodeEntry Reference Counting
`CodeEntry` objects are shared between the `InstructionStreamMap` and `ProfileTree`. They use manual reference counting:

```cpp
class CodeEntryStorage {
  static CodeEntry* Create(Args&&... args) {
    CodeEntry* entry = new CodeEntry(...);
    entry->mark_ref_counted();  // sets ref_count_ = 1
    return entry;
  }
  void AddRef(CodeEntry*);  // ++ref_count_
  void DecRef(CodeEntry*);  // --ref_count_, delete if 0
};
```

### StringsStorage (Interned Strings)
Function and resource names are **interned** (deduplicated) in a hash map:
```cpp
class StringsStorage {
  base::CustomMatcherHashMap names_;  // deduplication map
  base::Mutex mutex_;                 // thread-safe access
};
```
Strings are copied to the C++ heap so they survive even if the original JS strings are garbage collected. Reference counted for cleanup.

### WeakCodeRegistry (GC Integration)
Tracks `CodeEntry` → heap object associations via weak references. When code objects are GC'd, the registry notifies the profiler to clean up:
```cpp
class WeakCodeRegistry {
  std::vector<CodeEntry*> entries_;
  void Sweep(Listener* listener);  // Called after mark-sweep
};
```

## The Sampling Pipeline (End-to-End)

### Phase 1: Profiler Start
```
CpuProfiler::StartProfiling()
  └─> StartProcessorIfNotStarted()
      ├─> Symbolizer = new Symbolizer(code_map)
      ├─> processor_ = new SamplingEventsProcessor(...)
      │   └─> SamplingEventsProcessor constructor:
      │       ├─> sampler_ = new CpuSampler(isolate, this)
      │       └─> sampler_->Start()
      │           ├─> SignalHandler::IncreaseSamplerCount()
      │           │   └─> sigaction(SIGPROF, {HandleProfilerSignal, SA_SIGINFO|SA_RESTART|SA_ONSTACK})
      │           └─> SamplerManager::instance()->AddSampler(this)
      └─> processor_->StartSynchronously()  // Launches the thread
```

### Phase 2: Sampling Loop (Profiler Thread)
```
SamplingEventsProcessor::Run()
  while (running_):
    1. Calculate nextSampleTime = Now() + period_
    2. Process existing samples and code events:
       do:
         result = ProcessOneSample()
         if (FoundSampleForNextCodeEvent):
           ProcessCodeEvent()  // Dequeue from LockedQueue, update code_map_
       while (result != NoSamplesInQueue && Now() < nextSampleTime)
    3. Sleep until nextSampleTime (interruptible via ConditionVariable)
    4. sampler_->DoSample()
       └─> pthread_kill(vm_thread, SIGPROF)
```

### Phase 3: Signal Handler (JS Thread, in Signal Context)
```
SignalHandler::HandleProfilerSignal(signal, info, context)
  ├─> FillRegisterState(context, &state)  // Extract PC/SP/FP from ucontext_t
  └─> SamplerManager::instance()->DoSample(state)
      ├─> AtomicGuard (non-blocking try-lock)
      ├─> Find samplers for current thread
      └─> For each sampler:
          └─> sampler->SampleStack(state)  // CpuSampler::SampleStack
              ├─> processor_->StartTickSample()  // Get slot in circular buffer
              ├─> sample->Init(isolate, regs, ...)
              │   └─> TickSample::GetStackSample(...)
              │       ├─> Check: Not in GC? Isolate entered?
              │       ├─> Check: Not in frame setup/teardown?
              │       ├─> StackFrameIteratorForProfiler(isolate, pc, fp, sp, lr, js_entry_sp)
              │       └─> For each frame: frames[i++] = frame->unauthenticated_pc()
              └─> processor_->FinishTickSample()  // Mark slot as full
```

### Phase 4: Sample Consumption & Symbolization (Profiler Thread)
```
SamplingEventsProcessor::ProcessOneSample()
  ├─> Check VM-originated ticks (ticks_from_vm_buffer_) first
  ├─> Check signal-originated ticks (ticks_buffer_) next
  └─> SymbolizeAndAddToProfiles(record)
      ├─> symbolizer_->SymbolizeTickSample(tick_sample)
      │   ├─> For each PC in sample.stack[]:
      │   │   └─> FindEntry(address) in InstructionStreamMap
      │   │       └─> Returns CodeEntry* with function name, line info
      │   └─> Unwind inline stacks if present
      └─> profiles_->AddPathToCurrentProfiles(timestamp, stack_trace, ...)
          └─> For each active CpuProfile:
              └─> profile->AddPath(...)
                  └─> top_down_.AddPathFromEnd(path)  // Insert into CCT
```

### Phase 5: Profiler Stop
```
CpuProfiler::StopProfiling(id)
  ├─> StopProcessor()
  │   ├─> running_.store(false)
  │   ├─> running_cond_.NotifyOne()  // Wake up sleeping profiler thread
  │   ├─> processor_->Join()          // Wait for thread to finish
  │   └─> (Profiler thread drains remaining samples before exiting)
  └─> profiles_->StopProfiling(id)
      └─> Returns CpuProfile with the full CCT
```

## Stack Walking in Signal Context

### StackFrameIteratorForProfiler
**File**: `frames.h:1917-1964`

This is V8's **most safety-critical** code for profiling. It walks the stack during signal handler execution, where the program state may be inconsistent.

```cpp
class StackFrameIteratorForProfiler : public StackFrameIteratorBase {
  const Address low_bound_;   // Bottom of stack
  const Address high_bound_;  // Top of stack (js_entry_sp)
  StackFrame::Type top_frame_type_;
  ExternalCallbackScope* external_callback_scope_;
  Address top_link_register_;
};
```

**Safety checks performed**:
1. **Address validation**: `IsValidStackAddress(addr)` — every FP/SP must be within `[low_bound_, high_bound_]`
2. **Frame type validation**: `IsValidFrameType(type)` — rejects unknown frame types
3. **Exit frame validation**: `IsValidExitFrame(fp)` — validates exit frames match expectations
4. **Entry frame validation**: `HasValidExitIfEntryFrame(frame)` — ensures entry frames have valid exits

### Interpreted Frame Special Handling
For bytecode-interpreted frames, V8 reads the bytecode array pointer and offset directly from the frame:
```cpp
// tick-sample.cc:364-376
i::Address bytecode_array = base::Memory<i::Address>(
    frame->fp() + InterpreterFrameConstants::kBytecodeArrayFromFp);
i::Address bytecode_offset = base::Memory<i::Address>(
    frame->fp() + InterpreterFrameConstants::kBytecodeOffsetFromFp);

// Defensive: only use if they look like valid tagged pointers
if (HAS_STRONG_HEAP_OBJECT_TAG(bytecode_array) &&
    HAS_SMI_TAG(bytecode_offset)) {
  frames[i++] = reinterpret_cast<void*>(
      bytecode_array + Internals::SmiValue(bytecode_offset));
}
```
This avoids dereferencing the bytecode array object (which might be garbage if GC is moving objects).

## Code Event Tracking & Symbolization

### The Ordering Problem
Code can be created, moved, or deleted while profiling. The profiler must symbolize samples against the code map state that existed **when the sample was taken**.

V8 solves this with **monotonic sequence numbers**:

1. Every code event gets an incrementing `order` number via `last_code_event_id_`
2. Every tick sample records the current `last_code_event_id_` at capture time
3. The profiler thread processes events in order:
   - Process all code events up to the sample's order number
   - Then symbolize the sample

```cpp
// cpu-profiler.cc:256-278
SamplingEventsProcessor::ProcessOneSample() {
  // Check if a sample matches the current code event state
  const TickSampleEventRecord* record = ticks_buffer_.Peek();
  if (record->order != last_processed_code_event_id_) {
    return FoundSampleForNextCodeEvent;  // Need more code events first
  }
  SymbolizeAndAddToProfiles(record);
  ticks_buffer_.Remove();
}
```

### Two Tick Sources
V8 has two sources of tick samples:
1. **`ticks_buffer_`** (SamplingCircularQueue): From signal handler, lock-free
2. **`ticks_from_vm_buffer_`** (LockedQueue): From VM thread directly (e.g., `AddCurrentStack()`, `AddDeoptStack()`)

VM-originated ticks have priority and are checked first.

### Code Event Types
```cpp
enum Type {
  kCodeCreation,     // New code compiled
  kCodeMove,         // Code moved (by GC)
  kCodeDisableOpt,   // Optimization disabled
  kCodeDeopt,        // Deoptimization occurred
  kReportBuiltin,    // Builtin function registered
  kCodeDelete,       // Code deleted
  kNativeContextMove // Native context address changed
};
```

## Key Design Decisions & Tradeoffs

### 1. `pthread_kill` vs `setitimer`
**Choice**: `pthread_kill()` from profiler thread
**Why**: `setitimer()` delivers signals to an arbitrary thread in the process; `pthread_kill()` targets the exact thread being profiled. This is essential for multi-isolate scenarios and accurate per-thread profiling.
**Tradeoff**: Requires a dedicated profiler thread per isolate.

### 2. Drop Samples Rather Than Block
**Choice**: Non-blocking atomic guard + nullable circular buffer enqueue
**Why**: The signal handler cannot block — if it tries to acquire a held lock, the process deadlocks (the lock holder is the same thread, interrupted by the signal). Dropping occasional samples is acceptable for a statistical profiler.
**Impact**: The `ProfilerStats` class tracks drop reasons for diagnostics.

### 3. Fixed-Size TickSample (255 frame limit)
**Choice**: `kMaxFramesCount = 255` (8-bit), fixed-size stack array
**Why**: Fixed-size enables the lock-free circular buffer. Variable-size records would require allocation in the signal handler.
**Tradeoff**: Deep call stacks are truncated.

### 4. Cache-Line Aligned Circular Queue Entries
**Choice**: `alignas(PROCESSOR_CACHE_LINE_SIZE)` on each entry
**Why**: Prevents false sharing between producer (signal handler) and consumer (profiler thread). Without this, writing the marker in one entry could invalidate the cache line containing an adjacent entry being read.

### 5. Interruptible Sleep via ConditionVariable
**Choice**: `ConditionVariable::WaitFor()` instead of `usleep()`/`nanosleep()`
**Why**: The profiler thread needs to wake up immediately when profiling is stopped, not sleep for the full interval. The condition variable allows `StopSynchronously()` to notify and immediately join.

### 6. Ordering Discipline Between Code Events and Samples
**Choice**: Monotonic sequence numbers linking samples to code events
**Why**: Without this, a sample taken just after code is moved would be symbolized against stale addresses, producing incorrect function names.

### 7. Defensive Stack Walking
**Choice**: Extensive validation before every frame pointer dereference
**Why**: The signal can interrupt during frame setup/teardown, leaving FP/SP in inconsistent states. Invalid stack walks would crash the process or produce garbage. V8 validates every address and frame type, bailing out if anything looks wrong.

### 8. Separate String Interning
**Choice**: `StringsStorage` copies strings to C++ heap
**Why**: Function names on the JS heap can be GC'd. The profiler needs stable string references that outlive individual code objects. Interning also deduplicates to save memory.

## Source File Map

### Core Profiler Files
| File | Purpose |
|------|---------|
| `src/profiler/cpu-profiler.h/.cc` | Main CpuProfiler class, SamplingEventsProcessor (profiler thread), CpuSampler |
| `src/profiler/cpu-profiler-inl.h` | Inline methods for circular queue operations, code map updates |
| `src/libsampler/sampler.h/.cc` | Signal handler, Sampler base class, SamplerManager, AtomicGuard, platform-specific register extraction |

### Data Structures
| File | Purpose |
|------|---------|
| `src/profiler/circular-queue.h/-inl.h` | Lock-free SPSC circular queue for tick samples |
| `src/utils/locked-queue.h/-inl.h` | Lock-based MPMC queue for code events |
| `src/profiler/tick-sample.h/.cc` | TickSample struct, stack walking (`GetStackSample`), frame validation |
| `src/profiler/profile-generator.h/.cc` | ProfileTree (CCT), ProfileNode, InstructionStreamMap, CodeEntry, CodeEntryStorage, CpuProfile |

### Symbolization & Code Events
| File | Purpose |
|------|---------|
| `src/profiler/symbolizer.h/.cc` | Converts raw addresses to CodeEntry function names, unwinds inline stacks |
| `src/profiler/profiler-listener.h/.cc` | Listens for code events (JIT compile, move, delete) and converts to CodeEventsContainer |
| `src/profiler/strings-storage.h/.cc` | Thread-safe interned string storage for function/resource names |
| `src/profiler/weak-code-registry.h/.cc` | Tracks code objects via weak references for GC integration |

### Stack Frame Infrastructure
| File | Purpose |
|------|---------|
| `src/execution/frames.h` | StackFrameIteratorForProfiler — signal-safe stack walker with extensive validation |

### Diagnostics
| File | Purpose |
|------|---------|
| `src/profiler/profiler-stats.h/.cc` | Tracks reasons for dropped/unattributed samples |
