# P6C 记忆来源图、claim 存储与合成作业 — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p6-scope-and-split.md` §五 的 P6C 行的原话，逐字照用）：

> 建记忆的来源图与派生 claim 的存储与状态迁移（删除 Source 使派生 claim 失效；纠正产生新的
> Correction Source 并使旧 claim 进入 `SUPERSEDED`），以及后台合成作业

**本块拥有 / 产出的接口**（同行的第二栏，逐字）：`MemorySource`／`MemoryClaim`／scope 三枚类型
（字段名由 §283／§284 给定，§二）与「按需取 claim」的检索面；Synthesis 的调度依赖第 7 层（§八 八.2 第 4 条）。

**规范依据**：§282 §283 §284 §285 §286 §169 §170 §171 §172 §177 §179；《工程》§6.1（`docs/02-工程.md:361-364`）、
§6.2（`:372-376`）、§6.3（`:387-390`）、§6.4 判据 3（`:404`）、§9.2（`:600`）；《总纲》§6.2（`docs/01-总纲.md:1015-1095`）、
§6.3（`:1097`）。

**本文件只写 P6C。** 其余三块（P6A 上下文装配、P6B 信任标签、P6D Protected Core 与 Runtime Learning）
各有其设计，本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §12 写成一条
**具名的对账条目**，不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（判据与订正原文见 `docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

**行号口径**：本文件出现的每一处「文件:行号」都是在工作树 `.worktrees/p6-plan`（分支 `p6-plan`，
提交 `03e79c7`）里当场 `grep -n` / `sed -n` 实测的。切分文档的若干行号已随树漂移，本文按内容引，
并在 §3.3 一次列出替读者核对过的三处差异。

---

# 1. 范围

## 1.1 建什么

《工程》§6.1（`docs/02-工程.md:361-364`）的前四行归本块，逐行落到本设计的一节：

| §6.1 的组件 | 规范依据 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| Memory Source Graph | §282 §170 | §4 | 类型与存储全量：九臂的 `SourceKind`、`MemorySource`、二部图边、软删除语义；**写入面只有 `insert_source`，生产者见 §3.5 / §9.4，`Conversations` 的生产者缺口见 §12.1 第 15 条** |
| MemoryClaim 存储 | §283 §284 §179 | §5 | 全量：§283 七字段逐字、§284 五 scope 逐字、锚点侧表、唯一产生点 |
| Memory Synthesis 后台作业 | §285 §171 §172 | §8 | 作业记录 + 事务性 applier；**proposer 不建**（§8.5） |
| Memory Correction | §286 §177 | §7 | 全量：新 Correction Source + 旧 claim `SUPERSEDED`，一笔事务 |
| 「按需取 claim」的检索面 | 切分 §五 P6C 行；《工程》§6.3 `:391` | §6 | 全量（scope／锚点／子串／条数／终态开关） |
| 落库与迁移 | 切分 §四第 2 条 | §9 | 八张表，迁移 **190**（档见 §9.1） |
| §6.4 判据 3 | `docs/02-工程.md:404` | §7.4、§10.1 | 一条运行期照片（正反两侧） |
| 入度为零的源头之一 | §9.2 `docs/02-工程.md:600` | §3.4、§13 | 模块方向可观察 + 一处规范级申报（§12.1 第 9 条） |

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| `ContextPackage`、Context Compiler、六个字段的装配 | P6A（切分 §五）。本块只提供 §6 的检索面，不装配上下文 |
| 七个信任标签、标签的附加点与「外部内容不得与用户指令同级」的强制 | P6B（切分 §五）。本块不定义任何标签类型，也不把 `SourceKind` 当标签用（§6.3） |
| `ProtectedCore`、Runtime Learning 的产物规则、§321 五类语义的不可修改边界 | P6D（切分 §五）。本块的 `RUNTIME` scope 只存 claim，不施加 §321（§12.2 第 3 条） |
| Memory Synthesis 的**生成算法**：语义去重、矛盾检测、重要性估计、摘要生成 | 规范未给（§8.5 逐项列）。本块只建作业记录与事务性 applier，不建 proposer |
| 后台作业的调度：cron、空闲判定、「大量」的阈值、重试与恢复 | 第 7 层 Background Supervisor（§299，`docs/spec/05-normative.md:1892`；《工程》§7.1 `:428`、§7.3 `:453`）。本块只给触发面与准入判据（§8.2、§8.6） |
| `forget` / `do_not_mention` / 物理擦除 | §178（`docs/spec/03-product-drive.md:1153`）；不在本块的规范依据行里。本块的「删除 Source」是软删除（§4.4），不声称实现物理擦除 |
| Personal Memory 的放置与 capability 判定 | §182（`docs/spec/03-product-drive.md:1274`；《总纲》转述 `:1084`）属资源层／边界层。本块只按 `scope` 过滤，不做权限判定（§6.3） |
| Memory Summary 的渲染与 UI | 第 7 层／前端。本块只存 `MemorySummary` 的文本投影（§8.4） |
| 事件类型、审计类型 | §310 的九类与 §313 的八类都是 **MUST 最小清单**（`crates/continuum-events/src/event.rs:13-32`、`audit.rs:18-35`），不是封闭集合；本块按最小读法**零事件、零审计**（§9.5） |
| 向量／嵌入检索 | 规范未给。§72（`docs/spec/02-positioning.md:432`）只禁止「单一全局向量数据库作为唯一抽象」，没有要求向量检索；本块的检索是确定性的（§6.2） |
| 对外可构造的 `MemoryClaim` 或直接写 claim 的 API | **刻意不建**：§282 说 Memory 是 derived representation；claim 只由 §8 的 applier 产生（§5.5） |
| `MemorySourceGraph` 这个类型 | 规范只给名字，没给字段或操作。一个空的图类型只是第二套词汇；图落在 `memory_claim_source` 的边上（§4.3） |

---

# 2. 规范依据与挂靠

## 2.1 逐条：规范给了什么、落在哪、没给什么

| 依据 | 原文给的 | 本设计的落点 | 规范**没给**、由本设计定的 |
|---|---|---|---|
| §282（`05-normative.md:1556-1575`） | 长期 Memory **MUST** 基于 Memory Sources，不是仅维护人工条目表；Source **可能**来自 conversation／artifact／task history／project／explicit correction／connected source | §4.1、§4.2、§5.5 | Source 的字段、编码、删除机制；§282 的六个词与 §170 的八类怎么合（§2.2） |
| §283（`:1579-1593`） | `MemoryClaim` 七字段：`id` `scope` `statement` `sources[]` `validity` `confidence` `last_confirmed` | §5.1 逐字 | 七个字段的类型与取值域（§5.3）；`sources[]` 的元素取 id 还是对象 |
| §284（`:1597-1607`） | 五个 scope：`PERSONAL` `PROJECT` `PRODUCT` `TASK` `RUNTIME` | §5.2 逐字 | 每个 scope 的实例绑定（§5.4） |
| §285（`:1611-1627`） | 后台 **MAY** 执行 Memory Synthesis，用于去重／冲突处理／更新时效／归纳长期事实／生成 Memory Summary | §8.5 逐项 | 五个功能各自的算法（§8.5） |
| §286（`:1631-1645`） | 用户纠正 **MUST** 产生新的 Correction Source，并使旧 claim 进入 `SUPERSEDED`，不静默覆盖历史 | §7 | `SUPERSEDED` 存在哪个字段（§5.3）；correction source 与旧 claim 是否连边（§7.3） |
| §169（`03-product-drive.md:847-875`） | 第一抽象是 Memory Synthesis，不是 Saved Memory Database | §8.1、§8.5 | 同 §285 |
| §170（`:879-901`） | Source 的八类树：Conversations／Task History／Product History／Project Artifacts／Files／Connected Services／Explicit User Corrections／User Policies；Memory 是 derived representation | §4.1 的九臂（八臂来自本节） | 九臂中的第九臂来自 §282（§4.1） |
| §171（`:905-938`） | 合成作业的七步流程；可发生于 session 结束后／系统空闲时／大量项目状态改变后／用户主动要求时 | §8.2 四臂、§8.5 七步 | 「空闲」「大量」的阈值（§8.2、§12.1 第 6 条） |
| §172（`:942-976`） | Memory 只综合长期稳定偏好、长期项目、重要约束、持续目标、重要关系、常用工作方式、当前长期状态；Source Store 存历史，Synthesis 理解历史 | §8.5 边界、§11 | 「长期稳定」的判据（§11、§10.4） |
| §177（`:1127-1149`） | 纠正五步：找 claim → 找 sources → 加入 correction source → invalidate／supersede → 重新 synthesis | §7.2、§7.3 | 第 3 步「加入」是加入 source store 还是加入旧 claim 的 `sources[]`（§7.3） |
| §179（`:1189-1218`） | 五个 scope 各一例 | §5.2 | 同 §284 |
| 《工程》§6.1（`02-工程.md:361-364`） | 四行组件的规范依据 | §1.1 | —— |
| 《工程》§6.2（`:372-376`） | Source 是事实来源、claim 是派生；删除 Source 必须使派生 claim 失效，删除 claim 不影响 Source；Synthesis 输入 Source 集合、输出 claim 集合与 Summary；Correction 产生新 source 并 SUPERSEDED | §4.4、§5.5、§7、§8.1 | 删除的机制（软／硬，§4.4）；「删除 claim」的操作面（§4.3） |
| 《工程》§6.3（`:387-390`） | `Memory Source Graph ← 无`；`MemoryClaim ← Memory Source Graph`；`Synthesis ← MemoryClaim + Source 集合`；`Correction ← MemoryClaim + Source 集合` | §3.4、§4.3 | —— |
| 《工程》§6.4 判据 3（`:404`） | 用户纠正生成新的 Correction Source，旧 claim 进入 `SUPERSEDED` | §7.4、§10.1 | 「用户」的验真（§7.5） |
| 《工程》§9.2（`:600`） | 认知层的入度为零组件：Memory Source Graph、Protected Core | §3.4、§13 | 持久化是否算一条上游（§12.1 第 9 条） |
| 《总纲》§6.2（`01-总纲.md:1015-1095`） | 五级作用域表、来源图、claim 形态、合成触发、纠正、写入门、过期、隐私 | §5.2、§5.5、§5.3、§12.1 第 1 条 | 与 §284 的一处不一致（§2.2） |
| 《总纲》§6.3（`:1097`） | Runtime 可学习对象与 Protected Core | P6D；本块只提供 `RUNTIME` scope 的存储 | —— |

## 2.2 两处规范内部的不一致（先记，处置见 §12）

**其一：五个 scope 的名单有两份。** §284（`05-normative.md:1597-1607`）与 §179（`03-product-drive.md:1189-1199`）
都是 `PERSONAL`／`PROJECT`／`PRODUCT`／`TASK`／`RUNTIME`；而《总纲》§6.2 的五级作用域表
（`01-总纲.md:1021-1027`）逐行是 **Session**／Task／Project／Personal／Runtime——**没有 Product，多出 Session**。
切分 §二第 1 条要求「§284 的五个 scope，逐字」，故本设计取 §284 的五枚；`Session` 没有臂。
这一条不是措辞差异：它决定「当前交互的临时状态」有没有存储面。处置与收件人见 §12.1 第 1 条。

**其二：Source 的分类有两份。** §170 给八类树；§282 的六个词是 conversation／artifact／task history／project／
explicit correction／connected source。两者不是同一张表的两个抄本：§282 的 `project` 在 §170 里没有对应项，
§170 的 Product History／Files／User Policies 在 §282 里没有对应词。§282 的原文是「Source **可能**来自」，
不是封闭声明。本设计取**并集九臂**（§4.1），代价与翻案条件写在该节。收件人见 §12.1 第 3 条。

## 2.3 本块自定形状的总账（规范给到哪、代价、落点、收件人）

**这一节是自定形状的总账**：§2.1 只列「规范没给、由本设计定」，本节把本块**自己定的每一类形状**逐条给
「规范给到哪 / 代价 / 落点 / 收件人」。不进这张表的，本块没有再自定别的形状。

| # | 自定的形状 | 规范给到哪一步 | 代价 / 边界 | 落点 | 收件人 |
|---|---|---|---|---|---|
| 1 | `SourceKind` 九臂（§170 八类 ∪ §282 六词） | 两份都只给类别名；§282 是「可能来自」 | 一臂只在一份里、三臂只在另一份里；裁定后删臂 | §4.1 | 规范维护者 |
| 2 | `MemorySource` 五字段、`external_ref` 不透明、允许 `(kind, external_ref)` 重复 | §170／§282 **不给 Source 的字段或引用类型** | 无引用完整性；同一会话可有两条 source（不同观察时刻） | §4.2 | 规范维护者 + 协调者 |
| 3 | 软删除 + **无条件**失效派生 claim | §282／§6.2 说「删除必须使派生 claim 失效」，不给机制 | 不满足物理擦除；多来源 claim 也会整体失效（可用性代价） | §4.4 | 规范维护者 + 协调者 |
| 4 | `Validity` 三臂（`Active`／`Superseded`／`Invalidated`） | §286 只给 `SUPERSEDED`，§6.2 只给「失效」；§74 另有 TTL／decay／archived | `Active` 是未命名初态；无区间、无 TTL／decay／archived（§12.1 第 4、16 条） | §5.3 | 规范维护者 |
| 5 | `memory_claim_anchor` 侧表（`claim_id, anchor_id`） | §283 七字段**没有实例绑定** | 类型看不见锚；第八字段待裁定（§12.1 第 2 条） | §5.4 | 规范维护者 + 协调者 |
| 6 | claim 唯一产生点（`MemoryClaim::new` 为 `pub(crate)`，只由 `apply_synthesis` 写） | §282 说 Memory 是 derived representation；**没给产生机制** | `apply_synthesis` 公开，手写 proposal 挡不住；只保证形状 | §5.5 | 协调者 |
| 7 | `RetrievalQuery` 形状（scope + anchor + 子串 + 条数）与 `limit = 0` 返回空 | 规范**没有检索语义**；§6.3 只给「Compiler ← MemoryClaim」 | 无相关度、无向量检索；scope／anchor 由调用方给 | §6 | 规范维护者 + P6A |
| 8 | Correction 的 `statement` 落在 `memory_correction` 行 | §286／§177 只说产生新 Correction Source，**没说新说法存哪** | 与 source 分家；`external_ref` 仍是不透明串 | §7 | 规范维护者 |
| 9 | Synthesis 做成「作业记录 + 事务性 applier」；proposal 由外部产生 | §285／§171 给五个功能、七步流程，**不给算法** | 语义去重、矛盾检测、重要性、摘要生成都不在本块（§8.5） | §8 | 规范维护者 + 协调者 |
| 10 | 八表／六索引／迁移 `190` | §317／§318 只给持久化与事务原则，**不给 schema** | schema 是本块自定，改动要新迁移（§9） | §9 | 协调者 + P0 |
| 11 | 零事件、零审计 | §310／§313 给的是 **MUST 最小清单**，不是封闭集合 | 记忆变更不上事件流、不进审计链；代价见 §12.1 第 7 条 | §9.5 | 规范维护者 + 协调者 |

---

# 3. 归属、crate 与依赖边

## 3.1 crate 名：`continuum-memory`

本块新建**一个** crate：`continuum-memory`（实测：`grep -rn "continuum-memory" crates/ Cargo.toml` 在本工作树零命中；
四份 P6 设计文档里出现的 `continuum-memory` 是**本设计的 crate 名**，不是已存在的 crate，故不计入占用。
`crates/` 下也没有任何 `MemoryClaim` / `MemorySource` / `MemoryScope` 的命中）。

**名字的理由（三条）**：

1. **它逐字命名本块的四行**：§6.1 的四行都以 Memory 开头（Memory Source Graph、MemoryClaim、Memory Synthesis、
   Memory Correction）。`continuum-memory` 是这四行唯一不歧义的名字。
2. **它不与另外三块争词**：P6A 的 crate 会围绕 Context／Compiler 命名，P6B 围绕 Provenance／Trust，P6D 围绕
   Protected Core／Learning。`continuum-cognition` 会把四块都装进去，`continuum-context` 会与 P6A 撞车。
3. **它不叫 `continuum-memory-store`**：本块持有三枚规范给名的类型与检索面，不只是一个存储适配器；
   而拆成「类型 crate + 存储 crate」会让一个块占两条 `ALLOWED` 行、多一条实际不表达任何规范边界的依赖边，
   且与切分「P6C 新建的 crate」的单数口径不符。`MemorySource Graph ← 无` 的那条方向，本设计落在
   **同 crate 的模块方向**上（§3.4），不靠第二个 crate。

## 3.2 依赖边

依赖边表（**只登记实际用到的**，切分 §四第 5 条）：

| 被依赖 | 用到什么（逐项） | 边别 |
|---|---|---|
| `continuum-persist` | `Tx` / `Migration` / `PersistError` / `Value`（§9 的八张表与全部读写函数） | 普通 |

**只有这一条。** 不登记的边逐条给理由（零使用的边即假边，P2b 为此删过两条）：

- **不登记 `continuum-events`**：本块零事件、零审计（§9.5）。§310 的九类里没有记忆类事件，加第十类不是本块的权限。
- **不登记 `continuum-artifact`**：`ProjectArtifacts` 的引用是**不透明字符串**（`external_ref`），不取 `ArtifactId`。
  理由：§170／§282 都没有给 Source 的引用类型；取 `ArtifactId` 会让本 crate 依赖 `continuum-artifact`，
  即 §9.1 读法下的 `执行层 (3) → 认知层 (6)`；而图里这条边只以
  `执行层 (3) → 资源层 (4) → 认知层 (6)` 的传递形式存在。代价见 §4.2。
- **不登记 `continuum-core` / `continuum-semantics` / `continuum-graph`**：本块不构造任何来自它们的类型。
  claim 的 `id` 与 source 的 `id` 都是本 crate 的 newtype；`statement` 是自由文本；锚点是字符串（§5.4）。
- **不登记 `continuum-verify`**：P5a 的 `Evidence` 是否成为 Memory Source，规范没有给（§170 的八类里没有
  Evidence）。本块不建这条桥，也不造 wrapper（§12.1 第 13 条）。
- **不登记 `continuum-policy` / `continuum-capability`**：§182 的 Personal Memory 放置判定不在本块（§6.3）。

**dev 边**：没有。本块的用例只起真库、经 `Tx` 调用本 crate 的函数，不需要第二个 workspace crate。

## 3.3 共写文件、迁移号段与三处行号漂移

**共写文件**（切分 §一 1.3 的表只列了三行：workspace members、`dependency_direction.rs` 的新 crate 行、
`main.rs` 的注册；本块实际是**五处**。第 2、4 行是本设计补的——runtime 的 manifest 与 runtime 的
`ALLOWED` 行；请协调者确认，见 §12.2 第 6 条）：

| # | 文件 | 本块的改动 | 说明 |
|---|---|---|---|
| 1 | `Cargo.toml` 的 `[workspace] members`（`:3-31`） | 加一行 `crates/continuum-memory` | 切分 §一：每块只加自己这一行 |
| 2 | `crates/continuum-runtime/Cargo.toml` | 加 `continuum-memory = { path = "../continuum-memory" }` | `main.rs` 要调 `continuum_memory::p6c_memory_migrations()`；**没有这条 path 依赖，注册那一行编不过** |
| 3 | `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-memory", &["continuum-persist"])` | 新 crate 会先让 `:478` 的用例变红再被补齐，这是刻意的 |
| 4 | 同一文件的 `continuum-runtime` 行（`:391-412`） | 在数组里加 `"continuum-memory"` | 断言是双向的（`appeared == allowed.contains`，`:490-520`）：只加第 3 行会红 |
| 5 | `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:110`） | 加一行 `migrations.extend(continuum_memory::p6c_memory_migrations()); // 190：记忆八表` | 「谁的表谁注册」 |

**除这五处共写之外，不碰**任何既有 crate 的源码、类型或注记：本块不新增共享类型，也不改任何既有字段。
`crates/continuum-runtime/src/main.rs` 的那一行是共写表第 5 行，不是「不碰」的例外。

**迁移号段（切分 §四第 2 条）**：**P6 的档是 `190`–`199`；本块取 `190`。**

- 实测占用（逐 crate 读 `Migration::new` 的第一个参数）：`1`、`2`（`crates/continuum-persist/src/db.rs:29`、`:46`）、
  `10`（artifact）、`20`（graph）、`30`（workspace）、`40`（effect）、`41`（policy）、`50`（capability）、
  `80`（model-registry）、`100`（canonical）、`110`（semantics）、`120`（budget）、`130`（P5a 验证与证据五表）。
  **`190`–`199` 在 `Migration::new` 的调用里全段零命中**（含测试夹具；唯一的 `190`/`191` 出现在
  `crates/continuum-image/tests/pixels.rs:62` 的像素数据里，与迁移无关）。
- P5c 的设计声明 `150`（其 §9.3，`:937-951`），但**零表**——那是登记，不是迁移；P5d／P5e／P5f 均声明不占档。
  P5e 设计 §11.3（`:1142`）给过的次序建议把 P6 放在 `190`，本设计**接受并写死**。
- **`191`–`199` 不预分给 P6A／P6B／P6D**：它们今天都没有声明要落表。若某块在实现期落表，号从同一档内
  **现场核对空号后顺取**（切分 §四第 2 条）。本块只写死自己用的 `190` 与整档的边界。
- 落地时**必须重跑** `grep -rn "Migration::new(" crates/` 再取号：本表是 2026-10-17 的实测，
  「我没占它」不等于「它没人占」（P5f 设计 §10.3 的同一条纪律）。

**三处行号漂移（替读者核对，本文按内容引）**：

| 切分写的 | 本工作树实测 | 说明 |
|---|---|---|
| `runtime_migrations()` `:98` | `:110` | `:98` 是该方法文档注释里的一行，不是函数定义 |
| `dependency_direction.rs` 的用例 `:288`、`workspace_crates()` `:234` | `:478`、`:424` | 文件随 P5 增长 |
| `continuum-runtime` 的 `ALLOWED` 行 `:206` | `:391-412` | 同一张表的后段 |

## 3.4 层间位置与「入度为零」

§9.1（`docs/02-工程.md:561-588`）里与本块有关的两条，按「被依赖者 → 依赖者」读：

- `资源层 (4) → 认知层 (6)` 与 `边界层 (5) → 认知层 (6)`：本块**不登记**指向这两层的边（§3.2），
  故不新增反向边；不登记不是遗漏，是零使用。
- `认知层 (6) → 长期循环 (7)`：本块不接。Memory Summary 是投影，不是 `ProductReadiness`；
  长期循环要读的是 claim 的 `value + evidence + last_verified` 那一套（§232），本块不产出它。

**§6.3 的层内依赖**（`:387-390`）在 crate 内的落法（下图是**模块**依赖：`←` 读作「右者依赖左者」，不是 §9.1 的层间箭头）：

```
src/source.rs      ←  不 use 任何 claim 侧类型（Memory Source Graph ← 无）
src/claim.rs       ←  use source（MemoryClaim ← Memory Source Graph）
src/correction.rs  ←  use source + claim
src/synthesis.rs   ←  use source + claim
src/retrieval.rs   ←  use claim + source（只读）
```

**「入度为零」的兑现与它的一处申报**：§9.2 把 Memory Source Graph 列为依赖图的源点；**这一枚由本块拥有**
（另一枚 Protected Core 由 P6D 拥有，切分 §1.2）。本设计把它落成
**模块方向**——`src/source.rs` 不引用 `src/claim.rs`（§10.3 的 S3 照片）。但本 crate 必须落库，
故它依赖 P0 的 `continuum-persist`。**按 §9.2 的字面读法，这已经是一条上游**；本设计按「§9.1 的八层组件」
读，P0 基础设施不在那八层里，故层内入度仍为零。**这处读法有先例也有反例**：同表列为源点的
「Compute Node 注册」其 crate `continuum-node` 今天依赖 `continuum-artifact`（`ALLOWED` 行
`crates/continuum-runtime/tests/dependency_direction.rs:151`），已不是字面上的零入度；而 P5b 的
`continuum-method` 用空数组把「Execution Method Library ← 无」钉成了硬断言（同文件 `:157`）。
两处并存说明本仓对这句话有两种读法。**本设计不自称解决它**，记入 §12.1 第 9 条。

## 3.5 冻结清单：本块持有的类型与 P6A 的一页接口

**类型**（全部定义在 `continuum-memory`）：

```
MemorySourceId  SourceKind  MemorySource
MemoryClaimId   MemoryScope  Validity  Confidence  MemoryClaim
RetrievalQuery  MemoryRetrieval  MemoryStore
MemorySummaryId  MemorySummary
Correction  CorrectionOutcome  CorrectionRecord
SynthesisJobId  SynthesisStatus  SynthesisTrigger  SynthesisRequest
ProposedClaim  SynthesisProposal  SynthesisReport
MemoryError
```

**`MemoryError` 的臂（一枚，不另造第二个错误类型）**：

```
Persist(PersistError)
// Source
EmptyExternalRef  UnknownSource  SourceAlreadyRemoved  SourceRemoved
// Claim / scope
EmptyStatement  EmptySourceSet  DuplicateSourceId  DuplicateClaimId
UnknownClaim  ClaimNotActive  AnchorRequired  AnchorNotAllowed
// Correction
WrongCorrectionSourceKind
// Synthesis
DuplicateJobId  UnknownJob  JobNotRunning  EmptySummary
ForeignSource  ScopeMismatch  SupersedeMergeConflict
TriggerSourceOutsideInput  WrongTriggerSourceKind  EmptyTriggerSet  InvalidTriggerTime
// Retrieval
EmptyScopeSet
```

臂名是拒绝点的名字，逐条在 §4–§8 的正文里出现；**除 `Persist` 是数据库错误的透传外，每一枚都有具名拒绝点与
照片**（§10.1 的附表按公开操作逐条列出）。没有一枚是「为将来」预留的。
两处早先的死路已在形状上收口：第二次纠正先被 `correct` 第 2 步的 `ClaimNotActive` 挡住（§7.2）；
`limit = 0` 返回空集，不是错误（§6.2）。

**P6A／P6B 消费的访问器（字段私有，这是唯一的读面）**：

| 类型 | 访问器 | 说明 |
|---|---|---|
| `MemoryClaim` | `id() -> &MemoryClaimId`、`scope() -> MemoryScope`、`statement() -> &str`、`sources() -> &[MemorySourceId]`、`validity() -> Validity`、`confidence() -> &Confidence`、`last_confirmed() -> i64` | P6A 的 `memories[]`、P6B 的 `claim.id().as_str()` 都走这里 |
| `MemorySource` | `id() -> &MemorySourceId`、`kind() -> SourceKind`、`external_ref() -> &str`、`observed_at() -> i64`、`removed_at() -> Option<i64>` | 追溯与来源摘要 |
| `MemoryClaimId` | `new(impl Into<String>) -> Self`、`as_str() -> &str` | 构造只在装配／测试；读是纯字符串 |
| `MemorySourceId` | `new(impl Into<String>) -> Self`、`as_str() -> &str` | 同上 |
| `SynthesisJobId` | `new(impl Into<String>) -> Self`、`as_str() -> &str` | P7 构造作业 id；`new` **不校验唯一性**，唯一性由 `begin_synthesis` 的 `DuplicateJobId` 判（G6） |
| `Confidence` | `from_value(serde_json::Value) -> Self`、`as_value() -> &serde_json::Value` | 只存不判 |

**Source 的写入口（唯一）**：

```rust
pub fn insert_source(tx: &Tx<'_>, source: &MemorySource) -> Result<(), MemoryError>;
```

**`RetrievalQuery` 的 `text = Some("")` 归一为 `None`**（不设文本过滤；结果与空子串相同，但语义是「不过滤」
而不是「匹配空串」）。P6A §5.2 已按这一条对齐（`None` 与 `Some("")` 都归一为 `None`）。

**它是一个公开写入口，但本块不自动产生 source**：调用方是装配方或 P7——P7 在 `begin_synthesis` 之前把
「这次要合成的来源」逐条落库；**P6A 不是生产者**：它没有会话存储，也没有落库入口（P6A §11 第 17 条）。
今天 `Conversations` 没有存储面（§12.1 第 15 条），故这条写入口在「会话来源」上还没有真正的生产者；
这是上游缺口，不是本块的遗漏。

**P6A 消费的接口（这一页就是切分 §四第 6 条要求先冻的那一页）**：

**不新冻（切分 §2.1）**：本设计**不改名、不另造**规范已给形状的四组名字——`ContextPackage` 六字段、
§281 的七个标签、`MemoryClaim` 七字段与 §284 的五个 scope、`ProtectedCore` 五组规则。本块冻结的只有切分 §五
判给 P6C 的那三枚（`MemorySource`／`MemoryClaim`／scope）与检索面；`SourceKind`／`Validity`／`Confidence`／
作业类型是本块的**局部形状**，不是跨块冻结面——别的块不得引用它们，除非走 §12.2 的对账。

```rust
/// P6A 消费的**对象安全**检索面。`MemoryStore` 是它在 `&Tx` 上的实现；
/// 自由函数 `retrieve_claims` 是同一实现的底层原语。
pub trait MemoryRetrieval {
    fn retrieve(&self, query: &RetrievalQuery)
        -> Result<Vec<MemoryClaim>, MemoryError>;
}

pub struct MemoryStore<'a> { /* 持 &'a Tx<'a> */ }

impl<'a> MemoryStore<'a> {
    pub fn new(tx: &'a Tx<'a>) -> MemoryStore<'a>;
}

impl MemoryRetrieval for MemoryStore<'_> { /* 调 retrieve_claims */ }

// 只读。P6A 不得另造 claim / scope / source 的第二套类型。
pub fn retrieve_claims(tx: &Tx<'_>, query: &RetrievalQuery)
    -> Result<Vec<MemoryClaim>, MemoryError>;

pub fn load_source(tx: &Tx<'_>, id: &MemorySourceId)
    -> Result<Option<MemorySource>, MemoryError>;

pub fn load_sources(tx: &Tx<'_>, ids: &[MemorySourceId])
    -> Result<Vec<MemorySource>, MemoryError>;

pub fn load_summary(tx: &Tx<'_>, scope: MemoryScope, anchor: Option<&str>)
    -> Result<Option<MemorySummary>, MemoryError>;

/// 只返回最新的、`stale = false` 的摘要；没有则 `None`。
pub fn load_current_summary(tx: &Tx<'_>, scope: MemoryScope, anchor: Option<&str>)
    -> Result<Option<MemorySummary>, MemoryError>;
```

- `MemoryClaim` 的字段是私有的，P6A 只能经上表的访问器读；`sources[]` 是 `Vec<MemorySourceId>`。**两段式是刻意的**：
  claim 是派生表示，source 才是事实（§282）。**但据实：P6A 当前接口到不了第二段**——`compile` 只收
  `&dyn MemoryRetrieval`（无 `Tx`），而 `load_sources` 收 `&Tx`；要拿 source 的具体内容，得给
  `MemoryRetrieval` 加方法，或由装配方在请求里传入已读出的 source 摘要（§12.2 第 1 条）。本块不为此改 trait。
- **与 P6A 的冻结点已对齐（据实）**：P6A §5.2 已用 `scopes: Vec<MemoryScope>`、`anchor: Option<String>`、
  `memory_query: Option<String>`；P6A §5.3 已用 `RetrievalQuery::new(scopes, anchor, text, limit, false)?`
  构造（唯一构造点），`limit` 取 `budget.max_memories`、`include_terminal` 写死 `false`；P6A §5.6 把
  构造拒绝与检索失败原样透传 `MemoryError`。**`limit = 0` 与 `Some("")` 的语义以本设计 §6.1 为准，
  两边已一致**：0 返回空集，空串归一为 `None`。**scope／anchor 的选取是调用方的事，不是 P6C 的隐私判定**
  （§6.3）。`MemoryStore` 由装配方用 `&Tx` 构造。**P6C 不提供第二枚检索 trait**（若 P6A 另立 `&str` 适配，
  适配层留在 `continuum-context`；反过来会让 `continuum-memory` 依赖 `continuum-context`，是反向边）。
- P6A **不得**把 `MemorySummary` 当作 claim 的替代品塞进 `memories[]`：它是投影（§8.4）。
  若 P6A 需要摘要，用 `load_current_summary`（`stale = false` 才有值）。
- P6B 与本块**无共享类型**（切分 §四第 6 条已判）；P6D 与本块无共享类型。若 P6A 或 P6B 在写作中发现
  需要 claim 侧的新形状，按 §12 的对账条目走，不在本文件外就近加。

---

# 4. Memory Source 图

## 4.1 `SourceKind`：九臂，取 §170 八类 ∪ §282 六词的并集

| 臂 | §170（`03-product-drive.md:884-893`） | §282（`05-normative.md:1568-1575`） | 说明 |
|---|---|---|---|
| `Conversations` | Conversations | conversation | |
| `TaskHistory` | Task History | task history | |
| `ProductHistory` | Product History | —— | 只有 §170 有 |
| `ProjectArtifacts` | Project Artifacts | artifact | |
| `Files` | Files | —— | 只有 §170 有 |
| `ConnectedServices` | Connected Services | connected source | |
| `ExplicitUserCorrections` | Explicit User Corrections | explicit correction | §286 的 Correction Source 落这一臂 |
| `UserPolicies` | User Policies | —— | 只有 §170 有 |
| `Project` | —— | project | **只有 §282 有**；指项目自身的状态／上下文，不是它的制品 |

**`Project` 这一臂的读法（据实标注）**：本设计把它读作**项目自身的状态／上下文**，不是项目的制品
（制品已在 `ProjectArtifacts`）。若 §282 的 `project` 其实指的就是项目制品，则这一臂与 `ProjectArtifacts`
同义，应删——这是这一臂唯一的翻案条件，与下面那两处删臂条件并列。

**为什么取并集、不取任一份**：两份都是规范正文（§282 是 normative，§170 是产品驱动），且 §282 的前缀是
「**可能**来自」——它不是封闭声明。取交集会丢掉四类（Product History／Files／User Policies／Project），
而丢掉的每一类都可能是一条真实来源；取并集是「不发明」口径下唯一不静默删规范的读法。这与 P5a 对
`EvidenceType` 取 §258 九臂 ∪ §193 两臂的处置同形（P5a 设计 §4.2）。

**代价与翻案条件（据实）**：并集意味着九臂里**有一臂（`Project`）只在一份规范里出现，三臂只存在另一份里**。
若规范维护者裁定 §282 的六词是穷尽表，则 `ProductHistory`／`Files`／`UserPolicies` 三臂要删，`Project` 留；
若裁定 §170 的八类是穷尽表，则 `Project` 要删。删除是枚举加臂／减臂的一次改动，落点是 `SourceKind::ALL`
的数目断言与两个穷尽 `match`，编译期强制。**这一条记入 §12.1 第 3 条。**

**§169 的那串来源不算第三份分类**：`03-product-drive.md:861` 的「过去聊天、保存的信息、自定义指令、
Library 文件以及连接应用内容」是对 ChatGPT 现状的描述，不是本系统的规范分类。它没有「保存的信息」对应的臂：
那正是 §169 要取代的旧式「Saved Memory」抽象。本设计不为它加臂。

`SourceKind` 带 `ALL: [SourceKind; 9]`、`as_str` / `parse`，编码小写 + 下划线（与 `ArtifactType` 同形，
`crates/continuum-artifact/src/artifact.rs:28-67`）。`parse` 对表外串返回 `None`，不取默认臂——
理由同 `ArtifactType::parse` 的注释：把别的来源当成本型处理，比报错更坏。

## 4.2 `MemorySource` 的形状

§170／§282 都**没有给 Source 的字段**，故这是本设计定的形状（记入 §12.1 第 3 条与 §2.1）：

```rust
pub struct MemorySource {
    id: MemorySourceId,       // 本 crate 的 newtype；落库主键
    kind: SourceKind,         // 九臂，§4.1
    external_ref: String,     // 不透明定位符：会话 id / 文件路径 / 连接器 id / 项目 id
    observed_at: i64,         // 该来源事件发生的时间（Unix 毫秒），由调用方给
    removed_at: Option<i64>,  // 软删除的墓碑（§4.4）
}
```

**只这五个字段，逐条给理由**：

- `external_ref` 是**不透明的**，不是 `ArtifactId`／`IntentId`／`FilePath` 的联合。规范没有给 Source 的引用类型；
  取某一层的强类型会造一条 §9.1 没有画的直接边（§3.2）。代价是**没有引用完整性**：一个指向不存在会话的
  ref 也能入库。本块不为它造校验器——那需要知道每一类来源的存储，而那些存储分属别的层。
  **今天至少有一类没有解析者**：`Conversations` 在 `crates/` 下没有会话存储（`continuum-canonical` 的
  `ResolutionScope::Conversation` 是 §274 的解析范围，不是会话记录）。这一条记入 §12.1 第 15 条。
- `observed_at` 由调用方给，本层**不取系统时钟**（同 P5a 的 `occurred_at`、P1 的 `apply_transition`）。
- `removed_at` 是软删除标记（§4.4），不是来源的正文，也不是任何规范字段的替身。
- **没有 `explicit_by_user`**：§176 的「记住这个」会产生高优先级 Source，但本块不建 explicit remember 的入口
  （§1.2、§12.1 第 11 条）。加一个没有生产者的列就是「为将来可能用到」，与 P5a 的字段纪律相抵。
- **没有正文**：Source 的正文在它自己的存储里（会话、文件、制品）。本块只存定位符与时间。
  这是 §282「Source 才是事实来源」的落实：Memory 侧不复制一份正文。

构造点 `MemorySource::new(id, kind, external_ref, observed_at) -> Result<MemorySource, MemoryError>`：
`external_ref` 去空白后为空 ⇒ `Err(EmptyExternalRef)`。`MemorySource` 的字段私有、无 `Default`、无字面量构造
（照片 C5）。

**不禁止同一 `(kind, external_ref)` 的多条 source**：它们是不同 `observed_at` 的观察记录（同一会话在两次
合成之间可以被观察两次）。本块不建 `UNIQUE(kind, external_ref)`，去重是调用方的事；代价是来源追溯里可能
出现重复条目。若规范要「同一来源只有一条」，那是一条新的唯一约束 + 合并规则，记入 §2.3 第 2 行的自定形状总账。

**写入的拒绝点**：`insert_source` 对已存在的 `source_id` 返回 `DuplicateSourceId`（不覆盖）；照片 R31。

## 4.3 图的形状：只有一种边

**本块把「Memory Source Graph」读作 claim 与 source 的二部图**，边只有一种：

```
MemorySource ──→ MemoryClaim        （§9.1 读法：claim 依赖 source）
memory_claim_source(claim_id, source_id)   （这条依赖的落库边；列序是从属方向，不是依赖方向）
```

- **没有 source → source 的边**。§170 画的是分类树，不是来源之间的引用图；§282 只规定 claim 基于 source。
  造一条「来源引用来源」的边，就是把分类树当成了 DAG——规范没有给这条边，本设计不造。
- **`sources[]` 是引用，不是内嵌副本**。§283 只给 `sources[]`，没给元素类型；取 id 使「source 是事实、
  claim 是派生」在类型上成立——改了 source（删除）不会留下一份 claim 里的旧副本。
- **不建 `MemorySourceGraph` 类型**（§1.2 末行）：图的操作只有「按 claim 取 source」「按 source 取 claim」
  （后者在 §4.4 的删除路径里用），落在 SQL 上就是这张边表的两个方向。

**「删除 claim 不影响 Source」这一句**：本块**不提供物理删除 claim 的路径**（§5.5 的唯一产生点 +
没有 delete API），故这句话在本轮没有操作面——不是被违背，是没有第二个动作能碰到 source。
这一条记入 §10.4 的「拍不出」与 §12.1 第 12 条。

## 4.4 删除 Source：软删除 + 无条件失效派生 claim

**§282／§6.2 的 MUST**：删除 Source 必须使派生 claim 失效。本设计的落法：

```
remove_source(tx, id, occurred_at) -> Result<Vec<MemoryClaimId>, MemoryError>
```

一笔事务里做三件事：

1. `memory_source.removed_at = occurred_at`（已是非空 ⇒ `Err(SourceAlreadyRemoved)`；不存在 ⇒ `Err(UnknownSource)`）。
2. 把**所有**在 `memory_claim_source` 里引用它的 claim 的 `validity` 从 `Active` 改成 `Invalidated`
   （已经是终态的不动），返回这些 claim 的 id。
3. 把这些 claim 所属 scope/anchor 的 `memory_summary.stale` 置 1（§8.4）。

**为什么是软删除，不是物理删除**：

- §183（`docs/spec/03-product-drive.md:1305`；《总纲》转述 `:1093`）要求来源可追溯；物理删除会让「这条 claim 当初凭什么成立」永远消失。
- §286 的「不覆盖历史」是同一条纪律的另一面。
- **代价据实**：软删除不回收存储，也不满足「forget 必须让数据不存在」的读法。§178 的 forget
  （`03-product-drive.md:1153`）不在本块的规范依据里；物理擦除需要另一条路径（谁有权、擦哪些表、摘要怎么办），
  本块不声称实现它。**记入 §12.1 第 12 条。**

**为什么是「无条件」失效，而不是「只剩这一个来源时才失效」**：§6.2 的原文是「删除 Source 必须使派生
claim 失效」，没有「若还有其他来源则除外」。取无条件读法是 fail-closed 的一侧：一条有三个来源的 claim
在删掉一个来源后也失效，可能**多判**失效；重新合成可以再建立它。取条件读法则要发明「其余来源足够支持」
的判据——那正是 §285 没有给的算法（§8.5）。代价是可用性，不是正确性。

**重建的路径**：失效的旧行留在历史里；下一次合成用仍然存活的来源重建一条**新 id** 的 `Active` claim
（去重键只匹配 `Active`，§8.4 A2 第 5 条）。这不是「复活旧行」，是「用新证据重新断言」——§286 的
「不覆盖历史」在失效路径上同样成立。

**删除后的可见性**：retrieval（§6.2）与 synthesis 的输入（§8.2）都排除 `removed_at IS NOT NULL` 的 source；
`load_sources` **不排除**，因为追溯要看见它（带 `removed_at`）。这两个方向都拍照片（§10.1 R4）。

---

# 5. `MemoryClaim` 与 scope

## 5.1 §283 七字段，逐字

§283（`docs/spec/05-normative.md:1579-1593`）的原文：

```
MemoryClaim {
    id
    scope

    statement
    sources[]

    validity
    confidence
    last_confirmed
}
```

逐字段落法：

| §283 字段 | Rust 字段 | 类型 | 来历 / 读法 |
|---|---|---|---|
| `id` | `id` | `MemoryClaimId`（String newtype） | 照录；主键 |
| `scope` | `scope` | `MemoryScope`（五臂，§5.2） | 照录 |
| `statement` | `statement` | `String` | 照录字段名；**不解析**——§283 没给语法。构造时去首尾空白，空则 `Err` |
| `sources[]` | `sources` | `Vec<MemorySourceId>` | 元素类型是本设计定的（§4.3）；非空、去重、按 `MemorySourceId` 升序规范化（故 `PartialEq` 对集合相等成立，顺序不是语义） |
| `validity` | `validity` | `Validity`（三臂，§5.3） | 照录字段名；`SUPERSEDED` 落在这里（§5.3） |
| `confidence` | `confidence` | `Confidence`（JSON 包装） | 照录字段名；**只存不判**（§5.3） |
| `last_confirmed` | `last_confirmed` | `i64`（Unix 毫秒） | 照录字段名；写入时由来源时间派生（§5.3） |

**「七字段，不多不少」是结构事实**：`MemoryClaim` 没有第八个字段——没有 `status`、没有 `created_at`、
没有 `scope_ref`、没有 `anchor`。SUPERSEDED 落在 `validity`；实例绑定落在侧表（§5.4）。照片 C1 读一个
不存在的 `claim.status`／`claim.anchor`，编不过。

**`sources` 的顺序不是语义**：构造与读回都按 `MemorySourceId` 升序规范化。故 `MemoryClaim` 派生的
`PartialEq` 对「source 集合相同、顺序不同」的两枚返回相等——P6A 的 `ContextPackage` 按值比较不会因此假红。

**字段私有 + 访问器**：`MemoryClaim::new` 是 `pub(crate)`，crate 外只能读不能造（§5.5，照片 C2）。
这不是省事：它把「claim 只能由合成产生」变成编译期事实。

**`MemoryClaim` 实现 `Clone` / `PartialEq` / `Debug`**：P6A 的 `ContextPackage` 按值持有
`memories: Vec<MemoryClaim>`，且它的 `#[derive(Debug, Clone, PartialEq)]` 要求这三者
（P6A §4.1 的 `ContextPackage` 派生与 §7.2 的「P6C 给本块的」第 1 条）；`Confidence` 的
JSON 是深拷贝。若 P6A 改成按引用持有，这一条可以去掉——它不是规范要求，是消费面的需要。

## 5.2 §284 五个 scope，逐字

§284（`:1597-1607`）的原文：

```
PERSONAL
PROJECT
PRODUCT
TASK
RUNTIME
```

| §284 的 token | Rust 变体 | `as_str`（落库） | 实例锚点 | §179 的例子（`03-product-drive.md:1203-1217`） |
|---|---|---|---|---|
| `PERSONAL` | `Personal` | `personal` | 无 | 「用户喜欢中文」 |
| `PROJECT` | `Project` | `project` | 必填 | 「Core 不再使用 move」 |
| `PRODUCT` | `Product` | `product` | 必填 | 「这个产品决定使用 ADFIR」 |
| `TASK` | `Task` | `task` | 必填 | 「当前 bug 已定位 parser」 |
| `RUNTIME` | `Runtime` | `runtime` | 无 | 「Model X 做 verifier 经常失败」 |

- **变体名按 Rust 的 CamelCase，token 按 §284 逐字**：五个名字就是上表左列的五枚。**规范原文是「至少支持」**
  （§284 `:1599`；§179 `:1191` 的「至少划分」同），不是规范自身的封闭声明；本设计按切分 §二第 1 条把这五枚
  **冻成本块的闭集**，若协调者要加第六枚，见 §12.1 第 1 条。
  落库编码取小写，遵循本仓「枚举列一律小写 + 下划线」的既有纪律（P5a 设计 §10.2 第 2 条）；
  `as_str` / `parse` 是编码的唯一产生点，`ALL: [MemoryScope; 5]` 与穷尽 `match` 把关。
- **`Session` 不在表里**：这是 §2.2 那处不一致的直接后果。本设计取 §284 的五枚；若协调者裁定
  《总纲》的 Session 也要有存储面，那是第六臂，改动是枚举加臂 + 迁移的 scope 校验 + §12.1 第 1 条关闭。
- **scope 是类别，不是实例**：`PROJECT` 不说是哪个项目。实例绑定在 §5.4 的侧表里，**不是第八个字段**。

## 5.3 `validity`、`confidence`、`last_confirmed`

### 5.3.1 `Validity`：三臂，`SUPERSEDED` 的唯一槽位

```rust
pub enum Validity {
    Active,       // 初始态；规范没有给它名字（§12.1 第 4 条）
    Superseded,   // §286 逐字的 SUPERSEDED
    Invalidated,  // §6.2「删除 Source 使派生 claim 失效」的失效态
}
```

**为什么 SUPERSEDED 在 `validity` 里**：§283 的七个字段里没有第二个能承载生命周期；§286 只说「旧 claim 进入
SUPERSEDED」，没有给第八个字段。把 `validity` 读作生命周期槽位是唯一不增字段的落法。**代价据实**：
§174（`03-product-drive.md:1020-1051`）把「temporal validity」列为一个事实要保留的东西，而三臂枚举
表达不了时间区间（从何时到何时有效）。本设计只保留 `last_confirmed` 与状态，**不表达区间**；
若规范要区间，`validity` 需要变成结构体或加字段——那是规范级改动，记入 §12.1 第 4 条。

**状态迁移（本设计定）**：

```
Active ──→ Superseded    （§286 纠正；§8.4 的显式 supersede）
Active ──→ Invalidated   （§4.4 删除来源）
Superseded / Invalidated ──→ 任何状态：不允许（终态）
```

`Validity` 带 `as_str` / `parse`（三臂往返，编码小写；`parse` 对表外串返回 `None`，不取默认臂）。

终态不可复活是结构性的：`apply_synthesis` 只在「新建」时写 `Active`，`supersedes` 只从 `Active` 出发；
`correct` 只接受 `Active`；`remove_source` 只把 `Active` 改成 `Invalidated`。三道写入口（四条语句）都拒绝反向迁移。

### 5.3.2 `Confidence`：只存不判

```rust
/// §283 的 `confidence`。规范只给字段名与 §174 的两个例子（high / medium）；
/// 没有取值域、没有档位序、没有比较规则。故只有 as_value / from_value。
pub struct Confidence(serde_json::Value);
```

- **没有 `Ord` / `PartialOrd`**：给一个规范没给的序，就是发明（与 P5a 对 `EvidenceStrength` 的处置同形，
  P5a 设计 §4.3.4）。照片 C6 写 `a < b`，编不过。
- **没有命名档、没有默认值**：`Confidence::from_value` 收任意 JSON；落库原样序列化。
- 合并时的取值见 §8.4 A2 第 5 条：**后一次合成的判断覆盖前一次**——没有序，就没有「取更高」这种规则；
  「覆盖」是唯一不发明序的动作。**代价：旧 `confidence` 不留历史**；claim 行的 `confidence` 是最新一次
  合成的判断，不是一条可追溯的序列。若 §174 的「保留 confidence」被读成「保留历次值」，那需要一张
  修订表或一个历史字段，本设计没有（记入 §12.1 第 4 条的同一类缺口）。

### 5.3.3 `last_confirmed`：写入时派生，不是调用方填

`last_confirmed` 是 Unix 毫秒，**由 applier 计算**：claim 的全部来源里 `observed_at` 的最大值。
理由：§174 的「last confirmed」问的是「这个事实最后一次被来源确认是什么时候」，那个时间只能从来源来；
让提案方自己填，就是把一个可派生的事实变成可伪造的输入。`ProposedClaim` 里**没有这个字段**（§8.3，照片 C4）。

合并时取 `max(旧值, 新来源的 observed_at)`；删除来源不改变它（它记录的是「最后一次被确认」，不是「现在是否有效」）。

## 5.4 scope 的实例绑定：**一对字段** `scope` + `anchor: Option<String>`

**问题**：§283 的七字段只有 `scope` 这个类别，没有「哪个项目／哪个产品／哪个任务」。一条 `PROJECT` claim
若不知道属于哪个项目，`PROJECT` 这个 scope 在检索时就是不可用的——多项目的 claim 会混在一起。
规范没有给这个字段，而切分要求七字段逐字。

**唯一形状（本设计的收口）**：全块只用一对字段——`scope: MemoryScope` + `anchor: Option<String>`。
**没有第二种锚点类型**（任何把 scope 塞进锚点的类型都会让 scope 存两次，并让 P6A／P7 在两种形状之间拼一层）。
四个落点同一对：

| 落点 | 形状 |
|---|---|
| `memory_claim` + `memory_claim_anchor` | claim 行的 `scope` ＋ 侧表的 `anchor_id`（仅 anchored scope） |
| `RetrievalQuery`（§6.1） | `scopes: Vec<MemoryScope>` ＋ `anchor: Option<String>` |
| `SynthesisRequest`（§8.1） | `scope: MemoryScope` ＋ `anchor: Option<String>` |
| `MemorySummary` / `load_summary`（§8.4） | `scope` ＋ `anchor: Option<String>` / `Option<&str>` |

**同一条规则，写入与读取都执行**：

- `Project`／`Product`／`Task`：`anchor` **必填**；缺 ⇒ `Err(AnchorRequired)`。
- `Personal`／`Runtime`：`anchor` **必须为 `None`**；给 ⇒ `Err(AnchorNotAllowed)`。
- 一次查询／作业／摘要只能落在**一个锚点**上：`anchor` 为 `Some` 时 `scopes` 必须恰好是**一个** anchored
  scope（`Project` / `Product` / `Task`）；anchored 与 unanchored 不能混在同一个 `RetrievalQuery.scopes` 里——
  混了时 `anchor` 必然要么缺、要么多，分别落 `AnchorRequired` / `AnchorNotAllowed`（两种都拒绝，没有一条放行的路径）。
- 校验点是**一个函数**（`validate_scope_anchor(scope(s), anchor)`），由 `RetrievalQuery::new`、
  `begin_synthesis` 的 G5、`load_summary` / `load_current_summary` 三处调用；没有第四种写法。

**落法**：一张**不在 `MemoryClaim` 里**的侧表：

```
memory_claim_anchor (claim_id PK, anchor_id TEXT NOT NULL)
```

- 侧表只存 `anchor_id`：`scope` 在 `memory_claim` 行里，**不在侧表重复**。这不是第二种形状，是同一对字段的
  存储规范化（claim 行 + 一行锚点）。
- 锚点**不进入 `MemoryClaim` 类型**：P6A 拿到的 claim 仍然只有七字段，它看见的是「查询时已经按锚点筛过」的结果。
  **代价据实**：一个 claim 值离开查询上下文后，说不出自己属于哪个项目；要说得出来，只能再查侧表。
- `anchor_id` 是不透明字符串，不取 P4 的 `ProjectId`／`IntentId`：本块不登记到 `continuum-semantics` 的边（§3.2）。
  锚点的解析者是传它进来的调用方（P6A 或装配方）。
- **为什么不做成第八个字段**：切分 §二第 1 条要求 `MemoryClaim` 照 §283 七字段逐字；`anchor` 不在 §283 里。
  侧表让「七字段」与「可用性」同时成立，代价是类型看不见它。**这一条是本块最明显的一处「规范缺口 + 工程补丁」**，
  记入 §12.1 第 2 条。

## 5.5 唯一产生点：claim 只从 §8 的 applier 来

§282 说 Memory 是 derived representation；《总纲》§6.2 的写入门（`01-总纲.md:1073-1078`）说
Agent 不得仅依据自身的重要性判断直接写入长期 Memory。本设计把这两条落成结构：

- `MemoryClaim::new` 是 `pub(crate)`，且**只构造 `Active`**；crate 外**构造不出**一个 claim（照片 C2）。
  读路径用 crate 内的 `rehydrate` 从库行还原（它不经过 `new` 的校验——库里的数据已经过写入口）。
- `memory_claim` 表**只有一个写入者**：`apply_synthesis`（§8.4）。`correct` 不创建 claim；`remove_source` 不创建 claim；
  `insert_claim` 这个 API **不存在**。
- 因此「绕过合成直接写一条 claim」在类型上写不出来，在 crate 外也没有入口。

**残余（据实）**：`apply_synthesis` 是公开的，任何持有 `Tx` 的调用方都能递一份 proposal。它挡不住
「proposal 是某个人手写的」——它只保证**形状**（来源非空、来源在输入集合内、scope 由作业给定、终态不可复活）。
「这份 proposal 是不是一次真正的合成」没有类型能判。这与 P5a 的 `Evidence::from_tool_result` 同一形态：
唯一产生点保证的是形状，不是动机。

---

# 6. 检索面（P6A 的接法）

## 6.1 `RetrievalQuery`

```rust
pub struct RetrievalQuery {
    scopes: Vec<MemoryScope>,     // 非空；重复按集合去重
    anchor: Option<String>,       // 与 scopes 的规则见 §5.4
    text: Option<String>,         // 精确子串；Some("") 归一为 None
    limit: usize,                 // 0 = 返回空集（不报错）
    include_terminal: bool,       // 默认 false
}

impl RetrievalQuery {
    /// 唯一构造点：在这里执行 §5.4 的 scope/anchor 规则。
    pub fn new(
        scopes: Vec<MemoryScope>,
        anchor: Option<String>,
        text: Option<String>,
        limit: usize,
        include_terminal: bool,
    ) -> Result<RetrievalQuery, MemoryError>;

    pub fn scopes(&self) -> &[MemoryScope];
    pub fn anchor(&self) -> Option<&str>;
    pub fn text(&self) -> Option<&str>;
    pub fn limit(&self) -> usize;
    pub fn include_terminal(&self) -> bool;
}
```

**`new` 的拒绝点**：`scopes` 空 ⇒ `EmptyScopeSet`（重复项按集合语义去重，不报错）；anchored scope 缺 anchor
⇒ `AnchorRequired`；`anchor` 为 `Some` 而 `scopes` 不是恰好一个 anchored scope（含 anchored 与 unanchored 混排）
⇒ `AnchorNotAllowed`；unanchored scope 给 anchor ⇒ `AnchorNotAllowed`。
`text = Some("")` 归一为 `None`；`limit = 0` **接受**，`retrieve` 直接返回空集
（P6A 的 `max_memories = 0` 因此不是错误，见 §12.2 第 1 条）。

## 6.2 规则

1. **只返回 `Active`**，除非 `include_terminal = true`。`Superseded` 是「已被纠正的旧 claim」，
   `Invalidated` 是「来源已删」——两者都不得进入上下文。
2. **scope／anchor 过滤**按 §5.4 的同一套规则；`text = None`（含 `Some("")` 归一）**不做文本过滤**，
   `text = Some(s)` 时用 `instr(statement, ?) > 0`（区分大小写）。**不做**大小写折叠、不做 Unicode 归一化、
   不做词干化：这些都会改变语义，而规范没有给检索语义。
3. **顺序确定**：`last_confirmed DESC, claim_id ASC`。没有「相关度」——规范没有给相关度的判据。
4. **`limit` 是条数上限**：`0` 返回空集；没有「全量倾倒」的入口。§70 的最小必要裁剪是 P6A 的职责，
   但本块不给出一个无上限的入口。
5. **`sources[]` 里的 source 可能已 `removed_at` 非空**（终态 claim 的追溯会看到）；要只看活的，
   由调用方按 `MemorySource.removed_at` 过滤。
6. **摘要的读法**：`load_summary` 返回最新一条（含 `stale`），`load_current_summary` 只返回
   `stale = false` 的最新一条；两者都按 `generated_at DESC, summary_id ASC` 取。

## 6.3 不做的检索与不做的判定

- **不做语义检索／向量检索**：§72（`docs/spec/02-positioning.md:432`）只禁止把单一向量库作为唯一抽象，没有要求向量检索；本块的检索是确定性的，
  可在任何 SQLite 上复现。
- **不做隐私判定**：§182 的 Personal Memory 放置与 capability 不在本块（§1.2）。`RetrievalQuery` 的
  `scopes` 由调用方给，本块不默认包含 `PERSONAL`，也不替调用方做权限检查。
- **不做信任标签**：`MEMORY` 是 §281 的标签之一（P6B 的类型）。本块不把 `SourceKind` 当标签，
  也不决定「一条来自 WEB_UNTRUSTED 来源的 memory 该打哪个标签」——那是 P6A／P6B 的接缝（§12.2 第 2 条）。
- **`load_summary` 不是检索**：它返回一个 `MemorySummary` 投影（带 `stale` 标记），不参与 claim 的筛选。
  最新一条按 `generated_at DESC, summary_id ASC` 取；**`stale = true` 的 summary 不得进入 `ContextPackage`**——它
  的输入（claim／source）已经变过，继续用它就是把旧结论当新事实。

---

# 7. Correction（§286 §177）

## 7.1 形状

```rust
pub struct Correction {
    /// 被纠正的 claim。必须存在且处于 Active。
    pub claim: MemoryClaimId,
    /// **纠正后的新说法**。非空；原样存进 `memory_correction.statement`，
    /// 供下一次合成的 proposal 读取（§7.2 第 4 步）。
    pub statement: String,
    /// 新的 Correction Source。kind **必须**是 SourceKind::ExplicitUserCorrections。
    pub source: MemorySource,
    /// 纠正发生的时间（Unix 毫秒），由调用方给。
    pub occurred_at: i64,
}

pub struct CorrectionOutcome {
    pub correction_source: MemorySourceId,
    pub superseded: MemoryClaimId,
    /// 被标 stale 的 summary 条数（§8.4）。
    pub staled_summaries: usize,
}

pub struct CorrectionRecord {
    pub correction_source: MemorySourceId,
    pub claim: MemoryClaimId,
    /// 纠正后的新说法。下一次合成的输入；它不是 claim（§7.2「不做什么」）。
    pub statement: String,
    pub occurred_at: i64,
}

pub fn correct(tx: &Tx<'_>, correction: &Correction)
    -> Result<CorrectionOutcome, MemoryError>;

pub fn load_correction(tx: &Tx<'_>, claim: &MemoryClaimId)
    -> Result<Option<CorrectionRecord>, MemoryError>;
```

**`statement` 为什么在 correction 行上，而不在 `MemorySource` 上**：§286／§177 只说「产生新的 Correction Source」，
**没有说纠正后的新说法存在哪里**。`MemorySource` 的正文在它自己的存储里（§4.2）；会话存储今天不存在
（§12.1 第 15 条）。若不在 correction 行上存文本，`correct` 之后新说法就只剩一个指向不存在会话的
`external_ref`——**一个空指针**。故本设计把 `statement` 存在 `memory_correction` 行上：
它是下一次合成 proposal 的输入，不是一条 claim（claim 仍只由 `apply_synthesis` 产生，§5.5）。
这条自定形状记入 §12.1 第 17 条。

**Correction 不是后台作业**：§286 的 MUST 是「产生新 source + 旧 claim SUPERSEDED」，它必须在一笔事务里
立刻成立；「重新 synthesis」是 §8 的作业，由调用方在纠正之后另行发起。把纠正做成异步作业会让
「旧 claim 已失效」与「新 source 已在」之间出现一个可观察的空窗——本设计不取。

## 7.2 一笔事务里的步骤与拒绝点

**步骤（顺序即拒绝点；臂名与照片逐条标出）**：

| # | 步骤 | 拒绝臂 | 照片 |
|---|---|---|---|
| 1 | 读 claim；不存在 | `UnknownClaim` | R29 |
| 2 | `validity == Active`；否则 | `ClaimNotActive { validity }`。终态不可再纠正 | R6 |
| 3 | `source.kind == ExplicitUserCorrections`；否则 | `WrongCorrectionSourceKind { kind }` | R7 |
| 4 | `correction.statement` 去空白后非空；否则 | `EmptyStatement` | R29 |
| 5 | 插 `memory_source` 一行（主键冲突，不覆盖） | `DuplicateSourceId` | R30 |
| 6 | 插 `memory_correction` 一行（`claim_id` 有 UNIQUE；**API 到不了这里**，因为第 2 步先挡终态；这是数据库级防线） | `Persist(PersistError)`（约束错） | R8（直接 SQL） |
| 7 | `UPDATE memory_claim SET validity = 'superseded' WHERE claim_id = ? AND validity = 'active'`；受影响行数必须是 1 | `ClaimNotActive`（同一事务内的防线；第 2 步之后行数应为 1） | R5 |
| 8 | 把该 claim 的 scope/anchor 下的 summary 置 `stale = 1`（0 行不报错） | —— | R9 |
| 9 | 返回 `CorrectionOutcome` | —— | R5、R26 |

**它不做什么**：

- **不创建新 claim**。§286 只要求「新的 Correction Source + 旧 claim SUPERSEDED」；§177 的第 5 步是
  「重新 synthesis」，新 claim 由 §8 的合成产生。**纠正后的新说法已经落在 `memory_correction.statement`**，
  下一次合成可以读它；但在合成之前，新的说法在 `memories[]` 里**看不见**，旧的错误说法已经看不见
  （它被 SUPERSEDED）。这是 fail-closed 的一侧：宁可暂时没有，也不继续给错的。
- **不改旧 claim 的 `statement`**：旧 claim 的文本原样保留；它是一条历史记录。
- **不碰旧 claim 的 `sources[]`**（§7.3）。
- **不把 correction statement 当 claim**：它没有 id、没有 scope、没有 validity；它是纠正的输入，不是记忆事实。

## 7.3 为什么 correction source **不**挂到旧 claim 的 `sources[]`

§177 的第 3 步逐字是「加入 correction source」。它可以读成两种：

1. 把 correction source 加入**旧 claim 的 `sources[]`**；
2. 把 correction source 加入 **Source Store**（§172 的 `Source Store`）。

**本设计取第 2 种**，理由是一条图语义：`sources[]` 是「支持这条 claim 成立的来源」。correction source 说的是
「这条 claim 已经不成立了」——把它放进 `sources[]`，图上就会出现「这条 claim 由它的反证支持」的边。
那不是 §177 的意思，也会让 §183 的来源追溯给出相反的结论。

**落法**：correction source 与旧 claim 的关联只在 `memory_correction` 表里（可追溯「谁把它撤了」），
不在 `memory_claim_source` 里。新事实的来源是 correction source，新说法在 `memory_correction.statement`；
下一次合成的 proposal 用 `load_correction` 读这两样，把 correction source 挂到**新 claim** 上。

**代价**：一个逐字读 §177 第 3 步的人会期望在旧 claim 的 `sources[]` 里看到 correction source；
本设计明确说不在那里，并把它记入 §12.1 第 8 条。这是读法，不是规范明文。

## 7.4 §6.4 判据 3 的落点

《工程》§6.4 第 3 条（`docs/02-工程.md:404`）：「用户纠正生成新的 Correction Source，旧 claim 进入 SUPERSEDED」。
本块的兑现是 §7.2 的第 3–7 步，一笔事务；照片是 §10.1 的 R5（正例：新 source 行 + 旧 claim `superseded` +
旧 claim 的 `sources[]` 不变 + claim 总数不变 + `statement` 可读）与 R6／R7（两个反例：非 Active、kind 不对）。
**这一条判据的判据面在本块是完整的**：它不依赖 P6A 的装配，也不依赖 UI。

## 7.5 残余：「用户」不由类型校验

`SourceKind::ExplicitUserCorrections` 是一个**声明**，不是证明。任何能调 `correct` 的调用方都能造一个
kind 正确的 source；本块没有认证、没有会话所有权、没有「这条消息确实来自用户」的检查。
`external_ref` 指向会话只是让追溯有落点，不构成验真。**这是缺口，不是已解决**，记入 §12.1 第 10 条。
它与 P5a 的 `Claim` 自由文本残余同形：类型层能保证「结构里没有那个位置」，保证不了「内容里没有那句话」。

---

# 8. Memory Synthesis 后台作业（§285 §171 §172）

## 8.1 作业形状：两阶段，proposal 在事务外产生

```rust
pub struct SynthesisJobId(String);

impl SynthesisJobId {
    pub fn new(id: impl Into<String>) -> SynthesisJobId;
    pub fn as_str(&self) -> &str;
}

pub enum SynthesisStatus {
    Running,   // begin 之后、apply/fail 之前
    Applied,   // apply 成功
    Failed,    // fail 显式标记
}

pub struct SynthesisRequest {
    pub job: SynthesisJobId,
    pub scope: MemoryScope,
    pub anchor: Option<String>,     // §5.4 的规则同样适用于作业
    pub trigger: SynthesisTrigger,  // §8.2
    pub sources: Vec<MemorySourceId>, // 输入 Source 集合；非空
    pub started_at: i64,
}

pub fn begin_synthesis(tx: &Tx<'_>, request: &SynthesisRequest)
    -> Result<(), MemoryError>;

/// job 不存在 ⇒ UnknownJob；status 非 Running ⇒ JobNotRunning。
pub fn apply_synthesis(
    tx: &Tx<'_>,
    job: &SynthesisJobId,
    proposal: &SynthesisProposal,
    occurred_at: i64,
) -> Result<SynthesisReport, MemoryError>;

/// job 不存在 ⇒ UnknownJob；status 非 Running ⇒ JobNotRunning。
pub fn fail_synthesis(
    tx: &Tx<'_>,
    job: &SynthesisJobId,
    reason: &str,
    occurred_at: i64,
) -> Result<(), MemoryError>;
```

`SynthesisStatus` 带 `as_str` / `parse`（三臂往返；列编码的唯一产生点）。**这三臂是作业记录的形状，
规范未给**：§285 只规定作业存在，不规定状态机；它们不参与任何规范判定，只用于「同一份作业不被 apply 两次」
与失败收尾。

**为什么两阶段**：proposal 可能来自一次模型调用，耗时不可控；本仓的 `Tx` 持有连接锁（`crates/continuum-persist/src/db.rs:141-146`
的文档逐字写了「跨线程在 `Tx` 未提交时调用本方法会阻塞」）。**不能在持有 `Tx` 的期间去等模型**。
故：`begin_synthesis` 一笔短事务写作业与输入；proposal 在事务外产生；`apply_synthesis` 一笔事务落库。
这不是「调度」，是事务边界（调度在 §8.6）。

**作业行的状态机**：`Running` 是唯一可 apply／fail 的状态；`Applied` / `Failed` 都是终态。
apply 两次 ⇒ `Err(JobNotRunning)`（照片 R16）。`fail_synthesis` 只从 `Running` 出发，写
`report = {"error": reason}`；对 `Applied` / `Failed` 调 `fail_synthesis` ⇒ `JobNotRunning`（照片 R39）。
**apply／fail 的 job 不存在 ⇒ `UnknownJob`**（照片 R39）；`UnknownJob` 与 `JobNotRunning` 是两件事：
「没有这份作业」与「这份作业已经结束」。
**apply 失败（`Err`）不改变状态**：调用方回滚后作业仍是 `Running`，可以修正 proposal 再 apply，或调 `fail_synthesis` 收尾。

**崩溃留下的 `Running` 行**：本块**不自动恢复**。恢复（把孤立的 `Running` 标记为 `Failed` 或重跑）
属第 7 层／P0 的恢复钩子；本块提供 `fail_synthesis` 作为那个动作的落点。记入 §12.1 第 6 条与 §10.4。

## 8.2 触发面与判据

```rust
/// §171 的四个时机，逐字：session 结束后 / 系统空闲时 / 大量项目状态改变后 / 用户主动要求时。
pub enum SynthesisTrigger {
    SessionEnded { session: MemorySourceId },
    SystemIdle { since: i64 },
    BulkProjectStateChanged { changed: Vec<MemorySourceId> },
    UserRequested,
}
```

**落库的触发记录**：`begin_synthesis` 把触发落成两列——`trigger_kind` 取 `SynthesisTrigger::as_str`
（四臂：`session_ended` / `system_idle` / `bulk_project_state_changed` / `user_requested`），
`trigger_ref` 取该臂 payload 的 JSON（session id / `since` / changed ids / `null`）。`as_str` 是穷尽 `match`，
不提供 `parse`：作业行不反序列化成触发值，读侧只看 `trigger_kind` / `trigger_ref`。这是作业记录的
**追溯面**，不是调度器：它说明「这次作业是凭什么时机被开的」，不说明「谁在什么时候检查了条件」。

**本块能判的准入条件（`begin_synthesis` 的拒绝点）**：

| # | 判据 | 拒绝臂 |
|---|---|---|
| G0 | `sources` 非空、去重，且每个 source 存在、`removed_at IS NULL` | `EmptySourceSet` / `DuplicateSourceId` / `UnknownSource` / `SourceRemoved` |
| G1 | 触发臂携带的 source（`SessionEnded.session`、`Bulk.changed[]`）都必须∈ `sources`，且存在、未删 | `TriggerSourceOutsideInput` |
| G2 | `SessionEnded.session` 的 kind 必须是 `Conversations` | `WrongTriggerSourceKind` |
| G2b | `Bulk.changed[]` 的每个 source 的 kind 必须是 `Project`——项目状态改变**先落成一条 `Project` source**，再进 `changed` | `WrongTriggerSourceKind` |
| G3 | `Bulk.changed` 非空 | `EmptyTriggerSet` |
| G4 | `SystemIdle.since <= started_at`（只是时间自洽，**不是阈值**） | `InvalidTriggerTime` |
| G5 | scope 与 anchor 的规则同 §5.4（PERSONAL／RUNTIME 无锚，其余必填） | `AnchorRequired` / `AnchorNotAllowed` |
| G6 | `job` 的 id **已存在**于作业表 | `DuplicateJobId` |

**`Bulk.changed` 的 id 不是项目 id，而是 `Project` source 的 id**：项目状态改变先落成一条
`SourceKind::Project` 的 source（§4.1 的第九臂），再进 `changed`。这样「大量项目状态改变」与本块的
Source 词汇表是同一套，不引入 project id 的第二种形状。

**本块判不了的、明确留给第 7 层的**：

- 「系统空闲」的**时长阈值**：§171 只给「系统空闲时」四个字，没有说空闲多久。
- 「大量项目状态改变」的**数量阈值**：§171 只给「大量」。
- 谁在什么时刻检查这些条件（cron／事件驱动／用户请求的入口）、重试、并发、公平性。

**这不是漏判据，是判据的位置**：切分 §八 八.2 第 4 条已把调度判给第 7 层 Background Supervisor
（§299，`docs/spec/05-normative.md:1892`；《工程》§7.1 `:428`、§7.3 `:453`），且协调者已把 P7 搁下。
本块给的是**触发面**：四个时机有类型、有准入检查、有记录（`memory_synthesis_job.trigger_kind` / `trigger_ref`），
但没有「到点自动跑」的机制。**P7 接上时不需要改本块的类型**——它只需要调 `begin_synthesis` / `apply_synthesis` /
`fail_synthesis`。记入 §12.1 第 6 条。

## 8.3 Proposal：一个**不能**表达终态的输入类型

```rust
pub struct ProposedClaim {
    pub id: MemoryClaimId,
    pub statement: String,              // 去空白后非空
    pub sources: Vec<MemorySourceId>,   // 非空、去重
    pub confidence: Confidence,         // 只存不判
    pub supersedes: Vec<MemoryClaimId>, // 显式的退场名单；不推断
}

pub struct SynthesisProposal {
    pub claims: Vec<ProposedClaim>,
    /// §285 的 Memory Summary。None = 本次不生成。
    pub summary: Option<String>,
}
```

**`ProposedClaim` 里没有 `scope`、没有 `validity`、没有 `last_confirmed`、没有 `anchor`**——这四样都由作业给：

- `scope` / `anchor` 来自 `SynthesisRequest`：一次作业只写一个 scope／锚点。跨 scope 的合成要开两个作业。
- `validity` 由 applier 写 `Active`；提案**表达不了** `Superseded` / `Invalidated`（照片 C4）。
- `last_confirmed` 由 applier 从来源时间派生（§5.3.3）。
- `anchor` 由作业给，提案不能把一条 claim 写到另一个项目里。

**`supersedes` 是显式的**：§285 的「冲突处理」在本设计里不是「applier 检测矛盾」，而是「提案方点名哪些旧 claim
退场」。理由：矛盾的判据（两句话什么时候算冲突）规范没有给；让 applier 猜，就是发明一个语义判定器。
**代价**：proposer 漏点一条，旧 claim 就继续 Active；本块不会替它发现（§8.5、§12.1 第 5 条）。

## 8.4 `apply_synthesis` 的规则与不变量

**A1 作业前置**：job 存在（不存在 ⇒ `UnknownJob`）、`status = Running`（否则 ⇒ `JobNotRunning`）；
scope／anchor 从作业行读，**不从提案读**。

**A2 逐条 proposal，按顺序**：

1. `id` 不能与库中已有 claim 重复，也不能与本批前面的 proposal 重复 ⇒ `DuplicateClaimId`。
2. `statement` 去首尾空白后非空 ⇒ `EmptyStatement`。
3. `sources` 非空、去重 ⇒ `EmptySourceSet` / `DuplicateSourceId`。
4. 每个 source 存在且未删 ⇒ `UnknownSource` / `SourceRemoved`。
5. **去重键**：同 `(scope, anchor, 去空白后的 statement)` 且 `validity = Active` 的既有 claim。
   - 命中 ⇒ **合并**：把提案里尚未连到该 claim 的 source 逐条插 `memory_claim_source`；
     `last_confirmed = max(旧值, 新来源 observed_at)`；`confidence` 用提案的值覆盖（§5.3.2：没有序，覆盖是唯一不发明序的动作）。
   - 未命中 ⇒ **新建**：`MemoryClaim::new(提案 id, 作业 scope, statement, sources, confidence, max(observed_at))`，
     插 `memory_claim` + `memory_claim_source`；若作业 scope ∈ {`Project`, `Product`, `Task`}，再插 `memory_claim_anchor`。
   **两种分支都记下本次改动的 `(scope, anchor)`**，供 A3 的摘要失效用。
6. **来源边界**：对「新建」的 claim，全部 source 必须∈作业 `sources`；对「合并」的 claim，**新增的** source
   必须∈作业 `sources`，已连的旧 source 可以在输入集合外（它们不是这次新引入的）。
   违反 ⇒ `ForeignSource`。
7. `supersedes` 逐条：目标存在、`Active`、同 `(scope, anchor)`、且**不是**本条的合并目标 ⇒ 否则 `ScopeMismatch` /
   `ClaimNotActive` / `SupersedeMergeConflict`。通过则 `validity = 'superseded'`，并记下该 `(scope, anchor)`。

**A3 Summary（含旧摘要失效）**：

1. **先失效**：只要本次事务对某个 `(scope, anchor)` 的新建／合并／退场发生了，就把该 `(scope, anchor)` 的
   **既有** `memory_summary` 全部置 `stale = 1`（0 行不报错）。这一步与 A2 的 claim 写入同一事务。
2. **再写新摘要**：`proposal.summary` 为 `Some` 且去空白后非空 ⇒ 插 `memory_summary` 一行：
   `summary_id` 由调用方给的 `occurred_at` + job id 组成（`{job}:{occurred_at}`），`scope` / `anchor` 取作业，
   `body` 原样存（**不解析、不校验格式**，§173 只给了一个人类可读的例子），`job_id` 取作业，`stale = 0`。
   `Some("")` ⇒ `Err(EmptySummary)`；`None` ⇒ 不写——**若本次发生了 claim 变更**（新建／合并／退场），
   旧摘要已按第 1 步置 stale，`load_current_summary` 返回 `None`，`load_summary` 返回的那条带
   `stale = true`；**若没有 claim 变更**，旧摘要保持原状（`stale` 不被这一条改动）。

```rust
pub struct MemorySummary {
    pub id: MemorySummaryId,
    pub scope: MemoryScope,
    pub anchor: Option<String>,
    pub body: String,          // 自由文本；规范未给格式
    pub job: SynthesisJobId,
    pub generated_at: i64,
    pub stale: bool,           // claim 变更（纠正／来源删除／合成新建、合并或退场）后置 true（§9.3 I11）
}

pub struct SynthesisReport {
    pub created: Vec<MemoryClaimId>,
    pub merged: Vec<MemoryClaimId>,
    pub superseded: Vec<MemoryClaimId>,
    pub summary_written: bool,
    /// 本次被置 stale 的旧摘要条数（A3 第 1 步）。
    pub staled_summaries: usize,
}
```

**A4 作业收尾**：`status = Applied`、`finished_at = occurred_at`、`report = SynthesisReport 的 JSON`。

**A5 原子性**：以上全部在一笔 `Tx` 里；`Err` 时调用方必须回滚（本文件不 `commit`，同 P5a 的持久化文件头）。
`apply_synthesis` 不调用 `commit`。

**不变量（逐条编号，§9.3 是落库侧的同一条）**：

- **I1**：一个 `Active` claim 至少有一个 source，且这些 source 都满足 `removed_at IS NULL`。
- **I2**：`Superseded` / `Invalidated` 是终态；没有任何写入口能让它们回到 `Active`。
- **I3**：一个 claim 的 `scope` 与它的 anchor 形状符合 §5.4。
- **I4**：一次作业只写一个 `(scope, anchor)`；提案不能跨。
- **I5**：提案不能创建 source，不能删除 source，不能删除 claim，不能改已有 claim 的 `statement`／`id`／`scope`／`anchor`。
- **I6**：`memory_summary` 只由 `apply_synthesis` 写；`job_id` 指向一个存在的作业，且该作业在同一事务里收尾为 `Applied`。
  数据库层没有「job 必须是 Applied」的约束（先插 summary、后改 status），读侧按 `job_id` 追溯，不按状态过滤。
- **I7**：`last_confirmed` 等于该 claim 全部 source 的 `observed_at` 的最大值（合并时取旧值与新值的 max）。

## 8.5 §285 的五个功能与 §171 的七步：逐项落点

**§285 的五个功能**（`docs/spec/05-normative.md:1611-1627`）：

| §285 的功能 | 本块 | 缺的是哪一步 |
|---|---|---|
| 去重 | **部分**：同 `(scope, anchor, 精确 statement)` 的 Active claim 合并 | 语义级去重（同义改写、跨语言）没有判据；`statement` 是自由文本，本块不解析 |
| 冲突处理 | **部分**：只按 `supersedes` 的显式名单退场 | 矛盾检测的判据（两句话何时矛盾）规范未给；本块不造语义判定器 |
| 更新时效 | **是**：`last_confirmed = max(observed_at)` | 没有时间区间（§5.3.1 的代价） |
| 归纳长期事实 | **否** | 这是 proposer 的工作；本块没有 proposer（§8.1），也不判断「长期稳定」（§172） |
| 生成 Memory Summary | **部分**：存提案给的文本，不生成文本 | 摘要的生成算法与格式规范未给（§173 只给例子） |

**§171 的七步**（`docs/spec/03-product-drive.md:913-929`）：

| §171 的步骤 | 本块的落点 |
|---|---|
| Recent Sources | 作业的 `sources` 输入集合（`begin_synthesis` 的 G0） |
| Relevant historical sources | **不在本块**：proposer／检索侧；本块不建相关性判据 |
| Contradiction detection | **不在本块**：只接受显式 `supersedes` |
| Temporal reconciliation | **部分**：`last_confirmed` 的 max + 显式退场；没有区间推理 |
| Importance estimation | **不在本块**：没有重要性判据 |
| Memory synthesis | `apply_synthesis`：把 proposal 落成 claim／边／summary |
| Memory Summary | `memory_summary` 行（文本投影） |

**这张表是本块的边界声明**：`Memory Synthesis 后台作业` 在本轮是**作业记录 + 事务性 applier**，不是
「一个会自己合成记忆的模型」。proposer 的身份、放置与算法都没有规范依据；本块不替它选。
这与 P5a「只定义获取点、不建 VerificationProfile 生成器」的处置同形（P5a 设计 §5.3）。
**收件人：规范维护者 + 协调者**（§12.1 第 5 条）。

## 8.6 调度边界（第 7 层）

- **本块提供**：作业的创建（`begin_synthesis`）、应用（`apply_synthesis`）、失败标记（`fail_synthesis`），
  以及 §8.2 的八条准入检查（G0、G1、G2、G2b、G3–G6；G2b 是 G2 的 `Bulk` 分支）。
- **本块不提供**：定时器、空闲检测、「大量」的计数、并发控制、重试、崩溃恢复、优先级。
- **P7 接上时的形状**：P7 决定「现在该跑一次合成」，收集 Source 集合，调 `begin_synthesis`；拿 proposal（来自模型或人），
  调 `apply_synthesis`；失败调 `fail_synthesis`。**P7 不需要读本块的任何表**：三个函数就是全部接缝。
- **一条据实**：`SynthesisTrigger::UserRequested` 没有「谁请求的」字段。§171 只说「用户主动要求时」，
  没有给请求者身份；本块不造一个 `UserRequest` 类型。若 P7 或 UI 需要记录请求者，那是一个新字段／新类型，
  记入 §12.1 第 6 条。

---

# 9. 落库

## 9.1 迁移号段

见 §3.3：**P6 档 `190`–`199`，本块 `190`**。迁移函数名 `p6c_memory_migrations()`，返回 `Vec<Migration>`，
形状照 `crates/continuum-verify/src/persist.rs:63` 的 `p5a_verify_migrations()`。

## 9.2 表与列（八张）

```
memory_source          source_id PK, kind, external_ref, observed_at, removed_at
memory_claim           claim_id PK, scope, statement, validity, confidence(TEXT/JSON), last_confirmed
memory_claim_source    (claim_id, source_id) PK
memory_claim_anchor    claim_id PK, anchor_id
memory_correction      source_id PK, claim_id UNIQUE, statement, occurred_at
memory_synthesis_job   job_id PK, scope, anchor_id, trigger_kind, trigger_ref(TEXT/JSON),
                       status, started_at, finished_at, report(TEXT/JSON)
memory_synthesis_input (job_id, source_id) PK
memory_summary         summary_id PK, scope, anchor_id, body, job_id, generated_at, stale
```

索引（六条）：

```
idx_memory_source_kind              ON memory_source(kind)
idx_memory_source_ref               ON memory_source(external_ref)
idx_memory_claim_source_source      ON memory_claim_source(source_id)
idx_memory_claim_scope_validity_time ON memory_claim(scope, validity, last_confirmed)
idx_memory_claim_anchor             ON memory_claim_anchor(anchor_id)
idx_memory_summary_scope_time       ON memory_summary(scope, anchor_id, generated_at)
```

**「七字段 = 六列 + 一张边表」**：`memory_claim` 的标量列恰好是 §283 的六个标量字段；
`sources[]` 落在 `memory_claim_source`。这张表**没有**第七个标量列（没有 status、created_at、anchor_id），
与 §5.1 的类型事实逐条对应。

**外键**：`memory_claim_source` 的两个方向、`memory_claim_anchor.claim_id`、`memory_correction` 的两个方向、
`memory_synthesis_input` 的两个方向、`memory_summary.job_id` 都声明 `FOREIGN KEY`；`Db::open_with` 已开
`PRAGMA foreign_keys=ON`（`crates/continuum-persist/src/db.rs:74-79`）。软删除的 source 仍在表里，故外键不阻碍 §4.4。

## 9.3 不变量（落库侧；前七条与 §8.4 的 I1–I7 一一对应，I8–I13 是落库侧的补充）

1. **I1 来源非空**：`memory_claim` 的每一行 **`Active`** 至少有一条 `memory_claim_source`；且这些 source 的 `removed_at IS NULL`。
   SQLite 不能用一条简单约束表达「至少一条」，故由**写入口**（`apply_synthesis`）+ 测试里的诊断 SQL 把关；
   诊断 SQL 不导出为 API，不进入热路径。
2. **I2 终态不可逆**：`validity` 的 UPDATE 只有两条语句——`active → superseded`（§7.2 第 7 步、§8.4 A2 第 7 条）、
   `active → invalidated`（§4.4 第 2 步）；作业收尾不动 validity。没有 `superseded → *` 或
   `invalidated → *` 的语句。
3. **I3 scope 与 anchor**：`scope ∈ {personal, project, product, task, runtime}`；`project`／`product`／`task`
   必须有 `memory_claim_anchor` 行，`personal`／`runtime` 必须没有。写入口检查；诊断查询复核。
4. **I4 作业 scope**：`memory_claim` 的 scope/anchor 等于写它的作业的 scope/anchor（§8.4 A1）。
5. **I5 提案不造 source／不删**：`apply_synthesis` 的 SQL 里没有 `INSERT INTO memory_source`、没有 `DELETE`、
   没有对 `statement`／`claim_id`／`scope` 的 UPDATE。
6. **I6 summary 的作业**：`memory_summary.job_id` 指向 `memory_synthesis_job`；`body` 非空；`stale ∈ {0,1}`。
7. **I7 last_confirmed**：等于该 claim 全部 source 的 `observed_at` 最大值（合并时是旧值与新值的 max）。
8. **I8 correction 唯一**：`memory_correction.claim_id` 有 UNIQUE；一个 claim 最多被纠正一次（它之后是终态）。
   **这是数据库级防线**：`correct` 的第 2 步先挡终态，第二次纠正到不了这里；API 层因此没有对应的错误臂。
   照片 R8 是直接对表做第二次 INSERT（不经 API）。
9. **I9 correction 的 kind**：`memory_correction.source_id` 指向的 `memory_source.kind` 必须是
   `explicit_user_corrections`。写入口检查；诊断查询复核（SQLite 的 FK 不能跨列判 kind）。
10. **I10 软删除**：`memory_source` 没有 DELETE 语句；`removed_at` 一旦非空不再回到 NULL。
11. **I11 摘要的 stale**：`correct`、`remove_source` 与 `apply_synthesis`（当本次新建／合并／退场了 claim）
    都把受影响 `(scope, anchor)` 的既有 summary 置 stale；`apply_synthesis` 新写的 summary 是 `stale = 0`。
    没有「自动重新生成」——重新生成是一次新的作业。
12. **I12 迁移只产生一次**：`p6c_memory_migrations()` 是 `190` 的唯一产生点；SQL 此后不得再改
    （`Db::migrate()` 按 version 跳过已应用者，改号或改 SQL 对已建库都无效——同 P5a 的 130、P4 的 110）。
13. **I13 correction statement 非空**：`memory_correction.statement` 去空白后非空；它由 `correct` 第 4 步把关，
    不进入 claim 表。

## 9.4 写函数与事务

| 函数 | 写什么 | 事务 |
|---|---|---|
| `insert_source(tx, &MemorySource)` | `memory_source` 一行 | 调用方持有 `Tx`；不 `commit` |
| `remove_source` | `memory_source.removed_at` + 受影响 claim 的 `validity` + summary stale | 同上，原子 |
| `correct` | `memory_source` + `memory_correction`（含 `statement`）+ 旧 claim `validity` + summary stale | 同上，原子 |
| `begin_synthesis` | `memory_synthesis_job` + `memory_synthesis_input` | 同上 |
| `apply_synthesis` | `memory_claim` / `memory_claim_source` / `memory_claim_anchor` / `memory_summary` / 旧摘要 stale / 作业行 | 同上，原子 |
| `fail_synthesis` | 作业行 `status` + `report` | 同上 |

**Source 的生产者**：`insert_source` 是唯一写入口，但本块不自动产生 source。调用方是装配方或 P7——P7 在
`begin_synthesis` 之前把这次要合成的来源逐条落库；**P6A 不是生产者**：它没有会话存储，也没有落库入口
（P6A §11 第 17 条的收件人是协调者 ＋ P1）。没有生产者时
`memory_source` 为空，`begin_synthesis` 的 G0 会拒绝空输入；这是上游缺口（§12.1 第 15 条），不是本块的遗漏。

**返回 `Err` 之后调用方必须回滚**：本文件的写函数都不 `commit`，且失败路径上可能已经写了前几行
（例如 `correct` 先插 source、后撞 unique）。`Tx` 没有保存点接口，回滚只能由调用方丢弃句柄——
这条纪律照 P5a 的持久化文件头（`crates/continuum-verify/src/persist.rs:4-12`），不是本块新定的。

**读函数**：`retrieve_claims`、`load_source`、`load_sources`、`load_summary`、`load_current_summary`、
`load_correction` 只读，也走调用方的 `Tx`。

## 9.5 事件与审计：零（设计选择，不是规范强制）

- **零事件**：§310 的九类是 **MUST 最小清单**，不是封闭集合；里面没有记忆类事件。本块按最小读法不新增
  第十类。后果据实：记忆的写入、纠正、合成**不在事件流上**，不能经事件重放恢复；表本身是记录。
- **零审计**：§313 的八类同样是 **MUST 最小清单**，不是封闭集合；里面没有记忆类。本块按最小读法不新增
  第九类。
- **`correct` 是否落入 `user approvals` 的排除性分析**：`AuditKind::UserApprovals`
  （`crates/continuum-events/src/audit.rs:31`）是 §313 的「用户批准」——对 effect／authority／contract 的批准；
  `correct` 是对**记忆事实**的纠正，不是对某个动作的批准。故本设计判定它**不**落入该类别，不写审计。
  **这是读法，不是规范明文**：若规范维护者认为「用户批准」涵盖记忆纠正，本块要补一条 `UserApprovals`
  审计；收件人：规范维护者。
- **为什么这是缺口而不是省事**：§286 的纠正是「用户的显式动作」，本仓的审计面却没有它的位置。
  若规范要求「谁改了长期记忆」可审计，需要一个新的 `AuditKind`；**本块不造**——这是本块的选择，
  不是规范禁止（§310／§313 是最小清单），代价与收件人见 §12.1 第 7 条。
- **依赖后果**：本 crate 不依赖 `continuum-events`（§3.2），`ALLOWED` 行因此只有 `continuum-persist`。

---

# 10. 判据与照片

## 10.1 运行期（正例 + 反例成对；缺一条即不算钉住）

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| R1 | `p6c_memory_migrations()` 的版本恰为 `[190]`；八张表与六条索引逐名断言；迁移后表在库里 | 迁移的唯一性与完整性 |
| R2 | `insert_source` → `load_source` 往返；`removed_at` 为 `None` | Source 的形状 |
| R3 | 两条 claim 引用 source A，一条不引用；`remove_source(A)` 后两条 `invalidated`、第三条仍 `active` | §4.4 的**两侧**（失效与不误伤） |
| R4 | `remove_source` 后 `load_sources` 仍能读到该 source（`removed_at` 非空）；`retrieve_claims` 不返回其 claim | 追溯可见 / 上下文不可见，**两侧** |
| R5 | `correct`：新 source 行在、旧 claim `superseded`、旧 claim 的 `sources[]` 不变、claim 总数不变、`memory_correction.statement` 可读 | §6.4 判据 3 的正例 |
| R6 | 对 `Superseded` claim 再 `correct` ⇒ `Err(ClaimNotActive)` | 终态不可再纠正 |
| R7 | `correct` 传 `SourceKind::Files` ⇒ `Err(WrongCorrectionSourceKind)` | §7.2 第 3 步 |
| R8 | **直接对 `memory_correction` 做第二次 INSERT**（不经 `correct`）⇒ 约束错；这是数据库级防线，不是 API 判据 | `UNIQUE(claim_id)`（§9.3 I8） |
| R9 | 纠正后该 scope 的 summary `stale = 1`；另一 scope 的 summary 仍 `stale = 0` | I11 的**两侧** |
| R10 | 合成新建：claim 的 `last_confirmed == max(sources.observed_at)`；七字段可读 | §5.3.3 |
| R11 | 合成合并：同 statement 的提案只留一行 claim、两条 source 边、`last_confirmed` 取 max、`confidence` 被覆盖 | §8.4 A2 第 5 条 |
| R12 | 同 scope 不同 statement 的两条提案 ⇒ 两行 claim | 去重键的**另一侧**（不是「一律合并」） |
| R13 | 提案引用不在作业输入集合里的 source ⇒ `Err(ForeignSource)`，且没有任何 claim／边／summary 落库 | §8.4 A2 第 6 条 + 原子性 |
| R14 | 提案引用 `removed_at` 非空的 source ⇒ `Err(SourceRemoved)` | §4.4 与合成的接缝 |
| R15 | 提案 `sources` 为空 ⇒ `Err(EmptySourceSet)` | I1 |
| R16 | `apply_synthesis` 第二次（job 已 `Applied`）或对 `Failed` 作业 apply ⇒ `Err(JobNotRunning)` | §8.1 状态机 |
| R17 | `supersedes` 指向另一个 anchor 的 claim ⇒ `Err(ScopeMismatch)` | §8.4 A2 第 7 条 |
| R18 | `supersedes` 后目标 `superseded`；此后再提同一 statement ⇒ **新建**一行 `Active`（不是复活旧行） | 终态不可复活 + 新事实的新 id |
| R19 | `apply_synthesis` 带 summary ⇒ `memory_summary` 一行、`job_id` 正确；`load_summary` 返回最新一条；`stale` 可读；`staled_summaries` 计数正确 | §8.4 A3 |
| R19b | 新建／合并／退场且 `proposal.summary = None` ⇒ 旧 summary `stale = 1`，`load_current_summary` 返回 `None` | A3 第 1 步（I3） |
| R20 | `retrieve_claims` 默认不含 `Superseded`／`Invalidated`；`include_terminal = true` 含；scope／anchor 过滤；顺序 `last_confirmed DESC, claim_id ASC` | §6.2 的四个面 |
| R21 | `limit = 0` ⇒ `Ok(vec![])`（不是 `Err`）；`scopes` 为空 ⇒ `Err(EmptyScopeSet)` | 检索面的两种边界 |
| R22 | `Confidence` 的任意 JSON 往返**值相等（`==`）**；没有默认值 | §5.3.2（`serde_json` 会规范化空白／键序，故不承诺逐字） |
| R23 | `MemorySource::new` 空 `external_ref` ⇒ `Err(EmptyExternalRef)` | §4.2 的构造检查 |
| R24 | `correct` 后、合成前，`retrieve_claims` 看不到旧 claim，也看不到新说法（新说法在 `memory_correction.statement`，不在 claim 表） | §7.2 的「不创建新 claim」代价，**照实钉住** |
| R25 | `MemoryStore::new(&tx)` 作为 `&dyn MemoryRetrieval` 调 `retrieve(&q)`，与 `retrieve_claims(tx, &q)` 返回同一组 claim | §3.5 的 trait 与自由函数是同一实现 |
| R26 | `load_correction(claim)` 返回 `CorrectionRecord { statement }`；下一次 `apply_synthesis` 的 proposal 用该文本可建出新 claim | I1：纠正内容不丢，且不变成 claim |
| R27 | `RetrievalQuery::new([Project], None, ..)` ⇒ `Err(AnchorRequired)`；`new([Personal], Some(id), ..)`、`new([Personal, Project], Some(id), ..)`、`new([Project, Product], Some(id), ..)` ⇒ `Err(AnchorNotAllowed)`；`[Project]` 带 anchor ⇒ `Ok` | §5.4 的读取侧规则，**两侧** |
| R28 | 同一 `(scope, anchor)` 有 stale 与 fresh 两条摘要：`load_summary` 返回最新（可能 stale），`load_current_summary` 返回最新 fresh；全 stale ⇒ `None` | `stale` 的读侧消费 |
| R29 | `correct` 未知 claim ⇒ `Err(UnknownClaim)`；`statement` 为空 ⇒ `Err(EmptyStatement)` | §7.2 第 1、4 步 |
| R30 | `correct` 的 `source.id` 已在 `memory_source` ⇒ `Err(DuplicateSourceId)`，且 claim 仍 `Active`、无 correction 行 | §7.2 第 5 步 + 原子性 |
| R31 | `insert_source` 重复 id ⇒ `Err(DuplicateSourceId)`；`begin_synthesis`／`apply_synthesis` 的 source 列表含重复 id ⇒ 同一臂 | §4.2、§8.2 G0、§8.4 A2 第 3 条 |
| R32 | `load_summary(Project, None)`／`load_current_summary(Project, None)` ⇒ `AnchorRequired`；同一入口传 `Personal` + `Some(_)` ⇒ `AnchorNotAllowed`；`begin_synthesis` 的 `SynthesisRequest.scope` 同规则 | §5.4 的同一规则在三个入口上 |
| R33 | `remove_source` 未知 id ⇒ `UnknownSource`；`begin_synthesis`／`apply_synthesis` 的 source 不存在 ⇒ 同一臂 | §4.4 第 1 步、§8.2 G0、§8.4 A2 第 4 条 |
| R34 | `remove_source` 第二次 ⇒ `Err(SourceAlreadyRemoved)` | §4.4 第 1 步 |
| R35 | `begin_synthesis` 的 `sources` 为空 ⇒ `Err(EmptySourceSet)`；`apply_synthesis` 的 proposal `statement` 为空 ⇒ `Err(EmptyStatement)` | §8.2 G0、§8.4 A2 第 2 条 |
| R36 | `apply_synthesis` 的 claim id 已在库中或本批重复 ⇒ `Err(DuplicateClaimId)`；`supersedes` 指向不存在的 claim ⇒ `Err(UnknownClaim)` | §8.4 A2 第 1、7 条 |
| R37 | `apply_synthesis` 的 `supersedes` 指向 `Superseded` claim ⇒ `Err(ClaimNotActive)` | §8.4 A2 第 7 条 |
| R38 | `begin_synthesis` 的 job id 已存在 ⇒ `Err(DuplicateJobId)` | §8.2 G6 |
| R39 | `apply_synthesis`／`fail_synthesis` 的 job 不存在 ⇒ `Err(UnknownJob)`；对 `Applied` 作业调 `fail_synthesis` ⇒ `Err(JobNotRunning)` | §8.1 状态机 |
| R40 | `apply_synthesis` 的 `summary = Some("")` ⇒ `Err(EmptySummary)` | §8.4 A3 第 2 步 |
| R41 | `apply_synthesis` 的 `supersedes` 指向本条的合并目标 ⇒ `Err(SupersedeMergeConflict)` | §8.4 A2 第 7 条 |
| R42 | `begin_synthesis` 的四个触发拒绝：source 不在输入 ⇒ `TriggerSourceOutsideInput`；kind 不对 ⇒ `WrongTriggerSourceKind`；`Bulk.changed` 空 ⇒ `EmptyTriggerSet`；`SystemIdle.since > started_at` ⇒ `InvalidTriggerTime` | §8.2 G1–G4 |
| R43 | `begin_synthesis` 的 source 已 `removed_at` 非空 ⇒ `Err(SourceRemoved)` | §8.2 G0 |

**§10.1 的附表：拒绝点 → 臂 → 照片（全部公开操作）**

`Persist(PersistError)` 不在这张表里：它不是逻辑拒绝臂，而是任何 SQL 执行失败的透传（无照片可拍，见 §10.4）。
除它之外，`MemoryError` 的每一枚臂都能在下表找到**具名拒绝点**与**照片**；原先点名的死路已在 §3.5 末尾说明。

| 公开操作 | 具名拒绝点 | 臂 | 照片 |
|---|---|---|---|
| `MemorySource::new` | `external_ref` 去空白为空 | `EmptyExternalRef` | R23 |
| `insert_source` | `source_id` 已存在 | `DuplicateSourceId` | R31 |
| `remove_source` | id 不存在 | `UnknownSource` | R33 |
| `remove_source` | `removed_at` 已非空 | `SourceAlreadyRemoved` | R34 |
| `RetrievalQuery::new` | `scopes` 为空 | `EmptyScopeSet` | R21 |
| `RetrievalQuery::new` | anchored scope 缺 anchor | `AnchorRequired` | R27 |
| `RetrievalQuery::new` | anchor 与 scopes 不匹配 | `AnchorNotAllowed` | R27 |
| `load_summary` / `load_current_summary` | scope/anchor 规则不满足 | `AnchorRequired` / `AnchorNotAllowed` | R32 |
| `correct` | claim 不存在 | `UnknownClaim` | R29 |
| `correct` | claim 非 `Active` | `ClaimNotActive` | R6 |
| `correct` | source kind 不是 `ExplicitUserCorrections` | `WrongCorrectionSourceKind` | R7 |
| `correct` | `statement` 为空 | `EmptyStatement` | R29 |
| `correct` | correction source id 已存在 | `DuplicateSourceId` | R30 |
| `begin_synthesis` | `sources` 为空 | `EmptySourceSet` | R35 |
| `begin_synthesis` | source 不存在 | `UnknownSource` | R33 |
| `begin_synthesis` | source 已删 | `SourceRemoved` | R43 |
| `begin_synthesis` | `sources` 重复 | `DuplicateSourceId` | R31 |
| `begin_synthesis` | 触发 source 不在输入集合 | `TriggerSourceOutsideInput` | R42 |
| `begin_synthesis` | 触发 source kind 不对 | `WrongTriggerSourceKind` | R42 |
| `begin_synthesis` | `Bulk.changed` 为空 | `EmptyTriggerSet` | R42 |
| `begin_synthesis` | `SystemIdle.since > started_at` | `InvalidTriggerTime` | R42 |
| `begin_synthesis` | scope/anchor 规则不满足 | `AnchorRequired` / `AnchorNotAllowed` | R32 |
| `begin_synthesis` | job id 已存在 | `DuplicateJobId` | R38 |
| `apply_synthesis` | job 不存在 | `UnknownJob` | R39 |
| `apply_synthesis` | job 非 `Running` | `JobNotRunning` | R16 |
| `apply_synthesis` | claim id 已存在或本批重复 | `DuplicateClaimId` | R36 |
| `apply_synthesis` | proposal `statement` 为空 | `EmptyStatement` | R35 |
| `apply_synthesis` | proposal `sources` 为空 | `EmptySourceSet` | R15 |
| `apply_synthesis` | proposal `sources` 重复 | `DuplicateSourceId` | R31 |
| `apply_synthesis` | source 不存在 | `UnknownSource` | R33 |
| `apply_synthesis` | source 已删 | `SourceRemoved` | R14 |
| `apply_synthesis` | 新来源不在作业输入集合 | `ForeignSource` | R13 |
| `apply_synthesis` | `supersedes` 目标不存在 | `UnknownClaim` | R36 |
| `apply_synthesis` | `supersedes` 目标非 `Active` | `ClaimNotActive` | R37 |
| `apply_synthesis` | `supersedes` 目标跨 scope/anchor | `ScopeMismatch` | R17 |
| `apply_synthesis` | `supersedes` 目标是本条合并目标 | `SupersedeMergeConflict` | R41 |
| `apply_synthesis` | `summary = Some("")` | `EmptySummary` | R40 |
| `fail_synthesis` | job 不存在 | `UnknownJob` | R39 |
| `fail_synthesis` | job 非 `Running` | `JobNotRunning` | R39 |

**读操作**（`retrieve_claims`、`load_source`、`load_sources`、`load_correction`）没有逻辑拒绝臂；它们只透传
`Persist`。**这一点要单独写出来**：否则「没有臂」会被读成「没有覆盖」。

## 10.2 编译期（`trybuild`，本仓既有做法）

| # | 写什么 | 钉的是哪一条 |
|---|---|---|
| C1 | `claim.status` / `claim.anchor`（不存在的访问器） | 七字段之外没有第八个可读字段（§5.1；精确字段数由 S7 钉） |
| C2 | 在 crate 外调 `MemoryClaim::new(..)` | 唯一产生点；claim 只能由合成来（§5.5） |
| C3 | `MemoryScope::Session` | 五个 scope，没有 Session（§5.2） |
| C4 | `ProposedClaim { validity: .. }` 或 `{ last_confirmed: .. }` | 提案表达不了终态与时间（§8.3） |
| C5 | `MemorySource { .. }` 字面量 | 字段私有，构造点唯一（§4.2） |
| C6 | `let _ = a < b;`（两枚 `Confidence`） | 没有 `PartialOrd`（§5.3.2）。**注意**：内层 `serde_json::Value` 本身也没有 `PartialOrd`，故这条负例红的不只靠 `Confidence` 的派生；正控制 C7 覆盖「整块可编过」 |
| C7 | 正控制：五个 scope 与九个 `SourceKind` 各写一遍，能编过 | 防止 C1–C6 是「整块编不过」的假照片 |

## 10.3 源码面（`grep` / 文件级断言）

| # | 断言 | 钉的是哪一条 |
|---|---|---|
| S1 | `Cargo.toml` 的 members 里 `crates/continuum-memory` 恰一次；`crates/continuum-runtime/Cargo.toml` 有 `continuum-memory` 的 path 依赖恰一次 | §3.3 第 1、2 行 |
| S2 | `ALLOWED` 里 `("continuum-memory", &["continuum-persist"])` 恰一次；`continuum-runtime` 的 `ALLOWED` 行含 `"continuum-memory"` 恰一次；`continuum-memory/Cargo.toml` 的**内部 workspace crate** 只有 `continuum-persist`（外部依赖 `serde_json` 不在 `ALLOWED` 里） | §3.2、§3.3 第 3、4 行、切分 §四第 5 条 |
| S3 | `src/source.rs` 里没有 `use crate::claim`（也不得引用 claim 侧的任何类型） | 入度为零的**模块方向**（§3.4） |
| S4 | `continuum-memory` 的 `src/` 里 `EventType` / `AuditKind` / `append_event` / `append_audit` 零命中 | §9.5 零事件零审计 |
| S5 | `Migration::new(190` 只在 `src/persist.rs` 的 `p6c_memory_migrations` 里出现一次 | §9.3 I12 |
| S6 | `MemoryScope` 的变体恰五枚，`ALL` 的长度字面量是 `5`；`SourceKind` 的是 `9` | §4.1、§5.2 |
| S7 | `memory_claim` 的列清单恰好六列（`claim_id, scope, statement, validity, confidence, last_confirmed`） | §9.2 的「七字段 = 六列 + 边表」 |
| S8 | `Validity` 的 `as_str` / `parse` 对三臂往返；`superseded` 的编码逐字是 §286 的词（大小写按本仓约定） | §5.3.1 |
| S9 | 八张表在迁移 SQL 里**各有且仅有一个 `CREATE TABLE`**（不是「表名只出现一次」——表名还会出现在 INSERT 与索引名里） | §9.2 |
| S10 | `crates/continuum-runtime/Cargo.toml` 有 `continuum-memory` 的 path 依赖；`runtime_migrations()` 有 `p6c_memory_migrations()` 恰一次 | §3.3 第 2、5 行、I5 |
| S11 | `MemoryClaim` 的字段名恰为 §283 的七个（`grep` 定义处）；`MemorySource` 恰为 §4.2 的五个；没有第八个字段 | §5.1、§4.2、C1 |

## 10.4 拍不出照片的地方（据实）

1. **「用户」是用户**：§7.5。类型只能要求 kind，不能证明来源是人。
2. **调度的时机**：§8.6 把时机判给 P7；本块没有「空闲多久算空闲」的照片。
3. **合成的质量**：同义改写去重、矛盾检测、重要性估计都没有判据；本块拍不到「合成得好不好」，
   只拍得到「proposal 的形状与不变量」。
4. **§172 的「只保存长期稳定事实」**：applier 判不了「长期稳定」；它只能判来源非空、scope 合法。
   这条是 proposer 的政策，不是本块的运行期判据。
5. **物理擦除／forget**：§4.4 是软删除；本块没有「数据真的不存在了」的照片。
6. **Personal Memory 的放置**：§182 不在本块；`scope = personal` 的行落在哪个节点不由本块决定。
7. **「删除 claim 不影响 Source」**：本块没有删除 claim 的路径（§4.3），故这句话没有操作面，也没有照片。
8. **崩溃后孤立的 `Running` 作业**：本块不自动恢复（§8.1）；恢复的照片在 P7／P0 的恢复面。
9. **事件流上的记忆变更**：§9.5 零事件；没有「从事件重放记忆」的照片。
10. **`Persist(PersistError)` 的透传**：它不是逻辑拒绝臂；可以拍「坏 SQL／约束错」的数据库错误，但它不对应
    任何规范判据，故不列入 §10.1 的拒绝点表（表里只列逻辑臂）。

---

# 11. 明确不做（语义层清单）

| 不做的东西 | 为什么 | 归属 / 收件人 |
|---|---|---|
| 语义去重（同义改写、跨语言） | 规范没有给「同义」的判据；`statement` 是自由文本 | proposer；规范维护者（§12.1 第 5 条） |
| 矛盾检测 | 规范没有给「矛盾」的判据；本块只接受显式 `supersedes` | proposer；规范维护者 |
| 重要性估计 | §285 只给功能名，没有取值域 | proposer；规范维护者 |
| Memory Summary 的文本生成 | §173 只给人读的例子，没有格式／语法 | proposer；规范维护者 |
| 「只保存长期稳定事实」（§172）的判定 | 没有「长期稳定」的判据 | proposer；规范维护者 |
| TTL / decay / archived | §74 要求支持 TTL／decay／superseded／invalidated／archived；本块只有 `Active`／`Superseded`／`Invalidated`，没有 `Archived`，也没有 TTL／decay 的机制 | 规范维护者 + 协调者（§12.1 第 16 条） |
| Runtime Learning 的产物规则、Protected Core 的强制 | 切分 §五判给 P6D | P6D |
| Context Compiler、`ContextPackage`、信任标签 | 切分 §五判给 P6A／P6B | P6A／P6B |
| 执行器的接线、节点状态迁移 | 切分 §四第 4 条；执行器不在 P6 | 第 3 层 |
| 后台作业的调度器 | §299 属第 7 层；协调者已把 P7 搁下 | P7／协调者 |
| 事件的第十类、审计的第九类 | §310／§313 给的是 **MUST 最小清单**，不是封闭集合；本块按最小读法不新增 | 规范维护者 + 协调者（§12.1 第 7 条） |

---

# 12. 遗留与对账

## 12.1 遗留（逐条具名收件人；「缺的是哪一步」写到步骤）

1. **§284 的五个 scope 与《总纲》§6.2 的五级作用域表相抵**（§2.2）：§284 是 `PERSONAL`／`PROJECT`／`PRODUCT`／
   `TASK`／`RUNTIME`（原文是「**至少**支持」，§179 同）；《总纲》`01-总纲.md:1021-1027` 是
   Session／Task／Project／Personal／Runtime。本设计按切分 §二第 1 条把这五枚冻成**本块的闭集**，
   `Session` 没有臂。**缺的是「Session 是否要一个存储面、它是不是第六个 scope、还是它根本不属于长期记忆」
   这一步的裁定。收件人：规范维护者 + 协调者。**
2. **§283 没有实例绑定**（§5.4）：七个字段只有 scope 类别，没有 project／product／task 的实例 id。
   本设计用 `memory_claim_anchor` 侧表补，代价是 `MemoryClaim` 类型看不见锚点。**缺的是「§283 是否要加一个
   scope 实例字段（第八字段）」这一步；在那之前，侧表是唯一不违反「七字段逐字」的落法。收件人：规范维护者 + 协调者。**
3. **§282 的六词与 §170 的八类不是同一张表**（§2.2、§4.1）：本设计取并集九臂。
   **缺的是「哪一份是穷尽表」这一步的裁定**；裁定后删臂是一次编译期改动（`ALL` 的数目 + 两个穷尽 `match`）。
   收件人：规范维护者。
4. **§174 的 temporal validity 与 confidence 历史没有槽位**（§5.3.1、§5.3.2）：`validity` 被用作生命周期
   （`Active`／`Superseded`／`Invalidated`），表达不了「从何时到何时有效」的区间；`confidence` 合并时覆盖旧值，
   不留历次判断。**缺的是「§283 的 `validity` 到底是状态还是区间、`confidence` 要不要一张修订表」这一步。
   收件人：规范维护者。**
5. **§285 的合成算法没有规范依据**（§8.5）：去重、冲突、时效、归纳、摘要五项里，本块只完整落了「更新时效」，
   其余是部分或没有。proposer 不建。**缺的是「谁产生 proposal、它凭什么判同义／矛盾／重要」这一步。
   收件人：规范维护者 + 协调者。**
6. **§171 的「系统空闲」「大量」没有阈值**（§8.2）：本块只给触发面与准入检查，调度判给 P7。
   **缺的是「空闲时长、状态改变数量、检查频率、重试与恢复」这一步。收件人：协调者 + P7（§八 八.2 第 4 条）。**
7. **§310／§313 没有记忆类事件与审计**（§9.5）：§310／§313 给的是 **MUST 最小清单**，不是封闭集合；
   记忆的写入、纠正、合成不在事件流上，也不进审计链。**缺的是「长期记忆的变更是否需要可观察／可审计、
   若需要则加哪一类」这一步。收件人：规范维护者 + 协调者。**
8. **§177 第 3 步的「加入 correction source」有第二种读法**（§7.3）：本设计取「加入 Source Store」，
   不让 correction source 挂到旧 claim 的 `sources[]`。**缺的是「这一句是加入 claim 的来源表还是加入来源库」
   这一步的裁定。收件人：规范维护者。**
9. **§9.2 的「入度为零」与持久化相抵**（§3.4）：本 crate 依赖 P0 的 `continuum-persist`；同表列为源点的
   `continuum-node` 也依赖 `continuum-artifact`。**缺的是「P0 基础设施算不算一条上游、这句话是按层组件还是按
   crate 图读」这一步。收件人：规范维护者 + 协调者。**
10. **§286 的「用户」不可验真**（§7.5）：类型只能要求 `SourceKind::ExplicitUserCorrections`，不能证明来源是人。
    **缺的是「纠正请求的认证／会话所有权由谁判」这一步。收件人：P2（边界层）+ 协调者。**
11. **§176 的 explicit remember 没有入口**：本块没有 `explicit_by_user` 标志，也没有「记住这个」的 source 类型
    （§170／§282 都没有这一类）。**缺的是「显式记住落到哪一臂、要不要一个标志」这一步。收件人：规范维护者。**
12. **forget／物理擦除不在本块**（§4.4）：本块的删除是软删除，不满足「数据真的不存在」的读法。
    **缺的是「谁有权擦、擦哪些表（source／claim／correction／summary）、擦除后追溯怎么办」这一步。
    收件人：规范维护者 + 协调者。**
13. **P5a 的 `Evidence` 与 Memory Source 之间没有桥**（§3.2）：§170 的八类里没有 Evidence；工具结果能不能成为
    长期记忆来源，规范没有说。**缺的是「Evidence → Source 的转换是否存在、若存在落到哪一臂」这一步。
    收件人：P5a 的持有方 + 协调者。**
14. **§182 的 Personal Memory 隐私不在本块**（§6.3）：本块只按 scope 过滤，不做放置与 capability 判定。
    **缺的是「personal claim 的读取在哪个节点、由谁授权」这一步。收件人：P3e（计算放置）+ P2（边界层）。**
15. **`Conversations` 这一类来源今天没有存储面**（§4.2）：`crates/` 下没有会话记录；`continuum-canonical` 的
    `ResolutionScope::Conversation` 是 §274 的解析范围，不是会话本身。`external_ref` 可以写一个会话 id，
    但没有任何组件能解析它，也没有组件负责产生它。**这是 P6A 与 P6C 共享的上游缺口**（P6A 的
    `Conversation` 原料同样没有落点）。**缺的是「会话由哪一层持有、id 是什么形状、谁写它」这一步。
    收件人：协调者 + P1。**
16. **§74 的 TTL／decay／archived 没有落点**（§11）：本块的 `Validity` 只有 `Active`／`Superseded`／
    `Invalidated`，没有 `Archived`，也没有 TTL／decay 的机制；§283 的 `validity` 被用作生命周期槽位
    （§5.3.1），更放不下过期策略。**缺的是「过期策略由谁执行、`Archived` 是否成为第四态、TTL／decay
    的判据是什么」这一步。收件人：规范维护者 + 协调者。**
17. **§286／§177 没说纠正后的新说法存在哪里**（§7.1）：本设计把它存在 `memory_correction.statement`，
    作为下一次 synthesis 的输入；它不是 claim（claim 仍只由 `apply_synthesis` 产生）。**缺的是
    「纠正文本的规范承载物（source 正文？correction 行？claim 草稿？）」这一步。收件人：规范维护者 + 协调者。**

## 12.2 与各块的对账

1. **P6A（Context Compiler）**：本块冻结 §3.5 的三枚类型、访问器、`insert_source`、`MemoryRetrieval` trait
   与 `MemoryStore`；**已对齐（据实）**：P6A（`2026-10-17-p6a-context-compiler-design.md`）§5.2 的
   `scopes`／`anchor: Option<String>`／`memory_query` 三个字段、P6A §5.3 的 `RetrievalQuery::new(...)?`（`limit` 取 `budget.max_memories`、
   `include_terminal` 写死 `false`）、P6A §5.6 的 `MemoryError` 透传，都与本设计 §3.5／§6.1 一致。
   **三条接法（已满足）**：
   (i) **`anchor` 的类型**：`Option<String>`；`Some` 时 `scopes` 必须恰好是一个 anchored scope
   （`Project` / `Product` / `Task`），否则 `AnchorNotAllowed`；anchored scope 缺 anchor ⇒ `AnchorRequired`。
   `Personal`／`Runtime` 不能带 anchor。P6A §5.2 已按这一条写。
   (ii) **`max_memories = 0` 的语义**：P6A §5.3 不调检索、`memories[]` 为空；P6C 的 `limit = 0` 返回
   `Ok(vec![])`，**不是 `Err`**（§6.1／§6.2）。两边同向，`compile` 不因预算 0 失败。
   (iii) **`scopes` 为空**：P6A §5.3 不调检索；P6C 的 `RetrievalQuery::new` 对空 `scopes` 返回
   `EmptyScopeSet`——**空集合是构造错误，不是「查全部」**。P6A 不构造它，故这条不会触发。
   **跨块测试一条（两边按同一语义写）**：P6A 带 `[Project] + Some(anchor)` ⇒ 只得到该 anchor 的 claim；
   `[Project] + None` ⇒ `AnchorRequired`；`[Personal] + None, limit = 0` ⇒ 空 `memories[]`、无 `Err`；
   `[Personal, Project] + Some` ⇒ `AnchorNotAllowed`。这条测试同时钉住 §5.4 的读取侧与预算 0。
   **建议的接法**：`MemoryStore` 由装配方用 `&Tx` 构造；**P6C 不提供第二枚检索 trait**——若未来另立
   `&str` 适配，适配层留在 `continuum-context` 里（它依赖 `continuum-memory`，方向正确）；反过来让
   `continuum-memory` 实现 P6A 的 trait 会造 `continuum-memory → continuum-context` 的反向边（§3.2）。
   **一处层表差异（两边都已登记，本块不裁定）**：P6A §2.3 已同时登记 README `:26-31` 的「执行层 (3)」
   与《工程》§9.1 的 `资源层 (4) → 认知层 (6)` 两种读法，并明写「本块不裁定层表」；本设计按执行层 (3)
   论证（§3.2）。两边登记的是同一条差异，不是谁的单方面断言；收件人：协调者 ＋ P6C（P6A §11 第 22 条）。
   **其余接口（已确认）**：(i) P6A §7.2 的五项接口（`MemoryClaim`、`MemoryScope`、`MemoryError`、
   `MemoryRetrieval`、`RetrievalQuery`）里没有 `MemorySummary`，本设计据此判「不」；
   (ii) P6A 不按 `SourceKind` 决定标签（标签由 P6B 定，其 §5.4 只搬运 provenance）；
   (iii) `limit` 由 P6A 的 `budget.max_memories` 给（P6A §5.3），本块不设上限。
2. **P6B（信任标签）**：无共享类型（切分 §四第 6 条已判）。本块**不**把 `SourceKind` 当标签，也**不**给
   `MEMORY` 标签。**P6B 报来的一条语义缺口与本块的答复**：P6B 的设计（其 §2.4 第 15 行、§3.3 的
   「两处例外」第 2 条、§12 第 6 条）指出「一段由网页合成的记忆 claim 在入口处只会得到
   `MEMORY`，它的外部来源不可见」。本块的答复分两半：(i) **能给的**——`MemoryClaim.sources[]` 给出 source id，
   `load_sources` 按 id 给出 `SourceKind` 与 `external_ref`。**据实：P6A 当前接口到不了这一步**——`compile`
   只收 `&dyn MemoryRetrieval`（无 `Tx`），`load_sources` 收 `&Tx`；若日后要实现来源摘要，需给 trait 加方法，
   或把 Tx 侧结果传入请求（收件人：P6A ＋ 协调者）；
   (ii) **不能给的**——`SourceKind` **不是**信任类别（`Files` 既可能是用户文件，也可能是下载的网页），
   把 `SourceKind` 映射成 §281 的标签需要一条 P6B 的规则，本块不造。**若 P6B 要 claim 级的一个「来源信任摘要」，
   它是第八字段或一个派生值，不能塞进 §283 的七字段**；本块建议将来放在装配处派生（需先补 source 读取面），
   不写进 claim。收件人：P6B ＋ P6A ＋ 协调者 ＋ 规范维护者。
3. **P6D（Protected Core / Runtime Learning）**：本块的 `RUNTIME` scope 是「模型表现、工具行为、调度经验」的
   存储面（《总纲》§6.2 的五级作用域表），也是 Runtime Learning 的**输入**之一。**本块不施加 §321**：
   一条 `RUNTIME` claim 可以被 `apply_synthesis` 写，也可以被 `correct` 撤；它没有任何「不可修改」的保护。
   若 Runtime Learning 的产物要写进 `RUNTIME` scope，P6D 的边界必须自己保证「学习不能改 §321 的五类语义」——
   本块没有这道门。收件人：P6D 的设计者 + 协调者。
4. **P7（长期循环 / Background Supervisor）**：§8.6 的三个函数是全部接缝；调度、阈值、恢复不在本块。
   收件人：协调者（P7 搁下期间，`SynthesisTrigger` 的四个时机只有类型、没有触发器）。
5. **P5（验证与证据）**：迁移号段对账——P5a 已占 `130`（实际迁移），P5c 声明 `150`（零表），P5d／P5e／P5f 不占档；
   本块取 `190`。**P5 与 P6 的档不撞**：`130`／`150`／`190` 三点已实测。收件人：协调者。
6. **装配方（`continuum-runtime`）**：**五处**——根 `Cargo.toml` 的 members、`continuum-runtime/Cargo.toml` 的
   path 依赖、`dependency_direction.rs` 的新 crate 行、同一文件的 `continuum-runtime` 行、`runtime_migrations()`
   的一行（§3.3 的五行）。**切分 §1.3 的共写表只有三行**（members、`dependency_direction` 的新行、`main.rs`
   的注册）；本设计补的是 runtime 的 manifest 与 runtime 的 `ALLOWED` 行——请协调者确认切分表补行，
   否则「谁的表谁注册」在 runtime 的 manifest 上没有归属。
7. **`continuum-persist`（P0）**：本块只用 `Tx` / `Migration` / `PersistError` / `Value`，不改它。
   若未来要给 `memory_claim` 加「至少一条来源」的数据库级约束，需要 P0 的触发器支持或一次迁移；本块今天用写入口 + 诊断查询。

---

# 13. 切分 §四 六条横切约束的逐条落实

1. **不新增反向边，且可观察**：本块的 `ALLOWED` 行是 `("continuum-memory", &["continuum-persist"])`；
   除 `continuum-runtime` 那一行按切分 §四第 1 条的例外加 `continuum-memory` 一项外，
   既有行的允许集合**不因本块增枚**（本块不碰任何既有 crate 的类型）。并集的复核具名给 P6D（切分原文）。
   照片：S1、S2（含 runtime 的 manifest 与 `ALLOWED` 行）；门是用例
   `every_crate_depends_only_on_its_allowed_set`（本树 `:478`）。
2. **迁移号段**：本块写死 P6 档 `190`–`199`，自己取 `190`；`191`–`199` 留给同轮其余块现场顺取（§3.3、§9.1）。
   落地时重跑 `grep -rn "Migration::new(" crates/` 核对空号。收件人：协调者（与 P5 那轮的对账见 §12.2 第 5 条）。
3. **同一件事两个词汇表**：本块**不使用**「provenance」指来源（`Artifact.provenance` 是制品血缘，
   `crates/continuum-artifact/src/artifact.rs:178`）；**不使用**「trust」指来源种类（`NodeTrust` 是计算节点信任域，
   `crates/continuum-node/src/node.rs:60`）；**不使用**裸 `Scope`（P5a 的 `EvidenceScope` 是证据范围，本块的
   `MemoryScope` 是记忆作用域）。`SourceKind` 与 §281 的七个标签**不是**同一件事：前者是来源的分类，后者是上下文片段的
   信任标签；两者不互相赋值（§6.3）。照片：S3／S6 的命名检查 + 源码面 `grep`。
4. **工具结果有两个消费者**：本块**不是**工具结果的消费者。§279 的 Compiler（P6A）与 §89 的 Evidence 转换（P5a）
   之间的 wrapper 问题与本块无关；本块也不为「工具结果 → Memory Source」造 wrapper（§12.1 第 13 条）。
   若 P6A 或 P5a 需要这条桥，先解决它们的「同一枚原料两个 wrapper」问题，再把结果交给装配方，
   由装配方调本块的 `insert_source`。
5. **依赖边只登记实际用到的**：只有 `continuum-persist` 一条（§3.2）。`ALLOWED` 与实际 `Cargo.toml` 精确一致；
   新增 workspace 成员会先让门变红，这是刻意的（切分原文）。照片 S2。
6. **两对块共享未冻结接口**：P6A ↔ P6C 的那一对由本设计**先冻**——§3.5 的三枚类型、访问器表、
   `insert_source` 签名、`MemoryRetrieval` trait 与 `MemoryStore` 就是那一页；P6A 已对着它写
   （P6A §5.2／§5.3／§5.6），不另造。
   P6B ↔ P6C 无共享类型（本块不引用标签，标签不引用 claim），可全并行；P6D 与本块无共享类型。
   若 P6A 在写作中发现缺口，走 §12.2 第 1 条，不在本文件外就地加。

---

# 14. 收口

- **本轮的交付是设计，不是代码**（切分 §七）。实现计划另起。
- **本块对 §6.4 的贡献只有判据 3**（§7.4）；判据 1、2、4 分别属 P6A、P6B、P6D。
- **本块对 §9.2 的贡献是两枚入度为零源头之一**（Memory Source Graph）：模块方向见 §3.4，规范级的
  「P0 算不算上游」申报见 §12.1 第 9 条。
- **本块最需要复审去推翻的三处**：(i) §4.4 的「无条件失效」是否过严；(ii) §5.4 的侧表是否算绕开「七字段逐字」；
  (iii) §8.5 的「applier-only」是否把 §285 的作业缩得太小。三处都给了代价与翻案条件，没有一处写成「无代价」。
  **锚点形状的收口另算一处**：§5.4／§6.1 的 `scope + anchor: Option<String>` 是否够 P6A／P7 用；不够时改的是
  这一对字段的入口，不是再加第二种锚点类型。
