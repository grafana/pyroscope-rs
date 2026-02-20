# syntax=docker/dockerfile:1.4
ARG PLATFORM=x86_64
FROM quay.io/pypa/manylinux2014_${PLATFORM} AS builder

RUN yum -y install gcc libffi-devel openssl-devel glibc-devel make

RUN useradd -m builder \
    && mkdir -p /pyroscope-rs \
    && chown builder:builder /pyroscope-rs

USER builder
RUN test "$(id -u)" = "1000" || { echo "ERROR: builder uid is $(id -u), expected 1000"; exit 1; }

ENV RUST_VERSION=1.87
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --default-toolchain=${RUST_VERSION} --default-host=$(arch)-unknown-linux-gnu \
    && rm /tmp/rustup-init
ENV PATH=/home/builder/.cargo/bin:$PATH

WORKDIR /pyroscope-rs

RUN /opt/python/cp39-cp39/bin/python -m pip install --user build

ADD --chown=builder:builder pyproject.toml \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD --chown=builder:builder src src
ADD --chown=builder:builder pyroscope_ffi/ pyroscope_ffi/

RUN /opt/python/cp39-cp39/bin/python3 -c "
import re
content = open('pyproject.toml').read()
content = content.replace(
    'cargo_manifest_args=[\"--locked\"]',
    'cargo_manifest_args=[\"--locked\"]\nfeatures = [\"native-tls-vendored\"]\nno_default_features = true'
)
open('pyproject.toml', 'w').write(content)
"

RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,gid=1000 \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,gid=1000 \
    /opt/python/cp39-cp39/bin/python -m build --wheel

FROM scratch
COPY --from=builder  /pyroscope-rs/dist dist/
