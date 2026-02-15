#!/bin/sh
set -ex

cargo --version

/opt/python/cp39-cp39/bin/python -m build --wheel


for wheel in dist/*-linux_*.whl; do
  auditwheel repair "$wheel" -w dist2/
done

sha256sum dist/*.whl dist2/*.whl > dist2.checksums