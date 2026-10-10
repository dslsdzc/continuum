# P5c（代码领域算子）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建一个新 crate `continuum-code`，落《总纲》§8.3 那条代码任务执行链上的一组**可注册算子**——
各枚的 `input_schema` / `output_schema` / `determinism` / `side_effect_class` / `backend_candidates` 声明、
注册入口、以及它在 P5b 方法库 `software/` 目录里的 `realized_by` 条目。
本块**只声明算子，不实现算子的执行体**（`OperatorImpl` 不在交付物里，切分 §四第 4 条）。

**Architecture:** 本块是**跨领域层（第 8 层）的一个叶子块**。它经 `Operator` / `OperatorRegistry`
（`crates/continuum-operator`，P1）与第 3 层相接，**不新开跨层边**（切分 §三 末段）。
落地形态是：`continuum-code` 只登记**实际用到**的 workspace 内边（切分 §四第 6 条）——
普通边两条（`continuum-artifact` 的 `ArtifactType`、`continuum-operator` 的算子类型），
dev 边三条（`continuum-graph` 的复用判定、`continuum-workspace` 的后端名、`continuum-verify` 的 `Evidence`），
`continuum-method`（P5b 的 crate）落地后加第六条。
本块**零表、零迁移、零事件**（设计 §9），**不碰 `continuum-runtime` 的装配面**。

**本块是声明，不是执行**：「算子的实现体在哪里」在本块**没有答案**，因为交付物里没有 `OperatorImpl`；
「工作区句柄怎样到达算子实现体」不可表达（设计 §4.4 的 `NodeContext` 缺口）；
「`Deterministic` 声明的候选里混进不可复现的 backend」不可表达（注册期规则拒它，设计 §3.5）；
「本块自造证据类型」不可表达（本块不建证据构造代码，设计 §6.2）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-artifact`（`ArtifactType` / `ContentHash`）；
`continuum-operator`（`Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` /
`BackendId` / `OperatorRegistry` / `OperatorError`）；`thiserror`；
dev：`continuum-graph`（`cache_key` / `can_reuse` / `CacheKey`）、`continuum-workspace`（`WorkspaceBackend`）、
`continuum-verify`（`Evidence`）、`trybuild`；P5b 落地后加 `continuum-method`。

**设计依据：** `docs/superpowers/specs/2026-10-09-p5c-code-domain-operators-design.md`（**唯一事实来源**，1160 行）。
切片与冻结口径：`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`。
**本计划不改设计、不改规范、不改切分文档、不改代码以外的任何计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-10 实读工作树）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| P1 `continuum-operator`：`Operator { id, version, input_schema, output_schema, determinism, side_effect_class, backend_candidates }`（`crates/continuum-operator/src/definition.rs:79-88`）、`OperatorId`（`:24`，字段私有、`new` 非 `const fn`）、`OperatorVersion`（`:45`）、`BackendId`（`:66`）、`OperatorRegistry::{register, resolve}`（`crates/continuum-operator/src/registry.rs:24`、`:36`）、`OperatorError`（`:7-12`） | **已交付** | 九枚算子的类型、注册入口、注册期规则的错误臂 |
| P1 `continuum-artifact`：`ArtifactType` 六型（`crates/continuum-artifact/src/artifact.rs:14-21`）、`ALL`（`:28`，带数目字面量 `[ArtifactType; 6]`）、`as_str`（`:47`）/ `parse`（`:68`）两个穷尽 `match`、`ContentHash::of`（`src/content.rs:11`） | **已交付** | 九枚算子的端口类型；Task 4 的复用照片 |
| P1 `continuum-graph`：`cache_key`（`crates/continuum-graph/src/reuse.rs:16`）、`can_reuse`（`:29`）、`CacheKey`（`:9`）；另 `is_candidate_backend`（`src/execution.rs:87`） | **已交付** | Task 4 的 §3.6 照片（**dev 边**） |
| P2 `continuum-workspace`：`WorkspaceBackend` 两臂（`crates/continuum-workspace/src/backend.rs:22`、`:26`、`:31`）、`backend_str`（`src/persist.rs:177`，**`pub(crate)`**） | **已交付** | Task 5 的 §4.2 照片（**dev 边**） |
| P5a `continuum-verify` 的 `Evidence` 与 `Evidence::from_tool_result`（P5a 设计 §4.4） | **未交付**（实读 `ls crates/`：无 `continuum-verify`；`continuum-verify` 这个串只出现在六份设计文档里） | Task 6 的 trybuild 照片；**Task 6 的硬前置** |
| P5b `continuum-method`：`MethodRegistry` / `MethodId` / `MethodError` / `bind`（P5b 设计 §4.2–§4.5） | **未交付**（同上，`ls crates/` 无 `continuum-method`）；P5b 的实现计划**也不存在**（`docs/superpowers/plans/` 最新一份是 P4 的） | Task 7 的 `bind_code_methods`；**Task 7 的硬前置** |
| P4 `continuum-semantics` 的 `RequirementId` | **未交付**（`ls crates/` 无） | **本块不用**——设计 §5.2 的 `subject` 一律取 `EvidenceSubject::Unattached`，故 §2.1 的边表里没有它 |
| 本块新建的 `continuum-code` | **不存在**（实测 `grep -rn "continuum-code\|continuum_code" --exclude-dir=.git .` 在 `crates/` 与 `Cargo.toml` 里**零命中**，只有设计文档提到它） | 本计划的全部交付物 |

**一句话结论**：本块**依赖 P1/P2 的既有产物**（已交付），**依赖 P5a 与 P5b 的两个新 crate**（未交付），
**不依赖 P4**。前者的接口本计划逐条实测过；后两者的接口**只能照设计写**，落地时以 crate 源码为准。

### 二、哪些 task 在 P5a / P5b 未交付前动不了

- **Task 6（trybuild 的 `Evidence` 照片）** 的 `Files` 里要写 `crates/continuum-verify` 的 `Evidence`
  ——**那个 crate 今天不存在**（上表实测）。它 `use` 不到、编不过、`cargo test` 也起不来。
  **硬前置：P5a 的 `continuum-verify` 已交付且 `Evidence::from_tool_result` 已落地。**
- **Task 7（`bind_code_methods`）** 的 `Files` 里要写 `crates/continuum-method` 的 `MethodRegistry`
  ——**那个 crate 今天不存在**。**硬前置：P5b 的 `continuum-method` 已交付且 `seeded()` 已建出
  §187 的 `software/` 七条条目**（P5b 设计 §4.5）。
- **其余六个 task 一个既有文件的实现体都不依赖**：它们只新建 `crates/continuum-code/**`，
  改两处**共写文件**（`Cargo.toml` 的 `members` 与 `crates/continuum-runtime/tests/dependency_direction.rs`
  的 `ALLOWED`），两处都在切分 §一 的共写文件表里。**它们今天就可以开工**。
- **Task 1 与 Task 2 之间、Task 3 与 Task 4 与 Task 5 之间没有硬序**：Task 6 与 Task 7 排在最后，
  只是为了让「前六个 task 可立即开工」这件事在计划里是**可执行的**，不是一条风格偏好。
  **但 Task 4 与 Task 5 要改 `crates/continuum-runtime/tests/dependency_direction.rs` 的
  同一个内层数组**（`("continuum-code", &[…])` 那一项：Task 4 加 `continuum-graph`、
  Task 5 加 `continuum-workspace`）——**并行执行这两者会在该文件上互相覆盖**。
  Task 3 不碰该文件（它不登记任何新边，只新建一条测试）。
  「没有硬序」说的是依赖关系，不是文件所有权：两者并行时须**串行化对那一行的改动**
  （或由最后合入的一方核对两条边都在），这不是风格偏好。

### 三、本块对既有之物的请求（**本计划只作消费方写，一处都不实施**）

设计 §11 与 §12 已列。收件人逐条照原文，本计划**不为它们建 task**——它们要改的文件属别的层，
改它们会让同一个文件有两个改动方。

| 处 | 请求的内容 | 本块的角色 | 收件人 |
|---|---|---|---|
| `backend_str` 是 `pub(crate)`（`crates/continuum-workspace/src/persist.rs:177`） | 设计 §10.2 第 (p) 条把照片坐标指到它，而 **`pub(crate)` 的函数在 `continuum-code` 里调不到**——故本块**没有**「两个候选名等于 `workspace` 列的编码」这张照片（详见 Task 5 与 `## 遗留`） | **消费方**（只 `use` `WorkspaceBackend` 那两臂） | P2 ＋ 协调者 |
| Base 不在端口上（设计 §4.3）、`NodeContext` 没有工作区句柄（设计 §4.4） | 这两条是同一处改动的两半：算子的输入端口的**取值域**与执行上下文；两者都在 P1 / 第 3 层的改动范围里 | **消费方**（本块只声明 `input_schema` 为空、不补通道） | P1 ＋ 第 3 层的执行器 |
| `code-workspace` 的「每 Intent 一枚」（设计 §4.2 末） | §16（`docs/spec/01-concepts.md` 的「Git L2 Sandbox」：每个 Intent 创建独立 `ai/task-id`）要求一个 Intent 一份工作区，而算子是**每节点**的执行单元（§244）。本块**无法在类型层钉住「图上只有一个该节点的实例」** | **消费方**（只登记声明） | 协调者（Planner 的归属未定） |
| 迁移档号 `150`（设计 §9.3） | 本块**零表**，故 `150` 是一条**登记**不是一条死迁移；须与其余各块与 P6 的档合并成一张表 | **登记方** | 协调者 |
| 两个「Plan」同名（设计 §8 末、§11 第 5 条） | §228 的 `Plan` 是 P4 的对象（`docs/spec/05-normative.md` 的「Plan 数据模型」一节），`code-plan` 产出的是 `Json` 制品（§186 的 `Implementation Plan` 一步）——同名不同物 | **消费方**（不改名、不合并） | P4 的持有方 ＋ 协调者 |
| 「逐位可复现的 backend」封闭名单（设计 §3.5） | **规范没有一处给过 backend 的确定性**——缺的是「该 backend 的什么属性使输出逐位可复现」这一步 | **定义方**（名单是本块定的） | 规范维护者 |
| `TestResult` 的载荷 schema（设计 §5.4 第 4 条） | 承载处与写入方已定（`TestResult` 制品、`code-test-run` / `code-fuzz-run`），**载荷 schema 未给**——缺的是「一份 `TestResult` 里哪些字段是判『通过/不通过』所必需的」这一步 | **报出方**（不发明） | 规范维护者 ＋ P5a |

**两处设计里的「待对账」标记已关闭，本计划不把它们带进 brief**：

- **§7.2 的 `TDD` / `debugging` 无算子对家** —— **已关闭**：协调者 2026-10-10 已取该节处置表的第三案
  （认这两条无算子对家），落点是 P5b 的 `UNREALIZED_BY_DESIGN` 常量，裁定与两案被否的理由写在
  P5b 设计第十节判据 6。**Task 7 的用例照该裁定写**（`TDD` 与 `debugging` 的 `realized_by` **为空**，
  且这一条不与「非空即过」混读）。
- **§5.3 的两枚审查算子无臂可落** —— **已关闭**：本块报出的 `ModelReview` 已被 P5a 收下并落地
  （P5a 设计 §4.2 末三行）。**本块的证据产出面因此没有待定项**，而 §5.2 表里那两枚的
  `EvidenceType` 由**装配方**在调用时给出，本块**不写死它**（设计 §6.2 末）。

### 四、本节把什么交给谁

- **交给 P5a 的 `continuum-verify`**：`code-spec-review` / `code-quality-review` 的 `EvidenceType`
  （`ModelReview`）与 `code-test-run` / `code-fuzz-run` 的（`Test` / `Fuzz` / `Property`）**由装配方在构造时给**，
  本块只声明对应关系。**本块 crate 里没有这一层的函数**（设计 §6.2 的「不建第二枚包装」）。
- **交给 P5b 的 `continuum-method`**：`software/` 目录的**五条**非空 `realized_by`。`TDD` 与 `debugging`
  按已关闭的裁定**留空**（见上）。
- **交给第 3 层的执行器（未建）**：九枚算子的 `OperatorImpl`、`Queued → Running` 处的 `resolve` 调用点、
  以及 `register_code_operators` / `bind_code_methods` 的**生产调用方**——三者今天都没有落点
  （切分 §四第 4 条；P1 设计 §18 已有同形的一条）。本块的三个入口**今天没有生产调用方**，
  这不是本块的缺口（设计 §12 第 9 条）。
- **交给协调者**：迁移档 `150` 的合并、`code-workspace` 的基数归属、两个 `Plan` 同名的处置、
  以及本计划查出的设计问题（逐条见 `## 遗留`）。
- **交给规范维护者（本项目无此角色）**：§3.5 的 backend 确定性名单、`TestResult` 的载荷 schema。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划新增的 workspace 内边只有这些**（逐条由需要它的 task 登记，**一次不声明齐**）：
  - Task 1：`continuum-code ← continuum-artifact`（`ArtifactType` 进 `Operator` 的端口字段）、
    `← continuum-operator`（`Operator` 及其全部伴生类型）；
  - Task 4：`continuum-code ← continuum-graph`（**dev 边**，`cache_key` / `can_reuse`，唯一使用点是 `tests/reuse.rs`）；
  - Task 5：`continuum-code ← continuum-workspace`（**dev 边**，`WorkspaceBackend`，唯一使用点是 `tests/workspace_backends.rs`）；
  - Task 6：`continuum-code ← continuum-verify`（**dev 边**，`Evidence`，唯一使用点是 trybuild 的编译失败样例）；
  - Task 7：`continuum-code ← continuum-method`（**普通边**，`MethodRegistry` / `MethodId` / `MethodError` 在 `src/methods.rs` 的签名里）。
  **终态恰是设计 §2.1 共写文件表给的那六项**，
  `("continuum-code", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify", "continuum-workspace"])`。
  **上表之外一条边都不登记**——尤其没有指向 `continuum-core` / `continuum-persist` / `continuum-events` /
  `continuum-policy` / `continuum-capability` / `continuum-port` / `continuum-semantics` /
  `continuum-secret` / `continuum-sandbox` 的边（设计 §2.1 的「不登记的边，逐条给理由」段）。
  **外部 crate（`thiserror` / `trybuild`）不进 `ALLOWED`**——那张表只逐对断言 workspace 成员之间的边
  （`crates/continuum-runtime/tests/dependency_direction.rs:22-24` 的注记）。
- **dev 边也要登记**：`dependency_direction.rs` 的 `cargo_tree_direct` 带 `--edges all`（`:270-276`），
  dev 边与普通边一视同仁。本仓已有同形先例（表内 `continuum-provider` 与 `continuum-connector` 两行
  的注记各写着「**dev 边**」）。
- **`--depth 1` 只钉直接边，这就是这条规则的完整粒度，本计划不另设闭包断言。**
  Rust 的 crate 可见性要求**直接声明**才能 `use`，未写进 `Cargo.toml` 的传递依赖**写不出**
  `use continuum_method::…`。故「直接边」恰是这条规则的完整粒度——**写这一段的用途**：
  免得读到上面那条约束时以为「任何指向别层的边都会变红」，**那会是一条假保证**。
- **本块零迁移、零表、零事件。** 迁移档号 `150` 只作**登记**（设计 §9.3）：实测
  `grep -rn -A1 "Migration::new(" crates` 在 `:130`–`:199` 这一段**零命中**（全仓实测的号是
  `1` `2` `10` `20` `30` `40` `41` `50` `60` `80` `100` `101` `102`）。**本块不新增任何迁移**，
  故不动 `runtime_migrations()`。
- **本块不碰 `crates/continuum-runtime/src/**`。** 本块没有装配面：算子注册进 `OperatorRegistry`
  这一步的调用点在装配方 / 第 3 层，今天不存在（跨计划前置第四节）。本块在 `continuum-runtime`
  下的改动**只有一条 `ALLOWED` 条目**，落点是 `tests/dependency_direction.rs`。
- **本块对切分 §一 共写文件表的改动面只有两处**：`Cargo.toml` 的 `[workspace] members` 加一行
  `crates/continuum-code`；`dependency_direction.rs` 的 `ALLOWED` 加 `continuum-code` 条目。
  `crates/continuum-artifact/src/artifact.rs`（归 P5e）、`crates/continuum-graph/src/node.rs`（归 P5a）、
  `crates/continuum-operator/src/definition.rs`（归 P5e）**本块一处都不碰**——这是 §8「零请求」
  与 §6.2「不建第二枚包装」的直接后果。
- **枚举列的落库编码一律小写、多词以 `_` 连接**：本块**不落库**，故本块**不定义任何编码函数**。
  唯二与编码有关的两串是 `backend_candidates` 的元素（`"worktree"` / `"overlay"`），
  它们的权威在 P2 的 `backend_str`（`crates/continuum-workspace/src/persist.rs:177`）。
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets`
  同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，
  锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不建九枚算子的 `OperatorImpl`（执行体）**（切分 §四第 4 条；`crates/continuum-graph/src/execution.rs:76-82`
    的 `OperatorImpl::execute` 本块不实现任何一枚）。
  - **不建执行路径、不做 `Queued → Running` 的迁移、不写 `OperatorRegistry::resolve` 的调用点**（同上）。
  - **不建第二枚证据包装**（不定义 `evidence_from_code_output`），**不写任何证据构造代码**（设计 §6.2；
    协调者 2026-10-10 的四块统一口径写在 P5a 设计 §4.4）。
  - **不改 `ArtifactType`、不请求任何新变体**（设计 §8：本链零请求）。
  - **不建第二份复用判据**（§305 已由 `crates/continuum-graph/src/reuse.rs:29` 落地）、
    **不建第二份端口兼容判定**（§239 由 `crates/continuum-port/src/port.rs:94` 落地）、
    **不建第二份后端候选判定**（§245 由 `crates/continuum-graph/src/execution.rs:87` 落地）——切分 §四第 7 条。
  - **不建 `MethodEntry`、不 `register` 方法条目**（P5b 设计第六节禁止四块自造；设计 §7.3 的答复
    也是这一条——§186 的流程名不入 `software/` 目录，它们落成的是**算子的 id**，不是方法条目）；只调 `bind`。
  - **不建 `code-debug`**（设计 §7.2 第 2 条：链上没有调试这一步，且规范没给过调试的端口类型）。
  - **不定义 `TestResult` 的载荷 schema**（设计 §5.4 第 4 条）。
  - **不把 Integration Gate 算子化**（设计 §4.2：批准值没有位置装）。
  - **不建表、不写迁移、不写事件**（设计 §9.1）。
  - **不改 `docs/02-工程.md`、不改六份设计、不改切分文档**（设计 §13 的「只记，不改」）。

  **设计 §13 的三条（在切分文档里发现的错或缺口）在本块的落点逐条如下**——它们不改切分文档，
  但**要看得出本块按哪一条做**：
  - **第 1 条**（切分 §五 P5c 行「并让每步产出 P5a 的 `Evidence`」的外延过宽）：落点是
    Task 6 Step 3 与「交付给谁」里的**逐枚产出面**——`code-spec-review` / `code-quality-review` 是
    `ModelReview`、`code-test-run` 是 `Test`、`code-fuzz-run` 是 `Fuzz` / `Property`，
    其余五枚不产；以及 `## 遗留` 第四节第 (i) 条（产出面在本块的 crate 里无照片，它是声明）。
  - **第 2 条**（切分 §四第 3 条与 §五都没指出「`software/` 七条里有两条不是算子」）：**就是设计 §7.2**，
    已由 2026-10-10 的裁定关闭，落点见「跨计划前置」第二节与 Task 7 的 `k`。
  - **第 3 条**（切分 §四第 7 条只举了复用判据一例）：落点是本节的三条「不建第二份判据」
    （复用判据 / 端口兼容 / 后端候选判定）。**设计 §13 第 3 条 2026-10-10 的订正记：切分 §四第 7 条
    现已按实测扩到四处**——第四处是 §327 的「制品不得被默认覆盖」，它不邻本块，**本块不引它**。

### 本计划特有的「签名即判据」，照抄前须对源

- `Operator` 的七个字段与 `OperatorId` 的形状（`crates/continuum-operator/src/definition.rs:79-88`、`:24-31`）：
  **`OperatorId` 的字段私有、`new` 不是 `const fn`，且 `Operator` 含三枚 `Vec` 字段**——
  故 `all_operators()` 是**函数不是常量**（设计 §3.2 的两条理由）。引用本仓「带 `ALL` 常量的既有做法」
  （`crates/continuum-artifact/src/artifact.rs:28`、`crates/continuum-events/src/audit.rs:38`）时，
  **要连它的成立条件一起搬**：那两处是无字段枚举。
- `OperatorRegistry::register(&mut self, operator: Operator) -> Result<(), OperatorError>` 与
  `resolve(&self, id: &OperatorId, version: &OperatorVersion) -> Result<&Operator, OperatorError>`
  （`crates/continuum-operator/src/registry.rs:24`、`:36`）；`OperatorError::Duplicate { id, version }`
  （`:11`）与 `NotFound { id, version }`（`:9`）。
- `ArtifactType` 的 `ALL: [ArtifactType; 6]`（`crates/continuum-artifact/src/artifact.rs:28`）、
  `as_str`（`:47`）/ `parse`（`:68`）。**`parse` 收 `&str`、一定有通配臂**，故它**不是**「加一臂即编译不过」那张照片
  ——那是 `as_str` 与枚举上的穷尽 `match` 才有的性质。**本块不新增 `ArtifactType` 的任何臂**，故本块不碰这一对。
- `cache_key(operator: &Operator, input_hash: ContentHash) -> Option<CacheKey>`（`crates/continuum-graph/src/reuse.rs:16`）、
  `can_reuse(operator: &Operator, cached: Option<&CacheKey>, current_input_hash: &ContentHash, contract_affected: bool) -> bool`
  （`:29`）。**它读的是 `operator.determinism`，不看 backend**——这正是设计 §3.5 那处结构缺口的来源。
- `WorkspaceBackend` 的两臂（`crates/continuum-workspace/src/backend.rs:22`、`:26`、`:31`）与
  `backend_str`（`crates/continuum-workspace/src/persist.rs:177`，**`pub(crate)`**）。**后者的可见性是 Task 5 的一处硬限制。**
- `ContentHash::of(bytes: &[u8])`（`crates/continuum-artifact/src/content.rs:11`）。
- **P5a 的 `Evidence` 八字段全私有、无 `Default`、唯一构造点是
  `Evidence::from_tool_result(EvidenceId, EvidenceType, EvidenceSubject, Claim, EvidenceProducer, Vec<ArtifactId>, EvidenceStrength, EvidenceScope) -> Result<Evidence, VerifyError>`**
  （P5a 设计 §4.4）。**该 crate 尚未落地**，落地时以 `crates/continuum-verify/src/` 的源码为准，不符时回报。
- **P5b 的 `MethodRegistry::bind(&mut self, id: &MethodId, realized_by: &[&str]) -> Result<(), MethodError>`
  与 `MethodRegistry::seeded()`**（P5b 设计 §4.5）。**该 crate 尚未落地**，同上。

---

## 已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；
   正向的「变红」跑全量是加分。**每一处「预期谁红」都要标档位。四档是对「守卫的变异」的分档**：
   **取反**（把守卫的判定反过来）／**放宽**（把守卫少判一半条件）／**收紧**（把守卫多判一半条件）／
   **移除**（删掉整条守卫或它的一半动作）。
   **凡变异改的是被守卫断言的声明取值**（`operators.rs` 的某一列、名单里的某一项、语料里的某一个串），
   守卫本身一字未动——那一类按**把被断言的取值改成与期望不是同一枚**读，档位记**取反**，
   并写清它改的是哪一列；**同一形状的变异在全篇用同一个档位**。「把某一行的 `input_schema` 去掉一个型」
   与「给某一行加一个端口型」都属这一类，前者在 Task 1 与 Task 3 一律记取反（设计 §10.3 给前者这一
   形状的档位就是取反），后者记取反。**唯一的例外是设计 §10.3 明标了档位的两处 `determinism` 取值变异**
   （该列有宽紧方向：`Deterministic` 放宽 §305 的复用面、`NonDeterministic` 收紧它），那两处照录设计的
   **放宽**／**收紧**，本计划不把它们的档位改写成取反。
   **三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN。**本计划每条红条件都指明唯一锚点**
   （哪一枚算子的哪一列、哪个函数的哪一行）；凡变异点的字面量在 `all_operators()` 里出现多次的，
   红条件写清是**第几行**。
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**。
   **本块已预先识别的等价变异体逐处列出**：
   （i）**把某枚算子的 `backend_candidates` 里两项换序**——候选列表在本块全部断言里都是**集合**，
   `code-workspace` 的候选与 §3.5 的名单判定都不读次序（设计 §10.3 末行已写明），故**换序是全绿的等价变异体**；
   要打红它必须换变异体：**删掉一项**（届时 Task 5 与 Task 2 的名单逐项断言红）；
   （ii）**`Assertion` 里手抄的期望语料与被测清单同源**——§10.2 第 (m) 条与 Task 3 的边表语料
   **必须手写**，若从 `all_operators()` 派生即是**恒真的假照片**（设计 §10.2 的两条「预防」）。
   **`all_operators()` 的返回类型里的数目 `9` 不在此列，不要把它读成等价变异体**：
   `len() == 9` 是设计 §10.1 列的**守卫**，它钉的是「返回类型的数目正好是 9」——
   删掉一枚算子**必须同步**把返回类型写成 `[Operator; 8]`，那一处改动**会让它红**
   （设计 §10.3 的「移除」那一行预告的正是这一枚，见 Task 1 的用例注释）。它**不钉**的是
   「九枚的 id 与 §3.2 表逐行对得上」——那由 Task 1 的 `f`／`g`／`i` 与 Task 4／Task 5 的逐枚断言承担。
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
   cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
   **本计划里有三条照片本身是编译期照片**（Task 5 的 `WorkspaceBackend` 穷尽 `match`、
   Task 6 的 `Evidence` 不可构造、Task 7 的 `bind_code_methods` 返回类型），
   **它们照实写成编译期照片**，不冒充运行期红。
   **注意 `match` 的入参是不是枚举自身**决定了它成不成立：**收 `&str` 的解析函数一定有通配臂**
   （如 `ArtifactType::parse`，`crates/continuum-artifact/src/artifact.rs:68`），
   故它**不是**那张照片；`WorkspaceBackend` 上的穷尽 `match` 是。
   **`..` 豁免会让变异成为等价变异体**：凡靠穷尽解构钉的字段清单，变异时若在被改处留了 `..`，它照过。
2. **变异脚本必须带还原护栏**（本仓出过一次「变异留在源码里」的事故）：
   每次变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、**每轮用独立日志路径**，
   报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径」。模板（`MUT` 是改的文件、`BAK` 是备份）：
   ```bash
   cd /home/DslsDZC/Continuum
   before=$(sha256sum "$MUT" | cut -d' ' -f1)
   cp "$MUT" "$BAK"
   trap 'cp "$BAK" "$MUT"; echo "已还原"' EXIT INT TERM
   # …施加变异…
   TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$LOG"
   # …读 $LOG 判红…
   ```
   **`trap` 那一行不许省。**
3. **门读数的判据是日志里 `Running` 与 `test result:` 的行数，不是「命令退出码为 0」。**
   `cargo test` **不收 `--keep-going`**，用 `--no-fail-fast`；**Doc-tests 算一条**。
   **不带 `--no-fail-fast` 会在第一个失败目标处停住，后面的目标一行不跑**，
   而日志里仍会出现若干 `Running` 行——故「射程是否覆盖全部测试目标」这件事只能靠**数 `Running` 的行数**，
   不能靠 `Compiling` 行或退出码。
4. **凡注释写绝对措辞，必须有对应用例；且要带时点**——「唯一／一律／只有／全部／没有任何」这类词，
   要么有可写出的照片，要么**明写为什么没有**。**断言的作用域要与事实同宽**：
   「整张表的唯一 X」≠「某个字段的唯一 X」；「本 crate 的源码文本里零命中」≠「全仓零命中」；
   「在本 task 结束时」≠「永远」。**枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条臂各要一条照片，
   不抽代表（手写分支能各自漂移）。
   **本块的绝对措辞逐处落点**：`operators.rs` 里「九枚无一为 `NonIdempotent`」→ Task 1 的 `f`；
   「第 7、9 行是 `Deterministic`，其余七枚不是」→ Task 1 的 `g`；
   「§3.5 的名单恰为五项」→ Task 2 的逐项断言；「候选列表无序」→ 写成**等价变异体的说明**而不是断言；
   「档位记取反的唯一例外是设计 §10.3 那两处 `determinism` 取值变异」→ **设计 §10.3 的表本身**
   （逐行可读：该表里带**放宽**／**收紧**档的只有改 `determinism` 的那两行）。
5. **「自指计数不写数值」**：「下面这 N 条」这类话会被自己推翻。本计划凡引用设计或切分文档的编号，
   照原文的编号写（那是可实测的）；凡指本计划自己的清单，一律用「逐条」而不写数目。
6. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」。本块的失败面是 `CodeError` 的
   **两臂**（`UnverifiableDeterminism` / `Registry`）与透出的 `MethodError::{NotFound, Duplicate}`；
   **逐臂至少一条用例**（Task 2 的 `b` 与 `d`、Task 7 的 `n`），且 `d` 要同时断言**没有半写的副作用**
   （注册表内容与调用前逐枚相同）。
7. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本计划逐处标出「另一侧」是哪一个用例：
   §3.5 的注册期规则（`d` 拒的那侧 ＋ `c` 与 `e` 放行的那侧）、§3.4 的边表（下游输入侧 ＋ 上游输出侧，
   见 Task 3 的两组断言——**下游侧由「下游算子的 `input_schema` 与该行的语料型集合
   两个方向都相等」钉，上游侧由三列来源语料钉**，两组各是一条独立的断言）、
   §4.2 的两个候选名（运行期字面量侧 ＋ 编译期穷尽 `match` 侧）。
   **把两侧塞进同一条枚举式语料不算两侧都钉**：语料只覆盖它写到的那些取值，语料外的取值两侧都是 fail-open。
8. **编译期性质照实写成编译期照片，不冒充运行期红。** 本块的编译期照片逐处标出：
   `all_operators()` 的返回类型数目（Task 1）、`WorkspaceBackend` 的穷尽 `match`（Task 5）、
   `Evidence` 的不可构造性（Task 6）、`bind_code_methods` 的返回类型（Task 7）。
9. **各 task 的「运行，确认失败」一步要写清量到的是哪一种失败。** 只有**该用例引用的符号已经存在**时，
   那一步才可能量到运行期的红；否则量到的是 `error[E0432]`／`error[E0433]` 一类的**编译失败**，
   按本纪律第 1(c) 条**不算红**，那一步是空转的——凡属此类的步骤，本计划**照实标注为编译期照片**，
   并写明「要取到运行期的红需要先有什么」。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。

**变异日志是证据，必须活到复审结束**：**实现者保留 `.tmp/`，由协调者在复审结束后清理**。
报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。
**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。

**变异口径分层**：变异**条数**按「有多少条**互不相同**的守卫」定，不按分支数定，且分两档、
**不许混成一句「通过」**：
- **承重守卫 → 全量套件**：两侧对钉的守卫、**fail-open 的那一侧**、失败路径**判别哪一种 `Err`**、
  **跨 crate 才可见**的效果（`ALLOWED` 与实际依赖一致、`WorkspaceBackend` 的臂数变化、
  新 crate 进 workspace 后的迁移集合与计数不受影响）。
- **其余分支 → 受影响 crate 的包级套件**，报告里须**标明证据强度较低**并列出
  「这一条可能漏掉的跨 crate 观察点」。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；凡与既有 crate 交互的签名
（`Operator` 的字段名、`OperatorRegistry::{register, resolve}`、`cache_key` / `can_reuse`、`ContentHash::of`），
实现前须先读该 crate 的源码确认，不符时以源码为准并回报。

**本计划特有的三处「照抄前须对源」**：

- **`all_operators()` 是函数不是常量**，返回 `[Operator; 9]`；**数目写在返回类型里**，
  故「清单个数」这件事在编译期可见。**不要把 `const` 与 `[T; N]` 混着搬**——
  本仓带 `ALL` 常量的两处（`ArtifactType::ALL`、`AuditKind::ALL`）都是**无字段枚举**才成立。
- **`backend_str` 是 `pub(crate)`**（`crates/continuum-workspace/src/persist.rs:177`），
  跨 crate 调不到——故 Task 5 的照片**不是**「候选名等于落库编码」，是「候选名等于两臂的期望串 ＋
  两臂穷尽 `match`」。**这一处是本计划对设计 §10.2 第 (p) 条的据实收窄**，见 Task 5 与 `## 遗留`。
- **`Evidence` 与 `MethodRegistry` 两个 crate 尚未落地**（跨计划前置第一节实测）。
  凡 Task 6 / Task 7 的代码块写到的签名，都是**照 P5a 设计 §4.4 与 P5b 设计 §4.5 转抄的**，
  落地时以 crate 源码为准。

**`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；而且逐份各不相同，不许拿一个值套两处。**
本仓已为此付过代价（P3D 的计划给三份样例共用一个错误码 `E0603`，实测 rustc 1.95 下三份各不相同）。
**本计划给出的任何错误码都只是预期值**，落地时以 `trybuild` 实际产出的 `.stderr` 为准。

---

# 文件结构

```
crates/continuum-code/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/error.rs        CodeError（两臂）
  src/operators.rs    all_operators() 与九枚算子的声明
  src/register.rs     register_code_operators 与 REPRODUCIBLE_BACKENDS（§3.5 的封闭名单）
  src/methods.rs      bind_code_methods（**Task 7，硬前置：P5b 的 crate 已交付**）
  tests/operators.rs             九枚声明的逐列断言（§10.2 的 f/g/h/i）与 §8 的端口取值域
  tests/register.rs              注册的正例与反例、§3.5 的名单（§10.2 的 a/b/c/d/e）
  tests/schema_edges.rs          §3.4 的边表（§10.2 的 j）
  tests/reuse.rs                 §3.6 的复用照片（§10.2 的 o；dev 边 continuum-graph）
  tests/workspace_backends.rs    §4.2 的后端名照片（§10.2 的 p；dev 边 continuum-workspace）
  tests/methods.rs               §7.1 的条目（§10.2 的 k/l/m/n；**硬前置：P5b 的 crate 已交付**）
  tests/type_level.rs            trybuild 驱动（**硬前置：P5a 的 crate 已交付**）
  tests/compile_fail/*.rs        两份编译失败样例（各配同名 .stderr）
  tests/ui_pass/*.rs             正控制样例（编译通过）
```

**本计划要改的既有文件**（只有两处，都在切分 §一 的共写文件表里）

```
Cargo.toml（workspace）                                  members 加 crates/continuum-code（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 加 continuum-code 条目（Task 1 起，逐 task 增补内层数组）
```

**本计划不碰的文件**：`crates/continuum-runtime/src/**`（本块没有装配面）、
`crates/continuum-operator/**`（只 `use` 它的类型与注册表）、
`crates/continuum-artifact/**`（只 `use` `ArtifactType` / `ContentHash`，一个变体都不加）、
`crates/continuum-graph/**`、`crates/continuum-workspace/**`、`crates/continuum-port/**`、
`crates/continuum-core/**`、`crates/continuum-persist/**`、`crates/continuum-events/**`、
`crates/continuum-policy/**`、`crates/continuum-capability/**`、`crates/continuum-secrets/**`、
`crates/continuum-sandbox/**`、`docs/02-工程.md`、`docs/01-总纲.md`、`docs/spec/**`、
`docs/superpowers/specs/**`（六份设计一份都不改）。

---

## 各 task 的「运行，确认失败」一步量到的是哪一种失败（逐 task 扫过）

**这一步的名字只在「本 task 有新生产代码」时才成立。** 逐 task 实测的形态如下——
凡量到**编译失败**的，按纪律 1(c) **不算红**，那一步是空转的，本计划在那里**照实标注为编译期照片**；
凡本 task **不交付生产代码**（只新建测试）的，测试对的是 Task 1 已交付的声明，**首次即绿**，
那一类的步名不叫「确认失败」：

| task 的那一步 | 量到的是 | 为什么 |
|---|---|---|
| Task 1 Step 3 | **编译失败**（`E0432` / `E0433`） | `all_operators` 的声明面由 Step 4 才建出 |
| Task 2 Step 2 | **编译失败**（`E0432`） | `register_code_operators` / `REPRODUCIBLE_BACKENDS` 由 Step 3 才建出 |
| Task 3 Step 2 | **首次即绿** | 本 task 只新建测试（Step 3 明写「无生产代码」），断言的是 Task 1 已交付的声明 |
| Task 4 Step 2 | **首次即绿** | 同上（`cache_key` / `can_reuse` 是 P1 已交付的） |
| Task 5 Step 2 | **首次即绿** | 同上（`WorkspaceBackend` 是 P2 已交付的） |
| Task 6 Step 2 | **运行期红**（trybuild 报样例与 `.stderr` 不符） | 样例与 `.stderr` 都在本 task 内，`.stderr` 尚未取值 |
| Task 7 Step 2 | **编译失败**（`E0432` / `E0433`） | `bind_code_methods` 由 Step 3 才建出 |
| Task 8 | 无「确认失败」步 | 它是收尾与复核 |

**「首次即绿」不是缺陷，也不许改写成「先红后绿」**：那三个 task 的交付物就是断言本身，
它们的红由 Task 8 Step 5 的变异复核取（改 `operators.rs` 的某一列 → 对应断言红）。

---

### Task 1: `continuum-code` 骨架、`CodeError` 与九枚算子的声明

**Files:**
- Create: `crates/continuum-code/Cargo.toml`
- Create: `crates/continuum-code/src/lib.rs`
- Create: `crates/continuum-code/src/error.rs`
- Create: `crates/continuum-code/src/operators.rs`
- Create: `crates/continuum-code/tests/operators.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加一行 `crates/continuum-code`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-code` 条目）

**Interfaces:**
- Consumes: `continuum_artifact::ArtifactType`；`continuum_operator::{Operator, OperatorId, OperatorVersion, Determinism, SideEffectClass, BackendId}`
- Produces: `continuum_code::{all_operators, CodeError}`

**`CodeError` 的落点是本 task**。两臂，**不是三臂**——P5e 的 `MediaError` 有第三臂 `NotPermitted`，
而代码域**没有对应的授权面**（§34 管的是「生成内容替换原始素材」，九枚算子里没有任何一枚产「生成内容」）。
**这不是「为对称补一臂」：一条永远进不去的臂与一个死字段同类。**

- [ ] **Step 1: 建 crate 骨架并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`continuum-artifact`、`continuum-operator`、`thiserror`
（`CodeError` 上派生 `Error`，去掉即 `E0433`）。`continuum-graph` / `continuum-workspace` /
`continuum-verify` / `continuum-method` / `trybuild` **由需要它们的 task 增量加**。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加（**只列本 task 实际有的边**）：

```rust
    // P5c 代码领域算子（设计 §2.1）。本块只**声明**算子，不实现执行体，
    // 故生产代码只用 artifact 的端口类型与 operator 的算子类型。
    // 其余四条边由需要它们的 task 增量加：graph / workspace / verify 三条是 dev 边
    // （唯一使用点分别是对应用例），method 是普通边。
    ("continuum-code", &["continuum-artifact", "continuum-operator"]),
```

**条目不能省**——`every_crate_depends_only_on_its_allowed_set` 要求**每个** workspace 成员都在表里，
漏了即红；而 `workspace_crates()` 从 workspace 定义派生（`:234`），故**加 member 之后、补表之前**
该门是红的。**这是刻意的**（切分 §一 的共写文件表逐字写着这一点），不要靠「先不加 member」绕过。

- [ ] **Step 2: 写用例**

`tests/operators.rs`：

- `the_return_type_pins_the_count_and_the_ids_are_pairwise_distinct`：
  **两个断言，性质不同，注释里要分清**。
  （i）`all_operators().len() == 9`——**它是一条守卫，钉的是「返回类型的数目正好是 9」**
  （设计 §10.1 的第一条）。**它不是恒真的**：清单少一枚在 Rust 里编不过
  （数组字面量的元素个数与返回类型不符），故删一枚**必须同步**把返回类型写成 `[Operator; 8]`，
  而那一处改动**会让本条红**——这正是设计 §10.3 的「移除」那一行预告的 `len() == 9`（纪律 1(b)）。
  **它不钉的是什么**要一并写明：它**不**钉「九枚的 id 与 §3.2 表逐行对得上」，
  也**不**该被写成「清单里没少东西」这一层意思的唯一载体——逐行对应由本节其余各条与
  Task 4／Task 5 的逐枚断言承担。
  （ii）**九个 id 两两比较**（不是只查总数）全不相同，且每枚 `version == 1`。
  红条件：把第 5 行与第 6 行的 id 改成同一个（**取反档**）→（ii）红而（i）绿。
  **锚点唯一**：`operators.rs` 里第 5 行那个 `OperatorId::new("code-spec-review")`。
- `the_side_effect_class_of_every_row`（§10.2 第 (f) 条，**逐枚，不抽代表**）：
  第 2、3、5、6、9 行 == `SideEffectClass::Pure`；第 1、4、7、8 行 == `SideEffectClass::Idempotent`；
  **九枚无一为 `NonIdempotent`**（第三条单独一行断言，与前两条合起来才是完整的）。
  红条件：把第 4 行的 `side_effect_class` 改成 `NonIdempotent`（**取反档**）。
  **锚点唯一**：`operators.rs` 里 `code-implement` 那一行的 `side_effect_class` 字段。
  注释里写明来历：第 4 行 2026-10-10 由 `NonIdempotent` 改判 `Idempotent`——理由是
  「重跑得到另一份补丁」属 `Determinism`（两者正交），而「消耗资源」不是「改变状态」（设计 §3.3 第 8 条）。
- `the_determinism_of_every_row`（§10.2 第 (g) 条，**逐枚**）：
  第 7、9 行 == `Determinism::Deterministic`；第 1、2、3、4、5、6、8 行 == `NonDeterministic`。
  红条件：把第 1 行（`code-workspace`）改成 `Deterministic`（**放宽档**，范围少了一半条件）。
  **锚点唯一**：`operators.rs` 里 `code-workspace` 那一行的 `determinism` 字段。
  **并写明**：`worktree` 与 `overlay` 两个候选**都在** §3.5 的名单里，故这一条变异**不会**让
  Task 2 的注册期规则变红——**§3.5 的规则与这一条是两条独立的守卫**（设计 §3.5 末）。
- `code_workspace_takes_no_artifact_input_and_yields_a_source_tree`（§10.2 第 (h) 条）：
  第 1 行的 `input_schema` 是空 `Vec`（`Vec::is_empty()`），`output_schema == [ArtifactType::SourceTree]`。
  红条件：给它加一个 `[ArtifactType::Json]` 输入（**取反档**；按纪律 1：改的是被断言的声明取值，
  把「空输入」改成「与期望不是同一枚」）。
  注释里写明出处（设计 §3.3 第 2 条）：**Base 不是制品**，给它一个制品身份等于让下游把 Base 当输入制品消费，
  而 §256 的 `AI_WRITE(BaseWorkspace) = DENY` 正是要拦住这件事。代价是 Base 的形态不进端口。
- `the_two_review_operators_differ_only_in_id`（§10.2 第 (i) 条）：
  第 5 行与第 6 行的 `input_schema` / `output_schema` / `determinism` / `side_effect_class` / `backend_candidates`
  **逐字段相同**，而 `id` 不同。红条件：把第 6 行的 `input_schema` 改成 `[Patch]`（**取反档**；
  同形的改取值变异在 Task 3 也记取反，设计 §10.3 给这一形状的档位即取反）。
  **比较的写法要写死：用不带 `..` 的穷尽解构**（把 `Operator` 的七个字段逐个绑定再逐字段比较），
  **不许用 `..`**——留了 `..` 的话，`Operator` 将来多出一个字段时本条**静默照过**（纪律 1(c) 末段）；
  不带 `..` 时多一个字段即**编译不过**，那张照片才是「逐字段相同」而不是「我列出的那几列相同」。
  注释里写明：§186 逐字「规格审查与代码质量审查**分离**」（该节列的四条优势之一），而**分离这一条只有两枚同形的东西才说得上**
  （设计 §3.3 第 10 条）。
- `the_port_types_in_use_are_exactly_the_four_types_of_this_chain`：§8「本链零请求」的**可观察形态**。
  遍历九枚算子的 `input_schema ∪ output_schema`，断言出现的型**恰是**手写的四枚
  `{SourceTree, Patch, TestResult, Json}`（**手写期望，不从遍历结果派生**）。
  红条件：给某一枚算子加一个 `ArtifactType::Blob` 端口（**取反档**）。
  **并写明限度**：这条钉的是「本链用到的型恰是那四枚」，**不钉**「那四枚在 `ArtifactType` 里还在」
  ——后者由 `continuum-artifact` 自己的用例钉（本块一个变体都不加）。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test operators
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`src/operators.rs` 的声明面由 Step 4 才建出，故本步的失败形态是
`error[E0432] unresolved import`／`error[E0433] failed to resolve` 一类（`all_operators` 尚不存在），
不是断言失败。**这是本 task 的刻意顺序**：九枚的逐列取值与它的断言写在同一节里，
声明面一写出，取值也就定了，再把取值在下一节改一遍就是二次转录（这正是本计划要避免的）。
**若要取到运行期的红**，须在 Step 1 就给出 `all_operators` 的**签名**（函数体先写作 `todo!()`），
届时本步的红是**用例 panic**（运行期，纪律 1(c) 认它）。**本计划的判据按前者记**（不强制走这条）；
**若实现者走后者，那一行 `todo!()` 不许活到 Step 5 的提交里**。

- [ ] **Step 4: 实现**

```rust
/// 本领域九枚算子的全集。**顺序即设计 §3.2 表的行序**，用例按行序取。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**：`Operator` 含 `OperatorId`（字段私有、`new` 非 `const fn`）与三枚 `Vec`
/// 字段，在 crate 外构造不出常量值；本仓带 `ALL` 常量的两处成立的前提是那些类型**无字段**。
/// 数目 9 写在**返回类型**里，使清单个数的改动**编译期可见**——这是本行要保住的那条性质。
pub fn all_operators() -> [Operator; 9];

/// 两臂。**不为对称补第三臂**：代码域没有 §34 那一类授权面（见本 task 的说明）。
#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    /// §3.5 的注册期规则不成立。判定的持有者在本块。
    #[error("算子 {operator} 的候选 backend {backend} 不在逐位可复现名单内（§3.5）")]
    UnverifiableDeterminism { operator: OperatorId, backend: BackendId },

    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1。
    #[error(transparent)]
    Registry(#[from] OperatorError),
}
```

九枚算子的逐列取值**照设计 §3.2 的表**，本计划不重抄一遍（重抄就是第二份转录，正是本项目出错最多之处）；
**实现时对着该表的九行逐行填**，并逐行在 `operators.rs` 里留下出处注释（§3.2 表末「出处」列的原话）。

**设计 §3.3 的十一条逐枚理由，逐条给出落点**（此前只落了其中一部分——第 2、8、10 条与第 3 条，
其余各条没有落点要求，此处补齐；纪律 4：没有照片的明写为什么没有）：

| §3.3 | 理由 | 落点 |
|---|---|---|
| 第 1 条 | 第 1 行不在《总纲》§8.3 的六步上，来源两处 | 无断言：它是「为什么有这一枚」的理由。**`operators.rs` 第 1 行的出处注释**要写它；登记面是 Task 5 的 `p` |
| 第 2 条 | 第 1 行的 `input_schema` 为空 | Task 1 的 `h`（用例注释已引） |
| 第 3 条 | 第 1 行是 `NonDeterministic`（fail-closed 的一侧） | Task 4 的 `o` 的第 1 行那一枚（`cache_key` 为 `None`）；**注释要引 §3.3 第 3 条** |
| 第 4 条 | 第 1 行是 `Idempotent` 而非 `NonIdempotent` | Task 1 的 `f` 第 1 行；**注释要引 §3.3 第 4 条**（与第 8 条并列） |
| 第 5 条 | 第 7、8 行分作两枚，不合成一枚 | 无独立断言：可观察面是 Task 1 的 `g`（第 8 行为 `NonDeterministic`）与 Task 4 的 `o`（第 8 行 `cache_key` 为 `None`，故 fuzz 不进复用）；**注释要写「若合成一枚，那一枚的 `Deterministic` 声明当场为假」** |
| 第 6 条 | 第 7 行是 `Deterministic`，因此进复用 | Task 4 的 `o` 的第 7 行；**注释要引 §3.3 第 6 条**（它是收益，不是缺口） |
| 第 7 条 | 第 2、3、4、5、6 行是 `NonDeterministic` | Task 1 的 `g` 的第 2–6 行；注释写明「规范没有一处给过 operator 的 determinism」这一条代价 |
| 第 8 条 | 九枚都不是 `NonIdempotent` | Task 1 的 `f` 的第三条断言（用例注释已引） |
| 第 9 条 | 第 7、8 行是 `Idempotent` 而非 `Pure` | Task 1 的 `f` 第 7、8 行；**注释要引 §3.3 第 9 条**（执行体自己落盘） |
| 第 10 条 | 第 5、6 行是两枚算子，不是一枚 | Task 1 的 `i`（用例注释已引） |
| 第 11 条 | 第 9 行是「§262 第 1 级的判定器」，不是 P5a 的判定 | 无断言：比较器的实现不在本块（本块只声明算子）。**`operators.rs` 第 9 行的出处注释**要写它，并写明「级别序与候选过滤属 P5a」 |

**上表里标「无断言」的三条（第 1、5、11 条）是理由而非守卫**——它们没有可拍的照片，
据实写成注释而不是用例；**不许为了凑覆盖把它们写成恒真的断言**（纪律 1(b)(ii)）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(code): crate 骨架、CodeError 与九枚代码域算子的声明"
```

---

### Task 2: `register_code_operators`（先核后写）与 §3.5 的确定性名单

**Files:**
- Create: `crates/continuum-code/src/register.rs`
- Create: `crates/continuum-code/tests/register.rs`
- Modify: `crates/continuum-code/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 1 的 `all_operators` / `CodeError`；`continuum_operator::{Operator, OperatorRegistry, OperatorId, BackendId}`
- Produces: `continuum_code::{register_code_operators, REPRODUCIBLE_BACKENDS}`

**名单的取名是本计划自定的一处**（设计 §3.5 只说「一份封闭表」，没给函数名或常量名）。

- [ ] **Step 1: 写用例**

`tests/register.rs`：

- `registering_the_whole_set_then_resolving_each_succeeds`（§10.2 第 (a) 条）：
  以 `&all_operators()` 调 `register_code_operators`，逐枚 `resolve(&id, &OperatorVersion::new(1))` 成功。
  红条件：把 `register_code_operators` 改成空实现（**移除档**）→ 逐枚 `resolve` 得 `NotFound`。
- `registering_twice_yields_the_registry_arm_not_a_silent_overwrite`（§10.2 第 (b) 条）：
  同一注册表注册两次 ⇒ `Err(CodeError::Registry(OperatorError::Duplicate { .. }))`，
  **判定的那一臂要判准**（用 `matches!` 而不是「是 `Err` 就算过」）。
  红条件：把重复注册改成静默覆盖（**移除档**）。
  **锚点唯一**：`register.rs` 里把 `OperatorError` 转进 `CodeError::Registry` 的那个 `?`。
- `every_deterministic_operator_has_all_its_candidates_in_the_list`（§10.2 第 (c) 条，**逐枚**）：
  对每枚 `determinism == Deterministic` 的算子，其每个 `backend_candidate` 都在名单内。
- `the_list_is_exactly_these_five_names`（§10.1 的逐项断言）：
  **手写**五项 `{"worktree", "overlay", "builtin", "cargo-test", "local-test-runner"}`，
  断言 `REPRODUCIBLE_BACKENDS` 与之**逐项相等**（集合相等且个数相等）。
  **期望值必须手写、不从常量自身取**——否则是自证（纪律 1(b)(ii)）。
  红条件：从常量里删掉 `"builtin"`，**并把类型字面量同步改成 `[&str; 4]`**（**移除档**）
  → 本条红、**(c) 也红**（(c) 遍历九枚里 `determinism == Deterministic` 的两枚——第 7、9 行——
  逐枚判候选是否在名单内，而第 9 行 `code-verify` 的候选就是 `builtin`（设计 §3.2 表的 backend 候选列），
  删掉该项后第 9 行立刻不成），**而 (a) 不红**（它不涉及名单）。
  **类型字面量那一半要写进红条件**：`REPRODUCIBLE_BACKENDS` 是 `[&str; 5]`（本 task Step 3），
  只删一项而不改类型即**编译不过**，按纪律 1(c) **不算红**。
  **这一句要写进注释**：它说明为什么本条与 (c) 都要留。
- `an_operator_with_an_unlisted_backend_is_rejected_before_any_write`（§10.2 第 (d) 条，**两个子例，两条都要**）：
  （i）造一枚 `determinism == Deterministic` 而候选含名单外 backend（如 `BackendId::new("remote-cluster")`）的算子，
  连同若干合法算子一次传进去 ⇒ `Err(CodeError::UnverifiableDeterminism { operator, backend })`，
  **且判准 `operator` 与 `backend` 两个字段的值**；
  （ii）**同一调用之后，注册表内容与调用前逐枚相同**（先核后写，不留半注册）。
  红条件一：把名单判定改成恒真（**取反档**）→（i）红。
  红条件二：把函数改成边核边写（先注册前几枚、遇到违规才返回）（**取反档**）→（ii）红而（i）**仍绿**。
  **（i）为什么仍绿**：「边核边写」不改返回的 `Err` 值（仍是 `UnverifiableDeterminism { operator, backend }`），
  故 (i) 的两个字段判据照过；设计 §10.3 那一行也只预告 (ii) 那一个子例红、并明写「不该红 c、a」。
  **若 (i) 在这一档下也红了，说明两件不相干的事被混进了一条断言。**
  **（ii）是 fail-open 的那一侧**（纪律 7）：只钉（i）会让「注册表被写坏了一半」漂过去。
- `an_operator_with_an_id_outside_the_table_is_accepted`（§10.2 第 (e) 条）：
  用表外的 `OperatorId`（如 `"codew"`）造一枚候选全在名单内的 `Deterministic` 算子，
  与合法算子混合注册 ⇒ 逐枚成功。
  红条件：给 id 加一层白名单校验（**移除档**）。
  **这一条是上一条的另一侧**：否则 (d) 可以由「一律拒」满足——那是一条把合法输入也拒掉的假守卫。
  注释里写明：`OperatorId` 的取值域**不由本块封闭**（`crates/continuum-operator/src/definition.rs:24-31`
  是有字段的 newtype，无枚举），本块**不发明**一个 id 白名单。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test register
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`tests/register.rs` 要 `use continuum_code::{register_code_operators, REPRODUCIBLE_BACKENDS}`，
而这两个符号由 Step 3 才建出（`src/register.rs`），故本步的失败形态是
`error[E0432] unresolved import`，不是断言失败，**这一步是空转的**。
**要取到运行期的红**，须先有能编译、能跑出断言的 `register.rs`——而那一写就与 Step 3 重合，
故本 task 不做（本计划的判据以 Step 4 的全量跑与 Task 8 Step 5 的变异为准）。

- [ ] **Step 3: 实现**

```rust
/// §3.5 的「逐位可复现的 backend」封闭名单。**内容是设计 §3.5 定的，规范没有一处给 backend 的确定性。**
/// 名单里为什么有 `overlay`：§255 给非 Git 项目列的五种形态（`OverlayFS` / `Btrfs snapshot` /
/// `ZFS clone` / `CoW workspace` / `container layer`），而 P2 已落地的
/// `WorkspaceBackend`（`crates/continuum-workspace/src/backend.rs:22`）只有两臂；本块取那两臂，
/// 不按 §255 的五种各造一个名字——那会让候选里出现三个没有实现的取值。
pub const REPRODUCIBLE_BACKENDS: [&str; 5];

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核 §3.5 的声明，全部通过后再逐枚注册；任一枚不成即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚代码域算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§3.5 那条规则要能对**任意** `Operator` 触发，
/// 否则 §10.2 第 (d) 条写不出来。
pub fn register_code_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), CodeError>;
```

**这条规则拦不住哪一类（要写进函数的文档注释，否则会被当成一条全覆盖的守卫）**：它只查
「声明 `Deterministic` 的算子，其候选是否逐位可复现」，**不查「这个算子的真实输入是否在端口上」**。
`code-workspace` 正是后者：它的两个候选都在名单内，故把它声明成 `Deterministic` **能通过**本规则，
而 Task 1 的 `g` 判它错。**两者是两条独立的守卫，缺任一条都留下一类静默错误**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code
git commit -m "feat(code): register_code_operators 的先核后写与逐位可复现 backend 名单"
```

---

### Task 3: §3.4 的边表——§239 的注册期一半

**Files:**
- Create: `crates/continuum-code/tests/schema_edges.rs`

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_artifact::ArtifactType`
- Produces: 无生产类型（**语料是测试内的手写表**；本块不建端口兼容判定函数）

- [ ] **Step 1: 写用例**

`every_declared_edge_has_the_type_on_both_sides`（§10.2 第 (j) 条）：

**形态：两组断言，各钉设计 §3.4 判据的一侧**——**这里不是「一份三列语料同时钉两侧」**：
三列语料只覆盖它写到的那些型，**语料之外的取值两侧都是 fail-open**（给某枚下游算子加一个
语料里没列过的型，三列语料全绿）。故把两半各自交给一条**独立**的断言：

**（A）下游覆盖**——手写一份「有上游算子的下游 → 该下游在设计 §3.4 表里列出的型集合」的表
（`code-decompose` → `{Json}`；`code-implement` → `{Json}`；`code-spec-review` → `{Json, Patch}`；
`code-quality-review` → `{Json, Patch}`；`code-test-run` → `{SourceTree, Patch}`；
`code-fuzz-run` → `{SourceTree, Patch}`；`code-verify` → `{TestResult, Patch}`），**逐行断言两件事**：
（A1）表里列出的**每个**型都在该下游的 `input_schema` 里；
（A2）该下游的 `input_schema` 里**每个**型都在表里列出——**两个方向都判，合起来是集合相等**。
**（A2）承担的就是「下游的每一个型都能找到上游」这一半**：下游若多出一个语料外的型，
（A2）红，而在 (B) 里那一型根本没有对应的行——两处一起把「下游有孤儿型」逼出来。

**（B）上游来源**——手写三列语料 `(下游算子 id, 上游算子 id, 型)`，逐行对应设计 §3.4 的边表，
逐行断言该上游的 `output_schema` 含该型。

**（A）＋（B）就是设计 §3.4 的判据**：下游的每一型都在 (A) 的表里（A2 的后一个方向），
表里的每一型在 (B) 里都有一枚上游的输出含它。**这不是把设计判据改写**——设计写的就是
两半合起来的那一句，本计划只是**把两半各自交给一条独立的断言**：三列语料是 (B) 那一半，
它**不能**同时充当 (A)，否则下游侧只剩「语料枚举到的那几个型」，正是 fail-open 的那一侧。

- 红条件（下游侧·删型）：把 `code-spec-review` 的 `input_schema` 改成 `[Patch]`（去掉 `Json`）（**取反档**）
  →（A1）红（表里列了 `Json` 而下游没有）。
  **这一条正是设计 §10.3 预告的那一行**（它的判据是「下游的每一型都要能找到上游」）。
- 红条件（下游侧·增型）：给 `code-test-run` 的 `input_schema` 加一个 `ArtifactType::Json`（**取反档**）
  →（A2）红。**这一条是三列语料单独用时会漏掉的那一侧**——把它写进红条件，才说明 (A) 不是摆设。
- 红条件（上游侧）：把 `code-decompose` 的 `output_schema` 改成 `[Patch]`（**取反档**）
  →（B）里 `(code-implement, code-decompose, Json)` 那一行红。
  **三条红条件各指一个唯一锚点**（分别是第 5、7、3 行的端口字段）。
- **串 → 算子的查找方式要写死**：本用例与 Task 7 的 `l`／`m` 都按**串**指认算子，故测试里要有一个
  辅助函数，形态是「在 `all_operators()` 里按 `id` 线性查找，**找不到即 `panic!` 并打印那个串**」，
  两处**同形**。**不许写成「找不到就跳过」**：那样 Task 1 的 id 变异（把第 5、6 行的 id 改成同一个）
  会让本用例**静默通过**；写成 panic 之后，那一处变异在本用例的红是「找不到 `code-quality-review`」
  这一条 panic——**红的位置可解释**。
- **`code-plan` 那一行不入本用例**：它的输入由装配方以 `Json` 值投射、**不由任何算子的 `output_schema` 提供**，
  故它**没有上游算子可指**（设计 §3.4 的「一处例外」）。**若不写明这一点，本用例会被读成判据的反例。**
  它的上游是 §234 的 `Milestone` 与 §228 的 `Plan`（都是 P4 的对象），**以值**由装配方投射。
- **`code-workspace` 那一行的三元组数为零**（它的输入是空的，Base 不在端口上）——
  **明写为零，而不是「不在表里」**：两者在代码里长得一样，但前者是一条断言
  （它在 (A) 的表里，型集合为空集，且它的输入确实是空的——Task 1 的 `h` 钉后者），后者是漏写。

**依据栏的读法照设计 §3.4 写进注释**：语料里写「§186 的 `Implementation Plan`→`Engineering Tasks` 两步」这一类的是规范正文的步骤序；
写「§3.2 第 N 行」的是**该行的 `input_schema` 与全表 `output_schema` 的对应**（本设计的判定，规范未给边）。

**（A2）比的是「下游的 `input_schema`」与「设计 §3.4 表为该行列出的型」，不是「下游的输入与上游的输出
两侧相等」**——后者是设计 §3.4 末与 P5e 设计 §5.3 已否掉的那条读法（例如 `code-spec-review` 的
输入是 `{Json, Patch}`，而它的两枚上游的输出分别是 `{Json}` 与 `{Patch}`，两侧并不相等）。
本用例断言的始终是「交集中的型」，即 (B) 的逐行含型判定。

**这条用例钉不住什么（写进注释）**：它只判**下游的每一型都有一枚上游的输出含它**，
**不判**「图上的边该不该存在」（那是 Planner 的构造），也**不判**「两侧的 schema 相等」；**不替代** P1 的
`compatible`（`crates/continuum-port/src/port.rs:94`）——本块不写第二份那类判定（切分 §四第 7 条）。

- [ ] **Step 2: 运行（本 task 无生产代码，预期首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test schema_edges
```

**预期是绿**：本 task 只新建测试，断言的是 Task 1 已交付的声明，故这里没有「先红后绿」。
**若这一步红了，说明 Task 1 的声明与设计 §3.4 的边表不符**——那是要回 Task 1 查的，
不是本 task 的用例写错。这一条本 task 的红由 Task 8 Step 5 的变异复核取。

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付用例**。**不建**端口兼容函数、**不建**边表的常量、**不建** `Port` 的构造
（`continuum-port` 这条边本块不登记：设计 §2.1 逐字给了理由——本块的 `input_schema` / `output_schema`
取 `Vec<ArtifactType>`，不经 `Port`）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code
git commit -m "test(code): §3.4 边表的两侧断言——注册期可查的那一半"
```

---

### Task 4: §3.6 的复用照片——本块不建第二份判据

**Files:**
- Create: `crates/continuum-code/tests/reuse.rs`
- Modify: `crates/continuum-code/Cargo.toml`（dev-dep `continuum-graph`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（内层数组加 `continuum-graph`）

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_graph::{cache_key, can_reuse, CacheKey}`；`continuum_artifact::ContentHash`
- Produces: 无生产类型（本块**不建**第二个复用判据）

- [ ] **Step 1: 登记 dev 边并写用例**

`every_deterministic_operator_produces_a_cache_key_and_every_other_does_not`（§10.2 第 (o) 条，**逐枚**）：

- 第 7、9 行的 `cache_key(op, ContentHash::of(b"..."))` 是 `Some`；
  第 1、2、3、4、5、6、8 行是 `None`。**逐枚，不是抽样。**
- `can_reuse` 的**两侧**（纪律 7）：
  正例——第 7 行配它自己的 `CacheKey`、`input_hash` 未变、`contract_affected = false` ⇒ `true`；
  反例逐条——`input_hash` 变 ⇒ `false`；`operator_version` 变（造一枚 `version = 2` 的同 id 算子）⇒ `false`；
  `contract_affected = true` ⇒ `false`；`cached = None` ⇒ `false`。
- 红条件：把 `code-workspace` 的 `determinism` 改成 `Deterministic`（**放宽档**）
  → 第 1 行由 `None` 变 `Some`，本用例红、Task 1 的 `g` 红；
  **而 Task 2 的 (c) 不红**（`worktree` / `overlay` 两个候选都在名单里）——
  **这正是设计 §3.5 末点名的那两条独立守卫**，注释里要点明这一句。
- **锚点唯一**：`operators.rs` 里 `code-workspace` 那一行的 `determinism` 字段。

注释里写明本 task 的**性质**：它不是「本块实现了一个复用判据」，而是
「本块的 `determinism` 声明**让 §305 的判据成立或落空**」——§305 的三条件已由 P1 落地
（`crates/continuum-graph/src/reuse.rs:29`），**本块不另写一个**（切分 §四第 7 条的第一枚实例）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test reuse
```

**预期是绿**（同 Task 3 Step 2）：本 task 无生产代码可写，断言的是 Task 1 的 `determinism` 声明
与 P1 已交付的 `cache_key` / `can_reuse`。红了先查那两处，不是查本用例。

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付用例与一条 dev 边**。**不建** `can_reuse` 的包装、**不建** `CacheKey` 的存储
（`CacheKey` 今天没有存储，P1 设计 §18 把 `can_reuse` 列在「无执行点的机制」里——本块不替它建表）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "test(code): §3.6 的复用照片——determinism 声明是 §305 的前提"
```

---

### Task 5: §4.2 的后端名——两个候选与 P2 两臂的对应

**Files:**
- Create: `crates/continuum-code/tests/workspace_backends.rs`
- Modify: `crates/continuum-code/Cargo.toml`（dev-dep `continuum-workspace`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（内层数组加 `continuum-workspace`）

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_workspace::WorkspaceBackend`
- Produces: 无生产类型

**本 task 交付的照片比设计 §10.2 第 (p) 条写的**窄一半**，理由是实测出来的**：
设计把照片坐标指到 `backend_str`（`crates/continuum-workspace/src/persist.rs:177`），
而**它是 `pub(crate)`**——`continuum-code` 里**调不到它**。故本落地的照片是
「候选名等于两臂的期望串 ＋ 两臂穷尽 `match`」，**不是**「候选名等于落库编码」。
**这一条据实报出**（`## 遗留`），**不在本块就地补**（补它要么改 P2 的可见性、
要么加一条 `continuum-persist` 的 dev 边去读 `workspace` 列的原文——两者都超出设计 §2.1 的六项边表）。

- [ ] **Step 1: 登记 dev 边并写用例**

`the_workspace_candidates_are_the_two_declared_backends`，**两半合起来才算钉住**：

- **（p1）运行期侧**：第 1 行的 `backend_candidates` 与**手写的** `{"worktree", "overlay"}`
  **集合相等**（个数也判，故「删一项」红）。
  **不判次序**：候选列表在本块没有任何断言读它的次序，**「两项换序」是一枚等价变异体**
  （设计 §10.3 末行已写明）；**要打红它必须换变异体：删掉一项**——那时本条与 Task 2 的
  `the_list_is_exactly_these_five_names` 都红。**这一句要写进注释**，否则后来者会拿换序去验守卫。
  红条件：把第 1 行的 `overlay` 写成 §255 的散文名 `overlayfs`（**取反档**）→**（p1）红而（p2）仍绿**。
  **（p2）为什么仍绿**：它读的是 `WorkspaceBackend` 与**测试里手写的**期望串，
  **不读算子的 `backend_candidates`**，故改算子的那一列不动它。
  **若本意是让（p2）也红**，那要把它写成「`match` 的结果等于**算子的候选集合**」——那时它变成
  运行期断言，与它自陈的「编译期照片」混在一处，本计划**不取**那条写法（（p2）的用途只有一个：
  「P2 给枚举加一臂即编译不过」）。
  **锚点唯一**：`operators.rs` 里第 1 行的 `backend_candidates`。
- **（p2）编译期侧**：对 `WorkspaceBackend` 做**穷尽 `match`、不带 `..`**，每臂返回期望的候选串，
  断言 `match` 出来的集合等于**测试里同一份手写期望**——**（p1）与（p2）都只读 `WorkspaceBackend`
  与手写期望，两者都不读算子的 `backend_candidates`**（故（p1）的红条件改算子那一列时（p2）不动）。
  **这是一张编译期照片，照实写成编译期照片**：P2 若按 §255 的非 Git 项目那五种形态给 `WorkspaceBackend`
  加第三臂，`crates/continuum-code/tests/workspace_backends.rs` **编译不过**
  （具体错误码取自实跑，不许凭记忆写）。它**不是**「用例变红」。
  **它成立的前提是 `match` 的入参是枚举自身**——故这里的 `match` 收 `WorkspaceBackend`，
  而**不是**收 `&str`（收 `&str` 的解析函数一定有通配臂，做不成这张照片）。
- **并写明这条照片的限度**：它**不钉** `workspace` 列的落库编码——
  若 P2 只改 `backend_str` 的两个字面量而两臂名不变，**本用例不红**。
  该处的权威是 `backend_str` 本身（`persist.rs:166-176` 的注记写明这两个编码**显式给出、
  不由 serde 表示或 `Debug` 推出**），故本块**不拿 serde 表示替代它**：
  两者今天恰好同串，而它们是两件事（设计 §3.5 末）。

- [ ] **Step 2: 运行（本 task 无生产代码，预期首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test workspace_backends
```

**预期是绿**（同 Task 3 Step 2）：本 task 无生产代码可写，断言的是 Task 1 第 1 行的
`backend_candidates` 与 P2 已交付的两臂。红了先查那两处。

- [ ] **Step 3: 实现（无生产代码）**

**不建**工作区、**不调** `create_task_workspace` / `discard_task_workspace`——L2 工作区的创建在 P2，
本块只**声明**一枚算子来命名它（设计 §4.2）。**不把 Integration Gate 算子化**（同节）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "test(code): §4.2 的两个 backend 候选名与 P2 两臂的对应"
```

---

### Task 6: `Evidence` 的不可构造性（trybuild）

> **硬前置：P5a 的 `continuum-verify` 已交付，且 `Evidence` 的八字段私有、无 `Default`、
> 唯一构造点 `Evidence::from_tool_result` 已落地**（跨计划前置第一节实测：该 crate 今天不存在）。
> **未交付时本 task 动不了**——`Files` 里要 `use` 的那个类型在仓里还不存在，`cargo test` 起不来。
> **本 task 不改任何 P5a 的文件**：它只在本 crate 里写样例。

**Files:**
- Create: `crates/continuum-code/tests/type_level.rs`
- Create: `crates/continuum-code/tests/compile_fail/evidence_literal_is_private.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-code/tests/compile_fail/evidence_default_is_absent.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-code/tests/ui_pass/evidence_is_reachable.rs`
- Modify: `crates/continuum-code/Cargo.toml`（dev-deps `continuum-verify`、`trybuild`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（内层数组加 `continuum-verify`）

**Interfaces:**
- Consumes: `continuum_verify::Evidence`（**唯一产生点的使用侧**）
- Produces: 无生产类型

- [ ] **Step 1: 登记 dev 边并写用例**

`evidence_cannot_be_constructed_bypassing_its_only_constructor`：两份 `compile_fail` 样例，
**按文件主干配对**（`evidence_literal_is_private.rs` ↔ `evidence_literal_is_private.stderr`），
每份 `.stderr` **取自实跑**、不许凭记忆写、**两份各不相同不许套用同一个错误码**。

- 样例一：`let _ = Evidence { .. };`——字段私有。
- 样例二：`let _ = Evidence::default();`——无 `Default` 实现。
- **样例里注释也是照片坐标**：trybuild 把样例源码逐行打印进 `.stderr` 的输出里，
  故**改一句注释就会改 `.stderr`**——注释与代码一样是这张照片的一部分，**改任一处都要重跑取新值**。
- **正控制**（`tests/ui_pass/evidence_is_reachable.rs` ＋ `t.pass(...)`）：
  只取该构造点的**函数项**（不构造值、不传八个实参），证明「这个 crate 在本 crate 的编译环境里够得着」。
  **这条正控制的用途只有一个**：把「因为依赖没接上而编译失败」与「因为字段私有而编译失败」分开——
  **没有它，样例二的红可能是任何一种红**。
- **红条件照实写成编译期照片**：P5a 若给出 `Default` 实现、或把 `Evidence` 的字段开成 `pub`，
  样例转为**编译通过**，trybuild 报「expected test case to fail to compile」。
  **这不是「用例变红」**（纪律 8）；本块**没有**运行期照片能拍到「绕过构造点造出了 `Evidence`」。
- **这条证不到什么（写进注释）**：它证不到「产出只能经本块的某枚函数」——
  `Evidence::from_tool_result` 是 `pub`，**调用点在装配方**，本块不建第二枚包装（设计 §6.2）。
  故本块**没有**那类照片，也不该有。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test type_level
```

预期：**头一次跑会失败**——`.stderr` 还没有内容（或与实跑不符）。**把实跑输出写进 `.stderr`**，
再跑一次确认绿。**不许先生成 `.stderr` 再写样例。**

- [ ] **Step 3: 实现（无生产代码）**

本 task **只交付样例与一条 dev 边**，**不写任何证据构造代码**（设计 §6.2：不建 `evidence_from_code_output`）。
`EvidenceType` 的取值（`ModelReview` / `Test` / `Fuzz` / `Property`）由装配方在调用时给出，
**本块的生产面里一个字都不写死它**——这正是「本块不替 P5a 决定证据类型」的可观察形态。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "test(code): Evidence 唯一产生点的使用侧照片（trybuild）"
```

---

### Task 7: `bind_code_methods`——§7.1 的七条条目

> **硬前置：P5b 的 `continuum-method` 已交付，且 `MethodRegistry::seeded()` 已建出 §187 的
> `software/` 七条条目**（跨计划前置第一节实测：该 crate 今天不存在，P5b 的实现计划也不存在）。
> **未交付时本 task 动不了**——`Files` 里要 `use` 的 `MethodRegistry` 在仓里还不存在。
> **本 task 不 `register` 任何方法条目**（P5b 设计第六节：四块不得自造 `MethodEntry`）。

**Files:**
- Create: `crates/continuum-code/src/methods.rs`
- Create: `crates/continuum-code/tests/methods.rs`
- Modify: `crates/continuum-code/{src/lib.rs,Cargo.toml}`（导出、普通依赖 `continuum-method`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（内层数组加 `continuum-method`）

**Interfaces:**
- Consumes: Task 1／Task 2 的 `all_operators` / `register_code_operators`；
  `continuum_method::{MethodRegistry, MethodId, MethodError}`
- Produces: `continuum_code::bind_code_methods`

- [ ] **Step 1: 写用例**

`tests/methods.rs`：

- `every_software_entry_has_the_declared_realized_by`（§10.2 第 (k) 条，**逐条**）：
  **手写**设计 §7.1 的七行期望值，**逐条**断言（`planning` / `worktree` / `review` / `fuzz` /
  `verification` 五条非空，`TDD` 与 `debugging` **为空 `Vec`**）。
  **期望值不许写成「非空即通过」**——P5b 判据 6 的形态是「非空」，而本块**有意**让两条为空
  （协调者 2026-10-10 已裁定），写成「非空即过」会把那条裁定当场抹掉。
  红条件：把 `TDD` 的 `realized_by` 从空改成 `["code-implement", "code-test-run"]`（**取反档**）→ 本条红。
  **同时预告（写进注释）**：那一条变异下，下一条 (`l`) **仍绿**——因为那两个 id 都能 `resolve`。
  **若 (l) 也红了，说明 (l) 不是按 (k) 的方式写的**。
- `every_id_in_a_nonempty_realized_by_resolves`（§10.2 第 (l) 条）：
  先把 `&all_operators()` 注册进一个 `OperatorRegistry`，再对五条非空 `realized_by` 里的**每个串**
  `resolve(&OperatorId::new(s), &OperatorVersion::new(1))` 成功。
  **这一条是本块的补件**：`bind` 的参数是**文本**，本块**无法在编译期核对它与 `all_operators()` 的 id 一致**
  （P5b 设计 §3.2 末已把这条弱引用的无照片写清）。
  红条件：把 `fuzz` 一行的 `code-fuzz-run` 改成 `code-fuzz-runs`（**取反档**）→ 本条红、而 (k) 红
  （(k) 的期望里写的是正确的串）。**两条同时红是预期**，注释里要写明。
  **串→算子怎么找要写死**：这一条与上面的 `m` 都按「串」指认算子，故测试里要有一个辅助函数，
  形态是「在 `all_operators()` 里按 `id` 线性查找，**找不到即 `panic!` 并打印那个串**」。
  **不许写成「找不到就跳过」**：那样 Task 1 的 id 变异（把第 5、6 行的 id 改成同一个）会让本块
  以**静默通过**而非红收场。写成 panic 之后，那一处变异在本块的红是
  「找不到 `code-quality-review`」这一条 panic——**红的位置可解释**。
  查找方式要在 `tests/methods.rs` 与 `tests/schema_edges.rs` 两处**同形**（Task 3 的语料同样按串指认算子）。
- `exactly_the_eight_declared_operators_are_referenced_by_the_directory`（§10.2 第 (m) 条）：
  **手写**那八个 id 的名单（`code-workspace` `code-plan` `code-decompose` `code-spec-review`
  `code-quality-review` `code-test-run` `code-fuzz-run` `code-verify`），断言这八个**都**出现在
  某个非空 `realized_by` 里，**且 `code-implement` 不出现在任何一条里**。
  **语料必须手写**：若写成「遍历 `all_operators()`、再对每枚去 `realized_by` 里找」，
  它测的是「每个算子都在某个 `realized_by` 里」——而「某条目录条目绑的算子是否存在」不会被测到
  （那是 `l`），反过来「有没有孤儿算子」也不会被 `l` 测到。**语料从被测清单里取就是恒真的假照片。**
  红条件：把 `planning` 一行填成 `["code-implement"]`（**取反档**）→ 本条红而 (k) 仍绿
  （那一行仍是非空的）。**这一对红绿是本条存在的理由。**
  注释里写明：`code-implement` 不被任何 `realized_by` 引用**不构成缺陷**——
  §186 的 `Implement` 是链上的一步，而 §187 的 `software/` 七条是**方法名**（「怎样做得可靠」），
  两者不是同一张表。
- `bind_returns_the_method_error_type`（§10.2 第 (n) 条）：
  编译期照片——`let _: Result<(), MethodError> = bind_code_methods(&mut registry);` 编译通过。
  红条件：把返回类型改成 `Result<(), CodeError>`（**取反档**）→ **编译不过**（照实写成编译期照片）。
  注释里写明判据：`MethodError::NotFound` 的判定是「方法库里有没有这条方法」，
  与 `CodeError` 的两臂不同源，故**直接透出、不并进 `CodeError`**；
  这与把 `OperatorError::Duplicate` 包进 `CodeError::Registry` **是同一条判据的两面**
  ——判定是同一件事则合并，是两件事则分开。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-code --test methods
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`bind_code_methods` 由 Step 3 才建出，故本步的失败形态是 `error[E0432]`／`error[E0433]`
（`continuum_code::bind_code_methods` 尚不存在），不是断言失败，**这一步是空转的**。

- [ ] **Step 3: 实现**

```rust
/// 把设计 §7.1 表里七条 `software/` 的 `realized_by` 填进方法库。
/// 调用形态：对**每一条已 seed 的 id** 调一次 `MethodRegistry::bind`（P5b 设计第六节），
/// 其中 `TDD` 与 `debugging` 两条按已关闭的裁定填**空**。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
/// **不做二次登记**：不 `register` 方法条目——七条 id 由 P5b 的 `seeded()` 给出。
pub fn bind_code_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;
```

**七条 `realized_by` 的取值照设计 §7.1 的表**（本计划不重抄；实现时对表逐行填，
并在 `methods.rs` 里留下每一行的理由注释）。

**设计 §7.3（§186 的流程名不入 `software/` 目录，答复 P5b 设计 R6）的落点就是本函数**：
它的交付物是**没有任何一行 `register`**——`bind_code_methods` 只对 P5b `seeded()` 给出的七条 id
调 `MethodRegistry::bind`，**§186 的步骤名（`spec-review` / `quality-review` 一类串）一个字都不出现**
（P5b 设计第六节禁止四块自造 `MethodEntry`）。这一条**没有可拍的照片**（它是「代码里没有那一行」），
据实写成函数文档注释里的一句；它的**可观察近邻**是 Task 7 的 `m`（§186 的 `Implement` 那一步落成的
算子 id `code-implement` 不被任何 `realized_by` 引用）。

`TDD` 与 `debugging` 为空 `Vec` 的**理由要写在函数文档里**（不是「还没填」）：
`TDD` 的全部内容是**次序**（先写测试、后写实现），而 `realized_by` 的类型是算子 id 的一个**集合**，
**表达不了次序**——填两枚 id 会让 `TDD` 与「先实现后补测试」在登记面上不可区分；
`debugging` 在链上没有对应的步骤（《总纲》§8.3 的六步与 §186 那十行流程块都没有调试），
且规范没有一处给过「调试的输入与产物是什么」。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-code crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(code): bind_code_methods——software/ 目录的七条 realized_by"
```

---

### Task 8: 收尾与复核

**Files:**
- Modify: `crates/continuum-code/**`（**仅在复核发现缺口时**）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**仅在对账不齐时**）

- [ ] **Step 1: 全量验证（门读数按纪律 3 读）**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5c-final-test.log
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets 2>&1 | tee .tmp/p5c-final-build.log
```

预期：全绿、0 warning。**判据不是退出码**：逐条读日志里 **`Running` 的行数**与
**`test result:` 的行数**（Doc-tests 算一条），确认射程覆盖全部测试目标；
`--no-fail-fast` 必须带上（本仓的 `cargo test` 不收 `--keep-going`）。**管道结尾不许接 `tail`。**

- [ ] **Step 2: `ALLOWED` 与 `Cargo.toml` 的终态核对**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

**逐条核四件事**：
（a）`continuum-code` 的条目**恰为**设计 §2.1 共写文件表给的那六项，一个不多一个不少；
（b）`crates/continuum-code/Cargo.toml` 与 `ALLOWED` **精确一致**（`[dependencies]` 与 `[dev-dependencies]`
里**外部** crate 不进表，内部 crate 逐条对上）；
（c）**没有**指向 `continuum-core` / `continuum-persist` / `continuum-events` / `continuum-policy` /
`continuum-capability` / `continuum-port` / `continuum-semantics` / `continuum-secrets` / `continuum-sandbox`
的边；
（d）**本块没有动任何迁移**：`cargo test -p continuum-runtime --test migrations` 与 `--test startup`
仍全绿（本块零表，`runtime_migrations()` 一个字都没改）。
**并把粒度写清**：`--depth 1` 只钉**直接边**；传递可达在 Rust 下不可 `use`，
**故直接边即这条规则的完整粒度**，**不另设闭包断言**。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
grep -rn "continuum_core\|continuum_persist\|continuum_events\|continuum_policy\|continuum_capability\|continuum_port\|continuum_semantics\|continuum_secrets\|continuum_sandbox" \
  crates/continuum-code/src
grep -rn "OperatorImpl\|propagate_invalidation\|register_code_operators\|bind_code_methods" crates/continuum-code/src
grep -rn "SystemTime::now\|Instant::now" crates/continuum-code/src
grep -rn "evidence_from_" crates/continuum-code/src
grep -rn "EvidenceType::" crates/continuum-code/src
```

**五条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；
**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：
（a）九个层外 crate 零 `use`；（b）`OperatorImpl` **零实现**（本块不建执行体）——
而 `register_code_operators` / `bind_code_methods` 两个名字**必然出现**（它们是本块的交付物），
这一条的判据是「**没有调用方**」而不是「名字不出现」；
（c）系统时钟零命中（本块不取时钟）；
（d）`evidence_from_*` 零命中（不建第二枚包装）；
（e）`EvidenceType::` 在生产面（`src/`）零命中——**它由装配方给，本块不写死**。
**作用域写死**：这五条 grep 只扫 `src/`，**不含测试目录**（`tests/reuse.rs` 等要用 `continuum-graph` 的类型）。

- [ ] **Step 4: 逐条核对设计 §2.4 的发明判据**

设计 §2.4 的表是本块「发明判据」的总账，每一行给**代价／边界／落点／具名收件人**四件。
**本计划的动作是逐行核「落点栏是否指向一条真实存在的用例」**，不重述该表：

| §2.4 的行 | 落点栏指的东西 | 本计划的用例 |
|---|---|---|
| 第 1 条（九枚的 `determinism`） | §3.2 表的 `determinism` 列 | Task 1 的 `g`、Task 4 的 `o` |
| 第 2 条（九枚的 `side_effect_class`） | §3.2 表的 `side_effect_class` 列 | Task 1 的 `f` |
| 第 3 条（九枚的 `backend_candidates`） | §3.2 表的 backend 列 | Task 5 的 `p`、Task 1 的 `h`／`i` |
| 第 4 条（逐位可复现名单与注册期规则） | `register_code_operators` 的核 | Task 2 的 `c`／`d` |
| 第 5 条（§3.4 边表的边判据） | §3.4 边表 | Task 3 的 `j` |
| 第 6 条（`code-workspace` 算子化、Gate 不化） | `all_operators()` 返回的第 1 行 | Task 5 的 `p` |
| 第 7 条（§7.1 的 `realized_by` 映射） | §7.1 表、`bind_code_methods` | Task 7 的 `k`／`l`／`m` |
| 第 8 条（哪几步产 `Evidence`） | §5.2 表 | **本块的 crate 里无照片**——产出面是**声明**，调用点在装配方（Task 6 的注释已写明） |
| 第 9 条（迁移档号 `150`） | §9.3 | **本块零迁移，无照片**；Task 8 Step 2(d) 核的是「本块没动任何迁移」 |
| 第 10 条（九枚的 `input_schema` / `output_schema`） | §3.2 表的两列 | Task 1 的 `h`／`i` 与端口取值域那条、Task 3 的 `j` |

**第 8、9 两条的落点栏指的是「文档/登记」，不是用例**——它们在本块的 crate 里**拍不出照片**，
这一点**据实写在上面**（纪律 4：没有照片就要明写为什么没有），**不标注为覆盖**。
设计 §2.4 另有两处**报出而非发明**的判定（§5.3 的 `ModelReview` 臂已由 P5a 落地；
§5.4 的 `TestResult` 载荷 schema 属规范维护者 ＋ P5a）——**本块不为它们建用例**。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑**承重守卫**（两侧对钉的、fail-open 侧的、失败路径判别 `Err` 的、跨 crate 才可见的），
每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 → 还原后 `sha256sum` →
独立日志路径。**逐轮在报告里附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位（取反／放宽／收紧／移除；改被断言的声明取值的变异记取反并写明改的是哪一列）」**。

**设计 §10.3 的预告表逐行照跑**：其中「把 `code-spec-review` 的 `input_schema` 改成 `[Patch]`（去掉 `Json`）」
那一行，在**设计给的单侧判据**下不会红（单侧读法下它更容易满足），本计划的 Task 3 把那一半交给
**（A1）这条独立断言**（表里列了 `Json` 而下游没有 ⇒ 红），故它**会红**；该行的其余预告（谁该红、
谁不该红）照设计读。

**三条跨 crate 的承重守卫必须跑全量**：
（i）`ALLOWED` 与实际依赖一致（加/删一条边即红）；
（ii）新 crate 进 workspace 后**迁移集合与计数不受影响**（本块零迁移）；
（iii）Task 5 的 `WorkspaceBackend` 穷尽 `match`——**它的变异是「给 P2 的枚举加一臂」，
红的时点应在编译期；若它跑成了「编译失败」而不是「用例失败」，按纪律 1(c) 记为「编译不过」，
并另换一个真变异体**（把第 1 行的 `overlay` 改成 `overlayfs`，**取反档**，
红的是（p1）——（p2）不读算子，见 Task 5 Step 1）。
**注意（iii）的变异要改的是 `continuum-workspace` 的文件——不属于本块的交付物**，
故它只能在**本地工作树**上做、且**必须还原**；**报告里要写明这一点**，
**不许把那个改动提交**。

**等价变异体不许算作守卫**（跑它们是浪费，且会被误读成「守卫有效」）：
`code-workspace` 的 `backend_candidates` 两项**换序**（Task 5 已写明它是等价的）。

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。
**本计划不新建、也不修改第二份文档**：若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 7: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs(code): P5c 代码领域算子的收尾与复核"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**

### 一、设计问题：**本计划查出的相抵与缺口**（发现即报，本计划一处都不替设计补写）

1. **§10.2 第 (p) 条的照片坐标指向一个 `pub(crate)` 的函数**（**实测**）：
   该条写「`code-workspace` 的两个候选逐名等于 `WorkspaceBackend` 两臂的编码
   （`crates/continuum-workspace/src/persist.rs:177-182` 的 `backend_str`）」，
   而实测 `backend_str` 是 **`pub(crate)`**（`:177`）——**跨 crate 调不到**。
   故本块**没有**「候选名等于落库编码」这张照片。
   **本计划的取舍**：Task 5 照片取**两臂穷尽 `match` ＋ 手写期望串**——它钉住「候选名与两臂的对应」
   （P2 加一臂即编译不过），**不钉**「候选名与 `workspace` 列的编码一致」。
   **翻转条件**：若协调者要那张更强的照片，缺的是下面两条之一（**两处都不在本块**）——
   （i）把 `backend_str` 的可见性放开，或（ii）允许本块加一条 `continuum-persist` 的 dev 边、
   经 `Tx::query` 读 `workspace` 表的 `backend` 列原文。**（ii）会超出设计 §2.1 的六项边表。**
   **收件人：P2 ＋ 协调者。**
2. **§10.3 的预告表有一行在它自己给的判据下不会红**（**实测相抵**）：
   该行写「**取反**：把 `code-spec-review` 的 `input_schema` 改成 `[Patch]`（去掉 `Json`）→
   j 的第 5 行」，而 §3.4 的判据里「下游的每一型都要能找到上游」那一半，若**只**由一份
   枚举式三列语料承担，则**从下游删一个型会让它更容易满足**，那一行变异就会**全绿**（等价变异体）。
   **本计划的处置**：Task 3 把设计判据的两半各自交给一条**独立的**断言——
   （A）「下游的 `input_schema` 与该行的语料型集合**两个方向都相等**」承担下游侧
   （删型 ⇒（A1）红；**增型 ⇒（A2）红**），（B）三列来源语料承担上游侧（上游删型 ⇒ 红）。
   该变异随之真红，且不再有「语料外的型」这个 fail-open 的口子。**收件人：设计作者 ＋ 复审者。**
3. **§3.5 的封闭名单没有规范来源**（**设计已自陈并已给收件人**）：§245 只给「一个 Operator 多 backend」，
   不给 backend 的确定性。缺的是「该 backend 的什么属性使输出逐位可复现」这一步。
   **本计划照 §3.5 落地**（名单五项写在 `REPRODUCIBLE_BACKENDS`，逐项有 Task 2 的用例）。
   **收件人：规范维护者。**
4. **`TestResult` 的载荷 schema 未给**（**设计已自陈**）：缺的是「一份 `TestResult` 里哪些字段是
   判『通过/不通过』所必需的」这一步。**本块不发明它**（设计 §5.4 第 4 条）。
   **收件人：规范维护者 ＋ P5a。**
5. **两个「Plan」同名**（**设计已自陈**）：§228 的 `Plan`（P4 的对象）与 `code-plan` 产出的
   `Json` 制品（§186 的 `Implementation Plan` 一步）。**本块不改名、不合并。**
   **收件人：P4 的持有方 ＋ 协调者。**
6. **`code-workspace` 的「每 Intent 一枚」在类型层不可钉**（**设计已自陈**）：
   算子是每节点的执行单元，而「图上有几个该节点的实例」是 Planner 的构造。
   **本块的字面里没有它的照片**。**收件人：协调者（Planner 的归属未定）。**

### 二、设计里的「待对账 / 待裁」标记：**已关闭的两条**（留格是为了让「一处说法被订正」可查）

- **§7.2 的 `TDD` / `debugging` 无算子对家** —— **已关闭（2026-10-10）**：协调者取该节处置表的**第三案**
  （认这两条无算子对家，把 P5b 判据 6 的「非空」收窄为「除 `TDD` 与 `debugging` 外非空」），
  落点是 P5b 的 `UNREALIZED_BY_DESIGN` 常量。**两案被否的理由也写在 P5b 设计第十节判据 6。**
  Task 7 的 `k` 照该裁定写（两条**为空**，且不与「非空即过」混读）。
- **§5.3 的 `ModelReview` 臂无臂可落** —— **已关闭（2026-10-10）**：P5a 收下本块报出的臂并落地
  （P5a 设计 §4.2 末三行，枚举增至十四臂）。**本块的证据产出面因此没有待定项**；
  §5.2 表里那两枚算子的 `EvidenceType` 由装配方在调用时给出，**本块不写死**（Task 6 Step 3）。

### 三、对既有之物的请求（**本计划一处都不实施**，逐条见跨计划前置第三节）

- **`backend_str` 的可见性**：见第一节第 1 条。**收件人：P2 ＋ 协调者。**
- **Base 不在端口上**（设计 §4.3）＋ **`NodeContext` 没有工作区句柄**（设计 §4.4）：
  同一处改动的两半，本块**不就地补**（切分 §四第 4 条）。
  **两侧代价已写在设计里**：本轮该缺口的表现是 `code-workspace` 声明 `NonDeterministic`
  ——代价落在「不复用」上，不落在「复用了错的」上（fail-closed 的那一侧）。
  **收件人：P1 ＋ 第 3 层的执行器。**
- **迁移档 `150` 只在本块一侧写死**：P5d 与 P5f 的设计各已写出处置（两块均「不占档」，
  P5f 另建议自取 `180`）；**仍真的一半只剩 P6 未声明**。缺的是「六块与 P6 的档谁在哪一步合并成一张表」。
  **本块不新增任何迁移**（实测 `130`–`199` 全段零命中）。**收件人：协调者。**

### 四、拍不到照片的地方（设计 §10.5 已列，**逐条照原文，本计划不发明**）；本计划补三条

- **端到端**（一个代码 Intent 从 `Queued` 走到某处）：执行器未建，且切分 §四第 4 条禁止 P5 自建执行路径。
  本块的全部照片是**单元与集成层**。
- **`code-workspace` 真的建出一棵隔离工作区**：本块只声明算子，`OperatorImpl` 不在交付物里。
- **§305 的复用真的省下了一次重跑**：`can_reuse` 今天无生产调用方，且 `CacheKey` 无存储。
- **§307 的「非幂等不自动重试」在本块没有主体**：九枚无一为 `NonIdempotent`。
- **两条臂的证据真的产了出来**：产出面是**声明**，构造点在宿主的执行代码。
- **本计划补：手册侧的三条**——（i）**§5.2 的产出面对应关系**（哪几枚产哪种 `EvidenceType`）
  在本块的 crate 里**无照片**，它是**声明**（第一节第 8 条已写在 §2.4 的核对表里）；
  （ii）**§9.3 的档号 `150`** 是**登记**，无照片；
  （iii）**五个 `register_code_operators` / `bind_code_methods` / `all_operators()` 的生产调用方**
  今天不存在（第 3 层未建），故「它们被谁调用」无照片——**这不是本块的缺口**（设计 §12 第 9 条）。

### 五、本计划自定的形状与取值（**申报**，逐条说清代价）

- **`REPRODUCIBLE_BACKENDS` 的取名**（Task 2）：设计 §3.5 只写「一份封闭表」，**没给名字**。
  **代价**：名字是本计划定的；**收益**：那张表有一个可被用例指认的落点。
- **§3.4 边表的判据拆成两组断言**（Task 3）：设计 §3.4 把判据写成一句（下游的每一型能在
  该行的上游里找到，且那一枚的输出含该型），本计划把它拆成
  **（A）下游覆盖**（手写的「下游 → 型集合」表，两个方向都断言）与 **（B）上游来源**（三列语料）。
  **代价**：多一份手写表；**收益**：设计 §10.3 预告的那一行变异真红，且**下游新增语料外的型**
  这一侧不再是 fail-open（第一节第 2 条）。
- **§4.2 的照片取「两臂穷尽 `match` ＋ 手写期望串」**（Task 5）：见第一节第 1 条。
  **代价**：落库编码那一半无照片；**收益**：不为此加一条设计边表之外的 dev 边。
- **`all_operators()` 的返回类型是 `[Operator; 9]`**（Task 1）：**不是本计划自定**——设计 §3.2 已写死
  并给了两条理由。留这一条是为了让「数目写在返回类型里」这件事可查，
  并附上它的**副作用**：`len() == 9` 钉的是**返回类型的数目**（设计 §10.1 的守卫），
  它不钉「九枚的 id 与 §3.2 表逐行对得上」——后者由 Task 1 的 `f`／`g`／`i` 与 Task 4／Task 5 的逐枚断言承担。
- **Task 1 的 `the_port_types_in_use_are_exactly_the_four_types_of_this_chain` 是本计划新增的一条守卫**
  （§8「本链零请求」的可观察形态：九枚的 `input_schema ∪ output_schema` 恰为手写的四枚型）：
  它**不在设计 §10.2 的 (a)–(p) 之内**，内容与 §8 一致但设计没给它编号。
  **代价**：比设计多一条用例；**收益**：§8 的「零请求」有可跑的形态，而不是只写在设计的正文里。
  **（这一条先前漏申报，此处补入。）**
- **Task 6 与 Task 7 排在最后**：**不是风格偏好**，是跨计划前置第二节那条硬前置的直接后果。

### 六、设计 §11 的逐条对账条目与 §12 的遗留：**本计划不重述其内容**

设计 §11 的每一条都以「待与某方对账」开头并给了具名收件人，§12 的每一条都以「缺的是哪一步」写到底。
**重述就是第二份转录，正是本项目出错最多之处。** 本计划的动作只有两件：
（i）**把已关闭的两条移进上面的第二节**（§11 第 2 条与 §12 第 1、2 条已由 2026-10-10 的裁定与 P5a 的落地关闭）；
（ii）**把仍未关闭、且本块是消费方或登记方的四条搬进上面的第三节**（后端名可见性、两条通道缺口、档号）。
**其余各条照原文留在设计里**，收件人不变。

---

## 交付给谁

- **交给 P5a 的 `continuum-verify`**：`code-spec-review` / `code-quality-review` 的 `EvidenceType`
  是 `ModelReview`，`code-test-run` 是 `Test`，`code-fuzz-run` 是 `Fuzz` / `Property`——
  这四条**由装配方在调 `Evidence::from_tool_result` 时给**，本块的 crate 里一个字都不写死它
  （Task 6 Step 3 的源码面复核第 (e) 条）。
- **交给 P5b 的 `continuum-method`**：`software/` 目录的五条非空 `realized_by`（Task 7 的代码）；
  `TDD` 与 `debugging` 按已关闭的裁定留空。
- **交给第 3 层的执行器（未建）**：九枚算子的 `OperatorImpl`、`Queued → Running` 处的 `resolve` 调用点、
  以及本块三个入口（`all_operators` / `register_code_operators` / `bind_code_methods`）的生产调用方。
  **本块只注册、只声明**（切分 §四第 4 条）。
- **交给 P1**：`NodeContext` 的工作区句柄与 `input_schema` 的取值域（跨计划前置第三节）。
- **交给 P2**：`backend_str` 的可见性（同上）。
- **交给规范维护者（本项目无此角色）**：§3.5 的 backend 确定性名单（`REPRODUCIBLE_BACKENDS`）与
  `TestResult` 的载荷 schema。
- **交给协调者**：迁移档 `150` 的合并、`code-workspace` 的基数归属、两个 `Plan` 同名的处置、
  以及上面第一节列出的设计问题（逐条已给翻转条件或收件人）。
- **交给复审者**：本计划的**两处对设计的偏离**（第一节第 1 条与第 2 条）——
  两处都写明了「为什么不是那样」，复审时请对 Task 5 与 Task 3 的用例逐行核。
