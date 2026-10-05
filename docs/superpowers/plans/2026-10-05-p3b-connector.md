# P3 子项目 B（连接器 / `continuum-connector`）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 落地 §124 / §125——建 `continuum-connector`（边界层）：连接器的操作集与它的**声明完整性**、操作 → 授权的映射、连接器侧的强制点 (2)、以及凭据交付路径；并把 `continuum-secrets` 从「零引用的完整实现」接成有一条真依赖边的组件。

**Architecture:** 新 crate `continuum-connector` 承载四件东西：`ConnectorImpl`（连接器作者实现的扩展 trait，决定 B-4）、`OpBinding`（操作 → `CapabilityKind` 的绑定，唯一产生点是注册入口）、`ConnectorRegistry`（注册期四条核对的唯一产生点 ＋ 唯一调用入口）、以及**逐次调用的适配器**（凭据材料的交付路径，不动 §124 的 `invoke` 签名）。入口在调用实现之前做**四步核对**、按出示的那枚能力**逐次签发**凭据、取料、再经适配器转发。另在 `continuum-capability` 补一枚 `for_effect` 的逆（`CapabilityKind::effect`，本计划对该 crate 的**唯一**新增面），并在 `continuum-runtime` 的装配处装上 `SecretsRuntime`。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-core`（`ConnectorId` / `ConnectorOp` / `ConnectorDescriptor` / `ProviderError`）、`continuum-capability`（`Capability` / `CapabilityKind` / `AuthorizedEffect` 及新加的 `effect` 逆）、`continuum-secrets`（`SecretsRuntime` / `issue` / `material` / `SecretMaterial`）、`continuum-provider`（§124 的 `Connector` trait）、`continuum-effect`（`EffectType`）；`serde_json`、`async-trait`、`thiserror`；dev：`tokio`、`trybuild`、`continuum-persist`、`tempfile`。

**设计依据：** `docs/superpowers/specs/2026-10-05-p3b-connector-design.md`（**唯一事实来源**）。

---

## 跨计划前置

### 本计划依赖谁已交付什么

| 依赖 | 现状 | 本计划怎么用 |
|---|---|---|
| `continuum-capability` 的 `Capability` / `CapabilityKind` / `for_effect` / `mint` / `AuthorizedEffect` | 已在 `main`（P3A 交付） | 绑定的类型、出示值的类型、第 3 步的判据来源 |
| `continuum-secrets` 的 `SecretsRuntime` / `issue` / `material` / `SecretMaterial` / `SecretsError` | 已在 `main`，**全仓零引用、连依赖边都没有**（`docs/superpowers/p3a-followups.md` 第一节） | 强制点 (3) 的接线对象；本计划是它的第一个真消费方 |
| `continuum-core` 的 `ConnectorId` / `ConnectorOp` / `ConnectorDescriptor` / `ProviderError` | 已在 `main`（P0/P1） | 操作集与描述符；**不另建第二套 id 或描述类型** |
| `continuum-provider` 的 §124 `Connector` trait（`descriptor` / `invoke`） | 已在 `main`，**本计划一字不动它的形状**（决定 B-4） | 适配器实现它 |
| `continuum-effect` 的 `EffectType` | 已在 `main` | 第 3 步错误变体里的那个字段；逆对应的返回值 |

**不依赖 C、D、F 的任何产物**：B 走 §124，不经 §316（裁决 §一.2 的「B 不受影响」），而 §316 的请求类型由 C 定、F 传。执行序 **B → D → C → F** 中 B 在最前，故本计划**不得**引用 `AuthorizedToolInvocation` 或任何 C 的产物。

### 本计划把什么交给谁

| 交付物 | 收件人 | 说明 |
|---|---|---|
| `CapabilityKind::effect`（`for_effect` 的逆，`Option<EffectType>`） | `continuum-capability`（由本计划 Task 1 落地） | 设计 §11 第 11 条：B 对该 crate 的**唯一**新增面。**唯一消费方是 B 的入口** |
| `continuum-connector` crate | 驱动 | **本轮无生产消费者**：`runtime → connector` 这条边**不登记**（见「遗留」第 1 条，裁决 §六.3） |
| `continuum-runtime → continuum-secrets` 的边与 `main.rs` 的装配 | 本计划 Task 7 | 裁决 §六.2 把这条边的所有者写死为 B |
| 三条规范级缺口的现场记录 | 规范 | 「一条操作该绑哪一枚 kind」无判据、非效应臂的强制点归谁未规定、非效应臂那枚能力没有具名产生方（见「遗留」第 3、4 条） |

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。** 本计划新增的边（**每条由用它的那个 task 自己登记**，照 P3A 的 Tasks 3/4/5 做法）：
  `continuum-connector → continuum-core, continuum-capability, continuum-secrets, continuum-provider, continuum-effect`；
  `continuum-runtime → continuum-secrets`。
  `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 与各 `Cargo.toml` 必须**精确一致**（断言是逐对 `assert_eq!`，且覆盖 dev 边——那把 `cargo tree` 带 `--edges all`）。
- **本计划改 `continuum-capability` 的 `capability.rs` 与 `tests/vocabulary.rs` 两处，不改它的依赖边**（`EffectType` 已是它的依赖，`for_effect` 与 `effect` 同址）。接缝裁决（`docs/superpowers/p3bcdf-followups.md` **§七第 8 条**）记的「同一个 crate 被两个项目改」在本计划里就是这一处（F 改的是 `persist.rs`，与本计划不同文件），**该条判为已知非缺陷、不做任何事**——两处不同文件、无编译冲突，且都不新增该 crate 的依赖边。
  **本条判据的出处是 §七（权威转录），不是 `.superpowers/sdd-p3bcdf/impl-seam-map.md`**——后者是 gitignore 的 scratch，随时会丢（§七 的末段说明了为什么一律引 §七）。
- **不建表、不取迁移号**：B 不落库、不写审计（设计 §6.2、§7.3）。故本计划**没有**迁移编号核对这一步——不是漏了，是设计明写不建（§1 的表、§7.3 末段）。
- **§124 的 `Connector` trait 一字不改**（`crates/continuum-provider/src/connector.rs`）；**`ConnectorDescriptor` 的字段不改**；`continuum-core` 的任何类型不改。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿，**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。新增或变更 crate 依赖时 `Cargo.lock` 随之变化，**须一并提交锁文件**（各 task 的显式路径清单只列了源码与清单）。
- **本计划不发明判据**：设计没给判据的两处（非效应臂的强制点归属、「一条操作该绑哪一枚 `CapabilityKind`」）**一条都不补**，写进「遗留」并标「规范未给判据」。设计说「不建」的（真实连接器实现、§125 五个操作相配的 kind 臂、B 自己的审计行、工具调用路径）**一个 task 都不写**。

---

## 三条已付过代价的纪律

> 以下三条照抄自 `docs/superpowers/plans/2026-10-04-p3a-capability.md`（P3A 的实测教训），**逐条核对过对本计划的适用性**，核对结论紧跟在每条之后。

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」跑全量是加分。**变异的三条失效形态都要防**：(a) 锚点不唯一 → 变异没落到实现体却报 GREEN；(b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**；(c) **变异导致编译失败**——那不是「变红」。每次变异用**独立日志路径**，读前确认是本轮写的。**判据本身也会假阳性**：cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，故判「编译失败」要用 `could not compile` 或 `error[E….`。

   **对本计划的适用性核对（逐条，不抽代表）**：(a) 本计划的变异锚点**多数不唯一**，最典型的是十二臂的 `CapabilityKind::effect`（Task 1）与四步核对里的三处「比对相等」——**每次变异后必须读源码确认改到了目标那一臂**，只改一处就报绿是最可能的假绿形态；(b) **等价变异体的具体候选**：`effect()` 里把 `Some(x)` 换成 `Some(y)` 在部分臂上是等价的（若无用例区分），把注册期两条核对的**先后次序**互换也是等价（每条用例只触发一条核对，见 Task 3 的说明）——凡此都**换真变异体**（如把 `==` 换成 `!=`），不补用例；(c) 本计划 Task 6 的 trybuild 样例**以编译失败为判据**，故与 (c) 直接冲突：那里判红必须看 `could not compile` 与 `.stderr` 的比对结果，不能看「test failed」。
2. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**（标明了状态就不是漏枚举）。**枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条分支各要照片；手写分支能各自漂移，故要逐项钉。**改完一处枚举，通读整段。** 判据：用例里的值字面量是**手工写的**还是**被测函数返回的**——前者钉格式，后者钉路径。

   **对本计划的适用性核对**：本计划里带绝对措辞的注释集中在四处——`OpBinding` 的「两个字段」、`register` 的「四条核对」、入口的「四步」、`CapabilityKind::effect` 的「十二臂穷尽」。**四处都必须逐项有照片，不抽代表**：十二条 kind 各一张（Task 1）、四条注册期核对各一对（正反两侧，Task 2/3）、四步入口核对各一张（Task 4/5）。Task 3 那条「绑错了照样注册成功」是**刻意照出来的反例**，它的注释必须写明「规范没有『配得对不对』的判据，故这条照片断言的是 `Ok`」——否则它就是一句会被读反的绝对措辞。
3. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 Err」。

   **对本计划的适用性核对**：本计划的失败面有 **10 种** `ConnectorError` 变体（设计 §6.1 的表恰好十行：注册期 4 ＝ `UnboundOperation` / `UndeclaredBoundOperation` / `DuplicateKindBinding` / `OperationServiceMismatch`，调用期 4 ＝ `UnknownConnector` / `UndeclaredOperation` / `EffectAuthorizationRequired` / `AuthorizationMismatch`，加两个 `#[from]` ＝ `Credentials` / `Provider`），每种各要断言**具体变体**（用 `matches!` 或逐字段比对；`ProviderError` 与 `SecretsError` 都派生 `PartialEq`，故内层变体可逐字段比）。**本条不因「外层是 `#[from]`」而放松**：`ConnectorError::Provider(_)` 与 `Credentials(_)` 两条路径必须分别断言**内层是哪一个变体**。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志——`pgrep "cargo|rustc"` 看不见卡在 `D` 状态的测试二进制。受能力门控的用例要给执行/跳过条数，**承重断言不要放在门控之内**。

**临时目录的用法（P3A 实测的坑，本计划每个 task 都会踩）**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个（在 FUSE 挂载上，I/O 曾挂起）。`workspace` 的 overlay 用例会在其下留下**权限位 000 的 `work/` 目录**（那是被测对象，不是泄漏），直接 `rm -rf .tmp` 会报 `Permission denied` 而只删掉一半；收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**（它不在 `.gitignore` 里，`git status` 会显示，但只按显式路径 `git add` 就不会误提交）。

**变异日志是证据，必须活到复审结束**：报告里的变异表会引用每条变异的日志路径，而**实现者不得在交活前删掉 `.tmp/`**——分工是**实现者保留 `.tmp/`，由协调者在复审结束后清理**。同理，报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥（P3A 实测的坑，责任在协调者）**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。**实现者报告完成之前，协调者不得在该工作区里跑 cargo**；协调者的独立重跑**严格排在实现者的终稿之后**。这条不是礼节，是证据有效性的一部分。**精确边界**：危险的是**变异**与任何别的东西并行；**两份跑在同一份已提交字节上的验证并行是无害的**（各自结果都有效），代价只是抢核、变慢，以及负载可能诱发时序敏感用例。故「互斥」管的是**变异窗口**，不是「验证之间」。

**变异口径分层**：变异**条数**按「有多少条**互不相同**的守卫」而非「有多少个分支」定；且按下表分档，**不许把两档混成一句「通过」**：
- **承重守卫 → 全量套件**：「两侧对钉」的守卫（注册期双向覆盖的两侧、入口第 3 与第 4 步的两侧）、**fail-open 的那一侧**（第 3 步那条「有外部效应却没走强制点 (2)」）、失败路径**判别哪一种 `Err`**、以及**跨 crate 才可见**的效果（`ALLOWED` 与实际依赖一致、`CapabilityKind::effect` 的逆）。
- **其余分支 → 受影响 crate 的包级套件**，且报告里必须**标明证据强度较低**，并列出**这一条可能漏掉的跨 crate 观察点**。只写「包级通过」即视为未报证据。

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**（P1 出过 20+ 处事实错误），且已确立「**代码块是示意，正文的措辞才是约束**」。因此本计划中：类型定义与函数签名以**形状**给出，判定与次序以**正文**给出，**不给整段可粘贴实现**；凡与既有 crate 交互的签名（`ConnectorOp` / `ConnectorDescriptor` / `Connector` / `SecretsRuntime` / `Capability` / `AuthorizedEffect` / `ProviderError`），**实现前须先读该 crate 的源码确认**，不符时以源码为准并回报。凡本计划给的行号，**照抄前先对源**。

---

# 文件结构

```
crates/continuum-connector/
  Cargo.toml
  src/lib.rs        导出面与 crate 文档
  src/error.rs      ConnectorError（注册期 4 ＋ 调用期 4 ＋ 两个 #[from]）
  src/binding.rs    OpBinding、ConnectorImpl（连接器作者实现的扩展 trait）
  src/registry.rs   ConnectorRegistry、register（注册期四条核对的唯一产生点）
  src/entry.rs      入口：ConnectorAuthorization、四步核对、凭据签发、调用
  src/adapter.rs    逐次调用的适配器（决定 B-4）
  tests/register.rs 注册期四条核对的两侧 ＋ 「绑错了照样注册成功」
  tests/invoke.rs   入口四步核对的拒绝面与两条臂的成功面
  tests/audit.rs    否定照片：一次成功调用前后 audit_log 行数不变
  tests/type_level.rs      trybuild 驱动
  tests/compile_fail/*.rs  编译失败样例（各配同名 .stderr）

crates/continuum-capability/
  src/capability.rs        新增 CapabilityKind::effect（紧邻 for_effect）
  tests/vocabulary.rs      补逆对应的逐项照片
```

**既有的、本计划要改的文件**

```
crates/continuum-runtime/src/main.rs                     装配 SecretsRuntime（Task 7）
crates/continuum-runtime/src/secrets.rs                  新增：装配函数（Task 7）
crates/continuum-runtime/Cargo.toml                      runtime → secrets 边（Task 7）
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 登记（Task 2/4/5/7）
crates/continuum-runtime/tests/secrets_assembly.rs       新增：装配的二进制照片（Task 7）
Cargo.toml（workspace）                                   members（Task 2）
```

---

### Task 1: `CapabilityKind::effect` —— `for_effect` 的逆

**Files:**
- Modify: `crates/continuum-capability/src/capability.rs`
- Modify: `crates/continuum-capability/tests/vocabulary.rs`

**Interfaces:**
- Consumes: 既有的 `EffectType`（`continuum-effect`，已是本 crate 的依赖）、既有的 `for_effect`
- Produces: `continuum_capability::CapabilityKind::effect(&self) -> Option<EffectType>`

> **本 task 不改 `continuum-capability` 的依赖边**，故 `ALLOWED` 的那一条一个字不动。它是本计划对该 crate 的唯一新增面（设计 §11 第 11 条）。

- [ ] **Step 1: 写用例**

`tests/vocabulary.rs` 加三条，**逐项有照片、不抽代表**：

- `the_inverse_of_for_effect_returns_the_same_effect_for_every_effect_type`：对 `EffectType::ALL` 的**每一个**变体，断言 `CapabilityKind::for_effect(e).effect() == Some(e)`。**六条各断言一次**（用例里的 `EffectType` 值来自 `ALL` 的遍历，钉的是**路径**）。
- `every_kind_outside_the_image_has_no_effect`：逐项断言下面**六枚** kind 的 `effect()` 是 `None`——`Filesystem(FsAction::Read)`、`Filesystem(FsAction::Write)`、`Git(GitAction::Read)`、`Git(GitAction::WorktreeWrite)`、`Git(GitAction::CommitLocal)`、`Github(GithubAction::CreatePr)`。**六条各断言一次**（这里的 kind 字面量是**手工写的**，钉的是「非像的那六枚是哪些」这个事实本身）。设计 §3.3 第 2 条的**初稿**只写了五枚、漏了 `Filesystem(Write)`，**现已逐枚列全**（裁决文件 B1 裁为设计改，见「遗留」第 10 条）；本计划按那六枚写，与设计现文一致。
- `the_two_lists_are_disjoint`：把上面两条的域并起来比对——**像那六枚**由 `EffectType::ALL` 遍历经 `for_effect` 得到（**被测函数返回的值**，钉路径），**非像那六枚**是上一条里**手工写的字面量**（钉「有哪六枚」这个事实）——断言两组**互不相同**（并起来是十二枚互异的 kind）。它钉的是「那份手工清单里没有混进像里的那一枚」。

  **「十二枚就是全部 kind」这一半是编译期保证，不是运行期用例**——**不要**写「由 `as_str` 经 `parse` 反解出全集」那种写法：`CapabilityKind` 上**没有 `ALL`**，`as_str` 又是**实例方法**（`crates/continuum-capability/src/capability.rs:205` 的签名是 `pub fn as_str(&self)`），要拿到十二个串得先有十二个实例，**这是循环**（P3A 的 `tests/persist.rs` 遇到同一个限制，用的是手工清单）。取代它的是 [`CapabilityKind::effect`] 的 `match` **穷尽且无通配臂**：加第 13 枚 kind 时它编译不过，实现者必须补一个臂；**但那个新臂该给 `Some` 还是 `None`，编译器判不了**，只能由人在上面两张清单里补一项。**这一半没有运行期照片，据实标明**（纪律 2「写不出的就明写它为什么没有照片」）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-capability --test vocabulary
```

- [ ] **Step 3: 实现**

```rust
impl CapabilityKind {
    /// [`CapabilityKind::for_effect`] 的逆（设计 §3.2 第 4 条）。
    ///
    /// 对应关系由 `for_effect` 拥有，**逆也必须只有一个产生点**，故它与 `for_effect`
    /// 同址（同一 impl 块、紧邻）。`for_effect` 是单射而**不是满射**，故返回 `Option`。
    ///
    /// 十二个臂**逐个手写、穷尽且无通配臂**：加 kind 时本函数编译不过，逆不会漏分支。
    /// 「该给 `Some` 的恰是像那六枚」由 `tests/vocabulary.rs` 的三条用例钉（前两条逐项、
    /// 第三条钉两份清单互斥）；**编译器看不出把 `Some` 写成 `None` 的臂**。
    /// **而「kind 一共十二枚」这一半是编译期保证，不是运行期用例**——本 crate 没有
    /// `CapabilityKind::ALL`，且 `as_str` 是实例方法，运行期遍历不出全集（见该用例的说明）。
    pub fn effect(&self) -> Option<EffectType> { /* 十二个臂，逐臂写明 */ }
}
```

**它为什么落在这一处而不是连接器里**：对应关系由 `continuum-capability` 拥有（`for_effect` 在那里），逆落在别处就会让「哪一枚 kind 配哪一条效应」有第二个产生点——P2 为同类形状出过 Critical（设计 §3.2 第 4 条）。

- [ ] **Step 4: 变异与全量验证**

对 `effect` 做**三条**变异，各写独立日志路径（前两条改实现、第三条改夹具，第三条在验前两条的证据是否落在靶上）：
- 把某一臂的 `Some(...)` 改成 `Some(另一条效应)`（**锚点不唯一，改后必须读源码确认落在目标臂上**）；预期 `the_inverse_of_for_effect_returns_the_same_effect_for_every_effect_type` 红；
- 把某个非像臂的 `None` 改成 `Some(某效应)`；预期 `every_kind_outside_the_image_has_no_effect` 红。**`the_two_lists_are_disjoint` 这条不红，且这不是它的漏**——它比的是 kind 的集合，不读 `effect()`，域与这条变异不相交（**不要把「它没红」读成夹具没造对**）；
- 把**非像清单**里的某一枚换成像里的一枚（例如把 `Github(GithubAction::CreatePr)` 写成 `Email(EmailAction::Send)`）；预期 `the_two_lists_are_disjoint` 红。**这一条才是那条用例的守卫**（前一条变异打不到它）。**这三条里第二条是承重守卫（「该给 `None` 的臂」那一侧），须走全量套件。**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-capability
git commit -m "feat(capability): for_effect 的逆 CapabilityKind::effect"
```

---

### Task 2: `continuum-connector` 骨架与绑定的双向覆盖

**Files:**
- Create: `crates/continuum-connector/Cargo.toml`
- Create: `crates/continuum-connector/src/{lib.rs,error.rs,binding.rs,registry.rs}`
- Create: `crates/continuum-connector/tests/register.rs`
- Modify: `Cargo.toml`（workspace members）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 新增 `continuum-connector` 条目）

**Interfaces:**
- Consumes: `continuum_core::connector::{ConnectorId, ConnectorOp, ConnectorDescriptor}`、`continuum_capability::CapabilityKind`、`continuum_secrets::SecretMaterial`
- Produces: `continuum_connector::{ConnectorImpl, OpBinding, ConnectorRegistry, ConnectorError}`（本轮只到注册期四条变体中的两条）

- [ ] **Step 1: 建 crate 并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`continuum-core`（描述符与操作）、
`continuum-capability`（`OpBinding` 的 kind 与错误变体的字段）、`continuum-secrets`
（`ConnectorImpl::invoke_with` 的材料入参——**是公开 trait 签名的一部分，故是真依赖**）、
`serde_json`（同一签名的 `Value`）、`async-trait`（该方法的 async）、`thiserror`（`ConnectorError`）。
dev-dep `tokio` / `trybuild` / `continuum-persist` / `tempfile` **由需要它们的 task 增量加**
（`ALLOWED` 对叶子 crate 记的是**实际依赖**，一次声明齐会让条目在中间若干 task 里说谎）。

`ALLOWED` 加（**只列本 task 实际有的边**）：

```rust
    ("continuum-connector", &["continuum-capability", "continuum-core", "continuum-secrets"]),
```

workspace 的 `members` 加 `"crates/continuum-connector"`。**`continuum-provider`、`continuum-effect`
与三条 dev 边（`tokio` / `continuum-persist` / `tempfile`）都不在本步登记**——它们分别由 Task 4
（provider ＋ tokio ＋ persist ＋ tempfile）与 Task 5（effect）各自登记，**登记在用到它的那个 task 里，不提前也不推后**。

- [ ] **Step 2: 写用例**

`tests/register.rs` 的夹具：一个 `FakeConnector`（可声明任意描述符与任意绑定，并记录 `invoke_with` 的调用次数——**这个计数在 Task 4 起才用得上，本 task 先建**）。

- `an_operation_that_is_declared_but_not_bound_is_rejected`：声明 `{Email.send, Email.draft}`、只绑 `Email.send` → 断言 `ConnectorError::UnboundOperation { connector, op }`，且 `op` 是**没绑的那一条**（逐字比）。
- `an_operation_that_is_bound_but_not_declared_is_rejected`：声明 `{Email.send}`、绑 `Email.send → Email(Send)` 与 `Email.other → Filesystem(Read)` → 断言 `ConnectorError::UndeclaredBoundOperation { .. }`（**两条核对的另一侧**；只建一侧不算钉住）。
- `a_fully_bound_connector_registers`：**对照臂**——声明与绑定逐项对齐 → `Ok(())`。

**每条用例只触发一条核对**（构造时避开会同时触发别的核对的情形），故注册期四条核对的**先后次序在用例上不可观察**——次序由实现定死，**不要**为它写用例（写了也钉不住）。

> **`register` 收不到空声明集的描述符**：`ConnectorDescriptor::new` 对空操作集返回 `CoreError::ConnectorWithoutOperations`（`crates/continuum-core/src/connector.rs:77-84`，§125 的既有保证）。那是 P0/P1 的成果，本计划**不重做**、也不为它写用例——`register` 的入参类型里已不可能出现空集，这条由类型表达。

- [ ] **Step 3: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-connector --test register
```

- [ ] **Step 4: 实现**

```rust
/// 一条「操作 → 授权」的绑定。**两个字段都是既有类型，本 crate 不新增词汇表**（设计 §3.2）。
///
/// 字段**只有这两个**：没有 `EffectType`——效应是**推出来的**（由 `CapabilityKind::effect`
/// 给出），不是声明的（设计 §3.2 第 4 条）。这条「写不出来」由 Task 6 的编译失败样例钉。
pub struct OpBinding { op: ConnectorOp, kind: CapabilityKind }

impl OpBinding {
    /// **必须有这个公开构造入口**：连接器作者（以及本 crate 的集成测试，它们在 crate 外）
    /// 要在 `bindings()` 里造出绑定，而字段私有、无构造函数会让 `ConnectorImpl` 实现不了。
    /// 没有可校验的东西（两个入参各自的构造期校验已在它们自己的类型上做过），故不返回 `Result`。
    pub fn new(op: ConnectorOp, kind: CapabilityKind) -> Self;
}

/// 连接器作者实现的是本 trait（§4.5 有它为什么不是 §124 那个）。`Send + Sync` 与 §124 的
/// `Connector` 同要求，因为适配器要把实现放进一个满足 `Connector` 的类型里。
pub trait ConnectorImpl: Send + Sync {
    /// **复用 §124 的 `ConnectorDescriptor`，不另建第二个描述类型**（共享面 §二）。
    fn descriptor(&self) -> ConnectorDescriptor;
    /// 每个已声明操作恰一条。**这就是绑定的产生点**，唯一产生点是 [`ConnectorRegistry::register`]。
    fn bindings(&self) -> Vec<OpBinding>;
    /// 比 §124 的 `invoke` 多一个材料入参（决定 B-4）。
    async fn invoke_with(
        &self,
        op: &ConnectorOp,
        input: Value,
        material: &SecretMaterial,
    ) -> Result<Value, ProviderError>;
}

/// **注册期全部核对的唯一产生点**（设计 §3.2.1）。
///
/// # 它不核什么（写下来，免得被读成它核过）
///
/// 四条核对**没有一条**核对「这条操作该绑哪一枚 `CapabilityKind`」：规范没有那条判据，
/// 本 crate 也没有发明（设计 §3.4）。故作者把 `GitHub.merge` 绑到 `Github(CreatePr)` 上，
/// 四条全过、**注册成功**——照片见 `tests/register.rs` 的
/// `a_mis_bound_operation_still_registers`。这条限度是本 trait 与 `register` 的**口径**，
/// 不是本 crate 没做够。
pub struct ConnectorRegistry { /* 私有 */ }

impl ConnectorRegistry {
    pub fn register(&mut self, connector: Box<dyn ConnectorImpl>) -> Result<(), ConnectorError>;
}
```

本 task 的 `ConnectorError` **只有本 task 有产生方的那两个变体**（`UnboundOperation` /
`UndeclaredBoundOperation`）——**没有产生方的变体不建**（设计 §6.1）；**其余八条**（`DuplicateKindBinding` /
`OperationServiceMismatch` 在 Task 3，`UnknownConnector` / `UndeclaredOperation` / `AuthorizationMismatch` /
`Credentials` / `Provider` 在 Task 4，`EffectAuthorizationRequired` 在 Task 5）由后续 task 增量加。
**这个数是设计 §6.1 那十行减去本 task 的两行**，改一处要跟着扫这里。

**绑定的存放**：`ConnectorOp` 只有 `Hash` / `Eq`，**没有 `Ord`**（`crates/continuum-core/src/connector.rs:26`），
故存放容器用 `HashMap`（或按注册顺序的 `Vec`），**不要**写 `BTreeMap`。

- [ ] **Step 5: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-connector Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(connector): crate 骨架与绑定的双向覆盖"
```

---

### Task 3: 一一绑定与服务半边相符

**Files:**
- Modify: `crates/continuum-connector/src/{error.rs,registry.rs}`
- Modify: `crates/continuum-connector/tests/register.rs`

**Interfaces:**
- Consumes: Task 2 的 `register` 与两条核对
- Produces: `ConnectorError::{DuplicateKindBinding, OperationServiceMismatch}`

> 本 task **不改** `Cargo.toml` 与 `ALLOWED`（没有新增依赖）。

- [ ] **Step 1: 写用例**

- `two_operations_bound_to_the_same_kind_are_rejected`：声明 `{Email.send, Email.draft}`、两条都绑 `Email(Send)` → `ConnectorError::DuplicateKindBinding { connector, kind }`（**「一一」这条的理由是 §125 的意图——「`GitHub.merge` 能单独不授」；两条绑同一 kind 会让授权其一即授权另一，设计 §3.3**）。
- `an_operation_whose_service_half_is_not_this_connector_is_rejected`：id `GitHub`、声明 `{Email.send}`、绑 `Email.send → Email(Send)` → `OperationServiceMismatch { connector, op }`。
- `the_service_half_is_compared_case_sensitively`：id `GitHub`、声明 `{github.push_branch}`、绑 `github.push_branch → Git(GitAction::Push)` → **同样 `OperationServiceMismatch`**。判据是**逐字比较、不折叠大小写**（设计 §3.2.1 末段；被否掉的替代是折叠大小写）。
- `a_positive_arm_with_the_matching_service_half_registers`：id `GitHub`、声明 `{GitHub.push_branch}`、绑 `GitHub.push_branch → Git(GitAction::Push)` → `Ok(())`。**这条同时是设计 §3.4 照片 1 的一半**：`GitHub.push_branch` 这个**操作**绑的 kind 是 `Git(Push)`，resource 是 `git` 不是 `github`（设计 §11 第 12 条）——用例注释要把这条缝写下来，免得后来者当成笔误去「修」。
- `a_mis_bound_operation_still_registers`（**设计 §3.4 照片 2，本设计初稿写反的那一侧**）：id `GitHub`、声明 `{GitHub.merge}`、绑 `GitHub.merge → Github(GithubAction::CreatePr)` → 断言 `register` 返回 **`Ok(())`**。注释必须写明：**规范没有「一条操作该绑哪一枚 kind」的判据，故这里断言的是 `Ok`，不是 `Err`**；没有这张照片，本节就是在替规范声称一条它没有的判据。

> **不写「次序」用例**：四条核对里若有多条同时不通过（例如既绑了未声明的操作、它的服务半边又对不上），报哪一条由实现的次序定。P3A 对入口四步的处理是「次序只决定同时犯两种错时报哪一种 `Err`，不决定放行与否」，注册期同理——**上述每条用例都只触发一条核对**，故次序不进入判据，**也不要为它发明一条判据**。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-connector --test register
```

- [ ] **Step 3: 实现**

服务半边 = **操作串里第一个 `.` 之前的子串**，与本连接器的 `id` **逐字比较**（`ConnectorOp::new` 已保证串里至少有一个 `.`，故 `split_once('.')` 恒有 `Some`）。一一是「同一个连接器内，一枚 `CapabilityKind` 不得被两个操作绑定」。

- [ ] **Step 4: 变异与全量验证**

逐条守卫做变异（**每条各写独立日志路径**，改后读源码确认落在目标核对上）：
- 双向覆盖的两侧各一次（本 task 的对照臂会同时红——它是承重守卫，走全量）；
- 「一一」的比对改成恒 `false`；
- 服务半边改成**折叠大小写**的比较——预期只有 `the_service_half_is_compared_case_sensitively` 红（**若它不红，说明那条用例的夹具没造对**：折叠大小写正是设计否掉的那个替代）。

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-connector
git commit -m "feat(connector): 一一绑定与服务半边相符"
```

---

### Task 4: 入口（效应臂）：四步核对、凭据签发、逐次调用的适配器

**Files:**
- Create: `crates/continuum-connector/src/{entry.rs,adapter.rs}`
- Modify: `crates/continuum-connector/src/{lib.rs,error.rs,registry.rs}`
- Modify: `crates/continuum-connector/Cargo.toml`（加 `continuum-provider`；dev 加 `tokio`、`continuum-persist`、`tempfile`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 的 `continuum-connector` 条目加 `continuum-provider`、`continuum-persist`）
- Create: `crates/continuum-connector/tests/{invoke.rs,audit.rs}`

**Interfaces:**
- Consumes: Task 2/3 的 `ConnectorRegistry`、Task 1 的 `CapabilityKind::effect`、既有的 `continuum_secrets::{SecretsRuntime, SecretsError, SecretMaterial}`、`continuum_capability::AuthorizedEffect`、`continuum_provider::Connector`
- Produces: `continuum_connector::ConnectorRegistry::invoke`、`ConnectorError::{UnknownConnector, UndeclaredOperation, AuthorizationMismatch, Credentials, Provider}`

> **本 task 只做效应臂**：入口收 `AuthorizedEffect`（不是两臂的枚举）。这**不是**设计 §5.1 的终态形状——两臂的 `ConnectorAuthorization` 由 Task 5 加上。**中间态是 fail-closed 的**：非效应型的操作在本 task 里**根本递不进来**（入口没有收裸能力的入口），故不存在「绕开强制点 (2) 做副作用」的窗口。

- [ ] **Step 1: 加依赖边**

`Cargo.toml` 加 `continuum-provider`（适配器实现 §124 的 `Connector`）；dev-dep 加 `tokio`（`#[tokio::test]`）、
`continuum-persist` 与 `tempfile`（**Step 4 的 `tests/audit.rs` 要用它们开库、读 `audit_log`**）。
`ALLOWED` 的 `continuum-connector` 条目加 `"continuum-provider"` 与 `"continuum-persist"`。
**两处一起改**——那张表的断言是逐对 `assert_eq!`。

> **dev 边必须在用到它的那个 task 里加，不能推给后面的 task**：Step 5 就要跑 `--test audit`，
> 少了这条边那一步**编译不过**（不是「断言红」，是压根跑不起来）。`cargo tree --edges all`
> 把 dev 边算进依赖，故 `ALLOWED` 也要同时登记（设计 §7.3 已写明「但有一条 dev 边必须有：
> `continuum-connector → continuum-persist`」，理由与 C 的 `persist` dev 边同）。

- [ ] **Step 2: 写夹具与用例（第一组：拒绝面）**

**夹具（两个连接器注册进同一个注册表）**——服务半边必须与连接器自己的 id 相等（Task 3 的核对），
故两个操作分属两个服务时**不能塞进同一个连接器**：

| id | 声明并绑定 | 用途 |
|---|---|---|
| `GitHub` | `GitHub.push_branch` → `Git(GitAction::Push)` | 成功臂 ＋ 设计 §3.4 照片 1 的那一对（`GitHub.push_branch` 绑的是 `Git(Push)`——resource 是 `git` 不是 `github`，**这不是错**，注释要写明） |
| `Email` | `Email.send` → `Email(EmailAction::Send)` | kind 不符、能力已失效、`EffectAuthorizationRequired`（Task 5）几条 |

两个假实现各记录 `invoke_with` 的调用次数。

- `an_unregistered_connector_is_rejected`：请求一个没有注册的服务半边（如 `Slack.post`）→ `ConnectorError::UnknownConnector { connector }`。
- `an_undeclared_operation_is_rejected`：`Email` 声明 `{Email.send}`，请求 `Email.draft` → `UndeclaredOperation { connector, op }`。
- `a_presented_capability_of_another_kind_is_rejected`：`Email` 那条的绑定是 `Email(Send)`，出示**另一条真在像里的效应**包出的 `AuthorizedEffect` → `AuthorizationMismatch { op, bound, presented }`（三个字段逐字比）。
  **出示值必须是一枚真包得进 `AuthorizedEffect` 的能力**：`AuthorizedEffect::new` 会核对 `capability.kind() == for_effect(effect)`（`crates/continuum-capability/src/effect.rs:55-65`），而 `for_effect` 的像里**没有 `Filesystem(Read)`**——故「出示一枚 `Filesystem(Read)` 的能力」那种写法**构造不出来**。本例取 `Charge` → `Payment(PaymentAction::Charge)`（在像里、且与 `Email(Send)` 不等），断言 `presented == Payment(Charge)`、`bound == Email(Send)`。
- **上三条各自都要断言「实现未被调用」**：两个假实现的调用次数都断言为 `0`（设计 §9、§10：四步都在**调用实现之前**）。

- [ ] **Step 3: 写用例（第二组：成功路径与凭据）**

- `a_push_branch_operation_reaches_the_implementation`：**对照臂**——`GitHub.push_branch` 出示 `for_effect(PushBranch)` 包出的 `AuthorizedEffect` → 实现被调用**恰一次**，返回值就是实现给出的那个 `Value`。**这条同时补上设计 §3.4 照片 1 的「一次成功调用」那一半**（Task 3 只落了它的注册那一半；另一半此前没有落点，见「遗留」第 16 条）。
- `the_credential_carries_the_scope_of_the_presented_capability`（**效应臂**；非效应臂的同形用例在 Task 5）：出示能力的 scope 是 `repo/X` → 实现收到的材料取自 `repo/X`（文件源只登记这一条）。**另加一条对照**：把出示能力的作用域改成 `repo/Y`（源不覆盖）→ `Credentials(SecretsError::ScopeNotCovered { .. })`（**断言是这一种**）。
- `an_expired_capability_cannot_reach_the_implementation`：`expiry` 已过的能力（经 `AuthorizedEffect::new` 包好——它不读时钟）→ `Credentials(SecretsError::Capability(CapabilityError::Expired { .. }))`，且**实现未被调用**。**对照臂**：`expiry - 1` 照常到达（边界的那一格）。
- `the_material_never_leaves_through_the_return_value`（设计 §9 末行；本计划按 §4.5 的口径定写，订正记入「遗留」第 15 条）：假实现把**它收到的 `input`** 原样回显成返回值 → 断言 **B 的返回值里不含材料**（材料从不进 `input`，故「回显 `input`」的返回值里也没有它）。
  **必须带一条对照臂**：假实现**主动**把 `material.expose()` 写进返回值 → 断言材料**出现在返回值里**——**B 不拦这一步**：设计 §4.5 明确否决了「在出口扫材料」那条「手工校验层」（§1.2 点名的最易失效位置）。**没有这条对照臂，上面那条在「适配器把材料塞进了 `input` 而实现没回显」时也会绿。**
- `a_backend_error_comes_back_as_provider`：假实现返回 `ProviderError::Unavailable("...")` → `ConnectorError::Provider(ProviderError::Unavailable(...))`（**内层变体逐字段断言**，不只断言 `Err`）。

- [ ] **Step 4: 写用例（第三组：否定照片——不写审计）**

`tests/audit.rs`（**与 `tests/invoke.rs` 分开**：它要一个真数据库，其余用例不要）：
- `a_successful_connector_call_does_not_write_an_audit_row`：起一个临时库（`builtin_migrations()`，`audit_log` 表来自那里），记下 `SELECT COUNT(*) FROM audit_log`；跑一次**成功**的连接器调用；再记一次——**两次相同**。
- `the_control_arm_writes_exactly_one_audit_row`：**对照臂**——走一次会写审计的既有路径（`continuum_capability::authorize` 的成功分支，它写一条 `AuditKind::CapabilityGrants`），断言行数**恰加一**。
  **这条对照臂的库要多带一件东西**：`p3_capability_migrations()`（`tool` 表）＋ 一条 `save_tool` 登记项 ＋ 与它相配的出示能力——少了这些，`authorize` 会报 `UnknownTool`（不写审计行），**对照臂就以「行数没变」的方式绿了**，而它恰恰是来证「这里本来写得进一行」的。夹具照 `crates/continuum-capability/tests/authorize.rs` 的 `db()` / `profile()` 写。
  **没有这条对照臂，上面那条在「表根本没建起来」时也会绿。**

> **这条照片的强度要写准**：B 的入口**没有任何数据库句柄**（`ConnectorRegistry` 不持有 `Tx`、`Db`），故行数不变是**结构性的**，不是「本可以用另一条路径写而这次没写」。它的价值在于把「B 不重记 §313 的两项」这条**决定 B-5** 钉成一个可执行判据（设计 §6.2）。

- [ ] **Step 5: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-connector --test invoke
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-connector --test audit
```

- [ ] **Step 6: 实现**

入口按设计 §5.3 的**四步**（本 task 只落第 1、2、4 步；第 3 步由 Task 5 加）：

```rust
impl ConnectorRegistry {
    /// **唯一一条通往 `Connector::invoke` 的入口**（决定 B-2）。
    ///
    /// 四步都在**调用实现之前**；四步全过才可能返回 `Ok`。次序只决定同时犯两种错时
    /// 报哪一种 `Err`，不决定放行与否（与 P3A `authorize` 的写法同形）。
    ///
    /// # 限度（设计 §2.2）
    ///
    /// `Connector::invoke` 是公开方法，任何人都能直接调它。本入口能给的不是
    /// 「外部调不到 `invoke`」，而是「**本入口**要过一个只有核对通过才产生的值」。
    /// 若将来绕开，那是类型上表达得出来的选择，不是「忘了接线」。
    pub async fn invoke(
        &self,
        authorization: &AuthorizedEffect,
        op: &ConnectorOp,
        input: Value,
        now: i64,
    ) -> Result<Value, ConnectorError>;
}
```

**`now` 是入参，且入口内的两次判定共用同一个 `now`**（设计 §5.1 现已如此写，裁决 B2）：
`issue` 与 `material` 都要它（`crates/continuum-secrets/src/runtime.rs:176`、`:238`），
而本项目的既有约定是「核不收时钟、`now` 由调用方给」（`Capability::is_valid_at` 的文档；
`continuum-secrets` 的 `CredentialSource` 同）。**入口自己不取第二个时钟**——两次判定各取一次，
「签发时未过期、取料时已过期」那一格就会被抹掉，而它正是 §103 与 §51 要看得见的那一格。

**凭据路径**（设计 §4.1）：核对全过 → `SecretsRuntime::issue(出示的那枚能力, now)` →
`SecretsRuntime::material(&cred, now)` → 构造**逐次存活的适配器** → 对它调用 §124 的 `Connector::invoke`。
**逐次签发而不是构造时一次**（设计 §4.1）：作用域随能力逐次不同。

**适配器**（`src/adapter.rs`，决定 B-4）：持 `&dyn ConnectorImpl` 与 `&SecretMaterial`，
实现 `continuum_provider::Connector`：`descriptor()` 转发给实现，`invoke(op, input)` 转发给
`ConnectorImpl::invoke_with(op, input, material)`。**它只在入口里构造，材料只在调用期间存活**——
故它是 `pub(crate)` 或私有，**不开放为公开 API**（没有消费者，且公开它等于给材料多一个入口）。
`SecretMaterial` 只在 `invoke_with` 的带外参数里出现，**绝不进 `input`**（设计 §4.5 的否决理由：
`input` 是 `serde_json::Value`，实现可以原样回显到返回值，而返回值会流向 Artifact / Journal / 审计）。

**入口与适配器的注释里要写下轮换的可观察范围与它的限度**（设计 §4.4、§11 第 7 条）：`material` 判的三件事
（本运行时签出 / 未被轮换取代 / 未到期）**只在取料那一刻**发生；材料一旦取出并交给实现，
**后续的 `rotate` 不影响这次调用**——本阶段没有「用后即焚」或「调用中途复查」的机构。
故「轮换使旧凭据失效」在本子项目的可观察范围是**「下一次取料」**，不是「正在进行中的那一次调用」。
**这一段没有照片，也写不出照片**（一次调用之内没有可插入 `rotate` 的位置，见「遗留」第 5、18 条）——
按纪律 2，**明写它为什么没有照片**，不要只在报告里说。

**`ConnectorError` 本 task 加五个变体**（各有产生方，见上）：`UnknownConnector` /
`UndeclaredOperation` / `AuthorizationMismatch` / `Credentials(#[from] SecretsError)` /
`Provider(#[from] ProviderError)`（**数一下：五个**——设计 §6.1 的十行里，注册期四条已在 Task 2/3 落地，
`EffectAuthorizationRequired` 在 Task 5 落地，余下五条在此）。**`Provider` 独立于 `Credentials`**：后端不可用与凭据拿不到是两回事，
并成一个变体会让「凭据被拒」这条路径上的消息说一件不真的事（设计 §6.1）。`Credentials` **不展开**
`SecretsError` 的八个变体（再抄一层就会与它漂移），「是哪一种失败」由内层变体给出。

- [ ] **Step 7: 变异与全量验证**

逐条守卫做变异，**每条独立日志路径**：
- 第 4 步的 `==` 改成 `!=`（承重守卫，走全量）：预期 `a_presented_capability_of_another_kind_is_rejected` 红，且 `a_push_branch_operation_reaches_the_implementation` 也红（它出示的 kind 与绑定相等，`!=` 会把它一并拦下——**两条一起红才是这两条守卫都在靶上的证据**）；
- `issue` 的返回值改成 `material` 之前先丢弃（即把「先签发再取料」改成「只取料」）——**这条预期编译不过，故按纪律 1(c) 不算「变红」，改用**：把 `issue` 收到的那枚能力换成 `authorization.capability()` 之外的某一枚固定 kind 的能力（**等价性先自检**：举不出哪个入参上两版结果不同就换真变异体）；
- 把适配器改成把 `material` 也塞进 `input`：预期 `the_material_never_leaves_through_the_return_value` 红（**它正是为这一条变异写的**——材料一旦进了 `input`，实现「回显 `input`」就会把它带进返回值）；
- 把 `ProviderError` 吞成 `Ok(Value::Null)`：预期 `a_backend_error_comes_back_as_provider` 红。

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-connector crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(connector): 入口的核对、凭据签发与逐次调用的适配器"
```

---

### Task 5: 非效应臂与入口第 3 步

**Files:**
- Modify: `crates/continuum-connector/src/{error.rs,entry.rs,lib.rs}`
- Modify: `crates/continuum-connector/Cargo.toml`（加 `continuum-effect`；**dev 依赖本 task 无新增**——`tokio` / `continuum-persist` / `tempfile` 已在 Task 4 加过）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`continuum-connector` 条目加 `continuum-effect`）
- Modify: `crates/continuum-connector/tests/invoke.rs`
- Modify: `crates/continuum-connector/tests/audit.rs`（**本 task 改了入口签名，Task 4 建的那个文件里有一处调用要跟着改**——见 Step 2 末条）

**Interfaces:**
- Consumes: Task 1 的 `CapabilityKind::effect`、Task 4 的入口
- Produces: `continuum_connector::ConnectorAuthorization`（两臂）、`ConnectorError::EffectAuthorizationRequired`

> **入口签名一变，`tests/` 下所有调用点都要跟着变**：本 task 把 `invoke` 的入参从 `&AuthorizedEffect`
> 换成 `ConnectorAuthorization`，而 **Task 4 建的 `tests/audit.rs` 里有一次成功调用**（`a_successful_connector_call_does_not_write_an_audit_row`）。
> 只改 `tests/invoke.rs` 会让那个文件编译不过——**这是跨 task 的顺序问题，单看任何一个 task 都看不出来**。
> 故自检不能只跑新改的那个文件，**必须 `cargo build --workspace --all-targets`**（Step 2 已如此写）。

- [ ] **Step 1: 写用例**

**夹具**：在 Task 4 那两个连接器（`GitHub` / `Email`）之外再加**第三个**——id `Filesystem`、
声明并绑定 `Filesystem.read` → `Filesystem(FsAction::Read)`（**服务半边必须等于连接器 id**，
这是 Task 3 的核对；绑一枚不在 `for_effect` 像里的 kind 才拿得到非效应臂）。下面前三条用它。

- `a_non_effect_operation_is_reached_with_a_presented_capability`：出示该 kind 的裸 `Capability` → 实现被调用。**这条证明的是臂的机制**，**不是**「§125 的读操作已经被表达」——词汇表里今天**没有**与 `GitHub.read_repo` 语义相配的 kind（设计 §3.4、§11 第 3 条），用例注释要写明这一点。
- `an_effect_operation_presented_with_a_bare_capability_is_rejected`（**第 3 步、fail-open 的那一侧**）：绑定 kind 落在 `for_effect` 的像里（如 `Email(Send)`），出示**同一枚 kind** 的非效应臂 → 断言 `ConnectorError::EffectAuthorizationRequired { op, effect }`，`effect` 是 `EffectType::SendEmail`（逐字比），**且实现未被调用**。**注意这条用例必须让出示的 kind 恰好等于绑定的 kind**——否则第 4 步会先拦下来，本变体就拿不到照片（设计 §5.3 的两行表）。
- `the_reverse_mismatch_is_caught_by_the_fourth_step_not_the_third`（**两个方向各一条**）：绑定 kind **不**在像里（`Filesystem(Read)`），出示效应臂（kind 必在像里）→ 断言 `AuthorizationMismatch`，**而不是** `EffectAuthorizationRequired`——那一条的 `effect` 字段在绑定 kind 不在像里时**没有值可填**（设计 §5.3）。
- `the_credential_carries_the_scope_of_the_presented_capability_on_the_non_effect_arm`：**非效应臂的作用域照片**（设计 §9 那一行要「**两条臂各一条**」；效应臂那条在 Task 4）——出示的裸 `Capability` 的 scope 是 `repo/X`，断言凭据的作用域（即实现收到的材料所取自的作用域）就是 `repo/X`；**对照臂**：把出示能力的作用域改成源不覆盖的 `repo/Y` → `Credentials(SecretsError::ScopeNotCovered { .. })`。**这条不是重复**：两臂那枚能力的**来源不同**（效应臂由驱动铸、非效应臂由调用方铸），故两条各要自己的照片。
- `an_expired_capability_on_the_non_effect_arm_is_rejected`：过期能力经**非效应臂** → `Credentials(SecretsError::Capability(CapabilityError::Expired { .. }))`。**能力已失效这一条两条臂各一次**（设计 §9）。
- `every_variant_of_secrets_error_survives_the_conversion`：对 `SecretsError` 的八个变体逐项断言 `ConnectorError::from(e)` 之后内层仍是**原来那一个**（`Superseded` / `ForeignCredential` 也在内）。**这条照片的强度要写准**：它钉的是「转出即保留内层变体」，**不是**「入口路径上这八个都出现过」——`Superseded` 与 `ForeignCredential` 经入口**不可达**（见「遗留」第 5 条）。

- [ ] **Step 2: 改掉 Task 4 那一处调用点，再跑，确认失败**

改 `tests/audit.rs` 的 `a_successful_connector_call_does_not_write_an_audit_row`：把那次调用从
`invoke(&authorized_effect, …)` 改成 `invoke(&ConnectorAuthorization::Effect(authorized_effect), …)`
（**只改出示值的构造，断言一个字不动**）。

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-connector --test invoke
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

**两条都要跑**：前者确认新用例是红的（第 3 步与两臂都还没实现）；后者是**编译面**的自检——
本 task 改了公开签名，改漏的调用点不会让 `--test invoke` 报错，只会让**别的**测试目标编译不过。
**0 warning 也是这一条命令的必要输出**（Global Constraints）。

- [ ] **Step 3: 实现**

```rust
/// 入口的两臂出示值（设计 §5.1）。
///
/// **两臂共用一个入口而不是两个入口**：两个入口就是「谁记得调哪一个」的问题，
/// 而本项目点名的缺陷形态正是「机制建好、路径不经过」。臂与绑定的对应由入口按
/// 绑定 kind 是否落在 `for_effect` 的像里**推出来**再核对，**不由调用方选**。
pub enum ConnectorAuthorization {
    /// 效应臂：绑定 kind 落在 `for_effect` 的像里时，只能走这一臂。
    Effect(AuthorizedEffect),
    /// 非效应臂：绑定 kind 不在像里时（无外部效应，故没有 `AuthorizedEffect` 可出示）。
    Capability(Capability),
}
```

入口签名相应改为收 `ConnectorAuthorization`（`now` 保持入参），核对补上第 3 步：
**绑定 kind 经 [`CapabilityKind::effect`] 推出 `Some(e)` 而出示的不是效应臂 → `EffectAuthorizationRequired { op, effect: e }`**。

**第 3、4 步是两条不同的核对，不得并成一条**（设计 §5.3）：第 3 步管「有没有走强制点 (2)」，
第 4 步管「拿的是不是那一枚动作」。并成一条后，`<正确的 kind> + 错误的臂` 就能过，而那条路径
恰恰是**绕开强制点 (2) 做副作用**——**这是 fail-open 的那一侧**。

**两臂在类型强度上相等**（设计 §3.5）：两条臂能保证的都是「出示方持有一枚 `mint` 铸出的、
kind 相符的能力」。**不要把效应臂读成「不可伪造」**（`AuthorizedEffect::new` 是公开的），
也**不要**由此推出「非效应臂没有强制点，故不该有」——**决定 B-3b**：两条臂都由本入口把关，
判据同一条。**「强度相等」说的是类型那一层，不包括来源**：效应臂的能力由驱动铸、作用域取
`spec.target`；非效应臂由调用方铸、**作用域没有来源**（设计 §4.2.1）。这条不对称要写进
非效应臂分支的注释里，**不要用「相等」把它盖过去**。

- [ ] **Step 4: 变异与全量验证**

- 把第 3 步删掉（改成直接落到第 4 步）：预期 `an_effect_operation_presented_with_a_bare_capability_is_rejected` 红，且红的是**变体不对**（报 `AuthorizationMismatch`）——**这是承重守卫、fail-open 的那一侧，须走全量套件**；
- 把第 3 步的判据从「绑定 kind 在像里」改成「出示的是效应臂」：预期 `the_reverse_mismatch_is_caught_by_the_fourth_step_not_the_third` 红（两版在**绑定 kind 不在像里而出示效应臂**这个入参上给出不同结果，故不是等价变异体）；
- `#[from]` 的转出改成手工 `map_err` 丢掉内层：预期 `every_variant_of_secrets_error_survives_the_conversion` 红。

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-connector crates/continuum-runtime/tests/dependency_direction.rs Cargo.lock
git commit -m "feat(connector): 非效应臂与入口第 3 步"
```

`git add` 的路径是**目录** `crates/continuum-connector`，故 `tests/audit.rs` 的那处改动**已含在内**——
**不要**把路径收窄成 `tests/invoke.rs`（那正是本 task 最容易漏的一处）。

---

### Task 6: 编译失败样例（两处「写不出来」）

**Files:**
- Create: `crates/continuum-connector/tests/type_level.rs`
- Create: `crates/continuum-connector/tests/compile_fail/*.rs`（各配同名 `.stderr`）
- Modify: `crates/continuum-connector/Cargo.toml`（dev-dep `trybuild`）

> **不改 `ALLOWED`**：本 task 没有新增 workspace crate 的依赖边（`trybuild` 是外部 crate，不进那张表——它列的是**内部** crate 的允许集合）。

**Interfaces:**
- Consumes: Task 2 的 `OpBinding`、Task 5 的 `ConnectorAuthorization`

- [ ] **Step 1: 写样例**

两份样例（**判据是编译失败**，每份 `.stderr` 钉住预期报错——否则「因为拼错函数名而编译失败」也会让用例变绿）：

- `the_effect_arm_requires_an_authorized_effect.rs`：在 `ConnectorAuthorization::Effect(..)` 的位置传一枚裸 `Capability` —— **不编译**。
  **这条样例钉住的是哪一件事，要说准**（设计 §5.1）：它只钉「`Effect` 变体的字段类型是 `AuthorizedEffect`」，
  **钉不住**「效应型操作不得用裸能力驱动」——后者是**运行期**核对（Task 5 第 3 步），
  它的照片是 `EffectAuthorizationRequired` 那一条。**不要把两者混为一谈。**
- `op_binding_has_no_effect_field.rs`：**先 `OpBinding::new(op, kind)` 造出一条绑定，再对它写 `binding.effect = …`** —— **不编译**（`E0609`：`no field \`effect\` on type \`OpBinding\``）。
  这条钉的是「效应是推出来的、不是声明的」（设计 §3.2 第 4 条、§9）。

  **样例的形状是刻意的，判别力不如表面看上去那么强，要写准**：`OpBinding` 的字段是**私有**的，故**任何**外部结构体字面量（`OpBinding { op, kind }`）都编译不过，而 rustc 报的很可能是**隐私错误**（`E0603`，或「cannot construct with struct literal syntax due to private fields」）而不是「没有 `effect` 字段」——那样这条样例就分不出「效应是推出来的」与「外部构造不出 `OpBinding`」，**声称钉住的东西其实没钉住**。故本样例走**字段赋值**那条路（它只碰字段名，不碰可见性），并且 **`.stderr` 必须是 `E0609` / `no field \`effect\``**；**若生成出来的 `.stderr` 是隐私错误，这条样例按现形状不算数**，须换一条不落在可见性上的写法（例如断言 `OpBinding` 的公开构造入口只收两个入参：`OpBinding::new(op, kind, effect)` 报「参数个数不符」）。

`tests/type_level.rs` 用 `trybuild::TestCases::new().compile_fail("tests/compile_fail/*.rs")` 驱动（与 `continuum-capability` / `continuum-secrets` 的 `tests/type_level.rs` 同形——**照那两处写，不要另发明一种驱动方式**）。

- [ ] **Step 2: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-connector --test type_level
```

首次运行会生成/比对 `.stderr`：**生成出来的那份要人读一遍**（`TRYBUILD=overwrite` 只生成，不校对），
确认它报的是**上面逐条指定的那一个**（第一份＝类型不符，第二份＝`E0609` 没有 `effect` 字段），
而**不是**本 crate 里别的拼写错误或隐私错误。

- [ ] **Step 3: 变异与全量验证**

- 把 `ConnectorAuthorization::Effect` 的字段类型换成 `Capability`：预期**第一份样例不再失败**（用例红），第二份不受影响；
- 给 `OpBinding` **加上**一个公开的 `effect: Option<EffectType>` 字段：预期**第二份样例不再失败**（用例红）。
  **这一条同时是它的判别力自检**——若加了字段它仍「失败」，说明它红在**别的原因**上（隐私、拼写），
  **那条样例就没在钉它声称的东西**。

**这两条的判据是「编译失败」而不是「test failed」**（纪律 1(c)）：读日志时看 `could not compile`。

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-connector Cargo.lock
git commit -m "test(connector): 效应臂的字段类型与绑定的字段集"
```

---

### Task 7: 驱动装配 `SecretsRuntime` 与 `runtime → secrets` 边

**Files:**
- Create: `crates/continuum-runtime/src/secrets.rs`
- Modify: `crates/continuum-runtime/src/main.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`（加 `continuum-secrets`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`continuum-runtime` 条目加 `continuum-secrets`）
- Create: `crates/continuum-runtime/tests/secrets_assembly.rs`

**Interfaces:**
- Consumes: 既有的 `continuum_secrets::{SecretsRuntime, FileCredentialSource, EnvCredentialSource, SecretsError}`
- Produces: `continuum-runtime`（bin）的 `secrets::assemble`

> **裁决 §六.2 把这条边与这处装配的所有者写死为 B**（依据：`p3a-followups.md` 第一/三节——
> 「B 接上之前，这套保证在运行期等于不存在」，而那条边「登记它的时机是 B 把密钥运行时接上的那个 task」）。
> **本 task 不登记 `runtime → connector`**：那条边无生产消费者，裁决 §六.3 判它落成遗留、不做 task。

- [ ] **Step 1: 登记边，跑，确认现有断言变红**

先在 `ALLOWED` 的 `continuum-runtime` 条目加 `"continuum-secrets"`，**不写代码**，跑：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期**红**：`cargo tree` 里还没有这条边，而允许集合里已经有它（那张表是逐对 `assert_eq!`）。
**这一步是这条边的红-绿起点**——`ALLOWED` 登记的是**实际依赖**，只登记不接线就是假边。

- [ ] **Step 2: 写用例**

`tests/secrets_assembly.rs`（驱动真实二进制，与 `tests/task_cli.rs` 同法）：

- `a_broken_credential_source_fails_the_startup`：设 `CONTINUUM_CREDENTIALS` 指向一个不存在的路径，跑二进制 → **退出码非零**，stderr 含 `SecretsError::SourceIo` 的具体消息（`凭据源 … 读取失败`）。
- `an_absent_configuration_leaves_the_startup_working`：**对照臂**——不设该变量，跑一个既有的无害子命令（如 `recover --db <临时库>`）→ 成功。**没有这条，上一条在「驱动本来就起不来」时也会绿。**

> 环境变量是进程级的：这两条用例与同 crate 里别的二进制用例**必须分开进程**（每条用例各自 `Command::new`，不要用 `std::env::set_var` 改本进程环境）。`std::env::set_var` 在 Rust 2024 里已是 `unsafe`，**不要用**。

- [ ] **Step 3: 跑，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test secrets_assembly
```

- [ ] **Step 4: 实现**

`crates/continuum-runtime/src/secrets.rs`（bin 模块）：

```rust
/// 装配密钥运行时（设计 §4.1：**驱动装配好传进来**，不在连接器里自建——连接器自带凭据源会让
/// 「凭据从哪来」在装配处看不见）。
///
/// 凭据源的位置取自环境变量 `CONTINUUM_CREDENTIALS`：设了就用文件源（读不出来即失败，
/// **fail-closed**——吞掉它会让「凭据源配错了」这件事拖到第一次调用才现形）；未设则用
/// 环境变量源（前缀 `CONTINUUM_SECRET_`，TTL 取本模块的默认值）。
pub(crate) fn assemble() -> Result<SecretsRuntime, SecretsError>;
```

`main.rs` 在分派之前调用它，失败即打印「启动失败: {e}」并 `ExitCode::FAILURE`。

**配置来源是本计划自定的**（设计只写了「驱动装配」，没写凭据源的位置从哪来）——记入「遗留」第 7 条。

**装配出来的运行时的消费者是谁，必须如实写在代码注释里**：本轮**没有**——它的唯一消费者是
B 的连接器入口（在 `continuum-connector` 里），而那条边按裁决 §六.3 不接。故本步**不假装它被用到了**：
注释写明「本值与 §11 第 13b 条那条未接线同源，接上时它作为 `ConnectorRegistry::new(runtime)` 的入参」，
并在「遗留」第 2 条记下这个限度。

- [ ] **Step 5: 全量验证并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-runtime Cargo.lock
git commit -m "feat(runtime): 装配密钥运行时并登记 runtime → secrets"
```

**变异**：删掉 `main.rs` 里的装配调用 → 预期 `dependency_direction` 的同名用例红（`cargo tree` 里那条边消失，而 `ALLOWED` 里还在）**且** `a_broken_credential_source_fails_the_startup` 红。**这一条是承重守卫（跨 crate 才可见），须走全量套件。**

---

### Task 8: 收尾与复核

**Files:**
- Modify: `docs/superpowers/p3bcdf-followups.md`（**并入既有的那一份，不新建第二个同类文件**——同一类事项两个家正是本项目反复处置的毛病）

- [ ] **Step 1: 全量验证**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：全绿、0 warning。

- [ ] **Step 2: 逐条核对完成判据**

| 判据（设计 §10） | 证据 | 缺不缺 |
|---|---|---|
| 「连接器做副作用必须收下 `AuthorizedEffect`」的**编译期那一半** | Task 6 的 `the_effect_arm_requires_an_authorized_effect` | — |
| 同一条的**运行期那一半** | Task 5 的 `an_effect_operation_presented_with_a_bare_capability_is_rejected` ＋ 三条「拒时实现未被调用」 | **这一半不能由编译期那一半代表**（设计 §10） |
| 「凭据由密钥运行时按能力逐枚签发」 | Task 4 的凭据作用域用例 ＋ `Expired` 的两臂各一次（Task 4/5） | — |
| 「第一个真消费方在**生产路径**上」 | **未兑现**：`runtime → connector` 不接，装配出的运行时无消费者 | **据实标为未兑现**，见「遗留」第 1、2 条 |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 3: 复核 P3A 两处「建了但无生产调用方」的兑现情况**

逐项核对 `docs/superpowers/p3a-followups.md` 第一节那张表：
- **强制点 (2) 的连接器侧（`AuthorizedEffect`）**：本计划 Task 4/5 是它的消费方，**已兑现**（照片见上表）；
- **`continuum-secrets`**：本计划给它接上了 `continuum-runtime`（Task 7）与 `continuum-connector`（Task 4）两条真边，**但它仍无生产调用方**——照实写，并说明这是设计已声明（设计 §11 第 13b 条）还是漏接线（**不是漏接线**）。

- [ ] **Step 4: 残余落到有版本的文档**

`.superpowers/` 是 gitignore 的，**只写在报告或 ledger 里的结论会随 branch 消失**。把本计划的「遗留」逐条写进 `docs/superpowers/p3bcdf-followups.md`（**并入，不新建**），并标明每条的收件人。

- [ ] **Step 5: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs: P3 子项目 B 的收尾与复核"
```

---

## 遗留

> 本节各条**都要**落到 `docs/superpowers/p3bcdf-followups.md`（Task 8 Step 4）。凡设计未给判据的，
> 标明「规范未给判据」；凡本计划自己定了的，标明「设计未规定，本计划自定」。

```
1. runtime → connector 本轮无生产消费者（裁决 §六.3，不做 task）
   机制名：continuum-connector 的入口 ConnectorRegistry::invoke（Task 4/5 交付，全部用例跑在假连接器上）。
   缺的是哪一步接线：驱动执行效应今天只有「跑命令」一条路（Sandbox::spawn），没有
   「经连接器执行效应」的路径；接上时它作为 main.rs 装配处的一项（ConnectorRegistry::new(secrets_runtime)），
   并同步登记 runtime → connector 的 ALLOWED 条目。
   后果：continuum-connector 是一个完整实现且自测通过、但无人调用的 crate；设计 §10 的第一条完成判据
   在生产路径上没有落点。**这不是漏实现，设计 §11 第 13b 条已如此声明（收件人：协调者）。**
   收件人：协调者（派工时决定这条由谁接）。

2. runtime 装配出来的 SecretsRuntime 本轮也没有消费者（与第 1 条同源）
   Task 7 按裁决 §六.2 登记了 runtime → secrets 并落了 main.rs 的装配，但那个运行时的唯一消费者
   就是第 1 条里那个不接的入口。故这条边按 cargo tree 是**真依赖**、按使用是**零消费者**——
   与「零使用的边即假边」的既有口径**相抵**。本计划照裁决落，并在此标出这条相抵供复核。
   收件人：协调者。

3. 「一条操作该绑哪一枚 CapabilityKind」**规范未给判据**（设计 §3.4、§11 第 16 条）
   §125 只规定粒度（不得退化为服务级），没有规定这条对应；本设计也没有发明它。本 crate 保住的只是
   「不拿到声明的那一枚就走不到实现」，**没保住**「声明的那一枚是对的」。照片是反面：
   tests/register.rs 的 a_mis_bound_operation_still_registers（register 返回 Ok）。
   register 的文档已把这条限度写死（Task 2 Step 4）。
   收件人：规范（是否要定「操作 → kind」的对应，以及定在哪一层）。

4. 非效应臂的强制点归属**规范未给判据**，且那枚能力**没有具名的产生方**（设计 §3.5、§11 第 3b/17 条）
   决定 B-3b：两条臂都由 B 的入口按同一判据把关（出示的 kind 必须等于绑定的 kind）。
   效应臂的能力由驱动铸、作用域取 spec.target；**非效应臂由调用方铸，其作用域没有来源**——
   B 能保证的只有「凭据作用域 == 出示的能力作用域」这条同义反复式的相等，**不是**「作用域是对的」。
   收件人：规范（非效应的连接器操作，其强制点与作用域由谁给）。

5. Superseded / ForeignCredential 经本入口**不可达**（设计 §9 的「轮换」行与 §6.1 的产生方栏）
   入口一次调用内完成 issue → material，中间没有可插入 rotate 的位置；且凭据总是本运行时本代签出的。
   故「B 的入口把 Superseded 原样转出」这条**照片写不出来**——能写的只到
   Task 5 的 every_variant_of_secrets_error_survives_the_conversion（钉「转出保留内层变体」）。
   §103 四类事件那一组是**复用既有照片**（continuum-secrets 的 tests/issue.rs），不是本子项目的新证据。
   收件人：规范/设计（若要这条可观察，需要入口给出一个能落在 issue 与 material 之间的位置）。

6. 本计划对 P3A crate 的唯一新增面：CapabilityKind::effect（Task 1）
   它是 for_effect 的逆、唯一产生点，消费方是 B 的入口（Task 4 第 3 步的判据来源、Task 5 的错误字段）。
   本计划不改 continuum-capability 的依赖边。
   收件人：无（本计划内闭合）。

7. 凭据源的位置从哪来，**设计未规定**（设计 §4.1 只写「驱动装配好传进来」）
   本计划自定：环境变量 CONTINUUM_CREDENTIALS 给文件源路径，未设则用环境变量源；
   配了却读不出来即启动失败（fail-closed）。这两处都是本计划的选择，不是规范的判据。
   收件人：协调者（确认后固定；若改口径，Task 7 的用例与 main.rs 一起改）。

8. 材料出到实现之后无回收机构（设计 §11 第 9 条）
   材料在调用期间存活，随适配器析构。本阶段够用；若将来要求显式清零（§100 的硬件后端语境），
   替换点是适配器。
   收件人：长期阶段。

9. 真实连接器零个（设计 §11 第 10 条）
   本子项目全部用例跑在假连接器上，「按操作细分」在真实服务上的表现、以及「拿着这枚令牌真的调不动
   别的仓库」都没有照片。假连接器只能钉门，钉不了门后的东西。
   收件人：长期阶段。

10. **已闭（裁决 B1）**：设计 §3.3 第 2 条初稿列举「非像的 kind」时只写了五枚、漏了 Filesystem(Write)
    十二枚 kind 里非像的有**六枚**，漏的那一枚是 Filesystem(FsAction::Write)。**设计现已逐枚列全**
    （含一句「这一句初稿只列了五枚」的来历），故本条的**现在时陈述与收件人都已不成立**。
    **本计划按那六枚写 Task 1 的用例**（只列五枚会让 Filesystem(Write) 的臂漂移而无人发现）。
    留格是为了让「一处数量断言漏项是如何被查出的」可查。
    收件人：无（已闭）。

11. 空 ConnectorId 与「以 . 开头的操作」这条缝未议
    ConnectorId::new 不拒空串（crates/continuum-core/src/connector.rs:14），而 ConnectorOp::new(".x") 合法
    （非空且含 `.`），服务半边取第一个 `.` 之前的子串即空串——故 id 为空串的连接器可以声明这类操作
    并通过服务半边核对。规范未议此情形，**本计划不发明一条核对**，据实记此。
    收件人：规范 / continuum-core（要不要在 ConnectorId 的构造期拒空串）。

12. **已闭（裁决 B4）**：设计 §7.3 曾写「不新增 continuum-connector → continuum-persist」，
    而 §9 的「不写审计」照片要裸查 audit_log 的行数
    那张照片必须读表，故**必须**有一条 dev 边。**设计 §7.3 现已写明**「但有一条 dev 边必须有：
    `continuum-connector → continuum-persist`（dev-dependency）」，并给了理由（主依赖不落库、
    测试夹具要读库）、点明 `ALLOWED` 必须覆盖它（`cargo tree --edges all` 含 dev）。
    **「两处相抵」这个陈述已不成立**；本计划在 **Task 4 Step 1** 登记这条 dev 边与 `ALLOWED` 条目
    （**登记在用到它的那个 task，不能推后**——Task 4 Step 5 就要跑 `--test audit`）。
    留格是为了让这处相抵是怎么合的、以及「dev 边也会被 `ALLOWED` 抓」这条判据可查。
    收件人：无（已闭）。

13. 效应是推出来的、不是声明的这一条，除了 Task 6 的编译失败样例，没有别的照片
    OpBinding 上没有 EffectType 字段可填（编译期），而「连接器声明了 EffectType」这件事在类型上
    根本不存在——故不存在可照的反面。据实标明。
    （**该样例的判别力有限、形状是刻意的**：字段私有会让**任何**外部字面量都编译不过，
    故样例走字段赋值那条路，判据是 `.stderr` 报 `E0609` 而非隐私错误，见 Task 6 Step 1。）
    收件人：无。

14. **已闭（裁决 B2；本条由 plan-b 查出）**：入口的 now 曾是入参而设计 §5.1 的签名里没有它
    §4.1 的流程要 issue(cap, now) 与 material(&cred, now)，而本项目的既有约定是「核不收时钟、
    now 由调用方给」（Capability::is_valid_at 与 CredentialSource 的文档同）。
    **设计 §5.1 现已把 now 写进签名**，并写明「入口不收时钟」与「入口内的两次判定共用同一个 now」
    （各取一次会抹掉「签发时未过期、取料时已过期」那一格）。本计划 Task 4 Step 6 按那一版写。
    留格是为了让「计划查出、裁决改设计」这条来往可查。
    收件人：无（已闭）。

15. **设计 §9 末行与 §4.5 相抵，本计划取 §4.5 的口径**（计划侧订正，设计未改）
    §9 末行写「假实现把收到的材料原样回显 → B 的返回值里**不含**它」；而 §4.5 明确否决
    「在出口扫材料」（那是 §1.2 点名的「手工校验层」）。两条不可能同时为真：实现若**主动**把材料写进
    返回值，B 不拦也不该拦。故本计划的照片钉在**可证的那一半**上——**材料不进 `input`**
    （实现「回显它收到的 `input`」时，返回值里因此不含材料），并带一条对照臂把「实现主动回显」那一侧
    照出来（Task 4 Step 3 的 the_material_never_leaves_through_the_return_value）。
    **错误说法的来历留在原地**（纪律：订正时不删原句）。
    收件人：设计（§9 末行要么按 §4.5 改写，要么说明谁来扫出口）。

16. **设计 §3.4 照片 1 的「一次成功调用」那一半此前没有落点**
    照片 1 要求 Email.send→Email(Send) 与 GitHub.push_branch→Git(Push)「各一条注册 + **一次成功调用**」；
    设计 §9 的测试表里只有「两条臂的强度」那一条涉及成功调用，且用的是别的 kind。
    本计划把成功调用那一半补在 **Task 4 Step 3 的 `a_push_branch_operation_reaches_the_implementation`**
    （那正是照片 1 的那一对），另由 Task 5 的非效应臂用例覆盖臂的机制。**若设计判定「一对即可」，
    这条可标为不必补**——本计划不替设计判，据实记此。
    收件人：设计（照片 1 是否需要两对各自成立）。

17. CAPABILITY_LIFETIME_MS 的数值无规范来源（设计 §11 第 6 条），且「铸出 → issue」之间由谁引入间隔未定
    Task 4 的 Expired 用例自己造过期能力（不依赖那个常量），故本计划不押注它的数值。
    收件人：子项目 B 的实现（首次接上生产路径时核这个数）。

18. **设计 §11 第 7 条**：轮换只在下一次取料时可见，进行中的一次调用不受影响
    **本计划的落点**：Task 4 Step 6 要求把这段限度写进入口与适配器的注释
    （`material` 的三项判定只在取料那一刻发生；材料交给实现之后 `rotate` 不影响这次调用；
    可观察范围是「下一次取料」）。**没有照片，也写不出照片**——一次调用之内没有可插入 `rotate` 的位置
    （与第 5 条同源，但两条说的不是一件事：第 5 条是「`Superseded` 这个变体经入口达不到」，
    本条是「轮换对**进行中**的调用不起作用」，后者即使入口以后能停在中途也仍成立）。
    **本阶段的机构边界，不是本子项目没做够**；若要变更，须先有「用后即焚 / 调用中途复查」的机构。
    收件人：长期阶段（+ 子项目 B 的实现的注释已就地写明）。

19. **设计 §11 第 12 条**：两套串的服务半边对不齐（连接器说 `GitHub`、能力说 `git`）
    `GitHub.push_branch` 这个**操作**必须绑 `Git(Push)`（resource 是 `git`），今天这不是错——
    §88 与驱动的铸法都如此。**本计划的落点**：Task 3 的
    `a_positive_arm_with_the_matching_service_half_registers` 用例注释里写明这条缝
    （免得后来者当成笔误去「修」）。
    **未决的一半**：这条缝在 §125 的服务清单（§124 列了六个服务）与 `CapabilityKind` 的 resource 集之间
    **普遍存在**；若后续出现**第二个服务也需要同一枚 kind**，须重新处置（例如 `GitHub` 与 `GitLab`
    的 push 都只能绑 `Git(Push)`——一一绑定是**按连接器**判的，故两者各自绑它并不冲突，
    但两枚能力的作用域与语义会共用一个 kind）。**本计划不预先发明处置方式**。
    收件人：语义层（服务与 resource 的对应）+ 子项目 B 的实现（遇到第二个服务时回报）。
```
