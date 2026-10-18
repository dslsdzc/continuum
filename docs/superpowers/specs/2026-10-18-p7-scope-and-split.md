# P7 子项目（长期循环层）的归属与接口（共享面）

> 本文不是设计，是**四份设计并行的前提**。它的作用是：让 P7A / P7B / P7C / P7D 四份设计**对着同一套归属、
> 同一套接口、同一套名字**写，不至于各造一套词汇——「同一件事两个词汇表」在本项目一贯判为 **Critical**。
>
> 本文里的判断若与协调者的决定冲突，以本文为准并回报；本文之外的口径以规范为准。
>
> **本文只做切分与归属登记，不替任何子项目写设计。** 每块的具体类型、字段、判定算法与测试照片，
> 一律留给该块自己的设计。

---

## 一、路径归属

### 1.1 本阶段的规范范围

**P7 的规范范围是《工程》§7 全节**——§7.1 的八行组件表（节标题 `docs/02-工程.md:420`，表体 `:422-431`）、§7.2 的组件边界
（`:433-443`）、§7.3 的层内依赖（`:445-458`）、§7.4 的四条完成判据（`:460-467`）、§7.5 的两处缺口（`:469-473`）。
**阶段名与判据都只在工程侧成立**：《总纲》`:4` 明写「本文不规定实施顺序或里程碑安排」，
故「P7」是工程侧的分解名，不是规范的要求。

《总纲》第 7 章（`docs/01-总纲.md:1127-1322`）是这八行的展开：§7.1 两种 Intent（`:1131`）、§7.2 工程可行性
（`:1143`）、§7.3 双层循环（`:1174`）、§7.4 Milestone 惰性生成（`:1197`）、§7.5 产品状态评估（`:1229`）、
§7.6 无效迭代抑制（`:1244`）、§7.7 可发布状态（`:1262`）、§7.8 后台（`:1275`）、§7.9 资源让路（`:1290`）、
§7.10 会话启动（`:1304`）。**工程 §7 的八行与本文件的四块是同一件事的两种切法**，不是两套组件清单。

**一处必须先写在前面的事实**：§7.1 的「三层调度」**不是三件都从零建，也不是三件都已存在**。
Graph Scheduler 已在第 3 层落地（`crates/continuum-graph/src/scheduler.rs:28` 的 `select_runnable`，
`crates/continuum-graph/src/lib.rs:29` 导出）；**Resource Scheduler 在第 4 层只有碎片**：
`crates/continuum-model-registry/src/router.rs:199`／`:405` 只回答「用哪个模型」，`rank` 在 `:523`；
`crates/continuum-node/src/placement.rs:362` 的 `place` 只回答「Node 在哪里执行」，且 `src/` 里零生产调用点
（`:357-358`）；「哪个工具」由调用方给（P3F 设计 `:360`），「多少并行」只有 Graph Scheduler 的
`SchedulerConfig.max_parallel`（`scheduler.rs:14-17`）。**在三层调度里，P7 只建 Intent Scheduler**，并冻结一个由装配方
实现的**具名资源端口**（内部转调 `rank`／`place`／既有工具路径），把「三层必须分离」做成可观察的结构；
**P7 不重造 Graph／Resource Scheduler，第 4 层的剩余缺口另记 §6.2 第 12 条**。重复实现 Graph／Resource
Scheduler 是本文判定的 **Critical**（§五 P7B、§七第 5、6 条）；**P7 结束时 §304 仍不完整**，不得从
「主要判定面已分落」推出「三层调度已闭合」。

### 1.2 谁拥有什么

| 职责／路径 | 归属 | 规范依据 |
|---|---|---|
| Product 对象（含 `baseline`）、ProductReadiness 九维及其 `value/evidence/last_verified` | **P7A** | §150 §231 §232 §153；《工程》§7.1（`:424`）、§7.2（`:435`）、§7.3（`:448-449`） |
| Milestone 数据模型、惰性生成与批次、**Milestone→Product Intent 的 lower** | **P7A** | §233 §234 §151 §152；《工程》§7.1（`:425`）、§7.2（`:437`）、§7.3（`:450`） |
| Progress Gate、`progress_delta`、无进展时触发 replan／Decision（Decision 由 P7A 出；replan 消费方见 §6.2 第 13 条） | **P7A** | §337 §154；《工程》§7.1（`:426`）、§7.2（`:439`）、§7.3（`:451`）、§7.4 判据 2（`:464`） |
| Shippable Predicate、Shippable 与 post-shippable 的区分 | **P7A** | §335 §336 §155；《工程》§7.1（`:427`）、§7.3（`:455`）、§7.4 判据 4（`:466`） |
| Intent Scheduler 本体；§301 的三层分离结构 | **P7B** | §301 §302 §77；《工程》§7.1（`:431`）、§7.2（`:435`） |
| Graph Scheduler 的**消费**（复用，不重造） | **P7B** | §303；`crates/continuum-graph/src/scheduler.rs:28` |
| Resource Scheduler 的**具名端口**（P7B 定义、装配方实现；不重造；第 4 层只有碎片） | **P7B** | §304；《工程》§7.2（`:435`） |
| Resource Arbiter：资源压力、负载类别、四档优先级、协作式挂起、最小暴露面 | **P7C** | §300 §13 §14；《工程》§7.1（`:429`）、§7.2（`:441`）、§7.3（`:452`）、§7.4 判据 3（`:465`） |
| Background Supervisor：§299 的四类任务来源与后台任务治理 | **P7D** | §299 §12；《工程》§7.1（`:428`）、§7.2（`:443`）、§7.3（`:453`） |
| Session Orchestrator：Manual／Auto 两种模式与 Session Plan | **P7D** | §11；《工程》§7.1（`:430`）、§7.3（`:454`） |
| §171 Dreaming 的**触发点**（不持阈值；直接依赖 `continuum-memory` 调 P6C） | **P7D** | §171；P6C 的 `begin_synthesis`／`fail_synthesis`／`apply_synthesis`（`crates/continuum-memory/src/synthesis.rs:108`／`:217`／`:306`） |
| Product 的身份：与 `Intent.product_id` 同一枚 `ProductIdRef`，不另造 `ProductId` | **P7A** 消费，**不新建** | §231；`crates/continuum-semantics/src/intent.rs:372`、`:464` |
| P6D 合并门的重新基线化或退役 | **P7D**（协调者收） | P6D 设计 §7.3.5；`crates/continuum-runtime/tests/dependency_direction.rs:784` |

**新建 crate 的具体名字由各块自己的设计定**，本文只登记「这一块拥有它」；四块之间不得出现两枚同义 crate。
本文建议名：P7A `continuum-product`、P7B `continuum-intent-scheduler`、P7C `continuum-arbiter`、
P7D `continuum-supervisor`（Session Orchestrator 与 Supervisor 同 crate；若拆，拆出的第二枚仍归 P7D，两行都登记）。
**`continuum-intent-scheduler` 这个名字只表示 Intent Scheduler，不表示拥有 Graph／Resource Scheduler**；
Graph Scheduler 仍在 `continuum-graph`，Resource Scheduler 不在 P7 内。

### 1.3 共写文件

| 路径 | 谁改 |
|---|---|
| `Cargo.toml`（`[workspace] members`） | **共写**：每块加自己那一行 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | **共写**：每块加自己新建 crate 的那一行；**并集的复核具名给 P7D**（理由见 §四 第 1 条） |
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`） | **共写**：只有落库的块加自己那一行（**号段见 §四 第 2 条**） |
| `crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`） | **共写**：与上一行同批、同一块改；两份清单不得分叉 |
| `crates/continuum-runtime/tests/startup.rs` 的三处计数（`:26`、`:81`、`:94`） | **共写**：与上两行同批、同一块改（P6C 曾漏这里，见 `docs/superpowers/p6-followups.md:40-42`） |
| `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 与 `P6_EXPECTED_ROWS`（`:551`） | **P7D 一处改**：重新基线化或退役，**不得在 P6 的 fixture 上追加行**（P6D 设计 §7.3.5）；若重基线化，解析器 `:615` 与断言 1 `:790` 的两个 `27` 字面量同批改 |

---

## 二、接口冻结现状

### 2.1 规范已给形状、本轮照抄、不新冻

四份设计**不得改名、不得另造第二套词汇**。以下名字由规范给定：

- §150 的 `EngineeringBaseline` 十个字段：`product_goal` `target_users` `required_capabilities`
  `hard_constraints` `architecture` `technology_choices` `minimum_complete_system` `known_unknowns`
  `engineering_risks` `acceptance_criteria`（`docs/spec/03-product-drive.md:43-118`）；
- §231 的 `Product` 七个字段：`id` `intent_id` `baseline` `readiness` `current_milestone` `milestone_history[]`
  `shippable_predicate`（`docs/spec/05-normative.md:424-438`）；
- §232 的 `ProductReadiness` 九维：`functional` `correctness` `reliability` `performance` `security` `usability`
  `maintainability` `deployability` `documentation`，以及 `value` `evidence` `last_verified` 三项
  （`docs/spec/05-normative.md:442-466`）；
- §234 的 `Milestone` 七个字段与七个状态 `PROPOSED` `APPROVED` `ACTIVE` `VERIFYING` `COMPLETED` `FAILED`
  `SUPERSEDED`（`docs/spec/05-normative.md:498-525`）；
- §299 的四类后台任务来源：`authorized Intent` `policy` `maintenance schedule` `product drive`
  （`docs/spec/05-normative.md:1892-1903`）；
- §300 的四档资源优先级：`User workload` `> Foreground Runtime` `> Background Runtime` `> Maintenance`
  （`docs/spec/05-normative.md:1907-1919`）；
- §301 的三个调度器名字：`Intent Scheduler` `Graph Scheduler` `Resource Scheduler`
  （`docs/spec/05-normative.md:1925-1935`）；
- §302 的五个输入：`priority` `deadline` `blocking` `user focus` `background suitability`
  （`docs/spec/05-normative.md:1939-1955`）；
- §304 的四个问题：`Node 在哪里执行` `用哪个模型` `哪个工具` `多少并行`
  （`docs/spec/05-normative.md:1973-1982`）；
- §335 只给名字 `ShippablePredicate`，**没给字段**（`docs/spec/05-normative.md:2535-2547`）；
- §337 只给名字 `progress_delta`，**没给量纲与阈值**（`docs/spec/05-normative.md:2583-2597`）。

### 2.2 已在代码里、P7 一个都不碰

| 已有的东西 | 实测位置 | 对 P7 的约束 |
|---|---|---|
| `Intent` 的 §222 十五字段／代码 14 个存储字段（`child_intents[]` 派生）、`IntentStatus` 十三态、`IntentId`、`ProductIdRef`、`Priority` | `crates/continuum-semantics/src/intent.rs:434`、`:61`、`:372`、`:300`；`crates/continuum-semantics/src/ids.rs:41` | P7 只读不造第二份；`Product.id` 必须与 `ProductIdRef` 同一词汇 |
| `Decision` 与 `create_decision`（写 `decision.required`） | `crates/continuum-semantics/src/decision.rs:80`、`crates/continuum-semantics/src/persist.rs:1173` | P7A 的 Progress Gate 只经它出 Decision；不自造「决策单」 |
| `Evidence`、`CompletionVerdict`、`judge_completion` | `crates/continuum-verify/src/evidence.rs:222`、`completion.rs:24`、`:104` | P7A 的 readiness／shippable 只消费它们；不得另造 Evidence 或「完成判定」 |
| Graph Scheduler：`select_runnable`／`SchedulerConfig` | `crates/continuum-graph/src/scheduler.rs:28`、`:9` | P7B 只调用；不重写 READY／并发判定 |
| Resource Scheduler 的既有碎片：`RoutingRequest`／`RankedExecutionCandidates`、`place` | `crates/continuum-model-registry/src/router.rs:199`、`:405`、`crates/continuum-node/src/placement.rs:362` | P7 不拥有这些类型，不在 P7 里再造 router／placement；P7B 只冻结具名资源端口，装配方转调这些碎片；缺的工具选择／并行度／`place` 装配点见 §6.2 第 12 条 |
| 预算五维与账本（`DimensionKind`／`Dimensions`／`check`） | `crates/continuum-budget/src/lib.rs:1-68`、`dimensions.rs` | 它是**记账**，不是 Resource Arbiter；P7C 不得拿它当资源仲裁 |

### 2.3 本轮要新冻的：所有权声明与跨块冻结点

**先分两类**：**块内所有权声明**只钉「这件事只有这一块定义、只长一张脸」，没有别的块消费它，因此不产生串行；
**跨块冻结点**才是消费方要对着写的页，只有两页。**不新冻规范已给的东西**（§2.1），**也不新冻各块自己
就能闭合的算法**（评分、权重、模型提示）。

#### 2.3.1 块内所有权声明（不产生串行）

1. **P7A 的 Product 面**：`Product`／`ProductReadiness`／`Milestone`／`ProgressGateVerdict`／`ShippableVerdict`
   **各一份**，由 P7A 定义；字段名照 §231／§232／§234，块设计只补规范没给的部分。`Product.id` 与
   `Intent.product_id` 同一枚 `ProductIdRef`，**不新造 `ProductId`**。`Product.baseline` 的类型是 §150 的
   `EngineeringBaseline`，由 P7A 生成并写入（模型判断经 §三第 5 条的端口），不由调用方塞一个不透明值。**`ShippablePredicate` 是输入条件，
   `ShippableVerdict` 是判定结果，两个词不混用**；`ShippableVerdict` 至少三值：`Shippable`／`NotShippable`／
   `Undetermined`，`Shippable` 只在 `ShippablePredicate`＋readiness 成立且 Completion 不处于
   `Blocked`／`Undetermined` 时给出（§四第 4 条）；`Blocked` 或 predicate／readiness 明确不成立落
   `NotShippable`，输入不可判落 `Undetermined`。baseline 缺失或不可判时 ProductReadiness 只能
   `Undetermined`，不得默认通过。§336 的 `ContinuousProductDrive` 开关与 post-shippable
   分类的落点（`Product` 或 `ProductReadiness`／verdict 之一）由 P7A 设计定，本轮只登记归属。
2. **P7D 的 `TaskSource` 与 `AuthorizedTask`**：`TaskSource` 恰 §299 的四值，**不新增第五类**；
   `AuthorizedTask` 是 P7D 的唯一任务输入面，至少含 `source: TaskSource` 与授权证据。**证据形状按来源定**；
   本文只冻结 `product drive` 这一路（P7A→P7D 没有 policy／capability 边）：证据取 §222 `Intent` 的
   `contract_id`＋`status` 二元组，判定规则见 §三第 6 条。其余来源的 policy／capability 引用由 P7D 设计
   对着第 5 层写，本文不预冻。构造器私有，P7D 不接受裸 `TaskSource` 标签，也不在 P7D 内解释 policy 语义；
   授权判断留在第 5 层，P7D 只校验证据存在且来源匹配；**不新增 `continuum-policy`／`continuum-capability` 边**。
   `SynthesisTrigger` 的四个**时机**
   （`crates/continuum-memory/src/synthesis.rs:64`）与它不是同一枚举，见 §2.4。
3. **P7D 的 Session 面**：`SessionPlan` 与「后台新结果」的摘要面；Manual／Auto 两模式共用同一份形状。
   **模型提示、候选排序不冻**——Auto 的「最值得继续」必须调 P7B 的决策面，不得另写一套排名。
4. **P7D 的 §171 触发面**：P7D 直接依赖 `continuum-memory`，调 `begin_synthesis`／`apply_synthesis`／
   `fail_synthesis`；`SynthesisTrigger` 的四个时机由 P6C 冻结，P7D 只决定「何时调」，不改类型、不持阈值。
   这是第 6 层 → 第 7 层的既有方向，`ALLOWED` 登记 `continuum-memory` 这一行。

#### 2.3.2 真正跨块的冻结点（两页，消费方对着写）

1. **P7B 的 Intent Scheduler 决策面**：输入是 `&[Intent]` 加 §302 的**显式信号**（不是从 `Intent` 里猜；
   其中 `background suitability` 按 §2.4 的唯一投影收成不透明 `BackgroundSuitability`），输出是
   `IntentDecision`：`Advance(IntentId)` 或 `NoIntent`，加一份不透明的依据。`NoIntent` 表示没有可推进的
   Intent，P7D 必须按「无会话／无后台任务」处理，不得用哨兵 `IntentId`。**评分算法、权重、模型提示不冻**。
   这一页由 P7B 开工第一件事冻结，P7D 对着它写。
2. **P7C 的仲裁结论面**：对 AI／后台暴露的结论**只有** `resource_pressure` 与 `workload_class` 两项
   （§13 的原话，`docs/spec/01-concepts.md:717-722`）；**§300 的四档优先级另有一张只对装配方
   （`ResourceDirectivePort` 的实现）可见的面，不进 AI／后台暴露面，也不进 P7D 的结论面**——四档分类由
   P7C 给出，装配方只消费，不自行做资源仲裁。对 P7D 另有 `run`／`throttle`／`suspend`／`resume`
   四档动作，以及可观察的 `SuspensionPhase`（`Running`／`Throttled`／`Checkpointing`／`Suspended`）。
   **`suspend` 是两阶段：先 checkpoint 再 suspended**；`Checkpointing` 表示「已要求挂起但尚未完成」，
   checkpoint 失败不得报 `Suspended`。**`Checkpointing` 必须有失败臂**：checkpoint 失败返回错误
   （错误类型由 P7C 设计命名），相位回 `Throttled`；不得停在 `Checkpointing`，也不得报 `Suspended`；
   `Suspended` 只在 checkpoint 成功后可达。若实现选择重试，重试窗口内相位仍是 `Checkpointing`，但必须有
   上限／超时并最终落上述错误＋`Throttled`。**原始进程信息（进程名、PID、CPU/GPU 占用）不进公开类型**。
   这一页由 P7C 开工第一件事冻结，P7D 对着它写。

### 2.4 同名异物（**都不是**同一个概念）

| 已有的东西 | 它是什么 | 实测位置 | 处置 |
|---|---|---|---|
| `Product` | §231 的产品对象／`MemoryScope::Product`／`Intent.kind = Product`／`Intent.product_id` | `crates/continuum-memory/src/claim.rs:20`、`crates/continuum-semantics/src/intent.rs:40`、`:464` | P7A 只认 §231；Product 身份用 `ProductIdRef`；**不把 memory scope 当产品** |
| `Suspended` | `NodeState::Suspended`／《总纲》§14 的协作式挂起态／`IntentStatus::Suspended` | `crates/continuum-graph/src/node.rs:63`、`docs/spec/01-concepts.md:741`、`crates/continuum-semantics/src/intent.rs:77` | P7C 自建挂起状态机；**不复用 `NodeState`**，只把「挂起」结果映射到节点的既有迁移；`suspend` 两阶段：`Checkpointing` →（成功）`Suspended`／（失败）`Throttled`＋错误 |
| `Decision` | §331 的决策单／策略的 `Allow/Deny/RequireApproval` | `crates/continuum-semantics/src/decision.rs:39-44` | P7A 用前者；若同文件出现后者，一律别名 `PolicyDecision` |
| `priority` | `Intent.priority` 是**不透明** `Priority`／§302 的输入之一／§78 的七因子 | `crates/continuum-semantics/src/intent.rs:300`、`docs/spec/05-normative.md:1950` | P7B **不解析** `Intent.priority`；§302 的五个信号另收显式输入 |
| `workload_class` | §13 的四档进程类／§13 输出示例里的 `INTERACTIVE` | `docs/spec/01-concepts.md:708-715`、`:717-722` | `INTERACTIVE` **不在四档里**；P7C 只用四档，示例按笔误处理并报规范维护者 |
| `background suitability` | §302 的输入信号／P7B 的 `BackgroundSuitability`（不透明）／由装配方从 P7C 的 `workload_class`＋`resource_pressure` 唯一投影 | `docs/spec/05-normative.md:1939-1955`、`docs/spec/01-concepts.md:717-722` | **不是第二套资源判断**：投影只实现一次（装配方）；P7B 只收不透明值、不解析 `workload_class`，P7C 不依赖 P7B；两块**设计**可并行，运行时由装配方接线 |
| `TaskSource` | §299 的四类任务来源／`SynthesisTrigger` 的四个时机／§79 的四档交互优先级 | `docs/spec/05-normative.md:1892-1903`、`crates/continuum-memory/src/synthesis.rs:64`、`docs/spec/02-positioning.md:655-669` | 三个四值枚举**不得合并**；P7D 只拥有第一枚，第二枚由 P6C 拥有，第三枚是 §300 的解释面 |

---

## 三、跨层边

1. **§9.1 的层间图与 P7 有关的是四条**（`docs/02-工程.md:569-588`，箭头读作「被依赖者 → 依赖者」）：
   `资源层 (4) → 长期循环 (7)`、`边界层 (5) → 长期循环 (7)`、`认知层 (6) → 长期循环 (7)`、
   `跨领域 (8) → 长期循环 (7)`。故 P7 依赖第 2、4、5、6、8 层是**既有方向**，不是反向边。
2. **P7 的直连边只登记实际用到的**。本文按块给出「确定」与「条件」两栏；条件栏**用了才登记，不用不登记**：

| 块 | 拟名 crate | 确定的上游 | 条件边（设计定稿时按实际使用） | 绝不登记 |
|---|---|---|---|---|
| P7A | `continuum-product` | `continuum-semantics`（`Intent`／`Decision`）、`continuum-verify`（`Evidence`／`CompletionVerdict`）、`continuum-persist`（迁移 200）、`continuum-canonical`（§152 lower 确定构造 `Intent`，要 `EntityId`） | 无 | `continuum-runtime`、`continuum-provider`、`continuum-model-registry`、`continuum-node` |
| P7B | `continuum-intent-scheduler` | `continuum-semantics`（`Intent`）、`continuum-graph`（`select_runnable`／`SchedulerConfig`） | `continuum-persist`（仅当调度结果落库） | 第 4 层的 router／placement（经具名端口，见下条第 5 点） |
| P7C | `continuum-arbiter` | **无**（workspace 内零依赖） | 无 | 任何 workspace 内 crate；任何模型／上下文类型 |
| P7D | `continuum-supervisor` | `continuum-intent-scheduler`、`continuum-arbiter`、`continuum-semantics`、`continuum-memory`（§171 触发） | `continuum-persist`（仅当会话／后台作业落库）、`continuum-canonical`（仅当自己造 `Intent`）、`continuum-verify`（仅当后台发现以 `Evidence` 形态进入） | `continuum-product`（P7D 不持产品状态）、`continuum-runtime`、`continuum-provider` |

3. **`continuum-verify` 是 P7 的「验证结果」上游**：§232 的 `evidence` 与 §7.3 的「第 8 层验证结果」都指它。
   P7A **不把模型自述、原始工具输出或 `Claim` 当证据**；只认 `Evidence` 与 `CompletionVerdict`（§四 第 4 条）。
4. **`continuum-graph` 是 P7 的「执行层既有件」**：P7B 消费 `select_runnable`，不接管 `Queued → Running`、
   不接管节点状态迁移。**这条是第 3 层 → 第 7 层的直连边**，不在 §9.1 的相邻层链上，但不成环，
   且与 P5 分块「只消费第 3 层既有判据」同形；P7B 的设计要写明这一句。
5. **模型调用、资源放置与工具选择一律经端口，P7 不建第二条调用路径**：P3G 的模型调用路径在
   `crates/continuum-runtime/src/model_call.rs:430`（`call`），P7 的 crate **不得**直接依赖
   `continuum-runtime`／`continuum-provider`，也不得直接 `ModelProvider::invoke`。P7 各块按需定义
   「判断输入／输出」的端口，由装配方 `continuum-runtime` 实现并转调既有路径。**P7 不冻一个跨四块的
   通用模型端口**——四个使用点（Engineering Baseline 生成、Milestone 提议、Intent 排序、Session 摘要）
   是不同的语义判断，共用的只是底层调用路径，而那条路径不在 P7。**P7B 另冻一个具名 `ResourceDirectivePort`**：
   输入是一次执行请求（Intent／Graph 节点＋显式信号），输出是一份不透明指令值，至少能承载
   §304 的「Node 在哪里执行／用哪个模型／哪个工具／多少并行」；装配方实现，内部转调 `rank`／`place`／
   既有工具路径。**端口不是 Resource Scheduler 本体**，也不是「已有面」；`place` 的装配点由本端口第一次
   给出生产调用方，工具选择与并行度决策仍在第 4 层缺口里（§6.2 第 12 条）。**消费方写死：端口的实现
   （装配方）消费 P7C 的仲裁结论——`resource_pressure` 与 §300 的四档优先级；P7B 只定义 trait、持不透明
   指令值并把它用于调度结果，不依赖 P7C，也不读 `workload_class`／`resource_pressure`；P7C 不依赖 P7B。**
   即「P7B→端口←P7C」由装配方汇合，两块设计可全并行。
6. **P7A 与 P7D 之间有一条按值交接的 Product Intent**：P7A 按 §152 的 lower 确定产出
   `Intent.kind = Product` 的值（`product_id: ProductIdRef`，其余字段照 §222 的 `Intent`），P7D 的
   `product drive` 来源按值消费它；**这不是新的 crate 边**（P7D 不登记 `continuum-product`），
   也不要求 P7A 等 P7D。交接约定：只交 **`status == READY`** 的 Product Intent 值或 id；`PROPOSED` 的
   lower 结果不算交接完成，须由 §223 的状态机推进到 `READY` 后才可交；P7D 不读 §231 的 Product 记录。
   §156 的「后台继续低风险 Milestone」就是这一路。
   **`READY` 是唯一可用的状态证据**：P4 的迁移表里 `READY` 只有 `REVIEW → READY` 一条入边
   （`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:879`；
   `crates/continuum-semantics/src/intent.rs:249`），故「已过 §223 的 `REVIEW`」由状态本身可证明。
   运行态不能证明：同一张表允许 `PLANNING → SUSPENDED`（P4 设计 `:878`；`intent.rs:247`）与
   `SUSPENDED → RUNNING`（P4 设计 `:883`；`intent.rs:264`），故 `PLANNING → SUSPENDED → RUNNING`
   可绕过 `REVIEW`；把 `RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`EVOLVING` 任一当证据都会接受
   一条从未过审的 Intent。
   **授权证据（选项 a，本轮写死）**：不新增证据输入面；证据就是交接值上的 `contract_id`
   （`ContractRef { id, version }`，`crates/continuum-semantics/src/intent.rs:458`）与 `status`
   （`IntentStatus`，`:61`／`:445`）。P7D 的私有构造器按下列规则判定，任一不满足即拒绝：
   (i) `kind == Product`；(ii) `contract_id.id` 非空且 `contract_id.version >= 1`；
   (iii) `status == READY`；
   `PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW` 与 `RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／
   `EVOLVING`／`FAILED`／`CANCELLED` 一律拒绝。
   **`COMPLETED` 单列，不并入终态拒绝**：它对 Product 不是严格终态（P4 只允许 `COMPLETED → EVOLVING`、
   仅 Product，P4 设计 `:885`／`:893`；`intent.rs:222`）。但它也不是可执行任务，P7D 不接；处置是
   **P7A 在交接前把该 Product Intent 推进到 `EVOLVING`（或按新的 Milestone 重新 lower 出一个 `READY`
   的 Product Intent）**，P7D 只认 `READY`；`COMPLETED` 本身按「非 `READY`」拒绝。`FAILED`／`CANCELLED`
   才是终态。
   `contract_id` 指向 §224 的 `TaskContract`（含 `authority_limit` 与三组 policy）；P7D 只查这两个字段，
   不读合同内容、不解释 policy。**代价照实写**：这是「合同版本引用
   ＋状态」，不是第 5 层的 policy／capability 判定引用；它不能证明合同当前仍存在、policy 未被撤销，也不能
   防止生产方自报状态。若将来要求独立可验证的授权证据，走选项 (b)：新增
   `ProductIntentGrant { intent_id, intent_version, contract_id, contract_version, decision_ref }` 输入面，
   由装配方＋第 5 层填，协调者收——本轮不做，也不新增边。
7. **`continuum-runtime` 的 `ALLOWED` 行会增枚**（`:403` 起）：它是装配方，P7 的新 crate 由它注册迁移、
   实现端口。**既有行不因 P7 增枚**，唯一例外是这一行；`("continuum-core", &[])`（`:27`）保持为空。
8. **P6A 的 `continuum-context` 不是 P7 的上游**：它今天不在 workspace 里（见 §七第 10 条），P7 也不消费
   `ContextPackage`。P7 的四块不得为「将来可能要」预登记这条边。
9. **P6D 的合并门会被 P7 触发**：P7 的新 crate 让 `p6_union_adds_only_its_own_rows`（`:784`）变红，
   这是设计预期的到期日；处置见 §四 第 1 条与 §七第 11 条。**P7 不得在 P6 的 fixture 上追加行**。

---

## 四、六条横切约束（四份设计都必须遵守）

1. **不新增反向边，且可观察**：判据是 `dependency_direction.rs` 的 `ALLOWED` 逐对断言（`:26`）与
   `every_crate_depends_only_on_its_allowed_set`（`:494`），名单由 `workspace_crates()`（`:440`）派生。
   **四块各自在 `ALLOWED` 里加自己新建 crate 的那一行；并集的复核具名给 P7D**。
   既有行的允许集合不因 P7 增枚，唯一例外是装配方 `continuum-runtime`；`("continuum-core", &[])` 保持为空。
   **P6D 的合并门要由 P7D 重新基线化或退役**（P6D 设计 §7.3.5）：新 fixture 或从 `cargo metadata` 生成，
   **不允许**在 `p6_pre_allowed.txt` 上追加行。实测 P7 首先撞红的是断言 5（runtime 行增量 ⊆ `P6_NAMES`）
   与断言 8（新增 subject 不在 `P6_NAMES`），不是 P6D 设计写的断言 1（见 §七第 11 条）。**若选重基线化，
   除 fixture／`P6_EXPECTED_ROWS`／断言 2 计数外，还必须同批改解析器的 `rows.len() == 27`（`:615`）
   与断言 1 的 `baseline.len() == 27`（`:790`）两个字面量**；只改前三处会在解析器或断言 1 先红。
   若选退役，则把「P7D 一律退役、不 rebaseline」写成唯一路径。
2. **迁移号段**：本仓口径是「一个子项目一个十位档」（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:480`）。
   实测占用：`1`、`2`（`crates/continuum-persist/src/db.rs:29`、`:46`）、`10`（`crates/continuum-artifact/src/persist.rs:9`）、
   `20`（`crates/continuum-graph/src/persist.rs:14`）、`30`（`crates/continuum-workspace/src/persist.rs:21`）、
   `40`（`crates/continuum-effect/src/persist.rs:27`）、`41`（`crates/continuum-policy/src/persist.rs:30`）、
   `50`（`crates/continuum-capability/src/persist.rs:53`）、`80`（`crates/continuum-model-registry/src/persist.rs:106`）、
   `100`（`crates/continuum-canonical/src/persist.rs:83`）、`110`（`crates/continuum-semantics/src/persist.rs:207`）、
   `120`（`crates/continuum-budget/src/persist.rs:60`）、`130`（`crates/continuum-verify/src/persist.rs:64`）、
   `190`（`crates/continuum-memory/src/persist.rs:13`）。**P7 取 `200–209` 这一档**（P6 已取 `190–199`；
   `60`／`70`／`90` 是 P3 各子项目的空档，不许挪用）。**P7A 确定落库并占 `200`**；P7D 若落会话／后台作业表，
   在同一档内顺取 `201`；P7B／P7C 若无表则不占号。每个落库块在落地时**现场核对空号**，并同步改三处：
   `runtime_migrations()`（`main.rs:114`）、`expected_migrations()`（`tests/migrations.rs:37`）、
   `tests/startup.rs` 的三处计数（`:26`、`:81`、`:94`）。**190 的 SQL 此后不得再改**（同 110／130）。
3. **同一件事两个词汇表**：§2.4 的七组与 §6.2 第 2–4 条是本轮的高危处，逐组定「只许用哪一个」：
   - ProductReadiness **用 §232 的九个名字**（`functional` 等）。§153 的 `functional_completeness` 与
     §232 的 `functional` 是同一维的两种叫法，规范未定版本先后；**以 §232 为准**，§153 **不建第二套枚举**，
     两者顺序不同也以 §232 为准。
   - 资源优先级**用 §300 的四档**（`User workload` …），§79（`docs/spec/02-positioning.md:655-669`）的四档
     是解释面，不建第二套 `Priority`。
   - `workload_class` **用 §13 的四档进程类**；输出示例里的 `INTERACTIVE` 不是第五档，按笔误处理。
   - `Suspended` 的三处所指不同（§2.4）；P7C 的挂起状态机**不叫 `NodeState`**，也不与 `IntentStatus` 合并。
   - §299 的四类来源、`SynthesisTrigger` 的四个时机、§79 的四档优先级**是三个枚举**，不得合并。
4. **工具结果消费与 Shippable 三值**：P7 的 readiness／shippable／progress **只消费** `continuum_verify::Evidence` 与
   `CompletionVerdict`，**不消费**原始工具输出、模型自述或 `Claim` 文本。后台扫描发现（regression 等）
   若要走「生成 Intent」这条路，必须先按 §89 转成 `Evidence`，再进入 P7D；P7 不定义第二枚 Evidence、
   第二套「充分」判据或第二个完成判定。**`CompletionVerdict` 今天只有 `Blocked`／`Undetermined` 两臂**（§七第 7 条），
   没有「完成」正臂，故 Shippable 采用**三值 fail-closed**：`ShippableVerdict` = `Shippable`／`NotShippable`／
   `Undetermined`；**`Shippable` 只在 `ShippablePredicate`＋readiness 成立、且 Completion 不处于
   `Blocked`／`Undetermined` 时给出**。`Undetermined` 必须落到 `Undetermined` 或 `NotShippable`，**不能当作许可**。
   `ShippablePredicate` 是输入条件，`ShippableVerdict` 是判定结果，两个词不混用；许可面的规范缺口见 §六 6.2 第 7 条。
5. **依赖边只登记实际用到的**：`ALLOWED` 与实际依赖**精确一致**，断言是逐对 `assert_eq!`。新增 workspace
   成员会先让那道门变红，这是刻意的——不要靠「先不加 member」绕过。P7 的四块只登记自己的行；
   条件边（§三 表）用了才登记，不用不登记。
6. **两对块共享一个未冻结的接口，故那两对不能全并行**：
   - **P7B → P7D**：P7D 的 Session Orchestrator 要消费 P7B 的 Intent Scheduler 决策面（「现在推进哪个 Intent」）。
     **处置：P7B 开工第一件事冻结决策面（一页），P7D 对着它写**。若协调者不接受这处串行，把 P7D **并进 P7B**
     ——这样 Supervisor 与 Session 仍只依赖 P7C，不成环。
   - **P7C → P7D**：Supervisor 要消费 Arbiter 的结论与四档动作。**处置：P7C 先冻一页，P7D 对着它写**。
     若并块，只能把 P7D 并进 P7C 以外的方向；**不能把 P7C 并进 P7B／P7D**，那会让 Resource Arbiter 的 crate
     获得 workspace 依赖，§7.3 的「入度为零」与判据 3 的可观察性同时消失。
   - **P7A 与三块都无未冻接口，但有一条按值交接**：Product 面只在 P7A 内部；P7A **确定**按 §152 的 lower
     产出 Product Intent，P7D 的 `product drive` 按值消费它——这是语义交接，不是 crate 边（§三第 6 条），
     也不要求 P7A 等 P7D。`Intent` 形状由 §222 冻结，故 **A 可全并行**；P7D 的设计只需把 `status == READY` 的
     Product Intent 当输入（授权证据见 §三第 6 条）。**运行时的 `product drive` 以 P7A 的实际产出为前置**，这是值的前置，不是设计串行。
   - **P7B 与 P7C 之间无未冻接口，设计可全并行**：两块唯一的共享词汇是 §13 的四档 `workload_class` 与
     §300 的压力（均已冻结）；`BackgroundSuitability` 的投影只在装配方实现一次（§2.4），P7B 不解析
     `workload_class`，P7C 不依赖 P7B；运行时由装配方在 P7C 结论之后接线 `ResourceDirectivePort`——
     **端口实现消费 P7C 的结论，P7B 不直接消费 P7C**（§三第 5 条）。

---

## 五、四块的范围与不做

**为什么是四块，而不是三块或八块**：

1. **Product／ProductReadiness／Milestone／Progress Gate／Shippable 必须一块（P7A）**：五者共享
   `ProductReadiness` 与 `milestone_history`；拆开会出现两个 readiness 读者、两个 shippable 判定。
2. **Shippable 不跟 Supervisor／Session 走**：它的输入是 `ProductReadiness` 与第 8 层 Completion；
   放进后台块会让 P7D 持有产品状态，并把「可发布」与「后台还在跑」搅在一起。
3. **三层调度单独一块（P7B）**：只新建 Intent Scheduler 与具名资源端口；Graph Scheduler 已有实现，
   Resource Scheduler 只有第 4 层碎片、P7 不重造。合进产品块会被读成「P7 重造执行层」。
4. **Resource Arbiter 单独一块（P7C）**：判据 3 要求模型层完全不可用时仍能工作；只有独立 crate 的
   `ALLOWED == []` 能把这条做成可观察事实。合进 P7B／P7D 会让它获得 workspace 依赖。
5. **Background Supervisor 与 Session Orchestrator 一块（P7D）**：两者都消费「现在推进哪个 Intent」；
   分开会各写一套排名。它们对 P7B／P7C 是消费方，故排在两块之后。

| 块 | 一句话范围 | 主要规范依据 | 拥有的路径／接口 | 明确不做 |
|---|---|---|---|---|
| **P7A** | 建产品循环：§150 `EngineeringBaseline` → Product／ProductReadiness／Milestone 惰性生成／Milestone→Product Intent lower／Progress Gate／Shippable 与 post-shippable 的区分 | §150 §231 §232 §233 §234 §335 §336 §337 §151–§155 §156；《工程》§7.1（`:424-427`）、§7.2（`:435`、`:439`）、§7.4 判据 1／2／4 | 新建 `continuum-product`；迁移 `200`；`EngineeringBaseline` 的生成与 `Product.baseline`；`Product`／`ProductReadiness`／`Milestone`／`ProgressGateVerdict`／`ShippableVerdict` 各一份；`ContinuousProductDrive` 落点（设计定） | 不生成完整未来 Milestone 列表；不定义验证／证据；不执行 replan（只出 Decision；消费方见 §6.2 第 13 条）；不在 P5a 之外造「完成」正臂；不自造九维聚合或阈值；不新造 `ProductId`；baseline 缺失或不可判时不得默认通过 |
| **P7B** | 建 Intent Scheduler，并冻结一个由装配方实现的具名资源端口，把「三层调度必须分离」做成可观察结构；Graph Scheduler 复用，Resource Scheduler 不重造 | §301 §302 §303 §304 §77；《工程》§7.1（`:431`）、§7.2（`:435`） | 新建 `continuum-intent-scheduler`；Intent Scheduler 决策面一页；`ResourceDirectivePort` 一页（实现方消费 P7C 结论） | 不重写 Graph Scheduler（`continuum-graph` 已有）；不实现 Resource Scheduler 的 router／placement／工具选择／并行度（第 4 层只有碎片）；不接管节点状态迁移；不定义第二个「最值得继续」排名（P7D 复用本块的）；不直接消费 P7C；**不在 P7 内宣称 §304 已闭合** |
| **P7C** | 建独立于模型的资源仲裁：压力／负载类别、四档优先级、协作式挂起（含 `Checkpointing` 两阶段与失败回退）与最小暴露面 | §300 §13 §14；《工程》§7.1（`:429`）、§7.2（`:441`）、§7.4 判据 3；A14／OPEN-012 | 新建 `continuum-arbiter`，**workspace 内零依赖**；仲裁结论面一页（含 `SuspensionPhase` 与 checkpoint 失败结果） | 不依赖任何模型／上下文／执行层 crate；不终止进程作为默认让路；不把进程名／PID／路径放进公开类型；不定义后台任务来源；不接管节点状态迁移 |
| **P7D** | 建后台治理与会话启动：§299 四类来源的 Background Supervisor、Manual／Auto 的 Session Orchestrator、§171 触发点、`product drive` 的按值消费 | §299 §12 §11 §171 §156；《工程》§7.1（`:428`、`:430`）、§7.3（`:453-454`） | 新建 `continuum-supervisor`；`TaskSource` 四值与 `AuthorizedTask` 输入面；`SessionPlan`；`continuum-memory` 触发边；Supervisor 结果摘要面；P6D 合并门复核 | 不持产品状态；不定义第五类任务来源；不让模型自行创建无界后台任务；不设 §171 阈值（P10）；不实现记忆合成作业本体（P6C）；不实现执行器／UI |

### P7A 的完成判据对应

§7.4 的四条里，三条落在 P7A：判据 1（Milestone 逐个生成、无完整列表）、判据 2（无进展触发 replan 并生成 Decision）、
判据 4（Shippable 与 post-shippable 不互相覆盖）。**判据 2 只有「生成 Decision」这半由 P7A 交出**：P7A 生成
`continuum_semantics::Decision` 并落 `decision.required` 事件，**不执行 replan**。replan 的消费方是
**第 3 层 Planner**（《工程》§3.1 `:158` 的 receding-horizon Planner 接口）——该接口今天**无实现**，P4 明确
不建规划，现行 P8 范围也没有「消费 Decision 并 replan」这一项；故按 §6.2 第 13 条具名缺口处理，收件人：
第 3 层持有方＋协调者（若 P8 不收，就在 P8 范围新增一项）。**判据 2 的「replan」在 P7 内不闭合，P7A 不假装闭合。**
**判据 1 与判据 4 都不依赖 OPEN-010／OPEN-015 的关闭**：前者只测「不存在完整预定义列表」，后者只测两种状态
可区分且不互相覆盖；OPEN-015 影响的是「下一批的边界」，不是「是否惰性生成」。

### P7B 的完成判据（§7.4 没有，必须自报）

§7.4 只覆盖 P7A 与 P7C，**P7B 与 P7D 没有规范判据**。P7B 的设计必须自报一条可观察判据，建议形状：
「三个决策面各有独立入口／类型，且不存在一个函数同时产出 Intent、Graph、Resource 三者」——
这是 §301 的 MUST 的结构照片。**Intent Scheduler 的模型判断经端口**，不直接调 `ModelProvider`；
其决策面必须带 `NoIntent` 臂，且 `ResourceDirectivePort` 只是端口、不是第二个 Resource Scheduler。
**P7B 结束时不宣称 §304 已闭合**。

### P7C 的完成判据对应

判据 3（模型层完全不可用仍能限制后台负载）由 P7C 的 **workspace 内零依赖** 与「无模型调用路径」两条共同兑现。
P7C 的 `ALLOWED` 行必须是 `&[]`（同 P6D 的 `continuum-protected` 先例，`dependency_direction.rs:322`）；
**若 P7C 需要任何 workspace 内 crate，判据 3 就不再可观察**，该边必须先报协调者。
P7C 的结论面按 §2.3.2 第 2 条冻结，含 `Checkpointing` 两阶段与失败回退（失败返回错误并回
`Throttled`，不落 `Suspended`）；`workload_class`＋`resource_pressure` 是
P7B `BackgroundSuitability` 的唯一投影源，投影只在装配方实现；`ResourceDirectivePort` 的实现消费
`resource_pressure` 与 §300 的四档优先级（§三第 5 条），P7B 不直接消费 P7C。四档优先级的可见面只对
装配方，不进 AI／后台暴露面。

### P7D 的完成判据（§7.4 没有，必须自报）

P7D 的设计必须自报至少三条：
1. **任务来源闭集**：`TaskSource` 恰四值，且没有从模型输出构造第五类的入口（§299 的类型照片）；
   裸 `TaskSource` 不能构造 `AuthorizedTask`；`product drive` 的证据按 §三第 6 条的选项 (a) 判定，
   `status` 必须为 `READY`，缺失或来源不匹配时拒绝任务；
2. **会话选择不另造排名**：Auto 模式的「最值得继续」调 P7B 的 Intent Scheduler 决策面（含 `NoIntent`），
   Manual 模式只做候选展示；两模式共用同一份 `SessionPlan`；
3. **触发不越界**：§171 直接调 P6C（`continuum-memory` 是确定边），P7D 不持阈值、不改 `SynthesisTrigger` 类型。

**§299 的 `product drive` 来源在 P7D 里读作「`status == READY`（证据按 §三第 6 条判定）的 Product Intent」**（`Intent.kind = Product`，
`crates/continuum-semantics/src/intent.rs:40`）。它的生产者是 **P7A**：§152 的 Milestone→Intent lower 确定产出
Product Intent（`product_id: ProductIdRef`，其余字段照 §222 的 `Intent`，状态与 `completion_predicate` 按该类型），
P7D 按值消费，不登记 `continuum-product` 边。交接约定：P7A 只交 `status == READY` 的 Product Intent 值或 id；
授权证据按 §三第 6 条的选项 (a) 取 `contract_id`＋`status`，不另加边；P7D 不读 §231 的 Product 记录。
若设计发现必须读 Product 记录或合同内容才能判定授权，那是**一条要报协调者的新边**，
不在本文预登记。§156 的「后台继续低风险 Milestone」就是这一路。

---

## 六、缺口与未决（含与 P8／P9／P10 的关系）

### 6.1 §7.5 的两处（本文原样承接，不关闭）

| 编号 | 内容 | 收件人 | P7 如何在不关闭的前提下落地 |
|---|---|---|---|
| **A15 / OPEN-010** | §232 九维的**聚合函数**与 §337 `progress_delta` 的**阈值**未定义 | 规范维护者 ＋ P10（主树已列入 `p8-p10-scope.md:96`） | P7A 只存九维的 `value/evidence/last_verified`，把聚合与阈值做成**调用方申报的策略输入**；Progress Gate 出三值（触发／未触发／**不可判**）。没有申报策略时只能是「不可判」，**不得**自行 replan。判据 2 用一份申报策略可测；规范缺口仍在 |
| **OPEN-015** | §233 有限视野与 §152 并行／汇合的**批次边界**未定义 | 规范维护者 ＋ P10（主树已列入 `p8-p10-scope.md:97`） | P7A 的生成器只出**一个有界批次**，批次边界做成显式申报参数（依赖前沿／时间盒等由设计定，不写死为规范默认）；测试只钉「不出现完整列表」与「评估后再生成」，不钉批次大小 |

### 6.2 本轮新登记的缺口

| # | 缺口 | 收件人 | 可落地子集 |
|---|---|---|---|
| 1 | **A14 / OPEN-012**：Resource Arbiter 的最小暴露面只有 SHOULD，强制点在第 5 层（`docs/02-工程.md:345`；`docs/01-总纲.md:1698`、`:1881`） | 规范维护者 ＋ P9（边界；主树已列入 `p8-p10-scope.md:63`，接缝 `:68`） | P7C 对 AI／后台的公开结论类型**只有**压力＋负载类别两项；**§300 的四档优先级只对装配方（端口实现）可见，不进 AI／后台暴露面**；原始进程信息私有，公开类型的 `Debug`／序列化面也不得带原始信息。规范级「SHOULD 是否够」不关闭 |
| 2 | **§232 与 §153 的九维名字／顺序不一致**（`functional` vs `functional_completeness`） | 规范维护者 | P7A 用 §232；§153 **不建第二套枚举**（两者只是同一维的两种叫法，规范未定版本先后；顺序不同也以 §232 为准） |
| 3 | **§300 四档与 §79 四档是两张表**（`User workload` vs `Interactive`） | 规范维护者 | P7C 用 §300；§79 只作解释 |
| 4 | **§13 的四档进程类与输出示例 `INTERACTIVE` 不一致**（`docs/spec/01-concepts.md:708-722`） | 规范维护者 | P7C 只实现四档；`INTERACTIVE` 不设变体 |
| 5 | **`continuum-semantics` 没有「按状态列 Intent」的公开查询**：只有 `load_intent`（`:428`）与 `list_children`（`:448`） | P4 语义层持有方 ＋ 协调者 | P7B／P7D 把候选集作为 `&[Intent]` 输入；**不直读 `intent` 表**，不在 P7 里造第二套 Intent 查询 |
| 6 | **`continuum-verify` 的 Evidence 只写不读**：无 `load_evidence`，`evidence.rs:117` 明写「消费者未建」 | P5a 持有方 ＋ 协调者 | P7A 的 readiness 评估收 `&[Evidence]` 值，落库只存 `EvidenceId`；**不直读 `evidence` 表** |
| 7 | **`CompletionVerdict` 没有「完成」正臂**：只有 `Blocked`／`Undetermined`（`completion.rs:24`），第 2 项恒不可判（`:113-116`）；A1／OPEN-001 仍开 | P5a ＋ P9（ENG-004） | P7A 的 `ShippableVerdict` 用**三值 fail-closed**：`Shippable` 只在 Predicate＋readiness 成立且 Completion ∉ {`Blocked`,`Undetermined`} 时给出；Completion 的 `Undetermined` 一律落 `Undetermined`／`NotShippable`，不得当许可；判据 4 只测三态互不覆盖 |
| 8 | **§302 的五个输入里只有 `priority` 在 `Intent` 上，且它是不透明值**；`deadline`／`blocking`／`user focus`／`background suitability` 无落点 | 规范维护者 ＋ 协调者 | P7B 把五个信号做成**显式输入**；不解析 `Intent.priority`，不改 `intent` 表加列 |
| 9 | **§7.4 的判据不覆盖 P7B／P7D**（四条分别落在 A／A／C／A） | 协调者 ＋ 两块设计者 | P7B／P7D 在各自设计里自报判据（§五各自的完成判据段） |
| 10 | **§171 的阈值不属于 P7**：P6C 把「空闲时长、状态改变数量、检查频率、重试与恢复」判给 P7（`docs/superpowers/p6-followups.md:59`）；主树 `p8-p10-scope.md:95` 已把「阈值／频率／重试／触发策略」判给 P10、把触发点判给 P7D，`:105` 的前置是「P7 的调度框架」 | P10 ＋ 协调者 | **触发策略／阈值／频率／重试（P10）与触发点／调用面（P7D）分开**：P7D 直接依赖 `continuum-memory` 提供触发点，不持阈值；两处口径以本条为准 |
| 11 | **P6A 的 `continuum-context` 今天不在 workspace**：`p6-followups.md:21` 与主树 `p8-p10-scope.md:32` 都已写「实现中」（`:32` 并注明不得把期望行当成实际状态）；实测 crate 不存在（`ls crates` 零命中，`dependency_direction.rs:554` 只是期望行） | 协调者 ＋ P8 | P7 不依赖它；P8 的前置要按实际状态复核，不得把期望行当成实际存在 |
| 12 | **第 4 层没有完整的 Resource Scheduler**：只有模型路由（`crates/continuum-model-registry/src/router.rs:199`／`:405`，`rank` `:523`）与节点放置（`crates/continuum-node/src/placement.rs:362`，`src/` 零生产调用方）；缺「工具选择」「并行度决策」与 `place` 的装配点；`ExecutionProfile.compute_node` 仍是裸 `Option<String>`（`:357-358`） | **P3／第 4 层持有方 ＋ 协调者** | P7B 只冻结**具名 `ResourceDirectivePort`**（P7B 定义、装配方实现、内部转调 `rank`／`place`／工具路径），不重造第 4 层；**P7 结束时 §304 仍不完整**，不在 P7 验收里宣称三层调度已闭合 |
| 13 | **replan 的消费方无实现**：第 3 层 Planner 接口（《工程》§3.1 `:158`）在代码里无实现；P4 明确不建规划；现行 P8 范围没有「消费 Decision 并 replan」项 | **第 3 层持有方 ＋ 协调者（P8 范围）** | P7A 只出 `Decision` 并落 `decision.required`；replan 由第 3 层 Planner 消费，或在 P8 新增范围项。P7A 不执行 replan，§7.4 判据 2 在 P7 内只有「出 Decision」这半闭合 |

### 6.3 与 P9／P10 的归属（主树已落实）

主树 `docs/superpowers/p8-p10-scope.md` 已按第一轮建议落实（`b5e987b`）；以下行号按主树，P7 侧不再把这三项列为待协调：

- **P9（边界与验证）**：**A14／OPEN-012** 已列入 P9 表（`:63`），前置写明「A14 还需要 P7C 的 Arbiter 暴露面」
  （`:68`）。P7C 只交结论面（对 AI／后台是压力＋负载类别，对装配方另有 §300 四档优先级），规范强度的收口在 P9；两侧接缝已由 `:68` 写明。
- **P10（记忆与长期演化）**：**A15／OPEN-010** 与 **OPEN-015** 已列入 P10 表（`:96-97`）；§171 已拆成
  「阈值／频率／重试／触发策略（P10）」与「触发点／调用面（P7D）」（`:95`）。P10 的前置写「P7 的调度框架」
  （`:105`），三项都落在那套框架之上；P7A 只交申报策略输入，P7D 只交触发点。
- **P8**：不自动承接 P7 的未决项。P7 的模型判断端口由装配方实现，P8 的执行器/Provider Adapter 是它的下游；
  P6A 状态在主树 `:32` 已写「实现中」，但 crate 仍不存在（§6.2 第 11 条），P8 的前置要按实测复核，与 P7 无关。
  唯一待协调的是 6.4 第 5 条：replan 消费方是否作为 P8 的新范围项。
- **P7 的调度是 §171 Dreaming 的触发点**：P10 依赖 P7 的调度框架，P7 不反向依赖 P10。
  `SynthesisTrigger` 的四个时机由 P6C 冻结（`synthesis.rs:64`），P7D 只决定「何时调」，不改类型。

### 6.4 主树已落实与唯一待协调

主树 `docs/superpowers/p8-p10-scope.md` 已落实前四项（行号按主树 `b5e987b`）；P7 的验收不再因这四项保留「未关闭」，只剩第 5 项：

1. **已落实：P10 表补 A15／OPEN-010 与 OPEN-015**——`:96-97`。
2. **已落实：P9 表补 A14／OPEN-012**——`:63`；P9 的前置接缝见 `:68`。
3. **已落实：P6A 状态**——`:32` 已写「实现中」，并注明不得把期望行当成实际状态；§6.2 第 11 条只保留「crate 仍不存在」这一实测。
4. **已落实：拆 §171 的触发措辞**——`:95` 把「阈值／频率／重试／触发策略」归 P10、「触发点」归 P7D；
   P10 的前置 `:105` 写「P7 的调度框架」。
5. **待协调：P8 是否新增「消费 Decision 并 replan」范围项**——现行 P8 范围没有这一项；若不新增，replan 就留在
   第 3 层 Planner 的具名缺口里（§6.2 第 13 条）。这是 P7 验收唯一保留的未关闭项。

---

## 七、实测订正

> 行号为 2026-10-18 在 `p7-plan` 工作树（基线 `e4a1ab2`）上用 `grep -n`／`sed -n` 逐条读出；
> 按内容引优先，凡给行号处均写明是哪一枚文件。`p8-p10-scope.md` 与 `p6-followups.md` 的行号按主树
> （`b5e987b`／`636ff54`；本工作树的副本尚未同步这两个提交）；其余文件按本工作树。

| # | 文档／它说 | 实测 | 处置 |
|---|---|---|---|
| 1 | P6 切分引 `runtime_migrations()` 在 `main.rs:98`（P6 切分 `:60`、`:138`） | 实际在 `crates/continuum-runtime/src/main.rs:114` | 本文引 `:114`；后续按函数名引，不按旧行号 |
| 2 | P6 切分引 `ALLOWED` 在 `:25`、`every_crate…` 在 `:288`、`workspace_crates` 在 `:234`；P6A 设计引用例 `:478` | 实际 `ALLOWED` `:26`、`every_crate…` `:494`、`workspace_crates` `:440`；`:478` 落在 `cargo_tree_direct` 内 | 本文用实测行；下一轮引函数名 |
| 3 | P6 台账引 `migrations.rs:35`、`startup.rs:25/:92/:80` | 实际 `expected_migrations` `:37`；计数断言 `:26`／`:94`／`:81` | P7 的落库块按实测行同步四处 |
| 4 | 任务材料把 §77 记在 `docs/spec/04-method.md` | §77 在 `docs/spec/02-positioning.md:616`；`04-method.md` 的节号从 §185 起 | 工程 §7.1 的 `§77` 引法正确，但引文件时要写 `02-positioning.md` |
| 5 | §7.1 的「三层调度」读作三件都待建 | Graph Scheduler 已有：`crates/continuum-graph/src/scheduler.rs:28`（`select_runnable`），`lib.rs:29` 导出；`SchedulerConfig.max_parallel` 默认 1（`:14-17`） | P7B 只建 Intent Scheduler 与具名资源端口；**Graph Scheduler 不重造** |
| 6 | 同上，Resource Scheduler 读作 P7 的活 | **只有碎片**：`continuum-model-registry/src/router.rs:199`（`RoutingRequest`）、`:405`（`RankedExecutionCandidates`）、`continuum-node/src/placement.rs:362`（`place`，`src/` 零生产调用方）；缺工具选择／并行度／`place` 装配点；`continuum-budget` 是记账（`lib.rs:1-68`），不是资源仲裁 | P7B 只冻结 `ResourceDirectivePort`；P7C 与 budget 分离；第 4 层缺口报 P3（§6.2 第 12 条），**P7 结束时 §304 仍不完整** |
| 7 | 工程 §7.3「Shippable ← ProductReadiness ＋ 第 8 层 Completion Predicate」 | `CompletionVerdict` 只有 `Blocked`／`Undetermined`（`crates/continuum-verify/src/completion.rs:24`）；`judge_completion` 的第二项恒不可判（`:113-116`） | `ShippableVerdict` 三值 fail-closed；`Undetermined` 不得当许可；正许可面是 A1／OPEN-001／ENG-004 的缺口（§6.2 第 7 条） |
| 8 | 「ProductReadiness 取第 8 层验证结果」可直接读 | `continuum-verify` 的 Evidence 只写不读：`insert_evidence` 在 `persist.rs:118`，无 `load_evidence`；`evidence.rs:117` 明写「消费者未建」 | P7A 以 `&[Evidence]` 值为输入，落库只存 id；读面缺口报 P5a |
| 9 | Intent 存储可供调度器枚举候选 | `continuum-semantics` 只有 `load_intent`（`persist.rs:428`）与 `list_children`（`:448`），无按状态／全局列表；`Intent.priority` 是不透明 `Priority`（`intent.rs:300`），`product_id` 是不透明 `ProductIdRef`（`:372`、`:464`） | P7 不直读 `intent` 表；候选集作为输入；Product 身份复用 `ProductIdRef` |
| 10 | P8–P10 范围曾把 P6A 的 `continuum-context` 写成完成态（旧 `p8-p10-scope.md:32`） | 主树 `b5e987b` 已改为「实现中」并注明不得把期望行当成实际状态（`:32`）；`ls crates` 仍无 `continuum-context`，`grep -rn "continuum-context" crates/ Cargo.toml` 只命中 `dependency_direction.rs:554` 的期望行；`p6-followups.md:21` 也是「实现中」 | 主树口径已对齐；P7 不依赖它；P8 前置按实测复核 |
| 11 | P6D 设计 §7.3.5 说 P7 会让断言 1（27 行）与断言 8 二选一必红；用例注释（`:781-782`）已订正为只提断言 8 | 实际断言 1 只解析 fixture 并断言 27 行，新增 crate 不改 fixture，**初始树不会红**；先红的是断言 5（runtime 行增量 ⊆ `P6_NAMES`，`:837`）与断言 8（新增 subject，`:882`）；断言 2 的 `len()==5`（`:799`）也要随 P7 重建。**rebaseline 后的树还有解析器 `rows.len()==27`（`:615`）与断言 1 `baseline.len()==27`（`:790`）两个 `27`** | P7D 的 rebaseline 同批改 fixture／`P6_EXPECTED_ROWS`／断言 2 计数／`:615`／`:790` 五处；不得在 P6 fixture 追加行；或明确「一律退役」 |
| 12 | §7.4 的四条判据覆盖本层八组件 | 四条分别落在 P7A（判据 1、2、4）与 P7C（判据 3）；P7B／P7D 无判据 | 两块自报判据（§五末）；报协调者 |
| 13 | 任务材料把 P6C 记作「已占 190–199」 | 实际只占 `190`（`continuum-memory/src/persist.rs:13`），191–199 为空但属 P6 档 | P7 取 200–209，不占 190–199；P6 档内空号留给 P6 后续 |

---

## 八、复审与验收

每份设计由**另一个代理**交叉复审，且复审的任务是**试着推翻**它的断言、并找**漏项**——不是核对其措辞
（同 P3 共享面 §六、P6 切分 §六）。本轮追加五条：

1. **P7C 的 `ALLOWED` 行必须是 `&[]`**，并在真含 P7 全部 crate 的树上跑
   `cargo test -p continuum-runtime --test dependency_direction`；「零依赖」不靠逐字推演。
2. **P7D 的合并门复核要真跑**：先确认 P7 新增 crate 让哪几条断言变红（§七第 11 条），再改 fixture／
   `P6_EXPECTED_ROWS`／断言 2 的计数／`:615`／`:790`；改完在真树上绿；或按「一律退役」路径走。
3. **P7B 的三层分离要有结构照片**：三个决策面各有独立入口／类型，不存在一个函数同时产出三者；
   Intent Scheduler 的模型判断经端口，不在 P7 里直调 `ModelProvider`；决策面有 `NoIntent` 臂，
   `ResourceDirectivePort` 不是第二个 Resource Scheduler。
4. **P7A 的三个接缝要各有一张照片**：`Product.baseline` 来自 §150 的 `EngineeringBaseline`；
   §152 的 Milestone→Product Intent lower 产出 §222 的 `Intent`；Progress Gate 的 replan 只出 `Decision`，
   消费方按 §6.2 第 13 条另行落实。
5. **新增冻结点各有一张照片**：P7D 的 `AuthorizedTask` 对 `product drive` 只按 §三第 6 条的
   `contract_id`＋`status` 规则接受（选项 a），缺失或 `status != READY` 即拒绝；并加一条负例：
   `PLANNING → SUSPENDED → RUNNING` 的 Intent（状态是 `RUNNING`，但从未过 `REVIEW`）必须被拒绝——
   这是「允许集只剩 `READY`」的照片；P7C 的 `Checkpointing` 失败返回错误并回
   `Throttled`，不得停在 `Checkpointing`；`ResourceDirectivePort` 的实现（不是 P7B）消费 P7C 的结论。

**本轮验收**：四份设计文档过审 ＋ 各自的实现计划。**本轮不写实现代码**（设计未批不落实现）。

---

**本文的实测口径**：除 §七 的订正行外，正文引用的规范节号以 `docs/spec/05-normative.md` 为准；
《工程》与《总纲》分别指 `docs/02-工程.md` 与 `docs/01-总纲.md`。零命中类断言已限定语料（`crates/` 与
`Cargo.toml`）并逐处读过命中行；`continuum-context` 的「不存在」以 `ls crates` 与 `grep -rn` 两侧核对。
