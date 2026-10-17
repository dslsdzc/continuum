# P6C（记忆与学习）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**任务单：** `docs/superpowers/specs/2026-10-17-p6c-memory-and-learning-design.md`（**唯一事实来源**，1541 行，2026-10-17 定稿）。切分：`docs/superpowers/specs/2026-10-08-p6-scope-and-split.md`（§一 1.3 共写文件、§二 接口冻结、§四 六条横切约束、§五 P6C 行、§八 八.2 第 4 条）。规范：`docs/02-工程.md` §6.1／§6.2／§6.3／§6.4 判据 3／§9.2；`docs/spec/05-normative.md` §282／§283（七字段）／§284（五 scope）／§285／§286；`docs/spec/03-product-drive.md` §169–§179。格式先例：`docs/superpowers/plans/2026-10-10-p5a-verification-and-evidence.md`。

**Goal:** 新建 crate `continuum-memory`，落《工程》§6.1 的前四行：Memory Source 图（§282 §170）、`MemoryClaim` 存储（§283 §284 §179）、Memory Synthesis 作业记录与事务性 applier（§285 §171 §172）、Memory Correction（§286 §177），外加「按需取 claim」的确定性检索面（§6.3 的 `Context Compiler ← MemoryClaim`）与迁移 **190** 的八表六索引。交付物是**类型、存储、状态迁移与检索**，不是 proposer、调度器或 UI。

**Architecture:** 本块只新建 **`crates/continuum-memory`**，workspace 内**只依赖 `continuum-persist`**（`Tx`／`Migration`／`PersistError`／`Value`）。模块方向按设计 §3.4：`source.rs` 零 claim 引用；`claim.rs` 用 source；`correction.rs`／`synthesis.rs` 用 source＋claim；`retrieval.rs` 只读 claim。迁移 190 一次建八表六索引；此后 SQL 冻结。对 P6A 冻结的一页接口是 `MemoryRetrieval::retrieve(&RetrievalQuery)`、`MemoryStore`、`retrieve_claims`、`load_source(s)`／`load_summary`／`load_current_summary`、`MemoryClaim` 七访问器与 `MemoryClaimId::as_str`。六个公开写函数分属四个操作面：`insert_source`（Source）、`correct`（Correction）、`remove_source`（Removal）、`begin_synthesis`／`apply_synthesis`／`fail_synthesis`（Synthesis）。**零事件、零审计、无 proposer、无调度器、无物理擦除。**

**Tech Stack:** Rust 1.95.0／edition 2024；`continuum-persist`；`serde_json`（`Confidence` 与作业 report）；`thiserror`（`MemoryError`）；dev：`tempfile`、`trybuild`。**不依赖** `continuum-events`／`continuum-artifact`／`continuum-core`／`continuum-semantics`／`continuum-graph`／`continuum-verify`／`continuum-policy`／`continuum-capability`（设计 §3.2 逐条给了零使用的理由）。

**Spec:** `docs/superpowers/specs/2026-10-17-p6c-memory-and-learning-design.md`（执行者必须与计划并读；凡本计划写「设计 §x」均指该文件）。

---

## Global Constraints

- **工具链固定** rustc 1.95.0／cargo 1.95.0，edition 2024；`cargo test --workspace --no-fail-fast` 全绿且 **0 warning**，`cargo build --workspace --all-targets` 同样 0 warning。
- **`MemoryClaim` 七字段逐字、不多不少**（§283）：`id` `scope` `statement` `sources[]` `validity` `confidence` `last_confirmed`。没有第八个字段，没有 `status`／`created_at`／`scope_ref`／`anchor`。`sources[]` 的元素是 `MemorySourceId`，非空、去重、按 id 升序规范化。
- **`test-support` 只开测试构造面**：`crates/continuum-memory/Cargo.toml` 的 `[features] test-support = []` 是空 feature；只有它启用时才存在 `#[doc(hidden)]` 的 `MemoryClaim::for_test`。默认构建下 `new` 仍 `pub(crate)`，C2 不变；P6A 在 `[dev-dependencies]` 打开它。
- **五个 scope 逐字**（§284）：`PERSONAL`／`PROJECT`／`PRODUCT`／`TASK`／`RUNTIME`；Rust 变体 `Personal`／`Project`／`Product`／`Task`／`Runtime`；落库编码小写；`ALL: [MemoryScope; 5]`；**没有 `Session`**。
- **全块只有一种锚点形状**：`scope: MemoryScope` ＋ `anchor: Option<String>`。`Project`／`Product`／`Task` 必填（缺 ⇒ `AnchorRequired`）；`Personal`／`Runtime` 必须为 `None`（给 ⇒ `AnchorNotAllowed`）；`anchor` 为 `Some` 时 `scopes` 必须恰好是一个 anchored scope。校验点只有 `validate_scope_anchor` 一个函数。
- **检索的两个边界是语义，不是错误**：`limit = 0` ⇒ `Ok(vec![])`；`text = Some("")` 归一为 `None`（不做文本过滤）。`scopes` 为空 ⇒ `Err(EmptyScopeSet)`，空集合不是「查全部」。
- **删除 Source 是软删除**：`memory_source.removed_at` 置墓碑；**所有**引用它的 `Active` claim 无条件改 `Invalidated`（已是终态的不动）；受影响 `(scope, anchor)` 的摘要置 `stale = 1`。**没有 DELETE 语句，没有物理擦除。**
- **零事件、零审计**：§310／§313 是 MUST 最小清单，不是封闭集合；本块按最小读法**不新增**第十类事件／第九类审计，crate 内不出现 `EventType`／`AuditKind`／`append_event`／`append_audit`。
- **迁移档 `190`–`199`，本块取 `190`**（函数 `p6c_memory_migrations()`，版本恰 `[190]`，名称 `p6c_memory`）。落地时先重跑 `grep -rn "Migration::new(" crates/` 核对空号；`191`–`199` 本块不占。**Task 12 之后 190 的 SQL 不得再改**（`Db::migrate()` 按 version 跳过已应用者）。
- **写函数一律不 `commit`**：`insert_source`／`remove_source`／`correct`／`begin_synthesis`／`apply_synthesis`／`fail_synthesis` 只经调用方的 `Tx` 写；返回 `Err` 之后调用方必须丢弃句柄回滚（`Tx` 的 `Drop` 会 `ROLLBACK`；没有保存点）。
- **零逻辑臂之外不造臂**：`MemoryError` 的每一枚臂都在设计 §10.1 的拒绝点表里有具名拒绝点与照片；`Persist(PersistError)` 只做数据库错误透传，不拍逻辑照片。
- **外部依赖按需加、不进 `ALLOWED`**：`ALLOWED` 只逐对断言 workspace 成员之间的边；`serde_json`／`thiserror`／`tempfile`／`trybuild` 不登记。
- 代码注释、错误信息、测试断言信息用中文；标识符用英文。**不要 `git add -A`，不要 `git commit --amend`**；只 `git add <显式路径>`；新增依赖时 `Cargo.lock` 一并提交。
- 测试统一 `TMPDIR="$PWD/.tmp" timeout <秒>`；**管道结尾不要接 `tail`**（退出码会被吃掉）。收工前 `chmod -R u+rwX .tmp && rm -rf .tmp`。
- **不改设计、不改规范、不改本计划之外的任何计划**；设计缺项写进本文件 `## 遗留与待协调`。

---

## Review Focus

下面五条是设计暗示、但最容易在实现里「看起来对了、实际错了」的输入／失效模式。每条都在拥有该代码的 task 里有对应用例；**这五条不是附加功能，是正常工作的下限**。

1. **纠正后的说法丢失。** 用户说「这个已经不是这样了」，新说法必须存下来供下一次合成读；只把旧 claim 标 `Superseded`、或把新说法写进旧 claim 的 `statement`，都会让用户纠正完却无话可说。拥有者：Task 10 的 `a_correction_writes_a_new_source_and_supersedes_the_old_claim`（R5）与 `the_correction_statement_can_be_proposed_in_the_next_synthesis`（R26）。
2. **旧摘要未失效。** claim 新建／合并／退场、纠正、删来源之后，旧摘要仍是 `stale = 0`，`load_current_summary` 会把它当新事实送进上下文。拥有者：Task 8 的 `no_summary_stales_the_old_one_and_current_summary_is_none`（R19b）、Task 9 的 `remove_stales_the_summaries_of_the_affected_pairs`（R9 的另一半）、Task 10 的 `a_correction_stales_only_its_scope_summary`（R9）。
3. **`anchor` 缺失时跨项目混检。** 拿 `[Project]` 但不给 anchor，或给 `[Personal, Project]` 加 anchor，必须拒绝；放行会让 A 项目的 claim 出现在 B 项目的上下文里。拥有者：Task 7 的 `query_construction_applies_the_anchor_rules`（R27）与 `scope_and_anchor_filter`（R20）、Task 5 的 `begin_applies_scope_anchor_rules`（R32）。
4. **软删除后派生 claim 未失效，或误伤无关 claim。** 删 source A 必须让引用 A 的每一条 Active claim 失效，且**不**碰不引用 A 的 claim；`load_sources` 仍要能看见墓碑。拥有者：Task 9 的 `removing_a_source_invalidates_every_claim_that_references_it_and_no_other`（R3）与 `a_removed_source_stays_traceable_but_hides_its_claims`（R4）。
5. **两阶段作业的重复 id／状态错乱。** 同一 job 被 apply 两次、job id 重复、claim id 与库里或本批重复、apply 失败后 job 被误标 `Applied`——任何一条都会让 report 与表对不上。拥有者：Task 5 的 `begin_rejects_a_duplicate_job_id`（R38）、Task 8 的 `a_second_apply_of_the_same_job_is_rejected`（R16）、`duplicate_claim_ids_are_rejected`（R36）与 `a_failed_apply_leaves_the_job_running`。

---

## 跨计划前置

### 一、依赖的交付物与**实际状态**（2026-10-17 在工作树 `p6-plan` 实读）

| 依赖 | 实际状态 | 本块用在哪 |
|---|---|---|
| `continuum_persist::Db` / `open_with` / `migrate`（`crates/continuum-persist/src/db.rs:72`、`:80`、`:91`、`pub fn begin` `:147`） | **已交付** | 全部测试库与写路径 |
| `continuum_persist::Migration::new(version: i64, name: &'static str, sql: &'static str)`（`src/db.rs:14`、`:19`） | **已交付** | Task 2 的迁移 190 |
| `continuum_persist::Tx::{query, execute, commit}`（`src/tx.rs:26`、`:30`、`:39`）；`Tx` 的 `Drop` 在未提交时 `ROLLBACK`（`src/tx.rs:235-241`） | **已交付** | 全部读写；失败路径照片以「丢弃句柄」为前提 |
| `continuum_persist::PersistError`（`src/error.rs`）与 `Value`（`src/value.rs:8`） | **已交付** | `MemoryError::Persist`、参数与读回 |
| `continuum-runtime` 的 `runtime_migrations()`（`crates/continuum-runtime/src/main.rs:110`；现有到 `p5a_verify_migrations()` 为止） | **已交付** | Task 12 注册 190 |
| `dependency_direction.rs` 的 `ALLOWED`（`:25`）、`continuum-runtime` 行（`:391-412`）、门用例 `every_crate_depends_only_on_its_allowed_set`（`:478`） | **已交付** | Task 1／Task 12 |
| `crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`:35`）与 `startup.rs` 的计数断言（`:25`、`:80`、`:92`） | **已交付**，且**会因 190 变红** | Task 12 的「断言同步」，见下节与遗留 |
| `continuum-memory` | **不存在**（`ls crates/` 无该目录；本计划创建） | 全部 task |

### 二、共写五处（设计 §3.3 的五行）

| # | 文件 | 本块的改动 | 落在 |
|---|---|---|---|
| 1 | `Cargo.toml` 的 `[workspace] members`（`:3-31`） | 加一行 `crates/continuum-memory` | Task 1 |
| 2 | `crates/continuum-runtime/Cargo.toml` | 加 `continuum-memory = { path = "../continuum-memory" }` | Task 12 |
| 3 | `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-memory", &["continuum-persist"])` | Task 1 |
| 4 | 同一文件 `continuum-runtime` 行（`:391-412`） | 数组里加 `"continuum-memory"` | Task 12 |
| 5 | `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:110`） | 加 `migrations.extend(continuum_memory::p6c_memory_migrations()); // 190：记忆八表` | Task 12 |

**另外两处是「断言同步」，不是新的源码共写面**：`crates/continuum-runtime/tests/migrations.rs:35` 的 `expected_migrations()` 必须加 `p6c_memory_migrations()`（否则同文件 `:121` 起的集合比对红），`startup.rs:25`／`:80`／`:92` 的三处计数必须按实跑从 13／11 改成 14／12（否则启动门红）。设计 §3.3 的共写表只列五行、且写「除这五处之外不碰任何既有 crate 的源码」；这两处是第五行的**必然断言后果**，不列进设计是设计的漏项——本计划照做并在 `## 遗留与待协调` 第 1 条报告，不改设计。

**除这七处之外**，本块不碰任何既有 crate 的源码、类型或注记。

### 三、P6A 依赖本块的冻结接口（不得改名、不得另造）

```rust
// 对象安全；P6A 的 `compile` 只收 `&dyn MemoryRetrieval`，无 `Tx`。
pub trait MemoryRetrieval {
    fn retrieve(&self, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError>;
}
pub struct MemoryStore<'a> { /* 持 &'a Tx<'a> */ }
impl<'a> MemoryStore<'a> { pub fn new(tx: &'a Tx<'a>) -> MemoryStore<'a>; }
impl MemoryRetrieval for MemoryStore<'_> { /* 调 retrieve_claims */ }

pub fn retrieve_claims(tx: &Tx<'_>, query: &RetrievalQuery)
    -> Result<Vec<MemoryClaim>, MemoryError>;
pub fn load_source(tx: &Tx<'_>, id: &MemorySourceId)
    -> Result<Option<MemorySource>, MemoryError>;
pub fn load_sources(tx: &Tx<'_>, ids: &[MemorySourceId])
    -> Result<Vec<MemorySource>, MemoryError>;
pub fn load_summary(tx: &Tx<'_>, scope: MemoryScope, anchor: Option<&str>)
    -> Result<Option<MemorySummary>, MemoryError>;
pub fn load_current_summary(tx: &Tx<'_>, scope: MemoryScope, anchor: Option<&str>)
    -> Result<Option<MemorySummary>, MemoryError>;

impl MemoryClaim {
    pub fn id(&self) -> &MemoryClaimId;
    pub fn scope(&self) -> MemoryScope;
    pub fn statement(&self) -> &str;
    pub fn sources(&self) -> &[MemorySourceId];
    pub fn validity(&self) -> Validity;
    pub fn confidence(&self) -> &Confidence;
    pub fn last_confirmed(&self) -> i64;
}
impl MemoryClaimId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
```

**测试面例外（不改生产接口）**：P6A 的 claim 夹具经 `#[cfg(feature = "test-support")] #[doc(hidden)] MemoryClaim::for_test(...)` 构造（七字段签名见 `## 遗留与待协调` 第 11 条）；默认构建下该方法不存在，`new` 仍是 `pub(crate)`，C2 不变。

**与 P6A 已对齐的三点（据实）**：P6A §5.2 用 `scopes: Vec<MemoryScope>`／`anchor: Option<String>`／`memory_query: Option<String>`；P6A §5.3 的唯一构造点是 `RetrievalQuery::new(scopes, anchor, text, limit, false)?`，`limit` 取 `budget.max_memories`；P6A §5.6 原样透传 `MemoryError`。P6A 在 `max_memories == 0` 或 `scopes` 为空时**不调**检索面；本块的 `limit = 0` 仍必须返回 `Ok(vec![])`（两边语义同向）。P6B 只经 `claim.id().as_str()` 取弱引用字符串；P6D 与本块零共享类型；P7 只用 `begin_synthesis`／`apply_synthesis`／`fail_synthesis` 三个函数。

### 四、本计划把什么交给谁

- **交给装配方**：`insert_source` 的调用（本块不自动产生 source；`Conversations` 今天没有存储面，见设计 §12.1 第 15 条）。
- **交给 P6A**：上述一页接口与 `MemoryClaim` 的 `Clone`／`Debug`／`PartialEq`（P6A 的 `ContextPackage` 按值持有）；另加 `test-support` 下的 `for_test` 测试构造面（遗留第 11 条）。
- **交给 P7**：作业三函数与触发面；调度、阈值、恢复不在本块。
- **交给规范维护者／协调者**：设计 §12.1 的十七条，本计划一条都不代裁；其中会改本计划代码的见 `## 遗留与待协调` 第 7 条。

---

## File Structure

```
crates/continuum-memory/
  Cargo.toml
  src/lib.rs            导出面与 crate 文档（逐 task 增量加）
  src/error.rs          MemoryError（逐 task 按拒绝点增量加）
  src/persist.rs        p6c_memory_migrations()：迁移 190 的八表六索引（只此一处）
  src/source.rs         SourceKind、MemorySourceId、MemorySource、insert_source、
                        load_source、load_sources（**零 claim 引用**，S3）
  src/claim.rs          MemoryScope、Validity、Confidence、MemoryClaimId、MemoryClaim、
                        validate_scope_anchor、load_claim（crate 内）、
                        for_test（`test-support` feature-gated，跨 crate 测试装配）、
                        remove_source（跨 source＋claim，放这里以保住 source.rs 的零 claim 引用）
  src/summary.rs        MemorySummaryId、MemorySummary、load_summary、load_current_summary、
                        write_summary（crate 内，只许 apply_synthesis 调）、
                        mark_summaries_stale（crate 内）
  src/correction.rs     Correction、CorrectionOutcome、CorrectionRecord、correct、load_correction
  src/synthesis.rs      SynthesisJobId、SynthesisStatus、SynthesisTrigger、SynthesisRequest、
                        ProposedClaim、SynthesisProposal、SynthesisReport、
                        begin_synthesis、apply_synthesis、fail_synthesis
  src/retrieval.rs      RetrievalQuery、MemoryRetrieval、MemoryStore、retrieve_claims
  tests/common/mod.rs   起真库与夹具（逐 task 追加；`#![allow(dead_code)]`）
  tests/source.rs               Task 1
  tests/migrations.rs           Task 2（含 §10.3 的 S5／S7／S9）
  tests/source_store.rs         Task 3
  tests/claim.rs                Task 4（公开编码与 Confidence；私有构造在 src/claim.rs 的单元测试；`test-support` 下的 for_test 用例）
  tests/summary.rs              Task 6
  tests/synthesis_job.rs        Task 5
  tests/retrieval.rs            Task 7
  tests/apply_synthesis.rs      Task 8
  tests/remove_source.rs        Task 9
  tests/correction.rs           Task 10
  tests/type_level.rs           Task 11（trybuild 驱动 + 清单对钉）
  tests/source_face.rs          Task 11（S3／S4／S6／S8／S11＋§13.3 词汇表 grep）
  tests/compile_fail/*.rs        Task 11 的七份不可表达性样例（各配同名 .stderr）
  tests/wiring.rs               Task 12（S1／S2／S10）
```

**要改的既有文件**（只有 Task 1、Task 12 碰它们）：

```
Cargo.toml（workspace）                                  members 加一行（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 新行（Task 1）＋ runtime 行（Task 12）
crates/continuum-runtime/Cargo.toml                      加 path 依赖（Task 12）
crates/continuum-runtime/src/main.rs                     runtime_migrations() 加一行（Task 12）
crates/continuum-runtime/tests/migrations.rs             expected_migrations() 加一行（Task 12，断言同步）
crates/continuum-runtime/tests/startup.rs                三处计数 13／11→14／12（Task 12，断言同步）
```

**不碰**：`crates/continuum-persist/**`、`crates/continuum-events/**`、`crates/continuum-artifact/**`、`crates/continuum-graph/**`、`crates/continuum-verify/**`、`crates/continuum-core/**`、`crates/continuum-semantics/**`、`crates/continuum-policy/**`、`crates/continuum-capability/**`、`docs/**`。

---

### Task 1: crate 骨架、`MemoryError` 的初始臂、`SourceKind` 与 `MemorySource`

**Files:**
- Create: `crates/continuum-memory/Cargo.toml`
- Create: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/src/error.rs`
- Create: `crates/continuum-memory/src/source.rs`
- Create: `crates/continuum-memory/tests/source.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加 `crates/continuum-memory`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加一行 `("continuum-memory", &["continuum-persist"])`；该行内数组只有这一项）

**Interfaces:**
- Consumes: `continuum_persist::PersistError`
- Produces: `MemoryError`（本 task 只有 `Persist(PersistError)`、`EmptyExternalRef`）；`SourceKind`（九臂、`ALL: [SourceKind; 9]`、`as_str`／`parse`）；`MemorySourceId::{new,as_str}`；`MemorySource::{new,id,kind,external_ref,observed_at,removed_at}`。`MemorySource::new(id: MemorySourceId, kind: SourceKind, external_ref: impl Into<String>, observed_at: i64) -> Result<MemorySource, MemoryError>`（`id` 是 newtype，不是 `impl Into<String>`；夹具写 `MemorySourceId::new("s1")`）；`external_ref.trim()` 为空 ⇒ `EmptyExternalRef`；**存原串，不 trim**。

**本 task 的拒绝点 → 臂 → 照片：** `MemorySource::new` 的 `external_ref` 去空白为空 → `EmptyExternalRef` → R23。

- [ ] **Step 1: 写失败用例**

`tests/source.rs`：

- `source_kind_has_the_nine_arms_of_170_and_282`：`SourceKind::ALL.len() == 9`；逐臂 `parse(as_str()) == Some(kind)`；九个编码逐字是 `conversations`／`task_history`／`product_history`／`project_artifacts`／`files`／`connected_services`／`explicit_user_corrections`／`user_policies`／`project`。**红条件**：漏一臂（移除档）⇒ 数目断言红；`Project` 与 `ProjectArtifacts` 混写（取反档）⇒ 编码断言红。
- `source_kind_parse_does_not_default`：`parse("conversation")`、`parse("")`、`parse("PROJECT")` 均 `None`。**红条件**：末支返回默认臂（放宽档）⇒ 红。
- `memory_source_constructor_rejects_a_blank_external_ref`（R23）：`""`、`"   "`、`"\t\n"` 各 ⇒ `Err(MemoryError::EmptyExternalRef)`；`"  sess-1  "` ⇒ `Ok`，且 `external_ref()` **逐字**是原串（含空白）。**红条件**：用 `trim()` 后存（取反档）⇒ 最后一条红。
- `memory_source_exposes_the_five_fields`：`id`／`kind`／`external_ref`／`observed_at`／`removed_at` 五个访问器各读一次；`removed_at() == None`。
- `source_id_round_trips_as_a_string`：`MemorySourceId::new("s1").as_str() == "s1"`；接受 `String` 与 `&str` 两种 `Into<String>`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test source
```

**预期形态是编译失败**（`src/` 与 crate manifest 尚未建）：`error[E0433]`／`failed to load manifest`。按纪律，编译失败不是「运行期变红」，照实记为「确认编译失败」。

- [ ] **Step 3: 实现**

`Cargo.toml`：`[package] name = "continuum-memory"`、`version.workspace = true`、`edition.workspace = true`、`rust-version.workspace = true`；`[dependencies]` 只有 `continuum-persist = { path = "../continuum-persist" }`、`serde_json = { workspace = true }`、`thiserror = { workspace = true }`；`[dev-dependencies]` 只有 `tempfile = { workspace = true }`（`trybuild` 到 Task 11 再加）。

`src/error.rs`：`#[derive(Debug, PartialEq, Eq, thiserror::Error)] pub enum MemoryError { #[error(transparent)] Persist(#[from] PersistError), #[error("external_ref 去空白后为空")] EmptyExternalRef }`。**后续 task 只按各自拒绝点追加臂**；不预置未使用的臂。

`src/source.rs`：`SourceKind` 九臂照设计 §4.1 的表格顺序，派生 `Debug, Clone, Copy, PartialEq, Eq`；`ALL`、`as_str`、`parse` 均为穷尽 `match`（`parse` 表外返回 `None`）。`MemorySourceId(String)` 派生 `Debug, Clone, PartialEq, Eq, Hash`。`MemorySource` 五个字段全私有，派生 `Debug, Clone, PartialEq, Eq`；**无 `Default`、无字面量构造**。`new` 只做 `external_ref.trim().is_empty()` 检查。

`src/lib.rs`：`mod error; mod source;` 与三个 `pub use`。crate 文档写明「第 6 层认知层；只依赖 P0；入度为零的模块方向见设计 §3.4」。

`Cargo.toml`（根）与 `dependency_direction.rs` 同步两处共写；`ALLOWED` 里该行**只允许** `continuum-persist`（此时 `Cargo.toml` 已声明它，`cargo tree` 会看到）。

- [ ] **Step 4: 运行，确认通过（含依赖门）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test source
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction
```

**预期**：`source` 5 条全 PASS；`dependency_direction` 两条用例 PASS（新 crate 已在 `ALLOWED` 里，且其唯一内部边是 `continuum-persist`）。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(memory): 建 continuum-memory 骨架与 SourceKind／MemorySource 类型"
```

---

### Task 2: 迁移 190 的八表六索引

**Files:**
- Create: `crates/continuum-memory/src/persist.rs`
- Create: `crates/continuum-memory/tests/migrations.rs`
- Create: `crates/continuum-memory/tests/common/mod.rs`
- Modify: `crates/continuum-memory/src/lib.rs`

**Interfaces:**
- Consumes: `continuum_persist::{Migration, Db, Tx, Value}`
- Produces: `pub fn p6c_memory_migrations() -> Vec<Migration>`（版本 `[190]`，名称 `p6c_memory`，一次写齐八表六索引）。

**迁移 SQL 的列与索引（逐字，设计 §9.2）：**

```text
memory_source          source_id PK, kind, external_ref, observed_at, removed_at(可空)
memory_claim           claim_id PK, scope, statement, validity, confidence(TEXT/JSON), last_confirmed
memory_claim_source    (claim_id, source_id) PK
memory_claim_anchor    claim_id PK, anchor_id
memory_correction      source_id PK, claim_id UNIQUE, statement, occurred_at
memory_synthesis_job   job_id PK, scope, anchor_id(可空), trigger_kind, trigger_ref(TEXT/JSON),
                       status, started_at, finished_at(可空), report(TEXT/JSON, 可空)
memory_synthesis_input (job_id, source_id) PK
memory_summary         summary_id PK, scope, anchor_id(可空), body, job_id, generated_at, stale
```

六条索引逐名：`idx_memory_source_kind`、`idx_memory_source_ref`、`idx_memory_claim_source_source`、`idx_memory_claim_scope_validity_time`、`idx_memory_claim_anchor`、`idx_memory_summary_scope_time`（列序照设计 §9.2）。外键：`memory_claim_source` 两个方向、`memory_claim_anchor.claim_id`、`memory_correction` 两个方向、`memory_synthesis_input` 两个方向、`memory_summary.job_id`；`Db::open_with` 已开 `PRAGMA foreign_keys=ON`。所有 TEXT 列 `NOT NULL`，可空列只有 `removed_at`／`anchor_id`／`finished_at`／`report`。

**本 task 的拒绝点：** 无逻辑臂；迁移失败走 `Persist`。照片是 R1、S5、S7、S9。

- [ ] **Step 1: 写失败用例**

`tests/migrations.rs`（用 `Db::open_with(path, p6c_memory_migrations())` 起测试库；`tests/common/mod.rs` 里放 `new_db() -> (tempfile::TempDir, Db)`）：

- `the_migration_is_the_only_producer_of_version_190`（R1）：`p6c_memory_migrations()` 的 version 恰 `[190]`、name 是 `p6c_memory`；`migrate()` 后 `schema_migrations` 里有 `(190, "p6c_memory")`。
- `the_migration_creates_eight_tables_and_six_indexes`（R1）：`sqlite_master` 里 `type='table'` 的八张表逐名存在；`type='index'` 的六条索引逐名存在（`sqlite_autoindex_*` 不计）。
- `memory_claim_has_exactly_six_scalar_columns`（S7）：`PRAGMA table_info(memory_claim)` 的列名序列恰为 `["claim_id","scope","statement","validity","confidence","last_confirmed"]`；没有 `status`／`created_at`／`anchor_id`。
- `each_of_the_eight_tables_has_exactly_one_create_table`（S9）：对八张表逐表 `sql.matches("CREATE TABLE <name>").count() == 1`（**不是**「表名只出现一次」——表名还会出现在索引与 INSERT 里）。
- `migration_190_is_declared_once_in_persist_rs`（S5）：`include_str!("../src/persist.rs")` 里 `"Migration::new(190"` 恰出现一次，且该文件里 `p6c_memory_migrations` 恰定义一个 `pub fn`。
- `the_reference_tables_declare_their_foreign_keys`：`PRAGMA foreign_key_list` 的行数：`memory_claim_source` 2、`memory_claim_anchor` 1、`memory_correction` 2、`memory_synthesis_input` 2、`memory_summary` 1。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test migrations
```

**预期**：编译失败（`p6c_memory_migrations` 未定义，`E0433`／`E0425`）。

- [ ] **Step 3: 实现迁移 190**

`src/persist.rs` 只放 `p6c_memory_migrations()`；SQL 用**一条** `Migration::new(190, "p6c_memory", "...")`，八条 `CREATE TABLE` 与六条 `CREATE INDEX` 全在那一个 `&'static str` 里，语句以 `;` 分隔。建表顺序：`memory_source` → `memory_claim` → `memory_claim_source` → `memory_claim_anchor` → `memory_correction` → `memory_synthesis_job` → `memory_synthesis_input` → `memory_summary`（FK 的参照表先建）。**不要**给任何表加设计 §9.2 之外的列。`src/lib.rs` 加 `mod persist; pub use persist::p6c_memory_migrations;`。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test migrations
```

**预期**：6 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): 迁移 190 的八表六索引"
```

---

### Task 3: Source 存储（`insert_source`／`load_source`／`load_sources`）

**Files:**
- Modify: `crates/continuum-memory/src/source.rs`
- Modify: `crates/continuum-memory/src/error.rs`（加 `DuplicateSourceId`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/source_store.rs`
- Modify: `crates/continuum-memory/tests/common/mod.rs`（只加 `source(...)` 夹具；`new_db()` 已在 Task 2 加）

**Interfaces:**
- Consumes: Task 1 的 `MemorySource`／`MemorySourceId`／`SourceKind`；Task 2 的表
- Produces:
  - `pub fn insert_source(tx: &Tx<'_>, source: &MemorySource) -> Result<(), MemoryError>`
  - `pub fn load_source(tx: &Tx<'_>, id: &MemorySourceId) -> Result<Option<MemorySource>, MemoryError>`
  - `pub fn load_sources(tx: &Tx<'_>, ids: &[MemorySourceId]) -> Result<Vec<MemorySource>, MemoryError>`

**本 task 的拒绝点 → 臂 → 照片：** `insert_source` 的 `source_id` 已存在 → `DuplicateSourceId` → R31。`load_source` 未知 id 返回 `Ok(None)`，不是错误（设计 §10.1 的读操作没有逻辑拒绝臂）。

**本计划定的读回形状（设计只给了「load_sources 不排除已删」）：** `load_sources` 只返回存在的 id，按 `source_id` 升序；输入里的重复 id 按集合折叠；缺失的 id **静默省略**（与 `load_source` 的 `Option` 同向）。这条写进函数文档，并由用例钉住。

- [ ] **Step 1: 写失败用例**

`tests/source_store.rs`：

- `a_source_round_trips`（R2）：插入 `("s1", Files, "file:///a", 100)` → `load_source` 得五字段逐字相等、`removed_at() == None`。
- `a_duplicate_source_id_is_rejected_and_the_original_is_untouched`（R31）：同 id 第二次插入不同 kind／ref ⇒ `Err(DuplicateSourceId)`；再读原行，五字段仍是第一次的值（**不覆盖**）。
- `load_sources_returns_only_existing_ids_in_id_order_without_duplicates`：插入 `s3`／`s1`／`s2`，请求 `[s2, s1, s1, missing]` ⇒ 返回 `[s1, s2]`（升序、去重、缺失省略）。
- `an_unknown_source_reads_as_none`：`load_source("nope") == Ok(None)`。
- `removed_at_reads_back_as_a_timestamp`：直接 SQL 把某行 `removed_at` 置 `77`，`load_source` 得 `Some(77)`（读侧形状；删除路径在 Task 9）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test source_store
```

**预期**：编译失败（`insert_source` 等未定义）。

- [ ] **Step 3: 实现**

`insert_source`：先 `SELECT 1 FROM memory_source WHERE source_id = ?`，命中 ⇒ `Err(DuplicateSourceId)`；否则 `INSERT` 五列（`removed_at` 写 NULL，因为 `MemorySource::new` 造不出墓碑）。**不 `commit`**。

`load_source`：`SELECT source_id, kind, external_ref, observed_at, removed_at`；`kind` 用 `SourceKind::parse`，表外串 ⇒ `MemoryError::Persist(PersistError::Database(format!("memory_source.kind 表外取值: {raw}")))`（**不是**默认臂；设计 §10.4 第 10 条：这不是逻辑拒绝臂，故不拍照）。`removed_at` 的 `Value::Null` ⇒ `None`，`Int` ⇒ `Some`。

`load_sources`：把输入 id 去重、按 `as_str` 升序；逐 id 调 `load_source`，`Some` 收集、`None` 跳过。**不做 SQL `IN`**，避免拼参数个数；条数小、路径只读。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test source_store
```

**预期**：5 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): Source 的插入与读回（软删除墓碑可读）"
```

---

### Task 4: Claim 类型、scope/anchor 规则与 claim 行读回

**Files:**
- Create: `crates/continuum-memory/src/claim.rs`
- Modify: `crates/continuum-memory/src/error.rs`（加 `EmptyStatement`／`EmptySourceSet`／`DuplicateSourceId`（已有则复用）／`AnchorRequired`／`AnchorNotAllowed`／`EmptyScopeSet`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Modify: `crates/continuum-memory/Cargo.toml`（加 `[features] test-support = []`）
- Create: `crates/continuum-memory/tests/claim.rs`（公开编码与 `Confidence`；`test-support` 下的 `for_test` 用例）
- `src/claim.rs` 内的 `#[cfg(test)] mod tests`（私有构造函数、`validate_scope_anchor`、`load_claim`）

**Interfaces:**
- Consumes: Task 1 的 `MemorySourceId`；Task 2 的表；Task 3 的 source 读回（`load_claim` 不需要）
- Produces:
  - `MemoryScope`（五臂、`ALL: [MemoryScope; 5]`、`as_str`／`parse`、`pub(crate) is_anchored() -> bool`）
  - `Validity`（`Active`／`Superseded`／`Invalidated`、`ALL: [Validity; 3]`、`as_str`／`parse`）
  - `Confidence::{from_value(serde_json::Value),as_value() -> &serde_json::Value}`（`Debug, Clone, PartialEq`；**无 `Ord`／`PartialOrd`／默认值**）
  - `MemoryClaimId::{new,as_str}`（`Debug, Clone, PartialEq, Eq, Hash`）
  - `MemoryClaim`（七字段全私有；`Debug, Clone, PartialEq`；访问器见跨计划前置第三节）
  - `pub(crate) fn validate_scope_anchor(scopes: &[MemoryScope], anchor: Option<&str>) -> Result<(), MemoryError>`
  - `pub(crate) fn load_claim(tx: &Tx<'_>, id: &MemoryClaimId) -> Result<Option<MemoryClaim>, MemoryError>`
  - `pub(crate) fn claim_anchor(tx: &Tx<'_>, id: &MemoryClaimId) -> Result<Option<String>, MemoryError>`
  - `impl MemoryClaim { pub(crate) fn new(id: MemoryClaimId, scope: MemoryScope, statement: String, sources: Vec<MemorySourceId>, confidence: Confidence, last_confirmed: i64) -> Result<MemoryClaim, MemoryError> }`（只造 `Active`；`statement` 去首尾空白后存；`sources` 空 ⇒ `EmptySourceSet`、重复 ⇒ `DuplicateSourceId`、否则按 id 升序；crate 外不可见）
  - `impl MemoryClaim { pub(crate) fn rehydrate(id: MemoryClaimId, scope: MemoryScope, statement: String, sources: Vec<MemorySourceId>, validity: Validity, confidence: Confidence, last_confirmed: i64) -> MemoryClaim }`（**不校验**，只给库行还原用）
  - `#[cfg(feature = "test-support")] #[doc(hidden)] pub fn for_test(id: MemoryClaimId, scope: MemoryScope, statement: impl Into<String>, sources: Vec<MemorySourceId>, validity: Validity, confidence: Confidence, last_confirmed: i64) -> MemoryClaim`（`impl MemoryClaim` 内的关联函数；仅测试 feature；**不校验、不归一**，与 `rehydrate` 同形；默认构建下不存在。`new` 的可见性不变，仍 `pub(crate)`）

**本 task 的拒绝点 → 臂 → 照片：** claim 构造的 `EmptyStatement`／`EmptySourceSet`／`DuplicateSourceId`（§5.1）；`validate_scope_anchor` 的 `EmptyScopeSet`／`AnchorRequired`／`AnchorNotAllowed`（§5.4，读取侧照片在 Task 7 的 R27，写入侧在 Task 5 的 R32）。`Confidence` 的任意 JSON 往返是 R22。

- [ ] **Step 1: 写失败用例**

`tests/claim.rs`：

- `memory_scope_has_the_five_arms_of_284`（S6）：`MemoryScope::ALL.len() == 5`；逐臂 `parse(as_str()) == Some(scope)`；编码 `personal`／`project`／`product`／`task`／`runtime`。
- `memory_scope_parse_does_not_default`：`parse("session")`、`parse("PERSONAL")`、`parse("")` 均 `None`。**红条件**：给 `Session` 留默认（放宽档）⇒ 红。
- `validity_has_three_arms_and_round_trips`（S8）：`Validity::ALL == [Active, Superseded, Invalidated]`；逐臂往返；`Superseded.as_str() == "superseded"`（§286 的词，大小写按本仓小写约定）；**表外串不取默认臂**：`Validity::parse("unknown")`／`parse("ACTIVE")`／`parse("")` 均 `None`。
- `confidence_round_trips_arbitrary_json`（R22）：`null`、`"high"`、`{"grade":"medium","score":2}`、`[1,2,3]` 各 `from_value` 后 `as_value() == &input`（`serde_json` 会规范化空白／键序，故只承诺**值相等**）。
- `confidence_has_no_default_and_no_order`：只做编译期说明——运行期无法断言「没有 `Ord`」；本用例只断言 `Confidence::from_value(serde_json::json!(null))` 可构造，不依赖默认值。排序的负例在 Task 11 的 C6。

`src/claim.rs` 的单元测试：

- `a_claim_is_constructed_only_active_with_normalized_sources`：`new` 后 `validity() == Active`；`sources()` 按 id 升序；`statement()` 已 trim。
- `a_claim_constructor_rejects_empty_statement_and_empty_sources`：`"  "` ⇒ `EmptyStatement`；空 `sources` ⇒ `EmptySourceSet`。
- `a_claim_constructor_rejects_duplicate_sources`：`[s2, s1, s2]` ⇒ `DuplicateSourceId`；**不静默去重**。
- `validate_scope_anchor_applies_the_single_shape`：`[Project] + None` ⇒ `AnchorRequired`；`[Personal] + Some("p")` ⇒ `AnchorNotAllowed`；`[Personal, Project] + None` ⇒ `AnchorRequired`；`[Personal, Project] + Some("p")` ⇒ `AnchorNotAllowed`；`[Project] + Some("p")`／`[Personal] + None` ⇒ `Ok`；`[]` ⇒ `EmptyScopeSet`。
- `load_claim_round_trips_a_seeded_row`：直接 SQL 插一条 claim 与两条边；`load_claim` 得七字段；`sources()` 升序；`claim_anchor` 对 anchored 行返回 `Some`、对 unanchored 行返回 `None`。
- `rehydrate_does_not_validate`：用空 statement／空 sources 直接 `rehydrate` 能成功（库行已过写入口；它不承担校验）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test claim
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --lib
```

**预期**：两处均编译失败（`MemoryScope`／`MemoryClaim` 未定义）。

- [ ] **Step 3: 实现**

`claim.rs` 的类型定义逐字照设计 §5.1／§5.2／§5.3：`MemoryScope` 与 `Validity` 派生 `Debug, Clone, Copy, PartialEq, Eq`；`MemoryClaim` 字段顺序 `id, scope, statement, sources, validity, confidence, last_confirmed`；`MemoryClaim` 不派生 `Eq`（`serde_json::Value` 只到 `PartialEq`）。`MemoryScope::is_anchored()` 只对 `Project`／`Product`／`Task` 返回 true。`validate_scope_anchor` 的判定顺序：先空 `scopes` ⇒ `EmptyScopeSet`；再 `anchor.is_some()` 时要求 `scopes.len() == 1 && scopes[0].is_anchored()`，否则 `AnchorNotAllowed`；`anchor.is_none()` 时任一 `is_anchored()` ⇒ `AnchorRequired`。

`load_claim`：查 `memory_claim` 六列；`scope`／`validity` 用 `parse`，`confidence` 用 `serde_json::from_str`，表外／坏 JSON ⇒ `Persist(PersistError::Database(...))`；再查 `memory_claim_source` 的 `source_id`（升序），用 `rehydrate` 还原。`claim_anchor` 查 `memory_claim_anchor.anchor_id`，无行 ⇒ `None`。

- [ ] **Step 4: 落 `test-support` feature、`for_test` 与一条 feature-gated 用例**

`crates/continuum-memory/Cargo.toml` 加（放在 `[dev-dependencies]` 段之后）：

```toml
[features]
test-support = []
```

空 feature 只开可见性，不增依赖，默认构建不启用。

先在 `tests/claim.rs` 加一条（**先写用例，确认编译失败**——本步红在编译期：`for_test` 尚不存在，不是运行期断言变红）：

```rust
#[cfg(feature = "test-support")]
#[test]
fn for_test_is_available_under_the_test_support_feature() {
    let claim = MemoryClaim::for_test(
        MemoryClaimId::new("c-test"),
        MemoryScope::Personal,
        "test-only",
        vec![MemorySourceId::new("s-test")],
        Validity::Superseded,
        Confidence::from_value(serde_json::json!(1.0)),
        42,
    );
    assert_eq!(claim.id().as_str(), "c-test");
    assert_eq!(claim.scope(), MemoryScope::Personal);
    assert_eq!(claim.statement(), "test-only");
    assert_eq!(claim.sources().len(), 1);
    assert_eq!(claim.sources()[0].as_str(), "s-test");
    assert_eq!(claim.validity(), Validity::Superseded);
    assert_eq!(claim.confidence(), &Confidence::from_value(serde_json::json!(1.0)));
    assert_eq!(claim.last_confirmed(), 42);
}
```

`tests/claim.rs` 顶部确保有下面两条 import（已有则补缺）。**三个 feature-only 名字只在 `#[cfg(feature = "test-support")]` 用例里出现，import 必须同样 cfg-gate**——否则默认构建（feature 关闭）会报 `unused_imports`，`cargo test -p continuum-memory --test claim` 与 Task 12 的 `--workspace` 0-warning 门都会红。**不得用 `#[allow(unused_imports)]` 掩盖。**

```rust
use continuum_memory::{Confidence, MemoryScope, Validity};

#[cfg(feature = "test-support")]
use continuum_memory::{MemoryClaim, MemoryClaimId, MemorySourceId};
```

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test claim --features test-support
```

**预期**：编译失败 `E0599`（`MemoryClaim::for_test` 尚不存在）。然后在 `src/claim.rs` 加：

```rust
impl MemoryClaim {
    /// 仅供跨 crate 测试装配；未开 `test-support` 时本方法不存在。
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
    ) -> MemoryClaim {
        MemoryClaim::rehydrate(
            id,
            scope,
            statement.into(),
            sources,
            validity,
            confidence,
            last_confirmed,
        )
    }
}
```

重跑同一条命令：**预期公开 6 条全 PASS**（默认 5 条＋本 feature 下 1 条）。再跑默认构建：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test claim
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --lib
```

**预期**：默认公开 5 条、单元 6 条全 PASS；feature 用例不参与编译。**`new` 的可见性不变**：C2 仍只探 `MemoryClaim::new`，默认与 `--all-features` 下都编不过（E0624）；`for_test` 是另一个入口，不会让 `new` 变 `pub`。

- [ ] **Step 5: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test claim
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test claim --features test-support
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --lib
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --lib --features test-support
```

**预期**：默认公开 5 条、单元 6 条全 PASS；`--features test-support` 时公开 6 条（多一条 `for_test` 用例）、单元 6 条全 PASS。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): MemoryClaim 七字段、五 scope 与 scope/anchor 规则"
```

---

### Task 5: Synthesis 作业的 `begin_synthesis` 与 `fail_synthesis`

**Files:**
- Create: `crates/continuum-memory/src/synthesis.rs`
- Modify: `crates/continuum-memory/src/error.rs`（加 `UnknownSource`／`SourceRemoved`／`DuplicateJobId`／`UnknownJob`／`JobNotRunning`／`TriggerSourceOutsideInput`／`WrongTriggerSourceKind`／`EmptyTriggerSet`／`InvalidTriggerTime`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/synthesis_job.rs`
- Modify: `crates/continuum-memory/tests/common/mod.rs`（加 `begin_request(...)` 夹具）

**Interfaces:**
- Consumes: Task 3 的 `insert_source`／`load_source`；Task 4 的 `validate_scope_anchor`／`MemoryScope`
- Produces:
  - `SynthesisJobId::{new,as_str}`（`new` **不校验唯一性**）
  - `SynthesisStatus`（`Running`／`Applied`／`Failed`、`as_str`／`parse`）
  - `SynthesisTrigger`（四臂；`as_str` 穷尽 `match`，**不提供 `parse`**）
  - `SynthesisRequest { pub job: SynthesisJobId, pub scope: MemoryScope, pub anchor: Option<String>, pub trigger: SynthesisTrigger, pub sources: Vec<MemorySourceId>, pub started_at: i64 }`
  - `pub fn begin_synthesis(tx, request: &SynthesisRequest) -> Result<(), MemoryError>`
  - `pub fn fail_synthesis(tx, job: &SynthesisJobId, reason: &str, occurred_at: i64) -> Result<(), MemoryError>`

**本 task 的拒绝点 → 臂 → 照片（G0–G6，设计 §8.2）：**

| 判据 | 拒绝点 | 臂 | 照片 |
|---|---|---|---|
| G0 | `sources` 为空 | `EmptySourceSet` | R35 |
| G0 | `sources` 有重复 id | `DuplicateSourceId` | R31 |
| G0 | source 不存在 | `UnknownSource` | R33 |
| G0 | source 已 `removed_at` 非空 | `SourceRemoved` | R43 |
| G1 | 触发携带的 source 不在 `sources` | `TriggerSourceOutsideInput` | R42 |
| G2 | `SessionEnded.session` 的 kind 不是 `Conversations` | `WrongTriggerSourceKind` | R42 |
| G2b | `Bulk.changed[]` 的 kind 不是 `Project` | `WrongTriggerSourceKind` | R42 |
| G3 | `Bulk.changed` 为空 | `EmptyTriggerSet` | R42 |
| G4 | `SystemIdle.since > started_at` | `InvalidTriggerTime` | R42 |
| G5 | scope/anchor 规则不满足 | `AnchorRequired`／`AnchorNotAllowed` | R32 |
| G6 | `job` 已存在 | `DuplicateJobId` | R38 |
| 状态机 | `fail_synthesis` 的 job 不存在 | `UnknownJob` | R39 |
| 状态机 | `fail_synthesis` 的 job 非 `Running` | `JobNotRunning` | R39 |

**`trigger_ref` 的 JSON 形状（设计 §8.2 给了四个 payload 值、没给键名）：** `SessionEnded` ⇒ `{"session":"<id>"}`；`SystemIdle` ⇒ `{"since":<i64>}`；`BulkProjectStateChanged` ⇒ `{"changed":["<id>",…]}`；`UserRequested` ⇒ `null`。`trigger_kind` 取 `SynthesisTrigger::as_str` 的四枚 `session_ended`／`system_idle`／`bulk_project_state_changed`／`user_requested`；`changed` 数组保留请求里的顺序（不去重、不排序——重复 id 由 G0 的 `sources` 判，`changed` 本身不承担唯一性）。

- [ ] **Step 1: 写失败用例**

`tests/synthesis_job.rs`：

- `begin_writes_a_running_job_and_its_inputs`：`begin` 后 `memory_synthesis_job` 一行：`status = "running"`、`scope`／`anchor_id`／`trigger_kind`／`trigger_ref`／`started_at` 正确、`finished_at IS NULL`、`report IS NULL`；`memory_synthesis_input` 的 source 集合等于请求的集合。
- `synthesis_status_parse_does_not_default`：`SynthesisStatus::parse("unknown")`／`parse("RUNNING")`／`parse("")` 均 `None`；`Running`／`Applied`／`Failed` 三臂 `parse(as_str())` 各自往返。
- `begin_rejects_an_empty_source_set`（R35）：`sources = []` ⇒ `EmptySourceSet`。
- `begin_rejects_duplicate_source_ids`（R31）：`[s1, s1]` ⇒ `DuplicateSourceId`。
- `begin_rejects_unknown_and_removed_sources`（R33／R43）：不存在的 id ⇒ `UnknownSource`；先插 source 再直接 SQL 置 `removed_at` ⇒ `SourceRemoved`。
- `begin_rejects_a_trigger_source_outside_the_input`（R42）：`SessionEnded { session: s2 }` 而 `sources = [s1]` ⇒ `TriggerSourceOutsideInput`。
- `begin_rejects_a_session_trigger_of_the_wrong_kind`（R42）：`sources = [f1]`（`Files`），`SessionEnded { session: f1 }` ⇒ `WrongTriggerSourceKind`。
- `begin_rejects_a_bulk_trigger_of_the_wrong_kind`（R42）：`Bulk.changed = [c1]`（`Conversations`）且 `c1 ∈ sources` ⇒ `WrongTriggerSourceKind`（G2b）。
- `begin_rejects_an_empty_bulk_trigger`（R42）：`Bulk.changed = []` ⇒ `EmptyTriggerSet`。
- `begin_rejects_an_idle_trigger_that_starts_after_the_job`（R42）：`since = started_at + 1` ⇒ `InvalidTriggerTime`；`since == started_at` ⇒ `Ok`（另一侧）。
- `begin_rejects_a_duplicate_job_id`（R38）：同 job id 第二次 `begin` ⇒ `DuplicateJobId`，且第一条 job 的输入集合不变。
- `begin_applies_scope_anchor_rules`（R32）：`Project + None` ⇒ `AnchorRequired`；`Personal + Some("p")` ⇒ `AnchorNotAllowed`；`Project + Some("p")`／`Personal + None` ⇒ `Ok`。
- `begin_records_the_four_triggers`：四臂各 `begin` 一次，逐条断言 `trigger_kind` 与 `trigger_ref` 的 JSON（上表形状）。
- `fail_marks_a_running_job_failed`：`fail` 后 `status = "failed"`、`finished_at = occurred_at`、`report` 解析为 `{"error":"..."}`。
- `fail_rejects_unknown_and_finished_jobs`（R39）：未知 job ⇒ `UnknownJob`；对 `Applied`／`Failed` 作业 ⇒ `JobNotRunning`。**夹具用直接 SQL 把 `begin` 出的 job 的 `status` 改成 `applied`／`failed`**（`apply_synthesis` 到 Task 8 才有，本 task 不调用它）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test synthesis_job
```

**预期**：编译失败（`SynthesisJobId` 等未定义）。

- [ ] **Step 3: 实现**

`begin_synthesis` 的判定顺序**照设计 §8.2 的表**：G0 → G1 → G2／G2b → G3 → G4 → G5 → G6。G0 先判空、再判重复、再逐条 `load_source`（`None` ⇒ `UnknownSource`，`removed_at` 非空 ⇒ `SourceRemoved`）；把已读出的 `(id, kind)` 留在局部 `Vec` 里供 G1／G2 使用。G1 对 `SessionEnded.session` 与 `Bulk.changed[]` 逐个检查 `sources` 是否包含该 id。G2／G2b 用局部 `Vec` 里的 kind 比对。G3 只对 `Bulk` 分支判空；G4 只对 `SystemIdle` 判 `since <= started_at`。G5 调 `validate_scope_anchor(&[request.scope], request.anchor.as_deref())`。G6 查 `memory_synthesis_job` 是否有同 id。**全部检查通过后**才插 job 与 inputs（避免拒绝路径留下半写）。`trigger_ref` 用 `serde_json::json!` 构造后 `to_string`；`UserRequested` 写字符串 `"null"`。

`fail_synthesis`：查 job 的 status；`None` ⇒ `UnknownJob`；`Some(s) if s != Running` ⇒ `JobNotRunning`；否则 `UPDATE ... SET status = 'failed', finished_at = ?, report = ?`，report 为 `{"error": reason}` 的 JSON。**不 `commit`**。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test synthesis_job
```

**预期**：15 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): 合成作业的 begin/fail 与四类触发准入"
```

---

### Task 6: Summary 类型与读写

**Files:**
- Create: `crates/continuum-memory/src/summary.rs`
- Modify: `crates/continuum-memory/src/error.rs`（本 task 不新增臂；复用 `AnchorRequired`／`AnchorNotAllowed`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/summary.rs`
- `src/summary.rs` 内的 `#[cfg(test)] mod tests`（`write_summary`／`mark_summaries_stale`）

**Interfaces:**
- Consumes: Task 4 的 `MemoryScope`、`validate_scope_anchor`；Task 5 的 `SynthesisJobId`；Task 2 的两张表
- Produces:
  - `MemorySummaryId::{new,as_str}`（`Debug, Clone, PartialEq, Eq, Hash`）
  - `pub struct MemorySummary { pub id: MemorySummaryId, pub scope: MemoryScope, pub anchor: Option<String>, pub body: String, pub job: SynthesisJobId, pub generated_at: i64, pub stale: bool }`（字段名照设计 §8.4）
  - `pub fn load_summary(tx, scope: MemoryScope, anchor: Option<&str>) -> Result<Option<MemorySummary>, MemoryError>`
  - `pub fn load_current_summary(tx, scope, anchor) -> Result<Option<MemorySummary>, MemoryError>`
  - `pub(crate) fn write_summary(tx, job: &SynthesisJobId, scope, anchor: Option<&str>, body: &str, generated_at: i64) -> Result<MemorySummaryId, MemoryError>`（**只许 `apply_synthesis` 调**；id = `format!("{}:{}", job.as_str(), generated_at)`）
  - `pub(crate) fn mark_summaries_stale(tx, scope, anchor: Option<&str>) -> Result<usize, MemoryError>`（只把 `stale = 0` 的行改成 1，返回受影响行数）

**本 task 的拒绝点 → 臂 → 照片：** `load_summary`／`load_current_summary` 的 scope/anchor 规则 → `AnchorRequired`／`AnchorNotAllowed` → R32（与 Task 5／7 同一规则）。`write_summary` 的重复 id 走 `Persist`（数据库约束），不设逻辑臂；摘要正文由调用方先判空（Task 8 的 `EmptySummary`）。

- [ ] **Step 1: 写失败用例**

`tests/summary.rs`（**只列公开面**；集成测试是独立 crate，`pub(crate)` 的 `write_summary`／`mark_summaries_stale` 在这里不可见，它们的用例放在 `src/summary.rs` 的单元测试里。直接 SQL 造 job 行与 summary 行；`memory_summary.job_id` 有 FK，先插 job）：

- `load_summary_returns_the_latest_by_generated_at_then_id`（R28）：同 `(Project, Some("p1"))` 两条：`generated_at = 100` 与 `200`；`load_summary` 返回 200 那条。再插一条同 `200` 但 id 字典序更小的 ⇒ 返回 id 更小者（`generated_at DESC, summary_id ASC`）。
- `load_current_summary_skips_stale_rows`（R28）：最新一条 `stale = 1`、次新 `stale = 0` ⇒ `load_current_summary` 返回次新；`load_summary` 返回最新（可 stale）。
- `all_stale_yields_none`（R28）：两条都 stale ⇒ `load_current_summary == Ok(None)`。
- `empty_table_reads_as_none`：空表上 `load_summary(Project, Some("p1")) == Ok(None)`，`load_current_summary(Personal, None) == Ok(None)`（无行不是错误）。
- `summary_scope_anchor_rules_are_enforced`（R32）：`load_summary(Project, None)`／`load_current_summary(Project, None)` ⇒ `AnchorRequired`；`load_summary(Personal, Some("p"))`／`load_current_summary(Personal, Some("p"))` ⇒ `AnchorNotAllowed`；`[Project] + Some`／`[Personal] + None` ⇒ `Ok`。

`src/summary.rs` 的 `#[cfg(test)] mod tests`（直接调 `pub(crate)` 函数；同文件的单元测试模块能访问私有项）：

- `mark_summaries_stale_touches_only_the_pair`（R9／R19b 的基座）：同 scope 两个 anchor 各一条、另一 scope 一条；只把 `(Project, p1)` 置 stale ⇒ 计数 1；另两条仍 0；再调一次 ⇒ 计数 0（幂等）。
- `write_summary_is_fresh_and_uses_the_job`（I6）：`write_summary(job, Project, Some("p1"), "摘要", 300)` ⇒ id 等于 `format!("{}:300", job.as_str())`、`stale == false`、`job` 正确；`load_current_summary` 读到它。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test summary
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --lib
```

**预期**：编译失败（`MemorySummary` 等未定义）。

- [ ] **Step 3: 实现**

`MemorySummary` 的 `id`／`job` 是 newtype，其余照设计 §8.4。两个 `load_*` 都先调 `validate_scope_anchor(&[scope], anchor)`，再查 `memory_summary`；锚点 SQL 用 `anchor_id IS NULL` 或 `anchor_id = ?`，**不要**用 `COALESCE`（会把 `Some("")` 与 `None` 混同）。`load_summary` 排序 `generated_at DESC, summary_id ASC LIMIT 1`；`load_current_summary` 再加 `AND stale = 0`。`stale` 读 `Int`，`0` ⇒ false，非 0 ⇒ true。

`write_summary`：`summary_id = format!("{}:{}", job.as_str(), generated_at)`；插入七列，`stale = 0`；**不判正文**（空正文由 Task 8 的 `EmptySummary` 挡在调用前）。`mark_summaries_stale`：`UPDATE memory_summary SET stale = 1 WHERE scope = ? AND <anchor 条件> AND stale = 0`，返回 `execute` 的行数。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test summary
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --lib
```

**预期**：公开 5 条、本 task 单元 2 条、既有单元全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): MemorySummary 的读写与 stale 失效基座"
```

---

### Task 7: 检索面（`RetrievalQuery`／`MemoryRetrieval`／`MemoryStore`／`retrieve_claims`）

**Files:**
- Create: `crates/continuum-memory/src/retrieval.rs`
- Modify: `crates/continuum-memory/src/error.rs`（本 task 复用 `EmptyScopeSet`／`AnchorRequired`／`AnchorNotAllowed`；不加新臂）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/retrieval.rs`
- Modify: `crates/continuum-memory/tests/common/mod.rs`（加 `seed_claim(...)` 直接 SQL 夹具）

**Interfaces:**
- Consumes: Task 4 的 `MemoryClaim`／`MemoryScope`／`validate_scope_anchor`／`load_claim`；Task 2 的表
- Produces:
  - `RetrievalQuery`（五字段全私有；`new(scopes: Vec<MemoryScope>, anchor: Option<String>, text: Option<String>, limit: usize, include_terminal: bool) -> Result<RetrievalQuery, MemoryError>`；访问器 `scopes() -> &[MemoryScope]`／`anchor() -> Option<&str>`／`text() -> Option<&str>`／`limit() -> usize`／`include_terminal() -> bool`）
  - `pub trait MemoryRetrieval { fn retrieve(&self, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError>; }`
  - `pub struct MemoryStore<'a> { /* &'a Tx<'a> */ }`＋`new`＋`impl MemoryRetrieval`
  - `pub fn retrieve_claims(tx: &Tx<'_>, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError>`

**本 task 的拒绝点 → 臂 → 照片：** `RetrievalQuery::new` 的空 `scopes` → `EmptyScopeSet`（R21）；anchored 缺 anchor → `AnchorRequired`；anchor 与 scopes 不匹配（含混排）→ `AnchorNotAllowed`（R27）。`limit = 0` 不是拒绝，返回 `Ok(vec![])`（R21）。读操作只透传 `Persist`。

- [ ] **Step 1: 写失败用例**

`tests/retrieval.rs`（用 `seed_claim` 直接写 `memory_claim`／`memory_claim_source`／`memory_claim_anchor`；不经过 `apply_synthesis`，本 task 还没有它）：

- `query_construction_applies_the_anchor_rules`（R27）：`new([Project], None, ..)` ⇒ `AnchorRequired`；`new([Personal], Some("p"), ..)`、`new([Personal, Project], Some("p"), ..)`、`new([Project, Product], Some("p"), ..)` ⇒ `AnchorNotAllowed`；`new([Project], Some("p"), ..)`／`new([Personal], None, ..)` ⇒ `Ok`。
- `query_dedupes_scopes_without_error`：`new([Project, Project], Some("p"), ..)` ⇒ 去重后 `scopes().len() == 1`（顺序按首次出现）；**不**报错。另一侧：`new([Personal, Personal], None, ..)` ⇒ `scopes().len() == 1`。
- `query_normalizes_empty_text_to_none`：`new([Personal], None, Some(String::new()), 10, false)` ⇒ `text() == None`；`Some("  ")` 原样保留为 `Some("  ")`（设计只归一 `Some("")`）。
- `limit_zero_returns_empty_and_empty_scopes_is_an_error`（R21）：`new([], None, ..)` ⇒ `EmptyScopeSet`；合法 query 的 `limit = 0` ⇒ `retrieve_claims` 返回 `Ok(vec![])`（不是 `Err`）。
- `retrieve_returns_only_active_claims_by_default`（R20）：seed 三条：Active／Superseded／Invalidated；默认查询只返回 Active；`include_terminal = true` 返回三条。
- `scope_and_anchor_filter`（R20）：两个项目 `p1`／`p2` 各一条 Active，`Personal` 一条；`[Project] + Some("p1")` 只返回 p1；`[Personal] + None` 只返回 personal；`[Project, Product]` 无 anchor 是构造错误（上面已钉）。
- `text_filter_is_case_sensitive_substring`：statement `"Core 不再使用 move"`；`Some("move")` 命中；`Some("MOVE")` 不命中；`Some("不再使用")` 命中。**不**做大小写折叠／Unicode 归一／词干化。
- `order_is_last_confirmed_desc_then_id_asc`（R20）：三条同 scope：`(last_confirmed=100,id=b)`、`(200,id=a)`、`(200,id=c)` ⇒ 顺序 `[a, c, b]`。
- `limit_caps_without_panicking`：`limit = 2` 只返回前两条；`limit = 100` 返回全部。
- `memory_store_and_free_function_agree`（R25）：同一 query 经 `MemoryStore::new(&tx)` 的 `&dyn MemoryRetrieval` 与 `retrieve_claims(tx, &q)` 返回逐项相等的 `Vec<MemoryClaim>`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test retrieval
```

**预期**：编译失败（`RetrievalQuery`／`MemoryStore` 未定义）。

- [ ] **Step 3: 实现**

`RetrievalQuery::new`：先按首次出现去重 `scopes`；`text == Some(String::new())` ⇒ `None`；调 `validate_scope_anchor(&scopes, anchor.as_deref())`；`limit` 原样接受（0 合法）。五个字段全私有。

`retrieve_claims`：`limit == 0` 直接 `Ok(vec![])`。SQL 形如：

```sql
SELECT claim_id FROM memory_claim c
WHERE 1 = 1
  [AND c.validity = 'active']                 -- include_terminal == false
  AND c.scope IN (?, ...)
  [AND instr(c.statement, ?) > 0]             -- text == Some(s)
  [AND EXISTS (SELECT 1 FROM memory_claim_anchor a
               WHERE a.claim_id = c.claim_id AND a.anchor_id = ?)]   -- anchor == Some
ORDER BY c.last_confirmed DESC, c.claim_id ASC
LIMIT ?
```

逐条 id 调 `load_claim` 还原（N+1 可接受；本块的检索是确定性、单机、小集合）。`MemoryStore<'a> { tx: &'a Tx<'a> }`；`new` 只存引用；`impl MemoryRetrieval` 调自由函数。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test retrieval
```

**预期**：10 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): 确定性检索面与 P6A 冻结的 MemoryRetrieval"
```

---

### Task 8: `apply_synthesis`（A1–A5、全部拒绝点）

**Files:**
- Modify: `crates/continuum-memory/src/synthesis.rs`
- Modify: `crates/continuum-memory/src/error.rs`（加 `DuplicateClaimId`／`UnknownClaim`／`ClaimNotActive`／`ForeignSource`／`ScopeMismatch`／`SupersedeMergeConflict`／`EmptySummary`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/apply_synthesis.rs`
- Modify: `crates/continuum-memory/tests/common/mod.rs`（加 `apply_request(...)`／`begin_and_apply(...)` 夹具）

**Interfaces:**
- Consumes: Task 3 的 `load_source`；Task 4 的 `MemoryClaim::new`／`load_claim`／`claim_anchor`；Task 6 的 `write_summary`／`mark_summaries_stale`；Task 5 的 `begin_synthesis`／job 类型；Task 7 的 `retrieve_claims`
- Produces:
  - `ProposedClaim { pub id: MemoryClaimId, pub statement: String, pub sources: Vec<MemorySourceId>, pub confidence: Confidence, pub supersedes: Vec<MemoryClaimId> }`
  - `SynthesisProposal { pub claims: Vec<ProposedClaim>, pub summary: Option<String> }`
  - `SynthesisReport { pub created: Vec<MemoryClaimId>, pub merged: Vec<MemoryClaimId>, pub superseded: Vec<MemoryClaimId>, pub summary_written: bool, pub staled_summaries: usize }`
  - `pub fn apply_synthesis(tx, job: &SynthesisJobId, proposal: &SynthesisProposal, occurred_at: i64) -> Result<SynthesisReport, MemoryError>`

**本 task 的拒绝点 → 臂 → 照片（设计 §8.4）：**

| 步骤 | 拒绝点 | 臂 | 照片 |
|---|---|---|---|
| A1 | job 不存在 | `UnknownJob` | R39 |
| A1 | job 非 `Running` | `JobNotRunning` | R16 |
| A2.1 | claim id 已在库中或本批重复 | `DuplicateClaimId` | R36 |
| A2.2 | proposal `statement` 去空白为空 | `EmptyStatement` | R35 |
| A2.3 | proposal `sources` 为空 | `EmptySourceSet` | R15 |
| A2.3 | proposal `sources` 重复 | `DuplicateSourceId` | R31 |
| A2.4 | source 不存在 | `UnknownSource` | R33 |
| A2.4 | source 已删 | `SourceRemoved` | R14 |
| A2.6 | 新来源不在作业输入集合 | `ForeignSource` | R13 |
| A2.7 | `supersedes` 目标不存在 | `UnknownClaim` | R36 |
| A2.7 | `supersedes` 目标非 `Active` | `ClaimNotActive` | R37 |
| A2.7 | `supersedes` 目标跨 scope/anchor | `ScopeMismatch` | R17 |
| A2.7 | `supersedes` 目标是本条合并目标 | `SupersedeMergeConflict` | R41 |
| A3.2 | `summary = Some("")` 或纯空白 | `EmptySummary` | R40 |

**`apply_synthesis` 的执行顺序（本计划钉死；不许改顺序）：**

1. **A1**：读 job 行（`scope`／`anchor_id`／`status`）。`None` ⇒ `UnknownJob`；`status != Running` ⇒ `JobNotRunning`；`scope` 用 `MemoryScope::parse`。读 `memory_synthesis_input` 得输入集合 `job_sources: HashSet<String>`。
2. **A2 逐条 proposal，按顺序**：
   1. `id` 与库中已有 claim 或本批已见 id 重复 ⇒ `DuplicateClaimId`。
   2. `statement.trim()` 为空 ⇒ `EmptyStatement`；否则 **trim 后**作为新 claim 的 statement（合并键也用 trim 后的值）。
   3. `sources` 空 ⇒ `EmptySourceSet`；重复 ⇒ `DuplicateSourceId`；按 id 升序规范化。
   4. 逐条 `load_source`：`None` ⇒ `UnknownSource`；`removed_at` 非空 ⇒ `SourceRemoved`。**已连到合并目标的旧 source 也要过这一关**（设计 A2.4 说的是「每个 source」）。
   5. **去重键（只判定，不落库）**：同 `scope`、同 anchor（anchored 用 `memory_claim_anchor.anchor_id = ?`；unanchored 用 `NOT EXISTS`）、`validity = 'active'`、`statement` 逐字相等的最早一条 claim（`ORDER BY claim_id ASC LIMIT 1`）。
      - 命中 ⇒ **合并判定**：`load_claim` 取旧 sources／旧 `last_confirmed`；算出 `new_sources`（提案里尚未连上的）；`last_confirmed = max(旧值, 提案全部 source 的 observed_at 最大值)`；`confidence` 用提案值**覆盖**；记下本条改动的 `(scope, anchor)` 与合并目标 id。**此步不写任何行。**
      - 未命中 ⇒ **新建判定**：用 `MemoryClaim::new(...)` 构造（只造 `Active`；`last_confirmed = 提案全部 source 的 observed_at 最大值`）；记下 `(scope, anchor)` 与新建 id。**此步不写任何行。**
   6. **来源边界**：新建 ⇒ 提案全部 source 必须 ∈ `job_sources`；合并 ⇒ **新增的** source 必须 ∈ `job_sources`，已连的旧 source 可以在输入集合外。违反 ⇒ `ForeignSource`（此时尚未写任何 claim／边）。通过后**才落库**：合并插 `new_sources` 的边、更新 `last_confirmed`／`confidence`；新建插 `memory_claim`＋`memory_claim_source`＋（anchored 时）`memory_claim_anchor`。
   7. **`supersedes` 逐条**：目标不存在 ⇒ `UnknownClaim`；`validity != Active` ⇒ `ClaimNotActive`；目标的 `scope` 与 `claim_anchor` 必须等于作业的 `(scope, anchor)`，否则 ⇒ `ScopeMismatch`；若目标 id == 本条的合并目标 id ⇒ `SupersedeMergeConflict`。通过 ⇒ `UPDATE memory_claim SET validity = 'superseded' WHERE claim_id = ? AND validity = 'active'`（受影响行数必须 1，否则 `ClaimNotActive`），记下 `(scope, anchor)`。
3. **A3.1 先失效**：把第 2 步记下的所有 `(scope, anchor)`（去重）逐对调 `mark_summaries_stale`；`staled_summaries` 累加返回行数。**即使 `proposal.summary == None` 也做这一步**。
4. **A3.2 再写新摘要**：`proposal.summary == Some(body)`：`body.trim()` 为空 ⇒ `EmptySummary`；否则 `write_summary(job, scope, anchor, body /* 原样 */, occurred_at)`，`summary_written = true`。`None` ⇒ 不写；若第 2 步没有 claim 变更，旧摘要保持原状。
5. **A4 收尾**：`UPDATE memory_synthesis_job SET status = 'applied', finished_at = ?, report = ? WHERE job_id = ?`；report 是 `SynthesisReport` 的 JSON（`created`／`merged`／`superseded` 为字符串数组，`summary_written` 布尔，`staled_summaries` 整数）。
6. **A5**：以上全在同一 `Tx`；**不 `commit`**。`Err` 时调用方丢弃句柄回滚，job 仍是 `Running`。

- [ ] **Step 1: 写失败用例**

`tests/apply_synthesis.rs`：

- `a_new_claim_has_seven_fields_and_max_observed_at`（R10）：作业 `(Project, Some("p1"))` 输入两个 source（`observed_at = 100`／`300`）；提案一条 `statement = "  Core 不再使用 move  "`；`apply` 后经 `retrieve_claims` 读回：`id`／`scope`／`statement`（**已 trim**）／`sources`（两条、升序）／`validity == Active`／`confidence` 值相等／`last_confirmed == 300`。
- `a_duplicate_statement_merges_into_one_claim`（R11）：同一作业两条提案、不同 id、同 statement、source 集合分别为 `{s1}` 与 `{s1,s2}`；`apply` 后只有一行 claim（合并目标是第一条的 id）、两条边、`last_confirmed` 取 max、`confidence` 是第二条的值；report 的 `created` 一条（第一条 id）、`merged` 一条（**合并目标的 id**，不是第二条提案的 id）。
- `different_statements_create_two_claims`（R12）：同 scope 两条不同 statement ⇒ 两行 Active claim。
- `a_foreign_source_is_rejected_and_nothing_is_written`（R13）：作业输入 `{s1}`，提案引用 `s2`（存在但未进输入）⇒ `ForeignSource`；丢弃句柄后新事务里 claim／边／摘要计数都是 0，job 仍 `Running`。
- `a_source_removed_between_begin_and_apply_is_rejected`（R14）：`begin` 后直接 SQL 把某输入 source 的 `removed_at` 置非空；`apply` 提案引用它 ⇒ `SourceRemoved`；job 仍 `Running`。
- `an_empty_proposal_source_set_is_rejected`（R15）：`sources = []` ⇒ `EmptySourceSet`。
- `a_second_apply_of_the_same_job_is_rejected`（R16）：第一次 apply 成功并 `commit`；第二次 apply ⇒ `JobNotRunning`；对 `Failed` 作业 apply 也 ⇒ `JobNotRunning`。
- `supersedes_another_anchor_is_rejected`（R17）：另一 anchor 的 Active claim 进 `supersedes` ⇒ `ScopeMismatch`。
- `superseding_then_reproposing_creates_a_new_active_claim`（R18）：job1 建 X；job2 用另一条提案并把 X 放进 `supersedes` ⇒ X 变 `Superseded`；job3 用与 X 相同的 statement、新 id 提案 ⇒ 新建一行 Active（**不是复活 X**）；`retrieve_claims(include_terminal = true)` 同时看到 X（Superseded）与新行。
- `a_summary_is_written_and_old_ones_are_staled`（R19）：先有同 `(Project, p1)` 的一条旧 fresh 摘要；job 带 `summary = Some("新摘要")` 且新建 claim ⇒ `load_summary` 返回新行、`stale == false`、`job` 正确；旧行 `stale == true`；report `staled_summaries == 1`、`summary_written == true`。
- `no_summary_stales_the_old_one_and_current_summary_is_none`（R19b）：job 新建 claim、`summary = None`、旧摘要 fresh ⇒ 旧摘要 `stale == true`；`load_current_summary == None`；`load_summary` 仍返回旧行（stale）。
- `no_summary_without_claim_changes_leaves_the_summary_fresh`（A3.2 的另一侧）：job 的 `claims` 为空、`summary = None`、旧摘要 fresh ⇒ 旧摘要仍 `stale == false`；report 的三个 id 数组为空、`staled_summaries == 0`；job 仍收尾为 `Applied`。
- `duplicate_source_ids_in_a_proposal_are_rejected`（R31）：`[s1, s1]` ⇒ `DuplicateSourceId`。
- `an_unknown_source_in_a_proposal_is_rejected`（R33）：不存在的 id ⇒ `UnknownSource`。
- `an_empty_statement_is_rejected`（R35）：`"   "` ⇒ `EmptyStatement`。
- `duplicate_claim_ids_are_rejected`（R36）：与本批前一条同 id ⇒ `DuplicateClaimId`；与库里已有 claim 同 id（另一个 job 先建）⇒ 同臂。
- `supersedes_an_unknown_claim_is_rejected`（R36）：不存在的 claim id ⇒ `UnknownClaim`。
- `supersedes_a_terminal_claim_is_rejected`（R37）：目标已被前一 job supersede ⇒ `ClaimNotActive`。
- `an_empty_summary_is_rejected`（R40）：`summary = Some("   ")` ⇒ `EmptySummary`；job 仍 `Running`，claim 未落库（丢弃句柄后回滚）。
- `supersedes_the_merge_target_is_rejected`（R41）：同 statement 的提案命中既有 X，同时 `supersedes = [X]` ⇒ `SupersedeMergeConflict`。
- `apply_to_an_unknown_or_failed_job_is_rejected`（R39）：未知 job ⇒ `UnknownJob`；`fail_synthesis` 后的 job ⇒ `JobNotRunning`。
- `the_report_counts_created_merged_superseded_and_staled`（R19）：一个 job 里新建 1、合并 1、退场 1，带 summary 且旧摘要 2 条 ⇒ report 四数逐项正确（`created.len()==1`、`merged.len()==1`、`superseded.len()==1`、`staled_summaries==2`）；job 行的 `report` JSON 与返回的 `SynthesisReport` 四数一致，`status = "applied"`、`finished_at = occurred_at`。
- `a_failed_apply_leaves_the_job_running`：提案第一条合法、第二条 `EmptyStatement` ⇒ `Err`；丢弃句柄后新事务里 job 仍 `Running`、第一条 claim 不存在（整笔回滚）。
- `apply_does_not_create_or_delete_sources_or_rewrite_claims`（I5）：apply 前后 `memory_source` 行数不变；被合并的旧 claim 的 `statement`／`id`／`scope` 不变；没有 `DELETE` 路径。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-memory --test apply_synthesis
```

**预期**：编译失败（`apply_synthesis`／`SynthesisProposal` 未定义）。

- [ ] **Step 3: 实现**

按上面「执行顺序」逐步实现；SQL 只用参数绑定，不拼字符串。去重键的 anchor 条件分两支写（anchored 用 `EXISTS`，unanchored 用 `NOT EXISTS`），**不要**用 `anchor_id = NULL`（恒假）。合并时 `last_confirmed` 用 `max(old, 提案 source 的 max observed_at)`；新建时用提案 source 的 max。`confidence` 落库用 `serde_json::to_string(proposal.confidence.as_value())`。report 用 `serde_json::json!` 构造后 `to_string`。**绝不**在 `apply_synthesis` 里 `INSERT INTO memory_source`／`DELETE`／改 `statement`／`claim_id`／`scope`（I5）。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-memory --test apply_synthesis
```

**预期**：24 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): apply_synthesis 的新建/合并/退场、摘要失效与作业收尾"
```

---

### Task 9: `remove_source`（软删除、无条件失效、摘要 stale）

**Files:**
- Modify: `crates/continuum-memory/src/claim.rs`
- Modify: `crates/continuum-memory/src/error.rs`（加 `SourceAlreadyRemoved`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/remove_source.rs`

**Interfaces:**
- Consumes: Task 3 的 `load_source`；Task 4 的 `load_claim`／`claim_anchor`；Task 6 的 `mark_summaries_stale`；Task 8 的 `apply_synthesis`（测试夹具）
- Produces: `pub fn remove_source(tx: &Tx<'_>, id: &MemorySourceId, occurred_at: i64) -> Result<Vec<MemoryClaimId>, MemoryError>`

**本 task 的拒绝点 → 臂 → 照片：** 不存在 ⇒ `UnknownSource`（R33）；`removed_at` 已非空 ⇒ `SourceAlreadyRemoved`（R34）。

**本计划定的返回集合：** 只返回**本次从 `Active` 改成 `Invalidated`** 的 claim id，按 `claim_id` 升序；引用该 source 但已是终态的 claim 不返回（它们没有被这次操作改动）。设计只说「返回这些 claim 的 id」，这里取「被改动的那些」。

- [ ] **Step 1: 写失败用例**

`tests/remove_source.rs`（用 `begin_synthesis`＋`apply_synthesis` 造真 claim）：

- `removing_a_source_invalidates_every_claim_that_references_it_and_no_other`（R3）：source A／B；claim X 引用 A，claim Y 引用 A＋B，claim Z 只引用 B；`remove_source(A)` 后 X／Y 经 `retrieve_claims(include_terminal = true)` 读回 `Invalidated`，Z 仍 `Active`；返回值恰是 `[X, Y]`（升序）。
- `a_removed_source_stays_traceable_but_hides_its_claims`（R4）：`load_sources([A])` 仍返回 A，`removed_at == Some(occurred_at)`；`retrieve_claims` 默认不返回 X／Y；同一 query 加 `include_terminal = true` 后 X／Y 仍读回，且它们的 `sources()` 含 A（已删 source 在终态 claim 上仍可追溯，§6.2 第 5 条）。
- `remove_is_not_idempotent_by_silence`（R34）：第二次 `remove_source(A)` ⇒ `SourceAlreadyRemoved`。
- `remove_rejects_an_unknown_source`（R33）：未知 id ⇒ `UnknownSource`。
- `remove_stales_the_summaries_of_the_affected_pairs`（R9 的另一半）：在 `(Project, Some("p1"))` 与 `(Personal, None)` 各有一条 fresh 摘要；A 属于 p1 的 claim；删 A 后 p1 摘要 `stale = 1`，Personal 仍 `stale = 0`；同一 pair 被多条 claim 命中时只改一次（`mark_summaries_stale` 的 `stale = 0` 条件使其幂等）。
- `remove_does_not_touch_terminal_claims`：claim X 已 Superseded（先 apply 一个带 supersede 的 job）再删它的 source ⇒ X 仍 `Superseded`（不是 `Invalidated`），返回值不含 X。
- `remove_returns_only_the_claims_it_invalidated`：混合 Active／Superseded 引用同一 source ⇒ 返回值只有 Active 的那几条。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-memory --test remove_source
```

**预期**：编译失败（`remove_source` 未定义）。

- [ ] **Step 3: 实现**

`remove_source` 放 `claim.rs`（它在 source 与 claim 之间动状态；放 `source.rs` 会让该文件引用 `MemoryClaimId`，违反 S3）。顺序：`load_source` ⇒ `None` ⇒ `UnknownSource`；`removed_at.is_some()` ⇒ `SourceAlreadyRemoved`；`UPDATE memory_source SET removed_at = ? WHERE source_id = ? AND removed_at IS NULL`（受影响行数必须 1，否则 `SourceAlreadyRemoved`）。然后：

1. `SELECT claim_id FROM memory_claim_source JOIN memory_claim USING (claim_id) WHERE source_id = ? AND validity = 'active' ORDER BY claim_id ASC`；对每条记下 `(claim_id, scope, anchor)`（`scope` 从 claim 行读，anchor 用 `claim_anchor`）。
2. 逐条 `UPDATE memory_claim SET validity = 'invalidated' WHERE claim_id = ? AND validity = 'active'`；受影响行数必须 1，否则跳过（并发下已被别的路径改成终态）。
3. 对去重后的 `(scope, anchor)` 调 `mark_summaries_stale`。
4. 返回被改动的 id（升序）。

**不 `commit`**。`load_sources` **不**过滤墓碑（设计 §4.4 的追溯面）；`retrieve_claims` 默认只返回 Active，自然看不到失效 claim。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-memory --test remove_source
```

**预期**：7 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): 删除 Source 的软删除与派生 claim 无条件失效"
```

---

### Task 10: `correct`（§7.2 的九步）与 `load_correction`

**Files:**
- Create: `crates/continuum-memory/src/correction.rs`
- Modify: `crates/continuum-memory/src/error.rs`（加 `WrongCorrectionSourceKind`）
- Modify: `crates/continuum-memory/src/lib.rs`
- Create: `crates/continuum-memory/tests/correction.rs`

**Interfaces:**
- Consumes: Task 3 的 `insert_source`；Task 4 的 `load_claim`／`claim_anchor`；Task 6 的 `mark_summaries_stale`；Task 7 的 `retrieve_claims`；Task 8 的 `apply_synthesis`（测试夹具与 R26 的下一次合成）
- Produces:
  - `Correction { pub claim: MemoryClaimId, pub statement: String, pub source: MemorySource, pub occurred_at: i64 }`
  - `CorrectionOutcome { pub correction_source: MemorySourceId, pub superseded: MemoryClaimId, pub staled_summaries: usize }`
  - `CorrectionRecord`（`#[derive(Debug, Clone, PartialEq, Eq)]`）`{ pub correction_source: MemorySourceId, pub claim: MemoryClaimId, pub statement: String, pub occurred_at: i64 }`（整结构体 `assert_eq!` 需要 `Debug`＋`PartialEq`）
  - `pub fn correct(tx, correction: &Correction) -> Result<CorrectionOutcome, MemoryError>`
  - `pub fn load_correction(tx, claim: &MemoryClaimId) -> Result<Option<CorrectionRecord>, MemoryError>`

**本 task 的拒绝点 → 臂 → 照片（设计 §7.2 的步骤即顺序）：**

| # | 步骤 | 臂 | 照片 |
|---|---|---|---|
| 1 | 读 claim；不存在 | `UnknownClaim` | R29 |
| 2 | `validity == Active`；否则 | `ClaimNotActive { validity }` | R6 |
| 3 | `source.kind == ExplicitUserCorrections`；否则 | `WrongCorrectionSourceKind { kind }` | R7 |
| 4 | `correction.statement` 去空白后非空；否则 | `EmptyStatement` | R29 |
| 5 | 插 `memory_source`；id 已存在 | `DuplicateSourceId` | R30 |
| 6 | 插 `memory_correction`（`claim_id` UNIQUE） | `Persist`（数据库约束） | R8（直接 SQL） |
| 7 | `UPDATE ... WHERE validity = 'active'` 受影响行数 1 | `ClaimNotActive { validity }` | R5 |
| 8 | 该 scope/anchor 的 summary 置 stale | — | R9 |
| 9 | 返回 `CorrectionOutcome` | — | R5、R26 |

- [ ] **Step 1: 写失败用例**

`tests/correction.rs`：

- `a_correction_writes_a_new_source_and_supersedes_the_old_claim`（R5）：先用 apply 建 claim X（`Project, p1`，两条 source）；`correct(X, "新说法", correction source, t)` ⇒ 新 source 行在；X 的 `validity == Superseded`；X 的 `sources()` **逐项不变**；claim 总数不变；`memory_correction.statement == "新说法"`（原样，含调用方给的首尾空白也原样）。
- `a_second_correction_is_rejected`（R6）：对已 Superseded 的 X 再 `correct` ⇒ `ClaimNotActive { validity: Superseded }`。
- `a_correction_needs_the_correction_source_kind`（R7）：`source.kind = Files` ⇒ `WrongCorrectionSourceKind { kind: Files }`；没有任何行被写。
- `the_database_keeps_one_correction_per_claim`（R8）：第一次 `correct` 并 commit；直接 SQL 先插一条新 `memory_source`，再对同 `claim_id` 插第二条 `memory_correction` ⇒ 数据库约束错（`Persist`），**不经 API**；原 correction 行不变。
- `a_correction_stales_only_its_scope_summary`（R9）：`(Project, Some("p1"))` 与 `(Personal, None)` 各有 fresh 摘要；纠正 p1 的 claim ⇒ p1 的摘要 stale、Personal 的仍 fresh；`CorrectionOutcome.staled_summaries == 1`。
- `a_correction_does_not_create_a_claim`（R24）：纠正后 claim 表行数不变；新说法**不在** claim 表；`retrieve_claims` 默认看不到旧 claim（Superseded），也看不到新说法。
- `load_correction_returns_the_new_statement`（R26）：`load_correction(X) == Some(CorrectionRecord { correction_source, claim: X, statement, occurred_at })`；未知 claim 返回 `None`。
- `an_unknown_claim_is_rejected`（R29）：不存在 ⇒ `UnknownClaim`。
- `an_empty_statement_is_rejected`（R29）：`"   "` ⇒ `EmptyStatement`；claim 仍 Active、无 correction 行。
- `a_duplicate_correction_source_is_rejected_without_side_effects`（R30）：source id 已存在 ⇒ `DuplicateSourceId`；claim 仍 Active、无 correction 行。
- `the_correction_statement_can_be_proposed_in_the_next_synthesis`（R26 的下一半）：`correct` 后用 `load_correction` 的 `statement` 与 `correction_source` 组一个 `ProposedClaim`（新 id），`begin_synthesis`＋`apply_synthesis` ⇒ 新 claim 的 `statement` 等于纠正文本（trim 后）、`sources` 含 correction source。
- `a_correction_does_not_append_to_the_old_claim_sources`（§7.3）：纠正前后 X 的 `sources()` 逐项相等；correction source 只出现在 `memory_correction` 与下一次合成的新 claim 上。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-memory --test correction
```

**预期**：编译失败（`correct`／`Correction` 未定义）。

- [ ] **Step 3: 实现**

严格按 §7.2 的九步顺序；每步先判再写。第 1 步 `load_claim`；第 2 步读 `validity()`；第 3 步读 `correction.source.kind()`；第 4 步 `trim()`；第 5 步调 `insert_source(tx, &correction.source)`（它已做 `DuplicateSourceId` 预检）；第 6 步 `INSERT INTO memory_correction (source_id, claim_id, statement, occurred_at) VALUES (?,?,?,?)`；第 7 步 `UPDATE memory_claim SET validity = 'superseded' WHERE claim_id = ? AND validity = 'active'`，`execute` 返回 1 才继续，否则 `ClaimNotActive { validity: <重新读到的值> }`；第 8 步用第 1 步读到的 scope 与 `claim_anchor` 调 `mark_summaries_stale`；第 9 步返回 outcome。

`load_correction`：`SELECT source_id, claim_id, statement, occurred_at FROM memory_correction WHERE claim_id = ?`；`None` ⇒ `Ok(None)`。**不**把 correction 写进旧 claim 的 `sources[]`（§7.3）。**不**改旧 claim 的 `statement`。**不**创建新 claim。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-memory --test correction
```

**预期**：12 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "feat(memory): 纠正生成新 Correction Source 并使旧 claim SUPERSEDED"
```

---

### Task 11: 类型层与源码面照片（trybuild＋grep 断言）

**Files:**
- Create: `crates/continuum-memory/tests/type_level.rs`
- Create: `crates/continuum-memory/tests/source_face.rs`
- Create: `crates/continuum-memory/tests/compile_fail/{claim_has_no_status_or_anchor_accessor,claim_constructor_is_crate_private,memory_scope_has_no_session,proposed_claim_cannot_express_validity,proposed_claim_cannot_express_last_confirmed,memory_source_fields_are_private,confidence_has_no_order}.rs`
- Create: 同名七份 `.stderr`（**逐份取自实跑，不许凭记忆写**）
- Modify: `crates/continuum-memory/Cargo.toml`（dev-dep `trybuild`；**插回已有的 `[dev-dependencies]` 段内**，与 `tempfile` 同段，不得追加到 Task 4 的 `[features]` 段之后）
- Modify: `crates/continuum-memory/src/lib.rs`（只在需要时补导出，本 task 通常不动）

**Interfaces:**
- Consumes: Task 1–10 的全部公开类型
- Produces: 编译期不可表达性与源码面断言；不改生产代码的行为。

**C4 为什么拆成两份：** 设计 §10.2 的 C4 写「`ProposedClaim { validity: .. }` **或** `{ last_confirmed: .. }`」；本计划落成两份样例，因为它们是两条独立的「提案不能表达终态／时间」通道（同 P5a 把「字段私有」与「无 `Default`」拆成两份的先例）。C1–C7 的编号仍按设计，C4 记作 C4a／C4b。

- [ ] **Step 1: 写七份样例、`.stderr` 与两个驱动**

`crates/continuum-memory/Cargo.toml`：把 `trybuild = { workspace = true }` 加进**已有的 `[dev-dependencies]` 段**（与 `tempfile` 同段；按 `continuum-verify` 的写法在 `tempfile` 之后、同一段内）。**不要**新建第二个 `[dev-dependencies]`，也**不要**把它追加到 Task 4 的 `[features]` 段之后——`[features]` 只在 Task 4 建一次，`test-support = []` 留在原处。

七份样例（每个文件名即它钉的通道）：

1. `claim_has_no_status_or_anchor_accessor`（C1）：`fn f(c: &continuum_memory::MemoryClaim) { let _ = c.status(); let _ = c.anchor(); }` ⇒ 编不过（E0599）；钉 §5.1 的七字段之外没有第八个可读字段。
2. `claim_constructor_is_crate_private`（C2）：`continuum_memory::MemoryClaim::new(todo!(), todo!(), todo!(), todo!(), todo!(), 0)` ⇒ 编不过（E0624／私有）；钉 §5.5 的生产唯一产生点（测试面例外见 Task 4 的 `for_test`）。**C2 口径不因 `test-support` 改变**：`for_test` 是 Task 4 的 `#[cfg(feature = "test-support")]` 另一个入口；本样例只调 `new`，默认与 `--all-features` 下都仍是 E0624。
3. `memory_scope_has_no_session`（C3）：`continuum_memory::MemoryScope::Session` ⇒ 编不过（E0599）；钉 §5.2 的五个 scope。
4. `proposed_claim_cannot_express_validity`（C4a）：`ProposedClaim { id: todo!(), statement: String::new(), sources: vec![], confidence: todo!(), supersedes: vec![], validity: todo!() }` ⇒ 编不过（E0560）；钉提案不能写 `validity`。
5. `proposed_claim_cannot_express_last_confirmed`（C4b）：同上但多 `last_confirmed: 0` ⇒ 编不过（E0560）；钉提案不能写时间。
6. `memory_source_fields_are_private`（C5）：`MemorySource { id: todo!(), kind: SourceKind::Files, external_ref: String::new(), observed_at: 0, removed_at: None }` ⇒ 编不过（E0451）；钉 §4.2 的唯一构造点。
7. `confidence_has_no_order`（C6）：`fn f(a: Confidence, b: Confidence) { let _ = a < b; }` ⇒ 编不过（E0369）；钉 §5.3.2 没有 `PartialOrd`。

`tests/type_level.rs`：**照 `crates/continuum-verify/tests/type_level.rs` 的形状**，含 `t.compile_fail("tests/compile_fail/*.rs")`、手写 `LISTED_SAMPLES: [&str; 7]`、`the_sample_list_matches_the_directory`（清单 ↔ 目录 ↔ 文件头点名三处对钉），以及正控制 `c7_the_five_scopes_and_nine_kinds_compile`：遍历 `MemoryScope::ALL`（5），每臂读一次 `as_str()`；遍历 `SourceKind::ALL`（9），每臂 `let source = MemorySource::new(MemorySourceId::new("s"), kind, "r", 0).unwrap();` 并读回 `source.kind()`；`Validity::ALL` 三臂各读一次 `as_str()`；构造 `RetrievalQuery::new(vec![MemoryScope::Personal], None, None, 0, false).unwrap()`；调 `Confidence::from_value(serde_json::json!(null))`。**C7 是防止 C1–C6 是「整块编不过」的假照片。**

`tests/source_face.rs`：

- `source_module_does_not_reference_claims`（S3）：读 `src/source.rs`，断言不含 `crate::claim`、`MemoryClaim`、`Validity`。
- `no_event_or_audit_symbols`（S4）：读 `src/` 下九个 `.rs` 文件（`lib/error/persist/source/claim/summary/correction/synthesis/retrieval`），断言 `EventType`／`AuditKind`／`append_event`／`append_audit` 零命中。
- `enum_arity_literals_are_pinned`（S6）：`src/claim.rs` 含 `ALL: [MemoryScope; 5]` 恰一次；`src/source.rs` 含 `ALL: [SourceKind; 9]` 恰一次；运行期再断 `MemoryScope::ALL.len() == 5`、`SourceKind::ALL.len() == 9`。
- `validity_encodes_superseded_verbatim`（S8）：`Validity::Superseded.as_str() == "superseded"`；三臂往返。
- `cross_cutting_vocabulary_is_not_reused`（§13.3）：读 `src/` 下九个 `.rs` 文件，逐行去掉 `//` 起的注释后拼成一份源码语料（文档注释里的中文「来源」因此不参与）；先把语料按非标识符字符（`[A-Za-z0-9_]` 之外）切成 token，再断言三个源码 token 零命中：`provenance`（作为标识符子串，**大小写不敏感**）、`trust`（作为标识符子串，**大小写不敏感**；对每个 token 先整词豁免 `distrust`／`Distrust`，故 `TrustKind`／`NodeTrust`／`trust_level` 都命中，`distrust` 不算）、裸 `Scope`（前后都不是标识符字符，故 `MemoryScope`／`EvidenceScope` 不匹配）。这三个词分别对应设计 §13.3 的「不用 `provenance` 指来源」「不用 `trust` 指来源种类」「不用裸 `Scope`」；中文散文不在检查范围内。
- `claim_and_source_field_lists_are_exact`（S11）：取 `src/claim.rs` 里 `struct MemoryClaim {` 到其后第一个列 0 的 `}` 之间的块，以及 `src/source.rs` 里 `struct MemorySource {` 的同类块；用 `^\s*([a-z_][a-z0-9_]*)\s*:` 匹配字段行，断言匹配行数**恰为 7／5**，且收集到的字段名**集合**等于白名单集合（`MemoryClaim`：`id`／`scope`／`statement`／`sources`／`validity`／`confidence`／`last_confirmed`；`MemorySource`：`id`／`kind`／`external_ref`／`observed_at`／`removed_at`）——**只比集合、不比顺序**（设计 §10.3 只要求「恰为七个／五个」；字段顺序不在本照片的射程内），且不含 `status:`／`anchor:`。`created_at:`／`scope_ref:`／`extra:` 这类第八／第六个私有字段会同时让行数断言与白名单断言红。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test type_level
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test source_face
```

**预期**：`type_level` 以「缺 `.stderr`／产出 `wip/*.stderr`」失败；`source_face` 逐条通过（它断言的是已实现的源码面），若有失败则说明前序 task 的形状已漂。

- [ ] **Step 3: 接收 `.stderr` 并写完驱动**

逐份把 `wip/*.stderr` 搬进 `tests/compile_fail/`；逐份人工确认「因为预期的那条错而编不过」，不是拼写错。`LISTED_SAMPLES` 与文件头点名同时加到七份。**`.stderr` 一旦接受，就不再改那份 `.rs` 的注释**（trybuild 按文件主干配对且不规范化行号；改文件头会推移坐标，要改就重跑并一起提交）。

- [ ] **Step 4: 运行，确认通过**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test type_level
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test source_face
```

**预期**：`type_level` 的 `type_level_guarantees_hold`、`the_sample_list_matches_the_directory`、`c7_...` 全 PASS；`source_face` 6 条全 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory Cargo.lock
git commit -m "test(memory): 七份类型层照片与源码面断言"
```

---

### Task 12: 装配与收尾（共写第 2、4、5 处＋两处断言同步）

**Files:**
- Modify: `crates/continuum-runtime/Cargo.toml`（加 `continuum-memory` path 依赖）
- Modify: `crates/continuum-runtime/src/main.rs`（`runtime_migrations()` 追加一行）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`continuum-runtime` 行加 `"continuum-memory"`）
- Modify: `crates/continuum-runtime/tests/migrations.rs`（`expected_migrations()` 追加一行）
- Modify: `crates/continuum-runtime/tests/startup.rs`（三处计数与注释按实跑更新）
- Create: `crates/continuum-memory/tests/wiring.rs`

**Interfaces:**
- Consumes: Task 2 的 `p6c_memory_migrations`
- Produces: 装配后的 runtime；S1／S2／S10 的照片。

**本 task 的拒绝点：** 无逻辑臂；装配错的表现是既有门变红（`ALLOWED` 双向断言、`expected_migrations()` 集合比对、`startup.rs` 计数）。

- [ ] **Step 1: 写 wiring 用例（失败）**

`tests/wiring.rs`（用 `std::fs::read_to_string` 读仓库文件，路径以 `env!("CARGO_MANIFEST_DIR")` 为锚）：

- `workspace_manifest_lists_the_crate_once`（S1）：根 `Cargo.toml` 里 `"crates/continuum-memory"` 恰一次。
- `runtime_manifest_depends_on_memory_once`（S1）：`crates/continuum-runtime/Cargo.toml` 里 `continuum-memory = { path = "../continuum-memory" }` 恰一次。
- `allowed_table_has_the_memory_row_and_the_runtime_entry`（S2）：`dependency_direction.rs` 里 `("continuum-memory", &["continuum-persist"])` 恰一次；`"continuum-memory",` 在文件内恰出现**两次**（新 crate 行与 runtime 行各一次），且 `continuum-runtime` 数组块里恰一次。
- `runtime_registers_memory_migrations_once`（S10）：`main.rs` 里 `p6c_memory_migrations()` 恰一次。
- `memory_crate_has_only_the_persist_internal_edge`（S2）：读 `crates/continuum-memory/Cargo.toml`，**按 `[dependencies]` 段切片**（到下一个以 `[` 开头的段头为止），段内 `continuum-` 开头的条目只有 `continuum-persist`。**`[features] test-support = []` 是空 feature，不属于依赖面**：它不在 `[dependencies]` 段内，不计入；该断言不会因 Task 4 的 feature 行误红。

**S1／S2 的其余断言不受 feature 影响**：`workspace_manifest_lists_the_crate_once` 只数根 manifest 的 members 路径，`runtime_manifest_depends_on_memory_once` 只读 runtime manifest，两者都不数 `[features]` 行。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test wiring
```

**预期**：三条失败（`runtime_manifest_depends_on_memory_once`、`allowed_table_has_the_memory_row_and_the_runtime_entry`、`runtime_registers_memory_migrations_once`——runtime 还没依赖／`ALLOWED` 还没 runtime 项／还没注册）；`workspace_manifest_lists_the_crate_once`（Task 1 已加 members）与 `memory_crate_has_only_the_persist_internal_edge`（Task 1 的 manifest 已只有 persist）通过。

- [ ] **Step 3: 装配**

1. `crates/continuum-runtime/Cargo.toml` 加 `continuum-memory = { path = "../continuum-memory" }`（按字母序放在 `continuum-graph` 与 `continuum-model-registry` 之间）。
2. `main.rs` 的 `runtime_migrations()` 在 `p5a_verify_migrations()` 之后加：
   ```rust
   migrations.extend(continuum_memory::p6c_memory_migrations());   // 190：记忆八表
   ```
   并把该函数上方文档注释的「P5a 一条」段后补一句「P6C 一条（190 记忆八表）；190 的 SQL 此后不得再改」。
3. `dependency_direction.rs` 的 `continuum-runtime` 数组加 `"continuum-memory",`（`memory` < `model`，字母序在 `continuum-graph` 与 `continuum-model-registry` 之间）。
4. `migrations.rs` 的 `expected_migrations()` 加 `m.extend(continuum_memory::p6c_memory_migrations()); // 190：记忆八表`；文件头注释加 P6C 一句。
5. `startup.rs`：`:25` 与 `:92` 的 `"迁移应用 13 项"` 改为 `"迁移应用 14 项"`（P0 2＋P1 2＋P2 3＋P3 2＋P4 3＋P5a 1＋P6C 1）；`:80` 的 `"迁移应用 11 项"` 改为 `"迁移应用 12 项"`；三处注释补「P6C 一条」。**以实跑输出为准**：若输出不是 14／12，先核 `runtime_migrations()` 与 `expected_migrations()`，再按实际输出改断言并报告。

- [ ] **Step 4: 运行，确认通过（含全量门）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-memory --test wiring
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test migrations
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test startup
TMPDIR="$PWD/.tmp" timeout 2400 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 2400 cargo build --workspace --all-targets
```

**预期**：全部 PASS；`cargo test` 与 `cargo build` 输出 **0 warning**。`dependency_direction` 的两条用例（`:478` 起的双向断言）全绿；`migrations` 的集合比对与 `startup` 的三处计数全绿。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-memory crates/continuum-runtime/Cargo.toml crates/continuum-runtime/src/main.rs crates/continuum-runtime/tests/dependency_direction.rs crates/continuum-runtime/tests/migrations.rs crates/continuum-runtime/tests/startup.rs Cargo.lock
git commit -m "feat(runtime): 装配 continuum-memory 与迁移 190"
```

---

## 遗留与待协调

**本计划不改设计；以下是实现期必须让协调者／设计作者知道的事。**

1. **设计 §3.3 的共写表漏了两处测试断言（一处集合＋三处计数）。** 实读确认：`crates/continuum-runtime/tests/migrations.rs:35` 的 `expected_migrations()` 若不加 `p6c_memory_migrations()`，同文件 `:121` 起的 `the_runtime_applies_exactly_the_expected_migration_set` 会因实际集合多出 190 而红；`crates/continuum-runtime/tests/startup.rs:25` 与 `:92` 的 `"迁移应用 13 项"`、`:80` 的 `"迁移应用 11 项"` 在 190 落地后实际为 14／12，三处都会红。设计 §3.3 只列五行，且写「除这五处之外不碰任何既有 crate 的源码」，与「必须改这两个测试文件」相抵。本计划在 Task 12 保留这两处同步（`migrations.rs` 加一行、`startup.rs` 三处改数），并记作第五行的断言后果，不记作新的源码共写面；**请协调者裁定**：把这两行补进设计 §3.3 的共写表，或在表中明写「测试断言同步不计入源码共写面」。**收件人：协调者。影响：不裁定则设计表与可执行计划相抵，Task 12 无法在「五处之外不碰」的口径下收口。**
2. **`remove_source` 的模块落点设计没有指定。** 设计 §3.4 的模块图只有 `source.rs`／`claim.rs`／`correction.rs`／`synthesis.rs`／`retrieval.rs`，而 `remove_source` 返回 `Vec<MemoryClaimId>`，放 `source.rs` 会违反 S3「source.rs 不得引用 claim 侧类型」。本计划放 `claim.rs`（它本来就 use source），并在文件头写明理由。**收件人：设计作者（若要另立 `lifecycle.rs`，改的是文件归属，不是语义）。**
3. **`MemoryError` 的载荷形状设计只给了两枚。** 设计 §3.5 列臂名，只有 `ClaimNotActive { validity }` 与 `WrongCorrectionSourceKind { kind }` 带载荷；本计划其余臂一律无载荷（`UnknownSource`／`DuplicateSourceId` 等不携带 id）。测试按这个形状 match。**收件人：设计作者（若要在错误里带 id，改的是 `error.rs` 与各 match）。**
4. **`load_sources` 的读回形状设计未给。** 本计划定：只返回存在的 id、按 `source_id` 升序、输入重复折叠、缺失静默省略；`load_sources` **不**排除软删除的 source（设计 §4.4 只说了「不排除」，没说顺序与缺失）。**收件人：P6A／设计作者（若 P6A 要按请求顺序返回，需要改这一条）。**
5. **`remove_source` 的返回集合设计有歧义。** 「返回这些 claim 的 id」在本计划读作「本次从 Active 改成 Invalidated 的那些」，按 id 升序；已经终态、只是引用了该 source 的 claim 不返回。**收件人：设计作者。**
6. **`trigger_ref` 的 JSON 形状设计给了值映射、没给键名。** 设计 §8.2（`:937-941`）已给出 `trigger_ref` 的四个 payload 值（session id／`since`／changed ids／`null`），但没有给 JSON 键名；本计划钉死为 `{"session":...}`／`{"since":...}`／`{"changed":[...]}`／`null`。`SynthesisTrigger` 不提供 `parse`，读侧只看 `trigger_kind`／`trigger_ref`。**收件人：P7／设计作者（P7 若要与自己的调度记录对表，用这四个形状）。**
7. **设计 §12.1 的十七条全部仍开，本计划按设计的处置实现、一条都不代裁。** 其中**会改本计划代码**的是：第 1 条（`Session` 是否第六个 scope）、第 2 条（`MemoryClaim` 是否加第八个 anchor 字段）、第 3 条（`SourceKind` 九臂的并集裁定）、第 4 条（`validity` 是否变区间／`confidence` 是否留历史）、第 8 条（correction source 是否挂旧 claim 的 `sources[]`）、第 16 条（TTL／decay／`Archived`）、第 17 条（纠正文本的规范承载物）。其余十条是不建、上游缺口或零事件零审计的读法，**不改变本计划的函数形状**。**收件人：规范维护者＋协调者。**
8. **`Some("")` 的归一规则只覆盖 `text`。** 设计 §3.5／§6.1 明确 `text = Some("")` 归一为 `None`；对 `anchor = Some("")` 没有规定。本计划**不**归一 anchor（它是 opaque 字符串，`Some("")` 是一个「有锚点但值为空」的调用方错误），`validate_scope_anchor` 只判 `is_some`。**收件人：设计作者（若要把空 anchor 当缺失，改 `validate_scope_anchor` 一处）。**
9. **P6A 到不了 source 的第二段。** 设计 §3.5 已申报：`compile` 只收 `&dyn MemoryRetrieval`（无 `Tx`），`load_sources` 收 `&Tx`；本计划不为此改 trait（改 trait 会让 P6A 的接口与 P6C 的依赖方向一起动）。**收件人：P6A＋协调者。**
10. **迁移号落地前必须重跑核对。** 设计 §3.3 的实测是 2026-10-17；`grep -rn "Migration::new(" crates/` 必须在本块 Task 2 开工时再跑一次，`190` 若已被占则按切分 §四第 2 条现场顺取，并同步改本计划与设计引用。**收件人：实现者＋协调者。**
11. **P6A 的 claim 测试构造面：已按 `test-support` feature 对齐（fix round 3）。** P6A 计划的 Task 4–7 需要 crate 外可用的 claim 夹具。本计划在 Task 4 交付同一契约：
    - `crates/continuum-memory/Cargo.toml` 增 `[features] test-support = []`（空 feature，只开可见性，不增依赖）；
    - `#[cfg(feature = "test-support")] #[doc(hidden)] pub fn for_test(id: MemoryClaimId, scope: MemoryScope, statement: impl Into<String>, sources: Vec<MemorySourceId>, validity: Validity, confidence: Confidence, last_confirmed: i64) -> MemoryClaim`（`impl MemoryClaim` 内；七字段；不校验、不归一，与 `rehydrate` 同形；默认构建下不存在）；
    - `tests/claim.rs` 一条 `#[cfg(feature = "test-support")]` 用例，`cargo test -p continuum-memory --test claim --features test-support` 时跑。
    **C2 口径不需要改**：C2 测的是 `MemoryClaim::new` 仍是 `pub(crate)`（默认与 `--all-features` 下都编不过 E0624）；`for_test` 是另一个入口，不会让 `new` 变 `pub`。S11 只数 `struct MemoryClaim` 的七个字段，也不受方法影响。
    **与 P6A 的联动**：P6A 计划在 `crates/continuum-context/Cargo.toml` 的 `[dev-dependencies]` 以 `continuum-memory = { path = "../continuum-memory", features = ["test-support"] }` 打开它；**P6A 的 Task 4–7 claim 用例在该 feature 落地前挂起**（其 Step 3b 预检 `test-support`／`for_test`／`#[doc(hidden)]` 三处，任一未命中就整步跳过，Task 10 不得报全绿）。**收件人：P6A（执行）＋协调者（知会）。**

---

## 本块明确不拍（照设计 §10.4 的十条）

以下十条**不是漏拍**：本块没有对应操作面或判据，照片不在此处；实现者不得为它们造测试，也不得因它们改变本块的函数形状。

1. **「用户」是用户**（§7.5）：类型只能要求 `SourceKind::ExplicitUserCorrections`，不能证明来源是人。
2. **调度的时机**（§8.6）：时机判给 P7；本块没有「空闲多久算空闲」的照片。
3. **合成的质量**：同义改写去重、矛盾检测、重要性估计都没有判据；本块只拍 proposal 的形状与不变量。
4. **§172 的「只保存长期稳定事实」**：applier 只判来源非空、scope 合法；「长期稳定」是 proposer 的政策。
5. **物理擦除／forget**（§4.4）：软删除；本块没有「数据真的不存在了」的照片。
6. **Personal Memory 的放置**（§182）：不在本块；`scope = personal` 的行落在哪个节点不由本块决定。
7. **「删除 claim 不影响 Source」**（§4.3）：本块没有删除 claim 的路径，这句话没有操作面。
8. **崩溃后孤立的 `Running` 作业**（§8.1）：本块不自动恢复；恢复的照片在 P7／P0。
9. **事件流上的记忆变更**（§9.5）：零事件；没有「从事件重放记忆」的照片。
10. **`Persist(PersistError)` 的透传**：不是逻辑拒绝臂；不列入 §10.1 的拒绝点表。

---

## 自审

### 1. Spec coverage（设计 §1–§13 每节都有 task）

| 设计节 | 落点 |
|---|---|
| §1 范围／不建什么 | Global Constraints＋Task 1–12 的边界；不建项见「遗留」第 7 条 |
| §2 规范依据与自定形状 | Global Constraints（七字段、五 scope、锚点、软删除、零事件）＋各 task 的拒绝点表 |
| §3 crate／依赖边／共写／迁移号 | Task 1（crate＋members＋ALLOWED 新行）、Task 12（runtime 三处＋两处断言同步） |
| §4 Source 图 | Task 1（类型）、Task 3（存储）、Task 9（软删除） |
| §5 `MemoryClaim` 与 scope | Task 4（类型＋`validate_scope_anchor`＋`test-support` 的 `for_test`）、Task 8（唯一产生点）、Task 9／10（状态迁移） |
| §6 检索面 | Task 7（`RetrievalQuery`／`MemoryRetrieval`／`MemoryStore`／`retrieve_claims`） |
| §7 Correction | Task 10（九步＋`load_correction`＋R26） |
| §8 Synthesis | Task 5（begin／fail／G0–G6）、Task 8（apply／A1–A5＋I1–I7） |
| §9 落库 | Task 2（八表六索引）、Task 3–10 的写路径与读回 |
| §10 判据与照片 | 每个 task 的 R 编号；Task 2 的 S5／S7／S9；Task 11 的 C1–C7＋S3／S4／S6／S8／S11＋§13.3 词汇表；Task 12 的 S1／S2／S10；§10.4 的十条不拍项见 `## 本块明确不拍` |
| §11 明确不做 | Global Constraints＋「遗留」第 7 条 |
| §12 遗留与对账 | `## 遗留与待协调` 第 1–11 条 |
| §13 横切约束 | Task 1（不新增反向边）、Task 2（号段）、Task 11（`cross_cutting_vocabulary_is_not_reused`／零事件）、Task 12（并集观察） |

**照片总账**：R1（T2）、R2（T3）、R3／R4（T9）、R5–R9（T10）、R10–R19b（T8）、R20／R21／R25／R27（T7）、R22（T4）、R23（T1）、R24／R26（T10）、R28（T6）、R29／R30（T10）、R31（T3＋T5＋T8）、R32（T5＋T6＋T7）、R33（T3＋T5＋T8＋T9）、R34（T9）、R35（T5＋T8）、R36／R37（T8）、R38（T5）、R39（T5＋T8）、R40／R41（T8）、R42／R43（T5）。**R1–R43 无遗漏。** C1–C6 在 T11（C4 拆 C4a／C4b），C7 正控制在 T11；§13.3 的 `provenance`／`trust`／裸 `Scope` grep 在 T11 的 `cross_cutting_vocabulary_is_not_reused`。§10.4 的十条是**明确不拍**，见 `## 本块明确不拍`。

### 2. Step scan

- 每个 task 都是「写失败用例 → 跑失败 → 实现 → 跑通过 → 提交」五步；Task 4 额外多一个 `test-support`／`for_test` 的 feature-gated 步骤（Step 4，同样先写用例、确认编译失败再实现），共六步。每步一个动作，没有 TBD／「适当校验」／「写上述测试」。
- 每个实现步骤给的是**签名＋拒绝点顺序＋关键 SQL 谓词**；算法（apply 的执行顺序、remove 的三件事、correct 的九步）写成有序清单，不是整段可粘贴代码。
- 测试步骤给的是**用例名＋断言值**；红条件档位只在 T1 的 `source_kind_has_the_nine_arms_of_170_and_282`／`source_kind_parse_does_not_default`／`memory_source_constructor_rejects_a_blank_external_ref` 与 T4 的 `memory_scope_parse_does_not_default` 等少数用例显式标注，多数用例靠断言值本身判红。可执行命令逐条给出，均带 `TMPDIR` 与 `timeout`。
- 没有一步要求实现者「自己决定」设计已定的值：七字段、五 scope、四触发、九臂、六索引、八表、190 全部写死。

### 3. Type consistency

| 名字 | 定义处 | 使用处 | 一致？ |
|---|---|---|---|
| `MemorySourceId::{new,as_str}` | T1 | T3／T5／T8／T9／T10 | 是 |
| `SourceKind` 九臂／`ALL` | T1 | T5（G2／G2b）／T11 | 是 |
| `MemoryClaim` 七访问器 | T4 | T7／T8／T9／T10／P6A | 是 |
| `MemoryScope` 五臂／`ALL` | T4 | T5／T6／T7／T8／T9／T10 | 是 |
| `Validity` 三臂 | T4 | T8／T9／T10 | 是 |
| `Confidence::{from_value,as_value}` | T4 | T8／T11 | 是 |
| `validate_scope_anchor(&[MemoryScope], Option<&str>)` | T4 | T5／T6／T7 | 是 |
| `load_claim`／`claim_anchor`（crate 内） | T4 | T7／T8／T9／T10 | 是 |
| `MemorySummary` 七字段 | T6 | T8／T10（P6A 不消费 summary，设计 §3.5） | 是 |
| `mark_summaries_stale`／`write_summary`（crate 内） | T6 | T8／T9／T10 | 是 |
| `SynthesisJobId`／`SynthesisStatus`／`SynthesisTrigger` | T5 | T6／T8／T9／T10 | 是 |
| `RetrievalQuery::new(scopes, anchor, text, limit, include_terminal)` | T7 | T8／T9／T10／P6A | 是 |
| `MemoryRetrieval::retrieve(&RetrievalQuery)` | T7 | P6A | 是 |
| `ProposedClaim`／`SynthesisProposal`／`SynthesisReport` | T8 | T10／P7 | 是 |
| `Correction`／`CorrectionOutcome`／`CorrectionRecord` | T10 | P7／下一次合成 | 是 |

**一处刻意的形状**：`MemoryClaim::new` 是 `pub(crate)`，`rehydrate` 也是 `pub(crate)`；默认构建下 crate 外只能经 `apply_synthesis` 产生 claim。`test-support` 下的 `for_test` 是显式测试例外（不校验，见遗留第 11 条），不会让 `new` 变 `pub`。**没有**第二套 `MemoryClaim`／`MemoryScope`／`MemorySource` 类型。

### 4. Review Focus 归位

| Review Focus | 拥有该代码的 task 的测试 |
|---|---|
| 纠正后的说法丢失 | T10 `a_correction_writes_a_new_source_and_supersedes_the_old_claim`（R5）、`the_correction_statement_can_be_proposed_in_the_next_synthesis`（R26） |
| 旧摘要未失效 | T6 `load_current_summary_skips_stale_rows`（R28）、T8 `no_summary_stales_the_old_one_and_current_summary_is_none`（R19b）、T9 `remove_stales_the_summaries_of_the_affected_pairs`、T10 `a_correction_stales_only_its_scope_summary`（R9） |
| `anchor` 缺失时跨项目混检 | T7 `query_construction_applies_the_anchor_rules`（R27）、`scope_and_anchor_filter`（R20）；T5 `begin_applies_scope_anchor_rules`（R32） |
| 软删除后派生 claim 未失效／误伤 | T9 `removing_a_source_invalidates_every_claim_that_references_it_and_no_other`（R3）、`a_removed_source_stays_traceable_but_hides_its_claims`（R4） |
| 两阶段作业的重复 id／状态错乱 | T5 `begin_rejects_a_duplicate_job_id`（R38）、T8 `a_second_apply_of_the_same_job_is_rejected`（R16）、`duplicate_claim_ids_are_rejected`（R36）、`a_failed_apply_leaves_the_job_running` |

### 5. Proportion

本计划约 1180 行，设计 1541 行；计划／设计 ≈ 0.77，低于 P5a 的 1.3。差异来自：本块类型与 SQL 大多在设计里已逐列给定，计划只补**执行顺序、拒绝点归位、测试名与少量红条件**，不重抄设计正文。代码块以签名、DDL 列清单与 SQL 谓词为主，没有整段可粘贴实现。

---

**执行方式（本计划不自行选择）：** 按 `superpowers:subagent-driven-development` 或 `superpowers:executing-plans` 执行；每个 task 完成后由复审者按该 task 的 Interfaces 与照片逐条核对，再进入下一个 task。
