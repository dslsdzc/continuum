# P7A Product Loop — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-18-p7-scope-and-split.md` §五 P7A 行的原话，逐字照用）：

> 建产品循环：§150 `EngineeringBaseline` → Product／ProductReadiness／Milestone 惰性生成／
> Milestone→Product Intent lower／Progress Gate／Shippable 与 post-shippable 的区分

**本块拥有 / 产出的接口**（同行的第二栏）：`Product`／`ProductReadiness`／`Milestone`／
`ProgressGateVerdict`／`ShippableVerdict` 各一份；`EngineeringBaseline` 的生成与
`Product.baseline` 写入；`Milestone` 惰性生成与批次；`Milestone→Product Intent` 的 lower；
`Progress Gate` 的判定与 `Decision` 产出；`ShippablePredicate` 与 post-shippable 的相位。
**消费** `continuum-semantics` 的 `Intent`／`Decision`／状态机公开面，`continuum-verify`
的 `Evidence`／`CompletionVerdict`，`continuum-canonical` 的 `EntityId`／`Opaque`。
**不定义** `Intent`／`IntentStatus`／`Decision`／`Evidence`／`CompletionVerdict` 的第二份。

**规范依据**：§150 §151 §152 §153 §154 §155 §156 §231 §232 §233 §234 §335 §336 §337；
《工程》§7.1（`docs/02-工程.md:420-431`）、§7.2（`:433-443`）、§7.3（`:445-458`）、
§7.4 判据 1／2／4（`:460-467`）、§7.5（`:469-473`）；《总纲》第 7 章
（`docs/01-总纲.md:1127-1322`）。

**本文件只写 P7A。** 其余三块（P7B Intent Scheduler、P7C Resource Arbiter、P7D
Supervisor／Session）各有其设计，本文件不替它们定任何形状。凡本设计不得不假设别的块
的某枚形状之处，一律在 §8 写成一条**具名的设计问题**，不在正文里就近决定。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p7-plan`，切分定稿
`c89d839`，2026-10-18）里当场 `grep -n`／`sed -n` 实测的，未照抄任何一份别的文档。
切分文档已给的行号按切分原文引用，并在本文件需要处复核实测。

**关于缺口的两句话（写在最前，免得被后文读成已关闭）**：

1. **A15／OPEN-010 不关闭**：九维聚合函数与 `progress_delta` 阈值仍无规范定义。本设计把
   它们收成**调用方申报的 `ProgressPolicy`**；没有申报策略时 Progress Gate 只能
   `Undetermined`，不得自行 replan（切分 §6.1 `:389-394`）。
2. **OPEN-015 不关闭**：§233 的有限视野与 §152 的并行／汇合的**批次边界**仍无规范定义。
   本设计把批次边界收成**显式 `BatchBoundary`**；测试只钉「不出现完整未来列表」与
   「评估后再生成」，不钉规范级批次大小（切分 §6.1 `:394`）。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| `Product` 七字段与身份 | §231 `docs/spec/05-normative.md:424-438`；切分 §2.3.1 `:124-133` | §4.1 | 全量；`id` 复用 `ProductIdRef`，不新造 `ProductId` |
| `ProductReadiness` 九维与 `value/evidence/last_verified` | §232 `docs/spec/05-normative.md:442-466`；§153 `docs/spec/03-product-drive.md:223-253` | §4.2 | 全量；**不做聚合、不做阈值**（§2.3 第 1、2 条） |
| `EngineeringBaseline` 十字段与生成 | §150 `docs/spec/03-product-drive.md:43-114`；切分 §三.5 `:200-212` | §4.3 | 全量；模型判断经具名端口，`Product` 只能由 `establish_baseline` 产生 |
| `Milestone` 七字段、七状态、惰性生成与批次 | §233 `docs/spec/05-normative.md:470-495`；§234 `:498-525`；§151 `docs/spec/03-product-drive.md:118-171`；§152 `:175-219` | §4.4 | 全量；只生成**一个有界批次**；不生成完整未来 |
| `Milestone→Product Intent` 的 lower | §152；§222 `docs/spec/05-normative.md:132-155`；切分 §三.6 `:213-243` | §4.8 | 确定构造 `Intent`；**只经 `continuum-semantics` 公开面落库**；只交接 `READY` |
| `Progress Gate`、`progress_delta`、replan 的 `Decision` | §337 `docs/spec/05-normative.md:2583-2597`；§154 `docs/spec/03-product-drive.md:257-309` | §4.5 | 三值判定；申报策略可测；**只出 `Decision`，不执行 replan** |
| `ShippablePredicate`、`ShippableVerdict`、post-shippable 区分 | §335 `docs/spec/05-normative.md:2535-2547`；§336 `:2551-2580`；§155 `docs/spec/03-product-drive.md:313-373` | §4.6、§4.7 | 三值 fail-closed；`Shippable` 今天**不可达**（§8 第 3 条） |
| `ContinuousProductDrive` 的落点 | §336；切分 §2.3.1 `:132-133` | §4.7（提案） | **切分偏离，待协调**：切分只给 `Product`／`ProductReadiness`／verdict 三处；本设计提案 (b) 另立 `ProductDriveState`＋`product_drive` 表，须协调者做 C2 裁定并更新切分 §2.3.1 后才落；**C2 裁定前不应用迁移 200**（§8 第 12 条） |

## 1.2 不建什么（摘要；完整表在 §7）

| 不建的东西 | 归属 / 理由 |
|---|---|
| Planner、replan 执行、替换路线 | 第 3 层 Planner 接口在代码里无实现（切分 §6.2 第 13 条 `:412`）；P7A 只出 `Decision` |
| `Intent` 状态机、`intent` 表的直接 SQL | P4；P7A 只调 `load_intent`／`save_intent`／`apply_intent_transition`（§4.8） |
| `Decision`／`Evidence`／`CompletionVerdict` 的第二份 | P4／P5a；P7A 只消费（§6.2、§6.3） |
| 验证、证据生产、Completion 正臂 | P5a／P9；P7A 不在 P5a 之外造「完成」（§8 第 3 条） |
| UI、会话、后台治理、资源仲裁、调度器 | P7B／P7C／P7D；P7A 不碰它们的内部（§7） |
| 九维聚合、`progress_delta` 阈值、规范级批次大小 | 规范未给；本设计只给申报入口（§2.3） |
| 完整未来 Milestone 列表 | §233 明文禁止（`docs/spec/05-normative.md:470-495`） |

---

# 2. 依据与自定形状

## 2.1 crate 与依赖边

本块新建 **一个** crate：**`continuum-product`**（切分 §1.2 `:58` 的建议名；实测
`grep -rn "continuum-product\|continuum_product" crates Cargo.toml` 在 `p7-plan` 上零命中）。

依赖边**只登记实际用到的**（切分 §四第 5 条 `:294-296`）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-semantics` | `Intent`（`crates/continuum-semantics/src/intent.rs:434`）、`IntentId`／`ContractRef`／`Priority`／`ProductIdRef`（`crates/continuum-semantics/src/ids.rs:41`；`intent.rs:331`／`:300`／`:372`）、`IntentStatus`（`intent.rs:61`）、`IntentKind`（`intent.rs:40`）、`Decision`／`DecisionOption`／`DecisionId`（`decision.rs:80`／`:125`；`ids.rs:132`）、`IntentError`／`DecisionError`（`ProductError` 的透传变体）、`save_intent`／`load_intent`（`persist.rs:353`／`:428`）、`apply_intent_transition`（`intent_tx.rs:78`）、`create_decision`（`persist.rs:1173`）。同 crate 的其余类型不另列边 | 普通 |
| `continuum-verify` | `Evidence`（`crates/continuum-verify/src/evidence.rs:222`）、`EvidenceId`（`:21`）、`CompletionVerdict`（`completion.rs:24`） | 普通 |
| `continuum-canonical` | `EntityId`（`crates/continuum-canonical/src/candidate.rs:62`）、`Opaque`（`src/lib.rs:152`）——§152 lower 确定构造 `Intent.goal` 与不透明字段 | 普通 |
| `continuum-persist` | 迁移 **200**、`Tx`／`Value`／`Migration`／`PersistError` | 普通 |

**不登记的边，逐条给理由**：

- **不登记 `continuum-runtime`**：它是装配方；本块是库，端口由它实现（切分 §三第 5 条
  `:200-212`）。
- **不登记 `continuum-provider`／`continuum-model-registry`／`continuum-node`**：P7A 不调
  `ModelProvider`、不路由模型、不放置节点；`EngineeringBaseline` 与 Milestone 的模型判断
  只经本块定义的 trait，由装配方转调（§4.3、§4.4）。
- **不登记 `continuum-graph`**：P7A 不调度、不接管节点状态；Milestone 是产品层的记录，
  不是 ADFIR 节点。
- **不登记 `continuum-events`**：`decision.required` 与 `intent.created` 由
  `continuum-semantics` 的公开写函数产生，本块不直接 append 事件。
- **不登记 `continuum-budget`／`continuum-policy`／`continuum-capability`**：本块不记账、
  不查策略、不取 capability；`progress_delta` 的策略是调用方申报的 Rust 值，不是策略引擎。

**外部依赖**：`async-trait`（两个生成端口的异步 trait）、`serde_json`（**内部**存储编码）、
`thiserror`（`ProductError`）。**dev 依赖**：`tokio`（异步端口测试）、`tempfile`（真库测试）、
`trybuild`（字段私有／无聚合方法的编译期照片）。这些都不进 `ALLOWED`——那张表断言的是
workspace 成员之间的边（`crates/continuum-runtime/tests/dependency_direction.rs:26` 起）。

**共写文件**（本块只加自己那一行）：

| 文件 | 本块的改动 |
|---|---|
| `Cargo.toml` 的 `[workspace] members`（`Cargo.toml:3`） | 加 `crates/continuum-product` |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-product", &["continuum-canonical", "continuum-persist", "continuum-semantics", "continuum-verify"])`——四项，按字母序 |
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`） | 加 `continuum_product::p7a_product_migrations()`（200） |
| `crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`） | 同批加 200 |
| `crates/continuum-runtime/tests/startup.rs` 的三处计数（`:26`／`:81`／`:94`） | 空库 14→15、P0 后 12→13、首启 14→15；**同批改，不得只改一处** |

**不碰** `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 与 `P6_EXPECTED_ROWS`：
P7A 的新 crate 会让 P6D 的合并门变红，按切分 §四第 1 条由 **P7D** 重新基线化或退役；P7A
只加自己的 `ALLOWED` 行（§8 第 16 条）。

**迁移号段**：P7A 确定落库，占 **200**（切分 §四第 2 条 `:265-276`）。**200 的 SQL 一经应用
不得再改**（同 110／130／190 的纪律）。P7A 不占 201–209。

**C2 裁定（`product_drive` 的落点裁定）是 200 的硬前置**（截至本设计，200 尚未应用）：
该裁定未作出前，P7A
**不合并、不应用**迁移 200——不存在「先应用三表版本、之后再补第四张」的路径。裁定后
200 一次成型：批准提案 (b)（独立 `product_drive` 表）⇒ 四张表；落点选 (a)（既有面）⇒
三张表，且 §4.7 的独立表与 API 作废，drive 的读/写改由 C2 裁定的既有面承担（具体表／列／
更新 API 随裁定给出；J17b 的照片只在 (b) 下写）。这是「不改已应用的 SQL」的唯一一致解；若协调者坚持
drive 表另取号，那是切分变更（201 已预留 P7D），必须报协调者；本设计不选这条。

## 2.2 冻结清单：本块持有的类型与判定

**类型**（定义在 `continuum-product`）：

```
Product                   —— §231 七字段（§4.1）
ProductReadiness          —— §232 九维（§4.2）
ReadinessDimension        —— 每一维的 value/evidence/last_verified（§4.2）
ReadinessDimensionName    —— 九维的名字（§4.2）
ReadinessValue            —— Unassessed / Assessed(Opaque)（§4.2）
EngineeringBaseline       —— §150 十字段（§4.3）
BaselineRequest           —— 生成端口的输入（§4.3）
Milestone                 —— §234 七字段（§4.4）
MilestoneState            —— §234 七状态（§4.4）
MilestoneId               —— P7A 的 Milestone 身份（§4.4）
MilestoneDraft            —— 生成端口的单枚提案（§4.4）
MilestoneDependency       —— Existing / InBatch（§4.4）
MilestoneBatchDraft       —— 生成端口的有界批次（§4.4）
BatchBoundary             —— 显式批次边界（§4.4）
MilestoneProposalInput    —— 生成端口的输入（§4.4）
ProgressGateVerdict       —— Triggered / NotTriggered / Undetermined（§4.5）
ProgressDelta             —— Improved / NoProgress / Regressed / Undetermined（§4.5）
ProgressRecord            —— 一段产品状态的记录（§4.5）
ProgressUndetermined      —— 不可判的原因（§4.5）
ProgressPolicy            —— 调用方申报的 delta 与阈值（§4.5，trait）
ShippablePredicate        —— 产品定义的发布条件（§4.6）
ShippableClause           —— 谓词的一条子句（§4.6）
ShippableClauseId         —— 子句身份（§4.6）
ShippableVerdict          —— Shippable / NotShippable / Undetermined（§4.6）
NotShippableReason        —— 明确不成立的原因（§4.6）
ShippableUndetermined     —— 不可判的原因（§4.6）
ShippableAssessment       —— 子句判定的输入（§4.6）
ClauseAssessment          —— 一条子句的判定（§4.6）
ClauseVerdict             —— Satisfied / NotSatisfied / Undetermined（§4.6）
ContinuousProductDrive    —— §336 的开关（§4.7）
ProductPhase              —— PreShippable / Shippable / PostShippableImprovement（§4.7）
ProductDriveState         —— 开关＋相位（§4.7）
ReplanDraft               —— Progress Gate 的 Decision 内容（§4.5）
ProductError              —— 本块的错误类型（§4.9）
PortError                 —— 生成端口的错误（§4.3）
EngineeringBaselineGenerator —— §150 的生成端口（§4.3，trait）
MilestoneGenerator        —— §233／§152 的生成端口（§4.4，trait）
```

**共 37 个公开名字：34 个类型 ＋ 3 个 trait**（`ProgressPolicy`／
`EngineeringBaselineGenerator`／`MilestoneGenerator`）。**没有** `ProductId`；**没有 §153 的
第二套九维名字**——`ReadinessDimensionName` 是 §232 九名的枚举化，只用于
`required_dimensions` 与迭代；没有第二枚 `Evidence`／`CompletionVerdict`／`Decision`／
`IntentStatus`（§2.5、J19）。

**判定**（本块是它们的唯一落点）：

1. `Product` 的身份与七字段（§4.1）——规范只给字段名，类型与不变量由本设计定。
2. `ProductReadiness` 的九维记录与 `value/evidence/last_verified` 的对应（§4.2）——规范只给
   九维名与「SHOULD 保存」三项，**不给值的量纲、聚合、阈值**。
3. `EngineeringBaseline` 的类型与生成入口（§4.3）——规范只给十字段名与生成流程，不给字段类型。
4. `Milestone` 的字段类型、ID、生成批次与 lower（§4.4、§4.8）——规范只给七字段与七状态。
5. `ProgressGateVerdict` 的三值、`ProgressDelta` 的计算入口、replan 的 `Decision` 形状
   （§4.5）——规范只给 `progress_delta` 与「连续多个」。
6. `ShippablePredicate` 的子句形状、`ShippableVerdict` 的三值与 `ProductPhase`
   （§4.6、§4.7）——规范只给名字。
7. `ContinuousProductDrive` 的落点（§4.7）——规范只给开关名与 post-shippable 的语义。

**本块不持有的判定**：验证与证据（P5a）；`CompletionVerdict` 的正臂（P5a／P9）；
`Intent` 状态机（P4）；replan 的路线（第 3 层 Planner／P8）；资源仲裁（P7C）；
后台任务来源（P7D）；模型路由（第 4 层）。

## 2.3 本块发明的判据（逐条：规范给到哪、代价、边界、落点、收件人）

**这一节是 P7A 的「发明判据」总账。共 17 条（#1–#17）。** 规范没给、而本设计不得不定的
每一处判定，都在下表里有一条；**没进这张表的规范级判定，本块一处也没有定**。代码块只是
示意，正文措辞才是约束。第 17 条是**强制点设计**（保存前的评估门），不是规范缺口。

| # | 发明的判定 | 规范给到哪一步 | 本设计的选择 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|---|
| 1 | `ProductReadiness` 的 `value` 类型与**无聚合** | §232 `:442-466` 只给九维名与 `value/evidence/last_verified`；§153 `:223-253` 只说「重新评估」；**值的量纲与聚合函数未给** | `ReadinessValue::Assessed(Opaque)`／`Unassessed`；`ProductReadiness` **没有** `score()`／`average()`／`sum()`；聚合只发生在调用方申报的策略里 | 消费方不能比较两维数值；每枚 `value` 是不透明串，P7A 不解释 | 拦不住调用方在策略里发明一套数值刻度；拦不住「九维都写了但都是主观分」 | §4.2、§4.5 | 规范维护者 ＋ P10 |
| 2 | `progress_delta` 的计算 | §337 `:2583-2597` 只给名字；§154 `:257-309` 只给 `previous_product_state`／`current_product_state`／`progress_delta` 与「≈0」 | `ProgressPolicy::delta(&ProductReadiness, &ProductReadiness) -> ProgressDelta`；`ProgressDelta` 四值（Improved／NoProgress／Regressed／Undetermined） | 两个策略可对同一对状态给出不同 delta；P7A 不存数值 | 不覆盖「缓慢进展」；不覆盖「一维进步、另一维退步」的加权 | §4.5 | 规范维护者 ＋ P10 |
| 3 | `progress_delta` 的阈值与「连续多个」 | §337 只说「连续多个 Milestone」，**没有数字**；A15／OPEN-010 明文未定义 | `ProgressPolicy::stall_threshold() -> u32`；**无默认值**；0 视为无效 ⇒ `Undetermined`；没有策略 ⇒ `Undetermined` | 调用方可以把阈值设成 1 或极大；P7A 不替规范选默认 | 拦不住「有策略但策略永远返回 Improved」；拦不住不做策略申报的调用方 | §4.5 | 规范维护者 ＋ P10 |
| 4 | Milestone 的**批次边界** | §233 `:470-495` 要求有限视野；§152 `:175-219` 允许并行／汇合；**批次边界未给**（OPEN-015） | `BatchBoundary { max_milestones, max_dependency_depth }`；**显式参数、无默认**；超界**拒绝**（不静默截断） | 合法的长依赖链会被拆成多批；调用方可以申报一个巨大的边界，从而在事实上生成完整未来 | 拦不住「调用方申报 `usize::MAX`」；拦不住「生成器本身返回完整未来」——P7A 只查边界 | §4.4 | 规范维护者 ＋ P10 |
| 5 | `ContinuousProductDrive` 的形状与落点 | §336 `:2551-2580` 只给开关名与 post-shippable 的可继续项；§155 `:313-373` 只给「必须区分」；切分 §2.3.1 `:132-133` 只给三处落点 | **提案 (b)**：`ContinuousProductDrive { enabled }` ＋ `ProductPhase` ＋ `ProductDriveState`，落在 Product 面的独立状态（`product_drive` 表），**不是 §231 的第八个字段**。**这是切分未授权的第四种落点，作为切分变更报协调者，裁定并更新切分 §2.3.1 前不落库**（§8 第 12 条）；若协调者选 (a)，独立表与 API 作废、改为既有面（表／列／更新 API 随裁定给出；J17b 照片延后） | `Product` 的聚合跨两张表；P7D 看不到开关（它只按值收 Product Intent，切分 §三.6）；C2 裁定前 200 不得合并/应用；裁定后一次成型（(b) 独立表四张 / (a) 既有面三张） | 今天的 `Shippable` 不可达（§8 第 3 条），所以 `Shippable`／`PostShippableImprovement` 相位没有生产路径 | §4.7（提案）、§8 第 12 条 | 协调者（裁定）＋ 规范维护者 |
| 6 | `Product` 的字段类型 | §231 `:424-438` 只给七个字段名；`current_milestone`／`milestone_history[]` 的元素类型未给；`baseline` 是否可空未给 | `current_milestone: Option<MilestoneId>`；`milestone_history: Vec<MilestoneId>`（由 `milestone` 表按 `seq` 派生，不另存一列）；`baseline: EngineeringBaseline`（非可空） | 历史只有 id，不是快照；`current_milestone` 单数，不能同时表示并行批次里的多个当前项；没有 Product 行就没有 baseline | 拦不住「Milestone 行被外部删除后历史变短」；拦不住「并行 Milestone 的 current 语义有歧义」 | §4.1 | 规范维护者 ＋ 协调者 |
| 7 | `ShippablePredicate` 的形状 | §335 `:2535-2547` **只给名字**，一个字段都没给；§155 只给一个自然语言例子 | `required_dimensions[]` ＋ `clauses[]`；子句有稳定 id 与不透明 statement；**空谓词拒绝**；判定不在 P7A 内（`ShippableAssessment` 由调用方给） | 调用方要自己评估子句；P7A 不能保证子句真的被评估过 | 拦不住「调用方把所有子句都报 Satisfied」；拦不住「required dimension 有值但值是主观分」 | §4.6 | 规范维护者 |
| 8 | `ShippableVerdict` 的三值组合 | §335／§336；切分 §2.3.1 `:127-131`、§四第 4 条 `:286-293`；`CompletionVerdict` **只有两臂**（`crates/continuum-verify/src/completion.rs:24`） | 三值 fail-closed；**`MissingProduct` 是前置（`product == None` 时不看 completion）**；其余 `NotShippable` 优先于 `Undetermined`：**先查 completion `Blocked` 与 clause `NotSatisfied`，再查 assessment 结构错误（缺／多／重复）与其余不可判项**；`Undetermined` 不是许可；**`Shippable` 今天没有生产者** | 正许可面不可拍；`Shippable` 相位不可达；只能测负臂与不可判臂；结构错误的 assessment 不会遮蔽明确不成立 | 拦不住「外部直接构造 `ShippableVerdict::Shippable`」（它是公开枚举；§8 第 14 条） | §4.6 | P5a ＋ P9（ENG-004） |
| 9 | `EngineeringBaseline` 的字段类型与生成入口 | §150 `:43-114` 只给十字段名与流程；字段类型未给；切分 §三第 5 条把生成判给模型端口 | 十个字段取 `Opaque`／`Vec<Opaque>`；生成经 `EngineeringBaselineGenerator`；`Product` 只能由 `establish_baseline`＋`save_product` 产生；`product_goal` 与 `minimum_complete_system` 为空 ⇒ 基线不可用 | 模型输出只能是不透明文本；P7A 不判工程好坏；空列表的语义（「没有」还是「没填」）分不开 | 拦不住「生成器把十条都填成空串以外的东西」；拦不住「基线其实不成立」 | §4.3 | 规范维护者 |
| 10 | lower 的 `goal`／`id`／`status`／`completion_predicate` | §152 `:175-219` 只说 Milestone→Intent；§222 `:132-155` 给 `Intent` 字段；**lower 的确定构造规则未给** | `id = intent/{milestone_id}`；`goal = EntityId::from_str(objective)`（**不跑 Canonicalizer**）；`status = PROPOSED`；`parent_intent = Some(product.intent_id)`；`completion_predicate` 取 `exit_conditions` 的确定性编码（零条 ⇒ `""`；空元素在 §4.4 被拒，故 `""` 唯一）；`contract_id` 必须由调用方给 | `goal` 不是规范化实体；同一 objective 的两枚 Milestone 会得到同一个 `EntityId`；`completion_predicate` 对 P4／P5a 是不透明串，不能被解析 | 不覆盖「§152 的并行／汇合图」——`Intent` 只有单亲 `parent_intent`，图边留在 `milestone` 表（§8 第 6 条） | §4.8 | P4 ＋ 规范维护者 |
| 11 | replan 的 `Decision` 内容 | §337 只说「replan」；§154 `:299-305` 给的是**策略选项的例子**（A 重构／B 接受／C 改目标）；§331 只给 `Decision` 四字段 | `ReplanDraft { issue, options, blocking }` 由调用方给；P7A 只决定「是否出」，不决定「有哪些路线」；`Decision.id` 由 P7A 按 product+milestone 派生 | 没有调用方给 draft 时，gate 只能停在 Triggered 而没有 Decision；P7A 不能自己生成 replan 选项 | 拦不住「draft 的选项只是把问题重述一遍」；不执行 replan（消费方是具名缺口） | §4.5 | 协调者 ＋ 第 3 层 Planner ＋ P8 |
| 12 | Evidence 到 readiness 的映射 | §232／§153 只说「用验证结果重新评估」；P5a 的 `Evidence` **只写不读**（`crates/continuum-verify/src/evidence.rs:117`；切分 §6.2 第 6 条 `:405`） | `record_milestone_progress` 收 `&[Evidence]` 值，**只用它核 `EvidenceId` 的存在性**：遍历 `current.iter()`，对每维调 `ProductReadiness`／`ReadinessDimension` 的只读 getter（`value()`／`evidence()`／`last_verified()`）取出被引用的 id；落库只存 id；不读 `evidence` 表；不判 claim／type／strength | P7A 不证明某条 evidence 支持某一维；九维的 `Assessed` 值仍由调用方给 | 拦不住「调用方给了 id 但值与该证据无关」；拦不住「证据内容是负面的」 | §4.2（getter）、§4.5 | P5a ＋ 规范维护者 |
| 13 | Milestone 的状态迁移 | §234 `:498-525` 只给七状态，**没有迁移表**；《工程》§7.4 判据 1 只要求生成 | P7A 只存 `state`；`record_milestone_state(expected, to)` 做乐观并发（当前值必须等于 expected），**不判合法对** | 非法迁移可以被写入；P7A 不是 Milestone 状态机 | 拦不住「Proposed→Completed 直接跳」；拦不住「没有执行就改 Completed」 | §4.4 | 规范维护者 ＋ P1／协调者 |
| 14 | `Regressed` 是否计入「无进展」 | §337 只说 `progress_delta ≈ 0`；没有说负进展 | `NoProgress` 计数；`Regressed` **不**计入连续无进展（但它会打断当前 run） | 性能回退可能不会触发 replan；调用方要自己处理 Regressed | 拦不住「策略把回退报成 Improved」 | §4.5 | 规范维护者 ＋ P10 |
| 15 | `current_milestone` 的单数语义 | §231 只给单数字段；§152 允许并行 | `current_milestone` 是**游标**（当前推进的那一枚）；同批其他 Milestone 在 `milestone_history` | 不能同时表示多个 ACTIVE；并行批次里的「当前」由调用方选 | 拦不住「调用方把游标指向一个已经 Completed 的 Milestone」——`set_current_milestone` 会拒绝终态 | §4.1、§4.4 | 规范维护者 ＋ 协调者 |
| 16 | baseline 与 `ShippablePredicate` 的不可变性 | §150／§335 没给更新／版本化规则 | 本轮**创建后不可变**；没有 `set_baseline`／`set_shippable_predicate` | 产品合同变化只能新建 Product 或手工迁移；没有版本历史 | 拦不住「调用方绕过 P7A 直接改库」 | §4.1、§4.3、§4.6 | 规范维护者 ＋ P4 |
| 17 | **保存前的评估门**：每个 `COMPLETED` Milestone 必须先有 `ProgressRecord`，才允许保存下一批 | §233 `:470-495` 的循环是 observe→gap analysis→next；§153 `:223-253` 说每完成一个要重新评估；**强制点未给** | `save_milestones` 在写新批次前检查：该 Product 的每个 `COMPLETED` Milestone 都有 `product_progress` 行，否则 `MissingProgressForCompleted` | 调用方必须先记录 progress 再保存；一个失败的 Milestone 不要求 progress；生成调用可能已被浪费 | 拦不住「progress 记录是敷衍的」；拦不住「调用方把 state 设成 COMPLETED 但不记录」——下一次保存会被拒，但错误发生在保存时 | §4.4、§4.9 | 规范维护者 ＋ **协调者**（强制点归属须由协调者确认；P10 不是产品循环的强制点） |

**表外的两件事，明确不是「发明判据」**：①存储编码（JSON 形状、列名）是**内部实现**，不是
规范判定，见 §4.9；②`Product.id` 复用 `ProductIdRef` 是切分 §2.3.1 `:126` 的冻结，不是本设计
的选择。

## 2.4 切分 §四 六条横切约束的逐条落实

| 切分 §四 | 要求（要点） | 本设计的落点 | 可观察判据 |
|---|---|---|---|
| **第 1 条**（`:255-264`） | 不新增反向边；四块各自在 `ALLOWED` 加自己一行；既有行不因 P7 增枚，唯一例外是装配方 `continuum-runtime`；P6D 合并门由 P7D 复核 | §2.1 的四项边表；P7A 不加 `continuum-runtime`／provider／model-registry／node；不碰 P6 fixture | `cargo test -p continuum-runtime --test dependency_direction` 在含本 crate 的树上绿；`ALLOWED` 的 `continuum-product` 行恰是四项 |
| **第 2 条**（`:265-276`） | 迁移 200–209 档；P7A 确定落库占 200；同步三处计数 | §4.9：C2 裁定后 200 一次成型——(b) 四张表，(a) 三张表；裁定前不应用；`runtime_migrations()`／`expected_migrations()`／`startup.rs:26`／`:81`／`:94` 同批 | 迁移版本 `[200]`；启动输出 15／13／15；190 的 SQL 一字不改 |
| **第 3 条**（`:277-285`） | 同一件事两个词汇表：§232 九名优先；§153 不建第二套；不新造 `ProductId`；`Decision` vs `PolicyDecision` | §2.5；§4.1（`id: ProductIdRef`）；§4.2（九名字段）；本 crate 不定义 `Decision`／`PolicyDecision` | 源码面：无 `struct ProductId`、无 `functional_completeness` 字段、无 `enum IntentStatus` |
| **第 4 条**（`:286-293`） | 只消费 `Evidence`／`CompletionVerdict`；不消费原始工具输出／模型自述／`Claim`；Shippable 三值 fail-closed；`Undetermined` 不是许可 | §4.2 的 `EvidenceId`；§4.5／§4.6；§6.2 | 源码面：无 `ToolResult`／`Claim` 字段；运行期：`Undetermined` 分支不产生 `Shippable`／不产生 Decision |
| **第 5 条**（`:294-296`） | 依赖边只登记实际用到的；`ALLOWED` 与实际依赖精确一致 | §2.1 四行边表与 `ALLOWED` 四项一一对应；dev 外部依赖不进表 | `dependency_direction` 的两个方向都绿 |
| **第 6 条**（`:297-311`） | P7A 与三块无未冻接口；只有一条按值交接的 Product Intent；P7A 可全并行 | §4.8 的 `product_intent_for_handoff(tx, product, &milestone)`：只返回 `READY` 的值或 `None`（按 MilestoneId，不看当前游标）；P7D 不读 Product 记录；本块不登记 `continuum-supervisor` 边 | J11（只交接 READY＋两个 READY）；源码面：本 crate 无 `continuum_supervisor`／Product 记录的跨块读 |

## 2.5 同名异物（本块一个都不用）

| 已有的东西 | 它是什么 | 实测位置 | 本块的处置 |
|---|---|---|---|
| `Product` | §231 的产品对象／`MemoryScope::Product`／`Intent.kind = Product` | `crates/continuum-memory/src/claim.rs:20`；`crates/continuum-semantics/src/intent.rs:40`；`:464` | 本块只认 §231；产品身份用 `ProductIdRef`；**不把 memory scope 当产品** |
| `Decision` | §331 的决策单／策略的 `Allow/Deny/RequireApproval` | `crates/continuum-semantics/src/decision.rs:80`；`crates/continuum-policy` 的同名类型 | 本块只用前者；本 crate **不引用** policy，故不需要 `PolicyDecision` 别名 |
| `progress_delta` | §337 的名字／§154 的记录字段／本设计的四值枚举 | `docs/spec/05-normative.md:2583-2597`；`docs/spec/03-product-drive.md:269-273` | 本块的 `ProgressDelta` 是**判定值**；`ProgressRecord.progress_delta` 是记录字段；两者同名不同层，正文写清 |
| `Shippable` | §335 的名字／`ShippableVerdict::Shippable`／`ProductPhase::Shippable` | `docs/spec/05-normative.md:2535-2547`；§4.6、§4.7 | 谓词是输入（`ShippablePredicate`），判定是结果（`ShippableVerdict`），相位是生命周期（`ProductPhase`）；**三个词不混用** |
| `Suspended` | `NodeState::Suspended`／`IntentStatus::Suspended`／P7C 的挂起相位 | `crates/continuum-graph/src/node.rs:63`；`crates/continuum-semantics/src/intent.rs:77` | 本块不碰；P7A 没有挂起状态 |
| `Proposed`／`Verifying`／`Completed`／`Failed` | 两套独立状态机共用四个名字：`MilestoneState`（§234）与 `IntentStatus`（§223） | `crates/continuum-semantics/src/intent.rs:61`；§4.4 | 两套枚举各归各的层：`IntentStatus` 由 P4 的迁移表管，`MilestoneState` 只做乐观并发（§2.3 第 13 条）；不互相转换、不共用枚举，同处出现时以类型名限定 |

---

# 3. 规范依据逐条

**这一节的写法**：每条先给规范原文的落点，再给「规范给了什么 / 规范没给什么 / 本设计怎么落」。
规范没给的部分在 §2.3 里另记，不在这里冒充规范。

## 3.1 §150 Engineering Feasibility Pass（`docs/spec/03-product-drive.md:43-114`）

**原文（逐字，`:61-81`）**：

> EngineeringBaseline {
>     product_goal
>     target_users
>     required_capabilities
>     hard_constraints
>     architecture
>     technology_choices
>     minimum_complete_system
>     known_unknowns
>     engineering_risks
>     acceptance_criteria
> }

**规范给了什么**：①第一阶段的顺序（Idea→Requirement Analysis→Engineering Feasibility→
Architecture→Engineering Baseline，`:49-59`）；②十个字段名；③目标不是 MVP 清单，而是
「这个产品作为一个完整工程，怎样才能成立」（`:83-114`）。

**规范没给什么**：①十个字段的类型；②生成方式（模型？人？）；③「基线成立」的判据。

**本设计怎么落**：`EngineeringBaseline` 十字段（§4.3）；生成经 `EngineeringBaselineGenerator`
端口（模型判断，装配方实现）；`establish_baseline`＋`save_product` 是 Product 的唯一创建路径；
`product_goal` 与 `minimum_complete_system` 为空 ⇒ 基线不可用（§2.3 第 9 条）。

## 3.2 §231 Product Object（`docs/spec/05-normative.md:424-438`）

**原文（逐字，`:426-438`）**：

> Product {
>     id
>     intent_id
>
>     baseline
>     readiness
>     current_milestone
>     milestone_history[]
>
>     shippable_predicate
> }

**规范给了什么**：七个字段名与书写序；`milestone_history[]` 是数组。

**规范没给什么**：字段类型；`current_milestone` 是否可空；`milestone_history[]` 的元素是
Milestone 值还是 id；`baseline` 是否可空；`Product.id` 与 `Intent.product_id` 的关系。

**本设计怎么落**：§4.1 的七字段；`id: ProductIdRef`（与 `Intent.product_id` 同一枚类型，
切分 §2.3.1 `:126`）；`intent_id: IntentId`（锚 Product Intent）；`baseline:
EngineeringBaseline`（非可空）；`readiness: ProductReadiness`；`current_milestone:
Option<MilestoneId>`；`milestone_history: Vec<MilestoneId>`（由 `milestone` 表派生）；
`shippable_predicate: ShippablePredicate`。**没有第八个字段**；`ContinuousProductDrive` 在
独立状态里（§4.7）。

## 3.3 §232 Product Readiness（`docs/spec/05-normative.md:442-466`）

**原文（逐字，`:444-464`）**：

> ProductReadiness {
>     functional
>     correctness
>     reliability
>     performance
>     security
>     usability
>     maintainability
>     deployability
>     documentation
> }
>
> 该对象 SHOULD 保存：
>
> value
> evidence
> last_verified

**规范给了什么**：九个维度名；每一项 SHOULD 保存 `value`／`evidence`／`last_verified`，
而不是仅保存主观评分。

**规范没给什么**：`value` 的量纲；`evidence` 的元素类型；`last_verified` 的时间形态；
九维的聚合函数；每一项是否必须同时有三者。

**本设计怎么落**：`ProductReadiness` 九个字段（名字逐字，§4.2）；每项是
`ReadinessDimension { value, evidence, last_verified }`；`value: ReadinessValue`（不透明值或
未评）；`evidence: Vec<EvidenceId>`（只存 id，不复制 `Evidence`）；`last_verified:
Option<i64>`（毫秒，由调用方给，本层不取时钟）。**没有聚合方法**；聚合只在
`ProgressPolicy`／`ShippableAssessment` 里（§2.3 第 1、7 条）。**`ProductReadiness` 本身没有
`Undetermined` 态**：未评是 `ReadinessValue::Unassessed`（一个记录值）；「不可判」只在
`ShippableVerdict::Undetermined`／`ProgressUndetermined` 里表达（§4.2）。

**与 §153 的九维名不一致的处置**：§153（`docs/spec/03-product-drive.md:230-240`）写
`functional_completeness` 且顺序不同。切分 §四第 3 条 `:279-280` 已裁定**以 §232 为准**；
本设计用 §232 的九个名字；`ReadinessDimensionName` 只是这九名的枚举化，**不是 §153 的
第二套名字**（§2.5）。

## 3.4 §233 Product Drive（`docs/spec/05-normative.md:470-495`）

**原文（逐字，`:472-494`）**：

> Product Drive MUST 使用有限视野规划。
>
> 禁止：
> 一次生成整个未来无限 Milestone
>
> 应采用：
> observe → gap analysis → next milestone → execute → verify → observe again

**规范给了什么**：有限视野是 MUST；禁止一次生成完整未来；循环的六个阶段。

**规范没给什么**：一次生成**几个**；并行／汇合时下一批覆盖多少未来状态（OPEN-015）；
「observe」的输入是什么。

**本设计怎么落**：`propose_next_batch` 只生成**一个有界批次**（§4.4）；`BatchBoundary` 是
显式参数（§2.3 第 4 条）；生成器的输入是 `baseline + readiness + 已生成历史 + boundary`；
**没有**「只给 baseline 就返回全部 Milestone」的 API。`observe` 在本块是
`ProductReadiness`／`ProgressRecord`（§4.2、§4.5）。

## 3.5 §234 Milestone 数据模型（`docs/spec/05-normative.md:498-525`）

**原文（逐字，`:500-525`）**：

> Milestone {
>     id
>     product_id
>
>     objective
>     dependencies[]
>
>     entry_conditions[]
>     exit_conditions[]
>
>     state
> }
>
> 状态：
> PROPOSED APPROVED ACTIVE VERIFYING COMPLETED FAILED SUPERSEDED

**规范给了什么**：七个字段名；七个状态名。

**规范没给什么**：字段类型；ID 的生成规则；状态之间的合法迁移；`dependencies[]` 的元素
是 id 还是值；`entry_conditions`／`exit_conditions` 的语法。

**本设计怎么落**：§4.4 的七字段与七状态；`id: MilestoneId`（P7A 派生）；`product_id:
ProductIdRef`；`objective: Opaque`；`dependencies: Vec<MilestoneId>`；`entry_conditions:
Vec<Opaque>`；`exit_conditions: Vec<Opaque>`；`state: MilestoneState`。**不定义迁移表**；
`record_milestone_state` 只做乐观并发（§2.3 第 13 条）。

## 3.6 §335 Shippable Predicate（`docs/spec/05-normative.md:2535-2547`）

**原文（逐字，`:2537-2547`）**：

> Product MUST 明确定义：
> ShippablePredicate
>
> 达到后才能宣告：
> SHIPPABLE

**规范给了什么**：Product MUST 定义一个叫 `ShippablePredicate` 的东西；达到后才能宣告
`SHIPPABLE`。

**规范没给什么**：**一个字段都没有**；怎样算「达到」；`SHIPPABLE` 与 Product 状态的关系。

**本设计怎么落**：`ShippablePredicate { required_dimensions[], clauses[] }`（§4.6）；
`ShippableVerdict` 三值；`Shippable` 只在所有 required dimensions 已评、所有 clauses 为
`Satisfied`、且 Completion 不处于 `Blocked`／`Undetermined` 时给出。**今天 Completion 只有
两臂，故 `Shippable` 不可达**（§8 第 3 条）。

## 3.7 §336 Continuous Product Drive（`docs/spec/05-normative.md:2551-2580`）

**原文（逐字，`:2553-2579`）**：

> 如果用户启用：
> ContinuousProductDrive = true
>
> 达到 Shippable 后 MAY 继续：
> 维护 优化 性能提升 可靠性提升 技术债处理
>
> 但 MUST 区分：
> Shippable
> 与：
> Post-shippable improvement

**规范给了什么**：一个用户开关；Shippable 后可继续的五类工作；MUST 区分 Shippable 与
post-shippable improvement。

**规范没给什么**：开关存在哪里；post-shippable 是一个状态、标签还是工作类别；两者的
转换规则。

**本设计怎么落**：**提案 (b)**——`ContinuousProductDrive { enabled }`＋`ProductPhase`＋
`ProductDriveState`（§4.7）；开关默认 `false`；`Shippable` 相位只在
`ShippableVerdict::Shippable` 时进入；`PostShippableImprovement` 只在开关启用且开始一项
post-shippable 工作时进入；**相位只前进，不回退**（§4.7）。**落点本身是切分偏离：切分
§2.3.1 只给 `Product`／`ProductReadiness`／verdict 三处，独立表是第四种，须协调者裁定
（§8 第 12 条）；裁定前 §4.7 是提案，不应用迁移 200；裁定后 200 一次成型（(b) 四表／(a) 三表）。**

## 3.8 §337 Progress Gate（`docs/spec/05-normative.md:2583-2597`）

**原文（逐字，`:2585-2597`）**：

> Product Drive 必须检测：
> progress_delta
>
> 连续多个 Milestone 无实际进展时 MUST：
> replan
>
> 不得无限空转。

**规范给了什么**：必须检测 `progress_delta`；连续多个 Milestone 无实际进展 MUST replan；
不得无限空转。

**规范没给什么**：`progress_delta` 的量纲与计算；「连续多个」的数字；replan 的路线；
replan 由谁执行（切分 §6.2 第 13 条 `:412`）。

**本设计怎么落**：`ProgressPolicy` 给 delta 与阈值（§4.5）；`ProgressGateVerdict` 三值；
`Triggered` 时只经 `continuum_semantics::create_decision` 出 `Decision`（`decision.required`
事件由该函数写），**不执行 replan**；没有策略 ⇒ `Undetermined`（§2.3 第 2、3 条）。

## 3.9 §151–§156（`docs/spec/03-product-drive.md`）

| 节 | 原文要点 | 本设计的落点 |
|---|---|---|
| §151 `:118-171` | Milestone 数量无预设上限；惰性生成；不预先知道最终有几个 | §4.4 的批次；`milestone` 表只存已生成的行 |
| §152 `:175-219` | Milestone 是高层 Intent Graph；允许并行／汇合；`Milestone → Intent → ADFIR` | §4.4 的依赖；§4.8 的 lower；**图边留在 `milestone` 表**，`Intent` 只有单亲（§8 第 6 条） |
| §153 `:223-253` | 每完成一个 Milestone 重新评估产品；不是只检查代码能否运行 | `record_milestone_progress` 要求 `state == COMPLETED`；九维 readiness（§4.2、§4.5） |
| §154 `:257-309` | 记录 previous／current／`progress_delta`；≈0 时停止当前路线、重新分析、换方案；必要时 Decision Required | `ProgressRecord` 三字段；`ProgressGateVerdict`；`ReplanDraft`（§4.5） |
| §155 `:313-373` | Product Contract 必须定义 Shippable Predicate；达到后通知；post-shippable 不等于无限推迟完成 | `ShippablePredicate`／`ShippableVerdict`／`ProductPhase`（§4.6、§4.7） |
| §156 `:377-406` | Product Drive 可以长期驻留；后台继续低风险 Milestone | P7D 的 `product drive` 按值消费 `READY` Product Intent；P7A 只出值（§4.8、§6.1） |

## 3.10 《工程》§7.1–§7.5（`docs/02-工程.md:420-473`）

| 工程节 | 原文要点 | 本设计的落点 |
|---|---|---|
| §7.1 `:422-431` | 八行组件表；P7A 是 `Product 对象与 ProductReadiness`、`Milestone 生成`、`Progress Gate`、`Shippable Predicate` | §1.1、§4 |
| §7.2 `:433-443` | Milestone 有限视野，禁止完整未来；允许并行／汇合；Progress Gate 检测 `progress_delta`；Shippable 与 post-shippable 区分 | §4.4、§4.5、§4.6、§4.7 |
| §7.3 `:445-458` | `Product ← 第 2 层 Intent`；`ProductReadiness ← Product + 第 8 层验证结果`；`Milestone 生成 ← Product + ProductReadiness`；`Progress Gate ← Milestone 历史 + ProductReadiness`；`Shippable ← ProductReadiness + 第 8 层 Completion Predicate` | §4.1、§4.2、§4.4、§4.5、§4.6 |
| §7.4 `:460-467` | 四条完成判据（判据 1／2／4 落 P7A；判据 3 落 P7C） | §5.3 逐条表 |
| §7.5 `:469-473` | A15／OPEN-010 与 OPEN-015 | §2.3、§8 第 1、2 条；**不关闭** |

## 3.11 与 §222／§223／§331 的接口（只读／只经公开面）

- **§222 `Intent`**（`docs/spec/05-normative.md:132-155`；代码 `intent.rs:434`）：lower 输出
  十四个存储字段（`child_intents[]` 不落列，`save_intent` 的规则见 `persist.rs:353`）；
  `product_id: Option<ProductIdRef>`（`:464`）。P7A 不定义第二枚 `Intent`。
- **§223 状态机**（`docs/spec/05-normative.md:174-206`；代码 `intent.rs:61`、`:217`）：
  lower 只产出 `PROPOSED`；P7A **不**把 `PROPOSED` 推进到 `READY`（那要过 `SPECIFYING →
  PLANNING → REVIEW → READY`，由 P4 的审查流程驱动）。P7A 只对 **`COMPLETED → EVOLVING`**
  这一条 Product 专用边提供 `retire_completed_product_intent`，它调用
  `apply_intent_transition`（`intent_tx.rs:78`），**不修改状态机**（§4.8）。
- **§331 `Decision`**（`docs/spec/05-normative.md:2471-2482`；代码 `decision.rs:80`）：
  Progress Gate 用它；`create_decision`（`persist.rs:1173`）写 `decision.required` 事件；
  P7A 不自造决策单。

---

# 4. 类型与接口

## 4.1 `Product`（§231 七字段）

```
/// §231 的七个字段。字段名与书写序照规范逐字；字段私有，
/// 唯一的公开创建路径是 `establish_baseline`＋`save_product`（§4.3）。
/// 不实现 `Default`／`Deserialize`：那会造出绕过 baseline 的第二个构造点。
#[derive(Debug, Clone)]
pub struct Product {
    id: ProductIdRef,
    intent_id: IntentId,
    baseline: EngineeringBaseline,
    readiness: ProductReadiness,
    current_milestone: Option<MilestoneId>,
    milestone_history: Vec<MilestoneId>,
    shippable_predicate: ShippablePredicate,
}
```

| §231 字段 | 本设计的类型 | 说明 |
|---|---|---|
| `id` | `ProductIdRef` | 与 `Intent.product_id` 同一枚类型（`intent.rs:372`／`:464`）；**不新造 `ProductId`** |
| `intent_id` | `IntentId` | 锚 Product Intent；`Product.id` 必须等于该 Intent 的 `product_id` |
| `baseline` | `EngineeringBaseline` | §150 的十字段；非可空；创建后不可变（§2.3 第 16 条） |
| `readiness` | `ProductReadiness` | §232 九维；初始为 `ProductReadiness::unassessed()` |
| `current_milestone` | `Option<MilestoneId>` | 游标；不是「所有 ACTIVE Milestone」；并行批次见 §2.3 第 15 条 |
| `milestone_history` | `Vec<MilestoneId>` | 由 `milestone` 表按 `seq` 派生；不另存一列（一份真值） |
| `shippable_predicate` | `ShippablePredicate` | §335；创建后不可变（§2.3 第 16 条） |

**唯一公开构造路径**：`establish_baseline`（异步生成，§4.3）返回一个 `Product`，再由
`save_product(tx, &product)` 落库。**没有** `Product::new`／`Product::from_parts`／
`Product { .. }` 的公开入口。crate 私有的唯一构造点是 `Product::from_verified_parts`
（`establish_baseline` 与 `persist::row_to_product` 都经它，且它逐条校验 §4.1 的不变量）。
J5 因此分两条 grep：① `grep -rn "Product {"` 的允许命中是 `struct Product {` 的字段定义与
`impl Product {` 的固有实现块（本节的访问器；若拆成多个 impl 则逐块登记）；② 构造点用
`grep -rn -e "Self {" -e "Product::from_verified_parts"`，允许命中是 `from_verified_parts`
体内的唯一 `Self {` 与 `establish_baseline`／`row_to_product` 两个调用点。**清单外命中即红。**

**不变量**：

1. `anchor_intent.product_id` 必须是 `Some`，且 `product.id.as_str() ==
   anchor_intent.product_id.as_ref().unwrap().as_str()`。**`ProductIdRef` 没有 `PartialEq`
   （`intent.rs:372` 只有 `Debug, Clone`），实现必须按 `as_str()` 比较，不能逐字照抄 `==`。**
2. `anchor_intent.kind == IntentKind::Product`（`IntentKind` 派生 `PartialEq`，可直接比）。
3. `anchor_intent.contract_id.id` 非空且 `version >= 1`（与 P7D 的授权证据同向，切分 §三.6
   `(ii)`）。
4. `anchor_intent.status == IntentStatus::Ready`。**锚 Intent 必须是已过 §223 审查链的
   `READY`**：`SPECIFYING`／`PLANNING`／`REVIEW`／`PROPOSED` 及任何运行态都不许建立 baseline
   （否则 Product 会挂在一条未授权或已越过审查的 Intent 上）。`RUNNING` 也拒绝——baseline
   是产品工作的入口，不能在工作已开始后才补。
5. `milestone_history` 里每个 id 都属于本 Product（由查询保证）。
6. `current_milestone` 若非 `None`，必须是本 Product 的、且不处于 `Completed`／`Failed`／
   `Superseded`。

`load_product` 复核 **5–6** 与 baseline 的可用性、predicate 的构造校验、readiness 的三元组
不变量（见 §4.9），违反即 `CorruptRow`。**1–4 是写路径不变量**，由
`establish_baseline`／`save_product` 核对（`load_product` 不跨表回查 `intent`），分别落
`IntentNotProduct`／`MissingProductId`／`InvalidContract`／`AnchorIntentNotReady` 等具体臂，
不静默修正。

**访问器**（只读）：

```
impl Product {
    pub fn id(&self) -> &ProductIdRef;
    pub fn intent_id(&self) -> &IntentId;
    pub fn baseline(&self) -> &EngineeringBaseline;
    pub fn readiness(&self) -> &ProductReadiness;
    pub fn current_milestone(&self) -> Option<&MilestoneId>;
    pub fn milestone_history(&self) -> &[MilestoneId];
    pub fn shippable_predicate(&self) -> &ShippablePredicate;
}
```

**没有 `set_baseline`／`set_shippable_predicate`**：本轮不可变（§2.3 第 16 条）。`readiness`
的更新只经 `record_milestone_progress`（§4.5）；`current_milestone` 的更新只经
`set_current_milestone`（§4.4）。

## 4.2 `ProductReadiness`（§232 九维）

```
/// §232 的九个维度。字段名逐字；每一项是 value/evidence/last_verified 三元组。
/// 没有聚合方法：`score()`／`average()`／`sum()` 在类型上不存在（§2.3 第 1 条）。
/// 不派生 `PartialEq`：`Assessed(Opaque)` 里的 `Opaque` 没有 `PartialEq`。
#[derive(Debug, Clone)]
pub struct ProductReadiness {
    functional: ReadinessDimension,
    correctness: ReadinessDimension,
    reliability: ReadinessDimension,
    performance: ReadinessDimension,
    security: ReadinessDimension,
    usability: ReadinessDimension,
    maintainability: ReadinessDimension,
    deployability: ReadinessDimension,
    documentation: ReadinessDimension,
}

/// 一维的记录。字段私有；`assessed` 是唯一能造出「有值」的构造点。
/// **只读三元组必须经 `value()`／`evidence()`／`last_verified()` 取**——这是
/// `MilestoneGenerator`（gap analysis）与 `ProgressPolicy`（delta）的唯一输入面（C1）。
#[derive(Debug, Clone)]
pub struct ReadinessDimension {
    value: ReadinessValue,
    evidence: Vec<EvidenceId>,
    last_verified: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum ReadinessValue {
    /// 尚未评估。`evidence` 必空、`last_verified` 必 `None`。
    Unassessed,
    /// 已评估。值是不透明串（量纲由调用方的策略解释）；`evidence` 非空、`last_verified` 为 `Some`。
    Assessed(Opaque),
}

/// 九维的名字；用于 required_dimensions、迭代与错误消息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessDimensionName {
    Functional, Correctness, Reliability, Performance, Security,
    Usability, Maintainability, Deployability, Documentation,
}
```

**构造与只读读面**：

```
impl ReadinessValue {
    /// 只读判别：`Unassessed` ⇒ true；`Assessed` ⇒ false。
    pub fn is_unassessed(&self) -> bool;
    /// 只读判别：`Assessed(v)` ⇒ `Some(v)`；`Unassessed` ⇒ `None`。
    /// **不提供 `as_f64`／`score`**：量纲由调用方解释，P7A 不比较。
    pub fn as_assessed(&self) -> Option<&Opaque>;
}

impl ReadinessDimension {
    pub fn unassessed() -> Self;
    pub fn assessed(
        value: Opaque,
        evidence: Vec<EvidenceId>,
        last_verified: i64,
    ) -> Result<Self, ProductError>;

    // 只读三元组。`ProductReadiness::iter()`／`dimension()` 拿到的引用经这三个方法
    // 才能被策略与生成器读取；没有它们，§7.3 的两条输入都落空（C1）。
    pub fn value(&self) -> &ReadinessValue;
    pub fn evidence(&self) -> &[EvidenceId];
    pub fn last_verified(&self) -> Option<i64>;
}

impl ProductReadiness {
    pub fn unassessed() -> Self;
    pub fn dimension(&self, name: ReadinessDimensionName) -> &ReadinessDimension;
    pub fn with_dimension(self, name: ReadinessDimensionName, d: ReadinessDimension) -> Self;
    pub fn iter(&self) -> [(ReadinessDimensionName, &ReadinessDimension); 9];
}
```

**不变量**（`assessed` 与 `record_milestone_progress` 两侧保证）：

1. `Assessed` ⇒ `value.as_str()` 非空、`evidence` 非空且无重复 id、`last_verified: Some`。
2. `Unassessed` ⇒ `evidence` 空、`last_verified: None`。
3. 九维字段一个不少、一个不多；顺序照 §232。
4. **没有聚合**；`iter()` 只用于展示／策略输入，不产生分数。
5. **`ProductReadiness` 没有 `Undetermined` 态**：未评是 `Unassessed`（记录值），
   「不可判」只在 `ShippableVerdict::Undetermined`／`ProgressUndetermined` 里表达。

**读面是契约的一部分**：`ReadinessDimension` 的三个 getter 是 `MilestoneGenerator`
（`MilestoneProposalInput.readiness`）与 `ProgressPolicy::delta` 的唯一读入口；
`ReadinessValue::as_assessed` 让策略能区分「未评」与「已评」。没有这三个 getter，
两者都只能拿到 `&ProductReadiness` 却读不出三元组，delta 只能是常量（C1／J3、J12、J13）。

**证据纪律**：`record_milestone_progress` 收 `&[Evidence]` 值，只为核对每个 `EvidenceId`
确实在这批证据里：遍历 `current.iter()`，对每维按 `evidence()` 取出 id 集合，再核对它们
都在传入的 `&[Evidence]` 里；**不读 `evidence` 表**（P5a 只写不读，`evidence.rs:117`）；
**不判** `claim`／`type`／`strength`／`scope`；落库只存 id。某条 evidence 是否真的支持
这一维，由调用方（readiness 生产者）负责；P7A 只保证「引用的 id 在这一批值里存在」
（§2.3 第 12 条）。

**为什么 `value` 是不透明串而不是 `f64`**：§232 没有量纲；给 `f64` 就是给九维发明一个
0..1 或 0..100 的刻度，并把「聚合」偷偷带进来。不透明串把解释权留给申报策略；代价是
P7A 自己不能比较两维（§2.3 第 1 条）。

## 4.3 `EngineeringBaseline`（§150 十字段）与生成

```
/// §150 的十个字段。字段名逐字；类型是本设计定的（§2.3 第 9 条）。
/// 不派生 `PartialEq`：`Opaque` 没有 `PartialEq`（`crates/continuum-canonical/src/lib.rs:151`），
/// 比较由消费方按 `as_str` 做。
#[derive(Debug, Clone)]
pub struct EngineeringBaseline {
    pub product_goal: Opaque,
    pub target_users: Vec<Opaque>,
    pub required_capabilities: Vec<Opaque>,
    pub hard_constraints: Vec<Opaque>,
    pub architecture: Opaque,
    pub technology_choices: Vec<Opaque>,
    pub minimum_complete_system: Vec<Opaque>,
    pub known_unknowns: Vec<Opaque>,
    pub engineering_risks: Vec<Opaque>,
    pub acceptance_criteria: Vec<Opaque>,
}

/// 生成端口的输入：只带产品意图的文本与已知约束；具体提示词不在 P7A。
#[derive(Debug, Clone)]
pub struct BaselineRequest {
    pub idea: Opaque,
    pub constraints: Vec<Opaque>,
}

/// §150 的模型判断端口。P7A 定义 trait；装配方 `continuum-runtime` 实现并转调
/// 既有模型调用路径（切分 §三第 5 条）。**P7A 不依赖 provider／runtime。**
#[async_trait::async_trait]
pub trait EngineeringBaselineGenerator {
    async fn generate(&self, request: &BaselineRequest) -> Result<EngineeringBaseline, PortError>;
}

/// 端口的错误：不透明消息；实现方把模型错误映射进来。
#[derive(Debug, Clone, PartialEq)]
pub struct PortError { message: String }

impl PortError {
    pub fn new(message: impl Into<String>) -> Self;
    pub fn message(&self) -> &str;
}
```

**生成与落库是两步，且不跨 `Tx` 持有 `.await`**：

```
/// 只做判断，不碰数据库。`intent` 是调用方已读出的锚 Product Intent 值。
/// 校验：kind == Product；product_id == Some 且非空；**status == READY**（§4.1 不变量 4；
/// 其余十二态 ⇒ `AnchorIntentNotReady`）；contract_id 非空且 version >= 1；
///       baseline 的 product_goal 与 minimum_complete_system 非空；predicate 合法。
pub async fn establish_baseline(
    intent: &Intent,
    request: &BaselineRequest,
    shippable_predicate: ShippablePredicate,
    generator: &dyn EngineeringBaselineGenerator,
) -> Result<Product, ProductError>;

/// 同步落库：重新 `load_intent` 核对锚 Intent（含 status == READY），再写 `product`
/// （以及 C2 裁定后的 drive 落点；同一 `Tx`；不 commit）。已存在同 id ⇒ `ProductExists`。
/// **C2 裁定前不应用迁移 200，也不存在「只写 product」的三表先行版本**（§2.1、§8 第 12 条）。
/// 裁定 (b) ⇒ 同 Tx 写 `product_drive`，行以 `continuous_product_drive = false`、
/// `phase = PreShippable` 初始化；裁定 (a) ⇒ §4.7 的独立表与 API 作废，drive 的读/写改由
/// C2 裁定的既有面承担（具体表／列／更新 API 随裁定给出；§4.7 的默认值不变）。
pub fn save_product(tx: &Tx<'_>, product: &Product, occurred_at: i64)
    -> Result<(), ProductError>;
```

**为什么拆两步**：`Tx` 持有 `MutexGuard`（`crates/continuum-persist/src/tx.rs:12-22`），
跨 `.await` 持有它不是 `Send`，也会把数据库锁带进模型调用。`establish_baseline` 不拿
`Tx`；`save_product` 不调模型。两段之间的并发由 `save_product` 的主键与重读锚 Intent
兜底（失败即 `Err`，调用方回滚）。

**基线不可用**：`generator.generate` 返回 `Err`，或 `product_goal`／
`minimum_complete_system` 为空 ⇒ 不产生 `Product`。没有 Product 行时，后续
`evaluate_shippable(None, ..)` 返回 `Undetermined`（§4.6）；**不得**用
`ProductReadiness::unassessed()` 冒充已评估。

**锚 Intent 的 `READY` 要求**：`establish_baseline` 与 `save_product` 都要求锚 Intent
`status == READY`（§4.1 不变量 4）。这与 §8 第 10 条的缺口同向：`PROPOSED → READY` 的审查
链未接线前，P7A 实际拿不到可建 Product 的锚 Intent。**这是值的前置，不是 P7A 偷偷推进
状态**；P7A 仍不调用任何 `PROPOSED → READY` 的迁移。

## 4.4 `Milestone`（§234 七字段）与惰性生成

```
/// §234 的七个字段。字段名逐字；**字段私有**——唯一的公开产生路径是
/// `propose_next_batch`（生成）与 `load_milestone(s)`（读回）；外部写不出
/// `Milestone { .. }`，否则「只生成一个有界批次」可以被绕过。
/// 不派生 `PartialEq`：`objective`／`entry_conditions`／`exit_conditions` 里的
/// `Opaque` 没有 `PartialEq`。
#[derive(Debug, Clone)]
pub struct Milestone {
    id: MilestoneId,
    product_id: ProductIdRef,
    objective: Opaque,
    dependencies: Vec<MilestoneId>,
    entry_conditions: Vec<Opaque>,
    exit_conditions: Vec<Opaque>,
    state: MilestoneState,
}

impl Milestone {
    pub fn id(&self) -> &MilestoneId;
    pub fn product_id(&self) -> &ProductIdRef;
    pub fn objective(&self) -> &Opaque;
    pub fn dependencies(&self) -> &[MilestoneId];
    pub fn entry_conditions(&self) -> &[Opaque];
    pub fn exit_conditions(&self) -> &[Opaque];
    pub fn state(&self) -> MilestoneState;
}

/// §234 的七个状态；编码用小写、多词下划线（与 P4 的枚举列纪律同形）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneState {
    Proposed, Approved, Active, Verifying, Completed, Failed, Superseded,
}

/// P7A 的 Milestone 身份。不透明串。**`Milestone` 的 id 由 `propose_next_batch` 赋**；
/// `MilestoneId::from_str` 只供读回与测试构造，不是生产入口。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MilestoneId(String);

impl MilestoneId {
    pub fn as_str(&self) -> &str;
    /// 只供存储解码与测试构造。生产路径由 `propose_next_batch` 派生
    /// （`milestone/{product_id}/{ordinal}`，`ordinal = history.len() + i`）。
    pub fn from_str(s: &str) -> Self;
}
```

**生成端口**（模型判断经端口；P7A 不调 provider）：

```
/// 一个批次的上界。显式参数、无默认；`new` 拒绝 max_milestones == 0。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchBoundary {
    max_milestones: usize,
    max_dependency_depth: u32,
}

impl BatchBoundary {
    /// max_milestones == 0 ⇒ `InvalidBatchBoundary`；max_dependency_depth 可为 0
    /// （只允许批内无依赖的并行项）。没有默认边界。
    pub fn new(max_milestones: usize, max_dependency_depth: u32)
        -> Result<Self, ProductError>;
    pub fn max_milestones(&self) -> usize;
    pub fn max_dependency_depth(&self) -> u32;
}

/// 生成端口的输入。`history` 只含**已生成**的 Milestone，不含未来。
/// `readiness` 的 gap analysis 必须经 `iter()`＋`ReadinessDimension::value()`／
/// `evidence()`／`last_verified()` 读取（C1）；生成器拿到的不是只有字段名的空壳。
pub struct MilestoneProposalInput<'a> {
    pub product_id: &'a ProductIdRef,
    pub baseline: &'a EngineeringBaseline,
    pub readiness: &'a ProductReadiness,
    pub history: &'a [Milestone],
    pub boundary: BatchBoundary,
}

/// 单枚提案：没有 id、没有 product_id、没有 state——这三样由 P7A 赋。
/// `entry_conditions`／`exit_conditions` 的每个元素必须非空（`Opaque::from_str("")` 会让
/// `completion_predicate` 的空编码失去唯一性，§4.8）；由 `propose_next_batch` 与
/// `save_milestones` 拒绝（`EmptyMilestoneCondition`）。
#[derive(Debug, Clone)]
pub struct MilestoneDraft {
    pub objective: Opaque,
    pub dependencies: Vec<MilestoneDependency>,
    pub entry_conditions: Vec<Opaque>,
    pub exit_conditions: Vec<Opaque>,
}

/// 依赖只能是「已生成的历史」或「本批次里更早的一枚」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilestoneDependency {
    Existing(MilestoneId),
    InBatch(usize),
}

#[derive(Debug, Clone)]
pub struct MilestoneBatchDraft {
    pub milestones: Vec<MilestoneDraft>,
}

#[async_trait::async_trait]
pub trait MilestoneGenerator {
    async fn propose(
        &self,
        input: &MilestoneProposalInput<'_>,
    ) -> Result<MilestoneBatchDraft, PortError>;
}
```

**生成入口（异步，不碰 `Tx`）**：

```
/// 校验：批次非空；`len <= boundary.max_milestones()`；每枚 objective 非空；
/// 每枚的 `entry_conditions`／`exit_conditions` 元素非空（否则 `EmptyMilestoneCondition`）；
/// `InBatch(i)` 只指向更早的 `i`（无自环、无环）；`Existing(id)` 必须在 `history` 里；
/// 批内依赖深度 `<= boundary.max_dependency_depth()`；`history` 每枚的 product_id
/// 必须等于 `product.id`，且按 `seq` 升序（调用方从 `load_milestones` 取）。
/// 赋 id（`milestone/{product_id}/{ordinal}`）与 state = PROPOSED，返回有序 Vec。
/// **`ordinal` 是全局序号，不是批内下标：`ordinal = history.len() + i`**（`i` 是批次内
/// 从 0 起的下标；`history` 是 `load_milestones` 的全量、按 `seq` 升序）。这样第一批
/// 得 0/1、第二批得 2/3，不会撞 `MilestoneIdCollision`。超界 ⇒
/// `BatchTooLarge`／`DependencyDepthExceeded`，**不截断**。
pub async fn propose_next_batch(
    product: &Product,
    history: &[Milestone],
    boundary: BatchBoundary,
    generator: &dyn MilestoneGenerator,
) -> Result<Vec<Milestone>, ProductError>;
```

**落库（同步）**：

```
/// **开头先 `load_product(tx, product.id())`；不存在 ⇒ `UnknownProduct`，零行写入**
/// （防止调用方拿一个未 save 的 Product 值写出孤儿 Milestone）。
/// 同一 Tx 写 `milestone` 的每一行；seq = 当前 MAX(seq)+1 起；并要求每枚 id 的
/// `ordinal` 后缀（按最后一个 `/` 取）与该 seq 一致（不一致 ⇒ `MilestoneOrdinalMismatch`）；
/// 每枚的 product_id 必须等于 product.id、state 必须为 PROPOSED、id 不得已存在
/// （否则 `MilestoneProductMismatch`／`MilestoneStateMismatch`／`MilestoneIdCollision`）；
/// 每枚 objective 非空（否则 `EmptyMilestoneObjective`）；
/// 每枚的 `entry_conditions`／`exit_conditions` 元素非空（否则 `EmptyMilestoneCondition`）；
/// 写之前检查：该 Product 的每个 COMPLETED Milestone 都已有 `product_progress` 行，
/// 否则 `MissingProgressForCompleted`（§2.3 第 17 条的评估门）；
/// 若 product.current_milestone 为 NULL，设为批次第一枚；把 product.updated_at
/// 置为 `occurred_at`。
/// 任一行失败，调用方回滚；不 commit。
pub fn save_milestones(
    tx: &Tx<'_>,
    product: &Product,
    milestones: &[Milestone],
    occurred_at: i64,
) -> Result<(), ProductError>;
```

**状态更新（不是状态机）**：

```
/// 乐观并发：当前值必须等于 expected；只写 state 与 updated_at。
/// **不判**七状态之间的合法迁移（§234 没有迁移表；§2.3 第 13 条）。
/// 若 `to` 是终态（Completed／Failed／Superseded）且它正是 `current_milestone`，
/// 同一 Tx 内把游标推进到按 `seq` 的第一个非终态 Milestone（没有则 NULL），
/// 否则 `load_product` 的不变量 6 会被下一次读打破。
pub fn record_milestone_state(
    tx: &Tx<'_>,
    id: &MilestoneId,
    expected: MilestoneState,
    to: MilestoneState,
    occurred_at: i64,
) -> Result<(), ProductError>;
```

**游标**：

```
/// `Some` 时要求 Milestone 属于本 Product 且不处于 Completed／Failed／Superseded；
/// 写 `current_milestone` 与 `product.updated_at`。
pub fn set_current_milestone(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
    milestone: Option<&MilestoneId>,
    occurred_at: i64,
) -> Result<(), ProductError>;
```

**惰性生成的边界（据实写清）**：

- **可测**：批次不超过申报上界；批内依赖深度不超过申报上界；超过即拒绝且零行写入；
  生成器的输入只含已生成历史；不存在「只给 baseline 就返回全部 Milestone」的入口。
- **不可测／规范未给**：规范级的批次大小；调用方申报一个巨大上界时，P7A 拦不住（§2.3
  第 4 条）。`§7.4` 判据 1 的照片因此只钉「不出现完整预定义列表」与「评估后再生成」，
  不钉批次大小（切分 §6.1 `:394`）。
- **并行／汇合**：`dependencies` 允许一个 Milestone 依赖多枚（汇合）与多枚依赖同一枚
  （分叉）；`InBatch(i)` 只指向更早项，保证批次内的 DAG 可拓扑排序。图边留在
  `milestone` 表；`Intent` 侧不复制（§8 第 6 条）。

## 4.5 `Progress Gate`、`progress_delta` 与 replan `Decision`

```
/// §337 的三值判定。`Triggered` 只在有申报策略、阈值有效、且连续无进展达到阈值时给出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressGateVerdict {
    Triggered { consecutive_no_progress: u32, threshold: u32 },
    NotTriggered { consecutive_no_progress: u32, threshold: u32 },
    Undetermined { reason: ProgressUndetermined },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressUndetermined {
    NoPolicy,
    InvalidThreshold,
    ReadinessUnassessed,
    DeltaUndetermined,
    HistoryUndetermined,
}

/// §154 的 `progress_delta`：四值。计算由 `ProgressPolicy` 给；P7A 只计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressDelta {
    Improved,
    NoProgress,
    Regressed,
    Undetermined,
}

/// §154 的记录：previous_product_state／current_product_state／progress_delta。
/// 不派生 `PartialEq`：两个 readiness 字段里的 `Opaque` 没有 `PartialEq`。
#[derive(Debug, Clone)]
pub struct ProgressRecord {
    pub product_id: ProductIdRef,
    pub milestone_id: MilestoneId,
    pub previous_product_state: ProductReadiness,
    pub current_product_state: ProductReadiness,
    pub progress_delta: ProgressDelta,
    pub recorded_at: i64,
}

/// 调用方申报的 delta 与阈值。**P7A 不提供默认实现**。
pub trait ProgressPolicy {
    fn delta(&self, previous: &ProductReadiness, current: &ProductReadiness) -> ProgressDelta;
    fn stall_threshold(&self) -> u32;
}
```

**纯判定**：

```
/// 算法（本设计定，§2.3 第 2、3、14 条）。`history` 按 `recorded_at`／插入序升序
/// （`load_progress` 的返回序）；最后一条是最近的一条；**必须严格早于本次评估**：
/// 不含本次由 `previous/current` 与 `policy.delta` 代表的 `record`，也不含任何
/// 与其 `(product_id, milestone_id)` 相同的记录（step 6 才把本次计入一次）。
/// 1. policy 为 None ⇒ Undetermined(NoPolicy)。
/// 2. threshold == 0 ⇒ Undetermined(InvalidThreshold)。
/// 3. previous/current 全为 Unassessed ⇒ Undetermined(ReadinessUnassessed)。
/// 4. delta = policy.delta(previous, current)；delta == Undetermined ⇒ Undetermined(DeltaUndetermined)。
/// 5. 当前 delta == Improved/Regressed ⇒ NotTriggered，count = 0（不再看历史）。
/// 6. 当前 delta == NoProgress ⇒ count 从 1 起；从最近一条历史往回加 NoProgress，
///    遇到 Improved/Regressed 停止；若在达到 threshold 之前遇到 Undetermined，
///    ⇒ Undetermined(HistoryUndetermined)。
/// 7. 任一时点 count >= threshold ⇒ Triggered{count, threshold}；否则 NotTriggered{count, threshold}。
/// 8. `Regressed` 不直接触发 replan（§2.3 第 14 条）。
pub fn evaluate_progress(
    previous: &ProductReadiness,
    current: &ProductReadiness,
    history: &[ProgressRecord],
    policy: Option<&dyn ProgressPolicy>,
) -> ProgressGateVerdict;
```

**记录与判定（同步，同一 `Tx`）**：

```
/// 前置：Milestone 属于本 Product 且 state == COMPLETED；同一 (product, milestone)
/// 不得已有 ProgressRecord（重复 ⇒ `DuplicateProgress`）。
/// 收 `&[Evidence]` 只为核 `current` 里每个 EvidenceId 的存在性；
/// 不读 evidence 表、不判 claim/type/strength。
/// 同一 Tx：写 `product_progress` 一行、把 `product.readiness` 更新为 current、
/// 把 `product.updated_at` 置为 `occurred_at`；
/// policy 为 None 时 `progress_delta` 写 `Undetermined`（fail-closed，不猜）；
/// policy 为 Some 时 `progress_delta` 写 `policy.delta(previous, current)`——即使
/// `evaluate_progress` 因 readiness 不可判而返回 `Undetermined`，记录仍保留策略的输出。
/// 若 `product.current_milestone == Some(milestone.id())`，把游标推进到按 `seq` 的
/// 第一个非终态 Milestone（没有则置 NULL）——否则游标会停在刚完成的终态上，违反
/// §4.1 不变量 6；并行批次里即使完成的是最后一枚，也能找到前面的非终态项。
/// verdict 用**写入前**的历史计算（该 history 不含本次将写入的行）；不得先写库、
/// 再把 `load_progress` 的全量当作 verdict 的 history——那会把本行重复计入。
/// 返回记录与 verdict；不 commit。
pub fn record_milestone_progress(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &Milestone,
    current: ProductReadiness,
    evidence: &[Evidence],
    policy: Option<&dyn ProgressPolicy>,
    occurred_at: i64,
) -> Result<(ProgressRecord, ProgressGateVerdict), ProductError>;
```

**replan 的 `Decision`（只出，不执行）**：

```
/// 调用方给的 Decision 内容；P7A 不生成路线，也不改 Contract/Plan。
#[derive(Debug, Clone)]
pub struct ReplanDraft {
    pub issue: Opaque,
    pub options: Vec<DecisionOption>,
    pub blocking: bool,
}

/// **不接收 `ProgressGateVerdict`**：verdict 是公开枚举，任何调用方都能伪造
/// `Triggered{consecutive_no_progress: 99, threshold: 1}`（I8）。本函数用
/// `record.previous/current_product_state`＋`history`＋`policy` **重新调用
/// `evaluate_progress`**，只信重算结果。**`history` 必须排除 `record`**：调用方
/// 若把 `load_progress` 的全量直接传入，实现必须按 `milestone_id` 去重（`record`
/// 优先），同一条 NoProgress 只能计一次。阈值 3、实际连续 2 ⇒ `NotTriggered{2,3}`，
/// 不得因重复计数误触发。
/// - `Triggered` ⇒ 出 `Decision`，返回 `Some`；
/// - `NotTriggered`／`Undetermined` ⇒ **零行、零事件**，返回 `Ok(None)`（不是错误，
///   也不允许调用方用一个自报的 Triggered 绕过）。
/// 其余前置：record 必须属于 product（product_id 相等、milestone 属于本 Product）；
/// draft.options 非空、draft.issue 非空。Decision.id 由 P7A 派生：
///   `decision/{product_id}/replan/{milestone_id}`
/// intent_id = product.intent_id（锚 Product Intent）；options/issue/blocking 原样取 draft。
/// 调 `continuum_semantics::create_decision`（`persist.rs:1173`），由它写 `decision.required`。
/// 重复调用会撞 `decision` 主键（fail-closed），不会产生第二张单。
pub fn emit_replan_decision(
    tx: &Tx<'_>,
    product: &Product,
    record: &ProgressRecord,
    history: &[ProgressRecord],
    policy: Option<&dyn ProgressPolicy>,
    draft: ReplanDraft,
    occurred_at: i64,
) -> Result<Option<Decision>, ProductError>;
```

**`record_milestone_progress` 返回的 verdict 只是提示**：它不能被当作 `emit_replan_decision`
的输入（签名里没有这个参数），因此「伪造 NotTriggered」「计数写错」都不会产生 Decision；
要产生 Decision，必须让重算用的 `record`／`history`／`policy` 真的达到阈值。

**为什么 options 由调用方给**：§154 的例子（A 重构渲染架构／B 接受性能／C 改目标）是
**路线选择**，那是第 3 层 Planner 的活；P7A 若自己编选项，就是建了一个 Planner（切分 §五
P7A「不执行 replan」）。P7A 只决定「现在必须停下来重新分析」，并把它做成一张结构化
`Decision`；消费方见 §6.4（具名缺口）。**P7A 不创建 Plan／Contract 的新版本，不重跑
Intent Compiler。**

## 4.6 `ShippablePredicate`、`ShippableVerdict` 与 post-shippable 的判定输入

```
/// §335 的谓词。规范没给字段（§2.3 第 7 条）；本设计取：
/// - required_dimensions：至少一维；每一维必须已 Assessed；
/// - clauses：至少一条；每条有稳定 id 与不透明 statement。
/// 空谓词／空子句／重复 dimension／重复 clause id／空 clause id／空 statement 一律拒绝。
#[derive(Debug, Clone)]
pub struct ShippablePredicate {
    required_dimensions: Vec<ReadinessDimensionName>,
    clauses: Vec<ShippableClause>,
}

#[derive(Debug, Clone)]
pub struct ShippableClause {
    id: ShippableClauseId,
    statement: Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShippableClauseId(String);

impl ShippableClauseId {
    pub fn as_str(&self) -> &str;
    pub fn from_str(s: &str) -> Self;
}

/// 调用方对每条子句的判定；**P7A 不评估子句内容**。
#[derive(Debug, Clone, PartialEq)]
pub struct ShippableAssessment {
    pub clauses: Vec<ClauseAssessment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClauseAssessment {
    pub clause_id: ShippableClauseId,
    pub verdict: ClauseVerdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseVerdict {
    Satisfied,
    NotSatisfied,
    Undetermined,
}

/// 三值 fail-closed。`Undetermined` 不是许可。
#[derive(Debug, Clone, PartialEq)]
pub enum ShippableVerdict {
    Shippable,
    NotShippable { reason: NotShippableReason },
    Undetermined { reason: ShippableUndetermined },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotShippableReason {
    ClauseNotSatisfied(ShippableClauseId),
    CompletionBlocked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShippableUndetermined {
    MissingProduct,
    /// Product 在但 baseline 不可用（product_goal／minimum_complete_system 空）。
    /// 正常读路径由 `load_product` 先行判 `CorruptRow`；这是 defense-in-depth。
    BaselineUnusable,
    AssessmentMismatch,
    RequiredDimensionUnassessed(ReadinessDimensionName),
    ClauseUndetermined(ShippableClauseId),
    CompletionUndetermined,
}
```

**判定**：

```
/// 顺序（本设计定，§2.3 第 8 条；**`MissingProduct` 是前置；其余「明确不成立」先于
/// 「结构错误／不可判」**）：
/// 1. product 为 None（无 Product）⇒ Undetermined(MissingProduct)。**前置：先于所有
///    其他分支**——无 Product 时没有可判定的对象，即使 completion == Blocked 也不改判。
/// 2. completion == Blocked ⇒ NotShippable(CompletionBlocked)。
/// 3. 任一子句 NotSatisfied ⇒ NotShippable(ClauseNotSatisfied)（多枚不满足时按
///    predicate.clauses 的顺序取第一枚；assessment 里多出的重复项不能遮蔽这一条；
///    同一 clause id 有多条 assessment 时，任一 NotSatisfied ⇒ 该子句按 NotSatisfied 处理）。
/// 4. product 的 baseline 不可用（product_goal／minimum_complete_system 空）⇒
///    Undetermined(BaselineUnusable)。正常路径到不了这里（`load_product` 已判
///    `CorruptRow`），这是内存构造的 defense-in-depth。
/// 5. assessment 与 predicate.clauses 不是一一对应（缺／多／重复）⇒ Undetermined(AssessmentMismatch)。
/// 6. 任一 required dimension 不是 Assessed ⇒ Undetermined(RequiredDimensionUnassessed)
///    （按 required_dimensions 的顺序取第一枚）。
/// 7. 任一子句 Undetermined ⇒ Undetermined(ClauseUndetermined)（按 clauses 顺序取第一枚）。
/// 8. completion == Undetermined ⇒ Undetermined(CompletionUndetermined)。
/// 9. 若未来 completion 有正臂，则 Shippable；**今天没有第 9 步的实现**。
///
/// **这就是 C4 的修法**：第 1 步是 `MissingProduct` 前置，`None + Blocked` 仍返回
/// `Undetermined(MissingProduct)`（J15 的组合负例 C）；第 2、3 步先于第 5 步，所以
/// 「assessment 缺一条／多一条＋CompletionVerdict::Blocked／clause NotSatisfied」会得到
/// `NotShippable`，而不是 `Undetermined`。第 3 步按 predicate.clauses 查 assessment，缺项不会把 NotSatisfied
/// 藏掉；重复项也不会改变「某一子句明确不满足」这一事实。
///
/// 今天 `CompletionVerdict` 只有 Blocked／Undetermined 两臂（completion.rs:24）。
/// 实现必须在入口用一个**穷尽**的 match 处理 completion：`Blocked` 立即进第 2 步返回
/// `NotShippable`；`Undetermined` 只记下、留到第 8 步。**没有任何一行返回 `Shippable`**。
/// P5a 若加正臂，这个 match **编译不过**，实现者必须显式决定正臂在第几步参与——这是
/// 刻意留的编译期 tripwire，不是靠注释提醒。
pub fn evaluate_shippable(
    product: Option<&Product>,
    assessment: &ShippableAssessment,
    completion: &CompletionVerdict,
) -> ShippableVerdict;
```

**谓词构造**：

```
impl ShippablePredicate {
    pub fn new(
        required_dimensions: Vec<ReadinessDimensionName>,
        clauses: Vec<ShippableClause>,
    ) -> Result<Self, ProductError>;
    pub fn required_dimensions(&self) -> &[ReadinessDimensionName];
    pub fn clauses(&self) -> &[ShippableClause];
}

impl ShippableClause {
    pub fn new(id: ShippableClauseId, statement: Opaque) -> Result<Self, ProductError>;
    pub fn id(&self) -> &ShippableClauseId;
    pub fn statement(&self) -> &Opaque;
}
```

**为什么 `required_dimensions` 至少一维**：§232 的九维是 ProductReadiness 的全部；一个
「一维都不要求」的发布谓词等于把 readiness 整个跳过，与 §335 的「达到后才能宣告」方向
相反。代价：不关心某维的产品仍要在 `required_dimensions` 里点名它，或者把它排除、由
子句覆盖（§2.3 第 7 条）。**这是一条本设计的完整性规则，不是规范明文。**

**今天能拍到哪一步**：`Shippable` 的三条必要条件（required dimensions、clauses、
completion）都能测；但**正向分支没有实现**，所以「正向宣告 SHIPPABLE」**没有生产照片**。
`NotShippable` 与 `Undetermined` 的互不覆盖可拍（§5.2 J15）。**不把「三值类型存在」
写成「可发布状态已闭合」。**

## 4.7 `ContinuousProductDrive` 与 `ProductPhase`（落点：提案，待协调）

**落点状态**：切分 §2.3.1 `:132-133` 只给 `Product`／`ProductReadiness`／verdict 三处；
本节把开关与相位放进**独立的 `ProductDriveState`**（`product_drive` 表），是**第四种
落点**，只能作为**切分变更**处理：协调者先裁定并更新切分 §2.3.1，本节的表与 API 才落库
（§8 第 12 条）。**裁定前本节是提案，不是已定接口；J17 分两段：J17a 相位机（`advance_product_phase` 纯函数）无条件可拍；J17b 落库（`record_shippable_evaluation`／`begin_post_shippable_work`／`set_continuous_product_drive`）与 drive gate 交接以 C2 裁定并落库为前提，裁定前不拍；其中 J17b 的运行期断言与源码面照片只属 (b)。若 C2 选 (a)，J17b 整体延后到裁定给出既有面的读/写 API 与列后再写。**

```
/// §336 的用户开关。默认 false；只在 Product 创建后改变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuousProductDrive {
    pub enabled: bool,
}

/// §155／§336 的生命周期相位。三值互斥；**只前进，不回退**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPhase {
    PreShippable,
    Shippable,
    PostShippableImprovement,
}

/// 开关＋相位；与 §231 的 Product 分开存（`product_drive` 表），
/// **不是 §231 的第八个字段**（§2.3 第 5 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductDriveState {
    pub continuous_product_drive: ContinuousProductDrive,
    pub phase: ProductPhase,
}
```

**纯相位推进**：

```
/// 只前进：
/// - PostShippableImprovement ＋ 任何 verdict ⇒ PostShippableImprovement；
/// - Shippable ＋ 任何 verdict ⇒ Shippable（不回退到 PreShippable）；
/// - PreShippable ＋ Shippable ⇒ Shippable；
/// - PreShippable ＋ NotShippable/Undetermined ⇒ PreShippable。
pub fn advance_product_phase(
    current: ProductPhase,
    verdict: &ShippableVerdict,
) -> ProductPhase;
```

**落库与 post-shippable 的进入**：

```
/// 读 drive；phase = advance_product_phase(current, verdict)；写回（若变）。
/// 不存 verdict 本身：Shippable 是相位，不是一张评分表。
pub fn record_shippable_evaluation(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
    verdict: &ShippableVerdict,
    occurred_at: i64,
) -> Result<ProductDriveState, ProductError>;

/// 前置：milestone 属于本 Product 且不处于终态。判定顺序：
/// phase == PostShippableImprovement ⇒ 幂等 Ok（不再看开关）；
/// phase == PreShippable ⇒ `NotShippableYet`；
/// phase == Shippable 且开关关闭 ⇒ `DriveDisabled`；
/// phase == Shippable 且开关开启 ⇒ 写 phase = PostShippableImprovement。
pub fn begin_post_shippable_work(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
    milestone: &MilestoneId,
    occurred_at: i64,
) -> Result<ProductDriveState, ProductError>;

/// 开关的写入口；不改变 phase。
pub fn set_continuous_product_drive(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
    enabled: bool,
    occurred_at: i64,
) -> Result<ProductDriveState, ProductError>;
```

**为什么提案取独立状态而不是第八个字段**：§231 的字段清单是七项，加第八项会破坏冻结；
`ProductReadiness` 是九维评分，放一个生命周期开关会把「评分」与「用户设置」混在一起；
`ShippableVerdict` 是单次判定，放一个持久开关会让同一个 verdict 在不同时刻含义不同。
故**提案**取独立的 per-product 状态，键是 `ProductIdRef`（§2.3 第 5 条）。**这不是
「本设计可以自己定」的落点**：切分只授权了三种既有面；协调者若选 (a)（`Product` 面需先
解 §231 七字段冻结，或 `ProductReadiness`／verdict 面），本节的独立 API 与 200 的第四张
表一并作废，drive 的读/写改由 C2 裁定的既有面承担（§4.9；具体表／列／更新 API 随裁定
给出）。**J17b 的运行期断言与源码面照片只属 (b)**；选 (a) 时延后到裁定给出该面的读/写
API 后再写，本节不预写 `grep "phase = ?"` 对既有面的命中（§8 第 12 条）。

**今天的可达性**：`ShippableVerdict::Shippable` 没有生产者（§4.6），所以
`record_shippable_evaluation` 永远只能把相位留在 `PreShippable`。`Shippable` 与
`PostShippableImprovement` 的**类型区分**与**不回退**可拍；**进入 Shippable 的生产路径
不可拍**（§8 第 3 条）。J17a 用一个**合成的** `ShippableVerdict::Shippable` 测相位机，
并明确标注它不是 `evaluate_shippable` 的产物；J17b 的落库／交接段以 C2 选 (b) 并落库为前提，(a) 下整段延后。

**限度**：相位是**产品级标签**，P7A 不从 Milestone 的内容判断它是不是 post-shippable；
调用方必须在第一项 post 工作开始时调 `begin_post_shippable_work`。不调时相位停在
`Shippable`，那项工作仍可作为普通 Milestone 存在——「区分」在类型上成立，「每一项工作都
被正确归类」不在 P7A 的强制面里（§8 第 12 条）。

## 4.8 `Milestone → Product Intent` 的 lower 与交接

**输出形状**（§222 的 `Intent`，不新造类型）：

```
/// 前置：`milestone.product_id().as_str() == product.id().as_str()`，否则
/// `MilestoneProductMismatch`（`Milestone` 的 product_id 有 getter，见 §4.4）。
/// 不跑 Canonicalizer；`goal` 为空 ⇒ `EmptyMilestoneObjective`。
pub fn lower_milestone(
    product: &Product,
    milestone: &Milestone,
    contract: ContractRef,
    priority: Priority,
    created_at: i64,
    updated_at: i64,
) -> Result<Intent, ProductError>;
```

**逐字段**：

| §222 字段 | lower 的值 | 依据 / 说明 |
|---|---|---|
| `id` | `IntentId::from_str("intent/{milestone_id}")` | 确定构造；见下 |
| `version` | `1` | 写侧被 `save_intent` 忽略（`persist.rs:353` 的版本规则）；读侧权威 |
| `kind` | `IntentKind::Product` | 切分 §三.6 `:215`；不是 Task |
| `goal` | `EntityId::from_str(milestone.objective().as_str())` | 不跑 Canonicalizer；objective 为空 ⇒ `EmptyMilestoneObjective` |
| `status` | `IntentStatus::Proposed` | lower 只到 PROPOSED；到 READY 要过 §223 的审查链 |
| `priority` | 调用方给的 `Priority` | 不解释、不改写 |
| `created_at` / `updated_at` | 调用方给 | 本层不取时钟（同 P4） |
| `parent_intent` | `Some(product.intent_id.clone())` | 锚 Product Intent；子列表由 `list_children` 派生 |
| `contract_id` | 调用方给的 `ContractRef` | 必须 `id` 非空、`version >= 1`；否则 `InvalidContract` |
| `active_plan_id` | `None` | lower 不建 Plan |
| `graph_id` | `None` | lower 不建 ADFIR |
| `product_id` | `Some(product.id.clone())` | 切分 §三.6；与 `Product.id` 同一枚 |
| `completion_predicate` | `exit_conditions` 的确定性编码（按序、换行连接；**零条 ⇒ `Opaque::from_str("")`**；空元素已在 §4.4 被 `EmptyMilestoneCondition` 拒绝，故 `""` 唯一对应「没有 exit conditions」） | §2.3 第 10 条；P4 不解析它。**跨块约定**：P5a 必须把 `""` 映射为 `declared_completion_predicate = None`（§6.2）；`Some("")` 在 P5a 侧算「有内容」，不能直接透传（`completion.rs:70-76`、`:213-214`） |

**`id` 的确定性**：`intent/{milestone_id}`。`MilestoneId` 本身是
`milestone/{product_id}/{ordinal}`。两枚 Milestone 不会得到同一个 Intent id；同一枚
Milestone 重复 lower 得到同一个 id。**不借用 `IntentId` 的语义做解析**——它是不透明串。

**落库与幂等**：

```
/// **不接受调用方构造的 `Intent`**：函数内部用同一组入参重新调用 `lower_milestone`，
/// 因而 lower 的每一条构造规则（id／kind／goal／status=PROPOSED／product_id／
/// parent_intent／contract／completion_predicate）都在这里重新执行一遍——这是唯一写路径，
/// 不存在比 `lower_milestone` 更宽的入口（C3）。
/// 若同 id 的 Intent 已存在（字段比较一律按 `as_str()`／`ContractRef` 的 id＋version；
/// `Opaque`／`EntityId`／`ProductIdRef` 没有 `PartialEq`）：
/// - kind/goal/product_id/parent_intent/contract_id/completion_predicate 全同 ⇒ 返回现有值，
///   **不写**（保留它可能已经推进的 status：READY／RUNNING／COMPLETED 都不覆盖）；
/// - 任一不同 ⇒ `IntentIdCollision`。
/// 不存在 ⇒ `save_intent`（`persist.rs:353`）首次落库，写 `intent.created`。
/// **不直接写 intent 表**；不 commit。
pub fn save_lowered_intent(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &Milestone,
    contract: ContractRef,
    priority: Priority,
    created_at: i64,
    updated_at: i64,
) -> Result<Intent, ProductError>;
```

**为什么签名里没有 `Intent`**：`Intent` 的字段全是 `pub`（`intent.rs:434`），任何调用方都能
构造一枚 `status = READY` 的 Product Intent。若把 `&Intent` 收进来，即使逐项校验六个字段，
`status` 仍是调用方可写的；保留它就是第二条比 `lower_milestone` 更宽的写路径。新签名只收
`&Product`＋`&Milestone`＋`ContractRef`＋`Priority`＋时间戳，**status 由函数内的
`lower_milestone` 固定为 `PROPOSED`**。外部把 `READY`／`RUNNING`／`COMPLETED` 的 `Intent`
传进来在类型层就编不过（`trybuild` 照片，J10）。

**交接：只交 READY**：

```
/// **按 MilestoneId 取，不再只取当前游标**：一个非当前 Milestone 的 READY Intent
/// 也能交出去（§152 允许并行；§156 允许后台继续低风险 Milestone；I3）。
/// `milestone` 必须属于 `product`：
/// - Milestone 行不存在 ⇒ `UnknownMilestone`；
/// - `milestone.product_id().as_str() != product.id().as_str()` ⇒ `MilestoneProductMismatch`；
/// - 派生 id 的 Intent 不存在 ⇒ Ok(None)（还没 lower，不是错误）；
/// - 存在但 kind != Product 或 product_id != product.id 或 contract 无效
///   （id 空／version 0）⇒ `InvalidLoweredIntent`；
/// - status != READY ⇒ Ok(None)（drive 不必读）；
/// - **drive gate（I4）**：只在 `status == READY` 之后查。`load_product_drive` 缺失 ⇒
///   `MissingDriveState`；`phase == PostShippableImprovement && !continuous_product_drive.enabled`
///   ⇒ Ok(None)（用户关闭开关后不再把 post 工作交给 P7D）；
/// - `status == READY` 且 drive gate 放行 ⇒ Ok(Some(intent))。
/// P7D 按值消费它；P7D 不读 Product 记录（切分 §三.6 `:216-218`）。
/// 调用方（装配方）对 `milestone_history`／`load_milestones` 的每一枚 id 各调一次；
/// 两个 READY 时两次都返回 `Some`，由 P7D 决定先做哪一枚（P7A 不排名）。
pub fn product_intent_for_handoff(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &MilestoneId,
) -> Result<Option<Intent>, ProductError>;
```

**COMPLETED 的处置（只调公开状态机，不改状态机）**：

```
/// 唯一的 Intent 状态迁移调用点：COMPLETED → EVOLVING（仅 Product）。
/// 前置：kind == Product、status == COMPLETED。`event_id` 由调用方给（唯一性由调用方保证；
/// 该迁移在 Product 的生命周期里可能发生多次，不能派生一个固定 id）。
/// 调 `continuum_semantics::apply_intent_transition`（`intent_tx.rs:78`）；
/// 不自己写 intent 表、不改迁移表。
pub fn retire_completed_product_intent(
    tx: &Tx<'_>,
    intent_id: &IntentId,
    occurred_at: i64,
    event_id: &str,
) -> Result<(), ProductError>;
```

**PROPOSED → READY 不由 P7A 推进**：`SPECIFYING → PLANNING → REVIEW → READY` 的审查门
（`intent.rs:249` 只给 `REVIEW → READY` 一条入边）属于 P4 的审查流程；P7A 只 lower 出
`PROPOSED`，`product_intent_for_handoff` 只认 `READY`。**这不是漏项，是切分 §三.6 的
交接约定；缺的是审查流程的接线（§8 第 10 条）。**

## 4.9 错误类型与持久化：迁移 200 与 API

**错误类型**（一个 `ProductError`，不按子域拆第二枚；语义不同的臂必须分开，不许压成
一个 `Invalid`）：

```
#[derive(Debug)]
pub enum ProductError {
    // 上游错误原样透传（不包装成第二套判定）
    Persist(PersistError),
    Intent(IntentError),
    Decision(DecisionError),
    Port(PortError),

    // Product 身份与创建
    UnknownProduct { id: String },
    ProductExists { id: String },
    IntentNotProduct { id: String },
    MissingProductId { intent: String },
    /// 锚 Intent 不是 READY（§4.1 不变量 4）：其余十二态一律拒绝。
    AnchorIntentNotReady { id: String, status: &'static str },
    InvalidContract { intent: String },
    BaselineUnusable { missing: &'static str },
    EmptyShippablePredicate,
    DuplicateDimension { dimension: &'static str },
    DuplicateClause { id: String },
    EmptyClauseId,
    EmptyClauseStatement { id: String },

    // readiness / evidence
    EvidenceNotSupplied { id: String },
    DuplicateEvidence { id: String },
    EmptyReadinessValue { dimension: &'static str },
    ReadinessValueWithoutEvidence { dimension: &'static str },
    ReadinessEvidenceWithoutValue { dimension: &'static str },
    ReadinessLastVerifiedMismatch { dimension: &'static str },

    // Milestone / lower
    UnknownMilestone { id: String },
    MilestoneProductMismatch { milestone: String, product: String },
    MilestoneNotCompleted { id: String },
    TerminalMilestone { id: String },
    MilestoneStateMismatch { id: String, expected: String, actual: String },
    EmptyMilestoneObjective,
    /// `entry_conditions`／`exit_conditions` 的某个元素是空串；`field` 取
    /// `"entry_conditions"`／`"exit_conditions"`，`index` 是该元素下标。
    EmptyMilestoneCondition { field: &'static str, index: usize },
    DuplicateProgress { product: String, milestone: String },
    MissingProgressForCompleted { milestone: String },
    InvalidBatchBoundary,
    EmptyBatch,
    BatchTooLarge { limit: usize, actual: usize },
    DependencyOutOfRange { index: usize },
    DependencyDepthExceeded { limit: u32, actual: u32 },
    MilestoneIdCollision { id: String },
    /// `MilestoneId` 的 ordinal 与写入 seq 不一致（I1）。
    MilestoneOrdinalMismatch { id: String, expected: u64, actual: u64 },
    IntentIdCollision { id: String },
    InvalidLoweredIntent { id: String },

    // Progress Gate / Decision
    ProgressNotTriggered,
    ProgressUndetermined,
    EmptyReplanField { field: &'static str },

    // drive / 相位
    MissingDriveState { product: String },
    DriveDisabled,
    NotShippableYet,

    // 存储
    CorruptRow { table: &'static str, detail: String },
}
```

`From<PersistError>`／`From<IntentError>`／`From<DecisionError>`／`From<PortError>` 各一条。
**不派生 `PartialEq`**：`PersistError` 等不因此被要求实现它；测试用 `matches!` 断臂。
**没有 `ProductError::Invalid`**：每一臂都指一件具体的事；把「不可判」与「明确不成立」压成
同一臂会直接违反 §4.5／§4.6 的 fail-closed 区分。

**迁移 200（`p7a_product_migrations()`，名称 `p7a_product`）**：前三张表
（`product`／`milestone`／`product_progress`）是已定形状。**C2 裁定是 200 合并/应用的
硬前置**：裁定前 P7A 不应用 200（没有三表先行版）；裁定后 200 一次成型——批准 §4.7 的
提案 (b)（独立表）⇒ 四张表（含 `product_drive`）；落点 (a)（既有面）⇒ 三张表，
§4.7 的独立表与 API（含 `load_product_drive`）作废，drive 的读/写改由 C2 裁定的既有面
承担，具体表／列／更新 API 随裁定给出（§8 第 12 条）。**不存在「先应用三表、后补第四张」的路径。**

```
product (
    id                   TEXT PRIMARY KEY NOT NULL,
    intent_id            TEXT NOT NULL UNIQUE,
    baseline             TEXT NOT NULL,          -- EngineeringBaseline 的私有 JSON
    readiness            TEXT NOT NULL,          -- ProductReadiness 的私有 JSON
    current_milestone    TEXT,
    shippable_predicate  TEXT NOT NULL,          -- ShippablePredicate 的私有 JSON
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);
milestone (
    id                   TEXT PRIMARY KEY NOT NULL,
    product_id           TEXT NOT NULL,
    seq                  INTEGER NOT NULL,
    objective            TEXT NOT NULL,
    dependencies         TEXT NOT NULL,          -- JSON 数组（MilestoneId 串）
    entry_conditions     TEXT NOT NULL,          -- JSON 数组（Opaque 串）
    exit_conditions      TEXT NOT NULL,          -- JSON 数组（Opaque 串）
    state                TEXT NOT NULL,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    UNIQUE (product_id, seq)
);
product_progress (
    product_id            TEXT NOT NULL,
    milestone_id          TEXT NOT NULL,
    previous_product_state TEXT NOT NULL,        -- ProductReadiness JSON
    current_product_state  TEXT NOT NULL,        -- ProductReadiness JSON
    progress_delta         TEXT NOT NULL,        -- improved/no_progress/regressed/undetermined
    recorded_at            INTEGER NOT NULL,
    PRIMARY KEY (product_id, milestone_id)
);
-- 提案 (b)：C2 裁定后与 200 一次写入；裁定前不应用 200，不存在三表先行版（§8 第 12 条）。
product_drive (
    product_id                TEXT PRIMARY KEY NOT NULL,
    continuous_product_drive  INTEGER NOT NULL,  -- 0/1；其他值读回即 CorruptRow
    phase                     TEXT NOT NULL,     -- pre_shippable/shippable/post_shippable_improvement
    updated_at                INTEGER NOT NULL
);
CREATE INDEX idx_milestone_product_seq ON milestone(product_id, seq);
```

**存储编码是内部的，不是公开序列化契约**：`EngineeringBaseline`／`ProductReadiness`／
`ShippablePredicate` **不派生 `Serialize`／`Deserialize`**；`persist.rs` 用
`serde_json::json!` 手工编码／解码，未知或缺失的键、表外枚举值一律 `CorruptRow`，
**不取默认值**。`MilestoneState`／`ProductPhase`／`ProgressDelta`／`ReadinessDimensionName`
的 `as_str`／`parse` 与类型同址、`pub(crate)`；表外取值读回即 `CorruptRow`。
这张表与 P6A 的「字段私有、唯一构造点」同一纪律：反序列化不给公开类型开第二个构造点。

**读 API**：

```
/// 复核 §4.1 的 5–6（1–4 是写路径不变量，见 `establish_baseline`／`save_product`）；
/// 另外逐项重新走构造期校验（I6）：
/// - baseline 的 product_goal 与 minimum_complete_system 非空（空 ⇒ `CorruptRow`）；
/// - predicate 走 `ShippablePredicate::new`（空 required_dimensions／空 clauses／重复／
///   空 id／空 statement ⇒ `CorruptRow`）；
/// - readiness 每维重走 `ReadinessDimension` 的三元组不变量（`Assessed` 非空 value、
///   非空且无重复 evidence、`Some(last_verified)`；`Unassessed` 反之 ⇒ `CorruptRow`）。
/// 锚 Intent 的 `READY` 只在 `establish_baseline`／`save_product` 的写路径核对（不在
/// `load_product` 里跨表回查，避免把 P4 的 `intent` 读面拖进本函数）。行不存在 ⇒ `Ok(None)`。
pub fn load_product(tx: &Tx<'_>, id: &ProductIdRef)
    -> Result<Option<Product>, ProductError>;
/// 行存在但 drive 值表外 ⇒ `CorruptRow`；行缺失 ⇒ `Ok(None)`（`save_product` 之后
/// 不应缺失；缺失在交接函数里按 `MissingDriveState` 处理）。
pub fn load_product_drive(tx: &Tx<'_>, id: &ProductIdRef)
    -> Result<Option<ProductDriveState>, ProductError>;
/// 复核 objective 非空、`entry_conditions`／`exit_conditions` 的每个元素非空
/// （与 §4.4 的构造校验同向；违反 ⇒ `CorruptRow`）。
pub fn load_milestone(tx: &Tx<'_>, id: &MilestoneId)
    -> Result<Option<Milestone>, ProductError>;
/// 同 `load_milestone`：任一 Milestone 违反 ⇒ `CorruptRow`。
pub fn load_milestones(tx: &Tx<'_>, product_id: &ProductIdRef)
    -> Result<Vec<Milestone>, ProductError>;   // 按 seq 升序，即 milestone_history
pub fn load_progress(tx: &Tx<'_>, product_id: &ProductIdRef)
    -> Result<Vec<ProgressRecord>, ProductError>; // 按 recorded_at／插入序
```

**事务纪律**：所有写 API（`save_product`／`save_milestones`／`record_milestone_state`／
`record_milestone_progress`／`emit_replan_decision`／`record_shippable_evaluation`／
`begin_post_shippable_work`／`set_continuous_product_drive`／`set_current_milestone`）都
**不自己 `commit`**；`&Product`／`&Milestone` 必须是**同一个 `Tx` 里** `load_*` 得到的
当前快照；`Err` 后调用方必须回滚。`Tx` 持有 `MutexGuard`，故本 crate 的异步生成入口
（`establish_baseline`／`propose_next_batch`）**不接收 `Tx`**，写与判断分成两步（§4.3、
§4.4）。

**没有** `load_all_products`／按状态列 Intent／`load_evidence`；P7A 不补 P4/P5a 的查询面
（切分 §6.2 第 5、6 条）。

**迁移注册**：`runtime_migrations()`（`main.rs:114`）、`expected_migrations()`
（`crates/continuum-runtime/tests/migrations.rs:37`）、`startup.rs` 的三处计数（`:26`／`:81`／`:94`）同批改。P7A 只占
**200**；191–199 是 P6 档，不挪用；201 留给 P7D 若落库。

---

# 5. 判据与照片

## 5.1 判据表（每条挂规范节）

**共 19 条（J1–J19）。** 每条至少一种可测试的观测形态；拍不到的部分写在 §5.5。

| # | 判据 | 规范节 | 观测形态 |
|---|---|---|---|
| **J1** | `Product` 恰七个字段，名字与 §231 逐字；`id` 是 `ProductIdRef`；`baseline` 是 `EngineeringBaseline`；本 crate 无 `struct ProductId` | §231 `:424-438`；切分 §2.3.1 `:126` | 源码面（`sed` 读七行＋`grep`）＋ 编译期（外部结构体字面量编不过） |
| **J2** | `Product.id` 只能由锚 Intent 的 `product_id` 派生；kind 非 Product／product_id 缺失／contract 无效／**锚 Intent 不是 READY** ⇒ 拒绝；两个 Product 不能共用同一 id | §231；切分 §三.6 `:215-218` | 运行期（正例＋四类负例） |
| **J3** | `ProductReadiness` 恰九维，名字与 §232 逐字；每维有 `value/evidence/last_verified`，且三者经只读 getter 可取；**没有** `score`／`average`／`sum` 公开方法；`ReadinessValue::as_assessed` 能区分未评与已评 | §232 `:442-466`；§153 `:223-253`；C1 | 源码面（`sed` 读九行）＋ 编译期（`trybuild`：`readiness.score()` 编不过）＋ 运行期（getter 读回三元组、真实策略读值） |
| **J4** | `Assessed` ⇒ 非空 value、非空 evidence、`Some(last_verified)`；`Unassessed` ⇒ 空 evidence、`None`；每个 EvidenceId 必须在传入的 `&[Evidence]` 里；只存 id，不读 evidence 表 | §232；切分 §四第 4 条 `:286-293`、§6.2 第 6 条 `:405` | 运行期（正例＋五类负例）＋ 源码面（无 `FROM evidence`） |
| **J5** | `EngineeringBaseline` 恰十字段，名字与 §150 逐字；`Product` 只能由 `establish_baseline`＋`save_product` 产生；生成失败 ⇒ 零 Product 行；没有公开的 `Product::new`／`Deserialize` | §150 `:43-114`；切分 §三第 5 条 `:200-212` | 源码面＋运行期（假生成器）＋ 编译期（结构体字面量／`Default` 编不过） |
| **J6** | baseline 缺失或不可用 ⇒ `evaluate_shippable` 返回 `Undetermined`；空 baseline 的库行读回 ⇒ `CorruptRow`；不得用全 `Unassessed` 冒充已评估；`Product.baseline` 非可空 | §231；§232；切分 §2.3.1 `:131-132`；I6 | 运行期（`None`、空 baseline 行、内存 `BaselineUnusable` 三条） |
| **J7** | `Milestone` 恰七字段、七状态，名字与 §234 逐字；状态列的表外值读回即 `CorruptRow`；**空 objective，或 `entry_conditions`／`exit_conditions` 的空元素，读回即 `CorruptRow`** | §234 `:498-525` | 源码面＋运行期（往返＋表外值＋空 objective／空条件） |
| **J8** | 批次非空、`len <= max_milestones`、批内依赖深度 `<= max_dependency_depth`；超界拒绝且零行写入；**`entry_conditions`／`exit_conditions` 的空元素 ⇒ `EmptyMilestoneCondition`、零行；空 `objective` ⇒ `EmptyMilestoneObjective`、零行**；生成器输入只含已生成历史；没有「baseline → 全部 Milestone」的入口；COMPLETED Milestone 没有 ProgressRecord 时拒绝新批次；**两个批次各 2 枚时 id 全局唯一（ordinal = history.len()+i，seq 连续）**；**Product 未保存 ⇒ `UnknownProduct`、零行** | §233 `:470-495`；§151 `:118-171`；§152 `:175-219`；切分 §6.1 `:394`；§2.3 第 17 条；I1、I5 | 运行期（假生成器：边界内／超界／缺 progress／两批 id／孤儿 Product／空 objective／空条件）＋ 源码面（无完整列表 API） |
| **J9** | 依赖允许分叉／汇合（一依赖多、多依赖一）；`InBatch(i)` 只指向更早项；自环／环／越界 ⇒ 拒绝；图边留在 milestone 表 | §152 `:175-219` | 运行期（DAG 正例＋环负例） |
| **J10** | lower 产出合法 `Intent`：`kind = Product`、`product_id = Some`、`parent_intent = 锚`、`status = PROPOSED`、`contract` 有效、`completion_predicate` 取 exit conditions（**空 exit ⇒ `""`**）；跨 Product 的 Milestone ⇒ `MilestoneProductMismatch`；同输入两次 lower 的字段相同；`save_lowered_intent` 不接受 `Intent`（外部传 `READY`／`RUNNING`／`COMPLETED` 的 Intent 编不过） | §152；§222 `:132-155`；切分 §三.6 `:215`；I2、I7、C3 | 运行期（两次调用对照＋跨 Product 负例＋空 exit 映射）＋ 编译期（`trybuild`：`save_lowered_intent(tx, &forged_intent)` 编不过）＋ 源码面（无 `UPDATE intent`） |
| **J11** | 交接按 `MilestoneId` 只返回 `READY`；其余十二个状态（`PROPOSED`／`SPECIFYING`／`PLANNING`／`REVIEW`／`RUNNING`／`WAITING`／`SUSPENDED`／`VERIFYING`／`COMPLETED`／`FAILED`／`CANCELLED`／`EVOLVING`）一律 `None`；**两个 READY、current 只指向其中一个时，两次按 id 调用都返回 `Some`**；**特别是 `PLANNING→SUSPENDED→RUNNING` 的 Intent（状态是 `RUNNING` 但从未过 `REVIEW`）必须被拒绝**；P7A 不把 PROPOSED 推进到 READY；只对 COMPLETED→EVOLVING 调公开状态机 | §223 `:174-206`；`intent.rs:249`；切分 §三.6 `:226-237`、§八第 5 条 `:483-485`；I3 | 运行期（十二个非 READY 状态＋READY＋两个 READY）＋ 源码面（仅一个 `apply_intent_transition` 调用点） |
| **J12** | Progress Gate 三值；无策略 ⇒ `Undetermined(NoPolicy)`；阈值 0 ⇒ `InvalidThreshold`；全 `Unassessed` ⇒ `ReadinessUnassessed`；**真实策略（读 `dimension(name).value().as_assessed()` 并解释 `Opaque`）能对同一对 readiness 给出 Improved／NoProgress／Regressed 三种 delta**；没有策略时**绝不** `Triggered` | §337 `:2583-2597`；§154 `:257-309`；切分 §6.1 `:389-394`；C1 | 运行期（真实策略＋无策略，不再用返回常量的假策略） |
| **J13** | 连续 `NoProgress` 计数与阈值；`Improved`／`Regressed` 打断 run；`Undetermined` 在未达阈值时 ⇒ `HistoryUndetermined`；同一 Milestone 不得有两条记录；未 COMPLETED 的 Milestone 拒绝；**用真实策略生成的 `ProgressRecord` 序列（而非手填 delta）跑通上述计数** | §337；§154；§153 `:223-253`；C1 | 运行期（真实策略序列＋重复／未完成负例） |
| **J14** | 传入 `record`＋`history`＋`policy` 重算为 `Triggered` ⇒ 恰一张 `Decision`、恰一条 `decision.required`；重算为 `NotTriggered`／`Undetermined` ⇒ 零张、零事件；**阈值 3、实际连续 2（`record` 计 1＋`history` 计 1）⇒ `NotTriggered{2,3}`、零张；`history` 重复含 `record` 时按一条计，不得变成 Triggered**；**伪造 `Triggered`／`NotTriggered`／错误计数不能产生 Decision（签名里没有 verdict 输入）**；P7A 无 replan 执行路径；options 原样来自 draft | §337；§154 `:299-309`；§331 `:2471-2482`；切分 §6.2 第 13 条 `:412`；I8 | 运行期（重算＋行数＋事件数）＋ 源码面（无 Planner／无 Contract/Plan 写） |
| **J15** | `ShippableVerdict` 三值；无 Product ⇒ `Undetermined`；assessment 缺／多／重复（**且无明确不满足**）⇒ `Undetermined`；子句 NotSatisfied ⇒ `NotShippable`；required dimension 未评 ⇒ `Undetermined`；completion Blocked ⇒ `NotShippable`；completion Undetermined ⇒ `Undetermined`；**组合负例：assessment 缺一条＋completion Blocked ⇒ `NotShippable(CompletionBlocked)`；assessment 多一条＋某 clause NotSatisfied ⇒ `NotShippable(ClauseNotSatisfied)`；`None`＋completion Blocked ⇒ `Undetermined(MissingProduct)`**；**今天没有 `Shippable` 生产者** | §335 `:2535-2547`；§336；切分 §四第 4 条 `:286-293`、§6.2 第 7 条 `:406`；C4 | 运行期（八个分支＋三条组合负例）＋ 编译期（P5a 加正臂时 `evaluate_shippable` 的 match 会红） |
| **J16** | 空谓词／空 clause 集合／重复 dimension／重复 clause id／空 clause id／空 statement 一律拒绝；assessment 缺／多／重复（**且没有明确不满足的 clause／completion**）⇒ `Undetermined(AssessmentMismatch)`；P7A 不评估子句内容 | §335；§155 `:313-373` | 运行期（六类构造负例＋纯结构 assessment 负例） |
| **J17** | **J17a（纯相位机）**：相位只前进；`Shippable` 后 `NotShippable`／`Undetermined` 不回退；`PostShippableImprovement` 不被覆盖；`PreShippable` 不能直接进 `PostShippableImprovement`（`advance_product_phase` 从不返回 Post）；**J17b（落库／交接，(b) 专属；条件于 C2 裁定）**：`ContinuousProductDrive` 默认 false；开关关闭 ⇒ `begin_post_shippable_work` 拒绝；**关闭开关后，Post 相位的 READY Product Intent 交接 ⇒ `None`**；`PreShippable` 调 `begin_post_shippable_work` ⇒ `NotShippableYet` | §336 `:2551-2580`；§155 `:313-373`；§7.4 判据 4 `docs/02-工程.md:466`；I4 | 运行期（J17a 合成 `Shippable` 的相位机；J17b 只在 C2 选 (b) 并落库后拍开关／交接 gate 负例）＋ 源码面（J17b 只在 (b) 下拍：唯一 phase 写点 = `product_drive`；C2 选 (a) ⇒ J17b 的运行期与源码面照片都延后，待裁定给出既有面的表／列／更新 API） |
| **J18** | **C2 裁定是 200 的硬前置**：裁定前不应用 200；裁定后一次成型——批准提案 (b)（独立表）⇒ 恰四张表，落点 (a)（既有面）⇒ 恰三张表；启动计数 15／13／15；`crates/continuum-runtime/tests/migrations.rs:37` 的 `expected_migrations()` 含 200；190 的 SQL 未改；P7A **不直接写** `intent`／`decision`／`evidence` 表（只经公开函数） | 切分 §四第 2 条 `:265-276`；§4.9；§8 第 12 条 | 源码面（迁移清单／`PRAGMA table_info`）＋ 运行期（真库启动） |
| **J19** | 依赖恰四个 workspace 直接依赖（`cargo tree --depth 1` 的根包行与外部依赖行不计）；无 `continuum-runtime`／provider／model-registry／node；本 crate 无 `struct ProductId`／`enum IntentStatus`／`struct Evidence`／`enum CompletionVerdict`／`struct Decision`；无 `continuum-context` | 切分 §四第 1、3、5 条；§2.5 | 源码面（`Cargo.toml`／`ALLOWED`／`cargo tree --depth 1`／`grep`） |

## 5.2 照片清单（每条判据的可测试观测形态）

| # | 照片（具体断言或命令） | 形态 | 拍不到的部分 |
|---|---|---|---|
| **J1** | `sed -n` 读出 `struct Product` 的七行字段，逐行与 §231 的七个名字比对；`grep -rn "struct ProductId" crates/continuum-product/src` 零命中；`trybuild` 外部 `Product { .. }`／`Product::default()` 编不过 | 源码面＋编译期 | 字段顺序在运行期不可见；字段私有，外部不能穷尽解构（改用 `sed`） |
| **J2** | 正例：锚 Intent `kind=Product`、`product_id=Some("p1")`、`status=READY`、`contract_id={id:"c1",version:1}` ⇒ `establish_baseline` 成功、`save_product` 成功；负例：kind=Task／`product_id=None`／`product_id=Some("")`／contract.version=0／status=PROPOSED／status=RUNNING 各一条 ⇒ `Err`、零 `product` 行；再存同 id ⇒ `ProductExists` | 运行期 | 两个不同 Intent 共享同一 `product_id` 时，第二个会被主键拒绝——这是主键，不是新判据 |
| **J3** | `sed -n` 读出 `ProductReadiness` 的九行字段，逐行与 §232 比对；`dimension(name)` 九枚都能取到，且对每维 `value()`／`evidence()`／`last_verified()` 返回与 `assessed(..)` 入参一致；`ReadinessValue::as_assessed()` 对 `Unassessed` 返回 `None`、对 `Assessed` 返回 `Some`；**同一个 `DeltaByDimension` 策略经这些 getter 读 2→3／3→3／3→2，能分别得到 Improved／NoProgress／Regressed**；`trybuild` 三条：`readiness.score()`／`.average()`／`.sum()` 编不过；`grep -rn -e "fn score" -e "fn average" -e "fn sum" -e "fn aggregate" crates/continuum-product/src` 零命中 | 源码面＋编译期＋运行期 | 调用方可以在自己的策略里聚合（§2.3 第 1 条） |
| **J4** | 正例：`assessed(Opaque::from_str("0.8"), vec![e1], 100)`＋`evidence=[e1]` ⇒ `record_milestone_progress` 成功、库里 readiness 只含 `"e1"`；负例：空 value／空 evidence／重复 e1／`last_verified=None`／`evidence` 里没有 e2 各一条 ⇒ `Err`、零 progress 行；`grep -rn "FROM evidence" crates/continuum-product/src` 零命中 | 运行期＋源码面 | 证据是否真的支持该维（§2.3 第 12 条） |
| **J5** | 假生成器返回十字段基线 ⇒ Product 创建；假生成器返回 `Err` ⇒ 零 Product 行；`grep -rn "Product {" crates/continuum-product/src \| grep -v '^[^:]*:[0-9]*:[[:space:]]*//'` 的命中逐条登记，**允许清单恰为**：① `struct Product {` 的字段定义；② `impl Product {` 的固有实现块（§4.1 的访问器；拆成多个 impl 时逐块登记）；**清单外命中即红**；构造点另用 `grep -rn -e "Self {" -e "Product::from_verified_parts" crates/continuum-product/src \| grep -v '^[^:]*:[0-9]*:[[:space:]]*//'`：`Product::from_verified_parts` 的命中只允许 `establish_baseline`／`row_to_product` 两个调用点；`Self {` 的命中逐条登记，**属于 `Product` 构造点的恰一处且在 `from_verified_parts` 体内**，其余必须归为「非 `Product` 类型」并登记；出现无法归类的命中即红；`trybuild`：外部 `Product::default()`／`Product { .. }` 编不过 | 运行期＋源码面＋编译期 | 生成器输出的工程正确性（模型判断，P7A 不判） |
| **J6** | `evaluate_shippable(None, &assessment, &CompletionVerdict::Undetermined{..})` ⇒ `Undetermined(MissingProduct)`；用 `ProductReadiness::unassessed()` 的 Product 调同一函数 ⇒ `Undetermined(RequiredDimensionUnassessed(..))`；**手工插入 `product_goal = ""` 的行 ⇒ `load_product` 返回 `CorruptRow`**；**crate 内 `#[cfg(test)]` 构造一个 baseline 不可用的 `Product` ⇒ `evaluate_shippable` 返回 `Undetermined(BaselineUnusable)`**；断言没有任何分支返回 `Shippable` | 运行期 | 「没有 Product」与「Product 行损坏」的区分（后者 `CorruptRow`） |
| **J7** | `sed -n` 读出 `struct Milestone` 的七行字段，逐行与 §234 比对；`trybuild` 外部 `Milestone { .. }` 编不过；七个访问器逐枚取到；七状态逐枚往返；插入 `state='bogus'` 后 `load_milestone` ⇒ `CorruptRow`；分别插入 `objective=''`／`entry_conditions='[""]'`／`exit_conditions='[""]'` 的三行后，各自 `load_milestone` ⇒ `CorruptRow`；`dependencies`／`entry_conditions`／`exit_conditions` 原样往返 | 源码面＋运行期＋编译期 | 状态迁移合法性（§2.3 第 13 条） |
| **J8** | 假生成器返回 3 枚、`BatchBoundary::new(2, 0).unwrap()` ⇒ `BatchTooLarge`、零 `milestone` 行；返回 2 枚、边界 2 ⇒ 两行、`seq` 连续、`current_milestone` 指第一枚；第二次调用的 `input.history.len() == 2`，**再返回 2 枚 ⇒ id 的 ordinal 为 2/3、四枚 id 全局唯一、`seq` 连续 0..3**；把一枚 Milestone 置 COMPLETED 但不写 ProgressRecord 再保存新批次 ⇒ `MissingProgressForCompleted`、零新行；**不调 `save_product` 直接 `save_milestones` ⇒ `UnknownProduct`、零行**；**`entry_conditions: vec![Opaque::from_str("")]` 的 draft 与 `exit_conditions: vec![Opaque::from_str("")]` 的 draft 各一条 ⇒ `EmptyMilestoneCondition`、零行；crate 内构造同值的 `Milestone` 传给 `save_milestones` 也 ⇒ 同错、零行；`objective: Opaque::from_str("")` 的 draft ⇒ `EmptyMilestoneObjective`、零行，crate 内构造同值的 `Milestone` 传给 `save_milestones` ⇒ 同错、零行**；`grep -rn -e "fn all_milestones" -e "fn full_plan" crates/continuum-product/src` 零命中 | 运行期＋源码面 | 调用方申报巨大边界（§2.3 第 4 条） |
| **J9** | DAG：M1；M2 依赖 M1；M3 依赖 M1；M4 依赖 M2＋M3（`InBatch(0)`／`InBatch(1)`）⇒ 接受；自环／`InBatch(5)`／`InBatch` 指向更晚项 ⇒ `Err`；`dependencies` 读回含四枚 id | 运行期 | §152 的图到 Intent 的边（§8 第 6 条） |
| **J10** | 同一 Product＋Milestone＋contract＋priority＋时间戳调两次 `lower_milestone`：`kind`／`status`／`goal.as_str()`／`product_id`／`parent_intent`／`completion_predicate.as_str()` 逐项相等；**跨 Product 的 Milestone ⇒ `MilestoneProductMismatch`**；**空 `exit_conditions` ⇒ `completion_predicate.as_str() == ""`（P5a 侧映射为 `None` 的唯一值）**；`save_lowered_intent` 只收 Product／Milestone／contract／priority／时间戳，第一次写 `intent` 行、第二次不写（行数、version 不变）；**`trybuild`：把一枚 `status=READY`（或 `RUNNING`／`COMPLETED`）的 `Intent` 传给 `save_lowered_intent` 编不过**；`grep -rn -e "UPDATE intent" -e "INSERT INTO intent" crates/continuum-product/src` 零命中 | 运行期＋编译期＋源码面 | `goal` 不是 Canonicalizer 的产物（§2.3 第 10 条） |
| **J11** | 按 id 构造十三个状态各一枚 Intent：只有 `READY` 的 `product_intent_for_handoff(tx, product, &milestone)` 返回 `Some`，其余十二个 `None`；**两个 READY、`product.current_milestone` 只指向其中一个：两次按 id 调用都返回 `Some`**；**另加一条负例**：`PLANNING → SUSPENDED → RUNNING` 的 Intent（状态是 `RUNNING`，但从未过 `REVIEW`）仍必须 `None`；`COMPLETED` 调 `retire_completed_product_intent` 后 status=`EVOLVING`（再调 `product_intent_for_handoff` ⇒ `None`）；`grep -rn "apply_intent_transition" crates/continuum-product/src \| grep -v '^[^:]*:[0-9]*:[[:space:]]*//' \| grep -v '^[^:]*:[0-9]*:[[:space:]]*use '` 只命中 `retire_completed_product_intent` 一处 | 运行期＋源码面 | 谁把 PROPOSED 推进到 READY（§8 第 10 条） |
| **J12** | `evaluate_progress(.., None)` ⇒ `Undetermined(NoPolicy)`；阈值 0 的策略 ⇒ `InvalidThreshold`；全 `Unassessed` ⇒ `ReadinessUnassessed`；**真实策略 `DeltaByDimension`：用 `dimension(Documentation).value().as_assessed()` 取 `Opaque`、按调用方自己的整数刻度解释，分别构造 value 2→3／3→3／3→2 的三对 readiness，断言 delta 为 Improved／NoProgress／Regressed**；阈值 3、历史为空、delta=NoProgress ⇒ `NotTriggered{1,3}`；断言这些分支都不产生 `Decision` | 运行期 | 规范级阈值（§2.3 第 3 条） |
| **J13** | **真实策略序列**：用 `DeltaByDimension` 对三对真实 readiness 生成 `ProgressRecord`（不手填 `progress_delta`）；NoProgress×2（阈值 3）⇒ NotTriggered；第三条 NoProgress ⇒ Triggered{3,3}；中间插一条 Improved ⇒ 计数归零；历史含 Undetermined 且未达阈值 ⇒ HistoryUndetermined；同一 milestone 二次记录 ⇒ `DuplicateProgress`；`state=Active` ⇒ `MilestoneNotCompleted` | 运行期 | 策略本身是否诚实（§2.3 第 2 条） |
| **J14** | 传入 `record`＋`history`＋`policy` 重算为 Triggered＋draft（2 options）⇒ `decision` 表 1 行、`events` 里 `decision.required` 1 条；重算为 NotTriggered／Undetermined ⇒ 0／0；**阈值−1 负例：阈值 3、实际连续 2（`record`＋`history` 各一条 NoProgress）⇒ `NotTriggered{2,3}`、0／0；把 `record` 再重复塞进 `history`（同一 `milestone_id`）仍按 2 计、0／0**；**把 `history` 里一条 NoProgress 改成 Improved（错误计数）⇒ 0／0**；同一 draft 二次 emit ⇒ 主键 `Err`、仍 1 行；**签名不含 `ProgressGateVerdict`，伪造 `Triggered{99,1}` 无法传入（`trybuild`）**；`grep -rn -e "Planner" -e "replan(" crates/continuum-product/src \| grep -v '^[^:]*:[0-9]*:[[:space:]]*//' \| grep -v '^[^:]*:[0-9]*:[[:space:]]*use '` 零命中 | 运行期＋源码面 | replan 由谁消费（§8 第 5 条） |
| **J15** | 八个分支逐一：`None`→Undetermined(MissingProduct)；缺 clause→AssessmentMismatch；NotSatisfied→NotShippable；required dim unassessed→Undetermined；clause Undetermined→Undetermined；Completion Blocked→NotShippable；Completion Undetermined→Undetermined；全满足＋两臂都试过 ⇒ 仍不返回 Shippable（因为无第三臂）；**组合负例 A：缺一条 assessment＋CompletionVerdict::Blocked ⇒ NotShippable(CompletionBlocked)**；**组合负例 B：多一条 assessment＋某 clause NotSatisfied ⇒ NotShippable(ClauseNotSatisfied)**；**组合负例 C：`None`＋CompletionVerdict::Blocked ⇒ Undetermined(MissingProduct)，不是 NotShippable**；`grep -rn "Shippable" crates/continuum-product/src` 的命中逐条登记（定义／合成测试例外） | 运行期＋源码面 | 正向宣告（§8 第 3 条） |
| **J16** | `ShippablePredicate::new(vec![], vec![clause])`／`new(vec![Functional], vec![])`／重复 dimension／重复 clause id／空 clause id／空 statement 各一条 ⇒ `Err`；assessment 缺一条／多一条／重复一条（且无 NotSatisfied／Blocked）⇒ `Undetermined(AssessmentMismatch)`；断言 P7A 没有读 clause.statement 的判定分支 | 运行期＋源码面 | 子句内容是否真被评估（§2.3 第 7 条） |
| **J17** | **J17a（纯相位机）**：合成 `Shippable`：Pre→Shippable；Shippable＋NotShippable ⇒ Shippable；Shippable＋Undetermined ⇒ Shippable；Post＋任何 ⇒ Post；Pre＋NotShippable／Undetermined ⇒ Pre；断言 `advance_product_phase` 从不返回 `PostShippableImprovement`。**J17b（仅 C2 选 (b) 并落库后）**：`begin_post_shippable_work` 在 Pre ⇒ `NotShippableYet`、开关 false ⇒ `DriveDisabled`、Shippable＋true ⇒ Post；**Post 相位＋开关 false 时，该 Milestone 的 READY Intent 交接 ⇒ `None`；开关 true 后同一调用 ⇒ `Some`**；**源码面（仅 (b)）**：`grep -rn "phase = ?" crates/continuum-product/src` 只统计**SQL 写点**（参数占位符），且只命中 `product_drive` 的 phase 写点；排除 `let phase =` 之类的局部变量，且命中数等于该面唯一写点数。**C2 选 (a) ⇒ J17b 的上述运行期断言与源码面照片都不写**：§4.7 的独立 API 与 `product_drive` 作废（§4.9），drive 的读/写改由 C2 裁定的既有面承担，照片延后到裁定给出该面的表／列／更新 API 后再写 | 运行期＋源码面 | 生产路径进入 Shippable（§8 第 3 条）；J17b 仅在 (b) 下可拍，选 (a) 时整段延后 |
| **J18** | `p7a_product_migrations()` 版本恰 `[200]`、名称 `p7a_product`；**C2 裁定后 200 一次成型**：批准提案 (b)（独立表）⇒ `PRAGMA table_info` 四张表列清单，落点 (a)（既有面）⇒ 三张表（不得出现 `product_drive`）；**裁定前不应用 200（无三表先行版）**；真库启动输出 `迁移应用 15 项`／`13 项`／`15 项`；`crates/continuum-runtime/tests/migrations.rs:37` 的 `expected_migrations()` 含 200；`grep -Hn "190" crates/continuum-memory/src/persist.rs \| grep -v '^[^:]*:[0-9]*:[[:space:]]*//'` **只命中 `:13` 的迁移编号，不把注释计入**，且 `:13` 的 SQL 与设计前逐字相同（`git diff --exit-code c89d839 -- crates/continuum-memory/src/persist.rs` 为空）；`grep -rn -e "FROM intent" -e "FROM decision" -e "FROM evidence" crates/continuum-product/src` 只允许 `load_intent` 的公开调用（无 SQL） | 源码面＋运行期 | 200 应用后的未来改动（版本号已锁） |
| **J19** | `cargo tree -p continuum-product --depth 1 --edges normal \| grep -E '^[├└]── continuum-'` 恰输出四行，去掉树线与版本后缀后的 crate 名恰为 `continuum-canonical`／`continuum-persist`／`continuum-semantics`／`continuum-verify`（顺序不计）；**不断言总行数**：根包一行与外部依赖 `async-trait`／`serde_json`／`thiserror` 也在 depth 1，但被过滤掉；`ALLOWED` 行逐项相等；`grep -rn -e "struct ProductId" -e "enum IntentStatus" -e "struct Evidence" -e "enum CompletionVerdict" -e "struct Decision" crates/continuum-product/src` 零命中；`grep -rn "continuum_context" crates/continuum-product` 零命中 | 源码面 | 未来新增依赖（会先让 `dependency_direction` 红） |

## 5.3 §7.4 四条完成判据的逐条落点

| §7.4 判据（`docs/02-工程.md:462-467`） | 归属 | 本设计的落点 | 照片 / 限度 |
|---|---|---|---|
| **判据 1**：Milestone 逐个生成，系统中不存在预先定义的完整 Milestone 列表 | **P7A** | §4.4 的 `propose_next_batch`＋`BatchBoundary`；`milestone` 表只存已生成行；没有「baseline → 全部 Milestone」的 API | J8、J9。**限度**：只钉「不出现完整列表」与「评估后再生成」；不钉批次大小（OPEN-015 仍开） |
| **判据 2**：Progress Gate 在连续无进展时触发 replan 并生成 Decision | **P7A 只交「生成 Decision」这半** | §4.5：`evaluate_progress` 三值；`emit_replan_decision` 用 record／history／policy **重算**为 `Triggered` 后才 `create_decision`（不接受自报 verdict）；不执行 replan | J12、J13、J14。**replan 消费方是第 3 层 Planner（无实现）**，按切分 §6.2 第 13 条 `:412` 具名缺口；P7A 不假装闭合 |
| **判据 3**：Resource Arbiter 在模型层完全不可用的情况下仍能限制后台负载 | **P7C**（不是 P7A） | 本设计不含 Resource Arbiter；`continuum-product` 不依赖第 4 层 router／placement | 不拍；P7C 的 `ALLOWED == []` 是它的照片 |
| **判据 4**：Shippable 与 post-shippable 两种状态被区分，不互相覆盖 | **P7A** | §4.7 的 `ProductPhase`：三个互斥值；相位只前进；`Shippable` 不被 `NotShippable`／`Undetermined` 覆盖；`PostShippableImprovement` 不被覆盖 | J15、J17。**限度**：`Shippable` 的生产路径今天不可达（P5a 无正臂）；能拍的是类型区分、负臂与不回退；不能拍「正向进入 Shippable」 |

## 5.4 变异预告（谁红）

| 变异 | 该红 | 不该红 |
|---|---|---|
| 取反：`Product` 加第八个字段 | J1（穷尽解构少一个绑定／多一个字段） | J2–J19 |
| 取反：`ProductReadiness` 加 `score()` | J3（`trybuild` 的编译失败样例变成编译通过／源码面命中） | J4–J19 |
| 取反：`assessed` 接受空 evidence | J4 | J3（字段仍在）、J6 |
| 取反：`establish_baseline` 允许调用方塞一个 `EngineeringBaseline` | J5（源码面／编译期） | J6 |
| 取反：批次超界时静默截断 | J8（`BatchTooLarge` 的零行断言） | J9（DAG 仍对） |
| 取反：`InBatch` 允许指向更晚项 | J9（环／越界） | J8（边界内批次仍对） |
| 取反：lower 把 `status` 写成 `READY` | J10（status 断言）、J11（PROPOSED 交接） | J2 |
| 取反：`product_intent_for_handoff` 接受 `RUNNING` | J11 | J10 |
| 取反：`save_lowered_intent` 改回接收 `&Intent` | J10（`trybuild` 编译不过） | J11 |
| 取反：两批都从 ordinal 0 起 | J8（id 全局唯一） | J9 |
| 取反：`evaluate_shippable` 把 assessment mismatch 放回 Blocked 之前 | J15（组合负例） | J16（纯构造负例） |
| 取反：`emit_replan_decision` 重新接收自报 verdict | J14（`trybuild`／错误计数） | J12、J13 |
| 取反：无策略时返回 `NotTriggered` | J12（`NoPolicy` 断言） | J13（有策略的序列） |
| 取反：`Regressed` 计入 NoProgress | J13（序列计数）、J12 | J14 |
| 取反：Triggered 不写 `Decision` | J14 | J12、J13 |
| 取反：`Undetermined` 当许可（返回 `Shippable`） | J15、J17 | J16 |
| 取反：空谓词接受 | J16 | J15 |
| 取反：`PostShippableImprovement` 被 `NotShippable` 改回 `PreShippable` | J17 | J15 |
| 取反：迁移 SQL 改 190 | J18（190 的逐字对照） | J8–J17 |
| 取反：新增 `continuum-runtime` 依赖 | J19（`ALLOWED`／`cargo tree`） | 全部运行期判据 |

## 5.5 拍不出照片的地方

1. **正向 `Shippable`**：`CompletionVerdict` 只有两臂（`completion.rs:24`），
   `evaluate_shippable` 没有正向分支。**收件人：P5a ＋ P9（ENG-004）**（§8 第 3 条）。
2. **replan 的执行**：第 3 层 Planner 无实现；P7A 只出 `Decision`。**收件人：第 3 层
   持有方 ＋ 协调者（P8 范围）**（§8 第 5 条）。
3. **PROPOSED → READY 的审查**：P7A 不做，审查流程未接线。**收件人：P4 ＋ 协调者**
   （§8 第 10 条）。
4. **Evidence 是否支持某一维**：P7A 只核 id 存在，不判内容。**收件人：P5a ＋ 规范
   维护者**（§8 第 4 条）。
5. **§152 的图边到 Intent**：`Intent` 只有单亲 `parent_intent`，并行／汇合图不能完整
   lower 成 Intent 边。**收件人：P4 ＋ 规范维护者**（§8 第 6 条）。
6. **规范级批次大小与阈值**：A15／OPEN-010 与 OPEN-015 仍开；本设计只交申报入口
   （§2.3 第 2、3、4 条）。

---

# 6. 消费契约与边界

## 6.1 与 P7D：只按值交 Product Intent

- **交什么**：`product_intent_for_handoff(tx, product, &milestone)` 返回 `Some(Intent)`，且该
  Intent `kind == Product`、`product_id == Some(product.id)`、`status == READY`。**按
  `MilestoneId` 取，不是只取 `current_milestone`**：两个 READY 时两次调用都返回 `Some`，
  P7D 按自己的排名面选一枚（P7A 不排名）。
- **drive gate**：`phase == PostShippableImprovement && !enabled` ⇒ `None`（用户关闭开关后
  不再把 post 工作交给 P7D）；`ProductDriveState` 缺失 ⇒ `MissingDriveState`。
- **不交什么**：`PROPOSED` 的 lower 结果（不算交接完成）；`EVOLVING`／`COMPLETED`；
  Product 记录／baseline／readiness。P7D 不读 §231 的 Product（切分 §三.6 `:216-218`）。
- **授权证据**：Intent 上的 `contract_id`＋`status`（切分 §三.6 的选项 (a)）；P7A 在
  lower 时要求 `contract.id` 非空且 `version >= 1`，`status` 只到 `PROPOSED`，到 `READY`
  由审查链给。P7D 的私有构造器按 `kind`／`contract_id`／`status == READY` 判定；P7A 不
  替它判定。
- **COMPLETED 的处置**：P7A 的 `retire_completed_product_intent` 调公开状态机把
  `COMPLETED → EVOLVING`；新的 Milestone 重新 lower 出新的 `READY` Product Intent。
  `EVOLVING` 本身不是交接值（切分 §三.6 `:233-237`）。
- **没有 crate 边**：P7D 不登记 `continuum-product`；P7A 不登记 `continuum-supervisor`。
  这是值的前置，不是设计串行（切分 §四第 6 条 `:304-307`）。

## 6.2 与 P5a：只消费 `Evidence`／`CompletionVerdict`

- **Evidence**：`record_milestone_progress` 收 `&[Evidence]` 值，只核 `EvidenceId` 的
  存在性；落库只存 id；**不读 evidence 表**（无 `load_evidence`，切分 §6.2 第 6 条
  `:405`）；**不消费**原始工具输出、模型自述、`Claim` 文本。
- **CompletionVerdict**：`evaluate_shippable` 只读 `Blocked`／`Undetermined`；**不造**
  「完成」正臂；不把 `Undetermined` 当许可。
- **`completion_predicate` 的跨块映射（I7）**：`lower_milestone` 对空 `exit_conditions`
  产出 `Opaque::from_str("")`；空元素已在 §4.4 被 `EmptyMilestoneCondition` 拒绝，故 `""` 唯一
  对应「零条 exit conditions」。**P5a 装配时必须把 `""` 映射为
  `declared_completion_predicate = None`**；不要把 `Some(Value::String(""))` 直接传进去
  （P5a 的 `completion.rs:70-76`／`:213-214` 把 `Some("")` 算「有内容」，会产生一项不可判）。
  映射测试在 P7A 侧钉「空 exit ⇒ `""`」，P5a 侧钉「`""` ⇒ `None`」。
- **越界即缺口**：如果将来要 P7A 自己评估 readiness 或解析 evidence，那是 P5a 的读面与
  九维映射的新接口，不在本设计里预登记（§8 第 4 条）。

## 6.3 与 P4：只经公开面

| 用途 | 调用 | 不做什么 |
|---|---|---|
| 读锚 Intent | `load_intent`（`persist.rs:428`） | 不 `SELECT` `intent` 表 |
| 写 lower 结果 | `save_intent`（`persist.rs:353`） | 不 `INSERT/UPDATE intent` |
| 状态迁移（唯一一处） | `apply_intent_transition`（`intent_tx.rs:78`），仅 `COMPLETED → EVOLVING` | 不改迁移表、不推进 PROPOSED→READY |
| 出 replan Decision | `create_decision`（`persist.rs:1173`） | 不自造决策单、不直接写 `decision` 表 |

## 6.4 与装配方（`continuum-runtime`）：端口

- `EngineeringBaselineGenerator` 与 `MilestoneGenerator` 由装配方实现，内部转调既有
  模型调用路径（`crates/continuum-runtime/src/model_call.rs:430` 的 `call`）；P7A 不依赖
  provider／runtime。**端口不是 Resource Scheduler，也不是第二个模型调用路径。**
- 端口的提示词、模型选择、预算、重试都在装配方；P7A 只给输入／输出类型与错误。
- **replan 的消费方**：`Decision` 由第 3 层 Planner 消费（《工程》§3.1 `:158`），或在
  P8 新增范围项；P7A 不执行。**这是 §8 第 5 条的具名缺口。**

## 6.5 本轮能拍到哪一步（据实）

| 层次 | 能拍到 | 照片 |
|---|---|---|
| 类型层 | 七字段／九维／七字段七状态／三值／私有构造 | J1、J3、J5、J7、J15、J16 |
| 源码面 | 无第二份类型、无直接 SQL、无 Planner、依赖恰四项 | J4、J10、J14、J18、J19 |
| 运行期 | 惰性批次、图依赖、lower、Progress Gate、Decision、相位机 | J2、J6、J8、J9、J11–J14、J17 |
| 端到端 | **拍不到正向 Shippable**；**拍不到 replan 执行**；**拍不到 PROPOSED→READY 审查** | §5.5 |

---

# 7. 明确不做的（完整表）

**这一节是 §1.2 的完整表**；每条给出归属与理由。**不建、不留桩。**

| 不建的东西 | 归属 / 理由 |
|---|---|
| Planner、replan 执行、路线替换、Plan／Contract 的新版本 | 第 3 层 Planner 无实现（切分 §6.2 第 13 条 `:412`）；P7A 只出 `Decision` |
| `Intent` 状态机、迁移表、`intent` 表的直接写 | P4；P7A 只调 `save_intent`／`load_intent`／`apply_intent_transition`（§6.3） |
| 把 `PROPOSED` 推进到 `READY` | P4 的审查流程；P7A 只 lower 到 `PROPOSED`，只交接 `READY` |
| `Decision`／`Evidence`／`CompletionVerdict` 的第二份 | P4／P5a；P7A 只消费（§6.2、§6.3） |
| 验证、证据生产、`load_evidence`、Completion 正臂 | P5a／P9；P7A 不在 P5a 之外造「完成」 |
| Resource Arbiter、资源压力、负载类别、挂起相位 | P7C；P7A 的依赖表里没有它 |
| Intent Scheduler、Graph Scheduler、Resource Scheduler、节点放置 | P7B／第 3／4 层；P7A 不调度 |
| Background Supervisor、Session Orchestrator、§171 触发 | P7D；P7A 只按值交 Product Intent |
| UI、CLI、会话展示 | 不在 P7A；本 crate 是库 |
| 完整未来 Milestone 列表 | §233 明文禁止；P7A 只生成有界批次 |
| 九维聚合函数、`progress_delta` 的数值阈值、规范级批次大小 | A15／OPEN-010／OPEN-015 未关闭；本设计只给申报入口（§2.3） |
| 从 `Evidence` 到九维的自动映射 | 规范未给；P7A 只核 id（§2.3 第 12 条） |
| Milestone 状态机（合法迁移表） | §234 没给；P7A 只做乐观并发（§2.3 第 13 条） |
| 第二枚 `ProductId`／第二套九维枚举／第二套 `IntentStatus` | 切分 §四第 3 条；本 crate 用 `ProductIdRef` 与 §232 九名 |

---

# 8. 遗留与未决项

**共 17 条（第 1–16 条为规范／跨块遗留，第 17 条汇总本轮设计自查的阻断项；含 1 条
P6D 并集协调项与 1 条切分偏离待协调）。** 每条具名收件人；凡「缺的是哪一步」都写到步骤，
不用「后续」「长期阶段」兜。**本节不关闭任何一条规范缺口。**

1. **A15／OPEN-010：九维聚合函数与 `progress_delta` 阈值未定义**（§2.3 第 1–3 条；§3.3、
   §3.8）。本设计把 delta 与阈值收成 `ProgressPolicy`；没有策略 ⇒ `Undetermined`。
   **缺的是「九维怎样聚合、连续几个算多个、什么值算 ≈0」这一条规范。**
   **收件人：规范维护者 ＋ P10**（主树 `p8-p10-scope.md:96` 已列）。**不关闭。**

2. **OPEN-015：有限视野与并行／汇合的批次边界未定义**（§2.3 第 4 条；§3.4、§3.5）。
   本设计把边界收成显式 `BatchBoundary`；测试只钉「不出现完整列表」与「评估后再生成」。
   **缺的是「下一批覆盖多少未来状态」这一条规范。**
   **收件人：规范维护者 ＋ P10**（主树 `p8-p10-scope.md:97` 已列）。**不关闭。**

3. **`CompletionVerdict` 没有「完成」正臂，`Shippable` 不可达**（§4.6、§4.7；
   `crates/continuum-verify/src/completion.rs:24`；切分 §6.2 第 7 条 `:406`）。P7A 的
   `evaluate_shippable` 没有正向分支；`ProductPhase::Shippable` 没有生产路径。
   **缺的是「完成」正臂与它与 Shippable 的关系。**
   **收件人：P5a ＋ P9（ENG-004）**。A1／OPEN-001 仍开。**P7A 不造正臂。**

4. **Evidence 只写不读，且没有 Evidence→九维的映射**（§4.2、§4.5；`evidence.rs:117`；
   切分 §6.2 第 6 条 `:405`）。P7A 只收 `&[Evidence]` 值核 id，不读 `evidence` 表，
   不判 claim／type／strength，也不证明某条证据支持某一维。**缺的是 `load_evidence` 与
   「某条证据属于哪一维、方向是正还是负」的判据。**
   **收件人：P5a ＋ 规范维护者。**

5. **replan 的消费方无实现，`Decision` 的选项没有生产者**（§4.5；§3.8；切分 §6.2 第 13 条
   `:412`）。P7A 只决定「是否出」，`ReplanDraft` 的 issue／options／blocking 由调用方给；
   P7A 不执行 replan，不建 Planner。**缺的是「谁消费 Decision、谁给出 A/B/C 路线」这一步。**
   **收件人：第 3 层 Planner 持有方 ＋ 协调者（P8 范围）**。若 P8 不收，就在 P8 新增一项。

6. **§152 的并行／汇合图不能完整 lower 到 §222 的 `Intent`**（§4.4、§4.8）。`Intent` 只有
   单亲 `parent_intent`（`intent.rs:456`），没有 `dependencies[]`；P7A 把图边留在
   `milestone` 表，lower 只产出节点值（每个 Milestone 的 Product Intent 都挂在锚上）。
   **缺的是「Milestone 图边怎样进入 Intent 图」这一条类型／规范。**
   **收件人：P4 ＋ 规范维护者。**

7. **`current_milestone` 是单数，`milestone_history[]` 的元素类型未给**（§4.1；§2.3 第 6、
   15 条）。P7A 把 `current_milestone` 做成游标、把 history 做成 `Vec<MilestoneId>`（按
   `seq` 派生），并行批次里只能有一个「当前」。**缺的是「并行时哪个是 current、history
   存 id 还是快照」这一条规范。**
   **收件人：规范维护者 ＋ 协调者。**

8. **`Product.baseline` 非可空 vs「baseline 缺失或不可判」**（§4.1、§4.3；切分 §2.3.1
   `:131-132`）。P7A 的表示是「没有 Product 行」或「`evaluate_shippable(None, ..)` ⇒
   `Undetermined`」；没有「有 Product 但 baseline 为空」的第三种状态。**缺的是「baseline
   缺失」是否要成为 Product 的一个显式状态。**
   **收件人：规范维护者。**

9. **`ShippablePredicate` 的字段与更新规则未给**（§4.6；§2.3 第 7、16 条）。P7A 取
   `required_dimensions[]`＋`clauses[]`，空谓词拒绝，创建后不可变；assessment 由调用方给。
   **缺的是「谓词有哪些字段、子句怎样评估、产品合同变化时谓词怎样版本化」这一条规范。**
   **收件人：规范维护者 ＋ P4。**

10. **lower 只到 `PROPOSED`，`PROPOSED → READY` 的审查链没有接线**（§4.8；§3.11；
    `intent.rs:249`）。P7A 不推进（那会绕过 `REVIEW`）；`product_intent_for_handoff` 只认
    `READY`，`establish_baseline` 也要求锚 Intent 是 `READY`（§4.1 不变量 4）。**缺的是
    「谁在什么时候把 Product Intent 推进到 READY」这一步。**
    **收件人：P4 ＋ 协调者。** 在这一步接上之前，P7A 既拿不到可建 Product 的锚 Intent，
    P7D 的 `product drive` 也拿不到新值；这是值的运行前置，不是设计串行。

11. **Milestone 的七状态没有迁移表**（§4.4；§2.3 第 13 条；§234 `:498-525`）。P7A 只存
    `state`，`record_milestone_state` 做乐观并发，不判合法对；`Proposed→Completed` 可以
    被写入。**缺的是「Milestone 状态机」这一条规范／归属。**
    **收件人：规范维护者 ＋ P1／协调者。**

12. **`ContinuousProductDrive` 的落点是切分偏离，待协调**（§4.7；§2.3 第 5 条；§336；
    切分 §2.3.1 `:132-133`）。切分只给 `Product`／`ProductReadiness`／verdict 三处；本设计
    提案 (b) 另立 `ProductDriveState`＋`product_drive` 表，**这是第四种落点，不是本设计
    可自定的事项**。协调者须先做 C2 裁定并更新切分 §2.3.1；**C2 裁定是迁移 200 合并/应用的
    硬前置**：裁定前 P7A 不应用 200（没有三表先行版），§4.7 的 API 是提案；裁定后 200
    一次成型（批准提案 (b)（独立表）⇒ 四张表；落点 (a)（既有面）⇒ 三张表）。若选 (a)：
    放进 `Product` 须先解 §231 七字段冻结；放进 `ProductReadiness`／verdict 面则独立表与独立读面作废；
    两路都须由 C2 给出该面的表／列／更新 API，J17b 的运行期与源码面照片延后到那时再写。
    **缺的是「开关存在哪里、post-shippable 是状态还是标签、开关关闭后已开始的 post 工作
    怎样收尾、每一项工作怎样归类」这一条规范，以及切分落点的裁定。**
    **收件人：协调者（裁定并改切分）＋ 规范维护者。**

13. **`ProgressDelta::Regressed` 不直接触发 replan**（§4.5；§2.3 第 14 条；§337 只说
    ≈0）。性能回退可能不会触发 replan；P7A 只把它记录为打断 run 的值。**缺的是「负进展
    是否也算必须 replan 的情形」这一条规范。**
    **收件人：规范维护者 ＋ P10。**

14. **`ShippableVerdict` 是公开枚举，外部可以直接构造 `Shippable`**（§4.6；§2.3 第 8 条）。
    P7A 的 `evaluate_shippable` 是唯一的生产者，但类型层不阻止伪造；`ProductPhase` 的
    相位机也接受这个值。若要求「只有评估器能产出 Shippable」，要把构造点收私有，代价是
    正向相位机在 P5a 之前不可测。**缺的是「verdict 是否需要不可伪造」这一条裁定。**
    **收件人：P5a ＋ P9 ＋ 协调者。**

15. **baseline／predicate 的更新与版本化未定义**（§4.1、§4.3、§4.6；§2.3 第 16 条）。
    P7A 本轮创建后不可变；产品合同变化只能新建 Product 或手工迁移。**缺的是「baseline
    与 ShippablePredicate 变更时怎样产生新版本、旧 Product 怎样处置」这一条规范。**
    **收件人：规范维护者 ＋ P4。**

16. **P7A 新 crate 会让 P6D 的合并门变红，并集的复核归 P7D**（§2.1；切分 §四第 1 条
    `:255-264`、§七第 11 条 `:461`）。P7A 只加自己的 `ALLOWED` 行，不碰
    `p6_pre_allowed.txt`／`P6_EXPECTED_ROWS`；**缺的是「P7D 重新基线化还是退役」的并集
    复核**。**收件人：P7D ＋ 协调者。**

17. **本轮设计自查发现的实现级阻断项（已在正文修掉，列出以便计划阶段逐条有照片）**：
    `MilestoneId.ordinal` 必须是全局序号（I1；§4.4、J8）；`save_lowered_intent` 不再接受
    任意 `Intent`（C3；§4.8、J10）；`lower_milestone` 的跨 Product 校验（I2；§4.8、J10）；
    handoff 按 `MilestoneId` 而非当前游标（I3；§4.8、J11）；`save_milestones` 的孤儿
    Product 校验（I5；§4.4、J8）；`load_product` 对 baseline／predicate／readiness 的逐项
    复核（I6；§4.9、J6）；空 `exit_conditions` 到 P5a `None` 的映射（I7；§4.8、§6.2、J10）。
    **这些不是规范缺口**；它们的收件人是 P7A 的实现与计划评审——**任何一条没有对应照片，
    计划阶段不得开工**。**收件人：P7A 实现者 ＋ 协调者（计划评审）。**

---

**本设计的自评边界**：`Product`／`ProductReadiness`／`Milestone`／`ProgressGateVerdict`／
`ShippableVerdict` 五枚是切分 §2.3.1 指定的块内所有权；本文件没有给 P7B／P7C／P7D 定
任何形状。**§7.4 判据 2 的 replan 在 P7 内不闭合，P7A 只交 `Decision`；§7.4 判据 3 不在
本块；§7.4 判据 4 的正向 `Shippable` 今天没有照片。** 这三句是本设计的验收边界，不是
措辞保留。
