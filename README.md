# Continuum / PIER

**一个把「不可靠、能力各异、成本不同、模态不同的模型」组织成长期可靠执行系统的尝试。**（§62b）

它的基本抽象不是 `Message → Response`，而是 **`Intent → 自主执行`**（§1）：用户给出目标与不可违反的约束，
其余——拆解、模型选择、推理强度、Agent 创建、工具调用、模态选择、执行顺序、并行度、设备选择、
资源调度、重试、重规划、结果验证、Artifact 管理、后台维护——由 Runtime 负责。

目标形态是**持续存在的个人智能计算环境**，而不是一次请求对应一次回答的会话系统（§1）。

## 统一的执行链

现有工作分别覆盖 Agent graph、Agent OS、Model routing、Verifier、Capability security、
User preference、Distributed inference 与 Media Agent。本项目**不以其中任一方向作为唯一核心**，
而是把它们整合成一条链（§62）：

```
Intent → 版本化 Contract → 类型化动态执行图 →
模型 / 工具 / 节点 / 权限路由 → Evidence → 验证 → Artifact → 反馈 → Model Registry
```

最高抽象（§63b）：

```
Runtime.solve(Intent, Context, Authority) -> Artifact + Evidence
```

## 设计上最要紧的一条张力

```
high autonomy  ─┐
                ├─ 必须同时存在
hard boundaries ─┘
```

**缺执行自主性，系统退化为传统会话式助手；缺强制边界，系统失去可控性**（§1.5）。

这条张力落到成功判据上是具体的（§1.4）：核心指标是
**「用户仅提供高层目标时，系统在不违反要求的前提下独立完成任务的比例」**，
以及**完成同类任务所需的用户干预次数是否持续下降**——
而**「干预次数的下降不得通过越权替用户作出决策实现」**。这是首要强制边界。

**故本仓库把边界做成编译期可见的机制，而不是约定**：`crates/continuum-runtime/tests/dependency_direction.rs`
的 `ALLOWED` 表逐对断言 workspace 成员之间的依赖边，`cargo tree` 的直接边与之互为覆盖；
跨层输入一律以**值**传递、跨层输出一律以**类型**被消费。违反边界会**编译不过**，
而不是靠评审记得去查。

## 架构：八个层

《总纲》的八章即八个层，《工程》逐层给出建设内容、组件边界、依赖关系与**完成判据**：

| 层 | 《总纲》 | 《工程》 | 建造阶段 |
|---|---|---|---|
| 语义：Intent、Specification、Contract、Plan | §2 | §2 | **P4** |
| 执行：ADFIR 图与状态机 | §3 | §3 | **P1** |
| 资源：模型、工具、算力、Artifact | §4 | §4 | **P3** |
| 边界：工作区、权限、Authority、副作用 | §5 | §5 | **P2** |
| 认知：Context、Memory、学习 | §6 | §6 | **P6** |
| 长期循环：Product Drive | §7 | §7 | **P7** |
| 跨领域：代码、研究、媒体、图像 | §8 | §8 | **P5** |
| 基础设施（持久化、事件、外围边界） | — | §1 | **P0** |

**建造顺序 P0–P7 是工程侧的排布，不是规范的规定**——规范只定义「应该是什么」，
《总纲》与《工程》都明写「本文不规定实施顺序或里程碑安排」。阶段划分与各阶段的裁决记录见 `docs/superpowers/`。

## 进度

以依赖序推进，每阶段走「**设计 → 计划 → 逐任务实现 → 独立评审 → 全分支终审**」。
**以下是 2026-10-08 的实测状态，会随建造变化；以 `docs/superpowers/` 下的阶段文档为准。**

| | |
|---|---|
| **已合入** | P0 基础设施、P1 执行层、P2 边界层、**P3 外围与调用路径**（能力强制、连接器、Provider 中立边界、模型注册表与路由器、算力放置、工具调用路径） |
| **进行中** | P3g 模型调用路径（12 个任务中的 7 个）；P4 语义层（21 个中的第 1 个） |
| **切分中** | P5 领域算子层、P6 Context Compiler |
| **未开始** | P7 长期循环 |

## 构建与测试

工具链固定 **rustc 1.95.0 / cargo 1.95.0，edition 2024**。

```sh
cargo test --workspace --no-fail-fast
cargo build --workspace --all-targets
```

## 文档

| 位置 | 内容 |
|---|---|
| `docs/spec/` | **规范原文**，编号 1–348（＋ `62b` / `63b`）。**本 README 与全仓的 `§` 引用都以这里为准。** |
| `docs/01-总纲.md` | 定位与边界、核心抽象、各组成部分的设计依据；缺口见第 9 章、未决事项见第 10 章 |
| `docs/02-工程.md` | 逐层的建设内容、组件边界、依赖关系、完成判据 |
| `docs/superpowers/specs/` | 各子项目的**设计** |
| `docs/superpowers/plans/` | 各子项目的**实现计划**与阶段裁决 |
| `docs/superpowers/*-followups.md` | 各子项目**交接给后继者的遗留**（含「无照片」项与能区分它们的变异体） |

规范正文自 §219 起使用 **MUST / MUST NOT / SHOULD / SHOULD NOT / MAY** 的规范词；
§1–§218 是设计与定位，其中的「必须」「不能」只是强调。

## 许可证

**GNU General Public License v3.0**（全文见 [`LICENSE`](LICENSE)）。

SPDX：`GPL-3.0-only`——本仓库**未**声明「或更高版本」，故按 `only` 读。
