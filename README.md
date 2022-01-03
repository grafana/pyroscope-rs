## Pyroscope Profiler

**Pyroscope Profiler for Rust. Profile your Rust applications.**

[![license](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE) 
![tests](https://github.com/omarabid/pyroscope/workflows/Tests/badge.svg)
![build](https://github.com/omarabid/pyroscope/workflows/Build/badge.svg)
[![Crate](https://img.shields.io/crates/v/pyroscope.svg)](https://crates.io/crates/pyroscope)

---

You may be looking for:

- [An overview of Pyroscope](https://pyroscope.io/docs/)
- [Crate Documentation](https://docs.rs/pyroscope/)
- [Examples](examples)
- [Release notes](https://github.com/omarabid/pyroscope/releases)

### Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pyroscope = "0.2.0-beta"
```

Configure your profiler:

```rust
 let mut agent =
     PyroscopeAgent::builder("http://localhost:4040", "myapp-profile")
     .sample_rate(100)
     .build()?;
```

Profile your code:

```rust

 agent.start()?;
 // Profiled computation
 agent.stop()?;
 
 // Non-profiled computation
```

### Getting help

You can read the [Docs](https://docs.rs/pyroscope/) or check the [examples](examples) for detailed usage of the library. You can also join the [Slack channel](https://pyroscope.slack.com/archives/C02Q47F8LJH) if you have questions.

### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
