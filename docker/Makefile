PREFIX ?= pyroscope/rust_builder

.PHONY: build_x86_64
build_x86_64:
	docker build -t $(PREFIX)_manylinux2010_x86_64 -f Dockerfile.manylinux2010_x86_64 .

.PHONY: push_x86_64
push_x86_64: build_x86_64
	docker push $(PREFIX)_manylinux2010_x86_64

.PHONY: playground_x86_64
playground_x86_64: build_x86_64
	docker run --rm -ti -v $(shell pwd)/../:/playground $(PREFIX)_manylinux2010_x86_64

.PHONY: build_aarch64
build_aarch64:
	docker build -t $(PREFIX)_manylinux2014_aarch64 -f Dockerfile.manylinux2014_aarch64 .

.PHONY: push_aarch64
push_aarch64: build_aarch64
	docker push $(PREFIX)_manylinux2014_aarch64

.PHONY: playground_aarch64
playground_aarch64: build_aarch64
	docker run --rm -ti -v $(shell pwd)/../:/playground $(PREFIX)_manylinux2014_aarch64
