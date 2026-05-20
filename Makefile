
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


.PHONY: test
test: pprofrs/test  lib/test


.PHONY: rust/fmt
rust/fmt:
	cargo fmt --all


.PHONY: rust/fmt/check
rust/fmt/check:
	cargo fmt --all --check

.PHONY: rust/cross-compile/arm
rust/cross-compile/arm:
	docker build -t pyroscope-arm-cross -f ci/Dockerfile.arm-cross ci
	docker run --rm -v $(shell pwd):/work pyroscope-arm-cross cargo build --locked --target arm-unknown-linux-gnueabi --all-features
