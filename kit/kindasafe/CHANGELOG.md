# Changelog

## [0.2.0](https://github.com/grafana/pyroscope-rs/compare/kindasafe-0.1.0...kindasafe-0.2.0) (2026-03-09)


### Features

* add kindasafe crate for safe memory reading ([#394](https://github.com/grafana/pyroscope-rs/issues/394)) ([950d2d9](https://github.com/grafana/pyroscope-rs/commit/950d2d908a4fda525c4ba0dfc50119cd22835cbe))
* **kindasafe:** add aarch64 support for safe memory reading ([#421](https://github.com/grafana/pyroscope-rs/issues/421)) ([6573284](https://github.com/grafana/pyroscope-rs/commit/6573284e7a81aa58c340a6e13541512cf8ff0080))
* **kindasafe:** add macOS support for x86_64 and aarch64 ([#423](https://github.com/grafana/pyroscope-rs/issues/423)) ([b5b4cb4](https://github.com/grafana/pyroscope-rs/commit/b5b4cb4dd26b63bebccf38e815694e37ac559e0f))
* **kindasafe:** prepare kindasafe crates for crates.io publishing with trusted publishing ([#434](https://github.com/grafana/pyroscope-rs/issues/434)) ([17bd1b9](https://github.com/grafana/pyroscope-rs/commit/17bd1b98d1b242d64a10d8f71ba4a24e4152b418))
* split kindasafe into no_std read crate and kindasafe_init ([#420](https://github.com/grafana/pyroscope-rs/issues/420)) ([5975b08](https://github.com/grafana/pyroscope-rs/commit/5975b084ce2025887d5d9d79626998b50a09cad3))


### Bug Fixes

* **kindasafe:** use runtime page size in page boundary tests ([#425](https://github.com/grafana/pyroscope-rs/issues/425)) ([5dd0dac](https://github.com/grafana/pyroscope-rs/commit/5dd0dac6e6c5d6d20cea0cfb1c578ef85de93ca8))


### Miscellaneous Chores

* **deps:** pin dependencies ([#396](https://github.com/grafana/pyroscope-rs/issues/396)) ([90d21fa](https://github.com/grafana/pyroscope-rs/commit/90d21fa44dda218c70c94034b126f90a1b1a129f))
* **deps:** update rust crate anyhow to v1.0.102 ([#397](https://github.com/grafana/pyroscope-rs/issues/397)) ([8634a73](https://github.com/grafana/pyroscope-rs/commit/8634a7310acec5ea9aad9e0ab40ed40366809127))
