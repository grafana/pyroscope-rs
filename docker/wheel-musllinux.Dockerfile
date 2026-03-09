# syntax=docker/dockerfile:1.22@sha256:4a43a54dd1fedceb30ba47e76cfcf2b47304f4161c0caeac2db1c61804ea3c91
ARG PLATFORM=x86_64
FROM quay.io/pypa/musllinux_1_2_${PLATFORM} AS builder
ARG OPENSSL_VERSION=3.5.5

RUN apk add --no-cache gcc musl-dev libffi-dev make perl linux-headers

# Build OpenSSL from source
RUN curl -fsSL "https://github.com/openssl/openssl/releases/download/openssl-${OPENSSL_VERSION}/openssl-${OPENSSL_VERSION}.tar.gz" \
    -o /tmp/openssl.tar.gz \
    && tar xzf /tmp/openssl.tar.gz -C /tmp \
    && cd /tmp/openssl-${OPENSSL_VERSION} \
    && ./config no-shared no-tests --prefix=/usr/local/openssl \
    && make -j$(nproc) \
    && make install_sw \
    && ln -sf /usr/local/openssl/lib64 /usr/local/openssl/lib || true \
    && cd / \
    && rm -rf /tmp/openssl*

ENV OPENSSL_DIR=/usr/local/openssl
ENV OPENSSL_STATIC=1

RUN adduser -D builder \
    && mkdir -p /pyroscope-rs \
    && chown builder:builder /pyroscope-rs

USER builder
RUN test "$(id -u)" = "1000" || { echo "ERROR: builder uid is $(id -u), expected 1000"; exit 1; }

ENV RUST_VERSION=1.88
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --default-toolchain=${RUST_VERSION} --default-host=$(arch)-unknown-linux-musl \
    && rm /tmp/rustup-init
ENV PATH=/home/builder/.cargo/bin:$PATH

WORKDIR /pyroscope-rs

RUN /opt/python/cp310-cp310/bin/python -m pip install --user build

ADD --chown=builder:builder pyproject.toml \
    setup.py \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD --chown=builder:builder src src
ADD --chown=builder:builder kit/ kit/
ADD --chown=builder:builder pyroscope_ffi/ pyroscope_ffi/

ARG PYROSCOPE_CARGO_NO_DEFAULT_FEATURES=1
ARG PYROSCOPE_CARGO_FEATURES=native-tls
ENV PYROSCOPE_CARGO_NO_DEFAULT_FEATURES=${PYROSCOPE_CARGO_NO_DEFAULT_FEATURES}
ENV PYROSCOPE_CARGO_FEATURES=${PYROSCOPE_CARGO_FEATURES}

RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,gid=1000 \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,gid=1000 \
    /opt/python/cp310-cp310/bin/python -m build --wheel

RUN auditwheel repair dist/*.whl --wheel-dir dist-repaired/

FROM scratch
COPY --from=builder  /pyroscope-rs/dist-repaired dist/
