# 子任务 03：memory pprof encoder

## 目标

新增 memory pprof encoder，避免复用 CPU 专用的 `encode::pprof::encode()`。

## 为什么不能复用现有 encoder

当前 `src/encode/pprof.rs` 固定输出：

```text
sample_type = cpu / nanoseconds
period_type = cpu / nanoseconds
value       = count * cpu_period
```

mimalloc profile 需要 memory 语义：

```text
alloc_objects / count
alloc_space   / bytes
```

## v1 pprof 语义

```text
Profile.sample_type:
  alloc_objects / count
  alloc_space   / bytes

Profile.period_type:
  space / bytes

Profile.period:
  sample_interval_bytes

Profile.default_sample_type:
  alloc_space
```

每个 sample：

```text
location_id = allocation stack
value       = [weighted_objects, weighted_bytes]
```

## 时间字段

因为 `ReportData::RawPprof` 不会经过 Session 的 CPU encoder，memory encoder 必须自己设置：

```text
time_nanos     = SystemTime::now()
duration_nanos = report window
```

## 输出方式

backend 返回：

```rust
ReportBatch {
    profile_type: "memory".into(),
    data: ReportData::RawPprof(pprof_bytes),
}
```

## 测试

必须解码 protobuf 并断言：

- string table 包含 `alloc_objects`。
- string table 包含 `alloc_space`。
- string table 包含 `bytes`。
- 不包含 CPU-only `nanoseconds` sample type。
- `time_nanos > 0`。
- sample values 数量等于 sample type 数量。

## 后续增强

v2 live heap 后新增：

```text
inuse_objects / count
inuse_space   / bytes
```

并把 `default_sample_type` 改为 `inuse_space`。

## 当前实现状态

- 已新增 `src/encode/memory_pprof.rs`。
- 已输出 `alloc_objects/count` 和 `alloc_space/bytes`。
- 已由 mimalloc backend 传入 weighted allocation samples，而不是直接使用单次 allocation size。
- 已由 backend 通过 `ReportData::RawPprof` 返回。
- 已有解码测试确认不是 CPU `nanoseconds` 语义。
- 当前样本来自 allocation sampling；live heap `inuse_*` 尚未实现。
