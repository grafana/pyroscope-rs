
COMMIT = $(shell git rev-parse --short HEAD)
DOCKER_EXTRA ?=
DOCKER_BUILDKIT=1
BUMP ?= fix



.PHONY: lib/test
lib/test:
	cargo  test --manifest-path Cargo.toml

.PHONY: pprofrs/test
pprofrs/test:
	cargo  test --manifest-path Cargo.toml --features backend-pprof-rs


.PHONY: ffikit/test
ffikit/test:
	cargo  test --manifest-path pyroscope_ffi/ffikit/Cargo.toml

.PHONY: test
test: pprofrs/test  lib/test ffikit/test


.PHONY: rust/fmt
rust/fmt:
	cargo fmt --all


.PHONY: rust/fmt/check
rust/fmt/check:
	cargo fmt --all --check


.PHONY: ruby/version/bump
ruby/version/bump:
	BUMP=$(BUMP) bash scripts/bump_ffi_version.sh ruby


.PHONY: python/version/bump
python/version/bump:
	BUMP=$(BUMP) bash scripts/bump_ffi_version.sh python


include ffi.mk
