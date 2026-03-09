#!/usr/bin/env bash
set -euo pipefail

# Verifies that a release tag matches the corresponding Cargo package version.
#
# Usage: ./ci/check-tag-version.sh <tag>
#
# Supported tag formats:
#   lib-X.Y.Z     → checks root Cargo.toml (pyroscope crate)
#   python-X.Y.Z  → checks pyroscope_ffi/python/rust/Cargo.toml
#   ruby-X.Y.Z    → checks pyroscope_ffi/ruby/ext/rbspy/Cargo.toml

cd "$(dirname "$0")/.."

TAG="${1:-}"
if [[ -z "$TAG" ]]; then
  echo "Usage: $0 <tag>"
  exit 1
fi

extract_cargo_version() {
  awk -F'=' '/^version[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' "$1"
}

if [[ "$TAG" =~ ^lib-(.+)$ ]]; then
  tag_version="${BASH_REMATCH[1]}"
  cargo_file="Cargo.toml"
elif [[ "$TAG" =~ ^python-(.+)$ ]]; then
  tag_version="${BASH_REMATCH[1]}"
  cargo_file="pyroscope_ffi/python/rust/Cargo.toml"
elif [[ "$TAG" =~ ^ruby-(.+)$ ]]; then
  tag_version="${BASH_REMATCH[1]}"
  cargo_file="pyroscope_ffi/ruby/ext/rbspy/Cargo.toml"
else
  echo "Unknown tag format: $TAG"
  echo "Expected one of: lib-X.Y.Z, python-X.Y.Z, ruby-X.Y.Z"
  exit 1
fi

cargo_version=$(extract_cargo_version "$cargo_file")

if [[ -z "$cargo_version" ]]; then
  echo "Failed to read version from $cargo_file"
  exit 1
fi

if [[ "$tag_version" != "$cargo_version" ]]; then
  echo "Version mismatch: tag=$tag_version, $cargo_file=$cargo_version"
  exit 1
fi

echo "OK: tag $TAG matches $cargo_file version $cargo_version"
