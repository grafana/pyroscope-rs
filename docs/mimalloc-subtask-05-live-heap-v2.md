# 子任务 05：v2 live heap / inuse profile 评估

## 目标

评估是否在 v1 allocation profile 之后增加 live heap tracking，以输出 `inuse_objects` 和 `inuse_space`。

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

## 热路径成本

v2 会显著增加：

- alloc 成功后的 map insert。
- dealloc 的 map remove。
- realloc 成功/失败的状态转移。
- pointer metadata 内存占用。
- 锁竞争或 sharded map 复杂度。

因此 v2 必须是 opt-in，不能替换 v1 默认路径。

## realloc 规则

- 失败：旧 pointer metadata 保留。
- 成功且 pointer 相同：更新 size。
- 成功且 pointer 变化：删除旧 pointer，插入新 pointer。

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

## 验收门槛

v2 只有在满足以下条件时才考虑合入：

- 默认关闭。
- 明确最大 metadata 内存成本。
- 多线程 dealloc/realloc 测试通过。
- 性能 benchmark 可接受。
- Pyroscope 展示 `inuse_space` 正确。

