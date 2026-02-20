# syntax=docker/dockerfile:1.4
# Build wheel natively on Alpine Linux (musl libc).
# We cannot use CARGO_BUILD_TARGET=x86_64-unknown-linux-musl from a glibc host
# because that target does not support cdylib. Instead we run the entire build
# inside an Alpine container where the native toolchain already targets musl.
FROM python:3.9-alpine AS builder

RUN apk add --no-cache \
    gcc \
    musl-dev \
    libffi-dev \
    make \
    curl \
    git

ENV RUST_VERSION=1.87
RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --default-toolchain=${RUST_VERSION} \
    && rm /tmp/rustup-init
ENV PATH=/root/.cargo/bin:$PATH

# Linker wrapper: Rust emits -lgcc_s when crt-static is off (musl cdylib),
# causing a libgcc_s.so.1 NEEDED entry in the final .so. This wrapper removes
# -lgcc_s from the link command and substitutes the static libgcc_eh.a instead,
# so users need no runtime libgcc package.
RUN LIBGCC_EH=$(find /usr/lib/gcc -name libgcc_eh.a | head -1) && \
    { \
      echo '#!/usr/local/bin/python3'; \
      echo 'import sys, os'; \
      echo 'args = [a for a in sys.argv[1:] if a != "-lgcc_s"]'; \
      echo "args += ['-Wl,--push-state,--whole-archive,${LIBGCC_EH},--no-whole-archive,--pop-state']"; \
      echo "os.execvp('cc', ['cc'] + args)"; \
    } > /usr/local/bin/cc-no-gcc-s && \
    chmod +x /usr/local/bin/cc-no-gcc-s

WORKDIR /pyroscope-rs

RUN python -m pip install build

ADD pyproject.toml \
    rustfmt.toml \
    Cargo.toml \
    Cargo.lock \
    ./

ADD src src
ADD pyroscope_ffi/ pyroscope_ffi/

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    RUSTFLAGS="-C target-feature=-crt-static -C linker=/usr/local/bin/cc-no-gcc-s" \
    python -m build --wheel

FROM scratch
COPY --from=builder /pyroscope-rs/dist dist/
