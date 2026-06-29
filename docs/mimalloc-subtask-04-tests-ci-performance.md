# 子任务 04：测试、CI 与性能验证

## 目标

保证 `backend-mimalloc` 在 feature 组合、allocator 使用、pprof 语义和性能开销上可维护。

## 单元测试

### memory pprof encoder

- 空 profile 合法。
- 非空 sample 编码正确。
- sample_type 是 memory 语义。
- `duration_nanos` 正确写入。
- `period` 等于 `sample_interval_bytes`。

### MimallocConfig

- 默认值合理。
- `sample_interval_bytes = 0` 时初始化应失败或归一化为 1。
- 大字段不在热路径 clone。

## 集成测试

新增 `tests/mimalloc_backend.rs`：

- 声明 `SamplingMiMalloc` 为该 test binary 的 global allocator。
- 构建 `mimalloc_backend(MimallocConfig::default())`。
- 执行 allocation loop。
- 调 `report()`。
- 断言返回 `profile_type = "memory"` 和 `RawPprof`。

## all-features 验证

必须验证：

```bash
cargo check --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

注意：

- lib crate 内不能定义 global allocator。
- example/test 自己定义 global allocator 是独立 binary，不影响 lib。
- 如果 jemalloc example 和 mimalloc example 同时编译，必须通过 `required-features` 隔离。

## pre-commit gate

仓库要求提交前运行：

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

功能专项 gate：

```bash
cargo check --no-default-features --features backend-mimalloc
cargo test --locked --lib --tests --features backend-mimalloc
cargo test --locked --lib --tests --features backend-jemalloc
cargo test --locked --lib --tests --features backend-pprof-rs -- --test-threads 1
cargo test --locked --lib --tests --all-features -- --test-threads 1
```

## 性能验证

对比对象：

```text
mimalloc::MiMalloc baseline
SamplingMiMalloc inactive
SamplingMiMalloc active, interval = 1 MiB
SamplingMiMalloc active, interval = 512 KiB
SamplingMiMalloc active, interval = 4 KiB
```

当前已新增本地 benchmark examples：

```bash
cargo run --release --example mimalloc_baseline --features backend-mimalloc
cargo run --release --example mimalloc_overhead --features backend-mimalloc
MIMALLOC_BENCH_MODE=active cargo run --release --example mimalloc_overhead --features backend-mimalloc
MIMALLOC_BENCH_MODE=active MIMALLOC_BENCH_SAMPLE_INTERVAL=4096 cargo run --release --example mimalloc_overhead --features backend-mimalloc
```

也可生成可归档的 benchmark report artifact：

```bash
make mimalloc/bench/report
```

默认产物：

```text
target/mimalloc-benchmark/mimalloc-benchmark-report.md
target/mimalloc-benchmark/baseline.env
target/mimalloc-benchmark/inactive.env
target/mimalloc-benchmark/active-1m.env
target/mimalloc-benchmark/active-512k.env
target/mimalloc-benchmark/active-4k.env
```

CI 推荐配置：

```bash
MIMALLOC_BENCH_DURATION_MS=1000 make mimalloc/bench/report
```

如果要把阈值从报告告警升级为 CI hard gate：

```bash
MIMALLOC_BENCH_ENFORCE_THRESHOLDS=1 make mimalloc/bench/report
```

阈值环境变量：

```text
MIMALLOC_BENCH_INACTIVE_MAX_OVERHEAD_PCT
MIMALLOC_BENCH_ACTIVE_1M_MAX_OVERHEAD_PCT
```

可调环境变量：

```text
MIMALLOC_BENCH_DURATION_MS
MIMALLOC_BENCH_BATCH_SIZE
MIMALLOC_BENCH_MIN_SIZE
MIMALLOC_BENCH_MAX_SIZE
MIMALLOC_BENCH_SIZE_STEP
MIMALLOC_BENCH_LATENCY_SAMPLE_INTERVAL
MIMALLOC_BENCH_LATENCY_SAMPLE_LIMIT
MIMALLOC_BENCH_SAMPLE_INTERVAL
MIMALLOC_BENCH_RING_CAPACITY
MIMALLOC_BENCH_REPORT_DRAIN_LIMIT
```

目标：

- inactive overhead < 2%。
- 默认 1 MiB sampling overhead < 5%。
- 4 KiB 只作为压力诊断，不作为默认推荐。

指标：

- throughput。
- p50/p95/p99 allocation latency。
- `mimalloc_stats().recorded_samples`。
- `mimalloc_stats().flushes`。
- `mimalloc_stats().flushed_samples`。
- `mimalloc_stats().dropped_samples`。
- report drain duration。
- pprof encode duration。
- encoded pprof size。

当前 report artifact 已覆盖：

- baseline / inactive / active 1 MiB / active 512 KiB / active 4 KiB。
- throughput。
- allocations/sec。
- inactive 与 active 1 MiB 阈值对比。
- `recorded_samples`。
- `flushes`。
- `dropped_samples`。
- report drain duration。
- encoded pprof size。
- pprof encode duration。
- p50/p95/p99 allocation latency。

GitHub Actions 已通过 `mimalloc benchmark report` job 上传
`mimalloc-benchmark-report` artifact，保留 14 天。

仍待补齐：

- 历史趋势归档。
