# P2 边界层：上篇交接给下篇的事项

本文件只记下篇开工前必须知道、而代码与设计正文里看不出来的东西。上篇（Workspace 抽象与只读强制、
Sandbox 抽象与两种实现）的交付状态见主仓库的提交历史与设计正文；设计正文在
`docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md`。

放在这里而不放进 `p1-followups.md`：那一份是 **P1 交给执行器**的交接，收件人是执行器（阶段未定），
本文件各条是 **P2 上篇交给下篇**的内部交接，收件人明确。两份文件的收件人不同，混在一起会让
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

## 二、`discard` 与 `remove_workspace` 的次序由调用方促成，本层不收 `Tx`

`discard_task_workspace` 只动文件系统与 git，**不收 `Tx`、也不碰数据库**。若该工作区曾由
`save_workspace` 落库，**放弃成功之后要一并调用 `remove_workspace` 删除其记录**——这件事由
调用方促成，本层做不了。要求的实质是**次序**：`discard_task_workspace` 成功返回之后再删记录，
返回 `Err` 时不删。

下篇接驱动时会写「放弃一个 Intent」这条路径，**漏掉这一步不会在当期报警**，两个方向都要避免：

- **有记录无工作区**：`load_workspace` 给出的后端与路径不再对应任何实物，按它去回收只会失败
  （worktree 目录或分支已不在），而那条记录本身看不出已经作废；
- **有工作区无记录**：更隐蔽，要到同一 Intent 再次创建、撞上仍然存在的分支（worktree）或
  Intent 目录（overlay）时才显形。

包一层事务并不比这个次序多出原子性——本层的变更全在文件系统与 git 上，不可回滚。调用方的事务
里若还有别的写（放弃事件、审计记录），与那些写放进同一事务才有意义。

---

## 三、`GateApproval` 是「不绑定」的 token

`pub struct GateApproval(());`——字段私有、无公开构造函数（`crates/continuum-workspace/tests/
compile_fail/gate_approval_*.rs` 钉的就是这两点）。它**不携带也不校验**任何 base / task / intent
信息：任意一枚为任意一次集成背书，`apply_patch` / `merge` 收下后直接 `let _ = approval;`。
设计把产生点交给 Capability 与 Authority（设计 6.2），故这不是缺陷。

**但下篇不要以为它已经绑定了某一次具体的集成。** 在产生点落地之前，它只表示「有调用方认为该批准」，
不表示「这一枚批准的是这一次集成」。要让「批准的是哪一次」成立，需要把 base / task / 改动摘要
摘要进 token 并在校验点比对——那是产生点那一侧的活，不是 Gate 这一侧的。

---

## 四、`ALLOWED` 表两条目的语义不同（记账，非缺陷）

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 表里，两条目记的东西不一样：

```
叶子 crate（如 continuum-workspace）  记的是**实际使用的**依赖——
                                      Task 1 据此删掉了该条目里的 continuum-core
continuum-runtime                     记的是**规范允许集合**，含尚未使用的——
                                      core / events / provider 三条都零实际引用却早已登记
```

P2 上篇给 runtime 加了 `continuum-sandbox`（在 runtime 内同样零使用，消费者是下篇的驱动），
**依据是后者这一条既有的语义**，与其自身的先例一致。

两条目语义不同、却共用同一张表与同一条双向断言，**是这张表的一处不齐**。本子项目未处置。
若要统一，两种走法各有代价：一律按实际依赖记，则 runtime 要删三条既有的（牵动 P1 的决定）；
一律按允许集合记，则 `Cargo.toml` 会声明用不到的东西（那不是对代码的忠实陈述）。
留待后续阶段决定，**在改这张表之前先看清条目属于哪一类**。

---

## 五、`continuum-workspace` 的 `serde` 依赖只服务两个从未被消费的 derive

`Cargo.toml` 的 `serde` 当前只被 `ids.rs:3`（`IntentId`）与 `backend.rs:14/21`
（`WorkspaceBackend`）的 `Serialize`/`Deserialize` 用到，而这两个 derive 在**本 crate 内外
均无使用点**——**落库编码是刻意不走 serde 的**（`persist.rs` 用显式的 `backend_str` /
`parse_backend`，理由见设计：Rust 枚举的 serde 表示与落库编码是两件事）。

这与 Task 1 已修掉的「`serde_json` 零使用」是同一类，只是被 `derive` 遮住了：
零使用的**依赖**好认，零使用的 **derive** 不好认。

**不紧急**：删 derive 与依赖是一行级改动，但会动 `IntentId` 的公开面（它是否该可序列化，
取决于下篇的驱动会不会把它写进 CLI 参数或落库）。**随下篇一并决定**，本子项目不处置。

---

## 六、`ALL` 常量与枚举变体可能不同步（跨 crate 的改进候选）

带 `ALL` 常量的枚举在本项目有多个（`continuum-graph` 的 `NodeState`、
`continuum-effect` 的 `EffectState`，以及任何后续同类）。它们共有一个缺口：

```
加一个变体、不改别处        → 编译失败（穷尽 match 拦住，E0004）——有效拦截
加一个变体 + 补上那处 match 的臂、但忘了加进 ALL
                          → **全绿**，新变体不在 ALL 里，遍历它的用例静默少测
```

**仅靠断言关不掉第二条**：判定变体总数必须能枚举变体，来源只有手写名单本身（循环）
或 `std::mem::variant_count`——后者在 rustc 1.95 上仍是 unstable（`E0658`，issue #73662，
2026-10-03 实测）。

**真正能关掉它的写法**是让 `ALL` 与变体清单同源（一个 `macro_rules!` 同时展开枚举与 `ALL`，
加变体只能改那一处）。这会改枚举的**定义形态**，而现有几处都是普通定义，
故属跨 crate 的结构性决定，**本子项目未做**。

**当前的处置**：保留 E0004 的编译期拦截；在 `ALL` 附近如实写明该残留缺口与原因，
不写成绝对措辞。风险真实但很窄——第二条需连犯两处，而 E0004 会强制作者走到 `ALL` 所在的那一带。

**留待某一阶段统一处理**：若要引入该形态，几处枚举应一起改，否则惯用法会一分为二。

---

## 七、`PersistError` 缺少可区分的冲突变体

`Tx::execute` 把 rusqlite 的错误码抹成字符串，故唯一键冲突只能落在 `Database` 变体里、
按**消息文本**断言（`UNIQUE constraint failed: effect.idempotency_key`）。
调用方若想按类型分派（例如「撞唯一键就取既有记录」而不是「库坏了」），现在做不到。

`continuum-effect` 的 `record_planned` 与 `advance` 都受此影响，各自按消息断言。
**要类型层区分需改 `continuum-persist`**，那超出当时 task 的文件清单，故未动。

**留待后续阶段**：若多处需要按类型分派数据库错误，再统一给 `PersistError` 加冲突变体；
现在只有一处，按消息断言足够。
