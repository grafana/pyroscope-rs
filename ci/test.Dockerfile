ARG BASE_IMAGE=rust:trixie@sha256:a9cfb755b33f5bb872610cbdb25da61f527416b28fc9c052bbce4bef93e7799a
FROM ${BASE_IMAGE}

# Install make - needed by tikv-jemalloc-sys build script.
# Alpine doesn't include it; Debian-based images already have it.
RUN if command -v apk > /dev/null; then apk add --no-cache make; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
RUN cargo test --locked --lib --tests --features backend-pprof-rs
RUN cargo test --locked --lib --tests --features backend-jemalloc
