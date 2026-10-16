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

The backend delegates sampling, live tracking, symbolization and pprof
generation to the standalone
[`rustfs-mimalloc-profiling`](https://github.com/houseme/rustfs-mimalloc/tree/63b53f28e70fe596634187dcd25d508708a0699f/rustfs-mimalloc-profiling)
crate. Its allocator, config and diagnostic types are re-exported through
`pyroscope::backend::mimalloc`. The backend requires Rust 1.96 or newer.

The process must install `SamplingMiMalloc` before starting the agent. Only
Rust allocations through this wrapper after initialization are observed;
direct native allocations and process RSS are outside its coverage. Sizes are
sampling-weighted requested bytes. Only one standalone profiler or Pyroscope
mimalloc backend can own the recorder at a time. Thread tags are no-ops, as
with jemalloc; agent-wide labels are supported by the existing upload pipeline.

Unresolved physical addresses remain available for downstream symbolization.
Linux and Apple profiles include executable mappings and build IDs; filenames
and source lines require debug information. Sampled code must remain loaded
until reporting. Applications on Apple platforms must avoid concurrent library
loading or unloading during reports. Portable capture is the default;
`MimallocStackCapture::Native` is available on Linux GNU only.

Allocation reports contain interval counters and the `__delta__="false"`
pprof sample label, preventing cumulative subtraction by Pyroscope. Live heap
values remain snapshots. Keep `__delta__` reserved for this transport behavior
when configuring agent tags. The standalone crate owns allocator correctness,
optimized profile quality, stress tests and benchmark diagnostics. See its
README for limits, sampling controls and overhead measurement instructions.

This repository tests the adapter, raw HTTP profile preservation, authentication,
tenant and agent labels, and real Pyroscope ingestion/query behavior:

```sh
cargo test --locked --features backend-mimalloc
make mimalloc/test/server
PYROSCOPE_TEST_WRITE_PATH=segment-writer \
  PYROSCOPE_TEST_ARTIFACT_DIR=target/mimalloc-server-v2-e2e \
  bash scripts/test_mimalloc_server.sh
```

The server harness uses a digest-pinned Pyroscope 2.4.0 image and verifies first
and decreasing allocation intervals, retained live values, cross-thread frees,
and source metadata through ingestion and query. These checks do not establish
Grafana UI behavior, external symbol-service deployment or production overhead.

### Major Contributors

We'd like to give a big thank you to the following contributors who have made significant contributions to this project:

* [Abid Omar](https://github.com/omarabid)
* [Anatoly Korniltsev](https://github.com/korniltsev)
* [Bernhard Schuster](https://github.com/drahnr)


### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
