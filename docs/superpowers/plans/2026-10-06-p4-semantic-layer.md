# P4（语义层）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建三个新 crate，落《工程》§2.1 的**十二个组件**（`docs/02-工程.md:76-89`）：
`continuum-canonical`（Input Canonicalization Frontend §270 §196 §197 ＋ Reference Resolver §271–§278 ＋ Alias 存储）、
`continuum-semantics`（Intent §222 §223、Specification §227 §214、Contract §224 §226、ContractDiff §226、
Plan 与审查状态机 §228 §229 §338–§341、Plan Change 分类 §230、Decision §331 §332、Constraint Validator §225 §6）、
`continuum-budget`（预算树、`Allocation` / `Reservation` / `Remaining`、预算账本、Budget Validator §110、探索预算 §334）。

**Architecture:** **本层不依赖任何下层**（`docs/02-工程.md:72`）——落地形态是「**本层的三个 crate 不登记任何指向执行层／资源层／边界层的 `Cargo.toml` 依赖边**」，
由既有的 `ALLOWED` 逐对断言守卫（`crates/continuum-runtime/tests/dependency_direction.rs:22-160` 的表、`:265-283` 的逐对比对）。
三个 crate 的允许集**只含 `continuum-persist` 与本层前驱**：
`continuum-canonical ← continuum-persist`、`continuum-semantics ← continuum-canonical, continuum-persist`、
`continuum-budget ← continuum-semantics, continuum-persist`（设计 §2.2）。**本层落地时一条跨层边都不登记。**
凡跨层输入一律以**值**由装配方传入（候选集、`Node.constraints` / `Node.execution_policy`、
失败读数、`Now`），凡跨层输出一律以**类型**被下层消费（`ContractDiff`、`Remaining`）——本层不 `use` 它们的定义 crate。

**本层是判断，不是执行**：「不符合格候选时强行绑定」不发生在代码里（§4.2 的四条断言）、
「`unknowns` 被当成空表」不可表达（`Unknowns` 两变体的类型，§6.1）、
「已违反 REQUIRED 的候选计划被执行」不可表达（`ConstraintVerdict` 由调用方处置，本组件不挂起不写状态，§11.4）、
「`verification_requirement` 被判定」（OPEN-001 未决期间**结构性禁读**，§7.2）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-persist`（`Tx` / `Migration` / `Db` / `Value` / `PersistError` /
`RecoveryHook` / `RecoveryPhase`）；`continuum-semantics` 与 `continuum-budget` 各消费层内前驱；
`serde_json`（`Node.execution_policy` 原样值的容器）；`thiserror`；`trybuild`（类型层不可构造性与方法集的编译期照片）；
dev：`tempfile`。

**设计依据：** `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`（**唯一事实来源**，1987 行）。
**协调者裁定记录：** `docs/superpowers/2026-10-06-p4-decisions.md`（设计 §12.6.1 是它的转述，以那份为准）。
**本计划不改设计、不改规范、不改代码以外的任何计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-06 实读工作树）

| 依赖 | 状态 | 本层用在哪 |
|---|---|---|
| P0 `continuum_persist::{Db, Migration, Tx, Value, PersistError}`（`crates/continuum-persist/src/{db.rs:14,db.rs:72,tx.rs:12}`） | **已交付** | 三个 crate 的全部读写；迁移 100 / 110 / 120 |
| P0 `continuum_persist::{RecoveryHook, RecoveryPhase, RecoveryRegistry, run_recovery}`（`src/recovery.rs:9-15`、`:39`） | **已交付** | §12.3 第 2 处的恢复钩子（挂 `RecoveryPhase::MarkLostExecutions`） |
| P0 `Tx::append_event` / `Tx::append_audit`（`src/tx.rs:49`、`:79`） | **已交付** | §5.4 的六个写入点 |
| P1 `continuum-graph` 的 ADFIR 图、状态机、失效传播（`src/{graph.rs,node.rs,invalidation.rs}`） | **已交付** | **本层不使用**（§8.2：`propagate_invalidation` 由执行层调）；本层按 §11.2 以**原样值**读 `Node.constraints` / `Node.execution_policy` 的**形状** |
| P1 `continuum-artifact` / `continuum-port` / `continuum-operator` | **已交付** | **本层不用** |
| P2 `continuum-workspace` 的 Gate 与 Effect Journal（`src/gate.rs`） | **已交付** | **本层不用**；只在 §5.4 记「`AuditKind::UserApprovals` 已有生产方」这一事实 |
| P3A `continuum-capability`（迁移 50） | **已交付** | **本层不用**（对 `CapabilityKind` 的引用为零，§16 第 26 条） |
| P3D `continuum-model-registry`（迁移 80、`BudgetView`） | **部分已交付** | 只有 `BudgetView`（`src/budget.rs:38-44`）与本层有关：§12.6.1 的形状乙要求两侧字段集逐项相同（Task 19） |
| P3D `rank` / `list_registered` | **未交付** | **本层不用**（消费方是 G） |
| P3B `continuum-connector`（无迁移）、P3C `continuum-provider`、P3F/G | 已交付（B）／未交付（C 的 `ProviderRegistry`、F、G） | **本层全部不用**（§2.2 的边表里没有一条指向它们） |
| **本层的三个 crate** | **不存在**（实读 `ls crates/`：无 `continuum-canonical` / `continuum-semantics` / `continuum-budget`） | 本计划的全部交付物 |

**一句话结论**：本层**不依赖 P3 的任何产物**——§2.2 的边表里没有一条指向 P3 的 crate，
十二个组件的判据全部来自 §2.1 的组件表与其背后的规范节，不来自任何 P3 的接口。
这是 §1.2.3 那条规则的直接后果，不是巧合。

### 二、哪些 task 在 P3 未合入前动不了

- **Task 20（装配）是唯一硬依赖 P3 的 task，而且依赖的是「P3 的改动已合入」这件事，不是某个接口**：
  它要改 `crates/continuum-runtime/src/main.rs`（`runtime_migrations()`）、`src/recover_cmd.rs`（钩子注册块）、
  `Cargo.toml`、`tests/dependency_direction.rs`（`ALLOWED`）、`tests/migrations.rs`、`tests/startup.rs` ——
  **这六个文件里有五个是 P3 各子项目正在改的多写者单文件**
  （D 的 Task 5/14 与 G 的 Task 12 都改 `main.rs` 与那两个测试文件）。
  **故 Task 20 必须排在 D 与 G 合入之后**；否则既有两重后果：
  （i）合并冲突落在 `ALLOWED` 与迁移清单这两处逐行敏感的文本上；
  （ii）`startup.rs` 与 `migrations.rs` 的计数断言会互相踩——**计数断言的判别力依赖「作者按实际输出改断言」**
  （`tests/migrations.rs:1-11` 已写明这一点），两个改动方各按自己那一刻的输出改，合起来谁也说不清哪个数是对的。
- **Task 20 的计数必须以「当时工作树上的实际输出」为准，不许预判**：本计划给的是**预期区间**
  （现 9 项 ＋ 本层 3 项 ＝ 12 项；**若 P1 的迁移 21 已落地则为 13 项**），落地时以实跑读出的数字为准。
- **其余 20 个 task 只新建三个 crate 与它们自己的测试，一个既有文件都不碰**
  （除 Task 1 改 `Cargo.toml` 的 members 与 `dependency_direction.rs` 的 `ALLOWED` 加**空条目**——
  这一处是必要的，`every_crate_depends_only_on_its_allowed_set` 的 `:252-257` 要求**每个** workspace 成员都在表里，
  漏了即红）。**它们不因 P3 未合入而编不过**，但按协调者的执行序**一律排在 P3 之后**。

### 三、本层对既有之物的三处请求（**本计划只作消费方／依赖方写，一处都不实施**）

设计 §13.2／§13.4 已列。**收件人分别是执行层（P1）与协调者**，本计划**不为它们建 task**
（那三个文件属别的层的表与类型，改它们会让同一个文件有两个改动方——正是本节第二条要防的形状）。

| 处 | 请求的内容（逐条照录设计，**含会直接报错或静默失效的坑**） | 本层的角色 | 收件人 |
|---|---|---|---|
| `adfir_graph` 缺 `contract_version`（`crates/continuum-graph/src/persist.rs:17-23`、`graph.rs:49`） | **新增迁移 21**：`ALTER TABLE adfir_graph ADD COLUMN contract_version INTEGER NOT NULL DEFAULT 0`。三条落地要求：**（1）必须带 `DEFAULT`**（SQLite 在非空表上加 `NOT NULL` 列而不给默认值直接报错，而该表在生产库里非空）；**（2）不得改迁移 20 的 SQL**（`migrate()` 跳过已记录的 version，改 20 对已建库完全无效——静默失效）；**（3）`DEFAULT 0` 是「图建立时版本未知」的哨兵，读取侧须能把 0 与任何真版本区分开**（`0` 在 `TaskContract.version` 的取值域里不是合法版本） | **消费方**：本层定义 `ContractId` / `ContractRef { id, version }`（设计 §7.3），**不读 `adfir_graph`**，对 `continuum-graph` 的依赖边为零 | 执行层（P1）＋ 协调者 |
| `ContractIdRef` 与 `ContractId` 是同概念两类型（`crates/continuum-graph/src/ids.rs:41`） | 裁定唯一的 `ContractId`：本层定义真类型，`ContractIdRef` 复用（那要登记 `语义层 → 执行层` 这条**合法**方向的边，§1.2.1 的约定：执行层依赖本层），或明写 `ContractIdRef` 是不透明字符串引用、其权威在本层。生产点是**三处**（实读）：`AdfirGraph::new`、`load_graph` 的读回路径（`persist.rs:233`）、各测试 | **依赖方**：本层**定义** `ContractId`（Task 11），**不引用 `ContractIdRef`** | 执行层（P1）＋ 协调者 |
| `ExecutionProfile.cost_budget` 收成 `Allocation`（列在 `crates/continuum-graph/src/persist.rs:60`） | 类型本层交付（`continuum_budget::Allocation`，Task 17），**收紧的动作归执行层的接线 task**。这是对 ENG-005 §五（`…eng-005-budget-accounting.md:111` 判给「D 落地时」）的**一处改判**，判据是那列属执行层的表、而 D 的设计 §11 第 12 条自陈未做 | **依赖方**：本层只交付那个类型（上限，不是余量），**不读 `execution_profile`** | 执行层的接线 task ＋ 协调者 |

**三处的共同点**：本层要的都不是别人的**接口**，而是「别人的表与类型能不能表达本层已经交付的事实」。
故本计划**不改、不重述、不预演**它们的任何接口。

### 四、本节把什么交给谁

- **交给驱动的接线 task（未建）**：`Remaining` → `BudgetView` 的**生产投影**。裁定 §12.6.1 把它判给驱动，
  而驱动今天没有组装 `RoutingRequest` 的那一步——**本计划只交付那条断言（Task 19），不建一个没有调用方的生产函数**
  （§2.3「本层不定义一个没有实现者的 trait」的同一条判据）。
- **交给执行器（执行层，未建）**：`ContractDiff` 的消费——把 `DiffEntry` 映射到节点并调 `propagate_invalidation`
  （设计 §8.2）。本层产出判定所需的最小信息，**不产出节点集合**。
- **交给第 7 层（长期循环，未建）**：`BudgetOwner::SystemMaintenance` 的**创建方**（ENG-005 §三末行判给它）。
  本层只提供该变体与它的入账函数，**没有任何一方会创建它**（`## 遗留`）。
- **交给规范维护者（本项目无此角色）**：设计 §16 里以「收件人：规范维护者」结尾的各条——**本计划一条都不发明**，
  逐条抄在 `## 遗留`。
- **交给协调者**：本计划查出的**十条**设计问题（**三条相抵、四条没写清、两条判据缺口、一条设计已自陈并已给收件人**）——逐条见 `## 遗留` 第一节，
  其中 **Task 17 有一条开工前置挂在其上**（`BudgetError` 的变体三处不一致）；**Task 5 的那条前置已随设计补上
   §4.1.1（`ExplicitConstraint` ＋ `satisfies` ＋ 合取判据）而闭合**。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划新增的 workspace 内边只有这些**（逐条由需要它的 task 登记，**一次不声明齐**）：
  - Task 1：`continuum-canonical` 的 `ALLOWED` 条目**空数组**（本 task 对内部 crate 的引用为零，条目不能省）；
  - Task 7：`continuum-canonical ← continuum-persist`（`alias` 表经 `Tx`）；
  - Task 8：`continuum-semantics ← continuum-canonical`（`Intent.goal` 取 `EntityId` 类型，设计 §1.3 的读法甲）、`← continuum-persist`；
  - Task 17：`continuum-budget ← continuum-semantics`（预算树的 owner 取 `IntentId`）、`← continuum-persist`（账本经 `Tx`）；
  - Task 19／20：`continuum-runtime ← continuum-canonical, continuum-semantics, continuum-budget`（装配迁移与钩子）。
  **上表之外一条跨层边都不登记**——尤其没有指向 `continuum-graph` / `continuum-core` / `continuum-capability` /
  `continuum-model-registry` / `continuum-policy` / `continuum-connector` / `continuum-provider` 的边。
  **外部 crate（`thiserror` / `serde_json` / `trybuild` / `tempfile`）按需加，不进 `ALLOWED`**——那张表只逐对断言
  workspace 成员之间的边（`dependency_direction.rs:22-24`）。
- **`--depth 1` 只钉直接边，这就是这条规则的完整粒度，本计划不另设闭包断言。**
  `cargo_tree_direct` 带 `--depth 1`（`dependency_direction.rs:221-224`），故它**看不到传递可达**：
  「经一个中间 crate 间接够到执行层」不会被这条断言抓住。**但本仓不因此留缺口**——
  Rust 的 crate 可见性要求**直接声明**才能 `use`，未写进 `Cargo.toml` 的传递依赖**写不出**
  `use continuum_graph::…`。故「直接边」恰是这条规则的完整粒度。
  **写这一段的用途**：免得读到上面那条约束时以为它是「任何指向 graph 的边都会变红」——
  **那会是一条假保证**（设计 §1.2.2 规则 1 末段）。
- **`continuum-core` 的四个类型一次都不 `use`**：`ModelId` / `ToolId` / `ConnectorId` / `Usage`。
  「不 `use`」讲的是**依赖边为零**，不是「整篇不出现」——设计 §12.3 与 §13.4 两次**点名** `continuum_core::model::Usage`
  并以它立论（`crates/continuum-core/src/model.rs:51`）。本计划照这条口径写注释与遗留。
- **枚举列的落库编码一律小写、多词以 `_` 连接**，经显式辅助函数读写（`as_str` / `parse` 与类型同址），
  不依赖 serde、不用 `Debug`。**表外取值读回得具体 `Err`**（照 P3D 的枚举列纪律）。
  本层要编码的列：`IntentKind` / `IntentStatus` / `RequirementClass` / `PlanType` / `ReviewState` / `Severity` /
  `PlanChangeClass` / `DiffClass` / `AliasScope` / `DimensionKind` / `LedgerKind` / `BudgetOwner` 的判别串。
- **迁移编号取 100 段，每一档取用前现场核对空号**（不假定，设计 §2.4）：`100` = `alias`（Task 7）、
  `110` = `intent` / `specification` / `contract` / `contract_requirement` / `plan` / `decision` / `decision_result`（Task 8 起）、
  `120` = `budget_node` / `budget_ledger`（Task 17）。
  **两处干扰项不许误判为空号被占**：`60` 出现在 `crates/continuum-persist/src/bin/crash-writer.rs:17` 与
  `tests/crash_atomicity.rs:9`；`100` 出现在 `crates/continuum-persist/tests/migrations.rs:45`（表名 `layer_table`）。
  **四处都在各自的测试／工具 `Db` 里，不进生产链**（`Db::open_with` 的迁移集由调用方给），故它们**不占用**号段。
- **迁移 110 的 SQL 在 Task 8–15 之间逐表追加，Task 20 之后不得再改它的 SQL。**
  依据两条：（i）设计 §2.4 规定 110 含七张表，而「表要在用它之前建」要求每张表在它的第一个使用者之前存在；
  （ii）**在本层到 Task 20 之前，110 只被测试库应用过**（`runtime_migrations()` 到 Task 20 才含它，
  实读 `crates/continuum-runtime/src/main.rs:83-93` 今天只到 `p3d_model_migrations()`），故追加是安全的。
  **Task 20 之后改 110 的 SQL 就是设计 §13.2 第 2 条点名的那处静默失效**（`migrate()` 跳过已记录的 version）。
  **本计划不许在任何 task 里改迁移 1／2／10／20／30／40／41／50／80 的 SQL。**
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不建 ADFIR 图、不调 `propagate_invalidation`、不引用 `continuum-graph`**（设计 §8.2）。
  - **不建 Planner、不重规划、不选工具、不选模型**（§2.1 的组件表里没有它们；§12.4 的「先寻找合法低成本方案」是执行层 Planner 的动作）。
  - **不建 §198 的联合解析**（§3.1：§2.1 的组件表里没有承担联合约束传播的组件，本层不发明算法）。
  - **不建验证与证据**（§2.5：`verification_requirement` **只存不判**，第 8 层才收口）。
  - **不建 §214 的冲突检测器**（§6.1：只建类型，不建检测器——「什么算冲突」规范没给判据）。
  - **不建 §226 的表达式语义比较**（§8.1：判不了的走 `unjudged` 并列字段，**不新增第五类差异**）。
  - **不建会话边界的清退**（§4.6：`TURN` / `SESSION` 的过期时点规范未给；本层只实现「不自动永久化」）。
  - **不建 `BudgetView` 的生产投影函数**（裁定 §12.6.1 判给驱动，而驱动的那一步今天不存在；见 Task 19）。
  - **不为 `BudgetView` 加任何派生**（`crates/continuum-model-registry/src/budget.rs:36-37` 明写不加；
    且"为一条测试去改被断言类型的公开形状"是把断言的成本转嫁给它，见 Task 19）。
  - **不新增 `FailureClass` 变体、不新增 `EventType` / `AuditKind` 变体、不动 `EventType::ALL`**（§5.4、ENG-005 §三.2）。
  - **不新增 `RecoveryPhase` 的态**（`crates/continuum-persist/src/recovery.rs:18` 的注释写着「顺序即 §319 的执行顺序，不得改动」）。
  - **不改 `Node.constraints` / `Node.execution_policy` 的类型、不改 `propagate_invalidation` 的签名**（§13.2 末行）。
  - **不改 `docs/02-工程.md`**（§16 第 30 条的「《工程》§10.2 那一行待订正」不是本计划的事）。

### 本计划特有的两处「签名即判据」，照抄前须对源

- `Migration::new(version: i64, name: &'static str, sql: &'static str)` 与 `Db::open_with(path, Vec<Migration>)`
  （`crates/continuum-persist/src/db.rs:14`、`:72`）；`Tx::query(&self, sql, params: &[Value]) -> Result<Vec<Vec<Value>>, PersistError>`、
  `Tx::execute`、`Tx::commit`、`Tx::append_event`、`Tx::append_audit`（`src/tx.rs:26-84`）。
  **本层的写函数一律不自己 `commit`**（照 P1 的 `apply_transition` 形状；`docs/superpowers/p1-followups.md` §一.1：
  写入函数返回 `Err` 之后调用方必须回滚、不得提交），Task 9／11／15 的失败路径照片以这条为前提。
- `RecoveryHook` 是 `Send + Sync` 的 trait，`fn phase(&self) -> RecoveryPhase`、`fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError>`
  （`crates/continuum-persist/src/recovery.rs:39-42`）；**注册点是 `crates/continuum-runtime/src/recover_cmd.rs:38-41`**，
  **不是 `main.rs`**（实读：`main.rs` 里零命中；该函数是从 `main.rs` 迁过来的，迁移说明在 `recover_cmd.rs:3-7`）。

---

## 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；
   正向的「变红」跑全量是加分。**变异分四档，每一处「预期谁红」都要标档位**：
   **取反**（把判定反过来）／**放宽**（少判一半条件）／**收紧**（多判一半条件）／**移除**（删掉整条守卫）。
   **三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**。
   本层已预先识别的等价变异体有三处：**索引序**（`Ambiguous { candidates }` 的顺序若用 `sort_unstable` 且用例只断集合相等，
   则「改成不排序」是等价的）、**稳定排序**、**可推断的字面量**（`None` 与 `Some(0)` 的断言若夹具里两者恰好同值即等价）；
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
   cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
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
3. **凡注释写绝对措辞，必须有对应用例；且要带时点**——「唯一／一律／只有／全部／没有任何」这类词，
   要么有可写出的照片，要么**明写为什么没有**。**断言的作用域要与事实同宽**：
   「整张表的唯一 X」≠「某个字段的唯一 X」；「本 crate 的源码文本里零命中」≠「全仓零命中」；
   「在本 task 结束时」≠「永远」。**枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条臂各要一条照片，
   不抽代表（手写分支能各自漂移）。
4. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」；**并断言没有半写的副作用**
   （本层的失败面是 `IntentError` / `ContractError` / `PlanError` / `DecisionError` / `ConstraintError` /
   `BudgetError` / `AliasError` 的变体，逐变体至少一条用例；副作用面见 Task 9／11／15／17）。
5. **一个纯数据声明／纯投影的类型没有运行期照片，只钉类型与签名（编译期）；且钉类型的机制必须写死为
   「编译器强制穷尽的解构／结构体字面量」且不得带 `..`。**
   判据（设计 §12.6.1 的裁定文件 `2026-10-06-p4-decisions.md:37-42` 已记为它的初版漏项）：
   **Rust 没有反射——若「把字段名各列一遍」被理解成手写两份名单，那条断言恒真**
   （名单是手抄的，类型改了它不会红），它钉的只是「抄的时候两边一样」。
   **本计划里适用这条的类型**：`Intent`、`Specification`、`Requirement`、`TaskContract`、`ContractDiff`、
   `DiffEntry`、`Plan`、`ReviewFinding`、`ReviewIndependence`、`UserReviewView`、`Decision`、
   `ResolutionResult`、`Candidate`、`Allocation` / `Reservation` / `Remaining`、`Dimensions`、`BudgetView`。
6. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本计划逐处标出「另一侧」是哪一个用例
   （§4.2 断言 A/B/D 在 fail-open 侧、§11.3 的三侧、§11.5 的双向、§12.3 的有读数一侧、§12.6.1 的两侧）。
7. **「删掉 X 即红」要先问「删掉之后行为真的变了吗」**——索引序、稳定排序、可推断的字面量、`..` 豁免
   都会让它成为**等价变异体**（见第 1 条）。凡本计划标了「移除档」的地方，都已先答过这一问。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。

**变异日志是证据，必须活到复审结束**：**实现者保留 `.tmp/`，由协调者在复审结束后清理**。
报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。
**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。

**变异口径分层**：变异**条数**按「有多少条**互不相同**的守卫」定，不按分支数定，且分两档、**不许混成一句「通过」**：
- **承重守卫 → 全量套件**：两侧对钉的守卫、**fail-open 的那一侧**、失败路径**判别哪一种 `Err`**、
  **跨 crate 才可见**的效果（迁移编号与计数、`ALLOWED` 与实际依赖一致、`Remaining` 与 `BudgetView` 的字段集相同）。
- **其余分支 → 受影响 crate 的包级套件**，报告里须**标明证据强度较低**并列出「这一条可能漏掉的跨 crate 观察点」。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；凡与既有 crate 交互的签名
（`Tx::query` / `Tx::execute` / `Value` / `Migration::new` / `RecoveryHook`），实现前须先读该 crate 的源码确认，
不符时以源码为准并回报。

**本计划特有的三处「照抄前须对源」**：

- `Migration.name` 与 `sql` 是 `&'static str`（`db.rs:14`）；`sql` 里多条语句以 `;` 分隔一次执行（照 `p1_graph_migrations` 的形状）。
- `BudgetView` **没有任何 `#[derive]`**，字段全 `pub`、无方法、无构造闸门（`crates/continuum-model-registry/src/budget.rs:38-44`）。
  **故凡涉及它的用例不许写 `assert_eq!(a, b)`**——见 Task 19。
- `crates/continuum-graph/src/node.rs:15` 的字段是 `pub constraints: Vec<String>`、`:18` 是 `pub execution_policy: serde_json::Value`；
  本层按**原样类型**读它们（不收成自己的类型），Task 16 的签名即判据。

**`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；而且逐份各不相同，不许拿一个值套三处。**
本仓已为此付过代价（P3D 的计划 Task 3 给三份样例共用一个错误码 `E0603`，实测 rustc 1.95 下三份各不相同）。
**本计划给出的任何错误码都只是预期值**，落地时以 `trybuild` 实际产出的 `.stderr` 为准。

---

# 文件结构

```
crates/continuum-canonical/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/surface.rs      Surface、normalize、Mention、MentionKind、detect
  src/temporal.rs     Now、TemporalBinding、resolve_temporal
  src/candidate.rs    EntityId、Candidate、CandidateSource、CandidateSources
  src/explicit.rs     ExplicitConstraint、ResolutionScope、EntityKind、CandidateFacts、CandidateSet、satisfies、qualifying
  src/resolver.rs     ResolutionResult、ResolutionConfidence、RequiredStrength、resolve
  src/risk.rs         ResolutionRisk
  src/binding.rs      ExplicitBinding、BindingSource
  src/ambiguity.rs    Canonicalized、CanonicalRepresentation、UnresolvedMention、finish
  src/alias.rs        AliasScope、Alias、promote_alias、children 式的读函数
  src/persist.rs      alias 表（迁移 100）
  src/error.rs        CanonicalError、AliasError
  tests/surface.rs        规范化与 mention 切分
  tests/canonical.rs      和类型、Temporal、Ambiguity Manager
  tests/candidate.rs      EntityId、Candidate、CandidateSource 的方法集
  tests/resolver.rs       四态与 §4.2 的四条断言
  tests/risk.rs           门槛必填、binding 优先序
  tests/alias.rs          四作用域、不自动永久化、显式提升
  tests/type_level.rs     trybuild 驱动
  tests/compile_fail/*.rs 不可构造性样例（各配同名 .stderr）

crates/continuum-semantics/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/ids.rs          IntentId、PlanId、ContractId、RequirementId、DecisionId
  src/intent.rs       Intent、IntentKind、IntentStatus、Priority、ContractRef、GraphIdRef、ProductIdRef、迁移表
  src/spec.rs         Specification、SpecDraft、Unknowns、Unknown、Conflict、UndefinedBehavior、六元组的成员类型
  src/contract.rs     TaskContract、Requirement、RequirementClass、VerificationRequirement
  src/diff.rs         DiffClass、DiffEntry、UnjudgedChange、UnjudgedReason、ContractDiff、diff
  src/plan.rs         Plan、PlanType、PlanVersion、ReviewState、ReviewFinding、Severity、ReviewIndependence
  src/plan_change.rs  PlanChange、PlanChangeClass、classify
  src/review_view.rs  UserReviewView（§341 的八项投影）
  src/decision.rs     Decision、DecisionOption、DecisionResult
  src/constraint.rs   CandidatePlan、PlanFootprint、ConstraintVerdict、ViolatedProhibition、ConstraintError、validate
  src/persist.rs      七张表的迁移 110（逐表追加）与行级读写
  src/error.rs        IntentError、SpecError、ContractError、PlanError、DecisionError、ConstraintError
  tests/{intent,intent_state,spec,contract,diff,plan,plan_change,decision,constraint,type_level}.rs
  tests/compile_fail/*.rs

crates/continuum-budget/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/dimensions.rs   DimensionKind、Dimensions
  src/account.rs      Allocation、Reservation、Remaining
  src/tree.rs         BudgetOwner、分配与 I1/I2/I3
  src/ledger.rs       LedgerKind、append（只追加）
  src/settle.rs       SettleOutcome、settle、无读数规则
  src/validator.rs    BudgetVerdict、check（§110）
  src/exploration.rs  ExplorationVerdict、ExplorationThresholds、check_exploration（§334）
  src/recovery.rs     BudgetReservationRecovery（RecoveryHook，挂 MarkLostExecutions）
  src/persist.rs      迁移 120（budget_node / budget_ledger）
  src/error.rs        BudgetError
  tests/{dimensions,tree,settle,validator,exploration,recovery,persist}.rs
```

**本计划要改的既有文件**（只有 Task 1 与 Task 19／20 碰它们）

```
Cargo.toml（workspace）                                  members 加三个 crate
crates/continuum-runtime/Cargo.toml                      三条新依赖边（Task 20）
crates/continuum-runtime/src/main.rs                     runtime_migrations() 追加三行（Task 20）
crates/continuum-runtime/src/recover_cmd.rs              注册预算恢复钩子（Task 20）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED：三 crate 条目 + runtime 条目
crates/continuum-runtime/tests/migrations.rs             expected_migrations() 同步三行（Task 20）
crates/continuum-runtime/tests/startup.rs                计数与 MarkLostExecutions 钩子行（Task 20）
crates/continuum-runtime/tests/budget_projection.rs      形状乙的两条断言（Task 19，新文件）
crates/continuum-runtime/src/lib.rs                      （仅当需要导出投影断言用的东西；Task 19 预期不改）
```

**本计划不碰的文件**：`crates/continuum-graph/**`（三处请求只记遗留，见跨计划前置第三节）、
`crates/continuum-model-registry/**`（只 `use` 它的 `BudgetView`，一个字不改、一个派生不加）、
`crates/continuum-core/**`、`crates/continuum-capability/**`、`crates/continuum-workspace/**`、
`crates/continuum-policy/**`、`crates/continuum-connector/**`、`crates/continuum-provider/**`、
`docs/02-工程.md`、`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`、`docs/superpowers/2026-10-06-p4-decisions.md`。

---

### Task 1: `continuum-canonical` 骨架、Surface Normalizer 与 Mention Detector

**Files:**
- Create: `crates/continuum-canonical/Cargo.toml`
- Create: `crates/continuum-canonical/src/{lib.rs,surface.rs,error.rs}`
- Create: `crates/continuum-canonical/tests/surface.rs`
- Modify: `Cargo.toml`（members）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 登记空条目）

**Interfaces:**
- Produces: `continuum_canonical::{Surface, normalize, Mention, MentionKind, detect, Opaque, CanonicalError}`

**`Opaque` 的落点是本 task**（一个包 `String` 的不透明值 newtype，只有 `as_str` / `from_str`）。
它是设计里反复出现的占位类型名（`completion_predicate`、`Priority`、`ResolutionRisk` 的四个字段、
`TaskContract` 的七个字段、`PlanFootprint` 的四个字段……）——**本计划把它的定义收在一处**：
`continuum-canonical` 定义，`continuum-semantics` 经**既有的** `← continuum-canonical` 边取它，
**两个 crate 都不为此新增任何边**（`continuum-budget` 不用它）。

- [ ] **Step 1: 建 crate 骨架并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`thiserror`（`CanonicalError` 上派生 `Error`，去掉即 `E0433`）。
`continuum-persist` / `serde_json` / `trybuild` / `tempfile` **由需要它们的 task 增量加**。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加（**只列本 task 实际有的边**：
本 task 对内部 crate 的引用为零，故是空条目；它不能省——`:252-257` 要求每个 workspace 成员都在表里）：

```rust
    // P4 语义层（设计 §2.1）：Input Canonicalization Frontend 与 Reference Resolver。
    // 本层不登记任何指向执行层／资源层／边界层的边；允许集只含 continuum-persist
    // （alias 表经 Tx）与层内前驱。`continuum-semantics ← continuum-canonical` 是
    // 反向的读取方，不在这里。
    ("continuum-canonical", &[]),
```

- [ ] **Step 2: 写用例**

`tests/surface.rs`（§270 §196 §197 的 Surface Normalizer 与 Mention Detector）：

- `normalize_folds_unicode_width_whitespace_and_case`：**四类各一条**——Unicode 归一（`"ﬁ"` → `"fi"` 一类）、
  宽度归一（全角 `Ａ` → 半角 `a`）、空白折叠（连续空白与全角空格折叠成一个半角空格）、大小写折叠。
  红条件：只做 `to_lowercase` 而不做宽度归一（**收紧档**：范围少了一半，宽度那条红）。
- `normalize_does_not_substitute_synonyms_or_correct_typos`（**否定式照片，本仓接受**，§270：纠错是候选生成的活）：
  输入一个错字串，断言 `normalize` 的输出**逐字等于**该串经宽度/空白/大小写折叠后的结果——
  即**没有任何映射表参与**。红条件：给 `normalize` 加一张 `"claude" -> "Claude Code"` 的映射（**移除档**：
  删掉这条用例之后，类型里没有任何东西阻止纠错被写进 `normalize`）。
  **这条是本 task 唯一的绝对措辞守卫**（`surface.rs` 的文档注释会写「只做字面处理，不做同义词、不做纠错」）。
- `detect_spans_are_byte_offsets_into_the_surface`：切出的每个 `Mention` 的 `span` 在**同一个 `Surface`** 上取回的子串
  等于 `mention.text`（**逐 mention**，不抽代表）。红条件：`span` 用**字符数**而非字节偏移（**取反档**：
  含多字节字符的输入即红——故夹具**必须含多字节**，否则这条与索引序一样是等价的）。
- `an_input_without_any_mention_yields_an_empty_vec`：fail-closed 侧一条（不 panic、不造一个假 mention）。
- `mention_kind_is_an_opaque_string`：`MentionKind` 是**不透明串**——构造一个表外取值（如 `"pronoun_x"`），
  断言它经 `as_str` / `from_str` **往返**。**并写明这条的限度**：把 `MentionKind` 改成封闭枚举会让本用例**编译不过**
  ——而**编译不过不算变红**（纪律 1(c)），故本条的守卫是**运行期往返**，它钉的是「取值域不由本层封闭」（§16 第 3 条），
  **不是**「枚举加了臂会红」。**这一句要写进用例注释。**

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test surface
```

- [ ] **Step 4: 实现**

```rust
/// §270 的 Surface Normalizer 的产物。规范化只做字面处理。
pub struct Surface(String);   // 内部串私有，构造入口只有 normalize

/// §270：Unicode 归一 → 宽度归一 → 空白折叠 → 大小写折叠，**四步之外不做任何事**。
/// **不做同义词替换、不做错别字纠正**——那是候选生成（§197）的活。
pub fn normalize(raw: &str) -> Surface;

/// §270 的 Mention Detector 的产物。`span` 是**字节偏移**（在 `Surface` 的 UTF-8 串上）。
pub struct Mention { pub span: Range<usize>, pub text: String, pub kind: MentionKind }

/// §196/§197 **未给** `kind` 的取值域（只给了「代词／上下文引用」两个例），
/// 故取不透明串、**不做封闭枚举**（设计 §16 第 3 条）。
pub struct MentionKind(String);

pub fn detect(surface: &Surface) -> Vec<Mention>;

/// **不透明值**：一个包 `String` 的 newtype，只有 `as_str` / `from_str`（无判定、无解释、无排序）。
/// 设计里凡写 `Opaque` 的地方都取它——本计划把它的定义**收在这一处**（见 Interfaces 的说明）。
pub struct Opaque(String);
```

**为什么 `MentionKind` 不封闭**（写进类型的文档注释）：封闭一条本层没有判据的取值域，
会让「系统不认识的一个 mention 种类」在**编译期**就被拒——那是发明判据，而不是保存事实。

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(canonical): crate 骨架、Surface Normalizer 与 Mention Detector"
```

---

### Task 2: 规范化产物（两支互斥）与两个解析器

**Files:**
- Create: `crates/continuum-canonical/src/{canonical.rs,temporal.rs,ambiguity.rs}`
- Modify: `crates/continuum-canonical/src/lib.rs`（导出）
- Create: `crates/continuum-canonical/tests/canonical.rs`

**Interfaces:**
- Consumes: Task 1 的 `Surface` / `Mention` / `detect`
- Produces: `continuum_canonical::{Canonicalized, CanonicalRepresentation, UnresolvedMention, Now, TemporalBinding, resolve_temporal, finish}`

- [ ] **Step 1: 写用例**

- `every_mention_resolved_gives_the_canonical_arm`：夹具里每个 mention 都是 `Resolved`，断言结果**是** `Canonicalized::Canonical`。
- `one_unresolved_mention_gives_the_needs_resolution_arm`：**逐个未解态各一条**（`ConfirmRequired` / `Ambiguous` / `Unknown`），
  断言结果是 `NeedsResolution`，**且列表里恰好是那一个 mention**（不是「列表非空」——列表里混进已解的 mention 会让
  「挂起时只说清哪些没解」这件事失真）。红条件：把未解的 mention 也塞进 `Canonical` 的表示（**放宽档**）。
- `the_two_arms_are_complements`：§3.2 的「一支是另一支的补集，不是两个可以同时为真也可以同时为假的标志位」。
  **照片机制**：`Canonicalized` **没有第三支**（穷尽 `match` 不带 `..`，加第三支即编译不过），
  且产生它的**唯一入口**是 `finish`（`tests/type_level.rs` 的 trybuild 反例：crate 外无法构造 `CanonicalRepresentation`，
  故造不出「`Canonical` 里带着未解 mention」这种值）。红条件：给 `Canonicalized` 加一个 `bool` 标志位字段（**移除档**：
  两支互斥由类型保证，加标志位即让两者可同真）。
- `four_kinds_of_dirty_input_each_yield_one_of_the_two_arms`：**§2.4 第 1 条**的落点——含**错别字／简称／中英混写／代词**
  的输入**各一条**，逐条断言结果落在哪一支（不 panic、不产出空 mention 的 `Canonical`）。
  红条件：把两类输入吞成同一个 `Ok` 而不区分（**放宽档**）。**并写明**：真实候选来源今天零个（§16 第 5 条），
  故这四条钉的是**分支机制**，不是规范化的真实质量（设计 §15.2 第 3 条）。
- `temporal_resolution_takes_now_as_an_argument`：同一 mention 配两个相隔很远的 `Now` 得到不同 `TemporalBinding`。
  红条件：函数体内部取系统时钟（**取反档**）——那时两个不同 `Now` 给出**相同**结果，本用例红。
  **`Now` 是入参**（本层不取系统时钟，与 P2 的入口同一条纪律）；Task 21 另加一条**源码面**复核（`SystemTime::now` 零命中）
  作为这条的第二侧。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test canonical
```

- [ ] **Step 3: 实现**

```rust
/// §2.4 第 1 条的产物：一个和类型，两支互斥。
pub enum Canonicalized {
    Canonical(CanonicalRepresentation),
    NeedsResolution(Vec<UnresolvedMention>),
}

/// §3.1 的 Ambiguity Manager：聚合同一次输入里所有 mention 的 ResolutionResult。
/// **`Canonical` 这一支要求每个 mention 都是 `Resolved`**（Task 5 的闭集判定）。
pub fn finish(parts: Vec<(Mention, ResolutionResult)>) -> Canonicalized;

/// §3.1 的 Temporal Resolver。**「Now」是入参**，本层不取系统时钟。
pub fn resolve_temporal(m: &Mention, now: &Now) -> TemporalBinding;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical
git commit -m "feat(canonical): Canonicalized 两支互斥、Temporal Resolver 与 Ambiguity Manager"
```

---

### Task 3: 候选面——`EntityId` 的不透明性与 `CandidateSource` 的「只能返回集合」

**Files:**
- Create: `crates/continuum-canonical/src/candidate.rs`
- Create: `crates/continuum-canonical/tests/candidate.rs`
- Create: `crates/continuum-canonical/tests/type_level.rs`、`tests/compile_fail/*.rs`
- Modify: `crates/continuum-canonical/{src/lib.rs,Cargo.toml}`（导出、dev-dep `trybuild`）

**Interfaces:**
- Produces: `continuum_canonical::{EntityId, Candidate, CandidateSource, CandidateSources}`
  （**`CandidateSet` 不在本 task**：它要 `CandidateFacts`（§4.1.1 的判定事实），落在 Task 4；
  本 task 的 `CandidateSource::retrieve` 按 §4.3 返回 `Vec<Candidate>`）

- [ ] **Step 1: 写用例**

- `entity_id_round_trips_and_has_no_public_literal_construction`（**两侧**）：
  （a）`EntityId` 经 `as_str` / `from_str` 往返（多条，含空串的处置断言**具体**是哪一种）；
  （b）trybuild 编译失败样例：**在 crate 外**用元组构造写 `EntityId("x".to_owned())`（`.stderr` 实跑取值，
  不许凭记忆写）＋ 正控制 `EntityId::from_str("x")` 编译通过。
  **红条件**：把 `EntityId` 的第 0 个字段改成 `pub`（**移除档**——那时 (b) 的样例从「编译失败」变成「编译通过」，
  观测方式是「样例不再产出旧的 `.stderr`」，**这与「用例变红」不是一回事**，报告里要分清）。
  **「没有指向 `ToolId` / `ModelId` / `ConnectorId` 的转换」那一半不靠 trybuild 钉**——
  本 crate 的 `ALLOWED` 条目里没有 `continuum-core`，**那三个类型在本 crate 里根本写不出来**
  （写了即 `E0432` 未解析导入，**那是编译不过、不算变红**）；故这一半的判据是
  **`ALLOWED` 条目 ＋ Task 21 的 grep**（依赖边为零），不是一条 trybuild 样例。**这一句要写进用例注释。**
  **为什么 `EntityId` 是不透明 newtype**（写进类型文档）：引用资源层／边界层的三个 id 就要登记指向它们的边（§1.2.3 规则 3）；
  代价是「mention 解析出的实体」与「注册表里的 `ToolId`」的对应由装配方维持，对冲是 Task 5 的闭集不变量。
- `a_candidate_source_can_only_return_a_set`：§197 的分工（候选生成负责高召回，**不直接决定最终语义**）。
  **照片机制（写死）**：**trybuild 正例**——写一个只实现 `retrieve` 的 `CandidateSource` 实现，编译通过；
  **trybuild 反例**——调用一个不存在的方法（如 `source.pick_one(&m)`，返回单实体那个）编译不过（`.stderr` 实跑）。
  **并写明这条守卫的限度**：它钉的是「本层不为单实体定义方法」，**不钉**「实现者内部返回唯一候选」——后者由 Task 5 的四条断言钉。
- `the_candidate_records_exactly_the_three_fields_of_272`：字段清单逐项，机制 = `let Candidate { entity_id, evidence, retrieval_score } = c;`
  **不带 `..`**（任一侧加字段即**编译不过**）。字段名照 §272：`entity_id` / `evidence[]` / `retrieval_score?`。
  **明写**：这是**编译期**照片，运行期无照片（纯数据声明）。
- `a_score_is_optional_and_absent_is_not_zero`：`retrieval_score: Option<f64>`——**缺席与 `Some(0.0)` 可区分**，
  两条（`None` 一条、`Some(0.0)` 一条，断言读回各自是哪一枚）。红条件：把 `Option` 换成默认 `0.0`（**移除档**）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test candidate
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test type_level
```

- [ ] **Step 3: 实现**

```rust
/// §272 的三个字段照录。**不透明 newtype**，不是 ToolId / ModelId / ConnectorId 中的任何一个（§1.2.3 规则 3）。
pub struct EntityId(String);

/// §272 的候选。`retrieval_score` 只在**候选**上出现（§272：MAY 用于排序，MUST NOT 当成真实概率）——
/// 它不出现在已解出的结果上，见 Task 4 的 `ResolutionResult`。
pub struct Candidate { pub entity_id: EntityId, pub evidence: Vec<String>, pub retrieval_score: Option<f64> }

/// §201：候选集以**值**进入本层。§197 的七种手段里只有 alias registry 在本层（Task 7）。
/// **本层不为它们定义默认实现，也不内置任何一个**（§16 第 5 条：真实来源今天零个）。
pub trait CandidateSource {
    /// §197 的返回值只能是 `Vec<Candidate>`——**没有任何返回单一实体的方法**。
    fn retrieve(&self, m: &Mention) -> Vec<Candidate>;
}

/// 由装配方给出的若干来源。
pub struct CandidateSources { /* Vec<Box<dyn CandidateSource>> */ }
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical
git commit -m "feat(canonical): 候选面——EntityId 与 CandidateSource 的方法集"
```

---

### Task 4: 显式约束与「满足」判据（§4.1.1）＋ Reference Resolver 的四态（§4.1）与两个数不同轴（§4.4）

**Files:**
- Create: `crates/continuum-canonical/src/explicit.rs`
- Create: `crates/continuum-canonical/src/resolver.rs`
- Create: `crates/continuum-canonical/tests/explicit.rs`
- Create: `crates/continuum-canonical/tests/resolver.rs`
- Create: `crates/continuum-canonical/tests/compile_fail/*.rs`（追加）
- Modify: `crates/continuum-canonical/src/lib.rs`

**Interfaces:**
- Consumes: Task 3 的 `EntityId` / `Candidate`
- Produces: `continuum_canonical::{ExplicitConstraint, ResolutionScope, EntityKind, CandidateFacts, CandidateSet, satisfies, qualifying,
  alias_scope_to_resolution_scope, resolution_scope_to_alias_scope, ResolutionResult, Resolved, ConfirmRequired, Ambiguous, Unknown,
  UnknownReason, ResolutionConfidence, RequiredStrength}`、`resolver::resolve`

- [ ] **Step 1: 写用例——约束面（§4.1.1，**本设计自定，`## 遗留` 照抄它的来历**）**

- `the_three_explicit_constraints_each_have_a_positive_and_a_negative_case`（**逐枚 3×2＝6 条**）：
  `Scope` / `EntityKind` / `BoundTo` 各一条满足 + 各一条不满足。
  红条件：`satisfies` 里把某一枚写成恒真（**移除档**，那一枚的反面红）。
- `the_constraint_set_is_a_conjunction`：`K` 有两枚、候选只满足其一 ⟹ **不合格**（设计 §4.1.1 写死「判据是合取」）。
  红条件：改成析取（**取反档**，本用例红）。**这一条是断言 B 与 D 的定义基础**（「全部违反」＝合取下的空集）。
- `scope_is_equality_not_hierarchy`（**设计点名的反面**）：一个 `project` 层可见的候选在 `Scope(explicit)` 下**不合格**。
  红条件：把 `==` 写成「更广的 scope 包含更窄的」（**移除档**）——本用例钉的是「层级不会被顺手实现进去」。
  **并写明**：§274 的九个值是一条**优先级层级**，而**「哪一枚比哪一枚更广」的序规范没给**
  （§274 只给九个名字与一句「显式 scope 优先级最高」）；本层不发明那个序。
  **那句「优先级最高」是一条决胜规则**（多个 binding 落在不同 scope 时取哪一个），**不是准入规则**——
  把它当准入规则会把「`project` 层可见的候选」在 `explicit` 约束下判成不合格，**而规范没这么说**。
- `entity_kind_has_exactly_four_variants`（**四枚各一条**）：`Tool` / `Skill` / `Relation` / `ConnectorService`；
  机制 = 穷尽 `match` **不带 `..`**（加第五枚即**编译不过**）。
- `a_connector_service_mention_resolves_to_the_connector_service_kind`（**正面一条**）：
  `GitHub` 这类 mention 解出的候选，其 `CandidateFacts.kind` **恰是 `ConnectorService`**。
- `a_connector_service_is_neither_a_tool_nor_a_skill`（**反面逐枚**）：
  断言它**不是** `Tool`、**不是** `Skill`（逐枚两条，不是「不等于某个别的值」）。
  **这三条依据写进注释**：`EntityKind` 是**四枚、不是 §198 的三枚**——§16 第 26 条本层认领了
  「服务名作为 mention 的解析目标」并写明「与其它实体**同形**处理」，而 `GitHub` / `Email`
  **不是 `Tool` / `Skill` / `Relation` 中任何一个**；只有三枚时调用方只能**硬塞**，
  于是**静默错型、且没有任何断言会红**。**第四枚的出处**是 §124 的六项服务与 §125 的「按操作细分」。
- `the_alias_scope_mapping_has_exactly_the_four_alias_scopes_as_its_domain`：
  `alias_scope_to_resolution_scope` 的**定义域恰是 §278 的四枚**（`TURN`→`conversation`、`SESSION`→`session`、
  `PROJECT`→`project`、`USER_PERSISTENT`→`user`），四枚**各一条**；定义域是封闭四值，
  **没有第五枚可以喂进去**（由 `AliasScope` 的封闭性保证）。
  反向 `resolution_scope_to_alias_scope` 在**没有 alias 对应的五枚**（`workspace` / `tool` / `plugin` /
  `global` / `explicit`）上各一条 `None`/`Err`——**五枚逐枚，不抽代表**。
  **并写明这张表是本设计的决定**（规范没有给出九值与四值的关系），以及**它的用途**：
  一条以 alias scope 表述的约束**必须先经上表换成 §274 的值**，换不出来的**不能用 alias 表述**。
- `candidate_facts_are_given_by_the_caller`：`CandidateFacts { scope, kind }` 由调用方**与候选一并给出**——
  **本层查不到它们**（注册表不在本层）。签名层面：`qualifying` 的入参是 `&[(Candidate, CandidateFacts)]`，
  **不收 `&Tx`、不收 `dyn` 来源**。
- `the_272_candidate_shape_is_untouched`：`Candidate` 仍是 §272 的三字段（本 task **不往它里面加** `scope` / `kind`）
  ——字段清单逐项（穷尽解构，不带 `..`，Task 3 已钉）。**并写明**：往那里加字段会改掉一张照录的形状，
  故判定事实走**并列的 `CandidateFacts`**。

**（Step 1 续）四态与两个数（§4.1 §4.4）**

- `each_of_the_four_states_has_a_photo_and_the_state_is_asserted`：四态**各一条**，**每条断言是哪一枚**
  （不是 `is_ok`、不是 `!= Unknown`）：
  `Resolved` ← 唯一合格候选且门槛已过；`ConfirmRequired` ← 存在占优候选但**门槛未过**；
  `Ambiguous` ← 满足全部显式约束的候选多于一个；`Unknown` ← 零个。
  红条件：把 `ConfirmRequired` 与 `Ambiguous` 合并成一支（**移除档**——两支的判据不同源：
  一支是「占优但门槛未过」，一支是「多个合格」，合并会让门槛失掉唯一的可观察后果）。
- `unknown_reason_distinguishes_an_empty_set_from_all_rejected`（**两枚各一条，混成一枚即红**）：
  `UnknownReason::NoCandidate`（候选集**为空**，断言 C）与 `AllCandidatesRejected`
  （候选集**非空但全不合格**，断言 B）各一条。红条件：把两枚合成一枚（**移除档**）——
  那时「全不合格」与「本来就没有」不可分，而这两件事的处置方向相反（前者要扩大检索、后者要问用户）。
- `resolve_returns_the_result_type_with_no_err_arm`（§4.1 写死）：`resolve` 的返回类型**不是 `Result`**——
  §206 的 `abstain` 已把「没有可靠解析」收进 `Unknown`，而**本层没有第二种失败**
  （输入全是值、不做 I/O、不落库）。照片：四条用例都直接对 `ResolutionResult` 断言、
  **没有一条写 `unwrap()`**（若某天给它套上 `Result`，四条全红）。
- `every_arm_carries_exactly_its_declared_fields`：四支的**字段清单逐项**，机制 = 对四支
  **逐支穷尽解构、不带 `..`**，解构的是四个载荷类型自己的字段：
  `Resolved { entity, satisfied, candidate_count }` /
  `ConfirmRequired { first, surviving, candidate_count }` /
  `Ambiguous { surviving, candidate_count }` / `Unknown { reason, candidate_count }`。
  任一支加字段、少字段、改名即**编译不过**。**明写**：编译期照片，运行期无照片（纯数据声明）。
- `resolution_result_carries_no_score`（§4.4 第 1 条）：上一条的四份字段清单里**没有任何分数字段**
  （`retrieval_score` 只出现在 `Candidate` 上）。红条件：给 `Resolved` 加 `confidence` 或 `retrieval_score`
  （**移除档**——上一条的穷尽解构先红）。**`candidate_count` 不是分数**：它是**规模**（`usize`），
  断言 A/D 拿它核「集合里有几个」；**这一句要写进注释**，免得它与「已解出的结果携带不了分数」读起来相抵。
- `resolution_confidence_has_no_reader_that_yields_a_number`：§4.4 第 2 条——`ResolutionConfidence`
  **只有** `meets(&RequiredStrength) -> bool`，**没有** `as_probability` / `to_f64` / `From<…> for f64`。
  照片机制：trybuild **反例**（`c.to_f64()` 编译不过，`.stderr` 实跑）＋ **正控制**（`c.meets(&req)` 编译通过）。
  红条件：加 `impl From<ResolutionConfidence> for f64`（**取反档**）。
- `resolution_confidence_never_appears_in_a_candidate_or_a_resolved`（**两侧**）：
  `Candidate`（Task 3 的穷尽解构）与 `Resolved`（本 task 的穷尽解构）的字段清单里都不含它。
  红条件：把 confidence 塞进 `Candidate`（**取反档**）——那时 Task 3 的清单断言先红。
- `the_two_numbers_are_not_on_the_same_axis`：§4.4 第 3 条。照片机制：**公开面清单逐条**——
  `ResolutionConfidence` 的构造函数**不接受** `retrieval_score`（没有 `from_score` 一类），且没有 `From<f64>`；
  trybuild 反例 `ResolutionConfidence::from_score(0.9)` 编译不过。
  **并写明**：「`ResolutionConfidence` 的取值只能由门槛判定产出」这条**在类型上只到「不能被分数构造」**；
  「它必须经过校准」（§275）本层**没有判据**、也没有照片——那需要一个校准过程，本层没有。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test explicit
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test resolver
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test type_level
```

- [ ] **Step 3: 实现**

```rust
/// 一条显式约束。**取值域是闭集，三枚**——每一枚都能追到规范里已有的一个概念，
/// **不引入开放谓词语言**（那会变成一套本层发明的语法）。
pub enum ExplicitConstraint {
    Scope(ResolutionScope),      // §274 的九值（**只取九值**，「显式 scope 优先级最高」不归这一枚）
    EntityKind(EntityKind),      // 见下
    BoundTo(EntityId),           // §277 的 ExplicitBinding
}

/// 候选的实体种类。**闭集四枚**——**不是 §198 的三枚**（第四枚 `ConnectorService` 出处 §124／§125）。
pub enum EntityKind { Tool, Skill, Relation, ConnectorService }

/// §274 的九个作用域（`explicit` / `session` / `conversation` / `project` / `workspace` /
/// `tool` / `plugin` / `user` / `global`）——九值封闭。
pub enum ResolutionScope { /* 九枚 */ }

/// 候选的**判定事实**，由调用方与候选一并给出。**本层查不到它们**（注册表不在本层）。
/// **§272 的 `Candidate` 三字段不动**——判定事实走这个并列类型。
pub struct CandidateFacts { pub scope: ResolutionScope, pub kind: EntityKind }

/// 判定：**合取**。`Scope` 是**等值过滤**（`f.scope == *s`），**不做层级**（层级的序规范没给）。
pub fn satisfies(c: &Candidate, f: &CandidateFacts, k: &ExplicitConstraint) -> bool;

/// 「合格」＝候选集里满足全部约束的那些。
pub fn qualifying(cs: &[(Candidate, CandidateFacts)], k: &[ExplicitConstraint]) -> Vec<Candidate>;

/// §4.1 的四态。**四支都是返回值，不是错误**；载荷类型各自私有字段、无公开构造函数。
pub enum ResolutionResult { Resolved(Resolved), ConfirmRequired(ConfirmRequired), Ambiguous(Ambiguous), Unknown(Unknown) }
pub struct Resolved { /* entity / satisfied / candidate_count */ }
pub struct ConfirmRequired { /* first / surviving / candidate_count */ }
pub struct Ambiguous { /* surviving / candidate_count */ }   // surviving：满足全部约束的候选**全体**，至少两个
pub struct Unknown { /* reason / candidate_count */ }
/// **两枚都必须是可区分的**：断言 B 与断言 C 各钉一枚。
pub enum UnknownReason { NoCandidate, AllCandidatesRejected }

/// §275 的「是否允许自动执行」用的强度。**没有 Reader 能把它变成一个数**。
pub struct ResolutionConfidence(/* 私有 */);
impl ResolutionConfidence { pub fn meets(&self, required: &RequiredStrength) -> bool; }

/// §276 的门槛。**无 `Default`**、`resolve` 不收 `Option<RequiredStrength>`（Task 6 钉签名）。
pub struct RequiredStrength(/* 私有 */);
```

**`Resolved.satisfied` 与 `surviving` 的分工要写进注释**：`satisfied` 是**照录判定时用的那一组约束**
（不是重新推的），`surviving` 是**判定输出**（合格的那一部分）；`candidate_count` 是**判定输入的规模**——
三者分开是为了「不把判定输入混进判定输出」，同时让断言 A/D 不必自己再构造一次候选集
（自造再自核正是本仓「表驱动用例覆盖错路径」的同一形状）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical
git commit -m "feat(canonical): 显式约束与满足判据、四态与两个数的分轴"
```

---

### Task 5: §4.2 的四条断言——「不存在正确候选时不强行绑定」的可断言形态

**Files:**
- Create: `crates/continuum-canonical/src/resolver.rs`（续，四条断言的实现体）
- Create: `crates/continuum-canonical/tests/resolver.rs`（续）
- Modify: `crates/continuum-canonical/src/lib.rs`

**Interfaces:**
- Consumes: Task 4 的 `ExplicitConstraint` / `CandidateFacts` / `ResolutionResult` / `RequiredStrength`、Task 3 的 `Candidate`
- Produces: 无新公开类型（`resolve` 的判定路径补全）

- [ ] **Step 1: 写用例**（四条断言，三条在 fail-open 侧，故三条都必须有照片）

**每条夹具都含至少一条非空显式约束 `K`**（§4.1.1 的 `ExplicitConstraint` 闭集保证它可构造）。
约束集为空时每个候选都平凡地合格，用例会退化成「两个候选都在集合里」——**恒真**的用例。

- `assertion_a_a_resolved_entity_is_a_member_of_the_retrieved_set_or_the_injected_one`（**fail-open 侧**）：
  A 的现行判据是 **`e ∈ 检索集 ∪ {由 `ExplicitBinding` 注入的那一枚}`**（§4.2 重述后的 A）——
  **不是「工作集」**。夹具 `K` 非空，候选 `{c₁ 满足 K, c₂ 违反 K}` → 断言 `Resolved(c₁)`，
  且断言的 id 与检索集里那一个是**同一个 `EntityId` 值**（经 `as_str` 比），
  并用 `candidate_count` 核「判定用的规模与夹具给的检索集一致」。
  红条件：解析器自造一个实体、或把检索分数最高的候选**名字拼出来**（**取反档**）。
- `assertion_a_injection_guard`（**§4.2 点名的配套守卫，没有它 A 只是同义反复**）：
  构造一个「解析器自造了一个**既不在检索集、也没有 `ExplicitBinding` 背书**的实体」的情形 ⇒ **A 变红**
  （该实体不在「检索集 ∪ 注入的那一枚」里；观测形式：`Resolved.entity` 不在两个来源的并里，
  或 `candidate_count` 大于「检索集 ＋ 至多一次注入」的应有大小）。
  **红条件（这条用例本身的红）**：把 A 实现成「`e` ∈ 工作集」（**放宽档**）——那时注入之后**恒真**，
  自造实体也不再触发，本用例红。
  **来历写进注释**：A 的初稿写「`e` 是**输入候选集**里的成员」，而「输入候选集」两读都通——
  读成「检索器交出的那一集」则 §4.6 的注入**破了 A**；读成「解析器实际工作的那一集」则**注入后 A 恒真**、
  解析器可以自造任何实体而 A 永不响。故 A 拿的必须是**前者**。
- `assertion_b_each_candidate_violating_a_different_constraint_is_not_resolved`（**fail-open 侧**）：
  **逐候选 N 条**——构造 N 个候选，每个恰好违反一条**不同的**显式约束；N 次结果的**每一次**都断言
  **不是 `Resolved`**，且**逐次断言**是 `Unknown { reason: AllCandidatesRejected }`（该候选是唯一候选且不合格）
  还是 `ConfirmRequired`。红条件：把「全部候选都不合格」也判成 `Resolved`（**放宽档**）。
  **不抽代表**——每个候选各一条断言。
- `assertion_c_an_empty_candidate_set_is_unknown_with_no_candidate`（**fail-closed 侧，一条即可**）：
  断言 `Unknown { reason: NoCandidate }`。红条件：空集返回 `ConfirmRequired`，或把 reason 给成
  `AllCandidatesRejected`（**取反档**——两枚混用正是 `UnknownReason` 分两枚要防的）。
- `assertion_d_1_the_qualified_candidate_is_selected_even_when_an_unqualified_one_is_present`（**fail-open 侧**）：
  夹具 `{c₁ 满足 K, c₂ 违反 K}` → `Resolved(c₁)`。**钉住 A 不足以替代 D**（`c₂` 也在检索集里，
  若绑定到它，A 仍成立）。红条件：把「合格」判成「在集合里」（**移除档**：删掉约束求值，
  此时会绑定到检索分数更高的 `c₂`）。
- `assertion_d_2_multiple_qualified_candidates_give_ambiguous`（**fail-open 侧**）：
  夹具**同一条非空约束 `K`**、候选 `{c₁ 满足 K, c₂ 满足 K, c₃ 违反 K}`；断言**两条**：
  （1）结果是 `Ambiguous` 而**不是** `Resolved(c₁)`；
  （2）**`Ambiguous.surviving` 恰是 `{c₁, c₂}`（逐项：长度为 2、两枚都在、`c₃` **不在**，按 id 升序）**。
  **第（2）条是要紧的那一条**（本计划查出后由设计写死）：只断言「结果是 `Ambiguous`」时
  **`c₃` 不承重**——把「合格」判成**恒真**的实现会让 `{c₁,c₂,c₃}` 全合格，结果仍是 `Ambiguous`，用例照样绿。
  `c₃` 是对照组，靠第（2）条兑现；这也是 `Ambiguous` 必须带 `surviving` 的原因。
  红条件：合格判成恒真（**移除档**，`surviving` 里多出 `c₃`）；或把 `surviving` 的顺序交给 `HashSet` 迭代
  （**取反档**，逐项相等那条红——**这就是为什么断言逐项相等而不是集合相等**：
  集合相等会让「改成不排序」成为**等价变异体**，纪律 1(b) 已点名）。
- `the_other_side_a_nonempty_set_with_exactly_one_qualified_candidate_is_resolved`（**守卫的另一侧**）：
  候选集非空且**唯一**合格 → `Resolved`。**不能省**——只钉「拒绑」那侧会让一个**永远返回 `Unknown`** 的解析器
  照样全绿（设计 §14 的同名行）。
- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test resolver
```

- [ ] **Step 3: 实现**

判定顺序（**写死，且每一格的依据在设计里**）：**检索集** → `ExplicitBinding` 注入（**唯一允许的注入点**，
Task 6）→ 按显式约束**合取**过滤（Task 4 的 `qualifying`）→ 门槛判定（Task 6）→ 四态。

**工作集的定义要写进 `resolve` 的注释**：`工作集 = 检索集 ＋ binding 注入`（至多一次），
而**断言 A 拿的是「检索集 ∪ 注入的那一枚」，不是工作集**——两者的差别正是
`assertion_a_injection_guard` 钉的东西。**四条断言的判据逐条写进文档注释**，每条附一个用例名，
**注释里的「不存在正确候选时不强行绑定」这句绝对措辞因此有照片**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical
git commit -m "feat(canonical): §4.2 的四条断言（A/B/C/D）与两侧守卫"
```

---

### Task 6: 风险门槛（§276）、`ExplicitBinding` 的最高优先序（§277）与集合外注入（§4.6）

**Files:**
- Create: `crates/continuum-canonical/src/{risk.rs,binding.rs}`
- Create: `crates/continuum-canonical/tests/risk.rs`
- Modify: `crates/continuum-canonical/src/lib.rs`

**Interfaces:**
- Consumes: Task 4 的 `RequiredStrength` / `ResolutionResult` / `CandidateSet` / `ExplicitConstraint`、Task 3 的 `Candidate`
- Produces: `continuum_canonical::{ResolutionRisk, ExplicitBinding, BindingSource}`、`binding::apply`（注入点）

- [ ] **Step 1: 写用例**

- `required_strength_is_a_required_argument_with_no_default`（§4.5）：**签名层面 + 编译期**——
  `resolve` 不收 `Option<RequiredStrength>`；trybuild 反例：`RequiredStrength::default()` 编译不过、
  少传该参数编译不过。**并明写为什么没有运行期照片**：`f(effect, cost, reversibility, privacy)` 的形态规范没给，
  四维之间的**序与量纲都没有**，故「风险越高要求越严」这条单调性在本层**没有任何函数**能拍出来——
  **这不是「没有照片」，是「没有判据」**（设计 §15.2 第 2 条）。
- `the_layer_provides_no_baseline_mapping_from_risk_to_strength`：公开面清单逐条——
  `src/` 里没有 `impl From<ResolutionRisk> for RequiredStrength`、没有 `fn from_risk` 一类。
  trybuild 反例 `RequiredStrength::from_risk(&risk)` 编译不过。红条件：加一个「保守的」基线（**移除档**——
  本仓口径：无中生有的序会把「高成本」判成「低风险」，故基线比没有更坏）。
- `the_risk_facts_are_four_and_their_field_names_are_recorded_verbatim`：`ResolutionRisk` 的字段清单逐项
  （穷尽解构，不带 `..`），四个字段名逐字照录 §276：`effect` / `cost` / `reversibility` / `privacy`，
  四个都是不透明值（§16 第 6 条：取值域规范未给）。
- `an_explicit_binding_shrinks_the_candidate_set_instead_of_weighting_it`（§277）：给一个与 `ExplicitBinding`
  冲突的**高分**候选（`retrieval_score` 更高），断言结果仍是 `Resolved(binding.entity)`。
  红条件：把 `ExplicitBinding` 实现成「给 `entity` 加权后再排序」（**取反档**——那时高分候选胜出）。
- `a_binding_whose_entity_is_not_in_the_retrieved_set_is_injected_and_resolved`（**§4.6 照片 (a)**）：
  `entity` **不在检索给的候选集里** → 不新增分支，把它**作为一条候选注入工作集**（`evidence` 标 `user_explicit`），
  再照常过滤；断言结果 `Resolved(那个 entity)` **且 `Resolved.candidate_count` 比检索集多 1**
  （证明它确实被加入了）。红条件：把「不在检索集」判成 `Unknown`（**取反档**）——
  那会让检索结果**否决用户**，而 §277 的「SHOULD 在当前 scope 拥有最高优先级」正是要防这一格。
- `a_binding_does_not_exempt_the_entity_from_the_explicit_constraints`（**§4.6 照片 (b)，另一侧**）：
  同一个 `entity` 不在检索集里**且**违反一条 `Scope` 约束 → 断言 `Unknown { reason: AllCandidatesRejected }`
  （**binding 不是「绕过约束」的口子**：它只提高**检索**这一侧，不豁免 `ExplicitConstraint`）。
  红条件：让注入的候选跳过约束过滤（**放宽档**）。
  **这一对的两侧都必须有**：只钉 (a) 会让「注入即放行」的实现全绿；只钉 (b) 会让注入路径整个不存在也全绿。
  **并把「这是一次扩张」写进注释**：§277 的字面前提是「用户**选择候选**」（候选已由检索给出），
  而这里处理的是「用户**直接点名**一个检索没给出的实体」——**§277 只覆盖前一半**，
  后一半是本设计补的（收件人：规范维护者，`## 遗留` 照抄）。
- `an_explicit_binding_is_recorded_with_its_scope_and_source`：`ExplicitBinding { mention, entity, scope, source }`
  四个字段逐项（穷尽解构），`source = USER`（§277 的结构照录）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test risk
```

- [ ] **Step 3: 实现**

```rust
/// §276 的四个风险事实。字段名逐字照录，**取值域规范未给**（§16 第 6 条）——
/// 故四个都是不透明值，本层不解释、不比较、不排序。
pub struct ResolutionRisk { pub effect: Opaque, pub cost: Opaque, pub reversibility: Opaque, pub privacy: Opaque }

/// §277：`ExplicitBinding { mention, entity, scope, source = USER }`。
/// **「最高优先级」落在约束解析的取值顺序上**：先把候选集缩到 `entity` 这一个，**不是给它加权**。
///
/// **`entity` 不在检索集里时的处置**（§4.6）：**不新增分支**，把它作为一条候选**注入工作集**
/// （`evidence` 标 `user_explicit`），再照常过滤。**这是 `resolve` 里唯一允许的注入点**——
/// 断言 A 的注入守卫（Task 5）钉的就是「除它之外没有第二个来源」。
/// **这是一次扩张、不是 §277 的原文**：§277 的前提是「用户选择候选」，而这里是「用户直接点名」。
pub struct ExplicitBinding { /* … */ }
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical
git commit -m "feat(canonical): 风险门槛必填与 ExplicitBinding 的取值顺序"
```

---

### Task 7: Alias 存储（迁移 100）、四作用域与「不自动永久化」

**Files:**
- Create: `crates/continuum-canonical/src/{alias.rs,persist.rs}`
- Create: `crates/continuum-canonical/tests/alias.rs`
- Modify: `crates/continuum-canonical/{src/lib.rs,src/error.rs,Cargo.toml}`（导出、`AliasError`、dep `continuum-persist`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-persist`）

**Interfaces:**
- Produces: `continuum_canonical::{AliasScope, Alias, AliasError, p4_canonical_migrations, save_alias, load_alias, list_aliases, promote_alias}`

- [ ] **Step 1: 现场核对迁移号 100 为空号，并登记依赖边**

```bash
cd /home/DslsDZC/Continuum && grep -rn "Migration::new(" crates/*/src/persist.rs | grep -v "^.*://"
```

**预期**：命中的 version 是 10 / 20 / 30 / 40 / 41 / 50 / 80（实读 2026-10-06）。
**干扰项逐条核过**：`60`（`crates/continuum-persist/src/bin/crash-writer.rs:17`、`tests/crash_atomicity.rs:9`）
与 `100`（`crates/continuum-persist/tests/migrations.rs:45`，表名 `layer_table`）**都在测试／工具的 `Db` 里**，
不进生产链，**不占用**号段。若现场发现 100 已被生产链占用 → **停下报协调者，改用下一个空号**（不擅自换）。

`ALLOWED` 的 `continuum-canonical` 条目改为 `&["continuum-persist"]`。

- [ ] **Step 2: 写用例**

- `every_alias_scope_round_trips`：`TURN / SESSION / PROJECT / USER_PERSISTENT` **四值各一条**写入并读回，
  断言是**哪一枚**。红条件：`as_str` 少一档（**放宽档**）或多词不接 `_`（**取反档**：编码纪律）。
- `an_out_of_table_scope_reads_back_as_a_specific_err`：裸 `INSERT` 一个表外取值（如 `"global"`），
  读回得具体 `Err(AliasError::UnknownScope { value })`，断言 `value` 是**那一个**。
  红条件：`parse` 返回一个默认态（**放宽档**）。
- `a_session_alias_is_never_written_as_user_persistent`（§278 的正面禁令，**否定式照片**）：
  写一条 `SESSION` alias，断言 `alias` 表里 `scope = 'user_persistent'` 的**行数为 0**。
  红条件：`save_alias` 把 scope 硬编码成 `USER_PERSISTENT`（**移除档**）。
- `an_explicit_promotion_changes_the_scope`（**两侧的另一侧，不能省**）：
  `promote_alias(alias, USER_PERSISTENT)` 得 `Ok`，且读回 scope **变了**。
  **只钉「不提升」那侧会让提升路径整个不存在也照样绿**（设计 §4.6 第 2 条两端逐字点名）。
- `save_alias_writes_exactly_the_scope_it_was_given`：同一 alias 分别以 `SESSION` 与 `USER_PERSISTENT` 各写一次
  （两个库），断言两边的 `scope = 'user_persistent'` 行数分别是 **0** 与 **1**——
  这一对把「`save_alias` 不替调用方升级作用域」钉成可观察的事实（「没有任何函数会顺带调用 `promote_alias`」这句绝对措辞的照片）。
- `the_alias_table_has_exactly_the_expected_columns`：`PRAGMA table_info(alias)` 的**实际列**与手写期望清单**逐列**比对
  （多一列即红）。**与 Task 19 的那种清单不同，这一条不恒真**：一侧是运行期读出的实际列、一侧是手写的期望——
  **写明这一点**，免得被读成「手抄名单恒真」那类无照片的断言。
- `a_turn_alias_and_a_session_alias_both_persist_without_an_expiry_column`：§4.6 第 3 条——
  `TURN` / `SESSION` 的**过期时点规范未给**，故本层**只实现「不自动永久化」，不实现会话边界的清退**：
  断言 `alias` 表的列清单里**没有** `expires_at` 一类的列（**否定式照片**），且刚写的 `TURN` alias 立刻读得回来。
  **并写明**为什么没有：**没有定义「会话结束」是什么**（§16 第 7 条）。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-canonical --test alias
```

- [ ] **Step 4: 实现**

```rust
/// §278 的四个作用域，封闭四值。
pub enum AliasScope { Turn, Session, Project, UserPersistent }

/// **唯一的提升路径**（§278：MUST NOT 默认把 Session Alias 永久化）。
/// **没有任何别的函数会调用它**——`save_alias` 只写调用方给的 scope。
pub fn promote_alias(tx: &Tx<'_>, alias: &str, to: AliasScope) -> Result<(), AliasError>;

/// 写函数**不自己 `commit`**（同 P1 的 `apply_transition` 形状）。
pub fn save_alias(tx: &Tx<'_>, a: &Alias) -> Result<(), AliasError>;

/// 迁移 100：`alias` 表，`scope` 列是封闭枚举（ENG-009 由本层闭合，设计 §4.6）。
pub fn p4_canonical_migrations() -> Vec<Migration>;
```

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-canonical crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(canonical): alias 表（迁移 100）、四作用域与显式提升"
```

---

### Task 8: `continuum-semantics` 骨架、Intent 数据模型与版本语义

**Files:**
- Create: `crates/continuum-semantics/Cargo.toml`
- Create: `crates/continuum-semantics/src/{lib.rs,ids.rs,intent.rs,persist.rs,error.rs}`
- Create: `crates/continuum-semantics/tests/intent.rs`
- Modify: `Cargo.toml`（members）、`crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED`）

**Interfaces:**
- Consumes: `continuum_canonical::EntityId`（§1.3 的类型边）；`continuum_persist::{Tx, Migration, Db, Value}`
- Produces: `continuum_semantics::{IntentId, IntentKind, IntentStatus, Priority, ContractRef, GraphIdRef, ProductIdRef, Intent, IntentError, p4_semantics_migrations, save_intent, load_intent, list_children}`

- [ ] **Step 1: 建 crate 骨架、登记 members 与 `ALLOWED`**

`ALLOWED` 加（**本 task 实际有的两条边**）：

```rust
    // P4 语义层（设计 §2.1）：Intent / Specification / Contract / ContractDiff / Plan / Decision /
    // Constraint Validator。`← continuum-canonical` 是 §1.3 的类型边（Intent.goal 取 EntityId）；
    // `← continuum-persist` 由本 task 登记（intent 表）。
    // **允许集里没有 continuum-budget**——§11.5 的第 1 条守卫（两道门分离）。
    ("continuum-semantics", &["continuum-canonical", "continuum-persist"]),
```

- [ ] **Step 2: 写用例**

- `intent_carries_exactly_the_fifteen_fields_of_222`：字段清单**逐项**，机制 = `let Intent { … } = i;`
  **穷尽、不带 `..`**（任一侧加字段即**编译不过**）。**明写**：编译期照片，运行期无照片（纯数据声明）。
  十五个字段（十三行，`created_at`/`updated_at` 与 `parent_intent`/`child_intents` 各一行两字段）逐项列在实现体的文档注释里。
- `a_child_list_is_not_itself_stored`（§5.1 的「子列表不落列」）：两条——
  （a）`list_children(tx, parent)` 返回按 id 升序的两个子 Intent；
  （b）**否定式照片**：`PRAGMA table_info(intent)` 的实际列清单里**没有** `child_intents`。
  红条件：把子列表落成一个列（**移除档**——两份真值会漂移，本仓一贯判为缺陷的那一类）。
- `version_increments_only_on_a_semantic_change`（§5.2，**两侧都钉**）：
  （a）改 `goal` → `version` +1 **且** `updated_at` 变；
  （b）只做状态迁移 → `version` **不变**、`updated_at` 变。
  红条件：状态迁移也 +1（**放宽档**，(b) 红）；语义变化不 +1（**移除档**，(a) 红）。
- `version_increments_for_each_of_the_five_semantic_fields`：`kind` / `goal` / `parent_intent` /
  `completion_predicate` / `contract_id` **逐项各一条**（五条）+1 断言，**不抽代表**（§5.2 的五者之一改变时 +1）。
  红条件：只对 `goal` +1（**收紧档**，其余四条红）。
- `history_does_not_accumulate_in_the_intent_table`：改两次语义后 `intent` 表**行数仍是 1**（一行一 Intent），
  改动的权威在 `events` 表（§5.2）。**并写明**：这与 Contract 的处置**刻意不同**（§7.3：§226 明写 MUST NOT 原地修改），
  两者的差别来自规范原文的差别。
- `contract_id_carries_a_version`：`Intent.contract_id` 是 `ContractRef { id, version }`——
  字段清单逐项（穷尽解构）。**§222 只给 `contract_id`，版本是本设计补的**（§7.3：ContractDiff 必须知道基准版本），
  这一句写进类型文档。
- `graph_id_and_product_id_are_opaque_strings`：`GraphIdRef` / `ProductIdRef` 是不透明 `String`
  （§1.2.3 规则 3：取 `continuum_graph::GraphId` 会写出一条指向执行层的边）——
  两条各断言表外取值往返，并断言本 crate 里 `continuum_graph` 零命中（Task 21 的 grep 是第二侧）。
- `the_intent_table_has_exactly_the_expected_columns`：`PRAGMA table_info(intent)` 实际列 vs 手写期望，逐列。
- 迁移 110 已含 `intent` 表；**本 task 只加这一张表**（其余六张在 Task 10／11／13／15 追加，见 Global Constraints 的那一条）。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test intent
```

- [ ] **Step 4: 实现**

```rust
/// §222 的六个 kind，封闭六值。
pub enum IntentKind { Task, Product, Maintenance, Research, Background, RuntimeImprovement }

/// §222 的 `priority` **未给取值域**（§16 第 8 条）——本层取不透明表示、**不排序**。
pub struct Priority(Opaque);

/// §222 的 `contract_id` 补上版本（§7.3：ContractDiff 必须知道基准版本）。
pub struct ContractRef { pub id: ContractId, pub version: u32 }

/// §222 的十五个字段。**语义内容的五个字段变化时 `version` +1，状态迁移不 +1**（§5.2 的设计决定）。
pub struct Intent {
    pub id: IntentId, pub version: u32, pub kind: IntentKind, pub goal: CanonicalRepresentation,
    pub status: IntentStatus, pub priority: Priority, pub created_at: i64, pub updated_at: i64,
    pub parent_intent: Option<IntentId>, pub contract_id: ContractRef, pub active_plan_id: Option<PlanId>,
    pub graph_id: Option<GraphIdRef>, pub product_id: Option<ProductIdRef>, pub completion_predicate: Opaque,
}

/// 迁移 110。**本 task 只建 `intent` 一张表**；其余六张由用它们的 task 追加进同一个迁移
/// （Global Constraints 已写明「Task 20 之后不得再改它的 SQL」）。
pub fn p4_semantics_migrations() -> Vec<Migration>;
```

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(semantics): crate 骨架、Intent 十五字段与版本语义"
```

---

### Task 9: Intent 状态机（§223）与「状态迁移与事件同事务」

**Files:**
- Create: `crates/continuum-semantics/src/intent_tx.rs`
- Create: `crates/continuum-semantics/tests/intent_state.rs`
- Modify: `crates/continuum-semantics/src/{lib.rs,intent.rs,error.rs}`

**Interfaces:**
- Consumes: Task 8 的 `Intent` / `IntentStatus` / `save_intent`；`continuum_persist::{Tx, EventType, Db}`
- Produces: `continuum_semantics::{IntentStatusError, apply_intent_transition}`

- [ ] **Step 1: 写用例**

- `every_listed_pair_is_accepted`：§5.3 的迁移表**逐条** `Ok`（不抽代表）。
- `unlisted_pairs_are_illegal_and_the_error_names_the_pair`：若干条未列出的对得 `Err(Illegal { from, to })`，
  **断言 `from` / `to` 是给的那一对**（不是「返回了 Err」）。
- `self_loops_are_decided_on_both_sides`（**两侧都钉**，照 P3D 的纪律：自环不靠矩阵顺带）：
  `RUNNING → RUNNING` 与 `WAITING → WAITING` 两条 `Ok`；`PROPOSED → PROPOSED` 与 `COMPLETED → COMPLETED`
  两条 `Err(Illegal { from, to })`。红条件：自环随矩阵顺带判（**取反档**——`PROPOSED → PROPOSED` 那一侧红）。
- `completed_to_evolving_depends_on_the_kind`（**两侧**）：`kind = PRODUCT` 得 `Ok`；`kind = TASK` 得
  `Err(NotProduct { kind })`，**断言是哪一枚**。红条件：不判 kind（**放宽档**，TASK 那条红）。
- `the_intent_created_event_is_written_on_the_first_save`：§5.4——断言写出的 `EventType` **是 `IntentCreated`**
  （不是「写了事件」）。红条件：把事件类型写成别的（**取反档**）。
- `the_intent_completed_event_is_written_on_the_completed_transition`：断言是 `IntentCompleted`。
- `a_failed_event_write_leaves_no_half_written_row`（**失败路径 + 半写副作用**）：
  造法照本仓既有先例（`crates/continuum-workspace/src/gate.rs:2514-2536` 的形状）——用
  `Db::open_with(&path, 一个不含 events 表的迁移集)` 起库，于是 `intent` 表在、`events` 表不存在：
  状态迁移会先写 `intent` 行、再写事件而失败。断言**返回具体 `Err`** **且** `intent` 表里那一行的 `status`
  **是迁移前的值**（不是「表里没有行」——行是在的，改没改才是判据）。
  **前提**：`apply_intent_transition` **不自己 `commit`**（Global Constraints 已写死），调用方在 `Err` 后回滚。
- `writing_intent_completed_behind_a_skippable_event_is_rejected`（§5.4 第 2 条，**两侧都钉**）：
  （a）先在事件链上写一条**可跳过**（不可解码／版本未知）的事件，再写 `IntentCompleted` → 断言得具体 `Err`
  （消费者是 `crates/continuum-persist/src/tx.rs:369-373`）；
  （b）**反例一条**：链上无可跳过事件时写 `IntentCompleted` 得 `Ok`。
  红条件：只钉 (a) 会让「本层能不能写 `IntentCompleted`」这件事在正常路径上无人过问。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test intent_state
```

- [ ] **Step 3: 实现**

```rust
/// §223 的 `COMPLETED → EVOLVING` 只在 `kind = PRODUCT` 时合法（原文「Product Intent MAY……」）。
pub enum IntentStatusError { Illegal { from: IntentStatus, to: IntentStatus }, NotProduct { kind: IntentKind } }

/// 状态迁移与对应 `Event` **在同一事务**内写入（照 P1 的 `apply_transition` 形状）。
/// **本函数不自己 `commit`**（P1 的纪律：返回 `Err` 之后调用方必须回滚、不得提交）。
pub fn apply_intent_transition(tx: &Tx<'_>, id: &IntentId, to: IntentStatus) -> Result<(), IntentError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Intent 状态机与状态迁移/事件同事务"
```

---

### Task 10: Specification（§227 §214）与 `unknowns` 的强制形态

**Files:**
- Create: `crates/continuum-semantics/src/spec.rs`
- Create: `crates/continuum-semantics/tests/spec.rs`
- Create: `crates/continuum-semantics/tests/compile_fail/*.rs`
- Modify: `crates/continuum-semantics/{src/lib.rs,src/persist.rs,src/error.rs,Cargo.toml}`（追加 `specification` 表、dev-dep `trybuild`）

**Interfaces:**
- Consumes: Task 8 的 `IntentId` / `Intent`
- Produces: `continuum_semantics::{Specification, SpecDraft, Unknowns, Unknown, SpecError}`

- [ ] **Step 1: 写用例**

- `unknowns_has_exactly_two_variants`：`Enumerated(Vec<Unknown>)` / `NoneFound` 各一条；
  机制 = 穷尽 `match` **不带 `..`**（加第三枚即**编译不过**）。**明写**：编译期照片。
- `none_found_and_an_empty_enumerated_are_distinguishable_after_a_round_trip`（**本 task 的要害**）：
  `Unknowns::NoneFound` 与 `Unknowns::Enumerated(vec![])` 各写一次、读回，断言读回的是**各自那一枚**。
  红条件：把两者编成同一个落库表示（**移除档**）——这正是 §227 禁的
  「把未知项自动转换成假设」最容易发生的位置（`vec![]` 同时表示「已查过、没有」与「没查」）。
- `a_specification_cannot_be_constructed_outside_the_crate`：trybuild 反例（`.stderr` 实跑取值）。
- `a_draft_without_unknowns_cannot_be_finalized`：**签名层面**——`SpecDraft` 里**没有** `unknowns` 字段，
  `finalize` 的入参要求调用方**显式给出** `Unknowns` 之一（不是 `Option<Unknowns>`）；
  trybuild 反例：少给该参数编译不过。红条件：把 `finalize` 的入参改成 `Option<Unknowns>` 且 `None` 表示
  `NoneFound`（**移除档**——「没查」重新变得可表达）。
- `a_specification_has_exactly_the_six_tuple_fields`：字段清单逐项（穷尽解构，不带 `..`），
  六个字段名照 §227：`goals` / `invariants` / `acceptance_predicates` / `forbidden_states` / `preferences` / `unknowns`。
- `an_intent_version_and_its_specification_are_written_in_one_transaction`（§6.2）：
  （a）正常路径：写 Intent v3 与 Specification **同事务**，两张表都读到；
  （b）**失败路径**：写 Intent v3 而**不写** Specification 的路径若失败 → 断言**两张表都没有半写的行**
  （不是只断言 `Err` 哪一种）。
  红条件：把两次写拆成两个事务（**移除档**——届时 (b) 会留下一个没有 Specification 的 Intent v3，
  而设计要点名排除了这种可漂移状态）。
- `conflict_and_undefined_behaviour_are_types_only`（§6.1 的「不建检测器」）：
  **否定式照片**——`src/` 里没有 `detect_conflicts` / `find_undefined_behaviour` 一类的函数
  （trybuild 反例 + Task 21 的 grep）。**并写明为什么这不是漏项**：§214 的「什么算冲突」规范没给判据
  （§16 第 10 条），本层只把 `Conflict` / `UndefinedBehavior` 作为 `Unknowns::Enumerated` 的成员**保存**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test spec
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test type_level
```

- [ ] **Step 3: 实现**

```rust
/// §227 的六元组。**无公开构造函数**——唯一构造入口是 `SpecDraft::finalize`。
pub struct Specification { /* goals / invariants / acceptance_predicates / forbidden_states / preferences / unknowns */ }

/// §227 的两句 MUST 的落点：`Enumerated` 是「显式枚举出的未知项」，`NoneFound` 是「已查过、没有」。
/// **不是 `Vec<Unknown>`**——`vec![]` 会同时表示两者，而 §227 禁的正是后者被当成前者。
/// `SpecDraft` 里**没有** `unknowns` 字段，`finalize` 要求调用方显式给出 `Unknowns` 之一。
pub enum Unknowns { Enumerated(Vec<Unknown>), NoneFound }

pub struct SpecDraft { /* … */ }
impl SpecDraft {
    pub fn finalize(self, unknowns: Unknowns) -> Specification;   // 注意：不是 Option<Unknowns>
}
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Specification 六元组与 Unknowns 的强制形态"
```

---

### Task 11: Contract（§224）、`verification_requirement` 的禁读与版本化（§226）

**Files:**
- Create: `crates/continuum-semantics/src/contract.rs`
- Create: `crates/continuum-semantics/tests/contract.rs`
- Modify: `crates/continuum-semantics/{src/lib.rs,src/persist.rs,src/error.rs,src/ids.rs}`

**Interfaces:**
- Consumes: Task 8 的 `ContractRef` / `ContractId`；`continuum_persist::{Tx, AuditKind}`
- Produces: `continuum_semantics::{ContractId, RequirementId, Requirement, RequirementClass, VerificationRequirement, TaskContract, ContractError, save_contract, load_contract}`

- [ ] **Step 1: 写用例**

- `every_requirement_class_has_a_photo`：`REQUIRED / PREFERRED / FLEXIBLE / UNSPECIFIED` **四值各一条**
  写入并读回，断言是**哪一枚**。
- `verification_requirement_exposes_no_judging_method`（§7.2，OPEN-001 未决期间的**结构性禁读**）：
  公开方法集**逐条**——只有 `as_str` / `from_str`。照片机制：trybuild **反例**（`vr.is_satisfied()`、
  `vr.check(...)`、`vr.evaluate(...)` 各编译不过，`.stderr` 各自实跑取值）＋ **正控制**（`vr.as_str()` 编译通过）。
  **并写明为什么**：§2.5 明写该字段可以存储、但在 OPEN-001 解决前无法判定是否被满足——
  补三个可选方向（a/b/c）中的任何一个都是发明判据。
- `the_layer_has_no_function_that_takes_a_verification_requirement_as_a_judging_input`：
  **否定式照片**——`src/` 里没有以它为输入做判定的函数（公开面清单逐条 + Task 21 的 grep）。
  与上一条合起来是本组件的两侧。
- `the_contract_requirement_table_has_exactly_the_expected_columns`：`PRAGMA table_info(contract_requirement)`
  实际列 vs 手写期望，逐列（加一列即红）。字段名照 §224：`id` / `class` / `expression` / `source` / `verification_requirement`。
- `a_second_write_of_the_same_id_and_version_hits_the_primary_key`（§226 的 MUST NOT 原地修改）：
  裸 `INSERT` 同 `(id, version)` 得撞主键的 `Err`，**且原行逐列不变**。
  红条件：`save_contract` 用 `INSERT OR REPLACE`（**移除档**）。
- `a_new_version_writes_exactly_one_contract_changes_audit_row`：断言 `AuditKind` **是 `ContractChanges`**，
  且它与 INSERT **在同一事务**（失败路径：审计写失败 ⇒ `contract` 表**无半写行**，照 Task 9 的造法）。
  **并写明**：`EventType` / `AuditKind` 的变体一个不改、一张表不加（§5.4）——本层只使用既有的那六个写入点。
- `the_other_fields_of_224_are_opaque`：`TaskContract` 的其余字段（`budget` / `authority_limit` / `data_policy` /
  `model_policy` / `node_policy` / `acceptance_predicate`）规范**只给名字**，本层一律取不透明值——
  字段清单逐项，且每个字段的表外取值可往返。**并写明**：§224 的 `budget` 与 §12 的预算**不是同一个东西**
  （前者是声明字段、后者是记账对象，两者的桥规范没给，§16 第 11 条）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test contract
```

- [ ] **Step 3: 实现**

```rust
/// §224 的四个 class，封闭四值。
pub enum RequirementClass { Required, Preferred, Flexible, Unspecified }

/// §224 的 `verification_requirement`。**只有 `as_str` / `from_str`**，没有 `is_satisfied` / `check` / `evaluate`
/// ——OPEN-001 未决期间只存不判（§7.2、《工程》§2.5）。
pub struct VerificationRequirement(String);

pub struct Requirement {
    pub id: RequirementId, pub class: RequirementClass,
    pub expression: Opaque, pub source: Opaque, pub verification_requirement: VerificationRequirement,
}

/// §224 的 `TaskContract`。**落库主键 `(id, version)`，无 UPDATE 路径**（§226 MUST NOT 原地修改）。
pub fn save_contract(tx: &Tx<'_>, c: &TaskContract) -> Result<(), ContractError>;   // 只 INSERT，不 commit
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Contract 四 class、verification_requirement 禁读与版本化"
```

---

### Task 12: ContractDiff（§226）——四类差异与并列的 `unjudged`

**Files:**
- Create: `crates/continuum-semantics/src/diff.rs`
- Create: `crates/continuum-semantics/tests/diff.rs`
- Modify: `crates/continuum-semantics/src/lib.rs`

**Interfaces:**
- Consumes: Task 11 的 `TaskContract` / `Requirement` / `RequirementClass` / `ContractRef`
- Produces: `continuum_semantics::{DiffClass, DiffEntry, UnjudgedChange, UnjudgedReason, ContractDiff, diff}`

- [ ] **Step 1: 写用例**

- `each_diff_class_has_a_photo`：`Added` / `Removed` / `Strengthened` / `Weakened` **四枚各一条**用例。
  机制 = 穷尽 `match` **不带 `..`**（加第五枚即**编译不过**）。**§226 的枚举恰好四枚**——
  这条断言的用途是把「记录 vs 差异类」的分界钉在**类型**上（见下一条）。
- `added_and_removed_are_decided_by_the_id_set`：`(id)` 在 vN 无、vN+1 有 → `Added`；反向 → `Removed`。
- `a_rising_class_is_strengthened_and_a_falling_one_is_weakened`：同 id 两边都有，`class` 的序上升/下降各一条。
- `the_class_order_is_the_one_the_design_declared`：`UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`
  **三对逐对断言**。**并写明**：这个序**是本设计从四类含义推出来的、不是原文给的**（§8.1 的订正段：
  总纲的列举顺序与此相反）；**若复审判定此序不成立，`Strengthened` / `Weakened` 两类整个落进 `unjudged`**
  （§16 第 12 条，`## 遗留` 照抄）。
- `a_same_class_with_a_changed_expression_is_unjudged_and_not_in_the_entries`（**本 task 的要害**）：
  夹具：同 id 两边 `class` 相同、`expression` 不同 → 断言它出现在 `unjudged` 里，
  **且它在 `entries` 里不出现**（逐条比对 `entries` 的 `requirement_id`）。
  红条件：把未判定项并进 `entries`（**移除档**）。**这条断言把「记录 vs 差异类」的分界钉死**
  （§8.1 的订正段：一旦成为 diff 的第五个变体，它就是差异类，措辞声明救不回来）。
- `contract_diff_carries_the_two_versions_and_two_collections`：字段清单逐项（穷尽解构，不带 `..`）：
  `from` / `to` / `entries` / `unjudged`。**`unjudged` 是并列字段，不是第四个差异类**。
- `the_diff_is_computed_from_two_contract_refs_only`：签名层面——`diff` 的两个入参是两个 `ContractRef`
  ＋ 两侧的 Requirement 集合（**不收 `&Tx`、不碰图**）。§8.2：本层**不调 `propagate_invalidation`**、不引用 `continuum-graph`
  （Task 21 的 grep 是第二侧）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test diff
```

- [ ] **Step 3: 实现**

```rust
/// §226 的四类差异。**恰好四枚**，与 §226 的枚举一一对应。
pub enum DiffClass { Added, Removed, Strengthened, Weakened }

pub struct DiffEntry { pub requirement_id: RequirementId, pub class: DiffClass }

/// 本层判不了的变更。**不是差异类**——它是「记录」，不是「判定」。
pub struct UnjudgedChange { pub requirement_id: RequirementId, pub reason: UnjudgedReason }

pub struct ContractDiff {
    pub from: ContractRef, pub to: ContractRef,
    pub entries: Vec<DiffEntry>,          // 恰好 §226 的四类
    pub unjudged: Vec<UnjudgedChange>,    // 并列字段，**不并入 entries**
}
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): ContractDiff 四类差异与并列的 unjudged"
```

---

### Task 13: Plan 与审查状态机（§228 §229 §338–§341）

**Files:**
- Create: `crates/continuum-semantics/src/{plan.rs,review_view.rs}`
- Create: `crates/continuum-semantics/tests/plan.rs`
- Modify: `crates/continuum-semantics/{src/lib.rs,src/persist.rs,src/error.rs}`（追加 `plan` 表）

**Interfaces:**
- Consumes: Task 8 的 `IntentId`、Task 11 的 `ContractRef`、Task 10 的 `Specification`
- Produces: `continuum_semantics::{Plan, PlanType, PlanVersion, ReviewState, PlanError, ReviewFinding, Severity, ReviewIndependence, ModelRefRef, ReviewFindingRecord, UserReviewView}`
  （`ModelRefRef` 是**不透明 `String`**——§1.2.3 规则 3：取资源层的模型 id 类型会写出一条指向资源层的边）

- [ ] **Step 1: 写用例**

- `plan_carries_exactly_the_ten_fields_of_228`：字段清单逐项（穷尽解构，不带 `..`）：
  `id` / `version` / `intent_id` / `type` / `objectives` / `milestones` / `assumptions` / `risks` /
  `acceptance_conditions` / `review_state`。
- `plan_version_is_two_parts`（§9.1）：
  （a）`MATERIAL` 变更 ⇒ `major` +1；
  （b）`MINOR` 变更 ⇒ `minor` +1。
  **并写明**：`§228` **只给 `version` 一个字段名**，两段式是本设计的决定（收件人：复审者，`## 遗留` 照抄）。
  红条件：只 +1 一个数（**收紧档**，(a) 或 (b) 必有一侧红）。
- `every_listed_review_pair_is_accepted`：§9.2 的迁移表逐条 `Ok`。
- `unlisted_review_pairs_are_rejected`：若干条未列出的对 `Err(IllegalReview { from, to })`，断言给的那一对。
- `a_strategic_plan_cannot_be_executed_without_user_approval`（**一侧**）：`STRATEGIC` 未经 `USER_APPROVED`
  去 `READY` 得 `Err(UserApprovalRequired)`。
- `an_execution_plan_reaches_ready_without_user_approval`（**另一侧，不能省**）：
  `EXECUTION` 同路径得 `Ok`。§229 的 MUST 只对 Strategic Plan（§341 把用户审查限定在战略层）。
  红条件：把 MUST 套到所有 Plan（**收紧档**，本用例红）。
- `a_revision_is_a_new_version_and_the_old_one_becomes_superseded`（§9.2）：
  「修订」**不是第六个态**——产生新版本（`DRAFT`）＋ 旧版本转 `SUPERSEDED`。一条用例，断言两件事都成立。
  红条件：把修订实现成原地改状态（**移除档**）。
- `every_review_finding_severity_has_a_photo`：`BLOCKER / MAJOR / MINOR / SUGGESTION` **四值各一条**。
- `a_review_finding_record_carries_the_independence_facts`：`ReviewIndependence` 的字段清单逐项
  （穷尽解构，不带 `..`）：`separate_context` / `reviewer_model` / `author_model` / `cross_family`；
  且 `ReviewFindingRecord` 必带它（少一个字段即**编译不过**）。
- `a_same_model_same_context_review_is_observable_as_such`（§340 的落点）：
  构造一次「同一模型 + 同一上下文」的审查，断言读回 `separate_context == false` **且**两个 model ref 相同
  （两条断言都写，不靠一条顺带）。**并写明**：§340 的三条是 SHOULD、**「分离到什么程度算够」规范没给**
  （§16 第 14 条）——故本层**不禁止**「同模型同上下文」，只**使事实可观察**；那一条判定**没有照片**。
- `the_user_review_view_has_exactly_the_eight_lines_of_341`（**三侧，负向守卫**）：
  字段清单**逐项八条**——**多一项即红、少一项即红、`成本` 与 `权限` 合并成一项即红**。
  机制 = 穷尽解构**不带 `..`**：多一项/改名/合并都**编译不过**。
  字段名与 §341 的八行一一对应：`objectives` / `major_designs` / `key_constraints` / `milestones` /
  `risks` / `cost` / `authority` / `acceptance_conditions`。
  **这是负向守卫，本仓接受否定式照片**（`docs/superpowers/p3bcdf-followups.md` §四.4）。
  **这一条的来历要写进注释**：设计初稿把 `成本` 与 `权限` 合并成 `cost_and_authority` 并写成「七项（原文如此）」，
  **那句话是假的**——§341 写的是八行；**出错的恰是「少一项」那一侧**。
- `the_user_review_view_has_no_field_pointing_at_internal_steps`：**否定式照片**——八项里没有任何一项承载
  Plan 的内部执行步骤（§341 的清单是这个视图的**全部**判据）；本组件**不是 UI**（§2.1 的组件表里没有用户界面组件）。
- `the_plan_table_has_exactly_the_expected_columns`：实际列 vs 手写期望，逐列。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test plan
```

- [ ] **Step 3: 实现**

```rust
/// §228 的 `version` 取**两段**（本设计的决定，§9.1）：major 在 MATERIAL 时 +1、minor 在 MINOR 时 +1。
pub struct PlanVersion { pub major: u32, pub minor: u32 }

/// §340 的三条独立性事实。**必填**——不是布尔标志位，是三条各带来源的记录。
pub struct ReviewIndependence {
    pub separate_context: bool, pub reviewer_model: ModelRefRef,
    pub author_model: ModelRefRef, pub cross_family: bool,
}

/// §341 的用户审查视图。**字段恰好八项**，与 §341 的八行一一对应，**不合并任何两行**。
pub struct UserReviewView { /* objectives / major_designs / key_constraints / milestones / risks / cost / authority / acceptance_conditions */ }
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Plan 审查状态机、ReviewIndependence 与 §341 的八项视图"
```

---

### Task 14: Plan Change 分类（§230）与十个例

**Files:**
- Create: `crates/continuum-semantics/src/plan_change.rs`
- Create: `crates/continuum-semantics/tests/plan_change.rs`
- Modify: `crates/continuum-semantics/src/lib.rs`

**Interfaces:**
- Consumes: Task 13 的 `Plan` / `ReviewState`
- Produces: `continuum_semantics::{PlanChangeClass, PlanChange, classify, apply_plan_change}`

- [ ] **Step 1: 写用例**

- `both_plan_change_classes_have_a_photo`：`MINOR` / `MATERIAL` 各一条。
- `each_of_the_ten_examples_of_230_classifies_as_the_spec_says`（**逐项有照片，不抽代表**）：
  §230 的**十个例**（**五个 MINOR ＋ 五个 MATERIAL**，`docs/spec/05-normative.md:403-417`）**逐例一条**，共十条，
  每条断言是**哪一枚**。
  **并写明这条表驱动用例的限度**：「表驱动用例也可能覆盖错路径」（本仓已付过代价）——故**另加至少一条
  「直接调用分类函数」的用例**（不经表），并把两个 class 各自的**第一条**例在实现注释里点名。
  红条件：把 MATERIAL 的某一例判成 MINOR（**放宽档**，那一例红）。
  **数的来历要写进注释**：设计初稿把它写成「十一个例（五个 MINOR、五个 MATERIAL，原文实为五个 MATERIAL）」
  ——**「十一」与括号里的「五个＋五个」相抵**；原文本来就是**两个五**。
- `a_material_change_re_triggers_the_review_gate`：§230 的 MUST——`MATERIAL` 时 `apply_plan_change`
  产出**新版本**（`major` +1）且新版本 `review_state == DRAFT`；`MINOR` 时不重触发（`minor` +1、审查态保持）。
  **两侧都钉**——只钉 MATERIAL 那侧会让「MINOR 也重开审查」漂过去。
- `affected_scope_is_opaque`：`PlanChange` 的 `affected_scope` **§230 未给取值域** → 不透明值（§16 第 15 条）——
  表外取值往返一条。**并明写**：本层不发明取值域。
- `the_classification_reads_both_plans_and_both_footprints`：签名层面——
  `classify(&Plan, &Plan, &PlanFootprint, &PlanFootprint) -> PlanChangeClass`。
  **依据**：`权限扩大` 与 `成本显著扩大` 需要计划侧声明的影响面（§11.2 的 `PlanFootprint`）。
  **并明写**：「显著扩大」的**阈值规范未给**（§16 第 15 条）——本层**不发明阈值**；
  `PlanFootprint` 的两个 `estimated_cost` / `authority` 以不透明值传入，比较由调用方给出的
  `MaterialThresholds` 一类入参承载（**若无该入参，则本层只能判「两者不相等即 MATERIAL」，
  而那句话在规范里没有依据**——**故本 task 的前置是先有 `PlanFootprint`（Task 16）与一个显式的阈值入参**；
  阈值入参的形状属本计划自定，见 `## 遗留` 第五节）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test plan_change
```

- [ ] **Step 3: 实现**

```rust
/// §230 的两个 class，封闭两值。MATERIAL MUST 重触发审查门。
pub enum PlanChangeClass { Minor, Material }

pub struct PlanChange { pub class: PlanChangeClass, pub affected_scope: Opaque }

/// 分类函数收**两份计划 ＋ 两份声明的影响面**（§9.4）。
pub fn classify(before: &Plan, after: &Plan,
                before_fp: &PlanFootprint, after_fp: &PlanFootprint) -> PlanChangeClass;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Plan Change 的两个 class 与 §230 的十个例"
```

---

### Task 15: Decision 对象（§331）与恢复链（§332）

**Files:**
- Create: `crates/continuum-semantics/src/decision.rs`
- Create: `crates/continuum-semantics/tests/decision.rs`
- Modify: `crates/continuum-semantics/{src/lib.rs,src/persist.rs,src/error.rs}`（追加 `decision` / `decision_result` 表）

**Interfaces:**
- Consumes: Task 8 的 `Intent` / `apply_intent_transition`、Task 11 的 `Contract`、Task 13 的 `Plan`
- Produces: `continuum_semantics::{Decision, DecisionOption, DecisionResult, DecisionError, create_decision, save_decision_result, apply_result}`

- [ ] **Step 1: 写用例**

- `a_decision_option_cannot_be_built_without_a_consequence`（§106 的「每个选项带后果」落在**构造期**）：
  trybuild 反例：不存在只带选项的构造路径（`.stderr` 实跑取值）；**正控制**：
  `DecisionOption::new(option, consequence)` 编译通过。
- `options_and_consequences_are_equal_length_for_every_decision`：**枚举式的属性断言**——
  对每个读回的 `Decision` 断言 `options.len() == consequences.len()`（等长由类型保证，不由检查保证）。
  红条件：加一条「只带选项」的构造路径（**移除档**——那时上一条 trybuild 也红，两条一起钉）。
- `a_decision_is_created_with_blocking_true_at_the_two_trigger_points`（§10.1）：
  （a）Constraint Validator 判违反 REQUIRED；（b）Budget Validator 判预计超支且找不到合法低成本方案——
  **两条各一条用例**，断言 `blocking == true`。
  **并明写为什么没有 `false` 的照片**：`blocking` 的取值域 §331 未给，`false` 的语义「不阻塞」在规范里
  没有用武之地（§16 第 16 条）——本层保留该字段并**只在两个触发点写 `true`**，**不发明 `false` 的触发条件**。
- `the_public_face_has_exactly_one_type_named_decision`：`continuum-semantics` 的公开面清单里 `Decision`
  **恰好一个**（不是两个，与 `continuum_policy::Decision` 不混）。
  **并明写这条守卫的限度**：`PolicyDecision` 别名是**命名约定、不是类型守卫**——
  没有任何编译期机制阻止后来者并列 `use continuum_policy::Decision;` 与 `use continuum_semantics::Decision;`
  （§16 第 31 条），**故这条没有更强的照片**。收件人：复审者（要不要更强）。
- `a_decision_required_event_is_written_on_creation`：断言 `EventType` **是 `DecisionRequired`**。
- `a_decision_result_changes_only_the_fields_it_names`（§107）：
  一个只指名某个 Requirement 字段的 `DecisionResult` 应用后，**其余字段逐项不变**（逐项断言，不抽代表）。
  红条件：把 `apply_result` 实现成「重新跑一遍 Intent Compiler」（**移除档**）。
- `there_is_no_path_that_recompiles_a_decision_result_as_text`：**否定式照片**——公开面清单逐条：
  没有 `apply_result_from_text` / 把 `DecisionResult` 序列化成文本再重新编译的路径（§107）。
- `the_recovery_chain_is_atomic`（§10.2 的原子性，**失败路径 + 四张表**）：
  `DecisionResult` 落库 ＋ Contract/Plan 新版本 ＋ Intent 状态回迁**在同一事务**；任一步失败时断言
  `decision_result` / `contract` / `plan` / `intent` **四张表都没有半写的行**（**不只断言 `Err` 是哪一种**）。
  造法照 Task 9（迁移集里去掉其中一张表）。**前提**：写函数都不自己 `commit`。
- `a_user_approvals_audit_row_is_written_for_a_decision_result`：断言 `AuditKind` **是 `UserApprovals`**；
  **并写明**：本层是它的**第二个生产方**（第一个是 `continuum-workspace` 的 Integration Gate，
  `crates/continuum-workspace/src/gate.rs:222` / `:303` / `:348`，写的是操作名如 `"apply_patch"`），
  本层写的是决策单与被选项。`append_audit` 的 `payload` 是自由 JSON（`crates/continuum-persist/src/tx.rs:79-84`），
  **同一个 kind 下不保证载荷同形**——读 `audit_log` 的一方**不得假定「一个 kind 一种载荷」**（§5.4、§16 第 33 条）。
  本层**不改 Gate、也不新增 kind**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test decision
```

- [ ] **Step 3: 实现**

```rust
/// §331 的 `{ issue, options[], consequences[], blocking }`——「而不是只发一句自由文本问题」。
/// **`options[]` 与 `consequences[]` 是等长的配对**（§106），由**构造期**保证。
pub struct Decision { /* issue / options / consequences / blocking */ }

/// **不接受缺后果的选项**——故「等长」由类型保证而不是由检查保证。
impl DecisionOption { pub fn new(option: Opaque, consequence: Opaque) -> Self; }

/// §107：一个**结构化的、指向某个 `Decision` 与某个 `DecisionOption` 的值**。
/// `apply_result` 的输入只有它——不是自由文本、不重跑 Intent Compiler。
/// **四步同一事务**（§10.2）：结果落库 → Contract/Plan 新版本 → Intent 状态回迁（图失效由执行层做）。
pub fn apply_result(tx: &Tx<'_>, r: &DecisionResult) -> Result<(), DecisionError>;   // 不 commit
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Decision 对象与 §332 恢复链的原子性"
```

---

### Task 16: Constraint Validator（§225 §6）——两条拦下路径与三侧照片

**Files:**
- Create: `crates/continuum-semantics/src/constraint.rs`
- Create: `crates/continuum-semantics/tests/constraint.rs`
- Modify: `crates/continuum-semantics/src/lib.rs`
- Modify: `crates/continuum-semantics/Cargo.toml`（dep `serde_json`——`Node.execution_policy` 按**原样类型**进入）

**Interfaces:**
- Consumes: Task 13 的 `Plan`、Task 11 的 `TaskContract` / `Requirement` / `RequirementId` / `RequirementClass`
- Produces: `continuum_semantics::{CandidatePlan, PlanFootprint, ConstraintVerdict, ViolatedProhibition, ConstraintError, validate}`

- [ ] **Step 1: 写用例**

**输入面**（§11.2 的 `CandidatePlan` 与 `PlanFootprint`）：

- `the_candidate_plan_is_a_value_of_a_plan_plus_a_footprint`：签名层面——`CandidatePlan<'a> { plan: &'a Plan, footprint: &'a PlanFootprint }`；
  `validate` **不收 `&Tx`、不收 `&dyn` 来源**（§1.2.3 规则 3：跨层输入以值进入）。
- `the_four_footprint_fields_are_all_declared`：字段清单逐项（穷尽解构，不带 `..`）：
  `authority` / `privacy_class` / `external_effects` / `estimated_cost`。
  **并写明本类型的由来**：§228 的 `Plan` **没有**这几个字段，而 §225 的四条 MUST NOT 的后三条比的是
  **计划的声明值**与 Contract 的 `authority_limit` / `data_policy`——没有计划侧的对偶声明，后三条**无可判**。
  **故本类型是「让 §225 有判据」的载体，不是本层新造的判据**。

**`execution_policy` 的十条**（§11.2，收原样类型）：

- `the_seven_may_changes_are_all_allowed`：**逐项 7 条**——`换模型 / 换工具 / 换节点 / 换算法 / 重新规划 /
  改变并行度 / 改变内部执行顺序` 各一条，断言 `Satisfied`（放行）。
- `the_three_prohibitions_are_all_blocked`：**逐项 3 条**——`扩大权限 / 降低隐私约束 /
  增加高风险外部副作用`（§225 的 MUST NOT 的后三条）各一条，断言 `ViolatesProhibition { what }` 且
  `what` 是**对应那一枚**（`Authority` / `Privacy` / `Effect`）。
- **用例数写死**：**7 ＋ 3 ＝ 10**。**来历要写进注释**：设计初稿写「（14 项）」，而 §225 实为
  **MAY 七条 ＋ MUST NOT 四条**，故 7＋3＝10、7＋4＝11，**都不是 14**；
  §225 的**第四条** MUST NOT（「修改 REQUIRED」）**不是本表的行**，它是下面 `ViolatesRequired` 那一条。
- `the_node_side_inputs_keep_their_original_types`：签名层面——`Node.constraints` 以 `&[String]` 进入、
  `Node.execution_policy` 以 `&serde_json::Value` 进入，**不经过本层的类型转换**（§11.2、§13.2 末行）。
  第二侧：本 crate 对 `continuum-graph` 的引用为零（`ALLOWED` 条目 ＋ Task 21 的 grep）。

**裁决的五条**（§11.3）：

- `a_violation_of_a_required_requirement_is_blocked`：`ViolatesRequired { requirement }` 一条，断言 `requirement`。
- `a_violation_of_the_other_three_classes_is_not_blocked`：`PREFERRED` / `FLEXIBLE` / `UNSPECIFIED`
  **各一条**「违反但不拦下」（§224 的四类含义：PREFERRED「有充分理由时可调整」、FLEXIBLE「明确允许 Runtime 自动调整」）。
  红条件：把所有 class 的违反一律拦下（**放宽档**，这三条红）。

**「判不了」的三侧**（§11.3，**三侧合起来才算钉住**）：

- `unknown_constraint_key_is_an_error`：`constraints` 里放一个**不在当前 Contract 的 Requirement id 集合里**的键，
  断言 `Err(UnknownConstraintKey { key })`，且 `key` 是**那一个**。
- `empty_constraints_is_not_an_error`：`constraints` 为空**不是** `Err`（空集合法，`Node::new` 就置空，
  `crates/continuum-graph/src/node.rs:31`）。
- `a_key_pointing_at_a_preferred_requirement_is_not_an_error`（**第三条，评审查出后补的那一条**）：
  键**命中**一条 `class = PREFERRED` 的 Requirement → 断言**不是 `Err`、也不拦下**。
  **没有这一条，「不指向 REQUIRED 就报错」的实现照样全绿**，而那会绕掉上面
  `a_violation_of_the_other_three_classes_is_not_blocked` 的三条。
  **判据写死**：`Err` 的触发条件是「**`key` 不在当前 Contract 的 Requirement id 集合里**」，
  **与 `class` 无关**；命中一条非 REQUIRED 的键时，裁决由该 Requirement 的 `class` 决定。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-semantics --test constraint
```

- [ ] **Step 3: 实现**

```rust
/// 候选计划 = §228 的 Plan ＋ 声明的影响面（§11.2 的本设计类型）。
pub struct CandidatePlan<'a> { pub plan: &'a Plan, pub footprint: &'a PlanFootprint }
pub struct PlanFootprint { pub authority: Opaque, pub privacy_class: Opaque,
                           pub external_effects: Vec<Opaque>, pub estimated_cost: Opaque }

/// 判定**不是**「通过／违反／不确定」——「不确定」在 §6 里没有位置。
pub enum ConstraintVerdict {
    Satisfied,
    ViolatesRequired { requirement: RequirementId },
    ViolatesProhibition { what: ViolatedProhibition },
}
pub enum ViolatedProhibition { Authority, Privacy, Effect }

/// 「判不了」走 `Err`，不走第三个裁决值（§11.3 判据 1、2）。
pub enum ConstraintError {
    /// `Node.constraints` 里的一个串**不在当前 Contract 的 Requirement id 集合里**。
    /// **判据与 `class` 无关**。
    UnknownConstraintKey { key: String },
}

/// §11.4：本组件**自己不挂起、不写 Intent 状态**——只返回裁决。
/// 挂起与建 Decision 是调用方（本 crate 的 Intent 路径）的下一步。
pub fn validate(plan: &CandidatePlan<'_>, contract: &TaskContract,
                constraints: &[String], execution_policy: &serde_json::Value)
                -> Result<ConstraintVerdict, ConstraintError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-semantics
git commit -m "feat(semantics): Constraint Validator 的两条拦下路径与三侧照片"
```

---

### Task 17: `continuum-budget`——预算树、三角色三类型、I1/I2/I3 与账本

**Files:**
- Create: `crates/continuum-budget/Cargo.toml`
- Create: `crates/continuum-budget/src/{lib.rs,dimensions.rs,account.rs,tree.rs,ledger.rs,persist.rs,error.rs}`
- Create: `crates/continuum-budget/tests/{tree,persist}.rs`
- Modify: `Cargo.toml`（members）、`crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED`）

**Interfaces:**
- Consumes: `continuum_semantics::IntentId`（预算树的 owner 取它）；`continuum_persist::{Tx, Migration, Db, Value}`
- Produces: `continuum_budget::{DimensionKind, Dimensions, Allocation, Reservation, Remaining, BudgetOwner, LedgerKind, BudgetError, reserve, settle, SettleOutcome, allocate_child, remaining, dangling_reservations, p4_budget_migrations}`

> **开工前置（核一处设计相抵）**：`BudgetError` 的变体在设计的**三处**说法不同（§12.2 的散文写
> `Insufficient { dimension }`、§12.2 的枚举只列 `UnknownOwner` ＋ `NegativeActual`、§12.4 的枚举
> 只列 `UnknownOwner` 并写「只收一种」）。**本计划按三处的并集取三枚**
> （`UnknownOwner` / `Insufficient` / `NegativeActual`），逐枚注明出处，见 `## 遗留` 第一节第 5 条；
> 开工前先核设计是否已把它定死，未定则**照本计划的并集落地并回报**（不擅自删任何一枚——
> 「`reserve` 返回具体哪一维不足」是 §12.2 正文与 §14 表格都写着的判据，删了它那五条照片没有承载）。

- [ ] **Step 1: 现场核对迁移号 120 为空号，登记 members 与 `ALLOWED`**

`ALLOWED` 加（**本 task 实际有的两条边**）：

```rust
    // P4 语义层（设计 §2.1）：预算树、Allocation / Reservation / Remaining、账本、Budget Validator。
    // `← continuum-semantics` 是 owner 要 IntentId（§12.2）；`← continuum-persist` 是账本与树经 Tx。
    // **反向没有边**：continuum-semantics 的允许集里没有本 crate（§11.5 的两侧守卫之一）。
    ("continuum-budget", &["continuum-persist", "continuum-semantics"]),
```

- [ ] **Step 2: 写用例**

- `the_three_roles_are_three_types`（§12.1 的「三角色三类型」）：
  照片机制 = **trybuild 反例**：`let _: Allocation = remaining;` / `let _: Remaining = allocation;` 编译不过
  （`.stderr` 各份实跑取值）；**正控制**：三者各自的读法可用。
  三个 newtype 各包一个 `Dimensions`，**类型不共用**——故「把余量填进上限的位置」在类型上不可写。
  **明写**：编译期照片，运行期无照片（纯数据声明）。
- `each_role_has_exactly_one_constructor_and_no_second_public_path`（**§12.1 :1540-1544 点名给
  `plan-p4` 的那一条**）：三个角色**逐个**断言，各一条 trybuild 反例 ——
  在 crate 外**构造** `Allocation` / `Reservation` / `Remaining`（各用元组构造形式 `Allocation(dims)`）
  ⇒ **编译不过**，且 `.stderr` 钉住的是**它该有的那个错**（字段私有；
  **形态照 P3A 对 `AuthorizedTool` 的样例**，判据是「失败必须落在私有性上」，不是拼错名字之类的别的错）。
  **三个角色各一份反例，共三份、三份 `.stderr` 各不相同**（错误码一律以实跑为准、不许凭记忆写）。
  **正控制三条**（**缺了它们，「三个角色根本不存在」也照样绿**）：设计指定的那个入口各一条——
  分配方给 `Allocation`、`reserve` 给 `Reservation`、`remaining()` 给 `Remaining`，三条都编译通过。
  **六条合起来才算钉住**（三反例 ＋ 三正控制）：只钉反例，「三个角色的入口被删光」照样绿；
  只钉正控制，「crate 外随手能造一个」照样绿。
  **并写明这条断言的作用域与时点**：「除设计指定的那个入口外无第二个公开**构造路径**」是**在本 task 结束时**
  由这六份样例钉住的；**三个角色私有、无公开构造函数**这一句在 P4 落地前只是**设计写死的承诺**
  （代码层面当时只能核「设计承诺了」），本 task 是它第一次有代码层的照片。
  **「构造」与「读」要分清**（写进注释）：本断言**只管构造**——三个角色的**读法**（`dims()`、逐维读者）
  是公开的，它们**不构成**第二条构造路径（读不出可写的角色值）。
  这条区分的判据是**类型**：`dims()` 返回 `&Dimensions`（不是角色），故拿它拼不出一个 `Allocation`。
- `dimension_none_is_not_some_zero`：五个量纲**各一条**——`None`（该量纲不构成约束）与 `Some(0)`（额度为零）
  可区分，**含 `None → None`**。红条件：把 `Option<i64>` 换成默认 `0`（**移除档**）；或让某一条夹具里两者恰好同值
  （**那时它是等价变异体**，纪律 1(b) 已点名——故五个夹具的取值必须让 `None` 与 `Some(0)` 输出不同）。
  **依据**：`BudgetView` 的模块文档（`crates/continuum-model-registry/src/budget.rs:31-33`）明写两者不是同一件事，
  本层的 `Remaining` 必须守同一条。
- `the_five_dimension_names_are_recorded_verbatim`：`Dimensions` 的字段清单逐项（穷尽解构，不带 `..`），
  五个字段名照 §333：`money` / `wall_time` / `token` / `gpu_time` / `network_transfer`。
  **并写明**：五个量纲没有单位、没有取值域（§16 第 19 条）——本层**不解释、不换算、不做跨量纲比较**。
- `reserve_reports_the_specific_dimension`：**逐维五条**——某一维不足时返回
  `Err(BudgetError::Insufficient { dimension })`，且 `dimension` **是那一维**。
  红条件：返回笼统的 `Exceeds`（**取反档**——五条全红，因为拿不到具体维）。
  **依据**：ENG-005 的「没有预留，§110 这一步就没有判据」落在这一行上。
- `i1_a_reserve_beyond_the_available_amount_is_rejected`：`available(n,d) ≥ 0` 的违规一条（**fail-closed 侧**）。
- `i2_a_child_allocation_beyond_the_parent_available_is_rejected`：违规一条。
- `i3_settled_plus_reserved_beyond_the_allocation_is_rejected`：违规一条。
  **三条各自一条违规断言，不合并**——它们钉的是三条不同的算术恒等式。
  **I2 的措辞差别要写进注释**：ENG-005 原文说「兄弟节点的活跃预留之和 ≤ 父的剩余」，
  本设计判的是「**兄弟的分配额之和 ≤ 父的可用额**」——即**更强的一条**（分配额 ≥ 该子树任何预留之和，由 I3 保证）；
  **原句未述「分配是上限还是转移」**，本设计取「**分配是上限、不转移**」
  （否则 I2 恒真、那条约束不成立）。**若是裁「分配即转移」，I2 换成**
  `Σ_{执行中的子节点} reserved(child, d) ≤ allocation(n, d) − settled(n, d)`（§12.2，收件人：协调者）。
- `settle_within_refunds_the_unused_amount`：`Ok(SettleOutcome::Within { refunded })` 一条，断言 `refunded` 逐维是差额。
- `settle_over_reports_the_dimension_and_the_amount`：`Ok(SettleOutcome::Over { dimension, by })` 一条，
  断言 `dimension` 与 `by`（**不是笼统的「超了」**）。
- `a_negative_actual_is_rejected_and_the_ledger_is_untouched`（**§12.2 照片 (a)，失败路径两件事都断言**）：
  `actual` 某一维为负 ⇒ `Err(BudgetError::NegativeActual { dimension, value })`，
  `dimension` 是**那一维**、`value` 是**那个负数**；**并断言账本未变**——
  `reserve` 仍活跃（`dangling_reservations` 里还在）、`settled` 逐维未动、`budget_ledger` 行数不变。
  红条件：按 `0` 截断（**移除档**——那时返回 `Ok` 且账面被悄悄改小；
  **本仓口径：截断是静默的**，它把产生方的一个 bug 吞掉）。
  **判据写进注释**：负数是 **fail-open 的入口**——`settled` 会因此变小、`available` 反而变大，
  而 **I1 只约束 `available`、不约束 `actual` 的符号**，故一个负的 `actual` 能把已经花掉的额度
  「还回来」且没有任何断言会拦它。
- `a_zero_actual_settles_within_and_refunds_everything`（**反例，另一侧**）：
  `actual` 各维为 `0` ⇒ `Ok(Within { .. })` 且额度**全额**退回。
  **不能省**——只钉拒绝那侧会让「`0` 也被拒」漂过去（`0` 不是负数，§12.2 拒的是**负**值）。
- `an_over_settlement_writes_no_second_ledger_row`：**否定式照片**——一次 `Over` 结算后 `budget_ledger`
  **只多一条 `settle` 行**。**依据**：运行期实际超支的裁决**不在本层**（ENG-005 §三.2 判给
  `FailureClass::Constraint` ＋ 该节点的 `EscalationPolicy::Decision`，两者都是**执行层的词汇**，
  `crates/continuum-graph/src/failure.rs:8`、`:32`）——**本层只回答「超了多少、哪一维」**。
- `the_ledger_is_append_only_and_balances_are_derived`：账本表 `budget_ledger(owner, dimension, kind, amount, at)`，
  `kind ∈ {allocate, reserve, release, settle}`——**只追加、不 UPDATE**：先前的行在任何后续操作后
  **逐行不变**（逐行比对），且余额是**现算**的（`remaining()` 在账本变化后立刻反映）。
  红条件：把余额也落成一列并在原地更新（**移除档**——那时「先前的行不变」仍成立，但 `remaining()` 与账本会漂移；
  故本条另断言「表里没有 `balance` 一列」，逐列）。
- `a_dangling_reservation_is_detectable`：§12.2 的「半写的副作用在账本上表现为『有一条 `reserve` 而没有对应的
  `settle` 或 `release`』，**是可检出的**」——造一条悬空 `reserve`，断言 `dangling_reservations(tx)` 列出它。
  **这一条同时是「可检出」那句绝对措辞的照片**；`dangling_reservations` 的取名与落点是**本计划自定**
  （设计只写「可检出」，没给函数名，且 Task 18 的恢复钩子必须消费它，见 `## 遗留` 第五节）。
  反向一条：`settle` 或 `release` 之后同一条不再被列出（**两侧**）。
- `the_two_budget_tables_have_exactly_the_expected_columns`：两张表**逐列**（实际列 vs 手写期望）。
- `the_tree_is_one_tree_with_five_owner_variants`：`BudgetOwner` 四个变体
  （`SystemMaintenance` / `Intent(IntentId)` / `NodeRun { intent, node }` / `Exploration { intent, of_node }`）
  **逐枚穷尽解构不带 `..`**。
  **并写明**：`SystemMaintenance` 的**创建方是第 7 层**，**今天没有任何一方会创建它**——
  本层提供该变体与它的入账函数，但**不创建**（§16 第 18 条，`## 遗留`）。故本层**没有**任何
  「造一个 `SystemMaintenance` 节点」的生产路径（否定式照片：`src/` 里没有那个构造点，Task 21 的 grep 是第二侧）。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-budget
```

- [ ] **Step 4: 实现**

```rust
/// §333 的五个量纲。**单位与取值域规范未给**（§16 第 19 条）——本层不解释、不换算、不跨量纲比较。
/// 五个量纲**是 `pub` 的**：它是一份纯数据载体（同 `BudgetView` 的处置），
/// 而 Task 19 的「编译器强制穷尽的解构」要在**另一个 crate 的测试里**读这五个字段——
/// 字段私有则那条断言写不出来，而它正是裁定 §12.6.1 的全部防线。
pub struct Dimensions { pub money: Option<i64>, pub wall_time: Option<i64>, pub token: Option<i64>,
                        pub gpu_time: Option<i64>, pub network_transfer: Option<i64> }

/// 三个角色三个类型（照 D §6.3 的同一判据：上限与余量不能共用一个类型）。
/// **三者的内部字段一律私有、无公开构造函数**（§12.1）：只能经**分配方**、`reserve`、`remaining()` 得到。
/// 故「把余量填进上限的位置」在类型上不可写（`let _: Allocation = remaining;` 编译不过），
/// 且 crate 外**造不出**任何一枚（本 task 的六份 trybuild 样例钉的就是这两件事）。
/// **`pub` 的是 `Dimensions` 的五个字段**（它不是角色，没有可伪造的语义，且 Task 19 要在另一个
/// crate 里穷尽解构它）——**公开 `Dimensions` 的字段不削弱上面那条保证**（§12.1 已判过）。
pub struct Allocation(Dimensions);   // 声明式约束，分配方写
pub struct Reservation(Dimensions);  // 一次在飞的预扣，reserve 写
pub struct Remaining(Dimensions);    // 计算值：allocation − settled − reserved（逐维），remaining() 现算

impl Remaining {
    /// 五维的**公开读法**（Task 19 的穷尽解构与 §12.6 的恒等投影都要它）：
    /// 返回 `&Dimensions`，**不是**把三个角色之间的类型边打通——
    /// `Dimensions` 不是角色，读它不产生「余量当上限用」的可写位置。
    pub fn dims(&self) -> &Dimensions;
    /// §12.6 的投影逐维读法（`BudgetView { money: r.money(), … }`）。
    pub fn money(&self) -> Option<i64>;   // wall_time / token / gpu_time / network_transfer 同形，各一枚
}

/// I1：available ≥ 0，不满足则拒（fail-closed）。**返回具体哪一维不足**。
pub fn reserve(owner: BudgetOwner, dims: Dimensions) -> Result<Reservation, BudgetError>;

/// 结算。**`Over` 不写第二行账**——运行期超支的裁决不在本层。
/// **`actual` 的某一维为负 ⇒ `Err(NegativeActual { dimension, value })`——拒收，不截断**（§12.2）。
pub fn settle(r: Reservation, actual: Dimensions) -> Result<SettleOutcome, BudgetError>;

/// §12.2 的错误面。**本计划按设计三处的并集取三枚**（见本 task 的开工前置与 `## 遗留` 第一节第 5 条）。
pub enum BudgetError {
    /// 树上没有该 owner 的分配记录（`budget_node` 里查不到它）。**fail-closed**：不给 `Within`。
    UnknownOwner { owner: BudgetOwner },
    /// `reserve` 时某一维的可用额不足（§12.2 正文与 §14 的逐维五条照片靠它）。
    Insufficient { dimension: DimensionKind },
    /// `actual` 的某一维为负。**拒收，不按 0 截断**。
    NegativeActual { dimension: DimensionKind, value: i64 },
}

/// §12.2 的「有一条 `reserve` 而没有对应的 `settle` 或 `release`」——可检出。
/// **本函数是设计 §12.3 第 2 处的输入**（恢复钩子按预扣结算它）。
pub fn dangling_reservations(tx: &Tx<'_>) -> Result<Vec<DanglingReservation>, BudgetError>;

/// 迁移 120：`budget_node` / `budget_ledger`。
pub fn p4_budget_migrations() -> Vec<Migration>;
```

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-budget Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(budget): 预算树、三个角色的类型与 I1/I2/I3"
```

---

### Task 18: 无读数即按预扣（两处）、Budget Validator（§110）与探索预算（§334）

**Files:**
- Create: `crates/continuum-budget/src/{settle_policy.rs,validator.rs,exploration.rs,recovery.rs}`
- Create: `crates/continuum-budget/tests/{settle,validator,exploration,recovery}.rs`
- Modify: `crates/continuum-budget/src/lib.rs`

**Interfaces:**
- Consumes: Task 17 的全套；`continuum_persist::{RecoveryHook, RecoveryPhase, Tx}`
- Produces: `continuum_budget::{SettleOutcome, BudgetVerdict, ExplorationVerdict, ExplorationThresholds, BudgetReservationRecovery, check, check_exploration}`

- [ ] **Step 1: 写用例**

**无读数即按预扣结算**（§12.3 的一条规则、两处应用）：

- `a_streaming_call_without_a_reading_settles_at_the_reserved_amount`（第 1 处）：
  结算后 `settled == reserved` **且** `reserved == 0`（余量**不退**）。
- `a_dangling_reserve_is_settled_at_the_reserved_amount_by_the_recovery_hook`（第 2 处）：
  直接调 `BudgetReservationRecovery::run(tx)`，同样的两条断言；
  **并断言 `hook.phase()` 是 `RecoveryPhase::MarkLostExecutions`**（不是别的态）。
- `a_call_with_a_reading_settles_at_the_actual_amount`（**另一侧，不能省**）：
  有读数时 `settled == actual`、余量**退回**。**只钉「无读数按预扣」那侧会让「一律按预扣」的实现照样绿。**
  红条件：把 `settle` 实现成恒按预扣（**移除档**，本用例红）。
- `the_recovery_hook_does_not_repeat_what_mark_running_nodes_lost_did`：
  **否定式照片**——跑一次钩子后 `adfir_node` 表的**行内容逐行不变**（本层只读「哪些运行已 LOST」这个既成事实、
  只记账，**不写节点状态**；`MarkRunningNodesLost` 做的是把节点标 `LOST`，`continuum_graph::mark_running_nodes_lost`）。
  **这条判据的来历写进注释**：本层挂钩子的位置之所以是 `MarkLostExecutions` 而非
  `ReconcileRunningNodes`，正是为了**读得到**「LOST 已落库」这个事实（§13.1 的四步判据）。
- `the_hook_is_not_registered_on_reconcile_incomplete_effects`：签名层面 + 一条否定式断言——
  钩子的 `phase()` 不是 `ReconcileIncompleteEffects`。**依据**（写进注释）：那一阶段的钩子
  `MarkExecutingAsUnknown` 处理的是**效应**（把停在 `EXECUTING` 的外部效应标 `UNKNOWN`，
  `crates/continuum-effect/src/recovery.rs:47-49`），而悬空的**预算预留**不是一次效应——
  §12.3 的触发是「**这次执行没有读数**」，与「外部做没做成不知道」是两件事。

**Budget Validator（§110）**：

- `within_and_exceeds_each_have_a_photo`：`Within` 一条、`Exceeds` 一条。
- `an_unknown_owner_is_not_within`（**§12.4 第 3 条，fail-closed 侧**）：
  树上查不到该 owner 的分配记录 ⇒ `Err(BudgetError::UnknownOwner { owner })`，**不是 `Within`**。
  红条件：查不到就返回 `Within`（**放宽档**——那等于对一棵不存在的子树放行）。
  **并写明**：**超支不是 `Err`**，是 `BudgetVerdict::Exceeds`——§110 的流程要把「预计超支」
  当**一个可处置的结论**往下走（找替代方案 → 找不到则 Decision），做成 `Err` 会让调用方把它当失败而中止。
- `exceeds_reports_the_specific_dimension`：**逐维五条**，断言 `dimension` 是那一维、`by` 是那个数。
- `the_verdict_compares_against_remaining_not_allocation`（**本组件的要害**）：
  构造一个「**按 `allocation` 判会放行、按 `remaining` 判超支**」的用例——该 owner 已有一个兄弟节点占用了额度，
  故 `remaining < estimate ≤ allocation`；断言得 `Exceeds`。
  红条件：把判据换成 `allocation`（**取反档**，本用例红）。
  **依据**：`remaining` 已经把兄弟的占用算进去了，按 `allocation` 比会把 ENG-005 要的「一次全局判断」
  拆回「各层各自裁决」（§12.4）。
- `the_validator_does_not_seek_low_cost_alternatives`：**否定式照片**——公开面清单逐条：
  `check` **只返回裁决**，`src/` 里没有任何函数产出替代方案（§12.4：那是执行层 Planner 的动作）。
  **并明写**：「先寻找合法低成本方案」与「什么算找不到」的判定**不在本层**（§16 第 20 条）。

**探索预算（§334）**：

- `each_of_the_four_default_criteria_has_a_pass_photo_and_a_gate_photo`（**逐项，共八条**）：
  `wall_time ≤ 60s` / `money_cost = 0` / `external_side_effect = none` / 后两条的阈值——
  四个判据**各一条放行与各一条门控**，不抽代表。
- `the_external_effect_gate_carries_no_dimension`：`has_external_effect = true` 那一格
  `dimension` 为 **`None`**（因为「外部副作用」是**一个种类、不是一个量」——它的判定属边界层）。
  **枚举断言逐项**，**不靠「四格里三格顺带覆盖」**。
- `an_irreversible_action_is_gated_without_a_dimension`：`reversible = false` 同样 `dimension = None`
  （§334 的「可逆」未给量纲，故是必填入参）。
- `a_tightened_policy_gates_an_otherwise_allowed_action`：把 `wall_time` 阈值调小，同一个动作从
  `AutoAllowed` 变 `DecisionRequired { dimension: Some(WallTime) }`。
  **并明写**：四个阈值可由策略收紧，而**策略的来源（谁配、配在哪）规范未给**——故「策略真的收紧了」
  在本层**不可观察**，只有「给定阈值时的判定」可拍（设计 §15.2 第 10 条）。
- `the_two_thresholds_without_values_are_required_arguments`：§7 末两行的 `network_transfer <= threshold` 与
  `compute_cost <= threshold` **原文只写 `threshold`、没有数值**（§16 第 21 条）——故它们是**必填入参**，
  本层**不发明这两个数**（trybuild：无 `Default`）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-budget
```

- [ ] **Step 3: 实现**

```rust
/// §12.3 的一条规则、两处应用：**一次执行如果没有实际读数，就按预扣额结算；不退余量。**
/// 两处：流式调用（G §14 第 8 条）、悬空的预留（进程在预留与结算之间死掉）。
/// **理由一致**：无读数时「实际 ≤ 预扣」没有依据；按预扣结算**永不低记**（fail-closed），
/// 而释放余量是 fail-open（可能永久漏记一次真实消耗）。**代价写明**：连续多次无读数的执行会把额度用光，
/// 而实际可能没花那么多——这是一个**已知的保守偏差**，不是缺陷。
pub fn settle_without_a_reading(r: Reservation) -> (SettleOutcome, /* 记一条 settle */ ());

/// §13.1：挂在 `RecoveryPhase::MarkLostExecutions`。**不新增 phase**、
/// **不重复 `MarkRunningNodesLost` 已经做掉的那一半**（那一步写状态，本层记账）。
pub struct BudgetReservationRecovery;
impl RecoveryHook for BudgetReservationRecovery {
    fn phase(&self) -> RecoveryPhase { RecoveryPhase::MarkLostExecutions }
    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError>;   // 把悬空的 reserve 按预扣结算
}

/// §110：`check` 只判「超没超」，判据是**逐维比较 `estimate` 与 `remaining`**（不是 `allocation`）。
pub enum BudgetVerdict { Within, Exceeds { dimension: DimensionKind, by: i64 } }
pub fn check(estimate: &Dimensions, owner: BudgetOwner) -> Result<BudgetVerdict, BudgetError>;

/// §334 的落点。`dimension: None` = 因「不可逆 / 有外部副作用」而门控。
pub enum ExplorationVerdict { AutoAllowed, DecisionRequired { dimension: Option<DimensionKind> } }
/// 四个判据里有两个（`has_external_effect` / `reversible`）**不在 `Dimensions` 里**——必填入参。
pub fn check_exploration(dims: &Dimensions, thresholds: &ExplorationThresholds,
                         has_external_effect: bool, reversible: bool) -> ExplorationVerdict;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-budget
git commit -m "feat(budget): 无读数按预扣、Budget Validator 与探索预算"
```

---

### Task 19: 形状乙的两条断言（裁定 §12.6.1）——穷尽解构，无 `..`

> **本 task 只交付断言，不交付一个没有调用方的生产函数。** 裁定 §12.6.1 把 `Remaining → BudgetView` 的
> **生产投影**判给**驱动**，而驱动今天没有组装 `RoutingRequest` 的那一步（跨计划前置第四节已写）。
> 故本 task 的投影函数是**测试内的**，**不进生产面**——「不预先发明 API」（§2.3）在这一处的兑现方式。

**Files:**
- Create: `crates/continuum-runtime/tests/budget_projection.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`（dev 边 `continuum-budget`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（runtime 条目加 `continuum-budget`）

**Interfaces:**
- Consumes: `continuum_budget::{Remaining, DimensionKind}`、`continuum_model_registry::BudgetView`
- Produces: 无生产类型（**测试内的 `project` 一个**；**没有 `unproject`**，理由见 Step 3）

- [ ] **Step 1: 登记 dev 边并写用例**

`continuum-runtime` 的 `Cargo.toml` 加 **dev-dependency** `continuum-budget`（本 task 只用它跑测试；
Task 20 会把它升成普通依赖，理由是注册迁移与钩子）。`ALLOWED` 的 runtime 条目**同时加**它
（`cargo tree --edges all` 把 dev 边算进依赖，只加一处即红——`docs/superpowers/p3bcdf-followups.md` §四.1）。

- `remaining_and_budget_view_have_the_same_five_dimensions`（**裁定的全部防线**）：
  **照片机制写死为「编译器强制穷尽的解构／结构体字面量」，不用 serde；且 `..` 一个都不许写**——
  **两个方向都写**：

  ```rust
  // 第一行：BudgetView 侧穷尽解构（无 `..`）。任一侧加字段、或改一个维度名，即**编译不过**。
  let BudgetView { money, wall_time, token, gpu_time, network_transfer } = project(&r);
  // 第二行：Remaining 侧——**分两步**解构（三个角色的字段私有，§12.1 的 `let Remaining(dims) = r;`
  // 只在 **crate 内**成立；跨 crate 的读法是 `dims()`），两步都不带 `..`：
  let Dimensions { money: rm, wall_time: rw, token: rt, gpu_time: rg, network_transfer: rn } = r.dims();
  // 逐维比，不比较整个结构体 —— 故**不需要 PartialEq**（见判据 (e)）。五维各一行。
  assert_eq!(money, *rm);
  assert_eq!(wall_time, *rw);
  assert_eq!(token, *rt);
  assert_eq!(gpu_time, *rg);
  assert_eq!(network_transfer, *rn);
  ```

  **判据逐条写进注释**：
  （a）**`..` 一个都不许写**——带 `..` 的模式是**穷尽性豁免**，任一侧加字段它就照过，
  那时这条断言与手抄名单**没有区别**；
  （b）**不选 serde**：那条路要给 `BudgetView` 加 `#[derive(Serialize)]`，而它的模块文档
  （`crates/continuum-model-registry/src/budget.rs:36-37`）明写「没有当场消费方的派生在这里一律不加」
  ——**为一条测试去改被断言类型的公开形状，是把断言的成本转嫁给它**；
  而穷尽解构**不需要任何派生**，且把红的时点从**跑测试**提前到**编译**；
  （c）**「两侧字段名逐项相等」不是「数量相等」**——数量相等会让「`money` 换成 `cost`」漂过去；
  （d）**为什么要两步**：§12.1 写死「三个角色各自私有、无公开构造函数」，而 §12.6.1 的
  `let Remaining { money, … } = …` 是**按名解构一个 newtype 的字段**——**两者不能同真**
  （按名解构 newtype 的字段写不出来）。**本计划按 §12.1 办**：`Remaining` 侧经 `dims()` 读法取
  `&Dimensions` 再穷尽解构它。**跨 crate 的 `let Remaining(dims) = r;` 同样不成立**（字段私有），
  故 §12.1 那两行也只在本 crate 内是照片；**这一处已记 `## 遗留` 第一节第 3、4 条**；
  （e）**逐维比而不比整个结构体**（§12.6.1 已写死）：`assert_eq!(project(&unproject(&view)), view)`
  需要 `BudgetView: PartialEq`，**而本类型不加任何派生**——两处不能同真；逐维比对**不需要派生**，
  顺带把五个绑定都用上（否则 `unused_variables` 会报）。
  **红的时点**：任一侧加一个字段、或改一个维度名 ⇒ **编译不过**（记录实测的 `error[E0027]` 一类，
  **不许凭记忆写**）。**等价变异体警告**：把两个 `Option<i64>` 字段**互换**（如 `money` 与 `wall_time` 对调）
  在穷尽解构下**编译得过**——那正是下一条用例的作用，**故它不可省**。

  **「谁保证」逐格写清**（§12.6.1 已补这张表——**不能由「编译器」三个字替一个约定背书**）：

  | 保证 | 由谁 | 强度 |
  |---|---|---|
  | 全绑 ＋ `..` | 「编译器」**——但见下一行** | 名义上编译不过 |
  | **部分绑 ＋ `..`**（删掉一个绑定再加 `..`） | **不由编译器保证**（该 lint 只管「全绑了还写 `..`」） | **仅由站点注释 ＋ 复审守** |
  | 任一侧加字段（在两行都不带 `..` 的前提下） | **编译器**：模式不再穷尽 | 编译不过 |

  **本计划对第一行的订正要说准**：设计写「编译器（需 crate 级
  `#![deny(clippy::rest_pat_in_fully_bound_structs)]`）」，而**本仓的验证命令是 `cargo test` 与
  `cargo build`，不含 clippy**——`clippy::` 那条 tool lint 在 `cargo test` 下**不生效**，
  故第一格**实际也回落到「约定」**，与第二格同强。**本计划据此办两件事**：
  （i）把 `#![deny(clippy::rest_pat_in_fully_bound_structs)]` 写在 `tests/budget_projection.rs` 的文件头
  （**成本一行**；若将来把 clippy 纳入门禁，它立刻生效），并在注释里写明它**今天不生效**这件事实；
  （ii）**不建**那条「读这两行源码、断言不含 `..`」的自定义检查——设计 §16 第 34 条把它留给实现者，
  **本计划取「不建」**：它会引入一处读源码的测试，且本仓已有一处人工复核点（Task 21 Step 3 的源码面复核），
  把这两行加进那张清单比再造一个自指的断言便宜。**逐条见 `## 遗留` 第五节第 34 条。**
- `the_projection_is_value_preserving_dimension_by_dimension`：
  **五个量纲各一条**，逐维断言投影后的值与 `remaining` 相等，**含 `None` 那一侧**（`None → None`，
  **不是 `Some(0)`**）。
  **照片机制（须写死）**：**不许对结构体写 `assert_eq!`**——`BudgetView` **不派生 `PartialEq`**
  （`budget.rs:38-44` 无 `#[derive]`），`assert_eq!(project(&unproject(&view)), view)` **编译不过**。
  故往返断言**逐维比较五个 `Option<i64>`**（每维一次 `assert_eq!`，比较的是字段值不是结构体）。
  **这一处是设计 §12.6.1 的代码块与本仓既有代码的相抵**（设计写了那个 `assert_eq!`），
  **本计划按模块文档办**——这一条**已由设计闭合**（§12.6.1 改为逐维比，见 `## 遗留` 第一节的已闭清单）。
  红条件：把 `Remaining` 的 `money` 投影到 `BudgetView` 的 `wall_time`（**取反档**——逐维值断言红）。
- `none_is_not_some_zero_on_both_sides`（**两侧各一条，合起来才算钉住**）：
  （a）本层 `Remaining` 的 `None` 表示「该维不构成约束」——`None` 读回仍是 `None`；
  （b）`BudgetView` 的 `None` 同义（D 已在 `crates/continuum-model-registry/src/budget.rs:31-33` 写明）
  ——构造 `BudgetView { money: None, …, }` 与 `{ money: Some(0), … }` 两个值，断言两者**不被当成同一件事**
  （逐维比较 `is_none()` / `== Some(0)`）。
  **只钉本层那侧，D 侧把 `None` 当 `Some(0)` 仍会绿**——这是裁定点名的两侧。
  转换侧另加一条：**`None` 投影成 `None`，不是 `Some(0)`**（含在上面五维各一条里）。
  红条件：把某维的 `None` 折成 `Some(0)`（**移除档**）。
  **等价变异体警告**：若夹具里 `None` 与 `Some(0)` 恰好输出相同（例如只断言「不 panic」），
  这条就是等价变异体——故两个值必须让断言结果不同。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test budget_projection
```

- [ ] **Step 3: 实现（测试内的恒等投影）**

```rust
/// **测试内的**恒等投影——生产投影由驱动的接线 task 写（裁定 §12.6.1），本 plan 不预先发明它。
/// 本函数的用途只有一个：让「两侧字段集逐项相同」这件事**可断言**，从而让漂移变成编译错误。
fn project(r: &Remaining) -> BudgetView;
```

**没有 `unproject`，而这是设计的一处必然后果**：§12.1 写死三个角色**各自私有、无公开构造函数**，
故**在另一个 crate 的测试里造不出一个 `Remaining`**——「反向映射」在类型上不可写。
于是「两个方向都写」这句话的**可落地形态**是：`BudgetView` 侧穷尽解构一次（`project` 的返回值），
`Dimensions` 侧穷尽解构一次（经 `r.dims()`，`r` 由设计指定的入口得到），**两处都不带 `..`**。
**这不是把设计要求做小了**：两处解构各自都是编译器强制的穷尽性检查，
而「反向造一个 `Remaining`」正是 Task 17 那三份反例要挡掉的东西——
**若这里能写出 `unproject`，那三份反例就有一条会通过**。这一句要写进注释。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/tests/budget_projection.rs crates/continuum-runtime/Cargo.toml \
        crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "test(runtime): 形状乙的两条断言——五维字段集与 None ≠ Some(0)"
```

---

### Task 20: 装配——三处迁移注册、`RecoveryHook` 挂 `MarkLostExecutions`、`ALLOWED` 与三处计数

> **硬前置：D 与 G 已合入**（见跨计划前置第二节）。本 task 改的六个文件里有五个是 P3 的多写者单文件。

**Files:**
- Modify: `crates/continuum-runtime/Cargo.toml`（`continuum-budget` 由 dev 升为普通依赖；加 `continuum-canonical`、`continuum-semantics`）
- Modify: `crates/continuum-runtime/src/main.rs`（`runtime_migrations()` 追加三行）
- Modify: `crates/continuum-runtime/src/recover_cmd.rs`（注册预算钩子）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（runtime 条目）
- Modify: `crates/continuum-runtime/tests/migrations.rs`（`expected_migrations()` 同步三行）
- Modify: `crates/continuum-runtime/tests/startup.rs`（计数与 `MarkLostExecutions` 钩子行）

- [ ] **Step 1: 注册三个迁移与钩子**

`runtime_migrations()` 在 `continuum_model_registry::p3d_model_migrations()` 之后追加三行
（**追加在末尾，不改既有各行的次序**）：

```rust
    migrations.extend(continuum_canonical::p4_canonical_migrations());   // 100：alias
    migrations.extend(continuum_semantics::p4_semantics_migrations());   // 110：七张表
    migrations.extend(continuum_budget::p4_budget_migrations());         // 120：树与账本
```

`recover_cmd::run` 的注册块加第三个钩子（**唯一注册点是这里，不是 `main.rs`**）：

```rust
    let budget = continuum_budget::BudgetReservationRecovery;
    registry.register(Box::new(budget));
```

**注册块上方的模块文档要一并改**（它今天写着「注册两个钩子，各占 §319 的一个阶段」，
并把三个阶段逐条点名）——本 task 把它改成三个，并写明 `MarkLostExecutions` 这一态**此前没有任何钩子注册**
（`MarkRunningNodesLost` 的文档自称覆盖两个阶段、却只注册在 `ReconcileRunningNodes` 上，
`crates/continuum-runtime/src/recovery.rs:8-9` 与 `:27-29`）。
**`RecoveryPhase` 五态一个不动、不新增 phase。**

- [ ] **Step 2: 登记 `ALLOWED` 与三处计数**

`ALLOWED` 的 runtime 条目加 `continuum-budget`（Task 19 已加）、`continuum-canonical`、`continuum-semantics`
（**数组按字母序**，与本表其余条目同形）。**理由写进注释**：装配处注册迁移与钩子。

`tests/migrations.rs` 的 `expected_migrations()` 同步三行（与 `main.rs` 逐行同形），并更新它的文档注释。

`tests/startup.rs` 的「迁移应用 N 项」**按现场实跑读出的数字改**（`tests/migrations.rs:1-11` 已写明
计数断言的限度：数从 6 变 7 时把它改成 6 同样能过，故**它只是第二道**，第一道是本文件与 `main.rs` 的**集合比对**）。
**预期区间**：现 9 ＋ 本层 3 ＝ **12**；**若 P1 的迁移 21（`adfir_graph.contract_version`）已落地则为 13**。
**不许预判，以实跑为准**。

- [ ] **Step 3: 写用例**

- `the_runtime_applies_exactly_the_expected_migration_set`（既有用例，本 task 让它覆盖三张新表）：
  **集合比对**（不是计数）——`main.rs` 少注册或多注册一条都会红。
- `the_mark_lost_executions_phase_now_has_exactly_one_hook`（**新增一条断言**）：
  `recover` 的输出里 `mark lost executions: 1 钩子` 那一行。**这是「§319 点了名、却没有任何钩子注册」
  的那一态被补上的唯一可观察证据**（§13.1 的实测）。
  红条件：把钩子注册到 `ReconcileRunningNodes`（**取反档**）→ 该行变 `mark lost executions: 0 钩子`
  且 `reconcile running nodes: 2 钩子`。
- `the_three_new_tables_are_created_by_the_runtime_binary`：跑一次 `recover --db <新库>`，裸查
  `alias` / `intent` / `budget_node` 三张表各能读到 0 行（**不是「表存在」**——存在但读不到的路径是另一回事）。
- `an_alias_scope_out_of_table_still_reads_back_as_an_err_through_the_runtime_path`：
  **端到端一条**——经真实启动的库写一条表外 scope，断言读回是具体 `Err`（把 Task 7 的纪律在**生产链**上再钉一次）。

- [ ] **Step 4: 运行，确认失败 → 实现 → 全量**

`startup.rs` 的既有输出形式**不新增行**（它是 `tests/migrations.rs` 与 `tests/startup.rs` 的断言对象，
改打印会同时波及两处）——第三阶段的那一行**本来就会打印**（`for phase in &report.phases`），
故本 task 只改**断言**，不改打印。

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-runtime Cargo.lock
git commit -m "feat(runtime): 装配 P4 的三个迁移与 MarkLostExecutions 的预算恢复钩子"
```

---

### Task 21: 收尾与复核

**Files:**
- Modify: 三个新 crate 与 `crates/continuum-runtime/**`（**仅在复核发现缺口时**）

- [ ] **Step 1: 全量验证**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：全绿、0 warning。

- [ ] **Step 2: 三个 crate 的「不依赖下层」机检**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：`every_crate_depends_only_on_its_allowed_set` 全绿。**逐条核三件事**：
（a）三个新 crate 的 `ALLOWED` 条目只含 `continuum-persist` 与层内前驱，**没有任何指向
执行层／资源层／边界层的边**；（b）`continuum-semantics` 的条目里**没有 `continuum-budget`**
（§11.5 的第 1 条守卫），`continuum-budget` 的条目里**有 `continuum-semantics`**（它要 `IntentId`）；
（c）三处 `Cargo.toml` 与 `ALLOWED` **精确一致**（逐对 `assert_eq!` 已覆盖）。
**并把粒度写清**：`--depth 1` 只钉**直接边**；传递可达在 Rust 下不可 `use`，**故直接边即这条规则的完整粒度**，
**不另设闭包断言**。**不要把这句读成「任何指向 graph 的边都会变红」**——那会是一条假保证（设计 §1.2.2 规则 1 末段）。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
cd /home/DslsDZC/Continuum
grep -rn "continuum_core\|continuum_core::" crates/continuum-canonical/src crates/continuum-semantics/src crates/continuum-budget/src
grep -rn "ToolId\|ModelId\|ConnectorId\|Usage" crates/continuum-canonical/src crates/continuum-semantics/src crates/continuum-budget/src
grep -rn "continuum_graph\|continuum-graph\|propagate_invalidation" crates/continuum-canonical/src crates/continuum-semantics/src crates/continuum-budget/src
grep -rn "continuum_capability\|continuum_policy\|continuum_model_registry\|continuum_connector\|continuum_provider" crates/continuum-canonical/src crates/continuum-semantics/src crates/continuum-budget/src
grep -rn "SystemTime::now\|Instant::now" crates/continuum-canonical/src crates/continuum-semantics/src crates/continuum-budget/src
```

**另加一条人工复核（不是 grep）**：打开 `crates/continuum-runtime/tests/budget_projection.rs`，
**逐行确认那两处解构一个字都不含 `..`**（§16 第 34 条与设计 §12.6.1 的「谁保证」表：
「部分绑 ＋ `..`」那一格**没有机器判据**，本计划明写不建那条读源码的检查）。
**复核结论写进报告，写清是哪两行、当时是什么内容。**

**五条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；
**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：
（a）`continuum_core` **零 `use`**——注意「不 `use`」讲的是依赖边为零，**不是整篇不出现**
（设计 §12.3 与 §13.4 点名 `Usage` 并以它立论，本计划允许注释里出现它）；
（b）四个 id 类型零 `use`；（c）图相关零命中；（d）其余五个层外 crate 零命中；
（e）系统时钟零命中（**`Now` 一律入参**，本层不取时钟）。
**作用域写死**：这三条 grep 只扫 `src/`，**不含测试目录**（测试夹具允许构造外部值）。

- [ ] **Step 4: 逐条核对 §2.4 的六条完成判据**

| §2.4 的判据（`docs/02-工程.md:130-135`） | 证据 |
|---|---|
| 含错别字／简称／中英混写／代词的输入被规范化，或被明确标记为 Unknown / Ambiguous | Task 2 的 `four_kinds_of_dirty_input_each_yield_one_of_the_two_arms` ＋ `Canonicalized` 的两支互斥 |
| 不存在正确候选时系统不强行绑定（§273） | Task 5 的四条断言（A/B/C/D，B 逐候选、D 两条）＋ 单侧守卫的反面 |
| 带版本的 Specification，且 `unknowns` 显式保留 | Task 10 的 `none_found_and_an_empty_enumerated_are_distinguishable…` ＋ `a_draft_without_unknowns_cannot_be_finalized` ＋ Task 8 的版本语义 |
| 带版本的 Contract，每条 Requirement 带 `class` 与 `verification_requirement` | Task 11 的四 class 逐项 ＋ 方法集逐条 ＋ 主键拒二次写 |
| **ContractDiff 能正确判定哪些已有节点受影响** | **本层只做到一半**：§226 四类的判定在 Task 12；「哪些节点受影响」的判定在**执行层**、**今天无消费者**（§16 第 13 条）。**据实标注为未完整成立** |
| 违反 REQUIRED 的候选计划被拦截并生成 Decision | Task 16 的 `a_violation_of_a_required_requirement_is_blocked` ＋ Task 15 的两个触发点（同一 crate 的两步，不是两个 crate） |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑三 crate 的**承重守卫**（两侧对钉的、fail-open 侧的、失败路径判别 `Err` 的、跨 crate 才可见的），
每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 → 还原后 `sha256sum` → 独立日志路径。
**逐轮在报告里附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位（取反／放宽／收紧／移除）」**。
**三条跨 crate 的承重守卫必须跑全量**：迁移集合与计数、`ALLOWED` 与实际依赖一致、
`remaining_and_budget_view_have_the_same_five_dimensions`（**这一条的变异是「加一个字段」，红的时点应在编译期——
若它跑成了「编译失败」而不是「用例失败」，按纪律 1(c) 记为「编译不过」，**并另换一个真变异体**：
把某维投影到另一维（运行期红）**）。

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。**本计划不新建、也不修改第二份文档**：
若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 7: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs(semantics): P4 语义层的收尾与复核"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**
设计 §16 的 **34** 条**本计划一条都不发明**——收件人照原文，逐条落在下节第五节
（第 34 条的收件人是**实现者**，本计划已就它作出取舍，见第五节）。

### 一、设计问题：**已由设计闭合的七条** ＋ **仍开着的六条**（发现即报，本计划一处都不替设计补写）

**已闭合的七条（留格是为了让「一处说法被订正」可查；闭合处逐条点名，免得后来者按旧报告去找）**：

- **§4 没有「显式约束集」的类型与「满足」判据** —— **已闭**：设计新增 **§4.1.1**
  （`ExplicitConstraint` 三枚 `Scope`/`EntityKind`/`BoundTo`、`ResolutionScope` 九值、`EntityKind` 四枚、
  `CandidateFacts`、`satisfies`、`qualifying`、**合取**判据），并在 §4.1.1 开头写明「本节先前没有给出它们」、
  出处记为本计划作者。**Task 5 的开工前置因此撤销**（`## 遗留` 原文的「未补则停下报」不再适用）。
- **`ResolutionResult` 被点名却从未给出字段清单** —— **已闭**：设计 §4.1 补出四支的**载荷类型**
  （`Resolved` / `ConfirmRequired` / `Ambiguous` / `Unknown`）与各自字段（含 `candidate_count`）、
  以及 `UnknownReason` 的两枚。
- **§4.1 说四态「一律 `Ok`」而全节无 `Err` 支** —— **已闭**：设计 §4.1 写死「返回 `ResolutionResult`，无 `Err` 支」，
  并注明与本计划取了同一结论、理由一致。
- **§12.6.1 的 `assert_eq!(project(&unproject(&view)), view)` 与 `BudgetView` 无 `PartialEq` 相抵** —— **已闭**：
  设计 §12.6.1 改为**逐个绑定逐维比**（五行 `assert_eq!`），并写明「不需要任何派生」。
- **§4.2 照片 D-2 里 `c₃` 不承重** —— **已闭**：设计 §4.2 与 §14 都写成**两条断言**
  （结果是 `Ambiguous` ＋ `surviving` 恰是 `{c₁,c₂}` 且不含 `c₃`），并写明「第二条才让 `c₃` 承重」、
  出处记为本计划作者。Task 5 已照此写。
- **`ExplicitBinding` 指向候选集外时给哪一态未给** —— **已闭**：设计 §4.6 写死「**不新增分支，把该实体注入工作集**」
  并给了两侧照片；断言 A 随之重述为「检索集 ∪ 注入的那一枚」，并补了**注入守卫**照片。Task 5／6 已照此写。
- **§12.4 的 `check` 没有 `Err` 判据** —— **已闭**：设计 §12.4 补出三条（超支不是 `Err`；
  量纲不是 `Err`；`UnknownOwner` fail-closed）。Task 18 已照此补了 `an_unknown_owner_is_not_within`。

**仍开着的六条**：

1. **`Remaining` 的 crate 归属相抵**（**两处相抵，仍未闭合**）：§2.1 的组件表、§2.2 的依赖边表与 §12.1 都把
   `Remaining` 放在 **`continuum-budget`**；而 §12.6.1、裁定文件
   （`docs/superpowers/2026-10-06-p4-decisions.md:24`、`:19-21`）与**设计 :1843**（「它同时要
   `continuum_semantics::Remaining` 与 `continuum_model_registry::BudgetView`」）的字面写「在 `continuum-semantics`」。
   **两处不能同真。** **本计划的取舍**：按 §2.1／§2.2／§12.1（**结构专节 ＋ 依赖边表**）落在
   **`continuum-budget`**（Task 17／19）。**理由**：预算树与账本整块在 `continuum-budget`
   （§2.1 的组件表那一行就写着「`Allocation` / `Reservation` / `Remaining`」），
   而 §12.6.1 的 :1843 只说「再加 `continuum-semantics` 是合法方向」——它讲的是**边**不是**归属**。
   **翻转条件**：若协调者按字面裁定在 `continuum-semantics`，则 Task 17／19 的文件位置与两处 `ALLOWED` 条目一并改，
   **而 §2.1／§2.2 也要改**（否则同一处还有第二遍相抵）。**收件人：设计作者 ＋ 协调者。**
2. **§12.2 的 `I2` 与 ENG-005 原句的措辞差别**（**设计已自陈并已给收件人**）：原文说「兄弟的**活跃预留**之和
   ≤ 父的剩余」，本设计判「兄弟的**分配额**之和 ≤ 父的可用额」（更强的一条）。
   **本计划照 §12.2 取「分配是上限、不转移」**，**不代裁**——若协调者判「分配即转移」，
   `I2` 换成 `Σ_{执行中的子节点} reserved(child, d) ≤ allocation(n, d) − settled(n, d)`（§12.2 已给）。
   **收件人：协调者。**
3. **§12.1 的「三个 newtype 各自私有、无公开构造函数」与 §12.6.1 的按名解构相抵**（**部分闭合**）：
   §12.1 已把解构写死为**两步**（并补了「`pub` 字段会不会削弱保证」之判，结论是不会），
   **而 §12.6.1 的代码块 :1821 仍是单步按名解构** `let Remaining { money, … } = unproject(&v);`
   ——那一行**在 crate 外写不出来**（`Remaining` 是元组 newtype 且字段私有）。
   **本计划按 §12.1 办**：Task 19 经 **`dims()` 读法**取 `&Dimensions` 再穷尽解构它（**两步都不带 `..`**）。
   **收件人：设计作者 ＋ 复审者**（要改的是 §12.6.1 的 :1819-1821 三行）。
4. **`let Remaining(dims) = r;` 的作用域没写明**（**没写清**）：§12.1 的 :1549 那行在**本 crate 内**成立
   （私有字段可解构），**跨 crate 不成立**，而 §12.6.1 的两条断言是**住在驱动里**的（设计 :1843-1845 自己写的）。
   两处合起来看，§12.1 的那行示范会被读成「跨 crate 可用」。**本计划的落点**：三个角色各给一个**公开读法**
   （`dims()` 与五个逐维读者），跨 crate 的穷尽解构走它；**这是本计划自定的一处 API 形状**
   （设计只给了 `r.money()` 一类逐维读者，没给 `dims()`），记在第四节。
   **收件人：设计作者**（§12.1 的那行要不要注明作用域）。
5. **`BudgetError` 的变体在三处不一致，且 `reserve` 的「哪一维不足」没有变体承载**（**没写清 ＋ 判据缺口**）：
   §12.2 的**正文** :1569 写 `reserve(...) -> Result<Reservation, BudgetError::Insufficient { dimension }>`、
   §14 的表格 :2124 也要求「`reserve` 返回**具体哪一维**（逐维五条）」；而 §12.2 的**枚举** :1588-1593
   只列 `UnknownOwner` ＋ `NegativeActual`（**没有 `Insufficient`**），§12.4 的**枚举** :1669-1672
   只列 `UnknownOwner` 并写「`BudgetError` 只收**一种**」。**三处不能同真。**
   **本计划的取舍**：按三处的**并集**取三枚（`UnknownOwner` / `Insufficient` / `NegativeActual`），
   逐枚注明出处；**不删 `Insufficient`**——「返回具体哪一维」是 §12.2 正文与 §14 表格都写着的判据，
   删了它那五条照片没有承载。已写在 Task 17 的开工前置里。
   **翻转条件**：若裁定「`reserve` 的失败不区分维度」，则 §12.2 正文与 §14 表格要一并改，
   而那会掉两条判据（ENG-005「没有预留，§110 就没有判据」那一行的可观察后果）。
   **收件人：设计作者 ＋ 复审者。**
6. **§16 第 34 条说「编译器保证」的那一格，在本仓的验证命令下不成立**（**新增，本计划查出**）：
   设计 §12.6.1 的「谁保证」表第 1 行写「全绑 ＋ `..` —— **编译器**（需 crate 级
   `#![deny(clippy::rest_pat_in_fully_bound_structs)]`）｜编译不过」。
   而 **本仓跑的是 `cargo test` ＋ `cargo build`，不含 clippy**（Global Constraints 的验收命令即此），
   且 `clippy::` 那条 tool lint 在 `cargo test` 下**不生效**——故那一格**实际也回落到「约定」**，
   与第 2 行同强。**本计划据此办两件事**（Task 19 已写）：把 `#![deny(clippy::rest_pat_in_fully_bound_structs)]`
   写在测试文件头并在注释里写明它**今天不生效**；**不建**那条读源码的自定义检查（理由见第五节第 34 条）。
   **收件人：设计作者 ＋ 复审者**（若要把这一格变成真保证，缺的是「把 clippy 纳入门禁」这一步，
   那是仓库级决定，不在本计划范围）。

### 二、对既有之物的三处请求（**本计划一处都不实施**，逐条见跨计划前置第三节）

- **`adfir_graph` 加 `contract_version`**：**新增迁移 21、必须带 `DEFAULT`、不得改迁移 20 的 SQL**；
  `DEFAULT 0` 是「图建立时版本未知」的哨兵，**读取侧须能把 0 与任何真版本区分开**。
  **本层是消费方**（定义 `ContractRef`），**不读该列**。**收件人：执行层（P1）＋ 协调者。**
- **`ContractIdRef` 与 `ContractId` 的合一与否**：生产点三处（`AdfirGraph::new`、`load_graph` 的读回路径、
  各测试）——**「已有库不受影响」不成立**，已有库里的行会被读回。**本层是依赖方**（只定义 `ContractId`）。
  **收件人：执行层（P1）＋ 协调者。**
- **`ExecutionProfile.cost_budget` 收成 `Allocation`**：类型本层交付（Task 17），
  **收紧的动作归执行层的接线 task**（列在 `crates/continuum-graph/src/persist.rs:60`）。
  这是对 **ENG-005 §五**（`…eng-005-budget-accounting.md:111` 判给「D 落地时」）的**一处改判**，
  判据是那列属执行层的表、而 D 的设计 §11 第 12 条自陈未做。**收件人：执行层的接线 task ＋ 协调者。**

### 三、拍不到的照片（设计 §15.2 已列 10 条，**逐条照原文，本计划不发明**）；本计划补三条

11. **`Remaining` 五维的 `None` 语义在消费端不被抹平**：本层这侧有照片（Task 17／19），
    **消费端（子项目 G）那侧没有**——与 D 的 `## 遗留` 里「`BudgetView` 的 `None` ≠ `Some(0)` 谁守」同一条缝的两端。
    **收件人：驱动的投影实现（本项目尚无该 task）与子项目 G。**
12. **`Mention.kind` 的取值域**（§16 第 3 条）：**没有判据**——故只能拍「表外取值往返」，
    拍不出「该取值对不对」。**收件人：规范维护者。**
13. **三个 crate 对下层的零依赖**：**由 `ALLOWED` 钉直接边**，**传递可达不可拍**——
    而 Rust 下也未写进 `Cargo.toml` 的传递依赖不可 `use`，**故不需要那条断言**（设计 §1.2.2 规则 1 末段）。

### 四、本计划自定的形状与取值（**申报**，逐条说清代价）

- **迁移号 100／110／120 取用前现场核对空号**（不假定）；两处干扰项（`60` 在 crash-writer 与 crash 测试、
  `100` 在 `tests/migrations.rs` 的 `layer_table`）**都不是生产链上的占用**。
- **迁移 110 的 SQL 逐表追加**（Task 8／10／11／13／15）：**Task 20 之后不得再改它的 SQL**
  （那时它已进生产链，改它对已建库无效——设计 §13.2 第 2 条）。
- **`dangling_reservations` 的取名与落点**（Task 17）：设计 §12.2 只写「是可检出的」，**没给函数名**；
  而 §12.3 第 2 处的恢复钩子**必须**消费它，故这个读法是必需的。**代价**：名字是本计划定的。
- **`PlanChange` 分类的阈值入参**（Task 14）：§230 的「成本显著扩大」未给阈值（§16 第 15 条），
  故若无一个显式阈值入参，本层只能判「两者不相等即 MATERIAL」——**那句话在规范里没有依据**。
  本计划取「阈值以入参进入、本层不发明数值」。**代价**：多一个入参；**收益**：不发明判据。
- **`ResolutionResult` 的载荷形状与 `resolve` 不含 `Result`**：**不再是本计划自定** ——
  设计 §4.1 已把两者写死（四支载荷类型、`candidate_count`、`UnknownReason` 两枚、无 `Err` 支），
  Task 4 照它写。留这一条是为了让「这两处曾由计划先取、随后被设计采纳」可查。
- **三个角色的公开读法 `dims()`**（Task 17）：设计给了逐维读者（`r.money()` 一类）而**没给 `dims()`**，
  而跨 crate 的穷尽解构要一个能拿到 `&Dimensions` 的读法（三个角色字段私有，见第一节第 3、4 条）。
  **代价**：多一个公开读者；**收益**：不必把角色的字段开成 `pub`（那会与 §12.1 的私有承诺相抵）。
- **`#![deny(clippy::rest_pat_in_fully_bound_structs)]` 写进测试文件头**（Task 19）：
  本仓无 clippy 门禁，故它**今天不生效**；写它是为了让「将来纳入 clippy 时立刻生效」这件事**不依赖记忆**。
  **不建**那条读源码的检查，理由见第五节第 34 条。
- **形状乙的两条断言落在 `crates/continuum-runtime/tests/`**（Task 19），**生产投影函数本计划不建**
  （裁定判给驱动，而驱动的那一步今天不存在）。**代价**：投影的**生产**实现仍无人写，
  本 task 只交付裁定要的那条断言（「有断言的恒等转换与没有断言的恒等转换是两回事」）。
  **收件人：接线的那个 task。**

### 五、设计 §16 的 **34** 条：本计划一条都不发明，收件人照原文（**第 34 条例外：它的收件人是实现者，本计划已取用**）

设计 §16 的 34 条**逐条以「收件人」结尾**，本计划**不重述其内容**
（重述就是第二份转录，正是本项目出错最多之处）。

**第 34 条是唯一一条收件人写着「实现者」的**，本计划的取用写在 Task 19 与第一节第 6 条：
它要的那条自定义检查（在测试里读那两行源码、断言不含 `..`）**本计划不建**，
理由是它会引入一处**读源码的测试**、而本仓已有一处人工复核点（Task 21 Step 3 的源码面复核）——
把「那两行不得带 `..`」加进那张清单，比再造一个自指的断言便宜。
**代价写明**：于是「部分绑 ＋ `..`」那一格**仍只由站点注释与复审守**，没有机器判据；
若复审判定必须机器化，缺的就是那条读源码的检查（它的落点已被设计点名：读哪个文件的哪两行）。
**逐条照收件人归类**（**为让「核过」可查**，与 G 的 §13.2 末行同一做法）：

- **收件人：规范维护者**（本项目无此角色）——第 **1、3、4、6、7、8、10、11、12、14、15、16、17、19、20、21、32、33** 条。
  **本计划一条都不发明**；其中第 12 条的 `class` 四值序与第 32 条已由 Task 12 的照片与本计划第一节的已闭清单（`c₃` 承重）与第 2 条（`I2`）覆盖到可断言的部分。
- **收件人：执行层（P1）**——第 **2**（`Node.constraints` 的词汇表与注记）、**24**（`contract_version` 与 `ContractId`）、
  **25**（`cost_budget` 收成 `Allocation`）条。**第 24／25 条即本文第二节那三处请求。**
- **收件人：执行器（执行层，未建）＋ 协调者**——第 **13** 条（ContractDiff 今天没有消费者）。
- **收件人：第 7 层（长期循环，未建）**——第 **18** 条（`SystemMaintenance` 根的创建方）。
- **收件人：跨领域层（P5 或后续）**——第 **9** 条（`completion_predicate` 的判定）。
- **收件人：P2 的设计 ＋ 协调者**——第 **23** 条（`PolicyContext.explicit_current` 与 §8.2 第 2 级不是同一件事；
  该情形的照片要等两件事同时到位：本层落地 ＋ P2 给第 2 级一个承载位置）。**本层不擅自改 P2 的类型。**
- **收件人：《工程》文档维护者**——第 **30** 条（ENG-009 已由本层闭合，`docs/02-工程.md` §10.2 那一行待订正）。
  **本计划不改 `docs/02-工程.md`。**
- **收件人：复审者**——第 **22** 条（形状乙裁定的翻转条件与两条残留收件人）、第 **31** 条
  （`Decision` 与 `continuum_policy::Decision` 同名：本层用 `Decision`、跨界模块把策略的那个别名成 `PolicyDecision`；
  **这是命名约定、不是类型守卫，故没有照片**——更强的做法是重命名本层的类型，那会偏离 §331 的名字）。
- **收件人：子项目 B 的实现／规范维护者**——第 **26、27** 条（`p3bcdf-followups.md` §八.19 与 §三 的处置：
  **接一半、退一半**，退件的判据是「本层没有可挂的触发条件」）。**本层一条都不认领工具侧的词汇。**
- **第 5、28、29 条**——第 5 条（§197 的七种来源今天零个）的收件人是**实体注册表的持有方（P3 A/B/C/D）＋ §2.1 的组件表**；
  第 28 条是「核过」的留痕（**§八 的其余 24 条收件人无一条是语义层**）；第 29 条的两个规范面仍开、
  收件人是 **C 的设计 ＋ 规范维护者**。**三条本计划都不接。**
