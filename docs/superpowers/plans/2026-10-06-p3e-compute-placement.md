# P3 子项目 E（Compute Node 注册与节点放置）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新建 `continuum-node` crate，把《工程》§4.1 组件表的两行落成代码——
**Compute Node 注册**（§287–§291：`ComputeNode` 与进程内 `NodeRegistry`）与
**节点放置**（§291 §243 §93：硬闸门 `place` ＋ 排序接口 `PlacementPolicy`），
并把完成判据（`docs/02-工程.md:262`「隐私等级为 LOCAL_ONLY 的 Artifact 不会被放置到云端节点」）
拍成一对可跑的用例。

**Architecture:** **新建一个 crate**（`crates/continuum-node`），不落进 `continuum-runtime`。
四段：`node.rs`（§287 的 `ComputeNode` 与三个取值类型）／`registry.rs`（§4 的注册表，**入度为零**）／
`placement.rs`（请求面、硬闸门、策略 trait、`place`）／`error.rs`（`PlacementError`、`RulesError`）。
**`place` 是同步纯函数**：不接 `Tx`、不做 I/O、不读时钟——故它的错误枚举里没有 `Persist` 一类变体。
两段式照规范语气劈：**硬闸门（§243／§94 的 MUST）写死、策略不可放宽**；
**排序（§291／§93 的 SHOULD）只冻结接口，一个数值都不写**。

**Tech Stack:** Rust 1.95.0 / edition 2024；**唯一一条内部依赖边是 `continuum-artifact`**
（用 `Artifact` 与 `PrivacyClass` 两个**既有类型**）；`thiserror`（错误类型派生）；dev：`trybuild`。

**设计依据（唯一事实来源）：** `docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`，
**定稿 `d30167b`（1148 行）**；**并已按 `3a7ddad` 的五条订正逐条复核**——那五条正是本计划
第一轮报出的设计缺陷（见 `## 遗留` 第一节，**已闭**），本计划与订正后的设计**逐条一致**。
本计划写于 HEAD `3a7ddad`。
**实施前须复核**：若设计在其后还有修订，先核对本计划引用的**节号与裁决**
（本计划的**正文**（Task 1–7）引设计时**按内容引**——用例名、节题——故行号位移不影响它，
受影响的只会是节号本身；**例外是 `## 遗留` 第一节**：那里引的设计行号是**原报告的一部分**，
按 `d30167b` 的读数留档，见该节的说明）。**协调者裁定**：
`docs/superpowers/2026-10-06-p3e-decisions.md`——**注意它在 `docs/superpowers/` 下，不在 `specs/` 下**
（设计 §1.3 的订正块记过同一处误判）。本计划不改设计、不改规范、不改 C／D／F 的任何接口，
**对 `continuum-model-registry` 零依赖**。

---

## 跨计划前置（依赖谁已交付什么、把什么交给谁）

### 依赖状态（2026-10-06 实读工作树）

| 依赖 | 状态 | 用在哪 |
|---|---|---|
| `continuum_artifact::{Artifact, ArtifactId, ArtifactType, PrivacyClass, ContentHash}` | **已交付** | `PlacementRequest::artifacts` 的字段类型与 `PrivacyClass` 的键 |
| `PrivacyClass::ALL`（五枚）、`as_str`（`:119`）、`parse`（`:137`） | **已交付**，`ALL` 是覆盖率判据的唯一来源 | `PlacementRules::try_new` |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` | **已交付**，`every_crate_depends_only_on_its_allowed_set` 逐对双向断言 | 新 crate 的登记 |
| `trybuild`（workspace 依赖，D／P3A／B／F 在用） | **已交付** | `ComputeNode` 的字段私有性样例 |

**本计划没有任何 task 被外部交付阻塞**：E 的唯一依赖 `continuum-artifact` 早已落地。
**执行序上 E 与 B／C／D／F／G 无编译期关系**（它不新建任何共享面，也不消费它们的任何类型）。

### 本节把什么交给谁

- **交给协调者**：`plan-p3e` 报回的设计缺陷逐条（见 `## 遗留` 第一节），协调者按
  `docs/superpowers/2026-10-06-p3e-decisions.md` 第二节末的「计划作者报回的设计缺陷（待补）」栏登记。
- **交给规范维护者（本项目无此角色）**：设计 §10 的 17 条遗留，**本计划一条都不接、一条都不发明**，
  收件人与阻塞范围照原文抄在 `## 遗留` 第二节。
- **交给驱动侧的节点执行装配点**：`ExecutionProfile.compute_node`（`Option<String>`）是放置结果的落点，
  而**本计划的唯一生产调用方是测试**——**本计划不声称放置已接上生产路径**（设计 §10 第 15 条、
  协调者义务第 6 条）。`ComputeNodeId` 是 newtype，**塞不进那个裸 `Option<String>`**，
  本计划**不擅自收紧它**（那是 G 的设计 §7 已收件的那一件事）。
- **交给复审者**：`PlacementRules` 里 `LocalOnly` 那一格的形状取舍（设计 §10 第 17 条）；
  以及本计划自定的四处形状（`## 遗留` 第三节）。

---

## Global Constraints

**这一节是逐条可核的判据，不是背景说明。** 前六条是协调者裁定
（`docs/superpowers/2026-10-06-p3e-decisions.md` 第二节「必须写进计划 Global Constraints 的义务」）
的**逐条落点**，原文口径不改。

1. **「不读 `policy.rules()`」那条变异体必须在实施期真跑一次**（裁定义务 1；设计 §8 第 8 条）。
   判据三条：(a) 变异体的内容是**闸门只算 `spec_floor(level)`、完全不读 `policy.rules()`**；
   (b) **唯一红点**须落在 Task 5 的 `a_caller_supplied_rule_can_tighten_the_gate` 这一条的断言上；
   (c) 红点若落在别处或不止一处，**据实报告并停下**，不得改断言迁就实现。
   今天这一条**只有逐字推演的论证、没有照片**（设计无实现体）——**不跑则 `PlacementRules` 的取值
   可以完全不被读而整套用例全绿**。**实测结果写进交付报告**（哪条断言红、日志路径、变异前后 `sha256`）。
2. **`ALLOWED` 条目与 workspace `members` 必须同批加**（裁定义务 2）。
   `crates/continuum-runtime/tests/dependency_direction.rs` 的
   `every_crate_depends_only_on_its_allowed_set` 里有**双向**断言：
   「`workspace` 成员 ↔ `ALLOWED`」两条各管一个方向，
   且逐对断言是 `assert_eq!(appeared, allowed.contains(&other))`——
   **`ALLOWED` 列了而 `Cargo.toml` 没声明的边同样是红**。
   > **行号只是当时读数，会漂**（2026-10-07 收）：本条原写「`:249-262`」与「`:276-281`」，
   > 那是 **2026-10-06 定稿时**的读数；**2026-10-07 在 p3e 的 worktree 上实读**（HEAD 含 E 的 Task 1）
   > 三个锚点分别是 **`:267-272`（「未列入 `ALLOWED`」那条断言，信息在 `:270`）**、
   > **`:273-278`（「表已过期」那条，信息在 `:276`）**、**`:291-296`（逐对双向的 `assert_eq!`）**。
   > **±15 的完整分解（2026-10-07，E 的 Task 1 评审查出）**：本行第一版只说「差 6 行」，
   > **那只解释了 15 行总差里的 6 行**——完整分解是三棵树的实测（`git show <c>:<path>` 逐棵数）：
   >
   > | 树 | 「未列入 `ALLOWED`」那处 | 「表已过期」那处 | 逐对 `assert_eq!` |
   > |---|---|---|---|
   > | `0e0f253`（295 行）——**本计划与设计引的原值取自这棵树** | `253-256` | `259-262` | `276-281` |
   > | `ce335dd`（304 行）——**+9** | `262-265` | `268-271` | `285` |
   > | `c871c08`（310 行）——**再 +6** | `267-272` | `273-278` | `291-296` |
   >
   > **即：+15 ＝ +9 ＋ +6**。**那 9 行早于 E 的 Task 1**：`0e0f253` → `ce335dd` 中间是
   > `a0a8a07`（+5）与 `0b83d81`（+4），二者都不是本子项目的提交；**6 行**才是 E 的 Task 1
   > （`c871c08` 给这个文件加的正是 6 行）。**本计划与设计两处引的原值恰好对得上 `0e0f253` 那棵树**
   > ——说明定稿时的读数**取自那棵树，而不是取自定稿提交本身**（定稿提交与它之间已经隔了 9 行）。
   >
   > **判据（本条是全仓反复处理的那一类）**：**引用一律按名字／内容（函数名、断言原文），
   > 行号只作当时读数并标注日期**——凡引行号，写的时候就要预着它会漂；
   > **一般化后的那一条写在 Global Constraints 第 21 条。**
   > **本计划其余引 `dependency_direction.rs` 行号的地方（Task 1 的两处、Task 7 一处、
   > 「关于本计划的代码块」一处）同批按此订正。**
   故 Task 1 的三处
   （`crates/continuum-node/Cargo.toml` 的 `continuum-artifact` 声明／`Cargo.toml` 的 `members` 行／
   `ALLOWED` 条目）**必须落在同一个 commit 里**，且**两侧都要各拍一次红**（Task 1 Step 1、Step 2）。
3. **`continuum-model-registry/src/router.rs` 的行号一律「以当时工作树为准」，不写死**（裁定义务 3）。
   该文件近日有未提交改动（`M`，整体下移约 15 行），**行号随时会动**。
   **本计划在 2026-10-06 对当时工作树重取过一次**（`RankedExecutionCandidates` `:394`、`selected` `:403`、
   `ExecutionCandidate` `:345`、`RoutingReason` `:276`、`RankingPolicy` `:424`、`rank` `:512`，
   逐条属实）——**但这六个数字是那一次的读数，不是本计划给出的常量**。
   **本计划正文一处都不据它们下判断**：E 对 `continuum-model-registry` **零依赖**，
   凡需要说明那条边的地方都按内容引（§4.3 的第三条入边、裁定的形状甲），**不按行号引**。
   将来若取设计 §6 的读法 (b)（放置要读候选排序），**接手的人须在**那时**的工作树上再取一次**。
4. **多写者单文件：只登记自己那几条，不设集中登记 task**（裁定义务 4）。
   本轮与 E 共写 `ALLOWED`／workspace `Cargo.toml` 的其它子项目**各自登记自己的条目**；
   Task 1 **只加 `continuum-node` 这一条**，**不改动任何别的条目**（Task 7 Step 2 逐行核）。
5. **「注册表不引 `artifact`」的守卫是源码文本断言，匹配字面拼法**（裁定义务 5）。
   它是**下界**（别名、全限定路径、`include!` 逃逸），
   **封闭的那一层是 `ALLOWED` 的逐对断言**——**不得因这条守卫而省掉 `ALLOWED` 的任何一步**。
6. **不得声称放置已接上生产路径**（裁定义务 6）。
   本计划的唯一生产调用方是**测试**；`ExecutionProfile.compute_node` 是裸 `Option<String>`，
   `ComputeNodeId` 今天塞不进去。凡交付报告、提交信息、注释里提到调用方，**只写测试**。

**以下七条是设计本身的硬约束，逐条都必须对着实现核。**

7. **硬闸门 fail-closed，且比任何调用方的规则都更窄——调用方只能收紧，绝不能放宽。**
   机制只有一条：`spec_floor` **不读策略**，且 `strictest` 的 `TrustedPersonalOnly` 一侧**吸收一切**
   （设计 §5.3）。判据：`LocalOnly` 的制品**一概**上不了「非 `Personal` 或未 `TrustedPersonal`」的节点，
   **包括调用方把五档全写成 `AnyNode` 的情形**——这是绝对措辞，**它的照片是 Task 5 的
   `the_gate_cannot_be_widened_by_any_caller_table`（逐档遍历）**，不是靠读代码。
8. **`PlacementRules` 必填、无默认，且必须逐档覆盖 `PrivacyClass::ALL` 的每一档。**
   `try_new` 对**少一档、重复一档**都返回具名 `Err`（`MissingLevel` / `DuplicateLevel`）；
   `rule_for` **不返回 `Option`**（构造期已保证覆盖），查询落空返回 `TrustedPersonalOnly`
   （不可达，但若可达它是 fail-closed 的那一侧）。**覆盖率判据的来源是 `PrivacyClass::ALL`
   （`artifact.rs:100`），不是手抄的五档清单**——手抄清单在规范加档时不会失败。
   **`LocalOnly` 那一格是装饰性的**（`spec_floor(LocalOnly)` 恒为 `TrustedPersonalOnly`，
   它在 `strictest` 里永远被吞掉），但 `try_new` **仍强制它存在**：
   第一格是给 `ALL` 的完整性用的，不是给判据用的（设计 §5.4 的 I2 段）。**实现必须照此写，
   不得"优化"掉这一格。**
9. **「未知／未归类」的隐私等级不得到达云端节点。** 三条通道各自可指认（设计 §5.4）：
   (a) `spec_floor` 的 `match` **穷尽且无通配臂**（加档时编译不过——**一条编译期性质，
   没有运行期照片，不得为拍它去改 `PrivacyClass`**）；
   (b) 表必须覆盖五档，缺档即构造期 `Err`；
   (c) **E 从不经字符串读隐私等级**——进来的就是 `PrivacyClass` 值
   （唯一解码点 `PrivacyClass::parse`，落库侧把它翻成 `PersistError`）。
   **实现里不得出现任何 `parse`／字符串比较／`unwrap_or(默认档)` 的路径。**
10. **新 crate 的 `ALLOWED` 条目必须逐字是设计 §7.3 给的那一条**：
    `("continuum-node", &["continuum-artifact"])`，**且与它的 `Cargo.toml` 同批改**（见第 2 条）。
    **不登记 `continuum-model-registry`**：《工程》§4.3 的 `节点放置 ← Router 输出` 那条边
    **已记为阻断**（今天无落点，收下从不读的形参已按裁定删去）——**这不是漏登记**；
    **不登记 `continuum-core`**（零引用）、**不登记 `continuum-persist` / `continuum-events`**
    （本设计不写库）、**不登记 `continuum-graph`**（对 ADFIR 图零引用）、
    **`continuum-runtime` 侧一条边都不加**（E 在 runtime 里没有调用点）。
    这几条**要写进 `ALLOWED` 条目旁的注释**，否则会被后来者当成漏登记（设计 §7.3 的注释即是原文）。
11. **本设计不落库、不取任何迁移号。** `crates/continuum-runtime/src/main.rs` 的
    `runtime_migrations()` **一字不改**（`git diff` 该文件无输出是 Task 7 的判据之一）。
    号段裁决（A 50、B 60、C 70、D 80、E 90，D 的设计 §11 第 14 条）**本计划不据此占号**；
    将来若要落库，取号前须现场核对当时该档未占用（本项目删过一次「预留号」）。
12. **不发明阈值、权重、算法。** §291 的十一项因子里**九项连量纲都没有**（设计 §5.6），
    可用的只有 `privacy`（§243 给了判据）与 `trust`（二值域是本设计取的）。
    故本计划交付的排序侧只有：**接口**（`PlacementPolicy`，可换一份策略重跑同一组用例）
    ＋ **一条具名的、可替换的基线**（`BaselinePlacementPolicy`，`compare` 对任何一对返回
    `Ordering::Equal`，次序完全由 `ComputeNodeId` 兜底档决定）。
    **任何一处数值化的权重、评分公式、成本估算都不得出现在代码里**——
    缺的是**输入**，不是数值。该缺口记在 `## 遗留`。
13. **`place` 是同步纯函数，且由签名保证**：不接 `Tx`、不做 I/O、不读时钟。
    故 `PlacementError` **只有两枚**（`NoPlaceableNode` / `DuplicateNode { id }`），
    **不收** `Persist` 一类变体，**不收**「未归类的隐私等级」（在 `try_new` 就被拒，到不了 `place`），
    **不收**「无候选模型」（本设计的请求面里没有候选模型）。**三处「看起来该收但没有产生方」的
    逐条判据见设计 §5.8，实现时照它写，不得凭直觉补一枚。**
14. **失败路径的用例要断言是哪一枚 `Err`**，不只「返回了 `Err`」；
    `DuplicateNode` 还要断言**是哪一枚 id**（夹具里放两个不同的 id，否则断言恒真）。
15. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**。
    **枚举式绝对断言须逐项有照片**——「五档逐档」、「四种 `class` × `trust` 组合」、
    「`PrivacyClass::ALL` 逐档」的**每条臂各要一条照片，不抽代表**；
    **断言的作用域要与事实同宽**（「整张表的唯一 X」≠「某个字段的唯一 X」）。
16. **变异纪律**（本仓付过代价，照抄 G 的计划）：
    每条变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、**每轮用独立日志路径**，
    报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径」。
    **变异的三种失效形态都要防**：(a) **锚点不唯一** → 变异没落到实现体却报 GREEN；
    (b) **等价变异体**——判据是「这两版在哪个入参上会给出不同结果」，举不出即是等价，
    处理是**换真变异体而非补用例**（本计划已预先标出一处等价变异体，见 `## 遗留` 第四节）；
    (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
    cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
    **每一处「预期谁红」都标了档位**（取反 / 放宽 / 收紧 / 移除）。
17. **测试可见性（本计划逐条核过）**：`tests/` 是**独立的 crate**，只看得见 `pub` 项。
    本计划在 `tests/` 里用到的每一样东西都是 `pub`：`ComputeNode::new` 与六条访问器、
    `ComputeNodeId::new`、`NodeRegistry::{new, register, nodes}`、`PlacementRules::try_new` / `rule_for`、
    `PlacementRequest` 的两个 `pub` 字段、`place`、`PlacementPolicy`（trait，测试自己实现它）、
    `BaselinePlacementPolicy::new`、`PlacementError` / `RulesError` / `NodeRegistryError` 的变体。
    **三处私有性**（`ComputeNode` 的字段、`PlacementRules` 的字段、`NodeRegistry` 的字段）由
    **唯一 `pub` 构造入口**保证，测试**不碰私有字段**。
    `Artifact` 的十个字段**全是 `pub`**（`artifact.rs:170-181`，字段在 `:171-180`），故夹具可用**全字段字面量**构造它；
    `ArtifactId::new` 与 `ContentHash` 都有 `pub` 入口。
18. **每个 task 结束时整个 workspace 必须编译过、0 warning、`cargo test --workspace --no-fail-fast` 全绿。**
    工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
    **代码注释、错误信息、测试断言信息用中文**；标识符用英文。
19. **不要用 `git add -A`，不要 `git commit --amend`**，只 `git add <显式路径>`。
    **新增依赖会让 `Cargo.lock` 变化，须一并按显式路径提交锁文件**。
    本计划有**三次**清单改动、**三次**都要提交 `Cargo.lock`：`Cargo.toml`（workspace，Task 1）、
    `crates/continuum-node/Cargo.toml` 的 `trybuild`（Task 2）、同一个文件的 `serde_json`（Task 5）。
    **`Cargo.lock` 也是「临时改动」的受害者，这一点要写进还原护栏**（2026-10-07，E 的 Task 1 报出）：
    **凡临时改动会牵动 cargo 解析结果的地方**——workspace `members`、任何 `Cargo.toml` 的依赖声明
    ——**`trap` 要连 `Cargo.lock` 一起还原**。判据：把 `members` 行临时拿掉，cargo 会重解析并把
    `continuum-node` 的条目从锁文件里抹掉，而纪律 16 的模板只 `cp` 回那**一个源码文件**；
    **只还原源码时，「还原后逐字节相同」这句只对部分文件成立**。
    核对的判据是 `sha256sum Cargo.lock` 在动手前后一致（Task 1 Step 2 的注记里给了同一条）。
    **不修改用户目录的权限位**；不在仓库中写入任何凭据。
20. **范围与设计一致，不多做不少做。** 明确不做的，逐条列出免得被读成漏项：
    不建 `get(&ComputeNodeId)` / `deregister` / `len` / `is_empty`（零消费方）；
    不建节点状态刷新或心跳（§291 的 `current load` 因此连「会变」都不成立）；
    不建 `JobCapsule`（§290 的五个字段分属四处，装配点不在 E）；
    不建 Authority Host / Device Join / Trust Graph（不在 E 的组件表里，故信任的产生方不存在）；
    不建 Temporary Node 的租约（§287 的 `ComputeNode` 没有租约字段）；
    不新增 `AuditKind` 变体（那等于扩 §313 的必录清单，是规范的事）；
    不落库、不取迁移号；不做分布式执行本身（§343 明写 v0.1 可暂不实现）。
21. **引用 ＝ 文件 ＋ 定位，是断言的一部分**（2026-10-07，E 的 Task 1 评审查出后立）。
    **行数是全篇最廉价、最容易被查的那一句，恰恰是它错了最伤**：读者一查所指文件就对不上，
    **于是要么不再信其余数字，要么干脆不查**。
    故本计划的一切引用一律给**文件 ＋ 定位**（函数名／用例名／断言原文），
    **行号只作当时读数**，并**标注它是哪一天、在哪一棵树上取的**；
    **凡引行号，写的时候就要预着它会漂**（Global Constraints 第 2 条的那张三棵树分解表即是实例：
    同一批锚点在 `0e0f253`／`ce335dd`／`c871c08` 三棵树上分别是三组数，**+15 ＝ +9 ＋ +6**）。
    **它与「只读完整日志、不用 `tail`」是同族，不是新判据**：那条说的是「**引用一段输出时，
    截断过的输出不能作它的证据**」，这条说的是「**引用一个位置时，位置与所指必须对得上**」
    ——**都是「引用与所指必须对得上」**。
    **一处实测的来历（供后来者对账，2026-10-07）**：复核本计划那三处行号时手上是两份全量日志
    （**原话照留**：本行初稿写「一份 1442 行、一份 1502 行」），**两份在全部实质数字上一致**
    （`113` / `678` / `0` / `0`，逐项相同），**故那一次处置只需改引用、不需要重跑**。
    **按本仓规矩，这两份日志在 `.tmp/` 下、不进库，不得作为任何结论的唯一来历**——
    写在这里的只是「当时是怎么判的」，判据本身是前面那几句话。
    **订正（2026-10-07，按下面第 1 条自己重量过一次）**：**那个「1442」是错的，真实是 1441**——
    `wc -l .tmp/final-full.log` = **1441**（`68253` 字节，`sha256 8d6297f1…`）；
    另一份是 `wc -l .tmp/full-test.log` = **1502**（`70953` 字节，`sha256 ad7add0f…`）。
    **错因见第 1 条**：它是本条判据**自己的反例**。

    **四件事并进本条**（2026-10-07，E 的 Task 1 评审自己撞上本条的反例后补）：
    1. **数「一个文件有多少行」只许用 `wc -l`**（或 `grep -c ''`、`awk 'END{print NR}'`），
       **不许用任何会引入 ±1 的中间表示**：`len(open(f).read().split('\n'))`、`len(f.readlines())`、
       编辑器显示的行号都可能与 `wc -l` 差 1。
       **本条的第一次使用就栽在这里**：评审者用 `split('\n')` 得 1442，而
       **同一批输出里 `wc -l` 已经写着 1441**——**文件以换行结尾时 `split` 会多出一个尾部空串，
       恒等于「真行数 ＋ 1」**（我实测确认：`split('\n')` 对上面两份日志分别给出 **1442** 与 **1503**，
       两份都以换行结尾）。
    2. **写下任何行数之前，`wc -l` 实测，并把「量的是哪个路径」一起写**；**附 `sha256`／字节数更好**
       ——**别人据此可自证「是同一份文件还是另一份」**。本次两边正是靠
       **`sha256` 前缀 ＋ 字节数 ＋ `mtime` 三项对上**，才判定「**同一份文件、我读错了**」，
       而不是「**量了不同的东西**」——**这两个结论的处置完全不同**（前者改数，后者查路径）。
    3. **拒收「转抄来的数」是对的，且这一条也管本计划自己**：本轮协调者把评审者的 1442
       **转述**给实现者让它落笔，**实现者拒收、自己五法实测 1441、并留退路等对方自证**——
       **它是对的**。判据：**「转抄的数」与「我自己核过的数」在文档里长得一样，
       但只有后者能追责**。故**本计划里凡引一个实测值（或一句关于世界状态的断言），
       要么自己核过（文中写「实测」或给出该值的来历），要么标明「据 X、我未复核」**。
       **本条已有两个实例，一并记在此处**：①**一个数**——协调者转述评审者的「1442 行」，
       实现者拒收并实测为 1441（见上）；②**一句话**——2026-10-07 协调者**从本计划 Task 2
       那一行逐字抄了半句「设计 §9 的标签与它的正文相抵」进一份派单**，而设计早已在 `3a7ddad`
       订正、相抵不再成立，**是转抄让一句过期的话又漂到了下游一份派单里**
       （该处原文与来历已留在 Task 2 那一条里）。
       **两个实例的形态不同（一个是数、一个是关于世界状态的断言），但判据是同一条**：
       **转抄者对它没有追责能力**；**且第②例说明「过期」有两种**——数会漂，**句子也会过期**
       （句子过期时锚点照样命中，只能当断言重核一遍）。
       **第三面（2026-10-07 补）：量的时候对、报的时候已经旧了。**
       **「我自己量的」还分「量对了」与「量的时候对、报的时候已经旧了」**——
       **判据：数字与落笔之间若还发生过任何一次写，就必须重取。**
       **这一面从报告里最看不出来，因为那对数互相自洽**：本次的自报是「+49 / −10，现 1582 行」，
       **1543 ＋ 49 − 10 ＝ 1582 完全自洽**——**自洽性只证明算术没错，证明不了时机没错**。
       **实例出处 `bcefaf9`（可查）**：真值是 **+51 / −10、1584 行**
       （`git diff --numstat 8955017 bcefaf9 -- <本文件>` ＝ 51 / 10；
       `git show 8955017:<本文件> | wc -l` ＝ 1543、`git show bcefaf9:<本文件> | wc -l` ＝ 1584）。
       **方法没错**：`--stat` 与 `--numstat` 在**同一个提交**上给的是同一个数（已对过）；
       错在**量完之后又改了一处**（把「三条断言」订正为「两条断言」并补一行）
       **然后直接报了量之前的那对数**。**那 2 行的逐笔分解不复原**——中间态从未提交、已不可达，
       **再往下说就是把一个听起来合理的机制写成来历，那正是本条要挡的毛病**。
       **前两面与第三面的对照**：**第一面＝转抄**（对该数没有追责能力）；
       **第二面＝方法有偏**（如 `split('\n')` 的 ±1）；**第三面＝时机陈旧**（方法与数都对，
       只是报的时候已经不是那个数了）。
       **与「HEAD 一动就通知下游」互指**：那一条是同一件事在**跨 agent** 上的形态
       （作者还在改、评审者按旧版判），**本条是它在同一个人的「量 → 报」之间的形态**——
       **两者都是「读数与它被引用时的世界不是同一个」**。
       **本计划的自查（2026-10-07）**：文中每个 `file:line`
       （`artifact.rs` 的 `:5`／`:87-105`／`:100`／`:112`／`:170-181`／`:177-178`、
       `docs/02-工程.md:262`、`main.rs:83-93`、`continuum-operator/src/registry.rs:11`／`:27`、
       以及 `dependency_direction.rs` 的全部锚点）**都是我这一手实测的**；
       `## 遗留` 第一节里那组设计行号（`:796`／`:978`／`:983`／`:520`／`:586`／`:705`／`:692`）
       是**原报告的读数、已标注「不随设计修订重取」**——**它们是来历，不是本计划对今天世界的断言**。
    4. **与第 2 条那张 ±15 分解表是同一件事的两面**：那是一张**三棵树**的表，每棵树的读数都必须
       写明**是哪一棵树**（提交号）——**「量的是哪个路径」在版本维度上的形态就是「量的是哪棵树」**，
       两条互指。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；临时目录取仓库内的
`.tmp/`（`TMPDIR="$PWD/.tmp"`），**变异日志是证据，实现者保留 `.tmp/`，由协调者在复审结束后清理**。
**变异窗口与验证窗口互斥**：实现者报告完成之前，协调者不得在该工作区里跑 cargo。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
故本计划只给**类型签名、枚举取值与关键判定**，**不给整段可粘贴实现**。

> **报错码的纪律（本计划一律照此写，2026-10-06 收）**：
> **「预期报错码」必须来自实跑**——`.stderr` 不会替你报错，**码写错了它照样让用例绿**；
> **跑不了就把话说成「预期是某一类」**（「名字未解析」/「缺字段」/「字段私有」那一类），
> **不写具体码**。本计划下文出现的每一个 `E0xxx` 都是**预期**，不是判据；
> **唯一的判据是 `trybuild` 实跑产出并与源码一并入库的那份 `.stderr`**
> （Task 2 的 Step 4 会打开它读一遍：确认红的原因是**那一条**，而不是拼错路径之类的别的什么）。
> 凡本计划某个码没有对应的 `.stderr` 机制，就只是提示，**不得被引作证据**。

**本计划特有的三处「签名即判据」，照抄前须对源**：

- `PrivacyClass` 的五个变体名与 `ALL` 的次序（`crates/continuum-artifact/src/artifact.rs:87-105`）：
  `Public / Personal / Private / Secret / LocalOnly`，`ALL` 同序五枚。**变体名写错即编译不过，别凭记忆写。**
- `Artifact` 恰十个 `pub` 字段（`artifact.rs:170-181`，字段在 `:171-180`）：`id / artifact_type / content_hash / size /
  producer_node / input_artifacts / metadata / provenance / privacy_class / version`。
  **夹具要写全字段字面量**（少一个字段会编不过，**预期是「缺字段」那一类**——常见码 `E0063`，
  **以实跑为准**），`metadata` / `provenance` 用
  `serde_json::json!({})`。`Artifact` **不派生 `Default`**。
  **`continuum-artifact` 的 re-export 里没有 `Value`**（`src/lib.rs` 只导出
  `Artifact` / `ArtifactId` / `ArtifactType` / `PrivacyClass` / `BlobStore` / `ContentHash` /
  `ArtifactError` / `ArtifactStore` / `load_artifact` / `save_artifact` / 迁移函数；
  `Value` 只是 `artifact.rs:5` 的一条 `use`）——**故写这十个字段的夹具必须自己依赖 `serde_json`**，
  见下条的清单要点与 Task 5 的 Step 1。
- `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 是
  `&[(&str, &[&str])]`，**逐对双向断言**（`assert_eq!(appeared, allowed.contains(&other))`；
  **行号是当时读数**：定稿时 `:276-281`，**2026-10-07 实读 `:291-296`**）
  ——**声明的边与表必须精确相等**。

**新增 crate 的清单要点**（`crates/continuum-node/Cargo.toml`）：
`version` / `edition` / `rust-version` 三项用 `workspace = true`（照 `crates/continuum-artifact/Cargo.toml`）；
`[dependencies]` 只有 `continuum-artifact = { path = "../continuum-artifact" }` 与
`thiserror = { workspace = true }`；`[dev-dependencies]` 是 **`trybuild`（Task 2 登记）**
与 **`serde_json`（Task 5 登记）** 两条，**各由第一个用它的 task 登记**。
**`serde_json` 这条为什么必需**：`tests/placement.rs` 要构造 `Artifact` 的全字段字面量，
而 `metadata` / `provenance` 是 `serde_json::Value`，**`continuum-artifact` 不 re-export 它**
（见上一条）——**这是一条 dev 边，只出现在测试目标里**，库本体一个 `Value` 都不用。
它**不进 `ALLOWED`**（那张表只逐对断言 workspace 成员之间的边）。
**不加 `serde` / `tempfile` / `continuum-persist`**（本 crate 不派生序列化、不建临时库、不落库）。

---

# 文件结构

```
crates/continuum-node/                 ← 新建 crate（§7.1）
  Cargo.toml                           依赖只有 continuum-artifact + thiserror；
                                       dev：trybuild（Task 2 登记）＋ serde_json（Task 5 登记）
  src/lib.rs                           导出面与 crate 文档（每个 task 各登记自己那几行）
  src/node.rs                          ComputeNodeId、NodeClass、NodeTrust、ComputeNode
  src/registry.rs                      NodeRegistry、NodeRegistryError
  src/placement.rs                     PlacementRequest、TransferRule、spec_floor、strictest、
                                       PlacementRules、PlacementPolicy、BaselinePlacementPolicy、place
  src/error.rs                         PlacementError、RulesError
  tests/node.rs                        §3 的取值类型与访问器
  tests/registry.rs                    §4 的注册表 + 「注册表不引 artifact」的源码文本守卫
  tests/rules.rs                       §5.4 (b) 的覆盖判据
  tests/placement.rs                   §5.2–§5.9 的闸门、失败路径、确定性、策略可替换
  tests/type_level.rs                  trybuild 驱动（§9 的「写不出字段字面量」）
  tests/compile_fail/compute_node_fields_are_private.rs   与同名 .stderr（实跑产出）

既有 crate 里本计划要改的文件（**只有两个，都是多写者单文件，各登记自己那一行**）：
  Cargo.toml（workspace）                                 members 加 "crates/continuum-node"
  crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 加 continuum-node 一条
  Cargo.lock                                              新包与两次 dev 依赖（Task 1、2、5 各提交一次）
```

**本计划不碰的文件**：`crates/continuum-artifact/**`（只读它的公开面）、
`crates/continuum-model-registry/**`（**零依赖**）、`crates/continuum-core/**`、
`crates/continuum-graph/**`、`crates/continuum-runtime/src/main.rs`（**不建表、不装配**）、
`docs/02-工程.md`、C／D／F／G 的设计与计划。

---

### Task 1: 建 crate 骨架并登记（workspace `members` ＋ `ALLOWED`）

**Files:**
- Create: `crates/continuum-node/Cargo.toml`
- Create: `crates/continuum-node/src/lib.rs`（本 task 只有 crate 文档与空的 `pub mod` 声明）
- Modify: `Cargo.toml`（workspace 的 `members`，**只加一行**）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**只在 `continuum-model-registry` 条目之后加一条**）
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: 无（本 task 不写任何实现）
- Produces: workspace 成员 `continuum-node`；`ALLOWED` 里的 `("continuum-node", &["continuum-artifact"])`

**为什么 `continuum-artifact` 在**本 task**就登记**（而不是等它的第一个使用点）：`ALLOWED` 的断言是
**逐对双向**的（`assert_eq!(appeared, allowed.contains(&other))`；**行号是当时读数**：
定稿时 `:276-281`，2026-10-07 实读 `:291-296`），
故「表里列了、`Cargo.toml` 没声明」与「声明了、表里没列」**同样是红**。
三者（crate 的 `Cargo.toml`／workspace 的 `members`／`ALLOWED`）分在三个提交里，
中间态必然红。**「边由用它的那个 task 登记」这条规矩在这里的落点是「谁引入这个 crate，
谁一次登记齐它的唯一一条边」**——`continuum-node` 的边由设计 §7.2 定死，只有一条。

- [ ] **Step 1: 先拍「workspace 成员未列入 `ALLOWED`」这一侧的红**

建 `crates/continuum-node/Cargo.toml`（按「关于本计划的代码块」的清单要点，**但本 task 的
`[dev-dependencies]` 是空的**：`trybuild` 由 Task 2 登记、`serde_json` 由 Task 5 登记——
「边由用它的那个 task 登记」，**不提前铺开**）与
`crates/continuum-node/src/lib.rs`（**只放 crate 文档注释，不放任何 `pub mod`**），
在 workspace `Cargo.toml` 的 `members` 里加 `"crates/continuum-node",`，
**先不动** `dependency_direction.rs`。跑：

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：`every_crate_depends_only_on_its_allowed_set` 红，信息为
「workspace 成员 continuum-node 未列入 ALLOWED，它的依赖方向不会被检查」。
**该断言的定位按内容**（函数名 ＋ 上面那句原文）；**行号只是当时读数**：定稿时写 `:253-256`，
**2026-10-07 实读为 `:267-272`**（`assert!` 在 `:268-271`，**信息在 `:270`**）。
**这就是这一侧的照片；先记下它，再进 Step 2。**

- [ ] **Step 2: 拍另一侧的红（`ALLOWED` 列了、`members` 里没有）**

把 `members` 里刚加的那一行**临时去掉**，改成在 `ALLOWED` 里加条目，再跑同一条命令。
预期：红，信息为「ALLOWED 列出的 continuum-node 不是 workspace 成员：表已过期」。
**定位同样按内容**；**行号只是当时读数**：定稿时 `:259-262`，**2026-10-07 实读为 `:273-278`**
（**信息在 `:276`**）。
**这一侧不是可有可无的对称**：两条断言各管一个方向，只钉一侧时另一侧的静默漏检不会被发现
（本项目「守卫须两侧都钉」的同一条）。**记下两次红的原文，然后还原到「两边都有」。**

> **一处实现者实测的副作用，必须知道（2026-10-07，E 的 Task 1 报出）**：
> **把 `members` 行临时去掉时，cargo 会重新解析 workspace，并把已写进 `Cargo.lock` 的
> `continuum-node` 条目一并抹掉**——而这个文件**不在 `trap` 的还原范围里**
> （纪律 16 的模板只 `cp` 回被变异的那**一个源码文件**）。
> **后果**：还原 `Cargo.toml` 之后，`Cargo.lock` 可能仍是「少了 `continuum-node`」的状态，
> 于是「还原后与变异前逐字节相同」这句话**只对部分文件成立**——**这一步的诚实做法是把
> `Cargo.lock` 也一起还原，并核对它与动手前相同**（它本来就在 Step 5 的提交路径里，不是新增文件）。
> **判据（一般化，写进 Global Constraints 第 19 条）**：**凡临时改动会牵动 cargo 解析结果的地方**
> （workspace `members`、任何 `Cargo.toml` 的依赖声明），**`trap` 要连 `Cargo.lock` 一起还原**。
> **一处事实要说清、免得被读成否定**：本计划 Step 4 那句「新增一个**空** crate 也会让
> `Cargo.lock` 变化」**是真的**——它的为真恰恰体现在这里（`members` 在位时，**一条普通的
> `cargo test` 就会把那一行写回去**，实现者已用定点实验确认）。**这一条是对那句话的补强，
> 不是否定它。**

- [ ] **Step 3: 加 `ALLOWED` 条目（逐字照设计 §7.3）**

在 `crates/continuum-runtime/tests/dependency_direction.rs` 的 `continuum-model-registry` 条目**之后**
（该表整体不是字母序；本条目跟在同族的 model-registry 之后，与已登记各条的相邻关系一致）
加**一条**，注释照抄设计 §7.3 给的那一段（它写明三处「不登记」各是为什么，缺了它后来者会读成漏登记）：

```rust
    // P3 子项目 E：计算节点与放置。设计 §7.2 的唯一一条边是 artifact（`Artifact` /
    // `PrivacyClass`，§243 的隐私输入）。**不登记 model-registry**：《工程》§4.3 的
    // 「节点放置 ← Router 输出」**已记为阻断**（今天无落点，收下从不读的形参按裁定删去，
    // 设计 §6；收件人见 §10 第 8 条）——**不是漏登记**。
    // **不登记 core**：本设计对它零引用。**不登记 persist / events**：本设计不写库（§4.2）。
    ("continuum-node", &["continuum-artifact"]),
```

**只加这一条**：本轮多份计划共写这个文件，按裁定义务 4，**不设集中登记 task、不改动别的条目**。

- [ ] **Step 4: 运行，确认绿**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：两条用例全绿、0 warning。**顺便实测记下**：新增一个**空** crate 也会让 `Cargo.lock` 变化
（多一个 `[[package]]`），Step 5 要提交它。
**这句话的因果，2026-10-07 由 E 的 Task 1 补齐**：变的不只是「多一行」——**`members` 的增删会让
cargo 重解析并重写这个文件**，故**把 `members` 行临时拿掉时，`Cargo.lock` 里那一条会被抹掉**；
`trap` 若不还原它，「还原后逐字节相同」就只对部分文件成立（详见 Step 2 的注记与
Global Constraints 第 19 条）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add Cargo.toml Cargo.lock crates/continuum-node/Cargo.toml crates/continuum-node/src/lib.rs \
        crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(node): 建 continuum-node crate 并登记 workspace 成员与依赖边"
```

---

### Task 2: `ComputeNode` 与它的三个取值类型（§3）

**Files:**
- Create: `crates/continuum-node/src/node.rs`
- Modify: `crates/continuum-node/src/lib.rs`（加 `pub mod node;` 与**四条** `pub use`：
  `ComputeNodeId` / `NodeClass` / `NodeTrust` / `ComputeNode`，与本节 Produces 一栏的四个名字对齐。）
- Create: `crates/continuum-node/tests/node.rs`
- Create: `crates/continuum-node/tests/type_level.rs`
- Create: `crates/continuum-node/tests/compile_fail/compute_node_fields_are_private.rs`
- Create: `crates/continuum-node/tests/compile_fail/compute_node_fields_are_private.stderr`（**实跑产出**）
- Modify: `crates/continuum-node/Cargo.toml`（`[dev-dependencies]` 加 `trybuild`）
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: 无（本 task 不碰 `continuum-artifact`）
- Produces: `continuum_node::{ComputeNodeId, NodeClass, NodeTrust, ComputeNode}`

**取值域的三条，逐条照设计写，一个都不扩**：`NodeClass` 两枚（§287 唯一给出取值域的字段）、
`NodeTrust` 两枚（**取值域是本设计定的**，来源是 §293／§342.7／§289 的四处引文）、
`capabilities` / `resources` / `availability` 三个 `Vec<String>`（**只搬运、不解释、不比较、不排序**）。

- [ ] **Step 1: 写用例**

`tests/node.rs`：

- `the_six_accessors_return_what_new_was_given`：**六项逐条断言，不抽代表**。
  夹具的三组 `Vec<String>` 取**两两不同**的值（`["cap"]` / `["res"]` / `["avail"]`）、
  `id` 取 `"n1"`、`class` 取 `NodeClass::Personal`、`trust` 取 `NodeTrust::TrustedPersonal`。
  **红的条件（档位：取反）**：把 `new` 里 `resources` 与 `availability` 两个赋值**互换**
  （两者都是 `Vec<String>`，**互换后照样编译**）→ 那两条断言红。
  **这一条夹具的两两不同是承重的**：三组同类型字段若取相等的值，互换**不产生可观察差别**
  （等价变异体），断言会假绿——这正是「变异须真落到实现体」那条纪律在本 task 的落法。
- `the_id_is_the_registry_key_and_is_ordered`：`ComputeNodeId` 可比较且**大小有意义**：
  `ComputeNodeId::new("a") < ComputeNodeId::new("b")`，且 `new("a") == new("a")`。
  **红的条件（档位：取反）**：把序反写（如 `b.0.cmp(&a.0)`）→ 红。
  **但那一处必须连 `PartialOrd` 一起反写**（2026-10-07，E 的 Task 2 实现者实测；**本行初稿说窄了**）：
  `<` 走的是 **`PartialOrd::lt`**，而 `#[derive(PartialOrd)]` 与 `#[derive(Ord)]` 是**两个各自生成的
  独立派生**——**只反写 `Ord`、保留 `derive(PartialOrd)` 时，本用例**两条**断言
  （`assert!(… < …)` 与 `assert_eq!(…)`，实测 `tests/node.rs:88-89`）走的分别是
  `PartialOrd::lt` 与 `PartialEq::eq`，**都不经 `Ord`**，于是「编得过、全绿」**
  （**等价变异体**：那一版与本版在任何入参上都不给出不同结果，
  本 task 里没有一处调 `Ord::cmp`）。**故「删掉 `Ord`」或「只反写 `Ord`」都不是真变异体**；
  真变异体是**同时反写两个派生**（或把两者都删掉、改成按长度比较）。
  **来历照留**：本行初稿写「把 `Ord` 派生成按**长度**或反转」——**只点名了 `Ord`**，
  照它做会得到一次**假绿**（这正是「变异须真落到实现体」那条纪律里的等价变异体一档）。
  **它的消费方是 Task 6 的兜底档**（`a.id().cmp(b.id())`，那处走 `Ord::cmp`），
  故这条不是提前铺开的 API；**也正因本 task 还没有 `Ord::cmp` 的消费方，只反写 `Ord` 才无迹可寻**。
- `the_two_node_classes_are_the_two_the_spec_names`（**纯编译期照片**）：`tests/node.rs` 里写一个
  `fn class_label(c: NodeClass) -> &'static str`，函数体是一个覆盖
  `NodeClass::Personal` 与 `NodeClass::Temporary` **两臂、无通配臂**的 `match`。
  **判据是「它编得过」**：给 `NodeClass` 加第三枚时这个 `match` 编译不过，
  **预期是「非穷尽 match」那一类**（常见码 `E0004`，**以实跑为准**）。
  **它的红形态是「编译失败」，不是「断言失败」**——照实写进用例注释
  （同 G 的 `assert_send` 那条的处置）。
  **不写运行期断言**：「只有两枚」是**枚举定义的封闭性**（一条编译期性质），
  任何运行期断言（例如 `Personal != Temporary`）在 Rust 里**恒真**——**那是假照片**，
  它的红只会来自改动测试自己写的那个 `match`，与 crate 无关
  （同 §5.4 通道 (a) 的处置：编译期性质，没有运行期照片）。
  **这一条对 `NodeTrust`（`TrustedPersonal` / `OutsidePersonalTrustDomain`）各写一遍，不合并**：
  两个枚举能各自漂移，一个 `match` 覆盖不了另一个。

`tests/type_level.rs`（照 `crates/continuum-model-registry/tests/type_level.rs` 的形状，
文件头写清「不可表达性只有编译失败样例能钉」）：

- 一份 `tests/compile_fail/compute_node_fields_are_private.rs`：以**全字段结构体字面量**构造
  `continuum_node::ComputeNode { … }`，**判据是它编译不过**（字段私有，预期 `E0451`）。
  **错误码只是预期值，以 `trybuild` 实跑的 `.stderr` 为准**——本项目为「凭记忆写错误码」付过代价
  （D 的计划 §关于代码块那段有记录）。
  **这一份钉的措辞是「写不出字段字面量」，不是「构造不出来」**：`ComputeNode::new` 是 `pub` 的，
  「crate 外构造不出节点」这句话为假。
  **订正（2026-10-07）**：本句初稿在末尾写着「（设计 §9 那一行的**标签**与它的**正文**相抵，
  见 `## 遗留`）」——**那半句是过期引用**：设计已在 `3a7ddad` 把那个标签收到正文的口径
  （见 `## 遗留` 第一节第 5 条），**「相抵」今天不成立了**（旧话照留在此，来历见下）。
  **本条要钉的判据一字未变**（措辞取「字段私有」而非「不可外部构造」），变的只是
  「设计今天还相抵」这个**已经过期的断言**。
  **这半句的来历（据实记，它与纪律 21 第 3 条同族）**：2026-10-07 协调者**从本计划这一行逐字
  抄了这半句进一份派单**，被实现者核出它早已修好——**一次转抄把一句过期的话又送到下游**。
  「转抄的数／话在文档里长得一样，但只有自己核过的能追责」，这是那条判据的**第二个实例**
  （第一个是 `1442` 那个行数）。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node
```

预期：**「名字未解析」这一类**（`continuum_node::ComputeNode` 与它的类型尚未存在）。
**本 task 实得只有一个码：`E0432`（unresolved import）**——2026-10-07 由 E 的 Task 2 实现者实测：
本 task 的测试里那四个名字**只出现在 `tests/node.rs` 的一条 `use` 上**（`place` 属 Task 5／6，
本 task 的测试里还没有裸函数名或路径限定的调用），故**没有「三种写法都会有」这回事**
（**本行初稿说「三种写法都会有」，那是说宽了**——它描述的是本计划**整体的**可能性，
不是**这个 task** 的实况；旧话照留在此）。
**类别说法仍然成立**：码取决于**怎么引用**（`use` 进来的路径／路径限定调用／裸函数名在 rustc 下
不是同一个码），故本行**只写「这一类」、不把码写成断言**；**本 task 的实测值是 `E0432`，
以实跑为准**。
（本行更早的初稿把 `E0433` / `E0425` 两个码并排列出，2026-10-06 按「报错码必须来自实跑」的纪律
收成这一类说法。）

- [ ] **Step 3: 实现**

```rust
/// §287 的 `id`。**不新建第二个 `NodeId`**：`continuum_graph::NodeId` 已存在且指 ADFIR 节点（§3.6）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComputeNodeId(String);

impl ComputeNodeId {
    pub fn new(id: impl Into<String>) -> Self;
}

/// §287 的 "class"。两枚，逐字照录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeClass { Personal, Temporary }

/// 节点在信任域中的位置。**取值域是本设计定的**（见设计 §3.3 的四处引文）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTrust { TrustedPersonal, OutsidePersonalTrustDomain }

/// §287 的 `ComputeNode`。**字段私有，唯一构造入口是 `new`**（设计 §3.5）。
pub struct ComputeNode { /* id, class, trust, capabilities, resources, availability */ }

impl ComputeNode {
    pub fn new(
        id: ComputeNodeId,
        class: NodeClass,
        trust: NodeTrust,
        capabilities: Vec<String>,
        resources: Vec<String>,
        availability: Vec<String>,
    ) -> Self;
    pub fn id(&self) -> &ComputeNodeId;
    pub fn class(&self) -> NodeClass;
    pub fn trust(&self) -> NodeTrust;
    pub fn capabilities(&self) -> &[String];
    pub fn resources(&self) -> &[String];
    pub fn availability(&self) -> &[String];
}
```

**三条写进实现（不许省）**：

- **`NodeTrust` 的两个值各带一行文档注释写明来源**（`TrustedPersonal` ← §293 的终态
  `Trusted Personal Node`；`OutsidePersonalTrustDomain` ← §342.7 的「不得**自动**进入」），
  并写明**二值域是本设计取的、不是规范的形状**：若规范给出更细的分级，`NodeTrust` 要重取，
  且**放行方向会变**（加值时必须逐值重新归类，设计 §3.3 限度 1、§10 第 3 条）。
- **`ComputeNode` 只派生 `Debug`**：判据是**具体的一处**——`place` 返回 `Result<&ComputeNode, _>`，
  而 `Result::unwrap_err` 的签名带 `T: Debug`，故测试里最自然的失败路径写法
  （`let err = place(…).unwrap_err();`）要求它。（这条不写出来，实现者会在 Task 5 被迫回来补派生。）
  **`Clone` 与 `PartialEq` 都不得派生**：今天零消费方，而**派生 `PartialEq` 会把「两枚节点相等」
  变成一条本层没有判据的命题**（放置只比 `id`，§5.9）。`ComputeNodeId` 与 `NodeClass` / `NodeTrust`
  该有的派生在各自的接口处已列明。
- **三个 `Vec<String>` 字段的文档注释写「只搬运，不解释」**，并写明代价：§291 的十一项因子里
  有九项挂在它们与 `latency`／`money` 上，**本层一项都算不出来**（§3.4、§5.6、§10 第 4 条）。

- [ ] **Step 4: 运行，确认 `tests/node.rs` 绿、trybuild 产出 `.stderr`**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node
```

**首跑 trybuild 会因缺 `.stderr` 而失败**（它打的是「wip」并写出实际输出）。
按 D 的做法：`TRYBUILD=overwrite` 跑一次生成
`tests/compile_fail/compute_node_fields_are_private.stderr`，**再跑一次确认绿**，
**并打开那份 `.stderr` 读一遍**——确认红的原因是**字段私有**（**预期是「字段私有」那一类**，
常见码 `E0451`；**这一处的判据是那份实跑产出的 `.stderr` 本身**，不是这个码），
不是拼错路径或别的什么（「因为拼错函数名而编译失败」也会让用例变绿）。
**`.stderr` 的内容整份入库**。

- [ ] **Step 5: 实测「守卫会红」（内容不提交）**

把 `src/node.rs` 里 `id` 那个字段**临时改成 `pub`**，跑
`TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node --test type_level`。
预期：**红**。
**但红的机制不是「样例编过了」**（**本行初稿就是这么写的，2026-10-07 由 E 的 Task 2 实现者实测订正**）：
只把 `id` 改成 `pub` 时**其余五个字段仍是私有的**，那份样例如**仍编不过**，
**红来自 trybuild 的 `.stderr` mismatch**（实际错误与入库的那份 `.stderr` 不一致），
**不是来自「预期失败、实际编译通过」**。**守卫本身有效**（改一个字段它就能读出来），
**错的只是这一句对机制的描述**——而机制描述错了，会让人在读日志时找错判据
（「编过了」与「`.stderr` 不符」在 trybuild 的输出里是两段不同的话）。
**记下日志路径，再还原**（`trap` + 前后 `sha256`，纪律 16）。
**这一步是「守卫恒绿」与「守卫有效」的区分点**，不许省。

- [ ] **Step 6: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-node/Cargo.toml Cargo.lock crates/continuum-node/src/lib.rs \
        crates/continuum-node/src/node.rs crates/continuum-node/tests/node.rs \
        crates/continuum-node/tests/type_level.rs crates/continuum-node/tests/compile_fail
git commit -m "feat(node): §287 的 ComputeNode 与三个取值类型"
```

**提交后实测记一笔**：`trybuild` 是**外部 crate**，不进 `ALLOWED`（那张表只断言 workspace 成员之间的边），
但它在 `cargo tree --edges all --depth 1` 的输出里——故这一跑同时是「外部 dev 依赖不进表」的实测
（不据口径断言）。**若不绿，停下报告**。

---

### Task 3: Compute Node 注册（§4）与「注册表不引 artifact」的守卫

**Files:**
- Create: `crates/continuum-node/src/registry.rs`
- Modify: `crates/continuum-node/src/lib.rs`
- Create: `crates/continuum-node/tests/registry.rs`

**Interfaces:**
- Consumes: Task 2 的 `ComputeNode` / `ComputeNodeId`
- Produces: `continuum_node::{NodeRegistry, NodeRegistryError}`

- [ ] **Step 1: 写用例**

`tests/registry.rs`：

- `registering_three_nodes_keeps_the_registration_order`：登记三枚（id `"c"` / `"a"` / `"b"`，
  **故意不是升序**），断言 `nodes()` 的 id 序列**逐项**等于登记序。
  **红的条件（档位：取反）**：把 `register` 的 `push` 改成 `insert(0, …)`，
  **或在 `register` 的末尾加一次 `sort_by(id)`**（把登记序改写成 id 序）→ 红。
  **登记序不是升序**这一点是承重的：夹具若按升序登记，两种实现不可区分。
  **订正（2026-10-07，E 的 Task 3 实现者报出；旧话照留）**：
  - **用例名**：本行初稿叫 `registering_**two**_nodes_keeps_the_registration_order`，
    **「二」与同条正文的「三枚」相抵**（派单要求逐字照用，实现者未改名、实际登记三枚）。
    已改成 **`…_three_nodes_…`**。**这是本仓那条「用例名也是一种断言——声称的必须与断言实际
    覆盖的相等」（`6aeb450` 立）的又一个实例**：名字是最持久的一句，断言删改而名字没跟上时，
    假话就留在名字里。
  - **变异体形态**：本行初稿还写「（或让 `nodes()` **返回排序后的副本**）→ 红」——**那个形态写不出来**：
    `nodes()` 的签名是 `&self -> &[ComputeNode]`，**副本活不过返回**（要么改成返回 `Vec`，
    那就动了签名）。**可表达的等价变异体是在 `register` 里排序**（上面已写成那一形），
    实现者用它实测，结果与 `insert(0, …)` 一致。**故凡给变异体，必须给一个「在本 API 形态下
    写得出来」的形态**——写不出来的变异体等于没有变异体，而它看上去与写得出来的那个一模一样。
- `registering_the_same_id_twice_is_a_named_error`：登记 `a`、`b`，再登记 `a` →
  `Err(NodeRegistryError::Duplicate { id })`，**并断言 `id == ComputeNodeId::new("a")`**
  （两个不同的 id 在场，否则「是哪一枚」的断言恒真——纪律 14）。
  **红的条件（档位：移除）**：删掉判重那一步 → 返回 `Ok`，红。
- `a_failed_registration_does_not_overwrite`：登记 `a`（`trust = TrustedPersonal`）、`b`；
  再登记一枚 id 同为 `a` 但 `trust = OutsidePersonalTrustDomain` 的节点 → `Err`，
  **并断言 `nodes()` 仍是两枚、次序不变、且 id 为 `a` 的那一枚的 `trust()` 仍是
  `TrustedPersonal`**（**第一次那枚**）。
  **红的条件（档位：放宽）**：把判重改成**静默覆盖**（替换那一枚，不返回 `Err`）→ 本条红。
  **订正（2026-10-07，E 的 Task 3 实现者实测；旧话照留）**：本行初稿写「→ **三处断言同时红**」，
  **那句不成立**。实现者的 M3b 取的是**「先覆盖、再返回 `Err`」**这一形，实测红点分布是：
  **只有本用例红，且只红在 `trust()` 那一条断言上**——`expect_err`／`id`（是哪一枚）／枚数／次序
  **全绿**（因为那一形仍返回 `Err`，也仍不改变枚数与次序）。
  **这个分布很值钱，两条判据由此成立**：
  ①**「不静默覆盖」这条判据的照片是唯一的**——**只有本用例能拍到它**；
  ②**`registering_the_same_id_twice_is_a_named_error` 单独不够**——它只钉「返回了具名 `Err`」，
  对「覆盖之后再返回 `Err`」这种实现**全绿**（B 的实现阶段查出的 §八.20／§八.23 两处
  「静默后者胜」正是这个形状）。
  **另记一句（读这处的人会问）**：本用例的三条断言**不是同一条变异体的三个红点**，
  而是**一层覆盖另一层**——外层（`Err`）内侧还有「有没有改状态」那一层，
  **只有最里面那一条（`trust()`）能区分「拒绝」与「先改后拒」**。
  实现者已按纪律 11 订正了自己文件里的同一句并把错因留在原地；
  **本计划与当时的派单里也写着那一句，故此处一并订正**。
  **这一条是「不静默覆盖」这条判据唯一的照片**：只断「第二次返回 `Err`」时，
  一个「先覆盖、再返回 `Err`」的实现会绿（B 的实现阶段查出的 §八.20／§八.23 两处「静默后者胜」
  在本仓一律判为留白，设计 §4.1 判据 2）。
- `the_registry_module_names_no_artifact_type`（**源码文本守卫**，设计 §7.2 的 M4）：
  断言 `crates/continuum-node/src/registry.rs` 的全文里**不出现**三个名字的字面拼法：
  `continuum_artifact`、`Artifact`、`PrivacyClass`。**断言信息里列出命中的行号**
  （否则红的时候读不出是哪一处）；路径用
  `Path::new(env!("CARGO_MANIFEST_DIR")).join("src/registry.rs")`。
  **以下所有说明一律写在 `tests/registry.rs` 的文件头，一个字都不许写进 `src/registry.rs`**
  ——守卫匹配的是**那个文件的全文**，**把「本文件不出现 `Artifact`」这句话写进去，
  守卫会命中自己的注释而变红**（本仓的一个已知坑；2026-10-06 第一轮评审查出本行初稿没说清落点）。
  **文件头要写清它的证明力边界**（裁定义务 5）：匹配的是**字面拼法**，别名、全限定路径、
  `include!` 都逃逸——**它是下界，不是封闭判定**；**封闭的那一层是 `ALLOWED` 的逐对断言**
  （Task 1；本 crate 整体只允许 `continuum-artifact` 一条边）。
  同时写清**为什么 `Artifact` 这个词不能出现**：§4 的注册表按《工程》§4.3 与 §9.2 是**入度为零**的
  组件，它的零依赖不是靠一句话，是靠这条断言 + `ALLOWED`。
  **红的条件（档位：取反）**：在 `registry.rs` 里加一行 `use continuum_artifact::Artifact;` 即红。
  **实测这一步要真做一次**（临时加、跑、确认红且报出行号、再删，`trap` + 前后 `sha256`，纪律 16）。
- `the_guard_sees_the_module`（**守卫自身的正控制，先钉机制再看结论**）：
  读同一份文件，断言**非空**且能找到 `pub struct NodeRegistry` 的拼法。
  **没有这一条，一个「什么都没读到」的守卫会永远绿**——这正是「守卫须两侧都钉」里缺的那一侧。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node --test registry
```

- [ ] **Step 3: 实现**

```rust
/// 已注册的计算节点。登记顺序即遍历顺序。
pub struct NodeRegistry { /* nodes: Vec<ComputeNode> */ }

impl NodeRegistry {
    pub fn new() -> Self;
    /// 同一 `ComputeNodeId` 二次登记返回 `Err`，**不静默覆盖**。
    pub fn register(&mut self, node: ComputeNode) -> Result<(), NodeRegistryError>;
    pub fn nodes(&self) -> &[ComputeNode];
}

pub enum NodeRegistryError {
    Duplicate { id: ComputeNodeId },
}
```

**三条写进实现**：
**错误类型的派生（三处错误枚举同此，Task 4／Task 5 各写一遍不合并）**：
`#[derive(Debug, thiserror::Error)]`，每个变体配 `#[error("…")]` 的中文错误信息
（与 D 的 `error.rs` 同形）。**`Debug` 是必需的、不是装饰**：`place` 返回
`Result<&ComputeNode, PlacementError>`，测试里 `unwrap()` / `unwrap_err()` 的签名带
`T: Debug` / `E: Debug`；**不派生 `PartialEq`**——用例一律用 `matches!` 或 `match` 断言是哪一枚
（`PartialEq` 会把「两枚错误相等」变成一条本层没有判据的命题）。
形状沿用本仓已有的两个注册表（`ProviderRegistry` 与
`continuum-operator` 的 `registry.rs:11` 的 `Duplicate { id, version }`）——**沿用的是形状
（具名变体、不静默），不是字段数**（本设计的键只有 `ComputeNodeId` 一个，故变体只有一个字段）；
**不建** `get` / `deregister` / `len` / `is_empty`（**零消费方**，C 当年为 `model_providers()`
留下的教训）；**不落库**（设计 §4.2 的三条判据），故**本 crate 不取任何迁移号**——
把「进程重启后节点集合为空，`place` 只会返回 `NoPlaceableNode`」这条代价写进 `NodeRegistry`
的文档注释（设计 §4.2 末段）。
`NodeRegistry::new` **不派生 `Default`**：零消费方（若 clippy 提示，据实报告，不擅自加）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-node/src/registry.rs crates/continuum-node/src/lib.rs \
        crates/continuum-node/tests/registry.rs
git commit -m "feat(node): 进程内 Compute Node 注册表与模块面守卫"
```

---

### Task 4: `PlacementRules`——表必填、无默认、逐档覆盖（§5.4 (b)）

**Files:**
- Create: `crates/continuum-node/src/placement.rs`（本 task 只放 `TransferRule` 与 `PlacementRules`）
- Create: `crates/continuum-node/src/error.rs`（本 task 只放 `RulesError`）
- Modify: `crates/continuum-node/src/lib.rs`
- Create: `crates/continuum-node/tests/rules.rs`

**Interfaces:**
- Consumes: **既有的** `continuum_artifact::PrivacyClass`（含 `ALL`）
- Produces: `continuum_node::{TransferRule, PlacementRules, RulesError}`

**本 task 是 `continuum-artifact` 这条边的第一个使用点**（Task 1 已登记声明）。

- [ ] **Step 1: 写用例**

`tests/rules.rs`：

- `a_table_covering_every_level_is_accepted`：表由 **`PrivacyClass::ALL` 逐档生成**
  （`ALL.iter().map(|&l| (l, 该档的规则))`）→ `Ok`；再**逐档**断言
  `rule_for(level)` 返回的就是**给那一档的值**（**五条断言，不抽代表**）。
  夹具的取值**两枚都要用上**（例如 `Public` 与 `Private` 给 `TrustedPersonalOnly`，其余三档给
  `AnyNode`）——**理由**：`TransferRule` 只有两枚，若五档写同一个值，
  「`rule_for` 把两档写反」这类变异**不可观察**（等价变异体）。
  **红的条件（档位：取反）**：把 `rule_for` 的查表改成返回一个**常量** → **五条里红的那几条
  等于「与该常量不同的档数」**：常量取 `AnyNode` 时红 **2** 条（那两档 `TrustedPersonalOnly`）、
  取 `TrustedPersonalOnly` 时红 **3** 条——**总之不会五条全红，最少红 2 条**
  （本行初稿写「至少四条红」是**报多了**：夹具是 2 档对 3 档，任何单一常量最多对上 3 档
  ——2026-10-06 第一轮评审查出，已按实改）。
  **这不削弱「枚举断言须逐项有照片」**：五条各钉一档，常量变异体只能对上其中一侧，
  故它**必然**至少在两档上红——**这正是逐档写五条的作用**（换成「只断一档」就抓不到了）。
  **一处射程要写明（不许写成「写反即红」）**：`TransferRule` 只有两枚而档有五枚，
  故「把两档的值互换」**只在被换的两档取值不同时**才可观察——
  互换两个**同值**档（例如两条都是 `AnyNode`）是**等价变异体**，那不是用例不够，
  是两版在**任何入参上都不会给出不同结果**（纪律 16 的 (b)）。
  故本用例的判据写成：**任何改变「某档的返回值」的变异，都能被它这一档的断言抓到**；
  同名断言**逐档各一条**正是为了这一点。
- `a_table_missing_any_level_is_rejected`：**五档各一条**——对 `PrivacyClass::ALL` 的每一档 `L`，
  构造「去掉 `L` 那条」的表 → `Err(RulesError::MissingLevel { level })`，**并断言 `level == L`**。
  **红的条件（档位：移除）**：删掉覆盖率检查那一整段 → 五条同时红。
  **为什么要逐档而不抽代表**：一条按档分支的手写检查能各自漂移
  （「枚举断言须逐项有照片」）；这一条的**唯一红点**是五条都红，故实现时必须确认**五条都跑了**。
- `a_table_repeating_a_level_is_rejected`：六条（五档齐 ＋ 某一档重复一次）→
  `Err(RulesError::DuplicateLevel { level })`，**并断言重复的是哪一档**。
  **红的条件（档位：移除）**：删掉重复检查 → 该表（六条、覆盖齐）会被判 `Ok` → 红。
  **注意它与上一条的分工**：重复一条**不违反覆盖率**，故只钉覆盖率是抓不到它的。
- `the_coverage_criterion_reads_the_five_levels_from_the_type`：断言
  `PrivacyClass::ALL.len() == 5` 且 `ALL` 里的五枚**两两不同**（用计数断言，不用「看起来没有」）。
  **它的射程照实写进用例注释**：这一条钉的是**输入侧的事实**（`ALL` 是五枚且无重复），
  **它拍不到「实现用的是 `ALL` 还是手抄的同一份五档清单」**——那两版在今天**是等价变异体**
  （加第六档才能区分），见 `## 遗留` 第四节。故**不写一条声称「来源是 `ALL`」的运行期断言**。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node --test rules
```

- [ ] **Step 3: 实现**

```rust
/// 一档隐私等级允许被放置在什么样的节点上。
/// **取值域里没有「`LocalOnly` 可以上任何节点」这一项**——见 `spec_floor`（Task 5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferRule {
    /// 只允许放在 `class == Personal` **且** `trust == TrustedPersonal` 的节点上。
    TrustedPersonalOnly,
    /// 可以放在任何已登记的节点上。
    AnyNode,
}

/// §94 的「privacy × trust class」表。**值由调用方给，本设计一个都不填。**
pub struct PlacementRules { /* entries: Vec<(PrivacyClass, TransferRule)>，字段私有 */ }

impl PlacementRules {
    /// 必须覆盖 `PrivacyClass::ALL` 的每一档，**多一条、少一条、重复一条都是 `Err`**。
    pub fn try_new(entries: Vec<(PrivacyClass, TransferRule)>) -> Result<Self, RulesError>;
    /// 构造期已保证覆盖，故这里不含 `Option`。查询落空时的返回值是 `TrustedPersonalOnly`
    /// ——**不可达**（构造期已排除），但若真到达，它是 fail-closed 的那一侧。
    pub fn rule_for(&self, level: PrivacyClass) -> TransferRule;
}

pub enum RulesError {
    MissingLevel { level: PrivacyClass },
    DuplicateLevel { level: PrivacyClass },
}
```

**四条写进实现**：

- **`RulesError` 的派生**（与 Task 3 的 `NodeRegistryError` 同口径，**各写一遍不合并**）：
  `#[derive(Debug, thiserror::Error)]` ＋ 每个变体配中文 `#[error("…")]`。
  **`Debug` 是必需的**：`PlacementRules::try_new` 返回 `Result<Self, RulesError>`，
  而测试里 `unwrap_err()` 的签名带 `E: Debug`。**不派生 `PartialEq`**——用例一律 `matches!`／
  `if let` 断言是哪一枚、并**断言 `level` 是哪一档**。
- **覆盖率判据的来源逐字是 `PrivacyClass::ALL`**（`artifact.rs:100`），
  **不是**本设计手写的一张五档清单：一张手抄清单在规范加档时**不会失败**，而 `ALL` 会
  （设计 §5.4）——**即使今天两者不可区分，实现的取法也照此写**，并在注释里写明这一点。
- **`LocalOnly` 那一格是装饰性的，但 `try_new` 仍强制它存在**（设计 §5.4 的 I2 段，纪律 8）：
  `spec_floor(LocalOnly)` 恒为 `TrustedPersonalOnly`，它在 `strictest` 里永远被吞掉；
  第一格是给 `ALL` 的完整性用的。**注释里写明「它是接口上的坑，不是安全问题」**：
  一个以为「把 `LocalOnly` 填成 `AnyNode` 就放开了」的调用方会发现放不开
  （它的照片是 Task 5 的 `the_gate_cannot_be_widened_by_any_caller_table`）。
- **不得给 `PlacementRules` 加 `Default`**（纪律 8：必填、无默认）——一个默认表就是 fail-open 的入口。
- **`PlacementRules` 不派生任何东西**：字段私有，今天零消费方（同 D 的 `BudgetView` 的处置）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-node/src/placement.rs crates/continuum-node/src/error.rs \
        crates/continuum-node/src/lib.rs crates/continuum-node/tests/rules.rs
git commit -m "feat(node): §94 的隐私×信任表与逐档覆盖判据"
```

---

### Task 5: 请求面与硬闸门（§5.2–§5.4、§5.8）

**Files:**
- Modify: `crates/continuum-node/src/placement.rs`（`PlacementRequest`、`spec_floor`、`strictest`、
  `PlacementPolicy`、`BaselinePlacementPolicy`、`place` 的步骤 1–3）
- Modify: `crates/continuum-node/src/error.rs`（加 `PlacementError`）
- Modify: `crates/continuum-node/src/lib.rs`
- Create: `crates/continuum-node/tests/placement.rs`
- Modify: `crates/continuum-node/Cargo.toml`（`[dev-dependencies]` 加 `serde_json`）
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: Task 2 的 `ComputeNode` / `ComputeNodeId` / `NodeClass` / `NodeTrust`；
  Task 4 的 `PlacementRules` / `TransferRule`；**既有的** `continuum_artifact::Artifact`；
  **外部 crate** `serde_json`（**只在测试目标里**，用来写 `metadata` / `provenance` 两个 `Value`）
- Produces: `continuum_node::{PlacementRequest, PlacementPolicy, BaselinePlacementPolicy, place, PlacementError}`

> **本 task 只落 `place` 的步骤 1–3（判重 → 过闸门 → 判空），第 5 步「取头」取的是过滤后的第一枚、
> 不排序**；步骤 4（排序）与「兜底档」在 **Task 6**。**故本 task 的每一个夹具都构造成
> 「过闸门后恰好剩一枚」或「Ordering 无关」**——凡是需要多枚可比节点的断言一律留到 Task 6。
> 这是刻意的拆分（各自的判据分开写清），**不是** `place` 的最终形态。

- [ ] **Step 1: 登记 dev 依赖，再写用例**

**先登记 `serde_json`**（本 task 是它的第一个使用点）：在 `crates/continuum-node/Cargo.toml` 的
`[dev-dependencies]` 加 `serde_json = { workspace = true }`。**判据**：`Artifact.metadata` /
`provenance` 的类型是 `serde_json::Value`，而 `continuum-artifact` **不 re-export 它**
（`artifact.rs:5` 只有一条 `use`）——**不登记则本 task 的夹具编不过**
（**预期是「名字未解析」这一类**；具体码以实跑为准）。
它是**外部 crate**，**不进 `ALLOWED`**（那张表只逐对断言 workspace 成员之间的边）；
**但它会让 `Cargo.lock` 变化**（本包的依赖列表变了），故 Step 6 要按显式路径提交它。
**实测记一笔**：这一跑照绿（`dependency_direction` 不受影响），不据口径断言。

`tests/placement.rs` 的夹具先立好（后面几条共用）：

- `fn artifact(level: PrivacyClass) -> Artifact`：**全字段字面量**（十个字段，见「关于本计划的代码块」），
  `metadata` / `provenance` 取 `serde_json::json!({})`，`producer_node: None`，`size: 0`。
  **`pub privacy_class: level` 是唯一随参数变动的字段**。
- `fn cloud(id: &str) -> ComputeNode`：`class = Temporary`、`trust = OutsidePersonalTrustDomain`；
  `fn desktop(id: &str) -> ComputeNode`：`class = Personal`、`trust = TrustedPersonal`。
  **两个工厂都收 `id` 参数，不写死**——判据是本节后面**三处用例对 id 的要求彼此不同**：
  §4.4 的正面用例要**云节点取小 id**（下面逐条写明理由），判重用例要**两枚同 id**（`desktop("a")`
  与 `cloud("a")`），四种组合那条要**四枚 id 各不相同**。写死一个 id 会让这三处里至少两处编不过
  或断言恒真。（2026-10-06 第一轮评审查出本行初稿只给了 `fn cloud()`，未参数化。）
  **§4.4 正面用例的 id 取法**：**云节点取两枚里较小的那个 id**（`cloud("a")` ＋ `desktop("b")`）
  ——这样即使 `place` 不排序，返回的也是云节点，**「闸门真的过滤了」与「它只是恰好排在前面」
  两件事才分得开**（该用例因此是承重的：过滤若失效，返回的必是云节点）。
- `fn rules_all_any() -> PlacementRules`：五档全 `AnyNode`。
- `fn rules_with(level: PrivacyClass, rule: TransferRule) -> PlacementRules`：其余四档 `AnyNode`。
- 一份测试侧的 `struct RecordingPolicy`：`rules()` 交出给定的表，`compare` 返回 `Ordering::Equal`
  （Task 6 会另加一份真比较的策略）。

用例（**逐条**）：

> **下文的简写**：写 `cloud` / `desktop` 的地方一律读作 `cloud("a")` / `desktop("b")`
> （除少数用例**明写了别的 id**——判重用例两枚同 id、四种组合四枚不同 id）。
> 「云节点取小 id」这条取法**只对 §4.4 正面用例是承重的**，其余用例不依赖它。

- `a_local_only_artifact_lands_only_on_a_trusted_personal_node`（**判据 §4.4 的正面**）：
  `nodes = [cloud("a"), desktop("b")]`（云节点在前且 id 更小，见上面的 id 取法）、`artifacts = [artifact(LocalOnly)]`、
  `rules_all_any()` → `Ok(n)`，**逐项断言 `n.class() == Personal` 与 `n.trust() == TrustedPersonal`**。
  **红的条件（档位：移除）**：把过闸门那一步整段删掉 → 返回云节点，两条断言同时红。
- `a_local_only_artifact_with_no_trusted_personal_node_is_unplaceable`（**判据 §4.4 的否定面**）：
  `nodes = [cloud]`、同一个制品 → `Err(PlacementError::NoPlaceableNode)`。
  **它与上一条是成对的**：单看任一条分不清「被闸门滤掉」与「本来就没节点」
  （`NoPlaceableNode` 不区分两者是**刻意的**，判据见 §5.8——§243／§291 没有要求放置输出一个理由）。
  **红的条件（档位：放宽）**：把闸门改成「滤掉之后若为空就退回未过滤的集合」→ 红。
- `a_public_artifact_may_land_on_a_cloud_node`（**放行侧**）：`Public` ＋
  `rules_with(Public, AnyNode)` ＋ `nodes = [cloud]` → **`Ok(n)`**，
  **并断言 `n.id()` 是那枚云节点**、`n.class() != Personal`。
  **这一格是必需的**：没有它，一个「把所有制品都当 `LocalOnly` 一律拒掉」的实现
  能通过本表其余每一行——那正是 fail-closed 的反面（闸门静默拒绝一切）。
  **红的条件（档位：收紧）**：让闸门对所有等级都返回 `Err`——或**只把 `spec_floor` 对那四档**
  也改成 `TrustedPersonalOnly`。
  **这一条会红；同一条变异体还会红下面这**两**处，逐条点名**（本行初稿写「这一条与**下一条**」，
  那是错的：下一条本来就期望 `Err`。第一轮评审改成了「具名清单」，**但那份清单又多列了 Task 6 的
  三条**——定点复核查出那三条的节点 `class` / `trust` 在本计划里从未写明，**预测因此悬空**；
  本行按下面的夹具事实重写）：
  - `a_public_artifact_may_land_on_a_cloud_node`（**本条自己**）：`nodes = [cloud]`，而云节点是
    `Temporary / OutsidePersonalTrustDomain` ⇒ 变异后它过不了 `TrustedPersonalOnly` ⇒ `Err` ⇒ 红。
  - `the_gate_cannot_be_widened_by_any_caller_table` 的 **(a) 臂的四个 `AnyNode` 档**
    ——那一臂也是 `nodes = [cloud]`（同一情形）；**(b) 臂不红**：它是 `[cloud, desktop]`，
    变异后 `desktop`（Personal / Trusted）仍在，故那四档仍得到 `Ok`。
  **不会红的，逐条给出机制**（免得有人去查不存在的问题）：
  - `a_caller_supplied_rule_can_tighten_the_gate`：它**本来就期望 `Err`**；
  - `four_class_trust_combinations_and_only_one_passes`：全走 `LocalOnly`，`spec_floor` 没动；
  - `an_empty_artifact_set_filters_nothing` 与 `an_empty_artifact_set_returns_the_tiebreak_winner`：
    制品集为空 ⇒ 闸门连一档都不看；
  - `duplicate_node_ids_are_a_named_error` 与 `an_empty_node_set_is_unplaceable`：期望的 `Err`
    与该变异体同解；
  - **Task 6 的三条**（`the_same_node_set_in_any_order_yields_the_same_node` /
    `the_id_breaks_ties_when_the_policy_says_equal` / `swapping_the_policy_changes_the_result`）：
    **它们的节点是 `desktop(...)`（`class = Personal`、`trust = TrustedPersonal`）**
    ——那正是变异后仍然放行的那一类，故三条**全绿**。
    **这一条与 Task 6 的夹具是钉死的一对**：Task 6 已把那三枚节点的 `class` / `trust` 写死为
    `desktop(...)`；**若将来有人把那个夹具改成 `cloud(...)`，这份预测要同时改**。
- `a_caller_supplied_rule_can_tighten_the_gate`（**放行侧的收紧面；裁定义务 1 的落点**）：
  `Secret` ＋ `rules_with(Secret, TrustedPersonalOnly)` ＋ `nodes = [cloud]` → **`Err(NoPlaceableNode)`**。
  **这一条钉的是「调用方的表真的被读」**：一个**完全不读 `policy.rules()`、只算
  `spec_floor(level)`** 的实现，在 `Secret` 上得 `AnyNode` ⇒ 返回 `Ok` ⇒ **在这一格自己的断言上变红**。
  **同时它是设计 §10 第 2 条那句「那四档的宽严完全取决于调用方」的照片**——没有它，
  那句话既无照片、又在「不读表」的实现下为假。
  **红的条件（档位：移除）**：把 `strictest` 的两个操作数之一换成常量 `AnyNode`
  （即丢掉调用方那一侧）→ 红。
  **本条与下一条是同一对的两个方向，缺一不可**：只钉下一条（全 `AnyNode`）时，
  「不读表」的实现**照样全绿**——它与「五档全 `AnyNode`」同解。
- `the_gate_cannot_be_widened_by_any_caller_table`（**闸门不可被策略放宽；逐档遍历，不抽代表**）：
  策略的表**五档全写 `AnyNode`**，对 `PrivacyClass::ALL` 的**每一档**各跑两次：
  - (a) `nodes = [cloud]`：`LocalOnly` → `Err(NoPlaceableNode)`；其余四档 → `Ok`，
    且返回的 id 是那枚云节点；
  - (b) `nodes = [cloud, desktop]`：`LocalOnly` → `Ok(n)`，
    **逐项断言 `n.class() == Personal` 与 `n.trust() == TrustedPersonal`**；
    其余四档 → `Ok`，且返回的 id **在两枚之内**（两枚都允许，故此处不断言具体哪一枚——
    「取头」的次序判据在 Task 6）。
  **绝对措辞与照片**：设计 §5.3 写「`LocalOnly` **一概**上不了非 personal、或未被信任的节点——
  包括调用方把五档全写成 `AnyNode` 的情形」。**这条绝对措辞的照片就是本用例的 (a) 与 (b) 的
  `LocalOnly` 两条臂**，不是靠读代码。
  **红的条件（档位：放宽）**：把 `strictest` 改成返回第一个操作数（即调用方的规则胜出）→
  本条的 `LocalOnly` 两臂红（而 `a_caller_supplied_rule…` 仍绿——两条各自独立）。
  **顺带钉住另一句**：`LocalOnly` 那一格填什么都一样（`spec_floor` 覆盖它）——
  把 `rules_all_any()` 换成一份 `LocalOnly → TrustedPersonalOnly` 的表，**本用例的结果一字不变**；
  **这一句不另写用例**，它是同一组断言的一个参数化，写在此处备查。
- `four_class_trust_combinations_and_only_one_passes`（**四种组合逐项，不抽代表**）：
  `NodeClass::{Personal, Temporary} × NodeTrust::{TrustedPersonal, OutsidePersonalTrustDomain}` 四枚节点，
  **四枚的 id 各不相同**（若四枚同 id，会先撞上步骤 1 的判重，见下一条），
  逐个作为**唯一的**节点（即每次 `nodes` 只有一枚）、配 `rules_all_any()`、`artifacts = [artifact(LocalOnly)]`：
  只有 `(Personal, TrustedPersonal)` 那一次 `Ok`，**另外三次各断一次 `Err(NoPlaceableNode)`**。
  **判据**：这四枚就是二值 × 二值的**全部**组合（2 × 2 = 4）；
  **设计 §9 那一行把这个数目写成「五种」**，与本行的「二值 × 二值」自相抵，报在 `## 遗留` 第一节。
  **红的条件（档位：放宽）**：把闸门的判据改成 `class == Personal`（不看 `trust`）→
  `(Personal, Outside)` 那一格红；改成只看 `trust` → `(Temporary, TrustedPersonal)` 那一格红。
  **两处都要能各自红**，故**四次断言分开写**。
- `an_empty_node_set_is_unplaceable`：`nodes = []`、`artifacts = [artifact(Public)]` →
  `Err(NoPlaceableNode)`。**红的条件（档位：移除）**：删掉判空那一步 → 该条以 **panic** 失败
  （对空切片取头）。**那不是干净的红，但确实是红**，照实记（本项目已有「本判据的红形态就是 panic」
  的先例，G 的截止用例同形）。
- `duplicate_node_ids_are_a_named_error`：`nodes = [desktop("a"), cloud("a")]` →
  `Err(PlacementError::DuplicateNode { id })`，**并断言 `id == ComputeNodeId::new("a")`**。
  **夹具里两枚同 id、两枚不同 class**：这样「判重」与「过滤」两件事分得开
  （若两枚完全一样，`Err` 也可能来自别处）。
  **红的条件（档位：移除）**：删掉步骤 1 → 返回 `Ok`（或返回其中一枚），红。
  **判重的次序是承重的**：它是 §5.9「确定」那条断言的前提，不是结尾的卫生检查（设计 §5.2）。
- `an_empty_artifact_set_filters_nothing`：`artifacts = []`、`nodes = [cloud]`、`rules_all_any()`
  → `Ok(n)` 且 `n.id()` 是那枚云节点。**判据**：`LocalOnly` 的判据不适用（没有制品要保护）。
  **红的条件（档位：收紧）**：把空制品集当成「一律拒」（或当成 `LocalOnly`）→ 红。
  **多枚节点下「返回兜底档选中的那个」这一半在 Task 6**（本 task 没有排序）。
- `a_placement_request_is_built_from_artifacts_and_nodes_alone`（**反侧照片**，
  设计 §6.3、裁定配套要求 3）：**只给 `artifacts` 与 `nodes`** 就把 `PlacementRequest` 建出来
  并调用 `place`——**不调 `rank`、不构造任何候选集**。
  **它的证明力边界，写进用例注释**：本 crate 对 `continuum-model-registry` **零依赖**，
  故这份样例在类型上就写不出 `rank`；**它是下界**，**封闭的那一层是 Task 1 的 `ALLOWED` 逐对断言**
  （裁定义务 5 的同一条）。**本用例不重复那些闸门断言**：它只断言「请求建得出来、`place` 跑得通」。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node --test placement
```

- [ ] **Step 3: 实现**

```rust
/// 一次放置的请求。**纯数据，无 I/O**——与 D 的 `RoutingRequest` 同形。
pub struct PlacementRequest<'a> {
    /// §290 的 `required_artifacts` 那一面。§243 的隐私输入从这里读。
    pub artifacts: &'a [Artifact],
    /// §4.3 的「Compute Node 注册」。由 `NodeRegistry::nodes()` 交出。
    pub nodes: &'a [ComputeNode],
}

/// §42 的那一条例，逐字落在 `LocalOnly` 上；其余四档规范未给，故下限取 `AnyNode`
/// （**这不是「允许」，是「规范没有说不允许」**）。
fn spec_floor(level: PrivacyClass) -> TransferRule {
    match level {
        // §42：「Private photo: LOCAL_ONLY …… Cloud Node 即使性能最好，也不能被选择」
        PrivacyClass::LocalOnly => TransferRule::TrustedPersonalOnly,
        // 其余四档：§243／§94 说「按表决定」，而表未给出。本设计不替它填。
        PrivacyClass::Public
        | PrivacyClass::Personal
        | PrivacyClass::Private
        | PrivacyClass::Secret => TransferRule::AnyNode,
    }
}

/// 两层取更严者。**这是闸门不可被策略放宽的全部机制**：`spec_floor` 不读策略。
fn strictest(a: TransferRule, b: TransferRule) -> TransferRule;

pub trait PlacementPolicy {
    /// §94 的表。见 `PlacementRules`。
    fn rules(&self) -> &PlacementRules;
    /// 全序比较：`Ordering::Less` 表示 `a` 排在 `b` 前面。
    /// **操作数就是 `ComputeNode`**——不另立一个「已打分节点」类型（同 D 的 `compare`）。
    fn compare(&self, a: &ComputeNode, b: &ComputeNode) -> Ordering;
}

/// 具名基线。**它不对 §291 的任何一项因子作主张**——十一项里一项都算不出来。
/// `compare` 对任何一对返回 `Ordering::Equal`；次序完全由 `place` 的兜底档决定。
pub struct BaselinePlacementPolicy { /* rules: PlacementRules */ }

impl BaselinePlacementPolicy {
    pub fn new(rules: PlacementRules) -> Self;
}

/// 一次放置。**同步纯函数**：收请求与策略，不接 `Tx`、不做 I/O、不读时钟。
///
/// 返回**单枚**节点——不是候选序列：§291 要的是「放在哪」，`ExecutionProfile.compute_node`
/// 也只要一枚。序列是本函数内部的中间物。
pub fn place<'a>(
    request: &PlacementRequest<'a>,
    policy: &dyn PlacementPolicy,
) -> Result<&'a ComputeNode, PlacementError>;

pub enum PlacementError {
    /// 闸门过滤之后一个节点都不剩（含 `nodes` 为空的情形）。
    NoPlaceableNode,
    /// `nodes` 里同一个 `ComputeNodeId` 出现了两次。**它是「确定」这条断言的守门人**：
    /// 两条同 id 的节点无从定序，`ComputeNodeId` 兜底档也就兜不住。
    DuplicateNode { id: ComputeNodeId },
}
```

**`place` 的步骤 1–3，按次序**（本 task 落这三步，第 4、5 步见 Task 6）：

1. **判重**：`request.nodes` 里同一个 `ComputeNodeId` 出现两次即 `Err(DuplicateNode { id })`。
   放在第一步，因为它是「确定」那条断言的前提，不是结尾的卫生检查。
2. **过闸门**：对**每一枚** `request.artifacts` 读它的 `privacy_class`，算
   `strictest(policy.rules().rule_for(level), spec_floor(level))`，据此把节点集滤到剩下的那一批。
   `TrustedPersonalOnly` 的判据逐字是 `class == Personal && trust == TrustedPersonal`。
3. **判空**：第 2 步之后一枚不剩即 `Err(NoPlaceableNode)`。
   （本 task 在此**取过滤后的第一枚**并返回；Task 6 在这之前插入排序。）

**四条写进实现**：

- **`PlacementError` 的派生**（与 Task 3／Task 4 同口径，**各写一遍不合并**）：
  `#[derive(Debug, thiserror::Error)]` ＋ 两枚变体各配中文 `#[error("…")]`。
  **这一处比另两处更硬**：本 task 的**每一条**失败路径用例都要 `let err = place(…).unwrap_err();`
  ——`Result::unwrap_err` 的签名是 `impl<T: Debug, E> …`，故 `E: Debug` 是**硬约束**；
  `NoPlaceableNode` 那一枚**不带字段**，`#[error]` 直接给它一句中文。
  **不派生 `PartialEq`**（用例一律 `matches!`）。
- **`spec_floor` 的 `match` 穷尽、无通配臂**（纪律 9 的通道 (a)）：把「加档时本函数编译不过」
  写进文档注释。**它的照片不存在**（编译期性质），据实记，**不得为拍它去改 `PrivacyClass`**。
- **`spec_floor` 不读策略、`strictest` 的 `TrustedPersonalOnly` 一侧吸收一切**：
  这两句是本层唯一的 fail-closed 机制，注释里逐字写明。
- **`PlacementRequest` 两个字段都是引用、都是必填**（不是 `Option`）：一个想忽略节点集或制品集的
  调用方，必须先编出一个空切片，那是一次**看得见的选择**，不是一次遗漏（同 D 把 `budget` 写成必填）。
  **第三个字段（§4.3 的 Router 输出）已按裁定删除**，注释里写明这一点与它的后果
  （本 crate 对 `continuum-model-registry` 零依赖；那条边记为阻断）。
- **`place` 的文档注释里写清「今天的唯一生产调用方是测试」**（纪律 6），
  并写明 `ExecutionProfile.compute_node` 是裸 `Option<String>`、`ComputeNodeId` 塞不进去——
  本设计**不擅自收紧它**。

- [ ] **Step 4: 运行，确认绿**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node --test placement
```

- [ ] **Step 5: 真跑裁定义务 1 的那条变异体（**本 task 的核心交付物之一**）**

按纪律 16 的模板，对 `crates/continuum-node/src/placement.rs` 施加**一个**变异：
**把 `place` 步骤 2 里的 `strictest(policy.rules().rule_for(level), spec_floor(level))`
换成只算 `spec_floor(level)`**（即：闸门完全不读 `policy.rules()`）。

```bash
cd /home/DslsDZC/Continuum
MUT=crates/continuum-node/src/placement.rs
BAK="$PWD/.tmp/placement.rs.bak"
LOG="$PWD/.tmp/mut_rules.txt"
before=$(sha256sum "$MUT" | cut -d' ' -f1)
cp "$MUT" "$BAK"
trap 'cp "$BAK" "$MUT"; echo "已还原"' EXIT INT TERM
# …施加变异…
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$LOG"
```

**判据三条（协调者义务 1，逐条核）**：
(a) **唯一红点**是 `a_caller_supplied_rule_can_tighten_the_gate` 这一条的断言；
(b) 其余用例**全绿**（尤其是 `the_gate_cannot_be_widened_by_any_caller_table`——
它与「五档全 `AnyNode`」同解）；
(c) 报告里附「变异前 `sha256` / 还原后 `sha256` / 红在哪一条断言上 / 日志路径」。

**若红点不止一处、或不在这一条上，停下报告**（那说明设计 §9 那一格的推演有误，
是复审者要按 `docs/superpowers/2026-10-06-p3e-decisions.md` 第一节的翻转条件重取的输入）。
**还原后必须再跑一次全量确认全绿**，两次 `sha256` 必须相同。

- [ ] **Step 6: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-node/Cargo.toml Cargo.lock crates/continuum-node/src/placement.rs \
        crates/continuum-node/src/error.rs crates/continuum-node/src/lib.rs \
        crates/continuum-node/tests/placement.rs
git commit -m "feat(node): §243 的硬闸门与 place 的失败路径"
```

---

### Task 6: 排序、确定性与策略可替换（§5.5、§5.9、§5.2 的步骤 4–5）

**Files:**
- Modify: `crates/continuum-node/src/placement.rs`（加步骤 4 的排序；`compare` 的兜底档）
- Modify: `crates/continuum-node/tests/placement.rs`

**Interfaces:**
- Consumes: Task 5 的 `place` / `PlacementPolicy` / `BaselinePlacementPolicy`
- Produces: 无新公开面（**`place` 的最终形态**）

- [ ] **Step 1: 写用例**

在 `tests/placement.rs` 里再加一份测试侧的 `struct ByCapabilityCount { rules: PlacementRules }`：
`compare` 按 `capabilities().len()` **降序**（标签多者在前）。

> **方向与夹具必须同向读一遍再落笔**（本行是第一轮评审的 C2，改法是照复审给的修法）：
> 升序 + 「`"c"` 标签最多」= **赢家是 `"a"`**，与同一段断言的 `"c"` 相抵。
> 现取**降序**（标签多者在前），故下面的夹具里**标签最多的那一枚（`"c"`）胜出**。

**本 task 共用的三枚节点，写死如下**（**三条用例用的是同一组，不再各造一组**）：

| id | 工厂 | `class` | `trust` | `capabilities` 的标签数 |
|---|---|---|---|---|
| `"a"` | `desktop("a")` | `Personal` | `TrustedPersonal` | **0** |
| `"b"` | `desktop("b")` | `Personal` | `TrustedPersonal` | **1** |
| `"c"` | `desktop("c")` | `Personal` | `TrustedPersonal` | **3** |

**两处先说清，免得后来者改错**：
1. **`class` / `trust` 取 `desktop`（Personal / Trusted）是刻意的、且是承重的**：
   本 task 要的是「三枚**都过闸门**」的节点，而 §4.4 的闸门只对 `LocalOnly` 收紧；
   本 task 的制品是 `Public` ＋ `rules_all_any()`，故**四种 `class` × `trust` 组合都能过**
   （两枚 × 两枚 = 4，与 Task 5 那条逐项用例的口径同）——取 `desktop` 是因为
   **它与 Task 5 的工厂同名、语义最清楚**。
   **但它对 Task 5 那条变异体的预测是承重的**：Task 5 的「`spec_floor` 对四档也改成
   `TrustedPersonalOnly`」那条变异体下，**`desktop` 仍被放行 ⇒ 本 task 三条全绿**；
   Task 5 的预测里已经写明这一点（并写明「若这里改成 `cloud(...)`，那份预测要同时改」）。
   **改这一栏之前先读那一条。**
2. **id 与标签数反向**：`"a"` 的 id 最小而标签最少、`"c"` 的 id 最大而标签最多——
   于是**降序策略的赢家（`"c"`）与 id 升序的赢家（`"a"`）是两个不同的节点**，
   这正是下面「确定性」与「兜底档」两条各自可观察的前提。

- `the_same_node_set_in_any_order_yields_the_same_node`（**确定性**）：
  上表那三枚（`Public` 的制品 ＋ `rules_all_any()`），
  用 `ByCapabilityCount` 策略，把同一组节点按**三种不同顺序**放进 `nodes`，各调一次 `place`，
  **断言三次返回的是同一枚**（`"c"`，**策略的序**所指定的那一枚）。
  **红的条件（档位：移除）**：删掉步骤 4 的 `sort_by` 那一行 → 三次返回三枚不同的节点，红。
  **夹具的承重点**：策略的序（标签多者在前）与 id 序**相反**（`"c"` 的 id 最大而排第一）——
  否则「读不读策略」「排不排序」两件事不可观察（等价变异体）。
  **另一处要一起核**：这条的期望值 `"c"` 与本节首行的 `compare` 方向是**一对**——
  把方向改成升序（或把夹具的标签数配反）**必须同时改这一处**，否则照写必红
  （同 `swapping_the_policy_changes_the_result` 的「两次不同」也会一并失去意义）。
- `the_id_breaks_ties_when_the_policy_says_equal`（**兜底档**）：
  **上表那三枚**（`desktop("a")` / `desktop("b")` / `desktop("c")`），
  `BaselinePlacementPolicy`（`compare` 恒 `Equal`），nodes 按 `"c"` / `"a"` / `"b"` 的顺序给，
  → `Ok(n)` 且 **`n.id() == ComputeNodeId::new("a")`**（id 升序的最小者）。
  **红的条件（档位：移除）**：删掉 `.then_with(|| a.id().cmp(b.id()))` → `sort_by` 是**稳定**排序，
  全 `Equal` 时保留输入序，于是返回 `"c"` → 红。
  **第二种（档位：取反）**：把兜底档反过来（`b.id().cmp(a.id())`）→ 返回 `"c"`，红。
  **夹具的承重点**：输入的第一个元素**不是** id 最小的那一个（否则两种实现不可区分）。
- `an_empty_artifact_set_returns_the_tiebreak_winner`（**§9「无制品」那一行的后一半**）：
  `artifacts = []` ＋ **上表那三枚**（**按 `"c"` / `"a"` / `"b"` 的顺序给**）
  ＋ `BaselinePlacementPolicy` → 返回 id 最小的那一枚 `"a"`。
  **与 Task 5 的同名用例的分工写明**：那一条钉「不过滤」（单枚节点，Ordering 无关），
  这一条钉「返回的是兜底档选中的那一枚」（多枚节点）。
  **红的条件（档位：移除）**：删掉 `.then_with(|| a.id().cmp(b.id()))` → 稳定排序保留输入序
  （输入第一个是 `"c"`）→ 返回 `"c"`，红。（2026-10-06 第一轮评审查出本行初稿没给红的条件。）
  **它与上一条是同一条实现（兜底档）的两处观测点**：一条**经**闸门的过滤路径（制品非空）、
  一条**不经**（制品集为空）。**若实现下来发现两处的变异完全等价（同一行代码、同一组入参），
  据实合并成一条并记在报告里**——不为了凑两处而留两条等价用例
  （照 G 的计划 Task 11 对「三处观测点」的同一处置）。
- `swapping_the_policy_changes_the_result`（**策略可替换**）：
  **上表那三枚**（`"a"` 0 个标签、`"b"` 1 个、`"c"` 3 个，`class` / `trust` 同上表）、同一组制品，跑两次：
  一次 `BaselinePlacementPolicy`（全 `Equal`，由兜底档给出 id 最小者 `"a"`），
  一次 `ByCapabilityCount`（**降序**，标签多者在前 → `"c"`）；**断言两次不同**且各是各的那一枚。
  **它钉的是「排序真的读策略」，不钉「哪个策略对」**（设计 §9 那一行的口径逐字如此）。
  **红的条件（档位：移除）**：把 `place` 里的 `policy.compare(a, b)` 换成 `Ordering::Equal`
  （即不读策略）→ 两次都返回 `"a"`，红。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-node --test placement
```

预期：本 task 新加的四条红（Task 5 的用例**全绿**——步骤 4 只影响多枚可比节点的情形，
而 Task 5 的夹具都是单枚存活或 Ordering 无关）。**若 Task 5 的某一条此时变红，停下报告**：
那说明它的夹具其实依赖次序，与本 plan 的拆分前提相抵。

- [ ] **Step 3: 实现**

在步骤 3（判空）之后、返回之前插入步骤 4，并把返回值改成步骤 5：

```rust
// 4. 排序：无论策略给出什么样的比较，`ComputeNodeId` 升序都是最后的兜底档。
    //    `ComputeNodeId` 两两可比，且在同一份 `nodes` 里无相等——后者由步骤 1 的
    //    `DuplicateNode` 保证（这就是「判重在第一步」的理由）。
    filtered.sort_by(|a, b| policy.compare(a, b).then_with(|| a.id().cmp(b.id())));
// 5. 取头：返回排在第一的那一枚的借用。
```

**一句写进文档注释**：排序的机制在本层，**具体打分函数的数值明确推迟**
（§291 十一项因子里九项连量纲都没有，纪律 12）；`BaselinePlacementPolicy` 是**具名的、可替换的基线**，
它的 `compare` 恒 `Equal`——**它不对任何一项因子作主张**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-node/src/placement.rs crates/continuum-node/tests/placement.rs
git commit -m "feat(node): 全序排序与 ComputeNodeId 兜底档"
```

---

### Task 7: 收尾与复核

**Files:**
- Modify: 仅在复核发现缺口时（**任何改动都要写明它属于哪一条判据**）

- [ ] **Step 1: 全量验证**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
TMPDIR="$PWD/.tmp" timeout 600 cargo tree -p continuum-node --depth 1 --edges all --prefix none
```

预期：全绿、0 warning；`cargo tree` 的**直接边恰好是** `continuum-artifact`（＋ `thiserror`，
以及测试目标下的 `trybuild`）——**实测对照，不据口径断言**。

> **下面三条 `git diff` 的对照点取「开工时的 HEAD」**（本计划定稿时是 `3a7ddad`）。
> 开工时若仓库已前进，**把哈希换成实际起点**——判据是「**除本计划的改动之外，这些文件无别的改动**」，
> 不是那串哈希本身。

- [ ] **Step 2: 复核「只登记了自己那一条」（裁定义务 2、4）**

```bash
cd /home/DslsDZC/Continuum && git diff 3a7ddad -- Cargo.toml crates/continuum-runtime/tests/dependency_direction.rs
TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test dependency_direction
```

预期：`Cargo.toml` 的 diff **只有一行**（`members` 加 `crates/continuum-node`）；
`dependency_direction.rs` 的 diff **只有 `continuum-node` 那一条与它的注释**，**别的条目一字未动**。
**若别处被动过，据实报告并回退**（本轮多份计划共写这两个文件）。

- [ ] **Step 3: 复核「不落库、不取迁移号」（纪律 11）**

```bash
cd /home/DslsDZC/Continuum && git diff 3a7ddad --stat -- crates/continuum-runtime/src/main.rs crates/continuum-runtime/src/lib.rs
git diff 3a7ddad --stat -- crates/continuum-artifact crates/continuum-graph crates/continuum-model-registry
```

预期：**两条都无输出**（E 不改 `runtime_migrations()`、不改任何别的 crate 的源码）。
`crates/continuum-model-registry` 的**工作树**里有未提交改动（`M router.rs` / `M tests/router.rs`），
**那不是本计划造成的**——上面第二条 diff 是对 **`2ebbeda`** 比的，故本计划的两条必须为空；
**报告里要注明工作树里那两处未提交改动的存在与归属**，免得被读成本计划碰了 D。

- [ ] **Step 4: 通读复核「注册表入度为零」的两层落点**

```bash
cd /home/DslsDZC/Continuum && grep -rn "continuum_artifact\|Artifact\|PrivacyClass" crates/continuum-node/src/registry.rs
grep -rn "continuum_model_registry\|continuum-model-registry\|RankedExecutionCandidates\|rank(" crates/continuum-node/src/
```

两条**预期零命中**（第一条是 Task 3 的守卫已在钉的，此处是复核；
第二条是「§4.3 的 Router 输出那条边本层零读取」的复核）。
**命中数不是充分证据**：grep 按**行**匹配，跨行折行会漏——故**对 `crates/continuum-node/src/` 的三个
源文件通读一遍**（本 crate 很小，通读代价很低），**结论以通读为准、grep 只用来定位**。

- [ ] **Step 5: 逐条核对完成判据（找不到证据的不得标注为覆盖）**

| 判据 | 证据 |
|---|---|
| §4.4「`LocalOnly` 不会被放置到云端节点」 | Task 5 的正反一对（`a_local_only_artifact_lands_only_on_a_trusted_personal_node` ＋ `…is_unplaceable`） |
| §5.3 闸门不可被策略放宽（绝对措辞） | Task 5 的 `the_gate_cannot_be_widened_by_any_caller_table`（**逐档遍历**） |
| §5.4 (b) 表必填、逐档覆盖 | Task 4 的 `try_new` 三条（少一档逐档五条／重复一档／全档 `Ok`）＋ `rule_for` 逐档五条 |
| §5.4 fail-closed 三条通道 | (a) 编译期——**没有照片**，据实记；(b) Task 4；(c) 构造性事实（唯一解码点在 `PrivacyClass::parse`）＋ Task 7 Step 4 |
| §5.4 「调用方的表真的被读」 | Task 5 的 `a_caller_supplied_rule_can_tighten_the_gate` ＋ **Task 5 Step 5 真跑的变异体** |
| §5.8 两枚 `Err` 各是哪一枚 | Task 5 的 `duplicate_node_ids_are_a_named_error`（断言是哪一枚 id）与三条 `NoPlaceableNode` |
| §5.9 确定性与兜底档 | Task 6 的 `the_same_node_set_in_any_order_yields_the_same_node` ＋ `the_id_breaks_ties_when_the_policy_says_equal` |
| §5.5 策略可替换 | Task 6 的 `swapping_the_policy_changes_the_result` |
| §6.3 构造请求不依赖 Router 输出 | Task 5 的 `a_placement_request_is_built_from_artifacts_and_nodes_alone`（**下界**）＋ Task 1 的 `ALLOWED`（**封闭层**） |
| §7.2「注册表不引 artifact」 | Task 3 的源码文本守卫（**下界**）＋ `ALLOWED` 的逐对断言 |
| §3.5「`ComputeNode` 字段私有」 | Task 2 的 trybuild 样例 ＋ `.stderr` |
| §4.1 注册表三条判据 | Task 3 的三条用例（顺序／具名 `Err`／不静默覆盖） |
| §7.3 `ALLOWED` ＋ workspace `members` 同批 | Task 1 的两侧红 ＋ Step 4 的绿 ＋ Task 7 Step 2 |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 6: 残余落到有版本的文档**

`## 遗留` 在本计划内（`docs/superpowers/plans/` 是有版本的位置）。
**本计划不新建、也不修改第二份文档**；协调者裁定文件的「计划作者报回的设计缺陷」栏由**协调者**登记。

- [ ] **Step 7: 提交（只有当本 task 真的改了文件时才做）**

**本 task 通常一处都不改**（它跑验证与复核）。**若没有文件改动，跳过本步、不造空提交**；
若复核发现了缺口并改了文件，**按实际改动的显式路径逐个 `git add`**——形如：

```bash
cd /home/DslsDZC/Continuum && git add crates/continuum-node/src/placement.rs \
        crates/continuum-node/tests/placement.rs
git commit -m "fix(node): 收尾复核发现的缺口"
```

**不许 `git add -A`、不许 `--amend`**（纪律 19）；**报告里写明这次改动属于哪一条判据**
（Task 7 Step 5 的那张表）。

---

## 遗留

### 一、本计划查出的五条设计缺陷——**已闭（2026-10-06，设计 `3a7ddad` 逐条订正完毕）**

> **先读这一行：本节不是待办，五条都不要再登记。收件人：无（已闭）。** 设计已在 `3a7ddad`
> 按本计划的报告逐条订正，协调者的裁定文件同批记入（**不是**「待协调者登记进 `p3e-decisions.md`
> 的『待补』栏」——那一栏记的是「本计划报回过什么」，不是「还有五条待办」）。
> **下面每条的「原报告」一字未改**（来历照留，照本仓「订正把原话与来历留在原地」的惯例），
> **每条的末尾加一行「订正后的口径」**，写清设计现在是什么样、本计划随没随之改。
> **凡读到「设计还需改 X」的句子，一律以那一行为准。**
>
> **本节引的设计行号（`:796`／`:978`／`:983`／`:520`／`:586`／`:705`／`:692`）一律是 `d30167b` 时的读数**
> ——它们是**原报告的一部分**，作来历留档，**不随设计修订重取**（`3a7ddad` 已把它们整体位移）。
> **计划正文（Task 1–7）引设计时一律按内容引**（用例名、节题），不按行号引——
> 上面那句「按内容引」的射程**只到正文**，不含本节（2026-10-06 第一轮评审查出本计划头部那句
> 没限定射程，已改）。
>
> **为什么整段留着**：这五条里有三条是「同一类错在两处各自漂移」的实例（数目与正文相抵、
> 行号式互引静默失真、来历想当然），它们**在实施期仍会以别的形状出现**；
> 删掉报告就等于删掉这三条判据的来历。

**原处置说明（照留）**：按本仓纪律「计划作者报回、协调者裁定」，本计划**一处都不改设计**，
只在 `## 遗留` 记明证据与落点。**五条里没有一条阻塞实现计划**——它们全是文档层的一致性问题
（两处数目／标签、一组交叉引用的行号、一处来历、一处文件清单），故本计划照实现。

1. **§7.1 的 crate 文件清单漏两份文件**（**阻塞：不阻塞，但计划必须补上**）。
   清单写的是 `tests/{node,registry,placement,rules}.rs`，而 §9 自己要求一份
   「`ComputeNode` 不可外部构造｜trybuild 样例」，它需要 `tests/type_level.rs`
   ＋ `tests/compile_fail/*.rs` ＋ 同名 `.stderr`，以及 `trybuild` 这个 dev 依赖。
   **证据**：设计 `:796`（`tests/{node,registry,placement,rules}.rs`）与 `:978`（trybuild 那一行）。
   **本计划的处置**：`crates/continuum-node/tests/type_level.rs` ＋ `tests/compile_fail/`
   已写进文件结构与 Task 2（见 Task 2 的 Files）。**这一条是「手写文件清单是重灾区」的又一例。**
   > **订正后的口径（`3a7ddad`）**：设计 §7.1 的清单已逐行展开（`tests/node.rs` / `registry.rs` /
   > `placement.rs` / `rules.rs` / `type_level.rs` / `compile_fail/*.rs`），并加了一段「补漏」说明，
   > **同时点名了 `trybuild` 这条 dev 依赖**。**本计划无需改动**（Task 2 的 Files 已与它一致）。
   > **另注**：设计补的是 `trybuild`。**`serde_json` 这条 dev 依赖是设计两处清单都没有的**，
   > 由本计划补（见本节第 6 条），**它不是设计缺陷的残留**——设计从来没要求过那份夹具怎么写。
2. **§9 表那一行的数目与它自己的正文相抵**：`:983` 写「**五种** `class` × `trust` 组合」，
   而同一行的正文写「**二值 × 二值**逐项」——2 × 2 = **4**（`NodeClass` 两枚 × `NodeTrust` 两枚）。
   **证据**：`NodeClass` 两枚（设计 §3.2）、`NodeTrust` 两枚（§3.3）。**本计划按 4 组写夹具**
   （Task 5 的 `four_class_trust_combinations_and_only_one_passes`）。
   > **订正后的口径（`3a7ddad`）**：该格已改为「**四种** `class` × `trust` 组合
   > （`NodeClass` 两枚 × `NodeTrust` 两枚）」，并在格内留了订正说明。**本计划无需改动。**
3. **§9 的两处行号交叉引用在 I1／C4 补行之后没有重取**
   （**这是本设计自己反复处理的那一类错**）。
   - `:520`（§5.3）与 `:586`（§5.4 的 I2 段）都写「照片在 §9 **第 3 行**（最宽策略下的反例）」，
     而它们指的是「五档全写 `AnyNode`」那一格——**它现在是第 5 行**；第 3 行是后补的「放行侧」格。
   - `:705`（§5.9）写「（§9 **第 4 行**）」指确定性那一格——**它现在是第 8 行**。
   - **对照**：`:692`（§5.8）的「（§9 第 1、2 行）」**仍是对的**。
   **根因**：「放行侧」与「调用方收紧」两格是**插在表首附近**的，其后各行的序号整体后移。
   **本计划不受影响**：它引的每一格都**按内容引**（用例名），不按序号引。
   > **订正后的口径（`3a7ddad`）**：设计把这三处（**并同类扫到 §5.8 的第四处**）**一律改为按用例名引**
   > ——「§9 的『闸门不可被策略放宽』那一格」「§9 的『确定性』那一格」等，
   > 并在 §5.9 加了一段说明（判据是「**换引用方式，不是把数字改成新的数字**」：
   > 用例名稳定，插行不改变它）。**本计划无需改动**（它从一开始就按内容引）。
4. **§1.2 与 §5.4 的订正块里，那条「根因」不成立**（**来历错，改正本身是对的**）。
   两处都写错号「指向的是同文件里 `ArtifactType` 的那一套（`as_str` 在 `:47`、`parse` 在 `:68`）」。
   **实测**：`crates/continuum-artifact/src/artifact.rs:112` 是
   **`PrivacyClass::as_str` 自己的文档注释里的一个 `///` 行**（该函数在 `:119`），
   `:129` 是 **`PrivacyClass::parse` 文档注释的首行**（该函数在 `:137`）——
   两个号都落在 **`PrivacyClass` 自己的 impl 内**，与 `ArtifactType` 的两个函数号（`:47`／`:68`）无关。
   **为什么值得报**：本设计自己在 §1.2 的订正块里写过「**一个不成立的来历比没有来历更坏**——
   后来者会信它」；同一条判据在这里同样适用。**改后的号（`:119`／`:137`）是对的**，本计划据此写。
   > **订正后的口径（`3a7ddad`）**：设计在 §5.4 加了一段「**再订正**」，**把那条假根因原话留在原地**，
   > 并附一张对照表（`:112` 是 `as_str` 文档注释里的一行、`:129` 是 `parse` 的文档注释首行，
   > 都在同一个 `impl PrivacyClass` 内），结论与本节**逐字一致**：
   > 真正的错法是「**取了同一个 `impl` 内的文档注释行，而不是函数本体的行**」。
   > §1.2 那一行也已改为指向 §5.4 的订正块。**本计划无需改动。**
5. **§9 那一行的标签与它的正文相抵**：`:978` 的标签写「`ComputeNode` **不可外部构造**」，
   同行的正文写「crate 外**写不出字段字面量**」，而 §3.5 明写 `ComputeNode::new` 是 **`pub`**——
   故**构造点是存在的**，「构造不出来」为假。**本计划按正文写**（Task 2 的 trybuild 样例只钉字段私有，
   预期 `E0451`），**并按正文取用例名与文件头措辞**。
   > **订正后的口径（`3a7ddad`）**：该格标签已收到正文的口径——
   > 「**字段私有 ⇒ 写不出字段字面量（预期 `E0451`）**」，并说明为何值得改
   > （「不可构造」的标签会让人以为本 crate 外部拿不到 `ComputeNode`，与 §3.5 的访问器、
   > §4 的注册表都相反）。**本计划无需改动。**

6. **（本计划自报的第二轮缺陷，2026-10-06 计划评审查出；一并记在此处便于对账）
   Task 5 的夹具要用 `serde_json`，而本计划的 `Cargo.toml` 配方明令不加它、Files 也没列它。**
   **证据**：`Artifact.metadata` / `provenance` 是 `serde_json::Value`
   （`crates/continuum-artifact/src/artifact.rs:177-178`），而 `continuum-artifact` 的
   `src/lib.rs` **不 re-export `Value`**（`artifact.rs:5` 只有一条 `use`）——
   **照原样写必编不过**（原话照留；2026-10-06 收：**具体码以实跑读回的报错为准，
   预期是「名字未解析」这一类**——本行初稿在这里写死了 `E0433`，那是一个**没跑过就写下的码**）。
   **这一条不是设计的缺陷**：设计从未规定夹具怎么写，
   它只是本计划自己在「不加 `serde_json`」那句话里写死了一个错的口径。
   **处置（已改）**：`crates/continuum-node/Cargo.toml` 的 `[dev-dependencies]` 加
   `serde_json = { workspace = true }`，**由 Task 5 登记**（它是第一个使用点），
   Task 5 的 Files 与提交路径已加 `crates/continuum-node/Cargo.toml` 与 `Cargo.lock`，
   「关于本计划的代码块」的清单要点与 Global Constraints 第 19 条已同步改正。
   **它是外部 crate，不进 `ALLOWED`**；**库本体一个 `Value` 都不用**。

**另有一处不是缺陷、但需后来者对账**：`crates/continuum-model-registry/src/router.rs` 在**工作树**里
有未提交改动（`M`）。设计 §6.3 要求「计划接手前须重取」——**本计划在 2026-10-06 重取过一次**，
六个号在**那一次**的读数下逐条属实（`RankedExecutionCandidates` `:394`、`selected` `:403`、
`ExecutionCandidate` `:345`、`RoutingReason` `:276`、`RankingPolicy` `:424`、`rank` `:512`）。
**但那是那一次的读数，不是本计划给出的常量**（Global Constraints 第 3 条：
**一律「以当时工作树为准」**）。**E 对 D 零依赖**，故这些号**在本计划正文里一处都不承重**；
将来若取设计 §6 的读法 (b)，**接手的人须在**那时**的工作树上再取一次**。

### 二、设计 §10 的 17 条：本计划一条都不接，收件人与阻塞范围照原文

**逐条抄**（`docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md` §10）：

| 条 | 是什么 | 收件人 | 阻塞范围（照原文） |
|---|---|---|---|
| 1 | OPEN-007（隐私等级的设定与传播） | 规范维护者 | 只阻塞「判据的真实性」，不阻塞设计，也不阻塞计划 |
| 2 | §94／§243 的「privacy × trust class」表规范未给出 | 规范维护者＋候选产生方（`data_policy` **与 `node_policy`**，或用户策略） | 阻塞「保证的完整」，不阻塞接口冻结 |
| 3 | §287 的 `trust` 无取值域 | 规范维护者 | 不阻塞（已取二值并说明来历） |
| 4 | `capabilities` / `resources` / `availability` 无取值域 | 规范维护者 | 不阻塞接口，但使「排序」这一半在数值上不可实现 |
| 5 | §93 的 artifact locality 无输入 | 规范维护者 | 不阻塞 |
| 6 | 「云端节点」在 §287 里没有对应轴 | 规范维护者 | 不阻塞 |
| 7 | 排序没有基线，也没有可写的基线 | 规范维护者＋语义层设计 | 不阻塞（判据是 MUST 侧的性质，不依赖排序数值） |
| 8 | §4.3 的 `节点放置 ← Router 输出` 指输出的哪一部分 | 规范维护者＋复审者 | **阻塞接口，已按裁定取甲**（接口本身已冻结） |
| 9 | §289 的四条默认禁止与 §290 的 Job Capsule 无落点 | 协调者＋规范维护者 | 不阻塞 E |
| 10 | 注册表没有刷新入口 | 规范维护者 | 不阻塞 |
| 11 | 放置的隐私裁决是否应经 P2 的 Policy Engine | 复审者 → 协调者 | 不阻塞本设计；若改判，E 与 P2 两侧都要改 |
| 12 | `NodeTrust` 的产生方不存在 | 《工程》维护者（备查，**无需动作**） | 不阻塞 E，且**不构成任何待办** |
| ~~13~~ | ~~本设计不落库、不取迁移号~~ | **已删**（原位留删除线记录，编号不重排） | —— |
| 14 | §304 的「多少并行」有字段、无产生方 | 构造 `ExecutionProfile` 的那一方＋规范维护者 | 不阻塞 E |
| 15 | `ExecutionProfile.compute_node` 是放置结果的落点，但无产生方 | 驱动侧的节点执行装配点＋协调者 | 不阻塞（本设计的唯一生产调用方就是测试） |
| 16 | §224 的 `node_policy` 与 `PlacementRules` 是不是同一件事 | 语义层设计＋规范维护者 | 不阻塞 E；若取读法一，E 与 P4 之间多一条接缝，须两侧同改 |
| 17 | `PlacementRules` 里 `LocalOnly` 那一格是装饰性的，要不要给这个槽换个形状 | 复审者（形状取舍）＋规范维护者 | 不阻塞接口与计划（三种备选下 `place` 的行为完全相同） |

**本计划对第 20 条纪律（不发明）的落点**：第 2、4、7、16 条所缺的**值、量纲、权重、映射**
**本计划一个都不填**——`spec_floor` 只照录 §42 的那一条例，其余四档交给调用方；
`BaselinePlacementPolicy::compare` 恒 `Equal`；`capabilities` / `resources` / `availability`
只搬运不解释。**第 1 条**的限度（等级若赋错，闸门就按错的等级放行，E 无法察觉也无法补）
写进 `place` 的文档注释。

### 三、本计划自定的四处形状（设计没写，须复审确认）

按「设计没写清 → 回写设计」还是「为计划好写而改口径」逐条定性：

1. **`ComputeNodeId` 的面**（**设计 §3.6 只说「E 自己的 newtype」，没给它的面**）；
   本计划取 `String` newtype ＋ `new(impl Into<String>)` ＋ 派生
   `Debug, Clone, PartialEq, Eq, PartialOrd, Ord`，**不建 `as_str()`**。
   判据：`Ord` 的消费方是 Task 6 的兜底档（`a.id().cmp(b.id())`）；`PartialEq` 的消费方是
   `DuplicateNode` 的断言；**`as_str()` 今天零消费方**——C 为 `model_providers()` 留下的教训是
   「零消费方的公开方法即建好没人用」（设计 §4.1 自己引了这条）。
   **定性：补设计的空白**（不是改口径），方向对，**建议回写设计 §3.6**。
2. **`NodeClass` / `NodeTrust` 的派生集**（设计未给）：`Debug, Clone, Copy, PartialEq, Eq`。
   判据：闸门要比较它们（`class == Personal` / `trust == TrustedPersonal`），故 `PartialEq` ＋ `Eq` 必需；
   `Copy` 使访问器按值返回（照 D 的 `RoutableState::state()` 的处置）。
   **定性：补空白。**
3. **任务拆分点**：`place` 的步骤 1–3 在 Task 5、步骤 4–5 在 Task 6
   （Task 5 的 `place` **不排序**，返回过滤后的第一枚）。
   判据：这样 Task 6 的四条用例对 Task 5 的实现**是红的**，而 Task 5 的用例全是
   「过闸门后恰好剩一枚」或「Ordering 无关」的夹具——**两段各自红一次**。
   **定性：这是计划内部的执行序，不改设计的任何判据或签名**（`place` 的签名与返回在 Task 5 就定死，
   与设计 §5.2 一致）。**但它是本计划对设计正文的一处「分期兑现」，须在复审时被看见**。
4. **测试文件的划分与取名**：`tests/{node,registry,rules,placement,type_level}.rs` ＋
   `tests/compile_fail/`。**定性：补空白**（设计 §7.1 的清单漏了后两份文件，见第一节第 1 条），
   **建议随第 1 条一并回写设计 §7.1**。

### 四、相机拍不到的照片

**设计 §8 的八条全部沿用，不重述**（端到端的一次真实分布式放置；§291 九项因子的实际权衡；
OPEN-007 的赋值与传播；§290 的 Job Capsule 真的把数据限制住了；§5.4 通道 (a) 的枚举穷尽；
§342.7 的「不得自动进入」；§93 的 data movement cost；以及第 8 条——「调用方的表真的被读」
**今天只有论证、实施期第一件事就能拍**）。**本计划补三条**：

1. **「`PlacementRules` 的覆盖率来源是 `ALL` 而不是手抄清单」今天不可观察**。
   两版（`ALL` 驱动 vs. 手抄同样五档）在**今天**是**等价变异体**：举不出任何入参让两者给出不同结果
   （要区分它们必须先有一个第六档，而加档是规范的事）。
   **处置**：**不写一条声称「来源是 `ALL`」的运行期断言**（那会是假照片），
   只把 Task 4 的 `the_coverage_criterion_reads_the_five_levels_from_the_type` 写成
   **输入侧事实**（`ALL` 是五枚且无重复），并在实现里照 `ALL` 写、注释写明理由。
   **收件人：复审者**（若认为该换成别的形式钉）。
2. **反侧照片（构造请求不依赖 Router 输出）的证明力是下界**。
   `continuum-node` 对 `continuum-model-registry` 零依赖，故那份样例在类型上写不出 `rank`——
   **封闭的那一层是 `ALLOWED` 的逐对断言**（Task 1），与裁定义务 5 是同一条。
   **收件人：无（据实记，供后来者对账）。**
3. **「`ALLOWED` 的条目与 `Cargo.toml` 的声明逐对相等」这一条**，本计划只靠 Task 1 的两侧红
   ＋ Task 7 Step 2 的 `git diff` 复核；**本 crate 没有一条用例能钉住「将来有人给
   `crates/continuum-node/Cargo.toml` 加一条边却不改 `ALLOWED`」**——那条边一旦加上，
   `every_crate_depends_only_on_its_allowed_set` 会红（逐对双向断言；**行号只是当时读数**：
   定稿时 `:276-281`，2026-10-07 实读 `:291-296`），
   **故它其实是有照片的**，此处记明它的落点在 `continuum-runtime` 的测试里，不在本 crate。
   **收件人：无。**

### 五、E 不取迁移号（核实）与「不落库」的边界

**核实**：`crates/continuum-runtime/src/main.rs:83-93` 的 `runtime_migrations()` 是本仓唯一的装配清单，
E **不在其中、也不需要**（不落库）。**本计划不预留任何号**——号段裁决
（A 50、B 60、C 70、D 80、E 90）**不构成本计划占号的依据**；将来若要落库，
**取号前须现场核对当时该档未占用**（本项目删过一次「预留号」，D 的计划 Task 5 与 §11 第 14 条记着）。
**判据**：Task 7 Step 3 的两条 `git diff --stat` 均为空。

**「不落库」的代价照实记**（设计 §4.2 末段）：进程重启后节点集合为空，`place` 只会返回
`NoPlaceableNode`，直到有人重新登记——**今天的唯一登记方是测试，故这一条没有可观察形态**。

### 六、本设计无实现体：一条义务在今天仍未闭合

**设计 §8 第 8 条**（`d30167b` 新增）与 **§9 的「调用方收紧」格**都写着同一件事：
「调用方的表真的被读」这条**今天只有逐字推演的论证、没有照片**。
**本计划对它的落点是 Task 5 的 Step 1（用例）与 Step 5（真跑变异体）。**
**义务的闭合判据**：交付报告里必须有「唯一红点是 `a_caller_supplied_rule_can_tighten_the_gate`」
这句话，附日志路径与两次 `sha256`。
**若那句话不成立**（红点不止一处或不在这一条上），**按协调者裁定第一节的翻转条件处理**
（那意味着该格的推演有误，须由复审者重取），**不得改断言或改实现来让它成立**。
