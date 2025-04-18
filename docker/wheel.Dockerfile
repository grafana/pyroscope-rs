FROM ghcr.io/pyo3/maturin AS builder
RUN rustup default 1.85

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
ADD README.md README.md

RUN --mount=type=cache,target=/root/.cargo/registry \
    cd /pyroscope-rs/pyroscope_ffi/python && \
    maturin build --release --locked --compatibility=manylinux2014 &&\
    maturin build --release --sdist --locked

FROM scratch
COPY --from=builder /pyroscope-rs/target/wheels /dist
