include docker/*.mk

USE_CONTAINER ?= 0
CARGO_TARGET_DIR ?= target
ifeq ($(USE_CONTAINER),1)
	CARGO_TARGET_DIR := ./.tmp/target_container
endif

PROPAGATE_VARS := CARGO_TARGET_DIR
COMMIT = $(shell git rev-parse --short HEAD)
DOCKER_EXTRA ?=
DOCKER_BUILDKIT=1


.PHONY: cli/test
cli/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	cargo  test --manifest-path pyroscope_cli/Cargo.toml
endif


#, pprofrs, pyspy, rbspy, ffikit

.PHONY: lib/test
lib/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	cargo  test --manifest-path Cargo.toml
endif

.PHONY: pprofrs/test
pprofrs/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	cargo  test --manifest-path pyroscope_backends/pyroscope_pprofrs/Cargo.toml
endif

.PHONY: pyspy/test
pyspy/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	cargo  test --manifest-path pyroscope_backends/pyroscope_pyspy/Cargo.toml
endif

.PHONY: rbspy/test
rbspy/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	cargo  test --manifest-path pyroscope_backends/pyroscope_rbspy/Cargo.toml
endif


.PHONY: ffikit/test
ffikit/test:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	cargo  test --manifest-path pyroscope_ffi/ffikit/Cargo.toml
endif

.PHONY: test
test: cli/test pprofrs/test pyspy/test rbspy/test lib/test ffikit/test


.PHONY: cli/version
cli/version:
ifeq ($(USE_CONTAINER),1)
	$(RERUN_IN_CONTAINER)
else
	@ cd pyroscope_cli && cargo pkgid | cut -d @ -f 2
endif

.PHONY: cli/docker-image
cli/docker-image:
	 docker buildx build \
		--platform linux/amd64 \
		-t pyroscope/pyroscope-rs-cli:$(shell make cli/version)-$(COMMIT) \
		-f docker/Dockerfile.cli $(DOCKER_EXTRA) \
		.



# CI
drone:
	drone jsonnet -V BUILD_IMAGE_VERSION=$(BUILD_IMAGE_VERSION) --stream --format --source .drone/drone.jsonnet --target .drone/drone.yml
	drone lint .drone/drone.yml
	drone sign --save grafana/pyroscope-rs .drone/drone.yml