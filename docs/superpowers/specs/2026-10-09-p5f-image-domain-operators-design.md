# P5f 图像领域算子 — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md` §五 的 P5f 行「一句话范围」栏的原话，逐字照用）：

> 把 §323–§327 的两种原生操作落成 Operator 集——完整生成与局部修改**分作两个算子**（§161 明写「不能统一成一个 `image_tool(prompt)`」）、编辑区域的四种表示（Point 必须解析成语义区域，§325）、`outside_mask_change = FORBIDDEN` 的确定性强制作（§326），以及每次编辑产生新 Artifact 的谱系（§327）

**本块拥有 / 产出的接口**（同行的第二栏）：本领域的 Operator 集；**不拥有** `ArtifactType` 的改动（向 P5e 报出）。

切分文档在同行的**订正段**里（该行下方，2026-10-09 由 P5b 的设计查出）已把「与它在 P5b 目录里的条目」一句撤去并注明「那句没有对应物」——本设计据此把「本块无 `bind` 目标」写成一条显式处置（§8），不另立一份目录。

**规范依据**：§323 §324 §325 §326 §327（`docs/spec/05-normative.md`）；§161–§168（`docs/spec/03-product-drive.md`）；《工程》§8.1 第 14 行（`docs/02-工程.md:498`）、§8.2（`docs/02-工程.md:512`）、§8.3（`docs/02-工程.md:527`）；《总纲》§8.6（`docs/01-总纲.md:1504`）。

**本文件只写 P5f。** 其余五块（P5a 验证与证据、P5b 执行方法库、P5c/P5d 两个领域算子、P5e 媒体领域算子）各有其设计，本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §12 写成一条**具名的对账条目**，不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」（判据与订正原文见 `docs/02-工程.md:563-567`）。凡引他文的箭头，原样照录并标注。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p5f`）里当场 `grep -n` / `sed -n` 实测的，未照抄任何一份别的文档（含切分文档与 P5a/P5b/P5e 的设计）。指进共享文档（切分文档、其余块的设计）时按内容指，不写行号范围。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| 两种原生操作分作两枚算子（不得合并） | 切分 §五 P5f 行；§161 `:570-576` | §3.2 | 全量（端口类型、determinism、side_effect_class、backend 候选） |
| 编辑区域的四种表示（Point/Box/Mask/SemanticObject） | §325 `:2351-2358`；§166 `:745-776` | §4.1 | 全量（枚举 + 四种表示到 `EditMask` 的映射） |
| **Point MUST 解析成语义区域** | §325 `:2360-2366`；§164 `:682-691` | §4.2、§3.2 第 2 行 | 落点全量；**判据规范未给**，显式处置见 §4.3 |
| `outside_mask_change = FORBIDDEN` 的确定性强制作 | 切分 §五 P5f 行；§326 `:2372-2388`；§165 `:711-739`；《工程》`:512` | §5 | 强制作（`hard-composite`）+ 检出（`verify-outside-mask`）；**类型层拦不住**，据实写明 |
| 每次编辑产生新 Artifact 的谱系 | 切分 §五 P5f 行；§327 `:2394-2404`；§167 `:780-814` | §6 | 全量（落在 `Artifact` 的既有字段）；**不复建「不得覆盖」的判据**（§6.2） |
| 只产出 Evidence、只消费判定 | 切分 §四第 1 条 | §7 | 证据由**宿主的执行代码**调 P5a 的 `Evidence::from_tool_result`（唯一构造点）构造；本块**不写域包装**，也**不提供这一层的函数**（§7.2，也是 §2.1 把 `continuum-verify` 记为 dev 边的原因） |
| 与 P5b 方法库的接缝 | 切分 §五 P5f 行的订正段 | §8 | **本块无 `bind` 目标**（显式处置一） |
| §343 的分级后果 | 切分 §五末「§343 的分级后果」段 | §9 | 落地次序写成批次（显式处置二） |
| 迁移号段 | 切分 §二第 5 条 | §10 | **本块零迁移**；附实测占用表 |

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| Verifier 的选择、隔离输入、完成判定、`Evidence` 类型、`VerificationPolicy` | P5a（切分 §四第 1 条）。本块**不得**定义证据类型、自定义「充分」判据、自定义完成判定 |
| 方法的登记形态、`MethodRegistry`、`MethodEntry`、`MethodDomain` | P5b（切分 §四第 3 条：在 P5b 过审前不得自造）。**本块不调 `bind`**（§8） |
| 节点状态迁移、`Queued → Running`、`OperatorRegistry::resolve` 的调用点、算子的执行 | 第 3 层（切分 §四第 4 条）。本块只**注册**算子 |
| `execution_policy` 的判定 | P4（切分 §四第 5 条） |
| `verification_policy` 的解析与判定 | P5a（切分 §四第 5 条；其设计 §2.3 的裁定是「字段保持 `Value`，定型落在消费侧」）。本块不碰该字段 |
| `ArtifactType` 的任何新变体 | P5e（切分 §四第 2 条：只由 P5e 一处改）。**本块向 P5e 报出的清单为空**（§3.4 末） |
| §34 的 `GenerativePermission` 与授权判定 | P5e（该型在其设计 §7.1，落在 `continuum-media`）。本块不造第二枚；图像生成不在 §34 的五个旗标内（§3.6） |
| 复用判据（缓存键、`can_reuse`） | P1 已落地（`crates/continuum-graph/src/reuse.rs:29`）。本块**不建第二份**（§3.5） |
| 「不得覆盖原图」的判据 | §241 的落地已在 `save_artifact`（`crates/continuum-artifact/src/persist.rs:45`，`:31` 的注记逐字「同 id 已存在时返回数据库错误，不覆盖（§241）」）。本块**不建第二份**（§6.2） |
| §262 第 3 级的独立模型验证器（§168 的第一项检查） | 本块**不注册**该算子，理由与落点见 §5.5 |
| 3D 链（`Reconstruct3D` / `RenderScene`）、`3d/` 目录的对家 | **本轮无解**（切分 §八 的「`3d/` 在四个具名算子里没有对家」那一条）。本块不注册这两个算子 |
| 迁移 / 表 | 本块**零表**（§10） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：`continuum-image`（实测：`grep -rn "continuum-image" crates/ Cargo.toml` **零命中**，名字未占用。**命令与范围要写准**：不加限定时 `grep -rn "continuum-image" --exclude-dir=.git .` 有 6 处命中，**全部落在本设计自身**——零命中类判据要先问「本仓的哪条规矩会往那个语料里加字」，此处正是设计文档自己加的字，故范围取到 `crates/` 与根 `Cargo.toml`，读的人重跑才能得到同一个结果）。

依赖边表（**只登记实际用到的**，切分 §四第 6 条）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-artifact` | `ArtifactType`（§3.2 的端口类型取它）、`ArtifactId`（§4.2 签名里的 `source` 实参——**这是本 crate 签名里唯一的用点**） | 普通 |
| `continuum-operator` | `Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` / `BackendId` / `OperatorRegistry` / `OperatorError`（§3.1 的登记入口） | 普通 |
| `continuum-verify` | `Evidence`（§11.4 第 2 条的 `trybuild` 用例：`Evidence { .. }` 与 `Evidence::default()` 在本 crate 里编不过） | **dev 边** |
| `continuum-graph` | `can_reuse` / `cache_key` / `CacheKey`（§11.2 第 (o)、(p) 两条的照片：核本块声明的 `determinism` 真能让 §305 的判据成立或按预期不成立） | **dev 边** |

**两处订正，逐条留下原先那处说的是什么与为什么以现说为准**：

- **`continuum-artifact` 那一行的「用到什么」栏原先写的是「`BlobStore` / `ContentHash`（§5 的两个确定性函数读像素）」，已删。** §5.2 的两个确定性函数收 `&Raster`（§3.1），**读写落盘是调用方的事**，§5 全节一处不用 `BlobStore`／`ContentHash`。同一行原先附在 `ArtifactId` 后的「§7 的证据产出的 `artifact_refs`」也一并撤去——§7.2 明写本块不提供证据面那一层的函数。**以 §3.1／§5.2 为准的理由**：切分 §四第 6 条要求这一栏与实际用量精确一致，而这一栏正是 `ALLOWED` 那一行的依据；原先的理由把「掩码落在既有的 `Blob` 落盘路径上」（§10.1，那是 P1 的路径、不是本块的调用）读成了「本块调 `BlobStore`」。**边本身保留**：`ArtifactType` 与 `ArtifactId` 都在本块的公开签名上。
- **`continuum-verify` 那一行的边别原先是「普通」，已改为「dev 边」。** 原先的理由是「本块**直接调构造点**，不写包装」；而 §7.2 明写**本块不提供这一层的函数**（「本块与 P5a 之间的实际数据流」那一段）——证据由**宿主（第 3 层）的执行代码**调 `from_tool_result` 构造。故本 crate 的生产代码对这枚 crate **零调用**，唯一使用点是 §11.4 第 2 条的 `trybuild` 用例，按本条表下「边别按『谁在用』判」的口径即 **dev 边**。**以 §7.2 为准的理由**：§3.1 的 `ImageError` 四臂无一来自该 crate，§7.3 明写本块不调用 P5a 的判定；「直接调」那句只在「不经域包装」这个意义上成立（§7.2 已改写这句话）。

`continuum-verify` 是 P5a 新建的 crate，今天在 `crates/` 下不存在（实测：`crates/` 下无该目录）。本块以 dev 边依赖它，不构成环：P5a 的依赖边表（其设计 §2.1）里没有任何 P5 领域算子块。

**不登记**的边，逐条给理由（零使用的边即假边，P2b 为此删过两条）：

- **不登记 `continuum-method`**：本块**不调 `bind`**（§8），该 crate 的接口本块一个都不用。这是本块与 P5e 在依赖表上的一处**实质差别**（P5e 有四条 `video/` 方法要填，本块没有目录可填）。
- **不登记 `continuum-port`**：本块不构造 `Port`、不做端口兼容判定。`Operator.input_schema` / `output_schema` 取 `Vec<ArtifactType>`（`crates/continuum-operator/src/definition.rs:82-83`），不经 `Port`。`compatible` 是 P1 的（`crates/continuum-port/src/port.rs:94`）。
- **不登记 `continuum-semantics`**：本块不构造 `RequirementId`——本块不产出证据（§7.2：产出的证据由调用方构造，本块不写包装），也不构造归属。该 crate 今天在 `crates/` 下不存在（P4 未开始实现）。
- **不登记 `continuum-persist`**：本块零表（§10），不碰 `Tx` / `Migration`。
- **不登记 `continuum-events`**：本块不写事件（§10）。
- **不登记 `continuum-policy` / `continuum-capability`**：本块不查策略引擎、不取 Capability Token（§3.6：图像生成不在 §34 的五个旗标内，本块没有授权判定）。

**dev 边为什么也要登记**：`dependency_direction.rs` 的 `cargo tree` 带 `--edges all`（`crates/continuum-runtime/tests/dependency_direction.rs:270` 的 `cargo_tree_direct` 实参里有 `--edges`），dev 边与普通边一视同仁。本仓已有同形的先例：`continuum-provider` 那一行注记明写「Task 4 起加上 persist：**dev 边**」（同文件 `:32-34`）。

**边别按「谁在用」判**：本块有**两条 dev 边**，判据同形——生产代码不调它们，用到它们的是用例。`continuum-graph`：§11.2 第 (o)、(p) 两条的照片（对 `can_reuse` / `cache_key` 的断言）。`continuum-verify`：§11.4 第 2 条的 `trybuild` 用例（`Evidence` 在本 crate 里造不出）。**按同一口径，本块的另外两条边（`continuum-artifact`、`continuum-operator`）是普通边**——它们的类型在本块的公开签名上。

**外部依赖不进 `ALLOWED`**（该表断言的是 workspace 成员之间的边，`crates/continuum-runtime/tests/dependency_direction.rs:200-204` 的口径）：本块的 `ImageError`（§3.1）用 `thiserror` 派生 `Error` 与 `Display`，与 `crates/continuum-operator/Cargo.toml:11` 的既有用法同形。是否需要 `serde` 由实现计划定，本设计不预设。

**共写文件**（切分 §一 的共写文件表给的是五处；下表是本块的那部分，**只有两行**，逐行注明来历）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `crates/continuum-image` | 切分建议由先落地者一次加齐六行；本块**只加自己这一行** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-image", &["continuum-artifact", "continuum-graph", "continuum-operator", "continuum-verify"])`——**四项**，与 §2.1 边表的四行逐行对应（含 `continuum-graph` 与 `continuum-verify` 两条 dev 边；dev 边也进这张表，理由见 §2.1 的「dev 边为什么也要登记」） | 新 crate 会先让该门变红再被补齐（`:288` 的用例），这是刻意的；数组按字母序 |

**本块不碰切分 §一 共写表里的另外三处**（`artifact.rs` / `node.rs` / `crates/continuum-operator/src/definition.rs`）：本块不加 `ArtifactType` 变体（§1.2）、不改 `Node` 的任何一行（不碰 `verification_policy` 也不碰 `execution_policy`）、不改 `Checkpointable`（其错误类型归 P5e，切分 §二第 4 条）。**这是本块与 P5e 在共写表上的第二处实质差别**，据实列出以免下一轮照 P5e 的行数去猜。

## 2.2 冻结清单：本块持有的类型与判定

**类型**（定义在 `continuum-image`）：

```
EditRegionKind   EditRegion   EditMask   Raster   RegionDetector   ImageError
```

其中 `Raster` 是 §5 的两个确定性函数（合成与比较）的**图像**像素载体，其字段与「为什么是本块定的」见 §5.2；`EditMask`（§4.1）是同一对函数的**掩码**载体——单通道位图，故它与 `Raster` 不是同一型；`RegionDetector` 是 §4.2 的 `resolve_edit_region` 收的那个**接口**——本块定义它，其实现在 backend 一侧（第 3 层选定的后端）。另有**一枚构造函数与一枚常量**（都不是类型）：`all_operators() -> [Operator; 5]` 与 `V01_OPERATOR_IDS: [&str; 1]`（§9），它们承载的是 P1 的 `Operator`。（**`all_operators()` 这里不是 `const`**：`Operator` 在 crate 外构造不出常量值，理由见 §3.1。）**本块不新造算子类型**——§3.1 的登记形态照 P1。

**判定**（本块是它们的唯一落点）：

1. §325 的「Point MUST 被解析为 semantic edit region」的**唯一落点**（§4.2 的 `resolve_edit_region`）。
2. §326 的 mask 外像素**比较器**（§5.3 的 `outside_mask_difference`）——P5a 的设计 §14 第 3 条把它指给本块（原文：「插槽与『无物可读 ⇒ `Unknown`』的规则在本块（§7.5），**比较器在 P5f**」）。
3. §326 的 mask 外像素**强制作**（§5.2 的 `composite_masked`）。
4. 五枚算子的 `determinism` 声明（§3.2 的「定」字各格）。

**本块不持有的判定**：证据的充分性、覆盖、verifier 的级别与独立性、完成判定（全部 P5a）；方法的登记与选取（P5b）；复用的四条件（P1 的 `can_reuse`，§3.5）；「不得覆盖原图」（P1 的 `save_artifact`，§6.2）。

**一枚本块不持有的类型，写明**：§34 的 `GenerativePermission`。它归 P5e（其设计 §7.1 把它定义在 `continuum-media`）。本块既不定义它、也不向它加臂（§3.6）。

## 2.3 本块的层间位置

§9.1（`docs/02-工程.md:569-586` 的图：与本块有关的两行是 `:575` 的 `└──→ 跨领域 (8)` 与 `:585` 的 `跨领域 (8) ──→ 长期循环 (7)`；箭头的读法在 `:563-567`；`:588` 逐字「依赖方向单向，无环。」）里与本块有关的两条，按「被依赖者 → 依赖者」读：

- `执行层 (3) → 跨领域 (8)`：§2.1 的边表**四行**全是它的实例。`execution_policy` / `verification_policy` 的定型落在消费侧而不是字段上，也是同一条的后果（切分 §二第 2 条的裁定段；本块不重复它）。
- `跨领域 (8) → 长期循环 (7)`：本块**不接**这一条——本块不产出 `value + evidence + last_verified`（§232 的 `ProductReadiness` 属长期循环）。本块的产物注册进 `OperatorRegistry`，不写长期循环的表。

本块与 P5a（同层）之间有一条层内边 `continuum-image → continuum-verify`，**边别是 dev 边**（§2.1：本 crate 的生产代码对它零调用，用到它的是 §11.4 第 2 条的 `trybuild` 用例）。**层内边在本仓有同形的处置**：`continuum-provider → continuum-capability`（资源层内的「接口 → 能力类型」）被登记为层内边（`crates/continuum-runtime/tests/dependency_direction.rs:30` 的注记）。本条边是单向的：P5a 的依赖边表里没有任何 P5 领域算子块。

**本块的接口面与《工程》§8.3 的那一行对齐**：`领域算子 ← 第 3 层 Operator 注册表 ＋ 第 4 层 Router`（`docs/02-工程.md:527`）。本条是**跨层**目标（第 8 层行指向第 3 层），故本块与第 3 层之间的两条边（注册表、Router）是**其上行**，不是新边（切分 §三 已记「P5 的四个领域算子块都只经 `Operator` / `OperatorRegistry` 与第 3 层相接，不新开跨层边」）。

---

# 3. 算子集

## 3.1 登记形态：照 P1，不自造

§244（`docs/spec/05-normative.md:735-753`）的 `Operator` **七个**字段由 P1 落地为 `crates/continuum-operator/src/definition.rs:79-88`：`id` / `version` / `input_schema: Vec<ArtifactType>` / `output_schema: Vec<ArtifactType>` / `determinism` / `side_effect_class` / `backend_candidates: Vec<BackendId>`。**本块不加字段、不改签名**（切分 §二「无需重新冻结」段）。本块交付的是**内容**：五枚 `Operator` 值的清单与它们的注册入口。

**上面这个「七」是订正过的**：本句原写「`Operator` **八**字段」。**「八」的来历**：相邻条款 §258（`docs/spec/05-normative.md:1073-1088`）的 `Evidence` 是八个字段（`id` / `type` / `subject` / `claim` / `producer` / `artifact_refs[]` / `strength` / `scope`），本设计早前把那一条款的字段数记到了 `Operator` 上。**同一形状的错在别的块的设计里也有，而本分支只读得到其中一份**：`docs/superpowers/specs/2026-10-09-p5e-media-domain-operators-design.md` 的 §5.1 开头那一句，按**本分支读到的内容**同样写作「`Operator` 八字段」（该文件在本分支未带上订正——订正那一笔是本仓历史里的 `f1474ac`，`git merge-base --is-ancestor f1474ac HEAD` 为假，即**不在本分支的祖先里**）；另有一份在 `p5d` 分支上，**本分支读不到，故不替它断言内容**（同 §14 第 5 条的那条纪律）。**按本仓纪律，别处也错不豁免本块这一份。** **实测两个计数**：§244 `:735-753` 的围栏块逐行数出七个；P1 的 struct `crates/continuum-operator/src/definition.rs:79-88` 也是七个 pub 字段（本设计 §11.4 末的一段本就写作「七个 pub 字段」）——**同一份文档里两处相抵，以实测的七为准**。

注册入口的形状照 `OperatorRegistry`（`crates/continuum-operator/src/registry.rs:19-48`）：

```
/// 本域五枚算子的全集。顺序即 §3.2 表的行序。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**：`Operator` 在 crate 外构造不出常量值（两条理由见本段下）。
/// 数目 5 写在**返回类型**里，故清单个数的改动**编译期可见**。
pub fn all_operators() -> [Operator; 5];

/// §343 的 v0.1 批（§9）：本域在 v0.1 内要注册的那些算子的 id。
/// 判据：每一项都能在 `all_operators()` 里找到；且补集恰为其余四枚（逐枚断言，不是只看长度）。
pub const V01_OPERATOR_IDS: [&str; 1] = ["generate-image"];

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先查重后写**：先逐枚核「注册表里是否已有同 (id, version)」（判定在 P1 的
/// `OperatorRegistry::register`，`crates/continuum-operator/src/registry.rs:24-34`），
/// 全部通过后再逐枚注册；任一枚被拒即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚图像算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§9 的批次边界要能按 §343 换，
/// 且 §11.2 第 (c) 条（重复注册）才写得出来。
pub fn register_image_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), ImageError>;
```

**`all_operators()` 是函数不是常量（订正过；原写 `pub const ALL_OPERATORS: [Operator; 5];`）**。**这里不是 `const`**，两条理由各自独立成立：

1. **本仓带 `ALL` 常量的既有做法，成立前提是「无字段」**——取值可在 `const` 里直接枚举。本设计 §11.1 援引的那两处（`crates/continuum-artifact/src/artifact.rs:28` 的 `ArtifactType::ALL`、同文件 `:100` 的 `PrivacyClass::ALL`）都是**无字段枚举**，而 `Operator` 不是。**援引一条既有约定时要连它的成立条件一起搬。**
2. **`Operator` 在 crate 外构造不出常量值**：它的 `id` 型为 `OperatorId`，而 `OperatorId(String)` 的**字段私有**、`OperatorId::new` **不是 `const fn`**（`crates/continuum-operator/src/definition.rs:24-27`）；它另含三枚 `Vec` 字段（`input_schema` / `output_schema` / `backend_candidates`，同文件 `:79-88`）。两条都使 crate 外的 `const` 构造不成立。

**保住的正是原行要的那条性质**：数目 5 改由**返回类型**承载（`[Operator; 5]`），清单个数的改动仍**编译期可见**（写少一枚而不改返回类型，直接编不过）。**`V01_OPERATOR_IDS` 不动**：它的元素型是 `&str`，`const` 成立。

**「先查重后写」这一句也是订正过的**：原先的注写的是「**先核后写**：先逐枚核 §3.5 的前提，全部通过后再逐枚注册」。**§3.5 自述本块没有那条注册期前提**（两枚 `Deterministic` 算子的候选集合是单元集，「名单内／名单外」在本块没有可触发的形态），故「先核」的内容落不下来。**以查重为准的理由**：§11.2 第 (c2) 条要拍的正是「不留半注册」，而它是查重（`crates/continuum-operator/src/registry.rs:24-34` 的 `contains_key`）与写入（`:32` 的 `insert`）之间的**次序**——核的东西只能是注册表现状，不可能是 §3.5 那条本块没有的规则。

`ImageError` 的形状（§2.2 列出的本块六枚类型之一）。四臂，逐臂给出**判定的持有者**：

```
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1：
    /// `crates/continuum-operator/src/registry.rs:7` 的 `OperatorError::Duplicate`，
    /// 判定在 `:24-34` 的 `OperatorRegistry::register`。
    /// 本块**只带出来、不重述**——本臂与 `OperatorError::Duplicate` 是同一件事，
    /// 另立一枚 `ImageError::Duplicate` 就是「注册表里有没有这个算子」的第二套词汇表。
    #[error(transparent)]
    Registry(#[from] OperatorError),

    /// §325：该区域解析不出语义区域（§4.3）。判定的持有者在本块（`resolve_edit_region`）。
    #[error("区域 {region:?} 解析不出语义编辑区域（§325）")]
    RegionUnresolved { region: EditRegionKind },

    /// §326：参与逐点运算的像素载体形状不一致——逐点合成与比较无定义。
    /// 判定的持有者在本块（§5.2 的 `composite_masked`、§5.3 的 `outside_mask_difference`）。
    #[error("像素载体形状不一致：{left:?} 与 {right:?}（§326）")]
    ShapeMismatch {
        /// `(宽, 高, 通道数)`。掩码是单通道位图（§4.1），**没有色彩通道这一维**，
        /// 故掩码那一侧的第三分量是 `None`——两个形状比较时，`None` 的分量不参与。
        left: (u32, u32, Option<u8>),
        right: (u32, u32, Option<u8>),
    },

    /// 一个载体的 `pixels` 字节数与它自己声明的宽、高（、通道数）不符——
    /// 该值不满足自己的不变量（§4.1 的 `EditMask`、§5.2 的 `Raster`），
    /// 逐点运算的索引无定义。判定的持有者在本块（两个确定性函数的入口）。
    /// **与 `ShapeMismatch` 分得开**：那一个比的是**两个载体之间**的形状，
    /// 这一个比的是**一个载体内部**的字节数与它自己声明的形状。
    #[error("`pixels` 长度 {len} 与声明的形状不符（期望 {expected}）")]
    PixelLengthMismatch { len: usize, expected: usize },
}
```

**本臂的名字与字段是订正过的**：原先叫 `MaskMismatch { mask: (u32, u32), source: (u32, u32) }`，只表达「掩码与源图尺寸不一致」，且 §5.2 只钉了这一对前置条件。**订正的两处**：名字改成 `ShapeMismatch`，因为 §5.2、§5.3 要拦的不止掩码与源图（还有 `generated`↔`source`、`source`↔`result` 两对，见 §5.2、§5.3）；字段从 `(u32, u32)` 扩为 `(u32, u32, Option<u8>)`，因为**通道数也要拦**（逐点复制与逐点比较都要求两侧逐像素一一对应），而掩码那一侧没有通道这一维——`Option` 正是这个事实，不是占位。

**`PixelLengthMismatch` 是后加的一臂**：§5.2、§5.3 的入口要核「`pixels` 的字节数与声明的形状相符」，而这件事**表达不进 `ShapeMismatch` 的 `(u32, u32, Option<u8>)`**（要报的是两个 `usize` 的长度值）。**原先这一条前置的报错被写成 `ShapeMismatch`，已改**：形状表达不了这一对量，照原样写就是一句在该分支上无意义的承诺。

**两枚新类型对派生的要求，逐条写明**：`EditRegionKind` 要 `derive(Debug)`（`{region:?}` 要求）；`ShapeMismatch` 的两个字段取 `(u32, u32, Option<u8>)` 元组、`PixelLengthMismatch` 的两个字段取 `usize`，`{:?}` 只要求 `Debug`，**故本块不必为它们写 `Display`**——与 P5e 为其错误类型补 `BackendId` 的 `Display`（其设计 §4.4）情形不同：那一处的格式串用的是 `{backend}`，而本块的各处都用 `{:?}`。

**四臂各自的判据（为什么是这四臂、不是两臂、不是三臂）**：

- `Registry` 的存在理由是**不由本块重述 P1 的判定**（理由见上）。**这里不是「照 `CheckpointError` 的做法把注册期错误并进本型」**：P5e 的 `CheckpointError` 与 `OperatorError` **分得开**（一个是「这一次检查点/恢复为什么不成」，一个是注册期查重），而本臂的判定与 `OperatorError::Duplicate` 是同一件事。
- `RegionUnresolved` 与 `ShapeMismatch` 是**两个不同的处置**：前者是「这次解析没找到区域」（Point 点空处、SemanticObject 名不副实），后者是「区域／补丁找到了，但两张位图不是同一坐标系（或通道数不同）」。它们的**调用方**也不同（前者在 `resolve_edit_region` 的四个臂上，后者在两个确定性函数的入口）。合成一臂会让调用方分不清该重试解析还是该重跑检测。
- **`ShapeMismatch` 与 `PixelLengthMismatch` 也分得开**（都在两个确定性函数的入口，故这一条是**同一处两条不同的判据**）：前者比**两个载体之间**的形状（掩码↔源图、生成图↔源图、结果图↔源图），后者的两侧是**同一个载体**——它自己声明的形状与它自己字节序列的长度。**不能并臂的理由是形状**：`ShapeMismatch` 的字段是 `(宽, 高, 通道数)`，而这一对量是两个 `usize`；并进去就得给那个元组加一个在别的三对上无值的分量。
- **不给「backend 不支持掩码编辑」留臂**：那是一个**声明面**的事（`backend_candidates` 的取值），不是一次调用的失败；§3.5 的「被否决的更显然写法」段说明本块为什么不给它编一张名单。
- **不给 IO 臂**：本块的两个确定性函数收 `&Raster`（§5.2），**读写落盘是调用方的事**（`BlobStore` 在 `continuum-artifact`，`crates/continuum-artifact/src/blobstore.rs:70` 的 `put` / `:99` 的 `get`）。放一个 IO 臂就要求本块规定像素的存储形态，而规范没有给。**这一条对本块全节成立**：`resolve_edit_region`（§4.2）不落盘——`Box` 支投影出的掩码以位图形式返回（§4.1 的 `EditMask`），把它变成 `ArtifactType::Blob` 制品要经 `BlobStore::put`，那是调用方的事。故 §2.1 的边表里**没有** `BlobStore`／`ContentHash`。

## 3.2 五枚算子

出处列的口径：**算子名尽量照录规范**；**`determinism` / `side_effect_class` / backend 候选**在规范未给处由本设计定，并逐行标出「定」字。**规范没有一处给过 operator 的 determinism 或 side_effect_class**（这一句是本设计逐节读 §161–§168 与 §323–§327 的实测结论）。

| # | id | input_schema | output_schema | determinism | side_effect_class | backend 候选 | 出处 |
|---|---|---|---|---|---|---|---|
| 1 | `generate-image` | `[Image]` | `[Image]` | `NonDeterministic`（定） | `NonIdempotent`（定） | `image-gen-backend`（定） | §323 `:2316-2331`；§162 `:596-612`；§161 `:581` |
| 2 | `detect-edit-region` | `[Image]` | `[Blob]` | `NonDeterministic`（定） | `Pure`（定） | `object-detector-backend`（定） | §164 `:684`（Object / Region Detection）；§325 `:2360-2366`；§166 `:745-776` |
| 3 | `local-generative-edit` | `[Image, Blob]` | `[Image]` | `NonDeterministic`（定） | `NonIdempotent`（定） | `inpainting-backend`（定） | §164 `:688`（Local Generative Edit）；§165 `:724`（Generative Inpainting）；§324 `:2335-2345`；§163 `:626-665` |
| 4 | `hard-composite` | `[Image, Blob]` | `[Image]` | `Deterministic`（定） | `Idempotent`（定） | `builtin`（定） | §165 `:728`（Hard Composite）；§326 `:2378-2388`；§164 `:690`（Composite） |
| 5 | `verify-outside-mask` | `[Image, Blob]` | `[Json]` | `Deterministic`（定） | `Pure`（定） | `builtin`（定） | §168 `:818-835`；§326 `:2372`；《工程》`:512` |

**五枚的算子名逐枚有规范的原文出处**，两枚原生操作各占一枚（第 1、3 行），另三枚是它们内部流程上的具名步骤（§3.3）与链尾的检查器（第 5 行）。§3 全节的算子名**没有一处是本设计编的**。

**逐枚的判定理由（只写需要解释的，五枚）**：

1. **`generate-image` 的 `input_schema` 是 `[Image]` 而不是 `[]`**。§162 `:603` 的 `reference_artifacts` 与 §323 `:2321` 的 `references[]` 是**制品**，故它们是输入端口；「从零生成」（§161 `:584`）的那一次只是**没有接参考制品**（节点上没有那个输入端口），不是「该算子不接受制品」。**这里不是「照 P5e 的 `media-import` 取 `[]`」**：`media-import` 的输入是**外部素材**（不是上游制品），本算子的参考制品是上游制品。`input_schema` 是**类型**集合、不是**元数**（实测：把 `input_schema` / `output_schema` 读作 **`continuum-operator` 的 `Operator` 的那两个字段**时，`crates/` 下的**非测试**命中只有 `crates/continuum-operator/src/definition.rs:82-83` 两行声明——**没有一处读它们来派生端口**；端口在 `Node.inputs` / `Node.outputs` 上，`crates/continuum-graph/src/node.rs:13-14`）。**这个限定要写出来，否则它是一句过宽的全称**：裸标识符 `input_schema` 在别处另有非测试命中，而那些是同名的**另一个字段**（型为 `Value`，是 MCP 工具的参数 schema，与 `Operator` 的端口无关）——`crates/continuum-capability/src/tool.rs:44` / `:45` / `:82` / `:83` / `:86` / `:87`、`crates/continuum-capability/src/persist.rs:59` / `:60`、`crates/continuum-core/src/tool.rs:22`（测试侧另有 `crates/continuum-capability/tests/` 与 `continuum-graph/tests/` 等多处）。故取 `[Image]` **不会**凭空多出一条 DATA 边。
2. **`detect-edit-region` 的 `output_schema` 是 `[Blob]` 而不是 `[Image]`**。掩码是**单通道位图**，不是彩色图像；记成 `Image` 会让读它的人按图像的色彩空间 / Alpha 约定去读，而 §168 `:826` 的另一条检查恰好是「Alpha / 色彩空间是否异常」——两者会被同一型名混起来。`Blob` 已在枚举里（`crates/continuum-artifact/src/artifact.rs:20`），**故本块不向 P5e 要新变体**。**掩码的编码格式本设计不定义**（缺口见 §13 第 5 条）。
3. **第 1、3 行是 `NonDeterministic`**：它们都经生成后端（图像生成模型 / inpainting 模型），同输入同 prompt **不保证逐位相同**——§327 `:2396-2402` 的谱系里每一步记 `seed`（§167 `:803`）本身即「同 prompt 可以是另一张图」的承认。
4. **第 2 行是 `NonDeterministic`**：Object / Region Detection（§164 `:684`）是模型步，不是算法步。**这是本设计的判定**。
5. **第 4、5 行是 `Deterministic`**：`hard-composite` 是逐像素复制（§165 `:735-737` 逐字「mask 外像素直接从原图复制」），`verify-outside-mask` 是比较（§168 `:832-835` 的 `outside-mask pixel difference`「可以直接确定性检查」）。两者都不含模型解码。**§168 自己说了「可以直接确定性检查」，故这两枚的 determinism 不是我造的。**
6. **第 1、3 行是 `NonIdempotent`，第 2 行是 `Pure`**：§307 的 `RetryPolicy` 要求「非幂等 Effect MUST NOT 直接自动重试」，而重跑一次图像生成得到的是**另一张图**（这是副作用意义上的不可重放）；而区域检测是**分析**（它产出一个区域，不产出要被使用的最终制品）。P5e 的 §5.2 用的是同一条线（四枚生成算子 `NonIdempotent`，分析型 `Pure`），本块照同一口径取。
7. **`hard-composite` 的 `Idempotent` 而不是 `Pure`**：同输入同后端产出逐位相同的输出（幂等），但它产生一个**大制品**（一整张图，且 §327 要求它是**新的** Artifact）；`Pure` 在本仓的用法里与「可自由重算」相邻，而重算一次合成要落一个新 Artifact 与新事件。**这是本设计的判定**，理由与 P5e 对 `render` 的处置同形。

**五枚都不含「参考物是文本」的情况**：§162 `:603` / §323 `:2321` 的 `reference_artifacts` / `references[]` 未给类型，本设计取 `Image`（图像域内，参考物是图像）。若实际需要 `Text` 参考（如文字描述作为参考制品），那是**输入面的加宽**，走 §12 第 5 条报出——**本设计不预先收它**。

**§323/§324 的五个非制品条目住在哪（落点声明）**：规范在这两条里给了五个**不是制品**的字段——§323 的 `prompt` / `dimensions` / `constraints`（`docs/spec/05-normative.md:2316-2331`）与 §324 的 `instruction` / `preserve_outside_region`（`:2335-2345`）。**它们的落点是节点参数**（`Node` 的参数字段），理由与 §10.1 表里区域表示那一条同源：它们描述「这一次编辑要什么」，不是上游产出的制品，故**不进 `input_schema`**（`input_schema` 是 `Vec<ArtifactType>`，只能装制品类型）；而 `resolve_edit_region` 的入参（§4.2）与 `composite_masked` 的调用点都由这些参数供给。**本块不定义它们的名字、类型或存储**：`Node` 的参数字段 schema 规范未给，`Node` 也不在本块的共写表里（§2.1 末）——**这是一处缺口**（§13 第 11 条、§12 第 5 条 (iv)），收件人：规范维护者。**边界写清**：本块只声明「它们住在节点参数上、不进本块的任何类型」，**不替它们定形状**；`preserve_outside_region` 在本设计里只作为条件被引用（§5.4、§12 第 2 条），本块不为它写解析或校验。

## 3.3 「分作两个算子」的读法：**否决合并**，不是「全域恰好两枚」

§161 `:570-576` 的原话是「图像系统不应该统一成一个 `image_tool(prompt)`；至少应该区分两种原生操作」——**「至少」**。切分 §五 P5f 行的「分作两个算子」是这条的转述，它否决的是**合并**。

**本设计的排布**：两枚原生操作各有一枚具名算子落在 §3.2 表里（第 1 行 `generate-image` = §323 的 `GenerateImage`；第 3 行 `local-generative-edit` 与第 4 行 `hard-composite` 合起来 = §324 的 `LocalImageEdit`）。另两枚（第 2、5 行）是这两条链上的具名步骤。

**为什么局部修改落成两枚而不是一枚（这是本节的裁定，理由三条，按分量排）**：

1. **§165 的流程把 Hard Composite 画成独立一步**（`:720-731`：`Original Image → Region Mask → Generative Inpainting → Generated Patch → Hard Composite → Final Image`），且 §326 `:2378-2388` 逐字把它写成 **Runtime** 的动作（「若 backend 会重生成整张图：**Runtime SHOULD 使用** generated patch + hard composite 保持 mask 外原像素」）。
2. **§326 的条件挂在 backend 上，而 backend 是 Router 在运行期选的**（§245 `:756`）。「若…会重生成整张图」这个条件**在注册期不可知**，故它只能由**图上的步骤**表达——即「那一步可以被插进去」。
3. **合成藏进算子里，§326 的 SHOULD 就没有落点**。**这里不是「把 Composite 藏进 `local-generative-edit` 内部」：那样 `preserve_outside_region = true`（§324 `:2343`）的兑现只剩下后置检测（§5.3 的比较器），即「检出不合法」而不是「使之合法」；而 §326 给的是后者的路。**

**本条不声称「图像域只有这五枚算子」**：§161 的「至少」与 §10（`docs/spec/01-concepts.md:536-560`）的三枚媒体算子签名（那是**举例**）都不支持穷尽声明。本设计只声称「已实测的出处有 §3.2 表那五行」。

## 3.4 端口类型与边表

**先定关系**：上游算子的 `output_schema` 与下游算子的 `input_schema` **相交非空**——这是 §239 `:654`「运行时 MUST 拒绝不兼容连接」在**注册期**可检查的那一半；与 P5e 的 §5.3 同口径（那里已把「相邻步骤」与「数据依赖边」分开，本块照同一读法，不重复论证）。

**边表**（§11.2 第 (s) 条的语料）。括号里是**该来源的 `output_schema` 与下游 `input_schema` 交集中的型**：

| 下游算子 | 输入的来源（交集中的型） | 依据 |
|---|---|---|
| `generate-image` | 上游任一枚出的 `Image`（§162 的 `reference_artifacts`；可为零条） | §162 `:603`；§3.2 第 1 行 |
| `detect-edit-region` | 上游任一枚出的 `Image`（`generate-image` 或素材导入） | §164 `:682`（User Point 之后的第一步）；§3.2 第 2 行 |
| `local-generative-edit` | `detect-edit-region`（`Blob`）、上游任一枚出的 `Image`（源图） | §164 `:686-688`（Semantic Mask → Local Generative Edit）；§165 `:722-726` |
| `hard-composite` | `local-generative-edit`（`Image`）、上游源图（`Image`）、`detect-edit-region`（`Blob`） | §165 `:726-728`（Generated Patch → Hard Composite）；§326 `:2382-2386` |
| `verify-outside-mask` | `hard-composite`（`Image`）、上游源图（`Image`）、`detect-edit-region`（`Blob`） | §168 `:832`（`outside-mask pixel difference` 要比的是**原图**与**结果图**） |

**依据栏的读法**：写 §164/§165/§168/§326 的，出处是规范正文的步骤名；写「§3.2 第 N 行」的，出处是**该行的端口类型与全表的对应**（即本设计的判定，规范未给边）。**这张表不是全图**：它只列 §164/§165 那一串与链尾检查器；本块只**注册**算子，不建执行路径（切分 §四第 4 条）。

**这张表里的型是端口上的制品类型，不是本块函数的参数型**：`detect-edit-region` 的端口是 `Blob`（掩码制品），而 `composite_masked` / `outside_mask_difference` 收的是 `&EditMask`（位图，§4.1）——由掩码制品到 `EditMask` 的那一步（读 `BlobStore` 成位图）**是调用方的事**（§3.1 的「不给 IO 臂」）。这与 §5.2 的两个函数收 `&Raster` 而同表里写着 `Image` 是同一件事。

**本块不向 P5e 报出任何变体**（§1.2 的那一行，此处给出判据）：五枚算子的端口只用到**两枚已在枚举里的型**——`Image`（P5e 的 §3.2 清单第 1 行，本块消费）与 `Blob`（`crates/continuum-artifact/src/artifact.rs:20`，P1 既有）。**清单为空**是实测结论，不是「暂时没想到」。

## 3.5 确定性声明与 backend：**消费 P5e §5.4，不重写**

**事实（与 P5e §5.4 同源）**：`Operator.determinism` 是**算子级**字段（`crates/continuum-operator/src/definition.rs:84`），而 §245 `:756` 明写「同一个 Operator MAY 有多个 backend」，具体后端由 Router 决定。复用判定 `can_reuse`（`crates/continuum-graph/src/reuse.rs:29`）读的是 **`operator.determinism`**，不看 backend；`cache_key`（`:16`）对 `NonDeterministic` 返回 `None`（`:22`）。

**本块的处置**：

- **本块不重写 P5e §5.4 的那条注册期规则**（「`Deterministic` 的算子的每个候选 backend 必须在逐位可复现名单内」）。该规则**与领域无关**——它守的是 §305 复用的前提，不是媒体域的性质。**同一判据有两份**正是切分 §四第 7 条要避免的形态，而它的失效方向是「一套说能复用、另一套说不能」。
- **本块的两枚 `Deterministic` 算子各只有一个候选 backend**（§3.2 第 4、5 行的 `builtin`）。故「名单内 / 名单外」这一判定在本块**没有可触发的形态**（候选集合是单元集）。这是**据实留的形状**：本块因此没有该规则的第二份实现，也没有它的一份实现；缺口见 §13 第 3 条。
- **本块的 `determinism` 声明与 §305 的关系（照片，§11.2 第 (o)、(p) 条）**：第 4、5 两行的 `cache_key` 为 `Some(..)`（可用 P1 的判据复用），第 1、2、3 三行为 `None`（**永远不满足复用前提**）。**这是本块对 §305 的全部义务**（切分 §四第 7 条的第一枚实例是复用判据；本块**只让本领域算子满足或按预期不满足那条判据的前提**）。

**被否决的更显然写法：给 `local-generative-edit` 编一张「支持掩码编辑的 backend」名单**（照 P5e §5.4 对 `Deterministic` 算子的做法）。**这里不是那样：理由两条，各自独立**——(i) 那条名单的判据（backend 的什么属性使 mask 外的像素不变）规范没有一处给，编出来就是发明；(ii) §326 已经给了一条**不依赖它**的路（生成的整图经 `hard-composite` 强制回填），故名单不是必需的，而它会让「哪些 backend 可用」变成一张谁也核不动的封闭表。代价是「本可省掉一次合成」的性能——**这是性能代价，不是正确性代价**，与「编一张无判据的名单」相比取向明确。

**两枚 `NonIdempotent` 算子的重试面，据实写明**：§307 禁止非幂等效应自动重试；本块第 1、3 行是 `NonIdempotent`，故它们**不进自动重试**。代价是生成失败后要人来决定重跑——这是**代价，不是缺陷**（重跑得到另一张图，自动重试等于静默换结果）。

## 3.6 §34 的生成授权**不覆盖图像生成**（据实，不发明）

§34（`docs/spec/01-concepts.md:1417-1431`）的五个旗标逐行是 `allow_generated_broll` / `allow_generated_voice` / `allow_generated_music` / `allow_frame_interpolation` / `allow_3d_generation`（`:1424-1428`），**没有一枚对应图像生成**。`:1419` 逐字「剪辑与生成必须分开授权」，`:1431` 逐字「系统不得擅自使用生成内容替换原始素材」。

**本块的处置**：**不登记任何授权判定**，也不造一枚图像专用的旗标。三条理由：

1. **旗标是 Contract 的词汇，不是算子的**（§34 `:1421` 逐字「Task Contract 可以定义」）。本块不碰 Contract。
2. **`GenerativePermission` 归 P5e**（其设计 §7.1）。本块定义第二枚同物类型即「同一件事两个词汇表」——本仓一贯判为 Critical。
3. **编一枚规范没有的旗标就是发明**：`allow_3d_generation` 的存在说明规范**愿意**为「还没有算子的域」留旗标位置；而它**没有**为图像生成留一枚。这个「没有」是规范的选择，不是遗漏的推论——**据实记为缺口**（§13 第 4 条），收件人：规范维护者。

**后果，据实写明**：第 1 行 `generate-image` 与第 3 行 `local-generative-edit` 今天**没有授权门**——一个 Contract 未声明任何生成旗标时，本块不拦。这不是「本块放了行」，是**本块的算子集合里没有可据以拦的旗标**（§34 的清单里没有它）。若协调者判「§34 的清单是举例、图像生成按 `allow_3d_generation` 那样的同族旗标处理」，那一枚旗标归 P5e 的枚举（`continuum-media`），本块的算子面随之加一条映射——**改动是两处，不在本块内自行落地**。

---

# 4. §325 的编辑区域：Point 必须解析成语义区域

**本节是本设计里发明判据风险最大的一处**（与 §5 并列）。故本节把「规范给了什么」「规范没给什么」「本设计取什么」「失配方向是哪边」四件事分开写，不合并。

## 4.1 四种表示：照录

§325 `:2351-2358` 逐字列出四种：`Point` / `Box` / `Mask` / `SemanticObject`。§166 `:745-752` 列的是同一组的另两个名字（`Point` / `Bounding Box` / `Brush Mask` / `Semantic Object`），并在 `:772-776` 逐字写「最后都转换成 `EditMask`」。**本设计取 §325 的四个名字**（§325 是规范正文，§166 是设计说明），并记下两组名字的对应：

```
pub enum EditRegionKind { Point, Box, Mask, SemanticObject }

pub enum EditRegion {
    /// §163 :626-652：坐标归一化 `x, y ∈ [0, 1]`（实测该两句在 :648-649），避免与分辨率绑定。
    Point(NormalizedAnchor),
    /// §166 :749 的 Bounding Box。同样归一化。
    Box(NormalizedBox),
    /// §166 :750 的 Brush Mask：调用方已给出掩码位图（§166 :750 逐字如此），
    /// 故本支带的是**已经解析好的位图**，不是制品 id。
    Mask(EditMask),
    /// §166 :751 的 Semantic Object：以名字指一个对象（如 §166 :769 的「左边这个人」）。
    SemanticObject(SemanticObjectRef),
}

/// §166 :775 的 `EditMask`：四种表示解析后的共同产物。
/// **带的是掩码位图本身，不是指向掩码制品的 id**：`Box` 支的掩码由 §4.2 投影得出，
/// 那一刻仓里还没有它的制品；把它变成 `ArtifactType::Blob` 制品要经 `BlobStore::put`，
/// 而本块不做 IO（§3.1）。故此型自持位图，调用方要落盘时自己落。
pub struct EditMask {
    pub width: u32,
    pub height: u32,
    /// **单通道位图**（§3.2 第 2 行：掩码不是彩色图像）；
    /// 故本型没有 `channels` 字段——`Raster`（§5.2）有，因为它是图像。
    /// **不变量**：`pixels.len() == width as usize * height as usize`。
    /// 两个确定性函数在入口核它（§5.2 前置第 3 条，`ImageError::PixelLengthMismatch`）。
    pub pixels: Vec<u8>,
}
```

**`EditMask` 的形状是订正过的，原先那处说的是什么、为什么以现说为准**：原先是 `pub struct EditMask { artifact: ArtifactId, width: u32, height: u32 }`，即「掩码的**制品 id** 加尺寸」。**它与订正前的 §4.2 签名相抵**：`resolve_edit_region` 原先收 `&EditRegion`、`&ArtifactId`、`&RegionDetector`，不拿 `Raster`、也不拿 `BlobStore`（§3.1 明写本块不做 IO），而 `Box` 支要投影出一个掩码——**那个掩码在仓里还没有制品 id 可指**。**以现说为准的理由**：切分 §四第 6 条要求边表与实际用量精确一致（§2.1 的 `continuum-artifact` 行因此不含 `BlobStore`／`ContentHash`），而保留制品 id 就要求 `resolve_edit_region` 落盘，二者只能有一真；本块取「不落盘」这一侧，故掩码以位图在内存里传递。

**`width` / `height` 为什么仍在本型里**：§326 `:2382-2388` 的硬合成要求掩码与源图**同一坐标系**，而「分辨率是否保持」是 §168 `:825` 的一条检查——§5.2 的前置条件要比的就是这一对量与源图的那一对量。**源图的尺寸不在本型里**，也不由本函数去读：它作为 `resolve_edit_region` 的 `source_dims` 实参**由调用方给**（§4.2），因为从制品读尺寸要经 `Artifact.metadata`，而那份 schema 规范未给（§13 第 5 条，与 P5a 设计 §15 第 9 条、P5e 设计 §14 第 4 条互指）。**这正是「尺寸随掩码一起给」的落点**：掩码的尺寸随掩码（本型）给，源图的尺寸随调用给。

**三个辅助类型的形状据实留白**：`NormalizedAnchor` 的两字段按 §163 `:648-649` 取 `x, y ∈ [0, 1]`（这一条有规范出处）；`NormalizedBox` 是它的两个角（**本设计定的**，规范只给名字）；`SemanticObjectRef` 指什么**规范未给**——§166 `:769` 给的是一句自然语言（「左边这个人」），名字从哪来正是 §4.3 表里的一行缺口，故本设计**不给它定形状**（定形状就是填那条缺口）。

## 4.2 落点：**一枚算子 + 一个函数**，两处都不是新造的名字

§325 `:2360-2366` 的 MUST 是「Point MUST 被解析为 `semantic edit region`，而不是单像素修改」。**规范给了解析的路**：§164 `:682-691` 的 pipeline 逐行是 `User Point → Object / Region Detection → Semantic Mask → Local Generative Edit → Composite`；《总纲》§8.6 `:1530-1534` 把同一串再写一遍并补一句「Runtime 要根据点击位置找到 hand + held object 作为编辑区域」。

**落点两处，缺一不可**：

1. **`detect-edit-region`（§3.2 第 2 行）**——§164 `:684` 的 Object / Region Detection。它是一个**算子**，因为它要一个模型/后端（`object-detector-backend`），而 §245 `:756` 的「具体 backend 由 Router 选」要在算子面上才有落点。
2. **`resolve_edit_region`（本块的函数）**——四种表示到 `EditMask` 的**映射**，`EditMask` 是 §166 `:775` 的共同产物：

```
/// §325 :2360-2366 的唯一落点：把四种编辑区域表示解析成 §166 :775 的 `EditMask`。
///
/// **`source_dims` 是必需参数**：`Box` 支要把归一化矩形投影成像素掩码，
/// 而投影的坐标系就是源图的画布，故本函数要拿到 `(宽, 高)`。**本函数不自己读**：
/// 从制品读尺寸要经 `Artifact.metadata`（那份 schema 规范未给，§13 第 5 条），
/// 且那是一次 IO，而本块不做 IO（§3.1）。故尺寸作为实参收下。
/// **`detector` 是必需参数**：`Point` 与 `SemanticObject` 两支都要经对象/区域检测（§164 :684），
/// 故本函数**没有**任何一条「不传 detector 也能把 Point 变成掩码」的路径。
pub fn resolve_edit_region(
    region: &EditRegion,
    source: &ArtifactId,
    source_dims: (u32, u32),
    detector: &RegionDetector,
) -> Result<EditMask, ImageError>;
```

**`source_dims` 是订正时补上的**：原签名只有 `(region, source, detector)`，而 `Box` 支要把归一化矩形投影成像素掩码、`Point` 与 `SemanticObject` 两支要把归一化坐标交给 detector，**三处都要源图的画布尺寸**，它原先没有来源。补成实参而不是「本函数去读制品」：读是 IO（§3.1 不给 IO 臂），而尺寸是调用方手上就有的量（它正是拿着源制品的那一方）。

**四支各自的判据（逐支，不合并）**：

| `EditRegion` 支 | 解析方式 | 判据的来源 |
|---|---|---|
| `Box` | 归一化矩形 × `source_dims` → 掩码（**投影**，无模型） | §166 `:749`；本设计定的（规范只说它是四种之一） |
| `Mask` | 入参的掩码位图**原样**通过（含宽高与像素） | §166 `:750`（调用方已给掩码位图，`EditRegion::Mask` 因此直接带 `EditMask`） |
| `SemanticObject` | 经 `detector` 按名字找该对象的掩码 | §166 `:751`、`:769`；§164 `:684` |
| `Point` | **必须**经 `detector` 找该点所在的语义区域（§164 `:684` 的 Object / Region Detection），**不得**退化成单像素 | §325 `:2360-2366`（MUST）；§164 `:669-705` |

**`detector` 的返回值就是掩码位图**（`EditMask`）：三支要 detector 的过程都产出一张掩码，故本函数不需要第二枚「把 detector 的输出变成 `EditMask`」的类型；接口的其余形状由本块定、实现在 backend 一侧（§2.2）。

**`Point` 那一支的两侧（缺一条即不算钉住）**：

- **正例**：`detector` 给出的掩码被**原样**返回（`resolve_edit_region` 不合成像素、不改写区域）。
- **反例**：`detector` 找不到区域 ⇒ `Err(ImageError::RegionUnresolved { region: Point })`——**不是**一个单像素掩码，也**不是**整图掩码。

**为什么 `detector` 是参数而不是本块的一个字段**：§245 的 backend 由 Router 选，本块不持有 Router，也不持有后端（切分 §四第 4 条）。参数化使「哪一支要 detector」成为**类型层**的事实（要调 `Point` 那一支就必须给一个）；它**保证不了**「detector 给的是对的区域」——那是 §4.3 的缺口，**不是已解决的问题**。

## 4.3 显式处置：判据缺什么、本设计取什么

**规范给了什么（逐条实测）**：MUST（§325 `:2360`）、机制（§164 `:684` 的 Object / Region Detection）、一个例子（§164 `:699-705` 的「hand + held object」）、坐标约定（§163 `:648`）。

**规范没给什么（本设计逐条列出，缺的是哪一步写到步骤）**：

| 缺的那一步 | 规范里最接近的 | 为什么它不够 |
|---|---|---|
| 「点落在多个对象上时取哪一个」的判据 | §164 `:699-705` 的 `hand + held object` | 那是一例（手与手上的物体**同时**属于区域），不是选取规则 |
| 检测的**置信度阈值**与低于阈值时的处置 | 无 | 规范全文没有置信度这个量 |
| **零检出**时的处置（是报错、是放大到整图、还是请用户重选） | 无 | —— |
| 区域**大小的上限**（多大的区域就不再是「局部」修改） | §324 `:2344` 的 `preserve_outside_region = true` | 它说的是 mask 外不变，不是 mask 能有多大 |
| `SemanticObject` 的**名字**从哪来（自由文本？受控词表？） | §166 `:769` 的「左边这个人」 | 那是一句自然语言，不是名字空间 |

**本设计取的一条**：**零检出 ⇒ `Err(RegionUnresolved)`（fail-closed），不放大、不退化、不请模型猜**。理由：§325 的 MUST 是**否定式**的（「而不是单像素修改」），故最小的合法实现是「解析不出就拒绝」，而不是「解析不出就给一个凑合的区域」。放大到整图会让 §326 的 `outside_mask_change = FORBIDDEN` 变得无意义（mask 外为空集，任何改动都「合法」）；退化到单像素正被 §325 明文禁止。

**代价（这一条也要写，否则只写了落点与边界）**：**零检出即 `Err` 的直接代价是合法点击会被拒**。误检、稀疏对象、点击落在对象边缘之外（§164 `:699-705` 的「hand + held object」这类多对象区域尤其）都会走到这一支；而 `ImageError::RegionUnresolved` 是**终止性的**——它不重试、不放宽、不给近似区域，故每一次误拒都要**用户重选**（或重跑 detector）才能继续。取这个代价的理由是两侧不对称：误拒是一次可恢复的往返，而「放大到整图」会让 §326 的判定失去意义（`Err` 那一侧的不对称见 §5.4 的失配方向）。**这一条不声称代价小**，只声称它比另一侧的代价小。

**上表五条一条都没有在本设计里被填上**：它们是**缺口**（§13 第 1 条），收件人：规范维护者。**本设计不发明它们**——填其中任何一条，都是替 §325 定了一个规范没给的判据。

**本条不解决什么，写清**：本设计只是**给 §325 的 MUST 一个落点与一个 fail-closed 的边界**（`ImageError::RegionUnresolved`），没有让「Point → 语义区域」这件事变得有判据。**上表五条仍在**。

**本处无照片（写明）**：五条缺口都是**判据的边界**，而边界没有判据——故这里拍得到的只有「桩的返回被原样传出去」与「空返回即 `Err`」两条（§11.5 第 3 条），**拍不到「区域选得对不对」**。

## 4.4 结构性保证的两侧（照片，§11.4）

- **要 `Point` 那一支就必须给 `detector`（也要给 `source_dims`）**：少传任一实参 ⇒ 编译失败（`trybuild` 的 `compile_fail`）。这把「没有检测就没有区域」与「没有源图尺寸就投不出掩码」钉在类型上。
- **本 crate 里没有第二条从 `Point` 到 `EditMask` 的路**：`resolve_edit_region` 是唯一的转换点；`EditRegion` 的四个支共用同一个函数、同一个返回类型。**这一条的射程要写清（它是「结构事实」而不是承诺）**：它只说「本 crate 里没有绕开的路」，说不了「别的 crate 不会自己写一个 Point→整图的转换」——后者没有落点（本块的算子面只经 `Operator` 暴露）。

---

# 5. §326 `outside_mask_change = FORBIDDEN` 的确定性强制作

## 5.1 三层，逐层给出它保证到哪

| 层 | 本块的兑现 | 保证的强度 |
|---|---|---|
| **强制作（使之合法）** | `hard-composite`（§3.2 第 4 行）：mask 外的像素从**源图逐点复制**（§165 `:735-737` 逐字「mask 外像素直接从原图复制」）。函数形态见 §5.2。 | 走过这一步的结果图**按构造**满足 §326（§326 `:2378-2388` 逐字「Runtime SHOULD 使用 generated patch + hard composite 保持 mask 外原像素」） |
| **检出（发现不合法）** | `verify-outside-mask`（§3.2 第 5 行）与 §5.3 的比较器：算 mask 外的像素差（§168 `:832`），其证据由**宿主的执行代码**构造（§7.2：本块不提供这一层的函数） | **确定性**（§168 `:835` 逐字「可以直接确定性检查」），且**不读模型自述**（§168 `:837-843` 逐字「这比让模型说『我只修改了手。』可靠得多」） |
| **类型层** | **没有**。见 §5.4 | —— |

## 5.2 强制作：`composite_masked`

两个确定性函数（本节与 §5.3）收同一个**图像**像素载体，本块定义它（掩码走 §4.1 的 `EditMask`——单通道，与 `Raster` 不是同一型）：

```
/// 本块为两个确定性函数定的**最小图像像素载体**。
/// **本设计不定义像素格式**：§168 :826 只给了「Alpha / 色彩空间是否异常」这一条检查名，
/// 没有给通道语义、色彩空间或 Alpha 的表示。故本型只承载「宽、高、通道数、字节序列」，
/// 通道的语义由 backend 的约定决定——**缺的是「像素格式与色彩空间」这一步**（§13 第 5 条）。
/// 这里不是「直接收 `Vec<u8>` 加两个 usize」：三个量一起传时，调用方可以传错配的对，
/// 而合成与比较**都要**这三个量一致才有意义；收成一个值，错配在类型上就只有一个来源。
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    /// **不变量**：`pixels.len() == width as usize * height as usize * channels as usize`。
    /// 与 `EditMask` 同：两个确定性函数在入口核它（§5.2 前置第 3 条，
    /// `ImageError::PixelLengthMismatch`）。
    pub pixels: Vec<u8>,
}

/// §326 :2378-2388 的硬合成：mask 外的像素从 `source` 逐点复制，mask 内的取自 `generated`。
/// 纯函数、无 IO、无执行路径（切分 §四第 4 条只禁「执行路径」，不禁本块自己的判定函数）。
pub fn composite_masked(
    source: &Raster,
    generated: &Raster,
    mask: &EditMask,
) -> Result<Raster, ImageError>;
```

**两侧的判据**（§11.2 第 (i)、(j) 条）：

- **正例**：mask 外逐点等于 `source`（这就是 §326 的兑现）；mask 内逐点等于 `generated`。
- **反例**：**构造一个「`generated` 把 mask 外也改过」的输入** ⇒ 输出的 mask 外**仍逐点等于 `source`**。这一条才是「强制作」的照片——若只拍正例，一个「原样返回 `generated`」的实现也能过。

**入口的前置条件（逐条给出，三条都要）**：

1. **`mask` 与 `source` 的宽、高相同**：`(mask.width, mask.height) == (source.width, source.height)`；
2. **`generated` 与 `source` 的形状相同**：宽、高**与通道数**三者都相同；
3. **三个载体的 `pixels` 长度与各自声明的形状相符**：`pixels.len() == 宽 × 高`（掩码是单通道，§4.1）与 `pixels.len() == 宽 × 高 × 通道数`（两个 `Raster`，§5.2 的 `Raster` 那一段）。

第 1、2 条不成立即 `Err(ImageError::ShapeMismatch { left, right })`（`left` / `right` 取 `(宽, 高, 通道数)`；掩码那一侧的第三分量是 `None`，§3.1）；第 3 条不成立即 `Err(ImageError::PixelLengthMismatch { len, expected })`——**这一条不走 `ShapeMismatch`**，理由见本节末（那两个量化不进形状）。**这里不是「尺寸不符时按左上角对齐凑合」：** §168 `:825` 的「分辨率是否保持」是一条独立的检查，合成时按坐标截取会让「掩码与源图不同坐标系」这件事在结果里不可见——而它是 §326 的前提（掩码要真的指在同一张图上）。

**第 2 条是订正时补上的**：原先只钉了第 1 条。**它同样必需**——mask 内的像素取自 `generated`，逐点复制要求二者**逐像素一一对应**；`generated` 与 `source` 尺寸或通道数不同时（例如 3 通道的补丁填进 4 通道的图），「mask 内取自 `generated`」这一步没有定义，函数的行为是未定义的。故它与第 1 条同级，不是可省略的健全性检查。

**第 3 条为什么单出一臂（`PixelLengthMismatch` 而不并进 `ShapeMismatch`）**：前两条比的是**两个载体之间**的关系，第 3 条比的是**一个载体内部**的两件事——它自己声明的形状与它自己字节序列的长度；而 `ShapeMismatch` 的字段是 `(宽, 高, 通道数)`，**两个 `usize` 的长度值表达不进那个形状**。**原先第 3 条的报错写的是 `ShapeMismatch`，已改**（§3.1 的 `PixelLengthMismatch`）：照原样写，那句承诺在该分支上报不出它要报的两个量。

## 5.3 检出：`outside_mask_difference`

```
/// §168 :832-835 的 `outside-mask pixel difference`。
/// 确定性：逐点比较，无模型、无阈值。返回 mask 外**不相等**的像素数与总像素数。
pub fn outside_mask_difference(
    source: &Raster,
    result: &Raster,
    mask: &EditMask,
) -> Result<PixelDiff, ImageError>;
```

**入口的前置条件（与 §5.2 同形，三条）**：`mask`、`source`、`result` 三者的宽、高相同；`source` 与 `result` 的通道数相同；三个载体的 `pixels` 长度与各自声明的形状相符。前两条不成立即 `Err(ImageError::ShapeMismatch { left, right })`（形状取 `(宽, 高, 通道数)`）；第三条不成立即 `Err(ImageError::PixelLengthMismatch { len, expected })`（理由同 §5.2 末）。**这一条是订正时补上的**：原先本函数**一条前置都没有**，而「mask 外的像素」这个说法本身要求三者共用同一坐标系——掩码指到源图的哪个像素、结果图的哪个像素，只有三者宽高一致时才有定义；通道数不同则「逐点相等」没有定义。**这里不是「比较器只读不写、不必设前置」**：比较器的产出（差多少）会被下游当成判据的量，量在未定义输入上算出来的值比 `Err` 更坏。

- **两侧**：mask 外逐点相同 ⇒ `PixelDiff { differing: 0, .. }`；改一个像素 ⇒ `differing == 1`。**（§11.2 第 (l) 条）**
- **本函数不判「通过」**：它给出**差多少**，而「差多少算不通过」这个判据不在本块——证据的判定面归 P5a（切分 §四第 1 条；P5a 设计 §3 已裁定「不判充分」）。**这里不是「返回一个 bool」：** 返回 bool 就把一条判据（阈值）塞进了本块，而 §326 给的是 `= FORBIDDEN`（即阈值为 0 的那一个特例），§168 给的是「可以直接确定性检查」——**两者都不足以让本块替 P5a 定「多少算不过」**。故本块给**量**，判**在 P5a**。
- **`PixelDiff` 的形状不在本设计的约束范围内**（它是本块的返回值，随实现走）；本设计只约束「它必须含一个可以判 `== 0` 的差异量」这一条（否则 §11.2 的两侧写不出来）。

## 5.4 类型层拦不住：**据实写明**

`ArtifactType`（`crates/continuum-artifact/src/artifact.rs:14-21`）是**内容类型的枚举**：它没有、也不该有「这张 `Image` 与那张 `Image` 在 mask 外逐点相同」这种**跨制品关系**的表达力。故：

1. **§326 的 FORBIDDEN 不能靠类型表达**。它是**校验**（§5.3 的比较器）与**强制作**（§5.2 的合成），不是类型约束。
2. **本设计不为此向 P5e 要新变体**（§3.4 末：报出清单为空）。一枚 `Image` 不可能带像素级约束；要表达它得加一个与 `ArtifactType` 正交的关系类型，而 §239 的端口系统（`docs/spec/05-normative.md:638`）只有「节点输入/输出 MUST 有明确类型」这一条，没有跨制品关系的位置。

**失配方向（这是本条要回答的那一问）**：

- **当 `preserve_outside_region = true` 的节点没有接上 `hard-composite` 时**，**不报错**。产出是一张 mask 外被改动过的 `Image`，它与其他 `Image` 在类型上**完全一样**；只有跑了 §5.3 的比较器才看得见。⇒ **这一侧是 fail-open**。
- **故 §326 在本块的兑现是「提供使之成立的步骤 + 提供检出它的检查器」**，**不是**「保证它一定被用上」。后者要求「谁保证 `preserve_outside_region = true` 的节点一定接了 `hard-composite`」，而规范未给（§13 第 2 条，收件人：第 3 层的执行器与装配方）。
- **反向的一侧**：若 `hard-composite` 被接上但掩码与源图不同坐标系（或 `generated` 与源图形状不同）⇒ `Err(ImageError::ShapeMismatch)`（§5.2 的前置条件）。**这一侧是 fail-closed**，故它**不能**用来代替上面那一条：它拦的是「掩码／补丁配错」，拦不了「根本没有合成」。

**本处无照片（写明）**：拍得到的只有那两个函数的返回值（§11.2 第 (i)、(j)、(k) 条），**拍不到「图里真有一段管线没接上合成」**——那一侧的失配正是 fail-open 本身（§11.5 第 2 条）。

## 5.5 §168 的第一项检查：**本块不注册它，理由是判据不存在**

§168 `:820-827` 的四项检查里，「修改区域是否符合要求」（`:823`）是**语义判定**（要读 §324 `:2341` 的 `instruction`，判「改成这样算不算符合要求」）；其余三项（`:824` `非修改区域是否变化`、`:825` `分辨率是否保持`、`:826` `Alpha / 色彩空间是否异常`）都是**确定性**的。

**本设计取的分界**：

- **三项确定性检查 ⇒ 第 5 行 `verify-outside-mask`**（`Deterministic`）。把一项语义判定混进同一枚算子会让它的 `Deterministic` 声明为假（§3.5 的同一条理由）。
- **语义那一项 ⇒ 本块不注册算子**。理由：它的判据（「符合要求」）规范未给，而它要的是 §262 第 3 级的独立模型验证器（P5a 设计 §7.1 的五臂之一）。**给它编一枚算子等于给它编一个判据**——那正是 P5a 设计 §3 已显式处置为「不判」的那件事（OPEN-001）的同一形状。**这是据实留的缺口，不是漏项**，收件人：规范维护者（§13 第 6 条）。
- **`Alpha / 色彩空间是否异常`（§168 `:826`）落在第 5 行**，但**它的判据同样不完整**：规范给了这条检查的名字，没给色彩空间与 Alpha 的表示（§13 第 5 条）。故本设计声明：第 5 行的这一项检查**在像素格式定下来之前不可实现**——据实记，不发明。

---

# 6. §327 谱系：每次编辑产生新 Artifact

## 6.1 落点：`Artifact` 的既有字段，不新增列

§327 `:2394-2404`：`Image v1 → Image v2 → Image v3`，逐字「每次编辑必须创建新 Artifact」「不得默认覆盖原图」。§167 `:796-805` 要求在每一步记录 `source image` / `mask` / `prompt` / `model` / `seed` / `output`。

| §327 / §167 要的东西 | 落在哪个既有字段（实测） |
|---|---|
| 「新 Artifact」 | 一个新的 `ArtifactId`（`crates/continuum-artifact/src/artifact.rs:151`），经 `save_artifact` 写入既有表 `artifact`（`crates/continuum-artifact/src/persist.rs:9` 的迁移 10） |
| 谱系的**边**（v2 来自 v1） | `Artifact.input_artifacts: Vec<ArtifactId>`（`crates/continuum-artifact/src/artifact.rs:176`）——`save_artifact` 逐条写进既有的 `artifact_input` 表（同文件 `:21-25` 建表，`:71-76` 写入） |
| `v1 / v2 / v3` | `Artifact.version: u32`（同文件 `:180`），落在既有列 `artifact.version INTEGER NOT NULL`（同文件 `:16`） |
| §167 的六项每步记录 | `Artifact.provenance: Value`（同文件 `:178`）与 `input_artifacts`（`source image` 那一项就是它） |

**故 §327 不产生迁移**（§10）。**本块不向 P5e 要新变体**：谱系不是一个新 Artifact **类型**，是既有字段上的**关系**。

**一处读法，据实标注**：§240 的 `version`（`docs/spec/05-normative.md:675`）与 §327 的 `v1/v2/v3` **是不是同一个计数**，规范未明说。§241 `:683-697` 逐字「修改必须产生 `Artifact v2` 而不是覆盖 v1」，§327 逐字「每次编辑必须创建新 Artifact」——两句合读指向「沿谱系的第几步」。**本设计取这一读法**（`version` = 该 Artifact 在它那条谱系上的步号），并把「两处是不是同一个计数」记为缺口（§13 第 7 条）。**这里不是「`version` 是制品格式的版本号」的理由**：`Artifact` 已有 `content_hash` 与 `artifact_type` 表达格式与内容，再让 `version` 表示格式版本会让 §241 的「v2」无所指。

## 6.2 「不得默认覆盖原图」的判据**已由 P1 落地**：本块不建第二份

**事实（实测）**：`save_artifact`（`crates/continuum-artifact/src/persist.rs:45`）是**只写不改**的——它 `INSERT INTO artifact`（`:52-70`），而该表的 `id` 是主键（`:10`）；`:31` 的注记逐字「同 id 已存在时返回数据库错误，不覆盖（§241）」。`Artifact` 本身**没有更新路径**（该 crate 的 `persist.rs` 只有 `p1_artifact_migrations` / `save_artifact` / `load_artifact` 三个公开函数，`crates/continuum-artifact/src/persist.rs:7` / `:45` / `:90`）。

**故：本块不得再写一个「新 id ≠ 源 id」的判据**。切分 §四第 7 条要求「凡本仓已落地的判据，四个算子块不得另建第二份」；本条的落地是 `save_artifact` + 主键，**本块的兑现只是「让本领域的算子产出新 id 的制品，并把源图放进 `input_artifacts`」**——那是**算子产出的形状**，不是一条判据。

**失效方向（写清，因为这一条是「不建」）**：若本块另写一份「源图未被覆盖」的检查，两份判据的失配是「一份说覆盖了、另一份说没有」，而**两套都各有用例、都绿**（切分 §四第 7 条已把这条失效方向写给复用判据，本处同形）。

**照片（§11.2 第 (m)、(n) 条）**：用同一个 id 写第二次 ⇒ `Err`（**这是 P1 判据的照片，本块只是重跑它**）；编辑产出制品的 `input_artifacts == [source_id]` 且 `id != source_id`。

## 6.3 §167 的每步记录：**schema 缺的那一步**

§167 `:796-805` 的六项（`source image` / `mask` / `prompt` / `model` / `seed` / `output`）落进 `Artifact.provenance`（`Value`，`crates/continuum-artifact/src/artifact.rs:178`）。**该字段没有 schema**——`Value` 是结构性保存，谁的读者、写什么键，规范未给。

**这是一处跨三块的同一缺口，三处互指**：P5a 的设计 §15 第 9 条记的是「`Artifact.metadata` 的 schema 未给」（`crates/continuum-artifact/src/artifact.rs:177`），P5e 的设计 §14 第 4 条记的是「Timeline 六个字段写什么进 metadata / provenance」，本设计记的是「§167 的六项写什么进 `provenance`」。**三处记的是同一件事的三半，都留，互为指针**（切分 §八 的互指判据）。

**缺的是哪一步（写到步骤）**：「`Artifact.provenance`（与 `metadata`）的字段约定——含 §167 的六项各占哪个键、掩码是内联还是以 id 引用」这一步。收件人：规范维护者（§13 第 5 条）。

---

# 7. 与 P5a 的接缝：只产出、只消费

## 7.1 答复 P5a 设计 §14 第 3 条的询问：**够用，不请求新臂**

P5a 的设计 §14 第 3 条点名问本块：「`EvidenceType::VisualCheck` / `Benchmark` 是否够 P5e/P5f 用，若不够须向本块报出」。**本设计的答复：够，不请求新臂。**

| 本块算子的结果（由**宿主的执行代码**转成证据，§7.2） | 用的 `EvidenceType` 臂 | 判据 |
|---|---|---|
| `verify-outside-mask` 的像素比较（§5.3） | `VisualCheck` | P5a 的设计 §4.2 已在 `VisualCheck` 那一格逐字写「§326/§168 的 mask 外像素判定是它的实例」 |
| `verify-outside-mask` 的分辨率那一项（§168 `:825`） | `MetadataCheck` | P5a 的设计 §4.2 在 `MetadataCheck` 那一格逐字写「§29 的『读 resolution metadata』是它的实例」 |
| `verify-outside-mask` 的 Alpha / 色彩空间那一项（§168 `:826`） | `VisualCheck` | 它读的是像素，不是元数据；**该检查今天不可实现**（§5.5 末） |

**`Benchmark` 本块不用**：§168 的四项检查都不是性能比较。

## 7.2 本块**不写域包装**（偏离 P5e 设计 §9.2 的一处，理由写下）

P5e 的设计 §9.2 给 `continuum-media` 定义了一个 `evidence_from_media_output`，它唯一固定的事是 `subject` 取 `EvidenceSubject::Unattached`。**本块不照做**：本块**不写这一层**——证据由**宿主（第 3 层）的执行代码**直接调 P5a 的 `Evidence::from_tool_result`（P5a 设计 §4.4，`pub`）构造，不经任何域包装。**本 crate 的生产代码对这枚构造点零调用**（这也是 §2.1 把 `continuum-verify` 记为 dev 边的原因）。两条理由：

1. **一个只固定 `subject` 的包装，在两个域里各写一份，就是同一件事的两个词汇表**——本仓一贯判为 Critical。**这里不是「与 P5e 保持形状一致」**：一致性的对象是**接口**（`from_tool_result`），不是**包装层**；四块各写一个包装，等于把同一个转换写四遍。
2. **那个包装固定不了什么**。P5e 自己的设计 §12.4 已把这件事写明：`from_tool_result` 是 `pub`，「本块的生产代码可以直接调它——仍是同一个构造点，只是不经过本块的包装」。故包装的**唯一**内容是那一项 `subject`；而它**不是**类型级保证（调用方直接调构造点时，`Requirement(..)` 照样能传）。

**故本块与 P5a 之间的实际数据流是**：`detect-edit-region` / `verify-outside-mask` 两个算子的**宿主的执行代码**（第 3 层）拿本块的返回值，调 P5a 的 `from_tool_result` 造 Evidence。**本块不提供这一层的函数**；「域包装该不该存在、若该存在由谁唯一持有」记为对账条目（§12 第 3 条），收件人：P5a + P5e + 协调者。

**一条据实记录**：本块不登记 `continuum-semantics` 的边（§2.1），因为本块不构造 `RequirementId`。这与 P5e 的 §9.2 同向（那里的域包装也取 `Unattached`），差别只在**谁写那一行代码**。

## 7.3 消费面：本块不调用 P5a 的判定

`VerificationPolicy`（节点级）、`coverage`、`judge_completion` **都不在本块的调用集合里**。本块与 P5a 之间的实际数据流是**单向**的（本块提供比较器与算子，证据与判定在 P5a 一侧）；反向的那一半今天没有承载物，因为执行器未建（§11.5 第 1 条）。

**本块不定义「这一项检查通过」**：§5.3 的比较器只给**量**。**这一处分界就是切分 §四第 1 条对「只产出」的兑现方式**——本块不知道也不决定「差多少算不通过」。

---

# 8. 与 P5b 的接缝：**本块无 `bind` 目标**（显式处置一）

**事实（逐条实测）**：§187 的目录只有 `software/` `video/` `research/` `3d/` 四个，**没有 `image/`**（P5b 的设计在该条上实测过 `grep -n "image/\|images/" docs/spec/ docs/01-总纲.md docs/02-工程.md` 无命中，其第 8 节 R3）。P5b 的 `MethodDomain`（其设计 §4.1）因此**没有图像取值**，`seeded()`（§4.5）**没有图像条目**。

**本块的处置（三条）**：

1. **不调 `bind`，也不登记 `continuum-method` 依赖边**（§2.1）。本块**没有** `bind` 目标——这不是漏填，是没有可落入的目录名。
2. **不发明 `image/` 目录，也不借 `3d/` 域**。P5b 的设计 §6 的表对 P5f 那一行逐字写「不得自行建域或借 `3d/` 域」；切分 §八 的「`image/` 没有目录，而 P5f 声称有」那一条逐字写「**不在本文件里发明一个 `image/` 目录**」。
3. **本块能供的只有算子 id**。若协调者裁决补一个 `image/` 目录，**`MethodDomain` 加一臂与 `seeded()` 加条目都归 P5b**（切分 §四第 3 条：方法条目的登记形态归 P5b，四块只填内容）；那一刻本块经 `bind` 填入的内容是**本域算子的 id**（§3.1 的 `all_operators()` 里的 id），而**方法名本身属 §187 的名单**，不由本块编。

**为什么不「借 `3d/`」也不「并入 `video/`」**：两者都会让同一个 `MethodId` 落在两块的手里，而 `bind` 的语义是「重复 bind 覆盖旧值」（P5b 设计 §4.5）——**覆盖即静默丢掉先写的那一块的贡献**（P5b 设计 §5.1 把这条失效方向写明）。本块**不制造那个形态**。

**收件人：协调者**（裁决补 `image/` 目录、或裁定 image 的方法并入既有某目录）。**本条不解决什么，写清**：P5f 的算子在 P5b 的方法库里**今天没有条目**；本设计不为此加桩、不加占位条目。

---

# 9. §343 的分级后果与落地次序（显式处置二）

## 9.1 事实（逐条实测）

`docs/spec/05-normative.md:2709` 的 §343「v0.1 最小实现范围」有两张清单：

- `:2711` 起「首个符合 v0.1 的实现只需完成」的**十六项**：`Intent` / `Contract` / `Strategic Plan` / `Plan Review` / `ADFIR` / `Artifact` / `Node Scheduler` / `Model Router` / `Tool Runtime` / `Read-only Base Workspace` / `Task Workspace` / `Evidence` / `Verifier` / `Completion Predicate` / `Event Stream` / `Persistence`（`:2714-2734`）。
- `:2737` 起「可以暂不实现」的**六项**：`Distributed Compute` / `Temporary Authority` / `Full Product Drive` / **`Image Local Edit`（`:2743`）** / `Advanced Memory Synthesis` / `Runtime Self-Improvement`。

**据此的两条读法，分开写**：

1. **局部修改那一半在 v0.1 之外**——`Image Local Edit` 逐字在「可以暂不实现」里。**这是规范明写。**
2. **完整生成那一半「仍在内」是一条推断**：`generate-image`（§323 的 `GenerateImage`）**不在**六项暂缓清单里，但也**不在**十六项必做清单里。**故「在内」的判据是「未被列为暂缓」**，不是「被列为必做」。切分 §五末「§343 的分级后果」段取的是这一读法，本设计照取，并把它的**强度**据实写在此处（§14 第 1 条）。

## 9.2 本条在本块里的形态：**批次的注册边界**，不是「半截算子」

**注意一件事，否则本节会被误读**：本块对算子的「实现」**只到声明**——一枚 `Operator` 值是**纯数据**（id、版本、端口类型、determinism、side_effect_class、backend 候选），它的**执行**在第 3 层与 backend（切分 §四第 4 条）。故五枚算子值**都可以完整写出、无桩**，而 §343 的分级落在**谁进注册表**上。

| 批次 | 算子 | 判据 |
|---|---|---|
| **v0.1 批**（§3.1 的 `V01_OPERATOR_IDS`） | `generate-image` | 它不在 §343 `:2737-2746` 的六项暂缓里（§9.1 第 2 条） |
| **其后批** | `detect-edit-region` / `local-generative-edit` / `hard-composite` / `verify-outside-mask` | 四枚都在 `Image Local Edit` 那一半的链上（§3.3：局部修改 = 第 3 + 第 4 行）或它的检查器上（第 2、5 行），而 `Image Local Edit` 逐字在暂缓清单里 |

**故次序是**：先落 `generate-image`（§323 那一条链），再落局部修改的四枚（§324–§326 那一条链）。**这不改变分块**（切分 §五末原话）——五枚都在 P5f 之内，只是批次不同。

**一轮的边界要能被断出来**（§11.2 第 (q) 条）：`V01_OPERATOR_IDS` 逐项能在 `all_operators()` 里找到；且它的补集**恰为**上表「其后批」的四枚（**逐枚断言，不是只看长度**——只看长度的话，把两批的成员对调也能过）。

**本块不做的事**：不为「其后批」预留空条目（`all_operators()` 的五枚都是完整的 `Operator` 值，没有占位项），也不在 v0.1 批里塞一枚「将来会换成别的」的算子。**这里不是「照 §343 只定义 v0.1 那一枚」**：`Operator` 是数据，写全五枚**不产生任何执行路径**、不产生表、不产生迁移，而少写四枚会让 `all_operators()` 的返回类型随批次反复改（那是切分 §四第 2 条反对的「分次改动同一个枚举」的同形形态，只是对象换成了清单）。

---

# 10. 持久化：**本块零迁移**

## 10.1 结论

**本块不建表、不建迁移、不写事件。** 逐项给理由：

| 本块的产物 | 为什么不落库 |
|---|---|
| 五枚 `Operator` | `Operator` 今天**没有持久化路径**（实测：`crates/continuum-operator/` 下不存在 `persist.rs`）。注册是**装配期在内存里**做的事（`OperatorRegistry` 的 `entries: HashMap`，`crates/continuum-operator/src/registry.rs:16`） |
| `Image` 这一 Artifact 类型 | 它落进的是**既有的 TEXT 列**：`crates/continuum-artifact/src/persist.rs:11` 的 `artifact_type TEXT NOT NULL` 与 `crates/continuum-graph/src/persist.rs:42` 的同一名列——两列都**没有 CHECK 约束**。**加一枚枚举变体不产生 DDL**（该变体本身由 P5e 落，切分 §四第 2 条） |
| §327 的谱系与 §167 的六项记录 | 落进既有的 `artifact` 与 `artifact_input` 两张表（迁移 10）与 `provenance` TEXT 列（§6.1） |
| 掩码 | `ArtifactType::Blob` 的既有落盘路径：`BlobStore`（`crates/continuum-artifact/src/blobstore.rs:27`），内容按 `content_hash` 分桶。**这一行记的是它落在哪条既有路径上，不是本块的调用**——本块的函数都收 `&EditMask`（位图，§4.1），不调 `BlobStore`（§3.1、§2.1 的边表） |
| §323/§324 的五个非制品字段（`prompt` / `dimensions` / `constraints` / `instruction` / `preserve_outside_region`） | 它们同样是**节点参数**（§3.2 末的落点声明），不是制品；节点参数随图落库，图已有表（迁移 20）。**它们的键名与类型规范未给**（缺口见 §13 第 11 条） |
| 区域表示（Point / Box / SemanticObject） | 它们是**节点参数**（§4.2：`resolve_edit_region` 的入参），不是制品；节点参数随图落库，图已有表（迁移 20） |
| 证据 | P5a 的表（其设计 §10.2）。本块不写（§7） |
| §34 的授权旗标 | 住在 Contract 里（§34 `:1421` 逐字「Task Contract 可以定义」）；Contract 表属 P4。**且图像生成不在那五个旗标内**（§3.6） |
| 复用凭据 | `CacheKey`（`crates/continuum-graph/src/reuse.rs:9`）今天**也没有存储**；它的存储属 §305 的落点即 **P1**。本块不替它建表（§3.5） |

## 10.2 实测占用表（判据在切分 §二第 5 条）

**实测命令**：`grep -rn "Migration::new(" crates/`，逐处读**调用的第一个实参**（号在参数表的次行或同行）。

| 号 | 位置 | 是否进运行时装配链 |
|---|---|---|
| `1` `2` | `crates/continuum-persist/src/db.rs:29` / `:46` | 是（P0 内建，`builtin_migrations()`） |
| `10` | `crates/continuum-artifact/src/persist.rs:9` | 是 |
| `20` | `crates/continuum-graph/src/persist.rs:14` | 是 |
| `30` | `crates/continuum-workspace/src/persist.rs:21` | 是 |
| `40` | `crates/continuum-effect/src/persist.rs:27` | 是 |
| `41` | `crates/continuum-policy/src/persist.rs:30` | 是 |
| `50` | `crates/continuum-capability/src/persist.rs:53` | 是 |
| `80` | `crates/continuum-model-registry/src/persist.rs:106` | 是 |
| `60` | `crates/continuum-persist/src/bin/crash-writer.rs:16`、`crates/continuum-persist/tests/crash_atomicity.rs:8` | **否**（各自自建的库） |
| `50` | `crates/continuum-persist/tests/recovery.rs:65` | **否**（探针库） |
| `100` `101` `102` | `crates/continuum-persist/tests/migrations.rs:44` / `:68` / `:69` | **否**（测试夹具） |

「是」的九处即 `crates/continuum-runtime/src/main.rs:98` 的 `runtime_migrations()` 的装配集合：它自 `builtin_migrations()` 起逐行 `extend` 七个 crate 的迁移函数，其实测取值是 `1 2 10 20 30 40 41 50 80`。

**协调者已裁定的号段**（P3 那一轮）：A=`50`、B=`60`、C=`70`、D=`80`、E=`90`（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:480` 逐字「一个子项目一个十位档」）。P4 取 `100/110/120`。**P5a 已取 `130`**（切分 §二第 5 条）。

**实测空档**（十位档）：`140` `150` `160` `170` `180` `190` 六个档今天在**运行时装配链**里全部为空（上表的「是」九处无一落在 `130`–`199`；测试夹具占的 `100/101/102` 与 `60`/`50` 的探针副本不进装配链）。

## 10.3 本块的号段处置

**本块不占档**——零迁移，故没有号可占。**这是本设计的一处刻意决定，不是漏项**：

- 按本仓的既有取法，「预留一个没有表要建的编号就是留一条死迁移」（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:482-485` 逐字，P5e 的设计 §11.3 已引）。
- **若 P5f 的实现阶段确需落库**，那一刻取号即可，判据是**未占用**（同 p3d 的取法：「『未占用』的判据是按库说的，不是按全仓说的」）。**本设计给出的次序建议**是 `180`（P5 六块按 P3 的先例后延：P5a `130`、P5b `140`、P5c `150`、P5d `160`、P5e `170`、P5f `180`，P6 `190`）——**这是建议不是裁定**，六块与 P6 的档由协调者统一划（切分 §二第 5 条）。

**判据「我没占它」不等于「它没人占」的兑现**：上表是**全仓实测**（十五处 `Migration::new(` 调用逐处读出号），不是「按名字猜 `180` 空着」。故本块的「不占档」有实测支撑；而**若将来要占**，那一刻仍须重跑这条命令（别的块可能已取走）。

**收件人：协调者**。**这一节不另开对账条目**：本块零迁移故无号可占，号段怎么划与本块无关。

---

# 11. 测试策略与照片

## 11.1 枚举与清单的逐臂守卫

- `EditRegionKind`：四臂（§325 `:2354-2357`）各一条往返 + 数目断言（`as_str` / `parse` 两枚穷尽 `match` + `ALL` 常量，形状照 `crates/continuum-artifact/src/artifact.rs:28` / `:47` / `:68` 的既有做法）。**本型的 `ALL` 是 `const` 成立的**（与那两处同样是无字段枚举，取值可在 `const` 中直接枚举）——**这一条与下面 `all_operators()` 那条的差别就在此**，`Operator` 有字段，故它不能是 `const`（§3.1）。
- `V01_OPERATOR_IDS`：数目断言（1），且逐项在 `all_operators()` 里找得到、补集逐枚是其余四枚——**这一条的用例在 §11.2 第 (q) 条，此处不重开一条**（同一判据两处即两处维护）。
- `all_operators()`：`id` 逐枚不同（两两比较，不是只查总数）。**数目不再由一条 `len() == 5` 的断言承载**：它写在返回类型 `[Operator; 5]` 里，清单个数的改动**编译期就可见**（§3.1），比运行时断言强；补一条 `len() == 5` 的断言测的是 Rust 的 `len()`——本设计 §11.2 的「一条预防」对同形写法已有判据，故**不补**。

## 11.2 判定侧：正例 + 反例成对（缺一条即不算钉住）

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| a | 五枚算子的端口类型、`determinism`、`side_effect_class`、backend 候选**逐枚**断言（期望值手写） | §3.2 表的五行（不是抽样） |
| b | 以 `&all_operators()` 调 `register_image_operators` 注册五枚后，逐枚 `resolve` 成功 | 注册的正例 |
| c | 同一注册表注册两次 ⇒ `Err(ImageError::Registry(OperatorError::Duplicate { .. }))`（**不是**静默覆盖） | 注册的反例（§3.1：那一臂是 P1 的判定，本型只带出来） |
| c2 | 上一子例之后，注册表的内容与调用前**逐枚相同**（先查重后写，不留半注册） | 同一条规则的**写入面**（只看 `Err` 的话，「边查重边写」也能返回 `Err`） |
| d | `Box` → 掩码的**投影**：给定手写的 `source_dims`，四个角落在预期像素上（手写期望） | §4.2 的 `Box` 支 |
| e | `Mask` 原样通过：返回的 `EditMask` 与入参的掩码**逐字节相同**（含宽高） | §4.2 的 `Mask` 支 |
| f | `Point` + detector 给出的区域 ⇒ 返回的掩码**逐字节等于** detector 给的掩码（本函数不合成像素） | §4.2 的 `Point` 正例 |
| g | `Point` + detector 找不到 ⇒ `Err(RegionUnresolved { region: Point })`，**且不是单像素掩码、不是整图掩码** | §4.2 的 `Point` 反例；§325 的 MUST 的 fail-closed 侧 |
| h | 同一 `Point` 在两次调用里给出**同一个** detector 的返回（函数自身无随机） | 「合成像素」那一侧的对照（否则 f 可由「函数自己造一个区域」满足） |
| i | `composite_masked`：mask 外逐点等于 `source`，mask 内逐点等于 `generated` | §5.2 正例 |
| j | `composite_masked` 收一个「mask 外也被改过」的 `generated` ⇒ 输出的 mask 外**仍逐点等于 `source`** | **强制作的那一侧**（§5.2） |
| k | 形状不符 ⇒ `Err(ShapeMismatch { .. })`，**逐对给用例**：掩码↔源图（宽高）、`generated`↔源图（宽高）、`generated`↔源图（通道数）；`outside_mask_difference` 侧再给 `result`↔源图一对 | §5.2 与 §5.3 的前置条件（fail-closed 侧）；只拍一对的话，另两对删掉也照绿 |
| k2 | `pixels` 长度与声明形状不符（掩码侧与 `Raster` 侧**各一条**）⇒ `Err(PixelLengthMismatch { len, expected })`，**且不是 `ShapeMismatch`** | §5.2／§5.3 前置第 3 条；**这一条同时钉住「两臂没被并成一臂」**（若实现把它并进 `ShapeMismatch`，本条的臂别断言红） |
| l | `outside_mask_difference`：mask 外全同 ⇒ 差异为 0；改一个像素 ⇒ 差异为 1 | §5.3 的两侧 |
| m | 编辑产出制品的 `input_artifacts == [source_id]` 且 `id != source_id` | §6.1 的谱系边 |
| n | 用同一个 id 经 `save_artifact` 写第二次 ⇒ `Err`，且原行不变 | §6.2（**这是 P1 判据的重跑，本块不另建**） |
| o | `cache_key` 对 `hard-composite` / `verify-outside-mask` 为 `Some(..)`，且四条件齐时 `can_reuse` 为真 | §3.5（复用前提成立的一侧） |
| p | `cache_key` 对 `generate-image` / `detect-edit-region` / `local-generative-edit` **逐枚**为 `None` | §3.5（**另一侧**：否则 o 可由「一律 Some」满足） |
| q | `V01_OPERATOR_IDS` 的补集**逐枚**是「其后批」四枚 | §9.2 的批次边界 |
| r | `hard-composite` 与 `verify-outside-mask` 的 `backend_candidates.len() == 1` | §3.5 的「本块无名单检查的可触发形态」这一主张；**这一条红即说明本块需要自己那份名单检查** |
| s | §3.4 边表**逐行**：下游 `input_schema` 的每一型都能在该行列出的上游算子里找到，且那一枚的 `output_schema` 含该型（**交集中的型**，不是「两侧相等」） | §239 `:654` 的注册期一半（§3.4） |

**一条预防**：第 (a)、(b) 的期望清单**必须手写**。若 (a) 写成遍历 `all_operators()` 去比 `all_operators()`，它测的是恒等式；若 (b) 写成 `all_operators().len()`，它测的是 Rust 的 `len()`。**语料从被测清单里取就是恒真的假照片**（本仓已有此判据）。

**另一条预防**：第 (f)、(g)、(h) 三条的 detector **必须是用例自己写的桩**（一个返回固定掩码 / 一个返回空），**不得**取本块的任何名单或常量作语料——否则三条都在测「桩与它自己一致」。

## 11.3 变异预告（谁红）

四档，每档预告**哪些用例红、哪些不该红**（预告写反会让人去查不存在的问题）：

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`resolve_edit_region` 的 `Point` 支在 detector 空返回时**合成一个单像素掩码**（`Ok`） | g（`Err` 变 `Ok`） | f、h（正例那一侧不受影响）；d、e（另两支） |
| **放宽**：`composite_masked` 改成「原样返回 `generated`」 | j（mask 外不再等于 `source`） | i（mask 内仍等于 `generated`，且若语料的 mask 外恰好没被改动，i 也照过——**故 i 的语料必须有一个 mask 外被改过的对照件**，否则此档不红 = 缺用例） |
| **取反**：`ShapeMismatch` 的前置条件删掉（形状不符也照做） | k | i、j、k2（另两条前置不受影响） |
| **收紧**：把 `generate-image` 的 `determinism` 改成 `Deterministic` | p 的第一枚（`cache_key` 由 `None` 变 `Some`）；**a 的那一行**（它的期望值是手写的） | o、q（另两枚 `Deterministic` 的判据与批次边界不受影响） |
| **取反**：`register_image_operators` 改成边查重边写（先注册前几枚，遇到违规才返回） | c2（注册表内容与调用前不同） | b、c（`Err` 那一半仍绿） |
| **等价变异**：把某个 backend 候选串改成另一个同样合法的串（如 `builtin` → `core`） | **全绿**——这是一枚**等价变异体**（候选是文本，没有跨表约束）。要打红它必须换变异体：把 `hard-composite` 的候选加一个，那时 r 红 | —— |
| **放宽**：`all_operators()` 里少写一枚 | **编译失败**（返回类型是 `[Operator; 5]`，与实际元素数不符）——这一档不是用例红，是**编不过**（§3.1 的「数目改动编译期可见」）；**若连返回类型也改成 `[Operator; 4]`**，则 b 的对应枚与 q 的补集断言红 | 其余（`len()` 已不是断言，故不列；**这一档若在改类型后仍不红，说明 b 是按 `all_operators()` 自己遍历的**——即假照片） |

## 11.4 结构层的照片（`trybuild`）

本仓已有 `trybuild` 的既有做法（`crates/continuum-connector/Cargo.toml:53` 与其 `tests/type_level.rs` 的 `compile_fail`；`continuum-workspace` / `continuum-secrets` / `continuum-model-registry` / `continuum-capability` / `continuum-node` 同样登记了它）。**本块用两条**：

1. **`resolve_edit_region` 少传 `detector`（或少传 `source_dims`）⇒ 编不过**（§4.4）。这条钉的是「`Point` 那一支没有 detector 就调不动」与「`Box` 那一支没有源图尺寸就投不出掩码」。
2. **`Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现）在 `continuum-image` 里编不过** ⇒ 在本块的 crate 里**造不出** `Evidence`（P5a 设计 §4.4 的「唯一产生点」是结构事实）。**这一条在本块的分量比在 P5e 小**：本块不写域包装（§7.2），故它是本块侧唯一的一条证据面照片。

**原先拟列在这里的第一条已移出本块的照片清单**：原写「`Operator { .. , determinism: Deterministic }` 的构造不带 `backend_candidates` ⇒ 编不过」。那是 P1 的 `Operator`（`crates/continuum-operator/src/definition.rs:79-88`，七个 pub 字段、无 `Default`）的**结构事实**，不是本块造出来的照片——本块只是照它构造 `all_operators()`。

## 11.5 拍不出照片的地方

1. **端到端（一个图像 Intent 从 `Queued` 走到某处）**：执行器（第 3 层）未建，且切分 §四第 4 条禁止 P5 自建执行路径。本块的全部照片都是**单元与集成层**。
2. **§326 的「强制作一定被用上」**：谁把 `hard-composite` 接在 `local-generative-edit` 之后是装配方的图构造，本块拍不到（§5.4 的失配方向正是这一侧）。
3. **真实 detector 的区域质量**：§4.3 的五条判据缺失，故只能拍到「桩的返回被原样传出去」与「空返回即 `Err`」。
4. **`Alpha / 色彩空间是否异常`（§168 `:826`）**：像素格式与色彩空间未给（§13 第 5 条），故这一项检查拍不到「判出来对不对」，只拍得到「它在返回值里有一项」。
5. **§305 的复用真的省下了一次重算**：`can_reuse` 今天无生产调用方（P1 设计 §18 把它列在「无执行点的机制」里），且 `CacheKey` 无存储。故只能拍到「判据的返回值」（o、p 条），拍不到「少跑了一次」。

---

# 12. 与其余各块的对账条目（逐条具名）

以下每条都是**本设计假设了别的块的某枚形状**或**发现某处无归属**之处。

1. **登记（不是裁定请求）：三处「有判定/有步骤、无调用点」合并为一条，落点都在第 3 层的执行器**。本块的三处：(i) `register_image_operators`（§3.1）——注册的调用点是装配期，谁调未定；(ii) `resolve_edit_region`（§4.2）——§325 的 MUST 的落点，但「谁在什么时候解析区域」属执行路径；(iii) `hard-composite` 的**插入点**（§5.4）——§326 的 SHOULD 的兑现处。加上切分 §八「算子解析的落点仍无人认领」已有的那一处，同源：三处都在 `Queued → Running` 与图构造这两处之一。本块不自建（切分 §四第 4 条）。**收件人：第 3 层的执行器与装配方**（协调者转）。
2. **待与协调者 + 第 3 层对账（§5.4）**：`preserve_outside_region = true`（§324 `:2343`）的节点**由谁保证接上了 `hard-composite`**，规范未给。**缺的是「图构造期是否校验这条参数与下游步骤的一致性」这一步**。本块不发明它。
3. **待与 P5e 对账（三条）**：
   (i) **`ArtifactType` 的报出清单为空**（§3.4 末）——本块需要的两枚（`Image`、`Blob`）一枚已由 P5e 的清单承接、一枚是 P1 既有的。
   (ii) **域包装（`evidence_from_*`）该不该存在、若该存在由谁唯一持有**：本块**不写**（§7.2），理由是四块各写一个即同一件事的四个词汇表。**收件人：P5a + P5e + 协调者**。
   (iii) **§3.5 那条「`Deterministic` 算子的 backend 名单」规则与领域无关**（§3.5）：本块不重写它；若它该覆盖四个域，应由**一处**持有（P5e 的注册入口、或第 3 层的注册路径）。**收件人：P5e + 协调者**。
4. **待与规范维护者对账（§34 无图像旗标）**（§3.6）：§34 的五个旗标（`docs/spec/01-concepts.md:1424-1428`）无一对应图像生成，故 `generate-image` 与 `local-generative-edit` 今天**没有授权门**。**缺的是「图像生成是否也要单独授权、若要是哪一枚旗标」这一步。收件人：规范维护者。**
5. **待与规范维护者对账（四处 schema 与一处口径）**：
   (i) **§325 的判据五条缺失**（§4.3 的表）：选取规则、置信度、零检出、区域上限、`SemanticObject` 的名字空间。
   (ii) **`Artifact.provenance` / `metadata` 的字段约定未给**（§6.3）——与 P5a 设计 §15 第 9 条、P5e 设计 §14 第 4 条**是同一处缺口的三半，三处互指**。
   (iii) **掩码的编码格式未给**（§3.2 第 2 行、§5.2 `Raster` 的像素格式）。
   (iv) **§323/§324 的五个非制品字段（`prompt` / `dimensions` / `constraints` / `instruction` / `preserve_outside_region`）住在节点参数上，而节点参数的字段约定未给**（§3.2 末的落点声明、§13 第 11 条）：它们的键名、类型、取值域与「哪一枚参数属于哪一段流程」都没有规范出处。
   **收件人：规范维护者。**
6. **待与规范维护者 + P5a 对账（§168 的第一项检查）**（§5.5）：`修改区域是否符合要求`（`docs/spec/03-product-drive.md:823`）是语义判定，需要 §262 第 3 级的独立模型验证器，而**它的判据规范未给**。本块不注册那枚算子。**缺的是「这一项检查的判据与它的持有方」这一步。收件人：规范维护者 + P5a 的下一轮。**
7. **待与规范维护者对账（§240 的 `version` 与 §327 的 `v1/v2/v3`）**（§6.1）：两处是不是同一个计数，规范未明说；本设计取「沿谱系的步号」。**缺的是这一句明说。收件人：规范维护者。**
8. **待与 P5b 对账**（§8）：本块**无 `bind` 目标**，且**不登记 `continuum-method` 边**；若协调者裁决补 `image/` 目录，`MethodDomain` 的臂与 `seeded()` 的条目归 P5b，本块只供算子 id。**收件人：协调者 + P5b。**
9. **待与 P5a 对账（答复一条 + 假设一条）**：(i) 答复其设计 §14 第 3 条：`VisualCheck` + `MetadataCheck` 够用，**不请求新臂**（§7.1）。(ii) 本块假设 `Evidence::from_tool_result` 的签名（其设计 §4.4 的八个参数）在 P5a 落地时不变——**宿主（第 3 层）的执行代码**在 §7.2 的数据流上直接调它。**若它变，本块没有任何包装可以挡，改动落在调用点**——而**那个调用点不在本块内**（§7.2：本块不提供这一层的函数，§2.1 因此把 `continuum-verify` 记为 dev 边）。**收件人：P5a + 第 3 层的宿主**（本条原先只写 P5a；按 §7.2 的读法，改动落在本块之外的那一处）。

---

# 13. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **§325 的判据五条缺失**（§4.3）。**缺的是「点→语义区域」的选取规则、置信度门槛、零检出处置、区域上限、`SemanticObject` 的名字空间这五步。收件人：规范维护者。**
2. **`preserve_outside_region = true` 与下游 `hard-composite` 的一致性无人校验**（§5.4、§12 第 2 条）。**缺的是「图构造期校验这条参数」这一步。收件人：第 3 层的执行器与装配方。**
3. **`Deterministic` 算子的 backend 名单规则只写在 P5e 一处，且与领域无关**（§3.5）。本块不为两枚候选为单元集的算子建第二份。**缺的是「这条规则由哪一处唯一持有、如何覆盖四个域」这一步。收件人：P5e + 协调者。**
4. **§34 的五个旗标无一对应图像生成**（§3.6）。**缺的是「图像生成要不要单独授权、要的话是哪一枚旗标」这一步。收件人：规范维护者。**
5. **`Artifact.provenance` / `metadata` 无字段约定；掩码无编码格式；`Raster` 的像素格式未定**（§6.3、§3.2 第 2 行、§5.2）。**缺的是「§167 的六项各占哪个键、掩码怎么编码、Alpha / 色彩空间的表示」这三步。收件人：规范维护者**（与 P5a 设计 §15 第 9 条、P5e 设计 §14 第 4 条互指）。
6. **§168 的第一项检查（语义那项）没有落点**（§5.5）。**缺的是「这一项检查的判据与它的持有方」这一步。收件人：规范维护者 + P5a 的下一轮。**
7. **`Artifact.version` 与 §327 的 `v1/v2/v3` 是否同一计数**（§6.1）。**缺的是这一句明说。收件人：规范维护者。**
8. **P5f 的算子在方法库里没有条目**（§8）。**缺的是「§187 是否补一个 `image/` 目录、或 image 的方法并入哪一份既有目录」这一步的裁定。收件人：协调者。**
9. **`Benchmark` 臂在本块没有用处**（§7.1）：§168 的四项检查都不是性能比较。这不是缺口，是据实记录——**免下一轮照 P5a 的询问以为本块用过它**。
10. **本块的登记入口、区域解析函数、两个确定性函数今天都没有生产调用方**：`register_image_operators`（§3.1）、`resolve_edit_region`（§4.2）、`composite_masked`（§5.2）、`outside_mask_difference`（§5.3），四处。**这不是本块的缺口，是第 3 层的**（切分 §四第 4 条；P1 设计 §18 已有同形的一条；拍照的限制见 §11.5）。
11. **§323/§324 的五个非制品字段住在节点参数上，而节点参数的字段约定未给**（§3.2 末的落点声明、§12 第 5 条 (iv)）：`prompt` / `dimensions` / `constraints`（§323 `:2316-2331`）与 `instruction` / `preserve_outside_region`（§324 `:2335-2345`）。**缺的是「这五个键住在 `Node` 的哪一处、各自的类型与取值域、以及 `resolve_edit_region` 与 `composite_masked` 的调用方从哪一处取它们」这三步。收件人：规范维护者。** 本块只声明它们的落点是节点参数（不进 `input_schema`、不进本块的任何类型），**不替它们定形状**。

---

# 14. 在切分文档里发现的错或缺口（**只记，不改**）

逐条给出实测。**本设计不改切分文档**（切分 §七 与协调纪律）。

1. **切分 §五末「§343 的分级后果」段把「完整生成那一半仍在内」当作规范明写，而它不是。**
   实测：`docs/spec/05-normative.md:2711-2734` 是「只需完成」的十六项，**不含**任何图像算子；`:2737-2746` 是「可以暂不实现」的六项，含 `Image Local Edit`（`:2743`）。
   **故「完整生成在内」的判据是「不在暂缓清单里」，不是「在必做清单里」**——切分那一句的措辞（「**明确列为**『可以暂不实现』的含 `Image Local Edit`」）只对**局部修改**那一半成立，对**完整生成**那一半是推断。
   **动作不变**（本设计照取同一读法，§9.1），**只是把推断与明写的分量分开记**。
   **判据**：**「明写」与「未被排除」是两个强度**——一句话里对两半用了同一个「明确」，其中一半就带上了它没有的证据。
2. **切分 §五 P5f 行的「主要规范依据」栏漏了《工程》§8.2 的图像那一行。**
   实测：`docs/02-工程.md:512` 逐字「**图像局部编辑** 强制 `outside_mask_change = FORBIDDEN`（§326）。mask 外像素由原图直接复制，该判定可确定性完成（§168）。」——**这一行同时给了本块的两条主要义务**（§326 的强制作与 §168 的确定性检出），而切分那一行的依据栏只列了《工程》§8.1 第 14 行。
   **判据**：同 P5e 设计 §15 第 4、5 条（列依据时漏掉最直接的那一处）——**列依据要连「同一份文档里另有一行直接讲这件事」一起找**。
3. **切分 §四第 7 条的实例清单只有复用判据一处，本设计实测出第二处：§327 的「不得覆盖原图」。**
   实测：`crates/continuum-artifact/src/persist.rs:45` 的 `save_artifact` 只 `INSERT`（`:52-70`），`artifact.id` 是主键（`:10`），`:31` 的注记逐字「同 id 已存在时返回数据库错误，不覆盖（§241）」。
   **故 §327 的「不得默认覆盖原图」在本仓已有落点**，本块不得另建（§6.2）。切分 §四第 7 条自身写了「本层其余同类落地点一经发现即按本条办；发现一处、登记一处」，**故这不是该条的错，是该条的扩表请求**：建议把 §327 的「不得覆盖」加进 §四第 7 条的实例清单。
   **判据**：**「A 负责 X」这句话，要先问「X 的判据在仓里有没有落点」**——本条的 X 是「不覆盖」，落点在 P1 的写路径上，而参考方向是这份义务在**规范里**写在图像那一章（§327），不在 P1 那一章。
4. **已复测为正确的三条**（记在此处是为了让复审不必重做）：
   (i) 切分 §五 P5f 行的订正段说「§187 的四个目录里没有图像那一档」——与 P5b 设计第 8 节 R3 的实测一致，本设计不重复测（§8 引用它）。
   (ii) 切分 §二第 5 条说「P5a 已实测取 130（该档为空）」——本设计 §10.2 的全仓实测与之相符：`130` 在**本工作树**里仍为未占用（P5a 的迁移落库属其实现阶段，今天不在仓里）。
   (iii) 切分 §一 说的「工程 §8.1 第 14 行」（`docs/02-工程.md:498`）——实测 `:498` 逐字「| 图像领域算子 | §323 §324 §325 §326 §327 |」，是图像那一行。
5. **一处与切分无关、与派单措辞有关的实测（本项记的是「被要求照办的那条体例」，样本节号只是出处）**。
   派单要求本设计照「**一条规范没给的判定（发明判据），必须显式标出、写明它的代价与边界、给出落点、并具名收件人**」这一体例办，并给了两个节号作样本。**两个节号在本分支上都读不到**，原因不同，逐个实测：

   - 「P5e 设计 §16」：该文档的顶层小节是 §1–§15（`grep -n "^# "` 的最后四行是 §12 `:1065`、§13 `:1160`、§14 `:1218`、§15 `:1247`），**无 §16**。与它最接近的类比是它自己的 §14「遗留与未决项」。
   - 「P4 设计的 §16 遗留与未决项第 5b/5c 条」：**这两条不在本分支可见的 P4 设计里**——本分支那一份 `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md` 的 §16（标题逐字「遗留与未决项」）条目编号形如 `1.`–`17.`，不含 `5b` / `5c`。
     **它们是有的，在另一条分支上**：实测 `git show p4:docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`（`p4` = `57c8832ccf25908d353ba120fc7a7ec4d1bf73d2`）里，`5b` 在该 blob 的行 `2309`、`5c` 在 `:2319`（**那两个行号是该 blob 的行号，不是本分支任何文件的行号**）。
     **分支别是原因**：本工作树从 `p5e` 出，而这两条是 2026-10-09 在 P4 Task 2 实现期补入的。
     **故本项只声明「本分支读不到」，不声明「那两处不存在」**——关于另一棵树里那份文件的全称断言会随它主人的下一次编辑静默变假，本设计不写那种句子。

   **处置**：本设计照**那条要求本身**办，不照节号。要求在两处最吃紧的地方逐条兑现：

   - **§4.3**（`Point` → 语义区域）：判据缺口**五条**逐条列出（选取规则 / 置信度 / 零检出 / 区域上限 / `SemanticObject` 的名字空间）、本设计取的那一条与它的**边界**（fail-closed，不退化成单像素、不放大到整图）、**落点**（`detect-edit-region` + `resolve_edit_region`）、**具名收件人**（规范维护者），并写明「本条不解决什么」（五条仍在）；**无照片处另起一句写明**（见那一段末）。
   - **§5.4**（`outside_mask_change = FORBIDDEN` 的确定性强制作）：**代价与边界**（类型层拦不住）、**失配方向**（fail-open 那一侧与 fail-closed 那一侧分开写）、**落点**（`hard-composite` + `outside_mask_difference`）、**具名收件人**（第 3 层的执行器与装配方）；**无照片处另起一句写明**。
   - 同类处置另见 §3.5、§3.6、§5.5、§13（每条都写到「缺的是哪一步」并具名收件人）。

   **体例的四个成分，逐项对照**（读 `p4` 那两条得到的形状：**声明这是发明判据** + **代价与边界照实记（含实测）** + **无照片处写明** + **收件人，并指出这条补的是哪一处空位**）：本设计 §4.3、§5.4 四条齐全，第 4 条（无照片）见 §11.5 第 2、3 条与 §4.3、§5.4 各自的末句。

   **收件人：协调者**（派单措辞里的节号定点订正，不是文档缺陷；本条留在本设计里，是为了让复审不必再去别的分支找那两处，也不必在本分支里白找）。
