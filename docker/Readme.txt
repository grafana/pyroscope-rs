These dockerfile images do few things:

1. rustup now requires glibc >= 2.17 but manylinux2010 has glibc == 2.12

To make it work we download statically compiled (musl) rustup-init and then install 1.63 x86_64-unknown-linux-gnu toolchain

https://github.com/pyroscope-io/pyroscope-rs/pull/82#issuecomment-1434466795
https://blog.rust-lang.org/2022/08/01/Increasing-glibc-kernel-requirements.html

2. rust toolchain, libunwind, deps were downloaded and installed on every build in manylinux.sh
now they are installed once at image creation time
