# P2 边界层：上篇交接给下篇的事项

本文件只记下篇开工前必须知道、而代码与设计正文里看不出来的东西。上篇（Workspace 抽象与只读强制、
Sandbox 抽象与两种实现）的交付状态见主仓库的提交历史与设计正文；设计正文在
`docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md`。

放在这里而不放进 `p1-followups.md`：那一份是 **P1 交给执行器**的交接，收件人是执行器（阶段未定），
本文件的两条是 **P2 上篇交给下篇**的内部交接，收件人明确。两份文件的收件人不同，混在一起会让
两份都不再知道自己对谁说话——`p1-followups.md` 的收件人此前正是被记错成了 P2。

---

## 一、`backend` 与 Task 形态不匹配时，overlay 的**写**路径比读路径危险得多

`apply_patch` 与 `merge` 在 `WorkspaceBackend::Overlay` 分支上调 `integrate_overlay(base_root,
task_root)`（`crates/continuum-workspace/src/gate.rs`），它把两棵树**整棵**对差：先删掉
`base - task` 的每个文件，再复制 `task - base` 与内容不同的。把一个 **worktree 形态**的 Task
（Task 根里 `.git` 是**文件**）配成 `Overlay` 后端时，Base 的 `.git/` 是个**目录**：

- 删除那一趟先把 `base/.git/` 下的每个文件当「Task 侧已删」逐个移掉——`HEAD`、`config`、
  `objects/`、`refs/`…… 在 `collect_files` 眼里都与普通文件无异，而 `remove_entry` 对文件
  一律放行；
- 复制那一趟走到 Task 侧的 `.git` 时，目标 `base/.git` 是目录，`remove_entry` 这才拒绝，
  报 `GateRefused`（reason 里带「…… 是目录，而 Task 侧对应的是文件」）。

**拒绝确实会发生，但它发生在 Base 的 `.git` 已被掏空之后。** 这不是「先撞上拒绝、爆炸半径有限」：
Task 10 用一次临时探针实测过——Base 侧 `.git/HEAD` 与 `.git/config` 在 `integrate_overlay`
返回 `Err` 时都已不存在。「先变更、后审计」的次序在这里也帮不上忙：被删掉的文件既不在事务里，
也没有副本。

`cherry_pick` 不受影响（它在该分支上直接拒绝，不退化）。`view_diff` 与 `discard` 走的是别的
分支，也不受影响。

**下篇接驱动时**：`backend` 不要从命令行参数或请求里取，要从建 Task 时落库的那条记录取
（`workspace` 表的 `backend` 列），并让**唯一**的取用点在那里。若要更硬，可在
`apply_patch` / `merge` 进入 overlay 分支前加一道形态校验——worktree 形态的 Task 根含
`.git` 文件，overlay 形态的不含；两者与 `backend` 参数对不上时先拒，别让整棵对差跑起来。

---

## 二、`GateApproval` 是「不绑定」的 token

`pub struct GateApproval(());`——字段私有、无公开构造函数（`crates/continuum-workspace/tests/
compile_fail/gate_approval_*.rs` 钉的就是这两点）。它**不携带也不校验**任何 base / task / intent
信息：任意一枚为任意一次集成背书，`apply_patch` / `merge` 收下后直接 `let _ = approval;`。
设计把产生点交给 Capability 与 Authority（设计 6.2），故这不是缺陷。

**但下篇不要以为它已经绑定了某一次具体的集成。** 在产生点落地之前，它只表示「有调用方认为该批准」，
不表示「这一枚批准的是这一次集成」。要让「批准的是哪一次」成立，需要把 base / task / 改动摘要
摘要进 token 并在校验点比对——那是产生点那一侧的活，不是 Gate 这一侧的。
