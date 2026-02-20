#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

py_cfg_version=$(awk -F'=' '/^version[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' pyproject.toml)
rb_version=$(sed -nE "s/.*VERSION = '([^']+)'.*/\1/p" pyroscope_ffi/ruby/lib/pyroscope/version.rb)

py_rust_version=$(awk -F'=' '/^version[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' pyroscope_ffi/python/rust/Cargo.toml)
rb_rust_version=$(awk -F'=' '/^version[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' pyroscope_ffi/ruby/ext/rbspy/Cargo.toml)

if [[ -z "$py_cfg_version" || -z "$rb_version" || -z "$py_rust_version" || -z "$rb_rust_version" ]]; then
  echo "failed to read one or more version values"
  exit 1
fi

if [[ "$py_cfg_version" != "$py_rust_version" ]]; then
  echo "pyspy version mismatch: pyproject.toml=$py_cfg_version cargo=$py_rust_version"
  exit 1
fi

if [[ "$rb_version" != "$rb_rust_version" ]]; then
  echo "rbspy version mismatch: version.rb=$rb_version cargo=$rb_rust_version"
  exit 1
fi

echo "spy versions are in sync"
