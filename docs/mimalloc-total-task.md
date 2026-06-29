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

- 从简单 byte interval 采样升级为 weighted byte-based Poisson sampling。
- TLS ring buffer。
- raw IP stack capture。
- report 阶段聚合和符号化。
- recorder counters。

验收：

```bash
cargo test --locked --lib --tests --features backend-mimalloc -- --test-threads 1
```

当前进展：

- 已实现 weighted byte-based Poisson sampling：采样命中记录 `weighted_objects` 和 `weighted_bytes`。
- 已使用每线程 `splitmix64` PRNG 生成 exponential byte interval，`sample_interval_bytes` 是平均采样间隔。
- 已处理大对象跨多个随机 sample interval 时的 overshoot 和下一次剩余字节。
- 已让 TLS `remaining_bytes` 随 backend 初始化的 sampling config generation 刷新。
- 已实现固定容量全局 sample buffer。
- 已实现固定容量 TLS sample ring，采样命中先写入线程本地 ring，满时 try-flush 到全局 buffer。
- 已实现 allocation 命中时捕获 raw instruction pointer stack。
- 已实现 report 阶段按栈聚合和符号解析。
- 已实现 `report_drain_limit`，单次 report 超出 limit 的样本会保留到下一轮。
- 已实现 report 侧 flush request generation，其它线程会在下一次 allocation 时 opportunistic flush TLS ring。
- 已通过 `mimalloc_stats()` 暴露 `recorded_samples`、`flushes`、`flushed_samples`、`dropped_samples`，并把当前线程 TLS ring 计入 buffered samples。
- 已新增 `mimalloc_baseline` 和 `mimalloc_overhead` examples，支持 baseline、inactive、active overhead 本地对比。
- 已实现线程退出时自动尝试 flush 本线程 TLS sample ring，减少短生命周期 worker 线程的样本滞留。
- 已新增 mimalloc integration 多线程 allocation churn 测试，覆盖短生命周期 worker 线程退出后的 sample handoff 和 RawPprof 输出。
- 已新增并发 allocation 期间执行 `report()` 的 integration 测试，覆盖 flush request generation 与 RawPprof 解码稳定性。
- 已新增 ignored stress test 覆盖 1/2/4/8/16/32 线程矩阵和受限 recorder 容量下的 drop-pressure。
- 已同步 README、CHANGELOG 和 `examples/mimalloc.rs` 用户说明，明确 `SamplingMiMalloc`、`backend-mimalloc`、allocation profile 语义和本地验证命令。
- 已新增 `scripts/mimalloc_benchmark_report.sh` 和 `make mimalloc/bench/report`，可生成 baseline / inactive / active 采样开销 Markdown report 与 raw key-value artifact，并支持阈值告警或 hard gate。
- 已为 `MimallocConfig`、`SamplingMiMalloc` 和 `mimalloc_backend()` 补充 docs.rs API 示例，明确 allocation profile 语义、global allocator 要求和非 live heap 边界。
- 已在 GitHub Actions 增加 `mimalloc benchmark report` job，生成并上传 `mimalloc-benchmark-report` artifact。
- benchmark report 已覆盖 encoded pprof size，便于观察聚合和符号化输出规模。
- benchmark report 已覆盖 pprof encode duration 和抽样 p50/p95/p99 allocation latency。
- README 已补充 `make mimalloc/bench/report`、CI artifact 名称和 artifact 指标说明。
- 已将全局 sample buffer 从单个 `Mutex<Vec<RecordedAllocationSample>>` 改为原子总容量门控 + 8 个分片 `Mutex<Vec<_>>`，allocator hot path 仍只使用 `try_lock`，降低高并发 TLS flush 竞争。
- 已实现跨线程注册表驱动的主动同步 flush：每个采样线程注册自己的 TLS ring，`report()` 遍历活跃 ring 并主动 flush；线程 ring 正忙时跳过并保留 generation opportunistic flush 作为补偿路径。
- 待继续：benchmark 历史趋势归档。

当前剩余未实现功能：

1. 跨线程注册表驱动的主动同步 flush：已实现。`report()` 会遍历活跃线程 TLS ring 并主动 flush；如果目标线程正在写 ring，当前 report 跳过该 ring，后续 report、下一次 allocation generation flush 或线程退出 flush 会继续兜底。
2. 无锁或低锁竞争全局 sample queue：已实现低锁分片全局 sample buffer。当前仍不是完全 lock-free；高并发下单 shard `try_lock` 失败仍会 drop 当前 TLS ring，但相比单 mutex 已显著降低全局竞争面。
3. CI benchmark 报告归档：已有 `mimalloc_baseline` / `mimalloc_overhead` examples，已形成可重复的本地/CI Markdown artifact 和阈值对比，并已接入 GitHub Actions artifact 上传；尚未做历史趋势归档。
4. 更完整的多线程压力测试：已有集成 smoke、TLS flush 单测、短生命周期 worker allocation churn、并发 allocation/report 测试和 ignored 线程矩阵/drop-pressure stress test；已沉淀为 CI artifact 的核心指标，待继续扩展历史趋势归档。
5. 发布文档同步：README / CHANGELOG / example 注释和 docs.rs API 示例已覆盖可复制使用说明，并已补充 CI benchmark artifact 说明。
6. v2 live heap / inuse profile：仍保持默认不做，需单独评估 pointer tracking、dealloc/realloc metadata 成本和 opt-in API。

### Phase 4：性能和 CI

- 基准对比 `mimalloc::MiMalloc` 和 `SamplingMiMalloc`。
- 验证 inactive overhead、默认采样 overhead、report drain latency。
- 生成可归档 benchmark report artifact。
- 更新 CI matrix 并上传 artifact。
- 沉淀历史趋势归档。

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
