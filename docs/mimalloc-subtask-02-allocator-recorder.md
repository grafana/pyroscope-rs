# 子任务 02：SamplingMiMalloc 与 recorder

## 目标

实现低开销 allocator wrapper，在 allocation 成功时采样事件，并保证 allocator 热路径不 panic、不阻塞、不递归分配。

## v1 能力

- `alloc()` 成功后采样 `layout.size()`。
- `alloc_zeroed()` 成功后采样 `layout.size()`。
- `realloc()` 成功后采样 `new_size`。
- `dealloc()` v1 不记录。
- 输出 allocation profile，不输出 live heap profile。

## 热路径规则

allocator hook 内禁止：

- `log!`
- `format!`
- `String`
- `Vec` 动态扩容
- `HashMap`
- blocking lock
- panic / unwrap / expect

所有错误都只能：

- drop sample
- 增加 atomic dropped counter
- 继续返回真实 allocator 结果

## 递归保护

使用 TLS guard：

```rust
thread_local! {
    static IN_ALLOC_PROFILER: Cell<bool> = Cell::new(false);
}
```

流程：

```text
if guard is set:
    skip profiling
set guard
run recorder slow path
clear guard
```

## 采样策略

第一阶段可使用 byte interval sampling：

```text
remaining_bytes -= allocation_size
if remaining_bytes <= 0:
    crossed = 1 + overshoot / sample_interval_bytes
    record weighted sample
    remaining_bytes = sample_interval_bytes - overshoot % sample_interval_bytes
```

当前实现已经把第一阶段推进到 weighted byte-based Poisson sampling：

- 小对象命中采样点时，`weighted_bytes` 至少等于 `sample_interval_bytes`，避免只按当前 allocation size 累加导致系统性低估。
- 每个线程持有独立 `splitmix64` PRNG state，采样命中后通过 `-ln(random) * sample_interval_bytes` 抽取下一次随机 byte interval。
- 大对象跨多个随机采样周期时，`weighted_bytes` 覆盖跨过的 interval，并把 overshoot 结转到下一次 `remaining_bytes`。
- `weighted_objects` 按 `weighted_bytes / allocation_size` 做整数估算，最小为 1。
- TLS `remaining_bytes` 通过 config generation 感知 backend 重新初始化后的采样周期变化。

Poisson interval：

```text
next = -ln(random) * sample_interval_bytes
```

默认：

```text
sample_interval_bytes = 1 MiB
```

## 缓冲策略

最终目标：

```text
TLS fixed ring
  -> try flush
  -> global queue
  -> report drain
```

第一阶段可以先使用 atomic counters 建立功能链路，但必须在文档和代码注释中明确：这只是 API/encoder 骨架，不是最终 flamegraph recorder。

当前实现进展：

- 已从 atomic counters 推进到固定容量全局 sample buffer。
- 已实现 weighted byte-based Poisson sampling。
- 已实现固定容量 TLS sample ring；allocator hook 先写入 TLS ring，ring 满时 try-flush 到全局 buffer。
- 已在采样命中时捕获 raw instruction pointer stack。
- 已使用 `try_lock` 避免 allocator hook 阻塞等待。
- 已在 `report()` 阶段 drain、聚合和符号解析。
- 已兑现 `report_drain_limit`，避免单次 report 无上限 drain 全部样本。
- 已实现 flush request generation；其它线程在下一次 allocation 时 opportunistic flush 本线程 TLS ring。
- 已实现线程退出时自动尝试 flush 本线程 TLS ring，减少短生命周期线程退出后样本不可见的问题。
- 已通过 `mimalloc_stats()` 暴露 recorded、flushes、flushed、dropped 和包含当前线程 TLS ring 的 buffered recorder counters。
- 待继续：跨线程注册表驱动的主动同步 flush、无锁全局队列和 CI benchmark 报告归档。

## realloc 规则

- 只有返回非 null 时才记录新 allocation event。
- 失败返回 null 时旧指针仍有效，不做任何 recorder 状态变更。
- v1 不维护 pointer map，因此不需要删除旧 pointer metadata。

## 验收

- `SamplingMiMalloc` 可作为 global allocator。
- inactive 时 allocation 正常。
- active 时 allocation 不 panic。
- `realloc` 失败路径不破坏旧指针。
- 已通过 integration test 覆盖多线程短生命周期 worker allocation churn 和线程退出 handoff。
- 后续压力测试继续覆盖 report 并发 drain、更多线程矩阵和高压 drop-rate。
