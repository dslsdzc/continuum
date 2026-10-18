# P7B（Intent Scheduler）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**任务单：** 切分 §五 P7B 行（`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md:333`，逐字）：

> 建 Intent Scheduler，并冻结一个由装配方实现的具名资源端口，把「三层调度必须分离」做成可观察结构；
> Graph Scheduler 复用，Resource Scheduler 不重造

**本计划是该行的实现分解。设计与规范的唯一事实来源是**
`docs/superpowers/specs/2026-10-18-p7b-intent-scheduler-design.md`（1451 行，下称「设计」）。
**本计划不改设计、不改规范、不改切分文档、不改代码以外的任何计划。**
引用一律用**设计节号**；规范依据由设计 §3 逐条给出，本计划不复制规范原文。

**Goal:** 新建 crate `continuum-intent-scheduler`，交付**一个 Intent 决策面**（`IntentScheduler::decide` →
`Result<IntentDecision, IntentDecisionError>`，两臂 `Advance`／`NoIntent`）、**一个零逻辑 Graph 消费入口**
（`graph::runnable_nodes` → `Vec<NodeId>`）、**一个具名资源端口**（`ResourceDirectivePort` → 四槽不透明
`ResourceDirective`）与**一个转交入口**（`IntentScheduler::directive_for` → `Result<Option<ResourceDirective>,
DirectiveForError>`）；§302 的五个输入全部显式化，`BaselineIntentPolicy` 是确定性基线；**零表、零迁移、零事件**
（设计 §1.1、§2.2、§4、§5.4、§9）。

**Architecture:** 本块是第 7 层的叶子 crate，两条普通边 `P7B → continuum-semantics`（`Intent`／`IntentId`／
`Priority`／`IntentStatus`）与 `P7B → continuum-graph`（`AdfirGraph`／`NodeId`／`SchedulerConfig`／
`select_runnable`），一条 dev 边 `→ continuum-canonical`（仅 `tests/` 构造 `Intent` 夹具的 `EntityId`／`Opaque`）。
三个决策面**互不合并**：`decide` 只产 `IntentDecision`，`runnable_nodes` 只转调 `select_runnable`，
`ResourceDirectivePort::direct` 由装配方实现；`directive_for` 只把已作出的 `IntentDecision` 转交给端口，
不产出新的 Intent／Graph／Resource 决策（设计 §2.1、§3.1、§5.4）。
`IntentDecision` 的 `NoIntent` **不带任何 `IntentId`**，不是状态、不是哨兵；策略失败与越界 `Advance` 走
`IntentDecisionError`，不折叠成 `NoIntent`（设计 §4.1、§4.5、§5.3）。

**Tech Stack:** Rust 1.95.0 / edition 2024（workspace 的 `rust-version = "1.95"`）。
普通依赖：`continuum-semantics`、`continuum-graph`；dev 依赖：`continuum-canonical`。
**不引入 `thiserror`**：三个错误类型按设计 §4.1／§4.7／§4.8 手写 `Display`／`Error`。
不引入 `serde`／`serde_json`／`trybuild`；不依赖 `continuum-persist`／`continuum-events`／
`continuum-arbiter`／`continuum-model-registry`／`continuum-node`／`continuum-provider`／`continuum-runtime`
（设计 §2.1、§4.9）。

**Spec:** 设计全文（**唯一事实来源**）；切分
`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md` §五 P7B 行（`:333`）、§一、§2.3.2、§2.4、§三、§四、§六.2 第 5／8／12 条；
规范 §301（`docs/spec/05-normative.md:1925-1935`）、§302（`:1939-1955`）、§303（`:1959-1969`）、
§304（`:1973-1982`）、§77（`docs/spec/02-positioning.md:616-631`）、§13（`docs/spec/01-concepts.md:708-722`）、
§222／§223（`docs/spec/05-normative.md:130-205`）；
《工程》§7.1（`docs/02-工程.md:420-431`）、§7.2（`:433-443`）、§7.3（`:445-458`）、§7.4（`:460-467`）、
§7.5（`:469-473`）。**§7.4 的四条判据不覆盖 P7B**（设计 §3.7、§10 第 5 条）。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。`cargo test --workspace --no-fail-fast` 必须全绿、
  **0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。测试命令统一带
  `TMPDIR="$PWD/.tmp"`。
- **`NoIntent` 单一语义**：它表示「本轮没有新的可推进项」，**不表示没有会话**；它**不带 `IntentId`**，
  不是 `IntentStatus` 的变体，不是哨兵空串；`decide` 保证它的 `basis` 非空。P7D 收到它时保留既有会话与
  后台任务（设计 §2.3、§4.1、§5.1、§5.3）。
- **六个可推进状态，逐字**：`Ready`／`Running`／`Waiting`／`Suspended`／`Verifying`／`Evolving`。
  `Proposed`／`Specifying`／`Planning`／`Review`／`Completed`／`Failed`／`Cancelled` **不是候选**
  （设计 §2.5 第 3 条、§4.4）。`Evolving` 的会话存在性由 P7D 在构造候选集时过滤，P7B 只判状态。
- **`Advance` 只给 id**：不携带状态迁移动作、不承诺「启动」还是「继续」；P7B 不调用
  `IntentStatus::transition`、不调用 `apply_intent_transition`、不写 `intent` 表（设计 §4.1、§4.5、§5.1）。
- **不解析 `priority`**：只以 `PrioritySignal { intent, priority: Priority }` 携带；不读 `Intent.priority`，
  不比较、不排序、不建第二个 `Priority`。`Priority` 没有 `Ord`／`PartialOrd`（设计 §2.3、§3.6、§4.2）。
- **不解析 `workload_class`／`resource_pressure`**：`BackgroundSuitability` 是私有 `String` 载体，
  **整轮单值**；不定义 `Suitable`／`Unsuitable`，不引入四档，不依赖 `continuum-arbiter`（设计 §3.6、§4.3）。
- **不写 `intent` 表、零表零迁移零事件**：不登记 `continuum-persist`／`continuum-events`；
  `runtime_migrations()`（`crates/continuum-runtime/src/main.rs:114`）、`expected_migrations()`
  （`crates/continuum-runtime/tests/migrations.rs:37`）、`tests/startup.rs` 的三处计数
  （`:26`／`:81`／`:94`）**一行都不改**（设计 §9）。
- **`ALLOWED` 恰三项**：
  `("continuum-intent-scheduler", &["continuum-canonical", "continuum-graph", "continuum-semantics"])`；
  内层数组按字母序；`continuum-canonical` 是 **dev 边**（仅 `tests/` 构造 `Intent` 夹具）。
  **既有行一条都不因本块增枚**；**不碰 `continuum-runtime` 的 `ALLOWED` 行**（端口实现不在 P7B；
  设计 §2.1、§9.2）。
- **不实现 Resource Scheduler**：`ResourceDirectivePort` 是 trait，`src/` 零 `impl ResourceDirectivePort`；
  不出现 `rank`／`place`／`RoutingRequest`／`PlacementRequest`／`ComputeNode`／`ExecutionCandidate`；
  不宣称 §304 已闭合（设计 §3.4、§4.7、§7.2 J12／J15）。
- **不调用模型**：`src/` 不出现 `ModelProvider`／`InvokeRequest`；不依赖 `continuum-provider`／
  `continuum-runtime`（设计 §3.7、§7.2 J14）。
- **测试一律放 `crates/continuum-intent-scheduler/tests/`**，不用 `src/` 的 `#[cfg(test)]`——
  J1-b／J3-a／J4-a／J5-a／J10／J11／J12／J14／J15 的源码面语料限定 `src/`，测试构造不得混入。
- 代码注释、错误信息、测试断言信息用中文；标识符用英文。
- **不要用 `git add -A`，不要 `git commit --amend`**；只 `git add <显式路径>`。新 crate 会改
  `Cargo.lock`，提交时按显式路径带上。
- 本块**不碰** `crates/continuum-runtime/src/**`、`crates/continuum-graph/src/**`、
  `crates/continuum-semantics/src/**`、`crates/continuum-canonical/src/**`、`docs/02-工程.md`、
  `docs/spec/**`、`docs/superpowers/specs/**`。

---

## Review Focus

复审时**逐条按下面的用例名核**；每条都归到拥有该代码的 task 的测试，不留在「设计说了」层面。

| # | 要试着推翻的断言 | 归属 task 与用例 |
|---|---|---|
| 1 | **`WAITING`／`SUSPENDED` 活跃会话被基线过滤掉**：候选 `[READY, WAITING]`／`[READY, SUSPENDED]` 时只推进 `READY`，把「继续会话」当成「没有新工作」 | **Task 4** `baseline_selects_each_active_status_under_user_focus`（五个活跃状态逐组；若基线只留 `Ready`，聚焦命中失败、落到 `READY`，该组红） |
| 2 | **`NoIntent` 被当成「没有会话」或哨兵空串**：给 `NoIntent` 加 `IntentId`、用 `IntentId::from_str("")` 表示无 Intent，或消费方据 `NoIntent` 清理会话 | **Task 2** `decision_has_exactly_two_arms_and_no_intent_has_no_identifier`（无通配穷尽解构＋`advanced()==None`）＋**Task 3** `empty_candidates_yield_no_intent`＋**Task 7** `no_empty_intent_id_sentinel_in_source` |
| 3 | **策略失败／越界 `Advance` 被吞成 `NoIntent`**：`choose` 返回 `Err` 或返回候选集外的 id，`decide` 却给 `Ok(NoIntent)`；`directive_for` 把非法 `Advance` 当 `Ok(None)` | **Task 2** `an_advance_outside_the_candidate_set_is_rejected`／`an_advance_to_a_completed_candidate_is_rejected`／`a_policy_error_is_policy_failed_and_keeps_the_basis`＋**Task 6** `an_invalid_advance_is_rejected_without_calling_the_port` |
| 4 | **`COMPLETED` 被当可推进**：把「非终态」或「所有状态」当候选集，`COMPLETED` 进入 `Advance` | **Task 4** `seven_non_candidate_statuses_yield_no_intent`（逐状态含 `Completed`）＋**Task 2** `an_advance_to_a_completed_candidate_is_rejected` |
| 5 | **`EVOLVING` 无活跃会话仍被推进**：P7B 只判状态、无法判会话；若消费方不先过滤，`Advance` 会指向没有会话的 Product Intent | **Task 4** `six_advanceable_statuses_pass_the_decide_check` 与 `baseline_selects_each_active_status_under_user_focus` 的 `Evolving` 组；测试注释明写「状态合法≠会话存在，过滤责任在 P7D」（设计 §2.5 第 3 条、§5.1） |

---

## 跨计划前置（依赖的实际状态与共写文件）

### 一、依赖的交付物与它们在本工作树的**实际状态**（2026-10-19 实读）

| 依赖 | 实际状态 | 本块用在哪 |
|---|---|---|
| `continuum-semantics` | **已交付**：`Intent`（`crates/continuum-semantics/src/intent.rs:434`）、`IntentId`（`src/ids.rs:41`）、`Priority`（`intent.rs:300`）、`IntentStatus`（`intent.rs:61`）、`ContractRef`／`ContractId`（`intent.rs:331`／`ids.rs:83`）；根导出在 `src/lib.rs:272-275` | `SchedulerInputs.candidates: &[Intent]`、`PrioritySignal.priority`、`is_advanceable(IntentStatus)`、测试夹具 |
| `continuum-graph` | **已交付**：`select_runnable`（`crates/continuum-graph/src/scheduler.rs:28`）、`SchedulerConfig`（`:9`）、`AdfirGraph`（`src/graph.rs:41`）、`NodeId`（`src/ids.rs:40`）；根导出在 `src/lib.rs:21-29` | `graph::runnable_nodes` 的零逻辑转调 |
| `continuum-canonical` | **已交付**：`EntityId`（`crates/continuum-canonical/src/candidate.rs:62`）、`Opaque`（`src/lib.rs:152`）；仅 `tests/` 用于 `Intent.goal`／`completion_predicate` | **dev 边**；不进入 `src/` |
| `continuum-runtime` 的依赖门 | **已交付**：`ALLOWED` 在 `crates/continuum-runtime/tests/dependency_direction.rs:26`，门在 `:494`；`cargo_tree_direct` 用 `--edges all`（`:476-491`），故 dev 边计入直接边 | J16；新增行必须是三项 |
| `continuum-intent-scheduler` | **不存在**：`grep -rn "continuum-intent-scheduler\|continuum_intent_scheduler" crates/ Cargo.toml` 零命中 | 本计划的全部交付物 |
| P7C／P7D | **不依赖**：本块不登记 `continuum-arbiter`／`continuum-supervisor`；P7C 结论由端口实现持有，P7D 对着本块 §4.1 写 | 无 crate 边；设计 §6 |

**结论：本块没有硬前置。** 三个 workspace 依赖都已落地；P7C／P7D 尚未实现也不阻塞本块。
Task 2–8 只依赖本计划先前 task 的接口，不依赖外部块。

### 二、共写文件（**只有两处**）

| 文件 | 本块的改动 | 落在哪个 task |
|---|---|---|
| 根 `Cargo.toml` 的 `[workspace] members`（`:3`） | 加一行 `"crates/continuum-intent-scheduler"` | Task 1 Step 3 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-intent-scheduler", &["continuum-canonical", "continuum-graph", "continuum-semantics"])`；内层按字母序 | Task 1 Step 3 |

**不碰**：`runtime_migrations()`、`expected_migrations()`、`startup.rs` 三处计数、
`crates/continuum-runtime` 的 `ALLOWED` 行、`p6_pre_allowed.txt`、`P6_EXPECTED_ROWS`（设计 §2.1、§9.1）。
`Cargo.lock` 随新 crate 变化，Task 1 一并提交。

### 三、哪些 task 可立即开工

| task | 能否立即开工 | 说明 |
|---|---|---|
| Task 1–8 | **全部能** | 无外部硬前置；Task 2 起依赖前一 task 的接口，按序号执行即可 |

**新 crate 进 workspace 会让 `dependency_direction` 先红**：`workspace_crates()` 从 workspace 定义派生，
`ALLOWED` 缺行时 `every_crate_depends_only_on_its_allowed_set` 报「成员未列入 ALLOWED」。这是刻意的；
Task 1 同一步加 member 与 `ALLOWED` 行，**不要把这条红当回归**。

---

## File Structure

设计 §2.2 的模块表只有**五个**源文件；本计划**不建** `suitability.rs`／`port.rs`／`policy.rs`／`error.rs`
——拆细会把 `is_advanceable`（`pub(crate)`）与 `IntentScheduler` 的两个 `impl` 块拆散，并违反设计 §2.2
「不新增 crate 外的路径」。

```
crates/continuum-intent-scheduler/
  Cargo.toml             两条普通边；dev：continuum-canonical
  src/lib.rs             只做 pub mod 与 pub use，不定义类型（设计 §2.2）
  src/decision.rs        IntentDecision / IntentDecisionError / DecisionBasis /
                         IntentSelectionPolicy / BaselineIntentPolicy / is_advanceable /
                         IntentScheduler::decide（设计 §4.1、§4.4、§4.5）
  src/inputs.rs          SchedulerInputs / PrioritySignal / Deadline / Blocking / UserFocus /
                         BackgroundSuitability（设计 §4.2、§4.3）
  src/graph.rs           runnable_nodes：select_runnable 的零逻辑转调（设计 §4.6）
  src/resource.rs        ExecutionRequest / ExecutionSignals / ResourceDirective /
                         ResourceDirectiveError / DirectiveForError / ResourceDirectivePort /
                         IntentScheduler::directive_for（设计 §4.7、§4.8）
  tests/support/mod.rs   Intent 夹具、SchedulerInputs 构造器、FixedPolicy（Task 2 起）
  tests/inputs.rs        §302 五输入形状与 BackgroundSuitability 往返（Task 1）
  tests/decision.rs      J1-a、J8-a–e（Task 2）
  tests/baseline.rs      J1-c／J6、J9 七项、J3-b（Task 3）
  tests/status_matrix.rs J7 七负例／六正例、J7-b（Task 4）
  tests/graph.rs         J11-a／J11-b、J10-b（Task 5）
  tests/resource.rs      J13-a–e、J12-a／J12-b、J10-c（Task 6）
  tests/source_surface.rs J1-b、J3-a、J4-a、J5-a、J10-a、J14、J15（Task 7）
  tests/manifest.rs      清单恰三条边、零表／零资源／零运行时边（Task 8）
```

**本计划要改的既有文件**只有两处（都在共写表里）：

```
Cargo.toml                                             members 加 crates/continuum-intent-scheduler（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs  ALLOWED 加 continuum-intent-scheduler 行（Task 1）
```

**测试构造的限制（实读结论）**：`AdfirGraph` 的非空节点需要 `Node::new(NodeId, OperatorId, OperatorVersion)`，
而 `OperatorId`／`OperatorVersion` 由 `continuum-operator` 提供；端口还需要 `continuum-port`。设计 §2.1 的
dev 边只允许 `continuum-canonical`，因此 **Task 5 的运行期图夹具只建空图**；J11 的主证据是源码面
（函数体恰一行转调），不是非空图的行为。这条限制记在「遗留/待协调」第 5 条。

---

## Task 1: crate 骨架与 §302 五输入

**Files:**
- Create: `crates/continuum-intent-scheduler/Cargo.toml`
- Create: `crates/continuum-intent-scheduler/src/lib.rs`
- Create: `crates/continuum-intent-scheduler/src/inputs.rs`
- Create: `crates/continuum-intent-scheduler/tests/inputs.rs`
- Modify: `Cargo.toml`（`[workspace] members`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED`）

**Interfaces:**
- Consumes: `continuum_semantics::{Intent, IntentId, Priority}`（类型位置，不读字段）
- Produces: `continuum_intent_scheduler::{SchedulerInputs, PrioritySignal, Deadline, Blocking, UserFocus, BackgroundSuitability}`

- [ ] **Step 1: 写 `tests/inputs.rs` 的三条用例**

```rust
use continuum_intent_scheduler::{
    BackgroundSuitability, Blocking, Deadline, PrioritySignal, SchedulerInputs, UserFocus,
};
use continuum_semantics::{IntentId, Priority};

fn empty_inputs() -> SchedulerInputs<'static> {
    SchedulerInputs {
        candidates: &[],
        priority: &[],
        deadline: &[],
        blocking: &[],
        user_focus: UserFocus::None,
        background_suitability: BackgroundSuitability::from_projection("background"),
    }
}

#[test]
fn scheduler_inputs_carries_candidates_and_the_five_302_inputs() {
    let inputs = empty_inputs();
    // 不写 `..`：字段名/数量与 §302 的五个名字逐字绑定；改名、加字段、删字段都编译失败。
    let SchedulerInputs {
        candidates,
        priority,
        deadline,
        blocking,
        user_focus,
        background_suitability,
    } = &inputs;
    assert!(candidates.is_empty());
    assert!(priority.is_empty());
    assert!(deadline.is_empty());
    assert!(blocking.is_empty());
    assert!(matches!(user_focus, UserFocus::None));
    assert_eq!(background_suitability.projection(), "background");
}

#[test]
fn background_suitability_round_trips_projection_verbatim() {
    for value in ["", " ", " 后台 适合 ", "适合\u{1F642}"] {
        let carrier = BackgroundSuitability::from_projection(value);
        assert_eq!(carrier.projection(), value, "投影值不得改写、裁剪或归一");
    }
}

#[test]
fn priority_signal_carries_the_opaque_priority_verbatim() {
    let signal = PrioritySignal {
        intent: IntentId::from_str("i1"),
        priority: Priority::from_str("P0"),
    };
    assert_eq!(signal.priority.as_str(), "P0");
    assert_eq!(signal.intent.as_str(), "i1");
}
```

**结果：** 三条用例名可用 `cargo test -p continuum-intent-scheduler --test inputs -- --list` 列出（Step 3 之后）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test inputs
```

**结果：** 报「package ID specification did not match any packages」（package 未找到），
不是断言失败，也不是 `E0432`；按纪律这**不算红**。

- [ ] **Step 3: 建 crate、输入类型与两处共写文件**

**动作 3a：** 新建 `Cargo.toml`：

```toml
[package]
name = "continuum-intent-scheduler"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
# `AdfirGraph` / `NodeId` / `SchedulerConfig` / `select_runnable`（设计 §2.1）。
continuum-graph = { path = "../continuum-graph" }
# `Intent` / `IntentId` / `Priority` / `IntentStatus`（设计 §2.1）。
continuum-semantics = { path = "../continuum-semantics" }

[dev-dependencies]
# 仅 `tests/` 构造 `Intent` 夹具：`EntityId`（goal）与 `Opaque`（completion_predicate）。
# `cargo tree --edges all` 会把这条 dev 边算进直接边，故 ALLOWED 行是三项。
continuum-canonical = { path = "../continuum-canonical" }
```

**动作 3b：** 新建 `src/lib.rs`（Task 2／5／6 会追加模块与导出；本 task 只导出 inputs）：

```rust
//! P7B Intent Scheduler：§302 的决策面、§303 的零逻辑转调与 §304 的具名资源端口。
//! 本 crate 不持久化、不迁移 Intent 状态、不实现 Resource Scheduler。

pub mod inputs;

pub use inputs::{
    BackgroundSuitability, Blocking, Deadline, PrioritySignal, SchedulerInputs, UserFocus,
};
```

**动作 3c：** 新建 `src/inputs.rs`。类型、字段名与派生逐字照设计 §4.2／§4.3；**只加文档注释，不加方法**
（`BackgroundSuitability` 的两个方法除外）：

```rust
use continuum_semantics::{Intent, IntentId, Priority};

#[derive(Debug)]
pub struct SchedulerInputs<'a> {
    pub candidates: &'a [Intent],
    pub priority: &'a [PrioritySignal],
    pub deadline: &'a [Deadline],
    pub blocking: &'a [Blocking],
    pub user_focus: UserFocus,
    pub background_suitability: BackgroundSuitability,
}

#[derive(Debug, Clone)]
pub struct PrioritySignal {
    pub intent: IntentId,
    pub priority: Priority,
}

#[derive(Debug, Clone)]
pub struct Deadline {
    pub intent: IntentId,
    pub at_millis: i64,
}

#[derive(Debug, Clone)]
pub struct Blocking {
    pub intent: IntentId,
    pub blocks: Vec<IntentId>,
}

#[derive(Debug, Clone)]
pub enum UserFocus {
    None,
    Intent(IntentId),
    Area(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundSuitability(String);

impl BackgroundSuitability {
    pub fn from_projection(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn projection(&self) -> &str {
        &self.0
    }
}
```

**动作 3d：** 根 `Cargo.toml` 的 `members` 加 `"crates/continuum-intent-scheduler"`。

**动作 3e：** `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加一条
（放在 `continuum-graph` 行之后、按 crate 名字母序找位置；内层数组按字母序）：

```rust
    // P7B Intent Scheduler（设计 §2.1）。两条普通边 + 一条 dev 边：
    // graph（select_runnable / SchedulerConfig）、semantics（Intent / IntentId / Priority）、
    // canonical（仅 tests/ 构造 Intent 夹具）。**不碰 runtime 的 ALLOWED 行**。
    (
        "continuum-intent-scheduler",
        &[
            "continuum-canonical",
            "continuum-graph",
            "continuum-semantics",
        ],
    ),
```

- [ ] **Step 4: 运行本 task 的测试与依赖门，并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test inputs
TMPDIR="$PWD/.tmp" timeout 1800 cargo test -p continuum-runtime --test dependency_direction
git add crates/continuum-intent-scheduler/Cargo.toml \
        crates/continuum-intent-scheduler/src/lib.rs \
        crates/continuum-intent-scheduler/src/inputs.rs \
        crates/continuum-intent-scheduler/tests/inputs.rs \
        Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(intent-scheduler): crate 骨架与 §302 五输入"
```

**结果：** `inputs` 三条全绿；依赖门绿；`cargo tree -p continuum-intent-scheduler --depth 1 --edges all
--prefix none` 恰出现 `continuum-canonical`／`continuum-graph`／`continuum-semantics` 三个 workspace 成员；
`git status` 只剩 `.tmp/`（不入库）。

---

## Task 2: `IntentDecision`、错误面与 `decide`

**Files:**
- Create: `crates/continuum-intent-scheduler/src/decision.rs`
- Create: `crates/continuum-intent-scheduler/tests/support/mod.rs`
- Create: `crates/continuum-intent-scheduler/tests/decision.rs`
- Modify: `crates/continuum-intent-scheduler/src/lib.rs`（导出 decision）

**Interfaces:**
- Consumes: Task 1 的 `SchedulerInputs`；`continuum_semantics::{Intent, IntentId, IntentStatus}`；
  `continuum_canonical::{EntityId, Opaque}`（仅测试夹具）
- Produces: `IntentDecision`（两臂）、`DecisionBasis`、`IntentDecisionError`、
  `IntentSelectionPolicy`、`IntentScheduler::decide`；`pub(crate) fn is_advanceable(IntentStatus) -> bool`

- [ ] **Step 1: 写 `tests/support/mod.rs` 与 `tests/decision.rs`**

`tests/support/mod.rs`（所有后续集成测试共用；**不写进 `src/`**）：

```rust
#![allow(dead_code)]

use std::cell::Cell;

use continuum_canonical::{EntityId, Opaque};
use continuum_intent_scheduler::{
    BackgroundSuitability, DecisionBasis, IntentDecision, IntentSelectionPolicy, SchedulerInputs,
    UserFocus,
};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority,
};

pub fn intent(id: &str, status: IntentStatus) -> Intent {
    Intent {
        id: IntentId::from_str(id),
        version: 1,
        kind: IntentKind::Task,
        goal: EntityId::from_str("goal"),
        status,
        priority: Priority::from_str("p0"),
        created_at: 0,
        updated_at: 0,
        parent_intent: None,
        contract_id: ContractRef { id: ContractId::from_str("contract"), version: 1 },
        active_plan_id: None,
        graph_id: None,
        product_id: None,
        completion_predicate: Opaque::from_str("done"),
    }
}

pub fn inputs<'a>(candidates: &'a [Intent]) -> SchedulerInputs<'a> {
    SchedulerInputs {
        candidates,
        priority: &[],
        deadline: &[],
        blocking: &[],
        user_focus: UserFocus::None,
        background_suitability: BackgroundSuitability::from_projection("background"),
    }
}

pub fn advance(id: &str, basis: &str) -> IntentDecision {
    IntentDecision::Advance {
        intent: IntentId::from_str(id),
        basis: DecisionBasis::new(basis),
    }
}

pub fn no_intent(basis: &str) -> IntentDecision {
    IntentDecision::NoIntent { basis: DecisionBasis::new(basis) }
}

pub struct FixedPolicy {
    decision: Result<IntentDecision, DecisionBasis>,
    calls: Cell<usize>,
}

impl FixedPolicy {
    pub fn returning(decision: IntentDecision) -> Self {
        Self { decision: Ok(decision), calls: Cell::new(0) }
    }

    pub fn failing(basis: &str) -> Self {
        Self { decision: Err(DecisionBasis::new(basis)), calls: Cell::new(0) }
    }

    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl IntentSelectionPolicy for FixedPolicy {
    fn choose(&self, _inputs: &SchedulerInputs<'_>) -> Result<IntentDecision, DecisionBasis> {
        self.calls.set(self.calls.get() + 1);
        self.decision.clone()
    }
}
```

`tests/decision.rs`：

```rust
mod support;

use continuum_intent_scheduler::{
    DecisionBasis, IntentDecision, IntentDecisionError, IntentScheduler,
};
use continuum_semantics::IntentStatus;

fn arm(decision: &IntentDecision) -> &'static str {
    // 不写 `_`：新增第三臂会编译失败（J1-a 的穷尽解构）。
    match decision {
        IntentDecision::Advance { .. } => "advance",
        IntentDecision::NoIntent { .. } => "no_intent",
    }
}

#[test]
fn decision_has_exactly_two_arms_and_no_intent_has_no_identifier() {
    let no = IntentDecision::NoIntent { basis: DecisionBasis::new("none") };
    assert_eq!(arm(&no), "no_intent");
    assert!(no.advanced().is_none());
    assert!(no.is_no_intent());
    assert_eq!(no.basis().as_str(), "none");

    let yes = support::advance("i1", "chosen");
    assert_eq!(arm(&yes), "advance");
    assert_eq!(yes.advanced().map(|id| id.as_str()), Some("i1"));
}

#[test]
fn an_advance_outside_the_candidate_set_is_rejected() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::advance("ghost", "chosen"));

    match IntentScheduler::decide(&inputs, &policy) {
        Err(IntentDecisionError::InvalidAdvance { intent }) => {
            assert_eq!(intent.as_str(), "ghost")
        }
        other => panic!("期望 InvalidAdvance，实际 {other:?}"),
    }
}

#[test]
fn an_advance_to_a_completed_candidate_is_rejected() {
    let candidates = [support::intent("done", IntentStatus::Completed)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::advance("done", "chosen"));

    match IntentScheduler::decide(&inputs, &policy) {
        Err(IntentDecisionError::InvalidAdvance { intent }) => {
            assert_eq!(intent.as_str(), "done")
        }
        other => panic!("期望 InvalidAdvance，实际 {other:?}"),
    }
}

#[test]
fn a_legal_advance_is_returned_unchanged() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::advance("ready", "chosen"));

    let decision = IntentScheduler::decide(&inputs, &policy).expect("合法 Advance 应通过");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("ready"));
    assert_eq!(decision.basis().as_str(), "chosen");
}

#[test]
fn a_policy_error_is_policy_failed_and_keeps_the_basis() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::failing("model_unavailable");

    match IntentScheduler::decide(&inputs, &policy) {
        Err(IntentDecisionError::PolicyFailed { basis }) => {
            assert_eq!(basis.as_str(), "model_unavailable")
        }
        other => panic!("期望 PolicyFailed，实际 {other:?}"),
    }
}

#[test]
fn an_empty_no_intent_basis_is_normalized_to_no_intent() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::no_intent(""));

    let decision = IntentScheduler::decide(&inputs, &policy).expect("NoIntent 应通过");
    assert!(decision.is_no_intent());
    assert_eq!(decision.basis().as_str(), "no_intent");
}

#[test]
fn a_non_empty_no_intent_basis_is_returned_unchanged() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::no_intent("user_busy"));

    let decision = IntentScheduler::decide(&inputs, &policy).expect("NoIntent 应通过");
    assert_eq!(decision.basis().as_str(), "user_busy");
}
```

**结果：** 七条用例名可列出；`support` 中的 `intent()` 用 `continuum-canonical` 构造 `EntityId`／`Opaque`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test decision
```

**结果：** 编译失败（`E0432`：`continuum_intent_scheduler::IntentScheduler` 等尚不存在）。
**这不是红**，是编译期照片。

- [ ] **Step 3: 实现 `decision.rs` 并导出**

签名与语义逐字照设计 §4.1／§4.4／§4.5：

```rust
use continuum_semantics::{Intent, IntentId, IntentStatus};

use crate::inputs::SchedulerInputs;

#[derive(Debug, Clone)]
pub enum IntentDecision {
    Advance { intent: IntentId, basis: DecisionBasis },
    NoIntent { basis: DecisionBasis },
}

impl IntentDecision {
    pub fn advanced(&self) -> Option<&IntentId> {
        match self {
            Self::Advance { intent, .. } => Some(intent),
            Self::NoIntent { .. } => None,
        }
    }

    pub fn basis(&self) -> &DecisionBasis {
        match self {
            Self::Advance { basis, .. } | Self::NoIntent { basis } => basis,
        }
    }

    pub fn is_no_intent(&self) -> bool {
        matches!(self, Self::NoIntent { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionBasis(String);

impl DecisionBasis {
    pub fn new(detail: impl Into<String>) -> Self { Self(detail.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone)]
pub enum IntentDecisionError {
    PolicyFailed { basis: DecisionBasis },
    InvalidAdvance { intent: IntentId },
}

// `Display` / `Error`：文案照设计 §4.1；不引入 thiserror。

pub trait IntentSelectionPolicy {
    fn choose(
        &self,
        inputs: &SchedulerInputs<'_>,
    ) -> Result<IntentDecision, DecisionBasis>;
}

pub(crate) fn is_advanceable(status: IntentStatus) -> bool {
    matches!(
        status,
        IntentStatus::Ready
            | IntentStatus::Running
            | IntentStatus::Waiting
            | IntentStatus::Suspended
            | IntentStatus::Verifying
            | IntentStatus::Evolving
    )
}

#[derive(Debug, Default, Clone, Copy)]
pub struct IntentScheduler;

impl IntentScheduler {
    pub fn decide(
        inputs: &SchedulerInputs<'_>,
        policy: &dyn IntentSelectionPolicy,
    ) -> Result<IntentDecision, IntentDecisionError> {
        let decision = policy
            .choose(inputs)
            .map_err(|basis| IntentDecisionError::PolicyFailed { basis })?;
        match decision {
            IntentDecision::Advance { intent, basis } => {
                let eligible = inputs.candidates.iter().any(|candidate| {
                    candidate.id.as_str() == intent.as_str()
                        && is_advanceable(candidate.status)
                });
                if eligible {
                    Ok(IntentDecision::Advance { intent, basis })
                } else {
                    Err(IntentDecisionError::InvalidAdvance { intent })
                }
            }
            IntentDecision::NoIntent { basis } => {
                let basis = if basis.as_str().trim().is_empty() {
                    DecisionBasis::new("no_intent")
                } else {
                    basis
                };
                Ok(IntentDecision::NoIntent { basis })
            }
        }
    }
}
```

`src/lib.rs` 加 `pub mod decision;` 与设计 §2.2 的五个 decision 导出（`IntentScheduler` 也在内）。
**不派生 `PartialEq`**：`IntentId` 没有 `PartialEq`（设计 §4.1 末）；比较一律经 `as_str()`。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test decision
git add crates/continuum-intent-scheduler/src/decision.rs \
        crates/continuum-intent-scheduler/src/lib.rs \
        crates/continuum-intent-scheduler/tests/support/mod.rs \
        crates/continuum-intent-scheduler/tests/decision.rs
git commit -m "feat(intent-scheduler): IntentDecision 与 decide 的校验面"
```

**结果：** 七条全绿；`src/decision.rs` 的 `IntentDecision` 恰两臂。

---

## Task 3: `BaselineIntentPolicy` 的五步次序

**Files:**
- Modify: `crates/continuum-intent-scheduler/src/decision.rs`（追加基线）
- Create: `crates/continuum-intent-scheduler/tests/baseline.rs`

**Interfaces:**
- Consumes: Task 2 的 `IntentSelectionPolicy`／`IntentDecision`／`is_advanceable`；Task 1 的 `SchedulerInputs`
- Produces: `BaselineIntentPolicy`（`Debug, Default, Clone, Copy`）

- [ ] **Step 1: 写 `tests/baseline.rs` 的九条用例**

1. `empty_candidates_yield_no_intent`（**J1-c 与 J6 同一张照片**）：
   `candidates = &[]`，`IntentScheduler::decide(..., &BaselineIntentPolicy)` ⇒ `Ok(NoIntent)`；
   `advanced() == None`；`basis().as_str()` 非空，且等于 `"no_advanceable_candidate"`。
2. `user_focus_beats_deadline_and_blocking`：A 有最早 deadline 且 blocking 度更大，B 被
   `UserFocus::Intent(B)` 聚焦 ⇒ 选 B，`basis == "user_focus"`。
3. `the_earliest_deadline_wins_and_multiple_entries_take_the_minimum`：A 的 `Deadline` 条目为
   `50` 与 `20`，B 为 `30` ⇒ 选 A（每枚候选先取自己最早的一条），`basis == "earliest_deadline"`。
4. `the_largest_deduped_blocking_set_wins`：A 有两条 `Blocking` 条目，合并后 `blocks` 为
   `[x, x, y]` 与 `[y, z]`，去重计数 3；B 为 `[x, y]`，计数 2 ⇒ 选 A，
   `basis == "blocking_degree"`。**用 `as_str()` 去重，不用 `HashSet<IntentId>`**（`IntentId` 无 `Hash`）。
5. `candidate_order_is_the_last_tiebreak`：候选 `[Proposed, Ready]`，无 focus／deadline／blocking ⇒
   选第二枚 `Ready`（第一枚不是可推进候选），`basis == "candidate_order"`。
6. `priority_and_background_suitability_do_not_change_the_decision`（**J3-b＋J9⑤**）：
   A 输入序在前、`PrioritySignal` 为 `"z"`；B 为 `"a"`；其余为空。第一次基线选 A。把两条
   `PrioritySignal` 互换（A 为 `"a"`、B 为 `"z"`），并换一个 `BackgroundSuitability` 值，再调一次
   ⇒ 仍选 A。**若基线按 priority 排序，第一次就会选 B。**
7. `signals_pointing_outside_the_candidates_are_ignored`（**J9⑥**）：唯一候选 A；`Deadline`／
   `Blocking` 各有一条指向 `ghost` 的条目 ⇒ `decide` 仍 `Ok(Advance A)`，`basis == "candidate_order"`。
8. `duplicate_candidate_ids_are_not_deduplicated`（**J9⑦**）：候选 `[dup, dup, other]` 全为 `Ready`，
   无信号 ⇒ 只断言选中的 id 是 `"dup"`；**不断言选中哪一枚的元数据**（设计 §2.5 第 15 条）。
9. `the_same_inputs_yield_the_same_decision`：同一输入、同一基线连续调用两次；两次的
   `advanced().map(as_str)` 与 `basis().as_str()` 完全相同。

所有用例的 `SchedulerInputs` 用 `support::inputs` 起底，再逐字段改写；**不要给 `SchedulerInputs`
加 `Clone`／`Default`**（设计 §4.2 没有给）。

**结果：** 九条用例名可列出。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test baseline
```

**结果：** 编译失败（`E0432`：`BaselineIntentPolicy` 尚不存在）。**这不是红**。

- [ ] **Step 3: 在 `decision.rs` 追加基线**

次序逐字照设计 §4.4；**基线不读 `priority`、不读 `background_suitability`**。
`decision.rs` 的 `use crate::inputs::{Blocking, SchedulerInputs, UserFocus};` 要补齐
（`Deadline` 只经字段访问，不必单独命名）：

1. `eligible` = 候选里 `is_advanceable(status)` 为真者；空 ⇒
   `Ok(NoIntent { basis: "no_advanceable_candidate" })`。
2. `UserFocus::Intent(id)` 按 `as_str()` 命中 `eligible` ⇒ `Ok(Advance(id))`，依据 `"user_focus"`。
3. 在有 `Deadline` 条目的 `eligible` 中，每枚候选先取自己最早的一条，再选 `at_millis` 最小者；
   **同值按输入序**。依据 `"earliest_deadline"`。`Iterator::min_by_key` 在并列时返回第一个最小元素，
   可直接用；要显式保留这个语义，不要把候选先排序。
4. 在有 `Blocking` 条目的 `eligible` 中，合并该候选所有 `Blocking` 条目的 `blocks`，按 `as_str()`
   排序去重后计条数；选条数最大者，**同值按输入序**，依据 `"blocking_degree"`。
   **不要用 `Iterator::max_by_key`**：它在并列时返回最后一个最大值，会把「同值按输入序」变成
   「同值取最后」。用严格 `>` 的 `fold`／`for` 保留第一个最大值。
5. 否则选 `eligible[0]`，依据 `"candidate_order"`。

关键实现形状（只需这两处不被写错；其余照上面的次序）：

```rust
// 每个候选先取自己最早的 deadline；min_by_key 并列取第一个。
let deadline_pick = eligible
    .iter()
    .filter_map(|candidate| {
        inputs.deadline.iter()
            .filter(|d| d.intent.as_str() == candidate.id.as_str())
            .map(|d| d.at_millis)
            .min()
            .map(|at| (*candidate, at))
    })
    .min_by_key(|(_, at)| *at);

// blocking：合并 + 按 as_str 去重 + 严格 > 保留输入序第一个最大值。
let mut best: Option<(&Intent, usize)> = None;
for candidate in &eligible {
    let records: Vec<&Blocking> = inputs.blocking.iter()
        .filter(|b| b.intent.as_str() == candidate.id.as_str())
        .collect();
    if records.is_empty() {
        continue;
    }
    let mut blocked: Vec<&str> = records.iter()
        .flat_map(|b| b.blocks.iter().map(|id| id.as_str()))
        .collect();
    blocked.sort_unstable();
    blocked.dedup();
    let count = blocked.len();
    match best {
        Some((_, best_count)) if count <= best_count => {}
        _ => best = Some((*candidate, count)),
    }
}
```

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test baseline
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test decision
git add crates/continuum-intent-scheduler/src/decision.rs \
        crates/continuum-intent-scheduler/tests/baseline.rs
git commit -m "feat(intent-scheduler): 确定性基线策略"
```

**结果：** `baseline` 九条与 `decision` 七条全绿。

---

## Task 4: 六个可推进状态与七个非候选状态的矩阵

**Files:**
- Create: `crates/continuum-intent-scheduler/tests/status_matrix.rs`

**Interfaces:**
- Consumes: Task 2 的 `IntentScheduler::decide`／`IntentDecision`；Task 3 的 `BaselineIntentPolicy`；
  Task 2 的 `support::{intent, inputs, advance, FixedPolicy}`
- Produces: 无生产代码；三组运行期照片

- [ ] **Step 1: 写 `tests/status_matrix.rs` 的三条用例**

**第一条：七个非候选状态（J7 负例，逐状态一组）**

```rust
mod support;

use continuum_intent_scheduler::{IntentDecision, IntentScheduler, BaselineIntentPolicy};
use continuum_semantics::IntentStatus;

#[test]
fn seven_non_candidate_statuses_yield_no_intent() {
    let statuses = [
        IntentStatus::Proposed,
        IntentStatus::Specifying,
        IntentStatus::Planning,
        IntentStatus::Review,
        IntentStatus::Completed,
        IntentStatus::Failed,
        IntentStatus::Cancelled,
    ];
    for status in statuses {
        let candidates = [support::intent("x", status)];
        let inputs = support::inputs(&candidates);
        let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy)
            .unwrap_or_else(|error| panic!("{status:?} 不应产生错误面：{error:?}"));
        assert!(matches!(decision, IntentDecision::NoIntent { .. }), "{status:?} 不得被推进");
        assert!(decision.advanced().is_none());
        assert!(!decision.basis().as_str().is_empty());
    }
}
```

**第二条：六个可推进状态（J7 正例，逐状态一组；只证 `decide` 的候选校验，不证基线会选中）**

```rust
#[test]
fn six_advanceable_statuses_pass_the_decide_check() {
    let statuses = [
        IntentStatus::Ready,
        IntentStatus::Running,
        IntentStatus::Waiting,
        IntentStatus::Suspended,
        IntentStatus::Verifying,
        IntentStatus::Evolving,
    ];
    for status in statuses {
        let candidates = [support::intent("x", status)];
        let inputs = support::inputs(&candidates);
        let policy = support::FixedPolicy::returning(support::advance("x", "legal"));
        let decision = IntentScheduler::decide(&inputs, &policy)
            .unwrap_or_else(|error| panic!("{status:?} 应可推进：{error:?}"));
        assert_eq!(decision.advanced().map(|id| id.as_str()), Some("x"), "{status:?}");
        // Evolving 只按状态合法性构造；本用例不代表存在活跃会话。P7D 必须在候选集构造时过滤。
    }
}
```

**第三条：基线逐活跃状态正例（J7-b，五个活跃状态各一组）**

```rust
#[test]
fn baseline_selects_each_active_status_under_user_focus() {
    let active_statuses = [
        IntentStatus::Running,
        IntentStatus::Waiting,
        IntentStatus::Suspended,
        IntentStatus::Verifying,
        IntentStatus::Evolving,
    ];
    for status in active_statuses {
        let candidates = [
            support::intent("ready", IntentStatus::Ready),
            support::intent("active", status),
        ];
        let mut inputs = support::inputs(&candidates);
        inputs.user_focus =
            continuum_intent_scheduler::UserFocus::Intent(continuum_semantics::IntentId::from_str("active"));

        let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
        assert_eq!(
            decision.advanced().map(|id| id.as_str()),
            Some("active"),
            "{status:?} 不得被基线过滤掉；若被过滤，聚焦命中失败后会落到 READY"
        );
        assert_eq!(decision.basis().as_str(), "user_focus");
        // Evolving：本组只证明基线不按状态过滤；P7D 必须只在确有对应活跃会话时把它放进候选集。
    }
}
```

**结果：** 三条用例名可列出；**七负例／六正例／J7-b 三组是三个独立测试函数**，不混写。

- [ ] **Step 2: 运行，确认形态**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test status_matrix
```

**结果：** **首次即绿**——本 task 只新建测试，断言的实现由 Task 2／Task 3 交付。
它们的红由 Task 8 的变异复核取（把 `is_advanceable` 收紧到 `Ready`、把基线过滤到 `Ready`）。

- [ ] **Step 3: 提交**

```bash
git add crates/continuum-intent-scheduler/tests/status_matrix.rs
git commit -m "test(intent-scheduler): 六个可推进状态与七个非候选状态的矩阵"
```

**结果：** 三条全绿；无生产代码改动。

---

## Task 5: Graph 的零逻辑消费入口

**Files:**
- Create: `crates/continuum-intent-scheduler/src/graph.rs`
- Create: `crates/continuum-intent-scheduler/tests/graph.rs`
- Modify: `crates/continuum-intent-scheduler/src/lib.rs`（导出 `runnable_nodes`）

**Interfaces:**
- Consumes: `continuum_graph::{AdfirGraph, NodeId, SchedulerConfig, select_runnable}`
- Produces: `continuum_intent_scheduler::runnable_nodes(&AdfirGraph, &SchedulerConfig) -> Vec<NodeId>`

- [ ] **Step 1: 写 `tests/graph.rs` 的三条用例**

```rust
use continuum_graph::{select_runnable, AdfirGraph, ContractIdRef, GraphId, SchedulerConfig};

const GRAPH_SRC: &str = include_str!("../src/graph.rs");

fn code_without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn function_body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(signature).expect("函数签名应存在");
    let open = start + source[start..].find('{').expect("函数应有函数体");
    let close = open + source[open..].find('}').expect("函数应有闭合花括号");
    &source[open + 1..close]
}

#[test]
fn an_empty_graph_yields_no_runnable_nodes() {
    let graph = AdfirGraph::new(GraphId::new("g"), ContractIdRef::new("c"));
    let config = SchedulerConfig::default();
    let nodes = continuum_intent_scheduler::runnable_nodes(&graph, &config);
    assert!(nodes.is_empty());
    assert_eq!(nodes, select_runnable(&graph, &config));
}

#[test]
fn the_graph_entry_is_a_zero_logic_transfer() {
    let code = code_without_comments(GRAPH_SRC);
    let body = function_body(&code, "pub fn runnable_nodes");
    assert_eq!(body.trim(), "continuum_graph::select_runnable(graph, config)");
    for forbidden in [
        "if ", "for ", "while ", "match ", "NodeState", "edges_to", "predecessors",
        "apply_blocking", "apply_unblocking", "transition(", "max_parallel",
    ] {
        assert!(!body.contains(forbidden), "runnable_nodes 的函数体不得包含 {forbidden:?}");
    }
}

#[test]
fn graph_source_mentions_neither_intent_nor_resource() {
    let code = code_without_comments(GRAPH_SRC);
    for forbidden in ["IntentDecision", "ResourceDirective", "DirectiveForError"] {
        assert!(!code.contains(forbidden), "src/graph.rs 不得出现 {forbidden}");
    }
}
```

**结果：** 三条用例名可列出。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test graph
```

**结果：** 编译失败（`E0432`：`continuum_intent_scheduler::runnable_nodes` 尚不存在）。**这不是红**。

- [ ] **Step 3: 实现 `src/graph.rs` 并导出**

函数体**恰一行**；README／文档注释可以说明「P7B 不判 READY」，但代码行不得出现 READY 判定：

```rust
use continuum_graph::{AdfirGraph, NodeId, SchedulerConfig};

/// §303 的 Graph Scheduler 消费入口。**零逻辑转调**：READY／并发判定全部在
/// `continuum_graph::select_runnable`。本函数不读 `NodeState`、不遍历边、不判前驱、
/// 不截断、不排序、不做状态迁移；`SchedulerConfig.max_parallel` 原样传入，本块不解释它。
pub fn runnable_nodes(graph: &AdfirGraph, config: &SchedulerConfig) -> Vec<NodeId> {
    continuum_graph::select_runnable(graph, config)
}
```

`src/lib.rs` 加 `pub mod graph;` 与 `pub use graph::runnable_nodes;`。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test graph
git add crates/continuum-intent-scheduler/src/graph.rs \
        crates/continuum-intent-scheduler/src/lib.rs \
        crates/continuum-intent-scheduler/tests/graph.rs
git commit -m "feat(intent-scheduler): Graph 的零逻辑消费入口"
```

**结果：** 三条全绿；`function_body` 取到的函数体去掉空白后恰是
`continuum_graph::select_runnable(graph, config)`。

---

## Task 6: 资源端口与 `directive_for`

**Files:**
- Create: `crates/continuum-intent-scheduler/src/resource.rs`
- Create: `crates/continuum-intent-scheduler/tests/resource.rs`
- Modify: `crates/continuum-intent-scheduler/src/lib.rs`（导出 resource）

**Interfaces:**
- Consumes: Task 1 的 `SchedulerInputs`／`Deadline`／`BackgroundSuitability`；Task 2 的
  `IntentDecision`／`is_advanceable`；`continuum_semantics::{Intent, IntentId, Priority}`；
  `continuum_graph::NodeId`
- Produces: `ExecutionRequest`／`ExecutionSignals`／`ResourceDirective`／`ResourceDirectiveError`／
  `DirectiveForError`／`ResourceDirectivePort`／`IntentScheduler::directive_for`

- [ ] **Step 1: 写 `tests/resource.rs` 的八条用例**

公共假端口（**只在 `tests/`；`src/` 零 `impl ResourceDirectivePort`**）。测试的 `use` 按实际使用裁剪，
不留未使用导入（全量门要求 0 warning）：

```rust
mod support;

use std::cell::{Cell, RefCell};

use continuum_graph::NodeId;
use continuum_intent_scheduler::{
    DirectiveForError, ExecutionRequest, IntentDecision, IntentScheduler, ResourceDirective,
    ResourceDirectiveError, ResourceDirectivePort, SchedulerInputs, UserFocus,
};
use continuum_semantics::{IntentId, IntentStatus, Priority};

struct FakePort {
    directive: ResourceDirective,
    error: Option<ResourceDirectiveError>,
    calls: Cell<usize>,
}

impl FakePort {
    fn returning(directive: ResourceDirective) -> Self {
        Self { directive, error: None, calls: Cell::new(0) }
    }

    fn failing(message: &str) -> Self {
        Self {
            directive: ResourceDirective::new(None, None, None, None),
            error: Some(ResourceDirectiveError::new(message)),
            calls: Cell::new(0),
        }
    }
}

impl ResourceDirectivePort for FakePort {
    fn direct(
        &self,
        _request: &ExecutionRequest<'_>,
    ) -> Result<ResourceDirective, ResourceDirectiveError> {
        self.calls.set(self.calls.get() + 1);
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(self.directive.clone()),
        }
    }
}

struct Seen {
    intent: String,
    node: Option<String>,
    priority: Option<String>,
    deadline: Option<i64>,
    background: String,
}

struct RecordingPort {
    seen: RefCell<Option<Seen>>,
}

impl ResourceDirectivePort for RecordingPort {
    fn direct(
        &self,
        request: &ExecutionRequest<'_>,
    ) -> Result<ResourceDirective, ResourceDirectiveError> {
        *self.seen.borrow_mut() = Some(Seen {
            intent: request.intent.id.as_str().to_owned(),
            node: request.node.map(|node| node.as_str().to_owned()),
            priority: request.signals.priority.map(|priority| priority.as_str().to_owned()),
            deadline: request.signals.deadline.map(|deadline| deadline.at_millis),
            background: request.signals.background_suitability.projection().to_owned(),
        });
        Ok(ResourceDirective::new(None, None, None, None))
    }
}
```

八条用例：

1. `no_intent_returns_none_without_calling_the_port`（**J13-a**）：
   `decision = NoIntent { basis: "none" }`；`directive_for` ⇒ `Ok(None)`，`calls() == 0`。
2. `a_legal_advance_calls_the_port_once_and_returns_four_slots_verbatim`（**J13-b**）：
   候选 `Ready`；假端口返回 `ResourceDirective::new(Some("node-a"), Some("model-x"),
   Some("tool-y"), Some("8"))`；`directive_for` ⇒ `Ok(Some(d))`，`calls() == 1`，四个访问器逐字节
   等于填入值；另断言 `None` 槽仍是 `None`（用第二个指令值）。
3. `a_port_error_is_passed_through`（**J13-c**）：假端口 `Err(ResourceDirectiveError::new("rank unavailable"))`
   ⇒ `matches!(directive_for(...), Err(DirectiveForError::Port(e)) if e.message() == "rank unavailable")`；
   `calls() == 1`。
4. `an_invalid_advance_is_rejected_without_calling_the_port`（**J13-d**，两组）：
   `Advance` 指向候选集外 `ghost` ⇒ `Err(InvalidDecision)`、`calls() == 0`；
   `Advance` 指向 `Completed` 候选 ⇒ 同样 `Err(InvalidDecision)`、`calls() == 0`。
5. `the_port_receives_first_priority_earliest_deadline_and_round_background`（**设计 §2.5 第 15 条**）：
   候选 `Ready`；`priority` 条目两条（`"first"`、`"second"`）；`deadline` 条目 `500` 与 `100`；
   `background = "bg-value"`；`node = Some(&NodeId::new("n1"))`。用 `RecordingPort` 断言：
   `priority == Some("first")`、`deadline == Some(100)`、`background == "bg-value"`、
   `intent == "ready"`、`node == Some("n1")`。
6. `resource_directive_fields_are_private_and_default_is_absent`（**J13-e**）：读 `../src/resource.rs`，
   在 `pub struct ResourceDirective {` 与随后的 `}` 之间逐行断言**没有 `pub ` 字段**；
   断言全文件没有 `impl Default for ResourceDirective`。**不建 trybuild**：设计 §2.1 没有这条 dev 边。
7. `there_is_no_production_port_implementation_or_router_placement_surface`（**J12-a／J12-b**）：
   `src/resource.rs` 与 `src/` 全体（去注释行）零 `impl ResourceDirectivePort`；
   零 `rank(`／`place(`／`RoutingRequest`／`PlacementRequest`／`ComputeNode`／`ExecutionCandidate`／
   `continuum_model_registry`／`continuum_node`。
8. `directive_for_only_transfers_and_makes_no_new_decisions`（**J10-c**）：取 `pub fn directive_for`
   的函数体，断言包含 `port.direct` 与 `NoIntent`；不包含 `continuum_graph`／`select_runnable`／
   `ResourceDirective::new`／`apply_intent_transition`／`transition(`。它不填槽、不产出新的 Intent 决策。

**结果：** 八条用例名可列出。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test resource
```

**结果：** 编译失败（`E0432`：resource 类型与 `directive_for` 尚不存在）。**这不是红**。

- [ ] **Step 3: 实现 `src/resource.rs` 并导出**

类型、槽、派生逐字照设计 §4.7／§4.8：

- `ExecutionRequest<'a> { intent: &'a Intent, node: Option<&'a NodeId>, signals: ExecutionSignals<'a> }`
- `ExecutionSignals<'a> { priority: Option<&'a Priority>, deadline: Option<&'a Deadline>,
  background_suitability: &'a BackgroundSuitability }`
- `ResourceDirective`：四个**私有** `Option<String>` 槽 + `new` + 四个只读访问器；**无 `Default`**
- `ResourceDirectiveError(String)`：`new`／`message`／`Display`／`Error`
- `ResourceDirectivePort::direct(&self, &ExecutionRequest) -> Result<ResourceDirective, ResourceDirectiveError>`
- `DirectiveForError::{InvalidDecision { intent }, Port(ResourceDirectiveError)}`，手写 `Display` 与
  `Error::source`
- `IntentScheduler::directive_for(inputs, decision, node, port)`：
  `NoIntent ⇒ Ok(None)`，**不调端口**；`Advance` 的 id 不在候选集/不可推进 ⇒ `Err(InvalidDecision)`；
  合法 ⇒ 取该候选、第一条 `PrioritySignal`、最早 `Deadline`、整轮 `background_suitability`，
  组装 `ExecutionRequest`，调一次 `port.direct(&request)` 并 `map(Some)`／`map_err(Port)`。

`src/lib.rs` 加 `pub mod resource;` 与设计 §2.2 的导出（`ResourceDirectivePort` 在内）。

- [ ] **Step 4: 运行本 task 的测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test resource
git add crates/continuum-intent-scheduler/src/resource.rs \
        crates/continuum-intent-scheduler/src/lib.rs \
        crates/continuum-intent-scheduler/tests/resource.rs
git commit -m "feat(intent-scheduler): 资源端口与 directive_for 转交"
```

**结果：** 八条全绿；`src/` 的 `impl ResourceDirectivePort` 计数为 0（假实现只在 `tests/`）。

---

## Task 7: 源码面与三层分离照片

**Files:**
- Create: `crates/continuum-intent-scheduler/tests/source_surface.rs`

**Interfaces:**
- Consumes: Task 1–6 的全部 `src/` 文件
- Produces: 无生产代码；J1-b／J3-a／J4-a／J5-a／J10-a／J14／J15 的源码面照片

**扫描口径（所有用例共用）**：

```rust
const LIB: &str = include_str!("../src/lib.rs");
const DECISION: &str = include_str!("../src/decision.rs");
const INPUTS: &str = include_str!("../src/inputs.rs");
const GRAPH: &str = include_str!("../src/graph.rs");
const RESOURCE: &str = include_str!("../src/resource.rs");

fn src_texts() -> [(&'static str, &'static str); 5] {
    [
        ("lib.rs", LIB),
        ("decision.rs", DECISION),
        ("inputs.rs", INPUTS),
        ("graph.rs", GRAPH),
        ("resource.rs", RESOURCE),
    ]
}

/// 设计 §7.2 的源码面判据允许「只命中文档注释」；这里逐行去掉 `//` 开头的注释行。
/// grep 按行匹配，跨行折行会漏；**结论仍以人工通读为准**，测试只负责把已知词根钉住。
fn code_without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取一条函数签名（从 needle 到第一个 `{`），用于 J10-a 的返回类型比较。
fn signature<'a>(source: &'a str, needle: &str) -> &'a str {
    let start = source.find(needle).expect("签名应存在");
    let end = start + source[start..].find('{').expect("签名后应有函数体");
    &source[start..end]
}
```

- [ ] **Step 1: 写九条用例**

1. `lib_rs_only_declares_modules_and_reexports`：`code_without_comments(LIB)` 不含
   `struct `／`enum `／`impl `／`fn `／`const `／`static `；只含 `pub mod`／`pub use`。
2. `the_source_tree_has_exactly_the_five_design_modules`：用
   `Path::new(env!("CARGO_MANIFEST_DIR")).join("src")` 定位目录（不依赖测试进程的 cwd），
   文件名排序后**恰是** `["decision.rs", "graph.rs", "inputs.rs", "lib.rs", "resource.rs"]`；
   断言 `lib.rs` 含 `pub mod decision;`／`pub mod graph;`／`pub mod inputs;`／`pub mod resource;`。
3. `no_empty_intent_id_sentinel_in_source`（**J1-b**）：五个文件的代码行零
   `IntentId::from_str("")`。**这一条只覆盖空串哨兵**；`" "`／`"none"` 不在射程。
4. `priority_is_not_parsed_in_source`（**J3-a**）：逐行断言不含 `intent.priority`／
   `candidate.priority`；不含同一行同时出现 `sort_by` 与 `priority`；不含同一行同时出现
   `cmp` 与 `priority`；不含 `Ord for Priority`；不含同一行同时出现 `PartialOrd` 与 `Priority`。
5. `workload_class_and_pressure_are_not_parsed_in_source`（**J4-a**）：代码行零
   `workload_class`／`resource_pressure`／`INTERACTIVE_GAME`／`COMPUTE_WORKLOAD`／`COMPILER`／
   `VIDEO_RENDER`。
6. `no_intent_storage_or_transition_surface_in_source`（**J5-a**）：代码行零
   `load_intent`／`list_children`／`save_intent`／`apply_intent_transition`／`continuum_persist`。
7. `the_three_decision_faces_have_distinct_signatures`（**J10-a**）：用 `signature(source, needle)`
   取四条签名（从 needle 到第一个 `{`，空白折叠成单空格），断言：
   - `pub fn decide` 含 `IntentDecision` 与 `IntentDecisionError`，不含 `NodeId`、`ResourceDirective`；
   - `pub fn runnable_nodes` 含 `Vec<NodeId>`，不含 `IntentDecision`、`ResourceDirective`；
   - `fn direct` 含 `ResourceDirective` 与 `ResourceDirectiveError`，不含 `IntentDecision`、`Vec<NodeId>`；
   - `pub fn directive_for` 含 `Option<ResourceDirective>` 与 `DirectiveForError`；
   - 上述任一条签名中，`IntentDecision`／`Vec<NodeId>`／`ResourceDirective` 三者**只出现一个**。
   `directive_for` 是转交入口，不参与「三个决策面」的返回类型比较（设计 §5.4）。
8. `no_model_call_surface_in_source`（**J14**）：代码行零
   `ModelProvider`／`InvokeRequest`／`continuum_provider`／`continuum_runtime`。
9. `no_resource_scheduler_type_in_source`（**J15**）：代码行零 `ResourceScheduler`。

**结果：** 九条用例名可列出。

- [ ] **Step 2: 运行，确认形态**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test source_surface
```

**结果：** **首次即绿**（前提：Task 1–6 的源码面未漂）。若红，**修拥有该代码的 task，不改弱断言**
——这些用例是设计的判据，不是实现的可选项。

- [ ] **Step 3: 提交**

```bash
git add crates/continuum-intent-scheduler/tests/source_surface.rs
git commit -m "test(intent-scheduler): 源码面与三层分离照片"
```

**结果：** 九条全绿；`src/` 仍恰五个文件。

---

## Task 8: 清单边、依赖门与零表收尾

**Files:**
- Create: `crates/continuum-intent-scheduler/tests/manifest.rs`

**Interfaces:**
- Consumes: `crates/continuum-intent-scheduler/Cargo.toml`；`continuum-runtime` 的依赖门
- Produces: 无生产代码；J16／J5-b／J4-c 的收尾照片与变异复核

- [ ] **Step 1: 写 `tests/manifest.rs` 的两条用例**

```rust
const MANIFEST: &str = include_str!("../Cargo.toml");

fn keys_in(manifest: &str, section: &str) -> Vec<String> {
    let mut in_section = false;
    let mut keys = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section;
            continue;
        }
        if !in_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            keys.push(key.trim().to_owned());
        }
    }
    keys.sort();
    keys
}

#[test]
fn the_manifest_declares_exactly_three_allowed_edges() {
    assert_eq!(
        keys_in(MANIFEST, "[dependencies]"),
        vec!["continuum-graph", "continuum-semantics"]
    );
    assert_eq!(
        keys_in(MANIFEST, "[dev-dependencies]"),
        vec!["continuum-canonical"]
    );
}

#[test]
fn the_manifest_declares_no_persistence_resource_or_runtime_edge() {
    let all = [keys_in(MANIFEST, "[dependencies]"), keys_in(MANIFEST, "[dev-dependencies]")].concat();
    for forbidden in [
        "continuum-arbiter", "continuum-events", "continuum-model-registry", "continuum-node",
        "continuum-persist", "continuum-product", "continuum-provider", "continuum-runtime",
        "continuum-supervisor", "serde", "serde_json", "thiserror", "trybuild",
    ] {
        assert!(!all.iter().any(|key| key == forbidden), "Cargo.toml 不得登记 {forbidden}");
    }
}
```

**结果：** 两条用例名可列出；`[dependencies]` 恰两键、`[dev-dependencies]` 恰一键。

- [ ] **Step 2: 运行，确认形态**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-intent-scheduler --test manifest
```

**结果：** **首次即绿**——清单在 Task 1 已定型；若红，说明 Task 1 的依赖边与设计 §2.1 不符，回查。

- [ ] **Step 3: 跑依赖门与零表检查（J16／J5-b／J4-c）**

```bash
TMPDIR="$PWD/.tmp" timeout 1800 cargo test -p continuum-runtime --test dependency_direction
git diff --stat c89d839 -- crates/continuum-runtime/src/main.rs \
    crates/continuum-runtime/tests/migrations.rs crates/continuum-runtime/tests/startup.rs
grep -rn "continuum-intent-scheduler" crates/continuum-runtime/src/main.rs \
    crates/continuum-runtime/tests/migrations.rs crates/continuum-runtime/tests/startup.rs
TMPDIR="$PWD/.tmp" timeout 1200 cargo test -p continuum-runtime --test migrations --test startup
```

**结果：**
- 依赖门绿；`ALLOWED` 的 `continuum-intent-scheduler` 行**恰三项**
  （`continuum-canonical`／`continuum-graph`／`continuum-semantics`），既有行不出现本 crate 名。
- `git diff --stat c89d839` 对三个迁移相关文件输出为空；`grep` 零命中。
- `migrations` 与 `startup` 全绿，计数不变。

- [ ] **Step 4: 全量门**

```bash
TMPDIR="$PWD/.tmp" timeout 2400 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 1800 cargo build --workspace --all-targets
```

**结果：** 全绿，**0 warning**；全量日志里的 `Running` 与 `test result:` 行覆盖新增的八个测试目标。

- [ ] **Step 5: 变异复核（承重守卫，六轮）**

每轮：备份被改文件 → `sha256sum` 记录 → 改一处 → 跑指定测试读红位 → 还原 → `sha256sum` 与备份比对
一致。**每轮独立日志路径**（`.tmp/p7b-mutations/<编号>.log`）；**不得用 `git checkout --` 顺手还原**
（会连带未提交改动）。**等价变异体不算守卫**（注释文字、派生顺序、测试期望值书写顺序）。

| # | 变异 | 该红 | 不该红 |
|---|---|---|---|
| 1 | `is_advanceable` 只留 `IntentStatus::Ready` | Task 4 `six_advanceable_statuses_pass_the_decide_check`、`baseline_selects_each_active_status_under_user_focus` | Task 3 的 Ready-only 用例 |
| 2 | `decide` 跳过候选校验，直接返回策略的 `Advance` | Task 2 `an_advance_outside_the_candidate_set_is_rejected`、`an_advance_to_a_completed_candidate_is_rejected` | Task 2 `a_legal_advance_is_returned_unchanged` |
| 3 | `decide` 把策略 `Err` 折成 `NoIntent`（`unwrap_or(NoIntent…)`） | Task 2 `a_policy_error_is_policy_failed_and_keeps_the_basis` | Task 2 `an_empty_no_intent_basis_is_normalized_to_no_intent` |
| 4 | `directive_for` 的非法 `Advance` 分支改成 `Ok(None)` | Task 6 `an_invalid_advance_is_rejected_without_calling_the_port` | Task 6 `no_intent_returns_none_without_calling_the_port` |
| 5 | 基线只把 `Ready` 放进 `eligible` | Task 4 `baseline_selects_each_active_status_under_user_focus` | Task 3 `candidate_order_is_the_last_tiebreak` |
| 6 | 基线在 focus 之前按 `priority.as_str()` 升序排序 `eligible` | Task 3 `priority_and_background_suitability_do_not_change_the_decision` | Task 3 的 `user_focus_beats_deadline_and_blocking`（focus 仍应命中） |

**结果：** 六轮的「该红」逐条实测为红，「不该红」保持绿；六个文件的 sha256 前后一致；
`.tmp/p7b-mutations/` 的日志保留到复审结束。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-intent-scheduler/tests/manifest.rs
git commit -m "test(intent-scheduler): 清单边、依赖门与零表收尾"
```

**结果：** 只提交显式路径；`git status` 干净（`.tmp/` 不入库）。**若变异复核暴露了实现缺陷，
修复提交回到拥有该代码的 task，不在本 task 里顺手改语义。**

---

## 自审

### 一、Spec coverage（设计节 → task）

| 设计节 | 内容 | 落在哪个 task |
|---|---|---|
| §1.1／§1.2 | 建什么／不建什么 | 全部 task 的 Files 与 Global Constraints |
| §2.1 | crate 名、三条边、共写文件 | Task 1（`Cargo.toml`＋`ALLOWED`）＋Task 8 Step 1／3 |
| §2.2 | 五个源文件与「不新增类型」 | Task 1（inputs）／Task 2（decision）／Task 5（graph）／Task 6（resource）＋Task 7 的模块清单 |
| §2.3 | 同名异物（priority／workload／Decision／Suspended／NodeId／Blocking／Deadline／NoIntent） | Task 1（类型名）＋Task 7（源码面）＋Review Focus 2 |
| §2.4 | 层间位置与不登记执行层边 | Task 1（边表）＋Task 8（清单／依赖门） |
| §2.5 | 十六条发明判据 | Task 3（第 6 条基线次序）／Task 6（第 9–12、15–16 条）／Task 7（第 3、7 条源码面）／「遗留」 |
| §3.1–§3.8 | 规范依据逐条 | Task 2（§302）／Task 3（§78 缺口）／Task 4（§223 状态）／Task 5（§303）／Task 6（§304） |
| §4.1 | `IntentDecision`／`DecisionBasis`／`IntentDecisionError` | Task 2 |
| §4.2 | `SchedulerInputs` 与五输入形状 | Task 1 |
| §4.3 | `BackgroundSuitability` | Task 1（载体）＋Task 4（基线不读） |
| §4.4 | `IntentSelectionPolicy` 与基线五步 | Task 2（trait）＋Task 3（基线） |
| §4.5 | `IntentScheduler::decide` 与校验 | Task 2 |
| §4.6 | `graph::runnable_nodes` | Task 5 |
| §4.7 | 端口、执行请求、四槽指令 | Task 6 |
| §4.8 | `directive_for` 与错误面 | Task 6 |
| §4.9 | 与 §302／§77 的对齐 | Task 1／Task 5／Task 6／Task 7 |
| §5.1 | P7D 的消费契约 | **不在本块代码内**；Task 2／4 的测试注释钉住边界，契约留给 P7D（Review Focus 2／5） |
| §5.2 | 装配方接线 | **不在本块代码内**；Task 6 的假端口只证接口；实现方归 P7C／装配方（遗留第 3 条） |
| §5.3 | `NoIntent` 三种来源与硬约束 | Task 2（规范化）＋Task 3（空候选）＋Task 4（七负例）＋Task 7（空串哨兵） |
| §5.4 | 三层分离的结构照片 | Task 5（J10-b）＋Task 6（J10-c）＋Task 7（J10-a） |
| §6 | 跨块接缝 | Task 1（零 P7C／P7D 边）＋「跨计划前置」 |
| §7.1／§7.2 | J1–J16 与照片 | 下面的「照片归位」表 |
| §7.3 | 变异预告 | Task 8 Step 5（六条承重变异） |
| §7.4 | 拍不出照片的地方 | 「遗留/待协调」逐条列，不冒充覆盖 |
| §8 | 明确不做 | Global Constraints 末段；Task 7 的源码面 |
| §9 | 零表零迁移零事件 | Task 8 Step 3／4；Global Constraints |
| §10 | 十二条设计问题 | 「遗留/待协调」只列与本块实现直接相关的四条＋本计划新查出的夹具限制 |
| §11 | 实测与口径订正 | 本计划引用其结论，不重述 |
| §12 | 验收口径 | 自审「照片归位」＋Task 8 全量门 |

**照片归位（J1–J16）**：

| 判据 | 照片（用例／命令） | task |
|---|---|---|
| **J1-a** | `decision_has_exactly_two_arms_and_no_intent_has_no_identifier`（无通配穷尽解构） | Task 2 |
| **J1-b** | `no_empty_intent_id_sentinel_in_source` | Task 7 |
| **J1-c** | `empty_candidates_yield_no_intent`（与 J6 同一张照片） | Task 3 |
| **J2** | `scheduler_inputs_carries_candidates_and_the_five_302_inputs`（无 `..` 的穷尽解构） | Task 1 |
| **J3-a** | `priority_is_not_parsed_in_source` | Task 7 |
| **J3-b** | `priority_and_background_suitability_do_not_change_the_decision`（互换 `PrioritySignal`） | Task 3 |
| **J3-c** | `Priority` 无 `Ord`／`PartialOrd`（`intent.rs:288-290`）；**可选样例不建**（设计 §7.2 自注可选；不引入 `trybuild`） | 类型层，无本块用例 |
| **J4-a** | `workload_class_and_pressure_are_not_parsed_in_source` | Task 7 |
| **J4-b** | `background_suitability_round_trips_projection_verbatim` | Task 1 |
| **J4-c** | `the_manifest_declares_no_persistence_resource_or_runtime_edge`＋依赖门 | Task 8 |
| **J5-a** | `no_intent_storage_or_transition_surface_in_source` | Task 7 |
| **J5-b** | Task 8 Step 3 的 `git diff c89d839` 为空＋`migrations`／`startup` 绿 | Task 8 |
| **J6** | `empty_candidates_yield_no_intent` | Task 3 |
| **J7** | **七负例** `seven_non_candidate_statuses_yield_no_intent`；**六正例** `six_advanceable_statuses_pass_the_decide_check`（两个独立函数） | Task 4 |
| **J7-b** | `baseline_selects_each_active_status_under_user_focus`（五个活跃状态逐组；与 J7 分开） | Task 4 |
| **J8-a** | `an_advance_outside_the_candidate_set_is_rejected` | Task 2 |
| **J8-b** | `an_advance_to_a_completed_candidate_is_rejected` | Task 2 |
| **J8-c** | `a_legal_advance_is_returned_unchanged` | Task 2 |
| **J8-d** | `a_policy_error_is_policy_failed_and_keeps_the_basis` | Task 2 |
| **J8-e** | `an_empty_no_intent_basis_is_normalized_to_no_intent` | Task 2 |
| **J9** | 七项分列：`user_focus_beats_deadline_and_blocking`／`the_earliest_deadline_wins_and_multiple_entries_take_the_minimum`／`the_largest_deduped_blocking_set_wins`／`candidate_order_is_the_last_tiebreak`／`priority_and_background_suitability_do_not_change_the_decision`／`signals_pointing_outside_the_candidates_are_ignored`／`duplicate_candidate_ids_are_not_deduplicated`＋`the_same_inputs_yield_the_same_decision` | Task 3 |
| **J10-a** | `the_three_decision_faces_have_distinct_signatures` | Task 7 |
| **J10-b** | `graph_source_mentions_neither_intent_nor_resource` | Task 5 |
| **J10-c** | `directive_for_only_transfers_and_makes_no_new_decisions` | Task 6 |
| **J11-a** | `the_graph_entry_is_a_zero_logic_transfer`（函数体恰一行） | Task 5 |
| **J11-b** | 同一用例的禁止词根表 | Task 5 |
| **J12-a** | `there_is_no_production_port_implementation_or_router_placement_surface` | Task 6 |
| **J12-b** | 同一用例的 `rank`／`place`／第 4 层类型表 | Task 6 |
| **J13-a** | `no_intent_returns_none_without_calling_the_port` | Task 6 |
| **J13-b** | `a_legal_advance_calls_the_port_once_and_returns_four_slots_verbatim` | Task 6 |
| **J13-c** | `a_port_error_is_passed_through` | Task 6 |
| **J13-d** | `an_invalid_advance_is_rejected_without_calling_the_port` | Task 6 |
| **J13-e** | `resource_directive_fields_are_private_and_default_is_absent` | Task 6 |
| **J14** | `no_model_call_surface_in_source`＋清单门 | Task 7／Task 8 |
| **J15** | `no_resource_scheduler_type_in_source` | Task 7 |
| **J16** | Task 8 Step 3 的 `dependency_direction`；`ALLOWED` 行恰三项 | Task 8 |

### 二、Step scan

- 每个 Step 是**一个动作 + 一个可核对结果**：写用例／跑失败形态／实现或建骨架／跑测试并提交；
  test-only task（Task 4／7／8）的 Step 2 明写「首次即绿」及其理由，不伪装成红。
- **「运行，确认失败」逐 task 标注量到什么**：Task 1 是 package 未找到（不是编译失败）；
  Task 2／3／5／6 是 `E0432`（不算红）；Task 4／7／8 首次即绿。所有「首次即绿」的红由
  Task 8 的六条承重变异取。
- **测试命令精确到 `--test` 目标**：`inputs`／`decision`／`baseline`／`status_matrix`／`graph`／
  `resource`／`source_surface`／`manifest`。
- **没有一步同时改两个关注点**：Task 1 只做骨架与输入；Task 2 只做决策类型与 `decide`；
  Task 3 只做基线；Task 4 只做状态矩阵；Task 5 只做 Graph 转调；Task 6 只做端口与转交；
  Task 7 只做源码面；Task 8 只做清单与收尾。
- **提交只用显式路径**；`Cargo.lock` 只在 Task 1 随新 crate 提交。

### 三、Type consistency

- `IntentDecision`：两臂 `Advance { intent: IntentId, basis: DecisionBasis }` /
  `NoIntent { basis: DecisionBasis }`；`advanced()`／`basis()`／`is_no_intent()`；**无 `PartialEq`**。
- `DecisionBasis`：私有 `String`；`new`／`as_str`；`PartialEq, Eq`。
- `IntentDecisionError`：`PolicyFailed { basis }`／`InvalidAdvance { intent }`；手写 `Display`／`Error`。
- `IntentSelectionPolicy::choose(&self, &SchedulerInputs<'_>) -> Result<IntentDecision, DecisionBasis>`；
  `BaselineIntentPolicy` 实现它；`decide(inputs, &dyn IntentSelectionPolicy)`。
- `SchedulerInputs<'a>`：六字段逐字；`candidates: &'a [Intent]`；`user_focus: UserFocus`；
  `background_suitability: BackgroundSuitability`；**无 `Default`／`Clone`**。
- `PrioritySignal { intent, priority }`／`Deadline { intent, at_millis: i64 }`／
  `Blocking { intent, blocks: Vec<IntentId> }`／`UserFocus::{None, Intent, Area}`；
  `BackgroundSuitability::from_projection`／`projection`。
- `runnable_nodes(&AdfirGraph, &SchedulerConfig) -> Vec<NodeId>`；函数体恰一行。
- `ExecutionRequest<'a> { intent: &'a Intent, node: Option<&'a NodeId>, signals: ExecutionSignals<'a> }`；
  `ExecutionSignals<'a> { priority: Option<&'a Priority>, deadline: Option<&'a Deadline>,
  background_suitability: &'a BackgroundSuitability }`。
- `ResourceDirective::new(Option<String>, Option<String>, Option<String>, Option<String>)` 与四个访问器；
  `ResourceDirectiveError::new`／`message`；`ResourceDirectivePort::direct(&self, &ExecutionRequest<'_>)`。
- `DirectiveForError::{InvalidDecision { intent }, Port(ResourceDirectiveError)}`；
  `directive_for(inputs, &IntentDecision, Option<&NodeId>, &dyn ResourceDirectivePort) ->
  Result<Option<ResourceDirective>, DirectiveForError>`。
- **没有自造** `Intent`／`IntentId`／`Priority`／`IntentStatus`／`AdfirGraph`／`NodeId`／
  `SchedulerConfig`／`RoutingRequest`／`rank`／`place`／`ResourceScheduler`／`Decision`（设计 §2.2）。

### 四、Review Focus 归位

| Review Focus | 归属 task | 具体用例 |
|---|---|---|
| 1. `WAITING`／`SUSPENDED` 被基线丢掉 | Task 4 | `baseline_selects_each_active_status_under_user_focus` |
| 2. `NoIntent` 被当无会话／哨兵 | Task 2／Task 3／Task 7 | `decision_has_exactly_two_arms_and_no_intent_has_no_identifier`、`empty_candidates_yield_no_intent`、`no_empty_intent_id_sentinel_in_source` |
| 3. 策略失败／越界被吞成 `NoIntent` | Task 2／Task 6 | `an_advance_outside_the_candidate_set_is_rejected`、`an_advance_to_a_completed_candidate_is_rejected`、`a_policy_error_is_policy_failed_and_keeps_the_basis`、`an_invalid_advance_is_rejected_without_calling_the_port` |
| 4. `COMPLETED` 被当可推进 | Task 4／Task 2 | `seven_non_candidate_statuses_yield_no_intent`、`an_advance_to_a_completed_candidate_is_rejected` |
| 5. `EVOLVING` 无会话仍被推进 | Task 4 | `six_advanceable_statuses_pass_the_decide_check`、`baseline_selects_each_active_status_under_user_focus`（`Evolving` 组；注释明写 P7D 过滤责任） |

### 五、比例

- 设计 1451 行；本计划 **1655 行**（`wc -l` 实测）。任务 **8 个**；
  J1–J16 每条至少有一张照片（J3-c 设计自注可选，本计划不建；J7 的七负例／六正例／J7-b 三个函数分开；
  J10 的 a／b／c 分在 Task 7／5／6）。
- 生产代码集中在 `src/` 的五个文件；测试量大于生产量是刻意的——本块的判据几乎全是
  「形状＋状态边界＋否定面＋源码面」。
- 没有把设计 §10 的十二条全部搬成 task：与本块实现直接相关的四条进「遗留/待协调」，
  其余照原文留在设计里（重述就是第二份转录）。

---

## 遗留/待协调

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**

设计 §10 的十二条**逐条仍有效**；下面只列与本块实现直接相关的四条，以及本计划新查出的夹具限制。

1. **§302 五输入的组合函数／权重未定**（**规范未给判据**）。§302 只给五个名字；`BaselineIntentPolicy`
   的五步次序是**本设计自定**，不是规范排名；`priority` 不透明，基线不读它，§78 的「显式 priority
   最高语义权重」**不由默认基线兑现**，且不能用候选序补救。**收件人：规范维护者 ＋ 协调者 ＋ P7D。**
   代价：不同装配方的排名不可比；P7D 必须复用本策略或注入解释 `Priority` 的策略（设计 §3.6、§4.4、
   §10 第 1、2 条）。
2. **`BackgroundSuitability` 的投影取值域未定**（**规范未给判据**）。本块只冻 `String` 载体，
   **整轮单值**；不定义 `Suitable`／`Unsuitable`，不读 `workload_class`／`resource_pressure`。
   装配方是否把四档原文塞进载体，P7B 拦不住。**收件人：P7C ＋ 装配方 ＋ 规范维护者。**
   代价：基线不能使用该信号（设计 §4.3、§10 第 3 条）。
3. **端口消费方在装配方**（**实现不在本块**）。`ResourceDirectivePort` 的实现要持有 P7C 的
   `resource_pressure`／`workload_class` 与 §300 四档，并转调 `rank`／`place`／工具路径；
   本块的测试只用假端口。**收件人：P7C ＋ 装配方。** 若 P7C 要求按请求传入结论，才增加不透明
   `ResourceContext` 参数；本轮不加（设计 §4.7、§10 第 4 条）。
4. **§7.4 不覆盖 P7B**（**规范未给判据**）。四条判据分别落在 P7A（1、2、4）与 P7C（3）；
   P7B 没有规范判据，本设计自报 J10 作为完成判据，**不修改 §7.4**。**收件人：协调者 ＋ 《工程》维护者。**
   代价：P7B 的验收依赖自报判据，跨文档不一致时没有规范锚（设计 §3.7、§10 第 5 条）。
5. **非空 `AdfirGraph` 夹具需要本块未获准的 dev 边**（**本计划新查出**）。`Node::new` 需要
   `continuum_operator::{OperatorId, OperatorVersion}`，`add_port` 需要 `continuum_port::Port`；
   设计 §2.1 只允许 `continuum-canonical` 一条 dev 边。因此 Task 5 的运行期图夹具只建空图，
   J11 的主证据是「函数体恰一行转调」的源码面。**收件人：设计持有方 ＋ 协调者。**
   若要在运行期覆盖非空图的 READY／截断行为，需先改设计 §2.1 的 dev 边表，再补
   `continuum-operator`／`continuum-port` 两条 dev 边；本轮不擅自加。
6. **`EVOLVING` 的会话存在性不在 P7B**（**契约边界，不是缺陷**）。`is_advanceable` 只判状态；
   P7D 必须只在确有对应活跃会话时才把 `Evolving` 放进候选集，否则不得以 `Advance` 驱动其迁移。
   **收件人：P7D。** 本块的 Task 4 用测试注释钉住这条边界（设计 §2.5 第 3 条、§5.1）。
7. **`IntentId`／`Priority` 没有 `PartialEq`／`Ord`**（**既有类型的表达力**）。测试一律经
   `as_str()` 比较；本块不擅自给既有类型加比较能力。**收件人：P4 语义层持有方。**
   若 P4 将来给 `Priority` 可比较取值域，J3-b／J3-c 的照片要同步调整（设计 §11 第 3 条）。
8. **J13-e 的编译期半边没有 `trybuild`**（**本计划新查出**）。设计 §7.2 的 J13-e 写「编译期＋源码面」，
   但设计 §2.1 的 dev 边只有 `continuum-canonical`；要拍「crate 外写 `ResourceDirective { .. }` 编不过」
   需 `trybuild` 或一个独立编译失败样例，两者都不在本块的允许清单里。本计划用源码面
   （字段无 `pub`、无 `impl Default`）替代编译期半边。**收件人：设计持有方 ＋ 协调者。**
   若协调者要求补编译期样例，先改设计 §2.1 的 dev 边表，再补 `trybuild`。

---

**计划行数核对：** 写完后以 `wc -l` 复核；若与本自审的「1000 行左右」偏差超过 20%，在提交前
改「比例」一节，不改正文结构。
