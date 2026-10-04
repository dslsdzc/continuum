# P2 边界层（下篇）设计：Policy Engine、Effect Journal 与驱动

对应《总纲》第 5 章与《工程》第 5 章。

**前置阶段：** P0（基础设施）、P1（执行层）、P2 上篇（Workspace 抽象与只读强制、Task Workspace、
Integration Gate、Audit Log 接线、Sandbox——见
`docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md`）。

**上篇的交付状态与交接：** `docs/superpowers/p2-followups.md`。

**本子项目的完成判据：** 《工程》`§5.4` 五条中，上篇覆盖前两条，本子项目补后三条，逐条对照见第 10 节。

---

# 1. 范围

## 1.1 本子项目建什么

上篇已建《工程》`§5.1` 十一项中的五项（Workspace 抽象与只读强制、Task Workspace、
Integration Gate、Audit Log 接线，以及设计 §4.3 要求的 Sandbox）。本子项目建其余两项**并**给出
第十节的驱动——驱动正是这两项的消费者，这是本子项目与上篇在结构上的差别：上篇交付的机制
没有生产调用方（设计 §10 把驱动明确交给本子项目），本子项目若把机制与驱动分成两份计划，
前一份交付的仍是两个无人调用的 crate。

```
Policy Engine 与优先级   §104 §105 §320 §321
Effect Journal           §267 §268 §269 §52
驱动与装配               设计 §10；上篇十一个没有生产调用方的 pub 项
```

## 1.2 驱动是临时装配件，不是执行器

`§10` 的驱动**不是**执行器：它运行调用方给定的命令，不消费 ADFIR 图，也不决定图该怎么走。
本子项目没有执行器，也没有语义层（P4），故不存在「从 Intent 推导该做什么」的路径。

这一点须在设计、代码注释与计划三处写明，以免被读成执行器的雏形。P4 与执行器就位后，
由真实的 Intent 驱动取代它。

---

# 2. 与既有阶段的关系

## 2.1 复用上篇

```
BaseWorkspace / TaskWorkspace / WritablePath   工作区的类型层保证
detect_backend / create_task_workspace          后端判定与创建
discard_task_workspace                          放弃（不收 Tx）
WorkspaceRecord / save_workspace / load_workspace / remove_workspace
IntegrationGate / view_diff / apply_patch / cherry_pick / merge / discard
GateApproval / GateError
Sandbox / SandboxCapabilities / SandboxError    内核层隔离
OverlayBackend / in_user_namespace              命名空间判定
WorkspaceError
```

## 2.2 复用 P0 与 P1

```
Db / Tx / Migration / builtin_migrations        本子项目注册自己的迁移
run_recovery / RecoveryRegistry / RecoveryPhase Effect 恢复挂其一个阶段
Tx::append_event / append_audit
AuditKind                                       本子项目不再新增变体
```

## 2.3 上篇交接的三条

本子项目的设计直接落在上篇留下的三处交接上：

1. **`backend` 的来源**：上篇的三层封堵挡住的是「拿着错的 backend 调进来」，挡不住「上层记错了
   后端」。故本子项目的驱动**一律从落库记录取 `backend`，不从命令行取**，且取用点唯一。
2. **`discard` 与 `remove_workspace` 的次序**：本层不收 `Tx`，两者的一致性由调用方促成。
   驱动就是那个调用方——**先 `discard` 成功，再删记录**；失败则不删。
3. **`GateApproval` 是不绑定的 token**：本子项目把它改成绑定到具体一次集成，见第 7 节。

---

# 3. 组件划分与依赖方向

```
crates/
  continuum-effect    Effect Journal：记录体、状态机、执行前写入、幂等键、重启恢复
  continuum-policy    Policy 表达、六级裁决、冲突裁决、PolicyContext
```

驱动放在既有的 `continuum-runtime`——它已是装配点（迁移装配与恢复钩子都在那里）。

依赖方向：

```
continuum-effect   → continuum-persist, continuum-events
continuum-policy   → continuum-core, continuum-persist, continuum-artifact, continuum-effect
continuum-runtime  → 全部
```

`continuum-effect` 不依赖 `continuum-core`：该 crate 不使用其类型。叶子 crate 的 `ALLOWED`
条目记的是**实际依赖**（与 `continuum-runtime` 那条记「规范允许集合」不同，见
`docs/superpowers/p2-followups.md` 第四节），故无使用点的边即假边。

`continuum-policy` 的两条既有阶段的边由第 5.6 节的 `PolicyContext` 要求：它装 `PrivacyClass`
（属 `continuum-artifact`）与 `EffectType`（属 `continuum-effect`）。把这两个字段改成字符串可以
免掉这两条边，但那样策略条件失去类型检查，与第 5.1 节「条件不是自由文本」冲突。

无环。`continuum-policy` 不依赖 `continuum-workspace`——策略裁决不知道 Gate 的存在，
两者由驱动组合。

---

# 4. 驱动

## 4.1 形态

```
continuum task    --base <目录> --intent <id> --db <路径> --exec <命令>
                  [--apply] [--approve]
                  [--sandbox <机制>]
                  [--effect <类型>:<目标>]...
continuum recover --db <路径>
```

`task` 也须给出 `--db`，且是必填项：第 4.2 节第 3 步要 `save_workspace` 落库、
第 7 步要按落库记录判后端，库路径无从推得。本行由项目所有者裁定补齐（原用法行漏了它）。

## 4.2 `task` 的序列

```
1  判后端（detect_backend）
2  若为 overlay 且不在用户命名空间内 → 把自身 re-exec 进 `unshare -Urm`，重新从第 1 步开始
3  create_task_workspace，随即 save_workspace 落库
4  对每个 --effect：写 PLANNED → AUTHORIZED → EXECUTING（第 6.3 节）
   —— 若幂等键已存在记录，在此拒绝整条命令 ——
5  在 Sandbox 内运行 --exec，工作目录为 Task 根
6  命令退出码 0 → 各 Effect 写 COMMITTED；非 0 → 写 FAILED
7  若给 --apply：查策略 → 按裁决铸造或不铸造批准值 → 经 Gate 应用
8  若未给 --apply：discard_task_workspace 成功后再 remove_workspace
```

**第 7 步裁决为「不铸造」时（含第 1 级 Deny）：拒绝集成，但保留工作区**——不 `discard`。
用户的改动仍在 Task 里，给出 `--approve` 后可重跑。丢弃一份未被批准的改动会让用户
无从恢复，而保留它的代价只是一个目录。

`--apply` 与「集成被拒」是两件事：前者是请求，后者是结果。请求了但被拒，工作区不清理。

第 2 步的判定**必须排在创建任何东西之前**——否则会建出一个子进程看不见的工作区
（设计上篇 §5）。re-exec 用环境变量标识「已经进来过」，防止递归。

第 8 步的次序由上篇的裁定确定：先 `discard` 成功，再删记录；`discard` 失败则不删。
两个方向的不一致（有记录无工作区、有工作区无记录）都不会在当期报警，故次序由驱动固定。

## 4.3 沙箱机制的选择

驱动**按能力自动选**：优先 Landlock；若其 ABI 探测给出「机制不可用」（上篇 §4.3 的
`ABI < 1` 那一路），退到 bubblewrap；两者都不可用则**拒绝运行**。

`--sandbox <机制>` 允许显式指定，用于测试与诊断。这样上篇的 `capabilities()` 与那条
fail-closed 判定第一次有了真实调用方。

**显式指定与自动选择是两回事**：显式给出的那个机制在本机不可用时**返回 `Err`**，
不静默改用另一个——「显式指定」的全部意义就是不再自动挑，悄悄换掉会让调用方以为
自己在测 bubblewrap 而实际测的是 Landlock。自动选择（未给 `--sandbox`）才按上面的
次序回退。

## 4.4 后端一律从落库记录取

第 4.2 步之后的一切操作（第 5 步之后的 `discard`、第 7 步的 Gate 调用）所用的 `backend`
**一律取自 `load_workspace` 读回的记录**，不从命令行参数或请求里现取。

理由见第 2.3 节第 1 条：上篇的三层封堵挡的是「拿着错的 backend 调进来」，挡不住
「上层记错了后端、又照错的记法去回收别的资源」。取用点唯一是这条要求的前提。

## 4.5 驱动的地位

临时装配件，非执行器（第 1.2 节）。它的命令由外部给定，不从图推导。

---

# 5. Policy Engine

## 5.1 规则形状

```
Policy {
    level      优先级层（§105 的六级）
    condition  受限谓词
    decision   Allow | Deny | RequireApproval
    scope      User | Project
}
```

条件不是自由文本。谓词由一个小的、类型化的事实集合求值。

**条件的表示**：一个受限的 JSON 谓词，形式为「事实名 / 比较 / 取值」的合取，
例如 `{"all": [{"fact": "effect_type", "eq": "charge"}, {"fact": "privacy_class", "eq": "personal"}]}`。

- **事实名必须命中 `PolicyContext` 的字段**，`比较` 取一个封闭的集合（`eq` / `in` / `gte`），
  取值须与该字段的类型相符。**任何一项不符即拒绝该规则**，不是「不匹配」——
  一条写错的条件若静默退化为「永不匹配」，会让人以为策略生效了而实际没有。
- **空合取即恒真**（`{"all": []}`），用于表达「这一级对所有情形都适用」。
- 不用通用表达式引擎，也不用自由文本。理由同 `§321`：优先级与条件都是 security 语义，
  不接受可被「学习」或「配置出错却不报警」的形式。

## 5.2 六级与下篇的来源

```
1  System Safety                    内建常量，不可配置
2  Explicit Current Task Contract   下篇无 P4；**由 --approve 占这一级**（第 5.5 节）
3  User Persistent Policy           本层存储
4  Project Policy                   本层存储
5  Runtime Default                  本层内建
6  Model Suggestion                 不采纳
```

本层存储第 3、4 级，内建第 1、5 级，不采纳第 6 级。第 2 级在下篇由驱动的显式确认占位，
P4 就位后移交。

## 5.3 裁决

所有条件成立的规则中取最高层。同层内 `Deny > RequireApproval > Allow`，取更严的一条。

**无任何规则匹配时默认 `Deny`。** 单条规则的条件无法求值（所引用的事实不在上下文中）时，
该规则不匹配；这不等于全局放行——放行必须由某条明确的 `Allow` 给出。

该设计是 fail-closed 的，故第 5 级 `Runtime Default` 须预置已知安全操作的放行规则，
否则系统启动即全拒。

## 5.4 优先级不可学习

`§321` 禁止自动修改 security 语义。本层不提供修改层级的接口——层级是代码中的常量，
不落库、无 setter。该约束由结构给出，不由文档约定。

## 5.5 `--approve` 占第 2 级，及其后果

`--approve` 是驱动的显式确认参数，在裁决中占**第 2 级**（Explicit Current Task Contract）。
按 `§105` 的顺序，第 2 级高于第 3–5 级，故它的后果是：

```
第 1 级 System Safety 为 Deny        → 一律拒绝，--approve 无效
第 3–5 级为 Deny 或 RequireApproval  → 给 --approve 即放行
无规则匹配（默认 Deny）               → 给 --approve 即放行
```

**即：一个命令行开关可以推翻用户自己定的持久策略，只有内建的 System Safety 越不过。**
这是按 `§105` 的字面顺序推出的，是刻意的取舍：`§105` 把「显式当前指令」置于「用户持久策略」
之上，本子项目照此执行。若要 Deny 更硬，须改变 `--approve` 的定级，本节与第 5.7 节随之重写。

## 5.6 PolicyContext

```
PolicyContext {
    explicit_current  Option<ExplicitApproval>   --approve 是否给出
    privacy_class     Option<PrivacyClass>
    effect_type       Option<EffectType>
    task_class        Option<String>             由驱动注入
    duration_ms       Option<u64>
    model             Option<ModelFacts>         P3 就位后填充
}
```

这是本层唯一会随其他阶段长大的类型。P3、P4 就位后新增字段。

**新增字段会让原本不匹配的规则开始匹配**——这是本层最需要留意的变更风险。
每次扩充 `PolicyContext` 都要重新审视既有规则的语义。

## 5.7 消费者：驱动在集成前问策略

本子项目给 Policy Engine 的第一个真实消费者是驱动的集成路径：

```
最高层裁决（第 2 级除外）
  Allow             → 铸造批准值
  RequireApproval   → 有 --approve 才铸造
  Deny              → 有 --approve 才铸造（第 2 级高于 3–5）
第 1 级为 Deny      → 一律不铸造，--approve 无效
```

由此，`GateApproval` 的语义从「有调用方认为该批准」变成「策略或用户的显式确认批准了」，
而 `RequireApproval` 这个决策值第一次被真正使用。

---

# 6. Effect Journal

## 6.1 记录体

```
Effect {
    id                 EffectId
    effect_type        EffectType（封闭枚举）
    target             作用对象，文本
    parameters         JSON
    authorization      不透明凭据串；本子项目只记录，不校验
    idempotency_key    幂等键，唯一
    state              EffectState
    planned_at         Unix 毫秒
    updated_at         Unix 毫秒
}
```

## 6.2 状态机

```
PLANNED     → AUTHORIZED | FAILED | ROLLED_BACK
AUTHORIZED  → EXECUTING  | FAILED | ROLLED_BACK
EXECUTING   → COMMITTED  | FAILED | UNKNOWN
UNKNOWN     → COMMITTED  | FAILED | ROLLED_BACK | UNKNOWN
COMMITTED   → （终态）
FAILED      → （终态）
ROLLED_BACK → （终态）
```

表外迁移一律拒绝。`UNKNOWN` 有出边，因为它表示「不知道外部发生了什么」，而非终态。

## 6.3 驱动侧的写入次序与出口判定

`§268` 要求在**执行之前**记录。驱动的落点：

```
对每个 --effect <类型>:<目标>：
    写 PLANNED（含 authorization 与幂等键）
    写 AUTHORIZED
    写 EXECUTING
—— 全部写完，才执行 --exec 那条命令 ——
命令退出码 0        → 每条写 COMMITTED
命令退出码非 0      → 每条写 FAILED
命令中途进程被杀    → 记录停在 EXECUTING，重启后由恢复转 UNKNOWN
```

三种出口都由**命令的退出形态**判定，不需要知道命令内部做了什么。

## 6.4 「命令即效应」是一个近似

真实的效应由 P3 的连接器完成（`push_branch` 实际是 git push）。本子项目把「声明的效应」与
「实际跑的那条命令」绑在一起。**近似之处**：命令失败时无法知道它做到了哪一步，故一律记
`FAILED`；而崩溃时记 `UNKNOWN`、不猜。

`FAILED` 是「确定没成功」，`UNKNOWN` 是「不知道」——两者的区别正是这张表存在的理由，
故不得为了「干净」把 `UNKNOWN` 并入 `FAILED`。

## 6.5 幂等键

由 `意图 id / 类型 / 目标` 派生（设计上篇 §10）。**执行任何命令之前**先查该键；
**已存在记录即拒绝运行整条命令**。

这样「同一条命令重跑」不会第二次产生副作用，`§52` 的判据由此可在端到端测试里真验：
跑一次、杀掉、重跑，断言只存在一条记录。

拒绝而非静默跳过，是因为静默跳过会让调用方以为命令执行了。

## 6.6 重启恢复

挂在 P0 的 `RecoveryRegistry` 上：

```
EXECUTING 的记录 → 一律转 UNKNOWN
PLANNED / AUTHORIZED 的记录 → 保持原状（尚未执行，语义确定）
COMMITTED / FAILED / ROLLED_BACK → 终态，不动
```

**不猜。** `EXECUTING` 在崩溃时既可能已发出外部调用也可能没有，转 `FAILED` 与转 `COMMITTED`
都可能是错的。`UNKNOWN` 收敛到确定状态需要外部对账或人工决策，两者都不在本子项目范围内。

## 6.7 authorization 字段的边界

本子项目只记录该字段，不做校验。写入的是**批准值是否给出、以及策略的裁决结果**。
校验属 Capability（P3）与 Authority（长期）的职责。此边界须显式声明，否则会被读成「此处已强制」。

## 6.8 EffectType 取封闭枚举

```
send_email / push_branch / publish / delete_remote / charge / deploy
```

理由与 P1 的 `ArtifactType` 相同：策略要按效应类型裁决（「`charge` 必须人工批准」），
开放类型会让策略表漏判。代价是新增效应类型为编译期可见的破坏性变更。

---

# 7. 批准值

## 7.1 从「不可构造」到「只有一个具名产生点」

上篇的保证是「`GateApproval` 无公开构造函数」（类型层不可构造）。本子项目要让驱动能够批准，
故这一保证必须松。**松到哪里**：

> **只有一个具名的、可 grep 的产生点，且驱动是它唯一的生产调用方。**

维持手段是评审与 grep，不再是类型。上篇的两条编译失败样例仍然有效——它们钉的是
「没有**无名**构造路径」（字段私有、无 `new`），那一点没有变。

## 7.2 产生点绑定到具体一次集成

上篇交接的第三条指出：`GateApproval` 不携带也不校验任何 base / task / intent 信息，
任意一枚为任意一次集成背书。本子项目消除这一点——产生点收下这次集成的标识，
铸出的值携带其摘要：

```rust
/// 集成授权的唯一产生点。
///
/// `digest` 由 base 根、task 根与 `view_diff` 的结果派生，使这一枚**只对这一次集成**有效。
/// 调用方须先取得授权（第 5.7 节）；Capability 与 Authority 就位后，产生点移交给那一处。
pub fn approve_integration(
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<GateApproval, GateError>;
```

Gate 在写入前重算摘要并比对，不匹配即拒。**伪造一枚也换不了另一次集成的授权。**

摘要的算法是本子项目定的（具体形式由实现定，须写进文档）。它与 `view_diff` 的结果相关，
故**在集成前改动 Base 会使摘要失配**——这是刻意的：批准的是「把当前这份差异应用过去」。

---

# 8. 持久化

本子项目注册的迁移：

```
effect             id, effect_type, target, parameters, authorization,
                   idempotency_key, state, planned_at, updated_at
                   PK (id)；唯一索引 (idempotency_key)
policy             id, level, scope, condition, decision
                   PK (id)
```

枚举列（`effect_type`、`state`、`level`、`decision`、`scope`）一律小写、多词以 `_` 连接，
经显式辅助函数读写，**不依赖 serde**——该约定沿用 P1 与上篇，理由同：Rust 枚举的 serde
表示与落库编码是两件事。

每张表的读写函数与该表的定义放在同一 crate。`continuum-runtime` 不直接对这些列写 SQL 字面量。

迁移编号**须先查已占用的值**（上篇为 30），取一个未占用的。

装备点在 `continuum-runtime/src/main.rs`：`Db::open_with(path, 既有各组迁移 + 本子项目的)`。

---

# 9. 测试策略

判据是驱动这个**二进制**，故端到端测试驱动真实二进制（与 P1 的 `recovery_roundtrip`、
上篇的 overlay 用例同法）。

| 验什么 | 怎么验 |
|---|---|
| 完整链路 | 建工作区 → 沙箱内跑一条会写 Task 的命令 → 带批准 `--apply` → 断言 Base 出现该改动、Task 仍在 |
| 唯一通道 | 未 `--apply` 时断言 Base 一字未动、工作区已 `discard`、记录已删 |
| 执行前写入 | 命令中途被杀 → 重启 → 断言 `EXECUTING` 转 `UNKNOWN`，且**不猜**成 `COMMITTED`/`FAILED` |
| 幂等键 | 跑一次、杀掉、重跑 → 断言第二次**拒绝运行**且只存在一条记录 |
| 策略裁决 | 各级规则各造一次冲突；同层取更严；无匹配默认 `Deny` |
| `--approve` 的边界 | `Deny`（第 3 级）+ `--approve` → 放行；**`Deny`（第 1 级 System Safety）+ `--approve` → 仍拒** |
| 沙箱两臂 | 链路测试里既有「写 Task 成功」也有「写 Base 被拒」——只有一臂时，一个「拒绝一切写入」的沙箱同样能通过 |
| 后端来源 | 命令行给的 `backend` 与记录不符时，断言以**记录**为准（或不接受该参数） |

**Re-exec 的用例须按上篇的约定给出执行/跳过条数**：那类用例在无 `unshare` 的机器上会整体
跳过而套件仍绿，计数让它在读数时可见。

**变异验证**：关键断言须注入缺陷、确认目标断言在**正确位置**变红、再还原。
**否定结论（「不变红」）必须跑全量 `--no-fail-fast`**——本项目已出过一次「单跑一条用例时
不变红、全套跑却变红」的误判。

**端到端测试的语言限制**：`--exec` 的命令由测试给定，故测试可以只用本机已有的命令构造
「写 Task」「写 Base」两种行为，不必引入脚本依赖。

---

# 10. 完成判据对照

《工程》`§5.4` 五条，上篇覆盖前两条，本子项目补后三条：

| 判据 | 落点 |
|---|---|
| 1 对 Base 的写请求被拒绝，且发生在类型层或文件系统层，可独立验证 | 上篇：`trybuild` 编译失败用例 + 两臂用例 |
| 2 Task 的修改只能经 Integration Gate 进入 Base | 上篇的 Gate；本子项目：驱动是唯一生产调用方，且 `backend` 取自记录 |
| 3 所有不可回滚外部操作在执行前写入 Effect Journal | 第 9 节的崩溃注入用例：断言 `EXECUTING` 记录先于命令执行 |
| 4 Runtime 重启后能根据 Journal 恢复未完成的 Effect 状态 | 第 9 节的真杀真启用例：`EXECUTING` 转 `UNKNOWN` |
| 5 Policy 冲突按 `§105` 六级优先级裁决 | 第 9 节的跨层与同层冲突用例，加无匹配默认 `Deny` |

---

# 11. 遗留风险与未决项

```
Capability 强制       本子项目不建。Effect Journal 的 authorization 字段只记录不校验；
                      GateApproval 由策略或显式确认产生，不校验任何 Capability。
密钥运行时            本子项目不建。
P4 的 Current Contract --approve 占第 2 级是占位。P4 就位后须决定二者如何共存
                      （合并、还是 --approve 降为第 2 级的一个来源）。
执行器与语义层        驱动是临时装配件，非执行器。P4 与执行器就位后取代。
--approve 的定级      按 §105 字面取「显式当前指令高于用户持久策略」，
                      故一个命令行开关可推翻持久策略。若日后判定 Deny 应更硬，须改这里。
摘要算法              approve_integration 的摘要形式由实现定，须写进文档。
                      它与 view_diff 的结果相关，故集成前改动 Base 会使摘要失配——刻意的。
PolicyContext 的扩充  新增字段会让原本不匹配的规则开始匹配。每次扩充须重审既有规则。
两个 crate 的消费者    Policy Engine 的消费者是驱动的集成路径（第 5.7 节）；
                      Effect Journal 的消费者是驱动的效应声明（第 6.3 节）。
                      两者都比上篇多，但仍不是它们的最终消费者（P3 的连接器与 P4 的 Intent）。
Runtime Default 层     须预置已知安全操作的放行规则，否则无匹配即 Deny 会让系统启动即全拒。
沙箱在两台机器上的差异 Landlock 的只读白名单使 /proc /sys /tmp $HOME 不可达，沙箱内跑不了
                      cargo/mktemp/git；bubblewrap 的 --ro-bind / / 则可读。驱动跑真实命令时
                      会撞上这一点，算子画像所需的放行集合属执行器的范围。
```
