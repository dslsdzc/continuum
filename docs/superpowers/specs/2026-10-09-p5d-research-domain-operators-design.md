# P5d 研究领域算子 — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md` §五 的 P5d 行的原话，逐字照用）：

> 把 §330 的七步链（Question → Search → SourceSet → EvidenceExtraction → ContradictionCheck → Synthesis → CitationVerification）落成一组可注册的 Operator；其中 `ContradictionCheck` 与 `CitationVerification` 是给 P5a 的证据侧供给

**本块拥有 / 产出的接口**（同行的第二栏）：本领域的 Operator 集与它在 P5b 目录里的条目。

**规范依据**：§330（`docs/spec/05-normative.md:2447`）；《工程》§8.1 第 12 行（`docs/02-工程.md:496`）；《总纲》§8.4（`docs/01-总纲.md:1451`）。

**本文件只写 P5d。** 其余五块（P5a 验证与证据、P5b 执行方法库、P5c/P5e/P5f 三个领域算子）各有其设计，
本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §10 写成一条**具名的对账条目**，
不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被依赖（B 依赖 A）」
（判据与订正原文在 `docs/02-工程.md:563-564`，逐字「**箭头的读法：被依赖者 → 依赖者**」；
`:588` 逐字「依赖方向单向，无环。」）。凡引他文的箭头，原样照录并标注。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p5d`）里当场 `grep -n` / `awk` 实测的，
未照抄任何一份别的文档（含切分文档、P5a/P5b/P5e 三份设计）的行号。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| 研究领域的 Operator 集（7 枚，§330 的七步各一枚） | 切分 §五 P5d 行；§330 | §4 | 全量（id、端口类型、`determinism`、`side_effect_class`、backend 候选、逐枚理由） |
| 算子集在 `OperatorRegistry` 上的登记入口 | 切分 §二「无需重新冻结」段（`Operator` / `OperatorRegistry` 由 P1 落地） | §4.1 | 全量（一枚函数 + 逐枚注册的原子性规则） |
| `ContradictionCheck` / `CitationVerification` 的证据侧供给 | 切分 §五 P5d 行；P5a 设计 §4.2 与 §14 第 8 条 | §5 | 产出面全量；**两枚 `EvidenceType` 新臂由 P5a 落地**（本块只报出，§5.1） |
| `SourceSet` 的处置 | P5e 设计 §3.4（`docs/superpowers/specs/2026-10-09-p5e-media-domain-operators-design.md`，按小节引） | §3 | 全量（接受 P5e 的读法，逐条给实测理由与代价） |
| `research/` 目录的四条 `MethodEntry` 的 `realized_by` | P5b 设计第六节 | §7 | 四条逐条填出 |
| 迁移号段 | 切分 §二第 5 条 | §8 | **本块零迁移**；附实测空档表 |

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| `Evidence` 类型、`EvidenceType` 的臂、`VerificationPolicy`、Requirement Coverage、完成判定 | P5a（切分 §四第 1 条）。本块**不得**定义证据类型、「充分」判据、完成判定。本块对 `EvidenceType` 的需要以**报出**的形态出现（§5.1） |
| `ArtifactType` 的任何新变体 | P5e（切分 §四第 2 条：只由 P5e 一处扩展）。本块需要的型**全部是既有的**（§3.2） |
| 方法的登记形态、`MethodRegistry`、`MethodEntry`、`MethodDomain` | P5b（切分 §四第 3 条）。本块只调它的 `bind`（§7） |
| 节点状态迁移、`Queued → Running`、`OperatorRegistry::resolve` 的调用点、算子的执行 | 第 3 层（切分 §四第 4 条）。本块只**注册**算子 |
| `execution_policy` 的判定 | P4（切分 §四第 5 条）。本块不碰 `verification_policy`，也不碰 `execution_policy` |
| 「逐位可复现的 backend」名单（P5e 设计 §5.4 的那一份） | **本块不建第二份**（切分 §四第 7 条）。处置见 §6.2 |
| 复用判据（缓存键、`can_reuse`、`cache_key`） | P1 已落地（`crates/continuum-graph/src/reuse.rs:29` / `:16`）。本块只**消费**（§6.1） |
| §88 的 capability token（检索类算子触网所需的授权） | 能力面（P3a）。本块只声明算子的 `backend_candidates` 与 `side_effect_class`；授权以何形态进入执行路径不在本块 |
| §34 的生成授权旗标 | **不适用**：§34 的五个旗标（`docs/spec/01-concepts.md:1424-1428`）逐枚点名的是生成类内容（`allow_generated_broll` 等），本块不注册生成类算子 |
| 检查点（`Checkpointable`） | P5e（切分 §二第 4 条）。本块七枚算子都是短任务，不实现它 |
| 迁移 / 表 | 本块**零表**（§8） |
| 执行算子时的证据装配上下文（面向哪条 Requirement） | 装配方与 P5a。本块的产出取 `EvidenceSubject::Unattached`（§5.2） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：`continuum-research`（实测：`grep -rn "continuum-research" . --exclude-dir=.git --exclude-dir=target`
在本仓**零命中**，名字未占用）。

依赖边表（**只登记实际用到的**，切分 §四第 6 条）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-artifact` | `ArtifactType`（§4.2 七枚算子的 `input_schema` / `output_schema` 取 `Vec<ArtifactType>`）、`ArtifactId`（§5.2 的 `artifact_refs`） | 普通 |
| `continuum-operator` | `Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` / `BackendId` / `OperatorRegistry` / `OperatorError`（§4.1 的登记入口） | 普通 |
| `continuum-method` | `MethodRegistry` / `MethodId` / `MethodError` / `bind`（§7 的 `bind_research_methods` 收 `&mut MethodRegistry`、以 `MethodId` 逐条调 `bind` 并透出 `MethodError`） | 普通 |
| `continuum-verify` | `Evidence` / `EvidenceType` / `EvidenceId` / `EvidenceSubject` / `EvidenceProducer` / `Claim` / `EvidenceStrength` / `EvidenceScope` / `Evidence::from_tool_result` / `VerifyError`（切分 §四第 1 条：本块**只产出**证据） | 普通 |
| `continuum-graph` | `can_reuse` / `cache_key`（§6.1 的照片：核本块声明的 `determinism` 真能让 §305 的判据成立） | **dev 边** |

`continuum-method` 是 P5b 新建的 crate，其 workspace 依赖集合为空集（P5b 设计第九节）；本块依赖它，
不构成环：`continuum-method` 不反向依赖任何 workspace crate，故 `continuum-research → continuum-method` 是单向的。

**不登记**的边，逐条给理由（零使用的边即假边，P2b 为此删过两条）：

- **不登记 `continuum-port`**：本块不构造 `Port`、不做端口兼容判定。`Operator.input_schema` / `output_schema`
  取 `Vec<ArtifactType>`（`crates/continuum-operator/src/definition.rs:82-83`），不经 `Port`。§239 的 `compatible`
  是 P1 的（`crates/continuum-port/src/port.rs:94`）。
- **不登记 `continuum-semantics`**：本块不构造 `RequirementId`——产出的证据一律 `Unattached`（§5.2）。
  该 crate 今天在 `crates/` 下不存在（P4 未开始实现，切分 §二末「与 P4 的接口面」段已记）。
- **不登记 `continuum-persist`**：本块零表（§8），不碰 `Tx` / `Migration`。
- **不登记 `continuum-events`**：本块不写事件（§8）。
- **不登记 `continuum-policy` / `continuum-capability`**：本块不查策略引擎、不取 Capability Token（§1.2 表第 8 行）。

**dev 边为什么也要登记**：`dependency_direction.rs` 的 `cargo tree` 带 `--edges all`
（`crates/continuum-runtime/tests/dependency_direction.rs:274`），dev 边与普通边一视同仁；
本仓已有同形的先例（`continuum-provider` 那一行注记明写「Task 4 起加上 persist：**dev 边**」，
同文件 `:32-34`；`continuum-connector` 那一行同形，同文件 `:115-116`）。

**边别按「谁在用」判**：`continuum-graph` 是 **dev 边**——生产代码不调它，用到它的是 §6.1 的照片
（§9.2 第 (k) 条对 `can_reuse` / `cache_key` 的断言），那是用例。

**共写文件**（本块的那部分；切分 §一 的共写文件表已登记这两处共写）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members`（实测：`Cargo.toml:3-22` 的 `members` 列 18 个成员，无 `continuum-research`） | 加一行 `crates/continuum-research` | 切分建议由先落地者一次加齐六行；本块**只加自己这一行** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-research", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify"])`——**五项**，与 §2.1 边表的五行逐行对应（含 `continuum-graph` 的 dev 边） | 新 crate 会先让该门变红再被补齐（`:288` 的用例与 `:291-305` 的互为覆盖断言），这是刻意的；数组按字母序 |

**本块不改任何别的 crate 的文件**——这一点与 P5e（它要改 `crates/continuum-operator/src/definition.rs`
与 `crates/continuum-artifact/src/artifact.rs`）不同，理由是本块不新造类型、不新加枚举变体（§2.2、§3.2）。

## 2.2 冻结清单：本块持有**零**个类型、**零**条判定

**类型**：本块不新造任何类型。

**判定**：本块是任何判定都不持有——七枚算子的 `determinism` / `side_effect_class` / backend 候选是**声明**，
不是判定：`determinism` 的消费者是 §305 的复用判据（P1 落地的 `can_reuse`），backend 的选择由 Router 决定
（§245 `docs/spec/05-normative.md:773` 逐字「具体 backend 由 Router 决定」），两处的判定都不在本块。

**这是可断言的，不是一句自谦**：`continuum-research` 的 `pub` 面只有**四枚项**，逐枚列在下面，无一项的类型由本块定义：

```
pub fn all_operators() -> [Operator; 7];                                   // §4.1
pub fn register_research_operators(registry: &mut OperatorRegistry, operators: &[Operator])
    -> Result<(), OperatorError>;                                          // §4.1
pub fn bind_research_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;   // §7
pub fn evidence_from_research_finding(
    id: EvidenceId, evidence_type: EvidenceType, claim: Claim,
    producer: EvidenceProducer, artifact_refs: Vec<ArtifactId>,
    strength: EvidenceStrength, scope: EvidenceScope,
) -> Result<Evidence, VerifyError>;                                        // §5.2
```

四条逐一给「它新造了什么」的答复：

- 前两条的类型全在 `continuum-operator`（`Operator` / `OperatorRegistry` / `OperatorError`）。
- 第三条的类型全在 `continuum-method`（`MethodRegistry` / `MethodError`）。
- 第四条的类型全在 `continuum-verify`（P5a 的），本函数**只固定一件事**：`subject` 取 `EvidenceSubject::Unattached`
  （§5.2）；它**不判**任何东西。

**「零个类型」的代价据实记**：本块因此也没有一枚可供别的块命名的领域类型。
这不是遗漏：本块要表达的东西（七枚算子的取值、两处映射）都落在既有类型上，凭空造一枚类型是替规范定一个版本化的外部接口。

**须与 P5e 对照的一处不对称**：P5e 的设计持有三枚类型（`GenerativePermission` / `PermittedGenerativeContent` /
`MediaError`）与四条判定。两块的差别有来源，不是风格差异：P5e 的 §34 授权判定与
§5.4 的注册期规则是**媒体域自己新加的**规范要求（§34 逐条点名媒体），研究域没有对应的条文。

## 2.3 本块的层间位置

§9.1 的层间图里与本块有关的两条，按「被依赖者 → 依赖者」读：

- `执行层 (3) → 跨领域 (8)`：§2.1 的边表**五行**全是它的实例（例如 `continuum-research` 依赖
  `continuum-operator`、`continuum-artifact`）。本块不重复切分 §二第 2 条的裁定。
- `跨领域 (8) → 长期循环 (7)`：本块**不接**这一条——本块不产出 `value + evidence + last_verified`
  （§232 的 `ProductReadiness` 属长期循环）。本块的产物注册进 `OperatorRegistry`，不写长期循环的表。

本块与 P5a（同层）之间有一条层内边 `continuum-research → continuum-verify`。**层内边在本仓有同形的处置**：
`continuum-provider → continuum-capability`（资源层内的「接口 → 能力类型」）就被登记为层内边
（`crates/continuum-runtime/tests/dependency_direction.rs:29-31` 的注记逐字含「层内边」，行在 `:35-38`）。
本条边是单向的：`continuum-verify` 不依赖任何 P5 领域算子块。

---

# 3. `SourceSet` 的显式处置

## 3.1 事实（逐条实测）

- §330 是七个名字的**唯一**出处：`docs/spec/05-normative.md:2449` 逐字「研究任务 SHOULD 拆成：」，
  其后 `:2452` `Question` / `:2454` `Search` / `:2456` `SourceSet` / `:2458` `EvidenceExtraction` /
  `:2460` `ContradictionCheck` / `:2462` `Synthesis` / `:2464` `CitationVerification`，每两名之间一行 `↓`。
- `SourceSet` 在 `docs/spec/` 全量里**只有这一处命中**（实测 `grep -rn "SourceSet" docs/`：`docs/spec/05-normative.md:2456`
  与 `docs/01-总纲.md:1456` 两处，后一处是同一张链条图的照抄；其余命中全在本层的文档里）。
- 三份 Artifact 类型清单逐行实测，**无一含 `SourceSet`**：
  §9 Typed Artifact（`docs/spec/01-concepts.md:503-514` 十二行）、§239（`docs/spec/05-normative.md:645-651` 七行）、
  §328（同文件 `:2413-2419` 七行）。

## 3.2 处置：接受 P5e 的读法，`SourceSet` 在本块里的形态是**一枚算子的名字**

**接受 P5e 设计 §3.4 的读法**（该节的理由：§330 的七个名字里六个是步骤名或算子名，把 `SourceSet`
读成类型要求在同一张表的七行里只对它一行另取一种读法）。**本块补上独立的实测理由**（上节）：
`SourceSet` 在规范全量里只有一处类型学意义上的候选出处，而那一处是**步骤链**，它**不在任何一份 Artifact 类型清单里**——
P5e 设计 §3.2 的取法是「三份清单的并集」，`SourceSet` 对那份取法的贡献是零。

**故 `SourceSet` 在本块里的形态是**：**§330 第三个步骤落成的那一枚算子的 id**（`research-source-set`，§4.2 第 3 行），
它的**产物**由既有的 `ArtifactType::Json` 承载（`crates/continuum-artifact/src/artifact.rs:19`）。

**这里不是「§330 没提 Artifact 所以不收」**：判据是**有没有类型学出处的清单**，不是「这一节里有没有 Artifact 这个词」——
同一节里的六个名字与本条走的是同一条判据（它们都不是类型，故都不是变体）。

**代价据实记（本块为这一读法付的那一份）**：`research-search → research-source-set →
research-evidence-extraction` 三条相邻边的端口因此都是 `Json`，§239 `:654`「运行时 MUST 拒绝不兼容连接」
在这三条边上退化为「`Json` 接 `Json`」，**没有区分力**（§4.5 的边表逐行标出了这一点）。
**若协调者事后裁定 `SourceSet` 该收成一型**，改动是「向 P5e 报出一次枚举变体（按切分 §四第 2 条的『一次改动』惯例）
＋ §4.2 三行算子表的端口从 `Json` 改成 `SourceSet`」，本块的判定归属不变。

**本块不按切分 §四第 2 条报出**：那一条要的是「需要**新变体**就报出」，而本块的七枚算子需要的型**全部是既有的**
（§4.2 的端口一栏逐格可核：只用到 `Json` 与 `Report` 两型，两型都在今天的枚举里或在 P5e §3.2 的清单里）。
报出一个本块并不需要的变体，是发明。

**`Report` 这一型的生产者在本块**：§9 `docs/spec/01-concepts.md:514` 逐字 `Artifact<Report>`，
而 P5e 设计 §5.2 末明写 `Scene` / `Mesh` / `Report` 三型在**媒体块**没有生产者。本块的
`research-synthesis` 出 `[Report]`（§4.2 第 6 行），故 `Report` 的**第一个生产者在本块**。
**互指**：P5e 设计 §5.2 那句与本节是同一件事的两半，两处都留。

---

# 4. 七枚算子

## 4.1 登记形态：`Operator` 照 P1，**交付面是一枚函数不是常量**

§244（`docs/spec/05-normative.md:735-753`）的 `Operator` 八字段由 P1 落地为
`crates/continuum-operator/src/definition.rs:79-88`：`id` / `version` / `input_schema: Vec<ArtifactType>` /
`output_schema: Vec<ArtifactType>` / `determinism` / `side_effect_class` / `backend_candidates: Vec<BackendId>`。
**本块不加字段、不改签名**（切分 §二「无需重新冻结」段）。本块交付的是**内容**：七枚 `Operator` 值的清单与登记入口。

### 4.1.1 **本仓实测：`Operator` 的清单写不成 `const`**

P5e 设计 §5.1 把清单定成 `pub const ALL_OPERATORS: [Operator; 17]`，并在同节给出理由「本仓的既有约定是
『枚举/清单带 `ALL` 常量 + 数目断言』……并把「17」写成类型的一部分（`[Operator; 17]`），使清单个数的改动**编译期可见**」。
**实测：那个 `const` 编译不过，理由两条，各自独立成立**：

1. `Operator` 的字段含两个 `String`（经 `OperatorId` `crates/continuum-operator/src/definition.rs:24` 与
   `BackendId` `:66` 各持一个），而 `OperatorId::new`（`:25`）不是 `const fn`；Rust 的 `const` 上下文不能堆分配，
   故一个**非空**的 `String` 在 `const` 里造不出来。
2. **这一条更硬，且不依赖第 1 条**：`Operator` 的 `input_schema` / `output_schema` / `backend_candidates`
   都是 `Vec`（`:82-83`、`:88`）。`Vec::new()` 是 `const fn`（空 `Vec` 可 const），但**推入元素不是 const 操作**
   ⇒ 一个**非空**的 `Vec` 在 `const` 上下文里造不出来，与元素类型无关。
   本块七枚算子的三个 `Vec` **没有一个是空的**（§4.2 逐格可核：`research-question` 的 `input_schema` 为空，
   但它的 `output_schema` 与 `backend_candidates` 都非空）⇒ 七枚**逐一**不可 const。

**本仓的既有约定为什么在这里失效，写清**：`ArtifactType::ALL`（`crates/continuum-artifact/src/artifact.rs:28`）、
`EffectType::ALL`（`crates/continuum-effect/src/effect.rs:31`）、`PrivacyClass::ALL`（同 artifact 文件 `:100`）
**都是无字段枚举**——无字段枚举的变体是常量，故 `const` 成立。**约定照抄的是「带 `ALL` + 数目断言」这个形状，
而形状能成立的前提是「元素可 const 构造」，那个前提在这里不存在。**
**判据**：**引用一条既有约定时，要连它的成立条件一起搬**——只搬形状，就会写出一条编译不过的「约定」。

**本块的交付面**：

```
/// 本领域七枚算子的全集。顺序即 §4.2 表的行序。
/// **返回值是数组不是 `&'static [Operator]`**：`Operator` 含 `Vec` 与 `String`，
/// 不可 const 构造（理由见设计 §4.1.1），故只能每次构造。
/// 不变量：七枚的 id 逐枚不同、version 均为 1——由 §9.1 的 id 两两比较把守。
pub fn all_operators() -> [Operator; 7];
```

**「7」仍然写在类型里**（返回类型 `[Operator; 7]`），故 P5e 想要的那条性质——「清单个数的改动编译期可见」——
**由返回类型承载，不因它不是 `const` 而丢失**：把清单改成 6 枚或 8 枚，所有以 `[Operator; 7]` 为类型的位置一起编译不过。

```
/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核「(id, version) 是否已在表里」与「批内是否自撞」，全部通过后再逐枚注册；
/// 任一枚不成即返回 `Err`，此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚研究算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§9.2 第 (c)(d) 条要能对**任意** `Operator` 触发。
pub fn register_research_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), OperatorError>;
```

**为什么本函数不新造错误类型**：P5b 设计 §4.4 已把这条纪律写成「不新增第三个错误类型」，本块照办。
判据是「判定是同一件事则合并，是两件事则分开」：本函数要报的失败**只有**「(id, version) 撞车」一件事，
而它的判定**已经有一枚臂**（`crates/continuum-operator/src/registry.rs:11` 的 `OperatorError::Duplicate`，
判定在 `:24-34` 的 `OperatorRegistry::register`）。**这里不是「照 P5e 的样子包一层 `MediaError`」**：
P5e 的那个类型承载的是它自己的两条领域规则（§5.4 的注册期规则与 §7.2 的生成授权），本块**没有**领域规则（§6.2），
包一层就是「同一件事两个词汇表」。

**「先核后写」不是本块发明的判据**：它是**注册这个动作的语义**——`OperatorError::Duplicate` 的判定在 P1，
本函数只是把同一条判定在写之前跑完。核的实现只用 `OperatorRegistry` 的 `pub` 面（`resolve`，`:36-47`），
不读它的私有 `entries`。

**「先核后写」的失效方向（为什么值得写这一条）**：不写它，一次带重复项的注册会留下**半注册**的表——
而调用方拿到的 `Err` 说的是「这次调用没成功」。**调用方据 `Err` 重试一次，第二批会从重复项处再失败**，
表里那一半却已经在了。这是「一次失败留下不可见的副作用」，与 P5e §5.1 那条规则的取向同向。

## 4.2 七枚算子

出处列的口径：**算子名与端口类型**照录 §330 的步骤名（该节只给七个名字，**不给端口类型、不给 determinism、
不给 backend**——实测：§10 Domain Operator `docs/spec/01-concepts.md:540-552` 的六条签名里**没有一条是研究域的**，
故本表除「步骤名」一栏外，其余各栏都是**本设计的判定**，逐行标出）。

| # | id | input_schema | output_schema | determinism | side_effect_class | backend 候选 | §330 步骤（出处） |
|---|---|---|---|---|---|---|---|
| 1 | `research-question` | `[]` | `[Json]` | `NonDeterministic` | `Idempotent` | `primary-model` | `Question`（`:2452`） |
| 2 | `research-search` | `[Json]` | `[Json]` | `NonDeterministic` | `Idempotent` | `retrieval-backend` | `Search`（`:2454`） |
| 3 | `research-source-set` | `[Json]` | `[Json]` | `Deterministic` | `Pure` | `builtin` | `SourceSet`（`:2456`） |
| 4 | `research-evidence-extraction` | `[Json]` | `[Json]` | `NonDeterministic` | `Idempotent` | `primary-model` | `EvidenceExtraction`（`:2458`） |
| 5 | `research-contradiction-check` | `[Json]` | `[Json]` | `NonDeterministic` | `Idempotent` | `primary-model` | `ContradictionCheck`（`:2460`） |
| 6 | `research-synthesis` | `[Json]` | `[Report]` | `NonDeterministic` | `Idempotent` | `primary-model` | `Synthesis`（`:2462`） |
| 7 | `research-citation-verification` | `[Report, Json]` | `[Json]` | `NonDeterministic` | `Idempotent` | `primary-model` `retrieval-backend` | `CitationVerification`（`:2464`） |

七枚的 `version` 一律 `OperatorVersion::new(1)`。

**闭环的**：第 5 行与第 7 行是给 P5a 的证据侧供给（§5）；第 6 行是 `Report` 的生产者（§3.2 末）。

**逐条的出处**：id 一栏是本设计定的（§4.6 给判据）；§330 步骤一栏是规范逐字；
`input_schema` / `output_schema` 是**本设计按 §4.5 的边表定的**（§330 不给端口）；
`determinism` / `side_effect_class` / backend 候选三栏是本设计的判定（规范一处未给，§4.4 给判据）。

## 4.3 §330 的七步与七枚算子的对应

**取法一句话**：**§330 的七个名字各落成一枚算子，链上的 `↓` 落成算子之间的一条 DATA 边**（§4.5）。
七枚与七名**逐名一一对应，不合并、不增补**。

**这里不是「把 `SourceSet` 与 `Search` 合成一枚」**：合成之后，§330 的七步里有一步没有落点，
而「七步各一枚」这句话就变成「六枚加一次说明」——**判据是「§330 的七个名字能不能逐名指到一枚算子」**，
不是「六枚够不够用」。同理，`Question` 不省略（它常被读成「链条的输入，不是一个步骤」）：
§330 把它列在 `↓` 之前的第一行，与另外六个同处一张 `SHOULD 拆成` 的清单里。

**据实记一处：七步之间的顺序约束本块不表达。** `Operator` 没有「前驱」字段（`:79-88` 八字段）
⇒ 「Question 必须在 Search 之前」这件事**在算子定义里写不出来**，它落在图的构造（第 3 层，切分 §四第 4 条）。
本块能表达的只有端口类型的相容性（§4.5），那是**弱于顺序**的约束：`Json → Json` 的三条边（§4.5 第 2、3、4 行）
在类型上无法与「把链倒过来接」相区分。**这条缺口与 §3.2 的代价同源**（都出自「不新增变体」），
并记入 §11。

## 4.4 逐枚的判定理由（只写需要解释的五处）

1. **`research-question` 的 `input_schema` 为空**：它的输入是 `Intent` 里的研究问题，而 `Intent` 不是 `Artifact`。
   **这里不是「给它一个 `Text` 占位输入」：那会让图上多一条没有生产者的 DATA 边**，
   而 §303 的 READY 判据要求「所有输入 Port 已有可用的 DATA 入边来源」
   （`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:382`）——零个输入在类型里可表达
   （`Vec<ArtifactType>` 允许空），一个无来源的输入不可表达。
2. **只有 `research-source-set` 是 `Deterministic`**：它是纯算法（去重、记录来源身份与取回时刻），
   不接触本进程之外的状态，故「同输入 + 同版本 ⇒ 同输出」成立。其余六枚都经模型或检索服务。
   **这正是它进 §6.1 的照片的那一枚。**
3. **`side_effect_class` 的判据（本设计定，规范只给一条后果）**：§307（`docs/spec/05-normative.md:2022-2033`）
   给的唯一后果是「非幂等 Effect MUST NOT 直接自动重试」，而 P1 的落地注记同此
   （`crates/continuum-operator/src/definition.rs:16`）。故本表按**重试安全性**取：
   - `Pure` = 该算子不接触本进程之外的状态（⇒ 同输入同版本必得同输出）：本块一枚（第 3 行）。
   - `Idempotent` = 该算子接触本进程之外的状态（模型服务、检索服务），故**同输入重跑可能给出不同结果**；
     但它**不改变任何外部状态**，重复执行没有累积后果，故重试是安全的：本块六枚。
   - `NonIdempotent` = 重复执行会留下外部后果（§307 点名的那一类）：本块**零枚**。
4. **没有一枚是 `NonIdempotent`，理由不是「研究任务都安全」**：判据是**有没有外部后果**。
   本块七枚只有读（检索是只读、模型调用是只读、写作只产出本仓制品），没有一枚发送、发布、扣费、删除远端
   ——`crates/continuum-effect/src/effect.rs:16-23` 的 `EffectType` 六臂（`SendEmail` / `PushBranch` /
   `Publish` / `DeleteRemote` / `Charge` / `Deploy`）与本块七枚**无一对得上**。
5. **backend 候选每枚只给一枚或两枚，且名字是本设计定的**：§245 `:756` 只给「同一个 Operator MAY 有多个 backend」
   与一个例子（`:761-764` 的 `Transcribe` 三候选），**规范全量里没有一处给过研究域的后端名**。
   故本表的多候选只出现在有结构理由的两处：第 7 行（引文核验既读被引来源、又判「来源是否支持该断言」，
   两种能力可以落在两个后端上）。其余各枚只给一枚——**多给一个是发明**。
   `builtin` 的语义在 §6.2 给出（那是本块唯一一处需要解释的后端名）。
   **`primary-model` / `retrieval-backend` 两个名字的含义（本设计定）**：前者是「本任务的主模型」，
   后者是「做检索的后端」。两者都不是某个具体产品名，理由同 §4.4 第 5 条的第一句——规范没给名字，
   给具体产品名会让一次后端替换变成一次枚举改动。

**`side_effect_class` 与 P5e 的取值口径不同，据实记在此处**：P5e 设计 §5.2 对经模型的算子取 `Pure`
（例如 `narrative-plan`：`NonDeterministic` + `Pure`），本表对经模型的算子取 `Idempotent`。
**同一个字段在两个块里按两套口径取值**是「同一件事两个词汇表」的一种，**收件人见 §10 第 6 条**。
本块**不改成 P5e 的口径**：本块的口径（按重试安全性、按是否接触进程外状态）有 §307 的后果可指，
而「经模型的算子算不算 `Pure`」按 P5e 的口径给不出判据。**这是本块的读法，不是规范断言。**

## 4.5 端口类型与连接边

**先定关系**：上游算子的 `output_schema` 与下游算子的 `input_schema` **相交非空**——这是 §239 `:654`
「运行时 MUST 拒绝不兼容连接」在**注册期**可检查的那一半（另一半需要 `Port` 与图，不在本块）。

**边表**（§9.2 第 (l) 条的语料）。每行给「下游算子 ← 输入的来源」，括号里是**该来源的 `output_schema`
与下游 `input_schema` 交集中的型**：

| 下游算子 | 输入的来源（交集中的型） | 依据 | 区分力 |
|---|---|---|---|
| `research-question` | 无输入（问题来自 `Intent`，§4.4 第 1 条） | §4.2 第 1 行 | —— |
| `research-search` | `research-question`（`Json`） | §330 `:2452`→`:2454` | **无**（两侧都是 `Json`） |
| `research-source-set` | `research-search`（`Json`） | §330 `:2454`→`:2456` | **无** |
| `research-evidence-extraction` | `research-source-set`（`Json`） | §330 `:2456`→`:2458` | **无** |
| `research-contradiction-check` | `research-evidence-extraction`（`Json`） | §330 `:2458`→`:2460` | **无** |
| `research-synthesis` | `research-evidence-extraction`（`Json`）、`research-contradiction-check`（`Json`） | §330 `:2460`→`:2462`（矛盾结论进合成）、§4.2 第 6 行 | **无** |
| `research-citation-verification` | `research-synthesis`（`Report`）、`research-evidence-extraction`（`Json`） | §330 `:2462`→`:2464`；§4.2 第 7 行 | 有（`Report`） |

**依据栏的读法**：写 §330 的，出处是**该节的链**（相邻两名之间的 `↓`）；写「§4.2 第 N 行」的，
出处是**该行的 `input_schema` 与全表 `output_schema` 的对应**（即本设计的判定，规范未给边）。

**「区分力」一栏是据实的自陈，不是形式**：七条边里有**六条**落在 `Json`↔`Json` 上（§3.2 的代价），
故 §239 的检查在这六条上没有区分力——它只能拒掉「把 `Report` 接到只收 `Json` 的端口」这一类错。
**这一栏存在的目的是让下一轮的复审不必自己重新推一遍「本表能查出什么」**。

**判据（§9.2 第 (l) 条）**：逐行断言下游 `input_schema` 的**每一个型**都能在该行列出的上游里找到，
且所引的那一枚的 `output_schema` 含该型。断言的对象是「交集中的型」，不是「两侧相等」
（`research-citation-verification` 的 `input_schema` 是 `[Report, Json]`，两个来源各供一枚，两侧不相等）。

**本表不是全图**：它只列 §330 的链。生成物回流（例如把矛盾结论接回检索）属装配方的图构造，不在本块的声明范围内（§1.2）。

## 4.6 算子 id 的取法

**id 一律以 `research-` 前缀 + §330 步骤名的 kebab 形式**（`Question` → `research-question`，
`EvidenceExtraction` → `research-evidence-extraction`，余同）。

**为什么加前缀**：`OperatorRegistry` 以 `(id, version)` 为键（`crates/continuum-operator/src/registry.rs:16`），
键是**全局**的。`question` / `search` / `synthesis` 这类词是领域中性的，别的块有理由也想用；
一次撞车的后果是**整个注册批次失败**（`OperatorError::Duplicate`，`:24-34`），不是「那一枚被跳过」。
本仓已有同形的先例（P5e 的 `media-import`）。

**为什么不做「方法 id 与算子 id 同名」**：§187 的 `research/` 里有 `contradiction-check` 与 `citation-verification`
两条（`docs/spec/04-method.md:135-136`），它们的字符串与本块第 5、7 行的 id 只差前缀。
**这里不是「为了对齐而把前缀去掉」**：`MethodId` 与 `OperatorId` 是两个 crate 的两个类型，
同名字符串不产生任何编译期联系，却会让 `realized_by`（`Vec<String>`，P5b 设计 §4.3）里的一个串
既可读作方法名又可读作算子名——**那条弱引用本来就没有编译期照片**（P5b 设计 §3.2 末），
再引入同名会让 §9.2 第 (f) 条那条用例的失败信息也分不清是哪一个。

---

# 5. 与 P5a 的接缝：只产出、只消费

## 5.1 报出两枚 `EvidenceType` 新臂

切分 §四第 1 条：四个算子块**只产出**、**只消费**，**不得**自定义证据类型、「充分」判据、完成判定。
P5a 设计 §4.2 末与 §14 第 8 条点名问本块：「`CitationVerification` 是否需要自己的臂，须向本块报出」。

**本块的答复：需要，报出两条臂。**

| 本块的产出算子 | 需要的臂 | 出处（逐字） |
|---|---|---|
| `research-contradiction-check` | `ContradictionCheck` | §330 `docs/spec/05-normative.md:2460` 的步骤名 |
| `research-citation-verification` | `CitationVerification` | §330 `docs/spec/05-normative.md:2464` 的步骤名 |

**逐臂给理由，按分量排**：

1. **现有的十一臂没有一枚覆盖这两件事。** 逐臂过一遍 P5a 设计 §4.2 的表：九枚工程类（`Test` / `Fuzz` /
   `Property` / `FormalProof` / `StaticAnalysis` / `Benchmark` / `VisualCheck` / `MetadataCheck` /
   `HumanConfirmation`）加 `Differential` / `Metamorphic`，**没有一枚的语义是「一组断言之间是否互相矛盾」
   或「一条引文是否被它引的来源支持」**。这两件事的对象是**来源的内容与断言的关系**，而十一臂的对象
   是**制品本身的可观察性质**（能跑、能编译、像素对不对、元数据对不对）。
2. **落进最接近的那一臂会让 P5a 的集合判定产生假满足。** P5a 设计 §6.2 的缺项判据做的是
   `VerificationProfile.required_evidence_types` 与产出的证据类型之间的**集合包含**。
   若把引文核验记成 `MetadataCheck`，则一份**要求 `MetadataCheck`** 的档案会被一次引文核验满足——
   而两者要判的东西不同（一个读制品的元数据，一个读被引来源的内容与断言的关系）。
   **失配方向是「以为齐了」**，正是 P5a 设计 §4.2 选封闭枚举要挡的那一侧。
3. **两条臂都必须进 `from_tool_result` 的「需要制品」那一支。** 两枚算子都读制品
   （第 5 行读抽取结果、第 7 行读 `Report` 与来源集合），故它们与 P5a 设计 §4.4 那张表的十型同侧，
   与 `HumanConfirmation` 不同侧。**这一条是给 P5a 的具体施工要求**：它那张表是**穷尽 `match`、无通配臂**
   （P5a 设计 §4.4 自陈「加一臂时这条规则编译期被强制重新决定」），故加这两臂时编译器会要求作出这个决定——
   本块的要求是「落在『需要制品』那一支」。

**这两臂的名字照录 §330**，理由同 P5a 设计 §4.2 对 §258 九臂的做法（照录规范名，不自造）。
**拼法与 `as_str` 的编码由 P5a 定**：它持有那个枚举。

**若 P5a 判不收新臂，本块的退路与代价（据实列出）**：退路只有「落进 `MetadataCheck`」这一条，
而代价就是上面第 2 条（假满足）。**故本块报出，不自行退让**；若协调者判这一条该退，须同时指出
「要求 `MetadataCheck` 的档案会被引文核验满足」这件事是可以接受的——本块不替它取这个决定。

**两臂落地后的性质据实记**：本块的这两枚证据**判不出极性**。「查出矛盾」与「未查出矛盾」在 `Evidence`
里没有字段可放（§258 的八字段 `docs/spec/05-normative.md:1076-1088` 无一是结果位），
故极性只能落在 `claim` 的文本与 `artifact_refs` 指向的那份 `Json` 制品里。
**这与 P5a 设计 §8.3 第 6 项（`MandatoryTestsPass` 不可判，因为证据无 outcome 字段）是同一处缺口的两半**：
那一半记的是「测试通过与否没有承载位」，本半记的是「研究域的两枚判定的极性没有承载位」。
**两处都留，互为指针**；缺的是同一步：「证据的结果位」。

## 5.2 产出的形状：一律 `Unattached`

```
/// 把一枚研究领域判定算子的产物转成 `Evidence`（§89）。
/// **唯一产生点是 P5a 的 `Evidence::from_tool_result`（P5a 设计 §4.4），本函数只是它的领域包装**：
/// 它固定的只有一件事——`subject` 取 `EvidenceSubject::Unattached`。
pub fn evidence_from_research_finding(
    id: EvidenceId,
    evidence_type: EvidenceType,
    claim: Claim,
    producer: EvidenceProducer,
    artifact_refs: Vec<ArtifactId>,
    strength: EvidenceStrength,
    scope: EvidenceScope,
) -> Result<Evidence, VerifyError>;
```

- **签名与 `from_tool_result` 逐项对齐**：上列七项就是 P5a 那个构造点的八个参数**减去**本函数固定的
  `subject`。`id` / `strength` / `scope` **必须由调用方给**，本块不代它取值（§2.2：证据的充分性判定不属本块）。
  **这里不是「只收五项的瘦包装」：那样 `id` / `strength` / `scope` 就没有供上的位置，
  而 `from_tool_result` 要求调用方给出它们——包装要么漏参编不过，要么就得就地造值，
  那就成了 `Evidence` 的第二个产生点。**
- **`subject` 取 `Unattached`**，不是 `Requirement(..)`：证据先于归属存在（P5a 设计 §4.3.1 的读法）。
  附带的一个后果是**本块不需要 `RequirementId`**（属 P4 的 `continuum-semantics`，该 crate 今天不存在），
  故 §2.1 的边表里没有它。
- **`producer` 由调用方以值给**，取 `EvidenceProducer::Node { node, backend }`。
  **这里不是「收 `NodeId` + `BackendId` 再就地拼出 `EvidenceProducer::Node`」：`NodeId` 定义在
  `continuum-graph`（`crates/continuum-graph/src/ids.rs:40`），`pub fn` 的签名里点名它就把该依赖从
  **dev 边**升成普通边**，而本块的生产代码不查图（§2.1 第 5 行：那条边是 dev 边）。
- **时刻不在本函数的签名里**：`Evidence` 没有时间字段（P5a 设计 §4.1 逐字段照 §258 的八字段，无一是时刻）。
  P5a 的 `occurred_at` 是 `evidence` **表**的一列，由写入那一行证据的落库方给；本块零迁移、不写那一行（§8）。
- **本函数不判任何东西**：只做形状转换。「充分」的判据不存在（OPEN-001，P5a 设计 §3 的处置）——
  本块**不绕开也不补**。
- **与本块的 `evidence_type` 参数有关的一处据实自陈**：本函数**拦不住调用方传错臂**——
  「第 5 行的算子只能用 `ContradictionCheck`」这条映射不在类型里。收成类型级要本块定义一枚「研究域的判定算子」
  类型（§2.2 表明本块零类型），或要 `EvidenceType` 收窄成两个值——**两者都不是本块能定的**（枚举归 P5a）。
  故这条映射由 §9.2 第 (i) 条那条用例把守，**它是运行期的，不是类型级的**。

**与 P5e 的同形包装据实记（本块不隐藏这处重复）**：P5e 设计 §9.2 有一枚 `evidence_from_media_output`，
形状与本函数相同（同样只是 `from_tool_result` 的领域包装、同样固定 `subject = Unattached`）。
**这不是「两处判据」**——两者都不判任何东西（不判充分、不判有效性），故不触切分 §四第 7 条。
**但它是同一件事的两份代码**，收件人见 §10 第 1 条（若协调者判该只留一处，共同部分应上移到 P5a 的一次改动里）。

## 5.3 消费面：本块不调用 P5a 的任何判定

`VerificationPolicy`、`coverage`、`select_verifier`、`judge_completion` **都不在本块的调用集合里**。
本块与 P5a 之间的实际数据流是**单向**的（本块产出 Evidence），反向的那一半（P5a 的判定 → 本块）
今天没有承载物，因为执行器未建（§9.5 第 1 条）。

**一条据实的不适**：`CitationVerification` 与 `ContradictionCheck` 在 §262 的五级序里该是第几级，
**本块不声明**。判据：P5a 设计 §7.2 的候选以**值**传入（`AvailableVerifier`），级别是装配方给的字段；
本块声明它等于替装配方定了一条判据，而 §262 的两级定义（确定性检查器 / 独立模型验证器）与本块两枚算子的对应
不是显然的（两枚都经模型，但也都能在无模型时退化成规则检查）。
**这里不是「P5e 声明了所以本块也声明」**：P5e 的声明已由它自己记为对账条目（P5e 设计 §13 第 5 条 (ii)），
本块不重复那一处的形状，只记「本块不声明」这一条事实与理由。

---

# 6. 切分 §四第 7 条：本仓已落地的判据与本块的消费面

## 6.1 复用判据已由 P1 落地，本块只消费

- §305（`docs/spec/05-normative.md:1986-1998`）给的是复用的三条件：
  `input_hash unchanged` / `operator_version unchanged` / `contract unaffected`。
- **它已经落地**：`crates/continuum-graph/src/reuse.rs` 的 `CacheKey`（`:9-12`）、`cache_key`（`:16-24`）
  与 `can_reuse`（`:29-47`）。`cache_key` 对 `NonDeterministic` 的算子返回 `None`（`:22`）——
  这是 ENG-002 的裁定，即「非确定算子的输出不写缓存键，因此永远不满足复用前提」。
  `can_reuse` 另加一条前置：`operator.determinism != Deterministic` ⇒ 直接 `false`（`:35-37`）。
- **故本块不得再定义第二个复用判据。** 本块的兑现**不是一个函数**，而是**一枚算子的 `determinism` 声明**：
  `research-source-set`（§4.2 第 3 行）是 `Deterministic`，故它过 `cache_key` 得 `Some(..)`。
  照片见 §9.2 第 (k) 条。

**本块声明 `Deterministic` 之前先核过一件事**：`cache_key` 读的是 `operator.determinism`（`:17`）
而**不看 backend**，即 P5e 设计 §5.4 记的那处结构缺口（`determinism` 挂在 `Operator` 上、backend 由 Router 选）。
本块的处置见下一节。

## 6.2 本块**不建**第二份「逐位可复现的 backend」名单

P5e 设计 §5.4 建了一份封闭名单（`local-fs` `ffmpeg` `ffmpeg-scene-detect` `ffmpeg-filtergraph` `ffprobe`
`pyscenedetect` `librosa` `essentia` `builtin`）与一条注册期规则：`Deterministic` 算子的候选逐个必须在该名单内。
**本块不建第二份**，理由即切分 §四第 7 条写的那一条：两份名单并存且各有用例、各自都绿时，
**失配是「一套说能复用、另一套说不能」**。两处都有 `builtin` 这个串——同一个后端名被两份名单各分一次档，
正是那条失效方向的具体形态。

**但本块需要那条规则的服务**：本块有一枚 `Deterministic` 算子。处置分两半：

1. **本块把约束收成一条可用例断言的事实**：`research-source-set` 的 `backend_candidates` **只有 `builtin` 一枚**，
   而 `builtin` 在本块的定义是「**在进程内、随本 crate 的版本一起冻结的算法**」，
   即 P5e 那条规则要排除的对象（硬件编码器、云模型、远程服务）**在本块的候选集合里不存在**。
   故本条不构成第二份名单，只构成一句关于一枚算子的、可断言的事实（§9.2 第 (j) 条）。
2. **这条事实的强度据实标明**：它是**本块的声明**，不是一条被强制的注册期规则——
   本块没有 `register` 期的名单检查（§4.1 的 `register_research_operators` 只查重）。
   若将来本块第二枚 `Deterministic` 算子带着外部后端候选出现，**今天没有任何东西会拦它**。
   **这是据实留的形状**，收件人见 §11。

**「这份名单的家在哪」本块不决定**：它若该只有一处，那处不是本块能给 P5e 定的。
记为 §10 第 2 条的对账条目（收件人：协调者 + P5e），并指出撞车面：P5c/P5f 若也声明 `Deterministic` 算子，
会撞同一处。切分 §四第 7 条本身**没有处置这一类判据**（它是以 `can_reuse` 为唯一实例写的，
而那一条判据是**跨域唯一**的；「逐位可复现的 backend」是**按域不可避免要多份**的），
这一条记为 §12 第 3 条。

## 6.3 本仓已落地的判据：清点与本块的关系

切分 §四第 7 条要求「先去看仓里已有哪些判据」。**实测命令**：`grep -rn "^pub fn " crates/`，
把结果按「**判定型**（返回 `bool` / 枚举 / 状态迁移结果）」筛一遍——落库读写（`save_*` / `load_*`）、
迁移装配（`*_migrations`）、编排（`apply_*` / `propagate_*`）不在「判据」的射程里。
筛后**与本块可能相撞的五行**如下（判据是「本块有没有理由自己再写一份」）：

| 已落地的判据 | 位置（实测） | 本块的关系 |
|---|---|---|
| `can_reuse` | `crates/continuum-graph/src/reuse.rs:29` | **消费**（§6.1 的照片）；不另建 |
| `cache_key` | 同文件 `:16` | 同上 |
| `is_candidate_backend`（§245 的「后端是否在候选内」） | `crates/continuum-graph/src/execution.rs:87` | **不用也不另建**：本块不解析后端（§2.2） |
| `compatible`（§239 的端口相容） | `crates/continuum-port/src/port.rs:94` | **不用也不另建**：§4.5 的边表只保证本块声明的端口取值正确，相容判定是 P1 的 |
| `decide_retry`（§307 的重试判定） | `crates/continuum-graph/src/failure.rs:106` | **不用也不另建**：本块的 `side_effect_class` 声明（§4.4 第 3 条）是它的输入，不是它的第二份 |

同一轮扫描里另有若干判定型函数（`crates/continuum-graph/src/state.rs:60` 的 `is_terminal`、
`crates/continuum-graph/src/scheduler.rs:28` 的 `select_runnable`、
`crates/continuum-events/src/audit.rs:112` 的 `verify_chain`、
`crates/continuum-policy/src/engine.rs:48` 的 `decide` 等），**它们与本块不相撞**：对象都不在本块的射程里
（节点状态、可运行集合、审计链、策略上下文）。**这一句写在此处是为了让复审不必重跑那一次扫描**，
不是声称上表五行是仓里判定函数的全部——**那一次扫描的筛法是上面写的那一条，换一个筛法会得到另一张表**。

**本块的 `research-citation-verification` 不是「第二份判据」**：它判的是「一条引文是否支持它所属的断言」，
而这不是上表任何一行的对象，也不是同一轮扫描里任何一枚判定函数的对象。
它的判定在**算子实现体里**（那属执行，不在本设计），不构成一个供别的块复用的判据函数。

---

# 7. 与 P5b 的接缝：`bind`

P5b 设计第六节要求四块**只经 `bind` 填 `realized_by`**，且 `research/` 的四条逐条非空
（P5b 设计 §4.5：`has_counterpart(Research) == true` ⇒ 空即「漏 bind」）。本块填出：

| `MethodId`（§187，`docs/spec/04-method.md:133-136`） | `realized_by`（本块的算子 id） | 理由 |
|---|---|---|---|
| `retrieval` | `["research-search", "research-source-set"]` | §330 的 `Search`（`:2454`）与 `SourceSet`（`:2456`）两步合起来就是「取得来源并把它整理成一个集合」 |
| `evidence-analysis` | `["research-evidence-extraction"]` | §330 的 `EvidenceExtraction`（`:2458`）：从来源里抽出可据的断言 |
| `contradiction-check` | `["research-contradiction-check"]` | §330 的 `ContradictionCheck`（`:2460`），与 §187 的方法名同名同物 |
| `citation-verification` | `["research-citation-verification"]` | §330 的 `CitationVerification`（`:2464`），同上 |

**`bind` 的参数是文本**（P5b 设计 §4.3 的 `realized_by: Vec<String>`），故上表的值是字符串，
**本块无法在编译期核对它们与 `all_operators()` 的 id 一致**（P5b 设计 §3.2 末已把这条弱引用的无照片写清）。
本块的判据是**运行期的一条用例**（§9.2 第 (f) 条）：对每一条 `realized_by` 里的每个串，
`OperatorRegistry::resolve` 必须成功——这条用例**在本块的 crate 里闭得上**，因为本块同时持有
`all_operators()` 与 `MethodRegistry`（**这是 P5b 所说「消费块才能核」的那个消费块之一**）。

调用 `bind` 的**具名入口**：

```
/// 把上表四条 `realized_by` 填进方法库（P5b 设计第六节：每块对其领域目录里的**每一条已 seed 的 id**
/// 调用一次 `bind(id, &[...])`）。`research/` 四条逐条非空（P5b 设计 §4.5）。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
pub fn bind_research_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;
```

**错误类型不并入任何本块的类型**：本块没有自己的错误类型（§2.2），失败直接透出 `MethodError`
（`String` 弱引用的失败与「方法不存在」是同一件事，判定在 P5b）。
**本函数不做二次登记**：不 `register` 方法条目（P5b 设计第六节：四块**不得**自造 `MethodEntry`），
`research/` 的四条 id 由 P5b 的 `seeded()` 给出，本块只把这四条「每一条已 seed 的 id」逐条 `bind` 一次。

**§330 的两步没有目录可填，本条据实留（不是漏 bind）**：`research-question`（`:2452`）与
`research-synthesis`（`:2462`）**不在任何方法的 `realized_by` 里**——§187 的 `research/` 只有四条
（`:133-136`），而 §330 有七个步骤。`Synthesis` 是链条上产出 `Report` 的那一步，而 §187 没有对应的方法名。
**§330 与 §187 的 `research/` 不是同一张表**：前者是链上的步骤，后者是方法名。
本设计**不给 §187 补条目**（P5b 设计 §6：补一个规范无来源的方法名会动到 P5b 的 R3/R6/R9 裁决），
并把这一条记为 §11 的缺口——**P5e 那条对偶的形状（§33 的七个预处理算子没有目录可填）是「有步骤无方法」，
本条是同一形状**。

---

# 8. 持久化：**本块零迁移**

## 8.1 结论

**本块不建表、不建迁移、不写事件。** 逐项给理由：

| 本块的产物 | 为什么不落库 |
|---|---|
| 7 枚 `Operator` | `Operator` 今天**没有持久化路径**：`continuum-operator` 无 `persist.rs`（实测该 crate 的 `src/` 只有 `definition.rs` `registry.rs` `lib.rs`），注册是**装配期在内存里**做的事（`OperatorRegistry` 的 `entries: HashMap`，`registry.rs:16`）。本块照此，不新增存储 |
| 本块产出的 `Evidence` | 落进 P5a 的表（P5a 设计 §10.2 的四张表，号段 `130`）。本块不写那一行（§5.2 末） |
| 来源集合 / 抽取结果 / 矛盾结论 / 引文核验结果（四枚 `Json` 制品） | 落进**既有的** `artifact` 表（`crates/continuum-artifact/src/persist.rs:9` 的 `p1_artifact` 迁移）与 `BlobStore`；`artifact_type` 是 `TEXT NOT NULL` 且**无 CHECK 约束**（同文件 `:11`），既有取值 `json` 已够 |
| 合成产出的 `Report` | 同上（`report` 这一串要等 P5e 的枚举改动落地，`crates/continuum-artifact/src/artifact.rs` 的 `parser` 是唯一产生点） |
| §131 的复用凭据 | `CacheKey`（`crates/continuum-graph/src/reuse.rs:9-12`）今天**也没有存储**，它的存储属 §305 的落点即 P1。本块不替它建表（§6.1） |

## 8.2 实测占用表（判据在切分 §二第 5 条）

**实测命令**：`grep -rn "Migration::new(" crates/`，逐处读**调用的第一个实参**（号在参数表的次行或同行）。
实测结果（工作树 `p5d`，2026-10-09）：

| 号 | 位置 | 是否进运行时装配链 |
|---|---|---|
| `1` `2` | `crates/continuum-persist/src/db.rs:30` / `:47` | 是（P0 内建，`builtin_migrations()`） |
| `10` | `crates/continuum-artifact/src/persist.rs:9` | 是 |
| `20` | `crates/continuum-graph/src/persist.rs:15` | 是 |
| `30` | `crates/continuum-workspace/src/persist.rs:22` | 是 |
| `40` | `crates/continuum-effect/src/persist.rs:28` | 是 |
| `41` | `crates/continuum-policy/src/persist.rs:31` | 是 |
| `50` | `crates/continuum-capability/src/persist.rs:54` | 是 |
| `80` | `crates/continuum-model-registry/src/persist.rs:107` | 是 |
| `60` | `crates/continuum-persist/src/bin/crash-writer.rs:17`、`crates/continuum-persist/tests/crash_atomicity.rs:9` | **否**（各自自建的库） |
| `50` | `crates/continuum-persist/tests/recovery.rs:66` | **否**（探针库） |
| `100` `101` `102` | `crates/continuum-persist/tests/migrations.rs:45` / `:68` / `:69` | **否**（测试夹具） |

「是」的八处即 `crates/continuum-runtime/src/main.rs:98-107` 的 `runtime_migrations()` 的装配集合
（`:100-106` 逐行追加七个 crate 的迁移，加上 `:99` 的 `builtin_migrations()`）。

**实测空档（十位档）**：上表「是」的八处取值为 `1 2 10 20 30 40 41 50 80`，
**`130`–`199` 全部落在装配链之外**（含 P5a 已声明占用的 `130`——本块实测它今天仍未被任何一处占用，
P5a 的占用是**声明**，其迁移随 P5a 的实现落地）。测试夹具占的 `100/101/102` 不进装配链。

## 8.3 本块的号段处置

**本块不占档**——零迁移，故没有号可占。**这是本设计的一处刻意决定，不是漏项**，两条理由：

1. 按本仓的既有取法，「预留一个没有表要建的编号就是留一条死迁移」
   （`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:482-485`）。
2. **P5e 设计 §11.3 给过一份次序建议，其中把 P5d 列为 `160`。本块零迁移，故不占 `160`——该号仍空。**
   据实写明这一句，是因为**「我没占它」不等于「它没人占」**（切分 §二第 5 条的判据）：
   本块只能自证「本块不需要号」，不能自证「别的块没有声明 `160`」。

**若 P5d 的实现阶段确需落库**（例如把来源集合单列一表），那一刻取号即可，判据是**未占用**。
**收件人：协调者**（六块与 P6 的档由它统一划，切分 §二第 5 条）。**本节不另开对账条目**：
本块不参与号段划分，开条目是替别人的事记账。

---

# 9. 测试策略与照片

## 9.1 清单与枚举的守卫

- `all_operators()`：`len() == 7`（返回类型已写死 7，再断言一次是为了让「改了返回类型但没改断言」也红），
  且 `id` **逐枚不同**（两两比较，不是只查总数）。
- 七枚的 `version` 均为 `1`，且**逐枚断言 `OperatorVersion::new(1)`**。
- 本块的两处映射（`determinism` 与 `side_effect_class` 的取值、`research/` 的四条 `realized_by`）
  **各用手写期望**，不从被测清单里取（§9.2 的下方那条预防）。

## 9.2 判定侧：正例 + 反例成对（缺一条即不算钉住）

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| a | 七枚算子的 `id` 逐枚断言（手写七个串，按 §4.2 的行序） | 清单的正例；**语料手写**（见下方预防） |
| b | 七枚的 `(input_schema, output_schema, determinism, side_effect_class, backend 候选)` **逐格**断言（手写期望表，不由 `all_operators()` 自身推出） | §4.2 全表；**七行逐行，不是抽样** |
| c | 以 `&all_operators()` 调 `register_research_operators` 注册 7 枚后，逐枚 `resolve` 成功 | 注册的正例 |
| d | 同一批注册两次 ⇒ `Err(OperatorError::Duplicate { .. })`（**不是**静默覆盖），**且注册表内容与第一次之后逐枚相同** | 注册的反例；「表未变」那一半钉的是**先核后写**（§4.1 的失效方向） |
| e | 一批里含两枚同 `(id, version)` 的算子（批内自撞）⇒ `Err`，**且注册表内容与调用前逐枚相同**（不留半注册） | §4.1「先核后写」的**批内**那一侧 |
| f | 四条 `research/` 方法的 `realized_by` **逐条非空**，且每个串 `OperatorRegistry::resolve` 成功（先经 `bind_research_methods` 填入，§7） | §7（P5b 设计 §4.5 交办的判据）+ 弱引用的运行期补件 |
| g | `bind_research_methods` 之后，`select(MethodDomain::Research)` 返回**四条**（数目手写） | §7 的覆盖；**这一条钉的是「四条的 id 都被 bind 过」而不是「四条都存在」** |
| h | 对未 seed 的 `MethodId` 调 `bind` ⇒ `MethodError::NotFound` | §7 的反例（错透了 `MethodError`，本块没有自己的错误类型） |
| i | 两枚判定算子各产一条证据：`evidence_from_research_finding` 的返回的 `subject == Unattached`、`evidence_type` 与 §5.1 的两臂逐枚对应 | §5.2 的固定项；**两侧都要**：正例是这两枚，反例是「传 `MetadataCheck` 也能建成」（说明本函数不拦臂，§5.2 末的自陈不是空话） |
| j | `research-source-set` 的 `backend_candidates == ["builtin"]`（**逐元素断言**，不是「非空」） | §6.2 第 1 条那半句可断言的事实 |
| k | `cache_key(&research_source_set, h)` 为 `Some`，且四条件齐时 `can_reuse` 为真；**对另外六枚逐枚**断言 `cache_key(..)` 为 `None` | §6.1 的六行**逐行**（不是抽样）：一枚 `Some` + 六枚 `None` |
| l | §4.5 边表**逐行**：下游 `input_schema` 的每一型都能在该行列出的上游算子里找到，且那一枚的 `output_schema` 含该型（**交集中的型**，不是「两侧相等」） | §239 的注册期一半（§4.5） |
| m | 七枚算子的每一个端口类型都在**既有枚举**里可解析（`ArtifactType::parse(..).is_some()` 对每个 `as_str`） | §3.2 的「需要的型全部是既有的」这一断言 |

**一条预防**：第 (a)(b)(k) 三条的期望清单**必须手写**。若 (b) 写成遍历 `all_operators()` 再自比，
它测的是 Rust 的相等性；若 (a) 写成 `all_operators().len()`，它测的是 `len()`；
若 (k) 写成「非 `Deterministic` 的枚数等于 6」，它测的是一次减法。
**语料从被测清单里取就是恒真的假照片**（本仓已有此判据）。

## 9.3 变异预告（谁红）

四档，每档预告**哪些用例红、哪些不该红**（预告写反会让人去查不存在的问题）：

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`research-source-set` 的 `determinism` 改成 `NonDeterministic` | k 的第一枚（`Some` → `None`） | j（候选列表与 determinism 无关）、其余各条 |
| **取反**：`research-search` 的 `determinism` 改成 `Deterministic` | k 的第二组（该枚由 `None` 变 `Some`） | j、l、m（它们不看 determinism） |
| **移除**：`register_research_operators` 的「先核后写」删掉（边核边写） | d 的第二个子例、e 的第二个子例（表内容与调用前不同） | c（合法批次两侧都绿） |
| **移除**：`bind` 的 `retrieval` 那一条 | f 对 `research-search` / `research-source-set` 两个串的解析（它们从 `realized_by` 里消失）、g（`select` 那一条的 `realized_by` 变空） | 其余三条 `bind`、a–e、i–m |
| **放宽**：把 `research-question` 的 `input_schema` 改成 `[Text]` | l 的第 1 行（无输入的来源不存在了）、m 仍绿（`Text` 是既有型） | 其余各行 |
| **收紧**：把 `research-citation-verification` 的 `input_schema` 的 `Report` 删掉（只留 `Json`） | b（该行的端口断言）、**l 的第 7 行会变绿**（`Json` 在上游找得到）——**故 l 单独不足以钉住这一格**，b 与 l 要一起看 | j、k |
| **取反**：`evidence_from_research_finding` 的 `subject` 从 `Unattached` 改成 `Requirement(..)` | i 的第一个子例 | 其余全部 |
| **等价变异**：把 `research-source-set` 的候选从 `["builtin"]` 改成 `["Builtin"]`（大小写不同） | j 红（逐元素断言），**但这不是等价变异体**：`BackendId` 是大小写敏感的 `String`（`crates/continuum-operator/src/definition.rs:66`），改串即改身份 | —— |
| **等价变异**：把 `research-question` 的 `backend_candidates` 从 `["primary-model"]` 改成 `["primary_model"]` | **全绿**——这是一枚**等价变异体**（两个串在表里等价，都是本设计定的名字，无外部消费者）。要打红它必须换变异体：把它改成与另一枚算子共用的串以外的任意值，那时 b 红 | —— |

## 9.4 结构层的照片（`trybuild`）

本仓已有 `trybuild` 的既有做法（`crates/continuum-connector/Cargo.toml:53` 与其 `tests/type_level.rs`
的 `compile_fail`）。**本块用一条**：

1. `Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现）在 `continuum-research` 里编不过
   ⇒ 在本块的 crate 里**造不出** `Evidence`（字段私有、无 `Default`／`From`），绕过构造点在类型上写不出来。
   这是 P5a 设计 §4.4「唯一产生点」那条结构事实在本块的**使用侧**照片。
   **这一条证不到「产出只能经 `evidence_from_research_finding`」**：`Evidence::from_tool_result` 是 `pub`，
   本块的生产代码可以直接调它——仍是同一个构造点，只是不经过本块的包装（§5.2）。

**只有一条的理由**：本块没有第二处结构约束可拍。本块零类型（§2.2）⇒ 没有「某个领域类型不可构造」这类照片；
`subject = Unattached` 的固定在**签名**里（`subject` 不是参数），不靠编译失败来证（§5.2 末的自陈已把它的强度写明）。
**这里不是「照 P5e 的照片清单抄两条」**：P5e 的第二条依赖「P5a 不 re-export `RequirementId`」这个对别人块的假设，
本块不重复那条依赖。

## 9.5 拍不出照片的地方

1. **端到端（一个研究 Intent 从 `Queued` 走到某处）**：执行器（第 3 层）未建，
   且切分 §四第 4 条禁止 P5 自建执行路径。本块的全部照片都是**单元与集成层**。
2. **算子的实现体**：本块的七枚是 `Operator` **值**（八个元数据字段），它们的**执行**不在本层（§1.2）。
   故「检索真的返回了什么」「合成真的写了什么」在本块的射程之外——照片覆盖的是**清单与映射**，
   不是**行为**。这一条是 P5 四个算子块共同的形态。
3. **§305 的复用真的省下了一次重算**：`can_reuse` 今天无生产调用方，`CacheKey` 无存储（§8.1）。
   故只能拍到「判据的返回值」（k 条）。
4. **§330 的链顺序真的被遵守**：`Operator` 无前驱字段（§4.3 末），顺序落在图构造上。
   故拍不到「Question 在 Search 之前」。
5. **`SourceSet` 与抽取结果的内容**：四枚 `Json` 制品的 schema 未定义（§11 第 2 条），
   故拍不到「一份来源集合建得出、读得回」。

---

# 10. 与其余各块的对账条目（逐条具名）

以下每条都是**本设计假设了别的块的某枚形状**或**发现某处无归属**之处。

1. **待与 P5e 与 P5a 对账（两枚同形的证据包装）**：§5.2 的 `evidence_from_research_finding` 与
   P5e 设计 §9.2 的同类函数形状相同（都只是 `from_tool_result` 的领域包装，都固定 `subject = Unattached`）。
   两者都不判任何东西，故不触切分 §四第 7 条；**但它们是同一件事的两份代码**。
   若协调者判该只留一处，方案是：P5a 在自己的 crate 里补一个固定 `subject` 的构造点（一次改动、一条 `pub` 面），
   两块的包装随之删掉。**本块不替 P5a 决定它该不该多一条 `pub` 面。**
2. **待与协调者与 P5e 对账（「逐位可复现的 backend」名单的家）**：§6.2。撞车面是 P5c/P5f 与 P5e 三处。
3. **待与 P5a 对账（两条 `EvidenceType` 新臂）**：§5.1。要求两条一起落（一次改动，同 `ArtifactType`／P5e 的约定），
   且两条都进 `from_tool_result` 的「需要制品」那一支。
4. **待与 P5a 对账（证据的结果位）**：§5.1 末。本块两枚证据的极性没有承载位，
   与 P5a 设计 §8.3 第 6 项是同一处缺口的两半，两处互指。
5. **待与 P5a 对账（两枚判定算子的 §262 级别）**：§5.3。本块**不声明**级别；
   若协调者判该声明，须给出「级别由谁持有」的裁定（P5e 设计 §13 第 5 条 (ii) 记的是同一处未决）。
6. **待与 P5e 与协调者对账（`side_effect_class` 的口径）**：§4.4 末。同一个字段在两个块里按两套口径取值
   （经模型的算子在 P5e 的表里是 `Pure`，在本块是 `Idempotent`）。**同一个字段有两套口径**是
   「同一件事两个词汇表」的一种；本块给出自己的口径与它的依据（§307 的重试后果），并**请协调者裁定取哪一套**。
7. **待与 P5b 对账**：§7 的四条 `bind` 是本块填的；`realized_by` 的文本弱引用**没有编译期照片**
   （P5b 设计 §3.2 末），本块的补件是 §9.2 第 (f) 条那条运行期用例。
   另：§7 末的「`research-question` 与 `research-synthesis` 无目录可填」是**据实留的形状**，不是漏 bind。
8. **待与 P5e 对账（`Report` 的生产者）**：§3.2 末。P5e 设计 §5.2 记「`Report` 在媒体块没有生产者」，
   本块的 `research-synthesis` 是它的第一个生产者。两处互指。
9. **待与 P5e 对账（`ALL_OPERATORS` 的 const 写法）**：§4.1.1 的实测——`Operator` 的清单写不成 `const`，
   而 P5e 设计 §5.1 把它写成 `pub const ALL_OPERATORS: [Operator; 17]`，且 §12.2 的第 (f)(h) 条与 §12.3
   都以它为语料。**本条的收件人是 P5e**（它自己的实现任务会当场撞上），本块只提供实测与判据。
10. **待与协调者与装配方对账（本块的条目今天没有生产调用方）**：`all_operators` / `register_research_operators`
    （§4.1）、`bind_research_methods`（§7）、`evidence_from_research_finding`（§5.2），四条。
   **这不是本块的缺口，是第 3 层的**（切分 §四第 4 条禁止 P5 自建执行路径；拍照的限制见 §9.5）。
   **与切分 §八「算子解析的落点仍无人认领」那一条同源**，本块不把那一处各自再记一次。

---

# 11. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **`SourceSet` 未收成一型，代价是三条边失去区分力**（§3.2、§4.5）。
   **缺的是「研究域的来源集合该不该有自己的 `ArtifactType`」这一步的裁定。收件人：协调者 + P5e。**
2. **四枚 `Json` 制品的 schema 未定义**（§3.2、§9.5 第 5 条）：来源集合的条目字段、抽取结果里
   「一条断言 + 它的来源引用」的形态、矛盾结论的形态、引文核验结论的形态，**规范一处未给**。
   **缺的是「这四份载荷各写什么字段」这一步。收件人：规范维护者。**
   **与 P5e 设计 §14 第 4 条（Timeline 六字段无 schema）同形**，两处都是「名字有、字段没有」，两处互指。
3. **§330 的七步之间的顺序约束本块表达不出**（§4.3 末）：`Operator` 无前驱字段，链的顺序落在图构造上。
   **缺的是「链的顺序由谁表达、在哪一层检查」这一步。收件人：第 3 层的执行器 + 协调者。**
   同一处还挂着一个**规范侧的未决**：§330 `:2449` 的措辞是 **SHOULD**，而《总纲》§8.4 `:1453`
   逐字写「研究任务拆成**固定**链条（§330）」——**「固定」是不是意味着七步穷尽、有序、不可跳步**，
   规范没给。本设计按 §330（规范正文）读：七枚算子是**可注册的算子**，不是一条不可变更的编排。
   见 §12 第 2 条。
4. **本块没有 `Deterministic` 算子的注册期检查**（§6.2）：本块只有一枚 `Deterministic` 算子，
   它的事实由一条用例把守，**没有规则**。**缺的是「这份名单的家在哪」这一步的裁定。收件人：协调者 + P5e。**
5. **`research-question` 与 `research-synthesis` 在 §187 的 `research/` 里没有对应方法**（§7 末）。
   **缺的是「§330 的 `Synthesis` 该不该有一条方法名」这一步。收件人：规范维护者 + 协调者。**
6. **两枚判定算子的 §262 级别未声明**（§5.3）。**缺的是「verifier 候选的级别由谁持有」这一步。收件人：P5a + 装配方。**
7. **本块的 backend 名（`primary-model` / `retrieval-backend` / `builtin`）与 `side_effect_class` 的取值口径
   都是本设计定的**（§4.4 第 3、5 条），规范无来源。**缺的是「研究域的后端分类」这一步。收件人：规范维护者。**
8. **本块的四条 `pub` 项今天都没有生产调用方**（§10 第 10 条）。**这不是本块的缺口，是第 3 层的。**

---

# 12. 在切分文档、P5e 的设计与本轮派单里发现的错或缺口（**只记，不改**）

逐条给出实测。**本设计不改切分文档，也不改 P5e 的设计**（切分 §七 与协调纪律）。

1. **⚠️ 切分 §八 把 `SourceSet` 记为「未定义」而没有指出处置方。** 该条逐字是
   「§330 的 `SourceSet` 是否算 Artifact 类型未定义」（`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`
   的 §八「`ArtifactType` 的 P5 清单尚无权威来源」那一条）。**它给了缺口，没给落点**。
   实际给出处置义务的是 P5e 设计 §3.4（「若 P5d 需要它，按切分 §四第 2 条报出」）与本轮的派单。
   **判据**：**一句「X 未定义」若不带落点，六个块都会读成「不是我的事」**——
   本轮的接口冻结里恰好有一条（§四第 2 条）能承接它，但那一句里没有指向它的指针。
2. **⚠️ 切分 §五 P5d 行把 §330 与《总纲》§8.4 并列为主要规范依据，而两处的措辞强弱不同。**
   实测：§330 `docs/spec/05-normative.md:2449` 逐字「研究任务 SHOULD 拆成：」；
   《总纲》`docs/01-总纲.md:1453` 逐字「研究任务拆成**固定**链条（§330）：」。
   两处给的是同一条链的同一串名字（`:2452-2464` 与 `:1456-1457` 逐名相同），**但「SHOULD 拆成」与「固定链条」
   不是同一条约束**：前者允许别的拆法，后者读起来是「只有这一条」。切分把两处并列而未定以哪一处为准。
   **本设计取 §330**（规范正文），并把「七步是否穷尽、有序、不可跳步」记为 §11 第 3 条。
   **判据**：**列两处依据时，要核它们给的是不是同一件事**——同一串名字不等于同一条约束。
3. **⚠️ 切分 §四第 7 条没有处置「按域不可避免要多份」的判据。** 该条的正文逐字把它自己的实例限定为
   「凡本仓**已落地**的判据」，并举 `can_reuse` 为第一枚实例——那一条判据的性质是**跨域唯一**
   （任何域的复用都走同一个 `can_reuse`）。而 P5e 设计 §5.4 新造的那一条（「逐位可复现的 backend」封闭名单）
   是**按域分份**的：媒体域的后端名与研究域的后端名不可能合成一张表，**但两域可以出现同一个后端名**
   （实测：P5e 的名单与 §4.2 第 3 行都用 `builtin`，P5e 的 `transcribe` 候选与 §4.2 第 2 行的角色名也可能重合）。
   ⇒ **该条的两份判据并存时，失效方向与 §四第 7 条写的一模一样**，而 §四第 7 条的文字（「已落地」＋ 唯一实例）
   读起来不覆盖这种情形。本块据它办（§6.2：不建第二份），**并把这一格缺口报出**。
   **判据**：**一条横切约束的实例若都是「跨域唯一」型的，它挡不住「按域分份」型**——
   列约束时要问「反例长什么样」，而不是只列正例。
4. **⚠️ P5e 设计 §5.1 的 `pub const ALL_OPERATORS: [Operator; 17]` 编译不过**（§4.1.1 的两条实测理由：
   `String` 与**非空** `Vec` 都不可在 `const` 上下文构造）。**同节的理由句也随之不成立**——
   「把「17」写成类型的一部分（`[Operator; 17]`），使清单个数的改动**编译期可见**」这条性质，
   `const` 给不出，但**返回类型 `[Operator; 7]` 给得出**（§4.1.1）。
   **影响面（实测 P5e 设计里以它为语料的处所）**：§5.1 的常量声明与它的理由段、
   §12.1 的 `ALL_OPERATORS` 守卫、§12.2 的第 (f) 与 (h) 条、§12.3 的「放宽：`ALL_OPERATORS` 少注册一枚（16）」。
   **收件人：P5e**。
   **判据**：**引用一条既有约定（「枚举/清单带 `ALL` 常量」）时，要连它的成立条件一起搬**——
   本仓的三处 `ALL`（`ArtifactType` `crates/continuum-artifact/src/artifact.rs:28`、
   `EffectType` `crates/continuum-effect/src/effect.rs:31`、`PrivacyClass` 同 artifact 文件 `:100`）
   **都是无字段枚举**，元素是常量；`Operator` 不是。
5. **⚠️ 本轮派单里的一处引用不存在**：派单写「照 P5e 设计 §16 第 5b/5c 条的体例（发明判据必须显式标出并写落点）」，
   而 P5e 的设计实测**只有 15 节**（`grep -n "^# " docs/superpowers/specs/2026-10-09-p5e-media-domain-operators-design.md`
   末节是 `# 15.`），**没有 §16**。本设计据「发明判据必须显式标出并写落点」这条**要求**办
   （本块的发明项集中在 §4.4 第 3、5 条、§6.2 与 §11 第 7 条），不按一个找不到的节号办。
   **收件人：协调者**（派单层的引用也要实测）。
6. **✔ 已复测为正确的三处**（记在此处是为了让复审不必重做）：
   (i) 切分 §五 P5d 行的三个依据——§330 在 `docs/spec/05-normative.md:2447`、
   「工程 §8.1 第 12 行」实测是 `docs/02-工程.md:496`（表头 `:483`，`:485` 是第 1 行，故 `:496` 是第 12 行）、
   「总纲 §8.4」实测是 `docs/01-总纲.md:1451`。
   (ii) §187 的 `research/` 四条——`docs/spec/04-method.md:133-136` 逐行 `retrieval` / `evidence-analysis` /
   `contradiction-check` / `citation-verification`，与切分 §四第 3 条列的四个目录名一致。
   (iii) §330 的七个名字与行号——`:2452` `Question` / `:2454` `Search` / `:2456` `SourceSet` /
   `:2458` `EvidenceExtraction` / `:2460` `ContradictionCheck` / `:2462` `Synthesis` / `:2464` `CitationVerification`。
