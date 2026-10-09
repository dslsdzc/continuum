# P5c 代码领域算子 — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md` §五 的 P5c 行的原话，逐字照用）：

> 把代码任务的执行链（L2 隔离工作区上的规划 → 拆任务 → 实现 → 规格审查 → 质量审查 → 验证，《总纲》§8.3）
> 落成一组可注册的 Operator：各步的端口类型、determinism、side_effect_class、backend 候选，
> 并让每步产出 P5a 的 Evidence

**本块拥有 / 产出的接口**（同行的第二栏）：本领域的 Operator 集与它在 P5b 目录里的条目。

**规范依据**：§16 §186 §244 §245 §239；《工程》§8.1 第 11 行（`docs/02-工程.md:495`）、§8.3；《总纲》§8.3（`docs/01-总纲.md:1441`）。

**本文件只写 P5c。** 其余五块（P5a 验证与证据、P5b 执行方法库、P5d/P5e/P5f 三个领域算子）各有其设计，
本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §11 写成一条**具名的对账条目**，
不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（判据与订正原文见 `docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p5c`）里当场 `grep -n` / `awk` 实测的，
未照抄任何一份别的文档（含切分文档与 P5a/P5b/P5e 三份设计）的行号。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| 执行链各步落成可注册的 `Operator` | 切分 §五 P5c 行；《总纲》§8.3 `:1443-1449`；§186 `:57-77` | §3 | 全量（9 枚算子：端口类型、determinism、side_effect_class、backend 候选，逐枚给理由） |
| L2 隔离工作区的位置与端口 | 切分 §五 P5c 行；§16 `:788-835`；§255 `:1021-1037`；§256 `:1041-1049`；§257 `:1053-1069` | §4 | 位置与端口全量；**两处结构缺口据实报出**（§4.3、§4.4） |
| 每步产出 P5a 的 `Evidence` | 切分 §四第 1 条 | §5 | 逐枚给出产出面；**两枚审查算子无臂可落，向 P5a 报出**（§5.3） |
| 与 P5a 的接缝（只产出、只消费） | 切分 §四第 1 条 | §6 | 直接经 P5a 的唯一构造点；**本块不建第二枚包装**（§6.2） |
| `software/` 目录里的条目 | P5b 设计第六节（`docs/superpowers/specs/2026-10-08-p5b-execution-method-library-design.md`） | §7 | 七条逐条填出；**两条留空并报出结构性不匹配**（§7.2） |
| `ArtifactType` 够不够 | 切分 §四第 2 条 | §8 | **本链零请求**；向 P5e 报出「无」 |
| 迁移号段 | 切分 §二第 5 条 | §9 | **本块零迁移**；档号 **150**（实测未占用，写死报出） |

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| `Evidence` 的类型、`VerificationPolicy`、覆盖判定、完成判定、Verifier 的级别序与隔离输入 | P5a（切分 §四第 1 条）。本块**不得**定义证据类型、自定义「充分」判据、自定义完成判定 |
| 方法（Method）的登记形态、`MethodRegistry`、`MethodEntry`、`MethodDomain` | P5b（切分 §四第 3 条：在 P5b 过审前不得自造）。本块只调它的 `bind` |
| 节点状态迁移、`Queued → Running`、`OperatorRegistry::resolve` 的调用点、算子的执行 | 第 3 层（切分 §四第 4 条）。本块只**注册**算子 |
| `ArtifactType` 的扩展 | P5e（切分 §四第 2 条：只由 P5e 一处改）。本块**不请求任何新变体**（§8） |
| `Checkpointable` 的错误类型 | P5e（切分 §二第 4 条）。本块**不造第二个错误类型**（§2.2 末） |
| 复用判据（缓存键、`can_reuse`） | P1 已落地（`crates/continuum-graph/src/reuse.rs:29`）。本块**不建第二份**（§3.6） |
| §239 的端口兼容判定、§245 的「后端是否在候选内」判定 | P1 已落地（`crates/continuum-port/src/port.rs:94` 的 `compatible`；`crates/continuum-graph/src/execution.rs:87` 的 `is_candidate_backend`）。本块只**声明**自己的 schema 与候选（§3.5） |
| §229 的 Plan Review 状态机、§338–§341 的审查三层结构 | P4（P4 设计 §9） |
| `Node.execution_policy` 的判定 | P4（切分 §四第 5 条：该字段的注记逐字「结构保存，判定属 P4」，字段在 `crates/continuum-graph/src/node.rs:18`、注记在 `:17`）。本块只碰 `verification_policy`（同文件 `:20`，注记「结构保存，判定属 P5」） |
| `Task Contract` / `Requirement` / `Plan` / `Decision` 对象 | P4（`continuum-semantics`，该 crate 今天在 `crates/` 下不存在） |
| L2 工作区的创建、放弃与只读强制本身 | P2 已落地（`crates/continuum-workspace`）。本块只声明一枚算子来**命名**它（§4.2） |
| 内核层沙箱（§159、§256 的强制实现） | P2（`crates/continuum-sandbox`）。本块不把它做成算子，也不登记到它的边 |
| 迁移 / 表 | 本块**零表**（§9） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：`continuum-code`（实测：`grep -rn "continuum-code" . --exclude-dir=.git` 在本仓**零命中**，名字未占用）。

依赖边表（**只登记实际用到的**，切分 §四第 6 条）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-artifact` | `ArtifactType`（§3.2 的端口类型经它声明）、`ArtifactId`（§5.2 的证据产出） | 普通 |
| `continuum-operator` | `Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` / `BackendId` / `OperatorRegistry` / `OperatorError` | 普通 |
| `continuum-method` | `MethodRegistry` / `MethodId` / `MethodError` / `bind`（§7.1 的 `bind_code_methods` 收 `&mut MethodRegistry`、以 `MethodId` 逐条调 `bind` 并透出 `MethodError`） | 普通 |
| `continuum-verify` | `Evidence` / `EvidenceType` / `EvidenceProducer` / `EvidenceSubject` / `Claim` / `EvidenceStrength` / `EvidenceScope` / `EvidenceId` / `Evidence::from_tool_result` / `VerifyError`（切分 §四第 1 条：四块**只产出**证据） | 普通 |
| `continuum-graph` | `can_reuse` / `cache_key` / `CacheKey`（§3.6 的照片：核本块声明的 `determinism` 真能让 §305 的判据成立） | **dev 边** |
| `continuum-workspace` | `WorkspaceBackend`（§4.2 的照片：核 `code-workspace` 的两个 backend 名与 P2 已落地的两臂逐名一致，`crates/continuum-workspace/src/backend.rs:22`） | **dev 边** |

`continuum-method` 是 P5b 新建的 crate，其 workspace 依赖集合为空集（P5b 设计第九节）；本块依赖它，
不构成环：`continuum-method` 不反向依赖任何 workspace crate，故 `continuum-code → continuum-method` 是单向的。

**不登记**的边，逐条给理由（零使用的边即假边，P2b 为此删过两条）：

- **不登记 `continuum-port`**：本块不构造 `Port`、不做端口兼容判定。`Operator.input_schema` / `output_schema`
  取 `Vec<ArtifactType>`（`crates/continuum-operator/src/definition.rs:82-83`），不经 `Port`。§239 的 `compatible`
  是 P1 的（`crates/continuum-port/src/port.rs:94`）。
- **不登记 `continuum-semantics`**：本块不构造 `RequirementId`——产出的证据一律 `Unattached`（§5.2）。
  该 crate 今天在 `crates/` 下不存在（P4 未开始实现，切分 §二末「与 P4 的接口面」段已记）。
- **不登记 `continuum-persist`**：本块零表（§9），不碰 `Tx` / `Migration`。
- **不登记 `continuum-events`**：本块不写事件（§9）。`EventType::VerificationFailed`（`crates/continuum-events/src/event.rs:25`）
  的产生点是 P5a，不是本块（P5a 设计 §10.3）。
- **不登记 `continuum-policy` / `continuum-capability` / `continuum-secret` / `continuum-sandbox`**：
  本块不查策略引擎、不取 Capability Token、不构造 `Sandbox`。代码类算子的**执行**（含沙箱的施加、
  能力凭据的取用）都在第 3 层（切分 §四第 4 条），本块只声明 `backend_candidates` 这个**名单**。
- **不登记 `continuum-core`**：实测本块用到的每一枚类型都在上表五个 crate 里，无一来自 `continuum-core`。
  按「叶子 crate 的条目记实际依赖」的口径（`crates/continuum-runtime/tests/dependency_direction.rs` 里
  `continuum-workspace` / `continuum-sandbox` / `continuum-policy` 三行都逐字写了同一条），一条零使用的边不登记。

**dev 边为什么也要登记**：`dependency_direction.rs` 的 `cargo tree` 带 `--edges all`，dev 边与普通边一视同仁。
本仓已有同形的先例（`continuum-provider` 那一行注记明写「Task 4 起加上 persist：**dev 边**」，同文件 `:32-34`）。

**边别按「谁在用」判**：`continuum-graph` 与 `continuum-workspace` 是 **dev 边**——生产代码不调它们；
用到它们的是 §10.2 的两条运行期断言（第 (o) 与 (p) 条），那是用例。若实现期发现生产代码要调它们
（例如 `code-workspace` 的实现体），边别随之升为普通边，本设计**不预先**把它登记成普通边。

**共写文件**（切分 §一 的共写文件表给的是五处；下表是本块的那部分，**两行**）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `crates/continuum-code` | 切分建议由先落地者一次加齐六行；本块**只加自己这一行** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-code", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify", "continuum-workspace"])`——**六项**，与 §2.1 边表的六行逐行对应（含两条 dev 边） | 新 crate 会先让该门变红再被补齐，这是刻意的；数组按字母序 |

**本块对共写表的改动面是五处里最小的两处**：`crates/continuum-artifact/src/artifact.rs` 归 P5e、
`crates/continuum-graph/src/node.rs` 归 P5a、`crates/continuum-operator/src/definition.rs` 归 P5e。
本块**不碰这三处**——这一条是 §8「零请求」与 §5.2「不建第二枚包装」的直接后果。

## 2.2 冻结清单：本块持有的类型与判定

**类型**（定义在 `continuum-code`）：

```
CodeError
```

另有一枚**构造函数**（不是类型）：`all_operators() -> [Operator; 9]`，它承载的正是 `Operator`（P1 的类型）。
（**这里不是 `const`**：`Operator` 在 crate 外构造不出常量值，两条理由见 §3.2。）
**本块不新造算子类型**——§3.2 的登记形态照 P1 与 P5b。

**本块持有的类型只有一枚，且它只承载注册期的两件事**（§3.2 与 §3.5 的形状校验）。
**这不是省事**：切分 §四第 7 条要求「判据只许有一处」，而本块**独占**的判定逐条列在 §2.4（**十件**），
本块**用到的**其余判定都在仓里已有落点（§3.5、§3.6 逐条给）。
**这里不写成「本层每一件判定」**：那会把本块**不持有**的那些也算进来，而它们之中有的虽属别的块、
今天在仓里**并无落点**——见下面「本块不持有的判定」一栏里 P5a 的四件（其证据充分性判据记为 OPEN-001，尚无判据）。

**不新增第二个错误类型**：切分 §二第 4 条把「第二份错误类型」的禁令写给了 `Checkpointable` 那一处（归 P5e）；
本块同理不另造。`OperatorError`（`crates/continuum-operator/src/registry.rs:7-12`）与 `MethodError`（P5b 设计 §4.4）
各自带出来，不重述（§3.2 的 `CodeError` 两臂逐臂注明判定的持有者）。

**判定**（本块是它们的唯一落点，共**十件**，逐条见 §2.4；其中与仓里已有落点最相邻的两件是）：

1. 本领域 9 枚算子的 `determinism` 声明所依赖的「backend 逐位可复现」名单（§3.5）。
2. L2 工作区的两个 backend 名与 §255 的两种形态的对应（§4.2）。

**本块不持有的判定**：证据的充分性、覆盖、verifier 的级别、完成判定（全部 P5a）；方法的登记与选取（P5b）；
复用的四条件（P1 的 `can_reuse`，§3.6）；端口兼容（P1 的 `compatible`，§3.5）；后端是否在候选内（P1 的
`is_candidate_backend`，§3.5）。
**这八件里，落到别块且今天在仓里已有落点的是后四件**（P5b 的方法登记、P1 的三件）；
**前四件（P5a 的四件）今天在仓里没有落点**——就其中第一件（证据的充分性）而言，P5a 设计 §3 把「一堆有效证据合起来是否算满足」记为 **OPEN-001**，并明写本块**不判**它、也不宣布关闭它。
「不持有」只说判定归谁，**不含「那个落点已存在」**。

**上面那八条判定「不持有」的反面是下面的 §2.4**：规格没给、而本设计不得不定的每一处判定，
在那里逐条给出**代价、边界、落点与具名收件人**。**没进 §2.4 那张表的判定，本块一处也没有定。**

## 2.3 本块的层间位置

§9.1（`docs/02-工程.md` 的层间图，读法在同文件 `:561-567`，`:588` 逐字「依赖方向单向，无环。」）
里与本块有关的两条，按「被依赖者 → 依赖者」读：

- `执行层 (3) → 跨领域 (8)`：§2.1 的边表**六行**全是它的实例。`verification_policy` 的定型落在消费侧
  而不是字段上，也是同一条的后果（切分 §二第 2 条的裁定段；本块不重复它）。
- `跨领域 (8) → 长期循环 (7)`：本块**不接**这一条——本块不产出 `value + evidence + last_verified`
  （§232 的 `ProductReadiness` 属长期循环）。本块的产物注册进 `OperatorRegistry`，不写长期循环的表。

本块与 P5a（同层）之间有一条层内边 `continuum-code → continuum-verify`。**层内边在本仓有同形的处置**：
`continuum-provider → continuum-capability`（资源层内的「接口 → 能力类型」）就被登记为层内边
（`crates/continuum-runtime/tests/dependency_direction.rs:30` 的注记，与 P3c 设计 §4.1 订正段逐字「层内边」）。
本条边是单向的：`continuum-verify` 不依赖任何 P5 领域算子块（P5a 设计 §2.1 的边表里没有它们）。

**本块与 P2 之间没有普通边**，只有一条 dev 边（§2.1 第 6 行）。这一条要单独读出：
L2 工作区的创建在 P2（第 5 层），而本块只在**声明**里命名它的两个 backend——两层的相接处
是「算子的执行」，那在第 3 层（切分 §四第 4 条），故本块不登记一条生产边。

## 2.4 本块发明的判据（逐条：代价、边界、落点、具名收件人）

**这一节是本块的「发明判据」总账。** 规格没给、而本设计不得不定的每一处判定，都在下表里有一条；
**没进这张表的判定，本块一处也没有定**（§2.2 的「本块不持有的判定」那一栏是它的反面）。
表的每一行给**四件**：代价、边界（这条判定拦不住什么）、落点（判定写在哪个函数/表/用例上）、收件人。
**凡本节的行都不得只留在代码块或注释里**——**代码块只是示意，正文措辞才是约束**。

| # | 发明的判定 | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | 9 枚算子各自的 `determinism` 取值 | §244 `:737-750` 只给字段名，**规范没有一处给过 operator 的 determinism** | §3.3 第 5、7 条：判 `NonDeterministic` 的七枚不进 §305 的复用，每轮重算；判 `Deterministic` 的两枚进复用 | 它**不判**「真实输入是否在端口上」——那一类由 §3.3 第 3 条与第 (o) 条用例管（§3.5 末） | §3.2 表的 `determinism` 列；照片 §10.2 第 (c)(g)(o) 条 | 规范维护者 |
| 2 | 9 枚算子各自的 `side_effect_class` 取值 | §244 同上；§307 `:2033` 只给「非幂等 Effect MUST NOT 直接自动重试」 | §3.3 第 8 条：九枚**都不是** `NonIdempotent`（第 4 行 2026-10-10 由 `NonIdempotent` 改判 `Pure`），故 §307 的禁令在本块今天没有主体 | 它只影响重试策略；**不判**「重试几次、退避多少」（§307 的 `RetryPolicy` 字段面属 P1） | §3.2 表的 `side_effect_class` 列；照片 §10.2 第 (f) 条 | 规范维护者 |
| 3 | 9 枚算子各自的 `backend_candidates` 名单 | §245 `:754-773` 只给「一个 Operator MAY 有多个 backend」与一个例（`Transcribe` 三 backend），**不给取值域** | §3.3 第 3 条（`code-workspace` 只列 §255 的两种形态，不列 §255 `:1029-1037` 的五种） | 名单是「Router 可以选的东西」，**不判**「Router 实际选哪个」（§245 `:773` 逐字把选择判给 Router） | §3.2 表的 backend 列；照片 §10.2 第 (h)(p) 条 | 规范维护者 |
| 4 | 「逐位可复现的 backend」封闭名单（5 项）与注册期规则 | 无（§245 只说多 backend） | §3.5：远程执行不可用（性能代价，非正确性代价） | **拦不住**「候选都可复现、但真实输入不在端口上」那一类（§3.5 末已写明） | `register_code_operators`（§3.2）的核；错误臂 `CodeError::UnverifiableDeterminism`；照片 §10.2 第 (c)(d) 条 | 规范维护者（§11 第 7 条） |
| 5 | §3.4 边表的**边判据** | §186/《总纲》§8.3 给的是**步骤名与步骤序**；§239 `:654` 只给「运行时 MUST 拒绝不兼容连接」 | 无（它是注册期的相容性核对） | 它只判**相邻两枚的 schema 相交**；**不判**「图上的边该不该存在」（那是 Planner 的构造） | §3.4 边表；照片 §10.2 第 (j) 条 | 规范维护者（以「§3.2 第 N 行」为依据的那几行） |
| 6 | L2 工作区**落成算子**、Integration Gate **不落成算子** | §16 `:788-835` 说它是代码任务的基础，**没说它是不是算子**；§257 `:1053-1069` 说 Gate 的 MUST，**没说它的执行体** | §4.2 逐条给了两半的代价；替代方案（把 Gate 也算子化）被否的理由是「批准值没有位置装」 | 「每 Intent 一枚」这条**在类型层不可钉**（§4.2 末） | §4.2 的裁定；声明落点 `all_operators()` 返回的第 1 行 | 协调者（§11 第 4 条） |
| 7 | §7.1 表里 `realized_by` 的映射 | §187 `:116-142` 只给方法名；P5b 的 `MethodEntry` 只给 `Vec<String>` 的容器 | §7.2：`TDD` 与 `debugging` 填空，代价是 P5b 判据 6 在 `Software` 域变红 | 它**表达不了次序与环境**（§7.2 第 1 条） | §7.1 表；入口 `bind_code_methods`；照片 §10.2 第 (k) 条 | P5b 的持有方 ＋ 协调者（§11 第 2 条） |
| 8 | 链上**哪几步产 `Evidence`** 的分界 | §89 `:923-945` 只说「工具结果必须转换成 Evidence」，**不说每一步都产** | §5.2：第 1–4 行四枚不产证据，故这四步在链上没有「语义成败」的证据承载——它们对不对只能靠各自的审查（§229 的 Plan Review，P4）与下游算子的证据间接体现 | 它**不判**「一条证据够不够」（那是 OPEN-001，P5a 设计 §3 的处置） | §5.2 表（第 1–4 行四枚不产证据，其制品的存在性由 §266 第 1 项判） | P5a（§11 第 1 条） |
| 9 | 迁移档号 `150` | 切分 §二第 5 条只要求「写死后报出」，**不分配号** | §9.3：无（零表，登记的是归属不是 SQL） | 它**只声明本块那一段**；P5d/P5f/P6 三段本块无从声明 | §9.3 | 协调者（§11 第 6 条） |
| 10 | 9 枚算子各自的 `input_schema` / `output_schema` 取值 | §244 `:737-750` 只给字段名与 `Vec<ArtifactType>` 的类型，**规范没有一处给过任一算子的端口取值** | §3.4 的边表与本表建立在这两列上：端口的型一旦改，边表与 §10.2 第 (j) 条的语料随之改，而规范未给边、也无论证可据 | **拦不住**「上游真给了这个型」——它只声明本块算子的端口形状；连接期是否相容由 P1 的 `compatible` 判（§3.4） | §3.2 表的 `input_schema` / `output_schema` 两列；照片 §10.2 第 (h)(i)(j) 条 | 规范维护者 |

**另有两处「发明的判定」不在本块，但本块报了出**（它们由别的块持有，本块只报需求）：
§5.3 的 `ModelReview` 臂（P5a）与 §5.4 的 `TestResult` 载荷 schema（规范维护者 ＋ P5a）。
**报出而不是发明**，是切分 §四第 1 条与 §四第 2 条在本块的那两处兑现。

---

# 3. 执行链与算子集

## 3.1 链与步骤的出处（逐行实测）

《总纲》§8.3（`docs/01-总纲.md`）的两句，逐字：

- `:1443`：「代码任务的基础是 §5.1 的隔离工作区。L2 默认使用 git worktree，每个 Intent 一个独立分支（§16）。」
- `:1445`：「执行方法的默认形态（§186）：」，其后 `:1447-1449` 的代码块 `:1448` 是那一条链：

```
Milestone → 规划 → 拆任务 → 全新 subagent 实现 → 规格审查 → 质量审查 → 验证
```

§186（`docs/spec/04-method.md:53` 起）给的是同一条链的**更细的一版**，`:57-77` 逐行：
`Milestone`（`:58`）→ `Requirement / Design Review`（`:60`）→ `Implementation Plan`（`:62`）→
`Engineering Tasks`（`:64`）→ `Fresh Subagent`（`:66`）→ `Implement`（`:68`）→ `Spec Review`（`:70`）→
`Quality Review`（`:72`）→ `Verification`（`:74`）→ `Next Task`（`:76`）。

**两版的三处差**，逐处处置（**本设计取《总纲》`:1448` 的六个步骤为本块的链**，
它是规范正文里的摘要，而 §186 是同一件事的展开；两者不冲突，只是详略不同）：

| §186 多出来的 | 处置 |
|---|---|
| `Requirement / Design Review`（`:60`） | **不入本块**：它是 §229 的 Plan Review 状态机的一部分，属 P4（P4 设计 §9）。本块不为它注册算子 |
| `Fresh Subagent`（`:66`） | **不是一步**：它是 `Implement` 这一步的**隔离属性**（§186 `:83` 逐字「任务上下文隔离」），与 §263 的 Verifier 隔离同源但不同物。故它不落成算子，落成 §3.3 第 3 行对 `code-implement` 的 `backend_candidates` 的一项要求 |
| `Next Task`（`:76`） | **不是一步**：它是链的回边（下一轮任务），而回边是 ADFIR 的图结构（P1），不是算子 |

**「默认形态」这四个字要读出**：`:1445` 逐字是「执行方法的**默认**形态」。故上面这六个步骤是**默认**的链，
不是穷尽的算子清单。§3.2 的 9 枚里有 3 枚不在这六个步骤上，其正当性逐枚在 §3.3 注明；
**但这不构成「可以随手加算子」的许可**：本块只在两个来源下加算子——①规范点名的步骤，②P5b 交办的
`software/` 目录条目需要一枚对家（§7）。

## 3.2 九枚算子

出处列的口径：**算子名与端口类型**在规范给了的地方照录，**`determinism` / `side_effect_class` /
backend 候选**在规范未给处由本设计定，并逐行标出「定」字（与 P5e 设计 §5.2 的口径同）。

| # | id | 用途 | input_schema | output_schema | determinism | side_effect_class | backend 候选 | 出处 |
|---|---|---|---|---|---|---|---|---|
| 1 | `code-workspace` | 建立 L2 隔离工作区 | `[]` | `[SourceTree]` | `NonDeterministic`（定） | `Idempotent`（定） | `worktree` `overlay`（定；取 P2 `WorkspaceBackend` 两臂的编码，§3.5 末） | §16 `:790-794`；§255 `:1023-1037` |
| 2 | `code-plan` | 规划 | `[Json]` | `[Json]` | `NonDeterministic`（定） | `Pure`（定） | `primary-model`（定） | 《总纲》§8.3 `:1448`；§186 `:62` |
| 3 | `code-decompose` | 拆任务 | `[Json]` | `[Json]` | `NonDeterministic`（定） | `Pure`（定） | `primary-model`（定） | 《总纲》§8.3 `:1448`；§186 `:64` |
| 4 | `code-implement` | 全新 subagent 实现 | `[Json]` | `[Patch]` | `NonDeterministic`（定） | `Pure`（定） | `coding-agent` `local-agent-cli`（定） | 《总纲》§8.3 `:1448`；§186 `:66-68` |
| 5 | `code-spec-review` | 规格审查 | `[Patch, Json]` | `[Json]` | `NonDeterministic`（定） | `Pure`（定） | `independent-reviewer-model`（定） | §186 `:70`；`:85` |
| 6 | `code-quality-review` | 质量审查 | `[Patch, Json]` | `[Json]` | `NonDeterministic`（定） | `Pure`（定） | `independent-reviewer-model`（定） | §186 `:72`；`:85` |
| 7 | `code-test-run` | 验证：跑测试，产 `TestResult` | `[SourceTree, Patch]` | `[TestResult]` | `Deterministic`（定） | `Idempotent`（定） | `cargo-test` `local-test-runner`（定） | §186 `:74`；§259 `:1109-1119` |
| 8 | `code-fuzz-run` | 验证：§193 的 Fuzzing / Property | `[SourceTree, Patch]` | `[TestResult]` | `NonDeterministic`（定） | `Idempotent`（定） | `cargo-fuzz` `proptest`（定） | §193 `:361-374`；P5b 设计第六节的 `fuzz` 行 |
| 9 | `code-verify` | 验证：§262 第 1 级的确定性判定器 | `[TestResult, Patch]` | `[Json]` | `Deterministic`（定） | `Pure`（定） | `builtin`（定） | §186 `:74`；§262 `:1173-1179` |

注册入口的形状照 `OperatorRegistry`（`crates/continuum-operator/src/registry.rs:15-48`）：

```
/// 本领域 9 枚算子的全集。顺序即 §3.2 表的行序。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**：理由见下（`Operator` 在 crate 外构造不出常量值）。
/// 数目 9 写在**返回类型**里，使清单个数的改动编译期可见。
pub fn all_operators() -> [Operator; 9];

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核 §3.5 的声明，全部通过后再逐枚注册；任一枚不成即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚代码域算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§3.5 那条规则要能对**任意** `Operator` 触发，
/// §10.2 第 (d) 条才写得出来。
pub fn register_code_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), CodeError>;
```

`CodeError` 的形状（§2.2 列出的本块唯一一枚类型）。两臂，逐臂给出**判定的持有者**：

```
#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    /// §3.5 的注册期规则不成立：声明 `Deterministic` 的算子混进了名单外的 backend。
    /// 判定的持有者在本块（§3.5）。
    #[error("算子 {operator} 的候选 backend {backend} 不在逐位可复现名单内（§3.5）")]
    UnverifiableDeterminism { operator: OperatorId, backend: BackendId },

    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1（见下段）。
    #[error(transparent)]
    Registry(#[from] OperatorError),
}
```

**`CodeError` 只有两臂，不是三臂**：P5e 的 `MediaError` 有第三臂 `NotPermitted`（§34 的生成授权），
而代码域**没有对应的授权面**——§34 管的是「生成内容替换原始素材」，代码域的算子里没有任何一枚
产「生成内容」（它们产 `Plan` / `Patch` / `TestResult`）。**这里不是「为对称补一臂」：一条永远
进不去的臂与一个死字段同类**（P5b 设计 §4.3 第 2 条对它删 `purpose` 时用的正是这条理由）。

**`Registry` 这一臂为什么是包装而不是同名臂**：`(id, version)` 撞车的判定**已经有一枚臂**
（`crates/continuum-operator/src/registry.rs:11` 的 `OperatorError::Duplicate`，判定在 `:24-34` 的
`OperatorRegistry::register`），本块再立一枚 `CodeError::Duplicate` 就是「注册表里有没有这个算子」
的第二套词汇表。**判定是同一件事则合并，是两件事则分开**——同一条判据在 §7.1 末用在了 `MethodError` 上，
方向相反。

`all_operators()` 是**函数不是常量**。**这里不是 `const`**，两条理由各自独立成立：
其一，本仓带 `ALL` 常量的既有做法（`crates/continuum-artifact/src/artifact.rs:28` 的
`ArtifactType::ALL`、`crates/continuum-events/src/audit.rs:38` 的 `AuditKind::ALL`）**成立的前提是那些类型无字段**
（实测两处都是无字段枚举，取值可在 `const` 中直接枚举），`Operator` 不满足这个前提；
其二，`Operator` 含 `OperatorId`（`String`，**字段私有**，`new` 不是 `const fn`，
`crates/continuum-operator/src/definition.rs:24-27`）与三枚 `Vec` 字段
（`input_schema` / `output_schema` / `backend_candidates`，`:79-88`），
故在 crate 外**构造不出 `Operator` 的常量值**。
**引用一条既有约定时要连它的成立条件一起搬**。数目 9 改由**返回类型**承载
（`[Operator; 9]`），仍使清单个数的改动**编译期可见**——这正是本行要保住的那条性质。

## 3.3 逐枚的判定理由（只写需要解释的）

1. **第 1 行 `code-workspace` 不在《总纲》`:1448` 的六个步骤上**，它的来源是两处，各自独立：
   ①§16 `:790-794` 与 §255 `:1023-1027` 把 L2 工作区定为代码任务的基础（《总纲》`:1443` 逐字重复了这一点）；
   ②P5b 设计第六节把 §187 的 `worktree` 条目交给本块，要求「填其 `realized_by` 时须与 §16 的 git worktree
   语义一致」。**若不给这一枚算子，「L2 隔离工作区」在这条链上没有任何可注册的落点**——
   它就只能是一条注释，而注释不是登记。它的端口与两处结构缺口见 §4。

2. **第 1 行的 `input_schema` 为空**：它的输入是 **Base 工作区**，而 Base 不是制品。
   §244 的字段是 `Vec<ArtifactType>`，「零个输入」在类型里可表达。
   **这里不是「把 Base 做成一个 `SourceTree` 制品再给它」：那会声称 Base 是这条链的产物之一，
   而 Base 恰是链外的、用户的工作区（§5.1 `:769-770` 逐字 `BaseWorkspace = READ ONLY`）**——
   给它一个制品身份，等于让下游可以把 Base 当输入制品消费，而 §256 `:1046` 的
   `AI_WRITE(BaseWorkspace) = DENY` 正是要拦住这件事。代价是 Base 的形态不进端口，见 §4.3。

3. **第 1 行是 `NonDeterministic`**（这一条是 fail-closed 的一侧，代价写全）：
   它的真实输入（Base 的当前工作树）**不在端口上**（上一条），故 §305（`docs/spec/05-normative.md:1986-1998`）
   的 `input_hash` 无从取得。若声明 `Deterministic`，`cache_key` 会给出 `Some`，而 §305 的三条件里
   「`input_hash` 未变」这一条在**前提不成立**的地方被当成成立——上游 Base 动过之后，一份**旧的工作区树**
   会冒充本次的结果。取 `NonDeterministic` 的代价是每轮都重跑一次工作区创建（或撞上「同一 Intent 已存在」
   而失败，见第 4 行）；与「复用一棵陈旧的工作区树」相比取向明确。

4. **第 1 行是 `Idempotent` 而不是 `NonIdempotent`**：它的效果是「使该 Intent 的隔离工作区存在」，
   这是一个**目标态**，不是增量操作——重复调用不产生**第二份**工作区（P2 的实现里，同一 Intent 的
   第二次创建会撞上已存在的 `ai/<intent>` 分支或 Intent 目录而返回 `Err`）。它**改变了本仓之外的状态**
   （落下一棵工作区树），故不是 `Pure`；而重复执行得到同一状态，故不是 `NonIdempotent`——这正是
   `Idempotent` 的口径（见 P5d 设计 §4.4 第 3 条）。**`NonIdempotent` 管的是「重复执行会累积外部后果」那一类**
   （§307 `:2033` 点名：向外部服务投递、删除远端资源），本行不属于那一类。**这里不是照 `render` 的 `Idempotent` 抄的**：
   `render` 的 `Idempotent` 讲的是「落下本地文件、重复得到同一状态」，本行讲的是「重复调用不产生第二份」
   ——两条理由不同，只是落在同一个取值上。

5. **第 7 行与第 8 行分作两枚，不合成一枚多 backend 的算子**：分的原因是 `determinism`。
   `code-test-run` 判 `Deterministic`（同一棵树跑同一套测试，结果逐次相同），而 §193 `:368-369` 的
   Fuzzing 与 Property-based Testing 以熵源生成输入，同一棵树两次运行可能给出不同的覆盖与不同的发现，
   故 `code-fuzz-run` 判 `NonDeterministic`。**合成一枚并把 fuzz 放进它的候选里，会让那一枚声明的
   `Deterministic` 当场为假**——这正是 P5e 设计 §5.4 那条规则（`Deterministic` 的算子不得混进
   输出不逐位可复现的 backend）的同向应用。**代价**：`code-fuzz-run` 不进 §305 的复用（`cache_key` 返回
   `None`，`crates/continuum-graph/src/reuse.rs:22`），故同一棵树上的 fuzz 每轮重跑。这是算力代价，
   不是正确性代价。

6. **第 7 行是 `Deterministic`，因此它进复用**（这一条是**收益**，不是缺口）：
   `cache_key`（`crates/continuum-graph/src/reuse.rs:16`）对它给出 `Some`，故输入树未变、算子版本未变、
   Contract 未受影响时，§305 允许直接复用上一轮的 `TestResult`，不重跑测试套件。这与 P5e 设计 §6.2
   把 `shot-detect` 一类算法判定为 `Deterministic` 是同一个取向。

7. **第 2、3、4、5、6 行是 `NonDeterministic`**：五枚都经模型（规划、拆任务、实现、两种审查）。
   **这是本设计的判定，不是规范的**——规范没有一处给过 operator 的 determinism。
   据实记代价：规划与拆任务的产物因此不进 §305 的复用，每轮重算。

8. **九枚都不是 `NonIdempotent`**（第 4 行于 2026-10-10 由 `NonIdempotent` 改判 `Pure`；此前判 `NonIdempotent`
   的理由是「重跑一次实现得到的是**另一份补丁**」）。**那次取值把 `Determinism` 的理由当成了 `SideEffectClass`
   的理由**：模型解码的随机性属 `determinism`（第 4 行本就是 `NonDeterministic`），而「另一份补丁」仍只是
   **一个值**——第 4 行不改变本仓之外的状态。**判据**：「消耗资源」与「改变状态」是两件事，消耗算力记在预算维度
   （§333），不是副作用（口径与依据见 P5d 设计 §4.4 第 3 条）。§307 `:2033` 的禁令在本块**今天没有主体**：
   九枚无一向远端投递或删除；将来出现那类算子时才有主体。

9. **第 7、8 行的 `Idempotent` 而不是 `Pure`**：两枚都在 Task Workspace 里跑构建与测试（写出
   `target/` 一类的中间物），故不是 `Pure`（P5e 设计 §5.2 第 6 条对 `render` 用的是同一条理由）。
   第 9 行是 `Pure`：它是 §262 第 1 级的确定性检查器，只读制品、不写。

10. **第 5 行与第 6 行是两枚算子，不是一枚**：§186 `:85` 逐字「规格审查与代码质量审查分离」——
    **分离这一条本身是 §186 列的四个优势之一**，合成就把它抹掉。两枚的 `input_schema` 相同、
    `determinism` 与 `side_effect_class` 相同，**它们是两个问题**（「这段代码满足了规格吗」与
    「这段代码的质量合格吗」），不是一个问题的两种读法。

11. **第 9 行是「§262 第 1 级的判定器」，不是 P5a 的判定**：§262 `:1173-1179` 的五级序里
    第 1 级是 `deterministic checker`，而**级别序与候选过滤属 P5a**（其设计 §7），
    **比较器属本块**（同 P5e 设计 §5.5 的分工表）。本行是本块给出的那个比较器。

## 3.4 端口类型与 §239 的注册期一半

**先定关系**：上游算子的 `output_schema` 与下游算子的 `input_schema` **相交非空**——
这是 §239 `:654` 逐字「运行时 MUST 拒绝不兼容连接」在**注册期**可检查的那一半。

**边表**（§10.2 第 (j) 条的语料）。每行给「下游算子 ← 输入的来源」，括号里是**该来源的
`output_schema` 与下游 `input_schema` 交集中的型**：

| 下游算子 | 输入的来源（交集中的型） | 依据 |
|---|---|---|
| `code-workspace` | 无输入（Base 不在端口上，§3.3 第 2 条） | §3.2 第 1 行 |
| `code-plan` | **无算子上游**；其输入是 §228 的 `Plan` 或 Contract 的投影，**以 `Json` 值由装配方投射**（不由任何算子的 `output_schema` 提供，故不在 (j) 的断言范围内） | §3.2 第 2 行 |
| `code-decompose` | `code-plan`（`Json`） | §186 `:62`→`:64` |
| `code-implement` | `code-decompose`（`Json`） | §186 `:64`→`:68` |
| `code-spec-review` | `code-decompose`（`Json`）、`code-implement`（`Patch`） | §186 `:68`→`:70` |
| `code-quality-review` | `code-decompose`（`Json`）、`code-implement`（`Patch`） | §186 `:68`→`:72` |
| `code-test-run` | `code-workspace`（`SourceTree`）、`code-implement`（`Patch`） | §186 `:74`；§3.2 第 7 行 |
| `code-fuzz-run` | `code-workspace`（`SourceTree`）、`code-implement`（`Patch`） | §193 `:361` |
| `code-verify` | `code-test-run`（`TestResult`）、`code-implement`（`Patch`） | §186 `:74`；§3.2 第 9 行 |

**依据栏的读法**：写 §186/§193 的，出处是规范正文的步骤序；写「§3.2 第 N 行」的，出处是
**该行的 `input_schema` 与全表 `output_schema` 的对应**（即本设计的判定，规范未给边）。

**判据（§10.2 第 (j) 条）**：逐行断言下游 `input_schema` 的**每一个型**都能在该行列出的上游里找到，
且所引的那一枚的 `output_schema` 含该型。断言的对象是「交集中的型」，不是「两侧相等」——
P5e 设计 §5.3 已把「相等」那一条读法否掉，本块引用它而不重述。

**一处例外，要写清（否则会被当成判据的反例）**：`code-plan` 那一行的输入由装配方以 `Json` 值投射、
**不由任何算子的 `output_schema` 提供**，故该行不在 (j) 的断言范围内——判据要求「上游的
`output_schema` 含该型」，而这一行没有上游算子可指。这不是判据的漏洞：规范里那条边
（《总纲》§8.3 `:1448` 的 `Milestone` → 规划）的上游是 §234 的 `Milestone`（P4 的对象），
本块的算子集里没有它（见下段）。

**本表不是全图**：`code-plan` 的上游（《总纲》§8.3 `:1448` 的那个 `Milestone`，以及 §228 的 `Plan`）
不在本块的算子集里——`Milestone` 是 §234 的对象（P4），`Requirement / Design Review` 是 §229 的状态机（P4）。
两处都**以值**由装配方投射进 `code-plan` 的 `[Json]` 输入。

**与 P1 的分工**：`compatible`（`crates/continuum-port/src/port.rs:94`）判**端口连接**的兼容性，属 P1；
`is_candidate_backend`（`crates/continuum-graph/src/execution.rs:87`）判**后端是否在候选内**，也属 P1。
**本块只保证自己声明的 `input_schema` / `output_schema` / `backend_candidates` 取值正确**，
不写第二份这类的判定函数（切分 §四第 7 条）。

## 3.5 一处结构缺口：`determinism` 挂在 Operator 上，backend 由 Router 选

**事实**：`Operator.determinism` 是**算子级**字段（`crates/continuum-operator/src/definition.rs:84`），
而 §245 `:756` 逐字「同一个 Operator MAY 有多个 backend」，具体后端由 Router 决定（§245 `:773` 逐字
「具体 backend 由 Router 决定」）。复用判定 `can_reuse`（`crates/continuum-graph/src/reuse.rs:29`）
读的是 **`operator.determinism`**，不看 backend。故一个声明 `Deterministic` 的算子，若它的候选集合里
混进一个输出不逐位可复现的 backend，**复用就会给出一个与重算不同的结果，而这一点在类型层不可见**。

**本块的处置（注册期判据，两侧都钉）**：

- 本块持有一份**「逐位可复现的 backend」名单**（封闭表，内容是本设计定的）：
  `worktree` `overlay` `builtin` `cargo-test` `local-test-runner`。
- **注册期规则**：传给 `register_code_operators` 的每一枚 `determinism == Deterministic` 的算子，
  其 `backend_candidates` **逐个**必须在该名单内；否则该函数返回
  `CodeError::UnverifiableDeterminism { operator, backend }`（§3.2 的两臂之一），
  且**在写注册表之前**返回，不留下半注册。
- **偏离最显然写法处**：最显然的写法是给 `code-test-run` 的候选里放一个「远程测试集群」或
  「并行加速的测试分片器」（更快）。**这里不是那样：因为远程集群的镜像、工具链版本与并发顺序
  都不由本仓固定，同一棵树两次运行可能给出不同的失败集合**，混入即让该算子的 `Deterministic` 声明为假，
  而那条声明的消费者是 §305 的复用。代价是远程执行不可用——**这是性能代价，不是正确性代价**
  （同 P5e 设计 §5.4 对硬件编码器的处置形状）。

**名单里为什么有 `overlay`**：§255 `:1029-1037` 逐行列了非 Git 项目的五种形态，而 P2 已落地的
`WorkspaceBackend`（`crates/continuum-workspace/src/backend.rs:22`）只有两臂。**本块取那两臂**
（§4.2 的照片钉的就是这条），不按 §255 的五种各造一个 backend 名——**那会让 `backend_candidates`
里出现三个没有实现的取值**，而 §245 的候选是「Router 可以选的东西」，不是「规范提过的名字」。

**候选名取的是 P2 的编码，不是 §255 的散文形**：§255 `:1026` 写的是 `git worktree`（两个词），
`:1032` 写的是 `OverlayFS`，而 P2 已落地的编码是 `"worktree"` 与 `"overlay"`
（`crates/continuum-workspace/src/persist.rs:177-182` 的 `backend_str`；其注记 `:166-176` 明写这两个编码
**显式给出，不由 `WorkspaceBackend` 的 serde 表示或 `Debug` 推出**）。`backend_candidates` 的元素是
Router 拿去与 P2 的后端名比对的东西，故**以代码里的编码为准**：`git-worktree` 与 `overlayfs` 是
§255 的散文名，逐名不等于那两个编码，本块不采用（§10.2 第 (p) 条的照片据此拍）。

**这条规则的规范来源：没有。** §245 只给「一个 Operator 多 backend」，不给 backend 的确定性。
故名单本身是本设计定的，并**据实记为缺口**（§11 第 7 条）：真正的判据是「该 backend 的什么属性使输出
逐位可复现」（版本冻结？种子？浮点确定化？），规范未给。

**这条规则拦不住哪一类（要写清，否则会被当成一条全覆盖的守卫）**：它只查
「声明 `Deterministic` 的算子，其候选是否逐位可复现」，**不查「这个算子的真实输入是否在端口上」**。
`code-workspace` 正是后者：它的两个候选（`worktree` / `overlay`）**都在名单内**，故把它声明成
`Deterministic` **能通过注册期规则**，而 §3.3 第 3 条判它错（真实输入是 Base，不在端口上）。
**故 §3.5 的规则与 §3.3 第 3 条是两条独立的守卫，缺任一条都留下一类静默错误**：
前者抓「候选不可复现」，后者抓「输入不可见」。后者的照片是 §10.2 第 (o) 条那一枚断言——
**它是一条测试，不是一个注册期检查**（注册期无从知道一个算子的输入里有什么没上端口）。

## 3.6 §305 的复用：**不建第二份判据**

§305（`docs/spec/05-normative.md:1986-1998`）给的是三条件形式（`input_hash` 未变、`operator_version`
未变、`contract` 未受影响 ⇒ `Runtime MAY 直接复用 Artifact`），**它已由 P1 落地**：
`crates/continuum-graph/src/reuse.rs` 的 `CacheKey`（`:9`）、`cache_key`（`:16`）与 `can_reuse`（`:29`），
而 `cache_key` 对 `NonDeterministic` 的算子返回 `None`（`:22`）。

**故本块不得再定义第二个复用判据**（切分 §四第 7 条，第一枚实例即此）。本块的兑现
**不是一个函数**，而是 §3.2 表里那一列 `determinism` 声明——它是 §305 三条件中
`input_hash` 那一半的**前提**（P1 的内部注释把这条前提写成了「非确定节点的输出不写缓存键，
因此永远不满足复用前提」）。照片见 §10.2 第 (o) 条。

**本块的 9 枚里，进复用的只有 2 枚**（`code-workspace` 声明 `NonDeterministic` 故不进，§3.3 第 3 条；
`code-plan` / `code-decompose` / `code-implement` / `code-spec-review` / `code-quality-review` /
`code-fuzz-run` 六枚同样不进），即 **`code-test-run` 与 `code-verify` 两枚**。
`code-workspace` 虽然 `Deterministic` 的诱惑最大（工作区创建开销大），但它不进——理由见 §3.3 第 3 条。

---

# 4. L2 隔离工作区在这条链里的位置与端口

## 4.1 §16 与 §255/§256/§257 说的是什么

- §16（`docs/spec/01-concepts.md:788` 起）：`:790` 逐字「对于代码任务，L2 默认使用：」，`:793` 是
  `git worktree`；`:796` 逐字「每个 Intent 创建独立：」，`:799` 是 `ai/task-id`；`:812-816` 逐行列
  Agent 可以做的事（修改 / commit / reset / 测试 / 重构）；`:819` 逐字「但完全不碰用户当前 branch。」；
  `:824-826` 逐字「L2 = 可以自由修改自己的隔离世界」；`:828-835` 说明真正跨越（AI worktree → user branch）
  才属于 L3。
- §255（`docs/spec/05-normative.md:1021` 起）：`:1023` 逐字「Git 项目 SHOULD 使用：」，`:1026` 是
  `git worktree`；`:1029-1037` 列非 Git 项目的五种形态。
- §256（`:1041` 起）：`:1046` 逐字 `AI_WRITE(BaseWorkspace) = DENY`。
- §257（`:1053` 起）：`:1055-1059` 逐字「TaskWorkspace 的修改进入 BaseWorkspace 前 MUST 经过
  Integration Gate」；`:1063-1069` 列五种操作。

**已落地的 P2 面**（本块只读、只命名，不改）：`crates/continuum-workspace` 的
`WorkspaceBackend`（`crates/continuum-workspace/src/backend.rs:22`）、`detect_backend`（`:73`）、
`create_task_workspace`（`:88`）、`discard_task_workspace`（`:120`），以及
`TaskWorkspace` / `WritablePath`（`crates/continuum-workspace/src/lib.rs:32` 的 `pub use`）与
`IntegrationGate` / `GateApproval`（同文件 `:23` 的 `pub use`）。

## 4.2 位置：它在这条链上**是一枚算子，也是一个前提**——两半必须分开读

**裁定的内容**：本块把 L2 工作区的建立落成**第 1 枚算子 `code-workspace`**（§3.2 第 1 行），
而 §257 的 Integration Gate **不落成算子**。

**这一半是算子（`code-workspace`）**，理由两条：

1. **它是代码任务的链上第一步，而《总纲》`:1443` 把它写在了链的前面**（逐字「代码任务的基础是
   §5.1 的隔离工作区」）。把它做成算子，使「这条链有一个具名的起点、且它的产物是 `SourceTree`」
   成为可登记的事实；不给它算子，它就只能是一条注释（§3.3 第 1 条）。
2. **它有两个 backend，而 §255 的两种形态在这一层是同一个算子的两个候选**（§245 的机制）。
   这条同时钉住了 `code-workspace` 的两个候选名与 P2 已落地的 `WorkspaceBackend` 两臂的对应
   （§4.1 末），照片见 §10.2 第 (p) 条。

**这一半不是算子（Integration Gate）**，理由两条：

1. **§257 `:1055-1059` 的 MUST 管的是「修改进入 BaseWorkspace 之前」**，而那是**任务完成之后、
   向用户工作区集成时**的事，不是链上的一步。它在这条链**之外**（§16 `:828-835` 逐字把「真正跨越
   AI worktree → user branch」判给 L3）。
2. **P2 已把它落地为一个类型 + 一个铸值点**（`IntegrationGate` / `approve_integration`，
   `crates/continuum-workspace/src/lib.rs:23` 的 `pub use`），而它**收 `&GateApproval`**——
   即它需要一次用户批准。把它做成算子会要求 `OperatorImpl::execute` 拿得到批准值，而那条签名今天
   只有 `(inputs: &[ArtifactRef], ctx: &NodeContext)`（`crates/continuum-graph/src/execution.rs:77-81`），
   没有装批准值的位置。**故本块不把它算子化**，这一条与 §4.4 的第二处缺口同源。

**据实记一处残余**：`code-workspace` **每 Intent 一枚**（§16 `:796` 逐字「每个 Intent 创建独立」），
而算子是**每节点**的执行单元（§244）。两者不冲突的前提是**装配方只为一个 Intent 建一个该算子的节点**——
而「图上有几个该节点的实例」是 Planner 的构造，不是算子声明的性质。**本块无法在类型层钉住它**，
记为 §11 第 4 条。

## 4.3 端口：Base 不在端口上——一处结构缺口

**事实**（逐条实测）：`Operator` 的输入面是 `input_schema: Vec<ArtifactType>`（`crates/continuum-operator/src/definition.rs:82`），
`ArtifactType` 的六型是 `SourceTree` / `Patch` / `TestResult` / `Text` / `Json` / `Blob`
（`crates/continuum-artifact/src/artifact.rs:15-20`）。**Base 工作区不是这六型里的任何一型**，
也不该是（§3.3 第 2 条）。故 `code-workspace` 的 `input_schema` 是空 `Vec`。

**缺口的内容**：`code-workspace` 的**真实输入**（Base 的当前工作树与它的形态）**在端口上不可见**，
而在端口上不可见的东西，§303 的 READY 判据（所有输入端口已有可用的 DATA 入边来源）与 §239 的连接
判定**都看不到它**。这意味着「Base 变了，工作区该重建」这件事**不构成图上的一条失效传播**：
没有边可传（§238 的边连的是端口）。

**本设计不就地补它**，理由：补它要动的是 §244 的 `input_schema` 的**取值域**（让非制品的东西能当端口）
或 §236 的 `Node` 的字段面，两者都在第 3 层 / P1 的改动范围里（切分 §四第 4 条：P5 只**注册**算子）。
**缺的是「Base 的形态如何进入判定」这一步。收件人：P1（`Node` / `Operator` 的字段面）＋ 第 3 层的执行器**
（§11 第 3 条）。本设计给出的两侧代价：本轮该缺口的表现是 `code-workspace` 声明 `NonDeterministic`
（§3.3 第 3 条），即**代价落在「不复用」上，不落在「复用了错的」上**——fail-closed 的那一侧。

## 4.4 第二处结构缺口：`NodeContext` 没有工作区句柄

**事实**（逐条实测）：算子执行接口是 `OperatorImpl::execute(&self, inputs: &[ArtifactRef], ctx: &NodeContext)`
（`crates/continuum-graph/src/execution.rs:77-81`）；`ArtifactRef` 只有 `id` 与 `content_hash` 两个字段
（同文件 `:36-39`）；`NodeContext` 的字段是 `node` / `profile` / `cancelled` 三项
（同文件 `:43-47`；四个方法在 `:50`、`:58`、`:62`、`:70`）。**三者里没有一处装得下一个 `TaskWorkspace`。**

**后果**：代码域的 7 枚链上算子（第 2–9 行）里，第 4、5、6、7、8 行**都要在 Task Workspace 里干活**
（改文件、跑测试、跑 fuzz），而它们**没有任何一条渠道拿得到那个工作区的句柄**。§256 `:1046` 的
`AI_WRITE(BaseWorkspace) = DENY` 是由 P2 的 `WritablePath` 与 `continuum-sandbox` 的 `Sandbox::spawn`
共同强制的（实测其签名的第一个实参是 `task: &TaskWorkspace`，`crates/continuum-sandbox/src/sandbox.rs:182`），
而那条通道的**入口**正是在这里缺席的位置。

**本设计不就地补它**，理由与上一条同：补它要动 `NodeContext` 的字段面，那在 P1 / 第 3 层的改动范围里。
**缺的是「工作区句柄怎样到达算子实现体」这一步。收件人：P1（`NodeContext` 的字段面）＋ 第 3 层的执行器**
（§11 第 3 条）。**这一条与 §4.3 是同一处的两半**：§4.3 是「输入进不来」，本条是「前提进不去」；
两者都指向同一次改动（执行上下文与输入端口的取值域），故合并成 §11 的一条，不各记一次。

---

# 5. 每步的 `Evidence` 产出面

## 5.1 「每步产出 Evidence」这一句要读出的两件事

切分 §五 P5c 行的原话是「并让每步产出 P5a 的 Evidence」。本设计按两件事兑现它，两件都要读：

1. **产出面**（P5a 设计 §4.4）：凡本块的算子产出的是**对制品的判定**，就产 `Evidence`；
   凡它产出的是**制品本身**，就不产（§5.2）。
2. **必须经唯一产生点**：任何 `Evidence` 都只能经 `Evidence::from_tool_result`（P5a 设计 §4.4）造出，
   本块**不建第二枚包装**（§6.2）。

## 5.2 逐枚的产出面

| 算子 | 是否产 `Evidence` | `EvidenceType` | `EvidenceSubject` | 产生点 |
|---|---|---|---|---|
| `code-workspace` | 否 | —— | —— | —— |
| `code-plan` | 否 | —— | —— | —— |
| `code-decompose` | 否 | —— | —— | —— |
| `code-implement` | 否 | —— | —— | —— |
| `code-spec-review` | **是** | `ModelReview`（本块 §5.3 报出，P5a 已收，其设计 §4.2） | `Unattached` | `Evidence::from_tool_result` |
| `code-quality-review` | **是** | 同上 | `Unattached` | 同上 |
| `code-test-run` | **是** | `Test`（§258 第 1 臂） | `Unattached` | 同上 |
| `code-fuzz-run` | **是** | `Fuzz` / `Property`（§258 第 2、3 臂） | `Unattached` | 同上 |
| `code-verify` | 否（产判定，不产证据） | —— | —— | —— |

**前四枚为什么不产证据（这一条要写清，否则会被读成漏项）**：§258（`docs/spec/05-normative.md:1073-1088`）
的八个字段里，`type` 是「主张**是怎样被确立的**」。`code-workspace` / `code-plan` / `code-decompose` /
`code-implement` 四枚产出的是**制品**（`SourceTree` / `Json` / `Patch`），而**制品的存在**由 §266 的第 1 项
`ArtifactExists` 判（P5a 设计 §8.3 第 1 项，判据是「受验图的终端节点各自至少有一个产出制品」），
**不是一条 `Evidence`**。规划与拆任务的**审查**是 §229 的 Plan Review（P4），也不在本块。
**这里不是「这四步不产证据所以 §89 不适用」**：§89（`docs/spec/02-positioning.md:923`）逐字
「工具调用成功不代表语义成功……所以工具结果必须转换成 Evidence」——本块对 §89 的兑现是
**链尾四枚**（第 5–8 行）：它们的工具结果是「对制品的检查结果」，那才是「可能成功而语义不成功」的东西；
而「补丁被写出来了」这件事没有语义成败可言，它的产物就是制品本身。

**`code-verify` 为什么不产证据**：它是 §262 第 1 级的判定器，它的产出是**判定**（一个 `Json` 记录），
而 §258 的 `Evidence` 是判定的**输入**形态（§263 `:1196-1202` 逐字把 Task Contract / Artifact / Evidence
三者列为初次验证的输入）。**若让 `code-verify` 也产一条证据，判定的输入就包含了上一轮的判定**——
那正是 §120 所说的「共享证据路径」的一种形式，P5a 设计 §7.4 对 `TestValidity` 用的正是这条理由
（它明确写了「它只进 `coverage()`，不进 verifier 的输入」）。

**`subject` 一律取 `Unattached`**：理由借 P5a 设计 §4.3.1 自己的那条（「证据先于归属存在」），
附带的一个后果是**本块不需要 `RequirementId`**（属 P4 的 `continuum-semantics`，该 crate 今天不存在），
故 §2.1 的边表里没有它。

**`producer` 取 `EvidenceProducer::Node { node, backend }`**（P5a 设计 §4.3.3 的两臂之一），
由装配方以**值**给。**本块拦不住 `EvidenceProducer::Human`**——两臂的搭配由 P5a 的构造点判，
故「本块的产出都是 `Node`」这一句是**声明级**的，不是类型级的（同 P5e 设计 §9.2 的据实记录）。

## 5.3 报出：两枚审查算子在 §258 与 §193 里**无臂可落**

**事实（逐行实测）**：§258 `:1091` 逐字「Evidence 类型例如：」，其后 `:1094-1102` 是九臂
（`TEST` / `FUZZ` / `PROPERTY` / `FORMAL_PROOF` / `STATIC_ANALYSIS` / `BENCHMARK` / `VISUAL_CHECK` /
`METADATA_CHECK` / `HUMAN_CONFIRMATION`）；§193 `:368-373` 另列六种验证方式，P5a 设计 §4.2 把其中
`Differential` 与 `Metamorphic` 两臂补进了枚举，共十一臂。**「共十一臂」是本块报出时的数目**：
2026-10-10 本块的报出被 P5a 收下（`ModelReview` 落地），枚举增至十四臂（P5a 设计 §4.2）。

**报出时的这十一臂里没有一枚表达「由一个独立模型对制品作出的审查判定」**。而 §262 `:1176` 逐字把
`independent model verifier` 列为第 3 级——**规范承认这种判定存在，却没给它的证据类型**。
`code-spec-review` 与 `code-quality-review` 两枚产出**正是**这种东西。

**本块的处置（三条，逐条给理由）**：

1. **不自行加臂**（切分 §四第 1 条：证据类型是 P5a 的，四块不得自加）。
2. **不套用一枚语义不符的臂**。最接近的是 `StaticAnalysis`（§258 第 5 臂），
   **这里不是那样：因为 `STATIC_ANALYSIS` 指的是静态分析工具对源码作出的判定，
   而 `code-spec-review` 是模型对「补丁与规格是否一致」作出的判断——两者的生产者与失败模式都不同**
   （工具判定可穷尽重放，模型判定不可）。用一枚语义不符的臂，失配方向是**静默的**：
   一条 `StaticAnalysis` 的证据会被按「确定性检查」处置（§262 的级别序里它比模型判定优先），
   于是一次模型判断拿到了确定性检查的优先级。
3. **报出，并给出候选名**：本块提议 P5a 加一枚臂 `ModelReview`
   （与 P5a 设计 §4.2 末段已开的那个口子同形——那里写着 P5d 与 P5f 若需要各臂须向 P5a 报出）。
   **2026-10-10 该臂已被 P5a 收下并落地（其设计 §4.2）**，故这两枚算子的证据类型不再是待定：见 §5.2 表。
   在臂到位之前，本块这两枚算子的证据类型是待定的，而「待定」这一状态**不落在本块的代码里**
   （本块不产这两条证据的代码，见 §6.2 末）。

**§193 的 `Differential` / `Metamorphic` 两臂：本块不请求，也不给它们算子**——P5a 设计 §14 第 2 条
问本块「若无对应实现须说明」，答复如下：§8.3 `:1445` 逐字是「执行方法的**默认**形态」，
而差分与变形测试**不在**那六个步骤上；它们也不是 §187 的 `software/` 目录里的名字
（`:118-124` 逐行是 `planning` `TDD` `debugging` `worktree` `review` `fuzz` `verification`）。
**故这两臂本轮在本域无生产者**。本块**不主张删它们**（那是 P5a 的枚举），只据实报出这一点；
若将来某枚代码域算子产出该型证据，它按 §258 的第 3、4 臂走即可。

## 5.4 「测试是否通过」的承载（答复 P5a 设计 §14 第 2 条与 §15 第 14 条）

P5a 设计 §14 第 2 条问本块：「本设计假设『测试是否通过』由**制品**承载，故需要一个 `TestResult` 制品的
payload 约定。**若 P5c 把结果放在别处（或需要一枚新的 `EvidenceType` 臂），须向本块报出**」；
§15 第 14 条问的是「**谁写、写在哪一列**」。本设计逐条答：

1. **承载处：`TestResult` 这一型制品**，与 P5a 的假设一致，**不改**。
2. **谁写**：`code-test-run`（§3.2 第 7 行）与 `code-fuzz-run`（第 8 行）两枚算子的产出。
   `code-test-run` 是确定性的那次（`Deterministic`），`code-fuzz-run` 是不确定的那次。
3. **写在哪一列**：落进 `continuum-artifact` 的既有表——`crates/continuum-artifact/src/persist.rs:9`
   的迁移里 `artifact` 表的既有列（`artifact_type`、`content_hash`、`metadata`、`provenance` 等），
   **不新增列、不新增表**。`artifact_type` 取 `test_result`（`crates/continuum-artifact/src/artifact.rs:47`
   的 `as_str`，`TestResult` 那一臂在 `:51`）。
4. **payload 约定：本设计不定义**（这一条是**据实留的缺口**，不是漏项）。
   理由是 §258 的字段里没有 outcome、且 `Artifact.metadata` 的类型是 `serde_json::Value`
   （`crates/continuum-artifact/src/artifact.rs:177`）而其 schema 规范未给——**这正是 P5a 设计 §15 第 9 条
   已记的那处缺口**（该条把收件人写成「P5c（`TestResult` 载荷）＋ 规范维护者」）。
   **本设计接下这一条并说清缺的是哪一步**：需要的是「一个 `TestResult` 的载荷里，哪些字段是
   判『通过/不通过』所必需的」这一步的裁定（通过/失败计数？逐用例结果？退出码？）。
   **本块不发明它**：发明一份载荷 schema 就是替规范定了一个版本化的外部接口，
   而 §258 与 §240 都没给（同 P5e 设计 §8 对 Timeline 六字段的处置形状）。
   **收件人：规范维护者 ＋ P5a**（§11 第 8 条）。**互指**：本条与 P5a 设计 §15 第 9、14 两条是同一处的
   三个侧面，三处都留，互为指针。

---

# 6. 与 P5a 的接缝：只产出、只消费

## 6.1 消费面：本块不调用 P5a 的任何判定

`VerificationPolicy`（节点级）、`coverage`（P5a 设计 §6）、`judge_completion`（其 §8）
**都不在本块的调用集合里**。本块的消费止于「`code-verify` 的那一条级别声明」（§3.3 第 11 条）——
那是一条**声明**，不是一次判定调用。
**据实记录**：本块与 P5a 之间的实际数据流是**单向**的（本块产出 Evidence、声明 verifier 级别），
反向的那一半（P5a 判定 → 本块）今天没有承载物，因为执行器未建（§10.5 第 1 条）。

## 6.2 产出面：**本块不建第二枚包装**

P5e 已定义一枚包装 `evidence_from_media_output`（P5e 设计 §9.2），其签名与 P5a 的
`Evidence::from_tool_result` 逐项对齐，**唯一固定的项是 `subject = EvidenceSubject::Unattached`**。
本块的产出面与它**逐项相同**（§5.2：`subject` 一律 `Unattached`，七个参数由调用方给）。

**故本块不定义 `evidence_from_code_output`**。理由两条：

1. **切分 §四第 7 条的同一条判据适用于包装**：两枚函数做的是同一件事（把七项拼给 P5a 的唯一构造点，
   固定 `subject`），而两枚都正确时，「同一件事两个词汇表」在仓里就有了两个入口。
   该条的失效方向在本处同样是**静默的**：若将来四块里某一枚包装的固定项变了（例如某块开始构造
   `Requirement(..)` 归属），另一块不会跟着变，而两块的调用点都不会红。
2. **包装里没有任何一件是本域特有的**。P5e 的包装若真固定了本域的东西（例如固定的 `EvidenceType`），
   它才有存在的理由；而它固定的 `subject` 是**四个领域算子块共有**的取值。

**本块的产出路径**：由装配方／本块的具名入口直接调 `Evidence::from_tool_result`（`pub`，
P5a 设计 §4.4），`subject` 传 `Unattached`。**`TestValidity` 与 `VerdictAggregate` 都不在本块产出的东西里**。

**据实记一处残余**：本块因而**不产 §5.3 那两条待定臂的证据的任何代码**——`code-spec-review` /
`code-quality-review` 的 `EvidenceType` 由装配方在调用时给出，本块不把它写死。这与
「本块不建第二枚包装」是同一条取向的后果：**本块不替 P5a 决定证据类型**。

---

# 7. 与 P5b 的接缝：`bind`

## 7.1 七条 `software/` 方法的 `realized_by`

P5b 设计第六节要求四块**只经 `bind` 填 `realized_by`**，且 `software/` 的七条逐条非空
（P5b 设计 §4.5：`has_counterpart(Software) == true` ⇒ 空即「漏 bind」）。本块填出：

| `MethodId`（§187，`docs/spec/04-method.md:118-124`） | `realized_by`（本块的算子 id） | 理由 |
|---|---|---|
| `planning` | `["code-plan", "code-decompose"]` | §186 `:62` 的 `Implementation Plan` 与 `:64` 的 `Engineering Tasks` 两步都是规划 |
| `TDD` | **`[]`** | **次序约束**，见 §7.2 |
| `debugging` | **`[]`** | **链上无此步**，见 §7.2 |
| `worktree` | `["code-workspace"]` | §16 `:790-794` 的 git worktree；算子见 §4.2 |
| `review` | `["code-spec-review", "code-quality-review"]` | §186 `:85` 逐字「规格审查与代码质量审查分离」 |
| `fuzz` | `["code-fuzz-run"]` | §193 `:368-369` 的 Fuzzing 与 Property-based Testing |
| `verification` | `["code-test-run", "code-verify"]` | §186 `:74` 的 Verification；两枚的分工见 §3.3 第 5、11 条 |

调用 `bind` 的**具名入口**：

```
/// 把上表七条 `realized_by` 填进方法库（P5b 设计第六节：每块对其领域目录里的**每一条已 seed 的 id**
/// 调用一次 `bind(id, &[...])`）。其中 `TDD` 与 `debugging` 两条按 §7.2 填**空**。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
pub fn bind_code_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;
```

**错误类型不并进 `CodeError`**：`MethodError::NotFound`（P5b 设计 §4.4）的判定是「方法库里有没有
这条方法」，与 `CodeError` 的两臂（§3.2）不同源，故直接透出。这与 §3.2 把 `OperatorError::Duplicate`
包进 `CodeError::Registry` 是**同一条判据的两面**：判定是同一件事则合并，是两件事则分开。

**本函数不做二次登记**：不 `register` 方法条目（P5b 设计第六节：四块**不得**自造 `MethodEntry`），
七条 id 由 P5b 的 `seeded()` 给出。

**`bind` 的参数是文本**，故上表的值是字符串，**本块无法在编译期核对它们与 `all_operators()` 的 id 一致**
（P5b 设计 §3.2 末已把这条弱引用的无照片写清）。本块的补件是**运行期的一条用例**（§10.2 第 (l) 条）：
对每一条非空 `realized_by` 里的每个串，`OperatorRegistry::resolve` 必须成功——
这条用例**在本块的 crate 里闭得上**，因为本块同时持有 `all_operators()` 与 `MethodRegistry`。
**这是 P5b 所说「消费块才能核」的那个消费块之一**。

## 7.2 两条**无对家**的 `software/` 条目：`TDD` 与 `debugging`

**这一节是本设计在 P5b 冻结接口上发现的一处结构性不匹配，据实报出，不就地补。**

**事实（逐行实测）**：§187 `:118-124` 逐行列 `software/` 的七条方法名，而 §186 `:57-77` 的链上
只有 `planning`（`:62` 的 `Implementation Plan`）与 `verification`（`:74`）两处有对应的步骤；`review` 与 `fuzz` 各有对家
（§186 `:70`/`:72`、§193 `:368-369`）；`worktree` 由 §16 给（§4.2）；**剩下两条没有**：

1. **`TDD` 的全部内容是「次序」**——先写测试、后写实现。
   `MethodEntry.realized_by` 的类型是 `Vec<String>`（P5b 设计 §4.3），即**算子 id 的一个集合**，
   **它表达不了次序**：即便把 `["code-implement", "code-test-run"]` 填进去，那个列表也不说明
   哪一枚在前。**这里不是「填两枚 id 就算兑现」：那会让 `TDD` 与「先实现后补测试」在登记面上
   不可区分**，而两者的差别正是这条方法的全部内容。
   **§186 `:82-85` 的四条优势里没有 TDD**（逐字是「计划先于实现」「任务上下文隔离」「逐任务验证」
   「规格审查与代码质量审查分离」），故本块也没有第二处出处可据。
   **§188（`docs/spec/04-method.md:159-206`）与本节同一取向**：它逐字写「TDD = Implementation Discipline」
   （`:166-167`），给出的内容正是那一条序（`先写测试 → 测试失败 → 实现 → 测试通过`，`:180-186`），
   并明写它不构成正确性证明（`:161`；`:203-205` 逐字「Tests Passed ≠ Requirement Satisfied」）——
   即 §188 把 TDD 定位为**纪律（次序）**，同样不给它一个算子对家。
2. **`debugging` 在链上没有对应的步骤**。《总纲》`:1448` 的六个步骤里没有调试，
   §186 `:57-77` 的十行里也没有调试。**本块不发明一枚 `code-debug`**：
   发明它就要给它一个端口类型，而规范没有一处给过「调试的输入与产物是什么」——
   这与切分 §八 的「`3d/` 在四个具名算子里没有对家」是同一种处境（有目录、无对家），
   处置也照那一条：**记缺口，不硬塞**。

**故本块把这两条的 `realized_by` 填成空 `Vec`，并报出**（§11 第 2 条）。**后果据实写全**：
P5b 设计第十节的判据 6 在 `Software` 域上会**红**（该判据要求 `has_counterpart == true` 的域逐条非空；分开「刻意留空」与「漏 bind」的那个谓词在 P5b 设计 §4.5），
而它红的原因是**上表那条结构性不匹配，不是漏 bind**。**这条判据的失效方向在此处正好相反**：
它本来是用来抓「漏 bind」的，而在本域它抓到的是一件 P5b 的接口层看不出来的事——
两条方法名不是算子。**这正是该判据该有的行为**：它没有静默放过。

**三条处置建议，逐条给代价，本块不选**（选它是改 P5b 的冻结接口，不在本块范围内）：

| 处置 | 代价 |
|---|---|
| 给 `MethodEntry.realized_by` 加一条**次序**维度（如按序解释的 `Vec<String>`） | 改的是 P5b 的冻结类型；而「按序解释」对另外六条**没有意义**（跨块的语义分歧） |
| 给 `MethodEntry` 加一条**非算子**的承载（如指向 P2 的 `create_task_workspace` 的函数名） | 同上；且 P5b 设计 §3.2 已明写它**不解析** `realized_by`，加一个不解析的字段等于加一个死字段（P5b 已为此删过 `purpose`） |
| 认这两条**无算子对家**，并把「非空」的判据收窄为「除 `TDD` 与 `debugging` 外非空」 | 改的是 P5b 的判据 6，不在本块 |

**收件人：P5b 的持有方 ＋ 协调者**（§11 第 2 条）。**若协调者裁定「应给它们算子」，改动是本块加两枚
算子并各给端口 —— 而端口类型是规范未给之物，须同时给出来源**（那是那条裁定的实质内容）。

## 7.3 §186 的流程名不入目录（答复 P5b 设计 R6）

P5b 设计 §5.1 末的 R6 把「§186 的流程名（如 `spec-review`、`quality-review`）是否入 `software/` 目录」
交给本块。**本块的答复：不入。** 理由：§187 `:118-124` 的七条是方法名，§186 `:57-77` 的是流程步骤，
两者不是同一张表（P5b 已对 `video/` 与 §33 做过同样的判断）。本块把 §186 的步骤名落成**算子的 id**
（§3.2），**不把它们 `register` 进方法目录**——P5b 设计第六节禁止四块自造 `MethodEntry`。

---

# 8. `ArtifactType` 够不够：**本链零请求**

切分 §四第 2 条要求本块把需要的变体报给 P5e，由 P5e 在**一次改动**里落地。**本块的答复是「无」。**

**判据（逐项实测）**：§3.2 表里出现过的 `ArtifactType` 取值只有四枚——
`SourceTree`（第 1、7、8 行）、`Patch`（第 4–9 行）、`TestResult`（第 7、8 行）、`Json`（第 2–6、9 行）。
四枚**全在** `crates/continuum-artifact/src/artifact.rs:15-20` 的六型之内（其余两型是 `Text` 与 `Blob`，
本链不用）。

**为什么 `Json` 够用，而不是为「计划」「任务单」「审查报告」「判定记录」各立一型**：
它们都是**结构化文档**，而 `Json` 这一型正是「结构化但无专门类型的产物」的落点
（P1 设计 `:156-163` 的枚举注记把 `Blob` 写成「尚无专门类型的产物」，`Json` 与它同形）。
**这里不是「顺手用 `Json` 省事」：另立一型要付的是枚举边界上的永久代价**
（`as_str` / `parse` 是那枚编码的唯一产生点，删一型同样是破坏性变更），
而本链没有一处需要「按型区分」——§3.4 的边表里四枚 `Json` 端口的判定**全部靠相容性**，
不靠区分（`compatible` 按 `ArtifactType` 等值判，`crates/continuum-port/src/port.rs:94`）。

**一处要单独说明**：`§228` 的 `Plan`（`docs/spec/05-normative.md:334` 起）是 **P4 的一个对象**，
不是 `ArtifactType` 的变体。本块的 `code-plan` 产出的是**实现计划这一份制品**（§186 `:62`），
故落成 `Json`。**两者同名不同物**，这一条据实记在 §11 第 5 条（同一份文档里两个「Plan」的风险）。

**P5e 设计 §3.6 的排期不受本块影响**：§3.6 要求 P5e 的枚举实现任务排在 P5c/P5d/P5f 的「报出」之后；
**本块的报出是「无」**，故该排期对 P5c 这一侧没有待合并项。

---

# 9. 持久化：**本块零迁移**，档号 **150**（实测）

## 9.1 结论：本块不建表、不建迁移、不写事件

逐项给理由：

| 本块的产物 | 为什么不落库 |
|---|---|
| 9 枚 `Operator` | `Operator` 今天**没有持久化路径**：`continuum-operator` 无 `persist.rs`（实测该 crate 的 `src/` 只有 `definition.rs` `registry.rs` `lib.rs`），注册是**装配期在内存里**做的事（`OperatorRegistry` 的 `entries` 是 `HashMap`，`crates/continuum-operator/src/registry.rs:15-16`）。本块照此，不新增存储 |
| 七条 `realized_by` | 同上：`MethodRegistry` 是内存结构（P5b 设计 §4.5） |
| 链上产出的四型制品 | 落进 `continuum-artifact` 的**既有表与既有列**（§5.4 第 3 条），迁移号 `10` 已在（`crates/continuum-artifact/src/persist.rs:9`） |
| Evidence | 属 P5a 的表（其设计 §10.2），本块只经它的构造点产出对象 |
| L2 工作区的元数据 | 已由 P2 落库（`crates/continuum-workspace/src/persist.rs:21-22` 的迁移 `30`），本块只经 `code-workspace` **命名**它（§4.2） |
| §305 的复用凭据 | `CacheKey`（`crates/continuum-graph/src/reuse.rs:9`）今天**也没有存储**（P1 设计 §18 把 `can_reuse` 列在「无执行点的机制」里）。**本块不替它建表**（§3.6） |

## 9.2 实测占用表（判据在切分 §二第 5 条）

**实测命令**：`grep -rn -A1 "Migration::new(" crates/`，逐处读**调用的第一个实参**（号在参数表的次行或同行）。
实测结果（工作树 `p5c`，2026-10-09），出现的号（含次数）：

| 号 | 次数 | 位置 | 是否进运行时装配链 |
|---|---|---|---|
| `1` `2` | 各 1 | `crates/continuum-persist/src/db.rs`（`builtin_migrations()`） | 是 |
| `10` | 1 | `crates/continuum-artifact/src/persist.rs` | 是 |
| `20` | 1 | `crates/continuum-graph/src/persist.rs` | 是 |
| `30` | 1 | `crates/continuum-workspace/src/persist.rs` | 是 |
| `40` | 1 | `crates/continuum-effect/src/persist.rs` | 是 |
| `41` | 1 | `crates/continuum-policy/src/persist.rs` | 是 |
| `50` | 2 | `crates/continuum-capability/src/persist.rs`；`crates/continuum-persist/tests/recovery.rs` | 一是一否（后者是探针库） |
| `60` | 2 | `crates/continuum-persist/src/bin/crash-writer.rs`；`crates/continuum-persist/tests/crash_atomicity.rs` | 均否（各自自建的库） |
| `80` | 1 | `crates/continuum-model-registry/src/persist.rs` | 是 |
| `100` `101` `102` | 各 1 | `crates/continuum-persist/tests/migrations.rs` | 均否（测试夹具） |

**实测结论**：`130`–`199` 这一段在**全仓**（含测试夹具）**零命中**。
「是」的那些即 `crates/continuum-runtime/src/main.rs:98-107` 的 `runtime_migrations()` 的装配集合。

**协调者已裁定的号段**（P3 那一轮）：A=`50`、B=`60`、C=`70`、D=`80`、E=`90`
（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md` 的「一个子项目一个十位档」）。
**P5a 已取 `130`**（切分 §二第 5 条）。**P5b 与 P5e 零迁移**：本块实测——P5b 的设计全文无持久化一节
（`grep -n "迁移\|Migration\|号段"` 在该文件只有两处无关命中），P5e 的设计 §11.3 明写「本块不占档」。

## 9.3 本块的档号处置：**写死报出 `150`**

**本块不占档的方式与 P5e 相同（零表），但报出一个号**——这一条与 P5e 的处置不同，理由如下：

- 切分 §二第 5 条逐字要求「**其余五块与 P6 的档须在各自的块里写死后报出，不许各自挑**」，
  并给出判据「**「我没占它」不等于「它没人占」**」。
- P5e 设计 §11.3 已给出一个六块的排法**建议**：P5a `130`、P5b `140`、P5c `150`、P5d `160`、
  P5e `170`、P5f `180`，P6 `190`（该节逐字写明「这是建议不是裁定」）。
- **本块接受该建议并把它写死**：`150`。实测该号在 `130`–`199` 的整段里零命中（§9.2）。
  **这不产生迁移**——本块零表，故「150 归 P5c」是一条**登记**，不是一条死迁移。
  P5e 否定「预留一个没有表要建的编号」用的理由是「那会留下一条死迁移」，
  而**本处没有迁移可死**：登记的是一条归属，不是一条 SQL。

**收件人：协调者**（§11 第 6 条）：请把本块声明的 `150` 与其余各块已声明/未声明的档合并成一张表，
并把 P5d / P5f / P6 三段补齐——**本块的 `150` 与它们**都可能相撞，而本块只能声明自己那一段。

---

# 10. 测试策略与照片

## 10.1 清单的逐项守卫

- `all_operators()`：`len() == 9`，且 `id` 逐枚不同（两两比较，不是只查总数），
  且每枚的 `version == 1`。
- `CodeError`（2 臂）：两条错误路径各被覆盖一枚（§10.2 第 (b) 与 (d) 条即它们）。
- §3.5 的确定性名单（5 项）：一条**逐项**断言，且与 `all_operators()` 里
  `determinism == Deterministic` 的那些算子的候选逐项比对（不是抽样）。

## 10.2 判定侧：正例 + 反例成对（缺一条即不算钉住）

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| a | 以 `&all_operators()` 调 `register_code_operators` 注册 9 枚后，逐枚 `resolve` 成功 | 注册的正例 |
| b | 同一注册表注册两次 ⇒ `Err(CodeError::Registry(OperatorError::Duplicate { .. }))`（**不是**静默覆盖） | 注册的反例（§3.2：那一臂是 P1 的判定，本型只带出来） |
| c | `determinism == Deterministic` 的每一枚算子，其每个 backend 都在 §3.5 的名单内 | §3.5 的规则（正例） |
| d | 造一枚 `Deterministic` 但候选含名单外 backend 的算子，以它连同若干合法算子调 `register_code_operators` ⇒ `Err(CodeError::UnverifiableDeterminism { .. })`，**且注册表内容与调用前逐枚相同**（先核后写，不留半注册） | **fail-open 的那一侧**（§3.5）；「注册表未变」那一半钉的是同一条规则的**写入面** |
| e | `codew` 等**表外**的 `OperatorId` 构造出的算子与合法算子混合注册，逐枚成功（id 是开放串，不校验） | 上一条的**另一侧**（否则 d 可由「一律拒」满足） |
| f | 第 2、3、4、5、6、9 行的 `side_effect_class == Pure`，第 1、7、8 行的是 `Idempotent`，**九枚无一为 `NonIdempotent`** | §3.3 第 4、8、9 条，逐枚（不是抽样） |
| g | 第 7、9 行的 `determinism == Deterministic`，第 1、2、3、4、5、6、8 行是 `NonDeterministic` | §3.2 表那一列的**逐行**断言（不是抽样） |
| h | 第 1 行的 `input_schema` 是空 `Vec`，且 `output_schema == [SourceTree]` | §3.3 第 2 条 |
| i | 第 5 行与第 6 行的 `input_schema`、`determinism`、`side_effect_class` 逐字段相同，而 `id` 不同 | §3.3 第 10 条（「分离」这一条只有两枚同形的东西才说得上） |
| j | §3.4 边表**逐行**：下游 `input_schema` 的每一型都能在该行列出的上游算子里找到，且那一枚的 `output_schema` 含该型（**交集中的型**，不是「两侧相等」）。**`code-plan` 那一行除外**（其输入由装配方以 `Json` 值投射，不由任何算子提供，见 §3.4） | §239 的注册期一半（§3.4） |
| k | 七条 `software/` 方法的 `realized_by` **逐条**等于 §7.1 表（含 `TDD` 与 `debugging` 为空 `Vec`） | §7.1 的登记（**逐条**，不是「非空即过」） |
| l | 五条非空 `realized_by` 里的**每个串** `OperatorRegistry::resolve` 成功 | §7.1 末（P5b 的文本弱引用的运行期补件） |
| m | `all_operators()` 的 9 个 id 里有 **8 个**出现在 §7.1 表的非空 `realized_by` 里；**手写那 8 个的名单**（`code-workspace` `code-plan` `code-decompose` `code-spec-review` `code-quality-review` `code-test-run` `code-fuzz-run` `code-verify`），**`code-implement` 不在其中**（见下段） | §7.1 与 §3.2 之间的逐枚对应（这条钉的是「目录条目与算子集互相对得上」） |
| n | `CodeError` 与 `MethodError` 是两个不同的类型（`bind_code_methods` 的返回类型是 `Result<(), MethodError>`，不包进 `CodeError`） | §7.1 第 4 段（判定是两件事则分开） |
| o | §3.6 的照片：`code-test-run` / `code-verify` 两枚的 `cache_key` 为 `Some`，且四条件齐时 `can_reuse` 为真；`code-workspace` / `code-plan` / `code-decompose` / `code-implement` / `code-spec-review` / `code-quality-review` / `code-fuzz-run` 七枚为 `None` | §3.6 的**逐枚**（不是抽样）；其中 `code-workspace` 那一枚是 §3.3 第 3 条的**照片** |
| p | `code-workspace` 的两个 `backend_candidates` 逐名等于 `WorkspaceBackend` 两臂的编码（`crates/continuum-workspace/src/persist.rs:177-182` 的 `backend_str`，即 `"worktree"` 与 `"overlay"`） | §4.2 的「两个候选名与 P2 已落地的两臂对应」（§3.5 末：以编码为准，不取 §255 的散文形） |

**一条预防**：第 (m) 条的期望清单**必须手写**。若写成遍历 `all_operators()`、再对每枚去
`realized_by` 里找，它测的是「每个算子都在某个 `realized_by` 里」——**而「某条目录条目绑的算子
是否存在」不会被它测到**（那是第 (l) 条），反过来「有没有孤儿算子」也不会被 (l) 测到。
**语料从被测清单里取就是恒真的假照片。**

**另一条预防**：第 (k) 条的期望值必须手写，不能写成「非空即通过」。P5b 判据 6 的形态是「非空」，
而本块**有意**让两条为空（§7.2）；写成「非空即通过」会把 §7.2 的裁决当场抹掉。

**第 (m) 条要单独说明一处**：9 枚算子里有 **1 枚没有被任何 `realized_by` 引用**，即 `code-implement`。
**它不构成缺陷**：§186 `:68` 的 `Implement` 是链上的一步，而 §187 `:118-124` 的七条是**方法名**
（「怎样做得可靠」），两者不是同一张表（§7.3 已就 `review` 一侧写过同一条）。**故这一枚是刻意的**，
它使「算子集 ⊋ 目录条目的对家集」这件事在一条断言里可见——**若某一轮把 §7.1 表里某一行填错
（例如把 `planning` 填成 `["code-implement"]`），(m) 会红而 (k) 仍绿**（那一行仍是非空的）。

## 10.3 变异预告（谁红）

四档，每档预告**哪些用例红、哪些不该红**（预告写反会让人去查不存在的问题）：

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：§3.5 的名单判定改成恒真 | d 的第一个子例 | c（名单内的候选仍然通过）、a |
| **取反**：`register_code_operators` 改成边核边写（先注册前几枚，遇到违规才返回） | d 的第二个子例（注册表内容与调用前不同） | c、a（合法批次两侧都绿） |
| **放宽**：把 `code-workspace` 的 `determinism` 改成 `Deterministic` | o 的 `code-workspace` 那一枚（`None` 变 `Some`）、g 的第 1 行 | c（`worktree` / `overlay` 两个候选**都在** §3.5 的名单里，故注册期那条规则**不响**——见 §3.5 末）、p |
| **收紧**：把 `code-test-run` 的 `determinism` 改成 `NonDeterministic` | g 的第 7 行、o 的第二组 | c、a、p |
| **移除**：把 `code-fuzz-run` 从 `all_operators()` 删掉（8 枚） | `len() == 9`、a 的对应枚、k 的 `fuzz` 行、m（手写的那 8 个 id 名单里少一个） | 其余 |
| **取反**：把 `TDD` 的 `realized_by` 从空改成 `["code-implement", "code-test-run"]` | k（§7.1 表的 `TDD` 行） | l（那两个 id 都能 resolve，故 l 仍绿——**这一档若不红，说明 k 是按「非空即过」写的**） |
| **取反**：把 `code-spec-review` 的 `input_schema` 改成 `[Patch]`（去掉 `Json`） | j 的第 5 行（下游的每一型都要能找到） | 其余各行 |
| **等价变异**：把某枚算子的 `backend_candidates` 里两项**换序** | **全绿**——这是一枚**等价变异体**（候选列表无序，本块没有任何断言读它的次序）。**要打红它必须换变异体**：删掉一项，那时 c 或 p 红 | —— |

## 10.4 结构层的照片（`trybuild`）

本仓已有 `trybuild` 的既有做法（实测：`grep -rln "trybuild" crates/*/Cargo.toml` 命中**八处**——
`continuum-capability` / `continuum-node` / `continuum-secrets` / `continuum-provider` /
`continuum-connector`（其 `Cargo.toml:53` 是 `trybuild = { workspace = true }`，用法在 `:49` 的注记里）/
`continuum-workspace` / `continuum-model-registry` / `continuum-runtime`）。
**这八处是实测的清单，不是「例如」**——本处只引用它做先例，不代表本块会用到八处。**本块用一条**：

1. `Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现）在 `continuum-code` 里编不过
   ⇒ 在本块的 crate 里**造不出** `Evidence`，绕过构造点在类型上写不出来。
   这是 P5a 设计 §4.4「唯一产生点」那条结构事实在本块的**使用侧**照片。
   **这一条证不到「产出只能经本块的某枚函数」**：`Evidence::from_tool_result` 是 `pub`
   （P5a 设计 §4.4），本块的生产代码直接调它——**这正是 §6.2 的取向**（本块不建第二枚包装），
   故本块**没有**那类照片，也不该有。

**本块不用第二条 `trybuild`**：P5e 设计 §12.4 第 2 条用的是「`EvidenceSubject::Requirement(..)`
在本块里构造 ⇒ 编不过」。**那一枚在本块不成立**，因为 `continuum-verify` 的
`EvidenceSubject` 必须 re-export `RequirementId` 才能让 `from_tool_result` 的调用方传它——
P5e 已把该条的成立条件记成对 P5a 的一枚假设（P5e 设计 §13 第 5 条 (iii)）。
**本块不复制一枚成立条件挂在别人块上的照片**（本仓的判据：一条靠别人不改才成立的照片，
在别人改的那天静默变假）。

## 10.5 拍不出照片的地方

1. **端到端（一个代码 Intent 从 `Queued` 走到某处）**：执行器（第 3 层）未建，
   且切分 §四第 4 条禁止 P5 自建执行路径。本块的全部照片都是**单元与集成层**。
   与 P5a 设计 §12.4 第 1 条、P5e 设计 §12.5 第 1 条同形。
2. **`code-workspace` 真的建出一棵隔离工作区**：本块只**声明**算子，实现体（`OperatorImpl`）
   不在本块的交付物里（切分 §四第 4 条）；且 §4.4 的缺口使「算子的实现体拿不到工作区句柄」。
   故只能拍到「算子的声明取值正确」（第 (h)、(p) 条），拍不到「它建出了什么」。
3. **§305 的复用真的省下了一次重跑**：`can_reuse` 今天无生产调用方（P1 设计 §18 把它列在
   「无执行点的机制」里），且 `CacheKey` 无存储。故只能拍到 (o) 那条返回值。
4. **§307 的「非幂等不自动重试」在本块没有照片，据实记**：重试的决策点在
   `crates/continuum-graph/src/failure.rs` 一侧（P1），而本块九枚**无一为 `NonIdempotent`**（§3.3 第 8 条），
   故那条禁令在本块**没有主体可拦**——第 (f) 条拍的是「九枚的取值」，不是「拦下了一次重试」。
   **这不是缺口**：判据是「有没有改变外部世界」，本块今天没有那类算子；将来出现才有照片。
5. **两条臂的证据真的产了出来**：§5.3 报出的 `ModelReview` 臂于 2026-10-10 在 P5a 落地（其设计 §4.2）。
   **本条照片仍缺一半**：证据的**构造**在宿主的执行代码（第 3 层），而本块只声明 `evidence_type`
   （§5.2 表、§6.2 末）——**本块侧可拍的只有「不写死它」这件事**，它是「代码里没有那一行」。

---

# 11. 与其余各块的对账条目（逐条具名）

以下每条都是**本设计假设了别的块的某枚形状**或**发现某处无归属**之处。

1. **待与 P5a 对账**（四条）：
   (i) **本块报出一枚 `EvidenceType` 的需求**：`code-spec-review` 与 `code-quality-review` 两枚
   产出的「模型审查判定」在 §258 的九臂与 §193 的两臂里无臂可落（§5.3），本块提议加一枚
   `ModelReview`。臂的归属、命名与是否加由 P5a 决定（切分 §四第 1 条：证据类型是 P5a 的）。
   (ii) **§193 的 `Differential` / `Metamorphic` 两臂本轮在本域无生产者**（§5.3 末）——
   P5a 设计 §14 第 2 条要求本块就此说明。本块不主张删它们。
   (iii) **「测试是否通过」的四项答复**（§5.4）：承载处 = `TestResult` 制品；谁写 = `code-test-run`
   与 `code-fuzz-run`；写在哪一列 = `artifact` 表的既有列；**payload schema 本块不定义**，
   与 P5a 设计 §15 第 9 条是同一处缺口的三分之一。
   (iv) **`code-verify` 的级别声明**（§3.3 第 11 条）：本块声明它是 §262 第 1 级的比较器，
   而**候选的构造与级别序的判定在 P5a**（其设计 §7.2 的 `AvailableVerifier`）。
   「这一对应由谁持有」未定（与 P5e 设计 §13 第 5 条 (ii) 同形）。
2. **待与 P5b 的持有方 ＋ 协调者对账**（§7.2）：`software/` 的 `TDD` 与 `debugging` 两条
   `realized_by` 为空，原因是 `MethodEntry.realized_by: Vec<String>`（算子 id 的集合）**表达不了
   次序与环境**，且链上没有调试这一步。**后果**：P5b 判据 6 在 `Software` 域上会红，而红的原因不是漏 bind。
   三条处置建议与其代价写在 §7.2 表里，本块不选。**本块也不发明 `code-debug`**（理由同切分 §八 的 `3d/`）。
3. **待与 P1 ＋ 第 3 层的执行器对账**（两条，同一处的两半）：
   (i) **输入端口的取值域**：`code-workspace` 的真实输入（Base 工作区）不在
   `input_schema: Vec<ArtifactType>` 的表达范围内，故它不进 §303 的 READY 判据与 §239 的连接判定
   （§4.3）。
   (ii) **执行上下文**：`NodeContext`（`crates/continuum-graph/src/execution.rs:43-47`）没有工作区句柄，
   而链上第 4–8 行五枚算子都要在 Task Workspace 里干活（§4.4）。
   两处都指向同一次改动。**本块不就地补**（切分 §四第 4 条）。
   另：`Integration Gate` 因「批准值没有位置装」（同上 (ii) 的签名）而不算子化（§4.2）。
4. **待与协调者对账（`code-workspace` 的每 Intent 一枚）**：§16 `:796` 要求一个 Intent 一份工作区，
   而算子是每节点的执行单元（§244）。**本块无法在类型层钉住「图上只有一个该节点的实例」**（§4.2 末），
   故这条约束的落点在 Planner 的构造上。
5. **待与 P4 ＋ 协调者对账（两个「Plan」）**：§228 的 `Plan` 是 P4 的对象（`docs/spec/05-normative.md:334` 起），
   而 §186 `:62` 的 `Implementation Plan` 是本块 `code-plan` 产出的 `Json` 制品（§8）。
   **两者同名不同物**，而本仓对「同一件事两个词汇表」一贯判为 Critical——
   本处是「同一件事两个名字」的反向（同一名字两件事），失配方向是**读者把两者当成一个**。
   本块**不改名、也不合并**（改名要动 §186 的正文或产一枚新 `ArtifactType`，两者都不在本块范围）。
   **收件人：P4 的持有方 ＋ 协调者**（决定是否要在某处写明两者的分别）。
6. **待与协调者对账（迁移号段）**：本块声明 `150`（§9.3，实测 `130`–`199` 全段零命中）。
   P5a 已取 `130`；P5b 与 P5e 零迁移（本块实测，§9.2）；**P5d / P5f / P6 三段未声明**。
   请把六块与 P6 的档合并成一张表。**与 P5e 设计 §11.3 重复**：那一节已提同一建议
   （六块按 P3 先例后延）并明写「不另开对账条目」（理由是该块零迁移、无号可占）。
   本块与之的差别只在本块**写死了一个号 `150`**，故保留本条请求；号段由协调者一次合并，两处不各记一次。
7. **待与规范维护者对账（§3.5 的封闭表）**：§3.5 的「逐位可复现的 backend」名单是本设计定的，
   规范没有一处给 backend 的确定性。**缺的是「该 backend 的什么属性使输出逐位可复现」这一步。**
8. **待与规范维护者 ＋ P5a 对账（`TestResult` 的载荷）**：§5.4 第 4 条。与 P5a 设计 §15 第 9、14 条
   互为指针。
9. **待与 P2 对账（`code-workspace` 的两个 backend 名）**：§10.2 第 (p) 条的照片把两个候选名钉到
   `WorkspaceBackend` 两臂上。若 P2 后续按 §255 `:1029-1037` 补第三臂，本块的 `backend_candidates`
   与 §3.5 的名单随之各加一项（**这是 P2 的一次改动，不是本块的**）。

---

# 12. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **（已关闭，2026-10-10）两枚审查算子的 `EvidenceType` 无臂可落**（§5.3）——P5a 收下本块报出的
   `ModelReview` 并落地（P5a 设计 §4.2）。**此处保留原条目，是因为「这条缺口曾存在」这件事本身是
   §5.3 那三条处置的证据**；缺的那一步已由 P5a 走完，收件人不再是 P5a。
2. **`software/` 的 `TDD` 与 `debugging` 无算子对家**（§7.2）。**缺的是「方法是次序/环境约束时，
   登记形态怎么表达」这一步。收件人：P5b 的持有方 ＋ 协调者。**
3. **L2 工作区与算子之间的两条通道都缺席**：Base 进不了端口（§4.3）、工作区句柄进不了执行上下文（§4.4）。
   **缺的是「跨层的非制品输入怎样进入判定与执行」这一步。收件人：P1 ＋ 第 3 层的执行器。**
4. **`code-workspace` 的「每 Intent 一枚」在类型层不可钉**（§4.2 末、§11 第 4 条）。
   **缺的是「算子实例的基数由谁保证」这一步。收件人：协调者（Planner 的归属未定）。**
5. **§228 的 `Plan` 与本块的 `Json` 制品同名**（§8、§11 第 5 条）。**缺的是「要不要在某处写明
   两者的分别」这一步的裁定。收件人：P4 的持有方 ＋ 协调者。**
6. **§3.5 的 backend 确定性名单是本设计定的封闭表，规范无来源**（§11 第 7 条）。
   **收件人：规范维护者。**
7. **`TestResult` 的载荷 schema 未给**（§5.4 第 4 条）。**缺的是「一份 `TestResult` 里哪些字段是
   判『通过/不通过』所必需的」这一步。收件人：规范维护者 ＋ P5a。**
8. **`code-workspace` 的每轮重建代价**（§3.3 第 3 条）：它声明 `NonDeterministic` 故不进 §305 的复用。
   **这不是缺口，是取 fail-closed 的代价**；据实记在此处，免得被读成「忘了声明 `Deterministic`」。
9. **本块今天没有生产调用方的落点，逐类列出**：三处登记入口 `register_code_operators`（§3.2）、
   `bind_code_methods`（§7.1）、`all_operators()`（§3.2）；§3.2 表里的九枚 `determinism` 声明；
   以及 §5.2 的四条证据产出面（后者的调用方是装配方，本块不建那个包装，§6.2）。
   **这不是本块的缺口，是第 3 层的**（切分 §四第 4 条禁止 P5 自建执行路径；P1 设计 §18 已有同形的一条；
   拍照的限制见 §10.5）。
10. **迁移档 `150` 只在本块一侧写死**（§9.3、§11 第 6 条）：P5d / P5f / P6 三段未声明。
    **缺的是「六块与 P6 的档谁在哪一步合并成一张表」这一步。收件人：协调者。**

---

# 13. 在切分文档里发现的错或缺口（**只记，不改**）

逐条给出实测。**本设计不改切分文档**（切分 §七 与协调纪律）。

1. **⚠️ §五 P5c 行说「并让每步产出 P5a 的 Evidence」，而 §186 的链上有两步产不出证据、
   另有两步的证据类型在 §258/§193 里无臂可落。**
   实测：①`code-plan` 与 `code-decompose` 产出的是制品，其存在由 §266 第 1 项 `ArtifactExists` 判
   （P5a 设计 §8.3 第 1 项），不是 `Evidence`（§5.2）；②`code-spec-review` 与 `code-quality-review`
   的产出（模型审查判定）在 §258 `:1094-1102` 的九臂与 §193 `:368-373` 的并集里无臂（§5.3）。
   后果：若照那一句的字面去实现，「每步产出证据」会逼出一个语义不符的臂（`StaticAnalysis`），
   而失配是静默的（一次模型判断拿到确定性检查的优先级，§5.3）。
   **判据**：**「每步产出 X」这句话，要先逐点核「该步的产出是不是 X 那一类」**——
   本行把「链上的步」与「产证据的步」当成同外延了。
   **处置**：本设计**不改那一句**（它是范围声明），改为在 §5 逐枚给出产出面并报出两处无臂（§11 第 1 条）。
2. **⚠️ §五 P5c 行与 §四第 3 条都没有指出「`software/` 的七条里有两条不是算子」。**
   实测：§187 `:118-124` 的 `TDD` 的全部内容是**次序**（先写测试后写实现），而
   `MethodEntry.realized_by` 是 `Vec<String>`（P5b 设计 §4.3）——**一个集合表达不了次序**；
   `debugging` 则**在链上没有对应的步骤**（《总纲》`:1448` 的六步与 §186 `:57-77` 的十行都没有调试）。
   后果：照 §四第 3 条「四块只填内容」去填，两条要么被填上一个语义不符的算子对家，
   要么被留空而触发 P5b 判据 6 变红——而**两种都会在 P5b 的 crate 里看不出原因**
   （那两条判据都不读 §187 的正文）。
   **判据**：**「A 的条目由 B 填」这句话，要先核「A 的每一条在 B 的词汇表里都有对家吗」**——
   本行只核了「目录与块是一对一」（这一条已由 P5b 的设计订正过），没核**目录里的每一条**。
   与 §八 的「`3d/` 有目录、无对家」是同一种错位，只是粒度细一层（那是「整个目录」，
   本处是「目录里的两条」）。
3. **⚠️ §四第 7 条只举了「复用判据」一例，而它今天的射程里另有两处已落地的判据没举。**
   实测：§四第 7 条全文（切分文档 `:180-186`）只举复用判据一例
   （`crates/continuum-graph/src/reuse.rs:29` 的 `can_reuse`），并写「发现一处、登记一处」。
   **已落地的、与本块相邻的另两处是**：①§239 的端口兼容 `crates/continuum-port/src/port.rs:94`
   的 `compatible`；②§245 的后端候选判定 `crates/continuum-graph/src/execution.rs:87`
   的 `is_candidate_backend`。
   后果：读者会以为该条只约束复用一处，而它的**射程里今天就有三处**——另两处正是四个领域算子块
   **最容易各写一遍**的东西（「我这个 schema 能不能接那个」与「这个 backend 在不在候选里」
   各写一遍函数，太自然了）。
   **判据**：**列「已落地的判据」时，要按 `grep` 的结果列，不能按印象列**——
   本条在第一枚实例（复用判据）上做对了，另两处却漏了。
   **处置**：本设计按实测把 ①② 写进 §3.4 与 §1.2（本块只声明、不写第二份判定）。
   **订正来历**：本设计早先的稿子在此处写过「§四第 7 条举的三处例子（§226 四类差异、§214 冲突、
   表达式比较）」——**该三个例子出自协调者的派单口径，切分文档里没有**。实测：切分文档的
   `:180-186` 查不到 `§226` / `§214` / 表达式比较；`grep -rn "ContractDiff" crates/` 与
   `grep -rln "expression" crates/` 零命中，`crates/continuum-semantics` 不存在（P4 未开始实现）。
   三处实出自 P4 的设计与计划（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:1050`、
   `:1081-1083`；`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:155-156`），
   是 P4「只建类型不建检测器」的三项。**记在此处，以免同一条错误再被照抄。**
4. **✔ 已复测为正确的两条**（记在此处是为了让复审不必重做）：
   (i) §一 说的「工程 §8.1 组件表里的四行（`docs/02-工程.md:495-498`）」——实测 `:495` 是「代码领域算子」、
   `:496` 研究、`:497` 媒体、`:498` 图像，四行连续（表头 `:483`、第 1 行 `:485`），准确。
   (ii) §五 P5c 行的「主要规范依据」栏里的 `§16`——实测它**不在** `docs/spec/05-normative.md` 里
   （该文件无 §16），实质在 `docs/spec/01-concepts.md:788`「16. Git L2 Sandbox」；P5b 设计第六节的
   `worktree` 那一格已把这件事写明，本设计据它读。
