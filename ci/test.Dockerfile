ARG BASE_IMAGE=rust:trixie@sha256:a9cfb755b33f5bb872610cbdb25da61f527416b28fc9c052bbce4bef93e7799a
FROM ${BASE_IMAGE}

RUN if command -v apk > /dev/null; then apk add --no-cache make musl-dev; fi

WORKDIR /src
COPY . .

RUN cargo test --locked --lib --tests
# Single thread required for global allocator test
RUN cargo test --locked --lib --tests --features backend-pprof-rs -- --test-threads 1
RUN cargo test --locked --lib --tests --features backend-jemalloc
