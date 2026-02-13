
MANYLINUX_PREFIX=pyroscope/rust_builder
MANYLINUX_VERSION=4
BUILD_ARCH_AMD=manylinux2014_x86_64
BUILD_ARCH_ARM=manylinux2014_aarch64

.phony: pyroscope_ffi/clean
pyroscope_ffi/clean:
	make -C pyroscope_ffi/python/ clean


.phony: wheel/linux/amd64
wheel/linux/amd64: pyroscope_ffi/clean
	docker buildx build \
		--build-arg=PLATFORM=x86_64 \
	 	--platform=linux/amd64 \
	 	--output=pyroscope_ffi/python \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/linux/arm64
wheel/linux/arm64: pyroscope_ffi/clean
	docker buildx build \
		--build-arg=PLATFORM=aarch64 \
	 	--platform=linux/arm64 \
	 	--output=pyroscope_ffi/python \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/mac/amd64
wheel/mac/amd64:
	cd pyroscope_ffi/python && \
		pip install wheel setuptools-rust cffi pycparser && \
		CARGO_BUILD_TARGET=x86_64-apple-darwin \
			python setup.py bdist_wheel -p macosx-11_0_x86_64

.phony: wheel/mac/arm64
wheel/mac/arm64:
	cd pyroscope_ffi/python && \
		pip install wheel setuptools-rust cffi pycparser && \
		CARGO_BUILD_TARGET=aarch64-apple-darwin \
			python setup.py bdist_wheel -p macosx-11_0_arm64


.phony: gem/linux/amd64
gem/linux/amd64: pyroscope_ffi/clean
	docker buildx build \
		--build-arg=PLATFORM=x86_64 \
		--build-arg="TARGET_TASK=x86_64_linux:gem" \
		--output=pyroscope_ffi/ruby \
	 	--platform=linux/amd64 \
	 	-f docker/gem.Dockerfile \
	 	.

.phony: gem/linux/arm64
gem/linux/arm64: pyroscope_ffi/clean
	docker buildx build  \
		--build-arg=PLATFORM=aarch64 \
		--build-arg="TARGET_TASK=aarch64_linux:gem" \
		--output=pyroscope_ffi/ruby \
		--platform=linux/arm64 \
		-f docker/gem.Dockerfile \
	 	.

.phony: gem/mac/amd64
gem/mac/amd64: pyroscope_ffi/clean
	cd pyroscope_ffi/ruby && \
		bundle && \
		RUST_TARGET=x86_64-apple-darwin rake rbspy_install && \
		RUST_TARGET=x86_64-apple-darwin rake x86_64_darwin:gem

.phony: gem/mac/arm64
gem/mac/arm64: pyroscope_ffi/clean
	cd pyroscope_ffi/ruby && \
		bundle && \
		RUST_TARGET=aarch64-apple-darwin rake rbspy_install && \
		RUST_TARGET=aarch64-apple-darwin rake arm64_darwin:gem
