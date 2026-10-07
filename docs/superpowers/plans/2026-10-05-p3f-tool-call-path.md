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
配套：共享面 `docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md`、裁决
`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md`（**第五节**：删 `ToolInvocation`）、
交接与**接缝裁决** `docs/superpowers/p3bcdf-followups.md`（**§七**：八条接缝裁决的**权威转录**）。

> **本计划的约束来源一律是可入库的文档。** 接缝裁决引 **`p3bcdf-followups.md` §七**，**不要引
> `.superpowers/sdd-p3bcdf/impl-seam-map.md`**——那是 gitignore 的 scratch，**随时会丢**（本项目为此丢过一次），
> 引它当约束来源会让指针悬空。承重的几条已逐条抄进本计划正文（Task 1 的注、Task 3 的注、Task 9 的三处条目），
> 正文自带判据，不依赖任何 gitignore 的文件。

---

## 前置：本计划硬依赖子项目 C 的交付

**执行序是串行 `B → D → C → F`，F 在最后。** 下列产物由 **C** 出，**在 C 落地之前本计划从 Task 3 起编不过**：

| C 的交付 | 用在哪 |
|---|---|
| `continuum_provider::ProviderRegistry` 类型与其**登记入口** | `run_tool_call` 的入参、`tool_cmd` 的装配、库级用例的夹具 |
| `ProviderRegistry::invoke_tool(&AuthorizedTool, input) -> Result<ToolResult, ToolCallError>` | 步骤 6 那一跳（**工具侧唯一入口**） |
| `continuum_provider::ToolCallError`（`Unregistered { id }` / `Provider(ProviderError)`） | `TaskError::ToolCall` 的包装对象 |
| `ToolProvider::invoke` 的请求参数换成 `AuthorizedToolInvocation<'_>` | F 的**夹具适配器**要实现的那个 trait |
| `continuum_core::tool::ToolInvocation` **已删**、`crates/continuum-provider/tests/fake_provider.rs` 已同批改 | 全仓不再有第二个请求类型 |
| `continuum-provider` 的 `ALLOWED` 条目改为三元素 | 与 `Cargo.toml` 精确一致 |

**Task 1–2 不依赖 C**（`save_tool` 不变量、lib 化）；**Task 3 起硬依赖 C**。
执行者开工前须先确认：`crates/continuum-provider/src/registry.rs` 与 `src/tool.rs` 已存在，
且 `crates/` 里**不再有非注释的旧请求类型引用**：

```bash
cd /home/DslsDZC/Continuum && grep -rnw ToolInvocation crates/   # 预期：无输出，exit 1
```

> **订正（2026-10-07）——上面这条命令不是判据，原话照留，它的来历与错因一并记此。**
>
> **它今天不成立，而且必然不成立，是本仓自己的规矩让它不成立的**：C 落地 `ToolInvocation` 的删除时，
> 按本仓「**把错误说法与来历留在原地**」的既有做法，在注释里写下了那个已删的名字
> （如 `crates/continuum-provider/src/registry.rs:195` 的「旧类型 `ToolInvocation` 已裁删……」与
> `tests/compile_fail/a_bare_tool_id_cannot_be_passed_to_invoke_tool.rs:4` 的模块文档）。
> **注释里的名字照样被 `grep` 命中**，故「零命中」这条断言与「留来历」那条规矩**直接相抵**。
>
> **它作为判据还依赖分支**（实测）：同一条命令在 **`p3cf`** 上返回 **2 处命中，全在注释里**；
> 在**未合并 C 的 `p3d`（主检出）**上返回 **5 处命中，全是代码**（`core/src/tool.rs:26` 的定义、
> `provider/src/tool.rs:5`/`:12`、`provider/tests/fake_provider.rs:7`/`:109`）。
> **同一条命令在两处结论不同**，这本身就说明它不该当判据用。
>
> **判据改成**：**`crates/` 里不再有「非注释」的 `ToolInvocation` 引用**。
> 做法是**先 `grep -rnw ToolInvocation crates/` 定位，再逐处读那几行、确认每一处都在注释里**——
> 与本仓「**grep 只用来定位，结论以通读为准**」同一条。
>
> **原句的正面仍然成立、仍然需要，但它解决的是另一件事**：`-w` 词边界匹配**必须保留**——
> 不能写成 `grep -rn ToolInvocation crates/`，因为 C 一落地 `AuthorizedToolInvocation`，那条无边界的形式
> **必然把它子串命中**（`AuthorizedToolInvocation` 里就有 `ToolInvocation` 八个字母）。
> 两件事不要混：**`-w` 解决的是「子串误命中」，本节订正解决的是「来历注释必然命中」**。
>
> **判据留档（本节最要紧的一句）**：**「某名字零命中」是对一个开放语料（全仓文本）的断言**，
> 而本仓**要求**把删掉的名字留在注释里。**故凡「零命中」类判据，先问「本仓的哪条规矩会往那个语料里
> 加字」**——答得出，那条判据就立不住，要改成对**行**的判据（读那几行、判它是不是注释），
> 或改成对**结构化语料**的判据（编译产物、`cargo tree`、库表行数）。
> （同一句错误断言也出现在共享面文档里，已由协调者订正；计划侧即本处。）
>
> **这层区分要写准（同一份计划里两种都允许，判据是看它数的是什么）**：
>
> | | 合规 | 不合规 |
> |---|---|---|
> | 形态 | **对「行」的判据**：先 `grep` **定位**，再逐处读那几行下判断；**对「结构化语料」的判据**：编译是否通过、`cargo tree` 的边、库表行数、夹具调用次数、审计条数 | **对「全仓文本」的零命中**：`grep -rn <名字> crates/` 预期无输出 |
> | 为什么 | 数的是**闭集**（那几行、那张表、那次构建），本计划改了别处它照样成立 | 数的是**开放语料**，本仓的「留来历」规矩、以及本计划自己的改动**都会往里加字** |
> | 本计划里的例子 | `:264` 的 `grep "save_tool("` 配「逐处判断」；`:947` 引既有注释原文（Task 8 正要订正它）；P-1 的「零效应行 / 零审计 / 夹具零调用」；P-15 的审计条数；Task 9 的 `cargo tree` 与 `cargo build` | 前置里那条 `grep -rnw ToolInvocation … # 预期：无输出，exit 1`（本节订正的对象） |
>
> **第二条同族规矩（状态陈述要带时点）**：**「至今零引用 / 至今没有」是带时点的状态陈述，而写它的
> 那份计划往往正是要改变那个状态的那一份**——**故凡「至今没有」出现在「将要引入」的同一份文档里，
> 它自带一个到期日**，要写成「**截至……时** ＋ **本计划起……**」的形式，必要时一并写明分支
> （同一条命令在不同分支上结论可以不同，见上）。本计划里按此改过的两处：`tokio` 那条（Task 9 的遗留）、
> `save_tool` 生产调用方那条（同节）；另有一处**只用来当理由**的零命中断言（Task 2 Step 1 的
> `grep -n TaskError …`）已按此删去 grep 那半句、只留语义理由。

B 与 D 的产物（`CapabilityKind::effect`、`continuum-model-registry`、迁移 80/81 与 `main.rs`／
`tests/migrations.rs`／`tests/startup.rs` 的连带改）**在本计划开工时已经落地**——F 只做收口复核
（Task 9），不重复它们。

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
  crate，已在 workspace 依赖表里，故 **`ALLOWED`（内部边表）不受影响**。
  **订正（2026-10-07，回灌 F Task 3 的实测）**：本行原写「**`Cargo.lock` 也不变**」——**那句是假的**。
  dev 边同样会写进 `Cargo.lock`：`[[package]] continuum-runtime` 的 `dependencies` 列表里**多出一行
  `"async-trait"`**（`async-trait` 这个**包本身**早已在锁里，多的是这条**边**）。**实测**：Task 3 的提交
  `dfefb54` 对 `Cargo.lock` 的改动是 **`1 +`**，正是这一行。**故 Task 3 Step 11 的 `git add` 必须含
  `Cargo.lock`**（否则锁过期）。**旧话留此**，免得后来者再照它断言「锁不变」。
  这条 dev 边已据实记在设计 §3.2 与 §10.1（该处的原文是「`Cargo.toml` 的**内部依赖清单**不动」，
  与 dev 边的这条例外不冲突）——**那两处的「`Cargo.lock` 不受影响」半句已同批订正**。
- 迁移：**本子项目不建表、不取号。** `runtime_migrations()` 里属于 F 的注册**一条都没有**（Task 9 只复核
  合并结果）。B/D 已取的号（D 的 80／预留 81）不在本计划里复核——**「未占用」按库判**，且那是 D 的活。
- `continuum-runtime` 不直接对枚举列写 SQL 字面量；审计 `kind` 列的比较在**用例**里手写字面量
  `"capability grants"` / `"external effects"`（钉格式），生产代码取 `AuditKind::as_str`。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿，**0 warning**；`cargo build --workspace --all-targets`
  同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
- **本计划改 `continuum-capability` 的 `src/persist.rs` 与 `tests/persist.rs`（Task 1），不改它的
  `Cargo.toml`，也不改它的 `ALLOWED` 条目。** 判据（`p3bcdf-followups.md` §七第 8 条，**已知非缺陷、
  不做任何事**）：**同一个 `continuum-capability` 的源码被两个子项目各改一处**——**B** 改
  `capability.rs`（加 `CapabilityKind::effect` 逆），**F** 改 `persist.rs`（加 `save_tool` 不变量）；
  两者**不同文件、无编译冲突**，且**都不新增该 crate 的依赖边**（`EffectType` 已是依赖，`for_effect` 同 crate）。
  同一条还裁了另一件事：**C 的 `FakeTool` 与 F 的库级夹具是同一约定的两份副本，不合并**（两份各有其用，
  见 Task 3 Step 4）。

## 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」
   跑全量是加分。**变异的三条失效形态都要防**：(a) 锚点不唯一 → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「这两版在哪个入参上会给出不同结果」，举不出即是等价，处理是
   **换真变异体而非补用例**；(c) **变异导致编译失败**——那不是「变红」。每次变异用**独立日志路径**，
   读前确认是本轮写的。**判据本身也会假阳性**：cargo 在**用例失败**时也打印
   `error: test failed, to rerun pass …`，故判「编译失败」要用 `could not compile` 或 `error[E….`。
2. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**。
   **枚举式绝对断言须逐项有照片**——本计划有两处这类断言：设计 §5.3「六个无 `EffectType` 对应的 kind
   在本路径上**一律** `MissingCapability`」（Task 4 逐项一条）、设计 §12.1「命令路径**不过**强制点 (1)」
   （Task 8 的 P-19）。**改完一处枚举，通读整段。**
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
交互的签名（`Tx` / `Db` / `audit_log` 列 / `PolicyContext` 字段 / `EffectSpec` / `advance`／
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
  - **F（驱动）自己 / 将来的适配器子项目**：**一个可登记的 `ToolProvider` 实现**——注册表里没有它，
    故步骤 6 那一跳在生产里必然 `Unregistered`。装配者就是驱动自己（组合根，设计 §10.2），
    而 C 按中立性规则不会在 `src/` 里放 `impl`（**锚点是 C 设计 §7.1 末段**；**订正 2026-10-07**：
    原写「C 设计 §12.4 同此口径」——**C 设计没有 `12.4` 这一节**，§12 是按条编号的遗留清单）。
    **这条不指子项目 C。**
  - **后续阶段**：§312 的「tool invoked」落点、`--input` 的落库语义。
  - **规范维护者（本项目无此角色）**：`effect_class` 两轴之问、`trust`、工具侧 `cost` / `latency`。

---

# 文件结构

```
crates/continuum-runtime/
  Cargo.toml                 dev-dependencies 加 async-trait（唯一一处改动）
  src/lib.rs                 新增 pub mod error / pub mod tool_call / pub mod sandbox_select，再导出 TaskError
  src/error.rs               新建（lib）：`TaskError` 从 task_cmd.rs 原样搬来；Task 3 起加两个包装变体
  src/tool_call.rs           新建（lib）：now_millis、CAPABILITY_LIFETIME_MS、
                             mint_declared_effects（无事务共享函数）、run_tool_call（工具调用路径）
  src/sandbox_select.rs      从 bin 移进 lib（内容不动，`use` 路径随之改）
  src/main.rs                去掉 `mod sandbox_select;`；加 `mod tool_cmd;` 与 `Command::Tool` 的分派臂
  src/task_cmd.rs            TaskError 定义移出；authorize_declared_effects 换成共享函数；open_db 提 pub(crate)
                             （Task 3 再改一处：effect_key / authorization_field 的定义搬进 lib 的
                             `tool_call`，本文件改为 `use`——工具路径的调用点在 lib、定义在 bin，见 Task 3 的 Files 注）
  src/tool_cmd.rs            新建（bin）：`tool` 子命令的装配、调 lib、失败映射与退出码
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
**F 不新增迁移，两文件在 F 这一轮无改动**）、`Cargo.toml`（workspace）。
**`Cargo.lock` 不在「不改」之列（订正，2026-10-07）**：本行原把它与 `Cargo.toml`（workspace）并列。
Task 3 加 `async-trait` 的 dev 边会**多写进锁里一行**（实测 `dfefb54` 的 `Cargo.lock` 为 `1 +`），
故它**必须**进 Task 3 Step 11 的 `git add`。**旧话留此。**

---

### Task 1: `save_tool` 的登记期不变量

> **本 task 不依赖 C。** 它关掉的是设计 §5.2 那条「工具做了外部效应却没有效应记录」的洞。
>
> **这一步的所有者是 F。** 判据（`p3bcdf-followups.md` §七第 1 条，逐条抄此）：设计侧把这条不变量的
> 落点定在 `continuum-capability` 的 `save_tool`，而**本轮没有这个子项目**，故**没有任何一份计划会因它而红**；
> 裁给 F 的理由是——**它正是设计 §5.2 那条洞的唯一关闭点**（「关在唯一入口上」，`save_tool` 是 `tool` 表的
> 唯一生产写点），而 F 本轮已要改这个 crate。
> **实测今天的 `save_tool` 只是一条裸 `INSERT`，不变量并未实现**
> （`crates/continuum-capability/src/persist.rs:64-88`）。**这是真活，不是照录。**

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
  射程**：它只管 `Some(t)` 那一侧，`None` 的工具不受约束（不变量形如
  `effect_class == Some(t) ⇒ …`，前提不成立时无结论）。

**断言到变体**（纪律 3）：`save_tool` 的返回类型是 `Result<(), PersistError>`，本文件的既有约定是
「本文件只做委托与错误适配，表外取值一律转成 `PersistError`」（`persist.rs` 模块文档），而 `PersistError`
没有语义类变体。故**报错取 `PersistError::Database(消息)`**，用例断言
`matches!(err, PersistError::Database(ref m) if m.contains(…))`。**替代处置（否掉）**：给
`CapabilityError` 加一个变体——`save_tool` 的签名收的是 `PersistError`，改签名会牵动
`authorize`（它经 `#[from] PersistError` 转出）与 P3A 的既有用例，代价远大于收益。**实现前须读
`PersistError` 的定义确认变体集**，若已有更贴切的变体则改用它并在此处订正。

- [ ] **Step 2: 修正既有夹具的连带面（不修就必然红）**

`tests/persist.rs` 的 `tool(id, required)` 夹具（`:32-42`）恒取
`effect_class: Some(EffectType::DeleteRemote)`，而 `sample()`（`:44-52`）给的 `required_capabilities` 是
`[Filesystem(Read), Git(WorktreeWrite)]`——`for_effect(DeleteRemote) = Git(DeleteRemote)`（`capability.rs:102`），
**不含**，故凡经 `sample()` 的 `save_tool` 在本 task 之后必然被拒。**受影响的是四处，逐处给处置**：

| # | 用例 | 处置 |
|---|---|---|
| 1 | `tool_round_trips`（经 `registered("a")` / `unregistered("b")`） | 在 `sample()` 的 capability 表里补 `CapabilityKind::Git(GitAction::DeleteRemote)`——**保留 `Some` 那一侧的往返照片**（该用例的注释明写「设计 §10 第 9 条禁止让 `Some` 一侧不可观察」），故**不把 `effect_class` 改成 `None`** |
| 2 | `required_capabilities_round_trip`（`:290-338`） | 前半用 `tool("a", kinds)`：`kinds` 补 `Git(DeleteRemote)`，并同步该用例里那条**手写 JSON 断言**（`:316` 一带，`["git_push","filesystem_read","payment_charge","git_push"]`）。**后半的空表支 `tool("b", vec![])`（`:320-321`）另给处置**：该支的对象正是「空列表往返成空列表」，**补能力会把这一支弄没**，故改成**直接构造**（`effect_class: None` + 空表，与 `pure()` 同法），并在原处写明为何这一支与 `sample()` 的处置不同 |
| 3 | `saving_the_same_id_twice_is_rejected`（经 `registered("a")`） | 随 `sample()` 一起修好，用例本身不动 |
| 4 | `enum_columns_use_the_lowercase_encoding`（`:201-263`，`:205` 用 `registered("a")`） | 随 `sample()` 一起修好；**但该用例 `:217` 有第二处手写 JSON 断言**（`["filesystem_read","git_worktree_write"]`），**必须同步**，否则它独自变红 |

`insert_raw` 与 `a_column_of_the_wrong_type_is_rejected` 直接写 SQL、不经 `save_tool`，**不受影响**；
`tests/authorize.rs` 的 `profile()`（`:29-43`）`effect_class` 为 `None`，**也不受影响**。

**做法**：改完之后 `grep -n "save_tool(" crates/continuum-capability/` **逐处**判断该调用点用的是哪一份
夹具、是否满足不变量；**不要只改报错的那一条**（上面这张表是点名清单，不是替代品）。

- [ ] **Step 3: 跑，确认失败**

```bash
cargo test -p continuum-capability --test persist
```

- [ ] **Step 4: 实现**

在 `save_tool` 的 `INSERT` **之前**判定：

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

> **本 task 不依赖 C。** 它是纯搬家和一次提取，**行为不变**。三件事在设计的 §3.2 与 §6.4 里都有落点。

**Files:**
- Create: `crates/continuum-runtime/src/error.rs`（lib）
- Create: `crates/continuum-runtime/src/tool_call.rs`（lib，本 task 先只放时钟与共享函数）
- Move: `crates/continuum-runtime/src/sandbox_select.rs` → lib 的 `pub mod sandbox_select`
- Modify: `crates/continuum-runtime/src/lib.rs`、`src/main.rs`、`src/task_cmd.rs`

**Interfaces:**
- Produces（lib，**必须 `pub`**——bin 是另一个 crate，`pub(crate)` 对它不可见）：
  `continuum_runtime::TaskError`、`continuum_runtime::tool_call::{now_millis, CAPABILITY_LIFETIME_MS,
  mint_declared_effects}`、`continuum_runtime::sandbox_select::{self, SandboxSelectError}`
- **另有 bin 自己已在调用的项随搬家一并 `pub`（`arbitrate`／`policy_context`／`mints`／
  `decision_name`／`explicit_current_rule`），它们是「行为不变」的必要条件，不是新增 API。**
  **理由**：`mint_declared_effects` 的**函数体**要调 `arbitrate`／`mints`／`decision_name`／
  `policy_context`（逐条效应那次裁决用 `policy_context_for_effect`），而 bin 的 `task_cmd.rs`
  仍在调 `arbitrate`／`policy_context`／`mints`／`decision_name`／`explicit_current_rule`
  （`explicit_current_rule` 在 bin 的 `mod tests` 里）——**函数体搬进 lib 而调用方分处两个 crate**，
  故这些定义必须跟着搬进 lib 并取 `pub`（bin 看不见 `pub(crate)`）。**可见性加宽是搬家的必然后果**，
  语义一字未改；判据是 Task 2 Step 6 的「命令路径行为不变」那批既有用例**一条都不许改**。
  （**一处例外**：`policy_context_for_effect` 收窄为 `pub(crate)`——bin 侧只在注释里提它，没有代码取用；
  这一处由实现者据实处置并写明，见其文档。）

- [ ] **Step 1: 把 `TaskError` 搬进 lib，变体一个不改**

`task_cmd.rs:874-959` 的 `TaskError` **整块**移进 `src/error.rs`，变体、字段、`#[error(...)]` 文案、
文档注释**一律照录**。`lib.rs` 加 `pub mod error;` 与 `pub use error::TaskError;`；
`task_cmd.rs` 改 `use continuum_runtime::TaskError;`。**`main.rs` 不需要这个 `use`**——
它只经 `task_cmd::run` 的返回类型**间接**用到 `TaskError`，不做名字绑定。

> **订正（2026-10-07）**：本处原文还带半句佐证——「`grep -n TaskError crates/continuum-runtime/src`
> 除 `task_cmd.rs` 外零命中」。**该半句已删，理由不是它今天错，而是它会随本计划自己的改动变假**：
> Task 2 新加的 `src/error.rs` 是它的定义处，Task 3 的 `src/tool_call.rs` 又整篇引用它，
> 故「除 `task_cmd.rs` 外零命中」到 Task 2 之后就不再成立。**当一条 `grep` 只用来「说明某处为什么
> 不需要某样东西」时，它不该以「零命中」的形式出现**——它数的是**全仓文本**（开放语料），
> 而写它的这份计划正是要往那个语料里加字的那一份。留下的语义理由（「只经返回类型间接用到」）
> 与语料无关，故不受影响。

**被否掉的替代**（设计 §3.2，记此免得后来者重提）：另建一个 lib 侧的 `ToolCallPathError` + bin 侧逐变体
映射——那会给同一批失败造出**第二个错误类型**，正是本项目判为 Critical 的「同一件事两个类型」。

- [ ] **Step 2: 把 `sandbox_select` 一并搬进 lib**

`TaskError::SandboxSelect(#[from] SandboxSelectError)` 里的 `SandboxSelectError` 定义在
**`src/sandbox_select.rs`**，而该模块今天由 `main.rs` 的 `mod sandbox_select;` 声明，**属 bin**；
`TaskError` 要搬进 lib 又「变体一个不改」，故**该模块整体搬进 lib**：`lib.rs` 加
`pub mod sandbox_select;`，`main.rs` 删掉那一行，`task_cmd.rs` 改
`use continuum_runtime::sandbox_select::{self, SandboxSelectError};`，
`sandbox_select.rs` 内部的 `use continuum_runtime::cli::SandboxMechanism;` 改成 `use crate::cli::…`。

**替代处置（否掉）**：把变体改成 `SandboxSelect(String)`——那既违反「变体一个不改」，又把一个带类型的
错误降级成字符串。**该模块自带 `#[cfg(test)]` 单元用例**，随模块一起搬，用例内容不动。

- [ ] **Step 3: 搬时钟与常量，并提取无事务共享函数**

`now_millis`（`task_cmd.rs:867-872`）与 `CAPABILITY_LIFETIME_MS`（`task_cmd.rs:174-188`）移进
`src/tool_call.rs`，**`pub`**（bin 的 `task_cmd` 还在用 `now_millis`）。两者**只有一份定义**：
`expiry` 与 `planned_at` 若取自两份不同的时钟读数，那是同一件事两个产生点。

把 `authorize_declared_effects`（`task_cmd.rs:442-472`）**提取并加宽**成：

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

### Task 3: `tool` 子命令的解析面、工具调用路径与 bin 接线

> **硬依赖 C**（`ProviderRegistry` / `invoke_tool` / `ToolCallError` / 新请求类型）。
>
> **三件事必须同批落地，这是本 task 的硬约束**：`cli.rs` 加 `Command::Tool` 变体之后，`main.rs` 的
> `match cli::parse(args)`（**锚点是那条 `match cli::parse(args)`**，只有 `Recover` / `Task` / `Err`
> 三臂）**不同步加臂就是 `error[E0004]`（非穷尽）**。**故本 task 的自检含
> `cargo build --workspace --all-targets`。**
> 同理，`tool_cmd` 要调 lib 的 `run_tool_call`，而 `run_tool_call` 要用 `cli::ToolArgs`——三者环环相扣，
> 拆开必然出现「跑不绿」或「留桩」的中间态。
>
> **订正（2026-10-07，回灌 F Task 3 的实测）——原句在这里写的是**：「而 `cargo test --test cli`
> **单独能过**（**它不构 bin**）——自检若只跑那一条，这个缺口会滑过去」。**那句是假的**：**实测该命令
> 会构 bin**（cargo 为集成测试构建本包的 bin 目标，集成测试的 `CARGO_BIN_EXE_<name>` 依赖它），
> `E0004` **在它上面就现形**。**旧话留此，免得后来者照它得出「测试看不到编译缺口」的错结论。**
> **「守什么」与「为什么」要分开说（这是本节要留的那句）**：
> - **守的是什么**：`main.rs` 的 `match` 必须加 `Command::Tool` 臂（M1 的判别力，见 Step 10 的表）；
> - **为什么还留着这条守卫**：它是**workspace 级、`--all-targets`、0 warning** 的构建闸
>   （Global Constraints 的那一条），覆盖面比「单跑一个包的单个测试目标」宽（其余成员、其余 target，
>   以及本项目要求的 0 warning）。**它不是「唯一能看到 `E0004` 的命令」**——原句那么说，是错的。
> - **缺口的位置**（原句判错的地方）：`E0004` 是**编译期**缺口、落在 **bin 目标**里，
>   **任何会构 bin 的命令都会暴露它**，与「测试目标构不构 bin」无关。
>
> **同一条假句子还活在实现里**：Task 3 的实现在 `crates/continuum-runtime/src/main.rs` 的
> `Command::Tool` 臂注释里照抄了原计划的口径（「`cargo test --test cli` 单独能过（它不构 bin）」）。
> **那一处须由实现者另行订正，本计划不代改 `crates/`**——记此以免它躲过下一轮扫查。
> **这条待办的带时刻读数**（本仓「引用＝文件＋定位」的用法）：**在 `b367356` 这棵树上**，
> `crates/continuum-runtime/src/main.rs` 的 **blob sha 是
> `adcf3b9ffbab2884ed4a282f9d1a910624a1c571`**，那句假话在 **`:62`**；`git log -- <该文件>`
> 在 `dfefb54` 之后再无提交。**故它是一条真待办，不是指向已完成事项的过期指针。**
> **下一轮扫查若得出「零命中」，先核 blob sha**——对不上就说明**读错了树／分支，或根本没读输出**
> （本机的 cwd 会在主检出与各 worktree 之间翻、分支也会换；命令把命中行打了出来而结论写成「零命中」
> 也真实发生过一次）。**别照着一次读数把这条删掉——以本行的 blob sha 为准。**

**Files:**
- Modify: `crates/continuum-runtime/src/cli.rs`
- Modify: `crates/continuum-runtime/src/tool_call.rs`
- Modify: `crates/continuum-runtime/src/error.rs`
- Modify: `crates/continuum-runtime/src/main.rs`
- **Modify: `crates/continuum-runtime/src/task_cmd.rs`**（**订正 2026-10-07：原 Files 清单漏了这一条**，
  而它**必然**被改。三处：① `effect_key` 与 `authorization_field` 的**定义**搬进 lib 的 `tool_call`
  ——工具路径的调用点在 **lib**、定义在 **bin**，lib 叫不出 bin 的名字，就地再写一份就是同一件事两个
  产生点，故本文件改为 `use continuum_runtime::tool_call::{effect_key, authorization_field, …}`；
  ② `open_db` 提 **`pub(crate)`**（`tool_cmd` 与 `task_cmd` 都是 bin 的模块，`pub(crate)` 即够）；
  ③ 随 ① 搬走的单元用例（`the_effect_key_separates_the_intent_from_the_target`）与本文件里
  不再使用的导入。**实测**（`git show --numstat dfefb54`）：`crates/continuum-runtime/src/task_cmd.rs`
  是 **37 加 / 67 删（合计 104 行）**。）
- Create: `crates/continuum-runtime/src/tool_cmd.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`（dev-dependencies 加 `async-trait`）
- Modify: `crates/continuum-runtime/tests/cli.rs`
- Create: `crates/continuum-runtime/tests/tool_call.rs`
- Modify: `Cargo.lock`（**订正 2026-10-07：原 Files 清单也漏了这一条**。dev 边写进锁里一行
  `"async-trait"`，实测 `dfefb54` 的 `Cargo.lock` 为 `1 +`；见 Global Constraints 与 Step 11）

**Interfaces:**
- Consumes: `continuum_provider::{ProviderRegistry, ToolCallError, ToolProvider}`、
  `continuum_capability::{authorize, AuthorizedTool, CapabilityError}`、Task 2 的 `mint_declared_effects`
- Produces: `continuum_runtime::cli::{Command::Tool, ToolArgs}`、
  `CliError::{InvalidToolInput, OptionRequiresEffect}`、
  `continuum_runtime::tool_call::run_tool_call`、
  `TaskError::{Capability, ToolCall}`

- [ ] **Step 1: 写解析面用例（红）**

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
- `both_options_without_an_effect_report_the_intent`（**次序的那条照片**）：`--intent i1 --approve`
  **同时给出**、零 `--effect` → `option == "--intent"`。**没有这一条，次序承诺就没有照片**（前一条的
  两条断言各只给一个选项，钉不住次序）。
- `a_tool_call_without_any_effect_parses`（**零效应的正常侧**）：`--tool t1`（无 `--effect`/`--intent`/
  `--approve`）→ `Ok`。
- `tool_rejects_the_task_options`（P-14）：`--base` / `--exec` / `--apply` / `--sandbox` **各一条**，
  都 `Err(CliError::UnknownOption { name })` 且 `name` 等于该选项名。
- `tool_requires_db_and_tool`：两个必填项缺任一 → `CliError::MissingOption`（**两条**）。

**红的条件**：今天的 `parse` 只有 `task` / `recover` 两个臂，`"tool"` 落到 `UnknownSubcommand`。

- [ ] **Step 2: 实现解析面**

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

**判定次序写死**：零效应时若两个选项**同时**给出，报 `--intent`（按选项表次序先判）——照片是
`both_options_without_an_effect_report_the_intent`。

`--base` / `--exec` / `--apply` / `--sandbox` **不进 `tool` 的 `match`**，故自然落到既有的
`UnknownOption` 臂——**不要为它们写专门的臂**（那是同一件事两个产生点）。`USAGE` 同步补 `tool` 的用法行。

- [ ] **Step 3: 定 `run_tool_call` 的签名，并先读 C 的源码核对**

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

- [ ] **Step 4: 写库级用例（红）**

`tests/tool_call.rs`：库级用例，夹具适配器经 C 的注册表登记。**夹具的形状照
`crates/continuum-provider/tests/fake_provider.rs`**（跨 crate 的 `tests/` 目录不可导入，故这是**第二份
副本**——**两份不合并**，判据见 `p3bcdf-followups.md` §七第 8 条：C 的那一份钉的是「工具级失败走
`Ok(is_error: true)`、provider 级失败走 `Err`」这条约定，F 的这一份只服务本路径的用例，两份各有其用）。
夹具要记两件事：**被调用的次数**与**收到的那份请求**。

> **调用点的唯一所有者是 C（判据逐条抄自 `p3bcdf-followups.md` §七第 5 条）**：
> **`AuthorizedToolInvocation` 的构造点 → 唯一所有者 C（在 `invoke_tool` 之内）。
> F 只调 `ProviderRegistry::invoke_tool(&AuthorizedTool, input)`，不构造、也不命名那个请求类型。**
> 出处：「C 设计 §7.5 定的是那个类型**长什么样**，§7.1 定的是**谁构造、经哪条路调**」。
> 故 F 的**生产路径**只做一件事——把 `&AuthorizedTool` 与 `input` 交给 `invoke_tool`；
> **不自己 `new` 出请求，也不在别处出现第二个产生点**。
>
> **一处措辞要写准（与上条不冲突）**：实现 `continuum_provider::ToolProvider`（`#[async_trait]`）时，
> `impl` 的签名里**必须写下**那个请求类型（`AuthorizedToolInvocation<'_>`，C 设计 §7.5）——Rust 要求
> `impl` 侧与 trait 侧同形。上一条的「不命名」管的是**生产调用点**，不是夹具的 `impl` 签名；
> 记此以免实现者以为自己违规，也以免它被读宽成「夹具不许提这个名字」。

用例：

- `the_registry_is_the_only_way_the_tool_is_reached`（P-6）：登记夹具后跑一条放行的调用，断言夹具**收到
  一次调用**。**红的条件**：把步骤 6 换成「什么都不做、直接返回 `Ok`」→ 本条红。
- `the_tool_id_comes_from_the_authorized_proof`（P-7）：断言夹具收到的那份请求里，那枚授权证明的
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

- [ ] **Step 5: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
```

- [ ] **Step 6: 实现 `run_tool_call` 的七步**

次序与设计 §3 的表逐条对齐：

1. **幂等键预检**：逐条 `--effect` 用 `effect_key(intent, spec)`（**复用，不另派一次**；
   **订正（2026-10-07）：这个函数在本 task 搬进 lib 的 `tool_call`**——原稿写「`task_cmd.rs:512`」，
   那是在 Task 3 之前的树上数出来的**定义处**，而工具路径在 lib、定义在 bin 时它调不到，
   故定义随本 task 一并搬家，`task_cmd.rs` 改为 `use` 它（见本 task 的 Files 注）。**锚点用函数名**。）
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
   编码形状**复用**；**订正（2026-10-07）**：原稿写「`task_cmd.rs:537` 的既有编码形状」，
   该函数与 `effect_key` 同批搬进 lib 的 `tool_call`（Tasks 的 Files 注），**锚点用函数名**），
   `parameters` 填 `json!({})`（照 `task` 的先例），
   `tx.commit()`。**步骤 4 与 5 用同一个事务**——这正是设计 §3.2 那条承重性质（审计行与 `EXECUTING`
   行要么都在、要么都不在）的来源。
6. **那一跳**：`block_on(registry.invoke_tool(&authorized_tool, args.input.clone()))`。运行时用
   `tokio::runtime::Builder::new_current_thread().build()`——**不要用 `enable_all()`**：workspace 的 tokio
   features 是 `["rt-multi-thread", "macros"]`，`enable_all` 按 `net`/`time` 等 feature 门控，可能不可用；
   换 feature 会动 `Cargo.toml`，与「不动内部依赖清单」相抵。**若确需 `enable_all`，停下来回报**，
   不要就地改 feature。失败（含 `Unregistered`）→ **各效应记 `FAILED`**（与 `task` 第 5 步机制选择失败
   时的处置一致），再把 `TaskError::ToolCall(..)` 报出去。
   （**一处设计侧的引用勘误**：设计 §3.4 把 `task` 第 5 步的 `Sandbox::spawn` 记在 `task_cmd.rs:252`，
   实际 252 是 `sandbox_select::select_for_this_machine` 那一行，`sandbox.spawn(task, cmd)?` 在 `:588`。
   本计划不引 252，实现者按源码找。）
7. **终态与 stdout**：按 `ToolResult.is_error` 写 `COMMITTED` / `FAILED`；**无论 `is_error` 为何，先把
   `output` 以 JSON 一行打到 stdout**（设计 §3.3：工具调用**没有子进程**，stdout 是调用方仅有的通道）。
   本 task 先只做 `is_error == false` 那一支与打印；`is_error` 那一支与 `TaskError::ToolReportedError`
   在 Task 6 补。

**`TaskError` 加两个包装变体（设计 §8 的失败面表要求，本 task 落地）**：

```rust
/// 强制点 (1) 的失败（工具未登记、出示/缺失能力、能力失效、读登记项或写审计失败）。
/// **只是包装**：把 `continuum-capability` 的错误原样带出去，不重判、不合并变体。
#[error("工具授权失败：{0}")]
Capability(#[from] CapabilityError),

/// 工具侧唯一入口（`ProviderRegistry::invoke_tool`）的失败：路由未命中
/// （`Unregistered`）或适配器自己报错（`Provider`）。**只是包装**——F 不另立
/// 「没有适配器」之类的词汇（同一件事两个变体正是本设计在讲的同一类毛病）。
#[error("工具调用失败：{0}")]
ToolCall(#[from] ToolCallError),
```

`error.rs` 因此需要 `continuum_capability::CapabilityError` 与 `continuum_provider::ToolCallError`
（两者都在 `Cargo.toml` 的既有依赖里，**不动 `Cargo.toml`**）。

**不要做的事**（写了就是发明）：不建 Task 工作区、不经 `Integration Gate`、不写任何审计行、不读
`Tool::effect_class`、不自己开第二个工具调用入口。

- [ ] **Step 7: 写 `tool_cmd.rs` 与 `main.rs` 的分派臂**

`tool_cmd.rs`（bin）**做两件事、并把 `Err` 原样交给 `main`**，**不含路径逻辑**：

1. **装配**：`open_db`（**锚点是 `fn open_db`**，bin 的 `task_cmd.rs`；提到 **`pub(crate)`**，
   两条子命令共用同一份迁移集合——两处各写一份清单会让「注册的集合」有两个来源）
   + 构造一个 `ProviderRegistry`。**今天的驱动没有任何
   适配器可登记**，故**装配点是空的**——这一句要写成注释并说明它不是遗留物（缺的是**一个可登记的
   适配器实现**，而装配者＝驱动自己，收件人是驱动自己／将来的适配器子项目，见 `## 遗留`；
   **不是 C 的交付缺口**——C 已交付注册表机制，且按中立性规则**不会**在自己 `src/` 里提供 `impl`；
   **这里要说准**：**不是「类型上写不出来」**，而是一条规则 ＋ 一条**可执行的**模块面守卫
   （`continuum-provider/tests/neutrality.rs`），且该守卫自称**下界**——见设计 §10.2 那条订正，
   **原稿在此写「不可能提供 `impl`」已订正**）。**登记的实参表以 C 的源码为准**，本计划不写死它（设计 §10.2 明写 F 不预先发明）。
2. **调 lib**：`continuum_runtime::tool_call::run_tool_call(&db, &registry, &args)`，把它的 `Result` 原样返回。
   **不要把 `ToolResult.output` 在这里打印**——它在 lib 的步骤 7 已经打出（两条路径不能有两个输出点）。

**订正（2026-10-07，回灌 F Task 3 的实测）——原稿在这里自相抵**：上面原本写「`tool_cmd.rs`（bin）只做
**三件事**」，第 3 件是「**失败映射**：`Err(e)` → `eprintln!("工具调用失败: {e}")` + `ExitCode::FAILURE`」；
而下面那段又要求 `main.rs` 的新臂**与 `Task` 臂同形**（`Task` 臂的形态就是「`match task_cmd::run(..)`
里 `eprintln!` + `ExitCode::FAILURE`」）。**两者不能同真**：失败映射要么在 `tool_cmd` 里、要么在 `main`
臂里，写成两处就是同一次失败的两个打印点。
**取「同形」一侧（实现者的处置，本计划据实订正为它）**，理由：`main.rs` 的 `Task` / `Recover` 两臂
**都已经**是「子命令模块返回 `Result`、分派臂打印并给退出码」这个形状（`main.rs` 里那三个臂逐字同构）；
**只有 `tool` 一个子命令把映射放回模块里，就成了三个臂里的孤例**——同一件事（子命令失败怎么报）两个
形状。故：**`tool_cmd::run` 不打印任何东西，`Err` 原样返回**；失败映射落在 `main` 的分派臂，
文案 **`工具调用失败: {e}`**。

`main.rs`：加 `mod tool_cmd;`，并在 `match cli::parse(args)` 里加一个与 `Task` 臂同形的
`Command::Tool(a) => match tool_cmd::run(&a) { Ok(()) => ExitCode::SUCCESS, Err(e) => { eprintln!("工具调用失败: {e}"); ExitCode::FAILURE } }`。
**「同形」指的是这一臂的整块形状（含打印与退出码）与 `Task` 臂一致**——这正是上一段取「同形」一侧的
落点。**这一臂与 `cli.rs` 的变体必须同批**（见本 task 开头的硬约束）。

- [ ] **Step 8: 全量构建（**M1 的守卫；`--all-targets` 的 workspace 级构建闸**）**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：**0 warning、编译通过**。这一步抓的正是「加了枚举变体没加 `match` 臂」那类缺口——`E0004` 是
**编译期**缺口、落在 **bin 目标**里，故**任何会构 bin 的命令都会暴露它**。

**订正（2026-10-07，回灌 F Task 3 的实测）——「守什么」与「为什么」是两件事，「为什么」这一半要重写**：

- **守的是什么**：`main.rs` 的 `match` 必须加 `Command::Tool` 臂（M1 的判别力，见 Step 10 的表）。
  **这一半不变。**
- **为什么要有这条守卫（重写后）**：它是**workspace 级、`--all-targets`、0 warning** 的构建闸
  （Global Constraints 的那一条：`cargo build --workspace --all-targets` 0 warning），覆盖面比
  「单跑一个包的单个测试目标」宽（其余 workspace 成员、其余 target 形态，以及本项目要求的 0 warning）。
- **原句的哪一半是假的**：原写「（`--test cli` 单独能过）」——**实测该命令会构 bin**，
  `E0004` **在它上面就现形**。故**不能**用「`--test cli` 不构 bin」当这条守卫的理由：
  它不是「唯一能看到 `E0004` 的命令」。**旧话留此**，免得后来者靠一句假的理由去判断守卫的覆盖面。
- **同一条假句子还活在实现里**：Task 3 的实现在 `crates/continuum-runtime/src/main.rs` 的
  `Command::Tool` 臂注释里照抄了原计划的口径。**那一处须由实现者另行订正**（本计划不代改 `crates/`）。
  **带时刻的读数**：**在 `b367356` 这棵树上**，该文件的 blob sha 是
  `adcf3b9ffbab2884ed4a282f9d1a910624a1c571`、那句假话在 **`:62`**；`git log -- <该文件>` 在
  `dfefb54` 之后再无提交。**故它是真待办**；下一轮扫查若得「零命中」，先核 blob sha 再下结论
  （见本 task 开头那条订正块的完整说明）。

- [ ] **Step 9: 跑测试，确认转绿**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test cli
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_call
timeout 1500 cargo test --workspace --no-fail-fast
```

- [ ] **Step 10: 变异**

| 变异 | 期望 |
|---|---|
| 步骤 6 换成「直接 `Ok(())`」（不调注册表） | P-6 与 P-17 红 |
| `presented` 换成空集 | **实测 P-6 红**（**订正**：原写「Task 4 的 P-4 / P-5 红（**P-6 仍绿**）」，**那一行三点都错**，见下面的订正块）。同一机制下 Task 3 里其余**带 `--effect`** 的用例（如 P-7）也红；零效应的 P-17 **仍绿**——两半**都要在** |
| `input` 换成 `json!({})` 常量 | `the_input_reaches_the_adapter_verbatim` 红 |
| `main.rs` 的 `Command::Tool` 臂删掉 | `cargo build --workspace --all-targets` 报 `E0004`（**这不是「变红」而是编译失败**，记此只为证明 Step 8 的守卫有判别力） |

> **订正（2026-10-07，回灌 F Task 3 的实测）——上表第 2 行（`presented` 换成空集）原稿预测错，且与
> 本 task Step 1 自相抵，两点一并写明：**
>
> **(a) 为什么 P-6 会红（实测）。** P-6（`the_registry_is_the_only_way_the_tool_is_reached`）是一条
> **有效应**的调用：它的登记项声明了 `CapabilityKind::for_effect(EffectType::Charge)`，调用带一条能铸出
> 该 kind 的 `--effect`。`presented` 一换成空集，`authorize` 的两向合取里「**声明集 ⊆ 出示集**」这
> 一半必败（登记项声明的那枚能力在空出示集里找不到），于是**先**以 `MissingCapability` 拒——P-6 的
> 「放行的调用应当成功」那一步就此红。**道理是两条**：有效应 ⇒ 登记项必须声明该效应铸出的能力
> （否则更早被 `UndeclaredCapability` 拒）；出示集一空 ⇒ 必被 `MissingCapability` 拒。故
> 「**有效应调用 ＋ 空出示集**」下 **P-6 不可能仍绿**。**原稿那半句是假的。**（同一机制下，Task 3 里
> 其余带 `--effect` 的用例——如 P-7 的夹具同样声明 `Charge` 并用同一条放行调用——按用例正文即可判
> 定也红；**零效应**的 P-17 则不受影响、仍绿。）
>
> **(b) 它还与 Step 1 自相矛盾。** Step 1 的 P-17 条目（`an_effect_free_call_still_reaches_the_tool`）
> 把 P-17 立成「**P-6 的对照臂**」，并写明其红条件是「**一个『有效应才调用』的实现本条红，而 P-6 仍
> 绿**」——**那句话要求 P-6 是一条有效应调用**（否则「有效应才调用」这个实现与 P-6 的差异无从显现）。
> 而本行说 `presented` 换空集后「P-6 仍绿」，**在 P-6 是有效应调用时不可能成立**（理由见 (a)）。
> **两句相抵，只有 Step 1 那句对**，实测亦证实它。
>
> **(c) 这一行还指错了对象。** 原稿把期望写成「Task 4 的 P-4 / P-5 红」——那两条**到 Task 4 才写**，
> **Step 10 跑变异时它们还不存在**，拿一条尚未落地的用例当变异判据本身就不成立。该变异体真正落在
> **Task 3 已写的、带 `--effect` 的用例**上（P-6 / P-7 一类）。
>
> **结论（保留下来的那半句）**：第 2 行与第 1 行（「步骤 6 换成直接 `Ok(())`」）**仍要都在**——
> 两者**互不包含**：空出示集让**带效应的调用**红而**零效应的 P-17 绿**；「直接 `Ok(())`」让**两者都
> 红**（零效应的 P-17 也到不了适配器）。故**没有任何一条能代表另一条**。原稿的**结论**（两组都要在）
> 对，**理由要按上面重写**。

- [ ] **Step 11: 提交**

```bash
git add crates/continuum-runtime/src crates/continuum-runtime/tests/cli.rs \
        crates/continuum-runtime/tests/tool_call.rs crates/continuum-runtime/Cargo.toml \
        Cargo.lock
git commit -m "feat(runtime): tool 子命令的解析面、工具调用路径与 bin 接线"
```

**订正（2026-10-07，回灌 F Task 3 的实测）——`Cargo.lock` 是原清单漏掉的一项**：本 task 加了
`async-trait` 的 dev 边，**dev 边也写进锁文件**（`[[package]] continuum-runtime` 的 `dependencies`
多一行 `"async-trait"`），**不 add 它锁就过期**（本地 `cargo build` 会把它改出来，而它不在这条
`git add` 里 ⇒ 提交里的锁与 `Cargo.toml` 不一致）。**实测**：`dfefb54` 的 `Cargo.lock` 改动为 `1 +`。
**这一项不是「顺手带上」**：它是本 task 改动的**必然产物**（见 Global Constraints 那条订正）。

---

### Task 4: 强制点 (1) 的照片

> 本 task **只加用例与变异**，不改 `run_tool_call` 的语义（它在 Task 3 已经全量落地）。
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

  **这六条登记项的 `effect_class` 必须取 `None`**：取 `Some(t)` 会与本计划 Task 1 的新不变量相抵
  （`Some(t)` 而能力表不含 `for_effect(t)` 时 `save_tool` 直接拒），那会让本条在登记期就红，
  验不到本路径的 `MissingCapability`。

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

### Task 5: Journal 与审计的照片

**Files:**
- Modify: `crates/continuum-runtime/tests/tool_call.rs`

- [ ] **Step 1: 写用例**

- `the_intent_is_observable_in_the_idempotency_key`（P-2 的第二半）：先跑一次成功调用（带 `--intent i1`），
  读回那条 `effect` 行，断言 `idempotency_key` 形如 `<意图字节长>:<意图>:<类型>:<目标>` 且第一段就是 `i1`
  （键形见 `effect_key`；原稿引 `task_cmd.rs:512-520`，该函数随本 task 的 Task 3 搬进 lib 的
  `tool_call`，见 Task 3 的 Files 注）。**这一读是设计 §2.2 那条「`--intent` 是可观察的」的照片**——没有它，
  「条件必填」的理由就只是一句话。
- `a_repeated_declaration_is_rejected_and_writes_nothing`（P-2 的第一半）：再跑同一条声明 →
  `TaskError::EffectAlreadyRecorded { key }`（**复用**），且**零新行**。
- `the_audit_rows_are_exactly_one_grant_plus_four_per_effect`（P-15）：读 `audit_log.kind` **列**，
  k=1 时 multiset 恰为 `{capability grants × 1, external effects × 4}`、共 **5** 条；
  **k=0 时恰 1 条**（`capability grants`）。两处 `kind` 的值**手写字面量**（钉格式）；
  `4 = 1 次登记 + 3 次推进`（同 `continuum-effect/tests/persist.rs:252` 的既有注释，`assert_eq!` 在 `:253`）。
- `an_effect_free_call_touches_neither_the_effect_table_nor_the_mint`（P-16）：零 `--effect` → `effect`
  **0 行**、`audit_log` **恰 1 条**（`capability grants`），**且没有凭据捏造的「tool invoked」之类审计行**
  （§313 的八项是封闭清单，本子项目**不新增 `AuditKind`**）。
- `each_declared_scope_is_carried_into_the_audit_payload_verbatim`（P-18）：声明两条目标各不相同的
  `--effect`，跑完之后读 `audit_log` 的 `capability grants` 行、解析 payload，断言
  `capabilities[].scope` 的 multiset **逐个等于**各 `--effect` 的目标。

> **P-18 / P-15 为什么在「注册表为空」的世界里也成立**：审计行由步骤 4 的 `authorize` 写，与效应行在
> **同一次提交**（步骤 5）里落库——**早于**步骤 6 那一跳。故即便那一跳以 `Unregistered` 失败，
> 审计行与四条 `external effects` 已经在库里。这两条的库级版本在本 task，端到端形态在 Task 7。

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

### Task 6: 剩余失败面与两处刻意分岔

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
再返回本变体。**不新增任何别的变体**——Task 3 已落的两个包装变体（`Capability` / `ToolCall`）之外
不再加；尤其**不要**为「没有适配器」另立一个（那是 C 的 `ToolCallError::Unregistered`，F 包装而不另起
词汇），也不要加「裁决为 `Deny`」这类重复既有判断的变体（那是 `mints` 的判断）。

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

### Task 7: 端到端（真实二进制）

**Files:**
- Create: `crates/continuum-runtime/tests/tool_cli.rs`

- [ ] **Step 1: 写用例**

`tests/tool_cli.rs` 走 `env!("CARGO_BIN_EXE_continuum-runtime")`（与 `task_cli.rs` 同法）：

- `an_unparsable_input_fails_at_parsing_time`（P-12 的端到端那一半）：`--input '{'` → 退出码非零、
  stderr 含 `--input`；**库未被创建**（`--db` 指向的路径不存在）。
- `the_option_group_is_rejected_in_both_directions`（P-13）：两向各一条端到端用例。
- `the_task_only_options_are_unknown_here`（P-14）：`--base` / `--exec` / `--apply` / `--sandbox`
  **四条各一条**。
- `a_registered_tool_is_still_unrouted_and_the_call_fails_at_the_last_hop`：登记一条工具（经
  `save_tool`，测试里直接开库写）后跑 `tool` → **退出码非零**，且 **`audit_log` 的 `capability grants`
  已有 1 条**（步骤 4 已提交）、`effect` 行数等于声明的效应条数。**不要断言 stderr 的具体文案**：
  那是 C 的 `ToolCallError::Unregistered` 的 `Display`，F 没有冻结它（`F 未冻结它` 这一点记在 `## 遗留`）。
  这条同时是「强制点 (1) 在**正常路径上**被执行」的端到端照片，也是设计 §12 那条「有生产调用方」判据的落点。
- `the_declared_scopes_reach_the_audit_payload`（P-18 的端到端形态）：读 `audit_log` 的 payload，
  逐项断言 `capabilities[].scope`。**库级版本在 Task 5**，此处只钉「真二进制也走这条路」。

**红的条件**：把这些用例跑在 Task 3 之前的字节上（或把 `main.rs` 的分派臂摘掉）→ 全红。

- [ ] **Step 2: 跑，确认转绿**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test tool_cli
timeout 1500 cargo test --workspace --no-fail-fast
```

- [ ] **Step 3: 提交**

```bash
git add crates/continuum-runtime/tests/tool_cli.rs
git commit -m "test(runtime): tool 子命令的端到端用例"
```

---

### Task 8: 强制点 (1) 的射程边界（P-19）与 `dependency_direction.rs` 注释订正

**Files:**
- Modify: `crates/continuum-runtime/tests/task_cli.rs`（P-19 的落点）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**只订正 `:119-121` 的注释**）

- [ ] **Step 1: 写 P-19（否定式照片，**不受沙箱门控**）**

`task_cli.rs` 新增 `the_command_path_never_reaches_the_first_checkpoint`：

跑 `task --effect charge:x --exec true`（策略放行），随后断言
`SELECT COUNT(*) FROM audit_log WHERE kind = 'capability grants'` 为 **0**。

**为什么不受 `require_auto_selected_sandbox` 门控**：这条断言的承重处只有一处——**库里的行数**，
而步骤 4（`record_declared_effects`，写效应行之处）**早于**步骤 5 的沙箱选择。故即便本机选不出沙箱、
`task` 在第 5 步失败，效应行与「若接了强制点 (1) 就会写下的审计行」都已经在库里，行数断言照样有判别力
（本仓没有删除 `effect` 行的路径，清理路径只回收工作区与记录）。**退出码不作为本条的判据。**
**跑不到步骤 4 的机器**（例如库根本打不开）用既有的 `skip()` 显式跳过并报执行/跳过条数，不静默通过。

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

### Task 9: 收口（复核三处合并结果）

> 串行执行序的最后一位做一遍收口（前一阶段 P3A 就是这么收的）。**本 task 只复核、只在发现不一致时改**。
>
> **本 task 的判据直接抄自 `docs/superpowers/p3bcdf-followups.md` §七**（接缝裁决的权威转录），
> 逐条如下——**本 task 不依赖任何 gitignore 的文件**：
>
> - **第 7 条**：多写者单文件（`dependency_direction.rs` 的 `ALLOWED`、`main.rs` 的注册、workspace members）
>   由**每份计划各自登记自己那几条**，不设集中登记 task，串行执行保证不冲突；**F 计划的最后一个 task
>   是收口**——复核三处的并集并附 `cargo test --workspace` 的证据。
> - **第 2 条**：`runtime → secrets` 的边与 `SecretsRuntime` 的装配 → **所有者 B**（凭据交付是 B 的
>   §124 路径）。故 B 在 runtime 条目上加的 `continuum-secrets` 与它在 `main.rs` 的装配**都在本步的复核面内**。
> - **第 3 条**：`runtime → connector` **本轮无生产消费者 → 落成具名未决，不做 task**。**不加这条边。**
> - **第 4 条**：D 的迁移注册 → **所有者 D，注册在 `main.rs`**（「谁的表谁注册」；原先记在「用它的那个
>   task（＝子项目 G，本轮不存在）」名下是一条**空头收件人**）。**迁移号须在 D 的 task 里现场核对该库的空号。**

**Files:**
- Modify: 视复核结果而定（预期不改任何文件）

- [ ] **Step 1: 复核 `crates/continuum-runtime/tests/dependency_direction.rs` 的并集**

逐条核四件事：① **B 的 `continuum-connector` 自有条目**（B 建这个 crate，故它作**主体**要有一条）；
② **B 在 runtime 条目上加的 `continuum-secrets`**（§七第 2 条：`runtime → secrets` 的边与
`SecretsRuntime` 的装配都归 B）；③ D 的 `continuum-model-registry` 自有条目，以及 D 在 runtime 条目上
加的那条；④ C 的 `continuum-provider` 条目（三元素）。**F 自己不改 `ALLOWED`。**

> **不要把这句读成要加 `runtime → connector`**：**本轮不加这条边。** §七第 3 条已裁「`runtime → connector`
> 本轮无生产消费者，落成具名未决、不做 task」——B 的 `continuum-connector` 只在 `ALLOWED` 里**作为主体**
> 有自己的一条，**不作为 runtime 的依赖**出现。若复核时在 runtime 条目里看见 `continuum-connector`，
> 那是**多登记**，要当场处置。

判据是**逐对 `assert_eq!`**（含 dev 边）。跑：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
cd /home/DslsDZC/Continuum && cargo tree -p continuum-runtime --depth 1 --edges all --prefix none
```

**逐行比对** `cargo tree` 的输出与 `ALLOWED` 的 runtime 条目，**并集多一条少一条都要当场处置**。

**同时核 `crates/continuum-runtime/Cargo.toml`**：它的 normal 依赖清单必须与 `ALLOWED` 的 runtime 条目
**精确一致**（那张表的断言是逐对 `assert_eq!`，两处只改一处会红）。**本轮的 normal 新边由 B 与 D 各自
登记**（B 的 `continuum-secrets`、D 的 `continuum-model-registry`；**`continuum-connector` 不在其中**，
见上面的引用块），**F 一条 normal 边都不加**；F 只加 dev 边的 `async-trait`，而 dev 边不进 `ALLOWED`
的 normal 清单（`cargo tree --edges all` 会看到它，但 `async-trait` 是外部 crate，不在 `ALLOWED` 的
比较域内——比较域是 workspace 成员）。

- [ ] **Step 2: 复核 `main.rs` 的迁移与装配注册**

`runtime_migrations()`（`main.rs:57-66`）里是 **7 处 `extend` + 开头一处 `builtin_migrations()`**
（`let mut migrations = continuum_persist::builtin_migrations();` **不是 `extend`**）：
builtin + P1 两条（artifact、graph）+ P2 三条（workspace、policy、effect）+ P3 capability + D 的
model-registry。**按条数复核时别把 builtin 数成一次 `extend`。**
**D 的三张表注册在 `main.rs` 是 §七第 4 条的裁定**（「谁的表谁注册」，原先把这一步记在「用它的那个
task」名下——那等于子项目 G，本轮不存在，是一条空头收件人）；**迁移号由 D 在自己的 task 里现场核对
该库的空号**，F 只核「注册的集合」这一面。
与 `tests/migrations.rs` 的 `expected_migrations()` **互为覆盖**；复核 `tests/startup.rs` 的三处计数
（`:24` / `:90` 的「迁移应用 N 项」与 `:79` 的补应用数）与**行内注释**一致——B/D 各自改过，
**注释与断言互相打脸是本项目点过名的形状**。

**装配面同样在本步的射程内**（标题含「装配注册」就要真覆盖它）：`main.rs` 里除迁移外还有
**B 装配的 `SecretsRuntime`**（B 计划 Task 7）与 **F 的 `tool_cmd` 分派臂**。逐条核它们在
`main.rs` 里都到位、且 `mod` 声明与 `use` 路径正确（B 的装配若坏了，只在 F 的收口才看得出）。

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
| 强制点 (1) **有生产调用方** | Task 7 的 `a_registered_tool_is_still_unrouted_and_the_call_fails_at_the_last_hop`（真二进制走到步骤 4） |
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
工具登记无生产调用方     **截至本计划定稿时**（2026-10-05，分支 `p3cf`）：save_tool 只在测试里被调，
                        **生产代码零调用**；故真实库里 tool 表是空的，`tool` 子命令对任何 id 都报
                        UnknownTool，除非先经 save_tool 登记。
                        **该状态陈述带时点、也随分支可漂**：生产调用方的有无取决于别的子项目是否
                        新开一个登记入口，本计划不处置它、也不据此下任何断言（本计划**不发明**一个
                        登记用的 CLI）。**本计划不改这条。**
                        收件人：continuum-capability（本项目无此子项目）。
注册表里没有适配器       `ToolProvider` 在**生产代码里一个实现都没有**（`crates/continuum-provider/src/`
                        下零 `impl`，由中立性守卫钉住）；实现只在 `tests/` 下有几份夹具
                        （**实测 2026-10-07**：`tests/common/mod.rs` 的 `FakeTool` / `RecordingTool`
                        两份、`tests/registry_tools.rs` 两份；`tests/fake_provider.rs` 今天只
                        `mod common;` 再引用它们）。组合根登记不出东西，故步骤 6 那一跳
                        在生产路径上**必然**返回 ToolCallError::Unregistered。**这不是遗留物**，
                        它不挡强制点 (1)（authorize 在步骤 4，早于那一跳）。
                        **订正（2026-10-07，三处，旧话留此）**：本行原写「`ToolProvider` 的
                        **唯一实现**是测试夹具」——**「唯一」与「在哪个文件」两处都不准**
                        （实现不止一份、且 `fake_provider.rs` 里的那两份已移入 `tests/common/`）；
                        **承重的那半句仍成立**：生产代码里一个实现都没有。
                        **缺的是哪一步（说准）**：缺的不是注册表的**机制**（C 已交付它），而是
                        **一个可登记的适配器实现**——而装配者就是**驱动自己**（组合根，设计 §10.2；
                        **C 设计 §7.1 末段的原文是「装配者（composition root，即驱动自己）在装配期
                        构造适配器」**）。**原稿在此引「C 设计 §12.4 明写「装配者＝驱动」」——两处都
                        不对**：**C 设计没有 `12.4` 这一节**（§12 是按条编号的遗留清单，无子节号），
                        **且「装配者＝驱动」不是原文**；锚点改引 §7.1 末段、引文照录。
                        且 C 按中立性规则**不会**在自己 `src/` 里放 `impl`——**这里要说准**：这
                        **不是「类型上写不出来」**，而是一条**规则 ＋ 一条可执行的模块面守卫**
                        （`crates/continuum-provider/tests/neutrality.rs`），且该守卫自称**下界**
                        （全限定路径 / `use … as` 别名 / `include!` / `macro_rules!` 的 `$trait`
                        仍逃逸，见其文件头那张非穷尽的清单）。**原稿在此写「不可能在 `src/` 里放
                        `impl`」——「不可能」把「有规则且有守卫」写成了「做不到」，已订正。**
                        故这条**不指子项目 C**。
                        收件人：**F（驱动）自己**——准确说是**将来的适配器子项目 / 下一轮驱动**：
                        由它提供一个 `ToolProvider` 实现、经注册表的登记入口装进组合根。
                        F 本轮**不做**这一步（设计明写不建适配器），只把它记成具名未决。
C 的错误文案未冻结       Task 7 的端到端用例只断言退出码与库内行数，**不依赖
                        `ToolCallError::Unregistered` 的 Display 文案**——那段文案是 C 的，
                        F 无权冻结它。后来者若想断言文案，须先与 C 定死它。
granted() 的消费方      按 C 设计 §7.5，是**适配器**（在 invoke 的实现体内读授权证明、逐枚取
                        Capability::scope() 限定动作范围）。**F 不调它**，只把整枚 AuthorizedTool
                        经 invoke_tool 交出去。无照片（本阶段没有真实适配器）。
六个 kind 无策略事实     Filesystem(Read|Write)、Git(Read|WorktreeWrite|CommitLocal)、
                        Github(CreatePr) 在 EffectType 里没有对应项，故驱动铸不出它们，
                        含它们的工具在本路径上**一律** MissingCapability（Task 4 逐项拍了照片）。
                        **本路径不在登记期拦**（那会在别人的入口上装一道只有本层知道的口径）。
                        收件人：策略层（PolicyContext 的事实集合）与子项目 D。
作用域的**判定**        本路径不判作用域（authorize 只比 kind），强制落在凭据签发（§51）与执行点。
                        **原样带出**那一半有照片（P-18，Task 5 / Task 7）。
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
                        与 B 的密钥运行时接线一并做。**该边与 SecretsRuntime 装配的所有者是 B**
                        （p3bcdf-followups.md §七第 2 条），故本计划不提前登记、也不替它装配。
tokio 首次真使用         continuum-runtime/Cargo.toml 早已声明 tokio。**截至本计划定稿时**（2026-10-05）
                        `src/` 与 `tests/` 对它零引用；**本计划起使用**——为 block_on 异步的那一跳
                        第一次真用它（单次调用用当前线程运行时）。
                        **原话照留 ＋ 来历**：本行原写「而 src/ 与 tests/ 至今零引用」。**该说法自带到期日**
                        ——「至今零引用」是带时点的状态陈述，而写它的这份计划正是要改变那个状态的那一份，
                        故 Task 3 落地之后它就为假。**凡「至今没有」出现在「将要引入」的同一份文档里，
                        一律写成带时点的形式**（「截至……时」＋「本计划起……」）。
                        收件人：后续阶段（若驱动整体转异步，这个形状要重做）。
--input 省略即 {} /      两条都是本设计的**决定**，规范未规定，已由协调者本轮拍板接受；
--intent 条件必填        「--intent 可观察」已由 P-2 的第二半（Task 5）拍下来。
                        零效应时两选项同时给出的**判定次序**（报 --intent）是本计划定的实现细节，
                        规范与设计均无此判据，其照片是 Task 3 的
                        `both_options_without_an_effect_report_the_intent`。
AuthorizedTool 不进       TaskError::Capability 的 Display 只带内层 CapabilityError 的消息
错误上下文              （那些消息里没有作用域）。**这一条没有照片**（关于「驱动没写某句格式化」的
                        否定命题），记此以免后来者顺手加一句 {:?} 而没人发现。
effect_class 两轴 /      三条义务曾被 D 退件给「子项目 F ＋规范维护者」，而 F 的设计只声明「不读」。
trust / 工具侧           本计划**不做 task**（**协调者已裁，见 `p3bcdf-followups.md` §七第 6 条**）：
cost / latency          四份设计都写明「不发明」，而 §4.1 的组件表里**没有「工具选择」这个组件**，
                        故收件人是**规范维护者（本项目无此角色）**。
                        **这是「收件人挂了空」的第二次具名**，记此以免它再次无声挂空。
设计 §3.4 的一处行号错误 §3.4 把 `task` 第 5 步的 `Sandbox::spawn` 记在 `task_cmd.rs:252`；实际 252 是
                        `sandbox_select::select_for_this_machine`，`sandbox.spawn(task, cmd)?` 在 `:588`。
                        本计划不引该行号；实现者按源码找，勿照设计的行号去 252 找。

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
