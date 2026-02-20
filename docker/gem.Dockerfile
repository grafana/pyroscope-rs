ARG PLATFORM=x86_64
FROM quay.io/pypa/manylinux2014_${PLATFORM} AS builder

ENV RUST_VERSION=1.87
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o ./rustup-init \
    && chmod +x ./rustup-init \
    && ./rustup-init  -y --default-toolchain=${RUST_VERSION} --default-host=$(arch)-unknown-linux-gnu
ENV PATH=/root/.cargo/bin:$PATH
RUN yum -y install gcc libffi-devel perl-core wget gcc-c++ glibc-devel make

WORKDIR /pyroscope-rs

ADD rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD src src
ADD pyroscope_ffi/ pyroscope_ffi/
# TODO --frozen
RUN --mount=type=cache,target=/root/.cargo/registry cargo build -p ffiruby --release --no-default-features --features native-tls-vendored

FROM ruby:3.3@sha256:c8b7d45819460a8f8c9586d425ba5cd1e4da124da7a35800d419db4d8d9cde01 AS builder-gem
WORKDIR /gem
ADD pyroscope_ffi/ruby /gem/

RUN bundle install

COPY --from=builder /pyroscope-rs/target/release/librbspy.so lib/rbspy/rbspy.so
ARG TARGET_TASK
RUN rake ${TARGET_TASK}

FROM scratch
COPY --from=builder-gem /gem/pkg/ /pkg/
