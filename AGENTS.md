# pyroscope-rs

# Pre-Commit Checks

Before committing, always run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

All must pass with no errors before creating a commit.

# Documentation

User-facing documentation belongs in the `grafana/pyroscope` repository:
https://github.com/grafana/pyroscope

Keep this README focused on the project overview and links. Add detailed setup,
backend usage, platform limits, and troubleshooting guides to that repository's
documentation. Maintain Rust API documentation and compilable examples here.
