# Changelog

## [2.0.1](https://github.com/grafana/pyroscope-rs/compare/lib-2.0.0...lib-2.0.1) (2026-04-03)


### Bug Fixes

* **ci:** add changelog-sections to include chore/deps commits ([975ec76](https://github.com/grafana/pyroscope-rs/commit/975ec76b8cc9a7a472e974558de08172ec821f09))
* **ci:** move permissions to job level for zizmor compliance ([4dce16a](https://github.com/grafana/pyroscope-rs/commit/4dce16a2e236151a2182225e9e018f3997edfc0d))
* **ci:** pin release-please action SHA ([f1f9f96](https://github.com/grafana/pyroscope-rs/commit/f1f9f960cc9f87951f0303e1e99e5e3ec586ed1a))
* revert 0976d999e4a50c14459b8a0b39e72434be6d4bfc     ([#483](https://github.com/grafana/pyroscope-rs/issues/483)) ([ad3d032](https://github.com/grafana/pyroscope-rs/commit/ad3d03217a14a18868b8ba21f3dbf4ed844d759a))


### Miscellaneous Chores

* allow all renovate updates for pprof-pyroscope-fork crate ([cdc26ba](https://github.com/grafana/pyroscope-rs/commit/cdc26ba16468f0b7495aa47104503c339cdb4373))
* **deps:** lock file maintenance ([#467](https://github.com/grafana/pyroscope-rs/issues/467)) ([09411da](https://github.com/grafana/pyroscope-rs/commit/09411daca91120c4863983273863c55b02b4dd89))
* **deps:** lock file maintenance ([#478](https://github.com/grafana/pyroscope-rs/issues/478)) ([447d8ee](https://github.com/grafana/pyroscope-rs/commit/447d8eef3621be5b9b09b0b9eca3968bca081c2a))
* **deps:** update rust crate rbspy to 0.43 ([#471](https://github.com/grafana/pyroscope-rs/issues/471)) ([1e524d3](https://github.com/grafana/pyroscope-rs/commit/1e524d38d84dfedc799515cb70ee434ef91a28e6))
* **deps:** update rust crate rbspy to 0.44 ([#477](https://github.com/grafana/pyroscope-rs/issues/477)) ([c7cfe39](https://github.com/grafana/pyroscope-rs/commit/c7cfe39fecd8a0624df5c5d22b709e73747ddb4d))
* **deps:** update rust crate uuid to v1.23.0 ([#481](https://github.com/grafana/pyroscope-rs/issues/481)) ([9c4b53a](https://github.com/grafana/pyroscope-rs/commit/9c4b53aef55b13dc6da99396119e33ec9dc02aa0))
* **deps:** update softprops/action-gh-release action to v2.5.2 ([#473](https://github.com/grafana/pyroscope-rs/issues/473)) ([1700be3](https://github.com/grafana/pyroscope-rs/commit/1700be3e34e7351a2f5f030187a2fac1ce1082e9))
* **deps:** update softprops/action-gh-release action to v2.5.3 ([#474](https://github.com/grafana/pyroscope-rs/issues/474)) ([12e34c1](https://github.com/grafana/pyroscope-rs/commit/12e34c109bd0e408dc0401430f4da4b5245a09da))
* remove FFI code and kindasafe (moved to separate repos) ([#488](https://github.com/grafana/pyroscope-rs/issues/488)) ([35a4c6a](https://github.com/grafana/pyroscope-rs/commit/35a4c6aa0808f0392e39e752d95a91536068ab74))


### Continuous Integration

* add release-please configuration ([e1c6460](https://github.com/grafana/pyroscope-rs/commit/e1c6460b9089d85c78cf94e5b4679b3165d6f0d9))

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
