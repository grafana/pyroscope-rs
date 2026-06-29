# 子任务 01：架构与 API 骨架

## 目标

建立 `backend-mimalloc` 的最小公共 API 和模块边界，让后续 recorder、pprof encoder、测试和性能优化可以独立推进。

## 代码改动

### Cargo.toml

新增 optional dependency：

```toml
mimalloc = { version = "0.1.52", optional = true }
```

新增 feature：

```toml
backend-mimalloc = ["dep:mimalloc"]
```

原则：

- 不启用 `libmimalloc-sys/override`。
- 不在 crate 内定义全局 allocator。
- `backend-mimalloc` 必须能和 `backend-jemalloc`、`backend-pprof-rs` 同时编译。

### src/backend/mod.rs

新增：

```rust
#[cfg(feature = "backend-mimalloc")]
pub mod mimalloc;

#[cfg(feature = "backend-mimalloc")]
pub use mimalloc::*;
```

### src/backend/mimalloc.rs

新增 public API：

```rust
pub struct MimallocConfig {
    pub sample_interval_bytes: u64,
    pub max_depth: usize,
    pub ring_capacity: usize,
    pub report_drain_limit: usize,
}

pub struct SamplingMiMalloc { ... }

pub fn mimalloc_backend(config: MimallocConfig) -> BackendImpl<BackendUninitialized>;
```

`SamplingMiMalloc` 必须 public，因为最终 binary 才能声明：

```rust
#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();
```

## Backend 语义

`initialize()`：

- 校验配置。
- 设置全局 recorder active。
- 重置初始 counters。

`report()`：

- drain 当前 recorder 数据。
- 调 memory pprof encoder。
- 返回：

```rust
ReportBatch {
    profile_type: "memory".into(),
    data: ReportData::RawPprof(bytes),
}
```

`shutdown()`：

- 设置 recorder inactive。

`add_tag()`/`remove_tag()`：

- v1 no-op。

## 验收

```bash
cargo check --no-default-features --features backend-mimalloc
cargo test --locked --lib --tests --features backend-mimalloc
```

## 风险

- 如果 crate 内定义 `#[global_allocator]`，`--all-features` 会非常容易冲突。
- 如果 backend 使用普通 `mimalloc::MiMalloc` 而不是 `SamplingMiMalloc`，无法采样调用栈。
- 如果初始化时不检查 recorder，用户误配后会以为 profiling 生效。

