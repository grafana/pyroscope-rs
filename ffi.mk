
MANYLINUX_PREFIX=pyroscope/rust_builder
MANYLINUX_VERSION=4
BUILD_ARCH_AMD=manylinux2014_x86_64
BUILD_ARCH_ARM=manylinux2014_aarch64

.phony: pyroscope_ffi/clean
pyroscope_ffi/clean:
	cargo clean
	make -C pyroscope_ffi/python/ clean
	make -C pyroscope_ffi/ruby/ clean


.phony: wheel/linux/amd64
wheel/linux/amd64: pyroscope_ffi/clean
	docker build \
		--build-arg=BASE=$(MANYLINUX_PREFIX)_$(BUILD_ARCH_AMD):$(MANYLINUX_VERSION) \
	 	--platform=linux/amd64 \
	 	--output=pyroscope_ffi/python \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/linux/arm64
wheel/linux/arm64: pyroscope_ffi/clean
	docker build \
		--build-arg=BASE=$(MANYLINUX_PREFIX)_$(BUILD_ARCH_ARM):$(MANYLINUX_VERSION) \
	 	--platform=linux/arm64 \
	 	--output=pyroscope_ffi/python \
	 	-f docker/wheel.Dockerfile \
	 	.

.phony: wheel/mac/amd64
wheel/mac/amd64:
	cd pyroscope_ffi/python && \
		pip install wheel && \
		python setup.py bdist_wheel -p macosx-11_0_x86_64

.phony: wheel/mac/arm64
wheel/mac/arm64:
	cd pyroscope_ffi/python && \
		pip install wheel && \
		python setup.py bdist_wheel -p macosx-11_0_arm64


.phony: gem/linux/amd64
gem/linux/amd64: pyroscope_ffi/clean
	docker build \
		--build-arg=BASE=$(MANYLINUX_PREFIX)_$(BUILD_ARCH_AMD):$(MANYLINUX_VERSION) \
		--build-arg="TARGET_TASK=x86_64_linux:gem" \
		--output=pyroscope_ffi/ruby \
	 	--platform=linux/amd64 \
	 	-f docker/gem.Dockerfile \
	 	.

.phony: gem/linux/arm64
gem/linux/arm64: pyroscope_ffi/clean
	docker build \
		--build-arg=BASE=$(MANYLINUX_PREFIX)_$(BUILD_ARCH_ARM):$(MANYLINUX_VERSION) \
		--build-arg="TARGET_TASK=arm64_darwin:gem" \
	 	--platform=linux/arm64 \
	 	--output=pyroscope_ffi/ruby \
	 	-f docker/gem.Dockerfile \
	 	.

.phony: gem/mac/amd64
gem/mac/amd64: pyroscope_ffi/clean
	cd pyroscope_ffi/ruby && \
		bundle && \
		rake rbspy_install && \
		rake thread_id_install && \
		rake x86_64_linux:gem

.phony: gem/mac/arm64
gem/mac/arm64: pyroscope_ffi/clean
	cd pyroscope_ffi/ruby && \
		bundle && \
		rake rbspy_install && \
		rake thread_id_install && \
		rake arm64_darwin:gem