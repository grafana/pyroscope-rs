#!/bin/sh
set -e

# Install tooling
yum -y -q install wget gcc libffi-devel openssl-devel

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain=1.54.0 -y
export PATH=~/.cargo/bin:$PATH

# Install libunwind
wget https://github.com/libunwind/libunwind/releases/download/v1.6.2/libunwind-1.6.2.tar.gz
tar -zxvf libunwind-1.6.2.tar.gz
cd libunwind-1.6.2
./configure --disable-minidebuginfo --enable-ptrace --disable-tests --disable-documentation
make
make install
cd ..

# Build wheels
/opt/python/cp37-cp37m/bin/python setup.py bdist_wheel

# Audit wheels
for wheel in dist/*-linux_*.whl; do
  auditwheel repair $wheel -w dist/
  rm $wheel
done
