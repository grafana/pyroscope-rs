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

## Table of Contents
- [Quick Start](#quick-start)
- [Limitations](#limitations)
- [Getting Help](#getting-help)
- [License](#license)

### Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pyroscope = "0.4.0"
pyroscope-pprofrs = "0.1"
```

Configure and create the backend (pprof-rs)

```rust
let pprof_config = PprofConfig::new().sample_rate(100);
let pprof_backend = Pprof::new(pprof_config);
```

Configure the Pyroscope agent:

```rust
 let mut agent =
     PyroscopeAgent::builder("http://localhost:4040", "myapp-profile")
     .backend(pprof_backend)
     .build()?;
```

Profile your code:

```rust
 agent.start();
 // Profiled computation
 agent.stop();
 
 // Non-profiled computation
```

### Limitations

- **Backend**: The Pyroscope Agent uses [pprof-rs](https://github.com/tikv/pprof-rs) as a backend. As a result, the [limitations](https://github.com/tikv/pprof-rs#why-not-) for pprof-rs also applies.
- **Tagging**: Adding or removing tags is not possible within threads. In general, the [Pyroscope Agent](https://docs.rs/pyroscope/latest/pyroscope/pyroscope/struct.PyroscopeAgent.html) is not Sync; and as a result a reference cannot be shared between threads. A multi-threaded program could be profiled but the agent is not thread-aware and a particular thread cannot be tagged.
- **Timer**: epoll (for Linux) and kqueue (for macOS) are required for a more precise timer.
- **Shutdown**: The Pyroscope Agent might take some time (usually less than 10 seconds) to shutdown properly and drop its threads.

### Getting help

You can read the [Docs](https://docs.rs/pyroscope/) or check the [examples](examples) for detailed usage of the library. You can also join the [Slack channel](https://pyroscope.slack.com/archives/C02Q47F8LJH) if you have questions.

### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
