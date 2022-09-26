#!/bin/sh
set -e

# Install tooling
yum -y -q install wget gcc libffi-devel openssl-devel

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.63.0 -y
export PATH=~/.cargo/bin:$PATH

# Build wheels
/opt/python/cp37-cp37m/bin/python setup.py bdist_wheel

# Audit wheels
for wheel in dist/*.whl; do
  auditwheel repair $wheel -w dist/
  rm $wheel
done

# Extract wheels
for wheel in dist/*.whl; do
    /opt/python/cp37-cp37m/bin/wheel unpack $wheel -d wheelhouse
done
