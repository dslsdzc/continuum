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

---

## 八、批准值摘要与 worktree 后端的集成集合在 `.ai` 上不重合（fail-open）

**现象。** Task 7 给 `GateApproval` 加了内容派生摘要，其输入含两棵树的 `tree_digest`。
摘要按**名字**排除 `.git` 与 `.ai`（任意层级、不分条目种类）。overlay 后端上集成集合
与之同源（两侧都跑 `collect_files` 的那个名字测试），故两侧一致；**worktree 后端上不重合**，
因为 `view_diff` 与 `apply_patch` 那一侧是 git 的规则，两者有两处差别：

1. **只管目录。** `.git/info/exclude` 里写的是 `.ai/`（`worktree.rs` 的 `ensure_ai_excluded`），
   带尾斜杠，故 gitignore 只与**目录**匹配：名为 `.ai` 的**常规文件或符号链接**都不被忽略，
   会被 `git ls-files --others --exclude-standard` 报成新增（实测二者同形），而摘要按名字跳过它。
   **根层那一个复制不过去**（`<base>/.ai` 本是存储目录，撞上 `ensure_not_a_directory` 被拒），
   须取**嵌套**形态（`sub/.ai` 是常规文件或符号链接）才会既被报出又被复制——故根层那一处是
   **偶然，不是保障**。符号链接形态下复制是**按目标重建**（`read_link` + `symlink`），不是复制
   所指文件的内容：实测 Base 上落下的是一条指向**铸造之后**才改成的目标的链接。
   对照：`sub/.ai/` 是目录时，`git ls-files --others --exclude-standard` 与 `view_diff` 都为空
   ——既不报也不搬。
2. **只管未跟踪路径。** gitignore 不压制已跟踪路径。故 Base 里**已提交**的 `sub/.ai/f.txt`
   在 Task 里被改动时，`git diff` 会报出、`git apply` 会应用，而它的内容**不在摘要里**。

**第 2 条的方向是 fail-open。** 用户可在铸造批准值与写入操作之间改掉那个已跟踪文件：
摘要不变、`verify_approval` 照过、改动照落进 Base——为一份内容铸的批准值被应用到另一份上。
实测（worktree 后端）：`view_diff.modified()` 给出 `["sub/.ai/f.txt"]`，拿着铸造前那份内容
铸出的批准值 `apply_patch` 返回 `Ok(())`，Base 拿到的是铸造**之后**才写的那一份。
第 1 条同样成立，但要取**嵌套**形态（`sub/.ai` 是一个常规文件）：根层那个名为 `.ai` 的文件
会撞上 `<base>/.ai/` 本是目录，被 `ensure_not_a_directory` 拒掉——**那一处是偶然，不是保障**。

**可达性的现实前提。** Base 里须存在已提交的 `.ai/…`。`.ai/` 是 Runtime 的保留名字，
`worktree.rs` 在创建时就把它写进 `info/exclude`，故这只在「用户在排除规则生效**之前**
就提交过」这一类仓库里成立（实测用 `git add -f` 造出该形态）。可达性窄，但方向不利。

**为什么不在 Task 7 关掉。** 关掉要改排除规则本身，而这是**第二次设计裁定**，不是实现修复：

- 「收窄为根层」不解决问题——`<task_root>/.ai/…` 下被跟踪的内容仍会被 git 应用而摘要不收，
  只是把同一处缝挪到别的路径上。
- 给两个后端分别定规则（worktree 侧任务树一律不排 `.ai`；overlay 侧两棵树都排根层 `.ai`）
  会在 overlay 侧重新引入一处摘要与 Diff 的分歧。

**一条否定结论（免得重走）。** 不要试图通过 `info/exclude` 修第 2 条：把 `.ai/` 改成 `.ai`
能摸到名为 `.ai` 的文件，但 **gitignore 压不住已跟踪路径**，故该文件根本关不掉第 2 条。

**两条候选规则，待裁定。**

- **规则 R（只根层排除）**：`digest_dir` 的 `.ai` 名字测试只对 walked root 的直接子项生效
  （`rel` 为空），`.git` 仍任意层级；`collect_files` 保持现状或同样收窄。
  worktree 侧净效果相对今天为 fail-closed；overlay 侧摘要覆盖成为 Diff 的**超集**
  → fail-closed 但会多想，并重新产生一处（安全方向上的）摘要与 Diff 分歧。
- **严格按设计 §4.1**：两侧都只排除 `<base>/.ai` 这个保留子树，其余照常。

**现状**：本轮只订正了那处把「两侧判据逐项相同」写死的文档措辞，并写明上述 fail-open 与实测。
代码未改。

---

## 九、下篇实现过程中积累的残余（逐条待处置）

本节是**下篇各 task 评审与实现者报回的残余清单**，按处置所需的前置条件分组。凡是只在
`.superpowers/sdd/` 的报告或协调者 ledger 里出现过的，都收在这里——那个目录是 gitignore 的，
**只写在那里的事项会随 branch 消失**。

### 9.1 需要一次设计裁定的

- **`in: []` 是否该在解析期拒绝**（Task 4）。设计 §5.1 只列了三类「不符」（事实名不命中、
  比较符不在封闭集合、取值类型不符），空集合不属任何一类，故当时**按字面收下**（结构合法、
  永不匹配），并有用例钉住与写明理由。但 §5.1 的立论正是「不许静默退化为永不匹配」，
  故「要不要加第四类拒绝」是一次设计裁定，未做。
- **摘要排除规则要不要按后端分别定**（Task 7，与第八节同源）。第八节记了 worktree 侧那处
  fail-open 与两条候选规则（「只根层排除」／「两侧都只排 `<base>/.ai`」），待裁定。
- **「形如选项的取值被当作取值收下」要不要拒绝**（Task 8）。`task --base --intent i1` 会把
  `base` 吃成 `"--intent"`，随后报 `UnknownOption { name: "i1" }`——**报错指向了错的 token**。
  方向是 fail-closed、且无注释声称相反。当时**裁定不改行为**：设计没规定这一条，而「拒绝形如
  选项的取值」是一条**新规则**，将来若有取值合法地以 `-` 开头（路径、正则）会误伤。

### 9.2 需要动手改码的既有问题

- **`ArtifactType` 有两份独立的编解码表**（Task 4 复审发现）：
  `crates/continuum-artifact/src/persist.rs` 与 `crates/continuum-graph/src/persist.rs` 各一份，
  取值逐字相同。**这是活的重复**，与 Task 4 已消除的 `EffectType`/`PrivacyClass` 那一类同形，
  只是当时不在该 task 的 diff 内故未动。
- **`PrivacyClass` 的 serde 派生未被收掉**（Task 4）。当天查得无消费方，尚未构成第二套表示，
  但 `Artifact` 自身派生 `Serialize` 且字段含 `privacy_class`，**单独摘掉会让 `Artifact` 的派生
  编译不过**，故不是可独立移除的。已在该枚举的文档注释里写明「今天无序列化路径，但
  `to_value(&artifact)` 会踩上」。
- **P2a 用例 `tests/gate.rs::every_write_operation_refuses_a_stale_approval` 的快照判据比摘要宽**
  （Task 9）。`tree_snapshot` 什么也不排除，而 `tree_digest` 在任意层级跳 `.git`/`.ai`，
  故 git 的后台维护锁 `.git/objects/maintenance.lock` 的开合会让它偶发失败（加压时一次，44 次
  重跑 0 次；Task 11 期间**全量并行跑**时又复现过一次，同一支，单跑与紧接的下一次全量均通过）。**方向是改测试侧快照，不动产品代码**；实测已确认那次集成确实被拒（`ApprovalMismatch`
  断言排在快照断言之前且通过），**不是 fail-open**。

### 9.3 需要写进注释或文档的

- **摘要对路径拼写敏感**（Task 7）。`base.root()`／`task.root()` 的原始字节进摘要，故非规范路径、
  结尾多余斜杠、符号链接拼法、Base 被移动都会使摘要不同。fail-closed、今天无害；但驱动若在
  **两次独立调用**里用不同拼法铸造与应用，批准值会莫名失效。
- **`USAGE` 里的程序名取 `continuum`**（Task 8），而实际 bin 名是 `continuum-runtime`。
  取自设计 §4.1 的用法行。改名或加别名不在该 task 内。
- ~~**两条删除义务**（Task 9 起）~~ **已履行**（Task 10 删了 `ApplyNotWired` 与其用例，
  Task 11 删了 `EffectNotWired` 与其用例）。留此行只为记来历：那两条是**未接线期间的临时占位**，
  各配一条钉住「尚未接线」的用例，接线时必须一并删——**否则会留下一条钉着已不存在行为的用例**。
  （`task_is_a_declared_placeholder_until_it_is_wired` 属同一类，在 Task 9 分派到 `task_cmd::run`
  时即必然变红，已随之删除。）
- **`TaskError::RecordMissing` 是一条没有照片的防御分支**（Task 9）。它要求「本次运行刚写过的
  记录被别处删掉」，端到端造不出（要靠并发）。方向是 fail-closed（宁可报错也不按猜出来的后端
  回收），据实记为**未被任何用例覆盖的代码路径**。
- **两条等价变异体，不要试图为它们补用例**（Task 9）：`IN_NAMESPACE_ENV` 在本机冗余
  （`unshare -Urm` 之后 `in_user_namespace()` 本就为真；它的真实价值只在探测本身坏掉时作第二道
  保险）；「Base 与 Task 根也取自记录」那一半**在观察上不可分辨**（记录里的 `base_path` 与命令行
  给的是同一个路径、`path` 与句柄 `root()` 是同一个规范路径），故只有 `backend` 那一半有对照片。

### 9.4 `ALL` 常量的生成（跨 crate 改进候选）

续第六节：`EffectState::ALL` 的完整性只能靠「加变体时作者记得同步」——`mem::variant_count` 在
rustc 1.95 上仍 unstable（E0658），实测「加变体并补 `ordinal` 的臂但不加进 `ALL`」**全绿**。
若某一阶段要引入「由变体清单生成 `ALL`」的形态，几处枚举应一起改。

### 9.5 策略层与驱动：后续阶段要重新审视的两处

- **`mints` 的 `RequireApproval` 支目前只在 API/测试层可达**（Task 10）。协调者曾判定「本接法下
  `mints(RequireApproval, true)` 不可达」，**该结论是错的**，被实现者驳倒：`decide` 取最高层、
  同层取更严，而 `--approve` 已给出时第 2 级 `Allow` **只压得过第 3–5 级**的 `RequireApproval`。
  另有两条路照样返回 `RequireApproval`：**第 1 级 `RequireApproval`**，以及**落库一条与第 2 级
  同层（`ExplicitCurrent`）的 `RequireApproval`**（同层取更严，压过内建的 `Allow`）。两条都能
  由公开 API 造出：`Level` 六变体全在封闭集合，`save_policy` **不校验层级来源**。
  **但本 task 之后能写 `policy` 行的只有 `continuum_policy::save_policy` 这个 API，没有命令行入口**，
  故那两条路在真实调用里暂时造不出。**若将来加了写策略的 CLI，第 1 级与第 2 级同层规则会成为真实
  可达路径，届时 `mints` 的 `RequireApproval` 支要重新审视。**
- **第 5 级 `Runtime Default` 的预置放行规则尚未落地**（Task 10）。设计 §5.3 要求「第 5 级须预置
  已知安全操作的放行规则，否则系统启动即全拒」。本子项目的各 task 均未预置任何规则（brief 未要求），
  故**空库且无 `--approve` 时所有集成默认被拒**——这是 fail-closed 的**预期**结果，不是缺陷，
  但「预置规则由谁在何时落」尚无归属，记此以免被当成已覆盖。
- **`tests/startup.rs` 的迁移计数断言有一处不该改**（Task 10）：`second_startup_applies_no_migration`
  断言 `0`——第二次启动确实不再应用迁移，它与「迁移应用 N 项」的那些计数**不是同一类断言**。
  后来者改迁移集合时不要顺手改它；全仓「迁移应用」相关断言共五处，其中三处随迁移数变化。

### 9.6 效应 Journal 的三处残余（Task 11）

- **「写入早于执行」这条判据的照片在 Landlock 下不可构造，只能点名 bubblewrap 并在无 `bwrap` 时跳过。**
  根因不是白名单配少了，而是**两种机制在读侧本来就不等价**：库位于 `<base>` 之下，而 Landlock 下
  「Base 读写皆不可达」，bubblewrap 的 `--ro-bind / /` 才让 Base 可读（见上篇 §4.3 与
  `continuum-sandbox` 的文档）。故这条用例在无 `bwrap` 的机器上**整体跳过**——按项目约定给出
  执行/跳过条数。
  该用例的命令侧读的是**落库字节**（含 `-wal`）而非 SQL：本机不保证有 `sqlite3` CLI，为一条用例引
  脚本依赖不划算。**判别力与 SQL 读的等价性由一条变异证实**——若日后改动这条观察，要重新证一次。
- **第 6 步（写终态）的数据库写失败这一支，设计未覆盖**（Task 11）。实现选择「不集成 + 按第 8 步清理」。
  理由与「命令非 0 不集成」是同一条：终态写不进去意味着**这次运行的结局没有被记录**，此时集成等于
  把 Base 的改动挂在一个**没有记录的授权**上。附带：该分支下 `remove_workspace`（同为数据库写）多半
  也会失败，于是实际退化为「保留工作区与记录」——**两条路在这里自然收敛，不必另设分支**。
- **幂等键的「先查」与「库层唯一索引」是两道防线，去掉预查仍会拒绝**（Task 11）。错误的退化是
  从「点名幂等键」变成原始的 `UNIQUE constraint` 文本；预查的可观察差异是**错误质量与「先查后写」的
  次序**，不是「拒绝与否」。设计明写「先查该键」，故预查不冗余——**但不要以为去掉它没有代价**。

### 9.7 计划文本的一处自相矛盾（Task 11）

**计划 Task 11 的 Step 1 第 2 条自相矛盾**，实现按能成立的那个构造做了，此处记下这处**关于计划文本**
的事实（代码侧不缺理由——用例的 doc comment 已写清为何杀驱动）：

> 该条要求「跑一条**会自杀**的命令」，并断言「进程消失后……断言该记录转 `UNKNOWN`」。但**命令自杀时
> 驱动仍然活着**，会照 §4.2 第 6 步给该记录写下 `FAILED`——即**该构造根本达不到它自己那条断言**。
> 唯一能满足断言的构造是**杀掉驱动进程**（Task 11 的实现即此）。

这与本项目那条既有认识同源：手写的计划文本事实错误率高，**正文措辞是约束、但约束之间会互相冲突**，
冲突时要挑出哪一条与其余全部相容。
