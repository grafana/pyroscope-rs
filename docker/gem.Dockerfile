ARG PLATFORM=x86_64
ARG BASE_VERSION=v1
ARG REGISTRY=ghcr.io/grafana/pyroscope-rs
FROM ${REGISTRY}/builder-manylinux:${BASE_VERSION}-${PLATFORM} AS builder

USER root
WORKDIR /pyroscope-rs

ADD rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD src src
ADD kit/ kit/
ADD pyroscope_ffi/ pyroscope_ffi/
# TODO --frozen
RUN --mount=type=cache,target=/opt/rust/cargo/registry cargo build -p ffiruby --release --no-default-features --features native-tls-vendored

FROM ruby:4.0@sha256:66302616aabd939350e9bd7bc31ccad5ef993a5ba5e93f0cc029bb82e80a8d3b AS builder-gem
WORKDIR /gem
ADD pyroscope_ffi/ruby /gem/

RUN bundle install

COPY --from=builder /pyroscope-rs/target/release/librbspy.so lib/rbspy/rbspy.so
ARG TARGET_TASK
RUN rake ${TARGET_TASK}

FROM scratch
COPY --from=builder-gem /gem/pkg/ /pkg/
