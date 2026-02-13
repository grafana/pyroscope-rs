# syntax=docker/dockerfile:1.4
ARG PLATFORM=x86_64
FROM quay.io/pypa/musllinux_1_2_${PLATFORM} AS builder

RUN apk add --no-cache gcc libffi-dev openssl-dev make musl-dev rust cargo

WORKDIR /pyroscope-rs

ADD pyroscope_ffi/python/requirements.txt /pyroscope-rs/pyroscope_ffi/python/
RUN /opt/python/cp39-cp39/bin/python -m pip install -r /pyroscope-rs/pyroscope_ffi/python/requirements.txt

ADD rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD src src
ADD pyroscope_ffi/ pyroscope_ffi/

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cd /pyroscope-rs/pyroscope_ffi/python && ./musllinux.sh

FROM scratch
COPY --from=builder /pyroscope-rs/pyroscope_ffi/python/dist dist/
