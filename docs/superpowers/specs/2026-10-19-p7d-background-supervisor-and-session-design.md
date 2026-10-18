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
| `product drive` 的授权证据：`kind == Product`＋`contract_id`＋`status == READY` | 切分 §三第 6 条 `:213-237`；§五 P7D 完成判据 1 | §4.3、§5.2 | 十三态表；`PLANNING→SUSPENDED→RUNNING` 负例 |
| Background Supervisor：`next_tasks` 与 `decide` | §299；《工程》§7.3 `:453` | §4.4 | `next_tasks` 不排序；`decide` 只转调 P7B |
| Session Orchestrator：Manual／Auto，共用 `SessionPlan` | §11 `:576-620`；《工程》§7.1 `:430`、§7.3 `:454` | §4.5、§4.6 | 同一 `SessionPlan` 类型；Manual 不选、Auto 选 |
| 后台新结果的摘要面 | 切分 §2.3.1 第 3 条 `:142`；§11 `:606`；§156 `:395-404` | §4.5 | `BackgroundDigest`；不承载 `Evidence` |
| §171 的触发点（不持阈值） | §171 `:931-938`；切分 §6.2 第 10 条 | §4.8 | 三个转发函数；无频率／重试／阈值 |
| 消费 P7C 的资源结论与挂起状态 | P7C §7.1 `:1182-1201`；切分 §2.3.2 第 2 条 | §4.7 | `admit` 返回 `ResourceDecision`；`checkpoint` 两阶段 |
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
条件边「仅当会话／后台作业落库」一致。**代价与替代**：若协调者要求 P7D 字面直调
`begin_synthesis(tx, ...)`，则必须新增 `continuum-persist` 边（切分未预登记），见 §10 第 4 条。

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

**dev 边：一条**（`continuum-canonical`，仅 `tests/`）。测试用本 crate、四个普通依赖与这一条 dev 边；
假 `IntentSelectionPolicy`、假 `SynthesisIo`、假 `CheckpointPort` 由本 crate 的 `tests/` 自己写。
`trybuild` 是外部 dev 依赖，不进 `ALLOWED`（同 P6A §2.1 的口径）。

**共写文件**：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members`（`:3`） | 加一行 `crates/continuum-supervisor` | 本块只加自己这一行 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-supervisor", &["continuum-arbiter", "continuum-canonical", "continuum-intent-scheduler", "continuum-memory", "continuum-semantics"])`——**五项**（四普通＋一 dev），与上表逐行对应；数组按字母序 | 新 crate 会先让 `every_crate_depends_only_on_its_allowed_set`（`:494`）变红再被补齐，这是刻意的 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 P6D 合并门（`:551`、`:574`、`:582`、`:615`、`:784`、`:790`、`:799`） | **rebaseline**：`p6_pre_allowed.txt` → `p7_pre_allowed.txt`；`P6_EXPECTED_ROWS` → `P7_EXPECTED_ROWS`（四行）；两个 `27` → `31`；断言 2 的 `5` → `4`；测试改名 | 这是 P7D 独占的改动（切分 §一 1.3 `:72`）；不得在 P6 fixture 上追加行 |
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
SupervisorError           —— 决策、授权、资源、checkpoint、记忆的失败面（§4.9）
SessionMode               —— Manual／Auto（§4.5）
SessionCandidate          —— ContinueSession／Task／BackgroundFinding／NewConversation（§4.5）
SessionSelection          —— Task／ContinueSession（§4.5）
BackgroundNote            —— 一条后台结果摘要（§4.5）
BackgroundDigest          —— completed＋findings 的摘要面（§4.5）
SessionPlan               —— 两模式共用的会话计划（§4.5）
BackgroundSupervisor      —— next_tasks／decide／admit／checkpoint（§4.4、§4.7）
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
```

**控制面**（两个端口，不是决策面）：

```
BackgroundSupervisor::admit     -> Result<ResourceDecision, SupervisorError>（转调 P7C）
BackgroundSupervisor::checkpoint -> Result<SuspensionPhase, SupervisorError>（调 CheckpointPort＋P7C）
SynthesisTriggerPoint::{begin,apply,fail} -> Result<..., SupervisorError>（调 SynthesisIo）
```

**模块落点**（源码面判据的语料按这张表）：

| 文件 | 内容 |
|---|---|
| `src/lib.rs` | 只做 `pub mod` 与 `pub use`，不定义类型 |
| `src/task.rs` | `TaskSource`／`TaskError`／`AuthorizedTask`／`PolicyGrantRef`／`MaintenanceOccurrenceRef` |
| `src/supervisor.rs` | `BackgroundSupervisor`／`SupervisorDecision`／`next_tasks`／`decide`／`admit`／`checkpoint` |
| `src/session.rs` | `SessionMode`／`SessionCandidate`／`SessionSelection`／`BackgroundNote`／`BackgroundDigest`／`SessionPlan`／`SessionOrchestrator` |
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

**本块不持有的判定**：Intent 排序与候选状态过滤（P7B）；资源压力、负载类别、四档优先级、挂起相位
（P7C）；Product 的 readiness／shippable／progress（P7A）；Intent 状态迁移（P4）；记忆合成的语义
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

**这一节是本块的「发明判据」总账（共 17 条）。** 规范没给、而本设计必须落成实质行为的判定，
都在下表里有一条；**没进这张表的判定，本块一处也没有定**。代码块只是示意，正文措辞才是约束。

| # | 本设计自定的判定 | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | `TaskSource` 四值的**判定规则**：`AuthorizedIntent`＝非 Product 的 `READY` Intent＋有效 contract；`ProductDrive`＝`kind == Product` 且 `product_id` 非空的 `READY` Intent＋有效 contract；`Policy`＝非空 `PolicyGrantRef`；`MaintenanceSchedule`＝非空 `MaintenanceOccurrenceRef` | §299 `:1894-1901` 只给四个名字；**没有说四类如何区分、证据是什么** | 两个 Intent 来源靠 `kind` 区分；Product 不能走 `AuthorizedIntent`；policy／维护来源不携带 Intent | 不覆盖「policy 与 maintenance 的授权真实性」；不覆盖「同一任务同时属于两类」；不覆盖「模型建议」——没有第五类 | §4.2、§4.3、§5.1 | 规范维护者（§299）＋协调者＋第 5 层 |
| 2 | `AuthorizedTask` 的**私有字段与四个构造器**；裸 `TaskSource` 不能构造 | 切分 §2.3.1 第 2 条 `:138` 要求「构造器私有」；**形状未给** | 调用方必须按来源选构造器；没有通用 `new`；新增来源要加构造器 | 不覆盖「调用方伪造 `Intent` 值」——`Intent` 字段是 `pub`；构造器只校验字段 | §4.3 | 规范维护者＋P7A |
| 3 | **`product drive` 证据**：`kind == Product`、`product_id` 非空、`contract.id` 非空、`contract.version >= 1`、`status == READY`；其余十二态拒绝 | 切分 §三第 6 条 `:226-237` 已给规则（选项 a）；**代码形状未给** | 不读合同内容、不验证合同存在、不防生产方自报状态 | 不覆盖「合同已撤销」；不覆盖「policy 判定」；不覆盖「合同版本过期」 | §4.3、§5.2 | 协调者＋第 5 层＋P7A |
| 4 | **`AuthorizedIntent` 也要求 `READY`**，且 `kind != Product` | §299 只说「authorized Intent」；§223 `:172-205` 只给状态机 | 非 Product 的 `RUNNING` Intent 不能作为新后台任务；已运行会话走 `ContinueSession` | 不覆盖「已有运行会话的继续」——那不是新任务；不覆盖「Intent 已授权但未 READY」 | §4.3、§5.2 | 规范维护者＋P7B |
| 5 | **`PolicyGrantRef` / `MaintenanceOccurrenceRef` 的形状**：分别是 `issuer＋grant_id` 与 `schedule_id＋occurrence_id` 的不透明非空串 | 切分 §2.3.1 第 2 条 `:137-139` 说「其余来源的 policy／capability 引用由 P7D 设计与第 5 层写」，**未给形状** | P7D 不能验证 grant 的真实性、有效期、是否被撤销；只能校验非空 | 不覆盖「伪造 grant」；不覆盖「grant 与任务内容不匹配」；不覆盖「policy 语义」 | §4.3、§5.1 | 第 5 层＋协调者＋规范维护者 |
| 6 | **`SessionPlan` 的形状**：`mode`／`candidates`／`selection`／`background`／`basis` 五个字段，两模式共用 | §11 `:576-620` 只给流程与例子；**没有字段、没有类型** | 消费方按固定五字段读；新增字段是破坏性改动；`candidates` 的固定段序是本设计定的 | 不覆盖「UI 的实际呈现」；不覆盖「用户选择后的状态」 | §4.5 | 规范维护者＋前端＋P8 |
| 7 | **Manual 的候选段序**：existing sessions（输入序）→ tasks（输入序）→ findings（输入序）→ `NewConversation`；段内不排序 | §11 `:582-593` 只给四个示例；**没有顺序规则** | 固定段序可能不是 UI 想要的；用户可能期望「新对话」在最前 | 不覆盖「按用户偏好重排」；不覆盖「跨段优先级」 | §4.6、§5.4 | 前端＋规范维护者 |
| 8 | **Auto 的选择只来自 P7B**：`Supervisor::decide` 调 `IntentScheduler::decide`；`Advance` 映射到 `Task` 或 `ContinueSession`；`NoIntent` 映射到无选择；`Err` fail-closed | §11 `:595-620` 只说「选择当前最值得继续的任务」；切分 §五 P7D 完成判据 2 要求复用 P7B | P7D 不能对 P7B 的结果再加权；P7B 策略的失败直接变成 `SupervisorError` | 不覆盖「P7B 策略本身错误」；不覆盖「后台结果不是 Intent」——见第 9 条 | §4.4、§5.3 | P7B＋协调者 |
| 9 | **后台新结果不参与排名**：`BackgroundDigest` 只展示；P7D 不把 finding 转成 Intent、不把它塞进 `SchedulerInputs` | §11 `:604-608` 要求 Auto「分析后台新结果」；**P7B 的 §302 五输入没有「后台结果」这一项** | Auto 只能显示 findings，不能因 finding 改变选择；若要让 finding 影响选择，装配方须把它投影成 `user_focus` 或生成 Intent | 不覆盖「finding 应优先」；不覆盖「finding 与 Intent 的对应」 | §4.5、§5.4、§10 第 7 条 | 规范维护者＋P7B＋P7D |
| 10 | **`TaskSource` → `WorkloadDeclaration` 的映射**：`MaintenanceSchedule` ⇒ `maintenance=true`；其余 ⇒ 三个布尔全 false | P7C §7.1 `:1203-1218` 只给建议；**P7D 的映射未冻** | 所有 P7D 任务落 `BackgroundRuntime`；`ForegroundRuntime`／`UserWorkload` 两档在 P7D 路径上不可达 | 不覆盖「用户正在等待的后台任务」；不覆盖「前台会话的任务」——那不在 P7D | §4.7、§5.5 | P7C＋规范维护者 |
| 11 | **checkpoint 的端口**：P7D 调 `CheckpointPort`，成功→`checkpoint_succeeded`，失败→`checkpoint_failed`，超时→`checkpoint_timed_out` | §14 `:730-744` 与切分 §2.3.2 第 2 条 `:159-165` 只给相位与失败回退；**checkpoint 的内容／执行者未给** | 端口实现必须知道任务的可恢复状态；P7D 不保存 checkpoint 内容 | 不覆盖「checkpoint 数据是否完整」；不覆盖「重启恢复」 | §4.7、§5.5 | P7C＋装配方＋P1 |
| 12 | **§171 触发点做成 `SynthesisIo` 端口**：P7D 只拥有调用序列，事务句柄在装配方 | 切分 §2.3.1 第 4 条 `:144-146` 说「直接依赖 `continuum-memory`，调 `begin/apply/fail`」；**`begin_synthesis` 需要 `&Tx<'_>`，P7D 不依赖 `continuum-persist`** | 字面上不是 P7D 直接调 P6C 的自由函数；多一个端口实现；P6C 的错误经端口返回 | 不覆盖「端口实现是否真的调了 P6C」——那是装配方的契约 | §4.8、§10 第 4 条 | 协调者＋P6C＋装配方 |
| 13 | **零持久化**：`SessionPlan`／`AuthorizedTask`／`BackgroundDigest` 都是值；不建表、不占 201 | §11／§156 没有持久化要求；切分 §四第 2 条 `:273` 说「P7D 若落库才占 201」 | 重启后会话计划与后台摘要不恢复；装配方要自己保存后台结果 | 不覆盖「昨晚完成」的跨重启展示（§156 `:395-404`） | §9、§10 第 10 条 | 协调者＋装配方 |
| 14 | **「低风险 Milestone」不由 P7D 判**：P7D 收所有 `READY` Product Intent | §156 `:392` 只说「继续完成低风险 Milestone」；**没有风险字段／判据** | P7D 可能把非低风险 Milestone 纳入后台；风险判断在 P7A／Product Drive | 不覆盖「风险模型」；不覆盖「Milestone 的副作用」 | §5.1、§10 第 8 条 | P7A＋规范维护者 |
| 15 | **P6D 合并门选 rebaseline**：`p7_pre_allowed.txt` 31 行、`P7_EXPECTED_ROWS` 四行、两个 `27` → `31`、断言 2 的 `5` → `4` | 切分 §四第 1 条 `:259-264` 给两条路；P6D §7.3.5 给生命周期 | 多一个 fixture 与一次五处同改；下一个子项目还会到期 | 不覆盖「故意同时改基线」——同 P6D 的自保护边界 | §5.7、§9.2 | 协调者＋P6D |
| 16 | **`SessionPlan` 的 `selection` 不保证资源准入**：Auto 可以先选中一个随后被 `Suspend` 的任务 | §11 只说「选择」；P7C 的动作在 `direct` 时才知道 | 用户可能看到「已选中」但任务被挂起；UI 要展示资源状态 | 不覆盖「选择时的资源预检」；不覆盖「挂起后的重选」 | §5.5、§10 第 9 条 | P7C＋前端 |
| 17 | **§171 的触发策略不由 P7D 定义**：P7D 只给触发点；四个时机的判定、空闲阈值、频率、重试归 P10 | §171 `:931-938` 只给四个时机的名字；切分 §6.2 第 10 条把阈值／频率／重试判给 P10 | Dreaming 交付后仍没有自动触发器；调用方必须自己决定何时调 `begin` | 不覆盖「何时触发」；不覆盖「重试与恢复」 | §5.6、§10 第 17 条 | P10＋协调者 |

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
`SUSPENDED`／`VERIFYING`／`EVOLVING` 任一当证据都会接受一条从未过审的 Intent。J3 的负例
`PLANNING → SUSPENDED → RUNNING` 就是这条的照片。

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

---

# 4. 类型与接口

**这一节是 P7D 对装配方／UI 的冻结面。** 代码块是示意，正文措辞才是约束；字段名、变体名、访问器名
以本节为准。所有类型都在 `continuum-supervisor` 内定义；跨块类型只引用，不重定义。

## 4.1 模块落点

见 §2.2 的表。**测试位置**：全部放 `crates/continuum-supervisor/tests/`，不放 `src/` 的
`#[cfg(test)]`；这样「源码面判据」的语料（`src/`）里没有测试构造（同 P6A §8.2 的纪律）。测试只用
`std`＋本 crate＋四个普通依赖＋`trybuild`（外部 dev）；假 `IntentSelectionPolicy`／`SynthesisIo`／
`CheckpointPort` 由 `tests/` 自己写。

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
```

**四个构造器的校验规则**（逐条；J3／J4／J5）：

| 构造器 | `kind` | `status` | `contract` | 其他 |
|---|---|---|---|---|
| `authorized_intent` | `!= Product` | `Ready` | `id` 非空、`version >= 1` | 无 |
| `product_drive` | `== Product` | `Ready` | `id` 非空、`version >= 1` | `product_id: Some` 且非空 |
| `policy` | — | — | — | `issuer`／`grant_id` 非空 |
| `maintenance` | — | — | — | `schedule_id`／`occurrence_id` 非空 |

**对切分措辞的读法**：切分／任务材料里的「`AuthorizedTask` 只收 `READY` Product Intent」本设计读作
**`product drive` 这一路**的规则（切分 §2.3.1 第 2 条明写「本文只冻结 `product drive` 这一路」，
`:134-139`）；`authorized Intent` 是 §299 的另一类，同样是 `READY`＋有效 contract，但不是 Product。
若协调者要求 `AuthorizedTask` **全局**只允许 Product Intent，则 `authorized_intent` 构造器删除，
`TaskSource::AuthorizedIntent` 只能由第 5 层的 grant 表达；这是 §10 第 1 条的裁定点，不是本设计
在正文里能自行收窄的。

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
/// 后台治理入口。**无状态**（不持任务表、不持资源状态）；所有输入按值／按引用传入。
#[derive(Debug, Default, Clone, Copy)]
pub struct BackgroundSupervisor;

/// `decide` 的结果：只三臂。
#[derive(Debug, Clone)]
pub enum SupervisorDecision {
    /// P7B 选中了一枚 READY Intent，且它有对应的 `AuthorizedTask`。
    Task { task: AuthorizedTask, decision: IntentDecision },
    /// P7B 选中了一枚活跃 Intent，且它有既有会话；**不迁移状态**。
    ContinueSession { intent: IntentId, decision: IntentDecision },
    /// P7B 明确 `NoIntent`；**既有会话与后台任务必须保留**。
    NoIntent { decision: IntentDecision },
}

impl SupervisorDecision {
    pub fn intent_decision(&self) -> &IntentDecision;
    pub fn selected_intent(&self) -> Option<&IntentId>;
    pub fn basis(&self) -> &DecisionBasis;
}

impl BackgroundSupervisor {
    /// 本轮可运行的任务集合。**只过滤，不排序**：保持 `tasks` 的输入序。
    /// - 有 Intent 的任务：只保留其 Intent 在 `candidates` 中且 `status == READY` 的；
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
    /// - `Advance` 到 `RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`EVOLVING`：
    ///   在 `existing_sessions` 中找 id；找不到 ⇒ `Err(NoActiveSession)`；
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

**`next_tasks` 不是排名**：它按 `tasks` 的输入序返回引用；`Vec` 的顺序就是调用方给的顺序。它不读
`SchedulerInputs`、不调 `decide`、不比较 `priority`／`deadline`／`blocking`。Manual 模式用它做候选
展示；Auto 模式用 `decide` 做选择。**两处都不产生第二套顺序**（J7／J8）。

**`decide` 的状态分派不是排名**：它只按候选的 `IntentStatus` 决定「这是新任务还是既有会话」；
不比较两枚候选。P7B 的 `Advance` 已经给出唯一 id；P7D 只做身份查找与错误处理。**P7D 不调用
`apply_intent_transition`**：迁移由装配方／执行器按既有运行时路径做（P7B §5.1 的「或既有运行时
路径」；§11 第 5 条）。J7 的源码面会钉住本 crate 没有 `apply_intent_transition`。

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

#[derive(Debug, Clone)]
pub enum SessionSelection {
    Task { intent: IntentId, source: TaskSource },
    ContinueSession { intent: IntentId },
}

/// 一条后台结果摘要。**不透明文本**；不是 `Evidence`、不是 `Claim`。
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
#[derive(Debug, Clone)]
pub struct SessionPlan {
    mode: SessionMode,
    candidates: Vec<SessionCandidate>,
    selection: Option<SessionSelection>,
    background: BackgroundDigest,
    basis: Option<DecisionBasis>,
}

impl SessionPlan {
    pub fn mode(&self) -> SessionMode;
    pub fn candidates(&self) -> &[SessionCandidate];
    pub fn selection(&self) -> Option<&SessionSelection>;
    pub fn background(&self) -> &BackgroundDigest;
    pub fn basis(&self) -> Option<&DecisionBasis>;
}
```

**五个字段的理由**：

| 字段 | 作用 | 规范状态 |
|---|---|---|
| `mode` | 区分 Manual／Auto；两模式共用同一形状 | §11 两模式；字段名本设计自定 |
| `candidates` | Manual 的候选展示；Auto 的分析面 | §11 `:584-591`；形状本设计自定 |
| `selection` | Auto 的选择；Manual 恒 `None` | §11 `:608-620`；形状本设计自定 |
| `background` | 「昨晚完成／发现」的摘要 | §11 `:606`；§156 `:395-404`；形状本设计自定 |
| `basis` | P7B 的 `DecisionBasis`；Manual 恒 `None` | P7B §4.1 `:507`；本设计只承载 |

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
/// §11 的 Session Orchestrator。**无状态**；每次 Session Activation 调一次 `plan_*`。
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionOrchestrator {
    supervisor: BackgroundSupervisor,
}

impl SessionOrchestrator {
    pub fn new(supervisor: BackgroundSupervisor) -> Self;

    /// Manual：只生成候选；`selection` **恒 `None`**；不调 P7B。
    /// 候选段序：existing_sessions（输入序）→ tasks（输入序）→ findings（输入序）→ NewConversation。
    pub fn plan_manual(
        &self,
        tasks: &[AuthorizedTask],
        existing_sessions: &[IntentId],
        background: BackgroundDigest,
    ) -> SessionPlan;

    /// Auto：调 `BackgroundSupervisor::decide`；`selection` 来自 P7B 的 `Advance`，
    /// `NoIntent` ⇒ `selection = None` 且保留既有会话；`Err` ⇒ fail-closed。
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

**Manual 不选**：`plan_manual` 不调 `decide`、不读 `SchedulerInputs`、不读 `ResourceArbiter`；它只把
调用方给的候选按固定段序装进 `SessionPlan`，`selection = None`。用户的选择发生在 P7D 之外（UI／
调用方），P7D 不把「用户选了哪个」编码成第二个决策面。J10 钉住 `plan_manual` 的源码面没有
`IntentScheduler::decide`。

**Auto 的选择**：`plan_auto` 先调 `supervisor.decide(...)`，再构造与 Manual 相同的候选列表；若
`SupervisorDecision::Task` ⇒ `selection = Some(SessionSelection::Task { .. })`；若
`SupervisorDecision::ContinueSession` ⇒ `Some(SessionSelection::ContinueSession { .. })`；若
`NoIntent` ⇒ `None`，`basis = Some(NoIntent.basis)`。**`SessionPlan` 的两模式形状完全相同**，只有
`mode`／`selection`／`basis` 不同（J10）。**`plan_auto` 不恢复会话**：§11 `:620` 的「然后直接恢复」
由装配方／执行器按 `SessionSelection` 执行；P7D 只产出计划，不执行、不迁移状态。

**Auto 不读后台结果做排名**：`plan_auto` 把 `background` 原样放进 `SessionPlan`，不把
`BackgroundNote` 塞进 `SchedulerInputs`，也不因 finding 改变 `selection`。若要让 finding 影响选择，
调用方可以在构造 `SchedulerInputs.user_focus` 时表达（`UserFocus::Intent`／`Area`，P7B §4.2 `:617`），
但那是调用方的输入，不是 P7D 的排名（§10 第 7 条）。

## 4.7 资源消费：`admit` 与 `checkpoint`

```rust
impl BackgroundSupervisor {
    /// 把一枚任务交给 P7C 的 `ResourceArbiter::direct`（P7C §4.11 `:827`）。
    /// **返回 P7C 的结论原样**；P7D 不解释动作、不把 `None` 当 `Run`。
    /// 调用方负责周期调用 `ResourceArbiter::observe`；P7D 不读 OS、不读 `ResourceObservation`。
    pub fn admit(
        &self,
        task: &AuthorizedTask,
        state: &mut SuspensionState,
        arbiter: &ResourceArbiter,
        now_millis: u64,
    ) -> Result<ResourceDecision, SupervisorError>;

    /// 执行一次 checkpoint 并推进 P7C 的相位：
    /// - 端口成功 ⇒ `checkpoint_succeeded`；成功 ⇒ `Suspended`；
    /// - 端口失败 ⇒ `checkpoint_failed`；相位回 `Throttled`，返回 `Checkpoint { .. }`；
    /// - 超时／错相位 ⇒ `SupervisorError::Resource(SuspensionError)`。
    /// **不得停在 `Checkpointing`，不得在失败时报 `Suspended`**（P7C §5.6）。
    pub fn checkpoint(
        &self,
        task: &AuthorizedTask,
        state: &mut SuspensionState,
        port: &dyn CheckpointPort,
        now_millis: u64,
    ) -> Result<SuspensionPhase, SupervisorError>;
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

**`checkpoint` 的两阶段**：P7C 的 `direct` 只把相位推进到 `Checkpointing`；P7D 的 `checkpoint` 才调
端口、回报成功／失败。成功路径唯一；失败路径必须回到 `Throttled`。P7D 不保存 checkpoint 内容，
不实现重启恢复（§10 第 9 条）。

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

**为什么是端口而不是字面直调**：`begin_synthesis` 的签名是
`begin_synthesis(tx: &Tx<'_>, request: &SynthesisRequest)`（`crates/continuum-memory/src/synthesis.rs:108`），
要字面直调就必须命名 `continuum_persist::Tx<'_>`，从而新增 `continuum-persist` 边；切分 §三 P7D 行
只允许该边「仅当会话／后台作业落库」（`:193`）。本设计选端口，**让事务句柄留在装配方**；P7D 仍直接
依赖 `continuum-memory` 的类型（`SynthesisRequest`／`SynthesisJobId`／`SynthesisProposal`／
`SynthesisReport`／`MemoryError`），且三个方法的**调用序列**归 P7D。这是本设计对切分 §2.3.1 第 4 条
「直接调」的一处**具名偏差**，代价与收件人见 §10 第 4 条。若协调者要求字面直调，改法是：加
`continuum-persist` 边、把 `SynthesisIo` 换成接收 `&Tx<'_>` 的三个自由函数；不改
`SynthesisTriggerPoint` 的语义。

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
    Resource(SuspensionError),
    Checkpoint { error: CheckpointError, phase: SuspensionPhase },
    Memory(MemoryError),
    InvalidBackgroundNote,
}
```

- **`Task`**：`AuthorizedTask` 的拒绝（§4.3）。
- **`Decision`**：P7B 的 `PolicyFailed`／`InvalidAdvance`；P7D **不折成 NoIntent**。
- **`UnauthorizedAdvance`**：P7B 选中 READY Intent，但没有对应 `AuthorizedTask`。
- **`NoActiveSession`**：P7B 选中活跃 Intent，但没有既有会话。
- **`UnsupportedAdvance`**：P7B 返回了 P7D 不认识的候选状态（防御臂；P7B 已校验六态，理论上不可达）。
- **`Resource`**：P7C `direct`／`checkpoint_*` 的 `SuspensionError`。
- **`Checkpoint`**：端口失败；相位已经回 `Throttled`（P7C §5.6）。
- **`Memory`**：`SynthesisIo` 透传的 `MemoryError`。
- **`InvalidBackgroundNote`**：`BackgroundNote::new` 的空标签。

`Display`／`Error` 手写（`std` only）；`source()` 对 `Task`／`Decision`／`Resource`／`Memory`／
`Checkpoint` 返回内层错误。**不派生 `Clone`**：`MemoryError` 不是 `Clone`（`src/error.rs:8`）。

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
  3. `SessionPlan.mode == Manual` ⇒ `selection.is_none()`；`mode == Auto` 才可能有选择。
  4. `SessionPlan` 的 `candidates` 不排序；段序固定。
  5. `BackgroundDigest` 的每个 note 的 `label` 非空。
- **不实现**：`Serialize`／`Deserialize`；没有 `serde` 依赖；没有 `tokio`／执行器依赖。

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
`PLANNING→SUSPENDED→RUNNING` 的 Product Intent（`status == Running`）在 `product_drive` 上稳定落
`NotReady`，不会因为 contract 缺失而先落 `InvalidContract`——负例的可观察臂是固定的（J3）。**顺序是
本设计自定**（§2.4 第 3 条），规范只给三个条件的合取。

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
   - `status` 是 `Running`／`Waiting`／`Suspended`／`Verifying`／`Evolving`：在
     `existing_sessions` 中按 `as_str()` 找；无 ⇒ `Err(NoActiveSession { intent })`；
   - 其他状态 ⇒ `Err(UnsupportedAdvance { intent })`（防御臂）。
5. 返回 `Task`／`ContinueSession`。

**「第一个匹配」不是排名**：`tasks` 的顺序是调用方给的；重复任务的元数据不一致是调用方契约违反，
P7D 不定义选哪一枚（P7B §4.2 `:640-644` 对重复候选有同样的口径）。P7D 不比较 `priority`／
`deadline`／`blocking`；这些只在 P7B 的 `SchedulerInputs` 里。

**`AuthorizedTask` 是构造时的快照**：它的 `intent.status` 在构造时是 `READY`；之后 P4 可能把它推进到
别的状态。`decide` 的分派以 `SchedulerInputs.candidates` 的**当前**状态为准，不以
`AuthorizedTask.intent.status` 重新判候选；若当前状态不是 `READY`，`Advance` 走 `ContinueSession`
分支（并要求既有会话）。`AuthorizedTask` 不再被重新校验，也不因状态变化失效；它是「这枚 Intent
在构造时有过授权证据」的记录。

**`Advance` 到 `READY` 但无任务**：这是「策略选了没有授权证据的 Intent」。**不**把 `Advance` 当
`NoIntent`；**不**为它构造一个 `AuthorizedTask`；**不**放弃既有会话。返回 `Err`，调用方保留现状
（fail-closed）。

**`Advance` 到活跃状态但无会话**：P7B 的候选集应只包含 P7D 确有会话的 `Evolving`（P7B §2.4
`:224-227`）；`Running`／`Verifying` 也可能有既有会话。若无，P7D 不能凭空恢复，返回
`NoActiveSession`。这是本设计自定（§2.4 第 8 条）。

## 5.4 SessionPlan 的组成

**Manual**（`plan_manual`）：

1. `existing_sessions` 按输入序 ⇒ `SessionCandidate::ContinueSession { intent }`。
2. `tasks` 按输入序 ⇒ `SessionCandidate::Task(task.clone())`。
3. `background.findings()` 按输入序 ⇒ `SessionCandidate::BackgroundFinding(note.clone())`。
4. 末尾追加 `SessionCandidate::NewConversation`。
5. `selection = None`；`basis = None`；`background` 原样放入。

**Auto**（`plan_auto`）：

1. 调 `supervisor.decide(...)`；`Err` ⇒ 返回 `Err`（不生成计划）。
2. 用与 Manual 相同的四步构造 `candidates`（段序相同）。
3. `SupervisorDecision::Task { task, .. }` ⇒ `selection = Some(Task { intent: task.intent().id,
   source: task.source() })`；`basis = Some(decision.basis().clone())`。
4. `SupervisorDecision::ContinueSession { intent, .. }` ⇒ `selection = Some(ContinueSession { intent })`；
   `basis = Some(decision.basis().clone())`。
5. `SupervisorDecision::NoIntent { decision }` ⇒ `selection = None`；`basis = Some(decision.basis().clone())`。
6. `background` 原样放入；`basis` 只在 Auto 有值。

**固定段序不是排名**：段与段之间的顺序是 UI 的固定分区（继续／新任务／发现／新对话），不是
「哪个更值得」的比较；段内保持调用方输入序。P7D 不 sort、不 min／max、不按 `priority` 排。
**Manual 的 `selection` 恒 `None`**：用户选择在 P7D 之外；P7D 不把用户选择编码成第二个决策面。
**Auto 的 `selection` 只来自 P7B**：P7D 不读 `background` 来改选择。

**`SessionPlan` 的 `basis`**：Manual 为 `None`；Auto 为 P7B 的 `DecisionBasis` 的克隆。`DecisionBasis`
是不透明 `String`（P7B §4.1 `:507`）；P7D 可以展示，但**不得按内容分支**（P7B §5.1 `:1073-1074`）。

## 5.5 资源与 checkpoint

**`admit`**：

1. `declaration_for(task)` 按 §4.7 的唯一映射填三个布尔。
2. `arbiter.direct(declaration, state, now_millis)`。
3. `Ok(decision)` 原样返回；`Err(e)` ⇒ `SupervisorError::Resource(e)`。
4. P7D **不**把 `action == None` 解释为 `Run`；调用方必须按 `None` 不动作。
5. P7D **不**读 `decision.summary()` 做决策；summary 只是 P7C 的 AI／后台暴露面，P7D 可以把
   `resource_pressure`／`workload_class` 展示给用户，但不得据此重新排名。

**`checkpoint`**：

1. 调 `port.checkpoint(task)`。
2. `Ok(())` ⇒ `state.checkpoint_succeeded(now_millis)`；成功 ⇒ `Suspended`；`Err(CheckpointTimedOut)`
   ⇒ `SupervisorError::Resource`（相位已由 P7C 回 `Throttled`）。
3. `Err(port_error)` ⇒ `state.checkpoint_failed(now_millis)`；若返回 `CheckpointFailed` ⇒
   `SupervisorError::Checkpoint { error: port_error, phase: state.phase() }`；若返回
   `NotCheckpointing { phase }` ⇒ `SupervisorError::Resource(NotCheckpointing { phase })`。
4. **不得停在 `Checkpointing`**：端口失败与超时都必须回到 `Throttled`（P7C §5.6）。
5. P7D **不**在失败时重试；重试策略归 P7C 的 `attempts`／P10（§10 第 9 条）。P7D 只做一次
   checkpoint 调用；若调用方要重试，由它再次调 `admit` 得到新的 `Suspend`。

**P7D 不拥有资源状态**：`SuspensionState` 由调用方按任务持有并传入；P7D 不建任务表、不保存相位
（P7C §4.8 `:680-683` 明写「P7C 不集中管理任务表；P7D 持有」——本设计把「持有」落在调用方，
P7D 只做控制逻辑；若协调者要求 P7D 自己持表，见 §10 第 9 条）。

## 5.6 §171 触发的边界

**P7D 只提供**：

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

**选 rebaseline，不退役。** 理由：

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
| **P7C** | `admit` 调 `ResourceArbiter::direct`；`checkpoint` 调 `SuspensionState` 的成功／失败／超时；`TaskSource` → `WorkloadDeclaration` 三布尔 | §4.7；§5.5 | 不读 `PriorityTier`／`AssemblyView`／`ResourceObservation` | P7C §7.1 `:1182-1201` 已给义务；本设计对着写 |
| **P7A** | `product drive` 按值收 `READY` Product Intent；只交值或 id；P7D 不读 Product 记录 | `AuthorizedTask::product_drive(intent)`；§5.2 | 不登记 `continuum-product` 边 | P7A §4.8 `:1435-1454`；按值交接 |
| **P6C** | `SynthesisIo` 的实现转调 `begin_synthesis`／`apply_synthesis`／`fail_synthesis`；`SynthesisTrigger` 不重定义 | §4.8；§5.6 | 不依赖 `continuum-persist`（事务在实现方） | P6C §8.6 `:1101-1109`；本设计选端口（§10 第 4 条） |
| **P4** | `Intent`／`IntentStatus`／`ContractRef` 只读；候选集 `&[Intent]`；不直读 `intent` 表、不调 `apply_intent_transition` | §4.3、§4.4 | 不写 `intent` 表 | 切分 §6.2 第 5 条；J5／J19 |
| **P6D** | 合并门 rebaseline：31 行 fixture、四行 P7 期望、两个 `27` → `31`、断言 2 `5` → `4` | §5.7、§9.2 | 不在 P6 fixture 追加行 | P6D §7.3.5；切分 §四第 1 条 |
| **装配方 `continuum-runtime`** | 实现 `SynthesisIo`／`CheckpointPort`；周期调 `ResourceArbiter::observe`；执行 `ready → running` 的既有迁移；提供候选 `&[Intent]` | §4.7、§4.8；§5.3 | 不依赖 `continuum-runtime` | 实现不在 P7D；本轮不登记 runtime 边 |

**P7B 的接缝细节**：P7D 不实现 `IntentSelectionPolicy`；它接受调用方给的 `&dyn IntentSelectionPolicy`
（可以是 P7B 的 `BaselineIntentPolicy`，也可以是装配方注入的策略）。**P7D 自己实现一个新的
`IntentSelectionPolicy` 就是第二个排名**（P7B §5.1 `:1079-1081`）。`decide` 的 `Err` 必须 fail-closed，
不能当 `NoIntent`（P7B §5.1 `:1071-1072`）。

**P7C 的接缝细节**：P7D 不读 `PriorityTier`／`AssemblyView`；`WorkloadDeclaration` 只带三个布尔，
不带 §300 的档名。`admit` 返回 `ResourceDecision` 原样；P7D 不把 `action == None` 当 `Run`。
`checkpoint` 的两阶段与失败回退按 P7C §5.5／§5.6；P7D 不建第二套 `Suspended` 状态机。

**P7A 的接缝细节**：P7A 的 `product_intent_for_handoff` 只返回 `READY`；P7D 只收这个值。P7A 的
drive gate（`PostShippableImprovement`＋`ContinuousProductDrive` 关闭 ⇒ `None`）在 P7A 内；P7D 看不到
开关，也不读 Product 记录（P7A §4.8 `:1447-1450`）。若 P7D 需要读 Product 记录或合同内容才能判定
授权，那是**一条要报协调者的新边**（切分 §三第 6 条 `:382-383`），本设计不登记。

**P6C 的接缝细节**：`SynthesisIo` 是 P7D 定义的端口，不是 P6C 的第二个作业；实现方必须原样转调
三个函数，不复制准入判据（P6C 的 G0–G6 在 `begin_synthesis` 内）。P7D 不生成 proposal、不做
七步算法、不读 `memory_synthesis_job` 表。`SynthesisTrigger` 的四个时机由 P6C 冻结；P7D 不新增
第五个、不 `parse`、不把它与 `TaskSource` 合并。

**P6D 的接缝细节**：P7D 是合并门的唯一所有者（切分 §一 1.3 `:72`）；P7A／B／C 不在 P6 fixture
追加行。P7C 的 J1(ii) 依赖 P7D 先完成 rebaseline（P7C §8.4 第 10 条）；P7D 的 J15 是最终树的照片。

---

# 7. 判据与照片

## 7.1 判据表（每条挂规范节）

**切分 §五 P7D 的三条完成判据对应**：判据 1（任务来源闭集＋`AuthorizedTask` 只收 `READY`）由
**J1＋J2＋J3** 覆盖；判据 2（会话选择不另造排名）由 **J7＋J10** 覆盖；判据 3（§171 只提供触发点）由
**J14** 覆盖。其余 J4–J6、J8–J9、J11–J13、J15–J22 是本设计自报的补充判据；**§7.4 没有 P7D 的
判据**，本设计不修改 §7.4（J20）。

| # | 判据 | 规范节 | 观测形态 |
|---|---|---|---|
| **J1** | `TaskSource` 恰四值；无第五类、无 `ModelSuggested`、无 `parse`／`FromStr`；`as_str` 四字面量逐字 | §299 `:1894-1901`；切分 §五 P7D 完成判据 1 | 源码面（枚举定义＋`grep`）＋编译期（穷尽 `match`） |
| **J2** | **裸 `TaskSource` 不能构造 `AuthorizedTask`**：字段私有；没有 `new(source, evidence)`；四个构造器是唯一入口 | 切分 §2.3.1 第 2 条 `:138` | 编译期（`trybuild`：结构体字面量／`AuthorizedTask::new` 编不过）＋源码面 |
| **J3** | **`product drive` 只收 `READY`**：十三态表逐状态；`PLANNING→SUSPENDED→RUNNING`（`status == RUNNING`，从未过 `REVIEW`）必须 `Err(NotReady)`；`COMPLETED`／`EVOLVING` 也拒绝 | §223 `:172-205`；切分 §三第 6 条 `:226-237`；§八第 5 条 `:483-485` | 运行期（十三组）＋源码面（构造器的 `status == Ready`） |
| **J4** | `authorized Intent` 只收非 Product 的 `READY` Intent；Product 走 `product_drive` | §299；§223；切分 §2.3.1 第 2 条 | 运行期（`WrongKind`）＋源码面 |
| **J5** | `contract.id` 非空且 `version >= 1`；`product_id` 非空；错误臂分别可观察 | §222 `:148`；§224；切分 §三第 6 条 `:229-230` | 运行期（四类负例） |
| **J6** | `policy`／`maintenance` 只收非空引用；没有 Intent；来源匹配由构造器类型保证 | §299；切分 §2.3.1 第 2 条 | 运行期（空串负例）＋编译期（不能把 `PolicyGrantRef` 传给 `maintenance`） |
| **J7** | **不另写排名**：`src/` 无 `impl IntentSelectionPolicy`；无 `sort`／`sort_by`／`min`／`max`／`cmp`／`partial_cmp`／`BinaryHeap`；`decide` 只调 `IntentScheduler::decide` 一次 | 切分 §五 P7D 完成判据 2；P7B §5.1 `:1079-1081` | 源码面（`grep` 非注释行）＋运行期（假策略选第二枚，P7D 返回第二枚） |
| **J8** | `next_tasks` 保持 `tasks` 输入序；只过滤 READY；policy／maintenance 无条件保留；不读 `SchedulerInputs` | §299；§11 Manual 候选 | 运行期（顺序断言）＋源码面（无 sort） |
| **J9** | `decide` 映射：`Advance`＋READY＋有任务 ⇒ `Task`；`Advance`＋活跃＋有会话 ⇒ `ContinueSession`；`NoIntent` ⇒ `NoIntent`；`Err` ⇒ `Err`；`Advance`＋READY＋无任务 ⇒ `UnauthorizedAdvance`；`NoIntent` 保留既有会话 | P7B §4.1 `:500`、§5.1 `:1052-1081`；§302 | 运行期（六组假策略） |
| **J10** | **两模式共用 `SessionPlan`**：同一类型、五个字段／访问器；Manual 的 `selection` 恒 `None` 且不调 `decide`；Auto 的 `selection` 来自 P7B；`plan_manual` 源码面不出现 `IntentScheduler::decide` | §11 `:576-620`；切分 §五 P7D 完成判据 2 | 运行期（两模式同一类型）＋源码面（`plan_manual` 函数体） |
| **J11** | `SessionPlan` 字段私有；无 `Default`／`Deserialize`；唯一构造点是 `plan_manual`／`plan_auto`；`candidates` 段序固定且段内不排序 | §11；§2.4 第 6、7 条 | 编译期（`trybuild` 结构体字面量）＋源码面 |
| **J12** | **后台摘要不是 `Evidence`**：`BackgroundNote` 只有 `label`／`at_millis`；`src/` 无 `Evidence`／`CompletionVerdict`／`continuum_verify`；`BackgroundDigest` 不参与排名 | §89 `:939-945`；切分 §四第 4 条 | 源码面＋运行期（digest 原样进 plan） |
| **J13** | **资源消费**：`admit` 调 `ResourceArbiter::direct`；`declaration_for` 的映射逐项；`action == None` 原样返回；`src/` 无 `impl CheckpointPort`（端口实现不在本块）；`src/` 无 `PriorityTier`／`AssemblyView`／`ResourceObservation`；不启用 `assembly` | P7C §4.10 `:759`、§4.11 `:827`、§7.1 `:1182-1201`；§300 | 源码面＋运行期（四档动作对照） |
| **J14** | **§171 只提供触发点**：`SynthesisTriggerPoint` 三个方法各转发一次；`src/` 无 `impl SynthesisIo`（实现不在本块）；`src/` 无 `Duration`／`Instant`／`SystemTime::now`／`retry`／`interval`／`sleep`／`idle` 判定；`SynthesisTrigger` 不重定义；无阈值 | §171 `:931-938`；切分 §五 P7D 完成判据 3、§6.2 第 10 条 | 源码面＋运行期（假 `SynthesisIo` 计数） |
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
| **J1-a** | `src/task.rs` 的 `TaskSource` 定义恰四变体；`match` 穷尽（无 `_`）编译通过；`grep -rn "ModelSuggested\|TaskSource::Model\|UserRequested\|SystemIdle" crates/continuum-supervisor/src` 零命中（非注释） | 源码面＋编译期 | 外部代码仍可构造 `TaskSource` 标签；由 J2 兜住 |
| **J1-b** | `grep -rn "impl FromStr for TaskSource\|fn parse.*TaskSource" crates/continuum-supervisor/src` 零命中 | 源码面 | 字符串仍可出现在 `PolicyGrantRef` 里；那不是来源 |
| **J2** | `trybuild` 两个 `compile_fail`：①`AuthorizedTask { source: TaskSource::Policy, evidence: ... }` ⇒ E0451（字段私有）；②`AuthorizedTask::new(TaskSource::Policy, ...)` ⇒ E0599（无此函数） | 编译期 | 内部模块仍可构造；本 crate 只有四个构造器 |
| **J3** | 十三态表：`product_drive` 对 `PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW`／`READY`／`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`COMPLETED`／`EVOLVING`／`FAILED`／`CANCELLED` 各一枚 Intent；只有 `READY` ⇒ `Ok`，其余 ⇒ `Err(NotReady)`。**另加**：构造 `PLANNING`，按 §223 走 `PLANNING→SUSPENDED→RUNNING`，得到 `status == RUNNING` 的 Intent；`product_drive` ⇒ `Err(NotReady { status: Running })` | 运行期 | 谁把 `PROPOSED` 推进到 `READY`（P4 审查流程，不在本块；§3.7） |
| **J4** | `product_drive` 对 `IntentKind::Task` ⇒ `Err(WrongKind)`；`authorized_intent` 对 `IntentKind::Product` ⇒ `Err(WrongKind)`；`authorized_intent` 对 `Task`＋`READY`＋有效 contract ⇒ `Ok` | 运行期 | — |
| **J5** | `product_drive` 对 `contract.id == ""` ⇒ `Err(InvalidContract)`；`version == 0` ⇒ `Err(InvalidContract)`；`product_id == None`／`Some("")` ⇒ `Err(MissingProductId)`；`authorized_intent` 的同两类 contract 负例 | 运行期 | 合同是否真实存在（P7D 不读合同） |
| **J6** | `PolicyGrantRef::new("", "g")`／`new("i", "")` ⇒ `Err(EmptyPolicyGrant)`；`MaintenanceOccurrenceRef::new("", "o")`／`new("s", "")` ⇒ `Err(EmptyMaintenanceRef)`；`policy(grant).source() == Policy`；`maintenance(schedule).source() == MaintenanceSchedule`；`policy(...).intent() == None` | 运行期 | grant 的真实性（第 5 层） |
| **J7-a** | `grep -rn "impl IntentSelectionPolicy\|fn choose\|enum IntentDecision\|struct IntentScheduler\|struct SchedulerInputs" crates/continuum-supervisor/src` 零命中（非注释） | 源码面 | 测试里的假策略不算生产实现 |
| **J7-b** | `grep -rn "\.sort\|\.sort_by\|\.min\|\.max\|cmp(\|partial_cmp\|BinaryHeap\|BTreeSet" crates/continuum-supervisor/src` 零命中（非注释） | 源码面 | 注释里可能提到；限定非注释行 |
| **J7-c** | 假策略 `choose` 返回候选集**第二枚**的 `Advance`；`decide` 返回的 `selected_intent()` 是第二枚（按 `as_str()`），不是第一枚 | 运行期 | 只证明 P7D 不重排；P7B 策略本身仍可能错 |
| **J8** | `tasks = [t1, t2, t3]`（t1／t2／t3 的 Intent 状态分别为 `READY`／`PLANNING`／`READY`），`candidates` 含 t1／t3；`next_tasks` ⇒ `[t1, t3]`（按 `tasks` 输入序，不按候选序）；policy／maintenance 任务无条件保留 | 运行期 | 调用方仍可自己排序 |
| **J9** | 六组假策略：①`Advance(READY, 有任务)` ⇒ `Task`；②`Advance(RUNNING, 有会话)` ⇒ `ContinueSession`；③`Advance(RUNNING, 无会话)` ⇒ `NoActiveSession`；④`Advance(READY, 无任务)` ⇒ `UnauthorizedAdvance`；⑤`NoIntent` ⇒ `NoIntent` 且 `selected_intent() == None`；⑥策略 `Err` ⇒ `SupervisorError::Decision`，不是 `NoIntent` | 运行期 | P7B 策略内部错误（P7B 负责） |
| **J10-a** | `plan_manual` 与 `plan_auto` 都返回 `SessionPlan`；`mode()` 分别为 `Manual`／`Auto`；Manual 的 `selection() == None`；Auto 的 `selection()` 与假策略的 `Advance` 一致 | 运行期 | — |
| **J10-b** | 读 `src/session.rs` 的 `plan_manual` 函数体：不出现 `IntentScheduler::decide`／`decide(`／`SchedulerInputs`；只做四个候选段的拼接与 `SessionPlan` 构造 | 源码面 | 调用方仍可自己调 `decide`；那是 Auto |
| **J11** | `trybuild`：`SessionPlan { mode: ..., candidates: ..., ... }` ⇒ E0451；`SessionPlan::default()` ⇒ E0599（无 `Default`）；`serde_json::from_str::<SessionPlan>` ⇒ E0277（无 `Deserialize`）；读 `src/session.rs` 的字段声明与 `impl`，访问器恰五个 | 编译期＋源码面 | — |
| **J12** | `BackgroundNote` 的字段恰 `label`／`at_millis`；`grep -rn "Evidence\|CompletionVerdict\|continuum_verify" crates/continuum-supervisor/src` 零命中（非注释）；`BackgroundDigest` 只进 `SessionPlan.background`，不进 `SchedulerInputs` | 源码面＋运行期 | 调用方仍可把 finding 转成 Intent；那是 P5a／装配方 |
| **J13-a** | `declaration_for` 四来源表：`MaintenanceSchedule` ⇒ `{false,false,true}`；其余 ⇒ `{false,false,false}`；`admit` 对 `Critical` 压力下的 `BackgroundRuntime` ⇒ `ResourceDecision.action == Some(Suspend)`、`phase == Checkpointing`；`High` 下 `BackgroundRuntime` ⇒ `Some(Throttle)`、`Throttled`；`Low` 下 `BackgroundRuntime` ⇒ `None`、`Running`；`UserWorkload` 不出现（P7D 不产生该声明） | 运行期 | 实际降速多少（执行器；§10 第 9 条） |
| **J13-b** | `grep -rn "impl CheckpointPort\|enum ResourceAction\|struct ResourceDecision\|struct SuspensionState\|struct ResourceArbiter" crates/continuum-supervisor/src` 零命中（假实现在 `tests/`）；`grep -rn "PriorityTier\|AssemblyView\|ResourceObservation\|assembly" crates/continuum-supervisor/src crates/continuum-supervisor/Cargo.toml` 零命中（`src/` 非注释）；`Cargo.toml` 的 `continuum-arbiter` 无 `features = ["assembly"]` | 源码面 | 同一构建图统一启用特性时仍可命名；P9 的规范级强制 |
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
| `product_drive` 接受 `RUNNING` Intent | J3（`PLANNING→SUSPENDED→RUNNING` 组） | J4（kind 仍对） |
| `product_drive` 不校验 `product_id` | J5（`None`／空串组）、J22 | J3／J4 |
| `authorized_intent` 接受 Product | J4（`WrongKind` 不出现） | J3（Product 仍 READY） |
| 在 `decide` 里按 `priority` 排候选 | J7（源码面 sort／cmp）、J7-c（假策略选择被覆盖） | J9（状态映射仍对） |
| 在 `plan_manual` 里调 `decide` 并设 `selection` | J10（Manual 不选） | J11（形状仍对） |
| 在 `plan_auto` 里把 finding 当选择 | J10（selection 不来自 P7B）、J12 | J8（next_tasks 不变） |
| `BackgroundNote` 加 `Evidence` 字段 | J12（不是 Evidence） | J11（计划形状不变） |
| `admit` 把 `action == None` 当 `Run` | J13（`None` 不动作） | J5–J6（任务构造不变） |
| `checkpoint` 失败后报 `Suspended` | J13／J14 的 checkpoint 断言 | J3–J6 |
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
9. **P7D 是否应该持有 `SuspensionState` 表**：本设计把状态留给调用方；P7C §4.8 说「P7D 持有」。
   若协调者读作「P7D 必须持表」，则要新增内存表或 201 表。**收件人：P7C＋协调者。**
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
| 表、迁移、事件、审计 | P7D 零表（§9）；不占 201 |
| `kill`／终止进程／`NodeState` 迁移 | §14；P7C 的协作式挂起；P7D 不建 |
| `ResourceDirective`／§304 四问 | P7B 的端口；P7D 不回答；J21 |
| 「§7.4 已覆盖 P7D」的宣称 | §7.4 没有 P7D；本设计自报 J1–J3 等；J20 |

---

# 9. 持久化与共写文件

## 9.1 结论：零表、零迁移、零事件

本块**零表**。理由：

1. `SessionPlan` 是**一次 Session Activation 的计划值**，§11 没有要求持久化；它是 UI 的输入。
2. `AuthorizedTask` 是**本轮任务输入**，由调用方构造；P7D 不拥有任务表。
3. `BackgroundDigest` 是**摘要值**；「昨晚完成」的存储属于执行器／装配方，不是 P7D 的表。
4. §171 的作业记录表属于 P6C（`memory_synthesis_job`／`memory_synthesis_input`，迁移 190）；
   P7D 只调 P6C 的函数，不建表。
5. `SuspensionState` 是 P7C 的内存状态；P7D 不复制、不持久化。

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

**`P7_EXPECTED_ROWS`**（四行；按字母序；与各块设计边表逐行对应）：

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
   没有「selection」的语义。本设计定五字段与固定段序（§4.5、§4.6）。**缺的是「SessionPlan 的字段与
   两模式差异」这一条规范。收件人：规范维护者＋前端＋P8。** 代价：UI 要按固定五字段读；新增字段是
   破坏性改动；`candidates` 的段序可能不是 UI 想要的。

3. **`AuthorizedTask` 的授权证据只到「合同版本引用＋状态」**。切分 §三第 6 条选选项 (a)
   （`:226-243`）：`contract_id`＋`status`；P7D 不读合同内容、不验证合同存在、不防生产方自报。
   **缺的是「独立可验证的授权证据」这一步；选项 (b) 的 `ProductIntentGrant` 本轮不做。收件人：
   协调者＋第 5 层＋P7A。** 代价：`Intent` 字段是 `pub`，调用方可以伪造 `READY`；P7D 只能按字段
   校验，不能证明审查流程。

4. **§171 的「直接调」与「不新增 persist 边」相抵**。`begin_synthesis` 需要 `&Tx<'_>`
   （`synthesis.rs:108`）；要字面直调就必须依赖 `continuum-persist`，而切分 §三 P7D 行只允许该边
   「仅当会话／后台作业落库」（`:193`）。本设计选 `SynthesisIo` 端口（§4.8），P7D 仍直接依赖
   `continuum-memory` 的类型，但事务句柄留在装配方。**缺的是「触发点是否允许端口化」这一条裁定。
   收件人：协调者＋P6C＋装配方。** 代价：字面上不是 P7D 直接调 P6C 的自由函数；多一个端口实现；
   若协调者要求字面直调，改法是加 `continuum-persist` 边并换成 `&Tx<'_>` 的三个自由函数。

5. **`P6D` 合并门 rebaseline vs 退役**。本设计选 rebaseline（§5.7、§9.2）：31 行 fixture、四行 P7
   期望、两个 `27` → `31`、断言 2 `5` → `4`。**缺的是「P7D 是否允许改 P6D 的基线」这一条裁定；
   切分 §一 1.3 已把所有权给 P7D，但 P6D 的原文说所有者是「P6D（本轮）→ 协调者（P7 起）」
   （P6D §7.3.5 `:1069`）。收件人：协调者＋P6D。** 代价：rebaseline 是五处同改，漏一处会先红；
   退役会丢掉历史快照。

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

9. **`SuspensionState` 的持有者与 `throttle` 的量化**。P7C §4.8 `:680-683` 说「P7D 持有」
   `SuspensionState`；本设计把它留给调用方（`admit`／`checkpoint` 按 `&mut` 传入），P7D 不建任务表。
   P7C 的 `Throttle` 也不给量化；P7D 不给。**缺的是「P7D 是否必须持每任务状态表」与「throttle 的
   量化执行」两条。收件人：P7C＋协调者＋执行器。** 代价：若 P7D 必须持表，要加内存表或 201 表；
   若 throttle 要量化，要新增执行端口。

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

14. **`SessionPlan` 的 `selection` 不保证资源准入**。Auto 可能选中一个随后被 P7C `Suspend` 的任务；
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

17. **§171 的触发策略／阈值归 P10，P7D 只交触发点**。§171 只给「session 结束后／系统空闲时／
    大量项目状态改变以后／用户主动要求时」四个时机（`:931-938`），没有给空闲时长、数量阈值、检查
    频率、重试与恢复。本设计只提供 `SynthesisTriggerPoint` 的三个转发（§4.8），**不决定何时调用**。
    **缺的是「四个时机的判定策略、频率、重试」这一条；收件人：P10＋协调者。** 代价：P7D 交付后，
    Dreaming 仍没有自动触发器；若 P10 不接，只有调用面没有调度。**这是切分 §6.2 第 10 条
    `:409` 的边界，不是本块的漏项。**

---

# 11. 实测与口径订正（只记，不改）

以下行号／口径在本工作树实测；与切分文档或别家设计的旧行号不一致处，按本文件为准。

| # | 文档／它说 | 实测 | 处置 |
|---|---|---|---|
| 1 | 切分 §一 1.3 引 P6 fixture 与 `P6_EXPECTED_ROWS` 在 `:551` | `P6_EXPECTED_ROWS` 在 `dependency_direction.rs:551`；fixture 在 `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`（30 行＝3 注释＋27 数据） | 本文件按实测引 |
| 2 | 切分 §四第 1 条引解析器 `:615` 与断言 1 `:790` | 实测 `rows.len() == 27` 在 `:615`；`baseline.len() == 27` 在 `:790`；断言 2 `:799` | 本文件按实测引；rebaseline 同批改两个 `27` |
| 3 | 切分 §七第 11 条说 P7 先红的是断言 5 与断言 8 | 实测断言 5 在 `:837-854`、断言 8 在 `:882-895`；断言 1 只解析 fixture，初始树不会红 | 本文件按实测引；rebaseline 后仍改断言 1 |
| 4 | 切分 §一 1.3 说 P7D 顺取 201 | 本设计零表，**不占 201**；若将来落库再取 | 本文件 §9.1 |
| 5 | P7B 设计 §5.1 说 P7D 可经 `apply_intent_transition` 执行迁移 | 本设计不调用它：P7D 不写 `intent` 表；迁移由装配方／执行器按既有运行时路径做 | §4.4；若协调者要求 P7D 迁移，另立端口 |
| 6 | P7C 设计 §4.8 说「P7D 持有 `SuspensionState`」 | 本设计把状态留给调用方，按 `&mut` 传入；P7D 不建任务表 | §5.5、§10 第 9 条 |
| 7 | 切分 §2.3.1 第 4 条说 P7D「直接调」`begin/apply/fail` | `begin_synthesis` 需要 `&Tx<'_>`；本设计用 `SynthesisIo` 端口，事务在装配方 | §4.8、§10 第 4 条 |
| 8 | 任务材料把 §11 记在 `docs/spec/05-normative.md` | §11 在 `docs/spec/01-concepts.md:576-620`；`05-normative.md` 的 §299 在 `:1892-1903` | 本文件按实际文件引 |
| 9 | P6D 设计 §7.3.5 说 P7 新增 crate 会让断言 1 与断言 8 二选一必红 | 断言 1 只解析 fixture，不随新增 crate 红；先红的是断言 5／8；但 rebaseline 后两个 `27` 仍要改 | §3.9、§5.7 |
| 10 | 切分 §五 P7D 行说「不持产品状态」 | 本设计不登记 `continuum-product`、不读 Product 记录；`product drive` 只按值收 Intent | §2.1、§6 |

---

**本文的实测口径**：除 §11 的订正行外，正文引用的规范节号以 `docs/spec/05-normative.md`、
`docs/spec/01-concepts.md`、`docs/spec/03-product-drive.md` 为准；《工程》与《总纲》分别指
`docs/02-工程.md` 与 `docs/01-总纲.md`。零命中类断言已限定语料（`crates/continuum-supervisor/src`、
`Cargo.toml`）并逐处读过命中行；P6D 合并门的行号以本工作树的
`crates/continuum-runtime/tests/dependency_direction.rs` 为准。
