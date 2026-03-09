# syntax=docker/dockerfile:1.22@sha256:4a43a54dd1fedceb30ba47e76cfcf2b47304f4161c0caeac2db1c61804ea3c91
ARG PLATFORM=x86_64
ARG BASE_VERSION=v1
ARG REGISTRY=ghcr.io/grafana/pyroscope-rs
FROM ${REGISTRY}/builder-musllinux:${BASE_VERSION}-${PLATFORM} AS builder

ADD --chown=builder:builder pyproject.toml \
    setup.py \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD --chown=builder:builder src src
ADD --chown=builder:builder kit/ kit/
ADD --chown=builder:builder pyroscope_ffi/ pyroscope_ffi/

ARG PYROSCOPE_CARGO_NO_DEFAULT_FEATURES=1
ARG PYROSCOPE_CARGO_FEATURES=native-tls-vendored
ENV PYROSCOPE_CARGO_NO_DEFAULT_FEATURES=${PYROSCOPE_CARGO_NO_DEFAULT_FEATURES}
ENV PYROSCOPE_CARGO_FEATURES=${PYROSCOPE_CARGO_FEATURES}

RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,gid=1000 \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,gid=1000 \
    /opt/python/cp310-cp310/bin/python -m build --wheel

RUN auditwheel repair dist/*.whl --wheel-dir dist-repaired/

FROM scratch
COPY --from=builder  /pyroscope-rs/dist-repaired dist/
