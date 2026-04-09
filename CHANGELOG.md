# Changelog

## [2.0.1](https://github.com/grafana/pyroscope-rs/compare/lib-2.0.0...lib-2.0.1) (2026-04-09)


### Bug Fixes

* revert 0976d999e4a50c14459b8a0b39e72434be6d4bfc     ([#483](https://github.com/grafana/pyroscope-rs/issues/483)) ([ad3d032](https://github.com/grafana/pyroscope-rs/commit/ad3d03217a14a18868b8ba21f3dbf4ed844d759a))


### Miscellaneous Chores

* **deps:** lock file maintenance ([#467](https://github.com/grafana/pyroscope-rs/issues/467)) ([09411da](https://github.com/grafana/pyroscope-rs/commit/09411daca91120c4863983273863c55b02b4dd89))
* **deps:** lock file maintenance ([#478](https://github.com/grafana/pyroscope-rs/issues/478)) ([447d8ee](https://github.com/grafana/pyroscope-rs/commit/447d8eef3621be5b9b09b0b9eca3968bca081c2a))
* **deps:** lock file maintenance ([#498](https://github.com/grafana/pyroscope-rs/issues/498)) ([4072a04](https://github.com/grafana/pyroscope-rs/commit/4072a04626abd98aa18bc6cda37fb0ccc0803f59))
* **deps:** pin dependencies ([#427](https://github.com/grafana/pyroscope-rs/issues/427)) ([a46830a](https://github.com/grafana/pyroscope-rs/commit/a46830ac9a75a7634d9cc1ecc514a21b5c0f0603))
* **deps:** pin rust docker tag to e8e2bb5 ([#501](https://github.com/grafana/pyroscope-rs/issues/501)) ([c177d5b](https://github.com/grafana/pyroscope-rs/commit/c177d5b8f28041d3956851d93e61be3aebc6d975))
* **deps:** update dtolnay/rust-toolchain digest to 3c5f7ea ([#480](https://github.com/grafana/pyroscope-rs/issues/480)) ([c4f2e7d](https://github.com/grafana/pyroscope-rs/commit/c4f2e7dd1b1234f8870ce0f858f92542aef37c2e))
* **deps:** update dtolnay/rust-toolchain digest to 3c5f7ea ([#500](https://github.com/grafana/pyroscope-rs/issues/500)) ([fc21bda](https://github.com/grafana/pyroscope-rs/commit/fc21bda959bdf5d5d5a44f559a2b9de8072d2fba))
* **deps:** update rust crate env_logger to v0.11.10 ([#490](https://github.com/grafana/pyroscope-rs/issues/490)) ([24fb12d](https://github.com/grafana/pyroscope-rs/commit/24fb12d4c26c9cc167585f3fd93172015864a0e4))
* **deps:** update rust crate libc to v0.2.183 ([#489](https://github.com/grafana/pyroscope-rs/issues/489)) ([b44bac8](https://github.com/grafana/pyroscope-rs/commit/b44bac8f4274882d793e9e9f92ab0ad9fc4692f0))
* **deps:** update rust crate rbspy to 0.43 ([#471](https://github.com/grafana/pyroscope-rs/issues/471)) ([1e524d3](https://github.com/grafana/pyroscope-rs/commit/1e524d38d84dfedc799515cb70ee434ef91a28e6))
* **deps:** update rust crate rbspy to 0.44 ([#477](https://github.com/grafana/pyroscope-rs/issues/477)) ([c7cfe39](https://github.com/grafana/pyroscope-rs/commit/c7cfe39fecd8a0624df5c5d22b709e73747ddb4d))
* **deps:** update rust crate uuid to v1.23.0 ([#481](https://github.com/grafana/pyroscope-rs/issues/481)) ([9c4b53a](https://github.com/grafana/pyroscope-rs/commit/9c4b53aef55b13dc6da99396119e33ec9dc02aa0))
* **deps:** update rust-lang/crates-io-auth-action action to v1.0.4 ([#479](https://github.com/grafana/pyroscope-rs/issues/479)) ([f4e08f3](https://github.com/grafana/pyroscope-rs/commit/f4e08f34aa2b14a1e417c005d37f75981940d675))
* **deps:** update softprops/action-gh-release action to v2.5.2 ([#473](https://github.com/grafana/pyroscope-rs/issues/473)) ([1700be3](https://github.com/grafana/pyroscope-rs/commit/1700be3e34e7351a2f5f030187a2fac1ce1082e9))
* **deps:** update softprops/action-gh-release action to v2.5.3 ([#474](https://github.com/grafana/pyroscope-rs/issues/474)) ([12e34c1](https://github.com/grafana/pyroscope-rs/commit/12e34c109bd0e408dc0401430f4da4b5245a09da))
* **deps:** update softprops/action-gh-release action to v2.6.1 ([#475](https://github.com/grafana/pyroscope-rs/issues/475)) ([209c4c0](https://github.com/grafana/pyroscope-rs/commit/209c4c03ce08fe411046316dc333654fb64ab75b))
* remove FFI code and kindasafe (moved to separate repos) ([#488](https://github.com/grafana/pyroscope-rs/issues/488)) ([35a4c6a](https://github.com/grafana/pyroscope-rs/commit/35a4c6aa0808f0392e39e752d95a91536068ab74))


### Documentation

* fix broken Rust documentation and add doc-test CI ([#484](https://github.com/grafana/pyroscope-rs/issues/484)) ([98ae8a5](https://github.com/grafana/pyroscope-rs/commit/98ae8a595f18946f2edf27544ea6e2bb29724cbf))


### Continuous Integration

* add musl/Alpine to Rust test matrix ([#495](https://github.com/grafana/pyroscope-rs/issues/495)) ([a4b7ad4](https://github.com/grafana/pyroscope-rs/commit/a4b7ad4c9e530576adbbf278b104d65c970d43b0))
* add release-please configuration ([#491](https://github.com/grafana/pyroscope-rs/issues/491)) ([fbb2d8e](https://github.com/grafana/pyroscope-rs/commit/fbb2d8edc7cf400343650e77580464d32369d93d))

## v2.0.0

## Breaking Changes
- `Backend::report()` now returns `ReportBatch` instead of `Vec<Report>` ([#437](https://github.com/grafana/pyroscope-rs/pull/437), [#447](https://github.com/grafana/pyroscope-rs/pull/447))
- New `ReportData` enum replaces the plain `Vec<Report>` in `ReportBatch`, supporting both structured reports (`ReportData::Reports`) and pre-encoded pprof bytes (`ReportData::RawPprof`) ([#447](https://github.com/grafana/pyroscope-rs/pull/447))
- Minimum supported Rust version bumped from 1.64 to 1.66

## New Features
- **Jemalloc memory profiling backend** — new `backend-jemalloc` feature flag enables heap profiling via `jemalloc_pprof`. Use `pyroscope::backend::jemalloc::jemalloc_backend()` to get started ([#378](https://github.com/grafana/pyroscope-rs/pull/378))
- **`ReportBatch` type** — backends now return a `ReportBatch` with a `profile_type` field (e.g. `"process_cpu"`, `"memory"`), enabling multi-profile support ([#437](https://github.com/grafana/pyroscope-rs/pull/437))

## Dependencies
- Updated `pprof` (pprof-pyroscope-fork) to v0.1500.3 ([#407](https://github.com/grafana/pyroscope-rs/pull/407))
- Updated `object` crate to 0.38 ([#430](https://github.com/grafana/pyroscope-rs/pull/430))
- Disabled py-spy default features to exclude CLI dependencies ([#418](https://github.com/grafana/pyroscope-rs/pull/418))
- Added `jemalloc_pprof` 0.8 and `tokio` 1 as workspace dependencies

## v1.0.0
## Breaking Changes
- Removed `auth_token` from Python and Ruby FFI bindings and related code
- Removed `detect_subprocesses` from Python and Ruby configs
- Config constructor now requires app/spy identity and sample rate inputs
- Removed support for collapsed format
- Removed global tags from ruleset

## New Features
- Integrated pprof-rs backend into main crate behind optional `backend-pprof-rs` feature
- Switched to push API (from `/ingest` to `/push`)
- Generated push API protos
- Added `ThreadId` type
- Added `rustls-no-provider` TLS feature

## Bug Fixes / Improvements
- Unified signal logic
- Report cleanup functions
- Optimized ruleset
- Removed obscure thread id hash check
- Ruby: inline thread_id crate; remove detect_subprocess
- Dependency updates (reqwest 0.13, prost 0.14, thiserror 2.0, serde_json 1.0.115, uuid 1.20, libflate 2.1)

## v0.5.4
## New Features
- Add report transfromation function which allows changing the report before
  sending it to the Pyroscope Server.
- Add Gzip support.

## Bug Fixes
- Use URL Builder. ([786c89b](https://github.com/pyroscope-io/pyroscope-rs/commit/786c89bb99839c45778410012a6da60267d395df))

## v0.5.3
## New Features
- Add BackendConfig to make reporting of pid, thread_id and thread_name
  optional. 
- Backends can add a suffix to the "application_name"

## Bug Fixes
- **main**: fixed an obsecure bug when counting stacktraces ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/bdecaa13aeae3ce7d4c3d97f88bdd104ec35e7c5))

## v0.5.2
## New features
- Authentication Token support

## API Changes
- use rust-tls instead of openssl

## v0.5.1
## API Changes
- Backend shutdown bug
- Docs update

## v0.5.0
## API Changes
- New API for building, starting and stopping the profiling agent.
- Backend supports reporting multiple threads.
- Tagging within local thread-scope

## v0.4.0
## API Changes
- Backend now support passing a configuration struct.
- TimerSignal enum
- pprof-rs backend is split into a different package. It has to be imported manually.

## What's Changed
* fix: avoid binding two unrelated vars to the same type by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/18
* avoid almost all unwraps by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/14
* use more features of `thiserror` by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/11
* introduce LOG_TAGs, avoid repetitive prefixes by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/10
* allow configurable accumulation_cycle by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/21
* Add CI Targets by @omarabid in https://github.com/pyroscope-io/pyroscope-rs/pull/22
* 0.4.0 release by @omarabid in https://github.com/pyroscope-io/pyroscope-rs/pull/23

## New Contributors
* @drahnr made their first contribution in https://github.com/pyroscope-io/pyroscope-rs/pull/18

**Full Changelog**: https://github.com/pyroscope-io/pyroscope-rs/compare/0.3.1...lib-0.4.0

## v0.3.1
Minor release with bug fixes.

## Bug Fixes
- **session**: avoid breaking SessionManager if request fails ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/1254bcc9a3b0d76b7adbeb82ba21f33b875c0b39))
- **typo**: variable name typo ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/0d8caffbe7855bec8158333dba2923cd07286a5f))
- **pprof**: fix #12 ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/936d3d99a2cc82812bc8251fd2fbf8152a87d4cb))
- **copyright**: fix #13 ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/b8eaf13137810df932e1b70e33b3ad3e25b65ece))

## Code Refactoring
- **option**: replace unwrap for various Options ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/3fd4e794d74523855c66f65c0b7fc8ff9dfe4604))
- **time**: add get_time_range ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/a6d4dbcef494b2bfe8016a817201499937cf3528))

## v0.3.0
First stable release

## v0.0.2-alpha
Second beta release

## v0.0.1-alpha
Initial beta release
