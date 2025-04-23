ARG PLATFORM=x86_64
FROM quay.io/pypa/manylinux2014_${PLATFORM} AS builder

ENV RUST_VERSION=1.85
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o ./rustup-init \
    && chmod +x ./rustup-init \
    && ./rustup-init  -y --default-toolchain=${RUST_VERSION} --default-host=$(arch)-unknown-linux-gnu
ENV PATH=/root/.cargo/bin:$PATH
RUN yum -y install gcc libffi-devel openssl-devel wget gcc-c++ glibc-devel make

WORKDIR /pyroscope-rs

ADD Cross.toml \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD src src
ADD pyroscope_backends pyroscope_backends
ADD pyroscope_cli pyroscope_cli
ADD pyroscope_ffi/ pyroscope_ffi/
# TODO --frozen
RUN --mount=type=cache,target=/root/.cargo/registry cargo build -p ffiruby --release
RUN --mount=type=cache,target=/root/.cargo/registry cargo build -p thread_id --release

FROM ruby:3.3 as builder-gem
WORKDIR /gem
ADD pyroscope_ffi/ruby /gem/

RUN bundle install

COPY --from=builder /pyroscope-rs/target/release/librbspy.so lib/rbspy/rbspy.so
COPY --from=builder /pyroscope-rs/target/release/libthread_id.so lib/thread_id/thread_id.so
ARG TARGET_TASK
RUN rake ${TARGET_TASK}

FROM scratch
COPY --from=builder-gem /gem/pkg/ /pkg/
