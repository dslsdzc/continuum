# P2 边界层设计

对应《总纲》第 5 章与《工程》第 5 章。本层定义「什么绝对不能做」，以及违规时的强制手段。

**前置阶段：** P0（基础设施）、P1（执行层）。P1 的交付状态见
`docs/superpowers/p1-followups.md`。

**本子项目的完成判据：** 《工程》`§5.4` 的五条，逐条对照见第 14 节。

---

# 1. 范围

## 1.1 本子项目建什么

《工程》`§5.1` 列出本层十一个组件。其中五个的前置尚未建成，本子项目不含：

| 组件 | 前置 | 本子项目 |
|---|---|---|
| Workspace 抽象与只读强制 | 无 | 建 |
| Task Workspace | Workspace 抽象 | 建 |
| Integration Gate | Task Workspace + Base Workspace | 建 |
| Policy Engine 与优先级 | 无 | 建 |
| Audit Log | 无 | 接线（P0 已建记录格式与哈希链，本层把 `§313` 的必录点接上） |
| Effect Journal | Capability 执行点 | 建（记录与恢复；不做 Capability 强制） |
| Capability 强制执行点 | 第 4 层 Capability Token（P3） | 不建 |
| 密钥运行时 | Capability 执行点 | 不建 |
| Prompt 信任标签强制 | 第 6 层 Context Compiler（P6） | 不建 |
| Authority Host | 长期（`§5.2` 声明仅在其成为 Product Drive 对象时进入本层） | 不建 |
| Standby 与故障转移 | Authority Host | 不建 |

后五项在本子项目内不建、也不留桩。它们的强制点在后续阶段落到本层已有的接口上。

## 1.2 强制层次

`§256` 要求 `AI_WRITE(BaseWorkspace) = DENY`，且该限制「必须由文件系统或等价隔离机制强制实现，
而不是依赖 prompt 约束」（`§159`）。`§5.4` 的判据措辞是「拒绝发生在类型层**或**文件系统层」。

本子项目两层都做：

```
类型层    Base 路径构造不出可写类型，写操作在类型上不可表达
内核层    子进程在 LSM 或命名空间层面只被授予 Task Workspace 的写权限
```

两层各管一段，互不替代。类型层约束核心自身的 Rust 代码；内核层约束子进程及其所有子孙，
使核心代码出错时子进程仍写不进 Base。

**不修改用户目录的权限位。** Base Workspace 属于用户。Runtime 对其 `chmod` 既是越权也是破坏。
隔离施加于子进程，不改动用户的文件属性。

---

# 2. 与既有阶段的关系

## 2.1 复用 P0

```
Tx                    所有落库经同一事务接口
Migration             本子项目注册第 11 节的表
RecoveryRegistry      本子项目的 Effect 恢复挂为其一个阶段
RecoveryPhase
AuditRecord           本子项目的集成与效应审计复用该记录格式与哈希链
verify_chain
builtin_migrations
```

## 2.2 复用 P1

```
AdfirGraph / Node / Edge     驱动的 `--exec` 与 Task Workspace 的关联由外部给定，
                             P1 的图结构本子项目不消费
Tx::append_event             本子项目的审计与效应事件经它写入
Tx::append_audit
p1_graph_migrations          与本子项目的迁移在同一 `Db::open_with` 中装配
```

P1 的设计第 18 节列出若干「P1 定义了但没有生产调用方」的机制，接线属执行器而非本层。
本层不接手它们。

## 2.3 一处需订正的既有表述

P1 的设计与交接文档多处把「执行器」记为属于 P2（例如「P2 的 `Queued → Running` 是算子解析的
唯一合法落点」「P2 接执行器时才有消费者」）。按建造分解，P2 是边界层，不含执行器，也不接触
状态机。该表述在 P1 文档中的四处需按实际归属订正；本子项目不引入执行器。

---

# 3. 组件划分与依赖方向

```
crates/
  continuum-workspace   Workspace 抽象、Base 与 Task Workspace、
                        worktree 与 overlay 两种后端、Integration Gate
  continuum-sandbox     Sandbox 抽象、Landlock 与 bubblewrap 两种实现
  continuum-policy      Policy 表达、六级优先级、冲突裁决
  continuum-effect      Effect Journal：状态机、执行前写入、幂等键、重启恢复
```

Integration Gate 与两种 Workspace 同 crate：三者共用同一批路径类型与后端抽象，
拆开会使每次操作都要在 crate 间来回转换类型，且「Base 只经 Gate 写入」这条约束会
因为跨 crate 而无法用可见性表达。

依赖方向：

```
continuum-workspace  → continuum-core, continuum-persist, continuum-events
continuum-sandbox    → continuum-core, continuum-workspace
continuum-effect     → continuum-core, continuum-persist, continuum-events
continuum-policy     → continuum-core, continuum-persist, continuum-artifact,
                       continuum-effect
continuum-runtime    → continuum-workspace, continuum-sandbox, continuum-policy,
                       continuum-effect, 以及既有各 crate
```

`continuum-workspace → continuum-events` 由第 6.4 节要求：Gate 的审计记录经
`Tx::append_audit` 写入，该函数的 `kind` 参数类型 `AuditKind` 定义在 `continuum-events`。

`continuum-policy` 的两条既有阶段的边由第 8.5 节的 `PolicyContext` 要求：它引用
`PrivacyClass`（属 `continuum-artifact`）与 `EffectType`（属 `continuum-effect`）。
把这两个字段改成字符串可以免掉这两条边，但代价是策略条件失去类型检查，
与「条件不是自由文本」的设计目标冲突，故取边而非字符串。

`continuum-policy` 不依赖 `continuum-events`：策略变更不在 `§313` 的必录清单内
（该清单为 authority 变更、设备加入、设备撤销、capability 授予、外部副作用、
Contract 变更、用户批准、Runtime 自更新），策略本身也不产生事件。

`continuum-sandbox → continuum-workspace` 是刻意的，它是类型层保证向子进程传导的路径：
`Sandbox::spawn` 收 `&TaskWorkspace` 而非 `&Path`，因此「把 Base Workspace 交给子进程」在类型上
不可表达。若该参数是 `&Path`，这条保证退化为调用方的约定。

无环。`continuum-sandbox` 不依赖 `continuum-effect` 或 `continuum-policy`——沙箱不知道
Journal 与 Policy 的存在，三者由驱动装配。

---

# 4. 只读强制的三层结构

## 4.1 类型层

```
BaseWorkspace    持有 Base 路径，不对外暴露任何可写句柄
TaskWorkspace    持有 Task 路径，可产出 WritablePath
WritablePath     独立类型，唯一构造入口是 TaskWorkspace::writable_root()
```

本 crate 内所有写操作（建目录、写文件、改权限、执行会写盘的命令）一律收 `&WritablePath`。
该类型无公开构造函数，唯一来源是 `TaskWorkspace::writable_root()`。

该保证由两半构成，缺一即不成立：

1. **`BaseWorkspace` 不暴露任何可写句柄**——没有 `writable_root`，也没有通向 `WritablePath`
   的方法。故持有 Base 句柄的代码取不到可写路径。
2. **`TaskWorkspace` 的构造拒绝与 Base 重叠的根**——既不能等于 Base，也不能是 Base 的祖先。
   缺了这一半时，`TaskWorkspace::new_outside(&base, base.root(), id)` 会产出一个指向 Base 的
   可写根，而 `Sandbox::spawn` 收的正是 `&TaskWorkspace`，于是内核层的隔离会反过来给 Base
   开写权限——三层保证里最外一层被从内部绕过。

第 2 条允许 Task 根位于 Base **之内**（worktree 后端即此形态：Task 根在
`<base>/.ai/worktrees/<intent>`），因为此时可写范围是 Base 的一个子目录而非 Base 本身。

「Base 只经 Integration Gate 写入」由同一条机制给出：`BaseWorkspace` 不暴露可写句柄，
写 Base 的私有路径只被 Gate 的三种写入操作调用。

验证方式是 `trybuild` 的编译失败用例：`WritablePath::new(base_path)` 与
`Gate` 之外对 Base 的写入均须**编译失败**。运行期测试无法证明「不可构造」，编译失败用例是
这条保证的唯一直接证据。代价是引入一个 dev-dependency。

## 4.2 进程层

`Sandbox::spawn` 的签名决定子进程能拿到什么：

```rust
fn spawn(&self, task: &TaskWorkspace, cmd: &Command) -> Result<Child, SandboxError>
```

子进程的工作目录是 Task Workspace 根，argv 与环境变量由调用方给定且不包含 Base 路径。
本层不提供「把 Base 路径交给子进程」的接口。

## 4.3 内核层

两种机制，由 `Sandbox` 抽象统一。

**Landlock**（默认）：`landlock_restrict_self` 对当前进程生效并被子孙继承。在 `pre_exec`
中于 fork 之后、exec 之前施加规则集：默认拒绝，放行 `/usr`、`/lib`、`/etc` 等只读路径，
写权限只开 Task Workspace 及其子孙。

**bubblewrap**：以 `--ro-bind / /` 加 `--bind <task> <task>` 建立挂载命名空间，工作目录设为
Task Workspace。它额外提供网络与 PID 命名空间的隔离能力，本子项目不使用这些能力，
但保留该后端。

两处实现约束：

- **Landlock 的 ABI 版本须在运行期探测。** 内核不支持的文件系统类别要降级处理而非启动失败，
  且降级必须显式记录——否则会静默退化成「没有隔离」。
- **bubblewrap 的参数表须与 Landlock 走同一套测试。** 加固参数少写一项即等于未隔离，
  不能因为它「是现成工具」而假定配置正确。

## 4.4 对照臂

沙箱的测试必须有两臂：

```
实验臂    子进程尝试写 Base          → 断言被拒（EACCES）
对照臂    子进程在 Task 内写同一文件 → 断言成功
```

只有实验臂时，一个「拒绝一切写入」的沙箱同样能通过。

---

# 5. Workspace 抽象与两种后端

```
Workspace          公共操作集：根路径、是否存在、清理
TaskWorkspace      创建、放弃、产出 WritablePath
BaseWorkspace      只读路径；不暴露可写句柄
WorkspaceBackend   Worktree | Overlay
```

后端选择由 Base 是否为 Git 仓库决定，上层不感知差异。

**Worktree 后端**（`§255`）：在 `.ai/worktrees/<intent-id>` 下建 worktree，分支名 `ai/<intent-id>`
（`§16`）。创建、放弃、列出均经 `git worktree` 与 `git branch` 命令。Agent 在该分支内可自由
修改与提交，不触及用户的当前分支。

**Overlay 后端**：以 unprivileged user namespace 加 OverlayFS 建立可写覆盖层，
lowerdir 为 Base，upperdir 为该 Intent 的私有目录。放弃即丢弃 upper 层。

**overlay 的工作目录放在 Base 之外**，路径为 `<overlay 根>/<Base 标识>/<intent>/{upper,work,mnt}`，
其中 overlay 根由 Runtime 管理（默认 `~/.local/share/continuum/overlays`），Base 标识取其
规范路径的哈希。理由：若把 upper 放在 `<base>/.ai/` 下，则 lower 为整个 Base 时，
Intent A 的工作区能读到 `<base>/.ai/overlays/B/upper` 里 Intent B 的未提交工作——
这是跨 Intent 的读取泄漏，而规范只约束了写入。移到 Base 之外后 lower 中不再有 `.ai`。

worktree 后端无此问题：`.ai/` 被 gitignore，而 gitignore 的目录不会出现在 git worktree 中。
两个后端的工作区存储位置因此不同，此差异是本条要求的直接结果。

该后端有一条约束，由无特权 OverlayFS 的性质决定：**挂载只在其所在的用户与挂载命名空间内可见**，
命名空间退出即消失。因此该后端要求调用进程已处在一个用户与挂载命名空间内，且该命名空间须存活
至子进程用完工作区为止。驱动的处置是：需要该后端时，把自身 re-exec 进 `unshare -Urm`，
此后其子进程继承同一命名空间，挂载全程有效。

该约束须写入 `OverlayBackend` 的文档注释与驱动的启动逻辑。调用方若不在命名空间内而直接调用，
`create` 返回错误而非静默建出一个对子进程不可见的工作区。

**两种后端共用同一抽象**，选择一个后端的判据（是否 Git 仓库）在创建时确定并记录，
使后续操作不必重新探测。

---

# 6. Integration Gate

## 6.1 操作集

`§257` 的五个操作，按对用户环境的影响分级（`§15`）：

| 操作 | 语义 | 影响级 | 须批准 |
|---|---|---|---|
| `view_diff` | 比较 Task 与 Base，只读 | L0 | 否 |
| `discard` | 丢弃 Task Workspace，Base 不变 | L0 | 否 |
| `apply_patch` | 把 Task 的改动作为补丁应用到 Base | L3 | 是 |
| `cherry_pick` | 把 Task 分支上的指定提交摘到 Base 分支 | L3 | 是 |
| `merge` | 把 Task 分支并入 Base 分支 | L3 | 是 |

`§16` 要求从隔离工作空间向用户分支集成时影响域升级为 L3。

## 6.2 批准值

```rust
fn apply_patch(&self, task: &TaskWorkspace, approval: &GateApproval) -> Result<(), GateError>
```

`GateApproval` 无公开构造函数。本子项目内由驱动的显式确认参数产生；Capability 与 Authority
就位后，其产生点移交给那一处。该参数的作用是使「集成修改需要授权」有落点——缺少它时，
Gate 退化为「谁调用谁生效」，`§16` 的升级要求无处安放。

## 6.3 两种后端的操作语义

Git 后端下五个操作各自成立。Overlay 后端下 `cherry_pick` 与 `merge` 退化为同一个动作——
把 upper 层的内容合并进 Base；`view_diff` 变为逐文件比较；`discard` 为丢弃 upper 层。
公共操作集不变，后端各自实现。

## 6.4 审计

五种操作中三种写入 Base 的，以及 `discard`，各写一条审计记录（`§313` 的「用户批准」与
「外部副作用」类推）。审计记录与 Base 的变更在同一事务内提交（`§318` 的事务边界要求，
契约同 P1：写入函数返回 `Err` 后调用方必须回滚）。

## 6.5 与 Effect Journal 的分界

`apply_patch` / `cherry_pick` / `merge` 虽为 L3，但**可回滚**（Git 历史在），故不进 Effect Journal。
Journal 只覆盖无法通过 Artifact 回滚的外部操作（`§268`）——`push_branch` 才是。
「危险操作」与「不可回滚操作」不是同一个集合，该分界写在此处以免混淆。

---

# 7. Effect Journal

## 7.1 记录体

```
Effect {
    id                 EffectId
    effect_type        EffectType（封闭枚举，见 7.5）
    target             作用对象（URL / 分支名 / 收件人 / 资源标识），文本
    parameters         JSON
    authorization      不透明凭据串；本子项目只记录，不校验
    idempotency_key    幂等键，唯一
    state              EffectState
    planned_at         Unix 毫秒
    updated_at         Unix 毫秒
}
```

## 7.2 状态机

```
PLANNED     → AUTHORIZED | FAILED | ROLLED_BACK
AUTHORIZED  → EXECUTING  | FAILED | ROLLED_BACK
EXECUTING   → COMMITTED  | FAILED | UNKNOWN
UNKNOWN     → COMMITTED  | FAILED | ROLLED_BACK | UNKNOWN
COMMITTED   → （终态）
FAILED      → （终态）
ROLLED_BACK → （终态）
```

表外迁移一律拒绝。`UNKNOWN` 有出边，因为它表示「不知道外部发生了什么」，而非终态：
后续的对账或人工决策把它推到一个确定状态。

## 7.3 写入顺序

`§268` 要求在**执行之前**记录 intent、authorization、idempotency_key、planned_effect。
本子项目的落点：

```
写 PLANNED（含 authorization 与 idempotency_key）
写 AUTHORIZED
写 EXECUTING
—— 此后才向外部发出调用 ——
写 COMMITTED / FAILED
```

执行之后补记的日志救不了崩溃，而崩溃正是它要防的场景。状态变更与对应审计记录在同一事务内
提交。

## 7.4 重启恢复

`§269` 要求重启后根据 Journal 恢复状态。本子项目把该恢复挂为 P0 `RecoveryRegistry` 的一个阶段：

```
EXECUTING 的记录 → 一律转 UNKNOWN
PLANNED / AUTHORIZED 的记录 → 保持原状（尚未执行，语义确定）
COMMITTED / FAILED / ROLLED_BACK → 终态，不动
```

**不猜。** `EXECUTING` 在崩溃时既可能已发出外部调用也可能没有，转 `FAILED` 与转 `COMMITTED`
都可能是错的。转 `UNKNOWN` 并使其出现在恢复报告里，是本层唯一诚实的处置。
`UNKNOWN` 收敛到确定状态需要外部对账或人工决策，两者都不在本子项目的范围内。

## 7.5 EffectType 取封闭枚举

```
send_email / push_branch / publish / delete_remote / charge / deploy
```

理由与 P1 的 `ArtifactType` 相同：策略要按效应类型裁决（例如「`charge` 必须人工批准」），
开放类型会让策略表漏判。代价是新增效应类型为编译期可见的破坏性变更。

## 7.6 幂等键

`idempotency_key` 上有唯一约束。同键的第二次 `PLANNED` 被拒并返回既有记录，
使重试不产生第二个副作用（`§52`）。该约束在库层，不依赖调用方自觉。

## 7.7 authorization 字段的边界

本子项目只记录该字段，不做校验。校验属 Capability（P3）与 Authority（长期）的职责。
此边界须在文档中显式声明，否则会被读成「此处已强制」。

---

# 8. Policy Engine

## 8.1 规则形状

《总纲》`§104` 的五条示例归纳为同一形状：

```
Policy {
    level      优先级层（§105 的六级）
    condition  受限谓词
    decision   Allow | Deny | RequireApproval
    scope      User | Project
}
```

条件不是自由文本。谓词由一个小的、类型化的事实集合求值。

## 8.2 优先级

`§105` 的六级顺序：

```
1  System Safety                    内建，不是存储的规则
2  Explicit Current Task Contract   由语义层（P4）在运行期传入
3  User Persistent Policy           本层存储
4  Project Policy                   本层存储
5  Runtime Default                  本层内建
6  Model Suggestion                 本层不采纳
```

本层存储第 3、4、5 级，并参与六级的裁决。第 1 级内建、第 2 级由外部传入、第 6 级不采纳。
六级并非都可配置，此区分须写明。

## 8.3 裁决

所有条件成立的规则中取最高层。同层内 `Deny > RequireApproval > Allow`，取更严的一条。

**无任何规则匹配时默认 `Deny`。** 单条规则的条件无法求值（所引用的事实不在上下文中）时，
该规则不匹配；这不等于全局放行——放行必须由某条明确的 `Allow` 给出。

该设计是 fail-closed 的，因此 `Runtime Default` 层须预置已知安全操作的放行规则，
否则系统启动即全拒。

## 8.4 优先级不可学习

`§321` 禁止自动修改 security 语义。本层不提供修改层级的接口——层级是代码中的常量，
不落库、无 setter。该约束由结构给出，不由文档约定。

## 8.5 PolicyContext

```
PolicyContext {
    privacy_class   Option<PrivacyClass>
    effect_type     Option<EffectType>
    task_class      Option<String>        由驱动注入
    duration_ms     Option<u64>
    model           Option<ModelFacts>    P3 就位后填充
}
```

这是本层唯一会随其他阶段长大的类型。P3、P4 就位后新增字段。

**新增字段会让原本不匹配的规则开始匹配**——这是本层最需要留意的变更风险。
每次扩充 `PolicyContext` 都要重新审视既有规则的语义。

---

# 9. Sandbox 抽象

```rust
enum Sandbox {
    Landlock(LandlockSandbox),
    Bubblewrap(BubblewrapSandbox),
}

impl Sandbox {
    fn spawn(&self, task: &TaskWorkspace, cmd: &Command) -> Result<Child, SandboxError>;
    fn capabilities(&self) -> SandboxCapabilities;   // 实际生效的隔离项
}
```

`capabilities` 返回**实际生效**而非声称生效的隔离项，供调用方与测试断言。
Landlock 因内核 ABI 不足而降级时，该返回值如实反映降级结果。

引入 bubblewrap 的代价是一个非 Rust 的运行时依赖；其收益是网络与 PID 命名空间的隔离能力，
本子项目不使用，保留给后续阶段的网络策略与提示注入防护。

---

# 10. 驱动与装配

本层四块没有生产调用方：执行器、外部效应执行器、合并流程、语义层都尚未存在。
本子项目提供一个最小驱动把它们串成真实调用链，用于端到端验证。

```
continuum task    --base <目录> --intent <id> --exec <命令>
                  [--apply] [--approve]
                  [--effect <类型>:<目标>]...
continuum recover --db <路径>
```

`--effect` 每次出现声明一个外部操作，形式为 `类型:目标`，例如
`--effect push_branch:origin/main`。类型取第 7.5 节的六个之一，非法类型在启动时报错而非忽略。

**幂等键由 `意图 id / 类型 / 目标` 派生**，不由调用方另给。这样同一条命令重跑会得到同一个键，
而库层的唯一约束会拒掉第二条 `PLANNED` 并返回既有记录——`§52` 要求的
「重试不重复产生副作用」因此可在端到端测试里验证：跑一次、杀掉、重跑，断言只存在一条记录。

`task` 的动作序列：

```
建 Task Workspace（Git 项目走 worktree，非 Git 走 overlay）
在 Sandbox 内运行 --exec 指定的命令，工作目录为 Task Workspace
若给出 --apply：经 Integration Gate 应用回 Base（须 --approve）
过程中声明的外部操作在执行前写入 Effect Journal
```

`recover` 单跑 P0 的恢复流程，用于验证 Journal 的重启语义。

**该驱动是临时装配件，不是执行器。** 命令由外部给定，而非从 ADFIR 图推导——
本层没有执行器，也没有语义层。这一点须在设计、代码注释与计划三处写明，
以免被读成执行器的雏形。P4 与执行器就位后由真实的 Intent 驱动取代。

驱动产生的 `GateApproval` 是本子项目内该值的唯一产生点，见 6.2。

---

# 11. 持久化

本子项目注册的迁移：

```
effect             id, effect_type, target, parameters, authorization,
                   idempotency_key, state, planned_at, updated_at
                   PK (id)；唯一索引 (idempotency_key)
policy             id, level, scope, condition, decision
                   PK (id)
workspace          intent_id, backend, path, base_path, created_at
                   PK (intent_id)
```

枚举列（`effect_type`、`state`、`level`、`decision`、`backend`）一律小写、
多词以 `_` 连接，经显式辅助函数读写，**不依赖 serde**——该约定沿用 P1，
理由同：Rust 枚举的 serde 表示与落库编码是两件事，同一事实有两份写法即会分叉。

每张表的读写函数与该表的定义放在同一 crate。`continuum-runtime` 不直接对这些列写 SQL 字面量。

装备点在 `continuum-runtime` 的启动路径：

```rust
Db::open_with(path, builtin_migrations + p1_graph_migrations + p1_artifact_migrations
                    + p2_migrations)
```

---

# 12. 与既有层的接口

```
Tx                     所有落库
Migration              第 11 节的表
RecoveryRegistry       Effect 恢复作为其一个阶段
RecoveryPhase
AuditRecord            集成与效应的审计
verify_chain
EventType              本子项目不新增事件类型；复用既有九类中的
                       effect.committed 记录效应提交
```

---

# 13. 测试策略

各层用能真正证伪它的方式验证，不用同一种手段覆盖所有层：

| 目标 | 验证方式 |
|---|---|
| Base 路径不可构造出可写类型 | `trybuild` 编译失败用例 |
| Base 只经 Gate 写入 | 编译失败用例（Gate 之外无可调用的写路径） |
| Base 路径不进入子进程参数 | 检查实际 spawn 的 argv / env / cwd |
| 子进程写 Base 被内核拒绝 | 起真子进程写 Base（断言 EACCES），**加**写 Task 成功的对照臂 |
| 两种沙箱机制 | Landlock 与 bubblewrap 各跑同一套两臂用例 |
| 两种 Workspace 后端 | worktree 与 overlay 各跑创建、放弃、往返用例 |
| 五种 Gate 操作 | 真实 worktree 与 overlay 上各跑一遍 |
| Effect 执行前入 Journal | 注入「执行时崩溃」的假效应器，断言崩溃点之前已有 `EXECUTING` 记录 |
| 重启恢复 | 真起进程、真杀、真启 `recover`，断言 `EXECUTING` 转 `UNKNOWN` |
| 幂等键 | 同键二次 `PLANNED` 被拒，且不产生第二条记录 |
| Policy 六级裁决 | 构造跨层冲突断言按层裁决；同层冲突断言取更严；无匹配断言 `Deny` |
| 优先级不可改 | 无 setter；层级为常量（由类型与 API 面保证） |

**关键断言须做变异验证**：注入缺陷、确认目标断言在**正确位置**变红、再还原。
本项目已出过一次「变异红了但红在错的地方」（新增的前置判定改变了既有用例命中的分支），
故变异结果须给出失败的行号与左右值，而非仅「变红」。

---

# 14. 完成判据对照

《工程》`§5.4` 的五条：

| 判据 | 落点 |
|---|---|
| 对 Base Workspace 的写请求被拒绝，且拒绝发生在类型层或文件系统层，可独立验证 | 类型层：`trybuild` 编译失败用例。内核层：真子进程写 Base 断言 EACCES。两份证据独立 |
| Task Workspace 的修改只能经 Integration Gate 进入 Base Workspace | `BaseWorkspace` 不暴露可写句柄（编译失败用例）+ 五种操作在两种后端上的真实测试 |
| 所有不可回滚外部操作在执行前写入 Effect Journal | 崩溃注入用例断言 `EXECUTING` 记录先于外部调用 |
| Runtime 重启后能根据 Journal 恢复未完成的 Effect 状态 | 真杀真启的 `recover` 用例，断言 `EXECUTING` 转 `UNKNOWN` |
| Policy 冲突按 `§105` 六级优先级裁决 | 跨层与同层冲突用例，加无匹配默认 `Deny` |

---

# 15. 遗留风险与未决项

```
Capability 强制       本子项目不建。Effect Journal 的 authorization 字段只记录不校验，
                      故「效应已授权」在 P3 之前不成立。
密钥运行时            本子项目不建。authorization 与 Effect 的凭据在 P3 之前以不透明串存在。
Prompt 信任标签       本子项目不建。§281 的强制点在 P6 的 Context Compiler 就位后才成立。
Authority 与 Standby  长期。§292–§298 不在本子项目范围。
GateApproval 的产生点 本子项目内由驱动的显式参数产生，非授权校验。
                      P3 与 Authority 就位后移交。
PolicyContext 的扩充  新增字段会让原本不匹配的规则开始匹配。每次扩充须重审既有规则。
EffectType 集合       本子项目取六种。新增效应类型为编译期可见的破坏性变更。
UNKNOWN 的收敛        需要外部对账或人工决策，两者均不在本子项目范围内。
                       本子项目只保证它能被如实标记并出现在恢复报告里。
驱动的地位            临时装配件，非执行器。P4 与执行器就位后取代。
Landlock ABI 降级     内核不支持的文件系统类别降级处理，降级须显式记录。
                       降级后的实际隔离面由 SandboxCapabilities 如实反映。
bubblewrap 依赖       非 Rust 的运行时依赖。本子项目不使用其网络与 PID 隔离能力。
```

## 需订正的既有表述

P1 的设计与交接文档把执行器记为属于 P2，共四处（设计第 18 节两处、第 12 节一处、
交接文档一处）。按建造分解，执行器不属于 P2。该四处须订正为不指定阶段，
或指定为执行器实际所属的阶段。
