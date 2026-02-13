#!/bin/sh
set -ex

cargo --version

# Build wheels
/opt/python/cp39-cp39/bin/python setup.py bdist_wheel

echo "Wheel sha256 before auditwheel repair:"
sha256sum dist/*.whl

# Audit wheels
for wheel in dist/*-linux_*.whl; do
  auditwheel repair "$wheel" -w dist/
  rm "$wheel"
done

echo "Wheel sha256 after auditwheel repair:"
sha256sum dist/*.whl
