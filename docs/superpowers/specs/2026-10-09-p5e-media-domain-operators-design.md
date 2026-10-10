# P5e 媒体领域算子 — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md` §五 的 P5e 行的原话，逐字照用；引文里的「下方的补记二」指该行下方那条补记，即 §131 只列派生产物的种类、复用条件在 §305）：

> 把 §32 的剪辑链与 §328 的七种标准 Artifact 落成 Operator 集与 Artifact 类型，含 Timeline 的 Artifact 形态（§329）、
> Derived Artifact 在原素材未变时的复用（§305，见下方的补记二）、以及生成内容必须单独授权（§34）；
> **并一次性落地 `ArtifactType` 的 P5 扩展清单**（§四第 2 条）

**本块拥有 / 产出的接口**（同行的第二栏）：本领域的 Operator 集；`ArtifactType` 的 P5 新变体（**唯一落地点**）；
`Checkpointable` 的错误类型（切分 §二第 4 条）。

**规范依据**：§328 §329 §32 §33 §34 §131 §308 §239；《工程》§8.1 第 13 行（`docs/02-工程.md:497`）、§8.3、§8.5；
《总纲》§8.5（`docs/01-总纲.md:1460`）。

**本文件只写 P5e。** 其余五块（P5a 验证与证据、P5b 执行方法库、P5c/P5d/P5f 三个领域算子）各有其设计，
本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §13 写成一条**具名的对账条目**，
不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（判据与订正原文见 `docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p5e`）里当场 `grep -n` / `awk` 实测的，
未照抄任何一份别的文档（含切分文档）的行号。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| `ArtifactType` 的 P5 扩展清单（11 枚新变体） | 切分 §四第 2 条 | §3 | 全量（清单 + 逐条出处 + 一次改动的七处落点） |
| `Checkpointable` 的错误类型 | 切分 §二第 4 条 | §4 | 全量（形状 + 落点 + 第一枚实现 + 判据） |
| 媒体领域的 Operator 集（17 枚） | 切分 §五 P5e 行；§32 §328 §33 | §5 | 全量（端口类型、determinism、side_effect_class、backend 候选） |
| Derived Artifact 的复用（§131） | 切分 §五 P5e 行；§8.3 的「媒体衍生 Artifact」行 | §6 | **不建第二份判据**；本块的兑现是让本领域算子满足 P1 已落地的判据的前提 |
| 生成内容必须单独授权（§34） | 切分 §五 P5e 行 | §7 | 授权臂全量；**调用点不在本块**（切分 §四第 4 条） |
| Timeline 的 Artifact 形态（§329） | 切分 §五 P5e 行 | §8 | 形状落点全量；**字段级 schema 不定义**（缺口见 §14 第 4 条） |
| 只产出 Evidence、只消费判定 | 切分 §四第 1 条 | §9 | 产出**直接**走到 P5a 的 `Evidence::from_tool_result`（唯一构造点），**不写域包装**（§9.2；跨块口径见 P5a 设计 §4.4）；不新建证据类型 |
| 与 P5b 方法库的接缝 | P5b 设计第六节（`docs/superpowers/specs/2026-10-08-p5b-execution-method-library-design.md:371-390`） | §10 | 四条 `video/` 方法的 `realized_by` 逐条填出 |
| 迁移号段 | 切分 §二第 5 条 | §11 | **本块零迁移**；附实测空档表 |

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| Verifier 的选择、隔离输入、完成判定、`Evidence` 类型、`VerificationPolicy` | P5a（切分 §四第 1 条）。本块**不得**定义证据类型、自定义「充分」判据、自定义完成判定 |
| 方法（Method）的登记形态、`MethodRegistry`、`MethodEntry`、`MethodDomain` | P5b（切分 §四第 3 条：在 P5b 过审前不得自造）。本块只调它的 `bind` |
| 节点状态迁移、`Queued → Running`、`OperatorRegistry::resolve` 的调用点、算子的执行 | 第 3 层（切分 §四第 4 条）。本块只**注册**算子 |
| `execution_policy` 的判定 | P4（切分 §四第 5 条）。本块只碰 `verification_policy`（那是 P5a 的落点，本块不碰） |
| `Image` 的两个图像算子（完整生成、局部修改）、mask 外像素比较器 | P5f。`Image` 这一**型**由本块一次落地（§3），两个**算子**不属本块 |
| 3D 链（`Reconstruct3D` / `RenderScene`）、`3d/` 目录的对家 | **本轮无解**（切分 §八 的「`3d/` 在四个具名算子里没有对家」那一条）。本块**不注册**这两个算子；`Scene` 这一**型**的处理见 §3.3 |
| `SourceSet` 这一型 | 本设计**不收**，理由见 §3.4 |
| 复用判据本身（缓存键、`can_reuse`） | P1 已落地（`crates/continuum-graph/src/reuse.rs:29`）。本块**不建第二份**（§6） |
| 迁移 / 表 | 本块**零表**（§11） |
| 执行算子时的证据装配上下文（面向哪条 Requirement） | 装配方与 P5a。本块的算子产出 `EvidenceSubject::Unattached` 的证据（§9.2） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：`continuum-media`（实测：`grep -rn "continuum-media" . --exclude-dir=.git` 在本仓**零命中**，名字未占用）。

依赖边表（**只登记实际用到的**，切分 §四第 6 条）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-artifact` | `ArtifactType`（§3 的清单落在这里，本块经它声明端口类型） | 普通 |
| `continuum-operator` | `Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` / `BackendId` / `OperatorRegistry` / `OperatorError` / `Checkpointable` / `CheckpointError`（§4 的类型落在这个 crate 里） | 普通 |
| `continuum-method` | `MethodRegistry` / `MethodId` / `MethodError` / `bind`（§10 的 `bind_media_methods` 收 `&mut MethodRegistry`、以 `MethodId` 逐条调 `bind` 并透出 `MethodError`；P5b 设计 `:291` 明写四块**只经 `bind` 填内容**；P5b 设计 `:375` 要求每块对其领域目录里**每一条**已 seed 的 id 调用一次 `bind(id, &[...])`，故调用方是本块的具名入口 `bind_media_methods`，不是用例） | 普通 |
| `continuum-verify` | `Evidence` / `EvidenceSubject`（§12.4 第 1、2 条的 `trybuild` 用例：本 crate 里 `Evidence { .. }` / `Evidence::default()` / `EvidenceSubject::Requirement(..)` 编不过） | **dev 边** |
| `continuum-graph` | `can_reuse` / `cache_key` / `CacheKey`（§6.2 的照片：核本块声明的 `determinism` 真能让 §305 的判据成立） | **dev 边** |

`continuum-method` 是 P5b 新建的 crate，其 workspace 依赖集合为空集（P5b 设计第九节）；本块依赖它，
不构成环：`continuum-method` 不反向依赖任何 workspace crate，故 `continuum-media → continuum-method` 是单向的。

**不登记**的边，逐条给理由（零使用的边即假边，P2b 为此删过两条）：

- **不登记 `continuum-port`**：本块不构造 `Port`、不做端口兼容判定。`Operator.input_schema` / `output_schema`
  取 `Vec<ArtifactType>`（`crates/continuum-operator/src/definition.rs:82-83`），不经 `Port`。§239 的 `compatible`
  是 P1 的（`crates/continuum-port/src/port.rs:94`）。
- **不登记 `continuum-semantics`**：本块不构造 `RequirementId`——产出的证据一律 `Unattached`（§9.2）。
  该 crate 今天在 `crates/` 下不存在（P4 未开始实现，切分 §二末「与 P4 的接口面」段已记）。
- **不登记 `continuum-persist`**：本块零表（§11），不碰 `Tx` / `Migration`。
- **不登记 `continuum-events`**：本块不写事件（§11）。
- **不登记 `continuum-policy` / `continuum-capability`**：§34 的授权旗标以**值**从 Contract 传入（§7.2），本块不查策略引擎、不取 Capability Token。

**dev 边为什么也要登记**：`dependency_direction.rs` 的 `cargo tree` 带 `--edges all`
（`crates/continuum-runtime/tests/dependency_direction.rs:274`），dev 边与普通边一视同仁。
本仓已有同形的先例：`continuum-provider` 那一行注记明写「Task 4 起加上 persist：**dev 边**」
（同文件 `:32-34`）。

**边别按「谁在用」判，三处判法同一条**（零使用的边即假边，反之：生产代码用到的边即普通边）：

- `continuum-verify` 是 **dev 边**（**2026-10-10 由「普通」改判**）：原先的理由栏写的是「切分 §四第 1 条：
  四块只产出证据」，而本块**不提供这一层的函数**——原先唯一的生产侧入口 `evidence_from_media_output`
  已按协调者裁定删去（§9.2），产出由**宿主的执行代码**直接调 `from_tool_result` 构造。
  故本 crate 的**生产代码对这枚 crate 零调用**（`OperatorImpl::execute` 的返回是 `Vec<Artifact>`，
  `crates/continuum-graph/src/execution.rs:76-82`），唯一使用点是 §12.4 的两条 `trybuild` 用例。
  **判据的推广**：协调者 2026-10-10 的四块统一口径（P5a 设计 §4.4「四个领域算子块一律不写域包装」）
  使四块在该 crate 上的形态一致（P5f 设计 §2.1 早已是 dev 边，其来历写在该节的订正段）。
- `continuum-graph` 是 **dev 边**：生产代码不调它；用到它的是 §6.2 的照片（§12.2 第 (r) 条对
  `can_reuse` / `cache_key` 的断言），那是用例。
- `continuum-method` 是**普通边**：四条 `bind` 由本块的具名入口 `bind_media_methods`（§10）调用（§1.2 表第 2 行逐字
  「本块只调它的 `bind`」；§10 的四条 `realized_by` 是本块填的）。它**不是**「只在用例里出现」——
  §12.2 第 (s) 条那条运行期用例只是 `realized_by` 文本弱引用的补件（§10 的「`bind` 的参数是文本」一段），
  不是 `bind` 的使用点。

**共写文件**（切分 §一 的共写文件表给的是**五处**：`Cargo.toml`、`dependency_direction.rs`、
`artifact.rs`、`node.rs`、`crates/continuum-operator/src/definition.rs`。下表是本块的那部分，**另含切分未登记的三行**，逐行注明来历）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `crates/continuum-media` | 切分建议由先落地者一次加齐六行；本块**只加自己这一行** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-media", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify"])`——**五项**，与 §2.1 边表的五行逐行对应（含 `continuum-graph` 的 dev 边，理由见上） | 新 crate 会先让该门变红再被补齐（`:288` 的用例），这是刻意的；数组按字母序 |
| `crates/continuum-artifact/src/artifact.rs` | §3.5 的 (a)(b)(c) 三处 | 切分 §一 的共写文件表登记为「共写，但只由 P5e 一处改」 |
| `crates/continuum-artifact/tests/artifact_type.rs` | §3.5 的 (d) | 切分 §四第 2 条的 (d)（逐字点名该文件的 `:23`）；**切分 §一 的共写表里没有这一行**，故在此补记 |
| `crates/continuum-port/tests/compatibility.rs` | §3.5 的 (e) | 切分 §四第 2 条的清单已列入这一处（2026-10-09 折入；本设计定稿时它是漏项） |
| `crates/continuum-operator/src/definition.rs` | §4 的两行签名 + 新类型 + `BackendId` 的 `Display`（§4.4） | 切分 §一 的共写表已列入这一行（2026-10-09 折入；本设计定稿时它是漏项，见 §15 第 2 条） |
| `crates/continuum-operator/src/lib.rs` | §4.4 的 re-export 两枚 | 同上 |

## 2.2 冻结清单：本块持有的类型与判定

**类型**（定义在 `continuum-media`，例外逐条注明）：

```
GenerativePermission   PermittedGenerativeContent   MediaError
```

另有一枚**构造函数**（不是类型）：`all_operators() -> [Operator; 17]`，它承载的正是 `Operator`（P1 的类型）。
（**这里不是 `const`**：`Operator` 在 crate 外构造不出常量值，理由见 §5.1。）
**本块不新造算子类型**——§5.1 的登记形态照 P1 与 P5b。
以及**定义在 `continuum-operator`** 的两枚（落点理由见 §4.2）：

```
CheckpointError   CheckpointOwner
```

**判定**（本块是它们的唯一落点）：

1. `ArtifactType` 的 P5 扩展清单的**一次落地**（§3）——本块是它的唯一落地点（切分 §四第 2 条）。
2. 本领域 17 枚算子的 `determinism` 声明所依赖的「backend 逐位可复现」名单（§5.4）。
3. §34 的生成内容授权判定（§7）——判定在**本块**，调用点在第 3 层。
4. `Checkpointable` 的错误类型的形状（§4）。

**本块不持有的判定**：证据的充分性、覆盖、verifier 的级别、完成判定（全部 P5a）；
方法的登记与选取（P5b）；复用的四条件（P1 的 `can_reuse`，§6）。

## 2.3 本块的层间位置

§9.1（`docs/02-工程.md:570-586` 的图，读法在 `:561-567`，`:588` 逐字「依赖方向单向，无环。」）
里与本块有关的两条，按「被依赖者 → 依赖者」读：

- `执行层 (3) → 跨领域 (8)`：§2.1 的边表**五行**全是它的实例（例如 `continuum-media` 依赖
  `continuum-operator`、`continuum-graph`；五行对应 §3.1 的执行层组件与 §8.1 的跨领域组件）。
  `execution_policy` / `verification_policy` 的定型落在消费侧而不是字段上，也是同一条的后果
  （切分 §二第 2 条的裁定段；本块不重复它）。
- `跨领域 (8) → 长期循环 (7)`：本块**不接**这一条——本块不产出 `value + evidence + last_verified`
  （§232 的 `ProductReadiness` 属长期循环）。本块的产物注册进 `OperatorRegistry`，不写长期循环的表。

本块与 P5a（同层）之间有一条层内边 `continuum-media → continuum-verify`。**层内边在本仓有同形的处置**：
`continuum-provider → continuum-capability`（资源层内的「接口 → 能力类型」）就被登记为层内边
（`crates/continuum-runtime/tests/dependency_direction.rs:30` 的注记，与 P3c 设计 §4.1 订正段
`docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md:336` 逐字「层内边」）。
本条边是单向的：`continuum-verify` 不依赖任何 P5 领域算子块（P5a 设计 §2.1 的边表里没有它们）。
**边别为 dev 边**（2026-10-10 由「普通」改判：本块不写域包装后，生产代码对该 crate 零调用，§2.1）。

**原先此处引的先例已换掉**：原引 `docs/02-工程.md:577` 的补记行 `continuum-secrets → continuum-capability`。
实测那一行是 `资源层 (4) ──→ 边界层 (5)` 的**跨层**边（编号与行文都在 `docs/02-工程.md:577`），
不是层内边，故不足以做「同层边有先例」的论据。

---

# 3. `ArtifactType` 的 P5 扩展清单（切分 §四第 2 条的唯一落地点）

## 3.1 事实：今天六型

`crates/continuum-artifact/src/artifact.rs:14-21` 的枚举是六型：`SourceTree` / `Patch` / `TestResult` /
`Text` / `Json` / `Blob`（实测该文件 `:15-20` 逐行照录）。`ALL` 是 `:28` 的 `[ArtifactType; 6]`（`:28-35`）。
切分 §八「`ArtifactType` 的 P5 清单尚无权威来源」那一条明写「本清单尚无权威来源」，要求本设计自己定并写明逐条出处。

## 3.2 本设计定的清单：11 枚新变体

**取法（一句话）**：**取规范里三份「Artifact 类型清单」的并集，减去枚举里已有的四型**——
`§9 Typed Artifact`、`§239 Port 类型系统`、`§328 Media Operator`。三份清单都以「例如：」或「SHOULD 使用」开头，
故本清单**只声称「已实测的出处有下表各行」，不声称「Artifact 类型只有这 17 种」**（与 P5b 设计 R7 的取向一致）。

| # | 变体 | `as_str` 的串 | 出处（逐行实测） |
|---|---|---|---|
| 1 | `Image` | `image` | `docs/spec/01-concepts.md:504`（§9）；`docs/spec/05-normative.md:645`（§239）；`:2330`（§323） |
| 2 | `Audio` | `audio` | `docs/spec/01-concepts.md:505`（§9）；`docs/spec/05-normative.md:2414`（§328） |
| 3 | `Video` | `video` | `docs/spec/01-concepts.md:506`（§9）；`docs/spec/05-normative.md:646`（§239）、`:2413`（§328） |
| 4 | `Timeline` | `timeline` | `docs/spec/01-concepts.md:507`（§9）、`:1392`（§32）；`docs/spec/05-normative.md:647`（§239）、`:2417`（§328）、`:2442`（§329） |
| 5 | `Mesh` | `mesh` | `docs/spec/01-concepts.md:508`（§9） |
| 6 | `Scene` | `scene` | `docs/spec/01-concepts.md:509`（§9）；`docs/spec/05-normative.md:651`（§239） |
| 7 | `Transcript` | `transcript` | `docs/spec/01-concepts.md:513`（§9）；`docs/spec/05-normative.md:2416`（§328） |
| 8 | `Report` | `report` | `docs/spec/01-concepts.md:514`（§9） |
| 9 | `ShotSet` | `shot_set` | `docs/spec/05-normative.md:2415`（§328） |
| 10 | `SubtitleTrack` | `subtitle_track` | `docs/spec/05-normative.md:2418`（§328） |
| 11 | `Render` | `render` | `docs/spec/05-normative.md:2419`（§328） |

加型后 `ALL` 是 `[ArtifactType; 17]`。**串名沿用本文件既有的约定**（`:37` 的注记：小写、多词以 `_` 连接），
两个多词型因此是 `shot_set` / `subtitle_track`；`artifact_type.rs:32-35` 的既有断言正是这条约定的守卫。

`§239` 的七例里 `SourceTree` / `Patch` / `TestResult` 三例已在枚举内；`§9` 的十二例里
`Text` / `SourceTree` / `Patch` / `TestResult` 四例已在枚举内；`§328` 的七例**全部**不在枚举内。
故并集是 15 个名字，其中 4 个已有 ⇒ **11 枚新变体**。`Json` / `Blob` 两型不在三份清单里，
它们由 P1 引入（P1 设计 `:156-163` 的枚举；`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md`），本块**保留不动**。

**§239 那一句的后果必须写出来**：§239 `:640` 逐字是「每个 Node input/output MUST 有明确类型」，
`:654` 逐字是「运行时 MUST 拒绝不兼容连接」。该节的例子里有 `Artifact<Image>`（`:645`）与 `Artifact<Scene>`（`:651`），
而在加型之前这两型**不可表达** ⇒ 一个以它们为端口的连接在类型层写不出来，`:654` 的 MUST 对这两型无落点。
本清单把这两型补进枚举，是 §239 在媒体与图像域上的兑现，不只是「P5f 要用 `Image`」。

## 3.3 `Scene` 的显式处置：**收型，不收算子**

切分 §八「`3d/` 在四个具名算子里没有对家」那一条把「`Scene` 与 `3d/` 目录的归属」记为**本轮无解**，并明写「先记为缺口，不在切分里硬塞给某一块」。
本设计的处置分两半，两半必须分别读：

1. **收 `Scene` 这一型**（上表第 6 行）。理由两条，按分量排：
   - **它有规范出处，且出处正是本块的依据之一。** §239 `:651` 逐字列 `Artifact<Scene>`；该节正是
     `ArtifactType` 这一枚举的权威（`crates/continuum-artifact/src/artifact.rs:7` 的自述即「`§239` 的类型集合」）。
     不收它，等于让 §239 的一条例子在本版本里不可表达。
   - **§四第 2 条要的是「一次改动」。** 将来 3D 那一块落地时若必须补 `Scene`，就是**对同一个枚举的第二次改动**——
     正是切分 §四第 2 条要避免的形态（两次改动之间留下数目断言与 `ALL` 的两套取值）。现在收，代价是零：
     一个枚举变体不产生算子、不产生表、不产生迁移。
2. **不收 3D 的算子。** 本设计**不注册** `Reconstruct3D(ImageSet) -> Scene`（`docs/spec/01-concepts.md:547`）
   与 `RenderScene(Scene) -> Video`（`:549`）——它们的归属是本轮无解的那一条，本块不认领。
   `MethodDomain::ThreeD` 的 `realized_by` 继续为空（P5b 设计第七节）：**本块不向 `3d/` 目录 `bind` 任何东西**。

**本条不解决什么，写清**：切分 §八 的缺口仍在（`3d/` 无对家、`Scene` 无算子块）。本设计只是**不让这一型在类型层缺席**，
不用它替 3D 认领任何算子。若协调者裁定 `Scene` 连型也不该收，改动是「删上表第 6 行 + 删 `ALL` 的一项 + 数目断言 17→16」，
判定的归属不变。

## 3.4 不收 `SourceSet`：理由

切分 §八「`ArtifactType` 的 P5 清单尚无权威来源」那一条记「§330 的 `SourceSet` 是否算 Artifact 类型未定义」。**本设计不收**，理由一条：

**§330 与 §328 的措辞不是同一件事。** §328 `:2410` 逐字是「视频领域 SHOULD 使用**标准 Artifact**：」，
其后七行是要落的类型名；§330 `:2449-2465` 逐字是「研究任务 SHOULD **拆成**：」，其后是一条**步骤链**
（`Question` `:2452` / `Search` `:2454` / `SourceSet` `:2456` / `EvidenceExtraction` `:2458` /
`ContradictionCheck` `:2460` / `Synthesis` `:2462` / `CitationVerification` `:2464`）。
把 `SourceSet` 读成 Artifact 类型，就要在同一张表的七行里**对它另取一种读法**——另六行（`Question`、`Search`、
`EvidenceExtraction`、`ContradictionCheck`、`Synthesis`、`CitationVerification`）都是步骤名或算子名。
**这里不是「§330 没提 Artifact 所以不收」：是因为同一节内的七个名字只有它一个能被读成类型，而那种读法要求逐行分档。**

**若 P5d 需要它**（P5d 的设计今天在 `docs/superpowers/specs/` 下不存在，实测该目录只有 P5a、P5b 两份设计），
按切分 §四第 2 条向本块报出，由本块的实现任务一次落地（§3.6）。**本设计不预先收它**：
预先收一个只有一行出处、且那一行与它同节的六行读法不同的名字，是发明。

## 3.5 一次改动要同时碰的处所（**全部实测**）

切分 §四第 2 条已给出待改的处所与「不改但要重跑」的那几处。**本设计在当前树里逐条复测，并补它当时漏记的那一处**：

> **注记（2026-10-09）**：本节的复测写在切分文档接受这些发现**之前**——正文的「切分当时…」与下表
> 「切分给的行号」一栏描述的是**折入前**的切分文档（当时待改的是 (a)–(d) 四处）。切分文档现已折入
> (e) 与 `:24` 的注记（来历见 §15 开头的注记），故那几处读时应以切分文档**现在**的文本为准；
> 本节其余各条（本设计实测出来的那份清单）不受影响。

**(a)–(d) 复测结果（与切分一致）**：

| 项 | 切分给的行号 | 本设计实测 | 一致 |
|---|---|---|---|
| (a) 枚举本体 | `artifact.rs:14-21` | 枚举头 `:14`，六型 `:15-20`，闭合 `:21` | 一致 |
| (b) `ALL` | `artifact.rs:28-35` | 见下 | 一致（切分当时只点代码，未点 `:24` 的注记；该注记已于 2026-10-09 折入，行号亦随折入由 `:28-35` 收成 `:28`） |
| (c) `as_str` / `parse` | `artifact.rs:47` / `:68` | `:47` 是 `pub fn as_str`，`:68` 是 `pub fn parse` | 一致 |
| (d) 数目断言 | `tests/artifact_type.rs:23` | `:23` 逐字 `assert_eq!(ArtifactType::ALL.len(), 6, "ArtifactType 的名单与变体数不符")` | 一致 |

**(b) 的补充**：`ALL` 的**注记 `:24` 带数目字样**——逐字是「全部六型。供全量遍历的用例使用（与
[`PrivacyClass::ALL`] 同形）。」。切分当时的 (b) 只点了 `:28-35` 的代码，没点 `:24` 的文本；2026-10-09 已把 `:24` 的注记折入（见本节注记）。
**这一处不会编译失败**（它在 `///` 里），改完代码不改它，注释就变成假话。

**(e) 切分漏记的一处，且是唯一一处不会变红的**：

`crates/continuum-port/tests/compatibility.rs:47` 的用例名逐字是
`fn six_types_round_trip_through_serde()`，其函数体 `:49-54` 手工列了六型做 serde 往返。
加 11 型之后：

- 它**照旧编译、照旧通过**——六型仍在枚举里，`serde_json` 往返仍成立；
- 而它的**名字与它的清单都成了假话**（枚举已有 17 型，它只说 6 型）。

故「一次改动」的清单是**五处**（(a)–(e)；切分文档 2026-10-09 已把 (e) 折入，此前只列 (a)–(d)），其中带报警的四处是
(a)（编译失败）、(b)（编译失败：`[ArtifactType; 6]` 与 `ALL` 的项数不符）、(c)（编译失败：两个穷尽 `match` 各缺 11 臂）、
(d)（断言失败）；**(e) 是唯一不会响的一处**。**处置**：把该用例改为遍历 `ArtifactType::ALL`（并在改名时去掉名字里的数目字面量，
使下一次加型不必再改名字）。**这里不是「照既有写法补 11 行」：手工清单在这里已经证明过它会漂**
——它与 `ALL` 是两个各自维护的清单，而 `ALL` 有守卫、它没有。

**不改但要重跑的三处（切分 §四第 2 条三处全列，第三处是 `compatible`）**：

| 项 | 位置（实测） | 为什么不改 | 重跑要看什么 |
|---|---|---|---|
| `parse_type` | `crates/continuum-artifact/src/persist.rs:164` | 委托到 `ArtifactType::parse`（`:165`），不含自己的表 | 用例全绿；新型的串能落库并读回 |
| `parse_artifact_type` | `crates/continuum-graph/src/persist.rs:410` | 同上（`:411`） | 同上（这是 `adfir_port.artifact_type` 那一列的读路径，`:259` / `:278`） |
| `compatible` | `crates/continuum-port/src/port.rs:94` | 按 `ArtifactType` 等值判定（`:101`），与型数无关 | 用例全绿 |

**(f) 一处会静默变假、但不在「一次改动」清单里**：`crates/continuum-artifact/tests/artifact_type.rs:40` 的
六个「表外取值」里没有与新串相撞者（实测：`""` / `"source tree "` / `"SourceTree"` / `"source-tree"` / `"nope"` / `"patch_"`，
与上表 11 个新串无一相同），**故它不需要改**；这一条是**重跑时的核对项**，写在此处是为了让下一轮的实现者
不必自己重建「新串会不会撞上表外用例」这个问题。

**(g) 一处本设计不改、但会随加型而静默变旧的文本**：`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:573`
逐字写「ArtifactType 集合 本子项目取六种。P5 引入媒体类型时为编译期可见的破坏性变更。」。
它是 P1 设计对**P1 交付物**的历史陈述（该句后半句预言的正是本块的改动），本设计**不改别人的文件**，
只在此处记下它，收件人见 §13 第 9 条。

## 3.6 「一次改动」在本轮的排布下**不可能在设计期成立**

切分 §四第 2 条把「一次改动」写成：四块「向 P5e 报出所需变体，由 P5e 在一次改动里落地」。
**实测：这条在本轮不可能按设计期的口径兑现。**

- `docs/superpowers/specs/` 下今天只有两份 P5 的设计（P5a、P5b；P5e 是本文件的第三份）。
  **P5c、P5d、P5f 三份设计不存在**，故它们「报出」的那些变体不可能在本设计定稿之前到达。
- 本设计因此只能落地**本块有出处的**那份清单（§3.2）。若 P5c/P5d/P5f 事后各报一两型，那份清单要再改一次枚举。

**处置（本设计取这一条，并把判据写成可执行的）**：把「一次改动」的对象从**设计期**移到**实现期**——
`ArtifactType` 的那一次编辑发生在 **P5e 的实现任务**里，且**它是这六块里最后一次编辑这个枚举的编辑**。
可行的排法只有一种：**P5e 的枚举实现任务排在 P5c/P5d/P5f 的设计与「报出」之后**。
判据：实现该任务的 brief 里必须写明「本任务落地的是 P5e 设计的 §3.2 清单 + P5c/P5d/P5f 已报出的变体，
二者合并为一次编辑」；若该 task 先于那三份报出而运行，则「一次改动」当场不成立，且切分 §四第 2 条的两条后果
（编译不过的中间树、数目断言改两次）都会发生。

**收件人：协调者**（排期与派单）。切分文档本身不改（本设计只记，见 §15）。

**另一条可行性注记**：本设计**不**为了「一次改动」而预先收 P5c/P5d/P5f 可能需要的变体。
预收的判据只能是猜，而猜错的变体一旦进枚举就要在版本边界上永远带着（`as_str` / `parse` 是它的编码的
唯一产生点，删一型同样是破坏性变更）。

---

# 4. `Checkpointable` 的错误类型（切分 §二第 4 条）

## 4.1 今天的事实（逐条实测）

- `crates/continuum-operator/src/definition.rs:92` 起是 `pub trait Checkpointable`，`:93` 是关联类型
  `type Checkpoint: Send + Sync;`，`:95` 与 `:96` 两行逐字是
  `fn checkpoint(&self) -> Result<Self::Checkpoint, String>;` 与
  `fn restore(&self, checkpoint: &Self::Checkpoint) -> Result<(), String>;`——**两处 `String`**。
- `crates/continuum-operator/tests/registry.rs` 只测 `OperatorRegistry`；全仓 `grep -rn "Checkpointable" crates/`
  只有 `definition.rs:92` 一处命中 ⇒ **无实现、无调用点、无测试**（与切分 §八「`Checkpointable` 今天无实现、无调用点、无测试」那一条一致）。
- P1 的**设计**写的是 `Result<Checkpoint, OperatorError>` / `Result<(), OperatorError>`
  （`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:369-372`）；**计划**把它落成了 `String`，
  并把这条偏离记在遗留里（`docs/superpowers/plans/2026-10-01-p1-execution-layer.md:5563`：
  「definition.rs 的 Checkpointable 用 String 作错误类型，设计写的是 OperatorError。plan-mandated。」）。
  ⇒ **这不是一处无意的形状，「把它交给第一个实现者定」是记录在案的决定**（同计划 `:1142` 逐字：
  「用 `String` 而不是自定义错误类型：本子项目无实现，错误类型的形状应由第一个实现它的领域算子（P5）确定。」）。
- `Checkpointable` **未经 `lib.rs` re-export**（`crates/continuum-operator/src/lib.rs:7-9` 的 `pub use` 里没有它；
  P1 计划 `:5565` 记为遗留）。故今天引用它必须写全路径 `continuum_operator::definition::Checkpointable`。

## 4.2 落点：类型必须定义在 `continuum-operator`，不能定义在 `continuum-media`

**这是本节的第一个裁定，理由是依赖方向（不是命名偏好）。**

`Checkpointable` 是一个 **trait**，它的两个方法签名里出现错误类型。若错误类型定义在 `continuum-media`（第 8 层），
则 `continuum-operator`（第 3 层）要 `use` 它 ⇒ `跨领域 (8) → 执行层 (3)` 反向边；而 `continuum-media`
本身依赖 `continuum-operator`（它要注册 `Operator`）⇒ **第 3 层与第 8 层之间的 2-环**，
与 §9.1 `:588` 逐字写的「依赖方向单向，无环」直接相抵，并会让 `dependency_direction` 门在两个方向都红。

故：**形状由本块定（切分 §二第 4 条），类型定义落在 `crates/continuum-operator/src/definition.rs`**
——这与 `Checkpointable` 同处一个文件，且它需要的三个身份类型
（`OperatorId` `:24`、`OperatorVersion` `:45`、`BackendId` `:66`）都已在同文件内，
**落点不新增任何一条依赖边**。

**切分文档原先的一处漏项（2026-10-09 已折入）**：本设计定稿时，§一 的共写文件表只登记了 `Cargo.toml`、`dependency_direction.rs`、
`artifact.rs`、`node.rs` 四处，**没有 `crates/continuum-operator/`**；切分文档现已补上该行。而 §二第 4 条把错误类型指给 P5e 时，
没有指出它只能落在别人的 crate 里。本设计据此把它补进 §2.1 的共写表（两行：`definition.rs` 与 `lib.rs`），
并在 §15 第 2 条记为切分文档当时的漏项。

**「这里不是 `continuum-media`：因为那会让第 3 层依赖第 8 层，即 2-环。**」这是一条**硬**理由
（同切分 §二第 2 条订正段对 `verification_policy` 的处置形状：先例只是「同一处境曾有同一处置」，成环才是
「不得不这么办」——那一处的两条理由都写在小节里，本处同理只写后者，因为**本处没有先例**）。

## 4.3 形状：三臂

```
#[derive(Debug)]
pub struct CheckpointOwner {
    pub id: OperatorId,
    pub version: OperatorVersion,
}

// `CheckpointError` 的三个格式串以 `{owner}` / `{found}` / `{expected}` 引用本型，
// thiserror 要求 Display；该枚举自身 `derive(Debug)` 也要求本型实现 Debug。
impl std::fmt::Display for CheckpointOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

/// §308 的检查点接口的错误类型。
/// 它**不是**第二份 `OperatorError`：那两枚讲的是「注册表里有没有这个算子」，
/// 本型讲的是「这一次检查点/恢复为什么不成」。两者的调用方、恢复策略都不同
/// （§309 的 failure type 分类要能分开它们）。
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    /// §308 是 **SHOULD**，而 §245（`docs/spec/05-normative.md:756`）允同一个 Operator 有多个 backend
    /// ——故「不支持」是一个合法答案，且它的主语是**选中的那个 backend**，不是这个算子。
    #[error("backend {backend} 不支持检查点（§308）")]
    UnsupportedByBackend { backend: BackendId },

    /// 尚无状态可保存：算子还没产出任何可检查点的中间状态。
    #[error("算子 {owner} 尚无可检查点的状态")]
    NothingToCheckpoint { owner: CheckpointOwner },

    /// 恢复被拒：该检查点不属于本算子实例。
    #[error("检查点属于 {found}，本实例是 {expected}，恢复被拒")]
    RestoreRejected {
        expected: CheckpointOwner,
        found: CheckpointOwner,
    },
}
```

**三臂各自的判据（为什么是这三臂、不是两臂、不是四臂）**：

- `UnsupportedByBackend` 的**存在理由**是 §245 的多 backend。若把它删掉，则「后端不支持」只能表现为
  `Ok(())`——即**假装检查点成功**，而这在崩溃恢复时会把「没存」读成「存了」。这是三臂里唯一一条
  fail-open 反面的臂。
- `NothingToCheckpoint` 与 `RestoreRejected` 是**两个不同的处置**：前者是「现在没有东西可存，稍后再来」
  （可重试），后者是「这份检查点不是我的，重试无用」（不可重试）。把它们合成一臂，§309 的 failure type
  分类就没有落点。
- **不给 `operator.determinism` 之类的取值留臂**：那属 `OperatorError` 与注册期检查（§5.4），不属于一次检查点调用。
- **不给 IO 臂**：`checkpoint()` 的返回类型是 `Self::Checkpoint`（`:93`），落盘是调用方的事；
  本型里放一个 IO 臂就要求本块规定检查点的存储形态，而 §308（`:2037-2052`）只规定了「SHOULD 支持 checkpoint」，
  没给存储。**这是据实留的缺口**，见 §14 第 6 条。

**`CheckpointOwner` 为什么是「id + version」两字段而不是 `OperatorRef`**：`OperatorRef` 的定义在
`crates/continuum-graph/src/node.rs:42-45`（由 `OperatorId` + `OperatorVersion` 构成），而 `continuum-graph`
**依赖** `continuum-operator`（`dependency_direction.rs:49-58` 的 `continuum-graph` 那一行含 `continuum-operator`）。
若本型使用 `OperatorRef`，`continuum-operator` 就要依赖 `continuum-graph` ⇒ **2-环**。
故在此就地取那两个字段（它们与 `OperatorRef` 的两字段同形，不构成「同一件事两个词汇表」：
`OperatorRef` 是**图上节点**对算子的引用，本型是**检查点**对产出它的算子实例的引用，两者都不是彼此的副本
——这一点据实写在类型旁边，见下节。

**一处必须写下的重复**：`CheckpointOwner` 与 `OperatorRef`（`crates/continuum-graph/src/node.rs:42-45`）
两字段逐字相同。本设计**不**把它们合并（合并要么让 `continuum-operator` 依赖 `continuum-graph`，即 2-环；
要么把 `OperatorRef` 下移到 `continuum-operator`，那要动 P1 的图 API 与它的 `pub use`
（`crates/continuum-graph/src/lib.rs:24`））。**记为对账条目**（§13 第 2 条 (ii)），收件人：P1 与协调者。

## 4.4 改动面（两行签名 + 一枚新类型 + 两行 re-export）

| 位置（实测） | 改动 |
|---|---|
| `crates/continuum-operator/src/definition.rs:95` | `Result<Self::Checkpoint, String>` → `Result<Self::Checkpoint, CheckpointError>` |
| 同文件 `:96` | `Result<(), String>` → `Result<(), CheckpointError>` |
| 同文件末尾（`Checkpointable` 之后） | 加 `CheckpointOwner`、`impl std::fmt::Display for CheckpointOwner`（§4.3）与 `CheckpointError` |
| 同文件 `:76`（`BackendId` 之后） | 加 `impl std::fmt::Display for BackendId`——§4.3 与 §5.1 两个错误类型的格式串都以 `{backend}` 引用该字段，thiserror 要求它实现 `Display`（同文件 `:36-37`、`:57-58` 的既有注记即此规则），而该实现今天不存在（实测全 crate 只有 `OperatorId` `:38` 与 `OperatorVersion` `:59` 两个 `Display`）。实现体只依赖 `std::fmt`，**不新增 crate 依赖**，`dependency_direction.rs` 的 `continuum-operator` 那一行不动 |
| 同文件 `:90-91` 的注记 | 「本子项目只定义接口」改为「接口在 P1，错误类型与第一枚实现在 `continuum-media`」 |
| `crates/continuum-operator/src/lib.rs:7-9` | `pub use` 补 `Checkpointable` 与 `CheckpointError`（一并处理 P1 计划 `:5565` 记的「未 re-export」遗留） |
| `crates/continuum-operator/Cargo.toml:11` | `thiserror` 已在依赖里（`:11`），**不新增依赖** |

**`dependency_direction.rs` 的 `continuum-operator` 那一行不动**：实测该行是
`("continuum-operator", &["continuum-artifact"])`（`:46`），本节的改动不引入任何新 crate 依赖。

## 4.5 第一枚实现：`continuum-media` 的两枚长算子

切分 §八「`Checkpointable` 今天无实现、无调用点、无测试」那一条明写「一条今天完全没有测试的接口要靠 P5e 首次兑现」，要求写明落点与判据。
本设计把「首次兑现」定为**真的实现**，不是「定义了一个没人实现它的类型」：

- **实现的算子**：`generate-broll`（§308 `:2048` 点名的第一项长任务「video generation」）与 `render`（§5.2 第 11 行）。
  两枚都是本领域的长任务；§308 的「尤其」清单是**举例**（`:2045` 逐字「尤其：」），
  故 `render` 按同一条 SHOULD 一并实现，**不是**把它读成 §308 的穷尽清单。
- **`type Checkpoint`**：两枚各自定义一个检查点结构，字段是本算子的**进度状态**（例如已完成的片段区间与
  已确定的后端选择）。**字段清单不在本设计的约束范围内**：它随算子的实现走，本设计只约束下面三条判据。
- **判据（两侧都要，缺一条即不算钉住）**：
  1. `checkpoint()` 在**有进度**时返回 `Ok`，且 `restore` 该检查点后算子的进度与存前一致（正例）；
     在**尚无进度**时返回 `Err(NothingToCheckpoint { .. })`（反例）。
  2. 用**另一版本**（`OperatorVersion` 不同）的同一算子的检查点去 `restore` ⇒ `Err(RestoreRejected { .. })`，
     **且恢复失败后算子状态与调用前逐字段相同**（半恢复比不恢复更坏）。
  3. 选中的 backend 不支持检查点时 ⇒ `Err(UnsupportedByBackend { .. })`；支持时 ⇒ `Ok`（两侧）。
- **调用点不在本块**（据实写明）：谁在什么时候调 `checkpoint()` / `restore()`，规范未给；它属第 3 层的
  恢复路径（§311 / §312 那一组）。故「无调用点」这件事在 P5e 落地之后**仍然成立**，只是从「无实现、无调用点、
  无测试」变成「有实现、有测试、无生产调用点」。切分 §八「算子解析的落点仍无人认领」那一条记的
  与这一条**同源**，本设计把它们合并成一条对账条目（§13 第 1 条），不各自记一次。

## 4.6 残余：本型不规定检查点的存储

`Checkpoint` 是一个 `Send + Sync` 的 Rust 值（`:93`），它的持久化、版本兼容与损坏检测都不在本型的表达范围内。
故 `RestoreRejected` 的判据是「检查点**携带**的身份与本实例不同」——即**实现者必须让检查点自带身份**，
而这条义务今天只能靠实现者的自觉与用例（§4.5 判据 2），类型层拦不住。
**「该不该给 `type Checkpoint` 加一条身份约束（如 `CheckpointIdentity`）」不在本块**：
那改的是 trait 的**关联类型**，而切分 §二第 4 条给本块的是**错误类型**。记为 §14 第 6 条。

---

# 5. 算子集

## 5.1 登记形态：照 P1 与 P5b，不自造

§244（`docs/spec/05-normative.md:735-753`）的 `Operator` **七字段**由 P1 落地为
`crates/continuum-operator/src/definition.rs:79-88`（字段本体 `:80-87`）：
`id` / `version` / `input_schema: Vec<ArtifactType>` /
`output_schema: Vec<ArtifactType>` / `determinism` / `side_effect_class` / `backend_candidates: Vec<BackendId>`。
> **订正（2026-10-09，P5d 的复审查出）**：此处原写「**八字段**」。**§244 列的是七个**（实测
> `05-normative.md:735-753`，逐行七名），**八是 §258 的 `Evidence` 的字段数**（`05-normative.md:1073-1088`）——
> **那个数字是从相邻条款串过来的，不是 §244 说的。**
> **而这句话后面紧接着就只列了七个名字**——**句子与自己的清单相抵，却因为没有人数一遍自己列的名字而留了下来**（同一句被 P5d 与 P5f 的设计逐字抄去，各需改自己那份）。
**本块不加字段、不改签名**（切分 §二「无需重新冻结」段）。本块交付的是**内容**：
17 枚 `Operator` 值的清单与它们的注册入口。

注册入口的形状照 `OperatorRegistry`（`crates/continuum-operator/src/registry.rs:19-48`）：

```
/// 本领域 17 枚算子的全集。顺序即 §5.2 表的行序。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**：理由见下（`Operator` 在 crate 外构造不出常量值）。
/// 数目 17 写在**返回类型**里，使清单个数的改动编译期可见。
pub fn all_operators() -> [Operator; 17];

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核 §5.4 的声明，全部通过后再逐枚注册；任一枚不成即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚媒体算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§5.4 那条规则要能对**任意** `Operator` 触发，
/// §12.2 第 (i) 条才写得出来。
pub fn register_media_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), MediaError>;
```

`MediaError` 的形状（§2.2 列出的本块三枚类型之一）。三臂，逐臂给出**判定的持有者**：

```
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// §5.4 的注册期规则不成立：声明 `Deterministic` 的算子混进了名单外的 backend。
    /// 判定的持有者在本块（§5.4）。
    #[error("算子 {operator} 的候选 backend {backend} 不在逐位可复现名单内（§5.4）")]
    UnverifiableDeterminism { operator: OperatorId, backend: BackendId },

    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1（见下段）。
    #[error(transparent)]
    Registry(#[from] OperatorError),

    /// §34 的生成授权判定不成立：该算子所需的那一枚旗标未在 Contract 里声明为真（§7.2）。
    #[error("算子 {operator:?} 未获 {required:?} 授权（§34）")]
    NotPermitted {
        operator: OperatorId,
        required: GenerativePermission,
    },
}
```

**`Registry` 这一臂为什么是包装而不是同名臂**：`(id, version)` 撞车的判定**已经有一枚臂**
（`crates/continuum-operator/src/registry.rs:11` 的 `OperatorError::Duplicate`，判定在 `:24-34` 的
`OperatorRegistry::register`），本块再立一枚
`MediaError::Duplicate` 就是「注册表里有没有这个算子」的第二套词汇表。**这里不是「照
`CheckpointError` 的做法把注册期错误并进本型」**：§4.3 的 `CheckpointError` 与 `OperatorError`
**分得开**，因为两者的判定不同（一个是「这一次检查点/恢复为什么不成」，一个是注册期查重）；
本臂的判定与 `OperatorError::Duplicate` **是同一件事**，故只带出来、不重述。

`all_operators()` 是**函数不是常量**。**这里不是 `const`**，两条理由各自独立成立：
其一，本仓带 `ALL` 常量的既有做法（`crates/continuum-artifact/src/artifact.rs:28` 的
`ArtifactType::ALL`、同文件 `:100` 的 `PrivacyClass::ALL`）**成立的前提是那些类型无字段**——
取值可在 `const` 中直接枚举，`Operator` 不满足这个前提；
其二，`Operator` 含 `OperatorId`（`String`，**字段私有**，`new` 不是 `const fn`，
`crates/continuum-operator/src/definition.rs:24-27`）与三枚 `Vec` 字段
（`input_schema` / `output_schema` / `backend_candidates`，`:79-88`），
故在 crate 外**构造不出 `Operator` 的常量值**。
**引用一条既有约定时要连它的成立条件一起搬**。数目 17 改由**返回类型**承载
（`[Operator; 17]`），仍使清单个数的改动**编译期可见**——这正是本行要保住的那条性质。

## 5.2 十七枚算子

出处列的口径：**算子名与端口类型** 尽量照录规范，**`determinism` / `side_effect_class` / backend 候选**
在规范未给处由本设计定，并逐行标出「定」字。§245 `:761-764` 给了 `Transcribe` 的三个 backend 名，
§10（`docs/spec/01-concepts.md:554-572` 的 lowering 段：`FFmpeg` `:559`、`Blender` `:563`、`Git` `:567`、
`GPU backend` `:571`）给了后端 lowering 的例子——**这两处是照录**。

| # | id | input_schema | output_schema | determinism | side_effect_class | backend 候选 | 出处 |
|---|---|---|---|---|---|---|---|
| 1 | `media-import` | `[]` | `[Video, Audio]` | `Deterministic` | `Idempotent` | `local-fs` | §32 `:1366`「素材导入」 |
| 2 | `proxy-build` | `[Video]` | `[Video]` | `Deterministic` | `Idempotent` | `ffmpeg` | §33 `:1404`「建立 proxy」 |
| 3 | `shot-detect` | `[Video]` | `[ShotSet]` | `Deterministic` | `Pure` | `ffmpeg-scene-detect` `pyscenedetect` | §10 `:541`；§32 `:1368`；§33 `:1405` |
| 4 | `transcribe` | `[Audio]` | `[Transcript]` | `NonDeterministic` | `Pure` | `local-whisper` `cloud-model` `specialized-asr` | §10 `:543`；§245 `:761-764`（三 backend 照录） |
| 5 | `audio-analysis` | `[Audio]` | `[Json]` | `Deterministic` | `Pure` | `librosa` `essentia` | §32 `:1372`；§33 `:1409`「beat map」 |
| 6 | `vision-analysis` | `[Video, ShotSet]` | `[Json]` | `NonDeterministic` | `Pure` | `local-vision-model` `cloud-vision-model` | §32 `:1374`；§33 `:1407-1408` |
| 7 | `metadata-analysis` | `[Video, Audio]` | `[Json]` | `Deterministic` | `Pure` | `ffprobe` | §33 `:1410` |
| 8 | `narrative-plan` | `[ShotSet, Transcript, Json]` | `[Json]` | `NonDeterministic` | `Pure` | `primary-model` | §32 `:1376` |
| 9 | `timeline-compose` | `[Json, ShotSet, Audio]` | `[Timeline]` | `Deterministic` | `Pure` | `builtin` | §10 `:545`；§32 `:1378`；§329 |
| 10 | `effects-subtitle` | `[Timeline]` | `[Timeline, SubtitleTrack]` | `Deterministic` | `Pure` | `ffmpeg-filtergraph` | §32 `:1380`；§328 `:2418` |
| 11 | `render` | `[Timeline]` | `[Render]` | `Deterministic` | `Idempotent` | `ffmpeg` | §32 `:1382`；§328 `:2419` |
| 12 | `generate-broll` | `[Timeline]` | `[Video]` | `NonDeterministic` | `Pure` | `video-gen-backend` | §34 `:1424` |
| 13 | `generate-voice` | `[Transcript]` | `[Audio]` | `NonDeterministic` | `Pure` | `tts-backend` | §34 `:1425` |
| 14 | `generate-music` | `[Json]` | `[Audio]` | `NonDeterministic` | `Pure` | `music-gen-backend` | §34 `:1426` |
| 15 | `frame-interpolation` | `[Video]` | `[Video]` | `NonDeterministic` | `Pure` | `interpolation-backend` | §34 `:1427` |
| 16 | `verify-deterministic` | `[Render, Timeline]` | `[Json]` | `Deterministic` | `Pure` | `builtin` | §32 `:1384` |
| 17 | `verify-multimodal` | `[Render, Transcript, SubtitleTrack]` | `[Json]` | `NonDeterministic` | `Pure` | `independent-model` | §32 `:1386` |

**第 8 行的出处栏原先还引了 §31**（`docs/spec/01-concepts.md:1330-1356`）。实测该节逐行给的是
「主模型为纯文本时 Runtime 可以自动增加 Image / Vision / Audio / Video / 3D Model」——是**模型增补**，
没有算子名、没有端口类型，不符本表出处栏「算子名与端口类型尽量照录规范」的口径（本节开头那段），
故删去；该行的 `NonDeterministic` 仍由 §32 `:1376` 的 Narrative Planning 与 `primary-model` 候选支持。

**逐枚的判定理由（只写需要解释的，七枚）**：

1. **`media-import` 的 `input_schema` 为空**：它的输入是外部素材，不是上游制品。§244 的字段是
   `Vec<ArtifactType>`，「零个输入」在类型里可表达。**这里不是「给它一个占位输入类型」：那会让图上多一条
   不存在的 DATA 边，而 §303 的 READY 判据（P1 设计第 12 节，`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:382`
   逐字「所有输入 Port 已有可用的 DATA 入边来源」）要求所有输入端口有边。**
   它的候选只有 `local-fs` 一个。**这里不是「把远程抓取一并列进去」：抓取所得的字节由远端决定，
   而 P1 的复用凭据以「输入哈希」为前提（`crates/continuum-graph/src/reuse.rs:10` 的 `input_hash`）——
   对一个无输入制品的算子，那份哈希只能取自素材本身，而远端内容的可变性使这条前提无从成立**，
   列进去即让 §5.4 的名单规则当场为假。代价是远程素材导入不进复用；这一侧的代价据实记在此处。
2. **`transcribe` 是 `NonDeterministic`**（§5.4 的 fail-closed 取向）。代价据实写：§131 `:1955` 的
   `Transcript` 是它列的**第一个**派生产物，而 `NonDeterministic` 让 §305 的复用对它不成立。
   两侧代价见 §6.3。
3. **`shot-detect` / `audio-analysis` / `metadata-analysis` / `timeline-compose` / `effects-subtitle` 是
   `Deterministic`**：这五枚的判定是**算法**（切分点、节拍、容器元数据、按计划拼轨、按参数加滤镜），
   不含模型解码。**这是本设计的判定，不是规范的**——规范没有一处给过 operator 的 determinism。
4. **`narrative-plan` / `vision-analysis` / 生成类的四枚是 `NonDeterministic`**：它们都经模型或生成后端。
5. **生成类的四枚是 `Pure`**（2026-10-10 订正：此前取 `NonIdempotent`，理由「重跑一次得到**另一段**视频」）。
   **那次取值把 `Determinism` 的理由当成了 `SideEffectClass` 的理由**——而两者是两个正交字段：「重跑得到另一段」
   是非确定性，不是对外部世界的改变。四枚的产物是**一个值**（`Video` / `Audio`），它们**不改变本仓之外的状态**；
   消耗的算力／计费记在预算维度（§333 的 `gpu_time` / `money_cost`），不是副作用。口径与依据见 P5d 设计 §4.4 第 3 条。
   这与 P1 的 `SideEffectClass`（`crates/continuum-operator/src/definition.rs:17-21`）的语义一致：它只对
   「重复执行会累积外部后果」那一类（§307 点名）给出禁令，本块今天没有那一类算子。
6. **本块三枚 `Idempotent`（`media-import` / `proxy-build` / `render`）按口径的边界测试取**：三枚的
   **执行体（backend）自己有落盘动作**——`media-import` 经 `local-fs` 把外部素材写进本地存储、
   `proxy-build` 与 `render` 经 `ffmpeg` 在本地存储落下 proxy / 成品文件——故它们**改变本仓之外的状态**，
   而重复执行得到同一状态，取 `Idempotent`。**边界的测试**（看端口上是「值」还是「制品引用／句柄」、
   以及执行体有没有自己的落盘动作）见 P5d 设计 §4.4 第 3 条。**原句「它产生一个大制品、耗时长……
   重算一次渲染的成本不是零」已废**：2026-10-10 的裁定明写「消耗资源」与「改变状态」是两件事
   （§334 `docs/spec/05-normative.md:2528` 把「低成本」与「无外部副作用」并列即是此意），故成本不再是
   `Idempotent` 的判据；**此处置据的是「执行体自己落盘」这一条，不是成本**。**这是本设计的判定。**
7. **`verify-deterministic` 的 `Deterministic`**：它就是 §262 第 1 级的那个确定性检查器（§32 `:1384`）。
   **`verify-multimodal` 的 `NonDeterministic`**：它是 §262 第 3 级的独立模型验证器（§32 `:1386`）。

**没有 `Scene` 与 `Image` 的算子**：`Image` 的两个算子是 P5f 的（§161 `:568-594`，本块不注册）；
3D 链的两个算子是 §1.2 表里那条「本轮无解」（本块不注册）。**故 `Scene` / `Mesh` / `Report` 三型在本块
没有生产者**——这三型是 §3.2 按「规范的 Artifact 类型清单的并集」收进来的，它们有没有生产者**不改变
枚举的取值**（枚举的守卫是 `as_str` / `parse` 的穷尽性与数目断言，不是「每一型都有生产者」，
`crates/continuum-artifact/tests/artifact_type.rs:16-20` 的注记已把这条守卫的射程写清）。

## 5.3 端口类型与 `compatible`

**先定关系**：上游算子的 `output_schema` 与下游算子的 `input_schema` **相交非空**——这是 §239 `:654`
「运行时 MUST 拒绝不兼容连接」在**注册期**可检查的那一半。

**本节原先的两句相抵，择一为准，两句的来历都留在下面**：

- 原先第一句写「本块的**每一对相邻算子**，其 `output_schema` 与下游的 `input_schema` 相交非空」。
  「相邻」若按 §32 的**步骤序**读，这句为假：§32 是 11 步的**步骤序**，不是数据依赖序。逐对实测，
  四对相邻步骤相交为**空**——`shot-detect`（出 `[ShotSet]`）→ `transcribe`（入 `[Audio]`）、
  `transcribe`（出 `[Transcript]`）→ `audio-analysis`（入 `[Audio]`）、
  `audio-analysis`（出 `[Json]`）→ `vision-analysis`（入 `[Video, ShotSet]`）、
  `verify-deterministic`（出 `[Json]`）→ `verify-multimodal`（入 `[Render, Transcript, SubtitleTrack]`）。
- 原先第二句写「把 §5.2 表里的 17 行与它们之间的边逐条列出，**逐边断言 `ArtifactType` 相等**」。
  这与 §5.2 的表相抵：多数相邻算子的 schema 只相交、不相等（例如 `media-import` 出 `[Video, Audio]`，
  而 `proxy-build` 只入 `[Video]`；`effects-subtitle` 出 `[Timeline, SubtitleTrack]`，而 `render` 只入 `[Timeline]`）。

**以「数据依赖边 ＋ 相交非空」为准**（「相等」那一条删去，理由即上一条）。

**边表**（§12.2 第 (t) 条的语料）。每行给「下游算子 ← 输入的来源」，括号里是**该来源的 `output_schema`
与下游 `input_schema` 交集中的型**（一枚来源可以供上多枚，如第 7 行）：

| 下游算子 | 输入的来源（交集中的型） | 依据 |
|---|---|---|
| `media-import` | 无输入（外部素材，§5.2 第 1 行） | §5.2 第 1 行 |
| `proxy-build` | `media-import`（`Video`） | §33 `:1404` |
| `shot-detect` | `media-import`（`Video`） | §32 `:1366`→`:1368` |
| `transcribe` | `media-import`（`Audio`） | §32 `:1370`（ASR 读素材音轨，不经镜头切分） |
| `audio-analysis` | `media-import`（`Audio`） | §32 `:1372` |
| `vision-analysis` | `media-import`（`Video`）、`shot-detect`（`ShotSet`） | §32 `:1374`；§33 `:1407-1408` |
| `metadata-analysis` | `media-import`（`Video`、`Audio`） | §33 `:1410` |
| `narrative-plan` | `shot-detect`（`ShotSet`）、`transcribe`（`Transcript`）、`vision-analysis`（`Json`） | §32 `:1376`；§5.2 第 8 行的 `input_schema` |
| `timeline-compose` | `narrative-plan`（`Json`）、`shot-detect`（`ShotSet`）、`media-import`（`Audio`） | §32 `:1378`；§5.2 第 9 行的 `input_schema` |
| `effects-subtitle` | `timeline-compose`（`Timeline`） | §32 `:1380` |
| `render` | `effects-subtitle`（`Timeline`） | §32 `:1382` |
| `generate-broll` | `timeline-compose`（`Timeline`） | §5.2 第 12 行 |
| `generate-voice` | `transcribe`（`Transcript`） | §34 `:1425`；§5.2 第 13 行 |
| `generate-music` | `narrative-plan`（`Json`） | §5.2 第 14 行 |
| `frame-interpolation` | `media-import`（`Video`） | §5.2 第 15 行 |
| `verify-deterministic` | `render`（`Render`）、`timeline-compose`（`Timeline`） | §32 `:1384`；`Timeline` 那一支是本设计的判定（§8 末） |
| `verify-multimodal` | `render`（`Render`）、`transcribe`（`Transcript`）、`effects-subtitle`（`SubtitleTrack`） | §32 `:1386`；§5.2 第 17 行的 `input_schema` |

**依据栏的读法**：写 §32/§33/§34 的，出处是规范正文的步骤名；写「§5.2 第 N 行」的，出处是
**该行的 `input_schema` 与全表 `output_schema` 的对应**（即本设计的判定，规范未给边）。

**判据（§12.2 第 (t) 条）**：逐行断言下游 `input_schema` 的**每一个型**都能在该行列出的上游里找到，
且所引的那一枚的 `output_schema` 含该型。断言的对象是「交集中的型」，不是「两侧相等」。

**本表不是全图**：它只列 §32 的剪辑链、§33 的预处理、§34 的四枚生成算子与链尾两枚 verifier 的
输入来源。两枚生成算子的产物回流进 `timeline-compose` 之类的**其他**边属装配方的图构造，
不在本块的声明范围内（§1.2：本块只**注册**算子，不建执行路径）。

与 P1 的分工：`compatible`（`crates/continuum-port/src/port.rs:94`）判**端口连接**的兼容性，属 P1；
本块只保证自己声明的 `input_schema` / `output_schema` 取值正确。

## 5.4 一处结构缺口：`determinism` 挂在 Operator 上，backend 由 Router 选

**事实**：`Operator.determinism` 是**算子级**字段（`crates/continuum-operator/src/definition.rs:84`），
而 §245 `:756` 明写「同一个 Operator MAY 有多个 backend」，具体后端由 Router 决定（§245 `:767-771`）。
复用判定 `can_reuse`（`crates/continuum-graph/src/reuse.rs:35`）读的是 **`operator.determinism`**，
不看 backend。故一个声明 `Deterministic` 的算子，若它的候选集合里混进一个输出不逐位可复现的 backend，
**复用就会给出一个与重算不同的结果，而这一点在类型层不可见**。

**本块的处置（注册期判据，两侧都钉）**：

- 本块持有一份**「逐位可复现的 backend」名单**（封闭表，内容是本设计定的）：
  `local-fs` `ffmpeg` `ffmpeg-scene-detect` `ffmpeg-filtergraph` `ffprobe` `pyscenedetect` `librosa` `essentia` `builtin`。
- **注册期规则**：传给 `register_media_operators` 的每一枚 `determinism == Deterministic` 的算子，
  其 `backend_candidates` **逐个**必须在该名单内；否则该函数返回
  `MediaError::UnverifiableDeterminism { operator, backend }`（§5.1 的三臂之一），
  且**在写注册表之前**返回，不留下半注册。
- **偏离最显然写法处**：最显然的写法是给 `render` 的候选里放一个硬件编码器（性能更好）。
  **这里不是那样：因为硬件编码器的输出不保证与软件编码器逐位一致**，混入即让该算子的 `Deterministic`
  声明为假，而那条声明的消费者是 §305 的复用。代价是硬件加速不可用——**这是性能代价，不是正确性代价**，
  与「复用了一个不同的结果」相比取向明确。

**这条规则的规范来源：没有。** §245 只给「一个 Operator 多 backend」，不给 backend 的确定性。
故名单本身是本设计定的，并**据实记为缺口**（§14 第 7 条）：真正的判据是「该 backend 的什么属性使输出逐位可复现」
（版本冻结？种子？浮点确定化？），规范未给。

## 5.5 §32 链尾两步是 verifier，不是「证据产出算子」的分界

§32 `:1384` 的 `Deterministic Verify` 与 `:1386` 的 `Multimodal Verify` 落在本块的两枚算子上（§5.2 第 16、17 行），
而 §262（`docs/spec/05-normative.md:1169` 起）的五级序把「确定性检查器」与「独立模型验证器」列为第 1、3 级。
两者的分界写在下面，两侧都不许绕（切分 §四第 1 条）：

| 事 | 归属 |
|---|---|
| Verifier 的**级别序**、候选过滤、独立性判定、隔离输入面、判定的聚合 | **P5a**（其设计 §7） |
| 媒体域的**比较器**（读 `Render` 的元数据与像素，给出「这一步成不成」的判定） | **本块**（§5.2 第 16、17 行的算子） |
| 后者怎样变成前者的一个候选 | 装配方以**值**构造 `AvailableVerifier`（P5a 设计 §7.2 的形状），`level` 取本设计声明的那一级 |
| 算子的产物怎样变成 `Evidence` | **宿主（第 3 层）**在算子产出制品之后直接调 P5a 的 `Evidence::from_tool_result`；本块只声明形状（§9.2：`subject` 取 `Unattached`；臂见 §9.1），**不写这一层的函数** |

**本块声明的事实**：`verify-deterministic` 在 §262 的序里是第 1 级（`DeterministicChecker`）、
`verify-multimodal` 是第 3 级（`IndependentModelVerifier`）。**这是本设计的声明**——§32 只说这两步在链尾，
没说级别；本设计按 §262 的两级定义对应，并把「这一对应要不要由装配方持有」记为对账条目（§13 第 5 条 (ii)）。

---

# 6. §131 派生产物的复用：**不建第二份判据**

## 6.1 事实：§305 的判据已由 P1 落地

- §131（`docs/spec/02-positioning.md:1949-1968`）列 `Video ├ Transcript / ShotIndex / FaceIndex / AudioFeatures / Embeddings`
  为一组 Derived Artifacts，并在 `:1968` 逐字写「原始素材没变时无需重新计算」。
- §305（`docs/spec/05-normative.md:1986-1998`）给的是同一条的三条件形式：
  `input_hash unchanged` / `operator_version unchanged` / `contract unaffected` ⇒ `Runtime MAY 直接复用 Artifact`。
- **它已经落地**：`crates/continuum-graph/src/reuse.rs` 的 `CacheKey`（`:9-12`）、`cache_key`（`:16-24`）
  与 `can_reuse`（`:29-47`），带八条用例（`crates/continuum-graph/tests/reuse.rs:23-94`，
  `#[test]` 在 `:23` `:29` `:38` `:50` `:60` `:70` `:80` `:90`）。
  `cache_key` 对 `NonDeterministic` 的算子返回 `None`（`:22`）——这是 ENG-002 的裁定，
  即「非确定算子的输出不写缓存键，因此永远不满足复用前提」。

**故：本块不得再定义第二个复用判据。** 本仓对「同一件事两个词汇表」一贯判为 Critical；
切分 §四第 1 条与 §四第 2 条把「只产出一份」的口径写给了证据与 ArtifactType 两处，
**复用判据是同一形状的第三处——切分文档 2026-10-09 已把它写成 §四第 7 条**（本设计定稿时没有；
当时 §五 P5e 行把 §131 列为本块的义务，§八 没有指出 §305/P1 已有落点）。**这是切分文档当时的一处漏项**，记在 §15 第 1 条。

## 6.2 本块的兑现：让本领域算子满足那条判据的前提

§131 的兑现**不是**一个函数，而是**本领域算子的 `determinism` 声明**。判据（照片，§12.2 第 (r) 条）：

| 算子 | 它产的派生产物（出处见格内） | 本块的 `determinism` | `cache_key` 的结果 |
|---|---|---|---|
| `shot-detect` | `ShotIndex`（`:1956`） | `Deterministic` | `Some(..)` |
| `audio-analysis` | `AudioFeatures`（`:1958`）、`beat map`（§33 `:1409`） | `Deterministic` | `Some(..)` |
| `metadata-analysis` | `metadata analysis`（§33 `:1410`） | `Deterministic` | `Some(..)` |
| `transcribe` | `Transcript`（`:1955`） | `NonDeterministic` | `None` |
| `vision-analysis` | `FaceIndex`（`:1957`）、`Embeddings`（`:1959`） | `NonDeterministic` | `None` |
| `proxy-build` | `建立 proxy`（§33 `:1404`） | `Deterministic` | `Some(..)` |

**表头原先写「§131 里它产的派生产物」，而表里有两行不是 §131 的条目**：`proxy-build`（`建立 proxy`）
与 `metadata-analysis`（`metadata analysis`）出自身为**§33 的步骤**（`:1404` / `:1410`）。§131
（`docs/spec/02-positioning.md:1954-1959`）只列 Video / Transcript / ShotIndex / FaceIndex / AudioFeatures /
Embeddings。表头据此改为此形，两行的出处照实留在格里，不并进 §131 的清单。

**`FaceIndex` / `AudioFeatures` / `Embeddings` 三者的 Artifact 类型**：§131 的五个名字里，
只有 `Transcript` 与 `ShotIndex`（= `ShotSet`）在 §3.2 的清单内有对应变体；
其余三个**不在三份 Artifact 类型清单的任何一份里**。本设计把它们落成 `ArtifactType::Json`
（§5.2 第 5、6 行的 `output_schema`）。**这是本设计的判定，不是规范的**——据实记在 §14 第 7 条。

**`ShotIndex` 与 `ShotSet` 是同一个东西**：§131 `:1956` 叫 `ShotIndex`，§10 `:541` 与 §328 `:2415` 叫 `ShotSet`。
本设计取 `ShotSet`（它在**规范正文** §328 里，而 `ShotIndex` 只在 §131 的示意图里），
**不新增第二个变体**。**这里不是「两个名字各收一型」：那会让同一个东西有两个 `ArtifactType` 取值，
而 §239 `:654` 的连接判定按等值比较（`crates/continuum-port/src/port.rs:101`）——两型即两个端口连不上，
且失配方向是静默拒绝。**

## 6.3 模型型派生产物一侧的缺口（`Transcript` 与 `Embeddings`）

**据实记录**：§131 `:1968` 说这些派生产物「无需重新计算」，而本设计把 `transcribe` 与 `vision-analysis`
声明为 `NonDeterministic`，于是 §305 的复用对 `Transcript` / `FaceIndex` / `Embeddings` **不成立**。

**两侧代价，都写清**：

- **误判为 `Deterministic`**（本设计不取）：`can_reuse` 会在输入未变时直接复用上一轮的 `Transcript`，
  而重算可能给出**不同的转录文本**（模型解码的随机性）。这不是性能问题，是**静默地让一份不同的结果
  冒充这次的结果**，且它发生在「输入没变」这个最不容易被怀疑的条件下。
- **误判为 `NonDeterministic`**（本设计取）：§131 对这三类产物的「无需重新计算」不成立，
  每轮都重跑模型。代价是算力与等待时间。

**取向的理由是 §305 自己的措辞**：该节给的是「Runtime **MAY** 直接复用」——一个 **MAY**，
即「允许复用」而非「可以跳过重算的保证」。而 §131 的「无需」是**定位文档里的成本陈述**（`docs/spec/02-positioning.md`），
不是规范正文里的 MUST/SHOULD。故**判据的权威在 §305 一侧**，本设计按它取 fail-closed。

**缺的是哪一步（写到步骤）**：判据是「这个 backend 的输出在相同输入与相同模型版本下是否逐位可复现」，
而规范没有给任何一处可据以回答它的东西——需要**「模型输出的可复现性条件」这一步**
（含温度/种子/版本冻结三项是否构成充分条件）。规范里最接近的是 §22 的行为指纹与 §82，
但两者都讲**模型是否被换掉**，不讲**同一模型同一输入是否给同一输出**。
**收件人：规范维护者 + 协调者**（§14 第 3 条）。

**`Transcribe` 的混合候选是一处必须分开读的地方**：§245 `:761-764` 照录的三个 backend
（`local-whisper` / `cloud-model` / `specialized-asr`）里，只有本地那一个有可能被证明是可复现的。
本设计**不把 `transcribe` 拆成三个算子**（§245 的整条主张就是「ADFIR 只表达 `Transcribe`，具体 backend 由 Router 决定」），
故这三个候选**同属一枚 `NonDeterministic` 的算子**——这是 §5.4 那条规则的**同向应用**
（混装会让 `Deterministic` 的声明为假），只是方向相反：这次是把整枚算子压到严的那一侧。

---

# 7. §34 生成内容单独授权

## 7.1 授权旗标：五臂，逐臂照录

§34（`docs/spec/01-concepts.md:1417-1431`）：`:1419` 逐字「剪辑与生成必须分开授权。」，
`:1421`「Task Contract 可以定义：」，`:1423-1429` 逐行给出五个旗标，`:1431` 逐字
「系统不得擅自使用生成内容替换原始素材。」。

```
/// §34 的五个旗标（docs/spec/01-concepts.md:1424-1428），逐条照录。
// Debug：`MediaError` 自身 `derive(Debug)`，且其 `NotPermitted` 臂以 `{required:?}` 引用本型。
#[derive(Debug)]
pub enum GenerativePermission {
    Broll,               // allow_generated_broll
    Voice,               // allow_generated_voice
    Music,               // allow_generated_music
    FrameInterpolation,  // allow_frame_interpolation
    ThreeDGeneration,    // allow_3d_generation
}

/// Contract 里**已声明为真**的旗标集合。以**值**从 Contract 传入（P4 的规则：凡跨层输入以值传入）。
pub struct PermittedGenerativeContent { /* ... */ }
```

## 7.2 判定：算子 → 所需旗标的映射，与 fail-closed 的默认

```
/// 本领域的生成类算子各自需要哪一枚旗标。
/// 返回 `None` 表示该算子不是生成类（不触 §34 的约束）。
pub fn required_permission(operator: &OperatorId) -> Option<GenerativePermission>;

/// §34「系统不得擅自使用生成内容替换原始素材」的判定点。
/// 未声明 ⇒ 未授权（fail-closed），返回 `MediaError::NotPermitted { operator, required }`
/// （§5.1 的三臂之一，`required` 取 `required_permission` 给出的那一枚）。
pub fn authorize_generative(
    operator: &OperatorId,
    granted: &PermittedGenerativeContent,
) -> Result<(), MediaError>;
```

**映射（四条，逐条对应 §5.2 的四枚生成算子）**：
`generate-broll` → `Broll`；`generate-voice` → `Voice`；`generate-music` → `Music`；
`frame-interpolation` → `FrameInterpolation`。

**`ThreeDGeneration` 这一臂今天没有算子映射它**：§34 的旗标与算子不是一一对应（旗标是 Contract 的词汇，
算子是执行单元）。本设计**照录五臂**（`:1424-1428` 是规范正文的五项），并把「哪枚算子需要它」记为
与 §1.2 表里那条 3D 归属缺口同源的事。**不给它编一个算子**：编一个就落进 §八「`3d/` 在四个具名算子里没有对家」那条无解里。
**这不是死臂**：它是 `GenerativePermission` 的一个取值，而 §34 明写它可以出现在 Contract 里；
一个 Contract 声明 `allow_3d_generation` 而系统里没有 3D 生成算子，是**系统不提供该能力**，
不是「这一臂没人用」。

**fail-closed 的判据（两侧都钉，§12.2 第 (j)、(k) 条）**：未声明 `allow_generated_broll` 时
`authorize_generative(&generate_broll, &granted) == Err(..)`（反例）；声明为真时 `Ok`（正例）。
**这里不是「用 `unwrap_or(true)` 让缺省放行」：**§34 `:1431` 逐字是「不得擅自」，
而 `PermittedGenerativeContent` 的**缺省构造**若是「全放行」，那么一个没写旗标的 Contract
就自动获得全部生成权限——那正是「擅自」的形态。

## 7.3 调用点不在本块

§34 的判定**在**本块（§7.2 的两个函数），**调用点不在**：谁在 `Queued → Running` 前调它，
属第 3 层执行器（切分 §四第 4 条：P5 不得自建执行路径；§二第 5 条：本块不碰 `execution_policy`）。
**据实记录**：`authorize_generative` 在 P5e 落地后**没有生产调用方**，与 §4.5 的 `checkpoint()` 同形。
两条合并成 §13 第 1 条的对账条目。

**与 P4/P5a 的分界**：§225 的 Constraint Validator 属 P4（它判 Contract 的内部一致性）；
「生成内容是否被授权」是**一次节点执行的前置条件**，不是 Contract 的一致性，故它落在本块，
而它的**输入**（哪些旗标为真）从 Contract 以值传入。**这是一条读法**，若协调者判它属 P4，
改动是「§7.2 的两个函数移出 + §5.2 的四枚生成算子改为只声明 `backend_candidates`」，
算子的 determinism 与端口类型不变。

---

# 8. §329 Timeline 的形态

§329（`docs/spec/05-normative.md:2426-2443`）给 `Timeline` 的六个字段（`:2430-2435`：
`tracks[]` / `clips[]` / `transitions[]` / `effects[]` / `audio_mix` / `metadata`），
并在 `:2439` 与 `:2442` 两处逐字重申「Timeline 属于普通 `Artifact<Timeline>`」。
§32 `:1395` 补一句「而不是单独 IR」。

**落点，三条**：

1. **`Timeline` 是 `ArtifactType` 的一个变体**（§3.2 第 4 行），不是新类型、不是单独 IR。这就是 §329 `:2442`
   与 §32 `:1395` 的直接兑现：`timeline-compose` 的 `output_schema` 是 `[Timeline]`（§5.2 第 9 行），
   而它下游的 `effects-subtitle` 与 `render` 的 `input_schema` 含 `Timeline`。
2. **六个字段的载体是 `Artifact` 的既有字段，不新增列**：`Artifact` 的
   `metadata: Value` 与 `provenance: Value`（`crates/continuum-artifact/src/artifact.rs:177-178`），
   以及内容本身的落盘（`crates/continuum-artifact/src/blobstore.rs:27-30` 的 `BlobStore`，路径按
   `content_hash` 分桶）。**故 §329 不产生迁移**（§11）。
3. **字段级 schema 本设计不定义**。§329 给的六行是**字段名**，没有给每一字段的类型、没有给
   「tracks 与 clips 怎么互相引用」、没有给 `audio_mix` 的表示。**本块不发明它们**——
   发明一份 Timeline 的序列化格式，就是替规范定了一个版本化的外部接口，而 §329 只给了六个名字。
   **缺的是「Timeline 六个字段的类型与它们之间的引用关系」这一步。收件人：规范维护者**（§14 第 4 条）。
   **互指**：这与 P5a 设计 §15 第 9 条（`Artifact.metadata` 的 schema 未给，
   `crates/continuum-artifact/src/artifact.rs:177`）**是同一处缺口的两半**——P5a 记的是「谁读 metadata」，
   本设计记的是「Timeline 的六个字段写什么进去」。两处都留，互为指针。

**一处留档**：`verify-deterministic` 的 `input_schema` 含 `Timeline`（§5.2 第 16 行）而不只是 `Render`，
理由是 §32 `:1384` 的这一步要在时间轴上核「渲染结果与时间轴一致」（例如时长、字幕轨的存在性）。
**这是本设计的判定**——§32 只给了步骤名。

---

# 9. 与 P5a 的接缝：只产出、只消费

## 9.1 两枚证据类型，不请求新臂

切分 §四第 1 条：四个算子块**只产出**（把节点输出转成 `Evidence`）、**只消费**（接受 P5a 的判定结果），
**不得**自定义证据类型、自定义「充分」判据、自定义完成判定。

P5a 设计 §14 第 3 条（`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md:997`）
点名问本块：「`EvidenceType::VisualCheck` / `Benchmark` 是否够 P5e/P5f 用，若不够须向本块报出」。
**本设计的答复：够，不请求新臂。**

| 本块的产出算子 | 用的 `EvidenceType` 臂 | 判据 |
|---|---|---|
| `verify-deterministic` | `VisualCheck` | §32 `:1384` 的这一步的判据是**对可观察制品的检查**（渲染结果的元数据与像素），与 §258 的 `VISUAL_CHECK` 同物 |
| `verify-multimodal` | `VisualCheck` | 同上；「跨模态」改变的是检查的**输入面**（多一个字幕轨），不改变证据的**类型**——它仍是「对制品的可观察检查」 |

**一处据实的不适**：`VisualCheck` 这个名字读起来偏视觉，而 `verify-multimodal` 的输入含音频与字幕。
本设计**不改名、不加臂**（P5a 持有那个枚举，且 `VISUAL_CHECK` 是 §258 的措辞），只把不适记在此处；
若 P5a 的下一轮判它需要一枚更宽的臂，那是 P5a 的一次改动，本块的两个调用点随之改一行。

## 9.2 产出的形状：一律 `Unattached`（**本块不写域包装**）

**跨块口径（协调者 2026-10-10 裁定，写在 P5a 设计 §4.4）：四个领域算子块一律不写域包装，
一律直接调 `Evidence::from_tool_result`。** 本块是四块之一：§9.1 的两枚验证算子各产一条证据，
构造由**宿主的执行代码**（第 3 层）在本块的算子产出制品之后，直接调 P5a 的 `from_tool_result` 完成。

**原先的定义（留着来历，2026-10-10 删）**：

```
/// 把一个媒体算子节点的输出转成 `Evidence`（§89）。
/// **唯一产生点是 P5a 的 `Evidence::from_tool_result`（P5a 设计 §4.4），本函数只是它的领域包装**：
/// 它固定的只有一件事——`subject` 取 `EvidenceSubject::Unattached`。
pub fn evidence_from_media_output(
    id: EvidenceId,
    evidence_type: EvidenceType,
    claim: Claim,
    producer: EvidenceProducer,
    artifact_refs: Vec<ArtifactId>,
    strength: EvidenceStrength,
    scope: EvidenceScope,
) -> Result<Evidence, VerifyError>;
```

**删的是哪一层**：它**只是转发**——签名与 `from_tool_result` 逐项对齐（上列七项就是那个构造点的八个参数
**减去**它固定的 `subject`），无一项域内的形状转换。删去后本 crate 的 `pub` 面少一枚，
`continuum-verify` 随之由普通边改判 **dev 边**（§2.1）。**这一层删掉不等于「没有地方构造证据」**：
构造点是 `pub`，宿主直接调它。**P5d 的同形包装与本次一并删去**（其设计 §5.2）。

**保留下来的四条事实（它们不依赖那一枚函数）**：

- **`subject` 取 `Unattached`**，不是 `Requirement(..)`。理由借 P5a 设计 §4.3.1 自己的那条：
  「证据先于归属存在：工具节点产出一条证据时，它可能还不知道（或不该由它决定）面向哪条 Requirement」。
  附带的一个后果是**本块不需要 `RequirementId`**（属 P4 的 `continuum-semantics`，该 crate 今天不存在），
  故 §2.1 的边表里没有它。**这一条现在由宿主的调用点兑现**（传 `Unattached`），本块不再以签名固定它。
- **`producer` 由调用方以值给**，取 `EvidenceProducer::Node { node, backend }`（P5a 设计 §4.3.3 的两臂）。
  **这里不是「收 `NodeId` + `BackendId` 再就地拼出 `EvidenceProducer::Node`」：`NodeId` 定义在
  `continuum-graph`（`crates/continuum-graph/src/ids.rs:40`，经 `crates/continuum-graph/src/lib.rs:22`
  re-export），`pub fn` 的签名里点名它就把该依赖从 **dev 边**升成普通边，而本块的生产代码不查图
  （§2.1 第 5 行：那条边是 dev 边，只有 §6.2 的照片用它）。**
  **据实记一处：本块拦不住 `EvidenceProducer::Human`**——两臂的搭配由 P5a 的构造点判（P5a 设计 §4.4
  只拦「`HumanConfirmation` 与 `producer != Human`」这一对），故「本块的产出都是 `Node`」这一句是
  **声明级**的，不是类型级的。收成类型级要本块自己写一遍那两臂的搭配判定，那是把 P5a 的判据抄成
  第二份（§2.2 的「本块不持有的判定」），故不取。**删去包装后，连承载那条声明的参数也没有了**：
  声明留在 §9.1 的表里，兑现方是宿主的调用点。
- **时刻不在产出面里。** `Evidence` **没有时间字段**——P5a 设计 §4.1 逐字段照 §258 的八字段，
  无一是时刻。P5a 的 `occurred_at` 是 `evidence` **表**的一列（P5a 设计 §10.2），由写入那一行证据的
  落库方给，不是 `from_tool_result` 的参数；本块零迁移、不写那一行（§11）。**原先那一版收
  `occurred_at: i64`**，它与构造点对不上（`from_tool_result` 无此参数），故删。
- **本块不判定任何东西**：产出面只声明「这一步产一条证据、臂是哪一枚」，不判「这条证据够不够」。
  「充分」的判据不存在（OPEN-001，P5a 设计 §3 的处置）——本块**不绕开也不补**。

## 9.3 消费面：本块不调用 P5a 的判定

`VerificationPolicy`（节点级）、`coverage`（§6）、`judge_completion`（§8）**都不在本块的调用集合里**。
本块的消费止于「两枚验证算子的级别声明」（§5.5）——那是一条**声明**，不是一次判定调用。
**据实记录**：本块与 P5a 之间的实际数据流是**单向**的（本块产出 Evidence、声明 verifier 级别），
反向的那一半（P5a 判定 → 本块）今天没有承载物，因为执行器未建（§12.5 第 1 条）。

---

# 10. 与 P5b 的接缝：`bind`

P5b 设计第六节（`:371-390`）要求四块**只经 `bind` 填 `realized_by`**，且 `video/` 的四条逐条非空
（P5b 设计 §4.5 `:307-314`：`has_counterpart(Video) == true` ⇒ 空即「漏 bind」）。本块填出：

| `MethodId`（§187，`docs/spec/04-method.md:127-130`） | `realized_by`（本块的算子 id） | 理由 |
|---|---|---|
| `shot-analysis` | `["shot-detect", "vision-analysis"]` | §33 `:1405` 的 shot detection 与 `:1407-1408` 的 face/object indexing、visual embedding 都是「对镜头内容作出判定」的步骤 |
| `narrative-plan` | `["narrative-plan"]` | §32 `:1376` 的 Narrative Planning，与 §187 的 `narrative-plan` 同名同物 |
| `timeline-compose` | `["timeline-compose"]` | §32 `:1378` 的 Timeline Artifact；产物是 `Timeline`（§329） |
| `render-review` | `["verify-deterministic", "verify-multimodal"]` | §32 `:1384`、`:1386` 的链尾两步是「审看渲染结果」 |

**`bind` 的参数是文本**（P5b 设计 §4.3 的 `realized_by: Vec<String>`），故上表的值是字符串，
**本块无法在编译期核对它们与 `all_operators()` 的 id 一致**（P5b 设计 §3.2 末已把这条弱引用的无照片写清）。
本块的判据是**运行期的一条用例**（§12.2 第 (s) 条）：对每一条 `realized_by` 里的每个串，
`OperatorRegistry::resolve` 必须成功——这条用例**在本块的 crate 里闭得上**，因为本块同时持有
`all_operators()` 与 `MethodRegistry`。**这是 P5b 所说「消费块才能核」的那个消费块之一。**

调用 `bind` 的**具名入口**（此前全文只有类别名「本块的登记代码」，无一处签名收 `&mut MethodRegistry`）：

```
/// 把上表四条 `realized_by` 填进方法库（P5b 设计第六节 `:375`：每块对其领域目录里的**每一条已 seed 的 id**
/// 调用一次 `bind(id, &[...])`）。`video/` 四条逐条非空（P5b 设计 §4.5：`has_counterpart(Video) == true`）。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
pub fn bind_media_methods(registry: &mut MethodRegistry) -> Result<(), MethodError>;
```

**错误类型不并进 `MediaError`**：`MediaError` 的三臂是注册期规则（§5.4）、注册表查重（P1 的 `OperatorError`）
与生成授权（§7.2）；本函数的失败是「方法库拒绝了这次 `bind`」（`MethodError::NotFound`），判定、收件人与
处置都不同，故直接透出——这与 §5.1 把 `OperatorError::Duplicate` 包进 `MediaError::Registry` 是同一条判据的
两面：**判定是同一件事则合并，是两件事则分开**。
**本函数不做二次登记**：不 `register` 方法条目（P5b 设计第六节：四块**不得**自造 `MethodEntry`），
`video/` 的四条 id 由 P5b 的 `seeded()` 给出，本块只把这四条「每一条已 seed 的 id」逐条 `bind` 一次。

**§33 的七个预处理算子没有目录可填**：§187 的 `video/` 只有四条（`:127-130`），而 §33 `:1403-1411`
有七项预处理。故 `proxy-build` / `audio-analysis` / `metadata-analysis` 等算子**不在任何方法的
`realized_by` 里**，这不是漏 bind。**§33 与 §187 的 `video/` 不是同一张表**：前者是链上的步骤，
后者是方法名。本设计**不给 §187 补条目**（P5b 设计 §4.2：`MethodId` 开放，但补一个规范无来源的
方法名会动到 P5b 的裁决——P5b 设计第八节 R3/R6/R9）。

---

# 11. 持久化：**本块零迁移**

## 11.1 结论

**本块不建表、不建迁移、不写事件。** 逐项给理由：

| 本块的产物 | 为什么不落库 |
|---|---|
| 17 枚 `Operator` | `Operator` 今天**没有持久化路径**：`continuum-operator` 无 `persist.rs`（实测该 crate 的 `src/` 只有 `definition.rs` `registry.rs` `lib.rs`），注册是**装配期在内存里**做的事（`OperatorRegistry` 的 `entries: HashMap`，`registry.rs:16`）。本块照此，不新增存储 |
| 11 枚新 `ArtifactType` 变体 | 它落进的是**既有的 TEXT 列**：`crates/continuum-artifact/src/persist.rs:11` 的 `artifact_type TEXT NOT NULL` 与 `crates/continuum-graph/src/persist.rs:42` 的 `artifact_type TEXT NOT NULL`——两列都**没有 CHECK 约束**，新取值写入即成立。**加一枚枚举变体不产生 DDL** |
| Timeline 的六个字段 | 落进既有的 `artifact.metadata` / `provenance`（TEXT）与 `BlobStore` 的落盘（§8） |
| §34 的授权旗标 | 住在 Contract 里（§34 `:1421` 逐字「Task Contract 可以定义」），Contract 表属 P4 |
| §131 的复用凭据 | `CacheKey`（`crates/continuum-graph/src/reuse.rs:9-12`）今天**也没有存储**（实测：`cache_key` / `can_reuse` / `CacheKey` 在 `crates/` 下的非测试命中只有 `reuse.rs` 与 `lib.rs:28` 的 `pub use`）。它的存储属 §305 的落点即 **P1**（P1 设计 §18 `:568-590` 把 `can_reuse` 列在「无执行点的机制」里）。**本块不替它建表** |
| 检查点 | §4.6：`Checkpoint` 是返回值，存储属第 3 层的恢复路径 |

## 11.2 实测占用表（判据在切分 §二第 5 条）

**实测命令**：`grep -rn "Migration::new(" crates/`，逐处读**调用的第二个实参**（号在参数表的次行或同行）。
实测结果（工作树 `p5e`，2026-10-09）：

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
| `100` `101` `102` | `crates/continuum-persist/tests/migrations.rs:45` / `:68` / `:69` | **否**（测试夹具；P4 设计 `:296-299` 已把这条口径写明） |

「是」的九处即 `crates/continuum-runtime/src/main.rs:98-107` 的 `runtime_migrations()` 的装配集合
（`:100-106` 逐行追加七个 crate 的迁移，加上 `:99` 的 `builtin_migrations()`）。

**协调者已裁定的号段**（P3 那一轮）：A=`50`、B=`60`、C=`70`、D=`80`、E=`90`
（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:480` 逐字
「一个子项目一个十位档」）。P4 取 `100/110/120`（P4 设计 `:2052`）。**P5a 已取 `130`**（切分 §二第 5 条）。

**实测空档**（十位档）：`140` `150` `160` `170` `180` `190` 六个档今天在**运行时装配链**里全部为空
（上表「是」的九处取值为 `1 2 10 20 30 40 41 50 80`，无一落在 `130`–`199`；测试夹具占的 `100/101/102`
不进装配链，P4 设计 `:296-299` 已把这条口径写明）。

## 11.3 本块的号段处置

**本块不占档**——零迁移，故没有号可占。**这是本设计的一处刻意决定，不是漏项**：

- 按本仓的既有取法，「预留一个没有表要建的编号就是留一条死迁移」
  （`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:482-485` 逐字：「三张表全在 `80` 里，
  `81` 当下**没有使用点**，预留一个没有表要建的编号就是留一条死迁移；而按本仓既有的分工，
  「用它的那个 task 自己注册迁移、自己取号」本来就不需要预留。将来真有新表时，那一刻取一个未占用的号即可」）。
- **若 P5e 的实现阶段确需落库**，那一刻取号即可，判据是**未占用**（同 p3d 的取法，
  该文 `:487-493`：「『未占用』的判据是按库说的，不是按全仓说的」，
  并说明 `60` 与 `50` 各在别处自建的库里出现过而无害）。**本设计给出的次序建议**是
  `170`（P5 六块按 P3 的先例后延：P5a `130`、P5b `140`、P5c `150`、P5d `160`、P5e `170`、P5f `180`，P6 `190`）
  ——**这是建议不是裁定**，六块与 P6 的档由协调者统一划（切分 §二第 5 条）。
  **（2026-10-10 订正）领句原先写的是 `140`，与同一句括注相抵**：括注把 `140` 分给 P5b、`170` 分给 P5e，
  平行段落 P5f 设计 §10.3 的领句写的正是自己的号（`180`）。故领句改为 `170`；
  P5c 设计 §9.3 与 P5d 设计 §8.3 引的都是那个括注（两处引用不受影响）。

**收件人：协调者**。**这一节只是一句注记，不另开对账条目**：本块零迁移故无号可占，
号段怎么划与本块无关（原先在 §13 与 §14 各开过一条「迁移号段」的对账项，已删——
本块不参与那件事，开条目是替别人的事记账）。

---

# 12. 测试策略与照片

## 12.1 枚举与清单的逐臂守卫

- `ArtifactType`：加型后 `ALL.len() == 17`，且**逐型**往返
  （`crates/continuum-artifact/tests/artifact_type.rs:21-43` 的既有用例改了数目断言后即覆盖新型）；
  **11 枚新串逐条断非空、小写、`_` 连接**（同用例 `:32-35` 的既有断言，遍历 `ALL` 即覆盖）。
- `CheckpointError`（3 臂）、`GenerativePermission`（5 臂）：各一条**臂数与往返**用例
  （`as_str` / `parse` 两枚穷尽 `match` + 一条数目断言，形状照 `ArtifactType` 的既有做法）。
- `all_operators()`：`len() == 17`，且 `id` 逐枚不同（两两比较，不是只查总数）。

## 12.2 判定侧：正例 + 反例成对（缺一条即不算钉住）

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| a | 11 枚新型的 `as_str` 串逐条 `parse` 回自身 | §3.2 的编码（正例） |
| b | 六个既有「表外取值」（`artifact_type.rs:40`）仍一律 `None` | 解码侧的**另一侧**（新串没把表外取值吞掉） |
| c | 新型写入 `artifact_type` 列再读回，得到同一型 | 落库往返（§3.5 的「不改但要重跑」） |
| d | 改过的 `crates/continuum-port/tests/compatibility.rs:47` 用例改为**遍历 `ArtifactType::ALL`**，逐型断言 serde 往返（17 型，不是 6 型） | §3.5 的 (e)（本块改的唯一一处**不会自己变红**的遗漏） |
| e | `shot_set` / `subtitle_track` 两个多词串的往返 | 多词型的编码（唯一两条带 `_` 的新串） |
| f | 以 `&all_operators()` 调 `register_media_operators` 注册 17 枚后，逐枚 `resolve` 成功 | 注册的正例 |
| g | 同一注册表注册两次 ⇒ `Err(MediaError::Registry(OperatorError::Duplicate { .. }))`（**不是**静默覆盖） | 注册的反例（§5.1：那一臂是 P1 的判定，本型只带出来） |
| h | `Deterministic` 的每一枚算子，其每个 backend 都在 §5.4 的名单内 | §5.4 的规则（正例） |
| i | 造一枚 `Deterministic` 但候选含名单外 backend 的算子，以它连同若干合法算子调 `register_media_operators` ⇒ `Err(MediaError::UnverifiableDeterminism { .. })`，**且注册表内容与调用前逐枚相同**（先核后写，不留半注册） | **fail-open 的那一侧**（§5.4）；「注册表未变」那一半钉的是同一条规则的**写入面** |
| j | 未授权 `Broll` 时 `authorize_generative` ⇒ `Err(MediaError::NotPermitted { .. })` | §34（反例侧） |
| k | 授权 `Broll` 时 ⇒ `Ok` | 上一条的**另一侧**（否则 j 可由「一律 Err」满足） |
| l | `required_permission` 对四枚生成算子各返回**对应的**那一枚（逐枚断言是哪一枚，不是「是 `Some`」） | §7.2 的映射（**枚举断言逐项有照片**：四臂各一条） |
| m | `required_permission` 对一枚非生成算子返回 `None` | 上一条的**另一侧** |
| n | `generate-broll` / `frame-interpolation` 的 `side_effect_class == Pure`，`render` 与 `proxy-build` 的是 `Idempotent`，`media-import` 的亦 `Idempotent` | §5.2 的第 5、6 条理由，逐枚 |
| o | `checkpoint()` 有进度 ⇒ `Ok`；`restore` 后状态与存前一致 | §4.5 判据 1 的正例 |
| p | 无进度 ⇒ `Err(NothingToCheckpoint)` | §4.5 判据 1 的反例 |
| q | 另一版本的检查点 ⇒ `Err(RestoreRejected)`，**且算子状态与调用前逐字段相同** | §4.5 判据 2（两侧：错误类型 + 半恢复） |
| r | §305 的照片：`shot-detect` / `audio-analysis` / `metadata-analysis` / `proxy-build` 四枚 的 `cache_key` 为 `Some`，且四条件齐时 `can_reuse` 为真；`transcribe` / `vision-analysis` 两枚为 `None` | §6.2 的六行**逐行**（不是抽样） |
| s | 四条 `video/` 方法的 `realized_by` **逐条非空**，且每个串 `OperatorRegistry::resolve` 成功（先经 `bind_media_methods` 填入，§10） | §10（P5b 设计 §4.5 交办的判据）+ 「绑的是不是对的算子」的前半 |
| t | §5.3 边表**逐行**：下游 `input_schema` 的每一型都能在该行列出的上游算子里找到，且那一枚的 `output_schema` 含该型（**交集中的型**，不是「两侧相等」） | §239 的注册期一半（§5.3） |

**一条预防**：第 (a) 与 (f) 的期望清单**必须手写**。若 (a) 写成遍历 `ArtifactType::ALL`，
它测的是「`ALL` 与 `parse` 一致」——那正是 `ALL` 的既有守卫，**而「11 枚新型在不在 `ALL` 里」不会被它测到**；
若 (f) 写成 `all_operators().len()`，它测的是 Rust 的 `len()`。**语料从被测清单里取就是恒真的假照片。**

## 12.3 变异预告（谁红）

四档，每档预告**哪些用例红、哪些不该红**（预告写反会让人去查不存在的问题）：

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`authorize_generative` 改成「未声明 ⇒ 放行」 | j | k（授权那一侧不受影响）、l、m |
| **取反**：§5.4 的名单判定改成恒真 | i 的第一个子例 | h（名单内的候选仍然通过） |
| **取反**：`register_media_operators` 改成边核边写（先注册前几枚，遇到违规才返回） | i 的第二个子例（注册表内容与调用前不同） | h、f（合法批次两侧都绿） |
| **移除**：`RestoreRejected` 的身份比对删掉（一律 `Ok`） | q 的第一个子例 | o、p（有进度/无进度与身份无关）、r 全组 |
| **收紧**：`required_permission` 对四枚生成算子**都**返回 `Broll` | l 的后三条 | j、k、m（它们不看具体是哪一枚） |
| **放宽**：`transcribe` 的 `determinism` 改成 `Deterministic`（**订正 2026-10-10**：原记「取反」） | r 的第二组（`cache_key` 由 `None` 变 `Some`）；**同时该轮 h 会红**（`local-whisper` 等三个候选不在 §5.4 的名单里） | r 的第一组、i |
| **放宽**：`all_operators()` 少注册一枚（16） | `len() == 17` 的断言、f 的对应枚 | 其余（这**一档若不红，说明 f 是按 `all_operators()` 自己遍历的**——即假照片） |
| **等价变异**：把某个新型的串改成另一个**同样合法**的小写串（如 `shot_set` → `shotset`） | **全绿**——这是一枚**等价变异体**（编码是自洽的，往返仍成立）。**要打红它必须换变异体**：改成与 `artifact_type.rs:40` 的表外取值相撞的串（如 `nope`），那时 b 红 | —— |

## 12.4 结构层的照片（`trybuild`）

本仓已有 `trybuild` 的既有做法（`crates/continuum-connector/Cargo.toml:53` 与其 `tests/type_level.rs`
的 `compile_fail`；`continuum-workspace` / `continuum-secrets` / `continuum-model-registry` /
`continuum-capability` / `continuum-node` 同样登记了它）。**本块用两条**：第 1 条钉的是本块在 `Evidence`
上的结构约束（在本块里造不出 `Evidence`）；第 2 条钉的是本块不构造 `Requirement` 归属，而它的成立还挂在
P5a 的 `RequirementId` 归属上（该条自承的那条假设）：

1. `Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现）在 `continuum-media` 里编不过
   ⇒ 在本块的 crate 里**造不出** `Evidence`（字段私有、无 `Default`／`From`），绕过构造点在类型上写不出来。
   这是 P5a 设计 §4.4 里「**唯一产生点**」那条**结构事实**在本块的**使用侧**照片。
   （**订正 2026-10-10**：原引 `:319`。**该处已不指向那句话**——P5a 的设计其后被改过（`bfe1df8`），行号随之移；
   按本仓体例，**指进别人会改的文档一律按内容指、不写行号**，故此处改为按内容指。）
   **这一条证不到「产出只能经本块的某枚函数」**：`Evidence::from_tool_result` 是 `pub`
   （P5a 设计 `:324`），构造由**宿主**直接调它完成（§9.2），**调用点不在本 crate 里**——
   **（2026-10-10 订正）原句写「本块的生产代码可以直接调它……只是不经过本块的包装」，
   而那一枚包装已按协调者裁定删去**（§9.2）；这两条 `trybuild` 用例据此成为本块在
   `continuum-verify` 上的**唯一使用点**，也是 §2.1 把该边判为 dev 边的理由。
2. `EvidenceSubject::Requirement(..)` 在 `continuum-media` 里构造 ⇒ 编不过（§9.2 的「本块不构造
   Requirement 归属」）。**这一条的成立依赖 P5a 不 re-export `RequirementId`**——该类型的归属 P5a
   自记未定（P5a 设计 §4.3.1：「落地时以 P4 的 `RequirementId` 为准」），故它是一条**对别人块形状的
   假设**，按本文件第 15-17 行的纪律记为对账条目（§13 第 5 条 (iii)）。

**原先列在这里的第一条已移出本块的照片清单**：原写「`Operator { .. , determinism: Deterministic }`
的构造不带 `backend_candidates` ⇒ 编不过」。那是 P1 的 `Operator`
（`crates/continuum-operator/src/definition.rs:78-88`，七个 pub 字段、无 `Default`）的**结构事实**，
不是本块造出来的照片——本块只是照它构造 `all_operators()`，故不计入本块的照片。

## 12.5 拍不出照片的地方

1. **端到端（一个媒体 Intent 从 `Queued` 走到某处）**：执行器（第 3 层）未建，
   且切分 §四第 4 条禁止 P5 自建执行路径。本块的全部照片都是**单元与集成层**。
   与 P5a 设计 §12.4 第 1 条同形。
2. **§34 的授权在真实执行路径上的效果**（未授权的生成算子被真的拦下不执行）：
   调用点属第 3 层（§7.3），故只能拍到 `authorize_generative` 的返回值。
3. **`checkpoint()` / `restore()` 的崩溃恢复效果**（进程被杀后从检查点续跑）：
   同上，无执行器（§4.5 的判据只拍到「函数返回值与状态不变性」）。
4. **§131 的复用真的省下了一次重算**：`can_reuse` 今天无生产调用方（P1 设计 §18 把它列在
   「无执行点的机制」里），且 `CacheKey` 无存储。故只能拍到「判据的返回值」（r 条），
   拍不到「少跑了一次」。
5. **Timeline 的六个字段**：schema 未定义（§8 第 3 条），故拍不到「一份 Timeline 建得出、读得回」。

---

# 13. 与其余各块的对账条目（逐条具名）

以下每条都是**本设计假设了别的块的某枚形状**或**发现某处无归属**之处。

1. **登记（不是裁定请求）：三处「有判定、无调用点」合并为一条，落点都在第 3 层的执行器**。
   本块有两处「有判定、无调用点」，加上切分 §八「算子解析的落点仍无人认领」已有的那一处，同源：
   (i) `checkpoint()` / `restore()`（§4.5）：属第 3 层的恢复路径（§311 / §312 那一组）；
   (ii) `authorize_generative`（§7.3）：属 `Queued → Running` 的前置判定；
   (iii) `OperatorRegistry::resolve` 的调用点（切分 §八 的「算子解析的落点无人认领」；P1 设计 §18 `:576-580` 逐字
   「执行器就位后，`Queued → Running` 是它唯一的合法落点」）。
   **三处都在同一处落点**（`Queued → Running`）。本块不自建（切分 §四第 4 条），
   也不把三处各自记一次。**收件人：第 3 层的执行器**（谁建执行路径谁承接这三处；协调者转）。
   **这一条不是待裁定的事**——落点已由切分 §四第 4 条与 P1 设计 §18 `:576-580` 定死，
   本块只是在册子上把三处并成一笔。
2. **待与 P1 对账**：三条。
   (i) `CheckpointError` 与 `CheckpointOwner` 落在 `crates/continuum-operator/src/definition.rs`，
   并改 `:95` / `:96` 两行签名（§4.2、§4.4）——**这是本块对 P1 文件的改动**——切分 §一 的共写表当时未登记它，2026-10-09 已把 `crates/continuum-operator/src/definition.rs` 的 `Checkpointable` 一行补入（§15 第 2 条）。
   (ii) `CheckpointOwner` 与 `OperatorRef`（`crates/continuum-graph/src/node.rs:42-45`）两字段相同，
   本设计**不合并**（合并会成环，§4.3）。若 P1 判 `OperatorRef` 应下移到 `continuum-operator`，
   本块的 `CheckpointOwner` 随之删掉、改用 `OperatorRef`。
   (iii) 复用判据（`crates/continuum-graph/src/reuse.rs`）的**存储与调用点**：`CacheKey` 今天无存储、
   `can_reuse` 今天无生产调用方（§11.1）。本块不替它建表（§6）。
3. **待与规范维护者 + 协调者对账（§6.3）**：模型型派生产物（`Transcript` / `FaceIndex` / `Embeddings`）
   的可复现性判据不存在，故本块把 `transcribe` / `vision-analysis` 声明为 `NonDeterministic`，
   §131 `:1968` 的「无需重新计算」对这三类产物不成立。**缺的是「模型输出的可复现性条件」这一步**。
   **互指（2026-10-10 补）**：本文件 §14 第 3 条记的是同一处缺口——同一条事实在本文件的「对账条目」与
   「遗留与未决项」两节各记一次，**两处都留，互为指针**（本处指它、那里指回来）。
4. **待与规范维护者对账（§8）**：§329 的六个字段没有类型、没有引用关系、`audio_mix` 无表示。
   **缺的是「Timeline 六个字段的类型与互引」这一步**。与 P5a 设计 §15 第 9 条（`Artifact.metadata` 的 schema）
   是同一处缺口的两半，两处互指。**同一处缺口本文件另记一次，在本文件 §14 第 4 条，两处互为指针**。
5. **待与 P5a 对账**（三条）：
   (i) 本块答复 P5a 设计 §14 第 3 条的询问：`VisualCheck` 够用，**不请求新臂**（§9.1），
   但「`VisualCheck` 这个名字对跨模态检查偏窄」记在 §9.1 末。
   (ii) 两枚验证算子的 §262 级别声明（§5.5：第 1 级与第 3 级）由本块给出，
   **装配方以值构造 `AvailableVerifier`**（P5a 设计 §7.2）。「这一对应由谁持有」未定。
   (iii) **本块假设 `RequirementId` 在 `continuum-verify` 的名字空间里不可命名**：§12.4 第 2 条的
   `compile_fail` 用例（在 `continuum-media` 里写 `EvidenceSubject::Requirement(..)` 编不过）靠它成立。
   而 P5a 设计 §4.3.1 记的是该类型「以**值**承载它…落地时以 P4 的 `RequirementId` 为准」，**归属未定**。
   若 P5a 的下一轮把 `RequirementId` re-export 出来（或本块按 P4 的类型名可命名它），那条用例当场不成立，
   须换掉（判据仍在：本块不构造 `Requirement` 归属，只是照片从「编译不过」变成别的形式）。
6. **待与 P5a 与协调者对账（检查点的身份约束）**：`RestoreRejected` 的判据要求检查点**自带身份**，
   而 `type Checkpoint`（`crates/continuum-operator/src/definition.rs:93`）今天只有 `Send + Sync` 约束。
   加一条身份约束改的是 trait 的**关联类型**，不在切分 §二第 4 条给本块的范围内（§4.6）。
7. **待与规范维护者对账（§5.3、§5.4、§6.2 三处封闭表）**：
   (i) §5.4 的「逐位可复现的 backend」名单——规范没有一处给 backend 的确定性；
   (ii) §6.2 的 `FaceIndex` / `AudioFeatures` / `Embeddings` → `Json` 的映射——三者不在任何一份 Artifact 类型清单里；
   (iii) §5.3 边表的**边判据**——规范给的是步骤名与步骤序（§32/§33/§34），**数据依赖边**是本设计按该行的
   `input_schema` 与全表 `output_schema` 的对应定的（§5.3 依据栏的读法）。逐行登记在此条的是以「§5.2 第 N 行」
   为依据的那 **8 行**（第 1、8、9、12、13、14、15、17 行），以及第 16 行 `verify-deterministic` 的
   `Timeline` 那一支（该行格内自标「本设计的判定」，§8 末）。
   三处都是**本设计定的封闭表**，规范无来源。
8. **待与 P5b 对账**：§10 的四条 `bind` 是本块填的；`realized_by` 的文本弱引用**没有编译期照片**
   （P5b 设计 §3.2 末已写明），本块的补件是 §12.2 第 (s) 条那条运行期用例。
   另：§10 末的「§33 的七枚预处理算子无目录可填」是**据实留的形状**，不是漏 bind。
9. **待与协调者对账（P1 设计的一处会变旧的文本）**：`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:573`
    逐字「ArtifactType 集合 本子项目取六种。P5 引入媒体类型时为编译期可见的破坏性变更。」。
    本块不改别人的文件（§3.5 的 (g)），**收件人：协调者**（决定要不要在该文件里补一句指向本设计）。

---

# 14. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **§131 的复用判据已由 P1 落地，本块不建第二份**（§6.1）。切分 §五 P5e 行原先只把 §131 列为本块义务，
   未指出这一点；切分文档现已改引 §305 并在补记二里写明复用条件已由 P1 落地
   （2026-10-09 已折入，来历见 §15 第 1 条）。**收件人：协调者**（明确本块的兑现口径是「让算子满足那条判据的前提」）。
2. **`Checkpointable` 的类型只能落在 `continuum-operator`**（§4.2）；切分 §一 的共写表原先未登记它
   （2026-10-09 已折入，来历见 §15 第 2 条）。**收件人：协调者 + P1**。
3. **模型型派生产物的可复现性判据不存在**（§6.3）。**缺的是「温度/种子/版本冻结三项是否构成
   「同输入同输出」的充分条件」这一步。收件人：规范维护者 + 协调者。**
   **互指（2026-10-10 补）**：本文件 §13 第 3 条记的是同一处缺口，**两处都留，互为指针**。
4. **§329 的 Timeline 六个字段无 schema**（§8 第 3 条）。**缺的是「六个字段的类型与 tracks↔clips 的引用关系」
   这一步。收件人：规范维护者。**
   **互指（2026-10-10 补）**：本文件 §13 第 4 条记的是同一处缺口（那里另与 P5a 设计 §15 第 9 条互指），
   **两处都留，互为指针**。
5. **`ThreeDGeneration` 这一臂今天没有算子映射它**（§7.2）：与 §1.2 表里那条 3D 归属缺口同源
   （切分 §八 的「`3d/` 在四个具名算子里没有对家」那一条）。**缺的是「3D 链归哪一块」这一步的裁定。收件人：协调者。**
6. **检查点的存储形态未定义**（§4.6、§11.1）：`Checkpoint` 的持久化、版本兼容、损坏检测都不在 §308 里。
   **缺的是「检查点写在哪里、谁写、怎么判损坏」这一步。收件人：规范维护者 + 第 3 层。**
7. **§5.4 的 backend 名单、§6.2 的 `Json` 映射与 §5.3 边表的边判据都是本设计定的封闭表，规范无来源**（§13 第 7 条）。
   边表那一处要单独读出：§32/§33/§34 给的是步骤名与步骤序，**数据依赖边**是本设计按端口类型定的（§5.3）。
   **收件人：规范维护者。**
8. **§32 链尾两步的 §262 级别是本块的声明**（§5.5）。**缺的是「verifier 候选的级别由谁持有」这一步。
   收件人：P5a + 装配方。**
9. **本块的两处登记入口、两处判定今天都没有生产调用方**：`register_media_operators`
   （§5.1）、`bind_media_methods`（§10）、`authorize_generative`（§7.2）、`checkpoint()` / `restore()`
   （§4.5、§7.3），四者。**这不是本块的缺口，是第 3 层的**
   （切分 §四第 4 条禁止 P5 自建执行路径；P1 设计 §18 已有同形的一条；拍照的限制见 §12.5）。
   **（2026-10-10 订正）**原先这里列五者，含「一处产出包装 `evidence_from_media_output`（§9.2）」——
   那一枚已按协调者裁定删去（§9.2），故由五者减为四者；证据的构造点（`from_tool_result`）在宿主侧，
   本块**没有**那一层的调用点可列。

---

# 15. 在切分文档里发现的错或缺口（**只记，不改**）

逐条给出实测。**本设计不改切分文档**（切分 §七 与协调纪律）。

> **本节各条已在同一分支上折入切分文档（2026-10-09，协调者裁并执行）**：切分 §一 的共写表增了
> `crates/continuum-operator/` 一行；§四第 2 条的「一次改动」清单增了 (e) 与 `:24` 的注记；
> §四 增了**第 7 条横切约束**（「判据只许有一处：凡本仓已落地的判据，四个算子块不得另建第二份」，第一枚实例即复用判据）；
> §五 P5e 行的依据栏增了 §9 与 §10 两处来源，并附两条补记（§9/§10 的漏列、§131 与 §305 的分别）。
> **本节保留的是「发现时的样子」**，其条目描述的是**改动前**的切分文档——读本节时以切分文档**现在**的文本为准。

1. **⚠️ §五 P5e 行把 §131 列为本块的义务，而 §131 的判据已由 P1 落地（§305 的 `can_reuse`），
   切分文档没有指出这一点，也没有把「不得建第二份复用判据」写成一条横切约束。**
   实测：`crates/continuum-graph/src/reuse.rs:29` 的 `can_reuse` 与 `:16` 的 `cache_key`，
   §305 在 `docs/spec/05-normative.md:1986-1998`。§四的横切约束里，
   「只产出一份」只写给了证据（第 1 条）与 `ArtifactType`（第 2 条），**复用判据是第三处而没有写**。
   后果：若不指出来，P5e 很自然地会新写一个「原素材未变 ⇒ 复用」的函数——那时仓库里有两套复用判据，
   而两套的失配方向是「一套说能复用、另一套说不能」（§305 的 `contract unaffected` 那一条最容易漏）。
   **判据**：「A 拥有 X 的判定」这句话，要先问「X 的判定在仓里有没有落点」——本行只核了规范来源（§131），
   没核代码落点。
2. **⚠️ §一 的共写文件表漏了 `crates/continuum-operator/`。** 实测：§二第 4 条把
   `Checkpointable` 的错误类型指给 P5e，而该接口的定义在 `crates/continuum-operator/src/definition.rs:92-96`，
   其错误类型**只能**定义在同一个 crate（否则第 3 层与第 8 层成环，§4.2）。
   故这是一处必然发生、却未登记的共写。**判据**：「把 X 定给某块」这句话，要同时核「X 定义在谁的 crate 里」。
3. **⚠️ §四第 2 条的「一次改动要同时碰这几处」漏了 `crates/continuum-port/tests/compatibility.rs:47` 的
   `six_types_round_trip_through_serde`。** 它手工列了六型（`:49-54`），
   **加型后照旧编译、照旧通过**——即它是清单里唯一一处**不会报警**的遗漏（§3.5 的 (e)）。
   **判据**：「一次改动要同时碰哪几处」这句话，要连「不会变红的处所」一起列——
   会编译失败的地方自己会喊，不会喊的那些要靠清单。
4. **⚠️ §四第 2 条与 §五 P5e 行都没有提 `§9 Typed Artifact`（`docs/spec/01-concepts.md:496-514`）。**
   §八「`ArtifactType` 的 P5 清单尚无权威来源」那一条说「本清单尚无权威来源」时，举的是 P1 设计 `:168`、§328、§323、§330 四处，
   **漏了这一处**——而它是本仓里**最完整的一份 Artifact 类型清单**（12 个名字，
   且 `:498` 逐字写「领域差异主要通过 Artifact 类型表达」）。
   它也是本设计清单里 `Mesh` 与 `Report` 两型的**唯一**出处（§3.2 第 5、8 行）。
   **判据**：「尚无权威来源」这句话，要先穷尽搜索同一件事的既有清单——
   `grep -n "Artifact<" docs/` 一次就能找到它（实测命中 **25 处**：`docs/spec/01-concepts.md` 13 处、
   `docs/spec/05-normative.md` 9 处、`docs/spec/03-product-drive.md` 1 处、`docs/01-总纲.md` 2 处）。
5. **⚠️ §五 P5e 行与 §八 都没有提 `§10 Domain Operator`（`docs/spec/01-concepts.md:536-560`）。**
   它逐行给出三枚媒体算子的**签名**：`ShotDetect(Video) -> ShotSet`（`:541`）、
   `Transcribe(Audio) -> Transcript`（`:543`）、`ComposeTimeline(...) -> Timeline`（`:545`）——
   即算子**名与端口类型**的现成来源。§五 P5e 行的「主要规范依据」栏里没有它。
   **判据**：同第 4 条。
6. **✔ 已复测为正确的两条**（记在此处是为了让复审不必重做）：
   (i) §一 说的「工程 §8.1 组件表里的四行（`docs/02-工程.md:495-498`）」——实测 `:495` 是「代码领域算子」、
   `:496` 研究、`:497` 媒体、`:498` 图像，四行连续，切分的那一句准确。
   (ii) §一 说的「§8.1 第 13 行」是媒体领域算子——实测表头在 `:483`，`:485` 是第 1 行，
   故 `:497` 是第 13 行，与切分的说法一致。
