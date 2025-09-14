
COMMIT = $(shell git rev-parse --short HEAD)
DOCKER_EXTRA ?=
DOCKER_BUILDKIT=1



.PHONY: lib/test
lib/test:
	cargo  test --manifest-path Cargo.toml

.PHONY: pprofrs/test
pprofrs/test:
	cargo  test --manifest-path pyroscope_backends/pyroscope_pprofrs/Cargo.toml

.PHONY: pyspy/test
pyspy/test:
	cargo  test --manifest-path pyroscope_backends/pyroscope_pyspy/Cargo.toml

.PHONY: rbspy/test
rbspy/test:
	cargo  test --manifest-path pyroscope_backends/pyroscope_rbspy/Cargo.toml


.PHONY: ffikit/test
ffikit/test:
	cargo  test --manifest-path pyroscope_ffi/ffikit/Cargo.toml

.PHONY: test
test: pprofrs/test pyspy/test rbspy/test lib/test ffikit/test


include ffi.mk