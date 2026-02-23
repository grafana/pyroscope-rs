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

FROM ruby:4.0@sha256:36ec449d363dbe7ddb7784a12934a01880fd3a86f6bf059dc1f87f0d83afc953 AS builder-gem
WORKDIR /gem
ADD pyroscope_ffi/ruby /gem/

RUN bundle install

COPY --from=builder /pyroscope-rs/target/release/librbspy.so lib/rbspy/rbspy.so
ARG TARGET_TASK
RUN rake ${TARGET_TASK}

FROM scratch
COPY --from=builder-gem /gem/pkg/ /pkg/
