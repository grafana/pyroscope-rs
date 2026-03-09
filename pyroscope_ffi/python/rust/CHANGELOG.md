# Changelog

## [2.0.0](https://github.com/grafana/pyroscope-rs/compare/python-1.0.4...python-2.0.0) (2026-03-09)


### ⚠ BREAKING CHANGES

* Memory profiling for Rust using jemalloc crate ([#378](https://github.com/grafana/pyroscope-rs/issues/378))
* replace reports Vec on ReportBatch with ReportData enum ([#447](https://github.com/grafana/pyroscope-rs/issues/447))

### Features

* add pysignalprof — signal-based CPU profiler for CPython 3.14 ([#417](https://github.com/grafana/pyroscope-rs/issues/417)) ([0976d99](https://github.com/grafana/pyroscope-rs/commit/0976d999e4a50c14459b8a0b39e72434be6d4bfc))
* add ReportBatch type for multi-profile support ([#437](https://github.com/grafana/pyroscope-rs/issues/437)) ([979a3b2](https://github.com/grafana/pyroscope-rs/commit/979a3b2805b14069d42aa50e874d815f3f865d19))
* Memory profiling for Rust using jemalloc crate ([#378](https://github.com/grafana/pyroscope-rs/issues/378)) ([3a11f1e](https://github.com/grafana/pyroscope-rs/commit/3a11f1e2e4a3fb2c832f1ef0c98a5942bea6b622))
* replace reports Vec on ReportBatch with ReportData enum ([#447](https://github.com/grafana/pyroscope-rs/issues/447)) ([a773660](https://github.com/grafana/pyroscope-rs/commit/a773660952b33accaa441ef08e8acdd42f2c37f7))
* use prebuilt OpenSSL 3.5.5 instead of vendored openssl-src crate ([#422](https://github.com/grafana/pyroscope-rs/issues/422)) ([7bf43b0](https://github.com/grafana/pyroscope-rs/commit/7bf43b0c6a824c378358c6cd152f29919fe65ce5))


### Bug Fixes

* **ci:** add version to pysignalprof dep to fix release-please crash ([#452](https://github.com/grafana/pyroscope-rs/issues/452)) ([fc519f6](https://github.com/grafana/pyroscope-rs/commit/fc519f6c816e1c79f54e492fa870e7ba6d0d122d))


### Miscellaneous Chores

* centralize all dependency versions in workspace root Cargo.toml ([#390](https://github.com/grafana/pyroscope-rs/issues/390)) ([0be2912](https://github.com/grafana/pyroscope-rs/commit/0be29127056facb39136baa3b58fd6b9f8318e55))


### Continuous Integration

* add clippy job to Rust CI to catch warnings ([#384](https://github.com/grafana/pyroscope-rs/issues/384)) ([26fee2e](https://github.com/grafana/pyroscope-rs/commit/26fee2e7a5bb1c03a1e07fc673f6e0d81b522522))
