# P2 边界层：上篇交接给下篇的事项

本文件只记下篇开工前必须知道、而代码与设计正文里看不出来的东西。上篇（Workspace 抽象与只读强制、
Sandbox 抽象与两种实现）的交付状态见主仓库的提交历史与设计正文；设计正文在
`docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md`。

放在这里而不放进 `p1-followups.md`：那一份是 **P1 交给执行器**的交接，收件人是执行器（阶段未定），
本文件的两条是 **P2 上篇交给下篇**的内部交接，收件人明确。两份文件的收件人不同，混在一起会让
两份都不再知道自己对谁说话——`p1-followups.md` 的收件人此前正是被记错成了 P2。

---

## 一、`backend` 与 Task 形态不匹配：本层已封住，但下篇要保证 `backend` 的来源

**曾经是一条能毁掉用户仓库的路径。** `apply_patch` 与 `merge` 在 `WorkspaceBackend::Overlay`
分支上调 `integrate_overlay(base_root, task_root)`，把两棵树**整棵**对差。把一个 **worktree
形态**的 Task（根里 `.git` 是**文件**）配成 `Overlay` 后端时，Base 的 `.git/` 是个**目录**，
而旧实现是「边删边发现」：删除那一趟把 `base - task` 的每个文件移掉，而 `collect_files` 只收
文件与符号链接、`remove_entry` 只对**目录**拒绝——`base/.git/**` 全是文件，一路放行；等复制那
一趟走到 Task 侧的 `.git` 才拒绝。实测（Task 10 的临时探针，`integrate_overlay` 直接调）：

```
旧：Err(GateRefused { …「base/.git 是目录，而 Task 侧对应的是文件」… })
    base/.git/HEAD          exists = false
    base/.git/config        exists = false
    base/.git/objects/ab/cdef exists = false
新：Ok(())
    base/.git/HEAD          exists = true
    base/.git/config        exists = true
    base/.git/objects/ab/cdef exists = true
    base/要改的.txt          = "task\n"      ← 真正的改动照旧落进 Base
```

**本层的处置是三层的**，都在 `crates/continuum-workspace/src/`：

1. **形态校验排在最前**：`overlay::require_layout(task.root())`——overlay 后端要求 Task 根是
   本后端的布局（`<intent>/{upper,work,mnt}`，见 `overlay.rs`）。worktree 建出的 Task 根
   （`<base>/.ai/worktrees/<intent>`）必然不合格。判定与 `discard` 共用同一个函数，误用报的
   是同一个 `BackendUnavailable`。`apply_patch` 与 `merge` 两条路都有守卫用例。
2. **集成拆成两阶段**：`integrate_overlay` = `plan_overlay_integration`（算计划 + **全量校验**）
   → 按计划执行。凡是能从计划本身判定的失败（源不是条目、目标是目录）都在动第一个字节之前
   报出，`Err` 时 Base 一字未动。执行阶段仍可能因 I/O 错误或两侧树被并发改动而失败，那部分
   不在计划能判定的范围内，也没有事务可回滚。
3. **`.git` 不进差异集**：`collect_files` 按名字排除任何层级上的 `.git`（连同子树）。仓库的
   内部状态不是内容；两侧 `.git` 形态不同（目录 vs 文件）时按内容一比就是「Base 侧一整棵
   `.git` 被删」，而那正是让整棵删除合法化的那一步。排除后**合法的 overlay 集成路径不受影响**
   ——已在真挂载的覆盖层上跑过（`applying_a_patch_on_a_mounted_overlay_reaches_the_base`，
   执行 1、跳过 0）。

**下篇仍要做的一件事**：`backend` 不要从命令行参数或请求里现取，要从**建 Task 时落库的那条
记录**取（`workspace` 表的 `backend` 列），并让取用点唯一。上面三层挡住的是「拿着错的 backend
调进来」，它们挡不住「上层记错了后端、又照错的记法去回收别的资源」。

---

## 二、`GateApproval` 是「不绑定」的 token

`pub struct GateApproval(());`——字段私有、无公开构造函数（`crates/continuum-workspace/tests/
compile_fail/gate_approval_*.rs` 钉的就是这两点）。它**不携带也不校验**任何 base / task / intent
信息：任意一枚为任意一次集成背书，`apply_patch` / `merge` 收下后直接 `let _ = approval;`。
设计把产生点交给 Capability 与 Authority（设计 6.2），故这不是缺陷。

**但下篇不要以为它已经绑定了某一次具体的集成。** 在产生点落地之前，它只表示「有调用方认为该批准」，
不表示「这一枚批准的是这一次集成」。要让「批准的是哪一次」成立，需要把 base / task / 改动摘要
摘要进 token 并在校验点比对——那是产生点那一侧的活，不是 Gate 这一侧的。
