ARG BASE_IMAGE=rust:trixie@sha256:4a7e3a0c309c9bab658e469f842711bd595fae484936bc5d605e08ca0c631bf4
FROM ${BASE_IMAGE}

# Install make - needed by tikv-jemalloc-sys build script.
# Alpine doesn't include it; Debian-based images already have it.
RUN if command -v apk > /dev/null; then apk add --no-cache make; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
