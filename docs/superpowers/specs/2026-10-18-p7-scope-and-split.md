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

**P7 的规范范围是《工程》§7 全节**——§7.1 的八行组件表（`docs/02-工程.md:420-431`）、§7.2 的组件边界
（`:433-443`）、§7.3 的层内依赖（`:445-458`）、§7.4 的四条完成判据（`:460-467`）、§7.5 的两处缺口（`:469-473`）。
**阶段名与判据都只在工程侧成立**：《总纲》`:4` 明写「本文不规定实施顺序或里程碑安排」，
故「P7」是工程侧的分解名，不是规范的要求。

《总纲》第 7 章（`docs/01-总纲.md:1127-1322`）是这八行的展开：§7.1 两种 Intent（`:1131`）、§7.2 工程可行性
（`:1143`）、§7.3 双层循环（`:1174`）、§7.4 Milestone 惰性生成（`:1197`）、§7.5 产品状态评估（`:1229`）、
§7.6 无效迭代抑制（`:1244`）、§7.7 可发布状态（`:1262`）、§7.8 后台（`:1275`）、§7.9 资源让路（`:1290`）、
§7.10 会话启动（`:1304`）。**工程 §7 的八行与本文件的四块是同一件事的两种切法**，不是两套组件清单。

**一处必须先写在前面的事实**：§7.1 的「三层调度」**不是三件都从零建**。Graph Scheduler 已在第 3 层落地
（`crates/continuum-graph/src/scheduler.rs:28` 的 `select_runnable`，`crates/continuum-graph/src/lib.rs:29` 导出）；
Resource Scheduler 的主要判定面也已分落在第 4 层（`crates/continuum-model-registry/src/router.rs:199` /
`:405`、`crates/continuum-node/src/placement.rs:362`）。**P7 只建 Intent Scheduler，并把「三层必须分离」做成
可观察的结构**；重复实现 Graph／Resource Scheduler 是本文判定的 **Critical**（§五 P7B、§七第 5、6 条）。

### 1.2 谁拥有什么

| 职责／路径 | 归属 | 规范依据 |
|---|---|---|
| Product 对象、ProductReadiness 九维及其 `value/evidence/last_verified` | **P7A** | §231 §232 §153；《工程》§7.1（`:424`）、§7.2（`:435`）、§7.3（`:448-449`） |
| Milestone 数据模型、惰性生成与批次 | **P7A** | §233 §234 §151 §152；《工程》§7.1（`:425`）、§7.2（`:437`）、§7.3（`:450`） |
| Progress Gate、`progress_delta`、无进展时触发 replan／Decision | **P7A** | §337 §154；《工程》§7.1（`:426`）、§7.2（`:439`）、§7.3（`:451`）、§7.4 判据 2（`:464`） |
| Shippable Predicate、Shippable 与 post-shippable 的区分 | **P7A** | §335 §336 §155；《工程》§7.1（`:427`）、§7.3（`:455`）、§7.4 判据 4（`:466`） |
| Intent Scheduler 本体；§301 的三层分离结构 | **P7B** | §301 §302 §77；《工程》§7.1（`:431`）、§7.2（`:435`） |
| Graph Scheduler 的**消费**（复用，不重造） | **P7B** | §303；`crates/continuum-graph/src/scheduler.rs:28` |
| Resource Scheduler 的**调用边界**（不重造；实现在第 4 层） | **P7B** | §304；《工程》§7.2（`:435`） |
| Resource Arbiter：资源压力、负载类别、四档优先级、协作式挂起、最小暴露面 | **P7C** | §300 §13 §14；《工程》§7.1（`:429`）、§7.2（`:441`）、§7.3（`:452`）、§7.4 判据 3（`:465`） |
| Background Supervisor：§299 的四类任务来源与后台任务治理 | **P7D** | §299 §12；《工程》§7.1（`:428`）、§7.2（`:443`）、§7.3（`:453`） |
| Session Orchestrator：Manual／Auto 两种模式与 Session Plan | **P7D** | §11；《工程》§7.1（`:430`）、§7.3（`:454`） |
| §171 Dreaming 的**触发点**（不持阈值） | **P7D** | §171；P6C 的 `begin_synthesis`／`fail_synthesis`／`apply_synthesis`（`crates/continuum-memory/src/synthesis.rs:108`／`:217`／`:306`） |
| Product 的身份：与 `Intent.product_id` 同一枚 `ProductIdRef`，不另造 `ProductId` | **P7A** 消费，**不新建** | §231；`crates/continuum-semantics/src/intent.rs:372`、`:463` |
| P6D 合并门的重新基线化或退役 | **P7D**（协调者收） | P6D 设计 §7.3.5；`crates/continuum-runtime/tests/dependency_direction.rs:784` |

**新建 crate 的具体名字由各块自己的设计定**，本文只登记「这一块拥有它」；四块之间不得出现两枚同义 crate。
本文建议名：P7A `continuum-product`、P7B `continuum-scheduling`、P7C `continuum-arbiter`、
P7D `continuum-supervisor`（Session Orchestrator 与 Supervisor 同 crate；若拆，拆出的第二枚仍归 P7D，两行都登记）。

### 1.3 共写文件

| 路径 | 谁改 |
|---|---|
| `Cargo.toml`（`[workspace] members`） | **共写**：每块加自己那一行 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | **共写**：每块加自己新建 crate 的那一行；**并集的复核具名给 P7D**（理由见 §四 第 1 条） |
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`） | **共写**：只有落库的块加自己那一行（**号段见 §四 第 2 条**） |
| `crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`） | **共写**：与上一行同批、同一块改；两份清单不得分叉 |
| `crates/continuum-runtime/tests/startup.rs` 的三处计数（`:26`、`:81`、`:94`） | **共写**：与上两行同批、同一块改（P6C 曾漏这里，见 `docs/superpowers/p6-followups.md:36-38`） |
| `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 与 `P6_EXPECTED_ROWS`（`:551`） | **P7D 一处改**：重新基线化或退役，**不得在 P6 的 fixture 上追加行**（P6D 设计 §7.3.5） |

---

## 二、接口冻结现状

### 2.1 规范已给形状、本轮照抄、不新冻

四份设计**不得改名、不得另造第二套词汇**。以下名字由规范给定：

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
| `Intent` 十五字段、`IntentStatus` 十三态、`IntentId`、`ProductIdRef`、`Priority` | `crates/continuum-semantics/src/intent.rs:434`、`:61`、`:372`、`:300`；`crates/continuum-semantics/src/ids.rs:41` | P7 只读不造第二份；`Product.id` 必须与 `ProductIdRef` 同一词汇 |
| `Decision` 与 `create_decision`（写 `decision.required`） | `crates/continuum-semantics/src/decision.rs:80`、`crates/continuum-semantics/src/persist.rs:1173` | P7A 的 Progress Gate 只经它出 Decision；不自造「决策单」 |
| `Evidence`、`CompletionVerdict`、`judge_completion` | `crates/continuum-verify/src/evidence.rs:222`、`completion.rs:24`、`:104` | P7A 的 readiness／shippable 只消费它们；不得另造 Evidence 或「完成判定」 |
| Graph Scheduler：`select_runnable`／`SchedulerConfig` | `crates/continuum-graph/src/scheduler.rs:28`、`:9` | P7B 只调用；不重写 READY／并发判定 |
| Resource Scheduler 的既有面：`RoutingRequest`／`RankedExecutionCandidates`、`place` | `crates/continuum-model-registry/src/router.rs:199`、`:405`、`crates/continuum-node/src/placement.rs:362` | P7 不拥有这些类型，不在 P7 里再造 router／placement |
| 预算五维与账本（`DimensionKind`／`Dimensions`／`check`） | `crates/continuum-budget/src/lib.rs:1-68`、`dimensions.rs` | 它是**记账**，不是 Resource Arbiter；P7C 不得拿它当资源仲裁 |

### 2.3 本轮要新冻的：只冻块之间必须共用的

**不新冻规范已给的东西**（§2.1），**也不新冻各块自己就能闭合的东西**。只冻以下五处：

1. **P7A 的 Product 面**：`Product`／`ProductReadiness`／`Milestone`／`ProgressGateVerdict`／`ShippableVerdict`
   **各一份**，由 P7A 定义；字段名照 §231／§232／§234，块设计只补规范没给的部分。`Product.id` 与
   `Intent.product_id` 同一枚 `ProductIdRef`，**不新造 `ProductId`**。
2. **P7B 的 Intent Scheduler 决策面**：输入是 `&[Intent]` 加 §302 的**显式信号**（不是从 `Intent` 里猜），
   输出是「推进哪一个 `IntentId`」加一份不透明的依据。**评分算法、权重、模型提示不冻**。
   这一页由 P7B 开工第一件事冻结，P7D 对着它写。
3. **P7C 的仲裁结论面**：对 AI／后台暴露的结论**只有** `resource_pressure` 与 `workload_class` 两项
   （§13 的原话，`docs/spec/01-concepts.md:717-722`）；对 P7D 另有 `run`／`throttle`／`suspend`／`resume`
   四档动作。**原始进程信息（进程名、PID、CPU/GPU 占用）不进公开类型**。
   这一页由 P7C 开工第一件事冻结，P7D 对着它写。
4. **P7D 的 `TaskSource`**：恰 §299 的四值，**不新增第五类**；`SynthesisTrigger` 的四个**时机**
   （`crates/continuum-memory/src/synthesis.rs:64`）与它不是同一枚举，见 §2.4。
5. **P7D 的 Session 面**：`SessionPlan` 与「后台新结果」的摘要面；Manual／Auto 两模式共用同一份形状。
   **模型提示、候选排序不冻**——Auto 的「最值得继续」必须调 P7B 的决策面，不得另写一套排名。

### 2.4 同名异物（**都不是**同一个概念）

| 已有的东西 | 它是什么 | 实测位置 | 处置 |
|---|---|---|---|
| `Product` | §231 的产品对象／`MemoryScope::Product`／`Intent.kind = Product`／`Intent.product_id` | `crates/continuum-memory/src/claim.rs:20`、`crates/continuum-semantics/src/intent.rs:40`、`:463` | P7A 只认 §231；Product 身份用 `ProductIdRef`；**不把 memory scope 当产品** |
| `Suspended` | `NodeState::Suspended`／《总纲》§14 的协作式挂起态／`IntentStatus::Suspended` | `crates/continuum-graph/src/node.rs:63`、`docs/spec/01-concepts.md:741`、`crates/continuum-semantics/src/intent.rs:77` | P7C 自建挂起状态机；**不复用 `NodeState`**，只把「挂起」结果映射到节点的既有迁移 |
| `Decision` | §331 的决策单／策略的 `Allow/Deny/RequireApproval` | `crates/continuum-semantics/src/decision.rs:39-44` | P7A 用前者；若同文件出现后者，一律别名 `PolicyDecision` |
| `priority` | `Intent.priority` 是**不透明** `Priority`／§302 的输入之一／§78 的七因子 | `crates/continuum-semantics/src/intent.rs:300`、`docs/spec/05-normative.md:1950` | P7B **不解析** `Intent.priority`；§302 的五个信号另收显式输入 |
| `workload_class` | §13 的四档进程类／§13 输出示例里的 `INTERACTIVE` | `docs/spec/01-concepts.md:708-715`、`:717-722` | `INTERACTIVE` **不在四档里**；P7C 只用四档，示例按笔误处理并报规范维护者 |
| `TaskSource` | §299 的四类任务来源／`SynthesisTrigger` 的四个时机／§79 的四档交互优先级 | `docs/spec/05-normative.md:1892-1903`、`crates/continuum-memory/src/synthesis.rs:64`、`docs/spec/02-positioning.md:655-669` | 三个四值枚举**不得合并**；P7D 只拥有第一枚，第二枚由 P6C 拥有，第三枚是 §300 的解释面 |

---

## 三、跨层边

1. **§9.1 的层间图与 P7 有关的是四条**（`docs/02-工程.md:569-588`，箭头读作「被依赖者 → 依赖者」）：
   `资源层 (4) → 长期循环 (7)`、`边界层 (5) → 长期循环 (7)`、`认知层 (6) → 长期循环 (7)`、
   `跨领域 (8) → 长期循环 (7)`。故 P7 依赖第 2、4、5、6、8 层是**既有方向**，不是反向边。
2. **P7 的直连边只登记实际用到的**。本文按块给出「确定」与「条件」两栏；条件栏**用了才登记，不用不登记**：

| 块 | 拟名 crate | 确定的上游 | 条件边（设计定稿时按实际使用） | 绝不登记 |
|---|---|---|---|---|
| P7A | `continuum-product` | `continuum-semantics`（`Intent`／`Decision`）、`continuum-verify`（`Evidence`／`CompletionVerdict`）、`continuum-persist`（迁移 200） | `continuum-canonical`（仅当 P7A 自己构造 `Intent`，要 `EntityId`） | `continuum-runtime`、`continuum-provider`、`continuum-model-registry`、`continuum-node` |
| P7B | `continuum-scheduling` | `continuum-semantics`（`Intent`）、`continuum-graph`（`select_runnable`／`SchedulerConfig`） | `continuum-persist`（仅当调度结果落库） | 第 4 层的 router／placement（经端口，见下条第 5 点） |
| P7C | `continuum-arbiter` | **无**（workspace 内零依赖） | 无 | 任何 workspace 内 crate；任何模型／上下文类型 |
| P7D | `continuum-supervisor` | `continuum-scheduling`、`continuum-arbiter`、`continuum-semantics` | `continuum-persist`（仅当会话／后台作业落库）、`continuum-canonical`（仅当自己造 `Intent`）、`continuum-verify`（仅当后台发现以 `Evidence` 形态进入） | `continuum-product`（P7D 不持产品状态）、`continuum-runtime`、`continuum-provider` |

3. **`continuum-verify` 是 P7 的「验证结果」上游**：§232 的 `evidence` 与 §7.3 的「第 8 层验证结果」都指它。
   P7A **不把模型自述、原始工具输出或 `Claim` 当证据**；只认 `Evidence` 与 `CompletionVerdict`（§四 第 4 条）。
4. **`continuum-graph` 是 P7 的「执行层既有件」**：P7B 消费 `select_runnable`，不接管 `Queued → Running`、
   不接管节点状态迁移。**这条是第 3 层 → 第 7 层的直连边**，不在 §9.1 的相邻层链上，但不成环，
   且与 P5 分块「只消费第 3 层既有判据」同形；P7B 的设计要写明这一句。
5. **模型调用与资源放置一律经端口，P7 不建第二条调用路径**：P3G 的模型调用路径在
   `crates/continuum-runtime/src/model_call.rs:430`（`call`），P7 的 crate **不得**直接依赖
   `continuum-runtime`／`continuum-provider`，也不得直接 `ModelProvider::invoke`。P7 各块按需定义
   「判断输入／输出」的端口，由装配方 `continuum-runtime` 实现并转调既有路径。**P7 不冻一个跨四块的
   通用模型端口**——三个使用点（Milestone 提议、Intent 排序、Session 摘要）是不同的语义判断，
   共用的只是底层调用路径，而那条路径不在 P7。
6. **`continuum-runtime` 的 `ALLOWED` 行会增枚**（`:403` 起）：它是装配方，P7 的新 crate 由它注册迁移、
   实现端口。**既有行不因 P7 增枚**，唯一例外是这一行；`("continuum-core", &[])`（`:27`）保持为空。
7. **P6A 的 `continuum-context` 不是 P7 的上游**：它今天不在 workspace 里（见 §七第 10 条），P7 也不消费
   `ContextPackage`。P7 的四块不得为「将来可能要」预登记这条边。
8. **P6D 的合并门会被 P7 触发**：P7 的新 crate 让 `p6_union_adds_only_its_own_rows`（`:784`）变红，
   这是设计预期的到期日；处置见 §四 第 1 条与 §七第 11 条。**P7 不得在 P6 的 fixture 上追加行**。

---

## 四、六条横切约束（四份设计都必须遵守）

1. **不新增反向边，且可观察**：判据是 `dependency_direction.rs` 的 `ALLOWED` 逐对断言（`:26`）与
   `every_crate_depends_only_on_its_allowed_set`（`:494`），名单由 `workspace_crates()`（`:440`）派生。
   **四块各自在 `ALLOWED` 里加自己新建 crate 的那一行；并集的复核具名给 P7D**。
   既有行的允许集合不因 P7 增枚，唯一例外是装配方 `continuum-runtime`；`("continuum-core", &[])` 保持为空。
   **P6D 的合并门要由 P7D 重新基线化或退役**（P6D 设计 §7.3.5）：新 fixture 或从 `cargo metadata` 生成，
   **不允许**在 `p6_pre_allowed.txt` 上追加行。实测 P7 首先撞红的是断言 5（runtime 行增量 ⊆ `P6_NAMES`）
   与断言 8（新增 subject 不在 `P6_NAMES`），不是 P6D 设计写的断言 1（见 §七第 11 条）。
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
3. **同一件事两个词汇表**：§2.4 的六组与 §6.2 第 2–4 条是本轮的高危处，逐组定「只许用哪一个」：
   - ProductReadiness **用 §232 的九个名字**（`functional` 等），§153 的 `functional_completeness` 只是
     同一维的旧称（`docs/spec/03-product-drive.md:230-240`），**不建第二套枚举**；两者顺序不同，也以 §232 为准。
   - 资源优先级**用 §300 的四档**（`User workload` …），§79（`docs/spec/02-positioning.md:655-669`）的四档
     是解释面，不建第二套 `Priority`。
   - `workload_class` **用 §13 的四档进程类**；输出示例里的 `INTERACTIVE` 不是第五档，按笔误处理。
   - `Suspended` 的三处所指不同（§2.4）；P7C 的挂起状态机**不叫 `NodeState`**，也不与 `IntentStatus` 合并。
   - §299 的四类来源、`SynthesisTrigger` 的四个时机、§79 的四档优先级**是三个枚举**，不得合并。
4. **工具结果消费**：P7 的 readiness／shippable／progress **只消费** `continuum_verify::Evidence` 与
   `CompletionVerdict`，**不消费**原始工具输出、模型自述或 `Claim` 文本。后台扫描发现（regression 等）
   若要走「生成 Intent」这条路，必须先按 §89 转成 `Evidence`，再进入 P7D；P7 不定义第二枚 Evidence、
   第二套「充分」判据或第二个完成判定。**`CompletionVerdict` 今天只有「不得完成」的两臂**（§七第 7 条），
   故它只能**否决** Shippable，不能单独**许可** Shippable；许可面的缺口见 §六 6.2 第 7 条。
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
   - **P7A 与三块都无未冻接口**：Product 面只在 P7A 内部；P7D 不持产品状态（§三 表）。故 A 可全并行。
     **P7B 与 P7C 之间也无未冻接口**，可全并行。

---

## 五、四块的范围与不做

**为什么是四块，而不是三块或八块**：

1. **Product／ProductReadiness／Milestone／Progress Gate／Shippable 必须一块（P7A）**：五者共享
   `ProductReadiness` 与 `milestone_history`；拆开会出现两个 readiness 读者、两个 shippable 判定。
2. **Shippable 不跟 Supervisor／Session 走**：它的输入是 `ProductReadiness` 与第 8 层 Completion；
   放进后台块会让 P7D 持有产品状态，并把「可发布」与「后台还在跑」搅在一起。
3. **三层调度单独一块（P7B）**：只新建 Intent Scheduler；Graph／Resource Scheduler 已有实现，
   合进产品块会被读成「P7 重造执行层」。
4. **Resource Arbiter 单独一块（P7C）**：判据 3 要求模型层完全不可用时仍能工作；只有独立 crate 的
   `ALLOWED == []` 能把这条做成可观察事实。合进 P7B／P7D 会让它获得 workspace 依赖。
5. **Background Supervisor 与 Session Orchestrator 一块（P7D）**：两者都消费「现在推进哪个 Intent」；
   分开会各写一套排名。它们对 P7B／P7C 是消费方，故排在两块之后。

| 块 | 一句话范围 | 主要规范依据 | 拥有的路径／接口 | 明确不做 |
|---|---|---|---|---|
| **P7A** | 建产品循环：Product／ProductReadiness／Milestone 惰性生成／Progress Gate／Shippable 与 post-shippable 的区分 | §231 §232 §233 §234 §335 §336 §337 §151–§155；《工程》§7.1（`:424-427`）、§7.4 判据 1／2／4 | 新建 `continuum-product`；迁移 `200`；`Product`／`ProductReadiness`／`Milestone`／`ProgressGateVerdict`／`ShippableVerdict` 各一份 | 不生成完整未来 Milestone 列表；不定义验证／证据；不执行 replan（只出 Decision）；不在 P5a 之外造「完成」正臂；不自造九维聚合或阈值；不新造 `ProductId` |
| **P7B** | 建 Intent Scheduler，并把「三层调度必须分离」做成可观察结构；Graph／Resource Scheduler 只消费 | §301 §302 §303 §304 §77；《工程》§7.1（`:431`）、§7.2（`:435`） | 新建 `continuum-scheduling`；Intent Scheduler 决策面一页 | 不重写 Graph Scheduler（`continuum-graph` 已有）；不实现 Resource Scheduler 的 router／placement（第 4 层）；不接管节点状态迁移；不定义第二个「最值得继续」排名（P7D 复用本块的） |
| **P7C** | 建独立于模型的资源仲裁：压力／负载类别、四档优先级、协作式挂起与最小暴露面 | §300 §13 §14；《工程》§7.1（`:429`）、§7.2（`:441`）、§7.4 判据 3；A14／OPEN-012 | 新建 `continuum-arbiter`，**workspace 内零依赖**；仲裁结论面一页 | 不依赖任何模型／上下文／执行层 crate；不终止进程作为默认让路；不把进程名／PID／路径放进公开类型；不定义后台任务来源；不接管节点状态迁移 |
| **P7D** | 建后台治理与会话启动：§299 四类来源的 Background Supervisor、Manual／Auto 的 Session Orchestrator、§171 触发点 | §299 §12 §11 §171；《工程》§7.1（`:428`、`:430`）、§7.3（`:453-454`） | 新建 `continuum-supervisor`；`TaskSource` 四值；`SessionPlan`；Supervisor 结果摘要面；P6D 合并门复核 | 不持产品状态；不定义第五类任务来源；不让模型自行创建无界后台任务；不设 §171 阈值（P10）；不实现记忆合成作业本体（P6C）；不实现执行器／UI |

### P7A 的完成判据对应

§7.4 的四条里，三条落在 P7A：判据 1（Milestone 逐个生成、无完整列表）、判据 2（无进展触发 replan 并生成 Decision）、
判据 4（Shippable 与 post-shippable 不互相覆盖）。**判据 2 的「replan」不由 P7A 执行**：P7A 生成
`continuum_semantics::Decision` 并落 `decision.required` 事件，实际的计划重排由 P4／P8 一侧消费。
**判据 1 与判据 4 都不依赖 OPEN-010／OPEN-015 的关闭**：前者只测「不存在完整预定义列表」，后者只测两种状态
可区分且不互相覆盖；OPEN-015 影响的是「下一批的边界」，不是「是否惰性生成」。

### P7B 的完成判据（§7.4 没有，必须自报）

§7.4 只覆盖 P7A 与 P7C，**P7B 与 P7D 没有规范判据**。P7B 的设计必须自报一条可观察判据，建议形状：
「三个决策面各有独立入口／类型，且不存在一个函数同时产出 Intent、Graph、Resource 三者」——
这是 §301 的 MUST 的结构照片。**Intent Scheduler 的模型判断经端口**，不直接调 `ModelProvider`。

### P7C 的完成判据对应

判据 3（模型层完全不可用仍能限制后台负载）由 P7C 的 **workspace 内零依赖** 与「无模型调用路径」两条共同兑现。
P7C 的 `ALLOWED` 行必须是 `&[]`（同 P6D 的 `continuum-protected` 先例，`dependency_direction.rs:322`）；
**若 P7C 需要任何 workspace 内 crate，判据 3 就不再可观察**，该边必须先报协调者。

### P7D 的完成判据（§7.4 没有，必须自报）

P7D 的设计必须自报至少两条：
1. **任务来源闭集**：`TaskSource` 恰四值，且没有从模型输出构造第五类的入口（§299 的类型照片）；
2. **会话选择不另造排名**：Auto 模式的「最值得继续」调 P7B 的 Intent Scheduler 决策面，Manual 模式只做候选展示；
   两模式共用同一份 `SessionPlan`。

**§299 的 `product drive` 来源在 P7D 里读作「已授权的 Product Intent」**（`Intent.kind = Product`，
`crates/continuum-semantics/src/intent.rs:40`），不是对 P7A 的 Product 对象的依赖；P7D 因此不登记
`continuum-product` 边。若设计发现必须读 §231 的 Product 记录才能判定授权，那是**一条要报协调者的新边**，
不在本文预登记。

---

## 六、缺口与未决（含与 P8／P9／P10 的关系）

### 6.1 §7.5 的两处（本文原样承接，不关闭）

| 编号 | 内容 | 收件人 | P7 如何在不关闭的前提下落地 |
|---|---|---|---|
| **A15 / OPEN-010** | §232 九维的**聚合函数**与 §337 `progress_delta` 的**阈值**未定义 | 规范维护者 ＋ 协调者；建议 P10 收 | P7A 只存九维的 `value/evidence/last_verified`，把聚合与阈值做成**调用方申报的策略输入**；Progress Gate 出三值（触发／未触发／**不可判**）。没有申报策略时只能是「不可判」，**不得**自行 replan。判据 2 用一份申报策略可测；规范缺口仍在 |
| **OPEN-015** | §233 有限视野与 §152 并行／汇合的**批次边界**未定义 | 规范维护者 ＋ 协调者；建议 P10 收 | P7A 的生成器只出**一个有界批次**，批次边界做成显式申报参数（依赖前沿／时间盒等由设计定，不写死为规范默认）；测试只钉「不出现完整列表」与「评估后再生成」，不钉批次大小 |

### 6.2 本轮新登记的缺口

| # | 缺口 | 收件人 | 可落地子集 |
|---|---|---|---|
| 1 | **A14 / OPEN-012**：Resource Arbiter 的最小暴露面只有 SHOULD，强制点在第 5 层（`docs/02-工程.md:345`；`docs/01-总纲.md:1698`、`:1881`） | 规范维护者 ＋ 协调者；建议 **P9**（边界）收 | P7C 的公开结论类型**只有**压力＋负载类别两项，原始进程信息私有；公开类型的 `Debug`／序列化面也不得带原始信息。规范级「SHOULD 是否够」不关闭 |
| 2 | **§232 与 §153 的九维名字／顺序不一致**（`functional` vs `functional_completeness`） | 规范维护者 | P7A 用 §232；§153 按旧称记，不建第二套枚举 |
| 3 | **§300 四档与 §79 四档是两张表**（`User workload` vs `Interactive`） | 规范维护者 | P7C 用 §300；§79 只作解释 |
| 4 | **§13 的四档进程类与输出示例 `INTERACTIVE` 不一致**（`docs/spec/01-concepts.md:708-722`） | 规范维护者 | P7C 只实现四档；`INTERACTIVE` 不设变体 |
| 5 | **`continuum-semantics` 没有「按状态列 Intent」的公开查询**：只有 `load_intent`（`:428`）与 `list_children`（`:448`） | P4 语义层持有方 ＋ 协调者 | P7B／P7D 把候选集作为 `&[Intent]` 输入；**不直读 `intent` 表**，不在 P7 里造第二套 Intent 查询 |
| 6 | **`continuum-verify` 的 Evidence 只写不读**：无 `load_evidence`，`evidence.rs:117` 明写「消费者未建」 | P5a 持有方 ＋ 协调者 | P7A 的 readiness 评估收 `&[Evidence]` 值，落库只存 `EvidenceId`；**不直读 `evidence` 表** |
| 7 | **`CompletionVerdict` 没有「完成」正臂**：只有 `Blocked`／`Undetermined`（`completion.rs:24`），第 2 项恒不可判（`:113-116`）；A1／OPEN-001 仍开 | P5a ＋ P9（ENG-004） | P7A 的 Shippable 用 Completion 只作**否决**；正许可来自产品级 `ShippablePredicate` 与 readiness；判据 4 只测两态区分 |
| 8 | **§302 的五个输入里只有 `priority` 在 `Intent` 上，且它是不透明值**；`deadline`／`blocking`／`user focus`／`background suitability` 无落点 | 规范维护者 ＋ 协调者 | P7B 把五个信号做成**显式输入**；不解析 `Intent.priority`，不改 `intent` 表加列 |
| 9 | **§7.4 的判据不覆盖 P7B／P7D**（四条分别落在 A／A／C／A） | 协调者 ＋ 两块设计者 | P7B／P7D 在各自设计里自报判据（§五末两段） |
| 10 | **§171 的阈值不属于 P7**：P6C 把「空闲时长、状态改变数量、检查频率、重试与恢复」判给 P7（`docs/superpowers/p6-followups.md:55`），P8–P10 又把 §171 判给 P10（`docs/superpowers/p8-p10-scope.md:93`、`:101`） | 协调者 ＋ P10 | P7D 只提供**触发点**：调 `begin_synthesis`／`apply_synthesis`／`fail_synthesis`；阈值、频率、重试归 P10。两处口径以本条为准 |
| 11 | **P6A 的 `continuum-context` 今天不在 workspace**：`p6-followups.md:21` 说「实现中」，`p8-p10-scope.md:32` 说「已合入」；实测 crate 不存在（`ls crates` 零命中，`dependency_direction.rs:554` 只是期望行） | 协调者 ＋ P8 | P7 不依赖它；P8 的前置要按实际状态重述 |

### 6.3 与 P9／P10 的归属（本文的建议）

- **P9（边界与验证）**：建议收 **A14／OPEN-012**（Arbiter 暴露面的规范强度与过滤层）。它是边界／隐私强制问题，
  与 P9 的「边界」主题同族；**现行 P8–P10 范围表没有它**，协调者要加进去，否则它只能留在规范维护者手里。
- **P10（记忆与长期演化）**：建议收 **A15／OPEN-010**、**OPEN-015**、**§171 阈值**。P10 的前置已明写
  「P7 的调度框架」（`p8-p10-scope.md:101`），这三项都落在那套框架之上；**现行 P10 表只列了 §171**，
  另两项要协调者补进范围。
- **P8**：不承接 P7 的未决项。P7 的模型判断端口由装配方实现，P8 的执行器/Provider Adapter 是它的下游；
  P8 的「P6A 已合入」前置是错的（§6.2 第 11 条），与 P7 无关。
- **P7 的调度是 §171 Dreaming 的触发点**：P10 依赖 P7 的调度框架，P7 不反向依赖 P10。
  `SynthesisTrigger` 的四个时机由 P6C 冻结（`synthesis.rs:64`），P7D 只决定「何时调」，不改类型。

---

## 七、实测订正

> 行号为 2026-10-18 在 `p7-plan` 工作树（基线 `e4a1ab2`）上用 `grep -n`／`sed -n` 逐条读出；
> 按内容引优先，凡给行号处均写明是哪一枚文件。

| # | 文档／它说 | 实测 | 处置 |
|---|---|---|---|
| 1 | P6 切分引 `runtime_migrations()` 在 `main.rs:98`（P6 切分 `:60`、`:138`） | 实际在 `crates/continuum-runtime/src/main.rs:114` | 本文引 `:114`；后续按函数名引，不按旧行号 |
| 2 | P6 切分引 `ALLOWED` 在 `:25`、`every_crate…` 在 `:288`、`workspace_crates` 在 `:234`；P6A 设计引用例 `:478` | 实际 `ALLOWED` `:26`、`every_crate…` `:494`、`workspace_crates` `:440`；`:478` 落在 `cargo_tree_direct` 内 | 本文用实测行；下一轮引函数名 |
| 3 | P6 台账引 `migrations.rs:35`、`startup.rs:25/:92/:80` | 实际 `expected_migrations` `:37`；计数断言 `:26`／`:94`／`:81` | P7 的落库块按实测行同步四处 |
| 4 | 任务材料把 §77 记在 `docs/spec/04-method.md` | §77 在 `docs/spec/02-positioning.md:616`；`04-method.md` 的节号从 §185 起 | 工程 §7.1 的 `§77` 引法正确，但引文件时要写 `02-positioning.md` |
| 5 | §7.1 的「三层调度」读作三件都待建 | Graph Scheduler 已有：`crates/continuum-graph/src/scheduler.rs:28`（`select_runnable`），`lib.rs:29` 导出；`SchedulerConfig.max_parallel` 默认 1（`:14-17`） | P7B 只建 Intent Scheduler 与分离结构；**不重造** |
| 6 | 同上，Resource Scheduler 读作 P7 的活 | 既有面：`continuum-model-registry/src/router.rs:199`（`RoutingRequest`）、`:405`（`RankedExecutionCandidates`）、`continuum-node/src/placement.rs:362`（`place`）；`continuum-budget` 是记账（`lib.rs:1-68`），不是资源仲裁 | P7B 只定义调用边界；P7C 与 budget 分离 |
| 7 | 工程 §7.3「Shippable ← ProductReadiness ＋ 第 8 层 Completion Predicate」 | `CompletionVerdict` 只有 `Blocked`／`Undetermined`（`crates/continuum-verify/src/completion.rs:24`）；`judge_completion` 的第二项恒不可判（`:113-116`） | Completion 只能否决 Shippable；正许可面是 A1／OPEN-001／ENG-004 的缺口（§6.2 第 7 条） |
| 8 | 「ProductReadiness 取第 8 层验证结果」可直接读 | `continuum-verify` 的 Evidence 只写不读：`insert_evidence` 在 `persist.rs:118`，无 `load_evidence`；`evidence.rs:117` 明写「消费者未建」 | P7A 以 `&[Evidence]` 值为输入，落库只存 id；读面缺口报 P5a |
| 9 | Intent 存储可供调度器枚举候选 | `continuum-semantics` 只有 `load_intent`（`persist.rs:428`）与 `list_children`（`:448`），无按状态／全局列表；`Intent.priority` 是不透明 `Priority`（`intent.rs:300`），`product_id` 是不透明 `ProductIdRef`（`:372`、`:463`） | P7 不直读 `intent` 表；候选集作为输入；Product 身份复用 `ProductIdRef` |
| 10 | P8–P10 范围说「P6A 的 `continuum-context` 已合入」（`p8-p10-scope.md:32`） | `ls crates` 无 `continuum-context`；`grep -rn "continuum-context" crates/ Cargo.toml` 只命中 `dependency_direction.rs:554` 的期望行；`p6-followups.md:21` 说「实现中」 | 报协调者；P7 不依赖它 |
| 11 | P6D 设计 §7.3.5 说 P7 会让断言 1（27 行）与断言 8 二选一必红；用例注释（`:781-782`）已订正为只提断言 8 | 实际断言 1 只解析 fixture 并断言 27 行，新增 crate 不改 fixture，**不会红**；先红的是断言 5（runtime 行增量 ⊆ `P6_NAMES`，`:837`）与断言 8（新增 subject，`:882`）；断言 2 的 `len()==5`（`:799`）也要随 P7 重建。**用例注释漏了断言 5** | P7D 的 rebaseline 改 fixture／`P6_EXPECTED_ROWS`／断言 2 计数三处；不得在 P6 fixture 追加行 |
| 12 | §7.4 的四条判据覆盖本层八组件 | 四条分别落在 P7A（判据 1、2、4）与 P7C（判据 3）；P7B／P7D 无判据 | 两块自报判据（§五末）；报协调者 |
| 13 | 任务材料把 P6C 记作「已占 190–199」 | 实际只占 `190`（`continuum-memory/src/persist.rs:13`），191–199 为空但属 P6 档 | P7 取 200–209，不占 190–199；P6 档内空号留给 P6 后续 |

---

## 八、复审与验收

每份设计由**另一个代理**交叉复审，且复审的任务是**试着推翻**它的断言、并找**漏项**——不是核对其措辞
（同 P3 共享面 §六、P6 切分 §六）。本轮追加三条：

1. **P7C 的 `ALLOWED` 行必须是 `&[]`**，并在真含 P7 全部 crate 的树上跑
   `cargo test -p continuum-runtime --test dependency_direction`；「零依赖」不靠逐字推演。
2. **P7D 的合并门复核要真跑**：先确认 P7 新增 crate 让哪几条断言变红（§七第 11 条），再改 fixture／
   `P6_EXPECTED_ROWS`／断言 2 的计数；改完在真树上绿。
3. **P7B 的三层分离要有结构照片**：三个决策面各有独立入口／类型，不存在一个函数同时产出三者；
   Intent Scheduler 的模型判断经端口，不在 P7 里直调 `ModelProvider`。

**本轮验收**：四份设计文档过审 ＋ 各自的实现计划。**本轮不写实现代码**（设计未批不落实现）。

---

**本文的实测口径**：除 §七 的订正行外，正文引用的规范节号以 `docs/spec/05-normative.md` 为准；
《工程》与《总纲》分别指 `docs/02-工程.md` 与 `docs/01-总纲.md`。零命中类断言已限定语料（`crates/` 与
`Cargo.toml`）并逐处读过命中行；`continuum-context` 的「不存在」以 `ls crates` 与 `grep -rn` 两侧核对。
