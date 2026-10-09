# P6 子项目（认知层）的归属与接口（共享面）

> 本文不是设计，是**四份设计并行的前提**。它的作用是：让 P6A / P6B / P6C / P6D 四份设计**对着同一套归属、
> 同一套接口类型、同一套名字**写，不至于各造一套词汇——「同一件事两个词汇表」在本项目一贯判为 **Critical**。
>
> 本文里的判断若与协调者的决定冲突，以本文为准并回报；本文之外的口径以规范为准。

---

## 一、路径归属

### 1.1 本阶段的规范范围

**P6 的规范范围是《工程》§6.1 的建设内容全表——八行**（`docs/02-工程.md:361-368`），
判据是 §6.4 的四条完成判据（`docs/02-工程.md:401-405`）。
阶段表同此：`README.md` 的「架构：八个层」表（**`9288a98` 版**：`:58`）把「**认知：Context、Memory、学习**
（《总纲》§6／《工程》§6）」整行判给 **P6**。
**该文件的用法提示**：本文写作期间 `README.md` 在工作树里正被另一处改写，新版本已不含该表
（实测：`git show HEAD:README.md | grep -n "P6"` 命中 `:58`、`:75`；工作树版本零命中）。故本节引的是**版控里那一版**。

此判另有两处依据，缺一处都不成立：

- **判据侧**：§6.4 的四条判据里，第 3 条（`用户纠正生成新的 Correction Source，旧 claim 进入 SUPERSEDED`，
  `:404`）与第 4 条（`Runtime Learning 的产物无法修改 §321 列出的五类语义`，`:405`）**都不落在
  Context Compiler 上**。若本阶段只建 Context Compiler，这两条判据在本轮**无主**。
- **排除表侧**：P1 的设计把「Context Compiler」整项排除给 P6（`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:33`），
  P2 的设计把「Prompt 信任标签强制」整项排除给 P6（`docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md:28`）。
  两张表点的都是**「第 6 层 / Context Compiler」这一个名字**——它们是同一个阶段的不同提法，
  不是两个阶段。**《工程》§6.1 那八行里没有任何一行被别的阶段认领过**：实测
  `grep -rn "P6"`（排除本文）全仓共**十处**命中，逐处的归属如下——`README.md`（`9288a98` 版 `:58`、`:75`；
  阶段表与进度表，见本节与下段）、P1 设计 `:33` 与 P2 设计 `:28`／`:736`（排除表，本节与 §八 八.1 第 1、2 条）、
  `p0-followups.md:105` 与 P0 设计 `:56`／`:288` 与 P0 计划 `:3606`（§八 八.1 第 3、4 条）、
  `crates/continuum-graph/tests/execution.rs:94`（§八 八.1 第 2 条）。**十处全部登记在案，无一处认领记忆或
  Runtime Learning**。

**一处名字上的分工要写清**：「Context Compiler」在本仓同时被用作**整个 P6 阶段的简称**
（`README.md` `9288a98` 版 `:75` 的进度表「切分中：P5 领域算子层、**P6 Context Compiler**」）与
**P6A 那一个组件的名字**。**本文此后只用后一种含义**：P6 是阶段，Context Compiler 是 P6A。

**若协调者的口径确为「P6 只含 Context Compiler 一项」**，则 §五 的 P6C / P6D 两块需改派，
且 §6.4 的判据 3、4 要另找主——本文不擅自替换该口径，只把依据摆在此处。

### 1.2 谁拥有什么

| 职责／路径 | 归属 | 规范依据 |
|---|---|---|
| `ContextPackage` 的形状与装配；§279 的「节点执行前必经」通道 | **P6A** | §279 §280 §69 §70；《工程》§6.2（`:378`）、§6.3（`:391`）、§6.4 判据 1（`:402`） |
| 七个信任标签的类型与附加点 | **P6B** | §281 §71 §50；《工程》§6.2（`:380`）、§6.3（`:392`）、§6.4 判据 2（`:403`） |
| 「认知层 → 执行层」那条边的载荷（`ContextPackage` 的 `provenance[]` / `trust_labels[]`） | **P6B** 定形状，**P6A** 承载 | 《工程》§9.4（`:635`） |
| Memory Source 图、`MemoryClaim` 与其 scope、Correction、Synthesis 后台作业、以及它们的落库 | **P6C** | §282 §283 §284 §285 §286 §170 §171 §172 §177 §179；《工程》§6.1（`:361-364`）、§6.2（`:372-376`）、§6.3（`:385-388`）、§6.4 判据 3（`:404`） |
| Runtime Learning 与 Protected Core；既有 crate 依赖方向的加固 | **P6D** | §320 §321 §322 §216 §217；ENG-001（《工程》`:647`）、ENG-008（`:697`）；《工程》§6.1（`:367`、`:368`）、§6.4 判据 4（`:405`） |
| 两枚入度为零的源头（Memory Source Graph、Protected Core，《工程》§9.2：`:600`） | P6C、P6D 各一枚 | 同上 |

**新建 crate 的具体名字由各块自己的设计定**，本文只登记「这一块拥有它」——四块之间不得出现两枚同义 crate。

### 1.3 共写文件

| 路径 | 谁改 |
|---|---|
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:98`） | **共写**：每块把自己那一档迁移注册进去（本仓既有做法「谁的表谁注册」）。**号段见 §四 第 2 条** |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 表（`:25`） | **共写**：每块在 `ALLOWED` 里加自己新建 crate 的那一行；**并集的复核具名给 P6D**（理由见 §四 第 1 条） |
| `Cargo.toml`（`[workspace] members`） | **共写**：每块加自己那一行 |

---

## 二、接口冻结现状

### 2.1 规范已给形状、本轮照抄、不新冻

四份设计**不得改名、不得另造第二套词汇**。以下名字由规范给定：

- §280 的 `ContextPackage` 六个字段：`instructions[]` `references[]` `artifacts[]` `memories[]` `provenance[]` `trust_labels[]`
  （`docs/spec/05-normative.md:1522-1536`）；
- §281 的七个标签：`USER_INSTRUCTION` `TRUSTED_RUNTIME` `PROJECT_DATA` `WEB_UNTRUSTED` `TOOL_OUTPUT` `MEMORY` `MODEL_OUTPUT`
  （`docs/spec/05-normative.md:1538-1554`）；
- §283 的 `MemoryClaim` 七个字段与 §284 的五个 scope（`docs/spec/05-normative.md:1579-1609`）；
- §322 的 `ProtectedCore` 五组规则（`docs/spec/05-normative.md:2294`）。

### 2.2 已在代码里、P6 一个都不碰

§315 `ModelProvider` / §316 `ToolProvider` / §124 `Connector` 三组接口的 trait 与类型已在
`crates/continuum-provider` 与 `crates/continuum-core`（现文与订正见
`docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md:25-99`）。
P6 不新增、不改动这三组中的任何一项，也不借用它们的名字。

### 2.3 本轮要新冻的：两枚类型

`ContextPackage` 与信任标签的类型在 `crates/` 里**今天不存在**（实测：`grep -rn "ContextPackage" crates/`
与 `grep -rn "MemoryClaim\|MemorySource" crates/` 各零命中）。故：

- **`ContextPackage` 归 P6A，只有一份**；它的 `provenance[]` / `trust_labels[]` 两个字段的元素类型
  **引用 P6B 的类型**，不另定义。
- **标签类型归 P6B，只有一份**。

### 2.4 三处已有的同名异物（**都不是** §281 的信任标签）

现场核对时会被误认为「标签已经建好了」，故列在此处，处置见 §四 第 3 条：

| 已有的东西 | 它是什么 | 实测位置 |
|---|---|---|
| `Artifact.provenance`（`serde_json::Value`） | 制品血缘（谁产出了它） | `crates/continuum-artifact/src/artifact.rs:178`（结构体在 `:170`） |
| `PrivacyClass`（五档） | 算力放置的隐私等级 | `crates/continuum-artifact/src/artifact.rs:87`、`:179` |
| `NodeTrust`（两枚）与测试辅助 `fn trust_label` | 计算节点的信任域 | `crates/continuum-node/src/node.rs:60`、`crates/continuum-node/tests/node.rs:37` |

---

## 三、跨层边

1. **P6 的两条上游**（《工程》§9.1 的层间图，`:578-581`；箭头读作「被依赖者 → 依赖者」，读法见 `:563-567`）：
   `资源层 (4) → 认知层 (6)` 与 `边界层 (5) → 认知层 (6)`。故 P6 依赖第 4、5 层是既有方向，不是反向边。
2. **`认知层 → 执行层`**：载荷是 `ContextPackage` 的 `provenance[]` 与 `trust_labels[]`（《工程》§9.4，`:635`）。
   **这条边今天没有落点**：执行侧的 `InvokeRequest`（`crates/continuum-core/src/model.rs:44`）装的是
   `messages: Vec<Message>`（`:46`），而 `Message` 只有 `role` 与 `content`（`:38-41`），没有承载标签的位置；
   且**全仓没有 `InvokeRequest` 的非测试构造点**（实测：`grep -rn "InvokeRequest {" crates/` 只命中定义处）。
3. **§279 的 MUST 通道今天没有调用方**：`ContextPackage` 零命中，「节点执行前必经」的接线点不存在。
   谁是调用方（P1 的节点执行，或 G 的模型调用路径）**不在本文决定**；登记为 P6A 的对外义务（§八 八.1 第 2 条）。
4. **P6 不得为了 §281 新增反向边**：`P0` 设计写「crate 方向在 P6 只加固，不新增反向边」
   （`docs/superpowers/specs/2026-09-30-p0-infrastructure-design.md:56`，同文 `:288`）。
   这是「拒绝行为在第 5 层」那句话的实现约束——见 §八 八.2 第 1 条（`《工程》§5.3` 与 `§9.1` 相抵）。

---

## 四、六条横切约束（四份设计都必须遵守）

1. **不新增反向边，且可观察**：加固的对象是既有 crate 之间的依赖方向，判据是
   `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 逐对断言（`:25`）
   与它的两个用例：`every_crate_depends_only_on_its_allowed_set`（`:288`）＋ 从 workspace 定义派生的
   `workspace_crates()`（`:234`）。**四块各自在 `ALLOWED` 里加自己新建 crate 的那一行；并集的复核具名给 P6D**
   ——理由是这条线已经断过一次：`ALLOWED` 曾由三份计划共写，而「合并后的并集对不对」没有任何 task 认领
   （`docs/superpowers/p3f-followups.md:56`）。
   **既有行的允许集合不因 P6 增枚，唯一例外是装配方 `continuum-runtime`**（`:206`）——
   尤其 `("continuum-core", &[])`（`:26`）保持为空。
2. **迁移号段**（待与 P5 那轮对账）：本仓的分配口径是「一个子项目一个十位档」
   （`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:480`；**P4 的设计引的是 `:456`，实测已漂到 `:480`**）。
   实测占用：`1`、`2`（`crates/continuum-persist/src/db.rs:29`、`:46`）、`10`（artifact）、`20`（graph）、`30`（workspace）、
   `40`（effect）、`41`（policy）、`50`（capability）、`80`（model-registry）（逐 crate 实读见
   `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:277-280`）；被分配而**未占用**的有 `60`、`70`、`90`。
   **分配与占用是两件事**：注册点是 `runtime_migrations()`（`crates/continuum-runtime/src/main.rs:98-107`），
   实际打开的调用方是 `recover_cmd.rs:30` 与 `task_cmd.rs:314`。
   **P5 与 P6 都要落库**，两轮若各取各的档会撞。**处置**：P6 这一轮的档**由 P6C 的设计写死**
   （本块是四块里唯一确定要落库的块），其余块若也要落库，号从同一档内顺取；各块在落地时**现场核对空号**
   （本仓既有做法）。**待与 P5 那轮对账。**
3. **同一件事两个词汇表**：「信任标签」这个词在本仓已有两个所指——`NodeTrust`（计算节点信任域，
   `crates/continuum-node/src/node.rs:60`）与 §281 的上下文来源标签。同族还有 `Artifact.provenance`（制品血缘）
   与 §281 的 `Context Provenance`（上下文来源）。**§281 的七个标签必须另取一个不与之争的名字**，
   四块（含 P6A 引用该类型处）用同一个名字。
4. **工具结果有两个消费者**（待与 P5 那轮对账）：§279 的 Compiler 取它作上下文原料，§89
   （`docs/spec/02-positioning.md:923`）的 Evidence 转换是另一处（Verifier 一侧，P5）。
   同一枚原料的两个消费者**不得各造一套 wrapper**。**待与 P5 那轮对账。**
5. **依赖边只登记实际用到的**：`ALLOWED` 与实际依赖**精确一致**，断言是逐对 `assert_eq!`
   （同 P3 共享面 §四第 4 条，`docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md:125`）。
   **新增 workspace 成员会先让那道门变红**，这是刻意的——不要靠「先不加 member」绕过。
6. **两对块共享一个未冻结的接口，故那两对不能全并行**（本项目的前提是「只要接口保持完好、定义完整，
   其他组件都是可以并行的」——共享的接口没冻，这条前提就不成立）：
   - **P6A ↔ P6B**：`ContextPackage` 的 `provenance[]` 与 `trust_labels[]` 两个字段的元素类型归 P6B，
     而 `ContextPackage` 归 P6A。**处置：P6B 开工的第一件事是冻结标签类型（一页接口），P6A 对着它写**；
     若协调者不接受这处串行，则 A 与 B **并成一块**。
   - **P6A ↔ P6C**：`Context Compiler ← MemoryClaim`（《工程》§6.3，`:391`）。`MemorySource`／`MemoryClaim`／
     scope 三枚类型与「按需取 claim」的检索面归 P6C，P6A 消费它。**处置同上：P6C 先冻这三枚类型，
     P6A 对着它写**；否则并成一块。
   - **P6B ↔ P6C**：本文判为**无未冻接口**（标签类型与 `MemoryClaim` 互不引用），故这两块可全并行；
     若 P6B 的设计发现需要引用 claim 侧的类型，按上两条同法处置。
   - **P6D** 不定义上述任何类型，故与三块都无未冻接口；它与三块的接点在 §四 第 1 条那张共写的 `ALLOWED` 表上。

---

## 五、四份设计的范围

| 块 | 一句话范围 | 主要规范依据（`§` ＋《工程》小节） | 拥有的路径 | 它冻结或产出的接口 |
|---|---|---|---|---|
| **P6A** | 建「节点执行前必经」的装配通道：从 Conversation／Memory／Artifacts／Project／Tool results 取料，按最小必要裁成 `ContextPackage`，并把「不存在绕过路径」做成可观察的强制面 | §279 §280 §69 §70；《工程》§6.1（`:365`）、§6.2（`:378`）、§6.3（`:391`）、§6.4 判据 1（`:402`）、§9.4（`:635`） | P6A 新建的 crate（名由本块设计定）；`ContextPackage` 类型**一份** | **产出** `ContextPackage`（§280 的六个字段名）；**消费** P6C 的 `MemoryClaim` 检索面；两个字段的元素类型**引用** P6B 的类型 |
| **P6B** | 建七个来源标签的类型与附加点，使「外部内容 MUST NOT 与用户指令处于同一信任级别」在数据上成立，并把这条载荷送过「认知层 → 执行层」那条边（**拒绝行为不在本块**，判给边界层） | §281 §71 §50；《工程》§6.1（`:366`）、§6.2（`:380`）、§5.1（`:291`）、§5.2（`:310`）、§6.4 判据 2（`:403`）、§9.4（`:635`） | 标签类型**一份**（名字见 §四 第 3 条） | **产出** 标签类型；它是 `ContextPackage` 两个字段的元素类型；与 P2 的接口是「谁做拒绝」那一处冲突的处置（§八 八.1 第 1 条） |
| **P6C** | 建记忆的来源图与派生 claim 的存储与状态迁移（删除 Source 使派生 claim 失效；纠正产生新的 Correction Source 并使旧 claim 进入 `SUPERSEDED`），以及后台合成作业 | §282 §283 §284 §285 §286 §169 §170 §171 §172 §177 §179；《工程》§6.1（`:361-364`）、§6.2（`:372-376`）、§6.3（`:385-388`）、§6.4 判据 3（`:404`） | P6C 新建的 crate；本块的迁移与表（号段见 §四 第 2 条） | **产出** `MemorySource`／`MemoryClaim`／scope 三枚类型（字段名由 §283／§284 给定，§二）与「按需取 claim」的检索面；Synthesis 的调度依赖第 7 层（§八 八.2 第 4 条） |
| **P6D** | 建 §321／§322 那条「Learning 不能自行取消规则」的边界，并把 P0 已定的 crate 方向在本阶段加固（不新增反向边） | §320 §321 §322 §216 §217；ENG-001（《工程》`:647`）、ENG-008（`:697`）、《工程》§6.1（`:367`、`:368`）、§6.4 判据 4（`:405`）；P0 设计 `:56`／`:288` | 新建的 Protected Core／Learning crate（名由本块设计定）；**`ALLOWED` 表的并集复核**（§四 第 1 条）；`Cargo.toml` | **产出** Protected Core 的强制形态（ENG-001 的方向选择）与 §322 五组规则位置的落地；**不得自造**那五组规则的类型（分属别的层，§八 八.2 第 5 条） |

### P6D 的加固面：加固的是哪条边、判据是什么

**加固的边**＝既有 workspace 成员之间的依赖方向，即 `crates/continuum-runtime/tests/dependency_direction.rs`
的 `ALLOWED` 逐对断言所钉住的那张表（`:25`；用例 `:288`，名单派生处 `:234`）。

**可观察判据三条**（缺一条这句话就不可验收）：

- **(a)** `cargo test -p continuum-runtime --test dependency_direction` 在**含 P6 全部新 crate 的树**上绿；
- **(b)** P6 新建的每枚 crate 在 `ALLOWED` 里各占一行，行内只列它的**实际直接边**——
  这一条由 `workspace_crates()` 与 `ALLOWED` 的相互覆盖断言（`:288` 起）逼出，不靠人工自觉；
- **(c)** 既有行不因 P6 增枚，唯一例外是装配方 `continuum-runtime`（`:206`）；
  实测差异为「只增 `continuum-runtime` 那一行的枚数 ＋ 新增 P6 各行」。**增在别的既有行上即为新增反向边**。

---

## 六、复审安排

每份设计由**另一个代理**交叉复审，且复审的任务是**试着推翻**它的断言、并找**漏项**——不是核对其措辞。
这条做法与它的实测来历见 P3 共享面 §六（`docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md:141-144`）。

本轮追加一条：**P6D 的 (a)(b)(c) 三条判据要被真跑**（在真含新 crate 的树上跑，而不是逐字推演）。
本仓的教训是「零命中」类断言会随注释漂（同形先例见 P3 共享面 `:72-81`）。

---

## 七、本轮验收

四份设计文档过审 ＋ 各自的实现计划。**本轮不写实现代码**（brainstorming 的硬门：设计未批不落实现）。

---

## 八、本轮的已知缺口（先记在此，免得四份设计各自踩一遍）

### 八.1 从别处捞回的、欠 P6 的账（四条）

1. **§281 的「强制点」——`P2` 欠的。**
   `docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md:28` 把「Prompt 信任标签强制」标为
   「第 6 层 Context Compiler（P6）／**不建**」，`:736` 又写「§281 的强制点在 P6 的 Context Compiler 就位后**才成立**」。
   **P2 要交的**：无（它按上表不建，且 P2 已落地）。
   **P6 要交的**：标签的类型与附加点——**具名给 P6B**；**协同方**是 P2（拒绝行为那一侧）与 P5（工具结果这条原料的另一位消费者，§四 第 4 条）。
   **这处归属与《工程》相抵**，处置见 八.2 第 1 条。
2. **Context Compiler——`P1` 欠的。**
   `docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:33` 的范围排除表把「Context Compiler」整项判给 P6。
   **P6 要交的**：Compiler 本体（P6A）；**外加一条 P1 那边没有的**——§279 要求「节点执行前 MUST 通过」，
   而**今天没有调用方**（§三 第 2、3 条实测）。**谁接上这条通道，本文不决定**，但它是 P6A 必须具名回报的对外义务。
   收件人：协调者（P1 与 G 都已落地而无挂点）。
   **同一处的一条顺带事实**：P1 侧的接入点已备好——`crates/continuum-graph/tests/execution.rs:94`
   的注释把 `producer_node` 与 `ctx.profile()` 记作「P3 后端解析与 **P6 上下文编译**的接入点」，
   且该文件的用例覆盖了它们的读取行为。故 P6A 读 Artifact 那一侧不必新开口子。
3. **一条已闭合的清理项——`P0` 的 followup 欠的，且原注记的文件名是错的。**
   `docs/superpowers/p0-followups.md:105` 写「`main.rs:12` tokio/thiserror 未被引用，属 **P6 清理项**」。
   **实测**：该处指的是 P0 期 `crates/continuum-runtime/Cargo.toml:12-13`（P0 期 `main.rs:12` 是 `Err(e) => {`，
   该文件里从未出现过这两枚名字）；**且两枚今天各有使用点**——`tokio`：`crates/continuum-runtime/src/tool_call.rs:443`；
   `thiserror`：`cli.rs:75`、`error.rs:29`、`sandbox_select.rs:40`。
   **故该条已由后续阶段闭合，P6 无动作**。留此以免下一轮照旧注记再查一次。
4. **「只加固、不新增反向边」——`P0` 的设计交给 P6 的义务。**
   `docs/superpowers/specs/2026-09-30-p0-infrastructure-design.md:56`（同文 `:288`；P0 的计划在
   `docs/superpowers/plans/2026-09-30-p0-infrastructure.md:3606` 抄了同一句）：「本子项目定下的方向在 P6
   只加固，不新增反向边」。它指的是 ENG-001「Protected Core 的强制形态」的候选方向 a（独立 crate，
   《工程》`:647`）。**P6 要交的**：P6D 的加固与其三条判据（§五）。
   **未定的那一半**：`ProtectedCore`（§322 的五组规则）的类型分属多层的既有类型，P6 不得自造；
   且 ENG-008（《工程》`:697`）「Context Compiler 是否纳入 Protected Core」未决——它决定 P6A 的 crate
   要不要进 Protected Core，**必须由 P6A 与 P6D 两处给出同一个口径**，否则两块会各答一次。收件人：协调者。

### 八.2 本轮新登记的缺口

1. **《工程》里的两处「边的方向」相抵（规范级，两处同族）**：
   - **其一（§5.3 对 §9.1）**：§5.3（`:326`）写 `Prompt 信任标签强制 ← 第 6 层 Context Compiler`
     （读作「边界层的这一项依赖认知层」），而 §9.1 的层间图只画了 `边界层 (5) → 认知层 (6)`
     （读作「认知层依赖边界层」，`:581`），末句又是「依赖方向单向，无环」（`:588`）。两者并存即成环。
     同族的还有 §5.2（`:310`）「强制点**在本层**而非第 6 层」与 P2 设计 `:28` 的「强制点在 P6」。
   - **其二（§9.4 对 §9.1）**：§9.4（`:635`）把「**认知层 → 执行层** Context Package 的 provenance 与
     trust_labels 字段」列为一条跨层载荷。按 §9.1 的箭头读法（`:563`，「被依赖者 → 依赖者」），
     这条读作「**执行层依赖认知层**」——而 §9.1 的图里**没有这条边**，且它与「单向、无环」相抵：
     图上已有 `执行层 (3) → 资源层 (4) → 认知层 (6)`（`:573`、`:578`），即**认知层已传递地依赖执行层**，
     再加一条「执行层依赖认知层」即成环。
     **这一处直接压在 P6A／P6B 上**：ContextPackage 的消费者是谁、那条边从哪一侧登记，取决于是哪一种读法。
     **两种读法都不由本文裁定**；但**无论哪种读法，P6 的 crate 都不得成为既有 crate 的上游**
     （即 `ALLOWED` 里除 `continuum-runtime` 外的既有行不得因 P6 增枚，§四 第 1 条）。
   **收件人（两处）**：《工程》维护者 ＋ 协调者。
2. **OPEN-013／ENG-001 阻断本层**（《工程》§6.5 `:410`；《总纲》A13 `:1692`）：§320 与 §321 的执行机制
   自身归属未定，ENG-001 的方向（a／b／c）是 P6D 的前提。收件人：协调者／规范维护者。
3. **C4／OPEN-016**（《总纲》`:1731`）：capability 判定发生在 Compiler 内部还是外部——它决定 Compiler
   是否必须纳入 §322 的 Protected Core（即 八.1 第 4 条那半句）。收件人：协调者／规范维护者。
4. **Memory Synthesis 依赖第 7 层**：§285 的合成是**后台作业**，而「后台作业怎么被调度」属第 7 层的
   Background Supervisor（§299；《工程》§7.1 `:428`、§7.3 `:453`）。**P6C 要交的**：合成作业的本体与它的调度边界；
   **调度那一半留给 P7**（协调者已把 P7 搁下）。收件人：协调者。
5. **`ProtectedCore` 的五组规则类型分属多层**：`authority_rules`／`contract_rules` 在语义层与边界层、
   `capability_rules` 在资源层、`audit_rules`／`effect_rules` 在边界层。**P6D 不得为它们另造类型**，
   只能引用各层已冻结的那几枚（§322 只给了五个字段名）。收件人：P6D 的设计者；若某组规则在上游不存在，
   登记为上游缺口而不是在 P6 造一份。
6. **工具结果的两位消费者**（§279 的 Compiler 与 §89 的 Evidence 转换）与**迁移号段**（§四 第 2 条）：
   两条都**待与 P5 那轮对账**——本轮不预先裁定，留待两份切分文档并置后由协调者收。

---

**本文的实测口径**：行号为 2026-10-08 在 `main`（`9288a98`）工作树上用 `grep -n` / `sed -n` 逐条读出；
按内容引优先，凡给行号处均写明是哪一枚文件。
**一处例外**：`README.md` 在工作树里正被另一处改写，故凡引它处一律注明是 **`9288a98` 版**的行号
（见 §一 1.1）。零命中类断言按本仓规矩限定语料（`crates/`）并逐处读过命中行。
