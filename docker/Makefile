PREFIX ?= pyroscope/rust_builder

.PHONY: build_x86_64
build_x86_64:
	docker build -t $(PREFIX)_manylinux2010_x86_64 -f Dockerfile.manylinux2010_x86_64 .

.PHONY: run_x86_64
run_x86_64: build_x86_64
	docker run --rm -ti $(PREFIX)_manylinux2010_x86_64
