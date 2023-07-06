PREFIX ?= pyroscope/rust_builder
CLI_BUILDER_VERSION ?= 1
.PHONY: build_x86_64
build_x86_64:
	docker build -t $(PREFIX)_manylinux2010_x86_64 -f Dockerfile.manylinux2010_x86_64 .

.PHONY: push_x86_64
push_x86_64: build_x86_64
	docker push $(PREFIX)_manylinux2010_x86_64


.PHONY: build_aarch64
build_aarch64:
	docker build -t $(PREFIX)_manylinux2014_aarch64 -f Dockerfile.manylinux2014_aarch64 .

.PHONY: push_aarch64
push_aarch64: build_aarch64
	docker push $(PREFIX)_manylinux2014_aarch64


.PHONY: push_cli_builder
push_cli_builder:
	docker buildx build --platform linux/amd64,linux/arm64 -t $(PREFIX)_cli:$(CLI_BUILDER_VERSION) -f Dockerfile.cli_builder --push .

