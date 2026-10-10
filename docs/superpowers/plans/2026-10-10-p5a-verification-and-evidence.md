# P5a（验证与证据判定）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建一个新 crate `continuum-verify`，落《工程》§8.1 的**前九行**（`docs/02-工程.md:485-493`，即 `Evidence 数据模型` / `Requirement Coverage Graph` / `VerificationProfile` / `Verifier 优先级判定` / `Independent Verifier` / `Verifier Adversary` / `Mutation Verification` / `Completion Predicate 判定` / `Test Validity Verifier`）：
`Evidence`（§258）与其唯一产生点 `from_tool_result`（§89）、Requirement Coverage 的逐项判定（§260 §190 §261）、`VerificationPolicy` 的严格解析（§236）与 `VerificationProfile` 的取得点（§261 §194）、Verifier 的五级序选择与隔离输入面（§262 §263 §29 §120）、Completion Predicate 的十项合取判定（§266 §195）、Test Validity 的五检查与聚合（§191），连同它们的落库（迁移 130）与 `EventType::VerificationFailed` 的第一个产生点。

**Architecture:** 本块是**第 8 层（跨领域）**的新 crate，依赖 `执行层 (3)` 与更下层（§9.1 的 `执行层 (3) → 跨领域 (8)`，`docs/02-工程.md:573-586`），**不给语义层（P4 的 `continuum-semantics`）登记任何边**——`语义层 (2) → 执行层 (3) → 跨领域 (8)` 已在链上，反向登记即成环，而 `:588` 逐字写着「依赖方向单向，无环」。凡跨层输入（Contract 的字段、候选 verifier、`mandatory_effects_completed`、独立于 Worker 的判定值）一律以**值**由装配方传入（P4 设计 §1.2.3 的规则）。
**本块是判定，不是执行**：「工具结果绕过 `Evidence` 进入判定」不可表达（`Evidence` 八字段私有、无 `Default`、**不派生 `Deserialize`**，§4.4）；「Worker 的完成声明进初判输入」不可表达（`InitialVerifierInput` 的字段里没有那个类型，§7.4）；「完成」不可表达（`CompletionVerdict` 只有 `Blocked` / `Undetermined` 两臂，§8.2）；「覆盖充分」不可表达（`CoverageVerdict` 没有正臂，§6.2）；「非阻断失败」不可表达（`VerifierVerdict::Fail` 没有 `blocking: bool`，§7.5）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-persist`（`Tx` / `Migration` / `Db` / `Value` / `PersistError`）、`continuum-graph`（`NodeId` / `Node` / `AdfirGraph` / `ExecutionProfile`）、`continuum-artifact`（`Artifact` / `ArtifactId` / `ArtifactType` / `load_artifact`）、`continuum-operator`（`BackendId`）、`continuum-events`（`EventType::VerificationFailed` / `Event`）；`serde`、`serde_json`、`thiserror`；dev：`trybuild`、`tempfile`。

**设计依据：** `docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`（**唯一事实来源**，1046 行）。
**切分与冻结口径：** `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`（§一 路径归属、§二 接口冻结、§四 横切约束七条、§五 P5a 行、§六 复审安排、§七 验收、§八 已知缺口）。
**本计划不改设计、不改规范、不改代码以外的任何计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-10 在工作树 `p5a-plan` 实读）

| 依赖 | 实际状态 | 本块用在哪 |
|---|---|---|
| P0 `continuum_persist::{Db, Migration, Tx, Value, PersistError}`（`crates/continuum-persist/src/db.rs:14`、`:72`、`src/tx.rs:26`、`src/lib.rs:12-18`） | **已交付** | 迁移 130（Task 9）与全部写入路径 |
| P0 `continuum_events::{EventType, Event, Event::new}`（`crates/continuum-events/src/event.rs:13`、`:25`、`:63`、`:81`） | **已交付** | `VerificationFailed` 是本块唯一的写事件（Task 9） |
| **`EventType::VerificationFailed` 今天零个写入方**（实测 `grep -rn VerificationFailed crates/`：只有 `event.rs:25` 的定义、`:41` 的 `ALL`、`:54` 的 `as_str` 三处，加上 `crates/continuum-events/tests/event_envelope.rs:18` 的名表那一行与 `:27` 的往返用例） | **已交付的类型，本块是第一个产生点** | Task 9 |
| P1 `continuum-artifact` 的 `Artifact` / `ArtifactId` / `ArtifactType`（`src/artifact.rs:151`、`:170`、`:172`）、`load_artifact`（`src/persist.rs:90`） | **已交付** | `Evidence.artifact_refs`、§266 第 1 项的 `artifact_exists`（Task 9） |
| P1 `continuum-graph` 的 `NodeId`（`src/ids.rs`）、`Node`（`src/node.rs:20` 的 `verification_policy` 字段、`:19` 的注记）、`AdfirGraph`（`src/graph.rs:41`，`terminal_nodes` 访问器在 `:70`）、`Edge` / `EdgeKind`（`src/edge.rs:47`、`:9`）、`ExecutionProfile`（`src/execution.rs:20`，`model` `:21`、`provider` `:22`、`backend` `:24`） | **已交付** | 生产者与受验节点的身份、EVIDENCE 边核对、Worker 身份（Task 4 / 6 / 9） |
| P1 `continuum-operator` 的 `BackendId`（`src/definition.rs:66`，导出在 `src/lib.rs:7-9`） | **已交付** | 候选 backend 与 `EvidenceProducer::Node`（Task 2 / 6） |
| P1 `trybuild` 的既有做法（`crates/continuum-node/tests/type_level.rs` 与 `tests/compile_fail/*.rs`，`dev-dependencies` 里的 `trybuild` 就在 `crates/continuum-node/Cargo.toml:24`） | **已交付** | Task 10 照它的形状写（含 `LISTED_SAMPLES` 与目录对钉的那条用例） |
| **P4 `continuum-semantics`（`RequirementId` / `RequirementClass`）** | **未交付**：实测 `crates/` 下无 `continuum-semantics`，`p4` 分支上亦无；全仓 `grep -rn 'RequirementId\|RequirementClass' crates/` **零命中** | 见第二节 |
| **P4 的设计本体**（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`） | **已定稿、仍在动** | 只作读法来源（§7.1 §7.2 §16），不作依赖 |
| **本块的新 crate `continuum-verify`** | **不存在**（实读 `ls crates/` 无该目录；`grep -rn 'continuum-verify\|continuum_verify' crates/ Cargo.toml` 零命中——**`docs/superpowers/specs/2026-10-09-p5d-…-design.md` 有命中，那是 P5d 的设计文本，不是代码**） | 本计划的全部交付物 |
| **P5c / P5d / P5e / P5f 的设计** | **已定稿并合入 `main`**（`docs/superpowers/specs/2026-10-09-p5{c,d,e,f}-…-design.md` 各一份，实测该目录含这四份）；**四块的 crate 均不存在** | 只在跨块接口处读：P5c §5.3 报出的 `ModelReview`、P5d §5.1 报出的 `ContradictionCheck` / `CitationVerification`、P5f §7.1 与 P5e §13 第 5 条的「不请求新臂」（三臂已在设计 §4.2 收下，Task 1 一次落地） |
| P6 的切分（`docs/superpowers/specs/2026-10-08-p6-scope-and-split.md`） | 已定稿 | 迁移号段那一档与工具结果的两个消费者（设计 §14 第 9 条），本计划只记遗留 |

### 二、本块的**跨计划前置**：`RequirementId` 与 `RequirementClass` 今天不存在，而本块**不能**向语义层登记边

这是本计划开工前唯一一处必须先说清的事，理由是两个方向的代价都落在同一条规范规则上。

**事实（逐项实测）**：

- 设计 §4.3.1 写「`RequirementId` 的来源：§224 的 `Requirement.id`，属 P4 的 `continuum-semantics`」，并写「本设计**不**定义第二个 `RequirementId`」；§14 第 1 条把「`RequirementId` 由 P4 提供」列为待对账的假设。
- 设计 §2.1 的依赖边表**不含** `continuum-semantics`，且 §6.1 逐字写「**不登记到 `continuum-semantics` 的边**，也不造第二个 `TaskContract`」。
- 而 `语义层 (2) → 执行层 (3)`、`执行层 (3) → 跨领域 (8)`（`docs/02-工程.md:573-586`），**故 `跨领域 (8) → 语义层 (2)` 会闭成 2-环**，与 `:588` 的「依赖方向单向，无环」直接相抵。

**结论（本计划据此办）**：**不得**登记 `continuum-verify ← continuum-semantics` 的边；`Contract` 与 `Requirement` 的身份由本块**以值承载**（`RequirementId` 在 Task 2 落，`RequirementClass` / `ContractRequirement` / `ContractView` 在 Task 5 落），`RequirementClass` 的四值照 §224 的四个规范名取（`REQUIRED` / `PREFERRED` / `FLEXIBLE` / `UNSPECIFIED`）。
**代价据实写明**：这是本块的**第二个 `RequirementId`**（第一个在 P4，尚未落地），而设计 §4.3.1 逐字说过不定义第二个——**这一处相抵由本计划报出**（`## 遗留` 第一节），不在这里就地改设计。两枚类型在**臂的集合**上一致（`String` 承载的 id ＋ 四臂封闭枚举），取值可以一一对应，跨层由装配方在边界处转换；**但「同形」在 `RequirementClass` 上不成立**：P4 计划的那一枚带声明序（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:1303` 的用例逐字断言 `UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`），本块的没有（Task 5 不给 `Ord`、无该用例），**故跨层只转值、不转序**。

### 三、哪些 task 在 P4 未合入前动不了

- **Task 11（装配与收尾）依赖「P4 的改动已合入」这件事**，而不是某个接口：它要改 `crates/continuum-runtime/src/main.rs`（`runtime_migrations()`，`:98`）、`Cargo.toml`（members）、`crates/continuum-runtime/Cargo.toml`、`crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED`）、`crates/continuum-runtime/tests/migrations.rs`（`expected_migrations()`）、`crates/continuum-runtime/tests/startup.rs`（两处「迁移应用 N 项」）——**这六个文件里有五个是 P4 的 Task 20／21 正在改的多写者单文件**。
  **两重后果**：（i）冲突落在 `ALLOWED` 与迁移清单这两处逐行敏感的文本上；（ii）`startup.rs` 与 `migrations.rs` 的计数断言会互相踩——**计数断言的判别力依赖「作者按实际输出改断言」**（`crates/continuum-runtime/tests/migrations.rs:1-10` 已写明这一点），两个改动方各按自己那一刻的输出改，合起来谁也说不清哪个数是对的。
  **故排期上本块的 Task 11 排在 P4 合入之后**；Task 1 对 members 与 `ALLOWED` 的那一次改动同理（它是本块的第二处多写者改动）。
- **其余九个 task 只新建 `continuum-verify` 与它自己的测试，一个既有文件都不碰**（唯一例外是 Task 1 改 members 与 `ALLOWED` 加**空条目**——`every_crate_depends_only_on_its_allowed_set`（`:288`）与它的互为覆盖断言（`:291-305`）要求**每个** workspace 成员都在表里，漏了即红；以及 Task 4 改 `crates/continuum-graph/src/node.rs:19` 的一行注记——P4 的计划明写不碰 `crates/continuum-graph/**`，故那一行不与 P4 相撞）。
- **推论：整块排在 P4 合入之后。** 上面点名的两处是**动到既有文件的**两个 task，但可开工时刻不由改动面决定：Task 2–10 **全部**依赖 Task 1 建出的 `continuum-verify`（Task 1 才把它加进 workspace `members`），故 Task 1 一推迟，其余九个 task 一个都开不了。**「其余九个 task 一个既有文件都不碰」讲的是它们的改动面，不是它们可以先做。**
- **计数与迁移号一律以「当时工作树上的实际输出」为准，不许预判**：本计划给的是**预期值**（`startup.rs` 今天两处 `迁移应用 9 项`（`:24`、`:90`）与一处 `7 项`（`:79`），加本块一条后各加一），落地时以实跑读出的数字为准。

### 四、本块对既有之物的请求（**本计划只作消费方／依赖方写，一处都不实施**）

| 处 | 请求的内容（照录设计，含会直接报错或静默失效的坑） | 本块的形态 | 收件人 |
|---|---|---|---|
| `adfir_edge` 无主键，且 `Edge` 的四个端点里两个是端口（`crates/continuum-graph/src/edge.rs:47-54`） | §10.2 第 3 条的不变量（一行 `evidence` 对应图上一枚 `EdgeKind::Evidence` 边）**今天无法做成外键**，只能落成「写行时同事务核对图上存在该边，缺则 `Err`」；「EVIDENCE 边是否需要可寻址的身份」是设计 §14 第 6 条 (ii) 提给执行层的对账条目 | **消费方**：本块读 `AdfirGraph::edges()` 做核对，**不另造验证关系边**（切分 §二） | 执行层（P1）＋ 协调者 |
| 执行器的路由：`crates/continuum-graph/src/state.rs:30-31` 允许 `Verifying → Completed` | 本块**不产出许可它的判定**（`CompletionVerdict` 无「完成」臂，§8.2）。`Blocked` 与 `Undetermined` 各自路由到哪里（节点 `FAILED`？重试？升 `Decision`？）须由执行器／协调者指认（设计 §14 第 6 条 (iii)） | **依赖方**：本块只交付两个返回值 | 执行层（P1）＋ 协调者 |
| 第 3 层未建 ⇒ 本块的判定**今天没有生产调用方** | 本块的全部照片都是**单元与集成层**（直接构造输入、断言返回值）；端到端（一个节点从 `RUNNING` 经 `VERIFYING` 走到某处）拍不出来，切分 §四第 4 条也禁止 P5 自建执行路径 | **交付物**：判定函数与它们的用例 | 执行器的接线 task ＋ 协调者 |

**三处的共同点**：本块要的都不是别人的**接口**，而是「别人的表与类型能不能承载本块已经交付的事实」。故本计划**不改、不重述、不预演**它们的任何接口。

### 五、本节把什么交给谁

- **交给执行器（执行层，未建）**：`Evidence::from_tool_result` 的**调用**（§4.4：工具结果先转成 `Evidence`）、`InitialVerifierInput::assemble` 的装配、`record_verification_round` 的调用，以及 `Blocked` / `Undetermined` 的路由。本块产出判定与写入路径，**不产出节点状态迁移**。
- **交给 P5c / P5d / P5e / P5f（四个领域算子块）**：**协调者 2026-10-10 的裁定**——四块**一律不写域包装**，证据由宿主的执行代码直接调本块的 `from_tool_result`（`subject` 取 `Unattached`）。四块的产出面（算子 → `EvidenceType` 的对应）是**声明**。四块对 `continuum-verify` 的生产代码调用为零，故它们各自的边按「边别按『谁在用』判」为 **dev 边**。
- **交给第 7 层（长期循环，未建）**：`EvidenceId` 与只追加的证据表（§10）——**不提供** `ProductReadiness` 本身（§232 的 `value + evidence + last_verified` 要它）。**`Evidence` 的读取路径本块不建**（设计未给消费者，见 `## 遗留`）。
- **交给规范维护者（本项目无此角色）**：设计 §15 的**十八**条以「收件人：规范维护者」或其组合结尾；**本计划一条都不发明**，逐条照抄在 `## 遗留`。
- **交给协调者**：本计划查出的设计问题见 `## 遗留` 第一节——**本计划自定形状与取值**见第二节。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划新增的 workspace 内边只有这些**（逐条由需要它的 task 登记，**一次不声明齐**）：
  - Task 1：`("continuum-verify", &[])`——**空数组**（本 task 对内部 crate 的引用为零，条目不能省，`:291-305` 要求每个成员都在表里）；
  - Task 2：`continuum-verify ← continuum-artifact, continuum-graph, continuum-operator`（`ArtifactId`、`NodeId`、`BackendId`）；
  - Task 9：再加 `continuum-persist`（迁移 130 与写入路径经 `Tx`）与 `continuum-events`（写 `VerificationFailed`）；
  - Task 11：`continuum-runtime ← continuum-verify`（装配迁移）。
  **上表之外一条边都不登记**——尤其**没有指向 `continuum-semantics`（会成环，见跨计划前置第二节）**、`continuum-core`、`continuum-port`、`continuum-policy`、`continuum-effect`、`continuum-capability`、`continuum-model-registry`、`continuum-connector`、`continuum-provider`、`continuum-workspace` 的边。
  **外部 crate（`serde` / `serde_json` / `thiserror` / `trybuild` / `tempfile`）按需加，不进 `ALLOWED`**——那张表只逐对断言 workspace 成员之间的边。
- **`--depth 1` 只钉直接边，这就是这条规则的完整粒度，本计划不另设闭包断言。** `cargo_tree_direct` 带 `--depth 1`（`crates/continuum-runtime/tests/dependency_direction.rs:270-275`），故它**看不到传递可达**；而 Rust 的 crate 可见性要求**直接声明**才能 `use`，未写进 `Cargo.toml` 的传递依赖**写不出** `use continuum_semantics::…`。**写这一段的用途**：免得把上面那条约束读成「任何指向语义层的边都会变红」——**那会是一条假保证**。
- **枚举列的落库编码一律小写、多词以 `_` 连接**，经本 crate 的显式 `*_str` / `parse_*` 辅助函数读写（与类型同址），**不依赖 serde、不用 `Debug`**。**表外取值读回得具体 `Err`**（照 P3D 与 P1 的枚举列纪律：`NodeState` 的 serde 是 `SCREAMING_SNAKE_CASE`，直接反序列化会失败）。本块要编码的列：`EvidenceType`（`evidence.evidence_type`）、`Validity`（`test_validity.verdict`）、`TestValidityCheck`（`test_validity_check.check_name`）、`CheckVerdict`（`test_validity_check.verdict`）、`VerdictAggregate`（`verification_round.aggregate`）、`IndependenceVerdict`（`verification_round.independence`）、`AdversaryOutcome`（`verification_round.strategy_verdict`）。
- **迁移编号取 `130`，取用前现场核对空号**（设计 §10.1 已实测取 130）。**实测占用**（2026-10-10 逐 crate 读 `Migration::new(` 的第一个参数）：`10`（artifact，`crates/continuum-artifact/src/persist.rs:9`）、`20`（graph，`crates/continuum-graph/src/persist.rs:15`）、`30`（workspace，`crates/continuum-workspace/src/persist.rs:22`）、`40`（effect，`crates/continuum-effect/src/persist.rs:28`）、`41`（policy，`crates/continuum-policy/src/persist.rs:31`）、`50`（capability，`crates/continuum-capability/src/persist.rs:54`）、`80`（model-registry，`crates/continuum-model-registry/src/persist.rs:107`），加上 `continuum-persist` 内建的 `1` / `2`（`src/db.rs:29`、`:46`）。**四个号不许误判为空号被占**：`60` 在 `crates/continuum-persist/src/bin/crash-writer.rs:17`（测试夹具），`100`、`101`、`102` 在 `crates/continuum-persist/tests/migrations.rs`（实测分别在 `:45`、`:68`、`:69`）。**四个都在测试／工具的 `Db` 里，不进生产链**（`Db::open_with` 的迁移集由调用方给）。**P5 其余五块与 P6 的档由协调者统一划**（设计 §14 第 9 条），本块只声明自己用的号。
- **迁移 130 的 SQL 在 Task 9 内一次写齐五张表，Task 11 之后不得再改它的 SQL。** 依据两条：（i）设计 §10.2 规定 130 含五张表；（ii）**在 Task 11 之前 130 只被测试库应用过**（`runtime_migrations()` 到 Task 11 才含它），故那时改是安全的；**Task 11 之后改 130 的 SQL 就是设计 §13.2 第 2 条点名的那处静默失效**（`migrate()` 跳过已记录的 version，对已建库完全无效）。**本计划不许在任何 task 里改迁移 1／2／10／20／30／40／41／50／80 的 SQL。**
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。**新增依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不建 `VerificationProfile` 的生成器**（§5.3：生成者待定，造一个等于替 `OPEN-009` 选了一个方向；只冻结取得点，缺档案即 `Err`）。
  - **不建 Test Validity Verifier 的产生者**（§6.5：「谁跑这五个检查、什么时候跑」规范未给；只冻结产出与消费点）。
  - **不建 Verifier backend 的登记形态、不建方法登记形态**（切分 §四第 3 条：在 P5b 过审前任何块不得自造）。
  - **不判「覆盖是否充分」**（§3：OPEN-001 未决；`CoverageVerdict` 没有正臂，`AllRequiredRequirementsVerified` 恒 `Undetermined`）。
  - **不判 Mutation 的质量**（§9.2：`mutation_score` 无量纲、`EvidenceStrength` 无 `Ord`；只冻插槽）。
  - **不定义「高价值任务」、不给 Adversary 触发条件**（§9.1：`B1` 未定义；只冻类型与消费规则）。
  - **不实现 §262 第 4 级的 family 判定**（§7.3：`model` / `provider` 今天恒为 `None`，family 判据不存在）。
  - **不发明 Verifier 不可用时的降级路径**（§7.2 第 4 条：`A5`/`OPEN-004` 未定义；不可用即 `Err`）。
  - **不选 §121 的四种补救里的任何一种**（§7.5：只做检测，把 `AdjudicationRequired` 交回调用方）。
  - **不判 `ContractSatisfied`、不判强制效应完成否、不判测试是否通过**（§8.3：这四项以**值**传入，`None` 即不可判）。
  - **不解析 `claim`、不解析 `strength` / `scope` 的值、不解析 `Intent.completion_predicate` 的语法**（§4.3.2、§4.3.4、§8.4：规范未给语法）。
  - **不解析 `Requirement.verification_requirement`**（P4 设计 §7.2 的「只存不判」在本块延续为「只读不判」）。
  - **不另造验证关系边、不建执行路径、不做 `Queued → Running`、不调 `OperatorRegistry::resolve`**（切分 §二、§四第 4 条）。
  - **不新增 `EventType` / `AuditKind` 变体、不动 `EventType::ALL`**（§10.3：§313 的必录清单里没有验证类事件）。
  - **不写 `IntentCompleted`**（§10.3：本块不产生「完成」）。
  - **不定义 `Result` 的形状**（§8.4：§344 的 `Result` 由调用方据本块的返回值组装）。
  - **不建 `ArtifactType` 的 P5 扩展**（切分 §四第 2 条：只由 P5e 一处改）。
  - **不定义 `VerdictSet`**（设计 §2.2：该名已实测全仓零命中，是本设计删掉的死条目；本块做聚合的是 `VerdictAggregate`）。
  - **不改 `docs/02-工程.md`、不改 `docs/spec/**`、不改设计文档**（规范侧的相抵记在 `## 遗留`）。

### 本计划特有的「签名即判据」，照抄前须对源

- `Migration::new(version: i64, name: &'static str, sql: &'static str)` 与 `Db::open_with(path, Vec<Migration>)`（`crates/continuum-persist/src/db.rs:14`、`:72`）；`Tx::query(&self, sql, params: &[Value]) -> Result<Vec<Vec<Value>>, PersistError>`、`Tx::execute`、`Tx::commit`、`Tx::append_event(&self, event: &Event)`（`src/tx.rs:26`、`:30`、`:41`、`:49`）。
  **本块的写函数一律不自己 `commit`**（照 P1 的 `apply_transition` 形状，`crates/continuum-graph/src/transition_tx.rs:47`；「不自行开启或提交事务，提交由调用方负责」的原文在 `:26`，`Err` 之后须回滚在 `:31`，不代劳回滚在 `:39`）：写入函数返回 `Err` 之后调用方必须回滚、不得提交，Task 9 的失败路径照片以这条为前提。
- `Event::new(event_type: EventType, event_id: impl Into<String>, occurred_at: i64, payload: serde_json::Value) -> Event`，配 `with_intent` / `with_node`（`crates/continuum-events/src/event.rs:81`、`:105`、`:110`）；**`with_ignorable` 只有新增的事件类型才需要置为 true，九类既有事件不得调用**（`:99` 的原话）——`VerificationFailed` **是九类之一，故不调它**。
- `continuum_artifact::load_artifact(tx, id) -> Result<Option<Artifact>, PersistError>`（`crates/continuum-artifact/src/persist.rs:90`）；`Artifact` 的字段（`src/artifact.rs:170-178`：`id` / `artifact_type` / `content_hash` / `size` / `producer_node` / `privacy_class` / `version` / `metadata: Value` / `provenance: Value`）。
  **`Artifact.metadata` 是 `serde_json::Value`，其 schema 规范未给**（设计 §15 第 9 条）——故**本块不许读它的任何字段**：§266 第 1 项的 `artifact_exists` 只判存在，不读内容。
- `AdfirGraph::terminal_nodes() -> &[NodeId]`、`nodes() -> &[Node]`、`edges() -> &[Edge]`、`node(&NodeId) -> Option<&Node>`（`crates/continuum-graph/src/graph.rs:70`、`:160`、`:164`、`:152`）。
  **`AdfirGraph::validate()`（`:87`）不要求 `terminal_nodes` 非空**（实测：它只核 entry / terminal 的出入边）——故「终端集为空」是一个**可达**的输入，Task 9 有一条用例钉它的取值。
- `ExecutionProfile` 的三项（`crates/continuum-graph/src/execution.rs:20-24`）：`pub model: Option<String>`、`pub provider: Option<String>`、`pub backend: Option<BackendId>`。**P1 设计 §14 明写六处字段在 P3 之前恒为 `None`，实测 `crates/continuum-graph/src/execution.rs:15-17` 仍写着这一句**——故 Task 6 的独立性判定今天只能读到 `None`。
- `require` 与 `Value`：`continuum_persist::Value` 是**独立于 `serde_json::Value`** 的库层值类型（`crates/continuum-persist/src/value.rs`）；两者的转换**本 crate 自己做**（照 `crates/continuum-graph/src/persist.rs:102` 的 `Value::text(serde_json::to_string(...))` 与 `:251` 的 `parse_json(&text(&row[5])?)?` 形状）。实现前须先读该文件确认 `Value` 的构造与匹配方式。

---

## 已付过代价的纪律

**本节照 P4 计划的纪律清单逐条对源**（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:181-241`），**只承袭与本块实情相符的那些**；本块与 P4 对不上的一处（变异口径分层）单独申报，见本节末。

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」跑全量是加分。**变异分四档，每一处「预期谁红」都要标档位**：**取反**（把判定反过来）／**放宽**（少判一半条件）／**收紧**（多判一半条件）／**移除**（删掉整条守卫）。**三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN（改之前先 `grep -n` 数锚点，落在唯一处）。**本块的「删一臂」类变异一律以该枚举的 `ALL` 常量数组为锚点**：删数组里的一项 ⇒ 数目断言与逐字点名红（运行期红）；**删枚举定义里的那一臂不是这条的锚点**——用例文件与 `as_str` 里的 `Xxx::Arm` 引用会编译不过，那是编译失败、不是变红（见 (c)）；
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**。本块已预先识别的等价变异体有三处：**`AdfirGraph::edges()` 的索引序**（EVIDENCE 边核对的用例若只断集合相等，「改成不排序」是等价的）、**`Vec<EvidenceType>` 的顺序**（`MissingRequiredTypes.missing` 若只断「非空」，「改成不排序」是等价的——故 Task 5 的 (c) 断**恰是 `[Fuzz]`**）、**`Option::None` 与 `Some(false)` 在夹具里恰好同值**（若夹具漏了「`None` 与 `false` 各一条」，两者可互写）。
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。**本块有三条性质只有编译期照片**（Task 10 的五个 `compile_fail` 样例、Task 4 的字段清单穷尽解构、Task 6 的输入面字段清单），**照实写成「编译期照片」，并写明观测方式是「该样例从编译失败变成编译通过」或「编译不过」**，不假称它们会跑红。
   **变异脚本必须带还原护栏**（本仓出过一次「变异留在源码里」的事故）：每次变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、**每轮用独立日志路径**，报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位」。模板（`MUT` 是改的文件、`BAK` 是备份）：
   ```bash
   cd /home/DslsDZC/Continuum/.worktrees/<本块的工作树>
   before=$(sha256sum "$MUT" | cut -d' ' -f1)
   cp "$MUT" "$BAK"
   trap 'cp "$BAK" "$MUT"; echo "已还原"' EXIT INT TERM
   # …施加变异…
   TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$LOG"
   # …读 $LOG 判红…
   ```
   **`trap` 那一行不许省。**
2. **凡注释写绝对措辞，必须有对应用例；且要带时点**——「唯一／一律／只有／全部／没有任何／逐个」这类词，要么有可写出的照片，要么**明写为什么没有**。**断言的作用域要与事实同宽**：「整张表的唯一 X」≠「某个字段的唯一 X」；「本 crate 的源码文本里零命中」≠「全仓零命中」；「在本 task 结束时」≠「永远」。**枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条臂各要一条照片，不抽代表（手写分支能各自漂移）。**本块适用这条的地方**：`EvidenceType` 的十四臂（Task 1）、十三型「需要制品」的逐型拒绝（Task 2）、五检查逐臂（Task 3）、四 class 逐值（Task 5）、`VerdictAggregate` 五臂（Task 6）、`Conjunct` 十项（Task 7）、`AdversaryOutcome` 两臂（Task 8）。**「唯一产生点」这句话的照片只有编译期那两条**（Task 10），**本计划不写「运行期也证明不了第二个构造点」**。
3. **门读数要自证覆盖**。**判据是日志里 `Running` 的行数与 `test result:` 的行数**（Doc-tests 算一条），**不是** `Compiling` 的行数——`Compiling` 挡不住射程与截断。`cargo test` **不收 `--keep-going`**（那是 `cargo build` / `cargo check` 的），要用 **`--no-fail-fast`**，否则会在第一个失败目标处停住、后面的目标一行不跑。**每轮变异与每次全量门都要留日志路径**，报告里附「`Running` 行数 / `test result:` 行数 / 与上一轮的差」。**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」；**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。
4. **失败路径的用例要断言是哪一种 `Err`**，不只「返回了 `Err`」；**并断言没有半写的副作用。** 本块的失败面是 `VerifyError` 的变体，**逐变体至少一条用例**：`EvidenceWithoutArtifact`（Task 2 的十三型逐型）、`HumanConfirmationFromNonHuman`（Task 2）、`IncompleteTestValidityChecks`（Task 3，缺臂与重复臂各一条）、`UnknownVerificationPolicy`（Task 4 的四种输入逐条）、`ProfileMissing`（Task 4）、`NoAvailableVerifier` 与 `NoIndependentVerifier`（Task 6 各两条）、`EvidenceFromVerifiedNode`（Task 6 与它的 (n) 另一侧）、`MissingEvidenceEdge`（Task 9 与它的另一侧）。**副作用面**：Task 9 的 `an_evidence_row_is_insert_only`（第二次写不得改动原行）与 `record_verification_round` 的失败路径（`Err` 之后由调用方回滚，见「签名即判据」那一条）。**本块不写「事务已回滚」的运行期照片**——`Tx` 没有保存点接口、回滚只能由调用方丢弃句柄（`transition_tx.rs:39-40` 的同一条），这一点在 Task 9 的文档注释里写明。
5. **纯数据／纯投影的类型没有运行期照片，只钉类型与签名（编译期）；钉的机制写死为「编译器强制的穷尽解构／结构体字面量」且不得带 `..`。** 判据：**Rust 没有反射**——若「把字段名各列一遍」被理解成手写两份名单，那条断言恒真（名单是手抄的，类型改了它不会红），它钉的只是「抄的时候两边一样」。**`..` 豁免会让变异成为等价变异体**（第 1 条 (b) 的同一回事）：在被改处留了 `..` 的穷尽解构照过。**本块适用这条的地方**：`VerificationProfile` 的六字段（Task 4 的 `the_six_fields_of_261_are_all_present`）与 `InitialVerifierInput` 的三类输入（Task 6 的 `the_initial_input_names_exactly_the_three_inputs_of_263`）——两处用**穷尽解构**，都写了「不带 `..`」。`Evidence` 的八字段走的是**另一条编译期通道**（结构体字面量与缺失的 `Default` 实现，即 Task 10 的两份 `compile_fail` 样例），不涉及解构，故「`..` 豁免」那一句对它不适用。
6. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本计划逐处标出「**另一侧**」是哪一个用例：Task 2 的 `human_confirmation_needs_no_artifact` 与 `a_human_producer_on_another_type_is_accepted`、Task 3 的 `five_passes_give_valid` 与 `a_record_with_a_repeated_check_is_rejected`、Task 4 的 `a_well_formed_policy_parses_field_by_field` 与 `a_present_profile_is_returned_field_by_field`、Task 5 的 (d)／(f)、Task 6 的 `the_highest_available_level_wins` 的下半与 (n)、Task 7 的 (s) 第二条、Task 9 的 `the_producer_node_of_human_evidence_is_null_and_is_not_edge_checked` 与 `a_failed_round_writes_…`。
7. **「删掉 X 即红」要先问「删掉之后行为真的变了吗」**——索引序、稳定排序、可推断的字面量、`..` 豁免都会让它成为**等价变异体**（见第 1 条）。凡本计划标了「移除档」的地方，都已先答过这一问。

**变异口径分层：本块取收紧的一档，据实申报偏离。** P4 模板把变异分两档——承重守卫跑全量套件，其余分支跑受影响 crate 的包级套件并**标明证据强度较低**（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:238-241`）。**本块不用那一档**：第 1 条与 Task 11 Step 5 一律要求全量 `cargo test --workspace --no-fail-fast`。**这是收紧，不触任何规则**；**代价**：每轮变异更慢；**理由**：本块的承重守卫里有若干条只在跨 crate 处可见（迁移编号与 `startup.rs` 的计数、`ALLOWED` 与实际依赖一致、`EvidenceType` 的新臂对 `continuum-events` 侧名表的影响），包级套件看不到这些观察点，而它们正是本块要钉的那几条。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。**变异日志是证据，必须活到复审结束**：实现者保留 `.tmp/`，由协调者在复审结束后清理。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；凡与既有 crate 交互的签名（`Tx::query` / `Tx::execute` / `Value` / `Migration::new` / `Event::new` / `load_artifact` / `AdfirGraph` 的读法），实现前须先读该 crate 的源码确认，不符时以源码为准并回报。

**本计划特有的三处「照抄前须对源」**：

- `Migration.name` 与 `sql` 是 `&'static str`（`db.rs:14`）；`sql` 里多条语句以 `;` 分隔一次执行（照 `p1_graph_migrations` 的形状）。
- `Value` 与 `serde_json::Value` 是两个类型（`crates/continuum-persist/src/value.rs`）；`evidence.strength` / `scope` 两列按 §10.2 是 `TEXT/JSON`，写入走 `Value::text(serde_json::to_string(..))`、读出走同一个 `parse_json` 形状（照 `crates/continuum-graph/src/persist.rs:102`、`:251`）。
- **`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；而且逐份各不相同，不许拿一个值套三处。** 本仓已为此付过代价（P3D 的计划给三份样例共用一个错误码 `E0603`，实测 rustc 1.95 下三份各不相同）。**另一条坐标**：`trybuild` **按文件主干配对**（`foo.rs` ↔ `foo.stderr`），**且样例里的注释也是照片坐标**——改样例文件头会推移行号，而 trybuild 不规范化行号，故**`.stderr` 一旦接受，就不再改那份 `.rs` 的注释**（要改就重跑并把新 `.stderr` 一起提交）。

---

# 文件结构

```
crates/continuum-verify/
  Cargo.toml
  src/lib.rs            导出面与 crate 文档（逐任务的导出增量加）
  src/evidence_type.rs  EvidenceType（14 臂 + ALL + as_str / parse）
  src/error.rs          VerifyError（逐任务的变体增量加）
  src/evidence.rs       EvidenceId、EvidenceSubject、Claim、WorkerCompletionDeclaration、
                        EvidenceProducer、EvidenceStrength、EvidenceScope、Evidence、
                        Evidence::from_tool_result
  src/test_validity.rs  TestValidityCheck、CheckVerdict、Validity、TestValidity（含聚合与五臂齐）
  src/verifier_level.rs VerifierLevel（5 臂 + ALL + Ord）
  src/policy.rs         VerificationPolicy、VerificationPolicy::from_value
  src/profile.rs        VerificationProfile、VerificationProfileSource、profile_for
  src/contract_view.rs  RequirementId、RequirementClass、ContractRequirement、ContractView
  src/coverage.rs       RequirementCoverage、CoverageVerdict、IncompleteReason、counts、coverage
  src/selection.rs      AvailableVerifier、WorkerIdentity、VerifierSelection、independence_of、select_verifier
  src/verdict.rs        VerifierVerdict、VerdictAggregate、BlindVerdict、clamp_by_readability
  src/input.rs          InitialVerifierInput（含 assemble）、PostBlindVerifierInput
  src/conjunct.rs       Conjunct（10 臂 + ALL）
  src/completion.rs     CompletionVerdict、CompletionInput、judge_completion
  src/adversary.rs      AdversaryOutcome（2 臂 + ALL + 编码）
  src/persist.rs        p5a_verify_migrations()（迁移 130，五张表）、insert_evidence、
                        insert_test_validity、record_verification_round、artifact_exists
  tests/evidence_type.rs        十四臂的编码与数目
  tests/evidence.rs             唯一产生点的两侧、十三型逐臂
  tests/test_validity.rs        五检查、聚合三档、五臂齐
  tests/policy.rs               策略两侧、档案两侧、五级序
  tests/coverage.rs             迭代域、计数五条规则、两臂
  tests/verifier.rs             选择、隔离、两个输入面、聚合五臂
  tests/completion.rs           十项合取、`None` vs `false`、`NotEstablished` 那一臂
  tests/adversary.rs            两臂与消费规则
  tests/persist.rs              只追加、列编码、事件同事务、artifact_exists
  tests/type_level.rs           trybuild 驱动 + `LISTED_SAMPLES` 与目录对钉
  tests/compile_fail/*.rs       不可构造性样例（各配同名 .stderr）
```

**本计划要改的既有文件**（只有 Task 1、Task 4、Task 11 碰它们）

```
Cargo.toml（workspace）                                  members 加一行（Task 1）
crates/continuum-graph/src/node.rs                       只改 :19 的一行注记（Task 4）
crates/continuum-runtime/Cargo.toml                      加一条依赖边（Task 11）
crates/continuum-runtime/src/main.rs                     runtime_migrations() 追加一行（Task 11）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED：continuum-verify 条目（Task 1 起逐 task 补边）＋ runtime 条目（Task 11）
crates/continuum-runtime/tests/migrations.rs             expected_migrations() 同步一行（Task 11）
crates/continuum-runtime/tests/startup.rs                两处「迁移应用 N 项」（Task 11）
```

**本计划不碰的文件**：`crates/continuum-graph/**`（除 `node.rs:19` 那一行）、`crates/continuum-artifact/**`、
`crates/continuum-events/**`、`crates/continuum-operator/**`、`crates/continuum-persist/**`、
`crates/continuum-port/**`、`crates/continuum-policy/**`、`crates/continuum-effect/**`、
`crates/continuum-capability/**`、`crates/continuum-model-registry/**`、`crates/continuum-connector/**`、
`crates/continuum-provider/**`、`crates/continuum-workspace/**`、`docs/02-工程.md`、`docs/spec/**`、
`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`、`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`。

---

### Task 1: `continuum-verify` 骨架与 `EvidenceType`（十四臂）

**Files:**
- Create: `crates/continuum-verify/Cargo.toml`
- Create: `crates/continuum-verify/src/{lib.rs,evidence_type.rs}`
- Create: `crates/continuum-verify/tests/evidence_type.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加一行 `"crates/continuum-verify"`，接在现有条目之后）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED`（`:25`）加**空条目**）

**Interfaces:**
- Produces: `continuum_verify::EvidenceType`

**本 task 不建 `error.rs`**：`EvidenceType::parse` 与 `ArtifactType::parse` 同形，返回 `Option`，本 task 不需要错误类型；`VerifyError` 由第一个真的返回 `Result` 的 task（Task 2）建。**「叶子 crate 的条目记实际依赖」的同一条口径也用在类型上**（不为将来可能用到而先建）。

- [ ] **Step 1: 建 crate 骨架并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`serde = { workspace = true }`（与 `ArtifactType` 同形的 `#[derive(Serialize, Deserialize)]` ＋ `#[serde(rename_all = "snake_case")]`，见 `crates/continuum-artifact/src/artifact.rs:12-14`）。**`as_str` / `parse` 是列编码的唯一产生点，serde 派生不用于落库**（沿用 `ArtifactType::as_str` 文档里那条口径）。`serde_json` / `thiserror` / `trybuild` / `tempfile` 与三条内部依赖**由需要它们的 task 增量加**。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加（**只列本 task 实际有的边**：本 task 对内部 crate 的引用为零，故是空条目；它不能省——`:291-305` 的互为覆盖断言要求每个 workspace 成员都在表里）：

```rust
    // P5a 验证与证据判定（设计 §2.1）。本块是第 8 层，只依赖执行层与其下：
    // continuum-artifact / continuum-graph / continuum-operator（Task 2 加）、
    // continuum-persist / continuum-events（Task 9 加）。
    // **不含 continuum-semantics**：`语义层 (2) → 执行层 (3) → 跨领域 (8)` 已在链上，
    // 反向登记会闭成 2-环，与工程 §9.1「依赖方向单向，无环」相抵（跨计划前置第二节）。
    ("continuum-verify", &[]),
```

**与设计 §2.1 的一处偏离（据实写明）**：设计 §2.1（`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md:87`）写「新 crate 会**先让该门变红再被补齐**（`:288` 的用例），这是**刻意的**」。本计划把「建 crate」与「加空条目」放在同一步，**故那次红在本计划里不发生**——本块对这条门的可见形态是「条目与实际依赖逐对相等」（Task 11 Step 3 (f)），不是「先红一次」。**代价**：少一次「漏登记即红」的自然演示；**收益**：Task 1 的提交不会在序列里留下一个已知变红的门。

- [ ] **Step 2: 写用例**

`tests/evidence_type.rs`（设计 §4.2 的十四臂与它的编码）：

- `the_type_list_names_the_fourteen_arms_in_one_place`：`assert_eq!(EvidenceType::ALL.len(), 14, ...)`，并把 **`ALL` 的十四项逐字列出**（`assert_eq!(EvidenceType::ALL, [EvidenceType::Test, EvidenceType::Fuzz, …])`——十四臂一项不抽代表）。红条件：删掉 `ALL` 里一臂（**移除档**）⇒ 数目断言与逐字比对同时红。**不许写成 `EvidenceType::ALL.len()` 与另一个从 `ALL` 派生的量比较**（那是恒真的假照片，见设计 §12.2 的「一条预防」）。
- `every_evidence_type_round_trips_through_its_encoding`：遍历 `ALL`，逐型断言 `parse(t.as_str()) == Some(t)`（与 `crates/continuum-artifact/tests/artifact_type.rs` 同形）。红条件：改 `as_str` 里那一型的那一行，**或**改 `parse` 里那一型的那个字面量（**取反档**；两处各自是唯一锚点，不共用）⇒ 该型那一条红。
- `the_three_arms_reported_by_the_domain_blocks_carry_their_designated_encodings`：**逐枚一条**（不抽代表）——`ModelReview → "model_review"`、`ContradictionCheck → "contradiction_check"`、`CitationVerification → "citation_verification"`（设计 §4.2 末三行）。红条件：改其中一枚的串（**取反档**）⇒ 该条红。**这一条单独存在**是因为这三臂的出处与前十一臂不是同一种（领域算子块报出），串名是**本设计的命名**，不是规范逐字。
- `an_out_of_table_string_is_rejected`：`parse("unknown_evidence") == None`、`parse("Test") == None`（编码一律小写）、`parse("") == None`。红条件：`parse` 末支返回某个默认型而不是 `None`（**放宽档**）⇒ 三条全红。
- **这份清单要求「每一枚枚举的每一臂」都有数目断言**（设计 §12.1 首段）：本枚是十四臂，上面第一条即它的数目断言。**每个后续 task 建一枚枚举时都要照办**——本计划在 Task 3（`TestValidityCheck` 5 / `CheckVerdict` 3 / `Validity` 3）、Task 4（`VerifierLevel` 5）、Task 5（`CoverageVerdict` 2 / `IncompleteReason` 2 / `RequirementClass` 4）、Task 6（`VerifierVerdict` 3 / `VerdictAggregate` 5 / `IndependenceVerdict` 3 / `BlindVerdict` 3）、Task 7（`Conjunct` 10 / `CompletionVerdict` 2）、Task 8（`AdversaryOutcome` 2）各写一条。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test evidence_type
```

**预期形态是编译失败**：`src/evidence_type.rs` 的 `EvidenceType` 要 Step 4 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「**确认编译失败**」。

- [ ] **Step 4: 实现**

```rust
/// §258 的 `type` 字段（字段名不叫 `type`：Rust 保留字；与 `Artifact.artifact_type` 同形）。
/// **封闭枚举**：`VerificationProfile.required_evidence_types[]` 与产出的证据之间要做集合包含判定，
/// 开放字符串会把「必需类型齐了吗」退化成字符串比较，而漏判的方向是「以为齐了」（设计 §4.2）。
pub enum EvidenceType {
    Test, Fuzz, Property, FormalProof, StaticAnalysis, Benchmark, VisualCheck,
    MetadataCheck, HumanConfirmation, Differential, Metamorphic,
    ModelReview, ContradictionCheck, CitationVerification,
}

impl EvidenceType {
    /// 已实测的出处有十四行（设计 §4.2 的表）。**本常量带数目字面量**，
    /// 与 `ArtifactType::ALL`（`crates/continuum-artifact/src/artifact.rs:28`）同形。
    /// **不写「只有这十四臂」**：§258 的九臂前缀是「例如」，§193 的六种方式之外规范没有穷尽声明。
    pub const ALL: [EvidenceType; 14] = [ /* 十四项 */ ];

    /// §258 各型的字符串名：小写、多词以 `_` 连接。**穷尽 `match`、无通配臂**——加臂时编译不过。
    pub fn as_str(&self) -> &'static str { /* … */ }

    /// **严格逆**：表外字符串一律 `None`（不取默认型——理由同
    /// `crates/continuum-artifact/src/artifact.rs:61-63` 的注释）。
    pub fn parse(s: &str) -> Option<Self> { /* … `_ => return None` */ }
}
```

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test evidence_type
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(verify): crate 骨架与 EvidenceType 的十四臂"
```

---

### Task 2: `Evidence` 数据模型与唯一产生点 `from_tool_result`（§4/§89）

**Files:**
- Create: `crates/continuum-verify/src/{evidence.rs,error.rs,contract_view.rs}`（`contract_view.rs` 本 task 内**只有 `RequirementId` 一枚**——它是 `EvidenceSubject::Requirement` 的载荷；`RequirementClass` / `ContractView` 在 Task 5 追加到同一文件）
- Create: `crates/continuum-verify/tests/evidence.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,Cargo.toml}`（导出；加 `continuum-artifact` / `continuum-graph` / `continuum-operator` / `thiserror`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 的 `continuum-verify` 条目由 `&[]` 改为三条）

**Interfaces:**
- Consumes: Task 1 的 `EvidenceType`；`continuum_artifact::ArtifactId`；`continuum_graph::NodeId`；`continuum_operator::BackendId`
- Produces: `continuum_verify::{EvidenceId, RequirementId, EvidenceSubject, Claim, WorkerCompletionDeclaration, EvidenceProducer, EvidenceStrength, EvidenceScope, Evidence, VerifyError}`、`Evidence::from_tool_result`

**本 task 定的两处形状（设计只给语义，不给构造规则）：**

- **`Evidence` 的读取面**：八个字段全私有（设计 §4.4 的「唯一产生点」是结构事实），故**每个字段配一个只读访问器**（`id` / `evidence_type` / `subject` / `claim` / `producer` / `artifact_refs` / `strength` / `scope`）。**代价**：多八个公开读者；**收益**：不必把字段开成 `pub`（那会与 §4.4 相抵）。
- **`Evidence` 的派生清单**：允许 `Debug` / `Clone` / `PartialEq`（它们不产生构造路径）；**禁止 `Default`、`From`、`FromStr`，且禁止 `Serialize` / `Deserialize`**——**`Deserialize` 就是第二个构造点**，加它等于把「唯一产生点」这句话变假。设计 §4.4 只点名了前三个，**`Deserialize` 这一枚由本计划补上**（`## 遗留` 第二节）。

- [ ] **Step 1: 写用例**

`tests/evidence.rs`（设计 §4.4 的拒绝表与 §4.3 的三处形状）：

- `a_type_that_reads_an_artifact_is_rejected_when_artifact_refs_is_empty`：**十三型逐型各一条，不抽代表**——对 `{Test, Fuzz, Property, FormalProof, StaticAnalysis, Benchmark, VisualCheck, MetadataCheck, Differential, Metamorphic, ModelReview, ContradictionCheck, CitationVerification}` 各构造一次（`artifact_refs: vec![]`，`producer: Node{..}`）⇒ 每条 `Err(VerifyError::EvidenceWithoutArtifact)`。红条件：把那条穷尽 `match` 里的某一型挪到「不需要制品」支（**取反档**）⇒ 该型那一条红。**这一条实现成对 `evidence_type` 的穷尽 `match`（无通配臂）**——加臂时**编译不过**，**那是编译期照片、不是运行期红**，这一句要写进用例注释。
- `human_confirmation_needs_no_artifact`：`HumanConfirmation` ＋ 空 `artifact_refs` ＋ `producer: Human` ⇒ `Ok`。**这是上一条的对照侧**（只钉拒绝那侧会让「一律拒绝」漂过去）。
- `human_confirmation_from_a_node_is_rejected`：`HumanConfirmation` ＋ `producer: Node{..}` ⇒ `Err(VerifyError::HumanConfirmationFromNonHuman)`。红条件：删掉该检查（**移除档**）⇒ 红。
- `a_human_producer_on_another_type_is_accepted`：`Test` ＋ `producer: Human` ＋ 一件制品 ⇒ `Ok`。**另一侧**：上一条不许被实现成「`producer` 必须是 `Node`」。
- `the_two_subjects_are_distinguishable`：`EvidenceSubject::Requirement(id)` 与 `EvidenceSubject::Unattached` 各构造一条，断言读回的 `subject` 是哪一臂（穷尽 `match`，不抽代表）。红条件：把 `from_tool_result` 里构 `Evidence` 的**那一处 `subject` 字段初始化**改成恒写 `EvidenceSubject::Unattached`（**取反档**）⇒ 本条红。**锚点是唯一的**：`Evidence` 只有一个构造点，八个私有字段各只在那处初始化一次。**改枚举定义那一处不是本条锚点**——用例文件里的 `EvidenceSubject::Unattached` 引用会编译不过，那是编译失败、不是变红（纪律 1(c)）。**理由**：失配的方向是静默漏配（一条证据谁都不要，覆盖判定看到空集合，把「有证据但没挂上」误判成「没证据」）。
- `the_eight_accessors_read_back_what_was_constructed`：一次构造，八个访问器逐字段断言。**这条是编译期照片**（删掉某个访问器即编译不过），**不是运行期红**——照实写。
- `strength_and_scope_are_opaque_values`：`EvidenceStrength` / `EvidenceScope` 只有 `as_value` / `from_value`，**逐枚断言 `as_value()` 读回构造时给的那个 `serde_json::Value`**（含一个非标量值）。**并写明限度**：把某枚改成命名档枚举（例如加了 `Ord` 或阈值）会让本用例**编译不过**——**编译不过不算变红**（纪律 1(c)），故本条的守卫是**运行期往返**，它钉的是「取值域不由本块封闭」（设计 §4.3.4）。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test evidence
```

**预期形态是编译失败**：`src/evidence.rs` 与 `src/error.rs` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
pub struct EvidenceId(String);          // 不透明 newtype，只有 as_str / from_str（照 P4 对 id 类类型的处置）
pub struct Claim(String);               // §258 的 `claim`。**不解析**：规范未给语法。

/// §29 的「Worker reasoning summary / Worker 自己的完成声明」。
/// **它不是一个 `Evidence`，且没有任何到 `Evidence` 的转换**——
/// §263 禁止它进入初判输入，若它能变成 Evidence，那条禁止就没有落点。
/// **可以构造**（§29 的「之后才比较」需要它）；唯一消费者是 `PostBlindVerifierInput`（Task 6）。
pub struct WorkerCompletionDeclaration(String);

pub enum EvidenceSubject { Requirement(RequirementId), Unattached }
pub enum EvidenceProducer { Node { node: NodeId, backend: BackendId }, Human }
pub struct EvidenceStrength(serde_json::Value);   // §258：只有 as_value / from_value，**无 Ord、无阈值、无命名档**
pub struct EvidenceScope(serde_json::Value);      // 同上

/// §258 的八个字段：全部私有、无 `Default`、无 `From` / `FromStr`、**不派生 `Serialize` / `Deserialize`**。
pub struct Evidence { /* 私有字段八项 */ }

impl Evidence {
    /// §89：工具结果转成 `Evidence` 的**唯一**构造点。
    /// 拒绝两条：（1）十三型需要制品的证据而 `artifact_refs` 为空；
    /// （2）`HumanConfirmation` 而 `producer != Human`。
    /// **不检查**：`claim` 的内容、`strength` / `scope` 的值、`artifact_refs` 里制品是否存在
    /// （那要查库，属于 `artifact_exists`，Task 9）、生产者的节点是否真属该图（在 Task 6 的装配处查）。
    pub fn from_tool_result(
        id: EvidenceId, evidence_type: EvidenceType, subject: EvidenceSubject, claim: Claim,
        producer: EvidenceProducer, artifact_refs: Vec<ArtifactId>,
        strength: EvidenceStrength, scope: EvidenceScope,
    ) -> Result<Evidence, VerifyError>;
    // 八个只读访问器
}

pub enum VerifyError {
    /// `evidence_type` 在读制品的十三型内而 `artifact_refs` 为空（§4.4 第 1 条）。
    EvidenceWithoutArtifact { evidence_type: EvidenceType },
    /// §258 的 `HUMAN_CONFIRMATION` 与生产者不匹配（§4.4 第 2 条）。
    HumanConfirmationFromNonHuman,
    // 后续 task 逐条增量加变体
}
```

`EvidenceSubject::Requirement(RequirementId)` 里的 `RequirementId` **由本 task 定义**，落在 `src/contract_view.rs`（Task 5 再往同一文件追加 `RequirementClass` / `ContractRequirement` / `ContractView`）。**排期**：类型按「用它的 task 先建」——本 task 只用它一枚（`String` 承载、`as_str` / `from_str`），另三枚 Task 5 才用得到；Task 5 的 `Files` 行「`RequirementId` 已由 Task 2 落在此文件」是这一处的权威说法，**本 task 不把它推到 Task 5**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test evidence
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(verify): Evidence 数据模型与唯一产生点 from_tool_result"
```

---

### Task 3: Test Validity Verifier 的形状与聚合（§191/§6.4）

**Files:**
- Create: `crates/continuum-verify/src/test_validity.rs`
- Create: `crates/continuum-verify/tests/test_validity.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,src/error.rs}`（导出、加 `IncompleteTestValidityChecks`）

**Interfaces:**
- Consumes: Task 2 的 `EvidenceId` / `VerifyError`
- Produces: `continuum_verify::{TestValidityCheck, CheckVerdict, Validity, TestValidity}`、`TestValidity::new`

**为什么本 task 在覆盖判定之前**：`coverage`（Task 5）的签名收 `&[TestValidity]`，故它的实现体必须先存在（设计 §6.5 的「只定义消费点」）。

- [ ] **Step 1: 写用例**

`tests/test_validity.rs`（设计 §6.4）：

- `one_fail_makes_the_whole_record_invalid`：五条检查里一条 `Fail`、其余 `Pass` ⇒ `verdict == Validity::Invalid`。红条件：把聚合改成多数票、或改成「只要有一条 `Pass` 即 `Valid`」（**取反档**）⇒ 红。
- `five_passes_give_valid`：五条全 `Pass` ⇒ `Valid`。**这是上一条的对照侧**（只钉 `Invalid` 那侧会让「恒 `Invalid`」漂过去）。
- `no_fail_with_an_unknown_gives_unknown`：无 `Fail`，四条 `Pass` ＋ 一条 `Unknown` ⇒ `Validity::Unknown`（**不是 `Invalid`**）。红条件：把 `Unknown` 也算作 `Invalid`（**收紧档**）⇒ 红。**这一条是「更严的读法」被明确否掉的落点**（§6.4：那会把 §123 的正式未知降格成「判为无效」）。
- `a_record_missing_a_check_is_rejected`：只给四条（缺 `MissingBoundary`）⇒ `Err(VerifyError::IncompleteTestValidityChecks { .. })`。红条件：构造点不检查臂数（**移除档**）⇒ 红。
- `a_record_with_a_repeated_check_is_rejected`：**另一侧**——给五条但某臂重复（另一臂缺席）⇒ 同样 `Err`。**理由**：只钉「少于五条」会让「重复补齐五条」漂过去，而那条路把缺臂静默填平。
- `the_three_enums_each_name_their_arms`：`assert_eq!(TestValidityCheck::ALL.len(), 5)`（逐臂点名 §191 的五个检查名与它们的编码串）、`CheckVerdict::ALL.len() == 3`、`Validity::ALL.len() == 3`；三枚各做一次 `parse(as_str(t)) == Some(t)` 往返。红条件：改某一枚的串（**取反档**，锚点取 `as_str` 里那一型的那一行）⇒ 该型那一条红；删该枚举的 **`ALL` 常量数组**里的一臂（**移除档**）⇒ 数目断言与逐字点名红。**「删一臂」的锚点只取 `ALL`**——删枚举定义里那一臂是用例侧的编译失败，不是变红（纪律 1(a)、(c)）。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test test_validity
```

**预期形态是编译失败**：`src/test_validity.rs` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
/// §191 的五个检查，逐条照录成闭合枚举。
pub enum TestValidityCheck {
    RequirementMapping, AssertionStrength, ErrorImplWouldPass, MockBypassesRealPath, MissingBoundary,
}
pub enum CheckVerdict { Pass, Fail, Unknown }
pub enum Validity { Valid, Invalid, Unknown }

/// §191 的判定记录。**有效性是独立的记录，不是 `Evidence` 的一个字段**——
/// 否则「验证者改了被验之物」（设计 §4.1、§6.4）。
pub struct TestValidity { pub evidence: EvidenceId, pub checks: Vec<(TestValidityCheck, CheckVerdict)>, pub verdict: Validity }

impl TestValidity {
    /// 构造点**要求 `checks` 五臂齐且不重复**（缺臂返回 `Err`：这些检查里任何一条都可能是唯一
    /// 发现「测试无效」的那条，静默缺一条等于静默放宽）。**`verdict` 由本构造点按聚合规则算出**，
    /// 不由调用方给——否则「`checks` 与 `verdict` 不一致」这个值在类型上写得出来。
    /// **聚合规则（本设计定，规范未给）**：任一 `Fail` ⇒ `Invalid`；五条全 `Pass` ⇒ `Valid`；
    /// 其余（无 `Fail` 但有 `Unknown`）⇒ `Unknown`。取「任一 `Fail` 即整条失效」的理由是它是
    /// 三档里**更严的一档**：代价是可能多判为无效，而不是漏判。
    pub fn new(evidence: EvidenceId, checks: Vec<(TestValidityCheck, CheckVerdict)>) -> Result<TestValidity, VerifyError>;
}
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test test_validity
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify
git commit -m "feat(verify): Test Validity 的五检查、聚合与五臂齐"
```

---

### Task 4: `VerifierLevel`、`VerificationPolicy` 与 `VerificationProfile`（§5/§236/§261）

**Files:**
- Create: `crates/continuum-verify/src/{verifier_level.rs,policy.rs,profile.rs}`
- Create: `crates/continuum-verify/tests/policy.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,src/error.rs,Cargo.toml}`（导出；加 `UnknownVerificationPolicy` / `ProfileMissing`；加 `serde_json`）
- Modify: `crates/continuum-graph/src/node.rs`（**只改 `:19` 的一行注记**，不改字段类型）

**Interfaces:**
- Consumes: Task 1 的 `EvidenceType`
- Produces: `continuum_verify::{VerifierLevel, VerificationPolicy, VerificationProfile, VerificationProfileSource, profile_for}`

**为什么 `VerifierLevel` 落在本 task**：`VerificationPolicy.min_verifier_level: Option<VerifierLevel>`（§5.1）要用它，而 Task 6 才是消费它的地方——**类型按「用它的 task 先建」**。

**`node.rs:19` 的改动**：现注记是 `/// 结构保存，判定属 P5。`（实测 `crates/continuum-graph/src/node.rs:17-20`：`:17` 是 `execution_policy` 的注记，`:18` 是它的字段，`:19` 是 `verification_policy` 的注记，`:20` 是它的字段）。**补读者名**：`/// 结构保存，判定属 P5：读者是 `continuum-verify` 的 `VerificationPolicy::from_value`。` **字段类型不动**（仍是 `serde_json::Value`），理由是给字段定型会造出 `执行层 (3) ↔ 跨领域 (8)` 的 2-环（设计 §2.3）。

- [ ] **Step 1: 写用例**

`tests/policy.rs`（设计 §5.1 的两侧、§5.2 的六字段、§5.3 的两侧、§262 的顺序）：

- `a_null_policy_is_the_default_policy`（(j) 第一子例）：`VerificationPolicy::from_value(&Value::Null) == Ok(VerificationPolicy::default())`，并逐字段断言 `Default` 的两个字段取值（`min_verifier_level == None`、`required_evidence_types.is_empty()`）。红条件：`Value::Null` 那一支返回 `Err`（**收紧档**）⇒ 红。**并写明这不是「无要求」**：任务级 `VerificationProfile` 仍然适用（Task 5 有一条用例从下游钉这一点）。
- `a_non_empty_unrecognized_policy_is_rejected`（(j) 第二子例，**四种输入逐条**）：缺字段、多出未定义字段、`min_verifier_level` 是五级序之外的串、`required_evidence_types` 含未知类型名 ⇒ **各一条** `Err(VerifyError::UnknownVerificationPolicy { .. })`。红条件：未知分支返回 `Default` 而不是 `Err`（**放宽档**）⇒ 四条全红。**这一档若不红，说明只拍了空策略那一侧**（设计 §12.2 的预告）。
- `a_well_formed_policy_parses_field_by_field`：**正例**——两个字段各给一个合法值 ⇒ `Ok`，且逐字段断言读回值。**没有这一条，上一条可以由「一律 `Err`」满足**。
- `an_absent_profile_is_an_error_not_an_empty_profile`（(i)）：`profile_for(&VerificationProfileSource::Missing) == Err(VerifyError::ProfileMissing)`。红条件：`Missing` 返回「无要求的档案」（`required_evidence_types: vec![]`）而不是 `Err`（**放宽档**）⇒ 红。**这一条钉的是 fail-open 的那一侧**：无档案 ⇒ 无要求 ⇒ 不阻断。
- `a_present_profile_is_returned_field_by_field`：**另一侧**——`Provided(p)` ⇒ `Ok`，六个字段逐字段相等（否则可由「一律 `Err`」满足）。
- `the_six_fields_of_261_are_all_present`：**编译期照片**——按名穷尽解构，**不带 `..`**：
  `let VerificationProfile { required_evidence_types, optional_evidence_types, fuzz_budget, mutation_threshold, property_requirements, independent_verifier_required } = p;`
  红条件：加第七个字段（**移除档**）⇒ **编译不过**（照实写成编译期照片，不是运行期红）。
- `the_five_levels_are_declared_in_the_262_priority_order`：`assert_eq!(VerifierLevel::ALL.len(), 5)`，逐臂点名，并**逐对断言相邻两级的 `<`**（`DeterministicChecker < FormalStaticChecker`、`FormalStaticChecker < IndependentModelVerifier`、`IndependentModelVerifier < CrossFamilyVerifier`、`CrossFamilyVerifier < HumanReview`——**四条，不抽代表**）。红条件：把枚举声明序改一处（**取反档**）⇒ 该对红。**理由**：`Ord` 由声明序派生，声明序即 §262 的优先序，改一次声明序就改一次选择结果。
- `the_four_class_values_each_have_a_case` 的下半（`RequirementClass`）在 Task 5。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test policy
```

**预期形态是编译失败**：`src/{verifier_level.rs,policy.rs,profile.rs}` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
/// §262 的五臂，**声明顺序即优先序**（`Ord` 由声明序派生）。
/// §262 的「可确定性验证 SHOULD 优先于模型判断」是 SHOULD；本块实现成**严格取最高可用级**
/// （把 SHOULD 当 MUST 用）——这是**收紧**，不触 §342 第 1 条，据实记在 `## 遗留`。
pub enum VerifierLevel { DeterministicChecker, FormalStaticChecker, IndependentModelVerifier, CrossFamilyVerifier, HumanReview }

/// §236 的 `verification_policy`。**不是** §261 的 `VerificationProfile`：
/// 后者按**任务**，本型按**节点**（§236 的字段挂在 Node 上）。
#[derive(Default)]
pub struct VerificationPolicy {
    /// §262 的五级序里的**最低可接受级**。`None` = 不限，按 §262 的顺序取最高可用级。
    pub min_verifier_level: Option<VerifierLevel>,
    /// 本节点**额外**要求产出的证据类型（任务级 `VerificationProfile` 之外的要求）。
    pub required_evidence_types: Vec<EvidenceType>,
}

impl VerificationPolicy {
    /// §5.1 的唯一解析点。两侧：`Null` ⇒ `Default`（无限定，**不是「无要求」**）；
    /// **非空但不认识 ⇒ `Err`**（一个写着内容的策略被当成空策略，正是「静默放宽」；
    /// 与 `ArtifactType::parse` 对未知串返回 `None` 同形）。
    pub fn from_value(v: &serde_json::Value) -> Result<VerificationPolicy, VerifyError>;
}

/// §261 的六个字段。`required_evidence_types` 与 `independent_verifier_required` **进判定**；
/// 其余四项**只存不判**（§261 未给它们的用途或量纲）。
/// **不取 `Vec<String>`**：集合包含判定要编译期穷尽（设计 §5.2）。
pub struct VerificationProfile {
    pub required_evidence_types: Vec<EvidenceType>,
    pub optional_evidence_types: Vec<EvidenceType>,
    pub fuzz_budget: Option<serde_json::Value>,
    pub mutation_threshold: Option<serde_json::Value>,
    pub property_requirements: Vec<serde_json::Value>,
    pub independent_verifier_required: bool,
}

/// 取得该任务验证档案的**来源**。**它的形态本轮不决定**（来自 Contract 的哪一字段？
/// 还是另一个输入？取决于 `OPEN-009` 怎么解，设计 §5.3、§14 第 1 条）。
/// 本计划只取「有档案」与「没有档案」两臂——**这两臂是 §5.3 第 3 条要钉的东西**。
pub enum VerificationProfileSource { Provided(VerificationProfile), Missing }

/// **没有档案时返回 `Err(ProfileMissing)`，不返回「无要求」的档案**——
/// 后者会让每项 REQUIRED 的 required 集合为空、覆盖判定无缺项，即
/// 「无档案 ⇒ 无要求 ⇒ 不阻断」，正是 fail-open 的那一侧（设计 §5.3 第 3 条）。
pub fn profile_for(source: &VerificationProfileSource) -> Result<VerificationProfile, VerifyError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test policy
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify crates/continuum-graph/src/node.rs Cargo.lock
git commit -m "feat(verify): VerifierLevel 五级序、VerificationPolicy 的严格解析与 VerificationProfile 的取得点"
```

---

### Task 5: Requirement Coverage（§260 §190 §261 §6.1–§6.3）

**Files:**
- Create: `crates/continuum-verify/src/coverage.rs`
- Modify: `crates/continuum-verify/src/contract_view.rs`（追加 `RequirementClass` / `ContractRequirement` / `ContractView`；`RequirementId` 已由 Task 2 落在此文件）
- Create: `crates/continuum-verify/tests/coverage.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,src/error.rs}`（导出；加 `ProfileMissing` 已在 Task 4，本 task 不加变体）

**Interfaces:**
- Consumes: Task 2 的 `Evidence` / `EvidenceSubject`；Task 3 的 `TestValidity` / `Validity`；Task 4 的 `VerificationProfile`；Task 1 的 `EvidenceType`
- Produces: `continuum_verify::{RequirementClass, ContractRequirement, ContractView, RequirementCoverage, CoverageVerdict, IncompleteReason, counts, coverage}`（`RequirementId` 由 Task 2 产出，本 task 不重复列）

**为什么输入是 `ContractView` 而不是 `TaskContract`**：本块对 Contract 只读 `requirements[].{id, class}` 与 `verification_requirement`（只读不判），故取一个**视图**，其字段以**值**传入——**不给 `continuum-semantics` 登记边，也不造第二个 `TaskContract`**（设计 §6.1、§2.1）。**这是跨计划前置第二节那处相抵的具体形态**：`RequirementId` 与 `RequirementClass` 在本块各有一枚承载类型。

- [ ] **Step 1: 写用例**

`tests/coverage.rs`（设计 §6.1 的迭代域、§6.2 的两臂、§6.3 的五条计数规则）：

- `only_required_requirements_enter_the_result`（§6.1「四种取值各要一条用例」）：四条 requirement（`REQUIRED` / `PREFERRED` / `FLEXIBLE` / `UNSPECIFIED`）各一条 ⇒ 返回表里**只有** `REQUIRED` 那一条。红条件：把迭代域写成「全部 requirement」（**取反档**）⇒ 非 REQUIRED 那三条红。
- `coverage_returns_one_row_per_required_requirement`（(k)）：**手写 3 条 `REQUIRED`** ⇒ `coverage(...)?.len() == 3`，且**逐条**断言 `requirement` 是那三个 id（顺序按传入序）。红条件：迭代域去重（`HashSet`）或漏掉一条（**移除档**）⇒ 红。**期望值必须手写**——若写成 `contract.requirements.len()`，它测的是 Rust 的 `len()`、恒真（设计 §12.2 的「一条预防」）。**这一条是 §342 第 1 条在本块唯一的可执行守卫**。
- `a_required_requirement_with_no_evidence_is_incomplete`（(a)）：零证据 ⇒ `Incomplete { NoValidEvidence }`。
- `a_requirement_with_complete_valid_evidence_is_not_established`（(b)，**本设计最重要的一条**）：一条 `Test` 证据（配 `TestValidity` 五条全 `Pass`）、`profile.required_evidence_types == [Test]` ⇒ **恰好是 `NotEstablished`**（不是 `Incomplete`，也不是任何正臂）。红条件：把返回的那一臂换成 `Incomplete`，或给 `CoverageVerdict` 加一枚正臂（**取反档**）⇒ 红。**这是 §342 第 5 条的照片**：测试通过换不来「满足」。
- `a_missing_required_type_is_incomplete_with_exactly_that_type`（(c)）：`profile.required_evidence_types == [Test, Fuzz]`，证据只到 `Test` ⇒ `Incomplete { MissingRequiredTypes { missing } }`，**且 `missing` 恰是 `[Fuzz]`**。红条件：`missing` 里多列一型、或只断「非空」（**取反档**／等价变异体）⇒ 红。
- `a_present_required_type_is_not_missing`（(d)，**另一侧**）：同上但 `Fuzz` 已在 ⇒ `NotEstablished`。只钉 (c) 会让「只要 profile 非空就恒缺项」漂过去。
- `a_test_evidence_without_a_validity_record_does_not_count`（(e)）：`Test` 证据、无 `TestValidity` 记录 ⇒ 不计数 ⇒ `Incomplete { NoValidEvidence }`。红条件：把「无记录」当计数（**放宽档**）⇒ 红。判据是 §191 的原文「测试不能因为存在于 `test/` 目录里就自动被认为有效」。
- `a_test_evidence_with_a_valid_record_counts`（(f)，**另一侧**）：`Valid` ⇒ 计数 ⇒ `NotEstablished`。红条件：(e) 的反向（**收紧档**）。
- `a_test_evidence_marked_invalid_does_not_count`（§6.3 第 3 条）：`Invalid` ⇒ 不计数 ⇒ `NoValidEvidence`。
- `a_test_evidence_with_unknown_validity_still_counts`（§6.3 第 4 条，**上一条的另一侧**）：`Unknown` ⇒ **计数** ⇒ `NotEstablished`。红条件：把 `Unknown` 归到不计数（**收紧档**）⇒ 红。**这一条就是设计 §12.2 预告的那一档的语料**（「`Unknown` 的检查结果算作不计数 ⇒ b 只在『证据里有 `Unknown` 有效性』的语料上红 ⇒ 该用例须备一份这样的语料」）——**本计划把它做成一条独立用例，(b) 自身不必背这份语料**。(b) 对所有有效性取值都成立。
- `a_non_test_evidence_counts_without_a_validity_record`（§6.3 第 5 条）：一条 `Fuzz` 证据、无任何 `TestValidity` 记录 ⇒ 计数。红条件：把非测试证据也要求有效性记录（**收紧档**）⇒ 红。**判据**：§191 的判据只覆盖测试证据，规范未给非测试证据的「不计数」判据，本设计不发明。
- `evidence_of_another_requirement_does_not_count`（§6.3 第 1 条）：证据 `subject == Requirement(other)` ⇒ 本条 requirement 的计数集为空 ⇒ `NoValidEvidence`。红条件：`counts` 忽略 `subject`、把每条证据都算给每条 REQUIRED（**放宽档**）⇒ 红。
- `unattached_evidence_does_not_count`（**另一侧**）：`subject == Unattached` 的证据同样不计数。**两条都要**：`Unattached` 是 §4.3.1 的显式臂，漏了它会让「挂错 requirement」静默。
- `the_four_class_values_each_round_trip`：`assert_eq!(RequirementClass::ALL.len(), 4)`，四臂逐字点名，并各做一次编码往返（`REQUIRED` / `PREFERRED` / `FLEXIBLE` / `UNSPECIFIED` 照 §224 的四个规范名，编码取小写）。红条件：改某一臂的串（**取反档**）⇒ 该臂的往返红；删 **`RequirementClass::ALL`** 里的一臂（**移除档**）⇒ `len()` 断言与逐字点名红。**「删一臂」的锚点只取 `ALL`**（纪律 1(a)）。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test coverage
```

**预期形态是编译失败**：`src/coverage.rs` 与 `contract_view.rs` 里 Task 5 追加的三枚类型要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
/// §224 的 `Requirement.id` 的**本块承载类型**（跨计划前置第二节）。
pub struct RequirementId(String);

/// §224 的 `class` 四值。本块只做 `match`，**不在本块里判 `class` 的语义**（§6.1）。
pub enum RequirementClass { Required, Preferred, Flexible, Unspecified }

/// 视图里的一条 requirement：只带本块要读的两项（`id` 与 `class`），
/// 加 `verification_requirement` 一项**只读不判**（P4 设计 §7.2 的「只存不判」在本块延续）。
pub struct ContractRequirement { pub id: RequirementId, pub class: RequirementClass, pub verification_requirement: Option<String> }
pub struct ContractView { pub requirements: Vec<ContractRequirement> }

pub struct RequirementCoverage { pub requirement: RequirementId, pub verdict: CoverageVerdict }

/// **只有两臂，没有正臂**（§6.2、§3.2）。`NotEstablished` 这个名字是刻意的：
/// 它不叫 `Unknown`——它说的是**本层没有判据、因而不放行**。
pub enum CoverageVerdict {
    /// **确定的欠缺**。§260 的 `verification = INCOMPLETE` 落在这一臂。
    Incomplete { reason: IncompleteReason },
    /// **没有确定的欠缺，但「充分」无判据**（OPEN-001）。它**不是**「满足」，也不是「不确定的满足」。
    NotEstablished,
}

pub enum IncompleteReason {
    /// §190：该 Requirement 没有有效证据（集合为空，或全部被判 `Invalid`）。
    NoValidEvidence,
    /// §261：`required_evidence_types` 里有类型在本 Requirement 的证据集合里缺席。
    MissingRequiredTypes { missing: Vec<EvidenceType> },
}

/// §6.3 的五条计数规则。**第 1 条是前提，2–4 条互斥且穷尽 `TestValidity` 的三种取值形态，
/// 第 5 条覆盖非测试证据**。
/// **翻转条件（写进实现的文档注释，不留在本计划里）**：这套规则今天只决定 `Incomplete`
/// 与 `NotEstablished` 的分界；一旦 `CoverageVerdict` 加了正臂，**第 2 与第 4 条立即变成
/// 载荷规则**，必须重审。
pub fn counts(e: &Evidence, r: &RequirementId, validity: &[TestValidity]) -> bool;

/// 返回**逐项一条**，与 REQUIRED 项的**数目相同**（§342 第 1 条的唯一可执行守卫）。
pub fn coverage(contract: &ContractView, evidence: &[Evidence], validity: &[TestValidity],
                profile: &VerificationProfile) -> Result<Vec<RequirementCoverage>, VerifyError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test coverage
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify
git commit -m "feat(verify): Requirement Coverage 的逐项判定与计数规则"
```

---

### Task 6: Verifier 的选择、隔离与两个输入面（§7）

**Files:**
- Create: `crates/continuum-verify/src/{selection.rs,verdict.rs,input.rs}`
- Create: `crates/continuum-verify/tests/verifier.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,src/error.rs}`（导出；加 `NoAvailableVerifier` / `NoIndependentVerifier` / `EvidenceFromVerifiedNode`）
- **`Cargo.toml` 本 task 不动**：`InitialVerifierInput` 要的 `continuum_artifact::Artifact` 已在 Task 2 登记（「边由用它的那个 task 登记」的反面同理：**不为将来可能用到而提前登记**；已在表里的也不重复登记）

**Interfaces:**
- Consumes: Task 4 的 `VerifierLevel` / `VerificationPolicy` / `VerificationProfile`；Task 2 的 `Evidence` / `WorkerCompletionDeclaration`；Task 5 的 `ContractView`；`continuum_artifact::Artifact`；`continuum_graph::NodeId`
- Produces: `continuum_verify::{AvailableVerifier, WorkerIdentity, VerifierSelection, IndependenceVerdict, independence_of, select_verifier, VerifierVerdict, VerdictAggregate, BlindVerdict, clamp_by_readability, InitialVerifierInput, PostBlindVerifierInput}`

**本 task 定的两处形状（设计只给语义，不给字段清单）：**

- **`VerifierSelection` 的三个字段**：设计只在 §7.3 点名了 `.independence`。本计划取 `level` / `backend` / `independence` 三项——`level` 与 `backend` 是 §7.2 选择规则的结果本身（「级别内具体选哪个 backend」在 §14 第 4 条被点名），`independence` 是 §7.3 要求「在结果里标注」的那一枚。**代价**：字段集是本计划定的；**收益**：`record_verification_round` 的 `selection(JSON)` 列有可存之物。
- **`clamp_by_readability(readable: bool, raw: VerifierVerdict) -> VerifierVerdict`**：§7.5 末段要求「无物可读时判 `Unknown` 而不是 `Pass`」，但没给承载它的函数名。本计划取一个纯函数，**输入面里「有没有可读制品」由 `InitialVerifierInput::has_readable_artifact` 给出**。**代价**：函数名是本计划定的。

- [ ] **Step 1: 写用例**

`tests/verifier.rs`（设计 §7.2 的四条规则、§7.3 的三组判据、§7.4 的构造性保证、§7.5 的五臂）：

- `the_highest_available_level_wins`：三个候选（1、3、5 级）⇒ 选中 1 级。红条件：取最低（**取反档**）⇒ 红。**另一侧**：只有 3、5 级 ⇒ 选 3 级（否则可由「恒取第 1 级」满足）。
- `a_candidate_below_the_policy_floor_is_dropped`（(l) 第一子例）：`min_verifier_level = Some(FormalStaticChecker)`、只有 5 级候选 ⇒ `Err(VerifyError::NoAvailableVerifier)`。**另一侧**：2 级与 5 级各一 ⇒ 选中 2 级（不误伤）。红条件：不应用下限（**移除档**）⇒ 第一条红。
- `no_independent_candidate_is_an_error_when_independence_is_required`（(l) 第二子例）：`independent_verifier_required = true`，唯一候选与 Worker 同 `backend` ⇒ `Err(VerifyError::NoIndependentVerifier)`。**另一侧**：候选与 Worker 的 `backend` 不同、`model` 与 `provider` 均已知且不同 ⇒ `Ok`。红条件：降级到那个不独立的候选（**放宽档**）⇒ 第一条红。**这两条合起来钉的是「不可用时返回 `Err`，不静默降级」**（§7.2 第 4 条）。
- `a_same_backend_candidate_is_not_independent`（§7.3 判据一）：⇒ `IndependenceVerdict::NotIndependent`。
- `same_model_and_provider_is_not_independent`（§7.3 判据二）：`model` 与 `provider` 两项都已知且都相同 ⇒ `NotIndependent`。
- `an_unknown_identity_is_not_established`（§7.3 判据三，**另一侧**）：任一项为 `None`、且 `independent_verifier_required == false` ⇒ 候选**保留**且 `selection.independence == NotEstablished`（**不是 `NotIndependent`**）。红条件：写成 `NotIndependent`（**取反档**）⇒ 本条红，**且 Task 7 的 (s) 第一条随之红**（这是跨 task 的同一处变异）。
- `an_unknown_identity_is_dropped_when_independence_is_required`（§7.3 的 fail-closed 侧）：`required == true` ＋ 身份含 `None` ＋ 只剩这一个候选 ⇒ `Err(NoIndependentVerifier)`。
- `the_verified_nodes_own_evidence_is_rejected`（(m)）：`assemble` 的 `evidence` 里有一条 `producer == Node { node: verified_node, .. }` ⇒ `Err(VerifyError::EvidenceFromVerifiedNode { .. })`。红条件：删掉该检查（**移除档**）⇒ 红。
- `evidence_from_another_node_is_accepted`（(n)，**另一侧**）：别的节点产的同类证据 ⇒ `Ok`，且 `input.evidence().len()` 等于传入条数。红条件：把检查写成「一律 `Err`」⇒ 红。**前提写清**：本条不退化为「一律 `Err`」靠的是「图上有别的节点产同类证据」这个前提；无别的节点时按 `Err` 走（与本块其余 fail-closed 侧同向）。
- `the_initial_input_names_exactly_the_three_inputs_of_263`：**编译期照片**——按名穷尽解构 `let InitialVerifierInput { contract, artifacts, evidence } = input;`（不带 `..`）。红条件：加第四类字段（**移除档**）⇒ **编译不过**。**这条是 §263 与 §29 在本块的结构性兑现**：字段里没有任何一个装得下 Worker 的完成声明或完整推理。
- `a_level_one_check_without_a_readable_artifact_is_unknown_not_pass`（(o)）：`clamp_by_readability(false, VerifierVerdict::Pass) == VerifierVerdict::Unknown`。**另一侧**：`clamp_by_readability(true, VerifierVerdict::Pass) == VerifierVerdict::Pass`。红条件：写成恒等（**移除档**）⇒ 第一条红；写成恒 `Unknown`（**收紧档**）⇒ 第二条红。**理由**：level 1 的确定性检查需要一个可读的制品，没有制品时不许给 `Pass`——这是 §4.4 的构造点规则在判定侧的对应，**两侧都钉**。
- `pass_and_fail_together_require_adjudication`（(p)）：`VerdictAggregate::from_verdicts(&[Pass, Fail{..}]) == AdjudicationRequired`（**不是** `Failed`、**不是**多数票的 `Passed`）。红条件：把 `AdjudicationRequired` 与 `Failed` 的次序互换、或把 `Failed` 写成「至少一条 `Fail`」（**取反档**）⇒ 红。**这一档若不红，说明 p 只拍了单项 `Fail`**（设计 §12.2 的预告）。
- `each_of_the_five_aggregate_arms_has_one_case`：**逐臂一条**——`[]` ⇒ `Unknown`（空集落在「无 `Fail`、无 `Pass`」那一臂；**本计划取这条读法并在此写明**）、`[Unknown]` ⇒ `Unknown`、`[Pass]` ⇒ `Passed`、`[Pass, Unknown]` ⇒ `PassedWithUnknown`、`[Fail]` ⇒ `Failed`。红条件：改任一臂的判据（**取反档**）⇒ 该条红。**`[]` 那一条的理由**：空与「全是 `Unknown`」都落在 `Unknown` 臂，而**不许**落在 `Passed`（fail-closed 的一侧）。
- `a_blind_verdict_comes_from_a_verifier_verdict`：`BlindVerdict::from(VerifierVerdict::Pass) == BlindVerdict::Pass`，三臂**逐条**。红条件：`from` 里把某一臂映射错（**取反档**）⇒ 该条红。**并写明来历**：规范**未给** `BlindVerdict` 的臂（§29 只规定顺序），本设计取「与 `VerifierVerdict` 同形的三臂」，唯一构造点是这个 `From`。
- `the_four_enums_each_name_their_arms`：`VerifierVerdict::ALL.len() == 3`、`VerdictAggregate::ALL.len() == 5`、`IndependenceVerdict::ALL.len() == 3`、`BlindVerdict::ALL.len() == 3`，逐臂点名；`VerdictAggregate` 与 `IndependenceVerdict` 各做一次编码往返（它们要落列，见 Task 9）。红条件：改某一枚的串（**取反档**）⇒ 该枚那一条红；删**该枚举的 `ALL` 常量数组**里的一臂（**移除档**）⇒ 数目断言红。**「删一臂」的锚点只取 `ALL`**（纪律 1(a)）。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test verifier
```

**预期形态是编译失败**：`src/{selection.rs,verdict.rs,input.rs}` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
pub struct AvailableVerifier {
    pub level: VerifierLevel,
    /// 该候选的 backend（§245）。§263 的隔离判定与 §120 的「不同证据路径」都读它。
    pub backend: BackendId,
    /// 供 §263 / §120 判「独立性」的身份。**今天恒为 `None`**（P1 设计 §14）。
    pub model: Option<String>,
    pub provider: Option<String>,
}

/// §7.2 的 Worker 身份：受验节点最近一次尝试的 `ExecutionProfile` 的**三项**
/// （`crates/continuum-graph/src/execution.rs:21`、`:22`、`:24` 的 `model` / `provider` / `backend`；`:23` 是 `tool`）。
pub struct WorkerIdentity { pub model: Option<String>, pub provider: Option<String>, pub backend: Option<BackendId> }

pub struct VerifierSelection { pub level: VerifierLevel, pub backend: BackendId, pub independence: IndependenceVerdict }

pub enum IndependenceVerdict { Independent, NotIndependent, NotEstablished }

/// 规则逐条（§7.2）：取 `level` 最高的**可用**候选（`available` 里出现的即「可用」——
/// **本块不判可用性**，那是资源层与策略层的事）；下低于 `policy.min_verifier_level` 的；
/// `profile.independent_verifier_required` 为真时下不独立的；**一个不剩即 `Err`，不静默降级**。
pub fn select_verifier(available: &[AvailableVerifier], policy: &VerificationPolicy,
                       profile: &VerificationProfile, worker: &WorkerIdentity)
    -> Result<VerifierSelection, VerifyError>;

/// §7.3 的身份隔离判据（**本设计定，规范未给比较规则**）：同 `backend` ⇒ 不独立；
/// `model` 与 `provider` 均已知且两项都相同 ⇒ 不独立；**任一项为 `None` ⇒ 判不出**。
pub fn independence_of(candidate: &AvailableVerifier, worker: &WorkerIdentity) -> IndependenceVerdict;

pub enum VerifierVerdict {
    Pass,
    /// §266 的 `no_blocking_verification_failure` 为假。
    /// **没有 `blocking: bool` 参数**：规范未给「非阻断失败」的判据，故本轮所有失败都是阻断的。
    Fail { reason: Claim },
    /// §123：正式表达未知，不逼模型在证据不足时给结论。
    Unknown { reason: Claim },
}

/// 一轮里全部 verifier 判定的聚合。**五臂的判据互斥且有序**，声明序即优先序。
pub enum VerdictAggregate {
    /// §121：同时存在 `Pass` 与 `Fail`。**不得多数投票**。优先级最高。
    AdjudicationRequired,
    /// 至少一条 `Fail` **且无 `Pass`**。
    Failed,
    /// 无 `Fail`、无 `Pass`（含空集）。
    Unknown,
    /// 无 `Fail`、有 `Pass` 且有 `Unknown`。
    PassedWithUnknown,
    /// 全部 `Pass`。
    Passed,
}

impl VerdictAggregate {
    /// §7.5 的五臂，**声明序即优先序**（`AdjudicationRequired` 优先于 `Failed`——
    /// §121 对这一形态的处理是「进入 Adjudication，不得多数投票」，取 `Failed` 会把一个
    /// 需要裁决的形态压成一次普通失败）。
    pub fn from_verdicts(v: &[VerifierVerdict]) -> VerdictAggregate;
}

pub enum BlindVerdict { Pass, Fail { reason: Claim }, Unknown { reason: Claim } }
impl From<VerifierVerdict> for BlindVerdict { /* 逐臂同形 */ }

/// §7.5：**无物可读 ⇒ `Unknown`，不得 `Pass`**。
pub fn clamp_by_readability(readable: bool, raw: VerifierVerdict) -> VerifierVerdict;

/// §263 的初判输入面。**是一个独立的类型，不是 `Evidence` 的切片**——
/// 只有这样，「不共享 Worker 的完成声明与完整推理」才是结构上的。
/// **`TestValidity` 也不在输入面里**：它是 Test Validity Verifier 的**产出**，
/// 把它再喂进初判输入等于让上一轮的判定喂下一轮（§120 的「共享证据路径」的一种）。
pub struct InitialVerifierInput { pub contract: ContractView, pub artifacts: Vec<Artifact>, pub evidence: Vec<Evidence> }

impl InitialVerifierInput {
    /// 装配处做两件事：拒绝 `producer` 指向受验节点自己的证据（§263）；
    /// 输入面**只有** §263 点名的三类，不加第四类。**不接受 `WorkerCompletionDeclaration`，
    /// 也没有第二个构造点。**
    pub fn assemble(contract: ContractView, artifacts: Vec<Artifact>, evidence: Vec<Evidence>,
                    verified_node: &NodeId, occurred_at: i64) -> Result<InitialVerifierInput, VerifyError>;
    /// §7.5：本输入面里有没有可读的制品（`clamp_by_readability` 的入参来源）。
    pub fn has_readable_artifact(&self) -> bool;
}

/// §29 的「blind verification 之后才比较 Worker reasoning summary」。
/// **构造点的第一个参数是 `BlindVerdict`**：没有盲判结论，这一类输入写不出来。
pub struct PostBlindVerifierInput { pub blind: BlindVerdict, pub worker_declaration: WorkerCompletionDeclaration }
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test verifier
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify Cargo.lock
git commit -m "feat(verify): Verifier 的五级序选择、独立性判定与两个输入面"
```

---

### Task 7: Completion Predicate 判定（§8/§266/§195）

**Files:**
- Create: `crates/continuum-verify/src/{conjunct.rs,completion.rs}`
- Create: `crates/continuum-verify/tests/completion.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,Cargo.toml}`（导出；`serde_json` 已在 Task 4）

**Interfaces:**
- Consumes: Task 5 的 `RequirementCoverage`；Task 6 的 `VerdictAggregate` / `IndependenceVerdict`
- Produces: `continuum_verify::{Conjunct, CompletionVerdict, CompletionInput, judge_completion}`

**本 task 定的取一（设计已给规则、未给判据的边界）**：`declared_completion_predicate` 的「有内容」判据。设计 §8.4 只写「有内容 ⇒ 追加一项不可判；空/缺 ⇒ 不追加」。本计划取：**无内容 ⟺ `None` 或 `serde_json::Value::Null`**；其余任何值（含 `{}` / `[]` / `""`）都算**有内容** ⇒ 追加。**理由**：另一方向是把用户声明的追加谓词静默丢掉，触 §342 第 1 条的方向。**代价**：`{}` 这类空容器会被当成有内容。

- [ ] **Step 1: 写用例**

`tests/completion.rs`（设计 §8.1 的并集、§8.2 的两臂、§8.3 的逐项、§8.4 的输入面）：

- `the_ten_conjuncts_come_from_the_union_of_266_and_195`：`assert_eq!(Conjunct::ALL.len(), 10)`，**逐臂点名**，并逐条断言它的出处（前五项 §266 的第 1–5 项、后五项 §195）。红条件：删掉 **`Conjunct::ALL`** 里的一臂（**移除档**）⇒ 数目断言红（锚点只取 `ALL`，纪律 1(a)）。**并写明两条去重**：`AllRequiredRequirementsVerified` ≈ §195 的 `requirement_coverage_sufficient`（合并只在**方向**上安全：合并后的臂恒 `Undetermined`，故即便两者本非同义也不会放行任何一项——这是 fail-closed 的方向，不是同义性论证）、`ContractSatisfied` ≈ §195 的 `task_contract_satisfied`（判据同一：§225 的 Constraint Validator 的结论）。
- `a_none_option_is_undetermined_and_a_false_option_is_blocked`（(q)，**六项逐条 + 两侧**）：对 `contract_satisfied` / `mandatory_effects_completed` / `mandatory_tests_pass` / `applicable_properties_hold` / `applicable_fuzz_budget_complete` / `applicable_mutation_quality_sufficient` **逐一**：`None` ⇒ 该项列入 `Undetermined.undecidable`；`Some(false)` ⇒ 该项列入 `Blocked.failing`。红条件：把 `Option<bool>` 换成 `bool`（`None` 与 `false` 合流，**取反档**）⇒ 十二条里的第一条红。**两侧都钉，缺哪一侧都会让「没判」与「判为假」互写**——而压缩的方向是把不放行说成失败，会让调用方去查一个不存在的失败。
  **并写明**：设计 §12.1 (q) 写的是「四项」，而 §8.4 的 `Option<bool>` 字段**实有六项**（第 3、4、6、7、8、9 项各一）——**本计划按 §8.4 的字段定义取六项**（正文优先于它的用例表），见 `## 遗留` 第一节。
- `a_declared_predicate_with_content_adds_one_undecidable_item`（(r)）：`Some(json!({"x": 1}))` ⇒ 结果里**多一项不可判**；并且**两个不同的非空值给出同一种结果**（本块**不解析**它的内容——解析就是发明语法）。`None` 与 `Some(Value::Null)` ⇒ **不追加**。红条件：把「有内容」判成「无内容」（**放宽档**）⇒ 第一条红；把「一律追加」写死（**取反档**）⇒ `None` 那一条红。
- `the_second_conjunct_is_always_undetermined`：**本设计最重要的一条**——`coverage` 给一条 `Incomplete` 的 REQUIRED，其余各项都给「为真」的值（`artifact_exists = true`、`contract_satisfied = Some(true)`、`mandatory_effects_completed = Some(true)`、`verifier_verdicts = Passed`、`independence = Independent`、第 6–9 项 `Some(true)`、`declared_completion_predicate = None`）⇒ 结果是 `Undetermined { undecidable }` **且 `undecidable` 恰含 `AllRequiredRequirementsVerified`**（**不是 `Blocked`，也不是空表**）。红条件：把第 2 项改成读 `CoverageVerdict`（写成「有 `Incomplete` 即 `false`」⇒ `Blocked`；或写成「全 `NotEstablished` 即 `true`」⇒ 空表）（**取反档**）⇒ 红。**这条是 §342 第 10 条与 §3.2「覆盖无正臂」的连接点**，也是「完成不可表达」的运行期一侧。
- `a_false_conjunct_blocks_even_when_others_are_undecided`：`artifact_exists = false` ＋ 第 3、4、6–9 项全 `None` ⇒ `Blocked { failing: [ArtifactExists] }`（**假优先于不可判**，且 `failing` 恰是那一项）。红条件：把优先级反过来先取 `Undetermined`（**取反档**）⇒ 红。
- `independence_not_established_makes_the_tenth_item_undecidable`（(s) 第一子例）：`independence = NotEstablished` ⇒ 第 10 项**列入 `undecidable`**，**不是** `Blocked`。红条件：写成 `NotIndependent`，即「没判」写成「判为假」（**取反档**）⇒ 红。**这正是 §8.4 警告过的「让调用方去查一个不存在的失败」**。
- `not_independent_blocks_the_tenth_item`（(s) 第二子例，**另一侧**）：`NotIndependent` ⇒ `Blocked { failing: [IndependentVerificationPass] }`。红条件：与上一条同一次变异反向（**取反档**）⇒ 红。
- `an_independent_pass_makes_the_tenth_item_true`（第 10 项的正例）：`Independent` ＋ `VerdictAggregate::Passed` ⇒ 第 10 项**既不在 `failing` 也不在 `undecidable`**；`PassedWithUnknown` 同样为真。红条件：把它写成恒不可判（**收紧档**）⇒ 红（`undecidable` 里多一项）。
- `a_non_passing_aggregate_makes_the_tenth_item_false`：**第 10 项判据表的第四格，三臂逐条**——`independence = Independent` 而聚合取 `Failed`、`AdjudicationRequired`、`Unknown` 各一条 ⇒ 第 10 项**为假**（列入 `failing`）。**判据表**（§8.3 第 10 项只给了前三行，第四行由本计划补）：`NotIndependent` ⇒ 假；`NotEstablished` ⇒ 不可判；`Independent` ∧ 聚合 ∈ {`Passed`,`PassedWithUnknown`} ⇒ 真；`Independent` 而聚合非通过 ⇒ 假。**该表写进实现的文档注释，本条只拍第四格**——前三格的照片是 (s) 的第一／二子例与 `an_independent_pass_makes_the_tenth_item_true`。红条件：把「`Independent` 而聚合非通过」写成不可判（**收紧档**）⇒ 三条全红。
- `the_fifth_conjunct_follows_the_aggregate_arm_by_arm`：**五臂逐条**——`Failed` ⇒ 第 5 项为假；`AdjudicationRequired` ⇒ 为假；`Passed` / `PassedWithUnknown` / `Unknown` ⇒ 为真（判据是「不出现 `Failed`，且无 `AdjudicationRequired`」）。红条件：把 `Unknown` 也算作阻断（**收紧档**）⇒ 该臂红。
- `the_two_verdict_arms_are_named`：`CompletionVerdict::ALL.len() == 2`，逐臂点名。**并写明**：**没有「完成」这一臂**，理由是 `AllRequiredRequirementsVerified` 的输入是 `CoverageVerdict`，而那个枚举没有正臂，故合取**不可能为真**——**这是构造性论证，不是实测**；它的实测补件是 Task 10 的 `CompletionVerdict::Complete` 编译失败样例。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test completion
```

**预期形态是编译失败**：`src/{conjunct.rs,completion.rs}` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
/// §266 的五项与 §195 的八项的**并集**（两者不是同一张表的两个抄本：§195 有五项在 §266 里
/// 没有对应物；只取 §266 会把它静默丢掉，而 §195 的标题正是「Completion Predicate **强化**」，
/// 丢掉它是**放宽**，触 §342 第 1 条的方向）。**两条去重的理由见用例注释。**
pub enum Conjunct {
    ArtifactExists, AllRequiredRequirementsVerified, ContractSatisfied, MandatoryEffectsCompleted,
    NoBlockingVerificationFailure, MandatoryTestsPass, ApplicablePropertiesHold,
    ApplicableFuzzBudgetComplete, ApplicableMutationQualitySufficient, IndependentVerificationPass,
}

/// **没有「完成」这一臂**（§8.2）。
pub enum CompletionVerdict {
    /// 至少一项为假。列出为假的项。
    Blocked { failing: Vec<Conjunct> },
    /// 无一项为假，但至少一项**无判据或判不出**。列出这些项。
    Undetermined { undecidable: Vec<Conjunct> },
}

pub struct CompletionInput {
    pub artifact_exists: bool,                                  // 第 1 项：由受验图与制品表算出
    pub coverage: Vec<RequirementCoverage>,                     // 第 2 项
    pub contract_satisfied: Option<bool>,                       // 第 3、4 项：**以值传入**，`None` = 未给出
    pub mandatory_effects_completed: Option<bool>,
    pub verifier_verdicts: VerdictAggregate,                    // 第 5、10 项
    pub independence: IndependenceVerdict,
    pub mandatory_tests_pass: Option<bool>,                     // 第 6–9 项：以值传入
    pub applicable_properties_hold: Option<bool>,
    pub applicable_fuzz_budget_complete: Option<bool>,
    pub applicable_mutation_quality_sufficient: Option<bool>,
    /// §222 的 `Intent.completion_predicate`——**用户声明的追加谓词**。
    /// **本块只判「它有没有内容」**：有内容 ⇒ 追加一项不可判（fail-closed，§342 第 1 条）；空/缺 ⇒ 不追加。
    /// **不解析它**——解析就是发明语法。
    pub declared_completion_predicate: Option<serde_json::Value>,
}

/// `Blocked` / `Undetermined` 都**不是** §344 的 `Result` 本身；那个 `Result` 由调用方据本返回值组装。
/// **优先级**：任一项为假 ⇒ `Blocked`；否则 ⇒ `Undetermined`（第 2 项恒不可判，故本轮两个返回值都
/// 是「不得完成」）。
pub fn judge_completion(input: &CompletionInput) -> Result<CompletionVerdict, VerifyError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test completion
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify
git commit -m "feat(verify): Completion Predicate 的十项合取判定"
```

---

### Task 8: Verifier Adversary 与 Mutation 的插槽（§9）

**Files:**
- Create: `crates/continuum-verify/src/adversary.rs`
- Create: `crates/continuum-verify/tests/adversary.rs`
- Modify: `crates/continuum-verify/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 6 的 `VerdictAggregate`；`continuum_operator::BackendId`
- Produces: `continuum_verify::{AdversaryOutcome, adversary_permits_passing}`

**本 task 定的形状**：§9.1 第 3 条只写「消费规则：`CounterexampleFound` 时，该轮 `VerdictAggregate` **不得**取 `Passed`/`PassedWithUnknown`」，没给函数名。本计划取一个**纯谓词**（`adversary_permits_passing(outcome, aggregate) -> bool`），由 Task 9 的 `record_verification_round` 在写行前施加——**它不改写聚合值，只拒绝那一对组合**（§9.1 第 4 条明写本轮它「不是判定反转」，唯一的可见形态是 finding 记录 + `strategy_verdict` 的标注）。

- [ ] **Step 1: 写用例**

`tests/adversary.rs`：

- `a_counterexample_forbids_a_passing_aggregate`：**四条逐条**——`(CounterexampleFound{..}, Passed)` ⇒ 不允许；`(CounterexampleFound{..}, PassedWithUnknown)` ⇒ 不允许；`(CounterexampleFound{..}, Failed)` ⇒ 允许；`(NoCounterexample, Passed)` ⇒ 允许。红条件：谓词写成恒真（**移除档**）⇒ 前两条红；写成恒假（**取反档**）⇒ 后两条红；写成只拒 `Passed`（**放宽档**）⇒ 第二条红。**四条一起才钉住「不得」的边界**。
- `the_two_arms_round_trip_through_their_encoding`：`AdversaryOutcome` 两臂各一次 `parse(as_str(x)) == Some(x)`（它落到 `verification_round.strategy_verdict` 列，见 Task 9；**`CounterexampleFound` 的载荷里含 `passed_verifier: BackendId` 与 `counterexample: Claim`，编码只覆盖判别串**——`## 遗留` 第二节记这一处的取舍）。红条件：改某一臂的串（**取反档**）⇒ 红。
- `the_two_arms_are_named`：`assert_eq!(AdversaryOutcome::ALL.len(), 2)`，两臂逐字点名。红条件：删掉 **`AdversaryOutcome::ALL`** 里的一臂（**移除档**）⇒ 数目断言红（锚点只取 `ALL`，纪律 1(a)）。
- `the_mutation_slot_is_the_test_arm_and_a_threshold_that_nobody_reads`：**本 task 不判 mutation**（§9.2）——它落成一条**源码面**的核对（Task 11 执行，这里只留一条注释指向它）：本 crate 里 **`mutation_score` 零命中**、**`EvidenceStrength` 上没有 `Ord`**、`VerificationProfile.mutation_threshold` **没有任何读者**。**本 task 不写一条会跑的用例来覆盖这三条**，因为它们是「不存在的东西」——**照实写成「由 Task 11 的 grep 与人工通读钉，无运行期照片」**。「`mutation_threshold` 没有读者」另有一条编译期照片：给它写一个读者即编译过，而**今天没有**——**这一条没有守卫**，明写在此。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test adversary
```

**预期形态是编译失败**：`src/adversary.rs` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
/// §264 / §215 的产物。**「高价值任务」本轮不定义**（`B1`：触发条件与成本归属规范未给），
/// 故本类型本轮无触发——**它只冻结类型与消费规则**。
pub enum AdversaryOutcome {
    NoCounterexample,
    /// §264：找到反例 ⇒ `verification_strategy = insufficient`。
    CounterexampleFound { passed_verifier: BackendId, counterexample: Claim },
}

/// §9.1 第 3 条的消费规则：`CounterexampleFound` 时该轮的聚合**不得**取
/// `Passed` / `PassedWithUnknown`。**只拒绝那一对组合，不改写聚合值**。
pub fn adversary_permits_passing(aggregate: &VerdictAggregate, outcome: &AdversaryOutcome) -> bool;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test adversary
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify
git commit -m "feat(verify): Verifier Adversary 的类型与消费规则（本轮无触发）"
```

---

### Task 9: 持久化（迁移 130）与 `VerificationFailed` 的第一个产生点（§10）

**Files:**
- Create: `crates/continuum-verify/src/persist.rs`
- Create: `crates/continuum-verify/tests/persist.rs`
- Modify: `crates/continuum-verify/{src/lib.rs,Cargo.toml}`（导出；加 `continuum-persist` / `continuum-events` / `tempfile`（dev））
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 的 `continuum-verify` 条目补到五条，**数组按字母序**）

**Interfaces:**
- Consumes: Task 2 的 `Evidence` / `EvidenceProducer`；Task 3 的 `TestValidity`；Task 6 的 `VerifierSelection` / `VerdictAggregate` / `IndependenceVerdict` / `InitialVerifierInput`；Task 8 的 `AdversaryOutcome` / `adversary_permits_passing`；`continuum_persist::{Tx, Migration, Value, PersistError}`；`continuum_events::{Event, EventType}`；`continuum_graph::{AdfirGraph, GraphId, NodeId, EdgeKind}`
- Produces: `continuum_verify::{p5a_verify_migrations, insert_evidence, insert_test_validity, record_verification_round, artifact_exists}`

**本 task 定的两处形状（设计给了列、没给函数名与可空性）**：

- **`producer_node` 列对 `EvidenceProducer::Human` 一支写 `NULL`**（列可空），且 §10.2 第 3 条的边核对**只在 `Node` 一支执行**。**理由与代价**：§10.2 第 3 条的不变量逐字是「一行 `evidence` 对应图上一枚 `producer_node → verified_node` 的 `EdgeKind::Evidence` 边」，而 `HumanConfirmation` 的证据由人给出、**没有产生者节点**（§4.3.3 的 `Human` 臂）——两者不能同真。本计划取「可空列 ＋ 按支核对」，并把这一处**报给设计作者**（`## 遗留` 第一节）。
- **`verification_round.strategy_verdict` 列存 `AdversaryOutcome` 的编码**。设计 §10.2 只给列名、§2.2 的冻结清单里没有它的承载类型；本计划**不为它新造一个枚举**——`AdversaryOutcome` 的两臂正是「有没有反例」，而已知的一枚 `verification_strategy` 取值（§264 的 `insufficient`）不构成一个完整取值域（规范未给其余取值）。

- [ ] **Step 1: 写用例**

`tests/persist.rs`（用 `Db::open_with(path, p5a_verify_migrations())` 起测试库，照 P4 对 `contract` 表的做法）：

- `an_evidence_row_is_insert_only`：同一 `evidence_id` 写两次 ⇒ 第二次 `Err`，**且原行不变**（再读一次逐字段断言未变）。红条件：把 INSERT 写成 `INSERT OR REPLACE`（**放宽档**）⇒ 红。**这一条钉的是 §4.1 的「证据记录只追加」**，它的理由是具体的：有效性判定与证据必须是两条记录，否则「验证者改了被验之物」。
- `the_producer_node_of_human_evidence_is_null_and_is_not_edge_checked`：`HumanConfirmation` ＋ `producer: Human` ⇒ 写入成功、`producer_node` 读回为空。**另一侧**：`producer: Node{..}` 而图上**没有** `producer_node → verified_node` 的 `EdgeKind::Evidence` 边 ⇒ `Err(VerifyError::MissingEvidenceEdge { .. })`；图上**有**该边 ⇒ `Ok`。红条件：删掉边核对（**移除档**）⇒ 第二条红。
- `every_enum_column_round_trips_through_its_helper`：**逐列一次**——`evidence.evidence_type`、`test_validity.verdict`、`test_validity_check.check_name`、`test_validity_check.verdict`、`verification_round.aggregate`、`verification_round.independence`、`verification_round.strategy_verdict` 各写各读。红条件：某列直接用 `Debug` 或 serde 写（**取反档**）⇒ 该列读回红（`NodeState` 的 serde 是 `SCREAMING_SNAKE_CASE`，直接反序列化会失败——P1 设计 §15 的同一条纪律）。
- `an_out_of_table_value_in_a_column_is_an_error`：绕过 writer 直接 SQL 写一行表外取值 ⇒ 读回 `Err`，**不取默认型**。红条件：`parse` 末支返回默认（**放宽档**）⇒ 红。
- `a_failed_round_writes_a_verification_failed_event_in_the_same_transaction`：`record_verification_round` 配 `Failed` ⇒ 同一 `Tx` 里 `event` 表多一条 `verification.failed`，且 `node_id` 已置。**另一侧**：`Passed` ⇒ **不写**事件。红条件：删掉事件写入（**移除档**）⇒ 第一条红；写成「一律写」（**取反档**）⇒ 第二条红。
- `the_event_is_written_only_for_failed_and_adjudication_required`：**五臂逐条**——`Failed` 写、`AdjudicationRequired` 写、`Passed` / `PassedWithUnknown` / `Unknown` 不写。红条件：把 `AdjudicationRequired` 漏掉（**移除档**）⇒ 该臂红。
- `a_counterexample_with_a_passing_aggregate_cannot_be_recorded`（§9.1 的消费规则落在写行处）：`(CounterexampleFound, Passed)` ⇒ `Err`；`(CounterexampleFound, Failed)` ⇒ `Ok`；`(NoCounterexample, Passed)` ⇒ `Ok`。红条件：删掉这道核对（**移除档**）⇒ 第一条红。
- `the_written_aggregate_is_the_clamped_one`：给 `[Pass]` 而 `initial_input` 无可读制品 ⇒ 写下的 `aggregate` 是 `Unknown`（**不是 `Passed`**）。红条件：跳过 `clamp_by_readability`（**移除档**）⇒ 红。**这一条把 Task 6 的 (o) 与落库连起来**：§7.5 的规则在写入路径上是可观察的。
- `artifact_exists_needs_every_terminal_node_to_have_produced_something`（§8.3 第 1 项）：图上两个终端节点、制品表里只有其一 ⇒ `false`；两个都有 ⇒ `true`。红条件：把它写成「任一终端节点有制品」（**放宽档**）⇒ 第一条红。
- `artifact_exists_over_an_empty_terminal_set_is_true_and_that_is_a_fail_open_shape`：**终端集为空** ⇒ `true`（「各自至少有一个」空洞为真）。**并写明**：这是一处 **fail-open 的形状**，上游没有守卫（实测 `AdfirGraph::validate()`（`crates/continuum-graph/src/graph.rs:87`）不要求 `terminal_nodes` 非空）——**本计划照实钉住它，不改它的取值**（改它就要替 §266 第 1 项发明一条「空图不算存在」的判据），并把它报给设计作者（`## 遗留` 第一节）。红条件：把它写成「终端集为空 ⇒ 假」（**取反档**）⇒ 本条红（**这正是「钉住」的意义**：值一旦被改，有人会知道）。
- `the_migration_is_the_only_producer_of_version_130`：`p5a_verify_migrations()` 返回的 version 恰是 `[130]`，名字是 `p5a_verify` 一类；五张表都在它的 SQL 里（逐表名断言）。红条件：改成 131（**取反档**）⇒ 与 Task 11 的 `runtime_migrations()` / `expected_migrations()` 对不上时红。

- [ ] **Step 2: 运行，确认失败（预期形态是编译失败，不是运行期红）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test persist
```

**预期形态是编译失败**：`src/persist.rs` 要 Step 3 才建，故此时报的是 `error[E0433]`／`cannot find …`。按纪律 1(c)，**编译失败不是「变红」**，故这一步照实写成「确认编译失败」。

- [ ] **Step 3: 实现**

```rust
/// 迁移 130（§10.1 已实测该档为空）。五张表一次写齐（设计 §10.2）：
///   evidence              evidence_id PK, graph_id, verified_node, producer_node(可空),
///                         subject_kind, subject_requirement, evidence_type,
///                         claim, strength(TEXT/JSON), scope(TEXT/JSON), occurred_at
///   evidence_artifact     (evidence_id, artifact_id) PK
///   test_validity         evidence_id PK, verdict
///   test_validity_check   (evidence_id, check_name) PK, verdict
///   verification_round    round_id PK, graph_id, verified_node, selection(JSON),
///                         aggregate, independence, strategy_verdict, occurred_at
/// 不变量逐条见设计 §10.2；**本函数不 `commit`**（照 P1 的 apply_transition 形状）。
pub fn p5a_verify_migrations() -> Vec<Migration>;

/// 只 INSERT，无 UPDATE（§4.1）。写行时同事务核对图上存在 `producer_node → verified_node`
/// 的 `EdgeKind::Evidence` 边（`Human` 一支不核对，`producer_node` 写 NULL）。
pub fn insert_evidence(tx: &Tx<'_>, graph: &AdfirGraph, evidence: &Evidence,
                       verified_node: &NodeId, occurred_at: i64) -> Result<(), VerifyError>;

/// 写一条有效性记录（五臂齐已在 Task 3 的构造点把关）。
pub fn insert_test_validity(tx: &Tx<'_>, validity: &TestValidity) -> Result<(), VerifyError>;

/// 记一次判定：**逐条 clamp（§7.5）→ 聚合（§7.5 的五臂）→ 核对消费规则（§9.1）→ 写行**；
/// 聚合落 `Failed` / `AdjudicationRequired` 时**同事务**写一条 `EventType::VerificationFailed`
/// （payload 含逐项状态）。**不写 `IntentCompleted`**（本块不产生「完成」）。
/// **不写审计**：§313 的必录清单里没有验证类事件，故本块不新增 `AuditKind`。
pub fn record_verification_round(tx: &Tx<'_>, round_id: &str, graph_id: &GraphId,
                                 verified_node: &NodeId, selection: &VerifierSelection,
                                 initial_input: &InitialVerifierInput, verdicts: &[VerifierVerdict],
                                 independence: IndependenceVerdict, outcome: &AdversaryOutcome,
                                 event_id: &str, occurred_at: i64) -> Result<VerdictAggregate, VerifyError>;

/// §266 第 1 项：受验图的**每个终端节点**至少有一个产出制品（读 `artifact` 表的 `producer_node`）。
/// **不读 `Artifact.metadata`**（其 schema 规范未给，§15 第 9 条）。
pub fn artifact_exists(tx: &Tx<'_>, graph: &AdfirGraph) -> Result<bool, VerifyError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test persist
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(verify): 迁移 130、只追加的证据表与 VerificationFailed 的第一个产生点"
```

---

### Task 10: 类型层照片（`trybuild`）

**Files:**
- Create: `crates/continuum-verify/tests/type_level.rs`
- Create: `crates/continuum-verify/tests/compile_fail/{initial_verifier_input_has_no_place_for_a_worker_declaration,post_blind_verifier_input_cannot_be_built_without_a_blind_verdict,completion_verdict_has_no_complete_arm,evidence_fields_are_private,evidence_has_no_default}.rs`
- Create: 同名五份 `.stderr`（**取自实跑**）
- Modify: `crates/continuum-verify/Cargo.toml`（dev-dep `trybuild`）

**Interfaces:**
- Consumes: Task 2 / 6 / 7 的类型
- Produces: —

**为什么要单独一个 task**：这五份样例钉的都是**不可表达性**——运行期用例无论怎么写，都只能证明「我没这么构造过」，证明不了「做不到」（照 `crates/continuum-node/tests/type_level.rs` 文件头那段论证）。**它们的观测方式是「样例从编译失败变成编译通过」或「编译不过」，不是「用例变红」**——本 task 的 report 里要这样写。

- [ ] **Step 1: 写样例与驱动**

`tests/type_level.rs`：**照 `crates/continuum-node/tests/type_level.rs` 的形状**，含 `t.compile_fail("tests/compile_fail/*.rs")`、手写常量 `LISTED_SAMPLES: [&str; 5]`、以及与之配套的 `the_sample_list_matches_the_directory`（清单 ↔ 目录 ↔ 文件头点名三处对钉）。**本文件的文件头那段点名要逐份点名这五份**（点名项数必须等于目录里 `.rs` 的份数）。

五份样例**逐份钉一件事**（名字即它钉的那条通道）：

1. `initial_verifier_input_has_no_place_for_a_worker_declaration`：`InitialVerifierInput { contract, artifacts, evidence, worker_declaration }`（多出第四项）⇒ 编不过。**钉的是 §7.4 第 1 条**：初判输入的字段清单里**没有那个位置**。
2. `post_blind_verifier_input_cannot_be_built_without_a_blind_verdict`：`PostBlindVerifierInput::new(declaration)`（缺第一个参数）⇒ 编不过。**钉的是 §7.4 第 2 条**：没有盲判结论，这一类输入写不出来。
3. `completion_verdict_has_no_complete_arm`：`CompletionVerdict::Complete { failing: vec![] }` ⇒ 编不过。**钉的是 §8.2**：「完成」这一臂不存在——**这是「完成不可表达」那句构造性论证的实测补件**。
4. `evidence_fields_are_private`：`Evidence { … }` 全字段结构体字面量 ⇒ 编不过。**钉的是 §4.4 的「唯一产生点」是结构事实**。
5. `evidence_has_no_default`：`Evidence::default()` ⇒ 编不过（无实现）。**与第 4 份是两条不同的通道，不许并成一条**。

**两条坐标写死**（纪律与「关于本计划的代码块」那一节）：`.stderr` **逐份取自实跑，不许凭记忆写、不许拿一个值套五处**；**`.stderr` 一旦接受，就不再改那份 `.rs` 的注释**（trybuild 按文件主干配对且**不规范化行号**，改文件头会推移坐标——要改就重跑并一起提交新 `.stderr`）。

**首跑的接收流程**：`trybuild` 首跑把不匹配的 `.stderr` 落在 `wip/`；人工逐份看过（确认「因为预期的那条错而编不过」）再搬进 `tests/compile_fail/`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test type_level
```

**预期**：五份样例尚不存在或尚未接收 `.stderr`，本步以「产出 `wip/*.stderr`」或「样例与期望不符」的形式失败。

- [ ] **Step 3: 接收 `.stderr` 并写驱动**

逐份把 `wip/*.stderr` 搬进 `tests/compile_fail/`，并在 `LISTED_SAMPLES` 与文件头点名两处同时加上那五份名字。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-verify --test type_level
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-verify Cargo.lock
git commit -m "test(verify): 五份类型层照片与清单对钉"
```

---

### Task 11: 装配与收尾

**Files:**
- Modify: `crates/continuum-runtime/{Cargo.toml,src/main.rs}`（加依赖边；`runtime_migrations()`（`:98`）追加一行）
- Modify: `crates/continuum-runtime/tests/{dependency_direction.rs,migrations.rs,startup.rs}`
- Modify: `crates/continuum-verify/**`（**仅在复核发现缺口时**）

**Interfaces:**
- Consumes: Task 9 的 `p5a_verify_migrations`
- Produces: 装配后的运行时

- [ ] **Step 1: 装配**

三处逐条：

- `crates/continuum-runtime/Cargo.toml` 的 `[dependencies]` 加 `continuum-verify = { path = "../continuum-verify" }`（**按字母序落位**，它的邻居是 `continuum-secrets` / `continuum-workspace` 之间）——**这一条也写进 `dependency_direction.rs` 的 `ALLOWED`**：`("continuum-runtime", …)` 那一行加上 `"continuum-verify"`（数组按字母序）。
- `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:98-108`，**现有八行**：`builtin` 起、到 `p3d_model_migrations()` 止）追加一行 `migrations.extend(continuum_verify::p5a_verify_migrations());`。**「谁的表谁注册」**。
- `crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`（`pub` 面在 `:29`，函数体 `:30-38`）同步追加同一行——那是 `main.rs` 装配处的**第二份转录**，比对由 `the_runtime_applies_…` 拿实际落库的集合做。
- `crates/continuum-runtime/tests/startup.rs` 的两处「迁移应用 N 项」（今天 `:24` 与 `:90` 各写 9，`:79` 写 7）——**按实跑输出改数，不许预判**（该文件 `:88-90` 的注释已写明这几个计数同时是「迁移编号不重复」的守卫）。

- [ ] **Step 2: 全量验证**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$PWD/.tmp/gate-final.log"
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
grep -c '^ *Running' "$PWD/.tmp/gate-final.log"
grep -c '^test result:' "$PWD/.tmp/gate-final.log"
```

预期：全绿、0 warning。**判据是最后两行的两个数**（纪律 3）——`Running` 的行数与 `test result:` 的行数（Doc-tests 算一条），**不是** `Compiling` 的行数；**必须带 `--no-fail-fast`**，否则会在第一个失败目标处停住、后面的目标一行不跑。

- [ ] **Step 3: 依赖方向与源码面复核**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
grep -rn "continuum_semantics\|continuum-semantics" crates/continuum-verify/src
grep -rn "Ord" crates/continuum-verify/src/evidence.rs
grep -rn "mutation_score\|mutation_threshold" crates/continuum-verify/src
grep -rn "SystemTime::now\|Instant::now" crates/continuum-verify/src
grep -rn "UPDATE evidence\|INSERT OR REPLACE" crates/continuum-verify/src
```

**六条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：

- （a）**语义层零命中**——这一条钉的是「本块不给语义层登记边」（跨计划前置第二节）。**注意「零命中」讲的是 `src/` 里没有 `use`，不是整篇不出现**：注释里允许出现（跨计划前置与 `contract_view.rs` 的文档都要写这一条）。
- （b）`EvidenceStrength` / `EvidenceScope` 上**零 `Ord`**（§4.3.4：给一个规范没给的 `Ord` 就是发明）。
- （c）`mutation_score` 零命中；`mutation_threshold` **只出现在 `VerificationProfile` 的字段定义上一次**（零读者——§9.2 的「只冻插槽，不判」）。
- （d）系统时钟零命中（本块不取时钟，`occurred_at` 一律入参）。
- （e）`UPDATE evidence` 与 `INSERT OR REPLACE` 零命中（§4.1 的只追加）。
- （f）`ALLOWED` 的 `continuum-verify` 条目与 `Cargo.toml` **精确一致**（逐对 `assert_eq!` 已覆盖），且**只有五条**（artifact / events / graph / operator / persist）。**并把粒度写清**：`--depth 1` 只钉**直接边**；传递可达在 Rust 下不可 `use`，**故直接边即这条规则的完整粒度**，**不另设闭包断言**。
- **另一条人工复核（不是 grep）**：打开 `crates/continuum-graph/src/node.rs`，**逐字读改后的 `:19` 那一行**，确认它写出了读者的名字、且 `:20` 的字段类型一个字未动。**复核结论写进报告，写清当时那一行的内容**。

- [ ] **Step 4: 逐条核对《工程》§8.2 与 §8.4 的完成判据**

| 依据（行号） | 本块的证据 |
|---|---|
| §8.2「Evidence 是验证的唯一输入形态（§258）。工具结果必须先转为 Evidence 再进入判定（§89）」（`docs/02-工程.md:502`） | Task 2 的唯一产生点 ＋ Task 10 的两条编译期照片；Task 6 的输入面只收 `Evidence` 与 `Artifact`，不收裸工具结果 |
| §8.2「Requirement Coverage Graph 把每项 REQUIRED 映射到证据集合（§260）…是 §342 第 5 条不变量的执行点」（`:504`） | Task 5 的 (a)/(b)/(k) 三条；**「充分」那一半不存在**（§6.2 无正臂）——**据实标注为「只做到禁止面」** |
| §8.2「VerificationProfile 决定需要哪些证据类型（§261）。该组件的生成者未定义（A6）」（`:506`） | Task 4 的形状 ＋ `profile_for` 的两侧；**生成者不建** |
| §8.2「Completion Predicate 判定器独立于 Worker。输入为 Contract、Artifact、Evidence（§263），不接受 Worker 的完成声明」（`:508`） | Task 6 的输入面（编译期照片 1）＋ Task 7 的两臂 |
| §8.2「Verifier 优先级按 §262 的五级顺序…降级路径未定义（A5）」（`:510`） | Task 4 的五级序 ＋ Task 6 的两条 `Err` |
| §8.2「图像局部编辑 …mask 外像素由原图直接复制，该判定可确定性完成（§168）」（`:512`） | **本块只给第 1 级的插槽**（`clamp_by_readability` ＋ §7.5 的「无物可读 ⇒ `Unknown`」）；**比较器属 P5f**（设计 §14 第 3 条）——**据实标注为部分** |
| §8.4「测试通过不进入 Completed，除非 Requirement Coverage 判定充分」（`:530` 起） | 「除非」那一支**不存在**（§6.2 无正臂 ＋ §8.2 无完成臂）⇒ 该行实际是「测试通过不进入 Completed」——**收紧，不触 §342 第 1 条** |
| §8.4「每项 REQUIRED 存在对应证据集合，无证据时为 INCOMPLETE」 | Task 5 的 (a)/(k) |
| §8.4「模型不能自行宣告完成（§266）」 | Task 6 的输入面 ＋ Task 7 的两臂（类型层） |
| §8.4「Verifier 的初判输入不含 Worker 的完成声明与完整推理」 | Task 6 的编译期照片 1 ＋ **一条残余**：`Claim` 是自由文本，协议上仍可把推理写进 `claim`（设计 §15 第 2 条） |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑本 crate 的**承重守卫**，每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 → 还原后 `sha256sum` → 独立日志路径。**逐轮在报告里附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位（取反／放宽／收紧／移除）」，并按纪律 3 附 `Running` 行数与 `test result:` 行数**。

**设计 §12.2 已预告的八档，逐档对照**（预告写反会让人去查不存在的问题——**若不红，先看语料是否备齐，再看预言是否写错，两样都要报**）：

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`Incomplete` 与 `NotEstablished` 在计数规则里的位置互换（第 3 条计数变不计数） | Task 5 的 (a)(c)(e) | (b)(d)(f) 中对该臂身份不敏感的那一半；Task 4 的 (i)(j) |
| **取反**：`Validity` 的 `Valid` / `Invalid` | Task 3 的 `one_fail_makes_the_whole_record_invalid` 与 `five_passes_give_valid`；Task 5 的 (e)(f) | Task 5 的 (a)(b)(c)(d) |
| **放宽**：`from_value` 的未知分支返回 `Default` 而不是 `Err` | Task 4 (j) 的第二个子例 | 其余全部（**这一档若不红，说明 (j) 只拍了空策略那一侧**） |
| **收紧**：`Unknown` 的检查结果算作不计数（第 4 条计数规则取反） | Task 5 的 `a_test_evidence_with_unknown_validity_still_counts` | Task 5 的 (a)(c)（**本计划已把这份语料做成独立用例**） |
| **移除**：`MissingRequiredTypes` 这一臂从判定里删掉 | Task 5 的 (c) | Task 5 的 (d)(a)(b) |
| **移除**：`assemble` 里那条「受验节点自己产的证据」检查 | Task 6 的 (m) | Task 6 的 (n)（**应当仍绿**） |
| **取反**：`judge_completion` 把 `independence == NotEstablished` 当作 `NotIndependent`（「没判」写成「判为假」） | Task 7 的 (s) 第一条；**Task 6 的 `an_unknown_identity_is_not_established` 也会红**（同一处判据） | Task 7 的 (s) 第二条；(q)（`Option` 那六项是另一条纪律）；其余全部 |
| **取反**：`VerdictAggregate` 的 `AdjudicationRequired` 与 `Failed` 次序互换（`Pass ∧ Fail` 落到 `Failed`） | Task 6 的 (p) | 其余全部（**这一档若不红，说明 (p) 只拍了单项 `Fail`**） |

**本计划补的承重档**（设计 §12.2 未列，而它们各自钉一条本块独有的承诺）：`clamp_by_readability` 写成恒等（Task 6 的 (o) 第一条 ＋ Task 9 的 `the_written_aggregate_is_the_clamped_one`）；`counts` 忽略 `subject`（Task 5 的 `evidence_of_another_requirement_does_not_count` 与 `unattached_evidence_does_not_count` 两条同时红——**两条一起跑才看得出「只要一条守卫」不是够的**）；`insert_evidence` 的只追加改成 `INSERT OR REPLACE`（Task 9 的第一条）；`producer_node` 有制品就算「终端节点有产出」（Task 9 的 `artifact_exists_needs_every_terminal_node_…` 第一条）。

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。**本计划不新建、也不修改第二份文档**：若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-runtime crates/continuum-verify Cargo.lock
git commit -m "feat(runtime): 装配 P5a 的迁移 130 与依赖边"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**
设计 §15 的十八条与 §14 的十条对账条目**本计划一条都不发明**——收件人照原文，见第五节。

### 一、设计问题：**本计划查出的各条**（发现即报，一处都不替设计补写）

**这些条目分两类**：一类是「同一份设计里两处不能同真」（相抵或指错），一类是「漏项」。

1. **`RequirementId` 的第二份 vs 不给语义层登记边**（**相抵；这是开工前须知**）：§4.3.1（`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md:270`）写「本设计**不**定义第二个 `RequirementId`」并把来源指给 P4 的 `continuum-semantics`，§14 第 1 条把「由 P4 提供」列为假设；而 §2.1 的边表不含 `continuum-semantics`、§6.1 逐字写「**不登记到 `continuum-semantics` 的边**」。**两者不能同真**：不登记边就 `use` 不到 P4 的类型，而「以值承载」需要一个本块的类型。

   **相抵的最尖处在 §4.3.1 内部，不只在 §4.3.1 与 §2.1／§6.1 之间**：同一段里「**不**定义第二个 `RequirementId`」（`:270`）＋「本块以**值**承载它」＋理由 3（`:268` 逐字「**必须是强类型。** 若取 `String`，归属判定退化成字符串比较，且**失配的方向是静默漏配**」）**三者无解**——不登记边 ⇒ 拿不到 P4 的类型；强类型 ⇒ 不能是裸 `String`；剩下一枚本块类型 ⇒ 就是「第二个 `RequirementId`」。三条读法各自违反至少一处。

   **设计 §4.3.1 需要改的一句（改哪一句、为什么）**：改 `:270` 的「本设计**不**定义第二个 `RequirementId`」——改成「本块以一枚承载类型持有它，P4 落地后由 P4 的类型为准」一类。**为什么**：**「定型落消费侧」那条既有先例消解不了本条**——§2.3 那条裁决成立的前提是**源侧本来没有类型**（`crates/continuum-graph/src/node.rs:20` 是 `pub verification_policy: Value`，故本块的 `VerificationPolicy` 不是第二个同概念类型）；而 `RequirementId` 的**源侧有类型**：P4 计划 Task 11 的产出面逐字含 `continuum_semantics::{… RequirementId …}`（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:1223`，落在 `src/ids.rs`，同文件 `:295`），故「不登记反向边 ＋ 以值承载」**必然**造出一枚与 P4 同名的第二枚。**硬约束那一侧实测成立、且要保留**：`docs/02-工程.md` 的 `:570`（`语义层 (2) → 执行层 (3)`）、`:575`（`执行层 (3) → 跨领域 (8)`）、`:588`（「依赖方向单向，无环。」）——`continuum-verify ← continuum-semantics` 即 `8 → 2`，闭成 2-环。

   **同一条的另一半**：§2.2 的冻结清单还须把该承载类型与 `RequirementClass` / `ContractView` 补进去，否则上面那一句改完、漏项仍在（本节**第 10 条**已报，此处只作指针）。**设计那一份由协调者另行处置，本计划不改设计。** 本计划的取舍见跨计划前置第二节（本块定义 `RequirementId` / `RequirementClass` / `ContractRequirement` / `ContractView`）。**另**：P4 的 `continuum-semantics` **今天不存在**（实测 `crates/` 与 `p4` 分支都没有），故「以 P4 的 `RequirementId` 为准」在今天是一句无法执行的话。**收件人：设计作者 ＋ 协调者。**
2. **§12.1 (q) 的「四项」 vs §8.4 的六个 `Option<bool>` 字段**（**相抵**）：§8.4 的 `CompletionInput` 里 `Option<bool>` 实有**六项**（第 3、4、6、7、8、9 项），而 §12.1 (q) 逐字写「**四项** `Option` 为 `None`」、§8.3 第 10 项也写「与下面**四项** `Option` 的 `None` 同一条纪律」。**按「四项」写计划会漏钉第 3、4 项（`contract_satisfied` / `mandatory_effects_completed`）两侧**，而漏掉的正是「没判写成判为假」最容易漂的那两项。本计划按 §8.4 的字段定义取**六项**（正文优先于它的用例表）。**收件人：设计作者 ＋ 复审者。**
3. **`Undecidable` vs `Undetermined`**（**措辞相抵**）：§8.2 定义的是 `CompletionVerdict::Undetermined { undecidable }`，而 §12.1 (q) 写 `Undecidable{undecidable: […4 项…]}`（(r)(s) 只用「不可判」这个词，不涉变体名）。同一份设计里两个名字。本计划取 §8.2（类型定义那一处）。**收件人：设计作者。**
4. **§8.4 末段把「两侧都要钉」指到 §12.1 (c)/(d)**（**指错**）：(c)/(d) 是 `MissingRequiredTypes` 的缺项判据用例，与 `Option` 的 `None` vs `false` 是两条不相干的纪律；该指的应是 §12.1 **(q)**。**收件人：设计作者。**
5. **§12.2 的「第 5 条计数规则取反」是第 4 条**（**指错**）：`Unknown` 是否计数是 §6.3 的**第 4 条**，第 5 条讲的是非测试证据。**这一处会把人引去改错的地方**（改第 5 条不会让任何用例红，于是被读成「缺用例」）。**收件人：设计作者 ＋ 复审者。**
6. **`evidence` 表没有 `backend` 列，而 `EvidenceProducer::Node` 带 `backend: BackendId`**（**表与类型不相称**）：§4.3.3 的 `Node { node, backend }` 里 `backend` 的用途逐字是「§262 第 3、4 级的可用性、§120 的『不同 family / 不同证据路径』都要能命名『哪一条路径产出了这条证据』」，而 §10.2 的列清单里没有它。**今天不可观察**（设计未给 `Evidence` 的读取路径），故**它不是一条会红的缺口，是一件会静默丢信息的事**。**收件人：设计作者。**
7. **§10.2 第 3 条的不变量与 `EvidenceProducer::Human` 相抵**（**相抵**）：不变量逐字是「一行 `evidence` 对应图上一枚 `producer_node → verified_node` 的 `EdgeKind::Evidence` 边」，而 `HumanConfirmation` 的证据由人给出、**没有产生者节点**（§10.2 也没给 `producer_node` 的可空性）。本计划取「列可空 ＋ 按支核对」并在 Task 9 各钉一条。**收件人：设计作者。**
8. **§4.4 未点名 `Deserialize`**（**漏项，而它是第二个构造点**）：§4.4 的「唯一产生点」列的是「八字段全私有、无 `Default`、无 `From` / `FromStr`、无第二构造点」，没提 serde。而 `#[derive(Deserialize)]` **正是一个绕过 `from_tool_result` 的构造路径**，且 `EvidenceStrength` / `EvidenceScope` 两个载荷已经是 `serde_json::Value`，加派生的成本很低。本计划在 Task 2 明写禁止它（`## 遗留` 第二节）。**收件人：设计作者。**
9. **`artifact_exists` 在空终端集上空洞为真**（**漏项，fail-open 的形状**）：§8.3 第 1 项的判据是「受验图的终端节点**各自**至少有一个产出制品」，而实测 `AdfirGraph::validate()`（`crates/continuum-graph/src/graph.rs:87-107`）**不要求 `terminal_nodes` 非空**，故「终端集为空 ⇒ 真」是一条可达路径。本计划照实钉住它（Task 9 有一条用例写死这个值），**不改它的取值**——改它就要替 §266 第 1 项发明一条「空图不算存在」的判据。**收件人：设计作者 ＋ 执行层。**
10. **§2.2 的冻结清单漏了五枚本块定义的类型**（**漏项，而清单自称是跨块合同**）：清单收尾逐字写「它们是另外四块照本清单接线的合同的一部分」，而 `ContractView` / `RequirementId` / `RequirementClass` / `VerificationProfileSource` / `WorkerIdentity` **五枚都由本块定义、且都是本块判定函数的入参形状**，却不在清单里（其中 `WorkerIdentity` 还是 `select_verifier` 的形参类型）。**这不改变任何判定，只改变清单的射程**——读到它的人会以为那五枚由别处提供。**收件人：设计作者。**
11. **`strategy_verdict` 列没有承载类型**（**漏项**）：§10.2 给了 `verification_round.strategy_verdict` 这个列名，§9.1 只说「一条 finding 记录 + `verification_strategy` 的标注」，**§2.2 的冻结清单里没有任何一枚类型承载它**，而规范只给了一枚取值（§264 的 `insufficient`），不构成完整取值域。本计划取 `AdversaryOutcome` 的编码填这一列（Task 8／9），**不为它新造一个取值域**。**收件人：设计作者。**

12. **§6.2 末段的「两臂的严重度有序」在本块无落点、无用例**（**绝对措辞，在本块不可观察**）：`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md:468` 逐字写「**两臂的严重度有序**（`Incomplete` 严于 `NotEstablished`）」，并自称「这是本设计的一处刻意余量」。本计划的 `CoverageVerdict`（Task 5）把 `Incomplete` 声明在前，**但既不给 `Ord`、也没有一条用例断言这个序**。**本计划不补 `Ord`**——给一个规范没给的 `Ord` 就是发明（与 §4.3.4 对 `EvidenceStrength` 的处置同向），故这一条在本块**据实不落**，记在此以免被读成漏项。**收件人：设计作者 ＋ 复审者。**
13. **§12.3 的「本块用三条」与它下列的四条不符**（**内部计数不符**）：设计 `:970` 逐字写「本块用**三条**：」，其下 `:972-975` 列了**四条**（第 3 条 `CompletionVerdict::Complete`；第 4 条 `Evidence { .. }` 与 `Evidence::default()`）。本计划落成**五份**样例——Task 10 把第 4 条的两条通道拆开，理由是它们是两条不同的编译失败通道（字段私有 vs 无 `Default` 实现）。**设计那句的计数与它自己的列举不符**，此处报出。**收件人：设计作者。**

### 二、本计划自定的形状与取值（**申报**，逐条说清代价）

- **`VerificationProfileSource` 取「有档案 / 没有档案」两臂**（Task 4）：设计 §5.3 明写「它的形态**本轮不决定**」，而 `profile_for` 必须有一个入参类型。**代价**：这一枚的形状是本计划定的，`OPEN-009` 解出之后多半要改；**收益**：`Missing ⇒ Err` 那条 fail-open 侧有照片。
- **`VerifierSelection` 取 `level` / `backend` / `independence` 三字段**（Task 6）：设计只点名了 `.independence`。**代价**：字段集是本计划定的。
- **`clamp_by_readability` 与 `adversary_permits_passing` 两枚函数名**（Task 6／8）：设计给了规则、没给承载它的名字。**代价**：名字是本计划定的。
- **`Evidence` 的八个只读访问器**（Task 2）：八字段私有是设计的硬要求（§4.4），故必须另给读取面。**代价**：多八个公开读者；**收益**：不必把字段开成 `pub`。
- **`Evidence` 禁止 `Serialize` / `Deserialize` 派生**（Task 2）：见第一节第 8 条。**代价**：想序列化 `Evidence` 的地方要自己拼字段；**收益**：唯一产生点是结构事实。
- **`declared_completion_predicate`「有内容」的判据**（Task 7）：**无内容 ⟺ `None` 或 `Value::Null`**，其余（含 `{}` / `[]` / `""`）算有内容 ⇒ 追加一项不可判。**理由**：另一方向是把用户声明的追加谓词静默丢掉，触 §342 第 1 条的方向。**代价**：空容器被当成有内容。
- **第 10 项的判据表**（Task 7）：`NotIndependent` ⇒ 假；`NotEstablished` ⇒ 不可判；`Independent` ∧ 聚合 ∈ {`Passed`,`PassedWithUnknown`} ⇒ 真；`Independent` 而聚合非通过（`Failed` / `AdjudicationRequired` / `Unknown`）⇒ 假。设计 §8.3 第 10 项只给了前三条，**第四条（`Independent` 而聚合非通过）由本计划补**——不补它，「`Independent` ＋ `Failed`」这一格无判据。**代价**：这一格是本计划定的；**方向**：取假（fail-closed）。**照片**：Task 7 的 `a_non_passing_aggregate_makes_the_tenth_item_false`（三臂逐条，`Failed` / `AdjudicationRequired` / `Unknown`）。
- **空 `VerdictAggregate` 落在 `Unknown` 臂**（Task 6）：§7.5 的措辞是「无 `Fail`、无 `Pass`（全是 `Unknown`）」，空集是否落在这一臂未写。取「落在此臂」**而不是** `Passed`。**代价**：这一处的读法是本计划定的。
- **`AdversaryOutcome` 的编码只覆盖判别串**（Task 8）：`CounterexampleFound` 的载荷（`passed_verifier` / `counterexample`）不进 `strategy_verdict` 列。**代价**：反例的文本与它的出处不落库；**收益**：不为一列新造一套 JSON 约定（规范未给）。
- **`producer_node` 列可空**（Task 9）：见第一节第 7 条。
- **`ContractRequirement` 这一枚类型名**（Task 5）：设计给了「一条 requirement 的两个字段（`id` / `class`）＋ `verification_requirement` 只读不判」这条读法（§4.3.1、§6.1），**没给承载类型**。本块取一枚三字段结构体。**该名在本设计与 P4 设计里都零命中**（实测两份 `grep -c ContractRequirement` 均为 0），故它是本计划的**发明**，不是从设计里抄来的名字。**代价**：名字是本计划定的；**收益**：`ContractView.requirements` 有一个具名元素类型，而不是一枚三元组。
- **四个持久层函数名 `insert_evidence` / `insert_test_validity` / `record_verification_round` / `artifact_exists`**（Task 9）：设计 §10.2 给了表与列、§4.1 给了「只追加」、§9.1 给了「消费规则落在写行处」，**一个函数名都没给**。本计划取这四个（严重度低于上一条：设计给了列，只是没给名）。**代价**：名字是本计划定的；**收益**：Task 9 的照片与 Task 11 的装配都有具名调用点。
- **迁移 130 的 SQL 一次写齐五张表**（Task 9）：设计与 P4 的先例允许逐表追加，但本块的五个表同属一次判定链，一次写齐比五次追加少四次「改同一条 SQL」的机会。**Task 11 之后不得再改它的 SQL**（理由见 Global Constraints）。

### 三、拍不到的照片（设计 §12.4 已列 5 条，**逐条照原文，本计划不发明**）；本计划另补几处

**设计 §12.4 的五条**：（1）端到端（一个节点从 `RUNNING` 经 `VERIFYING` 走到某处）——执行器未建，本块全部照片都是单元与集成层；（2）§264/§215 的效果——在「不放行」的判定面上没有可观察的完成后果，只能拍到「finding 被记录 + `strategy_verdict` 被标注」；（3）`strength ↓`（§265）——无 `Ord` 可依赖；（4）「用户经 `Decision` 授权降级」——`Decision` 属 P4 且未建；（5）`mandatory_effects_completed` 的真值——Effect Journal 属边界层，照片在 P2 一侧。

**本计划另补的几处**（逐条列出，不给合计）：

- **`mutation_threshold` 没有读者这件事没有守卫**：它是一条「不存在的东西」，只能由 Task 11 的 grep 与人工通读钉（Task 8 已明写）。**加一个读者即静默**——**这一处今天只有约定**。
- **「唯一产生点」的运行期一侧拍不出**：能拍的只有 Task 10 的两条编译期照片；「没有任何一条运行期路径能绕过 `from_tool_result`」这句话**本计划不写**。
- **`artifact_exists` 的空终端集形状只有 Task 9 一条用例钉，上游没有守卫**（第一节第 9 条）。

### 四、对既有之物的请求（**本计划一处都不实施**，逐条见跨计划前置第四节）

- **`adfir_edge` 无主键、`Edge` 要求两个端口** ⇒ §10.2 第 3 条的不变量今天只能以「写行时同事务核对」实现。**收件人：执行层（P1）＋ 协调者。**
- **执行器的路由**：`Verifying → Completed` 允许而本块不产出许可（§8.2）。**收件人：执行层（P1）＋ 协调者。**
- **本块的判定今天没有生产调用方**：第 3 层未建。**据实记录以免被读成「忘了接线」**（P1 设计 §18 已有同形的一条）。**收件人：执行器的接线 task ＋ 协调者。**

### 五、设计 §15 的十八条与 §14 的十条：**本计划一条都不发明，收件人照原文**

设计 §15 的十八条**逐条以「收件人」结尾**，本计划**不重述其内容**（重述就是第二份转录，正是本项目出错最多之处）。**逐条照收件人归类**，以便「核过」可查：

- **收件人：规范维护者**——第 **1**（`strength` 的取值域与比较规则）、**2**（`claim` 的语法或形态约束）、**4**（§191 五检查的聚合规则）、**5**（`optional_evidence_types` 的用途）、**8**（`A5`/`OPEN-004` 的降级路径）、**12**（`B1`「高价值任务」）条。
- **收件人：规范维护者 ＋ 协调者**——第 **3**（OPEN-001 / `ENG-004` 的显式处置）、**11**（`A6`/`OPEN-009` 的生成者）、**17**（**§189 的 `Verification → 判断证据是否充分` 与本设计「不判充分」相抵**——**这是规范正文里直接要求判充分的那一句**，设计给了两种读法而**不替规范取**）条。
- **收件人：规范维护者 ＋ P4**——第 **7** 条（`Intent.completion_predicate` 的语法与求值器；P4 设计 §16 第 9 条把判定交给本层，本层只到「判有无内容」）。
- **收件人：规范维护者 ＋ 《工程》维护者**——第 **10** 条（§8.3 的 `Verifier 判定 ←` 那一行缺 Contract，与 §263/§8.2 相抵，本设计取并集）。
- **收件人：规范维护者 ＋ P5c**——第 **9** 条（`Artifact.metadata` 的 schema：§29 的「读 resolution metadata」与 §8.3 第 6 项的「测试是否通过」都依赖它）。
- **收件人：规范维护者 ＋ P5a 的下一轮**——第 **16** 条（Test Validity Verifier 的产生者与输入面：§8.3 逐字给「测试代码 + Requirement」，而「测试代码」在本设计里零命中）。
- **收件人：P5c ＋ 协调者**——第 **14** 条（「测试是否通过」的承载处未定义）。
- **收件人：协调者**——第 **6**（§262 的 SHOULD 被当 MUST 用，本块实现成严格取最高可用级——**这是收紧**）、**18**（P5c–P5f 的六行共写由谁加齐；**设计自己已订正过本条的两半**）条。
- **收件人：协调者 ＋ P3d**——第 **13** 条（`C3`：本设计按「§340 属计划审查段、不约束验证侧」读，**这是一条读法**；若该读法被否，缺的是「更强」的判据与它的持有方）。
- **第 15 条**（本块的判定今天没有生产调用方）**设计未给具名收件人**，它自陈「这不是本块的缺口，是第 3 层的」——本计划把它归在第四节，**不替它补收件人**。
- **另有一条不是 §15 的条目**：设计 §12.1 首段把 `BlindVerdict` / `CompletionVerdict` / `AdversaryOutcome` 三枚枚举补进「每臂都要有数目断言」的清单——**本计划 Task 6／7／8 已各落一条**，此处留痕以便「核过」可查。
- **本计划查出的各条设计问题**（第一节）的收件人已逐条写在那一节里。

**设计 §14 的十条对账条目**（「本设计假设了别的块的某枚形状」）**逐条照原文**：

- 第 **1**（待与 P4 对账）、第 **6**（待与 P1 对账，含 (i) `node.rs` 的注记——**本计划已在 Task 4 执行**、(ii) EVIDENCE 边的身份、(iii) 执行器的路由）条的收件人见原文。
- 第 **2**（与 P5c，**已答 2026-10-10**）、第 **3**（与 P5f / P5e，**两块均已答**）、第 **8**（与 P5d，**已答 2026-10-10**）条**已答，本块照答办**：三枚新臂一次落地（Task 1）、`VisualCheck` / `MetadataCheck` 够用故不加臂。
- 第 **4**（待与 P3d）、第 **5**（待与 P2）、第 **7**（待与 P5b）、第 **9**（与 P6 与协调者：迁移号段 ＋ 工具结果的两个消费者）、第 **10**（与装配方：`p5a_verify_migrations()` 的名字与返回类型照既有八处的形状——**本计划 Task 11 已照办**）条**仍开**，**本计划不代裁**。

---

## 交付给谁

- **交给执行器（第 3 层，未建）**：`Evidence::from_tool_result` 的调用、`InitialVerifierInput::assemble` 的装配、`record_verification_round` 的调用，以及 `Blocked` / `Undetermined` 的路由（节点 `FAILED`？重试？升 `Decision`？——**本块只给两个返回值**）。
- **交给 P5c / P5d / P5e / P5f**：**一律不写域包装**（协调者 2026-10-10 的裁定），证据由宿主执行代码直接调 `from_tool_result`，`subject` 取 `Unattached`；四块的产出面（算子 → `EvidenceType`）是**声明**。四块对 `continuum-verify` 的生产代码调用为零。
- **交给第 7 层（长期循环，未建）**：`EvidenceId` 与只追加的证据表（§10）。**本块不提供 `ProductReadiness`**，**也不建 `Evidence` 的读取路径**（设计未给消费者，见 `## 遗留` 第三节）。
- **交给协调者**：本计划查出的**各条设计问题**（`## 遗留` 第一节）——其中**第 1 条是开工前必须知道的一条**（`RequirementId` 的第二份 vs 不给语义层登记边，且 P4 的 crate 今天不存在）；**排期上整块排在 P4 合入之后**——不只是 Task 1 与 Task 11 这两处多写者改动：Task 2–10 全部依赖 Task 1 建出的 crate（跨计划前置第三节）。
- **交给规范维护者（本项目无此角色）**：设计 §15 的十八条与 §14 的十条对账条目，**逐条照原文**，收件人归类见 `## 遗留` 第五节。



