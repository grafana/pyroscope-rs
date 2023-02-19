#!/bin/bash
set -ex

if [ "${BUILD_ARCH}" != "manylinux2010_x86_64" ] && [ "${BUILD_ARCH}" != "manylinux2014_aarch64" ];
then
  echo set BUILD_ARCH to one of manylinux2010_x86_64 or manylinux2014_aarch64
  exit 239
fi

BUILD_DIR="/work"
MANYLINUX_PREFIX=pyroscope/rust_builder

docker run --rm -ti \
        -w /work/pyroscope_ffi/python \
        -v `pwd`:/work \
        ${MANYLINUX_PREFIX}_${BUILD_ARCH} \
        sh manylinux.sh
