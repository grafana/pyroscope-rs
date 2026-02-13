#!/bin/sh
set -ex

cargo --version

# Build wheels
# todo this one is deprecated, use "build" package
/opt/python/cp39-cp39/bin/python -m pip install -r requirements.txt setuptools-rust cffi pycparser
/opt/python/cp39-cp39/bin/python setup.py bdist_wheel

# Audit wheels
for wheel in dist/*-linux_*.whl; do
  auditwheel repair $wheel -w dist/
  rm $wheel
done
