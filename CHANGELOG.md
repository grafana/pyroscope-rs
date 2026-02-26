# v0.6.0
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

# v0.5.4
## New Features
- Add report transfromation function which allows changing the report before
  sending it to the Pyroscope Server.
- Add Gzip support.

## Bug Fixes
- Use URL Builder. ([786c89b](https://github.com/pyroscope-io/pyroscope-rs/commit/786c89bb99839c45778410012a6da60267d395df))

# v0.5.3
## New Features
- Add BackendConfig to make reporting of pid, thread_id and thread_name
  optional. 
- Backends can add a suffix to the "application_name"

## Bug Fixes
- **main**: fixed an obsecure bug when counting stacktraces ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/bdecaa13aeae3ce7d4c3d97f88bdd104ec35e7c5))

# v0.5.2
## New features
- Authentication Token support

## API Changes
- use rust-tls instead of openssl

# v0.5.1
## API Changes
- Backend shutdown bug
- Docs update

# v0.5.0
## API Changes
- New API for building, starting and stopping the profiling agent.
- Backend supports reporting multiple threads.
- Tagging within local thread-scope

# v0.4.0
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

# v0.3.1
Minor release with bug fixes.

## Bug Fixes
- **session**: avoid breaking SessionManager if request fails ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/1254bcc9a3b0d76b7adbeb82ba21f33b875c0b39))
- **typo**: variable name typo ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/0d8caffbe7855bec8158333dba2923cd07286a5f))
- **pprof**: fix #12 ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/936d3d99a2cc82812bc8251fd2fbf8152a87d4cb))
- **copyright**: fix #13 ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/b8eaf13137810df932e1b70e33b3ad3e25b65ece))

## Code Refactoring
- **option**: replace unwrap for various Options ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/3fd4e794d74523855c66f65c0b7fc8ff9dfe4604))
- **time**: add get_time_range ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/a6d4dbcef494b2bfe8016a817201499937cf3528))

# v0.3.0
First stable release

# v0.0.2-alpha
Second beta release

# v0.0.1-alpha
Initial beta release
