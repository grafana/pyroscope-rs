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
pyroscope = "0.5.0"
pyroscope-pprofrs = "0.2"
```

Include Pyroscope and pprof-rs dependencies:

```rust
use pyroscope::PyroscopeAgent;
use pyroscope_pprofrs::{pprof_backend, PprofConfig};
```

Configure the Pyroscope agent:

```rust
 let agent =
     PyroscopeAgent::builder("http://localhost:4040", "myapp-profile")
     .backend(pprof_backend(PprofConfig::new().sample_rate(100)))
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

The Pyroscope Agent send the profiling data to a [Pyroscope Server](https://pyroscope.io/docs/installing-pyroscope-overview/). You need to have a Pyroscope Server running in order to consume and visualize this data. It's not possible, currently, to forward the data to another endpoint.

### Multi-Threading

The Pyroscope Agent and the [pprof-rs backend](pyroscope_backends/pyroscope_pprofrs) can profile and report data from a multi-threaded program. [pprof-rs](https://github.com/tikv/pprof-rs), however, does not track child-processes and thus profiling is limited to a single process.

### Profiling Backends

The Pyroscope Agent doesn't do any profiling. The agent role is to orchasrate a profiling backend, and report the profiling data to the Pyroscope Server. The Agent can support external backends (in fact, all current backends are independent crates) and you can make your own. Backends can also be used seperately. The currently available backends are:

- [pprof-rs](pyroscope_backends/pyroscope_pprofrs): Rust profiler. A wrapper around [pprof-rs](https://github.com/tikv/pprof-rs).
- [rbspy](pyroscope_backends/pyroscope_rbspy): Ruby Profiler. A wrapper around [rbspy](https://rbspy.github.io/).
- [py-spy](pyroscope_backends/pyroscope_pyspy): Python Profiler. A wrapper around [py-spy](https://github.com/benfred/py-spy).


### Limitations

- **Backend**: The Pyroscope Agent uses [pprof-rs](https://github.com/tikv/pprof-rs) as a backend. As a result, the [limitations](https://github.com/tikv/pprof-rs#why-not-) for pprof-rs also applies.
- **Tagging**: ~~Adding or removing tags is not possible within threads. In general, the [Pyroscope Agent](https://docs.rs/pyroscope/latest/pyroscope/pyroscope/struct.PyroscopeAgent.html) is not Sync; and as a result a reference cannot be shared between threads. A multi-threaded program could be profiled but the agent is not thread-aware and a particular thread cannot be tagged.~~
As of 0.5.0, the Pyroscope Agent support tagging within threads. Check the [Tags](examples/tags.rs) and [Multi-Thread](examples/multi-thread.rs) examples for usage.
- **Timer**: epoll (for Linux) and kqueue (for macOS) are required for a more precise timer.
- **Shutdown**: The Pyroscope Agent might take some time (usually less than 10 seconds) to shutdown properly and drop its threads. For a proper shutdown, it's recommended that you run the `shutdown` function before dropping the Agent.

### Getting help

You can read the [Docs](https://docs.rs/pyroscope/) or check the [examples](examples) for detailed usage of the library. You can also join the [Slack channel](https://pyroscope.slack.com/archives/C02Q47F8LJH) if you have questions.

### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
