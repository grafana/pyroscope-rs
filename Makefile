
COMMIT = $(shell git rev-parse --short HEAD)
DOCKER_EXTRA ?=
DOCKER_BUILDKIT=1


.PHONY: cli/test
cli/test:
	cargo  test --manifest-path pyroscope_cli/Cargo.toml


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
test: cli/test pprofrs/test pyspy/test rbspy/test lib/test ffikit/test


.PHONY: cli/version
cli/version:
	@ cd pyroscope_cli && cargo pkgid | cut -d @ -f 2

.PHONY: cli/docker-image
cli/docker-image:
	 docker buildx build --platform linux/amd64 --load --progress=plain \
		-t pyroscope/pyroscope-rs-cli:$(shell make cli/version)-$(COMMIT)  \
		-f docker/cli.Dockerfile $(DOCKER_EXTRA) .

include ffi.mk