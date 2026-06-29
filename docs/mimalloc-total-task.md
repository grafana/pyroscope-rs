# pyroscope-rs backend-mimalloc 总任务

> 日期：2026-06-29
> 目标版本：pyroscope-rs 2.0.6+
> 总目标：为 pyroscope crate 增加 `backend-mimalloc`，通过 `SamplingMiMalloc` 采样真实分配路径并输出 Pyroscope 可消费的 memory pprof。

## 背景结论

`jemalloc` 后端可以直接调用 `jemalloc_pprof::dump_pprof()`，但 mimalloc 没有等价能力。mimalloc 的 `mi_heap_visit_blocks`、`mi_stats_*`、`mi_process_info` 只能提供 heap/进程统计，不能恢复每次分配发生时的调用栈。

因此，主方案是：

```text
SamplingMiMalloc global allocator
  -> allocation sampling
  -> raw IP sample buffer
  -> report() 聚合和符号化
  -> memory pprof encoder
  -> ReportData::RawPprof
```

## 范围

v1 范围：

- 新增 `backend-mimalloc` feature。
- 新增 public `SamplingMiMalloc`，由用户显式设置为 `#[global_allocator]`。
- 新增 `MimallocConfig` 和 `mimalloc_backend(config)`。
- 采样 allocation events，输出 `alloc_objects/count` 和 `alloc_space/bytes`。
- 通过 `ReportData::RawPprof` 复用当前 Session 上传链路。
- 加入 example、测试、CI/验证命令。

v1 不做：

- 不做 live heap / inuse tracking。
- 不维护 pointer -> allocation metadata。
- 不默认启用 mimalloc `override`。
- 不把 `mi_heap_visit_blocks` size-class 统计伪装成 flamegraph。

## 子任务拆分

1. `docs/mimalloc-subtask-01-architecture-api.md`
   - feature、API、模块边界、all-features 兼容。
2. `docs/mimalloc-subtask-02-allocator-recorder.md`
   - `SamplingMiMalloc`、递归保护、采样策略、缓冲。
3. `docs/mimalloc-subtask-03-memory-pprof.md`
   - memory pprof encoder、sample_type、时间语义、RawPprof。
4. `docs/mimalloc-subtask-04-tests-ci-performance.md`
   - 单元测试、集成测试、性能验证、CI gate。
5. `docs/mimalloc-subtask-05-live-heap-v2.md`
   - v2 live heap/inuse 设计评估。

## 实施顺序

### Phase 1：可编译 API 骨架

- 添加 `mimalloc` optional dependency。
- 添加 `backend-mimalloc` feature。
- 添加 `src/backend/mimalloc.rs`。
- 添加 `SamplingMiMalloc` 和 `MimallocConfig`。
- 后端初始化时启用 recorder，shutdown 时关闭 recorder。
- 添加 example 和最小 backend smoke test。

验收：

```bash
cargo check --no-default-features --features backend-mimalloc
cargo test --locked --lib --tests --features backend-mimalloc
```

### Phase 2：memory pprof encoder

- 新增 `src/encode/memory_pprof.rs`。
- 输出 `alloc_objects/count`、`alloc_space/bytes`。
- encoder 自己设置 `time_nanos` 和 `duration_nanos`。
- backend `report()` 返回 `ReportData::RawPprof`.

验收：

```bash
cargo test --locked --lib --tests --features backend-mimalloc memory_pprof
```

### Phase 3：采样器增强

- 从简单 byte interval 采样升级为 weighted byte interval sampling。
- 后续再升级为 Poisson sampling。
- TLS ring buffer。
- raw IP stack capture。
- report 阶段聚合和符号化。
- recorder counters。

验收：

```bash
cargo test --locked --lib --tests --features backend-mimalloc -- --test-threads 1
```

当前进展：

- 已实现 weighted byte interval sampling：采样命中记录 `weighted_objects` 和 `weighted_bytes`。
- 已处理大对象跨多个 sample interval 时的 overshoot 和下一次剩余字节。
- 已让 TLS `remaining_bytes` 随 backend 初始化的 sampling config generation 刷新。
- 已实现固定容量全局 sample buffer。
- 已实现 allocation 命中时捕获 raw instruction pointer stack。
- 已实现 report 阶段按栈聚合和符号解析。
- 已通过 `mimalloc_stats()` 暴露 `recorded_samples`、`dropped_samples` 和当前 buffered samples。
- 待继续：Poisson sampling、TLS ring buffer、无锁/try-flush 全局队列、性能 benchmark。

### Phase 4：性能和 CI

- 基准对比 `mimalloc::MiMalloc` 和 `SamplingMiMalloc`。
- 验证 inactive overhead、默认采样 overhead、report drain latency。
- 更新 CI matrix。

验收：

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## 合入标准

- API 不破坏现有 `backend-jemalloc` 和 `backend-pprof-rs`。
- `--all-features` 不产生 global allocator 冲突。
- 文档明确 v1 是 allocation profile，不是 live heap profile。
- memory pprof sample type 不是 `cpu/nanoseconds`。
- allocator hook 不 panic，不阻塞等待，不在热路径做动态分配。
