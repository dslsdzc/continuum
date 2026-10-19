# P6A（Context Compiler）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**任务单：** 切分 §五 P6A 行（`docs/superpowers/specs/2026-10-08-p6-scope-and-split.md:171`，逐字）：

> 建「节点执行前必经」的装配通道：从 Conversation／Memory／Artifacts／Project／Tool results 取料，
> 按最小必要裁成 `ContextPackage`，并把「不存在绕过路径」做成可观察的强制面

**本计划是该行的实现分解。设计与规范的唯一事实来源是**
`docs/superpowers/specs/2026-10-17-p6a-context-compiler-design.md`（1283 行，下称「设计」）。
**本计划不改设计、不改规范、不改切分文档、不改代码以外的任何计划。**

**Goal:** 新建一个 crate `continuum-context`，交付三件事：①§280 的 `ContextPackage` 类型**一份**
（六字段逐字、字段私有、唯一构造点 `compile`）；②装配入口 `compile(request, budget, memory)`；
③`ContextPackage` 的消费契约（六个只读访问器＋逐片段 `provenance` / `trust_labels`，不投影为
`Message`）。本块**不建执行器、不建节点状态迁移、不建模型调用、不建拒绝行为、零表零迁移**
（设计 §1.2、§9、§10）。

**Architecture:** 本块是**认知层（第 6 层）的一个叶子 crate**，依赖四条 workspace 内边：
`continuum-artifact`（`ArtifactId`）、`continuum-core`（`Message` / `ToolResult`）、
`continuum-memory`（P6C 的 `MemoryClaim` / `MemoryScope` / `MemoryError` / `MemoryRetrieval` /
`RetrievalQuery`）、`continuum-trust`（P6B 的 `ContextProvenance` / `ContextTrust` / `label` /
`PackageShape` / `validate` / `TrustError`）。**四条边全部是 P6A → 上游，P6B／P6C 不依赖本块**
（设计 §2.1、§2.3）。

`ContextPackage` 的六个字段是 `instructions` / `references` / `artifacts` / `memories` /
`provenance` / `trust_labels`（§280 的字段名与书写序逐字）。五类来源到四个内容字段的映射是：
Conversation → `references[]`（`Reference::Conversation`，当前指令另进 `instructions[]`）；
Memory → `memories[]`（只经 P6C 的检索面）；Artifacts → `artifacts[]`（只装 `ArtifactId`）；
Project → `references[]`（`Reference::Text`）；Tool results → `references[]`
（`Reference::ToolOutput`）（设计 §5.4）。

`provenance[]` 与 `trust_labels[]` 是**等长同序的平行数组**，片段序列＝
`instructions[]` → `references[]` → `artifacts[]` → `memories[]`（§280 的字段声明序）。
`compile` 每装入一个片段，就同步 push 一条 provenance 与一次 `continuum_trust::label(&provenance)`
的返回值；**标签只由 `label()` 产生，本 crate 不构造任何 `ContextTrust` 字面量**（设计 §4.2、§5.3
第 6 条）。`compile` 是纯函数：只读三个参数，不读时钟、环境、文件系统、数据库或任何全局状态
（设计 §5.5）。

**Tech Stack:** Rust 1.95.0 / edition 2024（workspace 的 `rust-version = "1.95"`）。
普通依赖：`continuum-artifact`、`continuum-core`、`continuum-memory`（P6C，**硬前置**）、
`continuum-trust`（P6B，**硬前置**）。dev 依赖：`serde_json`（J1-c 的反序列化样例与
`ToolResult` 的载荷构造）、`trybuild`（J1-a／J1-b／J1-c 与 J2 的编译失败样例）。
**不引入 `thiserror`**：本块不定义任何错误类型（设计 §2.2、§4.3 末、§5.6）；`MemoryError` 由
`continuum-memory` 提供并原样透传。`serde` 也不进本块的依赖：`ContextPackage` / `Reference`
不实现 `Serialize` / `Deserialize`（设计 §4.3、§4.5）。

**Spec:** 设计全文（**唯一事实来源**）；切分
`docs/superpowers/specs/2026-10-08-p6-scope-and-split.md` §五 P6A 行、§2.3、§四；
规范 §279（`docs/spec/05-normative.md:1500-1520`）、§280（`:1522-1536`）、§281（`:1538-1554`）、
§69（`docs/spec/02-positioning.md:322-355`）、§70（`:359-395`）；
《工程》§6.1（`docs/02-工程.md:361-368`）、§6.2（`:378`）、§6.3（`:391`）、§6.4 判据 1（`:402`）、
§9.4（`:635`）。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。`cargo test --workspace --no-fail-fast`
  必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **`ContextPackage` 的六字段逐字照 §280，一个不改、一个不加**：
  `instructions` / `references` / `artifacts` / `memories` / `provenance` / `trust_labels`。
  字段**私有**；只派生 `Debug, Clone, PartialEq`；**不实现** `Default` / `Deserialize` /
  `new` / `from_parts` / `from_fields`；**不提供** `pub` 字段、`&mut` 访问器、`into_parts()` /
  `into_iter()`（设计 §4.1、§4.3、§4.4）。**照片**：`Default` / `Deserialize` 由 Task 8 的
  `compile_fail` 钉住；`new` / `from_parts` / `from_fields` 由 Task 1 的源码面断言钉住；
  `into_parts` / `&mut` / `pub` 字段由 Task 1 的源码面断言与 Task 8 的第四份样例共同钉住。
- **`scopes` 为空 ⇒ 不构造 `RetrievalQuery`、不调检索面、`memories[]` 为空**；
  `max_memories == 0` **同样不调检索面**（设计 §5.3 第 5 条）。**一旦构造查询，传入的 `scopes`
  必须非空**——P6C 的 `RetrievalQuery::new` 对空 `scopes` 返回 `MemoryError::EmptyScopeSet`
  （P6C §6.1）。本块不把「空 scope」送进 `new`。
- `ContextRequest.anchor` 的类型是 **`Option<String>`**，原样传给
  `RetrievalQuery::new(scopes, anchor, text, limit, false)?`；`limit` 取
  `budget.max_memories`；`include_terminal` **写死 `false`**（设计 §5.2、§5.3 第 5 条）。
- `memory_query` **原样传递**：`None` 与 `Some("")` 都由 P6C 的 `new` 归一为 `None`；
  **`memory_query == Some(...)` 不是「要检索」的判据**，是否检索只由 `scopes` 与
  `max_memories` 决定（设计 §5.2、§5.3 第 5 条）。
- **标签只由 `continuum_trust::label(&provenance)` 产生**。`src/` 里**不得出现**
  `ContextTrust::` 的构造字面量、`impl ContextTrust`、`impl ContextProvenance`、
  `enum ContextTrust` / `enum ContextProvenance` / `struct MemoryClaim` / `enum MemoryScope`
  （设计 §4.6、§8.2 J9）。记忆片段的 provenance **只经**
  `ContextProvenance::memory(claim.id().as_str())` 构造（设计 §5.3 第 5 条）。
- **`compile` 只读三个参数**：`request`、`budget`、`memory`。**不读**时钟（`SystemTime` /
  `Instant`）、环境变量、当前目录、文件系统、数据库、`ArtifactStore`、全局变量或任何
  「当前 Agent／当前会话」的隐式状态（设计 §5.5）。
- **不持久化、不写事件、零表零迁移**：`crates/continuum-runtime/src/main.rs` 的
  `runtime_migrations()` **一行都不加**；`Cargo.toml` 不登记 `continuum-persist` /
  `continuum-events`（设计 §10）。
- **`ALLOWED` 恰是四条边**：
  `("continuum-context", &["continuum-artifact", "continuum-core", "continuum-memory", "continuum-trust"])`。
  内层数组按字母序。**其他既有行一条都不因本块增枚**；尤其不碰
  `continuum-graph` / `continuum-operator` / `continuum-provider` / `continuum-runtime` /
  `continuum-semantics` / `continuum-persist` / `continuum-events` / `continuum-policy` /
  `continuum-capability` / `continuum-secrets` / P5 的六个 crate（设计 §2.1）。外部 crate
  `serde_json` / `trybuild` 不进 `ALLOWED`——那张表只断言 workspace 成员之间的边。
- **不把 package 投影为 `Message` / `InvokeRequest` / prompt 文本**；不提供 `to_messages()` /
  `into_invoke_request()`；不实现 `Serialize` / `Deserialize`（设计 §6.2、§4.3）。
- **不定义 `ContextError`**；`compile` 的失败面只有 P6C 的 `MemoryError`，**原样透传、不包装**
  （设计 §2.2、§5.6）。
- **单元测试一律放 `crates/continuum-context/tests/`，不放 `src/` 的 `#[cfg(test)]`**——
  J1-d 的源码面语料限定在 `src/`，测试构造不得混入（设计 §8.2 J1-d）。
- 代码注释、错误信息、测试断言信息用中文；标识符用英文。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  新增依赖时 `Cargo.lock` 会变化，须一并提交。
- `TMPDIR` 取仓库内的 `.tmp/`（`TMPDIR="$PWD/.tmp"`）；变异脚本必须 `trap` 装还原、前后各核
  `sha256sum`、每轮独立日志路径（同 P5c 计划「已付过代价的纪律」）。
- **范围与设计一致，不多做不少做。** 明确不做：执行器、`Queued → Running`、调度、`OperatorImpl`、
  模型调用、`InvokeRequest` 构造、七个标签的类型与附加点、`may_sit_in_instructions` 的调用、
  Memory 存储／合成／纠正、Protected Core／Runtime Learning、capability 判定、tokenizer、
  token 计数、相关性打分、embedding、缓存、工具结果到 `Evidence` 的转换、Conversation／Project
  的存储、Prompt 模板与标签前缀（设计 §1.2、§9）。

---

## Review Focus

复审时**逐条按下面的用例名核**；每条都归到拥有该代码的 task 的测试，不留在「设计说了」层面。

| # | 要试着推翻的断言 | 归属 task 与用例 |
|---|---|---|
| 1 | **预算 `max_memories == 0` ＋ `memory_query = Some("x")` 时仍调检索面**（把「有查询文本」误当「要检索」） | **Task 4** `the_retrieval_surface_is_not_called_when_max_memories_is_zero`（假实现计数为 0；`scopes` 非空） |
| 2 | **`provenance[]` / `trust_labels[]` 错位**：标签数组与内容片段不同序、不同长，或某一位不是 `label(provenance[i])` | **Task 6** `the_package_passes_the_independent_validate`（调 P6B 的 `validate`，长度／逐位／类别一次判）＋ `relabeling_a_reference_changes_its_label_in_place`（换 provenance 后标签跟着变）；同步 push 的生产代码在 Task 3／Task 4 |
| 3 | **历史用户轮进 `instructions[]`**：把 `conversation_history()` 的候选放进 `request.instructions`，期望 `compile` 静默接受、`validate` 拒绝 | **Task 6** `compile_does_not_judge_historical_but_validate_does`（`validate` 返回 `HistoricalInstructionInInstructions`） |
| 4 | **`compile` 读时钟／环境／文件系统（隐藏来源）**：在 `compile` 里取 `SystemTime::now`、`std::env`、`current_dir` 或读文件 | **Task 7** `compile_reads_no_clock_environment_or_filesystem`＋**Task 9** `compile_reads_no_hidden_source`（源码面） |
| 5 | **消费方把包扁平化成 `Vec<Message>` 后标签丢失**：本块提供 `to_messages()` / `into_invoke_request()` 或把标签压进 `Message` | **Task 1** `there_is_no_projection_to_messages_or_invoke_request`＋**Task 9** `there_is_no_projection_serialization_or_persistence_surface`（源码面；`Message` 只有 `role` / `content`，没有标签位，见设计 §6.2） |

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-17 实读本工作树）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| **P6B `continuum-trust`**：`ContextProvenance` 八变体与十个构造入口、`ContextTrust` 七标签、`label()`、`PackageShape` / `validate` / `TrustError`（P6B 设计 §3.1–§3.5） | **未交付**（实读 `ls crates/`：无 `continuum-trust`；`grep -rn "continuum-trust\|continuum_trust" crates/ Cargo.toml` 零命中） | `ContextPackage.provenance` / `trust_labels` 的元素类型；`ContextCandidate.provenance`；J3 的 `validate`；`ArtifactProducer` / `ToolOutput { external }` 的构造。**硬前置** |
| **P6C `continuum-memory`**：`MemoryClaim` 七字段＋七访问器、`MemoryScope` 五臂、`MemoryError`、`MemoryRetrieval`、`RetrievalQuery`（P6C 设计 §3.5、§6.1） | **未交付**（同上，`ls crates/` 无 `continuum-memory`；`grep -rn "MemoryClaim\|MemoryScope\|MemoryRetrieval" crates/` 零命中） | `ContextPackage.memories` 的元素；`compile` 的检索分支；`MemoryError` 透传。**硬前置** |
| **P6C 的 claim 测试构造面**：`MemoryClaim::new(..)` | **`pub(crate)`**（P6C 设计 §5.1 逐字「crate 外只能读不能造」，§5.5 再次写明唯一产生点）。P6A 的假检索面**无法在 crate 外造出一枚 `MemoryClaim`**。**解法已选定**：P6C 交付 feature-gated 的 `#[doc(hidden)] pub fn for_test(...)`（`features = ["test-support"]`），**契约已对齐（fix round 3），代码待交付**；`[dev-dependencies]` 的 feature 行是 **Task 4 Step 3b 的条件步**，P6C 没有该 feature 前**不得加** | J4② / J5 的 memories 半 / J7a／J7b / J8 / J14 反例的 claim 夹具。**P6C 交付 `for_test` 前，Task 4–7 的相关用例挂起，Task 10 不得报全绿**（见「遗留/待协调」第 1 条） |
| `continuum-artifact`：`ArtifactId`（字段私有、`new` / `as_str`、`Hash`，`crates/continuum-artifact/src/artifact.rs:195-211`） | **已交付** | `artifacts[]` 的元素；去重 |
| `continuum-core`：`Message { role, content }`（`crates/continuum-core/src/model.rs:38-41`）、`ToolResult { output, is_error }`（`crates/continuum-core/src/tool.rs:26-30`） | **已交付** | `Reference::Conversation` / `Reference::ToolOutput` 的载荷 |
| `serde_json` / `trybuild` | **workspace 已登记**（根 `Cargo.toml` 的 `[workspace.dependencies]`） | dev 依赖；外部 crate 不进 `ALLOWED` |
| 本块新建的 `continuum-context` | **不存在**（`grep -rn "continuum-context\|continuum_context" crates/ Cargo.toml` 零命中） | 本计划的全部交付物 |

**兄弟计划（同工作树，已存在）的落地位置**：`2026-10-17-p6b-trust-labels.md` 的 Task 1–5
（`validate` 在 Task 4）；`2026-10-17-p6c-memory-and-learning.md` 的 Task 1–12
（`MemoryClaim` 在 Task 4、检索面在 Task 7）。**P6C 的 Task 11 的 C2 照片把
「crate 外调 `MemoryClaim::new`」钉成编译失败**——这不是本计划的猜测，是兄弟计划自己的判据；
**C2 不必改**：它钉的是 `new` 仍为 `pub(crate)`，而 P6A 要的是**另一个入口**
`#[cfg(feature = "test-support")] #[doc(hidden)] for_test`，两件事不冲突。P6A 的记忆用例必须等
P6C 交出这条测试构造面（遗留第一节第 1 条）。

**最小落地集（按兄弟计划的 task 粒度）**：P6A 的 Task 1 需要 P6B 的 Task 1–3（`ContextTrust` /
`ContextProvenance` / `label`）与 P6C 的 Task 1／4（`MemoryClaim` / `MemoryScope`）；Task 2 另需
P6C Task 4 的 `MemoryScope`；Task 3 的签名需要 P6C Task 7 的 `MemoryRetrieval`；Task 4 需要
P6C Task 7 的检索面与**claim 测试构造面**（`MemoryClaim::for_test` ＋ `test-support` feature，
见「遗留/待协调」第 1 条）；Task 6 需要 P6B Task 4 的 `validate`；Task 8 需要
两边 crate 能编译；Task 9 同样。**不必等 P6B／P6C 的全部 task 完成**，但**必须等上表列出的
最小集**。

**一句话结论**：本块**依赖两个尚未落地的 P6 兄弟 crate**（P6B／P6C），它们的**类型与签名已由设计冻结**，
但**代码不在仓里**。本计划对着 P6B §3.1–§3.5 与 P6C §3.5／§6.1 的**实际名**写；落地时以 crate 源码为准，
不符时回报，不在本块侧另造同义类型。

### 二、哪些 task 可立即开工、哪些必须等

| task | 源码能否先写 | `cargo test` 何时能跑 | 卡在哪 |
|---|---|---|---|
| Task 1（骨架／`ContextPackage`／`Reference`） | **能**（纯类型） | 等 P6B＋P6C | `lib.rs` 的字段类型直接命名 `ContextProvenance` / `ContextTrust` / `MemoryClaim`；两个 crate 不在，`E0432` |
| Task 2（请求／候选／预算） | **能**（纯类型） | 等 P6B＋P6C | `MemoryScope` / `Reference` / `ArtifactId` / `ContextProvenance` |
| Task 3（`compile` 骨架与五来源映射） | **能**（纯函数） | 等 P6B＋P6C | 同上；`label()` 的签名 |
| Task 4（Memory 检索与失败面） | 能写，但**claim 用例跑不了** | 等 P6B＋P6C＋ P6C 的 `for_test` | P6C 的 `MemoryClaim::new` 是 `pub(crate)`；假检索面返回不了 claim。**claim 用例挂起直到 P6C 交付 `for_test`**（新缺口第 1 条） |
| Task 5（预算／截断／去重） | **能** | 等 P6B＋P6C＋ P6C 的 `for_test` | J5 的 memories 半要 claim；**该半挂起直到 P6C 交付 `for_test`** |
| Task 6（标签对齐／`validate`） | **能** | 等 P6B＋P6C＋ P6C 的 `for_test` | J3 正例要 claim；**该条挂起直到 P6C 交付 `for_test`**；`validate` 来自 P6B |
| Task 7（无隐藏来源／确定性） | **能** | 等 P6B＋P6C＋ P6C 的 `for_test` | J7a／J7b 要 claim；**两条挂起直到 P6C 交付 `for_test`** |
| Task 8（trybuild） | 样例能先写，**`.stderr` 必须实跑取** | 等 P6B＋P6C | 样例 `use continuum_context::ContextPackage`；`serde_json` 反序列化样例 |
| Task 9（源码面 J1-d／J9） | **能** | 等 P6B＋P6C（测试要能编译） | 无 claim 需求 |
| Task 10（收尾） | **不能** | 等全部 | —— |

**「源码能先写」不等于「门能绿」**：在 P6B／P6C 落地前，`cargo test -p continuum-context` 的失败是
`E0432` / `E0433`（外部 crate 不存在），按 P5c 纪律 1(c) **不是红**。**本计划的执行起点是 P6B 与 P6C
两个 crate 已落地**；在那之前只做源码与用例的起草，不报「已实现」。
**在 P6C 交付 `for_test` 之前，Task 4–7 的 claim 用例一律按「挂起」计；Task 10 不得报全绿，只能报
「非 claim 面全绿＋claim 面挂起（等 P6C `for_test`）」。** **「挂起」的操作定义**：该测试体**不写入**
（或整段注释掉留 TODO）；`#[ignore]` 不行——它绕不过编译期的 `E0599`，整个 `--test` target 仍会编不过，
连带非 claim 用例一起跑不了。

**新 crate 进 workspace 会让 `dependency_direction` 先红**：`workspace_crates()` 从 workspace 定义派生，
`ALLOWED` 里没有 `continuum-context` 条目时 `every_crate_depends_only_on_its_allowed_set` 报
「workspace 成员未列入 ALLOWED」。**这是刻意的**（设计 §2.1 共写文件表逐字写着）；Task 1 同一步
加 member 与 `ALLOWED` 条目。**在 P6B／P6C 未落地时加 member，这道门还会因为
`cargo tree -p continuum-context` 失败而红**——所以**推荐把 member 的落点放在 P6B／P6C 已进树的同一步**；
若提前加，**不要把红当回归**。

### 三、共写文件（**只有两行**）

| 文件 | 本块的改动 | 落在哪个 task |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `"crates/continuum-context"` | Task 1 Step 3 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` | 加 `("continuum-context", &["continuum-artifact", "continuum-core", "continuum-memory", "continuum-trust"])`；内层数组按字母序 | Task 1 Step 3 |
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()` | **一行都不加**（本块零表） | —— |

**计划外的一处测试共写（补记，不计入上面两行）**：`crates/continuum-memory/tests/wiring.rs`
的 S2 断言（`allowed_table_has_the_memory_row_and_the_runtime_entry`）在实施期改为只在
`ALLOWED` 表段内计数，与 main 的 `e4a1ab2` 一致（本块的提交为 `481c19d`）。起因是 P6D 的
`P6_EXPECTED_ROWS` 与 P6A 的 `continuum-context` 条目都会引用 `continuum-memory`，全文计数
会随块数漂移。本块不改 P6C 的源码，只同步这一条测试断言。

**不碰的文件**：`crates/continuum-artifact/src/artifact.rs`、`crates/continuum-graph/src/**`、
`crates/continuum-operator/src/**`、`crates/continuum-runtime/src/**`（本块没有装配面）、
`docs/02-工程.md`、`docs/spec/**`、`docs/superpowers/specs/**`（设计一份都不改）。

### 四、本节把什么交给谁

- **交给 P1 的节点执行器（未建）**：`compile` 的生产调用点——节点从 `Ready`／`Queued` 进入
  `Running` **之前**调一次；`NodeContext` 是否增 `ContextPackage` 字段／runtime 的调用序，两个候选
  见设计 §6.3。**本块以库形态交付，不建执行器**（设计 §7.3、§11 第 1 条）。
- **交给 P6B 的 `continuum-trust`**：`provenance[]` / `trust_labels[]` 两个字段的承载物；本块逐片段调
  `label()`，不缓存、不覆盖、不合并（设计 §7.1）。
- **交给 P6C 的 `continuum-memory`**：本块不写 P6C 的表、不产生 claim；只消费 `MemoryRetrieval` 的返回
  （设计 §7.2）。**并请 P6C 交付 `#[cfg(feature = "test-support")] #[doc(hidden)] pub fn MemoryClaim::for_test(...)`**
  （七字段签名见「遗留/待协调」第 1 条；P6A 的 `[dev-dependencies]` feature 行是 Task 4 Step 3b 的
  条件步，预检通过后才加；**收件人：P6C＋协调者**）。
- **交给 P2**：`trust_labels[]` 的拒绝行为（§6.2 `:380`）；本块不判、不拒（设计 §7.5）。
- **交给 P3**：标签怎样过模型边界；本块不投影为 `Message`（设计 §7.4）。
- **交给规范维护者（本项目无此角色）**：§280 的元素类型、对齐规则、三类来源的落点、§70 与 §279
  的来源表差异、「最小必要」的相关性判据（设计 §11 第 3–7 条）。
- **交给协调者**：迁移档（本块零表）、`continuum-artifact` 的层归属、§9.4 的读法、capability 判定的
  内外（设计 §11 第 2、9、22 条）。

---

## File Structure

```
crates/continuum-context/
  Cargo.toml            四条普通依赖；dev：serde_json / trybuild
  src/lib.rs            crate 文档、ContextPackage 的六字段私有定义、导出面（不写 #[cfg(test)]）
  src/package.rs        ContextPackage 的六个只读访问器（§4.4；impl 块可在子模块）
  src/reference.rs      Reference：Text / Conversation / ToolOutput（§4.5）
  src/request.rs        ContextRequest / ContextBudget（§5.2）
  src/candidate.rs      ContextCandidate<T>（§5.2）
  src/compile.rs        compile：唯一构造点、五来源装配、预算、去重、标签 push（§5.1–§5.6）
  tests/package.rs      J2 的源码面（六字段名/序/私有、恰六个访问器）＋禁止构造入口＋「无投影 API」
  tests/request.rs      请求/候选/预算的构造面（Task 2）
  tests/compile_sources.rs  J4①③④⑤⑥、J6、J10、J11、对齐序（Task 3）
  tests/support/mod.rs  假检索面（计数／记录查询）与 claim 夹具（Task 4 起；claim 夹具待 P6C 构造面）
  tests/memory.rs       J4②、J8、J13、J14、J5 的 memories 半（Task 4）
  tests/budget.rs       J5 的 references/artifacts 半、J11 的预算 0（Task 5）
  tests/labels.rs       J3 的 `validate` 正例/反例＋换 provenance（Task 6）
  tests/purity.rs       J7a / J7b / 无时钟环境（Task 7）
  tests/type_level.rs   trybuild 驱动（Task 8）
  tests/compile_fail/package_literal_is_private.rs   （＋同名 .stderr）
  tests/compile_fail/package_default_is_absent.rs    （＋同名 .stderr）
  tests/compile_fail/package_deserialize_is_absent.rs（＋同名 .stderr）
  tests/compile_fail/package_fields_are_read_only.rs （＋同名 .stderr）
  tests/ui_pass/package_is_reachable.rs
  tests/source_surface.rs  J1-d / J9 的源码面（Task 9）
```

**没有 `src/error.rs`**：设计 §2.2 逐字「不新增的类型：… `ContextError`（本块没有自己的判定失败，
§5.6）」；§4.3 末与 §5.6 再次写明不定义错误类型。**不建一个空文件充数**。
**没有 `src/builder.rs`**：对齐由 `compile` 内部的局部变量与同步 push 保证；不新增公开类型
（设计 §4.2 第 5 条、§4.3）。

**为什么 `ContextPackage` 定义在 `src/lib.rs` 而不是 `src/package.rs`**：Rust 的私有字段只对
**定义它的模块及其后代**可见。`compile` 在 `src/compile.rs`（`lib.rs` 的子模块），`ContextPackage`
若定义在兄弟模块 `package` 里，`compile` **写不出结构体字面量**——那会逼出第二个构造点
（`pub(crate) from_parts`），与设计 §4.3 和 J1-d 相抵。把结构体定义在 crate 根（`lib.rs`），
`compile.rs` 与 `package.rs` 都是它的后代模块：前者能构造、后者能读字段；**crate 外仍然构造不出**
（J1-a 的编译失败样例照旧成立）。六个访问器留在 `package.rs`，职责不混。

**本计划要改的既有文件**（只有两处，都在切分 §一 的共写文件表里）：

```
Cargo.toml                                              members 加 crates/continuum-context（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 加 continuum-context 条目（Task 1）
```

---

## 各 task 的「运行，确认失败」一步量到的是哪一种失败（逐 task 扫过）

**这一步的名字只在「本 task 有新生产代码、且用例引用的符号已经存在」时才可能量到运行期的红。**
逐 task 实测形态如下——凡量到**编译失败**的，按 P5c 纪律 1(c) **不算红**，本计划照实标注；
凡本 task **不交付生产代码**（只新建测试）的，测试对的是前面已交付的实现，**首次即绿**，
那一类的步名不叫「确认失败」：

| task 的那一步 | 量到的是 | 为什么 |
|---|---|---|
| Task 1 Step 2 | **package 未找到**（不是编译失败） | Step 3 才建 `Cargo.toml` 并加入 workspace；此刻 cargo 先报 package 不存在，`E0432`／`E0433` 要到 Step 3 之后才可能出现 |
| Task 2 Step 2 | **编译失败**（`E0432`） | `ContextRequest` / `ContextBudget` / `ContextCandidate` 由 Step 3 才建出 |
| Task 3 Step 2 | **编译失败**（`E0432`） | `compile` 由 Step 3 才建出 |
| Task 4 Step 2 | **运行期红**（记忆分支未实现：`memories[]` 空、计数 0）；`for_test` 未交付时 claim 五条**不写入**（若先写则 `E0599`，拖垮整个 target） | `compile` 已在 Task 3 存在；本 task 加的是记忆分支。**claim 五条挂起直到 P6C 交付 `for_test`**（跨计划前置第二条） |
| Task 5 Step 2 | **运行期红**（未截断／未去重） | `compile` 存在；本 task 加的是预算与去重。两条 claim 用例在 `for_test` 交付前挂起 |
| Task 6 Step 2 | **首次即绿**（五条；J3 正例挂起） | 本 task 只新建测试，断言 Task 3／Task 4 已交付的同步 push 与 P6B 的 `validate`；claim 正例等 P6C |
| Task 7 Step 2 | **首次即绿**（一条；J7a／J7b 挂起） | 本 task 只新建测试，断言 Task 3／Task 4 已交付的纯函数行为；两条 claim 用例等 P6C |
| Task 8 Step 2 | **运行期红**（trybuild 报样例与 `.stderr` 不符） | 样例与 `.stderr` 都在本 task 内；`.stderr` 尚未取值 |
| Task 9 Step 2 | **首次即绿**（前提：J1-d 用修正后的两段计数） | 本 task 只新建源码面断言；旧的裸 `ContextPackage {` 计数会把 `impl` 算进去而必红，已改。若红了，说明 Task 1–8 的源码面已漂，要回查 |
| Task 10 | 无「确认失败」步 | 它是收尾与复核 |

**「首次即绿」不是缺陷，也不许改写成「先红后绿」**：Task 6／Task 7／Task 9 的交付物就是断言本身，
它们的红由 Task 10 的变异复核取（改 `compile.rs` 的某一处 → 对应断言红）。

---

### Task 1: `continuum-context` 骨架、`ContextPackage` 与 `Reference`

**Files:**
- Create: `crates/continuum-context/Cargo.toml`
- Create: `crates/continuum-context/src/lib.rs`
- Create: `crates/continuum-context/src/package.rs`
- Create: `crates/continuum-context/src/reference.rs`
- Create: `crates/continuum-context/tests/package.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加 `"crates/continuum-context"`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-context` 条目）

**Interfaces:**
- Consumes: `continuum_artifact::ArtifactId`；`continuum_core::model::Message`；
  `continuum_core::tool::ToolResult`；`continuum_memory::MemoryClaim`；
  `continuum_trust::{ContextProvenance, ContextTrust}`
- Produces: `continuum_context::{ContextPackage, Reference}`

**本 task 的四条用例对应 J2 的源码面、J1-d 的「禁止构造入口」一半（计数在 Task 9）、
Review Focus 5。** `ContextPackage` 的
`Debug` 只用于测试／诊断，**不作为审计或日志输出**（设计 §4.3）；本 task 的所有断言都不把整包写进
日志。

- [ ] **Step 1: 写 `tests/package.rs` 的四条用例**

**动作：** 新建 `tests/package.rs`，写下面四条。

- `the_six_fields_are_verbatim_in_declaration_order`：读 `include_str!("../src/lib.rs")`，
  在 `pub struct ContextPackage {` 与随后的 `}` 之间逐行取**非空行**的字段名（跳过空行），断言**恰是**
  `instructions` / `references` / `artifacts` / `memories` / `provenance` / `trust_labels`，
  且顺序逐字。**期望值手写，不从源码解析结果反向生成**；解析只用于取行。
- `the_package_has_only_read_only_accessors`：定义 `fn assert_accessors(package: &ContextPackage)`，
  逐个调用六个访问器并把返回值绑定到切片类型（编译期证明它们存在且只读）；测试主体写
  `let _: fn(&ContextPackage) = assert_accessors;` **显式引用它而不调用**（避免 `dead_code` warning）。
  先在 `tests/package.rs` 的**模块级**（所有用例之外）定义语料 helper
  `fn src_texts() -> Vec<(&'static str, &'static str)>`：Task 1 收
  `lib.rs` / `package.rs` / `reference.rs` 三个文件（Task 2 扩到五个、Task 3 扩到六个；**下游断言
  一律遍历这个 helper**，不在本地另写文件清单）。再读语料：
  ① 断言**没有** `&mut self`、`pub fn instructions_mut`、`pub fn into_parts`、`pub fn into_iter`、
  `pub instructions:` 这类公开字段行；
  ② 遍历 `src_texts()`，断言每个文件的 `impl ContextPackage {` 计数：`src/package.rs` **恰一处**，
  `src/lib.rs` / `src/reference.rs` **零处**（Task 2／Task 3 扩语料后，`request.rs` /
  `candidate.rs` / `compile.rs` 也各自零处——**最终六个文件合计恰一处**）；
  在那一处 `impl` 与随后的 `}` 之间逐个取 `pub fn <name>`，断言**恰六条**且名字依次是
  `instructions` / `references` / `artifacts` / `memories` / `provenance` / `trust_labels`
  ——**第七个只读访问器（或第二个 `impl` 块）会让这条红**（J2）；
  ③ 遍历 `src_texts()`，断言每个文件**不含** `pub fn new` / `from_parts` / `from_fields`
  （设计 §4.3 的三个禁止构造入口；注释里也不要写这三个名字；Task 2／Task 3 扩语料后覆盖六个文件）。
- `there_is_no_projection_to_messages_or_invoke_request`：遍历 `src_texts()`（Task 1 三个文件；
  `request.rs` / `candidate.rs` 由 Task 2 加入、`compile.rs` 由 Task 3 加入），断言
  `to_messages` / `into_invoke_request` / `InvokeRequest`
  **零命中**；并断言没有 `impl Serialize` / `impl Deserialize` /
  `Serialize for ContextPackage` / `Deserialize for ContextPackage` 这类实现名。
  **这条是 Review Focus 5 的一半**：`Message` 只有 `role` / `content`，没有标签位，
  投影必然丢标签（设计 §6.2）。
- `reference_keeps_message_and_tool_result_verbatim`：构造
  `Reference::Conversation(Message { role: Role::User, content: "…".into() })` 与
  `Reference::ToolOutput(ToolResult { output: serde_json::json!({"k":"v"}), is_error: true })`，
  断言它们 `Clone` 后相等、`PartialEq` 区分不同变体；**不序列化**、不拼接成字符串。

**结果：** 文件存在，四条用例名可用 `cargo test -p continuum-context --test package -- --list`
列出（P6B／P6C 就绪后）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test package
```

**这一步量到的不是 `E0432`／`E0433`**：Step 3 才建 `Cargo.toml`、`src/lib.rs` 并把 crate 加进
workspace；此刻 `cargo test -p continuum-context` 先报「package ID specification did not match any
packages」（**package 未找到**），连编译都不会开始。若只建了目录而没有 manifest，则是「找不到
`Cargo.toml`」。按纪律 1(c) 这**不算红**。**结果：** 命令以「package 未找到」/「找不到 manifest」
结束；不是断言失败，也不是 `E0432`／`E0433`。P6B／P6C 未落地的问题要到 Step 3 之后才会显现。

- [ ] **Step 3: 建 crate、类型与两处共写文件**

**动作 3a：** 新建 `Cargo.toml`：

```toml
[package]
name = "continuum-context"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
# `ArtifactId`：`artifacts[]` 的元素（设计 §4.1）。
continuum-artifact = { path = "../continuum-artifact" }
# `Message` / `ToolResult`：`Reference` 的两个载荷（设计 §4.5）。
continuum-core = { path = "../continuum-core" }
# `MemoryClaim`：`memories[]` 的元素；`MemoryRetrieval` / `RetrievalQuery` /
# `MemoryScope` / `MemoryError` 由 Task 4 使用。**P6C 的 crate，硬前置。**
continuum-memory = { path = "../continuum-memory" }
# `ContextProvenance` / `ContextTrust` / `label` / `PackageShape` / `validate`。
# **P6B 的 crate，硬前置**；本 crate 不定义这两个类型（设计 §4.6）。
continuum-trust = { path = "../continuum-trust" }

[dev-dependencies]
# `ToolResult.output` 的构造与 J1-c 的反序列化样例（设计 §8.2 J1-c）。
serde_json = { workspace = true }
```

**动作 3b：** 新建 `src/lib.rs`。`ContextPackage` 的**结构体定义放在 crate 根**（理由见
「File Structure」末段：私有字段只对定义模块及其后代可见，`compile.rs` 才写得出结构体字面量）：

```rust
//! P6A Context Compiler：§280 的 `ContextPackage` 与装配入口 `compile`。
//! 本 crate 不建执行器、不持久化；标签类型来自 `continuum-trust`（P6B）。

mod package;
mod reference;

use continuum_artifact::ArtifactId;
use continuum_memory::MemoryClaim;
use continuum_trust::{ContextProvenance, ContextTrust};

pub use reference::Reference;

/// §280 的六个字段。字段名与书写序照规范逐字，一个不改、一个不加。
/// 字段私有：唯一的构造点是 `crate::compile`（设计 §4.3）。
/// 不实现 `Default` / `Deserialize`（那会造出第二个构造点）。
#[derive(Debug, Clone, PartialEq)]
pub struct ContextPackage {
    instructions: Vec<String>,
    references: Vec<Reference>,
    artifacts: Vec<ArtifactId>,
    memories: Vec<MemoryClaim>,

    provenance: Vec<ContextProvenance>,
    trust_labels: Vec<ContextTrust>,
}
```

**动作 3c：** 新建 `src/reference.rs`，内容照设计 §4.5：

```rust
use continuum_core::model::Message;
use continuum_core::tool::ToolResult;

/// §280 的 `references[]` 元素。规范未给元素类型；设计定（§2.4 第 1 条）。
#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    /// 普通引用文本：文档、URL 内容、Project 片段、Intent 的文本投影。
    Text(String),
    /// 历史对话轮。复用 §315 的 `Message`（role + content），不另造 `Turn`。
    Conversation(Message),
    /// 工具结果。复用 §316 的 `ToolResult`，不造 wrapper。
    ToolOutput(ToolResult),
}
```

**动作 3d：** 新建 `src/package.rs`，只放六个只读访问器（`package` 是 crate 根的子模块，
可以读根模块的私有字段）：

```rust
use continuum_artifact::ArtifactId;
use continuum_memory::MemoryClaim;
use continuum_trust::{ContextProvenance, ContextTrust};

use crate::{ContextPackage, Reference};

impl ContextPackage {
    pub fn instructions(&self) -> &[String] { &self.instructions }
    pub fn references(&self) -> &[Reference] { &self.references }
    pub fn artifacts(&self) -> &[ArtifactId] { &self.artifacts }
    pub fn memories(&self) -> &[MemoryClaim] { &self.memories }
    pub fn provenance(&self) -> &[ContextProvenance] { &self.provenance }
    pub fn trust_labels(&self) -> &[ContextTrust] { &self.trust_labels }
}
```

**动作 3e：** 根 `Cargo.toml` 的 `[workspace] members` 加一行 `"crates/continuum-context"`；
`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加一条（**内层数组按字母序**）：

```rust
    // P6A Context Compiler（设计 §2.1）。四条边全部是 P6A → 上游：
    // artifact（ArtifactId）、core（Message / ToolResult）、
    // memory（P6C 的 claim 与检索面）、trust（P6B 的 provenance 与标签）。
    // **P6B／P6C 不依赖本块**；本块不碰 runtime_migrations()（零表）。
    (
        "continuum-context",
        &[
            "continuum-artifact",
            "continuum-core",
            "continuum-memory",
            "continuum-trust",
        ],
    ),
```

**结果：** `cargo metadata --no-deps` 的 workspace 成员包含 `continuum-context`；`ALLOWED` 的
`continuum-context` 行恰为四项；`src/lib.rs` 的字段顺序与 §280 一致；`src/package.rs` 只有
一个 `impl` 块、没有字段定义，且块内恰六条 `pub fn`。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test package
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(context): crate 骨架与 ContextPackage/Reference 的形状"
```

**结果：** `package` 测试全绿；全量测试日志里 `Running` 与 `test result:` 行数覆盖新增目标；
`git status` 里只剩 `.tmp/`（不入库）。**若 P6B／P6C 尚未落地，本步不执行**——先把源码留在工作树，
按跨计划前置第二条等硬前置。

---

### Task 2: `ContextRequest`、`ContextCandidate<T>` 与 `ContextBudget`

**Files:**
- Create: `crates/continuum-context/src/request.rs`
- Create: `crates/continuum-context/src/candidate.rs`
- Create: `crates/continuum-context/tests/request.rs`
- Modify: `crates/continuum-context/src/lib.rs`（导出）
- Modify: `crates/continuum-context/tests/package.rs`（把两个新源文件加入 `src_texts()` 语料：
  五文件；「无投影」、`impl ContextPackage` 计数与禁止构造入口三条断言都随之扩）

**Interfaces:**
- Consumes: `continuum_artifact::ArtifactId`；`continuum_memory::MemoryScope`；
  `continuum_trust::ContextProvenance`；Task 1 的 `Reference`
- Produces: `continuum_context::{ContextRequest, ContextCandidate, ContextBudget}`

**三个类型都是纯输入形状，没有方法、没有 `Clone`、没有 `Default`**：设计 §5.2 的代码块没有给
它们任何派生；`ContextRequest` 按值消费，**不给它加 `Clone`**（设计 §5.2 末：要重复编译就构造两份
逐字段相同的请求，J7b 的照片如此写）。

- [ ] **Step 1: 写 `tests/request.rs` 的三条用例**

- `a_request_carries_the_three_memory_controls`：构造一个 `ContextRequest`，`scopes` 为
  `vec![MemoryScope::Personal]`、`anchor` 为 `None`、`memory_query` 为 `Some("x".into())`；
  断言三个字段逐值相等（`MemoryScope` 用 `matches!` 或派生比较），并断言
  `instructions` / `references` / `artifacts` 三个容器各自能装下对应类型的候选。
- `a_candidate_carries_content_and_provenance`：分别构造
  `ContextCandidate<String>`、`ContextCandidate<Reference>`、`ContextCandidate<ArtifactId>`，
  断言 `content` 与 `provenance` 两个字段都按值可读；`provenance` 用
  `ContextProvenance::user_instruction()` / `project_data()` / `artifact(ArtifactProducer::Project)`。
- `the_budget_is_three_independent_counts`：构造 `ContextBudget { max_references: 0,
  max_artifacts: 2, max_memories: 1 }`，断言三个字段各自保留，**互不派生、互不覆盖**；
  不写「有没有 `Default`」这类恒真断言（本块不要求 `ContextBudget` 实现 `Default`，
  设计 §5.2 也没有给）。

**结果：** 三条用例名可列出；文件里的类型名与设计 §5.2 逐字。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test request
```

**结果：** 编译失败（`E0432`：`ContextRequest` / `ContextBudget` / `ContextCandidate` 尚不存在）。
**这不是红**，是编译期照片。

- [ ] **Step 3: 实现三个类型并导出**

`src/request.rs` 逐字照设计 §5.2：

```rust
use continuum_artifact::ArtifactId;
use continuum_memory::MemoryScope;

use crate::{ContextCandidate, Reference};

/// §279 五类来源的候选集合与记忆检索的三个控制位。
/// 按值消费：`compile` 可以把内容移进产物，不需要 `Clone`。
pub struct ContextRequest {
    pub instructions: Vec<ContextCandidate<String>>,
    pub references: Vec<ContextCandidate<Reference>>,
    pub artifacts: Vec<ContextCandidate<ArtifactId>>,
    pub scopes: Vec<MemoryScope>,
    pub anchor: Option<String>,
    pub memory_query: Option<String>,
}

/// 三个独立的条数上限。**不是 token 预算**（设计 §2.4 第 5 条）。
pub struct ContextBudget {
    pub max_references: usize,
    pub max_artifacts: usize,
    pub max_memories: usize,
}
```

`src/candidate.rs`：

```rust
use continuum_trust::ContextProvenance;

/// 一枚候选片段：内容 + 它的来源。`provenance` 由调用方经 P6B 的构造入口给；
/// 本块不构造它（记忆片段除外：`ContextProvenance::memory(claim.id().as_str())`）。
pub struct ContextCandidate<T> {
    pub content: T,
    pub provenance: ContextProvenance,
}
```

`src/lib.rs` 增加 `mod candidate; mod request;` 与
`pub use candidate::ContextCandidate; pub use request::{ContextBudget, ContextRequest};`。
**不添加任何 `impl`**。`ContextPackage` 已在 crate 根定义，不需要再 `pub use` 自己。
同时把 `tests/package.rs` 的 `src_texts()` 从三个文件扩到五个（加入 `src/request.rs` /
`src/candidate.rs`）——Task 1 时它们尚不存在；无投影、`impl ContextPackage` 计数与三个禁止
构造入口（`pub fn new` / `from_parts` / `from_fields`）三条断言**都遍历这个 helper**，一起扩到五文件。

**结果：** `src/request.rs` / `src/candidate.rs` 存在；字段名与设计 §5.2 逐字；`ContextRequest`
没有 `#[derive(Clone)]`。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test request
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "feat(context): 请求、候选与预算类型"
```

**结果：** `request` 测试全绿；全量门 0 warning。

---

### Task 3: `compile` 骨架与五来源映射（Memory 之外）

**Files:**
- Create: `crates/continuum-context/src/compile.rs`
- Create: `crates/continuum-context/tests/compile_sources.rs`
- Modify: `crates/continuum-context/src/lib.rs`（导出 `compile`）
- Modify: `crates/continuum-context/tests/package.rs`（把 `src/compile.rs` 加入 `src_texts()` 语料：
  六文件；「无投影」、`impl ContextPackage` 计数与禁止构造入口三条断言都随之终态化）

**Interfaces:**
- Consumes: Task 1 的 `ContextPackage` / `Reference`；Task 2 的 `ContextRequest` /
  `ContextCandidate` / `ContextBudget`；`continuum_memory::{MemoryError, MemoryRetrieval}`；
  `continuum_trust::label`
- Produces: `continuum_context::compile`

**签名逐字照设计 §5.1**（参数序是 `request, budget, memory`，不是 `request, memory, budget`）：

```rust
pub fn compile(
    request: ContextRequest,
    budget: ContextBudget,
    memory: &dyn MemoryRetrieval,
) -> Result<ContextPackage, MemoryError>;
```

本 task **只实现非记忆的四类来源**：`instructions` 全收（丢空串）、`references` 按输入序
（丢空 `Text`）、`artifacts` 按输入序（暂不去重、暂不截断）。`budget` 与 `memory` 在本 task
暂以 `_budget` / `_memory` 命名（**Task 4／Task 5 会改名并接线**），以免 0 warning 门红。
`ContextPackage` 的唯一构造点在本 task 的 `compile` 末尾；**`src/` 里 `pub struct ContextPackage {`
只在定义处一处，结构体字面量 `ContextPackage {` 只在 `compile` 末尾一处**（J1-d；
`impl ContextPackage {` 不是构造，Task 9 的断言已按此修正，不再用裸 `ContextPackage {` 计数）。

- [ ] **Step 1: 写 `tests/compile_sources.rs` 的十条用例**

- `instructions_are_taken_whole_and_empty_strings_are_dropped`：`instructions` 为
  `["a", "", "b"]`（provenance 均为 `user_instruction()`），预算 `(0,0,0)`，`scopes` 为空；
  断言 `instructions()` 逐项为 `"a"` / `"b"`（`String` 侧比较）、其余五个数组为空。
  **这一条同时是 J11 的指令全收面**：预算 0 不裁指令（设计 §5.3 第 2 条、J11）。
- `whitespace_only_text_is_not_dropped`：`instructions = [" "]`、`references` 放
  `Reference::Text(" ")`；预算 `ContextBudget { max_references: 1, max_artifacts: 0,
  max_memories: 0 }`——**`max_references` 必须非零**，否则 Task 5 的截断会把这条引用丢掉、
  `references().len() == 1` 必红；`scopes` 为空。断言两者都在包里
  （`instructions().len() == 1`、`references().len() == 1`）。**只丢 `is_empty`，不丢纯空白**
  （设计 §2.4 第 6 条）。
- `conversation_history_lands_in_references_with_user_instruction_label`（J4①）：
  `Reference::Conversation(Message { role: Role::User, content: "…".into() })` ＋
  `conversation_history()`；断言它在 `references()`、不在 `instructions()`，标签
  `USER_INSTRUCTION`。
- `project_text_lands_in_references_with_project_data_label`（J4④）：
  `Reference::Text("project".into())` ＋ `project_data()`；断言标签 `PROJECT_DATA`。
- `tool_output_lands_in_references_with_tool_output_label`（J4⑤）：
  `Reference::ToolOutput(ToolResult { .. })` ＋ `tool_output(false)`；断言标签 `TOOL_OUTPUT`。
- `external_provider_text_lands_in_references_with_web_untrusted_label`（J4⑥）：
  `Reference::Text(..)` ＋ `provider_supplied()`；断言标签 `WEB_UNTRUSTED`**不是**
  `TRUSTED_RUNTIME`（设计 §5.4 末）。
- `artifact_ids_land_in_artifacts_with_the_artifact_label`（J4③）：
  `ContextCandidate<ArtifactId>` ＋ `artifact(ArtifactProducer::Project)`；断言 id 在
  `artifacts()`、标签 `PROJECT_DATA`。
- `references_never_leak_into_instructions`（J6＋**J10 的第二半**）：把一条 `web_untrusted()` 的
  `Reference::Text` 放进 `references`、`instructions` 为空；断言 `instructions().is_empty()`
  **且 `references().len() == 1`、第 0 条就是那段文本**——这正是「有引用无指令 ⇒ `instructions[]`
  空、`references[]` 非空」的边界，两半必须同时断言（设计 J10）。**另一半**：把**同一段
  文本**改造成 `ContextCandidate<String>`（provenance 仍取 `web_untrusted()`）放进
  `request.instructions`，断言 `compile` **不拦**（`Ok`，该内容出现在 `instructions()`，
  标签 `WEB_UNTRUSTED`）——这是调用方的错，记录设计 §11 第 20 条。
- `an_empty_request_yields_six_empty_arrays`（J10）：六个字段全空（`scopes` 空、预算任意）；
  断言六个访问器都返回空切片。
- `the_alignment_arrays_are_non_empty_and_equal_length`：1 条指令＋1 条引用＋1 个 artifact；
  只断言 `provenance()` 与 `trust_labels()` **都非空且等长**。**这是装配健全性的弱检查，
  不是 J3**：不断言具体长度、不做逐位映射、不重算 `label()`；J3 的长度／逐位／类别由 Task 6 的
  `validate` 唯一承担（P6B §5.2 第 5 条）。Task 10 的变异表把这条弱检查的红位列在该红的备注里。

**结果：** 十条用例名可列出；所有期望标签都写成 `ContextTrust::…` 的**测试侧**比较值
（J9 的源码面只限 `src/`，测试里比较标签是允许的）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test compile_sources
```

**结果：** 编译失败（`E0432`：`continuum_context::compile` 尚不存在）。**这不是红**；
要取到运行期的红，须先有 `compile` 的签名（本计划不强制走 `todo!()` 那条）。

- [ ] **Step 3: 实现 `compile` 的非记忆装配**

**动作：** 新建 `src/compile.rs`。实现约束逐条如下（设计 §5.3 第 1／2／6 条、§4.2 第 5 条）：

1. 解构 `ContextRequest` 的 `instructions` / `references` / `artifacts`（`scopes` / `anchor` /
   `memory_query` 本 task 用 `..` 忽略）。
2. 为四个内容数组与两个标签数组各建一个局部 `Vec`；**每装入一个片段，按
   `content → provenance → label(&provenance)` 的次序同步 push 一次**。标签先算再 push，
   保证 `trust_labels[i]` 与 `provenance[i]` 同序：
   ```rust
   let label_value = continuum_trust::label(&candidate.provenance);
   contents.push(candidate.content);
   provenance.push(candidate.provenance);
   trust_labels.push(label_value);
   ```
3. `instructions`：`content.is_empty()` 的候选 `continue`；其余全部按输入序 push，**不看 budget**。
4. `references`：`Reference::Text(s)` 且 `s.is_empty()` 的候选 `continue`；**纯空白不丢**
   （设计 §2.4 第 6 条）；其余按输入序 push，**本 task 暂不截断**。
5. `artifacts`：按输入序 push，**本 task 暂不去重、暂不截断**。
6. `memories`：本 task 固定 `Vec::new()`；`let _ = memory;` 或参数名 `_memory`。
7. 末尾**只构造一次** `ContextPackage { instructions, references, artifacts, memories,
   provenance, trust_labels }`，返回 `Ok(..)`。**`compile` 之外不出现 `ContextPackage` 的
   结构体字面量**（`impl ContextPackage {` 是 impl 头，不是构造，不在计数内）。
8. `src/lib.rs` 加 `mod compile;` 与 `pub use compile::compile;`（导出面才可用）。
9. 把 `tests/package.rs` 的 `src_texts()` 从五个文件扩到**六个**（加入 `src/compile.rs`）；
   无投影、`impl ContextPackage` 计数与三个禁止构造入口三条断言随之扫六文件——Task 3 的
   「无投影」照片自此与 Task 9 同一语料，**并断言 `compile.rs` 零个 `impl ContextPackage`**。

**结果：** `src/compile.rs` 的 `ContextPackage {` 出现次数为 1；`tests/package.rs` 的 `src_texts()`
已覆盖六个源文件，`src/compile.rs` 的 `impl ContextPackage` 计数为 0；`compile` 的签名与设计 §5.1
逐字；`src/` 里没有 `ContextPackage {` 的测试构造；`continuum_context::compile` 可从
`tests/` 导入。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test compile_sources
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "feat(context): compile 骨架与五来源映射"
```

**结果：** 十条用例全绿；全量门 0 warning；`compile_sources` 的标签断言逐条对 `label()` 的返回值。

---

### Task 4: Memory 检索、查询构造与 `MemoryError` 透传

> **硬前置：P6C 的 `continuum-memory` 已交付，且已按「遗留/待协调」第 1 条交付
> `#[cfg(feature = "test-support")] #[doc(hidden)] pub fn MemoryClaim::for_test(...)`**（七字段签名逐字见该条）。
> **本 crate 的 `[dev-dependencies]` feature 行是条件步 Step 3b**：P6C 的 `test-support` 未落地前
> **不要加**——Cargo 在解析 manifest 时就会失败，连七条非 claim 用例都跑不了。
> **在 P6C 交付 `for_test` 之前，本 task 的 claim 用例（J4② / J5 的 memories 半 / J8 /
> J14 的反例）一律挂起**，只能先写源码与非 claim 用例；**不许**用 `unsafe`、伪造
> `MemoryClaim` 或直接写私有字段绕过。

**Files:**
- Create: `crates/continuum-context/tests/support/mod.rs`
- Create: `crates/continuum-context/tests/memory.rs`
- Modify: `crates/continuum-context/src/compile.rs`（加记忆分支；`_budget` / `_memory` 改名）
- Modify: `crates/continuum-context/Cargo.toml`（**只由条件步 Step 3b 改**：预检通过后才在
  `[dev-dependencies]` 加 `continuum-memory = { path = "../continuum-memory", features = ["test-support"] }`；
  预检不通过则**本文件一字不改**）

**Interfaces:**
- Consumes: `continuum_memory::{Confidence, MemoryClaim, MemoryClaimId, MemoryError,
  MemoryRetrieval, MemoryScope, MemorySourceId, RetrievalQuery, Validity}`；Task 2 的
  `ContextRequest.scopes` / `anchor` / `memory_query`；`continuum_trust::ContextProvenance::memory`
- Produces: `compile` 的记忆分支；`tests/support` 的假检索面

**`tests/support/mod.rs` 的形状**（所有记忆用例共用，**不写进 `src/`**；用到它的
integration test 顶部写 `mod support;`）：

```rust
#![allow(dead_code)] // 各集成测试各自 `mod support;`，不是每个 helper 都被每个目标用到。

pub struct CountingMemory {
    claims: Vec<MemoryClaim>,
    fail_with_empty_scope_set: bool,
    calls: std::cell::Cell<usize>,
    last_query: std::cell::RefCell<Option<QuerySnapshot>>,
}

#[derive(Clone)]
pub struct QuerySnapshot {
    pub scopes: Vec<MemoryScope>, // P6C 的 MemoryScope 需可 Clone；若未派生，改存 as_str 的 String
    pub anchor: Option<String>,
    pub text: Option<String>,
    pub limit: usize,
    pub include_terminal: bool,
}

impl CountingMemory {
    pub fn new(claims: Vec<MemoryClaim>) -> Self;
    pub fn failing_with_empty_scope_set() -> Self;
    pub fn calls(&self) -> usize;
    pub fn last_query(&self) -> Option<QuerySnapshot>; // 快照 Clone 后返回
}
```

`retrieve(&self, query: &RetrievalQuery)` 里：`calls.set(calls.get()+1)`；把 `query.scopes()` /
`anchor()` / `text()` / `limit()` / `include_terminal()` 拷进 `QuerySnapshot`；
`fail_with_empty_scope_set` 为真时返回 `Err(MemoryError::EmptyScopeSet)`，否则返回
`Ok(self.claims.clone())`。**错误不用 `Option<MemoryError>` 存**：P6C 的 `MemoryError` 没有
`Clone` 承诺（它含 `Persist(PersistError)`），存构造参数而不是存错误值。
**claim 夹具**：`fn test_claim(id: &str, statement: &str) -> MemoryClaim` 调 P6C 的
`MemoryClaim::for_test(...)`（`#[cfg(feature = "test-support")]`，**待 P6C 交付**）。七字段按
P6C §5.1 填：
`MemoryClaimId::new(id)` / `MemoryScope::Personal` / `statement` /
`vec![MemorySourceId::new(format!("src-{id}"))]` / `Validity::Active` /
`Confidence::from_value(serde_json::json!(1.0))` / `0`；`for_test` 返回 `MemoryClaim`
（不返回 `Result`），helper 直接返回。
**P6C 未交付 `for_test` 时 helper 以 `E0599`（方法不存在）编不过；若退回调 `new` 则是 `E0624`。
因此 `test_claim` helper 与五条 claim 用例同批挂起（Step 3b 预检通过后才写入）：未交付时不要把这个
helper 写进 `tests/support/mod.rs`**；其余 helper（`CountingMemory` / `QuerySnapshot`）不依赖 claim 构造，
照常写，非 claim 用例传 `CountingMemory::new(vec![])`。不伪造 claim、不用 `unsafe`、
不反序列化私有字段。

- [ ] **Step 1: 先写 `tests/memory.rs` 的七条非 claim 用例（目标十二条；五条 claim 用例留到 Step 3b）**

- `a_claim_from_the_retrieval_surface_lands_in_memories`（J4②）：`scopes = vec![Personal]`、
  `max_memories = 3`，假实现返回 claim C；断言 `memories()` 含 C、其 provenance 是
  `Memory { claim_id }`、标签 `MEMORY`；`instructions()` / `references()` / `artifacts()` 空。
  **挂起直到 P6C 交付 `for_test`。**
- `the_retrieval_surface_is_not_called_when_scopes_are_empty`（J14 的第一侧）：
  `scopes = vec![]`、`max_memories = 1`、`memory_query = Some("x")`；断言计数 0、`memories()` 空。
- `the_retrieval_surface_is_not_called_when_max_memories_is_zero`（**Review Focus 1**）：
  `scopes = vec![Personal]`、`max_memories = 0`、`memory_query = Some("x")`；断言计数 0、
  `memories()` 空。**这一条专门推翻「有查询文本就会检索」的错读**。
- `max_memories_one_calls_the_surface_once`（J14 的反例）：`scopes = vec![Personal]`、
  `max_memories = 1`；断言计数 1、`memories()` 长度为 1。**挂起直到 P6C 交付 `for_test`。**
- `the_query_carries_scopes_anchor_text_limit_and_terminal_false`（设计 §5.3 第 5 条）：
  `scopes = vec![Project]`、`anchor = Some("p1".into())`、`memory_query = Some("x")`、
  `max_memories = 2`；从 `QuerySnapshot` 断言五项逐一相等（尤其 `anchor == Some("p1")`），
  `include_terminal == false`、`limit == 2`。假实现返回空 `Vec`，**不需要 claim**。
- `a_query_construction_rejection_is_passed_through`（J13-a）：`scopes = vec![Project]`、
  `anchor = None`、`max_memories = 1`；断言 `compile` 返回
  `Err(MemoryError::AnchorRequired)`（用 `matches!` 判**同一臂**），且 `Err` 分支没有
  `ContextPackage` 逃出。
- `an_anchor_on_an_unanchored_scope_is_rejected`（J13 的补测）：`scopes = vec![Personal]`、
  `anchor = Some("p".into())`、`max_memories = 1`；断言 `compile` 返回
  `Err(MemoryError::AnchorNotAllowed)`（**同一臂**）。设计 §8.2 J13 只要求两条代表路径；
  这一条按 `RetrievalQuery::new` 的三臂补齐，不需要 claim。
- `a_retrieval_failure_is_passed_through`（J13-b）：`scopes = vec![Personal]`，假实现返回
  `Err(MemoryError::EmptyScopeSet)`；断言 `compile` 返回 `Err(MemoryError::EmptyScopeSet)`
  （**同一臂**，不是被包装成 `ContextError`）。
- `an_empty_memory_query_still_retrieves_and_normalizes_to_none`（§5.2 的补测）：
  两份请求都设 `scopes = vec![Personal]`、`max_memories = 3`，`memory_query` 分别为 `None` 与
  `Some("")`；**每次编译各用一个独立的 `CountingMemory`**（两个假实现各从 0 起），分别断言
  `calls() == 1`——**不要用同一个计数器写「计数 1／1」**（既可读成各 1，也可读成累计 1）；
  快照里的 `text` 都是 `None`（`Some("")` 由 P6C 的 `new` 归一，P6A 原样传递）；假实现返回
  空 `Vec`，**不需要 claim**。**对照的是「`Some("")` 不是不检索的信号」**，不是 P6A 自己归一。
- `duplicate_claim_ids_keep_the_first`（设计 §5.3 第 5 条）：假实现返回 `[A(id=c1, statement=first),
  A(id=c1, statement=second), B(id=c2)]`、`max_memories = 3`；断言 `memories()` 长度为 2，
  且 `c1` 的 `statement()` 是 `first`（首个保留）。**挂起直到 P6C 交付 `for_test`。**
- `memories_are_truncated_to_max_memories`（J5 的 memories 半）：3 条不同 id 的 claim、
  `max_memories = 2`；断言长度为 2、第 3 条不在包中。**挂起直到 P6C 交付 `for_test`。**
- `memories_come_only_from_the_retrieval_surface`（J8）：请求里没有任何 claim 输入；
  假实现返回 C 时 `memories() == [C]`，不调检索时 `memories()` 空；计数与返回逐次对应。
  **挂起直到 P6C 交付 `for_test`。**

**结果：** 七条非 claim 用例名可列出；五条 claim 用例（J4②、`max_memories_one_calls_the_surface_once`、
`duplicate_claim_ids_keep_the_first`、`memories_are_truncated_to_max_memories`、
`memories_come_only_from_the_retrieval_surface`）与 `test_claim` helper **不在本步写入**，
留到 Step 3b 预检通过后补；`tests/support` 只被 `tests/` 使用，`src/` 没有测试代码。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test memory
```

**结果：** **运行期红**（Task 3 的 `compile` 忽略 `memory`：`memories()` 空、计数 0）。
**Step 3b 尚未执行时**，五条 claim 用例与 `test_claim` helper **不写入**（因此本 target 仍可编译，
`Cargo.toml` 也没有 `test-support` 行，只跑非 claim 的七条）；**若执行者抢先把它们写进去**，会以
`E0599`（方法不存在；若退回调 `new` 则是 `E0624`）让整个 target 编不过，按纪律 1(c) **不算红**。
本 task 只报七条，claim 五条挂起并回报。

- [ ] **Step 3: 实现记忆分支**

**动作：** 修改 `src/compile.rs` 的 `compile`：

1. 把 `_budget` / `_memory` 改名为 `budget` / `memory`；解构 `ContextRequest` 时取出
   `scopes` / `anchor` / `memory_query`。
2. **只在 `!scopes.is_empty() && budget.max_memories > 0` 时**构造查询：
   ```rust
   let query = RetrievalQuery::new(
       scopes,
       anchor,
       memory_query,
       budget.max_memories,
       false,
   )?;
   let claims = memory.retrieve(&query)?;
   ```
   **`?` 直接透传 `MemoryError`**；不写 `ContextError`、不 `map_err`。
3. 对返回的 claim 按 `claim.id().as_str()` 去重（`HashSet<String>`，首个保留），并截断到
   `budget.max_memories`（先判长度再 push）。每个保留的 claim：
   ```rust
   let provenance = ContextProvenance::memory(claim.id().as_str());
   let label_value = continuum_trust::label(&provenance);
   memories.push(claim);
   provenance_out.push(provenance);
   trust_labels.push(label_value);
   ```
   **provenance 必须在移动 `claim` 之前构造**（借 `id()`）。
4. `scopes` 空或 `max_memories == 0` 时**不进这个分支**：不构造 `RetrievalQuery`、
   不调 `retrieve`、`memories` 保持空 `Vec`。

**结果：** 非 claim 七条全绿（若 Step 3b 未执行，claim 五条仍挂起、未写入）；`src/compile.rs` 里
`RetrievalQuery::new` 的五个实参与设计 §5.3 第 5 条逐字一致；`include_terminal` 的字面量是 `false`。

- [ ] **Step 3b（条件步）: 预检 P6C 的 `test-support`，通过后才加 feature 行与五条 claim 用例**

**先预检，不通过就整步跳过（不改 `Cargo.toml`、不写 `test_claim`、不写五条 claim 用例）**：

```bash
grep -n '^test-support *=' crates/continuum-memory/Cargo.toml
grep -n 'pub fn for_test' crates/continuum-memory/src/claim.rs
grep -n '#\[doc(hidden)\]' crates/continuum-memory/src/claim.rs
cargo metadata --format-version 1 --no-deps >/dev/null
```

**判读**：前三条 grep（`test-support` feature 行／`pub fn for_test`／`#[doc(hidden)]`）任一零命中
⇒ P6C 尚未交付 `test-support`／`for_test`，**本步不执行**；
`cargo metadata` 非零退出 ⇒ P6C 的 manifest 本身不可解析，同样不执行。要确认 feature 真在
（而不仅是 manifest 能读），用
`cargo metadata --format-version 1 --no-deps | grep '"test-support"'`，或改一行后在
`cargo tree -p continuum-context -e features | grep continuum-memory` 里看到 `test-support`。

**预检通过后，三件事必须同批落**（缺一条都会让 target 编不过或口径不完整）：

1. 在 `crates/continuum-context/Cargo.toml` 的 `[dev-dependencies]` 加
   `continuum-memory = { path = "../continuum-memory", features = ["test-support"] }`；
2. 在 `tests/support/mod.rs` 加 `test_claim` helper（七字段签名见「遗留/待协调」第 1 条）；
3. 在 `tests/memory.rs` 补齐五条 claim 用例（去掉「挂起」标记）。

**为什么必须同批**：Cargo 解析 `[dev-dependencies]` 的 feature 发生在编译前；P6C 没有该 feature
时加这一行会让整个 `--test memory` target 连七条非 claim 用例都编译不了，Task 10 的
「非 claim 面全绿＋claim 面挂起」就不可判定。**预检不通过时的回报**：
「非 claim 七条全绿＋claim 五条挂起（等 P6C `test-support`／`for_test`）」，并注明
`Cargo.toml` 无新增行。

**结果：** 预检不通过 → `Cargo.toml` 无新增行、`tests/support/mod.rs` 无 `test_claim`、
`tests/memory.rs` 只有七条；预检通过 → 三件事同批完成，`cargo test --test memory -- --list`
可列出十二条。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test memory
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "feat(context): 记忆检索、查询构造与 MemoryError 透传"
```

**结果：** 十二条全绿（未执行 Step 3b 时只报非 claim 七条全绿，claim 五条挂起且 `Cargo.toml`
无 feature 行）；全量门 0 warning；`Err` 三条用例（`AnchorRequired`／`AnchorNotAllowed`／检索失败）
判的是**同一臂**，而不是「只要是 Err」。

---

### Task 5: 预算、截断与去重

**Files:**
- Create: `crates/continuum-context/tests/budget.rs`
- Modify: `crates/continuum-context/src/compile.rs`（references／artifacts 的预算与去重）

**Interfaces:**
- Consumes: Task 3／Task 4 的 `compile`；Task 2 的 `ContextBudget`
- Produces: 无新增公开类型；`compile` 的预算与去重行为

**预算的顺序照设计 §5.3**：先丢空串（第 1 条），再按输入序取前 `max_references` 条（第 3 条）；
artifacts **先去重后截断**（第 4 条）；memories 的去重与截断已在 Task 4（第 5 条）。
**三个上限互相独立**；`max_* == 0` 不是错误（设计 §5.6）。

- [ ] **Step 1: 写 `tests/budget.rs` 的用例（目标六条；`for_test` 未交付时先写四条）**

- `references_are_truncated_in_input_order`（J5）：两条不同内容的 `Reference::Text`、
  `max_references = 1`；断言只保留第 1 条、第 2 条的内容不在 `references()`。
- `artifact_ids_are_deduped_then_truncated`（J5）：id 序列 `A, A, B`、`max_artifacts = 2`；
  断言 `artifacts() == [A, B]`（重复的 A 静默丢弃，不是错误）。另设一例 `A, B`、
  `max_artifacts = 1`，断言 `[A]`。
- `the_three_budgets_are_independent`（J5）：`max_references = 1`、`max_artifacts = 0`、
  `max_memories = 2`，各给 2 条候选；断言三个数组长度分别是 `1 / 0 / 2`。
  **挂起直到 P6C 交付 `for_test`**（memories 的 2 条要 claim）。
- `zero_budgets_do_not_drop_instructions`（J11）：两条非空指令、预算 `(0,0,0)`、
  `scopes = vec![Personal]`、`max_memories = 0`、`memory_query = Some("x")`；断言
  `instructions().len() == 2`、其余五个数组空、假实现计数 0。**不需要 claim**。
- `truncated_candidates_do_not_appear_anywhere`（J5）：被截断的 reference 的文本、被截断的
  artifact id、被截断的 claim id 都**不出现在六个数组的任何位置**（逐个字段扫）。
  **挂起直到 P6C 交付 `for_test`**（claim id 那一半要 claim）。
- `duplicate_artifact_ids_are_not_an_error`（设计 §5.6）：重复 id 的请求返回 `Ok`，
  `artifacts()` 只出现一次；**不得**返回 `Err`。

**结果：** 目标六条用例名可列出（`for_test` 未交付时先四条）；期望值手写，不从 `compile` 的
输出现场反推。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test budget
```

**结果：** **运行期红**：Task 3／Task 4 尚未截断 references／artifacts，前两条与第三条的
references／artifacts 断言失败；`instructions` 与 memories 的断言可能已绿。**这不是编译失败**。
**两条 claim 用例（`the_three_budgets_are_independent`／`truncated_candidates_do_not_appear_anywhere`）
在 P6C 交付 `for_test` 前挂起**，本步只报其余四条。

- [ ] **Step 3: 实现预算与去重**

**动作：** 修改 `src/compile.rs` 的两个循环：

1. `references`：在丢空串之后，用 `if references_out.len() >= budget.max_references { break; }`
   截断；**先过滤再计数**，空串不占预算位。
2. `artifacts`：用 `HashSet<String>` 记录 `candidate.content.as_str()`；已出现过就 `continue`；
   再用 `if artifacts_out.len() >= budget.max_artifacts { break; }` 截断。
3. `instructions`：**不加任何预算判断**（设计 §4.3 第 5 条、J11）。
4. memories：Task 4 已按 `max_memories` 截断；本 task 不改。

**结果：** `budget` 六条全绿（claim 两条须等 P6C 交付 `for_test`）；`compile` 里出现三处
`budget.max_*`，其中 `max_references` / `max_artifacts` 在本 task 新加，`max_memories` 在
Task 4 已加；`instructions` 循环里没有 `budget`。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test budget
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "feat(context): 预算、截断与去重"
```

**结果：** 六条全绿（未交付 `for_test` 时只报非 claim 四条全绿，两条 claim 用例挂起）；
全量门 0 warning。

---

### Task 6: 标签对齐与 P6B `validate` 的独立复核

**Files:**
- Create: `crates/continuum-context/tests/labels.rs`

**Interfaces:**
- Consumes: Task 1–Task 5 的 `compile`；`continuum_trust::{validate, PackageShape,
  TrustError, ContextProvenance, ContextTrust}`
- Produces: 无生产代码（本 task 只交付复核用例）

**本 task 是 J3 的落点，也是 Review Focus 2／3 的落点。** 设计 §8.2 J3 逐字要求：正例用
`PackageShape { instructions: 1, references: 2, artifacts: 1, memories: 1 }` 调
`continuum_trust::validate(shape, pkg.provenance(), pkg.trust_labels())` 并断言 `Ok`；
**J3 不自己写第二份长度／逐位断言**（P6B §5.2 第 5 条）。反例直接调 `validate`，不构造包。

- [ ] **Step 1: 写 `tests/labels.rs` 的用例（目标六条；`for_test` 未交付时先写五条）**

- `the_package_passes_the_independent_validate`（J3 正例）：`compile` 产出
  1 条当前指令（`user_instruction()`）＋1 条历史用户轮（`conversation_history()`，进
  `references`）＋1 条 `Reference::Text`（`web_untrusted()`，进 `references`）＋1 个 artifact
  ＋1 条 claim；用 `PackageShape { instructions: 1, references: 2, artifacts: 1, memories: 1 }`
  调 `validate`，断言 `Ok(())`。**挂起直到 P6C 交付 `for_test`**（claim 那一条要夹具）。
- `relabeling_a_reference_changes_its_label_in_place`（J3 的行为断言）：把同一条
  `Reference::Text` 的 provenance 从 `web_untrusted()` 换成 `project_data()` 重新 `compile`；
  断言 `references()` 第 0 条的标签由 `WEB_UNTRUSTED` 变为 `PROJECT_DATA`，且**读取偏移是**
  `instructions().len() + j`（设计 §4.2 第 6 条）。
- `validate_rejects_a_label_that_does_not_match_its_provenance`（J3 反例 a）：
  `validate(PackageShape { instructions: 0, references: 1, artifacts: 0, memories: 0 },
  &[WebUntrusted], &[ContextTrust::ProjectData])` ⇒
  `Err(TrustError::LabelDoesNotMatchProvenance { .. })`。
- `validate_rejects_a_current_instruction_in_a_data_slot`（J3 反例 b）：
  `provenance = [UserInstruction { historical: false }]`、references 位 ⇒
  `Err(TrustError::DataSlotCarriesUserInstruction { .. })`。
- `validate_rejects_a_historical_instruction_in_the_instruction_slot`（J3 反例 c）：
  `provenance = [UserInstruction { historical: true }]`、instructions 位 ⇒
  `Err(TrustError::HistoricalInstructionInInstructions { .. })`。
- `compile_does_not_judge_historical_but_validate_does`（**Review Focus 3**）：
  把 `conversation_history()` 的候选放进 `request.instructions`；断言 `compile` 返回 `Ok`
  且该内容在 `instructions()`（本块**不判** `historical`）；再对这个包调 `validate`，
  断言 `Err(HistoricalInstructionInInstructions { .. })`。**这一条把「生产路径不判、复核面判」
  的边界钉死**（设计 §5.4 末、§7.1）。

**结果：** 目标六条用例名可列出（`for_test` 未交付时先五条）；所有反例都判**具体的
`TrustError` 臂**，不写「只要是 Err 就算过」。
**J3 不写第二份长度／逐位断言**：长度、逐位映射与类别全部交给 `validate`（P6B §5.2 第 5 条）；
`relabeling_…` 只断言「换 provenance 后标签跟着变」这一条**行为**。

- [ ] **Step 2: 运行（本 task 无生产代码，预期首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test labels
```

**结果：** **预期是绿**：断言的是 Task 3／Task 4 已交付的同步 push 与 P6B 的 `validate`。
若红了，先查 `compile` 的 push 顺序或 P6B 的 `validate`，不是本用例写错。
**J3 正例（`the_package_passes_the_independent_validate`）在 P6C 交付 `for_test` 前挂起**；
本步只报其余五条。**不得把挂起计为绿。**

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付用例**。**不建**第二份 `validate`、**不建**按字段切片的标签访问器、
**不给 `compile` 加 `validate` 调用**（设计 §7.1：生产路径不调用 `validate`，只有 J3 调它）。
`compile` 的对齐由 Task 3／Task 4 的同步 push 保证。

**结果：** `src/` 里没有 `validate(` 的调用（`grep -n "validate(" crates/continuum-context/src`
零命中）；`tests/labels.rs` 是唯一的调用点。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "test(context): 标签对齐与 P6B validate 的独立复核"
```

**结果：** 全量门 0 warning；`labels` 六条全绿（未交付 `for_test` 时五条全绿＋J3 正例挂起，
不得报六条全绿）。

---

### Task 7: 无隐藏来源与确定性

**Files:**
- Create: `crates/continuum-context/tests/purity.rs`

**Interfaces:**
- Consumes: Task 1–Task 5 的 `compile`；`tests/support` 的假检索面
- Produces: 无生产代码（本 task 只交付用例）

**本 task 是 J7a／J7b 的落点，也是 Review Focus 4 的第一半。** 设计 §5.5 的可测面是：
`compile` 只依赖三个参数；请求外的 `ArtifactId` / claim 不出现；两份逐字段相同的请求在同一检索面下
产物相等。

- [ ] **Step 1: 写 `tests/purity.rs` 的用例（目标三条；`for_test` 未交付时先写一条）**

- `only_requested_artifacts_and_returned_claims_appear`（J7a）：请求只含 artifact A；假检索面
  返回 claim C；**另造一枚 claim D 只作为测试变量、不交给假实现**；断言 `artifacts()` 只含 A、
  `memories()` 只含 C、**D 不在包中**；再断言一个未出现在请求里的 artifact B 不在包中。
  **挂起直到 P6C 交付 `for_test`。**
- `two_identical_requests_produce_equal_packages`（J7b）：**手写两份逐字段相同**的
  `ContextRequest`（**不用 `Clone`**），配同一个假检索面，调两次；断言
  `p1 == p2`（`ContextPackage` 派生 `PartialEq`）。**挂起直到 P6C 交付 `for_test`**
  （假检索面要返回 claim，否则这条只测空包）。
- `compile_reads_no_clock_environment_or_filesystem`（Review Focus 4 的第一半）：
  读 `include_str!("../src/compile.rs")`，断言零命中 `std::env` / `env::var` /
  `SystemTime` / `Instant::now` / `current_dir` / `std::fs` / `File::open`。
  **作用域写死**：只扫 `compile.rs`，不扫 `tests/`（测试里读源码是允许的）。

**结果：** 目标三条用例名可列出（`for_test` 未交付时先一条）；J7a／J7b 的请求都手写，
不从同一个值派生。

- [ ] **Step 2: 运行（本 task 无生产代码，预期首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test purity
```

**结果：** **预期是绿**：`compile` 在 Task 3／Task 4 已交付为纯函数。若 J7b 红，先查假实现是否
每次返回同一批 claim（`Clone`），不是先改 `compile`。**J7a／J7b 两条在 P6C 交付 `for_test`
前挂起**；本步只报 `compile_reads_no_clock_environment_or_filesystem` 一条。不得把挂起计为绿。

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付用例**。**不建**缓存、**不建**「当前 Agent／当前会话」的隐式状态、
**不给 `compile` 加任何参数**。`compile` 的三个参数就是全部输入（设计 §5.5）。

**结果：** `src/` 里没有 `std::env` / `SystemTime` / `Instant::now` / `std::fs`；`compile` 签名
仍是三个参数。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "test(context): 无隐藏来源与确定性"
```

**结果：** 全量门 0 warning；`purity` 三条全绿（未交付 `for_test` 时一条全绿＋J7a／J7b 挂起，
不得报三条全绿）。

---

### Task 8: `ContextPackage` 唯一构造点的 trybuild 照片

> **硬前置：P6B 与 P6C 的 crate 已落地**（`ContextPackage` 的字段类型要能编译）。
> **本 task 不改 P6B／P6C 的任何文件**：它只在本 crate 的 `tests/` 里写样例。

**Files:**
- Create: `crates/continuum-context/tests/type_level.rs`
- Create: `crates/continuum-context/tests/compile_fail/package_literal_is_private.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-context/tests/compile_fail/package_default_is_absent.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-context/tests/compile_fail/package_deserialize_is_absent.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-context/tests/compile_fail/package_fields_are_read_only.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-context/tests/ui_pass/package_is_reachable.rs`
- Modify: `crates/continuum-context/Cargo.toml`（dev-dep `trybuild`）

**Interfaces:**
- Consumes: `continuum_context::ContextPackage`（**使用侧**）
- Produces: 无生产类型

**四份 `compile_fail` 覆盖 J1-a／J1-b／J1-c 与 J2 的外部写字段；一份 `ui_pass` 是正控制。**
**`.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；四份各不相同，不许拿一个值套四处。**
trybuild 把样例源码逐行打印进 `.stderr`，**改一句注释也会改 `.stderr`**。

- [ ] **Step 1: 写样例与驱动**

- `package_literal_is_private.rs`（J1-a）：
  ```rust
  use continuum_context::ContextPackage;

  fn main() {
      let _ = ContextPackage {
          instructions: vec![],
          references: vec![],
          artifacts: vec![],
          memories: vec![],
          provenance: vec![],
          trust_labels: vec![],
      };
  }
  ```
  字段私有 ⇒ 编译失败。**样例里不 `use` 任何 P6B／P6C 的类型**，让失败只来自私有字段。
- `package_default_is_absent.rs`（J1-b）：
  ```rust
  use continuum_context::ContextPackage;

  fn main() {
      let _ = ContextPackage::default();
  }
  ```
- `package_deserialize_is_absent.rs`（J1-c）：
  ```rust
  fn main() {
      let _: continuum_context::ContextPackage =
          serde_json::from_str("{}").expect("…");
  }
  ```
  `serde_json` 是 dev-dep；`ContextPackage` 没有 `Deserialize` ⇒ 编译失败。
- `package_fields_are_read_only.rs`（J2 的编译期半）：
  ```rust
  fn main() {
      let mut package: continuum_context::ContextPackage = todo!();
      package.instructions = vec![];
  }
  ```
  字段私有 ⇒ 写入编译失败。
- `ui_pass/package_is_reachable.rs`：一个只接收 `&ContextPackage` 并调用 `instructions()` 的函数，
  **加上 `fn main() {}`**（trybuild 的 `pass` 样例按**二进制**编译，缺 `main` 会 `E0601`，
  与「依赖没接上」或「字段私有」的失败混在一起）；**不构造值**。它的用途只有一个：把
  「因为依赖没接上而编译失败」与「因为字段私有而编译失败」分开。
- `tests/type_level.rs`：`trybuild::TestCases` 逐份 `compile_fail` ＋ `pass`。

**结果：** 五份样例文件与一份驱动文件存在；四份 `.stderr` 尚未写（Step 2 才取）。

- [ ] **Step 2: 运行，确认失败并取 `.stderr`**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test type_level
```

**结果：** **运行期红**（trybuild 报样例与 `.stderr` 不符，或 `.stderr` 缺失）。把**实跑输出**
逐份写进同名 `.stderr`，再跑一次确认绿。**不许先生成 `.stderr` 再写样例；不许四份共用错误码。**

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付样例与一条 dev 依赖**。**不加** `Default`、**不加** `Deserialize`、
**不开** `pub` 字段、**不加** `new` / `from_parts`——四份样例就是「没有第二个构造点」的使用侧照片。
**不把 `trybuild` 写进 `ALLOWED`**：它是外部 crate，那张表只断言 workspace 成员之间的边。

**结果：** `Cargo.toml` 的 `[dev-dependencies]` 是 `serde_json` / `trybuild`；**仅当 Task 4 Step 3b
预检通过**，才外加 `continuum-memory`（`features = ["test-support"]`，同一条 P6C 边的测试 feature）；
Step 3b 未执行时**没有**这一行。`src/` 一字未动。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context Cargo.lock
git commit -m "test(context): ContextPackage 唯一构造点的 trybuild 照片"
```

**结果：** `type_level` 全绿；四份 `.stderr` 各取自实跑；全量门 0 warning。

---

### Task 9: 源码面 `J1-d` / `J9`

**Files:**
- Create: `crates/continuum-context/tests/source_surface.rs`

**Interfaces:**
- Consumes: 本 crate 的 `src/` 文本（`include_str!`）
- Produces: 无生产代码（本 task 只交付源码面断言）

**本 task 是 J1-d 与 J9 的落点，也是 Review Focus 4 的第二半、Review Focus 5 的第二半。**
**零命中类断言会随注释漂**（切分 §六的教训）：本 task 的每个断言只针对**具体的词根/模式**，
不做「整份文件没有任何…」的泛化；命中行要在 Step 2 逐处读。

- [ ] **Step 1: 写 `tests/source_surface.rs` 的五条用例**

用一个 `fn src_text() -> String` 拼接 `include_str!` 的六个源文件（`lib.rs` / `package.rs` /
`reference.rs` / `request.rs` / `candidate.rs` / `compile.rs`）。

- `context_package_is_constructed_only_in_compile`（J1-d，**修正计数口径**）：
  ① `src_text().matches("pub struct ContextPackage {").count() == 1`——**定义处一处**；
  ② `include_str!("../src/compile.rs").matches("ContextPackage {").count() == 1`——**唯一构造一处**；
  ③ 把全部源码行按「含 `ContextPackage {` 且 `trim_start()` 不以 `impl ` 开头」过滤，计数**恰为 2**
  （定义＋构造）；**`impl ContextPackage {` 是 impl 头，不算构造**——旧写法
  `matches("ContextPackage {").count() == 2` 会把 `src/package.rs` 的 `impl` 算进去，必然红。
  ④ 逐文件断言 `impl ContextPackage {` 计数：`src/package.rs` **恰 1**，`src/lib.rs` /
  `src/reference.rs` / `src/request.rs` / `src/candidate.rs` / `src/compile.rs` **各 0**
  （六个源文件合计恰 1；`compile.rs` 零个 impl 是 J1-d 的明确一半）；
  ⑤ 逐文件断言不含 `pub fn new` / `from_parts` / `from_fields`（三个禁止构造入口扫六文件，
  与 Task 1 的同一断言在终态复跑）。
  并断言 `!contains("impl Default for ContextPackage")`、
  `!contains("for ContextPackage")`（任何 trait impl，含 `impl<'de> Deserialize<'de> for
  ContextPackage`）、`!contains("#[cfg(test)]")`。
  **注释里不要写 `ContextPackage {` 这个字面量**，否则 ③ 会大于 2（这是本断言的语料纪律）。
- `the_crate_defines_none_of_p6b_or_p6c_types`（J9）：
  `!contains("enum ContextTrust")`、`!contains("enum ContextProvenance")`、
  `!contains("struct MemoryClaim")`、`!contains("enum MemoryScope")`、
  `!contains("impl ContextTrust")`、`!contains("impl ContextProvenance")`、
  `!contains("impl MemoryClaim")`、`!contains("NodeTrust")`、`!contains("PrivacyClass")`；
  并断言 `!contains("ContextError")`（本块不定义自己的错误类型；设计 §2.2、§5.6）。
  **注释里也不要写 `ContextError` 这个字面量**，否则这条会随注释漂。
- `labels_are_not_constructed_in_this_crate`（J9）：
  `!contains("ContextTrust::")`（`src/` 里没有构造字面量；测试侧的比较值在 `tests/`，不在语料里）；
  `contains("use continuum_trust")`（类型来源是 P6B，至少一处）。
- `there_is_no_projection_serialization_or_persistence_surface`（Review Focus 5 的第二半）：
  `!contains("to_messages")`、`!contains("into_invoke_request")`、`!contains("InvokeRequest")`、
  `!contains("impl Serialize")`、`!contains("impl Deserialize")`、
  `!contains("Serialize for ContextPackage")`、`!contains("Deserialize for ContextPackage")`、
  `!contains("serde::Serialize")`、`!contains("serde::Deserialize")`、
  `!contains("continuum_persist")`、`!contains("continuum_events")`。
  **不要写成 `!contains("Serialize")` / `!contains("Deserialize")`**：`src/lib.rs` 的文档注释
  逐字写着「不实现 `Default` / `Deserialize`」（设计 §4.1 的原话），裸词会命中注释——那是
  **说明**，不是实现。
- `compile_reads_no_hidden_source`（Review Focus 4 的第二半）：
  `!contains("std::env")`、`!contains("env::var")`、`!contains("SystemTime")`、
  `!contains("Instant::now")`、`!contains("current_dir")`、`!contains("std::fs")`、
  `!contains("File::open")`。

**结果：** 五条用例名可列出；每条断言的模式都写死在用例里，不从源码派生。

- [ ] **Step 2: 运行（本 task 无生产代码，预期首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-context --test source_surface
```

**结果：** **预期是绿**（前提是 J1-d 用上面的两段计数；旧的裸 `ContextPackage {` 计数会因
`impl ContextPackage {` 必红）。若红了，逐处读命中行：是源码真的违反了设计，还是注释里出现了
被禁的字面量（后者改注释，不改断言）；**不许为了绿而删断言**。

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付源码面断言**。**不建**代码生成、**不建** lint、**不新增**依赖。
`ContextPackage` 的唯一构造点、标签来源、无投影面由 Task 1–Task 8 的源码保证，本 task 只是把它们
变成可跑的断言。

**结果：** `tests/source_surface.rs` 是唯一新增文件；`src/` 一字未动。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-context
git commit -m "test(context): 源码面 J1-d/J9"
```

**结果：** 五条全绿；全量门 0 warning。

---

### Task 10: 收尾与复核

**Files:**
- Modify: `crates/continuum-context/**`（**仅在复核发现缺口时**）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**仅在对账不齐时**）

- [ ] **Step 1: 全量验证（门读数按 P5c 纪律 3 读）**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p6a-final-test.log
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets 2>&1 | tee .tmp/p6a-final-build.log
```

**结果：** 全绿、0 warning。**判据不是退出码**：逐条读日志里 **`Running` 的行数**与
**`test result:` 的行数**（Doc-tests 算一条），确认射程覆盖全部测试目标；`--no-fail-fast` 必须带上。
**若 P6C 尚未交付 `for_test`，本步不得报全绿**：Task 4–7 的 claim 用例按「挂起」计（测试体不写入，
见跨计划前置第二条），报告只能写「非 claim 面全绿＋claim 面挂起（等 P6C `for_test`）」。

- [ ] **Step 2: `ALLOWED`、`Cargo.toml` 与迁移的终态核对**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test migrations
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test startup
```

**逐条核四件事**：
（a）`continuum-context` 的 `ALLOWED` 条目**恰为四项**：`continuum-artifact` / `continuum-core` /
`continuum-memory` / `continuum-trust`，一个不多一个不少；
（b）`crates/continuum-context/Cargo.toml` 的 `[dependencies]` 与 `ALLOWED` **精确一致**（四条）；
`[dev-dependencies]` 里**仅当 Task 4 Step 3b 预检通过后**才出现
`continuum-memory = { path = "../continuum-memory", features = ["test-support"] }`——**是同一条边的
测试 feature，不是第五条边**；未执行 Step 3b 时这一行不存在，`[dev-dependencies]` 只有
`serde_json` / `trybuild`；
**外部 crate** `serde_json` / `trybuild` 只出现在 `[dev-dependencies]`，不进表；
（c）**没有**指向 `continuum-graph` / `continuum-operator` / `continuum-provider` /
`continuum-runtime` / `continuum-semantics` / `continuum-persist` / `continuum-events` /
`continuum-policy` / `continuum-capability` / `continuum-secrets` / P5 六 crate 的边；
（d）**本块没有动任何迁移**：`runtime_migrations()` 一字未改，`migrations` / `startup` 仍绿，
P6A 的零表在运行期可见（设计 §10）。

**结果：** 三道门全绿；`ALLOWED` 的 `continuum-context` 行与 `Cargo.toml` 逐项对读一致；
`git diff -- crates/continuum-runtime/src/main.rs` 为空。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
grep -rn "pub struct ContextPackage {" crates/continuum-context/src
grep -n "ContextPackage {" crates/continuum-context/src/compile.rs
grep -rn "ContextPackage {" crates/continuum-context/src | grep -vE '^[^:]+:[0-9]+:[[:space:]]*impl '
grep -rn "enum ContextTrust\|enum ContextProvenance\|struct MemoryClaim\|enum MemoryScope\|ContextError" crates/continuum-context/src
grep -rn "ContextTrust::" crates/continuum-context/src
grep -rn "use continuum_trust" crates/continuum-context/src
grep -rn "impl .*ContextTrust\|impl .*ContextProvenance\|impl .*MemoryClaim" crates/continuum-context/src
grep -rn "to_messages\|into_invoke_request\|InvokeRequest\|impl .*Serialize\|impl .*Deserialize\|Serialize for ContextPackage\|Deserialize for ContextPackage" crates/continuum-context/src
grep -rn "std::env\|SystemTime\|Instant::now\|current_dir\|std::fs" crates/continuum-context/src
grep -rn "continuum_persist\|continuum_events" crates/continuum-context/src
grep -rn "NodeTrust\|PrivacyClass" crates/continuum-context/src
```

**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；**做法是 grep 出候选行后对 `src/`
逐处通读**，结论以通读为准、grep 只用来定位。**作用域写死**：只扫 `src/`，不含 `tests/`
（测试要用 `ContextTrust::` 比较、要用 `serde_json`）。

**结果：** 第一条（`pub struct`）恰一处（`lib.rs`）；第二条（`compile.rs` 内的结构体字面量）
恰一处；第三条过滤掉 `impl ContextPackage {` 后恰两处（定义＋构造）——**不能把 `impl` 算成构造**。
其余八条按预期零命中或只在 `use` / 签名处命中，命中行逐处抄进复核记录。

- [ ] **Step 4: 逐条核对设计 §2.4 的十四条发明判据**

设计 §2.4 的表是本块「发明判据」的总账，每一行给**代价／边界／落点／具名收件人**。
**本计划的动作是逐行核「落点栏是否指向一条真实存在的用例」**，不重述该表：

| §2.4 的行 | 落点栏指的东西 | 本计划的用例 |
|---|---|---|
| 第 1 条（四类内容数组的元素类型） | §4.1、§4.5 | Task 1 的 `lib.rs` / `package.rs` / `reference.rs`；Task 3 的六条来源用例 |
| 第 2 条（对齐规则） | §4.2 | Task 6 的 `the_package_passes_the_independent_validate`、`relabeling_a_reference_changes_its_label_in_place` |
| 第 3 条（Conversation／Project／Tool results 进 `references[]`） | §5.4 | Task 3 的 `conversation_history_…`、`project_text_…`、`tool_output_…`、`external_provider_text_…` |
| 第 4 条（候选序截断、去重） | §5.3 | Task 5 的 `references_are_truncated_in_input_order`、`artifact_ids_are_deduped_then_truncated`；Task 4 的 `duplicate_claim_ids_keep_the_first` |
| 第 5 条（条数预算，不是 token） | §5.2、§5.3 | Task 5 的 `the_three_budgets_are_independent`；Task 2 的 `the_budget_is_three_independent_counts` |
| 第 6 条（空串丢弃） | §5.3 | Task 3 的 `instructions_are_taken_whole_and_empty_strings_are_dropped`；`compile_sources` 的纯空白反例 |
| 第 7 条（字段私有、唯一构造点） | §4.3、§6.4 | Task 8 的四份 `compile_fail`；Task 9 的 `context_package_is_constructed_only_in_compile`（`pub struct` 一处＋`compile.rs` 构造一处，过滤 `impl` 行）；Task 1 的 `new`／`from_parts`／`from_fields` 源码面断言 |
| 第 8 条（不投影为 `Message`） | §6.2 | Task 1 的 `there_is_no_projection_to_messages_or_invoke_request`；Task 9 的 `there_is_no_projection_serialization_or_persistence_surface` |
| 第 9 条（三个检索输入与 `RetrievalQuery`） | §5.2、§5.3、§7.2 | Task 4 的十二条（尤其 `the_query_carries_scopes_anchor_text_limit_and_terminal_false`、`an_empty_memory_query_still_retrieves_and_normalizes_to_none`） |
| 第 10 条（`Reference::Conversation` 复用 `Message`） | §4.5 | Task 1 的 `reference_keeps_message_and_tool_result_verbatim` |
| 第 11 条（容器隔离） | §5.3、§5.4 末 | Task 3 的 `references_never_leak_into_instructions` |
| 第 12 条（纯函数／可重复） | §5.5、J7a／J7b | Task 7 的三条 |
| 第 13 条（空包合法／有引用无指令） | §4.3、J10 | Task 3 的 `an_empty_request_yields_six_empty_arrays`＋`references_never_leak_into_instructions` 的前半（`instructions` 空**且** `references` 非空） |
| 第 14 条（§70 与 §280 的对应） | §6.2 | Task 1／Task 9 的「无投影 API」；**模型面产物本身无照片**（本块不交付） |

**第 14 条的落点栏指「文档读法」，不是用例**——它在本块的 crate 里拍不出模型面产物，这一点
**据实写在上面**，不标注为覆盖。

**结果：** 十四行逐行有落点或明写「无照片」；没有一行被漏读。

- [ ] **Step 5: 变异复核**

按 P5c 纪律 1／2 跑**承重守卫**，每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量
`--no-fail-fast` → 读红位 → 还原后 `sha256sum` → 独立日志路径。**逐轮在报告里附
「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位」**。

**按设计 §8.3 的预告表逐行照跑，除 §8.3 的 J3 旧行（设计缺陷，标为不适用）**：

| 变异 | 该红 | 不该红 |
|---|---|---|
| `compile` 返回空包（不装任何候选） | J3 正例、J4、J5 正例、J11；Task 3 的 `the_alignment_arrays_are_non_empty_and_equal_length`（备注） | J1、J2、J10、J7b |
| `instructions` 也受预算截断 | J11 | J5 |
| `ContextPackage` 加 `pub` 字段或 `Default` | J1-a／J1-b；加 `pub` 字段还会让 J2 的 Task 1 源码面断言红 | J3–J11 |
| `trust_labels` 改成整包一个标签（长度 1） | J3（`validate` 的 `LengthMismatch`/`ShapeMismatch`）；Task 3 的弱检查（等长失败，备注） | J4、J5、J10 |
| 把 `references` 的候选也 push 进 `instructions[]` | J6、J4、J3 | J5 |
| 把 `label(provenance)` 换成写死 `USER_INSTRUCTION` | J3 正例（本计划混合来源）＋ J3 反例 | —— |
| **不跑**：移除 J3 的 `validate` 调用、改回直接断言长度 | ——（与设计 §8.1 J3 相抵：J3 不自写第二份长度／逐位断言；见「遗留」第一节第 4 条） | —— |
| `memories` 不调检索面、永远空 | J4②、J8、J3 正例、J7a、**J5 的 memories 正例**（`the_three_budgets_are_independent` 要求 2 条、`memories_are_truncated_to_max_memories` 要求 2 条）、J14 的反例（`max_memories_one_calls_the_surface_once` 要求 1 条） | J14 的第一侧（`scopes` 空／`max_memories == 0` 不调，永远不调反而绿） |
| `max_memories == 0` 仍调检索面 | J14 | J5 的正例 |
| 把 `MemoryError` 包装成自定义错误 | J13 | J1–J11 |

**claim 相关的变异行**（`memories` 永远空、J3 正例、J5 的 memories 正例等）在 P6C 交付
`for_test` 前**不跑**——对应用例挂起（测试体未写入），没有可读的红位；这些行标为「挂起」，
Task 10 不得把它们算作已复核。

**本计划新增的两条承重变异**（设计 §8.3 未列，但对应 Review Focus 4／5）：
（i）在 `compile.rs` 里加 `let _ = std::time::SystemTime::now();` → Task 7 的
`compile_reads_no_clock_environment_or_filesystem` 与 Task 9 的 `compile_reads_no_hidden_source`
红；
（ii）在 `package.rs` 里加 `pub fn to_messages(&self) -> Vec<Message>` → Task 1 的
`there_is_no_projection_to_messages_or_invoke_request` 与 Task 9 的
`there_is_no_projection_serialization_or_persistence_surface` 红。

**四处与设计 §8.3 预告表的差异（据实；以本表为准）**：

1. 设计表说「把 `label(provenance)` 换成写死 `USER_INSTRUCTION`」时 **J3 正例可能仍绿（若所有
   候选同源）**。本计划的 J3 正例是**混合来源**（当前指令＋历史轮＋web 文本＋artifact＋claim），
   写死标签会让 references / artifacts / memories 的标签全部变成 `USER_INSTRUCTION`，
   `validate` 的 I4/I5 会拒它——**所以本计划的 J3 正例也会红**。这是**更强的**结果，不是表错；
   注释里要写明。
2. 设计表把「`memories` 永远空」的「不该红」写成包含「J5 的 memories 预算」——**这一行照抄会错**：
   本计划的 `the_three_budgets_are_independent` 要求 `max_memories = 2` 时真有 2 条 memories，
   `memories_are_truncated_to_max_memories` 也要求 2 条；**`memories` 永远空会让这两条正例红**。
   上表已据实把「J5 的 memories 正例」移到「该红」。
3. 设计 §8.3 没有「`compile` 返回空包（不装任何候选）」这一行。本计划补它，并把 Task 3 的
   `the_alignment_arrays_are_non_empty_and_equal_length` 列进该红（备注）——这是本计划的
   补充行，不是设计表的原文。
4. 设计 §8.3 的「`trust_labels` 改成整包一个标签（长度 1）」一行没有列 Task 3 的弱检查。
   本计划在该行「该红」栏补列「Task 3 的弱检查（等长失败，备注）」；设计原文只列 J3。

**等价变异体不许算作守卫**：`ContextPackage` 的 `Debug` / `Clone` / `PartialEq` 派生换序、
注释文字改动、测试内期望值的书写顺序——这些不改变任何断言的结果。

**结果：** 每条可跑的承重变异都有「该红」的实测；claim 相关行在 `for_test` 交付前标为挂起、
不算已复核；不该红的用例按表保持绿；所有源码已还原，`sha256sum` 前后一致；`.tmp/` 日志保留到
复审结束。

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留/待协调` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。
**本计划不新建、也不修改第二份文档**；若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-context crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock docs/superpowers/plans/2026-10-17-p6a-context-compiler.md
git commit -m "docs(context): P6A 的收尾与复核"
```

**结果：** 只提交显式路径；`git status` 干净（`.tmp/` 不入库）。

---

## 自审

### 一、Spec coverage（设计节 → task）

| 设计节 | 内容 | 落在哪个 task |
|---|---|---|
| §1.1／§1.2 | 建什么／不建什么 | 全部 task 的 Files 与 Global Constraints |
| §2.1 | crate 名与四条依赖边 | Task 1（`Cargo.toml`＋`ALLOWED`）；Task 10 Step 2 对账 |
| §2.2 | 五个类型与「不新增的类型」 | Task 1／Task 2；无 `error.rs` |
| §2.3 | 层间位置与不登记执行层边 | Task 1 的 `ALLOWED`；Task 10 Step 2(c) |
| §2.4 | 十四条发明判据 | Task 10 Step 4 的逐行核对表 |
| §2.5 | 切分 §四六条 | Task 1（边表）＋ Task 10 Step 2（迁移／边） |
| §3.1–§3.9 | 规范依据逐条 | Task 3／Task 4（五来源）＋ Task 6（§281）＋ Task 7（§69） |
| §4.1 | 六字段逐字 | Task 1 |
| §4.2 | 对齐规则 | Task 3／Task 4（同步 push）＋ Task 6（复核） |
| §4.3 | 唯一构造点与五条不变量 | Task 1／Task 3／Task 4／Task 5／Task 8／Task 9 |
| §4.4 | 六个只读访问器 | Task 1 |
| §4.5 | `Reference` 三变体 | Task 1 |
| §4.6 | 不定义 P6B／P6C 类型 | Task 9 的 J9 |
| §5.1 | 签名、谁调、何时调 | Task 3（签名）＋「跨计划前置」第四节（调用方） |
| §5.2 | 请求与预算 | Task 2；Task 4 的三个控制位 |
| §5.3 | 六条选择规则 | Task 3（1／2／6）＋ Task 4（5）＋ Task 5（3／4） |
| §5.4 | 五来源到四字段 | Task 3／Task 4 |
| §5.5 | 无隐藏来源 | Task 7＋Task 9 |
| §5.6 | 失败面 | Task 4（J13）＋ Global Constraints（无 `ContextError`） |
| §6.1–§6.4 | 消费契约与执行器接入 | Task 1（无投影）＋ Task 9＋「跨计划前置」第四节 |
| §7.1／§7.2 | P6B／P6C 接口 | Task 6（`validate`）＋ Task 4（检索面） |
| §8.1–§8.4 | J1–J14 与照片 | 下面的「照片归位」表；J12 无照片 |
| §9 | 明确不做 | Global Constraints 末段；Task 10 Step 2(c) |
| §10 | 零表零迁移 | Task 10 Step 2(d) |
| §11 | 22 条遗留 | `## 遗留/待协调`（需要的照抄，其余照原文留在设计） |
| §12 | 切分文档的错／缺口 | 不重述；本计划不改切分文档 |

**照片归位（J1–J14）**：

| 判据 | 照片 | task |
|---|---|---|
| J1-a／J1-b／J1-c | 三份 `compile_fail` | Task 8 |
| J1-d | `pub struct ContextPackage {` 恰一处＋`compile.rs` 内构造恰一处（过滤 `impl` 行）＋无 `Default`/`Deserialize`；`new`/`from_parts`/`from_fields` 无命中 | Task 9（计数）＋Task 1（禁止入口） |
| J2 | 六字段名/序（源码面）＋`impl` 内 `pub fn` 恰六条且名字固定（源码面）＋外部写字段（编译期） | Task 1＋Task 8 |
| J3 | `validate` 正例＋三条反例＋换 provenance | Task 6 |
| J4 | 六条正例 | Task 3（①③④⑤⑥）＋Task 4（②） |
| J5 | 预算／截断／去重 | Task 5（references/artifacts）＋Task 4（memories） |
| J6 | 容器隔离 | Task 3 |
| J7a／J7b | 无隐藏来源／确定性 | Task 7 |
| J8 | 只来自检索面 | Task 4 |
| J9 | 源码面不定义 P6B/P6C 类型 | Task 9 |
| J10 | 空包＋「有引用无指令」（`instructions` 空且 `references` 非空） | Task 3 |
| J11 | 指令全收 | Task 3＋Task 5 |
| J12 | 端到端「节点执行前必经」 | **无照片**（执行器未建，设计 §8.4 第 1 条、§11 第 1 条） |
| J13 | `MemoryError` 三条透传（两条代表路径＋`AnchorNotAllowed` 补测） | Task 4 |
| J14 | `scopes` 空／`max_memories == 0` 不调检索 | Task 4 |

**挂起标注**：J3 正例、J4②、J5 的 memories 半、J7a／J7b、J8、J14 反例里需要 claim 夹具的用例，
在 P6C 交付 `for_test` 之前**挂起**；照片归位成立，但不得计为「已绿」。

### 二、Step scan

- **每个 Step 是一个可核对动作**：写用例／跑测试／实现／全量并提交，四拍固定；test-only task 的
  Step 3 明写「无生产代码」。Task 1 Step 3 的 3a–3e 是「建骨架」这一个动作的文件级分解
  （清单／根定义／引用类型／访问器／两处共写文件），不拆成五个可独立提交的步骤。
- **每个 Step 有可核对结果**：用例名可列出、命令的失败形态（编译失败／运行期红／首次即绿）、
  文件与字段计数、`ALLOWED` 四项、`grep` 命中行、sha256 前后一致。
- **测试命令精确到 `--test` 目标**：`package` / `request` / `compile_sources` / `memory` /
  `budget` / `labels` / `purity` / `type_level` / `source_surface`。
- **「运行，确认失败」逐 task 标注了量到什么**：Task 1 是 **package 未找到**（不是编译失败）；
  Task 2／3 是编译失败（不算红）；Task 4／5 是运行期红（claim 用例在 `for_test` 交付前挂起）；
  Task 6／7／9 是首次即绿（claim 用例挂起）；Task 8 是 trybuild 的运行期红。
- **没有一步同时改两个关注点**：Task 3 只做非记忆映射；Task 4 只做记忆分支；Task 5 只做预算／去重；
  Task 6 只做复核；Task 7 只做纯函数；Task 8 只做 trybuild；Task 9 只做源码面。

### 三、Type consistency

- `ContextPackage`：六字段**逐字**、**顺序**、**私有**；访问器名逐字、**恰六条**；派生只有
  `Debug, Clone, PartialEq`；**没有** `Default` / `Deserialize` / `new` / `from_parts` /
  `from_fields` / `into_parts` / `&mut`。**分工**：Task 1 的源码面断言钉 `new`／`from_parts`／
  `from_fields`（语料随 `src_texts()` 从三文件扩到六文件）、`&mut`／`into_parts`／`into_iter`／
  `pub` 字段与六访问器计数；Task 8 钉 `Default`／`Deserialize`／外部写字段；Task 9 钉
  `pub struct` 一处、`impl ContextPackage` 六文件合计一处（`compile.rs` 零）、`compile.rs`
  构造一处（过滤 `impl` 行）与三个禁止构造入口扫六文件。
- `Reference`：`Text(String)` / `Conversation(Message)` / `ToolOutput(ToolResult)` 三变体；
  不序列化；不造 `Turn` / `Conversation` / wrapper（Task 1）。
- `ContextRequest`：六字段顺序照设计 §5.2；**没有 `Clone`**；`anchor: Option<String>`；
  `memory_query: Option<String>`（Task 2）。
- `ContextCandidate<T>`：`content: T` ＋ `provenance: ContextProvenance` 两个 `pub` 字段
  （Task 2）。
- `ContextBudget`：`max_references` / `max_artifacts` / `max_memories`，三个 `usize`
  （Task 2）。
- `compile`：`pub fn compile(request: ContextRequest, budget: ContextBudget,
  memory: &dyn MemoryRetrieval) -> Result<ContextPackage, MemoryError>`——**参数序照设计 §5.1**；
  返回 `MemoryError`，**没有 `ContextError`**（Task 3／Task 4）。
- P6B：`ContextProvenance` / `ContextTrust` / `label(&ContextProvenance) -> ContextTrust` /
  `PackageShape { instructions, references, artifacts, memories }` /
  `validate(PackageShape, &[ContextProvenance], &[ContextTrust]) -> Result<(), TrustError>` /
  `TrustError::{LabelDoesNotMatchProvenance, DataSlotCarriesUserInstruction,
  HistoricalInstructionInInstructions}`——全部按 P6B §3.1–§3.5 的实际名（Task 6）。
- P6C：`MemoryClaim`（`id()` / `statement()` 等七访问器）、`MemoryScope::{Personal, Project}`、
  `MemoryError::{AnchorRequired, EmptyScopeSet}`、`MemoryRetrieval::retrieve(&RetrievalQuery) ->
  Result<Vec<MemoryClaim>, MemoryError>`、
  `RetrievalQuery::new(scopes, anchor, text, limit, include_terminal) -> Result<Self, MemoryError>`
  与五个访问器——全部按 P6C §3.5／§6.1 的实际名（Task 4）。
- **没有本块自造的 `ContextError` / `ContextTrust` / `ContextProvenance` / `MemoryClaim` /
  `MemoryScope` / `NodeContext` / `ArtifactRef`**（Task 9 的 J9）。

### 四、Review Focus 归位

| Review Focus | 归属 task | 具体用例 |
|---|---|---|
| 1. 预算 0 ＋ `memory_query=Some` 仍调检索 | Task 4 | `the_retrieval_surface_is_not_called_when_max_memories_is_zero` |
| 2. `provenance[]` / `trust_labels[]` 错位 | Task 6（生产代码在 Task 3／4） | `the_package_passes_the_independent_validate`、`relabeling_a_reference_changes_its_label_in_place` |
| 3. 历史用户轮进 `instructions[]` | Task 6 | `compile_does_not_judge_historical_but_validate_does`、`validate_rejects_a_historical_instruction_in_the_instruction_slot` |
| 4. `compile` 读时钟／环境 | Task 7＋Task 9 | `compile_reads_no_clock_environment_or_filesystem`、`compile_reads_no_hidden_source` |
| 5. 消费方扁平化成 `Vec<Message>` 后标签丢失 | Task 1＋Task 9 | `there_is_no_projection_to_messages_or_invoke_request`、`there_is_no_projection_serialization_or_persistence_surface` |

### 五、比例

- 设计 1283 行；本计划 **1820 行**。任务 **10 个**；
  J1–J14 共 14 条判据，除 J12 明写无照片外，每条至少有一条用例；**J1 另有 `pub struct` 一处＋
  `compile.rs` 构造一处的计数与三个禁止构造入口的源码面断言，J2 另有六访问器精确计数，J10 的
  「有引用无指令」半与「全空」半各一条**；J3／J4②／J5 的 memories 半／J7a／J7b／J8／J14 反例里
  的 claim 夹具在 P6C 交付 `for_test` 前挂起。没有「一个 task 包办全部照片」的写法。
- Task 1／2（类型）约占 19%，Task 3／4／5（装配与预算）约占 21%，Task 6／7（复核与纯函数）约占
  8%，Task 8／9（编译期与源码面）约占 11%，Task 10（收尾）约占 9%；其余约 32% 是头部、
  跨计划前置、自审与遗留。**生产代码集中在 `src/` 的六个文件**；测试量大于生产量是刻意的
  ——本块的判据几乎全是「形状＋边界＋否定面」。
- **比例里有一部分是实现体的誊写**：Task 1 的 Step 3 内动作 3b–3d（`lib.rs`／`reference.rs`／`package.rs`
  的类型与访问器）、Task 2 的 Step 3（`request.rs`／`candidate.rs`）、Task 3／4／5 的 Step 3
  （装配与预算的伪代码）含完整类型／伪代码抄写。**这是可执行计划的刻意选择**（执行者不必回读
  设计猜形状），代价是篇幅；它不是纯设计重述，主要增量仍是测试与 Step 细节。
- **没有把设计 §11 的 22 条搬成 task**：它们不是本块的实现面，逐条留在 `## 遗留/待协调`
  或设计原文里。

---

## 遗留/待协调

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**
设计 §11 的 22 条**逐条仍有效**；下面只照抄**与本块实现直接相关**的条目，其余照原文留在设计里
（重述就是第二份转录）。

### 一、本计划新查出的缺口（**发现即报，本计划不就地补**）

1. **P6C 的 `MemoryClaim` 在 crate 外不可构造，P6A 的 claim 夹具需要 feature-gated 测试构造面**
   （**实测**：P6C 设计 §5.1 逐字「`MemoryClaim::new` 是 `pub(crate)`，crate 外只能读不能造」；
   §5.5 再次写明唯一产生点；照片 C2（§10.1）把「在 crate 外调 `MemoryClaim::new`」列为
   **编译失败**的正例）。本计划的 Task 4／5／6／7 里，凡是「假检索面返回一枚 claim」的用例
   （J4②、J5 的 memories 半、J7a／J7b、J8、J14 的反例）都需要在 `continuum-context` 的
   `tests/` 里造 claim——**今天做不到**：`MemoryClaim::new` 不可见，字段私有，也没有
   `Default` / `Deserialize`。

   **选定方案（协调者裁定；P6C 计划遗留第 11 条在本轮 fix round 3 已按同一契约落定：
   `test-support` feature ＋ `#[doc(hidden)]` 七字段 `for_test`）**：P6C 增加
   feature-gated 的测试构造器；默认构建下 `new` 仍是 `pub(crate)`。
   **C2 不必改**：C2 钉的是「crate 外调 `MemoryClaim::new`」在**默认构建与 `--all-features`
   下都仍编不过**；`for_test` 是**另一个入口**（只在 `test-support` 下可见），不让 `new` 变
   `pub`，也不改 C2 的样例与 `.stderr`。

   ```rust
   // crates/continuum-memory/src/claim.rs
   impl MemoryClaim {
       /// 仅供跨 crate 测试装配。未开 `test-support` 时本方法不存在。
       /// 不校验、不归一（与 `rehydrate` 同形），参数由调用方保证。
       #[cfg(feature = "test-support")]
       #[doc(hidden)]
       pub fn for_test(
           id: MemoryClaimId,
           scope: MemoryScope,
           statement: impl Into<String>,
           sources: Vec<MemorySourceId>,
           validity: Validity,
           confidence: Confidence,
           last_confirmed: i64,
       ) -> MemoryClaim;
   }
   ```

   P6C 的 `crates/continuum-memory/Cargo.toml` 增 `[features] test-support = []`（空 feature，
   只开可见性）。**这是测试面的例外，不改变 P6A 的生产接口**：`compile` 仍只命名 P6C 的
   `MemoryClaim` / `MemoryScope` / `MemoryError` / `MemoryRetrieval` / `RetrievalQuery`；
   `Validity` / `Confidence` 只出现在 `#[cfg(feature = "test-support")]` 的 helper 里。
   P6C 设计 §3.5 把这两个类型列为局部形状；P6C 计划遗留第 11 条已注明「test-support 面例外」，
   设计 §5.5 的生产口径不变（默认构建下 crate 外仍构造不出 claim）。
   `for_test` 的**七个参数、五个类型**须在 `continuum_memory` crate 根可见
   （`MemoryClaimId`／`MemoryScope`／`MemorySourceId`／`Validity`／`Confidence`；
   `statement: impl Into<String>` 与 `last_confirmed: i64` 是标准类型，不需要 P6C 再导出；
   P6C 的 crate 根 `pub use` 要覆盖那五个：`MemorySourceId` 来自 `source`，其余四个
   （`MemoryClaimId`／`MemoryScope`／`Validity`／`Confidence`）来自 `claim`；
   否则 P6A 的 helper 先报 `E0432`）。P6A 侧在
   `crates/continuum-context/Cargo.toml` 用同一路径打开该 feature
   （只影响测试构建）——**这是 Task 4 Step 3b 的条件步，预检 P6C 的 `test-support` 通过后才加**：

   ```toml
   [dev-dependencies]
   continuum-memory = { path = "../continuum-memory", features = ["test-support"] }
   ```

   **在 P6C 交付 `for_test` 之前，Task 4–7 的 claim 用例按「挂起」计，Task 10 不得报全绿**
   （只能报「非 claim 面全绿＋claim 面挂起」）。`Cargo.toml` 的 feature 行同理**不得提前加**：
   feature 不存在时 Cargo 直接解析失败，非 claim 面也跑不了。**本计划不伪造 claim、不用
   `unsafe`、不反序列化私有字段、不直接写私有字段。**
   **收件人：P6C（`MemoryClaim` 的持有者）＋协调者。**
2. **`compile` 的参数序**：设计 §5.1 写的是 `compile(request, budget, memory)`；本计划**照设计**
   （不是 `request, memory, budget`）。若协调者要改序，改的是设计 §5.1 与 Task 3 的签名，
   **不是本计划单方面改**。**收件人：协调者（仅当要改序时）。**
3. **`ContextPackage` 的 `Debug` 会把 `MemoryClaim.statement` 带进日志**：设计 §4.3 已写明
   「`Debug` 只用于测试／诊断，不作为审计或日志输出」。本计划**没有**为它建用例（日志纪律不在
   本块的类型层可拍），**据实报出**。**收件人：P2／第 4 层（隐私）＋协调者。**
4. **设计 §8.3 有一行与 §8.1 J3 相抵**（**实测**）：§8.3 的变异预告写「移除：J3 不调 `validate`、
   改回直接断言长度／逐位 → J3 的 (b)(c) 红」，而 §8.1 J3 逐字要求「**J3 调 `validate`，不自写
   第二份长度／逐位断言**」（P6B §5.2 第 5 条同此）。若照 §8.1 写，J3 里**没有**可「改回」的
   直接断言，那一行变异没有对应的实现体。**本计划按 §8.1 执行**（Task 6 的六条用例全部走
   `validate`），并在 Task 10 的变异表里把该行标为**不跑**。**收件人：设计作者 ＋ 复审者。**
   **订正方向（二选一）**：删掉 §8.3 那一行；或把它改成「把 `validate` 调用换成只断言 `Ok`」
   并说明这是**测试判别力**的变异，不是生产代码变异。
5. **设计 §8.2 J1-d 的旧 `grep` 判据会把 `impl ContextPackage {` 算成构造**（**实测**：
   `grep -rn "ContextPackage {" crates/continuum-context/src` 会命中 `src/package.rs` 的
   `impl ContextPackage {`；设计原文「只命中定义处与 `compile` 内的一处构造」照跑会多出一处，
   **设计原文会红**）。本计划在 Task 9 的断言里按「含 `ContextPackage {` 且 `trim_start()`
   不以 `impl ` 开头」过滤，并在 Task 1／Task 9 把 `impl ContextPackage` 的计数与三个禁止
   构造入口扩到六个源文件。**请设计作者／协调者**把 §8.2 J1-d 的 grep 改为过滤 `impl` 行
   （或把语料限定为 `lib.rs` 与 `compile.rs`），否则设计与可执行计划不同口径。
   **收件人：设计作者＋协调者。**

### 二、设计 §11 的条目（照抄需要的；收件人照原文）

| §11 | 条目（照抄要点） | 本计划的处置 |
|---|---|---|
| 第 1 条 | 「节点执行前必经」**没有执行器挂点**；`NodeContext` 没有 `ContextPackage` 的位置；`continuum-graph` 不能依赖本块。缺的是「执行器在何处、以什么签名收 `ContextPackage`」这一步 | 本块**以库形态交付**；Task 10 Step 2 只核「没新增执行层边」；**收件人：P1 ＋ 协调者** |
| 第 2 条 | §9.4 与 §9.1 的**两种读法**；「认知层 → 执行层」这条载荷由哪一侧登记未裁定 | 本块不登记指向执行层的代码边；接线经 runtime；**收件人：《工程》维护者 ＋ 协调者** |
| 第 3 条 | §280 的**元素类型未给**；本设计定 `String` / `Reference` / `ArtifactId` / `MemoryClaim` | Task 1／Task 2 落地；**收件人：规范维护者** |
| 第 4 条 | §280／§281 的**对齐规则未给**；本设计定「按声明序拼接、等长同序」 | Task 6 的 `validate`；**收件人：规范维护者** |
| 第 5 条 | §280 **没有 Conversation／Project／Tool results 的专属字段**；本设计放进 `references[]` | Task 3 的四条映射；**收件人：规范维护者** |
| 第 6 条 | §70 的 Intent／Project State 与 §279 的五来源不同；本设计以 §279 五类为 MUST | Task 3 的 `instructions` / `references` 文本投影；**收件人：规范维护者** |
| 第 7 条 | 「最小必要」的**相关性判据未给**；本块用候选序，不做打分／检索 | Task 5 的预算与去重是唯一的「最小必要」实现；**收件人：规范维护者** |
| 第 8 条 | P3 的模型调用路径**没有标签位置**；本块不投影为 `Message` | Task 1／Task 9 的「无投影 API」；**收件人：P3 ＋ 协调者** |
| 第 9 条 | C4／OPEN-016 与 ENG-008：capability 判定在 Compiler 内部还是外部未裁定；本块口径「判定在外」 | 本块不接 capability 类型；**收件人：协调者 ＋ 规范维护者** |
| 第 10 条 | P2 的标签强制点**无法依赖 P6B**；标签怎样到达 P2 的拒绝点未定 | 本块只承载标签；**收件人：P6B ＋ P2 ＋ 协调者** |
| 第 13 条 | `into_parts()` / 按值取出内容的通道**不存在**；要不要开、开了怎样保持六字段绑定未定 | Task 1 不提供；**收件人：P1 ＋ 协调者** |
| 第 14 条 | `ContextPackage` **不能序列化**；跨进程／持久化／审计若需要要另立受控入口 | Task 8 的 `compile_fail`；**收件人：协调者** |
| 第 16 条 | §70 的「降低隐私暴露」**没有落点**；上下文装配时的隐私裁剪由谁做未定 | 本块只按调用方给的 `scopes` 取记忆；**收件人：P2 ＋ 第 4 层 ＋ 协调者** |
| 第 17 条 | Conversation **没有存储、没有轮次身份**；`Reference::Conversation` 只是承载形状 | Task 1 只定义载荷；**收件人：协调者 ＋ P1** |
| 第 18 条 | `ToolResult` **没有 `external` 判别位**、provider 身份没有随文本下来 | Task 3 只按调用方给的 provenance 装包；**收件人：P3F ＋ P3B ＋ P3G ＋ P1 ＋ 规范维护者** |
| 第 19 条 | artifact **没有类型化的生产者分类**；`Artifact.producer_node` 是无类型 `Option<String>` | Task 1／Task 3 只收 `ArtifactId`；**收件人：P1 ＋ 协调者** |
| 第 20 条 | 用户粘贴的外部内容与用户指令**不可区分**；本块不拦 | Task 3 的 `references_never_leak_into_instructions` 明写「本块不判」；**收件人：P2 ＋ 客户端／对话协议持有方 ＋ 规范维护者** |
| 第 21 条 | memory claim 的**外部性不进标签**；`Memory { claim_id }` 只投影 `MEMORY` | Task 4 只调 `ContextProvenance::memory`；**收件人：P6C ＋ P6B ＋ 规范维护者** |
| 第 22 条 | `continuum-artifact` 的**层归属**在两处层表里不一致 | 本块依赖方向不受影响；**收件人：协调者 ＋ P6C** |

**已闭合、不搬进本计划的两条**：§11 第 11 条（P6C 的检索面与 `MemoryClaim` 读访问器已冻结）、
第 12 条（P6B 的类型与 `validate` 已冻结）——设计已逐字写「收件人：无（已闭合）」，本计划不再重复。
**第 15 条**（迁移档）：本块零表、不占档，`runtime_migrations()` 一字不改，Task 10 Step 2(d) 核它。

### 三、本计划自定的形状与取值（**申报**）

- **`tests/support/mod.rs` 的假检索面与 `test_claim` helper**：设计只要求「假实现由本 crate 自己写」
  （§2.1 末），没给名字／形状。**本计划自定** `CountingMemory` / `QuerySnapshot`；代价是测试代码
  多一层模块，收益是 J8／J13／J14 的「调用计数」与「查询快照」有一个可指认的落点。
- **`ContextPackage` 的 `Builder` 不建**：本计划用局部 `Vec` ＋ 同步 push，不新增公开类型；
  这是设计 §4.2 第 5 条的直接落法，**不是本计划自定**。
- **Task 8 的四份 `compile_fail`**：设计 §8.2 J1 只列了三份（J1-a／b／c）；第四份
  `package_fields_are_read_only.rs` 是 J2 的编译期半，**本计划按 J2 的「外部写字段编不过」补出**。
  代价是多一份 `.stderr`，收益是 J2 不再只有源码面。
- **Task 9 的源码面断言用 `include_str!` 而不是 `grep` 脚本**：设计 §8.2 J1-d／J9 写的是 `grep`
  命令；本计划把它们做成 Rust 测试，**语料与作用域写死**（只扫 `src/`）。代价是注释里不能出现
  被禁的字面量（`ContextPackage {`）；收益是这些照片会随 `cargo test` 一起跑。
- **Task 10 的两条新增变异**（时钟／`to_messages`）：设计 §8.3 未列，对应本计划的 Review Focus
  4／5。**本计划自定**，不替代设计 §8.3 的十行。

---

## 交付给谁

- **交给 P1 的节点执行器（未建）**：`compile` 的生产调用点（节点进入 `Running` 之前一次）；
  `NodeContext` 是否收 `ContextPackage` 或 runtime 的调用序（设计 §6.3 的两个候选）。
  **本块只提供库，不建执行器。**
- **交给 P6B 的 `continuum-trust`**：`provenance[]` / `trust_labels[]` 两个字段的承载物；
  本块逐片段调 `label()`，不缓存、不覆盖、不合并。
- **交给 P6C 的 `continuum-memory`**：本块消费 `MemoryRetrieval` 的返回；不写 P6C 的表、不产生 claim。
  **并请 P6C 给出 crate 外可用的 claim 测试构造面**（遗留第一节第 1 条）。
- **交给 P2**：`trust_labels[]` 的拒绝行为；本块不判、不拒。
- **交给 P3**：标签怎样过模型边界；本块不投影为 `Message`。
- **交给规范维护者（本项目无此角色）**：§280 的元素类型与对齐规则、三类来源的落点、
  §70 与 §279 的来源表差异、「最小必要」的相关性判据。
- **交给协调者**：迁移档（本块零表）、`continuum-artifact` 的层归属、§9.4 的读法、capability 判定
  的内外、以及遗留第一节的新缺口。
- **交给复审者**：本计划的 **10 个 task**、**J1–J14 的照片归位**与 **Review Focus 5 条**；
  请先对 Task 4 的 J14、Task 6 的 J3、Task 8／Task 9 的「唯一构造点」逐行核。

---

## 收尾记录（2026-10-19）

- **全量门（工作树独立 `CARGO_TARGET_DIR=$PWD/target`）**：`EXIT=0`，**250** 个 `test result:`
  行（含 Doc-tests），**1449 passed / 0 failed / 2 ignored / 0 warning**。日志 `.tmp/full2.log`
  （不入库）。独立 target 是刻意的：共享 target 下测试二进制的 `CARGO_MANIFEST_DIR` 可能指向
  别的树，源码面语料会假绿。
- **计划外的一处测试共写**：见「跨计划前置」§三的补记——`crates/continuum-memory/tests/wiring.rs`
  的 S2 断言与 main `e4a1ab2` 对齐，本块提交为 `481c19d`。
- **终审修复（P6A Needs fixes）**：源码面 (w) 的语料收窄到 `continuum-trust/src`（P6A 的
  `compile.rs` 有四个设计要求的 `label()` 调用点）；`continuum-context` 补「生产路径不调用
  `validate`」的守卫；J4 的 project／tool／provider／artifact 四个用例补「不在别的字段」断言；
  清理测试文件里已过期的过程注释。
