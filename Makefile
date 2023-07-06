CLI_BUILDER_IMAGE ?= pyroscope/rust_builder_cli:1
LOCAL_CARGO_REGISTRY ?= $(shell echo $(HOME)/.cargo/registry)
CLI_VERSION ?= $(shell docker run --rm -v $(shell pwd):/mnt -w /mnt/pyroscope_cli $(CLI_BUILDER_IMAGE)  cargo pkgid | cut -d @ -f 2)



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
		-t pyroscope/pyroscope-rs-cli:$(CLI_VERSION) \
		-f docker/Dockerfile.cli \
		.

