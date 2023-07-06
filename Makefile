include docker/*.mk

PROPAGATE_VARS :=
USE_CONTAINER ?= 0
COMMIT = $(shell git rev-parse --short HEAD)
DOCKER_EXTRA ?=
DOCKER_BUILDKIT=1


.PHONY: cli/test
cli/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	CARGO_TARGET_DIR=./.tmp/target/cli
	/bin/sh
	cd pyroscope_cli && cargo test
endif

.PHONY: cli/version
cli/version:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	@ cd pyroscope_cli && cargo pkgid | cut -d @ -f 2
endif

.PHONY: cli/docker-image
cli/docker-image:
	CLI_VERSION=$(shell make cli/version)
	 docker buildx build \
		--platform linux/amd64 \
		-t pyroscope/pyroscope-rs-cli:$(CLI_VERSION)-$(COMMIT) \
		-f docker/Dockerfile.cli $(DOCKER_EXTRA) \
		.

