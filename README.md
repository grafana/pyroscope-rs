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

The mimalloc backend records allocation samples and emits memory pprof data
through the normal Pyroscope upload path. It is an allocation profile, not a
live heap/in-use profile, and it requires `SamplingMiMalloc`; using
`mimalloc::MiMalloc` directly will not capture allocation call stacks. Samples
with unresolved frames may be grouped under a synthetic fallback frame.

Useful local checks:

```bash
cargo run --example mimalloc --features backend-mimalloc
cargo run --release --example mimalloc_overhead --features backend-mimalloc
make mimalloc/bench/report
cargo test --locked --test mimalloc_backend --features backend-mimalloc -- --ignored
```

`make mimalloc/bench/report` writes a Markdown report and raw key-value
outputs under `target/mimalloc-benchmark/`. The GitHub Actions
`mimalloc benchmark report` job uploads the same directory as the
`mimalloc-benchmark-report` artifact, including throughput, overhead,
recorder counters, report latency, encoded pprof size, pprof encode time, and
sampled allocation latency percentiles.

### Major Contributors

We'd like to give a big thank you to the following contributors who have made significant contributions to this project:

* [Abid Omar](https://github.com/omarabid)
* [Anatoly Korniltsev](https://github.com/korniltsev)
* [Bernhard Schuster](https://github.com/drahnr)


### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
