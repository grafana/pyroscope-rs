Pre-built base Docker images for CI builds.

base-manylinux.Dockerfile and base-musllinux.Dockerfile pre-install:
- System packages (gcc, libffi, etc.)
- Rust toolchain
- Python build tools

These are pushed to GHCR as:
  ghcr.io/<repo>/builder-manylinux:v1-x86_64
  ghcr.io/<repo>/builder-manylinux:v1-aarch64
  ghcr.io/<repo>/builder-musllinux:v1-x86_64
  ghcr.io/<repo>/builder-musllinux:v1-aarch64

The manylinux base is shared by both Python wheel and Ruby gem builds.

To rebuild manually: cd docker && make base/all
Or trigger the build-base-images workflow via workflow_dispatch.

When bumping Rust version or changing base dependencies:
1. Update the base-*.Dockerfile files
2. Bump BASE_VERSION in ffi.mk, docker/Makefile, and consumer Dockerfiles
3. Merge to main (triggers automatic rebuild)
