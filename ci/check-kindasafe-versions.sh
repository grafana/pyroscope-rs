#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

kindasafe_version=$(awk -F'=' '/^version[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' kit/kindasafe/Cargo.toml)
kindasafe_init_version=$(awk -F'=' '/^version[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' kit/kindasafe_init/Cargo.toml)

if [[ -z "$kindasafe_version" || -z "$kindasafe_init_version" ]]; then
  echo "failed to read one or more version values"
  exit 1
fi

if [[ "$kindasafe_version" != "$kindasafe_init_version" ]]; then
  echo "kindasafe version mismatch: kindasafe=$kindasafe_version kindasafe_init=$kindasafe_init_version"
  exit 1
fi

echo "kindasafe versions are in sync: $kindasafe_version"
