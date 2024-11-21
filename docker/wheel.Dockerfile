ARG PLATFORM=x86_64
FROM quay.io/pypa/manylinux2014_${PLATFORM} AS builder

RUN curl https://static.rust-lang.org/rustup/dist/$(arch)-unknown-linux-musl/rustup-init -o ./rustup-init \
    && chmod +x ./rustup-init \
    && ./rustup-init  -y --default-toolchain=stable --default-host=$(arch)-unknown-linux-gnu
ENV PATH=/root/.cargo/bin:$PATH
RUN yum -y install gcc libffi-devel openssl-devel wget gcc-c++ glibc-devel make

# for python
ENV LIBUNWIND_VERSION=1.8.1
RUN wget https://github.com/libunwind/libunwind/releases/download/v${LIBUNWIND_VERSION}/libunwind-${LIBUNWIND_VERSION}.tar.gz \
    && tar -zxvf libunwind-${LIBUNWIND_VERSION}.tar.gz \
    && cd libunwind-${LIBUNWIND_VERSION} \
    && ./configure --disable-minidebuginfo --enable-ptrace --disable-tests --disable-documentation \
    && make \
    && make install

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