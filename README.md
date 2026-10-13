## Pyroscope Profiler

**Pyroscope Profiler for Rust. Profile your Rust applications.**

[![license](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE) 
[![Crate](https://img.shields.io/crates/v/pyroscope.svg)](https://crates.io/crates/pyroscope)

### Mimalloc Memory Profiling

Enable the optional `backend-mimalloc` feature and install
`SamplingMiMalloc` as the process global allocator:

```toml
[dependencies]
# Before the next crates.io release, use the branch or a local path that
# contains `backend-mimalloc`.
pyroscope = { git = "https://github.com/grafana/pyroscope-rs", features = ["backend-mimalloc"] }
```

```rust
use pyroscope::backend::mimalloc::{
    mimalloc_backend, MimallocConfig, SamplingMiMalloc,
};
use pyroscope::pyroscope::PyroscopeAgentBuilder;

#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = PyroscopeAgentBuilder::new(
        "http://localhost:4040",
        "my-rust-service",
        100,
        "pyroscope-rs",
        env!("CARGO_PKG_VERSION"),
        mimalloc_backend(MimallocConfig::default()),
    )
    .build()?;

    let agent_running = agent.start()?;
    // Run application workload.
    let agent_ready = agent_running.stop()?;
    agent_ready.shutdown();
    Ok(())
}
```

The default mimalloc backend records allocation events (`alloc_objects` and
`alloc_space`) through the normal Pyroscope upload path. Enable live heap
profiling to also report `inuse_objects` and `inuse_space`, with `inuse_space`
selected by default, matching the jemalloc backend's live memory view:

```rust
let backend = mimalloc_backend(MimallocConfig {
    live_heap_tracking: true,
    max_live_samples: 16_384,
    ..MimallocConfig::default()
});
```

Live tracking covers Rust allocations made through `SamplingMiMalloc` after
initialization, including cross-thread frees and reallocations. It reports
sampling-weighted requested sizes, not allocator usable sizes or allocations
made directly by native libraries. Sampling starts on `alloc` / `alloc_zeroed`.
Same-address reallocations preserve the original stack and sampling weight
while updating size. Moving reallocations independently sample the replacement
allocation, including replacements of previously untracked objects. Enabling live tracking
does not change the interval `alloc_*` accounting for reallocations.

Metadata is bounded, preallocated, and partitioned across shards. Full or
contended shards can omit new live samples, exposed by
`mimalloc_stats().dropped_live_samples`. Freeing a tracked pointer reliably
removes it, which may briefly wait for its shard lock. Reports copy shards one
at a time, so concurrent heap snapshots are approximate rather than atomic.

The feature uses `rustfs-mimalloc` and requires Rust 1.96 or newer. Applications
must install `SamplingMiMalloc`; initialization fails when it has not observed
allocations. Using `rustfs_mimalloc::MiMalloc` directly does not capture call
stacks. Only one mimalloc backend may be active per process. Unresolved frames
fall back to instruction addresses or a synthetic frame. Thread tags are
no-ops, as in the jemalloc backend; agent-wide tags remain supported.

Memory pprof locations retain instruction addresses and inline call chains.
Function filenames and source lines are included when debug information is
available. Rust display names omit hash/disambiguator decorations, while
`Function.system_name` preserves the original symbol for re-demangling. Profiler
frames are filtered once per resolved address and shared across stacks.
Unresolved addresses keep an empty location line table, allowing downstream
symbolizers to fill them without mistaking an address string for a function.
Stripped Apple builds also leave Mach-O image-header fallback symbols unresolved
instead of presenting them as functions; physical addresses and mappings remain.
To retain source information in optimized builds:

```toml
[profile.release]
debug = 1
strip = "none"
```

On Linux glibc, `MimallocConfig::stack_capture` can select
`MimallocStackCapture::Native`. It collects raw return addresses through the
MT-Safe libc interface without taking backtrace-rs's shared lock, and warms the
unwinder before recording starts. Symbolization remains in `report()`. The
portable engine stays the default: native capture is not universally faster,
and initialization rejects it on musl, Apple, Windows, and other targets.
CI checks optimized stripped and source-enabled profiles on Linux and macOS,
including physical call chains, mapping IDs, live retention, and cross-thread
release. The source-enabled variant also requires user filenames and lines.

```bash
cargo test --release --locked --no-default-features --features backend-mimalloc --test mimalloc_release
CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=none MIMALLOC_TEST_EXPECT_SOURCE=1 \
  cargo test --release --locked --no-default-features --features backend-mimalloc --test mimalloc_release
```

On Linux and Apple platforms, reports include executable mappings, file offsets,
and available build IDs (GNU build IDs or Mach-O UUIDs). Linux offsets are read
only from ELF files whose build ID matches the loaded image; unavailable or
unverifiable mappings remain unset. Other platforms retain addresses and local
symbols without mappings. Image metadata is collected at report time, not in
allocation hooks. Applications that dynamically unload code must keep sampled
images loaded until reporting; Apple image enumeration also requires that
applications avoid concurrent library loading or unloading during reports.
The upload path is covered by loopback HTTP integration tests
for memory profiles, gzip payloads, authentication, tenant headers, and labels;
these tests do not replace validation against a deployed Pyroscope server.

Mimalloc interval samples carry `__delta__="false"` as a pprof sample label.
Pyroscope promotes it to the series so `alloc_*` values are not differenced a
second time as cumulative counters. Live values remain heap snapshots. Do not
override `__delta__` in agent tags; it is a transport control, not a business tag.

For a real server round trip, Docker-based checks pin Pyroscope 2.4.0 by digest
and pair the selected storage write/read paths explicitly:

```bash
make mimalloc/test/server
PYROSCOPE_TEST_WRITE_PATH=segment-writer \
  PYROSCOPE_TEST_ARTIFACT_DIR=target/mimalloc-server-v2-e2e \
  bash scripts/test_mimalloc_server.sh
```

The checks cover all four memory sample types, first and decreasing allocation
intervals, application flamegraph frames, queried pprof source metadata, and
cross-thread release with an ingested positive control. Uploaded/query profiles,
flamegraph JSON, logs, and revision provenance are saved as CI artifacts. The
container uses temporary storage, binds only loopback, disables telemetry and
self-profiling, and is removed on exit. These are protocol/data checks, not a
Grafana UI interaction or external symbol-service deployment test.

For offline inspection, the overhead example can export a gzipped memory profile
after shutdown, outside the timed workload:

```bash
MIMALLOC_BENCH_MODE=live MIMALLOC_BENCH_PPROF_PATH=target/mimalloc.pprof.gz \
  cargo run --example mimalloc_overhead --features backend-mimalloc
go tool pprof -sample_index=inuse_space -top target/mimalloc.pprof.gz
```

Useful local checks:

```bash
cargo run --example mimalloc --features backend-mimalloc
cargo run --release --example mimalloc_overhead --features backend-mimalloc
MIMALLOC_BENCH_MODE=active MIMALLOC_BENCH_REPORT_INTERVAL_MS=50 \
  MIMALLOC_BENCH_RING_CAPACITY=16384 \
  cargo run --release --example mimalloc_overhead --features backend-mimalloc
MIMALLOC_BENCH_MODE=live cargo run --release --example mimalloc_overhead --features backend-mimalloc
make mimalloc/bench/report
cargo test --locked --test mimalloc_backend --features backend-mimalloc -- --ignored
```

`make mimalloc/bench/report` writes a Markdown report and raw key-value
outputs under `target/mimalloc-benchmark/`. The GitHub Actions
`mimalloc benchmark report` job uploads the same directory as the
`mimalloc-benchmark-report` artifact, including throughput, overhead,
recorder counters, report latency, encoded pprof size, pprof encode time, and
sampled allocation latency percentiles. Live heap scenarios at 1 MiB, 512 KiB,
and 4 KiB sampling intervals also retain a working set through report creation.
Their raw outputs include live sample counts, dropped live samples, and
preallocated metadata payload bytes (excluding hash control bytes, shard
headers, and allocator bookkeeping). Post-workload collection rows are pressure
diagnostics: low overhead while dropping records is not lossless sampling cost.
`steady-active-1m` and `steady-live-1m` drain a worker's TLS rings concurrently
every 50 ms using a 16,384-record queue. Reports record completed report counts,
drained allocation records, pending records, drop rate, and quality status.
`MimallocStats::reported_samples` counts raw allocation records encoded before
aggregation, not weighted bytes or live entries repeated across snapshots.
`MIMALLOC_BENCH_ENFORCE_QUALITY=1` rejects steady rows with excessive drops,
pending records, live drops, or no periodic report. The default record-drop
budget is 1%, adjustable through `MIMALLOC_BENCH_MAX_DROP_PCT`.
Benchmark warnings are diagnostic; performance claims also require stable,
resource-isolated repeat runs. Final draining is excluded from workload timing,
and a requested profile export contains the final report, not merged intervals.

Latency sampling selects one pseudorandom allocation per window, covering the
workload's size distribution instead of repeatedly timing the smallest size.
Raw outputs include the sampling policy, sample count, and sampled size range.
New history rows use `history/mimalloc-benchmark-history-v4.csv`; existing
history files are retained. Earlier fixed-cadence latency percentiles should
not be compared directly with the new stratified measurements.

For offline report reclassification, set `MIMALLOC_BENCH_INPUT_DIR` to a saved
ten-scenario raw-output directory. Replay does not run workloads or establish
their source revision; its history rows use `replay-unknown`.

For same-process concurrent capture comparisons, the overhead example supports
`MIMALLOC_BENCH_WORKERS=2` (up to 64) with a positive report interval and
`MIMALLOC_BENCH_STACK_CAPTURE=portable` or `native`. Workers start together;
throughput totals all workers over the common wall-clock interval, while latency
percentiles describe only the first worker. The standard baseline report stays
single-worker, and v4 history records the capture engine and worker count.

### Major Contributors

We'd like to give a big thank you to the following contributors who have made significant contributions to this project:

* [Abid Omar](https://github.com/omarabid)
* [Anatoly Korniltsev](https://github.com/korniltsev)
* [Bernhard Schuster](https://github.com/drahnr)


### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
