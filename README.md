## Pyroscope Profiler

**Pyroscope Profiler for Rust. Profile your Rust applications.**

[![license](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE) 
[![Crate](https://img.shields.io/crates/v/pyroscope.svg)](https://crates.io/crates/pyroscope)


### Major Contributors

We'd like to give a big thank you to the following contributors who have made significant contributions to this project:

* [Abid Omar](https://github.com/omarabid)
* [Anatoly Korniltsev](https://github.com/korniltsev)
* [Bernhard Schuster](https://github.com/drahnr)


### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.

### Individual CPU observations

With `backend-pprof-rs`, Linux and macOS applications can preserve individual
samples for timeline consumers:

```rust
use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};

let backend = pprof_backend(
    PprofConfig::default()
        .retain_individual_samples()
        .sample_limit(10_000),
    BackendConfig::default(),
);
```

Aggregation remains the default. Individual samples have count one and carry
`StackTrace::observation` metadata. The pprof exporter preserves this metadata
as numeric sample labels `trace_timestamp_ns` (unit `nanoseconds`) and
`os_thread_id`. The timestamp uses `CLOCK_BOOTTIME` on Linux and `CLOCK_UPTIME_RAW`
on macOS; it is host/boot relative, not Unix time. Native OS thread IDs are
captured before unwinding and remain distinct from pthread handles used for
thread tag matching. These labels allow consumers to select observations by
interval; they do not add a timeline UI to the server.

The optional limit defaults to unlimited; zero disables collection. After the
limit is reached, the sampler skips further unwinding until reset. Each report
and each thread-tag change resets collection and the limit. The guard remains
active until dropped. Individual observations use more memory, so set a limit
for long collection windows. When constructing `PprofConfig` or `StackTrace`
with struct literals, supply the new fields (or use `..Default::default()`).

This feature adapts [Yuval Peress's (@yperess) upstream work](https://github.com/tikv/pprof-rs/pull/302).
The sampler changes adapt commit `5504133398b6a607ee602535656e2334f5e7b8c6`
in the existing `src/backend/pprofrs` implementation.
