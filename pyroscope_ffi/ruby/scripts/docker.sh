#!/bin/bash
set -ex

BUILD_DIR="/work"
MANYLINUX_PREFIX=pyroscope/rust_builder
docker run --rm -ti \
        -w /work/pyroscope_ffi/ruby/elflib/rbspy \
        -v `pwd`:/work \
        ${MANYLINUX_PREFIX}_${BUILD_ARCH} \
        sh manylinux.sh

docker run --rm -ti \
        -w /work/pyroscope_ffi/ruby/elflib/thread_id \
        -v `pwd`:/work \
        ${MANYLINUX_PREFIX}_${BUILD_ARCH} \
        sh manylinux.sh
