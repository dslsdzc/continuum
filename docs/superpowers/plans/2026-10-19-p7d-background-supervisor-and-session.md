# P7D（Background Supervisor + Session Orchestrator）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**任务单：** 切分 §五 P7D 行（`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md:337`，逐字）：

> 建后台治理与会话启动：§299 四类来源的 Background Supervisor、Manual／Auto 的 Session Orchestrator、
> §171 触发点、`product drive` 的按值消费

**本计划是该行的实现分解。设计与规范的唯一事实来源是**
`docs/superpowers/specs/2026-10-19-p7d-background-supervisor-and-session-design.md`（1927 行，下称「设计」）。
**本计划不改设计、不改规范、不改切分文档、不改代码以外的任何计划。** 引用一律用**设计节号**；
规范原文与依据由设计 §3 逐条给出，本计划不复制。

**Goal:** 新建 crate `continuum-supervisor`，交付：①§299 四类来源闭集 `TaskSource` 与唯一任务输入面
`AuthorizedTask`（四个构造器、字段私有）；②Background Supervisor 的 `next_tasks`／`decide`／`admit`／
`checkpoint`／`checkpoint_timed_out`／`release`；③Manual／Auto 共用的 `SessionPlan` 与
`SessionOrchestrator`；④§171 的 `SynthesisIo` 端口与 `SynthesisTriggerPoint` 三个转发；⑤P6D 合并门的
rebaseline。**零表、零迁移、零事件；不另写排名；不执行会话恢复；不做记忆合成本体**
（设计 §1.1、§1.2、§9、§10）。

**Architecture:** 本块是第 7 层（长期循环）的消费方 crate，依赖四条普通边
`continuum-supervisor → continuum-intent-scheduler`／`continuum-arbiter`／`continuum-semantics`／
`continuum-memory`，一条 dev 边 `→ continuum-canonical`（仅测试夹具）。`TaskSource` 恰四值；
`AuthorizedTask` 字段私有、四个具名构造器；`SupervisorDecision` 是选择的**单一表示**（`Task`／
`ContinueSession`／`NoIntent`），`SessionPlan` 恰四字段 `mode`／`candidates`／`decision`／`background`，
`selection()`／`basis()` 只是只读投影。P7D 按任务持有
`HashMap<TaskKey, SuspensionState>` **内存表**（`TaskKey` 私有；Intent 任务用 `IntentId::as_str()`），
`admit` 原样返回 P7C 的 `ResourceDecision`，`checkpoint` 走成功／晚到成功／端口失败三出口，超时由调用方
按 `checkpoint_deadline` 驱动。§171 只做 `SynthesisIo` 端口与三个转发；**不持阈值、不取时钟、不重试**。
P6D 合并门选 **rebaseline**：`p7_pre_allowed.txt` 31 行数据、`P7_EXPECTED_ROWS` 四行、两个 `27` → `31`、
断言 2 的 `5` → `4`（设计 §5.7、§9.2）。

**Tech Stack:** Rust 1.95.0 / edition 2024（workspace 的 `rust-version = "1.95"`）。
普通依赖：`continuum-arbiter`、`continuum-intent-scheduler`、`continuum-memory`、`continuum-semantics`。
dev 依赖：`continuum-canonical`、`trybuild`、`serde_json`。**不引入** `thiserror`／`serde`／`tokio`；
错误类型手写 `Display`／`Error`（设计 §4.9）。**生产代码不依赖** `continuum-persist`／
`continuum-events`／`continuum-product`／`continuum-runtime`／`continuum-provider`。

**Spec:** 设计全文（**唯一事实来源**）；切分 `docs/superpowers/specs/2026-10-18-p7-scope-and-split.md`
§五 P7D 行（`:337`）、§2.3.1、§2.3.2、§2.4、§三、§四第 1／2／3／4／5／6 条、§六.2 第 5／10 条；
规范 §299（`docs/spec/05-normative.md:1892-1903`）、§11（`docs/spec/01-concepts.md:576-620`）、
§12（`:624-665`）、§171（`docs/spec/03-product-drive.md:905-938`）、§156（`:377-406`）、
§223（`docs/spec/05-normative.md:172-205`）；《工程》§7.1（`docs/02-工程.md:428`、`:430`）、
§7.2（`:443`）、§7.3（`:453-454`）。**§7.4 的四条判据不覆盖 P7D**（切分 §七第 12 条）；
本块自报 J1–J22，不修改 §7.4。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。每个 task 的测试命令统一带
  `TMPDIR="$PWD/.tmp"`；`cargo test --workspace --no-fail-fast` 与
  `cargo build --workspace --all-targets` 必须 **0 warning**。
- **`TaskSource` 恰四值**：`AuthorizedIntent`／`Policy`／`MaintenanceSchedule`／`ProductDrive`；
  **没有第五变体、没有 `ModelSuggested`、没有 `parse`／`FromStr`**（设计 §4.2、J1）。
  四类来源的判定只在 `AuthorizedTask` 的四个构造器里（设计 §5.1），`src/` 里没有第二个
  `match TaskSource` 决定证据。
- **`AuthorizedTask` 只收 `READY` 的两路**：`authorized_intent(intent)` 要求
  `kind != Product` 且 `status == Ready`；`product_drive(intent)` 要求 `kind == Product`、
  `product_id` 非空、`status == Ready`；两路都要求 `contract.id` 非空、`contract.version >= 1`。
  `policy`／`maintenance` **不携带 Intent**，以非空引用为证据（设计 §4.3、§5.1、§5.2）。
- **`NoIntent` 保留既有会话**：`SupervisorDecision::NoIntent` 不得清空、关闭、丢弃
  `existing_sessions`、后台任务或 `BackgroundDigest`；`SessionPlan::selection()` 对它是 `None`，
  但 `decision()` 仍返回 `Some(NoIntent)`、`basis()` 仍有值（设计 §4.4、§5.4、J10-d）。
- **不另写排名**：`decide` 只调 `continuum_intent_scheduler::IntentScheduler::decide` 一次；
  `src/` 不出现 `impl IntentSelectionPolicy`、`fn choose`、`sort`／`sort_by`／`min`／`max`／
  `cmp`／`partial_cmp`／`BinaryHeap`／`BTreeSet`。`plan_manual` 不调 `decide`；`plan_auto` 的选择
  只来自 P7B（设计 §4.4、§4.6、§5.3、§5.4；J7／J10）。
- **§171 只提供触发点**：`SynthesisIo` 是 P7D 定义的端口，装配方实现并持 `Tx`；P7D 只做
  `begin`／`apply`／`fail` 三个转发。**不持阈值、不取时钟、不建定时器、不做空闲判定、不重试、
  不重定义 `SynthesisTrigger`**（设计 §4.8、§5.6；J14）。设计 §9.1 第 4 条的「只调 P6C 的函数」
  按 §4.8 的端口读：真正的 P6C 自由函数由端口实现调用，P7D 不命名 `Tx<'_>`。
- **零表零迁移零事件**：`Cargo.toml` 不登记 `continuum-persist`／`continuum-events`；
  `src/` 不出现 `Migration`／`Tx`／`INSERT`／`UPDATE`／`load_intent`／`list_children`／
  `save_intent`／`apply_intent_transition`／`Event`／`EventType`。**不碰 `intent` 表**；候选集是
  调用方给的 `&[Intent]`。`runtime_migrations()`、`expected_migrations()`、`tests/startup.rs`
  三处计数一行不动；**不占迁移 201**（设计 §9.1；J17／J19）。
- **`ALLOWED` 恰五项**：`("continuum-supervisor", &["continuum-arbiter", "continuum-canonical",
  "continuum-intent-scheduler", "continuum-memory", "continuum-semantics"])`。内层数组按字母序；
  **既有行不因本块增枚**；`continuum-runtime` 的 `ALLOWED` 行不因**本块**改动（P7A 落地时会由它自己
  加 `continuum-product`，那是 P7A 的 diff；P7D 合并门只断言 runtime 增量 ⊆ P7 名字）。（设计 §2.1；J16）
- **P6D 合并门 rebaseline 的数据写死**：`p7_pre_allowed.txt` **31 行数据**（等于 `c89d839` 的
  `ALLOWED`，不含任何 P7 crate）；`P7_EXPECTED_ROWS` 恰四行；解析器与断言 1 的两个 `27` → `31`；
  断言 2 的 `P6_EXPECTED_ROWS.len() == 5` → `P7_EXPECTED_ROWS.len() == 4`；旧
  `p6_pre_allowed.txt` 删除；测试改名 `p7_union_adds_only_its_own_rows`（设计 §5.7、§9.2；J15）。
- **`SessionPlan` 恰四字段**：`mode`／`candidates`／`decision`／`background`（设计 §4.5）。
  设计 §3.3 正文写「五字段」，与 §4.5 的冻结面不符；**本计划按 §4.5 的四字段执行**，不改设计，
  在「遗留/待协调」记一条文档订正。字段私有；无 `Default`／`Deserialize`；唯一构造点是
  `plan_manual`／`plan_auto`；`selection()` 不是第二个存储点（J10／J11）。
- **一个 ref 一个任务**：`PolicyGrantRef`／`MaintenanceOccurrenceRef` 在一个
  `BackgroundSupervisor` 实例里**只应标识一个任务**。两枚任务使用同一 ref 会共享同一
  `SuspensionState`——这是调用方契约违反，P7D 不检测、不去重；T11 用负例照片钉住这一行为。
- **Auto 的 `Resume` 选中项不必出现在 `candidates`**：`ContinueSession { disposition: Resume }`
  只要求 P7B 的 `Advance` 命中 `inputs.candidates`，不要求 `existing_sessions` 命中；`plan_auto`
  的候选列表仍只由 existing sessions → visible tasks → findings → `NewConversation` 构成。
  被 `Resume` 选中的 Waiting／Suspended Intent **不会**被补成 `SessionCandidate::Task`（T9 照片）。
- **`NewConversation` 的模式归属**：它是固定 UI 选项；`plan_manual` 与 `plan_auto` 的候选列表
  都以它收尾。设计 §5.4 明写 Auto 与 Manual 使用同一四段构造（existing → visible → findings →
  `NewConversation`）；设计 §4.5 的「Manual 的固定选项」按「Manual 必有」读，不表示 Auto 没有。
  T9 的照片钉住两模式都含它；措辞订正记入「遗留/待协调」。
- **源码面零命中一律去掉注释行再判**：所有 `grep` 型断言使用 `code_only()`——逐行丢弃
  `trim_start()` 以 `//` 开头的行（`///`／`//!` 同属注释），不把说明性注释算作命中。
  **边界写死**：本 crate 的 `src/` 只使用整行注释；**不使用 `/* ... */` 块注释，也不在代码后写
  `//` 行尾注释**。Task 14 的 `src_uses_only_full_line_comments` 先拍这一条；违反时先改源码的注释
  形式，不扩 `code_only`。J12 的 `Evidence`／`CompletionVerdict` 与 J17 的 `Tx` 只出现在注释里时
  **不算违规**；测试代码在 `tests/`，`src/` 语料不包含测试构造（设计 §4.1）。
- **测试全部放 `crates/continuum-supervisor/tests/`**，不放 `src/` 的 `#[cfg(test)]`；
  假 `IntentSelectionPolicy`／`SynthesisIo`／`CheckpointPort` 由 `tests/` 自己写。
- 代码注释、错误信息、测试断言信息用中文；标识符用英文。
- **不要用 `git add -A`，不要 `git commit --amend`**；只 `git add <显式路径>`。新增 crate 会改
  `Cargo.lock`，按显式路径一并提交。
- **范围与设计一致，不多做不少做**：不建执行器、不恢复会话、不迁移 Intent 状态、不写 Product、
  不建第二套 Resource Scheduler、不读 OS／进程／窗口／键盘、不调模型（设计 §8；J18／J20／J21）。

---

## Review Focus

复审时**逐条按下面的用例名核**；每条都归到拥有该代码的 task 的测试，不留在「设计说了」层面。

| # | 要试着推翻的断言 | 归属 task 与用例 |
|---|---|---|
| 1 | **`SuspensionState` 键碰撞**：同一个 `PolicyGrantRef`（或同一 `MaintenanceOccurrenceRef`）被两枚任务使用，第二枚任务从第一枚的 `Checkpointing` 相位出发，而不是从 `Running` 出发 | **Task 11** `same_policy_reference_shares_one_state_and_is_a_contract_violation`（第二枚 `admit` 返回 `action == None`、相位仍 `Checkpointing`）＋`distinct_policy_references_get_independent_states`；T11 的 `TaskKey::for_task` 是唯一键派生点 |
| 2 | **Auto 的 `Resume` 选中项不在 candidates**：Waiting／Suspended Intent 被 `Advance` 后，`plan_auto` 反而为它造了一个 `SessionCandidate::Task`，或 `selection()` 丢失 | **Task 9** `resume_selection_need_not_appear_in_candidates`（`decision()` 是 `ContinueSession { Resume }`、`selected_intent()` 命中；`candidates()` 只有 `NewConversation`，没有该 Intent 的 Task） |
| 3 | **`NoIntent` 清空既有会话**：`plan_auto` 用 `NoIntent` 当「没有会话」，把 candidates 里的 `ContinueSession`／`BackgroundFinding` 删掉，或把 `background` 置空 | **Task 9** `no_intent_keeps_existing_sessions_and_background`（J10-d；输入一条 session、一条 completed、一条 finding，逐项保留） |
| 4 | **checkpoint 超时路径丢失状态**：端口在截止后成功或定时器触发后，相位仍报 `Suspended`／停在 `Checkpointing`，或晚到结果被写回 | **Task 12** `late_success_returns_timed_out_and_returns_to_throttled`／`timer_hook_returns_timed_out_and_throttles`／`port_failure_returns_checkpoint_error_and_throttles`；三条都断言相位 `Throttled` 且 `checkpoint_deadline()` 为 `None` |
| 5 | **P6D 断言 5／8 在 rebaseline 后仍红**：只换 fixture、没改两个 `27`／断言 2，或把 P7 新 crate 混进 31 行基线；最终树断言 8 仍报「新增 subject 不在 `P7_NAMES`」 | **Task 2** Step 1 先看 `p6_union_adds_only_its_own_rows` 的断言 8 红（新增 `continuum-supervisor` 不在 `P6_NAMES`；旧断言 5 也可能因 P7A 的 `continuum-product` 红）→ Step 4 跑绿（runtime 增量 ⊆ `P7_NAMES`）；**Task 14** `allowed_row_has_exactly_five_entries`＋最终树 `cargo test -p continuum-runtime --test dependency_direction` |

---

## 跨计划前置（依赖的实际状态、P6D rebaseline、共写文件）

### 一、依赖交付物与**实际状态**（2026-10-19 实读本工作树，分支 `p7-plan`，基线 `c89d839`＋设计提交）

| 依赖 | 实际状态 | 本块用在哪 |
|---|---|---|
| **`continuum-semantics`** | **已交付**：`Intent`（`crates/continuum-semantics/src/intent.rs:434`）、`IntentId`（`src/ids.rs:41`）、`IntentKind`（`intent.rs:36`）、`IntentStatus`（`:61`）、`ContractRef`（`:331`）、`ContractId`（`ids.rs`）、`Priority`（`intent.rs:300`）、`ProductIdRef`（`:372`） | `AuthorizedTask`、`decide`、`next_tasks`、`SessionCandidate` |
| **`continuum-memory`** | **已交付**：`SynthesisRequest`／`SynthesisJobId`／`SynthesisProposal`／`SynthesisReport`／`MemoryError`（`src/lib.rs:28-31`、`src/error.rs:8`）；`begin_synthesis`／`fail_synthesis`／`apply_synthesis`（`src/synthesis.rs:108`／`:217`／`:306`）；`SynthesisTrigger`（`:64`） | `SynthesisIo` 的返回／参数类型；**不**重定义 `SynthesisTrigger` |
| **`continuum-canonical`（dev）** | **已交付**：`EntityId`（`src/candidate.rs:62`）、`Opaque`（`src/lib.rs:152`） | 仅 `tests/` 构造 `Intent` 夹具 |
| **`continuum-graph`** | **已交付**：`select_runnable`（`src/scheduler.rs:28`）／`SchedulerConfig` | **不是 P7D 的直接边**：它是 P7B 的普通依赖；P7D 只经 `IntentScheduler::decide` 消费，`Cargo.toml` 与 `ALLOWED` 都不登记它 |
| **`continuum-intent-scheduler`** | **未交付**：`ls crates/` 无此目录；仓库只有 P7B 的设计与计划（`docs/superpowers/specs/2026-10-18-p7b-intent-scheduler-design.md`、`docs/superpowers/plans/2026-10-19-p7b-intent-scheduler.md`） | `IntentScheduler`／`IntentDecision`／`IntentDecisionError`／`SchedulerInputs`／`IntentSelectionPolicy`／`DecisionBasis`。**硬前置** |
| **`continuum-arbiter`** | **未交付**：`ls crates/` 无此目录；只有 P7C 设计与计划 | `ResourceArbiter`／`ResourceDecision`／`SuspensionState`／`SuspensionPhase`／`SuspensionError`／`ResourceAction`／`WorkloadDeclaration`。**硬前置** |
| **`continuum-product`** | **未交付**，且**不是本块的依赖**：P7D 只按值收 `Intent`，不读 Product 记录 | 无 |
| **`trybuild` / `serde_json`** | **workspace 已登记**（根 `Cargo.toml` 的 `[workspace.dependencies]`） | dev 依赖；外部 crate 不进 `ALLOWED` |
| **`continuum-supervisor`** | **不存在**：`grep -rn "continuum-supervisor\|continuum_supervisor" crates/ Cargo.toml` 零命中 | 本计划的全部交付物 |

**执行起点（写死）**：**全部 14 个 task 等待 `continuum-intent-scheduler` 与 `continuum-arbiter`
落地；任一 crate 不在树里，不得先写任何 task（包括 Task 2 的 rebaseline）——连源码起草都不算
「已开始」。** Task 1 的 `Cargo.toml` 一次登记四条普通边（设计 §2.1），`ALLOWED` 的五项与
`cargo tree` 逐对相等；`cargo metadata` 失败即停。P7A／P7B／P7C 的实现不由本计划交付，本计划按
它们设计的冻结名写；落地时若导出名不符，**回报协调者，不在 P7D 侧另造同义类型**。
**最终树门只有一个出口：Task 14 Step 3。** 它要求 P7A／P7B／P7C／P7D 四个 crate 全部在 workspace
里（设计 §5.7 第 7 步）；Task 2 只负责 rebaseline 的**本地树绿**（31 行 fixture／四行常量／两个
`27`／断言 2），**不叫「最终树门」**，也不对四块齐全作承诺；单块树只算部分绿。

### 二、P6D 合并门 rebaseline（唯一路径；Task 2 执行）

**选 rebaseline，不退役。** 数据与步骤照设计 §5.7／§9.2：

**新 fixture** `crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt`，**31 行数据**，逐行等于
`c89d839` 的 `ALLOWED`（P7A／B／C 加行之前；不含 `continuum-product`／`continuum-intent-scheduler`／
`continuum-arbiter`／`continuum-supervisor`）。文件头：

```
# P7 合并门：c89d839 的 ALLOWED（31 行，P7 四个 crate 加入之前）。
# 改这个文件是 P7D 的合并门改动，不是 P7A／P7B／P7C 执行块的改动；合并评审必须单独核这一处 diff。
# 这是 P7 合并门，不是永久门；P8 起重新基线化或退役，不得在此追加行。
```

31 行数据（**照抄，不改顺序、不追加 P7 行**）：

```
continuum-core:
continuum-events:continuum-core
continuum-persist:continuum-events
continuum-memory:continuum-persist
continuum-provider:continuum-capability,continuum-core,continuum-persist
continuum-artifact:continuum-core,continuum-events,continuum-persist
continuum-port:continuum-artifact
continuum-operator:continuum-artifact
continuum-graph:continuum-artifact,continuum-events,continuum-port,continuum-operator,continuum-persist
continuum-workspace:continuum-events,continuum-persist
continuum-sandbox:continuum-workspace
continuum-effect:continuum-events,continuum-persist
continuum-policy:continuum-artifact,continuum-effect,continuum-persist
continuum-capability:continuum-core,continuum-effect,continuum-events,continuum-persist
continuum-secrets:continuum-capability
continuum-connector:continuum-capability,continuum-core,continuum-effect,continuum-persist,continuum-provider,continuum-secrets
continuum-model-registry:continuum-capability,continuum-core,continuum-persist
continuum-node:continuum-artifact
continuum-method:
continuum-canonical:continuum-persist
continuum-semantics:continuum-canonical,continuum-persist
continuum-budget:continuum-persist,continuum-semantics
continuum-media:continuum-artifact,continuum-graph,continuum-method,continuum-operator,continuum-verify
continuum-verify:continuum-artifact,continuum-events,continuum-graph,continuum-operator,continuum-persist
continuum-research:continuum-artifact,continuum-graph,continuum-method,continuum-operator,continuum-verify
continuum-code:continuum-artifact,continuum-graph,continuum-method,continuum-operator,continuum-verify,continuum-workspace
continuum-image:continuum-artifact,continuum-graph,continuum-operator,continuum-persist,continuum-verify
continuum-protected:
continuum-learning:continuum-protected
continuum-runtime:continuum-artifact,continuum-budget,continuum-canonical,continuum-capability,continuum-core,continuum-effect,continuum-events,continuum-graph,continuum-memory,continuum-model-registry,continuum-persist,continuum-policy,continuum-protected,continuum-provider,continuum-sandbox,continuum-secrets,continuum-semantics,continuum-verify,continuum-workspace
continuum-trust:
```

**`P7_EXPECTED_ROWS`**（四行；P7A→P7B→P7C→P7D 顺序）：

```rust
const P7_EXPECTED_ROWS: &[(&str, &[&str])] = &[
    // P7A（其设计 §2.1，四项）
    (
        "continuum-product",
        &[
            "continuum-canonical",
            "continuum-persist",
            "continuum-semantics",
            "continuum-verify",
        ],
    ),
    // P7B（其设计 §2.1，三项：两普通＋一 dev）
    (
        "continuum-intent-scheduler",
        &[
            "continuum-canonical",
            "continuum-graph",
            "continuum-semantics",
        ],
    ),
    // P7C（其设计 §2.1，零项）
    ("continuum-arbiter", &[]),
    // P7D（本设计 §2.1，五项：四普通＋一 dev）
    (
        "continuum-supervisor",
        &[
            "continuum-arbiter",
            "continuum-canonical",
            "continuum-intent-scheduler",
            "continuum-memory",
            "continuum-semantics",
        ],
    ),
];
```

**五处同改**：①fixture 换名／删除旧文件／改 `include_str!`；②`P6_EXPECTED_ROWS` →
`P7_EXPECTED_ROWS`、`p6_names` → `p7_names`、`pre_p6_allowed` → `pre_p7_allowed`；③解析器
`assert_eq!(rows.len(), 27, ...)`（`dependency_direction.rs:615`）→ `31`；④断言 1
`assert_eq!(baseline.len(), 27, ...)`（`:790`）→ `31`；⑤断言 2
`assert_eq!(P6_EXPECTED_ROWS.len(), 5, ...)`（`:799`）→ `P7_EXPECTED_ROWS.len(), 4`。
测试 `p6_union_adds_only_its_own_rows`（`:784`）改名 `p7_union_adds_only_its_own_rows`；注释
`:551-573`／`:779-782` 同步改成 P7／31 行。**不在 P6 fixture 上追加行**。

### 三、共写文件（只有这些）

| 文件 | 本块的改动 | 落在哪个 task |
|---|---|---|
| 根 `Cargo.toml` 的 `[workspace] members`（`:3`） | 加 `"crates/continuum-supervisor"` | Task 1 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `continuum-supervisor` 五项（四普通＋一 dev） | Task 1 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 P6D 合并门（`:551`／`:574`／`:582`／`:615`／`:784`／`:790`／`:799`） | rebaseline：常量、函数名、两个 `27`、断言 2、测试名、注释 | Task 2 |
| `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` | 删除 | Task 2 |
| `crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt` | 新建，31 行数据 | Task 2 |
| `Cargo.lock` | 随新 crate 增枚 | Task 1（按显式路径提交） |

**不碰**：`crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`）、
`crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`）、
`crates/continuum-runtime/tests/startup.rs` 的三处计数（`:26`／`:81`／`:94`）、
`continuum-runtime` 的 `ALLOWED` 行、`docs/02-工程.md`、`docs/spec/**`、`docs/superpowers/specs/**`。

### 四、六条复评 Minor 在本计划里的落点

| Minor | 本计划的处置 | 落点 |
|---|---|---|
| §3.3 的「五字段」与 §4.5 的四字段相抵 | 按 §4.5 的**四字段**实现（`mode`／`candidates`／`decision`／`background`）；不改设计，记入遗留 | Global Constraints；Task 8／Task 10；自审 1 |
| J12／J17 的源码面零命中会误伤注释 | 所有零命中用 `code_only()` 去注释行；`Evidence`／`CompletionVerdict`／`Tx` 只在注释里不算命中 | Global Constraints；Task 8（J12）；Task 14（J17） |
| §9.1 把 §171 写成「只调 P6C 的函数」 | 按 §4.8 的端口写：P7D 定义并调用 `SynthesisIo`，装配方实现并持 `Tx` | Global Constraints；Task 13；自审 3 |
| policy／maintenance 的「一 ref 一任务」契约 | 写进 `TaskKey` 的文档与 T11 的负例：同一 ref 的两枚任务共享状态，是调用方契约违反 | Global Constraints；Task 11 |
| Auto 的 `Resume` 选中项与 candidates 的关系 | 写死：`Resume` 不要求 `existing_sessions`；被选中的 Waiting／Suspended 不补进 candidates | Global Constraints；Task 9 |
| `NewConversation` 的模式归属 | 写死：两模式的候选列表都以它收尾（§5.4 同一四段）；§4.5 的「Manual 的固定选项」按「Manual 必有」读 | Global Constraints；Task 9；遗留记措辞订正 |

### 五、各 task 的「运行，确认失败」量到的是哪一种失败

| task | Step 2 量到的是 | 为什么 |
|---|---|---|
| Task 1 | **编译失败 `E0432`** | 测试先写，`TaskSource`／`PolicyGrantRef` 由 Step 4 才建出；crate 骨架与 `ALLOWED` 已在 Step 1 |
| Task 2 | **运行期红（断言 8；可能兼有断言 5）** | `continuum-supervisor` 已进 `ALLOWED`，但 `P6_NAMES` 不认识它；P7A 的 `continuum-product` 也可能让旧断言 5 红；rebaseline 后都绿 |
| Task 3 | **编译失败 `E0432`／`E0599`** | `AuthorizedTask` 与四个构造器由 Step 3 才建出 |
| Task 4 | **trybuild 红（`.stderr` 缺失／不符）** | 样例先写，`.stderr` 在 Step 2 实跑取；本 task 不交付生产代码 |
| Task 5 | **编译失败 `E0432`／`E0451`** | `SupervisorError`／`ContinueDisposition`／`SupervisorDecision` 由 Step 3 才建出 |
| Task 6 | **编译失败 `E0432`／`E0599`** | `BackgroundSupervisor`／`next_tasks` 由 Step 3 才建出 |
| Task 7 | **编译失败 `E0599`／`E0432`** | `decide` 由 Step 3 才建出；依赖 P7B 的 `IntentScheduler` |
| Task 8 | **编译失败 `E0432`／`E0599`** | `SessionPlan`／`SessionOrchestrator::plan_manual` 由 Step 3 才建出 |
| Task 9 | **编译失败 `E0599`** | `plan_auto` 由 Step 3 才建出 |
| Task 10 | **trybuild 红（`.stderr` 缺失／不符）** | 样例先写，`.stderr` 在 Step 2 实跑取 |
| Task 11 | **编译失败 `E0599`／`E0432`** | `admit`／`release` 由 Step 3 才建出；依赖 P7C |
| Task 12 | **编译失败 `E0599`** | `checkpoint`／`checkpoint_deadline`／`checkpoint_timed_out` 由 Step 3 才建出 |
| Task 13 | **编译失败 `E0432`** | `SynthesisTriggerPoint` 由 Step 3 才建出 |
| Task 14 | **首次即绿**（源码面） | 本 task 只新建断言；若红，说明前面 task 的源码漂了，回查拥有该代码的 task |

**「首次即绿」不是缺陷**：Task 14 的交付物就是断言本身，它的红由 Task 14 Step 4 的变异复核取。
**编译失败不算红**（P5c 纪律 1(c)）；本表照实标注，不把 `E0432` 写成断言失败。

---

## File Structure

设计 §2.2 的模块表**是唯一依据**。**不建 `src/decision.rs`**：`SupervisorDecision` 与
`ContinueDisposition` 按设计落在 `src/supervisor.rs`；任务材料里的 `decision.rs` 不是设计表里的文件。

```
crates/continuum-supervisor/
  Cargo.toml             四条普通依赖；dev：continuum-canonical / serde_json / trybuild
  src/lib.rs             只做 pub mod 与 pub use；不定义类型（设计 §2.2）
  src/task.rs            TaskSource / TaskError / AuthorizedTask / PolicyGrantRef / MaintenanceOccurrenceRef
  src/supervisor.rs      BackgroundSupervisor / TaskKey / SupervisorDecision / ContinueDisposition /
                         next_tasks / decide / admit / checkpoint / checkpoint_timed_out /
                         checkpoint_deadline / suspension_phase / release
  src/session.rs         SessionMode / SessionCandidate / BackgroundNote / BackgroundDigest /
                         SessionPlan / SessionOrchestrator / plan_manual / plan_auto
  src/resource.rs        CheckpointPort / CheckpointError；declaration_for（私有）
  src/trigger.rs         SynthesisIo / SynthesisTriggerPoint
  src/error.rs           SupervisorError（十臂）
  tests/support/mod.rs   源码语料 code_only()／函数体提取；Intent／AuthorizedTask 夹具；
                         假 IntentSelectionPolicy／CheckpointPort／SynthesisIo；
                         首行 #![allow(dead_code)]（共享夹具，各 test crate 只用一部分）
  tests/task_source.rs   J1-a／J1-b；J6 的引用形状（Task 1）
  tests/authorized_task.rs J3／J4／J5／J6／J22（Task 3）
  tests/compile_fail.rs  J2 与 J6 类型不匹配的 trybuild 驱动（Task 4）
  tests/decision.rs      SupervisorDecision 访问器与 SupervisorError 十臂（Task 5）
  tests/next_tasks.rs    J8（Task 6）
  tests/decide.rs        J7-c／J9 的八组（Task 7）
  tests/session_plan.rs  J10-b／J10-c／J11 字段面／J12 运行期（Task 8）
  tests/plan_auto.rs     J10-a／J10-d、Resume 与 candidates、NewConversation（Task 9）
  tests/type_level.rs    J11 的 trybuild 与 serde_json 负例（Task 10）
  tests/resource.rs      J13-a／J13-d、一 ref 一任务（Task 11）
  tests/checkpoint.rs    J13-c 的三出口与定时器（Task 12）
  tests/trigger.rs       J14（Task 13）
  tests/source_surface.rs J7-a/b、J10-b、J12、J13-b、J14、J16–J21 的源码面（Task 7／8／11／13／14）
  tests/compile_fail/*.rs（＋.stderr）  Task 4／Task 10 的样例
  tests/ui_pass/*.rs                    正控制
```

**为什么 `SupervisorError` 单独在 `src/error.rs`**：它横跨任务面、决策面、资源面与记忆面；放在任一
功能模块里会让其余模块反向依赖它。`CheckpointError` 留在 `src/resource.rs`（设计 §2.2），
`SupervisorError::Checkpoint` 引用它，模块间循环只在类型引用层面，不产生运行时依赖。

每个 `tests/*.rs` 顶部写 `mod support;`；`tests/support/mod.rs` 只放夹具与 `code_only`／
`fn_body`，不放生产逻辑。集成测试的目标名与文件名一致（`tests/task_source.rs` →
`cargo test -p continuum-supervisor --test task_source`）。

---

### Task 1: `continuum-supervisor` 骨架、`TaskSource` 与两个授权引用

**Files:**
- Create: `crates/continuum-supervisor/Cargo.toml`
- Create: `crates/continuum-supervisor/src/lib.rs`
- Create: `crates/continuum-supervisor/src/task.rs`
- Create: `crates/continuum-supervisor/src/error.rs`（空模块，Task 5 填 `SupervisorError`）
- Create: `crates/continuum-supervisor/src/resource.rs`（空模块，Task 5 填端口类型）
- Create: `crates/continuum-supervisor/src/supervisor.rs`（空模块，Task 5 起填类型）
- Create: `crates/continuum-supervisor/src/session.rs`（空模块，Task 8 起填类型）
- Create: `crates/continuum-supervisor/src/trigger.rs`（空模块，Task 13 填端口）
- Create: `crates/continuum-supervisor/tests/task_source.rs`
- Create: `crates/continuum-supervisor/tests/support/mod.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加 `"crates/continuum-supervisor"`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加五项）

**Interfaces:**
- Consumes: `continuum_semantics::{ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority, ProductIdRef}`；`continuum_canonical::{EntityId, Opaque}`（仅测试）
- Produces: `continuum_supervisor::{TaskSource, TaskError, PolicyGrantRef, MaintenanceOccurrenceRef}`；
  `AuthorizedTask` **本 task 不定义**，由 Task 3 交付

- [ ] **Step 1: 建 crate 骨架、member 与 `ALLOWED` 行**

创建 `Cargo.toml`：

```toml
[package]
name = "continuum-supervisor"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-arbiter = { path = "../continuum-arbiter" }
continuum-intent-scheduler = { path = "../continuum-intent-scheduler" }
continuum-memory = { path = "../continuum-memory" }
continuum-semantics = { path = "../continuum-semantics" }

[dev-dependencies]
continuum-canonical = { path = "../continuum-canonical" }
serde_json = { workspace = true }
trybuild = { workspace = true }
```

`src/lib.rs` 只写：

```rust
//! P7D Background Supervisor + Session Orchestrator（设计 §2.2）。
//! 本 crate 零表、零迁移、零事件；不执行会话恢复、不写 intent 表。

pub mod error;
pub mod resource;
pub mod session;
pub mod supervisor;
pub mod task;
pub mod trigger;

pub use task::{MaintenanceOccurrenceRef, PolicyGrantRef, TaskError, TaskSource};
```

本 task 先创建空的 `src/error.rs`／`src/resource.rs`／`src/session.rs`／`src/supervisor.rs`／
`src/trigger.rs`（只有模块文档，没有类型）；**`lib.rs` 只导出本 task 已存在的四个 task 类型**，
后续 task 各自追加自己的 `pub use` 行（Task 3 加 `AuthorizedTask`，Task 5 加错误与端口类型，
Task 6 加 `BackgroundSupervisor`，Task 8 加 session 类型，Task 13 加 trigger 类型）。
**不要**提前写未实现的类型或 `todo!()`。根 `Cargo.toml` 的 `members` 加 `"crates/continuum-supervisor"`；
`ALLOWED` 在 `continuum-trust` 行之后追加：

```rust
    // P7D Background Supervisor + Session Orchestrator（设计 §2.1）。
    // 四普通边：arbiter（资源结论）、intent-scheduler（唯一排名）、
    // memory（§171 触发类型）、semantics（Intent 与证据）。
    // 一条 dev 边：canonical（仅 tests/ 构造 Intent 夹具）。
    (
        "continuum-supervisor",
        &[
            "continuum-arbiter",
            "continuum-canonical",
            "continuum-intent-scheduler",
            "continuum-memory",
            "continuum-semantics",
        ],
    ),
```

**结果：** `cargo metadata --no-deps` 的成员含 `continuum-supervisor`；`ALLOWED` 的该行恰五项；
`cargo test -p continuum-supervisor --no-run` 能解析（空模块可编译）。**此刻
`cargo test -p continuum-runtime --test dependency_direction` 的断言 8 会红**（新 subject 不在
`P6_NAMES`），这是 Task 2 的入口，不要把红当回归。

- [ ] **Step 2: 写 `tests/task_source.rs` 与夹具**

`tests/support/mod.rs` 首行写 `#![allow(dead_code)]`（共享夹具；每个测试二进制只用其中一部分，
P6A 计划 777 有先例；不加会让各 test crate 的未用 helper 触发 `dead_code`，0 warning 门会红），
再写 `pub fn code_only(source: &str) -> String`：逐行丢弃 `trim_start().starts_with("//")` 的行，
其余原样拼接。**边界写死**：本 crate 的 `src/` 只使用整行注释——不使用 `/* ... */` 块注释，也不在
代码后写 `//` 行尾注释；Task 14 的 `src_uses_only_full_line_comments` 会先断言这一点，之后所有零
命中都靠 `code_only`。再写 `pub fn intent(kind: IntentKind, status:
IntentStatus) -> Intent` 与 `pub fn product_intent(status: IntentStatus) -> Intent`，字段逐项：
`id: IntentId::from_str("intent-1")`、`version: 1`、`goal: EntityId::from_str("goal")`、
`priority: Priority::from_str("normal")`、`created_at: 0`、`updated_at: 0`、`parent_intent: None`、
`contract_id: ContractRef { id: ContractId::from_str("contract-1"), version: 1 }`、
`active_plan_id: None`、`graph_id: None`、`product_id: None`（Product 夹具用
`Some(ProductIdRef::from_str("product-1"))`）、`completion_predicate: Opaque::from_str("done")`。
`product_intent` 的 `kind` 是 `IntentKind::Product`。

`tests/task_source.rs` 写四条：

- `task_source_has_exactly_four_values_and_verbatim_labels`：对四个变体逐个调 `as_str()`，断言
  等于 `"authorized_intent"`／`"policy"`／`"maintenance_schedule"`／`"product_drive"`；写一个
  **无 `_` 通配臂**的 `fn exhaustive(source: TaskSource) -> &'static str` 覆盖四变体（编译期穷尽）。
- `task_source_has_no_parser_or_model_suggested_variant`：读 `code_only` 后的 **`src/` 全部文件**
  （`lib.rs` 与六个模块，不只 `task.rs`），断言四个来源词根 `ModelSuggested`／`TaskSource::Model`／
  `UserRequested`／`SystemIdle` 零命中，且无 `impl FromStr for TaskSource`／`fn parse`。另读
  `code_only(include_str!("../src/task.rs"))` 重复同一组来源词根（J1-a）；两处都必须空。
- `policy_and_maintenance_refs_reject_empty_fields`：`PolicyGrantRef::new("", "g")` 与
  `new("i", "")` ⇒ `Err(TaskError::EmptyPolicyGrant)`；`MaintenanceOccurrenceRef::new("", "o")`
  与 `new("s", "")` ⇒ `Err(TaskError::EmptyMaintenanceRef)`；非空构造的 `issuer()`／`grant_id()`／
  `schedule_id()`／`occurrence_id()` 逐字返回。
- `task_error_display_covers_all_six_arms`：构造每个 `TaskError` 臂（`NotReady`／`WrongKind`／
  `InvalidContract`／`MissingProductId`／`EmptyPolicyGrant`／`EmptyMaintenanceRef`），断言
  `Display` 非空、可读，且 `std::error::Error` 的 `source()` 为 `None`（本类型没有内层错误）。

**结果：** 四条用例名可列出；Step 3 之前是 `E0432`（`TaskSource` 未定义）。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test task_source
```

**结果：** 编译失败 `E0432`／`E0599`（`TaskSource`／`PolicyGrantRef`／`MaintenanceOccurrenceRef`
尚未定义）。按 P5c 纪律，这**不是红**；Step 4 之后才是断言级验证。

- [ ] **Step 4: 实现 `task.rs` 的四个类型**

`TaskSource` 逐变体照设计 §4.2，派生 `Debug, Clone, Copy, PartialEq, Eq`；`as_str` 的四个字面量
写死；**不实现 `FromStr`／`parse`**。`TaskError` 六臂照设计 §4.3，派生
`Debug, Clone, PartialEq, Eq`，手写 `Display`／`Error`。`PolicyGrantRef`／`MaintenanceOccurrenceRef`
字段私有，派生 `Debug, Clone, PartialEq, Eq`，`new` 只检查 `is_empty()`（**不 trim、不改写**）；
访问器返回 `&str`。
`AuthorizedTask` 与四个构造器**留到 Task 3**；本 task 的 `task.rs` 不写它们，避免半成品。

**结果：** `cargo test -p continuum-supervisor --test task_source` 全绿；`src/task.rs` 的
`TaskSource` 恰四个变体；`code_only` 的零命中成立。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-supervisor Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(supervisor): crate 骨架、TaskSource 四值与授权引用"
```

---

### Task 2: P6D 合并门 rebaseline

**Files:**
- Create: `crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt`（31 行数据，内容见「跨计划前置」二）
- Delete: `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`:551` 起的常量、`:574` 的
  `p6_names`、`:582` 的 `pre_p6_allowed`、`:615`／`:790` 的两个 `27`、`:799` 的断言 2、`:784` 的
  测试名、`:551-573`／`:779-782` 的注释）

**Interfaces:**
- Consumes: 当前 `ALLOWED`（Task 1 之后的 32 行；本 fixture 只取 P7 前的 31 行）
- Produces: `p7_pre_allowed.txt`、`P7_EXPECTED_ROWS`、`p7_names()`、`pre_p7_allowed()`、
  `p7_union_adds_only_its_own_rows`

- [ ] **Step 1: 运行当前合并门，记录断言 8 红**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction
```

**结果：** `p6_union_adds_only_its_own_rows` 失败，至少断言 8 命中「新增 subject 必须恰是已落地的
P6 crate」（`continuum-supervisor` 已在 `ALLOWED`，但不在 `P6_NAMES`）。**断言 5 也可能同时红**：
这一步还是旧门，用的是 `P6_NAMES`；P7A 已把 `continuum-product` 加进 runtime 行，这个增量是合法的
P7 增量，Step 3 把 `p6_names` 换成 `p7_names` 后会绿。**不要**因为断言 5 红去回滚 `ALLOWED`；只有
runtime 行出现 P7 名字之外的新增才是误改。最终口径是：runtime 增量 **⊆ P7 名字**——P7A 落地时为
`{continuum-product}`，只有 P7D 的单块树时为空；两者都合法。

- [ ] **Step 2: 新建 `p7_pre_allowed.txt`，删除旧 fixture**

把「跨计划前置」二的 31 行数据**逐行照抄**到
`crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt`，文件头三行照抄。用
`git rm crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 删除旧文件，**不要在旧文件上
追加 P7 行，也不要留下第二套基线**。

**结果：** `grep -vc '^#' p7_pre_allowed.txt` 输出 `31`；`test ! -e p6_pre_allowed.txt` 成功；
文件中不含 `continuum-product`／`continuum-intent-scheduler`／`continuum-arbiter`／
`continuum-supervisor`。

- [ ] **Step 3: 改常量、函数名、两个 `27` 与断言 2**

在 `dependency_direction.rs` 中按下面五处同批改：

1. `const P6_EXPECTED_ROWS` → `const P7_EXPECTED_ROWS`，数组体换成「跨计划前置」二的四行；
   上方注释从「P6 的五条期望行」改成「P7 的四条期望行」（P7A／B／C／D），所有权注释改成 P7D。
2. `fn p6_names()` → `fn p7_names()`，函数体仍由 `P7_EXPECTED_ROWS` 派生；调用点（`:800`／`:857`
   等）全部改名。
3. `fn pre_p6_allowed()` → `fn pre_p7_allowed()`；`include_str!("fixtures/p6_pre_allowed.txt")` →
   `include_str!("fixtures/p7_pre_allowed.txt")`；文档改成「P7 前基线（`c89d839` 的 `ALLOWED`，
   31 行）」；`assert_eq!(rows.len(), 27, ...)` → `31`。
4. 断言 1 的 `assert_eq!(baseline.len(), 27, ...)` → `31`（两处 panic 文案同步改 P7／31）。
5. 断言 2 的 `assert_eq!(P6_EXPECTED_ROWS.len(), 5, ...)` → `P7_EXPECTED_ROWS.len(), 4`；
   断言文案改成「P7_EXPECTED_ROWS 必须是四行」。

测试 `p6_union_adds_only_its_own_rows` → `p7_union_adds_only_its_own_rows`；文件头注释
`:779-782` 的「P6 合并门」「27 行」→「P7 合并门」「31 行」；断言 6／7／8 的循环变量与文案里
`P6` 改成 `P7`。**保留 `p6d_parse_allowed_source` 的函数名**（它解析的是 `ALLOWED` 源码，不是
P6 基线；设计 §5.7 的改名清单不含它）。

**结果：** `grep -n "P6_EXPECTED_ROWS\|p6_names\|p6_union_adds_only_its_own_rows\|p6_pre_allowed" 
crates/continuum-runtime/tests/dependency_direction.rs` 零命中；`grep -n "rows.len(), 31"`
与 `grep -n "baseline.len(), 31"` 各一处；`grep -n "P7_EXPECTED_ROWS.len(), 4"` 一处。

- [ ] **Step 4: 跑合并门，确认本地树绿**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction
```

**结果：** `p7_union_adds_only_its_own_rows` 全绿；断言 1 是 31 行、断言 2 是四行、断言 5 的
runtime 增量 ⊆ P7 名字（P7A 落地时为 `{continuum-product}`，单 P7D 树时为空）、断言 8 的新增
subject 恰是已落地的 P7 crate。**Task 2 只证明本地树（P6D rebaseline）绿**；最终树门只在 Task 14
Step 3 的单一出口复核四块齐全，这里不报「最终树」。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-runtime/tests/dependency_direction.rs crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt
git commit -m "test(runtime): P6D 合并门 rebaseline 到 P7 前 31 行"
```

Step 2 的 `git rm` 已把旧 fixture 的删除放进暂存区；这里不再重复 `git rm --cached`，也不要在
提交前把旧文件恢复出来。

---

### Task 3: `AuthorizedTask` 的四个构造器与校验顺序

**Files:**
- Modify: `crates/continuum-supervisor/src/task.rs`
- Modify: `crates/continuum-supervisor/src/lib.rs`（`pub use task::{...}` 加 `AuthorizedTask`）
- Create: `crates/continuum-supervisor/tests/authorized_task.rs`
- Modify: `crates/continuum-supervisor/tests/support/mod.rs`（补 `task()` 夹具与 `product_task()`）

**Interfaces:**
- Consumes: Task 1 的 `TaskSource`／`TaskError`／`PolicyGrantRef`／`MaintenanceOccurrenceRef`；
  `continuum_semantics::{Intent, IntentKind, IntentStatus, ContractRef}`
- Produces:
  `AuthorizedTask::{authorized_intent, product_drive, policy, maintenance}`（四个均返回 `Result<Self, TaskError>`）；
  `source() -> TaskSource`、`intent() -> Option<&Intent>`、`contract() -> Option<&ContractRef>`、
  `policy_grant() -> Option<&PolicyGrantRef>`、`maintenance_schedule() -> Option<&MaintenanceOccurrenceRef>`

- [ ] **Step 1: 写 `tests/authorized_task.rs` 与夹具**

夹具：`fn task(intent: Intent) -> AuthorizedTask` 调 `AuthorizedTask::authorized_intent(intent).expect(...)`；
`fn product_task(status: IntentStatus) -> AuthorizedTask` 用 `product_intent(status)` 调
`product_drive(...).expect(...)`（仅 READY 用）。测试：

- `product_drive_accepts_only_ready_across_the_thirteen_states`（J3）：对
  `Proposed`／`Specifying`／`Planning`／`Review`／`Ready`／`Running`／`Waiting`／`Suspended`／
  `Verifying`／`Completed`／`Evolving`／`Failed`／`Cancelled` 各造一枚 Product Intent（`product_id`
  与 contract 都有效）；只有 `Ready` 是 `Ok`，其余逐臂断言 `Err(TaskError::NotReady { status })`
  且 `status` 等于输入。
- `product_drive_rejects_running_constructed_directly`（J3）：直接构造
  `Intent { status: Running, ..product_intent(Ready) }`（`Intent` 字段是 `pub`，不经
  `apply_intent_transition`）；`product_drive` ⇒ `Err(NotReady { status: Running })`。**照片只证
  RUNNING 被拒绝**；「RUNNING 可绕过 REVIEW」由设计 §3.7 的 P4 迁移表证明，本测试不重走迁移。
- `authorized_intent_rejects_product_and_product_drive_rejects_non_product`（J4）：
  `product_intent(Ready)` 走 `authorized_intent` ⇒ `WrongKind { kind: Product }`；
  `intent(Task, Ready)` 走 `product_drive` ⇒ `WrongKind { kind: Task }`；
  `intent(Task, Ready)` 走 `authorized_intent` ⇒ `Ok`。
- `contract_and_product_id_are_validated_in_order`（J5）：`product_drive` 对
  `contract.id == ""` ⇒ `InvalidContract { id: "", version: 1 }`；`version == 0` ⇒
  `InvalidContract { .. }`；`product_id: None`／`Some(ProductIdRef::from_str(""))` ⇒
  `MissingProductId`；`authorized_intent` 对同两类 contract 负例 ⇒ `InvalidContract`。
  **顺序照片**：Product Intent 且 `status == Running`、contract 为空 ⇒ 必须是 `NotReady`
  （kind／product_id 先于 status，status 先于 contract）。
- `authorized_intent_accessors_match_the_evidence`（J4／J5）：`source() == AuthorizedIntent`；
  `intent()` 的 `id.as_str()` 与输入相同；`contract()` 是 `Some` 且 `id.as_str()`／`version` 相同；
  `policy_grant()`／`maintenance_schedule()` 都是 `None`。
- `policy_and_maintenance_tasks_have_no_intent_or_contract`（J6）：`policy(grant)` 的
  `source() == Policy`、`intent() == None`、`contract() == None`、`policy_grant() == Some(grant)`；
  `maintenance(schedule)` 对称；两枚任务都可 `Clone`，克隆后来源与引用逐字相同。
- `product_drive_reads_product_id_only_from_intent`（J22）：`product_id` 缺失／空串都拒绝；
  `Some("product-1")` 通过；`src/` 的 `code_only` 不含 `continuum_product`／`Product::`／
  `Milestone`／`load_product`（这条同时由 Task 14 复核）。

**结果：** 七条用例名可列出；`AuthorizedTask` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test authorized_task
```

**结果：** 编译失败 `E0432`／`E0599`（`AuthorizedTask` 与构造器由 Step 3 才建出）。

- [ ] **Step 3: 实现 `AuthorizedTask`**

按设计 §4.3 写字段私有的结构体与私有 `TaskEvidence` 枚举；四个构造器逐条按设计 §5.2 的顺序：

- `authorized_intent`：`kind != Product` → `WrongKind`；`status == Ready` → 否则 `NotReady`；
  contract `id` 非空且 `version >= 1` → 否则 `InvalidContract`。
- `product_drive`：`kind == Product` → 否则 `WrongKind`；`product_id` 是 `Some` 且
  `as_str()` 非空 → 否则 `MissingProductId`；`status == Ready` → 否则 `NotReady`；
  contract 两条同上 → 否则 `InvalidContract`。
- `policy`：`grant` 两字段非空 → 否则 `EmptyPolicyGrant`；`maintenance` 对称 →
  `EmptyMaintenanceRef`。
- **没有 `new(source, evidence)`**；`TaskEvidence` 私有；字段私有。

访问器按 Interfaces 返回。`AuthorizedTask` 派生 `Debug, Clone`，私有 `TaskEvidence` 同样派生
`Debug, Clone`；**不派生** `PartialEq`／`Eq`（`Intent` 没有这些能力，设计 §4.10）。

**结果：** `cargo test -p continuum-supervisor --test authorized_task` 全绿；J3 的十三态逐臂、
J4 的两个 `WrongKind`、J5 的四类负例、J6 的引用形状、J22 的 `product_id` 都可观察。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/task.rs crates/continuum-supervisor/src/lib.rs crates/continuum-supervisor/tests/authorized_task.rs crates/continuum-supervisor/tests/support/mod.rs
git commit -m "feat(supervisor): AuthorizedTask 四构造器与 READY 证据"
```

---

### Task 4: `AuthorizedTask` 的唯一构造点（trybuild）

**Files:**
- Create: `crates/continuum-supervisor/tests/compile_fail.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/authorized_task_literal_is_private.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/authorized_task_has_no_new.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/policy_grant_is_not_a_maintenance_ref.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/maintenance_ref_is_not_a_policy_grant.rs`
- Create: `crates/continuum-supervisor/tests/ui_pass/public_surface_is_reachable.rs`
- Create: 四个 `.stderr`（Step 2 实跑取，不预写）

**Interfaces:**
- Consumes: Task 3 的 `AuthorizedTask`／`PolicyGrantRef`／`MaintenanceOccurrenceRef`（**使用侧**）
- Produces: 无生产类型；J2 与 J6 的编译期照片

- [ ] **Step 1: 写四份 `compile_fail` 与一份 `ui_pass`**

- `authorized_task_literal_is_private.rs`（J2）：
  ```rust
  use continuum_supervisor::{AuthorizedTask, TaskSource};

  fn main() {
      let _ = AuthorizedTask {
          source: TaskSource::Policy,
          evidence: todo!(),
      };
  }
  ```
  字段私有 ⇒ 编译失败；样例里**不** `use` `TaskEvidence`（它是私有的）。
- `authorized_task_has_no_new.rs`（J2）：
  ```rust
  use continuum_supervisor::{AuthorizedTask, TaskSource};

  fn main() {
      let _ = AuthorizedTask::new(TaskSource::Policy, todo!());
  }
  ```
  没有 `new` ⇒ `E0599`。
- `policy_grant_is_not_a_maintenance_ref.rs`（J6）：
  ```rust
  use continuum_supervisor::{AuthorizedTask, PolicyGrantRef};

  fn main() {
      let grant = PolicyGrantRef::new("issuer", "grant").unwrap();
      let _ = AuthorizedTask::maintenance(grant);
  }
  ```
  类型不匹配 ⇒ `E0308`。
- `maintenance_ref_is_not_a_policy_grant.rs`（J6）：对称，`AuthorizedTask::policy(maintenance_ref)`
  ⇒ `E0308`。
- `ui_pass/public_surface_is_reachable.rs`：函数接收 `&AuthorizedTask` 并调用 `source()`／
  `intent()`／`contract()`；**不构造值**，加 `fn main() {}`。它只证明 crate 依赖接上了，
  把「依赖没接上」与「构造点被锁住」分开。
- `tests/compile_fail.rs`：`trybuild::TestCases::new()`，四份 `compile_fail` ＋ 一份 `pass`。

**结果：** 五份样例与驱动存在；四份 `.stderr` 尚未写。

- [ ] **Step 2: 运行，取 `.stderr`**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test compile_fail
```

**结果：** 运行期红：trybuild 报 `.stderr` 缺失或与样例不符。把**实跑输出**逐份写入同名
`.stderr`（四份各不相同，不许共用一个错误码）；再跑一次确认绿。`.stderr` 的内容以实跑为准，
**不许凭记忆写**；改一句注释也会改 `.stderr`。

- [ ] **Step 3: 不加生产代码**

本 task **只交付样例与 `.stderr`**。**不加** `pub` 字段、`new`、`Default`、`Deserialize`；
`AuthorizedTask` 仍只有四个具名构造器（Task 3 已交付）。**不把 `trybuild` 写进 `ALLOWED`**：
它是外部 dev 依赖。

**结果：** `cargo test -p continuum-supervisor --test compile_fail` 全绿；`src/task.rs` 一字未动。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/tests/compile_fail.rs crates/continuum-supervisor/tests/compile_fail crates/continuum-supervisor/tests/ui_pass
git commit -m "test(supervisor): AuthorizedTask 唯一构造点的 trybuild 照片"
```

---

### Task 5: 决策类型与错误面（`SupervisorDecision`／`ContinueDisposition`／`SupervisorError`）

**Files:**
- Modify: `crates/continuum-supervisor/src/supervisor.rs`
- Modify: `crates/continuum-supervisor/src/resource.rs`（只放 `CheckpointPort`／`CheckpointError`，供
  `SupervisorError::Checkpoint` 命名；`admit`／`checkpoint` 留给 Task 11／12）
- Modify: `crates/continuum-supervisor/src/error.rs`（Task 1 已建空模块，这里填 `SupervisorError`）
- Modify: `crates/continuum-supervisor/src/lib.rs`（导出上述类型）
- Create: `crates/continuum-supervisor/tests/decision.rs`

**Interfaces:**
- Consumes: Task 3 的 `AuthorizedTask`；`continuum_intent_scheduler::{IntentDecision, IntentDecisionError, DecisionBasis}`；
  `continuum_arbiter::{SuspensionError, SuspensionPhase}`；`continuum_memory::MemoryError`；
  Task 1 的 `TaskSource`／`TaskError`
- Produces:
  `ContinueDisposition::{Continue, Resume}`；
  `SupervisorDecision::{Task { task, decision }, ContinueSession { intent, disposition, decision }, NoIntent { decision }}`
  与 `intent_decision()`／`selected_intent()`／`basis()`／`continue_disposition()`；
  `SupervisorError` 十臂（照设计 §4.9）；`CheckpointPort`／`CheckpointError` 的类型面

- [ ] **Step 1: 写 `tests/decision.rs`**

- `supervisor_decision_projects_the_policy_result`：造一枚 `IntentDecision::Advance { intent, basis }`
  包在 `SupervisorDecision::ContinueSession { disposition: Resume, .. }` 里，断言
  `intent_decision()` 与 `basis().as_str()` 一致、`selected_intent()` 是同一 id、
  `continue_disposition() == Some(Resume)`；`NoIntent` 的 `selected_intent() == None`、
  `continue_disposition() == None`、`basis()` 非空。
- `supervisor_error_has_ten_arms_and_sources`：写一个**无 `_` 通配臂**的函数匹配十臂（编译期钉住
  臂数）；断言 `SupervisorError::Task(TaskError::EmptyPolicyGrant).source().is_some()`、
  `SupervisorError::NoActiveSession { .. }.source().is_none()`、
  `SupervisorError::Checkpoint { error: CheckpointError::new("x"), phase: SuspensionPhase::Throttled }.source().is_some()`。
- `checkpoint_error_keeps_the_message_verbatim`：`CheckpointError::new("磁盘已满")` 的
  `message()` 与 `Display` 都包含原文；不做裁剪。

**结果：** 三条用例名可列出；`SupervisorDecision` 与 `SupervisorError` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test decision
```

**结果：** 编译失败 `E0432`／`E0451`（类型由 Step 3 才建出）。

- [ ] **Step 3: 实现决策类型、错误面与 `CheckpointPort` 类型**

`ContinueDisposition` 两臂照设计 §4.4，派生 `Debug, Clone, Copy, PartialEq, Eq`；`SupervisorDecision`
三臂与四个访问器照 §4.4，派生 `Debug, Clone`。`SupervisorError` 十臂照 §4.9，派生 `Debug`；
`Display` 手写中文，`source()` 对 `Task`／`Decision`／`Resource`／`Memory`／`Checkpoint` 返回内层
错误，其余 `None`；**不派生 `Clone`**（`MemoryError` 不是 `Clone`）。
`CheckpointPort` 的 trait 方法签名照 §4.7；`CheckpointError` 字段私有，派生
`Debug, Clone, PartialEq, Eq`，`new`／`message`、手写 `Display`／`Error`。`lib.rs` 追加 `pub use error::SupervisorError;`、
`pub use resource::{CheckpointError, CheckpointPort};`、
`pub use supervisor::{ContinueDisposition, SupervisorDecision};`。**本 task 不实现
`admit`／`checkpoint`**，也不定义 `BackgroundSupervisor`（Task 6 才建）。

**结果：** `cargo test -p continuum-supervisor --test decision` 全绿；十臂的穷尽匹配编译通过；
`SupervisorError` 的 `source()` 逐臂符合设计 §4.9。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/supervisor.rs crates/continuum-supervisor/src/resource.rs crates/continuum-supervisor/src/error.rs crates/continuum-supervisor/src/lib.rs crates/continuum-supervisor/tests/decision.rs
git commit -m "feat(supervisor): SupervisorDecision/ContinueDisposition 与十臂错误面"
```

---

### Task 6: `BackgroundSupervisor` 的每任务状态表与 `next_tasks`

**Files:**
- Modify: `crates/continuum-supervisor/src/supervisor.rs`
- Modify: `crates/continuum-supervisor/src/lib.rs`（`pub use supervisor::BackgroundSupervisor`）
- Create: `crates/continuum-supervisor/tests/next_tasks.rs`
- Modify: `crates/continuum-supervisor/tests/support/mod.rs`（加 `fn_body`：从 `fn <name>` 起按配对的
  `{}` 截取函数体，忽略注释行）

**Interfaces:**
- Consumes: Task 3 的 `AuthorizedTask`；`continuum_semantics::Intent`；`continuum_arbiter::{SuspensionPhase, SuspensionState}`；
  `std::collections::HashMap`
- Produces:
  `BackgroundSupervisor`（字段私有：`states: HashMap<TaskKey, SuspensionState>`；派生 `Debug, Default`）；
  私有 `TaskKey` 与 `TaskKey::for_task(&AuthorizedTask)`；
  `next_tasks(&self, candidates: &[Intent], tasks: &'a [AuthorizedTask]) -> Vec<&'a AuthorizedTask>`；
  `suspension_phase(&self, task: &AuthorizedTask) -> Option<SuspensionPhase>`

- [ ] **Step 1: 写 `tests/next_tasks.rs`**

- `next_tasks_keeps_input_order_and_current_candidate_status`（J8）：四枚任务 `t1..t4`，构造快照都是
  READY；`candidates = [t1(Ready), t3(Ready), t4(Planning)]`，`t2` 不在候选集；`next_tasks` 返回
  `[t1, t3]`（`tasks` 输入序，不是候选序）；`t4` 证明用候选**当前**状态而非快照；`t2` 证明
  不在候选集即丢弃。
- `next_tasks_keeps_policy_and_maintenance_tasks_unconditionally`（J8）：在上一组中插入一枚
  `policy` 任务与一枚 `maintenance` 任务（都不在候选集），它们仍在结果中，且保持它们在
  `tasks` 里的相对位置。
- `next_tasks_does_not_read_scheduler_inputs_or_sort`：源码面读
  `code_only(include_str!("../src/supervisor.rs"))` 的 `next_tasks` 函数体（用 support 的
  `fn_body` 提取到配对的 `}`），断言不含 `SchedulerInputs`／`decide`／`sort`／`sort_by`／`min`／
  `max`／`cmp`；函数体只有一次 `tasks.iter()` 与 `candidates` 查找。
- `suspension_phase_is_none_before_admit`：`BackgroundSupervisor::default()` 对任意任务返回
  `None`（本 task 只读表；`admit` 在 Task 11）。

**结果：** 四条用例名可列出；`BackgroundSupervisor` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test next_tasks
```

**结果：** 编译失败 `E0432`／`E0599`。

- [ ] **Step 3: 实现状态表、`TaskKey` 与 `next_tasks`**

`BackgroundSupervisor` 字段私有 `states: HashMap<TaskKey, SuspensionState>`，派生 `Debug, Default`
（照设计 §4.4）。`TaskKey` 私有，三臂：
`Intent(String)`／`Policy { issuer, grant_id }`／`Maintenance { schedule_id, occurrence_id }`；
派生 `Debug, Clone, PartialEq, Eq, Hash`。`TaskKey::for_task` 按 `source()` 与 accessor 机械派生：
Intent 两路用 `intent().id.as_str().to_owned()`；Policy 用 `policy_grant()` 的两字段；Maintenance
用 `maintenance_schedule()` 的两字段。**`TaskKey` 的文档写明「一 ref 一任务」契约**：
同一 `PolicyGrantRef`／`MaintenanceOccurrenceRef` 在一个 supervisor 实例里只应有一个任务；
两枚任务共用同一 ref 会共享状态，是调用方契约违反，P7D 不检测、不去重。

`next_tasks` 的算法（设计 §4.4／§5.4）：

1. 按 `tasks` 输入序遍历；
2. 有 `intent()` 的任务：在 `candidates` 里按 `id.as_str()` 找**第一条**同 id 候选；找不到或
   该候选的 `status != Ready` ⇒ 丢弃；否则保留；
3. policy／maintenance：无条件保留；
4. 返回引用 `Vec<&AuthorizedTask>`，不排序、不读 `SchedulerInputs`、不调 `decide`。

`suspension_phase` 只 `self.states.get(&TaskKey::for_task(task)).map(SuspensionState::phase)`。

**结果：** `cargo test -p continuum-supervisor --test next_tasks` 全绿；`states` 已被
`suspension_phase` 读取（无 dead_code warning）。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/supervisor.rs crates/continuum-supervisor/src/lib.rs crates/continuum-supervisor/tests/next_tasks.rs crates/continuum-supervisor/tests/support/mod.rs
git commit -m "feat(supervisor): 每任务状态表与 next_tasks 过滤"
```

---

### Task 7: `decide`（只转调 P7B）与 `J7`／`J9` 的八组照片

**Files:**
- Modify: `crates/continuum-supervisor/src/supervisor.rs`
- Create: `crates/continuum-supervisor/tests/decide.rs`
- Modify: `crates/continuum-supervisor/tests/support/mod.rs`（假 `IntentSelectionPolicy`）

**Interfaces:**
- Consumes: P7B 的 `IntentScheduler::decide`／`SchedulerInputs`／`IntentSelectionPolicy`／
  `IntentDecision`／`IntentDecisionError`；Task 6 的 `BackgroundSupervisor`；Task 3 的 `AuthorizedTask`
- Produces:
  `BackgroundSupervisor::decide(&self, inputs: &SchedulerInputs<'_>, policy: &dyn IntentSelectionPolicy,
  tasks: &[AuthorizedTask], existing_sessions: &[IntentId]) -> Result<SupervisorDecision, SupervisorError>`

- [ ] **Step 1: 写假策略与八组用例**

`support` 里写 `struct FixedPolicy { decision: IntentDecision }` 实现
`IntentSelectionPolicy::choose`（返回 `Ok(self.decision.clone())`）与 `struct FailingPolicy` 返回
`Err(DecisionBasis::new("model unavailable"))`。再写 `fn inputs<'a>(candidates: &'a [Intent]) ->
SchedulerInputs<'a>`，五个信号用空切片／`UserFocus::None`／`BackgroundSuitability::from_projection("")`。

`tests/decide.rs` 的八组（J9）与 J7-c：

1. `decide_maps_ready_to_task`：候选 READY、任务集有同 id ⇒ `SupervisorDecision::Task`，
   `selected_intent()` 命中。
2. `decide_maps_running_verifying_evolving_to_continue`：三个状态逐个；`existing_sessions`
   含该 id ⇒ `ContinueSession { disposition: Continue }`。
3. `decide_requires_an_existing_session_for_continue`：三个状态各在没有会话时 ⇒
   `Err(SupervisorError::NoActiveSession)`。
4. `decide_maps_waiting_and_suspended_to_resume_without_session`：两状态、`existing_sessions = []`
   ⇒ `ContinueSession { disposition: Resume }`；不返回 `NoActiveSession`。
5. `decide_rejects_ready_without_authorized_task`：候选 READY、`tasks = []` ⇒
   `Err(UnauthorizedAdvance { intent })`；**不**临时构造任务、**不**折成 `NoIntent`。
6. `decide_returns_no_intent_and_preserves_basis`：假策略 `NoIntent { basis: "none" }` ⇒
   `SupervisorDecision::NoIntent`、`selected_intent() == None`、`basis().as_str() == "none"`。
7. `decide_fails_closed_on_policy_error`：`FailingPolicy` ⇒ `Err(SupervisorError::Decision(
   IntentDecisionError::PolicyFailed { .. }))`，**不是** `NoIntent`。
8. `decide_does_not_reorder_the_policy_choice`（J7-c）：两枚 READY 候选，假策略返回**第二枚**的
   `Advance`；`decide` 的 `selected_intent()` 是第二枚（按 `as_str()`），不是第一枚。

另写源码面（J7-a／J7-b）：`there_is_no_second_ranking_or_implicit_sort` 读
`code_only` 的 `src/supervisor.rs`，断言不含 `impl IntentSelectionPolicy`／`fn choose`／
`enum IntentDecision`／`struct IntentScheduler`／`struct SchedulerInputs`／`.sort`／`.sort_by`／
`.min(`／`.max(`／`cmp(`／`partial_cmp`／`BinaryHeap`／`BTreeSet`；并断言 `IntentScheduler::decide`
在 `decide` 函数体里**恰出现一次**（用 `fn_body` 提取）。

**结果：** 八组＋一条源码面可列出；`decide` 尚未实现，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test decide
```

**结果：** 编译失败 `E0599`（`decide` 不存在）或 `E0432`（P7B 未落地时）。

- [ ] **Step 3: 实现 `decide`**

1. 调 `IntentScheduler::decide(inputs, policy)`一次；
2. `Err(e)` ⇒ `Err(SupervisorError::Decision(e))`；
3. `NoIntent { .. }` ⇒ `SupervisorDecision::NoIntent { decision }`；
4. `Advance { intent, .. }`：在 `inputs.candidates` 按 `as_str()` 找候选（P7B 保证存在且可推进）；
   - `Ready`：在 `tasks` 按 `task.intent().id.as_str()` 找**第一个**匹配；无 ⇒
     `Err(UnauthorizedAdvance { intent })`；有 ⇒ `Task { task: task.clone(), decision }`；
   - `Waiting`／`Suspended` ⇒ `ContinueSession { intent: intent.clone(), disposition: Resume, decision }`，
     **不查** `existing_sessions`；
   - `Running`／`Verifying`／`Evolving` ⇒ 在 `existing_sessions` 按 `as_str()` 找；无 ⇒
     `Err(NoActiveSession { intent })`；有 ⇒ `ContinueSession { intent, disposition: Continue, decision }`；
   - 其他状态 ⇒ `Err(UnsupportedAdvance { intent })`（防御臂，P7B 的 `is_advanceable` 不会产生）。

不比较候选、不读 `priority`／`deadline`／`blocking`、不迁移状态、不调
`apply_intent_transition`。

**结果：** `cargo test -p continuum-supervisor --test decide` 全绿；八组逐臂可观察；
J7-c 证明 P7D 不重排。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/supervisor.rs crates/continuum-supervisor/tests/decide.rs crates/continuum-supervisor/tests/support/mod.rs
git commit -m "feat(supervisor): decide 转调 P7B 并分派复用/恢复控制值"
```

---

### Task 8: `SessionPlan`、后台摘要与 `plan_manual`

**Files:**
- Modify: `crates/continuum-supervisor/src/session.rs`
- Modify: `crates/continuum-supervisor/src/lib.rs`（`pub use session::{...}`）
- Create: `crates/continuum-supervisor/tests/session_plan.rs`
- Modify: `crates/continuum-supervisor/tests/support/mod.rs`（`seed` 夹具）

**Interfaces:**
- Consumes: Task 6 的 `next_tasks`；Task 3 的 `AuthorizedTask`；`IntentId`；`DecisionBasis`
- Produces:
  `SessionMode::{Manual, Auto}`；`SessionCandidate::{ContinueSession, Task, BackgroundFinding, NewConversation}`；
  `BackgroundNote::new`／`label`／`at_millis`；`BackgroundDigest::empty`／`new`／`completed`／`findings`
  （`Default` 是空摘要）；`SessionPlan` 四字段与六个访问器；`SessionOrchestrator::new`／
  `supervisor`／`supervisor_mut`／`plan_manual`

- [ ] **Step 1: 写 `tests/session_plan.rs`**

- `manual_plan_uses_four_segments_in_fixed_order`（J10-b／J10-c）：`existing_sessions = [id_a, id_b]`，
  `tasks = [t1, t2]`（t1 候选当前 READY，t2 候选当前 PLANNING），`background` 含一条 completed
  与两条 findings；`plan_manual` 的 `candidates()` 依次是
  `ContinueSession(id_a)` → `ContinueSession(id_b)` → `Task(t1)` → `BackgroundFinding(finding1)` →
  `BackgroundFinding(finding2)` → `NewConversation`；**t2 不出现**（当前候选非 READY）；
  `completed` 不作为 candidate。
- `manual_plan_does_not_select`（J10）：`decision() == None`、`selection() == None`、
  `basis() == None`；`mode() == SessionMode::Manual`；`background()` 与输入逐项相等。
- `session_plan_has_four_private_fields_and_six_accessors`（J11／J12 源码面）：读
  `code_only(include_str!("../src/session.rs"))`，在 `pub struct SessionPlan {` 与配对的 `}` 之间
  断言字段恰四行且顺序为 `mode`／`candidates`／`decision`／`background`，且 `background` 行是
  `background: BackgroundDigest,`（digest 只进 `SessionPlan.background`，不进 `SchedulerInputs`
  构造——该判据只落在 `plan_manual` 的函数体上，见下一条 `background_does_not_enter_scheduler_inputs`；
  **本条不禁止整个 `session.rs` 出现 `SchedulerInputs`**：Task 9 的 `plan_auto` 必须消费它）；
  断言 `impl SessionPlan {` 恰一处且 `pub fn` 恰六条（`mode`／
  `candidates`／`decision`／`selection`／`background`／`basis`）；断言不含
  `impl Default for SessionPlan`／`Deserialize`／`pub mode:`。
- `background_note_has_exactly_two_private_fields`（J12 源码面）：读 `code_only` 的
  `src/session.rs`，在 `pub struct BackgroundNote {` 与配对的 `}` 之间取非空字段行，断言恰两行且
  顺序为 `label: String`／`at_millis: i64`，两行都不以 `pub ` 开头。
- `background_note_rejects_empty_label`（J12）：`BackgroundNote::new("", 0)` ⇒
  `Err(SupervisorError::InvalidBackgroundNote)`；`new("发现", -1)` 通过且 `at_millis() == -1`
  （与会话时间同口径，不做非负校验）。
- `background_digest_keeps_input_order_and_empty_is_default`（J12）：`BackgroundDigest::empty()` 的
  两数组为空且等于 `BackgroundDigest::default()`；`new(completed, findings)` 的两个访问器逐项保持
  输入序；不排序。
- `background_does_not_enter_scheduler_inputs`（J12 源码面）：`code_only` 的
  `src/supervisor.rs` 不含 `BackgroundDigest`／`BackgroundNote`；`code_only` 的 `src/` 全部文件不含
  `CompletionVerdict`／`continuum_verify`，且按标识符边界判独立 `Evidence` 零命中（`TaskEvidence`／
  `evidence` 字段不算，见 Task 14 同一口径）；`plan_manual` 的函数体不含 `SchedulerInputs`／
  `decide`，且 `next_tasks` 恰出现一次（用 `fn_body`）。

**结果：** 七条用例名可列出；`SessionPlan`／`SessionOrchestrator` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test session_plan
```

**结果：** 编译失败 `E0432`／`E0599`。

- [ ] **Step 3: 实现 `session.rs` 与 `plan_manual`**

`SessionMode`／`SessionCandidate`／`BackgroundNote`／`BackgroundDigest`／`SessionPlan`／
`SessionOrchestrator` 照设计 §4.5／§4.6：`SessionMode` 派生 `Debug, Clone, Copy, PartialEq, Eq`；
`SessionCandidate` 派生 `Debug, Clone`；`BackgroundNote` 字段私有、派生
`Debug, Clone, PartialEq, Eq`，`new` 只拒绝空 `label`（`is_empty()`，不 trim）；
`BackgroundDigest` 派生 `Debug, Clone, PartialEq, Eq, Default`，`empty()` 返回 `Self::default()`。
`SessionPlan` 字段私有；派生 `Debug, Clone`；**不实现 `Default`／`Deserialize`**；六个访问器照设计
§4.5，`selection()` 只投影 Task／ContinueSession、`basis()` 委托 `decision`。`SessionOrchestrator`
派生 `Debug, Default`，持一个 `BackgroundSupervisor`；`new`／`supervisor`／`supervisor_mut` 照 §4.6。

`plan_manual` 严格按设计 §5.4：先 `let visible = self.supervisor.next_tasks(candidates, tasks);`，
再按 existing_sessions（输入序，`ContinueSession { intent: id.clone() }`）→ visible（输入序，
`Task(task.clone())`）→ `background.findings()`（输入序，`BackgroundFinding(note.clone())`）→
`NewConversation` 构造 `candidates`；`decision = None`；`background` 原样放入。
**不调 `decide`、不读 `SchedulerInputs`、不读 `completed` 作 candidate。**

**结果：** `cargo test -p continuum-supervisor --test session_plan` 全绿；四段顺序与过滤照片成立。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/session.rs crates/continuum-supervisor/src/lib.rs crates/continuum-supervisor/tests/session_plan.rs crates/continuum-supervisor/tests/support/mod.rs
git commit -m "feat(supervisor): SessionPlan 四字段、后台摘要与 plan_manual"
```

---

### Task 9: `plan_auto`、`NoIntent` 保留与 `Resume` 的候选关系

**Files:**
- Modify: `crates/continuum-supervisor/src/session.rs`
- Create: `crates/continuum-supervisor/tests/plan_auto.rs`

**Interfaces:**
- Consumes: Task 7 的 `decide`；Task 8 的 `SessionPlan`／`plan_manual` 段构造；P7B 的
  `SchedulerInputs`／`IntentSelectionPolicy`
- Produces: `SessionOrchestrator::plan_auto(&self, inputs, policy, tasks, existing_sessions, background)
  -> Result<SessionPlan, SupervisorError>`

- [ ] **Step 1: 写 `tests/plan_auto.rs`**

- `auto_plan_uses_the_same_segments_and_keeps_the_decision`（J10-a）：假策略返回
  `Advance(READY)` 且任务集有同 id ⇒ `mode() == Auto`、`decision()` 是 `Task`、
  `selection()` 是 `Some(Task)`、`candidates()` 段序仍是 existing → visible → findings →
  `NewConversation`。
- `no_intent_keeps_existing_sessions_and_background`（J10-d／Review Focus 3）：`existing_sessions = [id]`；
  `background` 含一条 completed＋一条 finding；假策略返回 `NoIntent { basis: "none" }` ⇒
  `decision()` 是 `Some(NoIntent)`、`selection() == None`、`basis().as_str() == "none"`、
  `candidates()` 仍含 `ContinueSession { id }` 与 `BackgroundFinding`、`background()` 与输入逐项相等。
- `resume_selection_need_not_appear_in_candidates`（Review Focus 2）：候选是一枚 `Waiting` Intent；
  `tasks = []`；`existing_sessions = []`；假策略 `Advance(Waiting)` ⇒ `decision()` 是
  `ContinueSession { disposition: Resume }`、`selected_intent()` 命中该 id、`selection()` 是 `Some`；
  `candidates()` **只有 `NewConversation`**，没有该 Intent 的 `Task` 或 `ContinueSession`。
- `auto_plan_includes_new_conversation_in_both_modes`（Minor）：同一组输入分别走 `plan_manual` 与
  `plan_auto`，两者的 `candidates().last()` 都是 `NewConversation`；Auto 的候选段序与 Manual 相同。
- `background_does_not_change_the_policy_choice`（J12）：同一候选／假策略，两次 `plan_auto` 仅
  `background` 不同（一条 finding vs 空）；两次 `decision()` 的 `selected_intent()`／`basis()` 相同，
  仅 `background()` 不同。
- `auto_plan_fails_closed_without_a_plan`：假策略 `Err` ⇒ `plan_auto` 返回 `Err(SupervisorError::Decision)`；
  **不返回**一个空 `SessionPlan`。

**结果：** 六条用例名可列出；`plan_auto` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test plan_auto
```

**结果：** 编译失败 `E0599`（`plan_auto` 不存在）。

- [ ] **Step 3: 实现 `plan_auto`**

先 `let decision = self.supervisor.decide(inputs, policy, tasks, existing_sessions)?;`；
再用 `self.supervisor.next_tasks(inputs.candidates, tasks)` 与**与 Manual 相同的四段构造**
（existing_sessions → visible → `background.findings()` → `NewConversation`）；`decision =
Some(decision)` 原样存放（Task／ContinueSession／NoIntent 三臂都不翻译）；`background` 原样放入。
**不补** `Resume` 选中项的 candidate；**不因 finding 改变 decision**；**不恢复会话**。

**结果：** `cargo test -p continuum-supervisor --test plan_auto` 全绿；J10-a／J10-d、Review Focus 2／3
与 NewConversation 的照片成立。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/session.rs crates/continuum-supervisor/tests/plan_auto.rs
git commit -m "feat(supervisor): plan_auto 复用 P7B 选择并保留 NoIntent"
```

---

### Task 10: `SessionPlan` 的编译期形状（trybuild + serde_json 负例）

**Files:**
- Create: `crates/continuum-supervisor/tests/type_level.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/session_plan_literal_is_private.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/session_plan_has_no_default.rs`
- Create: `crates/continuum-supervisor/tests/compile_fail/session_plan_is_not_deserialize.rs`
- Create: 三份 `.stderr`（Step 2 实跑取）
- Create: `crates/continuum-supervisor/tests/ui_pass/session_plan_is_reachable.rs`

**Interfaces:**
- Consumes: Task 8／9 的 `SessionPlan`／`SessionMode`／`BackgroundDigest`（**使用侧**）
- Produces: J11 的编译期照片；无生产类型

- [ ] **Step 1: 写三份 `compile_fail` 与 `ui_pass`**

- `session_plan_literal_is_private.rs`：
  ```rust
  use continuum_supervisor::{BackgroundDigest, SessionMode, SessionPlan};

  fn main() {
      let _ = SessionPlan {
          mode: SessionMode::Manual,
          candidates: vec![],
          decision: None,
          background: BackgroundDigest::default(),
      };
  }
  ```
  样例故意给全四字段；字段私有 ⇒ `E0451`。设计 J11 的另一半「缺字段 ⇒ `E0063`」在字段私有下
  从 crate 外不可达：外部构造（无论缺不缺字段）都先命中 `E0451`。本计划**以 `E0451` 代替
  `E0063`**，不静默少一条照片，也不为拍 E0063 在 crate 内加 `#[cfg(test)]` 构造点。
- `session_plan_has_no_default.rs`：`let _ = SessionPlan::default();` ⇒ `E0599`。
- `session_plan_is_not_deserialize.rs`：
  ```rust
  fn main() {
      let _: continuum_supervisor::SessionPlan = serde_json::from_str("{}").expect("…");
  }
  ```
  `serde_json` 是 dev-dep；没有 `Deserialize` ⇒ `E0277`。
- `ui_pass/session_plan_is_reachable.rs`：函数接收 `&SessionPlan` 并调用六个访问器；不构造值；
  加 `fn main() {}`。
- `tests/type_level.rs`：trybuild 驱动三份 `compile_fail` ＋ 一份 `pass`。

**结果：** 四份样例与驱动存在；三份 `.stderr` 未写。

- [ ] **Step 2: 运行，取 `.stderr`**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test type_level
```

**结果：** 运行期红（`.stderr` 缺失或与样例不符）。把实跑输出逐份写入同名 `.stderr`，再跑一次
确认绿；三份错误码可不同（`E0451`／`E0599`／`E0277`），不许套用同一个值。

- [ ] **Step 3: 不加生产代码**

本 task **只交付样例与 `.stderr`**。**不加** `Default`／`Deserialize`／`pub` 字段／第二个构造点；
`SessionPlan` 仍只由 `plan_manual`／`plan_auto` 构造。`serde_json` 只是 `[dev-dependencies]`，
不进 `ALLOWED`。

**结果：** `cargo test -p continuum-supervisor --test type_level` 全绿；`src/session.rs` 一字未动。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/tests/type_level.rs crates/continuum-supervisor/tests/compile_fail crates/continuum-supervisor/tests/ui_pass
git commit -m "test(supervisor): SessionPlan 唯一构造点的 trybuild 与反序列化负例"
```

---

### Task 11: `admit`、`release` 与「一 ref 一任务」的负例

**Files:**
- Modify: `crates/continuum-supervisor/src/supervisor.rs`
- Modify: `crates/continuum-supervisor/src/resource.rs`（`declaration_for` 私有映射）
- Create: `crates/continuum-supervisor/tests/resource.rs`

**Interfaces:**
- Consumes: P7C 的 `ResourceArbiter::direct`／`ResourceDecision`／`ResourceAction`／
  `SuspensionState`／`WorkloadDeclaration`；Task 6 的 `states`／`TaskKey`
- Produces:
  `BackgroundSupervisor::admit(&mut self, task, arbiter, now_millis) -> Result<ResourceDecision, SupervisorError>`；
  `release(&mut self, task) -> bool`；私有 `declaration_for(&AuthorizedTask) -> WorkloadDeclaration`

- [ ] **Step 1: 写 `tests/resource.rs`**

- `admit_maps_all_four_task_sources_to_declarations`（J13-a）：在 `High` 压力下
  （`ResourceObservation { cpu: Some(80), ..Default::default() }`）：`maintenance` 任务 ⇒
  `action == Some(Suspend)`、相位 `Checkpointing`；`policy`／`authorized_intent`／`product_drive`
  三枚任务各自 ⇒ `action == Some(Throttle)`、相位 `Throttled`。在 `Low` 下背景任务 ⇒
  `action == None`、相位 `Running`；在 `Critical` 下背景任务 ⇒ `Suspend`、`Checkpointing`。
- `admit_returns_none_as_none_and_keeps_phase`（J13-a）：`Low` 下同一任务连续两次 `admit`，第二次
  `action == None`、相位仍 `Running`；**不把 `None` 当 `Run`**。
- `admit_reuses_the_same_state_for_the_same_task`（J13-d）：`Critical` 下首次 `admit` ⇒
  `Checkpointing`；第二次 `admit` ⇒ `action == None`、相位仍 `Checkpointing`（同一枚状态，未重置）。
- `release_drops_state_and_a_new_admit_starts_fresh`（J13-d）：`release(task) == true`、
  `suspension_phase(task) == None`；再次 `release` 返回 `false`；再次 `admit`（`Critical`）⇒
  `Suspend`／`Checkpointing`（新状态从 `Running` 出发）。
- `same_policy_reference_shares_one_state_and_is_a_contract_violation`（Review Focus 1）：两个
  `AuthorizedTask::policy` 用**同一** `PolicyGrantRef` 构造；第一枚 `admit` 在 `Critical` 下进入
  `Checkpointing`；第二枚在**同一 `now_millis`（未到截止）** `admit` 返回 `action == None`、
  相位仍 `Checkpointing`。测试名与文档写明这是调用方契约违反，不是 P7D 的合并行为。
- `distinct_policy_references_get_independent_states`：两个不同 `grant_id` 的任务各自在 `Critical`
  下 `admit` 都得到 `Suspend`／`Checkpointing`；两枚相位独立。

**结果：** 六条用例名可列出；`admit`／`release` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test resource
```

**结果：** 编译失败 `E0599`（`admit`／`release` 不存在）或 `E0432`（P7C 未落地时）。

- [ ] **Step 3: 实现 `admit`、`release` 与 `declaration_for`**

`admit`：`let key = TaskKey::for_task(task);` →
`self.states.entry(key).or_insert_with(SuspensionState::new)` → `declaration_for(task)` →
`arbiter.direct(declaration, &mut state, now_millis)`；`Ok(decision)` 原样返回，`Err(e)` ⇒
`Err(SupervisorError::Resource(e))`。**不读 `decision.summary()` 做决策、不把 `action == None`
改写成 `Run`。** `declaration_for` 是私有函数，三个布尔写死为
`user_process: false`／`foreground: false`／`maintenance: matches!(task.source(),
TaskSource::MaintenanceSchedule)`（设计 §4.7）。
`release`：`self.states.remove(&TaskKey::for_task(task)).is_some()`；不推断终止、不做 TTL。

**结果：** `cargo test -p continuum-supervisor --test resource` 全绿；四来源映射、`None` 原样、
状态复用／释放与一 ref 一任务负例都可观察。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/supervisor.rs crates/continuum-supervisor/src/resource.rs crates/continuum-supervisor/tests/resource.rs
git commit -m "feat(supervisor): admit/release 与每任务挂起状态复用"
```

---

### Task 12: checkpoint 三出口、定时器与相位回退

**Files:**
- Modify: `crates/continuum-supervisor/src/supervisor.rs`
- Create: `crates/continuum-supervisor/tests/checkpoint.rs`
- Modify: `crates/continuum-supervisor/tests/support/mod.rs`（假 `CheckpointPort`）

**Interfaces:**
- Consumes: Task 11 的 `states`／`admit`；Task 5 的 `CheckpointPort`／`CheckpointError`；
  P7C 的 `SuspensionPhase`／`SuspensionError`
- Produces:
  `checkpoint(&mut self, task, port, now_millis) -> Result<SuspensionPhase, SupervisorError>`；
  `checkpoint_timed_out(&mut self, task, now_millis) -> Result<(), SupervisorError>`；
  `checkpoint_deadline(&self, task) -> Option<u64>`

- [ ] **Step 1: 写假端口与八条用例**

`support` 里的假端口：`struct CountingPort { result: Result<(), CheckpointError>, calls: Cell<usize> }`，
`impl CheckpointPort` 每次 `calls.set(calls.get()+1)` 后返回 `result.clone()`。

`tests/checkpoint.rs`：

1. `checkpoint_success_before_deadline_enters_suspended`：`Critical` 下 `admit`（默认
   `checkpoint_timeout_millis = 60_000`）⇒ `checkpoint_deadline == Some(now + 60_000)`；
   `checkpoint(task, ok_port, now + 1_000)` ⇒ `Ok(Suspended)`、相位 `Suspended`、
   `checkpoint_deadline == None`、端口计数 1。
2. `late_success_returns_timed_out_and_returns_to_throttled`：`admit` 后在 `now + 60_000` 调
   `checkpoint(ok_port)` ⇒ `Err(SupervisorError::Resource(SuspensionError::CheckpointTimedOut))`、
   相位 `Throttled`、`checkpoint_deadline == None`；**不是 `Suspended`**。
3. `port_failure_returns_checkpoint_error_and_throttles`：端口返回
   `Err(CheckpointError::new("磁盘已满"))` ⇒ `Err(SupervisorError::Checkpoint { error, phase:
   SuspensionPhase::Throttled })`；`error.message()` 是原文；相位 `Throttled`、
   `checkpoint_deadline == None`。
4. `timer_hook_returns_timed_out_and_throttles`：`admit` 后调
   `checkpoint_timed_out(task, now + 60_000)` ⇒ `Err(SupervisorError::Resource(
   SuspensionError::CheckpointTimedOut))`、相位 `Throttled`、`checkpoint_deadline == None`、
   端口计数 0。
5. `checkpoint_without_state_is_rejected`：未 `admit` 就调 `checkpoint` ⇒
   `Err(SupervisorError::NoSuspensionState { source })`；`release` 之后再调同样。
6. `checkpoint_outside_checkpointing_does_not_call_port`：`admit` 在 `Low` 下得到 `Running`；
   调 `checkpoint` ⇒ `Err(Resource(NotCheckpointing { phase: Running }))`，端口计数 0；
   `Suspended` 之后再调 ⇒ `NotCheckpointing { phase: Suspended }`，端口计数仍不增。
7. `checkpoint_failure_can_be_retried_and_never_stays_checkpointing`：端口失败后相位 `Throttled`；
   在 `Critical` 下再次 `admit`（满足 P7C 的 retry cooldown 时传足够大的 `now_millis`）⇒
   重新 `Checkpointing`；`checkpoint` 成功 ⇒ `Suspended`。**任何出口都不停在 `Checkpointing`。**
8. `checkpoint_timed_out_without_state_or_wrong_phase_is_rejected`：未 `admit` 就调
   `checkpoint_timed_out(task, now)` ⇒ `Err(SupervisorError::NoSuspensionState { source })`；
   成功 checkpoint 进入 `Suspended` 后再调 ⇒
   `Err(SupervisorError::Resource(SuspensionError::NotCheckpointing { phase: Suspended }))`，
   相位仍 `Suspended`。

**结果：** 八条用例名可列出；`checkpoint`／`checkpoint_deadline`／`checkpoint_timed_out` 尚未定义，
Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test checkpoint
```

**结果：** 编译失败 `E0599`（三个方法不存在）。

- [ ] **Step 3: 实现三个控制面方法**

`checkpoint`：
1. `let key = TaskKey::for_task(task); let Some(state) = self.states.get_mut(&key) else { return
   Err(SupervisorError::NoSuspensionState { source: task.source() }); };`
2. `if state.phase() != SuspensionPhase::Checkpointing { return
   Err(SupervisorError::Resource(SuspensionError::NotCheckpointing { phase: state.phase() })); }`
   ——**端口不被调用**；
3. `match port.checkpoint(task) { Ok(()) => match state.checkpoint_succeeded(now_millis) {
   Ok(phase) => Ok(phase), Err(e) => Err(SupervisorError::Resource(e)) },
   Err(port_error) => match state.checkpoint_failed(now_millis) {
   SuspensionError::CheckpointFailed => Err(SupervisorError::Checkpoint { error: port_error,
   phase: SuspensionPhase::Throttled }), other => Err(SupervisorError::Resource(other)) } }`。

`checkpoint_timed_out`：取 `states.get_mut`，无 ⇒ `NoSuspensionState`；`let e =
state.checkpoint_timed_out(now_millis); Err(SupervisorError::Resource(e))`——**没有 `Ok` 路径**。

`checkpoint_deadline`：`self.states.get(&TaskKey::for_task(task)).and_then(
SuspensionState::checkpoint_deadline_millis)`；无状态或不在 `Checkpointing` ⇒ `None`。

**不得**保存 checkpoint 内容、不得重试、不得建线程、不得把晚到结果写回 `Suspended`；失败与超时都
回 `Throttled`（设计 §4.7、§5.5）。

**结果：** `cargo test -p continuum-supervisor --test checkpoint` 全绿；三出口＋定时器＋错相位
八条都可观察。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/supervisor.rs crates/continuum-supervisor/tests/checkpoint.rs crates/continuum-supervisor/tests/support/mod.rs
git commit -m "feat(supervisor): checkpoint 三出口、超时与相位回退"
```

---

### Task 13: `SynthesisIo` 端口与 §171 三个转发

**Files:**
- Modify: `crates/continuum-supervisor/src/trigger.rs`
- Modify: `crates/continuum-supervisor/src/lib.rs`（`pub use trigger::{SynthesisIo, SynthesisTriggerPoint}`）
- Create: `crates/continuum-supervisor/tests/trigger.rs`
- Modify: `crates/continuum-supervisor/tests/support/mod.rs`（假 `SynthesisIo`）

**Interfaces:**
- Consumes: `continuum_memory::{MemoryError, SynthesisJobId, SynthesisProposal, SynthesisReport,
  SynthesisRequest}`；Task 5 的 `SupervisorError`
- Produces:
  `SynthesisIo` trait（`begin_synthesis`／`apply_synthesis`／`fail_synthesis`，签名照设计 §4.8）；
  `SynthesisTriggerPoint`（`Default, Clone, Copy`）与三个同名转发

- [ ] **Step 1: 写假 IO 与三条用例**

`support` 里的假 IO 用 `Cell<usize>` 三个计数；`begin`／`fail` 返回 `Ok(())`，`apply` 返回一个
`SynthesisReport { created: vec![], merged: vec![], superseded: vec![], summary_written: true,
staled_summaries: 0 }`。另写 `FailingIo` 返回 `Err(MemoryError::UnknownJob)`。

`tests/trigger.rs`：

- `begin_apply_fail_forward_exactly_once`（J14）：三个方法各调一次，三个计数都恰为 1；
  `apply` 原样返回假 IO 的 `SynthesisReport`；`SynthesisTriggerPoint` 是 `Copy`，复制后调用不改变计数。
- `memory_errors_are_wrapped_not_swallowed`：`FailingIo` 的三个方法各 ⇒
  `Err(SupervisorError::Memory(MemoryError::UnknownJob))`；**不是** `Ok`、不是 `NoIntent`。
- `trigger_point_has_no_clock_retry_or_threshold`（J14 源码面）：读 `code_only` 后的 **`src/` 全部
  文件**（`lib.rs` 与六个模块），断言 `Duration`／`Instant`／`SystemTime::now`／`thread::sleep`／
  `retry`／`interval`／`idle`／`threshold` 零命中；再读 `trigger.rs`，断言不含 `impl SynthesisIo`
  （实现不在本 crate）、`enum SynthesisTrigger`（不重定义）；三个函数体各恰一次对 `io` 的调用。

**结果：** 三条用例名可列出；`SynthesisIo`／`SynthesisTriggerPoint` 尚未定义，Step 2 是编译失败。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test trigger
```

**结果：** 编译失败 `E0432`。

- [ ] **Step 3: 实现端口与三个转发**

`SynthesisIo` 的三个方法照设计 §4.8；`SynthesisTriggerPoint` 派生
`Debug, Default, Clone, Copy`，不持状态、不取时钟；三个方法只做「转发 → `MemoryError` 包成
`SupervisorError::Memory` → 原样返回」。
**不构造 `SynthesisRequest`、不生成 `SynthesisProposal`／`SynthesisJobId`、不决定何时调用、
不重试、不取时钟**（`occurred_at` 由调用方给）。`trigger.rs` 的注释可说明「实现方持 `Tx`」，
但**不要**在代码里命名 `Tx<'_>` 或调用 P6C 的自由函数。

**结果：** `cargo test -p continuum-supervisor --test trigger` 全绿；三个计数各 1；
`MemoryError` 透传。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-supervisor/src/trigger.rs crates/continuum-supervisor/src/lib.rs crates/continuum-supervisor/tests/trigger.rs crates/continuum-supervisor/tests/support/mod.rs
git commit -m "feat(supervisor): SynthesisIo 端口与 §171 三个触发转发"
```

---

### Task 14: 源码面 J12／J13-d／J16–J21、全量门与变异复核

**Files:**
- Create: `crates/continuum-supervisor/tests/source_surface.rs`
- Modify: `crates/continuum-supervisor/tests/source_surface.rs`（仅在复核发现缺口时；本 task 不新增生产类型）
- **不修改** `src/`（除非前面 task 的源码漂了；那要回到拥有该代码的 task 修）

**Interfaces:**
- Consumes: 本 crate 的 `src/` 与 `Cargo.toml` 文本；`continuum-runtime` 的三个零表文件
- Produces: J12／J13-d／J16／J17／J18／J19／J20／J21 的源码面断言；最终树的全量门

- [ ] **Step 1: 写 `tests/source_surface.rs` 的十一条用例**

用 `support::code_only` 拼 `src/` 六个文件（`task.rs`／`supervisor.rs`／`session.rs`／
`resource.rs`／`trigger.rs`／`error.rs`）与 `lib.rs`；`Cargo.toml` 用 `include_str!("../Cargo.toml")`。

- `lib_rs_defines_no_types`：`code_only(src/lib.rs)` 不含 `pub struct`／`pub enum`／`pub trait`／
  `impl `（只做 `pub mod` 与 `pub use`）。
- `allowed_row_has_exactly_five_entries`（J16）：读
  `code_only(include_str!("../../../crates/continuum-runtime/tests/dependency_direction.rs"))`，先
  取 `const ALLOWED: &[(&str, &[&str])] = &[` 到其后 `\n];` 之间的表体；在表体内扫描引号串，找到
  值恰为 `continuum-supervisor` 的 subject，再从该引号串起始位置**向左**找最近的 `(`，从该 `(`
  向右做括号深度匹配（跨行、允许任意空白）找到配对的 `)`；只在这个括号范围内取引号串，**跳过
  subject 自身的引号串**，断言恰五项且按字母序为
  `continuum-arbiter`／`continuum-canonical`／`continuum-intent-scheduler`／`continuum-memory`／
  `continuum-semantics`；另用同一套「subject 引号串 → 左侧 `(` → 配对 `)`」定位取
  `continuum-runtime` 的元组范围，断言其中不含 `continuum-supervisor`（只看该元组自己的范围，不做
  全文查找）。**不要用 `("continuum-supervisor",` 这种把 `(` 与 subject 绑在同一行的字面定位**：
  Task 1 的插入是多行 tuple，`(` 与 subject 各占一行、`&[` 与 `]` 也各占一行；解析器必须允许
  `(` 与 subject 之间跨行／有空白。不跳过 subject 会数成六项，这是本条解析口径的写死点。
- `zero_persistence_and_no_intent_table`（J17）：`Cargo.toml` 不含 `continuum-persist`；
  `code_only` 的 `src/` 不含 `Migration`／`Tx`／`INSERT`／`UPDATE`／`load_intent`／
  `list_children`／`save_intent`／`apply_intent_transition`／`Event`／`EventType`；
  `crates/continuum-runtime/src/main.rs` 与 `tests/migrations.rs`／`tests/startup.rs` 不含
  `continuum_supervisor`／`continuum-supervisor`。
- `no_user_behavior_or_model_paths`（J18）：`code_only(src)` 不含 `std::process`／`Command::new`／
  `window`／`keyboard`／`screen`／`open files`／`process_name`／`pid`／`continuum_provider`／
  `ModelProvider`／`InvokeRequest`／`continuum_runtime`；`Cargo.toml` 不含这些依赖。
- `no_second_intent_store_or_product_record`（J19／J22）：`code_only(src)` 不含 `load_intent`／
  `list_children`／`save_intent`／`apply_intent_transition`／`continuum_product`／`Product::`／
  `Milestone`／`load_product`；`Cargo.toml` 不含 `continuum-product`。**`Product` 刻意收窄成
  `Product::`**：`ProductIdRef` 与 `IntentKind::Product` 是合法词汇，裸 `Product` 会误伤；设计 J22
  的裸 `Product` 按这条照片读。
- `no_shippable_or_readiness_types`（J20）：`code_only(src)` 不含 `Shippable`／`ProductReadiness`／
  `CompletionVerdict`；文档面的「§7.4 没有 P7D」由本计划 Header 与设计 §3.6 承担，不另造源码断言。
- `no_second_resource_scheduler`（J21）：`code_only(src)` 不含 `rank(`／`place(`／`RoutingRequest`／
  `PlacementRequest`／`parallelism`／`ResourceDirective`。
- `no_priority_tier_or_assembly_surface`（J13-b 的终态复核）：`code_only(src)` 与
  `Cargo.toml` 不含 `PriorityTier`／`AssemblyView`／`ResourceObservation`／`assembly`；
  `code_only(src)` 不含 `impl CheckpointPort`／`enum ResourceAction`／`struct ResourceDecision`／
  `struct SuspensionState`／`struct ResourceArbiter`。
- `background_is_not_evidence`（J12 源码面）：`code_only` 的 `src/` 全部文件按**标识符边界**判
  `Evidence`——分词后等于 `Evidence` 的标识符零命中；`TaskEvidence`／`evidence` 字段不算（它们是
  §4.3 的授权证据，不是第二枚 `Evidence`）；同时 `CompletionVerdict`／`continuum_verify` 子串零命中，
  `Cargo.toml` 也不含 `continuum-verify`。
- `suspension_state_is_owned_not_a_parameter`（J13-d 源码面）：读 `code_only` 的
  `src/supervisor.rs`，断言含 `states: HashMap<TaskKey, SuspensionState>`；提取该文件中**每个**
  `pub fn` 的签名（到 `{` 为止），断言任何参数表都不含 `SuspensionState`／`&mut SuspensionState`
  （状态表由 supervisor 持有，不交给调用方；重点核对 `admit`／`checkpoint`／`checkpoint_timed_out`／
  `release`）；提取 `checkpoint_timed_out` 的函数体，断言 `.checkpoint_timed_out(` 恰一次（调 P7C 的
  同名方法）。
- `src_uses_only_full_line_comments`：读**未过** `code_only` 的 `src/` 全部文件，逐行断言：若某行
  含 `//`，则该行 `trim_start()` 必须以 `//` 开头；且全文不含 `/*`／`*/`。字符串字面量同样不使用
  `//`／`/*`（避免把 URL 之类误当注释）。这是 `code_only` 只处理整行注释的边界照片；违反时改源码的
  注释形式，不扩 `code_only`。

**结果：** 十一条用例名可列出；本 task 是源码面断言，预期**首次即绿**（若红，逐处读命中行：
是源码真的违反了设计，还是注释里出现了被禁字面量；后者改注释，不改断言）。

- [ ] **Step 2: 运行源码面**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-supervisor --test source_surface
```

**结果：** 十一条全绿；每条零命中都限定在 `code_only` 的语料上；若某条红，先区分「实现漂移」
与「注释漂移」。

- [ ] **Step 3: 跑最终树的全量门与零表门**

```bash
TMPDIR="$PWD/.tmp" timeout 1800 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test migrations
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test startup
```

**结果：** 五个命令全绿、0 warning；这是**唯一的最终树门**（Task 2 只做本地树）。
`p7_union_adds_only_its_own_rows` 的断言 5 在最终树是 `{continuum-product}`（⊆ P7 名字；P7A 落地
前是空集），断言 8 在 P7A／B／C／D 四块齐全时成立；`migrations`／`startup` 的计数没有本 crate 的
迁移。**若 P7A／B／C 尚未落地**：`cargo test --workspace` 可在本树跑，但 J15 的「最终树」只在四块
齐全时成立；在交付回报里写明「部分树绿／最终树待四块合并」，不要把部分树写成最终树。

- [ ] **Step 4: 变异复核（确认断言会红）**

依次做三个最小变异，每次改完跑对应测试后**用 `git checkout -- <文件>` 还原**：

1. 在 `src/session.rs` 的 `plan_manual` 里把
   `let visible = self.supervisor.next_tasks(candidates, tasks);` 换成
   `let visible: Vec<&AuthorizedTask> = tasks.iter().collect();`（编译安全；不新增未解析路径）
   ⇒ `session_plan` 的 J10-b 源码面（`next_tasks` 恰一次）与 Manual 过滤用例应红；还原。
2. 在 `src/supervisor.rs` 的 `next_tasks` 里把候选当前状态改成 `task.intent().status`（快照）
   ⇒ `next_tasks` 的 J8 应红；还原。
3. 在 `src/supervisor.rs` 的 `checkpoint` 失败臂把 `CheckpointFailed` 改成 `Ok(Suspended)`
   （或直接返回 `Ok(Suspended)`）⇒ `checkpoint` 的端口失败用例应红；还原。

**结果：** 三个变异各自让对应断言红；还原后 `cargo test -p continuum-supervisor` 全绿。
**不许保留变异**；提交前 `git diff` 为空（除计划文件外）。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-supervisor/tests/source_surface.rs
git commit -m "test(supervisor): 源码面 J12/J13-d/J16-J21 与全量门复核"
```

---

## 自审

### 1. Spec coverage（逐节对表）

| 设计节 | 交付物／判据 | 落在 |
|---|---|---|
| §4.2 `TaskSource` 四值、`as_str` | J1-a／J1-b | Task 1 |
| §4.3 `AuthorizedTask`、四个构造器、`PolicyGrantRef`／`MaintenanceOccurrenceRef` | J2／J3／J4／J5／J6／J22 | Task 1／3／4 |
| §4.4 `SupervisorDecision`／`ContinueDisposition`／`next_tasks`／`decide` | J7／J8／J9 | Task 5／6／7 |
| §4.5 `SessionPlan` 四字段、`SessionCandidate`、`BackgroundNote`／`BackgroundDigest` | J10／J11／J12 | Task 8／9／10 |
| §4.6 `SessionOrchestrator`、`plan_manual`／`plan_auto` | J10-a～d | Task 8／9 |
| §4.7 `admit`／`checkpoint`／`checkpoint_timed_out`／`checkpoint_deadline`／`suspension_phase`／`release`／`CheckpointPort` | J13-a～d | Task 5／11／12（J13-d 的源码面在 Task 14） |
| §4.8 `SynthesisIo`／`SynthesisTriggerPoint` | J14 | Task 13 |
| §4.9 `SupervisorError` 十臂 | 错误面 | Task 5 |
| §4.10 可见性、唯一构造点、不变量 | J2／J11／J12 | Task 4／8／10 |
| §5.1／§5.2 四类来源与校验顺序 | J3／J4／J5／J6 | Task 3 |
| §5.3 `decide` 复用 P7B | J7／J9 | Task 7 |
| §5.4 `SessionPlan` 组成、NoIntent 保留 | J10／J12 | Task 8／9 |
| §5.5 `admit`／checkpoint 三出口／状态表 | J13 | Task 11／12 |
| §5.6 §171 边界 | J14 | Task 13 |
| §5.7／§9.2 P6D rebaseline | J15 | Task 2 |
| §6 跨块接缝 | J7／J13／J22 | Task 3／7／11／12 |
| §7.2 J1–J22 照片 | 全部 | Task 1–14（见下） |
| §9.1 零表零迁移 | J17 | Task 14 |
| §10 遗留 | 见「遗留/待协调」 | 本文件末节 |

**J1–J22 的归位**：J1 Task 1；J2 Task 4；J3 Task 3；J4 Task 3；J5 Task 3；J6 Task 1（引用）
＋Task 4（类型不匹配）；J7 Task 7；J8 Task 6；J9 Task 7；J10 Task 8（b／c）＋Task 9（a／d）；
J11 Task 8（字段面）＋Task 10（编译期）；J12 Task 8（运行期＋字段面）＋Task 14（源码面）；J13
Task 11（a／d 运行期）＋Task 12（c）＋Task 14（d 源码面）；J14 Task 13；J15 Task 2（本地树）＋
Task 14（最终树单一出口）；J16 Task 1＋Task 14；J17 Task 14；J18 Task 14；J19 Task 14；J20 Task 14；
J21 Task 14；J22 Task 3＋Task 14。

### 2. Step scan

每个 task 的步骤都是「写测试／样例 → 运行确认失败 → 实现（或明确无生产代码）→ 运行通过 →
提交」。没有「处理边界」「适当校验」「补测试」这类不决定动作的步骤；每个 Step 都给了可核对结果
（用例名、`E0432`／`E0599`／`E0277`、相位值、计数、grep 零命中、命令绿）。Task 14 的「首次即绿」
按 P6A 先例标注，不伪装成红。

### 3. Type consistency

- `TaskSource` 四值与 `as_str` 字面量在 Task 1 定义，Task 3／6／11 只引用，不重复定义。
- `AuthorizedTask` 的四个构造器名、五个访问器名（`source`／`intent`／`contract`／`policy_grant`／
  `maintenance_schedule`）在 Task 3 的 Interfaces 冻结；Task 4／6／7／11／12 只引用。
- `SupervisorDecision` 的 `Task`／`ContinueSession`／`NoIntent` 与 `ContinueDisposition` 在 Task 5
  定义；Task 7／9 使用同一名字，没有 `SessionSelection`。
- `SessionPlan` 的四字段与六个访问器在 Task 8 定义；Task 9／10 只使用，不新增字段。
- `TaskKey::for_task`、`states`、`admit`／`checkpoint`／`release` 在 Task 6／11／12 使用同一签名；
  `CheckpointPort` 的 `checkpoint(&self, &AuthorizedTask)` 在 Task 5 定义、Task 12 使用。
- `SynthesisIo` 的三个方法名与参数语义对应 P6C 的
  `begin_synthesis`／`apply_synthesis`／`fail_synthesis`；P7D 的端口不接收 `Tx`，事务句柄只留在
  实现方。
- `SupervisorError` 的十臂在 Task 5 定义；Task 7／12／13 只构造既有臂，不加第十一臂。

### 4. Review Focus 归位

| Review Focus | 归属用例 |
|---|---|
| 1 键碰撞 | Task 11 `same_policy_reference_shares_one_state_and_is_a_contract_violation`＋`distinct_policy_references_get_independent_states` |
| 2 Resume 不在 candidates | Task 9 `resume_selection_need_not_appear_in_candidates` |
| 3 NoIntent 清空会话 | Task 9 `no_intent_keeps_existing_sessions_and_background` |
| 4 checkpoint 超时丢失 | Task 12 `late_success_returns_timed_out_and_returns_to_throttled`／`timer_hook_returns_timed_out_and_throttles`／`port_failure_returns_checkpoint_error_and_throttles` |
| 5 P6D 断言 5／8 仍红 | Task 2 Step 1／Step 4；Task 14 `allowed_row_has_exactly_five_entries`＋最终树门 |

### 5. Proportion

本计划约与 P7C 计划同量级；生产代码只有七个源文件与一个 crate 清单，测试按 J 判据分组，没有把
实现体抄进计划。所有代码块只出现在「签名／算法不确定处」（`TaskKey`、`next_tasks`、`decide`、
`plan_manual`／`plan_auto`、`checkpoint` 三出口、`P7_EXPECTED_ROWS` 与 fixture），其余用
用例名＋断言值表达。

---

## 遗留/待协调

**设计缺项（不改设计；收件人照设计 §10）：**

1. **§14 越步**：P7D 的 `checkpoint` 只调 `CheckpointPort` 并推进 P7C 的相位；**不实现 checkpoint
   内容、不落库、不杀进程、不做重启恢复**。checkpoint 的数据完整性／存储归装配方／P1（设计 §7.4
   第 4 条、§10 第 9 条）。P7D 不建第二套 `Suspended` 状态机。
2. **判定器归属**：Auto 的「最值得继续」判定器是 P7B 的 `IntentSelectionPolicy`；P7D 只接受
   `&dyn IntentSelectionPolicy`，不实现任何策略，不按 `priority`／finding 重排（设计 §6、§10 第 7
   条）。模型不可用时的降级选择不在 P7D（设计 §10 第 16 条）。
3. **内存态跨重启**：`HashMap<TaskKey, SuspensionState>` 是内存表；重启后相位全部丢失，P7D 不
   落表、不占 201。恢复策略（重建 `Running` 或从持久化快照恢复）在装配方／P1（设计 §10 第 9 条）。
4. **throttle 量化归执行器**：P7D 只原样返回 P7C 的 `ResourceAction::Throttle`；降多少并发／速率
   不在 P7D，也不在 P7C（设计 §7.4 第 5 条、§10 第 9 条）。P7D 不新增 throttle 端口。

**文档措辞待订正（不改设计，只登记）：**

5. 设计 §3.3 正文写「`SessionPlan` 五字段」，§4.5 的冻结面是四字段；本计划按四字段实现，建议
   设计维护者把 §3.3 的「五」改成「四」。
6. 设计 §4.5 把 `NewConversation` 注为「Manual 的固定选项」，§5.4 的 Auto 又明写同一四段；
   本计划按「两模式都含」实现，建议把 §4.5 的注解改成「固定 UI 选项（两模式都含）」。
7. 设计 §9.1 第 4 条仍写「P7D 只调 P6C 的函数」；§4.8 已裁定端口。本计划按端口实现，建议
   §9.1 改成「P7D 只调 `SynthesisIo` 端口；装配方实现并持 `Tx`」。
8. 设计 J12 的 `grep -rn "Evidence"` 会命中 §4.3 自己定义的私有 `TaskEvidence` 与字段
   `evidence`；本计划的源码面按标识符边界判**独立** `Evidence`（`TaskEvidence`／`evidence` 不算），
   主照片是 `BackgroundNote` 字段恰 `label`／`at_millis`。建议设计把 J12 的 grep 改成「独立
   `Evidence` 类型」。

**执行顺序提醒（写死）**：**全部 14 个 task 等待 `continuum-intent-scheduler` 与
`continuum-arbiter` 落地；未交付前不得先写任何 task（包括 Task 2）。** Task 1 的 `Cargo.toml`
四条普通边与 `ALLOWED` 五项一次登记；Task 2 只做本地树绿，**最终树门只在 Task 14 Step 3**，要求
P7A／B／C／D 四块齐全。两者都不由本计划交付；未满足时只报「部分树绿」，不报「P7D 完成」。
