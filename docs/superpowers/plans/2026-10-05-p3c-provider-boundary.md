# P3 子项目 C（Provider 中立边界）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在既有的 `continuum-provider`（只定义 trait、零实现）里补上**注册与发现**，把**中立性**落成会红的用例，
并完成 §316 请求面那次**已裁定的改动**——`invoke` 的请求参数换成只装得下「已授权」的 `AuthorizedToolInvocation<'_>`，
同批删掉失去生产调用方的 `continuum-core::tool::ToolInvocation`。

**Architecture:** **不新建 crate**（设计 §10），扩 `continuum-provider`：新增 `src/registry.rs` 承载
`ProviderRegistry`（模型侧 `ModelId → Arc<dyn ModelProvider>`、工具侧 `ToolId → Arc<dyn ToolProvider>`）与两个错误类型。
暴露面**两侧不对称**：模型侧交出裸 `Arc<dyn ModelProvider>`；工具侧**只开只读入口与唯一受门禁的调用入口**
`invoke_tool(&AuthorizedTool, input)`，**不交出适配器**（无 `tool_for`、无 `tool_providers()`）。工具id 只有一个来源——
授权证明的 `tool_id()`。代价是一条层内边 `continuum-provider → continuum-capability`（「接口 → 能力类型」，不是「接口 → 实现」）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-core`（`ModelId` / `ToolId` / `ToolDescriptor` / `ToolResult` / `ProviderError`）；
`continuum-capability`（`AuthorizedTool` / `authorize` / `ToolId`）；`async-trait`；`serde_json`；**`thiserror`
（本 crate 今天没有它，Task 1 加——外部 crate，不进 `ALLOWED`、不产生依赖边）**；
`std::sync::Arc` + `std::collections::HashMap`；
dev-only：`continuum-persist`（`Tx`）、`tempfile`、`trybuild`；`tokio`（已在 dev-dependencies）。
**外部 crate 与「不新增依赖边」那条性质无关**——`ALLOWED` 只逐对断言 workspace 成员之间的边（设计 §10 末段）。

**设计依据：** `docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md`（下称「设计」）。
前提是共享面 `docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md`；裁决在
`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md`（**第五节：删 `ToolInvocation`**）；
写计划前的**八条接缝裁决**转录在 `docs/superpowers/p3bcdf-followups.md` **§七**（那一节是**权威转录**；
原始接缝分析所在的 `.superpowers/sdd-p3bcdf/impl-seam-map.md` 是 gitignore 的 scratch，**随时会丢**，
故本计划**不引它当约束来源**）。其中**对本计划承重的四条已逐条抄进下面正文**：
§七 第 5 条（请求类型的构造点唯一归 C）、第 6 条（`effect_class`/`trust`/工具侧 `cost`·`latency` 落遗留）、
第 7 条（多写者单文件各自登记自己那几条）、第 8 条（C 的 `FakeTool` 与 F 的库级夹具是两份副本，不合并）。

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。** 本计划唯一的边表改动是 `continuum-provider` 那一条。
  终值（设计的唯一产生点是 §10，本节不复述理由）：
  ```rust
      ("continuum-provider", &["continuum-capability", "continuum-core", "continuum-persist"]),
  ```
  `crates/continuum-provider/Cargo.toml` 与 `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 里
  **`("continuum-provider", …)` 那一个条目**（**按条目名找，不按行号**——B/D 在 C 之前各自往同一个数组里
  加过自己的条目，行号会漂）**必须精确一致**——断言是逐对 `assert_eq!`，且 `every_crate_depends_only_on_its_allowed_set`
  跑的是 `cargo tree --edges all`，故 **dev-dependency 也进表**。
  **只加 `Cargo.toml` 不加 `ALLOWED`（或反之）会红**（`p3bcdf-followups.md` §四.1 的硬提醒 1）。
  **这句话管的是内部 crate 的边**：`ALLOWED` **只逐对断言 workspace 成员之间的边**，
  **外部 crate（`thiserror` / `tempfile` / `trybuild` / `tokio` / `serde_json` / `futures-core` / `async-trait`）
  按需加进 `Cargo.toml`，不进这张表、也不构成这条约束意义上的「依赖边」**（设计 §10 末段）。
  **本计划里凡写「不新增依赖边」的地方，一律按「不新增内部 crate 的边」读**——外部 crate 是另一回事。
- **多写者单文件，各自登记自己那几条**（`p3bcdf-followups.md` §七 第 7 条）：
  `dependency_direction.rs` 的 `ALLOWED`、`main.rs` 的注册、workspace `members` 是**单一文件、多写者**，
  四份计划都要碰。故**不设集中登记 task**，**每份计划各自登记自己那几条**（照 P3A 的 Tasks 3/4/5 做法），
  由执行序（`B → D → C → F`，串行）保证不冲突。**C 只改 `continuum-provider` 那一个条目**
  （Task 3 改成二元素、Task 4 补成三元素），**不碰其余任何条目**。
- **本计划不新建 crate、不建表、不取迁移号**（设计 §10）。迁移编号只对 D 存在（它的设计 §3.3 取 80、预留 81），
  C 一行都不碰 `runtime_migrations()` / `expected_migrations()` / `tests/startup.rs`。
- **本计划不改 `continuum-runtime` 的源码。** 对 `continuum-runtime` 的改动**只有一处**：
  `crates/continuum-runtime/tests/dependency_direction.rs` 里 `continuum-provider` 的那个条目。
  `dependency_direction.rs` 里那句已成假的注释（**「core / events / provider 在 runtime 内至今无任何引用」
  ——按这句话的内容找，不按行号**：B 在 C 之前已经往同一个文件里加过自己的条目，行号会漂）**由 F 订正**
  ——F 首次真引用 `continuum-core` 与 `continuum-provider`，它的设计 §10.1 明写那句必须就地改并把来历留原地；
  本计划不动。
- **`continuum-provider` 不引用任何适配器实现类型**（设计 §4.1）。注册表只用
  `std::collections::HashMap` 与 `std::sync::Arc`，引用的类型是 `Arc<dyn ModelProvider>` /
  `Arc<dyn ToolProvider>` / `ModelId` / `ToolId` / `AuthorizedTool`——**无一是实现类型**。
  若注册表引用了任何具体适配器，编译期就会要求一条新边，而那条边必须先写进 `ALLOWED` 才绿。
- **`ToolId` 一律复用 `continuum_core::tool::ToolId`，本计划不另建第二个**（P3A 设计 §3.1；
  共享面 §二「再不要造第二个 `ToolId`」）。本计划只**再导出**。
- **`Tool`（§252）与 `ToolDescriptor`（§316）并存，不合并**（设计 §8）——本计划不改任何一边的字段。
- **本计划不接 `save_tool` 的登记期不变量。** 那条不变量（`effect_class == Some(t) ⇒ for_effect(t) ∈ required_capabilities`）
  约束的是 `tool` 表（`p3_capability_migrations()` 建的那张，编号 50）的**写入**——
  **生产写入的唯一写点是 `crates/continuum-capability/src/persist.rs:67` 的 `save_tool`**
  （`grep -rn "INSERT INTO tool" crates/` 的其余命中都在 `continuum-capability` 自己的测试里，
  它们正是要裸写表外取值的用例；已按现文件核过），
  **裁决：所有者是 F**（`p3bcdf-followups.md` §七 第 1 条）。本计划一行都不写它。
- **`docs/02-工程.md` §10.3 与共享面 §二（类型清单）由控制器改**（设计 §7.5 落地清单第 4、5 处）。
  本计划**不动那两个文件**，只在报告里点明它们待改。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿，**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增 crate 依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列了源码与清单，
  锁文件按本行办。

## 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」跑全量是加分。
   **变异的三条失效形态都要防**：(a) 锚点不唯一 → 变异没落到实现体却报 GREEN；(b) **等价变异体**——
   判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**；
   (c) **变异导致编译失败**——那不是「变红」。每次变异用**独立日志路径**，读前确认是本轮写的。
   **判据本身也会假阳性**：cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，
   故判「编译失败」要用 `could not compile` 或 `error[E….`。
   **分层**：承重守卫（`invoke_tool` 不交出适配器、失败路径**判别哪一种 `Err`**、`ALLOWED` 与依赖精确一致、
   `ToolInvocation` 的删除）→ **全量套件**；其余分支 → **受影响 crate 的包级套件**，且报告里须**标明证据强度较低**
   并列出「这一条可能漏掉的跨 crate 观察点」。
2. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**。
   **枚举式绝对断言须逐项有照片**——本计划有三处「逐项」：`list_tools` 的**每个**已登记适配器各调用一次（Task 2）、
   `invoke_tool` 的**每一条**失败通道各一条用例（Task 4）、编译失败样例的**每一份**各钉住预期报错（Task 6）。
   判据：用例里的值字面量是**手工写的**还是**被测函数返回的**——前者钉格式，后者钉路径。
2 之补（2026-10-06，Task 5 独立评审给出）——**「有照片」还不够，那条照片还必须是守卫**：
   **一条断言是守卫，当且仅当在它实际所处的位置上，存在一个「被测代码」的变异使它会红。**
   **恒真型**（不存在这样的变异体）与**蕴含型**（变异体存在，但同体内在它之前的断言先红并蕴含它）
   **同等处置：删**；**不许靠调序制造第二个落点**，除非两条断言钉的是互不相交的变异集。
   **位置是判据的一部分**——同一条断言文本可以在一处是守卫、在另一处不是。完整叙述与两条实例
   见 **Task 5 Step 1 的「统一判据」段**（那里有第一向/第二向与 Task 4 那条的对照）。
3. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 Err」。本计划的失败面有三处，**一个都不能只写 `is_err()`**：
   `RegistryError::NotFound` / `RegistryError::Duplicate` / `ToolCallError::Unregistered` / `ToolCallError::Provider`。
   **但「断言是哪一种」不等于「把互斥的另一臂也否一遍」**：`matches!(A)` 之后再加 `!matches!(B)`
   （`A`/`B` 是同一个枚举的两个变体）是**蕴含型**，按第 2 之补**删**。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志——
`pgrep "cargo|rustc"` 看不见卡在 `D` 状态的测试二进制。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
本计划的库用例用 `tempfile::tempdir()`（它读 `TMPDIR`），收工前 `chmod -R u+rwX .tmp && rm -rf .tmp`。
**`.tmp/` 不入库**（它不在 `.gitignore` 里，`git status` 会显示，但只按显式路径 `git add` 就不会误提交）。

**变异日志是证据，必须活到复审结束**：报告里的变异表会引用每条变异的日志路径，而**实现者不得在交活前删掉 `.tmp/`**
——**实现者保留 `.tmp/`，由协调者在复审结束后清理**。同理，报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径
作为任何东西的唯一来历（**设计文档在 `docs/superpowers/` 下，那个可以引**）。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。
**实现者报告完成之前，协调者不得在该工作区里跑 cargo**；协调者的独立重跑**严格排在实现者的终稿之后**。
**这条管的是变异窗口，不是「验证之间」**——两份跑在同一份已提交字节上的验证并行是无害的。

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**（P1 出过 20+ 处事实错误），且已确立「**代码块是示意，正文的措辞才是约束**」。
本计划据此**只给签名与关键判定**，不给可整段粘贴的实现；导入清单尤其不可照抄（它在 P3A 与 P3B 都是重灾区）。
**凡与既有 crate 交互的签名，实现前须先读该 crate 的源码确认**，不符时以源码为准并回报。
本计划已核对过的既有面（**照录**，实现时仍以源码为准）：
`crates/continuum-provider/src/{lib.rs,tool.rs,model.rs}`、`crates/continuum-core/src/{tool.rs,error.rs}`、
`crates/continuum-provider/tests/fake_provider.rs`、`crates/continuum-capability/src/registry.rs`、
`crates/continuum-capability/tests/authorize.rs` 的 `db()`（**函数名，不是行号**）、
`crates/continuum-runtime/tests/dependency_direction.rs` 里 `ALLOWED` 的 `("continuum-provider", …)` 条目（**条目名**）。

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

**依赖已交付（全部在 `main` 上，本计划不用再造）：**

- `continuum-capability`：`AuthorizedTool`（`tool_id()` / `granted()`）、`authorize`、`mint`、`Capability`、
  `CapabilityKind`、`Tool` / `ToolProfile` / `Trust` / `Cost` / `Latency`、`save_tool` / `load_tool` /
  `p3_capability_migrations`、`ToolId`（**再导出** `continuum_core::tool::ToolId`）。
- `continuum-core`：`ModelId` / `ModelDescriptor` / `InvokeRequest` / `InvokeResponse` / `Usage` / `CallId` /
  `ModelStream` / `ProviderHealth`；`ToolId` / `ToolDescriptor` / `ToolResult`；`ProviderError`。
- `continuum-persist`：`Db` / `builtin_migrations` / `Migration` / `Tx` / `Value` / `PersistError`（**dev 用**）。

**不依赖 B、D、F 的任何产物。** 四份计划之间唯一的硬依赖是 **C → F**（F 的库函数签名要收
`&ProviderRegistry`、调 `invoke_tool`；C 的 trait 变更与类型删除是 F 编译的前提），而 C 在它之前。
B、D 与 C 之间没有编译期依赖（B 走 §124，不经 §316；D 只用 P3A 与 `continuum-core`）。

**交给 F（硬前置，F 的编译前提）：** `ProviderRegistry`、`invoke_tool`、`AuthorizedToolInvocation<'_>`、
`ToolCallError`、`RegistryError`，以及 **`ToolInvocation` 的删除**。
**F 只调 `invoke_tool`，不构造也不命名请求类型**（`p3bcdf-followups.md` §七 第 5 条）。

**交给控制器（本计划不动那两个文件）：** 共享面 §二 §316 段的类型清单（删 `ToolInvocation`）、
`docs/02-工程.md` §10.3 的接口清单（标注请求面已变）——设计 §7.5 落地清单第 4、5 处。

**交给子项目 G（尚不存在）：** 模型侧调用面（设计 §6 的六步，**「取描述」与「取可用性」是两步、产物是两样东西**）：
1. **取描述**：`list_models()` / `describe_model(&ModelId)` → `ModelDescriptor`；
2. **取可用性**：`health()` → **`ProviderHealth`，那才是「当前可用性」**，不是第 1 步的 `ModelDescriptor`
   ——实现者不得拿第 1 步的产物当可用性（设计 §6 已把初稿混称的一步拆开）；
（第 3 步是 **D 的 Router 的候选排序**，不在本层也不在本 crate——此处跳过不是漏项。）
4. **解析**：`model_for(&ModelId)` → `Arc<dyn ModelProvider>`，未登记 → `RegistryError::NotFound`；
   **id 来自 D 的 Model Registry 表**，不是从本注册表枚举来的
   （**「枚举全部模型适配器」的入口 `model_providers()` 已裁删，2026-10-06**；理由三条与代价见 Task 1）；
5. **调用**：`invoke` / `stream`；6. **消费流**：读 `ModelStream.chunks`，要中止则 `cancel(&stream.call)`。

另：`usage()` 的语义未定义（见 `## 遗留` 二）。

**本计划必须自己声明的未决**：设计 §12 第 18 条（P1 的 `Resource` 义务在 C/D/F 三份里无人认领，收件人：控制器）、
§12 第 23 条（凭据要不要也交给适配器——第二个位置，本阶段不做）。两条都落在 `## 遗留`。

---

# 文件结构

```
crates/continuum-provider/
  Cargo.toml                     Task 1：normal + thiserror（外部 crate，不进 ALLOWED）；Task 3：normal + continuum-capability
                                 （内部边）；Task 4：dev + continuum-persist / tempfile；Task 6：dev + trybuild
  src/lib.rs                     再导出注册表、两个错误类型与新请求类型；模块文档补一句「注册表在、实现仍不在」
  src/registry.rs        （新）   ProviderRegistry、RegistryError、ToolCallError、invoke_tool（唯一受门禁入口）
  src/tool.rs                     §316 trait + 新请求类型 AuthorizedToolInvocation<'_>（构造入口 pub(crate)）；invoke 的请求参数换掉
  src/model.rs                   不动
  src/connector.rs               不动
  tests/common/mod.rs    （新）   C 侧夹具（FakeModel / FakeTool / OnceStream），由多个测试目标共同 include
  tests/fake_provider.rs         改为 mod common;，FakeTool 的 invoke 换签名
  tests/registry_models.rs （新） 模型侧登记 / 发现 / 重复 / 原子性 / 枚举 / §3.3 代价一
  tests/registry_tools.rs  （新） 工具侧登记 / 只读入口 / 并集 / 适配器失败
  tests/invoke_tool.rs     （新） invoke_tool 的四条路径 + 门禁内正常路径（真起库）；**新请求类型的两条性质也在这里观测**（crate 外构造不出它）
  tests/two_registries.rs  （新） 两个登记点不一致的两向（§3.3 代价三）
  tests/contract.rs        （新） is_error 与 Err 的分流（约定的夹具照片）
  tests/compile_fail/*.rs  （新） 五份不可表达性样例，各配同名 .stderr
  tests/type_level.rs      （新） trybuild glob 驱动
  tests/neutrality.rs      （新） 模块面中立性守卫（src/ 下不出现 impl … for …）
```

**既有的、本计划要改的文件**

```
crates/continuum-core/src/tool.rs                        删 ToolInvocation（裁决 §五；ToolId / ToolDescriptor / ToolResult 不动）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 的 continuum-provider 条目（唯一一处）
Cargo.lock                                               随依赖变化（见 Global Constraints）
```

**本计划明确不碰**：`crates/continuum-runtime/src/**`、`crates/continuum-capability/**` 的源码、
`docs/02-工程.md`、共享面、`crates/continuum-core/src/{model.rs,error.rs,connector.rs}`。

---

### Task 1: `ProviderRegistry` 骨架、两个错误类型、模型侧登记与发现

**Files:**
- Create: `crates/continuum-provider/src/registry.rs`
- Modify: `crates/continuum-provider/src/lib.rs`（`pub mod registry;` + 再导出）
- Create: `crates/continuum-provider/tests/common/mod.rs`（从 `tests/fake_provider.rs` 提出 `FakeModel` 与 `OnceStream`）
- Modify: `crates/continuum-provider/tests/fake_provider.rs`（改为 `mod common;`）
- Create: `crates/continuum-provider/tests/registry_models.rs`

**Interfaces:**
- Consumes: 既有的 `continuum_core::model::ModelId`、**`continuum_core::tool::ToolId`**
  （`ToolCallError::Unregistered { id }` 要用它）、`continuum_provider::model::ModelProvider`、
  `continuum_core::ProviderError`
- Produces: `continuum_provider::{ProviderRegistry, RegistryError, ToolCallError}`，
  `ProviderRegistry::{new, register_model, model_for}`

> **本 task 不新增任何**内部 crate 的**依赖边**（`HashMap` / `Arc` 来自 `std`）；
> **外部 crate 按需加，不进 `ALLOWED`**——那张表**只逐对断言 workspace 成员之间的边**（设计 §10 末段：
> 「外部 crate 不进这张表：`tokio` / `tempfile` / `serde_json` / `futures-core` 都不是 workspace 成员」）。
> **`Cargo.toml` 会加一行 `thiserror = { workspace = true }`**（本 crate 的 `RegistryError` / `ToolCallError`
> 派生 `thiserror::Error`，而本 crate 今天没有这个依赖）——它**不产生依赖边**，故本节要断言的关键性质不受影响。
>
> 要断言的关键性质是**「注册表加进来之后，`continuum-provider` 没有多出任何指向适配器实现的边」**
> （设计 §4.1），而它的照片就是**这一步之后 `every_crate_depends_only_on_its_allowed_set` 与
> `core_and_persist_do_not_depend_on_provider` 仍绿**。
>
> **订正（2026-10-06，Task 1 交回后）**：本段原话写的是「**`Cargo.toml` 与 `ALLOWED` 一字不动**」
> ——**那句过宽、而且与本 task 自己的代码块相抵**（代码块用了 `#[derive(thiserror::Error)]`，
> 而 `continuum-provider` 的 `[dependencies]` 里当时没有 `thiserror`，照抄编译不过）。
> 错误的**来历**：写那句话时的本意是「**不新增任何指向内部实现的依赖边**」，而 `ALLOWED` 只列内部 crate，
> **外部 crate 与那条性质无关**；把两者混成一句就变成了假命题。**原话照留在此，勿按它回退。**
>
> **`model_providers()` 已裁删（2026-10-06，设计在进行中删的它）。** 本 task **不实现、也不写它的用例**。
> **来历与理由三条**（设计 §3.1 的裁决段）：(a) 它**零消费方**；(b) 它返回
> `Vec<Arc<dyn ModelProvider>>`、**不带 id**，而调用方（子项目 G）是**按 id 解析**的（`model_for`），
> 两者无从对齐；(c) **「有哪些模型」的权威在 D 的 Model Registry 表**，不在本注册表。
> 代价据实记：将来若真需要「跨全部适配器聚合」（汇总健康、适配器级探活），提出者得**重新提出这个入口**
> ——那时他有真消费方、也能自己挑形状，是便宜的方向。**`model_for` 保留**（G 用它）。

> **执行序订正（2026-10-06，Task 1 实跑）**：本 task 原把「写用例」列为 Step 1、「把夹具提到
> `tests/common/mod.rs`」列为 Step 3，**而 Step 1 的用例要从 Step 3 才建的文件里取夹具**——
> 照原序执行，Step 2 得到的红会是「`file not found for module common`」，**不是**本 task 想要的那种红。
> 故三步按下面的次序做：**先把夹具提出来（Step 1）→ 再写用例（Step 2）→ 再跑出预期的红（Step 3）**。

- [ ] **Step 1: 把夹具提到 `tests/common/mod.rs`**

`FakeModel` 与 `OnceStream` 从 `tests/fake_provider.rs` **原样搬进** `tests/common/mod.rs`（`pub`），
`FakeConnector` 留在 `fake_provider.rs` 原地（它只被那一个目标用）。两个目标各写 `mod common;`。
**搬完之后 `fake_provider.rs` 的导入清单要跟着收**（搬走 `FakeModel` / `OnceStream` 后
`futures_core::Stream`、`ModelStream`、`StreamChunk`、`Pin`、`Context`、`Poll` 都成了未用导入，
实测会产生 warning，而「0 warning」是硬约束）——**本步做完先 `cargo build -p continuum-provider --all-targets`
核一次 0 warning**。

**该模块开头要有 `#![allow(dead_code)]`，并写明理由**：同一个 `common` 被多个测试目标 include，
**没有任何一个目标用得到全部夹具**，而本仓要求 0 warning。

> **订正（2026-10-06，Task 1 实跑）**：本段原写「去掉它会得到 `dead_code` 警告」——
> **在本 task 的时点上那句为假**：把该行删掉，`cargo test -p continuum-provider` 出 **0 warning、EXIT=0**，
> 因为 `FakeModel` 与 `OnceStream` 都还有人引用（后者经 `FakeModel::stream`）。
> 该行因此是**为后续 task 预留的**：Task 2 把 `FakeTool` 搬进来之后，`fake_provider.rs` 那个目标
> 大概就会用不到 `FakeTool`——**那时由 Task 2 自己实跑一次再拍**，不要照抄这句。
> **这条教训是通用的：同一句绝对措辞在不同 task 的时点上可以一真一假，写它的时候要指明时点。**

- [ ] **Step 2: 写用例**

`tests/registry_models.rs`（夹具从 `tests/common/mod.rs` 取，已由 Step 1 建好）：

- `a_registered_model_is_found_by_id`：登记 `fake-1 → FakeModel`，`model_for("fake-1")` 给出的
  `Arc` 与登记时那一个是**同一个**（`Arc::ptr_eq`，不靠行为推断）。
- `an_unregistered_model_id_is_reported_as_not_found`：`model_for("nope")` →
  `matches!(_, Err(RegistryError::NotFound { .. }))`——**断言是哪一种 `Err`**（纪律 3）。
- `registering_the_same_id_twice_is_rejected_as_duplicate`：同 id 再登记 → `RegistryError::Duplicate`，
  且**原条目不变**（第一次登记的 `Arc` 仍在，`Arc::ptr_eq` 钉住）。
- `registering_a_group_of_ids_is_all_or_nothing`：把 `[a, b]` 登记给一个适配器，其中 `b` 已占用 →
  返回 `Duplicate`，**且 `a` 也没有进去**（`model_for(a)` → `NotFound`）。
- **不再有「模型侧枚举」的用例**：`model_providers()` 已裁删（见上方裁决段），设计 §11 的对应行也随之删除。
  **不要**为它补一条「反正写了也无害」的用例——零消费方的公开面正是本项目说的「建好但没人用」。
- `a_registered_id_may_be_absent_from_the_adapters_own_list_models`：**设计 §3.3 代价一与 §11 的照片**——
  把 `FakeModel` 登记到它 `list_models()` **不含**的 id（`"m"`）：`model_for("m")` **命中**，
  而对同一 `Arc` 调 `describe_model("m")` 得 `Err(ProviderError::UnknownModel(_))`。
  这条用例钉的是「登记是路由的权威、适配器的 `list_*` 是描述的权威」这两件事**可以不一致**。

- [ ] **Step 3: 运行，确认失败**

```bash
timeout 300 cargo test -p continuum-provider --test registry_models
```

预期：**`E0432`**（`unresolved imports`：`continuum_provider::ProviderRegistry` 不存在）
——**这个码取自实跑**（`.superpowers/sdd-p3c-impl/task-1-report.md` 第 2 节，日志 `.tmp/step2-red.log`）。
**订正（2026-10-06）**：本节原写 `E0433`，是照「类型不存在」推测的；实跑的写法是
`use continuum_provider::{ProviderRegistry, ..};`，那是 **`E0432`（unresolved imports）**。
**「全限定路径才会给 `E0433`」那半句原样留在这里，但它是未实测的对照说法**（写它的时候没跑过）
——**已改为不写码的说法**：**同一处「类型不存在」在不同的写法下会给不同的码，故以实跑读回的
`.stderr` 为准**。**凡计划里写错误码的地方一律以实跑为准**（本计划其余处的码同此例）。
**判据留档**：**「未见实跑记录」的报错码，比写错的码只差一步**——两者都是**凭印象写下的、
看起来像事实的东西**；处置统一：**要么实跑、要么不写码**。

- [ ] **Step 4: 实现**

```rust
/// 登记与查找的失败。**与 `ProviderError` 判据不同、不可合并**（设计 §3.5）：
/// `ProviderError` 描述**一次 provider 调用**的失败（可能瞬时，可重试）；
/// 本类型描述**路由 / 装配**的失败（配置缺陷，重试不会变）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// 模型侧按 id 发现时未命中（设计 §3.1）。**工具侧不走这里**——工具侧的未命中是
    /// [`ToolCallError::Unregistered`]。
    #[error("模型 {} 未登记", id.as_str())]
    NotFound { id: ModelId },
    /// 同一个 id 被登记第二次（模型侧与工具侧共用）。
    ///
    /// **与 `crates/continuum-operator/src/registry.rs:10-11` 的 `OperatorError::Duplicate`
    /// 同的只是「重复登记有一个具名变体」这一取向，不是字段形状**：那里两个字段都是强类型
    /// （`OperatorId` / `OperatorVersion`），而本注册表有**两个 id 类型**（`ModelId` / `ToolId`），
    /// 收窄到某一种就得为另一种再造一个变体。
    ///
    /// **字段形状规范未给判据**（裁决 C5 明写归遗留）：本计划取「以文本承载 id，
    /// 是**哪一个登记表**（模型侧 / 工具侧，即本注册表里的两个 `HashMap`）由调用点可知
    /// （`register_model` / `register_tool`），故不另立字段」，
    /// 并把它作为**计划的取法**申报在 `## 遗留`（三）。
    #[error("id {id} 已登记")]
    Duplicate { id: String },
}

/// 受门禁的 `invoke_tool` 与工具侧只读入口的失败（设计 §3.5）。
///
/// **为什么需要第二个错误类型**：`invoke_tool` 的失败有两个不同来源——「这个 id 没有适配器」
/// （路由，配置缺陷）与「适配器调用失败」（可能瞬时）；一个 `Result` 只能带一个错误类型，
/// 把两者压进 `ProviderError` 就是把「配置错了」报成「provider 挂了」——而那件事在本仓
/// 已经发生过一次（`crates/continuum-provider/tests/fake_provider.rs` 的 **`FakeTool::describe_tool`**
/// 把「无此工具」报成 `Unavailable`，设计 §5.2 记的正是这一处）。**引条目名不引行号**：
/// 那个文件在本计划里会被搬动（Task 1/2），行号会漂。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ToolCallError {
    /// 这个 id 没有登记的适配器。（工具 id 只有授权证明一个来源，故本变体只由路由产生。）
    #[error("工具 {} 没有登记的适配器", id.as_str())]
    Unregistered { id: ToolId },
    /// 适配器自己失败。
    #[error("适配器调用失败: {0}")]
    Provider(#[from] ProviderError),
}

/// 进程内注册表（设计 §3.1）。**持两张表，不拆两个结构体**（设计 §3.4 的理由：装配点只有一个）。
///
/// 适配器是**代码**（`Arc<dyn …>`），不是数据——故它不进库，也不是一张表。
///
/// **暴露面两侧不对称，理由是强制点 (1) 只覆盖工具**（设计 §3.1）：
/// 模型侧 `model_for(&ModelId)` 直接交出裸 `Arc<dyn ModelProvider>`，**这是模型侧唯一的发现入口**
/// ——**「枚举全部模型适配器」的入口已裁删**（`model_providers()`，2026-10-06；三条理由见 Task 1）；
/// 工具侧**只开只读入口与唯一的调用入口 `invoke_tool`**（Task 4），**不交出适配器**。
#[derive(Default)]
pub struct ProviderRegistry { /* 两张 HashMap + 工具侧一份登记顺序的适配器列表（`list_tools` 要按登记顺序），字段私有 */ }

impl ProviderRegistry {
    pub fn new() -> Self { /* … */ }

    /// 把**一组 id** 登记给同一个适配器（设计 §3.1：id 由登记方显式给出，注册表不去问适配器）。
    ///
    /// **先全查后全插（原子）**：组内任一 id 已占用即返回 [`RegistryError::Duplicate`]，
    /// **一个都不登记**。半登记的注册表会让「谁服务这个 id」这件事在两次启动之间漂移，
    /// 而漂移没有任何用例看得见。**设计未规定这一点**，本计划取原子并给出照片
    /// （`registering_a_group_of_ids_is_all_or_nothing`），见 `## 遗留`。
    pub fn register_model(
        &mut self,
        ids: Vec<ModelId>,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<(), RegistryError>;

    /// 按 id 发现。未登记 → [`RegistryError::NotFound`]。
    ///
    /// **模型侧唯一的发现入口**（设计 §3.1）：没有「枚举全部适配器」的入口——`model_providers()`
    /// 已裁删（2026-10-06）。要列出「有哪些模型」，权威在 **D 的 Model Registry 表**，不在这里。
    pub fn model_for(&self, id: &ModelId) -> Result<Arc<dyn ModelProvider>, RegistryError>;
}
```

**`register_model` 的原子性**（先全查后全插）是**设计未给判据**的一点，本计划取上面的读数并附照片，
见 `## 遗留`（三）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
timeout 900 cargo build --workspace --all-targets
git add crates/continuum-provider
git commit -m "feat(provider): ProviderRegistry 骨架与模型侧登记发现"
```

预期：全绿、0 warning，且 `every_crate_depends_only_on_its_allowed_set` **仍绿**（本 task 没有加任何边）。

---

### Task 2: 工具侧登记与只读入口

**Files:**
- Modify: `crates/continuum-provider/src/registry.rs`、`src/lib.rs`
- Modify: `crates/continuum-provider/tests/common/mod.rs`（把 `FakeTool` 提进来）
- Modify: `crates/continuum-provider/tests/fake_provider.rs`
- Create: `crates/continuum-provider/tests/registry_tools.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProviderRegistry` / `ToolCallError`；既有的 `continuum_core::tool::{ToolId, ToolDescriptor}`
- Produces: `ProviderRegistry::{register_tool, list_tools, describe_tool}`

> 本 task 仍**不新增任何内部 crate 的依赖边**，也**不新增外部 crate**（要用的
> `continuum_core::tool::{ToolId, ToolDescriptor}` 与 `ProviderError` 都在既有依赖里；
> `std::sync::Arc` / `std::collections::HashMap` 来自 `std`）。`invoke_tool` 与 `AuthorizedTool` 在 Task 3 / Task 4。
> **不提供 `tool_for`、也不提供 `tool_providers()`**——它们的缺席是 Task 6 的编译失败样例钉的
> （本 task 只把它写进模块文档与 `lib.rs` 的注释，**不写「已钉住」**）。

- [ ] **Step 1: 写用例**

`tests/registry_tools.rs`：

- `a_registered_tool_is_described_by_id`：登记 `echo → FakeTool`，`describe_tool("echo")` 给出的
  `ToolDescriptor.id` 与 `list_tools()` 里那条一致。
- `an_unregistered_tool_id_is_reported_as_unregistered`：**夹具布置写死——沿用上一条已登记的
  `echo → FakeTool`，只把查询的 id 换成 `nope`**（**注册表非空**，这一点是判据的一部分，
  理由见下方的订正注记）→ `matches!(_, Err(ToolCallError::Unregistered { .. }))`——
  **不是** `ToolCallError::Provider`（纪律 3）。**本用例的承重断言是「未命中返的是哪一种 `Err`」**
  （`Unregistered` 与 `Provider` 分别描述「路由 / 配置缺陷」与「provider 调用失败」，设计 §5.2）。

  > **订正注记（2026-10-06，本计划扫同类 + 协调者裁定）。** **原话照留**：上面那半句
  > 「**不是** `ToolCallError::Provider`（纪律 3）」——**「蕴含型」，删**：同上，枚举互斥，
  > 前一条 `matches!(…Unregistered { .. })` 成立即蕴含它、不可能独立变红。
  > **落地现状：代码里仍在**（`tests/registry_tools.rs` 该用例里那一条
  > `!matches!(err, ToolCallError::Provider(_))`；**按用例名 + 断言内容找**），**协调者已另派删除**。
  > **这一处是本条注记自己犯的**：上一轮我为了把「未命中返哪一种 `Err`」这条承重说清，
  > **顺手把互斥的另一臂也否了一遍**——正是我刚在 Task 5 那段里判为「蕴含型、要删」的形状。
  > **来历照留在此**，因为它比一句干净的话有用：**「把承重说清」与「多否一臂」是两件事，
  > 前者靠一条能独立变红的断言，后者只会让报告里多一条永远绿的记录。**
  > **删后承重不变**：本用例余下的是 `Unregistered` 那一条，「报成 `Provider`」的变异**仍使它会红**
  > ——承重本来就是它。

  > **订正注记（2026-10-06，协调者裁定）**：本节原只写「`describe_tool("nope")` → …」，
  > **没写注册表里当时有什么**。若实现者按字面「什么都不登记」，注册表为空，
  > 于是「先扫各适配器的 `list_tools()` 再判未命中」这个变异体**在这里退化**（两版都扫零个适配器、
  > 都给 `Unregistered`），**永远绿**——与 Task 4 那条同一条成因。
  > **故把布置写死**：先登记 `echo → FakeTool`，再查 `nope`。
  > **落地核对**：Task 2 **早已实现并过审**，其 `crates/continuum-provider/tests/registry_tools.rs` 里
  > 该用例**正是这么布置的**（先 `register_tool(vec![tool("echo")], Arc::new(FakeTool))`，再
  > `describe_tool(&tool("nope"))`），与本注记一致——**故这是把计划的布置补齐，不是要改测试字节**。
  > **判据同前一条**：凡「某条用例不构造 X」的指令，先问「不构造它之后，我要钉的那个变异体
  > 还区分得出来吗」。
- `list_tools_is_the_union_of_every_registered_adapter`：登记两个各出一个工具的适配器 →
  `list_tools()` 长度 2，**顺序 = 登记顺序**，且**逐个**断言各自的 `id`（枚举式断言逐项有照片）。
- `list_tools_keeps_the_first_entry_when_two_adapters_declare_the_same_id`：两个适配器都声明 `echo`
  （描述不同）→ `list_tools()` 只有**一条** `echo`，且是**先登记的那个**给出的描述（并集语义）。
- `list_tools_reports_an_adapter_failure_as_provider`：把 `FakeTool` 换成一个 `list_tools` 返
  `Err(ProviderError::Transport(..))` 的适配器 → `matches!(_, Err(ToolCallError::Provider(_)))`，
  且**不是** `Unregistered`。**同一个适配器的失败不许被跳过**（跳过会让一个坏适配器静默消失）。

  > **订正注记（2026-10-06，本计划扫同类 + 协调者裁定）。** **原话照留**：上面那半句
  > 「且**不是** `Unregistered`」——**「蕴含型」，删**：`Provider` 与 `Unregistered` 枚举互斥，
  > 前一条成立即蕴含它，它不可能独立变红。**落地现状：代码里仍在**
  > （`tests/registry_tools.rs` 该用例里那一条 `!matches!(err, ToolCallError::Unregistered { .. })`；
  > **按用例名 + 断言内容找**），**协调者已另派删除**。
  > **删后承重不变**：余下的 `Provider(_)` 那条仍钉「适配器失败不许被跳过」这件事
  > ——「跳过失败适配器、返回 `Ok(空)`」的变异会使它红。
- `describe_tool_routes_by_registration_not_by_the_adapters_list_tools`：**设计 §3.1「只读入口的
  权威是已登记的适配器，不是登记表本身」的照片**——把 `FakeTool` 登记到它 `list_tools()` **不含**的 id
  （`"m"`）：`describe_tool("m")` **路由到适配器**，于是得到适配器自己的
  `Err(ProviderError::Unavailable("m"))`，包在 `ToolCallError::Provider` 里。
  **这条用例证明路由不查 `list_tools()`**——反序实现（先查 `list_tools` 再路由）在它上面会红。

  > **订正注记（2026-10-06，本计划扫同类 + 协调者裁定）。** 本用例在实现里另写了一条
  > `assert!(!matches!(routed, ToolCallError::Unregistered { .. }), "命中了登记的适配器，就不该报
  > Unregistered——报了就说明路由查的是 list_tools()")`（`tests/registry_tools.rs`；
  > **按用例名 + 断言内容找**）。**它是「蕴含型」，删**：紧挨在它前面的那条断的是
  > `matches!(routed, ToolCallError::Provider(ProviderError::Unavailable(_)))`，而两个变体**枚举互斥**
  > ——前一条成立即蕴含它。而它那句注释里点出的判别力（「反序实现会给 `Unregistered`」）
  > **本来就落在前一条上**：反序变异下前一条先红。故它**不可能独立变红**，**协调者已另派删除**。
  > **删后承重不变**：本用例余下的正是 `Provider(Unavailable(_))` 那条——它就是「路由不查
  > `list_tools()`」这条性质的守卫。

- [ ] **Step 2: 运行，确认失败**

```bash
timeout 300 cargo test -p continuum-provider --test registry_tools
```

- [ ] **Step 3: 实现**

```rust
impl ProviderRegistry {
    /// 工具侧登记。与 `register_model` 同形：一组 id、一个适配器、先全查后全插。
    /// **注意那是适配器登记，不是往 `tool` 表登记**——后者是 `continuum_capability::save_tool`，
    /// 属治理（要哪些能力、什么 effect_class）；本处只管路由（谁去跑）。设计 §3.3 代价三。
    pub fn register_tool(
        &mut self,
        ids: Vec<ToolId>,
        provider: Arc<dyn ToolProvider>,
    ) -> Result<(), RegistryError>;

    /// **已登记适配器**各自 `list_tools()` 的并集，按登记顺序。
    ///
    /// 任一适配器失败即返回该 `Err`（**不吞、不跳过**）——静默跳过一个坏适配器，会让
    /// 「哪些工具存在」在下游少一块而无人看得见。
    ///
    /// **并集语义**：同一个 `ToolId` 被多个适配器声明时保留**首次出现**的那一条。
    /// **设计未给判据**，本计划取并集并附照片，见 `## 遗留`。
    ///
    /// **它不按 id 查，故本入口上 [`ToolCallError::Unregistered`] 臂不可达**（设计 §3.5 的订正段）。
    /// 写用例与注释时**不要**声明本入口会报 `Unregistered`。
    pub async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ToolCallError>;

    /// 先按**登记**路由到适配器，再转出它的描述；id 未登记 → [`ToolCallError::Unregistered`]。
    ///
    /// **它不看 `list_tools()`**：登记是路由的权威、适配器的 `list_*` 是描述的权威，
    /// 两者可以不一致（设计 §3.1 末段）。
    pub async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ToolCallError>;
}
```

**只读入口的返回错误类型——已定案，不要再当作未决**：设计 §3.5 那一行原写「受门禁的 `invoke_tool` 用」，
与 §3.1「工具侧未命中（**含 `describe_tool`**）返回 `ToolCallError::Unregistered`」曾两说相抵；
**裁决 C1 取 §3.1，设计 §3.5 已就地订正并在原地留了来历**。故本计划的取法与裁决一致：
按 id 查的两个工具侧入口（`invoke_tool` 与只读的 `describe_tool`）返 `ToolCallError`；
`list_tools()` **不按 id 查**，它那条路径上 `Unregistered` 臂不可达。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider
git commit -m "feat(provider): 工具侧登记与只读入口"
```

---

### Task 3: §316 请求面换血——`AuthorizedToolInvocation<'_>`、trait 改参、删 `ToolInvocation`

**Files:**
- Modify: `crates/continuum-provider/src/tool.rs`（新请求类型 + `invoke` 换参）
- Modify: `crates/continuum-provider/src/lib.rs`（再导出）
- Modify: `crates/continuum-core/src/tool.rs`（**删 `ToolInvocation`**）
- Modify: `crates/continuum-provider/Cargo.toml`（**normal 只加 `continuum-capability`**）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**`ALLOWED` 的 provider 条目**，
  本 task 的终值 `["continuum-capability", "continuum-core"]`；`continuum-persist` 由 Task 4 增量加）
- Modify: `crates/continuum-provider/tests/common/mod.rs`（`FakeTool::invoke` 换参）

**Interfaces:**
- Consumes: Task 1 的 `ProviderRegistry`；既有的 `continuum_capability::AuthorizedTool`、`continuum_core::ProviderError`
- Produces: `continuum_provider::{AuthorizedToolInvocation, ToolProvider}`（trait 的 `invoke` 换参）

> **这是本计划对**已冻结接口**的唯一一次显式改动**（设计 §7.5、裁决 §一第 2 条）：§316 的四项方法名不变，
> 变的是 `invoke` 的请求参数。**删 `ToolInvocation` 与它同批做**（裁决 §五），不要拆成两次提交。
>
> **完成判据是 `cargo test --workspace` 通过**，不是「改完定义就收工」——删除的引用者全在编译期暴露。
> 规划时实测 `grep -rnw ToolInvocation crates/`（**`-w` 是词边界，见下**）恰五个行号：
> `crates/continuum-core/src/tool.rs:26`（定义）、`crates/continuum-provider/src/tool.rs:5,12`
> （trait 的导入与签名）、`crates/continuum-provider/tests/fake_provider.rs:7,109`（夹具的导入行与
> `FakeTool::invoke` 的签名）。**这三处都按条目名找，行号只是规划时那一份字节上的实测值。**
> **但本 task 执行时夹具已经不在那个文件了**：Task 1 把 `FakeModel`、Task 2 把 `FakeTool` 搬进了
> `crates/continuum-provider/tests/common/mod.rs`，故第三处要改的是**那里**（行号以当时文件为准，
> 本计划不预先写死）。
>
> **判据必须用词边界，不能用裸 `grep -rn ToolInvocation`**（整套终审 S-3）：本 task 一落地
> `AuthorizedToolInvocation`，裸匹配就**必然命中自己**（子串包含），这条判据**永远不可能绿**。
> 实跑确认（本仓当前字节 + 一个只含 `AuthorizedToolInvocation` 的探针文件）：
> `grep -rnw ToolInvocation crates/` 在探针存在时**不报探针**，而裸 `grep -rn` 会报——
> 因为 `AuthorizedToolInvocation` 里 `d` 与 `T` 之间**不是词边界**。
> **别照抄这句，自己实跑一遍再下判据。**
>
> **本 task 不新增任何 dev 依赖**：它没有任何运行用例（理由见下），故 `continuum-persist` / `tempfile`
> 由**真正用它们的** Task 4 增量加——「边由用它的那个 task 登记」对 dev 边同样成立。

> **本 task 没有运行用例，这是设计使然，不是漏了照片。** 构造入口 `pub(crate)`（裁决 C2）之后，
> **crate 外的集成测试根本构造不出 `AuthorizedToolInvocation`** —— `tests/` 下的目标是独立 crate。
> 而在 `src/` 里写单元测试就会在 crate 内造出**第二个构造点**（设计 §7.5 把构造点收成
> `invoke_tool` 一处，正是要避免这件事）。故新请求类型的两条可观察性质**改经 `invoke_tool` 观测**，
> 落在 **Task 4**：
> - 「输入被原样带到适配器」→ Task 4 的门禁内正常路径（echo 适配器的 `output == 输入`）；
> - 「适配器手里那枚授权就是传进去的那一枚」→ Task 4 的
>   `the_adapter_is_handed_the_authorization_that_was_passed_in`（记录型适配器读
>   `call.authorization().tool_id()`）。
>
> 另两条性质的照片在 **Task 6**：样例 3（裸 `ToolId` 传不进 `invoke_tool`）、样例 4（**外部 crate
> 持一枚真的 `AuthorizedTool` 仍构造不出**，`E0624`）。
> 本 task 自己的判据是**编译**：Step 1 的消费者先改、Step 2 看它失败、Step 4 看全量绿。

- [ ] **Step 1: 把消费者先改到新形状（写「用例」这一步在本 task 就是改夹具）**

改 `crates/continuum-provider/tests/common/mod.rs` 里 `FakeTool::invoke` 的签名与导入——
请求参数写成 `AuthorizedToolInvocation<'_>`，`use continuum_provider::tool::AuthorizedToolInvocation;`。
`tests/fake_provider.rs` 里若有引用它的地方一并改。

- [ ] **Step 2: 运行，确认失败**

```bash
timeout 900 cargo build --workspace --all-targets
```

预期：**E0432 **（`AuthorizedToolInvocation` 尚未定义），且 `ToolInvocation` 那条改动还没做。
**这就是本 task 的红**——先写消费者、再见它编译不过。

- [ ] **Step 3: 实现**

```rust
// crates/continuum-provider/src/tool.rs

/// §316 `invoke` 的请求侧：**只装得下「已授权」**（设计 §7.5，裁决 §一第 2 条）。
///
/// 依据：`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` §一第 2 条——
/// 「改的是请求侧，不是响应侧」（响应在调用完成之后才存在，往那里加位置解决不了问题）。
///
/// **可见性三件套（裁决 C2，2026-10-05；设计 §7.5 的「可见性」段）**：
/// **类型 `pub`**——它出现在**公开 trait 的方法签名**里，必须 `pub`；
/// **构造入口 `pub(crate)`**——只有 `continuum-provider` 内的注册表构造它
/// （[`crate::Registry::invoke_tool`]），crate 外没有第二条；
/// **字段私有**——`authorization` / `input` 只经访问器读。
///
/// 两条性质：
/// 1. **没有 [`AuthorizedTool`] 就构造不出它**，而 `pub(crate)` 让这条**更强**：**crate 外的代码
///    即使手里有一枚真的 `AuthorizedTool`，也构造不出这个请求**——它只能把证明交给注册表，
///    由注册表替它构造。「谁构造」是注册表一个产生点，与「谁持有证明」（F）是两件事。
///    （照片：Task 6 的样例 4，预期 `E0624`。）
/// 2. **工具 id 只有一个来源**：授权证明的 `AuthorizedTool::tool_id()`。本类型**不另收 `ToolId`**，
///    故「出示的 id 与被授权的 id 不是一个」这一种可能**不存在**。（照片：Task 6 的样例 3。）
///
/// **为什么装的是 `&AuthorizedTool`，不是 `Credential`**：这里承载的是**强制点 (1) 的证明**，
/// 不是密钥材料。凭据是 `continuum-secrets` 的词汇、走连接器路径（设计 §7.5 末段；若将来确需把
/// 凭据交给模型/工具适配器，那是一个**第二个位置**，见 `## 遗留`）。
pub struct AuthorizedToolInvocation<'a> { /* authorization: &'a AuthorizedTool, input: Value —— 两者皆私有 */ }

impl<'a> AuthorizedToolInvocation<'a> {
    /// **唯一的构造入口，`pub(crate)`。** 收 `&AuthorizedTool` 而非 `ToolId`——这正是「没有授权就
    /// 构造不出这次调用」的落点。
    ///
    /// **为什么不是 `pub`**：构造点是 `ProviderRegistry::invoke_tool` **一处**（设计 §7.5）。
    /// 若它是 `pub`，任何持有一枚真 `AuthorizedTool` 的代码都能自行拼出请求、直接调裸适配器的
    /// [`crate::ToolProvider::invoke`]——那就是「同一件事两个产生点」，而两条路**都编译得过**。
    pub(crate) fn new(authorization: &'a AuthorizedTool, input: Value) -> Self;

    /// 这次调用获准了什么。**消费方是工具适配器**（`ToolProvider` 的实现）：它在 `invoke` 的
    /// 实现体内逐枚取 `Capability::scope()`，把这次动作限定在该作用域内。
    /// **F 只持有并下传整枚值，不读 `granted()`**（设计 §7.5；`AuthorizedTool::tool_id()` 由注册表取，用于路由）。
    pub fn authorization(&self) -> &AuthorizedTool;

    pub fn input(&self) -> &Value;
}

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError>;
    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError>;
    /// **请求参数已换**（设计 §7.5、裁决 §一第 2 条）。四项方法名与其余三项一字不动。
    async fn invoke(&self, call: AuthorizedToolInvocation<'_>) -> Result<ToolResult, ProviderError>;
    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError>;
}
```

`crates/continuum-core/src/tool.rs`：**删掉 `ToolInvocation` 那一段**，其余（`ToolId` / `ToolDescriptor` /
`ToolResult`）一字不动；文件头的模块文档若提到它，一并改。**不要**给它留 `#[deprecated]` 或注释掉——
裁决要的是删。

`Cargo.toml`：**normal 只加** `continuum-capability = { path = "../continuum-capability" }`
（理由注释：`invoke` 的请求参数要能命名 `AuthorizedTool`，层内边，设计 §4.1 订正段）。
**dev 边一律不加**——本 task 没有运行用例，用不到 `Tx`。

`dependency_direction.rs` 的 `ALLOWED` 里**`("continuum-provider", …)` 那一个条目**（按条目名找，不按行号）
改为 `("continuum-provider", &["continuum-capability", "continuum-core"])`，
并更新其上方注释（照 P3A 各 task 更新注释的写法，写明「Task 3 起加上 capability；persist 是 Task 4
的 dev 边，届时增量加」）。**这是本 task 的终值**，Global Constraints 里的三元素是**全计划终态**，
两者不矛盾：中间态必须与当时的 `Cargo.toml` 精确一致，否则逐对断言红。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
timeout 900 cargo build --workspace --all-targets
grep -rnw ToolInvocation crates/   # 预期零命中。**`-w` 不能省**：裸匹配会被 AuthorizedToolInvocation 子串命中
git add crates/continuum-core crates/continuum-provider crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(provider): §316 请求面换成 AuthorizedToolInvocation，删 ToolInvocation"
```

**预期**：全绿、0 warning。若 `cargo tree` 让 `every_crate_depends_only_on_its_allowed_set` 红，
**先核 `ALLOWED` 与 `Cargo.toml` 是否精确一致**（本仓最常见的一处失手）。

---

### Task 4: `invoke_tool`——唯一受门禁的调用入口

**Files:**
- Modify: `crates/continuum-provider/src/registry.rs`、`src/lib.rs`
- Modify: `crates/continuum-provider/Cargo.toml`（**dev** + `continuum-persist`、`tempfile`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（provider 条目加 `continuum-persist`，补成三元素）
- Create: `crates/continuum-provider/tests/invoke_tool.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProviderRegistry`、Task 3 的 `AuthorizedToolInvocation`、
  既有的 `continuum_capability::{AuthorizedTool, authorize, mint, Tool, ToolProfile, Trust, save_tool, p3_capability_migrations}`
- Produces: `ProviderRegistry::invoke_tool`

> **这是本计划交给 F 的硬交付之一**（另一个是 Task 3 的请求类型）。
> **工具侧唯一的调用入口**：注册表不交出 `Arc<dyn ToolProvider>`，故任何持有注册表的代码都只能经它调到工具
> （设计 §7.1；F 已据此删掉 `ToolCaller` / `invoke_authorized` / `select_tool_caller`）。
>
> **dev 边在这里加，不在 Task 3**：本 task 是第一个真要用 `Tx` 去调 `authorize` 的 task
> （「边由用它的那个 task 登记」，dev 边同理）。`Cargo.toml` 与 `ALLOWED` **同批改**——
> 只改一处会红（`p3bcdf-followups.md` §四.1 的硬提醒 1）。改完 `ALLOWED` 的 provider 条目即
> Global Constraints 里的**三元素终值**。

- [ ] **Step 1: 写用例**

`tests/invoke_tool.rs`。**夹具**照 `crates/continuum-capability/tests/authorize.rs` 的 `db()` 同形
（`tempfile` + `Db::open_with` + `builtin_migrations()` + `p3_capability_migrations()` + `migrate()`），
再 `save_tool` 登记一条工具、`mint` 铸一枚能力、`authorize` 取 `AuthorizedTool`。
**每个测试目标各写自己的一份 `db()`**（本仓既有做法：`continuum-capability` 的 `tests/authorize.rs`
与 `tests/persist.rs` 各有一份），**不做跨目标的共享夹具**。
适配器用 `tests/common/mod.rs` 里的 `FakeTool`。

- `a_registered_tool_is_invoked_and_its_result_returned`：**门禁内正常路径**（设计 §11）——
  起库 + capability 迁移 + `save_tool` + `authorize` 取 `AuthorizedTool` + `register_tool` →
  `invoke_tool(&auth, json!({"x": 1}))` 得 `Ok(ToolResult { output: {"x":1}, is_error: false })`。
  **这一条要真跑通**，不是「用既有夹具即可、不新建夹具」那个量级（设计 §9 已把这条的代价说准）。
  **它同时是「输入被原样带到适配器」这条性质的照片**（echo 适配器的 `output == 输入`）——
  Task 3 不再有构造该请求的集成用例，那条性质在这里观测（设计 §7.5）。
- `an_unregistered_tool_is_not_called_at_all`：注册表里没有这个 id 的适配器 →
  `matches!(_, Err(ToolCallError::Unregistered { .. }))`，**且不是 `Provider(..)`**。
  **这条只钉路由本身**（本文件的 `db()` 照用，但**不调 `register_tool`**）；「`tool` 表有行、注册表无适配器」那一向
  是**另一个命题**，它在 Task 5，**不在这里重复**。

  > **订正注记（2026-10-06，Task 4 实现者实测 + 协调者裁定采纳偏离）。**
  > **原文照留**：上面那半句「本文件的 `db()` 照用，但**不调 `register_tool`**」。
  > **它错在哪**：照它办，**这条守卫就失去了主语**——注册表里一个适配器都没有时，
  > 「先逐个问遍所有适配器、再判未命中」与「直接查表判未命中」这两版**问的是零个适配器**，
  > 于是那个本该被抓的**变异体退化成了等价变异体**（纪律 1(b)：举不出「这两版在哪个入参上会给出不同结果」），
  > **永远绿**——而派单要求它在变异下必须红。
  > **实现改成**：该用例**登记一个服务于另一个 id 的 `RecordingTool`**（它记录自己有没有被调用），
  > 断言 `invoke_tool` 仍返回 `Unregistered` **且 `recorder.touches() == 0`**。
  > 「被测 id 无适配器」这一条**没有被削弱**；被钉住的命题仍是**路由**那一件事，
  > 对 `tool` 表**仍是零主张**（本用例要避开的「`tool` 表有行、注册表无适配器」那一向仍在 Task 5，不在此重复）。
  >
  > **判据（本项目通用，不限于本条）**：**凡「某条用例不构造 X」这类指令，先问一句
  > 「**不构造它之后，我要钉的那个变异体还区分得出来吗**」**——把 X 拿掉如果让变异体**退化**
  > （两版在所有入参上给出同一结果），**那条守卫就等于没写**，而它在报告里仍会显示为绿。
  > 「为了让用例隔离，就不构造某样东西」是这类指令的常见形态，代价往往就落在变异体上。

  > **订正注记（2026-10-06，本计划扫同类 + 协调者裁定）。** **原话照留**：上面那半句
  > 「`matches!(…Unregistered { .. })`，**且不是 `Provider(..)`**」。
  > **它是「蕴含型」，从计划里删掉**：`ToolCallError` 的 `Unregistered` 与 `Provider`
  > **枚举互斥**，故前一条成立**即蕴含**它——它**不可能独立变红**，本该出声的输入
  > （报成 `Provider`）已被前一条抢先红掉，而前一条的消息里已带 `{err:?}`。
  > **落地现状**：Task 4 的实现者在派单下**已自行删掉**这一条（`tests/invoke_tool.rs` 里留了注记）。
  > **计划必须跟着改**，否则后来者照计划又写回去。
  > **删后承重不变**：本用例余下的是 `Unregistered` 那条断言与 `recorder.touches() == 0`
  > ——「未命中臂改成别的结果」的变异**仍使前一条红**。
- `the_adapter_failure_is_reported_as_provider`：适配器 `invoke` 返 `Err(ProviderError::Transport(..))` →
  `matches!(_, Err(ToolCallError::Provider(_)))`，**且不是 `Unregistered`**。

  > **订正注记（2026-10-06，本计划扫同类 + 协调者裁定）。** **原话照留**：上面那半句
  > 「**且不是 `Unregistered`**」——**它是「蕴含型」，删**：两个变体枚举互斥，前一条成立即蕴含它，
  > 它不可能独立变红。**落地现状：代码里仍在**（`tests/invoke_tool.rs` 的该用例里那一条
  > `!matches!(err, ToolCallError::Unregistered { .. })`；**按用例名 + 断言内容找，行号会漂**），
  > **协调者已另派删除**。**删后承重不变**：余下的 `Provider(Transport(..))` 那条仍钉着
  > 「适配器的失败原样经 `Provider` 透出」，`recorder.seen_tool_id() == Some(..)` 那条另钉
  > 「这一条走的确实是适配器那条路」。
- `the_adapter_is_handed_the_authorization_that_was_passed_in`：适配器侧读
  `call.authorization().tool_id()`，与 `auth.tool_id()` 相同——**证明路由用的 id 与被下传的授权是同一枚**，
  亦即「工具 id 只有一个来源」这条性质在**调用面**上的照片（设计 §7.5；Task 3 拍不到它，理由见 Task 3）。
  这条要一个「把收到的授权回吐出来」的适配器（`common` 里加一个记录型 fixture），
  因为**结果里没有任何回执字段可比对**（设计 §7.1 的限度：类型层管住「id 从哪来」，管不住「适配器照着它做」）。

- [ ] **Step 2: 运行，确认失败**

```bash
timeout 300 cargo test -p continuum-provider --test invoke_tool
```

- [ ] **Step 3: 实现**

```rust
impl ProviderRegistry {
    /// **工具侧唯一的调用入口**（设计 §7.1）。解析与调用是同一步。
    ///
    /// 工具 id **只有这一个来源**：`authorized.tool_id()`。本函数**不另收 `ToolId`**
    /// （旧类型 `ToolInvocation` 已裁删），故「出示的 id 与被授权的 id 不是一个」这一种可能不存在。
    ///
    /// **未登记的 id 不构成本次调用失败**，它是**配置缺陷**（`ToolCallError::Unregistered`）——
    /// 调用方**不得**把它当作可重试（设计 §5.1）。
    ///
    /// **它交出的是结果，不是适配器**：注册表**不提供** `tool_for`、也**不提供**
    /// `tool_providers()` 枚举（设计 §7.1）。那两条入口的缺席由 Task 6 的编译失败样例钉。
    ///
    /// **顺序**：先按 id 路由（未命中即返 `Unregistered`，**适配器一次都没被碰过**），
    /// 构造请求，再调适配器。`AuthorizedToolInvocation` **在本函数内部构造**——调用方（F）
    /// 只传 `&AuthorizedTool` 与 `input`，不构造也不命名那个类型（`p3bcdf-followups.md` §七 第 5 条）。
    /// 该类型的构造入口是 `pub(crate)`（Task 3，裁决 C2），故**本函数是全仓唯一的构造点**。
    pub async fn invoke_tool(
        &self,
        authorized: &AuthorizedTool,
        input: Value,
    ) -> Result<ToolResult, ToolCallError>;
}
```

**三条结果、不是两条**：工具跑起来了但自身失败 → `Ok(ToolResult { is_error: true, .. })`；
provider 没跑成 → `Err(ToolCallError::Provider(e))`；没有适配器 → `Err(ToolCallError::Unregistered { .. })`。
**第一条是 C 加在适配器上的约定，§316 里没有出处、也没有强制**——它的夹具照片在 Task 7，
且**只钉夹具、不钉真实适配器**（设计 §7.1）。本 task 的实现**不替适配器做这个分流**，
`Ok` / `Err` 一律按适配器给的转出去。

**同批改 `Cargo.toml` 与 `ALLOWED`**：dev 加 `continuum-persist` 与 `tempfile`（理由注释：本文件的用例
要构造 `Tx` 去调 `authorize`）；`dependency_direction.rs` 的 provider 条目补成
`["continuum-capability", "continuum-core", "continuum-persist"]`，并更新注释。
**两处一起改**——逐对断言跑 `--edges all`，dev 边也在内。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(provider): invoke_tool——工具侧唯一受门禁的调用入口"
```

---

### Task 5: 两个登记点不一致的两向（§3.3 代价三）

**Files:**
- Create: `crates/continuum-provider/tests/two_registries.rs`

**Interfaces:**
- Consumes: Task 4 的 `invoke_tool`、Task 2 的 `register_tool`；`continuum_capability::authorize`
- Produces: 无新公开面

> **设计 §3.3 代价三**：「工具存在两个登记点」——capability 的 `tool` 表（**治理**：要哪些能力、
> 什么 effect_class）与 `ProviderRegistry`（**路由**：谁去跑）。两个方向的错法都可达、各有可观察形态，
> **逐向一条，一个都不能少**：只写一向等于「守卫只钉一侧」，而缺的那侧正是 fail-open 的那侧
> （第一向就是 fail-open：没有适配器却被放行到调用）。

- [ ] **Step 1: 写用例**

`tests/two_registries.rs`（夹具同 Task 4 的 `db()`）：

- `a_tool_in_the_table_without_an_adapter_passes_authorize_then_fails_to_route`（**第一向，fail-open 的那侧**）：
  `save_tool` 登记了、注册表**没**登记 → `authorize` **返回 `Ok`**（拿到 `AuthorizedTool`），
  随后 `invoke_tool` 返回 `Err(ToolCallError::Unregistered { .. })`。
  **两半都要断言**：只断后一半会让「authorize 也能挡住它」这条假说法活下来。

  > **分工写死（2026-10-06，协调者裁定）：本用例是一条第「序列 + 结果」的用例，不承担路由机制的变异守卫。**
  >- **它钉的是**：`authorize` **通过**（返回 `Ok`）**而**路由**未命中**这条**序列**，以及「未命中报的是
  >  `Unregistered` 这一种 `Err`」这个**结果**。
  >- **它真正能红的变异体**（两条，都是本 task Step 3 的档）：**(i)** 把 `invoke_tool` 的未命中臂改成
  >  别的结果（如静默返 `Ok(ToolResult { is_error: true, .. })`，或返 `Provider(..)`）——前一半的
  >  `Ok` 断言与后一半的变体断言各会红；**(ii)** 让 `authorize` 那半失败（如把 `tool` 表的行撤掉）
  >  ——第一半的 `Ok` 断言会红。
  >- **有一条变异体在它上面红不了，据实写明**：**路由机制**那个变异体（「先逐个问遍所有适配器、
  >  再判未命中」）——本用例的注册表里**一个适配器都没有**，两版问的都是零个，**它在这里退化成
  >  等价变异体**。**那条机制的照片在 Task 4 的 `an_unregistered_tool_is_not_called_at_all` 上**
  >  （该用例登记一个服务于**另一个 id** 的 `RecordingTool`，并断言 `recorder.touches() == 0`）。
  >- **故本用例不另配 `RecordingTool`**：同一件事的**第二个落点**正是本仓反对的形状——路由机制的
  >  证据只有 Task 4 那一份，此处**只**承担序列与结果。**点名到用例名，不引行号。**
- `an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call`（**第二向**）：
  注册表**登记了**适配器、`tool` 表**没** `save_tool` → `authorize` 返回
  `Err(CapabilityError::UnknownTool { .. })`，**在 `invoke_tool` 之前**。
  断言里写明：这条路径上 `ProviderRegistry` 一次都没被碰过。

  > **订正注记（2026-10-06，Task 5 独立评审 + 本计划的产物瑕疵）。** **原话照留**：上面那半句
  > 「**断言里写明：这条路径上 `ProviderRegistry` 一次都没被碰过**」——**正是这一句生出了两条非守卫的断言**。
  > 按它写出来的实现（评审前的版本）在第二向里放了 `RecordingTool` 与
  > `assert_eq!(recorder.touches(), 0, …)`、`assert_eq!(recorder.seen_tool_id(), None, …)`。
  > **这两条是「恒真型」**：`touches` / `seen_tool_id` 由**用例自身的控制流**决定——本用例在
  > `authorize` 被拒之后**压根不发任何调用**，`register_tool` 也不碰适配器，故**没有任何被测代码的
  > 变异能移动那个计数**。它在所有变异下都绿，**不是守卫**。**处置：删**（它该出声的输入不存在）。
  > **删后承重不变**：第二向余下的是 `authorize` 报 `UnknownTool` 那一条——Step 3 的变异 2
  > （把 `save_tool` 挪到 `authorize` 之前）**仍使它会红**，那个方向仍被钉住。
  >
  > **同一条判据下另删的一条（第一向）**：第一向的实现（评审前）另写了
  > `assert!(!matches!(err, ToolCallError::Provider(_)), …)`。**它是「蕴含型」**：
  > `ToolCallError` 的 `Unregistered` 与 `Provider` **枚举互斥**，故前一条
  > `matches!(Unregistered { id } if id == "t1")` 成立**即蕴含**它——它**不可能独立变红**，
  > 而它本该出声的输入（报成 `Provider`）已被前一条抢先红掉（前一条的消息里已带 `{err:?}`）。
  > **处置同样：删**，且**不得靠调序**把它挪到前一条之前来「救活」——两条断言钉的是
  > **同一个**变异集，不是互不相交的两个。**删后承重不变**：第一向余下 `authorize` 成功（`Ok`）
  > 与 `Unregistered` 那条；Step 3 的变异 1（未命中臂改成别的结果）**仍使它会红**。
  >
  > **那条「惰性断言」据实标注（本计划自身的产物瑕疵，不删）**：本向的 `save_tool` 调用
  > **对本用例的断言是惰性的**（只有它的 `.unwrap()` 会跑），且位于全部断言**之后**——
  > 它存在只为让 Step 3 的变异 2 成为**字面意义的「挪」**（把这一句移到 `authorize` 之前）
  > 而非「凭空插入」。**这是计划 Step 3 定下的产物，保留并据实标注**；它不是守卫，别当证据读。
  >
  > **保留的对照——同一条断言文本，一处是守卫、一处不是**：Task 4 的
  > `an_unregistered_tool_is_not_called_at_all` 里有**同名**的 `recorder.touches() == 0`
  > 断言，**那边是真守卫**：该用例登记了服务于**另一个 id** 的 `RecordingTool`，
  > 「先逐个问遍所有适配器、再判未命中」的变异体会**碰它**而红。**差别只在位置上**——
  > 那边有「会被碰到的适配器」，这边没有。

两条用例是**同一条事实的两向**（设计 §3.3 代价三的表），故两条必须**同时存在**。

> **统一判据（2026-10-06，Task 5 独立评审给出；本项目通用，比「绝对措辞须有用例」更具体）**：
>
> **一条断言是守卫，当且仅当在它实际所处的位置上，存在一个「被测代码」的变异使它会红。**
>
> **更利落的最终形式**（评审后来给的，比上面这句更准）：
> 设同一条用例里断言 `B`，`A` 是**同体内排在 `B` 之前的全部断言**。
> **`B` 是守卫 ⟺ 能举出一个具体的、仍能编译的变异体 `M`，使得在 `M` 下 `A` 仍通过、而 `B` 失败。**
> **关键在「`M` 是否真的存在」，而不是「`M` 是否被这一轮跑过」**——举不出来的就是非守卫，
> 哪怕这一轮没人去试；举得出来的就是守卫，哪怕它今天恰好没红。
>
> 两种形态同等处置，**都删**：
> - **恒真型**：不存在这样的变异体（该断言由**用例自身的控制流**或**夹具的构造**决定，例如
>   「本用例从不发某种调用」时去断言「那种调用没发生」）；
> - **蕴含型**：变异体存在，但**同一体内在它之前的断言先红并蕴含它**（例如枚举两变体互斥时，
>   先断 `matches!(A)` 再断 `!matches!(B)`）。
>
> **不许靠调序制造第二个落点**，除非两条断言钉的是**互不相交的变异集**。
> **位置是判据的一部分**：同一条断言文本，在一处有「会被碰到的对象」时是守卫，在另一处没有时
> 就不是——本节第一向/第二向与 Task 4 那条的对照即是它的证据。
>
> **反向的误用要一并防住——下面四条按本判据核过，是守卫，别顺手删**（协调者另列「别误删」清单）：
> Task 4 `an_unregistered_tool_is_not_called_at_all` 的 `recorder.touches() == 0`（那边有**会被碰到的
> 适配器**，「先逐个问遍所有适配器」的变异体会碰它）；Task 4
> `the_adapter_failure_is_reported_as_provider` 的 `recorder.seen_tool_id() == Some(..)`；
> Task 1 `registering_the_same_id_twice_is_rejected_as_duplicate` 的「**原条目不变**」（`Arc::ptr_eq`）
> ——存在「**先插后判**」这一变异体使它**单独变红**（`Duplicate` 那条仍通过，而条目已被改写）；
> Task 8 的 `the_guard_sees_the_source_tree`（那是守卫**自身**的正控制，防的是「什么都没读到也全绿」）。

- [ ] **Step 2: 先写反的那一版，跑一次，确认两向都红**

本 task **没有新实现**——两向用的是 Task 4 的 `invoke_tool` 与 P3A 既有的 `authorize`，
故正确版本一写就绿，**没有红的状态可看**。红要这样做：**先把两向的可观察形态对调**——

- 「第一向」的用例里断言 `Err(CapabilityError::UnknownTool { .. })`（那是第二向的形态）；
- 「第二向」的用例里断言 `Err(ToolCallError::Unregistered { .. })`（那是第一向的形态）。

```bash
timeout 300 cargo test -p continuum-provider --test two_registries
```

预期：**两向都红**。这一步证明的不是实现，而是**这两条用例各自钉住了一个方向**
——若它们对调后仍然绿，说明断言根本没在查那个东西。确认之后**把两向改回正确的那一版**，
再跑一次，**两向都绿**。

- [ ] **Step 3: 变异（本 task 的承重守卫，走全量套件）**

两条变异，各用独立日志路径，读前确认为本轮所写：

1. **把 `invoke_tool` 的未命中臂改成「静默返回 `Ok(ToolResult{ output: Null, is_error: true })`」**
   → 第一向必须**确定性地红**（它断言的是 `Unregistered` 这一种 `Err`）。
   **判据**：`could not compile` 不算红；要的是该用例名的断言失败。
2. **把第二向的 `save_tool` 挪到 `authorize` 之前**（即表里有行）
   → 第二向必须红（它断言的 `UnknownTool` 不再出现）。这条钉的是「断言确实在查表」。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider
git commit -m "test(provider): 两个登记点不一致的两向照片"
```

---

### Task 6: 不可表达性的编译失败样例

**Files:**
- Modify: `crates/continuum-provider/Cargo.toml`（dev + `trybuild`）
- Create: `crates/continuum-provider/tests/compile_fail/*.rs`（五份，各配同名 `.stderr`）
- Create: `crates/continuum-provider/tests/type_level.rs`

**Interfaces:**
- Consumes: Task 1/2/4 的公开面、Task 3 的 `AuthorizedToolInvocation`
- Produces: 无新公开面

> **编译失败样例是本子项目对「类型强制」那一半的证据**（设计 §11），与 P3A 的 `tests/compile_fail/` 同形：
> 判据是**编译不过**，由 `tests/type_level.rs` 的 **glob** 驱动，每份带 `.stderr` 钉住预期报错。
> **它的证明力有边界**（设计 §11 末段已经收窄）：样例钉的是**写下来的那几个拼法**，
> **不是**「任何名字的公开入口都不存在」这一全称否定。故判据是**公开面清单 + 样例**两条一起，
> 清单由评审核对。**Plan 里不要写「已证明不存在」这类绝对措辞。**

- [ ] **Step 1: 写五份样例**

`tests/compile_fail/` 下（**每份的文件头写清它钉的是哪一条性质**，与 P3A 的样例同写法）：

1. `tool_for_does_not_exist.rs`：`ProviderRegistry::new().tool_for(todo!())` —— 公开面上**没有**
   交出裸 `Arc<dyn ToolProvider>` 的入口（设计 §7.1）。
2. `tool_providers_does_not_exist.rs`：`ProviderRegistry::new().tool_providers()` —— 同上，
   工具侧**没有**枚举适配器的入口。
3. `a_bare_tool_id_cannot_be_passed_to_invoke_tool.rs`：
   `registry.invoke_tool(&continuum_core::tool::ToolId::new("t"), serde_json::json!({}))` ——
   请求侧**只收 `&AuthorizedTool`**，裸 id 传不进去（旧形状 `ToolInvocation` 已删，裁决 §五）。
   **这一份是「删 `ToolInvocation`」在类型面上的照片。**
4. `a_foreign_crate_cannot_build_authorized_tool_invocation.rs`：**外部 crate 手里有一枚真的
   `AuthorizedTool` 也构造不出这个请求**（裁决 C2；设计 §7.5 的第一条性质、§11 的照片行）——
   样例写成「把一个 `&AuthorizedTool` **参数**递给构造入口」：

   ```rust
   use continuum_capability::AuthorizedTool;
   use continuum_provider::AuthorizedToolInvocation;

   fn build(auth: &AuthorizedTool) -> AuthorizedToolInvocation<'_> {
       AuthorizedToolInvocation::new(auth, serde_json::json!({}))   // 预期 E0624：`new` 是私有的
   }
   fn main() {}
   ```

   **预期报错是 `E0603`（associated function `new` is private），不是 E0061 / E0308。**
   这一点是本样例的判据：**「少传一个参数」也能编译失败**，但那是另一回事——
   本样例要钉的是「**有**授权也构造不出」，故 `.stderr` 必须落在 `E0603` 上。

   > **订正注记（2026-10-06，Task 6 实测 + 本计划扫同类）。** **原话照留**：上面那两句里的
   > **`E0603`**，以及代码块里那行注释 `// 预期 E0603：`new` 是私有的`。
   > **码错在哪**：`E0603` 是「**按路径**访问私有**条目**」（自由函数 / 模块成员）的码；
   > 本样例走的是 `AuthorizedToolInvocation` 的**关联函数** `new`，rustc 实报的是
   > **`E0624`：associated function `new` is private**。**两码之间只差「私有条目」与「私有关联函数」这一处。**
   > **代码块里那行注释随之改为 `// 预期 E0624`**，`.stderr` 钉的也是 `E0624`。
   > **那条判据的实质不变**：本样例钉的是「**私有**」（有授权也够不着构造入口），
   > **不是**「元数不合（`E0061`）」或「类型不合（`E0308`）」——这一点原话成立，未动。
   > **补一句（2026-10-06）**：**`E0061` 那一式未实测，故它在本处只作否定式对照、不作判据**
   > （`E0308` 另说：样例 3 的 `.stderr` 实跑就是它）。后来者别把「不是 `E0061`」当结论引用。
   > **独立复核（本计划作者自己跑的 rustc 探针，`--edition 2024`）**：
   > `mod m { fn f() {} } fn main() { m::f(); }` → **`E0603`**；
   > `mod m { pub struct S; impl S { fn g() {} } } fn main() { m::S::g(); }` → **`E0624`**。
   > **落地现状**：Task 6 的实现者已按实跑生成 `.stderr`（`E0624`），并把这段来历写进了样例文件的
   > 模块文档；**计划此处是跟着改**。
   >
   > **判据（本项目通用，写进原地）**：**凡「预期报错码」这类断言，必须来自实跑**——
   > `E0603` 与 `E0624` 只差「私有条目」与「私有关联函数」这一处，**凭印象写必错**。
   > 而 **`.stderr` 不会替你报错**：它是 rustc 的逐字输出，写错了它照样让用例绿——
   > 因为「因为别的原因编译失败」也会满足它。故**生成之后必须逐份读回**（见 Step 2），
   > 而不是只看 trybuild 通过。
5. `authorized_tool_invocation_fields_are_private.rs`：结构体字面量构造（字段名写对时）被拒 ——
   字段私有，绕开构造入口这条路不存在（与 P3A 的 `authorized_tool_cannot_be_built.rs` 同形）。

**样例文件里可以命名 `continuum_core` / `continuum_capability` / `serde_json`**：trybuild 会把
crate under test 的 **`[dependencies]` 与 `[dev-dependencies]` 一并**放进生成的工程
（`trybuild-1.0.121/src/run.rs:237-239`）。**实现时先跑一遍确认**，若某一项不在，把理由记进报告再调整样例。

- [ ] **Step 2: 生成 `.stderr` 并逐份人工核对**

```bash
cd crates/continuum-provider
TRYBUILD=overwrite timeout 600 cargo test --test type_level
```

**生成之后必须逐份读 `.stderr`**，确认报错**就是**该样例要钉的那一条，**不是**「拼错名字」「缺 `main`」
「用了不存在的 crate」之类的别的错——否则「因为别的原因编译失败」也会让用例变绿（P3A 的同一条纪律）。
**核对结论要写进 task 报告（逐份一句）。**

- [ ] **Step 3: 运行，确认**

```bash
timeout 600 cargo test -p continuum-provider --test type_level
```

预期：五份全部按 `.stderr` 失败。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider
git commit -m "test(provider): 不可表达性的编译失败样例"
```

---

### Task 7: `is_error` 与 `Err` 的分流（**约定**，非规范）

**Files:**
- Modify: `crates/continuum-provider/tests/common/mod.rs`（`FakeTool` 加失败通道）
- Create: `crates/continuum-provider/tests/contract.rs`

> **订正注记（2026-10-06，Task 7 复核；原话照留）。** **这份清单漏了项**：把 `FakeTool` 三态化
> **打断了七个构造点、横跨四个文件**（下面按**用例名 / 代码内容**写；行号只是当时那一次读数的快照）：
>
> | # | 文件 | 那一个构造点 |
> |---|---|---|
> | — | `tests/common/mod.rs` | 定义本身：`FakeTool` 由单元结构体改为带 `outcome` 字段，**外加三个构造入口** |
> | 1 | `tests/fake_provider.rs` | `fake_implementations_satisfy_the_frozen_interfaces` 里那个 `let t = FakeTool;` → `FakeTool::echo()` |
> | 2 | `tests/invoke_tool.rs` | `a_registered_tool_is_invoked_and_its_result_returned` 里的 `register_tool(.., Arc::new(FakeTool))` |
> | 3 | `tests/registry_tools.rs` | `a_registered_tool_is_described_by_id` 里那处 |
> | 4 | `tests/registry_tools.rs` | `an_unregistered_tool_id_is_reported_as_unregistered` 里那处 |
> | 5 | `tests/registry_tools.rs` | `list_tools_reports_an_adapter_failure_as_provider` 里那处 |
> | 6 | `tests/registry_tools.rs` | `describe_tool_routes_by_registration_not_by_the_adapters_list_tools` 里那处 |
> | 7 | `tests/two_registries.rs` | `an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call` 里那处 |
>
> **正文原先只点了 `fake_provider.rs` 那一处**（在 Step 1 里），其余六处没有任何指令**——
> 它们是**编译错误逼出来的**（`FakeTool` 不再是单元结构体，`Arc::new(FakeTool)` 一处都过不去）。
> **这是清单类缺陷的典型形态：漏项不靠读清单发现，靠编译器发现**；但**清单漏了就得补**，
> 否则下一次改动的人仍要从编译错误里反推。**判据：凡动公开夹具的构造形状，先 grep 一遍它的构造点
> （`grep -rn "FakeTool" crates/continuum-provider/tests/`），把命中的用例逐个列进 Files。**

**Interfaces:**
- Consumes: Task 4 的 `invoke_tool`
- Produces: 无新公开面

> **这条约定没有规范出处、也没有强制**（设计 §7.1）：「工具级失败走 `Ok(is_error: true)`、
> provider 级失败走 `Err`」是 **C 写在适配器契约上的约定**，适配器完全可以对工具级失败返
> `Err(Protocol)`。**本 task 钉的是夹具对这条约定的服从，不是真实适配器**——
> 报告里不许写成「适配器都这么干」。真实适配器上这条分流**拍不到**（设计 §9）。

- [ ] **Step 1: 改造 `FakeTool`**

`tests/common/mod.rs` 的 `FakeTool` 从单元结构体改为带一个失败通道字段，三个构造入口：
`FakeTool::echo()`（今天的行为）、`FakeTool::failing_at_tool_level()`（`Ok(ToolResult { is_error: true })`）、
`FakeTool::failing_at_provider_level()`（`Err(ProviderError::Transport(..))`）。

**补一处计划缺口（2026-10-06，Task 7 复核）：工具级失败那一支的 `ToolResult.output` 该取什么值，
本节原稿与设计都没给。** 本计划取**一个固定的、非回显的值**（实现里取
`json!({"error": "工具跑起来了但自身失败"})`），理由两条：
**(a)** 若让它**回显 `input`**，`output` 会在两臂上**完全相同**，`is_error` 就成了两条用例之间
**唯一的区分轴**——而本 task 要钉的正是「两条不是等价变异体」，把区分轴收窄到一根，
等于让「`output` 那一路也分得开」这件事**没有照片**；**(b)** 回显还会把一个**用例自造的载荷
升格成看似规范的东西**（读代码的人会以为「工具级失败时 `output` 就是输入」是一条约定，
而它哪里都不是）。**标明：这是本计划的选择，不是规范要求**——§316 对 `output` 在失败时的取值
**没有任何规定**，适配器可以任意取；取固定值只是让本用例的断言更硬。
`tests/fake_provider.rs` 里**那个 `let t = FakeTool;`**（`fake_provider.rs` 的第二个 `#[tokio::test]` 之前的
那个用例体内；**按那一行代码找，不按行号**）改为 `FakeTool::echo()`。

**为什么不写成三个结构体**：设计 §11 的这一行写的是「用 `FakeTool`」，而一个单元结构体产不出两条失败通道；
三态化是让那一行字面成立的最小改动。**这是本计划对夹具形状的读数，不是设计给的判据**，见 `## 遗留`。

> **本夹具与 F 的库级夹具是同一约定的两份副本，裁决明写「不合并」**
> （`p3bcdf-followups.md` §七 第 8 条）。理由：跨 crate 的 `tests/` 目录不可互相导入，F 的库级用例
> 装进本注册表时只能另写一份（它按本文件的形状写）。**两份各有其用，不要试图抽公共 crate。**
> 代价据实记：**两者若漂移，「工具级失败走 `Ok(is_error: true)`」这条约定只在本 crate 的用例上红**，
> F 那份不会。这条不设护栏，靠评审。

- [ ] **Step 2: 写用例**

`tests/contract.rs`（夹具同 Task 4 的 `db()`）：

- `a_tool_level_failure_is_ok_with_is_error_true`：适配器返工具级失败 →
  `invoke_tool` 得 `Ok(ToolResult { is_error: true, .. })`，**不是 `Err`**。
- `a_provider_level_failure_is_err`：适配器返 provider 级失败 →
  `invoke_tool` 得 `Err(ToolCallError::Provider(_))`，**不是 `Ok(is_error: true)`**。

  > **这两句「不是 …」是期望的措辞，不是第二条断言（2026-10-06，协调者裁定）。**
  > 写成 `assert!` 就是**蕴含型**：`Ok(..)` 与 `Err(..)` 是对立的两支，前一条
  > `matches!(Ok(ToolResult { is_error: true, .. }))` 成立即蕴含「不是 `Err`」，
  > 反之亦然——**两者都不可能独立变红**。故**每个用例只写一条断言**，
  > 用 `matches!` 把要钉的那一支与它的载荷一并断在**同一条**里（`is_error` 的真假即载荷）。
  > 这条口径与 Task 5 Step 1 的「统一判据」段同源。

**这两条不是等价变异体**，判据（纪律 1(b)）是「这两版在哪个入参上会给出不同结果」：
把 `invoke` 的实现从 `Ok(ToolResult { is_error: true, .. })` 改成 `Err(Protocol)`（或反过来），
在**同一条输入**上给出的是**不同的结果**（`Ok` vs `Err`）。写报告时把这句写出来，
不要只说「补了两条用例」。

文件头写明：**本文件钉的是夹具，不是真实适配器**（设计 §7.1、§9），
并写明这条约定的出处是 C 而非 §316。

- [ ] **Step 3: 运行，确认失败**

```bash
timeout 300 cargo test -p continuum-provider --test contract
```

> **订正注记（2026-10-06，Task 7 复核；原话照留）。** **上面 Step 1 → Step 2 → Step 3 作为一个
> 「红—绿」序列是执行不了的**：**被测对象正是 Step 1 自己引入的夹具**——照写，Step 3 一跑就绿，
> **没有红可看**。这不是「红写得少」，而是**这个序列在本 task 上根本不成立**。
>
> **实际做法（实现者的拆分，评审判为对意图的正当读法）**：**先落夹具的「结构」**——枚举、
> 三个构造入口、以及上面表里那七个调用点**全部到位**，**而 `invoke` 暂不读 `outcome`
> （一律走 `Echo` 那一支）**；于是**两条用例各自在自己的断言上红**（工具级那条拿到
> `is_error: false`、provider 级那条拿到 `Ok` 而不是 `Err`）；**再把 `invoke` 接上 `outcome`**，
> 两例转绿。**两次红都不是编译失败**，都是断言失败，且**各自红在该用例自己的断言上**。
>
> **判据（本项目通用，写进原地）**：**当被测对象在同一个 task 里被造出来时，「先写用例看它红」
> 这个序列产不出红——红必须来自一个中间态（结构在位、行为未接）。**
> **判据是「红来自断言、且红在该用例自己的断言上」，不是「必须按某个步骤顺序」。**
> **反过来也成立**：若某个 task 声称「先写用例、再看它红」而被测对象正是它自己新建的，
> 那么那个「红」**要么来自中间态、要么就是假的**——`file not found`、`could not compile`
> 都不是这个意义上的红（与纪律 1(c) 同一条判据）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider
git commit -m "test(provider): is_error 与 Err 的分流（夹具约定）"
```

---

### Task 8: 模块面中立性守卫

**Files:**
- Create: `crates/continuum-provider/tests/neutrality.rs`

**Interfaces:**
- Consumes: 无（只读 `crates/continuum-provider/src/` 的源码文本）
- Produces: 无新公开面

> **这一条挡的是设计 §4.1 末段 / §4.2 形态 4 的那条全绿路径**：把一份具体适配器**写进
> `continuum-provider` 自己内部**（如 `src/deepseek.rs`）——它不引用任何外部 crate 的实现类型，
> 故 §4.2 形态 2 的机制不触发；`ALLOWED` 一字不改；`core_and_persist_do_not_depend_on_provider` 照绿；
> `src/lib.rs` 开头那句「本 crate 只定义 trait，不含实现」（**按这句话找，不按行号**）只是文档，
> 今天没有任何用例钉它。**结果：中立 crate 里躺着一份实现，
> 而一片全绿。这条路径今天没有别的东西挡。**

- [ ] **Step 1: 写用例**

`tests/neutrality.rs`：

- `the_guard_sees_the_source_tree`（**守卫自身的正控制，先钉机制再看结论**）：递归读
  `crates/continuum-provider/src/` 下全部 `.rs`，断言**至少读到 4 个文件**
  （`lib.rs` / `model.rs` / `tool.rs` / `connector.rs` / `registry.rs`）**且拼起来的文本里能找到
  `trait ModelProvider`**。**没有这一条，一个「什么都没读到」的守卫会永远绿**——这正是
  「守卫须两侧都钉」里缺的那一侧。
- `the_guard_folds_whitespace_before_matching`（**归一化自身的照片，必须先有**）：对
  `impl  ModelProvider for`（**两个空格**）、`impl\tModelProvider for`、`impl\nModelProvider for`
  三种输入，归一化函数都要折成 `impl ModelProvider for`。**没有这条，归一化写错了也没人知道**
  ——而它正是裁决 C7 要的那一件事。
- `the_neutral_crate_contains_no_implementation`：**折叠空白之后**的文本里**不出现**下面三个字面拼法：
  `impl ModelProvider for`、`impl ToolProvider for`、`impl Connector for`。
  断言信息里要**列出命中的文件名与行号**，否则红的时候读不出是哪一处。

**归一化是硬要求，不是可选**（裁决 C7，2026-10-05；设计 §4.1 末段：「**必须先做空白归一化**……
**留一种不归一就是没关严**」）：匹配前**把连续空白（空格 / `\t` / `\n`）折成一个空格**再比对。
上面那三种写法因此与一个字面拼法**是同一个拼法**，**不再是逃逸面**。

**归一化之后仍逃逸的三种**（它们不是同一个拼法，而是换了写法；设计 §4.1 末段）：
全限定 trait 路径（`impl crate::model::ModelProvider for DeepSeek`）、`use … as` 别名后实现、
把实现 `include!` 进来。故归一化后的守卫仍是**一个下界，不是封闭判定**；这三种写法**没有照片**，
落在评审。上界要解析 trait 路径（需要 `syn` 之类的新 dev 依赖），**本阶段不做**（设计 §12 第 21 条）。
**文件头要把上面这两段一并写明**：归一化关掉了什么、还剩哪三种。

- [ ] **Step 2: 运行，确认通过（这是本 task 的正常态）**

```bash
timeout 300 cargo test -p continuum-provider --test neutrality
```

**本 task 的红要这样做**（否则「守卫恒绿」无法与「守卫有效」区分）：
临时在 `src/` 下加一个 `struct Probe;` 与 **`impl  ModelProvider for Probe { … }`——两个空格**（**不提交**），
跑 `cargo test -p continuum-provider --test neutrality`，**确认它红且报出文件名与行号**，再删掉。
**故意用两个空格**：这样这一次红同时是**空白归一化生效**的照片（若归一化漏了，它会假绿）。
**这次红要留日志路径**（纪律 1：每次变异用独立日志路径）——它是这条守卫唯一的物证。
若这一步**不红**，先查归一化，**不要**去改字面拼法。

- [ ] **Step 3: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider
git commit -m "test(provider): 模块面中立性守卫"
```

---

### Task 9: 收尾与复核

**Files:**
- Modify: `docs/superpowers/p3bcdf-followups.md`（把实现期新发现的残余折进去——**不新建第二个同类文件**）

- [ ] **Step 1: 全量验证**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
timeout 900 cargo build --workspace --all-targets
timeout 600 cargo tree -p continuum-provider --depth 1 --edges all --prefix none
```

预期：全绿、0 warning；`cargo tree` 的直接边**恰为**
`continuum-capability` / `continuum-core` / `continuum-persist` 三个 workspace 成员
（外加 `async-trait` / `serde_json` / `tokio` / `futures-core` / `tempfile` / `trybuild` 六个外部 crate——
它们**不进 `ALLOWED`**，那张表只逐对断言 workspace 成员之间的边）。

- [ ] **Step 2: 逐条核对完成判据（找不到证据的不得标注为覆盖）**

| 判据（设计 §11 的行） | 照片 |
|---|---|
| 模型侧登记后按 id 发现 | Task 1 `a_registered_model_is_found_by_id` |
| 模型侧：登记 id 与 `list_models` 不一致（§3.3 代价一） | Task 1 `a_registered_id_may_be_absent_from_the_adapters_own_list_models` |
| 工具侧：公开面清单里没有返回裸适配器的入口 | 公开面清单（**评审读**）+ Task 6 样例 1、2 |
| 工具侧：授权证明进不到该进的地方 | Task 6 样例 3（裸 `ToolId` 传不进 `invoke_tool`）、样例 4（**外部 crate 持真 `AuthorizedTool` 仍构造不出**，`E0624`）、样例 5（字段私有） |
| 工具侧：门禁内的正常路径 | Task 4 `a_registered_tool_is_invoked_and_its_result_returned`；**新请求类型的两条性质也由它与其兄弟用例观测**（Task 4 的 `the_adapter_is_handed_the_authorization_that_was_passed_in`） |
| 工具侧：`is_error` 与 `Err` 的分流（**约定**，非规范） | Task 7 两条（**钉夹具，不钉真实适配器**） |
| 两个登记点不一致，向一 | Task 5 `…passes_authorize_then_fails_to_route` |
| 两个登记点不一致，向二 | Task 5 `an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call` |
| 中立性（模块面，§4.2 形态 4） | Task 8（**匹配前折叠空白**，裁决 C7）+ 它的正控制 `the_guard_sees_the_source_tree`；仍是**下界**，三种换写法的逃逸写在文件头 |
| 模型侧未登记 id | Task 1 `an_unregistered_model_id_is_reported_as_not_found` |
| 工具侧未登记 id | Task 2 `an_unregistered_tool_id_is_reported_as_unregistered` |
| 重复登记同一 id | Task 1 `registering_the_same_id_twice_is_rejected_as_duplicate` |
| 模型侧枚举 | **无此项**：`model_providers()` 已裁删（2026-10-06），设计 §11 的对应行已删。**不得标注为已覆盖** |
| 中立性（核心不依赖 provider） | 既有 `core_and_persist_do_not_depend_on_provider` |
| 中立性（`ALLOWED` 逐对精确） | 既有 `every_crate_depends_only_on_its_allowed_set` |
| `cancel` 对非流式不可达 | **结构事实**，照片是类型签名（设计 §5.3、§9），**不写运行用例** |
| `ToolInvocation` 的删除 | Task 3 Step 2 的 `cargo build --workspace --all-targets` 失败 + Step 4 全量绿 |

- [ ] **Step 3: 复核「无生产调用方」这一栏（据实记，不粉饰）**

逐项核对设计 §9 的表：`ProviderRegistry`、`ModelProvider` 的任何方法、`ToolProvider` 的任何方法、
§5.1 的建议映射表。**本计划交付之后生产调用方仍然为零**（模型侧等子项目 G、工具侧等 F），
**逐项列出并写明**：这是设计已声明（§1 订正段、§6、§12 第 12 条），**不是漏接线**。

- [ ] **Step 4: 残余落到有版本的文档**

`.superpowers/` 是 gitignore 的，**只写在报告或 ledger 里的结论会随 branch 消失**。
把本计划 `## 遗留` 里**属于实现期新发现**的那几条（尤其是 Task 1 / Task 2 / Task 3 的三处「设计未给判据」的读数）
折进 `docs/superpowers/p3bcdf-followups.md` 的**第三节**（仍开着的规范级缺口），
**并标明它们是「实现期取读」而非「规范给的判据」**。

- [ ] **Step 5: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs: P3 子项目 C 的收尾与复核"
```

---

## 遗留

```
（一）规范没给判据的，本计划一律不发明

非效应臂的强制点归属    §125 的读/草稿类操作（GitHub.read_repo / Email.draft）的强制点落在哪里，
                      规范未规定。C 不涉及（那是 B 的 §124 路径），列此以免读者以为漏了。
                      收件人：规范维护者（p3bcdf-followups §三.1）。
「一条操作该绑哪一枚   CapabilityKind 有 12 枚、for_effect 的像只有 6 枚，规范没有「配得对不对」的判据。
  CapabilityKind」       本计划不涉（C 不绑 kind），列此。收件人：规范维护者（§三.2）。
effect_class 两轴        tool 的 effect_class 是否与 Journal 的 EffectType 是两个轴——设计 §12 第 11 条
                      维持 P3A 的绑定、不改；本计划不改任何一边。收件人：F（工具调用路径）+ 规范维护者。
Trust / 工具侧          三者同属「工具选择」，而 §4.1 里没有「工具选择」这个组件，故没有天然收件人；
cost / latency          D 只接了两个（模型侧），trust 退件。本计划不读它们。
                       收件人：规范维护者（§4.1 无该组件）。

（二）设计已定案、本计划据此不设 task 的

非流式不可取消          InvokeResponse / ToolResult / AuthorizedToolInvocation 均不带可取消句柄，
                       cancel 的有效射程只有 stream()。照片是类型签名本身，不写运行用例（设计 §5.3、§9）。
usage() 的语义未定义     且无消费方（调用方是子项目 G，尚不存在）。本计划不碰它。收件人：子项目 G。
Runtime → provider      悬空边（Cargo.toml 声明了、runtime 内引用为零）。**边由用它的那个 task 登记**
                        ——使用点是 F。本计划不登记、也不删。收件人：F。
describe_tool 无        本阶段不加变体（按显式登记，正常路径上不可达）。F 接上之后「工具表里有、
  UnknownTool 变体      适配器不认」会变成可达，届时再定。收件人：F / 本 crate 的后续轮次。
input_schema 两个产生点  适配器的 ToolDescriptor.input_schema（调用的权威）与 Tool.input_schema（规划的权威）
                       的一致性无可强制。收件人：F（调用侧）+ Planner。
ExecutionProfile 的      `crates/continuum-graph/src/execution.rs:20-22` 的 model / provider / tool 仍是
  model/provider/tool  `Option<String>`，P1 预告 P3 接入 provider 时收紧为强类型 id。设计 §2.2 第五条与
  仍是 Option<String>   §12 第 9 条判**本轮不做**（**没有消费方要求它**：无 G、无 F）。
                       **实情（已按现文件核过，2026-10-06）**：`execution_profile` 表
                       （`crates/continuum-graph/src/persist.rs:53-62`）的列是
                       `graph_id / node_id / attempt / backend / timeout_ms / retry_policy / cost_budget`
                       ——**没有 model / provider / tool 三列**；全仓**没有 `save_execution_profile`**
                       （`grep -rn save_execution_profile crates/` 零命中，实测），**那张表今天没有写入方**；
                       `ExecutionProfile` 也**零生产构造点**（只有 `Default`）。
                       故收紧的代价是「**改 struct + 必要时加一条加列的迁移**」，比「动一张已落库的表」小。
                       **来历**：本计划初稿照设计初稿写「要动一张已落库的表与一个已冻结的 struct」，
                       **那是假的**，设计 §2.2 五与 §12 第 9 条已于 2026-10-06 就地订正。
                       **整套终审 S-7**：设计把收件人写成「子项目 G **或 F**」，那两个具名收件人**都不认领**，
                       四份计划零落点。**本计划的判定：不属 C** —— 收紧的是 `continuum-graph` 的
                       `ExecutionProfile`，C 的交付面里没有一个字段是它（**C 连 `continuum-graph` 都不依赖**）；
                       「谁先发 invoke 谁收紧」的主语是**模型调用路径**，即**子项目 G**，而 G 本轮不设计
                       （裁决 §一第 1 条）。故本条**明确记为未决**：**收件人：子项目 G（它尚不存在，
                       此条随它的设计一并处置）；若 G 落地前有人要动它，由规范维护者裁。**
                       本计划**不设 task**，不动那个 struct，也不动那张表。
ProviderError →         §5.1 那张表是**文档不是代码**（无消费方）。另注：FailureClass 有 Resource，
  FailureClass          而 ProviderError 没有表示「限流 / 配额耗尽」的变体，该类丢掉了。
                       收件人：F（工具路径）+ 子项目 G（模型路径）。
P1 的 Resource 义务     要求「P3 的 Router 必须为 RESOURCE 显式给出 max_attempts >= 2 与退避参数」，
                       而 D 说重试不属本层、F 的工具路径不调 decide_retry、C 把它折进上一条推走。
                       **须点一个所有方**（设计 §12 第 18 条）。收件人：控制器。
实现被写进中立 crate     §4.2 形态 4 的那条路径**没有行为照片**；守卫是 `src/lib.rs` 开头那句「不含实现」的
  内部                   定位声明
  内部                   + Task 8 的模块面断言 + 评审。Task 8 之后那三个字面拼法（**含空白变体**，
                       归一化之后是同一个拼法）会被红掉；**全限定路径 / 别名 / include! 三种仍全绿**
                       （设计 §4.1 末段；裁决 C7 已把空白变体从逃逸清单里划掉）。
save_tool 登记期不变量   C 不接（设计 §7.4 接缝二、§12 第 15 条）：它约束的是 tool 表的**写入**，
                       属 continuum-capability。**裁决：所有者是 F**（`p3bcdf-followups.md` §七 第 1 条）。
凭据要不要也交给         设计 §12 第 23 条：本阶段不做；若做，会引入 continuum-provider
  适配器               → continuum-secrets 的边，那是一个**第二个位置**。收件人：控制器（若提出）。

（三）实现期读数：设计/裁决未给判据，本计划取了读数（**须复核**）

只读入口的返回错误类型   **已由裁决 C1 定案，不再是未决**：设计 §3.5 那一行原写「受门禁的
                       invoke_tool 用」，与 §3.1「工具侧未命中（含 describe_tool）返回
                       ToolCallError::Unregistered」两说相抵；**裁决取 §3.1，设计 §3.5 已就地订正
                       并在原地留了来历**。本计划的取法与裁决一致（按 id 查的两个工具侧入口返
                       ToolCallError；list_tools 不按 id 查，其 Unregistered 臂不可达）。
                       本条列此只为把来历留全，**不需要再裁一次**。
RegistryError::Duplicate  **裁决 C5 明写归遗留、不发明**：字段形状规范未给判据。本计划取
  的字段形状            `Duplicate { id: String }`——两个 id 类型（ModelId / ToolId）共用一个变体，
                       是哪一个登记表（模型侧 / 工具侧）由调用点可知。另：它与 OperatorError::Duplicate
                       （crates/continuum-operator/src/registry.rs:10-11）**同的只是「有一个具名
                       Duplicate 变体」这一取向，不是字段形状**（那里两个字段都是强类型）。
register_* 的原子性      设计未写「一组 id 中有一个撞车时是否部分登记」（裁决 C3 接受计划的取法）。
                       本计划取**先全查后全插**（一个都不登记），照片在 Task 1。
                       设计若判部分登记可接受，那条用例要改写。
「模型侧枚举」这条      **已不再是遗留项**：设计在实现进行中（2026-10-06）把 model_providers()
  未决项已消失          **删掉**了（裁决 C4 的那条问句随入口一起消失——没有入口就没有「枚举粒度」）。
                       理由三条（设计 §3.1 裁决段）：零消费方 / 形状不带 id 与 G 按 id 解析对不上 /
                       「有哪些模型」的权威在 D 的 Model Registry 表。**代价**：将来要「跨全部适配器聚合」
                       得重新提出这个入口。本计划 Task 1 已同步（Produces、用例、结构体字段注释、
                       设计 §11 对应行）。**收件人：无（已闭）。**
list_tools 的并集语义    设计只说「并集」，未写同一 ToolId 由多个适配器声明时保留哪一条（裁决 C4）。
                       本计划取「首次出现者胜」。**工具侧仍有枚举入口（`list_tools`），故这条留下。**
FakeTool 三态化          设计 §11 写「用 FakeTool」，而一个单元结构体产不出两条失败通道。
                       本计划把它改成带失败通道的三态夹具（Task 7）。**是对夹具形状的读数，不是判据。**
AuthorizedToolInvocation  裁决 C2 已定：**构造入口 pub(crate)**，crate 外即使持一枚真
  的构造入口            AuthorizedTool 也构造不出（照片：Task 6 样例 4，**`E0624`**——**原写 `E0603`，2026-10-06 按实跑订正**）。故它**不是**未决，
                       也不再有「两条路都编译得过」的残留——设计 §7.5 说的「构造点一处」由此成立。
                       仍成立的残留只有一条（设计 §7.1）：**适配器照着 id 做**这件事类型层管不住。
模块面守卫的第四种逃逸    **已由裁决 C7 关掉**：守卫在匹配前折叠空白（空格 / tab / 换行折成一个空格），
                       `impl  ModelProvider for` 一类空白变体因此**是同一个字面拼法**，不再逃逸。
                       **仍逃逸的只剩三种**（全限定路径 / 别名 / include!），设计 §4.1 末段列明。
                       归一化本身的照片在 Task 8 的
                       `the_guard_folds_whitespace_before_matching`，以及 Step 2 那条
                       「两个空格」的探针（归一化若漏了，它会假绿）。
```

**另有一处本计划**没有**做、且必须点明的**：设计 §7.5 落地清单的第 4、5 处
（共享面 §二 §316 段的类型清单删 `ToolInvocation`、`docs/02-工程.md` §10.3 的接口清单标注请求面已变）
**由控制器改，本计划不动那两个文件**。实现完成时若那两处仍未改，**不得**把它们标为已覆盖。
