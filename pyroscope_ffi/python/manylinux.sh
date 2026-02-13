#!/bin/sh
set -ex

cargo --version

# Build wheels
/opt/python/cp39-cp39/bin/python -m pip install --upgrade build
/opt/python/cp39-cp39/bin/python -m build --wheel

# Audit wheels
for wheel in dist/*-linux_*.whl; do
  auditwheel repair $wheel -w dist/
  rm $wheel
done
