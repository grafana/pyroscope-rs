#!/bin/bash
set -ex

if [ "${BUILD_ARCH}" != "manylinux2014_x86_64" ] && [ "${BUILD_ARCH}" != "manylinux2014_aarch64" ];
then
  echo set BUILD_ARCH to one of manylinux2014_x86_64 or manylinux2014_aarch64
  exit 239
fi

BUILD_DIR="/work"
MANYLINUX_PREFIX=pyroscope/rust_builder
MANYLINUX_VERSION=4

docker run \
        -w /work/pyroscope_ffi/ruby/elflib/rbspy \
        -v `pwd`:/work \
        ${MANYLINUX_PREFIX}_${BUILD_ARCH}:${MANYLINUX_VERSION} \
        sh manylinux.sh

docker run \
        -w /work/pyroscope_ffi/ruby/elflib/thread_id \
        -v `pwd`:/work \
        ${MANYLINUX_PREFIX}_${BUILD_ARCH}:${MANYLINUX_VERSION} \
        sh manylinux.sh
