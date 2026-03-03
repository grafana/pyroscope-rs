# Signal-Based In-Process CPU Profilers — Candidate Survey

Criteria: uses signal handlers for sampling, runs in-process, walks the stack in-process (no `process_vm_readv` or similar syscalls).

Excluded from this survey: Go runtime profiler and async-profiler (Java) — covered in parallel sessions.

---

## Summary Table

| # | Profiler | Language | Signal | Stack Walking Method |
|---|---------|----------|--------|---------------------|
| 1 | StackProf | Ruby | SIGPROF / SIGALRM | `rb_profile_frames()` via `rb_postponed_job` |
| 2 | Vernier | Ruby | SIGPROF (`pthread_kill`) | Pre-allocated StackTable in C sampling thread |
| 3 | Pf2 | Ruby | SIGPROF (`timer_create`) | `rb_profile_thread_frames()` |
| 4 | perftools.rb | Ruby (legacy) | SIGPROF (`setitimer`) | gperftools frame pointer walking |
| 5 | dd-trace-rb profiler | Ruby | SIGPROF | Ruby frame walking via CpuAndWallTimeWorker |
| 6 | vmprof | Python / PyPy | SIGPROF / SIGALRM | Python frame stack + PyPy JIT frames |
| 7 | statprof | Python | SIGPROF | `sys._current_frames()` stack crawl |
| 8 | Scalene | Python | SIGPROF / SIGALRM / SIGVTALRM | Python frame inspection |
| 9 | profiling.sampling (Tachyon) | Python 3.15+ | Signal-based (TBD) | CPython internal frame walking |
| 10 | gperftools (libprofiler) | C / C++ | SIGPROF (`setitimer`) | Frame pointer / libunwind |
| 11 | pprof-rs | Rust | SIGPROF (`setitimer`) | backtrace-rs |
| 12 | V8 CPU Profiler | Node.js | SIGPROF (`pthread_kill`) | V8 internal stack walk (JS + C++) |
| 13 | sample_prof | PHP | SIGPROF (`setitimer`) | PHP opline / op_array inspection |
| 14 | honest-profiler | Java | SIGPROF (`setitimer ITIMER_PROF`) | AsyncGetCallTrace (AGCT) |
| 15 | JFR CPU-Time (JEP 509) | Java (JDK 25+) | SIGPROF | Cooperative sampling + JFR stack walk |
| 16 | LuaJIT Profiler | Lua | SIGPROF (`setitimer`) | Fast stack dump + JIT exit-to-interpreter |
| 17 | GHC RTS Profiler | Haskell | SIGVTALRM / SIGALRM | Cost centre stack sampling |
| 18 | Datadog .NET Profiler | .NET (Linux) | Custom signal | libunwind `unw_backtrace2` |

---

## Ruby

### 1. StackProf
- **Repo:** https://github.com/tmm1/stackprof
- **Signal:** SIGPROF via `setitimer(ITIMER_PROF)` for CPU time; SIGALRM via `setitimer(ITIMER_REAL)` for wall time
- **How it works:** Registers a signal handler via `sigaction`. On signal delivery, the handler enqueues a sampling job via `rb_postponed_job_register_one` (avoids running unsafe code in signal context). The deferred job calls `rb_profile_frames()` to walk the Ruby call stack from C.

### 2. Vernier
- **Repo:** https://github.com/jhawthorn/vernier
- **Requires:** Ruby 3.2.1+
- **Signal:** SIGPROF sent via `pthread_kill` to target specific Ruby threads
- **How it works:** Uses a separate C sampling thread that sends signals to Ruby threads. The signal handler records samples into a pre-allocated StackTable (no Ruby object allocation in handler). Tracks GVL activity, GC pauses, and idle time. Improves on StackProf's async-signal-safety limitations.

### 3. Pf2
- **Repo:** https://github.com/osyoyu/pf2
- **Signal:** SIGPROF via `timer_create` with `CLOCK_THREAD_CPUTIME_ID`
- **How it works:** Sends SIGPROF to target pthreads and calls `rb_profile_thread_frames()` in the signal handler (or via postponed_job). Collected data stored in a pre-allocated ring buffer. Supports wall-clock, CPU, and per-thread CPU time modes. Default interval: 19ms.

### 4. perftools.rb (legacy)
- **Repo:** https://github.com/tmm1/perftools.rb
- **Signal:** SIGPROF via `setitimer`
- **How it works:** Wraps Google gperftools as a Ruby C extension. `setitimer` periodically sends SIGPROF, then gperftools signal handler snapshots the stack. Deprecated in favor of StackProf for Ruby >= 2.1.

### 5. Datadog dd-trace-rb CPU Profiler
- **Docs:** https://docs.datadoghq.com/profiler/profiler_troubleshooting/ruby/
- **Signal:** SIGPROF
- **How it works:** Sends SIGPROF to Ruby application threads for CPU/wall time sampling via `CpuAndWallTimeWorker`. Has a fallback "no signals" mode for environments where SIGPROF causes EINTR issues with native extensions.

---

## Python

### 6. vmprof
- **Repo:** https://github.com/vmprof/vmprof-python
- **Signal:** SIGPROF via `setitimer(ITIMER_PROF)` by default; SIGALRM via `ITIMER_REAL` with `real_time=True`
- **How it works:** C extension registers a signal handler that reads the Python frame stack on each signal delivery. On PyPy, also recognizes JIT-compiled frames from the C stack. Thread-safe. SIGPROF is delivered at max ~250Hz on most Linux; SIGALRM supports higher frequencies.

### 7. statprof
- **Repo:** https://github.com/bos/statprof.py
- **Signal:** SIGPROF via `signal.setitimer(signal.ITIMER_PROF)`
- **How it works:** Pure Python. Registers a Python-level signal handler for SIGPROF. On each signal, inspects `sys._current_frames()`, crawls up the stack, and increments per-code-object sample counts. Default: 1000 samples/second. Unix-only.

### 8. Scalene
- **Repo:** https://github.com/plasma-umass/scalene
- **Paper:** https://arxiv.org/pdf/2006.03879
- **Signal:** SIGPROF for memory allocation tracking; SIGALRM / SIGVTALRM for CPU timing via `setitimer`
- **How it works:** Uses signal handlers to periodically sample both CPU time and memory allocations. SIGPROF fires after a threshold of bytes copied; SIGALRM/SIGVTALRM fire on timer intervals. Python-level signal handler reads current frame info and attributes time/memory to specific lines. Uses a temporary file to avoid lost signals.

### 9. profiling.sampling (CPython 3.15+ "Tachyon")
- **Docs:** https://docs.python.org/3.15/library/profiling.sampling.html
- **PEP:** https://peps.python.org/pep-0799/
- **Signal:** Signal-based periodic sampling (likely SIGPROF/SIGALRM via setitimer)
- **How it works:** New built-in statistical profiler in CPython 3.15. Periodically captures snapshots of the call stack. Supports multi-threaded programs, free-threading, async code, and remote attachment. Tightly coupled with CPython internals.

---

## C / C++

### 10. gperftools CPU Profiler (libprofiler)
- **Docs:** https://gperftools.github.io/gperftools/cpuprofile.html
- **Signal:** SIGPROF via `setitimer(ITIMER_PROF)` at default 100Hz. Also supports a custom control signal via `CPUPROFILESIGNAL`.
- **How it works:** `ProfilerStart()` sets up an `ITIMER_PROF` timer. On each SIGPROF, the signal handler captures the call stack via frame pointer walking or libunwind and stores it in a pre-allocated hash map. `ProfilerStop()` writes accumulated data. No malloc in signal handler; uses only pre-allocated memory and spin locks instead of futexes.

---

## Rust

### 11. pprof-rs (TiKV)
- **Repo:** https://github.com/tikv/pprof-rs
- **Signal:** SIGPROF via `setitimer`
- **How it works:** `setitimer` sends SIGPROF at constant intervals. Signal handler captures a backtrace via `backtrace-rs` and increments counts in a pre-allocated fixed-size hashmap. Uses `try_lock` (not blocking lock) in the signal handler to avoid deadlock. Uses spin locks instead of futex. Restores original SIGPROF handler on stop.

---

## Node.js / V8

### 12. V8 CPU Profiler
- **Docs:** https://v8.dev/docs/profile
- **Signal:** SIGPROF via `pthread_kill` from a dedicated sampling thread
- **How it works:** A dedicated sampling thread calls `pthread_kill(target_thread, SIGPROF)` at regular intervals (default 1ms). The SIGPROF handler captures the execution context (both JS and C/C++ frames) from the interrupted thread. Node.js suppresses SIGPROF when the event loop is sleeping to reduce overhead.

---

## PHP

### 13. sample_prof (nikic)
- **Repo:** https://github.com/nikic/sample_prof
- **Signal:** SIGPROF via `setitimer`
- **How it works:** Registers a SIGPROF interrupt handler. On each signal, reads the currently executing `active_op_array` and `opline_ptr` to determine the current file and line number. Provides line-level profiling resolution. Accepts sampling interval in microseconds.

---

## Java / JVM

### 14. honest-profiler
- **Repo:** https://github.com/jvm-profiling-tools/honest-profiler
- **Signal:** SIGPROF via `setitimer(ITIMER_PROF)`
- **How it works:** Sets up `ITIMER_PROF` to deliver SIGPROF at a configurable frequency. Signal handler calls the undocumented `AsyncGetCallTrace` (AGCT) JVM internal API, which extracts instruction/frame/stack pointers from the `ucontext` provided by the signal handler. The interrupted thread is NOT at a safepoint. Results written to a lock-free MPSC ring buffer. No malloc or blocking IO in the handler.

### 15. JFR CPU-Time Profiling (JEP 509, JDK 25+)
- **JEP:** https://openjdk.org/jeps/509
- **Signal:** SIGPROF (Linux only, experimental)
- **How it works:** Uses Linux CPU-time signals to sample method execution. Unlike `jdk.ExecutionSample` (samples only at safepoints), `jdk.CPUTimeSample` captures methods executing in native code (e.g., FFM API calls). Leverages JEP 518 (Cooperative Sampling) for safe stack walking after signal delivery.

---

## Lua

### 16. LuaJIT Built-in Profiler
- **Docs:** https://luajit.org/ext_profiler.html
- **Signal:** SIGPROF via `setitimer` on Unix
- **How it works:** SIGPROF handler sets a flag and patches the interpreter dispatch table. Callback samples the stack with a fast stack dump helper. For JIT-compiled code, the JIT compiler adds checks at function/line granularity; when flag is set, JIT code exits to the interpreter, which then invokes the profiler callback. Accessible via `jit.profile`. Default interval: 10ms.

---

## Haskell

### 17. GHC RTS Time Profiler
- **Docs:** http://downloads.haskell.org/ghc/latest/docs/users_guide/profiling.html
- **Signal:** SIGVTALRM or SIGALRM (depending on RTS configuration)
- **How it works:** The RTS installs a timer signal handler that fires at the RTS clock frequency. On each tick, the handler records a time profiling sample of the currently executing cost centre stack. Profiling interval configurable via `-i` RTS flag.

---

## .NET

### 18. Datadog .NET Continuous Profiler (Linux)
- **Blog:** https://www.datadoghq.com/blog/engineering/dotnet-continuous-profiler/
- **Signal:** Custom signal (not SIGPROF)
- **How it works:** `StackSamplerLoop` interrupts managed threads by sending a signal. Signal handler walks the stack using libunwind's `unw_backtrace2` to fill an array of instruction pointers. `ProfilerSignalManager` handles signal handler installation and chaining. Symbolization (mapping IPs to method names via `ICorProfilerInfo::GetFunctionFromIP`) occurs outside the signal handler.

---

## Excluded — Do NOT Meet Criteria

| Profiler | Language | Reason |
|----------|----------|--------|
| rbspy | Ruby | Out-of-process; uses `process_vm_readv` |
| py-spy | Python | Out-of-process; uses `process_vm_readv` / `ReadProcessMemory` |
| austin | Python | Out-of-process; uses `process_vm_readv` / `vm_read` |
| phpspy | PHP | Out-of-process; uses `process_vm_readv` |
| yappi | Python | Deterministic profiler (hooks via `PyEval_SetProfile`); not signal-based |
| Devel::NYTProf | Perl | Deterministic profiler (statement-level instrumentation) |
| Erlang eprof/fprof | Erlang/BEAM | Uses tracing BIFs, not signal-based |
