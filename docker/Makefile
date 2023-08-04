PREFIX ?= pyroscope/rust_builder
CLI_BUILDER_VERSION ?= 2
MANYLINUX_VERSION ?= 2
.PHONY: push_x86_64
push_x86_64:
	docker buildx build -t $(PREFIX)_manylinux2014_x86_64:$(MANYLINUX_VERSION) -f Dockerfile.manylinux2014_x86_64 --push .



.PHONY: push_aarch64
push_aarch64:
	docker buildx build -t $(PREFIX)_manylinux2014_aarch64:$(MANYLINUX_VERSION) -f Dockerfile.manylinux2014_aarch64 --push .


.PHONY: push_cli_builder
push_cli_builder:
	docker buildx build --platform linux/amd64,linux/arm64 -t $(PREFIX)_cli:$(CLI_BUILDER_VERSION) -f Dockerfile.cli_builder --push .

