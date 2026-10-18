# P7A（产品循环）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**任务单：** 切分 §五 P7A 行（`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md:332`，逐字）：

> 建产品循环：§150 `EngineeringBaseline` → Product／ProductReadiness／Milestone 惰性生成／
> Milestone→Product Intent lower／Progress Gate／Shippable 与 post-shippable 的区分

**本计划是该行的实现分解。设计与规范唯一事实来源是**
`docs/superpowers/specs/2026-10-18-p7a-product-loop-design.md`（1974 行，下称「设计」）。
**本计划不改设计、不改规范、不改切分文档。** 设计缺项只在文末「遗留/待协调」登记，不在计划正文里就近决定。

**Goal:** 新建 crate `continuum-product`，交付产品循环的八件事：①§150 `EngineeringBaseline`
的生成入口与 `Product` 的唯一创建路径；②§231 `Product` 七字段；③§232 `ProductReadiness`
九维（`value`／`evidence`／`last_verified`，**无聚合、无阈值**）；④§234 `Milestone` 七字段七状态
与**有界批次**的惰性生成；⑤`Milestone → Product Intent` 的确定 lower（只出 `PROPOSED`）与
**只交接 `READY`** 的 handoff；⑥§337 Progress Gate 的三值判定与 replan `Decision`（只出 Decision，
不执行 replan）；⑦§335 `ShippablePredicate` 与三值 fail-closed 的 `ShippableVerdict`；
⑧§336 `ContinuousProductDrive`／`ProductPhase`（**J17b 条件于 C2 裁定**）。
本块**不建** Planner／replan 执行、Intent 状态机、验证与证据生产、Completion 正臂、调度器、UI。

**Architecture:** 本块是**长期循环层（第 7 层）的一个库 crate**，依赖四条 workspace 内边：
`continuum-canonical`（`EntityId`／`Opaque`）、`continuum-persist`（迁移 **200**、`Tx`／`Value`／
`Migration`／`PersistError`）、`continuum-semantics`（`Intent`／`IntentStatus`／`IntentKind`／
`ProductIdRef`／`Priority`／`ContractRef`／`Decision`／`DecisionOption`／`save_intent`／
`load_intent`／`apply_intent_transition`／`create_decision`）、`continuum-verify`（`Evidence`／
`EvidenceId`／`CompletionVerdict`）。**不登记** `continuum-runtime`／provider／model-registry／node／
graph／events／budget／policy／capability 的边。

迁移 **200** 一次成型，但**C2 裁定是硬前置**：提案 (b)（独立 `product_drive` 表）⇒ 四张表
（`product`／`milestone`／`product_progress`／`product_drive`）；落点 (a)（既有面）⇒ 三张表，
设计 §4.7 的独立表与 API 作废。**裁定前不应用 200、不合并 200 的提交，也不存在「三表先行、
之后再补第四张」的路径。** C2 前可合入 Tasks 1–8（纯类型与纯判定），C2 后才执行 Tasks 9–16。

`Milestone` 惰性生成与 lower 是两条独立路径：`propose_next_batch` 只接受显式 `BatchBoundary`，
只把**已生成历史**交给生成器；`lower_milestone` 不跑 Canonicalizer，只产出 `PROPOSED` 的
Product Intent，`save_lowered_intent` 是唯一写路径。handoff 按 `MilestoneId` 取，不看当前游标，
只返回 `READY`；drive gate（C2=(b) 时）只在 `READY` 之后生效。

**Tech Stack:** Rust 1.95.0 / edition 2024（workspace 的 `rust-version = "1.95"`）。
普通依赖：`continuum-canonical`、`continuum-persist`、`continuum-semantics`、`continuum-verify`；
外部依赖：`async-trait`（两个生成端口）、`serde_json`（**内部**存储编码）、`thiserror`
（`ProductError`）。dev 依赖：`tokio`（异步端口测试）、`tempfile`（真库测试）、`trybuild`
（私有字段／无聚合方法／无 verdict 参数的编译期照片）。外部依赖**不进** `ALLOWED`。

**Spec:** 设计全文（**唯一事实来源**）。规范节：§150（`docs/spec/03-product-drive.md:43-114`）、
§231（`docs/spec/05-normative.md:424-438`）、§232（`:442-466`）、§233（`:470-495`）、
§234（`:498-525`）、§335（`:2535-2547`）、§336（`:2551-2580`）、§337（`:2583-2597`）、
§151–§156（`docs/spec/03-product-drive.md:118-406`）；《工程》§7.1–§7.5
（`docs/02-工程.md:420-473`）。切分：`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md`
§一／§二.3.1／§三.6／§四／§五 P7A／§六.1–§六.2。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。`cargo test --workspace --no-fail-fast`
  的**唯一已知红**是 P6D 的 `p6_union_adds_only_its_own_rows`（新 crate 使 P6 合并门到期，
  由 P7D 重新基线化或退役；P7A **不碰** `p6_pre_allowed.txt`／`P6_EXPECTED_ROWS`）。
  `cargo build --workspace --all-targets` 0 warning。
- **§231 七字段逐字**：`id` / `intent_id` / `baseline` / `readiness` / `current_milestone` /
  `milestone_history[]` / `shippable_predicate`。字段**私有**；只派生 `Debug, Clone`；
  **不实现** `Default` / `Deserialize` / `new` / `from_parts`。唯一公开创建路径是
  `establish_baseline`＋`save_product`；crate 私有唯一构造点是 `Product::from_verified_parts`。
- **§232 九维逐字**：`functional` / `correctness` / `reliability` / `performance` / `security` /
  `usability` / `maintainability` / `deployability` / `documentation`。每维是
  `ReadinessDimension { value, evidence, last_verified }`；`value: ReadinessValue`
  （`Unassessed`／`Assessed(Opaque)`）；`evidence: Vec<EvidenceId>`；`last_verified: Option<i64>`。
  **没有** `score()`／`average()`／`sum()`／`aggregate()`；**不做聚合、不做阈值**。
- **A15／OPEN-010 不关闭**：九维聚合函数与 `progress_delta` 阈值仍无规范定义。Progress Gate 只接受
  调用方申报的 `ProgressPolicy`；没有策略 ⇒ `Undetermined(NoPolicy)`；阈值 0 ⇒
  `Undetermined(InvalidThreshold)`；**不得自行 replan**。
- **OPEN-015 不关闭**：批次边界仍无规范定义。`BatchBoundary { max_milestones, max_dependency_depth }`
  是显式参数、**无默认**；超界**拒绝**，不静默截断。测试只钉「不出现完整未来列表」与
  「评估后再生成」，不钉规范级批次大小。
- **`ShippableVerdict` 三值 fail-closed**：`Shippable`／`NotShippable`／`Undetermined`。
  `MissingProduct` 是前置；`CompletionVerdict::Blocked` 与 clause `NotSatisfied` 优先于
  assessment 结构错误与其余不可判项；`Undetermined` **不是许可**。今天 `CompletionVerdict`
  只有 `Blocked`／`Undetermined` 两臂，**`evaluate_shippable` 没有任何一行返回 `Shippable`**；
  P5a 加正臂时那个穷尽 `match` 必须编译不过（刻意 tripwire）。
- **lower 只出 `PROPOSED`**：`lower_milestone` 的 `status` 固定 `IntentStatus::Proposed`；
  `kind = Product`；`product_id = Some(product.id)`；`parent_intent = Some(product.intent_id)`；
  `goal = EntityId::from_str(objective.as_str())`，**不跑 Canonicalizer**；
  `completion_predicate` 是 `exit_conditions` 按序换行连接，**零条 ⇒ `""`**。
  P7A **不**把 `PROPOSED` 推进到 `READY`。
- **handoff 只 `READY`**：`product_intent_for_handoff` 按 `MilestoneId` 取；其余十二个状态
  一律 `Ok(None)`；包括 `PLANNING→SUSPENDED→RUNNING` 的「状态是 RUNNING 但从未过 REVIEW」。
  唯一的状态迁移调用点是 `retire_completed_product_intent`（`COMPLETED → EVOLVING`，仅 Product）。
- **零事件**：`crates/continuum-product/src` 里**不得出现** `append_event`。事件只由上游公开函数写：
  `save_intent` 写 `intent.created`、`create_decision` 写 `decision.required`、
  `apply_intent_transition` 写 `intent.completed`。P7A 不直接写 `events` 表。
- **`ALLOWED` 恰是四条边**：
  `("continuum-product", &["continuum-canonical", "continuum-persist", "continuum-semantics", "continuum-verify"])`。
  内层数组按字母序。**另外** `continuum-runtime` 行加 `continuum-product`（装配方注册迁移；
  这是既有行唯一允许的增枚）。**其他既有行一条都不因本块增枚**；`("continuum-core", &[])` 保持为空。
- **迁移 200**：版本恰 `[200]`，名称 `p7a_product`；**C2 裁定后一次成型**。前三张表
  `product`／`milestone`／`product_progress` 是已定形状；C2=(b) 加 `product_drive`，C2=(a)
  不加且 §4.7 的独立表／API（含 `load_product_drive`）作废。**200 的 SQL 一经应用不得再改**
  （同 110／130／190）；P7A 不占 201–209。
- **共写计数**：`runtime_migrations()`、`tests/migrations.rs` 的 `expected_migrations()`、
  `tests/startup.rs` 三处计数同批改：空库 14→**15**、P0 后 12→**13**、首启 14→**15**。
  **不得只改一处。**
- 所有写 API（`save_product`／`save_milestones`／`record_milestone_state`／
  `record_milestone_progress`／`emit_replan_decision`／`record_shippable_evaluation`／
  `begin_post_shippable_work`／`set_continuous_product_drive`／`set_current_milestone`）
  **不自己 `commit`**；`Err` 后调用方必须回滚。`&Product`／`&Milestone` 必须是同一 `Tx` 里
  `load_*` 得到的当前快照。
- 异步生成入口（`establish_baseline`／`propose_next_batch`）**不接收 `Tx`**；`Tx` 持有
  `MutexGuard`，不得跨 `.await`。
- `ProductIdRef`／`IntentId`／`Opaque`／`EntityId` 都**没有 `PartialEq`**；比较一律按
  `as_str()`；`ContractRef` 比 `id.as_str()`＋`version`。
- 代码注释、错误信息、测试断言信息用中文；标识符用英文。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  新增依赖时 `Cargo.lock` 一并提交。
- `TMPDIR` 取仓库内的 `.tmp/`（`TMPDIR="$PWD/.tmp"`）；变异脚本必须 `trap` 装还原、
  前后各核 `sha256sum`、每轮独立日志路径。
- **范围与设计一致，不多做不少做。** 明确不做：Planner／replan 执行／Plan 或 Contract 新版本、
  `PROPOSED→READY`、`Intent` 状态机、第二枚 `Evidence`／`CompletionVerdict`／`Decision`／
  `IntentStatus`／`ProductId`、九维聚合、规范级阈值、完整未来 Milestone 列表、Milestone 状态机、
  UI／CLI／会话、调度器、资源仲裁、后台监督。

---

## Review Focus

复审时**逐条按下面的用例名核**；每条都归到拥有该代码的 task 的测试，不留在「设计说了」层面。

| # | 要试着推翻的断言 | 归属 task 与用例 |
|---|---|---|
| 1 | **completion `Blocked` 被 assessment mismatch 盖成 `Undetermined`**：assessment 缺一条／多一条，同时 completion 是 `Blocked`，调用方期望 `NotShippable(CompletionBlocked)` | **Task 5** `completion_blocked_wins_over_a_mismatched_assessment`（J15 组合负例 A）＋ `a_not_satisfied_clause_wins_over_an_extra_assessment`（组合负例 B） |
| 2 | **history 重复计数误触发**：`record` 同时出现在 `history` 里，阈值 3、实际连续 2，却数成 3 并写 Decision | **Task 13** `a_record_repeated_in_history_is_counted_once`（J14；去重后 history 不含 `record`，断言 `Ok(None)`、0 行、0 事件） |
| 3 | **跨 Product 的 lower**：拿 A 的 Milestone 给 B lower，产出 `product_id` 与 `parent_intent` 混搭的 Intent | **Task 14** `a_milestone_from_another_product_is_rejected`（J10；`MilestoneProductMismatch`，零 intent 行） |
| 4 | **空 baseline 默认通过**：`product_goal` 或 `minimum_complete_system` 为空时，`evaluate_shippable` 走到 `Shippable` 或 `NotShippable`，而不是 `Undetermined(BaselineUnusable)` | **Task 5** `an_unusable_baseline_is_undetermined`（J6，crate 内防御面）＋ **Task 10** `load_product_rejects_an_empty_baseline_row`（J6，库行读回 `CorruptRow`） |
| 5 | **`READY` 绕过 lower**：调用方直接构造一枚 `READY` 的 Intent，`save_lowered_intent` 收下它并覆盖 lower 的 `PROPOSED`；或 handoff 把 `RUNNING` 当 `READY` | **Task 14** `save_lowered_intent_does_not_accept_an_intent`（trybuild，J10）＋ **Task 15** `a_running_intent_that_never_passed_review_is_not_handed_off`（J11） |

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-19 实读本工作树）

| 依赖 | 实际状态 | 本块用在哪 |
|---|---|---|
| **`continuum-semantics`**：`Intent` 十四成员（`crates/continuum-semantics/src/intent.rs:434`）、`IntentKind`（`:40`）、`IntentStatus` 十三态（`:61`）、`ProductIdRef`（`:372`）、`Priority`（`:300`）、`ContractRef`（`:458`）、`IntentId`／`DecisionId`／`ContractId`（`ids.rs:41`／`:132`／`:92`）、`Decision`／`DecisionOption`（`decision.rs:80`／`:125`）、`save_intent`／`load_intent`（`persist.rs:353`／`:428`）、`apply_intent_transition`（`intent_tx.rs:78`）、`create_decision`（`persist.rs:1173`） | **已交付**。`Intent` 字段全 `pub`；`IntentStatus::transition` 是 `pub(crate)`，外部只能经 `apply_intent_transition`。`READY` 只有 `REVIEW → READY` 一条入边（`intent.rs:249`）。 | lower／handoff／retire／Progress Gate 的 Decision |
| **`continuum-verify`**：`Evidence`（`evidence.rs:222`，八字段私有、`from_tool_result` 唯一构造点）、`EvidenceId`（`:21`）、`CompletionVerdict`（`completion.rs:24`，**只有 `Blocked`／`Undetermined` 两臂**） | **已交付**。`Evidence` **只写不读**（`evidence.rs:117` 明写「消费者未建」；无 `load_evidence`）。`CompletionVerdict` **无正臂**，`judge_completion` 今天恒不返回「完成」。 | readiness 的 `evidence[]`、shippable 的 completion 输入 |
| **`continuum-canonical`**：`Opaque`（`src/lib.rs:152`，`from_str`／`as_str`，无 `PartialEq`）、`EntityId`（`candidate.rs:62`，无 `PartialEq`） | **已交付** | baseline／readiness／predicate／objective 的不透明值；lower 的 `goal` |
| **`continuum-persist`**：`Migration`／`Db`（`db.rs:17`／`:63`）、`Tx`（`tx.rs:12`，`query`／`execute`／`append_event`／`commit`）、`Value`（`value.rs:7`）、`PersistError` | **已交付**。迁移档实测占用 `1`／`2`／`10`／`20`／`30`／`40`／`41`／`50`／`80`／`100`／`110`／`120`／`130`／`190`；**200 空**。`Db::migrate()` 按 `schema_migrations.version` 跳过已应用者。 | 迁移 200、全部落库 API |
| **`continuum-runtime`**：`runtime_migrations()`（`src/main.rs:114`）、`expected_migrations()`（`tests/migrations.rs:37`）、`startup.rs` 三处计数（`:26`／`:81`／`:94`）、`ALLOWED` 的 runtime 行（`dependency_direction.rs:402`） | **已交付**。当前计数 14／12／14；`continuum-runtime` 的 `ALLOWED` 行**没有** `continuum-product`；`crates/continuum-runtime/Cargo.toml` **没有** `continuum-product` 依赖（设计中未列为共写文件，但没有它 `main.rs` 编不过——见「遗留/待协调」第 4 条）。 | 迁移注册、启动计数、依赖方向 |
| **本块新建的 `continuum-product`** | **不存在**（`grep -rn "continuum-product\|continuum_product" crates Cargo.toml` 零命中） | 本计划全部交付物 |

**一句话结论**：四条上游都**已交付且可编译**；本块不需要等任何兄弟 crate 的代码，**只等 C2 裁定**
才能落迁移与落库 API。`CompletionVerdict` 无正臂与 `Evidence` 只写不读是**已实测的缺口**，
不是计划假设；本块照设计的 fail-closed 处置消费它们。

### 二、C2 硬前置：`ContinuousProductDrive` 的落点尚未裁定

**设计 §4.7／§8 第 12 条的原话**：切分 §2.3.1 只授权 `Product`／`ProductReadiness`／verdict
三处；本设计提案 (b) 另立 `ProductDriveState`＋`product_drive` 表，是**第四种落点**，属于切分变更。
协调者须先做 **C2 裁定**并更新切分 §2.3.1；**C2 裁定是迁移 200 合并/应用的硬前置**。

**本计划的执行纪律（不得放宽）**：

1. **Tasks 1–8 可以在 C2 前写完、测绿、合入**：它们只交付纯类型与纯判定，不打开数据库、
   不注册迁移、不碰 `product_drive`。
2. **Task 9 的第一步是「确认 C2 裁定已书面作出且切分 §2.3.1 已更新」**。没有这一步，
   Task 9–16 **一步都不执行**；不得先写三表 SQL 再等裁定。
3. **Task 9 的 200 一次成型**：
   - C2=(b)（批准提案）⇒ `product_drive` 表存在，Tasks 10／15／16 按本计划正文执行；
   - C2=(a)（落既有面）⇒ 200 只有三张表；`product_drive`、`load_product_drive`、
     `record_shippable_evaluation`／`begin_post_shippable_work`／`set_continuous_product_drive`
     全部**不写**；drive 的读/写改用 C2 给出的既有面（表／列／更新 API 随裁定给出）。
     **J17b 的运行期与源码面照片整体延后**，Task 16 取消，Task 15 的 drive gate 改用 C2 面。
4. **J17b 是条件项**：本计划把 J17b 放在 Task 16（仅 (b)）；若 C2=(a)，Task 16 不创建任何文件，
   只在交付报告里写「J17b 延后：等 C2 给出既有面的读/写 API」。
5. **不得把 200 拆成「先三表、后四表」两次提交**；也不得把 `product_drive` 另取 201
   （201 已预留 P7D）。若协调者坚持另取号，那是切分变更，报协调者，本计划不选。

### 三、共写文件（**每一处只加本块自己的一行/一处**）

| 文件 | 本块的改动 | 落在哪个 task |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members`（`:3`） | 加 `"crates/continuum-product"` | Task 1 Step 3 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-product", &["continuum-canonical", "continuum-persist", "continuum-semantics", "continuum-verify"])`；**并在 `continuum-runtime` 行加 `"continuum-product"`** | Task 1 Step 3；runtime 行在 Task 9 Step 4 |
| `crates/continuum-runtime/Cargo.toml` | 加 `continuum-product = { path = "../continuum-product" }`（装配方注册迁移所需；设计 §2.1 的共写表未列，见遗留第 4 条） | Task 9 Step 4 |
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:114`） | 加 `continuum_product::p7a_product_migrations()`（200） | Task 9 Step 4 |
| `crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:37`） | 同批加 200 | Task 9 Step 4 |
| `crates/continuum-runtime/tests/startup.rs` 三处计数（`:26`／`:81`／`:94`） | 空库 14→15、P0 后 12→13、首启 14→15；**同批改** | Task 9 Step 4 |
| `Cargo.lock` | 新增依赖后由 cargo 更新，随 Task 1／Task 9 提交 | Task 1／Task 9 |

**不碰**：`crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`、`P6_EXPECTED_ROWS`、
`crates/continuum-memory/src/persist.rs` 的 190 SQL、`crates/continuum-semantics/**`、
`crates/continuum-verify/**`、`docs/spec/**`、`docs/02-工程.md`、切分与设计。

### 四、可开工顺序与「哪一步能绿」

| task | C2 前可写 | 测试何时能绿 | 卡点 |
|---|---|---|---|
| 1–8（纯类型／纯判定） | **能** | 立即 | 无 |
| 9（迁移 200） | **不能** | C2 后 | C2 裁定＋切分 §2.3.1 更新 |
| 10（Product 落库） | 不能 | 等 Task 9 | 200 的表 |
| 11（Milestone 落库） | 不能 | 等 Task 10 | `load_product` |
| 12（Progress 落库） | 不能 | 等 Task 10／11 | 表与 milestone 读回 |
| 13（Decision） | 不能 | 等 Task 12 | progress 记录与 `create_decision` |
| 14（lower／save） | 不能 | 等 Task 10／11 | Product／Milestone 落库 |
| 15（handoff／retire） | 不能 | 等 Task 14 | lowered Intent；drive 读面（(b) 或 C2=(a) 面） |
| 16（drive 落库，仅 (b)） | 不能 | 等 Task 15 | C2=(b) |
| 17（收尾） | 部分源码面能跑 | 全部落地后 | P6D 合并门是**已知红**，由 P7D 收 |

**「能写」不等于「能合」**：Tasks 1–8 的提交可以合入；Task 9 的提交必须等 C2。执行者在 Task 9
Step 1 未通过时**不得**把 Tasks 9–16 的任何文件写进提交。

### 五、本节把什么交给谁

- **交给 P7D**：`product_intent_for_handoff` 按值交出的 `READY` Product Intent（`kind = Product`、
  `product_id = Some`、`contract_id` 有效）。P7D 不读 Product 记录、不登记 `continuum-product` 边。
- **交给 P5a**：`completion_predicate == ""` 必须映射为 `declared_completion_predicate = None`；
  `Evidence` 只按值进 `record_milestone_progress`，P7A 不读 evidence 表。
- **交给 P4／协调者**：`PROPOSED → READY` 的审查链未接线；P7A 只 lower 到 `PROPOSED`，
  `establish_baseline` 也要求锚 Intent 是 `READY`（值的运行前置，不是设计串行）。
- **交给第 3 层 Planner／P8**：`Decision` 的消费方；P7A 只出 Decision，不执行 replan。
- **交给 P5a／P9**：`CompletionVerdict` 的正臂；正臂出现前 `Shippable` 没有生产路径。
- **交给 P7D／协调者**：P6D 合并门的重新基线化或退役（本块只加自己的 `ALLOWED` 行）。

---

## File Structure

```
crates/continuum-product/
  Cargo.toml            四条普通依赖＋async-trait/serde_json/thiserror；dev：tokio/tempfile/trybuild
  src/lib.rs            crate 文档、模块声明、公开 re-export（不写 #[cfg(test)]，唯一例外见 shippable）
  src/error.rs          ProductError（全部臂一次定义）＋PortError
  src/readiness.rs      ProductReadiness／ReadinessDimension／ReadinessValue／ReadinessDimensionName
  src/predicate.rs      ShippablePredicate／ShippableClause／ShippableClauseId
  src/baseline.rs       EngineeringBaseline／BaselineRequest／EngineeringBaselineGenerator
  src/product.rs        Product／访问器／from_verified_parts／establish_baseline
  src/shippable.rs      ShippableVerdict／NotShippableReason／ShippableUndetermined／
                        ShippableAssessment／ClauseAssessment／ClauseVerdict／evaluate_shippable
  src/drive.rs          ContinuousProductDrive／ProductPhase／ProductDriveState／advance_product_phase
  src/milestone.rs      Milestone／MilestoneId／MilestoneState／BatchBoundary／MilestoneDraft／
                        MilestoneDependency／MilestoneBatchDraft／MilestoneProposalInput／
                        MilestoneGenerator／propose_next_batch
  src/progress.rs       ProgressGateVerdict／ProgressUndetermined／ProgressDelta／ProgressRecord／
                        ProgressPolicy／ReplanDraft／evaluate_progress
  src/lower.rs          lower_milestone（纯构造）
  src/handoff.rs        save_lowered_intent／product_intent_for_handoff／
                        retire_completed_product_intent
  src/persist.rs        p7a_product_migrations（200）、Product／Milestone／Progress／Decision／
                        drive 的落库与读回、内部 JSON 编解码
  tests/support/mod.rs  临时库、假 baseline／milestone 生成器、Intent／readiness 夹具（Task 4 起）
  tests/readiness.rs    J3／J4 的构造面（Task 1）
  tests/predicate.rs    J16 的构造负例（Task 2）
  tests/baseline.rs     J5 的十字段形状与端口（Task 3）
  tests/product.rs      J1／J2 的负例、J5 的生成路径（Task 4）
  tests/shippable.rs    J15／J6 的八分支与组合负例（Task 5）
  tests/drive.rs        J17a 的纯相位机（Task 6）
  tests/milestone.rs    J7／J8／J9 的生成与 DAG（Task 7）
  tests/progress.rs     J12 的纯判定（Task 8）
  tests/migration.rs    J18 的 200 表形（Task 9）
  tests/persist_product.rs   J2 正例、J6 的库行读回（Task 10）
  tests/persist_milestone.rs J7 往返、J8 持久面、J9 依赖往返（Task 11）
  tests/persist_progress.rs  J4／J13 的落库面（Task 12）
  tests/decision.rs          J14（Task 13）
  tests/lower.rs             J10 的构造面（Task 14）
  tests/handoff.rs           J11（Task 15）
  tests/drive_persist.rs     J17b（Task 16，仅 C2=(b)）
  tests/source_surface.rs    J1／J5／J7／J14／J18／J19 的源码面（Task 17）
  tests/type_level.rs        trybuild 驱动（Task 1 起，逐 task 追加样例）
  tests/compile_fail/        readiness_has_no_score／average／sum／aggregate（Task 1）、
                             product_literal_is_private／product_default_is_absent（Task 4）、
                             milestone_literal_is_private（Task 7）、
                             emit_replan_decision_has_no_verdict_parameter（Task 13）、
                             save_lowered_intent_does_not_accept_an_intent（Task 14）
```

**为什么 `Product` 定义在 `src/product.rs` 而 `from_verified_parts` 是 `pub(crate)`**：字段私有只对
定义模块及其后代可见；`persist.rs` 与 `product.rs` 是兄弟模块，`row_to_product` 需要构造
`Product`，故唯一构造点必须是 `pub(crate)` 的关联函数，且只允许 `establish_baseline` 与
`row_to_product` 调用。**不建 `src/builder.rs`**：构造路径只有这一条，builder 会开第二个入口。

**为什么 `ShippablePredicate` 与 `ShippableVerdict` 分两个文件**：谓词是**输入条件**，verdict 是
**判定结果**；设计 §2.5 明写两个词不混用。拆开后 Task 2 的构造负例与 Task 5 的判定负例各自
有独立测试目标，复审可以分别拒绝。

**为什么 drive 的纯相位机与落库分开**：J17a **无条件可拍**（合成 `Shippable`），J17b 条件于 C2。
同文件会让 C2 前的提交带上一段不能测的落库代码；拆开后 Task 6 可以在 C2 前合入。

**没有 `src/state_machine.rs`**：Milestone 七状态没有迁移表（§234 未给），P7A 只做乐观并发；
建一个状态机会发明规范没有的判定。**没有 `src/planner.rs`**：replan 执行不在 P7A。

---

## 测试卫生与四拍约定

- 每个 task 的 Step 1 **只写本 task 的测试**；Step 2 跑精确的 `--test` 目标，记录失败形态；
  Step 3 写生产代码；Step 4 跑本 task 测试＋受影响的上游测试；Step 5 提交。
- **Step 2 的失败形态逐 task 标注**：补丁前新增符号不存在时是 `E0432`／`E0599`／
  「package 未找到」，按 P5c 纪律**不算红**；运行期红只在「符号已存在、行为未实现」时才算。
- `tests/support/mod.rs` 顶部写 `#![allow(dead_code)]`：每个 integration test 各自编译一份
  support，未用到的夹具不应产生 warning。
- `tests/type_level.rs` 的 `.stderr` **必须实跑取**：新增样例先
  `TRYBUILD=overwrite cargo test -p continuum-product --test type_level`，再以普通模式重跑，
  两次都通过才算照片落地。**不要手写 `.stderr`。**
- 所有真库测试用 `tempfile::tempdir()`；测试库的迁移是
  `builtin_migrations()` ＋ `p7a_product_migrations()` ＋（需要 Intent 表时）
  `continuum_semantics::p4_semantics_migrations()`。**不调用 runtime 的 `runtime_migrations()`**。
- 提交信息中文；`git add` 只列显式路径。

---

### Task 1: `continuum-product` 骨架、`ProductError` 与 `ProductReadiness`

**Files:**
- Create: `crates/continuum-product/Cargo.toml`
- Create: `crates/continuum-product/src/lib.rs`
- Create: `crates/continuum-product/src/error.rs`
- Create: `crates/continuum-product/src/readiness.rs`
- Create: `crates/continuum-product/tests/readiness.rs`
- Create: `crates/continuum-product/tests/type_level.rs`
- Create: `crates/continuum-product/tests/compile_fail/readiness_has_no_score.rs`（＋ `.stderr`）
- Create: `crates/continuum-product/tests/compile_fail/readiness_has_no_average.rs`（＋ `.stderr`）
- Create: `crates/continuum-product/tests/compile_fail/readiness_has_no_sum.rs`（＋ `.stderr`）
- Create: `crates/continuum-product/tests/compile_fail/readiness_has_no_aggregate.rs`（＋ `.stderr`）
- Modify: `Cargo.toml`（`[workspace] members`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 product 行）

**Interfaces:**
- Consumes: `continuum_canonical::{EntityId, Opaque}`；`continuum_persist::PersistError`；
  `continuum_semantics::{DecisionError, IntentError}`；`continuum_verify::EvidenceId`
- Produces:
  - `continuum_product::{ProductError, PortError, ProductReadiness, ReadinessDimension,
    ReadinessValue, ReadinessDimensionName}`
  - `ProductError` 的**全部臂**一次定义（设计 §4.9），后续 task 只加 `From` 使用者，不改枚举形状

**本 task 的用例对应 J3 的源码面／运行期，以及 J4 的构造面。** `ProductReadiness` 的
`assessed` 只做**类型级**校验；`evidence` 是否在某一批 `&[Evidence]` 里由 Task 12 的
`record_milestone_progress` 核。**`assessed` 的签名不含维度名**，故其错误里的 `dimension`
写 `"<unbound>"`；绑定维度后的真实名字由 `record_milestone_progress`／`load_product` 重验
（本计划自定，见遗留第 6 条）。

- [ ] **Step 1: 写 `tests/readiness.rs` 与四份 `compile_fail`**

`tests/readiness.rs` 写六条：

- `the_nine_dimensions_are_verbatim_in_order`：读 `include_str!("../src/readiness.rs")`，在
  `pub struct ProductReadiness {` 与随后的 `}` 之间逐行取非空字段名，断言恰是
  `functional` / `correctness` / `reliability` / `performance` / `security` / `usability` /
  `maintainability` / `deployability` / `documentation`，顺序逐字。
- `iter_yields_nine_names_in_normative_order`：`ProductReadiness::unassessed().iter()` 的九个
  `ReadinessDimensionName` 依次等于上面九个名字；`dimension(name)` 对九枚都返回同一个未评记录。
- `an_assessed_dimension_round_trips_its_three_parts`：
  ```rust
  let d = ReadinessDimension::assessed(
      Opaque::from_str("0.8"), vec![EvidenceId::from_str("e1")], 100).unwrap();
  assert!(matches!(d.value(), ReadinessValue::Assessed(v) if v.as_str() == "0.8"));
  assert_eq!(d.evidence(), &[EvidenceId::from_str("e1")]);
  assert_eq!(d.last_verified(), Some(100));
  ```
- `assessed_rejects_an_empty_value`：`Opaque::from_str("")` ⇒
  `Err(ProductError::EmptyReadinessValue { .. })`。
- `assessed_rejects_empty_or_duplicate_evidence`：`vec![]` ⇒ `ReadinessValueWithoutEvidence`；
  `vec![e1, e1]` ⇒ `DuplicateEvidence { id }`；`vec![e1, e2]` ⇒ `Ok`。
- `unassessed_has_no_evidence_or_last_verified`：`ReadinessDimension::unassessed()` 的
  `value().is_unassessed() == true`、`evidence().is_empty()`、`last_verified() == None`；
  `ReadinessValue::as_assessed()` 返回 `None`。

`tests/type_level.rs` 只写驱动：
```rust
#[test]
fn compile_failures_are_photos() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
```
四份样例分别调 `ProductReadiness::unassessed().score()` / `.average()` / `.sum()` / `.aggregate()`。
**期望值手写，不从源码解析结果反向生成。**

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test readiness
```

**结果：** `package ID specification did not match any packages`（crate 尚未建），不是断言失败；
按纪律**不算红**。

- [ ] **Step 3: 建 crate、`ProductError`、`ProductReadiness` 与两处共写**

**动作 3a：** `crates/continuum-product/Cargo.toml`：

```toml
[package]
name = "continuum-product"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
async-trait = { workspace = true }
continuum-canonical = { path = "../continuum-canonical" }
continuum-persist = { path = "../continuum-persist" }
continuum-semantics = { path = "../continuum-semantics" }
continuum-verify = { path = "../continuum-verify" }
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true }
trybuild = { workspace = true }
```

**动作 3b：** `src/error.rs` 按设计 §4.9 一次写全 `ProductError` 与 `PortError`。
`ProductError` 不派生 `PartialEq`；`PortError` 派生 `Debug, Clone, PartialEq`，有
`new(message: impl Into<String>)` 与 `message(&self) -> &str`。
`From<PersistError>`／`From<IntentError>`／`From<DecisionError>`／`From<PortError>` 各一条。
`Persist`／`Intent`／`Decision` 用 `#[from]`；`Port(PortError)` **不用 `#[from]`**（保留
`#[error("端口错误: {0:?}")]`，不要求 `PortError` 实现 `std::error::Error`，设计只给了
`message()`），在 `error.rs` **手写**
`impl From<PortError> for ProductError { fn from(e: PortError) -> Self { Self::Port(e) } }`
——`#[from]` 不会替这一臂生成 `From`，不手写就没有「各一条」。**没有
`ProductError::Invalid`。**

**动作 3c：** `src/readiness.rs` 逐字照设计 §4.2 的五个类型与方法：
`ReadinessValue::{Unassessed, Assessed(Opaque)}`、`ReadinessDimension::{unassessed, assessed,
value, evidence, last_verified}`、`ProductReadiness::{unassessed, dimension, with_dimension, iter}`、
`ReadinessDimensionName` 九枚＋`as_str`／`parse`（`pub(crate)`）。
`assessed` 的校验顺序：空 value ⇒ `EmptyReadinessValue { dimension: "<unbound>" }`；
空 evidence ⇒ `ReadinessValueWithoutEvidence { dimension: "<unbound>" }`；
重复 evidence ⇒ `DuplicateEvidence { id }`。`last_verified` 是 `i64`，`Some` 由类型保证。
**不实现 `Default`／`Deserialize`；不写 `score`／`average`／`sum`／`aggregate`。**

**动作 3d：** `src/lib.rs` 声明 `mod error; mod readiness;`，`pub use` 上述名字。
**动作 3e：** 根 `Cargo.toml` 的 `[workspace] members` 加 `"crates/continuum-product"`；
`ALLOWED` 加 product 行（四条边，内层字母序）与注释「P7A 产品循环（设计 §2.1）」。
**本步不改** runtime 的 `ALLOWED` 行（Task 9 才注册迁移）。

**结果：** `cargo metadata --no-deps` 含 `continuum-product`；`ALLOWED` 的 product 行恰四项；
`readiness.rs` 的九行字段顺序与 §232 一致；`src/` 无四个聚合方法名。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test readiness
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test type_level
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction every_crate_depends_only_on_its_allowed_set
```

**结果：** `readiness` 六条全绿；`type_level` 四份样例按 `.stderr` 编不过；依赖方向用例只跑
`every_crate_depends_only_on_its_allowed_set`（P6D 的合并门不在本步跑，它会在新 crate 进树后
变红，由 P7D 收）。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(product): crate 骨架、ProductError 与 ProductReadiness 九维"
```

---

### Task 2: `ShippablePredicate` 与 `ShippableClause`

**Files:**
- Create: `crates/continuum-product/src/predicate.rs`
- Create: `crates/continuum-product/tests/predicate.rs`
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProductError`、`ReadinessDimensionName`；`continuum_canonical::Opaque`
- Produces: `ShippablePredicate::{new, required_dimensions, clauses}`；
  `ShippableClause::{new, id, statement}`；`ShippableClauseId::{as_str, from_str}`

- [ ] **Step 1: 写 `tests/predicate.rs` 的五条用例**

- `a_predicate_requires_at_least_one_dimension_and_one_clause`：
  `ShippablePredicate::new(vec![], vec![clause])` ⇒ `EmptyShippablePredicate`；
  `new(vec![Functional], vec![])` ⇒ `EmptyShippablePredicate`。
- `duplicate_dimensions_are_rejected`：`vec![Functional, Functional]` ⇒
  `DuplicateDimension { dimension: "functional" }`。
- `duplicate_clause_ids_are_rejected`：两条同 id 的子句 ⇒ `DuplicateClause { id }`。
- `an_empty_clause_id_or_statement_is_rejected`：
  `ShippableClause::new(ShippableClauseId::from_str(""), Opaque::from_str("statement"))` ⇒
  `EmptyClauseId`；`ShippableClause::new(valid_id, Opaque::from_str(""))` ⇒
  `EmptyClauseStatement { id }`。**`ShippableClauseId::from_str("")` 本身返回 `Self`**（空校验
  在 `ShippableClause::new`），不得断言 `from_str("")` 失败。
- `accessors_return_the_constructed_lists_in_order`：构造两维三子句，断言
  `required_dimensions()` 与 `clauses()` 逐项同序；`ShippableClause::id()`／`statement()` 同值。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test predicate
```

**结果：** `E0432`（`ShippablePredicate` 尚未定义）。**不算红。**

- [ ] **Step 3: 实现 `predicate.rs` 并导出**

`ShippablePredicate` 字段私有；只派生 `Debug, Clone`。`new` 的校验顺序：空维度／空子句 ⇒
`EmptyShippablePredicate`；重复维度 ⇒ `DuplicateDimension`；重复 clause id ⇒ `DuplicateClause`。
`ShippableClause::new` 校验空 id 与空 statement。`ShippableClauseId` 是不透明串，
`from_str("")` 本身不失败（空校验在 `ShippableClause::new`）。
`ReadinessDimensionName::as_str` 返回 `functional` 等 §232 名字。

**结果：** 五条用例可列出；`grep -n "fn score\|fn average\|fn sum\|fn aggregate" src/` 零命中。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test predicate
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/predicate.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/predicate.rs
git commit -m "feat(product): ShippablePredicate 与子句的构造校验"
```

---

### Task 3: `EngineeringBaseline`、`BaselineRequest` 与生成端口

**Files:**
- Create: `crates/continuum-product/src/baseline.rs`
- Create: `crates/continuum-product/tests/baseline.rs`
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: `continuum_canonical::Opaque`；Task 1 的 `PortError`／`ProductError`
- Produces: `EngineeringBaseline`（十字段 `pub`）、`BaselineRequest { idea, constraints }`、
  `EngineeringBaselineGenerator`（`#[async_trait]`，返回 `Result<EngineeringBaseline, PortError>`）

- [ ] **Step 1: 写 `tests/baseline.rs` 的三条用例**

- `the_ten_baseline_fields_are_verbatim_in_order`：读 `include_str!("../src/baseline.rs")`，
  在 `pub struct EngineeringBaseline {` 与 `}` 之间取 `pub` 字段名，断言恰是
  `product_goal` / `target_users` / `required_capabilities` / `hard_constraints` / `architecture` /
  `technology_choices` / `minimum_complete_system` / `known_unknowns` / `engineering_risks` /
  `acceptance_criteria`。
- `a_port_error_carries_its_message`：`PortError::new("model unavailable")` 的
  `message() == "model unavailable"`，`Clone`／`PartialEq` 保持值。
- `the_generator_trait_is_usable_from_an_async_test`：实现一个返回固定十字段基线的假生成器，
  `#[tokio::test]` 里 `.generate(&request).await` 得到基线；**不调用模型、不读时钟**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test baseline
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 `baseline.rs`**

`EngineeringBaseline` 的字段全 `pub`（设计 §4.3 的代码块），只派生 `Debug, Clone`；
`BaselineRequest` 字段 `pub`；`EngineeringBaselineGenerator` 用 `#[async_trait::async_trait]`。
私有方法 `pub(crate) fn is_usable(&self) -> Result<(), ProductError>`：
`product_goal.as_str().is_empty()` ⇒ `BaselineUnusable { missing: "product_goal" }`；
`minimum_complete_system.is_empty()` ⇒ `BaselineUnusable { missing: "minimum_complete_system" }`。
**不判**工程好坏、不判 `target_users` 等其余字段。

**结果：** 三条用例绿；`EngineeringBaseline` 恰十个字段。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test baseline
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/baseline.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/baseline.rs
git commit -m "feat(product): EngineeringBaseline 十字段与生成端口"
```

---

### Task 4: `Product` 与 `establish_baseline`

**Files:**
- Create: `crates/continuum-product/src/product.rs`
- Create: `crates/continuum-product/tests/support/mod.rs`
- Create: `crates/continuum-product/tests/product.rs`
- Create: `crates/continuum-product/tests/compile_fail/product_literal_is_private.rs`（＋ `.stderr`）
- Create: `crates/continuum-product/tests/compile_fail/product_default_is_absent.rs`（＋ `.stderr`）
- Create: `crates/continuum-product/tests/compile_fail/product_deserialize_is_absent.rs`（＋ `.stderr`）
- Modify: `crates/continuum-product/src/lib.rs`
- Modify: `crates/continuum-product/tests/type_level.rs`（无需改，wildcard 自动纳入）

**Interfaces:**
- Consumes: Task 1 的 `ProductReadiness`／`ProductError`；Task 2 的 `ShippablePredicate`；
  Task 3 的 `EngineeringBaseline`／`BaselineRequest`／`EngineeringBaselineGenerator`；
  `continuum_semantics::{Intent, IntentKind, IntentStatus, ProductIdRef, IntentId}`
- Produces:
  - `Product` 七个只读访问器；`pub(crate) Product::from_verified_parts(..) -> Result<Self, ProductError>`
  - `pub async fn establish_baseline(intent: &Intent, request: &BaselineRequest,
    shippable_predicate: ShippablePredicate, generator: &dyn EngineeringBaselineGenerator)
    -> Result<Product, ProductError>`

- [ ] **Step 1: 写 `tests/support/mod.rs` 与 `tests/product.rs` 的十二条运行期／源码面用例（另三份 trybuild）**

`support` 至少提供：
```rust
pub fn anchor_intent(product_id: &str, status: IntentStatus) -> Intent { /* 14 字段全填 */ }
pub struct FixedBaselineGenerator { pub result: Result<EngineeringBaseline, PortError> }
impl EngineeringBaselineGenerator for FixedBaselineGenerator { /* async */ }
pub fn usable_baseline() -> EngineeringBaseline { /* 十字段，goal 与 minimum 非空 */ }
pub fn predicate() -> ShippablePredicate { /* 一维一子句 */ }
```

`tests/product.rs`：

- `establish_baseline_derives_the_product_id_from_the_anchor_intent`：锚 Intent
  `kind=Product`、`product_id=Some("p1")`、`status=Ready`、`contract_id={id:"c1",version:1}`；
  断言 `product.id().as_str() == "p1"`、`intent_id().as_str() == "intent/root"`、
  `baseline()` 十字段、`readiness()` 全 `Unassessed`、`current_milestone() == None`、
  `milestone_history().is_empty()`。
- `establish_baseline_rejects_a_task_intent`（`kind=Task` ⇒ `IntentNotProduct`）
- `establish_baseline_rejects_a_missing_product_id`（`None` ⇒ `MissingProductId`）
- `establish_baseline_rejects_an_empty_product_id`（`Some("")` ⇒ `MissingProductId`）
- `establish_baseline_rejects_an_invalid_contract`（`version=0` 或空 id ⇒ `InvalidContract`）
- `establish_baseline_rejects_a_proposed_anchor`（`status=Proposed` ⇒ `AnchorIntentNotReady`）
- `establish_baseline_rejects_a_running_anchor`（`status=Running` ⇒ `AnchorIntentNotReady`）
- `a_generator_error_produces_no_product`：假生成器返回 `Err(PortError::new("boom"))` ⇒
  `Err(ProductError::Port(..))`，**没有 `Product` 值可保存**。
- `an_unusable_baseline_is_rejected`：`product_goal=""` 与 `minimum_complete_system=vec![]`
  两例 ⇒ `BaselineUnusable`。
- `the_product_has_exactly_seven_read_only_accessors`：用函数类型绑定七个访问器，
  断言没有 `&mut self`／`pub` 字段／`into_parts`。
- `the_product_fields_are_verbatim_in_order`：读 `include_str!("../src/product.rs")`，
  `pub struct Product {` 与 `}` 之间恰七行 `id` / `intent_id` / `baseline` / `readiness` /
  `current_milestone` / `milestone_history` / `shippable_predicate`。
- `the_product_has_no_public_constructor_or_deserialize`：`product.rs` 里没有
  `pub fn new`／`pub fn from_parts`／`impl Deserialize`／`Serialize for Product`。
- `a_product_cannot_be_built_from_a_public_literal_or_default`：由 trybuild 三份样例钉住
  （字面量、`Default`、`serde_json::from_str::<Product>`）。

**结果：** 用例名可列出；`tests/type_level.rs` 已存在，新增样例自动纳入。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test product
```

**结果：** `E0432`（`Product`／`establish_baseline` 尚未定义）。**不算红。**

- [ ] **Step 3: 实现 `product.rs` 与 support**

`Product` 字段私有、只派生 `Debug, Clone`；七个访问器逐字照设计 §4.1。
`from_verified_parts` 按入参构造并校验：`id` 非空；`intent_id` 非空；`baseline.is_usable()`；
`current_milestone` 若 `Some` 必须在 `milestone_history` 里；`milestone_history` 无重复。
**1–4 的写路径不变量在 `establish_baseline` 核**（kind／product_id／contract／status）；
`from_verified_parts` 不跨表回查 Intent。

`establish_baseline` 的校验顺序照设计 §4.3：
1. `intent.kind == IntentKind::Product`，否则 `IntentNotProduct { id }`；
2. `intent.product_id` 为 `Some` 且非空，否则 `MissingProductId { intent }`；
3. `contract_id.id` 非空且 `version >= 1`，否则 `InvalidContract { intent }`；
4. `status == IntentStatus::Ready`，否则 `AnchorIntentNotReady { id, status }`；
5. `generator.generate(request).await?`；`baseline.is_usable()?`；
6. 返回 `Product::from_verified_parts(..)`，`readiness` 全 `Unassessed`，
   `current_milestone: None`，`milestone_history: vec![]`。

`status_name` 用私有 `fn status_name(IntentStatus) -> &'static str`（小写、十三臂穷尽），
只用于错误消息，不用于落库。

**结果：** 十二条运行期／源码面用例绿；三份 trybuild 样例编不过（`Product { .. }`、
`Product::default()`、`serde_json::from_str::<Product>`）。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test product
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test type_level
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/product.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/support/mod.rs crates/continuum-product/tests/product.rs crates/continuum-product/tests/type_level.rs crates/continuum-product/tests/compile_fail
git commit -m "feat(product): Product 七字段与 establish_baseline 的唯一创建路径"
```

---

### Task 5: `ShippableVerdict` 与 `evaluate_shippable`

**Files:**
- Modify: `crates/continuum-product/src/shippable.rs`（新建）
- Modify: `crates/continuum-product/src/product.rs`（`#[cfg(test)]` 的测试构造）
- Create: `crates/continuum-product/tests/shippable.rs`
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProductReadiness`／`ReadinessDimensionName`；Task 2 的
  `ShippablePredicate`；Task 4 的 `Product`；`continuum_verify::CompletionVerdict`
- Produces: `ShippableVerdict`／`NotShippableReason`／`ShippableUndetermined`／
  `ShippableAssessment`／`ClauseAssessment`／`ClauseVerdict`；
  `pub fn evaluate_shippable(product: Option<&Product>, assessment: &ShippableAssessment,
  completion: &CompletionVerdict) -> ShippableVerdict`
- Test-only: `#[cfg(test)] pub(crate) fn Product::for_test_with_unusable_baseline(..) -> Product`
  **定义在 `src/product.rs`**（`Product` 字段私有，`shippable.rs` 是兄弟模块，不能自己写
  `Product { .. }`）；只用于 J6 的 `BaselineUnusable` 防御面，生产路径不经过它。
  Task 17 的 J5 允许清单必须把这一处列为 `#[cfg(test)]` 例外。

- [ ] **Step 1: 写 `tests/shippable.rs` 的十二条用例**

- `no_product_is_undetermined_even_when_completion_is_blocked`：
  `evaluate_shippable(None, &assessment, &CompletionVerdict::Blocked { failing: vec![] })`
  ⇒ `Undetermined { reason: MissingProduct }`（J15 组合负例 C）。
- `completion_blocked_wins_over_a_mismatched_assessment`：assessment 缺一条，
  completion 为 `Blocked` ⇒ `NotShippable { reason: CompletionBlocked }`（Review Focus 1）。
- `a_not_satisfied_clause_wins_over_an_extra_assessment`：assessment 多一条，
  某 clause 为 `NotSatisfied` ⇒ `NotShippable { reason: ClauseNotSatisfied(id) }`。
- `a_missing_clause_assessment_is_undetermined`：缺一条且没有明确不满足 ⇒
  `Undetermined { reason: AssessmentMismatch }`。
- `a_duplicate_clause_assessment_is_undetermined`：同一 id 两条 ⇒ `AssessmentMismatch`。
- `an_extra_clause_assessment_is_undetermined`：assessment 比 `predicate.clauses()` 多一条，
  每条都是 `Satisfied`、completion 为 `Undetermined`（**没有 `NotSatisfied` 也没有
  `Blocked`**）⇒ `Undetermined { reason: AssessmentMismatch }`（J16 的「多」纯结构照片）。
- `a_not_satisfied_clause_is_not_shippable`：单个 `NotSatisfied` ⇒ `ClauseNotSatisfied`。
- `an_unassessed_required_dimension_is_undetermined`：`ProductReadiness::unassessed()` 的
  Product ⇒ `RequiredDimensionUnassessed(Functional)`。
- `a_clause_undetermined_is_undetermined`：所有结构对齐、某 clause `Undetermined` ⇒
  `ClauseUndetermined(id)`。
- `completion_undetermined_is_undetermined`：所有前置满足、completion `Undetermined` ⇒
  `CompletionUndetermined`。
- `neither_completion_arm_ever_produces_shippable`：`CompletionVerdict::ALL` 两臂逐一跑，
  断言没有任何分支返回 `Shippable`。
- `the_verdict_does_not_use_clause_statements`：同一 predicate 只改 `ShippableClause` 的
  `statement` 文本（id 不变），同一个 assessment 的 verdict 相同；**P7A 不解析 statement**。

**另在 `src/shippable.rs` 写唯一一处 crate 内 `#[cfg(test)] mod tests`**（设计 J6 明写
「crate 内构造」）：调 `src/product.rs` 里 `#[cfg(test)]` 的
`Product::for_test_with_unusable_baseline` 造一个内存 Product，断言
`evaluate_shippable(Some(&p), ..)` ⇒ `Undetermined(BaselineUnusable)`。构造点本身写在
`src/product.rs`，`shippable.rs` 只调用；Task 17 的单构造点扫描按 `#[cfg(test)]` 例外放行。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test shippable
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 `evaluate_shippable`**

判定顺序**逐字照设计 §4.6**：
1. `product == None` ⇒ `Undetermined(MissingProduct)`；**先于所有其他分支**。
2. `completion == Blocked` ⇒ `NotShippable(CompletionBlocked)`。
3. 按 `predicate.clauses()` 顺序找 assessment；同 id 多条时任一 `NotSatisfied` ⇒ 该子句
   `NotSatisfied`；多枚不满足取 predicate 顺序第一枚 ⇒ `ClauseNotSatisfied(id)`。
4. `product.baseline().is_usable()` 失败 ⇒ `Undetermined(BaselineUnusable)`。
5. assessment 与 predicate.clauses 不是一一对应（缺／多／重复）⇒ `Undetermined(AssessmentMismatch)`。
6. required dimension 未 `Assessed` ⇒ `Undetermined(RequiredDimensionUnassessed(name))`。
7. 某 clause `Undetermined` ⇒ `Undetermined(ClauseUndetermined(id))`。
8. completion `Undetermined` ⇒ `Undetermined(CompletionUndetermined)`。
9. **没有第 9 步**：入口用穷尽 `match completion`，`Blocked` 已在第 2 步返回，
   `Undetermined` 留到第 8 步；**没有任何 `Shippable` 返回行**。P5a 加正臂时这个 match
   编译不过。

`ShippableAssessment`／`ClauseAssessment` 字段 `pub`；`ClauseVerdict` 三值；所有枚举派生
`Debug, Clone, PartialEq`（`ShippableVerdict` 不要求 `Eq`）。

**测试构造放在 `src/product.rs`**：加 `#[cfg(test)] impl Product` 与
`pub(crate) fn for_test_with_unusable_baseline(..) -> Product`，体内用 `Self { .. }` 直接
拼一个 baseline 不可用的值（`Product` 字段私有，兄弟模块 `shippable.rs` 写不出字面量）。
它在测试构建中才存在，不调用 `from_verified_parts`；Task 17 的 J5 扫描把它登记为允许的
`#[cfg(test)]` 例外。

**结果：** 十二条用例＋crate 内一条全绿；`grep -n "Shippable," src/shippable.rs` 只命中枚举定义。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test shippable
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product shippable::tests
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/shippable.rs crates/continuum-product/src/product.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/shippable.rs
git commit -m "feat(product): ShippableVerdict 三值 fail-closed 与 evaluate_shippable"
```

---

### Task 6: `ContinuousProductDrive`、`ProductPhase` 与纯相位机（J17a）

**Files:**
- Create: `crates/continuum-product/src/drive.rs`
- Create: `crates/continuum-product/tests/drive.rs`
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: Task 5 的 `ShippableVerdict`
- Produces: `ContinuousProductDrive { enabled }`（`Default = false`）、
  `ProductPhase::{PreShippable, Shippable, PostShippableImprovement}`、
  `ProductDriveState { continuous_product_drive, phase }`（`Default` 为 disabled＋Pre）、
  `pub fn advance_product_phase(current: ProductPhase, verdict: &ShippableVerdict) -> ProductPhase`

- [ ] **Step 1: 写 `tests/drive.rs` 的五条用例**

- `pre_shippable_advances_only_on_shippable`：`Pre + Shippable ⇒ Shippable`；
  `Pre + NotShippable`／`Undetermined ⇒ Pre`。
- `shippable_is_not_overwritten_by_not_shippable_or_undetermined`：
  `Shippable + Shippable ⇒ Shippable`；`Shippable + NotShippable ⇒ Shippable`；
  `Shippable + Undetermined ⇒ Shippable`（三条都拍，匹配臂的第二臂也有照片）。
- `post_shippable_is_never_overwritten`：`Post + 任何 verdict ⇒ Post`（三臂逐一）。
- `pre_shippable_never_jumps_to_post`：`advance_product_phase(Pre, 任何 verdict)` 从不返回 Post。
- `the_drive_defaults_to_disabled`：`ContinuousProductDrive::default().enabled == false`；
  `ProductDriveState::default().phase == PreShippable`。

**J17a 的 `Shippable` 是合成的**：直接 `ShippableVerdict::Shippable`，并在测试注释里写明
「不是 `evaluate_shippable` 的产物」（今天没有生产者）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test drive
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 `drive.rs`**

三个类型照设计 §4.7；`advance_product_phase` 的 match：
```rust
match (current, verdict) {
    (ProductPhase::PostShippableImprovement, _) => ProductPhase::PostShippableImprovement,
    (ProductPhase::Shippable, _) => ProductPhase::Shippable,
    (ProductPhase::PreShippable, ShippableVerdict::Shippable) => ProductPhase::Shippable,
    (ProductPhase::PreShippable, _) => ProductPhase::PreShippable,
}
```
**函数体内没有 `PostShippableImprovement` 的返回值**；进入 Post 只能经 Task 16 的
`begin_post_shippable_work`。

**结果：** 五条绿；`grep -n "PostShippableImprovement" src/drive.rs` 只命中类型定义与
`advance` 的「保持不变」臂。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test drive
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/drive.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/drive.rs
git commit -m "feat(product): ContinuousProductDrive 三相位只前进的纯相位机"
```

---

### Task 7: `Milestone`、`BatchBoundary` 与 `propose_next_batch`

**Files:**
- Create: `crates/continuum-product/src/milestone.rs`
- Create: `crates/continuum-product/tests/milestone.rs`
- Create: `crates/continuum-product/tests/compile_fail/milestone_literal_is_private.rs`（＋ `.stderr`）
- Modify: `crates/continuum-product/tests/support/mod.rs`（假 Milestone 生成器）
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: Task 4 的 `Product`；Task 3 的 `EngineeringBaseline`；Task 1 的
  `ProductReadiness`；`continuum_semantics::ProductIdRef`；`continuum_canonical::Opaque`
- Produces: 设计 §4.4 的全部类型（`Milestone`／`MilestoneId`／`MilestoneState`／`BatchBoundary`／
  `MilestoneDraft`／`MilestoneDependency`／`MilestoneBatchDraft`／`MilestoneProposalInput`／
  `MilestoneGenerator`）与
  `pub async fn propose_next_batch(product: &Product, history: &[Milestone],
  boundary: BatchBoundary, generator: &dyn MilestoneGenerator) -> Result<Vec<Milestone>, ProductError>`
  ——**`history` 必须由调用方按 `seq` 升序传入（`load_milestones` 的返回序）；本函数
  不校验顺序，也不为错序新增错误臂**（见 Step 3 第 3 条；这是调用方前置，不是本函数的
  判定项）。
- Crate-private constructor: `pub(crate) Milestone::from_verified_parts(id, product_id, objective,
  dependencies, entry_conditions, exit_conditions, state) -> Result<Self, ProductError>`——
  只给 `propose_next_batch` 与 `persist::load_milestone(s)` 用；**不是公开入口**

- [ ] **Step 1: 写 `tests/milestone.rs` 的十四条运行期／源码面用例（另一份 trybuild）**

- `the_seven_milestone_fields_are_verbatim_in_order`：读 `include_str!("../src/milestone.rs")`，
  `pub struct Milestone {` 与 `}` 之间恰 `id` / `product_id` / `objective` / `dependencies` /
  `entry_conditions` / `exit_conditions` / `state`。
- `the_milestone_has_seven_read_only_accessors`：用函数类型绑定 `id`／`product_id`／
  `objective`／`dependencies`／`entry_conditions`／`exit_conditions`／`state`；断言没有
  `&mut self`、`pub` 字段、`into_parts`。
- `the_seven_states_are_listed_in_the_normative_order`：读 `include_str!("../src/milestone.rs")`，
  `pub enum MilestoneState {` 与 `}` 之间恰七枚 `Proposed` / `Approved` / `Active` / `Verifying` /
  `Completed` / `Failed` / `Superseded`。
- **crate 内一条**（`src/milestone.rs` 的 `#[cfg(test)] mod tests`）：
  `the_seven_states_round_trip_through_as_str_and_parse`：七枚 `as_str` 逐一再 `parse` 回原值；
  表外串 `parse` 为 `None`。**`as_str`／`parse` 是 `pub(crate)`**，integration test 调不到，
  故放 crate 内（设计 J7 的「逐枚往返」在编码层）。
- `batch_boundary_rejects_zero_milestones_and_allows_zero_depth`：
  `BatchBoundary::new(0, 0)` ⇒ `InvalidBatchBoundary`；`new(2, 0)` ⇒ `Ok`。
- `propose_next_batch_assigns_global_ordinals_across_two_batches`：第一批 2 枚 ⇒ id 后缀 0/1；
  第二批再 2 枚（history 长度 2）⇒ 后缀 2/3；四枚 id 全局唯一。
- `propose_next_batch_rejects_a_batch_larger_than_the_boundary`：3 枚、边界 2 ⇒
  `BatchTooLarge { limit: 2, actual: 3 }`，**返回的 `Err` 里没有部分批次**。
- `propose_next_batch_rejects_an_empty_batch`：`EmptyBatch`。
- `propose_next_batch_rejects_empty_objectives_and_conditions`：空 objective ⇒
  `EmptyMilestoneObjective`；空 entry element ⇒ `EmptyMilestoneCondition { field:
  "entry_conditions", index }`；空 exit element 同。
- `in_batch_dependencies_may_only_point_backwards`：`InBatch(0)` 在 index 0（自环）⇒
  `DependencyOutOfRange { index: 0 }`；`InBatch(2)` 在 index 1 ⇒ 同。
- `existing_dependencies_must_be_in_history`：`Existing("m-unknown")` ⇒ `UnknownMilestone`。
- `a_diamond_dag_is_accepted_and_fan_out_fan_in_is_preserved`：M1；M2 依赖 M1；M3 依赖 M1；
  M4 依赖 M2＋M3（`InBatch(0)`／`InBatch(1)`）⇒ 接受；M4 的 `dependencies()` 两项同序。
- `the_dependency_depth_limit_is_enforced`：链 M2→M1、M3→M2，`max_dependency_depth=1` ⇒
  `DependencyDepthExceeded { limit: 1, actual: 2 }`；`max_dependency_depth=2` ⇒ 接受。
  **`Existing` 依赖不计入批内深度**（它是已生成历史），故 `max_dependency_depth=0` 仍允许
  「只依赖历史」的批次。
- `the_generator_sees_only_the_generated_history`：假生成器记录 `input.history.len()` 与
  `input.boundary`；第一批看到 0，第二批看到 2；断言 `history` 里没有未来项。
- `there_is_no_api_that_returns_the_whole_future`：源码面 `grep -n "fn all_milestones\|fn full_plan" src/` 零命中。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test milestone
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product milestone::tests
```

**结果：** `E0432`（`propose_next_batch`／`MilestoneState::parse` 尚未定义）。**不算红。**

- [ ] **Step 3: 实现 `milestone.rs` 与 crate 内状态编码测试**

类型照设计 §4.4。`MilestoneId::from_str` 公开（读回与测试）；生产路径由 `propose_next_batch`
派生 `milestone/{product_id}/{ordinal}`，`ordinal = history.len() + i`。
`MilestoneState::as_str`／`parse` 是 `pub(crate)`。

`propose_next_batch` 的校验顺序：
1. `milestones.is_empty()` ⇒ `EmptyBatch`；
2. `len > max_milestones` ⇒ `BatchTooLarge`；
3. `history` 每枚 `product_id().as_str() == product.id().as_str()`，否则
   `MilestoneProductMismatch { milestone, product }`。**`history` 的 `seq` 升序是调用方
   前置，本函数不校验**：调用方必须传 `load_milestones` 的返回序；错序不在
   `propose_next_batch` 新增错误臂（`ProductError` 形状照设计 §4.9 冻结），ordinal 仍按
   `history.len() + i` 派生。**本计划不为错序做兜底断言**；它是接口前置，不是本函数的
   判定项；
4. 每枚 objective 非空 ⇒ 否则 `EmptyMilestoneObjective`；
5. 每枚 `entry_conditions`／`exit_conditions` 元素非空 ⇒ 否则 `EmptyMilestoneCondition`；
6. 依赖：`Existing(id)` 必须在 `history` 里（否则 `UnknownMilestone`）；`InBatch(i)` 必须
   `i < 当前 index`（否则 `DependencyOutOfRange { index: i }`）；
7. 批内深度 `depth[i] = if 有 InBatch 依赖 { 1 + max(depth[j]) } else { 0 }`（`Existing`
   贡献 0）；超过 `max_dependency_depth` ⇒ `DependencyDepthExceeded { limit, actual: max }`；
8. 赋 id 与 `state = Proposed`，返回按输入顺序的 `Vec<Milestone>`。

**不截断**；任何错误都在构造 `Vec` 之前返回，故没有部分批次。

`Milestone::from_verified_parts` 是 crate 内唯一构造点：校验 id 非空、product_id 非空、
objective 非空、`entry_conditions`／`exit_conditions` 元素非空；`propose_next_batch` 与
`persist::load_milestone(s)` 都经它。**crate 外仍写不出 `Milestone { .. }`。**

`src/milestone.rs` 末尾加 `#[cfg(test)] mod tests` 写上面的状态编码往返；**它不改变生产
可见性**（`as_str`／`parse` 仍是 `pub(crate)`）。

**结果：** 十四条 integration／源码面＋一条 crate 内全绿；一份新 trybuild 样例（Milestone
字面量）编不过。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test milestone
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product milestone::tests
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test type_level
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/milestone.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/support/mod.rs crates/continuum-product/tests/milestone.rs crates/continuum-product/tests/type_level.rs crates/continuum-product/tests/compile_fail
git commit -m "feat(product): Milestone 七字段与有界批次的惰性生成"
```

---

### Task 8: `ProgressGateVerdict`、`ProgressPolicy` 与 `evaluate_progress`

**Files:**
- Create: `crates/continuum-product/src/progress.rs`
- Create: `crates/continuum-product/tests/progress.rs`
- Modify: `crates/continuum-product/tests/support/mod.rs`（`DeltaByDimension` 策略）
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProductReadiness`／`ReadinessDimensionName`／`ReadinessValue`；
  `continuum_semantics::ProductIdRef`；Task 7 的 `MilestoneId`
- Produces: 设计 §4.5 的全部类型（`ProgressGateVerdict`／`ProgressUndetermined`／
  `ProgressDelta`／`ProgressRecord`／`ProgressPolicy`／`ReplanDraft`）与
  `pub fn evaluate_progress(previous: &ProductReadiness, current: &ProductReadiness,
  history: &[ProgressRecord], policy: Option<&dyn ProgressPolicy>) -> ProgressGateVerdict`

- [ ] **Step 1: 写 `tests/progress.rs` 与 support 的 `DeltaByDimension`**

`DeltaByDimension` **读真实三元组**，不返回常量：
```rust
fn score(r: &ProductReadiness) -> Option<i64> {
    r.dimension(ReadinessDimensionName::Documentation)
        .value().as_assessed()?
        .as_str().parse().ok()
}
impl ProgressPolicy for DeltaByDimension {
    fn delta(&self, prev: &ProductReadiness, curr: &ProductReadiness) -> ProgressDelta {
        match (score(prev), score(curr)) {
            (Some(a), Some(b)) if b > a => ProgressDelta::Improved,
            (Some(a), Some(b)) if b == a => ProgressDelta::NoProgress,
            (Some(a), Some(b)) => ProgressDelta::Regressed,
            _ => ProgressDelta::Undetermined,
        }
    }
    fn stall_threshold(&self) -> u32 { self.threshold }
}
```

`tests/progress.rs` 八条：
- `no_policy_is_undetermined` ⇒ `Undetermined(NoPolicy)`。
- `a_zero_threshold_is_invalid` ⇒ `Undetermined(InvalidThreshold)`。
- `all_unassessed_readiness_is_undetermined`：previous 与 current 九维全 `Unassessed` ⇒
  `Undetermined(ReadinessUnassessed)`。
- `a_real_policy_reads_the_dimension_and_reports_improved_no_progress_regressed`：
  用 `Documentation` 的 2→3／3→3／3→2 三对 readiness，断言三种 delta；**用 getter 读值**。
- `a_current_improved_or_regressed_resets_the_run`：history 两条 NoProgress、当前 Improved ⇒
  `NotTriggered { consecutive_no_progress: 0, threshold }`。
- `a_no_progress_run_counts_up_to_the_threshold`：阈值 3，历史空、当前 NoProgress ⇒
  `NotTriggered { 1, 3 }`；再接两条 NoProgress ⇒ `Triggered { 3, 3 }`。
- `an_undetermined_history_before_the_threshold_is_undetermined`：当前 NoProgress、历史最近
  一条 `Undetermined`、阈值 3 ⇒ `Undetermined(HistoryUndetermined)`。
- `regressed_does_not_trigger_replan`：当前 `Regressed` ⇒ `NotTriggered { 0, threshold }`。

**结果：** 八条用例名可列出。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test progress
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 `evaluate_progress`**

逐字照设计 §4.5 的八步：
1. `policy == None` ⇒ `Undetermined(NoPolicy)`；
2. `stall_threshold() == 0` ⇒ `Undetermined(InvalidThreshold)`；
3. previous 与 current 九维全 `Unassessed` ⇒ `Undetermined(ReadinessUnassessed)`；
4. `delta = policy.delta(previous, current)`；`Undetermined` ⇒ `Undetermined(DeltaUndetermined)`；
5. 当前 `Improved`／`Regressed` ⇒ `NotTriggered { 0, threshold }`（**不看历史**）；
6. 当前 `NoProgress` ⇒ `count = 1`；从 `history.iter().rev()` 加 `NoProgress`，遇
   `Improved`／`Regressed` 停止；若在达到 threshold 之前遇 `Undetermined` ⇒
   `Undetermined(HistoryUndetermined)`；
7. 任一时点 `count >= threshold` ⇒ `Triggered { count, threshold }`；
8. 否则 `NotTriggered { count, threshold }`；`Regressed` 不触发。

`ProgressRecord` 字段 `pub`；`ProgressPolicy` 是 trait；`ReplanDraft` 字段 `pub`。
**不提供默认策略、不提供默认阈值。**

**结果：** 八条绿；`evaluate_progress` 不产生 `Decision`、不读写库。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test progress
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/progress.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/support/mod.rs crates/continuum-product/tests/progress.rs
git commit -m "feat(product): Progress Gate 三值判定与调用方申报策略"
```

---

### Task 9: 迁移 200（**C2 硬前置**，四表或三表一次成型）

> **STOP GATE：** 本 task **只在 C2 裁定已书面作出、切分 §2.3.1 已更新后执行**。
> 裁定前 Tasks 1–8 可合入；本 task 的文件**不写、不提交**，不得用测试库应用 200，
> 也不得先落三张表。

**Files:**
- Create: `crates/continuum-product/src/persist.rs`（本 task 只放迁移函数）
- Create: `crates/continuum-product/tests/migration.rs`
- Modify: `crates/continuum-product/src/lib.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`
- Modify: `crates/continuum-runtime/src/main.rs`
- Modify: `crates/continuum-runtime/tests/migrations.rs`
- Modify: `crates/continuum-runtime/tests/startup.rs`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（runtime 行加 product）

**Interfaces:**
- Consumes: `continuum_persist::Migration`
- Produces: `pub fn p7a_product_migrations() -> Vec<Migration>`（版本恰 `[200]`，名称
  `p7a_product`）；C2=(b) 四张表，C2=(a) 三张表

- [ ] **Step 1: 核对 C2 裁定（不是代码步骤）**

**动作：** 读协调者的书面裁定与切分 §2.3.1 的更新提交；记录裁定是 (a) 还是 (b)。
**结果：** 裁定文本与提交号写进本 task 的提交信息正文（不是标题）。**没有裁定 ⇒ 停止，
不执行 Step 2–5；本 task 不产生任何文件。**

- [ ] **Step 2: 写 `tests/migration.rs` 的 200 表形照片**

C2=(b) 时：
```rust
#[test]
fn the_200_schema_has_the_four_product_tables() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(columns(&tx, "product"), [
        "id", "intent_id", "baseline", "readiness", "current_milestone",
        "shippable_predicate", "created_at", "updated_at"]);
    assert_eq!(columns(&tx, "milestone"), [
        "id", "product_id", "seq", "objective", "dependencies",
        "entry_conditions", "exit_conditions", "state", "created_at", "updated_at"]);
    assert_eq!(columns(&tx, "product_progress"), [
        "product_id", "milestone_id", "previous_product_state",
        "current_product_state", "progress_delta", "recorded_at"]);
    assert_eq!(columns(&tx, "product_drive"), [
        "product_id", "continuous_product_drive", "phase", "updated_at"]);
}
#[test]
fn the_200_migration_is_exactly_one_version_named_p7a_product() {
    let m = p7a_product_migrations();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].version, 200);
    assert_eq!(m[0].name, "p7a_product");
}
```
C2=(a) 时同一用例**只断三张表**，并加 `assert!(!table_exists(&tx, "product_drive"))`；
`product_drive` 的四列断言删除。**另一支的测试不写入。**

`new_db()` 用 `builtin_migrations()` ＋ `p7a_product_migrations()`＋
`continuum_semantics::p4_semantics_migrations()`（Task 10 起需要 Intent 表；本 task 可先只前两项）。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test migration
```

**结果：** `E0432`（`p7a_product_migrations` 尚未定义）。**不算红。**

- [ ] **Step 4: 写迁移 200 并注册**

**动作 4a：** `src/persist.rs` 写 `p7a_product_migrations()`。C2=(b) 的 SQL：
```sql
CREATE TABLE product (
    id                   TEXT PRIMARY KEY NOT NULL,
    intent_id            TEXT NOT NULL UNIQUE,
    baseline             TEXT NOT NULL,
    readiness            TEXT NOT NULL,
    current_milestone    TEXT,
    shippable_predicate  TEXT NOT NULL,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);
CREATE TABLE milestone (
    id                   TEXT PRIMARY KEY NOT NULL,
    product_id           TEXT NOT NULL,
    seq                  INTEGER NOT NULL,
    objective            TEXT NOT NULL,
    dependencies         TEXT NOT NULL,
    entry_conditions     TEXT NOT NULL,
    exit_conditions      TEXT NOT NULL,
    state                TEXT NOT NULL,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    UNIQUE (product_id, seq)
);
CREATE TABLE product_progress (
    product_id            TEXT NOT NULL,
    milestone_id          TEXT NOT NULL,
    previous_product_state TEXT NOT NULL,
    current_product_state  TEXT NOT NULL,
    progress_delta         TEXT NOT NULL,
    recorded_at            INTEGER NOT NULL,
    PRIMARY KEY (product_id, milestone_id)
);
CREATE TABLE product_drive (
    product_id                TEXT PRIMARY KEY NOT NULL,
    continuous_product_drive  INTEGER NOT NULL,
    phase                     TEXT NOT NULL,
    updated_at                INTEGER NOT NULL
);
CREATE INDEX idx_milestone_product_seq ON milestone(product_id, seq);
```
C2=(a) 时**删掉 `product_drive` 整段**，其余逐字不变。

**动作 4b：** `src/lib.rs` 加 `mod persist;` 与 `pub use persist::p7a_product_migrations;`。

**动作 4c：** `crates/continuum-runtime/Cargo.toml` 加
`continuum-product = { path = "../continuum-product" }`；`main.rs` 的 `runtime_migrations()`
末尾加 `migrations.extend(continuum_product::p7a_product_migrations()); // 200：产品循环四/三表`。
`tests/migrations.rs` 的 `expected_migrations()` 同批加同一行；
`tests/startup.rs` 三处计数改 `15`／`13`／`15`（`:26`／`:81`／`:94`）。
`dependency_direction.rs` 的 runtime 行加 `"continuum-product"`（字母序，插在
`"continuum-policy"` 之后、`"continuum-protected"` 之前）。

**结果：** 200 的表形与 C2 裁定一致；`p7a_product_migrations()` 版本恰 `[200]`；
`grep -n "190" crates/continuum-memory/src/persist.rs` 只命中 `:13` 的迁移编号。

- [ ] **Step 5: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test migration
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test migrations
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test startup
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction every_crate_depends_only_on_its_allowed_set
```

**结果：** `migration` 绿；`migrations` 的集合比对含 200；`startup` 三处计数为 15／13／15；
`git diff --exit-code c89d839 -- crates/continuum-memory/src/persist.rs` 为空（190 未改）。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-product/src/persist.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/migration.rs crates/continuum-runtime/Cargo.toml crates/continuum-runtime/src/main.rs crates/continuum-runtime/tests/migrations.rs crates/continuum-runtime/tests/startup.rs crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(product): 迁移 200 产品循环表（C2 裁定后一次成型）" \
  -m "C2 裁定：<裁定文本摘要>；裁定提交：<commit-sha>；切分 §2.3.1 更新：<commit-sha>"
```

**两个 `-m` 是 Step 1 的落点**：第一个是标题，第二个必须写裁定文本与两个提交号；
缺第二个 `-m` 即本 task 未完成（不得把裁定写进代码注释代替提交信息）。

---

### Task 10: Product 落库：`save_product` / `load_product` / `load_product_drive`

**Files:**
- Modify: `crates/continuum-product/src/persist.rs`
- Modify: `crates/continuum-product/src/lib.rs`（re-export `save_product`／`load_product`／`load_product_drive`）
- Create: `crates/continuum-product/tests/persist_product.rs`

**Interfaces:**
- Consumes: Task 9 的表；Task 4 的 `Product`／`establish_baseline`；
  `continuum_semantics::{load_intent, IntentError, IntentStatus}`；Task 6 的
  `ProductDriveState`（仅 (b)）；Task 1 的 `InvalidContract`／`IntentNotProduct`／
  `MissingProductId`／`AnchorIntentNotReady`
- Produces:
  - `pub fn save_product(tx: &Tx<'_>, product: &Product, occurred_at: i64) -> Result<(), ProductError>`
  - `pub fn load_product(tx: &Tx<'_>, id: &ProductIdRef) -> Result<Option<Product>, ProductError>`
  - `pub fn load_product_drive(tx: &Tx<'_>, id: &ProductIdRef)
    -> Result<Option<ProductDriveState>, ProductError>`（仅 C2=(b)；C2=(a) 由 C2 面替代）

- [ ] **Step 1: 写 `tests/persist_product.rs` 的十四条用例**

- `save_product_reloads_the_anchor_and_requires_ready`：先 `save_intent` 一枚 `Ready` 锚；
  `save_product` 成功；把锚改成 `Proposed` 后另存一个 Product ⇒ `AnchorIntentNotReady`。
- `save_product_rejects_a_missing_anchor_intent`：锚 Intent 从未 `save_intent`
  （`establish_baseline` 用的是内存锚）⇒ `ProductError::Intent(IntentError::UnknownIntent { .. })`，
  **不是** `AnchorIntentNotReady`；`product` 表零行。区分「锚不存在」与「存在但未 READY」。
- `save_product_rejects_an_anchor_whose_contract_became_invalid`：`Ready` 锚（contract 有效）
  `save_intent` 后用 `establish_baseline` 造出 Product；在 `save_product` 之前把同一锚的
  `contract_id.version` 改成 `0`（再 `save_intent`，它不校验 contract）⇒ `save_product`
  ⇒ `InvalidContract { intent }`，`product` 表零行（J2 写路径不变量 3 的重查照片）。
- `a_generator_error_leaves_zero_product_rows`：`establish_baseline` 用返回 `Err` 的假生成器
  ⇒ `Err`，`product` 表零行（J5 的「生成失败 ⇒ 零 Product 行」）。
- `save_product_writes_the_drive_row_disabled_in_pre_shippable`（仅 (b)）：保存后
  `load_product_drive` 为 `enabled=false`、`PreShippable`。
- `save_product_is_outer_insert_only`：同 id 二次保存 ⇒ `ProductExists`，原行不变。
- `a_saved_product_round_trips_its_baseline_readiness_and_predicate`：`establish_baseline`＋
  `save_product` 后 `load_product`；逐项比较 baseline 的十字段、readiness 九维
  （全 `Unassessed`）、predicate 的维度与子句。
- `load_product_returns_none_for_an_unknown_id` ⇒ `Ok(None)`。
- `load_product_rejects_an_empty_baseline_row`：手工 `UPDATE product SET baseline = '{"product_goal":"",...}'`
  ⇒ `CorruptRow { table: "product", .. }`（Review Focus 4）。
- `load_product_rejects_a_predicate_with_a_duplicate_dimension`：手工写重复维度 ⇒ `CorruptRow`。
- `load_product_rejects_an_assessed_dimension_without_evidence`：手工写
  `{"value":"0.8","evidence":[],"last_verified":100}` ⇒ `CorruptRow`。
- `load_product_rejects_an_assessed_dimension_without_last_verified`：手工写
  `{"value":"0.8","evidence":["e1"],"last_verified":null}` ⇒ `CorruptRow`（J4 的
  `Some(last_verified)` 不变量；`assessed` 的签名收 `i64`，运行期构造不出这一行，只能在
  库行读回面拍）。
- `load_product_rejects_a_current_milestone_from_another_product`：写一个别的 Product 的
  Milestone id 到 `current_milestone` ⇒ `CorruptRow`。
- `load_product_rejects_a_current_milestone_in_a_terminal_state`：终态 id 作 current ⇒
  `CorruptRow`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test persist_product
```

**结果：** `E0432`（`save_product` 尚未定义）。**不算红。**

- [ ] **Step 3: 实现 `persist.rs` 的 Product 面与内部编解码**

`save_product`：**在同一 `Tx` 里重新 `load_intent(tx, product.intent_id())` 并重查 §4.1
的 1–4 四条写路径不变量**（锚 Intent 可能在 `establish_baseline` 之后被改动）：
1. `load_intent` 返回 `None` ⇒ `ProductError::Intent(IntentError::UnknownIntent { id })`
   （**与「存在但非 READY」区分**；不把不存在混成 `AnchorIntentNotReady`）；
2. `kind != Product` ⇒ `IntentNotProduct`；
3. `product_id` 缺失或与 `product.id().as_str()` 不一致 ⇒ `MissingProductId`；
4. `contract_id.id` 为空**或 `version < 1`** ⇒ `InvalidContract { intent }`；
5. `status != Ready` ⇒ `AnchorIntentNotReady`。
然后 `INSERT INTO product`；已存在 ⇒ `ProductExists`（主键冲突映射）。C2=(b) 时同 Tx 写
`product_drive`（`0`／`pre_shippable`／`occurred_at`）。**不 commit。**

`load_product`：读 `product` 行；解码 `baseline`／`readiness`／`shippable_predicate`；
查 `milestone` 按 `seq` 得 `milestone_history`；再查 `current_milestone` 的存在性与状态；
`Product::from_verified_parts` 复核；baseline 不可用、predicate 构造失败、readiness 三元组
违反 ⇒ `CorruptRow { table, detail }`。行不存在 ⇒ `Ok(None)`。

`load_product_drive`（仅 (b)）：读行；`continuous_product_drive` 不是 0/1 ⇒ `CorruptRow`；
`phase` 表外 ⇒ `CorruptRow`；行缺失 ⇒ `Ok(None)`。

**内部 JSON 编码（本计划自定，设计 §4.9 允许内部实现）**：
- baseline：十个键逐字；
- readiness：九个键，每项 `{"value": null|"串", "evidence": ["e1"], "last_verified": null|100}`；
  `Assessed` 配 `last_verified: null`、空 evidence 或空 value 均 ⇒ `CorruptRow`；
- predicate：`{"required_dimensions":["functional"], "clauses":[{"id":"c1","statement":"..."}]}`；
- 未知键、缺失键、表外枚举值 ⇒ `CorruptRow`，**不取默认值**。

**结果：** 十四条绿（其中 drive 初始化那条只在 (b) 写；C2=(a) 时该条不写）；
`load_product` 对六种坏行都返回 `CorruptRow`（空 baseline／重复维度／Assessed 空 evidence／
Assessed 空 `last_verified`／跨 Product 的 current／终态 current）。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test persist_product
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/persist.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/persist_product.rs
git commit -m "feat(product): Product 落库与读回，坏行 fail-closed"
```

---

### Task 11: Milestone 落库、状态更新与游标

**Files:**
- Modify: `crates/continuum-product/src/persist.rs`
- Modify: `crates/continuum-product/src/milestone.rs`（`#[cfg(test)]` 的 `for_test`）
- Modify: `crates/continuum-product/src/lib.rs`（re-export 五个 Milestone 落库函数）
- Create: `crates/continuum-product/tests/persist_milestone.rs`

**Interfaces:**
- Consumes: Task 10 的 `load_product`；Task 7 的 `Milestone`／`MilestoneId`／`MilestoneState`
- Produces:
  - `pub fn save_milestones(tx: &Tx<'_>, product: &Product, milestones: &[Milestone],
    occurred_at: i64) -> Result<(), ProductError>`
  - `pub fn load_milestone(tx: &Tx<'_>, id: &MilestoneId) -> Result<Option<Milestone>, ProductError>`
  - `pub fn load_milestones(tx: &Tx<'_>, product_id: &ProductIdRef)
    -> Result<Vec<Milestone>, ProductError>`（按 `seq` 升序）
  - `pub fn record_milestone_state(tx: &Tx<'_>, id: &MilestoneId, expected: MilestoneState,
    to: MilestoneState, occurred_at: i64) -> Result<(), ProductError>`
  - `pub fn set_current_milestone(tx: &Tx<'_>, product_id: &ProductIdRef,
    milestone: Option<&MilestoneId>, occurred_at: i64) -> Result<(), ProductError>`
- Test-only: `#[cfg(test)] pub(crate) fn Milestone::for_test(..) -> Milestone`
  （crate 内测试共用：J8 的「伪造 Milestone」两条、J4 的空 evidence 重验（Task 12）、
  J10 的空 objective（Task 14）；不跨 crate）

- [ ] **Step 1: 写 `tests/persist_milestone.rs` 的十四条用例与 crate 内两条**

- `save_milestones_writes_contiguous_seq_starting_at_zero`：首批 2 枚，`seq` 为 0／1，
  `current_milestone` 指第一枚。
- `two_batches_get_global_ordinals_and_unique_ids`：第二批 2 枚 ⇒ ordinal 2/3、四枚 id 唯一、
  `seq` 0..3。
- `save_milestones_rejects_an_unsaved_product_without_writing`：不 `save_product` 直接保存 ⇒
  `UnknownProduct`，`milestone` 零行。
- `save_milestones_rejects_duplicate_ids_within_a_batch`：同一批两枚同 `MilestoneId`
  （如都写 `milestone/p/0`，使第二枚的 ordinal 也与 `seq` 不符）⇒
  `MilestoneIdCollision { id }`，`milestone` 零行（**不是第二次 INSERT 的底层
  `PersistError`，也不是 `MilestoneOrdinalMismatch`**）。
- `save_milestones_rejects_a_completed_milestone_without_progress`：把一枚置 `Completed`
  但不写 progress，再存新批次 ⇒ `MissingProgressForCompleted`，零新行。
- `load_milestone_round_trips_all_seven_fields`：objective／dependencies／entry／exit／state
  逐项同值。
- `all_seven_states_round_trip_through_the_table`：用 `record_milestone_state` 逐枚把同一行
  置为七状态（乐观并发只要求 expected 等于当前），每次 `load_milestone` 读回同值。
- `an_out_of_table_state_is_a_corrupt_row`：`UPDATE milestone SET state='bogus'` ⇒
  `CorruptRow { table: "milestone" }`。
- `an_empty_objective_or_condition_is_a_corrupt_row`：三行分别 `objective=''`、
  `entry_conditions='[""]'`、`exit_conditions='[""]'` ⇒ 各自 `CorruptRow`。
- `record_milestone_state_checks_expected_state`：expected 不符 ⇒
  `MilestoneStateMismatch { expected, actual }`，行不变。
- `completing_the_current_milestone_advances_the_cursor`：完成 current ⇒ 游标推进到按
  `seq` 的第一枚非终态；没有非终态 ⇒ `NULL`。
- `set_current_milestone_rejects_an_unknown_product`：`product_id` 没有 `product` 行 ⇒
  `UnknownProduct { id }`，**不得 UPDATE 0 行后返回 `Ok`**。
- `set_current_milestone_rejects_a_terminal_or_foreign_milestone`：终态 ⇒ `TerminalMilestone`；
  别的 Product 的 id ⇒ `MilestoneProductMismatch`。
- `dependencies_round_trip_in_order`：M4 依赖 M2＋M3，读回两项同序。

**crate 内两条**（`src/persist.rs` 的 `#[cfg(test)] mod tests`；设计 J8 要求「crate 内构造同值的
`Milestone`」，integration test 写不出私有字段）：
- `save_milestones_rejects_a_forged_empty_milestone`：用 `Milestone::for_test` 造
  objective 为空、entry 元素为空、exit 元素为空三枚；分别调 `save_milestones` ⇒
  `EmptyMilestoneObjective`／`EmptyMilestoneCondition`，`milestone` 零行。
- `save_milestones_rejects_a_forged_ordinal`：`Milestone::for_test` 的 id 为
  `milestone/p/99`、state `Proposed`，保存到空 Product ⇒ `MilestoneOrdinalMismatch`，零行。

**结果：** 十四条 integration 与两条 crate 内用例名可列出；`Milestone::for_test` 只在
`#[cfg(test)]` 下存在。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test persist_milestone
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product persist::tests
```

**结果：** `E0432`（`save_milestones`／`Milestone::for_test` 尚未定义）。**不算红。**

- [ ] **Step 3: 实现 `persist.rs` 的 Milestone 面**

`save_milestones`：开头 `load_product(tx, product.id())`，`None` ⇒ `UnknownProduct`（零行）。
**第一步是批内 `MilestoneId` 集合查重**：一次遍历收集本批 id，重复 ⇒
`MilestoneIdCollision { id }`（与「库中已存在」同一臂）；**这一步先于逐枚 ordinal 后缀
校验、先于任何 INSERT**。否则同 id 两枚的第二枚会先撞 `MilestoneOrdinalMismatch`，与
`save_milestones_rejects_duplicate_ids_within_a_batch` 冲突。随后才逐枚校验：product id
相等、state `Proposed`、objective 非空、条件元素非空、id 未存在、ordinal 后缀等于写入
`seq`。不把批内重复留给第二次 INSERT 的主键冲突。`seq` 从 `COALESCE(MAX(seq), -1) + 1` 起（首批 0；与设计 J8
的「两批 seq 0..3」及 `ordinal = history.len()+i` 对齐。设计 §4.4 正文的「MAX(seq)+1」与
J8 的这一行有张力，本计划按 J8 的 0 起，见遗留第一节第 9 条）。写前查评估门：
```sql
SELECT m.id FROM milestone m
LEFT JOIN product_progress p
  ON p.product_id = m.product_id AND p.milestone_id = m.id
WHERE m.product_id = ?1 AND m.state = 'completed' AND p.milestone_id IS NULL
LIMIT 1
```
有行 ⇒ `MissingProgressForCompleted { milestone }`。**以上校验（含批内查重与评估门）全部
在第一条 `INSERT` 之前完成；任何 `Err` 都保证 `milestone` 零新行**。逐行 `INSERT`；若
`product.current_milestone` 为 `NULL`，设为批次第一枚；`UPDATE product SET updated_at = ?`。

`load_milestone`／`load_milestones`：解码 state／objective／dependencies／conditions；
表外 state、空 objective、空条件元素 ⇒ `CorruptRow`。

`record_milestone_state`：读当前 state；不等于 expected ⇒ `MilestoneStateMismatch`；
`UPDATE milestone SET state, updated_at`；若 `to` 是终态且该 id 是 current，查按 `seq`
第一枚非终态（`state NOT IN ('completed','failed','superseded')`）并写 current，没有写 `NULL`。

`set_current_milestone`：**先 `load_product(tx, product_id)`；`None` ⇒ `UnknownProduct { id }`**
（否则 `Some` 指向已删／未存的 Product 会 UPDATE 0 行后返回 `Ok`）。再处理 `Some`：读
Milestone；不存在 ⇒ `UnknownMilestone`；跨 Product ⇒ `MilestoneProductMismatch`；终态 ⇒
`TerminalMilestone`；写 current 与 `product.updated_at`。`None` 只把 current 置 `NULL`，
仍要求 Product 行存在。

`src/milestone.rs` 加 `#[cfg(test)] pub(crate) fn for_test(id, product_id, objective,
dependencies, entry_conditions, exit_conditions, state) -> Milestone`，**只在测试构建可见**；
它不做校验（生产路径仍只有 `propose_next_batch` 与 `load_*` 两条）。

`src/persist.rs` 的 `#[cfg(test)] mod tests` 用 `for_test` 写上面两条；测试库与
integration test 同形（P0＋200＋语义迁移）。

**结果：** 十四条 integration＋两条 crate 内全绿；两批 `seq` 0..3、id 全局唯一；批内重复
id 与未知 Product 都零行拒绝。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test persist_milestone
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product persist::tests
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/persist.rs crates/continuum-product/src/milestone.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/persist_milestone.rs
git commit -m "feat(product): Milestone 落库、乐观并发状态更新与游标"
```

---

### Task 12: `record_milestone_progress` 与 `load_progress`

**Files:**
- Modify: `crates/continuum-product/src/persist.rs`
- Modify: `crates/continuum-product/src/readiness.rs`（`#[cfg(test)]` 的 `ReadinessDimension::for_test`）
- Modify: `crates/continuum-product/src/lib.rs`（re-export `record_milestone_progress`／`load_progress`）
- Create: `crates/continuum-product/tests/persist_progress.rs`
- Modify: `crates/continuum-product/tests/support/mod.rs`（Evidence 夹具＋readiness 种子）

**Interfaces:**
- Consumes: Task 8 的 `evaluate_progress`／`ProgressRecord`／`ProgressPolicy`；Task 10 的
  `load_product`；Task 11 的 `load_milestones`；`continuum_verify::{Evidence, EvidenceId}`
- Produces:
  - `pub fn record_milestone_progress(tx: &Tx<'_>, product: &Product, milestone: &Milestone,
    current: ProductReadiness, evidence: &[Evidence], policy: Option<&dyn ProgressPolicy>,
    occurred_at: i64) -> Result<(ProgressRecord, ProgressGateVerdict), ProductError>`
  - `pub fn load_progress(tx: &Tx<'_>, product_id: &ProductIdRef)
    -> Result<Vec<ProgressRecord>, ProductError>`（按 `recorded_at`／插入序升序）
- Test-only: `#[cfg(test)] pub(crate) fn ReadinessDimension::for_test(value: ReadinessValue,
  evidence: Vec<EvidenceId>, last_verified: Option<i64>) -> ReadinessDimension`
  （定义在 `src/readiness.rs`；只给 crate 内 J4 的「Assessed 但 evidence 空」重验，
  不做校验，生产路径不经过它）

- [ ] **Step 1: 写 `tests/persist_progress.rs` 的九条用例与 crate 内一条**

- `record_milestone_progress_requires_a_completed_milestone_of_the_product`：`Active` ⇒
  `MilestoneNotCompleted`；跨 Product ⇒ `MilestoneProductMismatch`。
- `record_milestone_progress_rejects_a_second_record_for_the_same_milestone`：
  同一 `(product, milestone)` 二次 ⇒ `DuplicateProgress`。
- `record_milestone_progress_requires_every_referenced_evidence_id`：readiness 引用 `e2`、
  传入 `&[e1]` ⇒ `EvidenceNotSupplied { id: "e2" }`，零 progress 行。
- `a_record_updates_readiness_and_returns_the_pre_write_verdict`：先用
  `support::seed_readiness(&tx, product_id, "2")` 把 Documentation 种到 2（其余八维
  `Unassessed`）；历史空、当前 Documentation 仍 2（NoProgress）、阈值 3 ⇒
  `NotTriggered { 1, 3 }`；写后 `load_product` 的 readiness 等于 current。
- `the_stored_readiness_only_contains_evidence_ids`：原始 `product.readiness` JSON 里只有
  `"e1"`，没有 `Evidence` 的 claim／type／strength。
- `completing_the_current_milestone_advances_the_cursor`：current 完成并记录 progress 后，
  current 推进到下一枚非终态。
- `load_progress_returns_records_in_recorded_at_order`：三条不同 `recorded_at` ⇒ 升序。
- `a_real_policy_progress_sequence_counts_to_the_threshold`（J13）：`seed_readiness`
  种 Documentation=2；用假生成器 `propose_next_batch` 三枚并 `save_milestones`。逐枚先
  `record_milestone_state(.., expected = Proposed, to = Completed, ..)` 置 `Completed`、再
  `load_milestone` 取快照，然后调 `record_milestone_progress`（它要求
  `milestone.state() == Completed`）；三条 `ProgressRecord` **全部由真实 `DeltaByDimension`
  （阈值 3，读 Documentation 的整数刻度）经 `record_milestone_progress` 写入，不手填
  `progress_delta`**；每条前都 `load_product` 取同一 `Tx` 的当前快照、current 都是
  `Documentation=2`、evidence `e1`。verdict 依次
  `NotTriggered { 1, 3 }`／`NotTriggered { 2, 3 }`／`Triggered { 3, 3 }`（第三条的 verdict
  在写入前算出，证明本行不计入自己的 history）；`load_progress` 恰三行，且三行的
  `progress_delta` 都读回 `NoProgress`（证明库里的 delta 来自策略）。
- `an_improved_record_in_a_real_sequence_resets_the_run`（J13）：同一真实策略，
  Documentation 刻度序列 2→2→3→3（四条记录、每枚 Milestone 独立；每条前先
  `record_milestone_state` 置 `Completed`、`load_milestone` 取快照、`load_product` 取当前
  快照，再调 `record_milestone_progress`）⇒ delta 依次
  `NoProgress`／`NoProgress`／`Improved`／`NoProgress`，verdict 依次
  `NotTriggered { 1, 3 }`／`NotTriggered { 2, 3 }`／`NotTriggered { 0, 3 }`／
  `NotTriggered { 1, 3 }`；`load_progress` 读回的四条 delta 与上述逐项相等。

**crate 内一条**（`src/persist.rs` 的 `#[cfg(test)] mod tests`；`ReadinessDimension` 字段
私有且 `#[cfg(test)]` 构造在 integration target 不可见）：
- `record_milestone_progress_rejects_assessed_without_evidence`：用
  `ReadinessDimension::for_test(ReadinessValue::Assessed(Opaque::from_str("0.8")), vec![],
  Some(100))` 经 `ProductReadiness::unassessed().with_dimension(Documentation, d)` 造
  current；先 `save_intent` 一枚 `Ready` 锚，Product 用 `Product::from_verified_parts` 造并
  `save_product`，Milestone 用 `Milestone::for_test`（Proposed）保存后
  `record_milestone_state` 置 `Completed`、再 `load_milestone` 取快照。调
  `record_milestone_progress` ⇒
  `ReadinessValueWithoutEvidence { dimension: "documentation" }`（真实维度名），零 progress
  行、`product.readiness` 未改。**这是 defense-in-depth**：公开 `assessed` 已在 Task 1
  拒绝空 evidence；`record_milestone_progress` 的重验路径仍需照片。

Evidence 夹具：`Evidence::from_tool_result`，`EvidenceProducer::Human`、
`EvidenceSubject::Unattached`、`EvidenceType::Test`、`Claim::from_str("...")`、
`EvidenceStrength::from_value(json!(1))`、`EvidenceScope::from_value(json!({}))`。
`support::seed_readiness(tx, product_id, value)` 只写 `product.readiness` 的内部 JSON
（九键中的 Documentation 为 `Assessed(value)`，其余 `Unassessed`），**不写
`product_progress`**；所有 delta 仍由策略计算。**seed 之后必须 `load_product` 取当前快照**
再调 `record_milestone_progress`（`&Product` 必须是同一 `Tx` 的当前快照）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test persist_progress
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product persist::tests
```

**结果：** `E0432`（`record_milestone_progress`／`ReadinessDimension::for_test` 尚未定义）。
**不算红。**

- [ ] **Step 3: 实现 progress 落库**

前置：`milestone.product_id().as_str() == product.id().as_str()`；`milestone.state() ==
Completed`；`SELECT 1 FROM product_progress ...` 无行。
证据纪律：遍历 `current.iter()`，每维按真实维度名重验：
- `Assessed`：value 非空、evidence 非空且无重复、每个 id 在 `evidence: &[Evidence]` 的
  `EvidenceId` 集合里、`last_verified: Some`；
- `Unassessed`：evidence 空、`last_verified: None`。
**不读 `evidence` 表、不判 claim／type／strength／scope。**

`previous = product.readiness().clone()`；`history = load_progress(tx, product.id())?`
（**写前**）；`verdict = evaluate_progress(&previous, &current, &history, policy)`；
`progress_delta = policy.map(|p| p.delta(&previous, &current)).unwrap_or(Undetermined)`。
**`progress_delta` 一律来自 `policy.delta(previous, current)`**（不是调用方的入参、不是
`evaluate_progress` 的 verdict）；`load_progress` 读回值必须与写入值逐项相等。
同 Tx 写 `product_progress`；`UPDATE product SET readiness = ?, updated_at = ?`；若 current
等于 `product.current_milestone()`，按 Task 11 的同一规则推进游标。返回 `(record, verdict)`。

`load_progress`：按 `ORDER BY recorded_at, rowid`；解码两个 readiness 与 delta；表外 delta ⇒
`CorruptRow`。

`src/readiness.rs` 末尾加
`#[cfg(test)] pub(crate) fn for_test(value: ReadinessValue, evidence: Vec<EvidenceId>,
last_verified: Option<i64>) -> ReadinessDimension`，**不做校验、只在测试构建可见**；生产
构造仍只有 `assessed`／`unassessed`。`src/persist.rs` 的 `#[cfg(test)] mod tests` 用它与
`Milestone::for_test` 写上面那条空 evidence 的重验测试。

**结果：** 九条 integration＋一条 crate 内绿；`grep -n "FROM evidence" src/` 零命中；
真实策略序列与库内 delta 对得上。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test persist_progress
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product persist::tests
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/persist.rs crates/continuum-product/src/readiness.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/persist_progress.rs crates/continuum-product/tests/support/mod.rs
git commit -m "feat(product): Milestone 进度记录、证据 id 核对与写前判定"
```

---

### Task 13: `emit_replan_decision`（只出 Decision，不执行 replan）

**Files:**
- Modify: `crates/continuum-product/src/persist.rs`
- Modify: `crates/continuum-product/src/lib.rs`（re-export `emit_replan_decision`）
- Create: `crates/continuum-product/tests/decision.rs`
- Create: `crates/continuum-product/tests/compile_fail/emit_replan_decision_has_no_verdict_parameter.rs`（＋ `.stderr`）

**Interfaces:**
- Consumes: Task 8 的 `evaluate_progress`／`ReplanDraft`；Task 12 的 `ProgressRecord`；
  `continuum_semantics::{create_decision, Decision, DecisionId, DecisionOption}`
- Produces: `pub fn emit_replan_decision(tx: &Tx<'_>, product: &Product, record: &ProgressRecord,
  history: &[ProgressRecord], policy: Option<&dyn ProgressPolicy>, draft: ReplanDraft,
  occurred_at: i64) -> Result<Option<Decision>, ProductError>`

- [ ] **Step 1: 写 `tests/decision.rs` 的九条用例**

- `a_triggered_recomputation_writes_one_decision_and_one_event`：record＋history＋policy 重算
  为 `Triggered`，draft 两个 options ⇒ `decision` 表 1 行、`decision.required` 事件 1 条；
  options／issue／blocking 原样。
- `a_not_triggered_recomputation_writes_nothing`：重算 `NotTriggered` ⇒ `Ok(None)`、0 行、0 事件。
- `an_undetermined_recomputation_writes_nothing`：`policy=None` ⇒ `Ok(None)`、0／0。
- `a_record_repeated_in_history_is_counted_once`（Review Focus 2）：阈值 3、
  `history = [M2 的 NoProgress, M2 的 Improved, M3 的 NoProgress, record 的副本]`
  （M2 重复且最后一条是 Improved；副本同 `record.milestone_id`）；去重后
  `[M2 的 Improved, M3 的 NoProgress]`，`record` 计 1 ⇒ 实际连续 2，断言 `Ok(None)`、
  0／0；**若保留 M2 的第一条（NoProgress）或没去掉副本，连续数都至少变 3、会误报
  `Triggered`**；不得变成 `Triggered`。
- `an_incorrect_history_count_cannot_trigger`：把 history 里一条 NoProgress 改成 Improved
  （错误计数）⇒ 0／0。
- `a_second_emit_hits_the_decision_primary_key`：同一 draft 二次 ⇒ `Err(ProductError::Decision(..))`，
  仍 1 行。
- `empty_issue_or_options_are_rejected`：空 issue ⇒ `EmptyReplanField { field: "issue" }`；
  空 options ⇒ `EmptyReplanField { field: "options" }`；零行。
- `the_decision_id_is_derived_from_product_and_milestone`：
  `decision/{product_id}/replan/{milestone_id}`；`intent_id == product.intent_id()`。
- `there_is_no_planner_or_replan_execution`：源码面 `grep -n -e "Planner" -e "replan(" src/`
  零命中（排除注释与 `use` 行）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test decision
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 `emit_replan_decision`**

前置：`record.product_id.as_str() == product.id().as_str()`；`record.milestone_id` 属于本
Product（`load_milestone`）；`draft.issue` 非空、`draft.options` 非空。
**重算**：构造去重后的 history——**只做两件事**：①去掉所有
`milestone_id == record.milestone_id` 的行；②其余重复 `milestone_id` 只保留最后一条（保留
最后一条本身、丢弃同 id 的此前各行，输出仍按输入顺序）。**不把 `record` 放回 history**：`record` 只作为
`previous_product_state`／`current_product_state` 参数传入，`evaluate_progress` 第 6 步从
`count = 1` 起算本次记录；放回会让同一条 NoProgress 计两次（阈值 3、实际连续 2 会误报
`Triggered { 3, 3 }`）。然后调
`evaluate_progress(&record.previous_product_state, &record.current_product_state, &deduped, policy)`。
- `Triggered` ⇒ 构造 `Decision { id: DecisionId::from_str(&format!("decision/{}/replan/{}",
  product.id().as_str(), record.milestone_id.as_str())), intent_id: product.intent_id().clone(),
  issue: draft.issue, options: draft.options, blocking: draft.blocking }`，
  `create_decision(tx, &decision, occurred_at)?`，返回 `Some(decision)`；
- `NotTriggered`／`Undetermined` ⇒ **零行、零事件**，返回 `Ok(None)`。

**签名里没有 `ProgressGateVerdict`**；调用方无法把伪造的 `Triggered` 传进来。trybuild 样例
尝试多传一个 verdict 参数，必须编不过（E0061）。

**结果：** 九条绿；`decision` 表与 `events` 表计数精确；源码面零 Planner。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test decision
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test type_level
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/persist.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/decision.rs crates/continuum-product/tests/type_level.rs crates/continuum-product/tests/compile_fail
git commit -m "feat(product): Progress Gate 只出 replan Decision，不执行 replan"
```

---

### Task 14: `lower_milestone` 与 `save_lowered_intent`

**Files:**
- Create: `crates/continuum-product/src/lower.rs`（含 crate 内 `#[cfg(test)] mod tests`）
- Create: `crates/continuum-product/tests/lower.rs`
- Create: `crates/continuum-product/tests/compile_fail/save_lowered_intent_does_not_accept_an_intent.rs`（＋ `.stderr`）
- Modify: `crates/continuum-product/src/handoff.rs`（新建，本 task 只放 `save_lowered_intent`）
- Modify: `crates/continuum-product/src/lib.rs`

**Interfaces:**
- Consumes: Task 4 的 `Product`；Task 7 的 `Milestone`；`continuum_canonical::{EntityId, Opaque}`；
  `continuum_semantics::{Intent, IntentId, IntentKind, IntentStatus, ContractRef, Priority,
  save_intent, load_intent}`
- Produces:
  - `pub fn lower_milestone(product: &Product, milestone: &Milestone, contract: ContractRef,
    priority: Priority, created_at: i64, updated_at: i64) -> Result<Intent, ProductError>`
  - `pub fn save_lowered_intent(tx: &Tx<'_>, product: &Product, milestone: &Milestone,
    contract: ContractRef, priority: Priority, created_at: i64, updated_at: i64)
    -> Result<Intent, ProductError>`

- [ ] **Step 1: 写 `tests/lower.rs` 的十条用例与 `src/lower.rs` 的 crate 内一条**

- `lower_milestone_builds_a_product_intent_in_proposed`：逐字段断言 `id`／`version=1`／
  `kind=Product`／`goal.as_str()==objective`／`status=Proposed`／`parent_intent=Some(锚)`／
  `contract_id`／`active_plan_id=None`／`graph_id=None`／`product_id=Some(product.id)`。
- `two_lowerings_of_the_same_milestone_are_field_identical`：两次调用的六个可比较字段相等。
- `an_empty_exit_condition_list_encodes_as_the_empty_string`：
  `completion_predicate.as_str() == ""`；P5a 侧映射为 `None`（见遗留第 2 条）。
- `exit_conditions_are_joined_in_order`：`["a","b"]` ⇒ `"a\nb"`。
- `a_milestone_from_another_product_is_rejected`（Review Focus 3）：
  `MilestoneProductMismatch { milestone, product }`，零 intent 行。
- `an_invalid_contract_is_rejected`：contract.version=0 或空 id ⇒ `InvalidContract`。
  **空 objective 不在 integration target 拍**：`Milestone` 字段私有、`propose_next_batch`
  与 `load_milestone` 都拒绝空 objective，`tests/lower.rs` 构造不出一枚空 objective 的
  `Milestone`。改由下面的 crate 内用例用 `Milestone::for_test` 拍。
- `save_lowered_intent_writes_once_and_does_not_overwrite_a_ready_intent`：第一次写
  `intent` 行＋`intent.created`；把该行改成 `READY` 后再调，返回现有值，**status 仍 READY**，
  version 不变。
- `save_lowered_intent_rejects_a_field_collision`：同 id 已存在但 goal 不同 ⇒
  `IntentIdCollision { id }`，不写。
- `save_lowered_intent_writes_exactly_one_intent_created_event`：首次 1 条，第二次 0 条。
- `lower_does_not_write_intent_tables_directly`：源码面 `grep -n -e "UPDATE intent" -e
  "INSERT INTO intent" src/` 零命中。

**crate 内一条**（`src/lower.rs` 的 `#[cfg(test)] mod tests`；`Milestone` 字段私有，
`Milestone::for_test` 只在 crate 内可见）：
- `lower_milestone_rejects_an_empty_objective`：用 `Milestone::for_test` 造一枚
  `product_id` 与 Product 相同、`objective = Opaque::from_str("")` 的 Milestone，Product 用
  `Product::from_verified_parts` 造 ⇒ `EmptyMilestoneObjective`。**不调 `propose_next_batch`
  或 `load_milestone`**（它们会先拒绝空 objective，拍不到 `lower_milestone` 自己的防御臂）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test lower
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product lower::tests
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 `lower.rs` 与 `save_lowered_intent`**

`lower_milestone`：先比 `milestone.product_id().as_str()` 与 `product.id().as_str()`；
objective 非空；contract 有效。构造：
```rust
Intent {
    id: IntentId::from_str(&format!("intent/{}", milestone.id().as_str())),
    version: 1,
    kind: IntentKind::Product,
    goal: EntityId::from_str(milestone.objective().as_str()),
    status: IntentStatus::Proposed,
    priority,
    created_at,
    updated_at,
    parent_intent: Some(product.intent_id().clone()),
    contract_id: contract,
    active_plan_id: None,
    graph_id: None,
    product_id: Some(product.id().clone()),
    completion_predicate: Opaque::from_str(&milestone.exit_conditions().iter()
        .map(Opaque::as_str).collect::<Vec<_>>().join("\n")),
}
```
**不跑 Canonicalizer。**

`src/lower.rs` 末尾加 `#[cfg(test)] mod tests`，用 `Milestone::for_test` 与
`Product::from_verified_parts` 写上面的空 objective 用例；**它不改变生产可见性**，也不新增
构造入口。

`save_lowered_intent`：**不接受 `Intent` 参数**；内部重新调 `lower_milestone`。同 id 已存在时
按 `kind`／`goal.as_str`／`product_id.as_str`／`parent_intent.as_str`／`contract.id.as_str`＋
`version`／`completion_predicate.as_str` 六项比较：全同 ⇒ 返回现有值（保留 status）；任一不同
⇒ `IntentIdCollision`。不存在 ⇒ `save_intent(tx, &intent)?`（写 `intent.created`）。
**不直接写 intent 表。**

**结果：** 十条 integration＋一条 crate 内绿；trybuild 样例把一枚 `READY` Intent 传给
`save_lowered_intent` 编不过。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test lower
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product lower::tests
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-product --test type_level
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/lower.rs crates/continuum-product/src/handoff.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/lower.rs crates/continuum-product/tests/type_level.rs crates/continuum-product/tests/compile_fail
git commit -m "feat(product): Milestone→Product Intent 的确定 lower 与唯一写路径"
```

---

### Task 15: `product_intent_for_handoff` 与 `retire_completed_product_intent`

**Files:**
- Modify: `crates/continuum-product/src/handoff.rs`
- Modify: `crates/continuum-product/src/lib.rs`（re-export `product_intent_for_handoff`／`retire_completed_product_intent`）
- Create: `crates/continuum-product/tests/handoff.rs`

**Interfaces:**
- Consumes: Task 14 的 lowered Intent；Task 10 的 `load_product_drive`（仅 (b)）；
  `continuum_semantics::{load_intent, apply_intent_transition, IntentStatus, IntentKind}`
- Produces:
  - `pub fn product_intent_for_handoff(tx: &Tx<'_>, product: &Product,
    milestone: &MilestoneId) -> Result<Option<Intent>, ProductError>`
  - `pub fn retire_completed_product_intent(tx: &Tx<'_>, intent_id: &IntentId,
    occurred_at: i64, event_id: &str) -> Result<(), ProductError>`

- [ ] **Step 1: 写 `tests/handoff.rs` 的七条用例**

- `only_ready_intents_are_handed_off`：按 id 构造十三个状态各一枚 Intent；只有 `Ready` 返回
  `Some`，其余十二个 `None`。
- `two_ready_intents_are_both_handed_off_by_milestone_id`：两枚 READY、
  `current_milestone` 只指一个；两次按 id 调用都 `Some`。
- `a_running_intent_that_never_passed_review_is_not_handed_off`（Review Focus 5）：
  `PLANNING→SUSPENDED→RUNNING` 的 Intent（状态 RUNNING）⇒ `None`。
- `a_missing_lowered_intent_is_not_an_error`：Milestone 存在但没有 Intent ⇒ `Ok(None)`。
- `a_malformed_lowered_intent_is_rejected`：同 id 的 Intent 是 `kind=Task`、product_id 不匹配、
  或 contract 无效 ⇒ `InvalidLoweredIntent { id }`。
- `retire_moves_completed_to_evolving_and_stops_handoff`：`COMPLETED` Product Intent 调
  `retire_completed_product_intent` 后 `load_intent` 为 `Evolving`；再 handoff ⇒ `None`。
- `handoff_requires_a_drive_state`（仅 (b)）：手工删除 `product_drive` 行 ⇒
  `MissingDriveState { product }`；C2=(a) 时改用 C2 面的「缺状态」错误。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test handoff
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现 handoff 与 retire**

`product_intent_for_handoff`：
1. `load_milestone(tx, milestone)`；`None` ⇒ `UnknownMilestone`；
2. `milestone.product_id().as_str() != product.id().as_str()` ⇒ `MilestoneProductMismatch`；
3. 派生 `intent/{milestone_id}`，`load_intent`；`None` ⇒ `Ok(None)`；
4. `kind != Product`／`product_id` 不匹配／contract 无效 ⇒ `InvalidLoweredIntent { id }`；
5. `status != Ready` ⇒ `Ok(None)`（**十二个状态全部走这里**）；
6. drive gate：`load_product_drive`；`None` ⇒ `MissingDriveState`；若
   `phase == PostShippableImprovement && !enabled` ⇒ `Ok(None)`；
7. `Ok(Some(intent))`。

C2=(a) 时第 6 步改用 C2 给出的既有面读/写 API；**具体表／列／错误随裁定**，本计划不预写。

`retire_completed_product_intent`：`load_intent`；`None` ⇒
`ProductError::Intent(IntentError::UnknownIntent { id })`；`kind != Product` 或
`status != Completed` ⇒ `InvalidLoweredIntent`；调
`apply_intent_transition(tx, intent_id, IntentStatus::Evolving, occurred_at, event_id)?`。
**不自己写 intent 表。**

**结果：** 七条绿；`grep -n "apply_intent_transition" src/` 只命中 `retire_completed_product_intent`
一处（排除 `use` 行）。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test handoff
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/handoff.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/handoff.rs
git commit -m "feat(product): 只交接 READY 的 Product Intent 与 COMPLETED→EVOLVING"
```

---

### Task 16: Drive 落库与 J17b（**仅 C2=(b)**）

> **条件 task：** C2=(a) 时**整 task 不执行**，不创建 `tests/drive_persist.rs`，不写
> `record_shippable_evaluation`／`begin_post_shippable_work`／`set_continuous_product_drive`，
> 不写 `product_drive` 的 SQL。J17b 的运行期与源码面照片整体延后，等 C2 给出既有面的
> 表／列／更新 API 后再补。**不得用 (b) 的 API 去猜 (a) 的形状。**

**Files:**
- Modify: `crates/continuum-product/src/persist.rs`
- Modify: `crates/continuum-product/src/lib.rs`（re-export 三个 drive 落库函数）
- Create: `crates/continuum-product/tests/drive_persist.rs`

**Interfaces:**
- Consumes: Task 6 的 `ProductDriveState`／`advance_product_phase`；Task 10 的
  `load_product_drive`；Task 15 的 handoff
- Produces:
  - `pub fn record_shippable_evaluation(tx: &Tx<'_>, product_id: &ProductIdRef,
    verdict: &ShippableVerdict, occurred_at: i64) -> Result<ProductDriveState, ProductError>`
  - `pub fn begin_post_shippable_work(tx: &Tx<'_>, product_id: &ProductIdRef,
    milestone: &MilestoneId, occurred_at: i64) -> Result<ProductDriveState, ProductError>`
  - `pub fn set_continuous_product_drive(tx: &Tx<'_>, product_id: &ProductIdRef,
    enabled: bool, occurred_at: i64) -> Result<ProductDriveState, ProductError>`

- [ ] **Step 1: 写 `tests/drive_persist.rs` 的八条用例**

- `set_continuous_product_drive_changes_only_the_switch`：true／false 往返，phase 不动。
- `begin_post_shippable_work_rejects_pre_shippable` ⇒ `NotShippableYet`。
- `begin_post_shippable_work_rejects_a_disabled_drive`：phase 先置 `Shippable`、开关 false ⇒
  `DriveDisabled`。
- `begin_post_shippable_work_enters_post_once`：Shippable＋true ⇒ Post；再调 ⇒ 幂等 Ok。
- `recording_not_shippable_never_leaves_pre_shippable`：`record_shippable_evaluation` 用
  `NotShippable`／`Undetermined` ⇒ 仍 Pre。
- `recording_shippable_then_not_shippable_does_not_regress`：合成 `Shippable` 后
  `NotShippable` ⇒ 仍 Shippable。
- `the_phase_has_a_single_write_point`：源码面 `grep -n "phase = ?" src/` 只命中
  `product_drive` 的 phase 写点（排除 `let phase =`）。
- `handoff_gate_closes_when_the_drive_is_disabled_in_post_phase`：Post 相位＋开关 false 时，
  该 Milestone 的 READY Intent handoff ⇒ `None`；`set_continuous_product_drive(true)` 后同一
  调用 ⇒ `Some`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test drive_persist
```

**结果：** `E0432`。**不算红。**

- [ ] **Step 3: 实现三个 drive API**

`record_shippable_evaluation`：`load_product_drive`（`None` ⇒ `MissingDriveState`）；
`next = advance_product_phase(current, verdict)`；若变，`UPDATE product_drive SET phase = ?,
updated_at = ?`；返回新状态。**不存 verdict 本身。**
`begin_post_shippable_work`：`load_milestone` 校验属于本 Product 且不终态；`load_product_drive`；
`Post` ⇒ 幂等 `Ok`；`Pre` ⇒ `NotShippableYet`；`Shippable`＋`!enabled` ⇒ `DriveDisabled`；
`Shippable`＋`enabled` ⇒ 写 Post。
`set_continuous_product_drive`：只写 `continuous_product_drive`（0/1）与 `updated_at`，不改 phase。

**结果：** 八条绿；J17b 的运行期与源码面照片落地（仅 (b)）。

- [ ] **Step 4: 运行本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test drive_persist
```

**结果：** 本 task 的测试目标全绿；没有 assertion 失败。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/src/persist.rs crates/continuum-product/src/lib.rs crates/continuum-product/tests/drive_persist.rs
git commit -m "feat(product): ProductDrive 落库、post-shippable 进入与交接 gate"
```

---

### Task 17: 收尾：J19、源码面复核、全量门与变异抽查

**Files:**
- Create: `crates/continuum-product/tests/source_surface.rs`
- Modify: `crates/continuum-product/tests/type_level.rs`（只在需要时补样例；通常已齐）
- 不改生产代码（除复核发现的缺陷；若改，回到对应 task 的用例）

**Interfaces:**
- Consumes: 全部前序 task 的公开面
- Produces: 无新公开 API；只交付源码面照片与全量复核

- [ ] **Step 1: 写 `tests/source_surface.rs` 的十三条用例（C2=(b) 另加一条）**

- `there_is_no_second_product_id_intent_status_evidence_completion_or_decision`：遍历
  `src/` 全部文件，断言 `struct ProductId`／`enum IntentStatus`／`struct Evidence`／
  `enum CompletionVerdict`／`struct Decision` 零命中。
- `the_product_has_a_single_private_construction_point`：扫 `src/` 并**排除注释行**。
  `Product {` 的允许命中逐条登记为：① `struct Product {` 字段定义；② 生产
  `impl Product {` 的访问器块；③ **`src/product.rs` 的 `#[cfg(test)] impl Product {` 测试
  构造块**（J6 的 `for_test_with_unusable_baseline`）。`Product::from_verified_parts` 的
  **生产调用点恰 `establish_baseline`／`row_to_product` 两处**；测试里还有两处调用：
  ④ `src/persist.rs` 的 `#[cfg(test)] mod tests`（Task 12 的
  `record_milestone_progress_rejects_assessed_without_evidence`）；⑤ `src/lower.rs` 的
  `#[cfg(test)] mod tests`（Task 14 的 `lower_milestone_rejects_an_empty_objective`）。
  ④⑤ 逐条登记为 `#[cfg(test)]` 例外；对 `Product::from_verified_parts` 的调用按
  `#[cfg(test)]` 块切分并断言：**块外恰 2 处、块内恰 2 处且各落在上述具名测试**；清单外
  命中即红（J5）。`Self {` 中属于 `Product`
  构造点的恰两处：`from_verified_parts` 体内一处、`#[cfg(test)]
  for_test_with_unusable_baseline` 体内一处；其余 `Self {` 必须逐条登记为「非 `Product`
  类型」，不能归入 `Product` 构造点。**`#[cfg(test)]` 的第三构造点是设计 J6 明文允许的
  例外，必须列在清单里，不能算作生产构造点**。
- `there_is_no_direct_sql_against_intent_decision_or_evidence`：`FROM intent`／
  `FROM decision`／`FROM evidence` 零命中（`load_intent` 等公开调用不算 SQL）。
- `there_are_no_planner_replan_contract_or_plan_writes`：`Planner`／`replan(`／
  `save_contract`／`save_plan` 零命中（排除注释与 `use` 行）。
- `there_is_no_full_future_milestone_api`：`all_milestones`／`full_plan` 零命中。
- `there_is_no_readiness_aggregate_method`：`fn score`／`fn average`／`fn sum`／
  `fn aggregate` 零命中。
- `the_only_intent_transition_call_is_retire`：`apply_intent_transition` 只命中
  `handoff.rs` 的 `retire_completed_product_intent`（排除 `use`）。
- `there_is_no_direct_event_append`：遍历 `src/` 全部 `.rs` 并**排除注释行**，
  `append_event` 零命中；事件只能由 `save_intent`／`create_decision`／
  `apply_intent_transition` 间接写（Global Constraints 的「零事件」面）。
- `the_crate_has_exactly_the_four_allowed_workspace_dependencies`：`cargo tree -p
  continuum-product --depth 1 --edges normal --prefix none`，过滤 `continuum-` 行、去掉根包，
  集合恰为 `continuum-canonical`／`continuum-persist`／`continuum-semantics`／
  `continuum-verify`。
- `the_product_crate_does_not_depend_on_runtime_provider_registry_node_or_context`：上述树里
  `continuum-runtime`／`continuum-provider`／`continuum-model-registry`／`continuum-node`／
  `continuum-context` 零命中。
- `the_allowed_row_is_exactly_four_entries`：读
  `include_str!("../../continuum-runtime/tests/dependency_direction.rs")`，定位
  `"continuum-product"` 行，解析四个字符串并断言之。
- `the_p6_fixture_is_untouched`：`git diff --exit-code c89d839 --
  crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 为空；`P6_EXPECTED_ROWS` 的
  五行未变（源码面计数）。
- `the_190_sql_is_unchanged`：`git diff --exit-code c89d839 --
  crates/continuum-memory/src/persist.rs` 为空。
- (b) 加一条 `the_phase_has_a_single_sql_write_point`：`grep -n "phase = ?" src/` 只命中
  `product_drive` 的 phase 写点。

- [ ] **Step 2: 运行源码面与全量测试**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test source_surface
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-product --test type_level
TMPDIR="$PWD/.tmp" timeout 1800 cargo test -p continuum-product
TMPDIR="$PWD/.tmp" timeout 1800 cargo build --workspace --all-targets
```

**结果：** `source_surface` 全绿；`type_level` 全绿；`continuum-product` 全部测试目标绿；
`build` 0 warning。

- [ ] **Step 3: 全量门（含已知红）**

```bash
TMPDIR="$PWD/.tmp" timeout 3600 cargo test --workspace --no-fail-fast
```

**结果：** 除 `p6_union_adds_only_its_own_rows` 外全绿。**这是设计预期的到期日**：P7A 的新
crate 让 P6D 合并门变红，由 P7D 重新基线化或退役；P7A **不碰** `p6_pre_allowed.txt` 与
`P6_EXPECTED_ROWS`。若出现**第二个**失败测试，停在原地，按该测试所属的 task 回查，不得
把 P6D 的已知红当作遮挡。

- [ ] **Step 4: 变异抽查（设计 §5.4 的 20 行中抽 6 行）**

**抽样说明：** 设计 §5.4 的表共 **20 行**；本计划只抽 **#1／#2／#3／#5／#7／#11** 六行
（30%），覆盖 J1／J3／J4／J8／J10／J15 六个判据。**未抽查的 14 行（#4／#6／#8／#9／#10／
#12–#20）不是本块的执行门**，也不构成本计划对设计 §5.4 的完整覆盖；完整表以设计为准。
每次只改一处，跑指定测试，确认变红，再用 `git checkout -- <file>` 还原；前后各跑
`sha256sum`。**不提交变异。**

| 设计 §5.4 行 | 变异 | 该红的用例 |
|---|---|---|
| #1 | `Product` 加第八个字段 | Task 4 `the_product_fields_are_verbatim_in_order` |
| #2 | `ProductReadiness` 加 `score()` | Task 1 `the_nine_dimensions_…` 的源码面＋`readiness_has_no_score` |
| #3 | `assessed` 接受空 evidence | Task 1 `assessed_rejects_empty_or_duplicate_evidence` |
| #5 | 批次超界静默截断 | Task 7 `propose_next_batch_rejects_a_batch_larger_than_the_boundary` |
| #7 | lower 把 `status` 写成 `READY` | Task 14 `lower_milestone_builds_a_product_intent_in_proposed` |
| #11 | `evaluate_shippable` 把 assessment mismatch 放到 `Blocked` 之前 | Task 5 `completion_blocked_wins_over_a_mismatched_assessment` |

**结果：** 六次都按表变红；还原后 `git status` 干净。若某次不变红，补该 task 的用例，
**不在 Task 17 里改生产代码**。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-product/tests/source_surface.rs
git commit -m "test(product): J19 依赖面、源码面与全量复核"
```

---

## 自审

### 一、Spec coverage（设计 §1–§8 每节有 task）

| 设计节 | 落点 task | 说明 |
|---|---|---|
| §1.1 七项具名义务 | 1／2／3／4／5／6／7／8／9–16 | 七字段、九维、baseline、Milestone、lower、Progress Gate、Shippable、drive 各至少一个 task |
| §1.2 不建什么 | Global Constraints＋§7 表 | 不建 Planner／状态机／第二份类型／完整未来 |
| §2.1 crate 与四条边 | Task 1／9／17 | Cargo.toml、`ALLOWED`、`cargo tree` |
| §2.2 冻结清单（34 类型＋3 trait） | Tasks 1–8、14、15、16 | 全部公开名逐 task 定义；没有 `ProductId` |
| §2.3 十七条发明判据 | #1 Task 1／5；#2–#3 Task 8；#4 Task 7；#5 Task 6／15／16；#6 Task 4；#7–#8 Task 2／5；#9 Task 3／4；#10 Task 14；#11 Task 13；#12 Task 12；#13 Task 11；#14 Task 8；#15 Task 11；#16 Task 4；#17 Task 11 | 每条都有实现与照片；#17 的强制点在 `save_milestones` |
| §2.4 切分六条横切 | Task 1／9／17＋Global Constraints | 反向边、迁移档、词汇表、三值、依赖精确、按值交接 |
| §2.5 同名异物 | Global Constraints＋Task 17 | 无 `ProductId`／无第二套九维／不碰 policy 的 Decision |
| §3.1–§3.11 规范依据 | Tasks 1–16 | 每个类型／判定对着对应规范节 |
| §4.1 Product | Task 4／10 | 七字段、唯一构造、读回复核 |
| §4.2 ProductReadiness | Task 1／12 | 九维、三元组、无聚合、证据纪律 |
| §4.3 EngineeringBaseline | Task 3／4 | 十字段、生成端口、establish_baseline、baseline 不可用 |
| §4.4 Milestone | Task 7／11 | 七字段七状态、批次、依赖、落库、游标、评估门 |
| §4.5 Progress Gate | Task 8／12／13 | 三值、策略、记录、重算、Decision |
| §4.6 ShippablePredicate／Verdict | Task 2／5 | 构造负例、八分支、组合负例、无正臂 |
| §4.7 ContinuousProductDrive | Task 6／16 | J17a 无条件；J17b 仅 C2=(b) |
| §4.8 lower／handoff | Task 14／15 | 确定构造、唯一写路径、只 READY、retire |
| §4.9 错误与持久化 | Task 1／9–16 | 一个 `ProductError`、迁移 200、读 API、事务纪律 |
| §5.1–§5.2 J1–J19 | 见下「J 照片归位」 | 每条至少一条用例；条件项已标 |
| §5.3 §7.4 判据 1／2／4 | Task 7／13／6／16 | 判据 1：J8／J9；判据 2：J14（只出 Decision）；判据 4：J15／J17 |
| §5.4 变异预告 | Task 17 Step 4 | 从 20 行抽 #1／#2／#3／#5／#7／#11 六行（30%）；未抽查 14 行明确不覆盖，完整表留在设计 |
| §5.5 拍不出照片 | 遗留第 2／3／7／8 条＋Global Constraints 的 A15／OPEN-015 | 正向 Shippable、replan 执行、READY 审查、证据内容、规范级阈值／批次边界 |
| §6.1 与 P7D | Task 15＋跨计划前置五 | 按值交接、只 READY、drive gate |
| §6.2 与 P5a | Task 12／5＋遗留第 2 条 | 只核 EvidenceId、`""→None` 映射 |
| §6.3 与 P4 | Task 14／15／13 | 只经 `load_intent`／`save_intent`／`apply_intent_transition`／`create_decision` |
| §6.4 与装配方 | Task 3／7／9 | 两个生成端口由 runtime 实现；P7A 不依赖 runtime |
| §6.5 本轮能拍到哪一步 | Task 5／13／15／17 | 类型层／源码面／运行期；端到端三处拍不到 |
| §7 明确不做 | Global Constraints | 不建、不留桩 |
| §8 十七条遗留 | 文末「遗留/待协调」第一张表 | **17 条逐条登记**（#1–#17，各带收件人／本计划落点）；其中 9 条另有正文展开；不关闭任何规范缺口 |

### 二、J 照片归位

| 判据 | 主 task | 具体用例／命令 |
|---|---|---|
| J1 | 4／17 | `the_product_fields_are_verbatim_in_order`；`Product { .. }` trybuild；`struct ProductId` 零命中 |
| J2 | 4／10 | 正例＋四类负例＋`ProductExists`；`save_product_reloads_the_anchor_and_requires_ready`；`save_product_rejects_a_missing_anchor_intent`（区分 UnknownIntent）；`save_product_rejects_an_anchor_whose_contract_became_invalid`（contract 写路径复验） |
| J3 | 1 | 九行源码面；`iter`／`dimension` 九枚；`readiness_has_no_score`／`average`／`sum`／`aggregate`；真实策略读值在 Task 8 |
| J4 | 1／10／12 | `assessed` 五类负例：空 value／空 evidence／重复 evidence（Task 1）、Assessed 空 `last_verified`（Task 10 库行读回）、evidence id 不在传入集合（Task 12）；另加 crate 内 `ReadinessDimension::for_test` 的 `record_milestone_progress_rejects_assessed_without_evidence`（defense-in-depth）；`FROM evidence` 零命中 |
| J5 | 3／4／10／17 | 十字段源码面；假生成器正／负例；`a_generator_error_leaves_zero_product_rows`；`Product` 字面量／`Default`／`Deserialize` 三份 trybuild；`the_product_has_a_single_private_construction_point` 的允许清单含 `src/product.rs` 的 `#[cfg(test)] for_test_with_unusable_baseline`，以及 `src/persist.rs`／`src/lower.rs` 测试模块的两处 `from_verified_parts` 调用 |
| J6 | 5／10 | `an_unusable_baseline_is_undetermined`；`load_product_rejects_an_empty_baseline_row` |
| J7 | 7／11 | 七字段源码面；七状态往返；表外值／空 objective／空条件 `CorruptRow` |
| J8 | 7／11 | 批次边界、空批次、两批 ordinal、孤儿 Product、缺 progress、空条件；零行断言 |
| J9 | 7／11 | 菱形 DAG、自环／越界、依赖往返 |
| J10 | 14 | 两次 lower 对照、跨 Product、空 exit `""`、空 objective（crate 内 `Milestone::for_test`）、无效 contract、`save_lowered_intent` 幂等／碰撞、trybuild |
| J11 | 15 | 十三状态、两个 READY、RUNNING 未过 REVIEW、retire、唯一 `apply_intent_transition` |
| J12 | 8 | 无策略、阈值 0、全未评、真实策略三 delta、无策略不 Triggered |
| J13 | 8／12 | `a_real_policy_progress_sequence_counts_to_the_threshold`（三条记录均由 `DeltaByDimension` 经 `record_milestone_progress` 写入，断言库里的 delta）；`an_improved_record_in_a_real_sequence_resets_the_run`；纯判定侧的 Regressed／Undetermined 历史；重复／未完成负例 |
| J14 | 13 | 重算 Triggered／NotTriggered／Undetermined、行数／事件数、重复 history（去重后不含 record）、伪造 verdict trybuild、无 Planner |
| J15 | 5 | 八分支＋三条组合负例；两臂都不产生 `Shippable` |
| J16 | 2／5 | Task 2 六类构造负例（含空 clause id 经 `ShippableClause::new`）；Task 5 的缺／多／重复三条纯结构 `AssessmentMismatch`，多一条那条无 NotSatisfied／Blocked |
| J17a | 6 | 只前进、不回退（含 Shippable＋Shippable）、不跳 Post |
| J17b | 16（仅 C2=(b)） | 默认 false、开关拒绝、Post 交接 gate、唯一 phase 写点；C2=(a) 延后 |
| J18 | 9／17 | 版本 `[200]`／名称、四表或三表、启动 15／13／15、190 未改 |
| J19 | 1／9／17 | 四条边、`ALLOWED` 精确、无 runtime／provider／registry／node／context、无第二份类型；`append_event` 零命中 |

### 三、Step scan

- 每个 task 的 Step 1 只写本 task 的测试；Step 2 只跑本 task 的 `--test` 目标；Step 3 只写
  本 task 的生产代码；Step 4 跑本 task＋受影响的上游；Step 5 提交。
- **Step 2 的失败形态**：Task 1 是 `package 未找到`；Tasks 2–8 是 `E0432`；Tasks 9–16 也是
  `E0432`（新函数尚未定义），**按纪律都不算红**。真正的红由 Step 3 之后的断言取。
- Task 16 的 Step 2 在 C2=(a) 时**不执行**（整 task 取消）。
- Task 17 的 Step 1／2 是 test-only／复核，首次可能即绿；它的红由 Step 4 的变异抽查取。
- 没有一步同时改两个关注点：Task 1 只做 readiness；Task 2 只做 predicate；Task 3 只做
  baseline 类型；Task 4 只做 Product 创建；Task 5 只做 verdict；Task 6 只做相位机；
  Task 7 只做生成；Task 8 只做纯判定；Task 9 只做迁移；Task 10–16 各管一个落库面。

### 四、Type consistency

- `Product.id: ProductIdRef`、`intent_id: IntentId`、`baseline: EngineeringBaseline`、
  `readiness: ProductReadiness`、`current_milestone: Option<MilestoneId>`、
  `milestone_history: Vec<MilestoneId>`、`shippable_predicate: ShippablePredicate`；
  `pub(crate) Product::from_verified_parts(..) -> Result<Product, ProductError>` 是 crate 内唯一
  **生产**构造点（Task 12／14 的 crate 内测试也调用它，属 J5 逐条登记的 `#[cfg(test)]`
  例外；`#[cfg(test)]` 的 `for_test_with_unusable_baseline` 另列在下方）。
- `ReadinessDimension::assessed(Opaque, Vec<EvidenceId>, i64) -> Result<Self, ProductError>`；
  `ReadinessValue::as_assessed() -> Option<&Opaque>`；`ProductReadiness::iter() ->
  [(ReadinessDimensionName, &ReadinessDimension); 9]`。
- `EngineeringBaselineGenerator::generate(&self, &BaselineRequest) ->
  Result<EngineeringBaseline, PortError>`；`MilestoneGenerator::propose(&self,
  &MilestoneProposalInput<'_>) -> Result<MilestoneBatchDraft, PortError>`。
- `pub(crate) Milestone::from_verified_parts(id, product_id, objective, dependencies,
  entry_conditions, exit_conditions, state) -> Result<Milestone, ProductError>`——只给
  `propose_next_batch`／`load_milestone(s)`；crate 外写不出 `Milestone { .. }`。
- `establish_baseline(&Intent, &BaselineRequest, ShippablePredicate,
  &dyn EngineeringBaselineGenerator) -> Result<Product, ProductError>`（async）。
- `propose_next_batch(&Product, &[Milestone], BatchBoundary, &dyn MilestoneGenerator) ->
  Result<Vec<Milestone>, ProductError>`（async）。
- `evaluate_progress(&ProductReadiness, &ProductReadiness, &[ProgressRecord],
  Option<&dyn ProgressPolicy>) -> ProgressGateVerdict`。
- `record_milestone_progress(&Tx<'_>, &Product, &Milestone, ProductReadiness, &[Evidence],
  Option<&dyn ProgressPolicy>, i64) -> Result<(ProgressRecord, ProgressGateVerdict), ProductError>`。
- `emit_replan_decision(&Tx<'_>, &Product, &ProgressRecord, &[ProgressRecord],
  Option<&dyn ProgressPolicy>, ReplanDraft, i64) -> Result<Option<Decision>, ProductError>`。
- `lower_milestone(&Product, &Milestone, ContractRef, Priority, i64, i64) -> Result<Intent,
  ProductError>`；`save_lowered_intent(&Tx<'_>, &Product, &Milestone, ContractRef, Priority,
  i64, i64) -> Result<Intent, ProductError>`。
- `product_intent_for_handoff(&Tx<'_>, &Product, &MilestoneId) ->
  Result<Option<Intent>, ProductError>`。
- `advance_product_phase(ProductPhase, &ShippableVerdict) -> ProductPhase`。
- 测试构造（全部 `#[cfg(test)] pub(crate)`、不改变生产可见性）：
  `Product::for_test_with_unusable_baseline`（Task 5，定义在 `src/product.rs`）、
  `Milestone::for_test`（Task 11，`src/milestone.rs`）、
  `ReadinessDimension::for_test`（Task 12，`src/readiness.rs`）。它们只服务 J6／J8／J4／J10
  的防御面，生产构造点仍各只有一条。
- 所有落库 API 的 `Tx<'_>` 参数、`i64` 时间戳、`Result<_, ProductError>` 在 Task 9–16 中一致；
  **没有第二个错误类型**。

### 五、Review Focus 归位

| Review Focus | 归属 task | 用例 |
|---|---|---|
| 1 completion `Blocked` 被 mismatch 盖成 `Undetermined` | Task 5 | `completion_blocked_wins_over_a_mismatched_assessment` |
| 2 history 重复计数误触发 | Task 13 | `a_record_repeated_in_history_is_counted_once` |
| 3 跨 Product lower | Task 14 | `a_milestone_from_another_product_is_rejected` |
| 4 空 baseline 默认通过 | Task 5＋Task 10 | `an_unusable_baseline_is_undetermined`；`load_product_rejects_an_empty_baseline_row` |
| 5 `READY` 绕过 lower | Task 14＋Task 15 | save_lowered_intent trybuild；`a_running_intent_that_never_passed_review_is_not_handed_off` |

### 六、比例

- 设计 1974 行；本计划 **2316 行**；任务 **17 个**。头部／跨计划前置／文件结构／自审／
  遗留约占 40%，生产任务占 60%。测试量大于生产量是刻意的：本块判据几乎全是
  「形状＋边界＋否定面」。
- 设计 §8 的 17 条**已逐条登记**在「遗留/待协调」第一张表（含 #6／#8／#9／#11／#13／
  #14／#15 的具名收件人）；它们不是本块的实现 task，计划只登记、不关闭。
- 条件项只有一处：J17b／Task 16（C2=(b)）。**没有把条件项写成已闭合。**

---

## 遗留/待协调

### 〇、设计 §8 十七条逐条登记

设计 §8 共 **17 条**（第 1–16 条规范／跨块遗留，第 17 条是设计自查的阻断项汇总）。下表
逐条给出本计划的落点与具名收件人；**本计划不关闭任何一条**，也不把其中的规范缺口就地补成
实现。

| 设计 §8 # | 缺口 | 本计划落点／申报 | 收件人 | 状态 |
|---|---|---|---|---|
| 1 | A15／OPEN-010：九维聚合与 `progress_delta` 阈值未定义 | `ProgressPolicy` 由调用方申报；Task 8 `NoPolicy`／`InvalidThreshold`；**不设默认阈值** | 规范维护者＋P10 | 开 |
| 2 | OPEN-015：有限视野与并行／汇合批次边界未定义 | Task 7 `BatchBoundary` 显式参数、超界拒绝；测试只钉「无完整未来」与「评估后再生成」 | 规范维护者＋P10 | 开 |
| 3 | `CompletionVerdict` 无正臂，`Shippable` 不可达 | Task 5 `evaluate_shippable` 无 `Shippable` 返回行；Task 6 只接受合成值；P5a 加正臂时穷尽 `match` 会红（tripwire） | P5a＋P9（ENG-004） | 开 |
| 4 | `Evidence` 只写不读，且无 Evidence→九维映射 | Task 12 只核 `EvidenceId` 存在性、只存 id；不读 `evidence` 表 | P5a＋规范维护者 | 开 |
| 5 | replan 消费方无实现，`Decision` 选项无生产者 | Task 13 只出 `Decision`、只写 `decision.required`；不建 Planner、不执行 replan | 第 3 层 Planner 持有方＋协调者（P8 范围） | 开 |
| 6 | §152 的 Milestone 图边不能完整 lower 到 §222 的 `Intent`（单亲 `parent_intent`） | Task 7 把图边留在 `milestone.dependencies`；Task 14 每次 lower 只产出节点值、都挂锚 Intent；**不伪造 `dependencies[]`** | P4＋规范维护者 | 开 |
| 7 | `current_milestone` 单数、`milestone_history[]` 元素类型未给 | Task 11 `current_milestone` 作游标、`load_milestones` 按 `seq` 派生 history（只存 id）；并行时只有一个 current | 规范维护者＋协调者 | 开 |
| 8 | `Product.baseline` 非可空 vs「baseline 缺失或不可判」 | Task 4 baseline 非可空；「没有 Product」＝`evaluate_shippable(None,..)` ⇒ `Undetermined(MissingProduct)`；「有行但 baseline 空」⇒ Task 10 `load_product` 的 `CorruptRow`；**不引入第三种 Product 状态** | 规范维护者 | 开 |
| 9 | `ShippablePredicate` 字段与更新规则／版本化未给 | Task 2 取 `required_dimensions[]`＋`clauses[]`、空谓词拒绝；Task 4／10 创建后不可变、读回重走构造校验；**不做增量更新** | 规范维护者＋P4 | 开 |
| 10 | lower 只到 `PROPOSED`，`PROPOSED → READY` 审查链未接线 | Task 14 lower 固定 `Proposed`；Task 15 handoff 只认 `Ready`；**P7A 不推进** | P4＋协调者 | 开 |
| 11 | Milestone 七状态没有迁移表 | Task 11 只存 `state`，`record_milestone_state` 做乐观并发（`expected` 匹配即可写），**不判合法对**；不建状态机 | 规范维护者＋P1／协调者 | 开 |
| 12 | `ContinuousProductDrive` 落点是切分偏离，C2 未裁定 | Task 9–16 全部条件于 C2；C2=(a) ⇒ Task 16 取消、J17b 延后、Task 15 改用 C2 面；C2=(b) ⇒ 四表与 Task 16 | 协调者（裁定并改切分 §2.3.1）＋规范维护者 | 开 |
| 13 | `ProgressDelta::Regressed` 不直接触发 replan | Task 8 中 `Regressed` ⇒ `NotTriggered{0,..}` 并打断 run；Task 12 记录其 delta 但不出 Decision | 规范维护者＋P10 | 开 |
| 14 | `ShippableVerdict` 公开可伪造 | Task 6 相位机接受合成 `Shippable`（J17a 的照片依赖它）；Task 5 的 `evaluate_shippable` 今天不生产该值；**不把构造点收私有**（否则正向相位机在 P5a 前不可测） | P5a＋P9＋协调者 | 开 |
| 15 | baseline／predicate 的更新与版本化未定义 | Task 4／10 本轮创建后不可变（无 `set_baseline`／`set_shippable_predicate`）；产品合同变化只能新建 Product 或手工迁移 | 规范维护者＋P4 | 开 |
| 16 | P7A 新 crate 会让 P6D 合并门变红 | Task 17 Step 3 列为**唯一已知红**；P7A 不碰 `p6_pre_allowed.txt`／`P6_EXPECTED_ROWS`；由 P7D 重新基线化或退役 | P7D＋协调者 | 开 |
| 17 | 设计自查的实现级阻断项（I1 ordinal／I2 跨 Product lower／I3 handoff 按 id／I5 孤儿 Product／I6 读回复核／I7 空 exit→`None`／C3 不收 `Intent`） | 逐条有照片：I1 Task 7／11；I2 Task 14；I3 Task 15；I5 Task 11；I6 Task 10；I7 Task 14；C3 Task 14 trybuild。**任何一条失去照片，计划阶段不得开工** | P7A 实现者＋协调者（计划评审） | 本计划已落照片 |

### 一、设计缺项的展开（对应上表 #1–#16 中的 9 条）（**发现即报，本计划不就地补**）

1. **C2 裁定（`ContinuousProductDrive` 的落点）尚未作出**（设计 §4.7／§8 第 12 条；
   切分 §2.3.1 `:132-133`）。计划把 Task 9 的迁移 200 与 Tasks 10–16 的落库全部写成
   **C2 裁定后的提交**；Tasks 1–8 可在 C2 前合入。**C2=(a) ⇒ Task 16 取消、J17b 延后、
   Task 15 的 drive gate 改用 C2 既有面；C2=(b) ⇒ 四表与 Task 16 按本计划执行。**
   **收件人：协调者（裁定并更新切分 §2.3.1）＋规范维护者。**
2. **P5a 的 `""→None` 映射**（设计 §4.8、§6.2；`completion.rs:70-76`／`:213-214`）：
   `lower_milestone` 对空 `exit_conditions` 产出 `Opaque::from_str("")`；P5a 装配时必须把
   它映射为 `declared_completion_predicate = None`。P7A 侧只钉「空 exit ⇒ `""`」（Task 14），
   P5a 侧必须钉「`""` ⇒ `None`」。**收件人：P5a。**
3. **replan 的消费方无实现**（设计 §4.5、§8 第 5 条；切分 §6.2 第 13 条 `:412`）：
   P7A 只出 `Decision` 并落 `decision.required`，不执行 replan；`ReplanDraft` 的
   issue／options／blocking 由调用方给。**收件人：第 3 层 Planner 持有方＋协调者
   （P8 范围是否新增「消费 Decision 并 replan」）。**
4. **设计 §2.1 的共写文件表漏了 `crates/continuum-runtime/Cargo.toml` 与 runtime 的
   `ALLOWED` 行**：没有 `continuum-product` 依赖，`main.rs` 无法调用
   `p7a_product_migrations()`；没有 runtime 行的 `continuum-product`，依赖方向用例会红。
   本计划在 Task 9 补这两处（只加一行），**请设计作者／协调者在切分 §一.3 的共写表中登记**。
   **收件人：设计作者＋协调者。**
5. **P6D 合并门会因新 crate 变红**（设计 §2.1、§8 第 16 条；切分 §四第 1 条）：
   `p6_union_adds_only_its_own_rows` 的断言 5／8 会先红；P7A **不碰**
   `p6_pre_allowed.txt`／`P6_EXPECTED_ROWS`，由 P7D 重新基线化或退役。
   本计划的 Task 17 Step 3 把这条列为**唯一已知红**。**收件人：P7D＋协调者。**
6. **`ReadinessDimension::assessed` 的错误维度名**（本计划自定）：设计 §4.2 的签名不含
   维度名，而 `ProductError` 的四个 readiness 臂带 `dimension: &'static str`。本计划在
   `assessed` 里填 `"<unbound>"`，绑定维度后由 `record_milestone_progress`／`load_product`
   用真实名字重验。**若设计作者认为应改签名，改的是设计 §4.2 与 Task 1，不是本计划
   单方面改。收件人：设计作者＋协调者（仅当要改签名时）。**
7. **`PROPOSED → READY` 的审查链未接线**（设计 §4.8、§8 第 10 条）：P7A 不推进；
   `establish_baseline` 要求锚 Intent 是 `READY`；在审查链接线前，P7A 拿不到可建 Product
   的锚 Intent。**收件人：P4＋协调者。**
8. **`CompletionVerdict` 无正臂、`Evidence` 只写不读**（设计 §8 第 3、4 条）：
   `Shippable` 今天没有生产路径；`record_milestone_progress` 只核 `EvidenceId` 存在性。
   **收件人：P5a＋P9（ENG-004）＋规范维护者。**
9. **设计 §4.4 的 `seq = MAX(seq)+1` 与 J8 的「两批 seq 0..3」有张力**（设计 §4.4 `:913-916`、
   §5.2 J8 `:1704`）：若首批用裸 `MAX(seq)+1`，首批 `seq` 从 1 起，`ordinal = history.len()+i`
   却是 0 起，Task 11 的 `MilestoneOrdinalMismatch` 会把一批正常生成全拒。本计划取
   `COALESCE(MAX(seq), -1)+1`（首批 0），与 `ordinal` 和 J8 对齐；**请设计作者确认**，
   若设计要 1 起，则 `ordinal` 与 J8 的 `0..3` 同批改。**收件人：设计作者＋协调者。**

### 二、本计划自定的形状（**申报**）

- `tests/support/mod.rs` 的假生成器、`DeltaByDimension`、`seed_readiness`、Intent／Evidence
  夹具：设计只要求「假实现由测试自己写」，没给名字／形状。本计划自定；`seed_readiness` 只
  写内部 readiness JSON 作为测试起点，不伪造 delta；`#![allow(dead_code)]` 避免跨 target
  warning。
- 三处 `#[cfg(test)]` 构造（`Product::for_test_with_unusable_baseline`／
  `Milestone::for_test`／`ReadinessDimension::for_test`）是本计划为防御面拍的测试构造，
  不是公开 API，也不在 Task 17 的生产构造点计数里；J5 的允许清单把第一处列为逐条登记的
  例外，后两处不涉及 `Product` 构造点。另有 Task 12／14 的 crate 内测试各调用一次
  `Product::from_verified_parts`，同样按 `#[cfg(test)]` 例外逐条登记。
- 内部 JSON 编码（baseline／readiness／predicate／dependencies／conditions）是**内部实现**，
  不是公开序列化契约；本计划在 Task 10 固定键名，测试按此断言。设计 §4.9 允许内部编码。
- `ContinuousProductDrive`／`ProductDriveState` 的 `Default` 是「默认 false／Pre」的落点；
  `Product` 明确**不实现** `Default`。若复审认为 drive 也不该有 `Default`，改的是 Task 6
  与 Task 16 的构造，不影响 C2 的形状。
- Task 17 的 `source_surface.rs` 用 `include_str!` 而不是外部 `grep` 脚本：语料与作用域写死
  在测试里，随 `cargo test` 一起跑；代价是注释里不能出现被禁字面量。
- `cargo test --workspace` 的「唯一已知红」是 P6D 合并门，不是 P7A 的缺陷；本计划不把
  它写成绿，也不在 P7A 侧修。
