# 子任务 05：v2 live heap / inuse profile 评估

## 目标

评估是否在 v1 allocation profile 之后增加 live heap tracking，以输出 `inuse_objects` 和 `inuse_space`。

## 评估结论

**结论：v2 live heap / inuse profile 不进入 `backend-mimalloc` v1，必须作为后续 opt-in 独立能力评估和实现。**

原因：

- v1 allocation profile 已经能提供真实分配调用栈，并且不会在 `dealloc` 热路径维护 metadata。
- live heap 需要 `ptr -> metadata` 生命周期跟踪，会把 allocator wrapper 从“采样记录器”升级为“分配状态机”。
- `dealloc`、`realloc`、跨线程释放和线程退出都会进入正确性边界，风险显著高于当前 allocation profile。
- 如果默认启用，会改变当前 v1 对性能、内存占用和锁竞争的承诺。

因此，v2 只能通过显式配置或独立 feature 开启，例如未来的：

```rust
MimallocConfig {
    live_heap_tracking: true,
    ..MimallocConfig::default()
}
```

当前 v1 文档和 API 必须继续明确：输出 `alloc_objects` / `alloc_space`，不承诺 `inuse_*`。

## v2 能力

新增：

```text
inuse_objects / count
inuse_space   / bytes
```

完整 sample_type：

```text
alloc_objects / count
alloc_space   / bytes
inuse_objects / count
inuse_space   / bytes
```

默认展示：

```text
default_sample_type = inuse_space
```

## 必要数据结构

需要 pointer tracking：

```text
ptr -> {
  requested_size,
  usable_size,
  sampled_stack_id,
  weight_objects,
  weight_bytes,
}
```

推荐 v2 只跟踪“被采样命中的 allocation pointer”，而不是所有 pointer。这样可以把 metadata 数量控制在采样规模内，并保持与 allocation profile 的 weighted sampling 语义一致。

推荐 metadata：

```text
ptr -> LiveAllocationMetadata {
  requested_size,
  sampled_stack,
  weighted_objects,
  weighted_bytes,
  allocation_time_nanos,
}
```

暂不建议依赖 mimalloc usable size 作为默认语义。v2 初版应优先使用 Rust `Layout` / `realloc new_size` 的 requested size，避免 C allocator usable size 与 Rust allocation API 之间出现解释偏差。

## 热路径成本

v2 会显著增加：

- alloc 成功后的 map insert。
- dealloc 的 map remove。
- realloc 成功/失败的状态转移。
- pointer metadata 内存占用。
- 锁竞争或 sharded map 复杂度。

因此 v2 必须是 opt-in，不能替换 v1 默认路径。

## 推荐架构

推荐采用：

```text
SamplingMiMalloc
  -> allocation sampling hit
  -> insert sampled ptr metadata into sharded live map
  -> dealloc/realloc remove or update sampled ptr metadata
  -> report snapshot sharded live map
  -> aggregate by sampled stack
  -> emit inuse_objects / inuse_space pprof
```

关键约束：

- 只在 allocation sampling 命中时插入 live map。
- `dealloc` 必须无条件尝试 remove pointer，但只能使用 `try_lock` 或低竞争 shard，不允许阻塞 allocator 热路径。
- `realloc` 必须同时处理旧 pointer 和新 pointer 状态。
- report 允许阻塞读取 live map shard，但不能持锁执行符号化或 pprof encode。
- live map 需要独立容量限制，例如 `max_live_tracked_allocations`。

## 成本模型

v2 需要在设计文档和 benchmark 中量化：

```text
tracked_allocations ~= live_allocated_bytes / sample_interval_bytes
metadata_memory     ~= tracked_allocations * metadata_size * map_overhead
dealloc_overhead    ~= one sharded lookup/remove per dealloc
realloc_overhead    ~= one remove + optional insert/update per realloc
report_overhead     ~= live map snapshot + aggregation + encode
```

v2 合入前必须给出 1 MiB、512 KiB、4 KiB sampling interval 下的 metadata 上限估算和实际 benchmark artifact。

## realloc 规则

- 失败：旧 pointer metadata 保留。
- 成功且 pointer 相同：更新 size。
- 成功且 pointer 变化：删除旧 pointer，插入新 pointer。

补充规则：

- `realloc(ptr, old_layout, 0)` 按 allocator contract 处理，不能错误插入新 live metadata。
- `realloc` 成功但新 pointer 未命中采样时，如果旧 pointer 已 tracked，需要删除旧 metadata，避免 inuse 泄漏。
- `alloc_zeroed` 与 `alloc` 的 tracking 语义相同。
- `dealloc` 找不到 pointer 必须是正常路径，不能计为错误。

## 可选实现策略

### 策略 A：全局 sharded map

优点：

- 实现直接。
- report 容易遍历。

缺点：

- 热路径锁竞争。
- metadata 分配可能递归。

### 策略 B：TLS ownership + global handoff

优点：

- 分配线程本地化。
- 热路径更低竞争。

缺点：

- 跨线程 dealloc 复杂。
- 线程退出需要 flush。

### 策略 C：只采样 tracked pointers

优点：

- 降低 metadata 数量。
- 与 sampling profile 一致。

缺点：

- inuse 需要权重校正。
- 大对象概率处理必须更严谨。

### 推荐选择

优先选择 **策略 C + sharded map**：

- 只跟踪采样命中的 pointer，控制 metadata 数量。
- 使用 pointer hash 选择 shard，降低跨线程 dealloc 竞争。
- report 直接 snapshot shard，避免 TLS ownership 在跨线程 dealloc 下的复杂 handoff。

不推荐 TLS ownership 作为 v2 初版主路径，因为 Rust 和 C 生态中跨线程释放很常见，TLS ownership 会把 dealloc 正确性推向更复杂的 owner transfer 协议。

## 验收门槛

v2 只有在满足以下条件时才考虑合入：

- 默认关闭。
- 明确 `MimallocConfig` opt-in 字段和默认值。
- 明确最大 metadata 内存成本。
- 明确 sampled pointer tracking 的权重校正方式。
- 明确 `alloc_*` 与 `inuse_*` 同时存在时的 default sample type。
- 多线程 dealloc/realloc 测试通过。
- 跨线程 dealloc 测试通过。
- short-lived thread + tracked live allocation 测试通过。
- 性能 benchmark 可接受。
- Pyroscope 展示 `inuse_space` 正确。

## 必测场景

- sampled `alloc` 后未释放，report 输出 `inuse_space`。
- sampled `alloc` 后 `dealloc`，report 不再输出该 pointer 的 `inuse_space`。
- sampled `alloc_zeroed` 后 report 输出 `inuse_space`。
- `realloc` 失败，旧 pointer metadata 保留。
- `realloc` 成功且 pointer 相同，size 更新。
- `realloc` 成功且 pointer 改变，旧 metadata 删除，新 metadata 按采样规则处理。
- allocation thread 和 deallocation thread 不同。
- live map 容量满时降级行为明确，不 panic、不阻塞。
- `backend-mimalloc` 与 `backend-jemalloc` / `backend-pprof-rs` all-features 仍可编译。

## v2 PR 拆分建议

1. 只加 opt-in config、live metadata 类型和 sharded map 骨架，默认关闭。
2. 接入 sampled pointer insert/remove/realloc 状态机。
3. 扩展 memory pprof encoder 支持 `inuse_objects` / `inuse_space`。
4. 增加多线程 dealloc/realloc integration tests。
5. 增加 live heap benchmark artifact 与阈值。
6. 更新 README、docs.rs 和 CHANGELOG，明确 v1/v2 语义区别。

## 当前状态

- v1 不实现 live heap / inuse profile。
- v1 已明确 allocation profile 语义。
- v2 评估已完成，推荐作为后续 opt-in 独立 PR，而不是混入当前 mimalloc backend v1 合入范围。
