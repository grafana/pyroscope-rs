## Pyroscope Profiler

**Pyroscope Profiler for Rust. Profile your Rust applications.**

[![license](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE) 
![tests](https://github.com/pyroscope-io/pyroscope-rs/workflows/Tests/badge.svg)
![build](https://github.com/pyroscope-io/pyroscope-rs/workflows/Build/badge.svg)
[![Crate](https://img.shields.io/crates/v/pyroscope.svg)](https://crates.io/crates/pyroscope)

---

You may be looking for:

- [An overview of Pyroscope](https://pyroscope.io/docs/)
- [Crate Documentation](https://docs.rs/pyroscope/)
- [Examples](examples)
- [Release notes](https://github.com/omarabid/pyroscope/releases)
- [Pyroscope CLI](pyroscope_cli)

## Table of Contents
- [Quick Start](#quick-start)
- [Pyroscope Server](#pyroscope-server)
- [Multi-Threading](#multi-threading)
- [Profiling Backends](#profiling-backends)
- [Limitations](#limitations)
- [Getting Help](#getting-help)
- [License](#license)

### Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pyroscope = { version = "0.5", features = ["backend-pprof-rs"] }
```

Include Pyroscope and pprof-rs dependencies:

```rust
use pyroscope::PyroscopeAgent;
use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
```

Configure the Pyroscope agent:

```rust
 let agent =
     PyroscopeAgent::builder("http://localhost:4040", "myapp-profile")
     .backend(pprof_backend(PprofConfig { sample_rate: 100 }, BackendConfig::default()))
     .build()?;
```

Profile your code:

```rust
 let agent_running = agent.start()?;

 // Computation to profile 

 let agent_ready = agent_running.stop()?;
 agent_ready.shutdown();
```

### Pyroscope Server

The Pyroscope Agent sends the profiling data to a [Pyroscope Server](https://pyroscope.io/docs/installing-pyroscope-overview/). You need to have a Pyroscope Server running in order to consume and visualize this data. It's not possible, currently, to forward the data to another endpoint.

### Multi-Threading

The Pyroscope Agent and the pprof-rs backend (built into `pyroscope` via the `backend-pprof-rs` feature) can profile and report data from a multi-threaded program. [pprof-rs](https://github.com/tikv/pprof-rs), however, does not track child-processes, and thus profiling is limited to a single process.

### Profiling Backends

The Pyroscope Agent doesn't do any profiling. The agent role is to orchestrate a profiling backend, and report the profiling data to the Pyroscope Server. The Agent can support external backends (in fact, all current backends are independent crates), and you can make your own. Backends can also be used separately. The currently available backends are:

- pprof-rs: Rust profiler backend included in this crate behind the `backend-pprof-rs` feature flag. Powered by [pprof-rs](https://github.com/tikv/pprof-rs).

### Native Integration

Pyroscope can be used directly in your projects with native integration. No agents or external programs are required.

- [Python](https://pypi.org/project/pyroscope-io/): Python Package. [Readme](https://github.com/pyroscope-io/pyroscope-rs/tree/main/pyroscope_ffi/python#readme) - [Documentation](https://pyroscope.io/docs/python/)
- [Ruby](https://rubygems.org/gems/pyroscope): Ruby Gem. [Readme](https://github.com/pyroscope-io/pyroscope-rs/tree/main/pyroscope_ffi/ruby#readme) - [Documentation](https://pyroscope.io/docs/ruby/)


### Limitations

- **Backend**: The Pyroscope Agent uses [pprof-rs](https://github.com/tikv/pprof-rs) as a backend. As a result, the [limitations](https://github.com/tikv/pprof-rs#why-not-) for pprof-rs also applies.
- **Tagging**: As of 0.5.0, the Pyroscope Agent support tagging within threads.
- **Timer**: epoll (for Linux) and kqueue (for macOS) are required for a more precise timer.
- **Shutdown**: The Pyroscope Agent might take some time (usually less than 10 seconds) to shutdown properly and drop its threads. For a proper shutdown, it's recommended that you run the `shutdown` function before dropping the Agent.

### Getting help

You can read the [Docs](https://docs.rs/pyroscope/) for detailed usage of the library. You can also join the [Slack channel](https://pyroscope.slack.com/archives/C02Q47F8LJH) if you have questions.

### Major Contributors

We'd like to give a big thank you to the following contributors who have made significant contributions to this project:

* [Abid Omar](https://github.com/omarabid)
* [Anatoly Korniltsev](https://github.com/korniltsev)
* [Bernhard Schuster](https://github.com/drahnr)


### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
