FROM rust:latest AS builder

RUN rustup default 1.85
#RUN rustup target add aarch64-unknown-linux-musl
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get -y install musl-dev
WORKDIR /app
ADD pyroscope_backends ./pyroscope_backends
ADD pyroscope_cli ./pyroscope_cli
ADD pyroscope_ffi ./pyroscope_ffi
ADD src ./src
ADD Cargo.toml ./Cargo.toml

RUN cd pyroscope_cli && \
    cargo build --release --bin pyroscope-cli --target x86_64-unknown-linux-musl

FROM scratch as final
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/pyroscope-cli /pyroscope-cli
ENTRYPOINT ["/pyroscope-cli"]
