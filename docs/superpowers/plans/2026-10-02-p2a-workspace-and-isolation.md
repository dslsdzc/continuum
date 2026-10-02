# P2 边界层（上）实现计划：Workspace 与只读强制

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立 Workspace 抽象与三层只读强制，使对 Base Workspace 的写入在类型层、进程层与内核层都不可表达或被拒。

**Architecture:** 新增 `continuum-workspace`（Base/Task Workspace、`WritablePath`、worktree 与 overlay 两种后端、Integration Gate）与 `continuum-sandbox`（Landlock 与 bubblewrap 两种隔离机制）。`continuum-sandbox` 依赖 `continuum-workspace`，使沙箱的 spawn 接口只能接受 `TaskWorkspace`，Base 路径无法进入子进程参数。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-persist`（SQLite 事务与迁移）；`continuum-events`（审计链）；新增 `landlock`（crate）与外部命令 `git`、`mount`、`unshare`、`bwrap`。

**设计依据：** `docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md` 的第 3、4、5、6、9、11 节。

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。** 本计划新增的边：
  `continuum-workspace → continuum-persist, continuum-events`（不含 `continuum-core`：本层不使用其类型，
  且 `ALLOWED` 的比对是双向的，声明了就必须真实存在）；
  `continuum-sandbox → continuum-core, continuum-workspace`。
  `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALL_CRATES` 已改为从 workspace 成员派生，
  故**新增 crate 后该测试会立即失败**，必须在 `ALLOWED` 表中登记后才能通过。
- `continuum-core` 不含 I/O。
- 数据库连接对象在 `continuum-persist` 内私有，外部 crate 只能通过 `Tx` 访问数据库。
- 枚举列的落库编码一律小写、多词以 `_` 连接，经显式辅助函数读写，**不依赖 serde**。每张表的读写函数与表定义放在同一 crate。`continuum-runtime` 不直接对这些列写 SQL 字面量。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- `cargo test --workspace` 必须全绿，**0 warning**。
- 不在仓库中写入任何凭据。
- **不修改用户目录的权限位。** Base Workspace 属于用户，隔离施加于子进程，不改动用户的文件属性。

## 关于本计划的代码块

本项目的既有事实：手写的计划代码块错误率很高（P1 的计划文本出过 20+ 处事实错误），
且已确立「**代码块是示意，正文的措辞才是约束**」这一判定标准。因此本计划中：

- 类型定义、SQL、函数签名、测试断言给出完整代码；
- 直白的过程代码以正文描述给出，不逐行展开；
- 凡与既有 crate 交互的签名，实现前须先读该 crate 的源码确认，不符时以源码为准并回报。

---

# 文件结构

```
crates/continuum-workspace/
  src/lib.rs          导出面
  src/error.rs        WorkspaceError
  src/ids.rs          IntentId
  src/base.rs         BaseWorkspace
  src/task.rs         TaskWorkspace、WritablePath
  src/backend.rs      WorkspaceBackend 枚举与探测
  src/worktree.rs     Git worktree 后端
  src/overlay.rs      OverlayFS 后端（含命名空间约束）
  src/gate.rs         IntegrationGate、GateApproval、五种操作
  src/persist.rs      workspace 表的迁移与读写
  tests/type_level.rs         trybuild 编译失败用例
  tests/writable.rs           写入路径与对照臂
  tests/backend_worktree.rs   真实 worktree 往返
  tests/backend_overlay.rs    真实 overlay 往返（经命名空间）
  tests/gate.rs               五种操作
  tests/compile_fail/*.rs     trybuild 的失败样例

crates/continuum-sandbox/
  src/lib.rs          导出面
  src/error.rs        SandboxError
  src/sandbox.rs      Sandbox、SandboxCapabilities
  src/landlock.rs     Landlock 实现
  src/bubblewrap.rs   bubblewrap 实现
  tests/isolation.rs  两臂隔离用例
  tests/capabilities.rs 隔离项如实反映
```

---

### Task 1: `continuum-workspace` 骨架与类型层保证

**Files:**
- Create: `crates/continuum-workspace/Cargo.toml`
- Create: `crates/continuum-workspace/src/{lib.rs,error.rs,ids.rs,base.rs,task.rs}`
- Create: `crates/continuum-workspace/tests/type_level.rs`
- Create: `crates/continuum-workspace/tests/writable.rs`
- Create: `crates/continuum-workspace/tests/compile_fail/*.rs`
- Modify: `Cargo.toml`（workspace members 与 workspace.dependencies 加 `trybuild`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 登记新 crate）

**Interfaces:**
- Produces: `continuum_workspace::{BaseWorkspace, TaskWorkspace, WritablePath, IntentId, WorkspaceError}`

- [ ] **Step 1: 建 crate 骨架**

`Cargo.toml`：

```toml
[package]
name = "continuum-workspace"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-core = { path = "../continuum-core" }
continuum-events = { path = "../continuum-events" }
continuum-persist = { path = "../continuum-persist" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
trybuild = "1"
```

根 `Cargo.toml` 的 `members` 加 `"crates/continuum-workspace"`。

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 加：

```rust
    (
        "continuum-workspace",
        &["continuum-core", "continuum-events", "continuum-persist"],
    ),
```

**先跑一次 `cargo test -p continuum-runtime --test dependency_direction`**，确认不登记时该测试失败、
登记后通过——`ALL_CRATES` 已派生自 workspace 成员，新增 crate 会立即触发覆盖断言。

- [ ] **Step 2: 写类型层的编译失败用例**

`crates/continuum-workspace/tests/compile_fail/writable_path_from_base.rs`：

```rust
// 应编译失败：Base 路径不能构造出可写类型
fn main() {
    let base = continuum_workspace::BaseWorkspace::new("/tmp").unwrap();
    let _ = continuum_workspace::WritablePath::from(base.root());
}
```

`crates/continuum-workspace/tests/compile_fail/writable_root_on_base.rs`：

```rust
// 应编译失败：BaseWorkspace 没有 writable_root
fn main() {
    let base = continuum_workspace::BaseWorkspace::new("/tmp").unwrap();
    let _ = base.writable_root();
}
```

`crates/continuum-workspace/tests/type_level.rs`：

```rust
#[test]
fn base_path_cannot_be_made_writable() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
```

- [ ] **Step 3: 运行，确认失败**

```bash
cargo test -p continuum-workspace --test type_level
```

预期：编译失败，`BaseWorkspace` 与 `WritablePath` 不存在。

- [ ] **Step 4: 实现类型**

`src/ids.rs`：`IntentId` 为字符串新类型，派生 `Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize`，
提供 `new` / `as_str` / `Display`。与 P1 的 `ArtifactId` 同形。

`src/error.rs`：`WorkspaceError` 为 `thiserror` 枚举，中文文案。变体至少含
`MissingBase { path }`、`NotADirectory { path }`、`EscapesRoot { path }`、
`BackendUnavailable { reason }`、`NotInNamespace`、`GitFailed { code, stderr }`、
`GateRefused { reason }`。

`src/base.rs`：

```rust
/// Base Workspace：用户的工作区，永久只读。
///
/// 本类型**不提供任何可写句柄**——没有 `writable_root`，没有 `WritablePath` 的构造路径。
/// 写 Base 的私有路径仅被 `gate` 模块的三种写入操作调用。该不可构造性是 §256 在类型层的落点。
pub struct BaseWorkspace { root: PathBuf }

impl BaseWorkspace {
    /// `root` 必须存在且为目录。
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, WorkspaceError>;
    pub fn root(&self) -> &Path;
}
```

`src/task.rs`：

```rust
/// Task Workspace：本 Intent 的可写位置。
pub struct TaskWorkspace { root: PathBuf, intent_id: IntentId }

impl TaskWorkspace {
    /// 唯一的公开构造入口。三种后端与测试夹具都经它产出句柄。
    ///
    /// `root` 不得与 `base` 重叠：既不能等于 Base，也不能是 Base 的祖先。
    /// 前者使「Task 就是 Base」，后者使「Task 是包含 Base 的一层」——两者都会让
    /// `Sandbox::spawn` 把 Base 当作可写根，内核层隔离反过来给 Base 开写权限。
    ///
    /// **允许 `root` 位于 `base` 之内**（worktree 后端即此形态：Task 根在
    /// `<base>/.ai/worktrees/<intent>`），因为此时可写范围是 Base 的一个子目录，而非 Base 本身。
    pub fn new_outside(
        base: &BaseWorkspace,
        root: impl Into<PathBuf>,
        intent_id: IntentId,
    ) -> Result<Self, WorkspaceError>;
    pub fn root(&self) -> &Path;
    pub fn intent_id(&self) -> &IntentId;
    /// 本 crate 内唯一能产出 `WritablePath` 的入口。
    pub fn writable_root(&self) -> WritablePath;
}

/// 已被证明位于 Task Workspace 内的路径。
///
/// **无公开构造函数。** 唯一来源是 `TaskWorkspace::writable_root()`。
/// 本 crate 内所有写操作收 `&WritablePath` 而非 `&Path`，故「对 Base 写」不可表达。
pub struct WritablePath(PathBuf);

impl WritablePath {
    pub fn as_path(&self) -> &Path;
    /// 拼接相对路径；拒绝 `..` 与绝对路径，越出 Task 根时返回 `Err`。
    pub fn join(&self, rel: &str) -> Result<WritablePath, WorkspaceError>;
    pub fn write(&self, rel: &str, bytes: &[u8]) -> Result<(), WorkspaceError>;
    pub fn create_dir_all(&self, rel: &str) -> Result<(), WorkspaceError>;
    pub fn read(&self, rel: &str) -> Result<Vec<u8>, WorkspaceError>;
}
```

`join` 的 `..` 与绝对路径拒绝是必要的：没有它，`WritablePath` 只保证**起点**在 Task 内，
不保证**终点**在 Task 内，类型层的保证会被一个 `join("../../etc/passwd")` 绕过。

- [ ] **Step 5: 写行为用例**

`tests/writable.rs`：

```rust
#[test]
fn writable_path_writes_inside_the_task_root() {
    let (dir, base) = base_and_task_dirs();
    let task = TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1")).unwrap();
    let w = task.writable_root();
    w.write("a/b.txt", b"hello").unwrap();
    assert_eq!(w.read("a/b.txt").unwrap(), b"hello");
}

#[test]
fn writable_path_rejects_parent_traversal() {
    let (dir, base) = base_and_task_dirs();
    let task = TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1")).unwrap();
    let w = task.writable_root();
    let err = w.join("../escape").unwrap_err();
    assert!(matches!(err, WorkspaceError::EscapesRoot { .. }), "实际 {err:?}");
}

`writable_path_rejects_absolute_paths` 的断言内容：同一夹具下 `w.join("/etc/passwd")`
返回 `EscapesRoot`，错误信息里带该路径。

#[test]
fn task_root_may_not_be_the_base() {
    let (dir, base) = base_and_task_dirs();
    let err = TaskWorkspace::new_outside(&base, base.root(), IntentId::new("i1")).unwrap_err();
    assert!(matches!(err, WorkspaceError::Overlaps { .. }), "实际 {err:?}");
}

#[test]
fn task_root_may_not_be_an_ancestor_of_the_base() {
    // 以 Base 的父目录为 root：可写范围会包含 Base 本身
    let (dir, base) = base_and_task_dirs();
    let err = TaskWorkspace::new_outside(&base, dir.path(), IntentId::new("i1")).unwrap_err();
    assert!(matches!(err, WorkspaceError::Overlaps { .. }), "实际 {err:?}");
}

#[test]
fn base_workspace_rejects_a_missing_directory() {
    let err = BaseWorkspace::new("/definitely/not/here").unwrap_err();
    assert!(matches!(err, WorkspaceError::MissingBase { .. }), "实际 {err:?}");
}
```


- [ ] **Step 6: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(workspace): 类型层只读保证与 WritablePath"
```

---

### Task 2: Git worktree 后端

**Files:**
- Create: `crates/continuum-workspace/src/backend.rs`
- Create: `crates/continuum-workspace/src/worktree.rs`
- Modify: `crates/continuum-workspace/src/lib.rs`
- Create: `crates/continuum-workspace/tests/backend_worktree.rs`

**Interfaces:**
- Consumes: Task 1 的 `BaseWorkspace` / `TaskWorkspace` / `IntentId`
- Produces: `continuum_workspace::{WorkspaceBackend, detect_backend, create_task_workspace, discard_task_workspace}`

- [ ] **Step 1: 写测试**

`tests/backend_worktree.rs` 的用例：在临时目录建一个 Git 仓库（`git init` 加一次提交），
然后

```rust
#[test]
fn worktree_backend_creates_an_isolated_branch() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    assert_eq!(detect_backend(&base), WorkspaceBackend::Worktree);

    let task = create_task_workspace(&base, &IntentId::new("i1")).unwrap();

    // 分支名符合 §16 的 ai/<task-id>
    assert_eq!(branch_of(task.root()), "ai/i1");
    // worktree 位于 .ai/worktrees/ 下
    assert!(task.root().starts_with(base_path.join(".ai/worktrees")));
    // 用户的当前分支未被改动
    assert_eq!(current_branch(&base_path), "main");
}

#[test]
fn discarding_a_worktree_leaves_the_base_untouched() {
    // 在 task 内写文件并提交，discard 之后：
    // 文件不在 base 中，worktree 目录已移除，分支已删除
}

#[test]
fn worktree_writes_do_not_reach_the_base() {
    // 在 task 内写一个新文件，断言 base 下不存在该文件
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-workspace --test backend_worktree
```

预期：编译失败，`detect_backend` 等不存在。

- [ ] **Step 3: 实现**

`src/backend.rs`：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceBackend { Worktree, Overlay }

/// Base 是否为 Git 仓库决定后端。
///
/// 判据是 `<base>/.git` 存在（目录或文件——worktree 中它是文件）。不使用 `git rev-parse`，
/// 因为它会把父目录的仓库也算进来，而本函数的语义是「Base 自身是不是一个仓库」。
pub fn detect_backend(base: &BaseWorkspace) -> WorkspaceBackend;

/// 依后端创建 Task Workspace。后端在创建时确定并返回给调用方记录。
pub fn create_task_workspace(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<(TaskWorkspace, WorkspaceBackend), WorkspaceError>;

pub fn discard_task_workspace(
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<(), WorkspaceError>;
```

`src/worktree.rs`：经 `std::process::Command` 调用 `git`。创建用
`git -C <base> worktree add -b ai/<intent> <base>/.ai/worktrees/<intent>`；
放弃用 `git -C <base> worktree remove --force <path>` 加 `git -C <base> branch -D ai/<intent>`。
外部命令的失败要带 `code` 与 `stderr` 返回，不吞。

**注意**：`.ai/` 会被 git 视为未跟踪目录，故创建前须确保它已被忽略——否则用户在 Base 的
`git status` 里会看到它。本函数在创建前检查 `<base>/.git/info/exclude` 是否含 `.ai/`，
缺失时**追加**一行（先读、保留原有内容与结尾换行）。

**不要改 `<base>/.gitignore`。** 那是用户的跟踪文件，改它会让用户的工作树出现一个非用户
所做的未提交修改，与 `§256` 的 Base 只读冲突。`.git/info/exclude` 是仓库本地的忽略清单，
不进工作树、不是跟踪文件、不影响其他 clone。

排除文件的路径**须经 `git rev-parse --git-path info/exclude` 定位**，不要硬拼
`<base>/.git/info/exclude`：Base 本身可以是 linked worktree，此时 `.git` 是文件而非目录，
硬拼会以 `Not a directory` 失败。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(workspace): Git worktree 后端"
```

---

### Task 3: OverlayFS 后端

**Files:**
- Create: `crates/continuum-workspace/src/overlay.rs`
- Modify: `crates/continuum-workspace/Cargo.toml`（加 `sha2`）
- Modify: `crates/continuum-workspace/src/backend.rs`
- Modify: `crates/continuum-workspace/src/lib.rs`
- Create: `crates/continuum-workspace/tests/backend_overlay.rs`

**Interfaces:**
- Produces: `continuum_workspace::{OverlayBackend, in_user_namespace}`

**背景。** 无特权 OverlayFS 的挂载**只在其所在的用户与挂载命名空间内可见**，命名空间退出即消失。
因此本后端要求调用进程已处在一个用户与挂载命名空间内。不在时 `create` 返回
`WorkspaceError::NotInNamespace`，不静默建出一个对子进程不可见的工作区。

判定「已在命名空间内」的方式：读 `/proc/self/uid_map`，非 root 命名空间下其首行形如
`0 <真实 uid> 1`；若 `uid_map` 中映射的 uid 序列与宿主不同即视为已在用户命名空间内。
实现须同时覆盖「以 root 运行」的情形（此时无 userns 也能挂载）。

- [ ] **Step 1: 写测试**

`tests/backend_overlay.rs`。测试进程本身不在命名空间内，故用例须自行 re-exec：

```rust
/// 本用例要求进程处于用户与挂载命名空间内。不在时把自身重新执行进 `unshare -Urm`，
/// 由子进程完成断言。`CONTINUUM_OVERLAY_CHILD` 环境变量用于标识「已经进来过」，
/// 防止无 `unshare` 时无限递归。
#[test]
fn overlay_backend_round_trips() {
    if std::env::var("CONTINUUM_OVERLAY_CHILD").is_err() {
        let exe = std::env::current_exe().unwrap();
        let out = std::process::Command::new("unshare")
            .args(["-Urm", exe.to_str().unwrap(), "overlay_backend_round_trips", "--exact"])
            .env("CONTINUUM_OVERLAY_CHILD", "1")
            .output()
            .expect("unshare 无法执行");
        assert!(out.status.success(), "子进程失败:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base");
    std::fs::create_dir_all(&base_path).unwrap();
    std::fs::write(base_path.join("f.txt"), b"lower").unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();

    let task = create_task_workspace(&base, &IntentId::new("i1")).unwrap();

    // 工作目录在 Base 之外：Base 内不得出现 .ai/overlays
    assert!(!task.root().starts_with(&base_path), "overlay 工作区不得位于 Base 内");
    assert!(!base_path.join(".ai").exists(), "Base 内不得出现 .ai");

    // 写 task 中的同一文件：覆盖 lower，base 不受影响
    task.writable_root().write("f.txt", b"upper").unwrap();
    assert_eq!(std::fs::read(task.root().join("f.txt")).unwrap(), b"upper");
    assert_eq!(std::fs::read(base_path.join("f.txt")).unwrap(), b"lower");

    // 读一个只在 base 里的文件：overlay 透传 lower
    assert_eq!(std::fs::read(task.root().join("f.txt")).unwrap(), b"upper");
}
```

`overlay_does_not_leak_a_sibling_intent` 的断言内容：在同一 Base 上为 Intent `i1` 与 `i2`
各建一个 overlay 工作区，在 `i2` 的工作区内写一个文件，然后断言 `i1` 的工作区里
既看不到该文件、也看不到 `.ai/` 目录。这条用例是「overlay 目录移出 Base」的直接守卫。

```rust
#[test]
fn overlay_create_outside_a_namespace_reports_it() {
    // 不在命名空间且非 root 时，create 返回 NotInNamespace 而不是静默建出不可见的工作区
    if in_user_namespace() { return; }
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let err = create_task_workspace(&base, &IntentId::new("i1")).unwrap_err();
    assert!(matches!(err, WorkspaceError::NotInNamespace), "实际 {err:?}");
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-workspace --test backend_overlay
```

- [ ] **Step 3: 实现**

`src/overlay.rs`：

```rust
/// OverlayFS 后端。
///
/// **命名空间约束**：无特权 OverlayFS 的挂载只在其所在的用户与挂载命名空间内可见，
/// 命名空间退出即消失。故本后端要求调用进程已处在一个用户与挂载命名空间内，
/// 且该命名空间须存活至子进程用完工作区为止。驱动在需要时把自身 re-exec 进 `unshare -Urm`。
pub struct OverlayBackend;

pub fn in_user_namespace() -> bool;
```

`create` 的动作：建 `<overlay 根>/<Base 标识>/<intent>/{upper,work,mnt}`，以
`mount -t overlay overlay -o lowerdir=<base>,upperdir=…,workdir=… <mnt>` 挂载，
返回以 `<mnt>` 为根的 `TaskWorkspace`。`discard` 先 `umount <mnt>` 再删目录。

**工作目录必须在 Base 之外。** 若放在 `<base>/.ai/` 下，lower 为整个 Base 时，
Intent A 的工作区能读到 `<base>/.ai/overlays/B/upper` 里 Intent B 的未提交工作——
这是跨 Intent 的读取泄漏，而规范只约束了写入。故 overlay 根默认取
`~/.local/share/continuum/overlays`（可由环境变量 `CONTINUUM_OVERLAY_ROOT` 覆盖，测试用），
Base 标识取其规范路径的 sha256 前 16 位十六进制。

该哈希需要 `sha2`，故 `continuum-workspace` 的 `[dependencies]` 加 `sha2 = { workspace = true }`，
并在 `ALLOWED` 之外无需改动——`sha2` 是外部 crate，不在依赖方向的检查范围内。

worktree 后端无此问题（`.ai/` 被 gitignore，不进 worktree），故两端的存储位置不同，
这是本条要求的直接结果，须写入 `OverlayBackend` 的文档注释。

`umount` 与 `mount` 同样经外部命令。两次命令的失败都须带 stderr 返回。

**注意**：`lowerdir` 是整个 Base 根，因此 Base 内的 `.ai/` 也会出现在 overlay 的 lower 中。
这在功能上无害（upper 层的改动仍隔离），但会让 Task 内看到自己的元数据目录。
处置：把 `.ai` 从 lower 中排除——OverlayFS 支持在 `lowerdir` 中以 `:` 分隔多层，
但无「排除」语法。可行的做法是 lower 用 Base，并在 Task 根下建一个 `.ai` 空目录占位；
若要真正排除，需改用多层 lower（父目录 + 逐项）。本 task 取「不做排除」并在
`OverlayBackend` 的文档注释中写明该可见性，以及它与 worktree 后端的差异。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(workspace): OverlayFS 后端与命名空间约束"
```

---

### Task 4: workspace 落库与后端记录

**Files:**
- Create: `crates/continuum-workspace/src/persist.rs`
- Modify: `crates/continuum-workspace/src/lib.rs`
- Modify: `crates/continuum-runtime/src/main.rs`（装配新迁移）
- Create: `crates/continuum-workspace/tests/persist.rs`

**Interfaces:**
- Produces: `continuum_workspace::{p2_workspace_migrations, save_workspace, load_workspace, WorkspaceRecord}`

- [ ] **Step 1: 写测试**

```rust
#[test]
fn workspace_record_round_trips() {
    // save_workspace 后 load_workspace 取回同样的 backend / path / base_path
}

#[test]
fn backend_column_uses_the_lowercase_encoding() {
    // 直接查表：backend 列的值是 "worktree" / "overlay"，不是 Rust 枚举的 Debug 表示
}
```

第二条用例是必要的：B4 起本项目已两次因「同一事实两份编码」出缺陷，
本 crate 是本计划里唯一新写枚举列的处所。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-workspace --test persist
```

- [ ] **Step 3: 实现**

迁移：

```rust
pub fn p2_workspace_migrations() -> Vec<Migration> {
    vec![Migration::new(20, "p2_workspace", "CREATE TABLE workspace (
        intent_id  TEXT PRIMARY KEY,
        backend    TEXT NOT NULL,
        path       TEXT NOT NULL,
        base_path  TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );")]
}
```

迁移编号**须先查 P1 已占用的编号**（`continuum_artifact::p1_artifact_migrations()` 与
`continuum_graph::p1_graph_migrations()` 各自的 `Migration::new` 首参），取一个未占用的值。
本计划写 20 是示意；编号冲突会使两条迁移互相覆盖。`Migration::new` 的签名以
`continuum-persist` 的实际定义为准。

`save_workspace(tx, &WorkspaceRecord)` / `load_workspace(tx, &IntentId) -> Result<Option<WorkspaceRecord>, PersistError>`。
编码经显式辅助函数 `backend_str` / `parse_backend`，与 `state_str` / `parse_state` 同形，
放在 `persist.rs` 内与表定义同址。

`continuum-runtime/src/main.rs` 的迁移装配加 `continuum_workspace::p2_workspace_migrations()`。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(workspace): workspace 表与后端记录"
```

---

### Task 5: `continuum-sandbox` 骨架与 Sandbox 抽象

**Files:**
- Create: `crates/continuum-sandbox/Cargo.toml`
- Create: `crates/continuum-sandbox/src/{lib.rs,error.rs,sandbox.rs}`
- Modify: `Cargo.toml`（members；`landlock` 进 workspace.dependencies）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（登记）

**Interfaces:**
- Produces: `continuum_sandbox::{Sandbox, SandboxCapabilities, SandboxError}`

- [ ] **Step 1: 建骨架**

`Cargo.toml`：

```toml
[package]
name = "continuum-sandbox"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-core = { path = "../continuum-core" }
continuum-workspace = { path = "../continuum-workspace" }
thiserror = { workspace = true }
landlock = "0.4"

[dev-dependencies]
tempfile = { workspace = true }
```

根 `Cargo.toml` 的 `workspace.dependencies` 加 `landlock = "0.4"`，`members` 加该 crate。
`ALLOWED` 加：

```rust
    (
        "continuum-sandbox",
        &["continuum-core", "continuum-workspace"],
    ),
```

- [ ] **Step 2: 实现抽象**

`src/sandbox.rs`：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxCapabilities {
    /// 文件系统写入被限制在 Task Workspace 内。本子项目的两种机制都应为真。
    pub restricts_filesystem_writes: bool,
    /// 网络命名空间隔离。bubblewrap 可为真，Landlock 恒为假。
    pub isolates_network: bool,
    /// PID 命名空间隔离。同上。
    pub isolates_pid: bool,
    /// 实际生效的机制名，用于报告与断言。
    pub mechanism: &'static str,
}

pub enum Sandbox {
    Landlock(LandlockSandbox),
    Bubblewrap(BubblewrapSandbox),
}

impl Sandbox {
    pub fn landlock() -> Self;
    pub fn bubblewrap() -> Self;

    /// 在 Task Workspace 内启动命令。
    ///
    /// **签名是刻意的**：参数是 `&TaskWorkspace` 而非 `&Path`，
    /// 因此「把 Base Workspace 交给子进程」在类型上不可表达。
    pub fn spawn(&self, task: &TaskWorkspace, cmd: &mut std::process::Command) -> Result<std::process::Child, SandboxError>;

    /// 实际生效的隔离项。因内核 ABI 不足而降级时，该返回值如实反映降级结果。
    pub fn capabilities(&self) -> SandboxCapabilities;
}
```

- [ ] **Step 3: 运行并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(sandbox): Sandbox 抽象与能力报告"
```

---

### Task 6: Landlock 实现

**Files:**
- Create: `crates/continuum-sandbox/src/landlock.rs`
- Modify: `crates/continuum-sandbox/src/lib.rs`
- Create: `crates/continuum-sandbox/tests/isolation.rs`

- [ ] **Step 1: 写两臂测试**

`tests/isolation.rs`：

```rust
/// 实验臂：子进程写 Base 必须被拒。对照臂：写 Task 内必须成功。
/// 只有实验臂时，一个「拒绝一切写入」的沙箱同样能通过，故两臂必须成对。
#[test]
fn landlock_denies_writes_to_base_and_allows_writes_to_task() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base");
    std::fs::create_dir_all(&base_path).unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let task = TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1")).unwrap();
    std::fs::create_dir_all(task.root()).unwrap();

    let sandbox = Sandbox::landlock();

    // 对照臂
    let status = spawn_sh(&sandbox, &task, &format!("echo ok > {}/in_task.txt", task.root().display()));
    assert!(status.success(), "对照臂：Task 内写入应成功");

    // 实验臂
    let status = spawn_sh(&sandbox, &task, &format!("echo bad > {}/in_base.txt", base_path.display()));
    assert!(!status.success(), "实验臂：Base 写入应被拒");
    assert!(!base_path.join("in_base.txt").exists(), "Base 中不得出现该文件");
}

`landlock_confines_descendants_too` 的断言内容：子进程再 fork 一个孙进程去写 Base，
同样被拒——隔离随进程继承。

**第三条守卫（必做）：符号链接逃逸。** 类型层只做路径分量检查（设计 §4.1），检出不了
「Task 根内有一个指向根外的符号链接」，故该逃逸由本层承担，须有对应用例：

```rust
#[test]
fn writing_through_a_symlink_inside_the_task_is_denied() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base");
    std::fs::create_dir_all(&base_path).unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let task = TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1")).unwrap();

    // 在 Task 根内建一个指向 Base 的符号链接，再经它写文件
    let link = task.root().join("link");
    std::os::unix::fs::symlink(&base_path, &link).unwrap();

    let sandbox = Sandbox::landlock();
    let status = spawn_sh(&sandbox, &task, &format!("echo bad > {}/evil.txt", link.display()));
    assert!(!status.success(), "经 Task 内符号链接写 Base 应被拒");
    assert!(!base_path.join("evil.txt").exists(), "Base 中不得出现该文件");
}
```

Task 7 里须对 bubblewrap 再跑一遍。**若某个沙箱机制挡不住它，不得跳过或放宽断言**——
据实报告，由我裁定是换机制还是把该缺口写进设计的遗留。
```

`spawn_sh` 是本文件内的辅助函数：用 `Sandbox::spawn` 起 `sh -c <脚本>`，`wait` 后返回 `ExitStatus`。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-sandbox --test isolation
```

- [ ] **Step 3: 实现**

规则集在 `pre_exec` 中施加：`pre_exec` 的闭包在 fork 之后、exec 之前运行，
在其中调用 `landlock` crate 的接口建立规则集（默认拒绝，放行只读路径，写权限开 Task 根），
再 `restrict_self`。

要放行的只读路径至少含 `/usr`、`/lib`、`/lib64`、`/etc`、`/bin`、`/sbin`，以及 `/dev/null`
等常见设备。不足时子进程会因无法加载动态库而失败，表现为对照臂也失败。

`pre_exec` 是 `unsafe`。闭包内不得分配内存（fork 后 exec 前只允许 async-signal-safe 操作）——
`landlock` crate 的接口若涉及分配，须在 `pre_exec` 之前把规则集构造好，`pre_exec` 内只做
`restrict_self`。

**ABI 降级**：`landlock` crate 会在内核 ABI 低于所需版本时报错。实现须捕获该情形，
降级为「不施加该文件系统类别」并**记录**，`capabilities()` 如实反映。
降级的判定与报告须有用例覆盖（可由环境变量强制模拟 ABI 不足，见 Task 7 的做法）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(sandbox): Landlock 隔离"
```

---

### Task 7: bubblewrap 实现

**Files:**
- Create: `crates/continuum-sandbox/src/bubblewrap.rs`
- Modify: `crates/continuum-sandbox/src/lib.rs`
- Modify: `crates/continuum-sandbox/tests/isolation.rs`

- [ ] **Step 1: 写测试**

把 Task 6 的两条用例按机制参数化，对 `Sandbox::landlock()` 与 `Sandbox::bubblewrap()` 各跑一遍。
`bwrap` 不存在时跳过并**显式标记跳过**（不得静默通过）。

另加：

```rust
#[test]
fn bubblewrap_reports_the_stronger_capabilities() {
    // bwrap 的 capabilities 中 isolates_network 与 isolates_pid 为真；Landlock 为假
}

#[test]
fn bubblewrap_refuses_to_start_when_the_binary_is_missing() {
    // PATH 中无 bwrap 时 spawn 返回 SandboxError，不静默退化
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-sandbox --test isolation
```

- [ ] **Step 3: 实现**

`spawn` 构造 bwrap 参数：`--ro-bind / /` 加 `--bind <task> <task>` 加 `--chdir <task>`，
加 `--dev /dev` 与 `--proc /proc`。原命令作为 bwrap 的参数追加。

**加固参数少写一项即等于未隔离**，故本实现必须与 Landlock 走同一套两臂用例——
不能因为它「是现成工具」而假定配置正确。

`bwrap` 的参数构造应集中在一个纯函数里（输入 `&TaskWorkspace` 与原命令，输出参数向量），
以便单独断言参数表本身，而不必每次都真起进程。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(sandbox): bubblewrap 隔离"
```

---

### Task 8: Integration Gate 的只读操作

**Files:**
- Create: `crates/continuum-workspace/src/gate.rs`
- Modify: `crates/continuum-workspace/src/lib.rs`
- Create: `crates/continuum-workspace/tests/gate.rs`

**Interfaces:**
- Produces: `continuum_workspace::{IntegrationGate, GateApproval, GateError, Diff}`

- [ ] **Step 1: 写测试**

```rust
#[test]
fn view_diff_reports_the_task_changes() {
    // 在 task 内改一个文件、删一个文件、加一个文件，断言 Diff 三类都列出
}

另两条用例（以下为断言内容，函数体由实现者写出）：

- `view_diff_does_not_modify_the_base`：调用 `view_diff` 前后各取一次 Base 目录的递归快照
  （路径加内容哈希的集合），断言两次相等。
- `discard_removes_the_task_and_leaves_the_base_unchanged`：在 Task 内写文件后调用 `discard`，
  断言 Task 根目录已不存在、且 Base 下不出现该文件。Git 后端下另断言分支 `ai/<intent>` 已删除。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-workspace --test gate
```

- [ ] **Step 3: 实现**

`IntegrationGate` 持有 `&BaseWorkspace`。`Diff` 为三类改动的集合（新增 / 修改 / 删除），
每项含相对路径。

只读操作不收 `GateApproval`。**写入操作（Task 9）一律收 `&GateApproval`**，其类型在本 task 内
先定义但无公开构造函数。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(workspace): Integration Gate 的只读操作"
```

---

### Task 9: Integration Gate 的写入操作与批准

**Files:**
- Modify: `crates/continuum-workspace/src/gate.rs`
- Modify: `crates/continuum-workspace/tests/gate.rs`

- [ ] **Step 1: 写测试**

```rust
#[test]
fn apply_patch_requires_an_approval_value_to_exist() {
    // GateApproval 无公开构造函数：本用例以「编译失败样例」覆盖，
    // 见 tests/compile_fail/gate_approval_construction.rs
}

#[test]
fn apply_patch_integrates_the_task_changes_into_the_base() {
    // 改一个文件后 apply，断言 base 中出现该改动，且 task 仍在
}

#[test]
fn cherry_pick_brings_only_the_named_commits() {
    // task 上两个提交，只摘第一个，断言 base 上只有第一个的改动
}

另一条用例的断言内容：`merge_brings_the_whole_task_branch`——Task 上有两个提交，
`merge` 之后断言 Base 的当前分支包含这两个提交的改动。

```rust

#[test]
fn gate_operations_write_an_audit_record() {
    // 应用前后 audit_log 行数加一，且 kind 与操作对应
}
```

编译失败样例 `tests/compile_fail/gate_approval_construction.rs`：

```rust
// 应编译失败：GateApproval 无公开构造函数
fn main() {
    let _ = continuum_workspace::GateApproval::new();
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-workspace --test gate
cargo test -p continuum-workspace --test type_level
```

- [ ] **Step 3: 实现**

```rust
/// 集成修改的批准值。
///
/// **无公开构造函数。** 本子项目内由驱动的显式确认参数产生；Capability 与 Authority
/// 就位后其产生点移交给那一处。该参数使「集成修改需要授权」有落点（§16）。
pub struct GateApproval(());
```

三种写入操作分别实现：Git 后端用 `git -C <base> apply` / `cherry-pick` / `merge`；
Overlay 后端把 upper 层的内容合并进 Base。

审计经 `Tx::append_audit(kind, occurred_at, payload)`。`kind` 取 `AuditKind` 的哪个变体，
以 `continuum-events/src/audit.rs` 的实际定义为准——若既有变体不覆盖「集成」这一语义，
**报回来再决定**是新增变体还是复用，不要自行扩枚举。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(workspace): Integration Gate 的写入操作与批准"
```

---

### Task 10: 装配、依赖方向与收尾

**Files:**
- Modify: `crates/continuum-runtime/src/main.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`
- Modify: `docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md`（若实现暴露设计问题）

- [ ] **Step 1: 装配**

`continuum-runtime` 加对 `continuum-workspace` 与 `continuum-sandbox` 的依赖，
`ALLOWED` 的 `continuum-runtime` 条目补这两个名字。

**注意**：`ALLOWED` 中的 runtime 条目现在列六个 crate，加上两个即八个。
`ALL_CRATES` 的覆盖断言会检查每个 workspace 成员都出现在 `ALLOWED` 里作为**主体**，
而非检查该数组的完整性——故漏加某个名字只会让那条依赖方向的断言失效，
不会让测试失败。**须手工核对这一条**。

- [ ] **Step 2: 全量验证**

```bash
cargo test --workspace
cargo build --workspace --all-targets
```

预期：全绿，0 warning。

- [ ] **Step 3: 逐条核对本计划的完成判据**

对照设计第 14 节的前两条：

| 判据 | 证据 |
|---|---|
| 对 Base 的写请求被拒绝，且发生在类型层或文件系统层，可独立验证 | `tests/type_level.rs` 的编译失败用例；`tests/isolation.rs` 的两臂用例 |
| Task 的修改只能经 Integration Gate 进入 Base | `tests/gate.rs` 的五种操作；`tests/compile_fail/` 中 Gate 之外无可调用写路径的样例 |

**若某条判据找不到对应证据，不得在计划里标注为覆盖**，据实报告缺口。

- [ ] **Step 4: 订正 P1 文档中「执行器属于 P2」的表述**

设计第 2.3 节记录了这处需订正：P1 的设计与交接文档把「执行器」记为属于 P2，
而按建造分解 P2 是边界层、不含执行器。共四处：P1 设计第 18 节两处、第 12 节一处、
`docs/superpowers/p1-followups.md` 一处。

订正方式是**把阶段归属去掉**，改为不指定阶段（「执行器就位后」这类措辞），
因为执行器实际所属的阶段尚未确定，指定一个同样可能是错的。
逐处改动后核对 `grep -n 'P2' ` 在两个文件中的剩余命中，确认剩下的都是正确用法
（P1 设计第 2 处提到 Effect Journal 属 P2、Capability 强制点属 P2，这两处是对的）。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(runtime): 装配 P2 上篇的 crate 与迁移；订正 P1 文档的执行器归属"
```

---

## 遗留

```
`.ai/` 在 overlay 的 lower 中可见   见 Task 3，取「不排除」并写明差异。
Overlay 后端的命名空间约束          要求调用进程已在用户与挂载命名空间内；
                                    驱动的 re-exec 在下篇实现。
Landlock 的只读路径白名单           本计划给出最小集合，实际以后端算子所需为准，
                                    不足时表现为对照臂也失败。
GateApproval 的产生点               本子项目内由驱动产生，下篇实现。
Capability 强制与密钥运行时         不在本计划范围，见设计第 15 节。
```
