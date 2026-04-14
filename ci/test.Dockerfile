ARG BASE_IMAGE=rust:trixie@sha256:652612f07bfbbdfa3af34761c1e435094c00dde4a98036132fca28c7bb2b165c
FROM ${BASE_IMAGE}

# Install make - needed by tikv-jemalloc-sys build script.
# Alpine doesn't include it; Debian-based images already have it.
RUN if command -v apk > /dev/null; then apk add --no-cache make; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
