# P3 子项目 G（模型调用路径）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把**模型调用路径**接上生产路径——取 `ProviderHealth` 快照、组装 `RoutingRequest` 并调 D 的 `rank`、
消费 `RankedExecutionCandidates`、对选中的模型发 `ModelProvider::invoke` / `stream`、把适配器的失败转成 `FailureClass`。

**Architecture:** **不新建 crate**，落在 `crates/continuum-runtime/src/model_call.rs`（与 F 的 `tool_call.rs` 同址；
两者只共享 crate，**路径不同、强制点覆盖不同**——G 这条路径上**没有任何强制点**）。模块内部按设计 §3.1 的
四段切：`plan_candidates`（同步段，唯一收 `Tx` 的入口）→ `select`（异步，取快照 + 调 `rank`）→
`call` / `call_stream`（异步，发起）→ `abort`（异步，中止一条流）。**异步段一律不收 `Tx`**（设计 §3.4：`Tx` 持
`MutexGuard`，它无条件 `!Send`；且整个库是单连接单 `Mutex`，跨 `await` 持有会把整仓库访问挡住）。

**G 不做判断，只做执行**：模型能不能被路由由 **D** 判（闸门在 `RoutableModel::try_new`），
id 由哪个适配器服务由 **C** 判（`model_for`），G 把两者接起来发一次调用。
G 的公开面只接受 `&ExecutionCandidate`，**不接受 `ModelId`**（设计 §2.2）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-core`（`ModelId` / `InvokeRequest` / `InvokeResponse` /
`ModelStream` / `CallId` / `Message` / `ProviderHealth` / `ProviderError`）、`continuum-provider`
（`ModelProvider` trait、`ProviderRegistry`）、`continuum-model-registry`（`RoutingRequest` / `TaskSkillRequirement` /
`FamilyPreference` / `BudgetView` / `RankingPolicy` / `rank` / `RankedExecutionCandidates` / `ExecutionCandidate` /
`RoutingError` / `RoutableModel` / `list_registered` / `load_profile`）、`continuum-persist`（`Tx`）、
`continuum-graph`（`FailureClass`）；`tokio`（`time` feature，**由本计划新增**）；dev：`async-trait`、`futures-core`、
`tempfile`。

**设计依据：** `docs/superpowers/specs/2026-10-06-p3g-model-calling-path-design.md`（**唯一事实来源**）。
本计划不改设计、不改规范、不改 C 与 D 的任何接口——**对 C/D 只作消费方**。

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

**执行序是串行 `B → D → C → F`，G 在最后**（`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` §四：
「G 本轮不设计，等 B/C/D/F 落计划后再起」）。故 G 是全仓**唯一同时依赖 C 与 D 的产物**的流。

### 依赖的交付物与它们的**实际状态**（2026-10-06 实读工作树）

| 依赖 | 状态 | 用在哪 |
|---|---|---|
| `continuum_provider::ModelProvider` trait（§315 七方法，`crates/continuum-provider/src/model.rs:14-21`） | **已交付** | G 的整个发起面；夹具适配器实现它 |
| `continuum_core::model::{ModelId, InvokeRequest, InvokeResponse, ModelStream, CallId, Message, ProviderHealth}`、`continuum_core::ProviderError` | **已交付** | G 的请求/响应类型 |
| `continuum_graph::failure::FailureClass` | **已交付** | 分类的像 |
| `continuum_model_registry::{RoutingRequest, TaskSkillRequirement, FamilyPreference, BudgetView}` | **已交付**（D 的 Task 10） | `select` 组装请求 |
| `continuum_model_registry::{RoutableModel, RoutingError, load_profile, register_model}` | **已交付** | `plan_candidates` 的闸门与画像 |
| `continuum_model_registry::{rank, RankedExecutionCandidates, ExecutionCandidate, RankingPolicy, CandidateScore, RoutingReason}` | ~~**未交付**（D 的计划 Task 11；实读 `grep -rn "pub fn rank\|RankedExecutionCandidates" crates/continuum-model-registry/src/` 零命中）~~ **已交付**（订正见下） | `select` 的输出、`call` 的入参 |
| `continuum_model_registry::list_registered` | ~~**未交付**（D 的计划 Task 14 Step 1，`docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md:1408`）~~ **已交付**（订正见下） | `plan_candidates` 的 id 清单**唯一来源** |

**订正注记（2026-10-07，原话照留）**：上面那一格的「**未交付**」**在本计划写下的时点成立，现已过期**——
D 的 Task 14 Step 1 已把它交付：`crates/continuum-model-registry/src/persist.rs` 的
`pub fn list_registered(tx: &Tx<'_>) -> Result<Vec<(ModelId, LifecycleState)>, PersistError>`
（**按 id 升序**；**不过滤状态**——闸门在 `RoutableModel::try_new`，此处滤掉会把「哪些模型存在」与
「哪些能路由」混成一件），提交 `510a90d` / `80250f2`，照片是
`list_registered_lists_every_registered_model`（`crates/continuum-model-registry/tests/persist.rs:535`）。
故 **G 的 Task 6 起不再卡在这一项上**；其余与之相邻的状态词见下面的扫描结果。
**同一条注记也覆盖上表那一格的 `rank` 一族**（原话照留：`未交付` ＋ 那句「实读 `grep …` 零命中」）：
**两者都已交付**——D 的 **Task 11** 落的，`crates/continuum-model-registry/src/router.rs`
有 `pub fn rank`（`:523`）与 `pub struct RankedExecutionCandidates`（`:405`），
同模块还有 `ExecutionCandidate` / `RankingPolicy` / `CandidateScore` / `RoutingReason`。
**故那句「零命中」现在也是错的**，而它**尤其危险**：它读起来像一次实测，却是**带时点的事实陈述**——
时点一过，它与「我刚 grep 过」无法从字面区分。

**判据写进原地**：**「未交付／尚未／待补」这类词是带时点的状态陈述，不是标签**——
**写它要带时点（必要时带分支）；事实变了必须回来改那一条**，否则后来者会
**照它去做一件已经做过的事**，或者**以为一件已经能做的事还做不了**
（本仓已在 D 的计划里栽过一次同类：遗留条在代码落地前就写着「已补」）。
它与本仓已立的「『会／不会 X』要问一句『在哪个时点』」是同一条。
**另一条同源的判据（见 `:50` 那节）**：**状态陈述在「同一份计划被两条分支同时读」时会两边都像真的**
——故凡「某处尚未存在」这类话，**要带分支与时点**。
| `continuum_provider::{ProviderRegistry, RegistryError}`、`ProviderRegistry::model_for` | **截至 2026-10-07，在分支 `p3d` 上：未交付**（C 的计划 Task 1，`docs/superpowers/plans/2026-10-05-p3c-provider-boundary.md:326`；**实现落在 `.worktrees/p3cf` 那条流里**——同一天在该分支上它已存在） | `plan_candidates` 解析适配器 |
| `crates/continuum-runtime/src/error.rs`（F 的 `TaskError` / `ToolReportedError` 在里面） | **截至 2026-10-07，在分支 `p3d` 上：未交付**（实读：该文件不存在；**F 的实现同样落在 `.worktrees/p3cf` 那条流里**） | **`ModelCallError` 的落点**（设计 §10.2） |

### 哪些 task 在别的子项目交付之前动不了（**按子项目分别成立；带时点与分支**）

**订正（2026-10-07，原话照留）**：本小节原标题是「哪些 task 在 **C/D 未交付**之前动不了」——
**那句对 D 已不准**（D 的 Task 11 与 Task 14 Step 1 都已交付，见上表的订正注记），**对 C 仍准**。
**故本节的结论一律按子项目分别读**，且**都要带时点与分支**：
**截至 2026-10-07**，D 侧**在分支 `p3d` 上**已交付 `rank` 一族与 `list_registered`；
**C 与 F 的实现落在 `.worktrees/p3cf` 那条流上**，**在 `p3d` 上尚未交付**。
**判据写进原地**：**状态陈述在「同一份计划被两条分支同时读」时会两边都像真的**——
**故凡「某处尚未存在」这类话，要带分支与时点**。

- **Task 1–5 全不依赖 C 与 D**（`ModelCallError` 的落点除外，见下）：它们只用
  `continuum-core` 的既有类型、`ModelProvider`（已交付的 trait）、`FailureClass`、以及**源码文本**。
- **Task 6 起全部硬依赖**：`plan_candidates` 要 `list_registered`（D）与 `model_for`（C）；
  `select` / `call` / `call_stream` / `abort` 的**签名里就写着** `RankedExecutionCandidates` / `ExecutionCandidate`，
  故 D 的 Task 11 未落地时**从 Task 6 起编不过**（不是「行为不对」，是**编译失败**）。
  **订正（2026-10-07，原话照留）**：**D 的那一半已过期**——`rank` 一族与 `list_registered`
  **截至 2026-10-07 在分支 `p3d` 上已交付**（证据见上表的订正注记）；**C 的那一半仍成立**
  （`model_for` 实现在 `.worktrees/p3cf` 那条流上，`p3d` 上未交付）。故今日的实际阻塞项是 **C（＋F）**，不是 C/D。
- **Task 1 的落点依赖 F**：设计 §10.2 把 `ModelCallError` 记在 `crates/continuum-runtime/src/error.rs`
  （「F 已在那里放了 `TaskError` 与 `ToolReportedError`」）。**截至 2026-10-07，在分支 `p3d` 上：
  F 尚未落地、该文件不存在**（F 的实现落在 `.worktrees/p3cf` 那条流里）——
  故 G 的实际前置是 **C / D / F 三个**，而设计 §4/§10 只写了 C 与 D。
  **本计划不代 F 建那个文件**（那会让同一个文件有两个创建者，而 F 的 Task 2 还要把 `TaskError` 整块搬进去）。
  开工前先核：

```bash
cd /home/DslsDZC/Continuum && ls crates/continuum-runtime/src/error.rs crates/continuum-runtime/src/tool_call.rs
```

  两者都在 → Task 1 只**增补**。任一不在 → **停下报协调者**（记在 `## 遗留` 的同名条）。

### 本节把什么交给谁

- **交给 C 的注册表使用者（G 自己，本计划只作消费方）**：G 调 `model_for(&ModelId)` 并**把
  `RegistryError::NotFound` 当成「这一条不是候选」丢掉**（设计 §3.2 的合取 (c)）。
  **G 不改 `ProviderRegistry`、不改 `model_for` 的签名、不新增任何注册表入口**。
- **交给 D 的实现者（消费方视角，不改其接口）**：G 调 `list_registered(tx)` 建候选集的 id 清单，
  并调 `rank(&RoutingRequest, &[RoutableModel], &dyn RankingPolicy)`
  （设计 §4.1、§14 第 10 条：这条请求**已由 D 的计划 Task 14 Step 1 认领**——**订正（2026-10-07，原话照留）：
  「尚未落地为代码」已过期，它已交付**，见上表那一格的订正注记：`persist.rs` 的 `list_registered`、
  提交 `510a90d` / `80250f2`、照片 `list_registered_lists_every_registered_model`）。
  **G 不重述、不改动这两个接口**。
- **交给协调者**：`ModelCallError` 的落点文件依赖 F（见上）；设计 §3.1 代码块与正文的两处不一致（见 `## 遗留`）。
- **规范维护者（本项目无此角色）**：设计 §14 的第 1、2、3、4、5、6、8、9 条——**本计划一条都不发明**，
  收件人照 §14 原文，逐条抄在 `## 遗留`。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。本计划不新增任何 workspace 成员之间的依赖边**：`continuum-runtime` 的 `ALLOWED` 条目
  已含 G 需要的全部 crate（实读 `crates/continuum-runtime/tests/dependency_direction.rs:164-179`：
  `continuum-provider` / `continuum-model-registry` / `continuum-persist` / `continuum-core` / `continuum-graph`）。
  `ALLOWED` **一字不改**，`crates/continuum-runtime/Cargo.toml` 的 `[dependencies]` **一条不加**。
  本计划只改 **dev-dependencies**（`async-trait`、`futures-core`）与 **workspace 的 `tokio` feature**（加 `time`）——
  两者都是**外部 crate**，不进 `ALLOWED`（那张表只逐对断言 workspace 成员之间的边）。
- **`ALLOWED` 挡不住本节的一条守卫，据实写明**：runtime 的条目本来就含 `continuum-capability` /
  `continuum-effect` / `continuum-secrets`（F 在用），故「G 不碰能力 / 效应 / 凭据」**不可能由依赖边来钉**
  （多引一个已允许的 crate 不会让任何断言变红）。它的守卫只能是**模块面**（Task 5），且那是**下界**。
- **本计划不改的文件**：`continuum-provider` 的任何文件（只**调用**它的 trait 与注册表）、
  `continuum-model-registry` 的任何文件（只**调用**它的公开面）、`continuum-core` 的任何文件、
  `docs/02-工程.md`、共享面、C 与 D 的设计与计划。**G 不许顺手修 C/D 的任何东西**。
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增 dev 依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，
  锁文件按本行办。
- **`tokio` 的 `time` feature 由本计划打开**（设计 §3.5：工作区今天只开 `rt-multi-thread` 与 `macros`，
  实读 `Cargo.toml:29`；全仓 `grep -rn "tokio::time\|tokio::spawn" crates/` 零命中）。
  **它是外部 crate 的 feature，`ALLOWED` 不受影响**；是否牵动 `Cargo.lock` 的包集合**由实现时实测**，
  本计划不预判（设计 §3.5 同此口径）。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不调 `list_models()` / `describe_model()` / `usage()`**（设计 §3.2 末条、§5；三条各有一张否定式照片）。
  - **不发明探活超时、不给流的分片消费加截止**（设计 §3.3、§3.5；规范未给判据，记 §14 第 3、4 条）。
  - **不做重试裁决，也不做升级触发**（设计 §6.4 的三条判据：Contract / Budget 不是 G 的词汇；
    `decide_retry` 要 `&Operator` 与 `&RetryPolicy`；失败裁决者只能有一个）。
  - **不记账、不解释 `BudgetView` 的量纲、不做减法**（设计 §8.1；ENG-005 归语义层）。
  - **不产生 `FailureClass::Constraint`**（要预算树，ENG-005 §三.2）。
  - **不写审计行、不写执行画像、不写 Effect Journal**（设计 §9：两处落点都不存在，且一次模型调用不是 §268 的外部效应）。
  - **不收紧 `ExecutionProfile` 的 `model` / `provider`**（设计 §7：缺的是节点执行的装配点，不在 G 的交付面里）。
  - **不定义 `usage()` 的语义**（设计 §5：两种读法都不成立）。
  - **不新建 crate、不建 trait、不发明类型**（设计 §10.1）。
- **迁移编号与 `main.rs` 注册**：G **一个新迁移都没有**（它不建表）。`runtime_migrations()` 一字不改。

### 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；
   正向的「变红」跑全量是加分。**变异的三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，
       处理是**换真变异体而非补用例**；
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
       cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
   **每一处「预期谁红」都标了档位**（**取反 / 放宽 / 收紧 / 移除**），见各 task 的用例条。
2. **变异脚本必须带还原护栏**（本仓出过一次「变异留在源码里」的事故）：
   每次变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、**每轮用独立日志路径**，
   报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径」。
   模板（`MUT` 是改的文件、`BAK` 是备份）：
   ```bash
   cd /home/DslsDZC/Continuum
   before=$(sha256sum "$MUT" | cut -d' ' -f1)
   cp "$MUT" "$BAK"
   trap 'cp "$BAK" "$MUT"; echo "已还原"' EXIT INT TERM
   # …施加变异…
   TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$LOG"
   # …读 $LOG 判红…
   ```
   **`trap` 那一行不许省**——本条是这一轮唯一一次事故的直接产物。
   > **订正（2026-10-07，G Task 6 修复轮实测：护栏本身失效过一次）**：
   > 那一轮实现者的**通用平台脚本用了 `cp --preserve=mtime`**，而**本机的 `cp` 不认这个参数**
   > ⇒ **备份压根没建成**，于是 `trap` 的还原也失败，**变异跑完源文件仍是变异版**。
   > 它是靠「**变异前哈希 ≠ 还原后哈希**」这两个数对不上才发现的，然后反向改回那一处、核回 `e0279d25…` 复原。
   > **两条要写进模板**：
   > - **基线副本一律用不带任何 `--preserve`／`-p` 的 `cp`**——**mtime 不由 `cp` 管，交给 `touch`**
   >   （`touch` 是给 cargo 用的，不是给备份用的；`cp -p` 在这里既无用又可能不被支持）。
   >   **更稳的一条**：**备份建成后立刻核一次 `[ -s "$BAK" ] && diff -q "$MUT" "$BAK"`**——
   >   **备份没建成时，`trap` 不会报错，它只是还原了一个空文件或什么也没做**。
   > - **「变异前哈希」与「还原后哈希」必须同框打印**（`echo "$before vs $(sha256sum "$MUT" | cut -d' ' -f1)"`）——
   >   **本轮正是这两个数不一致才暴露了护栏失败**；分成两处写在报告的不同小节里，就看不出来了。
   > **判据（可搬用）**：**一个「还原护栏」的失效是静默的**——**它失败时不会报错，只会留下一份被改过的源码，
   > 而下一轮的读数会建在它上面。故护栏必须自证**：**备份建成了没有**、**还原后与变异前是不是同一个值**，
   > 这两件都要当场打印，不能只写在报告里。
3. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**。
   **枚举式绝对断言须逐项有照片**——「五个变体逐项」的每条臂各要一条照片，不抽代表。
   **断言的作用域要与事实同宽**：「整张表的唯一 X」≠「某个字段的唯一 X」（这一轮已两次栽在这里）；
   「本模块的源码文本里零命中」≠「全仓零命中」。
4. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」；**并断言没有半写的副作用**
   （本计划的所有失败面都是 `ModelCallError` 的五个变体之一，逐变体至少一条用例；
   副作用面见 Task 11 的行数不变断言）。
5. **一个纯数据声明 / 纯投影的类型没有运行期照片，只钉类型与签名（编译期）**——
   本计划的 `Candidate` / `CallPlan` / `RouteInput` / `CallInput` 四个类型全是这一类，处置见 Task 1、Task 6。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。

**变异日志是证据，必须活到复审结束**：**实现者保留 `.tmp/`，由协调者在复审结束后清理**。
报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。
**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；凡与既有 crate 交互的签名
（`Tx::query` / `Tx::execute` / `Value` / `model_for` / `rank` / `list_registered` / `load_profile`），
实现前须先读该 crate 的源码确认，不符时以源码为准并回报。

**本计划特有的四处「签名即判据」，照抄前须对源**：

- `ModelProvider` 的七方法签名（`crates/continuum-provider/src/model.rs:14-21`）：`invoke` / `stream` 收
  **`InvokeRequest`（按值）**，`cancel` 收 **`&CallId`**，`health` **无参**且**返回
  `ProviderHealth`（不是 `Result`）**。夹具适配器与实现都照这份签名写。
- `InvokeRequest` 恰三个字段（`model` / `messages` / `max_tokens`，`crates/continuum-core/src/model.rs:44-48`）：
  **没有授权位**——这是设计 §2.1 的强制点 (1) 那一格的正面依据，Task 5 把它拍成编译期照片。
- `ModelStream` 的字段是 `call: CallId` 与 `chunks`（`:89-92`），**它不带适配器句柄**——
  故 G 的中止入口必须另收一枚句柄（Task 9）。
- `Tx<'a>` 持 `std::sync::MutexGuard<'a, rusqlite::Connection>`（`crates/continuum-persist/src/tx.rs:12-15`），
  **无条件 `!Send`**（设计 §3.4 已把初稿的归因订正过）。

---

# 文件结构

```
crates/continuum-runtime/
  Cargo.toml                     dev-dependencies 加 async-trait / futures-core（Task 2、Task 3 各登记自己那一条）
  src/lib.rs                     加 pub mod model_call;（与 F 的 pub mod error / tool_call 同一份文件，各登记自己那几行）
  src/error.rs                   G 向 F 已建的文件**增补** ModelCallError（Task 1；F 未落地则停下报，不代建）
  src/model_call.rs              G 的全部实现：Candidate、plan_candidates、snapshot、RouteInput、select、
                                 CallPlan、CallInput、call、call_stream、abort、classify、ModelCallError→…的转换
  tests/model_call.rs            同步段与发起面的运行期用例（含夹具播种 D 的画像行）
  tests/common/mod.rs            假模型适配器（C 的 tests/common/mod.rs 里那个 FakeModel 的**第二份副本**，
                                 **两份不合并**——`docs/superpowers/p3bcdf-followups.md` §七.8）
  tests/model_call_discipline.rs 模块面守卫（用词纪律 / 中立性 / 零能力与凭据引用 + 正控制 + 归一化照片）
  tests/model_call_face.rs       「没有强制点」的编译期照片（InvokeRequest 无授权位、七方法逐个可调、
                                 异步段的 future 是 Send）

Cargo.toml（workspace）          tokio 的 features 加 "time"（Task 8 登记，它的第一个使用者）
```

**测试目标的划分**：`tests/model_call.rs` 用一个 `mod common;`（integration test 各是一个 crate，
夹具要靠 `mod common;` 共享）；`tests/model_call_discipline.rs` **不** include 夹具（它只读源码文本）。

**既有 crate 里本计划要改的文件**（只有两处，都是**多写者单文件**，各登记自己那几行）：

```
crates/continuum-runtime/src/lib.rs                     加 pub mod model_call; 与 pub use error::ModelCallError;
crates/continuum-runtime/src/error.rs                   增补 ModelCallError（文件由 F 建）
crates/continuum-runtime/Cargo.toml                     dev-dependencies 加 async-trait / futures-core
Cargo.toml（workspace）                                  tokio features 加 "time"
```

**本计划不碰的文件**：`crates/continuum-provider/**`、`crates/continuum-model-registry/**`、
`crates/continuum-core/**`、`crates/continuum-runtime/tests/dependency_direction.rs`（**一个字都不改**——
G 不加任何边）、`crates/continuum-runtime/src/main.rs`（G 不建表、不装配）。

---

### Task 1: `ModelCallError` 与它到 `ProviderError` 的转换

**Files:**
- Modify: `crates/continuum-runtime/src/error.rs`（**由 F 创建**，本 task 只增补）
- Modify: `crates/continuum-runtime/src/lib.rs`
- Create: `crates/continuum-runtime/src/model_call.rs`（本 task 只放**两个函数** `classify` / `into_call_error`，不含四段流程）
  **订正（2026-10-07，G Task 1 实测；原话照留）**：原话写「本 task 只放**类型与转换**」——这读起来像
  **类型也落在 `model_call.rs`**，**是错的**：**类型 `ModelCallError` 落 `crates/continuum-runtime/src/error.rs`**
  （设计 §10.2 点名「G 向 `error.rs` 增补 `ModelCallError`」），`model_call.rs` 只放 `classify` 与
  `into_call_error` 两个函数。实测实现即如此落的（`crates/continuum-runtime/src/model_call.rs`）。
- Create: `crates/continuum-runtime/tests/model_call.rs`

**Interfaces:**
- Consumes: **既有的** `continuum_core::ProviderError`、`continuum_graph::failure::FailureClass`、
  `continuum_model_registry::RoutingError`、`continuum_persist::PersistError`
- Produces: `continuum_runtime::ModelCallError`、`continuum_runtime::model_call::{classify, into_call_error}`

**开工前置**：`crates/continuum-runtime/src/error.rs` 与 `src/tool_call.rs` 必须已存在（F 已落地）；
不存在即**停下报协调者**，**不代 F 建**（见「跨计划前置」）。

- [ ] **Step 1: 写用例**

`tests/model_call.rs`（本 task 只有分类那部分，其余用例由后续 task 追加）：

- `each_provider_error_variant_maps_to_its_class`：**五个变体逐项**——
  `Transport` / `Unavailable` → `Some(Transient)`；`Protocol` / `UnknownModel` → `Some(Permanent)`；
  `Cancelled` → **`None`**。**逐项各写一次断言，不抽代表**（`ProviderError` 的五个变体是枚举式绝对断言，
  五个臂各要照片——手写分支能各自漂移）。
  **红的条件（档位：移除）**：删掉 `Protocol` 那一臂（或把它的像改成 `Transient`）→ 只有 `Protocol` 那一条红。
- `a_cancelled_call_is_not_a_failure`：**两侧对钉的具名用例**——同一次转换，
  `Cancelled` 得 `None`、`Transport` 得 `Some(..)`。
  **只钉一侧是 fail-open 的那一侧**：一个「什么都返回 `Some(Transient)`」的实现会让
  「四个失败臂各得类别」全绿，**只有这一条会红**。依据是设计 §6.1「它是调用方自己发的取消，不是失败」。
  **订正（2026-10-07，G Task 1 实测；原话照留）**：上面那句「**只有这一条会红**」**不成立**——
  只要用例一按本 brief 写了第五条（`Cancelled` → `None`），**用例一自己也会红**：
  M2（把 `Cancelled` 也归成 `Transient`）实测的 red **同时**落在
  `tests/model_call.rs:38`（用例一：`left: Some(Transient)` / `right: None`）
  与 `tests/model_call.rs:57`（用例二）。
  **本条真实的必要性**：它钉的是 **`Cancelled` 得 `None` 与 `Transport` 得 `Some(..)` 这一对镜像**
  ——M2 实测红在 `tests/model_call.rs:57`、M3（「一律返回 `None`」）实测红在 `tests/model_call.rs:61`，
  **两向互为镜像、各挡一侧**。**与用例一的分工**：用例一逐变体把每一枚 `ProviderError` 映射到的像钉死
  （枚举式绝对断言，五臂各一张照片）；本条则把**同一对入参上「取消」与「失败」的对照**钉死——
  用例一给的是「每一枚各自的像」，本条给的是「这两枚必须在同一次转换里分道」。
  **用例本身保留不动。**
  **红的条件（档位：放宽）**：把 `Cancelled` 也归成 `Transient` → 本条红（`tests/model_call.rs:57`）；
  用例一内**只有 `Cancelled` 那一臂**红、四条失败臂不红，且**用例三也红**（`tests/model_call.rs:141`）。
  **一条一般化判据**：**「必要性理由句写错」与「这条用例该不该留」是两件事**——
  理由句被实测证伪时，要订正的是理由句，不是删掉用例。
- `the_error_type_carries_the_provider_error_verbatim`：`ModelCallError::Provider { class, source }` 的
  `source` 是**给进去的那一枚**（逐变体各断一次：五个 `ProviderError` 各构一个 `ModelCallError`）。
  **`Cancelled` 走 `ModelCallError::Cancelled`，且那一枚不带 `class` 字段**。
  **红的条件（档位：取反）**：把 `Provider` 的两个字段写反（`class` / `source` 互换）→ 红。
  **订正（2026-10-07，G Task 1 实测；原话照留）**：上面这句红条件**不可实现**，有两种读法、**两种都不成立**：
  (a) 真去互换两个字段的值——`class: FailureClass` 与 `source: ProviderError` **不同型**，**写反根本编不过**，
  产出的不是红用例而是编译错误；(b) 读成「对调两个具名字段的**声明次序**」——那是**等价变异体**
  （具名字段与次序无关），**照样全绿**，不红。
  **可表达的真变异体（各把红单独落在本条用例上，实测）**：
  - **M4（档位：取反）**：`classify` 的 `source` 处换成**一枚钉死的常数**（不再逐字回读入参）→
    本条红在 `tests/model_call.rs:84`（「`source` 该是给进去的那一枚，逐字回读」，`left: Unavailable("钉死的常数")`
    / `right: Transport("连接被重置")`），另两条用例**仍全绿**。
  - **M5（档位：放宽）**：把 `Cancelled` 那一支**改投 `Provider`** → 本条红在 `tests/model_call.rs:141`
    （「`Cancelled` 该走 `Cancelled` 那一枚，实际 `Provider { class: Unknown, source: Cancelled(..) }`」），
    另两条用例**仍全绿**。
  实测日志：`.tmp/mut-m4_source_not_verbatim.log`、`.tmp/mut-m5_cancelled_routed_to_provider.log`
  （汇总 `.tmp/mutations-summary.log`）。
  **一般化判据**：**「互换两字段」只在两个字段同型时才是可观察变异——而它看上去与真变异体一模一样。**
  写红条件前先问「这两枚值的类型相同吗」；不同型时的「互换」是编译错误，同型但具名时的「对调次序」是等价变异体，
  两者都长得像一条真变异体。
  **这条是「错误类型不压平」的照片**：设计 §6.1 明写 `Routing` 要**带出 D 的错**、`Provider` 要**带出类别**，
  两者都不是「一枚同名的变体」。

**「`Cancelled` 不带 `class`」这一条为什么值得单列**：它不是一个字段的取舍，
它是「一次正常中止不会被记成一次失败」这条判据**在类型上的落点**——若 `Cancelled` 也带 `class`，
调用方会在一个没有失败的地方读到 `FailureClass`。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

预期：**「名字未解析」这一类**编译错误（`ModelCallError`、`classify` 尚未存在）。
**订正（2026-10-07，原话照留）**：原句写死了 `E0433` / `E0425`，**两枚都无实跑记录**，
按本仓今天的统一口径收成**类别**——**具体码以实跑为准**。判据是那条：
**「预期报错码」必须来自实跑；`.stderr` 不会替你报错——码写错了它照样让用例绿。**

- [ ] **Step 3: 实现**

```rust
/// G 这一侧的失败面。**五枚逐条对应设计 §6.1 的表**。
pub enum ModelCallError {
    /// D 的排序失败。**带出 D 的错，不压平成一枚同名的变体**——那会是同一件事两个类型。
    /// 从 G 这条路径到得了的只有 `NoEligibleCandidate`，其余三枚的可达性逐条记在设计 §4.5。
    Routing(RoutingError),
    /// D 的表读失败（`list_registered` / `load_profile`）。
    Storage(PersistError),
    /// 这一次调用在截止内没有完成（设计 §3.5）。
    Deadline { elapsed_ms: u64 },
    /// 适配器调用失败，**带出它的类别**。
    Provider { class: FailureClass, source: ProviderError },
    /// 调用方发起的取消（来源是 `ProviderError::Cancelled`）。**它不是失败，故不带类别**。
    Cancelled { source: ProviderError },
}

/// 设计 §6.1 的分类表：**G 这一侧的默认口径，不是对适配器的断言**。
/// 适配器要对某次失败改口径，应当在 `ProviderError` 的取值上表达（明知永久就报 `Protocol`）。
pub fn classify(e: &ProviderError) -> Option<FailureClass>;

/// `ProviderError` → `ModelCallError` 的**唯一转换点**，`call` / `call_stream` / `abort` 共用它。
pub fn into_call_error(e: ProviderError) -> ModelCallError;
```

**三句话写进实现，不写成可选说明**：

- **分类表不写成边界上的函数**（C §5.1 拒绝过的那件事，理由在本路径上同样成立：
  `Transport` 在一个适配器上可能是瞬时的、在另一个上可能是永久的，故这张表**是否为真本子项目答不出**）。
- **已知射程边界，据实记**：既有夹具把**永久性配置缺陷**（「这个 provider 没有这个工具」）报成
  `Unavailable`（`crates/continuum-provider/tests/fake_provider.rs:106`，C §5.1 记录了它）——
  **订正（2026-10-07，G Task 1 实测；原引坐标照留）**：上面这处 `fake_provider.rs:106` **已不成立**——
  工具侧夹具在该文件之后被搬走，`fake_provider.rs` 现存 **104 行**、只剩 `FakeConnector`（`:13`／`:16`），
  行 `:106` 根本不存在。**实读的新坐标**（p3g 树，commit `404c464`）：
  **`crates/continuum-provider/tests/common/mod.rs:177`**（`FakeTool::describe_tool` 里的
  `.ok_or_else(|| ProviderError::Unavailable(..))`；该文件 `:120-122` 的文档注释记了这条来由）。
  若模型侧的真实适配器也这样用，那张表会把一次配置缺陷分成 `Transient`。**修它在适配器，不在这里。**
- **`FailureClass::Resource` 这一格无输入**：`ProviderError` 没有「限流 / 配额耗尽」的变体
  （C §12 第 7 条）。**G 不擅自扩 `continuum-core` 的取值域**——那是 §315 的接口类型。
  这句话写进 `classify` 的文档注释，**并且不写一张 `Resource` 的臂**（一个没有输入的臂是假接口）。

**四个纯数据类型的处置（Step 3 之后、本 task 不建它们，此处只记判据）**：`Candidate` / `CallPlan` /
`RouteInput` / `CallInput` 都是**纯数据声明/纯投影**，从构造到读回之间**没有本 crate 的代码**，
故任何运行期断言必然恒真（照 D 的 `BudgetView` 与 `router.rs` 的同一裁决）。
它们的照片形态是**编译期**：`tests/model_call_face.rs` 里以**全字段字面量**构造/解构，
**判据是编译通过本身**，外加各字段的类型后缀写全（D 的 Task 9 的实测：整数字面量随字段类型推断，
不写全后缀时改类型**照样编译**）。**本计划不为这四个类型写一条运行期断言。**

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/src/model_call.rs crates/continuum-runtime/src/error.rs \
        crates/continuum-runtime/src/lib.rs crates/continuum-runtime/tests/model_call.rs
git commit -m "feat(runtime): G 的错误类型与模型侧失败分类"
```

---

### Task 2: 假模型适配器夹具

**Files:**
- Modify: `crates/continuum-runtime/Cargo.toml`（dev-dependencies）
- Create: `crates/continuum-runtime/tests/common/mod.rs`
- Modify: `crates/continuum-runtime/tests/model_call.rs`（加 `mod common;` 并写夹具自身的用例）

**Interfaces:**
- Consumes: **既有的** `continuum_provider::ModelProvider` 与 `continuum_core::model::*`
- Produces: `common::{FakeModel, OnceStream}`（**只在测试目标内可见**）

> **这是 C 的 `crates/continuum-provider/tests/common/mod.rs` 里那个 `FakeModel` 的第二份副本，
> 两份不合并**（`docs/superpowers/p3bcdf-followups.md` §七.8：「同一约定的两份副本，**不合并**，两份各有其用」）。
> **判据**：集成测试各是一个独立 crate，`continuum-runtime` 的测试**引用不到** `continuum-provider` 的
> `tests/` 下的任何东西（那不是它导出的面）；把夹具提取成一个共享 crate 会造出一条为测试而生的依赖边，
> 而那条边要进 `ALLOWED`——**代价大于收益**。**这条取舍记在 `tests/common/mod.rs` 的文件头。**

- [ ] **Step 1: 登记 dev 依赖**

`crates/continuum-runtime/Cargo.toml` 的 `[dev-dependencies]` 加：

- `async-trait`：`ModelProvider` 是 `#[async_trait]` 的 trait，夹具实现它必须同法标注
  （`continuum-provider` 自己的 `tests/fake_provider.rs` 就是这么写的——**本处仍成立**
  （2026-10-07 实读，p3g 树 `404c464`）：该文件仍在，且仍在 `:15-16` 以 `#[async_trait]`
  实现 `Connector`（`FakeConnector`，`:13`），这处引据不受同批坐标搬迁影响）。**F 的 Task 2 也会加它**——
  两处是同一条边的两个使用者，**谁先落地谁登记**，后到的那个核一遍即可（不是重复登记）。
- `futures-core`：手写单分片流要它的 `Stream` trait 与 `Pin`。
  **不用 `futures-util`**（`stream::iter` 在它里面，本仓不用它，判据同
  `crates/continuum-provider/tests/fake_provider.rs:17-19` 的注释）。
  **订正（2026-10-07，G Task 1 实测；原引坐标照留）**：原引的 `fake_provider.rs:17-19` **已不成立**
  （该区间现存的是 `FakeConnector::descriptor` 的函数体，不是注释）。那条「`futures-core` 只给 trait、
  `stream::iter` 属 `futures-util`」的注释随 `OnceStream` 一起搬进了
  **`crates/continuum-provider/tests/common/mod.rs:31-32`**（实读，p3g 树 `404c464`）。

**两者都是外部 crate，`ALLOWED` 不动。** `dev-dependencies` 的边**也在** `ALLOWED` 的覆盖范围内
（`every_crate_depends_only_on_its_allowed_set` 跑 `cargo tree --edges all`），但那张表只断言
**workspace 成员之间**的边，故外部 crate 不进表——**这一点要实测确认**（跑一次
`cargo test --workspace --no-fail-fast` 看 `dependency_direction` 是否照绿），不据口径断言。

- [ ] **Step 2: 写用例**

`tests/model_call.rs` 里加三条**夹具自身的**用例（**先钉机制再看结论**——一条恒不生效的夹具会让
后续每一个用例都假绿）：

- `the_fake_adapter_reports_the_health_it_was_configured_with`：配 `Unavailable` 的假适配器
  `health()` 返回 `Unavailable`；**两侧对钉**：改配 `Healthy` → 返回 `Healthy`。
  **红的条件（档位：取反）**：夹具把配置项读反（或忽略它、永远返回 `Healthy`）→ 向一红。
- `the_fake_adapter_counts_every_method_call`：`invoke` / `stream` / `cancel` / `usage` / `list_models` /
  `describe_model` 各调一次，**六个计数器逐个**断言为 1。
  这是后续「不调 `usage()`」「不调 `list_models()`」「探活三次」三类否定式照片的共用仪表。
  **红的条件（档位：移除）**：删掉某一个计数器 → 对应的那一条红。
- `the_recording_policy_captures_the_five_budget_dimensions`：记录用的 `RankingPolicy` 把
  `&RoutingRequest` 的**五个 `Option<i64>`** 抄进自己的记录（`Option<i64>` 是 `Copy`，
  **故这条照片不需要 `BudgetView` 派生 `Clone` 或 `PartialEq`**，设计 §8.2）。本 task 只钉
  「抄得下来」（对一份手工构造的 `RoutingRequest` 断五个值），两向对钉在 Task 7。
  **红的条件（档位：移除）**：漏抄一维 → 该维断言红。

- [ ] **Step 3: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 4: 实现夹具**

`FakeModel` 的**可配置面**（逐项都要有，后续 task 依赖它们）：
`health()` 的返回值；`invoke` / `stream` 的返回值或错误；`list_models()` 的返回值或错误（**含一个
不在 D 的表里的 id**）；`describe_model()` 与 `usage()` 的错误；**永不返回的 `invoke`**；
**「略早于截止返回」的 `invoke`**；以及 `invoke` / `stream` / `cancel` / `usage` / `list_models` /
`describe_model` 的调用计数器与 `cancel` 收到的 `CallId` 记录。

**单分片流**照 `crates/continuum-provider/tests/fake_provider.rs:19-29` 的 `OnceStream` 形状
（`futures-core` 只给 trait，`stream::iter` 属 `futures-util`）。
**订正（2026-10-07，G Task 1 实测；原引坐标照留）**：原引的 `fake_provider.rs:19-29` **已不成立**——
`fake_provider.rs` 现存 104 行且**不含 `OnceStream`**（`grep -n OnceStream` 零命中）。实读的新坐标
（p3g 树 `404c464`）：**`crates/continuum-provider/tests/common/mod.rs:33-41`**（`OnceStream` 的
定义与其 `Stream` impl；`:35` 为 `impl Stream for OnceStream`）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/Cargo.toml crates/continuum-runtime/tests/common/mod.rs \
        crates/continuum-runtime/tests/model_call.rs Cargo.lock
git commit -m "test(runtime): 假模型适配器夹具（C 侧那份的第二份副本）"
```

---

### Task 3: 可用性快照（适配器级 → 模型级）— **不依赖 C 与 D**

**Files:**
- Modify: `crates/continuum-runtime/src/model_call.rs`
- Modify: `crates/continuum-runtime/tests/model_call.rs`

**Interfaces:**
- Consumes: Task 2 的 `FakeModel`；**既有的** `continuum_core::model::{ModelId, ProviderHealth}`、
  `continuum_model_registry::RoutableModel`
- Produces: `continuum_runtime::model_call::Candidate`、`continuum_runtime::model_call::snapshot`

**本 task 与 Task 6 的关系**：`Candidate` 在这里先落地（它只装**已交付**的 `RoutableModel` 与
`Arc<dyn ModelProvider>`，**两者今天都存在**），故快照这一半在 C/D 落地之前就能写完并测完。

- [ ] **Step 1: 写用例**

- `one_probe_per_candidate`：三个候选 → 假适配器的 `health()` 被问**三次**（计数器夹具）。
  **钉「不跳着问」**。**红的条件（档位：移除）**：改成「同一适配器只问一次、结果复用」→ 计数为 1，红。
  **这一条钉的是本设计写死的粒度**：快照是**逐候选**取的，不是逐适配器（设计 §3.3 只规定
  「对每个候选调一次它所对应的适配器的 `health()`」）——**它不是一条性能选择，是 §4.4 第 1 条
  「`availability` 必须覆盖候选集」的构造性保证**（逐候选取 ⇒ 天然全覆盖）。
- `the_snapshot_is_indexed_by_the_candidates_own_model_id`：三个候选，各自的适配器配**不同的**
  `ProviderHealth`；断言 `(id, health)` 的**逐项对应**（三个 `(ModelId, ProviderHealth)` 各断一次）。
  **红的条件（档位：取反）**：把 `(id, health)` 写反、或按候选次序错位配到另一个 id 上 → 红。
- `the_snapshot_is_as_long_as_the_candidate_set`：断言 `len()` 相等**且 id 集合相等**（不是只断 `len`）。
  这是设计 §4.5 里 `UnknownAvailability` 在 G 路径上**不可达**的那条构造性断言的一半（另一半在 Task 11）。
  **红的条件（档位：收紧）**：在快照里加一条「只对 `Healthy` 的候选给条目」的过滤 → id 集合不等，红。
- `a_unavailable_adapter_is_carried_through_verbatim`：配 `Unavailable` 的适配器，**快照里仍是 `Unavailable`**
  ——**G 不在这一层做任何二次裁剪**（设计 §4.4 第 2 条：只过滤 `Unavailable` 是 `rank` 的事，
  `Degraded` 与 `Healthy` 之间没有判据）。
  **两侧对钉**：改配 `Degraded` → 快照里是 `Degraded`（**不是被丢掉、也不是被升格成 `Healthy`**）。
  **红的条件（档位：收紧）**：在快照里就把 `Unavailable` 滤掉（丢掉该候选）→ 这条与上一条同时红。
  > **订正（2026-10-07，G Task 6 评审发现两处标签不自洽，协调者裁定）**：本行初稿标的是**「放宽」**。
  > **它与上一条（`the_snapshot_is_as_long_as_the_candidate_set`，标「收紧」）做的是同一类事**——
  > 两条的变异体都是「**加一条过滤、丢条目**」（一个是「只对 `Healthy` 的候选给条目」，
  > 一个是「把 `Unavailable` 滤掉」），**却一个标收紧一个标放宽**。
  > 按本仓口径（**加过滤 ＝ 收紧；去掉一个过滤的效果 ＝ 放宽**——后者的实例见 E 那边标「放宽」的
  > 「滤掉之后若为空就退回未过滤的集合」），**两枚都是「收紧」**。**故本行改为「收紧」。**

**一条刻意不写的用例**：探活**并发与否**。设计 §3.3 明写「是否并发探活**不影响可观察结果**」
（`rank` 只按 id 查条目，对次序无判据），故它是实现选择、**没有照片**——本计划不规定，
**也不为它写一条「必须是顺序」的断言**（那会把一个没有判据的实现选择钉死）。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 实现**

```rust
/// 一个候选：**D 的闸门产物**与它对应的**适配器句柄**，同一次定下。
/// 解析只发生一次（设计 §3.1 第 1 条）——若在发起时再解析一次，就是同一件事的第二个产生点。
pub struct Candidate {
    model: RoutableModel,
    adapter: Arc<dyn ModelProvider>,
}

impl Candidate {
    pub fn model(&self) -> &RoutableModel;
    pub fn adapter(&self) -> &Arc<dyn ModelProvider>;
}

/// 逐候选取一次可用性快照（设计 §3.3）。
/// **同一个适配器名下某个模型单独不可用这件事在 §315 的类型上不可表达**——故只能把它
/// 原样摊给该适配器服务的每个模型。这句话写进函数的文档注释（设计 §3.3 已记在 §14 第 2 条）。
pub async fn snapshot(candidates: &[Candidate]) -> Vec<(ModelId, ProviderHealth)>;
```

**`health()` 没有截止，本层不发明一条**（设计 §3.3）：一句话写进文档注释，并在 §14 第 3 条留收件人。
**本计划不为它加一个 `Option<Duration>` 参数**。

**`Candidate` 是纯数据 + 两条访问器**：字段私有（构造点唯一在 `plan_candidates` 里）、
**不派生任何东西**（判据同 D 的 `BudgetView`：没有当场消费方）。
`Candidate` 的构造只能发生在 crate 内，故它在集成测试里**由 `plan_candidates` 产出**——
**这意味着 Task 3 的快照用例要先用 Task 6 的 `plan_candidates` 造候选**。
**这是本计划的一处任务序倒置，如实写明**：Task 3 的**用例**在 Task 6 落地之前编不过（`Candidate` 造不出来）。
**处置**：Task 3 的 Step 1/2 与 Task 6 合并执行——**快照的实现与它的用例随 Task 6 一起落地**，
本 task 的位置保留在计划里是为了把「快照」与「候选集构造」两件事分开写清**各自的判据**。
（**若 Task 3 先做**，替代办法是给 `Candidate` 加一条 `#[cfg(test)]` 的 crate 内构造口——
**不取**：那会在测试与生产之间开第二条构造通道，而 `Candidate` 的字段私有本身是设计 §3.1 第 1 条的落点。）

- [ ] **Step 4: 运行全部测试并提交（与 Task 6 同批）**

见 Task 6 的 Step 5。

---

### Task 4: 模块面守卫三条（用词纪律 / 中立性 / 零能力与凭据引用）— **不依赖 C 与 D**

**Files:**
- Create: `crates/continuum-runtime/tests/model_call_discipline.rs`

**Interfaces:**
- Consumes: 无（只读 `crates/continuum-runtime/src/model_call.rs` 的源码文本）
- Produces: 无新公开面

> **这一组守卫挡的是设计 §1.3 与 §10.3 的三条判据**，而它们**没有任何运行期形态**：
> 混称是**文本**上的事，不是行为上的事。**它们各有明确的逃逸面，两条都要写明**——
> 匹配的是**字面拼法**，故 `use … as` 别名、全限定路径、`include!` 都逃逸；
> **订正（2026-10-08，G Task 4 实测：本行抄 C 的那三条逃逸面，对本 needle 集不成立）**：
> 本处五枚 needle 是**单 token 标识符**（`AuthorizedTool` / `AuthorizedToolInvocation` /
> `ToolProvider` / `invoke_tool` / `authorize`），**`use … as` 别名与全限定路径都必须把原拼法写出来
> ⇒ 它们命中，不是逃逸**。（那三条逃逸面出自 **C**，那边的 needle 是
> `impl ModelProvider for` 这种**多 token 短语**——别名与换序才会改变文本。**同一个词在两种 needle 下含义相反。**）
> **真正的逃逸面（据实测改写，不穷尽）**：**他处写出后经 `crate::` 拐弯引用**、**`include!`**、
> **宏展开**、**清单外的同义名字**。**这四条里只有前两条与新清单绑定，后两条是「下界」本身的限度。**
> **归一化之后它仍是一个下界，不是封闭判定**（与 C §4.1 末段的处置相同，上界要 `syn`，本阶段不做）。

- [ ] **Step 1: 写用例**

`tests/model_call_discipline.rs`：

- `the_guard_sees_the_module`（**守卫自身的正控制，先钉机制再看结论**）：读
  `crates/continuum-runtime/src/model_call.rs` 的全文，断言**非空且能找到 `pub`**（例如
  `pub fn plan_candidates` 或 `pub async fn call` 的拼法）。**没有这一条，一个「什么都没读到」的守卫
  会永远绿**——这正是「守卫须两侧都钉」里缺的那一侧。
- `the_guard_folds_whitespace_before_matching`（**归一化自身的照片，必须先有**）：对
  `AuthorizedTool  Invocation`（两个空格）、`AuthorizedTool\tInvocation`、`AuthorizedTool\nInvocation`
  三种输入，归一化函数都要折成 `AuthorizedTool Invocation`。**归一化写错了没人知道**，
  而它正是裁决 C7 要的那一件事（空白变体是同一个字面拼法）。
- `the_module_does_not_borrow_the_tool_paths_names`（设计 §1.3）：**折叠空白之后**的文本里
  **不出现**这五个标识符的**词边界**匹配：`AuthorizedTool`、`AuthorizedToolInvocation`、
  `ToolProvider`、`invoke_tool`、`authorize`。断言信息里**列出命中的行号**，否则红的时候读不出是哪一处。
  **红的条件（档位：取反）**：在 `model_call.rs` 里写一句「本模块不碰 `ToolProvider`」的注释即红——
  **这是刻意的**：守卫的判据是**拼法**，不是「用了它」；设计 §1.3 要的正是模块里不出现这个拼法。
- `the_module_names_no_adapter_implementation`（设计 §10.3）：同法，文本里**不出现**
  `continuum-adapter` / `continuum_adapter` / `DeepSeek` / `OpenAi` / `Anthropic`。
  **更硬的一层其实已经有了**（设计 §10.3）：若 G 真想持有实现类型，它必须依赖那个 crate，
  而 `every_crate_depends_only_on_its_allowed_set` 会因为 runtime 的条目里没有它而红
  （`crates/continuum-runtime/tests/dependency_direction.rs:246`，逐对 `assert_eq!` 在 `:276-281`）。
  **这一句写进文件头**：本守卫是下界，上界那条边由 `dependency_direction.rs` 兜住。
- `the_module_touches_neither_capability_nor_effects_nor_credentials`（设计 §2.2 第一条：
  **「它没有可越的权」**）：同法，文本里**不出现**这些 **crate 路径与类型名**：
  `continuum_capability` / `continuum-capability` / `continuum_effect` / `continuum-effect` /
  `continuum_secrets` / `continuum-secrets` / `AuthorizedEffect` / `EffectJournal` / `CapabilityKind` /
  `SecretsRuntime` / `SecretStore`。
  **取词判据（写进文件头，免得后来者改宽或改窄）**：取**crate 路径与类型名**，
  **不取裸词 `capability` / `effect` / `secret`**——那样一句中文注释里带一个英文词就会误伤，
  守卫会退化成「禁止谈论」而不是「禁止使用」，而**一个会误伤的守卫会被绕过或被删掉**。

  **三条判据写进文件头，缺一不可**：
  1. **它挡的是什么**：G 一旦引用了能力 / 效应 / 凭据的任何东西，它就有了「可越的权」这个问题的**物质基础**；
     这条守卫把「它没有可越的权」从一句话变成**一条会红的用例**。它同时是设计 §2.1 的三个强制点
     逐个核在 G 侧的落点。
  2. **`ALLOWED` 挡不住它，据实写明**：`continuum-runtime` 的条目**本来就含**这三个 crate
     （F 在用），故多引一个**不会**让任何依赖断言变红。**这条守卫是本判据唯一的结构性落点**。
  3. **它是下界**：匹配的是字面拼法。**逃逸面按上面的订正读**（别名与全限定路径**不**逃逸；
     真正的逃逸面是「他处写出后经 `crate::` 拐弯引用 / `include!` / 宏展开 / 清单外的同义名字」），**没有照片**。

  **红的条件（档位：取反）**：在 `model_call.rs` 里加一行 `use continuum_capability::AuthorizedTool;`
  即红。**实测这一步要真做一次**（照 C 的 Task 8 Step 2 的做法）：临时加进去、跑、**确认它红且报出行号**、
  再删掉，**日志路径留在报告里**——**「守卫恒绿」与「守卫有效」必须能被区分**。
  **这一次的临时内容故意用两个空格或一个 tab 的变体**，这样这一次红**同时**是空白归一化生效的照片。
  > **订正（2026-10-08，G Task 4 实测：本句为假）**：**M4 那枚变异体**（把 `normalize` 换成恒等函数）
  > 下，**三条判定逐条仍绿**，只有归一化自身的照片红。**成因**：**三张清单里没有一枚 needle 含内部空白**，
  > 而匹配用**词边界**——**折与不折，命中集完全相同**；上面那次临时内容红的只是「**拼法出现了**」。
  > **故归一化在本文件上不承重**：它留着只是**照 C 的形状**、**为将来 needle 改成多 token 短语预留**。
  > **这是本仓点过名的形状**：**一个机制有照片、但没有任何判定依赖它**——
  > **它的照片会绿，而它若坏了也照样绿**（除非那条照片自己的变异体来红）。**据实写明，不假装它在承重。**

  **反向的边界也要写**：这条守卫**不禁止注释里用中文说「能力」**（那不是一个标识符），
  也不禁止 `ModelCallError` 里出现 `FailureClass`（那是 `continuum-graph`，不在三个强制点的任何一条上）。

- [ ] **Step 2: 运行，确认通过（这是本 task 的正常态）**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call_discipline
```

- [ ] **Step 3: 做一次「守卫会红」的实测（内容不提交）**

见 Step 1 的 `the_module_touches_neither_capability_nor_effects_nor_credentials` 与
`the_module_does_not_borrow_the_tool_paths_names` 两条的红条件说明。**两次临时修改都要留独立日志路径**，
**改前改后各核一次 `sha256sum`**（纪律 2）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/tests/model_call_discipline.rs
git commit -m "test(runtime): 模型调用路径的三条模块面守卫"
```

---

### Task 5: 「模型调用没有任何强制点」的编译期照片 — **不依赖 C 与 D**

**Files:**
- Create: `crates/continuum-runtime/tests/model_call_face.rs`

**Interfaces:**
- Consumes: **既有的** `continuum_core::model::{InvokeRequest, Message, ModelId, Role}`、
  `continuum_provider::ModelProvider`
- Produces: 无新公开面

> **设计 §2.1 的三行是「没有强制点」这个命题的依据**（三个强制点逐个核，没有一个管模型）。
> 那三行今天**只有一条是有照片的**（强制点 (3)：§315 的请求面里没有承载凭据的位置）。
> 本 task 把能拍的两条拍成**编译期照片**，并把拍不出来的那条**明写为什么**。

- [ ] **Step 1: 写用例**

`tests/model_call_face.rs`（**全是「编译得过」形态的照片**，**没有 `assert`**——
判据是**编译通过本身**，与设计 §3.4 的 `assert_send` 同一形态）：

- `the_invoke_request_has_no_authorization_slot`：以**恰三个字段的全字段字面量**构造
  `InvokeRequest { model, messages, max_tokens }`（`model` 是 `ModelId`、`messages` 是
  `Vec<Message>`、`max_tokens` 是 `Option<u32>`，**三个字段名与类型都写全**）。
  **它钉的是设计 §2.1 强制点 (1) 那一格的原话：「模型侧的 `invoke` 收的是 `InvokeRequest`，
  里面没有任何授权位」。** 判据是**编译通过**；**给 `InvokeRequest` 加第四个字段即
  这个文件编译不过**（`E0063`，字段缺失）——**实测的错误码为准**。
  **为什么这个形态比 trybuild 好**：被钉的对象是**请求面本身的性质**（「放不下一个授权位」），
  写成一个能编译的字面量比写一份「这样写编译不过」的样例更直接，也不需要新增 `trybuild` dev 依赖。
- `the_model_provider_face_is_callable_in_seven_ways`：一个泛型函数
  `async fn probe<P: ModelProvider + ?Sized>(p: &P, id: &ModelId, req: InvokeRequest)`，
  函数体里**逐个**调用 `list_models` / `describe_model` / `invoke` / `stream` / `cancel` / `usage` / `health`
  并各绑一次返回值。**七个各写一次，不抽代表**——它钉的是「§315 的七项在模型侧都在，
  且 G 的发起面拿得到它们」。
  **它的证明力有边界，写明**：它钉的是**有这七个**，**不钉「没有第八个」**。
  「`ModelProvider` 上不存在一个 `authorize` 方法」这一侧**本阶段没有照片**——
  要它需要 trybuild 与一份样例，而 `ModelProvider` 是 **C 的 trait**（不是 G 的），
  它的面在 C 的计划里管；**本计划不为别人的 trait 新增一条 dev 依赖**。
- `the_async_segments_future_is_send`（设计 §3.4，**本 task 只放占位，实体在 Task 8**）：
  **本 task 不写它**（`select` / `call` 尚不存在），记此以免被当成漏项。
  > **订正（2026-10-08，G Task 5 实测：本行原写「实体在 Task 10」，与本计划自己相抵）**：
  > **Task 8 那一节的 `Files:` 就明写着**「Modify: `crates/continuum-runtime/tests/model_call_face.rs`
  > （**Task 5 的占位，此处补实体**）」，且 Task 8 的 Step 5 里就列着这条用例。
  > **Task 10 是另一件事**（`usage()` / `list_models()` / `describe_model()` 的否定式照片）。
  > **判据**：**一个交叉引用若与它所指那一节的 `Files:` / `Interfaces:` 相抵，以后者为准**——
  > **因为 `Files:` 是那一节的执行面，而交叉引用只是一句话**。
  > **这条我（协调者）也照错抄进了派单**——**交叉引用的错会被下游原样复制**。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call_face
```

预期：第一份**本来就该通过**（`InvokeRequest` 已存在）；第二份**因 `ModelProvider` 未导入 /
`#[async_trait]` 的关联函数用法写错而失败**，实现时按实跑修正。

**本 task 的「红」要这样做**（否则「恒绿」与「有效」无法区分）：
临时给 `InvokeRequest` 加第四个字段 `pub authorized: bool` 与相应构造改动（**不提交**），
跑本条，**确认 `the_invoke_request_has_no_authorization_slot` 编译不过**，
**读实测的错误码**记进报告，再还原（**`trap` + 前后 sha256**，纪律 2）。
**注意**：`InvokeRequest` 是 `continuum-core` 的类型，加字段会让**别的 crate** 一起红——
**判据只取 `tests/model_call_face.rs` 这一次编译失败**，其余的红是连带的，不算这条守卫的照片。

- [ ] **Step 3: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/tests/model_call_face.rs
git commit -m "test(runtime): 模型侧请求面没有授权位的编译期照片"
```

---

### Task 6: `plan_candidates`（同步段）— **硬依赖 C 的 `model_for` 与 D 的 `list_registered`**

**Files:**
- Modify: `crates/continuum-runtime/src/model_call.rs`
- Modify: `crates/continuum-runtime/tests/model_call.rs`（含 Task 3 的快照用例，见 Task 3 Step 3 的说明）

**Interfaces:**
- Consumes: Task 1 的 `ModelCallError`、Task 3 的 `Candidate` / `snapshot`；
  **C**：`continuum_provider::{ProviderRegistry, RegistryError}`、`ProviderRegistry::model_for(&ModelId)`；
  **D**：`continuum_model_registry::{list_registered, load_profile, RoutableModel, LifecycleState}`
- Produces: `continuum_runtime::model_call::plan_candidates`

**夹具的第一件事：裸 SQL 播 D 的画像行。** `ModelProfile` 在 crate 外**构造不出来**（字段私有、
`try_new` 是 `pub(crate)`），而 `save_profile` **收一枚 `ModelProfile`**——**这条链不能自举**。
故测试必须先用裸 SQL 播下 `model_registry` 与 `model_profile` 两行，再走类型化接口读回
（D 的 Task 11 的夹具段是同一形状，D 的计划明写「**第一枚画像必须由裸 SQL 播下**」）。
需要的迁移是 `builtin_migrations() + p1_graph_migrations() + p3d_model_migrations()`
（前者给 `events` / `audit_log`，第二个给 `node_attempt`，第三个给三张模型表）。

> **这一步设计里没有对应段落，而它是 G 的测试绕不开的**（设计 §11 只说要一个假适配器）。
> **它的代价据实记**：G 的测试里**嵌了 D 的表结构**（列名与 JSON 容器编码），
> 故 D 的表变一次，G 的夹具会**静默失配**（要么报 SQL 错、要么播进对的列而值是错的）。
> **收件人：D 的实现者 + G 的实现者**，记在 `## 遗留`。
> **替代方案都更坏**：给 `ModelProfile` 开一条 `#[cfg(test)]` 的构造口（**测试专用后门**，
> 正是设计 §2.1 那条保证要挡的东西）；或让 G 的测试**改在 `continuum-model-registry` 里**（放错了 crate）。

- [ ] **Step 1: 写用例**

- `a_model_with_a_profile_past_the_gate_and_an_adapter_is_a_candidate`：**正面照片**——
  登记 + 画像 + 迁到 `active` + 注册表里有适配器 → 候选集里**有它**。
  **只钉三个「不进」而不钉这一条时，一个「永远返回空向量」的实现全绿**（这正是 fail-open 的反面）。
  **红的条件（档位：取反）**：把 `try_new` 的状态参数换成常量 `Active`（**丢弃真实状态**）→ 这条**不**红，
  但下面那条 `a_stale_registered_model_is_not_a_candidate` 红（**两条配对才是完整的守卫**）。
- `each_of_the_three_conjuncts_keeps_a_model_out`：**三条合取逐项各一条用例，不抽代表**（设计 §3.2）：
  - `a_model_with_a_profile_past_the_gate_and_an_adapter_is_a_candidate`（合取成立）；
  - `a_model_without_a_profile_is_not_a_candidate`：登记了、没过画像 → 不进。
    **红的条件（档位：移除）**：把 `load_profile` 的 `Ok(None)` 那一臂从「丢弃」改成 `.expect("有画像")`
    → 夹具里那个无画像的模型让它 panic，而用例断言的是「它不在候选集里、其余候选仍在」→ 红。
  - `a_stale_registered_model_is_not_a_candidate`：画像齐全但状态是 `stale` → 不进。
    **红的条件（档位：取反）**：把传进 `RoutableModel::try_new` 的状态参数换成常量（如 `Active`）→ 红。
    **这一条与 D 的十态用例的分工写明**：十态**逐项**的照片在 D
    （`only_the_six_routable_states_pass_the_gate`）；G 只钉**与 §3.2 相异的那一格**（`stale`），
    **不重跑十态**（唯一入口原则，设计 §11 末的刻意不重复 (a)）。
  - `a_model_with_no_adapter_is_not_a_candidate`：登记 + 画像 + 过闸门，但注册表里没有它 → 不进。
    **红的条件（订正 2026-10-07，G Task 6 评审实测）**：
    - **本行初稿给的那一枚编不过**——「把 `Err(RegistryError::NotFound { .. })` 那一臂改成 `?`」
      报 **`error[E0277]`**：**`From<RegistryError>` 没有为 `ModelCallError` 实现**，故 `?` 用不了。
    - **改用「收紧」档**（计划原写「放宽」也是错的）：让那一臂**对「无适配器」直接返回一个 `Err`**
      ⇒ 用例断言的是「`Ok` 且候选集里没有它、其余仍在」→ 红。**实测红。**
    - **而「放宽」那一档是写得出来的，只是当时是等价变异体**：评审写的 `T64-relax`
      （**借用上一枚候选的句柄**）**编得过、全量门下 112 ok / 0 FAILED**——成因是
      `list_registered` 的 **`ORDER BY id ASC`** 让夹具里无适配器的 `m-lonely` 排在前面。
      **故「写不出来」与「写出来是等价的」是两条结论，不要混。**
- 「无适配器即不是候选」**两侧对钉**（设计 §3.2 的丢弃规则）：
  - 向一：只在 D 的表里、注册表里没有 → **不进候选集**；**若它是唯一候选 → 整批返回
    `Err(ModelCallError::Routing(RoutingError::NoEligibleCandidate))`**（断言是哪一枚）。
  - 向二：**同一条 `registry.register_model` 之后它回到候选集**。**只钉向一会让一个「永远返回空」的实现全绿**。
  - **丢弃是显式的、不是静默的**：向二那条用例就是它的照片（丢弃之后登记回来 = 它真的回来了，
    而不是「一直都不在」）。
- `the_candidate_set_comes_only_from_the_registry_table`（设计 §4.5 的「逐行来自主键表」）：
  假适配器的 `list_models()` 返回一个**不在 D 的表里**的 id；断言**候选集的 id 集合 ⊆
  `list_registered(tx)` 的 id 集合**，且**候选集 id 两两不同**（用计数断言，不用「看起来没有」）。
  **这一条同时是 `DuplicateModelCandidate` 在 G 路径上不可达的照片**（`id` 是 `model_registry` 的主键列）。
  **红的条件（订正 2026-10-07，G Task 6 评审实测；本行初稿给的那一枚编不过）**：
  - **本行初稿**：「把候选集的 id 来源从 `list_registered` 换成适配器的 `list_models()`」
    ⇒ **编不过**：`error[E0728]: await is only allowed inside async functions and blocks`
    （`ModelProvider::list_models` 是 **async**，而 `plan_candidates` 是**同步**的）。
  - **本条的两条结构断言里，「两两不同」那一半写得出来**：评审写的 `T66-distinctness`
    （**把 `list_registered` 的结果复制一份**）**在全量门下红 7 条**，**含本条**。
    **「`⊆` 那一半没有可写的变异体」才是真的**——实读：`ModelProfile::try_new` 是 `pub(crate)`、
    `model_profile.id REFERENCES model_registry(id)`、`PRAGMA foreign_keys=ON`、注册表无枚举入口，
    **故表外的 id 进不来**（**这是推理，不是穷举**，评审已据实这样标注）。
  - **另据实记一条覆盖缺口（评审 I1）**：**本条在「候选集恒空」的实现下整条绿**——
    三条断言在空集上**全空真**，`.expect` 拿到的是 `Ok(vec![])`。**修复轮已给它加非空锚。**
- `an_empty_candidate_set_is_no_eligible_candidate`：**两个来路各一条**——
  (a) 一个适配器都没登记（或一个模型都没登记）；(b) 全部被闸门挡下。
  **各断言是哪一枚 `Err`**（`ModelCallError::Routing(RoutingError::NoEligibleCandidate)`），
  **且断言 `rank` 一次都没被调用**（用一个会 panic 的 `RankingPolicy` 作证：它若被调到，用例以 panic 失败）。
  **后一半（「`rank` 一次都没被调用」）在本 task 造不出来（订正 2026-10-07，G Task 6 评审实测）**：
  **`plan_candidates` 的参数表里没有 `&dyn RankingPolicy`**（实读 `src/model_call.rs` 的签名）——
  **这是类型上做不到，不是有规则在挡**。**故本行初稿称它是「本条唯一的承重部分」是派错了地方。**
  > **协调者裁定（2026-10-07）：这一半落到 Task 11**（标题正是「**可达性与射程边界**」，
  > 且它硬依赖 Task 7，是本计划里唯一能端到端看到 `rank` 被不被调的地方）。
  > **Task 7 不做**：`select` 永远收不到空 `Vec`——空集在 `plan_candidates` 就短路了。
  > **全计划里这一句只在本处出现过一次**，没有第二落点——**这正是它被派错却没被发现的原因。**
  **红的那一半本 task 仍留**：**红的条件（档位：移除）**：把空判定的短路删掉（照常往下走）→ 红。
- `a_storage_failure_is_reported_as_storage_and_leaves_nothing_behind`：**失败路径**——
  在**一个不带 `p3d_model_migrations()` 的库**上跑 `plan_candidates`（`list_registered` 必然抛真实的库错误，
  造法照 `crates/continuum-workspace/src/gate.rs:2514-2536` 的三行：`Db::open_with(&path, Vec::new())`）。
  断言 `matches!(err, ModelCallError::Storage(_))`，**并断言内层是 `PersistError`**（两层都断）。
  **半写副作用的断言**：该库上没有任何表，故「没有半写」这条由 Task 11 的行数不变用例承重（此处只钉错误面）。

**一条刻意不写的用例：三个判定的次序。** 设计未给判据（合取的三条谁先谁后），
而**次序在一处是可观察的**：一个画像列有表外取值的登记项，若先查 `model_for` 并命中失败，
它会被丢弃而不报 `Storage`；反过来会报 `Storage`。**本计划取一个次序并写死，
但所有用例都只让一个条件成立**——**不构造两条判定同时可能的输入**，故换一个次序不会让哪条用例变绿或变红。
**判据同 D 的 Task 11 的同一处置：次序没有判据，用例就不该依赖它。**

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 实现**

```rust
/// ① 同步段：读 D 的登记表、过 D 的闸门、解析适配器。**收 `Tx`**——
/// 它是 G 全模块**唯一**收 `Tx` 的入口（设计 §3.4）。
///
/// 候选集**只从 D 的 `model_registry` 出发**（设计 §3.2）：登记的每一行，三件事的合取——
/// (a) 在表里（由 `list_registered` 给）；(b) `load_profile` 有画像且 `RoutableModel::try_new` 过闸门；
/// (c) `registry.model_for(id)` 命中。**任一不成立即丢弃这一条，不报错。**
///
/// **G 不自己再判一次闸门**（(b) 由 `try_new` 判）、**G 不调 `list_models()`**
/// （登记是路由的权威，`list_*` 是描述的权威）。
///
/// 候选集为空 → `Err(ModelCallError::Routing(RoutingError::NoEligibleCandidate))`，
/// **不调 `rank`**（设计 §3.1 第 5 条）。
pub fn plan_candidates(tx: &Tx<'_>, registry: &ProviderRegistry)
    -> Result<Vec<Candidate>, ModelCallError>;
```

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/src/model_call.rs crates/continuum-runtime/tests/model_call.rs
git commit -m "feat(runtime): 候选集的构造（同步段）"
```

---

### Task 7: `select`（组装请求、调 `rank`、成对带出句柄）— **硬依赖 D 的 Task 11**

**Files:**
- Modify: `crates/continuum-runtime/src/model_call.rs`
- Modify: `crates/continuum-runtime/tests/model_call.rs`

**Interfaces:**
- Consumes: Task 3 的 `snapshot`、Task 6 的 `Candidate`；
  **D**：`continuum_model_registry::{rank, RankedExecutionCandidates, ExecutionCandidate, RankingPolicy,
  RoutingRequest, TaskSkillRequirement, FamilyPreference, BudgetView, RoutingError}`
- Produces: `continuum_runtime::model_call::{RouteInput, CallPlan, select}`

**本 task 的两处形状**（`select` 按值收 `Vec<Candidate>`、`CallPlan`）**已回写设计 §3.1**（2026-10-06）：
设计作者据本计划的报出把四处口径写回正文——所以**本 task 与设计现在是一套形状**，
下面是当时的判据（作来历照留）：

1. **`select` 收 `Vec<Candidate>`（按值），不是 `&[Candidate]`。** 判据：`rank` 收 `&[RoutableModel]`，
   而 `RoutableModel` **不可克隆**、`Candidate` **按值持有它**——`&[Candidate]` 变不出 `&[RoutableModel]`。
   设计 §3.1 第 2 条的正文只要求「**收的是候选切片，不是库句柄**」，按值仍是候选切片，不违正文。
2. **新增 `CallPlan`**：设计 §3.1 的代码块在 ②③ 之间**把适配器句柄丢了**
   （②返回 D 的 `RankedExecutionCandidates`、③只收 `&ExecutionCandidate`，而句柄无处安放），
   而正文第 1 条（「候选集与适配器句柄**同一次**定下」）与 §3.5（`cancel(&stream.call)` 要句柄）
   都要求它在这里。`CallPlan` 就是把「D 的排序结果」与「每个候选的句柄」成对带出的那一枚值。
   **回写后同一条判据也产生了设计 §3.1 的正文第 6 条**：配对键是 `ModelId`、它的唯一性有两个来源
   （D 的 `model_registry.id` 主键 + D 在 `rank` 内的判重）——**这一条是初稿一个字都没写的那一半。**

- [ ] **Step 1: 写用例**

- `the_request_reaching_the_policy_carries_the_candidates_availability`：交给 `rank` 的
  `availability` 的 id 集合 **== 候选集的 id 集合**（**集合相等，不是 `len` 相等**）。
  这是设计 §4.5 里 `UnknownAvailability` 在 G 路径上**不可达**的那条构造性断言
  （另一半在 Task 3 的 `the_snapshot_is_as_long_as_the_candidate_set`）。
  **红的条件（档位：收紧）**：在组装请求时只把 `Healthy` 的条目放进 `availability` → **本条红**。
  > **订正（2026-10-08，G Task 7 评审实测：本行初稿把机制写错了）**。初稿写「→ **集合不等，红**」，
  > **实测不是**：那一枚（记作 **M1a**）红在 **`select(...).expect(...)`** 上——两个 panic 点
  > （`tests/model_call.rs` 的 `.expect`）报 `Routing(UnknownAvailability { … })`，
  > **而那条集合相等断言根本没被求值**。成因：**D 在 `rank` 里对 `UnknownAvailability` 是 fail-closed**
  > （D 设计 §5.3），故请求里的条目先少了一枚、`rank` 就先返回 `Err` 了。
  > **要红在集合断言上，得另做一枚隔离版（M1b）**：**多塞一条表外条目**——
  > 它让 `rank` 收下请求，于是红**恰好落在**那条集合相等 `assert_eq!` 上
  > （消息逐字：`候选 {"m-a","m-b","m-c"}，请求里 {"ghost-not-a-candidate",…}`）。
  > **故本行钉的东西由 `M1b` 提供，不由 `M1a`**——**「别处先红」不等于「这条断言有照片」**。
  > **判据（本轮又一次付代价的）**：**报「某枚变异体让某条断言红」之前，看红的**位置**是不是那条断言**。
- `a_probed_health_actually_reaches_the_ranking`（设计 §11）：假适配器配 `Unavailable` →
  该模型**不被选中**；**两侧对钉**：改回 `Healthy` → 它**回到输出**（且成为 `selected()`）。
  **只钉向一时，一个「永远返回空候选集」的实现全绿**。
  **红的条件（档位：移除）**：把 `availability` 一律填 `Healthy`（不取快照）→ 向一红。
- `the_budget_reaches_the_policy_verbatim_both_ways`（设计 §8.2，**两向对钉**）：
  记录用的 `RankingPolicy` 抄下收到的五个 `Option<i64>`——
  **向一**：传 `budget.money = None` → 请求里 `money` **仍是 `None`**；
  **向二**：传 `budget.token = Some(0)` → 它**仍是 `Some(0)`**。
  **这两向为什么都要钉**：fail-open 的那一侧是**向二的反面**——把 `Some(0)` 读成 `None`
  就是「把额度为零读成不构成约束」，路由会在额度耗尽时照常花钱，而它在结果上**看不出来**；
  反向的 `None → Some(0)` 是 fail-closed（凭空没有候选），错得刺眼、容易发现。
  **红的条件**：向一（档位：**放宽**）把 `None` 折成 `Some(0)` → 向一红；
  向二（档位：**收紧**）把 `Some(0)` 当「没有约束」折成 `None` → 向二红。
  **两条各自独立**：只写向一时，第二档变异不红。
- `the_result_pairs_every_candidate_with_its_own_adapter`：`CallPlan::selected()` 给出的
  `(ExecutionCandidate, Arc<dyn ModelProvider>)` **是同一个模型的那一对**（三个候选、三个不同适配器，
  逐项断言 `execution_candidate.model() == 该适配器服务的那一个 id`）。
  **红的条件（档位：取反）**：把 `adapters` 的次序与 `candidates` 错位 → 红。
  **这条是本计划新增类型的唯一承重用例**：没有它，一个「句柄随便给一个」的实现
  （按 id 发起调用时会打到**另一个模型**）会全绿——而那是这条路径上最坏的一种错。
- `alternatives_is_the_tail_of_the_same_list`：`CallPlan::alternatives()` 的 id 序列
  **== `ranked.alternatives()` 的 id 序列**（**不重新排序、不重新解析**——G 不做第二份判断）。
  **红的条件（档位：放宽）**：在 G 里对 alternatives 再排一次序 → 红（当输入次序与 D 的输出次序不同时）。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 实现**

```rust
/// 输入的三个值**来自别处，G 不产它们**（设计 §3.1）：`requirements` / `family` 来自规划侧，
/// `budget` 来自驱动从语义层 `Budget` 的投影（D §6.1）。**这三个值今天的生产方都不存在**
/// （设计 §12 第 1 条）——故所有用例的输入都由测试直接构造，钉不了「驱动真的这么传」。
/// **按值收**：`RoutingRequest` 持有这三者，而 `TaskSkillRequirement` 没有 `Clone`。
pub struct RouteInput {
    pub requirements: TaskSkillRequirement,
    pub family: FamilyPreference,
    pub budget: BudgetView,
}

/// D 的排序结果与**每个候选自己的适配器句柄**成对带出（见本节开头的自定说明 2）。
pub struct CallPlan { /* ranked + adapters，字段私有 */ }

impl CallPlan {
    /// §84 的 `selected_model` 与**它对应的**句柄。
    pub fn selected(&self) -> (&ExecutionCandidate, &Arc<dyn ModelProvider>);
    /// §84 的 `alternatives` 与各自的句柄，**次序与 D 的输出相同**。
    pub fn alternatives(&self) -> Vec<(&ExecutionCandidate, &Arc<dyn ModelProvider>)>;
}

/// ② 异步段：取可用性快照 → 组装 `RoutingRequest` → 调 `rank`。**不收 `Tx`**（设计 §3.4）。
pub async fn select(candidates: Vec<Candidate>, input: RouteInput,
                    policy: &(dyn RankingPolicy + Sync))
    -> Result<CallPlan, ModelCallError>;
```

> **订正（2026-10-09，G Task 9 评审查出本块一直没跟上）**：本块的参数原写 `&dyn RankingPolicy`（无界），
> **而那条 future 必须是 `Send`**（设计 §3.4／§11）——`select` 在 `snapshot(..).await` **之后**才用 `policy`，
> `&T: Send` 要 `T: Sync`，而 D 的 `RankingPolicy` 没有超界，**故原写法编不过**（Task 8 实测 `E0277`）。
> 设计与实现都已改成 `+ Sync`（**不是 `+ Send + Sync`**：评审实测 `+ Sync` 单独就够，
> `+ Send` 多余——`&(dyn RankingPolicy + Send)` 单独反而不够）。**本块是最后一处没改的**。

**G 在这一段里不做任何二次裁剪**（设计 §4.4 第 2 条）：`Degraded` / `Healthy` 都照原样送进
`availability`，**不因健康度做任何判断**——那会是在 D 已写死的地方加第二个判据。
**这句话写进 `select` 的文档注释**，它的照片是 Task 3 的 `a_unavailable_adapter_is_carried_through_verbatim`。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/src/model_call.rs crates/continuum-runtime/tests/model_call.rs
git commit -m "feat(runtime): 调 rank 并成对带出适配器句柄"
```

---

### Task 8: `call`（发起一次调用与截止）— **硬依赖 D 的 Task 11**

**Files:**
- Modify: `crates/continuum-runtime/src/model_call.rs`
- Modify: `Cargo.toml`（workspace：`tokio` 的 features 加 `"time"`）
- Modify: `crates/continuum-runtime/tests/model_call.rs`
- Modify: `crates/continuum-runtime/tests/model_call_face.rs`（Task 5 的占位，此处补实体）
  > **本 task 顺带要订正的三处（2026-10-08，G Task 5 评审列出，在此托管）**：
  > 1. **该文件里对**本计划**的行号引用全部漂移了**——它们在 `2c35f8e` 时实测全对，
  >    而**协调者的 `608327e` 往本计划插进一段订正后，五处引用一起漂**。
  >    **判据**：**一个文件引用另一个文件的行号时，后者的编辑者不会知道**——
  >    故要么按内容引，要么**在下一次改动那个文件时重取一遍**（本行就是「下一次」）。
  > 2. 该文件 `:88` 引「计划 `:405-409`」，而它要引的那句（D 的 Task 9）**实测在 `:410-411`**。
  > 3. 该文件文件头 `:7` 写「本文件的**三条**用例」，**实际只有 2 条 `#[test]`**（第三条是编译期形态、不是 `#[test]`）。

**Interfaces:**
- Consumes: Task 7 的 `CallPlan`；Task 1 的 `into_call_error`；
  **既有的** `continuum_core::model::{InvokeRequest, InvokeResponse, Message}`
- Produces: `continuum_runtime::model_call::{CallInput, call}`

**本 task 是 `tokio` 的 `time` feature 的第一个使用者，故由它登记那条 workspace 改动**
（Global Constraints 的口径：边由用它的那个 task 增量加）。

- [ ] **Step 1: 写用例**

- `a_successful_call_hands_the_candidate_s_model_id_to_the_adapter`：断言适配器收到的
  `InvokeRequest.model` **正是该候选的 id**（不是别的候选的），`messages` 与 `max_tokens` 也逐项相符。
  **红的条件（档位：取反）**：把 `model` 取自别处（如固定一个字面量）→ 红。
  **这条同时是「`call` 收 `&ExecutionCandidate` 而不收 `ModelId`」那条判据的正面照片**：
  id **只有一处来源**——候选自己。
- `each_provider_failure_keeps_its_class_and_its_source`：假适配器分**四次**返回
  `Transport` / `Unavailable` / `Protocol` / `UnknownModel`，逐次断言
  `ModelCallError::Provider { class, source }` 的**两半**（`class` 是哪一枚、`source` 仍是给的那一枚）。
  **红的条件（档位：取反）**：把某一臂的 `class` 写反（`Protocol → Transient`）→ 只有那一条红。
- `a_cancelled_provider_error_is_not_a_failure`：适配器返回 `ProviderError::Cancelled` →
  `ModelCallError::Cancelled { source }`，**且**（编译期事实）那一枚**没有 `class` 字段**。
  **红的条件（档位：放宽）**：把 `Cancelled` 归进 `Provider { class: Transient, .. }` → 红。
- `a_call_that_never_returns_hits_the_deadline`（设计 §3.5，**两侧对钉**）：
  - **向一**：假适配器的 `invoke` **永不返回** → 带 `Some(短截止)` 的调用
    （**建议 50–200ms 的真时间**，见下）返回 `ModelCallError::Deadline { .. }`；
  - **向二**：同一个适配器改成「**略早于截止返回**」→ **成功**（钉住截止不会无故触发）。
  **只钉向一时，一个「永远返回 `Deadline`」的实现全绿。**
  **红的条件**：向一（档位：**移除**）去掉 `timeout`（直接 await）→ 用例挂住并超时（**那不是干净的红**，
  故这条要配合 `timeout` 命令跑）；向二（档位：**收紧**）把截止的判定写得过紧（如无脑超时）→ 向二红。
  **`Deadline { elapsed_ms }` 的数值不断言**：设计 §3.1 末段把口径写死为**实测耗时**、
  且写明它**今天没有消费方**，故断一个数值就是钉一次巧合（实测耗时在调度抖动下不等于截止值）
  （见 `## 遗留`）。
  **不用 `tokio::time::pause()` / `advance()`**：那两个要 `tokio` 的 `test-util` feature，
  而**多开一个 feature 只为让一条用例跑得快**不值当；真时间 + 短截止已经够稳
  （两个方向都不依赖真实时钟的精度，只依赖「50ms 内不返回」与「50ms 内返回」这两件事）。
  **这是一处实现选择，不是判据**——若实现时发现它不稳，改用 `test-util` 并记在报告里。
- `the_deadline_wraps_one_call_only`：同一个 `CallInput` 下，**两次**连续的 `call` 各带同一个短截止，
  两次**各自**都能完成（断言两次都 `Ok`）。**它钉的是「截止不是被消费一次的共享值」**——
  设计 §3.5 的「截止只包住一次调用」。
  **红的条件（档位：取反）**：把 `deadline` 做成一次性的（如 `Option<Duration>` 被 move 进第一次调用后
  第二次恒超时）→ 第二次红。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 登记 `tokio` 的 `time` feature**

`Cargo.toml`（workspace）的 `tokio` 行加 `"time"`。**实测两件事并记进报告**：
(a) `cargo test --workspace --no-fail-fast` 照绿、0 warning；
(b) **`Cargo.lock` 的包集合有没有变**（设计 §3.5 明写「由实现时实测，本设计不预判」——
**实测的结论要写进提交信息或报告**，不能只说「应该没变」）。

- [ ] **Step 4: 实现**

```rust
/// 发起一次调用。
///
/// **第二个参数是那枚值本身，不是它的类型名**（见 Task 7 的自定说明 2）：设计 §3.1 的代码块
/// 只写了 `candidate: &ExecutionCandidate`，而句柄在 ②③ 之间被丢了——`ModelStream` 也**不带句柄**
/// （`crates/continuum-core/src/model.rs:93-96` 只有 `call: CallId` 与 `chunks`），
/// 故发起与中止两侧都必须另收一枚。
/// **本计划据此在 `call` / `call_stream` / `abort` 的参数表里各加一枚 `&Arc<dyn ModelProvider>`。**
///
/// **不收 `ModelId`**（设计 §2.2）：id 只有一处来源——`candidate.model()`。
pub async fn call(
    candidate: &ExecutionCandidate,
    adapter: &Arc<dyn ModelProvider>,
    input: CallInput<'_>,
    deadline: Option<Duration>,
) -> Result<InvokeResponse, ModelCallError>;

/// 发起一次调用要装的东西。`messages` / `max_tokens` 就是 `InvokeRequest` 的另两个字段
/// （`model` 不在这里，它来自候选）。
pub struct CallInput<'a> {
    pub messages: &'a [Message],
    pub max_tokens: Option<u32>,
}
```

**截止的来源是值，不是 `ExecutionProfile` 本体**（设计 §3.5）：`ExecutionProfile.timeout_ms`
今天**零生产构造点**，故 G 收一个 `Option<Duration>`，由驱动从那个字段投影过来
——**与 D 收 `ProviderHealth`、收 `BudgetView` 的形状相同**。这句话写进 `call` 的文档注释。

**`deadline` 是 `Copy` 的 `Option<Duration>`**，故「只包住一次调用」是它的自然语义
（`the_deadline_wraps_one_call_only` 钉的就是这一点不被实现破坏）。

- [ ] **Step 5: 补 Task 5 的占位：异步段的 future 是 `Send`**

`tests/model_call_face.rs` 加：

- `the_async_segments_future_is_send`（设计 §3.4）：`fn assert_send<T: Send>(_: &T) {}` 之后
  `assert_send(&select(...)); assert_send(&call(...)); assert_send(&call_stream(...));`，再 `.await` 它们。
  **若实现把 `Tx` 带进了异步段，这条用例编译不过**。
  **为什么用「编译得过」而不是 trybuild**：trybuild 钉的是「这样写编译不过」，
  而这里要钉的是「**这个签名的 future 是 `Send`**」——被钉的对象是签名本身的性质。
  **它的证明力有边界，写在文件头**：它只覆盖被断言的这三个 future，
  **不证明 G 的每一处异步代码都不持 `Tx`**；后者由「异步段的参数表里没有 `Tx`」这条构造性事实兜住
  （`plan_candidates` 是唯一的 `Tx` 收口）。
  **红的条件（档位：取反）**：给 `select` 加一个 `tx: &Tx<'_>` 参数（并让它跨 `await` 活着）→
  这条用例**编译不过**。**注意「编译不过不算变红」**——这一条是**刻意的例外并要写明**：
  本判据的**红形态就是编译失败**（与 P3A 用 trybuild 钉「写不出来」同类），
  因为被钉的对象是类型层的性质、不是运行期行为。

- [ ] **Step 6: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add Cargo.toml Cargo.lock crates/continuum-runtime/src/model_call.rs \
        crates/continuum-runtime/tests/model_call.rs crates/continuum-runtime/tests/model_call_face.rs
git commit -m "feat(runtime): 发起模型调用与截止"
```

---

### Task 9: `call_stream` 与中止入口 — **硬依赖 D 的 Task 11**

**Files:**
- Modify: `crates/continuum-runtime/src/model_call.rs`
- Modify: `crates/continuum-runtime/tests/model_call.rs`
- Modify: `crates/continuum-runtime/tests/model_call_face.rs`
  （**订正 2026-10-08，G Task 8 评审发现：本行原缺，而缺了它会静默丢一件事**）
  > **交回本 task 的一行**：Task 8 那条 `assert_send` 用例今天只断言了**两枚 future**
  > （`select` 与 `call`）——**`call_stream` 是第三枚，它今天还不存在**。
  > **故 Task 9 要往「同一个 probe 函数」里加一行**，别另起一个。
  > **交接现在存在于两处，别让它们只活在别处**：该文件头的第四节
  > （`tests/model_call_face.rs` 的「一条据实记的边界：`call_stream` 今天不在这条用例里」）
  > 与 `docs/superpowers/p3g-followups.md` 的 **M-8-1**。
  > **判据**：**跨 task 的交接若只写在被交出去那个文件里，派单时就会丢**——
  > **派 Task 9 的人的 brief 是按本 task 一节切的，切不到别的文件里去。**
  > **故本 task 的 `git add` 也要跟着加这个文件**（见本节 Step 末的提交命令）。
  > **同一族另有一例**，派 Task 11 时要先想清楚：本计划 Task 11 的
  > `the_path_writes_nothing_to_the_database` 红条件写「在 `call` 里插 `tx.execute(...)`」，
  > **而交付的 `call` 没有 `Tx`**——**那条变异也只能以改签名表达**（与 Task 8 的
  > `the_deadline_wraps_one_call_only` 同形：**唯一可观察者是调用侧编译失败，而按纪律那不算红**）。

**Interfaces:**
- Consumes: Task 8 的 `CallInput`；**既有的** `continuum_core::model::{ModelStream, CallId}`
- Produces: `continuum_runtime::model_call::{call_stream, abort}`

- [ ] **Step 1: 写用例**

- `a_stream_call_hands_the_candidate_s_model_id_to_the_adapter`：与 Task 8 那条同形，
  钉 `stream` 收到的 `InvokeRequest.model` 是该候选的 id。
  **红的条件（档位：取反）**：同 Task 8。
- `the_stream_carries_the_call_id_the_adapter_produced`：返回的 `ModelStream.call`
  **就是适配器在那一枚流里给的那个 `CallId`**（G 不新造、不改写它）。
  **红的条件（档位：取反）**：G 自己 `CallId::new(...)` 造一个 → 红。
  **这一条是 `abort` 那条用例的前提**：若 `call` 返回的 `CallId` 与适配器记的不是同一个，
  「`abort` 真的取消了那一条流」这件事就无从谈起。
- `dropping_a_stream_does_not_cancel`（设计 §3.5，**否定式照片，两侧对钉的向一**）：
  发起一次流式调用、**丢弃** `ModelStream`、再断言假适配器记录的 `cancel` **为空**。
  **红的条件（档位：放宽；**订正 2026-10-09，G Task 9 评审逐条实测**）**：
  **本行初稿写「在 `ModelStream` 的 `Drop` 里调 `cancel` → 红」，而那枚变异体在本 crate 里写不出来**——
  `ModelStream` 是 **C 的类型**，`impl Drop for ModelStream` 实测报 **`E0117`（孤儿规则）＋ `E0120`**；
  绕开它要 G 自己包一层 `Stream` 实现，**而 `Stream` 是 `futures_core` 的 trait、在本 crate 里只挂 dev 依赖**
  （lib 里实现不了）。**故取它的可写邻形**：「**在 `call_stream` 里建立流之后顺手取消一次**」→ **红三条**
  （本条 ＋ 两条 `abort` 计数条；**注意**：多列 `a_stream_call_...` 的 `stream_calls() == 1` 是错的，
  那一条**要让 `stream` 被调两次**才红）。**用例本身照留**——它是那张否定式照片的落点。
  **为什么这一向必须单独钉**：「取消经 `CallId` 走 `cancel()`，**不经流的 drop**」是
  `ModelStream` 的文档已经写死的契约（`crates/continuum-core/src/model.rs:89-92`）。
  一个「drop 即取消」的实现在**适配器侧什么也没做**（drop 不产生任何远端动作），
  而它会让**向二**照样绿——**只钉向二等于没钉**。
- `aborting_a_stream_calls_cancel_with_the_streams_own_call_id`（**向二**）：
  经 G 的中止入口 `abort(&adapter, &stream)` → 断言
  (a) 适配器的 `cancel` **被调用了一次**，(b) 收到的 `CallId` **== `stream.call`**。
  **三个断言缺一不可**（「被调用」「恰好一次」「是那一个」）——
  只断「被调用」时，一个 `cancel(&CallId::new(""))` 的实现会绿。
  **红的条件**：向二（档位：**取反**）把 `abort` 里的 `CallId` 换成一个新造的 → (b) 红。
- `aborting_a_finished_stream_is_not_an_error`：适配器对 `cancel` 返回 `Ok(())`
  （「这个 `CallId` 已完成」不算错），`abort` 返回 `Ok(())`。
  **设计 §3.5：取消的语义是幂等、尽力而为**——**一次取消与一次完成天然竞态**，
  故 G 的中止入口不把「这个 `CallId` 已完成」判成错误。
  **红的条件（档位：收紧；**订正 2026-10-09，G Task 9 评审实测**）**：
  **本行初稿写「对『已完成』的 `CallId` 直接返回一个 `Err`」，而那在 G 手里表达不出来**——
  G **没有任何「这条流已完成」的读数**：`ModelStream` 只有 `call` 与 `chunks`（`chunks` 要 `Pin<&mut>` 才推得动，
  而 `abort` 收的是 `&ModelStream`），`cancel` 的返回值也不带状态。
  **故只能写成「在 `Ok` 路径上返 `Err`」**，**实测红两条**（本条 ＋ `aborting_a_stream_calls_cancel_with_the_streams_own_call_id`）。
  **据实记它的强度**：它是「**G 不在 `Ok` 路径上自行合成 `Err`**」的**回归护栏**，
  **不是能把「已完成／未完成」分开的判别式**（**但它不是空转用例**——那一枚挡得住）。
  **注意这一条钉的是 G 侧的判定，不是适配器的行为**：假适配器返回 `Ok`，
  而 G **不得**在它返回 `Ok` 之后自行合成一个 `Err`。
- `each_provider_failure_on_abort_keeps_its_class`：`cancel` 返回 `Transport` → `abort` 返回
  `ModelCallError::Provider { class: Transient, source }`（**走同一个 `into_call_error`**）。
  **判据**：同一批失败两个转换点，正是本项目一贯判为缺陷的形状。
  **红的条件（档位：放宽）**：在 `abort` 里另写一套转换（如一律 `Transient`）→ 红。

**「流式调用没有用量读数」这条没有用例，写明为什么**：`StreamChunk` 只有 `delta` 与 `done`
（`crates/continuum-core/src/model.rs:64-67`），**没有 usage**，故一次流式调用的用量
**在 §315 的类型上拿不到**（设计 §8.3）。**G 不编一个用量、也不在 `done` 那一帧假造一个**——
这条的处置是**不做**，它的收件人在设计 §14 第 8 条。**本计划不为它写一条「读不到 usage」的断言**：
那种断言钉的是 `StreamChunk` 的字段数（**C 的类型**），不是 G 的行为。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 实现**

```rust
/// ④ 流式的那一支，与 `call` 同形；**截止只包住「建立这一次调用」，不包住流的消费**（设计 §3.5）：
/// 一个长回答本来就要跑很久，分片的节奏由适配器决定——把截止套在分片消费上，
/// 会把「回答长」误判成「调用失败」。
pub async fn call_stream(
    candidate: &ExecutionCandidate,
    adapter: &Arc<dyn ModelProvider>,
    input: CallInput<'_>,
    deadline: Option<Duration>,
) -> Result<ModelStream, ModelCallError>;

/// 中止一条流。**G 的中止入口**——`ModelStream` 自己不带适配器句柄，故它另收一枚。
/// 语义是**幂等、尽力而为**（C §5.3 定案）：不把「这个 `CallId` 已完成」判成错误。
/// **参数序与 `call` / `call_stream` 一致**：句柄在前、被作用的那个值在后
/// （设计 §3.1 的代码块即是此序；本计划初稿写成 `(stream, adapter)`，**已按设计改齐**，2026-10-06）。
pub async fn abort(adapter: &Arc<dyn ModelProvider>, stream: &ModelStream)
    -> Result<(), ModelCallError>;
```

**一处机制边界（据实记，无照片）**：截止只到「一次调用的建立」。
**一个既不来分片、也不结束的流会把 G 挂住**——要它可中止，须先有「流的分片消费也带截止」的机构，
而规范没有给判据（设计 §14 第 4 条）。**这不是本设计没做够，是本阶段的机构边界**
（照 B 的 §11 第 18 条的同一写法）。这句话写进 `call_stream` 的文档注释。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/src/model_call.rs crates/continuum-runtime/tests/model_call.rs \
        crates/continuum-runtime/tests/model_call_face.rs
git commit -m "feat(runtime): 流式调用与中止入口"
```

---

### Task 10: 不调 `usage()` / `list_models()` / `describe_model()` 的否定式照片

**Files:**
- Modify: `crates/continuum-runtime/tests/model_call.rs`

**Interfaces:**
- Consumes: Task 2 的 `FakeModel`（三个方法可配成返回 `Err`）、Task 6/7/8/9 的四段流程
- Produces: 无新公开面（**纯测试**）

> **否定式照片在本仓是被接受的**（`docs/superpowers/p3bcdf-followups.md` §四.4：
> 「F 的 P-19 是断言行数为 0，B 的『不写审计』照片是表行数不变。**别再把否定命题当作『没有照片』而跳过**」）。
> 本 task 的三条钉的是**行为上真的没调**（调用计数为 0）。
>
> **订正（2026-10-09，G Task 10 实现者实测：本段原写的一句与交付物无对应物）**：
> 本段原先接着写「**本 task 的三条与 Task 4 的模块面守卫互补：Task 4 钉文本里不出现**」——
> **那句为假**：`crates/continuum-runtime/tests/model_call_discipline.rs` 的三张清单是
> `TOOL_PATH_NAMES`(5) / `ADAPTER_NAMES`(5) / `NO_POWER_NAMES`(11)，
> **`usage` / `list_models` / `describe_model` 各零命中**，设计 §11 也没有「文本里不出现这三个方法名」那一格。
> **故「Task 4 钉了那三个名字」不存在**，那句「互补」在交付物里**没有对应物**。
> **本 task 因此不为它写一条源文本守卫**（它在本 task 的 Files 之外、也无判据）。
> **判据（可搬用）**：**「与另一处的守卫互补」这句话，必须在那一处真的找得到那条守卫**——
> **否则它写的是一种关系，而关系的一头不存在。** 本段原句是**设计者凭印象写的**，
> 而它**读起来像已经核对过**。

- [ ] **Step 1: 写用例**

- `the_path_does_not_ask_for_usage`：假适配器的 `usage()` 返回
  `Err(ProviderError::Unavailable(…))`，而**正常路径照过**（成功路径 `Ok`），
  **并断言 `usage` 的调用计数为 0**。
  **判据写进用例注释**（设计 §5）：`usage()` 的语义两种读法都不成立
  （读作「累计」= 同一事实的第二个产生点，读作「本会话」= 「会话」在 §315 里没有定义），
  故 **G 不定义它、也不用它**。G 实际依赖的是**逐次**的那个——`InvokeResponse.usage`
  （与那次调用**同一物证**的量，不需要与任何「累计」对账）。
  **红的条件（档位：移除）**：在 `call` 里加一句 `let _ = adapter.usage().await;` → 计数变 1，红。
- `the_path_does_not_ask_the_adapter_what_models_it_serves`：假适配器的 `list_models()` 返回
  **一个不在 D 的表里的 id**；断言**候选集不变**（与 Task 6 的
  `the_candidate_set_comes_only_from_the_registry_table` 同一份夹具），**且 `list_models` 计数为 0**。
  **判据**（设计 §3.2）：登记是**路由**的权威、`list_*` 是**描述**的权威，G 要的是路由事实。
  **红的条件（档位：放宽）**：把候选集的 id 来源换成 `list_models()` → 计数变 1 且候选集多出那个 id，红。
- `the_path_does_not_ask_for_model_descriptions`：假适配器的 `describe_model()` 返回
  `Err(ProviderError::Unavailable(…))` → 正常路径照过，**计数为 0**。
  **判据**（设计 §4.3）：描述是 `ModelDescriptor`、可用性是 `ProviderHealth`，**两个方法、两个类型、两件事**
  ——G 用 `health()` 取快照，**不拿第 1 步的产物当可用性**。
  **红的条件（档位：放宽）**：把快照改成从 `describe_model` 的返回里推 → 计数变 1，红。
- `the_three_negative_methods_share_one_adapter_and_the_positive_path_still_works`：
  **三条共用一个假适配器**（三个方法**同时**配成返回 `Err`），跑通**一条成功路径与一条失败路径**，
  两条都**不受那三个 `Err` 影响**（成功的那条 `Ok`、失败的那条是**它自己那个**类别）。
  **红的条件（档位：取反）**：让三个 `Err` 中的任何一个污染路径（例如把 `describe_model` 的 `Err`
  当成 `health` 的读数）→ 那两条的红/绿状态改变即红。
  **这条是三条否定式照片的「正控制」**：没有它，一个「整个 `select` 永远返回 `Err`」的实现
  也会让上面三条全绿（它们只要求「照过」，而一个恒错的实现也「没调那三个方法」）。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/tests/model_call.rs
git commit -m "test(runtime): 不调 usage / list_models / describe_model 的否定式照片"
```

---

### Task 11: 不写库、可达性与射程边界 — **硬依赖 Task 6、Task 7**

**Files:**
- Modify: `crates/continuum-runtime/tests/model_call.rs`

**Interfaces:**
- Consumes: Task 6/7/8/9 的四段流程；**既有的** `continuum_persist::{Db, builtin_migrations}`、
  `continuum_graph::p1_graph_migrations`、`continuum_model_registry::p3d_model_migrations`
- Produces: 无新公开面（**纯测试**）

- [ ] **Step 1: 写用例**

- **`rank_is_never_called_when_no_candidate_survives`**（**订正 2026-10-07 新增，协调者裁定**）：
  **「`rank` 一次都没被调用」的照片落在这里**——它原在 Task 6 的
  `an_empty_candidate_set_is_no_eligible_candidate` 一条里，**而那里造不出来**
  （`plan_candidates` 的参数表没有 `&dyn RankingPolicy`；**类型上做不到，不是有规则在挡**）。
  本 task 是本计划里**唯一能端到端看到 `rank` 被不被调**的地方（硬依赖 Task 7），
  **且标题正是「可达性与射程边界」**。
  **构造**：用**一个会 panic 的 `RankingPolicy` 作证**——让它成为走到 `select` 时的实参，
  再让候选集**一个都不剩**（两个来路各一条：一个适配器都没登记；或全部被闸门挡下），
  于是流程在 `plan_candidates` 就短路、**根本走不到 `select`**。**它若被调到，用例以 panic 失败。**
  **红的条件（档位：移除）**：把 `plan_candidates` 的空判定短路删掉（让它照常返回空集、
  一路走到 `select`）→ 那条 panic 策略被调到，红。
  **这一条为什么必须存在**：只断 `Err` 的话，**一个「装作没候选、其实调了 rank」的实现会全绿**。
  **它与 Task 6 那一侧的分工写明**：Task 6 只留「报哪一枚 `Err`」，
  **而 Task 6 那条用例本身还有一个覆盖缺口**（评审 I1：**三条断言在空集上全空真**，
  一个「候选集恒空」的实现让它整条绿）——**那一侧由修复轮加非空锚**，与本条不是同一件事。
- `the_path_writes_nothing_to_the_database`（设计 §9，**否定式照片**）：
  跑通**一条成功路径与一条失败路径**，断言 `events` / `audit_log` / `node_attempt` /
  `model_registry` / `model_profile` 五张表的行数**逐表不变**。
  **这一条同时是「G 不写半写副作用」的照片**：G 没有可半写的副作用，而这条断言把它钉成事实而不是声明。
  **红的条件（档位：移除）**：在 `call` 里插一句 `tx.execute("INSERT INTO node_attempt …")`
  （或用 `Db::begin` 写一行）→ 行数变，红。
  **为什么五张表都要**（不是「审计与画像两张」）：设计 §9 说两处落点都空
  （`AuditKind` 的八个变体里没有「模型调用」；`execution_profile` 表没有那三列），
  而**「没有落点」的最佳证据是「所有可能被写到的表都没变」**——
  `events`（§318 的状态更新与事件同事务）与三张模型表（G 唯一读的东西）也在其中。
  `execution_profile` **不列**：那张表**没有任何读写函数**（设计 §7 事实 3），
  列进去是一条**永远为真**的断言。
- `the_manager_of_the_availability_is_one_to_one_with_the_candidates`（设计 §4.5 的
  `UnknownAvailability` 不可达）：**端到端**的对应——从 `plan_candidates` 到交进 `rank` 的
  `availability`，id 集合**双向相等**（不是包含）。
  **这一条与 Task 3、Task 7 的两条是同一条判据的三处观测点吗？不是**，写明分工：
  Task 3 钉**快照自身的长度与 id**、Task 7 钉**组装进请求之后仍相等**、本 task 钉
  **在一个「候选集非空但某条被丢掉」的真实夹具上**仍成立（前两条的夹具里候选集是全集）。
  **红的条件（档位：收紧）**：在丢弃分支里**忘了**把对应的快照条目也去掉（或反过来多留一条）→ 红。
  **若实现下来发现三处观测点的变异完全等价**（同一处代码、同一组入参），
  **据实合并成一条并记在报告里**——**不为了凑三处而留两条等价用例**。
- `a_routable_model_does_come_out_as_the_selected_candidate`：**正面照片**（设计 §11）——
  一个 `active` 的模型经 `plan_candidates` → `select` **确实成为 `CallPlan::selected()`**。
  **只钉「`stale` 被挡」而不钉这一侧时，整条路径可以在「永远返回 `NoEligibleCandidate`」的情况下全绿**
  ——而那正是 fail-open 的反面。
  **红的条件（档位：取反）**：让 `select` 恒返回 `NoEligibleCandidate` → 红。
- `the_degraded_candidate_stays_in_the_output_unchanged`：`Degraded` 的候选**仍在输出里**
  （设计 §4.4 第 2 条、§11）。
  **两侧对钉**：改回 `Healthy` → 输出**逐项相同**（两次的 `selected()` 与 `alternatives()` 的 id 序列相同）。
  **红的条件（档位：放宽）**：在 G 里对 `Degraded` 做降权或裁剪 → 两次输出不同，红。
  **`Degraded` 该不该降权规范未给判据**（那是 `RankingPolicy` 的事，D 的基线不读它），
  故 G 这一侧的证据只能是「原样带过」——见 `## 遗留` 的收件人。
- **`NotRoutable` 不可达的构造性事实**：候选集的元素类型是 `RoutableModel`
  （字段私有、唯一构造点在 `RoutableModel::try_new` 内），故 G 的候选集里**不可能**有未过闸门的模型，
  `rank` 也就拿不到 `NotRoutable`。
  **本 task 不写一条「构造不出未过闸门的候选」的用例**——**判据**：G 侧**没有**第二条构造通道，
  故这条是「编译期事实 + D 的 trybuild 样例」，它的照片在 D（`tests/compile_fail/routable_model_fields_are_private.rs`）。
  **G 只钉与 §3.2 相异的那一格**（Task 6 的 `a_stale_registered_model_is_not_a_candidate`）。

- [ ] **Step 2: 运行，确认失败**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test model_call
```

- [ ] **Step 3: 运行全部测试并提交**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-runtime/tests/model_call.rs
git commit -m "test(runtime): 不写库、可达性与射程边界"
```

---

### Task 12: 收尾与复核

**Files:**
- Modify: `crates/continuum-runtime/**`（仅在复核发现缺口时）

- [ ] **Step 1: 全量验证**

```bash
cd /home/DslsDZC/Continuum && TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
TMPDIR="$PWD/.tmp" timeout 600 cargo tree -p continuum-runtime --depth 1 --edges all --prefix none
```

预期：全绿、0 warning；`cargo tree` 的**直接边**与开工前**逐项相同**
（本计划不加任何 workspace 成员之间的边）。**实测对照，不据口径断言**。

- [ ] **Step 2: 复核「G 不加任何边」（本计划唯一的 `ALLOWED` 判据）**

```bash
cd /home/DslsDZC/Continuum && git diff --stat -- crates/continuum-runtime/tests/dependency_direction.rs
TMPDIR="$PWD/.tmp" timeout 300 cargo test -p continuum-runtime --test dependency_direction
```

预期：`git diff` **无输出**（该文件一个字都没改），用例绿。
**若该文件被动过，据实报告并回退**——设计 §10.1 第 2 条明写 G **不新增任何依赖边**、
`ALLOWED` 一字不改。

- [ ] **Step 3: 通读复核「这条路径上没有强制点」的三条落点**

```bash
cd /home/DslsDZC/Continuum && grep -rn "AuthorizedTool\|ToolProvider\|invoke_tool\|authorize" crates/continuum-runtime/src/model_call.rs
grep -rn "continuum_capability\|continuum_effect\|continuum_secrets\|AuthorizedEffect\|EffectJournal" crates/continuum-runtime/src/model_call.rs
grep -rn "ModelId" crates/continuum-runtime/src/model_call.rs
```

前两条**预期零命中**（Task 4 的三条守卫已经在钉它们，此处是复核）。
第三条**预期只出现在这几处**：`Candidate` / `snapshot` / `CallPlan` 的**类型位置**（`(ModelId, ProviderHealth)`、
`&ModelId` 参数）——**发起面 `call` / `call_stream` / `abort` 的参数表里不出现它**。

**这两条是「按词根取候选、再人工读」的判据，不是「命中数即结论」的判据**：grep 按**行**匹配，
故**跨行折行会漏**（`ModelId` 与 `candidate` 被折到两行时，第三条的读数就是错的）。
**故命中数不是充分证据**——本轮判据的实际做法是：grep 出候选行后，**对 `src/model_call.rs` 通读一遍**
（本模块只有一个文件，通读代价很小），**结论以通读为准、grep 只用来定位**
（同一通病见 D 的 Task 14 Step 3 与 D 的 Task 7 的连带面判据）。

- [ ] **Step 4: 逐条核对完成判据（找不到证据的不得标注为覆盖）**

| 判据 | 证据 |
|---|---|
| §3.2 候选集的三条合取 | Task 6 的四条（正面 + 无画像 + `stale` + 无适配器） |
| §3.2 「无适配器即不是候选」两侧 | Task 6 的向一（不进 + 唯一候选→`NoEligibleCandidate`）与向二（登记后回来） |
| §3.2 G 不调 `list_models()` | Task 10 的第二条（**否定式**，计数为 0） |
| §3.3 快照逐候选、原样摊给每个模型 | Task 3 四条（含 `Unavailable` 原样带过） |
| §3.4 `Tx` 不跨 `await` | Task 8 Step 5 的 `assert_send`（**编译期**，红形态就是编译失败） |
| §3.5 截止两侧 + 只包住一次调用 | Task 8 的 `a_call_that_never_returns_hits_the_deadline` 两向 + `the_deadline_wraps_one_call_only`。
  **订正（2026-10-09，G Task 9 评审实测）：本格原先读起来像「§3.5 的截止面已覆盖」，而 `call_stream` 那一侧是零覆盖**——
  **把 `Some(limit)` 那整臂删掉，35 条全绿**（三个调用点全传 `None`；`StreamOutcome` 也没有 `Never`／`ReplyAfter` 一类的可配面）。
  **故本格只覆盖 `call` 那一侧。** `call_stream` 的截止**记在 `docs/superpowers/p3g-followups.md` 的 M-9-7**（具名缺口，未指派），
  **本格不得被读成「两侧都覆盖了」** |
| §3.5 截止（`call_stream` 侧） | **无照片**——见上一格的订正与本计划台账 M-9-7。**本行是「找不到证据的不得标注为覆盖」的一个实例** |
| §3.5 「丢弃流不是取消」两侧 | Task 9 的 `dropping_a_stream_does_not_cancel` 与 `aborting_a_stream_calls_cancel_with_the_streams_own_call_id` |
| §4.4 第 1 条 `availability` 覆盖候选集 | Task 3 + Task 7 + Task 11 的三处（**若等价则据实合并**） |
| §4.5 `NotRoutable` / `DuplicateModelCandidate` / `UnknownAvailability` 不可达 | 前者**构造性**（D 的样例，G 不重钉）；后者 Task 6 的 `the_candidate_set_comes_only_from_the_registry_table`；第三 Task 11 的一一对应 |
| §5 G 不调 `usage()` | Task 10 的第一条（**否定式**） |
| §6.1 五变体分类表 | Task 1 的四条（逐臂）+ Task 8 的四条（分类落到错误类型上） |
| §8.2 `None` ≠ `Some(0)` 两向 | Task 7 的 `the_budget_reaches_the_policy_verbatim_both_ways` |
| §9 记录面两处都空 | Task 11 的五张表行数不变（**否定式**） |
| §1.3 用词纪律 | Task 4 的第三条守卫（**下界**，逃逸面写在文件头） |
| §10.3 中立性 | Task 4 的第四条守卫（**下界**）+ `every_crate_depends_only_on_its_allowed_set`（**上界在那一侧**） |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 5: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。**本计划不新建、也不修改第二份文档**：
若要并进 `docs/superpowers/p3bcdf-followups.md`，由**协调者**做。

- [ ] **Step 6: 提交**

```bash
cd /home/DslsDZC/Continuum && git add <本 task 改动的显式路径>
git commit -m "docs(runtime): P3 子项目 G 的收尾与复核"
```

---

## 遗留

### 一、本计划自定的四处形状——**已了断（2026-10-06）：四处全部「回写设计」**

**定性**（逐条按「设计没写清 → 回写设计」还是「为计划好写而改口径 → 收回」判）：

- **三处是「设计没写清/写错」，本计划的补齐方向对，故回写设计**：
  `select` 按值收（初稿的 `&[Candidate]` 是一条**写不出来的签名**）、`CallPlan`（初稿的代码块**漏了句柄**，
  且正文第 1 条要求它在）、`call`/`call_stream`/`abort` 各带一枚句柄（同上）、`RouteInput` 去掉空生命周期
  （初稿的 `<'a>` 在正文里没有对应物）。
- **没有一处是「为计划好写而改口径」**：四处都不改任何**判据**，只补设计的空白；
  其中 `Vec<Candidate>` / `CallPlan` 两处的判据（`RoutableModel` 不可克隆、`ModelStream` 不带句柄）
  是**类型上的事实**，不是口径偏好。
- **回写后设计多出一条正文**（本计划的报出逼出来的）：§3.1 的**第 6 条**——配对键是 `ModelId`、
  它的唯一性有两个来源。**初稿一个字都没写这一半**，而「句柄与候选不会错配」这句话的依据就是它。

**设计侧的落点**：`docs/superpowers/specs/2026-10-06-p3g-model-calling-path-design.md` §3.1
（代码块 + 正文第 3、4、6、7 条 + `Candidate` / `CallPlan` 两枚随行类型的声明 + `Deadline` 口径）。
**本计划的 task 正文与它现已一致，只差一处签名**：`abort` 的参数序本计划初稿写成
`(stream, adapter)`、与设计 §3.1 的 `(adapter, stream)` **相反**，**已按设计改齐**
（Task 9 的签名与用例两处，2026-10-06）。
**订正**：本行初稿写的是「**无需改动 Task 6–9 的签名**」——**那句为假**：
**只要参数序不一致，就是签名不一致**（序是签名的一部分）。其余四处形状无需改动 Task 6–9。

```
（以下四段是**当时的判据**，作来历照留；设计作者已据此把口径写回设计正文。）

select 收 Vec<Candidate>           设计 §3.1 的代码块写 `&[Candidate]`。**判据**：`rank` 收
而非 &[Candidate]                  `&[RoutableModel]`，而 `RoutableModel` 不可克隆、`Candidate`
                                  按值持有它——`&[Candidate]` 变不出 `&[RoutableModel]`。
                                  设计 §3.1 第 2 条的**正文**只要求「收的是候选切片，不是库句柄」，
                                  按值仍是候选切片，不违正文。**→ 已回写设计 §3.1 第 3 条。**

新增 CallPlan                       设计 §3.1 的代码块在 ②③ 之间**把适配器句柄丢了**（②返回
                                  `RankedExecutionCandidates`、③只收 `&ExecutionCandidate`），
                                  而正文第 1 条（「候选集与适配器句柄**同一次**定下」）与 §3.5
                                  （`cancel(&stream.call)` 要句柄）都要求它在那里。
                                  `CallPlan` 是把「D 的排序结果」与「每个候选的句柄」成对带出的那一枚值。
                                  **→ 已回写设计 §3.1（随行类型的声明与第 6 条）。**

call / call_stream / abort          同上：三个签名各加一枚 `&Arc<dyn ModelProvider>`。
各加一枚适配器句柄                   判据两条：(a) 设计 §2.2 与 §3.5 要求句柄在这里而代码块没有它；
                                  (b) `ModelStream` **自己不带句柄**（`crates/continuum-core/src/
                                  model.rs:93-96`），故中止入口非收不可。
                                  **→ 已回写设计 §3.1 第 4 条。**

RouteInput 没有生命周期参数         设计 §3.1 写 `RouteInput<'_>`，而正文里没有对应物：
                                  `RoutingRequest` **持有**三者（按值），且 `TaskSkillRequirement`
                                  **没有 `Clone`**，故收引用就要一次克隆、而克隆不可得。
                                  本计划按值收，去掉那个空的 `<'a>`。
                                  **→ 已回写设计 §3.1 第 7 条。**
```

### 二、本计划查出的三条缺口——**已了断（2026-10-06）：三条全部回写设计**

```
G 的测试必须裸 SQL 播 D 的画像行    **→ 已回写设计 §11 前置二**：设计明写「**接受这条耦合，不向 D 要
                                  播种入口**」，并写清「接受的是什么」（D 的列名与 JSON 容器编码）、
                                  「它是有意的耦合 + 护栏」、「射程」（**加「可空或带默认值」的列不会红**，
                                  只有改掉 G 依赖的那些列名/编码、以及**新增一列必填的列**才会红，
                                  因为 G 按列名写的 `INSERT` 会缺一个值），
                                  三条替代为何更坏，收件人是谁。
                                  **本计划的那句「D 的表变一次，G 的夹具会静默失配」要按射程收窄**：
                                  改列名/编码时它**不静默**（夹具会红）；只有「加可空/带默认值的列」时不红，
                                  而那种加列无害。
                                  **（订正，2026-10-06 复核指出）**：本行上一版写的是「加列不会红」/「加列无害」
                                  ——**射程宽了一格**：新增一列**必填且无默认值**的列时，G 的 `INSERT`
                                  照样红。决定红不红的是**那一列要不要值**，不是「按不按列名写」。
                                  **收件人：D 的实现者（表结构变更须通知）＋ G 的实现者**（设计 §14 第 18 条）。

ModelCallError 的落点依赖 F          **→ 已回写设计 §10.2 的订正段**：设计把那句「F 已放好了」标为假
                                  （原话照留），写明 **G 的实际前置是 C、D、F 三个流**、
                                  **缺的那一步是「F 的 Task 2 建出 `error.rs` 并搬入 `TaskError`」**、
                                  以及 G 为什么不代 F 建（同文件两个创建者）。设计 §14 第 17 条列了它。
                                  **收件人：协调者（排期：G 的 Task 1 排在 F 的 Task 2 之后）。**

Deadline 的 elapsed_ms 口径           **→ 已回写设计 §3.1 末段**：口径写死为**实测耗时**（不是截止值），
                                  判据是「字段名与语义必须同宽」（写成截止值会让 `elapsed_ms` 这个名字
                                  说谎，而名字与语义两说是本项目反复出错的形状）。设计并写明
                                  **它今天没有消费方**，故用例只断言是哪一枚 `Err`、不断言数值
                                  （设计 §14 第 16 条）。**本计划 Step 1 的取法（不断言数值）不变**，
                                  但**不再声称「取截止值」**——那是本计划当时的取法，与设计现口径相抵，
                                  以设计为准。
```

### 三、相机拍不到的照片（设计 §12 已列 8 条，本计划补 2 条；**逐条写明为什么没有**）

- **设计 §12 的 8 条全部沿用**，不重述：端到端（`TaskSkillRequirement` / `FamilyPreference` /
  `BudgetView` 三个输入今天的生产方都不存在）、「闸门真的挡住了生产路径」、
  `FailureClass` 驱动了重试、升级/降级的实际触发、超时对真实适配器的行为、探活与调用的真实时长、
  流式的用量、多模型/多适配器的真实形态（本仓**零个真实模型适配器**）。
- **本计划补的一条：「G 的公开面放不下一个裸 `ModelId`」没有 G 侧的照片。**
  设计 §2.2 把这条写成 G 的公开面判据（两个发起入口都收 `&ExecutionCandidate`），
  而它的照片**在 D 侧**——`ExecutionCandidate` 的字段私有、唯一构造点在 `rank` 内
  （D 的 `tests/compile_fail/ranked_candidates_cannot_be_built.rs`）。**G 不重复钉它**（唯一入口原则）。
  G 侧能给的只有**签名本身（评审读）**与 Task 8 的
  `a_successful_call_hands_the_candidate_s_model_id_to_the_adapter`（钉 id **只有一处来源**）。
  **为什么不为它加一条 trybuild**：要它得给 `continuum-runtime` 新增一个 `trybuild` dev 依赖，
  而那份样例钉的是一个**别人的类型**的性质——**代价与收益不成比例**，本阶段不做。
- **本计划补的第二条：「模型调用没有任何强制点」的三个强制点里，只有 (1) 的一格有编译期照片。**
  Task 5 钉住的是「`InvokeRequest` 放不下一个授权位」（强制点 (1) 在模型侧的落点）。
  强制点 (2)（`AuthorizedEffect`）与 (3)（逐枚签发的凭据）在 G 侧**没有照片，也不可能有**：
  G 既不做副作用、也不取凭据，故**没有可观察的行为面**——它们的证据是
  **Task 4 的第四条模块面守卫（零引用）**，而那是一条**下界**。

### 四、设计 §14 的 15 条：本计划一条都不接，收件人照原文

**逐条抄收件人（哪些需要 G 做什么，也已逐条写明）**：

- **第 1、2、3、4、5、6、8、9 条**——收件人分别是**规范维护者**（第 1、4、5 条）、
  **C 的设计 + 规范维护者**（第 2、3、9 条）、**D 的设计 + 规范维护者**（第 6 条）、
  **语义层设计 + 规范维护者**（第 8 条）。**G 一条都不发明**：这三处缺口的存在本身已由本计划的
  Task 4 的参数表与 `call_stream` 的文档注释**据实记在实现里**（不发明探活超时、不给分片消费加截止、
  不扩 `ProviderError` 的取值域、不编一个流式的用量）。
- **第 7 条**（`ExecutionProfile` 的模型侧两个字段）：收件人是**构造 `ExecutionProfile` 的那一方
  （驱动侧的节点执行装配点）＋ `continuum-graph` ＋ 规范维护者**。**G 这条路径不经过 `ExecutionProfile`**
  ——它只取 `timeout_ms` 的**值**（§3.5），故本计划**不改 `continuum-graph` 的任何文件**。
- **第 10 条**（`list_registered`）：**已由 D 的计划 Task 14 Step 1 认领，且已交付**
  （**订正 2026-10-07，原话照留**：原句结尾是「尚未落地为代码」，那句已过期；
  交付证据见本计划上表那一格的订正注记——`persist.rs` 的 `list_registered`、提交 `510a90d` / `80250f2`、
  照片 `list_registered_lists_every_registered_model`）。
  **G 侧只作消费方**（Task 6），**不重述、不改动那个接口**。
- **第 11、12 条**（`model_providers()` 零消费方、C §6 第 2 步的混称）：**已闭（2026-10-06，C 已落实）**。
  **G 的处置是照现文写**：用 `model_for`（Task 6）、用 `health()`（Task 3）、
  **不调 `list_models()` / `describe_model()`**（Task 10）。
- **第 13 条**（三条退件：`effect_class` / `trust` / 工具侧 `cost`·`latency`）：
  **本子项目不接**（判据逐条在设计 §13.3）。收件人照原件：前两条是**子项目 F ＋ 规范维护者**，
  第三条是**协调者 ＋ 规范维护者**。**本计划不为它们建 task**。
- **第 14 条**（`model_for` 交出裸适配器、直发 `invoke` 绕得过 G）：**收不回**，
  且模型侧**没有可加的位置**。C 已于 2026-10-06 认领并同意该判断（C §12 第 24 条）。
  收件人：**规范维护者（§249 的射程是否扩到模型调用）＋ 控制器**。
  **G 侧据实记在 `plan_candidates` 的文档注释里**：本函数保证的是「**经 G 这条生产路径发出的调用，
  其模型必已过闸门**」——**这条有射程，它到持有注册表者直接 `model_for` + `invoke` 为止**。
- **第 15 条**（`decide_retry` 零生产调用点）：**G 的分类产物今天没有消费方**——
  **这不是 G 的漏做**（分类在 G、重试策略在执行层、裁决由 `decide_retry`）。
  收件人：**驱动侧的节点执行装配点（同第 7 条）＋ 协调者**。

### 五、其余两条（本计划的判断题，如实记）

```
Degraded 的降权判据                D §11 第 24 条 + 本设计 §13.2 的表：`ProviderHealth::Degraded`
                                  的降权判据**规范未给**，而降权是**排序策略**的事（D 的
                                  `RankingPolicy`），**G 不排序**。**但 G 是 `availability` 的唯一生产方**
                                  ——故本计划把这条据实记在此处：**这条判据缺了，会让 G 送出的
                                  `Degraded` 在 D 的基线里不产生任何效果**（Task 11 的
                                  `the_degraded_candidate_stays_in_the_output_unchanged` 正是它的照片）。
                                  **收件人：规范维护者。**

「G 是那条边的第二个使用点」          C §12 第 2 条记 `continuum-runtime → continuum-provider`
                                  是悬空边（`Cargo.toml` 声明了、引用为零），**收件人：F**。
                                  本计划记明 **G 是这条边的第二个使用点**（模型侧），F 是第一个（工具侧）。
                                  **边由用它那个 task 登记**（P2b 的规矩）：F 先落地就由 F 登记，
                                  G 落地时**只核不改**（Task 12 Step 2 的那条 `git diff` 就是这次核对）。
                                  同理 `runtime → continuum-core` 与 `runtime → continuum-model-registry`
                                  也各自有了真使用点。**收件人：无（据实记，供后来者对账）。**
```
