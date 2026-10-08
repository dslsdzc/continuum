# Continuum / PIER

Continuum 是一个面向「意图驱动自主执行」的运行时。调用方给出目标与不可违反的约束，
执行过程中的模型选择、工具调用、资源调度、验证、失败恢复与结果交付由运行时负责。

最高抽象：

```rust
Runtime.solve(Intent, Context, Authority) -> Artifact + Evidence
```

本仓库是该运行时的 Rust 实现。

## 源码布局

`crates/` 下 18 个 crate，全部是 workspace 成员。

**基础设施**

| crate | 内容 |
|---|---|
| `continuum-persist` | 持久化机制：迁移框架、事务 API、恢复钩子注册表 |
| `continuum-events` | Event Stream 与 Audit Log |
| `continuum-core` | 核心类型。不含 I/O |

**执行层**

| crate | 内容 |
|---|---|
| `continuum-graph` | ADFIR 图、Node、Edge 与连接校验 |
| `continuum-artifact` | Artifact 数据模型、内容寻址与 lineage |
| `continuum-port` | Port 定义与连接兼容性判定 |
| `continuum-operator` | Operator 定义与注册表 |

**资源层**

| crate | 内容 |
|---|---|
| `continuum-model-registry` | 模型注册表与路由器 |
| `continuum-node` | 计算节点注册与节点放置 |
| `continuum-connector` | 连接器边界层：操作集与声明完整性 |

**边界层**

| crate | 内容 |
|---|---|
| `continuum-workspace` | Workspace 抽象与只读强制 |
| `continuum-policy` | Policy Engine：规则形状、六级优先级与受限条件 |
| `continuum-effect` | Effect Journal：记录体、状态机、写入与推进 |
| `continuum-capability` | Capability：能力的半封闭词汇表，及其与 `EffectType` 的对应 |
| `continuum-sandbox` | 子进程的内核层隔离 |
| `continuum-secrets` | 密钥运行时 |

**外围与装配**

| crate | 内容 |
|---|---|
| `continuum-provider` | 外围中立接口。只定义 trait 与登记表，不含实现，也不引用任何实现类型 |
| `continuum-runtime` | 运行时的库面与可执行文件 |

核心与外围的依赖方向由 `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 表逐对断言；
向核心引入一条反向边会让该测试失败。

**尚未建立**：语义层的 `continuum-canonical` / `continuum-semantics` / `continuum-budget`
（这三个 crate 的设计与计划在 `docs/superpowers/` 下，实现刚开始）；认知层与长期循环层的 crate 亦然。
上表只列当前 `crates/` 下实际存在的成员。

## 构建

工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。

```sh
cargo build --workspace --all-targets
cargo test --workspace --no-fail-fast
```

## 文档

| 位置 | 内容 |
|---|---|
| `docs/spec/` | 规范原文，编号 1–348（另有 `62b` / `63b`）。代码注释中的 `§` 引用以这里为准 |
| `docs/01-总纲.md` | 系统定位、职责边界、核心抽象 |
| `docs/02-工程.md` | 逐层的建设内容、组件边界、依赖关系与完成判据 |
| `docs/superpowers/specs/` | 各子项目的设计 |
| `docs/superpowers/plans/` | 各子项目的实现计划与阶段裁决 |
| `docs/superpowers/*-followups.md` | 各子项目交接给后继者的遗留 |

规范正文自 §219 起使用 MUST / MUST NOT / SHOULD / SHOULD NOT / MAY；§1–§218 是设计与定位。

实现按依赖序分阶段推进，阶段划分与进度以 `docs/superpowers/` 下的阶段文档为准。

## 许可证

GNU General Public License v3.0。全文见 [`LICENSE`](LICENSE)。

SPDX：`GPL-3.0-only`。
