ARG BASE_IMAGE=rust:trixie@sha256:e4f09e8fe5a2366e7d3dc35e08bd25821151e3ed8fdbd3a6a16b51555f0c551d
FROM ${BASE_IMAGE}

# Install make - needed by tikv-jemalloc-sys build script.
# Alpine doesn't include it; Debian-based images already have it.
RUN if command -v apk > /dev/null; then apk add --no-cache make; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
