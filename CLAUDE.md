# pyroscope-rs

# Pre-Commit Checks

Before committing, always run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

All must pass with no errors before creating a commit.
