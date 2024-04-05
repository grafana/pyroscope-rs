ARG BASE

FROM ${BASE} as builder

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

RUN cd /pyroscope-rs/pyroscope_ffi/python && ./manylinux.sh

FROM scratch
COPY --from=builder /pyroscope-rs/pyroscope_ffi/python/dist dist/