# P3 子项目 F（工具调用路径）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 Capability 的**强制点 (1)** 接上生产路径——新增 `tool` 子命令，走
`authorize` → `ProviderRegistry::invoke_tool`，含审计行、Journal 与失败路径。这是
`docs/superpowers/p3a-followups.md` 第一节那张表里「`authorize` 本 crate 之外零生产调用方」那一格的兑现处。

**Architecture:** 子命令的参数解析、装配与失败映射留在 bin（`tool_cmd`）；工具调用路径本身是一个
**库函数**（`tool_call`），签名收**已经打开**的 `&Db` 与 `&ProviderRegistry`——理由是**用例必须能注入一个
持有夹具适配器的注册表**（生产装配点今天是空的）。为此把 `TaskError` 从 bin 移进 lib，并把
`mint_declared_effects`（强制点 (2) 的铸币判定）提取成一个**不收事务**的共享函数，命令路径与工具路径同调它。
工具调用**不建 Task 工作区、不经 Integration Gate、不新增任何审计行**；它的结果由驱动打到 stdout。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-capability`（`authorize` / `AuthorizedTool` /
`mint` / `AuthorizedEffect` / `CapabilityKind::for_effect` / `save_tool`）、`continuum-provider`
（`ProviderRegistry::invoke_tool`，**子项目 C 交付**）、`continuum-core`（`ToolId` / `ToolResult` /
`ProviderError`）、`continuum-policy`（`load_policies` / `decide` / `PolicyContext`）、`continuum-effect`
（`record_planned` / `advance`）、`continuum-persist`（`Db` / `Tx`）、`tokio`（`block_on` 异步的那一跳）、
`thiserror`、`serde_json`；dev：`tempfile`、**`async-trait`**（见 Global Constraints 的最后一条）。

**设计依据：** `docs/superpowers/specs/2026-10-05-p3f-tool-call-path-design.md`（**唯一事实来源**）。
配套：共享面 `…p3-bcdf-ownership-and-interfaces.md`、裁决 `…p3-bcdf-set-decisions.md`（**第五节**：删
`ToolInvocation`）、交接 `docs/superpowers/p3bcdf-followups.md`、接缝图与协调者裁决
`.superpowers/sdd-p3bcdf/impl-seam-map.md`（**第六节**）。

---

## 前置：本计划硬依赖子项目 C 的交付

**执行序是串行 `B → D → C → F`，F 在最后。** 下列产物由 **C** 出，**在 C 落地之前本计划从 Task 4 起编不过**：

| C 的交付 | 用在哪 |
|---|---|
| `continuum_provider::ProviderRegistry` 类型与其**登记入口** | `run_tool_call` 的入参、`tool_cmd` 的装配、库级用例的夹具 |
| `ProviderRegistry::invoke_tool(&AuthorizedTool, input) -> Result<ToolResult, ToolCallError>` | 步骤 6 那一跳（**工具侧唯一入口**） |
| `continuum_provider::ToolCallError`（`Unregistered { id }` / `Provider(ProviderError)`） | `TaskError::ToolCall` 的包装对象 |
| `ToolProvider::invoke` 的请求参数换成 `AuthorizedToolInvocation<'_>` | F 的**夹具适配器**要实现的那个 trait |
| `continuum_core::tool::ToolInvocation` **已删**、`crates/continuum-provider/tests/fake_provider.rs` 已同批改 | 全仓不再有第二个请求类型 |
| `continuum-provider` 的 `ALLOWED` 条目改为三元素 | 与 `Cargo.toml` 精确一致 |

**Task 1–3 不依赖 C**（`save_tool` 不变量、lib 化、CLI 解析面）；**Task 4 起硬依赖 C**。
执行者开工前须先确认：`crates/continuum-provider/src/registry.rs` 与 `src/tool.rs` 已存在，
且 `grep -rn ToolInvocation crates/` **零命中**。

B 与 D 的产物（`CapabilityKind::effect`、`continuum-model-registry`、迁移 80/81 与 `main.rs`／
`tests/migrations.rs`／`tests/startup.rs` 的连带改）**在本计划开工时已经落地**——F 只做收口复核
（Task 10），不重复它们。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向；本计划不改 `ALLOWED`，也不改任何内部 crate 边。**
  `continuum-runtime` 条目（`crates/continuum-runtime/tests/dependency_direction.rs:123-138`）**已经含**
  本子项目要用的全部内部 crate（`capability` / `core` / `provider` / `policy` / `effect` / `persist` /
  `workspace` / `sandbox`）。**唯一要改的是 `:119-121` 那段注释**——它写着「core / events / provider 在
  runtime 内**至今无任何引用**」，本子项目第一次真的引用 `core` 与 `provider`（`events` 仍无）。
- **`crates/continuum-runtime/Cargo.toml` 只改一处：dev-dependencies 加 `async-trait`。** 理由：库级用例的
  夹具适配器要实现 `#[async_trait]` 标注的 `continuum_provider::ToolProvider`，而 Rust 要求 `impl` 侧同样
  标注该宏（`continuum-provider` 自己的 `tests/fake_provider.rs` 就是这么写的）。`async-trait` 是**外部**
  crate，已在 `Cargo.lock` 与 workspace 依赖表里，故 **`ALLOWED` 不受影响、`Cargo.lock` 也不变**。
  **设计侧「`Cargo.toml` 不动」这句因此有一处例外**，据实记在此处，不假装它没发生。
- 迁移：**本子项目不建表、不取号。** `runtime_migrations()` 里属于 F 的注册**一条都没有**（Task 10 只复核
  合并结果）。B/D 已取的号（D 的 80／预留 81）不在本计划里复核——**「未占用」按库判**，且那是 D 的活。
- `continuum-runtime` 不直接对枚举列写 SQL 字面量；审计 `kind` 列的比较在**用例**里手写字面量
  `"capability grants"` / `"external effects"`（钉格式），生产代码取 `AuditKind::as_str`。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿，**0 warning**；`cargo build --workspace --all-targets`
  同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
- **本计划改 `continuum-capability` 的 `src/persist.rs` 与 `tests/persist.rs`（Task 1），不改它的
  `Cargo.toml`，也不改它的 `ALLOWED` 条目。**（接缝图第六节第 8 条：B 改该 crate 的 `capability.rs`、
  F 改 `persist.rs`，两者不同文件、无编译冲突。**本计划不改 `capability.rs`。**）

## 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」
   跑全量是加分。**变异的三条失效形态都要防**：(a) 锚点不唯一 → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「这两版在哪个入参上会给出不同结果」，举不出即是等价，处理是
   **换真变异体而非补用例**；(c) **变异导致编译失败**——那不是「变红」。每次变异用**独立日志路径**，
   读前确认是本轮写的。**判据本身也会假阳性**：cargo 在**用例失败**时也打印
   `error: test failed, to rerun pass …`，故判「编译失败」要用 `could not compile` 或 `error[E….`。
2. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**。
   **枚举式绝对断言须逐项有照片**——本计划有两处这类断言：设计 §5.3「六个无 `EffectType` 对应的 kind
   在本路径上**一律** `MissingCapability`」（Task 5 逐项一条）、设计 §12.1「命令路径**不过**强制点 (1)」
   （Task 9 的 P-19）。**改完一处枚举，通读整段。**
3. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 Err」。本计划里凡是 `TaskError` 的出口，
   用例一律断言到**变体**（`TaskError::Capability(CapabilityError::UnknownTool { .. })` 这一层）。

**另两条运行纪律**：跑测试加 `timeout`，**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）。
**临时目录取仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`）；收工前 `chmod -R u+rwX .tmp && rm -rf .tmp`——但
**实现者不得在交活前删掉 `.tmp/`**（变异日志是证据，由协调者在复审结束后清理）。

**变异窗口与验证窗口互斥**：变异是「改源码 → 跑全量 → 还原」。**实现者报告完成之前，协调者不得在该
工作区里跑 cargo**；协调者的独立重跑严格排在实现者的终稿之后。

**否定式照片在本仓被接受**（交接 §四.4）：本计划的 P-19 就是「跑 `task` 路径后断言
`capability grants` 行数为 0」。**不要因为是否定命题就跳过它。**

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**（P1 出过 20+ 处事实错误），且已确立「**代码块是示意，
正文的措辞才是约束**」。故本计划只给**签名、错误变体与关键判定**，不给整段可粘贴实现；凡与既有 crate
交互的签名（`Tx` / `Db` / `audit_log` 列 / `PolicyContext` 字段 / `EffectSpec` / `Advance`／
`record_planned` 的形状），**实现前须先读该 crate 的源码确认**，不符时以源码为准并回报。
**导入清单是重灾区**：本计划不列 `use` 行，实现者按实际源码补。

## 本节依赖谁已交付什么 / 本节把什么交给谁

- **依赖已交付**：C 的 `ProviderRegistry` / `invoke_tool` / `AuthorizedToolInvocation<'_>` /
  `ToolCallError`，以及 `ToolInvocation` 的删除（**硬前置**，见上一节）；P3A 的
  `authorize` / `AuthorizedTool` / `mint` / `AuthorizedEffect` / `CapabilityKind::for_effect` /
  `save_tool` / `ToolProfile`；既有命令路径的 `record_declared_effects` / `open_db` / `TaskError`
  （由本计划自己提取与搬家）。
- **交给**：
  - **`continuum-capability`**（本项目无此子项目，只留条目）：登记项不变量落在 `save_tool`（本计划
    Task 1 落地，落地后 `tool` 表才有真正的写入闸）；`tool` 表的登记**至今无生产调用方**。
  - **`continuum-runtime` 的下一轮**：`runtime → secrets` 的边与凭据签发调用点（设计 §14 第 10 条）。
  - **子项目 C**：注册表里没有任何工具适配器（步骤 6 那一跳在生产里必然 `Unregistered`）。
  - **后续阶段**：§312 的「tool invoked」落点、`--input` 的落库语义。
  - **规范维护者（本项目无此角色）**：`effect_class` 两轴之问、`trust`、工具侧 `cost` / `latency`。

---

# 文件结构

```
crates/continuum-runtime/
  Cargo.toml                 dev-dependencies 加 async-trait（唯一一处改动）
  src/lib.rs                 新增 pub mod error / pub mod tool_call / pub mod sandbox_select，再导出 TaskError
  src/error.rs               新建（lib）：`TaskError` 从 task_cmd.rs 原样搬来
  src/tool_call.rs           新建（lib）：now_millis、CAPABILITY_LIFETIME_MS、
                             mint_declared_effects（无事务共享函数）、run_tool_call（工具调用路径）
  src/sandbox_select.rs      从 bin 移进 lib（内容不动，`use` 路径随之改）
  src/main.rs                去掉 `mod sandbox_select;`；分派 `Command::Tool`
  src/task_cmd.rs            TaskError 定义移出；authorize_declared_effects 换成共享函数；open_db 提 pub(crate)
  src/tool_cmd.rs            新建（bin）：`tool` 子命令的解析结果 → 装配 → 调 lib → 失败映射与退出码
  src/cli.rs                 `Command::Tool` / `ToolArgs` / 两个新 CliError 变体 / USAGE
  tests/tool_call.rs         新建：库级用例（夹具适配器经 C 的注册表登记）
  tests/tool_cli.rs          新建：端到端（真实二进制）
  tests/cli.rs               解析面用例（P-12 / P-13 / P-14 的解析那一半）
  tests/task_cli.rs          P-19 的落点（跑在 `task` 子命令上）
  tests/dependency_direction.rs  ALLOWED 不改；`:119-121` 注释订正

crates/continuum-capability/
  src/persist.rs             save_tool 加登记期不变量
  tests/persist.rs           既有夹具按不变量的连带修正 + 新用例
```

**既有的、本计划明确不改的文件**：`crates/continuum-provider/**`（C 的地盘）、
`crates/continuum-capability/src/capability.rs`（B 改过的文件）、
`crates/continuum-runtime/tests/{migrations.rs,startup.rs}`（B/D 改过的连带断言；
**F 不新增迁移，两文件在 F 这一轮无改动**）、`Cargo.toml`（workspace）、`Cargo.lock`。

---

### Task 1: `save_tool` 的登记期不变量

> **本 task 不依赖 C。** 它关掉的是设计 §5.2 那条「工具做了外部效应却没有效应记录」的洞——
> 协调者裁决（接缝图第六节第 1 条）把这一步判给 F：**实测今天的 `save_tool` 只是一条裸 `INSERT`，
> 不变量并未实现**（`crates/continuum-capability/src/persist.rs:64-88`）。**这是真活，不是照录。**

**Files:**
- Modify: `crates/continuum-capability/src/persist.rs`（`save_tool`）
- Modify: `crates/continuum-capability/tests/persist.rs`（新用例 + 既有夹具的连带修正）

**Interfaces:**
- Consumes: `continuum_capability::CapabilityKind::for_effect`（已在）、`Tool::effect_class` /
  `Tool::required_capabilities`（已在）
- Produces: `save_tool` 的新语义——`effect_class == Some(t)` 而 `required_capabilities` 不含
  `for_effect(t)` 时返回 `Err`，且**一行都不写**

- [ ] **Step 1: 写用例（红）**

`tests/persist.rs` 新增三条，**两条方向都要有**（只写拒绝那一侧不算钉住）：

- `saving_a_tool_whose_effect_class_is_not_covered_is_rejected`：`effect_class: Some(Charge)`、
  `required_capabilities: [Filesystem(Read)]` → `save_tool` 返回 `Err`。**红的条件**：今天的 `save_tool`
  写成功、返回 `Ok`。**且断言被拒之后 `tool` 表里没有那一行**（`count_tools` 不增）——这是「写都不写」，
  不是「写了再删」。
- `a_declared_effect_class_covered_by_the_capabilities_is_accepted`：`Some(Charge)` + `[Payment(Charge)]`
  → `Ok`，且读得回来。**方向相反的那一半**：缺了它，一个「恒拒绝」的实现也全绿。
- `a_tool_without_an_effect_class_may_declare_anything`：`None` + `[]` → `Ok`。**这一条钉的是不变量的
  射程**：它只管 `Some(t)` 那一侧，`None` 的工具不受约束（设计 §5.2 的不变量形如
  `effect_class == Some(t) ⇒ …`，前提不成立时无结论）。

**断言到变体**（纪律 3）：`save_tool` 的返回类型是 `Result<(), PersistError>`，本文件的既有约定是
「本文件只做委托与错误适配，表外取值一律转成 `PersistError`」（`persist.rs` 模块文档），而 `PersistError`
没有语义类变体。故**报错取 `PersistError::Database(消息)`**，用例断言
`matches!(err, PersistError::Database(ref m) if m.contains(…))`。**替代处置（否掉）**：给
`CapabilityError` 加一个变体——`save_tool` 的签名收的是 `PersistError`，改签名会牵动
`authorize`（它经 `#[from] PersistError` 转出）与 P3A 的既有用例，代价远大于收益。**实现前须读
`PersistError` 的定义确认变体集**，若已有更贴切的变体则改用它并在此处订正。

- [ ] **Step 2: 修正既有夹具的连带面（不修就必然红）**

`crates/continuum-capability/tests/persist.rs` 的 `tool(id, required)` 夹具取
`effect_class: Some(EffectType::DeleteRemote)`，而 `sample()` 给的 `required_capabilities` 是
`[Filesystem(Read), Git(WorktreeWrite)]`——**它不含 `Git(DeleteRemote)`，故在本 task 之后必然被拒**。
受影响的是 `tool_round_trips`、`required_capabilities_round_trip`、`saving_the_same_id_twice_is_rejected`。

处置：**保留 `Some` 那一侧的往返照片**（P3A 的 `tool_round_trips` 注释明写「设计 §10 第 9 条禁止让
`Some` 一侧不可观察」），故**改夹具的 capability 表而不是把 `effect_class` 改成 `None`**——即在
`sample()` 与 `required_capabilities_round_trip` 的 `kinds` 里补上 `CapabilityKind::Git(GitAction::DeleteRemote)`，
并同步更新那条**手写的 JSON 串断言**（`["git_push","filesystem_read","payment_charge","git_push"]`）。
`insert_raw` 直接写 SQL、不经 `save_tool`，不受影响。

**做法**：改完之后 `grep -n "save_tool(" crates/continuum-capability/` **逐处**判断该调用点用的是哪一份
夹具、是否满足不变量；**不要只改报错的那一条**。

- [ ] **Step 3: 跑，确认失败**

```bash
cargo test -p continuum-capability --test persist
```

- [ ] **Step 4: 实现**

在 `save_tool` 的 `INSERT` **之前**判定（设计 §5.2 的不变量）：

```rust
/// 登记期不变量（设计 §5.2）：声明了 `effect_class == Some(t)` 的工具，
/// 其 `required_capabilities` 必须含 `CapabilityKind::for_effect(t)`。
///
/// 为什么落在本函数：`save_tool` 是 `tool` 表**唯一的生产写点**，故不变量
/// 「关在唯一入口上」，不依赖任何调用方自觉。
/// 为什么这条不变量够：它使 `authorize` 的两向合取**要求**出示集里有一枚
/// `for_effect(t)`，而出示集只可能来自 `--effect` 铸出的能力（`for_effect` 是单射），
/// 于是那条效应必进 Journal——「工具做了外部效应却没有效应记录」的洞由此堵上。
/// **射程**：前提不成立（`None`）时无结论；本函数不管 `None` 的工具。
fn assert_effect_class_is_covered(tool: &Tool) -> Result<(), PersistError> { /* 一个判定，一处 */ }
```

判定的**关键点**（照抄会错的地方）：
- `for_effect` **写在 `CapabilityKind` 上**（不是 `EffectType` 上），本 crate 已依赖 `continuum-effect`，
  **不新增边**；
- 比对**按 `CapabilityKind` 的相等**（`required_capabilities.contains(&expected)`），**不要**比字符串；
- 不变量只管 `Some(t)`；`required_capabilities` 里的**重复项**不影响判定（它是列表不是集合）。

- [ ] **Step 5: 运行，确认转绿；再跑全量**

```bash
cargo test -p continuum-capability
cargo test --workspace --no-fail-fast
```

- [ ] **Step 6: 变异（两条方向各一次，日志各用独立路径）**

| 变异 | 期望 |
|---|---|
| 把判定从 `save_tool` 里**删掉**（回到裸 `INSERT`） | `saving_a_tool_whose_effect_class_is_not_covered_is_rejected` 红 |
| 把判定改成**恒 `Err`** | `a_declared_effect_class_covered_by_the_capabilities_is_accepted` 与 `a_tool_without_an_effect_class_may_declare_anything` 红 |

**第二条变异必须是真变异体**：判据是「这两版在哪个入参上给出不同结果」——恒 `Err` 与正版在
`Some(Charge)+[Payment(Charge)]` 上不同，成立。

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-capability/src/persist.rs crates/continuum-capability/tests/persist.rs
git commit -m "feat(capability): save_tool 的登记期不变量（effect_class 必须被能力表覆盖）"
```

---

### Task 2: lib 化与共享函数提取

> **本 task 不依赖 C。** 它是纯搬家和一次提取，**行为不变**。三件事在设计的 §3.2 与 §6.4 里都有落点，
> 但**有一件的代价设计没有算到**：`TaskError::SandboxSelect` 的 `#[from]` 目标是
> **bin 模块** `src/sandbox_select.rs` 里的 `SandboxSelectError`——设计 §3.2 说它「来自 lib 已依赖的
> crate」，**那句为假**。处置见 Step 2。

**Files:**
- Create: `crates/continuum-runtime/src/error.rs`（lib）
- Create: `crates/continuum-runtime/src/tool_call.rs`（lib，本 task 先只放时钟与共享函数）
- Move: `crates/continuum-runtime/src/sandbox_select.rs` → lib 的 `pub mod sandbox_select`
- Modify: `crates/continuum-runtime/src/lib.rs`、`src/main.rs`、`src/task_cmd.rs`

**Interfaces:**
- Produces（lib，**必须 `pub`**——bin 是另一个 crate，`pub(crate)` 对它不可见）：
  `continuum_runtime::TaskError`、`continuum_runtime::tool_call::{now_millis, CAPABILITY_LIFETIME_MS,
  mint_declared_effects}`、`continuum_runtime::sandbox_select::{self, SandboxSelectError}`

- [ ] **Step 1: 把 `TaskError` 搬进 lib，变体一个不改**

`task_cmd.rs:874-959` 的 `TaskError` **整块**移进 `src/error.rs`，变体、字段、`#[error(...)]` 文案、
文档注释**一律照录**。`lib.rs` 加 `pub mod error;` 与 `pub use error::TaskError;`；
`task_cmd.rs` 与 `main.rs` 改 `use continuum_runtime::TaskError;`。

**被否掉的替代**（设计 §3.2，记此免得后来者重提）：另建一个 lib 侧的 `ToolCallPathError` + bin 侧逐变体
映射——那会给同一批失败造出**第二个错误类型**，正是本项目判为 Critical 的「同一件事两个类型」。

- [ ] **Step 2: 把 `sandbox_select` 一并搬进 lib（设计漏算的一步）**

`TaskError::SandboxSelect(#[from] SandboxSelectError)` 里的 `SandboxSelectError` 定义在
**`src/sandbox_select.rs`**，而该模块今天由 `main.rs` 的 `mod sandbox_select;` 声明，**属 bin**。
故「变体一个不改」要求**该模块整体搬进 lib**：`lib.rs` 加 `pub mod sandbox_select;`，
`main.rs` 删掉那一行，`task_cmd.rs` 改
`use continuum_runtime::sandbox_select::{self, SandboxSelectError};`，
`sandbox_select.rs` 内部的 `use continuum_runtime::cli::SandboxMechanism;` 改成 `use crate::cli::…`。

**代价**：这一步比设计写的「只搬 `TaskError`」大。**替代处置（否掉）**：把变体改成
`SandboxSelect(String)`——那既违反「变体一个不改」，又把一个带类型的错误降级成字符串。
**该模块自带 `#[cfg(test)]` 单元用例**，随模块一起搬，用例内容不动。

- [ ] **Step 3: 搬时钟与常量，并提取无事务共享函数**

`now_millis`（`task_cmd.rs:867-872`）与 `CAPABILITY_LIFETIME_MS`（`task_cmd.rs:174-188`）移进
`src/tool_call.rs`，**`pub`**（bin 的 `task_cmd` 还在用 `now_millis`）。两者**只有一份定义**：
`expiry` 与 `planned_at` 若取自两份不同的时钟读数，那是同一件事两个产生点。

把 `authorize_declared_effects`（`task_cmd.rs:442-471`）**提取并加宽**成：

```rust
/// 强制点 (2) 的铸币判定（设计 §3.2）。**不收事务**——它不碰库。
///
/// **它并不「纯」**：`mint` 的 `expiry` 取 `now_millis() + CAPABILITY_LIFETIME_MS`，
/// 而 `now_millis()` **读时钟**。要说准的是「不碰库」。
///
/// 返回 `(AuthorizedEffect, Decision)` 成对：`Decision` 不再被丢掉——工具路径要用它
/// 填 `effect.authorization`（那条效应**自己**的那次裁决），而它不得第二次调 `arbitrate`
/// （那正是设计 §5.1 禁止的第二个判定点）。
pub fn mint_declared_effects(
    policies: &[Policy],
    effects: &[EffectSpec],
    approve: bool,
) -> Result<Vec<(AuthorizedEffect, Decision)>, TaskError>;
```

函数体与 `authorize_declared_effects` 逐字相同，只在 `push` 时**把 `decision` 一并带上**。
`record_declared_effects` 改调它，并**照旧丢弃**那个 `Decision`（它要的是集成那次裁决，另有来源）。

- [ ] **Step 4: 写用例（红）**

`src/tool_call.rs` 的 `#[cfg(test)] mod tests`：

- `the_shared_minting_function_returns_each_effect_own_decision`：给两条 `--effect` 与一张只对
  **其中一条类型**成立的 `Allow` 规则表，断言返回的两项里，**能铸出的那条带 `Allow`**、另一条
  报 `TaskError::EffectNotAuthorized`；换一张表（两条都放行）后两项各带**本条**的裁决。
  **红的条件**：把 `mint_declared_effects` 的返回改成只给 `AuthorizedEffect`（即丢掉 `Decision`）
  → 编译失败不算红；**真变异体**是「两项都填第一条的 `Decision`」，此时两条效应类型不同、裁决不同，
  断言必红。
- `the_shared_minting_function_reports_the_first_unmintable_in_declaration_order`：与既有
  `capability_gate.rs` 的同名端到端用例同法，但**在 lib 里无门控地跑**（不依赖沙箱）。**红的条件**：
  改成报最后一条即红。

- [ ] **Step 5: 跑，确认失败**

```bash
cargo test -p continuum-runtime --lib
```

（此时 `mint_declared_effects` 还不存在，编译不过——**这是「红」的形态之一**，但纪律 1 说「变异导致编译
失败不算变红」。本 step 是新函数的首次落地，属红-绿里正常的「还没有实现」，不是变异判定，记此以免混淆。）

- [ ] **Step 6: 跑全量，确认命令路径行为不变**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
```

预期：全绿、0 warning。**命令路径的全部既有端到端用例（`task_cli.rs` / `capability_gate.rs`）是这一步
「搬家没改语义」的照片**——它们一条都不许改。

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-runtime/src crates/continuum-runtime/tests
git commit -m "refactor(runtime): TaskError 与 sandbox_select 移进 lib，提取无事务的铸币共享函数"
```

---

### Task 3: `tool` 子命令的解析面

> **本 task 不依赖 C。** 只动 `cli.rs`（lib）与 `tests/cli.rs`。

**Files:**
- Modify: `crates/continuum-runtime/src/cli.rs`
- Modify: `crates/continuum-runtime/tests/cli.rs`

**Interfaces:**
- Produces: `continuum_runtime::cli::{Command::Tool, ToolArgs}`、
  `CliError::{InvalidToolInput, OptionRequiresEffect}`

- [ ] **Step 1: 写用例（红）**

`tests/cli.rs` 新增（**四条 `UnknownOption` 各一条，不抽代表**——它们是四条独立分支）：

- `tool_parses_its_own_options`：`tool --db d --tool t1 --input '{"a":1}' --effect charge:x --intent i1`
  → `ToolArgs` 各字段与输入逐项相同。
- `tool_input_defaults_to_an_empty_object`：省略 `--input` → `input == json!({})`（**不是 `null`**）。
- `tool_rejects_an_unparsable_input`（P-12）：`--input '{'` → `Err(CliError::InvalidToolInput { .. })`，
  **断言到变体**。
- `an_effect_without_an_intent_is_rejected`（P-13 之一）：`--effect charge:x` 无 `--intent` →
  `Err(CliError::OptionRequiresEffect { option: "--intent" })`。
- `an_intent_or_an_approval_without_an_effect_is_rejected`（P-13 之二）：
  `--intent i1` 无 `--effect` → `option == "--intent"`；`--approve` 无 `--effect` → `option == "--approve"`。
- `a_tool_call_without_any_effect_parses`（**零效应的正常侧**）：`--tool t1`（无 `--effect`/`--intent`/
  `--approve`）→ `Ok`。
- `tool_rejects_the_task_options`（P-14）：`--base` / `--exec` / `--apply` / `--sandbox` **各一条**，
  都 `Err(CliError::UnknownOption { name })` 且 `name` 等于该选项名。
- `tool_requires_db_and_tool`：两个必填项缺任一 → `CliError::MissingOption`（**两条**）。

**红的条件**：今天的 `parse` 只有 `task` / `recover` 两个臂，`"tool"` 落到 `UnknownSubcommand`。

- [ ] **Step 2: 跑，确认失败**

```bash
cargo test -p continuum-runtime --test cli
```

- [ ] **Step 3: 实现**

```rust
pub enum Command { Task(TaskArgs), Tool(ToolArgs), Recover(RecoverArgs) }

/// `tool` 子命令的参数（设计 §2.1）。只承载解析结果。
pub struct ToolArgs {
    pub db: PathBuf,
    /// 工具 id：复用 `continuum_core::tool::ToolId`，**不另建第二个**（设计 §2.1）。
    /// 字形不校验：未登记的工具由强制点 (1) 以 `UnknownTool` 拒掉。
    pub tool: ToolId,
    /// 请求侧承载的那段 `input` JSON。**省略即 `{}`**（设计 §2.1 的决定，
    /// 判据照 `task_cmd` 对 `parameters` 填 `json!({})` 的先例）。
    pub input: serde_json::Value,
    /// 幂等键的第一段。**有 `--effect` 时必填**，零效应时必须不给（设计 §2.2）。
    pub intent: Option<IntentId>,
    /// 与命令路径同一个含义（第 2 级显式确认）。
    pub approve: bool,
    pub effects: Vec<EffectSpec>,
}
```

`CliError` 加两个变体（**只加这两个**）：

```rust
/// `--input` 不是合法 JSON。解析期即拒（同 `EffectType::parse` 的判据：
/// 不可解析的输入不该到运行期才失败）。`reason` 取 serde_json 的消息。
#[error("--input 不是合法 JSON（{value}）：{reason}")]
InvalidToolInput { value: String, reason: String },

/// `--effect` 与 `--intent` / `--approve` **必须同进同出**（设计 §2.2），
/// 两向都拒。`option` 点名是哪一个选项缺了它的搭档。
#[error("选项 {option} 与 --effect 必须同进同出：给出 --effect 时必须给 --intent；\
         未给任何 --effect 时不得给 {option}")]
OptionRequiresEffect { option: &'static str },
```

**判定次序写死、并有用例**：零效应时若 `--intent` 与 `--approve` **同时**给出，报的是
`--intent`（按选项表次序先判）。这一条由 `an_intent_or_an_approval_without_an_effect_is_rejected`
的两条断言共同钉住。

`--base` / `--exec` / `--apply` / `--sandbox` **不进 `tool` 的 `match`**，故自然落到既有的
`UnknownOption` 臂——**不要为它们写专门的臂**（那是同一件事两个产生点）。`USAGE` 同步补 `tool` 的用法行。

- [ ] **Step 4: 跑，确认转绿**

```bash
cargo test -p continuum-runtime --test cli
timeout 1500 cargo test --workspace --no-fail-fast
```

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-runtime/src/cli.rs crates/continuum-runtime/tests/cli.rs
git commit -m "feat(runtime): tool 子命令的解析面"
```

---

### Task 4: 库函数 `run_tool_call` 与「那一跳」

> **硬依赖 C**（`ProviderRegistry` / `invoke_tool` / `ToolCallError` / 新请求类型）。本 task 落地设计 §3
> 的**全部七步**，并把「F 确实经注册表调用」这一格拍下来。

**Files:**
- Modify: `crates/continuum-runtime/src/tool_call.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`（dev-dependencies 加 `async-trait`）
- Create: `crates/continuum-runtime/tests/tool_call.rs`

**Interfaces:**
- Consumes: `continuum_provider::ProviderRegistry`（含其登记入口——**签名以 C 的源码为准**）、
  `Task 1-3` 的产物
- Produces: `continuum_runtime::tool_call::run_tool_call`

- [ ] **Step 1: 定签名，并先读 C 的源码核对**

```rust
/// 工具调用路径（设计 §6.4）。**收已经打开的 `&Db` 与 `&ProviderRegistry`**：
/// 开库与装配留在 bin，本函数只做路径本身。
///
/// 入库的**唯一**理由：用例必须能注入一个持有**夹具适配器**的注册表——生产装配点今天是空的。
pub fn run_tool_call(
    db: &Db,
    registry: &ProviderRegistry,
    args: &ToolArgs,
) -> Result<(), TaskError>;
```

**这条签名是示意**：`ProviderRegistry` 的**构造**与**登记入口** C 未在 F 的设计里冻结
（设计 §10.2 明写 F 不预先发明它）。实现前读 `crates/continuum-provider/src/registry.rs` 与
`src/tool.rs`：**方法与实参表以源码为准**，不符时在后面各步骤里按源码写并回报。

- [ ] **Step 2: 写用例（红）**

`tests/tool_call.rs`：库级用例，夹具适配器经 C 的注册表登记。**夹具的形状照
`crates/continuum-provider/tests/fake_provider.rs`**（跨 crate 的 `tests/` 目录不可导入，故这是**第二份
副本**——C 的那一份钉的是「工具级失败走 `Ok(is_error: true)`、provider 级失败走 `Err`」这条约定，F 的
这一份只服务本路径的用例，**不合并**）。夹具要记两件事：**被调用的次数**与**收到的那份请求**。

> **一处措辞要写准**：实现 `continuum_provider::ToolProvider`（`#[async_trait]`）时，`impl` 的签名里**必须
> 写下**那个请求类型（`AuthorizedToolInvocation<'_>`，C 设计 §7.5）。设计说「F 不构造也不命名那个请求
> 类型」，**那句话的射程是生产调用点**——F 的生产路径把 `&AuthorizedTool` 与 `input` 交给
> `invoke_tool`，**不自己 `new` 出请求、也不在别处再出现一个产生点**。夹具的 `impl` 签名是另一回事，
> 记此以免实现者以为自己违规。

用例：

- `the_registry_is_the_only_way_the_tool_is_reached`（P-6）：登记夹具后跑一条放行的调用，断言夹具**收到
  一次调用**。**红的条件**：把步骤 6 换成「什么都不做、直接返回 `Ok`」→ 本条红。
- `the_tool_id_comes_from_the_authorized_proof`（P-7）：断言夹具收到的请求里那枚授权证明的
  `tool_id()` **等于** `--tool` 给的那个。**红的条件**：把 `authorize` 的 `tool_id` 实参写成常量即红。
- `the_input_reaches_the_adapter_verbatim`（P-7 的第二半）：断言夹具收到的 `input` 与 `--input` **逐字相同**。
  **红的条件**：把 `input` 写成 `json!({})` 常量即红。
- `an_effect_free_call_still_reaches_the_tool`（P-17，**P-6 的对照臂**）：零 `--effect`、登记项声明空能力表、
  策略放行 → 夹具**真的被调用**。**红的条件**：一个「有效应才调用」的实现本条红，而 P-6 仍绿——**两条都
  要在**。
- `an_unregistered_id_reports_the_routing_failure`（P-9）：**不登记任何适配器** → 
  `TaskError::ToolCall(ToolCallError::Unregistered { id })`，`id` 与 `--tool` 相同。**红的条件**：把
  `Unregistered` 吞成 `Ok`、或换成 `Provider(..)` 即红。**断言到内层变体**。
- `a_provider_failure_is_carried_through`（P-10）：夹具返回 `Err(ProviderError::Transport(..))` →
  `TaskError::ToolCall(ToolCallError::Provider(..))` 原样带出。**红的条件**：把它包装成另一种变体即红。

> **P-7 的措辞与设计 §9 有一处出入**：设计 §9 的表写的断言对象是「收到的 `call.tool`」——那是已删类型
> `ToolInvocation` 的字段；按 C 设计 §7.5，新请求类型只有「授权证明 + `input`」，**没有 `tool` 字段**。
> 本计划按 §7.5 写（读授权证明的 `tool_id()`），并把这条出入回报给协调者。

- [ ] **Step 3: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
```

- [ ] **Step 4: 实现七步**

次序与设计 §3 的表逐条对齐：

1. **幂等键预检**：逐条 `--effect` 用 `effect_key(intent, spec)`（`task_cmd.rs:512`，**复用，不另派一次**）
   查 `find_by_idempotency_key`，任一已存在即 `TaskError::EffectAlreadyRecorded { key }` 拒**整条**。
   次序照命令路径：**幂等键检查排在强制点之前**。
2. **读策略表一次**：`load_policies(&tx)`。
3. **强制点 (2)**：`mint_declared_effects(&policies, &args.effects, args.approve)?` →
   `Vec<(AuthorizedEffect, Decision)>`。**不重写裁决、不新增判定点。**
4. **强制点 (1)**：`presented` **由步骤 3 的产物逐枚 `capability().clone()` 而来，按 `--effect` 的声明次序**
   （设计 §4.1：不另建一个自由浮动的 `Vec<Capability>`——那会让同一批能力有两个可以各自构造的容器）。
   `now` 在这一步**取一次**，交给 `authorize` 与下面各条效应的 `planned_at` / `updated_at` 复用。
   失败 → `TaskError::Capability(..)` 原样带出（`#[from]`）。
5. **写效应行并提交**：逐条 `PLANNED → AUTHORIZED → EXECUTING`（`record_planned` + 两次 `advance`），
   `authorization` 填**这条效应自己那次裁决**（`authorization_field(args.approve, decision)`，
   `task_cmd.rs:537` 的既有编码形状**复用**），`parameters` 填 `json!({})`（照 `task` 的先例），
   `tx.commit()`。**步骤 4 与 5 用同一个事务**——这正是设计 §3.2 那条承重性质（审计行与 `EXECUTING`
   行要么都在、要么都不在）的来源。
6. **那一跳**：`block_on(registry.invoke_tool(&authorized_tool, args.input.clone()))`。运行时用
   `tokio::runtime::Builder::new_current_thread().build()`——**不要用 `enable_all()`**：workspace 的 tokio
   features 是 `["rt-multi-thread", "macros"]`，`enable_all` 按 `net`/`time` 等 feature 门控，可能不可用；
   换 feature 会动 `Cargo.toml`，与「不动内部依赖清单」相抵。**若确需 `enable_all`，停下来回报**，
   不要就地改 feature。失败（含 `Unregistered`）→ **各效应记 `FAILED`**（与 `task` 第 5 步机制选择失败
   时的处置一致），再把 `TaskError::ToolCall(..)` 报出去。
7. **终态与 stdout**：按 `ToolResult.is_error` 写 `COMMITTED` / `FAILED`；**无论 `is_error` 为何，先把
   `output` 以 JSON 一行打到 stdout**（设计 §3.3：工具调用**没有子进程**，stdout 是调用方仅有的通道），
   `is_error == true` 时再返回 `TaskError::ToolReportedError { tool }`（Task 7 落地该变体；**本 task 先只做
   `is_error == false` 那一支与打印**，`is_error` 那一支在 Task 7 补——见 Task 7 的说明）。

**不要做的事**（写了就是发明）：不建 Task 工作区、不经 `Integration Gate`、不写任何审计行、不读
`Tool::effect_class`、不自己开第二个工具调用入口。

- [ ] **Step 5: 跑，确认转绿；再跑全量**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
timeout 1500 cargo test --workspace --no-fail-fast
```

- [ ] **Step 6: 变异**

| 变异 | 期望 |
|---|---|
| 步骤 6 换成「直接 `Ok(())`」（不调注册表） | P-6 与 P-17 红 |
| `presented` 换成空集 | P-7 之外，Task 5 的 P-4 / P-5 红（**P-6 仍绿**——两个变异体落在不同用例上，故两组都要在） |
| `input` 换成 `json!({})` 常量 | `the_input_reaches_the_adapter_verbatim` 红 |

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-runtime/src/tool_call.rs crates/continuum-runtime/tests/tool_call.rs \
        crates/continuum-runtime/Cargo.toml
git commit -m "feat(runtime): 工具调用路径的库函数与经注册表的那一跳"
```

---

### Task 5: 强制点 (1) 的照片

> 本 task **只加用例与变异**，不改 `run_tool_call` 的语义（它在 Task 4 已经全量落地）。
> **红的条件由变异给出**——本仓接受变异为红条件。

**Files:**
- Modify: `crates/continuum-runtime/tests/tool_call.rs`

- [ ] **Step 1: 写用例**

- `an_unmintable_effect_refuses_the_call_and_the_tool_is_never_invoked`（P-1）：空策略表 ⇒ `Deny` →
  `TaskError::EffectNotAuthorized { effect, target, decision }`（**复用既有变体**，报**声明次序第一条**
  铸不出的）；**夹具的调用次数为 0**；`effect` 表 **0 行**、`audit_log` 的 `capability grants` **0 行**。
- `an_unknown_tool_is_rejected`（P-3）：库里无该 id → `TaskError::Capability(CapabilityError::UnknownTool { id })`；
  夹具调用次数 0；效应 0 行。
- `a_presented_capability_the_tool_did_not_declare_is_rejected`（P-4）：登记项声明 `A`，用一条 `--effect`
  铸出 kind `B` → `UndeclaredCapability { kind: B }`。
- `a_required_capability_that_is_missing_is_rejected`（P-5）：登记项声明 `A + B`，只声明铸出 `A` 的那条
  `--effect` → `MissingCapability { kind: B }`。
- `a_corrupt_registration_is_reported_as_a_persist_failure`（P-8）：照
  `crates/continuum-capability/tests/authorize.rs` 的 `insert_bad_capabilities` 写坏 `required_capabilities`
  列 → `TaskError::Capability(CapabilityError::Persist(..))` 原样带出。
- `every_kind_without_a_policy_fact_is_missing_capability`（**设计 §5.3 的绝对措辞，逐项一条**）：
  登记**六条**工具，`required_capabilities` 各含下列之一，**不声明任何 `--effect`**，断言每条都得到
  `MissingCapability { kind }` 且 kind 与该工具声明的那枚**相同**：
  `Filesystem(Read)`、`Filesystem(Write)`、`Git(Read)`、`Git(WorktreeWrite)`、`Git(CommitLocal)`、
  `Github(CreatePr)`。**六项逐项断言，不抽代表**（手写分支能各自漂移）。

**红的条件（逐条）**：
- P-1 / P-3 / P-4 / P-5 / P-8：把 `authorize` 那一跳**挪到写效应行之后**（或把它的 `Err` 吞掉继续往下走）
  → 「零效应行 / 零审计 / 零调用」的断言全红；
- `every_kind_without_a_policy_fact_is_missing_capability`：把 `presented` 换成「按登记项的
  `required_capabilities` 逐枚铸」的那条被否掉的路 → 六条全部变成 `Ok`（或另一变体），本条红。

- [ ] **Step 2: 跑，确认转绿**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
```

- [ ] **Step 3: 变异（**承重守卫 → 全量套件**）**

| 变异 | 期望 |
|---|---|
| 把 `authorize(...)` 从步骤 4 挪到步骤 5 的 `tx.commit()` **之后** | P-1 / P-3 / P-4 / P-5 全红（`effect` 行与 `EXECUTING` 已落库） |
| 把 `authorize` 的 `Err` 用 `unwrap_or_else(\|_\| ...)` 换成放行 | 同上四条红，且零调用的断言红 |

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-runtime/tests/tool_call.rs
git commit -m "test(runtime): 强制点 (1) 的两个拒绝方向与零调用的照片"
```

---

### Task 6: Journal 与审计的照片

**Files:**
- Modify: `crates/continuum-runtime/tests/tool_call.rs`
- Modify: `crates/continuum-runtime/tests/tool_cli.rs`（端到端那两条，**本 task 先建文件的最小骨架**；
  真正的端到端接线在 Task 8——见下）

- [ ] **Step 1: 写用例**

- `the_intent_is_observable_in_the_idempotency_key`（P-2 的第二半）：先跑一次成功调用（带 `--intent i1`），
  读回那条 `effect` 行，断言 `idempotency_key` 形如 `<意图字节长>:<意图>:<类型>:<目标>` 且第一段就是 `i1`
  （键形见 `task_cmd.rs:512-520`）。**这一读是设计 §2.2 那条「`--intent` 是可观察的」的照片**——没有它，
  「条件必填」的理由就只是一句话。
- `a_repeated_declaration_is_rejected_and_writes_nothing`（P-2 的第一半）：再跑同一条声明 →
  `TaskError::EffectAlreadyRecorded { key }`（**复用**），且**零新行**。
- `the_audit_rows_are_exactly_one_grant_plus_four_per_effect`（P-15）：读 `audit_log.kind` **列**，
  k=1 时 multiset 恰为 `{capability grants × 1, external effects × 4}`、共 **5** 条；
  **k=0 时恰 1 条**（`capability grants`）。两处 `kind` 的值**手写字面量**（钉格式）；
  `4 = 1 次登记 + 3 次推进`（同 `continuum-effect/tests/persist.rs:252` 的既有计数）。
- `an_effect_free_call_touches_neither_the_effect_table_nor_the_mint`（P-16）：零 `--effect` → `effect`
  **0 行**、`audit_log` **恰 1 条**（`capability grants`），**且没有凭据捏造的「tool invoked」之类审计行**
  （§313 的八项是封闭清单，本子项目**不新增 `AuditKind`**）。
- `each_declared_scope_is_carried_into_the_audit_payload_verbatim`（P-18）：声明两条目标各不相同的
  `--effect`，跑完之后读 `audit_log` 的 `capability grants` 行、解析 payload，断言
  `capabilities[].scope` 的 multiset **逐个等于**各 `--effect` 的目标。

> **P-18 为什么能在端到端跑通（即使注册表是空的）**：审计行由步骤 4 的 `authorize` 写，与效应行在
> **同一次提交**（步骤 5）里落库——**早于**步骤 6 那一跳。故端到端跑到步骤 6 以 `Unregistered` 失败，
> 审计行**已经在库里**。P-15 同理（终态那一次 `advance` 也照写）。

**红的条件**：
- P-2：把 `effect_key` 换成普通分隔符拼接 → 既有单测 `the_effect_key_separates_the_intent_from_the_target` 红；
  把预检**删掉** → 本条的「零新行」红。
- P-15：**把 `record_planned` 或任一次 `advance` 删掉** → 条数变成 4 或 3，本条红；**多写一条审计**同样红
  （这正是「只断言『有一条 capability grants』会漏掉」的那一格）。
- P-16：把零效应那一支改成仍开事务写审计 → 本条红。
- P-18：把 `spec.target` 换成常量、或换成 `effect_type` 的字面串 → 本条红。

- [ ] **Step 2: 跑，确认转绿**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
```

- [ ] **Step 3: 变异（P-15 是**跨 crate 才可见的效果 → 全量套件**）**

逐条跑上表四个变异，各用独立日志路径；P-15 的两条必须在**全量**下得出否定结论。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-runtime/tests/tool_call.rs
git commit -m "test(runtime): 幂等键、审计行数与作用域原样带出的照片"
```

---

### Task 7: 剩余失败面与两处刻意分岔

**Files:**
- Modify: `crates/continuum-runtime/src/error.rs`（加 `ToolReportedError`）
- Modify: `crates/continuum-runtime/src/tool_call.rs`（步骤 7 的 `is_error` 支）
- Modify: `crates/continuum-runtime/tests/tool_call.rs`

- [ ] **Step 1: 写用例（红）**

- `a_tool_that_reports_its_own_failure_marks_the_effects_failed`（P-11）：夹具返回
  `Ok(ToolResult { is_error: true, .. })` → `TaskError::ToolReportedError { tool }`（`tool` 等于 `--tool`
  的那个 id），且各效应记 `FAILED`（读回 `effect.state`）。
  **红的条件**：把 `is_error` 当成功（写 `COMMITTED`）即红；把它当 `ToolCallError` 包装即红
  （设计 §8：`is_error` **不是** provider 失败）。
- `the_authorization_field_records_this_effect_own_decision`（设计 §7.2 与 §14 第 6 条的**刻意分岔**）：
  造一条只对**这一条效应的类型**成立的 `RequireApproval` 规则，给出 `--approve`，跑通后读回效应行，
  断言 `authorization` 是 `approve=true;policy=<本条效应用它自己的上下文裁出的裁决>`——
  **不是**集成那次裁决（工具路径没有集成裁决）。
  **红的条件**：把共享函数返回的 `Decision` 换成 `arbitrate(&policies, &policy_context(args.approve))`
  （即集成那次裁决）即红；把 `approve` 那一半写成常量即红。

> **一处据实写明**：P-11 在设计 §9 里的断言是「效应记 `FAILED`、**stdout 仍有 output**」。
> **「stdout 仍有 output」这半边本计划拍不到**：P-11 是**库级**用例（生产注册表为空 ⇒ 端到端到不了那一跳），
> 而库级用例里 `println!` 的输出被测试框架捕获、库函数也不把那个值交出来。**据实记入 `## 遗留`**，
> 不发明一个「把 stdout 换成可注入的 writer」的接口（那是设计没有的位置）。

- [ ] **Step 2: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
```

- [ ] **Step 3: 实现**

`TaskError` 加**一个**变体：

```rust
/// 工具跑起来了、但**自报失败**（`ToolResult.is_error`）。
///
/// 与 [`TaskError::ToolCall`] 是两件事：那一个是「provider 没跑成」（传输 / 协议 /
/// 没有适配器），本变体是「跑成了，结果是错误」。按 C 设计 §7.1 的三分法，两者
/// 走**不同的通道**（`Ok(is_error: true)` vs `Err`），故不得合并。
#[error("工具 {} 自报失败", tool.as_str())]
ToolReportedError { tool: ToolId },
```

`run_tool_call` 步骤 7 补 `is_error` 那一支：**先打印 `output`**（与成功支同一句），再写 `FAILED`，
再返回本变体。**不新增任何别的变体**——尤其**不要**为「没有适配器」另立一个（那是 C 的
`ToolCallError::Unregistered`，F 包装而不另起词汇），也不要加「裁决为 `Deny`」这类重复既有判断的变体
（那是 `mints` 的判断）。

- [ ] **Step 4: 跑，确认转绿；再跑全量**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
timeout 1500 cargo test --workspace --no-fail-fast
```

- [ ] **Step 5: 变异**

| 变异 | 期望 |
|---|---|
| `is_error` 支写成 `EffectState::Committed` | P-11 红 |
| `authorization` 那一处改用集成那次裁决 | `the_authorization_field_records_this_effect_own_decision` 红 |

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-runtime/src/error.rs crates/continuum-runtime/src/tool_call.rs \
        crates/continuum-runtime/tests/tool_call.rs
git commit -m "feat(runtime): 工具自报失败与 authorization 字段的分岔"
```

---

### Task 8: bin 侧 `tool_cmd`、`main.rs` 分派与端到端

**Files:**
- Create: `crates/continuum-runtime/src/tool_cmd.rs`
- Modify: `crates/continuum-runtime/src/main.rs`（`mod tool_cmd;` + `Ok(Command::Tool(a)) => …`）
- Create/Modify: `crates/continuum-runtime/tests/tool_cli.rs`

- [ ] **Step 1: 写端到端用例（红）**

`tests/tool_cli.rs` 走 `env!("CARGO_BIN_EXE_continuum-runtime")`（与 `task_cli.rs` 同法）：

- `an_unparsable_input_fails_at_parsing_time`（P-12 的端到端那一半）：`--input '{'` → 退出码非零、
  stderr 含 `--input`；**库未被创建**（`--db` 指向的路径不存在）。
- `the_option_group_is_rejected_in_both_directions`（P-13）：两向各一条端到端用例。
- `the_task_only_options_are_unknown_here`（P-14）：`--base` / `--exec` / `--apply` / `--sandbox`
  **四条各一条**。
- `a_registered_tool_is_still_unrouted_and_the_call_fails_at_the_last_hop`：登记一条工具（经
  `save_tool`，测试里直接开库写）后跑 `tool` → 退出码非零、stderr 指 `未登记`；**但 `audit_log` 的
  `capability grants` 已有 1 条**（步骤 4 已提交）——这条同时是「强制点 (1) 在**正常路径上**被执行」的
  端到端照片，也是设计 §12 那条「有生产调用方」判据的落点。
- `the_declared_scopes_reach_the_audit_payload`（P-18 的端到端形态）：读 `audit_log` 的 payload，
  逐项断言 `capabilities[].scope`。**本条的库级版本在 Task 6**，此处只钉「真二进制也走这条路」。

**红的条件**：`main.rs` 没有 `Command::Tool` 臂 → 这些用例全红（解析成功的调用会落到「未知子命令」或
不产出任何库文件）。

- [ ] **Step 2: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_cli
```

- [ ] **Step 3: 实现**

`tool_cmd.rs`（bin）只做三件事，**不含路径逻辑**：

1. **装配**：`open_db`（`task_cmd.rs:323` 提到 **`pub(crate)`**，两条子命令共用同一份迁移集合——
   两处各写一份清单会让「注册的集合」有两个来源）+ 构造一个 `ProviderRegistry`。**今天的驱动没有任何
   适配器可登记**，故**装配点是空的**——这一句要写成注释并说明它不是遗留物（是 C 的交付缺口，
   见 `## 遗留`）。**登记的实参表以 C 的源码为准**，本计划不写死它（设计 §10.2 明写 F 不预先发明）。
2. **调 lib**：`continuum_runtime::tool_call::run_tool_call(&db, &registry, &args)`。
3. **失败映射**：`Err(e)` → `eprintln!("工具调用失败: {e}")` + `ExitCode::FAILURE`。
   **不要把 `ToolResult.output` 在这里打印**——它在 lib 的步骤 7 已经打出（两条路径不能有两个输出点）。

`main.rs` 加一个 `match` 臂，与 `Task` 臂同形。

- [ ] **Step 4: 跑，确认转绿；再跑全量**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_cli
timeout 1500 cargo test --workspace --no-fail-fast
```

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-runtime/src/tool_cmd.rs crates/continuum-runtime/src/main.rs \
        crates/continuum-runtime/src/task_cmd.rs crates/continuum-runtime/tests/tool_cli.rs
git commit -m "feat(runtime): tool 子命令的装配、分派与端到端"
```

---

### Task 9: 强制点 (1) 的射程边界（P-19）与 `dependency_direction.rs` 注释订正

**Files:**
- Modify: `crates/continuum-runtime/tests/task_cli.rs`（P-19 的落点）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**只订正 `:119-121` 的注释**）

- [ ] **Step 1: 写 P-19（否定式照片）**

`task_cli.rs` 新增 `the_command_path_never_reaches_the_first_checkpoint`：

跑 `task --effect charge:x --exec true`（策略放行、退出码 0、**受既有的
`require_auto_selected_sandbox` 门控**——但**承重断言不放在门控之内**：门控只跳过「必须真跑起来」的
那一半，**库里的行数断言**在门控外照跑，若门控跳过则不静默通过而是显式跳过并报执行/跳过条数），
随后断言 `SELECT COUNT(*) FROM audit_log WHERE kind = 'capability grants'` 为 **0**。

**红的条件**：把强制点 (1) 接到命令路径上（例如在 `record_declared_effects` 里也调一次 `authorize`）
→ 本条红。**这是本计划唯一一条跑在另一条子命令上的用例**——它钉的正是设计 §12.1 那条边界。

- [ ] **Step 2: 订正 `dependency_direction.rs:119-121` 的注释**

该处写着「core / events / provider 在 runtime 内**至今无任何引用**（`grep -rn 'continuum_core\|…'`
无命中）」——**本子项目第一次真的引用 `continuum-core` 与 `continuum-provider`**（`events` 仍无）。
**改那半句，并把错误说法的来历留在原地**（本仓的既有做法），注明是哪一次改的：

```
// **订正（P3 子项目 F）**：本行原文写「core / events / provider 在 runtime 内至今无任何引用」。
// F 新增 tool 子命令之后，前半句已成假——`cli.rs` 取用 `continuum_core::tool::ToolId`，
// `tool_call.rs` / `tool_cmd.rs` 取用 `continuum_provider::ProviderRegistry` 与 `ToolCallError`。
// events 仍无引用。错误说法的来历留此，免得下一轮扫查照着它得出错的结论。
```

**`ALLOWED` 表本身一个字都不改**（它已经含这三个 crate，见 Global Constraints）。

- [ ] **Step 3: 跑**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test task_cli
```

- [ ] **Step 4: 变异**

把 `authorize` 那一跳复制一份到 `task_cmd` 的 `record_declared_effects` 里（**临时**）→ P-19 必须红；
还原。**这一次变异同时是「P-19 确实有判别力」的证据**，日志单独存。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-runtime/tests/task_cli.rs crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "test(runtime): 强制点 (1) 的射程边界；订正 runtime 依赖注释"
```

---

### Task 10: 收口（复核三处合并结果）

> 串行执行序的最后一位做一遍收口（前一阶段 P3A 就是这么收的）。**本 task 只复核、只在发现不一致时改**。

**Files:**
- Modify: 视复核结果而定（预期不改任何文件）

- [ ] **Step 1: 复核 `crates/continuum-runtime/tests/dependency_direction.rs` 的并集**

逐条核：B 的 `continuum-connector` 条目与其对 runtime 的边、D 的 `continuum-model-registry` 条目与
runtime 的边、C 的 `continuum-provider` 条目（三元素）、**F 自己不改 `ALLOWED`**。
判据是**逐对 `assert_eq!`**（含 dev 边）。跑：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
cd /home/DslsDZC/Continuum && cargo tree -p continuum-runtime --depth 1 --edges all --prefix none
```

**逐行比对** `cargo tree` 的输出与 `ALLOWED` 的 runtime 条目，**并集多一条少一条都要当场处置**。

- [ ] **Step 2: 复核 `main.rs` 的迁移与装配注册**

逐条核 `runtime_migrations()` 里的八处 `extend`（P0 内置 + P1 两条 + P2 三条 + P3 的 capability + D 的
model-registry），与 `tests/migrations.rs` 的 `expected_migrations()` **互为覆盖**；
复核 `tests/startup.rs` 的三处计数（`:24` / `:90` 的「迁移应用 N 项」与 `:79` 的补应用数）与
**行内注释**一致——B/D 各自改过，**注释与断言互相打脸是本项目点过名的形状**。

**F 在这一步的期望是「零改动」**：F 不建表、不取号、不新增迁移。

- [ ] **Step 3: 复核 workspace `members`**

`Cargo.toml`（workspace）应含 `continuum-connector` 与 `continuum-model-registry` 两个新成员；
`dependency_direction.rs` 的派生名单（`workspace_crates()`）与 `ALLOWED` 的 `subjects` **互为覆盖**。
跑 `every_crate_depends_only_on_its_allowed_set` 即可判（它内部就断言这两个方向）。

- [ ] **Step 4: 全量证据**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：全绿、**0 warning**。**把两次运行的完整输出路径（`.tmp/` 下的日志）写进报告**——这就是收口的证据。

- [ ] **Step 5: 逐条核对完成判据**

| 判据 | 证据 |
|---|---|
| §4.4「工具调用前校验 Capability」 | P-1（拒时零调用）+ P-17 / P-6（放行时真的调用）这对**对照臂** |
| 强制点 (1) **有生产调用方** | Task 8 的 `a_registered_tool_is_still_unrouted_and_the_call_fails_at_the_last_hop`（真二进制走到步骤 4） |
| 强制点 (1) 的射程边界 | P-19 |
| 登记项不变量 | Task 1 的两向用例 |
| Capability 不可与裸字符串互换 | **P3A 已交付**，本子项目**不重复** |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 6: 残余落到有版本的文档**

`.superpowers/` 是 gitignore 的，**只写在报告或 ledger 里的结论会随 branch 消失**。把残余写进
`docs/superpowers/p3bcdf-followups.md`（**不新建第二个同类文件**——同一类事项两个家正是本项目反复处置
的毛病）。本 task 追加的内容以 `## 遗留` 一节为准。

- [ ] **Step 7: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs: P3 子项目 F 的收口与复核"
```

---

## 遗留

```
工具登记无生产调用方     save_tool 只在测试里被调（生产零调用），故真实库里 tool 表是空的，
                        `tool` 子命令对任何 id 都报 UnknownTool，除非先经 save_tool 登记。
                        **本计划不改这条**（不发明一个登记用的 CLI）。
                        收件人：continuum-capability（本项目无此子项目）。
注册表里没有适配器       `ToolProvider` 的唯一实现是测试夹具；组合根登记不出东西，故步骤 6 那一跳
                        在生产路径上**必然**返回 ToolCallError::Unregistered。**这不是遗留物**，
                        它不挡强制点 (1)（authorize 在步骤 4，早于那一跳）。
                        收件人：子项目 C。
granted() 的消费方      按 C 设计 §7.5，是**适配器**（在 invoke 的实现体内读授权证明、逐枚取
                        Capability::scope() 限定动作范围）。**F 不调它**，只把整枚 AuthorizedTool
                        经 invoke_tool 交出去。无照片（本阶段没有真实适配器）。
六个 kind 无策略事实     Filesystem(Read|Write)、Git(Read|WorktreeWrite|CommitLocal)、
                        Github(CreatePr) 在 EffectType 里没有对应项，故驱动铸不出它们，
                        含它们的工具在本路径上**一律** MissingCapability（Task 5 逐项拍了照片）。
                        **本路径不在登记期拦**（那会在别人的入口上装一道只有本层知道的口径）。
                        收件人：策略层（PolicyContext 的事实集合）与子项目 D。
作用域的**判定**        本路径不判作用域（authorize 只比 kind），强制落在凭据签发（§51）与执行点。
                        **原样带出**那一半有照片（P-18，Task 6/Task 8）。
                        收件人：子项目 B。
两处与命令路径的刻意分岔 (a) effect.authorization 在本路径上写**这条效应自己那次裁决**，而 task 写的是
                        集成那次——工具调用没有集成裁决；(b) 本路径的 --input **不落库**
                        （§267 的 parameters 语义未规定，照 task 先例填 {}）。
                        **不要按 task 的写法「顺手统一」。** 收件人：后续阶段（与 §312 一同定）。
工具调用不建 Task 工作区 若某个工具将来确实需要一个工作区，§316 的请求侧里没有把它交给适配器的通道。
                        收件人：子项目 C（改 §316 的调用面是 C 的活）。
§312「tool invoked」无落点 全仓没有 Execution Trace 设施；本子项目不发明一张 trace 表，
                        也不新增 AuditKind（§313 的八项是封闭清单）。故「某次工具调用被发起」这件事
                        只在**有 --effect 时**经效应记录可查，零效应的调用不留痕。
                        收件人：后续阶段（与 §312 / §317 的落点一起定）。
凭据签发的接线          若这条调用需要凭据，唯一来源是 continuum_secrets::issue(&cap, now)。
                        本计划**不登记** runtime → secrets 的边、不加调用点（零使用的边即假边）。
                        收件人：continuum-runtime 的下一轮，或按 p3a-followups 第三节末尾的原计划
                        与 B 的密钥运行时接线一并做（协调者已把那条边的所有者裁给 B）。
tokio 首次真使用         continuum-runtime/Cargo.toml 早已声明 tokio，而 src/ 与 tests/ 至今零引用；
                        本子项目为 block_on 异步的那一跳**第一次真用**它（单次调用用当前线程运行时）。
                        收件人：后续阶段（若驱动整体转异步，这个形状要重做）。
--input 省略即 {} /     两条都是本设计的**决定**，规范未规定，已由协调者本轮拍板接受；
--intent 条件必填        「--intent 可观察」已由 P-2 的第二半（Task 6）拍下来。
AuthorizedTool 不进       TaskError::Capability 的 Display 只带内层 CapabilityError 的消息
错误上下文              （那些消息里没有作用域）。**这一条没有照片**（关于「驱动没写某句格式化」的
                        否定命题），记此以免后来者顺手加一句 {:?} 而没人发现。
effect_class 两轴 /      三条义务曾被 D 退件给「子项目 F ＋规范维护者」，而 F 的设计只声明「不读」。
trust / 工具侧           本计划**不做 task**：四份设计都写明「不发明」，而 §4.1 的组件表里
cost / latency          **没有「工具选择」这个组件**，故收件人是**规范维护者（本项目无此角色）**。
                        **这是「收件人挂了空」的第二次具名**，记此以免它再次无声挂空。

**没有照片的失败路径**（据实，不发明夹具）
CapabilityError::Expired **F 自己那次 authorize 调用上不可达**（步骤 3 铸出与步骤 4 校验之间没有 I/O，
                        两处 now 相差微秒级，而 expiry 在 15 分钟之后 ⇒ 那一行比较会跑、但永不触发）。
                        **这不是「本路径上不可达」**——能力确实会离开 F（经整枚 AuthorizedTool 交给
                        适配器），而 invoke 的时长 F 没有上界，故**下游**（适配器取用 / 凭据签发）
                        才是它第一次可能真的判出过期的地方。**F 这一侧没有照片，也不该有**：
                        为它造用例只能靠改常量或改时钟，那是把被测对象换成夹具。
                        照片在 continuum-capability 自己的 tests/authorize.rs。
终态写入失败            同 task_cmd.rs:266-267 记的那条（DB 写失败时多半别的写也失败，两条路自然收敛；
                        造不出「已打开的连接写不进去」而不改夹具）。
P-11 的「stdout 仍有 output」这一半  P-11 是库级用例（生产注册表为空 ⇒ 端到端到不了那一跳），而库级用例里
                        println! 的输出被测试框架捕获、库函数也不把那个值交出来。故设计 §9 那一行
                        只拍到了「效应记 FAILED、报 ToolReportedError」这一半。
                        **不发明**一个「可注入 writer」的接口（设计没有这个位置）。
```
