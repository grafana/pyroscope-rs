FROM quay.io/pypa/manylinux2014 AS toolchain-musl
RUN mkdir -p /opt/cross/musl
RUN curl https://musl.cc/x86_64-linux-musl-cross.tgz  | tar -xzf - -C /opt/cross/musl
RUN curl https://musl.cc/aarch64-linux-musl-cross.tgz | tar -xzf - -C /opt/cross/musl

FROM quay.io/pypa/manylinux2014 AS toolchain-rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain 1.85 --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu


FROM quay.io/pypa/manylinux2014 AS builder
ENV PATH="/venv/bin:/root/.cargo/bin:/opt/cross/x86_64-linux-musl-cross/bin:/opt/cross/aarch64-linux-musl-cross/bin:$PATH"

RUN /usr/local/bin/python3.13 -m venv /venv
RUN pip install maturin>=1.8 maturin[patchelf]


COPY --from=toolchain-musl /opt/cross/musl /opt/cross/musl
COPY --from=toolchain-rust /root/.cargo/ /root/.cargo/


WORKDIR /pyroscope-rs
ADD Cross.toml rustfmt.toml Cargo.toml Cargo.lock README.md ./
ADD src src
ADD pyroscope_backends pyroscope_backends
ADD pyroscope_cli pyroscope_cli
ADD pyroscope_ffi/ pyroscope_ffi/

WORKDIR /pyroscope-rs/pyroscope_ffi/python
# TODO move close to the PATH
ENV VIRTUAL_ENV=/venv

#RUN --mount=type=cache,target=/root/.cargo/registry \
#    maturin build --release --locked --compatibility=manylinux2014 --target x86_64-unknown-linux-gnu

#TODO crosscompile to arm
#
#FROM alpine:3.18 AS builder-alpine
#ENV PATH="/venv/bin:/root/.cargo/bin:$PATH"
#ENV CC_x86_64_unknown_linux_gnu=gcc
#RUN <<EOF
#set -ex
#apk add curl libgcc gcc python3 libc-dev
#curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
#  | sh -s -- -y --default-toolchain 1.85 --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu
#python3 -m venv /venv
#pip install maturin>=1.8 maturin[patchelf]
#EOF
#ENV VIRTUAL_ENV=/venv
#WORKDIR /pyroscope-rs
#ADD Cross.toml rustfmt.toml Cargo.toml Cargo.lock README.md ./
#ADD src src
#ADD pyroscope_backends pyroscope_backends
#ADD pyroscope_cli pyroscope_cli
#ADD pyroscope_ffi/ pyroscope_ffi/
#
#WORKDIR /pyroscope-rs/pyroscope_ffi/python
#RUN --mount=type=cache,target=/root/.cargo/registry \
#    maturin build --release --locked  --sdist

#FROM scratch
#COPY --from=builder /pyroscope-rs/target/wheels /dist
