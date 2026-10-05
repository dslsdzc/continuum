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
`continuum-capability`（`AuthorizedTool` / `authorize` / `ToolId`）；`async-trait`；`serde_json`；`thiserror`；`std::sync::Arc` + `std::collections::HashMap`；
dev-only：`continuum-persist`（`Tx`）、`tempfile`、`trybuild`；`tokio`（已在 dev-dependencies）。

**设计依据：** `docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md`（下称「设计」）。
前提是共享面 `docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md`；裁决在
`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md`（**第五节：删 `ToolInvocation`**）；
接缝裁决在 `.superpowers/sdd-p3bcdf/impl-seam-map.md` **第六节**。

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。** 本计划唯一的边表改动是 `continuum-provider` 那一条。
  终值（设计的唯一产生点是 §10，本节不复述理由）：
  ```rust
      ("continuum-provider", &["continuum-capability", "continuum-core", "continuum-persist"]),
  ```
  `crates/continuum-provider/Cargo.toml` 与 `crates/continuum-runtime/tests/dependency_direction.rs:29` 的
  这一条**必须精确一致**——断言是逐对 `assert_eq!`，且 `every_crate_depends_only_on_its_allowed_set`
  跑的是 `cargo tree --edges all`，故 **dev-dependency 也进表**。
  **只加 `Cargo.toml` 不加 `ALLOWED`（或反之）会红**（`p3bcdf-followups.md` §四.1 的硬提醒 1）。
- **本计划不新建 crate、不建表、不取迁移号**（设计 §10）。迁移编号只对 D 存在（接缝图 §二 #3），
  C 一行都不碰 `runtime_migrations()` / `expected_migrations()` / `tests/startup.rs`。
- **本计划不改 `continuum-runtime` 的源码。** 对 `continuum-runtime` 的改动**只有一处**：
  `crates/continuum-runtime/tests/dependency_direction.rs` 里 `continuum-provider` 的那个条目。
  `:121` 那句已成假的注释**由 F 订正**（接缝图 §一 F 表末行），本计划不动。
- **`continuum-provider` 不引用任何适配器实现类型**（设计 §4.1）。注册表只用
  `std::collections::HashMap` 与 `std::sync::Arc`，引用的类型是 `Arc<dyn ModelProvider>` /
  `Arc<dyn ToolProvider>` / `ModelId` / `ToolId` / `AuthorizedTool`——**无一是实现类型**。
  若注册表引用了任何具体适配器，编译期就会要求一条新边，而那条边必须先写进 `ALLOWED` 才绿。
- **`ToolId` 一律复用 `continuum_core::tool::ToolId`，本计划不另建第二个**（P3A 设计 §3.1、接缝图 §二）。
- **`Tool`（§252）与 `ToolDescriptor`（§316）并存，不合并**（设计 §8）——本计划不改任何一边的字段。
- **本计划不接 `save_tool` 的登记期不变量。** 那条不变量（`effect_class == Some(t) ⇒ for_effect(t) ∈ required_capabilities`）
  约束的是 `tool` 表的**写入**，协调者已裁定**所有者是 F**（接缝图 §六第 1 条）。本计划一行都不写它。
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
3. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 Err」。本计划的失败面有三处，**一个都不能只写 `is_err()`**：
   `RegistryError::NotFound` / `RegistryError::Duplicate` / `ToolCallError::Unregistered` / `ToolCallError::Provider`。

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
`crates/continuum-capability/tests/authorize.rs` 的 `db()`、`crates/continuum-runtime/tests/dependency_direction.rs:29`。

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

**依赖已交付（全部在 `main` 上，本计划不用再造）：**

- `continuum-capability`：`AuthorizedTool`（`tool_id()` / `granted()`）、`authorize`、`mint`、`Capability`、
  `CapabilityKind`、`Tool` / `ToolProfile` / `Trust` / `Cost` / `Latency`、`save_tool` / `load_tool` /
  `p3_capability_migrations`、`ToolId`（**再导出** `continuum_core::tool::ToolId`）。
- `continuum-core`：`ModelId` / `ModelDescriptor` / `InvokeRequest` / `InvokeResponse` / `Usage` / `CallId` /
  `ModelStream` / `ProviderHealth`；`ToolId` / `ToolDescriptor` / `ToolResult`；`ProviderError`。
- `continuum-persist`：`Db` / `builtin_migrations` / `Migration` / `Tx` / `Value` / `PersistError`（**dev 用**）。

**不依赖 B、D、F 的任何产物**（接缝图 §三 硬依赖表）。

**交给 F（硬前置，F 的编译前提）：** `ProviderRegistry`、`invoke_tool`、`AuthorizedToolInvocation<'_>`、
`ToolCallError`、`RegistryError`，以及 **`ToolInvocation` 的删除**。
**F 只调 `invoke_tool`，不构造也不命名请求类型**（协调者接缝裁决第 5 条）。

**交给控制器（本计划不动那两个文件）：** 共享面 §二 §316 段的类型清单（删 `ToolInvocation`）、
`docs/02-工程.md` §10.3 的接口清单（标注请求面已变）——设计 §7.5 落地清单第 4、5 处。

**交给子项目 G（尚不存在）：** 模型侧调用面 `model_for` / `model_providers` / `list_models` / `describe_model` /
`invoke` / `stream` / `cancel` / `usage` / `health`（设计 §6）。

**本计划必须自己声明的未决**：设计 §12 第 18 条（P1 的 `Resource` 义务在 C/D/F 三份里无人认领，收件人：控制器）、
§12 第 23 条（凭据要不要也交给适配器——第二个位置，本阶段不做）。两条都落在 `## 遗留`。

---

# 文件结构

```
crates/continuum-provider/
  Cargo.toml                     normal + continuum-capability；dev + continuum-persist / tempfile / trybuild
  src/lib.rs                     再导出注册表、两个错误类型与新请求类型；模块文档补一句「注册表在、实现仍不在」
  src/registry.rs        （新）   ProviderRegistry、RegistryError、ToolCallError、invoke_tool（唯一受门禁入口）
  src/tool.rs                     §316 trait + 新请求类型 AuthorizedToolInvocation<'_>；invoke 的请求参数换掉
  src/model.rs                   不动
  src/connector.rs               不动
  tests/common/mod.rs    （新）   C 侧夹具（FakeModel / FakeTool / OnceStream），由多个测试目标共同 include
  tests/fake_provider.rs         改为 mod common;，FakeTool 的 invoke 换签名
  tests/registry_models.rs （新） 模型侧登记 / 发现 / 重复 / 原子性 / 枚举 / §3.3 代价一
  tests/registry_tools.rs  （新） 工具侧登记 / 只读入口 / 并集 / 适配器失败
  tests/invoke_tool.rs     （新） invoke_tool 的四条路径 + 门禁内正常路径（真起库）
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
- Consumes: 既有的 `continuum_core::model::ModelId`、`continuum_provider::model::ModelProvider`、`continuum_core::ProviderError`
- Produces: `continuum_provider::{ProviderRegistry, RegistryError, ToolCallError}`，
  `ProviderRegistry::{new, register_model, model_for, model_providers}`

> **本 task 不新增任何依赖边**（`HashMap` / `Arc` 来自 `std`）。`Cargo.toml` 与 `ALLOWED` **一字不动**——
> 这一条要写进 task 报告，因为「注册表加进来之后 provider 没有多出任何指向**实现**的边」正是设计 §4.1
> 要断言的关键性质，而它的照片就是**这一步之后 `every_crate_depends_only_on_its_allowed_set` 仍绿**。

- [ ] **Step 1: 写用例**

`tests/registry_models.rs`（夹具从 `tests/common/mod.rs` 取，见 Step 3）：

- `a_registered_model_is_found_by_id`：登记 `fake-1 → FakeModel`，`model_for("fake-1")` 给出的
  `Arc` 与登记时那一个是**同一个**（`Arc::ptr_eq`，不靠行为推断）。
- `an_unregistered_model_id_is_reported_as_not_found`：`model_for("nope")` →
  `matches!(_, Err(RegistryError::NotFound { .. }))`——**断言是哪一种 `Err`**（纪律 3）。
- `registering_the_same_id_twice_is_rejected_as_duplicate`：同 id 再登记 → `RegistryError::Duplicate`，
  且**原条目不变**（第一次登记的 `Arc` 仍在，`Arc::ptr_eq` 钉住）。
- `registering_a_group_of_ids_is_all_or_nothing`：把 `[a, b]` 登记给一个适配器，其中 `b` 已占用 →
  返回 `Duplicate`，**且 `a` 也没有进去**（`model_for(a)` → `NotFound`）。
- `model_providers_returns_every_registered_adapter_in_registration_order`：登记两个适配器 →
  `model_providers()` 长度 2，顺序与登记一致（逐位 `Arc::ptr_eq`）。**枚举式断言，逐项各一条断言**。
- `a_registered_id_may_be_absent_from_the_adapters_own_list_models`：**设计 §3.3 代价一与 §11 的照片**——
  把 `FakeModel` 登记到它 `list_models()` **不含**的 id（`"m"`）：`model_for("m")` **命中**，
  而对同一 `Arc` 调 `describe_model("m")` 得 `Err(ProviderError::UnknownModel(_))`。
  这条用例钉的是「登记是路由的权威、适配器的 `list_*` 是描述的权威」这两件事**可以不一致**。

- [ ] **Step 2: 运行，确认失败**

```bash
timeout 300 cargo test -p continuum-provider --test registry_models
```

预期：`E0433`（`continuum_provider::ProviderRegistry` 不存在）。

- [ ] **Step 3: 把夹具提到 `tests/common/mod.rs`**

`FakeModel` 与 `OnceStream` 从 `tests/fake_provider.rs` **原样搬进** `tests/common/mod.rs`（`pub`），
`FakeConnector` 留在 `fake_provider.rs` 原地（它只被那一个目标用）。两个目标各写 `mod common;`。

**该模块开头要有 `#![allow(dead_code)]`，并写明理由**：同一个 `common` 被多个测试目标 include，
**没有任何一个目标用得到全部夹具**，而本仓要求 0 warning。这条注释不是装饰——去掉它会得到
`dead_code` 警告，而「0 warning」是硬约束。

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
    /// 同一个 id 被登记第二次（模型侧与工具侧共用）。与
    /// `crates/continuum-operator/src/registry.rs:20-29` 的 `OperatorError::Duplicate` 同形。
    ///
    /// id 以文本承载：两个登记表各用各的 id 类型（`ModelId` / `ToolId`），本变体为两者共用，
    /// 收窄到某一种就得为另一种再造一个变体。**是哪一张表**由调用点可知（`register_model` /
    /// `register_tool`），故不另立字段。
    #[error("id {id} 已登记")]
    Duplicate { id: String },
}

/// 受门禁的 `invoke_tool` 与工具侧只读入口的失败（设计 §3.5）。
///
/// **为什么需要第二个错误类型**：`invoke_tool` 的失败有两个不同来源——「这个 id 没有适配器」
/// （路由，配置缺陷）与「适配器调用失败」（可能瞬时）；一个 `Result` 只能带一个错误类型，
/// 把两者压进 `ProviderError` 就是把「配置错了」报成「provider 挂了」——而那件事在本仓
/// 已经发生过一次（`crates/continuum-provider/tests/fake_provider.rs:106` 把「无此工具」
/// 报成 `Unavailable`，设计 §5.2 记的正是这一处）。
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
/// 模型侧 `model_for` 直接交出裸 `Arc<dyn ModelProvider>`，另开 `model_providers()` 供枚举；
/// 工具侧**只开只读入口与唯一的调用入口 `invoke_tool`**（Task 4），**不交出适配器**。
#[derive(Default)]
pub struct ProviderRegistry { /* 两张 HashMap + 两份登记顺序的适配器列表，字段私有 */ }

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
    pub fn model_for(&self, id: &ModelId) -> Result<Arc<dyn ModelProvider>, RegistryError>;

    /// 全部模型适配器，**按登记顺序**，每次登记调用贡献一项（设计 §3.1「另各持一份登记顺序的
    /// 适配器列表（供枚举）」）。返回 `Vec<Arc<…>>` 的克隆而非切片：调用方（子项目 G）
    /// 要在 `await` 期间持有它们。
    pub fn model_providers(&self) -> Vec<Arc<dyn ModelProvider>>;
}
```

**`model_providers` 的枚举语义**（每次登记调用一项、同一 `Arc` 登记两次即出现两次）与
**`register_model` 的原子性**都是**设计未给判据**的两点，本计划取上面的读数并附照片，见 `## 遗留`。

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

> 本 task 仍**不新增依赖边**。`invoke_tool` 与 `AuthorizedTool` 在 Task 3 / Task 4。
> **不提供 `tool_for`、也不提供 `tool_providers()`**——它们的缺席是 Task 6 的编译失败样例钉的
> （本 task 只把它写进模块文档与 `lib.rs` 的注释，**不写「已钉住」**）。

- [ ] **Step 1: 写用例**

`tests/registry_tools.rs`：

- `a_registered_tool_is_described_by_id`：登记 `echo → FakeTool`，`describe_tool("echo")` 给出的
  `ToolDescriptor.id` 与 `list_tools()` 里那条一致。
- `an_unregistered_tool_id_is_reported_as_unregistered`：`describe_tool("nope")` →
  `matches!(_, Err(ToolCallError::Unregistered { .. }))`——**不是** `ToolCallError::Provider`（纪律 3）。
- `list_tools_is_the_union_of_every_registered_adapter`：登记两个各出一个工具的适配器 →
  `list_tools()` 长度 2，**顺序 = 登记顺序**，且**逐个**断言各自的 `id`（枚举式断言逐项有照片）。
- `list_tools_keeps_the_first_entry_when_two_adapters_declare_the_same_id`：两个适配器都声明 `echo`
  （描述不同）→ `list_tools()` 只有**一条** `echo`，且是**先登记的那个**给出的描述（并集语义）。
- `list_tools_reports_an_adapter_failure_as_provider`：把 `FakeTool` 换成一个 `list_tools` 返
  `Err(ProviderError::Transport(..))` 的适配器 → `matches!(_, Err(ToolCallError::Provider(_)))`，
  且**不是** `Unregistered`。**同一个适配器的失败不许被跳过**（跳过会让一个坏适配器静默消失）。
- `describe_tool_routes_by_registration_not_by_the_adapters_list_tools`：**设计 §3.1「只读入口的
  权威是已登记的适配器，不是登记表本身」的照片**——把 `FakeTool` 登记到它 `list_tools()` **不含**的 id
  （`"m"`）：`describe_tool("m")` **路由到适配器**，于是得到适配器自己的
  `Err(ProviderError::Unavailable("m"))`，包在 `ToolCallError::Provider` 里。
  **这条用例证明路由不查 `list_tools()`**——反序实现（先查 `list_tools` 再路由）在它上面会红。

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
    pub async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ToolCallError>;

    /// 先按**登记**路由到适配器，再转出它的描述；id 未登记 → [`ToolCallError::Unregistered`]。
    ///
    /// **它不看 `list_tools()`**：登记是路由的权威、适配器的 `list_*` 是描述的权威，
    /// 两者可以不一致（设计 §3.1 末段）。
    pub async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ToolCallError>;
}
```

**只读入口的返回错误类型**：设计 §3.5 说 `ToolCallError` 是「受门禁的 `invoke_tool` 用」，
而 §3.1 说「未命中：……工具侧返回 `ToolCallError::Unregistered`」。本计划取后者（两个只读入口也返
`ToolCallError`），因为 `Unregistered` 的语义（「这个 id 没有适配器」）在只读路径上一字不改地成立，
另造一个错误类型反而会造出两个描述同一件事的类型。**这是设计两处相抵之处**，见 `## 遗留` 与报告。

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
- Modify: `crates/continuum-provider/Cargo.toml`（normal + `continuum-capability`；dev + `continuum-persist`、`tempfile`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**`ALLOWED` 的 provider 条目，唯一一处**）
- Modify: `crates/continuum-provider/tests/common/mod.rs`（`FakeTool::invoke` 换参）
- Create: `crates/continuum-provider/tests/invocation.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProviderRegistry`；既有的 `continuum_capability::AuthorizedTool`、`continuum_core::ProviderError`
- Produces: `continuum_provider::{AuthorizedToolInvocation, ToolProvider}`（trait 的 `invoke` 换参）

> **这是本计划对**已冻结接口**的唯一一次显式改动**（设计 §7.5、裁决 §一第 2 条）：§316 的四项方法名不变，
> 变的是 `invoke` 的请求参数。**删 `ToolInvocation` 与它同批做**（裁决 §五），不要拆成两次提交。
>
> **完成判据是 `cargo test --workspace` 通过**，不是「改完定义就收工」——删除的引用者全在编译期暴露，
> 全仓只有三处（五个行号）：`crates/continuum-core/src/tool.rs:26`（定义）、
> `crates/continuum-provider/src/tool.rs:5,12`（trait 的导入与签名）、
> `crates/continuum-provider/tests/fake_provider.rs:7,109`（夹具）——已实测
> `grep -rn ToolInvocation crates/` 只有这五个行号。**改完再 grep 一次**，确认零命中。

- [ ] **Step 1: 写用例**

`tests/invocation.rs`。**夹具**照 `crates/continuum-capability/tests/authorize.rs` 的 `db()` 同形
（`tempfile` + `Db::open_with` + `builtin_migrations()` + `p3_capability_migrations()` + `migrate()`），
再 `save_tool` 登记一条工具、`mint` 铸一枚能力、`authorize` 取 `AuthorizedTool`。
**每个测试目标各写自己的一份 `db()`**（本仓既有做法：`continuum-capability` 的 `tests/authorize.rs`
与 `tests/persist.rs` 各有一份），**不做跨目标的共享夹具**。

- `the_authorization_is_the_one_that_was_passed_in`：`AuthorizedToolInvocation::new(&auth, json!({}))`
  的 `authorization()` 给出的 `tool_id()` 与 `auth.tool_id()` 相同。
- `the_input_is_carried_through`：`input()` 给出构造时那份 `Value`（手工写的字面量比，**钉格式**）。
- `the_request_carries_no_second_tool_id`：**这一条不写成运行用例**——「不另收 `ToolId`」是关于**签名**的
  命题，只有编译能钉。它的照片是 Task 6 的样例 3（裸 `ToolId` 传不进 `invoke_tool`），
  在本文件的模块文档里写明这条去向即可（与设计 §5.3 对 `cancel` 的处置同法：结构事实的照片是类型签名本身）。
- `the_old_request_shape_is_gone`：**删除的判据是编译**，不是运行用例——本 task 的 Step 2 与 Step 5
  的 `cargo test --workspace` 就是这条的照片（四处引用者全在编译期暴露）。**不写一条假装在跑期验证它的用例。**

- [ ] **Step 2: 运行，确认失败**

```bash
timeout 300 cargo test -p continuum-provider --test invocation
timeout 900 cargo build --workspace --all-targets
```

第二条此时**应当报错**（`AuthorizedToolInvocation` 不存在），**这就是删除的判据的样子**。

- [ ] **Step 3: 实现**

```rust
// crates/continuum-provider/src/tool.rs

/// §316 `invoke` 的请求侧：**只装得下「已授权」**（设计 §7.5，裁决 §一第 2 条）。
///
/// 依据：`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` §一第 2 条——
/// 「改的是请求侧，不是响应侧」（响应在调用完成之后才存在，往那里加位置解决不了问题）。
///
/// 两条性质：
/// 1. **没有 [`AuthorizedTool`] 就构造不出它**——唯一的公开构造入口 [`Self::new`] 收 `&AuthorizedTool`，
///    字段私有（照片：`tests/compile_fail/` 的两份样例，Task 6）。
/// 2. **工具 id 只有一个来源**：授权证明的 `AuthorizedTool::tool_id()`。本类型**不另收 `ToolId`**，
///    故「出示的 id 与被授权的 id 不是一个」这一种可能**不存在**。
///
/// **为什么装的是 `&AuthorizedTool`，不是 `Credential`**：这里承载的是**强制点 (1) 的证明**，
/// 不是密钥材料。凭据是 `continuum-secrets` 的词汇、走连接器路径（设计 §7.5 末段；若将来确需把
/// 凭据交给模型/工具适配器，那是一个**第二个位置**，见 `## 遗留`）。
pub struct AuthorizedToolInvocation<'a> { /* authorization: &'a AuthorizedTool, input: Value —— 两者皆私有 */ }

impl<'a> AuthorizedToolInvocation<'a> {
    /// **唯一的公开构造入口。** 收 `&AuthorizedTool` 而非 `ToolId`——这正是「没有授权就构造不出
    /// 这次调用」的落点。
    ///
    /// **它是公开的，且必须公开**：设计 §7.5 的收紧论证是「此前装配者持有裸
    /// `Arc<dyn ToolProvider>` 就能调 `invoke`，**现在连裸适配器也要求一枚 `AuthorizedTool`**」
    /// ——若构造入口是 `pub(crate)`，装配者根本调不了 `invoke`，那句话就成了另一回事。
    /// 公开的代价写在 `## 遗留`（持有一枚真 `AuthorizedTool` 的代码仍可自行构造）。
    pub fn new(authorization: &'a AuthorizedTool, input: Value) -> Self;

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

`Cargo.toml`：normal 加 `continuum-capability = { path = "../continuum-capability" }`（理由注释：
`invoke` 的请求参数要能命名 `AuthorizedTool`，层内边，设计 §4.1 订正段）；
dev 加 `continuum-persist` 与 `tempfile`（理由注释：`tests/invocation.rs` 等要构造 `Tx` 去调 `authorize`）。

`dependency_direction.rs:29` 改为本节 Global Constraints 里的三元素条目，并更新其上方注释
（**照 P3A 各 task 更新注释的写法**，把「Task 3 起加上 capability / dev 加 persist」写清楚）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
timeout 900 cargo build --workspace --all-targets
git add crates/continuum-core crates/continuum-provider crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(provider): §316 请求面换成 AuthorizedToolInvocation，删 ToolInvocation"
```

**预期**：全绿、0 warning。若 `cargo tree` 让 `every_crate_depends_only_on_its_allowed_set` 红，
**先核 `ALLOWED` 与 `Cargo.toml` 是否精确一致**（本仓最常见的一处失手）。

---

### Task 4: `invoke_tool`——唯一受门禁的调用入口

**Files:**
- Modify: `crates/continuum-provider/src/registry.rs`、`src/lib.rs`
- Create: `crates/continuum-provider/tests/invoke_tool.rs`

**Interfaces:**
- Consumes: Task 1 的 `ProviderRegistry`、Task 3 的 `AuthorizedToolInvocation`、
  既有的 `continuum_capability::{AuthorizedTool, authorize, mint, Tool, ToolProfile, Trust, save_tool, p3_capability_migrations}`
- Produces: `ProviderRegistry::invoke_tool`

> **这是本计划交给 F 的硬交付之一**（另一个是 Task 3 的请求类型）。
> **工具侧唯一的调用入口**：注册表不交出 `Arc<dyn ToolProvider>`，故任何持有注册表的代码都只能经它调到工具
> （设计 §7.1；F 已据此删掉 `ToolCaller` / `invoke_authorized` / `select_tool_caller`）。

- [ ] **Step 1: 写用例**

`tests/invoke_tool.rs`（夹具同 Task 3 的 `db()`；适配器用 `common` 里的 `FakeTool`）：

- `a_registered_tool_is_invoked_and_its_result_returned`：**门禁内正常路径**（设计 §11）——
  起库 + capability 迁移 + `save_tool` + `authorize` 取 `AuthorizedTool` + `register_tool` →
  `invoke_tool(&auth, json!({"x": 1}))` 得 `Ok(ToolResult { output: {"x":1}, is_error: false })`。
  **这一条要真跑通**，不是「用既有夹具即可、不新建夹具」那个量级（设计 §9 已把这条的代价说准）。
- `an_unregistered_tool_is_not_called_at_all`：注册表里没有这个 id 的适配器 →
  `matches!(_, Err(ToolCallError::Unregistered { .. }))`，**且不是 `Provider(..)`**。
  **这条只钉路由本身**（不起库、不涉 `authorize`）；「`tool` 表有行、注册表无适配器」那一向
  是**另一个命题**，它在 Task 5，**不在这里重复**。
- `the_adapter_failure_is_reported_as_provider`：适配器 `invoke` 返 `Err(ProviderError::Transport(..))` →
  `matches!(_, Err(ToolCallError::Provider(_)))`，**且不是 `Unregistered`**。
- `the_adapter_is_handed_the_authorization_that_was_passed_in`：适配器侧读
  `call.authorization().tool_id()`，与 `auth.tool_id()` 相同——**证明路由用的 id 与被下传的授权是同一枚**。
  这条要一个「把收到的 id 记下来」的适配器（`common` 里加一个记录型的 fixture），
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
    /// 只传 `&AuthorizedTool` 与 `input`，不构造也不命名那个类型（协调者接缝裁决第 5 条）。
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

- [ ] **Step 4: 运行全部测试并提交**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-provider
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

`tests/two_registries.rs`（夹具同 Task 3 的 `db()`）：

- `a_tool_in_the_table_without_an_adapter_passes_authorize_then_fails_to_route`（**第一向，fail-open 的那侧**）：
  `save_tool` 登记了、注册表**没**登记 → `authorize` **返回 `Ok`**（拿到 `AuthorizedTool`），
  随后 `invoke_tool` 返回 `Err(ToolCallError::Unregistered { .. })`。
  **两半都要断言**：只断后一半会让「authorize 也能挡住它」这条假说法活下来。
- `an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call`（**第二向**）：
  注册表**登记了**适配器、`tool` 表**没** `save_tool` → `authorize` 返回
  `Err(CapabilityError::UnknownTool { .. })`，**在 `invoke_tool` 之前**。
  断言里写明：这条路径上 `ProviderRegistry` 一次都没被碰过。

两条用例是**同一条事实的两向**（设计 §3.3 代价三的表），故两条必须**同时存在**。

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
4. `authorized_tool_invocation_needs_an_authorization.rs`：
   `AuthorizedToolInvocation::new(serde_json::json!({}))` —— **唯一的公开构造入口**要一枚
   `&AuthorizedTool`，没有它就构造不出请求（设计 §7.5 的第一条性质）。
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
`tests/fake_provider.rs:162` 的 `let t = FakeTool;` 改为 `FakeTool::echo()`。

**为什么不写成三个结构体**：设计 §11 的这一行写的是「用 `FakeTool`」，而一个单元结构体产不出两条失败通道；
三态化是让那一行字面成立的最小改动。**这是本计划对夹具形状的读数，不是设计给的判据**，见 `## 遗留`。

- [ ] **Step 2: 写用例**

`tests/contract.rs`（夹具同 Task 3 的 `db()`）：

- `a_tool_level_failure_is_ok_with_is_error_true`：适配器返工具级失败 →
  `invoke_tool` 得 `Ok(ToolResult { is_error: true, .. })`，**不是 `Err`**。
- `a_provider_level_failure_is_err`：适配器返 provider 级失败 →
  `invoke_tool` 得 `Err(ToolCallError::Provider(_))`，**不是 `Ok(is_error: true)`**。

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
> `src/lib.rs:1` 那句「不含实现」只是文档，今天没有任何用例钉它。**结果：中立 crate 里躺着一份实现，
> 而一片全绿。这条路径今天没有别的东西挡。**

- [ ] **Step 1: 写用例**

`tests/neutrality.rs`：

- `the_guard_sees_the_source_tree`（**守卫自身的正控制，先钉机制再看结论**）：递归读
  `crates/continuum-provider/src/` 下全部 `.rs`，断言**至少读到 4 个文件**
  （`lib.rs` / `model.rs` / `tool.rs` / `connector.rs` / `registry.rs`）**且拼起来的文本里能找到
  `trait ModelProvider`**。**没有这一条，一个「什么都没读到」的守卫会永远绿**——这正是
  「守卫须两侧都钉」里缺的那一侧。
- `the_neutral_crate_contains_no_implementation`：同一份文本里**不出现**下面三个字面拼法：
  `impl ModelProvider for`、`impl ToolProvider for`、`impl Connector for`。
  断言信息里要**列出命中的文件名与行号**，否则红的时候读不出是哪一处。

文件头**必须逐条写明这条守卫的逃逸面**（设计 §4.1 末段已给三条，本计划另补第四条，见 `## 遗留`）：
全限定 trait 路径（`impl crate::model::ModelProvider for X`）、`use … as` 别名、`include!`，
以及**空白变体**（`impl  ModelProvider for`，两个空格）。
故它是**一个下界，不是封闭判定**；上界要解析 trait 路径（需要 `syn` 之类的新 dev 依赖），
**本阶段不做**（设计 §12 第 21 条）。

- [ ] **Step 2: 运行，确认通过（这是本 task 的正常态）**

```bash
timeout 300 cargo test -p continuum-provider --test neutrality
```

**本 task 的红要这样做**（否则「守卫恒绿」无法与「守卫有效」区分）：
临时在 `src/` 下加一个 `struct Probe;` 与 `impl ModelProvider for Probe { … }`（**不提交**），
跑 `cargo test -p continuum-provider --test neutrality`，**确认它红且报出文件名与行号**，再删掉。
**这次红要留日志路径**（纪律 1：每次变异用独立日志路径）——它是这条守卫唯一的物证。

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
| 工具侧：授权证明进不到该进的地方 | Task 6 样例 3、4、5 |
| 工具侧：门禁内的正常路径 | Task 4 `a_registered_tool_is_invoked_and_its_result_returned` |
| 工具侧：`is_error` 与 `Err` 的分流（**约定**，非规范） | Task 7 两条（**钉夹具，不钉真实适配器**） |
| 两个登记点不一致，向一 | Task 5 `…passes_authorize_then_fails_to_route` |
| 两个登记点不一致，向二 | Task 5 `an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call` |
| 中立性（模块面，§4.2 形态 4） | Task 8（**下界**，逃逸面写在文件头） |
| 模型侧未登记 id | Task 1 `an_unregistered_model_id_is_reported_as_not_found` |
| 工具侧未登记 id | Task 2 `an_unregistered_tool_id_is_reported_as_unregistered` |
| 重复登记同一 id | Task 1 `registering_the_same_id_twice_is_rejected_as_duplicate` |
| 模型侧枚举 | Task 1 `model_providers_returns_every_registered_adapter_in_registration_order` |
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
ProviderError →         §5.1 那张表是**文档不是代码**（无消费方）。另注：FailureClass 有 Resource，
  FailureClass          而 ProviderError 没有表示「限流 / 配额耗尽」的变体，该类丢掉了。
                       收件人：F（工具路径）+ 子项目 G（模型路径）。
P1 的 Resource 义务     要求「P3 的 Router 必须为 RESOURCE 显式给出 max_attempts >= 2 与退避参数」，
                       而 D 说重试不属本层、F 的工具路径不调 decide_retry、C 把它折进上一条推走。
                       **须点一个所有方**（设计 §12 第 18 条）。收件人：控制器。
实现被写进中立 crate     §4.2 形态 4 的那条路径**没有行为照片**；守卫是 lib.rs:1 的定位声明
  内部                   + Task 8 的模块面断言 + 评审。Task 8 之后那三个字面拼法会被红掉，
                       全限定路径 / 别名 / include! / 空白变体仍全绿。
save_tool 登记期不变量   C 不接（设计 §7.4 接缝二、§12 第 15 条）：它约束的是 tool 表的**写入**，
                       属 continuum-capability。协调者已裁定**所有者是 F**（接缝图 §六第 1 条）。
凭据要不要也交给         设计 §12 第 23 条：本阶段不做；若做，会引入 continuum-provider
  适配器               → continuum-secrets 的边，那是一个**第二个位置**。收件人：控制器（若提出）。

（三）实现期新发现：设计未给判据，本计划取了读数（**须复核**）

只读入口的返回错误类型   设计 §3.5 说 ToolCallError 是「受门禁的 invoke_tool 用」，§3.1 却说
                       「未命中：……工具侧返回 ToolCallError::Unregistered」——**两处相抵**。
                       本计划取 §3.1：list_tools / describe_tool 也返 ToolCallError。
                       若不取这个读数，就得为只读路径再造一个描述同一件事的错误类型。
register_* 的原子性      设计未写「一组 id 中有一个撞车时是否部分登记」。本计划取**先全查后全插**
                       （一个都不登记），照片在 Task 1。设计若判部分登记可接受，那条用例要改写。
model_providers 的        设计只说「另各持一份登记顺序的适配器列表（供枚举）」，未写同一适配器
  枚举语义              登记给多组 id 时出现几次。本计划取「每次登记调用一项」。
list_tools 的并集语义    设计只说「并集」，未写同一 ToolId 由多个适配器声明时保留哪一条。
                       本计划取「首次出现者胜」。与上一条同源：都属「注册表枚举的粒度」。
AuthorizedToolInvocation  **设计 §7.5 说它是「唯一的公开构造入口」**，其收紧论证（「连裸适配器也要求
  构造入口是公开的     一枚 AuthorizedTool」）也要求公开；而 F 的设计 §6.3 与协调者接缝裁决第 5 条
                       的措辞（「请求只能由 invoke_tool 构造」）**隐含 pub(crate)**。**两说相抵**。
                       本计划按 §7.5 取**公开**。残留据实记：**持有一枚真 AuthorizedTool 的代码
                       仍可自行构造请求、直接调裸适配器的 invoke**——这与设计 §7.5 自己的说法
                       （保证升到 trait 级，不是「只有注册表能调」）一致。**须设计侧复核。**
FakeTool 三态化          设计 §11 写「用 FakeTool」，而一个单元结构体产不出两条失败通道。
                       本计划把它改成带失败通道的三态夹具（Task 7）。**是对夹具形状的读数，不是判据。**
模块面守卫的第四种逃逸    设计 §4.1 末段列了三条逃逸（全限定路径 / 别名 / include!），
                       **另有空白变体**（`impl  ModelProvider for`，两个空格）照样全绿——
                       文本匹配对空白敏感。本计划**未**加空白归一化（那会超出设计的判据范围），
                       据实记此，交设计侧决定要不要扩。
```

**另有一处本计划**没有**做、且必须点明的**：设计 §7.5 落地清单的第 4、5 处
（共享面 §二 §316 段的类型清单删 `ToolInvocation`、`docs/02-工程.md` §10.3 的接口清单标注请求面已变）
**由控制器改，本计划不动那两个文件**。实现完成时若那两处仍未改，**不得**把它们标为已覆盖。
