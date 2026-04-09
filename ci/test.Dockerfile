ARG BASE_IMAGE=rust:trixie@sha256:e8e2bb5ff27ad3b369a4f667392464e6ec399cfe81c1230ae78edb1036b9bd74
FROM ${BASE_IMAGE}

# Install make - needed by tikv-jemalloc-sys build script.
# Alpine doesn't include it; Debian-based images already have it.
RUN if command -v apk > /dev/null; then apk add --no-cache make; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
