REGISTRY ?= ghcr.io/grafana/pyroscope-rs
BASE_VERSION ?= v1

.PHONY: base/manylinux/amd64
base/manylinux/amd64:
	docker buildx build \
		--build-arg=PLATFORM=x86_64 \
		--platform=linux/amd64 \
		-t $(REGISTRY)/builder-manylinux:$(BASE_VERSION)-x86_64 \
		-f base-manylinux.Dockerfile \
		--push .

.PHONY: base/manylinux/arm64
base/manylinux/arm64:
	docker buildx build \
		--build-arg=PLATFORM=aarch64 \
		--platform=linux/arm64 \
		-t $(REGISTRY)/builder-manylinux:$(BASE_VERSION)-aarch64 \
		-f base-manylinux.Dockerfile \
		--push .

.PHONY: base/musllinux/amd64
base/musllinux/amd64:
	docker buildx build \
		--build-arg=PLATFORM=x86_64 \
		--platform=linux/amd64 \
		-t $(REGISTRY)/builder-musllinux:$(BASE_VERSION)-x86_64 \
		-f base-musllinux.Dockerfile \
		--push .

.PHONY: base/musllinux/arm64
base/musllinux/arm64:
	docker buildx build \
		--build-arg=PLATFORM=aarch64 \
		--platform=linux/arm64 \
		-t $(REGISTRY)/builder-musllinux:$(BASE_VERSION)-aarch64 \
		-f base-musllinux.Dockerfile \
		--push .

.PHONY: base/all
base/all: base/manylinux/amd64 base/manylinux/arm64 base/musllinux/amd64 base/musllinux/arm64
