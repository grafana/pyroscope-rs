CLI_BUILDER_IMAGE ?= pyroscope/rust_builder_cli:1
LOCAL_CARGO_REGISTRY ?= $(shell echo $(HOME)/.cargo/registry)
CLI_VERSION ?= $(shell docker run --rm -v $(shell pwd):/mnt -w /mnt/pyroscope_cli $(CLI_BUILDER_IMAGE)  cargo pkgid | cut -d @ -f 2)
COMMIT ?= $(shell git rev-parse --short HEAD)
DOCKER_PUSH ?= 0
DOCKER_OUTPUT ?= 0

ifeq ($(DOCKER_PUSH),1)
	DOCKER_PUSH_FLAG := --push
else
	DOCKER_PUSH_FLAG :=
endif

ifeq ($(DOCKER_OUTPUT),1)
	DOCKER_OUTPUT_FLAG := --output=.
else
	DOCKER_OUTPUT_FLAG :=
endif

.PHONY: cli/build
cli/build:
	DOCKER_BUILDKIT=1 docker run --rm -v $(shell pwd):/mnt -v $(LOCAL_CARGO_REGISTRY):/root/.cargo/registry \
		-w /mnt/pyroscope_cli $(CLI_BUILDER_IMAGE) \
		cargo build

.PHONY: cli/test
cli/test:
	DOCKER_BUILDKIT=1 docker run --rm -v $(shell pwd):/mnt -v $(LOCAL_CARGO_REGISTRY):/root/.cargo/registry \
		-w /mnt/pyroscope_cli $(CLI_BUILDER_IMAGE) \
		cargo test

.PHONY: cli/docker-image
cli/docker-image:
	DOCKER_BUILDKIT=1 docker buildx build \
		--platform linux/amd64 \
		-t pyroscope/pyroscope-rs-cli:$(CLI_VERSION)-$(COMMIT) \
		-f docker/Dockerfile.cli $(DOCKER_OUTPUT_FLAG) $(DOCKER_PUSH_FLAG) \
		.

.PHONY: info
info:
	@printf "CLI_BUILDER_IMAGE      = $(CLI_BUILDER_IMAGE)\n"
	@printf "CLI_VERSION            = $(CLI_VERSION)\n"
	@printf "LOCAL_CARGO_REGISTRY   = $(LOCAL_CARGO_REGISTRY)\n"
	@printf "COMMIT                 = $(COMMIT)\n"
