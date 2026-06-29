# pyroscope-rs mimalloc 后端落地方案

> 日期：2026-06-29
> 适用版本：pyroscope-rs 2.0.6
> 目标：为 pyroscope crate 构建高质量 `backend-mimalloc`，提供可用于 Pyroscope 的 mimalloc allocation memory profile。

## Review 结论

当前最重要的判断是：**mimalloc 不能按 jemalloc 后端的方式简单复制实现**。

`src/backend/jemalloc.rs` 之所以很薄，是因为 `jemalloc_pprof::PROF_CTL.dump_pprof()` 已经能从 jemalloc 内部 profiling 数据直接生成 pprof bytes。mimalloc 公开 API 主要提供 allocator、统计、heap block 遍历、进程信息等能力，没有等价的“采样分配调用栈并 dump pprof”的接口。

因此，`backend-mimalloc` 的正确主线不是 `mi_heap_visit_blocks`，而是：

```text
SamplingMiMalloc global allocator wrapper
  -> allocation sampling recorder
  -> report 周期聚合和符号化
  -> memory pprof encoder
  -> ReportData::RawPprof
  -> Session 现有 raw pprof 上报路径
```

`mi_heap_visit_blocks` 只能作为辅助诊断或将来的非 flamegraph 统计能力，不能作为 v1 的主后端能力。它能回答“当前 heap 大致由哪些 size class 构成”，不能回答“这些内存由哪些调用路径分配”。

## 当前文档修正点

本次 review 后，需要纠正旧方案里的几个关键风险：

1. `mi_heap_visit_blocks` visitor 中抓 backtrace 是错误语义：抓到的是 profiling 线程遍历 heap 时的栈，不是分配发生时的栈。
2. area 级聚合 profile 不能生成真实火焰图；把 `[mimalloc] block_size=N` 做成 synthetic location 只会展示 size class，不是 call-site profiling。
3. `alloc_objects == inuse_objects`、`alloc_space == inuse_space` 是误导性近似，不应作为正式 memory profile 输出。
4. 只遍历 `mi_heap_main()` 会遗漏 thread-local heap，不能代表整个进程的 mimalloc 使用。
5. `backend-mimalloc = ["dep:libmimalloc-sys"]` 不够；真正的 profiling 需要 allocator wrapper 和栈采样能力。
6. `cargo test --features backend-mimalloc` 不是充分 gate；allocator profiling 必须覆盖 all-features、递归分配、多线程、pprof 解码和性能退化。

## 现有架构可复用部分

### Backend 生命周期

`Backend` trait 已经足够表达 mimalloc 后端生命周期：

```rust
pub trait Backend: Send {
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(self: Box<Self>) -> Result<()>;
    fn report(&mut self) -> Result<ReportBatch>;
    fn add_tag(&self, tag: ThreadTag) -> Result<()>;
    fn remove_tag(&self, tag: ThreadTag) -> Result<()>;
}
```

mimalloc 后端应实现为：

- `initialize()`：检查 sampling recorder 是否安装、配置是否合法、可选预热 unwinder。
- `report()`：drain 已采样事件，聚合、符号化、编码 memory pprof，返回 `ReportData::RawPprof`。
- `shutdown()`：关闭 recorder 或标记 inactive，不强制释放全局 allocator。
- `add_tag()`/`remove_tag()`：v1 no-op。allocator sample 不天然支持当前 thread tag rule 语义。

### ReportBatch 和 Session

当前 `ReportBatch` 已支持：

```rust
pub enum ReportData {
    Reports(Vec<Report>),
    RawPprof(Vec<u8>),
}
```

mimalloc 后端应走 `RawPprof`，理由：

- 现有 `encode::pprof::encode()` 是 CPU profile encoder，硬编码 `cpu/nanoseconds`。
- `Report` 当前每个栈只有一个 `usize` count，不能表达 memory profile 的多 sample value。
- Session 对 `RawPprof` 已能直接写入 `RawSample.raw_profile` 并附加 `service_name`、`__name__` 等 series labels。

`ReportBatch.profile_type` 应设置为：

```text
memory
```

这样 Session 会发送：

```text
__name__ = "memory"
```

## 不采用的方案

### 方案 A：只用 mi_heap_visit_blocks

不作为主方案。

优点：

- 实现简单。
- 对 allocation 热路径零开销。
- 可得到 size class、reserved、committed、used 等统计。

问题：

- 无分配调用栈。
- 多线程 heap 覆盖不完整。
- 只能生成 size-class 统计图，不能生成真实 flamegraph。
- pprof `alloc_*` 语义无法成立。
- 如果 synthetic location 写成 `[mimalloc] block_size=N`，用户会误以为这是代码路径。

可保留为后续诊断 API，例如 `mimalloc_stats_backend()` 或 debug log，但不应命名为 memory profiling 后端。

### 方案 B：在 heap visitor 中抓 backtrace

不采用。

这会抓到后端 report 线程的栈，而不是每个 block 的分配栈，数据语义错误。即使能生成 pprof，也会把所有样本归因到 `report()`/visitor 路径。

### 方案 C：默认启用 mimalloc override feature

不采用。

`libmimalloc-sys` 的 `override` feature 会影响 C runtime malloc/free，可能改变用户进程、native 依赖、测试进程和其他 allocator 假设。pyroscope 是库 crate，不应该默认接管全进程 C allocator。

用户必须显式选择 allocator：

```rust
#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::default();
```

## 推荐 v1：SamplingMiMalloc allocation profile

### 能力边界

v1 做 allocation profile：

- 记录被采样的分配调用栈。
- 输出 `alloc_objects/count` 和 `alloc_space/bytes`。
- 不做 live heap / inuse tracking。
- 不在 dealloc 热路径维护 pointer map。

这样可以先交付低风险、低热路径成本、语义准确的 memory profile。

v2 再做 live heap：

- 记录 pointer -> allocation metadata。
- dealloc/realloc 更新 live map。
- 输出 `inuse_objects/count` 和 `inuse_space/bytes`。
- 成本、复杂度和内存占用都会显著增加。

### 用户 API

建议 API：

```rust
use pyroscope::backend::mimalloc::{
    mimalloc_backend, MimallocConfig, SamplingMiMalloc,
};

#[global_allocator]
static ALLOC: SamplingMiMalloc = SamplingMiMalloc::default();

let agent = PyroscopeAgentBuilder::new(
    "http://localhost:4040",
    "example.mimalloc",
    100,
    "pyroscope-rs",
    env!("CARGO_PKG_VERSION"),
    mimalloc_backend(MimallocConfig::default()),
)
.build()?;
```

`SamplingMiMalloc` 必须是 public，因为 Rust 只允许 binary 选择全局 allocator。pyroscope crate 自己不能替下游设置。

### 配置结构

```rust
pub struct MimallocConfig {
    pub sample_interval_bytes: u64,
    pub max_depth: usize,
    pub ring_capacity: usize,
    pub report_drain_limit: usize,
}
```

默认值：

```text
sample_interval_bytes = 1 MiB
max_depth = 64
ring_capacity = 512
report_drain_limit = 1_000_000
```

默认采样间隔选 1 MiB，理由：

- 与 jemalloc `lg_prof_sample:19` 的 512 KiB 量级接近。
- 对一般服务热路径开销更保守。
- 可通过配置降低到 512 KiB 或更小用于诊断。

### 文件结构

```text
src/backend/mimalloc.rs
src/backend/mimalloc/allocator.rs
src/backend/mimalloc/config.rs
src/backend/mimalloc/recorder.rs
src/backend/mimalloc/sample.rs
src/encode/memory_pprof.rs
examples/mimalloc.rs
tests/mimalloc_backend.rs
```

如果希望首个 PR 更小，也可以先把 `mimalloc` 子模块放在单文件中，但推荐从一开始拆开 allocator、recorder、encoder，避免 unsafe 和 pprof 逻辑混在一起。

## 热路径设计

### GlobalAlloc wrapper

`SamplingMiMalloc` 包装 `mimalloc::MiMalloc`：

```rust
pub struct SamplingMiMalloc {
    inner: mimalloc::MiMalloc,
}
```

实现 `GlobalAlloc`：

- `alloc()`
- `alloc_zeroed()`
- `realloc()`
- `dealloc()`

v1 只在成功分配后记录 allocation sample：

- `alloc()` 成功后：按 `layout.size()` 采样。
- `alloc_zeroed()` 成功后：按 `layout.size()` 采样。
- `realloc()` 成功后：按 `new_size` 采样。
- `dealloc()`：只调用 inner，不记录。

`realloc()` 注意事项：

- 失败返回 null 时，旧指针仍有效，不能记录为新分配成功。
- 成功时旧指针所有权转移，v1 不维护 live map，所以只记录一次新 allocation event。
- `new_size == 0` 的行为按 `GlobalAlloc` 契约处理，不额外扩展语义。

### 递归保护

allocator hook 内不能分配、不能 panic、不能等待锁。

使用 TLS guard：

```rust
thread_local! {
    static IN_ALLOC_PROFILER: Cell<bool> = const { Cell::new(false) };
}
```

规则：

- guard 已设置时直接跳过 profiling。
- 进入慢路径前设置 guard。
- `Drop` 只重置 bool，不做任何可能分配的工作。
- hook 内所有错误都吞掉，并增加 atomic dropped counter。
- hook 内不使用 `log!`、`format!`、`Vec`、`HashMap`、`String`、`std::sync::Mutex`。

### 采样算法

推荐 byte-based Poisson sampling。

每线程 TLS 保存：

```text
remaining_bytes: i64
prng_state: u64
```

流程：

```text
on_alloc(size):
  remaining_bytes -= size
  if remaining_bytes > 0:
      return
  sample allocation
  remaining_bytes += next_poisson_interval(sample_interval_bytes)
```

大对象处理：

- 如果 `size >= sample_interval_bytes / 2`，提高采样概率。
- 超大对象可以必采。
- sample weight 按采样概率校正，避免低估大对象。

### 栈采样

v1 应记录 raw instruction pointers，不在 allocator hook 内符号化。

样本结构建议：

```rust
struct AllocationSample {
    frames: [usize; MAX_DEPTH],
    depth: u8,
    size: u64,
    weight_objects: u64,
    weight_bytes: u64,
    thread_id: u64,
}
```

栈展开策略：

- 优先复用/抽取现有 pprofrs 的 no-alloc unwinder 思路。
- hook 内只抓 raw IP。
- `report()` 阶段再 resolve symbols。
- unwinder busy 或失败时 drop sample。
- 初始化时预热 unwinder 元数据，避免第一次 allocation sample 触发大额 lazy init。

### 缓冲设计

目标：allocation 热路径零全局锁。

推荐两层缓冲：

```text
TLS fixed ring buffer
  -> try-flush to global queue
  -> report() drain global queue
```

规则：

- TLS ring 固定容量，不动态扩容。
- ring 满时尝试 flush。
- flush 失败直接丢弃本批或新样本，并增加 dropped counter。
- 不在 allocation hook 内 HashMap 聚合。
- `report()` 中进行 HashMap 聚合、符号化和 pprof 编码。

## pprof 编码语义

新增 `src/encode/memory_pprof.rs`。

不要复用当前 `src/encode/pprof.rs`，它是 CPU encoder。

### v1 sample_type

v1 输出 allocation profile：

```text
sample_type:
  alloc_objects / count
  alloc_space   / bytes

period_type:
  space / bytes

period:
  sample_interval_bytes

default_sample_type:
  alloc_space
```

每个 sample：

```text
location_id = sampled allocation stack
value       = [weighted_objects, weighted_bytes]
```

### time_nanos 和 duration_nanos

因为 `RawPprof` 路径不会由 Session 补时间，encoder 必须设置：

```text
time_nanos     = report 构造时刻
duration_nanos = report 覆盖的采样窗口
```

实现上，backend 保存 `last_report_time`：

- 第一次 report：duration 可为 0 或 upload interval。
- 后续 report：duration = now - last_report_time。

### labels

默认不写 pprof sample label。

全局 tags、`service_name`、`__name__` 已由 Session 作为 series labels 发送。不要重复写入 pprof sample。

可选低基数 label：

- `thread_id`
- `thread_name`
- `pid`

这些应由 `MimallocConfig` 控制，默认关闭，避免高基数污染。

## Feature 和依赖设计

### Cargo.toml

建议：

```toml
[dependencies]
mimalloc = { version = "0.1.52", optional = true }

[features]
backend-mimalloc = ["dep:mimalloc"]
```

如果需要直接调用 `libmimalloc-sys` 的扩展 API，再引入：

```toml
libmimalloc-sys = { version = "0.1.49", optional = true }
backend-mimalloc = ["dep:mimalloc", "dep:libmimalloc-sys"]
```

不要默认启用：

```text
libmimalloc-sys/override
```

### all-features 风险

`cargo clippy --all-targets --all-features -- -D warnings` 是本仓库提交前 gate。

因此 `backend-mimalloc` 必须能和现有 `backend-jemalloc`、`backend-pprof-rs` 同时编译。

注意：

- crate 内部不要定义 `#[global_allocator]`，否则 all-features 容易冲突。
- examples/tests 中如果同时存在多个 allocator 示例，必须用 required-features 隔离。
- 集成测试中的 `#[global_allocator]` 只能出现在该测试 crate 自己的文件里，不影响 lib。

## 平台策略

优先支持：

```text
linux x86_64
linux aarch64
macos x86_64
macos aarch64
```

如果 unwinder 复用 `backend-pprof-rs` 的平台能力，应沿用相同 gating。不要在 unsupported target 上静默编译出不可用 feature。

示例 gating：

```rust
#[cfg(all(
    feature = "backend-mimalloc",
    any(
        all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")),
        all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64")),
    )
))]
pub mod mimalloc;
```

如果 v1 使用只依赖 std backtrace 的保守实现，可扩大平台范围，但必须通过 CI 或手工验证后再声明。

## 测试计划

### 编码测试

新增 `memory_pprof` 单元测试：

- 解码生成的 protobuf。
- 断言 `sample_type == alloc_objects/count, alloc_space/bytes`。
- 断言不是 `cpu/nanoseconds`。
- 断言 `time_nanos > 0`。
- 断言 `duration_nanos` 合理。
- 断言空样本也能生成合法 profile。

### allocator wrapper 测试

新增 feature-gated 测试：

- `SamplingMiMalloc` 可作为 `#[global_allocator]`。
- 大量 `Box`/`Vec` allocation 后 report 非空。
- `alloc_zeroed` 路径可采样。
- `realloc` 成功路径可采样。
- recursion guard 不递归爆栈。
- recorder inactive 时 allocation 正常工作。
- ring 满或 global queue busy 时 allocation 不失败。

### backend 测试

新增 `tests/mimalloc_backend.rs`：

```rust
#[cfg(feature = "backend-mimalloc")]
mod tests {
    use pyroscope::backend::mimalloc::{mimalloc_backend, MimallocConfig, SamplingMiMalloc};
    use pyroscope::backend::ReportData;

    #[global_allocator]
    static ALLOC: SamplingMiMalloc = SamplingMiMalloc::default();

    #[test]
    fn mimalloc_backend_reports_raw_memory_pprof() {
        let mut backend = mimalloc_backend(MimallocConfig {
            sample_interval_bytes: 1024,
            ..MimallocConfig::default()
        })
        .initialize()
        .expect("initialize mimalloc backend");

        let allocations: Vec<Vec<u8>> = (0..4096).map(|_| vec![0_u8; 1024]).collect();
        std::hint::black_box(&allocations);

        let batch = backend.report().expect("report memory profile");
        assert_eq!(batch.profile_type, "memory");
        assert!(matches!(batch.data, ReportData::RawPprof(_)));
    }
}
```

### 多线程压力测试

测试场景：

- 1/2/4/8/16/32 threads allocation churn。
- 并发 allocation 时调用 `report()`。
- 统计 sampled、dropped、flushes、report duration。

该测试可放在 ignored test 或 benchmark，不作为默认 `cargo test` 长时间 gate。

### 命令 gate

提交前必须运行：

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

新增功能还应运行：

```bash
cargo check --no-default-features --features backend-mimalloc
cargo test --locked --lib --tests --features backend-mimalloc
cargo test --locked --lib --tests --features backend-jemalloc
cargo test --locked --lib --tests --features backend-pprof-rs -- --test-threads 1
cargo test --locked --lib --tests --all-features -- --test-threads 1
```

## 性能验证

建议新增 benchmark 或独立示例，对比：

```text
mimalloc::MiMalloc baseline
SamplingMiMalloc inactive
SamplingMiMalloc active, interval = 1 MiB
SamplingMiMalloc active, interval = 512 KiB
SamplingMiMalloc active, interval = 4 KiB
```

目标：

- inactive 热路径开销小于 2%。
- 默认 1 MiB 采样平均开销小于 5%。
- 高压 4 KiB 采样只用于诊断，不作为默认承诺。
- report drain 不应长时间阻塞 allocation。

需要记录：

- allocation throughput。
- p50/p95/p99 latency。
- sampled count。
- dropped count。
- ring flush count。
- report drain duration。
- symbolize duration。
- encoded pprof size。

## PR 拆分

### PR 1：API 和文档骨架

内容：

- `backend-mimalloc` feature。
- `MimallocConfig`。
- `SamplingMiMalloc` public type skeleton。
- `mimalloc_backend()` skeleton。
- `examples/mimalloc.rs`。
- 明确文档说明：没有安装 `SamplingMiMalloc` 时 backend 初始化失败。

验收：

- all-features 可编译。
- 不引入 global allocator 冲突。
- 文档不承诺 live heap/inuse。

### PR 2：allocation sampling recorder

内容：

- TLS guard。
- byte-based sampling。
- fixed ring buffer。
- global queue。
- dropped counters。
- `alloc`/`alloc_zeroed`/`realloc` hook。

验收：

- recursion 测试通过。
- 多线程 smoke 通过。
- inactive 开销可接受。

### PR 3：memory pprof encoder

内容：

- `src/encode/memory_pprof.rs`。
- raw IP stack 聚合。
- 符号化。
- `ReportData::RawPprof` 输出。

验收：

- 解码测试确认 memory sample_type。
- Pyroscope 本地 ingest smoke。
- 空 profile 合法。

### PR 4：性能和 CI 加固

内容：

- benchmark。
- CI matrix 增加 `backend-mimalloc`。
- docs 完整示例。
- CHANGELOG。

验收：

- pre-commit gate 通过。
- 默认配置性能目标达标。

### PR 5：v2 live heap 设计评估

内容：

- pointer tracking PoC。
- dealloc/realloc live map。
- `inuse_objects` / `inuse_space` pprof。

验收：

- 明确额外内存成本。
- 明确最大可接受 overhead。
- 决定是否合入主线或作为 opt-in feature。

## 风险清单

| 风险 | 严重性 | 规避 |
|------|--------|------|
| 将 size-class 统计误称为 flamegraph | 高 | v1 主线使用 allocation sampling |
| allocator hook 递归分配 | 高 | TLS guard、热路径禁用分配 |
| all-features allocator 冲突 | 高 | crate 内不定义 global allocator |
| `realloc` 失败路径错误 | 中 | 只在成功返回非 null 后记录 |
| 栈展开开销过高 | 高 | 采 raw IP，report 阶段符号化 |
| 锁竞争 | 高 | TLS ring + try flush，失败 drop sample |
| pprof 语义错误 | 高 | 新增 memory encoder，不复用 CPU encoder |
| 平台不支持 | 中 | 明确 cfg gating 和 compile_error |
| 用户误用 mimalloc 普通 allocator | 中 | backend initialize 检查 recorder 是否 active |

## 最终交付定义

`backend-mimalloc` 只有在满足以下条件时才算可合入：

1. 用户可以通过 `SamplingMiMalloc` 显式启用 mimalloc profiling。
2. `report()` 返回 `profile_type = "memory"` 和合法 `RawPprof`。
3. pprof sample type 是 memory 语义，不是 CPU 语义。
4. allocation 热路径无动态分配、无阻塞等待、无 panic。
5. 默认配置性能开销有 benchmark 证明。
6. `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test` 通过。
7. 文档清楚说明 v1 是 allocation profile，不是 live heap/inuse profile。

## 一句话方案

高质量 mimalloc 后端应以 `SamplingMiMalloc` 为核心：在分配发生时低成本采样真实调用栈，在 report 周期聚合并生成 memory pprof；`mi_heap_visit_blocks` 只作为辅助统计，不能作为主 profiling 方案。
