
MANYLINUX_PREFIX=pyroscope/rust_builder
MANYLINUX_VERSION=4
BUILD_ARCH_AMD=manylinux2014_x86_64
BUILD_ARCH_ARM=manylinux2014_aarch64

.phony: wheel/amd64
wheel/amd64:
	docker build \
		--build-arg=BASE=$(MANYLINUX_PREFIX)_$(BUILD_ARCH_AMD):$(MANYLINUX_VERSION) \
	 	--platform=linux/amd64 \
	 	--output=pyroscope_ffi/python \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/arm64
wheel/arm64:
	docker build \
		--build-arg=BASE=$(MANYLINUX_PREFIX)_$(BUILD_ARCH_ARM):$(MANYLINUX_VERSION) \
	 	--platform=linux/arm64 \
	 	--output=pyroscope_ffi/python \
	 	-f docker/wheel.Dockerfile \
	 	.