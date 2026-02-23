#!/usr/bin/env bash
set -euo pipefail

lang="${1:-}"
bump_part="${BUMP:-fix}"

if [[ -z "$lang" ]]; then
  echo "Usage: BUMP=<major|minor|fix> $0 <ruby|python>" >&2
  exit 1
fi

bump_semver() {
  local current="$1"
  local major minor patch

  major="$(echo "$current" | cut -d. -f1)"
  minor="$(echo "$current" | cut -d. -f2)"
  patch="$(echo "$current" | cut -d. -f3)"

  case "$bump_part" in
    major)
      major=$((major + 1))
      minor=0
      patch=0
      ;;
    minor)
      minor=$((minor + 1))
      patch=0
      ;;
    fix)
      patch=$((patch + 1))
      ;;
    *)
      echo "Invalid bump type '$bump_part'. Use major, minor, or fix." >&2
      exit 1
      ;;
  esac

  echo "$major.$minor.$patch"
}

case "$lang" in
  ruby)
    ruby_current="$(sed -n "s/.*VERSION = '\([0-9]*\.[0-9]*\.[0-9]*\)'.*/\1/p" pyroscope_ffi/ruby/lib/pyroscope/version.rb)"
    ruby_new="$(bump_semver "$ruby_current")"
    sed -i -E "s/(VERSION = ')[0-9]+\.[0-9]+\.[0-9]+('\\.freeze)/\1$ruby_new\2/" pyroscope_ffi/ruby/lib/pyroscope/version.rb
    sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/s//version = \"$ruby_new\"/" pyroscope_ffi/ruby/ext/rbspy/Cargo.toml
    cargo update --package ffiruby
    echo "Ruby versions bumped: gem/rust cargo $ruby_current -> $ruby_new"
    ;;
  python)
    python_current="$(sed -n 's/^version = "\([0-9]*\.[0-9]*\.[0-9]*\)"/\1/p' pyproject.toml)"
    python_new="$(bump_semver "$python_current")"
    sed -i -E "s/^(version = \")[0-9]+\.[0-9]+\.[0-9]+(\")/\1$python_new\2/" pyproject.toml
    sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/s//version = \"$python_new\"/" pyroscope_ffi/python/rust/Cargo.toml
    cargo update --package pyroscope_python_extension
    echo "Python versions bumped: package/rust cargo $python_current -> $python_new"
    ;;
  *)
    echo "Invalid language '$lang'. Use ruby or python." >&2
    exit 1
    ;;
esac
