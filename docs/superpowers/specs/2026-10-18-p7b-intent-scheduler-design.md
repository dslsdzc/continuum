# P7B Intent Scheduler — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-18-p7-scope-and-split.md` §五 P7B 行的原话，逐字照用）：

> 建 Intent Scheduler，并冻结一个由装配方实现的具名资源端口，把「三层调度必须分离」做成可观察结构；
> Graph Scheduler 复用，Resource Scheduler 不重造

**本块拥有 / 产出的接口**：新建 `continuum-intent-scheduler`；`IntentDecision`、`IntentDecisionError`、
`SchedulerInputs`、`BackgroundSuitability`、`IntentSelectionPolicy`、`ResourceDirectivePort`、
`DirectiveForError` 与不透明指令载体 `ResourceDirective`。**消费** `continuum-semantics` 的 `Intent` 与
`continuum-graph` 的 `select_runnable` / `SchedulerConfig`；**不消费** P7C 的任何类型。

**规范依据**：§301 §302 §303 §304 §77 §78；《工程》§7.1（`docs/02-工程.md:420-431`）、§7.2（`:433-443`）、
§7.3（`:445-458`）、§7.4（`:460-467`）、§7.5（`:469-473`）。§7.4 的四条判据不覆盖 P7B（§3.7、§10 第 5 条）。

**前置事实（切分 §1.1 的 C1 订正）**：Graph Scheduler 已在第 3 层落地
（`crates/continuum-graph/src/scheduler.rs:28` 的 `select_runnable`，`crates/continuum-graph/src/lib.rs:29`
导出）；Resource Scheduler 在第 4 层只有碎片：`crates/continuum-model-registry/src/router.rs:199`
（`RoutingRequest`）／`:405`（`RankedExecutionCandidates`）／`:523`（`rank`），
`crates/continuum-node/src/placement.rs:362`（`place`，`src/` 零生产调用方，`:356-358`）。
**P7 只建 Intent Scheduler**；`ResourceDirectivePort` 是具名端口，不是 Resource Scheduler。
**P7 结束时 §304 仍不完整**，本设计不宣称三层调度已闭合（§3.4、§8、§10 第 4 条）。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（`docs/02-工程.md:561-567`）。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p7-plan`，基线 `c89d839`）上当场
`grep -n` / `sed -n` 实测的，未照抄任何一份别的文档。切分文档里已漂移的行号在 §11 逐条记出。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| `IntentDecision`：`Advance(IntentId)` 或 `NoIntent`，加一份不透明依据；`decide` 用 `Result<IntentDecision, IntentDecisionError>` 承载失败 | §302 `docs/spec/05-normative.md:1939-1955`；切分 §2.3.2 | §4.1 | 两臂 + 显式错误臂；不落库 |
| §302 的五个输入，显式形状 | §302；切分 §6.2 第 8 条 | §4.2 | 五输入全量；组合经策略 trait |
| `BackgroundSuitability` 的唯一投影载体 | 切分 §2.4；§13 `docs/spec/01-concepts.md:708-722` | §4.3 | 不透明载体；投影取值域不定义 |
| `IntentSelectionPolicy` 与确定性基线 | §302；《工程》§7.2 `:435` 的「模型判断」 | §4.4 | trait＋基线；模型驱动实现由装配方提供 |
| Graph Scheduler 的消费（复用，不重造） | §303 `:1959-1969`；切分 §三第 4 条 | §4.6 | 零逻辑转调；不判 READY／并发 |
| `ResourceDirectivePort` 与不透明指令值 | §304 `:1973-1982`；切分 §三第 5 条 | §4.7 | trait＋四槽载体；无实现 |
| 三层分离的结构照片 | §301 `:1925-1935`；§77 `docs/spec/02-positioning.md:616-631` | §5.4、J10 | 三个独立决策面 + 一个转交入口；无合并函数 |
| `NoIntent` 的负例 | 切分 §2.3.2（本设计订正其消费措辞） | §4.1、§5.3、J6–J8 | 空候选／无可推进候选／策略明确不推进三组；策略失败与越界走错误臂 |
| 不解析 `priority`／`workload_class` 的源码面判据 | 切分 §2.4 | J3、J4 | 类型层＋源码面 |

## 1.2 不建什么（摘要；完整表在 §8）

| 不建的东西 | 归属 / 理由 |
|---|---|
| Graph Scheduler 的 READY／并发判定、`Queued → Running`、节点状态迁移 | `continuum-graph` 已有（`scheduler.rs:28`）；P7B 只转调；切分 §五 P7B 行「不重写」 |
| Resource Scheduler 的 router／placement／工具选择／并行度 | 第 4 层只有碎片（`router.rs:199`／`:405`／`:523`；`placement.rs:362`）；切分 §6.2 第 12 条 |
| Resource Arbiter、`resource_pressure`／`workload_class`、§300 四档 | P7C；P7B **不依赖** `continuum-arbiter`；切分 §三第 5 条 |
| `intent` 表、迁移、第二套 Intent 查询 | P4 语义层只有 `load_intent`（`persist.rs:428`）与 `list_children`（`:448`）；切分 §6.2 第 5 条 |
| Intent 状态机的新状态／新迁移 | §223 `docs/spec/05-normative.md:172-205`；`IntentStatus::transition` 是 `pub(crate)`（`intent.rs:217`），P7B 调不到 |
| 模型调用、`InvokeRequest`、`ModelProvider::invoke` | P3G 的模型调用路径；P7B 不依赖 `continuum-provider`／`continuum-runtime` |
| 第二个「最值得继续」排名 | P7B 的 `IntentSelectionPolicy` 是唯一决策面；P7D 复用（切分 §五 P7D 完成判据 2） |
| 持久化、事件、审计表 | 本块零表、零事件（§9） |
| 「§304 已闭合」的宣称 | **不宣称**；端口只承载，不计算；切分 §1.1、§6.2 第 12 条 |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：**`continuum-intent-scheduler`**。

**名字的四条理由**：

1. `Intent Scheduler` 是 §301／§77 的唯一词根；切分 §1.2 建议名即此，设计采用，不另造同义词。
2. 名字只表示 Intent Scheduler，**不表示拥有 Graph／Resource Scheduler**（切分 §1.2 `:60-61`）。
   Graph Scheduler 仍在 `continuum-graph`；Resource Scheduler 不在 P7 内。
3. 不与其余三块争名：`continuum-product` 归 P7A、`continuum-arbiter` 归 P7C、
   `continuum-supervisor` 归 P7D。
4. **实测未占用**：`grep -rn "continuum-intent-scheduler\|continuum_intent_scheduler" crates/ Cargo.toml`
   在本仓**零命中**（2026-10-18，`p7-plan` 工作树；命中只在切分文档的建议名里，不在代码里）。

依赖边表（**只登记实际用到的**，切分 §四第 5 条）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-semantics` | `Intent`（`crates/continuum-semantics/src/intent.rs:434`）、`IntentId`（`crates/continuum-semantics/src/ids.rs:41`）、`Priority`（`intent.rs:300`）、`IntentStatus`（`intent.rs:61`） | 普通 |
| `continuum-graph` | `AdfirGraph`（`crates/continuum-graph/src/graph.rs`）、`NodeId`（`crates/continuum-graph/src/ids.rs:40`）、`SchedulerConfig`（`scheduler.rs:9`）、`select_runnable`（`scheduler.rs:28`） | 普通 |
| `continuum-canonical` | `EntityId`（`crates/continuum-canonical/src/candidate.rs:62`）、`Opaque`（`crates/continuum-canonical/src/lib.rs:152`）；仅 `tests/` 构造 `Intent` 夹具时使用 | **dev** |

**为什么 dev 边必须有 `continuum-canonical`**：J7／J8／J9／J13 要构造候选 `Intent`。`Intent.goal: EntityId`
（`crates/continuum-semantics/src/intent.rs:442`）与 `Intent.completion_predicate: Opaque`（`:467`）是仅有的
两枚无法从 `continuum-semantics` 构造的字段；`continuum-semantics` 没有 re-export 它们（`lib.rs:273-275`）。
`Intent` 没有 `Intent::new`（字段是 `pub`，测试用结构体字面量构造），但 `EntityId`／`Opaque` 只能从
`continuum-canonical` 构造；`load_intent` 又要 `continuum-persist`（`persist.rs:428`，本块不登记）。
`dependency_direction.rs` 的 `cargo_tree_direct` 用 `--edges all`（`:7-11`、`:480`），把 dev 边算进直接边；
故 `ALLOWED` 行是三项。

**不登记**的边，逐条给理由（零使用的边即假边）：

- **不登记 `continuum-persist`**：本块零表、零迁移；候选集以 `&[Intent]` 输入，**不直读 `intent` 表**
  （切分 §6.2 第 5 条）。`continuum-semantics` 的 `load_intent`（`persist.rs:428`）与
  `list_children`（`:448`）都不是本块的调用面。
- **不登记 `continuum-arbiter`**：P7C 的 crate 在 P7B 设计时尚未落地；P7B 只定义
  `ResourceDirectivePort` 的 trait 与不透明指令值，**不依赖 P7C**（切分 §三第 5 条）。
  `P7B → 端口 ← P7C` 由装配方汇合，两块设计可全并行。
- **不登记 `continuum-model-registry` / `continuum-node`**：router／placement 在第 4 层，
  P7B **只经具名端口**；`RoutingRequest`（`router.rs:199`）、`RankedExecutionCandidates`（`:405`）、
  `rank`（`:523`）、`place`（`placement.rs:362`）都不出现在本块的 `Cargo.toml` 与源码里。
- **不登记 `continuum-runtime` / `continuum-provider`**：P7B 是库，装配方是 `continuum-runtime`；
  P7B 不调用 `ModelProvider`，不构造 `InvokeRequest`。模型驱动的 `IntentSelectionPolicy` 实现由装配方提供，
  它可以在内部走 P3G 的既有路径（`crates/continuum-runtime/src/model_call.rs:430` 的 `call`）。
- **不登记 `continuum-events`**：零事件。
- **不登记 `continuum-core`**：不构造 `Message`／`ToolResult`。

**dev 边：一条**（`continuum-canonical`，仅 `tests/`）。本块的测试用本 crate、两个普通依赖与这一条 dev 边；
假 `ResourceDirectivePort` 与假 `IntentSelectionPolicy` 由本 crate 的 `tests/` 自己写。

**共写文件**（切分 §一 的共写文件表给的是六行；本块只碰**两处**）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members`（`:3`） | 加一行 `crates/continuum-intent-scheduler` | 本块**只加自己这一行** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-intent-scheduler", &["continuum-canonical", "continuum-graph", "continuum-semantics"])`——**三项**（两普通 + 一 dev），与上表逐行对应；数组按字母序 | 新 crate 会先让 `every_crate_depends_only_on_its_allowed_set`（`:494`）变红再被补齐，这是刻意的 |

**本块不碰** `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`）、
`crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`）、
`crates/continuum-runtime/tests/startup.rs` 的三处计数——本块零表（§9）。
**本块不动 `continuum-runtime` 的 `ALLOWED` 行**：端口实现属装配方的下一轮，P7B 设计不落实现；
若装配方在本轮实现端口，它才在自己的行里加 `continuum-intent-scheduler`（切分 §三第 7 条）。

**`Cargo.lock` 会变**：新 crate 的加入会让锁文件增枚；提交时按显式路径带上锁文件（同 P6A 设计 §2.1 的处置）。

## 2.2 冻结清单：本块持有的类型与入口

**类型**（定义在 `continuum-intent-scheduler`）：

```
IntentDecision            —— Advance { intent, basis } / NoIntent { basis }（§4.1）
IntentDecisionError       —— 策略失败 / 越界 Advance 的显式错误臂（§4.1、§4.5）
DecisionBasis             —— 不透明依据；String 载体（§4.1）
SchedulerInputs<'a>       —— 候选集 + §302 五输入（§4.2）
PrioritySignal            —— 每枚候选的显式 priority；不透明（§4.2）
Deadline                  —— 每枚候选的显式 deadline；毫秒（§4.2）
Blocking                  —— 每枚候选的显式 blocking；出边集合（§4.2）
UserFocus                 —— None / Intent(IntentId) / Area(String)（§4.2）
BackgroundSuitability     —— 装配方唯一投影的不透明载体；整轮单值（§4.3）
IntentSelectionPolicy     —— 排序策略 trait；失败走 Result（§4.4）
BaselineIntentPolicy      —— 确定性基线策略（§4.4）
IntentScheduler           —— 决策入口 decide / 转交入口 directive_for（§4.5、§4.8）
ExecutionRequest<'a>      —— Intent + 可选 NodeId + 显式信号（§4.7）
ExecutionSignals<'a>      —— priority / deadline / background suitability（§4.7）
ResourceDirective         —— 四槽不透明指令值（§4.7）
ResourceDirectiveError    —— 端口错误；不透明 String（§4.7）
DirectiveForError         —— 转交入口的错误：非法 decision / 端口失败（§4.8）
ResourceDirectivePort     —— 资源端口 trait（§4.7）
```

**决策面**（三个，互不合并；§5.4）：

```
IntentScheduler::decide     -> Result<IntentDecision, IntentDecisionError>
graph::runnable_nodes       -> Vec<NodeId>（零逻辑转调 continuum-graph）
ResourceDirectivePort::direct -> Result<ResourceDirective, ResourceDirectiveError>
```

**转交入口**（一个，不产出新决策；§4.8、J10-c）：

```
IntentScheduler::directive_for -> Result<Option<ResourceDirective>, DirectiveForError>
```

**模块落点**（源码面判据的语料按这张表；不新增 crate 外的路径）：

| 文件 | 内容 |
|---|---|
| `src/lib.rs` | 只做 `pub mod` 与 `pub use`，不定义类型 |
| `src/decision.rs` | `IntentDecision`／`IntentDecisionError`／`DecisionBasis`／`IntentSelectionPolicy`／`BaselineIntentPolicy`／`is_advanceable`／`IntentScheduler::decide` |
| `src/inputs.rs` | `SchedulerInputs`／`PrioritySignal`／`Deadline`／`Blocking`／`UserFocus`／`BackgroundSuitability` |
| `src/graph.rs` | `runnable_nodes`（零逻辑转调） |
| `src/resource.rs` | `ExecutionRequest`／`ExecutionSignals`／`ResourceDirective`／`ResourceDirectiveError`／`DirectiveForError`／`ResourceDirectivePort`／`IntentScheduler::directive_for` |

**不新增的类型**：不定义 `Intent` / `IntentId` / `Priority` / `IntentStatus`（P4 的）、
`AdfirGraph` / `NodeId` / `SchedulerConfig` / `select_runnable`（P1 的）、
`RoutingRequest` / `RankedExecutionCandidates` / `rank`（第 4 层 model-registry 的）、
`PlacementRequest` / `place` / `ComputeNode`（第 4 层 node 的）、
`resource_pressure` / `workload_class` / §300 四档（P7C 的）、
`Decision`（§331 的决策单；本块的 `DecisionBasis` 不是它，也不写 `decision.required`）。

**本块不持有的判定**：READY／并发／节点状态迁移（`continuum-graph`／P1）；放置、模型选择、工具选择、
并行度（第 4 层，经端口）；资源压力、负载类别、§300 四档（P7C）；Intent 的存储、状态迁移、
版本语义（P4）；§331 的 `Decision`（P7A 的 Progress Gate 用它，不是本块）。

## 2.3 同名异物（**都不是**同一个概念）

| 同名词／类型 | 它是什么 | 实测位置 | 本块的处置 |
|---|---|---|---|
| `priority` | `Intent.priority` 是**不透明** `Priority`／§302 的输入之一／§78 的七因子／§300 的四档资源优先级／§79 的四档交互优先级 | `crates/continuum-semantics/src/intent.rs:300`；`docs/spec/05-normative.md:1950`；`docs/spec/02-positioning.md:635-651`；`docs/spec/05-normative.md:1907-1921`；`docs/spec/02-positioning.md:655-669` | P7B 只收 `PrioritySignal`；**不比较**、不排序、不建第二个 `Priority`；`Priority` 没有 `Ord`／`PartialOrd`（`intent.rs:288-290`） |
| `workload_class` | §13 的四档进程类／`BackgroundSuitability` 的投影源／`resource_pressure` 的另一项投影源 | `docs/spec/01-concepts.md:708-715`、`:717-722` | P7B 只收不透明 `BackgroundSuitability`；**不定义四档**、不读 `workload_class`／`resource_pressure` |
| `Decision` | §331 的决策单／`DecisionBasis` 的不透明依据 | `crates/continuum-semantics/src/decision.rs:80` | 两个词不混用；`DecisionBasis` **不是** `Decision`，本块不写 `decision.required` |
| `Suspended` | `NodeState::Suspended`／`IntentStatus::Suspended` | `crates/continuum-graph/src/node.rs:63`；`crates/continuum-semantics/src/intent.rs:77` | P7B 只把 `IntentStatus::Suspended` 当作可推进候选状态之一；**不碰** `NodeState::Suspended`，不建挂起状态机（P7C 的） |
| `NodeId` / `IntentId` | 执行层节点身份／语义层 Intent 身份 | `crates/continuum-graph/src/ids.rs:40`；`crates/continuum-semantics/src/ids.rs:41` | 两个不同层的 id；`runnable_nodes` 只返回 `NodeId`，`decide` 只返回 `IntentId`，无转换 |
| `ResourceDirective` | P7B 的四槽不透明载体／`RoutingRequest`／`PlacementRequest`／`SchedulerConfig` | §4.7；`router.rs:199`；`placement.rs:202`；`scheduler.rs:9` | P7B 只定义载体；不定义 router／placement／Graph 配置的替代品 |
| `Blocking` | `Decision.blocking: bool`（§331 决策单的「是否阻塞」）／本块的 `Blocking { intent, blocks }`（出边集合） | `crates/continuum-semantics/src/decision.rs:92`；§4.2 | 同名不同物；P7B 只收出边集合，不读 `Decision.blocking` |
| `Deadline` | `model_call.rs` 的 `deadline: Option<Duration>`（单次模型调用的超时）／本块的 `Deadline { intent, at_millis }`（Intent 的绝对截止毫秒） | `crates/continuum-runtime/src/model_call.rs:434`；§4.2 | 同名不同物；P7B 的 deadline 是毫秒时间点，不是调用超时 |
| `NoIntent` | 调度结果的一臂（`IntentDecision::NoIntent`）／**不是** `IntentStatus` 的变体（更不是终态） | §4.1；`intent.rs:61` | `NoIntent` **不是** Intent 状态，也不是哨兵 `IntentId`；P7D 收到它时**保留既有会话**，不得清理或关闭它们 |

## 2.4 本块的层间位置

§9.1（`docs/02-工程.md:569-588`；读法 `:561-567`；`:588` 逐字「依赖方向单向，无环。」）里与本块有关的几条：

- **`资源层 (4) → 长期循环 (7)`、`边界层 (5) → 7`、`认知层 (6) → 7`、`跨领域 (8) → 7`** 是既有方向；
  P7B 依赖第 2 层的 `continuum-semantics` 与第 3 层的 `continuum-graph`，不是反向边。
- **`continuum-graph` 是第 3 层的既有件**：P7B 消费 `select_runnable`，**不接管** `Queued → Running`、
  不接管节点状态迁移。这条是**第 3 层 → 第 7 层的直连边**，不在 §9.1 的相邻层链上，但**不成环**，
  且与 P5 分块「只消费第 3 层既有判据」同形（切分 §三第 4 条）。本设计按这一句写：
  `continuum-graph` 不依赖 `continuum-intent-scheduler`；方向是 P7B → graph。
- **P7B 是 P7D 的上游**：《工程》§7.3 `:453` 的 `Background Supervisor ← Resource Arbiter + Intent Scheduler`
  与 `:454` 的 `Session Orchestrator ← Intent 存储 + Background Supervisor`。P7D 消费本块的
  `IntentDecision`；本块**不依赖** P7D。`Intent 存储` 不在本块（切分 §6.2 第 5 条）。
- **P7C 与 P7B 无未冻接口**：`BackgroundSuitability` 的投影只在装配方实现一次；
  `ResourceDirectivePort` 的实现由装配方消费 P7C 的结论。**P7C 结论由端口实现持有，不是 trait 参数**：
  `direct` 只收 `ExecutionRequest`；实现方在构造端口值时持有 `resource_pressure`／`workload_class`／§300 四档
  （快照或句柄），调用时读自己的状态。P7B 不引入 P7C 的任何类型。若 P7C 的设计要求每次请求传入结论，
  则在本 trait 增加一个不透明 `ResourceContext` 参数——那是 P7C 落地时的接口变更，不是本设计的缺口；
  本轮不加。P7B 不依赖 P7C，P7C 不依赖 P7B（切分 §三第 5 条）。
- **P7D 的上游决策面不是 READY-only**：候选状态集是 `READY` 加五个活跃状态
  （`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`EVOLVING`）；其中 `EVOLVING` 仅在 P7D 确有
  对应活跃会话时才进入候选。`Advance` 只给 id，P7D 按候选状态走 §223 的既有迁移或不迁移（§5.1）。
  `NoIntent` 不表示既有会话不存在；P7D 必须保留既有会话。

## 2.5 本块发明的判据（逐条：规范给到哪、代价、边界、落点、收件人）

**这一节是本块的「发明判据」总账（共 16 条）。** 规范没给、而本设计已识别的自定判定，在下表里各有一条；
**以下每一条都是「本设计自定」**（不是规范转录），规范给到哪一步见第三列，代价与收件人见第四、七列。
**本表是本轮已识别的自定判定，不宣称穷尽；未识别的判定在实现轮补表。** 代码块只是示意，正文措辞才是约束。

| # | 发明的判定 | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | `IntentDecision` 的 Rust 形状：`Advance { intent, basis }` / `NoIntent { basis }` | §302 只给「现在推进哪个 Intent」；切分 §2.3.2 只给 `Advance(IntentId)`／`NoIntent` 加「不透明依据」 | 消费方按变体 match；加第三臂是破坏性改动 | 不覆盖「同一轮推进多个 Intent」；不覆盖「Advance 之后失败」 | §4.1 | 规范维护者 ＋ P7D |
| 2 | `DecisionBasis` 是 `String` 载体，不透明 | 切分 §2.3.2 只说「不透明的依据」，**没给形状** | 没有结构化原因；P7D 只能展示，不能按原因分支 | 不覆盖「依据的可审计结构」；不覆盖「依据的持久化」 | §4.1 | P7D ＋ 规范维护者 |
| 3 | 候选资格：`IntentStatus` 的六个可推进状态——`Ready`／`Running`／`Waiting`／`Suspended`／`Verifying`／`Evolving`；`Advance` 只给 id，P7D 按候选状态分派 | §302 没给候选状态集；§223 只给状态机 | 候选集可同时含新工作与活跃会话；P7D 必须自己区分「启动」与「继续」；`Evolving` 仅在 P7D 确有对应活跃会话时才进入候选；`Proposed`／`Specifying`／`Planning`／`Review`／`Completed`／`Failed`／`Cancelled` 不会被选中 | 不覆盖「P7D 如何构造候选集」；不覆盖「状态迁移本身」——P7B 不调用 `transition` | §4.4、§4.5、§5.1 | 规范维护者 ＋ P7D |
| 4 | 五输入的显式形状：`PrioritySignal` / `Deadline` / `Blocking` / `UserFocus` | §302 只给五个名字；切分 §6.2 第 8 条说四个没有落点 | 调用方要构造四类记录；`PrioritySignal` 与 `Intent.priority` 可能不一致，P7B 不校验 | 不覆盖「信号的存储与读取」；同一 Intent 多条信号的合并见第 15 条 | §4.2 | 规范维护者 ＋ 装配方 |
| 5 | `BackgroundSuitability` 是 `String` 不透明载体，**整轮单值**，**不定义** `Suitable`／`Unsuitable` | 切分 §2.4 只说「不透明」；§13 只给 `workload_class` 四档与 `resource_pressure` 示例，**投影取值域未给** | 基线策略不能使用它；投影取值域由装配方定义；P7B 无法验证投影是否正确 | 不覆盖「投影值的语义」；不覆盖「装配方把四档原文塞进载体」——那是装配方的契约 | §4.2、§4.3 | P7C ＋ 装配方 ＋ 规范维护者 |
| 6 | `IntentSelectionPolicy` trait ＋ `BaselineIntentPolicy` 的次序：可推进候选内，用户聚焦 > 最早 deadline > 最大 blocking 度 > 输入序 | §302 没给组合函数／权重；§78 只给因子与「显式 priority 最高语义权重」 | 基线**不读** `priority` 与 `background_suitability`；不同装配方可换策略，排名不可比；P7D 必须复用本策略，不能另写 | 不覆盖「priority 的最高语义权重」——`Priority` 不透明且候选序不能补救；不覆盖「多目标的加权」 | §4.4 | 规范维护者 ＋ 协调者 ＋ P7D |
| 7 | `decide` 校验策略输出：`Advance` 的 id 必须是候选且状态可推进，否则返回 `Err(InvalidAdvance)`；策略自身失败返回 `Err(PolicyFailed)` | 规范没给「策略越界／策略失败」的处置 | 调用方必须处理 `Result`；坏策略不再静默变成 `NoIntent` | 不覆盖「策略的副作用」；不覆盖「策略内部重试」——由策略实现负责 | §4.5 | P7D ＋ 协调者 |
| 8 | `graph::runnable_nodes` 是零逻辑转调 | 切分 §三第 4 条说 P7B 消费 `select_runnable`；§301 说三层必须分离 | P7B 暴露一个 Graph 入口，可能被误读为拥有 Graph Scheduler | 不覆盖「谁在何时调用它」；不覆盖「Graph 的装配点」 | §4.6 | 协调者 ＋ P1／第 3 层 |
| 9 | `ExecutionRequest` 的字段：`Intent` ＋ 可选 `NodeId` ＋ 三个信号 | §304 只说「执行请求」，**没给形状** | 节点可选；没有图句柄；装配方要自己解析 `Intent.graph_id` 与图 | 不覆盖「多节点执行请求」；不覆盖「图级并发」 | §4.7 | 装配方 ＋ P1 |
| 10 | `ResourceDirectivePort::direct` 是同步、对象安全、返回 `Result<ResourceDirective, ResourceDirectiveError>` | §304 没给签名 | 阻塞调用；没有 async；装配方要把 `rank`／`place` 的错误映射成不透明错误 | 不覆盖「长耗时资源决策的取消」；不覆盖「并发端口的线程安全」——`&self` 由实现负责 | §4.7 | 装配方 ＋ P7C |
| 11 | `ResourceDirective` 是四槽 `Option<String>`：`node_placement` / `model` / `tool` / `parallelism`；**无 `Default`** | §304 给四个问题，**没给类型** | 消费方要解析槽内容；没有逐槽类型安全；`None` 不是默认值，是「未答」 | 不覆盖「四问的语义正确性」；不覆盖「槽内容的版本」 | §4.7 | 装配方 ＋ P8 ＋ 规范维护者 |
| 12 | `directive_for` 在 `NoIntent` 时返回 `Ok(None)`，**不调用端口** | 规范没给 | `NoIntent` 没有指令；装配方要处理 `None` | 不覆盖「`NoIntent` 时的资源释放」——本块不持资源 | §4.8 | P7D ＋ 装配方 |
| 13 | 零持久化、零事件 | 切分 §三第 2 条说「仅当调度结果落库」才登记 `continuum-persist`；本设计选不落库 | 调度决策不可审计；P7D 若要审计必须自己落 | 不覆盖「决策回放」；不覆盖「跨进程恢复」 | §9 | 协调者 ＋ P7D |
| 14 | `background_suitability` 是**整轮单值**，不是每候选一条 | §302 只给名字；§13 的投影是资源结论 | 同一轮里不能表达「A 适合后台、B 不适合」；投影粒度由装配方承担 | 不覆盖按候选的背景适合度；不覆盖 `BackgroundSuitability` 的取值域 | §4.2、§4.3 | P7C ＋ 装配方 ＋ 规范维护者 |
| 15 | 重复候选与多信号的合并：重复 `IntentId` 是调用方契约违反，不校验、不定义选哪一枚；同一 Intent 多条 `priority` 取输入序第一条（基线不读）；多条 `deadline` 取最早；多条 `Blocking` 合并后按 `as_str()` 去重计条数 | §302 只给名字；§78 的 `blocking degree` 是解释面 | 重复候选的行为不保证；`priority` 的第一条选择对基线无影响 | 不覆盖「同一 Intent 两个候选的元数据不一致」；不覆盖「按候选聚合其他信号」 | §4.2、§4.4、§4.8 | 规范维护者 ＋ 装配方 |
| 16 | `directive_for` 的错误面：非法 `Advance` ⇒ `Err(InvalidDecision)`（契约违反，不调端口）；端口失败 ⇒ `Err(Port(e))`（原样透传） | 规范没给 | 两种失败分开；P7D 可区分「自己的 decision 坏了」与「端口坏了」 | 不覆盖「端口错误的重试／降级」——装配方负责 | §4.8 | P7D ＋ 装配方 |

## 2.6 切分 §四 六条横切约束的逐条落实

| 切分 §四 | 要求（要点） | 本设计的落点 | 可观察判据 |
|---|---|---|---|
| **第 1 条**（`:253-264`） | 不新增反向边；四块各自在 `ALLOWED` 加自己一行；既有行不因 P7 增枚，唯一例外是装配方 `continuum-runtime` | §2.1 的边表（两普通 + 一 dev，全部是本块指向第 2／3 层）；§2.4 的「graph 不依赖本块」 | J16：`ALLOWED` 的 `continuum-intent-scheduler` 行恰是三项；`cargo test -p continuum-runtime --test dependency_direction` 在含本 crate 的树上绿 |
| **第 2 条**（`:265-276`） | 迁移号段 200–209；P7A 占 200，P7D 顺取 201；P7B 无表不占号 | §9：本块零表、零迁移；`runtime_migrations()` 不加行 | J5：`Cargo.toml` 无 `continuum-persist`；`runtime_migrations()`／`expected_migrations()`／`startup.rs` 无本块改动 |
| **第 3 条**（`:277-285`） | 同一件事两个词汇表 | §2.3 的九组同名异物；§4.2／§4.3 的「不解析」 | J3、J4、J10 的源码面 |
| **第 4 条**（`:286-293`） | 工具结果消费与 Shippable 三值 | **不适用**：P7B 不消费 `Evidence`／`CompletionVerdict`，不判 Shippable | 无；本块没有这条的落点 |
| **第 5 条**（`:294-296`） | 依赖边只登记实际用到的；`ALLOWED` 与实际依赖精确一致 | §2.1 的两行普通边 + 一行 dev 边与 `ALLOWED` 的三项一一对应；dev 边一条 | J16 的两个方向：已声明而越界、本表列了却没用到，都绿 |
| **第 6 条**（`:297-311`） | P7B → P7D 的决策面先冻；P7B 与 P7C 无未冻接口；`BackgroundSuitability` 只投影一次；端口实现持有 P7C 结论 | §4.1 冻结决策面；§4.3 冻结载体；§4.7 冻结端口；§6 的接缝表 | J10、J13；P7D 对着 §4.1 写，P7C 对着 §4.7 写 |

---

# 3. 规范依据逐条

**这一节的写法**：每条先给规范原文的落点，再给「规范给了什么 / 规范没给什么 / 本设计怎么落」。
**没有一条判据是本节发明的**——规范没给的部分在 §2.5 里另记。

## 3.1 §301 Scheduler 分层（`docs/spec/05-normative.md:1925-1935`）

**原文（逐字，`:1927-1935`）**：

> 必须区分：
> ```
> Intent Scheduler
> Graph Scheduler
> Resource Scheduler
> ```
> 不得把三者实现成一个不可分离的大模型决策。

**规范给了什么**：三个调度器必须区分；不得合并成一个不可分离的大模型决策。

**规范没给什么**：三者的类型、入口、数据形状；「区分」是类型层、crate 层还是调用层。

**本设计怎么落**：三个独立决策面 + 一个转交入口（§5.4）：
1. `IntentScheduler::decide` 只返回 `Result<IntentDecision, IntentDecisionError>`；
2. `graph::runnable_nodes` 只返回 `Vec<NodeId>`，且是 `continuum-graph::select_runnable` 的零逻辑转调；
3. `ResourceDirectivePort::direct` 只返回 `Result<ResourceDirective, ResourceDirectiveError>`，实现不在 P7B；
4. `IntentScheduler::directive_for` 只把已作出的 `IntentDecision` 转交给端口，不产出新的 Intent／Graph 决策。

**没有任何一个函数同时产出三者**（J10）；`directive_for` 是转交入口，不属于三层决策面（J10-c）。
Intent 入口的模型判断经 `IntentSelectionPolicy` 端口，P7B 不直接调 `ModelProvider`（J14）。

## 3.2 §302 Intent Scheduler（`:1939-1955`）

**原文（逐字，`:1941-1955`）**：

> 负责：
> ```
> 现在推进哪个 Intent
> ```
> 输入：
> ```
> priority
> deadline
> blocking
> user focus
> background suitability
> ```

**规范给了什么**：Intent Scheduler 的职责是「现在推进哪个 Intent」；五个输入的名字。

**规范没给什么**：五个输入的类型、取值范围、组合函数、权重；哪个输入是每枚候选的、哪个是整轮的。
实测：**只有 `priority` 在 `Intent` 上**（`crates/continuum-semantics/src/intent.rs:447`），且它是不透明
`Priority`（`:300`）；`deadline`／`blocking`／`user focus`／`background suitability` **在 `Intent` 上没有落点**
（切分 §6.2 第 8 条）。`continuum-semantics` 也没有「按状态列 Intent」的公开查询：只有 `load_intent`
（`persist.rs:428`）与 `list_children`（`:448`）（切分 §6.2 第 5 条）。

**本设计怎么落**：五个输入全部做成**显式输入**（§4.2），候选集以 `&[Intent]` 输入；不读 `intent` 表；
不解析 `Intent.priority`。组合函数做成 `IntentSelectionPolicy`（§4.4），P7B 提供一个确定性基线，
不宣称它是规范排名（§10 第 1 条）。

## 3.3 §303 Graph Scheduler（`:1959-1969`）

**原文（逐字，`:1961-1969`）**：

> 负责：
> ```
> 哪些 ADFIR Node READY
> 哪些可以并发
> 哪些必须等待
> ```
> 主要为确定性 Runtime 逻辑。

**规范给了什么**：Graph Scheduler 的三个问题；它主要是确定性 Runtime 逻辑。

**规范没给什么**：READY 的具体判据、并发上限、与 Intent Scheduler 的交接形状。

**本设计怎么落**：**不重写**。`continuum-graph` 已有 `select_runnable`（`scheduler.rs:28`，
READY 条件在 `:22-27`，并发上限在 `:14-17` 与 `:37`）；P7B 只提供 `graph::runnable_nodes` 的零逻辑转调
（§4.6）。P7B 不接管 `Queued → Running`，不接管节点状态迁移，不解释 `max_parallel`。

## 3.4 §304 Resource Scheduler（`:1973-1982`）

**原文（逐字，`:1975-1982`）**：

> 负责：
> ```
> Node 在哪里执行
> 用哪个模型
> 哪个工具
> 多少并行
> ```

**规范给了什么**：Resource Scheduler 的四个问题。

**规范没给什么**：四个问题的类型、输入、算法；与 Intent／Graph 的交接形状。

**本设计怎么落**：**P7B 只冻结具名端口，不实现 Resource Scheduler**（§4.7）：
`ResourceDirectivePort` 的输入是一次执行请求（`Intent` ＋ 可选 `NodeId` ＋ 显式信号），输出是
`ResourceDirective` 四槽不透明载体。装配方实现端口，内部转调 `rank`（`router.rs:523`）／
`place`（`placement.rs:362`）／既有工具路径。**端口不是 Resource Scheduler 本体**：
它不判 READY、不选模型、不选工具、不定并行度，只承载装配方的答案。

**P7B 结束时不宣称 §304 已闭合。** 第 4 层只有碎片：`rank`／`place` 之外，
「工具选择」「并行度决策」与 `place` 的装配点都不存在；`place` 在 `src/` 里零生产调用方
（`placement.rs:356-358`）；`ExecutionProfile.compute_node` 仍是裸 `Option<String>`
（字段在 `crates/continuum-graph/src/execution.rs`；`crates/continuum-node/src/placement.rs:357-359`
记录了它没有装配点）。
本块的 `ResourceDirective` 四槽只保证「能承载」，不保证「有答案」。这条缺口登记在切分 §6.2 第 12 条，
收件人：P3／第 4 层持有方 ＋ 协调者。

## 3.5 §77 Scheduler 分层（`docs/spec/02-positioning.md:616-631`）

**原文（逐字，`:618-631`）**：

> 至少需要三个调度层。
> ```
> Intent Scheduler
> 决定哪个任务现在值得推进
>
> Graph Scheduler
> 决定哪些 ADFIR 节点可以并发执行
>
> Resource Scheduler
> 决定在哪个模型/设备/资源上执行
> ```
> 不能把它们全部塞进一个 scheduler。

**规范给了什么**：三个调度层各自「决定什么」；不能塞进一个 scheduler。

**规范没给什么**：三个调度层的类型／入口。

**本设计怎么落**：§4.1 的 `decide` 对应「哪个任务现在值得推进」；§4.6 的 `runnable_nodes` 对应
「哪些 ADFIR 节点可以并发执行」；§4.7 的 `ResourceDirectivePort` 对应「在哪个模型/设备/资源上执行」。
三个决策面的返回类型互不相同（J10）；`directive_for` 是转交入口，不参与这个返回类型比较。

## 3.6 §78 Priority Model、§300 Resource Arbiter、§13 Resource Arbiter（`02-positioning.md:635-651`；`05-normative.md:1907-1921`；`01-concepts.md:669-724`）

**§78 给了什么**：任务优先级不能只依赖用户手动设置；可以考虑 `deadline`／`dependency`／`user pin`／
`expected value`／`blocking degree`／`resource opportunity`／`background suitability`；
**「用户显式 priority 应拥有最高语义权重」**（`:651`）。

**§78 没给什么**：七因子的量纲、权重、组合函数；`Priority` 的取值域。

**本设计怎么落**：五个显式输入覆盖了 §302 的五个名字；§78 的七因子不全部进本块——P7B 只接
§302 的五个名字（`priority`／`deadline`／`blocking`／`user focus`／`background suitability`），
不接 `dependency`／`expected value`／`resource opportunity`（`dependency` 属 Intent／Graph 的边界，
`expected value`／`resource opportunity` 属 Resource 的解释面）。
**`priority` 不透明，P7B 不能解析**（`Priority` 没有 `Ord`／`PartialOrd`，`intent.rs:288-290`）；
基线策略因此**不按 priority 排序**。这是本设计对 §78 的一处**如实缺口**：若要让显式 priority 拥有最高语义权重，
需要（a）P4 给 `Priority` 一个可比较的取值域，或（b）装配方提供一个解释 `Priority` 的策略实现。
**不能用「按 priority 排候选」补救**：基线的第 3／4 步（deadline／blocking）会先于输入序决定结果，
候选序只在没有 deadline／blocking 时兜底。**收件人：规范维护者 ＋ P4 语义层持有方**（§10 第 2 条）。

**§300 给了什么**：四档资源优先级 `User workload > Foreground Runtime > Background Runtime > Maintenance`
（`:1911-1919`）；Resource Arbiter SHOULD 只向 AI 暴露最小化资源结论（`:1921`）。

**本设计怎么落**：P7B **不建 §300 的四档**，也不把它们放进 `ResourceDirective` 的槽名；四档只由
`ResourceDirectivePort` 的实现（装配方）消费，P7B 不读。`ResourceDirective` 的四槽是 §304 的四个问题，
不是 §300 的四档（§2.3）。

**§13 给了什么**：`workload_class` 的四档进程类 `INTERACTIVE_GAME` / `COMPUTE_WORKLOAD` / `COMPILER` /
`VIDEO_RENDER`（`:708-715`）；交给 AI 的仅是 `resource_pressure` 与 `workload_class`（`:717-722`）。
**规范的一处笔误**：输出示例里的 `INTERACTIVE`（`:721`）**不在四档里**；切分 §2.4 按笔误处理，本设计不设
第五档。

**本设计怎么落**：`BackgroundSuitability` 是装配方从 `workload_class` ＋ `resource_pressure` 做的**一次**
投影（§4.3）；P7B 只收不透明载体，不定义四档，不读 `workload_class`／`resource_pressure`（J4）。
投影取值域未定，见 §10 第 3 条。

## 3.7 《工程》§7.1–§7.5（`docs/02-工程.md:420-473`）

**§7.1 建设内容**（`:420-431`）：八行组件的最后一行是「三层调度 | §301 §302 §303 §304 §77」（`:431`）。
本块只拥有这一行里的 Intent Scheduler 与具名资源端口；Graph Scheduler 在 `continuum-graph`；
Resource Scheduler 不在 P7 内。

**§7.2 组件边界**（`:433-443`）：`:435` 逐字「**三层调度必须相互分离**（§301）。Intent Scheduler 包含模型判断
（§302 的输入含 user focus、background suitability），Graph Scheduler 是确定性逻辑（§303），
Resource Scheduler 落在第 4 层。」
本设计的落法：
- Intent 的模型判断经 `IntentSelectionPolicy`（§4.4）；P7B 不直接调 `ModelProvider`（J14）。
- Graph 是确定性逻辑：`runnable_nodes` 零逻辑转调（§4.6）。
- Resource 落在第 4 层：`ResourceDirectivePort` 的实现由装配方提供（§4.7）。

**§7.3 层内依赖**（`:445-458`）：`:453` 的 `Background Supervisor ← Resource Arbiter + Intent Scheduler`
与 `:454` 的 `Session Orchestrator ← Intent 存储 + Background Supervisor` 把 P7B 放在 P7D 的上游。
本块不依赖 P7C，也不依赖 P7D；`Intent 存储` 不在本块（候选集是 `&[Intent]`）。

**§7.4 完成判据**（`:460-467`）：四条判据分别落在 P7A（判据 1、2、4）与 P7C（判据 3）；
**P7B 没有规范判据**（切分 §七第 12 条）。本设计自报 J10（三层分离的结构照片）作为 P7B 的完成判据，
不修改 §7.4，也不把 J10 冒充成 §7.4 的第五条。收件人：协调者 ＋ 《工程》维护者（§10 第 5 条）。

**§7.5 相关缺口**（`:469-473`）：A15／OPEN-010 影响 Progress Gate，OPEN-015 影响 Milestone 生成；
两处都在 P7A，**P7B 不关闭、不引用、不假装闭合**。

## 3.8 §222／§223 Intent 数据模型与状态机（`docs/spec/05-normative.md:130-205`）

**§222 给了什么**：`Intent` 的十五个字段（`:132-156`），其中 `priority` 是不透明值；`kind` 至少支持六值
（`:159-168`）。代码里 `Intent` 有十四个存储字段（`child_intents[]` 派生），
`crates/continuum-semantics/src/intent.rs:434`；`Priority` 是不透明 `Opaque`（`:300`）。

**§223 给了什么**：十三个状态（`:174-205`），进入运行的五步链
`PROPOSED → SPECIFYING → PLANNING → REVIEW → READY → RUNNING`；运行后六态；Product Intent 的 `EVOLVING`。
`IntentStatus` 在 `intent.rs:61`，迁移判定 `transition` 是 `pub(crate)`（`:217`）。

**本设计怎么落**：P7B **只读** `Intent.id` 与 `Intent.status`；`priority` 只经显式 `PrioritySignal` 携带
（**不读** `Intent.priority` 字段）。不改状态机、不调用 `transition`、不写 `intent` 表。候选资格认六个
可推进状态（`Ready`／`Running`／`Waiting`／`Suspended`／`Verifying`／`Evolving`，§2.5 第 3 条）。
`Advance` 之后：`Ready`／`Waiting`／`Suspended` 由 P7D 走既有 `→ Running`（`intent.rs:252`／`:260`／`:264`）；
`Running`／`Verifying` 已在运行，P7D 继续会话、**不调用** `transition`。`Evolving` 不由此自动等同
「已在运行」：仅当 P7D 确有对应活跃会话时才继续，否则不得进入候选集、也不得调用 `transition`。`Running` 有
`Running → Running` 自环（`:259`），但继续不需要调用；`Verifying`／`Evolving` 没有自环，继续也不需要迁移；
`Verifying → Running`／`Evolving → Running` 是返工／回到运行的另一条语义（`:266-269`）。
P7B 不调用 `transition`（它是 `pub(crate)`），也不做任何迁移。

---

# 4. 类型与接口

## 4.1 `IntentDecision`、`DecisionBasis` 与 `IntentDecisionError`

```rust
/// §302 的「现在推进哪个 Intent」。
///
/// **两臂封闭**：`Advance` 带一个候选的 `IntentId`；`NoIntent` **不带任何 `IntentId`**。
/// `NoIntent` 不是哨兵 `IntentId`：空串在语义层是合法值
/// （`crates/continuum-semantics/src/ids.rs:49-56`），若用它当「没有 Intent」，
/// 调用方会把空串当成一枚真 Intent。本设计不提供这条路径。
///
/// `decide` 返回 `Result<IntentDecision, IntentDecisionError>`；策略违约与策略失败走错误臂，
/// **不**折叠成 `NoIntent`。
///
/// `Advance` 只表示「这一枚现在值得推进」；它不承诺「启动一个新的 READY」还是「继续一个活跃会话」。
/// P7D 按候选的 `IntentStatus` 分派（§5.1）。
#[derive(Debug, Clone)]
pub enum IntentDecision {
    Advance { intent: IntentId, basis: DecisionBasis },
    NoIntent { basis: DecisionBasis },
}

/// 决策的不透明依据。P7B 不解释内容；P7D 可以展示，但不得按内容分支。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionBasis(String);

impl DecisionBasis {
    pub fn new(detail: impl Into<String>) -> Self { Self(detail.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
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

/// `decide` 的失败面。`IntentDecision` 保持两臂；策略失败与越界不折叠成 `NoIntent`。
#[derive(Debug, Clone)]
pub enum IntentDecisionError {
    /// 策略自身失败（模型不可用／无法判断）：`choose` 返回的 `Err`。
    PolicyFailed { basis: DecisionBasis },
    /// 策略返回的 `Advance` 不在候选集，或候选状态不可推进。
    InvalidAdvance { intent: IntentId },
}

impl std::fmt::Display for IntentDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyFailed { basis } => write!(f, "policy failed: {}", basis.as_str()),
            Self::InvalidAdvance { intent } => {
                write!(f, "policy chose ineligible intent {}", intent.as_str())
            }
        }
    }
}

impl std::error::Error for IntentDecisionError {}
```

**为什么不派生 `PartialEq`**：`IntentId` 只派生 `Debug, Clone`（`ids.rs:41`），没有 `PartialEq`／`Eq`；
本块不擅自给既有类型加比较能力。测试用 `advanced().map(IntentId::as_str)` 比较 id，用
`basis().as_str()` 比较依据。

**`NoIntent` 的负例**（J6–J8）：空候选、没有可推进候选、策略明确选择不推进，三种都落 `NoIntent`；
策略失败与策略返回越界 `Advance` **不再**落 `NoIntent`，走 `IntentDecisionError` 的显式错误臂。
`decide` 保证返回的 `NoIntent` 的 `basis` 非空；但 P7D **不得**从 `basis` 反推一个 `IntentId`。

## 4.2 `SchedulerInputs` 与 §302 的五个输入

```rust
/// §302 的五个输入 + 候选集。
///
/// 字段名逐字照 §302：`priority` / `deadline` / `blocking` / `user_focus` /
/// `background_suitability`；`candidates` 是切分 §2.3.2 的候选集，**不是第六个 §302 输入**。
#[derive(Debug)]
pub struct SchedulerInputs<'a> {
    /// 候选集：调用方给的 `&[Intent]`。P7B 不查 `intent` 表、不造第二套 Intent 查询。
    pub candidates: &'a [Intent],

    /// §302 的 `priority`：每枚候选一条，显式给。**不透明**；P7B 不比较、不排序。
    pub priority: &'a [PrioritySignal],

    /// §302 的 `deadline`：每枚候选一条，显式给。毫秒，与 `Intent.created_at` 同口径。
    pub deadline: &'a [Deadline],

    /// §302 的 `blocking`：每枚候选一条，显式给。`blocks` 是它直接阻塞的 Intent。
    pub blocking: &'a [Blocking],

    /// §302 的 `user focus`：整轮的聚焦点。
    pub user_focus: UserFocus,

    /// §302 的 `background suitability`：装配方唯一投影的不透明载体，**整轮单值**（§2.5 第 14 条）。
    pub background_suitability: BackgroundSuitability,
}

/// 一枚候选的显式 `priority`。**不透明**：本块没有比较 API。
#[derive(Debug, Clone)]
pub struct PrioritySignal {
    pub intent: IntentId,
    pub priority: Priority,
}

/// 一枚候选的显式 `deadline`。毫秒；`Intent.created_at` / `updated_at` 也是 i64 毫秒
/// （`crates/continuum-semantics/src/intent.rs:448-454`），本块不取时钟。
#[derive(Debug, Clone)]
pub struct Deadline {
    pub intent: IntentId,
    pub at_millis: i64,
}

/// 一枚候选的显式 `blocking`：它直接阻塞的 Intent 集合。不建「阻塞度」数值——
/// 数值由策略从集合算，P7B 不定义量纲。
#[derive(Debug, Clone)]
pub struct Blocking {
    pub intent: IntentId,
    pub blocks: Vec<IntentId>,
}

/// §302 的 `user focus`。`Area` 是自由文本，P7B 不解释；基线只认 `Intent`。
#[derive(Debug, Clone)]
pub enum UserFocus {
    None,
    Intent(IntentId),
    Area(String),
}
```

**形状的四条约束**：

1. **`priority` 不解析**：`PrioritySignal.priority` 是 `continuum_semantics::Priority`，它没有
   `Ord`／`PartialOrd`（`intent.rs:288-290`），也没有比较方法；本块不构造第二个 priority 类型。
   装配方把 `Intent.priority` **原样**搬进 `PrioritySignal`，这是显式输入的唯一来源；
   P7B 不读 `Intent.priority` 字段（J3）。
2. **`deadline` 是毫秒 `i64`**：与 `Intent.created_at`／`updated_at` 同口径；本块不取时钟，
   由调用方给。同一 `IntentId` 多条 deadline 时，基线与 `directive_for` 都取**最早**（`min`）；
   这是策略行为，不是输入不变量。
3. **`blocking` 是出边集合**：`blocks` 是它直接阻塞的 Intent；基线按**按 `as_str()` 去重后的条数**比较。
   同一 `IntentId` 多条 `Blocking` 时，基线合并所有 `blocks` 后再按 `as_str()` 去重；`IntentId` 没有
   `PartialEq`／`Eq`／`Hash`（`ids.rs:41`），实现不得用 `==` 或 `HashSet<IntentId>`。
   不建「阻塞度」量纲（§78 的 `blocking degree` 是解释面，不是本块的类型）。
4. **`user_focus` 三臂**：`None` 表示没有聚焦；`Intent(id)` 表示聚焦某枚候选；`Area(String)` 是自由文本，
   基线不解释，留给模型驱动策略。`Area` 不是第二个 `IntentId`，也不是 `TaskSource`。

**候选集不变量**：`candidates` 是调用方按需要给的切片；P7B 在 `decide` 里只按**六个可推进状态**过滤
（§2.5 第 3 条），**不**校验 `priority`／`deadline`／`blocking` 的条目是否都指向候选。指向非候选的条目被基线忽略，
不改变决策（J9 的一条反例）。**P7B 也不校验候选 id 的唯一性**：同一 `IntentId` 出现两次是调用方的
契约违反；`decide` 只要求「存在一枚可推进候选」，不定义重复时选哪一枚（两枚 id 相同，选中的 id 相同）；
若两枚重复候选的 `status` 不同，行为不保证。多信号合并规则见 §2.5 第 15 条。

## 4.3 `BackgroundSuitability`

```rust
/// §302 的 `background suitability` 的唯一投影载体。
///
/// **取值域不在本块**：装配方从 P7C 的 `workload_class` ＋ `resource_pressure`
/// 做一次投影，把结果装进这个 String。P7B **不**定义 `Suitable` / `Unsuitable`，
/// **不**读 `workload_class` / `resource_pressure`，**不**依赖 `continuum-arbiter`。
///
/// **粒度是整轮单值**：`SchedulerInputs.background_suitability` 是一条，不是每候选一条。
/// 代价：同一轮里「A 适合后台、B 不适合」无法表达；若 P7C／规范要求每任务适合度，
/// 需另加显式输入，不在本轮（§2.5 第 14 条）。
///
/// 代价照实写：P7B 无法验证投影是否正确；装配方若把四档原文（`INTERACTIVE_GAME` 等）
/// 塞进载体，P7B 也拦不住。这是装配方的契约（§10 第 3 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundSuitability(String);

impl BackgroundSuitability {
    /// 装一次投影结果。**不改写、不裁剪**（与 `Priority` 的 `from_str` 同口径）。
    pub fn from_projection(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 取里子串。P7B 不解释它；基线策略不读它。
    pub fn projection(&self) -> &str {
        &self.0
    }
}
```

**为什么是 `String` 而不是三值枚举**：切分 §2.4 要求「不透明」，且投影的取值域未定。
三值枚举（`Suitable`／`Unsuitable`／`Unknown`）会把「投影值」变成 P7B 的第二套资源词汇，
并让 P7B 开始解释资源结论。本设计选载体；若协调者要求 P7B 也解释三值，那是另一条设计决定，
接口不变（§10 第 3 条）。

## 4.4 `IntentSelectionPolicy` 与 `BaselineIntentPolicy`

```rust
/// §302 五输入的组合面。**评分算法、权重、模型提示不冻**（切分 §2.3.2）。
///
/// 这是排序策略的端口，**不是通用模型端口**：参数是 `SchedulerInputs`，
/// 返回 `Result<IntentDecision, DecisionBasis>`；P7B 不在这里调 `ModelProvider`。
///
/// `Ok(NoIntent)` = 策略明确选择本轮不推进；`Err(basis)` = 策略自身失败
/// （模型不可用／无法判断），**不是**「没有工作」。`decide` 把后者包成
/// `IntentDecisionError::PolicyFailed`，不折叠成 `NoIntent`。
/// **失败只能走 `Err`**：策略实现不得用 panic 表达可恢复失败；P7B 不捕获 panic。
pub trait IntentSelectionPolicy {
    fn choose(
        &self,
        inputs: &SchedulerInputs<'_>,
    ) -> Result<IntentDecision, DecisionBasis>;
}

/// 候选状态是否可推进：`Ready`（启动）与五个活跃状态（继续／恢复）。
/// `Running`／`Verifying` 已在运行，继续它们不需要状态迁移；`Evolving` 仅在 P7D
/// 确有对应活跃会话时才继续，且同样不需要迁移。`is_advanceable` 只判状态合法性，
/// 不检查会话是否存在。
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

/// 确定性基线。**它不是规范排名**；它是 P7D 可以直接用的默认策略。
#[derive(Debug, Default, Clone, Copy)]
pub struct BaselineIntentPolicy;

impl IntentSelectionPolicy for BaselineIntentPolicy {
    fn choose(
        &self,
        inputs: &SchedulerInputs<'_>,
    ) -> Result<IntentDecision, DecisionBasis> { /* 见下 */ }
}
```

**线程模型不冻**：trait 本身不声明 `Send + Sync`；P7D 若跨线程持有策略，由实现方加 bound 或包装。
P7B 不替调用方选线程模型。

**基线的次序（本设计自定，§2.5 第 6 条）**：

1. `eligible` = `candidates` 中 `is_advanceable(status)` 为真的项；空 ⇒
   `Ok(NoIntent { basis: "no_advanceable_candidate" })`。
2. `UserFocus::Intent(id)` 按 `as_str()` 命中 `eligible` ⇒ `Ok(Advance(id))`，依据 `"user_focus"`。
3. 否则，在**有 `Deadline`** 的 `eligible` 中，每枚候选先取自己最早的一条，再选 `at_millis` 最小者；
   同值按输入序。依据 `"earliest_deadline"`。
4. 否则，在**有 `Blocking`** 的 `eligible` 中，每枚候选合并其所有 `Blocking` 条目的 `blocks`，
   再按 `as_str()` 去重计条数；选条数最大者，同值按输入序。依据 `"blocking_degree"`。
5. 否则，选 `eligible` 的第一项（输入序）。依据 `"candidate_order"`。
6. `priority` 与 `background_suitability` **不参与基线**：`Priority` 不透明（§3.6），
   `BackgroundSuitability` 的语义未定（§4.3）。一个模型驱动的实现可以读它们；P7B 只提供 trait。

**基线不按状态加权**：`Ready` 与五个活跃状态在同一个 `eligible` 里按同一顺序比较。P7D 若想
「先启动新工作」或「先继续活跃会话」，应在候选集构造时表达（只传 `Ready`，或只传活跃状态，
或按需要的顺序混合）；P7B 不替它判断「启动 vs 继续」。

**默认基线不兑现 §78 的「显式 priority 最高语义权重」，且不能用候选序补救**：`Priority` 不透明，
基线不读它；基线的第 3／4 步（deadline／blocking）会先于输入序决定结果，把候选按 priority 排序
并不能让 priority 获得最高权重。要兑现 §78，必须由装配方注入一个解释 `Priority` 的
`IntentSelectionPolicy`，或等 P4 给 `Priority` 一个可比较的取值域。这是缺口，挂 P4／规范维护者
（§10 第 2 条），默认基线不宣称尊重 §78。

**基线不做的**：不读 `intent.priority`；不比较 `Priority`；不把 `BackgroundSuitability` 当布尔；
不按 `Intent.kind` 或 `product_id` 加分；不调用模型；不写日志。

## 4.5 `IntentScheduler::decide`

```rust
/// Intent Scheduler 的唯一决策入口（§302）。
#[derive(Debug, Default, Clone, Copy)]
pub struct IntentScheduler;

impl IntentScheduler {
    /// 调策略，并校验策略的输出：
    /// - 策略返回 `Err(basis)` ⇒ `Err(IntentDecisionError::PolicyFailed { basis })`；
    /// - `Advance` 的 id 必须是 `candidates` 里的一枚且状态可推进，否则
    ///   `Err(IntentDecisionError::InvalidAdvance { intent })`；
    /// - `NoIntent` 原样返回；`basis` 为空时补 `"no_intent"`（`decide` 的不变量）。
    ///
    /// **不迁移状态**：`Advance` 只给 id；P7D 按候选状态走 §223 的既有迁移或不迁移（§5.1）。
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

**为什么校验**：`IntentDecision` 的变体是公开的，外部策略可以构造任意 `Advance`；`decide` 是唯一决策面，
必须保证它只返回候选集里可推进的 Intent（J8）。**失败面**：策略失败与越界都走 `Err`，不再静默变成
`NoIntent`；P7D 必须 fail-closed，不能把 `Err` 当作 `NoIntent`（§5.1）。`decide` 只规范化
`NoIntent.basis` 的非空，不改变策略的 `Advance` 依据。

## 4.6 Graph 的消费：`graph::runnable_nodes`

```rust
use continuum_graph::{AdfirGraph, NodeId, SchedulerConfig};

/// §303 的 Graph Scheduler 消费入口。**零逻辑转调**：
/// READY／并发判定全部在 `continuum_graph::select_runnable`
/// （`crates/continuum-graph/src/scheduler.rs:28`）。
///
/// 本函数**不**读 `NodeState`、**不**遍历边、**不**判前驱、**不**截断、**不**排序、
/// **不**做状态迁移；`SchedulerConfig.max_parallel` 原样传入，本块不解释它
/// （默认 1 见 `scheduler.rs:14-17`）。
pub fn runnable_nodes(graph: &AdfirGraph, config: &SchedulerConfig) -> Vec<NodeId> {
    continuum_graph::select_runnable(graph, config)
}
```

**这个入口不是 P7B 的决策面**。它存在的唯一理由是切分 §三第 4 条要求 P7B 消费 `select_runnable`，
并让 §301 的三层分离有一个可指认的入口。**P7B 不作出 READY／并发判定**；判定在 `continuum-graph`。
若协调者认为 P7B 不应暴露这个入口，删除它不影响 Intent Scheduler 的任何判据，
`select_runnable` 的调用点回到装配方（§10 第 6 条）。

## 4.7 `ResourceDirectivePort`、`ExecutionRequest`、`ResourceDirective`

```rust
/// §304 的具名资源端口。**P7B 定义 trait，装配方实现**。
///
/// 实现方的契约（不在本块可执行）：
/// - 消费 P7C 的结论：`resource_pressure`、`workload_class` 与 §300 的四档优先级；
///   **这些结论由实现在构造端口值时持有（快照或句柄），不是 `direct` 的参数**；
/// - 内部转调 `rank`（`router.rs:523`）／`place`（`placement.rs:362`）／既有工具路径；
/// - 返回的 `ResourceDirective` 至少能承载 §304 的四问。
/// P7B 不引入 P7C 的任何类型；若 P7C 要求按请求传入结论，才增加不透明 `ResourceContext`
/// 参数，本轮不加（§2.5 第 10 条、§10 第 4 条）。
pub trait ResourceDirectivePort {
    fn direct(
        &self,
        request: &ExecutionRequest<'_>,
    ) -> Result<ResourceDirective, ResourceDirectiveError>;
}

/// 一次执行请求：Intent ＋ 可选 Graph 节点 ＋ 显式信号。
#[derive(Debug)]
pub struct ExecutionRequest<'a> {
    pub intent: &'a Intent,
    pub node: Option<&'a NodeId>,
    pub signals: ExecutionSignals<'a>,
}

/// 进入资源请求的显式信号。`user focus` / `blocking` 只影响 Intent 选择，
/// 不进入资源请求；装配方需要时它已持有 `SchedulerInputs`。
#[derive(Debug)]
pub struct ExecutionSignals<'a> {
    pub priority: Option<&'a Priority>,
    pub deadline: Option<&'a Deadline>,
    /// 整轮单值；不是每候选一条（§2.5 第 14 条）。
    pub background_suitability: &'a BackgroundSuitability,
}

/// §304 四问的不透明载体。**槽存在是结构保证；槽内容由装配方定义，P7B 不解析**。
/// `None` 表示「该问未答／不可判」，**不是**默认值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDirective {
    node_placement: Option<String>,
    model: Option<String>,
    tool: Option<String>,
    parallelism: Option<String>,
}

impl ResourceDirective {
    pub fn new(
        node_placement: Option<String>,
        model: Option<String>,
        tool: Option<String>,
        parallelism: Option<String>,
    ) -> Self {
        Self { node_placement, model, tool, parallelism }
    }

    pub fn node_placement(&self) -> Option<&str> { self.node_placement.as_deref() }
    pub fn model(&self) -> Option<&str> { self.model.as_deref() }
    pub fn tool(&self) -> Option<&str> { self.tool.as_deref() }
    pub fn parallelism(&self) -> Option<&str> { self.parallelism.as_deref() }
}

/// 端口错误。**不透明**：P7B 不解释错误类别；装配方把 `rank`／`place` 的错误映射进来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDirectiveError(String);

impl ResourceDirectiveError {
    pub fn new(message: impl Into<String>) -> Self { Self(message.into()) }
    pub fn message(&self) -> &str { &self.0 }
}

impl std::fmt::Display for ResourceDirectiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ResourceDirectiveError {}
```

**P7C 结论由实现方持有，不是 trait 参数**：`direct` 只收 `ExecutionRequest`；实现方在构造端口值时
持有 `resource_pressure`／`workload_class`／§300 四档（快照或句柄），调用时读自己的字段。
P7B 不引入 P7C 类型，因此切分 §四第 6 条的「P7B 与 P7C 无未冻接口」成立。若 P7C 的设计要求
每次请求传入结论，才增加一个不透明 `ResourceContext` 参数——那不得把 P7C 类型带进 P7B（§10 第 4 条）。

**四槽与 §304 的对应**：`node_placement` ↔「Node 在哪里执行」；`model` ↔「用哪个模型」；
`tool` ↔「哪个工具」；`parallelism` ↔「多少并行」。四槽是**承载**，不是**计算**：
P7B 不填槽、不校验槽内容、不定义槽格式。装配方实现端口时决定每个槽是 `Some` 还是 `None`；
`None` 不是「用默认」，是「未答」。

**`parallelism` 槽与 `SchedulerConfig.max_parallel` 不是同一个东西**：前者是资源调度对「多少并行」的
答案（不透明字符串，由装配方填），后者是 Graph Scheduler 的现有旋钮（`usize`，默认 1，
`scheduler.rs:14-17`）；P7B 不把两者合并，也不读 `max_parallel`（J11）。`ResourceDirective` 的四槽
不是 §300 的四档，也不是 Graph 的并发上限；它只是 §304 四问的承载位。

**为什么是 `Option<String>` 而不是四个具体类型**：`NodeId`／`ModelId`／工具引用／并行度分别属于
`continuum-graph`／`continuum-model-registry`／第 4 层／Graph Scheduler，P7B 依赖它们就会把
第 4 层的类型拉进本块，并让端口变成第二个 Resource Scheduler。字符串槽让 P7B 只拥有「四问的形状」，
不拥有「四问的答案」。代价是消费方要解析槽内容（§2.5 第 11 条）。

**线程模型不冻**：`ResourceDirectivePort` 的 trait 本身不声明 `Send + Sync`；P7D／装配方若跨线程
共享端口，由实现方加 bound 或包装。P7B 不替装配方选线程模型（§2.5 第 10 条）。

**P7B 结束时不宣称 §304 已闭合**：`ResourceDirective` 只保证四槽存在；`rank`／`place`／工具选择／
并行度的实现仍在第 4 层缺口里（切分 §6.2 第 12 条）。

## 4.8 `IntentScheduler::directive_for`

```rust
use crate::decision::is_advanceable;

/// `directive_for` 的错误面：契约违反与端口失败分开，不折叠成 `Ok(None)`。
#[derive(Debug, Clone)]
pub enum DirectiveForError {
    /// `Advance` 的 id 不在候选集，或候选状态不可推进。
    InvalidDecision { intent: IntentId },
    /// 端口实现失败，原样透传。
    Port(ResourceDirectiveError),
}

impl std::fmt::Display for DirectiveForError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDecision { intent } => {
                write!(f, "advance id {} is not an advanceable candidate", intent.as_str())
            }
            Self::Port(error) => write!(f, "resource directive port failed: {error}"),
        }
    }
}

impl std::error::Error for DirectiveForError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Port(error) => Some(error),
            Self::InvalidDecision { .. } => None,
        }
    }
}

impl IntentScheduler {
    /// 把一次已作出的 `IntentDecision` 交给资源端口，并原样返回不透明指令值。
    ///
    /// - `NoIntent` ⇒ `Ok(None)`，**不调用端口**；
    /// - `Advance` 的 id 不在候选集或状态不可推进 ⇒ `Err(InvalidDecision)`（契约违反）；
    /// - 端口错误 ⇒ `Err(Port(e))`，原样透传，不包装。
    ///
    /// 同一 Intent 多条 `priority` 取输入序第一条；多条 `deadline` 取最早；
    /// `background_suitability` 是整轮单值（§2.5 第 14、15 条）。
    pub fn directive_for(
        inputs: &SchedulerInputs<'_>,
        decision: &IntentDecision,
        node: Option<&NodeId>,
        port: &dyn ResourceDirectivePort,
    ) -> Result<Option<ResourceDirective>, DirectiveForError> {
        let IntentDecision::Advance { intent, .. } = decision else {
            return Ok(None);
        };

        let candidate = inputs
            .candidates
            .iter()
            .find(|candidate| candidate.id.as_str() == intent.as_str())
            .filter(|candidate| is_advanceable(candidate.status))
            .ok_or_else(|| DirectiveForError::InvalidDecision {
                intent: intent.clone(),
            })?;

        let priority = inputs
            .priority
            .iter()
            .find(|signal| signal.intent.as_str() == intent.as_str())
            .map(|signal| &signal.priority);
        let deadline = inputs
            .deadline
            .iter()
            .filter(|deadline| deadline.intent.as_str() == intent.as_str())
            .min_by_key(|deadline| deadline.at_millis);

        let request = ExecutionRequest {
            intent: candidate,
            node,
            signals: ExecutionSignals {
                priority,
                deadline,
                background_suitability: &inputs.background_suitability,
            },
        };

        port.direct(&request).map(Some).map_err(DirectiveForError::Port)
    }
}
```

**`directive_for` 是转交入口，不是第四个决策面**：它只做「已作出的 `IntentDecision` → 端口调用」，
不产出 Intent／Graph 决策、不填槽、不做状态迁移（J10-c）。切分 §三第 5 条说「P7B 只定义 trait、
持不透明指令值并把它用于调度结果」；这个函数就是「持」的落点。P7B 不读指令内容，只把它交给调用方。
P7D 可以直接用 `decide`（不取指令），也可以在需要资源指令时用 `directive_for`。若协调者要求把转交
也移出 P7B，删除它不影响 Intent Scheduler 的任何判据，P7D／装配方可在自己的接线里调用 `direct`。

## 4.9 与 §302／§77 的对齐

| §302 输入 | 本块类型 | 来源 | P7B 解释它吗？ |
|---|---|---|---|
| `priority` | `PrioritySignal { intent, priority: Priority }` | 装配方把 `Intent.priority` 原样搬进显式输入 | **不**；`Priority` 没有 `Ord`／`PartialOrd`；基线不读 |
| `deadline` | `Deadline { intent, at_millis: i64 }` | 显式输入 | 基线比较 `at_millis`；同一 Intent 多条取最早；P7B 不取时钟 |
| `blocking` | `Blocking { intent, blocks: Vec<IntentId> }` | 显式输入 | 基线按 `as_str()` 去重条数比较；不定义量纲 |
| `user focus` | `UserFocus::{None, Intent, Area}` | 显式输入 | 基线只匹配 `Intent`；`Area` 留给模型策略 |
| `background suitability` | `BackgroundSuitability(String)` | 装配方从 P7C 的 `workload_class`＋`resource_pressure` 唯一投影；**整轮单值** | **不**；基线不读 |
| （候选集） | `&[Intent]` | 调用方给，不查表 | 只按六个可推进状态过滤（§2.5 第 3 条） |

§77 的三层对齐：`decide` ↔「哪个任务现在值得推进」；`runnable_nodes` ↔「哪些 ADFIR 节点可以并发执行」；
`ResourceDirectivePort` ↔「在哪个模型/设备/资源上执行」。三个决策面的返回类型互不相同（J10）；
`directive_for` 是转交入口，不参与这个比较。

---

# 5. 消费契约与三层分离

## 5.1 P7D 消费 `IntentDecision`

- **`Advance`**：`intent` 是候选集里的一枚**可推进** Intent；`Advance` 只给 id，**不携带**状态迁移动作。
  P7D 按候选的 `IntentStatus` 分派：
  - `Ready` → `Ready → Running`（启动，`intent.rs:252`）；
  - `Waiting` → `Waiting → Running`（恢复，`:260`）；
  - `Suspended` → `Suspended → Running`（恢复，`:264`）；
  - `Running`／`Verifying` → 已在运行，**不调用** `transition`，继续当前会话；
  - `Evolving` → 仅当 P7D 确有对应活跃会话时才继续（**不调用** `transition`）；否则不得把该 Intent
    放进候选集，也不得以 `Advance` 驱动其迁移。
  这些迁移由 P7D 经 `apply_intent_transition`（`crates/continuum-semantics/src/lib.rs:276`）或既有运行时路径执行；
  P7B 不迁移状态；P7D 不得对 `Running`／`Verifying`／`Evolving` 重复走 `Ready → Running`。
  `PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW` 不是 P7B 候选状态；P7D 不得借 `Advance` 或本契约
  驱动它们的迁移，前向链由 P4／审查流程持有。
  `COMPLETED` 不由 P7B 直接推进；Product 由 P7A 迁到 `EVOLVING`；`FAILED`／`CANCELLED` 才是终态，
  不进入候选。
- **`NoIntent`**：表示**本轮没有新的可推进项**（空候选、无可推进候选，或策略明确选择不推进）。
  它**不表示没有会话**：P7D **必须保留**既有会话与后台任务，不得据此清理、关闭或丢弃它们。
  **不得**用哨兵 `IntentId`，不得把空串当 Intent，不得把 `NoIntent` 映射成某个默认 Intent。
- **`Err(IntentDecisionError)`**：策略失败（`PolicyFailed`）或策略越界（`InvalidAdvance`）。
  P7D 必须 fail-closed：保留现状、报告错误，**不得**把 `Err` 当作 `NoIntent`。
- **`basis`**：P7D 可以展示 `basis().as_str()`，但**不得**按内容分支；若 P7D 需要结构化原因，
  那是 §10 第 10 条的设计问题。
- **候选集**：P7D 持有并构造它传给 `decide` 的 `&[Intent]`；要「启动新工作」就只传 `Ready`，
  要「继续活跃会话」就传五个活跃状态，混合时由策略排序（P7B 不按状态加权）。
  `Advance` 之后按 id 查回 Intent 元数据（包括 `status`，用于上面的分派）。
  P7B 不复制 `Intent` 的字段，也不返回 `Intent` 本体。
- **不另写排名**：P7D 的 Auto 模式必须调 `IntentScheduler::decide`；它可以使用本块的
  `BaselineIntentPolicy`，或装配方注入的 `IntentSelectionPolicy`。**P7D 自己实现一个新的
  `IntentSelectionPolicy` 就是第二个「最值得继续」排名**，违反切分 §五 P7D 的完成判据 2。

**对切分 §2.3.2 的订正**：该节原文的消费措辞与 §223 的恢复路径不相容：P7D 可能在已有
`RUNNING`／`WAITING`／`SUSPENDED` 会话时调用 `decide`，得到 `NoIntent` 只表示「本轮没有新的可推进项」，
不能据此丢弃既有会话。本设计按 C1／C2 的收口订正为「保留既有会话」，建议协调者同步切分 §2.3.2
（§11 第 7 条）。

## 5.2 装配方消费 `ResourceDirectivePort`

装配方的接线（**不在 P7B 内实现**）：

1. 从 P7C 取 `resource_pressure` 与 `workload_class`（§13 `:717-722`）；
2. 做**一次** `BackgroundSuitability` 投影（§4.3），填进 `SchedulerInputs`；
3. 调 `IntentScheduler::decide`；`Err(IntentDecisionError)` ⇒ fail-closed，保留既有会话并报告错误；
4. 若 `Ok(Advance)`，调 `IntentScheduler::directive_for`，传一个实现 `ResourceDirectivePort` 的装配方值；
   `Ok(None)` 不调端口；`Err(DirectiveForError)` 按契约违反／端口失败分别处理；
5. 该实现**持有** P7C 的结论与 §300 的四档（`:1911-1919`），内部转调 `rank`／`place`／工具路径，
   返回 `ResourceDirective`。P7C 结论在构造端口值时取得，**不是 `direct` 的参数**。

**P7B 不依赖 P7C**；P7C 不依赖 P7B。唯一的汇合点是装配方的端口实现（切分 §三第 5 条）。
在 P7C 落地前，P7B 的测试用假端口；端口实现（以及它如何持有 P7C 结论）在 P7C／装配方的设计里（§10 第 4 条）。

## 5.3 `NoIntent` 的语义

`NoIntent` 是**调度结果**，不是 Intent 状态，也不是节点状态。它的三种来源（J6–J8）：

| 来源 | 触发 | 照片 |
|---|---|---|
| 空候选 | `candidates.is_empty()` | J6：`decide` 返回 `Ok(NoIntent)`，`advanced() == None` |
| 无可推进候选 | 候选状态都不在六个可推进状态（例如全是 `PROPOSED`／`PLANNING`／`COMPLETED`） | J7：逐状态一组，均 `Ok(NoIntent)` |
| 策略明确不推进 | `choose` 返回 `Ok(NoIntent)` | J8-e：`Ok(NoIntent)`，`basis` 非空（`decide` 规范化） |

**不属于 `NoIntent` 的失败面**：策略返回 `Err(basis)` ⇒ `IntentDecisionError::PolicyFailed`；
策略返回越界 `Advance` ⇒ `IntentDecisionError::InvalidAdvance`。两者都是 `Err`，P7D 不得当 `NoIntent`。
**两个入口的契约违反同面**：`decide` 的 `InvalidAdvance` 与 `directive_for` 的 `InvalidDecision`
都表示「策略／decision 指向了不可推进的 id」，都返回 `Err`；不一个折叠成 `NoIntent`、一个返回 `Err`。

**`NoIntent` 不表示「没有会话」**：P7D 收到它时必须保留既有会话与后台任务；它只表示「本轮没有新的
可推进项」。这是对切分 §2.3.2 原措辞的订正（§5.1、§11 第 7 条）。

**负例的硬约束**：`NoIntent` 没有 `IntentId` 字段；`grep -rn "IntentId::from_str(\"\")"` 在本块 `src/` 零命中；
P7D 不得用 `IntentId::from_str("")` 或任何空串 id 表示「没有 Intent」（J1）。

## 5.4 三层分离的结构照片

| 层 | 入口 | 返回类型 | 谁作出判定 | P7B 的角色 |
|---|---|---|---|---|
| Intent | `IntentScheduler::decide` | `Result<IntentDecision, IntentDecisionError>` | P7B 的策略（`IntentSelectionPolicy`） | 拥有 |
| Graph | `graph::runnable_nodes` | `Vec<NodeId>` | `continuum_graph::select_runnable` | 零逻辑转调；不判 READY／并发 |
| Resource | `ResourceDirectivePort::direct` | `Result<ResourceDirective, ResourceDirectiveError>` | 装配方的实现（持有 P7C 结论） | 定义 trait 与四槽载体；不填槽 |

**转交入口（不是第四个决策面）**：`IntentScheduler::directive_for` 的签名是
`Result<Option<ResourceDirective>, DirectiveForError>`；它只把已作出的 `IntentDecision` 交给
`ResourceDirectivePort`，不产出 Intent／Graph／Resource 决策，不填槽，不做状态迁移。J10-c 单独钉它。

**P7B 自报的完成判据（J10）**：三个决策面各有独立签名，返回类型互不相同；
**不存在一个函数同时返回 `IntentDecision`、`Vec<NodeId>`、`ResourceDirective` 中的两个或三个**。
`IntentScheduler::decide` 的签名不出现 `NodeId`／`ResourceDirective`；`runnable_nodes` 的签名不出现
`IntentDecision`／`ResourceDirective`；`direct` 的返回不出现 `IntentDecision`／`Vec<NodeId>`。
`directive_for` 只返回资源指令／错误，属于转交入口，不参与「三个决策面」的返回类型比较。
这是 §301 的 MUST 的结构照片，也是 P7B 对「§7.4 没有判据」的自报（§3.7、§10 第 5 条）。

**严格说**：`decide` 是 P7B 的**决策**；`runnable_nodes` 与 `direct` 是 P7B 的**消费入口**，
判定分别由 `continuum-graph` 与装配方作出；`directive_for` 是**转交**。J10 检查的是
「P7B 的决策面不产出 Graph／Resource 决策」，不是「P7B 不调用它们」——后者的判据是 J11／J12／J10-c。

---

# 6. 跨块接缝

| 块 | 接缝 | P7B 给出 | P7B 不依赖 | 状态 |
|---|---|---|---|---|
| **P7C** | `ResourceDirectivePort` 的实现**持有** P7C 的 `resource_pressure`／`workload_class` 与 §300 四档（实现方状态，不是 trait 参数）；`BackgroundSuitability` 的投影由装配方从 P7C 的结论做一次 | §4.3 的载体；§4.7 的 trait 与四槽 | `continuum-arbiter` 零依赖 | 设计可全并行；运行时由装配方接线 |
| **P7D** | Session Orchestrator 的 Auto 模式消费 `IntentDecision`；`NoIntent` ⇒ **保留既有会话**，不清理；`Advance` 按候选状态分派；不另写排名 | §4.1 的决策面；§5.1 的消费契约 | 不依赖 `continuum-supervisor` | P7D 对着 §4.1 写 |
| **P7A／`continuum-product`** | Product Intent 只经候选集 `&[Intent]` 进入；P7B 不读 Product 记录、不依赖 `continuum-product` | §4.2 的候选集 | 不依赖 `continuum-product` | 按值交接，无 crate 边 |
| **P1／`continuum-graph`** | 消费 `select_runnable`／`SchedulerConfig`；不接管节点状态迁移 | §4.6 的零逻辑转调 | 不依赖 P1 的执行器 | Graph 已有；P7B 只转调 |
| **P4／`continuum-semantics`** | `Intent`／`IntentId`／`Priority`／`IntentStatus` 只读；候选集 `&[Intent]` | §4.2 的输入形状 | 不写 `intent` 表、不造查询 | 既有类型已冻结；P7B 只读 |
| **装配方 `continuum-runtime`** | 实现 `ResourceDirectivePort`（在构造时持有 P7C 结论）；投影 `BackgroundSuitability`；接线 P7C 与 P7B | §4.7 的 trait；§4.8 的 `directive_for` | 不依赖 `continuum-runtime` | 实现不在 P7B；本轮不登记 runtime 边 |

**P7C 的接缝细节**：P7C 的结论面（`resource_pressure`／`workload_class`／§300 四档）不在本块；
P7B 的 `BackgroundSuitability` 只是载体。**P7C 结论由端口实现持有，不是 trait 参数**：实现方在构造
端口值时取得结论（快照或句柄），`direct` 调用时读自己的状态。若 P7C 的投影需要把三值／等级写进载体，
那是 P7C／装配方的取值域决定（§10 第 3 条）。**P7B 不读 `workload_class`／`resource_pressure`**（J4）。
若 P7C 要求每次请求传入结论，才增加不透明 `ResourceContext` 参数，不把 P7C 类型带进 P7B。

**P7D 的接缝细节**：`NoIntent` **不**触发任何清空动作；P7D 必须保留既有会话与后台任务。
`Advance` 只给 id，P7D 按候选状态分派 §223 的既有迁移或不迁移（§5.1）。P7D 不实现
`IntentSelectionPolicy`；它使用本块的 `BaselineIntentPolicy` 或装配方注入的策略（§5.1）。

---

# 7. 判据与照片

## 7.1 判据表（每条挂规范节）

| # | 判据 | 规范节 | 观测形态 |
|---|---|---|---|
| **J1** | `IntentDecision` 恰两臂 `Advance`／`NoIntent`；`NoIntent` 不带 `IntentId`；`decide` 的 `Result` 另有 `IntentDecisionError` 错误臂；本块 `src/` 无空串哨兵 | §302 `:1939-1955`；切分 §2.3.2 | 源码面（枚举定义＋`grep`）＋运行期（空候选的 `Ok(NoIntent)`） |
| **J2** | `SchedulerInputs` 的字段恰为 `candidates` ＋ §302 五个名字：`priority`／`deadline`／`blocking`／`user_focus`／`background_suitability` | §302 `:1947-1955` | 源码面（穷尽解构，无 `..`）＋编译期 |
| **J3** | **不解析 `priority`**：本块 `src/` 不读 `Intent.priority`；`Priority` 只作为 `PrioritySignal` 的字段携带；无 `Ord`／`PartialOrd`／比较／按 priority 排序 | §302；§78 `:651`；切分 §2.4 | 源码面（`grep`）＋类型层（`Priority` 无比较 API）＋运行期（换 priority 不改变基线决策） |
| **J4** | **不解析 `workload_class`／`resource_pressure`**：本块 `src/` 无四档原文；`BackgroundSuitability` 是私有字段的 `String` 载体；无 `continuum-arbiter` 依赖 | §302；§13 `:708-722`；切分 §2.4 | 源码面（`grep`＋`Cargo.toml`）＋运行期（载体逐字节往返） |
| **J5** | 候选集是 `&[Intent]`；本块无 `continuum-persist` 依赖、无 `load_intent`／`list_children`／`save_intent`／`apply_intent_transition`、无 `intent` 表、无迁移 | §222 `:130-156`；切分 §6.2 第 5 条 | 源码面（签名＋`grep`＋`Cargo.toml`）＋`runtime_migrations()` 无本块行 |
| **J6** | 空候选 ⇒ `decide` 返回 `Ok(NoIntent)`；`advanced() == None`；`basis` 非空 | §302；切分 §2.3.2 | 运行期 |
| **J7** | 七个 P7B 非候选状态（`PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW`／`COMPLETED`／`FAILED`／`CANCELLED`）在基线策略下 ⇒ `Ok(NoIntent)`；六个可推进状态（`READY`／`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`EVOLVING`）各一组正例，假策略返回合法 `Advance` ⇒ `Ok(Advance)`。六组正例只证 `decide` 的 `is_advanceable` 校验面；基线对活跃状态的选择由 J7-b 补 | §223 `:172-205`；§302 | 运行期（逐状态一组） |
| **J8** | 策略返回的 id 不在候选集、或候选状态不可推进 ⇒ `Err(InvalidAdvance)`；策略返回 `Err` ⇒ `Err(PolicyFailed)`；合法 `Advance` 原样返回 `Ok(Advance)`；`Ok(NoIntent)` 返回（空 `basis` 被规范化为 `"no_intent"`） | §302；§223；§2.5 第 7 条 | 运行期（假策略五组） |
| **J9** | 基线确定性：同一输入＋同一策略 ⇒ 同一决策；可推进候选内，用户聚焦 > 最早 deadline > 最大 blocking 度（按 `as_str()` 去重）> 输入序；`priority`／`background_suitability` 不改变基线决策；重复候选与多信号合并按 §2.5 第 15 条 | §302；§78 `:641-651` | 运行期（表驱动＋对照） |
| **J10** | **三层分离**：三个决策面 `decide`／`runnable_nodes`／`direct` 的返回类型互不相同；不存在一个函数同时返回其中两个或三个；`decide` 的签名不出现 `NodeId`／`ResourceDirective`；`directive_for` 是转交入口，只做决策→端口转交（J10-c） | §301 `:1925-1935`；§77 `:616-631`；切分 §五 P7B 完成判据 | 源码面（公开入口签名：`pub fn` ＋ trait 方法＋`grep`） |
| **J11** | Graph 消费是零逻辑：`runnable_nodes` 的函数体恰是 `select_runnable(graph, config)`；本块 `src/` 无 `NodeState`／`edges_to`／`predecessors`／`apply_blocking`／`apply_unblocking`／`transition(`／`max_parallel` | §303 `:1959-1969`；切分 §三第 4 条 | 源码面（函数体＋`grep`） |
| **J12** | Resource 是端口不是实现：`ResourceDirectivePort` 是 trait；本块 `src/` 无 `impl ResourceDirectivePort`；无 `rank`／`place`／`RoutingRequest`／`PlacementRequest`／`ComputeNode`／`ExecutionCandidate`；无 `continuum-model-registry`／`continuum-node` 依赖 | §304 `:1973-1982`；切分 §三第 5 条 | 源码面（`grep`＋`Cargo.toml`） |
| **J13** | 指令值不透明且四槽：`ResourceDirective` 字段私有、四槽、无 `Default`；`directive_for` 在 `NoIntent` 不调端口、在合法 `Advance` 只调一次并原样返回；非法 `Advance` ⇒ `Err(InvalidDecision)` 且不调端口；端口错误 ⇒ `Err(Port(e))` | §304；§2.5 第 11、12、16 条 | 运行期（假端口计数＋四槽断言）＋源码面（字段私有） |
| **J14** | 无模型调用：本块无 `continuum-provider`／`continuum-runtime` 依赖；`src/` 无 `ModelProvider`／`InvokeRequest`；模型驱动策略只在装配方 | 《工程》§7.2 `:435`；切分 §五 P7B 完成判据 | 源码面（`grep`＋`Cargo.toml`） |
| **J15** | 不宣称 §304 闭合：本块无 `ResourceScheduler` 类型、无四问的计算；`ResourceDirective` 只有四槽载体；设计明写「§304 仍不完整」 | §304；切分 §1.1、§6.2 第 12 条 | 源码面（类型名＋`grep`）＋文档面（本设计 §3.4） |
| **J16** | `ALLOWED` 的 `continuum-intent-scheduler` 行恰是 `["continuum-canonical", "continuum-graph", "continuum-semantics"]`（三项，含一条 dev）；在含本 crate 的树上 `cargo test -p continuum-runtime --test dependency_direction` 绿；既有行不因本块增枚 | 切分 §四第 1、5 条 | 运行期（依赖门）＋源码面（`ALLOWED` 行） |

## 7.2 照片清单（每条判据的观测形态：运行期／编译期／源码面）

| # | 照片（具体断言或命令） | 形态 | 拍不到的部分 |
|---|---|---|---|
| **J1-a** | `src/decision.rs` 的 `IntentDecision` 定义恰两臂；`match` 穷尽解构（无 `_`）通过 | 源码面＋编译期 | 外部代码仍可构造 `Advance`；由 J8 兜住 |
| **J1-b** | `grep -rn "IntentId::from_str(\"\")" crates/continuum-intent-scheduler/src` 零命中（**文档注释除外**）；**这条只覆盖空串哨兵这一种**，`" "`／`"none"`／`"__no_intent__"` 等不在此 grep 的射程 | 源码面 | 枚举形状是主证据；grep 只作辅助 |
| **J1-c** | 空候选 ＋ `BaselineIntentPolicy`：`decide(...) ⇒ Ok(NoIntent)`；`advanced() == None`；`basis().as_str()` 非空 | 运行期 | —— |
| **J2** | 在测试里 `let SchedulerInputs { candidates, priority, deadline, blocking, user_focus, background_suitability } = &inputs;`（**无 `..`**）；字段名与 §302 逐字比对 | 源码面＋编译期 | 字段顺序在运行期不可见；靠源码面 |
| **J3-a** | `grep -rn "intent\.priority\|candidate\.priority\|sort_by.*priority\|cmp.*priority\|Ord for Priority\|PartialOrd.*Priority" crates/continuum-intent-scheduler/src` 零命中 | 源码面 | 注释里可能提到 `priority`；语料限定 `src/` 的非注释行 |
| **J3-b** | 两个候选的 `PrioritySignal` 互换、其余输入相同：基线决策的 `advanced()` 不变 | 运行期 | 只证明基线不读；一个模型策略仍可能读 |
| **J3-c** | `Priority` 没有 `Ord`／`PartialOrd`（`crates/continuum-semantics/src/intent.rs:288-290`）；`trybuild` 或编译失败样例：`p1 < p2` 编不过（可选，不在本块强制） | 类型层 | 不给 `Priority` 加比较能力是本块不做的；P4 若加，这条要改 |
| **J4-a** | `grep -rn "workload_class\|resource_pressure\|INTERACTIVE_GAME\|COMPUTE_WORKLOAD\|COMPILER\|VIDEO_RENDER" crates/continuum-intent-scheduler/src` 只命中文档注释，不命中类型／表达式 | 源码面 | 装配方仍可把四档原文塞进载体；见 §4.3 的边界 |
| **J4-b** | `BackgroundSuitability::from_projection("x")` ⇒ `projection() == "x"`（含空白／非 ASCII 的往返） | 运行期 | 投影语义的正确性（P7C／装配方） |
| **J4-c** | `crates/continuum-intent-scheduler/Cargo.toml` 无 `continuum-arbiter`；`ALLOWED` 行无 `continuum-arbiter` | 源码面 | —— |
| **J5-a** | `SchedulerInputs.candidates: &'a [Intent]`；`grep -rn "load_intent\|list_children\|save_intent\|apply_intent_transition\|continuum_persist" crates/continuum-intent-scheduler/src` 零命中（**文档注释除外**） | 源码面 | 调用方仍可从库里读出 Intent 再传进来；本块不拦 |
| **J5-b** | `Cargo.toml` 无 `continuum-persist`；`runtime_migrations()`（`main.rs:114`）无本块行；`expected_migrations()`（`migrations.rs:37`）与 `startup.rs` 计数不变 | 源码面 | —— |
| **J6** | `candidates = &[]`、其余输入任意，基线策略：`decide(...) ⇒ Ok(NoIntent)`，`basis` 非空 | 运行期 | —— |
| **J7** | 七个 P7B 非候选状态（`PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW`／`COMPLETED`／`FAILED`／`CANCELLED`）各一组，基线策略下 `decide(...) ⇒ Ok(NoIntent)`；六个可推进状态（`READY`／`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`EVOLVING`）各一组，假策略返回合法 `Advance` ⇒ `Ok(Advance)`。六组正例只证 `decide` 的候选校验，不证基线会选中它们 | 运行期 | 只测这十三个状态；新增状态要补；`EVOLVING` 正例只按状态合法性构造，不代表存在活跃会话 |
| **J7-b** | 基线正例对照：候选集含 `READY`／`RUNNING`／`WAITING` 各一枚，`BaselineIntentPolicy` 按 J9 次序选中预期 id ⇒ `Ok(Advance)`；证明基线不会把活跃状态过滤掉 | 运行期 | 只证基线可选中活跃状态，不证 P7D 的会话存在性；`EVOLVING` 仍受 P7D 的活跃会话条件约束 |
| **J8-a** | 假策略返回 `Advance { intent: 不在候选集的 id, .. }` ⇒ `Err(IntentDecisionError::InvalidAdvance { .. })` | 运行期 | —— |
| **J8-b** | 假策略返回 `Advance { intent: COMPLETED 候选的 id, .. }` ⇒ `Err(InvalidAdvance { .. })` | 运行期 | —— |
| **J8-c** | 假策略返回 `Advance { intent: 任一可推进候选（如 READY）的 id, .. }` ⇒ `decide` 原样返回 `Ok(Advance)` | 运行期 | —— |
| **J8-d** | 假策略返回 `Err(DecisionBasis::new("model_unavailable"))` ⇒ `Err(PolicyFailed { basis })`，依据原样带出 | 运行期 | —— |
| **J8-e** | 假策略返回 `Ok(NoIntent { basis: "" })` ⇒ `Ok(NoIntent)`，`basis` 被规范化为 `"no_intent"` | 运行期 | —— |
| **J9** | 表驱动（基线策略）：①聚焦命中；②最早 deadline（同一 Intent 多条取最早）；③最大 blocking（合并后按 `as_str()` 去重）；④输入序；⑤priority／background 变化不改变结果；⑥`Deadline`／`Blocking` 条目的 `intent` 不是候选时不影响结果；⑦重复候选 id 的行为按 §2.5 第 15 条（只断言选中的 id，不断言元数据） | 运行期 | 策略内部的权重（基线没有权重） |
| **J10-a** | `grep -n "pub fn \|fn direct" crates/continuum-intent-scheduler/src/*.rs` 逐条读签名并归类：`decide -> Result<IntentDecision, _>`、`runnable_nodes -> Vec<NodeId>`、`direct -> Result<ResourceDirective, _>` 是三个决策面；`directive_for -> Result<Option<ResourceDirective>, _>` 是转交入口；没有一条签名同时返回两个决策类型的值 | 源码面 | 新增函数要人工复核；本表是设计期清单 |
| **J10-b** | `grep -rn "IntentDecision" src/graph.rs` 零命中（文档注释除外）；`grep -rn "NodeId" src/decision.rs` 零命中（文档注释除外）；`grep -rn "ResourceDirective" src/decision.rs src/graph.rs` 零命中（文档注释除外） | 源码面 | `src/resource.rs` 允许出现 `IntentDecision`（`directive_for` 的签名）；不再要求它零命中 |
| **J10-c** | 读 `src/resource.rs` 的 `directive_for` 函数体：只做「`NoIntent` ⇒ `Ok(None)`／查找候选并校验 `is_advanceable`／组装 `ExecutionRequest`／调 `port.direct`／映射 `DirectiveForError`」；不含 `continuum_graph`／`select_runnable`／`ResourceDirective::new`／`apply_intent_transition`／`transition(`，不填槽、不产出新的 Intent 决策 | 源码面 | 转交逻辑仍可能在别处复制；由 J13 的运行期计数兜住 |
| **J11-a** | `src/graph.rs` 的 `runnable_nodes` 函数体恰一行 `continuum_graph::select_runnable(graph, config)`；无分支、无循环、无状态迁移 | 源码面 | 函数体可被改；由 J11-b 复核 |
| **J11-b** | `grep -rn "NodeState\|edges_to\|predecessors\|apply_blocking\|apply_unblocking\|transition(\|max_parallel" crates/continuum-intent-scheduler/src` 零命中（文档注释除外） | 源码面 | 注释里可能提到；限定非注释行 |
| **J12-a** | `grep -rn "impl ResourceDirectivePort" crates/continuum-intent-scheduler/src` 零命中；假实现只在 `tests/` 里 | 源码面 | 假实现可能被误当生产实现；路径区分 `src/`／`tests/` |
| **J12-b** | `grep -rn "rank(\|place(\|RoutingRequest\|PlacementRequest\|ComputeNode\|ExecutionCandidate\|continuum_model_registry\|continuum_node" crates/continuum-intent-scheduler/src` 零命中 | 源码面 | 注释里可能提到；限定非注释行 |
| **J13-a** | 假端口计数 `Cell<usize>`：`NoIntent` ⇒ `directive_for` 返回 `Ok(None)`，计数 0 | 运行期 | —— |
| **J13-b** | 合法 `Advance` ⇒ 计数 1；返回的 `ResourceDirective` 的四个访问器逐字节等于假端口填入的值 | 运行期 | 端口实现是否正确填四槽（装配方） |
| **J13-c** | 假端口返回 `Err(ResourceDirectiveError::new("x"))` ⇒ `matches!(directive_for(...), Err(DirectiveForError::Port(e)) if e.message() == "x")` | 运行期 | 错误类别（本块不定义） |
| **J13-d** | `Advance` 的 id 不在候选集／状态不可推进 ⇒ `Err(DirectiveForError::InvalidDecision { .. })`，计数 0 | 运行期 | —— |
| **J13-e** | `ResourceDirective` 的字段私有；`ResourceDirective { .. }` 在 crate 外编不过；无 `impl Default` | 编译期＋源码面 | —— |
| **J14** | `Cargo.toml` 无 `continuum-provider`／`continuum-runtime`；`grep -rn "ModelProvider\|InvokeRequest\|continuum_provider\|continuum_runtime" crates/continuum-intent-scheduler/src` 零命中（**文档注释除外**） | 源码面 | 装配方的策略实现仍可调模型；那是端口外的实现 |
| **J15** | `grep -rn "ResourceScheduler\|struct ResourceScheduler\|enum ResourceScheduler" crates/continuum-intent-scheduler/src` 零命中（文档注释除外）；本设计 §3.4 明写「§304 仍不完整」 | 源码面＋文档面 | 文档措辞会漂；实现期复核 §3.4 |
| **J16** | `cargo test -p continuum-runtime --test dependency_direction` 在含本 crate 的树上绿；`ALLOWED` 的 `continuum-intent-scheduler` 行恰三项（`continuum-canonical`／`continuum-graph`／`continuum-semantics`）；既有行不出现本 crate 名 | 运行期 | `cargo tree` 的传递依赖不在本表射程；本表只逐对断言直接边 |

## 7.3 变异预告（谁红）

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`NoIntent` 改成 `NoIntent(IntentId)` 或哨兵空串 | J1（枚举形状＋`grep`）、J6 | J2、J9（基线仍可跑） |
| **取反**：`decide` 不做候选校验，直接把策略输出返回 | J8-a／J8-b | J6、J7（空候选／无可推进候选仍返回 `Ok(NoIntent)`） |
| **取反**：`decide` 把 `InvalidAdvance`／`PolicyFailed` 折叠成 `NoIntent` | J8-a／J8-b／J8-d | J8-c／J8-e（合法路径仍对） |
| **放宽**：候选资格从六个可推进状态改成「所有非终态」 | J7（`PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW` 组会变 `Advance`） | J6（空候选仍 `Ok(NoIntent)`） |
| **取反**：把策略 `Err` 折叠成 `NoIntent` | J8-d（`PolicyFailed` 变 `NoIntent`） | J8-e（策略明确 `NoIntent` 仍对） |
| **取反**：`directive_for` 把非法 `Advance` 当 `Ok(None)` | J13-d（`InvalidDecision` 变 `Ok(None)`） | J13-a（真 `NoIntent` 仍 `Ok(None)`） |
| **取反**：基线按 `Priority` 排序 | J3-a／J3-b（若加了比较 API 才能编译，J3-c 也会红） | J9 的其他四项 |
| **取反**：`BackgroundSuitability` 改成四档枚举 | J4-a（四档原文进入 `src/`）、J4-c（若同时加 P7C 类型） | J9（基线仍不读） |
| **取反**：`SchedulerInputs` 从 `Intent` 里读 `priority` | J3-a（`intent.priority` 命中） | J2（字段名仍对） |
| **取反**：`runnable_nodes` 自己写 READY／前驱逻辑 | J11-a／J11-b | J10（返回类型仍不同） |
| **取反**：P7B 里实现 `ResourceDirectivePort` | J12-a | J13（假端口测试仍可跑，但生产实现混入 `src/`） |
| **取反**：`ResourceDirective` 加 `Default` | J13-e | J13-a／J13-b（行为不变） |
| **放宽**：`directive_for` 在 `NoIntent` 也调端口 | J13-a（计数 1） | J13-b（`Advance` 仍对） |
| **取反**：P7B 直接调 `ModelProvider` | J14 | J10（入口仍分） |
| **取反**：P7B 加 `continuum-persist` 并写 `intent` 表 | J5-a／J5-b、J16（ALLOWED 行对不上） | J6–J9（运行期判据仍可跑） |

## 7.4 拍不出照片的地方

1. **P7D 对 `NoIntent` 的处理**：P7B 只能证明 `NoIntent` 不带 `IntentId`、不表示会话不存在；
   P7D 是否真的**保留既有会话**、不清理或关闭它们，要在 P7D 的设计／测试里拍（§5.1）。收件人：P7D。
2. **P7C 的投影正确性**：P7B 只能证明自己不读 `workload_class`／`resource_pressure`；装配方的投影是否
   把 P7C 的结论正确映射到 `BackgroundSuitability`，不在本块（§4.3）。收件人：P7C ＋ 装配方。
3. **端口实现是否真的回答四问**：P7B 只能证明 `ResourceDirective` 有四槽；装配方是否填对
   `node_placement`／`model`／`tool`／`parallelism`，不在本块（§4.7）。收件人：装配方 ＋ 第 4 层。
4. **端到端三层执行**：本块没有执行器／装配方；`runnable_nodes` 的返回如何进入 `Queued → Running`、
   `ResourceDirective` 如何进入执行，都在 P1／装配方。本块的全部照片是类型层、源码面与单元／集成层。
   收件人：P1 ＋ 装配方。
5. **§304 的闭合**：本块不宣称；`rank`／`place` 之外的工具选择与并行度没有实现（切分 §6.2 第 12 条）。
   收件人：P3／第 4 层持有方 ＋ 协调者。
6. **`Priority` 的语义权重**：`Priority` 不透明，本块不能证明基线尊重 §78 的「最高语义权重」；
   候选序**不能**补救（deadline／blocking 会先于输入序决定结果），只能由注入的
   `IntentSelectionPolicy` 或 P4 的可比较取值域兑现（§3.6、§10 第 2 条）。收件人：规范维护者 ＋ P4。
7. **P7D 对 `Advance` 的状态分派**：P7B 只给 id，不携带「启动 vs 继续」；P7D 是否按候选状态走
   `Ready → Running`／`Waiting → Running`／`Suspended → Running`，并对 `Running`／`Verifying` 不迁移、
   对 `Evolving` 仅在确有对应活跃会话时继续，在 P7D 的设计／测试里拍。收件人：P7D。

---

# 8. 明确不做的

**这一节是 §1.2 的完整表**；每条给出归属与理由。**不建、不留桩。**

| 不建的东西 | 归属 | 理由 |
|---|---|---|
| Graph Scheduler 的 READY／并发／等待判定 | `continuum-graph`（`scheduler.rs:28`） | 已有实现；切分 §1.1、§五 P7B 行「不重写」 |
| `Queued → Running`、节点状态迁移、`apply_blocking`／`apply_unblocking` | P1／执行层 | P7B 不接管节点状态迁移；切分 §三第 4 条 |
| Resource Scheduler 的 router／placement／工具选择／并行度 | 第 4 层（P3） | 只有碎片（`router.rs:199`／`:405`／`:523`；`placement.rs:362`）；切分 §6.2 第 12 条 |
| Resource Arbiter、`resource_pressure`／`workload_class`、§300 四档 | P7C | P7B 不依赖 `continuum-arbiter`；切分 §三第 5 条 |
| `RoutingRequest`／`RankedExecutionCandidates`／`rank` 的第二份 | `continuum-model-registry` | P7B 不登记该边；端口实现转调 |
| `PlacementRequest`／`place`／`ComputeNode` 的第二份 | `continuum-node` | P7B 不登记该边；端口实现转调 |
| `intent` 表、迁移、`load_intent`／`list_children` 的第二套查询 | P4 语义层 | 候选集以 `&[Intent]` 输入；切分 §6.2 第 5 条 |
| Intent 状态机的新状态／新迁移／`apply_intent_transition` | P4 语义层 | §223；`transition` 是 `pub(crate)`（`intent.rs:217`） |
| Intent 的会话恢复动作（`Ready → Running`／`Waiting → Running`／`Suspended → Running`） | P7D／装配方 | P7B 只给 id；`Advance` 不携带迁移动作，P7D 按候选状态分派（§5.1） |
| 模型调用、`InvokeRequest`、`ModelProvider::invoke` | P3G／装配方 | P7B 不依赖 `continuum-provider`／`continuum-runtime`；J14 |
| 第二个「最值得继续」排名 | P7D 不写；本块的 `IntentSelectionPolicy` 是唯一面 | 切分 §五 P7D 完成判据 2 |
| 持久化、事件、审计 | 本块零表零事件 | 切分 §三第 2 条；本设计选不落库（§2.5 第 13 条） |
| §304 闭合的宣称 | 第 4 层／协调者 | 切分 §1.1、§6.2 第 12 条；本设计不宣称 |

---

# 9. 持久化与共写文件

## 9.1 结论：不建表、不建迁移、不写事件

本块**零表**。理由：

1. 调度决策是**瞬时结果**，不是规范要求持久化的对象；§302 没有给「调度结果表」。
2. 候选集以 `&[Intent]` 输入；P7B 不拥有 `Intent` 存储（P4 的 `intent` 表，迁移 110）。
3. 切分 §三第 2 条把 `continuum-persist` 列为 P7B 的**条件边**：「仅当调度结果落库」；
   本设计选**不落库**，故不登记该边。

**不占迁移号段**：P7 的 `200–209` 档由 P7A 占 `200`、P7D 条件占 `201`；P7B 不占号。
`runtime_migrations()`（`crates/continuum-runtime/src/main.rs:114`）、
`expected_migrations()`（`crates/continuum-runtime/tests/migrations.rs:37`）、
`tests/startup.rs` 的三处计数（`:26`／`:81`／`:94`）**都不改**。
`p6_pre_allowed.txt` 与 `P6_EXPECTED_ROWS` 也不改（P7D 的合并门复核，切分 §四第 1 条）。

## 9.2 共写文件

本块只碰两处（§2.1）：`Cargo.toml` 的 `members` 与 `dependency_direction.rs` 的 `ALLOWED`。
`ALLOWED` 的新行是三项 `["continuum-canonical", "continuum-graph", "continuum-semantics"]`，
其中 `continuum-canonical` 是 dev 边（仅 `tests/` 构造 `Intent` 夹具）。
**`continuum-runtime` 的 `ALLOWED` 行不动**：端口实现不在 P7B；若装配方下一轮实现，它再自己加行。
**`Cargo.lock` 随新 crate 变化**，提交时按显式路径带上。

## 9.3 事件

零事件。本块无 `continuum-events` 依赖，不写 `Event::new`／`EventType`。若将来要审计调度决策，
那是 P7D 或装配方的表，不是 P7B 的（§2.5 第 13 条、§10 第 12 条）。

---

# 10. 设计问题（逐条具名）

**每条给「缺的是哪一步」与收件人；不用「后续」「长期阶段」兜。**

1. **§302 五输入的组合函数／权重未定**。§302 只给五个名字，§78 只给因子；本设计用
   `IntentSelectionPolicy` 把组合面留成策略，`BaselineIntentPolicy` 是一个**具名基线**，不是规范排名。
   **缺的是「五个信号怎样合成一个全序」这一条规范。收件人：规范维护者 ＋ 协调者 ＋ P7D。**
   代价：两个装配方可以给出不同排名；P7D 的「最值得继续」依赖所选策略，跨实现不可比。

2. **`priority` 不透明 vs §78「用户显式 priority 应拥有最高语义权重」**。`Priority` 只派生
   `Debug, Clone`，没有 `Ord`／`PartialOrd`（`intent.rs:288-290`）；P7B 被切分 §2.4 要求**不解析**它。
   **缺的是「`Priority` 的可比较取值域」或「装配方解释 `Priority` 的策略」这一步。
   收件人：规范维护者 ＋ P4 语义层持有方。** 代价：默认基线不按 priority 排序，且**不能用候选序补救**
   （deadline／blocking 会先于输入序决定结果）；要兑现 §78 必须注入自定义 `IntentSelectionPolicy`（§4.4）。

3. **`BackgroundSuitability` 的投影取值域未定**。切分 §2.4 要求装配方从 P7C 的
   `workload_class`＋`resource_pressure` 做**一次**投影；本设计只冻载体，不定义 `Suitable`／`Unsuitable`。
   **缺的是「投影的取值域与语义」这一步。收件人：P7C ＋ 装配方 ＋ 规范维护者。**
   代价：基线不能使用该信号；装配方与模型策略必须自己约定字符串词汇。

4. **`ResourceDirectivePort` 的实现要消费 P7C 结论（P7C 未落地）**。P7B 可以现在冻 trait 与四槽，
   但真正的实现要消费 P7C 的 `resource_pressure`／`workload_class`／§300 四档，并转调
   `rank`／`place`／工具路径。**P7C 结论由实现方在构造端口值时持有，不是 trait 参数**；切分 §四第 6 条
   的「P7B 与 P7C 无未冻接口」成立。**缺的是「端口实现」这一步，不是接缝。收件人：P7C ＋ 装配方。**
   代价：P7B 的测试只能用假端口；四槽是否够用、槽格式是否可消费，要等实现轮验证。
   若 P7C 要求每次请求传入结论，才增加不透明 `ResourceContext` 参数，不把 P7C 类型带进 P7B。

5. **§7.4 的四条判据不覆盖 P7B**。四条分别落在 P7A（判据 1、2、4）与 P7C（判据 3），
   P7B 没有规范判据（切分 §七第 12 条）。本设计自报 J10（三层分离的结构照片）作为完成判据，
   **不修改 §7.4**。**缺的是「P7B 是否需要在《工程》§7.4 有一条正式判据」这一步。
   收件人：协调者 ＋ 《工程》维护者。** 代价：P7B 的验收依赖本设计的自报判据，跨文档不一致时没有规范锚。

6. **Graph 转调入口的归属**。切分 §三第 4 条说 P7B 消费 `select_runnable`；本设计把它做成
   `graph::runnable_nodes` 的**零逻辑转调**。另一种做法是把调用点完全留给装配方，P7B 不暴露 Graph 入口。
   **缺的是「P7B 是否应暴露这个入口」这一步。收件人：协调者 ＋ P1／第 3 层持有方。**
   代价：暴露入口可能被误读为 P7B 拥有 Graph Scheduler；不暴露则 `select_runnable` 仍没有生产调用方，
   切分 §三第 4 条的「消费」只在装配方发生。

7. **候选状态集是六个可推进状态**。§302 没有给候选状态集；本设计按 §223 取 `READY` 与五个活跃状态
   （`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`EVOLVING`）。**不再要求先迁到 `READY`**：
   `Waiting → READY`／`Suspended → READY` 在 §223 不存在，`transition` 也是 `pub(crate)`。
   P7D 对 `Advance` 按候选状态分派：`Ready`／`Waiting`／`Suspended` 走既有 `→ Running`，
   `RUNNING`／`VERIFYING` 不迁移、只继续；`EVOLVING` 仅当 P7D 确有对应活跃会话时才继续，否则不进候选。
   **缺的是「Intent Scheduler 的候选状态集」
   这一条规范。收件人：规范维护者 ＋ P7D。** 代价：候选集包含活跃会话时，P7D 必须自己按状态分派；
   P7B 不替它选「启动 vs 继续」。

8. **`deadline`／`blocking`／`user focus` 的形状与语义未定**。§302 只给名字；本设计给
   `Deadline { intent, at_millis }`／`Blocking { intent, blocks }`／`UserFocus::{None, Intent, Area}`。
   **缺的是「三个信号的类型、量纲、合并规则」这一条规范。收件人：规范维护者 ＋ 装配方。**
   代价：调用方要自己构造；同一 Intent 多条信号的合并只在基线里有约定，不是规范不变量。

9. **模型驱动策略的边界**。《工程》§7.2 `:435` 说 Intent Scheduler 包含模型判断；本设计把模型判断
   放在 `IntentSelectionPolicy` 的实现里，P7B 只定义 trait 与确定性基线。**失败面已冻结**：`choose`
   返回 `Result<IntentDecision, DecisionBasis>`，策略失败走 `Err`，由 `decide` 包成
   `IntentDecisionError::PolicyFailed`，不再折成 `NoIntent`；panic 不是失败面，策略实现不得用它表达
   可恢复失败。**缺的是「模型驱动策略由谁实现、提示、
   重试／降级」这一步。收件人：协调者 ＋ P3G／P8。** 代价：P7B 不保证模型策略可用；
   它只保证 Intent 入口不直接依赖 `ModelProvider`（J14），且失败可区分。

10. **`DecisionBasis` 的不透明程度 vs P7D 的展示需要**。本设计是 `String`；P7D 可以展示，
    但不能按内容分支。若 `SessionPlan` 需要结构化原因（例如「因资源不适合而 NoIntent」），
    现在不冻，P7D 只能拿字符串。**缺的是「依据要不要结构化」这一步。收件人：P7D ＋ 规范维护者。**
    代价：P7D 的 UI／审计表达力受限；若将来结构化，`DecisionBasis` 是破坏性改动。

11. **`ResourceDirective` 四槽的内容格式未定**。四槽是 `Option<String>`；装配方填，消费方解析。
    **缺的是「槽内容的语法／版本」这一步。收件人：装配方 ＋ P8。** 代价：消费方要自己解析；
    没有逐槽类型安全；不同装配方可能用不同格式。

12. **零持久化：调度决策是否要审计**。本块不落库、不写事件；P7D 若要审计，必须自己落。
    **缺的是「调度决策的审计归属」这一步。收件人：协调者 ＋ P7D。** 代价：决策不可回放；
    跨进程恢复只能靠重新决策。

---

# 11. 实测与口径订正（只记，不改）

1. **§7.4 不覆盖 P7B**：四条判据分别落在 P7A（判据 1、2、4）与 P7C（判据 3）；P7B 没有规范判据。
   本设计自报 J10，不修改 §7.4（§3.7、§10 第 5 条）。
2. **§13 的输出示例 `INTERACTIVE` 不在四档里**：四档是 `INTERACTIVE_GAME`／`COMPUTE_WORKLOAD`／
   `COMPILER`／`VIDEO_RENDER`（`docs/spec/01-concepts.md:708-715`），示例 `:721` 的 `INTERACTIVE`
   按笔误处理（切分 §2.4）；本设计不设第五档。
3. **`IntentId`／`Priority` 没有 `PartialEq`／`Ord`**：`IntentId` 只派生 `Debug, Clone`
   （`crates/continuum-semantics/src/ids.rs:41`）；`Priority` 只派生 `Debug, Clone`，且明写
   「不实现 `Ord`／`PartialOrd`、也没有比较方法」（`intent.rs:288-290`）。本设计的类型不强行补比较；
   测试用 `as_str()` 比较。
4. **`select_runnable` 今天没有生产调用方**：`continuum-graph` 的测试是唯一调用面
   （`crates/continuum-graph/tests/scheduler.rs`）；P7B 的 `runnable_nodes` 是第一个库入口，
   真正的生产调用仍要等装配方。**不把「P7B 包了一层」当成「执行器已接线」。**
5. **`place` 今天没有生产调用方**：`crates/continuum-node/src/placement.rs:356-358` 逐字写
   「本函数在 `src/` 里零调用点」；`ExecutionProfile.compute_node` 是裸 `Option<String>`
   （`crates/continuum-graph/src/execution.rs`）。P7B 的端口不收紧这个字段；`place` 的装配点是
   第 4 层的缺口（切分 §6.2 第 12 条）。
6. **切分 §2.3.2 的 `Advance(IntentId)`／`NoIntent` 是文字形状**：本设计取结构体变体
   `Advance { intent, basis }`／`NoIntent { basis }`，两臂语义不变；这是 §2.5 第 1 条的自定。
7. **切分 §2.3.2 的 `NoIntent ⇒ 无会话` 措辞与 §223 不相容**：P7D 可能在已有 `RUNNING`／`WAITING`／
   `SUSPENDED` 会话时调 `decide`；`NoIntent` 只能表示「本轮没有新的可推进项」，不能表示「没有会话」。
   本设计按此订正为「P7D 必须保留既有会话」；建议协调者同步切分 §2.3.2。收件人：协调者 ＋ P7D。

---

# 12. 验收口径

P7B 交付的是：

1. **一个 Intent Scheduler**：`IntentScheduler::decide` 返回
   `Result<IntentDecision, IntentDecisionError>`；`IntentDecision` 保持两臂（含 `NoIntent`），
   `Result` 另有显式错误臂；不解析 `priority`，只从 `&[Intent]` 候选集的六个可推进状态里选
   （`READY` 与五个活跃状态；`EVOLVING` 仅在 P7D 确有对应活跃会话时进入候选）。
2. **一个具名资源端口**：`ResourceDirectivePort` 是 trait；`ResourceDirective` 是四槽不透明载体；
   P7B 不实现端口、不依赖 P7C、不宣称 §304 闭合。
3. **一个零逻辑 Graph 消费入口**：`graph::runnable_nodes` 是 `select_runnable` 的转调；
   不判 READY、不接管节点状态迁移。
4. **三层分离的结构照片**：J10；三个决策面的返回类型互不相同，没有合并函数；`directive_for` 是转交入口，不产出新的 Intent／Graph 决策。
5. **可测试的判据**：J1–J16；其中 J3／J4／J5／J10／J11／J12／J14／J15 以源码面为主，
   J1／J2 是源码面＋编译期（J1 另有运行期），J6–J9／J13 是运行期，J16 是依赖门。

**P7B 结束时不宣称 §304 已闭合。** 第 4 层的工具选择、并行度与 `place` 装配点仍在切分 §6.2 第 12 条；
本设计只把「三层必须分离」做成可观察结构，并把资源答案的**承载形状**留给装配方。
