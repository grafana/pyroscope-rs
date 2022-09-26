#!/bin/bash
set -ex

BUILD_DIR="/work"

docker run \
        -w /work/pyroscope_ffi/ruby/elflib/rbspy \
        -v `pwd`:/work \
        quay.io/pypa/${BUILD_ARCH} \
        sh manylinux.sh

docker run \
        -w /work/pyroscope_ffi/ruby/elflib/thread_id \
        -v `pwd`:/work \
        quay.io/pypa/${BUILD_ARCH} \
        sh manylinux.sh
