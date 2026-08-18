# pyroscope-rs

# Pre-Commit Checks

Before committing, always run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

All must pass with no errors before creating a commit.
- Security issues should be reported via [Grafana's security issue reporting page](https://grafana.com/legal/report-a-security-issue/) and not directly in this repository.
