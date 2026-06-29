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

目标：

- inactive overhead < 2%。
- 默认 1 MiB sampling overhead < 5%。
- 4 KiB 只作为压力诊断，不作为默认推荐。

指标：

- throughput。
- p50/p95/p99 allocation latency。
- sampled count。
- dropped count。
- report drain duration。
- pprof encode duration。
- encoded pprof size。

