#!/bin/bash
set -ex

BUILD_DIR="/work"

docker run \
        -w /work/pyroscope_ffi/python \
        -v `pwd`:/work \
        quay.io/pypa/${BUILD_ARCH} \
        sh manylinux.sh
