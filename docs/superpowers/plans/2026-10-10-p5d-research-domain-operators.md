# P5d（研究领域算子）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新建 `crates/continuum-research`，把规范 §330 的七步链（`Question` → `Search` → `SourceSet` →
`EvidenceExtraction` → `ContradictionCheck` → `Synthesis` → `CitationVerification`）落成**七枚可注册的 `Operator`**，
并提供两个入口——一枚把它们注册进 `OperatorRegistry`（先核后写、不留半注册），
一枚把 §187 `research/` 四条方法的 `realized_by` 经 P5b 的 `MethodRegistry::bind` 填出。

**Architecture:** 本块**零类型、零判定、零迁移**（设计 §2.2、§8）。
`pub` 面只有三枚项，逐枚列在下面，**无一项的类型由本块定义**：

```
pub fn all_operators() -> [Operator; 7];                                                  // 设计 §4.1
pub fn register_research_operators(registry: &mut OperatorRegistry, operators: &[Operator])
    -> Result<(), OperatorError>;                                                         // 设计 §4.1
pub fn bind_research_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;   // 设计 §7
```

前两枚的类型全在 `continuum-operator`，第三枚的类型全在 `continuum-method`。
本块**不新造 `ArtifactType` 变体、不报出任何变体、不新造 `EvidenceType` 臂、不写证据包装函数、不建表**。
七枚算子里六枚经模型或检索、一枚是纯算法；七枚的 `side_effect_class` 一律 `Pure`
（口径与依据见设计 §4.4 第 3 条，**本节是那一口径的家**）。

**不变量（本计划的承重面）**：清单的顺序即 §4.2 的行序；七枚的 `id` 两两不同；`version` 均为 1；
`research-source-set` 是七枚里**唯一**的 `Deterministic`，其 `backend_candidates` **恰为** `["builtin"]` 一枚；
§4.5 的边表逐行成立；`register_research_operators` **先核后写**（任一枚不成即整个批次不写一个字）。

**Tech Stack:** Rust 1.95.0 / edition 2024。普通依赖 `continuum-artifact`（`ArtifactType`）、
`continuum-operator`（`Operator` 及其七字段的类型与注册表）、`continuum-method`（P5b 新建）；
dev 依赖 `continuum-graph`（§6.1 的照片：`reuse::{cache_key, can_reuse}`）、
`continuum-verify`（§9.4 的 `trybuild` 样例）与 `trybuild`。**无 `thiserror`、无 `serde`、
无 `continuum-persist`**——本块没有自己的错误类型、零表、不做 I/O。

**设计依据：** `docs/superpowers/specs/2026-10-09-p5d-research-domain-operators-design.md`（**唯一事实来源**，930 行）。
**切分与冻结口径：** `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`（§一、§二、§四、§五、§八）。
**本计划不改设计、不改规范、不改任何别的块的计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-10 实读工作树 `p5d-plan`）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| P1 `continuum_operator::{Operator, OperatorId, OperatorVersion, BackendId, Determinism, SideEffectClass}`（`crates/continuum-operator/src/definition.rs:9`、`:17`、`:24`、`:45`、`:66`、`:79-88`） | **已交付** | 七枚算子的七个字段与 `all_operators()` 的全部类型 |
| P1 `continuum_operator::{OperatorRegistry, OperatorError}`（`src/registry.rs:6-12`、`:14-48`） | **已交付** | Task 2 的登记面；`OperatorError::Duplicate`（`:11`）；**无迭代面**（`entries` 私有，`:16`） |
| P1 `continuum_artifact::ArtifactType`（`crates/continuum-artifact/src/artifact.rs:14-21` 六臂、`:28` 的 `ALL`、`:47` 的 `as_str`、`:68` 的 `parse`） | **已交付（六臂）** | 七枚算子的端口类型；**`Report` 不在内**（见下） |
| P1 `continuum_graph::reuse::{CacheKey, cache_key, can_reuse}`（`crates/continuum-graph/src/reuse.rs:9-12`、`:16`、`:29`） | **已交付** | Task 5 的 (k) 条（**dev 边**） |
| P5a `continuum-verify`（`Evidence`、`EvidenceType`、`VerificationPolicy`、`VerifyError`…） | **未交付**：`ls crates/` 无 `continuum-verify`；全仓 `grep -rn "struct Evidence\|enum EvidenceType" crates/` **零命中** | Task 6 的 `trybuild` 样例（**dev 边**）；§5.1 报出的两臂 |
| P5b `continuum-method`（`MethodRegistry`、`MethodId`、`MethodDomain`、`MethodEntry`、`MethodError`、`seeded()`） | **未交付**：`ls crates/` 无 `continuum-method` | Task 3 的全部（**普通边**） |
| P5e `ArtifactType::Report`（P5e 设计 §3.2 清单第 8 行定的新变体） | **未交付**：`artifact.rs:14-21` 六臂里无 `Report`，`:68-78` 的 `parse` 对 `"report"` 返回 `None` | **§4.2 第 6、7 行的端口**——故本块今天编不过 |
| P4 `continuum-semantics`（`RequirementId`） | **未交付**（`crates/` 下不存在） | **本块不用**：产出的证据一律 `Unattached`，故不登记那条边（设计 §2.1、§5.2） |
| **本块的 crate `continuum-research`** | **不存在**（实读：`crates/` 下无该目录；`grep -rn "continuum-research" crates/ Cargo.toml` **零命中**） | 本计划的全部交付物 |
| workspace `members` | **18 项**（`Cargo.toml:3` 起，`:22` 收） | Task 1 加一行 |

### 二、本计划的硬门：**三处前置未落地则整块编不过，且会把整条工作区的门一起弄红**

本块的依赖形态与本仓此前的块不同：`continuum-method`（P5b）与 `continuum-verify`（P5a）**今天不存在**，
`ArtifactType::Report`（P5e）**今天不在枚举里**。三个后果逐条写清：

1. **`Report` 不在枚举里 ⇒ 本 crate 的源码编不过**（§4.2 第 6、7 行的端口要用它）。这不是「测试缺一条」，
   是 `cargo build -p continuum-research` 就红。
2. **把一个成员加进 `members` 而它的路径依赖不存在 ⇒ 整个工作区的 cargo 命令一起失败**：
   路径依赖解析失败发生在 `cargo metadata` 之前，`cargo test -p <别的 crate>` 也一并失败。
   故本计划**不许**在 P5a/P5b 落地前把 `continuum-research` 加进 `members`，也不许先声明那两条路径依赖。
3. **据此，本计划的全部 task 一律排在 P5a、P5b、P5e 三处落地之后**（三处的落地各有其自己的计划与合并），
   且 Task 1 的「加成员行」与「写 `Cargo.toml`」是**同一步**——
   「工作区里不放一个编不过的成员」这条纪律不靠约定，靠这一步的原子性。

**开工前置核对（Task 1 Step 1 逐条跑，任一不成立即停下报协调者，不擅自补桩）**：

```bash
ls -d crates/continuum-verify crates/continuum-method
grep -n "Report" crates/continuum-artifact/src/artifact.rs
```

预期：前两条各列出一个目录；第三条在枚举本体、`ALL`、`as_str`、`parse` 四处各命中一次。
**未落地时的处置是「等」，不是「先把 `Report` 换成 `Json` 顶上」**——后者的产物与设计 §4.2 第 6、7 行不同，
且会在 P5e 落地时留下一处需要人来发现的差异。

**本块的中间态风险与本计划无关的一条**：本块零迁移，故不参与号段划分；
`crates/continuum-runtime/src/main.rs:98` 的 `runtime_migrations()` **一字不动**（见 Global Constraints）。

### 三、本计划对既有之物**一处请求都不提**

设计 §2.1 的边表只含既有 crate 与 P5a/P5b；§10 的对账条目里，需要别的块动的是
「`SourceSet` 是否收成一型」（P5e）、「逐位可复现 backend 名单的家」（P5e + 协调者）、
「两枚判定算子的 §262 级别」（P5a + 装配方）、「`Evidence` 的结果位」（P5a）四条——
**四条都是设计与协调者层的事，本计划不为它们建 task，逐条抄在 `## 遗留`**。

### 四、本节把什么交给谁

- **交给第 3 层（执行器）＋ 装配方**：本块三枚 `pub` 项**今天没有生产调用方**（设计 §10 第 10 条）。
  本块的算子只**注册**，节点的状态迁移与 `OperatorRegistry::resolve` 的调用点在执行器（切分 §四第 4 条）。
  **本计划不建那条执行路径。**
- **交给 P5a**：§5.1 报出的两枚 `EvidenceType` 臂（`ContradictionCheck` / `CitationVerification`）**已由 P5a 收下**
  （P5a 设计 §4.2 末：2026-10-10 一次改动加三臂，本块两枚均并入 `from_tool_result` 的「需要制品」那一支）
  ——**本计划的 task 里没有与它们相关的编码工作**。两枚证据的**极性**没有承载位，记在 `## 遗留`。
- **交给 P5b**：四条 `realized_by`（Task 3）；
  另有一条**据实留的形状**：`research-question` 与 `research-synthesis` 在 `research/` 里没有对应方法，
  故不在任何 `realized_by` 里（设计 §7 末）——**那不是漏 bind**。
- **交给 P5e**：`Report` 的**第一个生产者在本块**（`research-synthesis`）；P5e 设计 §5.2 记「`Report` 在媒体块没有生产者」，
  两处互指。另有「backend 名单的家」一条（`builtin` 同时出现在 P5e 的名单与本块 §4.2 第 3 行）。
- **交给协调者**：迁移号段——**本块零迁移、不占档**，P5e 设计 §11.3 建议给 P5d 的 `160` 因此**仍空**
  （判据：**「本块没占它」不等于「它没人占」**，见 `## 遗留` 第五节）。
- **交给规范维护者**（本项目无此角色）：设计 §11 第 2、5、7 条——四枚 `Json` 制品的 schema、
  `Synthesis` 该不该有一条方法名、研究域的后端分类。**本计划一条都不发明，逐条抄在 `## 遗留`。**
- **交给复审者**：本计划在设计里查出的**六条**问题（逐条见 `## 遗留` 第一节）——其中
  §9.3 的两处**预告错误**必须让实现者知道改过口径，否则会去查不存在的红。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划新增的 workspace 内边只有这些**（**逐条由需要它的 task 登记，一次不声明齐**——
  设计 §2.1 给的是**终态**，而「只登记实际用到的」是切分 §四第 6 条）：
  - Task 1：`continuum-research ← continuum-artifact`、`← continuum-operator`（两条普通边）；
  - Task 3：`continuum-research ← continuum-method`（普通边）；
  - Task 5：`continuum-research ← continuum-graph`（**dev 边**）；
  - Task 6：`continuum-research ← continuum-verify`（**dev 边**）。
  **终态五项，与设计 §2.1 的 `ALLOWED` 行逐项相同**（数组按字母序）。
  **上表之外一条边都不登记**——尤其没有指向 `continuum-port` / `continuum-semantics` / `continuum-persist` /
  `continuum-events` / `continuum-policy` / `continuum-capability` / `continuum-core` / `continuum-runtime` 的边
  （设计 §2.1 的五条「不登记」理由逐条照办）。
- **dev 边也要登记进 `ALLOWED`**：`cargo_tree_direct` 带 `--edges all`（`crates/continuum-runtime/tests/dependency_direction.rs`
  的 `cargo_tree_direct`），dev 边与普通边一视同仁。**本仓已有同形先例**：`continuum-provider` 那一行的注记
  明写「Task 4 起加上 persist：**dev 边**」（同文件 `:32-34`）、`continuum-connector` 那一行同形（`:115-116`）。
  **边别按「谁在用」判**：`continuum-graph` 与 `continuum-verify` 都是 dev 边——生产代码对它们零调用，
  用到它们的是用例（设计 §2.1）。
- **`--depth 1` 只钉直接边，这就是这条规则的完整粒度，本计划不另设闭包断言。**
  `cargo_tree_direct` 带 `--depth 1`，故它**看不到传递可达**：经一个中间 crate 间接够到别的层不会被抓住。
  **但本仓不因此留缺口**——Rust 要求**直接声明**才能 `use`，未写进 `Cargo.toml` 的传递依赖写不出
  `use …`。故「直接边」恰是这条规则的完整粒度。
  **写这一段的用途**：免得读到上面那条约束时以为它是「任何指向别的层的边都会变红」——**那会是一条假保证**。
- **本块零迁移、零表、零事件。** `crates/continuum-runtime/src/main.rs:98` 的 `runtime_migrations()` 一字不动；
  `crates/continuum-runtime/Cargo.toml` 一字不动（本块的三枚 `pub` 项没有生产调用方，装配方不加边）；
  **不新增任何 `Migration::new(..)`**（Task 7 Step 6 自证零命中）。
  **本块也不占号段**：`160` 仍空（判据见 `## 遗留` 第五节）。
- **枚举列的落库编码纪律在本块不适用**——本块零表。设计 §4.2 表里的 `determinism` / `side_effect_class` /
  `backend_candidates` 三栏是**算子值的字段**，经 `Operator` 的类型系统表达，不经库列。
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
  每一个 `git add`/`git commit` 之前先看 `git diff --cached --stat`。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不定义任何类型**（设计 §2.2）：本 crate 的 `pub` 面**恰三枚**，逐枚列在 Architecture 里。
  - **不写域包装**：不建 `evidence_from_research_finding` 一类函数（协调者 2026-10-10 裁定，
    口径写在 P5a 设计 §4.4）；**本 crate 的生产代码对 `continuum-verify` 零调用**
    （`OperatorImpl::execute` 的返回是 `Vec<Artifact>`），构造由宿主的执行代码直接调 `Evidence::from_tool_result`。
  - **不新造 `ArtifactType` 变体、不向 P5e 报出变体**：本块需要的两枚型里 `Json` 是既有的、
    `Report` 已由 P5e 的清单收下（设计 §3.2）。
  - **不新造 `EvidenceType` 臂**：两枚由本块**报出**，已由 P5a 落地（设计 §5.1）。
  - **不建 `MethodEntry`、不 `register` 方法、不建第二份方法目录**（切分 §四第 3 条、P5b 设计第六节）：
    本块只经 `bind` 填 `realized_by`。
  - **不建第二份复用判据**（切分 §四第 7 条）：不写 `can_reuse` 一类的函数，只声明 `determinism`
    让 P1 的判据成立（Task 5）。
  - **不建第二份「逐位可复现的 backend」名单**（切分 §四第 7 条、设计 §6.2）：
    本块不设注册期的名单检查，`register_research_operators` **只查重**。
  - **不建执行路径、不迁节点状态、不调 `OperatorRegistry::resolve` 于生产代码**（切分 §四第 4 条）。
  - **不碰 `execution_policy` / `verification_policy`**（切分 §四第 5 条、P4）。
  - **不碰 `Checkpointable`**（切分 §二第 4 条判给 P5e）。
  - **不定义四枚 `Json` 制品的 schema**（设计 §11 第 2 条：规范一处未给）。
  - **不声明两枚判定算子在 §262 五级序里的级别**（设计 §5.3）。

### 本计划特有的「签名即判据」，照抄前须对源

设计 §2.1 与 §4.1 已把要接的签名抄了一遍，但**本计划给出的任何签名都只是预期值**，
实现前须先读源确认，不符时以源码为准并回报。逐处写明**这一处容易错在哪**：

- `Operator` 的**七个**字段（`definition.rs:79-88`，字段本体 `:80-87`）：
  `id` / `version` / `input_schema: Vec<ArtifactType>` / `output_schema: Vec<ArtifactType>` /
  `determinism` / `side_effect_class` / `backend_candidates: Vec<BackendId>`。
  **设计 §4.1 的订正注记写明「八字段」那个数串自相邻的 §258 `Evidence`**——本计划照**七**写。
- **`Operator` 的清单写不成 `const`**（设计 §4.1.1 的两条独立实测理由）：
  `OperatorId::new`（`:27`）与 `BackendId::new`（`:69`）**都不是 `const fn`**，
  且 `Operator` 的三个 `Vec` 里**没有一个是空的**（§4.2 逐格可核）⇒ `Vec::push` 不是 `const` 操作。
  **本仓三处 `ALL`（`ArtifactType::ALL` `artifact.rs:28`、`EffectType::ALL`、`PrivacyClass::ALL` `artifact.rs:100`）
  都是无字段枚举**，元素是常量，故 `const` 成立——**形状能成立的前提在这里不存在**。
  交付面因此是 `pub fn all_operators() -> [Operator; 7]`；**「7」仍落在返回类型里**，
  「清单个数的改动编译期可见」这条性质不因它不是 `const` 而丢失。
- `OperatorId::new(id: impl Into<String>)` / `as_str()`；`OperatorVersion::new(u32)` / `as_u32()`；
  `BackendId::new(impl Into<String>)` / `as_str()`。
  **`BackendId` 是大小写敏感的 `String`**（`:66`）——故 `["builtin"]` → `["Builtin"]` **不是等价变异体**
  （它与设计 §9.3 的第一条预告别同向）。
- `OperatorRegistry::{new, register(&mut self, Operator) -> Result<(), OperatorError>, resolve(&self, &OperatorId, &OperatorVersion) -> Result<&Operator, OperatorError>}`
  （`registry.rs:20`、`:24`、`:36`）。**`entries` 是私有 `HashMap`（`:16`），本块没有迭代面**——
  本计划里所有「表的内容」断言只能经 `resolve` 逐枚观察。**这一点决定了 Task 2 的用例形状**。
- `OperatorError::{NotFound, Duplicate}` **恰两枚**（`registry.rs:6-12`）；`Duplicate` 的判定在 `register` 内（`:24-34`）。
- `MethodRegistry::{new, seeded, register, resolve, bind, select}`（P5b 设计 §4.5）：
  **`bind(&mut self, id: &MethodId, realized_by: &[&str]) -> Result<(), MethodError>`**（第二参是 `&[&str]`，不是 `Vec<String>`）；
  **`select(&self, domain: MethodDomain) -> Vec<&MethodEntry>`（返回该领域的全部条目，与 `realized_by` 是否为空无关）**；
  `MethodError::{NotFound { id }, Duplicate { id }}`；`seeded()` 建 18 条、`research/` **四条**。
- `ArtifactType::ALL`（`artifact.rs:28`，带数目字面量 `[ArtifactType; 6]`）、`as_str`（`:47`）、
  `parse`（`:68`，对表外串返回 `None`）。
- `continuum_graph::reuse::{cache_key(&Operator, ContentHash) -> Option<CacheKey>, can_reuse(&Operator, Option<&CacheKey>, &ContentHash, bool) -> bool}`
  （`reuse.rs:16`、`:29`）；**`cache_key` 对 `NonDeterministic` 返回 `None`**（`:22`）。
- `Evidence::from_tool_result` 是**八参数**（P5a 设计 §4.4）——**本块不调用它**，
  只在 `## 遗留` 里记它是宿主侧的构造点。
- **`trybuild` 把 `[dev-dependencies]` 也算进样例的依赖**（实测 `trybuild-1.0.121/src/run.rs:239`：
  `dependencies.extend(source_manifest.dev_dependencies);`）。**故 §9.4 的两份样例可以引用
  `continuum_verify::Evidence`，而不必把那条边升成普通边**——这是 Task 6 的 dev 边成立的依据。
- **`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；而且逐份各不相同，
  不许拿一个值套两处。** 本仓已为此付过代价（P3D 的计划给三份样例共用一个错误码，实测三份各不相同）。
- **`trybuild` 的两条坐标**：（i）样例与 `.stderr` **按文件主干配对**（`x.rs` 配 `x.stderr`），
  改名只改一侧即报「缺少期望输出」；（ii）**样例里的注释也是照片坐标**——
  注释写着「应编译失败：…」时，那句说明与样例实际钉的东西必须一致（本仓既有样例的注释里
  专门写了「这条样例钉住的是哪一件事，要说准」）。

---

## 已付过代价的纪律（**按本块实情改写**，不是 P4 那套的照抄）

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；
   正向的「变红」跑全量是加分。**变异分四档，每一处「预期谁红」都要标档位**：
   **取反**（把某个判定或取值反过来）／**放宽**（实现比声明的接受更多）／**收紧**（实现比声明的接受更少）／
   **移除**（删掉整条守卫或整条实现）。**三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，
   处理是**换真变异体而非补用例**；
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
   cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
   **本块已预先识别的等价变异体（逐条给判据）**：
   - **`research-question` 的 `backend_candidates` 由 `["primary-model"]` 改成 `["primary_model"]`**——
     设计 §9.3 把它标成「等价变异体、全绿」，**本计划的结论相反**：那个串被 §9.2 第 (b) 条逐格断言的
     手写期望表**取到了**，故两版在该用例上给出不同结果 ⇒ **它是真变异体，(b) 红**。
     详见 `## 遗留` 第一节第 1 条。
   - **`research-source-set` 的 `backend_candidates` 由 `["builtin"]` 改成 `["Builtin"]`**——
     `BackendId` 是大小写敏感的串（`definition.rs:66`），改串即改身份 ⇒ **不是等价变异体**，(j) 红。
   - **`all_operators()` 的返回类型 `[Operator; 7]`**：把某枚的 `version` 由 1 改成 1（无操作）是等价；
     改成 2 即 (b) 的 version 断言与之相抵。**故凡改 `version` 的变异体都要指明改的是哪一枚。**
2. **变异脚本必须带还原护栏**：每次变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、
   **每轮用独立日志路径**，报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径」。模板：
   ```bash
   cd /home/DslsDZC/Continuum/.worktrees/p5d-plan
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
   不抽代表。**本块的落点**：`research-source-set` 是七枚里**唯一**的 `Deterministic`（Task 1 逐枚断言）、
   其 backend **恰为** `builtin` 一枚（Task 1 的 (j)）、`register_research_operators` **只**报一件事（Task 2）。
4. **枚举式判据的通病**：它钉的是**枚举到的那些**，不是**那个全称**。
   **本块有两处只能靠枚举承担，必须照实写明**：
   - §4.5 的边表 **(l) 只钉住枚举到的七行**，且其中六行落在 `Json`↔`Json` 上、**无区分力**——
     「§330 的七步之间的顺序」本块**表达不出**（`Operator` 无前驱字段，设计 §4.3 末）。
   - **(m) 恒真**（端口类型的值必在 `ArtifactType::ALL` 里，而 `parse ∘ as_str == id` 由 P1 的
     `every_artifact_type_round_trips_through_its_encoding` 把守）⇒ **不许写成「两枚型都由 P5e 收下」的全称**，
     见 `## 遗留` 第一节第 5 条。
5. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」。本块的错误面只有两族、共三臂：
   `OperatorError::{NotFound, Duplicate}`（Task 2 覆盖 `Duplicate` 两次）与
   `MethodError::{NotFound, Duplicate}`（Task 3 覆盖 `NotFound` 一次）。
   **`register_research_operators` 只可能返回 `Duplicate`**（设计 §4.1：本函数要报的失败只有「(id, version) 撞车」一件事）。
6. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本计划逐处标出「另一侧」是哪一个用例：
   Task 2 的 (c) 与 (e)（登记成功 ↔ 一个字都不写）、Task 3 的 (f) 与 (h)（解析成功 ↔ `NotFound`）、
   Task 5 的「一枚 `Some`」与「六枚 `None`」、Task 1 的 (j) 与 (b) 的其余六枚。
7. **「删掉 X 即红」要先问「删掉之后行为真的变了吗」**——索引序、稳定排序、可推断的字面量、`..` 豁免
   都会让它成为**等价变异体**（第 1 条 (b)）。凡本计划标了「移除档」的地方，都已先答过这一问。
8. **「断言必须有照片」，且照片的坐标有三个维度**：**位置**（那一处是否存在）、
   **可见性**（那个符号从被测侧看不看得见——`pub(crate)`／私有会让照片根本写不出来）、
   **坐标**（名字与行号会不会漂）。**本块的可见性约束**是硬的：`OperatorRegistry::entries` 私有，
   故「表的内容」只能经 `resolve` 观察（Task 2 的全部「表未变」断言都建在这条之上）。
9. **正控制不得自证**：**语料若从被测清单里取，就是恒真的假照片。** 本块的语料纪律——
   §9.2 第 (a)(b)(k) 三条的期望**必须手写**（不从 `all_operators()` 推出），
   (l) 的**边表**手写（端口取值才从被测清单读），(f) 的 `realized_by` 期望**必须手写**（不从 `all_operators()` 的 id 推出）。
   **三件一起断**：漏（少一枚）、多（多一枚）、位置（顺序错）——**故 (a) 断言的是七对逐位相等，不是集合相等**。
10. **自指计数不写数值**：凡「本文件/本计划里有 N 处」这类数**要么写成不含自身的范围，要么留成一条命令**
    （设计 §2.1 的注记：那条数写下即旧）。**本计划里所有「恰 N 枚」的数值断言，钉的都是被测清单的数目，
    不是本计划的篇幅**——两者不得混写。
11. **门读数的判据是日志里 `Running` 与 `test result:` 的行数**（Doc-tests 算一条），
    不是「命令退出码为 0」也不是「`Compiling` 出现过」。`cargo test` **不收 `--keep-going`，用 `--no-fail-fast`**；
    不带它会停在第一个失败目标处、后面的目标一行不跑。

**另两条运行纪律**：跑测试加 `timeout`；**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。

**变异日志是证据，必须活到复审结束**：**实现者保留 `.tmp/`，由协调者在复审结束后清理**。
报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。
**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。

**变异口径分层**：变异**条数**按「有多少条**互不相同**的守卫」定，不按分支数定，且分两档、**不许混成一句「通过」**：
- **承重守卫 → 全量套件**：两侧对钉的守卫（Task 2 的 (c)/(e)、Task 5 的 `Some`/`None` 两侧）、
  **fail-open 的那一侧**（Task 2 的「半注册」那两个夹具）、失败路径**判别哪一种 `Err`**、
  **跨 crate 才可见**的效果（`ALLOWED` 与实际依赖一致、`members` 与工作区一致、
  Task 3 的 `MethodRegistry` 是 P5b 的、Task 6 的 `Evidence` 是 P5a 的）。
- **其余分支 → 本 crate 的包级套件**（`cargo test -p continuum-research`），
  报告里须**标明证据强度较低**并列出「这一条可能漏掉的跨 crate 观察点」。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**七枚算子的取值表、类型签名与关键判定**，**不给整段可粘贴实现**；
凡与既有 crate 交互的签名（`OperatorRegistry::{register, resolve}`、`MethodRegistry::{seeded, bind, select}`、
`ArtifactType::{as_str, parse}`、`reuse::{cache_key, can_reuse}`），实现前须先读该 crate 的源码确认
（Task 1 起，`continuum-method` 与 `continuum-verify` 是先落地后引入的，**它们的源码那时才存在**），
不符时以源码为准并回报。

**本计划特有的三处「照抄前须对源」**：

- **`MethodRegistry::bind` 的第二参是 `&[&str]`**，不是 `Vec<String>`——写 `bind(&id, &["a"])`，
  不写 `bind(&id, vec![...])`。
- **`select(MethodDomain)` 返回该领域的全部条目，与 `realized_by` 是否绑过无关**——
  故「四条都被 bind 过」不能由 `select` 的数目断言承担（`## 遗留` 第一节第 3 条）。
- **`Operator` 的字段全 `pub`**（`definition.rs:80-87`）⇒ 用例可以**就地构造任意 `Operator`**
  （Task 2 的 (e) 与补充夹具就靠这条），**不必**经 `all_operators()` 取语料。这是「正控制不得自证」在本块的一个便宜出口。

---

# 文件结构

```
crates/continuum-research/
  Cargo.toml                依赖逐 task 增量登记（三普通 + 二 dev）
  src/lib.rs                导出面与 crate 文档；三枚 pub 项各一行
  src/operators.rs          all_operators() 与七枚算子的取值（§4.2 表的实现体）
  src/register.rs           register_research_operators()（先核后写）
  src/methods.rs            bind_research_methods() 与四条 realized_by（§7 表）
  tests/catalog.rs          §9.1 的两条清单守卫 + §9.2 的 (a)(b)(j)
  tests/register.rs         §9.2 的 (c)(d)(e) + 本计划补的半注册夹具
  tests/methods.rs          §9.2 的 (f)(g)(h)
  tests/edges.rs            §9.2 的 (l)(m)
  tests/reuse.rs            §9.2 的 (k)
  tests/type_level.rs       trybuild 驱动
  tests/compile_fail/*.rs   §9.4 的两份样例（各配同名 .stderr）
```

**本计划要改的既有文件**（只有 Task 1 碰它们，各一行）

```
Cargo.toml                                               members 加 crates/continuum-research
crates/continuum-runtime/tests/dependency_direction.rs    ALLOWED 加 continuum-research 条目
```

**本计划不碰的文件**：`crates/continuum-runtime/{Cargo.toml,src/**}`（本块三枚 `pub` 项无生产调用方）、
`crates/continuum-artifact/**`（只 `use` 它的 `ArtifactType`，一个字不改、一个变体不加）、
`crates/continuum-operator/**`、`crates/continuum-graph/**`（只 `use` 它的 `reuse`）、
`crates/continuum-method/**`、`crates/continuum-verify/**`、`crates/continuum-port/**`、
`crates/continuum-semantics/**`、`crates/continuum-persist/**`、`docs/spec/**`、`docs/02-工程.md`、
`docs/superpowers/specs/2026-10-09-p5d-research-domain-operators-design.md`。

---

### Task 1: crate 骨架、七枚算子表与 `all_operators()`

**Files:**
- Create: `crates/continuum-research/Cargo.toml`
- Create: `crates/continuum-research/src/{lib.rs,operators.rs}`
- Create: `crates/continuum-research/tests/catalog.rs`
- Modify: `Cargo.toml`（`members` 加一行）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加条目，**先记两条边**）

**Interfaces:**
- Consumes: `continuum_operator::{Operator, OperatorId, OperatorVersion, Determinism, SideEffectClass, BackendId}`、
  `continuum_artifact::ArtifactType`
- Produces: `continuum_research::all_operators() -> [Operator; 7]`

- [ ] **Step 1: 前置核对与骨架登记**

先跑跨计划前置第二节的三条命令。**任一不成立即停下报协调者。**

`Cargo.toml` 的依赖**只声明本 task 用得到的**：`continuum-artifact`、`continuum-operator`（两条路径依赖）。
**`continuum-method` / `continuum-graph` / `continuum-verify` / `trybuild` 由需要它们的 task 增量加**
（切分 §四第 6 条：只登记实际用到的）。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加（数组按字母序）：

```rust
    // P5 跨领域层（设计 §2.1）：研究领域的七枚算子。
    // 本 crate 不新建表、不写事件、不碰策略与能力面。
    // `continuum-graph`（§6.1 的照片）与 `continuum-verify`（§9.4 的 trybuild 样例）
    // 两条都是 **dev 边**——生产代码对它们零调用，由各自的 task 增量补进来。
    ("continuum-research", &["continuum-artifact", "continuum-operator"]),
```

- [ ] **Step 2: 写用例（`tests/catalog.rs`）**

- `the_seven_ids_are_written_out_by_hand`（§9.2 第 (a) 条）：**手写七个串**，按 §4.2 的行序
  与 `all_operators()` **逐位** `assert_eq!`。**顺序是这条用例的一半**（集合相等会让「调换两行」成为等价变异体）。
  红条件：改 `src/operators.rs` 中第 3 枚的 id 字面量（锚点：`OperatorId::new("research-source-set")` 那一处的串，
  七枚字面量两两不同，故锚点唯一）⇒ **取反档**，本用例的第 3 位红。
  另一条红条件：把第 1、2 枚交换顺序（锚点：`operators.rs` 里那两枚的**行序**）⇒ 逐位相等在第 1、2 位红。
- `the_seven_rows_are_asserted_cell_by_cell`（§9.2 第 (b) 条）：**手写七行期望表**，
  逐格断言 `input_schema` / `output_schema` / `determinism` / `side_effect_class` / `backend_candidates`
  与 `version`。**七行逐行，不是抽样。**
  红条件（**取反档**）：改 `research-contradiction-check`（第 5 行）那一枚的 `side_effect_class`
  由 `Pure` 改成 `Idempotent`（锚点：该枚的 `side_effect_class:` 字段值）⇒ 第 5 行红。
  红条件（**取反档**）：改第 3 行的 `determinism` 由 `Deterministic` 改成 `NonDeterministic` ⇒ 第 3 行红，
  同时 Task 5 的 (k) 也红（两条独立红点，见 §9.3 第 1 行）。
  **`backend_candidates` 那一格逐元素断言**：`["builtin"]` 与 `["Builtin"]` 在 `BackendId` 上是两个值（`definition.rs:66`），
  故这一格不是等价变异区。
- `the_one_deterministic_operator_is_the_source_set`（§9.2 第 (j) 条与设计 §6.2 第 1 条）：
  `research-source-set` 的 `backend_candidates` **恰等于** `[BackendId::new("builtin")]`
  （**逐元素**，不是「非空」），且七枚里 `determinism == Deterministic` 的**恰一枚**、其 id 是 `research-source-set`。
  红条件（**放宽档**）：把它改成 `["builtin", "primary-model"]`（锚点：该枚的 `backend_candidates` 字段值）
  ⇒ 长度与逐元素两半各红。
  **并写明本条的作用域**：它钉的是**本块的声明**，**不是一条被强制的注册期规则**——
  `register_research_operators` 只查重，将来第二枚 `Deterministic` 算子带外部后端候选出现时**今天没有任何东西会拦它**
  （设计 §6.2 第 2 条）。
- `the_catalog_lists_seven_distinct_ids_despite_the_return_type`（§9.1）：`len() == 7` **且**两两 `id` 不同。
  红条件（**取反档**）：把第 7 枚的 id 写成第 6 枚那个串（锚点：第 7 枚的 `id` 字面量）
  ⇒ 「两两不同」那一半红，而**返回类型 `[Operator; 7]` 的数目仍绿**——这一条正是为这个差而写。
- `every_operator_declares_version_one`（§9.1）：逐枚 `assert_eq!(op.version, OperatorVersion::new(1))`。
  红条件（**取反档**）：改第 4 枚的 `version` 为 `OperatorVersion::new(2)`（锚点：该枚的 `version` 字段值）
  ⇒ 第 4 位红；**返回类型不变，故编译仍过**——这也说明它不是编译期照片。

- [ ] **Step 3: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum/.worktrees/p5d-plan
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-research --test catalog
```

预期：`cargo` 编不过（`all_operators` 未定义）——**这是「命令失败」，不是「用例红」**，
本轮据此确认「待写的实现体确实不存在」；**并在这一轮同时跑一次 `cargo tree -p continuum-research --depth 1 --edges all`
核 `ALLOWED` 的两条边已生效**。

- [ ] **Step 4: 实现**

`src/operators.rs` 的取值照设计 §4.2 的七行逐格落，`src/lib.rs` 导出：

```rust
/// 本领域七枚算子的全集。顺序即设计 §4.2 表的行序。
/// **返回值是数组不是 `&'static [Operator]`**：`Operator` 含 `Vec` 与 `String` 且三者均非空，
/// 不可 const 构造（见设计 §4.1.1）。
/// 不变量：七枚的 id 逐枚不同、version 均为 1——由 `tests/catalog.rs` 的两条用例把守。
pub fn all_operators() -> [Operator; 7];
```

**七枚的取值（逐格照设计 §4.2，本表是示意，以设计的表为准）**：

| # | id | input_schema | output_schema | determinism | side_effect_class | backend_candidates |
|---|---|---|---|---|---|---|
| 1 | `research-question` | `[]` | `[Json]` | `NonDeterministic` | `Pure` | `["primary-model"]` |
| 2 | `research-search` | `[Json]` | `[Json]` | `NonDeterministic` | `Pure` | `["retrieval-backend"]` |
| 3 | `research-source-set` | `[Json]` | `[Json]` | `Deterministic` | `Pure` | `["builtin"]` |
| 4 | `research-evidence-extraction` | `[Json]` | `[Json]` | `NonDeterministic` | `Pure` | `["primary-model"]` |
| 5 | `research-contradiction-check` | `[Json]` | `[Json]` | `NonDeterministic` | `Pure` | `["primary-model"]` |
| 6 | `research-synthesis` | `[Json]` | `[Report]` | `NonDeterministic` | `Pure` | `["primary-model"]` |
| 7 | `research-citation-verification` | `[Report, Json]` | `[Json]` | `NonDeterministic` | `Pure` | `["primary-model", "retrieval-backend"]` |

**`determinism` 与 `side_effect_class` 是两个正交字段**（设计 §4.4 第 3 条）：第 3 行 `Deterministic + Pure` 自洽，
第 1、2、4、5、6、7 行 `NonDeterministic + Pure` 也自洽——**「经模型/检索」不等于「有外部副作用」**。
**`side_effect_class` 判的是「算子自身的执行有没有改变本仓之外的状态」**，依据与边界测试在设计 §4.4 第 3 条
（那段是这一口径的家，P5c/P5e/P5f 都指向它）；**本块的七枚一律取 `Pure`**，理由是七枚都只读、只产出本仓制品
（`crates/continuum-effect/src/effect.rs:17-24` 的 `EffectType` 六臂与本块七枚无一对得上）。
**注意 `NonDeterministic` 与 `Pure` 同现不是笔误**——写进 `operators.rs` 的文档注释。

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-research Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git diff --cached --stat
git commit -m "feat(research): crate 骨架、七枚研究算子与 all_operators()"
```

---

### Task 2: `register_research_operators`——「先核后写」与不留半注册

**Files:**
- Create: `crates/continuum-research/src/register.rs`
- Create: `crates/continuum-research/tests/register.rs`
- Modify: `crates/continuum-research/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_operator::{OperatorRegistry, Operator, OperatorError, OperatorId, OperatorVersion}`
- Produces: `continuum_research::register_research_operators(registry: &mut OperatorRegistry, operators: &[Operator]) -> Result<(), OperatorError>`

- [ ] **Step 1: 写用例（`tests/register.rs`）**

三条设计给的用例 ＋ **一条本计划补的承重夹具**（申报见 `## 遗留` 第四节）：

- `a_full_batch_registers_and_every_id_resolves`（§9.2 第 (c) 条）：以 `&all_operators()` 调一次
  ⇒ `Ok`，随后**逐枚** `resolve(&OperatorId::new(那个手写的串), &OperatorVersion::new(1))` 成功
  且返回的算子与该枚的 id 相等。
  红条件（**移除档**）：把函数体改成直接 `Ok(())` 不写表（锚点：函数体内的注册循环）
  ⇒ 七次 `resolve` 全是 `Err(NotFound)` ⇒ 本用例红。
- `registering_the_same_batch_twice_returns_duplicate`（§9.2 第 (d) 条）：第一次 `Ok`；
  第二次同一批 ⇒ `Err(OperatorError::Duplicate { id, version })`，**断言 `id` 恰是批首那一枚**
  （`research-question`）与 `version == 1`——**不是「返回了 Err」**。
  红条件（**取反档**）：把「遇重复即报错」改成「遇重复即跳过」⇒ 第二次返回 `Ok` ⇒ 本用例红。
  **本条的第二半「表未变」在本夹具下不承重，须照实写进注释**：同一批的**首枚**即撞车，
  故「边核边写」的实现也什么都不写、表也变不了 ⇒ 那一半**恒真**。
  **真正的守卫是下一条**（`## 遗留` 第一节第 2 条）。
- `a_duplicate_after_a_new_operator_leaves_no_half_registration`（**本计划补，承重**）：
  空表先注册**只含批首那一枚**的批；再调 `[probe, 批首那一枚]`（`probe` 是 `research-probe`，
  就地构造的 `Operator`，字段全 `pub`）⇒ `Err(Duplicate)`，
  **且 `probe` 用 `resolve` 查不到**（`Err(NotFound)`）。
  红条件（**移除档**）：删掉「先核后写」（改成逐枚 `register` 并在错处返回）⇒ `probe` 已写进表
  ⇒ 本用例红。**这是本块唯一能区分「先核后写」与「边核边写」的夹具。**
- `a_batch_with_an_internal_collision_registers_nothing`（§9.2 第 (e) 条，**批内自撞**）：
  空表，批 = `[probe_a, probe_a₂]`，两枚的 `(id, version)` 相同（就地构造）⇒ `Err(Duplicate)`
  且 `probe_a` **查不到**。
  红条件（**移除档**）：同上的「边核边写」⇒ `probe_a` 可查 ⇒ 本用例红。

**两条夹具共用的前提写进注释**：`Operator` 的七个字段全 `pub`（`definition.rs:80-87`），
故**任意 `Operator` 就地可造**——这正是设计 §4.1「参数带一批算子（不是只吃 `all_operators()`）」的用途。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-research --test register
```

- [ ] **Step 3: 实现（`src/register.rs`）**

判定顺序**写死**：先逐枚核「(id, version) 是否**已在表里**」（经 `OperatorRegistry::resolve`，
**不读它的私有 `entries`**），再核「**批内是否自撞**」（同一批里两枚的 `(id, version)` 相同），
**全部通过后**再逐枚 `register`。

```rust
/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核「(id, version) 是否已在表里」与「批内是否自撞」，全部通过后再逐枚注册；
/// 任一枚不成即返回 `Err`，此时注册表的内容与调用前逐枚相同（不留半注册）。
///
/// 一枚研究算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **本函数不新造错误类型**：要报的失败只有「(id, version) 撞车」一件事，
/// 而它的判定已有一枚臂（`OperatorError::Duplicate`，判定在 `OperatorRegistry::register`）。
///
/// **「先核后写」为什么值得写**：不写它，一次带重复项的注册会留下半注册的表，
/// 而调用方拿到的 `Err` 说的是「这次调用没成功」——调用方据 `Err` 重试一次，
/// 第二批会从重复项处再失败，表里那一半却已经在了。
pub fn register_research_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), OperatorError>;
```

`src/lib.rs` 的 crate 文档补一句「本 crate 只有三枚 `pub` 项，逐枚列在此处」。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-research
git diff --cached --stat
git commit -m "feat(research): register_research_operators 的先核后写"
```

---

### Task 3: `bind_research_methods`——`research/` 四条方法的 `realized_by`

**Files:**
- Create: `crates/continuum-research/src/methods.rs`
- Create: `crates/continuum-research/tests/methods.rs`
- Modify: `crates/continuum-research/src/lib.rs`（导出）
- Modify: `crates/continuum-research/Cargo.toml`（普通依赖 `continuum-method`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-method`）

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_method::{MethodRegistry, MethodId, MethodDomain, MethodError}`
- Produces: `continuum_research::bind_research_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>`

- [ ] **Step 1: 写用例（`tests/methods.rs`）**

**本 task 的头一步是先读 `crates/continuum-method` 的源码**，核 `bind` / `select` / `MethodId::new` / `seeded()`
与「签名即判据」一节列的四处一致；**不符即以源码为准并回报**。

四条 `realized_by` 的期望**手写**（不从 `all_operators()` 的 id 推出）：

| `MethodId` | 期望的 `realized_by` |
|---|---|
| `retrieval` | `["research-search", "research-source-set"]` |
| `evidence-analysis` | `["research-evidence-extraction"]` |
| `contradiction-check` | `["research-contradiction-check"]` |
| `citation-verification` | `["research-citation-verification"]` |

- `every_research_method_is_bound_to_the_four_handwritten_lists`（§9.2 第 (f) 条）：
  `MethodRegistry::seeded()` 起一个注册表、`OperatorRegistry` 注册七枚（否则解析必失败）、
  调 `bind_research_methods` ⇒ `Ok`；随后**按上表逐条** `resolve(&MethodId::new(名))`，
  断言 `realized_by` **逐位等于**上表那一条（**不是非空**），且**每个串**在 `OperatorRegistry::resolve` 上成功
  （`OperatorVersion::new(1)`）。
  红条件（**移除档**）：删掉 `bind_research_methods` 里 `retrieval` 那一次 `bind`（锚点：那一次调用的 `MethodId` 字面量）
  ⇒ `retrieval` 的 `realized_by` 为空 ⇒ 逐位相等红。
  红条件（**取反档**）：把 `"research-source-set"` 写成 `"research-source-set "`（多一个尾空格，锚点：那一枚字面量）
  ⇒ 逐位相等的第 2 位红**且** `resolve` 得 `Err(NotFound)` ⇒ 本用例红。
  **并写明这条的补件性质**：`realized_by` 是 `Vec<String>`，**块间是文本弱引用、没有编译期照片**
  （P5b 设计 §3.2 末）——本用例是它的**运行期补件**，且它**在本 crate 内闭得上**，
  因为本块同时持有 `all_operators()` 与 `MethodRegistry`。
- `the_research_domain_selects_exactly_four_entries`（§9.2 第 (g) 条）：
  `bind_research_methods` 之后 `select(MethodDomain::Research)` 返回**四条**，且四条 `id` 恰是手写的四个。
  红条件（**移除档**，**跨 crate**）：删掉 P5b `seeded()` 里 `research/` 的一条（锚点：`crates/continuum-method`
  的 `seeded()` 中那一条 `MethodId`）⇒ 本用例红。
  **并写明本条钉的是什么（设计此处自陈的方向与本例相反，见 `## 遗留` 第一节第 3 条）**：
  P5b 的 `select(MethodDomain)` **返回该领域的全部条目、与 `realized_by` 是否绑过无关**，
  故「四条都被 bind 过」**不是**本条钉的（那是上一条钉的）；
  本条钉的是「P5b 的 `seeded()` 在 `research/` 域**恰四条**」——即**多一枚或漏一枚都不会静默**。
- `binding_an_unseeded_method_id_is_not_found`（§9.2 第 (h) 条）：
  对**未 seed 的** `MethodId::new("research-probe")` 调 `MethodRegistry::bind` ⇒
  `Err(MethodError::NotFound { id })` 且 `id` 恰是那一个。
  红条件（**取反档**）：把 `MethodRegistry::bind` 的未命中分支改成 `Ok(())`（锚点：`crates/continuum-method`
  的 `bind` 实现体）⇒ 本用例红。
  **并写明本条的作用域（据实，见 `## 遗留` 第一节第 4 条）**：本条**钉的是 P5b 的 `bind` 的错误臂**，
  **不是本块的失败路径**——`bind_research_methods` 的四个 id 全部已 seed，
  故**它不可能返回 `Err`**，本块对它**没有失败路径的照片**；「本块不新增错误类型」这条纪律
  在本块内的照片只有**签名**（返回类型里写的是 `MethodError`）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-research --test methods
```

- [ ] **Step 3: 实现（`src/methods.rs`）**

```rust
/// 把 §7 表的四条 `realized_by` 填进方法库（P5b 设计第六节：每块对其领域目录里的
/// **每一条已 seed 的 id** 调用一次 `bind(id, &[...])`）。`research/` 四条逐条非空
/// （P5b 设计 §4.5：`has_counterpart(Research) == true` ⇒ 空即「漏 bind」）。
///
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
/// **本函数不做二次登记**：不 `register` 方法条目（四块不得自造 `MethodEntry`）。
/// **本函数不吞错**：任一条 `bind` 返回 `Err` 即原样透出 `MethodError`。
/// 失败直接透出 `MethodError`，本块没有自己的错误类型。
///
/// **`research-question` 与 `research-synthesis` 不在上表里，这不是漏 bind**：
/// §187 的 `research/` 只有四条，而 §330 有七个步骤——`Synthesis` 在方法库里没有对应名。
pub fn bind_research_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;
```

**四处调用逐条写死**（`MethodId::new(..)` ＋ `&[&str]`），顺序与 §7 表的行序一致。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-research Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git diff --cached --stat
git commit -m "feat(research): bind_research_methods 与 research/ 四条 realized_by"
```

---

### Task 4: §4.5 的边表与端口类型的可解析性

**Files:**
- Create: `crates/continuum-research/tests/edges.rs`
- Modify: `crates/continuum-research/src/operators.rs`（**仅当边表断言查出端口取值错**）

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_artifact::ArtifactType`
- Produces: 无新公开类型

- [ ] **Step 1: 写用例（`tests/edges.rs`）**

- `every_downstream_input_type_is_supplied_by_a_listed_upstream`（§9.2 第 (l) 条）：
  **边表手写**（七行，逐行给「下游 id → 上游 id 的列表」），断言**逐行**：
  下游 `input_schema` 的**每一个型**都能在该行列出的上游里找到，且所引的那一枚的 `output_schema` 含该型。
  **断言的对象是「交集中的型」，不是「两侧相等」**——第 7 行的 `input_schema` 是 `[Report, Json]`，
  两个来源各供一枚，两侧不相等。
  边表（照设计 §4.5）：

  | 下游算子 | 上游（来源） |
  |---|---|
  | `research-question` | 无（问题来自 `Intent`，不是 `Artifact`） |
  | `research-search` | `research-question` |
  | `research-source-set` | `research-search` |
  | `research-evidence-extraction` | `research-source-set` |
  | `research-contradiction-check` | `research-evidence-extraction` |
  | `research-synthesis` | `research-evidence-extraction`、`research-contradiction-check` |
  | `research-citation-verification` | `research-synthesis`、`research-evidence-extraction` |

  红条件（**放宽档**）：把第 1 行 `research-question` 的 `input_schema` 由 `[]` 改成 `[Text]`
  （锚点：该枚的 `input_schema` 字段值）⇒ 该行「无上游」而下游要一个型 ⇒ 本用例红。
  红条件（**收紧档**）：把第 7 行 `research-citation-verification` 的 `input_schema` 里的 `Report` 删掉
  （只留 `Json`）⇒ Task 1 的 (b) 红，而**本用例的第 7 行变绿**（`Json` 在上游找得到）
  ⇒ **故 (l) 单独不足以钉住这一格，(b) 与 (l) 要一起看**（设计 §9.3 已列此条）。
  **并写明本条的作用域（照实，不许写成全称）**：
  **它只钉住枚举到的这七行**；其中**六行落在 `Json`↔`Json` 上、无区分力**
  （§3.2 的代价：`SourceSet` 未收成一型），它只能拒掉「把 `Report` 接到只收 `Json` 的端口」这一类错。
  **「§330 的七步之间的顺序」本块表达不出**（`Operator` 无前驱字段，设计 §4.3 末）——
  本用例的断言**弱于顺序**：`Json → Json` 的三条边在类型上无法与「把链倒过来接」相区分。
- `every_declared_port_type_is_a_known_artifact_type`（§9.2 第 (m) 条）：
  对七枚的每个 `input_schema` / `output_schema` 里的型 `t`，断言 `ArtifactType::parse(t.as_str()).is_some()`。
  红条件（**取反档**，**跨 crate**）：删掉 `crates/continuum-artifact/src/artifact.rs` 的 `parse` 里
  `"report"` 那一臂（锚点：`artifact.rs:68-78` 的 `"report"` 臂）⇒ `Report` 的 `as_str` 取回 `"report"`、
  `parse` 返回 `None` ⇒ 本用例红。
  **并写明本条的实际强度（据实，见 `## 遗留` 第一节第 5 条）**：**它是恒真的**——
  端口类型的取值必在 `ArtifactType::ALL` 里，而 `parse ∘ as_str == id` 已由 P1 的
  `every_artifact_type_round_trips_through_its_encoding` 把守；本条的真正内容是
  **「本 crate 能编译」这件事的推论**（`Report` 不在枚举里则本 crate 根本编不过），
  故**不许**把它写成「本块声明了两枚正确的型」的全称。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-research --test edges
```

- [ ] **Step 3: 实现**

本 task **预期不改生产代码**——它把 Task 1 落的七行端口取值**对着 §4.5 的边表**复核一遍。
若断言查出错，改的是 `src/operators.rs` 的端口取值（**以设计 §4.2／§4.5 为准，不自行发明边**），
并把这处差异记进 `## 遗留`。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-research
git diff --cached --stat
git commit -m "test(research): §4.5 边表与端口类型的两条断言"
```

---

### Task 5: 复用判据的消费面——`determinism` 声明与 §305 的照片

**Files:**
- Create: `crates/continuum-research/tests/reuse.rs`
- Modify: `crates/continuum-research/Cargo.toml`（**dev 依赖** `continuum-graph`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-graph`，**dev 边**）

**Interfaces:**
- Consumes: Task 1 的 `all_operators`；`continuum_graph::reuse::{cache_key, can_reuse, CacheKey}`；
  `continuum_artifact::ContentHash`
- Produces: 无新公开类型（**本块不定义第二个复用判据**）

- [ ] **Step 1: 写用例（`tests/reuse.rs`）**

- `only_the_source_set_yields_a_cache_key_and_the_other_six_do_not`（§9.2 第 (k) 条）：
  **两侧都钉**——一（且仅一）枚为 `Some`，六枚为 `None`：
  （i）对 `research-source-set` 断言 `cache_key(&op, h)` 是 `Some(k)` 且 `k.operator_version == OperatorVersion::new(1)`、
  `k.input_hash == h`；对**另外六枚逐枚**断言 `cache_key(&op, h).is_none()`（**逐枚六条，不抽代表**）。
  （ii）`can_reuse` 的四条件齐时（`cached = Some(&k)`、`current_input_hash = h`、`contract_affected = false`）为真。
  红条件（**取反档**）：把第 3 枚的 `determinism` 由 `Deterministic` 改成 `NonDeterministic`
  ⇒ 那一枚由 `Some` 变 `None` ⇒ 本用例红（设计 §9.3 第 1 行）。
  红条件（**取反档，另一侧**）：把第 2 枚的 `determinism` 由 `NonDeterministic` 改成 `Deterministic`
  ⇒ 该枚由 `None` 变 `Some` ⇒ 本用例红（设计 §9.3 第 2 行）。
  **并写明本条的作用域**：本块**不建第二份复用判据**（切分 §四第 7 条），
  本条的兑现**不是本块的一个函数**，而是**一枚算子的 `determinism` 声明**——
  `cache_key` 读的正是 `operator.determinism`（`reuse.rs:17`）而**不看 backend**
  （即 P5e 设计 §5.4 记的那处结构缺口；本块的处置见设计 §6.2 与 `## 遗留`）。
- `the_cache_key_is_not_a_second_reuse_judgement`（**否定式照片**，切分 §四第 7 条）：
  断言 `crates/continuum-research/src/` 里没有 `can_reuse` / `cache_key` 一类的第二份定义——
  **照片机制是 Task 7 Step 3 的 grep（源码面复核）**，本用例内不写读源码的断言
  （本仓已有一处人工复核点，不再造自指的断言；与 P4 的同一取舍）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-research --test reuse
```

- [ ] **Step 3: 实现**

本 task **预期不改生产代码**——它把 Task 1 的 `determinism` 声明对着 §6.1 的判据复核一遍。
dev 依赖与 `ALLOWED` 的 `continuum-graph` 由本 task 登记。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-research Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git diff --cached --stat
git commit -m "test(research): determinism 声明与 §305 复用判据的六行"
```

---

### Task 6: 结构层的照片（`trybuild`）

**Files:**
- Create: `crates/continuum-research/tests/type_level.rs`
- Create: `crates/continuum-research/tests/compile_fail/evidence_struct_literal_is_not_constructible.rs`（＋同名 `.stderr`）
- Create: `crates/continuum-research/tests/compile_fail/evidence_has_no_default.rs`（＋同名 `.stderr`）
- Modify: `crates/continuum-research/Cargo.toml`（**dev 依赖** `continuum-verify`、`trybuild`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-verify`，**dev 边**）

**Interfaces:**
- Consumes: `continuum_verify::Evidence`
- Produces: 无新公开类型

- [ ] **Step 1: 写用例（`tests/type_level.rs` 与两份样例）**

```rust
/// 本 crate 里**造不出** `Evidence`：字段私有、无 `Default`（P5a 设计 §4.4 的「唯一产生点」）。
/// 这是那条结构事实在本块的**使用侧**照片。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
```

两份样例各配**同名** `.stderr`（按文件主干配对）：
（i）外部**结构体字面量** `Evidence { .. }`（字段私有）编不过；
（ii）`Evidence::default()`（无 `Default` 实现）编不过。
**两份的 `.stderr` 内容与错误码一律取自实跑**——逐份各不相同，不许拿一个值套两处。
**样例里的注释也是照片坐标**：注释里写清「这条钉的是哪一件事」，且与样例实际钉的东西一致。

红条件（**移除档**，**锚点都在 P5a 的 crate 里**）：把 `Evidence` 的第 0 个字段改成 `pub`、
或给 `Evidence` 加一个 `Default` 派生（锚点：`crates/continuum-verify` 的 `Evidence` 定义与它的派生）
⇒ 对应那份样例**编译通过** ⇒ `trybuild` 报「expected failure but compiled successfully」⇒ **本用例红**。
**并写明这不是「变异导致编译失败」**：这里红的是**用例本身报 mismatch**，与纪律 1(c) 说的是两回事。
红条件（**移除档**，本 crate 侧）：删掉 `Cargo.toml` 的 dev 依赖 `continuum-verify`
⇒ 样例报「未解析导入」、与旧的 `.stderr` 不符 ⇒ 本用例红（**依赖 trybuild 也把 dev 依赖算进样例**，
实测 `trybuild-1.0.121/src/run.rs:239`）。

**并写明本条的限度（设计要求，设计 §9.4 第 1 条末）**：本条**证不到**「产出只能经 `Evidence::from_tool_result`」——
那个构造点是 `pub`，构造由**宿主**的执行代码直接调它完成，**调用点不在本 crate 里**。
故本条是本块在 `continuum-verify` 上的**唯一使用点**，也是那条边为 **dev 边**的理由。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-research --test type_level
```

预期：两份样例的 `.stderr` 尚未落盘 ⇒ `trybuild` 报「wip」并要求确认 ⇒ 本轮据此确认
「待写的照片确实不存在」。**落盘时逐份按实跑输出抄，不凭记忆写错误码。**

- [ ] **Step 3: 实现**

只有 `.stderr` 两份是「实现」（`tests/type_level.rs` 在 Step 1 已写完）。
**样例不引用 `continuum_research`**——它们钉的是 P5a 那一层的结构事实，经本 crate 的测试面驱动。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-research Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git diff --cached --stat
git commit -m "test(research): trybuild 照片——Evidence 造不出来"
```

---

### Task 7: 收尾与复核

**Files:**
- Modify: 本 crate 与 `crates/continuum-runtime/tests/dependency_direction.rs`（**仅在复核发现缺口时**）

- [ ] **Step 1: 全量验证（门读数按判据取）**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：全绿、0 warning。**门读数的判据是日志里 `Running` 与 `test result:` 的行数**（Doc-tests 算一条）——
**逐条数、写进报告**；不带 `--no-fail-fast` 会在第一个失败目标处停住、后面的目标一行不跑。

- [ ] **Step 2: 依赖边的机检与逐对核对**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

**逐条核三件事**：（a）`continuum-research` 的条目**恰五项**、按字母序、与设计 §2.1 逐项相同；
（b）`Cargo.toml` 与 `ALLOWED` **精确一致**（逐对 `assert_eq!` 已覆盖，含两条 dev 边）；
（c）`members` 从 18 项变 19 项，且新成员在表里（漏了即 `workspace 成员 … 未列入 ALLOWED` 红）。
**并把粒度写清**：`--depth 1` 只钉**直接边**；传递可达在 Rust 下不可 `use`，**故直接边即这条规则的完整粒度**，
**不另设闭包断言**。**不要把这句读成「任何指向别的层的边都会变红」**——那会是一条假保证。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
cd /home/DslsDZC/Continuum/.worktrees/p5d-plan
grep -rn "Migration::new(\|continuum_persist\|Tx\b" crates/continuum-research
grep -rn "can_reuse\|cache_key\|fn is_candidate_backend\|fn compatible\|fn decide_retry" crates/continuum-research/src
grep -rn "evidence_from_\|from_tool_result\|EvidenceType" crates/continuum-research/src
grep -rn "MethodEntry\|MethodRegistry::register\|UNREALIZED_BY_DESIGN" crates/continuum-research/src
grep -rn "serde_json\|thiserror" crates/continuum-research
grep -rn "reqwest\|std::fs\|File::create" crates/continuum-research/src
```

**六条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；
**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：
（a）**零迁移、不碰 `Tx`**；（b）**没有第二份判据**（`can_reuse` / `cache_key` / `is_candidate_backend` /
`compatible` / `decide_retry` 五枚一枚都不在本 crate 里重写）；
（c）**不写域包装**（`evidence_from_*` / `from_tool_result` 零调用，`EvidenceType` 零出现）；
（d）**不自造方法条目**（`MethodEntry` 与 `MethodRegistry::register` 零出现）；
（e）**无 `serde_json` / `thiserror`**（本块没有自己的错误类型、不传值）；
（f）**无文件系统与网络访问**（本块不落盘、不触网）。
**作用域写死**：六条只扫 `crates/continuum-research`（`src/` 与测试分开看），**不含别的 crate**。

- [ ] **Step 4: 逐条核对 §9.2 的 (a)–(m)、§9.4 与 §9.5**

| 设计的条目 | 落点 |
|---|---|
| (a)(b)(j) | Task 1 `tests/catalog.rs` |
| (c)(d)(e) ＋ 本计划的半注册夹具 | Task 2 `tests/register.rs` |
| (f)(g)(h) | Task 3 `tests/methods.rs` |
| (i) **已撤销**（设计 §9.2、§5.2：包装函数已删） | **无落点，据实标注为「随 (i) 撤销」** |
| (k) | Task 5 `tests/reuse.rs` |
| (l)(m) | Task 4 `tests/edges.rs` |
| §9.4 的 trybuild | Task 6 |
| §9.1 的两条清单守卫 | Task 1 `tests/catalog.rs` |
| §9.5 的七条 | **无照片，逐条照原文抄进 `## 遗留`** |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑**承重守卫**（两侧对钉的、fail-open 侧的、失败路径判别 `Err` 的、跨 crate 才可见的），
每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 → 还原后 `sha256sum` → 独立日志路径。
**逐轮在报告里附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位」**。
**必跑的六轮**：
（i）**移除档**：`register_research_operators` 的「先核后写」删掉 ⇒ 应红 `tests/register.rs` 的
`a_duplicate_after_a_new_operator_leaves_no_half_registration` 与 `a_batch_with_an_internal_collision_registers_nothing`；
**不应红** `a_full_batch_registers_and_every_id_resolves`（合法批次两侧都绿）。
（ii）**移除档**：`bind_research_methods` 的 `retrieval` 那一条 ⇒ 应红 (f) 的两个串那一半；
**不应红** (k)、(l)、(m)。
（iii）**取反档**：`research-source-set` 的 `determinism` ⇒ 应红 (k) 的第一枚；
**不应红** (j)、(l)、(m)（它们不看 determinism）。
（iv）**放宽档**：第 1 行 `input_schema` 由 `[]` 改成 `[Text]` ⇒ 应红 (l) 第 1 行；
**不应红** (b) 之外的行——**注意 (b) 也会红**（它逐格断言端口），故这两条一起看。
（v）**收紧档**：第 7 行删掉 `Report` ⇒ (b) 红、(l) 第 7 行**变绿**（设计 §9.3 已列）。
（vi）**跨 crate 承重**：删掉 `ALLOWED` 的 `continuum-research` 条目 ⇒ 应红
`every_crate_depends_only_on_its_allowed_set` 的「workspace 成员未列入 ALLOWED」那一句。
**若某轮跑成了「编译失败」而不是「用例失败」，按纪律 1(c) 记为「编译不过」，并另换一个真变异体。**

- [ ] **Step 6: 零迁移与装配面未改的自证**

```bash
cd /home/DslsDZC/Continuum/.worktrees/p5d-plan
grep -rn "Migration::new(" crates/continuum-research || echo "本 crate 零迁移"
git diff --stat main -- crates/continuum-runtime/src crates/continuum-runtime/Cargo.toml
```

预期：第一条零命中；第二条**无输出**（`runtime_migrations()` 与 runtime 的清单一字未动）。
**并写明号段**：本块**不占档**，`160` 仍空——**判据是「本块不需要号」，不是「160 没人占」**
（后者本块自证不了）。

- [ ] **Step 7: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。**本计划不新建、也不修改第二份文档**：
若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 8: 提交**

```bash
git add <本 task 改动的显式路径>
git diff --cached --stat
git commit -m "chore(research): P5d 研究领域算子的收尾与复核"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**
设计 §11 的 **8** 条与 §10 的 **10** 条**本计划一条都不发明**——收件人照原文，逐条落在本节第五、六节。

### 一、本计划在设计里查出的**六条**问题（发现即报，本计划一处都不替设计补写）

1. **⚠️ §9.3 最后一行把「改 backend 串`-`→`_`」标成「等价变异体、全绿」，与 §9.2 第 (b) 条相抵。**
   该行逐字：「**等价变异**：把 `research-question` 的 `backend_candidates` 从 `["primary-model"]`
   改成 `["primary_model"]`（大小写不同）｜**全绿**——这是一枚**等价变异体**（两个串在表里等价，
   都是本设计定的名字，无外部消费者）。要打红它必须换变异体：把它改成与另一枚算子共用的串以外的任意值，
   那时 b 红」。
   **两处对不上**：（i）§9.2 第 (b) 条要求**手写期望表**逐格断言 `backend_candidates`——
   源码里的串一改，那格就 `assert_eq!` 失败 ⇒ **本变异体打红 (b)**，不是全绿；
   （ii）该行自己给的那条规则（「改成不与另一枚算子共用的串 ⇒ b 红」）**也涵盖 `primary_model`**
   （别的算子用的是 `primary-model`），故该行**自相矛盾**。
   另有一处笔误：括注「（大小写不同）」描述的是上一行的 `builtin` → `Builtin`，
   `primary-model` → `primary_model` 是**分隔符**不同，不是大小写。
   **本计划的取法**：按「能指出一个把两版分开的输入，它就是真变异体」办——
   **(b) 就是那个输入**，故本块的 `primary_model` 变异体按**取反档**记，预期 (b) 红。
   **收件人：设计作者 ＋ 复审者。**
2. **⚠️ §9.2 第 (d) 条的「表未变」半在「同一批注册两次」这一夹具下恒真。**
   该条要求「同一批注册两次 ⇒ `Err(Duplicate)`，**且注册表内容与第一次之后逐枚相同**」，
   并自陈「「表未变」那一半钉的是**先核后写**」，§9.3 也预告「移除档 ⇒ **d 的第二个子例**、e 的第二个子例红」。
   **实测的夹具语义**：第二批的**首枚**即与表里那枚撞车 ⇒ 「边核边写」的实现在**第一枚**上就返回 `Err`、
   **一个字都没写**，表因此也变不了 ⇒ 那一半**恒真**（与「索引序」「可推断的字面量」同型的假照片）。
   故 §9.3 对 d 的预告**不成立**——照它去查会去找一个不存在的红。
   **本计划的取法**：保留 (d)（它仍钉「`Err` 而不是静默覆盖」），在那条用例的注释里照实写明
   第二半在本夹具下不承重；**另补一条** `a_duplicate_after_a_new_operator_leaves_no_half_registration`
   （批 = `[新算子, 已注册的那一枚]`）作**承重**夹具——这是本块的 (d)/(e) 里唯一能区分
   「先核后写」与「边核边写」的形状。**收件人：设计作者 ＋ 复审者。**
3. **⚠️ §9.2 第 (g) 条的自陈与 `select` 的语义相反。**
   该条要求「`select(MethodDomain::Research)` 返回**四条**」，并自陈「**这一条钉的是「四条的 id 都被 bind 过」
   而不是「四条都存在」**」。**而 P5b 设计 §4.5 写死 `select` 「返回该领域的全部条目，按 id 升序」
   ——与 `realized_by` 是否绑过无关**（`bind` 只改 `realized_by`，不改条目的存在与否）。
   故一个 `select` 的数目断言钉的恰恰是「**四条都存在**（在 `seeded()` 的目录里）」，
   **钉不到**「四条都被 bind 过」——后者是第 (f) 条的「逐条非空」钉的。两句话正好反了。
   **本计划的取法**：`tests/methods.rs` 的第二条用例**照实标注作用域**为
   「P5b 的 `seeded()` 在 `research/` 域**恰四条**」（多一枚或漏一枚都不会静默），
   并在注释里写明「四条都被 bind 过」是 (f) 钉的。**收件人：设计作者 ＋ 复审者。**
4. **⚠️ §9.2 第 (h) 条在本块内不可达。**
   该条要求「对未 seed 的 `MethodId` 调 `bind` ⇒ `MethodError::NotFound`」，其说明栏写
   「§7 的反例（错透了 `MethodError`，本块没有自己的错误类型）」。
   **而 `bind_research_methods` 的四个 id 全部已 seed**，`MethodError` 里 `bind` 只能返回 `NotFound`
   ⇒ **该函数不可能返回 `Err`**，故「透出 `MethodError`」这件事**在本块内没有可达的失败路径**。
   照该条直写，用例会去调 `MethodRegistry::bind` —— 那钉的是 **P5b 的** 错误臂（P5b 设计 §10 判据 4 已有一条同形的）。
   **本计划的取法**：用例照写，但**照实标注它钉的是 P5b 的 `bind`**，并在同一处写明
   「本块对 `bind_research_methods` 的失败路径**没有照片**」——
   「本块不新增错误类型」在本块内的照片只有**签名**。**收件人：设计作者 ＋ 复审者。**
5. **⚠️ §9.2 第 (m) 条恒真，它的实际内容是「本 crate 能编译」。**
   该条要求「七枚算子的每一个端口类型都在 `ArtifactType` 里可解析（`ArtifactType::parse(..).is_some()`
   对每个 `as_str`）」，其说明栏写它钉的是「§3.2 的『两枚型中 `Json` 既有、`Report` 由 P5e 的清单收下』这一断言」。
   **而端口取值是 `Vec<ArtifactType>`，其每个值必在 `ArtifactType::ALL` 里**，
   且 `parse ∘ as_str == id` 已由 P1 的 `every_artifact_type_round_trips_through_its_encoding` 把守
   ⇒ 该断言在 P1 的门为真时**恒真**。它真正承载的是**可编译性**（`Report` 不在枚举里则本 crate 编不过），
   而该条自己的括注（「这条用例要等 P5e 的枚举改动落地后才写得出」）说的正是这件事。
   **本计划的取法**：用例照写（它是一条便宜的跨 crate 交叉核对），但**照实标注强度**，
   **不许**写成「本块声明了两枚正确的型」的全称。**收件人：设计作者 ＋ 复审者。**
6. **一处据实的边界，不是错**（记在此是为了让复审不必重新推一遍）：**§4.5 的边表七行里六行无区分力**
   （§3.2 的代价：`SourceSet` 未收成一型 ⇒ 三条相邻边都是 `Json` 接 `Json`）。
   故 §9.2 第 (l) 条的实际判别力只落在**第 1 行**（`[]` 与 `[Text]` 之分）与**第 7 行**（`Report`）上；
   **「§330 的七步之间的顺序」本块表达不出**（`Operator` 无前驱字段，设计 §4.3 末、§11 第 3 条）。
   Task 4 的注释已照此写。**收件人：复审者。**

### 二、跨计划前置：本块的**硬门**（逐条见跨计划前置第二节）

- **P5a（`continuum-verify`）未交付** ⇒ Task 6 的 dev 边与两份 `trybuild` 样例写不出来。
- **P5b（`continuum-method`）未交付** ⇒ Task 3 整体写不出来（普通边）。
- **P5e 的 `ArtifactType::Report` 未落地** ⇒ **本 crate 编不过**（§4.2 第 6、7 行的端口）。
- **中间态的危险形状**：在 P5a/P5b 落地前把成员加进 `members`、或先声明那两条路径依赖，
  会让**整个工作区**的 cargo 命令一起失败（路径依赖解析失败发生在 `cargo metadata` 之前）。
  故 Task 1 的「加成员行」与「写 `Cargo.toml`」是同一步，且整个计划排在三处落地之后。

### 三、拍不到的照片（设计 §9.5 已列 **7** 条，**逐条照原文，本计划不发明**）；本计划补两条

8. **本块对 `bind_research_methods` 的失败路径没有照片**（第一节第 4 条）：四个 id 全已 seed，
   该函数不可能返回 `Err`。**收件人：设计作者 ＋ 复审者。**
9. **本块的 `ALLOWED` 五项里两条是 dev 边，而「生产代码零调用」这件事只有间接照片**：
   `tests/type_level.rs` 与 `tests/reuse.rs` 在，而 `src/` 里零 `use`（Task 7 Step 3 的 grep）。
   「零 `use`」是**源码文本层面**的观察，**不是**类型层的禁止。**收件人：复审者。**

### 四、本计划自定的形状与取值（**申报**，逐条说清代价）

- **依赖边与 `ALLOWED` 条目逐 task 增量登记**（先两条普通边，Task 3 加 `continuum-method`，
  Task 5／6 各加一条 dev 边）：设计 §2.1 给的是**终态五项**，而切分 §四第 6 条要求「只登记实际用到的」。
  **代价**：`ALLOWED` 会被改四次；**收益**：任何一次提交里的表与实际依赖都精确一致。
- **Task 2 补的承重夹具** `a_duplicate_after_a_new_operator_leaves_no_half_registration`：
  设计 §9.2 的 (d)/(e) 两条在「同一批两次」与「空表 + 批内自撞」两个夹具下**只有一个**能区分
  「先核后写」与「边核边写」（第一节第 2 条）。**代价**：多一条用例（它同时需要就地构造 `Operator`）；
  **收益**：fail-open 的那一侧有了照片。
- **Task 3 的 (f) 除「逐条非空 + `resolve` 成功」外，还断言 `realized_by` 的逐位内容**（手写表）：
  只断言「非空」会让「填错成别的算子、但那个算子恰好存在」静默通过。
  **代价**：多一份手写期望表；**收益**：§7 的四行有逐位照片。
- **Task 3 的 (g) 照实改标作用域**（第一节第 3 条）、**Task 4 的 (m) 照实改标强度**（第 5 条）、
  **Task 4 的 (l) 写明只钉枚举到的七行**（第 6 条）：**三处都是标签订正，不改设计要求的断言本身**。
- **`research-probe` 这个就地构造的算子**：仅出现在 `tests/register.rs` 的夹具里，
  **不进 `all_operators()`**；它的 `(id, version)` 与七枚两两不同（否则夹具自身会先撞车）。
- **测试文件按 §9.2 的分组切**（`catalog` / `register` / `methods` / `edges` / `reuse` / `type_level`）：
  设计把 (a)–(m) 列成一张表，本计划把它按「被测的那一枚 `pub` 项 / 那一份判定面」分成六个文件。
  **代价**：文件名是本计划定的；**收益**：每一门的失败面在报告里可指认。
- **`select(MethodDomain::Research)` 的断言落在 `tests/methods.rs`**：它测的是 P5b 的 `seeded()`，
  但**本块是 P5b 所说「消费块才能核」的那个消费块之一**，故这条判据由本块承担。

### 五、设计 §11 的 **8** 条：本计划一条都不发明，收件人照原文

设计 §11 的 8 条**逐条以「收件人」结尾**，本计划**不重述其内容**（重述就是第二份转录，正是本项目出错最多之处）。
**逐条照收件人归类**（**为让「核过」可查**）：

- **收件人：协调者 ＋ P5e**——第 **1** 条（`SourceSet` 该不该有自己的 `ArtifactType`；
  缺的是那一步裁定）、第 **4** 条（「逐位可复现的 backend 名单」的家在哪；撞车面是 P5c/P5f/P5e 三处）。
- **收件人：规范维护者**——第 **2** 条（四枚 `Json` 制品的 schema）、第 **5** 条（`Synthesis` 该不该有一条方法名）、
  第 **7** 条（研究域的后端分类）。
- **收件人：第 3 层的执行器 ＋ 协调者**——第 **3** 条（七步之间的顺序约束本块表达不出；
  同处还挂着 §330 的 SHOULD 与《总纲》§8.4 的「固定链条」这一处规范侧未决）。
- **收件人：P5a ＋ 装配方**——第 **6** 条（两枚判定算子的 §262 级别未声明）。
- **第 8 条**（本块的四条 `pub` 项今天都没有生产调用方）**不是本块的缺口，是第 3 层的**——
  与切分 §八「算子解析的落点仍无人认领」同源，本计划不把那一处各自再记一次。

### 六、设计 §10 的 **10** 条对账条目：**已关闭的留痕，仍开的照收件人**

- **已关闭**：第 **1** 条（与 P5e/P5a 的两枚同形证据包装，协调者 2026-10-10 裁定四块一律不写域包装）、
  第 **3** 条（两条 `EvidenceType` 新臂，已一次改动落地于 P5a 设计 §4.2）、
  第 **6** 条（`side_effect_class` 的口径，协调者裁定以「对外部世界的持久改变」为准）、
  第 **9** 条（P5e 的 `ALL_OPERATORS` const 写法，P5e 已按本条订正）。
  **四条本计划都不接**——它们留在那里是为了让「一处说法曾被订正」可查。
- **仍开**：第 **2** 条（backend 名单的家：协调者 ＋ P5e）、第 **4** 条（证据的结果位：P5a）、
  第 **5** 条（两枚判定算子的 §262 级别：P5a ＋ 装配方）、第 **7** 条（与 P5b 的四条 `bind` 与文本弱引用：P5b）、
  第 **8** 条（`Report` 的生产者：P5e，两处互指）、第 **10** 条（本块三枚 `pub` 项无生产调用方：第 3 层 ＋ 装配方）。
- **设计 §12 的六条**（在切分文档、P5e 的设计与本轮派单里发现的错或缺口）**本计划不重复记录**；
  其中第 **6** 条已复测为正确的三处（§五 P5d 行的三个依据、§187 的 `research/` 四条、§330 的七名与行号）
  本计划**照用**，不再另测。

### 七、迁移号段

**本块零迁移、不占档。** P5e 设计 §11.3 的次序建议把 P5d 列为 `160`，**故 `160` 仍空**。
**判据照切分 §二第 5 条**：**「本块没占它」不等于「它没人占」**——本块只能自证「不需要号」。
**收件人：协调者**（六块与 P6 的档由它统一划）。**本节不另开对账条目**：本块不参与号段划分。

---

## 交付给谁

- **交给驱动的接线 task（未建）／第 3 层（执行器）**：本块三枚 `pub` 项**今天没有生产调用方**。
  本块只**注册**算子与**填** `realized_by`；节点状态迁移、`OperatorRegistry::resolve` 的调用点、
  算子的执行体都在第 3 层（切分 §四第 4 条）。**本计划只交付清单与两个入口，不建执行路径。**
- **交给 P5a**：§5.1 报出的两枚 `EvidenceType` 臂**已收下并落地**（P5a 设计 §4.2），本块无编码工作；
  仍开的一条是**证据的结果位**（两枚证据的极性没有承载位，与本块 §5.1 末同源）。
- **交给 P5b**：四条 `realized_by`（Task 3）；
  另有一条**据实留的形状**——`research-question` 与 `research-synthesis` 在 `research/` 里没有对应方法
  （§330 有七步、§187 的 `research/` 只有四条），**不是漏 bind**。
- **交给 P5e**：`Report` 的**第一个生产者在本块**（`research-synthesis`），与 P5e 设计 §5.2 互指；
  以及「逐位可复现的 backend 名单的家」一处（`builtin` 同时出现在两处）。
- **交给协调者**：本计划在设计里查出的**六条**问题（`## 遗留` 第一节，其中**第 1、2 两条是预告错误**，
  它们会让实现者去查不存在的红）；迁移号段（本块不占档，`160` 仍空）；设计 §11 第 1、4 条与 §10 第 2 条。
- **交给规范维护者（本项目无此角色）**：设计 §11 第 2、5、7 条与设计 §12 第 2 条里那处规范侧未决
  （§330 的 SHOULD 与《总纲》§8.4 的「固定链条」）。**本计划一条都不发明。**
- **交给复审者**：`## 遗留` 第一节的六条与第三节的两条补记；
  以及本计划的**申报项**（第四节七条）——它们都是「本计划自定的形状」，判据是「代价写清了没有」。
