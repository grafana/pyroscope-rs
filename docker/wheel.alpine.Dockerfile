# syntax=docker/dockerfile:1.4
# Build wheel natively on Alpine Linux (musl libc).
# We cannot use CARGO_BUILD_TARGET=x86_64-unknown-linux-musl from a glibc host
# because that target does not support cdylib. Instead we run the entire build
# inside an Alpine container where the native toolchain already targets musl.
FROM python:3.9-alpine AS builder

RUN apk add --no-cache \
    gcc \
    musl-dev \
    libffi-dev \
    make \
    curl \
    git

ENV RUST_VERSION=1.87
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --default-toolchain=${RUST_VERSION} \
    && rm /tmp/rustup-init
ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /pyroscope-rs

RUN python -m pip install build

ADD pyproject.toml \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD src src
ADD pyroscope_ffi/ pyroscope_ffi/

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    RUSTFLAGS="-C target-feature=-crt-static" python -m build --wheel

FROM scratch
COPY --from=builder /pyroscope-rs/dist dist/
