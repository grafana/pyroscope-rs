# Changelog

## [2.0.0](https://github.com/grafana/pyroscope-rs/compare/lib-1.0.2...lib-2.0.0) (2026-03-09)


### ⚠ BREAKING CHANGES

* Memory profiling for Rust using jemalloc crate ([#378](https://github.com/grafana/pyroscope-rs/issues/378))
* replace reports Vec on ReportBatch with ReportData enum ([#447](https://github.com/grafana/pyroscope-rs/issues/447))

### Features

* add pysignalprof — signal-based CPU profiler for CPython 3.14 ([#417](https://github.com/grafana/pyroscope-rs/issues/417)) ([0976d99](https://github.com/grafana/pyroscope-rs/commit/0976d999e4a50c14459b8a0b39e72434be6d4bfc))
* add ReportBatch type for multi-profile support ([#437](https://github.com/grafana/pyroscope-rs/issues/437)) ([979a3b2](https://github.com/grafana/pyroscope-rs/commit/979a3b2805b14069d42aa50e874d815f3f865d19))
* **kindasafe:** add aarch64 support for safe memory reading ([#421](https://github.com/grafana/pyroscope-rs/issues/421)) ([6573284](https://github.com/grafana/pyroscope-rs/commit/6573284e7a81aa58c340a6e13541512cf8ff0080))
* **kindasafe:** add macOS support for x86_64 and aarch64 ([#423](https://github.com/grafana/pyroscope-rs/issues/423)) ([b5b4cb4](https://github.com/grafana/pyroscope-rs/commit/b5b4cb4dd26b63bebccf38e815694e37ac559e0f))
* **kindasafe:** prepare kindasafe crates for crates.io publishing with trusted publishing ([#434](https://github.com/grafana/pyroscope-rs/issues/434)) ([17bd1b9](https://github.com/grafana/pyroscope-rs/commit/17bd1b98d1b242d64a10d8f71ba4a24e4152b418))
* Memory profiling for Rust using jemalloc crate ([#378](https://github.com/grafana/pyroscope-rs/issues/378)) ([3a11f1e](https://github.com/grafana/pyroscope-rs/commit/3a11f1e2e4a3fb2c832f1ef0c98a5942bea6b622))
* replace reports Vec on ReportBatch with ReportData enum ([#447](https://github.com/grafana/pyroscope-rs/issues/447)) ([a773660](https://github.com/grafana/pyroscope-rs/commit/a773660952b33accaa441ef08e8acdd42f2c37f7))
* split kindasafe into no_std read crate and kindasafe_init ([#420](https://github.com/grafana/pyroscope-rs/issues/420)) ([5975b08](https://github.com/grafana/pyroscope-rs/commit/5975b084ce2025887d5d9d79626998b50a09cad3))
* use prebuilt OpenSSL 3.5.5 instead of vendored openssl-src crate ([#422](https://github.com/grafana/pyroscope-rs/issues/422)) ([7bf43b0](https://github.com/grafana/pyroscope-rs/commit/7bf43b0c6a824c378358c6cd152f29919fe65ce5))


### Bug Fixes

* **ci:** add cargo-workspace plugin to release-please config ([#455](https://github.com/grafana/pyroscope-rs/issues/455)) ([6a81280](https://github.com/grafana/pyroscope-rs/commit/6a81280cd8808237fc6f9d536fbc20e1a95e8e93))
* **ci:** add version to pysignalprof dep to fix release-please crash ([#452](https://github.com/grafana/pyroscope-rs/issues/452)) ([fc519f6](https://github.com/grafana/pyroscope-rs/commit/fc519f6c816e1c79f54e492fa870e7ba6d0d122d))
* **ci:** bump Rust toolchain to 1.88.0 for naked_functions stabilization ([#419](https://github.com/grafana/pyroscope-rs/issues/419)) ([37bd11f](https://github.com/grafana/pyroscope-rs/commit/37bd11f3d8788b5747b4a6ee7104b4a1cdb9c16c))
* **ci:** configure release-please to update Cargo.lock ([#454](https://github.com/grafana/pyroscope-rs/issues/454)) ([218900e](https://github.com/grafana/pyroscope-rs/commit/218900e255405b6cc59242a2344c27acb9c60d9a))
* **ci:** make job keys and display names unique across FFI workflows ([#435](https://github.com/grafana/pyroscope-rs/issues/435)) ([2e80296](https://github.com/grafana/pyroscope-rs/commit/2e80296ef93980bf07dd8e47f41a033c838fe8c3))
* **ci:** remove invalid cargo-workspace extra-files entries ([#456](https://github.com/grafana/pyroscope-rs/issues/456)) ([4f5cb0b](https://github.com/grafana/pyroscope-rs/commit/4f5cb0b0f8dc2abf7d090c9e8dc208de4f6b8146))
* **ci:** remove kindasafe_init extra-file to fix dependency cycle ([#457](https://github.com/grafana/pyroscope-rs/issues/457)) ([4ca3d0c](https://github.com/grafana/pyroscope-rs/commit/4ca3d0c0c7b44ebbff07d0ab758290e53d049d5c))
* **ci:** use valid extra-files type for release-please ([#448](https://github.com/grafana/pyroscope-rs/issues/448)) ([0933fad](https://github.com/grafana/pyroscope-rs/commit/0933fadeba7045325c6bc8afda8927961dba0a54))
* **deps:** update rust crate object to 0.38 ([#430](https://github.com/grafana/pyroscope-rs/issues/430)) ([d9514e4](https://github.com/grafana/pyroscope-rs/commit/d9514e490855ff3b99f86ab00643a4397c3ff405))
* **kindasafe:** use runtime page size in page boundary tests ([#425](https://github.com/grafana/pyroscope-rs/issues/425)) ([5dd0dac](https://github.com/grafana/pyroscope-rs/commit/5dd0dac6e6c5d6d20cea0cfb1c578ef85de93ca8))


### Miscellaneous Chores

* add Renovate config for OpenSSL and downgrade to 3.5.4 to test ([#424](https://github.com/grafana/pyroscope-rs/issues/424)) ([c6e3741](https://github.com/grafana/pyroscope-rs/commit/c6e3741f688fc5848f36dd0950065c1cdfb8deff))
* **config:** migrate Renovate config ([#431](https://github.com/grafana/pyroscope-rs/issues/431)) ([ae0b38a](https://github.com/grafana/pyroscope-rs/commit/ae0b38af43fda93b9d5a940c99980ee128b265b5))
* **deps:** disable py-spy default features to exclude CLI dependencies ([#418](https://github.com/grafana/pyroscope-rs/issues/418)) ([6ca5a16](https://github.com/grafana/pyroscope-rs/commit/6ca5a166d7e684a1096a5e9d549c8022e77d0257))
* **deps:** lock file maintenance ([#416](https://github.com/grafana/pyroscope-rs/issues/416)) ([1d192b0](https://github.com/grafana/pyroscope-rs/commit/1d192b046d41f98a5dd9819a865eaff37ab71bd9))
* **deps:** pin dependencies ([#404](https://github.com/grafana/pyroscope-rs/issues/404)) ([d55cf4c](https://github.com/grafana/pyroscope-rs/commit/d55cf4c4578a6836682f5378cd715e985ffeff70))
* **deps:** Revert update dependency openssl/openssl to v3.5.5 ([#428](https://github.com/grafana/pyroscope-rs/issues/428))" ([#433](https://github.com/grafana/pyroscope-rs/issues/433)) ([c81f039](https://github.com/grafana/pyroscope-rs/commit/c81f039b2b334bc00d1ac4ddbb24c5411ff480e6))
* **deps:** update dependency openssl/openssl to v3.5.5 ([#428](https://github.com/grafana/pyroscope-rs/issues/428)) ([3ebdee9](https://github.com/grafana/pyroscope-rs/commit/3ebdee9126011b4b23d4225c54979060900e88d8))
* **deps:** update dependency openssl/openssl to v3.5.5 ([#441](https://github.com/grafana/pyroscope-rs/issues/441)) ([886d586](https://github.com/grafana/pyroscope-rs/commit/886d586c2306bc9e9fa8f96e575a2ba9e930dbce))
* **deps:** update docker/dockerfile docker tag to v1.22 ([#405](https://github.com/grafana/pyroscope-rs/issues/405)) ([6c9bb7f](https://github.com/grafana/pyroscope-rs/commit/6c9bb7fa40608fa73ee178e671abd623f39ce57d))
* **deps:** update ruby:4.0 docker digest to 6630261 ([#402](https://github.com/grafana/pyroscope-rs/issues/402)) ([2429e52](https://github.com/grafana/pyroscope-rs/commit/2429e521890be36a0149115aeda7269b718e3722))
* **deps:** update ruby/setup-ruby action to v1.289.0 ([#406](https://github.com/grafana/pyroscope-rs/issues/406)) ([1463f69](https://github.com/grafana/pyroscope-rs/commit/1463f690dd939e905f1ec2e3a11b1b1cb6601dbe))
* **deps:** update rust crate pprof to v0.1500.3 ([#407](https://github.com/grafana/pyroscope-rs/issues/407)) ([0c6bcbc](https://github.com/grafana/pyroscope-rs/commit/0c6bcbc785d6f17c92e109f38190ef035bacb6cc))


### Continuous Integration

* add release-please and PR title check workflows ([#403](https://github.com/grafana/pyroscope-rs/issues/403)) ([b844fa6](https://github.com/grafana/pyroscope-rs/commit/b844fa6fdfcd8060e1ff5307edc538c1d8dc7e7f))
* fix PR title check and version script for renovate PRs ([#408](https://github.com/grafana/pyroscope-rs/issues/408)) ([a846ee4](https://github.com/grafana/pyroscope-rs/commit/a846ee476a7050add1e29aa9c6ef16683ae159e9))
* fix release-please illegal path traversal in extra-files ([#411](https://github.com/grafana/pyroscope-rs/issues/411)) ([f8a5fbf](https://github.com/grafana/pyroscope-rs/commit/f8a5fbf53ecaff40abb74bb4d1e7640b542f8e8f))
* pin get-vault-secrets action to commit SHA ([#412](https://github.com/grafana/pyroscope-rs/issues/412)) ([09125d0](https://github.com/grafana/pyroscope-rs/commit/09125d09f1b2773ee47e8cef94ce77ca72b31645))
* use github-hosted runners on forks, grafana runners on upstream ([#410](https://github.com/grafana/pyroscope-rs/issues/410)) ([20f22fd](https://github.com/grafana/pyroscope-rs/commit/20f22fd315fdd04433ea8569117d56f999f69bed))

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
