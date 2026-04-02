# pyroscope-rs

# Build & Packaging

When adding new workspace crates or source directories needed for Rust compilation, update ALL of these:
- `MANIFEST.in` — include Cargo.toml and source files so Python sdist contains them
- `docker/wheel.Dockerfile` — ADD the directory for Python manylinux wheel builds
- `docker/wheel-musllinux.Dockerfile` — ADD the directory for Python musllinux wheel builds
- `docker/gem.Dockerfile` — ADD the directory for Ruby gem builds

All three Dockerfiles and the MANIFEST.in must stay in sync with workspace members in the root `Cargo.toml`.

# Pre-Commit Checks

Before committing, always run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
for manifest in kit/*/Cargo.toml; do
    cargo test --manifest-path "$manifest"
done
```

All must pass with no errors before creating a commit.
