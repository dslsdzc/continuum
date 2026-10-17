# P6A Context Compiler — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p6-scope-and-split.md` §五 P6A 行的原话，逐字照用）：

> 建「节点执行前必经」的装配通道：从 Conversation／Memory／Artifacts／Project／Tool results 取料，
> 按最小必要裁成 `ContextPackage`，并把「不存在绕过路径」做成可观察的强制面

**本块拥有 / 产出的接口**（同行的第二栏）：`ContextPackage` 的类型**一份**（§280 的六个字段名）；
装配入口 `compile`；`ContextPackage` 的消费契约。**消费** P6C 的 `MemoryClaim` 检索面；
`provenance[]` / `trust_labels[]` 两个字段的元素类型**引用** P6B 的类型（**不定义**，切分 §2.3）。

**规范依据**：§279 §280 §69 §70；《工程》§6.1（`docs/02-工程.md:365`）、§6.2（`:378`）、
§6.3（`:391`）、§6.4 判据 1（`:402`）、§9.4（`:635`）；§281 只在本块**引用 P6B 的类型**处出现
（本块**不判** §281，判据 2 归 P6B）。

**本文件只写 P6A。** 其余三块（P6B 七个信任标签、P6C Memory、P6D Protected Core／Learning）各有其设计，
本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §7 与 §11 写成一条
**具名的对账条目**，不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（判据与订正原文见 `docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p6-plan`，2026-10-17）里当场
`grep -n` / `sed -n` 实测的，未照抄任何一份别的文档（含切分文档）。切分文档里已漂移的行号在 §12 逐条记出。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| `ContextPackage` 的形状（§280 六字段） | 切分 §五 P6A 行；§280 `:1522-1536` | §4 | 全量（字段名逐字照抄；**元素类型规范未给**，本设计定，§2.4 第 1 条） |
| 「节点执行前必经」的装配通道（§279 MUST） | 切分 §五 P6A 行；§279 `:1500-1520`；《工程》§6.4 判据 1 `:402` | §5、§6 | 装配入口全量；**端到端强制面缺执行器**（§11 第 1 条） |
| 从五类来源取料 | §279 `:1507-1519`；§70 `:369-378` | §5.4 | 五类都有落点；Conversation／Project／Tool results 经 `references[]`（§2.4 第 3 条） |
| 按「最小必要」裁剪 | §279；§70 四目标 `:390-394`；《工程》§6.2 `:378` | §5.3 | 预算、去重、空串丢弃、容器隔离；**相关性判据规范未给**（§2.4 第 4 条） |
| 每片段承载来源与标签 | §281 `:1538-1554`；《工程》§6.2 `:380`；§9.4 `:635` | §4.2、§7.1 | **只承载**；标签类型与附加点归 P6B（切分 §五 P6B 行） |
| Agent 上下文隔离 | §69 `:322-355` | §5.5 | 「无隐藏来源」可测；**capability 判定不在本块**（§11 第 9 条） |

## 1.2 不建什么（摘要；完整表在 §9）

以下各条**不建、不留桩**，逐条给出归属：

| 不建的东西 | 归属 / 理由 |
|---|---|
| 执行器、节点状态迁移、调度、`Queued → Running` 的接线 | P1（切分 §四第 4 条与 §八.1 第 2 条；本块只提供 `compile` 与被消费的类型） |
| 模型调用、`InvokeRequest` 的构造、把 package 投影为 `Message` | P3 的模型调用路径（`crates/continuum-runtime/src/model_call.rs:373-379`）；投影规则规范未给（§6.2） |
| 七个信任标签的类型、附加点、拒绝行为 | P6B（类型与附加点）与 P2（拒绝，§6.2 `:380`） |
| `MemoryClaim` / `MemorySource` / 五个 scope / 检索存储 / 合成 / 纠正 | P6C（切分 §五 P6C 行） |
| Protected Core / Runtime Learning | P6D（切分 §五 P6D 行） |
| tokenizer、token 计数、相关性打分、embedding | 规范未给；本块不发明（§2.4 第 4、5 条） |
| 表、迁移、事件 | 本块**零表**（§10） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：**`continuum-context`**。

**名字的四条理由**（切分 §1.2 要求「四块之间不得出现两枚同义 crate」，§四第 3 条要求不与既有词汇争名）：

1. **`context` 是 §279／§280 的唯一词根**，不另造第二套词汇（§279 的标题是 `Context Compiler`，
   §280 的类型名是 `ContextPackage`）。
2. **不含 `compiler`**：切分 §1.1 逐字写明「Context Compiler」在本仓同时被用作**整个 P6 阶段的简称**
   与 **P6A 那一个组件的名字**，并裁定「本文此后只用后一种含义」。crate 名取 `continuum-context`
   而非 `continuum-context-compiler`，避免它被读成「整个认知层」或与阶段简称混淆；文档里的
   「Context Compiler」只指 P6A 这一枚组件。
3. **不与其余三块同义**：`trust`／`provenance` 留给 P6B，`memory` 留给 P6C，`protected`／`learning`
   留给 P6D。`continuum-context` 不占用这三个词。
4. **实测未占用**：`grep -rn "continuum-context\|continuum_context" crates/ Cargo.toml` 在本仓**零命中**
   （2026-10-17 实测；命中只在切分文档的 crate 名建议里，不在代码里）。

依赖边表（**只登记实际用到的**，切分 §四第 5 条）：

| 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|
| `continuum-artifact` | `ArtifactId`（`artifacts[]` 的元素类型；`crates/continuum-artifact/src/artifact.rs:195` 的 `ArtifactId`） | 普通 |
| `continuum-core` | `Message`（`crates/continuum-core/src/model.rs:38-41`，`Reference::Conversation` 的载荷）与 `ToolResult`（`crates/continuum-core/src/tool.rs:26-30`，`Reference::ToolOutput` 的载荷） | 普通 |
| `continuum-memory`（P6C 的 crate，已冻结） | `MemoryClaim`（`memories[]` 的元素）、`MemoryError`（检索失败的透传）、`MemoryRetrieval`（检索面 trait）、`RetrievalQuery` / `MemoryScope`（§5.3 第 5 条构造查询） | 普通 |
| `continuum-trust`（P6B 的 crate，已冻结） | `ContextProvenance`（`provenance[]` 的元素与 `ContextCandidate` 的字段）、`ContextTrust`（`trust_labels[]` 的元素）、`label(&ContextProvenance) -> ContextTrust`（附加点）、`ContextProvenance::memory(claim.id().as_str())`（本块为记忆片段构造 provenance 的唯一调用点）、`user_instruction()`／`conversation_history()`／`provider_supplied()`（候选的 `historical` 与 provider 来源）、`PackageShape`／`validate`／`TrustError`（J3 的独立复核面） | 普通 |

**不登记**的边，逐条给理由（零使用的边即假边）：

- **不登记 `continuum-graph`**：本块不构造 `NodeContext`、不读 `NodeId`、不调 `OperatorImpl::execute`。
  `ArtifactRef`（`crates/continuum-graph/src/execution.rs:36-40`）是执行层的类型；本块的 `artifacts[]`
  只存 `ArtifactId`，不复制「id + content_hash」这一对（§4.1 第 4 条）。**这条拒绝是刻意的**：
  `continuum-graph` 属执行层，本块属认知层，§9.1 的图里**没有** `执行层 (3) → 认知层 (6)` 这条边
  （只有 `执行层 → 资源层 → 认知层` 的传递路径，`docs/02-工程.md:573`、`:578`）；再加一条直接边
  就是 §八.2 第 1 条点名的那处相抵的第三种写法。
- **不登记 `continuum-operator` / `continuum-provider` / `continuum-runtime`**：本块不注册算子、不调
  Provider、不装配执行路径。`continuum-runtime` 是**装配方**（它依赖本块，不是本块依赖它）。
- **不登记 `continuum-semantics`**：本块不构造 `Intent` / `TaskContract` / `RequirementId`。§70 的
  `Intent` 以**文本**由调用方投影进 `instructions` / `references`（§5.4 末）。§9.1 里也没有
  `语义层 (2) → 认知层 (6)` 这条边。
- **不登记 `continuum-persist` / `continuum-events`**：本块零表、不写事件（§10）。
- **不登记 `continuum-policy` / `continuum-capability` / `continuum-secrets`**：本块不查策略引擎、
  不取 Capability Token、不判隐私等级。§69 的 capability 判定位置未定（C4／OPEN-016，§11 第 9 条）。
- **不登记 P5 的六个 crate**（`continuum-code` / `continuum-media` / `continuum-research` /
  `continuum-image` / `continuum-verify` / `continuum-method`）：它们产出的 `Artifact` 经
  `ArtifactId` 进入本块，不需要类型边；P5a 的 `Evidence` **不进** `ContextPackage`（§7.6）。

**dev 边：零条。** 本块的 `trybuild` 用例只引用本 crate 自身，不需要别的 workspace crate；测试里的
`MemoryRetrieval` 假实现由本 crate 自己写（trait 在 P6C，假实现不依赖 P6C 的存储）。

**外部 dev 依赖（不是 workspace 边，不进 `ALLOWED`）**：

| 外部 crate | 用途 | 形态 |
|---|---|---|
| `trybuild` | J1-a／J1-b／J1-c 的三条 `compile_fail` 样例 | `[dev-dependencies]`（workspace 已有） |
| `serde_json` | J1-c 的 `serde_json::from_str::<ContextPackage>` 编译失败样例 | `[dev-dependencies]`（workspace 已有） |

**这两条不是「dev 边」**：`ALLOWED` 表只断言 workspace 成员之间的依赖；外部 crate 不进那张表（同
`crates/continuum-runtime/tests/dependency_direction.rs:382-383` 的注释口径：「本表断言的是工作区成员
之间的边」）。

**共写文件**（切分 §一 的共写文件表给的是三处；下表是本块的那部分，**两行**）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `crates/continuum-context` | 本块**只加自己这一行** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-context", &["continuum-artifact", "continuum-core", "continuum-memory", "continuum-trust"])`——**四项**，与上表四行逐行对应；数组按字母序；两个实际名都已冻结（`continuum-memory`、`continuum-trust`） | 新 crate 会先让该门变红再被补齐（`every_crate_depends_only_on_its_allowed_set`，实测 `:478`；切分引的 `:288` 已漂），这是刻意的 |

**本块不碰** `crates/continuum-artifact/src/artifact.rs`、`crates/continuum-graph/src/node.rs`、
`crates/continuum-operator/src/definition.rs`（P5e／P5a 的落点），也**不在** `runtime_migrations()`
（`crates/continuum-runtime/src/main.rs:110`）里加行——本块零表。

**为什么本块要依赖 `continuum-artifact` 与 `continuum-core` 而不是把这两枚类型复制进来**：`ArtifactId`
是制品身份的唯一类型（§241 的提交口径），`Message` / `ToolResult` 是 §315／§316 的接口类型
（切分 §2.2：P6 一个都不碰、不借用它们的名字）。复制一份就是「同一件事两个词汇表」，
在本项目一贯判为 Critical。

## 2.2 冻结清单：本块持有的类型与判定

**类型**（定义在 `continuum-context`）：

```
ContextPackage        —— §280 的六字段，字段私有，唯一构造点是 compile（§4.3）
ContextRequest        —— 五类来源的候选集合（§5.2）
ContextCandidate<T>   —— 一枚候选片段：内容 + P6B 的 provenance（§5.2）
ContextBudget         —— 三个条数上限（§5.2）
Reference             —— references[] 的元素：Text / Conversation / ToolOutput（§4.5）
```

**不新增的类型**：不定义 `ContextProvenance` / `ContextTrust`（P6B 的；**包括 `UserInstruction { historical }`
变体与 `user_instruction()`／`conversation_history()`／`provider_supplied()` 三个构造入口**）、
`MemoryClaim` / `MemoryScope` / `MemoryError`（P6C 的）、`ContextError`（本块没有自己的判定失败，§5.6）、
`NodeContext` / `ArtifactRef`（P1 的）、`Message` / `ToolResult`（§315／§316 的）。

**判定**（本块是它们的唯一落点）：

1. `ContextPackage` 六字段的**元素类型**与**对齐规则**（§4.1、§4.2）——规范只给字段名。
2. 装配入口的形状与选择规则（§5）——规范只说「最小必要」。
3. 「唯一构造点」的类型层落法（§4.3）——§6.4 判据 1 的类型层兑现。
4. 五来源到四字段的映射（§5.4）——§280 没有 conversation／project／tool_results 字段；**历史用户轮只进
   `references[]`、当前指令只进 `instructions[]`**的数组选择也在这里（P6B §3.6 I4/I5）。

**本块不持有的判定**：七个标签的取值与附加（P6B）；「外部内容与用户指令不同级」的判定（P6B，
§6.4 判据 2）；拒绝行为（P2，§6.2 `:380`）；claim 的 scope、有效性、检索与合成（P6C）；
capability 判定（未定，§11 第 9 条）；节点执行与调度（P1）；模型调用与失败分类（P3）。

## 2.3 本块的层间位置

§9.1（`docs/02-工程.md:569-586` 的图，读法在 `:561-567`，`:588` 逐字「依赖方向单向，无环。」）
里与本块有关的三条，按「被依赖者 → 依赖者」读：

- **`continuum-artifact` 的层归属在两处层表里不一致**：README `:26-31` 把它列在**执行层 (3)**；
  《工程》§4 的标题（`docs/02-工程.md:352`「资源：模型、工具、算力、Artifact」）与 §9.1 的
  `资源层 (4) → 认知层 (6)`（`:578`）把它读作资源层。**本块不裁定层表**（P6C §12.2 第 1 条已报出）：
  若按 README，`continuum-context → continuum-artifact` 是 **3→6 的直接边**，而 §9.1 的图里只有
  `3→4→6` 的传递；若按 §4 的标题，它是 4→6 的实例。**两种读法下本块都不让 P6 成为既有 crate 的
  上游**（依赖方向是 P6A → artifact）；`ALLOWED` 只登记实际依赖。收件人：协调者 ＋ P6C
  （§11 第 22 条）。
- **`边界层 (5) → 认知层 (6)`**（`:581`）：P6B／P6C 的 crate 对边界层的依赖属这一条；本块**不接**
  这一条——本块不依赖任何边界层 crate（§2.1 的「不登记」）。
- **`认知层 (6) → 长期循环 (7)`**（`:583`）：本块**不接**——`ContextPackage` 是节点执行期的值，
  不落长期循环的表（§10）。

**§9.4 那一处相抵，本块的口径**：§9.4（`:635`）把「认知层 → 执行层 Context Package 的 provenance
与 trust_labels 字段」列为一条跨层载荷。按 §9.1 的箭头读法（`:563`），这条读作「执行层依赖认知层」，
而 §9.1 的图里没有这条边，且它与「单向、无环」相抵（切分 §八.2 第 1 条）。**本设计不裁定哪一种读法**，
只把本块的可执行口径写死为两条：

1. **本块不登记任何指向执行层 crate 的代码边**（不依赖 `continuum-graph` / `continuum-operator` /
   `continuum-provider` / `continuum-runtime`）。故无论 §9.4 读作哪一侧，本块都不新增反向边。
2. **接线经装配方 `continuum-runtime`**：`continuum-runtime` 是切分 §四第 1 条唯一允许因 P6 增枚的
   既有行（切分引 `:206`；实测 `crates/continuum-runtime/tests/dependency_direction.rs:392` 的
   `("continuum-runtime", &[…])` 那一行）。本块被它依赖，而不是反过来。

**层内边**：本块与 P6B／P6C 的边是**认知层内部的边**（`continuum-context → continuum-trust`、
`continuum-context → continuum-memory`）。层内边在本仓有同形处置：`continuum-provider →
continuum-capability` 就被登记为层内边（`crates/continuum-runtime/tests/dependency_direction.rs:28-34`
的注记）。两条都是单向的：P6B／P6C 不依赖本块。

## 2.4 本块发明的判据（逐条：规范给到哪、代价、边界、落点、收件人）

**这一节是本块的「发明判据」总账（共 14 条）。** 规范没给、而本设计不得不定的每一处判定，都在下表里有一条；
**没进这张表的判定，本块一处也没有定。** 代码块只是示意，正文措辞才是约束。

| # | 发明的判定 | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | 四类内容数组的**元素类型**：`instructions[]` / `references[]` 为文本与 `Reference`，`artifacts[]` 为 `ArtifactId`，`memories[]` 为 `MemoryClaim` | §280 `:1522-1536` **只给字段名**，四类元素的类型一处未给 | 消费方要按 `Reference` 的变体 match；`artifacts[]` 不带 `content_hash`，无法在包内校验「引用的是哪一版」 | 不覆盖「一段引用有多个来源」；不覆盖「制品内容变了而 id 未变」 | §4.1、§4.5 | 规范维护者 |
| 2 | `provenance[]` / `trust_labels[]` 与四类内容数组的**对齐规则**：按 §280 的字段声明序拼接、**等长同序** | §281 `:1540` 只说「每个上下文片段 SHOULD 标注」，§280 只给两个数组，**对齐规则未给** | 消费方要按声明序计算偏移；规范若改字段序，对齐随之改 | 不覆盖「一个片段多个来源」；不覆盖「一个来源多段内容」的合并 | §4.2 | 规范维护者 |
| 3 | `references[]` 承载 Conversation／Project／Tool results；§280 没有它们的专属字段 | §279 `:1507-1519` 列五类来源，§280 的六字段里**没有** conversation／project／tool_results | 消费方按 `Reference` 变体区分历史对话、Project 片段与工具结果；Project State 的结构化读取被压成文本 | 不覆盖「Project State 的字段级读取」；不覆盖「对话的会话边界」 | §5.4 | 规范维护者 |
| 4 | 选择规则：指令全收；其余按**候选序**截断；`ArtifactId` 与 claim id 去重 | §279 只说「选择最小必要上下文」；§70 只给四个目标，**相关性判据未给** | 相关性由调用方给的候选序承担；P6A 不判「这段是否真的相关」 | 不覆盖「预算内的最优选择」；不覆盖「候选序本身有偏」 | §5.3 | 规范维护者 |
| 5 | 预算用**片段条数**（三个独立上限），不用 token | §70 `:391` 的「减少 token」是目标，**token 预算与 tokenizer 规范未给** | token 仍可能超；长片段与短片段的成本在预算里同权 | 不覆盖「长片段挤占预算」；不覆盖「模型的上下文窗口」 | §5.2、§5.3 | 规范维护者 |
| 6 | 空串候选在装配前丢弃 | 「减少无关信息」是目标（§70 `:393`），**空串规则未给** | 输入 n 条可能输出少于 n 条；调用方不能靠「我传了 n 条」断言输出 n 条 | 不覆盖「仅含空白的格式片段」（本设计只丢 `is_empty`，不丢空白串） | §5.3 | 规范维护者 |
| 7 | `ContextPackage` 字段私有、**唯一构造点** `compile`、不实现 `Default` / `Deserialize` | §6.4 判据 1 `:402` 要求「不存在绕过路径」，**类型层形态未给** | 跨进程／持久化不能直接反序列化；需要受控入口时得另立一条（本块不立） | **不覆盖「执行器不调 compile」**——类型层拦不住，见 §6.3 与 §11 第 1 条 | §4.3、§6.4 | P1 ＋ 协调者 |
| 8 | 不把 package 投影为 `Message` / `InvokeRequest` | §9.4 `:635` 只说载荷是 provenance／trust_labels；`CallInput.messages` 是 `&[Message]`（`crates/continuum-runtime/src/model_call.rs:373-379`），**没有标签位置** | 消费方要自己投影；投影规则规范未给，P6A 不发明 | 不覆盖「模型实际看到什么」；不覆盖「标签在模型边界是否丢失」 | §6.2 | P3 ＋ 协调者 |
| 9 | `ContextRequest` 的 scope／锚点／文本三个检索输入，以及 `compile` 构造 `RetrievalQuery` | §6.3 `:391` 只说 `Context Compiler ← MemoryClaim`；P6C 已冻 `RetrievalQuery`（P6C §6.1），**规范未给** | P6A 依赖 P6C 的 `RetrievalQuery`；`include_terminal` 写死 `false`；`scopes` 空或 `max_memories == 0` 不调检索 | **不覆盖隐私判定**——scope 由调用方选，`Personal` 的读取授权在 P2／第 4 层（§182）；不覆盖「相关度」 | §5.2、§5.3、§7.2 | 规范维护者 ＋ P6C |
| 10 | `Reference::Conversation` 复用 `core::model::Message`，不造 `Turn` / `Conversation` 类型 | §279 的 Conversation 没有类型；仓里**没有** Conversation 类型（`grep -rn "struct Conversation" crates/` 零命中） | 复用 §315 的 `Message`；对话的「会话／轮」边界表达不了 | 不覆盖「会话边界」；不覆盖「非模型消息形态的历史」 | §4.5 | 规范维护者 |
| 11 | **容器隔离**：`references` / `artifacts` / `memories` 的候选不得进 `instructions[]`；`instructions[]` 只来自 `request.instructions` | §70 `:394` 的「降低 prompt injection 面」与 §281 `:1552` 的「外部内容不得与用户指令同级」是**目标／MUST**，但「按容器隔离」是本设计选的落法，**规范未给** | 外部内容被错放进 `instructions` 时本块不拦（P6B／P2 的缺口）；调用方要自己分容器 | 不覆盖「instructions 容器内的候选是否真是用户指令」；不覆盖「同一段内容同时是用户指令与外部引用」 | §5.3、§5.4 末 | 规范维护者 |
| 12 | **纯函数与可重复**：`compile` 除检索面外不读任何状态；同一请求＋同一检索面的产物可重复；请求外的来源不出现 | §69 `:324` 的隔离与 §70 `:361` 的「不能全塞」是目标；**「纯函数／可重复」是本设计的选择** | 不能读全局缓存；重复编译要重新检索 | 不覆盖「调用方给的候选本身越权」；不覆盖「检索面的实现有隐藏状态」 | §5.5、J7a／J7b | 规范维护者 |
| 13 | **空包合法**：空请求 ⇒ 六数组皆空（`instructions[]` 也空） | §280 只给六个数组字段，**没有给「包必须非空」**；空包是边界用例，规范未禁止 | 一个空包仍算「通过了 Compiler」；执行器可能拿到空上下文 | 不覆盖「空包是否应被拒绝」——本块不拒；若规范要求非空，加一条臂 | §4.3、J10 | 规范维护者 |
| 14 | **§70 的 `Model Context Package` 与 §280 的 `ContextPackage` 的对应关系**：本设计读作「前者是消费方投影后的模型面产物，后者是结构化包」 | §70 `:382-386` 只给输出名，§280 `:1525` 只给类型名，**两者的关系未给** | 消费方要自己投影；本块不交付模型面产物 | 不覆盖「投影后标签是否保留」——那是消费方的投影规则 | §6.2 | 规范维护者 |

## 2.5 切分 §四 六条横切约束的逐条落实

**先说一句「不新冻」**：§2.2 的五个类型是 P6A 的**装配形状**（`ContextPackage` 的六字段名来自 §280，
其余四枚是本块的输入／预算形状），**不是对 P6B／P6C 的冻结**。本块不定义 `ContextProvenance` / `ContextTrust` /
`MemoryClaim` / `MemoryScope`；六字段与七标签的名字照抄规范，一个不改（切分 §2.1）。

**这一节逐条对账切分 §四 的六条**（切分文档 `:123-163`）。每条给「切分的要求 / 本设计的落点 /
可观察判据」；**没有一条是本块新加的约束**。

| 切分 §四 | 要求（要点） | 本设计的落点 | 可观察判据 |
|---|---|---|---|
| **第 1 条**（`:125-132`） | 不新增反向边；四块各自在 `ALLOWED` 加自己一行；既有行不因 P6 增枚，唯一例外是装配方 `continuum-runtime` | §2.1 的边表（四条，全部是本块指向别层／同层）；§2.3 的「不登记指向执行层的代码边」；§6.3 的 (b) 候选 | `cargo test -p continuum-runtime --test dependency_direction` 在含本 crate 的树上绿；`ALLOWED` 的 `continuum-context` 行恰是边表的四项；既有行（除 `continuum-runtime`）不出现 `continuum-context` |
| **第 2 条**（`:133-142`） | 迁移号段：P6 的档由 P6C 写死；其余块从同一档顺取；现场核对空号 | §10.2：本块**零表、不占档**；`runtime_migrations()` 不加行 | 本 crate 的 `Cargo.toml` 无 `continuum-persist`；`runtime_migrations()` 无 `continuum_context` 的调用 |
| **第 3 条**（`:143-146`） | 同一件事两个词汇表：`NodeTrust` / `Artifact.provenance` 与 §281 的标签／`ContextProvenance` 不得混名；七标签另取名字，四块用同一个 | §3.5 的三处同名异物表（一个都不用）；§4.6（不定义 `ContextProvenance` / `ContextTrust`）；§7.1（名字用 P6B 已冻结的 `ContextTrust`） | J9 的源码面：本 crate 无 `enum ContextTrust` / `enum ContextProvenance`；无 `NodeTrust` / `PrivacyClass` 的引用 |
| **第 4 条**（`:147-149`） | 工具结果有两个消费者（§279 的 Compiler 与 §89 的 Evidence 转换），不得各造一套 wrapper | §7.6：本块的 `Reference::ToolOutput` 直接装 `core::tool::ToolResult`；P5a 的 `Evidence::from_tool_result` 直接读字段；两边都不建 wrapper | J4 的第⑤条（`ToolOutput` 进 `references[]`）＋ §7.6 的源码面（本 crate 无 `from_tool_result` 的包装） |
| **第 5 条**（`:150-152`） | 依赖边只登记实际用到的；`ALLOWED` 与实际依赖精确一致（逐对 `assert_eq!`） | §2.1 的四行边表与 `ALLOWED` 的四项一一对应；dev 边零条 | `dependency_direction` 的两个方向：已声明而越界、本表列了却没用到，都绿 |
| **第 6 条**（`:153-163`） | 两对块共享未冻接口：P6B 先冻标签类型、P6C 先冻三枚类型与检索面；否则并块 | §7.1 的「P6B 给本块的一页接口」；§7.2 的「P6C 给本块的四枚」；§11 第 11、12 条 | **P6B／P6C 的一页接口已落地**（P6B §3.1；P6C §3.5、§6.1）；两页接口均已落地，包括 `MemoryClaim` 的读访问器 |

**第 4 条的「待与 P5 那轮对账」**：P5 已落地；P5a 设计 `:1051` 的对账条目 (ii) 已给出「两者不共享
wrapper」的答复。本节的第 4 条即对该答复的接续，不另开对账条目（§7.6）。

---

# 3. 规范依据逐条

**这一节的写法**：每条先给规范原文的落点，再给「规范给了什么 / 规范没给什么 / 本设计怎么落」。
**没有一条判据是本节发明的**——本节的每一条都指回一个规范节；规范没给的部分在 §2.4 里另记。

## 3.1 §279 Context Compiler（`docs/spec/05-normative.md:1500-1520`）

**原文（逐字，`:1502-1519`）**：

> Node 执行前 MUST 通过：
> ```
> Context Compiler
> ```
> 从：
> ```
> Conversation
> Memory
> Artifacts
> Project
> Tool results
> ```
> 选择最小必要上下文。

**规范给了什么**：①通道的**时点**是「Node 执行前」，强度是 **MUST**；②五类来源；③「最小必要」这个目标。

**规范没给什么**：①「通过」的形态（函数？trait？类型？）；②「最小必要」的判据（相关性？预算？）；
③五类来源与 §280 六字段的对应（§280 没有 conversation／project／tool_results 字段）。

**本设计怎么落**：通道的形态是 `compile`（§5.1）；「最小必要」落成预算、去重、空串丢弃、容器隔离
（§5.3），相关性判据报出（§2.4 第 4 条）；五来源到四字段的映射在 §5.4（§2.4 第 3 条）。

## 3.2 §280 Context Package（`:1522-1536`）

**原文（逐字，`:1524-1535`）**：

> ```
> ContextPackage {
>     instructions[]
>     references[]
>     artifacts[]
>     memories[]
>
>     provenance[]
>     trust_labels[]
> }
> ```

**规范给了什么**：六个字段的**名字与书写序**。**这是本块唯一被冻结的形状**（切分 §2.1：照抄、不新冻）。

**规范没给什么**：六个字段的元素类型；`provenance[]` / `trust_labels[]` 与内容数组的对齐规则；
字段的可见性、构造方式、序列化。

**本设计怎么落**：字段名与序逐字照抄（§4.1）；元素类型本设计定（§2.4 第 1 条）；对齐规则本设计定
（§4.2，§2.4 第 2 条）；字段私有、唯一构造点（§4.3）。

## 3.3 §69 Agent Context Isolation（`docs/spec/02-positioning.md:322-355`）

**原文（逐字，`:324`、`:335-351`、`:355`）**：

> 不同 Agent 默认拥有不同上下文。
> …
> Coding Agent 不应自动获得：
> ```
> 用户全部聊天
> 其他项目
> 邮件
> 个人 Memory
> ```
> 它只获得：
> ```
> 本任务需要的信息
> Task Contract
> 相关 Artifact
> 必要工具
> ```
> 原则：
> > «Context 也属于 capability。»

**规范给了什么**：①不同 Agent 的上下文**默认隔离**；②只给「本任务需要的信息」；③Context 属于
capability。

**规范没给什么**：①隔离的强制点（capability 判定发生在 Compiler 内部还是外部——C4／OPEN-016，
《工程》§6.5 `:412`）；②「本任务需要」的判据。

**本设计怎么落**：`compile` 只读参数（请求、预算、检索面），**不读任何全局来源**（§5.5）——「不同 Agent
不同上下文」由调用方传不同请求实现，P6A 不合并全局状态。capability 的**判定**不在本块：本块的口径是
「判定在外，第 4／5 层做，Compiler 只接收已授权的原料集合」；若 C4 裁定「判定在内」，本块要接
capability 类型并重审是否纳入 Protected Core（§11 第 9 条）。**这一口径与 P6D 必须一致**（切分 §八.1
第 4 条）。

## 3.4 §70 Context Compiler（`docs/spec/02-positioning.md:359-395`）

**原文（逐字，`:361`、`:369-378`、`:388-394`）**：

> 不能直接把所有信息塞进模型 context。
> …
> 负责从：
> ```
> Intent
> Artifact
> Memory
> Tool Result
> Conversation
> Project State
> ```
> 选择当前节点真正需要的信息。
> …
> 目标：
> ```
> 减少 token
> 降低隐私暴露
> 减少无关信息
> 降低 prompt injection 面
> ```

**规范给了什么**：①「不能全塞」；②六个来源（比 §279 多 `Intent` 与 `Project State`）；③四个目标。

**规范没给什么**：①四个目标的**数值判据**（减到多少 token？降到什么隐私档？）；②`Intent` 与
`Project State` 的形态。

**本设计怎么落**：三个目标有落点（§5.3 的预算／去重／容器隔离、§5.5 的无隐藏来源、§4.2 的逐片段标签）；
**「降低隐私暴露」在本块没有落点**——本块只按调用方给的 `scopes` 取记忆、不判隐私等级、不做放置裁决；
它归 P2／第 4 层／协调者（§11 第 16 条）。`Intent` 以文本由调用方投影进 `instructions`／`references`
（§5.4 末），`Project State` 以文本进 `references`；**本设计以 §279 的五类为 MUST 输入面**
（切分 §五 P6A 行照 §279 写），§70 多出的两类是同一编译器的可选原料，不另立字段（§2.4 第 3 条）。

## 3.5 §281 Context Provenance（`:1538-1554`）——**本块只引用，不判**

**原文（逐字，`:1540-1552`）**：

> 每个上下文片段 SHOULD 标注：
> ```
> USER_INSTRUCTION
> TRUSTED_RUNTIME
> PROJECT_DATA
> WEB_UNTRUSTED
> TOOL_OUTPUT
> MEMORY
> MODEL_OUTPUT
> ```
> 外部内容 MUST NOT 与用户指令处于同一信任级别。

**规范给了什么**：七个标签名；「每个片段 SHOULD 标注」；「外部内容 MUST NOT 与用户指令同级」。

**规范没给什么**：标签的类型形态、附加点、`provenance[]` 与标签的关系。

**本设计怎么落**：`trust_labels[]` 的元素类型是 P6B 已冻结的 `ContextTrust`，`provenance[]` 的元素类型是
P6B 已冻结的 `ContextProvenance`；本块**不定义、不改名、不赋标签字面量**（§4.6、§7.1）。本块在装配时
对每个片段调用 P6B 的 `label(&provenance)`，把返回值按同序装进 `trust_labels[]`；「外部内容与用户指令
不同级」的**判定**归 P6B（判据 2，§6.4 `:403`），本块不重复判。

**三处同名异物（切分 §2.4）不是标签，本块一个都不用**：

| 已有的东西 | 它是什么 | 实测位置（2026-10-17） |
|---|---|---|
| `Artifact.provenance`（`serde_json::Value`） | 制品血缘（谁产出了它） | `crates/continuum-artifact/src/artifact.rs:222`（结构体在 `:214`；切分引的 `:170/:178` 已漂） |
| `PrivacyClass`（五档） | 算力放置的隐私等级 | `crates/continuum-artifact/src/artifact.rs:131`（字段在 `:223`） |
| `NodeTrust`（两枚）与测试辅助 `fn trust_label` | 计算节点的信任域 | `crates/continuum-node/src/node.rs:60`；`crates/continuum-node/tests/node.rs:37` |

**本块的 `provenance[]` 是 §281 的 `ContextProvenance`，不是 `Artifact.provenance`**：前者是「这段上下文
从哪来」，后者是「这份制品由谁产出」。两者同名不同物，本块只在 `artifacts[]` 的**元素是 `ArtifactId`**
这一点上与制品层相接，不读也不写 `Artifact.provenance`。

## 3.6 《工程》§6.2（`docs/02-工程.md:378`）

**原文（逐字）**：

> **Context Compiler** 是节点执行的强制通道（§279 MUST），同时实现四个目标：减少 token、降低隐私暴露、
> 减少无关信息、降低 prompt injection 面（§70）。

**落点**：§5（装配通道）与 §5.3（**三个目标**的逐条落法：减少 token、减少无关信息、降低 prompt injection 面；「降低隐私暴露」无落点，见 §11 第 16 条）。**「强制通道」的端到端形态缺执行器**（§11 第 1 条）。

## 3.7 《工程》§6.3（`:391`）

**原文（逐字）**：

> `Context Compiler ← MemoryClaim + Artifact + Conversation`

**落点**：§5.4 的三条上游——`MemoryClaim` 经 P6C 的检索面进 `memories[]`；`Artifact` 经 `ArtifactId` 进
`artifacts[]`；`Conversation` 经 `Reference::Conversation(Message)` 进 `references[]`。**`Project` 与
`Tool results` 不在这条层内依赖里**，但 §279 要求它们作原料，故它们在 §5.4 的映射表里各占一行
（`references[]`）。

## 3.8 《工程》§6.4 判据 1（`:402`）

**原文（逐字）**：

> 节点执行前必经 Context Compiler，不存在绕过路径

**落点**：§4.3（唯一构造点）、§6（消费契约）、§8 的 J1。**本轮能拍到的是类型层与源码面**；端到端的
「必经」没有照片，因为执行器未建（§8.4 第 1 条、§11 第 1 条）。

## 3.9 《工程》§9.4（`:635`）

**原文（逐字）**：

> 认知层 → 执行层    Context Package 的 provenance 与 trust_labels 字段

**落点**：本块把 `provenance[]` / `trust_labels[]` 作为 `ContextPackage` 的**两个独立字段**承载（不压进
`Message`，§6.2），使这条载荷在类型上存在。**这条边的代码落点不在本块**：本块不依赖执行层 crate，
接线经 `continuum-runtime`（§2.3）。两种读法的处置见 §2.3 与 §11 第 2 条。

---

# 4. `ContextPackage` 的类型与接口

## 4.1 六字段（照 §280 逐字）

```
/// §280 的六个字段。字段名与书写序照规范逐字，一个不改、一个不加。
/// 字段私有：唯一的构造点是 [`compile`]（§4.3）。
/// 不实现 `Default` / `Deserialize`（那会造出第二个构造点）。
#[derive(Debug, Clone, PartialEq)]
pub struct ContextPackage {
    instructions: Vec<String>,
    references: Vec<Reference>,
    artifacts: Vec<ArtifactId>,
    memories: Vec<MemoryClaim>,

    provenance: Vec<ContextProvenance>,     // P6B 的类型；本 crate 不定义
    trust_labels: Vec<ContextTrust>,   // P6B 的类型；本 crate 不定义
}
```

**字段名的逐字一致**：§280 的 `instructions[]` / `references[]` / `artifacts[]` / `memories[]` /
`provenance[]` / `trust_labels[]` 里的 `[]` 是规范的**数组记法**；Rust 侧对应 `Vec<T>`，**字段名一个
字母不改**（`instructions` / `references` / `artifacts` / `memories` / `provenance` / `trust_labels`）。

**逐字段的「元素类型 / 来源 / 规范状态」**：

| 字段 | 元素类型 | 来源（§279／§70） | 规范状态 |
|---|---|---|---|
| `instructions[]` | `String` | 用户指令；§70 的 `Intent` 以文本投影 | **元素类型规范未给**（§2.4 第 1 条） |
| `references[]` | `Reference`（本块定义，§4.5） | Conversation、Project、Tool results；普通引用文本 | **元素类型与来源映射规范未给**（§2.4 第 1、3 条） |
| `artifacts[]` | `ArtifactId`（`continuum-artifact`） | §279 的 Artifacts；§6.3 的 `Artifact` | **元素类型规范未给**；只存 id，不复制内容 |
| `memories[]` | `MemoryClaim`（P6C） | §279 的 Memory；§6.3 的 `MemoryClaim` | **元素类型规范未给**；P6C 的 §283 七字段 |
| `provenance[]` | P6B 的 `ContextProvenance` | 每片段一个 | **P6B 定**（切分 §2.3；本块只承载） |
| `trust_labels[]` | P6B 的 `ContextTrust` | 每片段一个 | **P6B 定**（§281 七值；本块只承载） |

**为什么 `artifacts[]` 是 `ArtifactId` 而不是 `Artifact` 或 `ArtifactRef`**：

- **不是 `Artifact`**：整份制品含 `metadata` / `provenance` / `privacy_class` 等字段
  （`crates/continuum-artifact/src/artifact.rs:214-225`），把它塞进上下文违背 §70 的「减少 token」与
  §279 的「最小必要」；且 `Artifact.provenance` 会被误读成 §281 的 `ContextProvenance`（§3.5 的同名异物）。
- **不是 `ArtifactRef`**：它是执行层的类型（`crates/continuum-graph/src/execution.rs:36-40`），
  本块依赖它会新增 `执行层 → 认知层` 的直接边（§2.1 第 1 条不登记的理由）。
- **代价据实记**：不带 `content_hash`，包内无法校验「引用的是哪一版制品」。这一条是 §2.4 第 1 条的
  「边界」，不是漏项；校验属制品层（`ArtifactStore` 的 `ContentMismatch`，`crates/continuum-artifact/src/store.rs:28`）。

**为什么 `memories[]` 是整枚 `MemoryClaim` 而不是 claim id**：§6.3 逐字写的是 `Context Compiler ←
MemoryClaim`（不是 `MemoryClaimId`）；§283 的 claim 是派生表示（《总纲》§6.2 `:1031` 的来源图段：
「Source 才是事实来源，Memory 是 derived representation」），它本身就是上下文要的内容。代价：claim 的
`sources[]` 等字段随行；本块不裁 claim 内部，只选整枚 claim（§2.4 第 4 条的边界）。

## 4.2 对齐规则（`provenance[]` / `trust_labels[]` 与内容片段）

**规则（本设计定，规范未给；§2.4 第 2 条）**：

1. **片段序列**＝ `instructions[]`（按序）→ `references[]`（按序）→ `artifacts[]`（按序）→
   `memories[]`（按序）。这个序就是 §280 的**字段书写序**（`:1526-1529`），本设计不另排序。
2. **等长**：`provenance.len() == trust_labels.len() == instructions.len() + references.len()
   + artifacts.len() + memories.len()`。空包时三者都是 0。
3. **同序**：第 `i` 个片段的 provenance 是 `provenance[i]`，标签是 `trust_labels[i]`。
4. **一个片段一个 provenance**：不表达「一段内容多个来源」；**多来源的内容在请求侧拆成多个候选**
   （§5.2）。P6B 的 `ContextProvenance` 是**八变体枚举**（P6B §3.2），**每枚只描述一个来源，
   不聚合多来源**。
5. **构造时同步 push**：`compile` 每装入一个片段，就把该候选的 provenance 与
   `label(&provenance)` 各 push 一次（§5.3 第 6 条）；因此等式在构造后**必然**成立。`debug_assert`
   与测试断言是防御，不是输入校验。
6. **消费方按声明序读**：只读 `references()` 的消费方，若要读第 `j` 个引用的标签，偏移是
   `instructions.len() + j`。本块**不提供**按字段切片的标签访问器（那会多一枚类型或一组方法，
   且不解决「四数组共享两数组」的根本形状）。
7. **数组与 provenance 的约束（P6B 的 I4/I5）**：`UserInstruction { historical: false }` 只进
   `instructions[]`；`{ historical: true }` 只进 `references[]`；`USER_INSTRUCTION` 不进
   `artifacts[]` / `memories[]`；`TrustedRuntime` 可进 `instructions[]` 或 `references[]`
   （P6B §3.1 的 `may_sit_in_instructions`）。本块的装配按 §5.4 的映射与 §7.1 的构造入口选数组；
   **判定由 P6B 的 `validate` 复核（J3）**，`compile` 的生产路径不调用它（P6B §5.2 第 5 条）。

**规范未给这一条的代价**：消费方必须知道拼接规则才能正确配对；若将来规范给另一种对齐（例如
`provenance[]` 只给「非指令片段」），本块的对齐与所有照片要一起改。**收件人：规范维护者**（§11 第 4 条）。

**为什么不在类型层把「内容 + 标签」配成一枚结构**：§280 明确把六个字段**分开**列；若本块另造
`Vec<LabeledFragment>`，就违背了「照抄六字段」的冻结（切分 §2.1），也会让消费方以为规范给的是配对结构。
对齐只能落在**规则**上，不能落在**类型**上——这一条是本设计的取舍，不是省事。

## 4.3 唯一构造点与不变量

**唯一构造点**：`compile`（§5.1）是**唯一**返回 `ContextPackage` 的 `pub` 函数。本块不提供：

- `ContextPackage::new` / `from_parts` / `from_fields`；
- `impl Default for ContextPackage`；
- `impl<'de> Deserialize<'de> for ContextPackage`（反序列化是第二个构造点）；
- 任何 `pub` 的字段或 `pub` 的可变访问器。

**允许的**：`Clone`（值的复制，不产生新的内容组合）、`Debug`、`PartialEq`、六个只读访问器（§4.4）。
`Clone` 不构成「第二个构造点」：它只能复制一个已经由 `compile` 构造出的值，不能把任意六数组拼成一个包。

**`Debug` 只用于测试／诊断，不作为审计或日志输出**：`memories[]` 是整枚 `MemoryClaim`，`Debug` 会把
`statement` 等字段带进日志。若某处需要记录「编译了什么」，只记计数与 id，不记 `Debug` 全文（隐私目标
归 P2／第 4 层，§11 第 16 条）。

**`compile` 内保证的五条不变量**（照片见 §8.2）：

1. **对齐**：§4.2 的等长同序。
2. **容器来源**：`instructions[]` 只来自 `request.instructions`；`references[]` 只来自
   `request.references`；`artifacts[]` 只来自 `request.artifacts`；`memories[]` 只来自检索面。
   （「外部内容不得进 instructions」的**判定**不在这里，见 §5.4 末。）
3. **去重**：`artifacts[]` 的 `ArtifactId` 逐枚不同；`memories[]` 的 claim id 逐枚不同。
4. **预算**：三个数组的长度分别不超过 `budget.max_references` / `max_artifacts` / `max_memories`。
5. **指令全收**：`request.instructions` 里的非空候选**全部**进 `instructions[]`，不受预算约束。

**这五条不产生 `ContextError`**：它们由 `compile` 自己的装配逻辑保证，不是输入校验失败。唯一的失败是
P6C 的 `MemoryError`：`RetrievalQuery::new` 的构造拒绝（`EmptyScopeSet`／`AnchorRequired`／
`AnchorNotAllowed`）或 `retrieve` 的检索失败（§5.6）。**本块不定义错误类型**——没有本块的判定失败，
就没有第二枚错误类型（同 P5c 设计 §3.2 的「判定是同一件事则合并，是两件事则分开」）。

**「不存在绕过路径」在类型层的射程**：它拦得住「外部代码用结构体字面量／`Default`／反序列化造一个包」，
**拦不住**「执行器不调 `compile` 就直接执行」。后半句是 §6.3 的缺口，不是本块能在类型层补的。

## 4.4 访问器

```
impl ContextPackage {
    pub fn instructions(&self) -> &[String];
    pub fn references(&self) -> &[Reference];
    pub fn artifacts(&self) -> &[ArtifactId];
    pub fn memories(&self) -> &[MemoryClaim];
    pub fn provenance(&self) -> &[ContextProvenance];
    pub fn trust_labels(&self) -> &[ContextTrust];
}
```

- **只有只读访问器**：没有 `&mut`、没有 `pub` 字段。标签一旦由 `compile` 装上，消费方改不了。
- **没有 `into_parts()` / `into_iter()`**：拆包不是「第二个构造点」，但它会让六字段的绑定关系在包外
  消失（消费方可以拿到四个内容数组与两个标签数组后自行重组）。本块不提供这条通道；若将来某个消费者
  确实需要按值拿走内容，那是一条要单独审的接口（§11 第 13 条）。
- **访问器的顺序与 §280 的字段序一致**，便于对照。

## 4.5 `Reference`：`references[]` 的元素类型

```
/// §280 的 `references[]` 元素。规范未给元素类型；本设计定（§2.4 第 1 条）。
/// 三个变体对应 §279 的三类非指令来源；普通引用文本走 `Text`。
#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    /// 普通引用文本：文档、URL 内容、Project 片段、Intent 的文本投影。
    Text(String),
    /// 历史对话轮。复用 §315 的 `Message`（role + content），不另造 `Turn`（§2.4 第 10 条）。
    Conversation(Message),
    /// 工具结果。复用 §316 的 `ToolResult`（output + is_error），不造 wrapper（§7.6）。
    ToolOutput(ToolResult),
}
```

**为什么是枚举而不是 `String`**：`ToolResult` 的 `output` 是 `serde_json::Value`
（`crates/continuum-core/src/tool.rs:27`），`Message` 有 `role` 与 `content` 两个字段
（`crates/continuum-core/src/model.rs:38-41`）。压成 `String` 要在 P6A 里发明「工具结果如何序列化」
与「对话轮如何拼接」两条规则，而规范一处都没给；枚举把结构原样保留，把投影规则留给消费方（§6.2）。

**为什么不是三个独立字段**：§280 只给 `references[]` 一个字段，本块不能增字段；枚举是同一字段内的
三种形态，不违背六字段的冻结。

**`Reference` 需要 `Clone` / `PartialEq` / `Debug`**：`ContextPackage` 的派生需要它们；`Message` 与
`ToolResult` 都已实现这三者（实测 `crates/continuum-core/src/model.rs:37`、`crates/continuum-core/src/tool.rs:25`）。
**不派生 `Serialize` / `Deserialize`**：本块不落库，且反序列化会给 `ContextPackage` 开第二个构造点
（§4.3）；`Reference` 单独序列化没有消费者。

## 4.6 与 P6B／P6C 的类型引用（本块不定义）

- `ContextProvenance` 与 `ContextTrust` 的**类型名、字段、构造入口**已由 P6B 冻结（P6B §3.1–§3.3）；
  本块用**实际名**，不定义、不改名。`UserInstruction { historical }` 的两个构造入口
  `user_instruction()`（当前）／`conversation_history()`（历史）由 P6B 给；`provider_supplied()` 是
  `WebUntrusted` 的具名入口。切分 §四第 6 条的串行已走完：P6B 先冻，本块对着写。
- `MemoryClaim` / `MemoryScope` / `MemoryError` 由 P6C 定（§283／§284 的字段名照规范）。本文件用
  `MemoryClaim` 指代 P6C 的类型，不是本 crate 的定义。
- **源码面判据**：`continuum-context` 的 `src/` 里没有 `enum ContextTrust`、`enum ContextProvenance`、
  `struct MemoryClaim`、`enum MemoryScope`（§8.2 的 J9）。这一条是「不另定义」的可观察形态。

---

# 5. 装配入口

## 5.1 签名、谁调、何时调

```
/// §279 的「节点执行前必经」通道。**本 crate 唯一的 `ContextPackage` 构造点**（§4.3）。
///
/// 调用方：节点执行器（P1，未建）在节点进入 `Running` 之前调用一次。`continuum-runtime` 今天
/// **没有节点执行路径**，故本轮以**库的形态**交付；§279 的端到端通道归 P1（§6.3、§11 第 1 条）。
/// 本函数不读节点状态、不写事件、不调度、不缓存。
///
/// 失败面：`RetrievalQuery::new` 的构造拒绝或 `retrieve` 的检索失败，均为 P6C 的 `MemoryError`，
/// 原样透传；本块不定义第二枚错误类型（§5.6）。
pub fn compile(
    request: ContextRequest,
    budget: ContextBudget,
    memory: &dyn MemoryRetrieval,
) -> Result<ContextPackage, MemoryError>
```

- **谁调**：§279 的主语是「Node 执行前」，故规范上的调用方是**节点执行器**。P1 的执行器今天未建
  （切分 §八.1 第 2 条：P1 已把 Context Compiler 整项排除给 P6，而执行侧没有接线点）。本设计**具名**：
  调用方是 P1 的节点执行器；`continuum-runtime` **今天没有节点执行路径**（`OperatorImpl::execute`
  只有 trait 定义与测试实现；`task_cmd` 是命令／效应路径），故它不是一个现成的节点级前置点。
  **只有在 P1 的节点执行器落地后，runtime 才可能有一个可接线的位置**；本轮本块以库形态交付，
  §279 的端到端通道仍归 P1。
- **何时调**：节点从 `Ready`／`Queued` 进入 `Running` **之前**（§237 的状态迁移由 P1 管，本块不碰）。
  一次节点执行调一次；产物供该节点的整个执行期使用。**不按模型调用次数编译**——一个节点可能多次调用
  模型，但 §279 的时点是「Node 执行前」，不是「每次 `invoke` 前」。
- **不缓存**：同一节点重复调用 `compile` 会重新装配。规范没有给缓存判据；本块不建第二份（同 §305 的
  复用判据在 P1，`crates/continuum-graph/src/reuse.rs:29`）。
- **同步**：`compile` 是同步函数；`MemoryRetrieval` 的实现可以是同步的内存／DB 读取。本块**不建**
  异步检索面；若装配方需要异步，由它在调 `compile` 之前取好 claim、另立适配（不在本块的接口里）。

## 5.2 请求与预算

```
/// §279 五类来源的候选集合与记忆检索的三个控制位
/// （其中 Conversation／Project／Tool results 三类共用 `references`，§5.4）。
/// 按值消费：`compile` 可以把内容移进产物，不需要 `Clone`。
pub struct ContextRequest {
    /// 当前用户指令（`user_instruction()`）／Runtime 系统指令（`trusted_runtime()`）的候选；§70 的
    /// `Intent` 以文本投影在这里。**不受预算约束**（§5.3 第 2 条）。**历史用户轮不放这里**——它们在
    /// `references`（`conversation_history()`，§5.4）。
    pub instructions: Vec<ContextCandidate<String>>,
    /// Conversation／Project／Tool results／普通引用。四类来源共用这一个容器，
    /// 由 `Reference` 的变体与 provenance 区分（§5.4）。
    pub references: Vec<ContextCandidate<Reference>>,
    /// §279 的 Artifacts；只装 id，不装内容（§4.1）。
    pub artifacts: Vec<ContextCandidate<ArtifactId>>,
    /// 记忆检索的 scope 集合（§284 的五档）。**空 ⇒ 不调检索面、`memories[]` 为空**
    /// （P6A 不默认全 scope，尤其不默认含 `Personal`；§5.3 第 5 条）。
    pub scopes: Vec<MemoryScope>,
    /// 实例锚点；直接进 `RetrievalQuery::new` 的 `anchor`。`Some` 时 `scopes` 必须恰好是一个
    /// anchored scope（`Project`／`Product`／`Task`），否则 P6C 的 `new` 返回 `AnchorNotAllowed`；
    /// anchored scope 缺 anchor ⇒ `AnchorRequired`；`Personal`／`Runtime` 不能带 anchor。
    pub anchor: Option<String>,
    /// 记忆检索的文本过滤，映射到 `RetrievalQuery.text`。**原样传递**：
    /// `None` 与 `Some("")` 在 P6C 的 `RetrievalQuery::new` 里都归一为 `None`（不做文本过滤）；
    /// 它**不是**「不检索」（是否检索由 `scopes` 与 `max_memories` 决定，§5.3 第 5 条）。
    pub memory_query: Option<String>,
}

/// 一枚候选片段：内容 + 它的来源。`provenance` 由调用方经 P6B 的构造入口给；
/// 本块不构造它（记忆片段除外：`ContextProvenance::memory(claim.id().as_str())`，§5.3 第 5 条）。
pub struct ContextCandidate<T> {
    pub content: T,
    pub provenance: ContextProvenance,   // P6B 的类型
}

/// 三个独立的条数上限。**不是 token 预算**（§2.4 第 5 条）。
/// `max_memories == 0` 与 `scopes` 为空**都导致不调检索面**、`memories[]` 为空（§5.3 第 5 条）。
pub struct ContextBudget {
    pub max_references: usize,
    pub max_artifacts: usize,
    pub max_memories: usize,
}
```

**为什么按值消费**：`ContextRequest` 里的 `String` / `Reference` / `ArtifactId` / `MemoryClaim` 都要移进
`ContextPackage`；按引用收会要求每一项 `Clone`，而 `MemoryClaim` 的 `Clone` 成本由 P6C 定。按值消费把
「候选集是调用方的一次性输入」写进签名，也避免了「编译后调用方还持有同一批候选并改写」的共享。
**要重复编译就构造两份逐字段相同的请求**（J7b 的照片如此写，不给 `ContextRequest` 加 `Clone`）。

**为什么 `ContextCandidate` 带 provenance 而不是让 `compile` 自己判来源**：切分 §五 P6B 行把「标签的
附加点」判给 P6B；P6A 的任务是**装配**，不是**判定来源**。候选的 provenance 由产生该候选的那一层
（或调用方）经 P6B 的构造入口给出；`compile` 只把它原样搬进 `provenance[]`，再调 `label` 得标签。
**P6A 不猜来源**：请求里没有 provenance 的候选在类型上不存在。

**为什么 `scopes` / `anchor` / `memory_query` 是三个字段**：P6C 已冻结
`MemoryRetrieval::retrieve(&RetrievalQuery)` 与 `RetrievalQuery`（P6C §3.5、§6.1）；`RetrievalQuery`
的字段私有，**唯一构造点是 `RetrievalQuery::new(scopes, anchor, text, limit, include_terminal)`**，
它执行 `P6C §5.4` 的 scope／anchor 规则（`P6C §6.1` 的 `new`）。本块把前三个投影到请求面，`limit` 取
`budget.max_memories`，`include_terminal` 写死 `false`（P6C §6.1：true 只供追溯，**不得进
`ContextPackage`**）。`new` 的拒绝点：`scopes` 空 ⇒ `EmptyScopeSet`；anchored scope 缺 anchor ⇒
`AnchorRequired`；anchor 与 scopes 不匹配 ⇒ `AnchorNotAllowed`（P6C §6.1）。**scope 的选择是调用方的
事，不是 P6C 的隐私判定**：P6C 只按给定的 `scopes` 过滤（P6C §6.3）；`Personal` 的读取授权在 P2／
第 4 层（§182），本块不判、不默认包含。

## 5.3 选择规则（§279 的「最小必要」在本块的落法）

**逐步规则**（`compile` 的实现约束；每一步都可测试，§8.2）：

1. **丢空串**：`instructions` 里 `content.is_empty()` 的候选丢弃；`references` 里
   `Reference::Text(s)` 且 `s.is_empty()` 的丢弃。**只丢 `is_empty`，不丢纯空白**（§2.4 第 6 条）。
2. **指令全收**：`request.instructions` 剩下的候选**全部**进 `instructions[]`，按输入序；不占预算，
   不截断。按 §5.4，`request.instructions` 只应装**当前用户指令**（`user_instruction()`，
   `historical: false`）与 **Runtime 系统指令**（`trusted_runtime()`）；历史用户轮应在 `references`
   （`conversation_history()`）。**P6A 不在这里判 `historical`**——那是 J3 的 `validate` 复核
   （P6B §3.6 I4/I5）。**代价**：指令本身过长时包会超 token；本块不替规范截断用户指令（§281 的
   「外部内容不得与用户指令同级」是这里的方向依据：不能为了塞外部内容而丢指令）。
3. **references 截断**：按输入序取前 `budget.max_references` 条；其余丢弃。
4. **artifacts 去重后截断**：按输入序扫，`ArtifactId` 首次出现才保留，取前 `budget.max_artifacts` 条；
   重复的 id 静默丢弃（不是错误）。
5. **memories 检索、去重、截断**：仅当 `!request.scopes.is_empty() && budget.max_memories > 0` 时，
   **构造查询**：`let query = RetrievalQuery::new(request.scopes, request.anchor, request.memory_query,
   budget.max_memories, false)?;`（唯一构造点；`?` 透传 P6C 的 `MemoryError`），再调
   `memory.retrieve(&query)`；对返回的 claim 按 `claim.id().as_str()` 去重（首个保留），取前
   `budget.max_memories` 条。每个 claim 的 provenance 是
   `ContextProvenance::memory(claim.id().as_str())`（P6B 的构造入口；本块只传 id，不命名标签）。
   **`scopes` 为空或 `max_memories == 0` ⇒ 不调检索面、`memories[]` 为空**（不触发 P6C 的
   `EmptyScopeSet`；`limit = 0` 在 P6C 是 `Ok(vec![])`，本块在 0 时不调检索）。`memory_query` 原样
   传递：`None` 与 `Some("")` 在 `new` 里都归一为 `None`（不做文本过滤），**不是**「不检索」。
6. **同步装配**：按 `instructions → references → artifacts → memories` 的序，把每个片段的 provenance
   与 `label(&provenance)` 各 push 一次（§4.2 第 5 条）。

**「最小必要」的可测面与不可测面（据实写清）**：

- **可测**：预算上限（三个数组都不超）、去重（artifact id 与 claim id 各自唯一）、空串不出现、
  指令全收、被截断的候选不出现。
- **不可测／规范未给**：「这段是否真的相关」——本块用**候选序**承担相关性，不做打分、不做 embedding、
  不做检索。**这一条是本设计最大的边界**：`compile` 只能保证「在你给的候选里选前 N 条」，
  不能保证「选中的就是最该给的」。收件人：规范维护者（§2.4 第 4 条、§11 第 7 条）。

**为什么不用 token 预算**：规范没有给 tokenizer、没有给「一段文本算多少 token」的规则；本块引入任何
一个 tokenizer 都会把模型实现绑进认知层，且不同模型的计数不同。条数预算是**可移植的下界**，代价是
长片段与短片段同权（§2.4 第 5 条）。

## 5.4 五来源到四字段的映射

| §279 来源 | 请求侧容器 | 输出字段 | 本设计的处置 |
|---|---|---|---|
| **Conversation** | `references`，`Reference::Conversation(Message)`；当前指令在 `instructions` | `references[]` / `instructions[]` | `role == User` 的历史轮用 `conversation_history()`（`historical: true`）进 `references[]`；当前指令用 `user_instruction()`（`false`）进 `instructions[]`；`role == Assistant` 用 `model_output()` 进 `references[]`；`role == System` 由 provider 提供用 `provider_supplied()`（进 `references[]`）、由 runtime 提供用 `trusted_runtime()`（可进 `instructions[]` 或 `references[]`）（P6B §4.2 第 1a/1b/2/4/7 行） |
| **Memory** | `scopes`／`anchor`／`memory_query` ＋ `MemoryRetrieval` | `memories[]` | P6C 的检索面；claim 整枚装入（§4.1） |
| **Artifacts** | `artifacts`，`ArtifactId` | `artifacts[]` | 只装 id；内容由消费方按 id 取（§4.1） |
| **Project** | `references`，`Reference::Text` | `references[]` | Project State 的结构化读取不在本块（无类型、无存储）；调用方投影为文本（§2.4 第 3 条） |
| **Tool results** | `references`，`Reference::ToolOutput(ToolResult)` | `references[]` | 直接装 `core::tool::ToolResult`；不转 Evidence（P5a 的构造点，§7.6）。**第三方 provider 的工具 schema／系统消息**不是工具结果，走 `Reference::Text` ＋ `provider_supplied()`（WEB_UNTRUSTED；P6B §4.2 第 4 行、§3.3 `:362`） |
| （§70 的 Intent） | `instructions` 或 `references` | 对应字段 | §279 没列 Intent；本设计以 §279 五类为 MUST，Intent 以文本投影（§3.4） |
| （§70 的 Project State） | `references`，`Reference::Text` | `references[]` | 同 Project |

**这张表的读法**：左边是 §279 的五类（外加 §70 的两类），右边是 §280 的四个内容字段。映射不是
一一对应——Conversation／Project／Tool results 三类共用一个 `references[]`，靠 `Reference` 的变体与
provenance 区分。**这是本设计对 §280 缺字段的处置**（§2.4 第 3 条），不是「§280 允许三类混在一起」的
规范断言。

**历史用户轮与当前指令的数组选择（P6B 的 I4/I5）**：`UserInstruction { historical: false }` 只进
`instructions[]`；`{ historical: true }` 只进 `references[]`；`USER_INSTRUCTION` 不进 `artifacts[]` /
`memories[]`（P6B §3.6）。**本块的容器选择按 `historical` 位**：当前指令的候选放 `instructions`，
历史轮的候选放 `references`；判定由 P6B 的 `validate` 在 J3 复核（§8.1）。

**provider 供给的文本/工具 schema**：`provider_supplied()` → `WEB_UNTRUSTED`（data-only），走
`references[]`；它不是 `TRUSTED_RUNTIME`（P6B §4.2 第 4 行、§3.3 `:362`、§12 第 2 条）。本块用 `Reference::Text`
承载它，**不新造 `Reference` 变体**。

**「外部内容不得被提升为指令」的边界（必须写清）**：本块的**容器隔离**只保证「`references` 容器里的
候选不会出现在 `instructions[]` 里」。它**不判**「调用方放进 `instructions` 容器的候选是不是真的
用户指令」——那要靠 P6B 的 provenance／标签与 P2 的拒绝。**本块不调用 P6B 的
`ContextTrust::may_sit_in_instructions` 谓词**：切分把「外部内容与用户指令不同级」的判定判给 P6B
（§6.4 判据 2），本块重复判就是第二份判据。
**代价**：一个把 `WEB_UNTRUSTED` 候选错放进 `instructions` 的调用方，本块不会拦；这是调用方与 P6B／P2
的缺口，不是本块能补的（§11 第 20 条）。

## 5.5 §69 的隔离：无隐藏来源

`compile` 的输入只有三个：`request`、`budget`、`memory`。它**不读**：

- 全局变量、环境变量、当前目录；
- 数据库、`ArtifactStore`、`MemoryClaim` 的存储（只经 `MemoryRetrieval`）；
- 任何「当前 Agent」「当前会话」的隐式状态。

**因此**：「不同 Agent 默认拥有不同上下文」（§69 `:324`）在本块的兑现是——**调用方传什么，就只编什么**。
两个 Agent 的隔离由调用方构造两个不同的 `ContextRequest` 实现；本块不合并、不补全、不跨请求缓存。
**可测试**：两份逐字段相同的请求（J7b）在同一检索面下产物相等；请求之外的 `ArtifactId` 绝不出现在
产物里（J7a）。

**隐私裁剪不在本块**：§182 的 Personal Memory 放置与 capability 判定在 P2／第 4 层；本块只按调用方
给的 `scopes` 取记忆，不默认包含 `Personal`，也不替调用方做权限检查（§11 第 16 条）。

**capability 判定不在本块**：§69 的「Context 也属于 capability」需要一个判定「这个 Agent 能不能拿到
这个来源」。本块的口径是**判定在外**（第 4／5 层），`compile` 接收的是**已授权的候选集合**。
理由：①切分 §八.2 第 3 条把「判定在 Compiler 内部还是外部」记为 C4／OPEN-016，未裁定；②P6A 的判据
只有 §6.4 判据 1，没有 capability 判据；③若判定在内，本块要接 `continuum-capability` 的类型并重审
是否纳入 Protected Core（ENG-008），那是另一条边。**这一口径与 P6D 必须一致**（§11 第 9 条）。

## 5.6 失败面

- **唯一的失败**：`RetrievalQuery::new`（构造拒绝）或 `MemoryRetrieval::retrieve`（检索失败）返回
  `Err(MemoryError)`；`compile` 原样透传 `Result<ContextPackage, MemoryError>`。**不包装**成
  `ContextError`：判定是同一件事（P6C 的检索面拒绝），包装只会造第二枚错误类型（P5c 设计 §3.2 的
  同一条判据）。**照片 J13**：两条——(a) `RetrievalQuery::new([Project], None, ...)` ⇒
  `MemoryError::AnchorRequired`；(b) 假实现返回 `MemoryError::EmptyScopeSet` ⇒ `compile` 返回
  **同一臂**；两条都断言 `Err` 分支不产生 `ContextPackage`。
- **不产生失败的情形**：预算为 0、候选为空、claim 重复、artifact 重复、`scopes` 为空、
  `max_memories == 0`、`memory_query == None`——这些都有确定的产物（空数组／去重后的数组），不是错误。
  **`scopes` 空与 `max_memories == 0` 都不调检索面**（§5.3 第 5 条），故 P6C 的 `EmptyScopeSet`
  不会由本块的正常输入触发；`limit = 0` 在 P6C 是 `Ok(vec![])`（P6C §6.1），本块在 0 时不调检索。
- **P6C 的检索面已冻结为 `Result`**（P6C §3.5）：`compile` 按可失败写，**不保留**「不可失败、
  返回 `Vec<MemoryClaim>`」的备选；那会让 P6A 与 P6C 的接口分叉。

---

# 6. 消费契约与执行器接入

## 6.1 消费方读什么

`ContextPackage` 的消费方有两类，读的是同一枚值的不同字段：

| 消费方 | 读什么 | 用途 | 本块给什么 |
|---|---|---|---|
| **P2 的拒绝点**（第 5 层） | `trust_labels[]`（与 `provenance[]`） | §281 的「外部内容 MUST NOT 与用户指令同级」的拒绝行为（§6.2 `:380`） | 标签的承载物；**不判、不拒**（§3.5） |
| **执行器／模型调用路径**（P1／P3） | `instructions[]` / `references[]` / `artifacts[]` / `memories[]` | 投影为模型输入或执行输入 | 内容与结构；**不投影**（§6.2） |

**读标签的偏移规则**：按 §4.2 的声明序拼接。只读 `references()` 的消费方读第 `j` 条引用的标签时，
下标是 `instructions.len() + j`。本块不提供按字段切片的标签访问器（§4.2 第 6 条）。

## 6.2 为什么不由 P6A 投影为 `Message`

**事实（逐条实测）**：

- `Message` 只有 `role` 与 `content` 两个字段（`crates/continuum-core/src/model.rs:38-41`），**没有**
  承载 provenance／trust_labels 的位置。
- `InvokeRequest` 只有 `model` / `messages` / `max_tokens`（同文件 `:44-48`）；P3 的模型调用路径
  `CallInput` 只有 `messages: &[Message]` 与 `max_tokens`（`crates/continuum-runtime/src/model_call.rs:373-379`），
  且文档逐字写「**原样搬运**：不重排、不改写、不补一条系统消息」（同文件 `:374`）。
- §9.4（`docs/02-工程.md:635`）把这条跨层载荷写成「Context Package 的 provenance 与 trust_labels
  **字段**」——不是「Messages 的某个字段」。

**结论**：若 P6A 把 package 投影成 `Vec<Message>`，标签在模型边界**必然丢失**，§9.4 的载荷就不存在了。
故本块**不提供** `to_messages()` / `into_invoke_request()` / 任何序列化到 §315 类型的函数。

**P6A 产出的是结构化的 `ContextPackage`**（§280 六字段）；**§70 的 `Model Context Package`
（`docs/spec/02-positioning.md:382-386`）是消费方投影后的模型面产物，本块不交付**。两者不是同一个
对象：本块的包带 provenance／trust_labels，投影后的模型面产物是否保留这两个字段由消费方决定
（本块不规定投影规则）。这一处对应关系是**本设计的读法**，不是规范明文；若规范维护者裁定两者同名
同物，改的是本节的措辞与 §2.4 的登记，不是六字段。

**投影规则规范未给**：六个字段如何变成模型的 system／user／assistant 消息、引用与工具结果放在哪一段、
标签是否要变成 prompt 里的前缀——这些一处规范都没有。本块不发明；投影由消费方（执行器／模型调用路径）
自己做，或者由后续的规范／设计另立（§11 第 8 条）。

**代价**：消费方要自己写投影；`ContextPackage` 不能直接喂给 `ModelProvider::invoke`。这是本设计取
「标签不丢」的代价，不是省事。

## 6.3 执行器接入：两个候选，本设计不裁定

§279 的「节点执行前 MUST 通过」要落到执行器。今天执行器的入口是
`OperatorImpl::execute(&self, inputs: &[ArtifactRef], ctx: &NodeContext)`
（`crates/continuum-graph/src/execution.rs:76-82`）；`NodeContext` 的字段是 `node` / `profile` /
`cancelled`（同文件 `:43-47`），**没有装 `ContextPackage` 的位置**。两个候选：

| 候选 | 形状 | 代价 | 与切分 §四第 1 条的关系 |
|---|---|---|---|
| **(a) 执行器直接收 package** | `NodeContext` 增一个 `context: &ContextPackage` 字段，或 `execute` 增一个参数 | `continuum-graph` 必须依赖 `continuum-context`——**既有行 `continuum-graph` 增枚**，切分 §四第 1 条禁止（除 `continuum-runtime`） | 需要协调者裁定「例外是否扩到 graph」，或把 `ContextPackage` 放到 `continuum-core`（但那违背切分 §2.3 的「归 P6A」） |
| **(b) 装配方接线（未来）** | 只有在 P1 的节点执行器落地后，`continuum-runtime` 才有一个可接线的**节点级前置点**：在它调用执行器之前调 `compile`，把 package 留在装配层；执行器不收它，模型调用／标签检查由 runtime 从 package 取值。**本轮 runtime 没有节点执行路径，故这不是一个现成挂点** | 「必经」落在 runtime 的**调用序**上，不在类型层；执行器若绕过 runtime 直接跑，类型层拦不住 | **不增任何既有行**；`continuum-runtime` 是唯一允许增枚的既有行（切分引 `:206`；实测 `:392`） |

**本设计建议 (b) 作为本轮不新增反向边的落法**：(a) 的类型强制更强，但它要改 `ALLOWED` 的例外或
`ContextPackage` 的归属，两者都不在本块的处置权里。**这一条不裁定，报给 P1 ＋ 协调者**（§11 第 1 条）。

**无论 (a) 还是 (b)**：`compile` 都是唯一构造点；「执行器不调 `compile`」在类型层拦不住——这是
§6.4 判据 1 的**残余**，要等执行器落地后由「执行入口只接受 `ContextPackage`」或「runtime 的调用序」
补上。本设计不把「类型层唯一构造点」写成「端到端必经」（§8.4 第 1 条）。

## 6.4 本轮能拍到哪一步（据实）

| 层次 | 能拍到 | 照片 |
|---|---|---|
| 类型层 | 字段私有、唯一构造点、无 `Default`／`Deserialize`／`new` | `trybuild` 三条 compile_fail（§8.2 J1） |
| 源码面 | `ContextPackage` 的构造只出现在 `compile`；无标签字面量；无 `MemoryClaim` 定义 | `grep` 三条（§8.2 J1、J9） |
| 运行期 | 五条不变量、五来源映射、预算／去重／容器隔离 | 单元测试（§8.2 J2–J11） |
| 端到端 | **拍不到**：执行器未建，§279 的「节点执行前必经」没有生产调用点 | ——（§8.4 第 1 条） |

---

# 7. 跨块接口

## 7.1 P6B（信任标签的类型与附加点）

**本块给 P6B 的**：

- `ContextPackage` 的 `provenance[]` 与 `trust_labels[]` 两个字段（元素类型是 P6B 的）；
- `ContextCandidate<T>` 的 `provenance` 字段（P6B 的类型）；
- 装配时**逐片段**调用 P6B 的 `label`，把返回值原样装进 `trust_labels[]`——P6A 不缓存、不覆盖、不合并。

**P6B 给本块的**（一页接口，已冻结）：

1. `ContextProvenance`：单片段粒度，八个变体（`UserInstruction { historical }` / `TrustedRuntime` /
   `ProjectData` / `WebUntrusted` / `ToolOutput { external }` / `Memory { claim_id }` / `ModelOutput` /
   `Artifact { producer }`；P6B §3.2）。**两个 `UserInstruction` 构造入口**：`user_instruction()`
   （`historical: false`，当前指令）与 `conversation_history()`（`historical: true`，历史用户轮）；
   `provider_supplied()` 是 `WebUntrusted` 的具名入口（第三方 provider 文本/工具 schema），语义同
   `web_untrusted()`（P6B §3.2、§4.2 第 1a/1b/4 行、§3.3 `:362`）；
2. `ContextTrust`：§281 的七个取值，一份（P6B §3.1）；
3. `label(&ContextProvenance) -> ContextTrust`：全函数、确定性、无 I/O（P6B §3.3）；
4. `ContextProvenance::memory(claim.id().as_str())`：`memory` 收 `impl Into<String>`；本块传
   `claim.id().as_str()`（P6B §3.2；P6C 的 claim 访问器见 §7.2）；
5. `ContextTrust::may_sit_in_instructions(&self) -> bool`（P6B §3.1 的公开面）：若 P6B 希望本块在
   装配时拒绝「非指令候选进 `instructions`」可以用它；本设计**不要求**（§5.4 末），若用，本块加一条检查；
6. `PackageShape` / `validate` / `TrustError`（P6B §3.4、§3.5）：**J3 的独立复核面**（§8.1 J3）。

**已冻的**：类型名、七标签、`label` 的签名（P6B §3.1–§3.3）。切分 §四第 6 条要求
「P6B 先冻、P6A 对着写」——**P6B 已冻，本块按实际名写**，不再等待。

**`validate` / `PackageShape` 在本块的调用清单里，只有 J3 一处**：P6B §5.2 第 5 条逐字要求
「P6A 的 J3 改为调用 `validate(PackageShape { .. }, &provenance, &trust_labels)`，不再自写第二份
长度/逐位断言」。**`compile` 的生产路径不调用 `validate`**（P6B §5.2 第 5 条：本轮不要求；本块的
对齐由 §4.2 第 5 条的同步 push 保证），`validate` 也不是第二个构造点。J3 拿到的 `Err` 不得继续消费
这个包（P6B §3.4）。

**边**：`continuum-context → continuum-trust`（普通，层内边）；**P6B 不依赖本块**（它只定类型，不需要
`ContextPackage`；若它要引用 `ContextPackage`，那就是环，报协调者）。

**若 P6B 日后把标签类型搬进 `continuum-core`**：本块的边改为 `continuum-core`，`ALLOWED` 行少一项；
「标签类型一份」不变。这是 P6B 与协调者的裁定，不是本块能定的。

## 7.2 P6C（MemoryClaim 与检索面）

**本块给 P6C 的**：无（本块不写 P6C 的表、不产生 claim、不做合成／纠正）。

**P6C 给本块的**：

1. `MemoryClaim`（§283 的七个字段：`id`、`scope`、`statement`、`sources[]`、`validity`、`confidence`、
   `last_confirmed`；`docs/spec/05-normative.md:1579-1609`）；**它要实现 `Clone` ＋ `PartialEq` ＋
   `Debug`**——本块的 `ContextPackage` 的 `#[derive(Debug, Clone, PartialEq)]` 依赖这三者（§4.1）；
   **读访问器已冻结**：`id() -> &MemoryClaimId`、`scope() -> MemoryScope`、`statement() -> &str`、
   `sources() -> &[MemorySourceId]`、`validity() -> Validity`、`confidence() -> &Confidence`、
   `last_confirmed() -> i64`（P6C §3.5）。本块只用 `id()`，消费方读其余字段；
2. `MemoryScope`（§284 的五档）；`anchor` 是 `Option<String>`，与 `scopes` 的规则在 P6C §5.4、由
   `RetrievalQuery::new` 执行（`P6C §6.1`）；
3. `MemoryError`（检索失败的透传类型；P6C §3.5 的臂）；
4. `MemoryRetrieval`：对象安全、只读的检索面，**已冻结**为
   `fn retrieve(&self, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError>`（P6C §3.5）；
5. `RetrievalQuery`：字段私有，唯一构造点 `RetrievalQuery::new(scopes, anchor, text, limit,
   include_terminal) -> Result<Self, MemoryError>`；`scopes`（非空）、`anchor: Option<String>`、
   `text: Option<String>`、`limit`、`include_terminal`（P6C §6.1）。
   本块按 §5.3 第 5 条构造它；`include_terminal` 写死 `false`。

**已冻的**：三枚类型（`MemorySource`／`MemoryClaim`／`MemoryScope`）、检索面与 `RetrievalQuery`
（P6C §3.5、§6.1）。切分 §四第 6 条要求「P6C 先冻、P6A 对着写」——**P6C 已冻，本块按
`&RetrievalQuery` 写**，不再保留 `&str, usize` 的备选。**`MemoryClaim` 的读访问器也已冻结**（P6C §3.5）。

**边**：`continuum-context → continuum-memory`（普通，层内边）；**P6C 不依赖本块**。

**为什么不再保留 `Vec<MemoryClaim>` 备选**：P6C 已冻结唯一检索 trait（P6C §12.2 逐字「P6C 不提供
第二枚检索 trait」）；若本块另收 `Vec<MemoryClaim>`，§279 的「从 Memory 取料」就被拆成「调用方取 +
P6A 选」，且调用方要为每个 claim 构造 provenance。本设计取 P6C 的 trait，**`compile` 内构造
`RetrievalQuery`**（P6C §12.2 的建议接法）。

## 7.3 P1（执行器）

**本块给 P1 的**：`compile` 与 `ContextPackage`；§6.1 的消费契约；§6.3 的两个候选。

**P1 给本块的**：

- 节点执行前的**调用点**（谁在 `Running` 之前调 `compile`）；
- `NodeContext` 的扩展（若选 (a)）或 runtime 的调用序（若选 (b)）；
- 「执行入口只接受 `ContextPackage`」的类型强制（若选 (a)）。

**未决（具名收件人）**：切分 §八.1 第 2 条要求 P6A **具名回报**「§279 的 MUST 通道今天没有调用方」。
本设计的回报是：**调用方是 P1 的节点执行器；它未建**。`continuum-runtime` 里**没有节点执行路径**
（`OperatorImpl::execute` 只有 trait 定义与测试实现；`task_cmd` 是命令／效应路径），故它今天**不是**
一个可接线的节点级前置点；只有在 P1 的节点执行器落地后，runtime 才可能有一个可接线的位置。P3 的模型
调用路径是 **per-invocation**，不是 §279 的「Node 执行前」。收件人：P1 ＋ 协调者（§11 第 1 条）。

## 7.4 P3（模型调用路径）

**本块给 P3 的**：`ContextPackage` 的内容与标签；**不提供**到 `InvokeRequest` / `Message` 的投影
（§6.2）。

**P3 给本块的**：`CallInput.messages: &[Message]`（`crates/continuum-runtime/src/model_call.rs:373-379`）
——**没有标签位置**。P3g 设计 §1.1 明写「模型调用没有任何强制点」；P6A 的 `compile` 不是 G 的强制点，
它发生在 G **之前**（节点执行前）。

**未决**：标签怎么过模型边界？本块的立场是「不压成 Message」（否则 §9.4 的载荷丢失）；若 P3 要在
`InvokeRequest` 里加标签位置，那是 §315 接口的改动（切分 §2.2：P6 不碰 §315），要由 P3 ＋ 协调者裁。
收件人：P3 ＋ 协调者（§11 第 8 条）。

## 7.5 P2（拒绝行为）

**本块给 P2 的**：`ContextPackage` 的 `provenance[]` / `trust_labels[]`（§281 载荷的承载物）。

**P2 给本块的**：拒绝行为（§6.2 `:380`：「标签的附加在本层，拒绝行为在第 5 层」；§5.2 `:310`：
「强制点在本层而非第 6 层」）。

**未决**：P2 的 crate 不能依赖 P6B 的 crate——`ALLOWED` 的既有行不因 P6 增枚（切分 §四第 1 条），而 P2
的拒绝点要读 P6B 的类型。两条路：①标签类型放 `continuum-core`——但 P2 的四个 crate
（`continuum-workspace`／`continuum-sandbox`／`continuum-effect`／`continuum-policy`）的 `ALLOWED` 行
**都不含 `continuum-core`**（实测 `crates/continuum-runtime/tests/dependency_direction.rs` 的对应行），
放 core 仍要各自增枚；②runtime 把标签以值／基本类型传给 P2 的已有接口，P2 不引用 P6B 的类型。
**两条都要改某个既有行的依赖面或加一层转换**，不是本块能定的。收件人：P6B ＋
P2 ＋ 协调者（§11 第 10 条）。**本块不建拒绝、不判级别**。

## 7.6 P5（六个领域 crate 与 P5a 的证据）

**P5 六 crate 是原料的生产者，不是 `ContextPackage` 的消费者**：`continuum-code` / `continuum-media` /
`continuum-research` / `continuum-image` / `continuum-verify` / `continuum-method` 声明算子、产
`Artifact`；本块只收 `ArtifactId`，**不依赖它们**，它们也**不依赖本块**（零边）。它们的算子执行体
（`OperatorImpl`）未建（P5 的设计逐块写明「只注册、不执行」），故今天没有「算子执行时消费上下文」
这件事；将来若有，那是执行器的投影（P1），不是 P6A 的依赖。

**与 P5a 的唯一交点：`ToolResult` 的两个消费者**（切分 §四第 4 条）。P5a 的
`Evidence::from_tool_result`（`crates/continuum-verify/src/evidence.rs:248`）收八个字段，不直接收
`ToolResult`；本块的 `Reference::ToolOutput(ToolResult)` 直接装整枚 `ToolResult`（`output` + `is_error`）。
**两者不共享 wrapper、也不各造 wrapper**：P5a 的构造点与 P6A 的 `Reference` 各自直接读 core 的类型。
P5a 设计 `:1051` 的对账条目 (ii) 已写「两者不共享 wrapper」；本设计接上这一条，**不请求 P5a 改任何形状**。
**生产端的缺口**：P6B 的 `ContextProvenance::ToolOutput { external }` 需要一个 `external` 判别位，而
`ToolResult` 今天没有它（§11 第 18 条）；artifact 的 `ContextProvenance::Artifact { producer }` 需要
`ArtifactProducer`，而 `Artifact.producer_node` 是无类型的 `Option<String>`（§11 第 19 条）。本块只给
**承载形状**，不补生产端。

**`Evidence` 不进 `ContextPackage`**：§279 的五类来源里没有 Evidence；§6.3 的层内依赖是
`Context Compiler ← MemoryClaim + Artifact + Conversation`，也没有 Evidence。P5a 的证据是验证的输入
（§263），不是模型上下文。若将来要让模型看证据，那是 `references[]` 的一个 `Reference::Text` 投影，
由调用方决定，不是本块新增字段。

---

# 8. 判据与照片

## 8.1 判据表（每条挂规范节）

| # | 判据 | 规范节 | 观测形态 |
|---|---|---|---|
| **J1** | `ContextPackage` 只有一个构造点（`compile`）；外部不能字面量／`Default`／反序列化构造；源码里 `ContextPackage` 的构造只出现在 `compile` | §6.4 判据 1 `:402`；§279 `:1502` | 编译期（`trybuild`）＋ 源码面（`grep`） |
| **J2** | 六字段的名字与书写序逐字等于 §280；字段私有；只有六个只读访问器 | §280 `:1524-1535` | 源码面（`sed` 六行）＋ 编译期（外部读写字段编不过） |
| **J3** | 包的 `provenance[]`／`trust_labels[]` 与四数组通过 P6B 的 `validate`（长度、逐位映射、指令位／数据位类别）；**J3 调 `validate`，不自写第二份长度／逐位断言**；保留「换 provenance 后标签跟着变」；**当前指令（`historical: false`）只落 `instructions[]`、历史轮（`historical: true`）只落 `references[]`**（I4/I5，由 `validate` 的 `DataSlotCarriesUserInstruction`／`HistoricalInstructionInInstructions` 复核） | §280；§281 `:1540`；P6B §3.4、§3.6 I4/I5、§5.2 第 5 条 | 运行期（`validate` 正例＋反例） |
| **J4** | 五类来源都能进包：Conversation → `references[]`、Memory → `memories[]`、Artifacts → `artifacts[]`、Project → `references[]`、Tool results → `references[]`；**另加 provider 供给**（`provider_supplied()` → `references[]`，标签 `WEB_UNTRUSTED`） | §279 `:1507-1519`；§6.3 `:391`；P6B §4.2 第 4 行、§3.3 `:362` | 运行期（每类一条正例） |
| **J5** | 预算生效：三个数组分别不超过三个上限；被截断的候选不在包中；同一 `ArtifactId` 只出现一次；同一 claim id 只出现一次 | §279「最小必要」；§70 `:391`、`:393` | 运行期（正例＋反例） |
| **J6** | 容器隔离：`references`／`artifacts`／`memories` 的候选不会出现在 `instructions[]`；`instructions` 只来自 `request.instructions` | §70 `:394`（降低 prompt injection 面）；§281 `:1552`；P6B §3.6 | 运行期（把外部候选放 references，断言 instructions 空） |
| **J7a** | 无隐藏来源：`compile` 只依赖三个参数；请求外的 `ArtifactId`／claim 不出现；没有全局状态 | §69 `:324`；§70 `:361` | 运行期（请求外来源的负例） |
| **J7b** | 确定性：两份逐字段相同的请求 ＋ 同一检索面，产物逐字段相等 | §69 `:324`；§70 `:361`（「纯函数／可重复」是本设计的判定，§2.4 第 12 条） | 运行期（两次调用对照） |
| **J8** | `memories[]` 只来自 `MemoryRetrieval`；检索面未被调用时 `memories[]` 空；本块没有 `continuum-persist` 边、不读写 claim 存储 | §6.3 `:391`；§279 的 Memory | 源码面（依赖表）＋ 运行期（假实现的返回值与调用计数） |
| **J9** | `provenance[]`／`trust_labels[]` 的元素类型是 P6B 的类型；本块 `src/` 里没有 `enum ContextTrust` / `enum ContextProvenance` / `struct MemoryClaim` / `enum MemoryScope`；没有标签构造字面量；`ContextTrust` 只经 `use continuum_trust` 引入 | §281 `:1538-1554`；切分 §2.3 | 源码面（`grep`）＋ 编译期（`use` 的来源） |
| **J10** | 空请求 ⇒ 六数组皆空；有引用无指令 ⇒ `instructions[]` 空、`references[]` 非空 | §280 的六字段都是数组（空数组是本设计的边界用例；规范未禁止） | 运行期（两条边界） |
| **J11** | 指令全收：非空指令全部进包，不受预算约束；预算只作用于 references／artifacts／memories | §279「最小必要」；§281 `:1552` 的「外部内容 MUST NOT 与用户指令处于同一信任级别」（**方向依据**，不是「指令不可裁」的明文） | 运行期（预算 0 时指令仍在） |
| **J12** | 端到端「节点执行前必经」 | §6.4 判据 1 `:402` | **无照片**（执行器未建，§8.4 第 1 条） |
| **J13** | P6C 的 `MemoryError` 原样透传：`RetrievalQuery::new` 的构造拒绝与 `retrieve` 的检索失败各一条 ⇒ `compile` 返回**同一臂**，且 `Err` 分支不产生 `ContextPackage` | §6.3 `:391`（检索失败由检索面带来；透传是本设计的契约，§5.6） | 运行期（构造与检索两条错误路径） |
| **J14** | `max_memories == 0` 或 `scopes` 为空 ⇒ **不调检索面**、`memories[]` 为空（`limit = 0` 在 P6C 是 `Ok(vec![])`；本块在 0 时不调检索） | §279「最小必要」；P6C §6.1 的 `limit` 语义 | 运行期（假实现调用计数为 0） |

**照片的 scope 约定**：凡涉及记忆检索的照片（J3／J4／J5／J7a／J7b／J8／J11／J13-b／J14），`scopes`
一律用 `vec![Personal]`（P6C §5.4：`Personal` 无 anchor，`new` 不会因缺 anchor 拒绝）；涉及 anchor 的
照片（J13-a）单列 `[Project] + None`。这样每张照片不会因 scope／anchor 规则而红在与判据无关的地方。

## 8.2 照片清单（每条判据的观测形态：运行期／编译期／源码面）

| # | 照片（具体断言或命令） | 形态 | 拍不到的部分 |
|---|---|---|---|
| **J1-a** | `trybuild`：`ContextPackage { instructions: vec![], references: vec![], artifacts: vec![], memories: vec![], provenance: vec![], trust_labels: vec![] }` 在 crate 外**编不过**（字段私有） | 编译期 | 拦不住「执行器不调 compile」 |
| **J1-b** | `trybuild`：`ContextPackage::default()` 编不过（无 `Default`） | 编译期 | 同上 |
| **J1-c** | `trybuild`：`serde_json::from_str::<ContextPackage>(...)` 编不过（无 `Deserialize`） | 编译期 | 若将来加序列化，这条要改；本设计不主张加 |
| **J1-d** | `grep -rn "ContextPackage {" crates/continuum-context/src` 只命中定义处与 `compile` 内的一处构造；**单元测试放 `crates/continuum-context/tests/`，不放 `src/` 的 `#[cfg(test)]`**，故 `src` 语料里没有测试构造；`grep -rn "impl Default for ContextPackage\|Deserialize.*ContextPackage" crates/continuum-context/src` 零命中 | 源码面 | 零命中类断言会随注释漂（切分 §六的教训）；本条的语料限定 `crates/continuum-context/src` 的非测试行，并在实现期逐处读命中行 |
| **J2** | `sed -n` 读出 `ContextPackage` 的六行字段，逐行与 §280 的六个名字比对；`trybuild`：外部写 `package.instructions = ...` 编不过 | 源码面＋编译期 | 字段顺序在运行期不可见；靠源码面 |
| **J3** | 正例：`compile` 产出 1 条当前指令（`user_instruction()`）＋1 条历史用户轮（`conversation_history()`，进 references）＋1 条 `Reference::Text`（`web_untrusted()`，进 references）＋1 个 artifact＋1 条 claim；用 `PackageShape { instructions: 1, references: 2, artifacts: 1, memories: 1 }` 调 `continuum_trust::validate(shape, pkg.provenance(), pkg.trust_labels())`，断言 `Ok`；行为断言：把**那条 `Reference::Text`** 的 provenance 从 `web_untrusted()` 换成 `project_data()` 重编译，断言 references 位上的标签由 `WEB_UNTRUSTED` 变为 `PROJECT_DATA`。反例（直接调 `validate`，不构造包）：(a) `provenance=[WebUntrusted]`、`trust_labels=[PROJECT_DATA]` ⇒ `LabelDoesNotMatchProvenance`；(b) `UserInstruction { historical: false }` 放 references 位 ⇒ `DataSlotCarriesUserInstruction`；(c) `UserInstruction { historical: true }` 放 instructions 位 ⇒ `HistoricalInstructionInInstructions`（P6B §9.1 (j)/(j2) 的跨块复测） | 运行期 | 同类别互换（P6B §3.4 边界）；`validate` 抓不到内容来源本身（P6B §9.5 第 1 条） |
| **J4** | 六条正例：①`Reference::Conversation`（`role == User` 的历史轮，`conversation_history()`）进 `references[]`；②`MemoryClaim` 进 `memories[]`；③`ArtifactId` 进 `artifacts[]`；④`Reference::Text`（Project，`project_data()`）进 `references[]`；⑤`Reference::ToolOutput`（`tool_output(false)`）进 `references[]`；⑥`Reference::Text` ＋ `provider_supplied()` 进 `references[]`，标签为 `WEB_UNTRUSTED`（不是 `TRUSTED_RUNTIME`）。每条断言「在对应字段、不在别的字段」 | 运行期 | Project State 的结构化读取（只收文本）；provider 身份的真实判别（P6B §12 第 2 条） |
| **J5** | 预算 `(1,1,1)` ＋ 2 条**不同**的 references／2 个**不同 id** 的 artifact／2 条 claim：断言各取 1 条、被截断的那条内容不在包中；**另设一例**：2 个 artifact 的 id 相同、预算 2，断言只出现一次（去重） | 运行期 | token 预算（本块不做） |
| **J6** | 把一条 `Reference::Text`（provenance 取 `ContextProvenance::web_untrusted()`，P6B 的构造入口）放进 `references`，`instructions` 为空：断言 `instructions().is_empty()`；再把它放进 `instructions`，断言本块**不拦**（记录这是调用方的错，§11 第 20 条） | 运行期 | 「外部内容不得进 instructions」的判定（本块不判） |
| **J7a** | 请求只含 artifact A；假检索面返回 claim C：断言产物只含 A 与 C，不含请求外的 B；`compile` 的签名只有三个参数（无全局来源） | 运行期 | 「这个 Agent 有没有权限拿这个来源」（capability，不在本块） |
| **J7b** | 手写两份**逐字段相同**的 `ContextRequest`（不用 `Clone`），配同一个假检索面，调两次：断言 `p1 == p2`（`ContextPackage` 派生 `PartialEq`） | 运行期 | 检索面实现自身的隐藏状态（假实现无状态） |
| **J8** | 源码面：`crates/continuum-context/Cargo.toml` 无 `continuum-persist`；`ALLOWED` 行无它；运行期：假实现用 `Cell<usize>` 计数，返回的 claim 逐枚等于 `memories[]`；`scopes` 空或 `max_memories == 0` 时计数为 0、`memories[]` 空 | 源码面＋运行期 | 检索面内部的 scope 过滤（P6C 按调用方给的 `scopes`）；隐私判定（P2／第 4 层） |
| **J9** | `grep -rn "enum ContextTrust\|enum ContextProvenance\|struct MemoryClaim\|enum MemoryScope" crates/continuum-context/src` 零命中；`grep -rn "ContextTrust::" crates/continuum-context/src` 只命中 `use`／签名，不命中构造字面量；`grep -rn "use continuum_trust" crates/continuum-context/src` 至少一处（类型来源是 P6B）；`grep -rn "impl .*ContextTrust\|impl .*ContextProvenance" crates/continuum-context/src` 零命中 | 源码面 | 若 P6B 把类型放 core，`ContextTrust::` 的语料要改 |
| **J10** | `ContextRequest { instructions: vec![], references: vec![], artifacts: vec![], scopes: vec![], anchor: None, memory_query: None }` ＋ 预算任意：断言六个访问器都返回空切片 | 运行期 | —— |
| **J11** | 预算 `(0,0,0)` ＋ `scopes` 非空 ＋ 2 条非空指令：断言 `instructions().len() == 2`，`references/artifacts/memories` 全空；假实现计数为 0（与 J14 可共用一次） | 运行期 | 指令过长时的 token 成本（本块不做） |
| **J12** | **无照片** | —— | 执行器未建；`NodeContext` 无 package 位置（§6.3） |
| **J13** | (a) `scopes = vec![Project]`、`anchor = None`：`RetrievalQuery::new` 返回 `Err(MemoryError::AnchorRequired)`，`compile` 返回**同一臂**；(b) `scopes = vec![Personal]`、假实现返回 `Err(MemoryError::EmptyScopeSet)`：`compile` 返回**同一臂**；两条都断言 `Err` 分支没有 `ContextPackage` 逃出 | 运行期 | 其它 `MemoryError` 臂逐臂对照（本块只测这两条代表路径） |
| **J14** | `scopes = vec![Personal]`、`max_memories = 0`、`memory_query = Some("x")`：断言 `memories[]` 空、假实现计数 0；反例：`max_memories = 1` 时计数 1 | 运行期 | P6C 的 `limit = 0` 分支本身（本块在 0 时不调检索） |

## 8.3 变异预告（谁红）

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`compile` 返回一个空包（不装任何候选） | J4、J5 的正例、J11 | J1（构造点仍在）、J2、J10、J7b（两份空包仍相等） |
| **取反**：`instructions` 也受预算截断 | J11 | J5（references／artifacts／memories 的预算仍对） |
| **放宽**：`ContextPackage` 加 `pub` 字段或 `Default` | J1-a／J1-b | J3–J11（运行期断言仍可过，故 J1 是唯一抓构造点的） |
| **放宽**：`trust_labels` 改成「整包一个标签」（长度 1） | J3（`validate` 的 `LengthMismatch`／`ShapeMismatch`） | J4、J5、J10（它们不读标签长度） |
| **取反**：把 `references` 的候选也 push 进 `instructions[]` | J6（容器隔离）、J4（「在对应字段、不在别的字段」那条）、J3（`validate` 的 I4：`InstructionSlotCarriesDataOnlyLabel` 或 `HistoricalInstructionInInstructions`） | J5（预算） |
| **取反**：把 `label(provenance)` 换成写死的 `USER_INSTRUCTION` | J3 的反例（`validate` 的 `LabelDoesNotMatchProvenance`） | J3 的正例（若所有候选同源，正例可能仍绿——这是为什么 J3 要成对） |
| **移除**：J3 不调 `validate`、改回直接断言长度／逐位 | J3 的 (b)(c) 两条类别反例（`validate` 的 I4/I5 没有被测） | J3 的正例（直接断言仍能过） |
| **移除**：`memories` 不调检索面、永远空 | J4 的第②条、J8 | J14（它测的是 0 不调，永远不调反而绿）、J5 的 memories 预算（0 条仍在预算内） |
| **取反**：`max_memories == 0` 仍调检索面 | J14（计数 > 0） | J5 的正例（预算 1 的那组） |
| **取反**：把 `MemoryError` 包装成自定义错误 | J13（同一臂的断言） | J1–J11（不涉及错误路径） |

## 8.4 拍不出照片的地方

1. **端到端「节点执行前必经」**：执行器未建；`OperatorImpl::execute` 不收 `ContextPackage`，
   `NodeContext` 没有它的位置（§6.3）。本块的全部照片是类型层、源码面与单元／集成层。
   与 P5c 设计 §10.5 第 1 条、P5e 设计 §12.5 第 1 条同形。**收件人：P1 ＋ 协调者**（§11 第 1 条）。
2. **模型实际看到什么**：本块不投影为 `Message`（§6.2）；投影由消费方做，规范未给规则。
3. **标签的拒绝**：P2 的行为不在本块（§7.5）。
4. **「最小必要」的相关性**：候选序由调用方给；本块不判「这段是否相关」（§5.3）。
5. **token 预算**：本块用条数（§2.4 第 5 条）。
6. **`ContextPackage` 的跨进程／持久化**：不实现 `Serialize`／`Deserialize`（§4.3）；若将来需要，
   要另立受控入口（§11 第 14 条）。
7. **§70 的「降低隐私暴露」**：本块只按调用方给的 `scopes` 取记忆，不判隐私等级、不做放置裁决；
   本块没有这条目标的照片（§2.4 第 9 条的边界、§11 第 16 条）。**收件人：P2 ＋ 第 4 层 ＋ 协调者。**

---

# 9. 明确不做的（执行器／调度／模型调用等）

**这一节是 §1.2 的完整表**；每条给出归属与理由。**不建、不留桩。**

| 不建的东西 | 归属 / 理由 |
|---|---|
| 节点执行器、`Queued → Running` 的迁移、图调度、`OperatorRegistry::resolve` 的调用点 | P1（切分 §四第 4 条）。本块只提供 `compile` 与被消费的类型；`compile` 不读节点状态、不写事件 |
| 模型调用、`InvokeRequest` 的构造、`ModelProvider::invoke` / `stream` | P3（`crates/continuum-runtime/src/model_call.rs:436`、`:512` 已有生产构造点）。本块不调 Provider |
| 把 `ContextPackage` 投影为 `Message` / `InvokeRequest` / prompt 文本 | 规范未给投影规则；本块不发明（§6.2）。消费方自己做 |
| 七个信任标签的类型、附加点、`may_sit_in_instructions` 谓词 | P6B（切分 §五 P6B 行）。本块只调用 `label`／`ContextProvenance::memory` |
| 「外部内容与用户指令不同级」的判定、拒绝行为、策略、Audit | P6B（判定）＋ P2（拒绝，§6.2 `:380`；§5.2 `:310`）。本块不判、不拒 |
| `MemoryClaim` / `MemorySource` / 五个 scope / claim 存储 / 检索实现 / 合成 / 纠正 | P6C（切分 §五 P6C 行）。本块只消费 `MemoryRetrieval` 的返回 |
| Protected Core / Runtime Learning / §321 的五类语义 | P6D（切分 §五 P6D 行） |
| capability 判定（「这个 Agent 能不能拿这个来源」） | C4／OPEN-016 未裁定；本块口径是「判定在外」（§5.5）。若裁定在内，另立一条边 |
| tokenizer、token 计数、模型上下文窗口的检查 | 规范未给；本块用条数预算（§2.4 第 5 条） |
| 相关性打分、embedding、向量检索、rerank | 规范未给；本块用候选序（§2.4 第 4 条） |
| 表、迁移、事件、审计记录 | 本块零表（§10）；`ContextPackage` 是执行期值 |
| 缓存、复用判据、`cache_key` | P1 已落地（`crates/continuum-graph/src/reuse.rs:29`）；本块不建第二份 |
| 工具结果到 `Evidence` 的转换 | P5a（`crates/continuum-verify/src/evidence.rs:248`）；本块不建 wrapper（§7.6） |
| Conversation / Project 的存储与查询 | 仓里无落点；调用方投影为 `references`（§5.4）；上游缺口见 §11 第 17 条 |
| Prompt 模板、标签的 prompt 前缀、序列化格式 | 规范未给；本块不发明（§6.2） |

---

# 10. 持久化：**本块零迁移**

## 10.1 结论：不建表、不建迁移、不写事件

| 本块的产物 | 为什么不落库 |
|---|---|
| `ContextPackage` | 它是**节点执行期的值**：§279 的时点是「Node 执行前」，产物供该节点执行期使用；规范没有要求保存上下文包 |
| `ContextRequest` / `ContextCandidate` / `ContextBudget` | 装配期输入，不是持久对象 |
| `Reference` | 同上；`ToolResult` / `Message` 的生命周期由产生它们的层管 |
| 标签与 provenance | 随 `ContextPackage` 存活；§9.4 的载荷是「字段」，不是「表」 |

## 10.2 迁移号段：**本块不占档**

- 切分 §四第 2 条：P6 这一轮的档**由 P6C 的设计写死**（P6C 是四块里唯一确定要落库的块）；其余块若也要
  落库，号从同一档内顺取。
- 本块**零表**，故**不声明号、不占档**。`runtime_migrations()`（`crates/continuum-runtime/src/main.rs:110`）
  里**不加本块的行**。
- 实测占用（2026-10-17，本工作树）：`100`（canonical）／`110`（semantics）／`120`（budget）／
  `130`（verify）已注册；`140`–`199` 在 `crates/` 的 `Migration::new` 里**零命中**。P6C 的档由它自己
  现场核对空号后写死。
- **「我没占它」不等于「它没人占」**：本块只说「P6A 零表、不占档」，不替 P6C 声明。

## 10.3 事件

本块**不写任何 `continuum-events` 的事件**。§313 的必录清单里没有「上下文编译」这一类；`compile` 是
每次节点执行的内部步骤，不是审计事件。若将来要把「编译了什么」记入审计，那是 §313 的扩展与 P2 的
审计面，不是本块。

---

# 11. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。共 22 条。**

1. **「节点执行前必经」没有执行器挂点**（§6.3、§8.4 第 1 条）。今天 `OperatorImpl::execute` 收
   `inputs` ＋ `NodeContext`，后者没有 `ContextPackage` 的位置；`continuum-graph` 不能依赖本块
   （既有行不增枚）。**缺的是「执行器在何处、以什么签名收 `ContextPackage`」这一步。**
   **收件人：P1 ＋ 协调者。** 两条候选与代价见 §6.3；本设计建议 (b)（不增反向边的装配方接线），
   但 (b) **只有在 P1 的节点执行器落地后**才是一个节点级挂点——本轮本块以库形态交付，不裁定。
2. **§9.4 与 §9.1 的两种读法**（§2.3）。「认知层 → 执行层」这条载荷的消费者是谁、边从哪一侧登记，
   取决于读法；本块的口径是不登记指向执行层的代码边、接线经 runtime。**缺的是「这条边由哪一侧登记」
   的裁定。收件人：《工程》维护者 ＋ 协调者。** 本块不改规范。
3. **§280 的四类内容数组元素类型未给**（§2.4 第 1 条）。本设计定 `String` / `Reference` /
   `ArtifactId` / `MemoryClaim`。**缺的是「§280 的元素类型」这一条规范。收件人：规范维护者。**
4. **§280／§281 的对齐规则未给**（§2.4 第 2 条）。本设计定「按 §280 字段声明序拼接、等长同序」。
   **缺的是「`provenance[]`／`trust_labels[]` 与内容片段怎样对应」这一条规范。收件人：规范维护者。**
5. **§280 没有 Conversation／Project／Tool results 的专属字段**（§2.4 第 3 条）。本设计把它们放进
   `references[]`。**缺的是「这三类来源在六字段里的落点」这一条规范。收件人：规范维护者。**
6. **§70 的 Intent／Project State 与 §279 的五来源不同**（§3.4）。本设计以 §279 五类为 MUST，§70 的
   两类以文本投影。**缺的是「§70 的六来源与 §279 的五来源哪一张是 MUST」的裁定。收件人：规范维护者。**
7. **「最小必要」的相关性判据未给**（§5.3、§2.4 第 4 条）。本块用候选序，不做打分／检索。
   **缺的是「什么算相关」这一条规范。收件人：规范维护者。** 在它给出之前，本块的「最小必要」只能
   保证预算与去重，不能保证选中项的语义相关性。
8. **P3 的模型调用路径没有标签位置**（§6.2、§7.4）。`CallInput.messages` 是 `&[Message]`，P6A 不投影
   为 `Message`（否则 §9.4 的载荷丢失）。**缺的是「标签怎样过模型边界」这一步的裁定。收件人：P3 ＋
   协调者。**
9. **C4／OPEN-016 与 ENG-008**（§5.5、§2.3）。capability 判定在 Compiler 内部还是外部，决定本块是否
   要接 capability 类型、是否纳入 §322 的 Protected Core。本块口径：**判定在外、不纳入**；与 P6D 的
   设计必须同一口径（切分 §八.1 第 4 条）。**收件人：协调者 ＋ 规范维护者。**
10. **P2 的标签强制点无法依赖 P6B**（§7.5）。`ALLOWED` 的既有行不因 P6 增枚（除 runtime），而 P2 的
    拒绝点要读标签类型。**缺的是「标签怎样到达 P2 的拒绝点」这一步。收件人：P6B ＋ P2 ＋ 协调者。**
11. **（已闭合）P6C 的检索面与 `MemoryClaim` 读访问器均已冻结**（§7.2）。`MemoryRetrieval::retrieve(&RetrievalQuery)`
    与 `RetrievalQuery::new` 已冻结（P6C §3.5、§6.1）；`MemoryClaim` 的七枚读访问器也已冻结
    （P6C §3.5），本块的 `claim.id().as_str()` 可用。**保留本条作为「曾未冻」的记录**；实现期不再
    等待 P6C 补访问器，**收件人：无（已闭合；P6C 已交）**。
12. **P6B 的标签类型、附加点与 `validate` 已冻**（§7.1）。`ContextProvenance`／`ContextTrust`／`label`／
    `validate`／`PackageShape` 已冻结（P6B §3.1–§3.5）；本块按实际名写。**`validate` 的跨块调用方
    只有本块的 J3**（P6B §5.2 第 5 条）；J3 已改成调用 `validate`（§8.1）。`compile` 的生产路径不调用
    `validate`（P6B §5.2 第 5 条）。**收件人：无（已接；P6B §3.4 的边界仍由 P6B 记录）。**
13. **`into_parts()` / 按值取出内容的通道不存在**（§4.4）。若某个消费者需要按值拿走 `Vec<Reference>`，
    本块今天不提供；**缺的是「这条通道要不要开、开了怎样保持六字段的绑定」这一步。收件人：P1 ＋
    协调者。**
14. **`ContextPackage` 不能序列化**（§4.3）。跨进程／持久化／审计若需要它，要另立受控入口。
    **缺的是「要不要、以及在哪一层做」这一步。收件人：协调者。**
15. **迁移档**（§10.2）。本块零表、不占档；P6C 的档由 P6C 写死。**缺的是「P6 这一轮的档谁在哪一步
    合并成一张表」这一步。收件人：协调者 ＋ P6C。**
16. **§70 的「降低隐私暴露」没有落点**（§3.4、§2.4 第 9 条、§8.4 第 7 条）。本块只按调用方给的
    `scopes` 取记忆，不判隐私等级、不做放置裁决；P6C 也不做（P6C §6.3 逐字「不做隐私判定」）。
    **缺的是「上下文装配时的隐私裁剪由谁做」这一步**：P2 的拒绝？第 4 层的放置裁决？还是调用方的
    scope 选择？**收件人：P2 ＋ 第 4 层（P3e 计算放置）＋ 协调者。**
17. **Conversation 没有存储、没有轮次身份**（P6B §12 第 3 条；P6C §12.1 第 15 条逐字「P6A 的 `Conversation`
    原料同样没有落点」）。本块的 `Reference::Conversation(Message)` 只是承载形状；`conversation_history()`
    的 `historical: true` 只能对着 `&[Message]` 的临时切片声明；`crates/` 里没有对话表、没有 `MessageId`。
    **缺的是「会话由哪一层持有、id 是什么形状、谁写它」这一步。收件人：协调者（会话存储的归属未定）
    ＋ P1（若会话落在执行层）。**
18. **`ToolResult` 没有 `external` 判别位、provider 身份没有随文本下来**（P6B §12 第 2 条）。本块的
    `Reference::ToolOutput(ToolResult)` 只有 `output`／`is_error`；P6B 的
    `ContextProvenance::ToolOutput { external }` 需要调用方给 `external`；`provider_supplied()` 需要
    调用方知道「这条 `Role::System` 是第三方 provider 给的」，而 `Message` / `ToolDescriptor` 没有
    provider 字段。**缺的是「`ToolResult`／连接器返回值／provider 文本怎样携带来源判别」这一步。
    收件人：P3F（工具调用路径）＋ P3B（连接器）＋ P3G（模型调用路径的 provider 身份）＋ P1（artifact
    持有方，若 `external` 由制品侧给）＋ 规范维护者。**
19. **artifact 没有类型化的生产者分类**（P6B §12 第 5 条）。本块的 `ContextCandidate<ArtifactId>` 要
    调用方给 provenance；调用方只有 `Artifact.producer_node: Option<String>` 与无类型 JSON。**缺的是
    「artifact 的产出通道进入 `Artifact` 的类型面」这一步。收件人：P1（artifact 持有方）＋ 协调者。**
20. **用户粘贴的外部内容与用户指令不可区分**（P6B §12 第 4 条）。本块**不拦**：一段 `Role::User` 的
    `Message` 既可能是用户指令，也可能是一段从网页复制来的文字。**缺的是「客户端／对话协议怎样标记
    粘贴内容」这一步。收件人：P2（边界层）＋ 客户端／对话协议的持有方 ＋ 规范维护者。**
21. **memory claim 的外部性不进标签**（P6B §12 第 6 条）。本块的 `ContextProvenance::memory(...)` 只投影
    `MEMORY`；claim 的 `sources[]` 外部性留在 P6C 的 source 图里。**缺的是「claim 级来源摘要要不要
    进包／进标签」这一步。收件人：P6C ＋ P6B ＋ 规范维护者。**
22. **`continuum-artifact` 的层归属在两处层表里不一致**（§2.3；P6C §12.2 第 1 条报出）。README
    `:26-31` 把它列在执行层 (3)，《工程》§4 的标题与 §9.1 把它读作资源层 (4)。**缺的是「`continuum-artifact`
    属哪一层」这一步的统一。收件人：协调者 ＋ P6C。** 本块的依赖方向（P6A → artifact）在两种读法下
    都不让 P6 成为既有 crate 的上游；`ALLOWED` 只登记实际依赖，不登记层表。

---

# 12. 在切分文档里发现的错或缺口（**只记，不改**）

逐条给出实测。**本设计不改切分文档**（切分 §七 与协调纪律）。

1. **⚠️ §三第 2 条说「全仓没有 `InvokeRequest` 的非测试构造点」——已不成立。**
   实测：`crates/continuum-runtime/src/model_call.rs:436` 与 `:512` 各有一处生产构造
   （`call` 与 `call_stream`）。P3g 的模型调用路径已落地，切分写于 2026-10-08，早于它。
   **后果**：§八.1 第 2 条「P1 与 G 都已落地而无挂点」里的「G 已落地」成立，但「没有 `InvokeRequest`
   构造点」这一句不能再用来论证「没有接线点」。**本设计的 §6.2／§7.4 按实测写。**
2. **⚠️ §四第 2 条的迁移占用已过时。** 切分写「被分配而未占用的有 `60`、`70`、`90`」；实测
   `100`（canonical）／`110`（semantics）／`120`（budget）／`130`（verify）已注册
   （`crates/continuum-runtime/src/main.rs:110` 起的 `runtime_migrations()`），`140`–`199` 零命中。
   **本设计 §10.2 按实测写；P6C 的档要在最新树上现场核对。**
3. **⚠️ §2.4 的行号已漂。** 切分引 `Artifact.provenance` 在
   `crates/continuum-artifact/src/artifact.rs:178`（结构体 `:170`）；实测结构体在 `:214`、`provenance`
   在 `:222`、`PrivacyClass` 的枚举在 `:131`（字段 `:223`）。`NodeTrust`（`crates/continuum-node/src/node.rs:60`）
   与 `fn trust_label`（`crates/continuum-node/tests/node.rs:37`）仍对。**本设计 §3.5 按实测写。**
4. **⚠️ §四第 1 条的 `runtime_migrations()` 行号已漂。** 切分引 `:98`；实测
   `pub(crate) fn runtime_migrations()` 在 `crates/continuum-runtime/src/main.rs:110`。**不影响判据，
   只影响按行号核对的人。**
5. **✔ 已复测为正确的**：§2.3 的「`ContextPackage` 与标签类型今天不存在」——实测
   `grep -rn "ContextPackage" crates/` 零命中（只有切分文档的命中）；`grep -rn "MemoryClaim\|MemorySource"
   crates/` 零命中。§四第 1 条的 `("continuum-core", &[])` 在 `crates/continuum-runtime/tests/dependency_direction.rs:26`
   仍对。
