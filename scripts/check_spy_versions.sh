#!/usr/bin/env bash
set -euo pipefail

python_cfg_version="$(sed -n 's/^version= *//p' pyroscope_ffi/python/setup.cfg | tr -d '[:space:]')"
ruby_version="$(sed -n "s/.*VERSION = '\([^']*\)'.*/\1/p" pyroscope_ffi/ruby/lib/pyroscope/version.rb)"
python_rust_name="$(sed -n 's/^const PYTHON_SPY_NAME: &str = "\(.*\)";/\1/p' pyroscope_ffi/python/lib/src/lib.rs)"
python_rust_version="$(sed -n 's/^const PYTHON_SPY_VERSION: &str = "\(.*\)";/\1/p' pyroscope_ffi/python/lib/src/lib.rs)"
ruby_rust_name="$(sed -n 's/^const RUBY_SPY_NAME: &str = "\(.*\)";/\1/p' pyroscope_ffi/ruby/ext/rbspy/src/lib.rs)"
ruby_rust_version="$(sed -n 's/^const RUBY_SPY_VERSION: &str = "\(.*\)";/\1/p' pyroscope_ffi/ruby/ext/rbspy/src/lib.rs)"

if [[ -z "$python_cfg_version" || -z "$ruby_version" || -z "$python_rust_name" || -z "$python_rust_version" || -z "$ruby_rust_name" || -z "$ruby_rust_version" ]]; then
  echo "failed to extract one or more version values"
  exit 1
fi

if [[ "$python_rust_name" != "pyspy" ]]; then
  echo "Python spy name mismatch: expected pyspy rust=$python_rust_name"
  exit 1
fi

if [[ "$ruby_rust_name" != "rbspy" ]]; then
  echo "Ruby spy name mismatch: expected rbspy rust=$ruby_rust_name"
  exit 1
fi

if [[ "$python_cfg_version" != "$python_rust_version" ]]; then
  echo "Python spy version mismatch: setup.cfg=$python_cfg_version rust=$python_rust_version"
  exit 1
fi

if [[ "$ruby_version" != "$ruby_rust_version" ]]; then
  echo "Ruby spy version mismatch: version.rb=$ruby_version rust=$ruby_rust_version"
  exit 1
fi

echo "Spy versions are in sync."
