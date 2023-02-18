#!/bin/sh
set -ex

cargo --version

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
