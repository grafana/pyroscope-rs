# syntax=docker/dockerfile:1.22@sha256:4a43a54dd1fedceb30ba47e76cfcf2b47304f4161c0caeac2db1c61804ea3c91
ARG PLATFORM=x86_64
FROM quay.io/pypa/musllinux_1_2_${PLATFORM}

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

RUN /opt/python/cp310-cp310/bin/python -m pip install --user build
