
MANYLINUX_PREFIX=pyroscope/rust_builder
MANYLINUX_VERSION=4
BUILD_ARCH_AMD=manylinux2014_x86_64
BUILD_ARCH_ARM=manylinux2014_aarch64

.phony: wheel/linux/amd64
wheel/linux/amd64:
	docker buildx build \
		--build-arg=PLATFORM=x86_64 \
	 	--platform=linux/amd64 \
	 	--output=. \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/linux/arm64
wheel/linux/arm64:
	docker buildx build \
		--build-arg=PLATFORM=aarch64 \
	 	--platform=linux/arm64 \
	 	--output=. \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/musllinux/amd64
wheel/musllinux/amd64:
	docker buildx build \
		--build-arg=PLATFORM=x86_64 \
	 	--platform=linux/amd64 \
	 	--output=. \
	 	-f docker/wheel-musllinux.Dockerfile \
	 	.

.phony: wheel/musllinux/arm64
wheel/musllinux/arm64:
	docker buildx build \
		--build-arg=PLATFORM=aarch64 \
	 	--platform=linux/arm64 \
	 	--output=. \
	 	-f docker/wheel-musllinux.Dockerfile \
	 	.

.phony: wheel/mac/amd64
wheel/mac/amd64:
	MACOSX_DEPLOYMENT_TARGET=11.0 CARGO_BUILD_TARGET=x86_64-apple-darwin python -m build --wheel
	wheel tags --platform-tag macosx_11_0_x86_64 --remove dist/*.whl

.phony: wheel/mac/arm64
wheel/mac/arm64:
	MACOSX_DEPLOYMENT_TARGET=11.0 CARGO_BUILD_TARGET=aarch64-apple-darwin python -m build --wheel
	wheel tags --platform-tag macosx_11_0_arm64 --remove dist/*.whl

