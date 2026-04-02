
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

.PHONY: check/lib-tag-version
check/lib-tag-version:
	@TAG_VERSION=$${TAG#lib-}; \
	CARGO_VERSION=$$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1); \
	if [ "$$TAG_VERSION" != "$$CARGO_VERSION" ]; then \
		echo "error: tag version ($$TAG_VERSION) does not match Cargo.toml version ($$CARGO_VERSION)"; \
		exit 1; \
	fi; \
	echo "tag version ($$TAG_VERSION) matches Cargo.toml version ($$CARGO_VERSION)"
