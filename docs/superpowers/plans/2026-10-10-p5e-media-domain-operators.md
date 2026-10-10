# P5e（媒体领域算子）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建一个 crate `continuum-media`，落三件事：
（1）**17 枚媒体领域算子的 `Operator` 值清单与注册入口**（设计 §5，出处 §10 `:541-545`、§32 `:1366-1386`、§33 `:1403-1411`、§34 `:1424-1427`）；
（2）**`ArtifactType` 的 P5 扩展清单（11 枚新变体）的一次落地**（设计 §3，切分 §四第 2 条的**唯一落地点**）；
（3）**`Checkpointable` 的错误类型**（`CheckpointError` 三臂 ＋ `CheckpointOwner`，定义在 `crates/continuum-operator`，设计 §4）
与它的**第一枚实现**（`generate-broll` / `render` 两枚长算子，设计 §4.5）。
另落三处判定：§5.4 的「逐位可复现 backend」注册期名单、§34 的生成授权判定、§5.3 的端口边表。

**Architecture:** 本块只**注册**算子、只**声明**形状，**不自建执行路径**（切分 §四第 4 条）。
`continuum-media` 的对外面是**数据与判定**：`all_operators()` / `register_media_operators` /
`required_permission` / `authorize_generative` / `bind_media_methods` / 两枚 `Checkpointable` 实现。
**证据一律由宿主的执行代码直接调 P5a 的 `Evidence::from_tool_result` 构造，本块不写域包装**（设计 §9.2，协调者 2026-10-10 裁定）；
据此本块对 `continuum-verify` 的边是 **dev 边**，生产代码对它零调用。
**本块零迁移、零表、零事件**（设计 §11）：17 枚算子在装配期进内存注册表，11 枚新变体落进既有的两列 `TEXT`（无 CHECK 约束），
Timeline 的六字段落进既有的 `artifact.metadata` / `provenance`。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-artifact`（`ArtifactType`、`Artifact`、`ContentHash`、`ArtifactStore`）；
`continuum-operator`（`Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` / `BackendId` /
`OperatorRegistry` / `OperatorError` / `Checkpointable`）；`thiserror`；
dev：`continuum-graph`（`cache_key` / `can_reuse` / `CacheKey`，设计 §12.2 第 (r) 条的照片）、
`continuum-verify`（两条 `trybuild` 的结构照片）、`trybuild`、`tempfile`。

**设计依据：** `docs/superpowers/specs/2026-10-09-p5e-media-domain-operators-design.md`（**唯一事实来源**，1346 行）。
**切分口径：** `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`（§一 共写表、§二 接口冻结、§四 横切约束七条、§五 P5e 行、§八 缺口）。
**本块无独立的裁定文件**——协调者的裁定（2026-10-10 的四块统一口径、生成类算子的 `side_effect_class` 改判等）**以设计正文里的引文为准**。
**本计划不改设计、不改规范、不改代码以外的任何计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-10 实读工作树 `p5e-plan`）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| P1 `continuum_artifact::ArtifactType`（六型 ＋ `ALL` ＋ `as_str` / `parse`） | **已交付**（`crates/continuum-artifact/src/artifact.rs:14-21`、`:28-35`、`:47`、`:68`） | Task 1 把它扩成 17 型 |
| P1 `continuum_artifact` 的 `Artifact` / `ContentHash` / `ArtifactStore` | **已交付**（`src/artifact.rs:170-180` 的 `metadata` / `provenance` 在 `:177` / `:178`；`src/content.rs:10-11` 的 `ContentHash::of`） | Task 1 的 (c)、Task 7 的 `ContentHash` |
| P1 `continuum_port::{compatible, Port}` 与 `tests/compatibility.rs` | **已交付**（`src/port.rs:94` 的 `compatible`，等值判定在 `:101`） | Task 1 的 (e)：改 `tests/compatibility.rs:47` 一处用例 |
| P1 `continuum_operator::{Operator, OperatorId, OperatorVersion, Determinism, SideEffectClass, BackendId, OperatorRegistry, OperatorError}` | **已交付**（`src/definition.rs:79-88` 的七字段、`:9-12`、`:17-21`、`:24-34`、`:44-55`、`:66-76`；`src/registry.rs:7-12`、`:19-48`） | Task 3／4：算子清单与注册 |
| P1 `continuum_operator::Checkpointable` | **已交付但空转**：`src/definition.rs:92-96`，两处返回 `Result<_, String>`；**无实现、无调用点、无测试**（实测 `grep -rn "Checkpointable" crates/` 只命中 `definition.rs:92` 一处） | Task 2 改签名与新类型；Task 6 交第一枚实现 |
| P1 `continuum_graph::{cache_key, can_reuse, CacheKey}` | **已交付**（`src/reuse.rs:9-12`、`:16-24`、`:29-47`，八条用例在 `tests/reuse.rs`） | Task 7 的照片（**dev 边**）；本块**不建第二份复用判据**（切分 §四第 7 条） |
| P1 `continuum_graph::{OperatorRef, OperatorImpl}` | **已交付**（`src/node.rs:42-45`、`src/execution.rs:81` 的返回类型 `Vec<Artifact>`） | 只在遗留里出现：`CheckpointOwner` 与 `OperatorRef` 同形**不合并**（设计 §4.3） |
| **P5a `continuum-verify`** | **设计已定稿；crate 不存在**（实测 `ls crates/` 无该目录；`grep -rln "continuum-verify" --include="*.toml"` 零命中） | Task 9 的两条 `trybuild`；Task 9 才登记那条 **dev 边** |
| **P5b `continuum-method`** | **设计已定稿；crate 不存在**（同上，`--include="*.toml"` 零命中） | Task 8 的 `bind_media_methods`；Task 8 才登记那条**普通边** |
| P4 `continuum-semantics` 的 `RequirementId` | **未交付**（crate 不存在） | 本块**不用它**——产出的证据一律 `Unattached`（设计 §9.2）；Task 9 第 2 条的照片挂在它的归属上（见该 task 的假设） |
| P5c／P5d／P5f 三份设计 | **已定稿**（`docs/superpowers/specs/2026-10-09-p5{c,d,f}-…-design.md` 三份都在） | 设计 §3.6 的排期判据据此闭合，见本节第二条 |
| 迁移号段 | **本块零迁移** | 无：不动 `runtime_migrations()`（`crates/continuum-runtime/src/main.rs:98-108`），不占任何档 |
| `continuum-media` | **不存在**（实测：`grep -rn "continuum-media\|continuum_media"` 在本仓只命中三份**设计文档**，无 `Cargo.toml`、无源码） | 本计划的交付物 |

### 二、哪些 task 在 P5a／P5b 的实现落地前动不了

- **Task 8 硬依赖 `continuum-method`**：`bind_media_methods(registry: &mut MethodRegistry)` 的形参类型住在那个 crate 里，
  且 §12.2 第 (s) 条的判据要在本 crate 里同时拿到 `all_operators()` 与 `MethodRegistry`（设计 §10 末）。
  **该 crate 今天不存在，故 Task 8 在 P5b 的骨架落地之前编不过。**
- **Task 9 硬依赖 `continuum-verify`**：两条 `trybuild` 样例断言「在本 crate 里造不出 `Evidence`」，
  而 `Evidence` 的八字段私有、无 `Default` / `From` 是 P5a 设计的**结构事实**（其 §4.4）。**该 crate 今天不存在。**
- **两条边按「用它的那个 task 自己登记」办**：Task 3 的 `ALLOWED` 条目只含当时实际有的两条边
  （`continuum-artifact`、`continuum-operator`），`continuum-graph` / `continuum-method` / `continuum-verify` 三条
  分别由 Task 7／8／9 追加。**一次声明齐会让条目在中间若干 task 里说谎**，而 `dependency_direction` 的门是**逐对 `assert_eq!`**
  （`crates/continuum-runtime/tests/dependency_direction.rs:288` 起的用例），说过头与说不够都红。
- **其余八个 task** 只碰 `continuum-artifact` / `continuum-port` / `continuum-operator` / `continuum-graph`(dev)
  与本块新建的 crate，**不因 P5a／P5b 未落地而编不过**。
- **设计 §3.6 的排期判据已闭合，且判据是实测的**：设计 §3.6 要求「`ArtifactType` 的那一次编辑发生在
  P5c／P5d／P5f 三份的『报出』之后」。三份设计现已定稿，**逐份的报出都是「无」**：
  P5c §8 逐字「**本块的答复是「无」**」并按端口逐项列出它只用四枚既有型；
  P5d §3.2 逐字「**两枚都不由本块报出**」（`Json` 既有、`Report` 已在本设计 §3.2 清单第 8 行）；
  P5f §3.4 末逐字「**本块不向 P5e 报出任何变体**」「**清单为空**是实测结论，不是「暂时没想到」」。
  ⇒ **Task 1 落地的是设计 §3.2 的 11 枚，无待合并项**；本 task 的 brief 里须写明这一句
  （设计 §3.6 明写「实现该任务的 brief 里必须写明」）。

### 三、本块对既有之物的改动（**都在切分 §一 的共写文件表里，一处都不多**）

| 文件 | 改动 | 依据 |
|---|---|---|
| `Cargo.toml`（`[workspace] members`） | 加一行 `"crates/continuum-media"` | 切分 §一 的共写表；**本块只加自己这一行**，六行齐否由协调者定 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-media", &[…])`，**五项不出现在同一次改动里**（Task 3／7／8／9 各追加） | 切分 §四第 6 条 |
| `crates/continuum-artifact/src/artifact.rs` | 枚举本体、`:24` 的注记、`ALL`、`as_str`、`parse` 五处 | 切分 §四第 2 条 (a)(b)(c) |
| `crates/continuum-artifact/tests/artifact_type.rs` | `:23` 的数目断言 | 切分 §四第 2 条 (d) |
| `crates/continuum-port/tests/compatibility.rs` | `:47` 的用例改为遍历 `ALL`，并逐型断言 **serde 串与 `as_str()` 相同** | 切分 §四第 2 条 (e)——**清单里唯一不会自己变红的一处**；加的那条断言是本计划自定，申报见 `## 遗留` 第三节 |
| `crates/continuum-operator/src/definition.rs` | `:95` / `:96` 两行签名、`:76` 之后加 `BackendId` 的 `Display`、`:90-91` 的注记、末尾加两枚新类型 | 设计 §4.4；切分 §一 的共写表 |
| `crates/continuum-operator/src/lib.rs` | `:7-9` 的 `pub use` 补两枚 | 设计 §4.4（一并清掉 P1 计划 `:5565` 记的「未 re-export」遗留） |

**本块不碰**：`crates/continuum-graph/**`（只 `use` 它的 `cache_key` / `can_reuse`，一个字不改）、
`crates/continuum-persist/**`、`crates/continuum-runtime/src/**`（零迁移、零钩子）、
`docs/02-工程.md`、`docs/spec/**`、任何一份设计文档、任何一份别的计划。

### 四、本节把什么交给谁

- **交给第 3 层的执行器（未建）**：设计 §13 第 1 条把**三处「有判定、无调用点」并成一笔**——
  `checkpoint()` / `restore()`（本计划 Task 6 交实现与用例）、`authorize_generative`（Task 5）、
  `OperatorRegistry::resolve` 的调用点（切分 §八 已有的那一处）。**三处同落 `Queued → Running`**，
  本块不自建（切分 §四第 4 条），**本计划不为它们建任何接线 task**。
- **交给 P5a（`continuum-verify`）**：设计 §9.1 的答复「`VisualCheck` 够用、不请求新臂」；
  两枚验证算子的 §262 级别声明（第 1 级与第 3 级，设计 §5.5）。**本计划只交付那两条声明，不交付 `AvailableVerifier` 的构造。**
  那两条声明的载体是 `src/operators.rs` 里 `verify-deterministic` / `verify-multimodal` 两枚算子值上的文档注释（Task 3 Step 3）。
- **交给 P5b（`continuum-method`）**：四条 `video/` 方法的 `realized_by`（Task 8）。
  **本块不 `register` 方法条目**（P5b 设计第六节：四块不得自造 `MethodEntry`）。
- **交给装配方**：`Evidence` 的构造调用点（`subject` 取 `Unattached`、`producer` 取 `EvidenceProducer::Node{..}`），
  与 §34 授权判定在真实执行路径上的调用（设计 §7.3、§9.2）。
- **交给规范维护者**：设计 §14 第 3、4、6、7 条与 §13 第 3、4、7 条——**本计划一条都不发明**，逐条抄在 `## 遗留` 第四节。
- **交给协调者**：本计划查出的**四处设计问题**（逐条见 `## 遗留` 第一节：`GenerativePermission` 的容器形状未给、
  `CheckpointError` 的编码与设计 §12.1 相抵、两枚检查点实现的类型名与它们和 `all_operators()` 的关系未给、
  `PermittedGenerativeContent` 的「缺省」问题）——**Task 5／6 的开工不挂在它们上**（本计划各取一个最小形状并申报）。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划新增的 workspace 内边只有这些**（逐条由需要它的 task 登记，**一次不声明齐**）：
  - Task 3：`continuum-media ← continuum-artifact`（`ArtifactType` 用于端口类型）、
    `continuum-media ← continuum-operator`（`Operator` 族与注册表）；
  - Task 7：追加 `continuum-media ← continuum-graph`（**dev 边**：`cache_key` / `can_reuse`）；
  - Task 8：追加 `continuum-media ← continuum-method`（**普通边**：`bind_media_methods` 收 `&mut MethodRegistry`）；
  - Task 9：追加 `continuum-media ← continuum-verify`（**dev 边**：两条 `trybuild` 样例）。
  **上表之外一条跨层边都不登记**——尤其没有指向 `continuum-port` / `continuum-semantics` / `continuum-persist` /
  `continuum-events` / `continuum-policy` / `continuum-capability` 的边（设计 §2.1 的五行之外逐条给了理由）。
  **外部 crate（`thiserror` / `trybuild` / `tempfile`）按需加，不进 `ALLOWED`**——那张表只逐对断言 workspace 成员之间的边。
- **边别按「谁在用」判，不看「设计里写没写」**：生产代码用到的边是普通边；只在用例／样例里出现的边是 **dev 边**，
  而 `cargo tree` 带 `--edges all`（`crates/continuum-runtime/tests/dependency_direction.rs:10`），**dev 边与普通边一视同仁地进 `ALLOWED`**。
  本块据此把 `continuum-verify` 与 `continuum-graph` 记成 dev 边，理由是**生产代码对它们零调用**
  （`OperatorImpl::execute` 的返回是 `Vec<Artifact>`，`crates/continuum-graph/src/execution.rs:81`）。
- **`--depth 1` 只钉直接边，这就是这条规则的完整粒度，本计划不另设闭包断言**（`dependency_direction.rs:274` 的 `--depth 1`）。
  写这一段的用途：免得读到上面那条约束时以为它是「任何指向别层的边都会变红」——**那会是一条假保证**。
- **本块零迁移、零表、零事件**：不新增 `Migration::new`，不改任何既有迁移的 SQL，不动
  `crates/continuum-runtime/src/main.rs:98-108` 的 `runtime_migrations()`，不注册任何 `RecoveryHook`。
  **设计 §11.2 的实测占用表是这一条的判据**（进装配链的九处取值为 `1` `2` `10` `20` `30` `40` `41` `50` `80`；
  测试夹具里出现的 `50` `60` `100` `101` `102` **都在各自自建的库里，不进装配链**）。
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。不碰 `.claude/`、`CLAUDE.md`、config。
- **不要用 `git add -A` / `git add .`，不要 `git commit --amend`。** 只 `git add <显式路径>`；
  **新增依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不定义第二份复用判据**（切分 §四第 7 条；§131 的兑现是「让本领域算子满足 §305 的前提」，Task 7）。
  - **不定义证据类型、不定义「充分」判据、不定义完成判定**（切分 §四第 1 条；证据只有 P5a 的构造点，设计 §9）。
  - **不写域包装函数**（协调者 2026-10-10 裁定；原 `evidence_from_media_output` 已删，设计 §9.2）。
  - **不定义方法（Method）的登记形态、不自造 `MethodEntry`、不 `register` 方法条目**（切分 §四第 3 条与 P5b 设计第六节）。
  - **不注册 3D 链的两枚算子**（`Reconstruct3D` / `RenderScene`）：归属本轮无解（切分 §八），设计 §3.3 只收 `Scene` 这一型。
  - **不注册 `Image` 的两枚算子**（完整生成、局部修改）：P5f 的（设计 §1.2、§5.2 末）。
  - **不接管节点状态迁移、不调 `OperatorRegistry::resolve` 去驱动执行**（切分 §四第 4 条）。
  - **不碰 `execution_policy` 与 `verification_policy`**（切分 §四第 5 条；后者的定型在 P5a）。
  - **不新增 `EvidenceType` 臂**（设计 §9.1：答复「够用」）。
  - **不为 §329 的 Timeline 六字段发明任何 schema**（设计 §8 第 3 条：那是缺口，收件人是规范维护者）。
  - **不收 `SourceSet` 这一型**（设计 §3.4）。
  - **不建 `MethodDomain` / `has_counterpart` / `UNREALIZED_BY_DESIGN`**：那些定义在 `continuum-method`（P5b 设计 §4）。
  - **不给 §187 补方法名、不向 `3d/` 目录 `bind` 任何东西**（设计 §10 末、§3.3 第 2 条）。

### 本计划特有的「签名即判据」，照抄前须对源

- `ArtifactType`：`pub const ALL: [ArtifactType; 6]`（`crates/continuum-artifact/src/artifact.rs:28`）、
  `pub fn as_str(&self) -> &'static str`（`:47`）、`pub fn parse(s: &str) -> Option<Self>`（`:68`）——
  **`as_str` 是穷尽 `match` 无通配臂，`parse` 从 `&str` 出发故有 `_ => return None`**。
  **这一处决定了「加型即编译不过」这条照片在哪个函数上成立、在哪个函数上不成立**（Task 1 的 Step 1 逐条写明）。
  枚举带 `#[serde(rename_all = "snake_case")]`（`:13`），`parse` 与 `as_str` 的串**逐型相同**（`:42-46` 的注记）。
- `Operator`：七个 `pub` 字段、无 `Default`（`crates/continuum-operator/src/definition.rs:78-88`）——
  **故 17 枚算子值可以在 crate 外用手写字面量构造**（Task 4 第 (i) 条要造一枚违规算子）。
  `OperatorId::new(impl Into<String>)`（`:27`）、`OperatorVersion::new(u32)`（`:48`）、`BackendId::new(impl Into<String>)`（`:69`）。
- `OperatorRegistry::{register, resolve}`（`src/registry.rs:24`、`:36`）与 `OperatorError::{NotFound, Duplicate}`（`:7-12`）。
  `resolve` 收 `(&OperatorId, &OperatorVersion)`，**不是值**。
- `Checkpointable`（`src/definition.rs:92-96`）：关联类型 `type Checkpoint: Send + Sync;`、两个方法今天收 `String` 作为错误。
- `cache_key(&Operator, ContentHash) -> Option<CacheKey>`（`crates/continuum-graph/src/reuse.rs:16`）、
  `can_reuse(&Operator, Option<&CacheKey>, &ContentHash, bool) -> bool`（`:29`）、
  `CacheKey { input_hash, operator_version }`（`:9-12`）；`ContentHash::of(&[u8])`（`crates/continuum-artifact/src/content.rs:11`）。
- P5b 的 `MethodRegistry::bind(&mut self, id: &MethodId, realized_by: &[&str]) -> Result<(), MethodError>`、
  `MethodRegistry::seeded()`、`MethodRegistry::new()`、`MethodRegistry::select(&self, domain: MethodDomain) -> Vec<&MethodEntry>`、
  `MethodId::new(impl Into<String>)`、`MethodEntry::realized_by: Vec<String>`、`MethodDomain::Video`、
  `MethodError::{NotFound, Duplicate}`
  （`docs/superpowers/specs/2026-10-08-p5b-execution-method-library-design.md` §4.1／§4.2／§4.3／§4.4／§4.5 的形状块）。
  **落地前须读 `continuum-method` 的源码确认**——本计划引用的是**设计**，该 crate 今天不存在。
- P5a 的 `Evidence::from_tool_result` 八参数（`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md` §4.4）、
  `EvidenceSubject::{Requirement, Unattached}`（其 §4.3.1）。**同样只有设计，落地前须对源。**

---

## 已付过代价的纪律（照 P4 那份按本块实情改写）

1. **变异的红必须在全量 `cargo test --workspace --no-fail-fast` 下由「用例失败」得出**；正向的「变红」跑全量是加分。
   **变异分四档，每一处「预期谁红」都标档位**：**取反**（把判定反过来）／**放宽**（少判一半条件）／
   **收紧**（多判一半条件）／**移除**（删掉整条守卫）。**三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**；
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
   cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，**不能拿它当判据**）。
   **本块已预先识别的三处等价变异体**（换变异体，不补用例）：
   - **把某个新变体的串换成另一个同样合法的小写串**（`shot_set` → `shotset`）：**只看 serde 往返时**它全绿
     ——要打红它，或换成与 `crates/continuum-artifact/tests/artifact_type.rs:40` 的表外取值相撞的串，
     或直接断言**串的字面值**（Task 1 的 (a) 取后者）、或断言 **serde 串与 `as_str()` 相同**
     （Task 1 的 (d) 取后者；两处都已落，见该 task 与 `## 遗留` 第二、三节）；
   - **把 `[Operator; 17]` 的数目字面量改成 16**：那是**编译期**的红（返回类型不一致），**不算变红**；
     运行期可观察的那一半是 `len()` 断言与 (f) 的逐枚 `resolve`（Task 3）；
   - **把某一枚旗标的串改成另一个未被占用的合法串**（`as_str` 与 `parse` **两处一起**改）：往返仍成立、全绿
     ——打红它要靠**串的字面值断言**，即 Task 5 的第 1 条用例（`the_five_flags_are_exactly_the_five_from_section_34`
     逐臂断言 `parse(as_str(p)) == Some(p)` 与串的字面值）；**(l) 与它无关**（(l) 断言的是「算子 → 旗标」的映射）。
     **别把它写成「把两枚的串互换」**：互换也要在 `as_str` 与 `parse` 两处**一致地**做才是等价变异体；
     **只在一处改，`parse(as_str(p)) == Some(p)` 当场为假、往返即红**——那是真变异体，报成「等价」会白跑一轮。
2. **变异脚本必须带还原护栏**（本仓出过一次「变异留在源码里」的事故）：
   每次变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、**每轮用独立日志路径**，
   报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位」。模板（`MUT` 是改的文件、`BAK` 是备份）：
   ```bash
   cd /home/DslsDZC/Continuum/.worktrees/p5e-plan
   before=$(sha256sum "$MUT" | cut -d' ' -f1)
   cp "$MUT" "$BAK"
   trap 'cp "$BAK" "$MUT"; echo "已还原"' EXIT INT TERM
   # …施加变异…
   TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$LOG"
   # …读 $LOG 判红…
   ```
   **`trap` 那一行不许省。**
3. **门读数的判据是日志里的行数，不是「命令退出码为 0」**：
   `cargo test --workspace --no-fail-fast` 的读数取**日志里 `Running ` 的行数与 `test result:` 的行数**，
   两者须与当时的 test target 数一致（**Doc-tests 各算一条 `Running` 与一条 `test result:`**）。
   `cargo test` **不收 `--keep-going`**（那是 `cargo build` 的开关），不加 `--no-fail-fast` 会在第一个失败目标处停住、
   后面的目标一行不跑——**「后面的目标是绿的」这句话在没有 `--no-fail-fast` 的日志里读不出来**。
4. **断言必须有照片，且语料不得从被测清单里取**（**正控制不得自证**）：
   「11 枚新变体都在枚举里」若写成遍历 `ArtifactType::ALL`，测的是「`ALL` 与 `parse` 一致」——
   那正是 `ALL` 的既有守卫，**「那 11 枚在不在 `ALL` 里」不会被它测到**。
   本块的两处恒真假照片，设计 §12.2 末已点名：第 (a) 条须**手写 11 枚清单**、第 (f) 条须**手写 17 枚 id 清单**。
5. **自指计数不写数值**：凡「N 条用例」「N 枚算子」这类**自指**计数，**一律不写数字**
   （写了就要靠人同步，而没有任何东西会在它漂掉时变红）；数目由**类型或运行期断言**承载——
   本块承载它的是 `[Operator; 17]` 的返回类型、`ALL` 的数组长度与各 task 里那几条 `len()` 断言。
6. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本计划逐处标出「另一侧」是哪一个用例：
   §5.4 的名单规则（Task 4 的 (h) 正例 ＋ (i) 反例）、§34 的授权（Task 5 的 (j) 反例 ＋ (k) 正例 ＋
   「空授权集什么都不放行」那一条）、检查点的三臂（Task 6 的 (o)(p)(q) ＋ 后端那两侧）、
   注册入口（Task 3 的 (f) 正例 ＋ (g) 反例）。
7. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」；**并断言没有半写的副作用**：
   本块的失败面是 `MediaError` 的三臂（Task 4 的 (i)、Task 3 的 (g)、Task 5 的 (j)）与
   `CheckpointError` 的三臂（Task 6），**逐臂至少一条用例**；
   「没有半写」在 (i) 的第二半（注册表内容与调用前逐枚相同）。
8. **一个纯数据声明没有运行期照片，只钉类型与签名（编译期）；且钉类型的机制必须写死为
   「编译器强制穷尽的解构／结构体字面量」且不得带 `..`。**
   **Rust 没有反射**——若「把字段名各列一遍」被理解成手写两份名单，那条断言恒真。**本块适用这条的类型**：
   17 枚 `Operator` 值（结构体字面量，七字段全写，`crates/continuum-operator/src/definition.rs:79-88`）、
   `PermittedGenerativeContent`、两枚检查点类型与它们的 `Progress`。
9. **凡注释写绝对措辞，必须有对应用例；且要带时点**——「唯一／一律／只有／全部／没有任何」这类词，
   要么有可写出的照片，要么**明写为什么没有**。**断言的作用域要与事实同宽**：
   「本 crate 的源码文本里零命中」≠「全仓零命中」；「在本 task 结束时」≠「永远」。
   **枚举式绝对断言须逐项有照片**——本块的 (l)（四枚生成算子各一条）、(n)（五枚 `side_effect_class` 各一条）、
   (r)（六行复用各一行）、(t)（17 行边表逐行）**都不抽代表**。
10. **trybuild 的两条坐标**：（i）**配对按文件主干**——`tests/compile_fail/<stem>.rs` 与 `<stem>.stderr`；
    （ii）**样例里的注释也是照片坐标**——`.stderr` 记的是 `文件:行:列`，样例里增删一行注释就会让行号漂、
    从而让「编译不过」与「断言的那条编译不过」不再是一件事。**两份 `.stderr` 的内容一律取自实跑，不许凭记忆写**，
    且**逐份各不相同，不许拿一个值套两处**。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。
**变异日志是证据，必须活到复审结束**：实现者保留 `.tmp/`，由协调者在复审结束后清理。
**变异窗口与验证窗口互斥**：实现者报告完成之前，协调者不得在该工作区里跑 cargo。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**。

**本计划特有的三处「照抄前须对源」**：

- **§5.2 的 17 行算子表**（Task 3 Step 1）是**照抄设计的转录**：设计 §5.2 逐行给了
  `id` / `input_schema` / `output_schema` / `determinism` / `side_effect_class` / backend 候选 / 出处。
  照抄前须**逐行对设计原文**；本计划的表**不含出处列**（那是设计的事），六列之中 backend 候选一列
  是 Task 4 的名单规则的输入，**抄错即让 (h) 与 (i) 同时失真**。
- **`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑**（Task 9），不许凭记忆写。
- **§5.4 的名单常量**（Task 4）在设计 §5.4 里是**九枚 backend 名**；照抄前对源，
  且**名单之外一律拒绝**（这一侧正是 fail-open 的反面）。

---

# 文件结构

```
crates/continuum-media/                 （新建）
  Cargo.toml
  src/lib.rs              导出面与 crate 文档；P5e 的四处判定各一行指向实现处
  src/error.rs            MediaError（三臂）
  src/operators.rs        all_operators() -> [Operator; 17]（§5.5 的两条级别声明在两枚验证算子值的
                          文档注释上）、register_media_operators、
                          逐位可复现 backend 的名单常量与注册期规则（§5.4）
  src/permission.rs       GenerativePermission（五臂 ＋ ALL ＋ as_str / parse）、
                          PermittedGenerativeContent、required_permission、authorize_generative（§34）
  src/checkpoint.rs       GenerateBroll / Render 两枚类型、Progress，与它们的 Checkpointable 实现
                          及那组读法／写法（§4.5）
  src/methods.rs          bind_media_methods（§10；唯一入口，只调 MethodRegistry::bind）
  tests/operators.rs      (f)(g)(n)：清单、注册的两侧、五枚 side_effect_class 逐枚
  tests/ports.rs          (t)：§5.3 的 17 行边表逐行
  tests/determinism.rs    (h)(i)：§5.4 的名单规则两侧
  tests/permission.rs     (j)(k)(l)(m)：§34 的授权两侧与四枚映射
  tests/checkpoint.rs     (o)(p)(q) 与后端两侧：§4.5 的三条判据
  tests/reuse.rs          (r)：§6.2 的六行复用照片（dev 边 continuum-graph）
  tests/methods.rs        (s)：四条 realized_by 非空、每个串可 resolve、不新增条目
  tests/type_level.rs     trybuild 驱动（dev 边 continuum-verify）
  tests/compile_pass/*.rs 正控制样例（编译通过）
  tests/compile_fail/*.rs 两条结构样例（各配同名 .stderr）
```

**本计划要改的既有文件**（Task 1／2／3）：

```
Cargo.toml（workspace）                                   members 加 crates/continuum-media（Task 3）
crates/continuum-artifact/src/artifact.rs                 五处（Task 1）
crates/continuum-artifact/tests/artifact_type.rs          :23 的数目断言（Task 1）
crates/continuum-artifact/tests/artifact_types_p5.rs      新建：(a)(c)(e) 的用例（Task 1；字母取设计 §12.2）
crates/continuum-port/tests/compatibility.rs              :47 的用例改为遍历 ALL，并逐型断言 serde 串与 as_str() 相同（Task 1）
crates/continuum-operator/src/definition.rs               两行签名、BackendId 的 Display、:90-91 注记、末尾两枚新类型（Task 2）
crates/continuum-operator/src/lib.rs                      pub use 补两枚（Task 2）
crates/continuum-runtime/tests/dependency_direction.rs    ALLOWED 的 continuum-media 条目（Task 3／7／8／9 各追加）
```

**本计划不新建、不修改任何文档**（`## 遗留` 一节在本计划内，`docs/superpowers/plans/` 是有版本的位置）。

---

### Task 1: `ArtifactType` 的 P5 扩展——**一次改动的五处**

**Files:**
- Modify: `crates/continuum-artifact/src/artifact.rs`（枚举本体 `:14-21`、注记 `:24`、`ALL` `:28-35`、`as_str` `:47-56`、`parse` `:68-78`）
- Modify: `crates/continuum-artifact/tests/artifact_type.rs`（**只改** `:23` 的数目断言 `6` → `17`）
- Modify: `crates/continuum-port/tests/compatibility.rs`（`:47` 的用例改为遍历 `ALL`，并逐型断言 serde 串与 `as_str()` 相同）
- Create: `crates/continuum-artifact/tests/artifact_types_p5.rs`（(a)(c)(e) 的用例，字母取设计 §12.2：手写的 11 对 ＋ 落库往返；
  **新文件而不是改 `artifact_store.rs`**，理由见 Step 1）

**Interfaces:**
- Consumes: 无需别的 task
- Produces: `ArtifactType` 的 11 枚新变体与 `ALL: [ArtifactType; 17]`——**本块的全部下游（P5c/P5d/P5f 的实现、Task 3 的端口声明）都消费它**

> **排期判据（设计 §3.6，本计划已核）**：本 task 是**六块里对该枚举的最后一次编辑**；
> P5c／P5d／P5f 三份设计的报出**逐份为「无」**（实引见「跨计划前置」第二节）。
> **本 task 之后不得再有人改这个枚举**；P5f 需要的 `Image` 由本 task 提供，不由 P5f 自己加。

- [ ] **Step 1: 写用例**

**落点分两处，为的是让「先跑红」真的可执行**（纪律与「`Files` 里写实现体前先核它存不存在」同一条）：
`artifact_type.rs` 的既有用例只用 `ALL`，**故它只改数目字面量即可单独跑**——**它是本 task 唯一的运行期红点**；
新用例（(a) 与 (c)）**一律落在新文件** `tests/artifact_types_p5.rs`，它们引用还不存在的新变体，
**故在 Step 2 是编译失败，不是用例失败**（照实记账）。

`crates/continuum-artifact/tests/artifact_type.rs`（**只改 `:23` 的数目字面量** `6` → `17`；
既有的 `every_artifact_type_round_trips_through_its_encoding` 其余部分**一字不动**，它的 `:40` 那六个表外取值也不动）：

`crates/continuum-artifact/tests/artifact_types_p5.rs`（新建）：

> **字母用哪一套（本节与 Step 3 的表不是同一套，读时别看串）**：**本 step 里用例的字母取设计 §12.2 的一套**
> ——(a) 11 枚新串的往返、(c) 落库往返、(d) `compatibility.rs`、(e) 两个多词串；
> **Step 3 那张表的字母取设计 §3.5 的一套**——(a) 枚举本体、(b) `ALL`（含 `:24` 的注记）、
> (c) `as_str` / `parse`、(d) 数目断言、(e) `compatibility.rs`。**同一个 `(e)` 在两套里是两件事**
> （§12.2 的 (e) 是多词串，§3.5 的 (e) 是那个用例）。本计划不合并两套字母（合并就要重述设计的表）。

- `every_p5_artifact_type_has_exactly_its_declared_encoding`（**第 (a) 与 (e) 条**，设计 §12.2 的字母）：
  语料是**手写的 11 对**（`变体`, `串`），**不从 `ALL` 取**——逐对断言两件事：
  （i）`ty.as_str() == 期望的串`；（ii）`ArtifactType::parse(该串) == Some(ty)`。
  **（i）是承重的那一半**，理由是设计 §12.3 已把「只断往返」标成**等价变异体**：
  把 `shot_set` 改成 `shotset`，**在只看 serde 往返的语料里**仍成立、仍绿；
  **能把两版分开的输入就是那个期望串本身**。
  **红条件（取反档）**：把 `SubtitleTrack` 的 `as_str` 写成 `"subtitletrack"` ⇒（i）红。
  **红条件（收紧档）**：把 `ShotSet` 的串写成 `"shot_set "`（尾空格）⇒（i）红，且 `artifact_type.rs` 里
  「小写 + `_`」那条断言也红（`:32-35`，它对 `ALL` 逐型跑）。
  **多词型只有两条**（`shot_set` / `subtitle_track`），故「多词串」这一条不抽代表：两条各在自己的那一对里。
- `a_new_type_survives_a_commit_and_read_back`（**第 (c) 条**）：
  用 `ArtifactStore` 提交一枚 `artifact_type: ArtifactType::Timeline` 的制品，读回并断言 `artifact_type == Timeline`；
  **11 枚新变体逐枚各一条**（不抽代表——落库路径共享，但**编码是逐型产生的**）。
  **红条件（放宽档）**：`parse` 里漏掉 `timeline` 这一臂 ⇒ 该枚读回是 `Err`。
  **这一条钉的是 `crates/continuum-artifact/src/persist.rs:164` 的 `parse_type` 委托链**（它委托到 `parse`，`:165`，不含自己的表）。
  **新建文件而不是改 `artifact_store.rs`**：后者的夹具 `fn artifact(...)` 固定 `ArtifactType::Text`（`:8-21`），
  改它会动到既有四条用例的语料。
- 第 (b) 条（六个表外取值仍 `None`）**本 task 不改**：`crates/continuum-artifact/tests/artifact_type.rs:40` 那条断言
  已覆盖它，**本 task 只需在 Step 4 重跑时核对**（设计 §3.5 的 (f)：11 个新串与那六个无一相同）。
  **它的红条件（取反档）**：把 `parse` 的某一臂写成 `_ => Some(..)`（兜底为默认型）⇒ 那条既有断言红。

`crates/continuum-port/tests/compatibility.rs:47` 的 `six_types_round_trip_through_serde`（**第 (d) 条**）：
- **改为遍历 `ArtifactType::ALL`**，**逐型加一条断言：serde 串与 `as_str()` 相同**
  （`serde_json::to_string(&t)` 去掉两侧引号后与 `t.as_str()` 逐字相同），
  并**把函数名里的数目字面量去掉**（改名，使下一次加型不必再改名字）。
  ```
  fn every_artifact_type_round_trips_through_serde()
  ```
  **为什么要加那条断言（本计划自定，申报见 `## 遗留` 第三节）**：只做 serde 往返时，
  「删掉 `artifact.rs:13` 的 `#[serde(rename_all = "snake_case")]`」这一档**不红**——
  `SubtitleTrack` 序列化成 `"SubtitleTrack"`、反序列化仍解回 `SubtitleTrack`，往返照旧成立，
  **那是一枚等价变异体**（举不出会给出不同结果的入参）。**能把两版分开的入参就是那两个串本身**，
  故把「serde 表示 == `as_str()`」写成断言；它同时是 `artifact.rs:41-46` 那条既有注记
  （「`rename_all = "snake_case"` 给出的字符串与本函数逐型相同」）的照片。
  **这是本 task 唯一一处「不改也会照旧编译、照旧通过」的地方**（设计 §3.5 的 (e)）：它手工列了六型
  （`:49-54`），加 11 型之后它仍绿，而**它的名字与它的清单都成了假话**。
  **红条件（移除档）**：把循环改回手工六型 ⇒ **本用例不变红**——这一处的守卫是**清单本身**
  （它与 `ALL` 是两个各自维护的清单，而 `ALL` 有守卫、它没有），故**红条件写在另一处**：
  把 `ArtifactType` 的 `#[serde(rename_all = "snake_case")]`（`artifact.rs:13`）删掉 ⇒ 遍历 `ALL` 时
  `SubtitleTrack` 的 serde 串是 `"SubtitleTrack"`、`as_str()` 是 `"subtitle_track"` ⇒ 那条新加的断言红。
  **照实记**：「删 `rename_all` 会红」这句话只在**加了这条断言之后**成立；只断往返时该变异是等价变异体。
  **并写明**：既有用例 `different_type_is_rejected_with_both_sides_named` 等三条**不动**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-artifact --test artifact_type
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-artifact --test artifact_types_p5
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-port --test compatibility
```

**预期，逐条分开读**：
- `cargo test -p continuum-artifact --test artifact_type` ⇒ **运行期失败**，红在
  `assert_eq!(ArtifactType::ALL.len(), 6, "ArtifactType 的名单与变体数不符")`（`tests/artifact_type.rs:23`）
  ——**这是本 task 唯一的「先跑红」，它是运行期的，可作 Step 2 的读数。**
- `cargo test -p continuum-artifact --test artifact_types_p5` ⇒ **编译失败**（`ArtifactType::Timeline` 等 11 枚还不存在）。
  **按纪律 1(c) 这不是「用例变红」**，照实记为编译期红点。
- `cargo test -p continuum-port --test compatibility` ⇒ **这一步直接绿**（`ALL` 今天就存在，改成遍历它不会失败）。
  **照实记**：本 task 对 (e) 的兑现是**改掉一个会静默变假的清单**，不是「让它由红转绿」。

- [ ] **Step 3: 实现（一次改动，五处同时）**

**本表的字母取设计 §3.5 的一套**（与 Step 1 的 §12.2 字母不同，见 Step 1 开头的注记）：

| 处 | 落点（实测） | 改法 |
|---|---|---|
| (a) 枚举本体 | `crates/continuum-artifact/src/artifact.rs:14-21` | 追加 11 枚变体，**顺序照设计 §3.2 的表**（`Image` `Audio` `Video` `Timeline` `Mesh` `Scene` `Transcript` `Report` `ShotSet` `SubtitleTrack` `Render`） |
| (b) 注记 | 同文件 `:24` | 「全部六型。供全量遍历的用例使用（与 [`PrivacyClass::ALL`] 同形）。」→ **「全部十七型」**。**这一处在 `///` 里，不会编译失败**；只改代码不改它，注释即成假话（设计 §3.5 的 (b) 补充） |
| (b) `ALL` | 同文件 `:28-35` | `[ArtifactType; 6]` → `[ArtifactType; 17]` 并补齐 11 项。**`[ArtifactType; 6]` 与 17 项不符即编译失败**——这是 (b) 的报警机制 |
| (c) `as_str` / `parse` | 同文件 `:47-56` / `:68-78` | 两个穷尽 `match` 各补 11 臂。**`as_str` 无通配臂，加型即编译失败**；**`parse` 从 `&str` 出发，一定要有 `_ => return None`——它不是那张「加型即编译不过」的照片**（这一句要写进用例注释） |
| (d) 数目断言 | `tests/artifact_type.rs:23` | `6` → `17` |
| (e) 手工清单 | `crates/continuum-port/tests/compatibility.rs:47`（六型列举在 `:49-54`） | 循环改为 `for t in ArtifactType::ALL`，函数去掉数目字面量，**并逐型加「serde 串 == `as_str()`」断言** |

**串名沿用本文件既有的约定**（`:37` 的注记：小写、多词以 `_` 连接）：`shot_set` / `subtitle_track`。

**不改但要重跑的三处**（设计 §3.5 的末表）：`crates/continuum-artifact/src/persist.rs:164` 的 `parse_type`、
`crates/continuum-graph/src/persist.rs:410` 的 `parse_artifact_type`（两者都委托到 `ArtifactType::parse`，
不含自己的表）、`crates/continuum-port/src/port.rs:94` 的 `compatible`（按等值判定，`:101`）。**一处都不改。**

- [ ] **Step 4: 运行全部测试并提交**

**重跑要看什么**（Step 4 的读数，逐条记进报告）：
（i）`cargo test -p continuum-artifact` 全绿（`artifact_store.rs` / `blobstore.rs` / `privacy_class.rs` 也在其中）；
（ii）`cargo test -p continuum-graph --test persistence` 全绿（`adfir_port.artifact_type` 那一列的读回路径，
`:259` / `:278`）；（iii）`cargo test -p continuum-port` 全绿。

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-artifact crates/continuum-port Cargo.lock
git commit -m "feat(artifact): ArtifactType 的 P5 扩展清单（11 枚新变体）一次落地"
```

> **提交信息里不要写数目以外的承诺**；本 task 之后**不得再改这个枚举**（设计 §3.6）。

---

### Task 2: `Checkpointable` 的错误类型——两行签名、两枚新类型、`BackendId` 的 `Display`

**Files:**
- Modify: `crates/continuum-operator/src/definition.rs`（`:95` / `:96` 两行；`:76` 之后加 `Display`；`:90-91` 注记；文件末尾加两枚新类型）
- Modify: `crates/continuum-operator/src/lib.rs`（`:7-9` 的 `pub use` 补 `Checkpointable` 与 `CheckpointError`）
- Create: `crates/continuum-operator/tests/checkpoint_error.rs`

**Interfaces:**
- Consumes: 同文件的 `OperatorId`（`:24`）、`OperatorVersion`（`:45`）、`BackendId`（`:66`）
- Produces: `continuum_operator::{Checkpointable, CheckpointError}`、`continuum_operator::definition::CheckpointOwner`

> **落点为什么在别人的 crate（本节的第一个裁定，理由是依赖方向不是命名偏好）**：
> 错误类型若定义在 `continuum-media`（第 8 层），则 `continuum-operator`（第 3 层）要 `use` 它 ⇒
> **第 3 层与第 8 层之间的 2-环**，与 §9.1 `:588` 逐字写的「依赖方向单向，无环」相抵，
> 且会让 `dependency_direction` 门在两个方向都红。**落点不新增任何一条依赖边**（三个身份类型都在同文件内）。
> **「这里不是 `continuum-media`：因为那会让第 3 层依赖第 8 层，即 2-环。」**——这一句要写进类型的文档注释。
> **同一处境有先例、而先例不是硬理由**：切分 §二第 2 条订正段对 `verification_policy` 的处置形状
> （「先例只是「同一处境曾有同一处置」，成环才是「不得不这么办」」）。**本处没有先例，故只写成环这一条。**

- [ ] **Step 1: 写用例**

`crates/continuum-operator/tests/checkpoint_error.rs`：

- `each_of_the_three_arms_renders_its_own_discriminating_text`（**三臂各一条，不抽代表**）：
  （1）`UnsupportedByBackend { backend: BackendId::new("nvenc") }` 的显示文案含 `nvenc`；
  （2）`NothingToCheckpoint { owner }` 的文案含**该算子的 id 与版本两者**；
  （3）`RestoreRejected { expected, found }` 的文案**同时**含两个 owner 的 id 与版本。
  **红条件（取反档）**：把 `CheckpointOwner` 的 `Display` 写成只打印 `id`（丢掉版本）⇒
  第（2）（3）条红。**这一条钉的是「检查点的身份是 id ＋ version 两字段」这件事在**输出上**可见**——
  设计 §4.3 明写 `CheckpointOwner` 的两个字段与 `OperatorRef` 逐字相同而**不合并**，
  故它就是那个「两个字段都要有人读」的地方。
  **并写明**：`CheckpointError` **没有** `as_str` / `parse` / `ALL`——它的编码就是 thiserror 的 `Display`
  （本仓 `OperatorError`（`src/registry.rs:6-12`）同形：一个带载荷的错误枚举没有字符串往返这一说）。
  **设计 §12.1 要求它有 as_str/parse 与数目断言，与本设计 §4.3 的类型相抵**——见 `## 遗留` 第一节第 2 条。
- `backend_id_display_names_the_backend`：`BackendId::new("ffmpeg").to_string() == "ffmpeg"`。
  **红条件（取反档）**：把它实现成 Debug 形态（`BackendId("ffmpeg")`）⇒ 红；且 `UnsupportedByBackend`
  那条文案里会带引号与括号——**两条用例会一起红**（这是刻意：同一个 `Display` 被两处消费）。
- `the_two_trait_methods_are_typed_by_checkpoint_error`（**编译期照片，照实写**）：
  在本 crate 的测试里定义一个**最小实现体**：
  ```
  struct Minimal;
  impl Checkpointable for Minimal {
      type Checkpoint = u32;
      fn checkpoint(&self) -> Result<u32, CheckpointError> { Err(CheckpointError::NothingToCheckpoint { owner: .. }) }
      fn restore(&self, _c: &u32) -> Result<(), CheckpointError> { Ok(()) }
  }
  ```
  并**在运行期**调用它、`match` 出 `CheckpointError::NothingToCheckpoint { .. }` 这一臂。
  **两条腿差别写清**：（i）「签名收的是 `CheckpointError`」是**编译期照片**——
  trait 若仍是 `Result<_, String>`，这个实现体编不过（**那是 `could not compile`，不算变红**）；
  （ii）**运行期可观察的那一半**是「取回的错误能按三臂之一匹配」。
  **红条件（移除档）**：把 `NothingToCheckpoint` 与 `RestoreRejected` 合成一臂 ⇒ 实现体里的 `match` 臂对不上，
  **编译失败**——**故本条的失败路径守卫是第 1 条那三臂的文案断言，不是这一条**。
- `the_two_names_are_reachable_from_the_crate_root`（re-export 的照片）：
  测试文件顶部 `use continuum_operator::{Checkpointable, CheckpointError};`（**不用全路径**）。
  **红条件（移除档）**：删掉 `lib.rs` 的那两枚 re-export ⇒ **`E0432` 未解析导入**，
  即**编译不过、不算变红**；**故本条的运行期照片只有一条**：这个文件能跑起来（`cargo test` 收得住它）。
  **并写明**：P1 计划 `:5565` 记的「`Checkpointable` 未 re-export」是遗留项，**本 task 一并清掉**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-operator --test checkpoint_error
```

**预期**：**编译失败**（`CheckpointError` / `CheckpointOwner` 还不存在）。**这不是「用例变红」**；
**故本 task 的可读红点由 Step 1 的 `:90-91` 注记与签名改动提供不了**——**照实记**：
本 task 的红点在实现前**只有编译期**（`.stderr` 式的证据），**Step 2 的读数是 `could not compile`**，
报告里要按纪律 1(c) 标注，不许写成「先跑红再实现」。

- [ ] **Step 3: 实现**

| 位置（实测） | 改动 |
|---|---|
| `crates/continuum-operator/src/definition.rs:95` | `Result<Self::Checkpoint, String>` → `Result<Self::Checkpoint, CheckpointError>` |
| 同文件 `:96` | `Result<(), String>` → `Result<(), CheckpointError>` |
| 同文件 `:76` 之后（`BackendId` 的 `impl` 块之后） | 加 `impl std::fmt::Display for BackendId`（只依赖 `std::fmt`，**不新增 crate 依赖**；该 crate 的 `Cargo.toml` 里有 `thiserror`，`:11`） |
| 同文件 `:90-91` 的注记 | 「本子项目只定义接口」→ 「接口在 P1，错误类型与第一枚实现在 `continuum-media`」 |
| 同文件末尾（`Checkpointable` 之后） | `CheckpointOwner { pub id: OperatorId, pub version: OperatorVersion }`（`derive(Debug)`）＋ `impl std::fmt::Display for CheckpointOwner`（`write!(f, "{}@{}", self.id, self.version)`）＋ `CheckpointError` 三臂 |
| `crates/continuum-operator/src/lib.rs:7-9` | `pub use` 补 `Checkpointable` 与 `CheckpointError`（**`CheckpointOwner` 是否一并 re-export 由实现者定**：设计只点名前两枚，本计划不额外要求） |

**三臂各自的判据（写进文档注释，逐条照设计 §4.3）**：
`UnsupportedByBackend { backend }` 的存在理由是 §245 的多 backend——删掉它，「后端不支持」只能表现为 `Ok(())`
即**假装检查点成功**，而崩溃恢复会把「没存」读成「存了」；`NothingToCheckpoint { owner }` 可重试、
`RestoreRejected { expected, found }` 不可重试，两者合成一臂就没有 §309 的 failure type 分类落点。
**不给 `determinism` 留臂、不给 IO 臂**（§308 只规定 SHOULD 支持 checkpoint，没给存储）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-operator Cargo.lock
git commit -m "feat(operator): Checkpointable 的错误类型（CheckpointError 三臂）与 BackendId 的 Display"
```

**本 task 不动 `dependency_direction.rs`**：实测 `continuum-operator` 那一行是
`("continuum-operator", &["continuum-artifact"])`（`crates/continuum-runtime/tests/dependency_direction.rs:46`），
本节的改动不引入任何新 crate 依赖。

---

### Task 3: `continuum-media` 骨架、`MediaError`、17 枚算子与注册入口

**Files:**
- Create: `crates/continuum-media/Cargo.toml`
- Create: `crates/continuum-media/src/{lib.rs,error.rs,operators.rs}`
- Create: `crates/continuum-media/tests/operators.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加一行）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-media` 条目，**只列本 task 实际有的两条边**）

**Interfaces:**
- Consumes: `continuum_artifact::ArtifactType`（Task 1 的 17 型）、
  `continuum_operator::{Operator, OperatorId, OperatorVersion, Determinism, SideEffectClass, BackendId, OperatorRegistry, OperatorError}`
- Produces: `continuum_media::{all_operators, register_media_operators, MediaError}`——
  **本 task 的 `MediaError` 只有 `Registry` 那一臂**（撞车由 P1 的 `OperatorError::Duplicate` 判）。
  **另两臂的落点写死，不许落空臂、也不许提前落**：`UnverifiableDeterminism` 由 Task 4 加（它的判定是 §5.4 的名单规则），
  `NotPermitted` 由 Task 5 加（它的判定是 §34 的授权）。**「一个没有判定者的臂」在本块等于一条永不触发的死分支。**
  **另交付设计 §5.5 的两条声明**（载体是 `src/operators.rs` 里第 16、17 行两枚算子值上的
  文档注释，见 Step 3 末；**纯声明、无运行期照片**）。

- [ ] **Step 1: 写用例——17 行清单与注册的两侧**

**语料的来源（写死，照抄前对源）**：`tests/operators.rs` 里的 17 枚 id 清单**手写**，
内容逐行取自**设计 §5.2 的表**（`id` / `input_schema` / `output_schema` / `determinism` / `side_effect_class` / backend 候选，共 17 行）。
**本计划不复述那张表的逐格取值**（复述就是第二份转录，而设计是唯一事实来源）；**落地时逐行对设计原文**。
**唯一由本计划固定下来的是 id 的次序**（设计 §5.1 写死「顺序即 §5.2 表的行序」），逐枚照录：

```
media-import   proxy-build      shot-detect         transcribe        audio-analysis
vision-analysis  metadata-analysis  narrative-plan  timeline-compose  effects-subtitle
render         generate-broll   generate-voice      generate-music    frame-interpolation
verify-deterministic  verify-multimodal
```

- `the_seventeen_ids_come_in_the_designed_order`：`all_operators()` 逐枚的 `id` 与上表**逐位相同**。
  **红条件（取反档）**：把 `effects-subtitle` 与 `render` 两枚在数组里互换 ⇒ 红。
  **这一条的用途**：设计 §5.1 那句「顺序即 §5.2 表的行序」若不钉住，它就是一句**没有照片的绝对措辞**
  （纪律 9）；且次序是 (t) 那张边表的语料顺序。

- `the_seventeen_ids_are_hand_listed_and_each_resolves`（**第 (f) 条**）：
  以 `&all_operators()` 调 `register_media_operators`，然后**对**手写的 17 个 id **逐枚** `resolve` 成功。
  **语料必须手写**（设计 §12.2 末：若写成按 `all_operators()` 自己遍历，就是恒真的假照片）。
  **红条件（放宽档）**：`all_operators()` 少注册一枚（从 `[Operator; 17]` 的数组里删一项 ⇒ **编译不过**，
  故改法取「把某一枚的 id 改成另一枚的 id」或「把某两枚合成一枚且把数组长度改成 16」——
  **后者是编译期的**）：**运行期可观察的改法是把某一枚的 id 串改错**（如 `shot-detect` → `shot_detect`）
  ⇒ 该 id 的 `resolve` 返回 `NotFound`，红。
- `all_operators_has_seventeen_distinct_ids_and_every_version_is_one`：断言 `len() == 17`、
  且 17 个 id **两两不同**（两两比较，不是只查总数）、且每枚 `version == 1`（设计 §5.1 的不变量）。
  **照实记：`len() == 17` 这一半不承重**——`all_operators()` 的返回类型是 `[Operator; 17]`，
  在没有编译错误的前提下它**恒真**（纪律 5 已就「数目由返回类型承载」作过说明；设计 §12.1 也把这条写成
  `len() == 17`）。它是一个**读数**，本条真正承重的是那两两比较与 `version == 1` 两条；
  「清单少了一枚」这一档的红由**返回类型**（编译期）与 (f) 的逐枚 `resolve` 承担，不由这一行承担。
  **红条件（取反档）**：把两枚算子的 id 写成同一个 ⇒ 两两比较红（**注意**：那时 `register` 也会返回 `Duplicate`，
  两条用例会一起红——**这是刻意的**，它说明两处守卫抓手不同）。
- `a_second_identical_registration_returns_the_registry_error`（**第 (g) 条**）：
  同一注册表上注册两次同一批次 ⇒ `Err(MediaError::Registry(OperatorError::Duplicate { .. }))`，
  **且断言那一臂的 `id` / `version` 是撞车的那一枚**（不是「返回了 Err」）。
  **红条件（移除档）**：把 `register_media_operators` 改成静默跳过已存在的算子 ⇒ 第二条返回 `Ok`，红。
- `the_side_effect_class_of_the_five_named_operators_is_asserted_one_by_one`（**第 (n) 条**）：
  `generate-broll` / `frame-interpolation` ⇒ `Pure`；`render` / `proxy-build` / `media-import` ⇒ `Idempotent`。
  **五枚各一条断言，不抽代表**（设计 §5.2 的第 5、6 条理由逐枚给过判据）。
  **红条件（取反档）**：把 `generate-broll` 改回 `NonIdempotent` ⇒ 该枚红。
  **并写明这条守卫的射程**：它钉的是**这五枚**，不是「全部 17 枚的 `side_effect_class` 都对」——
  其余 12 枚的取值**由 (h) 与 (t) 之外的哪条用例都钉不住**，本计划**不发明**那条全称（见 `## 遗留` 第三节）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test operators
```

**预期**：`error: package ID specification `continuum-media` did not match any packages`（crate 还不存在）。
**照实记**：这一步的红是「目标不存在」，**不是用例变红**；可读的第一处红是 Step 3 建好骨架之后的
`--test operators`（它会**编译失败**，直到 `all_operators()` 有 17 项）。

- [ ] **Step 3: 实现**

`crates/continuum-media/Cargo.toml` 的依赖：**只声明本 task 用得到的**——
`continuum-artifact`、`continuum-operator`（都是 path 依赖）、`thiserror`（`MediaError` 派生）。
**`continuum-graph` / `continuum-method` / `continuum-verify` / `trybuild` 由需要它们的 task 增量加**（Task 7／8／9）。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加（数组按字母序）：
```rust
    // P5e 媒体领域算子（设计 §2.1）。本 task 只登记当时实际有的两条边；
    // continuum-graph / continuum-method / continuum-verify 三条分别由需要它们的 task 追加
    // （前两条是 dev 边，理由见各 task：生产代码对它们零调用）。
    (
        "continuum-media",
        &["continuum-artifact", "continuum-operator"],
    ),
```

产出的三个面：
```
/// 本领域 17 枚算子的全集。顺序即设计 §5.2 表的行序。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**，两条理由各自独立成立：本仓带 `ALL` 常量的既有做法
/// （`ArtifactType::ALL`、`PrivacyClass::ALL`）成立的前提是那些类型**无字段**；且
/// `Operator` 含 `OperatorId`（字段私有、`new` 不是 `const fn`）与三枚 `Vec` 字段，
/// 故在 crate 外**构造不出 `Operator` 的常量值**。
/// 数目 17 写在**返回类型**里，使清单个数的改动编译期可见。
pub fn all_operators() -> [Operator; 17];

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// 一枚媒体算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§5.4 那条规则要能对**任意** `Operator` 触发。
pub fn register_media_operators(registry: &mut OperatorRegistry, operators: &[Operator]) -> Result<(), MediaError>;

/// 本块的错误类型。三臂见 Task 4／5。
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1。
    /// **这一臂是包装不是同名臂**：撞车的判定已经有一枚臂（`OperatorError::Duplicate`），
    /// 本块再立一枚 `MediaError::Duplicate` 就是「注册表里有没有这个算子」的第二套词汇表。
    #[error(transparent)]
    Registry(#[from] OperatorError),
}
```
**「先核后写」在 Task 4 才落地**（那时才有可核的规则）；本 task 的 `register_media_operators`
只做「逐枚 `register`、任一失败即返回」。

**设计 §5.5 的两条声明的落点（本小节交付它们；`src/operators.rs` 里那两枚算子值各带一段文档注释）**：
第 16 行 `verify-deterministic` 与第 17 行 `verify-multimodal` 的 `Operator` 值上，
各逐字抄设计 §5.5 的那两句——「`verify-deterministic` 在 §262 的序里是第 1 级（`DeterministicChecker`）、
`verify-multimodal` 是第 3 级（`IndependentModelVerifier`）」——并写明两件事：
（i）**这是本设计的声明**（§32 只说这两步在链尾，没说级别）；
（ii）**「这一对应由谁持有」未定**（设计 §13 第 5 条 (ii)：装配方以值构造 `AvailableVerifier`，
`level` 取本设计声明的那一级）。
**据实记**：`Operator` 的七字段里没有级别，故这两条**只有文档注释这一个载体、没有运行期照片**
（同纪律 8 的「纯数据声明」；列进 `## 遗留` 第五节）。**它仍是交付物**：设计 §9.3 逐字
「本块的消费止于「两枚验证算子的级别声明」（§5.5）」，`## 跨计划前置` 第四节「交给 P5a」那一笔即指此处。
**不建 `AvailableVerifier`、不给 `Operator` 加字段**（设计 §5.5 的归属表：级别序与候选过滤属 P5a）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(media): 建 continuum-media 骨架、17 枚算子清单与注册入口"
```

**门读数**：`ALLOWED` 与实际依赖必须**精确一致**（逐对 `assert_eq!`）；新 crate 会先让该门变红再被补齐，**这是刻意的**。

---

### Task 4: §5.4 的确定性 backend 名单（注册期，两侧都钉）与 §5.3 的端口边表

**Files:**
- Modify: `crates/continuum-media/src/operators.rs`（名单常量与「先核后写」）
- Modify: `crates/continuum-media/src/error.rs`（加 `UnverifiableDeterminism` 臂）
- Modify: `crates/continuum-media/src/lib.rs`（导出）
- Create: `crates/continuum-media/tests/determinism.rs`
- Create: `crates/continuum-media/tests/ports.rs`

**Interfaces:**
- Consumes: Task 3 的 `all_operators()` / `register_media_operators` / `MediaError`
- Produces: `MediaError::UnverifiableDeterminism { operator: OperatorId, backend: BackendId }`

- [ ] **Step 1: 写用例**

`tests/determinism.rs`：

- `every_deterministic_operator_declares_only_bit_reproducible_backends`（**第 (h) 条，正例侧**）：
  遍历 `all_operators()` 里 `determinism == Deterministic` 的**每一枚**，逐枚核它的**每一个** backend 在名单内。
  **红条件（取反档）**：给 `render` 的候选里加 `nvenc`（硬件编码器）⇒ 遍历到它时红。
  **偏离最显然写法处（写进注释）**：最显然的写法是给 `render` 放一个硬件编码器（性能更好）；
  **这里不是那样：因为硬件编码器的输出不保证与软件编码器逐位一致**，混入即让该算子的 `Deterministic`
  声明为假，而那条声明的消费者是 §305 的复用。代价是硬件加速不可用——**性能代价，不是正确性代价**。
  **并写明这条规则的规范来源：没有。** §245 只给「一个 Operator 多 backend」，不给 backend 的确定性；
  名单本身是本设计定的（设计 §14 第 7 条）。
- `an_out_of_list_backend_is_rejected_before_anything_is_written`（**第 (i) 条，fail-open 的反面**）：
  两半，**逐半一条断言**：
  （i）造一枚 `Deterministic` 但候选含名单外 backend 的算子（`Operator` 七字段全 `pub`，
  可以直接写字面量构造，`crates/continuum-operator/src/definition.rs:79-88`），
  连同若干**合法**算子调 `register_media_operators` ⇒
  `Err(MediaError::UnverifiableDeterminism { operator, backend })`，**并断言 `operator` 与 `backend`
  正是那一枚**（不是「返回了那一臂」）；
  （ii）**注册表的内容与调用前逐枚相同**：调用前先注册一个合法批次并记下每枚的 `resolve` 结果，
  违规批次之后再逐枚 `resolve` 一次，断言与调用前**逐枚相同**（先核后写，不留半注册）。
  **红条件（取反档）**：（i）把名单判定写成恒真 ⇒ 返回 `Ok`，红；
  （ii）把实现改成**边核边写**（先注册前几枚，遇到违规才返回）⇒ 注册表里多出前几枚，红。
  **这两半是「同一处规则的写入面」**：只钉返回值那一半会让「半注册」漂过去。

`tests/ports.rs`：

- `every_downstream_input_type_has_a_declared_upstream_source`（**第 (t) 条**）：
  **语料手写**（设计 §5.3 的 17 行边表：下游算子 ← 输入的来源（交集中的型）），逐行断言
  「下游 `input_schema` 的**每一个型**都能在该行列出的上游算子里找到，且所引的那一枚的 `output_schema` 含该型」。
  **断言的对象是「交集中的型」，不是「两侧相等」**（设计 §5.3 改判后的口径：多数相邻算子的 schema 只相交、不相等）。
  **红条件（取反档）**：把 `transcribe` 的 `input_schema` 从 `[Audio]` 改成 `[Video]` ⇒ 第 4 行红
  （上游 `media-import` 的 `Audio` 对不上）；把 `verify-multimodal` 的 `SubtitleTrack` 去掉 ⇒
  第 17 行的 `effects-subtitle` 那一支红。
  **并写明这张表的射程**：它钉的是**设计声明的那 17 行关系**，**不是「所有可能的连接都合法」**；
  §5.3 末已写「本表不是全图」——两枚生成算子的产物回流进 `timeline-compose` 之类的**其他**边属装配方的图构造。
  **逐行、不抽代表**：17 行各一条断言（可以用表驱动，但**每一行都要有一条会红的对照**）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test determinism
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test ports
```

**预期**：两条**编译失败**（`MediaError::UnverifiableDeterminism` 不存在）——**照实记为编译期**。
**`tests/ports.rs` 在 Task 3 的实现下应当直接绿**（17 行的端口取值已在 Task 3 写对）——
**这一步要照实记**：**「ports 没有先跑红」不是缺陷**，它是同一批声明的两条照片；
若它红，说明 Task 3 的端口取值抄错了（那时改 Task 3 的取值，不改这张表）。

- [ ] **Step 3: 实现**

名单常量（**封闭表**，私有，内容照设计 §5.4 逐名照抄、照抄前对源）：
`local-fs` `ffmpeg` `ffmpeg-scene-detect` `ffmpeg-filtergraph` `ffprobe` `pyscenedetect` `librosa` `essentia` `builtin`。

```
/// §5.4 的注册期规则：声明 `Deterministic` 的算子，其每一个候选 backend 都必须在本 crate 的名单内。
/// **先核后写**：先逐枚核，全部通过后再逐枚注册；任一枚不成即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 本规则**没有规范来源**（§245 只给「一个 Operator 多 backend」，不给 backend 的确定性）；
/// 名单是本块定的，缺口记在设计 §14 第 7 条。
fn verify_deterministic_backends(operators: &[Operator]) -> Result<(), MediaError>;
```

**偏离最显然写法处（写进注释）**：最显然的写法是把 `verify` 与 `register` 合成一趟循环（少一次遍历）；
**这里不是那样：因为「先核后写」那条性质的唯一可观察后果就是「违规时注册表没被动过」，
合成一趟之后它当场为假**（Task 4 的 (i) 第二半）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media
git commit -m "feat(media): §5.4 的逐位可复现 backend 注册期规则与 §5.3 的端口边表断言"
```

---

### Task 5: §34 的生成内容单独授权（五臂、四条映射、fail-closed）

**Files:**
- Create: `crates/continuum-media/src/permission.rs`
- Create: `crates/continuum-media/tests/permission.rs`
- Modify: `crates/continuum-media/src/{lib.rs,error.rs}`（导出；加 `NotPermitted` 臂）

**Interfaces:**
- Consumes: `continuum_operator::OperatorId`、Task 3 的 `MediaError`
- Produces: `continuum_media::{GenerativePermission, PermittedGenerativeContent, required_permission, authorize_generative}`、
  `MediaError::NotPermitted { operator: OperatorId, required: GenerativePermission }`

- [ ] **Step 1: 写用例**

- `the_five_flags_are_exactly_the_five_from_section_34`（**§12.1 的臂数与往返**）：
  `GenerativePermission::ALL.len() == 5`，**逐臂** `parse(as_str(p)) == Some(p)`，并**逐臂断言串的字面值**
  （`broll` / `voice` / `music` / `frame_interpolation` / `three_d_generation`——**串的取值是本计划定的**
  （设计只给了类型名与五个旗标的规范名 `allow_generated_broll` 一类，没给 `as_str` 的串），见 `## 遗留` 第三节）。
  **红条件（取反档）**：把 `Music` 与 `Voice` 的串**在 `as_str` 与 `parse` 两处一致地**互换 ⇒ 逐臂的字面值断言红。
  **据实记**：这一档**只断往返时不会红**（两处一致地互换是自洽的重编码），故承重的是字面值那一条；
  **只在 `as_str` 一处改则 `parse(as_str(p)) == Some(p)` 当场为假、往返即红**——两档不是同一枚变异体
  （口径见纪律 1 的第三枚）。
  **`ThreeDGeneration` 这一枚旗标的地位要写进注释**：§34 的旗标与算子不是一一对应；**本块不给它编算子**
  （那会落进切分 §八「`3d/` 无对家」那条无解）；**这不是死臂**——一个 Contract 声明 `allow_3d_generation`
  而系统里没有 3D 生成算子，是**系统不提供该能力**。
- `each_of_the_four_generative_operators_requires_its_own_flag`（**第 (l) 条，四臂各一条**）：
  `required_permission` 对四枚生成算子**逐枚断言是哪一枚**（`generate-broll`→`Broll`；`generate-voice`→`Voice`；
  `generate-music`→`Music`；`frame-interpolation`→`FrameInterpolation`），**不是断言「是 `Some`」**。
  **红条件（收紧档）**：让四枚**都**返回 `Broll` ⇒ 后三条红。
- `a_non_generative_operator_requires_no_flag`（**第 (m) 条，另一侧**）：
  对若干非生成算子（例如 `render` / `transcribe` / `verify-deterministic`）返回 `None`。
  **红条件（放宽档）**：让 `required_permission` 一律返回 `Some(Broll)` ⇒ 红。
- `an_ungranted_generative_operator_is_rejected`（**第 (j) 条，反例侧**）：
  未声明 `Broll` 时 `authorize_generative(&generate_broll, &granted) == Err(MediaError::NotPermitted { .. })`，
  **且断言 `required` 是 `Broll`**。**红条件（取反档）**：把判定改成「未声明 ⇒ 放行」⇒ 红。
- `a_granted_generative_operator_is_allowed`（**第 (k) 条，另一侧**）：
  声明为真时 `Ok`。**没有这一条，第 (j) 条可以由「一律 `Err`」满足。**
- `the_empty_grant_set_permits_nothing`（**fail-closed 的那一侧，四枚逐枚一条**）：
  **用本 crate 提供的「空集」构造**得到的 `PermittedGenerativeContent`，对四枚生成算子**逐枚** `Err`。
  **这一条钉的是「缺省不放行」**：设计 §7.2 逐字「**这里不是「用 `unwrap_or(true)` 让缺省放行」**：
  §34 `:1431` 逐字是「不得擅自」，而 `PermittedGenerativeContent` 的**缺省构造**若是「全放行」，
  那么一个没写旗标的 Contract 就自动获得全部生成权限——那正是「擅自」的形态」。
  **红条件（取反档）**：把「空集」构造实现成全放行（或给它一个全放行的 `Default`）⇒ 四条全红。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test permission
```

**预期**：编译失败（两个类型与两个函数还不存在）。**照实记为编译期红点**。

- [ ] **Step 3: 实现**

```
/// §34 的五个旗标（`docs/spec/01-concepts.md:1424-1428`），逐条照录。
/// Debug：`MediaError` 自身 derive(Debug)，且其 `NotPermitted` 臂以 `{required:?}` 引用本型。
/// **比设计 §7.1 多三项派生**（设计只给 `#[derive(Debug)]`）：`PartialEq` 是 §12.1 的逐臂往返断言
/// （`parse(as_str(p)) == Some(p)`）所需的值比较；`Clone` / `Eq` 照本仓枚举措辞的既有做法
/// （`crates/continuum-operator/src/definition.rs:7`、`:15` 的两枚枚举同形）。
/// **不取 `Copy`**：设计未把本型定成可复制值，而 `contains` 按值收一枚即可。
/// **这是本计划自定的外部面，不是照抄设计**（申报见 `## 遗留` 第三节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerativePermission { Broll, Voice, Music, FrameInterpolation, ThreeDGeneration }

/// 臂数与编码的守卫：`ALL` 与两个穷尽 `match`，形状照 `ArtifactType` 的既有做法。
impl GenerativePermission {
    pub const ALL: [GenerativePermission; 5];
    pub fn as_str(&self) -> &'static str;      // 穷尽 match，无通配臂
    pub fn parse(s: &str) -> Option<Self>;     // 从 &str 出发，有 `_ => None`
}

/// Contract 里**已声明为真**的旗标集合。以**值**从 Contract 传入（P4 的规则：凡跨层输入以值传入）。
/// **没有 `Default`**：本型的「空集」由一个具名构造函数给出，理由是 §34 的 fail-closed——
/// 一个默认构造是全放行的类型，会让「没写旗标的 Contract」自动获得全部生成权限。
pub struct PermittedGenerativeContent { /* 一份已授权旗标的集合 */ }

impl PermittedGenerativeContent {
    /// 空集：什么都不放行。
    pub fn none() -> Self;
    /// 逐枚给定已授权的旗标。
    pub fn from_permissions(permissions: &[GenerativePermission]) -> Self;
    pub fn contains(&self, permission: GenerativePermission) -> bool;
}

/// 本领域的生成类算子各自需要哪一枚旗标。`None` 表示该算子不是生成类（不触 §34 的约束）。
pub fn required_permission(operator: &OperatorId) -> Option<GenerativePermission>;

/// §34「系统不得擅自使用生成内容替换原始素材」的判定点。
/// 未声明 ⇒ 未授权（fail-closed），返回 `MediaError::NotPermitted { operator, required }`。
pub fn authorize_generative(operator: &OperatorId, granted: &PermittedGenerativeContent) -> Result<(), MediaError>;
```

`MediaError::NotPermitted` 的臂（格式串用 `{operator:?}` 与 `{required:?}`，**故两枚都只需 `Debug`**）：
```
/// §34 的生成授权判定不成立：该算子所需的那一枚旗标未在 Contract 里声明为真（§7.2）。
#[error("算子 {operator:?} 未获 {required:?} 授权（§34）")]
NotPermitted { operator: OperatorId, required: GenerativePermission },
```

**调用点不在本块（写进注释）**：谁在 `Queued → Running` 前调 `authorize_generative` 属第 3 层执行器
（切分 §四第 4 条）。**据实记录**：它在 P5e 落地后**没有生产调用方**，与 Task 6 的 `checkpoint()` 同形。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media
git commit -m "feat(media): §34 的生成内容单独授权（五旗标、四条映射、fail-closed）"
```

---

### Task 6: `Checkpointable` 的第一枚实现——`generate-broll` 与 `render`

**Files:**
- Create: `crates/continuum-media/src/checkpoint.rs`
- Create: `crates/continuum-media/tests/checkpoint.rs`
- Modify: `crates/continuum-media/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 2 的 `continuum_operator::{Checkpointable, CheckpointError, CheckpointOwner}`、
  `OperatorId` / `OperatorVersion` / `BackendId`；Task 3 的 `all_operators()`
- Produces: `continuum_media::{GenerateBroll, Render}` 两枚类型与它们的 `Checkpointable` 实现，
  **以及三条判据的用例必须能用到的读法／写法**（本计划自定，申报见 `## 遗留` 第三节）：

  ```
  /// 尚无进度、尚未选定 backend 的初值。
  pub fn new() -> Self;
  /// 与该类型对应的那一枚算子的 id（见下「与 all_operators() 的关系」）。
  pub fn operator_id(&self) -> OperatorId;
  /// 推进进度：记下第 index 个片段已完成（判据 1 的「有进度」由它造出）。
  pub fn record_completed_segment(&mut self, index: u32);
  /// 指定本次要用的 backend（判据 3 的两侧由它造出）。
  pub fn select_backend(&mut self, backend: BackendId);
  /// 读回当前进度（判据 1／2 的「逐字段相同」以它取存前的值）。
  pub fn progress(&self) -> &Progress;
  ```

  **`type Checkpoint` 取 `Progress`**（本计划取这一枚：快照就是进度值本身）。`Progress` 是一枚
  **具名结构体、字段全 `pub`**——判据 1／2 要求「逐字段相同」写成**穷尽解构、不带 `..`**，
  而这只有在字段可见、且是具名结构体（不是元组、不是 `u32`）时才写得出（纪律 8）。
  它**自带身份**（身份字段取 Task 2 的 `CheckpointOwner`，即 `OperatorId` ＋ `OperatorVersion`
  两字段，设计 §4.6：`RestoreRejected` 的判据要求检查点携带身份），故「用另一版本的检查点去 `restore`」
  这条用例可以**直接构造**一枚身份不同的 `Progress`，不必依赖任何绕过路径。

  **「选中的 backend 支不支持检查点」的判据（判据 3 两侧的落点，本计划自定）**：
  **选中的 backend 在该算子 `all_operators()` 那枚 `Operator` 值的 `backend_candidates` 内 ⇒ 支持；
  否则不支持。** 不另立第二份支持名单——§5.4 的九枚名单判的是 `determinism`，不是 checkpoint 能力，
  混用就是「同一件事两套词汇表」。照此写用例时（候选取值见设计 §5.2 第 11、12 行）：
  `render` 的候选是 `ffmpeg` ⇒ `select_backend(ffmpeg)` 后 `checkpoint()` 为 `Ok`；
  `generate-broll` 的候选是 `video-gen-backend` ⇒ 同；`select_backend(BackendId::new("nvenc"))`
  （17 枚的候选里没有它）⇒ `Err(CheckpointError::UnsupportedByBackend { backend })`，
  **且断言 `backend` 正是选中的那一枚**。

> **本计划自定的形状（设计只给了「哪两枚算子」与三条判据，没给类型名、没给这组读写面、
> 也没给它们与 `all_operators()` 的关系）**：
> 两枚类型各持自己的**进度状态**，各有一个 `operator_id() -> OperatorId` 使它与 Task 3 的清单对得上
> （一个 `OperatorId`，比对成本最低）；三条判据要用的读写面即上面那五个方法，
> 缺了它们「推进进度」「指定 backend」两处**在用例里写不出来**。
> 落点与代价见 `## 遗留` 第三节；**收件人：设计作者**。

- [ ] **Step 1: 写用例**

`tests/checkpoint.rs`（**判据三组，逐组两边，照设计 §4.5**）：

- `a_checkpoint_restores_the_progress_it_recorded`（**判据 1 的正例**）：
  以 `record_completed_segment` 推进进度到「非空」（读法／写法见 `Interfaces`）之后 `checkpoint()` 返回 `Ok`，
  再 `restore` 该检查点，断言**算子的进度与存前逐字段相同**（以 `progress()` 取存前的值，穷尽解构，不带 `..`）。
  **红条件（移除档）**：把 `restore` 写成 no-op ⇒ 恢复后进度与存前不同，红。
- `checkpointing_without_progress_is_rejected`（**判据 1 的反例**）：
  尚无进度时 `checkpoint()` ⇒ `Err(CheckpointError::NothingToCheckpoint { owner })`，
  **并断言 `owner` 的 id 与 version 是这一枚算子**。
  **红条件（取反档）**：返一个「空进度」的检查点并给 `Ok` ⇒ 红。
- `restoring_another_operators_checkpoint_is_rejected_and_leaves_state_unchanged`（**判据 2，两侧**）：
  **本用例用 `Render` 那一枚实现**（Step 3 有两枚 `impl Checkpointable`，此处写死是哪一枚；
  Task 10 Step 5 第 4 轮的变异也据此落到同一枚上）。
  用**另一版本**（`OperatorVersion` 不同）的同一算子的检查点去 `restore` ⇒ `Err(CheckpointError::RestoreRejected { .. })`，
  **且恢复失败后算子状态与调用前逐字段相同**（半恢复比不恢复更坏）。
  **红条件（移除档）**：删掉身份比对（一律 `Ok`）⇒ 第一个子例红（返回 `Ok`）。
  **两侧的另一半**：同一版本、同一算子的检查点 `restore` ⇒ `Ok`（**否则「一律 `Err`」也能满足前一半**）。
- `an_unsupported_backend_is_rejected_and_a_supported_one_succeeds`（**判据 3，两侧**）：
  以 `select_backend` 指定 backend 之后，选中的那一个不支持检查点时（§308 是 **SHOULD**，§245 允同一个
  Operator 有多个 backend，故「不支持」是一个合法答案，且它的主语是**选中的那个 backend**）
  ⇒ `Err(CheckpointError::UnsupportedByBackend { backend })`，**并断言 `backend` 正是选中的那一枚**；
  支持时 ⇒ `Ok`。**两侧各一条**；照 `Interfaces` 里那条判据取值（候选在 `backend_candidates` 内 ⇒ 支持）。
  **红条件（取反档）**：把不支持的后端也放行为 `Ok` ⇒ 红（这正是「假装检查点成功」的形态）。
- `the_two_implementations_are_the_two_named_operators`：两枚类型的 `operator_id()` 与
  `all_operators()` 里 `generate-broll` / `render` 的 `id` 相同（**逐枚**），
  **且它们确实实现了 `Checkpointable`**（照片机制：在本 crate 内以 `&dyn` 或泛型 `T: Checkpointable` 调一次两个方法）。
  **红条件（取反档）**：把 `render` 的 `operator_id()` 改成另一枚算子的 id ⇒ 红。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test checkpoint
```

**预期**：编译失败（两枚类型还不存在）。**照实记为编译期红点**；
**可读的运行期红点在 Step 3 之后仍在**（三条判据的每一侧各有一条会被下面列出的变异打红的断言）。

- [ ] **Step 3: 实现**

两枚类型各定义自己的 `type Checkpoint`（**字段清单不在设计的约束范围内**：它随算子的实现走；
本计划取「已完成的片段区间 ＋ 已确定的后端选择」一类的最小进度状态，**且让检查点自带身份**
（设计 §4.6：`RestoreRejected` 的判据要求检查点**携带**身份，而类型层拦不住——**这条义务靠实现者的用例钉**）。

```
/// §308 点名的第一项长任务（video generation）的检查点实现。
pub struct GenerateBroll { /* 进度状态 ＋ 选中的 backend */ }
impl Checkpointable for GenerateBroll { type Checkpoint = Progress; … }

/// 同一条 SHOULD 下的另一枚长算子（设计 §4.5：§308 的「尤其」清单是举例，不是穷尽清单）。
pub struct Render { /* 进度状态 ＋ 选中的 backend */ }
impl Checkpointable for Render { type Checkpoint = Progress; … }

/// 检查点的负载：已完成的片段区间 ＋ 选中的 backend ＋ 身份（`CheckpointOwner`）。
/// 字段全 `pub`：判据 1／2 的「逐字段相同」要能写成穷尽解构（不带 `..`）。
pub struct Progress { /* … */ }
```

**上面那五个方法与 `Progress` 的字段清单是本任务必须落出来的读写面**（`Interfaces` 一块），
三条判据的用例全靠它们写；其余实现细节（`checkpoint()` 内部怎样快照、片段区间用什么类型表示）随实现走。

**调用点不在本块（写进注释）**：谁在什么时候调 `checkpoint()` / `restore()` 规范未给，属第 3 层的恢复路径
（§311 / §312 那一组）。故「无调用点」这件事在 P5e 落地之后**仍然成立**，
只是从「无实现、无调用点、无测试」变成「**有实现、有测试、无生产调用点**」。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media
git commit -m "feat(media): Checkpointable 的第一枚实现（generate-broll 与 render）"
```

---

### Task 7: §6.2 的复用照片——**不建第二份判据**

**Files:**
- Create: `crates/continuum-media/tests/reuse.rs`
- Modify: `crates/continuum-media/Cargo.toml`（`[dev-dependencies]` 加 `continuum-graph`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 的 `continuum-media` 条目追加 `"continuum-graph"`，**dev 边**）

**Interfaces:**
- Consumes: Task 3 的 `all_operators()`、`continuum_graph::{cache_key, can_reuse, CacheKey}`、`continuum_artifact::ContentHash`
- Produces: 无（**本 task 只交付一条用例**——它钉的是「本块声明的 `determinism` 真能让 §305 的判据成立」）

> **为什么本 task 只有用例**：切分 §四第 7 条（判据只许有一处）——§305 的三项条件**已由 P1 落地**
> （`crates/continuum-graph/src/reuse.rs:29` 的 `can_reuse`），故本块**不得**另写一个「原素材未变 ⇒ 复用」的函数，
> **只让本领域算子满足那条判据的前提**。失效方向：两份判据并存且各有用例、各自都绿时，
> 失配是「一套说能复用、另一套说不能」——而这种不一致**不会在任何一条用例里显形**。

- [ ] **Step 1: 写用例**

- `the_six_derived_artifact_producers_have_the_declared_reuse_outcome`（**第 (r) 条，六行逐行**）：
  **语料手写**（设计 §6.2 的表：`shot-detect` / `audio-analysis` / `metadata-analysis` / `proxy-build` ⇒
  `cache_key` 为 `Some` 且四条件齐时 `can_reuse` 为真；`transcribe` / `vision-analysis` ⇒ `cache_key` 为 `None`），
  **逐行一条断言、不抽样**。
  **红条件（取反档）**：把 `shot-detect` 的 `determinism` 改成 `NonDeterministic` ⇒ 该行的 `cache_key` 由 `Some` 变 `None`，红。
  **并写明两侧代价**（设计 §6.3）：`transcribe` / `vision-analysis` 取 `NonDeterministic` 的代价是
  §131 对这三类产物（`Transcript` / `FaceIndex` / `Embeddings`）的「无需重新计算」**不成立**；
  反过来的代价是**静默地让一份不同的结果冒充这次的结果**，且它发生在「输入没变」这个最不容易被怀疑的条件下。
  取向的理由是 §305 自己的措辞是 **MAY**（允许复用），而 §131 的「无需」是定位文档里的成本陈述。
- `the_contract_condition_is_the_fourth_thing_can_reuse_checks`（**另一侧**）：
  同一枚算子、同一输入哈希、同一版本，但 `contract_affected == true` ⇒ `can_reuse` 为假。
  **红条件（移除档）**：把 `contract_affected` 那一支删掉（那是 P1 的代码，**本 task 不改**）——
  故**这一条的作用是留下「本块声明的 determinism 不是复用的全部条件」的读数**，
  **不是本块要建的第二份判据**。**照实写成「本 task 只核、不改 P1 的那一支」。**
  **照片坐标写清**：本 task 的 `continuum-graph` 是 **dev 边**（生产代码不调它），
  这条边的使用点就是本文件（另见设计 §2.1 第 2 行）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test reuse
```

**预期**：**这一步可能直接绿**（Task 3 已把 `determinism` 写对）。
**照实记这一点**：本条不是「先假后真」型的用例，它是**声明与既有判据之间的一致性照片**；
**它的价值在变异那一轮显现**（Step 3 列出的两处变异）。

- [ ] **Step 3: 实现（只有一条：把 dev 边与用例接起来）**

`Cargo.toml` 的 `[dev-dependencies]` 加 `continuum-graph = { path = "../continuum-graph" }`；
`ALLOWED` 的 `continuum-media` 条目改成
`&["continuum-artifact", "continuum-graph", "continuum-operator"]`（**字母序**），
**并把 dev 边的理由写进注释**：`cargo tree` 带 `--edges all`（`dependency_direction.rs:10`），dev 边与普通边一视同仁；
本仓已有同形先例（`continuum-provider` 那一行注记「Task 4 起加上 persist：**dev 边**」，`:32-34`）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "test(media): §305 复用的六行照片（dev 边上核对 determinism 声明）"
```

---

### Task 8: `bind_media_methods`——四条 `video/` 的 `realized_by`

> **硬前置：P5b 的 `continuum-method` 已建**（见「跨计划前置」第二节）。**该 crate 今天不存在，
> 本 task 在此之前编不过。**

**Files:**
- Create: `crates/continuum-media/src/methods.rs`
- Create: `crates/continuum-media/tests/methods.rs`
- Modify: `crates/continuum-media/src/lib.rs`、`crates/continuum-media/Cargo.toml`（普通依赖 `continuum-method`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 追加 `"continuum-method"`）

**Interfaces:**
- Consumes: `continuum_method::{MethodRegistry, MethodId, MethodError, MethodDomain, MethodEntry}`、
  Task 3 的 `all_operators()` 与 `continuum_operator::OperatorRegistry`
  （`MethodDomain` 与 `MethodEntry` 是本 task 的用例要用的：`registry.select(MethodDomain::Video)`
  与逐条读 `MethodEntry::realized_by`；`MethodRegistry::new()` 是失败路径那条用例的夹具）
- Produces: `continuum_media::bind_media_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>`

- [ ] **Step 1: 写用例**

- `every_video_method_gets_at_least_one_realized_by_and_each_string_resolves`（**第 (s) 条，两条腿**）：
  （i）先 `MethodRegistry::seeded()`，再 `bind_media_methods(&mut registry)`（**必须返回 `Ok`**），
  然后对四条 `video/` 的 id（`shot-analysis` / `narrative-plan` / `timeline-compose` / `render-review`）
  **逐条**断言它的 `realized_by` **非空**；
  （ii）对每一条 `realized_by` 里的**每一个串**，`OperatorRegistry`（经 `register_media_operators` 装满 17 枚之后）
  `resolve` **必须成功**。
  **红条件（放宽档）**：把 `render-review` 的 `realized_by` 写成空 `Vec` ⇒（i）红。
  **红条件（取反档）**：把某一枚的 `realized_by` 里写一个非算子的串（如 `"render-review"` 自己）⇒（ii）红。
  **并写明这条判据的限度**（P5b 设计 §3.2 末）：`realized_by` 是**文本弱引用**，**没有编译期照片**；
  本条是运行期的补件——它是 P5b 所说「消费块才能核」的那个消费块之一（本块同时持有 `all_operators()` 与 `MethodRegistry`）。
- `binding_does_not_add_or_remove_any_method_entry`（**另一侧**）：
  `bind_media_methods` 调用前后，`registry.select(MethodDomain::Video)` 的**条目数相同**（四条）。
  **红条件（取反档）**：让 `bind_media_methods` 内部顺手 `register` 一条新条目 ⇒ 数量变化，红。
  **这一条钉的是 §10 的「本函数不做二次登记」**（四块**不得**自造 `MethodEntry`）。
- `binding_an_unseeded_id_returns_not_found`（**失败路径**）：
  对一张**没有该条目**的注册表（`MethodRegistry::new()`）调 `bind_media_methods` ⇒
  `Err(MethodError::NotFound { .. })`，**且断言是哪一条 id 未找到**（不是「返回了 Err」）。
  **错误类型不并进 `MediaError`**：本函数的失败是「方法库拒绝了这次 bind」，判定、收件人与处置都不同，
  **故直接透出**——这与 §5.1 把 `OperatorError::Duplicate` 包进 `MediaError::Registry` 是同一条判据的两面：
  **判定是同一件事则合并，是两件事则分开。**

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test methods
```

**预期**：编译失败（`continuum_method` 无法解析）。**照实记为编译期**。

- [ ] **Step 3: 实现**

```
/// 把四条 `video/` 方法的 `realized_by` 填进方法库（P5b 设计第六节：每块对其领域目录里的**每一条已 seed 的 id**
/// 调用一次 `bind(id, &[...])`）。`video/` 四条逐条非空。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
pub fn bind_media_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;
```

四条映射（**逐条照设计 §10 的表**，落地时对源）：

| `MethodId` | `realized_by` |
|---|---|
| `shot-analysis` | `["shot-detect", "vision-analysis"]` |
| `narrative-plan` | `["narrative-plan"]` |
| `timeline-compose` | `["timeline-compose"]` |
| `render-review` | `["verify-deterministic", "verify-multimodal"]` |

**实现只调 `bind`**：不 `register` 方法条目、不构造 `MethodEntry`、**不给 §187 补条目**。
**§33 的七枚预处理算子没有目录可填**（§187 的 `video/` 只有四条）：`proxy-build` / `audio-analysis` /
`metadata-analysis` 等**不在任何方法的 `realized_by` 里**，**这不是漏 bind**——**§33 与 §187 的 `video/` 不是同一张表**
（前者是链上的步骤，后者是方法名）。**这一句要写进注释**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(media): bind_media_methods——四条 video/ 方法的 realized_by"
```

---

### Task 9: 两条结构层的照片（`trybuild`）

> **硬前置：P5a 的 `continuum-verify` 已建**（见「跨计划前置」第二节）。**该 crate 今天不存在。**

**Files:**
- Create: `crates/continuum-media/tests/type_level.rs`
- Create: `crates/continuum-media/tests/compile_fail/{evidence_fields_are_private,evidence_subject_requires_a_named_requirement}.rs` 与同名 `.stderr`
- Create: `crates/continuum-media/tests/compile_pass/evidence_type_is_nameable.rs`
- Modify: `crates/continuum-media/Cargo.toml`（`[dev-dependencies]` 加 `continuum-verify`、`trybuild`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 追加 `"continuum-verify"`）

**Interfaces:**
- Consumes: `continuum_verify::{Evidence, EvidenceSubject}`
- Produces: 无（两条**证不到「产出只能经本块的某枚函数」**的结构照片，见下）

- [ ] **Step 1: 写用例**

`tests/type_level.rs`（形状照本仓既有做法：`crates/continuum-connector/tests/type_level.rs`）：
```
#[test]
fn the_type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    t.pass("tests/compile_pass/*.rs");
}
```

1. `evidence_fields_are_private.rs`：在本 crate 里写 `Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现）。
   **这一条是 P5a 设计 §4.4「唯一产生点」那条结构事实在本块的**使用侧**照片**
   （实测该段在 `docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md:324`；
   **本块的设计把它引成 `:319`**——本计划按实测的 `:324` 引，两处差 5 行）。
   **它证不到「产出只能经本块的某枚函数」**：`Evidence::from_tool_result` 是 `pub`，
   构造由**宿主**直接调它完成（设计 §9.2），**调用点不在本 crate 里**。
2. `evidence_subject_requires_a_named_requirement.rs`：在本 crate 里构造 `EvidenceSubject::Requirement(..)`。
   **这一条的成立依赖 P5a 不 re-export `RequirementId`**——该类型的归属 P5a 自记未定
   （其设计 §4.3.1：「落地时以 P4 的 `RequirementId` 为准」），故它是**对别人块形状的假设**。
   **据实写进注释**：**若 P5a 的下一轮把 `RequirementId` re-export 出来，这条用例当场不成立、须换掉**；
   **判据仍在**（本块不构造 `Requirement` 归属），只是照片从「编译不过」变成别的形式。

**正控制（不得自证）**：`tests/compile_pass/evidence_type_is_nameable.rs` 断言 `Evidence` 这个**类型名可命名**
（例如一个只接收 `Option<continuum_verify::Evidence>` 的函数签名），**证明第 1 条的失败不是因为「本 crate 里根本写不出这个名字」**。
**语料不从被测清单里取**：正控制的语料是**名字本身**，不是被断言那两条样例的任何一个片段。

**两条坐标（写进 `tests/type_level.rs` 的文件头注释）**：
（i）**配对按文件主干**——`<stem>.rs` 与 `<stem>.stderr`；
（ii）**样例里的注释也是照片坐标**——`.stderr` 记的是 `文件:行:列`，样例里增删一行注释就会让行号漂。
**两份 `.stderr` 一律取自实跑**（`TRYBUILD=overwrite` 生成之后逐份读一遍，确认它失败在**预期的那一条**上——
否则「因为拼错函数名而编译失败」也会让用例变绿）。**不许凭记忆写，不许拿一个值套两处。**

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-media --test type_level
```

**预期**：编译失败（`continuum_verify` 无法解析）。**照实记为编译期**。
**第一次跑之后**：`TRYBUILD=overwrite` 生成两份 `.stderr`，再跑一次确认绿；**两次的读数都要记进报告**。

- [ ] **Step 3: 实现（只有依赖与清单）**

`Cargo.toml` 的 `[dev-dependencies]` 加 `continuum-verify = { path = "../continuum-verify" }` 与 `trybuild`。
`ALLOWED` 的 `continuum-media` 条目改成
`&["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify"]`（**字母序**），
**注记写明两件事**：两条 **dev 边**的理由（生产代码对 `continuum-verify` 与 `continuum-graph` 零调用，
唯一使用点是结构层与 §6.2 的用例）；以及**层内边**在本仓有同形处置
（`continuum-provider → continuum-capability` 是层内边，`dependency_direction.rs:30` 那一行的注记）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-media crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "test(media): 两条结构层照片（Evidence 不可构造、归属不可命名）"
```

---

### Task 10: 收尾与复核

**Files:**
- Modify: `crates/continuum-media/**`（**仅在复核发现缺口时**）

- [ ] **Step 1: 全量验证与门读数**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5e-final-test.log
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets 2>&1 | tee .tmp/p5e-final-build.log
grep -c "^ *Running " .tmp/p5e-final-test.log
grep -c "^test result:" .tmp/p5e-final-test.log
```

**判据（纪律 3）**：两条读数的**数目相等**，且与当时的 test target 数一致（**Doc-tests 各算一条**）；
**不许只看退出码**。预期：全绿、0 warning。

- [ ] **Step 2: 依赖方向的机检**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

**逐条核四件事**：（a）`continuum-media` 的条目**恰为五条**，与 `Cargo.toml` 精确一致；
（b）其中 `continuum-graph` 与 `continuum-verify` 是 **dev 边**（注释里写明理由），
且**生产代码对它们零调用**（复核方式：`grep -rn "continuum_graph\|continuum_verify" crates/continuum-media/src`——**预期零命中**）；
（c）**没有一条指向 `continuum-port` / `continuum-semantics` / `continuum-persist` / `continuum-events` /
`continuum-policy` / `continuum-capability` 的边**；
（d）`--depth 1` 只钉**直接边**，**不另设闭包断言**（传递可达在 Rust 下不可 `use`）。
**若 (b) 的 grep 有命中，那不是「多一条边」，是设计 §2.1 的边别判错了**——报回来。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
cd /home/DslsDZC/Continuum/.worktrees/p5e-plan
grep -rn "evidence_from\|from_tool_result\|Evidence" crates/continuum-media/src
grep -rn "MethodEntry\|fn register" crates/continuum-media/src
grep -rn "Migration::new\|Tx\b\|append_event\|append_audit" crates/continuum-media/src
grep -rn "Reconstruct3D\|RenderScene\|image_tool" crates/continuum-media/src
grep -rn "propagate_invalidation\|apply_transition\|OperatorRegistry::resolve" crates/continuum-media/src
```

**五条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；
**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：
（a）**证据面零调用**（本块不写域包装、不构造 `Evidence`）；
（b）**零 `MethodEntry` 构造、零 `register`**（只调 `bind`）；
（c）**零迁移、零 `Tx`、零事件**（本块零表）；
（d）**3D 与图像域的两枚/三枚算子零注册**；
（e）**零节点状态迁移**（`resolve` 只可能出现在用例里，`src/` 里若出现即违反切分 §四第 4 条）。
**作用域写死**：五条 grep 只扫 `src/`，**不含测试目录**（测试要用 `resolve` 做 (f) 与 (s)）。

- [ ] **Step 4: 逐条核对设计 §12.2 的 20 条用例是否都有落点**

**本表的字母取设计 §12.2 的一套**（与 Task 1 Step 3 那张表的 §3.5 字母不同：同一个 `(e)` 在两套里是两件事）。

| 条 | 落在哪个 task／用例 | 备注 |
|---|---|---|
| (a) | Task 1 `every_p5_artifact_type_has_exactly_its_declared_encoding` | 手写 11 对，含串字面值 |
| (b) | Task 1：**既有**的 `artifact_type.rs:40` 那六个表外取值（本 task 不改它，重跑时核对） | 语料不在新用例里，它是既有守卫 |
| (c) | Task 1 `a_new_type_survives_a_commit_and_read_back` | 11 枚逐枚 |
| (d) | Task 1 `every_artifact_type_round_trips_through_serde`（改后） | 遍历 `ALL`，并逐型断言 serde 串与 `as_str()` 相同 |
| (e) | 同 (a) 的 `shot_set` / `subtitle_track` 两对 | 多词型只有两条 |
| (f) | Task 3 `the_seventeen_ids_are_hand_listed_and_each_resolves` | 语料手写 |
| (g) | Task 3 `a_second_identical_registration_returns_the_registry_error` | 断言是哪一臂、哪一枚 |
| (h) | Task 4 `every_deterministic_operator_declares_only_bit_reproducible_backends` | 逐枚逐 backend |
| (i) | Task 4 `an_out_of_list_backend_is_rejected_before_anything_is_written` | 两半：返回值 ＋ 注册表未动 |
| (j) | Task 5 `an_ungranted_generative_operator_is_rejected` | 反例侧 |
| (k) | Task 5 `a_granted_generative_operator_is_allowed` | 另一侧 |
| (l) | Task 5 `each_of_the_four_generative_operators_requires_its_own_flag` | 四臂各一条 |
| (m) | Task 5 `a_non_generative_operator_requires_no_flag` | 另一侧 |
| (n) | Task 3 `the_side_effect_class_of_the_five_named_operators_is_asserted_one_by_one` | 五枚各一条 |
| (o) | Task 6 `a_checkpoint_restores_the_progress_it_recorded` | 判据 1 正例 |
| (p) | Task 6 `checkpointing_without_progress_is_rejected` | 判据 1 反例 |
| (q) | Task 6 `restoring_another_operators_checkpoint_is_rejected_and_leaves_state_unchanged` | 判据 2 两侧 |
| (r) | Task 7 `the_six_derived_artifact_producers_have_the_declared_reuse_outcome` | 六行逐行 |
| (s) | Task 8 `every_video_method_gets_at_least_one_realized_by_and_each_string_resolves` | 两条腿 |
| (t) | Task 4 `every_downstream_input_type_has_a_declared_upstream_source` | 17 行逐行 |

**设计 §12.2 只列到 (t)（20 条）；本计划另加五条**（`the_seventeen_ids_come_in_the_designed_order`、
`all_operators_has_seventeen_distinct_ids_and_every_version_is_one`、`the_empty_grant_set_permits_nothing`、
`the_two_implementations_are_the_two_named_operators`、`binding_does_not_add_or_remove_any_method_entry`），
**逐条在 `## 遗留` 第三节申报**。
**若某条找不到对应落点，不得标注为覆盖**，据实报告缺口。

**§12.2 的表不含设计 §5.5 的两条声明**（它不是 §12.2 的用例，是 §5.5 的交付物）：它的落点是
`src/operators.rs` 里 `verify-deterministic` / `verify-multimodal` 两枚算子值上的文档注释（Task 3 Step 3），
**纯声明、无运行期照片**（列在 `## 遗留` 第五节）。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑**承重守卫**（两侧对钉的、fail-open 侧的、失败路径判别 `Err` 的、跨 crate 才可见的），
每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 → 还原后 `sha256sum` → 独立日志路径。
**逐轮在报告里附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位」**。**至少要跑的七轮**：

| 轮 | 变异（档位） | 该红 | 不该红 |
|---|---|---|---|
| 1 | `authorize_generative` 改成「未声明 ⇒ 放行」（取反） | Task 5 的 (j) 与空集那条 | (k)、(l)、(m) |
| 2 | §5.4 的名单判定改成恒真（取反） | Task 4 的 (i) 第一半 | (h)（名单内的候选仍通过） |
| 3 | `register_media_operators` 改成边核边写（取反） | Task 4 的 (i) 第二半 | (h)、(f) |
| 4 | `RestoreRejected` 的身份比对删掉（移除）——**落在 `Render` 那一枚上**（即 (q) 实际用到的那一枚；两枚都改亦可） | Task 6 的 (q) 第一半 | (o)、(p)、后端两侧 |
| 5 | `required_permission` 对四枚都返回 `Broll`（收紧） | Task 5 的 (l) 后三条 | (j)、(k)、(m) |
| 6 | 把某一枚新变体的 `as_str` 串改成另一个**同样合法**的小写串（`shot_set` → `shotset`） | **Task 1 的 (a)**（手写语料里那一枚的期望串对不上）**与改后的 (d)**（serde 串与 `as_str()` 不再逐型相同，两条腿各红一次） | (b)（六个表外取值不含 `shotset`，仍绿）、(c)（读写走同一对函数，仍绿） |
| 7 | 某一条 `realized_by` 写成空 `Vec`（放宽） | Task 8 的 (s) 第一腿 | Task 8 的第二腿、其余 |

**第 4 轮的锚点是 `Render` 那一枚，不许落在 `GenerateBroll` 上**：Task 6 有两枚 `impl Checkpointable`，
两处都含身份比对；若变异只落在未被 (q) 用到的那一枚上，会报 GREEN 而判据其实未被改动（纪律 1(a) 的失效形态）。
**第 6 轮的档位是设计 §12.3 原判的「等价变异体」，本计划把它变成真变异体**（口径与来历见 `## 遗留` 第二节）：
设计 §12.3 那一行预告「全绿」是按「只断往返」的语料写的，而本计划的 (a) 直接断言串的字面值、(d) 断言
serde 串与 `as_str()` 相同，故这一档**红在 (a) 与 (d)**。**按设计的旧预告执行会白跑一轮。**

**一条跨 crate 的承重守卫必须跑全量**：`ALLOWED` 与实际依赖一致（Task 3／7／8／9 每加一条边都要核一次）。
**若某轮跑成了「编译失败」而不是「用例失败」，按纪律 1(c) 记为「编译不过」，并另换一个真变异体。**

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。**本计划不新建、也不修改第二份文档**：
若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 7: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs(media): P5e 媒体领域算子的收尾与复核"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**
设计 §13 的 **9 条**对账条目与 §14 的 **9 条**遗留**本计划一条都不发明**——收件人照原文，逐条落在下节第四节。

### 一、设计问题：本计划查出的**四处**（发现即报，本计划一处都不替设计补写）

1. **`PermittedGenerativeContent` 的形状未给，而它的「缺省」正是 §34 的判据所在**（**没写清**）：
   设计 §7.1 给的是 `pub struct PermittedGenerativeContent { /* ... */ }`——**成员类型、构造函数、查询方法一处都没有**；
   而 §7.2 的反例判据（「未声明 ⇒ `Err`」）与 §12.2 的 (j)(k) 两条都要**先有一个「未授权/已授权」的值**。
   更关键的是 §7.2 那句「**这里不是「用 `unwrap_or(true)` 让缺省放行」**」：它要求「空集」这个状态
   **有一处具名的、可被用例指认的产生点**——若本块的实现顺手给一个 `Default`，那条判据就当场为假。
   **本计划的处置**：**不给 `Default`**，给两个具名构造函数（`none()` / `from_permissions(&[..])`）与 `contains`，
   并加一条用例把「空集什么都不放行」四枚逐枚钉住（Task 5）。
   **这是本计划自定的一处 API 形状**（申报见第三节）。**收件人：设计作者 ＋ 复审者。**
2. **`CheckpointError` 的编码与设计 §12.1 相抵**（**两处相抵**）：
   §12.1 逐字要求「`CheckpointError`（3 臂）、`GenerativePermission`（5 臂）：各一条**臂数与往返**用例
   （`as_str` / `parse` 两枚穷尽 `match` + 一条数目断言，形状照 `ArtifactType` 的既有做法）」；
   而 §4.3 给的 `CheckpointError` **三臂都带载荷**（`BackendId` / `CheckpointOwner` / 两个 `CheckpointOwner`），
   §4.3 与 §7.1 的类型定义里**都没有 `as_str` / `parse` / `ALL`**。
   **两处不能同真**：带载荷的错误枚举没有「字符串往返」这一说——本仓的 `OperatorError`
   （`crates/continuum-operator/src/registry.rs:6-12`）正是同形，它只有 thiserror 的 `Display`。
   **本计划的取舍**：`GenerativePermission`（无载荷）照 §12.1 落 `ALL` ＋ `as_str` ＋ `parse` ＋ 数目断言；
   `CheckpointError` **只落三臂各自的 `Display` 断言**（Task 2 第一条用例），**不落 `as_str` / `parse`**。
   **翻转条件**：若协调者判 §12.1 那句适用于 `CheckpointError`，则须先给它一个**无载荷**的臂集
   （那会改掉 §4.3 的形状与「三臂各自判据」三条理由中的两条）。
   **收件人：设计作者 ＋ 协调者。**
3. **两枚检查点实现的类型名与它们跟 `all_operators()` 的关系未给，三条判据要用的读写面也没给**（**没写清**）：
   设计 §4.5 定了「实现的算子是 `generate-broll` 与 `render`」与三条判据，也写了「`type Checkpoint`
   的字段清单不在本设计的约束范围内」；但**没有给承载 `impl Checkpointable` 的类型名**，
   也没有说这两枚类型与 §5.2 那 17 个 `Operator` 值（纯数据）之间的关系由谁维持；
   **更没有给测试侧的写入面**——三条判据里有两条要求用例能**推进进度**与**指定选中的 backend**
   （判据 1 的「有进度」、判据 3 的「选中的 backend」），而哪两个方法做这两件事一处未给，
   **按 §4.5 的文本写不出那两条用例**。
   **本计划的处置**：`continuum_media::{GenerateBroll, Render}` 两枚类型，各带 `operator_id() -> OperatorId`
   与 `new` / `record_completed_segment` / `select_backend` / `progress` 五个读法／写法
   （`type Checkpoint` 取一枚字段全 `pub` 的 `Progress`），
   并加一条用例逐枚核 `operator_id()` 与 `all_operators()` 里对应那一枚的 `id` 相同（Task 6）。
   **代价**：「类型名与这组读写面都是本计划定的」；**收益**：`Checkpointable` 的第一枚实现有一个可指认的
   落点，三条判据的用例也真的写得出来（而不是「定义了一个没人实现它的类型」——
   切分 §八 明写本块要写明落点与判据）。
   **收件人：设计作者。**
4. **`all_operators()` 的 `side_effect_class` 与 `determinism` 各有一批没有对应用例的承担者**（**判据缺口**）：
   设计 §12.2 只点名了五枚（(n)）与「`Deterministic` 的每一枚的 backend」（(h)）。
   **未被任何用例钉住的 `determinism` 是十一枚**（(r) 的六行经 `cache_key` 的 `Some` / `None` 钉住六枚，
   17 − 6 = 11）；**(h) 按 `determinism` 的取值挑要遍历的算子，但它不断言那个取值**
   （把一枚 `Deterministic` 改成 `NonDeterministic` 只让 (h) 少遍历一枚，仍绿），
   故 `determinism` 一侧的承担者只有 (r) 的六枚；**`side_effect_class` 一侧未钉住的是十二枚**（17 − 5），
   因为 `Pure` / `Idempotent` / `NonIdempotent` 的划分只有五枚有照片。
   **本计划不发明那条全称**（给 17 枚各写一条断言会让「枚举式断言」变成第二份转录），
   **照实写明**：**「17 枚的 `side_effect_class` / `determinism` 都对」这句话在本计划里没有照片**，
   钉住的只是被点名的那几枚。
   **收件人：复审者**（若判必须全量，缺的是「把设计的 §5.2 表落成一条表驱动用例并逐行给人读的对照」这一步）。

### 二、设计 §12.3 的变异预告：本计划照抄，并补**两处预测的修正**

设计 §12.3 的七行预告**本计划一处不改**（它是本块变异那一轮的输入）。**两处要单独读出**：

- 表里「**放宽**：`all_operators()` 少注册一枚（16）」那一行自陈「**这一档若不红，说明 f 是按
  `all_operators()` 自己遍历的**——即假照片」。**本计划的 (f) 语料手写**（Task 3），故这一档会红。
- 表里「**等价变异**」那一行的处置是「换成与表外取值相撞的串」——**本计划另加了两半**：
  Task 1 的 (a) **直接断言串的字面值**，**改后的 (d) 又逐型断言 serde 串与 `as_str()` 相同**，
  故「把串换成另一个合法小写串」这一档**红在 (a) 与 (d) 两处，不必再换变异体**
  （只断 serde 往返时它确实全绿——那条旧预告按当时的语料是对的，Task 10 Step 5 第 6 轮据此订正）。
  **这是本计划的一处加强**（申报见第三节）。

### 三、本计划自定的形状与取值（**申报**，逐条说清代价）

- **`GenerativePermission::as_str` 的五个串**（`broll` / `voice` / `music` / `frame_interpolation` /
  `three_d_generation`）：设计只给了类型名与规范侧五个旗标名（`allow_generated_broll` 一类），**没给编码串**。
  取值沿用本仓既有约定（小写、多词以 `_` 连接，`crates/continuum-artifact/src/artifact.rs:37` 的注记）。
  **代价**：这是本块定的外部编码，将来若要改是破坏性变更。
- **`PermittedGenerativeContent` 的两个构造函数与 `contains`**（见第一节第 1 条）。**代价**：多两个公开入口；
  **收益**：§34 的 fail-closed 有一个可被用例指认的产生点。
- **`GenerateBroll` / `Render` 两枚类型名、`operator_id()` 与那四个读法／写法**
  （`new` / `record_completed_segment` / `select_backend` / `progress`，与 `type Checkpoint = Progress`
  一枚字段全 `pub` 的结构体）（见第一节第 3 条）。
  **代价**：本计划定的外部面有六项（两枚类型名 ＋ 一组读写面）；**收益**：§4.5 的三条判据在用例里真的写得出来，
  且「选中的 backend 支不支持」由 `Operator::backend_candidates` 承载，不另立第二份名单。
- **`GenerativePermission` 比设计 §7.1 多三项派生**（设计只给 `#[derive(Debug)]`，Task 5 取
  `#[derive(Debug, Clone, PartialEq, Eq)]`）：`PartialEq` 是 §12.1 的逐臂往返断言所需，
  `Clone` / `Eq` 照本仓枚举措辞的既有做法，**不取 `Copy`**（设计未把本型定成可复制值）。
  **代价**：本型的外部面比设计宽三项派生。
- **`verify_deterministic_backends` 作为独立的私有函数**（Task 4）：把「先核后写」拆成两步。
  **代价**：多一次遍历；**收益**：那条性质的唯一可观察后果（违规时注册表没被动过）成立。
- **(a) 直接断言 11 枚新串的字面值 ＋ (d) 逐型断言 serde 串与 `as_str()` 相同**（Task 1）：
  设计 §12.2 的 (a) 只说「往返」、(d) 只说「遍历 `ALL` 逐型断言 serde 往返」，
  本计划把「串的字面值」与「serde 表示 == `as_str()`」两条也断言了。
  **代价**：将来改串要改两处（`as_str` 与那条手写清单）；**收益**：把设计 §12.3 自己标出的等价变异体
  变成真变异体——「删 `rename_all`」这一档在只断往返时不红，加了 (d) 的那条断言才红。
- **`a_new_type_survives_a_commit_and_read_back` 落在新文件**（`tests/artifact_types_p5.rs`）
  而不是改 `crates/continuum-artifact/tests/artifact_store.rs`：**代价**：多一个文件；
  **收益**：`artifact_store.rs` 的既有夹具（`fn artifact(...)` 固定 `ArtifactType::Text`，`:8-21`）不动。
- **本计划另加的五条用例**（Task 3 的 id 互不相同、Task 3 的 id 次序、Task 5 的「空集什么都不放行」、
  Task 6 的「两枚实现是那两枚算子」、Task 8 的「bind 不增删条目」）：**逐条都是「守卫两侧都钉」那一侧的补件**，
  **不是设计要求的**。**代价**：五条用例的实现成本；**收益**：五处 fail-open 侧各有照片，
  且设计 §5.1 那句「顺序即 §5.2 表的行序」不再是没照片的绝对措辞。

### 四、设计 §13 的 9 条与 §14 的 9 条：**本计划一条都不发明，收件人照原文**

设计 §13（对账条目）与 §14（遗留与未决项）**逐条以「收件人」结尾**，**本计划不重述其内容**
（重述就是第二份转录，正是本项目出错最多之处）。**本计划的取用**只有三处，逐条说明：

- **§13 第 1 条**（三处「有判定、无调用点」并成一笔）**本计划照它办**：三处分别是 Task 6 的
  `checkpoint()` / `restore()`、Task 5 的 `authorize_generative`、以及切分 §八 已有的
  `OperatorRegistry::resolve` 调用点。**本计划不为它们建接线 task**，三处一律无生产调用方。
- **§13 第 8 条**（与 P5b 对账）**本计划照它办**：Task 8 的 (s) 就是它说的那条运行期补件。
- **§14 第 9 条**（「本块的两处登记入口、两处判定今天都没有生产调用方：四者」）**本计划据实复核**：
  `register_media_operators`（Task 3）、`bind_media_methods`（Task 8）、`authorize_generative`（Task 5）、
  `checkpoint()` / `restore()`（Task 6）——**四者**，与设计订正后的数目一致（设计 2026-10-10 把五者减为四者）。

**逐条照收件人归类**（**为让「核过」可查**，与 P4 那份同一做法）：

- **收件人：第 3 层的执行器（未建）**——§13 第 1 条（三处并一笔）。
- **收件人：P1 ＋ 协调者**——§13 第 2 条（三条：`CheckpointError` / `CheckpointOwner` 落在 `continuum-operator`
  并改 `:95` / `:96`；`CheckpointOwner` 与 `OperatorRef` 合不合并；复用判据的存储与调用点）。
- **收件人：规范维护者 ＋ 协调者**——§13 第 3 条 ＝ §14 第 3 条（模型型派生产物的可复现性判据不存在；两处互指）。
- **收件人：规范维护者**——§13 第 4 条 ＝ §14 第 4 条（§329 的 Timeline 六字段无 schema；两处互指，
  并各与 P5a 设计 §15 第 9 条互指）；§13 第 7 条 ＝ §14 第 7 条（§5.4 的 backend 名单、§6.2 的 `Json` 映射、
  §5.3 边表的边判据三处封闭表都没有规范来源）。
- **收件人：P5a（`continuum-verify`）＋ 装配方**——§13 第 5 条（三条：`VisualCheck` 够用；
  两枚验证算子的 §262 级别声明由谁持有；`RequirementId` 的归属假设）＋ §14 第 8 条。
- **收件人：P5a ＋ 协调者**——§13 第 6 条（检查点的身份约束：`type Checkpoint` 今天只有 `Send + Sync`）。
- **收件人：协调者**——§14 第 1 条（本块的兑现口径是「让算子满足 §305 的前提」）、
  §14 第 2 条（`Checkpointable` 的类型只能落在 `continuum-operator`）、
  §14 第 5 条（`ThreeDGeneration` 这一臂今天没有算子映射它）、§13 第 9 条
  （P1 设计 `:573` 那句会随加型而变旧的文本，`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:573`）。
  **本计划不改别人的文件**（设计 §3.5 的 (g)）。
- **收件人：规范维护者 ＋ 第 3 层**——§14 第 6 条（检查点的存储形态未定义）。
- **§14 第 9 条**——收件人是**第 3 层**（见上）。

### 五、拍不到的照片

设计 §12.5 的**五条**（端到端、§34 授权在真实执行路径上的效果、`checkpoint()` / `restore()` 的崩溃恢复效果、
§131 的复用真的省下一次重算、Timeline 六字段）**逐条照原文，本计划不发明**。**本计划另补三条**
（编号是本节的补件序号，与设计 §12.5 的五条无关）：

- **设计 §5.5 的两条 §262 级别声明只有文档注释这一个载体**：`verify-deterministic` 是第 1 级、
  `verify-multimodal` 是第 3 级（Task 3 Step 3），而 `Operator` 的七字段里没有级别，
  故**它没有运行期照片**（同纪律 8 的「纯数据声明」）。**这不是漏拍**：级别序、候选过滤与独立性判定
  都属 P5a（设计 §5.5 的归属表），本块交付的就是那两条声明本身。
- **「17 枚的 `side_effect_class` / `determinism` 都对」是拍不到的**：见第一节第 4 条——
  钉住的是被点名的那些，不是那个全称。
- **`bind_media_methods` 的四条 `realized_by` 与算子的对应关系只有运行期照片**：
  `realized_by: Vec<String>`（P5b 设计 §4.3）在编译期核不了，
  故「绑的是不是**对的**算子」这一半只有「串能 `resolve`」那一条腿——
  **「`render-review` 该绑 `verify-deterministic` 与 `verify-multimodal`」这个判断本身没有照片**
  （它是设计 §10 的判定，规范只给了 §32 的步骤名）。

---

## 交付给谁

- **给驱动的接线方（未建）**：三处判定的调用点，同落 `Queued → Running`——`authorize_generative`（Task 5）、
  `checkpoint()` / `restore()`（Task 6）、`OperatorRegistry::resolve`（切分 §八 已有的那一处）。
- **给 P5a 的实现方**：`Evidence` 的构造调用点（宿主侧，`subject` 取 `Unattached`）＋
  本块的两条结构照片（Task 9）在 `continuum-verify` 落地后**必须重跑并重取 `.stderr`**。
- **给 P5b 的实现方**：四条 `video/` 的 `realized_by`（Task 8）＋「本块不 `register` 方法条目」这条边界。
- **给 P5c／P5d／P5f 的实现方**：`ArtifactType` 的 11 枚新变体由 Task 1 一次落地（**三份设计的报出都是「无」**）；
  P5f 需要的 `Image`、P5d 需要的 `Report` 都在其中；**Task 1 之后不得再改这个枚举**。
- **给协调者**：本计划第一节查出的**四处设计问题**、第二节对设计 §12.3 的两处修正、**第三节逐条申报的本计划自定形状**；
  以及「Task 8／9 在 P5a／P5b 的实现落地前动不了」这一条排期事实。
- **给复审者**：**变异那一轮的七轮记录**（纪律 2 要求的 sha256 与日志路径）、Task 10 Step 1 的**两条门读数**、
  以及 `## 遗留` 第五节另补的那几条**拍不到的照片**。
