# P5f（图像领域算子）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建一个新 crate `continuum-image`，落 §323–§327 的两条链与 §326 的确定性强制作：
五枚**可注册算子**的声明（`generate-image` / `detect-edit-region` / `local-generative-edit` /
`hard-composite` / `verify-outside-mask`）与它们的注册入口、编辑区域的四种表示与 §325 的唯一落点
（`resolve_edit_region`）、两个确定性函数（`composite_masked` / `outside_mask_difference`）、
以及 §343 的批次边界（`V01_OPERATOR_IDS`）。
本块**只声明算子，不实现算子的执行体**（`OperatorImpl` 不在交付物里，切分 §四第 4 条）。

**Architecture:** 本块是**跨领域层（第 8 层）的一个叶子块**。它经 `Operator` / `OperatorRegistry`
（`crates/continuum-operator`，P1）与第 3 层相接，**不新开跨层边**（切分 §三 末段；
《工程》§8.3 的 `领域算子 ← 第 3 层 Operator 注册表 ＋ 第 4 层 Router` 是既有的一行）。
落地形态是：`continuum-image` 只登记**实际用到**的 workspace 内边（切分 §四第 6 条）——
普通边两条（`continuum-artifact` 的 `ArtifactType` 与 `ArtifactId`、`continuum-operator` 的算子类型）、
dev 边三条（`continuum-graph` 的复用判定、`continuum-persist` 的写读夹具、`continuum-verify` 的 `Evidence`）。
本块**零表、零迁移、零事件**（设计 §10），**不碰 `continuum-runtime` 的装配面**。

**本块是声明与判定，不是执行**：「算子的实现体在哪里」在本块**没有答案**（交付物里没有 `OperatorImpl`）；
「mask 外像素被改动过的 `Image`」不可表达（`ArtifactType` 是内容类型的枚举，没有跨制品关系的表达力，设计 §5.4）；
「解析不出区域时给一个凑合的区域」不可表达（`Point` 那一支零检出即 `Err`，且没有第二条从 `Point` 到 `EditMask` 的路，设计 §4.2）；
「本块自造证据类型」不可表达（本块不建证据构造代码，设计 §7.2）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-artifact`（`ArtifactType` / `ArtifactId`）；
`continuum-operator`（`Operator` / `OperatorId` / `OperatorVersion` / `Determinism` / `SideEffectClass` /
`BackendId` / `OperatorRegistry` / `OperatorError`）；`thiserror`；
dev：`continuum-graph`（`cache_key` / `can_reuse` / `CacheKey`）、`continuum-persist`（`Db` / `Tx`）、
`continuum-verify`（`Evidence`）、`tempfile`、`trybuild`。
**本块不引入 `serde`**：本块的所有类型都不序列化，`Operator` 与 `ArtifactType` 的派生由 P1 提供。

**设计依据：** `docs/superpowers/specs/2026-10-09-p5f-image-domain-operators-design.md`（**唯一事实来源**，887 行）。
切分与冻结口径：`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`。
规范：`docs/spec/05-normative.md`（§239 §240 §241 §244 §245 §305 §323–§327 §343）、
`docs/spec/03-product-drive.md`（§161–§168）、`docs/spec/04-method.md`（§187）、
`docs/02-工程.md`（§8.1 第 14 行 `:498`、§8.2 图像那一行 `:512`、§8.3 `:527`）、`docs/01-总纲.md`（§8.6 `:1504`）。
**本计划不改设计、不改规范、不改切分文档、不改代码以外的任何计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-10 实读工作树）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| P1 `continuum-operator`：`Operator` 七个 pub 字段（`crates/continuum-operator/src/definition.rs:79-88`）、`OperatorId`（`:24-31`，字段私有、`new` 非 `const fn`）、`OperatorVersion`（`:45-53`）、`BackendId`（`:66-73`）、`Determinism`（`:8-11`）、`SideEffectClass`（`:16-20`）、`OperatorRegistry::{register, resolve}`（`crates/continuum-operator/src/registry.rs:24`、`:36`）、`OperatorError::{NotFound, Duplicate}`（`:7-12`） | **已交付** | 五枚算子的类型、注册入口、`ImageError::Registry` 的来源臂 |
| P1 `continuum-artifact`：`ArtifactType` 六型（`crates/continuum-artifact/src/artifact.rs:14-21`）、`ALL`（`:28`，带数目字面量 `[ArtifactType; 6]`）、`as_str`（`:47`）/ `parse`（`:68`）、`ArtifactId`（`:151`）、`Artifact`（`:169-181`：derive 在第 169 行、`pub struct Artifact {` 在第 170 行，含 `input_artifacts`、`version`、`provenance`）、`ContentHash::of`（`src/content.rs:11`）、`save_artifact`（`src/persist.rs:45`）/ `load_artifact`（`:90`）/ `p1_artifact_migrations`（`:7`）；**`ArtifactType` 今天没有 `Image`** | **已交付**（**但缺 `Image`**） | 五枚算子的端口类型（Task 2）；`resolve_edit_region` 的 `source` 实参与 detector 的接口（Task 5）；Task 7 的写读路径 |
| P1 `continuum-graph`：`CacheKey`（`crates/continuum-graph/src/reuse.rs:9`）、`cache_key`（`:16`）、`can_reuse`（`:29`）；另 `OperatorImpl::execute`（`src/execution.rs:76`）、`is_candidate_backend`（`:87`） | **已交付** | Task 7 的 §3.5 复用照片（**dev 边**） |
| P1 `continuum-persist`：`Db::{open, open_with, migrate, begin}`（`crates/continuum-persist/src/db.rs:68`、`:72`、`:96`、`:147`）、`Tx::{query, execute, commit}`（`src/tx.rs:26`、`:30`、`:41`）、`builtin_migrations`（`src/db.rs:27`）、`PersistError` | **已交付** | Task 7 的 §6 谱系照片要起真库（**dev 边，本计划自定**，见 `## 遗留` 第一节第 1 条） |
| **P5e 的 `ArtifactType::Image`**（P5e 设计 §3.2 第 1 行：`Image` / `image`；落地后 `ALL` 是 `[ArtifactType; 17]`） | **未交付**（实测 `crates/` 下无 `continuum-media`；`ArtifactType` 六型里没有 `Image`；`docs/spec/05-normative.md:645` 的 `Artifact<Image>` 今天不可表达） | **Task 2 的端口类型；Task 2 的硬前置** |
| P5a 的 `continuum-verify`：`Evidence` 八字段全私有、无 `Default`、无 `From`/`FromStr`、唯一构造点 `Evidence::from_tool_result(...)`（P5a 设计 §4.4） | **未交付**（实测 `ls crates/` 无 `continuum-verify`；`grep -rln "continuum-verify"` 的命中**全部落在五份设计文档里**） | **Task 8 的 trybuild 照片；Task 8 的硬前置** |
| P4 `continuum-semantics` 的 `RequirementId` | **未交付**（`ls crates/` 无） | **本块不用**——本块不构造归属、不产证据的包装，故设计 §2.1 的边表里没有它 |
| P5b `continuum-method` | **未交付**（`ls crates/` 无） | **本块不用**——本块**无 `bind` 目标**（设计 §8），§2.1 明写不登记该边 |
| 本块新建的 `continuum-image` | **不存在**（实测 `grep -rn "continuum-image" crates/ Cargo.toml` **exit 1、零命中**；不含限定时的全仓命中全落在本块的设计文档自身） | 本计划的全部交付物 |

**一句话结论**：本块**依赖 P1 的既有产物**（已交付，接口逐条实测过），
**依赖 P5e 与 P5a 的两个未交付物**，**不依赖 P4 / P5b**。
P1 的接口按实测写；后两者的接口**只能照设计写**，落地时以 crate 源码为准。

### 二、哪些 task 在 P5e / P5a 未交付前动不了

- **Task 2（五枚算子的声明）** 的端口类型要写 `ArtifactType::Image`，
  而**那个变体今天不存在**（上表实测）。写它即 `error[E0599]`／把类型写成别的型即语义错。
  **硬前置：P5e 已落地 `ArtifactType::Image`**（P5e 设计 §3.2 第 1 行）。
  **本块不因此加一个本地占位型**：端口类型是 `Vec<ArtifactType>`，占位型会改掉 `Operator` 的形状与 P1 的判据。
- **Task 8（两张结构层的编译期照片）** 的三份样例里有一份要写 `continuum_verify::Evidence`
  ——**那个 crate 今天不存在**（上表实测）。`use` 不到、编不过、`cargo test` 也起不来。
  **硬前置：P5a 的 `continuum-verify` 已交付**（P5a 设计 §4.4）。
- **除 Task 2 与 Task 8 外，其余各 task 不直接依赖任何未交付的 crate**：它们只新建
  `crates/continuum-image/**`，改两处**共写文件**（`Cargo.toml` 的 `members` 与
  `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`），两处都在切分 §一 的共写文件表里。
  **但「不直接依赖未交付的 crate」不等于「今天可以开工」**：**今天可开工的只有 Task 1**，
  以及随后只依赖 Task 1 的类型的 **Task 5 与 Task 6**；
  **Task 3、Task 4、Task 7 的夹具都是 Task 2 的 `all_operators()`**（下一段与各自的 `Interfaces` 逐条写明），
  故它们**随 Task 2 一起被 P5e 的 `ArtifactType::Image` 挡住，今天动不了**。
- **各 task 会改 `ALLOWED` 里同一个内层数组**（`("continuum-image", &[…])` 那一项）：
  Task 1 建起时只有 `continuum-operator`，Task 2 加 `continuum-artifact`，
  Task 7 加 `continuum-graph` 与 `continuum-persist`，Task 8 加 `continuum-verify`
  （终态见上文 `Global Constraints`）。**并行执行会让该行互相覆盖**——这是文件所有权问题，不是依赖关系问题：
  需要并行时须**串行化对那一行的改动**，或由最后合入的一方核对五项都在。
- **Task 1 与 Task 2 之间有真依赖**（Task 2 的 `ImageError` 臂、`EditMask` 都在 Task 1），
  其余各 task 之间**除下面这一条外没有硬序**：**Task 3、Task 4、Task 7 的夹具都取自 Task 2 的声明
  （`all_operators()`）**，Task 5／Task 6 用 Task 1 的类型。**Task 8 排在最后、Task 2 紧跟 Task 1**，
  是两条硬前置（Task 2 要 P5e 的变体、Task 8 要 P5a 的 crate）的直接后果，不是一条风格偏好。

### 三、本块对既有之物的请求（**本计划只作消费方写，一处都不实施**）

设计 §12 与 §13 已列。收件人逐条照原文，本计划**不为它们建 task**——它们要改的文件属别的层或别的块，
改它们会让同一个文件有两个改动方。

| 处 | 请求的内容 | 本块的角色 | 收件人 |
|---|---|---|---|
| 两个算子的**无授权门**（设计 §3.6）：§34 的五个旗标（`docs/spec/01-concepts.md:1424-1428`）无一对应图像生成 | `generate-image` 与 `local-generative-edit` 今天没有可据以拦的旗标。**本块不发明第六枚旗标**，也不登记任何授权判定 | **消费方**（只声明算子） | 规范维护者 |
| **§325 的判据五条缺失**（设计 §4.3 的表）：选取规则 / 置信度门槛 / 零检出处置 / 区域大小上限 / `SemanticObject` 的名字空间 | 本块只给 §325 的 MUST 一个落点与一个 fail-closed 的边界，**不让这件事变得有判据** | **落点方**（`resolve_edit_region` 的 `Err(RegionUnresolved)`） | 规范维护者 |
| **`preserve_outside_region = true` 与下游 `hard-composite` 的一致性无人校验**（设计 §5.4） | 「谁保证带这条参数的节点一定接了 `hard-composite`」规范未给。缺的是「图构造期校验这条参数与下游步骤的一致性」这一步 | **消费方**（提供使之成立的步骤与检出它的检查器，**不保证它一定被用上**） | 第 3 层的执行器 ＋ 装配方 |
| **五个非制品字段住在节点参数上，而节点参数的字段约定未给**（设计 §3.2 末、§13 第 11 条）：`prompt` / `dimensions` / `constraints` / `instruction` / `preserve_outside_region` | 本块只声明它们「不进 `input_schema`、不进本块的任何类型」，**不替它们定形状** | **报出方**（不发明） | 规范维护者 |
| **`Artifact.provenance` / `metadata` 无字段约定；掩码无编码格式；像素格式与色彩空间未给**（设计 §13 第 5 条） | 三处与 P5a 设计 §15 第 9 条、P5e 设计 §14 第 4 条**互指** | **报出方**（不发明） | 规范维护者 |
| **§168 的第一项检查（语义那项）没有落点**（设计 §5.5）：它要 §262 第 3 级的独立模型验证器，而判据未给 | 本块**不注册**那枚算子（给它编一枚算子等于给它编一个判据） | **落点方**（只注册三项确定性检查的那一枚：`verify-outside-mask`） | 规范维护者 ＋ P5a 的下一轮 |
| **`Artifact.version` 与 §327 的 `v1/v2/v3` 是否同一计数**（设计 §6.1、§13 第 7 条） | 本设计取「沿谱系的第几步」；缺的是这一句明说 | **消费方**（不新增列） | 规范维护者 |
| **`Deterministic` 算子的 backend 名单规则只写在 P5e 一处，且与领域无关**（设计 §3.5、§13 第 3 条） | 本块的两枚 `Deterministic` 算子候选是**单元集**，故那条规则在本块**没有可触发的形态**；本块**不重写它、也不给它建第二份** | **消费方** | P5e ＋ 协调者 |
| **§187 的四个目录里没有 `image/`**（实测 `docs/spec/04-method.md:117` / `:126` / `:132` / `:138` 是 `software/` / `video/` / `research/` / `3d/`，**无 `image/`**） | 本块**无 `bind` 目标**：不调 `bind`、不登记 `continuum-method` 边、**不发明 `image/` 目录、不借 `3d/` 域**（设计 §8） | **无目录可填的那一块** | 协调者 ＋ P5b |
| 迁移档号（设计 §10.3） | 本块**零迁移**，故没有号可占；设计给的建议是 `180`，**那是建议不是裁定** | **登记方**（零迁移） | 协调者 |

### 四、本节把什么交给谁

- **交给 P5e**：`ArtifactType::Image` 的落地（本块只消费）；以及**本块向 P5e 报出的变体清单为空**
  ——判据实测：五枚算子的端口只用两枚型，`Image`（P5e 的清单承接）与 `Blob`
  （`crates/continuum-artifact/src/artifact.rs:20`，P1 既有）。
- **交给 P5a 的 `continuum-verify`**：本块给它的答复是**够用、不请求新臂**
  （`VisualCheck` ＋ `MetadataCheck` 覆盖 §5.3 的两项确定性检查；`Benchmark` 本块不用）。
  两张结构层的照片要用到它的 `Evidence`（Task 8）。
- **交给第 3 层的执行器（未建）**：五枚算子的 `OperatorImpl`、`Queued → Running` 处的 `resolve` 调用点、
  `hard-composite` 的**插入点**（§326 的 SHOULD 的兑现处）、`resolve_edit_region` 的**调用时机**、
  以及本块两个入口（`all_operators` / `register_image_operators`）的生产调用方——
  **五处今天都没有落点**（设计 §13 第 10 条与 §12 第 1 条；P1 设计 §18 已有同形的一条）。
  本块的入口**今天没有生产调用方**，这不是本块的缺口。
- **交给装配方**：`preserve_outside_region = true` 的下游步骤一致性（设计 §5.4 的 fail-open 那一侧）。
- **交给协调者**：§187 是否补 `image/` 目录（或 image 的方法并入既有某份目录）、迁移档的合并、
  以及本计划查出的设计问题（逐条见 `## 遗留` 第一节，每条给翻转条件或具名收件人）。
- **交给规范维护者（本项目无此角色）**：§325 的五条判据、节点参数的字段约定、
  `provenance` / `metadata` 的字段约定、掩码的编码格式、像素格式与色彩空间、
  §168 第一项检查的判据与持有方、§240 的 `version` 与 §327 的 `v1/v2/v3` 是否同一计数、
  §34 是否有图像生成的旗标。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划新增的 workspace 内边只有这些**（逐条由需要它的 task 登记，**一次不声明齐**）：
  - Task 1：`continuum-image ← continuum-operator`（`OperatorError` 进 `ImageError::Registry` 的来源臂）；
  - Task 2：`continuum-image ← continuum-artifact`（`ArtifactType` 进 `Operator` 的端口字段、
    `ArtifactId` 进 Task 5 的签名）；
  - Task 7：`continuum-image ← continuum-graph`（**dev 边**，`cache_key` / `can_reuse` / `CacheKey`，
    唯一使用点是 `tests/reuse.rs`）、`← continuum-persist`（**dev 边**，`Db` / `Tx`，
    唯一使用点是 `tests/lineage.rs`——**这一条不在设计 §2.1 的边表里，是本计划自定的一处，见 `## 遗留` 第一节第 1 条**）；
  - Task 8：`continuum-image ← continuum-verify`（**dev 边**，`Evidence`，唯一使用点是 trybuild 的编译失败样例）。
  **终态**：`("continuum-image", &["continuum-artifact", "continuum-graph", "continuum-operator", "continuum-persist", "continuum-verify"])`
  ——**设计 §2.1 给的是四项**（不含 `continuum-persist`），**本计划加的是第五项**（数组按字母序）。
  **上表之外一条边都不登记**——尤其没有指向 `continuum-core` / `continuum-events` / `continuum-policy` /
  `continuum-capability` / `continuum-port` / `continuum-semantics` / `continuum-node` / `continuum-secrets` /
  `continuum-sandbox` / `continuum-workspace` / `continuum-provider` / `continuum-connector` /
  `continuum-model-registry` / `continuum-method` / `continuum-media` 的边（设计 §2.1 的「不登记的边」段）。
  **外部 crate（`thiserror` / `tempfile` / `trybuild`）不进 `ALLOWED`**——那张表只逐对断言
  workspace 成员之间的边（`crates/continuum-runtime/tests/dependency_direction.rs:22-24` 的注记）。
- **dev 边也要登记**：`dependency_direction.rs` 的 `cargo_tree_direct` 带 `--edges all`
  （`:270-276` 的实参里有 `--edges`），dev 边与普通边一视同仁。本仓已有同形先例
  （表内 `continuum-provider` 那一行（`:32-34`）与 `continuum-connector` 那一行的注记各写着「**dev 边**」）。
- **`--depth 1` 只钉直接边，这就是这条规则的完整粒度，本计划不另设闭包断言。**
  Rust 的 crate 可见性要求**直接声明**才能 `use`，未写进 `Cargo.toml` 的传递依赖**写不出**
  `use continuum_method::…`。故「直接边」恰是这条规则的完整粒度——**写这一段的用途**：
  免得读到上面那条约束时以为「任何指向别层的边都会变红」，**那会是一条假保证**。
- **本块零迁移、零表、零事件。** 实测（2026-10-10）：全仓 `Migration::new(` 十五处，号是
  `1` `2`（`crates/continuum-persist/src/db.rs:29` / `:46`）、`10`（`crates/continuum-artifact/src/persist.rs:9`）、
  `20`（`crates/continuum-graph/src/persist.rs:14`）、`30`（`crates/continuum-workspace/src/persist.rs:21`）、
  `40`（`crates/continuum-effect/src/persist.rs:27`）、`41`（`crates/continuum-policy/src/persist.rs:30`）、
  `50`（`crates/continuum-capability/src/persist.rs:53`；另一处 `50` 在 `crates/continuum-persist/tests/recovery.rs:65`）、
  `60`（`crates/continuum-persist/src/bin/crash-writer.rs:16`；另一处 `60` 在
  `crates/continuum-persist/tests/crash_atomicity.rs:8`）、`80`（`crates/continuum-model-registry/src/persist.rs:106`）、
  `100` `101` `102`（`crates/continuum-persist/tests/migrations.rs:44` / `:68` / `:69`）。
  进**运行时装配链**的九处是 `1 2 10 20 30 40 41 50 80`（`crates/continuum-runtime/src/main.rs` 的
  `runtime_migrations()`）；`130`–`199` 这一段在装配链里**全空**。
  **本块不新增任何迁移，故不动 `runtime_migrations()`。**
- **本块不碰 `crates/continuum-runtime/src/**`。** 本块没有装配面：算子注册进 `OperatorRegistry`
  这一步的调用点在装配方 / 第 3 层，今天不存在（跨计划前置第四节）。本块在 `continuum-runtime`
  下的改动**只有一条 `ALLOWED` 条目**，落点是 `tests/dependency_direction.rs`。
- **本块对切分 §一 共写文件表的改动面只有两处**：`Cargo.toml` 的 `[workspace] members` 加一行
  `crates/continuum-image`；`dependency_direction.rs` 的 `ALLOWED` 加 `continuum-image` 条目。
  `crates/continuum-artifact/src/artifact.rs`（归 P5e）、`crates/continuum-graph/src/node.rs`（归 P5a）、
  `crates/continuum-operator/src/definition.rs`（归 P5e）**本块一处都不碰**。
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets`
  同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不实现五枚算子的 `OperatorImpl`（执行体）**（切分 §四第 4 条；
    `crates/continuum-graph/src/execution.rs:76-82` 的 `OperatorImpl::execute` 本块不实现任何一枚）。
  - **不建执行路径、不做 `Queued → Running` 的迁移、不写 `OperatorRegistry::resolve` 的调用点**（同上）。
  - **不建第二枚证据包装**（不定义 `evidence_from_image_output`），**不写任何证据构造代码**
    （设计 §7.2；协调者 2026-10-10 的四块统一口径写在 P5a 设计 §4.4）。本块的 `src/` 里
    `Evidence` 与 `EvidenceType` **零命中**。
  - **不调 `bind`、不登记 `continuum-method` 边、不发明 `image/` 目录、不借 `3d/` 域**（设计 §8）。
  - **不注册 §168 第一项检查（语义那项）的算子**（设计 §5.5：判据不存在，给它编一枚算子等于给它编一个判据）。
  - **不加 `ArtifactType` 的任何变体、不向 P5e 报出任何变体**（设计 §3.4 末：报出清单为空）。
  - **不定义 §34 的 `GenerativePermission`、不造图像专用的授权旗标、不登记任何授权判定**（设计 §3.6）。
  - **不重写 P5e 的 §3.5 那条 backend 名单规则、也不建第二份**（设计 §3.5）。
  - **不建第二份复用判据**（§305 已由 `crates/continuum-graph/src/reuse.rs:29` 落地）、
    **不建第二份端口兼容判定**（§239 由 `crates/continuum-port/src/port.rs:94` 落地）、
    **不建第二份后端候选判定**（§245 由 `crates/continuum-graph/src/execution.rs:87` 落地）、
    **不建第二份「制品不得被默认覆盖」的判据**（§327 的落点是 `artifact.id` 主键 ＋
    全仓没有任何 `UPDATE artifact` 这条缺席，切分 §四第 7 条第四处——**它没有承重的函数**，
    列它时要连这一点一起列）。本块的兑现只是「让本领域的算子产出新 id 的制品，并把源图放进
    `input_artifacts`」，**那是算子产出的形状，不是一条判据**（设计 §6.2）。
  - **不做 IO**：本块不调 `BlobStore` / `ContentHash` / `serde_json`，不落盘、不读制品
    （设计 §3.1 的「不给 IO 臂」）。掩码以位图在内存里传递（`EditMask`，设计 §4.1）。
  - **不定义掩码的编码格式、不定义像素格式与色彩空间**（设计 §13 第 5 条）。
  - **不建表、不写迁移、不写事件**（设计 §10.1）。
  - **不改 `docs/02-工程.md`、不改六份设计、不改切分文档**（设计 §14 的「只记，不改」）。

  **设计 §14 的四条（在切分文档里发现的错或缺口）在本块的落点逐条如下**——它们不改切分文档，
  但**要看得出本块按哪一条做**：
  - **第 1 条**（切分 §五末「§343 的分级后果」段把「完整生成仍在内」当作规范明写）：
    落点是 Task 2 的 (q)——`V01_OPERATOR_IDS` 的补集**逐枚**断言。**强度据实**：
    「局部修改在 v0.1 之外」是规范明写（`docs/spec/05-normative.md:2737` 的清单含
    `:2743` 的 `Image Local Edit`），而「完整生成仍在内」是**推断**（`generate-image`
    **不在**那份暂缓清单里，也**不在** `:2711` 的必做十六项里）。
  - **第 2 条**（切分 §五 P5f 行的依据栏漏了《工程》§8.2 的图像那一行）：落点是本计划开头的
    **Goal 与设计依据两处都列了 `docs/02-工程.md:512`**（该行逐字：
    「**图像局部编辑** 强制 `outside_mask_change = FORBIDDEN`（§326）。mask 外像素由原图直接复制，
    该判定可确定性完成（§168）。」——它同时给出本块的两条主要义务）。
  - **第 3 条**（切分 §四第 7 条的实例清单只有复用判据一处，本设计实测出第二处：§327 的「不得覆盖原图」）：
    落点是本节「不建第二份判据」里的四条，其中**第四条连它的「没有承重的函数」一起列**。
    设计 §14 第 3 条已记「该扩表请求已被执行（2026-10-10），切分 §四第 7 条现列四处」。
  - **第 4 条**（已复测为正确的三条）：**本块不重复测**——它记的是「切分 §五 P5f 行的订正段、
    切分 §二第 5 条、切分 §一 的工程 §8.1 第 14 行」三条已被实测确认，
    本计划**不复述它们的行号**（指进共享文档按内容指）。

### 本计划特有的「签名即判据」，照抄前须对源

- `Operator` 的七个 pub 字段（`crates/continuum-operator/src/definition.rs:79-88`）与
  **`OperatorId` 的字段私有、`new` 不是 `const fn`、`Operator` 含三枚 `Vec` 字段**（`:24-31`、`:79-88`）
  ——故 `all_operators()` 是**函数不是常量**（设计 §3.1 的两条理由各自独立成立）。
  引用本仓「带 `ALL` 常量的既有做法」（`crates/continuum-artifact/src/artifact.rs:28`、
  `crates/continuum-events/src/audit.rs:38`）时，**要连它的成立条件一起搬**：那两处是**无字段枚举**。
  **`V01_OPERATOR_IDS` 与 `EditRegionKind::ALL` 的 `const` 是成立的**——它们的元素型是 `&str`
  与一枚无字段枚举，与上面两条同形的理由是「无字段」而不是「名字里有 ALL」。
- `OperatorRegistry::register(&mut self, operator: Operator) -> Result<(), OperatorError>` 与
  `resolve(&self, id: &OperatorId, version: &OperatorVersion) -> Result<&Operator, OperatorError>`
  （`crates/continuum-operator/src/registry.rs:24`、`:36`）；`OperatorError::Duplicate { id, version }`（`:11`）
  与 `NotFound { id, version }`（`:9`）。
  **`OperatorRegistry` 的 `entries` 是私有字段、没有迭代器、没有 `len()`**（`:15-17` 实测）
  ——故「注册表的内容与调用前逐枚相同」这件事**只能经 `resolve` 观测**（Task 4 的 (c2)）。
- `ArtifactType` 的 `ALL: [ArtifactType; 6]`（`crates/continuum-artifact/src/artifact.rs:28`）、
  `as_str`（`:47`）/ `parse`（`:68`）。**`parse` 收 `&str`、一定有通配臂**，
  故它**不是**「加一臂即编译不过」那张照片——那是 `as_str` 与枚举上的穷尽 `match` 才有的性质。
  **本块不新增 `ArtifactType` 的任何臂**（Task 2 只消费 `Image` 与 `Blob`）。
- `cache_key(operator: &Operator, input_hash: ContentHash) -> Option<CacheKey>`
  （`crates/continuum-graph/src/reuse.rs:16`）与
  `can_reuse(operator: &Operator, cached: Option<&CacheKey>, current_input_hash: &ContentHash, contract_affected: bool) -> bool`
  （`:29`）。**它读的是 `operator.determinism`，不看 backend**——这正是设计 §3.5 那处结构缺口的来源。
- `ContentHash::of(bytes: &[u8])`（`crates/continuum-artifact/src/content.rs:11`）。
- `save_artifact(tx: &Tx<'_>, artifact: &Artifact, event_id: &str, occurred_at: i64) -> Result<(), PersistError>`
  （`crates/continuum-artifact/src/persist.rs:45`）与 `load_artifact(tx: &Tx<'_>, id: &ArtifactId) -> Result<Option<Artifact>, PersistError>`
  （`:90`）。**`save_artifact` 不自己 `commit`**，且它的文档注释逐字写着
  「返回 `Err` 之后调用方必须回滚该事务，不得提交」——Task 7 的 (n) 以此为前提。
- `Db::open_with(path: &Path, migrations: Vec<Migration>) -> Result<Db, PersistError>`（`:72`）、
  `Db::migrate(&self) -> Result<u32, PersistError>`（`:96`）、`Db::begin(&self) -> Result<Tx<'_>, PersistError>`（`:147`）。
  本仓既有的起库形状见 `crates/continuum-artifact/tests/artifact_store.rs:109-110`。
- **P5a 的 `Evidence` 八字段全私有、无 `Default`、唯一构造点**
  `Evidence::from_tool_result(EvidenceId, EvidenceType, EvidenceSubject, Claim, EvidenceProducer, Vec<ArtifactId>, EvidenceStrength, EvidenceScope) -> Result<Evidence, VerifyError>`
  （P5a 设计 §4.4）。**该 crate 尚未落地**，落地时以 `crates/continuum-verify/src/` 的源码为准，不符时回报。
- **P5e 的 `ArtifactType::Image`**（P5e 设计 §3.2 第 1 行，`as_str` 的串是 `image`）。
  **该变体尚未落地**，落地时以 `crates/continuum-artifact/src/artifact.rs` 的源码为准，不符时回报。

---

## 已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；
   正向的「变红」跑全量是加分。**每一处「预期谁红」都要标档位。四档是对「守卫的变异」的分档**：
   **取反**（把守卫的判定反过来）／**放宽**（把守卫少判一半条件）／**收紧**（把守卫多判一半条件）／
   **移除**（删掉整条守卫或它的一半动作）。
   **凡变异改的是被守卫断言的声明取值**（`operators.rs` 的某一列、名单里的某一项、语料里的某一个串），
   守卫本身一字未动——那一类按**把被断言的取值改成与期望不是同一枚**读，档位记**取反**，
   并写清它改的是哪一列；**同一形状的变异在全篇用同一个档位**。
   **唯一带宽紧方向的是 `determinism` 那一列**：`Deterministic` **放宽** §305 的复用面、
   `NonDeterministic` **收紧**它（本层同层设计的预告表里只有 P5c 那一份逐字如此；
   P5d 与 P5e 记「取反」、本块设计记「收紧」——**口径不一**，逐条见 `## 遗留` 第一节第 5 条。
   本计划按 §305 的实质统一记「放宽」）。
   **故「把 `generate-image` 的 `determinism` 改成 `Deterministic`」记「放宽」，不记「收紧」**
   ——设计 §11.3 那一行的档位与之相反，**本计划按放宽记**，见 `## 遗留` 第一节第 5 条。
   **三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN。**本计划每条红条件都指明唯一锚点**
   （哪一枚算子的哪一列、哪个函数的哪一行）；凡变异点的字面量在 `all_operators()` 里出现多次的，
   红条件写清是**哪一枚算子的哪一列**（不写行号——用例按 id 取行，行号会随表的重排漂）。
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**。
   **先排除一处易误判的**：**把某枚算子的候选串换成另一个同样合法的串**（如 `builtin` → `core`）
   **不是等价变异体**——Task 2 的 (a) 对 `backend_candidates` **逐枚断言、期望值手写**，故它**会红那一枚**。
   本块设计 §11.3 的预告表把它记作「全绿」，那一行与设计自己的 §11.2 第 (a) 条相抵，
   **本计划按「取反」记**（见 Task 2 的用例注释与 `## 遗留` 第一节第 12 条）。
   **本块已预先识别的等价变异体逐处列出**：
   （i）**把某枚算子的 `backend_candidates` 里的两项换序**（`[builtin, core]` → `[core, builtin]`）
   ——本块的断言把候选读作**集合**（`backend_candidates` 的次序不是判据，见 Task 2 的用例注释），故**全绿**；
   （ii）**把 `all_operators()` 五行的次序换一下**——本计划要求用例**按 id 取行**、不按下标取，
   故换序是**全绿的等价变异体**（唯一能打红它的是「某条用例按下标取」，而那种写法本计划不许）；
   （iii）**`Assertion` 里手抄的期望清单与被测清单同源**——Task 2 的 (a) 与 Task 3 的两组语料
   **必须手写**；若 (a) 写成遍历 `all_operators()` 去比 `all_operators()`，它测的是恒等式，
   若 (b) 写成 `all_operators().len()`，它测的是 Rust 的 `len()`（设计 §11.2 的两条「预防」）。
   **这就是「正控制不得自证」**：语料若从被测清单里取，就是一张恒真的假照片——
   **恒绿的守卫比没有更坏**，它会被读成「这一条已经钉住了」。本块的落点：Task 2 的 (a) 的手写五列期望、
   Task 3 的（A）与（B）两份手写语料、Task 5 的两个 detector 桩（**桩不许取自本块的任何名单或常量**，
   否则那些用例都在测「桩与它自己一致」）。**三件要一起断**：漏掉的那个方向、多出来的那个取值、
   以及它在表里的位置——Task 3 的（A1）断漏、（A2）断多、（B）断位置。
   **`all_operators()` 的返回类型里的数目 `5` 不在此列**：删一枚算子必须同步把返回类型写成
   `[Operator; 4]`，而那一处改动**会让 Task 2 的 (a) 的对应行、手写的 id 清单断言与 (q) 的补集断言红**
   （设计 §11.3 的「放宽」那一行预告的是 `b` 与 `q`；本计划把 `b` 那一枚换成了手写 id 清单，
   理由见 Task 9 Step 5 与 `## 遗留` 第一节第 10 条）。
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
   cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
   **本计划的编译期照片逐处列出，照实写成编译期照片，不冒充运行期红**：
   Task 1 的 `EditRegionKind::as_str` 穷尽 `match`、Task 1 与 Task 6 的字段清单穷尽解构、
   Task 2 的 `all_operators()` 返回类型数目、Task 8 的三份 trybuild 样例（对应设计 §11.4 的两条照片，
   其中一条被拆成两份样例，理由见 Task 8）。
   **注意 `match` 的入参是不是枚举自身**决定了它成不成立：**收 `&str` 的解析函数一定有通配臂**
   （如 `ArtifactType::parse`，`crates/continuum-artifact/src/artifact.rs:68`），故它**不是**那张照片；
   `EditRegionKind::as_str` 是。
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
   **有一条变异的靶子不在本块的交付物里**（Task 7 的 (m) 的变异要改 `continuum-artifact` 的
   `save_artifact`）：那一轮**只能在本地工作树上做、必须还原，不许把那个改动提交**，报告里要写明这一点。
3. **门读数的判据是日志里 `Running` 与 `test result:` 的行数，不是「命令退出码为 0」。**
   `cargo test` **不收 `--keep-going`**，用 `--no-fail-fast`；**Doc-tests 算一条**。
   **不带 `--no-fail-fast` 会在第一个失败目标处停住，后面的目标一行不跑**，
   而日志里仍会出现若干 `Running` 行——故「射程是否覆盖全部测试目标」这件事只能靠**数 `Running` 的行数**，
   不能靠 `Compiling` 行或退出码。
4. **凡注释写绝对措辞，必须有对应用例；且要带时点**——「唯一／一律／只有／全部／没有任何」这类词，
   要么有可写出的照片，要么**明写为什么没有**。**断言的作用域要与事实同宽**：
   「整张表的唯一 X」≠「某个字段的唯一 X」；「本 crate 的源码文本里零命中」≠「全仓零命中」；
   「在本 task 结束时」≠「永远」。
   **枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条臂各要一条照片，不抽代表
   （手写分支能各自漂移）。**枚举式判据的通病是它钉的是「枚举到的那些」，而不是「那个全称」**：
   某一半只能靠枚举承担时，**照实写明那一半的射程，不许写成全称**。
   本块的两处落点：`EditRegion` 的**四支**（设计 §11.2 的 (d)–(h) 只覆盖三支，第四支由 Task 5 补一条，
   见 `## 遗留` 第一节第 2 条）、**§3.4 的表→下游**那一个方向（设计 §11.2 的 (s) 写的是「下游 → 表」
   那一个方向，表→下游这一侧由 Task 3 的（A1）补，见第一节第 6 条）。
   **本块的绝对措辞逐处落点**：`operators.rs` 里「五枚都不含 `NonIdempotent`」→ Task 2 的 (a) 的
   `side_effect_class` 列（逐枚）；「第 4、5 行是 `Deterministic`，其余三枚不是」→ Task 2 的 (a)；
   「mask 外像素直接从原图复制」→ Task 6 的 (j)；「`EditRegion` 的四个支共用同一个函数、
   同一个返回类型」→ Task 5 的四支逐支用例 ＋ Task 9 Step 3 的源码面复核；
   「本 crate 里没有第二条从 `Point` 到 `EditMask` 的路」→ Task 9 Step 3
   （`resolve_edit_region` 是本 crate 唯一返回 `EditMask` 的函数）；「`ImageError` 恰四臂」→ Task 1 的穷尽解构；
   「本域的端口只用两枚型」→ Task 2 的 `the_port_types_in_use_…`；
   「本块不注册第二项检查的那枚算子」→ Task 2 的 `no_sixth_operator_is_registered` 里**手写的五枚 id 清单**。
   **反面的一半要单独写**：`EditRegionKind` 的四臂与 §166 的四个名字**不是一一对应**（§4.1 明写
   本设计取 §325 的名字），故「四臂」这句话的射程是**本块的编码**，不是「规范只有四个区域种类」。
5. **照片的坐标有三个维度：位置、可见性、坐标。** 三者缺一条，那张照片就写不出来或会静默变假：
   - **位置**：哪个文件的哪一行、哪一枚算子的哪一列、哪一条用例。**本计划逐处写死锚点**，
     且凡同一字面量在多处出现的，红条件指到**哪一枚算子的哪一列**（不写行号——用例按 id 取行，
     行号会随表的重排漂）。
   - **可见性**：`pub` / `pub(crate)` / 私有**决定那张照片能不能写**。**本块的实例**：
     `OperatorRegistry` 的 `entries` 是**私有字段、无迭代器、无 `len()`**
     （`crates/continuum-operator/src/registry.rs:15-17`）——故「注册表内容与调用前逐枚相同」
     **只能经 `resolve` 观测**（Task 4 的 (c2)）；`ImageError` 的四臂与 `EditMask` / `Raster` /
     `PixelDiff` 的字段**全 `pub`**，故穷尽解构那种照片在本 crate 内外都写得出来。
     反面的实例：`Evidence` 的八个字段**全私有**，故本块只能拍「造不出来」（Task 8 的样例 3）。
   - **坐标**：名字与行号会不会漂。**本块的两处**：Task 2 的用例**按 id 取行**（不按下标，
     以免表的重排让红条件指到别的行）；Task 8 的 `.stderr` **按文件主干配对**，
     且**样例里的注释也是照片坐标**（注释写错，`.stderr` 引的行号与列位置随之漂）。
6. **「自指计数不写数值」**：「下面这 N 条」这类话会被自己推翻。本计划凡引用设计或切分文档的编号，
   照原文的编号写（那是可实测的）；凡指本计划自己的清单，一律用「逐条」而不写数目。
7. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」。本块的失败面是 `ImageError` 的
   **四臂**（`Registry` / `RegionUnresolved` / `ShapeMismatch` / `PixelLengthMismatch`）；
   **逐臂至少一条用例**（Task 4 的 (c)、Task 5 的 `g` 与 `a_semantic_object_…`、
   Task 6 的 (k) 与 (k2)），**并逐臂断准它的字段值**（`Duplicate` 的 `id`/`version`、
   `RegionUnresolved` 的 `region`、`ShapeMismatch` 的 `left`/`right`、`PixelLengthMismatch` 的 `len`/`expected`）；
   `(c2)` 与 `(n)` 要同时断言**没有半写的副作用**。
8. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本计划逐处标出「另一侧」是哪一个用例：
   §3.5 的复用（Task 7 的 (o) 有读数那一侧 ＋ (p) 空读数那一侧）、
   注册（Task 4 的 (c) 拒的那侧 ＋ (d) 放行的那侧）、
   §325 的 `Point`（Task 5 的 `g` 拒的那侧 ＋ `f`／`h` 放行的那侧）、
   §3.4 的边表（Task 3 的 (A) 下游侧两个方向 ＋ (B) 上游侧）、
   §326（Task 6 的 (i) 正例 ＋ (j) 强制作那一侧；**并且**其 fail-open 的一侧在 §5.4 那一层
   **没有照片**，据实写明）。
   **把两侧塞进同一条枚举式语料不算两侧都钉**：语料只覆盖它写到的那些取值，语料外的取值两侧都是 fail-open
   （Task 3 与 Task 5 各有一条就是为此补的）。
9. **编译期性质照实写成编译期照片，不冒充运行期红。** 本块的编译期照片逐处标出：
   `EditRegionKind::as_str` 的穷尽 `match` 与 `EditRegionKind::ALL` 的数目、
   四个类型（`EditMask` / `Raster` / `Operator` 的端口、`PixelDiff`）的字段清单穷尽解构、
   `all_operators()` 的返回类型数目、Task 8 的三份 trybuild 样例（对应设计 §11.4 的两条照片）。
10. **各 task 的「运行，确认失败」一步要写清量到的是哪一种失败。** 只有**该用例引用的符号已经存在**时，
   那一步才可能量到运行期的红；否则量到的是 `error[E0432]`／`error[E0433]` 一类的**编译失败**，
   按本纪律第 1(c) 条**不算红**，那一步是空转的——凡属此类的步骤，本计划**照实标注为编译期照片**，
   并写明「要取到运行期的红需要先有什么」。
11. **本块是新 crate**：**Task 1 是第一个 task，`Files` 里写的每一个实现体今天都不存在**
    （没有一条「某某的既有实现体」）。故 Task 1 的「确认失败」步量到的必然是编译失败
    （`error[E0433] failed to resolve`：`tests/region.rs` 要 `use continuum_image::…`，而那个 crate 由 Step 4 才建出）。
    **本计划照实标注它为空转，不把它写成「先跑红再实现」**。

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
  **跨 crate 才可见**的效果（`ALLOWED` 与实际依赖一致、新 crate 进 workspace 后迁移集合与计数不受影响、
  §3.5 的 `cache_key` 读的是 P1 的 `reuse.rs`）。
- **其余分支 → 受影响 crate 的包级套件**，报告里须**标明证据强度较低**并列出
  「这一条可能漏掉的跨 crate 观察点」。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；凡与既有 crate 交互的签名
（`Operator` 的字段名、`OperatorRegistry::{register, resolve}`、`cache_key` / `can_reuse`、
`ContentHash::of`、`Db::open_with` / `migrate` / `begin`、`save_artifact` / `load_artifact`），
实现前须先读该 crate 的源码确认，不符时以源码为准并回报。

**本计划特有的三处「照抄前须对源」**：

- **`all_operators()` 是函数不是常量**，返回 `[Operator; 5]`；**数目写在返回类型里**，
  故「清单个数」这件事在编译期可见。**不要把 `const` 与 `[T; N]` 混着搬**——
  本仓带 `ALL` 常量的两处（`ArtifactType::ALL`、`AuditKind::ALL`）都是**无字段枚举**才成立；
  而 `V01_OPERATOR_IDS`（元素型 `&str`）与 `EditRegionKind::ALL`（无字段枚举）的 `const` **是**成立的。
- **`RegionDetector` 的方法集是本计划自定的取值**（设计 §2.2 只说「本块定义它，其实现在 backend 一侧」，
  §4.2 只说「`detector` 的返回值就是掩码位图（`EditMask`）」，**没有给方法名、参数与返回类型**）。
  本计划取两个方法、返回 `Option<EditMask>`、收 `source` 与 `source_dims`——**代价与依据见 `## 遗留` 第一节第 3 条**。
- **`Evidence` 与 `ArtifactType::Image` 两处的 crate / 变体尚未落地**（跨计划前置第一节实测）。
  凡 Task 2 与 Task 8 的代码块写到的名字，都是**照 P5a 设计 §4.4 与 P5e 设计 §3.2 转抄的**，
  落地时以 crate 源码为准。

**`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；而且逐份各不相同，不许拿一个值套两处。**
本仓已为此付过代价（P3D 的计划给三份样例共用一个错误码 `E0603`，实测 rustc 1.95 下三份各不相同）。
**本计划给出的任何错误码都只是预期值**，落地时以 `trybuild` 实际产出的 `.stderr` 为准。
**trybuild 的两条坐标**：**按文件主干配对**（`X.rs` ↔ `X.stderr`，且 `tests/type_level.rs` 的通配
`tests/compile_fail/*.rs` 收走全部 `.rs`）；**样例里的注释也是照片坐标**——注释写错了，
`.stderr` 里引的行号与列位置随之漂，两份文件一起重取。
**本仓没有 `tests/ui_pass/` 目录、也没有一处 `t.pass`**（实测：`find crates -type d -name ui_pass` 零命中；
`grep -rn "t\.pass\|\.pass(" crates/*/tests/*.rs` 零命中）。
**故正控制由普通运行期用例承担**（它能编译即已说明正常调用路径成立），**本计划不新建 `ui_pass/` 目录**。

---

# 文件结构

```
crates/continuum-image/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/error.rs        ImageError（四臂）
  src/region.rs       EditRegionKind、EditRegion、EditMask、NormalizedAnchor、NormalizedBox、SemanticObjectRef
  src/operators.rs    all_operators() 与 V01_OPERATOR_IDS（**Task 2，硬前置：P5e 的 ArtifactType::Image**）
  src/register.rs     register_image_operators（先查重后写）
  src/resolve.rs      RegionDetector 与 resolve_edit_region（§325 的唯一落点）
  src/pixels.rs       Raster、PixelDiff、composite_masked、outside_mask_difference、三个前置条件
  tests/region.rs                四臂的编码与四个载荷、ImageError 的四臂（§11.1）
  tests/operators.rs             五枚声明的逐列断言与批次边界（§11.2 的 a、q、r）
  tests/schema_edges.rs          §3.4 的边表（§11.2 的 s；A/B 两组断言）
  tests/register.rs              注册的正例与两条例外臂（§11.2 的 b、c、c2、d）
  tests/resolve_region.rs        §4.2 四支的照片与 §4.3 的边界（§11.2 的 d–h；dev 边无）
  tests/pixels.rs                §5.2／§5.3 的正反例与三条前置条件（§11.2 的 i、j、k、k2、l）
  tests/reuse.rs                 §3.5 的复用照片（§11.2 的 o、p；dev 边 continuum-graph）
  tests/lineage.rs               §6 的谱系照片（§11.2 的 m、n；dev 边 continuum-persist）
  tests/type_level.rs            trybuild 驱动（**Task 8，硬前置：P5a 的 continuum-verify**）
  tests/compile_fail/*.rs        三份编译失败样例（各配同名 .stderr）
```

**本计划要改的既有文件**（只有两处，都在切分 §一 的共写文件表里）

```
Cargo.toml（workspace）                                  members 加 crates/continuum-image（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 加 continuum-image 条目（Task 1 起，逐 task 增补内层数组）
```

**本计划不碰的文件**：`crates/continuum-runtime/src/**`（本块没有装配面）、
`crates/continuum-operator/**`（只 `use` 它的类型与注册表，一个字段都不加）、
`crates/continuum-artifact/**`（只 `use` `ArtifactType` / `ArtifactId` / `ContentHash` / `save_artifact` /
`load_artifact` / `p1_artifact_migrations`，**一个变体都不加**）、
`crates/continuum-graph/**`、`crates/continuum-persist/**`、`crates/continuum-port/**`、
`crates/continuum-core/**`、`crates/continuum-events/**`、`crates/continuum-policy/**`、
`crates/continuum-capability/**`、`crates/continuum-secrets/**`、`crates/continuum-sandbox/**`、
`crates/continuum-workspace/**`、`crates/continuum-provider/**`、`crates/continuum-connector/**`、
`crates/continuum-model-registry/**`、`crates/continuum-node/**`、
`docs/02-工程.md`、`docs/01-总纲.md`、`docs/spec/**`、`docs/superpowers/specs/**`（六份设计一份都不改）。

---

## 各 task 的「运行，确认失败」一步量到的是哪一种失败（逐 task 扫过）

**这一步的名字只在「本 task 有新生产代码」时才成立。** 逐 task 实测的形态如下——
凡量到**编译失败**的，按纪律 1(c) **不算红**，那一步是空转的，本计划在那里**照实标注为编译期照片**；
凡本 task **不交付生产代码**（只新建测试）的，测试对的是前面 task 已交付的声明，**首次即绿**，
那一类的步名不叫「确认失败」：

| task 的那一步 | 量到的是 | 为什么 |
|---|---|---|
| Task 1 Step 3 | **编译失败**（`E0433`，`tests/region.rs` 的 `use continuum_image::…`） | crate 由本 task 的 Step 4 才建出；**本 task 是新 crate 的第一个 task，这个「确认失败」步必然空转**（纪律 11） |
| Task 2 Step 2 | **编译失败**（`E0432` / `E0433`） | `all_operators` / `V01_OPERATOR_IDS` 由 Step 3 才建出 |
| Task 3 Step 2 | **首次即绿** | 本 task 只新建测试，断言的是 Task 2 已交付的声明；**它的红由 Task 9 Step 5 的变异复核取** |
| Task 4 Step 2 | **编译失败**（`E0432`） | `register_image_operators` 由 Step 3 才建出 |
| Task 5 Step 2 | **编译失败**（`E0432`） | `RegionDetector` / `resolve_edit_region` 由 Step 3 才建出 |
| Task 6 Step 2 | **编译失败**（`E0432`） | `composite_masked` / `outside_mask_difference` 由 Step 3 才建出 |
| Task 7 Step 3 | **首次即绿** | 本 task 只新建测试（`cache_key` / `can_reuse` / `save_artifact` 都是 P1 已交付的） |
| Task 8 Step 2 | **运行期红**（trybuild 报样例与 `.stderr` 不符） | 样例与 `.stderr` 都在本 task 内，`.stderr` 尚未取值 |
| Task 9 | 无「确认失败」步 | 它是收尾与复核 |

**「首次即绿」不是缺陷，也不许改写成「先红后绿」**：那两个 task 的交付物就是断言本身，
它们的红由 Task 9 Step 5 的变异复核取。

---

### Task 1: `continuum-image` 骨架、`ImageError` 与编辑区域的四种表示

**Files:**
- Create: `crates/continuum-image/Cargo.toml`
- Create: `crates/continuum-image/src/lib.rs`
- Create: `crates/continuum-image/src/error.rs`
- Create: `crates/continuum-image/src/region.rs`
- Create: `crates/continuum-image/tests/region.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加一行 `crates/continuum-image`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-image` 条目）

**Interfaces:**
- Consumes: `continuum_operator::OperatorError`（`ImageError::Registry` 的来源臂）
- Produces: `continuum_image::{EditRegionKind, EditRegion, EditMask, NormalizedAnchor, NormalizedBox, SemanticObjectRef, ImageError}`

**四枚辅助类型的落点是本 task**：`NormalizedAnchor` / `NormalizedBox` / `SemanticObjectRef` 与
`EditRegionKind` 的编码（`as_str` / `parse` / `ALL`）。
**`ImageError` 的四臂照设计 §3.1 逐臂照录**，它不是本 task 的自由决定——
臂数是**四**（`Registry` / `RegionUnresolved` / `ShapeMismatch` / `PixelLengthMismatch`），
每一臂的判据持有者与理由在设计 §3.1 逐条给出，本计划不重抄（重抄就是第二份转录）。

- [ ] **Step 1: 建 crate 骨架并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`continuum-operator`（`OperatorError`）、`thiserror`
（`ImageError` 上派生 `Error` 与 `Display`，去掉即 `E0433`）。
`continuum-artifact` / `continuum-graph` / `continuum-persist` / `continuum-verify` / `tempfile` / `trybuild`
**由需要它们的 task 增量加**。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加（**只列本 task 实际有的边**）：

```rust
    // P5f 图像领域算子（设计 §2.1）。本块只**声明**算子与**判定**（区域解析、两个确定性函数），
    // 不实现执行体，故生产代码只用 operator 的算子类型与 artifact 的端口类型。
    // 其余四条边由需要它们的 task 增量加：artifact（Task 2 起，普通边），
    // graph 与 persist（Task 7，dev 边），verify（Task 8，dev 边）。
    ("continuum-image", &["continuum-operator"]),
```

**条目不能省**——`every_crate_depends_only_on_its_allowed_set` 要求**每个** workspace 成员都在表里，
漏了即红；而 `workspace_crates()` 从 workspace 定义派生（`:234`），故**加 member 之后、补表之前**
该门是红的。**这是刻意的**（切分 §一 的共写文件表逐字写着这一点），不要靠「先不加 member」绕过。

- [ ] **Step 2: 写用例**

`tests/region.rs`：

- `every_edit_region_kind_round_trips_through_its_encoding`（§11.1）：**四臂各一条往返**
  （`Point` / `Box` / `Mask` / `SemanticObject` 经 `as_str` / `parse` 往返），
  加 `EditRegionKind::ALL` 的**逐臂成员断言**（四条，`ALL` 里恰含这四枚）与四串的**手写期望**。
  红条件：`as_str` 少一档（**放宽档**）→ 该臂的往返与 `ALL` 那条红。
  **编译期照片写清**：`as_str` 的 `match` 的**入参是枚举自身**、无通配臂 ⇒ 加第五臂即**编译不过**
  （按纪律 1(c) **不算运行期红**）；而 `parse` 收 `&str`、**必有通配臂**，故它**不是**那张照片。
  **并写明「四臂」这句话的射程**：它是**本块的编码**恰四枚，**不是**「规范只有四个区域种类」——
  §166 `:745-752` 列的是同一组的另两个名字（`Point` / `Bounding Box` / `Brush Mask` / `Semantic Object`），
  本设计取 §325 的四个名字（设计 §4.1）。
- `an_alias_from_166_is_not_in_the_encoding`（**本计划自定**，见 `## 遗留` 第五节）：
  `EditRegionKind::parse("brush_mask")` 与 `parse("bounding_box")` 各一条 ⇒ `None`。
  **理由（写进注释）**：`as_str` / `parse` 是本编码**唯一的一对产生点与逆**（照
  `crates/continuum-artifact/src/artifact.rs:9-11` 的注记口径），收进别名会让「一型两名」可表达；
  而 §4.1 记录的两组名字之间的关系是**本设计的对应表**，不是编码的一部分。
  红条件：把 `"brush_mask"` 也收进 `parse`（**移除档**）→ 本条红。
- `the_four_error_arms_carry_exactly_their_declared_fields`（**编译期照片**）：
  对 `ImageError` 的四臂**逐臂穷尽解构、不带 `..`**，解的是各自载荷的形状：
  `Registry(OperatorError)`、`RegionUnresolved { region: EditRegionKind }`、
  `ShapeMismatch { left: (u32, u32, Option<u8>), right: (u32, u32, Option<u8>) }`、
  `PixelLengthMismatch { len: usize, expected: usize }`。任一支加字段、少字段、改名即**编译不过**。
  **明写**：编译期照片，运行期无照片——**不要把「四臂」这一句话写成一条恒真的运行期断言**
  （Rust 没有反射，把四个变体名手抄一遍再比对是手抄自己）。
  **并写明 `ShapeMismatch` 第三分量为 `Option` 的理由**（设计 §3.1）：掩码是单通道位图（本 task 的
  `EditMask` 没有 `channels` 字段），故掩码那一侧是 `None`；`None` 的分量在比较时不参与。
- `region_unresolved_names_the_kind_that_failed`（**运行期**）：
  把 `ImageError::RegionUnresolved { region: EditRegionKind::Point }` 与
  `{ region: EditRegionKind::SemanticObject }` **各一条**转成字符串（`to_string()`），
  断言串里分别含 `Point` 与 `SemanticObject`。
  **来历**：`EditRegionKind` 要 `derive(Debug)` 正是因为它（设计 §3.1；本块不必为它写 `Display`，
  各处都用 `{:?}`）。红条件：把该臂的 `#[error]` 串改成一个不含 `region` 的固定串（**移除档**）
  → 两条都红。**注意**：这条照片钉的是「消息里带得出是哪一型」，**不钉**「这一型是不是判对了」
  ——后者由 Task 5 的两条 `g`／`a_semantic_object_…` 承担。
- `the_mask_carries_the_bitmap_and_no_channel_dimension`（**编译期照片**）：
  `EditMask` 的字段清单**逐项穷尽解构、不带 `..`**：`width: u32`、`height: u32`、`pixels: Vec<u8>`；
  **没有 `channels`**（与 Task 6 的 `Raster` 的差别正在此：它是单通道位图，不是图像）、
  **没有 `artifact` / `ArtifactId` 一类字段**（设计 §4.1：`Box` 支的掩码由投影得出，
  那一刻仓里还没有它的制品 id；把它变成 `ArtifactType::Blob` 制品要经 `BlobStore::put`，而本块不做 IO）。
  红条件：给 `EditMask` 加一个 `channels: u8` 字段（**移除档** → 穷尽解构先编译不过）。
  **并写明**：`EditMask` 的不变量 `pixels.len() == width as usize * height as usize` 的**判据在
  Task 6 的两个函数入口**，本 task 只有声明与文档注释。
- `the_four_region_arms_carry_exactly_the_four_declared_payloads`（**编译期照片**）：
  `EditRegion` 四支**逐支穷尽解构、不带 `..`**：
  `Point(NormalizedAnchor)`、`Box(NormalizedBox)`、`Mask(EditMask)`、`SemanticObject(SemanticObjectRef)`。
  红条件：把 `Box` 支的载荷改成 `(NormalizedAnchor, NormalizedAnchor)`（**取反档**）→ 编译不过。
- `every_region_arm_reports_its_own_kind`（**运行期，四支逐支不抽代表**）：
  `EditRegion::Point(..).kind() == EditRegionKind::Point`、`Box(..)` → `Box`、`Mask(..)` → `Mask`、
  `SemanticObject(..)` → `SemanticObject` **各一条**。
  红条件：把 `kind()` 的 `SemanticObject` 那一支写成 `EditRegionKind::Point`（**取反档**）→ 第四条红。
  **`kind()` 是本计划自定的一处读法**（设计只要求 `RegionUnresolved { region }` 表达得出是哪一型，
  没给读法的名字）——代价与理由见 `## 遗留` 第五节。
- `the_two_carriers_declare_exactly_their_fields`（**编译期照片 + 一条运行期**）：
  `NormalizedAnchor` 的两个字段逐项（穷尽解构）；`NormalizedBox` 的两个角逐项。
  运行期一条：两角**次序对调**的两个 `NormalizedBox`（`p`/`q` 互换）在 Task 5 的投影下**给出同一个掩码**
  ——这条用例在 Task 5 建（它要 `resolve_edit_region`），本 task 只钉字段清单与「两角的名字不表达几何次序」
  这一句写进类型的文档注释。**理由（写进注释）**：§166 只给「Bounding Box」一个名字，
  **哪一角是左上规范未给**，故本型不表达次序，投影时逐维取 min/max。
  **字段名 `p` / `q` 是本计划自定**（设计只说「它的两个角」，没给名字），见 `## 遗留` 第五节。
- `a_semantic_object_ref_is_an_opaque_string`（**运行期**）：
  `SemanticObjectRef` 经 `as_str` / `from_str` 往返，**含一个表外取值**（如 `"左边这个人"`——
  §166 `:769` 给的正是一句自然语言）。
  **并写明这一条的限度与设计 §4.1 的关系**：`SemanticObjectRef` 的形状是**不透明串**，
  它**不填** §4.3 表第 5 行那条缺口（「名字从哪来：自由文本？受控词表？」）——
  一枚不透明串既不规定名字的来源，也不封闭取值域，故把 `SemanticObjectRef` 定成不透明串
  **不是「给规范没给的东西定形状」**；设计 §4.1 的「不给它定形状」讲的是**不替它定名字空间**。
  **这一句要写进用例注释**（否则读者会以为本用例与设计相抵）。
  红条件：把 `SemanticObjectRef` 换成封闭枚举（**移除档**）→ 本条**编译不过**
  （按纪律 1(c) 不算红），**故本条的守卫是运行期往返，不是「加一臂会红」**。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test region
```

**这一步必然空转，本计划照实说明**：本 task 是**新 crate 的第一个 task**，
`Files` 里没有一个「既有的实现体」可依赖；`tests/region.rs` 要 `use continuum_image::…`，
而那个 crate 由 Step 4 才建出，故本步的失败形态是 `error[E0433] failed to resolve` 一类，
不是断言失败，按纪律 1(c) **不算红**。**本计划不把它写成「先跑红再实现」**
（纪律 11）。**要取到运行期的红**，须在 Step 1 就把七个名字的**签名**给出（函数体先写作 `todo!()`），
届时本步的红是用例 panic（运行期，纪律 1(c) 认它）。**本计划的判据按前者记**（不强制走这条）；
**若实现者走后者，那些 `todo!()` 不许活到 Step 5 的提交里。**

- [ ] **Step 4: 实现**

```rust
// src/error.rs
/// 四臂。逐臂的判据持有者与理由见设计 §3.1，本计划不重抄。
/// **不为「backend 不支持掩码编辑」留臂**：那是一个声明面的事（`backend_candidates` 的取值），
/// 不是一次调用的失败。**不给 IO 臂**：本块的两个确定性函数收 `&Raster`，读写落盘是调用方的事。
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1（`OperatorError::Duplicate`）。
    /// 本块**只带出来、不重述**——另立一枚 `ImageError::Duplicate` 就是
    /// 「注册表里有没有这个算子」的第二套词汇表。
    #[error(transparent)]
    Registry(#[from] OperatorError),

    /// §325：该区域解析不出语义区域（§4.3）。判定的持有者在本块（`resolve_edit_region`）。
    #[error("区域 {region:?} 解析不出语义编辑区域（§325）")]
    RegionUnresolved { region: EditRegionKind },

    /// §326：参与逐点运算的像素载体形状不一致——逐点合成与比较无定义。
    /// 掩码是单通道位图，**没有色彩通道这一维**，故掩码那一侧的第三分量是 `None`。
    #[error("像素载体形状不一致：{left:?} 与 {right:?}（§326）")]
    ShapeMismatch { left: (u32, u32, Option<u8>), right: (u32, u32, Option<u8>) },

    /// 一个载体的 `pixels` 字节数与它自己声明的形状不符——该值不满足自己的不变量。
    /// **与 `ShapeMismatch` 分得开**：那一个比的是**两个载体之间**的形状，
    /// 这一个比的是**一个载体内部**的字节数与它自己声明的形状。
    #[error("`pixels` 长度 {len} 与声明的形状不符（期望 {expected}）")]
    PixelLengthMismatch { len: usize, expected: usize },
}
```

```rust
// src/region.rs
/// §325 `:2351-2358` 的四种表示（§166 `:745-752` 是同一组的另两个名字，对应关系见 §4.1）。
/// 封闭四值。
pub enum EditRegionKind { Point, Box, Mask, SemanticObject }

impl EditRegionKind {
    /// 全部四枚。**这里的 `const` 是成立的**——本型是无字段枚举，
    /// 与 `crates/continuum-artifact/src/artifact.rs:28` 的 `ArtifactType::ALL` 同形。
    /// **这一条与 `all_operators()` 那一处的差别就在此**：`Operator` 有字段，故它不能是 `const`（§3.1）。
    pub const ALL: [EditRegionKind; 4];

    /// 本编码**唯一的产生点**（照 `ArtifactType::as_str` 的既有做法）：
    /// 小写、多词以 `_` 连接。match 穷尽且无通配臂：加一臂时本函数编译不过。
    pub fn as_str(&self) -> &'static str;

    /// [`EditRegionKind::as_str`] 的**严格逆**：对 `ALL` 里每枚都有 `parse(t.as_str()) == Some(t)`，
    /// 表外字符串一律 `None`。**它收 `&str`、必有通配臂**，故它不是「加一臂即编译不过」那张照片。
    pub fn parse(s: &str) -> Option<Self>;
}

/// §163 `:626-652` 的坐标归一化（`x, y ∈ [0, 1]`，实测该两句在 `:648-649`），避免与分辨率绑定。
/// 字段名 `x` / `y` 有规范出处；**类型名是本设计定的**（规范原文那个类型叫 `ImageEditAnchor`，
/// `docs/spec/03-product-drive.md:639`），见 `## 遗留` 第一节第 8 条。
pub struct NormalizedAnchor { pub x: f64, pub y: f64 }

/// §166 `:749` 的 Bounding Box：它的**两个角**（设计 §4.1 定的；规范只给名字）。
/// **两角的名字不表达几何次序**：§166 没给「哪一角是左上」，故投影时逐维取 min/max，
/// 两角对调给出同一个掩码（Task 5 有一条用例钉它）。
pub struct NormalizedBox { pub p: NormalizedAnchor, pub q: NormalizedAnchor }

/// §166 `:751` / `:769` 的 Semantic Object：以名字指一个对象。
/// **不透明串**——它不规定名字的来源、也不封闭取值域。
/// **这不填 §4.3 表第 5 行的缺口**（设计 §4.1 的「不给它定形状」讲的是不替它定名字空间）。
pub struct SemanticObjectRef(String);

/// §166 `:775` 的 `EditMask`：四种表示解析后的共同产物。
/// **带的是掩码位图本身，不是指向掩码制品的 id**（Box 支的掩码由 §4.2 投影得出，
/// 那一刻仓里还没有它的制品；本块不做 IO）。
/// **单通道位图**，故本型**没有 `channels` 字段**——`Raster`（Task 6）有，因为它是图像。
/// **不变量**：`pixels.len() == width as usize * height as usize`（Task 6 的两个函数在入口核它）。
pub struct EditMask { pub width: u32, pub height: u32, pub pixels: Vec<u8> }

/// §325 `:2351-2358` 的四种表示。
pub enum EditRegion {
    Point(NormalizedAnchor),
    Box(NormalizedBox),
    /// §166 `:750` 的 Brush Mask：调用方已给出掩码位图，故本支带的是**已经解析好的位图**。
    Mask(EditMask),
    SemanticObject(SemanticObjectRef),
}

impl EditRegion {
    /// 本支的判别式。`RegionUnresolved { region }` 要它才能表达「是哪一型没解析出来」。
    /// **本计划自定的一处读法**（设计没给它的名字）。
    pub fn kind(&self) -> EditRegionKind;
}
```

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t1-test.log
git add crates/continuum-image Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(image): crate 骨架、ImageError 四臂与编辑区域的四种表示"
```

---

### Task 2: 五枚算子的声明与 §343 的批次边界

> **硬前置：P5e 已落地 `ArtifactType::Image`**（跨计划前置第二节）。该变体不存在时本 task 编不过。

**Files:**
- Create: `crates/continuum-image/src/operators.rs`
- Create: `crates/continuum-image/tests/operators.rs`
- Modify: `crates/continuum-image/src/lib.rs`（导出）
- Modify: `crates/continuum-image/Cargo.toml`（dep `continuum-artifact`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-artifact`）

**Interfaces:**
- Consumes: `continuum_artifact::ArtifactType`（`Image` 由 P5e 落地、`Blob` 是 P1 既有）；
  `continuum_operator::{Operator, OperatorId, OperatorVersion, Determinism, SideEffectClass, BackendId}`；
  Task 1 的 `ImageError`
- Produces: `continuum_image::{all_operators, V01_OPERATOR_IDS}`

- [ ] **Step 1: 写用例**

`tests/operators.rs`：

- `every_row_of_the_table_is_pinned_verbatim`（§11.2 第 (a) 条，**逐枚、逐列，不抽代表**）：
  五枚算子的端口两列、`determinism`、`side_effect_class`、`backend_candidates` **逐枚断言**，
  **期望值手写**（照设计 §3.2 表的五行），**用例按 id 取行**（不按下标）。
  红条件逐列给出，**每条都指到唯一锚点**（「哪一枚算子的哪一列」）：
  把 `generate-image` 的 `input_schema` 写成空 `Vec`（**取反档**）→ 那一枚的红；
  把 `detect-edit-region` 的 `output_schema` 改成 `[ArtifactType::Image]`（**取反档**）→ 那一枚的红；
  把 `local-generative-edit` 的 `determinism` 改成 `Deterministic`（**放宽档**——见纪律 1 的宽紧口径）→ 那一枚的红；
  把 `hard-composite` 的 `side_effect_class` 改成 `Pure`（**取反档**）→ 那一枚的红；
  把 `verify-outside-mask` 的候选串 `builtin` 改成 `core`（**取反档**）→ 那一枚的红。
  **两条预防写进注释**（设计 §11.2）：期望清单**必须手写**——若写成遍历 `all_operators()`
  去比 `all_operators()`，它测的是恒等式；也不拿 `backend_candidates` 的去重后的集合作语料。
  **并写明**：`backend_candidates` 的**次序**在本块的全部断言里都不是判据（它是候选集合；
  §3.5 的判定与 §245 的 `is_candidate_backend` 都按集合读），故**「两项换序」是等价变异体**
  （纪律 1(b) 的 (i)）；**「换成另一个同样合法的串」不是**——本用例的期望值手写、逐枚断言，
  它**会红那一枚**（纪律 1(b) 里「先排除一处易误判的」那段与 `## 遗留` 第一节第 12 条）。
- `the_ids_are_pairwise_distinct_and_all_versions_are_one`：
  **一行手写的五枚 id 清单**（`generate-image` / `detect-edit-region` / `local-generative-edit` /
  `hard-composite` / `verify-outside-mask`）与 `all_operators()` 的 id **逐枚**比对（按 id 取行，
  不是按下标），且**两两比较**全不相同（不是只查总数）、每枚 `version == 1`。
  红条件：把 `hard-composite` 与 `verify-outside-mask` 的 id 改成同一个（**取反档**）→ 两两比较那条红。
  **这一条同时是「数目」的运行期照片**：清单少一枚（且返回类型同步改成 `[Operator; 4]`）时，
  手写清单比不齐即红。**本块不写 `len() == 5`**——设计 §11.1 明写「不补」，
  它测的是 Rust 的 `len()`，而数目已写在**返回类型** `[Operator; 5]` 里（编译期可见）。
  **这一句要写进用例注释**，免得实现者顺手补一条恒真的断言。
- `the_v01_batch_is_generate_image_and_the_complement_is_the_other_four`（§11.2 第 (q) 条）：
  两个方向都断：（i）`V01_OPERATOR_IDS` 的每一项都能在 `all_operators()` 的 id 里找到；
  （ii）它的补集**恰为**其余四枚——**逐枚断言**（`detect-edit-region` / `local-generative-edit` /
  `hard-composite` / `verify-outside-mask` 各一条「不在 v0.1 批里」＋ `generate-image` 一条「在」），
  **不是只看长度**（只看长度的话，把两批的成员对调也能过）。
  红条件：把 `V01_OPERATOR_IDS` 换成 `["hard-composite"]`（**取反档**）→（ii）里
   `generate-image` 那条「在 v0.1 批里」与 `hard-composite` 那条「不在」**各一条红**（逐枚，不抽代表）。
  **强度据实写进注释**（设计 §9.1 第 2 条与设计 §14 第 1 条）：
  「局部修改在 v0.1 之外」是**规范明写**（`docs/spec/05-normative.md:2737` 的「可以暂不实现」清单含
  `:2743` 的 `Image Local Edit`）；「完整生成仍在内」是**推断**（`generate-image` 既不在那份暂缓清单里，
  也不在 `:2711` 的必做十六项里）——**若协调者把那一条裁反，本条的期望值随之改，而动作不变**。
- `the_two_deterministic_operators_have_exactly_one_backend_candidate`（§11.2 第 (r) 条，**逐枚两条**）：
  `hard-composite` 与 `verify-outside-mask` 的 `backend_candidates.len() == 1` 各一条。
  红条件：给 `hard-composite` 的候选加一项（**取反档**）→ 那一条红。
  **并写明它的用途**（设计 §3.5）：**这一条红即说明本块需要自己那份 §3.5 的名单检查**——
  候选集合是单元集时，「名单内 / 名单外」在本块没有可触发的形态，本块因此不重写那条与领域无关的规则，
  也不给它建第二份实现。
- `the_port_types_in_use_are_exactly_the_two_types_of_this_domain`：
  遍历五枚的 `input_schema ∪ output_schema`，断言出现的型**恰是**手写的两枚
  `{ArtifactType::Image, ArtifactType::Blob}`（**手写期望，不从遍历结果派生**）。
  **并写明**：这是设计 §3.4 末「本块向 P5e 报出的清单为空」的可观察形态；
  **它钉的是「本块用到的型恰是那两枚」，不钉「那两枚在 `ArtifactType` 里还在」**
  ——后者由 `continuum-artifact` 自己的用例钉（本块一个变体都不加）。
  红条件：给某一枚算子加一个 `ArtifactType::Json` 端口（**取反档**）→ 红。
- `no_sixth_operator_is_registered`（**§5.5 的可观察形态**）：断言 `all_operators()` 的 id 集合
  **恰等于**上面那条手写的五枚清单（同一条用例的第二半），**特别地不含**任何一枚承担
  §168 第一项检查（「修改区域是否符合要求」）的算子。
  **来历写进注释**（设计 §5.5）：那一项是**语义判定**，判据规范未给，它要的是 §262 第 3 级的独立模型验证器；
  **给它编一枚算子等于给它编一个判据**。**这一条与上一条共用同一份手写清单**——
  两处维护同一件事会更早漂。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test operators
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`all_operators` / `V01_OPERATOR_IDS` 由 Step 3 才建出，故本步的失败形态是
`error[E0432] unresolved import`／`error[E0433] failed to resolve` 一类，不是断言失败，**这一步是空转的**。
**要取到运行期的红**，须在 Step 1 就给出 `all_operators` 的**签名**（函数体先写作 `todo!()`），
届时本步的红是用例 panic。**本计划的判据按前者记**；若走后者，那些 `todo!()` 不许活到 Step 4 的提交里。

- [ ] **Step 3: 实现**

```rust
/// 本领域五枚算子的全集。**顺序即设计 §3.2 表的行序**，用例按 id 取行（不按下标）。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**，两条理由各自独立成立（§3.1）：
/// （1）`OperatorId(String)` 的字段私有、`OperatorId::new` 不是 `const fn`；
/// （2）`Operator` 另含三枚 `Vec` 字段（`input_schema` / `output_schema` / `backend_candidates`）。
/// 本仓带 `ALL` 常量的既有做法成立的前提是**那些类型无字段**——援引一条既有约定时
/// 要连它的成立条件一起搬。
/// 数目 5 改由**返回类型**承载，故清单个数的改动**编译期可见**（写少一枚而不改返回类型，直接编不过）。
pub fn all_operators() -> [Operator; 5];

/// §343 的 v0.1 批：本域在 v0.1 内要注册的那些算子的 id（设计 §9.2）。
/// **这里的 `const` 成立**——元素型是 `&str`，与 `all_operators()` 那一处不同。
/// 判据：每一项都能在 `all_operators()` 里找到；且补集恰为其余四枚（**逐枚断言，不是只看长度**）。
pub const V01_OPERATOR_IDS: [&str; 1] = ["generate-image"];
```

五枚算子的逐列取值**照设计 §3.2 的表**，本计划不重抄一遍（重抄就是第二份转录，正是本项目出错最多之处）；
**实现时对着该表的五行逐行填**，并逐行在 `operators.rs` 里留下出处注释（§3.2 表末「出处」列的原话）。
**其中第 1、3 行的 `side_effect_class` 是 `Pure`**（2026-10-10 订正：此前取 `NonIdempotent`）——
出处注释要连那次订正的**理由**一起写：两枚的产物是**一个值**、它们不改变本仓之外的状态；
消耗的算力记在预算维度，不是副作用（设计 §3.2 第 6 条）。
**第 4 行是 `Idempotent`**，理由是本行要写进注释的那一条：**它的执行体自己有落盘动作**，
故它改变本仓之外的状态，而重复执行得到同一状态；**「它产生一个大制品 / 重算一次要落一个新 Artifact
与新事件」那句已废**——「消耗资源」与「改变状态」是两件事（设计 §3.2 第 7 条）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t2-test.log
git add crates/continuum-image Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(image): 五枚图像域算子的声明与 §343 的批次边界"
```

---

### Task 3: §3.4 的边表——§239 的注册期一半

**Files:**
- Create: `crates/continuum-image/tests/schema_edges.rs`

**Interfaces:**
- Consumes: Task 2 的 `all_operators`；`continuum_artifact::ArtifactType`
- Produces: 无生产类型（**语料是测试内的手写表**；本块不建端口兼容判定函数——§239 的判定已由
  `crates/continuum-port/src/port.rs:94` 的 `compatible` 落地）

- [ ] **Step 1: 写用例**

`every_declared_edge_has_the_type_on_both_sides`（§11.2 第 (s) 条）：

**形态：两组断言，各钉 §3.4 判据的一侧**——**这里不是「一份语料同时钉两侧」**：
语料只覆盖它写到的那些型，**语料之外的取值两侧都是 fail-open**（给某枚下游算子加一个语料里没列过的型，
两侧都照绿）。故把两半各自交给一条**独立**的断言：

**（A）下游覆盖**——手写一份「有上游算子的下游 → 该下游在 §3.4 表里列出的型集合」的表
（`generate-image` → `{Image}`；`detect-edit-region` → `{Image}`；
`local-generative-edit` → `{Image, Blob}`；`hard-composite` → `{Image, Blob}`；
`verify-outside-mask` → `{Image, Blob}`），**逐行断言两件事**：
（A1）表里列出的**每个**型都在该下游的 `input_schema` 里；
（A2）该下游的 `input_schema` 里**每个**型都在表里列出——**两个方向都判，合起来是集合相等**。
**（A1）是本计划新增的一半**（设计 §11.2 第 (s) 条写的「下游 `input_schema` 的每一型都能在该行列出的
上游算子里找到」**正是**本计划的（A2）那一个方向；**表中列了而下游没有的型无人过问**，
那一侧才是新补的（A1）），理由与依据见 `## 遗留` 第一节第 6 条与第五节。
**（A2）承担的就是「下游的每一个型都能找到上游」这一半**：下游若多出一个语料外的型，（A2）红，
而在（B）里那一型根本没有对应的行——两处一起把「下游有孤儿型」逼出来。

**（B）上游来源**——手写三列语料 `(下游算子 id, 上游算子 id, 型)`，逐行对应设计 §3.4 的边表，
逐行断言该上游的 `output_schema` 含该型：
`detect-edit-region ← generate-image : Image`；`local-generative-edit ← detect-edit-region : Blob`；
`hard-composite ← local-generative-edit : Image`；`hard-composite ← detect-edit-region : Blob`；
`verify-outside-mask ← hard-composite : Image`；`verify-outside-mask ← detect-edit-region : Blob`。
**第 1 行（`generate-image`）的来源在 §3.4 表里写的是「上游**任一枚**出的 `Image`」**——
语料为它写一条 `generate-image ← generate-image : Image`（本域至少有一枚算子输出 `Image`，
而 `generate-image` 自身即满足），**并写明这一条与其余各行的强度不同**：
其余各行指认了一枚**具体**的上游，这一行只指认「**存在**一枚」。

**另有三行的来源是同形的「上游源图（`Image`）」**：设计 §3.4 表给 `local-generative-edit`、
`hard-composite`、`verify-outside-mask` 各列了这一来源（与 `generate-image` 那一行的「上游任一枚出的
`Image`」是**同一个命题**：存在一枚上游其 `output_schema` 含 `Image`）。
**语料不为它们各写一行，理由在此写明**：那三行加上 `generate-image` 那一行是同一个断言的四次出现，
逐行抄写会把「任一枚」读成「某四枚具体的上游」；**它们的承重与 `generate-image` 那一行相同**
（都由 `generate-image ← generate-image : Image` 这一条钉住）。
**射程据实**：这一条钉的是「本域**存在**一枚上游输出 `Image`」，**不钉**「那三行的源图就是
`generate-image` 的产物」——后者的上游是哪一枚，规范与该表都写作「任一枚」。

**（A）＋（B）就是设计 §3.4 的判据**：下游的每一型都在（A）的表里（即（A2）），
表里的每一型在（B）里都有一枚上游的输出含它。**这不是把设计判据改写**——设计写的就是两半合起来的那一句。
**交集中的型与「两侧相等」的分界写进注释**：第 1 行的上游是「任一枚」，故该行**不**要求
上游的 `output_schema` 与下游的 `input_schema` 相等；断言的是**交集非空**这一半的可检查形式。

**红条件（三条，各指唯一锚点）**：
（A1）从 `local-generative-edit` 的 `input_schema` 里删掉 `Blob`（**取反档**）→（A1）红、（B）里
那一行（`local-generative-edit ← detect-edit-region : Blob`）**仍绿**（它读的是上游的输出）；
（A2）给 `verify-outside-mask` 的 `input_schema` 加一个 `ArtifactType::Text`（**取反档**）→（A2）红，
（A1）仍绿；
（B）把 `hard-composite` 的 `output_schema` 里的 `Image` 删掉（**取反档**）→（B）里**只红一行**
（`verify-outside-mask ← hard-composite : Image`）；`hard-composite ← local-generative-edit : Image`
**仍绿**——它读的是**上游** `local-generative-edit` 的输出，删 `hard-composite` 的输出与它无关。
（A1）与（A2）也不受影响（`hard-composite` 自己的 `input_schema` 一字未动）。

- [ ] **Step 2: 运行**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test schema_edges
```

**本 task 只新建测试，不交付生产代码**（Step 3 明写「无生产代码」），断言的是 Task 2 已交付的声明，
故这一步**首次即绿**，步名不叫「确认失败」。它的红由 Task 9 Step 5 的变异复核取
（改 `operators.rs` 的某一列 → 对应的（A1）／（A2）／（B）红）。

- [ ] **Step 3: 实现**

**无生产代码。** 本 task 的产物只有一份测试语料；**不许**为了让用例通过而在 `src/` 里加一个
「端口兼容判定」函数——§239 的判定已由 `crates/continuum-port/src/port.rs:94` 落地（切分 §四第 7 条）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t3-test.log
git add crates/continuum-image
git commit -m "test(image): §3.4 的边表——下游覆盖与上游来源两组断言"
```

---

### Task 4: `register_image_operators`（先查重后写）

**Files:**
- Create: `crates/continuum-image/src/register.rs`
- Create: `crates/continuum-image/tests/register.rs`
- Modify: `crates/continuum-image/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 2 的 `all_operators`、Task 1 的 `ImageError`；
  `continuum_operator::{Operator, OperatorId, OperatorVersion, OperatorRegistry, OperatorError}`
- Produces: `continuum_image::register_image_operators`

- [ ] **Step 1: 写用例**

`tests/register.rs`：

- `registering_the_whole_set_then_resolving_each_succeeds`（§11.2 第 (b) 条）：
  以 `&all_operators()` 调 `register_image_operators`，**逐枚**
  `resolve(&id, &OperatorVersion::new(1))` 成功，且读回的那一枚的 `id` 与期望相同。
  红条件：把函数改成空实现（**移除档**）→ 逐枚 `resolve` 得 `NotFound`。
- `registering_twice_yields_the_registry_arm_not_a_silent_overwrite`（§11.2 第 (c) 条）：
  同一注册表注册两次 ⇒ `Err(ImageError::Registry(OperatorError::Duplicate { .. }))`，
  **判定臂要判准**（用 `matches!` 与逐字段比较，不是「是 `Err` 就算过」），
  且**判准 `id` 与 `version` 两个字段的值**。
  红条件：把重复注册改成静默覆盖（**移除档**）→ 本条红（不再是 `Err`）。
  **锚点唯一**：`register.rs` 里把 `OperatorError` 转进 `ImageError::Registry` 的那个 `?`。
- `a_rejected_batch_leaves_the_registry_exactly_as_it_was`（§11.2 第 (c2) 条，**先查重后写的写入面**）：
  **夹具的顺序是这条断言承重的一部分，必须写进用例注释**——批 = `[新算子 A, 新算子 B, 算子 A]`
  （**重复项排在批的末尾**），注册表里**已有**算子 A ⇒ `Err`；随后逐枚 `resolve` 断言
  **一个都没有留下**（新算子 A：`resolve` 读回的仍是**原来那一枚**，逐字段同调用前；
  新算子 B：`NotFound`）。
  **观测方式写死**：`OperatorRegistry` 的 `entries` 是**私有字段、无迭代器、无 `len()`**
  （`crates/continuum-operator/src/registry.rs:15-17` 实测）——故「注册表内容与调用前逐枚相同」
  **只能经 `resolve` 观测**，**不比 `len()`**、不读私有字段。
  红条件：把函数改成边查重边写（先注册前几枚、遇到违规才返回）（**取反档**）→ 新算子 B 会被写入，
  第二条 `resolve` 断言红；而 (c) **仍绿**（返回的 `Err` 臂没变）。
  **并写明若把重复项排在首位，本用例在这一档下也绿**——「先查重后写」与「边查重边写」的差别
  只在一个**中间位置**上显形。
- `an_operator_with_an_id_outside_the_table_is_accepted`（§11.2 第 (d) 条，**另一侧**）：
  用表外的 `OperatorId`（如 `"image-extra"`）造一枚合法算子，与 `all_operators()` 的批混合注册 ⇒
  逐枚成功。红条件：给 id 加一层白名单校验（**移除档**）→ 红。
  **这一条是 (c) 的另一侧**：只钉 (c) 会让「一律拒」的实现全绿。
  **并写明**：`OperatorId` 的取值域**不由本块封闭**
  （`crates/continuum-operator/src/definition.rs:24-31` 是有字段的 newtype、无枚举），
  本块**不发明**一个 id 白名单。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test register
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`tests/register.rs` 要 `use continuum_image::register_image_operators`，而它由 Step 3 才建出，
故本步的失败形态是 `error[E0432] unresolved import`，不是断言失败，**这一步是空转的**。
**要取到运行期的红**，须先有能编译、能跑出断言的 `register.rs`——而那一写就与 Step 3 重合，
故本 task 不做（判据以 Step 4 的全量跑与 Task 9 Step 5 的变异为准）。

- [ ] **Step 3: 实现**

```rust
/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先查重后写**：先逐枚核「注册表里是否已有同 (id, version)」（判定在 P1 的
/// `OperatorRegistry::register`，`crates/continuum-operator/src/registry.rs:24-34`），
/// 全部通过后再逐枚注册；任一枚被拒即返回 `Err`，此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚图像算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§9 的批次边界要能按 §343 换，
/// 且 §11.2 第 (c2) 条（不留半注册）才写得出来。
pub fn register_image_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), ImageError>;
```

**「先查重后写」这一句的来历要写进函数的文档注释**：设计 §3.1 初稿写的是「先**核**后写：先逐枚核
§3.5 的前提」，而 **§3.5 自述本块没有那条注册期前提**（两枚 `Deterministic` 算子的候选集合是单元集，
「名单内／名单外」没有可触发的形态），故「先核」的内容落不下来；
**以查重为准的理由**：§11.2 第 (c2) 条要拍的正是「不留半注册」，而它是查重（`contains_key`）与写入
（`insert`）之间的**次序**——核的东西只能是注册表现状，不可能是 §3.5 那条本块没有的规则。

**这条函数拦不住哪一类（要写进文档注释，否则会被当成一条全覆盖的守卫）**：它只查注册表里的重复，
**不查**「这个算子的候选 backend 是否逐位可复现」（那条规则与领域无关，属 P5e 一处，设计 §3.5），
也**不查**「这个算子的端口取值域对不对」（那是 Task 2 的 (a) 与 Task 3 的边表）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t4-test.log
git add crates/continuum-image
git commit -m "feat(image): register_image_operators 的先查重后写"
```

---

### Task 5: `resolve_edit_region` 与 `RegionDetector`（§325 的唯一落点）

**Files:**
- Create: `crates/continuum-image/src/resolve.rs`
- Create: `crates/continuum-image/tests/resolve_region.rs`
- Modify: `crates/continuum-image/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 1 的 `EditRegion` / `EditRegionKind` / `EditMask` / `NormalizedAnchor` / `NormalizedBox` /
  `SemanticObjectRef` / `ImageError`；`continuum_artifact::ArtifactId`
- Produces: `continuum_image::{RegionDetector, resolve_edit_region}`

**本 task 是本块的第一条重活**：§4.3 明写「本节是本设计里发明判据风险最大的一处」。
它的判据缺口**一条都不在本 task 里被填**——本 task 给的是**落点**与**一个 fail-closed 的边界**。

- [ ] **Step 1: 写用例**

`tests/resolve_region.rs`。
**梗的夹具不许取自本块的任何名单或常量**（设计 §11.2 的第二条预防）：
`Point` 与 `SemanticObject` 两支的 detector **必须是用例自己写的桩**
（一个返回固定掩码、一个返回空），否则那些用例都在测「桩与它自己一致」。

- `the_box_arm_projects_the_normalized_rect_onto_the_source_canvas`（§11.2 第 (d) 条）：
  手写 `source_dims`（如 `(8, 4)`），`NormalizedBox` 的两角给出手写值；
  断言返回的 `EditMask` 的宽高等于 `source_dims`，且**被置位的像素集合恰是手写期望的那个集合**
  （逐像素比对，不是只看长度、不是只看「有像素被置位」）。
  红条件：把矩形的 `x` 那一维用 `height` 相乘（**取反档**）→ 期望集合不符。
  **并写明**：`Box` 支是**投影**（无模型），它是四支里唯一不调 detector 的一支（§4.2 的表）。
- `a_box_order_swapped_gives_the_same_mask`（**运行期**，钉 Task 1 的「两角名字不表达几何次序」）：
  同一个矩形的两角**对调**构造两个 `NormalizedBox` ⇒ 两次的掩码**逐字节相同**。
  红条件：把投影写成「假定 `p` 是左上、不取 min/max」（**取反档**）→ 对调后掩码不同，本条红。
- `a_box_reaching_outside_the_unit_square_is_clamped_to_the_canvas`（**本计划自定**，
  见 `## 遗留` 第一节第 4 条与第五节）：`NormalizedBox` 带一个 >1 的角（如 `x = 1.5`）⇒ `Ok`，
  且掩码的宽高仍是 `source_dims`、`pixels.len()` 满足不变量、被置位的像素不越界。
  红条件：把夹取去掉（**移除档**）→ 该用例 panic（越界索引）或长度断言红。
  **并写明为什么是夹取而不是新增一枚错误臂**（写进用例注释与函数的文档注释）：
  `ImageError` 的四臂里**没有**一枚表达「坐标越界」——`RegionUnresolved` 说的是「解析不出区域」、
  `ShapeMismatch` 说的是「两载体形状不同」，两者都表达不了这一件事；而**臂数是设计 §3.1 钉死的四臂**，
  故本计划取夹取。**代价**：越界输入被静默地当成「贴边」；**收益**：不新增一枚设计没有的臂，也不 panic。
  §163 给了 `x, y ∈ [0, 1]` 的约定而**没有给越界输入的处置**——这一处是本计划补的，**不是规范给的**。
- `the_mask_arm_passes_the_bitmap_through_verbatim`（§11.2 第 (e) 条）：
  返回的 `EditMask` 与入参的掩码**逐字节相同**（含宽高）。
  红条件：把 `Mask` 支也走一遍投影（**取反档**）→ 逐字节断言红。
  **并写明射程**：这一条钉的是「调用方已给的位图不被本函数改写」（§166 `:750` 逐字「调用方已给出掩码位图」），
  **不钉**「该位图的尺寸与 `source_dims` 一致」——§4.2 的表**没有**这条前置，见本节末的一条据实说明。
- `the_point_arm_returns_the_detectors_region_verbatim`（§11.2 第 (f) 条）：
  桩 detector 返回一个固定的掩码 ⇒ 返回的掩码**逐字节等于**该掩码（本函数不合成像素、不改写区域）。
  红条件：把 `Point` 支改成「以该点为中心造一个固定大小的方框」（**取反档**）→ 红。
- `a_point_with_no_region_is_rejected_and_never_degenerates`（§11.2 第 (g) 条）：
  桩 detector 返回「找不到」⇒ `Err(ImageError::RegionUnresolved { region: EditRegionKind::Point })`，
  **判准 `region` 字段的值恰是 `Point`**。
  **「且不是单像素掩码、不是整图掩码」这一半的写法要写死**：`Err` 那一支没有掩码可看，
  故这条钉的是**返回了 `Err` 而不是 `Ok`**——「退化成单像素」的实现会返回 `Ok`，
  本条即红；「放大到整图」的实现同样返回 `Ok`，本条同样红。
  **红条件**：把零检出改成「合成一个单像素掩码并返回 `Ok`」（**取反档**）→ 红；
  改成「放大到整图并返回 `Ok`」（**取反档**）→ 红。
  **并写明这条钉不到什么**（§4.3 末句与 §11.5 第 3 条）：它钉的是**零检出的处置**，
  **钉不到**「区域选得对不对」——§4.3 表里那五条缺口（选取规则 / 置信度门槛 / 零检出 / 区域上限 /
  `SemanticObject` 的名字空间）在这一点上**没有判据**，故本处只能拍「桩的返回被原样传出去」与
  「空返回即 `Err`」两条。
  **理由写进注释**（设计 §4.3）：§325 的 MUST 是**否定式**的（「而不是单像素修改」），
  故最小的合法实现是「解析不出就拒绝」；放大到整图会让 §326 的 `outside_mask_change = FORBIDDEN`
  变得无意义（mask 外为空集，任何改动都「合法」），退化到单像素正被 §325 明文禁止。
- `a_semantic_object_with_no_match_is_rejected_with_its_own_kind`（**本计划补的第四支**，
  见 `## 遗留` 第一节第 2 条）：桩 detector 对名字返回「找不到」⇒
  `Err(ImageError::RegionUnresolved { region: EditRegionKind::SemanticObject })`，
  **判准 `region` 字段是 `SemanticObject` 而不是 `Point`**。
  红条件：把 `resolve_edit_region` 的两处 `RegionUnresolved` 都写成 `region: Point`（**取反档**）→ 红。
  **来历写进注释**：设计 §11.2 的 (d)–(h) 覆盖了 `Box` / `Mask` / `Point` 三支，
  **`SemanticObject` 那一支没有照片**，而 §4.2 的表列了四支、§3.1 的 `RegionUnresolved` 注又逐字写着
  它也覆盖它（「Point 点空处、`SemanticObject` 名不副实」）——**枚举到的那几支之外，第四支是 fail-open**。
- `the_same_region_yields_the_same_return_across_calls`（§11.2 第 (h) 条，**两支各一条，不抽代表**）：
  同一个 `Point` 与同一个 `SemanticObject` **各**在两次调用里给出**逐字节相同**的返回值
  （函数自身无随机）。
  红条件：在函数里掺一次与入参无关的扰动（**取反档**）→ 两条都红。
  **并写明它是 (f) 的对照**（设计 §11.2 的预防）：只钉 (f) 的话，一个「函数自己造一个区域」的实现
  也能满足 (f)——本函数造出的区域在两次调用里相同，(h) 照样绿；**(f) 与 (h) 合起来**才要求
  「返回的等于 detector 给的」而「两次相同」。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test resolve_region
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`RegionDetector` / `resolve_edit_region` 由 Step 3 才建出，故本步的失败形态是
`error[E0432] unresolved import`，不是断言失败，**这一步是空转的**。

- [ ] **Step 3: 实现**

```rust
/// §164 `:682-691` 的 Object / Region Detection 的**接口**。本块定义它，实现在 backend 一侧
/// （第 3 层选定的后端）。
///
/// **返回值的形状**：`Option<EditMask>`——`None` 表示「这一支找不到区域」，
/// `resolve_edit_region` 把它变成 `Err(RegionUnresolved { .. })`。
///
/// **两个方法的形状、参数与这个名字都是本计划自定的取值**：设计 §2.2 只说「本块定义它，
/// 其实现在 backend 一侧」，§4.2 只说「`detector` 的返回值就是掩码位图（`EditMask`）」，
/// **没有给方法名、参数与返回类型**。依据与代价见 `## 遗留` 第一节第 3 条：
/// 两个方法收 `source` 与 `source_dims`——**收 `source` 是必需的**，否则 `resolve_edit_region`
/// 的 `source` 实参在函数体内没有用途（一个死形参），而 §4.2 的签名里它有。
/// 两处都要归一化坐标，故两处都要画布尺寸（§4.2：「三处都要源图的画布尺寸」）。
pub trait RegionDetector {
    /// §164 `:684`：把一个**归一化点**解析成该点所在的语义区域。
    fn region_at(
        &self,
        source: &ArtifactId,
        source_dims: (u32, u32),
        point: &NormalizedAnchor,
    ) -> Option<EditMask>;

    /// §166 `:751` / `:769`：按**名字**找该对象的掩码。
    fn region_named(
        &self,
        source: &ArtifactId,
        source_dims: (u32, u32),
        name: &SemanticObjectRef,
    ) -> Option<EditMask>;
}

/// §325 `:2360-2366` 的唯一落点：把四种编辑区域表示解析成 §166 `:775` 的 `EditMask`。
///
/// **`source_dims` 是必需参数**：`Box` 支要把归一化矩形投影成像素掩码，而投影的坐标系就是源图的画布；
/// `Point` 与 `SemanticObject` 两支也要把它交给 detector。**本函数不自己读**：从制品读尺寸要经
/// `Artifact.metadata`（那份 schema 规范未给，§13 第 5 条），且那是一次 IO，而本块不做 IO（§3.1）。
/// **`detector` 是必需参数**：`Point` 与 `SemanticObject` 两支都要经对象/区域检测（§164 `:684`），
/// 故本函数**没有**任何一条「不传 detector 也能把 `Point` 变成掩码」的路径。
///
/// **本函数的四支共用同一个入口、同一个返回类型**——这是 §4.4 所说「本 crate 里没有第二条从
/// `Point` 到 `EditMask` 的路」的兑现方式。**这一句话的射程**：它只说「本 crate 里没有绕开的路」，
/// 说不了「别的 crate 不会自己写一个 Point→整图的转换」（后者没有落点）。
///
/// **本函数不核 detector 返回的掩码与 `source_dims` 是否一致**：§4.2 的表没有这条前置，
/// 故尺寸不符的掩码会 `Ok` 通过区域解析，直到 `composite_masked` 才被 `ShapeMismatch` 拦住。
/// 于是「区域解析成功」与「掩码可用于合成」是两件事，**只有前者在本函数的返回类型里**。
pub fn resolve_edit_region(
    region: &EditRegion,
    source: &ArtifactId,
    source_dims: (u32, u32),
    detector: &dyn RegionDetector,
) -> Result<EditMask, ImageError>;
```

**四支各自的判据照 §4.2 的表**（`Box` 投影 / `Mask` 原样 / `SemanticObject` 经 detector 按名字 /
`Point` **必须**经 detector 找语义区域、**不得**退化成单像素），本计划不重抄该表。

**§4.3 的处置要写进函数的文档注释，逐条照录、一条都不发明**：

- **规范给了什么**：MUST（§325 `:2360`）、机制（§164 `:684` 的 Object / Region Detection）、
  一个例子（§164 `:699-705` 的「hand + held object」）、坐标约定（§163 `:648`）。
- **规范没给什么**（**五条，逐条给出「规范里最接近的」与「为什么它不够」**）：
  ①「点落在多个对象上时取哪一个」的判据（最接近的是 §164 `:699-705` 的 `hand + held object`，
  那是一例、不是选取规则）；②检测的**置信度阈值**与低于阈值时的处置（规范全文没有置信度这个量）；
  ③**零检出**时的处置（无）；④区域**大小的上限**（最接近的是 §324 `:2343` 的
  `preserve_outside_region = true`，它说的是 mask 外不变、不是 mask 能有多大）；
  ⑤`SemanticObject` 的**名字**从哪来（最接近的是 §166 `:769` 的「左边这个人」，那是一句自然语言、
  不是名字空间）。
- **本设计取的那一条**：**零检出 ⇒ `Err(RegionUnresolved)`（fail-closed），不放大、不退化、不请模型猜**
  （理由见 Task 5 Step 1 的 `a_point_with_no_region_…` 那条用例的注释）。
- **代价（这一条也要写，否则只写了落点与边界）**：**零检出即 `Err` 的直接代价是合法点击会被拒**。
  误检、稀疏对象、点击落在对象边缘之外都会走到这一支；而 `ImageError::RegionUnresolved` 是
  **终止性的**——它不重试、不放宽、不给近似区域，故每一次误拒都要**用户重选**（或重跑 detector）
  才能继续。取这个代价的理由是两侧不对称：**误拒是一次可恢复的往返**，而「放大到整图」会让
  §326 的判定失去意义。**这一条不声称代价小**，只声称它比另一侧的代价小。
- **本条不解决什么，写清**：本函数只是**给 §325 的 MUST 一个落点与一个 fail-closed 的边界**，
  **没有让「Point → 语义区域」这件事变得有判据**；上表五条**仍在**。
- **本处无照片（写明）**：五条缺口都是**判据的边界**，而边界没有判据——故这里拍得到的只有
  「桩的返回被原样传出去」与「空返回即 `Err`」两条。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t5-test.log
git add crates/continuum-image
git commit -m "feat(image): resolve_edit_region 与 RegionDetector——§325 的唯一落点与 fail-closed 边界"
```

---

### Task 6: 两个确定性函数与三条前置条件（§5.2 §5.3）

**Files:**
- Create: `crates/continuum-image/src/pixels.rs`
- Create: `crates/continuum-image/tests/pixels.rs`
- Modify: `crates/continuum-image/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 1 的 `EditMask` / `ImageError`
- Produces: `continuum_image::{Raster, PixelDiff, composite_masked, outside_mask_difference}`

**本 task 是本块的第二条重活**：§5.4 明写「类型层拦不住」，而它的 fail-open 那一侧**没有照片**。

- [ ] **Step 1: 写用例**

`tests/pixels.rs`：

- `the_two_carriers_declare_exactly_their_fields`（**编译期照片**）：
  `Raster` 的字段清单**逐项穷尽解构、不带 `..`**：`width: u32`、`height: u32`、`channels: u8`、
  `pixels: Vec<u8>`；`PixelDiff` 的字段清单逐项：含一个可判 `== 0` 的差异量。
  红条件：给 `Raster` 加一个 `color_space` 字段（**移除档** → 穷尽解构先编译不过）。
  **明写两条**：（i）编译期照片，运行期无照片（纯数据声明）；
  （ii）`PixelDiff` 的**形状**：`differing` 这个**字段名是设计给的**——设计 §11.2 第 (l) 条的用例
  逐字写 `PixelDiff { differing: 0, .. }`（实测在设计 `:520`）；**本计划自定的是它的整体形状
  与总数那一枚的名字**，因为设计 §5.3 只约束「它必须含一个可以判 `== 0` 的差异量」与
  「返回 mask 外**不相等**的像素数与总像素数」两句（见 `## 遗留` 第五节）。
  **`Raster` 的文档注释要写清设计 §5.2 的那一条**：本块**不定义像素格式**——
  §168 `:826` 只给了「Alpha / 色彩空间是否异常」这一条检查名，没有给通道语义、色彩空间或 Alpha 的表示；
  故本型只承载「宽、高、通道数、字节序列」，通道的语义由 backend 的约定决定。
- `composite_copies_each_region_from_its_own_source`（§11.2 第 (i) 条）：
  夹具 `source` 与 `generated` 在 **mask 内不同、mask 外相同**；断言输出的 mask 外**逐点等于** `source`、
  mask 内**逐点等于** `generated`（逐像素比对）。
- `composite_forces_the_outside_back_to_the_source`（§11.2 第 (j) 条，**强制作的那一侧**）：
  夹具的 `generated` 在 **mask 内与 mask 外都不同**于 `source`；断言输出的 mask 外**逐点等于** `source`
  （mask 内等于 `generated`）。
  红条件：把 `composite_masked` 改成「原样返回 `generated`」（**放宽档**）→ 本用例红，
  而 (i) **仍绿**（它的夹具在 mask 外本来就没有差别）。
  **两张夹具的分工要写进注释**（设计 §11.3 那一行的落脚）：这一档只红 (j) 是**预期**；
  **若连 (j) 都不红，说明夹具里没有一个 mask 外被改过的对照件**——(i) 与 (j) 两张夹具合起来
  才是「强制作」的照片，**只有 (i) 的夹具时此档全绿**。
- `a_shape_mismatch_is_rejected_on_every_pair`（§11.2 第 (k) 条，**逐对，不抽代表**）：
  **四对各一条**——① `mask` 与 `source` 的宽高不同；② `generated` 与 `source` 的宽高不同；
  ③ `generated` 与 `source` 的**通道数**不同；④ `outside_mask_difference` 的 `result` 与 `source`
  的宽高不同 ⇒ 各得 `Err(ImageError::ShapeMismatch { left, right })`，
  **并判准 `left` 与 `right` 两个字段的值**（掩码那一侧的第三分量是 `None`，见 Task 1 的 `ShapeMismatch`）。
  红条件：删掉第③对的那条判定（**放宽档**）→ 第三条红，其余三条**仍绿**。
  **并写明为什么第③对与第①②对同级**（设计 §5.2）：mask 内的像素取自 `generated`，逐点复制要求二者
  **逐像素一一对应**；尺寸或通道数不同时「mask 内取自 `generated`」这一步没有定义，函数的行为是未定义的。
- `a_pixel_length_mismatch_is_its_own_arm_on_both_sides`（§11.2 第 (k2) 条，**两条**）：
  掩码侧与 `Raster` 侧**各一条**（一个载体的 `pixels` 字节数与它自己声明的形状不符）⇒
  `Err(ImageError::PixelLengthMismatch { len, expected })`，**并判准 `len` 与 `expected` 两个字段的值**，
  **且用 `matches!` 断言它不是 `ShapeMismatch`**。
  红条件：把这一条并进 `ShapeMismatch`（**移除档**）→ 两条的臂别断言红。
  **并写明**：这一条同时钉住「两臂没被并成一臂」（设计 §3.1 的四臂判据）——
  若实现把第六感并进 `ShapeMismatch`，本节第 (k) 条的四对断言**照样绿**，只有本条的臂别断言会红。
  **理由写进注释**：（k）比的是**两个载体之间**的关系，本比比的是**一个载体内部**的两件事——
  它自己声明的形状与它自己字节序列的长度；而 `ShapeMismatch` 的字段是 `(宽, 高, 通道数)`，
  **两个 `usize` 的长度值表达不进那个形状**。
- `the_comparator_reports_the_difference_and_the_total`（§11.2 第 (l) 条，**两侧**）：
  mask 外**全同** ⇒ 差异量为 `0`（**并判总数**）；把 mask 外**一个**像素改掉 ⇒ 差异量恰为 `1`。
  **「总数」是哪一种总数要写死，并写明它的来历**：设计 §5.3（实测在设计 `:510`）逐字只写
  「返回 mask 外**不相等**的像素数与**总像素数**」，**没有指明后者是整图的像素数还是 mask 外的像素数**
  （两种读法都通；`docs/spec/03-product-drive.md:832` 的 §168 只给了 `outside-mask pixel difference`
  这个检查名，没有给这两个量的定义）。**本计划取「mask 外的像素数」**，理由是分子与分母同域：
  分母若换成整图，则「mask 外全同」时读到的比例是 `0 / 整图像素数`，与「mask 外的改动占比」不是同一个量。
  **这一读数属本计划自定，已入 `## 遗留` 第五节**；**若设计作者取的是另一种读法，本条的期望值与红条件
  随之改、而动作不变**（收件人见 `## 遗留` 第一节第 11 条）。
  **夹具要含一个掩码**（mask 外的像素数 ≠ 整图的像素数），否则两种读法在读数上不可分辨，
  本条既钉不住本计划的读法、也钉不住另一种。
  红条件：把计数改成「mask 内」的（**取反档**）→ 第一条红（mask 外的改动被漏掉）；
  把总数写成「整图的像素数」（**取反档**）→ 第一条的总数断言红。
  **两侧都要**：只钉「全同 ⇒ 0」会让一个恒返回 `0` 的实现全绿（比较器什么都看不见）。
- `the_comparator_declares_no_verdict`（§5.3 的第二半、§7.3）：
  返回类型是 `Result<PixelDiff, ImageError>`，**不是 `Result<bool, _>`**；
  `PixelDiff` 上**没有** `passed()` / `is_ok()` 一类判定方法。
  **照片的落点分两处，写清**：（i）**返回类型**那一半是**编译期照片**——本用例把返回值绑到
  `PixelDiff` 上并读它的字段，返回类型若是 `Result<bool, _>`，本用例**编不过**；
  （ii）「`PixelDiff` 上**没有**判定方法」那一半**不是穷尽解构能证的**（解构只钉字段清单，
  钉不到方法的有无），它由 **Task 9 Step 3 的源码面复核 (g)**（`grep -rn "Result<bool\|-> bool"`）取。
  **理由写进注释**（§5.3）：返回 `bool` 就把一条判据（阈值）塞进了本块，而 §326 给的是
  `= FORBIDDEN`（阈值为 0 的那一个特例）、§168 给的是「可以直接确定性检查」——
  **两者都不足以让本块替 P5a 定「多少算不过」**。故本块给**量**，判**在 P5a**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test pixels
```

**这一步量到的是编译失败，按纪律 1(c) 不算红**——照实写成**编译期照片**：
`composite_masked` / `outside_mask_difference` / `Raster` / `PixelDiff` 由 Step 3 才建出，
故本步的失败形态是 `error[E0432] unresolved import`，不是断言失败，**这一步是空转的**。

- [ ] **Step 3: 实现**

```rust
/// 本块为两个确定性函数定的**最小图像像素载体**。
/// **本设计不定义像素格式**（§168 `:826` 只给了「Alpha / 色彩空间是否异常」这一条检查名）。
/// **这里不是「直接收 `Vec<u8>` 加两个 usize」**：三个量一起传时调用方可以传错配的对，
/// 而合成与比较**都要**这三个量一致才有意义；收成一个值，错配在类型上就只有一个来源。
/// **不变量**：`pixels.len() == width as usize * height as usize * channels as usize`。
pub struct Raster { pub width: u32, pub height: u32, pub channels: u8, pub pixels: Vec<u8> }

/// §168 `:832-835` 的 `outside-mask pixel difference` 的产物。
/// 设计只约束「它必须含一个可以判 `== 0` 的差异量」与「mask 外不相等／总的像素数」两句。
/// **`differing` 这个字段名是设计给的**（设计 §11.2 第 (l) 条的用例逐字写 `PixelDiff { differing: 0, .. }`）；
/// **本计划自定的是整体形状与总数那一枚的名字**——设计未指明「总像素数」是整图还是 mask 外，
/// 本计划取后者（理由与收件人见 Task 6 的 `the_comparator_reports_the_difference_and_the_total`
/// 与 `## 遗留` 第五节）。
pub struct PixelDiff { /* differing / total */ }

/// §326 `:2378-2388` 的硬合成：mask 外的像素从 `source` 逐点复制，mask 内的取自 `generated`。
/// 纯函数、无 IO、无执行路径（切分 §四第 4 条只禁「执行路径」，不禁本块自己的判定函数）。
///
/// **三条入口前置条件，逐条给出**：
/// 1. `(mask.width, mask.height) == (source.width, source.height)`；
/// 2. `generated` 与 `source` 的宽、高**与通道数**三者都相同；
/// 3. 三个载体的 `pixels` 长度与各自声明的形状相符。
/// 第 1、2 条不成立即 `ShapeMismatch`（掩码那一侧的第三分量是 `None`）；
/// 第 3 条不成立即 `PixelLengthMismatch`，**这一条不走 `ShapeMismatch`**。
/// **这里不是「尺寸不符时按左上角对齐凑合」**：§168 `:825` 的「分辨率是否保持」是一条独立的检查，
/// 合成时按坐标截取会让「掩码与源图不同坐标系」这件事在结果里不可见——而它是 §326 的前提。
pub fn composite_masked(source: &Raster, generated: &Raster, mask: &EditMask) -> Result<Raster, ImageError>;

/// §168 `:832-835` 的 `outside-mask pixel difference`。
/// 确定性：逐点比较，无模型、无阈值。返回 mask 外**不相等**的像素数与总像素数。
/// **前置条件与 `composite_masked` 同形三条**（`mask` / `source` / `result` 三者宽高相同；
/// `source` 与 `result` 通道数相同；三个载体的 `pixels` 长度与各自声明的形状相符）。
/// **本函数不判「通过」**：它给出差多少，「差多少算不通过」的判据不在本块（§5.3）。
pub fn outside_mask_difference(source: &Raster, result: &Raster, mask: &EditMask)
    -> Result<PixelDiff, ImageError>;
```

**§5.4 的处置要写进这两个函数的文档注释，逐条照录**：

- **三层，逐层给出它保证到哪**：**强制作（使之合法）**＝`hard-composite`（Task 2 的第 4 行）——
  走过这一步的结果图**按构造**满足 §326；**检出（发现不合法）**＝`verify-outside-mask`
  （第 5 行）与 `outside_mask_difference`——**确定性**，且**不读模型自述**
  （§168 `:837-843` 逐字「这比让模型说『我只修改了手。』可靠得多」）；**类型层**＝**没有**。
- **类型层拦不住的依据**（设计 §5.4）：`ArtifactType`（`crates/continuum-artifact/src/artifact.rs:14-21`）
  是**内容类型的枚举**，它没有、也不该有「这张 `Image` 与那张 `Image` 在 mask 外逐点相同」这种
  **跨制品关系**的表达力。故 §326 的 FORBIDDEN 是**校验**与**强制作**，不是类型约束；
  **本设计不为此向 P5e 要新变体**。
- **失配方向（这是本条要回答的那一问）**：
  **A. 当 `preserve_outside_region = true` 的节点没有接上 `hard-composite` 时，不报错**——
  产出是一张 mask 外被改动过的 `Image`，它与其他 `Image` 在类型上**完全一样**，
  只有跑了比较器才看得见。**这一侧是 fail-open。**
  故 §326 在本块的兑现是「提供使之成立的步骤 ＋ 提供检出它的检查器」，
  **不是**「保证它一定被用上」——后者要求「谁保证带这条参数的节点一定接了 `hard-composite`」，
  而规范未给（§13 第 2 条）。
  **B. 反向的一侧**：若 `hard-composite` 被接上但掩码与源图不同坐标系（或 `generated` 与源图形状不同）
  ⇒ `Err(ShapeMismatch)`。**这一侧是 fail-closed**，故它**不能**用来代替 A：
  它拦的是「掩码／补丁配错」，拦不了「根本没有合成」。
- **本处无照片（写明）**：拍得到的只有这两个函数的返回值；**拍不到「图里真有一段管线没接上合成」**
  ——那一侧的失配正是 fail-open 本身。

**前置条件的实现要收敛到一处**（一个私有函数核第 3 条、一个私有函数比形状），
**且两个公开函数都要调它**——否则两个函数会各自漂移（本仓「同一件事两套词汇表」的形状）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t6-test.log
git add crates/continuum-image
git commit -m "feat(image): composite_masked 与 outside_mask_difference——§326 的强制作与检出"
```

---

### Task 7: §3.5 的复用照片与 §6 的谱系照片（两处判据都在 P1，本块只重跑）

**Files:**
- Create: `crates/continuum-image/tests/reuse.rs`
- Create: `crates/continuum-image/tests/lineage.rs`
- Modify: `crates/continuum-image/Cargo.toml`（dev-dep `continuum-graph`、`continuum-persist`、`tempfile`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加两条 dev 边）

**Interfaces:**
- Consumes: Task 2 的 `all_operators`；`continuum_artifact::{Artifact, ArtifactId, ArtifactType, ContentHash,
  PrivacyClass, save_artifact, load_artifact, p1_artifact_migrations}`；
  `continuum_graph::{cache_key, can_reuse, CacheKey}`；
  `continuum_persist::{Db, builtin_migrations}`
- Produces: 无生产类型（**语料与夹具都在测试内**）

**本 task 的两组照片是「P1 判据的重跑」，不是本块判据的照片**——这一点要写进两组用例的注释，
免得复审按与本块别的用例同等的强度读它们（`## 遗留` 第四节）。

- [ ] **Step 1: 加两条 dev 边**

`ALLOWED` 的 `continuum-image` 条目的内层数组改为（**数组按字母序**）：

```rust
    ("continuum-image",
     &["continuum-artifact", "continuum-graph", "continuum-operator", "continuum-persist"]),
```

**`continuum-persist` 这一项不在设计 §2.1 的边表里**（那一节逐字写着「**不登记 `continuum-persist`**：
本块零表，不碰 `Tx` / `Migration`」），而 §11.2 第 (m)、(n) 两条照片要起真库、经 `Tx` 调
`save_artifact` / `load_artifact`——**两处只能有一真**。**本计划的取舍与本计划的理由见
`## 遗留` 第一节第 1 条**（收件人：设计作者 ＋ 协调者）。
**这一条 dev 边的边别依据**：生产代码对它零调用（`src/` 里 `continuum_persist` 零命中，
见 Task 9 Step 3 的源码面复核第 (a) 条），唯一使用点是 `tests/lineage.rs`。

- [ ] **Step 2: 写用例**

`tests/reuse.rs`（§3.5 的复用照片，**dev 边 `continuum-graph`**）：

- `the_two_deterministic_operators_get_a_cache_key_and_can_be_reused`（§11.2 第 (o) 条，**逐枚两条**）：
  对 `hard-composite` 与 `verify-outside-mask` 各一条——`cache_key(op, input_hash)` 为 `Some(..)`；
  且四条件齐时（`cached = Some(&key)`、`current_input_hash == key.input_hash`、
  `contract_affected = false`）`can_reuse` 为真。
  红条件：把 `hard-composite` 的 `determinism` 改成 `NonDeterministic`（**收紧档**——见纪律 1 的宽紧口径）
  → 那一条红、(p) **仍绿**。
  **这一条是「有读数」的那一侧**（§11.2 第 (o) 条把复用前提成立的那一半交给它）。
- `the_three_nondeterministic_operators_never_get_a_cache_key`（§11.2 第 (p) 条，**逐枚三枚**）：
  `cache_key` 对 `generate-image` / `detect-edit-region` / `local-generative-edit` **逐枚**为 `None`。
  红条件：把 `generate-image` 的 `determinism` 改成 `Deterministic`（**放宽档**，与设计 §11.3 那一行的
  档位不同，见 `## 遗留` 第一节第 5 条）→ 那一条红，**且 Task 2 的 (a) 的对应行也红**（期望值手写）。
  **两条合起来才是这条守卫**（纪律 8）：只钉 (p) 会让「一律 `None`」的实现全绿
  （`can_reuse` 永远为假）；只钉 (o) 会让「一律 `Some`」的实现全绿。
  **并写明这一条与 Task 2 的 (a) 是两条独立的守卫**（设计 §3.5 末与 §11.3）：
  (a) 读的是**声明的取值**，(o)/(p) 读的是 P1 的**判定结果**；把某一枚的 `determinism` 改成
  `Deterministic` 会让两条都红，而把某一枚的 `backend_candidates` 改名只让 (a) 红。
  **两条都要留。**

`tests/lineage.rs`（§6 的谱系照片，**dev 边 `continuum-persist`**）：

起库形状照本仓既有先例（`crates/continuum-artifact/tests/artifact_store.rs:109-110`）：
`tempfile::TempDir` ＋ `Db::open_with(&dir.path().join("t.db"), builtin_migrations() 加 p1_artifact_migrations())`
＋ `db.migrate()` ＋ `db.begin()`。

- `an_edited_artifact_carries_the_source_in_its_inputs_and_a_new_id`（§11.2 第 (m) 条）：
  写一枚 `Artifact`（`id = "img-v2"`、`input_artifacts = [ArtifactId::new("img-v1")]`、
  `version = 2`、`artifact_type = ArtifactType::Image`——**这一枚要 P5e 的变体**）并读回 ⇒
  `id` 是 `img-v2`、`input_artifacts` 恰是 `[img-v1]`、`id != ` 源 id；
  **并裸查 `artifact_input` 表**（`tx.query`）确认那一行在
  （`(artifact_id, input_artifact_id) == ("img-v2", "img-v1")`）。
  **明写这一条的性质（写进用例注释）**：它**不是**本块判据的照片——
  本块的 crate 里**没有产出 Artifact 的代码**（本块不实现 `OperatorImpl`，切分 §四第 4 条），
  故这一条是**P1 写读路径的重跑**：它钉「`input_artifacts` 与 `id` 经 `save_artifact` / `load_artifact`
  往返可读」，**钉不到**「本域的算子产出时确实这么填」。
  后者的承重者是 §327 在 P1 侧的那条**缺席**（`artifact.id` 是 `TEXT PRIMARY KEY` ＋
  全仓没有任何 `UPDATE artifact`，切分 §四第 7 条第四处）——**它没有承重的函数**，**不在本块**。
  红条件（**靶子不在本块的交付物里，只做一轮本地变异**）：把 `continuum-artifact` 的 `save_artifact`
  里写 `artifact_input` 的那个循环删掉（**移除档**）→ 本条红。
  **这一轮只能在本地工作树上做、必须还原，不许把那个改动提交**（纪律 2）。
- `a_second_write_of_the_same_artifact_id_is_rejected_and_the_row_is_unchanged`（§11.2 第 (n) 条）：
  起库 → 一个事务里 `save_artifact` 写一枚并 `commit` → `load_artifact` 取 `before`；
  再开一个事务，用**同一个 `id`** 写第二次 ⇒ `Err`（**具体是哪一种 `PersistError` 要在报告里记下来，
  本计划不预设它的变体名**），该事务**不提交**（照 `save_artifact` 的文档注释：
  返回 `Err` 之后调用方必须回滚该事务）；第三个事务 `load_artifact` 取 `after` ⇒
  **`assert_eq!(before, after)`**（`Artifact` 有 `PartialEq`，逐字段相同）。
  **判据的作用域要写清**：这一条钉的是**「行没被改」**，**不是**「行还在」
  ——只判「行还在」的话，一个 `INSERT OR REPLACE` 的实现照样绿。
  红条件（**靶子同样不在本块**）：把 `save_artifact` 的 `INSERT INTO artifact` 改成
  `INSERT OR REPLACE INTO artifact`（**移除档**）→ 本条红（`after != before`）。
  **并写明它是 P1 判据的重跑**（设计 §6.2 逐字：「用同一个 id 写第二次 ⇒ `Err`（**这是 P1 判据的照片，
  本块只是重跑它**）」），本块**不建第二份「源图未被覆盖」的检查**。

- [ ] **Step 3: 运行**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test reuse
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test lineage
```

**本 task 只新建测试，不交付生产代码**（`cache_key` / `can_reuse` / `save_artifact` / `load_artifact`
都是 P1 已交付的），故这一步**首次即绿**，步名不叫「确认失败」。
它的红由 Task 9 Step 5 的变异复核取（两组各一轮，其中 (m)、(n) 两轮的靶子在 `continuum-artifact`）。

- [ ] **Step 4: 实现**

**无生产代码。** 本 task 的产物只有测试夹具体。**不许**为了让用例通过而在 `src/` 里加
「复用判定」或「不覆盖检查」的函数——§305 与 §327 的判定都已在 P1 落地（切分 §四第 7 条）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t7-test.log
git add crates/continuum-image Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "test(image): §3.5 的复用照片与 §6 的谱系照片"
```

---

### Task 8: 结构层的编译期照片（trybuild）

> **硬前置：P5a 的 `continuum-verify` 已交付**（跨计划前置第二节）。该 crate 不存在时本 task 编不过。

**Files:**
- Create: `crates/continuum-image/tests/type_level.rs`
- Create: `crates/continuum-image/tests/compile_fail/<三份样例>.rs`（各配同名 `.stderr`）
- Modify: `crates/continuum-image/Cargo.toml`（dev-dep `continuum-verify`、`trybuild`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-verify`）

**Interfaces:**
- Consumes: Task 5 的 `resolve_edit_region` / `EditRegion` / `NormalizedAnchor`；
  `continuum_artifact::ArtifactId`；**`continuum_verify::Evidence`**（P5a 的，未交付）
- Produces: 无生产类型

- [ ] **Step 1: 写样例与驱动**

`tests/type_level.rs` 的形状照本仓既有做法（`crates/continuum-connector/tests/type_level.rs:12-16`）：
一个 `#[test]` 函数，`trybuild::TestCases::new()` 加一条
`t.compile_fail("tests/compile_fail/*.rs")`。
**样例名描述的是它钉住的那条不可表达性命题**（不是「某某用例 1」），**逐份各不相同**：

1. **`a_point_region_without_a_detector_does_not_compile.rs`**：写一次
   `resolve_edit_region(&region, &source, (8, 4))`（**少传 `detector`**）⇒ 编不过。
   钉的是「`Point` 那一支没有 detector 就调不动」（设计 §4.4 的第一条）。
2. **`a_box_region_without_the_source_dims_does_not_compile.rs`**：写一次
   `resolve_edit_region(&region, &source, &detector)`（**少传 `source_dims`**）⇒ 编不过。
   钉的是「`Box` 那一支没有源图尺寸就投不出掩码」（同上）。
   **两份分开的理由要写进注释**：它们是**两个不同的实参**缺了，而设计 §11.4 第 1 条把它们写成一句话
   （「少传 `detector`（或少传 `source_dims`）」）——**一份样例只能钉其中一个**，
   且**两份的 `.stderr` 逐份不同**，不许拿一个错误码套两处。
3. **`evidence_cannot_be_built_outside_its_crate.rs`**：用结构体字面量试造 `Evidence`
   （`continuum_verify::Evidence` 的八个字段全私有）与 `Evidence::default()`（无实现）⇒ 编不过。
   **`.stderr` 与样例的最终形态取自实跑**（本计划给的名字与写法都只是示意）；
   **样例里的注释也是照片坐标**——注释写错了，`.stderr` 引的行号与列位置随之漂。
   **判据**：P5a 设计 §4.4 逐字——「`Evidence` 的八个字段全部私有、无 `pub` 字段、无 `Default`、
   无 `From`/`FromStr`、**无第二构造点**。故『绕过 `from_tool_result` 造一条证据』在类型上写不出来」。
   **这一条在本块的分量比在 P5e 小**（设计 §11.4 第 2 条）：本块不写域包装（§7.2），
   故它是本块侧**唯一**的一条证据面照片。**这一句话要写进样例的注释。**

**正控制不另建目录**：本仓**没有** `tests/ui_pass/` 目录、**也没有一处 `t.pass`**（实测，见
「关于本计划的代码块」一节）。**正控制由普通运行期用例承担**——Task 5 的四支用例本身就是正控制
（它们能编译即已说明正常调用路径成立）。

**原先拟列在本块照片清单里的第一条已移出，不重列**（设计 §11.4 末）：
`Operator { .., determinism: Deterministic }` 的构造不带 `backend_candidates` ⇒ 编不过——
那是 P1 的 `Operator` 的结构事实，不是本块造出来的照片，本块只是照它构造 `all_operators()`。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-image --test type_level
```

**这一步量到的是运行期红**（trybuild 报样例与 `.stderr` 不符）：样例与 `.stderr` 都在本 task 内，
`.stderr` 尚未取值。**取 `.stderr` 的方式**：跑一次，用 trybuild 打印出来的 `wip` 输出回填，
**逐份取、不许凭记忆写、不许三份共用一个错误码**（本仓已为此付过代价）。
**回填之后要再跑一次**，确认三份都按预期失败（而不是「因为拼错函数名而编译失败」）。

- [ ] **Step 3: 实现**

**无生产代码。** 三份样例与它们的 `.stderr`、加一条驱动，就是本 task 的全部产物。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-t8-test.log
git add crates/continuum-image Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "test(image): 三份编译失败样例——两张结构层的照片"
```

---

### Task 9: 收尾与复核

**Files:**
- Modify: `crates/continuum-image/**`（**仅在复核发现缺口时**）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**仅在对账不齐时**）

- [ ] **Step 1: 全量验证（门读数按纪律 3 读）**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p5f-final-test.log
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets 2>&1 | tee .tmp/p5f-final-build.log
```

预期：全绿、0 warning。**判据不是退出码**：逐条读日志里 **`Running` 的行数**与
**`test result:` 的行数**（Doc-tests 算一条），确认射程覆盖全部测试目标；
`--no-fail-fast` 必须带上（本仓的 `cargo test` 不收 `--keep-going`）。**管道结尾不许接 `tail`。**

- [ ] **Step 2: `ALLOWED` 与 `Cargo.toml` 的终态核对**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

**逐条核四件事**：
（a）`continuum-image` 的条目**恰为五项**——设计 §2.1 的四项 ＋ 本计划自定的 `continuum-persist`，
一个不多一个不少（多的那项的依据见 `## 遗留` 第一节第 1 条）；
（b）`crates/continuum-image/Cargo.toml` 与 `ALLOWED` **精确一致**（`[dependencies]` 与 `[dev-dependencies]`
里的**外部** crate 不进表，内部 crate 逐条对上）；
（c）**没有**指向 `continuum-core` / `continuum-events` / `continuum-policy` / `continuum-capability` /
`continuum-port` / `continuum-semantics` / `continuum-node` / `continuum-secrets` / `continuum-sandbox` /
`continuum-workspace` / `continuum-provider` / `continuum-connector` / `continuum-model-registry` /
`continuum-method` / `continuum-media` 的边；
（d）**本块没有动任何迁移**：`cargo test -p continuum-runtime --test migrations` 与 `--test startup`
仍全绿（本块零表，`runtime_migrations()` 一个字都没改）。
**并把粒度写清**：`--depth 1` 只钉**直接边**；传递可达在 Rust 下不可 `use`，
**故直接边即这条规则的完整粒度**，**不另设闭包断言**。**不要把这句读成「任何指向别层的边都会变红」**
——那会是一条假保证。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
grep -rn "continuum_core\|continuum_persist\|continuum_events\|continuum_policy\|continuum_capability\|continuum_port\|continuum_semantics\|continuum_node\|continuum_secrets\|continuum_sandbox\|continuum_workspace\|continuum_provider\|continuum_connector\|continuum_model_registry\|continuum_method\|continuum_media" crates/continuum-image/src
grep -rn "OperatorImpl\|propagate_invalidation\|\.bind(" crates/continuum-image/src
grep -rn "Evidence\|evidence_from_\|EvidenceType::" crates/continuum-image/src
grep -rn "BlobStore\|ContentHash\|content_hash\|serde_json" crates/continuum-image/src
grep -rn "SystemTime::now\|Instant::now" crates/continuum-image/src
grep -rn "Migration::new\|p1_artifact_migrations\|CREATE TABLE\|INSERT INTO\|UPDATE artifact" crates/continuum-image/src
grep -rn "Result<bool\|-> bool" crates/continuum-image/src
grep -rn "fn resolve_edit_region\|Result<EditMask" crates/continuum-image/src
```

**八条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；
**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：
（a）**十六个层外／别的块的 crate 零 `use`**——注意 `continuum_persist` 必须零命中
（它是 **dev 边**，只有 `tests/lineage.rs` 用它），这一条的语料限定在 `src/`；
（b）`OperatorImpl` **零实现**（本块不建执行体）、`propagate_invalidation` 零命中、
**`.bind(` 零命中**（本块无 `bind` 目标，设计 §8）——本块的两个入口名**必然出现**
（它们是交付物），判据是「**没有调用方**」而不是「名字不出现」；
（c）`Evidence` 与 `EvidenceType` 在生产面**零命中**（本块不写域包装、不构造证据，设计 §7.2）；
（d）**IO 面零命中**（`BlobStore` / `ContentHash` / `serde_json`——本块不落盘、不序列化，设计 §3.1）；
（e）系统时钟零命中（本块不取时钟）；
（f）**零迁移、零 DDL、零 DML**（`Migration::new` / `CREATE TABLE` / `INSERT INTO` / `UPDATE artifact`
全零命中——本块零表，且**不建第二份「不得覆盖原图」的判据**）；
（g）**没有一个返回 `bool` 的比较器**（§5.3 的第二半：`the_comparator_declares_no_verdict` 的源码面那一半）；
（h）**`resolve_edit_region` 是本 crate 唯一返回 `EditMask` 的函数**——这一条兑现 §4.4 的
「本 crate 里没有第二条从 `Point` 到 `EditMask` 的路」。**射程写清**：这是「结构事实」，不是承诺；
它只说本 crate，说不了「别的 crate 不会自己写一个 Point→整图的转换」。
**作用域写死**：这八条 grep 只扫 `src/`，**不含测试目录**（`tests/lineage.rs` 要用 `continuum-persist`
的类型、`tests/compile_fail/` 要用 `Evidence`）。

- [ ] **Step 4: 逐条核对设计 §2.2 的四条判定与 §1.1 的八行义务**

设计 §2.2 把「本块是它们唯一落点」的判定列为**四条**，§1.1 的「建什么」表列为**八行**。
**本计划的动作是逐行核「落点栏是否指向一条真实存在的用例」**，不重述那两张表：

| 设计 §2.2 的判定 | 本计划的用例 |
|---|---|
| 第 1 条（§325 的 MUST 的唯一落点） | Task 5 的 (d)–(h) 与 `a_semantic_object_…`、`a_box_reaching_outside_…` |
| 第 2 条（§326 的 mask 外像素**比较器**） | Task 6 的 (l)、(k) 的第④对、(k2) 的 `Raster` 侧 |
| 第 3 条（§326 的 mask 外像素**强制作**） | Task 6 的 (i) 与 (j) |
| 第 4 条（五枚算子的 `determinism` 声明） | Task 2 的 (a) 的 `determinism` 列、Task 7 的 (o) 与 (p) |

| 设计 §1.1 的具名义务 | 本计划的用例 |
|---|---|
| 两种原生操作**分作两枚**算子（不得合并） | Task 2 的 (a) 的手写五枚 id 清单（第 3、4 行各占一枚）＋ Task 2 的 `no_sixth_operator_is_registered` |
| 编辑区域的**四种表示** | Task 1 的四臂与四个载荷、Task 5 的四支 |
| **Point MUST 解析成语义区域** | Task 5 的 `f` / `g` / `h`；**判据缺口见 §4.3 与 `## 遗留`** |
| `outside_mask_change = FORBIDDEN` 的**强制作** | Task 6 的 (i) / (j)（强制作）＋ (k) / (k2) / (l)（检出的前置条件与量）；**类型层拦不住，见 §5.4** |
| 每次编辑产生新 Artifact 的**谱系** | Task 7 的 (m) / (n)（**P1 判据的重跑**） |
| **只产出 Evidence、只消费判定** | **本块的 crate 里无照片**——本块不写域包装、不调构造点（Task 8 的样例 3 拍的是「造不出来」） |
| 与 P5b 方法库的接缝 | **本块无 `bind` 目标**（设计 §8）：Task 9 Step 3 的 (b) 核 `.bind(` 零命中 |
| §343 的分级后果 | Task 2 的 (q) |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑**承重守卫**（两侧对钉的、fail-open 侧的、失败路径判别 `Err` 的、跨 crate 才可见的），
每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 → 还原后 `sha256sum` →
独立日志路径。**逐轮在报告里附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位
（取反／放宽／收紧／移除；改被断言的声明取值的变异记取反并写明改的是哪一列）」**。

**设计 §11.3 的预告表逐行照跑**，其中数行要按本计划的口径读（逐行在下文标明）：

- 「**取反**：`resolve_edit_region` 的 `Point` 支在 detector 空返回时合成一个单像素掩码」——
  该红 `g`，**不该红** `f`、`h`、`d`、`e`。
- 「**放宽**：`composite_masked` 改成原样返回 `generated`」——该红 `j`，**不该红** `i`、`k`、`k2`；
  **若 `j` 也不红**，按 Task 6 的注释记「缺用例」（夹具里没有 mask 外被改过的对照件）。
- 「**取反**：`ShapeMismatch` 的前置条件删掉」——该红 `k`，**不该红** `i`、`j`、`k2`。
- 「**放宽**：把 `generate-image` 的 `determinism` 改成 `Deterministic`」（**设计记作「收紧」，本计划
  记作「放宽」**，理由见 `## 遗留` 第一节第 5 条）——该红 `p` 的第一枚与 Task 2 的 (a) 的对应行，
  **不该红** `o`、`q`。
- 「**取反**：`register_image_operators` 改成边查重边写」——该红 `c2`，**不该红** `b`、`c`；
  **夹具的重复项必须排在批的末尾**，否则该档全绿（Task 4 的注释）。
- 「**等价变异**：把某个 backend 候选串改成另一个同样合法的串」——**设计记作「全绿」，本计划按
  「取反」读**（设计这一行与它自己的 §11.2 第 (a) 条相抵：那一条对 `backend_candidates` **逐枚断言、
  期望值手写**，故换串**会红那一枚的 (a) 行**）——该红 Task 2 的 (a) 的那一枚，**不该红** (r)。
  **本块真正的等价变异体是「`backend_candidates` 的两项换序」**（候选读作集合），它红不了任何一条。
- 「**放宽**：`all_operators()` 里少写一枚」——**编译失败**（返回类型是 `[Operator; 5]`），
  **这一档不是用例红，是编不过**（按纪律 1(c) 不上报为红）；
  **若连返回类型也改成 `[Operator; 4]`**，则**红位在本计划里是 Task 2 的 (a) 的对应行、手写的 id 清单断言
  与 (q) 的补集断言**——**这是对设计 §11.3 那一行的偏离**（设计预告的是 `b` 的对应枚与 `q`；
  而设计的 (b) 自己就是「以 `&all_operators()` 调注册、逐枚 `resolve`」，按设计 §11.3 那句
  「若改类型后仍不红，说明 b 是按 `all_operators()` 自己遍历的——即假照片」，**它本就不承重**）。
  **本计划把承重者放到手写的五枚 id 清单与 (q)，理由与收件人见 `## 遗留` 第一节第 10 条。**
  **若改类型后仍不红，说明那些断言是按 `all_operators()` 自己遍历的**——即假照片。

**本计划另加的两条必须跑的变异**（设计 §11.3 没有它们，而它们是本计划新增守卫的红来源）：
- **取反**：把 `resolve_edit_region` 的两处 `RegionUnresolved` 都写成 `region: Point`
  ——该红 `a_semantic_object_with_no_match_is_rejected_with_its_own_kind`，**不该红** `g`。
- **取反**：从 `verify-outside-mask` 的 `input_schema` 里删掉 `Blob`
  ——该红 Task 3 的**（A1）**（表里列出的 `Blob` 不在 `input_schema` 里了）；
  **不该红（A2）**（剩下的 `input_schema = {Image}` 仍**是**表的子集）、**也不该红**（B）
  （（B）读的是上游的 `output_schema`，本变异不动它）。

**三条跨 crate 的承重守卫必须跑全量**：
（i）`ALLOWED` 与实际依赖一致（加/删一条边即红）；
（ii）新 crate 进 workspace 后**迁移集合与计数不受影响**（本块零迁移，
`migrations` 与 `startup` 两个既有测试仍全绿）；
（iii）Task 7 的 (p) 的第一枚——它读的是 `continuum-graph` 的 `cache_key`
（**跨 crate 才可见**：本块只声明 `determinism`，判定在 P1）。
**第 (m)、(n) 两轮的靶子在 `continuum-artifact`**（Task 7 已写明）：
那两轮**只能在本地工作树上做、必须还原、不许提交**，报告里要写明这一点。

**等价变异体不许算作守卫**（跑它们是浪费，且会被误读成「守卫有效」）：
`backend_candidates` 的两项换序、`all_operators()` 的五行的次序（本计划的用例按 id 取行）。

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。
**本计划不新建、也不修改第二份文档**：若要并进 `docs/superpowers/*-followups.md`，由**协调者**做。

- [ ] **Step 7: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs(image): P5f 图像领域算子的收尾与复核"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**

### 一、设计问题：**本计划查出的相抵与缺口**（发现即报，本计划一处都不替设计补写）

1. **§11.2 的第 (m)、(n) 两条要求 `continuum-persist`，而 §2.1 明写「不登记 `continuum-persist`」（相抵）**：
   （m）与（n）要经 `Tx` 调 `save_artifact` / `load_artifact`，而 `Tx` 与 `Db` 只能从
   `continuum-persist` 取（实测 `crates/continuum-artifact/src/lib.rs` 只重导出
   `save_artifact` / `load_artifact` 等函数，**不重导出 `Db` / `Tx`**），
   故**没有一条不命名 `continuum_persist` 的写法**。**两处只能有一真。**
   **本计划的取舍**：加一条 **`continuum-persist` 的 dev 边**（Task 7），
   `ALLOWED` 的条因此是五项而不是设计给的四项。
   **理由**：(i) 边别按「谁在用」判——生产代码对它零调用（Task 9 Step 3 的 (a) 条核过），
   唯一使用点是 `tests/lineage.rs`，与设计 §2.1 给 `continuum-verify` 的处置同形；
   (ii) §2.1 那句「不碰 `Tx` / `Migration`」说的是**生产面**（它给的理由是「本块零表」），
   而（m）、（n）是**用例**。
   **代价**：`ALLOWED` 的条目与设计 §2.1 的边表差一项，读者须读本节才知道为什么。
   **翻转条件**：若协调者判「§2.1 的边表一字不改」，则本计划删去 Task 7 的 `tests/lineage.rs`
   与这一项，**并同时记「§11.2 的第 (m)、(n) 两条在本块没有落点」**——那两条照片随之只剩
   「§327 的判据已在 P1 落地、本块不建第二份」这一句声明。
   **收件人：设计作者 ＋ 协调者。**
2. **§11.2 的 (d)–(h) 五条照片缺 `SemanticObject` 那一支**（**枚举行缺一条臂**）：
   §4.2 的表列了**四支**、§3.1 的 `RegionUnresolved` 注又逐字写着它也覆盖它
   （「Point 点空处、`SemanticObject` 名不副实」），而 §11.2 的第 (d)–(h) 只覆盖
   `Box`（d）、`Mask`（e）、`Point`（f、g、h）——**枚举到的那几支之外，第四支是 fail-open**
   （本仓「枚举式断言的通病」）。**本计划的处置**：Task 5 补一条
   `a_semantic_object_with_no_match_is_rejected_with_its_own_kind`（连同它自己的红条件）。
   **收件人：设计作者。**
3. **`RegionDetector` 的方法集没有形状，且与 §2.1 的「`ArtifactId` 是本 crate 签名里唯一的用点」相抵**：
   §2.2 只说「本块定义它，其实现在 backend 一侧」，§4.2 只说「`detector` 的返回值就是掩码位图」——
   **没有给方法名、参数与返回类型**。
   而 §2.1 的表逐字写着 `ArtifactId`「（§4.2 签名里的 `source` 实参——**这是本 crate 签名里唯一的用点**）」。
   **两读各有一处不成立**：若 detector 的方法**不收** `source`，则 `resolve_edit_region` 的 `source`
   实参在函数体内**没有用途**（一个死形参，而本仓判「一条永远进不去的臂与一个死字段同类」）；
   若**收**，则「唯一的用点」这句话不成立（两处）。
   **本计划的取舍**：detector 的两个方法**都收** `source`（要能检测「哪一张图上的区域」，它必须有这张身份）；
   §2.1 那一句的「唯一」应收窄为「§4.2 的 `source` 实参 ＋ `RegionDetector` 的两个方法」。
   **代价**：`RegionDetector` 的方法集（名字、参数、`Option<EditMask>` 的返回）**是本计划自定的取值**。
   **翻转条件**：若协调者判 `source` 应从 detector 的方法里删去，则 `resolve_edit_region` 的 `source`
   实参同时成为死形参，§4.2 的签名与 §2.1 那一句都要改。
   **收件人：设计作者 ＋ 复审者。**
4. **§4.2 没有「归一化坐标越界」的处置，而 `ImageError` 的四臂里没有一枚能承载它**：
   §163 `:648-649` 给了 `x, y ∈ [0, 1]` 的约定，`Box` 支的投影要对越界输入给出定义；
   而 `RegionUnresolved` 说的是「解析不出区域」、`ShapeMismatch` 说的是「两载体形状不同」——
   **两者都表达不了「坐标越界」**，且臂数是设计 §3.1 钉死的四臂。
   **本计划的取舍**：`Box` 支**夹取**到画布（Task 5 的一条用例与它的红条件）。
   **代价**：越界输入被静默地当成「贴边」；**收益**：不新增一枚设计没有的臂，也不 panic。
   **翻转条件**：若协调者判「越界应当被拒」，则须给 `ImageError` 加第五臂（或把越界并入 `RegionUnresolved`
   的语义），两处都超出本块的范围。
   **收件人：设计作者 ＋ 规范维护者。**
5. **同一形状的变异在 P5c／P5d／P5e／P5f 四份同层设计里被记成了三个不同的档位名**（**口径不一**）：
   同一个变异——**把某枚算子的 `determinism` 改成 `Deterministic`**——P5c 的预告表记「**放宽**」，
   P5d 与 P5e 各记「**取反**」，P5f（**本块设计**）记「**收紧**」。
   **本计划的判据是「放宽」**：`Deterministic` 让 `cache_key` 由 `None` 变 `Some`，
   即**放宽** §305 的复用面（§305 要求 `operator_version` 未变等四条件，`Deterministic` 使其更常满足）。
   故 **P5c 那一份对，P5d／P5e／P5f 三份错**——**那三份要改的是各自预告（变异预告）表里
   `determinism` 那一行的档位名**（按内容指：P5d 的 §9.3、P5e 的 §12.3、P5f 的 §11.3；
   那一侧的改动不属本计划，由协调者另行处置）。
   **本计划按「放宽」统一记**（纪律 1 的宽紧口径），**并在此处留下设计那一行的来历**，
   免得复审按「收紧」去找一条不存在的症状。
   **动作不变**（该红的用例与不该红的用例，两处写的是同一组），**变的只是档位名**。
   **收件人：设计作者（P5d／P5e／P5f 三份）＋ 复审者 ＋ 协调者**（这一处口径跨四份设计）。
6. **§11.2 第 (s) 条只给了一个方向**：它写「下游 `input_schema` 的每一型都能在该行列出的上游算子里找到，
   且那一枚的 `output_schema` 含该型（交集中的型，不是「两侧相等」）」——**下游那一侧只判了
   「下游 → 表」这一个方向**，**表中列了而下游没有的型无人过问**（那一个方向的取值是 fail-open 的）。
   **本计划的处置**：Task 3 把下游那一半拆成两条独立断言——（A1）表 → 下游、（A2）下游 → 表，
   合起来是集合相等。**这是一处计划侧的加强**，理由与同层另一份计划的同形处置一致。
   **收件人：设计作者。**
7. **§165 的名字 `preserve_outside_mask = REQUIRED` 在设计的节点参数字段清单里没有落点**（**实测**）：
   规范里同一条约束有**三个名字**——§165 `:714` 的 `preserve_outside_mask = REQUIRED`
   （实测该名在 `docs/spec/03-product-drive.md:714`；`:711` 是那一节的首行「局部修改最重要的约束应该是：」，不是锚点）、
   §324 `:2343` 的 `preserve_outside_region = true`（算子的入参字段）、
   §326 `:2372` 的 `outside_mask_change = FORBIDDEN`（默认值）。
   而设计 §13 第 11 条那张「五个非制品字段」的清单是从 §323 / §324 **推出来的**
   （`prompt` / `dimensions` / `constraints` / `instruction` / `preserve_outside_region`），
   **§165 的那个名字不在清单里**；§5.4 用的也是 §324 的名字作触发条件。
   **本块不认领任何一枚的权威**（节点参数的字段约定未给，§13 第 11 条），
   **本块不为它们写解析或校验**（设计 §3.2 末的边界）。
   **收件人：规范维护者 ＋ 设计作者。**
8. **§4.1 给 §163 的锚类型改了名，而改名没有留痕**（小项）：规范原文那个类型叫
   `ImageEditAnchor { x, y }`（`docs/spec/03-product-drive.md:639`），设计 §4.1 叫 `NormalizedAnchor`，
   而那一处的注释说的是「两字段按 §163 `:648-649` 取 `x, y ∈ [0, 1]`（**这一条有规范出处**）」
   ——**那一句讲的是取值约定有出处，不是名字有出处**。**本计划照设计的名字写，并在此记下规范的名字。**
   **收件人：设计作者。**
9. **`resolve_edit_region` 不核 detector 返回的掩码与 `source_dims` 是否一致**（**没有前置**）：
   §4.2 的表给了四支的解析方式，**一条前置条件都没有**；故一个尺寸不符的掩码会 `Ok` 通过区域解析，
   直到 `composite_masked` 才被 `ShapeMismatch` 拦住。
   **这不是静默失效**（下游会拦），但**「区域解析成功」与「掩码可用于合成」是两件事**，
   而只有前者在本函数的返回类型里。**本计划不就地补前置**（补它就要一枚设计没有的臂，或改签名），
   只把这一条写进函数的文档注释（Task 5 的 `the_mask_arm_…` 与 Step 3）。
   **收件人：设计作者。**
10. **§11.3 预告「`all_operators()` 少写一枚＋返回类型同步改小」时红的位是 (b)，而那一枚在本设计自己的
    判据下不承重**（**这是一处本计划对设计预告的偏离**，不是设计缺口）：
    §11.3 那一行写「若连返回类型也改成 `[Operator; 4]`，则 **b 的对应枚**与 q 的补集断言红」，
    而 §11.2 第 (b) 条自己的写法是「以 `&all_operators()` 调 `register_image_operators`、逐枚 `resolve`」
    —— 它**从 `all_operators()` 取语料**，正是 §11.3 那一行末句警告的假照片（「若改类型后仍不红，
    说明 b 是按 `all_operators()` 自己遍历的」）。
    **本计划的取舍**：承重者放到 Task 2 的**手写五枚 id 清单**（`the_ids_are_pairwise_distinct_…`）、
    §11.2 第 (a) 的对应行与第 (q) 的补集断言（逐枚），**不把 (b) 当那一档的红位**。
    **代价**：与设计的预告表有一处不一致，读者须读本节才知道为什么。
    **翻转条件**：若设计作者坚持 (b) 承重，则须把 (b) 的语料改成手写的五枚 id（那会与 §11.2 的
    「(b) 的期望清单必须手写」那条预防合流），本计划的 Task 4 与 Task 2 随之调整。
    **收件人：设计作者 ＋ 复审者。**
11. **§5.3 的「总像素数」没有指明是整图的还是 mask 外的**（**读数未定**）：
    §5.3 逐字只写「返回 mask 外**不相等**的像素数与**总像素数**」（实测在设计 `:510`）——两种读法都通；
    `docs/spec/03-product-drive.md:832` 的 §168 只给了 `outside-mask pixel difference` 这个检查名，
    没有给这两个量的定义。**本计划取「mask 外的像素数」**（理由与夹具要求见 Task 6 的
    `the_comparator_reports_the_difference_and_the_total`，并已入第五节）。
    **代价**：若设计作者取的是另一种读法，照本计划写出的用例会把那一版实现判红。
    **翻转条件**：若判「整图的像素数」，则 Task 6 那一条的总数期望值与「把总数写成整图的像素数」那条红条件
    对调，动作不变。
    **收件人：设计作者 ＋ 复审者。**
12. **§11.3 的「等价变异」那一行（backend 候选串）与设计自己的 §11.2 第 (a) 条相抵**：
    §11.3 把「把某个 backend 候选串改成另一个同样合法的串」预告为**全绿**，而 §11.2 第 (a) 条是
    「端口两列、`determinism`、`side_effect_class`、backend 候选**逐枚**断言（期望值手写）」
    —— 期望值既然手写，换串**必然红那一枚**，故那一行**不是等价变异体**。
    **本计划按「取反」记**（纪律 1(b) 的「先排除一处易误判的」段与 Task 2 的用例注释）。
    **动作不变**；**变的只是那一行的档位名**。
    **收件人：设计作者 ＋ 复审者。**

### 二、设计里的「待对账 / 待裁」标记：**本计划不重述其内容**

设计 §12 的每一条都以「待与某方对账」开头并给了具名收件人，§13 的每一条都以「缺的是哪一步」写到底。
**重述就是第二份转录，正是本项目出错最多之处。** 本计划的动作只有三件：
（i）**把本块是消费方或落点方的那些搬进「跨计划前置」第三节**（授权旗标、§325 的五条、
`preserve_outside_region` 的一致性、五个非制品字段、`provenance` 与掩码编码、§168 第一项检查、
`version` 的计数、backend 名单、`image/` 目录、迁移档）；
（ii）**把设计已自陈的两条照原文留在设计里**——§12 第 3 条 (ii)（域包装该不该存在）
**已由协调者 2026-10-10 的裁定关闭**（四块一律不写、一律直接调 `from_tool_result`；口径写在
P5a 设计 §4.4），本块的落点是 Task 9 Step 3 的 (c) 条与 Task 8 的样例 3；§13 第 9 条
（`Benchmark` 臂在本块没有用处）**不是缺口，是据实记录**，**本计划不引它**；
（iii）**其余各条照原文留在设计里**，收件人不变。

### 三、对既有之物的请求（**本计划一处都不实施**，逐条见「跨计划前置」第三节）

- **`ArtifactType::Image`**：见「跨计划前置」第一节与第二节。**本块是消费方**。**收件人：P5e。**
- **`continuum-verify` 的 `Evidence`**：见同上。**本块用它的唯一处是 Task 8 的样例 3。**
  **收件人：P5a。**
- **§187 是否补 `image/` 目录**（或 image 的方法并入既有某份目录）：**本块不发明目录、不借 `3d/` 域**。
  若协调者裁决补，`MethodDomain` 加一臂与 `seeded()` 加条目**都归 P5b**（切分 §四第 3 条），
  本块经 `bind` 填入的内容是**本域算子的 id**，而**方法名本身属 §187 的名单、不由本块编**。
  **缺的是这一步的裁定。收件人：协调者 ＋ P5b。**
- **迁移档的合并**：本块**零迁移**（实测 `130`–`199` 在装配链里全空），故没有号可占。
  设计与本块的一致：**本块不自取档号**（设计只给了一条建议 `180`，而它是建议不是裁定）。
  **收件人：协调者。**
- **`continuum-persist` 的 dev 边**：见第一节第 1 条。**收件人：设计作者 ＋ 协调者。**

### 四、拍不到照片的地方（设计 §11.5 已列，**逐条照原文，本计划不发明**）；本计划补四条

设计 §11.5 的五条照原文：**端到端**（执行器未建，切分 §四第 4 条禁止 P5 自建执行路径；
本块的全部照片是**单元与集成层**）；**§326 的「强制作一定被用上」**（谁把 `hard-composite` 接在
`local-generative-edit` 之后是装配方的图构造，本块拍不到——§5.4 的失配方向正是这一侧）；
**真实 detector 的区域质量**（§4.3 的五条判据缺失）；**`Alpha / 色彩空间是否异常`**（像素格式未给，
只拍得到「它在返回值里有一项」）；**§305 的复用真的省下了一次重算**（`can_reuse` 今天无生产调用方，
且 `CacheKey` 无存储）。

**本计划补四条**：
- **§4.3 表里那五条判据缺口**：拍得到的只有「桩的返回被原样传出去」与「空返回即 `Err`」两条，
  **拍不到「区域选得对不对」**（Task 5 的 `g` 与 `a_semantic_object_…` 的注释已各写一遍）。
- **§11.2 第 (m)、(n) 两条照片的靶子不在本块的交付物里**：本块的 crate 里没有产出 Artifact 的代码
  （不实现 `OperatorImpl`），故两条都是 **P1 写读路径的重跑**——它们的变异靶子在
  `crates/continuum-artifact/src/persist.rs`。**证据强度较低**，报告里要标明，
  并写明它们**钉不到**「本域的算子产出时确实这么填」。
- **本块的三个入口（`all_operators` / `register_image_operators`）与两个判定函数
  （`resolve_edit_region` / `composite_masked`）的生产调用方**今天不存在（第 3 层未建），
  故「它们被谁调用」无照片——**这不是本块的缺口**（设计 §13 第 10 条）。
- **「别的 crate 不会自己写一个 Point→整图的转换」**：Task 9 Step 3 的 (h) 只拍得到本 crate 内的结构事实，
  **拍不到别的 crate**。**这不是缺口，是那句话本来就有的射程**（设计 §4.4 第二半）。

### 五、本计划自定的形状与取值（**申报**，逐条说清代价）

- **`EditRegionKind` 的四个编码串**（Task 1）：设计 §11.1 只说「`as_str` / `parse` 两枚穷尽 `match`
  ＋ `ALL` 常量，形状照 `crates/continuum-artifact/src/artifact.rs:28` / `:47` / `:68` 的既有做法」，
  **没给四个串**。本计划取 `point` / `box` / `mask` / `semantic_object`（小写、多词以 `_` 连接，
  照本仓的编码纪律与 `ArtifactType` 的既有取值）。
- **`an_alias_from_166_is_not_in_the_encoding` 是本计划新增的一条守卫**（Task 1）：
  §166 的另四个名字不进 `parse`。设计没有要求这一条，**它钉的是「一型两名不可表达」**。
  **代价**：比设计多一条用例；**收益**：§4.1 的两组名字之间的关系在编码面上有可跑的形态。
- **`EditRegion::kind()` 的读法**（Task 1）：设计要求 `RegionUnresolved { region }` 表达得出是哪一型，
  **没给读法的名字**。**代价**：名字是本计划定的；**收益**：那张诊断有一个可被用例指认的落点。
- **`NormalizedBox` 的字段名 `p` / `q`**（Task 1）：设计说「它的两个角」，**没给名字**。
  取不作方向暗示的两个名字，是为了不发明「哪一角是左上」这条规范没给的约定。
- **`SemanticObjectRef` 定成不透明串**（Task 1）：设计 §4.1 说「不给它定形状」，
  而代码里一个变体的载荷必须有型。**代价与边界写明在 Task 1 的用例注释与类型的文档注释里**：
  不透明串**不填** §4.3 表第 5 行那条缺口。
- **`RegionDetector` 的方法集**（Task 5）：见第一节第 3 条。**代价**：方法名、参数与返回类型
  （`Option<EditMask>`）都是本计划定的；**收益**：`resolve_edit_region` 的 `source` 实参有用途，
  且「零检出 ⇒ `None` ⇒ `Err`」这条链在类型上只有一处载体。
- **`Box` 支的夹取**（Task 5）：见第一节第 4 条。
- **`PixelDiff` 的整体形状与「总数」那一枚的名字**（Task 6）：设计约束了「含一个可判 `== 0` 的差异量」
  与「返回 mask 外不相等的像素数与总像素数」两句，但也**已在 §11.2 第 (l) 条的用例里逐字给出
  `differing` 这个字段名**（实测在设计 `:520`）——故**自定的不是 `differing`**，而是总数那一枚的名字
  与整体形状（例如是否把总数与差异量放进同一枚结构体）。**`Raster` 的四个字段名**同样：设计约束了
  「宽、高、通道数、字节序列」四件，**没给字段名**。
- **`PixelDiff` 的「总像素数」取「mask 外的像素数」**（Task 6）：设计 §5.3 的那一句**没指明是整图还是
  mask 外**（两种读法都通，见第一节第 11 条）。**代价**：与设计作者若取另一种读法相比，本条的期望值
  与红条件会相反；**收益**：分子（mask 外不相等的像素数）与分母同域。**收件人见第一节第 11 条。**
- **§3.4 边表的判据拆成两组断言（（A1）＋（A2）＋（B））**（Task 3）：见第一节第 6 条。
  **代价**：多一份手写表；**收益**：**表中列了而下游的 `input_schema` 里没有的型**这一侧不再是 fail-open
  （设计 §11.2 第 (s) 条只判了「下游 → 表」那一个方向，见表→下游一侧在（A1）里补上）。
- **Task 8 排在最后、Task 2 紧跟 Task 1**：**不是风格偏好**，是「跨计划前置」第二节那两条硬前置的
  直接后果——Task 2 要 P5e 的变体，Task 8 要 P5a 的 crate。
- **`all_operators()` 的返回类型是 `[Operator; 5]`**：**不是本计划自定**——设计 §3.1 已写死并给了两条理由。
  留这一条是为了让「数目写在返回类型里」这件事可查，并附上它的**副作用**：
  **本块不写 `len() == 5`**（设计 §11.1 明写「不补」）——数目的运行期照片由
  Task 2 的**手写五枚 id 清单**承担，它与「逐枚断言」共用同一份清单，两处维护同一件事会更早漂。
- **`four_kinds_of_dirty_input_…` 式的「枚举到的那几支之外是 fail-open」**：本计划在两处各补了一条
  （`SemanticObject` 那一支、§3.4 的（A1）），两处都记在第一节（第 2、6 条）。

### 六、设计 §12 的逐条对账条目与 §13 的遗留：**本计划不重述其内容**

见第二节。**本计划的动作只有三件**，已在第二节逐条列出；**其余各条照原文留在设计里，收件人不变。**

---

## 交付给谁

- **交给 P5e**：`ArtifactType::Image` 的落地（本块只消费；Task 2 的硬前置）。
  **本块向 P5e 报出的变体清单为空**——判据实测：五枚算子的端口只用 `Image`（P5e 的清单承接）
  与 `Blob`（P1 既有），由 Task 2 的 `the_port_types_in_use_…` 与 Task 3 的（A）钉住。
- **交给 P5a 的 `continuum-verify`**：本块**不写域包装**、不提供证据面那一层的函数
  （设计 §7.2，协调者 2026-10-10 的四块统一口径）；本块对它的全部用处是 Task 8 的样例 3
  （拍「在本 crate 里造不出 `Evidence`」）。**答复它设计 §14 第 3 条的询问：够用、不请求新臂**
  （`VisualCheck` ＋ `MetadataCheck`；`Benchmark` 本块不用）。
- **交给 P5b 的 `continuum-method`**：**本块无 `bind` 目标**（§187 的四个目录里没有图像那一档）。
  若协调者裁决补一个 `image/` 目录，本块经 `bind` 填入的内容是**本域算子的 id**
  （`all_operators()` 里的 id），而**方法名本身属 §187 的名单**。
- **交给第 3 层的执行器与装配方（未建）**：五枚算子的 `OperatorImpl`、`Queued → Running` 处的
  `resolve` 调用点、`hard-composite` 的插入点、`resolve_edit_region` 的调用时机、
  以及本块两个入口的生产调用方，**五处**；以及 `preserve_outside_region = true`
  与下游 `hard-composite` 的一致性校验（§5.4 的 fail-open 那一侧）。
- **交给装配方**：`resolve_edit_region` 的两个实参（源制品 id 与源图尺寸）从哪一处取——
  §323/§324 的五个非制品字段住在节点参数上，而节点参数的字段约定未给。
- **交给规范维护者（本项目无此角色）**：§325 的五条判据、节点参数的字段约定、
  `Artifact.provenance` / `metadata` 的字段约定、掩码的编码格式、像素格式与色彩空间、
  §168 第一项检查的判据与持有方、§240 的 `version` 与 §327 的 `v1/v2/v3` 是否同一计数、
  §34 是否有图像生成的旗标，以及 §165／§324／§326 三个名字的统一。
- **交给协调者**：§187 是否补 `image/` 目录、迁移档的合并、`continuum-persist` 那条 dev 边的取舍、
  `determinism` 档位在四份同层设计里口径不一这一处的跨块裁定（第一节第 5 条），
  以及上面第一节列出的设计问题（逐条已给翻转条件或具名收件人）。
- **交给复审者**：本计划对设计的偏离（**逐条**）——第一节第 1 条（新增一条 dev 边）、
  第 4 条（`Box` 支的夹取）、第 10 条（`all_operators()` 少写一枚＋改返回类型的红位由设计的 `b`
  移到 Task 2 的 (a) 的对应行、手写 id 清单与 `q`）——**每条都写明了「为什么不是那样」**；
  以及第一节第 5 条与第 12 条的档位订正（设计与本计划的档位名相反，本计划按纪律 1 的口径记）。
  复审时请对 Task 7 Step 1、Task 5 的 `a_box_reaching_outside_…` 与 Task 9 Step 5 逐行核。
