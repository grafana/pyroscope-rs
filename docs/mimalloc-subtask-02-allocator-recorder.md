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
    record sample
    remaining_bytes += sample_interval_bytes
```

后续升级为 Poisson sampling：

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
- 已在采样命中时捕获 raw instruction pointer stack。
- 已使用 `try_lock` 避免 allocator hook 阻塞等待。
- 已在 `report()` 阶段 drain、聚合和符号解析。
- 待继续：TLS fixed ring、Poisson sampling、跨线程 flush 和 drop counters 暴露。

## realloc 规则

- 只有返回非 null 时才记录新 allocation event。
- 失败返回 null 时旧指针仍有效，不做任何 recorder 状态变更。
- v1 不维护 pointer map，因此不需要删除旧 pointer metadata。

## 验收

- `SamplingMiMalloc` 可作为 global allocator。
- inactive 时 allocation 正常。
- active 时 allocation 不 panic。
- `realloc` 失败路径不破坏旧指针。
- 后续压力测试覆盖多线程 allocation churn。
