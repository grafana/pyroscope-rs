ARG BASE

FROM ${BASE} as builder-native

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
RUN cargo build -p rbspy --release
RUN cargo build -p thread_id --release

FROM ruby:3.3 as builder-gem
WORKDIR /gem
ADD pyroscope_ffi/ruby /gem/

RUN bundle install

COPY --from=builder-native /pyroscope-rs/target/release/librbspy.so lib/rbspy/rbspy.so
COPY --from=builder-native /pyroscope-rs/target/release/libthread_id.so lib/thread_id/thread_id.so
ARG TARGET_TASK
RUN rake ${TARGET_TASK}

FROM scratch
COPY --from=builder-gem /gem/pkg/ /pkg/
