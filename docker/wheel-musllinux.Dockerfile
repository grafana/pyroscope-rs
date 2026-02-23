# syntax=docker/dockerfile:1.21@sha256:27f9262d43452075f3c410287a2c43f5ef1bf7ec2bb06e8c9eeb1b8d453087bc
ARG PLATFORM=x86_64
FROM quay.io/pypa/musllinux_1_2_${PLATFORM} AS builder

RUN apk add --no-cache gcc musl-dev libffi-dev make

RUN adduser -D builder \
    && mkdir -p /pyroscope-rs \
    && chown builder:builder /pyroscope-rs

USER builder
RUN test "$(id -u)" = "1000" || { echo "ERROR: builder uid is $(id -u), expected 1000"; exit 1; }

ENV RUST_VERSION=1.87
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --default-toolchain=${RUST_VERSION} --default-host=$(arch)-unknown-linux-musl \
    && rm /tmp/rustup-init
ENV PATH=/home/builder/.cargo/bin:$PATH

WORKDIR /pyroscope-rs

RUN /opt/python/cp39-cp39/bin/python -m pip install --user build

ADD --chown=builder:builder pyproject.toml \
    setup.py \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD --chown=builder:builder src src
ADD --chown=builder:builder pyroscope_ffi/ pyroscope_ffi/

ARG PYROSCOPE_CARGO_NO_DEFAULT_FEATURES=1
ARG PYROSCOPE_CARGO_FEATURES=native-tls-vendored
ENV PYROSCOPE_CARGO_NO_DEFAULT_FEATURES=${PYROSCOPE_CARGO_NO_DEFAULT_FEATURES}
ENV PYROSCOPE_CARGO_FEATURES=${PYROSCOPE_CARGO_FEATURES}

RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,gid=1000 \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,gid=1000 \
    /opt/python/cp39-cp39/bin/python -m build --wheel

USER root
RUN auditwheel repair dist/*.whl --wheel-dir dist-repaired/

FROM scratch
COPY --from=builder  /pyroscope-rs/dist-repaired dist/
