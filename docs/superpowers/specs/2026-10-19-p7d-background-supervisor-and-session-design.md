# P7D Background Supervisor + Session Orchestrator — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-18-p7-scope-and-split.md` §五 P7D 行的原话，逐字照用）：

> 建后台治理与会话启动：§299 四类来源的 Background Supervisor、Manual／Auto 的 Session Orchestrator、
> §171 触发点、`product drive` 的按值消费

**本块拥有 / 产出的接口**（同行的第二栏）：新建 `continuum-supervisor`；`TaskSource` 四值与
`AuthorizedTask` 输入面；`SessionPlan`；`continuum-memory` 触发边；Supervisor 结果摘要面；
P6D 合并门复核。**消费** P7B 的 `IntentScheduler`／`IntentDecision`／`SchedulerInputs`／
`IntentSelectionPolicy`，P7C 的 `ResourceArbiter`／`ResourceDecision`／`SuspensionState`／
`WorkloadDeclaration`，P4 的 `Intent`／`IntentStatus`／`ContractRef`，P6C 的
`SynthesisRequest`／`SynthesisProposal`／`SynthesisReport`／`MemoryError`。
**不消费** P7A 的 `continuum-product`：`product drive` 只按值收 `READY` Product Intent
（切分 §三第 6 条）。

**规范依据**：§299（`docs/spec/05-normative.md:1892-1903`）、§12（`docs/spec/01-concepts.md:624-665`）、
§11（`docs/spec/01-concepts.md:576-620`）、§171（`docs/spec/03-product-drive.md:905-938`）、
§156（`docs/spec/03-product-drive.md:377-406`）；《工程》§7.1（`docs/02-工程.md:428`、`:430`）、
§7.2（`:443`）、§7.3（`:453-454`）；§7.4 的四条判据**不覆盖 P7D**（切分 §七第 12 条）。
P6D 合并门的原约定：P6D 设计 §7.3.5（`docs/superpowers/specs/2026-10-17-p6d-runtime-learning-and-protected-core-design.md:1044-1125`）。

**前置事实**：

- P7B 已定稿（`docs/superpowers/specs/2026-10-18-p7b-intent-scheduler-design.md`），本设计对着其 §4.1
  的 `IntentDecision`（`:500`）、§4.2 的 `SchedulerInputs`（`:572`）、§4.4 的 `IntentSelectionPolicy`
  （`:694`）与 §4.5 的 `decide`（`:773`）写。
- P7C 已定稿（`docs/superpowers/specs/2026-10-19-p7c-resource-arbiter-design.md`），本设计对着其 §4.7
  的 `ResourceAction`（`:642`）、§4.8 的 `SuspensionState`（`:685`）、§4.10 的 `ResourceDecision`
  （`:759`）与 §4.11 的 `ResourceArbiter::direct`（`:827`）写。
- P7A 已定稿（`docs/superpowers/specs/2026-10-18-p7a-product-loop-design.md`），`product_intent_for_handoff`
  只返回 `status == READY` 的 Product Intent（`:1435-1454`）；本设计只按值消费，不登记
  `continuum-product` 边。
- P6C 的 `begin_synthesis`／`fail_synthesis`／`apply_synthesis` 在
  `crates/continuum-memory/src/synthesis.rs:108`／`:217`／`:306`；`SynthesisTrigger` 四时机在 `:64`。

**本文件只写 P7D。** P7A／P7B／P7C 的内部一律不写；凡不得不假设别的块的形状之处，一律在 §6 与 §10
写成**具名的对账条目**，不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（`docs/02-工程.md:561-567`）。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p7-plan`，基线 `c89d839`）上当场
`grep -n`／`sed -n` 实测的，未照抄任何一份别的文档。切分文档与 P7B／P7C 设计里已漂移的行号在 §11
逐条记出。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| `TaskSource` 恰四值：`authorized Intent`／`policy`／`maintenance schedule`／`product drive` | §299 `:1894-1903`；切分 §2.3.1 第 2 条 | §4.2 | 闭集四变体；无第五变体；无 `parse` |
| `AuthorizedTask`：唯一任务输入面；含 `source` 与授权证据；构造器私有 | 切分 §2.3.1 第 2 条 `:134-139`；§五 P7D 完成判据 1 | §4.3 | 四个具名构造器；裸 `TaskSource` 不能构造 |
| `product drive` 的授权证据：`kind == Product`＋`contract_id`＋`status == READY` | 切分 §三第 6 条 `:213-237`；§五 P7D 完成判据 1 | §4.3、§5.2 | 十三态表；直接构造的 `RUNNING` 负例（状态机依据见 §3.7） |
| Background Supervisor：`next_tasks` 与 `decide` | §299；《工程》§7.3 `:453` | §4.4 | `next_tasks` 不排序；`decide` 只转调 P7B |
| Session Orchestrator：Manual／Auto，共用 `SessionPlan` | §11 `:576-620`；《工程》§7.1 `:430`、§7.3 `:454` | §4.5、§4.6 | 同一 `SessionPlan` 类型；Manual 不选、Auto 选 |
| 后台新结果的摘要面 | 切分 §2.3.1 第 3 条 `:142`；§11 `:606`；§156 `:395-404` | §4.5 | `BackgroundDigest`；不承载 `Evidence` |
| §171 的触发点（不持阈值） | §171 `:931-938`；切分 §6.2 第 10 条 | §4.8 | `SynthesisIo` 端口＋三个转发；无频率／重试／阈值 |
| 消费 P7C 的资源结论与挂起状态 | P7C §7.1 `:1182-1201`；切分 §2.3.2 第 2 条 | §4.7 | P7D 持每任务 `SuspensionState` 内存表；`admit` 返回 `ResourceDecision`；`checkpoint` 三出口 |
| §156 的「后台继续低风险 Milestone」 | §156 `:383-393`；切分 §三第 6 条 `:216-218` | §5.1、§4.4 | 只按值收 `READY` Product Intent；**不判「低风险」**（§10 第 8 条） |
| P6D 合并门：rebaseline 或退役 | 切分 §四第 1 条 `:253-264`；P6D 设计 §7.3.5 | §5.7、§9.2 | **选 rebaseline**；31 行 fixture＋四个 P7 行 |

## 1.2 不建什么（摘要；完整表在 §8）

| 不建的东西 | 归属 / 理由 |
|---|---|
| Intent Scheduler 本体、§302 五输入的排序算法 | P7B；本块只调 `IntentScheduler::decide`（`:773`），不实现 `IntentSelectionPolicy` |
| Resource Arbiter、压力／负载类别／§300 四档 | P7C；本块只消费 `ResourceDecision`／`SuspensionState`，不读 `PriorityTier`／`AssemblyView` |
| §171 的阈值、频率、重试、空闲判定 | P10（切分 §6.2 第 10 条）；本块只给触发点 |
| 记忆合成作业本体、proposal 生成、七步算法 | P6C；本块只转发 `begin/apply/fail` |
| `intent` 表、第二套 Intent 查询、Intent 状态迁移 | P4；本块以 `&[Intent]` 为输入，不直读表、不调用 `apply_intent_transition` |
| Product 记录、Milestone、ProductReadiness、Shippable | P7A；本块只按值收 `READY` Product Intent |
| 执行器、进程管理、UI、Planner Agent 本体 | P1／P8／前端；本块只产出 `SessionPlan` 与控制值 |
| 模型调用、`InvokeRequest`、`ModelProvider::invoke` | P3G；本块不依赖 `continuum-provider`／`continuum-runtime` |
| 第二个「最值得继续」排名 | 切分 §五 P7D 完成判据 2；本块只复用 P7B |
| 第五类任务来源（含「模型建议」） | §299 `:1903`；`TaskSource` 闭集四值 |
| 表、迁移、事件 | 本块**零表**（§9）；不占 201 |
| 「§7.4 已覆盖 P7D」的宣称 | §7.4 四条落在 A／A／C／A；本块自报 J1–J3 等（§7） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：**`continuum-supervisor`**。

**名字的理由**：

1. `Background Supervisor` 是 §299 的唯一词根；切分 §1.2 建议名即此，设计采用，不另造同义词。
2. Session Orchestrator 与 Supervisor **同 crate**（切分 §1.2 `:57-59`）；两者都消费「现在推进哪个
   Intent」，分开会各写一套排名。
3. 不与其余三块争名：`continuum-product` 归 P7A、`continuum-intent-scheduler` 归 P7B、
   `continuum-arbiter` 归 P7C。
4. **实测未占用**：`grep -rn "continuum-supervisor\|continuum_supervisor" crates/ Cargo.toml`
   在本工作树**零命中**（2026-10-19 实测；命中只在切分文档的建议名里，不在代码里）。

依赖边表（**只登记实际用到的**，切分 §四第 5 条）：

| 被依赖 | 用到什么（逐项实测／冻结） | 边别 |
|---|---|---|
| `continuum-intent-scheduler` | `IntentScheduler`（P7B §4.5 `:763`）、`SchedulerInputs`（§4.2 `:572`）、`IntentDecision`（§4.1 `:500`）、`IntentDecisionError`（§4.1 `:535`）、`IntentSelectionPolicy`（§4.4 `:694`）、`DecisionBasis`（§4.1 `:507`） | 普通 |
| `continuum-arbiter` | `ResourceArbiter`（P7C §4.11 `:801`）、`ResourceDecision`（§4.10 `:759`）、`SuspensionState`（§4.8 `:685`）、`SuspensionPhase`（§4.8 `:666`）、`SuspensionError`（§4.9 `:735`）、`ResourceAction`（§4.7 `:642`）、`WorkloadDeclaration`（§4.5 `:584`） | 普通 |
| `continuum-semantics` | `Intent`（`crates/continuum-semantics/src/intent.rs:434`）、`IntentId`（`crates/continuum-semantics/src/ids.rs:41`）、`IntentKind`（`intent.rs:36`）、`IntentStatus`（`intent.rs:61`）、`ContractRef`（`intent.rs:331`） | 普通 |
| `continuum-memory` | `SynthesisRequest`／`SynthesisJobId`／`SynthesisProposal`／`SynthesisReport`／`MemoryError`（`crates/continuum-memory/src/lib.rs:28-31`、`src/error.rs:8`）；`SynthesisTrigger` 只经 `SynthesisRequest.trigger` 传递，本 crate **不重定义、不解析** | 普通 |
| `continuum-canonical` | `EntityId`（`crates/continuum-canonical/src/candidate.rs:62`）、`Opaque`（`crates/continuum-canonical/src/lib.rs:152`）；仅 `tests/` 构造 `Intent` 夹具 | **dev** |

**为什么没有 `continuum-persist`**：本块零表（§9），且 §171 的触发点做成
`SynthesisIo` 端口（§4.8）——事务句柄留在装配方，P7D 不命名 `Tx<'_>`。这与切分 §三 P7D 行的
条件边「仅当会话／后台作业落库」一致。**评审已裁定选端口**（2026-10-19）：切分 §2.3.1 第 4 条
已同步为「P7D 定义调用面，装配方持 `Tx`；何时调用归 P10／调用方」；本设计不再把它记作偏差
（§10 第 4 条、§11 第 7 条）。

**不登记**的边，逐条给理由（零使用的边即假边）：

- **不登记 `continuum-product`**：`product drive` 只按值收 `READY` Product Intent（切分 §三第 6 条
  `:213-218`）；本 crate 不读 `Product`／`ProductReadiness`／`Milestone`／`ProductDriveState`。
- **不登记 `continuum-persist`**：零表；§171 的事务句柄在 `SynthesisIo` 实现方（装配方）。
- **不登记 `continuum-verify`**：后台发现以**不透明摘要文本**进入 `BackgroundDigest`，本块不定义
  `Evidence`、不判 `CompletionVerdict`。若后台发现要生成 Intent，必须先按 §89 转成 `Evidence`
  （`docs/spec/02-positioning.md:939-945`），再由 P5a／装配方转成 Intent；P7D 不做这一步。
- **不登记 `continuum-policy`／`continuum-capability`**：`PolicyGrantRef` 是不透明串；授权判断留在
  第 5 层，P7D 只校验「存在且来源匹配」（切分 §2.3.1 第 2 条 `:137-139`）。
- **不登记 `continuum-runtime`／`continuum-provider`／`continuum-model-registry`／`continuum-node`／
  `continuum-graph`**：本块不执行、不调模型、不放置 Node、不写 Graph。`IntentScheduler::decide`
  是 P7B 的入口；P7D 不调用 `select_runnable`。
- **不登记 `continuum-events`**：零事件。

**dev 边：一条**（`continuum-canonical`，仅 `tests/`）。测试用本 crate、四个普通依赖、这一条
dev 边与 `trybuild`；假 `IntentSelectionPolicy`、假 `SynthesisIo`、假 `CheckpointPort` 由本 crate
的 `tests/` 自己写。`trybuild` 与 `serde_json` 是外部 dev 依赖，不进 `ALLOWED`（同 P6A §2.1 的
口径）；`serde_json` 只用于 J11 的 `Deserialize` 负例（§7.2），在
`crates/continuum-supervisor/Cargo.toml` 的 `[dev-dependencies]` 声明。

**共写文件**：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members`（`:3`） | 加一行 `crates/continuum-supervisor` | 本块只加自己这一行 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-supervisor", &["continuum-arbiter", "continuum-canonical", "continuum-intent-scheduler", "continuum-memory", "continuum-semantics"])`——**五项**（四普通＋一 dev），与上表逐行对应；数组按字母序 | 新 crate 会先让 `every_crate_depends_only_on_its_allowed_set`（`:494`）变红再被补齐，这是刻意的 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 P6D 合并门（`:551`、`:574`、`:582`、`:615`、`:784`、`:790`、`:799`） | **rebaseline**：`p6_pre_allowed.txt` → `p7_pre_allowed.txt`；`P6_EXPECTED_ROWS` → `P7_EXPECTED_ROWS`（四行）；两个 `27` → `31`；断言 2 的 `5` → `4`；测试改名 | 这是 P7D 独占的改动（切分 §一 1.2／1.3 `:55`／`:72` 已把 P6D 合并门在 P7 起转给 P7D；P6D §7.3.5 `:1069` 的「协调者」是收件人，不是执行者）；不得在 P6 fixture 上追加行 |
| `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` | **删除**；新 fixture `p7_pre_allowed.txt` 31 行 | 旧 fixture 不作为第二套基线留下 |
| `Cargo.lock` | 随新 crate 增枚 | 提交时按显式路径带上（同 P6A §2.1） |

**本块不碰**：`crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`）、
`crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`）、
`crates/continuum-runtime/tests/startup.rs` 的三处计数（`:26`／`:81`／`:94`）——本块零表（§9）。
**本块不动 `continuum-runtime` 的 `ALLOWED` 行**：P7D 是库，装配方下一轮实现端口时才加行。

## 2.2 冻结清单：本块持有的类型与入口

**类型**（定义在 `continuum-supervisor`）：

```
TaskSource                —— §299 的四值闭集（§4.2）
TaskError                 —— AuthorizedTask 的拒绝臂（§4.3）
AuthorizedTask            —— 唯一任务输入面；字段私有、四个构造器（§4.3）
PolicyGrantRef            —— policy 来源的不透明授权引用（§4.3）
MaintenanceOccurrenceRef  —— maintenance schedule 来源的不透明引用（§4.3）
SupervisorDecision        —— Task／ContinueSession／NoIntent（§4.4）
ContinueDisposition       —— Continue／Resume；ContinueSession 的控制值（§4.4）
SupervisorError           —— 决策、授权、资源、checkpoint、记忆的失败面（§4.9）
SessionMode               —— Manual／Auto（§4.5）
SessionCandidate          —— ContinueSession／Task／BackgroundFinding／NewConversation（§4.5）
BackgroundNote            —— 一条后台结果摘要（§4.5）
BackgroundDigest          —— completed＋findings 的摘要面（§4.5）
SessionPlan               —— 两模式共用的会话计划；`decision` 持 `SupervisorDecision`（§4.5）
BackgroundSupervisor      —— next_tasks／decide／admit／checkpoint；持每任务挂起状态（§4.4、§4.7）
SessionOrchestrator       —— plan_manual／plan_auto（§4.6）
CheckpointPort            —— 执行 checkpoint 的端口；实现方是装配方／执行器（§4.7）
CheckpointError           —— checkpoint 端口错误；不透明（§4.7）
SynthesisIo               —— §171 三个调用的端口；实现方是装配方（§4.8）
SynthesisTriggerPoint     —— begin／apply／fail 的触发面（§4.8）
```

**入口**（三个决策／计划面，互不合并）：

```
BackgroundSupervisor::next_tasks -> Vec<&AuthorizedTask>       （只过滤，不排序）
BackgroundSupervisor::decide     -> Result<SupervisorDecision, SupervisorError>（转调 P7B）
SessionOrchestrator::plan_manual -> SessionPlan                 （Manual：只展示）
SessionOrchestrator::plan_auto   -> Result<SessionPlan, SupervisorError>（Auto：用 P7B 选择）
SessionOrchestrator::supervisor_mut -> &mut BackgroundSupervisor（装配方共用同一实例，C1）
SessionOrchestrator::supervisor     -> &BackgroundSupervisor（只读）
```

**控制面**（端口＋P7C 状态机；不是决策面）：

```
BackgroundSupervisor::admit     -> Result<ResourceDecision, SupervisorError>（转调 P7C）
BackgroundSupervisor::checkpoint -> Result<SuspensionPhase, SupervisorError>（端口＋P7C 成功／失败）
BackgroundSupervisor::checkpoint_timed_out -> Result<(), SupervisorError>（定时器出口；总是错误）
BackgroundSupervisor::checkpoint_deadline  -> Option<u64>（给调用方安排定时器）
BackgroundSupervisor::suspension_phase     -> Option<SuspensionPhase>（只读；无状态时 None）
BackgroundSupervisor::release              -> bool（任务终止时丢状态）
SynthesisTriggerPoint::{begin,apply,fail} -> Result<..., SupervisorError>（调 SynthesisIo）
```

`admit`／`checkpoint`／`checkpoint_timed_out`／`release` 取 `&mut self`，因为每任务
`SuspensionState` 在 `BackgroundSupervisor` 的内存表里（§4.4、§5.5）；`next_tasks`／`decide`
与两个只读访问器取 `&self`。

**模块落点**（源码面判据的语料按这张表）：

| 文件 | 内容 |
|---|---|
| `src/lib.rs` | 只做 `pub mod` 与 `pub use`，不定义类型 |
| `src/task.rs` | `TaskSource`／`TaskError`／`AuthorizedTask`／`PolicyGrantRef`／`MaintenanceOccurrenceRef` |
| `src/supervisor.rs` | `BackgroundSupervisor`／`SupervisorDecision`／`ContinueDisposition`／`next_tasks`／`decide`／`admit`／`checkpoint`／`checkpoint_timed_out`／`checkpoint_deadline`／`suspension_phase`／`release` |
| `src/session.rs` | `SessionMode`／`SessionCandidate`／`BackgroundNote`／`BackgroundDigest`／`SessionPlan`／`SessionOrchestrator` |
| `src/resource.rs` | `CheckpointPort`／`CheckpointError`；`WorkloadDeclaration` 映射（私有） |
| `src/trigger.rs` | `SynthesisIo`／`SynthesisTriggerPoint` |
| `src/error.rs` | `SupervisorError` |

**不新增的类型**：不定义 `Intent`／`IntentId`／`IntentKind`／`IntentStatus`／`ContractRef`（P4 的）、
`IntentDecision`／`IntentDecisionError`／`SchedulerInputs`／`IntentSelectionPolicy`／`DecisionBasis`
（P7B 的）、`ResourceArbiter`／`ResourceDecision`／`SuspensionState`／`SuspensionPhase`／
`SuspensionError`／`ResourceAction`／`WorkloadDeclaration`（P7C 的）、
`SynthesisTrigger`／`SynthesisRequest`／`SynthesisJobId`／`SynthesisProposal`／`SynthesisReport`／
`MemoryError`（P6C 的）、`Decision`（§331 的）、`Evidence`／`CompletionVerdict`（P5a 的）、
`Product`／`Milestone`／`ProductReadiness`（P7A 的）。

**本块不持有的判定**：Intent 排序与候选状态过滤（P7B）；资源压力、负载类别、四档优先级、
相位转移规则（P7C）——P7D 只持有每任务 `SuspensionState` 的值并按 P7C 的方法推进，不重写转移表；
Product 的 readiness／shippable／progress（P7A）；Intent 状态迁移（P4）；记忆合成的语义
（P6C）；§171 的阈值／频率／重试（P10）；执行与 UI（P1／P8）。

## 2.3 本块的层间位置

§9.1（`docs/02-工程.md:569-588`；读法 `:561-567`；`:588` 逐字「依赖方向单向，无环。」）里与本块有关的
几条，按「被依赖者 → 依赖者」读：

- **`认知层 (6) → 长期循环 (7)`**（`:583`）：`continuum-memory` → `continuum-supervisor` 是这一条；
  §171 的触发边是既有方向。
- **`资源层 (4) → 长期循环 (7)`**（`:578`）：P7C 的 `continuum-arbiter` → P7D 是这一条；P7D 不反向
  依赖任何第 4 层 crate。
- **`长期循环 (7) → 长期循环 (7)` 的层内边**：P7B → P7D 是层内边；P7B 不依赖 P7D。
- **§7.3 的 `Background Supervisor ← Resource Arbiter + Intent Scheduler`**（`:453`）与
  **`Session Orchestrator ← Intent 存储 + Background Supervisor`**（`:454`）：本块把前者落成
  `continuum-arbiter`／`continuum-intent-scheduler` 两条普通边；把后者落成「候选集 `&[Intent]` 由
  调用方给」——**`Intent 存储` 不是一条 crate 边**（切分 §6.2 第 5 条 `:404`）。
- **本块不接 `边界层 (5) → 长期循环 (7)`**：不依赖 policy／capability；policy 授权引用是不透明值。

**层内位置**：P7D 是 P7B／P7C 的消费方；P7B／P7C 不依赖 P7D。P7D 与 P7A 之间只有按值交接
（切分 §三第 6 条），没有 crate 边。

## 2.4 本设计自定的判定（逐条：规范给到哪、代价、边界、落点、收件人）

**这一节是本块的「发明判据」总账（共 24 条；其中 #8／#15／#17 是切分／规范已给的复述，自定 21 条）。**
凡会改变跨块可观察行为的自定判定都在下表里有一条；纯机械转发、访问器与错误包装不在表内，也不得与
本表冲突。代码块只是示意，正文措辞才是约束。

| # | 判定（**自定**／**复述**） | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | `TaskSource` 四值的**判定规则**：`AuthorizedIntent`＝非 Product 的 `READY` Intent＋有效 contract；`ProductDrive`＝`kind == Product` 且 `product_id` 非空的 `READY` Intent＋有效 contract；`Policy`＝非空 `PolicyGrantRef`；`MaintenanceSchedule`＝非空 `MaintenanceOccurrenceRef` | §299 `:1894-1901` 只给四个名字；**没有说四类如何区分、证据是什么** | 两个 Intent 来源靠 `kind` 区分；Product 不能走 `AuthorizedIntent`；policy／维护来源不携带 Intent | 不覆盖「policy 与 maintenance 的授权真实性」；不覆盖「同一任务同时属于两类」；不覆盖「模型建议」——没有第五类 | §4.2、§4.3、§5.1 | 规范维护者（§299）＋协调者＋第 5 层 |
| 2 | `AuthorizedTask` 的**私有字段与四个构造器**；裸 `TaskSource` 不能构造 | 切分 §2.3.1 第 2 条 `:138` 要求「构造器私有」；**形状未给** | 调用方必须按来源选构造器；没有通用 `new`；新增来源要加构造器 | 不覆盖「调用方伪造 `Intent` 值」——`Intent` 字段是 `pub`；构造器只校验字段 | §4.3 | 规范维护者＋P7A |
| 3 | **`product drive` 证据**：`kind == Product`、`product_id` 非空、`contract.id` 非空、`contract.version >= 1`、`status == READY`；其余十二态拒绝 | 切分 §三第 6 条 `:226-237` 已给规则（选项 a）；**代码形状未给** | 不读合同内容、不验证合同存在、不防生产方自报状态 | 不覆盖「合同已撤销」；不覆盖「policy 判定」；不覆盖「合同版本过期」 | §4.3、§5.2 | 协调者＋第 5 层＋P7A |
| 4 | **`AuthorizedTask` 的 `READY` 口径**：`authorized_intent` 也要求 `READY` 且 `kind != Product`；`product_drive` 要求 `READY` 且 `kind == Product`；**`READY` 约束只适用于这两条携带 `Intent` 的路**；`policy`／`maintenance` 没有 Intent，以非空引用为证据 | §299 只说「authorized Intent」；§223 `:172-205` 只给状态机；切分 §五判据 1 的「只收 READY」未区分来源 | 非 Product 的 `RUNNING` Intent 不能作为新后台任务；已运行会话走 `ContinueSession`；policy／maintenance 不受 READY 约束 | 不覆盖「已有运行会话的继续」——那不是新任务；不覆盖「Intent 已授权但未 READY」；不覆盖 policy／maintenance 的真实性（见第 5 条） | §4.3、§5.2 | 规范维护者＋P7B＋第 5 层 |
| 5 | **`PolicyGrantRef` / `MaintenanceOccurrenceRef` 的形状**：分别是 `issuer＋grant_id` 与 `schedule_id＋occurrence_id` 的不透明非空串 | 切分 §2.3.1 第 2 条 `:137-139` 说「其余来源的 policy／capability 引用由 P7D 设计与第 5 层写」，**未给形状** | P7D 不能验证 grant 的真实性、有效期、是否被撤销；只能校验非空 | 不覆盖「伪造 grant」；不覆盖「grant 与任务内容不匹配」；不覆盖「policy 语义」 | §4.3、§5.1 | 第 5 层＋协调者＋规范维护者 |
| 6 | **`SessionPlan` 的形状**：`mode`／`candidates`／`decision`／`background` 四个字段，两模式共用；`selection()`／`basis()` 是从 `decision` 派生的只读投影，不另存 | §11 `:576-620` 只给流程与例子；**没有字段、没有类型** | 消费方按固定四字段读；新增字段是破坏性改动；`candidates` 的固定段序是本设计定的 | 不覆盖「UI 的实际呈现」；不覆盖「用户选择后的状态」 | §4.5 | 规范维护者＋前端＋P8 |
| 7 | **Manual 的候选段序**：existing sessions（输入序）→ tasks（输入序）→ findings（输入序）→ `NewConversation`；段内不排序 | §11 `:582-593` 只给四个示例；**没有顺序规则** | 固定段序可能不是 UI 想要的；用户可能期望「新对话」在最前 | 不覆盖「按用户偏好重排」；不覆盖「跨段优先级」 | §4.6、§5.4 | 前端＋规范维护者 |
| 8 | **复述**：**Auto 的选择只来自 P7B**：`Supervisor::decide` 调 `IntentScheduler::decide`；`Advance` 映射到 `Task` 或 `ContinueSession`；`NoIntent` 映射到无选择；`Err` fail-closed | §11 `:595-620` 只说「选择当前最值得继续的任务」；切分 §五 P7D 完成判据 2 要求复用 P7B | P7D 不能对 P7B 的结果再加权；P7B 策略的失败直接变成 `SupervisorError` | 不覆盖「P7B 策略本身错误」；不覆盖「后台结果不是 Intent」——见第 9 条 | §4.4、§5.3 | P7B＋协调者 |
| 9 | **后台新结果不参与排名**：`BackgroundDigest` 只展示；P7D 不把 finding 转成 Intent、不把它塞进 `SchedulerInputs` | §11 `:604-608` 要求 Auto「分析后台新结果」；**P7B 的 §302 五输入没有「后台结果」这一项** | Auto 只能显示 findings，不能因 finding 改变选择；若要让 finding 影响选择，装配方须把它投影成 `user_focus` 或生成 Intent | 不覆盖「finding 应优先」；不覆盖「finding 与 Intent 的对应」 | §4.5、§5.4、§10 第 7 条 | 规范维护者＋P7B＋P7D |
| 10 | **`TaskSource` → `WorkloadDeclaration` 的映射**：`MaintenanceSchedule` ⇒ `maintenance=true`；其余 ⇒ 三个布尔全 false | P7C §7.1 `:1203-1218` 只给建议；**P7D 的映射未冻** | 所有 P7D 任务落 `BackgroundRuntime`；`ForegroundRuntime`／`UserWorkload` 两档在 P7D 路径上不可达 | 不覆盖「用户正在等待的后台任务」；不覆盖「前台会话的任务」——那不在 P7D | §4.7、§5.5 | P7C＋规范维护者 |
| 11 | **checkpoint 的三出口与定时器控制面**：端口 Ok → `checkpoint_succeeded`；端口 Err → `checkpoint_failed`；定时器 → `checkpoint_timed_out`；P7D 暴露 `checkpoint_deadline` 给调用方安排超时；晚到成功被 `checkpoint_succeeded` 拒绝 | §14 `:730-744` 与切分 §2.3.2 第 2 条 `:159-165` 只给相位与失败回退；**checkpoint 的内容／执行者／P7D 定时器面未给** | 端口实现必须知道任务的可恢复状态；P7D 不保存 checkpoint 内容；调用方必须按截止轮询（P7D 不建线程） | 不覆盖「checkpoint 数据是否完整」；不覆盖「重启恢复」；不覆盖「调用方完全不轮询」 | §4.7、§5.5 | P7C＋装配方＋P1 |
| 12 | **§171 触发点做成 `SynthesisIo` 端口**：P7D 定义调用面与三个转发，装配方持 `Tx`；评审已裁定（§10 第 4 条） | 切分 §2.3.1 第 4 条原写「直接依赖 `continuum-memory`，调 `begin/apply/fail`」；**`begin_synthesis` 需要 `&Tx<'_>`，P7D 不依赖 `continuum-persist`**；切分已按本裁定同步 | 多一个端口实现；P6C 的错误经端口返回；字面上不是 P7D 直接调自由函数 | 不覆盖「端口实现是否真的调了 P6C」——那是装配方的契约 | §4.8、§10 第 4 条 | 协调者＋P6C＋装配方 |
| 13 | **零持久化**：`SessionPlan`／`AuthorizedTask`／`BackgroundDigest` 都是值；不建表、不占 201 | §11／§156 没有持久化要求；切分 §四第 2 条 `:273` 说「P7D 若落库才占 201」 | 重启后会话计划与后台摘要不恢复；装配方要自己保存后台结果 | 不覆盖「昨晚完成」的跨重启展示（§156 `:395-404`） | §9、§10 第 10 条 | 协调者＋装配方 |
| 14 | **「低风险 Milestone」不由 P7D 判**：P7D 收所有 `READY` Product Intent | §156 `:392` 只说「继续完成低风险 Milestone」；**没有风险字段／判据** | P7D 可能把非低风险 Milestone 纳入后台；风险判断在 P7A／Product Drive | 不覆盖「风险模型」；不覆盖「Milestone 的副作用」 | §5.1、§10 第 8 条 | P7A＋规范维护者 |
| 15 | **复述**：**P6D 合并门选 rebaseline**：`p7_pre_allowed.txt` 31 行、`P7_EXPECTED_ROWS` 四行、两个 `27` → `31`、断言 2 的 `5` → `4` | 切分 §四第 1 条 `:259-264` 给两条路；P6D §7.3.5 给生命周期 | 多一个 fixture 与一次五处同改；下一个子项目还会到期 | 不覆盖「故意同时改基线」——同 P6D 的自保护边界 | §5.7、§9.2 | 协调者＋P6D |
| 16 | **`SessionPlan` 的选择不保证资源准入**：Auto 可以先选中一个随后被 `Suspend` 的任务 | §11 只说「选择」；P7C 的动作在 `direct` 时才知道 | 用户可能看到「已选中」但任务被挂起；UI 要展示资源状态 | 不覆盖「选择时的资源预检」；不覆盖「挂起后的重选」 | §5.5、§10 第 9 条 | P7C＋前端 |
| 17 | **复述**：**§171 的触发策略不由 P7D 定义**：P7D 只给触发面；四个时机的判定、空闲阈值、频率、重试归 P10 | §171 `:931-938` 只给四个时机的名字；切分 §6.2 第 10 条把阈值／频率／重试判给 P10 | Dreaming 交付后仍没有自动触发器；调用方必须自己决定何时调 `begin` | 不覆盖「何时触发」；不覆盖「重试与恢复」 | §5.6、§10 第 17 条 | P10＋协调者 |
| 18 | **`SuspensionState` 由 P7D 按任务持有**：内存表 `states: HashMap<TaskKey, SuspensionState>`；键对 Intent 任务是 `IntentId::as_str()` 的 `String`（`IntentId` 没有 `Hash`／`Eq`），policy／maintenance 用不透明引用；零持久化 | P7C §4.8 `:680`／`:714` 只说「P7D 持有」；**表的形状、生命周期、跨重启行为未给** | 重启后全部相位丢失；表随任务数增长；任务终止时装配方必须调 `release` | 不覆盖「跨重启恢复」；不覆盖「多进程共享状态」；不覆盖「伪造同一引用」 | §4.4、§5.5、§9.1 | P7C＋协调者＋装配方 |
| 19 | **`next_tasks` 的过滤口径**：Intent 任务以 `candidates` 的**当前** `status == READY` 为准（按 `as_str()` 找 id），不看 `AuthorizedTask.intent().status` 的构造快照；不在候选集即丢弃；policy／maintenance 无条件保留；保持 `tasks` 输入序 | §299 只给来源；§11 只给候选示例；**没有说快照与候选冲突时取谁** | 构造时 READY、后来变为非 READY 的任务不再展示；调用方必须给当前候选集 | 不覆盖「候选集本身过期」；不覆盖「重复 id 选哪一枚」——调用方契约违反 | §4.4、§5.4 | P7B＋前端 |
| 20 | **`decide` 的状态分派与恢复控制值**：`Ready` ⇒ `Task`；`Waiting`／`Suspended` ⇒ `ContinueSession { disposition: Resume }`，**不要求 `existing_sessions`**；`Running`／`Verifying`／`Evolving` ⇒ `ContinueSession { disposition: Continue }`，要求既有会话；其他 ⇒ `UnsupportedAdvance`（防御） | P7B §5.1 给了启动／恢复／已在运行的分支；**P7D 侧的控制值、无会话时怎么办未给** | 新增 `ContinueDisposition`；`NoActiveSession` 只对 `Continue` 分支；装配方必须按控制值执行恢复 | 不覆盖「恢复失败」；不覆盖「会话句柄的建立」 | §4.4、§5.3 | P7B＋装配方 |
| 21 | **候选与摘要的形状**：`SessionCandidate` 四臂；`BackgroundNote { label, at_millis: i64 }`；`BackgroundDigest { completed, findings }` 保持输入序；`BackgroundDigest::default()` 是空摘要（允许的 `Default`；计划本体 `SessionPlan` 不实现 `Default`） | §11 只给候选示例；§156 只给展示语义；**没有字段** | 字段固定；`at_millis` 与会话时间同口径（`i64`），与资源时钟 `u64` 不混用 | 不覆盖「UI 呈现」；不覆盖「跨重启展示」 | §4.5 | 前端＋P8＋规范维护者 |
| 22 | **`SupervisorDecision`／`SessionPlan`／`SupervisorError` 的形状与单一表示**：`SessionPlan.decision: Option<SupervisorDecision>` 是唯一选择表示；`selection()` 只投影 Task／Continue；`basis()` 委托；**没有 `SessionSelection`**；`SupervisorError` 十臂 | §11 只说「选择」与「Session Plan」；**没有类型** | 消费方要匹配 `SupervisorDecision`；`NoIntent` 也占一个 `decision` 值；错误臂是冻结面 | 不覆盖「UI 自己的展示模型」；不覆盖「错误文案」 | §4.4、§4.5、§4.9 | 前端＋规范维护者 |
| 23 | **`plan_manual` 不调 `decide`、`plan_auto` 不恢复会话**：Manual 只 `next_tasks`＋拼接；Auto 只产出计划；§11 的「然后直接恢复」由装配方按 `ContinueDisposition` 执行 | §11 说 Auto「然后直接恢复」；切分判据 2 只说选择复用 P7B，**没说谁执行恢复** | 计划与执行分离；调用方必须消费 `ContinueDisposition` | 不覆盖「恢复失败」；不覆盖「会话生命周期」 | §4.6、§5.4 | 装配方＋P8 |
| 24 | **checkpoint 的定时器出口**：`checkpoint_deadline` 暴露 P7C 的截止；`checkpoint_timed_out` 是独立控制面；端口在定时器之后返回也只落到已定义的 `NotCheckpointing`／`Checkpoint` 臂，不把状态写回 `Suspended` | P7C §5.6 给相位与错误，**没有给 P7D 的定时器 API** | 调用方要按 `checkpoint_deadline` 轮询；P7D 不建线程 | 不覆盖「调用方完全不轮询」 | §4.7、§5.5 | P7C＋装配方 |

## 2.5 切分 §四 六条横切约束的逐条落实

| 切分 §四 | 要求（要点） | 本设计的落点 | 可观察判据 |
|---|---|---|---|
| **第 1 条**（`:253-264`） | 不新增反向边；四块各自在 `ALLOWED` 加自己一行；既有行不因 P7 增枚，唯一例外是装配方 `continuum-runtime`；P6D 合并门由 P7D 复核 | §2.1 的五项边表；§9.2 的 rebaseline | J15（合并门）＋J16（依赖门） |
| **第 2 条**（`:265-276`） | 迁移号段 200–209；P7D 若落库顺取 201 | §9：本块零表、零迁移；`runtime_migrations()`／`expected_migrations()`／`startup.rs` 不动 | J17 |
| **第 3 条**（`:277-285`） | 同一件事两个词汇表：`TaskSource`／`SynthesisTrigger`／§79 四档不得合并；`Suspended` 三处不合并 | §2.6 的同名异物表；§4.2 的 `TaskSource` 四值；§4.8 不重定义 `SynthesisTrigger` | J1、J14 |
| **第 4 条**（`:286-293`） | 工具结果／Evidence 消费；后台发现若生成 Intent 必须先转 `Evidence` | §2.1 不登记 `continuum-verify`；§5.4 的 `BackgroundDigest` 只是不透明摘要；§3.8 | J12 |
| **第 5 条**（`:294-296`） | 依赖边只登记实际用到的；`ALLOWED` 与实际依赖精确一致 | §2.1 的四普通＋一 dev 与 `ALLOWED` 五项一一对应 | J16 的两个方向 |
| **第 6 条**（`:297-311`） | P7B → P7D、P7C → P7D 的冻结点先冻；P7A 与 P7D 按值交接 | §6 的接缝表；§4.4 调 P7B `decide`；§4.7 调 P7C `direct`；§4.3 按值收 Product Intent | J7、J13、J22 |

## 2.6 同名异物（**都不是**同一个概念）

| 同名词／类型 | 它是什么 | 实测位置 | 本块的处置 |
|---|---|---|---|
| `TaskSource` | §299 的四类后台任务来源 | `docs/spec/05-normative.md:1894-1901` | P7D **只拥有这一枚**；四值闭集；无 `parse` |
| `SynthesisTrigger` | §171 的四个时机 | `crates/continuum-memory/src/synthesis.rs:64` | P6C 拥有；P7D 只经 `SynthesisRequest.trigger` 传递，**不重定义、不合并** |
| §79 的四档交互优先级 | 解释面 | `docs/spec/02-positioning.md:655-669` | 不建第二套 `Priority`；P7D 不读 |
| `AuthorizedTask` | P7D 的唯一任务输入面 | 本设计 §4.3 | 不是 `Intent` 的别名；不是 `SynthesisTrigger`；不是 `WorkloadDeclaration` |
| `SessionPlan` | §11 的会话计划 | 本设计 §4.5 | 不是 §228 的 `Plan`（P4 的 Plan 数据模型），也不是 P7B 的 `IntentDecision` |
| `SessionSelection`（**已删**） | 评审前设计里的第二套选择表示 | 本设计 §4.5 | **不建**：`SessionPlan.decision` 直接持 `SupervisorDecision`；`selection()` 只是只读投影，不存第二份值 |
| `Decision` | §331 的决策单／策略的 `Allow/Deny/RequireApproval` | `crates/continuum-semantics/src/decision.rs:39-44`、`:80` | P7D 不写 `decision.required`；`SupervisorDecision` 不是 §331 |
| `Suspended` | `NodeState::Suspended`／§14 的协作式挂起态／`IntentStatus::Suspended` | `crates/continuum-graph/src/node.rs:63`、`docs/spec/01-concepts.md:741`、`crates/continuum-semantics/src/intent.rs:77` | P7D 只消费 P7C 的 `SuspensionPhase`；不建挂起状态机、不复用 `NodeState` |
| `priority` | `Intent.priority`（不透明）／§302 输入／§78 七因子／§300 四档 | `crates/continuum-semantics/src/intent.rs:300`、`docs/spec/05-normative.md:1950`、`docs/spec/02-positioning.md:635-651`、`:1907-1921` | P7D 不解析；P7B 的 `PrioritySignal` 是 P7B 的；P7D 只把 `Intent` 交给 P7B |
| `BackgroundFinding` | P7D 的不透明摘要行 | 本设计 §4.5 | 不是 `Evidence`；不是 `Claim`；不承载验证结论 |
| `SessionCandidate::Task` | P7D 的 `AuthorizedTask` 包装 | 本设计 §4.5 | 不是 `Intent` 的第二个候选集；不排序 |

---

# 3. 规范依据逐条

**这一节的写法**：每条先给规范原文的落点，再给「规范给了什么 / 规范没给什么 / 本设计怎么落」。
**没有一条判据是本节发明的**——规范没给的部分在 §2.4 里另记。

## 3.1 §299 Background Supervisor（`docs/spec/05-normative.md:1892-1903`）

**原文（逐字，`:1894-1903`）**：

> 后台任务 MUST 来自：
> ```
> authorized Intent
> policy
> maintenance schedule
> product drive
> ```
> 不得因为模型"觉得有必要"就无限创建后台任务。

**规范给了什么**：①后台任务的来源闭集四类；②「MUST 来自」；③禁止模型自行无限创建。

**规范没给什么**：①四类的判定规则与证据；②`AuthorizedTask` 的类型；③「authorized Intent」与
「product drive」的区分；④policy／maintenance 的引用形状；⑤「无限」的边界。

**本设计怎么落**：`TaskSource` 恰四值（§4.2）；`AuthorizedTask` 的四个私有构造器按来源校验
（§4.3）；`product drive` 取 `READY` Product Intent＋`contract_id`＋`status`（§5.2）；
`AuthorizedIntent` 取非 Product 的 `READY` Intent＋有效 contract；policy／maintenance 取不透明
非空引用。**没有第五类**，没有模型建议入口（J1）。

## 3.2 §12 Background Supervisor（`docs/spec/01-concepts.md:624-665`）

**原文（逐字节选，`:626`、`:643-665`）**：

> 软件关闭后，Runtime 可以继续执行授权过的后台任务。
> …
> Repository changed
>  ↓
> Background Supervisor
>  ↓
> affected modules
>  ↓
> spawn scanner
>  ↓
> 发现 regression
>  ↓
> 生成 Intent
> …
> 后台自治针对的是项目与任务世界，而不是持续监测用户行为。

**规范给了什么**：①软件关闭后可继续授权后台任务；②Background Supervisor 的扫描→发现→生成 Intent
例子；③后台自治的对象是项目与任务世界，不是用户行为监测。

**规范没给什么**：①扫描器的实现；②「发现 regression → 生成 Intent」的接线；③后台任务的存储；
④`TaskSource` 的判定。

**本设计怎么落**：P7D 只建后台治理与会话启动，不建扫描器（§8）；后台发现以不透明摘要进入
`BackgroundDigest`；**不生成 Intent**——若发现要走 Intent，必须先按 §89 转 `Evidence`，再由
P5a／装配方生成（§3.8、§5.4）。P7D 不读进程／窗口／键盘（J18）。

## 3.3 §11 Session Orchestrator（`docs/spec/01-concepts.md:576-620`）

**原文（逐字，`:578-620`）**：

> 用户每次打开软件，形成一次 Session Activation。
> 系统提供两种模式。
> **Manual**
> Runtime 自动给用户生成候选：
> ```
> 继续 Core
> 继续 LCM
> 查看昨晚发现的问题
> 开始新对话
> ```
> 用户选择。
> **Auto**
> 启动专门的 Planner Agent：
> ```
> App Open
>  ↓
> Session Orchestrator
>  ↓
> 分析未完成 Intent
>  ↓
> 分析后台新结果
>  ↓
> 选择当前最值得继续的任务
>  ↓
> 创建 Session Plan
> ```
> …
> 然后直接恢复。

**规范给了什么**：①每次打开软件一次 Session Activation；②Manual／Auto 两模式；③Manual 的候选
示例；④Auto 的流程与 `SessionPlan` 这个名字；⑤Auto 直接恢复。

**规范没给什么**：①`SessionPlan` 的字段；②候选的顺序／过滤；③「最值得继续」的算法；④Planner Agent
由谁实现；⑤「分析后台新结果」如何进入选择。

**本设计怎么落**：`SessionPlan` 五字段（§4.5）；Manual 只展示、不选（§4.6）；Auto 调 P7B
`IntentScheduler::decide`，不另写排名（§5.3）；Planner Agent 不建，模型判断在 P7B 的
`IntentSelectionPolicy` 实现里（§10 第 16 条）；后台结果只展示、不参与排名（§5.4、§10 第 7 条）。

## 3.4 §156 Product Drive 与 Background Supervisor（`docs/spec/03-product-drive.md:377-406`）

**原文（逐字，`:379-406`）**：

> Product Drive 可以长期驻留。
> 例如：
> ```
> 用户离开
>  ↓
> Background Supervisor
>  ↓
> 继续测试
> 继续 fuzz
> 继续 benchmark
> 继续静态分析
> 继续完成低风险 Milestone
> ```
> 用户再次打开软件：
> ```
> 昨晚完成：
> M17 parser optimization
> M18 regression suite
>
> 当前：
> M19 benchmark validation
> ```
> 因此系统可以真正持续向成品推进。

**规范给了什么**：①Product Drive 长期驻留；②Background Supervisor 可继续低风险 Milestone；
③用户重开时展示「昨晚完成／当前」；④持续向成品推进。

**规范没给什么**：①「低风险」的判据；②「昨晚完成」的存储与读取；③Product Intent 与 Milestone 的
交接形状；④后台结果的摘要格式。

**本设计怎么落**：`product drive` 来源按值收 `READY` Product Intent（§4.3、§5.1）；`BackgroundDigest`
承载「昨晚完成／发现」的展示（§4.5），但**不持久化**（§9、§10 第 10 条）；「低风险」不由 P7D 判
（§10 第 8 条）。P7D 不读 Product 记录。

## 3.5 §171 Dreaming / Memory Synthesis（`docs/spec/03-product-drive.md:905-938`）

**原文（逐字，`:907-938`）**：

> 系统后台周期运行：
> ```
> Memory Synthesis Job
> ```
> 流程：
> ```
> Recent Sources
>       ↓
> Relevant historical sources
>       ↓
> Contradiction detection
>       ↓
> Temporal reconciliation
>       ↓
> Importance estimation
>       ↓
> Memory synthesis
>       ↓
> Memory Summary
> ```
> 它可以发生：
> ```
> session结束后
> 系统空闲时
> 大量项目状态改变以后
> 用户主动要求时
> ```

**规范给了什么**：①后台周期运行 Memory Synthesis Job；②七步流程；③四个可发生时点；④Memory Summary
这个产物名。

**规范没给什么**：①「空闲」的时长与阈值；②「大量」的数量；③检查频率；④重试与恢复；⑤四个时点与
`SynthesisTrigger` 的映射（P6C 已给四臂）。

**本设计怎么落**：`SynthesisTriggerPoint` 只提供 `begin`／`apply`／`fail` 三个转发（§4.8）；四个时机
由 P6C 的 `SynthesisTrigger` 表达（`synthesis.rs:64`），P7D **不重定义、不持阈值、不做空闲判定、不做
重试**；阈值／频率／重试归 P10（切分 §6.2 第 10 条）。J14 钉「只提供触发点」。

## 3.6 《工程》§7.1–§7.4（`docs/02-工程.md:420-473`）

**§7.1**（`:420-431`）：`Background Supervisor` 的规范依据是 `§299 §12`（`:428`）；`Session Orchestrator`
是 `§11`（`:430`）。本块只拥有这两行，不拥有 Product／Milestone／Progress Gate／Shippable／Resource
Arbiter／三层调度。

**§7.2**（`:433-443`）：`:443` 逐字「**Background Supervisor** 的任务来源限于四类（§299）。」本设计的
`TaskSource` 闭集四值即这条的落点（J1）。

**§7.3**（`:445-458`）：`:453` 的 `Background Supervisor ← Resource Arbiter + Intent Scheduler` 落成
两条普通边；`:454` 的 `Session Orchestrator ← Intent 存储 + Background Supervisor` 落成「候选集由调用方
给＋同 crate 的 Supervisor」；**`Intent 存储` 不是 crate 边**（切分 §6.2 第 5 条）。

**§7.4**（`:460-467`）：四条判据分别落在 P7A（判据 1、2、4）与 P7C（判据 3）；**没有 P7D**（切分 §七
第 12 条）。本设计自报 J1–J3 与 J4–J22 作为 P7D 的完成判据，**不修改 §7.4**，也不把 J 编号冒充成
§7.4 的第五条（§10 第 6 条）。

## 3.7 §223 Intent 状态机与 `READY` 证据（`docs/spec/05-normative.md:172-205`）

**§223 给了什么**：十三个状态；进入运行的五步链 `PROPOSED → SPECIFYING → PLANNING → REVIEW →
READY → RUNNING`；运行后六态；Product Intent 的 `EVOLVING`。代码在 `intent.rs:61`，迁移判定
`transition` 是 `pub(crate)`（`:217`）。

**规范没给什么**：`READY` 的「授权」含义；`READY` 之外的状态能否作为后台任务证据。

**本设计怎么落**：`AuthorizedTask` 的两个 Intent 来源都要求 `status == READY`；`COMPLETED`／`EVOLVING`
也拒绝（不是可执行任务）。**`READY` 是唯一可用的状态证据**：P4 设计
（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`）的迁移表里 `READY` 只有
`REVIEW → READY` 一条入边（`:879`；`intent.rs:249`），故「已过审查」由状态本身可证明。
运行态不能证明：`PLANNING → SUSPENDED`（`:878`；`intent.rs:247`）与
`SUSPENDED → RUNNING`（`:883`；`intent.rs:264`）可绕过 `REVIEW`；把 `RUNNING`／`WAITING`／
`SUSPENDED`／`VERIFYING`／`EVOLVING` 任一当证据都会接受一条从未过审的 Intent。
**J3 的 `RUNNING` 负例直接构造 `Intent { status: Running, .. }`**（`Intent` 字段是 `pub`）：
它只证明「`RUNNING` 被拒绝」；「`RUNNING` 可绕过 `REVIEW`」由上面的 P4 迁移表证明，不由运行期
照片走一遍 `apply_intent_transition` 证明（P7D 没有 `continuum-persist` 边，§4.1）。

**`COMPLETED` 单列**：它不是严格终态（P4 只允许 `COMPLETED → EVOLVING`、仅 Product，P4 设计
`:885`／`:893`；`intent.rs:222`），但也不是可执行任务；P7D 不接，P7A 先把 Product Intent 推进到
`EVOLVING`，新的 Milestone 重新 lower 出新的 `READY` Product Intent。`FAILED`／`CANCELLED` 是终态。

## 3.8 §89 Tool Result Validation（`docs/spec/02-positioning.md:923-945`）

**原文（逐字，`:939-945`）**：

> 所以工具结果必须转换成：
> ```
> Evidence
> ```
> 再交给节点 Completion Predicate 判断。

**规范给了什么**：工具结果不等于语义成功；必须转成 `Evidence` 再判完成。

**规范没给什么**：Evidence 的转换实现（P5a 已有 `Evidence::from_tool_result`）；后台发现如何进入
Intent。

**本设计怎么落**：P7D 不消费工具结果、不定义第二枚 `Evidence`（切分 §四第 4 条）。`BackgroundDigest`
只装不透明摘要文本；若后台发现要生成 Intent，必须先按 §89 转 `Evidence`，再由 P5a／装配方转 Intent；
P7D 不在本块做这个转换（§2.1、§3.8、§5.4）。

## 3.9 P6D 合并门（P6D 设计 §7.3.5；`dependency_direction.rs`）

**规范／切分给了什么**：P6D 的并集门是 **P6 期间的合并门**，不是永久门；P7 起二选一：①以
`PRE_P7_ALLOWED` 新 fixture 替换（保留同一八条断言，`P6_EXPECTED_ROWS`／`P6_NAMES`／断言 2 计数随 P7
重建）；②退役，改为从 `Cargo.toml`／`cargo metadata` 生成基线。**不允许**在 `p6_pre_allowed.txt` 上
追加行（切分 §四第 1 条 `:259-264`；P6D §7.3.5 `:1044-1125`）。

**实测**：

- `P6_EXPECTED_ROWS` 在 `crates/continuum-runtime/tests/dependency_direction.rs:551`；`p6_names` `:574`；
  `pre_p6_allowed` `:582`；解析器 `rows.len() == 27` `:615`；测试
  `p6_union_adds_only_its_own_rows` `:784`；断言 1 `baseline.len() == 27` `:790`；断言 2
  `P6_EXPECTED_ROWS.len() == 5` `:799`。
- fixture `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 有 **30 行**（3 行注释＋27 行
  数据）。
- 当前 workspace 有 **31 个 crate**（`Cargo.toml` 的 `members`），P7 将新增四个
  （`continuum-product`／`continuum-intent-scheduler`／`continuum-arbiter`／`continuum-supervisor`）。
- P7 首先撞红的是断言 5（runtime 行增量 ⊆ `P6_NAMES`，`:852`）与断言 8（新增 subject 不在
  `P6_NAMES`，`:894`），不是断言 1；但 rebaseline 后旧断言 1 与解析器的两个 `27` 也必须同批改
  （切分 §七第 11 条 `:461`）。

**本设计怎么落**：**选 rebaseline**（不退役），细节在 §5.7 与 §9.2。两个 `27` 都改成 `31`；断言 2
的 `5` 改成 `4`；fixture 换成 `p7_pre_allowed.txt`；旧 fixture 删除；测试改名
`p7_union_adds_only_its_own_rows`。**不在 P6 fixture 上追加行**。

**所有权**：切分 §一 1.2（`:55`）与 §一 1.3（`:72`）已把 P6D 合并门在 P7 起转给 P7D；P6D
§7.3.5（`:1069`）的「所有者：P6D（本轮）→ 协调者（P7 起）」里，「协调者」是**收件人**，不是
执行者。本设计按切分执行；合并评审时由协调者确认这一执行者（§10 第 5 条）。

---

# 4. 类型与接口

**这一节是 P7D 对装配方／UI 的冻结面。** 代码块是示意，正文措辞才是约束；字段名、变体名、访问器名
以本节为准。所有类型都在 `continuum-supervisor` 内定义；跨块类型只引用，不重定义。

## 4.1 模块落点

见 §2.2 的表。**测试位置**：全部放 `crates/continuum-supervisor/tests/`，不放 `src/` 的
`#[cfg(test)]`；这样「源码面判据」的语料（`src/`）里没有测试构造（同 P6A §8.2 的纪律）。测试只用
`std`＋本 crate＋四个普通依赖＋`trybuild`＋`serde_json`（仅 J11 的 `Deserialize` 负例；两者都是
外部 dev 依赖）；假 `IntentSelectionPolicy`／`SynthesisIo`／`CheckpointPort` 由 `tests/` 自己写。

## 4.2 `TaskSource`

```rust
/// §299 的四类后台任务来源。**恰四变体；没有第五类，没有 `ModelSuggested`。**
/// 不实现 `FromStr`／`parse`：模型输出或裸字符串不能构造来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSource {
    /// §299 的 `authorized Intent`：非 Product 的 `READY` Intent ＋有效 contract。
    AuthorizedIntent,
    /// §299 的 `policy`：第 5 层的非空授权引用。
    Policy,
    /// §299 的 `maintenance schedule`：维护计划的非空 occurrence 引用。
    MaintenanceSchedule,
    /// §299 的 `product drive`：P7A 交接的 `READY` Product Intent。
    ProductDrive,
}

impl TaskSource {
    pub fn as_str(&self) -> &'static str;
}
```

- **`as_str` 的四个字面量**：`"authorized_intent"`／`"policy"`／`"maintenance_schedule"`／
  `"product_drive"`。**不提供 `parse`**：没有「从字符串恢复来源」的入口，避免模型输出成为第五类。
- **`TaskSource` 是公开的**：UI 可以显示来源；但它**不能**单独构造 `AuthorizedTask`（§4.3）。
- **不合并 `SynthesisTrigger`**：两个四值枚举不同物（切分 §2.4 `:177`）；本 crate 的 `TaskSource`
  不出现 `SessionEnded`／`SystemIdle`／`BulkProjectStateChanged`／`UserRequested`。

## 4.3 `AuthorizedTask` 与授权证据

```rust
/// P7D 的唯一任务输入面。**字段私有；四个构造器；没有 `new(source, evidence)`。**
/// 裸 `TaskSource` 不能构造本类型（J2）。
#[derive(Debug, Clone)]
pub struct AuthorizedTask {
    source: TaskSource,
    evidence: TaskEvidence,
}

#[derive(Debug, Clone)]
enum TaskEvidence {
    Intent(Intent),
    Policy(PolicyGrantRef),
    Maintenance(MaintenanceOccurrenceRef),
}

impl AuthorizedTask {
    /// `authorized Intent`：非 Product 的 READY Intent＋有效 contract。
    pub fn authorized_intent(intent: Intent) -> Result<Self, TaskError>;

    /// `product drive`：READY Product Intent＋有效 contract（§三第 6 条的选项 a）。
    pub fn product_drive(intent: Intent) -> Result<Self, TaskError>;

    /// `policy`：第 5 层的非空授权引用；不携带 Intent。
    pub fn policy(grant: PolicyGrantRef) -> Result<Self, TaskError>;

    /// `maintenance schedule`：非空 occurrence 引用；不携带 Intent。
    pub fn maintenance(schedule: MaintenanceOccurrenceRef) -> Result<Self, TaskError>;

    pub fn source(&self) -> TaskSource;
    pub fn intent(&self) -> Option<&Intent>;
    pub fn contract(&self) -> Option<&ContractRef>;
    pub fn policy_grant(&self) -> Option<&PolicyGrantRef>;
    pub fn maintenance_schedule(&self) -> Option<&MaintenanceOccurrenceRef>;
}
```

**`PolicyGrantRef` / `MaintenanceOccurrenceRef`**：

```rust
/// policy 来源的不透明授权引用。**不是** §331 的 `Decision`，也不是 `PolicyDecision`。
/// 两个字段都非空；P7D 不解释它们的语义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyGrantRef {
    issuer: String,
    grant_id: String,
}

impl PolicyGrantRef {
    pub fn new(issuer: impl Into<String>, grant_id: impl Into<String>)
        -> Result<Self, TaskError>;
    pub fn issuer(&self) -> &str;
    pub fn grant_id(&self) -> &str;
}

/// maintenance schedule 来源的不透明 occurrence 引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOccurrenceRef {
    schedule_id: String,
    occurrence_id: String,
}

impl MaintenanceOccurrenceRef {
    pub fn new(schedule_id: impl Into<String>, occurrence_id: impl Into<String>)
        -> Result<Self, TaskError>;
    pub fn schedule_id(&self) -> &str;
    pub fn occurrence_id(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    NotReady { status: IntentStatus },
    WrongKind { kind: IntentKind },
    InvalidContract { id: String, version: u32 },
    MissingProductId,
    EmptyPolicyGrant,
    EmptyMaintenanceRef,
}

// `SupervisorError::source()` 要返回 `TaskError`，故它必须实现 `Display`／`Error`（M5）。
impl std::fmt::Display for TaskError { /* 逐臂：拒绝原因，不含敏感内容 */ }
impl std::error::Error for TaskError {}
```

**四个构造器的校验规则**（逐条；J3／J4／J5）：

| 构造器 | `kind` | `status` | `contract` | 其他 |
|---|---|---|---|---|
| `authorized_intent` | `!= Product` | `Ready` | `id` 非空、`version >= 1` | 无 |
| `product_drive` | `== Product` | `Ready` | `id` 非空、`version >= 1` | `product_id: Some` 且非空 |
| `policy` | — | — | — | `issuer`／`grant_id` 非空 |
| `maintenance` | — | — | — | `schedule_id`／`occurrence_id` 非空 |

**`READY` 的口径（写死）**：`READY` 约束**只适用于携带 `Intent` 的两路**——`authorized_intent` 与
`product_drive`；`policy`／`maintenance` 不携带 Intent，也没有状态概念，以非空引用为证据。
切分／任务材料里的「`AuthorizedTask` 只收 `READY`」按这个口径读（切分 §2.3.1 第 2 条明写
「本文只冻结 `product drive` 这一路」，`:134-139`）。

**`authorized_intent` 保留（写死）**：§299 把 `authorized Intent` 列为独立来源；本设计**不**把它
收窄成 Product-only。若未来要求 `AuthorizedTask` 全局只允许 Product Intent，那会删除 §299 的一类
来源，必须另开设计并重开 §299 的读法；不在本设计内留成裁定点（§10 第 1 条）。

**为什么 `AuthorizedTask` 持有整枚 `Intent`**：P7D 需要把 Intent 交给 P7B 的 `decide`（候选集是
`&[Intent]`）、需要 `id`／`status`／`contract_id` 做授权与分派、需要 `product_id` 做 `product drive`
校验。只存 `IntentId`＋`ContractRef` 会让 `next_tasks` 无法确认候选状态，也会逼 P7D 再读一次
Intent 存储（切分 §6.2 第 5 条禁止）。代价：`Intent` 的 `goal`／`priority` 等字段随行；P7D 不解释
`priority`，只原样交给 P7B。

**`Intent` 字段是 `pub`**：调用方可以伪造一枚 `status == READY` 的 Intent 值。`AuthorizedTask` 的
构造器只校验字段，不能证明它来自 P4 的存储或审查流程。**这是切分 §三第 6 条选项 (a) 的已知代价**：
证据是「合同版本引用＋状态」，不是独立可验证的授权。若将来要求独立证据，走选项 (b) 的
`ProductIntentGrant`，由装配方＋第 5 层填；本轮不做（§10 第 3 条）。

**为什么 `policy`／`maintenance` 不携带 Intent**：§299 的这两类来源不是 Intent；P7D 不生成 Intent。
若它们需要一个任务身份，由调用方在 `PolicyGrantRef`／`MaintenanceOccurrenceRef` 里表达；P7D 不建
`TaskId`，因为那会是第三套身份词汇。

## 4.4 `BackgroundSupervisor`、`next_tasks` 与 `decide`

```rust
/// 后台治理入口。**持有每任务的 `SuspensionState` 内存表**（C1）。
/// 不建任务内容表：`AuthorizedTask` 不存；表只存相位／尝试数／截止。
/// 任务终止时由装配方调 `release`；P7D 不推断终止。
#[derive(Debug, Default)]
pub struct BackgroundSupervisor {
    states: HashMap<TaskKey, SuspensionState>,
}

/// 状态表的私有键。**不是公开身份类型**，不出现在公开 API。
/// `IntentId` 没有 `Hash`／`Eq`（`ids.rs:41`；P7B §4.2 也禁止 `HashSet<IntentId>`），
/// 故 Intent 任务用 `as_str()` 的 `String`；policy／maintenance 用其不透明引用。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TaskKey {
    Intent(String),
    Policy { issuer: String, grant_id: String },
    Maintenance { schedule_id: String, occurrence_id: String },
}

impl TaskKey {
    fn for_task(task: &AuthorizedTask) -> Self { /* 按 source 与 accessor 机械派生；见 §5.5 */ }
}

/// `ContinueSession` 的控制值。**不是** P7C 的 `ResourceAction`，也不是相位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueDisposition {
    /// `Running`／`Verifying`／`Evolving`：会话已在运行，交给执行器继续。
    Continue,
    /// `Waiting`／`Suspended`：P7B 的「恢复」分支；**不要求**既有会话。
    Resume,
}

/// `decide` 的结果：只三臂。
#[derive(Debug, Clone)]
pub enum SupervisorDecision {
    /// P7B 选中了一枚 READY Intent，且它有对应的 `AuthorizedTask`。
    Task { task: AuthorizedTask, decision: IntentDecision },
    /// P7B 选中了一枚活跃 Intent；**不迁移状态**。
    /// `disposition` 区分「继续」（要求既有会话）与「恢复」（不要求）。
    ContinueSession {
        intent: IntentId,
        disposition: ContinueDisposition,
        decision: IntentDecision,
    },
    /// P7B 明确 `NoIntent`；**既有会话与后台任务必须保留**。
    NoIntent { decision: IntentDecision },
}

impl SupervisorDecision {
    pub fn intent_decision(&self) -> &IntentDecision;
    pub fn selected_intent(&self) -> Option<&IntentId>;
    pub fn basis(&self) -> &DecisionBasis;
    pub fn continue_disposition(&self) -> Option<ContinueDisposition>;
}

impl BackgroundSupervisor {
    /// 本轮可运行的任务集合。**只过滤，不排序**：保持 `tasks` 的输入序。
    /// - 有 Intent 的任务：只保留「`candidates` 中存在同 id 的候选，且**该候选的当前**
    ///   `status == READY`」的；`task.intent().status` 是构造快照，**不用于过滤**（#19）；
    /// - 候选集中同 id 出现多次时取第一条；重复是调用方契约违反（P7B §4.2 同口径）；
    /// - policy／maintenance 任务：无条件保留（它们不参与 P7B 的 Intent 选择）；
    /// - 不检查资源（资源在 `admit` 里）。
    pub fn next_tasks<'a>(
        &self,
        candidates: &[Intent],
        tasks: &'a [AuthorizedTask],
    ) -> Vec<&'a AuthorizedTask>;

    /// **唯一的选择入口**：调 P7B 的 `IntentScheduler::decide`（P7B §4.5 `:773`）。
    /// P7D 不实现 `IntentSelectionPolicy`，不做第二套候选排序（J7）。
    ///
    /// 映射（J9）：
    /// - `Advance` 到 `READY`：找对应 `AuthorizedTask`；找不到 ⇒ `Err(UnauthorizedAdvance)`；
    /// - `Advance` 到 `WAITING`／`SUSPENDED`：`ContinueSession { disposition: Resume }`；
    ///   **不要求** `existing_sessions`（P7B §5.1 的「恢复」分支）；
    /// - `Advance` 到 `RUNNING`／`VERIFYING`／`EVOLVING`：要求 `existing_sessions` 有该 id；
    ///   无 ⇒ `Err(NoActiveSession)`；`disposition` 为 `Continue`；
    /// - `NoIntent` ⇒ `SupervisorDecision::NoIntent`；
    /// - `Err(IntentDecisionError)` ⇒ `Err(SupervisorError::Decision)`，**不折成 NoIntent**。
    pub fn decide(
        &self,
        inputs: &SchedulerInputs<'_>,
        policy: &dyn IntentSelectionPolicy,
        tasks: &[AuthorizedTask],
        existing_sessions: &[IntentId],
    ) -> Result<SupervisorDecision, SupervisorError>;
}
```

**为什么不是字面的 `HashMap<IntentId, SuspensionState>`**：`IntentId` 没有 `Hash`／`Eq`
（`ids.rs:41`；P7B §4.2 禁止 `HashSet<IntentId>`），加派生会改 P4 的冻结面；且
policy／maintenance 任务没有 `IntentId`，却同样需要 P7C 的每任务状态。故用私有 `TaskKey`：
对 Intent 任务它的值就是 `IntentId::as_str()` 的字符串，对另外两路是它们的不透明引用；
**公开面仍是 `SuspensionState`，不新增身份类型**。

**`next_tasks` 不是排名**：它按 `tasks` 的输入序返回引用；`Vec` 的顺序就是调用方给的顺序。它不读
`SchedulerInputs`、不调 `decide`、不比较 `priority`／`deadline`／`blocking`。Manual 与 Auto 都用它
做候选面（§5.4）；Auto 的选择用 `decide`。**三处都不产生第二套顺序**（J7／J8）。

**`next_tasks` 用当前候选状态，不用快照**（#19）：`AuthorizedTask` 的 `intent.status` 在构造时是
`READY`；P4 之后可能推进它。过滤以 `candidates` 里的同 id 候选为准，避免 Manual 展示已经不可推进
的旧任务。代价：调用方必须把当前候选集传进来；只给 `tasks` 不能完成过滤。

**`decide` 的状态分派不是排名**：它只按候选的 `IntentStatus` 决定「这是新任务还是既有会话」；
不比较两枚候选。P7B 的 `Advance` 已经给出唯一 id；P7D 只做身份查找与错误处理。**P7D 不调用
`apply_intent_transition`**：迁移由装配方／执行器按既有运行时路径做（P7B §5.1 的「或既有运行时
路径」；§11 第 5 条）。J7 的源码面会钉住本 crate 没有 `apply_intent_transition`。

**`ContinueDisposition` 的语义（写死）**：`Waiting`／`Suspended` 走 `Resume`，**不要求**
`existing_sessions`——P7B §5.1 明确这两态是「恢复」，没有会话也要恢复；`Running`／`Verifying`／
`Evolving` 走 `Continue`，要求既有会话，否则 `NoActiveSession`。装配方按 `Resume` 执行
`WAITING/SUSPENDED → RUNNING`，按 `Continue` 把会话交给执行器继续。P7D 自己不做迁移（#20）。

**`NoIntent` 的语义**：表示**本轮没有新的可推进项**；P7D 必须保留既有会话与后台任务，不得据此
清理、关闭或丢弃它们（P7B §5.3 `:1103-1122`）。`SupervisorDecision::NoIntent` 不携带哨兵
`IntentId`；`selected_intent()` 返回 `None`（J9）。

**`Advance` 到 `READY` 但无 `AuthorizedTask`**：这是「P7B 选了一枚没有授权证据的 Intent」。P7D
fail-closed，返回 `UnauthorizedAdvance`，**不**临时构造 `AuthorizedTask`，也**不**把 `Advance` 当成
`NoIntent`。这是 `AuthorizedTask` 作为唯一任务输入面的照片（J9）。

## 4.5 `SessionPlan`、候选与后台摘要

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    Manual,
    Auto,
}

#[derive(Debug, Clone)]
pub enum SessionCandidate {
    /// 既有会话：Manual 展示「继续 Core／继续 LCM」。
    ContinueSession { intent: IntentId },
    /// P7D 的授权任务：Manual 展示，Auto 可能被 P7B 选中。
    Task(AuthorizedTask),
    /// 后台新结果：只展示，不参与排名（§5.4）。
    BackgroundFinding(BackgroundNote),
    /// Manual 的固定选项「开始新对话」。
    NewConversation,
}

/// 一条后台结果摘要。**不透明文本**；不是 `Evidence`、不是 `Claim`。
/// `at_millis` 是**会话时间**：`i64` 毫秒，与 `Intent.created_at`／`updated_at` 同口径；
/// 资源时钟 `now_millis` 是 `u64` 单调毫秒（P7C）。两者不混用；`BackgroundNote` 从不进
/// `admit`／`checkpoint`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundNote {
    label: String,
    at_millis: i64,
}

impl BackgroundNote {
    pub fn new(label: impl Into<String>, at_millis: i64) -> Result<Self, SupervisorError>;
    pub fn label(&self) -> &str;
    pub fn at_millis(&self) -> i64;
}

/// 后台新结果的摘要面：completed＋findings。两数组保持输入序，不排序。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackgroundDigest {
    completed: Vec<BackgroundNote>,
    findings: Vec<BackgroundNote>,
}

impl BackgroundDigest {
    pub fn empty() -> Self;
    pub fn new(completed: Vec<BackgroundNote>, findings: Vec<BackgroundNote>) -> Self;
    pub fn completed(&self) -> &[BackgroundNote];
    pub fn findings(&self) -> &[BackgroundNote];
}

/// 两模式共用的会话计划。**字段私有；唯一构造点是 `plan_manual`／`plan_auto`。**
/// `decision` 是选择的**唯一**表示（C2）：`SupervisorDecision` 的三臂原样存放；
/// `selection()` 只把 Task／Continue 投影出来，`basis()` 委托给 `decision`。
#[derive(Debug, Clone)]
pub struct SessionPlan {
    mode: SessionMode,
    candidates: Vec<SessionCandidate>,
    decision: Option<SupervisorDecision>,
    background: BackgroundDigest,
}

impl SessionPlan {
    pub fn mode(&self) -> SessionMode;
    pub fn candidates(&self) -> &[SessionCandidate];
    /// Auto：P7B 的完整结论（含 `NoIntent` 的 basis）；Manual：`None`。
    pub fn decision(&self) -> Option<&SupervisorDecision>;
    /// 只读投影：Manual／`NoIntent` ⇒ `None`；`Task`／`ContinueSession` ⇒ `Some`。
    pub fn selection(&self) -> Option<&SupervisorDecision>;
    pub fn background(&self) -> &BackgroundDigest;
    /// 委托 `decision().map(SupervisorDecision::basis)`；Manual 恒 `None`。
    pub fn basis(&self) -> Option<&DecisionBasis>;
}
```

**四个字段的理由**：

| 字段 | 作用 | 规范状态 |
|---|---|---|
| `mode` | 区分 Manual／Auto；两模式共用同一形状 | §11 两模式；字段名本设计自定 |
| `candidates` | Manual 的候选展示；Auto 的分析面 | §11 `:584-591`；形状本设计自定 |
| `decision` | Auto 的完整选择结论；Manual 恒 `None`。`selection()`／`basis()` 是它的只读投影，不另存 | §11 `:608-620`；P7B §4.1 `:500`；形状本设计自定 |
| `background` | 「昨晚完成／发现」的摘要 | §11 `:606`；§156 `:395-404`；形状本设计自定 |

**`selection()` 与 `basis()` 不是第二个存储点**：`SessionPlan` 只有 `decision` 一个字段承载选择；
`selection()` 的返回类型就是 `Option<&SupervisorDecision>`（评审 C2 选项①的「只读视图」），不是
第二个 `SessionSelection`；它对 `NoIntent` 返回 `None`（保持「无选择」的旧语义），`decision()` 仍
返回 `Some(NoIntent)` 以便读 basis；`basis()` 直接委托。**删除了评审前的 `SessionSelection` 类型**
（C2；§2.6）。

**不实现 `Default`／`Deserialize`**：`SessionPlan` 的字段私有，唯一构造点是 §4.6 的两个函数；反序列化
会开第二个构造点。`BackgroundDigest` 可以 `Default`（空摘要），因为它不是计划本体。

**`BackgroundNote` 不是 `Evidence`**：它只有 `label` 与 `at_millis`，没有 `EvidenceId`、没有
`CompletionVerdict`、没有充分性判据。若一个后台发现要以「验证结果」进入后续流程，先按 §89 转
`Evidence`（§3.8）；P7D 不定义第二枚 Evidence（J12）。

**`SessionCandidate::Task` 持有 `AuthorizedTask` 的值**：`AuthorizedTask` 的字段私有，`Clone` 只复制
已校验的值；`SessionPlan` 不能因此产生新的 `AuthorizedTask`。**`SessionCandidate::NewConversation`
是固定 UI 选项**，不是任务来源，也不进 `TaskSource`。

## 4.6 `SessionOrchestrator`

```rust
/// §11 的 Session Orchestrator。**持一个 `BackgroundSupervisor`**（因此也持每任务挂起状态）；
/// 每次 Session Activation 调一次 `plan_*`。
#[derive(Debug, Default)]
pub struct SessionOrchestrator {
    supervisor: BackgroundSupervisor,
}

impl SessionOrchestrator {
    pub fn new(supervisor: BackgroundSupervisor) -> Self;

    /// 装配方用同一枚 supervisor 调 `admit`／`checkpoint`／`checkpoint_timed_out`／`release`；
    /// **不要另建第二个 `BackgroundSupervisor`**，否则资源状态表会分叉（C1）。
    pub fn supervisor(&self) -> &BackgroundSupervisor;
    pub fn supervisor_mut(&mut self) -> &mut BackgroundSupervisor;

    /// Manual：只生成候选；`decision` **恒 `None`**；不调 P7B。
    /// 先 `next_tasks(candidates, tasks)` 过滤（#19），再按段序：
    /// existing_sessions（输入序）→ 可见 tasks（输入序）→ findings（输入序）→ NewConversation。
    pub fn plan_manual(
        &self,
        candidates: &[Intent],
        tasks: &[AuthorizedTask],
        existing_sessions: &[IntentId],
        background: BackgroundDigest,
    ) -> SessionPlan;

    /// Auto：调 `BackgroundSupervisor::decide`；`decision` 来自 P7B 的 `Advance`／`NoIntent`，
    /// `NoIntent` ⇒ `selection()` 为 `None` 且保留既有会话；`Err` ⇒ fail-closed。
    pub fn plan_auto(
        &self,
        inputs: &SchedulerInputs<'_>,
        policy: &dyn IntentSelectionPolicy,
        tasks: &[AuthorizedTask],
        existing_sessions: &[IntentId],
        background: BackgroundDigest,
    ) -> Result<SessionPlan, SupervisorError>;
}
```

**同一个 supervisor 实例（C1 的装配约束）**：`SessionOrchestrator` 持有 `BackgroundSupervisor`；
装配方通过 `supervisor_mut()` 调 `admit`／`checkpoint`／`checkpoint_timed_out`／`release`，再通过
`plan_manual`／`plan_auto` 读同一张状态表。**不要**在装配方另建一枚 `BackgroundSupervisor`：两枚
各有自己的 `states`，资源相位会分叉。`SessionOrchestrator` 不复制状态，也不把 `SuspensionState`
交给调用方。

**Manual 不选**：`plan_manual` 不调 `decide`、不读 `SchedulerInputs`、不读 `ResourceArbiter`；它先用
`self.supervisor.next_tasks(candidates, tasks)` 过滤，再把结果按固定段序装进 `SessionPlan`，
`decision = None`。用户的选择发生在 P7D 之外（UI／调用方），P7D 不把「用户选了哪个」编码成第二个
决策面。J10 钉住 `plan_manual` 的源码面没有 `IntentScheduler::decide`。

**Auto 的选择**：`plan_auto` 先调 `supervisor.decide(...)`，再用
`self.supervisor.next_tasks(inputs.candidates, tasks)` 构造与 Manual 相同的候选列表；
`decision = Some(decision)` 原样存放 `SupervisorDecision`（Task／ContinueSession／NoIntent 三臂），
**不再翻译成第二个 `SessionSelection`**。`selection()` 对 `NoIntent` 返回 `None`，`basis()` 委托给
`decision`。**`SessionPlan` 的两模式形状完全相同**，只有 `mode`／`decision` 不同（J10）。
**`plan_auto` 不恢复会话**：§11 `:620` 的「然后直接恢复」由装配方／执行器按
`SupervisorDecision::ContinueSession.disposition` 执行；P7D 只产出计划，不执行、不迁移状态。

**Auto 不读后台结果做排名**：`plan_auto` 把 `background` 原样放进 `SessionPlan`，不把
`BackgroundNote` 塞进 `SchedulerInputs`，也不因 finding 改变 `decision`。若要让 finding 影响选择，
调用方可以在构造 `SchedulerInputs.user_focus` 时表达（`UserFocus::Intent`／`Area`，P7B §4.2 `:617`），
但那是调用方的输入，不是 P7D 的排名（§10 第 7 条）。

## 4.7 资源消费：`admit` 与 `checkpoint`

```rust
impl BackgroundSupervisor {
    /// 把一枚任务交给 P7C 的 `ResourceArbiter::direct`（P7C §4.11 `:827`）。
    /// 每任务状态在 `self.states` 里：首次调用插入 `SuspensionState::new()`，之后复用同一枚。
    /// **返回 P7C 的结论原样**；P7D 不解释动作、不把 `None` 当 `Run`。
    /// 调用方负责周期调用 `ResourceArbiter::observe`；P7D 不读 OS、不读 `ResourceObservation`。
    pub fn admit(
        &mut self,
        task: &AuthorizedTask,
        arbiter: &ResourceArbiter,
        now_millis: u64,
    ) -> Result<ResourceDecision, SupervisorError>;

    /// 执行一次 checkpoint 并推进 P7C 的相位（三个出口，写死）：
    /// 1. 端口 `Ok` 且在截止前 ⇒ `checkpoint_succeeded` ⇒ `Ok(Suspended)`；
    /// 2. 端口 `Ok` 但已过截止 ⇒ `checkpoint_succeeded` 返回 `CheckpointTimedOut`，
    ///    相位回 `Throttled` ⇒ `Err(Resource(CheckpointTimedOut))`；**晚到成功不报 `Suspended`**；
    /// 3. 端口 `Err` ⇒ `checkpoint_failed`，相位回 `Throttled` ⇒
    ///    `Err(Checkpoint { error: 端口错误, phase: Throttled })`。
    /// 调用前无状态 ⇒ `Err(NoSuspensionState)`；有状态但相位不是 `Checkpointing` ⇒
    /// `Err(Resource(NotCheckpointing { phase }))`，**不调端口**。
    /// **不得停在 `Checkpointing`，不得在失败时报 `Suspended`**（P7C §5.6）。
    pub fn checkpoint(
        &mut self,
        task: &AuthorizedTask,
        port: &dyn CheckpointPort,
        now_millis: u64,
    ) -> Result<SuspensionPhase, SupervisorError>;

    /// 定时器出口：调 P7C 的 `checkpoint_timed_out`；相位回 `Throttled`。
    /// **没有 `Ok` 路径**：无状态 ⇒ `NoSuspensionState`；错相位 ⇒
    /// `Resource(NotCheckpointing)`；其余 ⇒ `Resource(CheckpointTimedOut)`。
    /// 调用方用 `checkpoint_deadline` 安排何时调它。
    pub fn checkpoint_timed_out(
        &mut self,
        task: &AuthorizedTask,
        now_millis: u64,
    ) -> Result<(), SupervisorError>;

    /// P7C 的 `checkpoint_deadline_millis()`；无状态或不在 `Checkpointing` ⇒ `None`。
    pub fn checkpoint_deadline(&self, task: &AuthorizedTask) -> Option<u64>;

    /// 只读当前相位；无状态 ⇒ `None`。
    pub fn suspension_phase(&self, task: &AuthorizedTask) -> Option<SuspensionPhase>;

    /// 任务终止时丢弃它的状态；返回此前是否存在。P7D 不推断终止，由装配方调。
    pub fn release(&mut self, task: &AuthorizedTask) -> bool;
}

/// checkpoint 的执行端口。**P7D 不实现它**；装配方／执行器按任务的真实状态实现。
pub trait CheckpointPort {
    fn checkpoint(&self, task: &AuthorizedTask) -> Result<(), CheckpointError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointError(String);

impl CheckpointError {
    pub fn new(message: impl Into<String>) -> Self;
    pub fn message(&self) -> &str;
}

// `SupervisorError::source()` 要返回 `CheckpointError`，故它必须实现 `Display`／`Error`（M5）。
impl std::fmt::Display for CheckpointError { /* 输出 message；不裁剪 */ }
impl std::error::Error for CheckpointError {}
```

**`TaskSource` → `WorkloadDeclaration` 的唯一映射**（私有函数；J13）：

```rust
fn declaration_for(task: &AuthorizedTask) -> WorkloadDeclaration {
    WorkloadDeclaration {
        user_process: false,
        foreground: false,
        maintenance: matches!(task.source(), TaskSource::MaintenanceSchedule),
    }
}
```

- **`user_process` 恒 false**：P7D 只治理后台任务，不拥有用户直接操作的外部进程。
- **`foreground` 恒 false**：P7D 的任务是后台任务；前台会话的「用户在等」不在本块。若将来要表达，
  由 P7D 的设计新增一个显式事实输入，不在这里从 `TaskSource` 猜（§10 第 11 条）。
- **`maintenance` 只对 `MaintenanceSchedule` 为 true**：这是 P7C §7.1 `:1203-1218` 建议的映射；
  `ProductDrive` 是背景工作，不是维护计划。
- **P7D 不读 `PriorityTier`／`AssemblyView`／`ResourceObservation`**：P7C 的四档优先级只对装配方；
  P7D 只传 `WorkloadDeclaration` 三布尔（J13）。P7D 不启用 `assembly` 特性。

**`admit` 的返回值是 P7C 的 `ResourceDecision`**：P7D 不再造一个 `TaskControl` 类型，避免第二套动作
词汇。调用方按 `action()`：
- `Some(Suspend)` ⇒ 调 `checkpoint`；
- `Some(Throttle)` ⇒ 按执行器自己的语义降速（P7C 不给量化，P7D 也不给；§10 第 9 条）；
- `Some(Run)`／`Some(Resume)` ⇒ 继续／恢复；
- `None` ⇒ **不动作**，不是 `Run`（P7C §4.7 `:655-656`）。

**`checkpoint` 的三个出口（写死）**：P7C 的 `direct` 只把相位推进到 `Checkpointing`；P7D 的
`checkpoint` 才调端口并回报结果：

1. **端口 Ok + 截止前**：`state.checkpoint_succeeded(now)` ⇒ `Ok(Suspended)`；这是唯一进入
   `Suspended` 的路径。
2. **端口 Ok + 截止后（晚到成功）**：`checkpoint_succeeded` 返回 `Err(CheckpointTimedOut)`，
   相位已回 `Throttled`；P7D 返回 `Err(SupervisorError::Resource(CheckpointTimedOut))`，
   **不报 `Suspended`**。
3. **端口 Err**：`state.checkpoint_failed(now)` 返回 `CheckpointFailed`，相位回 `Throttled`；
   P7D 返回 `Err(SupervisorError::Checkpoint { error: 端口错误, phase: Throttled })`。
4. **定时器**：调用方按 `checkpoint_deadline()` 到期调 `checkpoint_timed_out()`；相位回
   `Throttled`，返回 `Err(Resource(CheckpointTimedOut))`。P7D 不建线程，定时器由装配方驱动。

**错相位与晚到事件**：`checkpoint` 在调用端口前先看 `suspension_phase(task)`；不是
`Checkpointing` ⇒ `Err(Resource(NotCheckpointing { phase }))`，**端口不被调用**。若定时器已先把
相位推到 `Throttled`，随后才返回的端口结果不会再写回 `Suspended`，只会落上面的 `NotCheckpointing`
臂。**P7D 不保存 checkpoint 内容**，不实现重启恢复（§10 第 9 条）。

**状态表的所有权与生命周期（C1）**：`BackgroundSupervisor` 持 `states: HashMap<TaskKey,
SuspensionState>`；每个任务一枚，不共享、不复制。`TaskKey` 是私有键：Intent 任务用
`IntentId::as_str()`（`IntentId` 没有 `Hash`／`Eq`），policy／maintenance 用其不透明引用的字段。
表是**内存表**，不是数据库表：重启后相位全部丢失（§10 第 9 条的已知缺口）。任务终止时装配方调
`release(task)`；P7D 不推断终止，也不做 TTL／淘汰。`admit` 首次见任务时插入
`SuspensionState::new()`；同一任务后续 `admit`／`checkpoint` 复用同一枚。

## 4.8 §171 触发点：`SynthesisIo` 与 `SynthesisTriggerPoint`

```rust
/// §171 三个记忆合成调用的端口。**P7D 定义 trait，装配方实现**。
/// 实现方的契约：把三个方法原样转到 `continuum_memory` 的
/// `begin_synthesis`／`apply_synthesis`／`fail_synthesis`（`synthesis.rs:108`／`:306`／`:217`），
/// 并持有 `Tx<'_>`。P7D 不知道事务，也不重定义 `SynthesisTrigger`。
pub trait SynthesisIo {
    fn begin_synthesis(&self, request: &SynthesisRequest) -> Result<(), MemoryError>;
    fn apply_synthesis(
        &self,
        job: &SynthesisJobId,
        proposal: &SynthesisProposal,
        occurred_at: i64,
    ) -> Result<SynthesisReport, MemoryError>;
    fn fail_synthesis(
        &self,
        job: &SynthesisJobId,
        reason: &str,
        occurred_at: i64,
    ) -> Result<(), MemoryError>;
}

/// §171 的触发面。**无状态；不持阈值、不取时钟、不做空闲判定、不重试。**
#[derive(Debug, Default, Clone, Copy)]
pub struct SynthesisTriggerPoint;

impl SynthesisTriggerPoint {
    pub fn begin_synthesis(
        &self,
        io: &dyn SynthesisIo,
        request: &SynthesisRequest,
    ) -> Result<(), SupervisorError>;

    pub fn apply_synthesis(
        &self,
        io: &dyn SynthesisIo,
        job: &SynthesisJobId,
        proposal: &SynthesisProposal,
        occurred_at: i64,
    ) -> Result<SynthesisReport, SupervisorError>;

    pub fn fail_synthesis(
        &self,
        io: &dyn SynthesisIo,
        job: &SynthesisJobId,
        reason: &str,
        occurred_at: i64,
    ) -> Result<(), SupervisorError>;
}
```

**三个函数只做三件事**：转发、把 `MemoryError` 包成 `SupervisorError::Memory`、原样返回结果。它们
**不**构造 `SynthesisRequest`（调用方给）、**不**生成 `SynthesisProposal`（P6C 的 proposal 由模型／人
产生）、**不**决定何时调用（调用方／P10）、**不**重试、**不**取时钟（`occurred_at` 由调用方给）。

**为什么是端口（评审已裁定）**：`begin_synthesis` 的签名是
`begin_synthesis(tx: &Tx<'_>, request: &SynthesisRequest)`（`crates/continuum-memory/src/synthesis.rs:108`），
要字面直调就必须命名 `continuum_persist::Tx<'_>`，从而新增 `continuum-persist` 边；切分 §三 P7D 行
只允许该边「仅当会话／后台作业落库」（`:193`）。**评审裁定（2026-10-19）选端口**：P7D 定义调用面，
事务句柄留在装配方；P7D 仍直接依赖 `continuum-memory` 的类型（`SynthesisRequest`／`SynthesisJobId`／
`SynthesisProposal`／`SynthesisReport`／`MemoryError`）。切分 §2.3.1 第 4 条已同步为「P7D 定义调用面，
装配方持 `Tx`；何时调用归 P10／调用方」（§11 第 7 条）。**本设计不再把端口称作偏差**；若未来改选字面
直调，才需要加 `continuum-persist` 边并把 `SynthesisIo` 换成接收 `&Tx<'_>` 的三个自由函数。

**`SynthesisTrigger` 不重定义**：P7D 不出现
`enum SynthesisTrigger`、不提供 `parse`、不新增第五个时机。四个时机由 P6C 冻结
（`synthesis.rs:64`）；P7D 只经 `SynthesisRequest.trigger` 传递（J14）。

## 4.9 错误类型

```rust
#[derive(Debug)]
pub enum SupervisorError {
    Task(TaskError),
    Decision(IntentDecisionError),
    UnauthorizedAdvance { intent: IntentId },
    NoActiveSession { intent: IntentId },
    UnsupportedAdvance { intent: IntentId },
    NoSuspensionState { source: TaskSource },
    Resource(SuspensionError),
    Checkpoint { error: CheckpointError, phase: SuspensionPhase },
    Memory(MemoryError),
    InvalidBackgroundNote,
}
```

- **`Task`**：`AuthorizedTask` 的拒绝（§4.3）。
- **`Decision`**：P7B 的 `PolicyFailed`／`InvalidAdvance`；P7D **不折成 NoIntent**。
- **`UnauthorizedAdvance`**：P7B 选中 READY Intent，但没有对应 `AuthorizedTask`。
- **`NoActiveSession`**：P7B 选中 `Continue` 分支的活跃 Intent，但 `existing_sessions` 没有它；
  `Waiting`／`Suspended` 的 `Resume` 分支不会落这里（§5.3）。
- **`UnsupportedAdvance`**：P7B 返回了 P7D 不认识的候选状态。**防御臂，理论上不可达**：P7B 的
  `is_advanceable` 已把 `Advance` 限制在六个可推进状态；J9 不为它拍运行期照片（M9）。
- **`NoSuspensionState`**：在 `admit` 之前调 `checkpoint`／`checkpoint_timed_out`（或状态被
  `release` 后仍调用）。**调用方契约违反**；防御臂，不写回任何相位。
- **`Resource`**：P7C `direct`／`checkpoint_*` 的 `SuspensionError`。
- **`Checkpoint`**：端口失败；相位已经回 `Throttled`（P7C §5.6）。
- **`Memory`**：`SynthesisIo` 透传的 `MemoryError`。
- **`InvalidBackgroundNote`**：`BackgroundNote::new` 的空标签；**只有这一条触发路径**（J12 的
  空标签负例）。

`Display`／`Error` 手写（`std` only）；`source()` 对 `Task`／`Decision`／`Resource`／`Memory`／
`Checkpoint` 返回内层错误（`TaskError`／`CheckpointError` 的 `Display`／`Error` 见 §4.3／§4.7；
其余内层类型已实现）；`NoSuspensionState` 等无内层错误的臂返回 `None`。**不派生 `Clone`**：
`MemoryError` 不是 `Clone`（`src/error.rs:8`）。

## 4.10 可见性、构造点与不变量

- **私有字段**：`AuthorizedTask`、`PolicyGrantRef`、`MaintenanceOccurrenceRef`、`SessionPlan`、
  `BackgroundNote`、`BackgroundDigest`、`CheckpointError`。没有 `pub` 字段；没有 `Default`／
  `Deserialize`（`BackgroundDigest` 的 `Default` 是空摘要，不是计划）。
- **唯一构造点**：`AuthorizedTask` 只由四个构造器产出；`SessionPlan` 只由 `plan_manual`／
  `plan_auto` 产出；`BackgroundNote` 只由 `new` 产出；`CheckpointError` 只由 `new` 产出。
- **`TaskSource` 可以公开构造**：它是标签，不是任务；单独的 `TaskSource` 不能构造 `AuthorizedTask`
  （J2）。
- **不变量**：
  1. `AuthorizedTask.source()` 与 `evidence` 变体匹配：`Intent` 只对应 `AuthorizedIntent`／
     `ProductDrive`；`Policy` 只对应 `Policy`；`Maintenance` 只对应 `MaintenanceSchedule`。
  2. 有 Intent 的 `AuthorizedTask` 的 `intent.status == Ready`。
  3. `SessionPlan.mode == Manual` ⇒ `decision.is_none()`（因而 `selection()`／`basis()` 都是 `None`）；
     `mode == Auto` 才有 `decision`；`selection()` 对 `NoIntent` 返回 `None`，但 `decision()` 仍有值。
  4. `SessionPlan` 的 `candidates` 不排序；段序固定。
  5. `BackgroundDigest` 的每个 note 的 `label` 非空。
  6. `BackgroundSupervisor` 的 `states` 每个键对应一枚 `SuspensionState`；`suspension_phase(task) ==
     None` ⇔ 没有该键；`checkpoint_deadline(task).is_some()` ⇒ 该任务的相位是 `Checkpointing`。
- **不实现**：`Serialize`／`Deserialize`；**生产依赖**没有 `serde`；没有 `tokio`／执行器依赖。
  `serde_json` 只是 `[dev-dependencies]` 的负例工具（J11），不进生产面。`SuspensionState` 的相位是
  P7C 的内存值，P7D 不序列化它（§9.1）。

---

# 5. 判定规则

## 5.1 四类来源的判定（§299）

**唯一的判定表**（§4.3 的四个构造器；P7D 内部没有第二个 `match TaskSource` 来决定证据）：

| §299 来源 | 构造器 | 证据 | 判定 |
|---|---|---|---|
| `authorized Intent` | `authorized_intent(intent)` | 整枚 `Intent` | `kind != Product`；`status == Ready`；`contract.id` 非空；`contract.version >= 1` |
| `policy` | `policy(grant)` | `PolicyGrantRef` | `issuer`／`grant_id` 非空；不检查 policy 语义 |
| `maintenance schedule` | `maintenance(schedule)` | `MaintenanceOccurrenceRef` | `schedule_id`／`occurrence_id` 非空；不检查计划语义 |
| `product drive` | `product_drive(intent)` | 整枚 `Intent` | `kind == Product`；`product_id` 非空；`status == Ready`；`contract.id` 非空；`contract.version >= 1` |

**为什么 `authorized Intent` 要求 `kind != Product`**：§299 的 `product drive` 是单独一类，P7A 的
`product_intent_for_handoff` 只交 Product Intent（P7A §4.8 `:1435-1454`）。若允许 Product 走
`authorized_intent`，同一枚 Product Intent 会有两个来源标签，且 `product_drive` 的 `product_id` 校验
可以被绕过。**这是本设计自定的分界**（§2.4 第 1 条）；规范没有说四类互斥。

**为什么 `policy`／`maintenance` 不要求 Intent**：§299 的四类不是「四种 Intent」；policy 和维护计划
可以产生不属于任何 Intent 的后台工作。P7D 只校验引用非空，授权判断在第 5 层（切分 §2.3.1 第 2 条
`:137-139`）。

## 5.2 `AuthorizedTask` 的校验顺序

对 `product_drive(intent)`：

1. `intent.kind == IntentKind::Product`，否则 `Err(WrongKind { kind })`。
2. `intent.product_id` 是 `Some` 且 `as_str()` 非空，否则 `Err(MissingProductId)`。
3. `intent.status == IntentStatus::Ready`，否则 `Err(NotReady { status })`。
4. `intent.contract_id.id.as_str()` 非空，否则 `Err(InvalidContract { id, version })`。
5. `intent.contract_id.version >= 1`，否则 `Err(InvalidContract { .. })`。
6. 通过 ⇒ `AuthorizedTask { source: ProductDrive, evidence: Intent(intent) }`。

对 `authorized_intent(intent)`：

1. `intent.kind != Product`，否则 `Err(WrongKind { kind })`。
2. `intent.status == Ready`，否则 `Err(NotReady { status })`。
3. contract 的两条同上。
4. 通过 ⇒ `AuthorizedTask { source: AuthorizedIntent, evidence: Intent(intent) }`。

**顺序的意义**：先判 `kind`／`product_id`，再判 `status`，最后判 contract。这样
`status == Running` 的 Product Intent（直接构造，代表 `PLANNING→SUSPENDED→RUNNING` 的状态结果）在
`product_drive` 上稳定落 `NotReady`，不会因为 contract 缺失而先落 `InvalidContract`——负例的可观察臂
是固定的（J3）。**顺序是本设计自定**（§2.4 第 3 条），规范只给三个条件的合取。

**`COMPLETED` / `EVOLVING`**：`NotReady`。**`FAILED` / `CANCELLED`**：`NotReady`。**`PROPOSED` /
`SPECIFYING` / `PLANNING` / `REVIEW`**：`NotReady`。只有 `READY` 通过。

## 5.3 Supervisor 的选择（复用 P7B）

`BackgroundSupervisor::decide` 的步骤：

1. 调 `continuum_intent_scheduler::IntentScheduler::decide(inputs, policy)`（P7B §4.5 `:773`）。
2. `Err(e)` ⇒ `Err(SupervisorError::Decision(e))`；**不**把 `PolicyFailed`／`InvalidAdvance` 当
   `NoIntent`（P7B §5.1 `:1071-1072`）。
3. `Ok(NoIntent { basis })` ⇒ `SupervisorDecision::NoIntent { decision }`；既有会话与后台任务保留。
4. `Ok(Advance { intent, .. })`：
   - 在 `inputs.candidates` 中按 `id.as_str()` 找到候选（P7B 保证存在且可推进）；
   - `status == Ready`：在 `tasks` 中按 `task.intent().id.as_str()` 找**第一个**匹配的
     `AuthorizedTask`；无 ⇒ `Err(UnauthorizedAdvance { intent })`；
   - `status` 是 `Waiting`／`Suspended`：`ContinueSession { disposition: Resume }`；
     **不查** `existing_sessions`（P7B §5.1 的恢复分支）；
   - `status` 是 `Running`／`Verifying`／`Evolving`：在 `existing_sessions` 中按 `as_str()` 找；
     无 ⇒ `Err(NoActiveSession { intent })`；`disposition` 为 `Continue`；
   - 其他状态 ⇒ `Err(UnsupportedAdvance { intent })`（防御臂）。
5. 返回 `Task`／`ContinueSession`。

**「第一个匹配」不是排名**：`tasks` 的顺序是调用方给的；重复任务的元数据不一致是调用方契约违反，
P7D 不定义选哪一枚（P7B §4.2 `:640-644` 对重复候选有同样的口径）。P7D 不比较 `priority`／
`deadline`／`blocking`；这些只在 P7B 的 `SchedulerInputs` 里。

**`AuthorizedTask` 是构造时的快照**：它的 `intent.status` 在构造时是 `READY`；之后 P4 可能把它推进到
别的状态。`decide` 的分派以 `SchedulerInputs.candidates` 的**当前**状态为准，不以
`AuthorizedTask.intent.status` 重新判候选；若当前状态是 `Waiting`／`Suspended`，`Advance` 走
`ContinueSession { Resume }`；若是 `Running`／`Verifying`／`Evolving`，走 `ContinueSession { Continue }`
并要求既有会话。`AuthorizedTask` 不再被重新校验，也不因状态变化失效；它是「这枚 Intent 在构造时
有过授权证据」的记录。

**`Advance` 到 `READY` 但无任务**：这是「策略选了没有授权证据的 Intent」。**不**把 `Advance` 当
`NoIntent`；**不**为它构造一个 `AuthorizedTask`；**不**放弃既有会话。返回 `Err`，调用方保留现状
（fail-closed）。

**`Advance` 到 `Continue` 分支但无会话**：P7B 的候选集应只包含 P7D 确有会话的 `Evolving`（P7B §2.4
`:224-227`）；`Running`／`Verifying` 也可能有既有会话。若无，P7D 不能凭空继续，返回
`NoActiveSession`。`Waiting`／`Suspended` 的 `Resume` 分支**不**落这里。这是本设计自定
（§2.4 第 20 条）。

## 5.4 SessionPlan 的组成

**Manual**（`plan_manual`）：

1. `let visible = self.supervisor.next_tasks(candidates, tasks);`（#19 的过滤；保持 `tasks` 输入序）。
2. `existing_sessions` 按输入序 ⇒ `SessionCandidate::ContinueSession { intent }`。
3. `visible` 按输入序 ⇒ `SessionCandidate::Task(task.clone())`。
4. `background.findings()` 按输入序 ⇒ `SessionCandidate::BackgroundFinding(note.clone())`。
5. 末尾追加 `SessionCandidate::NewConversation`。
6. `decision = None`（因而 `selection()`／`basis()` 都是 `None`）；`background` 原样放入。

**Auto**（`plan_auto`）：

1. 调 `supervisor.decide(...)`；`Err` ⇒ 返回 `Err`（不生成计划）。
2. `let visible = self.supervisor.next_tasks(inputs.candidates, tasks);`，再用与 Manual 相同的
   四步构造 `candidates`（existing → visible → findings → NewConversation，段序相同）。
3. `decision = Some(decision)`：`SupervisorDecision::Task`／`ContinueSession`／`NoIntent` **原样**
   存入；不再翻译成 `SessionSelection`，也不再单独存 `basis`。
4. `SupervisorDecision::NoIntent { decision }` ⇒ `selection()` 为 `None`，但 `decision()` 仍是
   `Some(NoIntent)`、`basis()` 仍有 P7B 的 basis；**既有会话与 `background` 原样保留**（I3）。
5. `background` 原样放入；`basis()` 只在 Auto 有值（委托 `decision()`）。

**固定段序不是排名**：段与段之间的顺序是 UI 的固定分区（继续／新任务／发现／新对话），不是
「哪个更值得」的比较；段内保持调用方输入序。P7D 不 sort、不 min／max、不按 `priority` 排。
**Manual 的 `decision` 恒 `None`**：用户选择在 P7D 之外；P7D 不把用户选择编码成第二个决策面。
**Auto 的 `decision` 只来自 P7B**：P7D 不读 `background` 来改选择。

**`SessionPlan` 的 `basis()`**：Manual 为 `None`；Auto 委托 `decision().map(SupervisorDecision::basis)`，
即 P7B 的 `DecisionBasis`。`DecisionBasis` 是不透明 `String`（P7B §4.1 `:507`）；P7D 可以展示，
但**不得按内容分支**（P7B §5.1 `:1073-1074`）。`NoIntent` 也保留 basis（I3）。

## 5.5 资源与 checkpoint

**`admit`**：

1. `let key = TaskKey::for_task(task)`；`self.states.entry(key).or_insert_with(SuspensionState::new)`。
2. `declaration_for(task)` 按 §4.7 的唯一映射填三个布尔。
3. `arbiter.direct(declaration, &mut state, now_millis)`。
4. `Ok(decision)` 原样返回；`Err(e)` ⇒ `SupervisorError::Resource(e)`。
5. P7D **不**把 `action == None` 解释为 `Run`；调用方必须按 `None` 不动作。
6. P7D **不**读 `decision.summary()` 做决策；summary 只是 P7C 的 AI／后台暴露面，P7D 可以把
   `resource_pressure`／`workload_class` 展示给用户，但不得据此重新排名。

**`checkpoint`**（三出口；I4）：

1. 取状态：没有该任务的键 ⇒ `Err(NoSuspensionState { source })`（调用方没先 `admit`，或已
   `release`）。
2. 相位不是 `Checkpointing` ⇒ `Err(Resource(NotCheckpointing { phase }))`；**不调端口**。
3. `port.checkpoint(task)`：
   - `Ok(())` ⇒ `state.checkpoint_succeeded(now_millis)`：
     - `Ok(phase)` ⇒ `Ok(phase)`（唯一进入 `Suspended` 的路径）；
     - `Err(CheckpointTimedOut)` ⇒ `Err(Resource(CheckpointTimedOut))`（**晚到成功**；相位已回
       `Throttled`，不报 `Suspended`）；
     - `Err(NotCheckpointing { phase })` ⇒ `Err(Resource(NotCheckpointing { phase }))`（防御）。
   - `Err(port_error)` ⇒ `state.checkpoint_failed(now_millis)`：返回 `CheckpointFailed` 时相位已回
     `Throttled`，P7D 返回 `Err(Checkpoint { error: port_error, phase: Throttled })`；
     若返回 `NotCheckpointing { phase }` ⇒ `Err(Resource(NotCheckpointing { phase }))`（防御）。
4. **定时器出口**：调用方按 `checkpoint_deadline(task)` 到期调 `checkpoint_timed_out(task, now)`；
   `state.checkpoint_timed_out(now)` 把相位回 `Throttled`，P7D 返回
   `Err(Resource(CheckpointTimedOut))`（无状态／错相位分别落 `NoSuspensionState`／
   `Resource(NotCheckpointing)`）。**P7D 不建线程**；定时器由装配方驱动。
5. **不得停在 `Checkpointing`**：端口失败与超时都必须回到 `Throttled`（P7C §5.6）。
6. P7D **不**在失败时重试；重试策略归 P7C 的 `attempts`／P10（§10 第 9 条）。P7D 只做一次
   checkpoint 调用；若调用方要重试，由它再次调 `admit` 得到新的 `Suspend`。

**P7D 持有资源状态（C1，写死）**：`SuspensionState` 由 `BackgroundSupervisor` 按任务持有并传入
`ResourceArbiter::direct`（与 P7C §4.8 `:680`／`:714` 的「P7D 持有」一致），键是私有的
`TaskKey`：Intent 任务用 `IntentId::as_str()`（`IntentId` 没有 `Hash`／`Eq`），policy／maintenance
用其不透明引用。**这是内存表，不是数据库表**：零迁移、零持久化；重启后相位丢失（§10 第 9 条的
已知缺口）。表不存任务内容、不存 checkpoint 数据；任务终止时装配方调 `release`。P7D 不共享
两枚任务的状态，也不把状态交给调用方按 `&mut` 传回。

## 5.6 §171 触发的边界

**P7D 定义调用面**（评审已裁定；切分 §2.3.1 第 4 条已同步）：

1. `SynthesisTriggerPoint::begin_synthesis`：把调用方给的 `SynthesisRequest` 原样转给 `SynthesisIo`。
2. `SynthesisTriggerPoint::apply_synthesis`：把调用方给的 `SynthesisProposal` 原样转给 `SynthesisIo`。
3. `SynthesisTriggerPoint::fail_synthesis`：把调用方给的 `reason` 原样转给 `SynthesisIo`。

**P7D 不做**：

- 不做「session 结束后」的自动判定；不监听 session 生命周期。
- 不做「系统空闲」的时长判定；不取时钟、不建定时器。
- 不做「大量项目状态改变」的数量判定。
- 不做「用户主动要求」的 UI 入口。
- 不做重试、退避、失败恢复、作业恢复。
- 不生成 `SynthesisJobId`（调用方给）。
- 不生成 `SynthesisProposal`（P6C 的 proposer 不在本块；P6C §8.5 明写不建 proposer）。
- 不把 `SynthesisTrigger` 的四个时机映射成新的枚举。

**四个时机的归属**：`SynthesisTrigger` 由 P6C 冻结（`synthesis.rs:64`）；P7D 只经
`SynthesisRequest.trigger` 传递。阈值／频率／重试／触发策略归 P10（切分 §6.2 第 10 条 `:409`）。
P7D 的 J14 只证明「触发点存在且不越界」，不证明「何时触发」——后者在 P10。

## 5.7 P6D 合并门的 rebaseline 步骤（唯一路径）

**选 rebaseline，不退役。** 所有权：切分 §一 1.2（`:55`）与 §一 1.3（`:72`）已把 P6D 合并门在
P7 起转给 P7D；P6D §7.3.5（`:1069`）的「协调者」是收件人，不是执行者。P7D 执行本节的五处同改，
合并评审时由协调者确认执行者（§10 第 5 条）。理由：

1. P6D 的并集门保护的是「既有 crate 的行不被顺手改宽」；退役会丢掉这层历史快照。
2. `every_crate_depends_only_on_its_allowed_set` 只做「表 ↔ 实际依赖」的逐对相等，抓不到「两边
   一起改宽」；P6D §7.5 已说明这层保护的必要性。
3. Rebaseline 到 P7 前的 31 行，让 P7 的并集也有同一张照片；P7D 是最后一个落地块，正好收口。
4. 退役路径只在「协调者认为 P7 之后不再需要历史快照」时采用；本设计不选。

**步骤**（同一批改动；不得分批）：

1. **生成新 fixture**：从 `c89d839` 的 `ALLOWED`（即 P7A／B／C 加行之前的 31 行）生成
   `crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt`。格式照旧：`crate:` 空集合，
   `crate:dep1,dep2` 非空；**31 行数据**；文件头写「P7 合并门，P8 起重新基线化或退役」。
   **不得**从 P6 fixture 追加；**不得**包含任何 P7 新 crate。
2. **删除** `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`（旧基线不留第二份）。
3. **改名与改常量**：
   - `pre_p6_allowed` → `pre_p7_allowed`，`include_str!("fixtures/p7_pre_allowed.txt")`；
   - `P6_EXPECTED_ROWS` → `P7_EXPECTED_ROWS`，四行（§9.2 的表）；
   - `p6_names` → `p7_names`；
   - `p6_union_adds_only_its_own_rows` → `p7_union_adds_only_its_own_rows`。
4. **改两个 `27`**：解析器的 `assert_eq!(rows.len(), 27, ...)`（`:615`）→ `31`；断言 1 的
   `assert_eq!(baseline.len(), 27, ...)`（`:790`）→ `31`。两处**同批**改；只改一处会在另一处先红。
5. **改断言 2**：`P6_EXPECTED_ROWS.len() == 5`（`:799`）→ `P7_EXPECTED_ROWS.len() == 4`。
6. **改注释**：`:779-782` 的「P6 合并门」「27 行」改成「P7 合并门」「31 行」；`:551-573` 的所有权
   注释改成 P7D；`:578` 的文档改成 P7 前基线。
7. **跑**：`cargo test -p continuum-runtime --test dependency_direction` 在**含 P7 四块全部 crate**
   的树上绿。单块树只算部分绿；P7D 的 J15 要求最终树。

**断言 3 的行数自检**（`:812-819`）会检查 `current.len() == workspace.len()`；P7 后 workspace 35，
`ALLOWED` 必须 35 行。**断言 8**（`:882-895`）会检查新增 subject 恰是四个 P7 crate。**断言 5**
（`:837-854`）会检查 runtime 行增量 ⊆ P7 名字；本设计不改 runtime 行，故增量应为空集。

---

# 6. 跨块接缝

| 块 | 接缝 | P7D 给出 | P7D 不依赖 | 状态 |
|---|---|---|---|---|
| **P7B** | Auto 的「最值得继续」调 `IntentScheduler::decide`；`Advance` 按候选状态分派；`NoIntent` 保留既有会话 | §4.4 的 `decide`；§5.3 的映射 | 不依赖 P7D 的反向边 | P7B §4.1／§5.1 已冻；本设计对着写 |
| **P7C** | `admit` 调 `ResourceArbiter::direct`（状态取自 P7D 的每任务内存表）；`checkpoint` 走成功／失败／定时器三出口；`TaskSource` → `WorkloadDeclaration` 三布尔 | §4.7；§5.5 | 不读 `PriorityTier`／`AssemblyView`／`ResourceObservation` | P7C §7.1 `:1182-1201` 已给义务；「P7D 持有 `SuspensionState`」已落实（C1） |
| **P7A** | `product drive` 按值收 `READY` Product Intent；只交值或 id；P7D 不读 Product 记录 | `AuthorizedTask::product_drive(intent)`；§5.2 | 不登记 `continuum-product` 边 | P7A §4.8 `:1435-1454`；按值交接 |
| **P6C** | `SynthesisIo` 的实现转调 `begin_synthesis`／`apply_synthesis`／`fail_synthesis`；`SynthesisTrigger` 不重定义 | §4.8；§5.6 | 不依赖 `continuum-persist`（事务在实现方） | P6C §8.6 `:1101-1109`；评审裁定端口，切分 §2.3.1 第 4 条已同步（§10 第 4 条） |
| **P4** | `Intent`／`IntentStatus`／`ContractRef` 只读；候选集 `&[Intent]`；不直读 `intent` 表、不调 `apply_intent_transition` | §4.3、§4.4 | 不写 `intent` 表 | 切分 §6.2 第 5 条；J5／J19 |
| **P6D** | 合并门 rebaseline：31 行 fixture、四行 P7 期望、两个 `27` → `31`、断言 2 `5` → `4` | §5.7、§9.2 | 不在 P6 fixture 追加行 | P6D §7.3.5；切分 §一 1.2／1.3 已把执行者转给 P7D，P6D 的「协调者」是收件人（§10 第 5 条） |
| **装配方 `continuum-runtime`** | 实现 `SynthesisIo`／`CheckpointPort`；周期调 `ResourceArbiter::observe`；经 `SessionOrchestrator::supervisor_mut()` 共用同一枚 `BackgroundSupervisor`；按 `checkpoint_deadline` 驱动超时；任务终止时调 `release`；执行 `ready → running`／`waiting／suspended → running` 的迁移；提供候选 `&[Intent]` | §4.6、§4.7、§4.8；§5.3 | 不依赖 `continuum-runtime` | 实现不在 P7D；本轮不登记 runtime 边 |

**P7B 的接缝细节**：P7D 不实现 `IntentSelectionPolicy`；它接受调用方给的 `&dyn IntentSelectionPolicy`
（可以是 P7B 的 `BaselineIntentPolicy`，也可以是装配方注入的策略）。**P7D 自己实现一个新的
`IntentSelectionPolicy` 就是第二个排名**（P7B §5.1 `:1079-1081`）。`decide` 的 `Err` 必须 fail-closed，
不能当 `NoIntent`（P7B §5.1 `:1071-1072`）。

**P7C 的接缝细节**：P7D 不读 `PriorityTier`／`AssemblyView`；`WorkloadDeclaration` 只带三个布尔，
不带 §300 的档名。`admit` 返回 `ResourceDecision` 原样；P7D 不把 `action == None` 当 `Run`。
**P7D 按任务持有 `SuspensionState`（内存表，C1）**；`checkpoint` 的三出口与失败回退按 P7C
§5.5／§5.6；P7D 不建第二套 `Suspended` 状态机，也不把状态交回调用方。

**P7A 的接缝细节**：P7A 的 `product_intent_for_handoff` 只返回 `READY`；P7D 只收这个值。P7A 的
drive gate（`PostShippableImprovement`＋`ContinuousProductDrive` 关闭 ⇒ `None`）在 P7A 内；P7D 看不到
开关，也不读 Product 记录（P7A §4.8 `:1447-1450`）。若 P7D 需要读 Product 记录或合同内容才能判定
授权，那是**一条要报协调者的新边**（切分 §三第 6 条 `:382-383`），本设计不登记。

**P6C 的接缝细节**：`SynthesisIo` 是 P7D 定义的端口，不是 P6C 的第二个作业；实现方必须原样转调
三个函数，不复制准入判据（P6C 的 G0–G6 在 `begin_synthesis` 内）。P7D 不生成 proposal、不做
七步算法、不读 `memory_synthesis_job` 表。`SynthesisTrigger` 的四个时机由 P6C 冻结；P7D 不新增
第五个、不 `parse`、不把它与 `TaskSource` 合并。

**P6D 的接缝细节**：P7D 是合并门的执行者（切分 §一 1.2／1.3 已转给 P7D；P6D §7.3.5 的「协调者」
是收件人）；P7A／B／C 不在 P6 fixture 追加行。P7C 的 J1(ii) 依赖 P7D 先完成 rebaseline
（P7C §8.4 第 10 条）；P7D 的 J15 是最终树的照片。

---

# 7. 判据与照片

## 7.1 判据表（每条挂规范节）

**切分 §五 P7D 的三条完成判据对应**：判据 1（任务来源闭集＋`AuthorizedTask` 的 Intent 两路只收
`READY`）由 **J1＋J2＋J3** 覆盖；判据 2（会话选择不另造排名）由 **J7＋J10** 覆盖；判据 3（§171 只提供
触发点）由 **J14** 覆盖。其余 J4–J6、J8–J9、J11–J13、J15–J22 是本设计自报的补充判据；**§7.4 没有
P7D 的判据**，本设计不修改 §7.4（J20）。

| # | 判据 | 规范节 | 观测形态 |
|---|---|---|---|
| **J1** | `TaskSource` 恰四值；无第五类、无 `ModelSuggested`、无 `parse`／`FromStr`；`as_str` 四字面量逐字 | §299 `:1894-1901`；切分 §五 P7D 完成判据 1 | 源码面（枚举定义＋`grep`）＋编译期（穷尽 `match`）＋运行期（四个 `as_str` 断言） |
| **J2** | **裸 `TaskSource` 不能构造 `AuthorizedTask`**：字段私有；没有 `new(source, evidence)`；四个构造器是唯一入口 | 切分 §2.3.1 第 2 条 `:138` | 编译期（`trybuild`：结构体字面量／`AuthorizedTask::new` 编不过）＋源码面 |
| **J3** | **`product drive` 只收 `READY`**：十三态表逐状态；直接构造的 `RUNNING` Intent（字段 `pub`；等价于 `PLANNING→SUSPENDED→RUNNING` 的状态结果）必须 `Err(NotReady)`；`COMPLETED`／`EVOLVING` 也拒绝。**照片只证 `RUNNING` 被拒绝；「可绕过 `REVIEW`」由 §3.7 的 P4 迁移表证明** | §223 `:172-205`；切分 §三第 6 条 `:226-237`；§八第 5 条 `:483-485` | 运行期（十三组）＋源码面（构造器的 `status == Ready`） |
| **J4** | `authorized Intent` 只收非 Product 的 `READY` Intent；Product 走 `product_drive` | §299；§223；切分 §2.3.1 第 2 条 | 运行期（`WrongKind`）＋源码面 |
| **J5** | `contract.id` 非空且 `version >= 1`；`product_id` 非空；错误臂分别可观察 | §222 `:148`；§224；切分 §三第 6 条 `:229-230` | 运行期（四类负例） |
| **J6** | `policy`／`maintenance` 只收非空引用；没有 Intent；来源匹配由构造器类型保证 | §299；切分 §2.3.1 第 2 条 | 运行期（空串负例）＋编译期（`trybuild`：不能把 `PolicyGrantRef` 传给 `maintenance`，反之亦然） |
| **J7** | **不另写排名**：`src/` 无 `impl IntentSelectionPolicy`；无 `sort`／`sort_by`／`min`／`max`／`cmp`／`partial_cmp`／`BinaryHeap`；`decide` 只调 `IntentScheduler::decide` 一次 | 切分 §五 P7D 完成判据 2；P7B §5.1 `:1079-1081` | 源码面（`grep` 非注释行）＋运行期（假策略选第二枚，P7D 返回第二枚） |
| **J8** | `next_tasks` 保持 `tasks` 输入序；只保留「候选当前 READY」的 Intent 任务（不用快照）；policy／maintenance 无条件保留；不读 `SchedulerInputs` | §299；§11 Manual 候选；§2.4 #19 | 运行期（顺序＋快照冲突断言）＋源码面（无 sort） |
| **J9** | `decide` 映射：`Advance`＋READY＋有任务 ⇒ `Task`；`Advance`＋RUNNING／VERIFYING／EVOLVING＋有会话 ⇒ `ContinueSession { Continue }`；`Waiting`／`Suspended`（无会话也） ⇒ `ContinueSession { Resume }`；`Advance`＋Continue 分支无会话 ⇒ `NoActiveSession`；`Advance`＋READY＋无任务 ⇒ `UnauthorizedAdvance`；`NoIntent` ⇒ `NoIntent` 且保留既有会话；策略 `Err` ⇒ `Err` | P7B §4.1 `:500`、§5.1 `:1052-1081`；§302；§2.4 #20 | 运行期（八组假策略；`UnsupportedAdvance` 防御臂不拍） |
| **J10** | **两模式共用 `SessionPlan`**：同一类型、四个字段／六个访问器；Manual 的 `decision` 恒 `None` 且不调 `decide`；Auto 的 `decision` 来自 P7B（含 `NoIntent`）；`NoIntent` 时既有会话仍在 candidates、background 不变；`plan_manual` 源码面不出现 `IntentScheduler::decide`，且确实调 `next_tasks` | §11 `:576-620`；切分 §五 P7D 完成判据 2 | 运行期（两模式同一类型＋NoIntent 保留＋Manual 过滤）＋源码面（`plan_manual` 函数体） |
| **J11** | `SessionPlan` 字段私有（四字段）；无 `Default`／`Deserialize`；唯一构造点是 `plan_manual`／`plan_auto`；`candidates` 段序固定且段内不排序 | §11；§2.4 第 6、7 条 | 编译期（`trybuild` 结构体字面量；`serde_json` 负例）＋源码面 |
| **J12** | **后台摘要不是 `Evidence`**：`BackgroundNote` 只有 `label`／`at_millis`；`BackgroundNote::new("", _)` ⇒ `Err(InvalidBackgroundNote)`；`src/` 无 `Evidence`／`CompletionVerdict`／`continuum_verify`；`BackgroundDigest` 不参与排名 | §89 `:939-945`；切分 §四第 4 条 | 源码面＋运行期（digest 原样进 plan＋空标签负例） |
| **J13** | **资源消费与状态持有**：`admit` 调 `ResourceArbiter::direct` 且复用同一任务的状态；`declaration_for` 的映射逐项；`action == None` 原样返回；`checkpoint` 三出口（成功／晚到成功／端口失败）＋定时器出口；`release` 丢状态；`src/` 无 `impl CheckpointPort`（端口实现不在本块）；`src/` 无 `PriorityTier`／`AssemblyView`／`ResourceObservation`；不启用 `assembly` | P7C §4.10 `:759`、§4.11 `:827`、§5.6、§7.1 `:1182-1201`；§300；§2.4 #18／#24 | 源码面＋运行期（四档动作对照＋三出口＋状态复用／释放） |
| **J14** | **§171 只提供触发点**：`SynthesisTriggerPoint` 三个方法各转发一次；`src/` 无 `impl SynthesisIo`（实现不在本块）；`src/` 无 `Duration`／`Instant`／`SystemTime::now`／`retry`／`interval`／`sleep`／`idle` 判定；`SynthesisTrigger` 不重定义；无阈值 | §171 `:931-938`；切分 §五 P7D 完成判据 3、§6.2 第 10 条；§10 第 4 条 | 源码面＋运行期（假 `SynthesisIo` 计数） |
| **J15** | **P6D 合并门 rebaseline**：`p7_pre_allowed.txt` 31 行；`P7_EXPECTED_ROWS` 四行；解析器与断言 1 的 `27` → `31`；断言 2 的 `5` → `4`；测试改名；旧 P6 fixture 删除；最终树 `cargo test -p continuum-runtime --test dependency_direction` 绿 | 切分 §四第 1 条 `:259-264`；P6D §7.3.5 | 运行期（依赖门）＋源码面（fixture／常量／两个 `27`） |
| **J16** | `ALLOWED` 的 `continuum-supervisor` 行恰五项（`arbiter`／`canonical`／`intent-scheduler`／`memory`／`semantics`）；既有行不因本块增枚；`cargo test -p continuum-runtime --test dependency_direction` 绿 | 切分 §四第 1、5 条 | 运行期（依赖门）＋源码面（`ALLOWED` 行） |
| **J17** | **零表、零事件**：`Cargo.toml` 无 `continuum-persist`；`src/` 无 `Migration`／`Tx`／`INSERT`／`UPDATE`／`load_intent`／`list_children`／`apply_intent_transition`／`Event`／`EventType`；`runtime_migrations()`／`expected_migrations()`／`startup.rs` 三处不变；不占 201 | 切分 §四第 2 条 `:265-276` | 源码面＋运行期（runtime 三处无本 crate） |
| **J18** | **不读用户行为／不调模型**：`src/` 无 `std::process`／`Command`／`window`／`keyboard`／`screen`／`open files`／`process_name`／`pid`；无 `continuum_provider`／`ModelProvider`／`InvokeRequest`／`continuum_runtime` | §12 `:665`；§13 `:697-706`；切分 §五 P7D 行 | 源码面＋`Cargo.toml` |
| **J19** | **不建第二套 Intent 存储／查询**：`src/` 无 `load_intent`／`list_children`／`save_intent`／`apply_intent_transition`；候选集是 `&[Intent]`；不读 Product 记录（无 `continuum_product`） | 切分 §6.2 第 5 条；§三第 6 条 | 源码面＋`Cargo.toml` |
| **J20** | **不宣称 §7.4 覆盖 P7D**：本设计 §3.6 明写「§7.4 没有 P7D」；`SessionPlan`／`AuthorizedTask` 不带 `Shippable`／`ProductReadiness` | 切分 §七第 12 条；§7.4 `:460-467` | 文档面＋源码面（无这些类型） |
| **J21** | **不建第二个 Resource Scheduler**：`src/` 无 `rank`／`place`／`RoutingRequest`／`PlacementRequest`／`parallelism`；`ResourceDirective` 不在本 crate | §301 `:1925-1935`；§304 `:1973-1982`；切分 §6.2 第 12 条 | 源码面＋`Cargo.toml` |
| **J22** | **Product Intent 按值消费**：`AuthorizedTask::product_drive` 只收 `Intent`；`src/` 无 `continuum_product`／`Product`／`Milestone`／`ProductReadiness`；``product_id` 只从 `Intent.product_id` 读 | 切分 §三第 6 条 `:213-218`；P7A §4.8 | 源码面＋运行期（`product_id` 缺失拒绝） |

**J1–J3 是切分 §五 P7D 明确要求的三条完成判据**；J4–J22 是本设计自报的补充判据。**§7.4 没有
P7D 的判据**；本设计不修改 §7.4，也不把 J 编号冒充成 §7.4 的第五条（J20）。

## 7.2 照片清单（每条判据的观测形态：运行期／编译期／源码面）

| # | 照片（具体断言或命令） | 形态 | 拍不到的部分 |
|---|---|---|---|
| **J1-a** | `src/task.rs` 的 `TaskSource` 定义恰四变体；`match` 穷尽（无 `_`）编译通过；四个 `as_str()` 分别等于 `"authorized_intent"`／`"policy"`／`"maintenance_schedule"`／`"product_drive"`；`grep -rn "ModelSuggested\|TaskSource::Model\|UserRequested\|SystemIdle" crates/continuum-supervisor/src` 零命中（非注释） | 源码面＋编译期＋运行期 | 外部代码仍可构造 `TaskSource` 标签；由 J2 兜住 |
| **J1-b** | `grep -rn "impl FromStr for TaskSource\|fn parse.*TaskSource" crates/continuum-supervisor/src` 零命中 | 源码面 | 字符串仍可出现在 `PolicyGrantRef` 里；那不是来源 |
| **J2** | `trybuild` 两个 `compile_fail`：①`AuthorizedTask { source: TaskSource::Policy, evidence: ... }` ⇒ E0451（字段私有）；②`AuthorizedTask::new(TaskSource::Policy, ...)` ⇒ E0599（无此函数） | 编译期 | 内部模块仍可构造；本 crate 只有四个构造器 |
| **J3** | 十三态表：`product_drive` 对 `PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW`／`READY`／`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`COMPLETED`／`EVOLVING`／`FAILED`／`CANCELLED` 各一枚 Intent；只有 `READY` ⇒ `Ok`，其余 ⇒ `Err(NotReady)`。**另加**：直接构造 `Intent { status: Running, .. }`（`Intent` 字段是 `pub`；不经 `apply_intent_transition`，也不新增 `continuum-persist` dev 边）；`product_drive` ⇒ `Err(NotReady { status: Running })`。**该照片只证明 RUNNING 被拒绝**；「RUNNING 可绕过 REVIEW」由 §3.7 的 P4 迁移表证明 | 运行期 | 谁把 `PROPOSED` 推进到 `READY`（P4 审查流程，不在本块；§3.7）；「从未过 REVIEW」的路径不由本照片走一遍 |
| **J4** | `product_drive` 对 `IntentKind::Task` ⇒ `Err(WrongKind)`；`authorized_intent` 对 `IntentKind::Product` ⇒ `Err(WrongKind)`；`authorized_intent` 对 `Task`＋`READY`＋有效 contract ⇒ `Ok` | 运行期 | — |
| **J5** | `product_drive` 对 `contract.id == ""` ⇒ `Err(InvalidContract)`；`version == 0` ⇒ `Err(InvalidContract)`；`product_id == None`／`Some("")` ⇒ `Err(MissingProductId)`；`authorized_intent` 的同两类 contract 负例 | 运行期 | 合同是否真实存在（P7D 不读合同） |
| **J6** | `PolicyGrantRef::new("", "g")`／`new("i", "")` ⇒ `Err(EmptyPolicyGrant)`；`MaintenanceOccurrenceRef::new("", "o")`／`new("s", "")` ⇒ `Err(EmptyMaintenanceRef)`；`policy(grant).source() == Policy`；`maintenance(schedule).source() == MaintenanceSchedule`；`policy(...).intent() == None`；`trybuild` 两个 `compile_fail`：`AuthorizedTask::maintenance(policy_grant)`／`AuthorizedTask::policy(maintenance_ref)` ⇒ E0308（来源类型不匹配） | 运行期＋编译期 | grant 的真实性（第 5 层） |
| **J7-a** | `grep -rn "impl IntentSelectionPolicy\|fn choose\|enum IntentDecision\|struct IntentScheduler\|struct SchedulerInputs" crates/continuum-supervisor/src` 零命中（非注释） | 源码面 | 测试里的假策略不算生产实现 |
| **J7-b** | `grep -rn "\.sort\|\.sort_by\|\.min\|\.max\|cmp(\|partial_cmp\|BinaryHeap\|BTreeSet" crates/continuum-supervisor/src` 零命中（非注释） | 源码面 | 注释里可能提到；限定非注释行 |
| **J7-c** | 假策略 `choose` 返回候选集**第二枚**的 `Advance`；`decide` 返回的 `selected_intent()` 是第二枚（按 `as_str()`），不是第一枚 | 运行期 | 只证明 P7D 不重排；P7B 策略本身仍可能错 |
| **J8** | `tasks = [t1, t2, t3, t4]`（四枚的**构造快照**都是 `READY`；`AuthorizedTask` 的构造器只收 READY），`candidates` 含 t1／t3／t4，其中 t4 的**当前**状态是 `PLANNING`，t2 不在候选集；`next_tasks` ⇒ `[t1, t3]`（按 `tasks` 输入序，不按候选序；t4 证明用当前候选状态而非快照；t2 证明不在候选集的丢弃）；policy／maintenance 任务无条件保留 | 运行期 | 调用方仍可自己排序 |
| **J9** | 八组假策略：①`Advance(READY, 有任务)` ⇒ `Task`；②`Advance(RUNNING, 有会话)` ⇒ `ContinueSession { Continue }`；③`Advance(RUNNING, 无会话)` ⇒ `NoActiveSession`；④`Advance(READY, 无任务)` ⇒ `UnauthorizedAdvance`；⑤`NoIntent` ⇒ `NoIntent` 且 `selected_intent() == None`、`basis()` 非空；⑥策略 `Err` ⇒ `SupervisorError::Decision`，不是 `NoIntent`；⑦`Advance(WAITING, 无会话)` ⇒ `ContinueSession { Resume }`；⑧`Advance(SUSPENDED, 无会话)` ⇒ `ContinueSession { Resume }`。`UnsupportedAdvance` 是防御臂，不拍（§4.9） | 运行期 | P7B 策略内部错误（P7B 负责） |
| **J10-a** | `plan_manual` 与 `plan_auto` 都返回 `SessionPlan`；`mode()` 分别为 `Manual`／`Auto`；Manual 的 `decision() == None`／`selection() == None`／`basis() == None`；Auto 的 `decision()` 与假策略的 `Advance` 一致，`selection()` 与 `ContinueDisposition` 可从 `decision()` 读出 | 运行期 | — |
| **J10-b** | 读 `src/session.rs` 的 `plan_manual` 函数体：不出现 `IntentScheduler::decide`／`decide(`／`SchedulerInputs`；出现一次 `next_tasks`；只做候选段拼接与 `SessionPlan` 构造 | 源码面 | 调用方仍可自己调 `decide`；那是 Auto |
| **J10-c** | Manual 过滤照片：`tasks` 含一枚当前候选为 `PLANNING` 的任务（构造快照 READY）与一枚不在候选集的任务；`plan_manual(candidates, tasks, …)` 的 `candidates()` 不含这两枚，只含候选当前 READY 的任务 | 运行期 | 调用方仍可自己构造 `SessionPlan`？不能；字段私有（J11） |
| **J10-d** | **`NoIntent` 保留照片**：`existing_sessions = [id]`、`background` 含一条 completed＋一条 finding；假策略返回 `NoIntent { basis: "none" }`；`plan_auto` ⇒ `decision()` 是 `Some(NoIntent)`、`selection() == None`、`basis().as_str() == "none"`、`candidates()` 仍含 `ContinueSession { id }`、`background()` 与输入逐项相等 | 运行期 | P7B 的 basis 语义（P7B 负责） |
| **J11** | `trybuild`：`SessionPlan { mode: ..., candidates: ..., decision: ..., background: ... }` 缺字段 ⇒ E0063 / 字段私有 ⇒ E0451；`SessionPlan::default()` ⇒ E0599（无 `Default`）；`serde_json::from_str::<SessionPlan>` ⇒ E0277（无 `Deserialize`；`serde_json` 在 crate 的 `[dev-dependencies]`，不进 `ALLOWED`）；读 `src/session.rs` 的字段声明与 `impl`：字段恰四个、访问器恰六个（`mode`／`candidates`／`decision`／`selection`／`background`／`basis`） | 编译期＋源码面 | — |
| **J12** | `BackgroundNote` 的字段恰 `label`／`at_millis`；`BackgroundNote::new("", 0)` ⇒ `Err(InvalidBackgroundNote)`；`grep -rn "Evidence\|CompletionVerdict\|continuum_verify" crates/continuum-supervisor/src` 零命中（非注释）；`BackgroundDigest` 只进 `SessionPlan.background`，不进 `SchedulerInputs` | 源码面＋运行期 | 调用方仍可把 finding 转成 Intent；那是 P5a／装配方 |
| **J13-a** | `declaration_for` 四来源表：`MaintenanceSchedule` ⇒ `{false,false,true}`；其余 ⇒ `{false,false,false}`；`supervisor.admit(task, &arbiter, now)` 对 `Critical` 压力下的 `BackgroundRuntime` ⇒ `ResourceDecision.action == Some(Suspend)`、`phase == Checkpointing`、`suspension_phase(task) == Some(Checkpointing)`；`High` 下 `BackgroundRuntime` ⇒ `Some(Throttle)`、`Throttled`；`Low` 下 `BackgroundRuntime` ⇒ `None`、`Running`；`UserWorkload` 不出现（P7D 不产生该声明） | 运行期 | 实际降速多少（执行器；§10 第 9 条） |
| **J13-b** | `grep -rn "impl CheckpointPort\|enum ResourceAction\|struct ResourceDecision\|struct SuspensionState\|struct ResourceArbiter" crates/continuum-supervisor/src` 零命中（假实现在 `tests/`）；`grep -rn "PriorityTier\|AssemblyView\|ResourceObservation\|assembly" crates/continuum-supervisor/src crates/continuum-supervisor/Cargo.toml` 零命中（`src/` 非注释）；`Cargo.toml` 的 `continuum-arbiter` 无 `features = ["assembly"]` | 源码面 | 同一构建图统一启用特性时仍可命名；P9 的规范级强制 |
| **J13-c** | **三出口＋定时器**：①新任务 `admit`（Critical）后 `checkpoint_deadline(task) == Some(now + 60_000)`；`checkpoint(task, port_ok, now+1_000)` ⇒ `Ok(Suspended)`、`suspension_phase == Suspended`、deadline `None`。②新任务 `admit` 后在 `now+60_000` 调 `checkpoint(port_ok)`（**晚到成功**）⇒ `Err(Resource(CheckpointTimedOut))`、相位 `Throttled`，**不是 `Suspended`**。③新任务 `admit` 后 `checkpoint(port_err)` ⇒ `Err(Checkpoint { error, phase: Throttled })`、相位 `Throttled`。④新任务 `admit` 后 `checkpoint_timed_out(task, now+60_000)` ⇒ `Err(Resource(CheckpointTimedOut))`、相位 `Throttled`。⑤未 `admit` 就 `checkpoint` ⇒ `Err(NoSuspensionState)`；已 `Suspended` 再 `checkpoint` ⇒ `Err(Resource(NotCheckpointing { phase: Suspended }))` 且端口计数不增 | 运行期 | 调用方完全不轮询时相位不会自动推进（P7C 的库边界） |
| **J13-d** | **状态表复用／释放**：同一任务连续两次 `admit`（第一次已 `Suspend`）第二次 ⇒ `action == None`、相位仍 `Checkpointing`（证明复用同一枚状态，未重置）；`release(task)` ⇒ `true`，`suspension_phase(task) == None`；再次 `admit` ⇒ 新状态从 `Running` 出发（Critical 下再次 `Suspend`）。源码面：`src/supervisor.rs` 有 `states: HashMap<TaskKey, SuspensionState>`；`checkpoint_timed_out` 调 P7C 的同名方法；没有把 `SuspensionState` 作为公开参数 | 运行期＋源码面 | 跨重启恢复（§10 第 9 条） |
| **J14** | 假 `SynthesisIo` 用 `Cell<usize>` 计数：`begin_synthesis` 恰 1；`apply_synthesis` 恰 1；`fail_synthesis` 恰 1；`src/trigger.rs` 的三个函数体各只有一次转发调用；`grep -rn "impl SynthesisIo" crates/continuum-supervisor/src` 零命中；`grep -rn "Duration\|Instant\|SystemTime::now\|thread::sleep\|retry\|interval\|idle\|threshold" crates/continuum-supervisor/src` 零命中（非注释） | 运行期＋源码面 | 何时调用（P10） |
| **J15** | 见 §9.2：`wc -l p7_pre_allowed.txt` 数据 31 行；`grep -n "P6_EXPECTED_ROWS\|p6_names\|p6_union\|p6_pre_allowed" dependency_direction.rs` 零命中；`grep -n "rows.len(), 31\|baseline.len(), 31\|P7_EXPECTED_ROWS.len(), 4"` 各一处；`cargo test -p continuum-runtime --test dependency_direction` 绿 | 运行期＋源码面 | 故意同时改 fixture 与测试仍可过；P6D §7.3.5 的自保护边界 |
| **J16** | `cargo test -p continuum-runtime --test dependency_direction` 绿；`ALLOWED` 的 `continuum-supervisor` 行恰五项；既有行不出现本 crate 名（除 runtime 行若装配方加） | 运行期 | 传递依赖不在逐对断言里 |
| **J17** | `Cargo.toml` 无 `continuum-persist`；`src/` 无 `Migration`／`Tx`／`load_intent`／`list_children`／`save_intent`／`apply_intent_transition`／`INSERT INTO intent`／`UPDATE intent`／`Event`／`EventType`；`runtime_migrations()`／`expected_migrations()`／`startup.rs` 无本 crate；`cargo test -p continuum-runtime --test migrations` 与 `--test startup` 绿 | 源码面＋运行期 | — |
| **J18** | `grep -rn "std::process\|Command::new\|window\|keyboard\|screen\|process_name\|pid\|continuum_provider\|ModelProvider\|InvokeRequest\|continuum_runtime" crates/continuum-supervisor/src` 零命中（非注释）；`Cargo.toml` 无这些依赖 | 源码面 | 调用方仍可把用户数据放进 `BackgroundNote.label`；那是调用方 |
| **J19** | `grep -rn "load_intent\|list_children\|save_intent\|apply_intent_transition\|continuum_product\|ProductReadiness\|Milestone" crates/continuum-supervisor/src` 零命中（非注释）；`Cargo.toml` 无 `continuum-product` | 源码面 | 调用方仍可从库里读 Intent 再传进来；本块不拦 |
| **J20** | 本设计 §3.6 明写「§7.4 没有 P7D」；`grep -rn "Shippable\|ProductReadiness\|CompletionVerdict" crates/continuum-supervisor/src` 零命中 | 文档面＋源码面 | 文档措辞会漂；实现期复核 |
| **J21** | `grep -rn "rank(\|place(\|RoutingRequest\|PlacementRequest\|parallelism\|ResourceDirective" crates/continuum-supervisor/src` 零命中（非注释） | 源码面 | — |
| **J22** | `product_drive` 的 `product_id` 来自 `Intent.product_id`；`grep -rn "continuum_product\|Product::\|Milestone\|load_product" crates/continuum-supervisor/src` 零命中；`product_id == None` ⇒ `MissingProductId` | 源码面＋运行期 | Product 记录是否存在（P7A） |

## 7.3 变异预告（谁红）

| 变异 | 该红 | 不该红 |
|---|---|---|
| 给 `TaskSource` 加第五变体 `ModelSuggested` | J1（枚举形状）、J18（若接模型） | J3–J6（构造器行为不变） |
| 给 `AuthorizedTask` 加 `pub fn new(source, evidence)` | J2（`trybuild` 不再失败；源码面多一个入口） | J3–J6（字段校验仍可过） |
| `product_drive` 接受 `RUNNING` Intent | J3（直接构造的 `RUNNING` 组） | J4（kind 仍对） |
| `product_drive` 不校验 `product_id` | J5（`None`／空串组）、J22 | J3／J4 |
| `authorized_intent` 接受 Product | J4（`WrongKind` 不出现） | J3（Product 仍 READY） |
| 在 `decide` 里按 `priority` 排候选 | J7（源码面 sort／cmp）、J7-c（假策略选择被覆盖） | J9（状态映射仍对） |
| 在 `plan_manual` 里调 `decide` 并设 `decision` | J10（Manual 不选） | J11（形状仍对） |
| `next_tasks` 用 `task.intent().status` 快照而不是候选当前状态 | J8（t4 组）、J10-c（Manual 过滤） | J9（选择映射不变） |
| 在 `plan_auto` 里把 finding 当选择 | J10（`decision` 不来自 P7B）、J12 | J8（next_tasks 不变） |
| `NoIntent` 分支清空 `candidates`／`background` | J9（⑤）、J10-d（NoIntent 保留） | J11（形状仍对） |
| 把 `SessionSelection`／独立 `basis` 再加回 `SessionPlan` | J10（单一表示）、J11（字段数） | J9（`decide` 不变） |
| `BackgroundNote` 加 `Evidence` 字段 | J12（不是 Evidence） | J11（计划形状不变） |
| `admit` 把 `action == None` 当 `Run` | J13（`None` 不动作） | J5–J6（任务构造不变） |
| 把 `SuspensionState` 留给调用方（`admit`／`checkpoint` 收 `&mut`） | J13-d（源码面／状态复用） | J5–J6 |
| `checkpoint` 失败后报 `Suspended` | J13-c（端口失败／晚到成功） | J3–J6 |
| 超时走 `checkpoint_failed` 而不是 `checkpoint_timed_out` | J13-c（定时器出口） | J3–J6 |
| 在 `SynthesisTriggerPoint` 里加空闲阈值／重试 | J14（无阈值／无重试） | J1（来源不变） |
| 给 P7D 加 `continuum-persist` 并建表 | J17（零表）、J16（`ALLOWED` 行不符） | J3–J6（任务构造仍可跑） |
| 不 rebaseline，直接在 P6 fixture 追加 P7 行 | J15（31 行／断言 1／断言 8） | J16（若 `ALLOWED` 行对，逐对门仍绿） |
| 只改 fixture 不改两个 `27` | J15（解析器或断言 1 先红） | J16 |
| 把 `P7_EXPECTED_ROWS` 写成五行（含 `continuum-context`） | J15（四行计数／断言 8） | J16（`continuum-context` 不在 workspace 时不参与断言 6） |

## 7.4 拍不出照片的地方（据实）

1. **P7B 策略的实现质量**：P7D 只能证明自己不重排、只转调；P7B 的 `IntentSelectionPolicy` 是否
   合理、模型策略是否可用，在 P7B／装配方。**收件人：P7B＋装配方。**
2. **`AuthorizedTask` 的 Intent 值是否来自 P4 的真实存储**：`Intent` 字段是 `pub`，构造器只校验
   字段；P7D 不能证明它过了 `REVIEW`。**这是切分 §三第 6 条选项 (a) 的已知边界**。收件人：协调者＋
   第 5 层。
3. **policy／maintenance grant 的真实性**：P7D 只校验非空；grant 是否由第 5 层签发、是否被撤销，不在
   本 crate。**收件人：第 5 层＋协调者。**
4. **checkpoint 是否可恢复**：P7D 只调端口并回报相位；checkpoint 的内容、存储、重启恢复在
   装配方／执行器。**收件人：装配方＋P1。**
5. **`throttle` 的量化**：P7D 只返回 P7C 的 `Throttle`；降多少并发／速率不在本 crate（P7C 也
   不给）。**收件人：执行器＋P7C。**
6. **「昨晚完成」的跨重启展示**：P7D 零表；`BackgroundDigest` 是调用方给的值。P7D 不能保证
   重启后还能展示。**收件人：装配方＋协调者。**
7. **「低风险 Milestone」的判定**：P7D 只收 `READY` Product Intent，不判风险。**收件人：P7A＋
   规范维护者。**
8. **Auto 的「分析后台新结果」**：P7D 只展示 findings，不因 finding 改变选择；P7B 的 §302 五输入
   没有后台结果项。**收件人：规范维护者＋P7B。**
9. **P7D 的 `SuspensionState` 内存表跨重启丢失**：本设计按 C1 选 (A)，P7D 持每任务内存表；
   重启后相位全部丢失，P7D 不落表、不占 201。恢复策略（重建 `Running` 或从持久化快照恢复）在
   装配方；P7D 不实现。**收件人：装配方＋P1＋协调者。**
10. **P6D 合并门的自保护**：P7D 的 rebaseline 仍是「两个文件＋评审 diff」，挡不住故意同时改；
    这不是防篡改。**收件人：协调者＋合并评审。**

---

# 8. 明确不做的

**这一节是 §1.2 的完整表**；每条给出归属与理由。**不建、不留桩。**

| 不建的东西 | 归属 / 理由 |
|---|---|
| Intent Scheduler 本体、§302 五输入排序、`IntentSelectionPolicy` 实现 | P7B；P7D 只调 `IntentScheduler::decide`（P7B §4.5 `:773`）；J7 |
| Resource Arbiter、`ResourcePressure`／`WorkloadClass`／`PriorityTier`／`AssemblyView` | P7C；P7D 只消费 `ResourceDecision`／`SuspensionState`；J13 |
| §171 阈值、频率、空闲判定、重试、恢复 | P10（切分 §6.2 第 10 条）；P7D 只给触发点；J14 |
| Memory Synthesis 作业本体、proposal 生成、七步算法 | P6C；P7D 只转发 `begin/apply/fail` |
| `intent` 表、第二套 Intent 查询、`apply_intent_transition` | P4；候选集 `&[Intent]`；J17／J19 |
| Product 记录、ProductReadiness、Milestone、Shippable | P7A；P7D 只按值收 `READY` Product Intent；J22 |
| 执行器、进程管理、UI、Planner Agent 本体 | P1／P8／前端；P7D 只产出 `SessionPlan` 与控制值 |
| 模型调用、`InvokeRequest`、`ModelProvider::invoke` | P3G；P7D 不依赖 `continuum-provider`／`continuum-runtime`；J18 |
| 第二套「最值得继续」排名 | 切分 §五 P7D 完成判据 2；P7D 只复用 P7B；J7 |
| 第五类任务来源／「模型建议」 | §299 `:1903`；`TaskSource` 闭集；J1 |
| `Evidence`／`CompletionVerdict` 的第二枚 | P5a；P7D 的 `BackgroundNote` 不是 Evidence；J12 |
| `SynthesisTrigger` 的第二个枚举 | P6C；P7D 不重定义；J14 |
| `TaskId`／`SessionId` 的第三套身份 | P4／P6C；P7D 用 `IntentId`／`PolicyGrantRef`／`MaintenanceOccurrenceRef`；不另造 |
| 表、迁移、事件、审计 | P7D **零数据库表**（`SuspensionState` 只是内存映射，不是表）；不占 201 |
| `kill`／终止进程／`NodeState` 迁移 | §14；P7C 的协作式挂起；P7D 不建 |
| `ResourceDirective`／§304 四问 | P7B 的端口；P7D 不回答；J21 |
| 「§7.4 已覆盖 P7D」的宣称 | §7.4 没有 P7D；本设计自报 J1–J3 等；J20 |

---

# 9. 持久化与共写文件

## 9.1 结论：零表、零迁移、零事件

本块**零表**。理由：

1. `SessionPlan` 是**一次 Session Activation 的计划值**，§11 没有要求持久化；它是 UI 的输入。
2. `AuthorizedTask` 是**本轮任务输入**，由调用方构造；P7D 不拥有任务内容表，只持每任务
   `SuspensionState` 的内存映射（§9.1 第 5 条）。
3. `BackgroundDigest` 是**摘要值**；「昨晚完成」的存储属于执行器／装配方，不是 P7D 的表。
4. §171 的作业记录表属于 P6C（`memory_synthesis_job`／`memory_synthesis_input`，迁移 190）；
   P7D 只调 P6C 的函数，不建表。
5. `SuspensionState` 是 P7C 的内存状态；P7D 按 C1 在 `BackgroundSupervisor` 里持
   `HashMap<TaskKey, SuspensionState>` 的内存表，但**不复制到数据库、不持久化**；重启丢失。

**不占迁移号段**：P7 的 `200–209` 档由 P7A 占 `200`；P7D **不占 201**（若将来要落会话／后台表，
再在同一档顺取 201，并同步三处 runtime 计数）。本设计不改
`runtime_migrations()`（`crates/continuum-runtime/src/main.rs:114`）、
`expected_migrations()`（`crates/continuum-runtime/tests/migrations.rs:37`）、
`tests/startup.rs` 的三处计数（`:26`／`:81`／`:94`）。

## 9.2 P6D 合并门：rebaseline 的具体数据

**新 fixture** `crates/continuum-runtime/tests/fixtures/p7_pre_allowed.txt`：31 行数据，逐行等于
`c89d839` 的 `ALLOWED`（P7 加行之前）。**不含** `continuum-product`／`continuum-intent-scheduler`／
`continuum-arbiter`／`continuum-supervisor`。格式照 P6 fixture：

```
continuum-core:
continuum-events:continuum-core
...
continuum-runtime:continuum-artifact,continuum-budget,...,continuum-workspace
```

**`P7_EXPECTED_ROWS`**（四行；按 P7A→P7B→P7C→P7D 顺序；与各块设计边表逐行对应）：

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

**五处同改**（J15）：

1. fixture：`p6_pre_allowed.txt` 删除，`p7_pre_allowed.txt` 31 行新建；`include_str!` 路径改。
2. `P6_EXPECTED_ROWS` → `P7_EXPECTED_ROWS`（四行）；`p6_names` → `p7_names`。
3. 解析器 `assert_eq!(rows.len(), 27, ...)`（`:615`）→ `31`。
4. 断言 1 `assert_eq!(baseline.len(), 27, ...)`（`:790`）→ `31`。
5. 断言 2 `assert_eq!(P6_EXPECTED_ROWS.len(), 5, ...)`（`:799`）→ `P7_EXPECTED_ROWS.len(), 4`。

**测试改名**：`p6_union_adds_only_its_own_rows` → `p7_union_adds_only_its_own_rows`（`:784`）；
文件头注释（`:779-782`）与 fixture 头（`p6_pre_allowed.txt:1-3`）同步改。**旧 P6 fixture 删除**，
不留第二份基线。

**为什么是 31**：当前 `Cargo.toml` 的 `members` 有 31 个 crate（P6 已落地 `continuum-memory`／
`continuum-trust`／`continuum-protected`／`continuum-learning`；`continuum-context` 仍不存在）；
P7 前的 `ALLOWED` 也是 31 行。P7 后 workspace 35，`ALLOWED` 35；断言 3 的 `current.len() ==
workspace.len()` 仍成立。

**为什么不选退役**：见 §5.7 的四条理由。退役会丢掉「既有行不被顺手改宽」的历史快照；P7D 选择
rebaseline，并明确这是**唯一路径**，不提供第二套「P7D 一律退役」的分支。

## 9.3 共写文件与锁文件

本块碰两处（§2.1）：`Cargo.toml` 的 `members`、`dependency_direction.rs` 的 `ALLOWED`；另加 P6D
合并门的三处（fixture／`P7_EXPECTED_ROWS`／两个 `27` 与断言 2）。**`continuum-runtime` 的 `ALLOWED`
行不动**：P7D 是库，装配方下一轮实现端口时才加行。**`Cargo.lock` 随新 crate 变化**，提交时按显式路径
带上。

---

# 10. 设计问题（逐条具名）

**每条给「缺的是哪一步」与收件人；不用「后续」「长期阶段」兜。**

1. **§299 四类来源的判定与互斥**。§299 只给四个名字（`:1894-1901`），没有说四类如何区分、是否互斥、
   `authorized Intent` 与 `product drive` 的边界。本设计按 `Intent.kind` 分：非 Product 走
   `AuthorizedIntent`，Product 走 `ProductDrive`；policy／maintenance 不携带 Intent。**缺的是
   「四类的判定规则与互斥」这一条规范。收件人：规范维护者＋协调者＋P7A。** 代价：同一枚 Product
   Intent 不能走 `AuthorizedIntent`；若规范认为四类可重叠，本设计的 `WrongKind` 会过严。

2. **`SessionPlan` 的形状没有规范**。§11 只给流程与例子（`:576-620`），没有字段、没有候选顺序、
   没有「选择」的语义。本设计定四字段与固定段序（§4.5、§4.6），选择只由 `decision` 一个字段承载。
   **缺的是「SessionPlan 的字段与两模式差异」这一条规范。收件人：规范维护者＋前端＋P8。** 代价：
   UI 要按固定四字段读；新增字段是破坏性改动；`candidates` 的段序可能不是 UI 想要的。

3. **`AuthorizedTask` 的授权证据只到「合同版本引用＋状态」**。切分 §三第 6 条选选项 (a)
   （`:226-243`）：`contract_id`＋`status`；P7D 不读合同内容、不验证合同存在、不防生产方自报。
   **缺的是「独立可验证的授权证据」这一步；选项 (b) 的 `ProductIntentGrant` 本轮不做。收件人：
   协调者＋第 5 层＋P7A。** 代价：`Intent` 字段是 `pub`，调用方可以伪造 `READY`；P7D 只能按字段
   校验，不能证明审查流程。

4. **§171 的端口化（已裁定）**。`begin_synthesis` 需要 `&Tx<'_>`（`synthesis.rs:108`）；要字面直调
   就必须依赖 `continuum-persist`，而切分 §三 P7D 行只允许该边「仅当会话／后台作业落库」
   （`:193`）。**评审裁定（2026-10-19）选 `SynthesisIo` 端口**：P7D 定义调用面，P7D 仍直接依赖
   `continuum-memory` 的类型，事务句柄留在装配方；切分 §2.3.1 第 4 条已同步为「P7D 定义调用面，
   装配方持 `Tx`；何时调用归 P10／调用方」。**遗留的不是裁定，而是端口实现的契约**（不得绕过
   P6C 的 G0–G6）。收件人：P6C＋装配方。代价：多一个端口实现；字面上不是 P7D 直接调自由函数。

5. **P6D 合并门的所有权措辞（已裁定，待确认执行者）**。本设计选 rebaseline（§5.7、§9.2）：31 行
   fixture、四行 P7 期望、两个 `27` → `31`、断言 2 `5` → `4`。切分 §一 1.2／1.3（`:55`／`:72`）
   已把 P6D 合并门在 P7 起转给 P7D；P6D §7.3.5（`:1069`）的「所有者：P6D（本轮）→ 协调者（P7 起）」
   里，「协调者」是**收件人**，不是执行者。**缺的不是「是否允许改」，而是协调者在合并评审时确认
   P7D 是执行者。收件人：协调者＋P6D。** 代价：rebaseline 是五处同改，漏一处会先红；退役会丢掉
   历史快照。

6. **§7.4 的四条判据不覆盖 P7D**。四条分别落在 P7A（判据 1、2、4）与 P7C（判据 3）；P7D 没有规范
   判据（切分 §七第 12 条）。本设计自报 J1–J3（切分 §五要求）与 J4–J22（补充），**不修改 §7.4**。
   **缺的是「P7D 是否需要在《工程》§7.4 有一条正式判据」这一步。收件人：协调者＋《工程》维护者。**
   代价：P7D 的验收依赖本设计的自报判据；跨文档不一致时没有规范锚。

7. **Auto 的「分析后台新结果」没有进入 P7B 的输入**。§11 的 Auto 流程包含「分析后台新结果」
   （`:604-608`），但 P7B 的 §302 五输入是 `priority`／`deadline`／`blocking`／`user focus`／
   `background suitability`（P7B §4.2 `:564-590`），没有「后台结果」。本设计只把 `BackgroundDigest`
   展示，不因 finding 改变选择。**缺的是「后台结果如何影响选择」这一条规范或 P7B 输入。
   收件人：规范维护者＋P7B＋P7D。** 代价：Auto 可能忽略「昨晚发现的问题」；若要让 finding 生效，
   装配方须把它投影成 `UserFocus` 或生成 Intent。

8. **§156 的「低风险 Milestone」没有判据**。§156 `:392` 只说「继续完成低风险 Milestone」，
   `Intent` 没有 `risk` 字段，P7A 的 `product_intent_for_handoff` 也不返回风险。本设计收所有
   `READY` Product Intent。**缺的是「低风险」的定义与字段。收件人：P7A＋规范维护者。** 代价：
   P7D 可能把非低风险 Milestone 纳入后台；若风险判断在 P7A，P7D 不需要改。

9. **`SuspensionState` 的持有（已选 A）与 `throttle` 的量化**。P7C §4.8 `:680`／`:714` 说「P7D
   持有」`SuspensionState`；本设计选 **(A) P7D 真的持有**：`BackgroundSupervisor` 里
   `HashMap<TaskKey, SuspensionState>` 内存表，零数据库表；对 Intent 任务等价于
   `HashMap<IntentId, SuspensionState>`（`IntentId` 没有 `Hash`／`Eq`，键用 `as_str()` 字符串）。
   重启丢失列为已知缺口（§7.4 第 9 条）。P7C 的 `Throttle` 仍不给量化；P7D 不给。**遗留的是
   「throttle 的量化执行」一条。收件人：执行器＋P7C。** 代价：若 throttle 要量化，要新增执行端口；
   P7D 不替执行器定速率。

10. **后台结果的跨重启持久化**。§156 `:395-404` 要求用户重开时看到「昨晚完成／当前」；本设计零表
    （§9），`BackgroundDigest` 是调用方给的值。**缺的是「后台结果存哪里」这一步。收件人：协调者＋
    装配方（P8 执行器）。** 代价：P7D 不能独立兑现 §156 的跨重启展示；若由装配方存，P7D 只负责
    摘要形状，不保证有数据。

11. **`TaskSource` → `WorkloadDeclaration` 的 `foreground` 映射**。P7C §7.1 `:1203-1218` 给建议，
   本设计把 `foreground` 恒 false（P7D 只治理后台任务）。这使 `ForegroundRuntime`／`UserWorkload`
    两档在 P7D 路径上不可达。**缺的是「后台任务是否可能是用户正在等待的前台任务」这一条判断。
    收件人：P7C＋规范维护者。** 代价：用户等待的后台任务会落 `BackgroundRuntime`，在 `High` 压力下
    先于前台任务让路；若需要前台档，要加显式事实输入。

12. **`PolicyGrantRef`／`MaintenanceOccurrenceRef` 的签发与验证**。本设计只校验非空；grant 是否由
    第 5 层签发、是否被撤销、是否与任务内容匹配，P7D 都不知道。**缺的是「policy／maintenance 的
    授权引用形状与验证面」。收件人：第 5 层＋协调者＋规范维护者。** 代价：调用方可以伪造 grant；
    P7D 的「来源匹配」只是类型匹配，不是授权验证。

13. **Manual 的「继续 Core／继续 LCM」与 `existing_sessions` 的映射**。§11 的例子是 UI 文案；
   本设计用 `SessionCandidate::ContinueSession { intent }`，由调用方把「Core／LCM」映射成 `IntentId`。
    **缺的是「项目名→Intent」的映射。收件人：前端＋P8。** 代价：P7D 不解释项目名；若调用方给不出
    `IntentId`，Manual 候选只能显示空白。

14. **`SessionPlan` 的选择不保证资源准入**。Auto 可能选中一个随后被 P7C `Suspend` 的任务；
    P7D 不把资源预检放进 `decide`（P7B 的决策面不含资源）。**缺的是「选择时是否要看资源」这一步。
    收件人：P7C＋前端＋协调者。** 代价：用户可能看到「已选中」但任务被挂起；UI 需要展示 P7C 的状态。

15. **P7D 的 `next_tasks` 对 policy／maintenance 任务「无条件保留」**。它们不参与 P7B 的 Intent
    选择；P7D 不排序，但也不判断它们是否已完成、是否重复、是否与 Intent 任务冲突。**更关键的是**：
    Auto 的 `decide` 只从 Intent 候选里选，policy／maintenance 任务**不会被 Auto 选中**；若 §11 的
    「最值得继续的任务」可能是非 Intent 任务，就需要 P7B 的候选面扩展或在 P7D 另立决策面——后者会
    变成第二套排名，本设计不做。**缺的是「非 Intent 任务如何进入 Auto 选择」这一步。收件人：
    P7B＋规范维护者＋协调者。** 代价：调用方要自己维护非 Intent 任务集合；P7D 只是过滤器，
    Manual 可以展示它们，Auto 不能选它们。

16. **模型不可用时的 Auto 行为**。P7B 的 `IntentSelectionPolicy` 可能返回 `PolicyFailed`；P7D
    fail-closed，保留既有会话并报告错误，但不提供降级选择。**缺的是「模型不可用时 Auto 是否退回
    Manual／基线」这一步。收件人：P7B＋P8＋协调者。** 代价：Auto 在模型失败时没有计划；用户可以
    切 Manual，但 P7D 不自动切。

17. **§171 的触发策略／阈值归 P10，P7D 只交触发面**。§171 只给「session 结束后／系统空闲时／
    大量项目状态改变以后／用户主动要求时」四个时机（`:931-938`），没有给空闲时长、数量阈值、检查
    频率、重试与恢复。本设计只提供 `SynthesisTriggerPoint` 的三个转发（§4.8），**不决定何时调用**。
    **缺的是「四个时机的判定策略、频率、重试」这一条；收件人：P10＋协调者。** 代价：P7D 交付后，
    Dreaming 仍没有自动触发器；若 P10 不接，只有调用面没有调度。**这是切分 §6.2 第 10 条
    `:409` 的边界，不是本块的漏项。**

---

# 11. 实测与口径订正（含 2026-10-19 的切分同步）

以下行号／口径在本工作树实测；与切分文档或别家设计的旧行号不一致处，按本文件为准。

| # | 文档／它说 | 实测 | 处置 |
|---|---|---|---|
| 1 | 切分 §一 1.3 引 P6 fixture 与 `P6_EXPECTED_ROWS` 在 `:551` | `P6_EXPECTED_ROWS` 在 `dependency_direction.rs:551`；fixture 在 `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`（30 行＝3 注释＋27 数据） | 本文件按实测引 |
| 2 | 切分 §四第 1 条引解析器 `:615` 与断言 1 `:790` | 实测 `rows.len() == 27` 在 `:615`；`baseline.len() == 27` 在 `:790`；断言 2 `:799` | 本文件按实测引；rebaseline 同批改两个 `27` |
| 3 | 切分 §七第 11 条说 P7 先红的是断言 5 与断言 8 | 实测断言 5 在 `:837-854`、断言 8 在 `:882-895`；断言 1 只解析 fixture，初始树不会红 | 本文件按实测引；rebaseline 后仍改断言 1 |
| 4 | 切分 §一 1.3 说 P7D 顺取 201 | 本设计零表，**不占 201**；若将来落库再取 | 本文件 §9.1 |
| 5 | P7B 设计 §5.1 说 P7D 可经 `apply_intent_transition` 执行迁移 | 本设计不调用它：P7D 不写 `intent` 表；迁移由装配方／执行器按既有运行时路径做 | §4.4；若协调者要求 P7D 迁移，另立端口 |
| 6 | P7C 设计 §4.8 说「P7D 持有 `SuspensionState`」 | **已按 C1 选 (A) 落实**：P7D 持 `HashMap<TaskKey, SuspensionState>` 内存表；零数据库表，重启丢失是已知缺口 | §4.4、§5.5、§10 第 9 条 |
| 7 | 切分 §2.3.1 第 4 条原说 P7D「直接调」`begin/apply/fail` | `begin_synthesis` 需要 `&Tx<'_>`；**评审裁定端口**，切分第 4 条已同步为「P7D 定义调用面，装配方持 `Tx`；何时调用归 P10／调用方」 | §4.8、§10 第 4 条 |
| 8 | 任务材料把 §11 记在 `docs/spec/05-normative.md` | §11 在 `docs/spec/01-concepts.md:576-620`；`05-normative.md` 的 §299 在 `:1892-1903` | 本文件按实际文件引 |
| 9 | P6D 设计 §7.3.5 说 P7 新增 crate 会让断言 1 与断言 8 二选一必红 | 断言 1 只解析 fixture，不随新增 crate 红；先红的是断言 5／8；但 rebaseline 后两个 `27` 仍要改 | §3.9、§5.7 |
| 10 | 切分 §五 P7D 行说「不持产品状态」 | 本设计不登记 `continuum-product`、不读 Product 记录；`product drive` 只按值收 Intent | §2.1、§6 |

---

**本文的实测口径**：除 §11 的订正行外，正文引用的规范节号以 `docs/spec/05-normative.md`、
`docs/spec/01-concepts.md`、`docs/spec/03-product-drive.md` 为准；《工程》与《总纲》分别指
`docs/02-工程.md` 与 `docs/01-总纲.md`。零命中类断言已限定语料（`crates/continuum-supervisor/src`、
`Cargo.toml`）并逐处读过命中行；P6D 合并门的行号以本工作树的
`crates/continuum-runtime/tests/dependency_direction.rs` 为准。
