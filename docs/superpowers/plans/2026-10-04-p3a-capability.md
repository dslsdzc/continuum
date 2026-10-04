# P3 子项目 A（Capability）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建 `Capability` 类型与 Tool Registry，并把 Capability 的三个强制点全部接上——其中两个落在边界层，是 P2 因「这枚 token 尚不存在」而特意留下的。

**Architecture:** 新增两个 crate：`continuum-capability`（资源层：五要素类型 + 半封闭词汇表 + Tool/ToolProfile + ToolRegistry + 唯一签发点）、`continuum-secrets`（边界层：密钥运行时）。强制点 (1) 落在 Tool Registry，用「非法状态不可表达」（`AuthorizedTool`）而非调用方自觉；强制点 (2) 落在驱动 `Sandbox::spawn` 之前，连接器侧的半个义务由类型承载；强制点 (3) 是密钥运行时的凭据签发。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-persist`（`Tx`、迁移）；`continuum-events`（审计，`AuditKind::CapabilityGrants` 首次有产生方）；`continuum-effect`（`EffectType`，词汇表的对应基准）；`serde` / `serde_json`；`thiserror`；`trybuild`（类型层不可构造性）。

**设计依据：** `docs/superpowers/specs/2026-10-04-p3a-capability-design.md`。

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。** 本计划新增的边：
  `continuum-capability → continuum-core, continuum-effect, continuum-persist, continuum-events`；
  `continuum-secrets → continuum-core, continuum-capability`；
  `continuum-runtime → continuum-capability, continuum-secrets`。
  `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 与各 `Cargo.toml` 必须**精确一致**（断言是逐对 `assert_eq!`）。
- `continuum-core` 不含 I/O。数据库连接对象在 `continuum-persist` 内私有，外部 crate 只能通过 `Tx` 访问。
- **枚举列的落库编码一律小写、多词以 `_` 连接，经显式辅助函数读写，不依赖 serde。** 编码挂在其类型上，不在 `persist` 里另建表。
- 迁移编号**须先查已占用的值**，取未占用的。迁移的注册**由用它的那个 task 自己完成**。
- `continuum-runtime` 不直接对枚举列写 SQL 字面量。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿，**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 执行期间工作区由实现者与协调者共用，只 `git add <显式路径>`。
  **新增或变更 crate 依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列了源码与清单，锁文件按本行办。

## 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」跑全量是加分。**变异的三条失效形态都要防**：(a) 锚点不唯一 → 变异没落到实现体却报 GREEN；(b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**；(c) **变异导致编译失败**——那不是「变红」。每次变异用**独立日志路径**，读前确认是本轮写的。**判据本身也会假阳性**：cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，故判「编译失败」要用 `could not compile` 或 `error[E….`。
2. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**（标明了状态就不是漏枚举）。**枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条分支各要照片；手写分支能各自漂移，故要逐项钉。**改完一处枚举，通读整段。** 判据：用例里的值字面量是**手工写的**还是**被测函数返回的**——前者钉格式，后者钉路径。
3. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 Err」。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志——`pgrep "cargo|rustc"` 看不见卡在 `D` 状态的测试二进制。受能力门控的用例要给执行/跳过条数，**承重断言不要放在门控之内**。

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**（P1 出过 20+ 处事实错误），且已确立「**代码块是示意，正文的措辞才是约束**」。因此本计划中：类型定义、函数签名、SQL、关键断言给出完整代码；直白的过程代码以正文描述；**凡与既有 crate 交互的签名，实现前须先读该 crate 的源码确认**，不符时以源码为准并回报。

---

# 文件结构

```
crates/continuum-capability/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/capability.rs   Capability、CapabilityKind、词汇表、Issuer、签发点
  src/tool.rs         Tool、ToolProfile 与各类编码
  src/registry.rs     ToolRegistry、AuthorizedTool、强制点 (1)
  src/persist.rs      tool 表与读写（与表定义同址）
  src/error.rs        CapabilityError
  tests/vocabulary.rs     半封闭词汇表与 EffectType 的对应
  tests/type_level.rs     trybuild 驱动
  tests/compile_fail/*.rs     不可构造性样例（各配 .stderr）
  tests/authorize.rs      强制点 (1) 两个方向 + 审计
  tests/persist.rs        tool 表往返

crates/continuum-secrets/
  Cargo.toml
  src/lib.rs          导出面
  src/runtime.rs      SecretsRuntime、Credential
  src/source.rs       凭据源（文件 + 环境变量）
  src/error.rs        SecretsError
  tests/issue.rs      作用域不越能力、到期不越能力、轮换
```

**既有的、本计划要改的文件**

```
crates/continuum-events/src/audit.rs          不动（CapabilityGrants 已存在，只是首次有产生方）
crates/continuum-runtime/src/task_cmd.rs      强制点 (2) 的落点
crates/continuum-runtime/src/main.rs          迁移装配
crates/continuum-runtime/Cargo.toml           新依赖边
crates/continuum-runtime/tests/dependency_direction.rs   ALLOWED 登记
crates/continuum-runtime/tests/migrations.rs  期望迁移集合
crates/continuum-runtime/tests/startup.rs     迁移计数
Cargo.toml（workspace）                        members
```

---

### Task 1: `continuum-capability` 骨架与半封闭词汇表

**Files:**
- Create: `crates/continuum-capability/Cargo.toml`
- Create: `crates/continuum-capability/src/{lib.rs,capability.rs,error.rs}`
- Create: `crates/continuum-capability/tests/vocabulary.rs`
- Modify: `Cargo.toml`（members）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 登记）

**Interfaces:**
- Produces: `continuum_capability::{CapabilityKind, FsAction, GitAction, GithubAction, EmailAction, RegistryAction, PaymentAction, EnvAction, CapabilityError}`

- [ ] **Step 1: 建 crate 骨架并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`continuum-effect`（要用 `EffectType`）。
`continuum-core` / `continuum-persist` / `continuum-events` / `serde` / `serde_json` / `thiserror`
与 dev-dep `tempfile` / `trybuild` **由需要它们的 task 增量加**（`ALLOWED` 对叶子 crate 记的是
**实际依赖**，一次声明齐会让条目在中间若干 task 里说谎；判据同「迁移由用它的 task 注册」）。

`ALLOWED` 加：

```rust
    (
        "continuum-capability",
        &[
            "continuum-core",
            "continuum-effect",
            "continuum-events",
            "continuum-persist",
        ],
    ),
```

- [ ] **Step 2: 写用例**

`tests/vocabulary.rs` 的断言内容：

- `every_effect_type_has_exactly_one_capability`：对 `EffectType::ALL` 的**每一个**变体，断言 `CapabilityKind::for_effect(effect)` 给出**恰一个** `CapabilityKind`；六条各断言一次（**逐项有照片**，不抽代表）。**方法写在 `CapabilityKind` 上而非 `EffectType` 上**——反过来会让 `continuum-effect` 反向依赖本 crate。
- `no_capability_kind_maps_to_two_effect_types`：反向查一遍，断言六个 `EffectType` 得到六个**互不相同**的 `CapabilityKind`（单射）。

- [ ] **Step 3: 运行，确认失败**

```bash
cargo test -p continuum-capability --test vocabulary
```

- [ ] **Step 4: 实现**

```rust
/// §88 与 §253 的例子给到的三个 resource，加四个由 `EffectType` 反推的（设计 §2.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    Filesystem(FsAction),
    Git(GitAction),
    Github(GithubAction),
    Email(EmailAction),
    Registry(RegistryAction),
    Payment(PaymentAction),
    Environment(EnvAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FsAction { Read, Write }

/// §88 给例：`git.read` / `git.worktree.write` / `git.commit.local` / `git.push`。
/// `DeleteRemote` 由 `EffectType::DeleteRemote` 反推（设计 §2.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitAction { Read, WorktreeWrite, CommitLocal, Push, DeleteRemote }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GithubAction { CreatePr }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmailAction { Send }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryAction { Publish }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentAction { Charge }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvAction { Deploy }
```

**`EffectType` → `CapabilityKind` 的对应**写成 `continuum-capability` 上的一个方法（**不是** `EffectType` 上的：那会让 `continuum-effect` 反向依赖本 crate）：

```rust
impl CapabilityKind {
    /// `EffectType` 与 `CapabilityKind` 的一一对应（设计 §2.2 的对照表）。
    /// 两个词汇表必须互cover：不 cover 就会各走各的，而 P2 已因这类分裂吃过 Critical。
    pub fn for_effect(effect: EffectType) -> Self { /* 六个臂，穷尽无通配 */ }
}
```

**六个臂逐个手写、不抽代表**——手写分支能各自漂移，故 `tests/vocabulary.rs` 逐项钉。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-capability Cargo.toml crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(capability): 半封闭词汇表与 EffectType 的对应"
```

---

### Task 2: `Capability` 五要素与三条结构性保证

**Files:**
- Modify: `crates/continuum-capability/src/{lib.rs,capability.rs,error.rs}`
- Create: `crates/continuum-capability/tests/type_level.rs`
- Create: `crates/continuum-capability/tests/compile_fail/*.rs`（各配同名 `.stderr`）

**Interfaces:**
- Consumes: Task 1 的 `CapabilityKind`
- Produces: `continuum_capability::{Capability, Issuer}`

- [ ] **Step 1: 写 trybuild 样例**

四个 `compile_fail` 样例，**判据是编译失败且失败原因正确**（每份 `.stderr` 钉住预期报错——否则「因为拼错函数名而编译失败」也会让用例变绿）：

- `capability_has_no_constructor.rs`：`Capability::new(...)` 不存在。
- `capability_is_not_from_string.rs`：`let c: Capability = "git.push:origin/main".parse().unwrap();` —— 无 `FromStr`。
- `capability_has_no_full_access.rs`：`CapabilityKind::FullAccess` 不存在（§253 禁止的那个形状**写不出来**）。
- `capability_fields_are_private.rs`：结构体字面量构造被拒（私有字段）。
- `an_illegal_combination_cannot_be_written.rs`：`CapabilityKind::Filesystem(FsAction::Push)` 一类组合拼不出——半封闭词汇表的意义就在这里，**判据同样是编译失败**。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-capability --test type_level
```

- [ ] **Step 3: 实现**

```rust
/// 设计 §2.4：签发来源。规范里的终极来源是 Authority Host（§292，长期阶段），
/// 本阶段尚无该宿主；`AuthorityHost` 变体在本阶段**不产出**，它标出移交的去向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issuer {
    PolicyWithExplicitApproval,
    AuthorityHost,
}

/// §253 的五要素。字段私有、无公开构造函数：唯一产出路径是 [`crate::mint`]。
///
/// 三条结构性保证（设计 §2.3）：
/// 1. `full_access` 不存在——不是「禁止作默认」，是没有这个成员；
/// 2. `expiry` 必填，无「不过期」的表示（§51 的 short-lived 是结构而非约定）；
///    **本类型不读时钟**：校验收 `now`。
/// 3. 不可与裸字符串互换——无 `From<&str>`、无 `FromStr`，只有单向的 `Display`（供审计与日志）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    kind: CapabilityKind,
    scope: String,
    expiry: i64,
    issuer: Issuer,
}

impl Capability {
    pub fn kind(&self) -> CapabilityKind { self.kind }
    pub fn scope(&self) -> &str { &self.scope }
    pub fn expiry(&self) -> i64 { self.expiry }
    pub fn issuer(&self) -> Issuer { self.issuer }

    /// §253 的二字段形状（`resource` / `action`）。存储与比较用 `kind`，
    /// 故 `Filesystem.Push` 这类组合无从写出。
    pub fn resource(&self) -> &'static str { /* 七个 resource 各自的串 */ }
    pub fn action(&self) -> &'static str { /* 各 action 的串；encoding 同落库约定 */ }

    /// 本类型不读时钟：`now` 由调用方给（本项目既有约定）。
    pub fn is_valid_at(&self, now: i64) -> Result<(), CapabilityError> {
        if now < self.expiry { Ok(()) } else { Err(CapabilityError::Expired { expiry: self.expiry, now }) }
    }
}

/// **唯一的签发点**（设计 §2.4）。今天的临时签发方是「策略裁决 + 显式确认」，
/// 也就是 `continuum-workspace` 的 `gate.rs` 已经点名的那个位置。
///
/// `granted` 是「什么授权了这一次」的表示，由调用方（驱动）从策略裁决构造——
/// 本 crate 不依赖 `continuum-policy`，故用本 crate 自己的类型。
pub fn mint(
    kind: CapabilityKind,
    scope: String,
    expiry: i64,
    granted: Grant,
) -> Result<Capability, CapabilityError>;

/// 「什么授权了这一次」。`ExplicitApproval` 对应 §5.5 的 `--approve`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grant { Policy(Verdict), PolicyWithExplicitApproval(Verdict) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict { Allow, RequireApproval, Deny }
```

`Display` 输出形如 `git.push:origin/main`（§253 的例子形状），**只出不进**。

- [ ] **Step 4: 写有效期用例**

`tests/capability.rs`：`is_valid_at` 的三个边界（未到期、**恰好到期**、已过期），各断言具体 `Err`。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-capability
git commit -m "feat(capability): 五要素与三条结构性保证"
```

---

### Task 3: `Tool` 与 `ToolProfile`

**Files:**
- Create: `crates/continuum-capability/src/tool.rs`
- Modify: `crates/continuum-capability/src/{lib.rs,error.rs}`
- Create: `crates/continuum-capability/tests/tool.rs`

**Interfaces:**
- Consumes: Task 1 的 `CapabilityKind`、Task 2 无
- Produces: `continuum_capability::{Tool, ToolId, ToolProfile, Trust, Cost, Latency}`

- [ ] **Step 1: 写用例**

- `a_profile_contains_its_tool`：构造一个 `ToolProfile` 后，其 `tool()` 给出的 `id`/`version`/`required_capabilities` 与构造时一致——**钉住「登记项含定义」而不是并列两份**（设计 §3.1）。
- `effect_class_none_means_no_external_effect`：`effect_class: None` 的工具不携带任何 `EffectType`。
  （`trust` 必填这一条**没有用例，也不该有**：它由类型表达（无 `Option<Trust>` 的入口），
  而「某入口不存在」只有 trybuild 能钉——本 task 不为它造，理由写在 `ToolProfile` 的文档注释里。）

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-capability --test tool
```

- [ ] **Step 3: 实现**

```rust
pub struct ToolId(String);   // 同 ArtifactId/IntentId 的既有形态：私有字段 + as_str + Display

/// §252。`effect_class` 绑到 `EffectType`（设计 §3.2），**不另造分类**。
pub struct Tool {
    id: ToolId,
    version: String,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    required_capabilities: Vec<CapabilityKind>,
    effect_class: Option<EffectType>,
    deterministic: bool,
}

/// §87 的登记项。**含定义，不与它并列**（设计 §3.1）。
///
/// `cost` / `latency` 只定**容器形状**，取值域留空——它们服务子项目 D 的候选排序，
/// 而排序依据要到那时才存在（§250 §84）。**不预先发明度量。**
/// `trust` 非 `Option`：登记进 Registry 就必然有信任判定，没有「尚未判定」这一状态。
pub struct ToolProfile {
    tool: Tool,
    cost: Option<Cost>,
    latency: Option<Latency>,
    trust: Trust,
}
```

`Cost` 与 `Latency` **就是单位结构体**（`pub struct Cost;` / `pub struct Latency;`），**不是待填的壳**：取值域属子项目 D 的候选排序，而排序依据要到那时才存在（§250 §84）。**不预先发明度量**——这是本项目「据实写明未定」的一贯做法。`ToolProfile` 的这两个字段写成 `Option<Cost>` / `Option<Latency>`，故「尚未登记画像」与「画像为空」在类型上分得开。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-capability
git commit -m "feat(capability): Tool 与 ToolProfile"
```

---

### Task 4: `tool` 表与读写

**Files:**
- Create: `crates/continuum-capability/src/persist.rs`
- Modify: `crates/continuum-capability/src/{lib.rs,error.rs}`
- Create: `crates/continuum-capability/tests/persist.rs`
- Modify: `crates/continuum-runtime/src/main.rs`（迁移装配）
- Modify: `crates/continuum-runtime/tests/{migrations.rs,startup.rs}`（连带断言）

**Interfaces:**
- Produces: `continuum_capability::{p3_capability_migrations, save_tool, load_tool, load_tools}`

- [ ] **Step 1: 写用例**

- `tool_round_trips`：存一条，读回，逐字段相同。
- `enum_columns_use_the_lowercase_encoding`：直接查表，断言列值是小写串、多词 `_` 连接——**不是** `Debug` 表示。
- `required_capabilities_round_trip`：多个 `CapabilityKind` 存回一致的顺序与内容。
- `saving_the_same_id_twice_is_rejected`：同 id 再存返回具体 `Err`，**且原行内容不变**（裸 `INSERT`，不是 `OR REPLACE`）。
- `an_unknown_enum_column_value_is_rejected`：裸 SQL 写入表外取值，`load_tool` 返回具体 `Err`。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-capability --test persist
```

- [ ] **Step 3: 实现**

迁移（编号取未占用值，**先查再定**）：

```sql
CREATE TABLE tool (
    id                     TEXT PRIMARY KEY,
    version                TEXT NOT NULL,
    input_schema           TEXT NOT NULL,
    output_schema          TEXT NOT NULL,
    required_capabilities  TEXT NOT NULL,
    effect_class           TEXT,
    deterministic          INTEGER NOT NULL,
    cost                   TEXT,
    latency                TEXT,
    trust                  TEXT NOT NULL
);
```

签名：

```rust
pub fn p3_capability_migrations() -> Vec<Migration>;
pub fn save_tool(tx: &Tx<'_>, profile: &ToolProfile) -> Result<(), PersistError>;
pub fn load_tool(tx: &Tx<'_>, id: &ToolId) -> Result<Option<ToolProfile>, PersistError>;
pub fn load_tools(tx: &Tx<'_>) -> Result<Vec<ToolProfile>, PersistError>;
```

**编码挂在其类型上**（`CapabilityKind` 的 `as_str`/`parse`、`Trust`/`EffectType` 同理），`persist.rs` 只做委托与错误适配——同 P2 的 Task 4 裁定；`load_*` 里若有表外取值，返回具体 `Err` 而非默认值。

- [ ] **Step 4: 注册迁移并更新连带断言**

`runtime_migrations()` 加 `continuum_capability::p3_capability_migrations()`。**全仓搜「迁移应用」与迁移计数断言，逐条确认新值**；注意 `tests/startup.rs` 的 `second_startup_applies_no_migration` 断言 `0`，它与「迁移应用 N 项」**不是同一类断言**，不要改。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-capability crates/continuum-runtime
git commit -m "feat(capability): tool 表与读写"
```

---

### Task 5: 强制点 (1)：`AuthorizedTool`

**Files:**
- Create: `crates/continuum-capability/src/registry.rs`
- Modify: `crates/continuum-capability/src/{lib.rs,error.rs}`
- Create: `crates/continuum-capability/tests/authorize.rs`

**Interfaces:**
- Consumes: Task 2 的 `Capability`、Task 4 的 `load_tool`
- Produces: `continuum_capability::{authorize, AuthorizedTool}`

- [ ] **Step 1: 写用例**

- `a_required_capability_that_is_missing_is_rejected`：工具声明了 `Git.Push`，调用方给空 → 具体 `Err`。
- `a_presented_capability_that_the_tool_did_not_declare_is_rejected`：工具只声明 `Filesystem.Read`，调用方另给 `Git.Push` → 具体 `Err`（**超范围同样不许**，§252 的另一半）。
- `an_expired_capability_is_rejected`：未过期通过、**恰好到期**被拒、已过期被拒。
- `a_successful_authorization_writes_one_capability_grants_audit_row`：恰一条、`kind` 手写字面量为 `"capability grants"`、payload 含工具 id 与作用域。
- `a_rejected_authorization_writes_no_audit_row`：**方向相反的对照臂**。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-capability --test authorize
```

- [ ] **Step 3: 实现**

```rust
/// **唯一能把工具交给调用方的路径**（设计 §3.4）。
///
/// 取**自由函数**而非方法：它要的都从参数来（`Tx` 用于读登记项与写审计），
/// 没有需要挂在 `self` 上的状态——与 `approve_integration` 同形。
///
/// `AuthorizedTool` 字段私有、无公开构造函数，故拿不到它就调不了工具——
/// 「忘了校验」这一路径在**类型上**不存在（与 P2 的 `WritablePath` 同形）。
pub fn authorize(
    tx: &Tx<'_>,
    tool_id: &ToolId,
    presented: &[Capability],
    now: i64,
) -> Result<AuthorizedTool, CapabilityError>;

pub struct AuthorizedTool(/* 私有：工具 id 与它已获准的那些能力 */);
```

两个方向的拒绝各用一个错误变体（`MissingCapability { kind }` 与 `UndeclaredCapability { kind }`）——**纪律 3：失败路径要断言是哪一种**。

**审计**：成功授权写一条 `AuditKind::CapabilityGrants`（该变体自 P0 起预留、至今无产生方，在此第一次有）；**拒绝不写**，由调用方处置。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-capability
git commit -m "feat(capability): 强制点 (1)——AuthorizedTool"
```

---

### Task 6: `continuum-secrets`：密钥运行时

**Files:**
- Create: `crates/continuum-secrets/Cargo.toml`
- Create: `crates/continuum-secrets/src/{lib.rs,runtime.rs,source.rs,error.rs}`
- Create: `crates/continuum-secrets/tests/issue.rs`
- Modify: `Cargo.toml`（members）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`

**Interfaces:**
- Consumes: Task 2 的 `Capability`
- Produces: `continuum_secrets::{SecretsRuntime, Credential, SecretsError, CredentialSource}`

- [ ] **Step 1: 建 crate 并登记**

`ALLOWED` 加 `("continuum-secrets", &["continuum-capability", "continuum-core"])`。

- [ ] **Step 2: 写用例**

- `a_credential_never_exceeds_its_capability_scope`：拿一枚 `Github.CreatePr` 作用域 `repo/X` 的能力，索取 `repo/Y` 的凭据 → 具体 `Err`。**且断言「要不到」而非「被检查掉」**——即 `issue` 的签名只收能力本身，没有「指定一个别的 scope」的参数（这一条由签名表达，用例里写明）。
- `a_credential_never_outlives_its_capability`：凭据到期 **≤** 能力 `expiry`；构造一个凭据源声称更长有效期，断言运行时**截到能力的到期时刻**。
- `there_is_no_way_to_ask_for_all_secrets`：trybuild 样例——`issue_all` 一类入口不存在（§51：Agent 不直接获得所有密钥）。
- `a_rotation_event_invalidates_old_credentials`：按 §103 的四类事件之一轮换后，旧凭据失效、新凭据可用。

- [ ] **Step 3: 运行，确认失败**

```bash
cargo test -p continuum-secrets --test issue
```

- [ ] **Step 4: 实现**

```rust
impl SecretsRuntime {
    /// **唯一取得凭据的路径**（设计 §5.1）。凭据的作用域不超出所给的能力。
    ///
    /// 签名只收能力本身，故「拿窄能力去要宽凭据」**要不到**，而不是被检查掉
    /// ——调用方没有可以指定另一个 scope 的位置。
    pub fn issue(&self, cap: &Capability, now: i64) -> Result<Credential, SecretsError>;
}

/// 作用域 + 到期时刻 + 取得凭据材料的句柄。
///
/// **不含能力本身**：凭据一旦签出即与那枚能力解耦，故后续无法凭它反推或扩大权限（设计 §5.2）。
pub struct Credential { /* 私有 */ }
```

**凭据源**（`source.rs`）：文件 + 环境变量。**这是真实现、不是桩**：能签发、能按能力校验作用域、能过期、能轮换。设计 §5.3 已把与 §100（「应优先」TPM/Secure Enclave）的偏离记为据实偏离并指向长期替换点——**代码注释里也要写这句**。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-secrets Cargo.toml crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(secrets): 密钥运行时与凭据签发"
```

---

### Task 7: 强制点 (2)：执行点接进驱动

**Files:**
- Modify: `crates/continuum-capability/src/lib.rs`（连接器侧的值类型）
- Modify: `crates/continuum-runtime/src/{task_cmd.rs,Cargo.toml}`
- Create: `crates/continuum-runtime/tests/capability_gate.rs`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`

**Interfaces:**
- Consumes: Task 2 的 `Capability` 与 `mint`、Task 5 的 `AuthorizedTool`
- Produces: `continuum_capability::AuthorizedEffect`（连接器侧义务的载体）

- [ ] **Step 1: 写用例**

- `an_effect_without_a_capability_refuses_to_run_the_command`：策略裁决铸不出能力 → **命令未跑**（断言命令自身的副作用没发生），退出码非零，具体 `Err`。
- `an_effect_with_a_capability_runs_the_command`：能铸出 → 命令跑了、Journal 按既有约定落库。
- `the_capability_is_checked_before_the_command_runs`：命令在读库时能观察到自己的能力已登记（与 P2 的「写入早于执行」同法）。
- `the_integration_approval_still_binds_one_specific_integration`：**并存关系的照片**——能力只表示「这一类动作准不准」，拿这次批准去用另一次仍被拒（沿用 P2 的既有用例，不新写）。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-runtime --test capability_gate
```

- [ ] **Step 3: 实现**

强制点排在 `Sandbox::spawn` **之前**。检查对象是命令声明的那几条 `--effect`：逐条问策略（`PolicyContext.effect_type` 已有该字段），据裁决铸 `Capability`；铸不出即拒绝运行命令。**不发明新机构。**

**这与 P2 的「策略只查一次」不冲突，但必须写清楚，否则会被读成冲突**：P2 那条裁定的对象是**集成**的裁决（`--apply` 那一支，第 7 步复用第 4 步的那一次），而这里是**逐条效应**的裁决——问的是不同的问题（「这条效应准不准」vs「这次集成准不准」），上下文里填的字段也不同。两处各自查一次，互不复用。

**裁决到 `Verdict` 的映射**（`continuum-capability` 不依赖 `continuum-policy`，故在驱动侧转）：`Decision::Allow → Verdict::Allow`、`RequireApproval → Verdict::RequireApproval`、`Deny → Verdict::Deny`；给了 `--approve` 时用 `Grant::PolicyWithExplicitApproval`，否则 `Grant::Policy`。**三个臂逐条写，不抽代表**（枚举式断言逐项有照片）。

**连接器侧那半个义务由类型承载**（设计 §4.2）：

```rust
/// 只有校验路径能产出的值。连接器（子项目 B）要做副作用就必须收它。
///
/// 若 B 将来绕过，那是**类型上表达得出来的选择**（收的是别的类型），
/// 而不是「忘了接线」这种查不出来的漏——后者正是 P2b 全分支终审花一整轮才查出的那一类
/// （驱动的清理路径绕过 Gate 的 `discard`，审计行从未写下）。
pub struct AuthorizedEffect(/* 私有 */);
```

`Cargo.toml` 加 `continuum-capability` 与 `continuum-secrets` 两条边，`ALLOWED` 的 runtime 条目同步。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-capability crates/continuum-runtime
git commit -m "feat(runtime): 强制点 (2)——执行前的 Capability 校验"
```

---

### Task 8: 收尾与复核

**Files:**
- Modify: `docs/superpowers/p3a-followups.md`（新建，或并入既有的 followups 文件）

- [ ] **Step 1: 全量验证**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
timeout 900 cargo build --workspace --all-targets
```

预期：全绿、0 warning。

- [ ] **Step 2: 逐条核对完成判据**

| 判据 | 证据 |
|---|---|
| §4.4「工具调用前校验 Capability」 | Task 5 的两个方向用例 |
| §4.4「Capability 不可与裸字符串互换」 | Task 2 的 trybuild 样例 |
| 强制点 (1) | Task 5 |
| 强制点 (2) | Task 7 |
| 强制点 (3) | Task 6 |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 3: 复核 P2 留的两个组件是否都接上了**

逐项核对：`Capability 强制执行点`、`密钥运行时`——**P2 设计 §1.1 声明「不建、也不留桩」的那两项**。**仍无生产调用方的逐项列出**并说明是设计已声明还是漏接线。

- [ ] **Step 4: 残余落到有版本的文档**

`.superpowers/` 是 gitignore 的，**只写在报告或 ledger 里的结论会随 branch 消失**。把残余写进 `docs/superpowers/` 下的 followups 文件（**不新建第二个同类文件**——同一类事项两个家正是本项目反复处置的毛病）。

- [ ] **Step 5: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs: P3 子项目 A 的收尾与复核"
```

---

## 遗留

```
凭据源偏离 §100       无硬件后端，本阶段用文件 + 环境变量。§100 是 SHOULD 非 MUST，故非违规；
                     替换点在长期阶段（TPM / Secure Enclave）。
Authority Host 移交    Issuer::AuthorityHost 本阶段不产出；移交触发条件是该宿主就位。
effect_class 的取值集  规范未定义，本子项目绑到 EffectType。若后续判定是两个轴，要重新处置。
Tool 与 ToolProfile   规范未明说二者关系，本子项目判为「登记项含定义」。
连接器侧的强制点 (2)   由 AuthorizedEffect 承载，兑现是子项目 B 的义务；B 的设计须显式说明它收了那个值。
OPEN-003 / OPEN-007   均不影响本子项目（前者压子项目 D 的成本决策，后者整块阻断子项目 E）。
```
