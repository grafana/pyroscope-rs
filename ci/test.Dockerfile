ARG BASE_IMAGE=rust:trixie@sha256:39d8cb39a54e7d1da665c4fabfdd265e532a5f836c11ab5aee27fd5c73891ce4
FROM ${BASE_IMAGE}

# Install make - needed by tikv-jemalloc-sys build script.
# Alpine doesn't include it; Debian-based images already have it.
RUN if command -v apk > /dev/null; then apk add --no-cache make; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
