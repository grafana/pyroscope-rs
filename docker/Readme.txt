These dockerfile.manylinux* images do few things:

1. rust toolchain, libunwind, deps were downloaded and installed on every build in manylinux.sh
now they are installed once at image creation time
