# P7C Resource Arbiter — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-18-p7-scope-and-split.md` §五 P7C 行的原话，逐字照用）：

> 建独立于模型的资源仲裁：压力／负载类别、四档优先级、协作式挂起（含 `Checkpointing` 两阶段与失败回退）与最小暴露面

**本块拥有 / 产出的接口**（同行的第二栏）：`ResourcePressure`、`WorkloadClass`（§13 的四档）、
四档动作 `run`／`throttle`／`suspend`／`resume`、`SuspensionPhase`（含 `Checkpointing` 两阶段与失败回退）、
以及仲裁结论面。**对 AI／后台的结论只有** `resource_pressure` 与 `workload_class`；**§300 的四档优先级
另有一张只对装配方可见的面**；**对 P7D 另有四档动作与 `SuspensionPhase`**。**本 crate 在 workspace 内零依赖**。

**规范依据**：§300（`docs/spec/05-normative.md:1907-1921`）、§13（`docs/spec/01-concepts.md:669-724`）、
§14（`docs/spec/01-concepts.md:728-754`）；《工程》§7.1（`docs/02-工程.md:429`）、§7.2（`:441`）、
§7.3（`:452`、`:458`）、§7.4 判据 3（`:465`）；A14／OPEN-012（`docs/02-工程.md:345`；
`docs/01-总纲.md:1698-1702`、`:1881-1884`）。

**本文件只写 P7C。** P7A／P7B／P7D 的内部一律不写；凡不得不假设别的块的形状之处，一律在 §7 与 §11
写成**具名的对账条目**，不在正文里就近决定。

**箭头约定**：本文件沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」
（`docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

**行号**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p7-plan`，HEAD `bc0993c`，
2026-10-19）当场 `grep -n`／`nl -ba` 实测的，不照抄任何一份别的文档。切分文档的行号按定稿
`c89d839`；P7B 的行号按定稿 `9134676`。

---

# 1. 范围

## 1.1 建什么

| 具名义务 | 出处 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| `ResourcePressure`（资源压力） | §13 `:671-684`、`:720`；§300 `:1907-1921` | §4.2、§5.1 | 四档等级＋六项聚合读数；**量纲与阈值规范未给**（§2.4 第 1、2 条） |
| `WorkloadClass`（§13 四档进程类） | §13 `:708-715` | §4.3 | 四档闭集；`INTERACTIVE` 不设变体（§2.4 第 3 条） |
| `resource_pressure`＋`workload_class` 的最小结论面 | §13 `:717-724`；§300 `:1921`；切分 §2.3.2 第 2 条 `:155-165` | §4.4、§6.1 | `ResourceSummary` 恰两枚结论；**不承载四档优先级、原始进程信息** |
| §300 四档优先级（只对装配方） | §300 `:1909-1919`；切分 §2.3.2 第 2 条 `:155-165` | §4.5、§4.6、§4.12 | `PriorityTier` 在私有模块、`assembly` 特性下才公开；默认构建不可命名 |
| 四档动作 `run`／`throttle`／`suspend`／`resume` | 切分 §2.3.2 第 2 条 `:158-159`；§14 `:730-744` | §4.7、§4.10、§5.4 | `ResourceAction` 恰四变体；`Option` 只表示「无转移」，不是第五动作 |
| `SuspensionPhase` 四相位 | §14 `:732-744`；切分 §2.3.2 第 2 条 `:159-165` | §4.8、§5.5 | `Running`／`Throttled`／`Checkpointing`／`Suspended`；**`Suspended` 只在 checkpoint 成功后可达** |
| `Checkpointing` 两阶段与失败回退 | 切分 §2.3.2 第 2 条 `:160-164`；§14 `:730` | §5.5、§5.6 | 失败／超时返回错误并回 `Throttled`；不终止进程 |
| 独立于 AI 模型 | §13 `:673`；《工程》§7.2 `:441`；§7.4 判据 3 `:465` | §2.1、§7.3、§8 J1／J2 | workspace 内零依赖；无模型调用路径；模型不可用时仍产出限流动作 |
| 最小暴露面 | §300 `:1921`；A14／OPEN-012；切分 §6.2 第 1 条 `:400` | §6、§8 J6／J7／J14 | 公开类型无进程名／PID／逐进程 CPU／GPU；默认构建不导出四档优先级 |

## 1.2 不建什么（摘要；完整表在 §9）

以下各条**不建、不留桩**，逐条给出归属：

| 不建的东西 | 归属 / 理由 |
|---|---|
| Resource Scheduler 本体：`Node 在哪里执行`／`用哪个模型`／`哪个工具`／`多少并行`（§304 `:1973-1982`） | 第 4 层；P7B 只冻结具名端口，装配方实现；P7C 不重造（切分 §1.1 `:29-36`、§6.2 第 12 条 `:411`） |
| P7B 的 `ResourceDirectivePort` 实现 | 装配方 `continuum-runtime`；P7B 定义 trait，P7C 只提供结论（切分 §三第 5 条 `:205-212`） |
| `continuum-provider`／`continuum-model-registry`／`continuum-node` 的依赖或转调 | 与 §7.3「入度为零」相抵；P7C 不选模型、不放置 Node、不选工具（§2.1、§9） |
| 模型调用、提示、上下文装配、`ContextPackage` | P3／P6A；P7C 没有模型面（§13 `:673`） |
| 进程探测（读 CPU／RAM／GPU／VRAM／Disk IO／Network、进程名、PID） | 本机 Resource Arbiter 组件的本地探测器（装配方落地；**不在零依赖决策核 `continuum-arbiter` 内**）；P7C 只接收聚合读数与**已分类**的 `WorkloadClass`，公开类型不携带进程名／PID（§2.4 第 4 条） |
| `throttle` 的量化语义（降多少并发／速率） | P7D；P7C 只给动作名（§11 第 7 条） |
| checkpoint 的内容、落点、重试执行、重启恢复 | P7D；P7C 只拥有 `Checkpointing`／`Suspended` 相位与失败结果（§11 第 8 条） |
| 后台任务来源（§299 四类）与 `TaskSource` | P7D；P7C 的 `WorkloadDeclaration` 不是 `TaskSource`（§2.6） |
| 表、迁移、事件、审计记录 | P7C **零表**（§10）；不占 `200–209` 档 |
| 终止进程、`kill`、`Kill` 动作 | §14 `:730`「后台任务不能粗暴 kill」；P7C 只做协作式挂起（§9） |
| §331 的 `Decision`、`decision.required` | P7A；P7C 的 `ResourceDecision` 不是 §331 的决策单（§2.6） |
| 产品状态、Milestone、Session、后台治理 | P7A／P7D；P7C 不碰它们的内部（§7） |

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：**`continuum-arbiter`**。

**名字的理由**（切分 §1.2 `:57-61` 建议名；四块之间不得出现同义 crate）：

1. **`arbiter` 是 §300 的唯一词根**（`Resource Arbiter`，`docs/spec/05-normative.md:1907`），不另造
   「resource-manager」「pressure-controller」一类第二套词汇。
2. **不叫 `continuum-resource-scheduler`**：那会把 P7C 读成第 4 层的 Resource Scheduler（§304 `:1973-1982`），
   而本块**明确不建** Resource Scheduler（§9）。
3. **不叫 `continuum-budget`**：`continuum-budget` 是**记账**（预算树、账本、`check`；
   `crates/continuum-budget/src/lib.rs:1-68`、`dimensions.rs:14`、`validator.rs:73`），不是资源仲裁
   （切分 §2.2 `:114`）。同名或合并会把「花了多少」与「该不该让路」搅成一件。
4. **实测未占用**：`grep -rn "continuum-arbiter\|continuum_arbiter" crates/ Cargo.toml` 在本工作树**零命中**
   （2026-10-19 实测）。

**依赖边表**（只登记实际用到的）：

| 被依赖 | 用到什么 | 边别 |
|---|---|---|
| **无** | — | **workspace 内零依赖（硬断言）** |

**外部依赖：零条。** 不使用 `serde`／`thiserror`／`sysinfo`／`rand`／`tokio`；只使用 `std`。
错误类型手写 `Display`／`Error`（§4.9、§4.11）。测试也只使用 `std`，**没有 dev-dependencies**，
因此 `cargo tree -p continuum-arbiter --edges all` 只有本 crate 一行（§8 J1）。

**`Cargo.toml` 的形状**（示意；实现期照此落地）：

```toml
[package]
name = "continuum-arbiter"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[features]
default = []
assembly = []

[dependencies]
# 空：workspace 内零依赖、外部零依赖

[dev-dependencies]
# 空

# `tests/assembly.rs` 只在启用 assembly 特性时构建；默认 `cargo test -p continuum-arbiter`
# 不得编译它（否则 `continuum_arbiter::assembly` 不存在，默认全套测试红）。
[[test]]
name = "assembly"
required-features = ["assembly"]
```

**P6D 合并门的前置**：加入 `("continuum-arbiter", &[])` 后，`p6_union_adds_only_its_own_rows`
的断言 8（新增 subject 恰是已落地的 P6 crate）必红——`continuum-arbiter` 不在 `P6_NAMES`。
P7C 本地只验 J1(i)；J1(ii) 的整树绿依赖 P7D 先完成 P6D 合并门的 rebaseline／退役（§11 第 14 条）。
本块**不**改 `P6_EXPECTED_ROWS`、`p6_pre_allowed.txt` 或断言计数。

**共写文件**（切分 §一 的共写文件表给的是多处；本块只碰下面两处）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `crates/continuum-arbiter` | 只加自己这一行 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-arbiter", &[])` | **空数组是硬断言**（同 `continuum-protected` 的先例，`:322`）；任何指向 workspace 内别的 crate 的直接边都会让逐对断言变红 |

**本块不碰**：

- `continuum-runtime` 的 `ALLOWED` 行——P7C 不被 runtime 依赖；装配方落地时才把本 crate 加进它自己的行，
  并启用 `assembly` 特性（§6.5、§7.2）。P7C 本轮只加自己那一行。
- `runtime_migrations()`（`crates/continuum-runtime/src/main.rs:114`）、`expected_migrations()`
  （`crates/continuum-runtime/tests/migrations.rs:37`）、`tests/startup.rs` 的三处计数
  （`:26`、`:81`、`:94`）——本块**零表**（§10）。
- P7B 的 `ResourceDirectivePort`、P7D 的 `SuspensionState` 持久化、P7A 的任何产品面。

**为什么本块必须零依赖**：§7.3 逐字写「Resource Arbiter ← 无（独立于模型层）」「Resource Arbiter 的
入度为零」（`docs/02-工程.md:452`、`:458`），§7.2 写「不能依赖第 4 层的任何组件」（`:441`）。
只有独立 crate 的 `ALLOWED == []` 能把这条做成可观察事实（切分 §五 P7C 行 `:334`、§五判据段 `:356-365`）。
**若本块需要任何 workspace 内 crate，判据 3 就不再可观察**；该边必须先报协调者（§11 第 1 条）。

## 2.2 冻结清单：本块持有的类型与判定

**类型**（定义在 `continuum-arbiter`；共 15 枚，默认公开面 13 枚，装配方特性面 2 枚）：

```
默认公开面：
ResourcePressure          —— §13 的压力等级（量纲本设计定，§2.4 第 1 条）
WorkloadClass             —— §13 的四档进程类（四变体，无 INTERACTIVE，§2.4 第 3 条）
ResourceObservation       —— 六项聚合读数＋已分类的 workload_class（输入面）
ResourceSummary           —— 对 AI／后台的两项结论（§4.4）
WorkloadDeclaration       —— 四档优先级的分类输入（§4.5）
ResourceAction            —— run／throttle／suspend／resume（§4.7）
SuspensionPhase           —— Running／Throttled／Checkpointing／Suspended（§4.8）
SuspensionState           —— 每任务的挂起状态机（§4.8）
SuspensionError           —— checkpoint 失败／超时／错相位的错误（§4.9）
ResourceDecision          —— 对 P7D 的结论（summary＋action＋phase，§4.10）
ArbiterPolicy             —— 压力阈值、cooldown、超时、重试上限（§4.11）
PolicyError               —— 策略构造错误（§4.11）
ResourceArbiter           —— 仲裁入口：observe／summary／direct（§4.11）

装配方特性面（`#[cfg(feature = "assembly")]`）：
PriorityTier              —— §300 的四档优先级；只在 `assembly` 模块公开（§4.6）
AssemblyView              —— summary＋priority；只对装配方（§4.12）
```

**判定**（本块是它们的唯一落点）：

1. 六项聚合读数到 `ResourcePressure` 的映射与滞回（§5.1、§5.2）——规范只给「压力」示例 `HIGH`。
2. `WorkloadClass` 的闭集与文本编码（§4.3）——规范只给四个类名与一个不一致的示例。
3. `ResourceSummary` 的**两项**结论与不可伪造性（§4.4）——§13 说「交给 AI 的仅是」两项。
4. `WorkloadDeclaration` 到 §300 四档的分类（§5.3）——§300 只给顺序，没给分类规则。
5. 压力＋优先级到四档动作的阶梯（§5.4）——§300／§14 都没给这张表。
6. `SuspensionPhase` 的转移、失败、超时、重试上限（§5.5、§5.6）——§14 只给相位与滞回的名字。
7. 四档优先级只在装配方可见的类型层与构建层落法（§4.6、§4.12、§6.5）。

**本块不持有的判定**：OS 探测与进程名分类（本机 Resource Arbiter 组件的本地探测器，装配方落地，
不在零依赖决策核内；§2.4 第 4 条）；`throttle` 的执行语义（P7D）；
checkpoint 的内容与存储（P7D）；后台任务来源（P7D，§299）；模型选择／放置／工具／并行度（第 4 层，§304）；
产品状态与 Shippable（P7A）；Intent 调度（P7B）；§331 的 `Decision`（P7A）。

## 2.3 本块的层间位置

§9.1（`docs/02-工程.md:569-588`）里与本块有关的三条，按「被依赖者 → 依赖者」读：

- **`资源层 (4) → 长期循环 (7)`**（`:578`）：P7 依赖第 4 层是**既有方向**；但 P7C **不**接这条边——
  它不依赖 router／placement／任何第 4 层 crate（§2.1）。这条边由 P7B 的端口实现（装配方）接。
- **`认知层 (6) → 长期循环 (7)`**（`:583`）：P7C 不接；它没有模型／上下文类型。
- **`边界层 (5) → 长期循环 (7)`**（`:581`）：P7C 不接；它不做 policy／capability 判定。

**层内位置**：§7.3 的 `Resource Arbiter ← 无`（`:452`）与「入度为零」（`:458`）是本块的**硬约束**。
P7C 的消费方是 P7D 与装配方；**P7C 不依赖 P7B、不依赖 P7D、不依赖 P7A**。P7B 与 P7C 之间没有
未冻接口（切分 §四第 6 条 `:308-311`）：P7B 只定义端口，P7C 只给结论，装配方在两者之间接线。

**与 §301／§304 的边界**：P7C **不是** Resource Scheduler。§301 要求三个调度器分离
（`docs/spec/05-normative.md:1925-1935`）；P7C 不回答 §304 的四问，也不提供 `ResourceDirective`。
**P7 结束时 §304 仍不完整**，本设计不在 P7C 验收里宣称三层调度已闭合（切分 §1.1 `:36`、
§6.2 第 12 条 `:411`）。

## 2.4 本设计自定的判定（逐条：规范给到哪、代价、边界、落点、收件人）

**这一节是本块的「发明判据」总账（共 15 条自定判定）。** 规范没给、而本设计必须落成实质行为的判定，
都在下表里有一条。**接口细节**（`parse` 的精确大小写、`rank` 的 3/2/1/0、`Debug` 形状、
`checkpoint_failed` 的返回类型）不进这张表，但它们不是规范判定；除此之外，本块没有另定规范级判定。
代码块只是示意，正文措辞才是约束。**§11 的 14 条是跨块设计问题，不是这张 15 条表的子集**；
两处编号不要互相套用（评审按 13 条看会漏掉两条自定判定）。

| # | 本设计自定的判定 | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | **`ResourcePressure` 的量纲（本设计自定）**：`Low`／`Moderate`／`High`／`Critical` 四档；六项聚合读数取**最大值**；默认阈值 `50/75/90`（百分比） | §13 `:720` 只给示例 `resource_pressure = HIGH`；**等级集合、单位、阈值、聚合函数一处未给** | 四档不是规范值；`HIGH` 只对上 `High`；最大值把所有资源同权，Disk IO 尖峰可以与 CPU 饱和同级 | 不覆盖「按资源加权」「按资源分别设阈」「多资源同时中等」；不提供 `Unknown`（见第 2 条） | §4.2、§5.1 | 规范维护者（§13）＋协调者 |
| 2 | **聚合观察的取值域**：六项 `Option<u8>` 百分比（0–100）；`None` 忽略；**全 `None` 保持上一压力**；`>100` 钳到 100 | §13 `:675-684` 只列六项名字；**单位、缺失语义、钳位未给** | 探测器要负责归一化；探测器在 Low 时失效会**保持 Low**（fail-open），不会自动升压 | 不覆盖「读数过期」「探测器整体失联」；不覆盖每进程 CPU／GPU（那是第 4 条） | §4.2、§5.1 | 规范维护者＋装配方 |
| 3 | **`WorkloadClass` 四档闭集**：`InteractiveGame`／`ComputeWorkload`／`Compiler`／`VideoRender`；**不设 `Interactive`**；`parse("INTERACTIVE") == None` | §13 `:708-715` 给四档；`:721` 的输出示例写 `workload_class = INTERACTIVE`，**不在四档里** | 若 `INTERACTIVE` 原意是「汇总后的暴露类」，本设计暴露的是具体类（如 `INTERACTIVE_GAME`），比示例更细 | 不覆盖「四档之外的进程」——未知返回 `None`，不猜；不建第五变体 | §4.3、§8 J5 | 规范维护者（§13）＋P9 |
| 4 | **`workload_class` 的判定来源（本设计自定）**：本机 Resource Arbiter **组件**的本地探测器（属于部署整体，**不是零依赖决策核 `continuum-arbiter`**）读进程名并分类；决策核只接收**已分类**的 `Option<WorkloadClass>`；**进程名／PID 不进 P7C 公开类型** | §13 `:686-706` 允许读进程名／PID，`:708` 说「进程名只在本机 Resource Arbiter 中分类」；**没有进程名→四档的映射表，也没说分类器在哪个 crate** | P7C 无法验证分类是否正确；分类器不在零依赖决策核内，可能各实现不同；未知进程只能 `None` | 不覆盖「分类器把游戏误判为编译」；不覆盖「用户改名规避」；不覆盖「读进程的隐私授权」（A14／P2）；判定器归属待协调者确认（§11 第 4 条） | §4.2、§6.5、§11 第 4 条 | 规范维护者＋协调者＋装配方 |
| 5 | **`workload_class` 可为 `None`**：`ResourceSummary.workload_class: Option<WorkloadClass>` | §13 `:717-722` 的示例总带一个 `workload_class`；**没有说它一定可判** | AI 可能看到「没有类别」；示例的「总是有」不能兑现 | 不覆盖「没有类别时 AI 应如何理解」；不把 `None` 当成某一档 | §4.4、§8 J6 | 规范维护者 |
| 6 | **四档优先级的分类规则（本设计自定）**：输入 `WorkloadDeclaration { user_process, foreground, maintenance }`；**判定顺序**（不是资源优先级）先 `maintenance`、再 `user_process`、再 `foreground`，否则默认 `BackgroundRuntime` | §300 `:1909-1919` 只给 `User workload > Foreground Runtime > Background Runtime > Maintenance` 的**顺序**；**没有说谁声明、怎么判定** | 调用方可以自报 `user_process = true` 逃避限流；P7C 没有独立证据；三个布尔的组合有歧义，只能按上面的判定顺序消解 | 不覆盖「声明造假」；不覆盖「同一任务在不同时刻改变来源」；不覆盖 §299 的 `TaskSource`（见 §2.6） | §4.5、§5.3、§8 J7／J12 | 规范维护者（§300）＋协调者＋P7D |
| 7 | **压力×优先级的动作阶梯（本设计自定）**：见 §5.4 的 4×4 表；`UserWorkload` 恒 `Run`；`ForegroundRuntime` 仅 `Critical` 时 `Throttle`；`BackgroundRuntime` `High` 时 `Throttle`、`Critical` 时 `Suspend`；`Maintenance` `Moderate` 时 `Throttle`、`High`／`Critical` 时 `Suspend` | §300 只给顺序；§14 只给相位；**没有压力→动作的映射** | 切点不是规范值；不同实现会给出不同让路时机；本设计不提供按部署调整这张表的策略面（只调阈值） | 不覆盖「同一压力下不同任务类型的差异」；不覆盖 `throttle` 的量化（P7D） | §5.4、§8 J12 | 规范维护者＋协调者 |
| 8 | **滞回参数与默认值**：`moderate_at=50`、`high_at=75`、`critical_at=90`；`resume_at=Low`；`cooldown_millis=30_000`；升压**立即**生效，降压只在 `raw <= resume_at` 且 cooldown 已过时生效 | §14 `:746-754` 只给 `suspend threshold`／`resume threshold`／`cooldown` 三个名字；**没有数值、没有升降方向、没有默认值** | 默认值可能过慢或过度挂起；`resume_at=Low` 很保守，可能让后台长时间停着；没有按资源分别滞回 | 不覆盖「临界点附近的高频抖动」——本设计只保证不因降压立即回弹；不覆盖「用户手动强制恢复」 | §4.11、§5.2、§8 J4 | 规范维护者＋协调者 |
| 9 | **`SuspensionPhase` 转移图（当前读法）**：`Suspend` 可从 `Running` 或 `Throttled` 直接进 `Checkpointing`；`Throttled` **不是**必经点；`Checkpointing` 不被压力动作打断；失败／超时→`Throttled`；成功→`Suspended`；`Resume`→`Running` | §14 `:735-743` 列出了相位序列 `Running → Throttled → Checkpointing → Suspended → Resume`；**没有说箭头是否强制、是否允许跳过、谁能打断** | 若规范维护者读作「必须经过 `Throttled`」，P7C 的动作序列要改（先 `Throttle` 再 `Suspend`）；本设计选择「相位集合＋受控转移」，**未裁定前 J8 按当前读法写** | 不覆盖「规范要求先降速再 checkpoint」；不覆盖「checkpoint 期间的资源压力变化」——本设计一律不打断 | §5.5、§8 J8、§8.4 第 9 条 | 规范维护者（§14）＋协调者＋P7D |
| 10 | **失败结果类型（本设计自定）**：`SuspensionError::{CheckpointFailed, CheckpointTimedOut, NotCheckpointing { phase }}`；失败／超时后相位回 `Throttled` | §14 与切分 §2.3.2 第 2 条 `:160-164` 要求「失败返回错误并回 `Throttled`」，**没有给错误类型名、臂、是否带原因** | P7D 要自己映射错误；本设计不保存失败原因（避免原始信息）；错误名不是规范值 | 不覆盖「失败原因的分类」；不覆盖「重试策略的可观察原因」 | §4.9、§5.6、§8 J9／J10 | P7D＋规范维护者 |
| 11 | **超时／重试／时钟**：调用方给 `now_millis`；`checkpoint_timeout_millis` 默认 `60_000`；`max_checkpoint_attempts` 默认 `3`；超时在 `direct`／`checkpoint_succeeded` 时检查；**不建定时器线程**；`Run`（从 `Throttled`）或 `Resume` 后重置尝试计数，`checkpoint_succeeded` 不重置；**`Resume` 的 cooldown 用 `SuspensionState.last_transition_millis`（状态时钟），不用 `ResourceArbiter.last_pressure_change_millis`（压力时钟）** | §14 只给 cooldown；**没有重试上限、超时、时钟归属** | 若调用方从不调用 `direct`，`Checkpointing` 不会被自动推进；「不卡死」只在每次调用时成立；晚到的 `checkpoint_succeeded` 会被超时拒绝；压力已 `Low` 但挂起不足 cooldown 时不会恢复 | 不覆盖「调用方完全不轮询」；不覆盖「跨进程重启后的超时」 | §4.8、§5.5、§5.6、§8 J8／J10／J11 | P7D＋协调者 |
| 12 | **四档优先级的暴露机制**：`PriorityTier` 定义在**私有模块** `priority`；只有 `#[cfg(feature = "assembly")]` 的 `assembly` 模块公开它；默认构建的公开面没有 `PriorityTier`／`AssemblyView`；`ResourceSummary`／`ResourceDecision` 不含优先级 | §300 `:1921` 是 **SHOULD**；切分 §2.3.2 第 2 条 `:155-165` 要求「只对装配方可见」；A14／OPEN-012 说强制点未定 | `assembly` 特性不是安全边界：同一构建图里启用特性后别的 crate 也可能命名它；跨 crate 可见性 Rust 无法按消费方区分；P7D 只能靠源码纪律 | 不覆盖「恶意消费方启用特性」；不覆盖「P7D 把 `ResourceDecision` 转发给 AI」；规范级强制仍在 P9 | §4.6、§4.12、§6.5、§8 J6／J7 | P9＋协调者＋规范维护者 |
| 13 | **纯决策核＋本地探测器**：P7C 只做决策，不读 OS；六项聚合读数与 `workload_class` 由本机 Resource Arbiter 组件的本地探测器（装配方落地）填入 | §13 `:675-684` 说 Arbiter 读聚合状态；**没说探测器属于哪个 crate**；§7.3 要求零依赖 | 两个部件要接线；探测器失效／过期时 P7C 只能保持上一压力；探测器质量不在 P7C | 不覆盖「探测器读数造假」（公开输入面可伪造，见 §4.13）；不覆盖「读取频率」；不覆盖「探测器自身对模型的依赖」（装配方要保证不接） | §4.2、§7.3、§11 第 9 条 | 装配方＋协调者 |
| 14 | **零持久化**：`SuspensionState`／`ResourceArbiter` 都是内存值；不建表、不占迁移号、不实现 `Serialize`／`Deserialize` | §14 没有持久化要求；切分 §四第 2 条 `:265-276` 说「无表则不占号」 | 重启后压力与相位丢失；P7D 若要恢复任务，必须自己决定如何重建状态；本 crate 不提供反序列化入口 | 不覆盖「重启后已挂起任务的恢复」；不覆盖「相位审计」 | §10、§11 第 8 条 | P7D＋协调者 |
| 15 | **`ResourceDecision.action: Option<ResourceAction>`**：`None` 表示「当前相位已经是所需状态，无转移」；`ResourceAction` 仍恰四变体 | 切分 §2.3.2 第 2 条 `:158-159` 说四档动作；**没有说幂等时返回什么** | P7D 必须把 `None` 当「不动作」，不能当 `Run`；多一个 `Option` 分支 | 不覆盖「P7D 误读 `None`」；不覆盖「需要显式 no-op 动作」的消费方 | §4.7、§4.10、§8 J13 | P7D |

## 2.5 切分 §四 六条横切约束的逐条落实

| 切分 §四 | 要求（要点） | 本设计的落点 | 可观察判据 |
|---|---|---|---|
| **第 1 条**（`:253-264`） | 不新增反向边；四块各自在 `ALLOWED` 加自己一行；既有行不因 P7 增枚，唯一例外是装配方 `continuum-runtime` | §2.1：`("continuum-arbiter", &[])`；不碰 runtime 行；P7D 落地时加 `continuum-supervisor` 行 | J1(i)：`cargo tree` 无子节点、`ALLOWED` 行精确为空、`every_crate_depends_only_on_its_allowed_set` 绿；J1(ii)：**最终树** `dependency_direction` 全绿（前置：P7D 已完成 P6D 合并门 rebaseline／退役） |
| **第 2 条**（`:265-276`） | 迁移号段；无表不占号 | §10：零表、零迁移；不改 `runtime_migrations()`／`expected_migrations()`／`startup.rs` | J15：`Cargo.toml` 无 `continuum-persist`；`src/` 无 `Migration`／`Tx`；runtime 三处无本 crate |
| **第 3 条**（`:277-285`） | 同一件事两个词汇表：§300 四档、§13 四档、`INTERACTIVE` 笔误、`Suspended` 三处 | §2.6 的同名异物表；§4.3 不设 `Interactive`；§4.8 的 `SuspensionPhase` 不复用 `NodeState`／`IntentStatus` | J5：四档闭集；`parse("INTERACTIVE")==None`；J14：公开类型无原始进程信息 |
| **第 4 条**（`:286-293`） | 工具结果／Evidence 的消费；P7 只消费 `Evidence`／`CompletionVerdict` | P7C **不消费**任何工具结果或证据；没有 `continuum-verify` 边 | J1／J16：零依赖；`src/` 无 `Evidence`／`CompletionVerdict` |
| **第 5 条**（`:294-296`） | 依赖边只登记实际用到的；`ALLOWED` 与实际依赖精确一致 | §2.1：零边；无 dev 边 | J1(i)：`cargo tree --edges all` 只有本 crate；`ALLOWED` 空 |
| **第 6 条**（`:297-311`） | P7C → P7D：P7C 先冻一页，P7D 对着写；P7B 与 P7C 无未冻接口 | §4 冻结论／控制／装配三面；§7.1 给 P7D 的消费清单；§7.2 给装配方的端口消费；P7C 不依赖 P7B | J6／J7／J13：三面形状；P7D 只依赖 `ResourceDecision`／`SuspensionState`；P7B 端口不引入 P7C 类型 |

## 2.6 同名异物（**都不是**同一个概念）

| 已有的东西 | 它是什么 | 实测位置 | 本设计的处置 |
|---|---|---|---|
| `Suspended` | `NodeState::Suspended`／`IntentStatus::Suspended`／§14 的协作式挂起相位 | `crates/continuum-graph/src/node.rs:63`；`crates/continuum-semantics/src/intent.rs:77`；`docs/spec/01-concepts.md:741` | P7C 自建 `SuspensionPhase`；**不复用** `NodeState`／`IntentStatus`；只把「挂起」结果映射到 P7D 的既有任务状态（P7D 的事） |
| `priority` | `Intent.priority`（不透明）／§302 的输入之一／§78 的七因子／§300 的四档资源优先级／§79 的四档交互优先级 | `crates/continuum-semantics/src/intent.rs:300`；`docs/spec/05-normative.md:1950`；`docs/spec/02-positioning.md:635-651`、`:655-669`；`docs/spec/05-normative.md:1907-1921` | P7C 只用 §300 的四档，内部名 `PriorityTier`；**不解析** `Intent.priority`；**不建** §79 的第二套 `Priority`；P7B 不读 |
| `workload_class` | §13 的四档进程类／输出示例里的 `INTERACTIVE` | `docs/spec/01-concepts.md:708-715`、`:717-722` | P7C 只用四档；`INTERACTIVE` 不设变体；`parse("INTERACTIVE")==None`；报规范维护者（§11 第 2 条） |
| `TaskSource` | §299 的四类后台任务来源（`authorized Intent`／`policy`／`maintenance schedule`／`product drive`） | `docs/spec/05-normative.md:1892-1903` | P7C 的 `WorkloadDeclaration` **不是** `TaskSource`；`maintenance` 布尔不是「maintenance schedule」；P7D 拥有 `TaskSource` 与映射（§7.1） |
| `Decision` | §331 的决策单／策略的 `Allow/Deny/RequireApproval` | `crates/continuum-semantics/src/decision.rs:39-44` | P7C 的 `ResourceDecision` 是仲裁结论，不是 §331；不写 `decision.required`；P7A 用 §331 |
| `ResourceDirective` | P7B 的 §304 四槽不透明载体 | P7B 设计 `:832-936` | P7C **不定义** `ResourceDirective`，不填四槽，不回答 §304；P7B 的端口实现消费 P7C 结论（§7.2） |
| `ResourceDecision` | P7C 的仲裁结论（summary／action／phase）／P7B 的 `ResourceDirective`（§304 四槽） | §4.10；P7B 设计 `:832-936` | 两者不是同一个值：P7C 不给 Node／模型／工具／并行度；P7B 不给压力／类别／挂起相位；装配方在端口实现里把两者分开使用 |
| `Budget` | `continuum-budget` 的五维记账（`DimensionKind`／`Dimensions`／`check`） | `crates/continuum-budget/src/lib.rs:1-68`、`dimensions.rs:14`、`validator.rs:73` | P7C **不依赖** budget，不把预算当资源仲裁；预算读数是否影响压力见 §11 第 13 条 |
| `SuspensionPhase` vs `Checkpointing` | §14 的相位名／checkpoint 的执行动作 | §14 `:735-743` | P7C 只拥有相位；**checkpoint 的内容与执行**是 P7D 的（§11 第 8 条） |

---

# 3. 规范依据逐条

**这一节的写法**：每条先给规范原文的落点，再给「规范给了什么 / 规范没给什么 / 本设计怎么落」。
**没有一条判据是本节发明的**——本节的每一条都指回一个规范节；规范没给的部分在 §2.4 另记。

## 3.1 §300 Resource Arbiter（`docs/spec/05-normative.md:1907-1921`）

**原文（逐字，`:1909-1921`）**：

> 资源优先级：
> ```
> User workload
> >
> Foreground Runtime
> >
> Background Runtime
> >
> Maintenance
> ```
> Resource Arbiter SHOULD 只向 AI 暴露最小化资源结论。

**规范给了什么**：①四档的**名字与顺序**；②对 AI 的暴露是 **SHOULD**「最小化」；③组件名
`Resource Arbiter`。

**规范没给什么**：①四档的**分类规则**（谁算 User workload？谁算 Foreground Runtime？）；②四档之间的
**动作差异**（高压力时哪一档先让路）；③「最小化」的**强制机制**（A14／OPEN-012）；④`SHOULD` 的
偏离条件。

**本设计怎么落**：四档只作为 `PriorityTier` 的内部/装配方面存在（§4.6）；分类规则见 §5.3；
动作阶梯见 §5.4；AI／后台只拿 `ResourceSummary` 两项（§4.4）；四档优先级不进 `ResourceSummary`／
`ResourceDecision`（§6）。**规范级强制仍归 P9**（§11 第 1、10 条）。

## 3.2 §13 Resource Arbiter（`docs/spec/01-concepts.md:669-724`）

**原文（逐字节选，`:671-724`）**：

> 后台任务不能影响用户正常使用机器。
> Resource Arbiter 独立于 AI 模型运行。
> 默认只读取聚合资源状态：
> ```
> CPU
> RAM
> GPU
> VRAM
> Disk IO
> Network
> ```
> 当资源明显竞争时，才最小化读取高占用进程。
> 允许读取：
> ```
> process name
> CPU usage
> GPU usage
> PID
> ```
> 默认不读取：
> ```
> command line
> working directory
> window title
> screen
> open files
> keyboard input
> ```
> 进程名只在本机 Resource Arbiter 中分类：
> ```
> INTERACTIVE_GAME
> COMPUTE_WORKLOAD
> COMPILER
> VIDEO_RENDER
> ```
> 交给 AI 的仅是：
> ```
> resource_pressure = HIGH
> workload_class = INTERACTIVE
> ```
> 避免把用户行为直接送进模型。

**规范给了什么**：①**独立于 AI 模型**（`:673`）；②默认读**六项聚合状态**（`:675-684`）；
③只在明显竞争时读高占用进程（`:686`）；④允许读进程名／CPU／GPU／PID，默认不读命令行／工作目录／
窗口标题／屏幕／打开文件／键盘（`:688-706`）；⑤进程名在**本机**分类为四档（`:708-715`）；
⑥交给 AI 的仅是 `resource_pressure` 与 `workload_class`（`:717-724`）。

**规范没给什么**：①压力的**量纲/等级/阈值**（只有 `HIGH` 示例）；②六项聚合的**单位与聚合函数**；
③进程名→四档的**映射表**；④`INTERACTIVE` 与四档的关系（笔误，§11 第 2 条）；⑤「最小化」的
**强制点**；⑥`workload_class` 是否总是可判。

**本设计怎么落**：P7C 是**零依赖的决策核**（§2.1），不读 OS、不调模型；六项聚合读数与已分类的
`WorkloadClass` 由本机 Resource Arbiter 组件的本地探测器填入 `ResourceObservation`（§4.2、§11 第 9 条）。
压力四档与阈值是本设计
自定（§2.4 第 1 条）；四档进程类严格照 §13，`INTERACTIVE` 不设变体（§4.3）。对 AI 的结论只有
`ResourceSummary` 两项（§4.4）；原始进程信息不进公开类型（§8 J14）。**§13 的「独立于 AI」由零依赖
与无模型路径共同兑现**（J1／J2）。

## 3.3 §14 Cooperative Suspension（`docs/spec/01-concepts.md:728-754`）

**原文（逐字，`:730-754`）**：

> 后台任务不能粗暴 kill。
> 状态：
> ```
> Running
>  ↓
> Throttled
>  ↓
> Checkpointing
>  ↓
> Suspended
>  ↓
> Resume
> ```
> 支持 hysteresis：
> ```
> suspend threshold
> resume threshold
> cooldown
> ```
> 避免资源利用率在临界点反复启停。

**规范给了什么**：①**不能 kill**（`:730`）；②四相位与 `Resume` 箭头（`:732-744`）；③滞回三要素
（`:746-752`）；④滞回的目的（`:754`）。

**规范没给什么**：①转移的**守卫条件**（谁能进 `Checkpointing`、`Checkpointing` 能否被压力打断）；
②失败结果类型与失败后的相位（切分补充了「回 `Throttled`」，但没有类型）；③重试上限／超时；
④checkpoint 的内容、执行者、存储；⑤`Throttled` 是否是必经点。

**本设计怎么落**：§5.5 给转移表；§5.6 给失败／超时／重试；`SuspensionError` 是本设计命名的失败结果
（§2.4 第 10 条）。**P7C 不执行 checkpoint**；它只把「已要求挂起但尚未完成」表达为 `Checkpointing`，
P7D 执行 checkpoint 后回报成功／失败／超时（§7.1）。**没有 `kill` 动作**（§9、J17）。

## 3.4 《工程》§7.1／§7.2／§7.3／§7.4

| 节 | 原文/要点 | 本设计的落点 |
|---|---|---|
| §7.1（`docs/02-工程.md:429`） | `Resource Arbiter` 的规范依据是 `§300 §13 §14` | §3.1–§3.3；本设计不引入第四个规范节作为其定义 |
| §7.2（`:441`） | 「Resource Arbiter 独立于 AI 模型运行……不能依赖第 4 层的任何组件」 | §2.1 零依赖；§7.3 无 router／placement 边；J1／J2 |
| §7.3（`:452`、`:458`） | 「Resource Arbiter ← 无（独立于模型层）」「入度为零」 | §2.1 的 `ALLOWED == []`；§2.3 |
| §7.4 判据 3（`:465`） | 「Resource Arbiter 在模型层完全不可用的情况下仍能限制后台负载」 | J1＋J2；**只覆盖这一条，不覆盖 P7C 的其余交付**（§8.5、§11 第 5 条） |

**§7.4 的覆盖范围（据实）**：四条判据里，判据 1／2／4 落 P7A，判据 3 落 P7C
（切分 §七第 12 条 `:462`）。判据 3 只测「模型层完全不可用仍能限流」；它**不测**
`ResourcePressure`／`WorkloadClass` 的形状、四档优先级的分类、`Checkpointing` 两阶段、失败回退、
最小暴露面、不 kill。**P7C 不修改 §7.4**；其余交付由本设计自报 J3–J17（§8.5、§11 第 5 条）。

## 3.5 A14／OPEN-012：最小暴露面（`docs/02-工程.md:345`；`docs/01-总纲.md:1698-1702`、`:1881-1884`）

**原文（要点）**：§300 说 SHOULD 只向 AI 暴露最小化资源结论；SHOULD 意味着可以偏离；§13 描述设计
意图（只传 `pressure` 与 `workload_class`），但它不是规范段；当前缺少强制机制，无法保证实现不会把
进程名、路径等超出最小暴露面的信息传入 Context。

**本设计怎么落**：

1. **公开结论类型只有两项**：`ResourceSummary` 恰 `resource_pressure`＋`workload_class`
   （§4.4）；没有优先级、没有进程名／PID、没有逐进程 CPU／GPU。
2. **四档优先级只对装配方**：`PriorityTier` 在私有模块、`assembly` 特性下才公开（§4.6、§4.12）；
   `ResourceSummary`／`ResourceDecision` 不含它。
3. **原始进程信息私有**：`ResourceObservation` 只有六项聚合读数与已分类的 `WorkloadClass`；
   `ResourceSummary`／`ResourceDecision`／`SuspensionState` 的 `Debug` 面也不带进程名／PID（J14）。
4. **规范级强制不关闭**：本设计只交可观察的结构面；**A14／OPEN-012 的收口在 P9**（主树
   `docs/superpowers/p8-p10-scope.md:63`、`:68`）。P7C 不声称 SHOULD 已变成 MUST（§11 第 1 条）。

## 3.6 与 `continuum-budget`／router／placement 的边界

**`continuum-budget` 是记账，不是资源仲裁**：它拥有预算树、三个角色、账本、`BudgetValidator` 与
探索预算（`crates/continuum-budget/src/lib.rs:1-68`）；`DimensionKind` 是 §333 的五维
（`dimensions.rs:14`），`check` 在 `validator.rs:73`。P7C **不依赖**它、不读预算、不把预算余额当压力。
「预算够不够」与「机器让不让路」是两件事（切分 §2.2 `:114`）。预算读数是否应影响压力/动作，见 §11 第 13 条。

**router／placement 是第 4 层碎片，不是 P7C**：`RoutingRequest`（`crates/continuum-model-registry/src/router.rs:199`）、
`RankedExecutionCandidates`（`:405`）、`rank`（`:523`）；`PlacementRequest`（`crates/continuum-node/src/placement.rs:202`）、
`place`（`:362`），且 `place` 在 `src/` 里零生产调用点（`:357-358`）。P7C **不依赖**、不转调、不重造；
这些是 P7B 的 `ResourceDirectivePort` 实现（装配方）的转调对象（切分 §1.1 `:29-36`、§6.2 第 12 条 `:411`）。

## 3.7 §301／§304：P7C 不是 Resource Scheduler

§301 要求 Intent Scheduler／Graph Scheduler／Resource Scheduler 三者分离
（`docs/spec/05-normative.md:1925-1935`）；§304 的 Resource Scheduler 回答四问
（`:1973-1982`）。P7C 不回答其中任何一问：

- 不选 Node 放置（无 `place`）；
- 不选模型（无 `model`）；
- 不选工具（无 tool path）；
- 不给并行度（无 `parallelism`）。

P7C 给的是**让路判断**：压力、类别、优先级（装配方）、四档动作、挂起相位。**§304 的闭合不在 P7C**；
P7B 的端口只承载四问，工具选择与并行度仍在第 4 层缺口里（切分 §6.2 第 12 条 `:411`）。
**P7 结束时 §304 仍不完整**，本设计不把它写成已闭合。

---

# 4. 类型与接口

**这一节是 P7C 对 P7D／装配方／AI 的冻结面。** 代码块是示意，正文措辞才是约束；字段名、变体名、
访问器名以本节为准。所有类型都在 `continuum-arbiter` 内定义，**不引用任何 workspace 内 crate 的类型**。

## 4.1 模块落点（源码面判据的语料）

| 文件 | 内容 |
|---|---|
| `src/lib.rs` | 只做 `mod`／`pub use`／`#[cfg(feature = "assembly")] pub mod assembly`；不定义类型 |
| `src/pressure.rs` | `ResourcePressure`、`ResourceObservation`、原始压力计算 |
| `src/workload.rs` | `WorkloadClass`（四档闭集、`ALL`、`as_str`／`parse`） |
| `src/summary.rs` | `ResourceSummary`（两项结论；字段私有） |
| `src/declaration.rs` | `WorkloadDeclaration`（三个布尔） |
| `src/priority.rs` | `PriorityTier`（**私有模块**；只有 `pub(crate) fn classify` 与内部使用） |
| `src/assembly.rs` | `#[cfg(feature = "assembly")]`：`pub use crate::priority::PriorityTier`、`AssemblyView`、`ResourceArbiter::assembly_view` |
| `src/action.rs` | `ResourceAction` |
| `src/state.rs` | `SuspensionPhase`、`SuspensionState`、`SuspensionError` |
| `src/policy.rs` | `ArbiterPolicy`、`PolicyError` |
| `src/arbiter.rs` | `ResourceArbiter`、`ResourceDecision`、动作阶梯、`direct` |

**测试位置**：全部放 `crates/continuum-arbiter/tests/`，**不放 `src/` 的 `#[cfg(test)]`**。这样
「源码面判据」的语料（`src/`）里没有测试构造，`grep` 的零命中断言不会被测试代码污染（同 P6A §8.2 的
纪律）。测试只用 `std`，没有 dev-dependencies。

**`tests/assembly.rs` 的门控**：`Cargo.toml` 必须有
`[[test]] name = "assembly" required-features = ["assembly"]`（§2.1 的形状）。没有它，默认
`cargo test -p continuum-arbiter` 也会把 `tests/assembly.rs` 当集成测试编译，而默认构建没有
`continuum_arbiter::assembly`，全套测试会因编译失败而红。J15 专门验这道门控。

## 4.2 `ResourcePressure`、`ResourceObservation`

```rust
/// §13 的压力等级。**规范只给示例 HIGH**；四档是本设计自定（§2.4 第 1 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePressure {
    Low,
    Moderate,
    High,
    Critical,
}

impl ResourcePressure {
    pub fn as_str(&self) -> &'static str;   // "LOW" / "MODERATE" / "HIGH" / "CRITICAL"
    pub fn parse(s: &str) -> Option<Self>;  // 精确大写；未知 → None
}

/// 六项**聚合**读数＋已分类的负载类别。**不是**每进程读数；没有进程名／PID。
/// **聚合 CPU／GPU 允许；逐进程 CPU／GPU 禁止**（§6.1、J14）。
/// 六项单位是百分比（0–100）；`None` 表示该项不可读；`>100` 在 observe 里钳到 100。
/// `workload_class` 由本机 Resource Arbiter 组件的本地探测器分类后填入；P7C 不读进程名（§2.4 第 4 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceObservation {
    pub cpu: Option<u8>,
    pub ram: Option<u8>,
    pub gpu: Option<u8>,
    pub vram: Option<u8>,
    pub disk_io: Option<u8>,
    pub network: Option<u8>,
    pub workload_class: Option<WorkloadClass>,
}
```

**为什么 `ResourceObservation` 是公开的**：它是本机 Resource Arbiter 组件探测器的**输入面**，不是 AI
的结论面。它只有六项聚合读数与一个类，**没有进程名／PID／逐进程 CPU／GPU**（§13 `:686-706` 允许读
进程名，但本设计不让它进入公开类型；分类在探测器里完成，P7C 只收结果）。`ResourceObservation` 的 `Debug` 会
打印百分比，这是聚合读数，不是用户行为；它也不实现 `Serialize`。

## 4.3 `WorkloadClass`

```rust
/// §13 的四档进程类（`:708-715`）。**闭集四枚；没有 INTERACTIVE**（`:721` 是笔误，§11 第 2 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadClass {
    InteractiveGame,   // "INTERACTIVE_GAME"
    ComputeWorkload,   // "COMPUTE_WORKLOAD"
    Compiler,          // "COMPILER"
    VideoRender,       // "VIDEO_RENDER"
}

impl WorkloadClass {
    pub const ALL: [WorkloadClass; 4] = [
        WorkloadClass::InteractiveGame,
        WorkloadClass::ComputeWorkload,
        WorkloadClass::Compiler,
        WorkloadClass::VideoRender,
    ];
    pub fn as_str(&self) -> &'static str;
    pub fn parse(s: &str) -> Option<Self>;  // 精确大写；"INTERACTIVE" → None
}
```

**为什么不设第五个 `Unknown`**：§13 的四档是闭集；未知进程不是「第五档」，而是「没有类别」。
本设计用 `Option<WorkloadClass>` 表达缺席（§4.4），不建第五变体。**代价**：消费方要处理 `None`；
**边界**：`None` 不能被当作 `InteractiveGame` 或任何一档。

**`INTERACTIVE` 的处置**：`parse("INTERACTIVE")` 返回 `None`，不映射到 `InteractiveGame`。这让笔误
在测试里可观察（J5），也防止一个不存在的第五档悄悄进入类型。若规范维护者确认 `INTERACTIVE` 是
「汇总后的暴露类」，那要另立一个暴露投影，不是给本枚举加变体（§11 第 2 条）。

## 4.4 `ResourceSummary`：对 AI／后台的两项结论

```rust
/// §13 `:717-724`：交给 AI 的仅是 resource_pressure 与 workload_class。
/// 字段私有；唯一构造点是 `ResourceArbiter::observe`／`summary`（不提供 `Default`／`Deserialize`）。
/// **没有优先级、进程名、PID、逐进程 CPU／GPU。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceSummary {
    resource_pressure: ResourcePressure,
    workload_class: Option<WorkloadClass>,
}

impl ResourceSummary {
    pub fn resource_pressure(&self) -> ResourcePressure;
    pub fn workload_class(&self) -> Option<WorkloadClass>;
}
```

**为什么字段私有**：`ResourceSummary` 是「最小暴露面」的结论，不能由装配方或 AI 用结构体字面量
伪造一个「低压力」；唯一的生产者是 `ResourceArbiter`。`Copy` 允许装配方把快照交给 P7B 的端口实现
（§7.2），不产生第二个构造点。**不实现 `Serialize`／`Deserialize`／`Display`**：P7B 的
`BackgroundSuitability` 是字符串载体，由装配方用两个访问器投影；本类型不提供绕过访问器的序列化面。

**`resource_pressure` 是整机压力，不是这份工作的压力**；`workload_class` 是整机观测到的**竞争进程类**
（来自 `ResourceObservation.workload_class`），不是 `WorkloadDeclaration` 对「这份工作」的分类。
两者不要混：前者回答「机器紧不紧」，后者回答「该不该让路」。

**`workload_class` 是 `Option`**：§13 的示例总带一个值，但默认只读聚合状态时，P7C 没有进程名，
无法分类；`None` 是「未分类」，不是第五档（§2.4 第 5 条）。**代价**：AI 可能只拿到压力；**边界**：
不能把 `None` 当成「无用户行为」或「安全」。

## 4.5 `WorkloadDeclaration`：四档优先级的分类输入

```rust
/// 四档优先级的分类输入。**不是** §299 的 `TaskSource`；三个布尔描述的是「这份工作是谁的」，
/// 不是「它从哪个授权来源来」。分类规则见 §5.3。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkloadDeclaration {
    /// 用户直接在机器上操作的外部工作负载（不是 runtime 任务）。
    pub user_process: bool,
    /// 用户在等待其结果的 runtime 任务。
    pub foreground: bool,
    /// 维护计划中的任务。
    pub maintenance: bool,
}
```

**为什么是三个布尔而不是第四个四值枚举**：§300 已经有四档；再造一个四值 `WorkloadOrigin` 会把
「同一件事两个词汇表」再犯一次（切分 §四第 3 条 `:277-285`）。三个布尔是**事实输入**，四档是 P7C 的
**分类结果**；映射函数只在 `priority.rs` 里写一次（§5.3）。**代价**：组合有歧义，按
`maintenance` ＞ `user_process` ＞ `foreground` ＞ 默认背景消解；**边界**：调用方可以自报
`user_process = true` 逃避限流，P7C 没有独立证据（§11 第 3 条）。

**`maintenance` 不是 `TaskSource::MaintenanceSchedule`**：`TaskSource` 是 §299 的四类任务来源
（P7D）；`WorkloadDeclaration.maintenance` 是「这份工作是否属于维护计划」这一条事实。P7D 负责把
`TaskSource` 投影到三个布尔（§7.1）；本 crate 不解释 §299。

## 4.6 `PriorityTier`：§300 四档，只对装配方

```rust
/// §300 的四档优先级（`:1909-1919`）。**定义在私有模块 `priority`**；默认构建的公开面没有它。
/// 只有 `#[cfg(feature = "assembly")]` 的 `assembly` 模块把它 re-export 给装配方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityTier {
    UserWorkload,       // "User workload"
    ForegroundRuntime,  // "Foreground Runtime"
    BackgroundRuntime,  // "Background Runtime"
    Maintenance,        // "Maintenance"
}

impl PriorityTier {
    #[cfg(feature = "assembly")]
    pub fn as_str(&self) -> &'static str;  // 逐字 §300 的名字（含空格）
    #[cfg(feature = "assembly")]
    pub fn rank(&self) -> u8;              // User=3, Foreground=2, Background=1, Maintenance=0
}
```

**分类规则（本设计自定，§2.4 第 6 条）**：

```
maintenance == true                  → PriorityTier::Maintenance
else user_process == true            → PriorityTier::UserWorkload
else foreground == true              → PriorityTier::ForegroundRuntime
else                                 → PriorityTier::BackgroundRuntime
```

**`rank` 不是 `Ord`**：四档的顺序只在装配方展示/测试里用；内部动作阶梯用 `match`，不依赖枚举的
派生顺序。这样不会因为变体声明顺序变化而悄悄改变让路顺序。

## 4.7 `ResourceAction`：四档动作

```rust
/// 四档动作（切分 §2.3.2 第 2 条 `:158-159`）。恰四变体；没有 `Kill`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAction {
    Run,
    Throttle,
    Suspend,
    Resume,
}

impl ResourceAction {
    pub fn as_str(&self) -> &'static str;   // "run" / "throttle" / "suspend" / "resume"
    pub fn parse(s: &str) -> Option<Self>;
}
```

**`None` 不是第五动作**：`ResourceDecision.action` 是 `Option<ResourceAction>`；`None` 表示「当前相位
已经是所需状态，没有转移」（§2.4 第 15 条）。P7D 必须把 `None` 当「不动作」，不能当 `Run`。

**为什么没有 `Kill`**：§14 `:730` 逐字「后台任务不能粗暴 kill」；本设计只做协作式挂起。
`ResourceAction` 的穷尽 `match` 是 J17 的照片：加第五变体即编译不过。

## 4.8 `SuspensionPhase` 与 `SuspensionState`

```rust
/// §14 的四相位（`:735-743`）。**不复用** `NodeState`／`IntentStatus`（§2.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionPhase {
    Running,
    Throttled,
    Checkpointing,
    Suspended,
}

impl SuspensionPhase {
    pub fn as_str(&self) -> &'static str;   // "Running" / "Throttled" / "Checkpointing" / "Suspended"
    pub fn parse(s: &str) -> Option<Self>;
}
// `Resume` 不是第五相位：§14 `:743` 的 Resume 是 `Suspended → Running` 的动作，
// 落在 `ResourceAction::Resume`；`SuspensionPhase` 恰四相位。

/// 每个后台/维护任务的挂起状态机。P7D 持有；P7C 不集中管理任务表。
/// 字段私有；唯一构造点 `new()`；不实现 `Default`／`Deserialize`。
/// `last_transition_millis` 在每一次**被接受的相位转移**时更新（含进入 `Suspended`）；
/// `Resume` 的 cooldown 从它起算，不从压力时钟起算（§5.5）。
#[derive(Debug, PartialEq, Eq)]
pub struct SuspensionState {
    phase: SuspensionPhase,
    attempts: u32,
    last_transition_millis: u64,
    checkpoint_deadline_millis: Option<u64>,
}

impl SuspensionState {
    pub fn new() -> Self;  // Running, attempts=0, last_transition=0, deadline=None
    pub fn phase(&self) -> SuspensionPhase;
    /// checkpoint 的截止时刻；`Some` 只在 `Checkpointing`。P7D 用它安排定时器。
    pub fn checkpoint_deadline_millis(&self) -> Option<u64>;

    /// 只允许从 `Checkpointing` 调用。截止前成功 → `Suspended`（Ok）；
    /// 已过截止 → 相位回 `Throttled` 并返回 `Err(CheckpointTimedOut)`（晚到成功也拒绝）。
    pub fn checkpoint_succeeded(&mut self, now_millis: u64)
        -> Result<SuspensionPhase, SuspensionError>;

    /// 只允许从 `Checkpointing` 调用。相位回 `Throttled`，返回 `CheckpointFailed`；
    /// 错相位返回 `NotCheckpointing { phase }`。**总是返回错误（没有 Ok 路径）**。
    pub fn checkpoint_failed(&mut self, now_millis: u64) -> SuspensionError;

    /// 定时器钩子；只允许从 `Checkpointing` 调用。相位回 `Throttled`，
    /// 返回 `CheckpointTimedOut`；错相位返回 `NotCheckpointing { phase }`。
    /// `direct` 也会在截止时自动做同一转移。**总是返回错误（没有 Ok 路径）**。
    pub fn checkpoint_timed_out(&mut self, now_millis: u64) -> SuspensionError;
}
```

**P7C 不集中管理任务表**：`SuspensionState` 由 P7D 按任务持有并传入 `direct`；本 crate 不定义
`TaskId`、不存任务映射、不读任务存储。这样 P7C 不需要知道 P7D 的任务身份，也不会把后台任务表
变成第二套调度面。

**Resume 的 cooldown 时钟（写死）**：`Suspended → Running` 的 cooldown 只看
`SuspensionState.last_transition_millis`——即「进入 `Suspended` 的时刻」；`ResourceArbiter` 的
`last_pressure_change_millis` 只服务 §5.2 的降压，不参与 `Resume`。两种时钟在 J8 的照片里给出的
结果不同：`checkpoint_succeeded(0)` 后 `observe(Low,30_000)`，压力时钟差为 0，状态时钟差为
30_000；本设计取后者（§5.5、§8 J8）。

**`attempts` 不公开**：它是内部重试预算；P7D 通过「`direct` 不再发出 `Suspend`」观察上限（J11），
不需要读计数。**`last_transition_millis` 不公开**：cooldown 由 `direct` 内部判断；P7D 只按
`checkpoint_deadline_millis` 安排超时。派生 `Debug` 会打印相位／尝试数／最后转移时刻／截止，
仍不含进程名／PID（§4.13）。

## 4.9 `SuspensionError`：失败结果类型（本设计命名）

```rust
/// checkpoint 失败／超时／错相位的错误。**不是** `PolicyError`；也不是 P7D 的 checkpoint 错误。
/// `CheckpointFailed`／`CheckpointTimedOut` 返回时，相位已经回到 `Throttled`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionError {
    CheckpointFailed,
    CheckpointTimedOut,
    NotCheckpointing { phase: SuspensionPhase },
}
```

**`Display` / `Error`**：手写实现（`std` only；不引入 `thiserror`）。`CheckpointFailed` 与
`CheckpointTimedOut` 不携带原因字符串，避免把 checkpoint 内部信息带进公开面；P7D 若要记录原因，
记在它自己的 checkpoint 记录里，不进 P7C 的类型。

**返回类型**：`checkpoint_failed`／`checkpoint_timed_out` 直接返回 `SuspensionError`，**不是**
`Result`——它们没有成功路径，`Result` 会让 P7D 误以为存在 `Ok`。`checkpoint_succeeded` 仍返回
`Result<SuspensionPhase, SuspensionError>`，因为截止前成功是正常出口。

**`NotCheckpointing`**：在非 `Checkpointing` 相位调用 `checkpoint_*` 时返回；相位不变。
`phase` 字段让调用方知道当前相位，不需要再查一次。

## 4.10 `ResourceDecision`：对 P7D 的结论

```rust
/// 一次 `direct` 的结论。对 P7D：summary（AI/后台两项）＋ action（四档之一或 None）＋ phase。
/// **没有优先级**；`action == None` 表示无转移，不是第五动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDecision {
    summary: ResourceSummary,
    action: Option<ResourceAction>,
    phase: SuspensionPhase,
}

impl ResourceDecision {
    pub fn summary(&self) -> ResourceSummary;
    pub fn action(&self) -> Option<ResourceAction>;
    pub fn phase(&self) -> SuspensionPhase;
}
```

**为什么把 `summary` 嵌进来**：P7D 是后台，读 `resource_pressure`／`workload_class` 是允许的；
把两者放在同一个结论值里，避免 `direct` 之后再读一个可能已经变化的全局快照。**为什么没有
`priority`**：§300 的四档只对装配方（§6.3）；P7D 只看到动作与相位。

## 4.11 `ArbiterPolicy`、`PolicyError`、`ResourceArbiter`

```rust
/// 压力阈值、滞回与 checkpoint 策略。字段公开（配置面），`ResourceArbiter::new` 校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbiterPolicy {
    pub moderate_at: u8,              // 默认 50
    pub high_at: u8,                  // 默认 75
    pub critical_at: u8,              // 默认 90
    pub resume_at: ResourcePressure,  // 默认 Low；必须 < High
    pub cooldown_millis: u64,         // 默认 30_000
    pub checkpoint_timeout_millis: u64, // 默认 60_000；0 表示下一次调用即超时
    pub max_checkpoint_attempts: u32,   // 默认 3；0 表示从不 suspend（只 throttle）
}

impl Default for ArbiterPolicy { /* 上列默认值 */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    ThresholdsNotIncreasing,  // 需要 1 <= moderate_at < high_at < critical_at <= 100
    ResumeAtTooHigh,          // 需要 resume_at < ResourcePressure::High
}

/// 仲裁入口。字段私有；`new` 是唯一构造点。
#[derive(Debug)]
pub struct ResourceArbiter {
    policy: ArbiterPolicy,
    pressure: ResourcePressure,
    last_pressure_change_millis: u64,
    workload_class: Option<WorkloadClass>,
}

impl ResourceArbiter {
    pub fn new(policy: ArbiterPolicy) -> Result<Self, PolicyError>;

    /// 更新压力与类别，返回当前结论。`observation` 是六项聚合读数＋已分类的类别。
    /// 全 `None` 的读数不改变压力（§2.4 第 2 条）。
    pub fn observe(&mut self, observation: &ResourceObservation, now_millis: u64)
        -> ResourceSummary;

    /// 只读当前结论；供 P7B 端口实现/装配方取快照。
    pub fn summary(&self) -> ResourceSummary;

    /// 对一份工作计算动作并推进其状态机。
    /// - 相位 `Checkpointing` 且未到截止：Ok(action=None, phase=Checkpointing)；
    ///   到截止：相位回 `Throttled`，返回 Err(CheckpointTimedOut)；
    /// - 其余相位：按 §5.4／§5.5 选动作；`None` 表示无转移；
    /// - `Suspend` 只把相位推进到 `Checkpointing`，**不会**直接到 `Suspended`；
    ///   若 `attempts >= max_checkpoint_attempts`，`Running` 退化为 `Throttle`、
    ///   `Throttled` 退化为 `None`（§5.6）；
    /// - `Suspended` 的 `Resume` cooldown 只看 `state.last_transition_millis`（§5.5）。
    pub fn direct(
        &self,
        declaration: WorkloadDeclaration,
        state: &mut SuspensionState,
        now_millis: u64,
    ) -> Result<ResourceDecision, SuspensionError>;
}
```

**为什么 `direct` 是 `&self`**：压力/类别是 `ResourceArbiter` 的只读快照；每任务状态在
`SuspensionState` 里。这样同一个 arbiter 可以服务多个任务，不需要 `&mut` 争用。
**为什么 `direct` 返回 `Result`**：`Checkpointing` 到截止时的超时是错误（§5.6），不是普通决策；
其余情况不失败。**为什么 `observe` 不返回 `Result`**：读数缺失有确定语义（保持上一压力），不是错误。

## 4.12 `AssemblyView` 与 `assembly` 特性

```rust
#[cfg(feature = "assembly")]
pub mod assembly {
    pub use crate::priority::PriorityTier;

    /// 只对装配方（`ResourceDirectivePort` 的实现）可见的结论面：
    /// summary 两项＋§300 四档。**不进 AI／后台的 `ResourceSummary`／`ResourceDecision`。**
    /// 默认构建不可命名；装配构建下不承诺跨 crate 强制（§6.5、J7）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AssemblyView {
        summary: ResourceSummary,
        priority: PriorityTier,
    }

    impl AssemblyView {
        pub fn summary(&self) -> ResourceSummary;
        pub fn priority(&self) -> PriorityTier;
    }

    impl ResourceArbiter {
        pub fn assembly_view(&self, declaration: WorkloadDeclaration) -> AssemblyView;
    }
}
```

**默认构建没有 `PriorityTier`／`AssemblyView`**：`priority` 模块在 `lib.rs` 里是 `mod priority;`（不是
`pub mod`）；只有 `assembly` 模块在特性下 re-export。**装配方**在 `Cargo.toml` 里启用
`features = ["assembly"]`；P7D 不启用。**这不是安全边界**：同一构建图里特性可能被统一启用，
跨 crate 的可见性 Rust 无法按消费方区分；它只是**暴露面纪律**（§6.5、§11 第 10 条）。

## 4.13 可见性、构造点与不变量

- **私有字段**：`ResourceSummary`、`ResourceDecision`、`AssemblyView`、`SuspensionState`、
  `ResourceArbiter`。没有 `pub` 字段、没有 `Default`／`Deserialize`。**`Default` 的例外只有两个**：
  输入面 `ResourceObservation` 与配置面 `ArbiterPolicy`；其余类型（含 `SuspensionState`／
  `ResourceSummary`／`ResourceDecision`）没有 `Default`。
- **唯一构造点**：`ResourceSummary` 只由 `observe`／`summary` 产出；`ResourceDecision` 只由 `direct`
  产出；`AssemblyView` 只由 `assembly_view` 产出；`SuspensionState` 只由 `new` 产出；
  `ResourceArbiter` 只由 `new` 产出（并校验策略）。
- **输入面**：`ResourceObservation` 与 `WorkloadDeclaration` 是公开字段，装配方／P7D 可以构造。
  它们不承载结论，但 `ResourceObservation` **会改变共享压力**：任何持有 `&mut ResourceArbiter` 的
  调用方都能用伪造读数影响所有消费者看到的 `ResourceSummary` 与 `direct` 结果。P7C 不验证输入
  真实性；**输入面可信性由装配方负责**（§8.4 第 4 条、§11 第 9 条）。`WorkloadDeclaration` 的自报
  只影响该次 `direct` 的动作，但同样无法验证（§11 第 3 条）。
- **不实现**：`Serialize`／`Deserialize`／`Display`（`SuspensionError`／`PolicyError` 的 `Display`
  除外）；没有 serde 依赖。
- **`Debug`**：`ResourceSummary` 只打印两项；`ResourceDecision` 只打印三项；`SuspensionState` 打印
  相位／尝试数／最后转移时刻／截止；没有进程名／PID。

---

# 5. 判定规则

## 5.1 压力计算（`ResourceObservation` → `ResourcePressure`）

**步骤**（`ResourceArbiter::observe` 的实现约束）：

1. 取六项读数中**非 `None`** 的最大值；每项先钳到 `0..=100`（`>100` 当 100）。
2. 若六项全 `None`：**不改变当前压力**；只更新 `workload_class`。
3. 否则按阈值分档：
   - `max >= critical_at` → `Critical`
   - `max >= high_at` → `High`
   - `max >= moderate_at` → `Moderate`
   - 否则 → `Low`
4. 把原始档位交给 §5.2 的滞回；更新 `last_pressure_change_millis`（仅在实际变化时）。
5. `workload_class` 取本次 observation 的值（`None` 就清空，不保留旧类别）。
6. 返回 `summary()`。

**为什么取最大值**：§13 只给「资源明显竞争」这个方向；最大值是**最保守**的单一聚合，任何一项饱和
都算竞争。**代价**：Disk IO 尖峰与 CPU 饱和同权；没有按资源加权（§2.4 第 1 条）。**边界**：不覆盖
「多资源同时中等但合起来很重」；不覆盖每进程占用。

## 5.2 滞回（§14 `:746-754`）

**规则**（`ResourceArbiter` 内部）：

```
if raw > current:
    current = raw; last_change = now          // 升压立即
else if raw < current:
    if raw <= policy.resume_at
       && now.saturating_sub(last_change) >= policy.cooldown_millis:
        current = raw; last_change = now      // 降压受 resume threshold + cooldown 约束
    else:
        current 不变
else:
    current 不变
```

- **升压立即**：资源突然紧张时不等 cooldown。
- **降压保守**：只有原始压力降到 `resume_at` 或以下、且 cooldown 已过，才降；否则保持当前档位。
  这避免 49/50、74/75、89/90 附近反复升降（§14 `:754`）。
- **`resume_at` 默认 `Low`**：后台任务只在压力回到 Low 后才恢复；比「Moderate 就恢复」更保守，
  避免刚恢复又被 `Maintenance` 的 `Moderate→Throttle` 打回。
- **`now_millis` 由调用方给**：P7C 不取时钟（§5.7）；cooldown 的可信度由调用方的单调时钟保证。
  **代价**：调用方传入非单调时间会破坏滞回；P7C 不做时钟校验（§2.4 第 11 条）。

## 5.3 四档优先级分类（§300）

**唯一的分类函数**（`priority.rs`，`pub(crate)`）：

```rust
pub(crate) fn classify(declaration: WorkloadDeclaration) -> PriorityTier {
    if declaration.maintenance {
        PriorityTier::Maintenance
    } else if declaration.user_process {
        PriorityTier::UserWorkload
    } else if declaration.foreground {
        PriorityTier::ForegroundRuntime
    } else {
        PriorityTier::BackgroundRuntime
    }
}
```

- **`maintenance` 先判**：它是**最低**的资源优先级；先判是为了让维护任务先让路（对用户安全），
  不是把它抬高。
- **`user_process` 先于 `foreground`**：用户直接操作的外部工作负载不应被 runtime 任务压过。
- **全 false = `BackgroundRuntime`**：P7D 的普通后台任务默认落在这里。
- **`AssemblyView.priority` 与 `direct` 共用这一个函数**：装配方不自行分类（切分 §2.3.2 第 2 条
  `:157-158`）。`direct` 内部调用 `classify` 选动作；`assembly_view` 把同一结果给装配方。
- **`TaskSource` 不在本函数里**：P7D 先把 §299 的来源投影成三个布尔（§7.1）；本函数不解释授权。

## 5.4 压力×优先级的动作阶梯

**这张表是本设计自定（§2.4 第 7 条）**。`Resume` 不出现在表里：它是 `Suspended` 的恢复动作，由
§5.5 的相位规则决定。

| 优先级 \ 压力 | `Low` | `Moderate` | `High` | `Critical` |
|---|---|---|---|---|
| `UserWorkload` | `Run` | `Run` | `Run` | `Run` |
| `ForegroundRuntime` | `Run` | `Run` | `Run` | `Throttle` |
| `BackgroundRuntime` | `Run` | `Run` | `Throttle` | `Suspend` |
| `Maintenance` | `Run` | `Throttle` | `Suspend` | `Suspend` |

**读法**：

- 四档的顺序体现在「谁先让路」：`Maintenance` 在 `Moderate` 就让，`BackgroundRuntime` 到 `High` 才让，
  `ForegroundRuntime` 只在 `Critical` 降速，`UserWorkload` 从不让路。
- **`Suspend` 一律走两阶段**：`direct` 只把相位推进到 `Checkpointing`；P7D 执行 checkpoint 后再调
  `checkpoint_succeeded`／`checkpoint_failed`／`checkpoint_timed_out`（§5.5）。
- **`Throttle` 的量化不在本表**：P7C 只给动作名；降多少并发／速率是 P7D 的实现（§11 第 7 条）。
- **`Run` 不是「保证有资源」**：它只表示该档在当前压力下不需要 P7C 发出让路动作；实际准入仍由
  P7D／第 4 层决定。

## 5.5 相位转移

**转移表**（`SuspensionState` 的内部规则；`direct` 只调用合法动作）：

| 当前相位 | 事件 | 结果 | 说明 |
|---|---|---|---|
| `Running` | `Run` | `Running` | 无转移（`direct` 返回 `action=None`） |
| `Running` | `Throttle` | `Throttled` | 立即降速 |
| `Running` | `Suspend` | `Checkpointing` | 若 `attempts >= max_checkpoint_attempts`（含 `max=0`）：改为发 `Throttle`，相位到 `Throttled`，**不增加 `attempts`**（§5.6）；否则 `attempts += 1`、设置截止时刻、`last_transition = now` |
| `Running` | `Resume` | — | `direct` 不发出；不是合法动作 |
| `Throttled` | `Run` | `Running` | `attempts = 0` |
| `Throttled` | `Throttle` | `Throttled` | 无转移 |
| `Throttled` | `Suspend` | `Checkpointing` | 若 `attempts >= max_checkpoint_attempts`：`action=None`，相位保持 `Throttled`，**不增加 `attempts`**（§5.6）；否则**首次尝试**（`attempts == 0`）立即，**重试**（`attempts > 0`）需 `now - last_transition >= cooldown`，不满足时 `action=None`、相位保持 `Throttled`；通过后 `attempts += 1`、设置截止时刻、`last_transition = now` |
| `Throttled` | `Resume` | — | `direct` 不发出；用 `Run` |
| `Checkpointing` | 压力动作（`Run`／`Throttle`／`Suspend`／`Resume`） | `Checkpointing` | **不打断**；`direct` 返回 `action=None` |
| `Checkpointing` | `checkpoint_succeeded`（截止前） | `Suspended` | 唯一进入 `Suspended` 的路径；`last_transition = now` |
| `Checkpointing` | `checkpoint_succeeded`（截止后） | `Throttled` | 返回 `Err(CheckpointTimedOut)`；晚到成功拒绝；`last_transition = now` |
| `Checkpointing` | `checkpoint_failed` | `Throttled` | 返回 `CheckpointFailed`；`last_transition = now` |
| `Checkpointing` | `checkpoint_timed_out`／`direct` 到截止 | `Throttled` | 返回 `CheckpointTimedOut`；`last_transition = now` |
| `Suspended` | `Resume`（压力 ≤ `resume_at` 且 `now - state.last_transition_millis >= cooldown`） | `Running` | `attempts = 0`；清空截止；`last_transition = now` |
| `Suspended` | `Resume`（条件不满足） | `Suspended` | `direct` 返回 `action=None`；不更新 `last_transition` |
| `Suspended` | `Run`／`Throttle`／`Suspend` | `Suspended` | 无转移；已挂起不需要再降速 |
| 非 `Checkpointing` | `checkpoint_*` | 不变 | 返回 `NotCheckpointing { phase }`（`checkpoint_succeeded` 为 `Err`，另两个为直接返回值） |

**`last_transition_millis` 的更新**：表中每一条**实际转移**都把 `last_transition_millis` 设为
`now_millis`；`action=None` 的 no-op 不更新。`Resume` 的 cooldown 从进入 `Suspended` 的那一刻
起算，**不从压力变化的那一刻起算**；后者只用于 §5.2 的降压。

**`Throttled` 是否必经（当前读法）**：本设计读作「相位集合＋受控转移」，不把 §14 `:735-743` 的
箭头读成「每次挂起必须先经过 `Throttled`」。`Suspend` 从 `Running` 直接进 `Checkpointing`，因为
「已要求挂起但尚未完成」本身就是 `Checkpointing` 的定义（切分 §2.3.2 第 2 条 `:160-161`）。
**这是当前读法，不是已裁定结论**：若规范维护者读作必经，J8 与 §5.4 的动作阶梯必须重写（先
`Throttle` 再 `Suspend`）；具名前置见 §8.4 第 9 条、§11 第 6 条。

## 5.6 失败、超时、重试

**失败**：

- `checkpoint_failed(now)` 只允许从 `Checkpointing`；相位回 `Throttled`，返回
  `SuspensionError::CheckpointFailed`（**直接返回，不是 `Result`**）。
- **不得**报 `Suspended`；**不得**停在 `Checkpointing`（切分 §2.3.2 第 2 条 `:161-164`）。
- `checkpoint_succeeded` 在非 `Checkpointing` 相位返回 `Err(NotCheckpointing { phase })`，相位不变。

**超时**：

- 进入 `Checkpointing` 时设置 `checkpoint_deadline_millis = now + checkpoint_timeout_millis`
  （`saturating_add`）。
- `direct` 在开始时检查：若 `Checkpointing` 且 `now >= deadline`，相位回 `Throttled`，返回
  `Err(CheckpointTimedOut)`。
- `checkpoint_succeeded` 在截止后调用，同样回 `Throttled` 并返回 `Err(CheckpointTimedOut)`；
  **晚到的成功不报 `Suspended`**（fail-closed）。
- `checkpoint_timed_out(now)` 是定时器钩子；P7D 可以用 `checkpoint_deadline_millis()` 安排。
  它直接返回 `SuspensionError::CheckpointTimedOut`（非 `Result`）。
- **P7C 不建定时器线程**：若调用方从不调用 `direct`／`checkpoint_succeeded`／`checkpoint_timed_out`，
  相位不会被自动推进。这是库的边界；P7D 的轮询/定时器是其职责（§11 第 11 条）。**`Checkpointing` 不是
  可停留的稳态**：只要任一**相关状态入口**（`direct`／`checkpoint_succeeded`／`checkpoint_failed`／
  `checkpoint_timed_out`）被调用，超时就会把它推到 `Throttled` 并返回错误；`observe`／`summary`／
  `phase()` 不接收状态，不会推进超时。P7D 的 `checkpoint_deadline_millis()` 定时器是契约的一部分
  （§7.1 义务 3）。

**重试**：

- 每次**进入 `Checkpointing`** 才 `attempts += 1`；`attempts >= max_checkpoint_attempts` 时
  `direct` 不再发出 `Suspend`（也不增加 `attempts`），而是落到下面两条之一：
  - 当前 `Running`（只可能发生在 `max_checkpoint_attempts == 0`）：发出 `Throttle`，相位转
    `Throttled`（满足「0 表示从不 suspend（只 throttle）」，不是 `None`、也不是停在 `Running`）；
  - 当前 `Throttled`：返回 `action=None`，相位保持 `Throttled`（已在降速态，没有可再发的动作）。
- **首次从 `Throttled` 进 `Checkpointing` 不要求 cooldown**（`attempts == 0`）：压力从 `High` 升到
  `Critical` 时不应因为刚 throttle 过就延迟挂起。**失败后的重试**（`attempts > 0`）要求
  `now.saturating_sub(state.last_transition_millis) >= cooldown`，避免失败后立即重试；这里用的是
  **状态时钟**（`last_transition_millis`），不是压力时钟。cooldown 未过时 `direct` 返回
  `action=None`，相位保持 `Throttled`（不重试、也不改成 `Throttle`）。
- `Run`（从 `Throttled`）与 `Resume`（从 `Suspended`）把 `attempts` 重置为 0；一个新的压力周期有
  新的预算。`Resume` 的 cooldown 同样用 `state.last_transition_millis`（§5.5）；成功转 `Running`
  时把 `last_transition` 设为 `now`。
- **重试窗口内相位仍是 `Checkpointing`**：如果 P7D 选择在内部重试，它不应在每次失败时调
  `checkpoint_failed`，而应保持 `Checkpointing` 直到最后一次失败或超时；`checkpoint_timeout_millis`
  是外层上限。P7C 不观察内部重试次数；它只保证最终有 `CheckpointFailed`／`CheckpointTimedOut` 出口。
- **`max_checkpoint_attempts = 0`**：从不 `Suspend`，只可能 `Throttle`。`Running` 在 `Critical` 下
  第一次 `direct` 得到 `action=Throttle`、`phase=Throttled`；同压力下再次 `direct` 得到
  `action=None`、`phase=Throttled`；压力回 `Low` 后 `Run` 回 `Running`。这是策略选择，不是错误。

## 5.7 纯函数、确定性与无时钟

- `ResourceArbiter` 的输入只有 `ResourceObservation`、`WorkloadDeclaration`、`SuspensionState` 与
  调用方给的 `now_millis`；**没有全局状态、没有环境变量、没有 I/O、没有模型调用**。
- **不取时钟**：`src/` 里没有 `SystemTime::now`／`Instant::now`（J17）；cooldown 与超时都由
  `now_millis` 驱动。
- **确定性**：同一观察序列＋同一声明＋同一 `now` 序列，产出的 `ResourceSummary`／`ResourceDecision`／
  `SuspensionPhase` 逐字段相等（J17）。
- **无随机**：没有 `rand`／`thread_rng`；动作只由压力、优先级、相位、策略决定。

---

# 6. 暴露面与消费契约

## 6.1 AI／后台的结论面：`ResourceSummary`

| 项 | 内容 |
|---|---|
| 类型 | `ResourceSummary`（§4.4） |
| 字段/访问器 | `resource_pressure`、`workload_class`（恰两个） |
| 不含 | 四档优先级、进程名、PID、逐进程 CPU／GPU、命令行、工作目录、窗口标题、屏幕、打开文件、键盘 |
| 构造 | 只由 `ResourceArbiter::observe`／`summary` 产出；字段私有 |
| 序列化 | 无 `Serialize`／`Deserialize`／`Display`；装配方用两个访问器投影 |
| 规范 | §13 `:717-724`；§300 `:1921`；A14／OPEN-012 |

**`workload_class` 是 `Option`**：`None` 只表示「未分类」，不是第五档（§2.4 第 5 条）。AI 侧不得把
`None` 当成「没有用户行为」或「可以全速」。

**聚合 CPU／GPU 允许，逐进程 CPU／GPU 禁止**：`ResourceSummary` 本身不含任何读数；输入面
`ResourceObservation` 的六项是整机聚合（§4.2）。切分 §2.3.2 第 2 条 `:164` 的「CPU/GPU 占用」按
**聚合**读；任何带进程维度的 CPU／GPU 都不在公开类型里（J14）。

## 6.2 P7D 的控制面：`ResourceDecision`／`SuspensionState`

| 项 | 内容 |
|---|---|
| 类型 | `ResourceDecision`（summary＋action＋phase）、`ResourceAction`、`SuspensionPhase`、`SuspensionState`、`SuspensionError` |
| 读什么 | `summary`（两项）、`action`（四档之一或 `None`）、`phase`（四相位） |
| 不读什么 | `PriorityTier`、`AssemblyView`、任何原始进程信息 |
| 输入 | `WorkloadDeclaration`（三个布尔；P7D 按任务事实填）、`now_millis` |
| 失败 | `CheckpointFailed`／`CheckpointTimedOut`／`NotCheckpointing`；前两者返回时相位已是 `Throttled` |
| 规范 | §14 `:730-744`；切分 §2.3.2 第 2 条 `:158-165` |

**P7D 不读 `PriorityTier`**：`WorkloadDeclaration` 只带「这份工作是不是用户进程／前台等待／维护计划」
三条事实，不带 §300 的档名或排序；P7D **不能直接指定优先级**，只能声明工作性质。四档分类结果只在
P7C 内部与装配方的 `AssemblyView` 里出现。

**P7D 的义务**（不在 P7C 实现）：

1. 每个后台/维护任务持有自己的 `SuspensionState`；不共享、不复制。
2. 按压力变化/轮询先 `observe`（或由装配方周期 `observe`）再 `direct`；`action == None` 表示
   不动作，不能当 `Run`。
3. 收到 `Suspend` 时执行 checkpoint；用 `checkpoint_deadline_millis()` 安排超时；
   成功后 `checkpoint_succeeded`，失败后 `checkpoint_failed`，超时后 `checkpoint_timed_out`。
4. 收到 `Throttle` 时按自己的语义降速；P7C 不定义降多少（§11 第 7 条）。
5. 不把 `ResourceDecision`／`ResourceSummary` 原样转发给 AI；AI 只拿 `ResourceSummary` 两项
   （P7C 不能跨 crate 强制，A14／P9）。

## 6.3 装配方的面：`AssemblyView`（§300 四档）

| 项 | 内容 |
|---|---|
| 类型 | `AssemblyView`（summary＋priority）、`PriorityTier` |
| 何时可用 | 只在 `#[cfg(feature = "assembly")]`；默认构建不可命名 |
| 谁用 | `ResourceDirectivePort` 的实现（`continuum-runtime`），以及装配方的 `BackgroundSuitability` 投影 |
| 读什么 | `summary` 两项＋`priority` 四档 |
| 不含 | 进程名／PID／逐进程 CPU／GPU；checkpoint 内容 |
| 规范 | §300 `:1909-1921`；切分 §2.3.2 第 2 条 `:155-165` |

**装配方不自行分类**：`priority` 只能来自 `ResourceArbiter::assembly_view`；装配方不得用
`WorkloadDeclaration` 自己写一份 `match`（那会是第二份分类规则）。**装配方只消费**四档，不改变
压力/动作。

## 6.4 P7B 的 `ResourceDirectivePort`：不引入 P7C 类型

P7B 的端口（P7B 设计 `:832-936`）是：

```rust
pub trait ResourceDirectivePort {
    fn direct(&self, request: &ExecutionRequest<'_>)
        -> Result<ResourceDirective, ResourceDirectiveError>;
}
```

**P7C 不要求给这个 trait 加参数**：

- P7B 的实现方在**构造端口值时**持有 `ResourceSummary`（Copy）与 `AssemblyView`／`PriorityTier`
  的快照或句柄；`direct` 调用时读自己的状态（P7B 设计 `:913-916`）。
- **不引入 `ResourceContext` 参数**：P7C 不要求每次请求传入结论；P7B 的 trait 保持不变。
- **P7B 不依赖 P7C**；P7C 不依赖 P7B。装配方在两者之间接线（切分 §三第 5 条 `:205-212`）。
- **`BackgroundSuitability` 的投影**：装配方从 `summary.resource_pressure()`＋
  `summary.workload_class()` 做一次投影；P7C 不定义投影的取值域（§11 第 12 条）。P7B 只收不透明载体，
  不读 `workload_class`／`resource_pressure`（P7B 设计 J4）。

## 6.5 暴露面的强度与限制（据实）

- **默认构建**：`PriorityTier` 在私有模块；`AssemblyView` 不存在；`ResourceSummary`／
  `ResourceDecision` 无优先级字段。`cargo check -p continuum-arbiter` 的公开面就是 §6.1／§6.2。
- **装配构建**：`--features assembly` 才公开 `PriorityTier`／`AssemblyView`；装配方在
  `Cargo.toml` 里启用。
- **不是安全边界**：Cargo 特性在同一个构建图里可能被统一启用；Rust 的 `pub` 不能按消费方
  （装配方 vs AI/后台）区分。一个能依赖本 crate 的消费方，理论上可以启用特性并命名
  `PriorityTier`；P7D 是否遵守「不 import」只能靠源码纪律与它的 J 判据。**规范级强制归 P9**
  （§11 第 1、10 条）。
- **P7C 能拍到的**：默认构建的源码面/公开面（J6／J7）；`ResourceSummary` 恰两项；`ResourceDecision`
  无优先级；`Debug` 无原始信息。**拍不到的**：AI 实际收到了什么（装配方投影）、P7D 是否转发
  （§8.4）。

---

# 7. 跨块接口

## 7.1 P7D（Background Supervisor）

**P7C 给 P7D 的**：

1. `ResourceArbiter::observe`／`summary`：拿 `ResourceSummary`（压力＋类别）。
2. `ResourceArbiter::direct(declaration, &mut state, now)`：拿 `ResourceDecision`
   （summary＋action＋phase）。
3. `SuspensionState`：每任务状态机；`phase()`、`checkpoint_deadline_millis()`、
   `checkpoint_succeeded`／`checkpoint_failed`／`checkpoint_timed_out`。
4. `ResourceAction`：`Run`／`Throttle`／`Suspend`／`Resume`；`None` 是无转移。
5. `SuspensionError`：checkpoint 失败／超时／错相位。
6. `WorkloadDeclaration`：P7D 按任务事实填三个布尔。

**P7D 给 P7C 的**：

1. `now_millis`：调用方的单调毫秒；P7C 不取时钟。
2. `WorkloadDeclaration` 的三个布尔：任务是不是用户进程／前台等待／维护计划。
3. checkpoint 的执行结果：成功／失败／超时；P7C 只记录相位，不执行 checkpoint。

**P7D 不读**：`PriorityTier`、`AssemblyView`、`ResourceObservation` 的原始读数（若它只做控制）。
**P7C 不读**：P7D 的 `TaskSource`、`SessionPlan`、后台任务表、checkpoint 存储。

**`TaskSource` → `WorkloadDeclaration` 的映射**（P7D 设计定；本设计只给建议，不冻结）：

| §299 来源 | 建议的三个布尔 |
|---|---|
| `authorized Intent` | `user_process=false`；`foreground` 由 P7D 判断（用户是否在等）；`maintenance=false` |
| `policy` | 同上 |
| `maintenance schedule` | `maintenance=true`；其余 false |
| `product drive` | 同上（通常 `foreground=false`） |

**`TaskSource` 是四类，`WorkloadDeclaration` 是三个布尔**：两者不是同一个枚举；P7D 的映射是它的
设计决定（§11 第 3 条只要求分类规则由 P7C 给，不要求 P7D 的来源映射由 P7C 定）。

**P7D 的 checkpoint 语义**：P7C 不定义 checkpoint 的内容、存储、重试执行；P7D 在执行 `Suspend` 后
负责把任务恢复到可继续的状态。失败时 P7C 只保证相位回 `Throttled`，不保证任务数据完整
（§11 第 8 条）。

## 7.2 P7B／装配方（`ResourceDirectivePort`）

**P7C 给装配方的**：

1. `ResourceSummary`：两项结论；`Copy`，可被端口实现持有。
2. `AssemblyView`（`assembly` 特性）：summary＋`PriorityTier`；§300 四档只在这里。
3. `ResourceArbiter::observe`／`summary`／`assembly_view`。
4. 不提供 `ResourceContext`；不要求 P7B 的 trait 加参数。

**装配方给 P7C 的**：

1. `ResourceObservation`：六项聚合读数＋已分类的 `WorkloadClass`；由本机 Resource Arbiter 组件的
   本地探测器填（装配方落地，不在零依赖决策核内）。
2. 在构造端口值时持有 `ResourceSummary`／`AssemblyView`；调用时读自己的状态。
3. 启用 `assembly` 特性；在 `ALLOWED` 里把 `continuum-arbiter` 加进 runtime 行（装配方落地时）。

**P7B 不依赖 P7C**：P7B 只定义 `ResourceDirectivePort` 与四槽载体；P7C 只给结论。**P7B 的设计不需要
改**：本设计不要求 `ResourceContext` 参数，不要求 `ResourceDirective` 增加槽，不要求 P7B 读
`workload_class`／`resource_pressure`（P7B 设计 `:913-916`、`:1092-1100`）。

## 7.3 P7A／P7D 的 `product drive`（无 crate 边）

P7A 按 §152 的 lower 产出 Product Intent，P7D 的 `product drive` 按值消费（切分 §三第 6 条
`:213-243`）。**P7C 不参与这条交接**：它不读 Product、不读 Intent、不读 `contract_id`；P7D 把
`product drive` 任务映射成 `WorkloadDeclaration`（通常 `maintenance=false`、`foreground` 由 P7D 判断）
即可。**P7C 不登记 `continuum-product` 边**，也不登记 `continuum-semantics` 边（§2.1）。

## 7.4 `continuum-budget`／router／placement（无 crate 边）

- **budget**：P7C 不依赖、不读预算；预算读数是否应该影响压力是 §11 第 13 条，不是本设计的隐含边。
- **router／placement**：P7C 不依赖、不转调；`rank`／`place` 是 P7B 端口实现（装配方）的对象。
- **`continuum-provider`／`continuum-model-registry`／`continuum-node`**：P7C 零依赖；J1／J16。

## 7.5 P7C 不碰 P7A／P7B／P7D 的内部

| 块 | 内部 | P7C 的关系 |
|---|---|---|
| P7A | Product／ProductReadiness／Milestone／Progress Gate／Shippable | 不读、不写、不依赖；P7C 的 `ResourceDecision` 不是 §331 `Decision` |
| P7B | `IntentScheduler`／`IntentDecision`／`ResourceDirective`／端口 trait | 不依赖、不实现、不读；装配方接线 |
| P7D | `TaskSource`／`AuthorizedTask`／`SessionPlan`／Background Supervisor／checkpoint 执行 | 不读、不写；P7D 消费 P7C 的结论与状态机 |

---

# 8. 判据与照片

## 8.1 判据表（每条挂规范节）

| # | 判据 | 规范节 | 照片（可执行断言/命令） |
|---|---|---|---|
| **J1** | **零 workspace 依赖（硬断言，分两段）**：**J1(i) P7C 本地**——`ALLOWED` 行是 `("continuum-arbiter", &[])`；`cargo tree --edges all` 只有本 crate 一行；没有 dev 边；`every_crate_depends_only_on_its_allowed_set` 绿。**J1(ii) 最终树**——`cargo test -p continuum-runtime --test dependency_direction` 全绿，**前置：P7D 已完成 P6D 合并门 rebaseline／退役**（§11 第 14 条） | §7.3 `:452`、`:458`；§7.4 判据 3 `:465`；切分 §八第 1 条 `:472-473` | (a) `tests/zero_deps.rs` 跑 `cargo tree -p continuum-arbiter --depth 1 --edges all --prefix none`，断言输出恰一行 `continuum-arbiter v…`、无子节点；(b) 按 `ALLOWED` 的 **subject 行**精确匹配 `("continuum-arbiter", &[])` 并断言解析出的允许集合为空；**不**断言全文件只命中一行（最终树 runtime 行也会出现本 crate 名，J1(b) 的旧写法只在 P7C 自己的树上成立）；(c) `cargo test -p continuum-runtime --test dependency_direction every_crate_depends_only_on_its_allowed_set` 绿（P7C 本地可达）；(d) `Cargo.toml` 的 `[dependencies]`／`[dev-dependencies]` 为空；(e) J1(ii) 最终树全绿；在 P7D 完成前，`p6_union_adds_only_its_own_rows` 的断言 8（新增 subject 不在 `P6_NAMES`）会红，**不算 J1(ii) 绿灯** |
| **J2** | **模型层完全不可用仍能限制后台负载**：没有模型依赖/调用；高压力下 `BackgroundRuntime`／`Maintenance` 得到 `Throttle`／`Suspend`，`UserWorkload` **无让路动作**（`action=None`、`phase=Running`，不是“仍 Run”这个动作） | §13 `:673`；《工程》§7.2 `:441`；§7.4 判据 3 `:465` | `cargo test -p continuum-arbiter --test without_model`：构造 `observe(cpu=95,0)`；**每个声明用独立的 `SuspensionState::new()`**（`direct` 会推进状态，复用同一个状态会让第二个／第三个声明不从 `Running` 出发）：`direct(BackgroundRuntime, &mut s1, 0)` ⇒ `action=Suspend`、`phase=Checkpointing`；`direct(Maintenance, &mut s2, 0)` ⇒ `action=Suspend`、`phase=Checkpointing`；`direct(UserWorkload, &mut s3, 0)` ⇒ `action=None`、`phase=Running`；测试文件只 `use continuum_arbiter::…`，没有任何模型 crate；J1(i) 的 `cargo tree` 同时证明没有模型路径 |
| **J3** | **压力分档与聚合**：`Low/Moderate/High/Critical` 四档；六项取最大值；`None` 忽略；全 `None` 保持上一压力；`101` 钳到 100；非法策略被拒 | §13 `:675-684`、`:720`；§2.4 第 1、2 条 | `tests/pressure.rs` 表驱动（每档用新 arbiter）：`cpu=49→Low`、`50→Moderate`、`75→High`、`90→Critical`；`cpu=10,gpu=100→Critical`；`all None` 在 `Critical` 后仍 `Critical`；`cpu=101→Critical`；`cpu=0→Low`；`ResourcePressure::parse("HIGH")==Some(High)`、`parse("HIGHER")==None`；`ResourceArbiter::new` 对 `moderate_at=80,high_at=70` 返回 `Err(ThresholdsNotIncreasing)`、对 `resume_at=High` 返回 `Err(ResumeAtTooHigh)`；四档穷尽 `match` 编译通过 |
| **J4** | **滞回**：升压立即；降压只在 `raw <= resume_at` 且 cooldown 已过；冷却期内不反复 | §14 `:746-754`；§2.4 第 8 条 | `tests/hysteresis.rs`（默认 cooldown 30_000）：`observe(95,0)→Critical`；`observe(10,10_000)→Critical`；`observe(10,30_000)→Low`；`observe(80,30_001)→High`（升压不等 cooldown）；`observe(80,31_000)→High` |
| **J5** | **`WorkloadClass` 四档闭集**：`ALL.len()==4`；`as_str`／`parse` 往返；`parse("INTERACTIVE")==None`；没有 `Interactive` 变体 | §13 `:708-715`、`:721`；切分 §2.4 `:175`；§2.4 第 3 条 | `tests/workload.rs`：四档 `ALL`；`parse(as_str(c))==Some(c)`；`parse("INTERACTIVE")==None`；`parse("interactive_game")==None`（精确大写）；对 `WorkloadClass` 的穷尽 `match`（若加第五变体，测试编译失败） |
| **J6** | **AI／后台结论只有两项**：`ResourceSummary` 恰两个访问器；无优先级字段；`Debug` 不含优先级/原始信息；无 `Serialize`／`Deserialize`／`Display` | §300 `:1921`；§13 `:717-724`；A14；切分 §6.2 第 1 条 `:400` | `tests/source_surface.rs`：**全量 `src/` 扫描（不是只扫 `summary.rs`）**——`grep -rn "impl ResourceSummary" src/` 恰一处；用花括号配对取该 `impl` 块，块内 `pub fn` 恰 `resource_pressure`／`workload_class`；该块与 `ResourceSummary` 的字段声明中 `priority`／`PriorityTier`／`process_name`／`process_id`／`pid`／`cpu_usage`／`gpu_usage` 零命中；**去注释/字符串后** `src/` 所有文件都没有第二个 `impl ResourceSummary`（防在 `lib.rs`／`arbiter.rs` 里加 `pub fn priority`）；`format!("{:?}", summary)` 不含 `Priority`／`pid`／`process`；`Serialize`／`Deserialize`／`serde` 零命中；运行期 `summary.resource_pressure()`／`.workload_class()` 可读；编译失败夹具 `tests/fixtures/default_surface` 的 `summary_priority` bin（`summary.priority()`）与 `summary_literal` bin（结构体字面量）在默认特性下 `cargo check` 必红（E0599／E0451） |
| **J7** | **默认构建不可命名 `PriorityTier`／`AssemblyView`；装配构建下不承诺跨 crate 强制**：默认构建的公开面没有这两个名字；`PriorityTier` 的实现只允许在 `priority.rs`／`assembly.rs`／`arbiter.rs` 三个文件里 | §300 `:1909-1921`；切分 §2.3.2 第 2 条 `:155-165`；A14／OPEN-012 | (a) `src/lib.rs` 是 `mod priority;`（不是 `pub mod`）＋ `#[cfg(feature = "assembly")] pub mod assembly;`；(b) **全量** `grep -rn "PriorityTier" src/` 只允许 `priority.rs`／`assembly.rs`／`arbiter.rs` 命中（白名单之外零命中）；`pub use crate::priority::PriorityTier` 只允许出现在 `assembly.rs`；`impl ResourceSummary`／`impl ResourceDecision` 块内不出现 `PriorityTier`；(c) `cargo check -p continuum-arbiter` 绿；`cargo test -p continuum-arbiter --features assembly --test assembly` 绿：`maintenance→Maintenance`、`user_process→UserWorkload`、`foreground→ForegroundRuntime`、全 false→`BackgroundRuntime`、`maintenance+user_process→Maintenance`；`PriorityTier::as_str` 逐字四档、`rank` 3/2/1/0；(d) 编译失败夹具 `tests/fixtures/default_surface` 的 `priority` bin（`use continuum_arbiter::PriorityTier`）与 `assembly_view` bin（`use continuum_arbiter::AssemblyView`）在默认特性下 `cargo check` 必红（E0432）；(e) **强度边界**：装配构建下不承诺跨 crate 强制——同一构建图里启用 `assembly` 的消费方仍可命名；P7D 不启用与是否 import 由 P7D 的判据拍；规范级强制归 P9，P7C 只拍自己的默认公开面 |
| **J8** | **两阶段成功路径（当前读法）**：`Suspend` 只到 `Checkpointing`；`checkpoint_succeeded` 才到 `Suspended`；`Resume` 回 `Running`；**Resume cooldown 用状态时钟 `last_transition_millis`** | §14 `:735-743`；切分 §2.3.2 第 2 条 `:159-161`；§2.4 第 11 条 | `tests/suspension.rs`：`Critical` 下 `direct(BackgroundRuntime)` ⇒ `action=Suspend`、`phase=Checkpointing`；`checkpoint_succeeded(0)` ⇒ `Ok(Suspended)`；`observe(Low,30_000)` 后 `direct(30_000)` ⇒ `action=Resume`、`phase=Running`（进入 `Suspended` 的 `last_transition=0`，状态时钟差 30_000；**若误用压力时钟，`observe(Low,30_000)` 刚把压力时钟设为 30_000，差为 0，本断言会红**）；**反例**：`checkpoint_succeeded(20_000)` 进 `Suspended`、`observe(Low,30_000)`（压力时钟从 0 起已过 30_000，压力可降到 `Low`）、`direct(30_000)` ⇒ `action=None`／`Suspended`（压力已是 `Low`，但状态只挂起 10_000 < cooldown 30_000）；再 `direct(50_000)` ⇒ `action=Resume`／`Running`；`Suspended` 只在 `checkpoint_succeeded` 之后出现；本照片按“`Throttled` 不必经”的当前读法写，规范若读作必经则整条重写（§8.4 第 9 条） |
| **J9** | **失败回 `Throttled`**：`checkpoint_failed` 返回 `CheckpointFailed`（直接值，不是 `Err`）；相位 `Throttled`；不报 `Suspended`；错相位返回 `NotCheckpointing` | §14 `:730-744`；切分 §2.3.2 第 2 条 `:161-164` | `tests/suspension.rs`：`Suspend` ⇒ `Checkpointing`；`checkpoint_failed(1)` ⇒ `CheckpointFailed` 且 `phase()==Throttled`；`checkpoint_succeeded(2)` ⇒ `Err(NotCheckpointing { phase: Throttled })`；在 `Running` 调 `checkpoint_failed(3)` ⇒ `NotCheckpointing { phase: Running }` 且相位不变；断言没有 `Suspended` 逃出 |
| **J10** | **超时不得停在 `Checkpointing`**：截止后 `direct`／`checkpoint_succeeded` 返回 `Err(CheckpointTimedOut)` 并回 `Throttled`；`checkpoint_timed_out` 直接返回 `CheckpointTimedOut`；晚到成功拒绝；`checkpoint_deadline_millis` 在 `Checkpointing` 时 `Some` | 切分 §2.3.2 第 2 条 `:161-164`；§14 `:746-754` | `tests/timeout.rs`（`checkpoint_timeout_millis=100`）：`Suspend(0)` ⇒ `Checkpointing`、deadline `Some(100)`；`direct(99)` ⇒ `Ok(action=None, phase=Checkpointing)`；`direct(100)` ⇒ `Err(CheckpointTimedOut)` 且 `phase()==Throttled`、deadline `None`；另一例 `checkpoint_succeeded(100)` ⇒ `Err(CheckpointTimedOut)`、`phase()==Throttled`；`checkpoint_timed_out(50)` ⇒ `CheckpointTimedOut`、`Throttled`；在 `Running` 调 `checkpoint_timed_out(50)` ⇒ `NotCheckpointing { phase: Running }` |
| **J11** | **重试有上限**：连续失败到 `max_checkpoint_attempts` 后不再 `Suspend`；`Run`／`Resume` 都重置预算；重试受状态时钟 cooldown 约束；内部重试期间相位保持 `Checkpointing`；`max=0` 只 throttle | 切分 §2.3.2 第 2 条 `:162-164`；§2.4 第 11 条；§5.6 | `tests/retry.rs`（max=2，cooldown=0）：`Suspend→failed` ×2 后 `direct` 在 `Critical` 下 `action=None`、`phase=Throttled`；`observe(Low)` 后 `direct` ⇒ `Run`；再 `observe(Critical)` ⇒ `Suspend`（**`Run` 重置**）；**`Resume` 重置**（max=1，cooldown=0）：`Critical→Suspend`、`checkpoint_succeeded(0)` ⇒ `Suspended`、`observe(Low,30_000)`、`direct(30_000)` ⇒ `Resume`／`Running`；再 `observe(Critical,30_001)`、`direct(30_001)` ⇒ `Suspend`（若 `Resume` 不重置，`attempts=1` 会按上限抑制并退化为 `Throttle`）；另一例（cooldown=1_000）：`Suspend(0)`→`checkpoint_failed(1)` 后 `direct(500)` ⇒ `action=None`，`direct(1_001)` ⇒ `Suspend`（重试需状态时钟 cooldown）；另一例：`Suspend(0)` 后不调失败，`direct(49)` ⇒ `Ok(action=None, phase=Checkpointing)`，`checkpoint_succeeded(50)` ⇒ `Suspended`（重试/等待期间相位仍是 `Checkpointing`）；**`max=0`**：`Critical`、`BackgroundRuntime`、`Running`，`direct` ⇒ `action=Throttle`、`phase=Throttled`（不是 `None`、也不停在 `Running`）；同压力再次 `direct` ⇒ `action=None`、`phase=Throttled` |
| **J12** | **四档动作阶梯**：§5.4 的 4×4 表逐格可测；`UserWorkload` 恒 `Run`；`ForegroundRuntime` 仅 `Critical` `Throttle`；`BackgroundRuntime` `High` `Throttle`、`Critical` `Suspend`；`Maintenance` `Moderate` `Throttle`、`High`／`Critical` `Suspend` | §300 `:1909-1919`；§2.4 第 7 条 | `tests/ladder.rs` 表驱动 16 格：**每格新建 `ResourceArbiter`＋新建 `SuspensionState`**，先 `observe(10/60/80/95)` 到目标压力，断言 `arbiter.summary().resource_pressure()` **等于目标档**（防止 `resume_at=Low` 下从 `High` 只观察到 `Moderate` 时压力仍是 `High`，用 `High` 行冒充 `Moderate` 行），再从 `Running` 调 `direct`；断言 `action` 与 `phase`（`Run`→`action=None`／`phase=Running`；`Throttle`→`action=Throttle`／`phase=Throttled`；`Suspend`→`action=Suspend`／`phase=Checkpointing`）；**不采用**“先用 `BackgroundRuntime` 在 `High` 打到 `Throttled` 再 observe 目标压力”的旧写法（那会漏掉 4 个 `Moderate` 格）；`ResourceAction` 穷尽 `match` 四变体（无 `Kill`） |
| **J13** | **P7D 结论形状**：`ResourceDecision` 恰 `summary`／`action`／`phase` 三个访问器；无优先级；`action=None` 不是第五动作 | 切分 §2.3.2 第 2 条 `:158-159`；§2.4 第 15 条 | `tests/source_surface.rs`：**全量 `src/` 扫描（不是只扫 `arbiter.rs`）**——`grep -rn "impl ResourceDecision" src/` 恰一处；用花括号配对取该 `impl` 块，块内 `pub fn` 恰 `summary`／`action`／`phase`；该块与字段声明中 `priority`／`PriorityTier` 零命中（防在 `state.rs` 或别的文件里加 `pub fn priority`）；`format!("{:?}", decision)` 不含 `Priority`；运行期 `decision.summary()`／`.action()`／`.phase()` 可读；`action` 是 `Option<ResourceAction>`，`None` 时相位不变；编译失败夹具的 `decision_priority` bin（`decision.priority()`）在默认特性下 `cargo check` 必红（E0599） |
| **J14** | **原始进程信息不进公开类型**：没有 `process_name`／`process_id`／`pid`（词边界）／`cpu_usage`／`gpu_usage`／`command_line`／`working_directory`／`window_title` 字段；**聚合 CPU／GPU 允许，逐进程 CPU／GPU 禁止**；`ResourceSummary`／`ResourceDecision`／`SuspensionState` 的 `Debug` 无原始信息 | §13 `:686-706`；§300 `:1921`；A14；切分 §五 P7C `:334` | `tests/source_surface.rs`：`src/` 对上述标识符零命中（`pid` 用词边界，`process_id` 单独列出；`process_name` 不会覆盖它）；公开类型定义逐字段读出（`ResourceObservation` 六项聚合＋class；`ResourceSummary` 两项；`ResourceDecision` 三项；`SuspensionState` 相位/计数/时间）；`format!("{:?}", …)` 断言不含 `pid`／`process_id`／`process_name`／`command_line`；`grep -rn "std::process" src` 零命中；源码面允许聚合字段名 `cpu`／`gpu`／`vram`，不允许 `cpu_usage`／`gpu_usage` 这类逐进程命名 |
| **J15** | **零表、零迁移＋默认测试面干净**：不依赖 `continuum-persist`；`src/` 无 `Migration`／`Tx`；`runtime_migrations()`／`expected_migrations()`／`startup.rs` 无本 crate；默认 `cargo test -p continuum-arbiter` 不编译 `assembly` 测试目标 | 切分 §四第 2 条 `:265-276`；§10；§4.1 | `tests/source_surface.rs`：`Cargo.toml` 无 `continuum-persist`；有 `[[test]] name = "assembly"` 且 `required-features = ["assembly"]`；`src/` 无 `Migration`／`Tx`／`persist`；`crates/continuum-runtime/src/main.rs:114` 的 `runtime_migrations()` 无 `continuum_arbiter`；`tests/migrations.rs:37`／`tests/startup.rs:26,81,94` 无本 crate；`cargo test -p continuum-arbiter` 绿且不启数据库；`cargo test -p continuum-arbiter --no-run --message-format=json` 的 artifact 里**没有** `target.name == "assembly"`；`cargo test -p continuum-arbiter --features assembly --test assembly` 绿 |
| **J16** | **不建 Resource Scheduler／不碰 budget**：无 `continuum-provider`／`continuum-model-registry`／`continuum-node`／`continuum-graph`／`continuum-budget` 依赖或引用；不回答 §304 四问 | §301 `:1925-1935`；§304 `:1973-1982`；《工程》§7.3 `:452`；切分 §6.2 第 12 条 `:411` | `tests/source_surface.rs`：`src/` 对 `continuum_provider`／`continuum_model_registry`／`continuum_node`／`continuum_graph`／`continuum_budget`／`RoutingRequest`／`PlacementRequest`／`ModelProvider`／`select_runnable` 零命中；`Cargo.toml` 无这些依赖；`ResourceArbiter` 无 `place`／`rank`／`model`／`tool`／`parallelism` 的决策方法；J1(i) 的 `cargo tree` 一并证明 |
| **J17** | **确定性、无时钟、无 kill**：同一序列重复结果相等；`src/` 无 `SystemTime::now`／`Instant::now`／`rand`；`ResourceAction` 恰四变体、`SuspensionPhase` 恰四相位；无 `Kill` | §13 `:673`；§14 `:730`；§2.4 第 9、11 条 | `tests/determinism.rs`：两次相同 `observe`／`direct` 序列得到相等 `ResourceSummary`／`ResourceDecision`／`SuspensionPhase`；`src/` 对 `SystemTime::now`／`Instant::now`／`UNIX_EPOCH`／`thread_rng`／`rand::` 零命中；`src/` 对 `std::process`／`Command::new`／`kill(`／`std::fs`／`std::net`／`std::env`／`unsafe` 零命中；`ResourceAction`／`SuspensionPhase` 的 `as_str`／`parse` 往返；对 `ResourceAction`／`SuspensionPhase` 的穷尽 `match`（加第五变体即编译失败） |

## 8.2 照片的语料与读法

- **源码面**：`tests/source_surface.rs` 用 `std::fs` 读 `env!("CARGO_MANIFEST_DIR")/src/*.rs`，
  先去掉 `//`／`///` 注释与字符串字面量，再逐条断言；零命中类断言只限定 `src/`（非测试）语料，
  并**逐处读命中行**，避免「注释里出现一次」就误报（切分 §七第 11 条的教训）。测试放 `tests/`，
  故 `src/` 里没有测试构造。
- **运行期**：`tests/*.rs` 用公开 API；不读私有字段；断言 `ResourceSummary`／`ResourceDecision`
  的访问器与 `SuspensionState::phase()`。
- **依赖面**：J1(i) 的 `cargo tree`／`ALLOWED` 精确行／`every_crate_depends_only_on_its_allowed_set`
  在 P7C 自己的树上真跑；J1(ii) 的整树 `dependency_direction` 要等 P7D 完成 P6D 合并门
  rebaseline／退役后再真跑，不靠逐字推演（切分 §八第 1 条 `:472-473`）。J7 的 `--features assembly`
  测试由 `[[test]] required-features` 单独构建；J6 的默认构建测试不启用特性。源码面全量扫描
  `src/`，不按文件白名单偷懒；编译失败夹具用 `std::process::Command` 调 `cargo check`，target 目录
  隔离到 `target/ui/`。夹具 `tests/fixtures/default_surface/Cargo.toml` 带空 `[workspace]`（避免被
  父 workspace 吸纳），每个反例是一个独立 `--bin`（`priority`／`assembly_view`／`summary_priority`／
  `summary_literal`／`decision_priority`），单跑时断言非零退出与对应 rustc 错误码。
- **P7D／装配方的照片不在本 crate**：P7C 只能证明自己的公开面；AI 实际收到什么、P7D 是否转发
  优先级、装配方是否把 summary 投影正确，分别由 P9／P7D／装配方的判据证明（§8.4）。

## 8.3 变异预告（谁红）

| 变异 | 该红 | 不该红 |
|---|---|---|
| 给本 crate 加任一 workspace 依赖（如 `continuum-budget`） | J1(i)（`cargo tree` 出现子节点／`ALLOWED` 不符）、J2（若引入模型路径）、J16（若引入 router／placement） | J3–J12（单元行为仍可过） |
| 把 `INTERACTIVE` 加成第五变体，或把 `parse("INTERACTIVE")` 映射到 `InteractiveGame` | J5（闭集／笔误断言） | J3、J4、J8–J12（不读该变体） |
| 给 `ResourceSummary` 加 `priority` 字段或访问器 | J6（恰两个访问器／源码面）、J13（若 `ResourceDecision` 也带） | J1–J5、J8–J12（不读 summary 的优先级） |
| 默认构建公开 `PriorityTier`（把 `mod priority` 改成 `pub mod` 或去掉 `cfg`） | J7（默认面／源码面） | J3–J6、J8–J12（行为仍可过） |
| `checkpoint_failed` 后报 `Suspended`，或停在 `Checkpointing` | J9（失败回 `Throttled`）、J10（超时出口） | J8（成功路径不受影响，若失败路径不被触发） |
| 不做超时检查，`Checkpointing` 可以无限停 | J10（截止后仍 `Checkpointing`）、J11（重试上限的最终出口） | J8（成功路径） |
| 压力动作忽略优先级（四档同表） | J12（阶梯表）、J2（`UserWorkload` 无让路动作的断言） | J3、J4（压力本身仍对） |
| 加 `Kill` 动作或 `Suspended` 直接由 `Suspend` 到达 | J17（动作/相位穷尽）、J8（两阶段） | J12（若阶梯表没改，仍可能过；J8 抓两阶段） |
| 把压力/相位写进表或迁移 | J15（零表）、J1(i)（若加 `continuum-persist`） | J3–J12（内存行为仍可过） |
| 用 `SystemTime::now` 代替调用方时间 | J17（无时钟）、J4／J10（时间相关断言可能变得不可控） | J3、J5、J12（不读时间） |
| 在公开类型里加 `process_name`／`process_id`／`pid` | J14（全量源码面）、J6（若进 summary） | J1–J5、J8–J12（不读原始信息） |
| 在 `lib.rs`／`state.rs` 里给 summary／decision 加 `pub fn priority` | J6／J13（全量 `impl` 块扫描＋编译失败夹具） | J3–J5、J8–J12（行为仍可过） |

## 8.4 拍不出照片的地方（据实）

1. **AI 实际收到什么**：P7C 只能保证 `ResourceSummary` 两项；装配方把 summary 投影进上下文时是否
   夹带别的东西，不在本 crate。**收件人：装配方＋P9**（A14／OPEN-012）。
2. **P7D 是否把 `ResourceDecision`／`Summary` 转发给 AI**：P7C 不能跨 crate 禁止；P7D 的源码纪律
   与 J 判据负责。**收件人：P7D＋P9**。
3. **「模型层完全不可用」的真实停机**：J2 是结构证明（零依赖、无模型路径），不是把模型进程杀掉的
   端到端演练；P7C 不拥有模型进程。**收件人：装配方＋P9**。
4. **探测器读数是否真实/新鲜，以及输入面可伪造**：P7C 只消费 `ResourceObservation`；读数来源、
   频率、失败语义在装配方（§11 第 9 条）。`ResourceObservation` 是公开可构造的输入面：任何持有
   `&mut ResourceArbiter` 的调用方都能伪造读数，改变共享压力与所有消费者看到的 `ResourceSummary`；
   P7C 不验证真实性，**输入面可信性由装配方负责**。**收件人：装配方＋P7D**。
5. **`throttle` 是否真的降速**：P7C 只发动作；P7D 的执行语义不在本 crate（§11 第 7 条）。
6. **checkpoint 是否可恢复**：P7C 只保证相位；内容、存储、重启恢复在 P7D（§11 第 8 条）。
7. **四档优先级的跨 crate 强制**：`assembly` 特性不是安全边界（§6.5）；规范级强制在 P9
   （§11 第 1、10 条）。
8. **重启后的相位**：P7C 零持久化（§10）；恢复策略在 P7D（§11 第 8 条）。
9. **§14 箭头是否强制经过 `Throttled`**：J8／§5.4 按**当前读法**（`Suspend` 可从 `Running` 直达
   `Checkpointing`）写；若规范维护者读作必经，J8 与动作阶梯必须重写（先 `Throttle` 再 `Suspend`），
   不是实现期再补。**收件人：规范维护者＋协调者。这是 P7C 计划开工前的具名前置。**
10. **P6D 合并门尚未 rebaseline／退役**：P7C 的 `("continuum-arbiter", &[])` 会让
    `p6_union_adds_only_its_own_rows` 断言 8 红（新增 subject 不在 `P6_NAMES`）。J1(ii) 在 P7C 自己的
    交付点上不可达；**P7D 完成合并门前，J1(ii) 不是绿灯**。**收件人：P7D＋协调者**（§11 第 14 条）。

## 8.5 §7.4 判据 3 与本设计自报判据的关系

- **§7.4 判据 3**（`docs/02-工程.md:465`）只要求「模型层完全不可用仍能限制后台负载」。
  **J1(i)（零依赖结构）＋J2（无模型路径的高压力限流）**是这条判据的两半；J1(ii) 只是整树复核，
  不额外证明判据 3。J1(i) 证明结构上不可能依赖模型，J2 证明行为上仍能产出 `Throttle`／`Suspend`。
- **§7.4 不覆盖 P7C 的其余交付**：压力分档（J3）、滞回（J4）、四档进程类（J5）、最小暴露面
  （J6／J7／J14）、两阶段成功（J8）、失败（J9）、超时（J10）、重试上限（J11）、动作阶梯（J12）、
  P7D 结论形状（J13）、零表（J15）、不建 Resource Scheduler／不碰 budget（J16）、确定性/无 kill
  （J17）都没有 §7.4 判据。**P7C 不修改 §7.4**；这些是**本设计自报**的完成判据（§11 第 5 条）。
- **不宣称 §304 闭合**：J16 只证明 P7C 不建 Resource Scheduler；§304 的四问仍由 P7B 端口实现与
  第 4 层缺口负责；P7 结束时 §304 仍不完整（切分 §1.1 `:36`、§6.2 第 12 条 `:411`）。

---

# 9. 明确不做的

**这一节是 §1.2 的完整表**；每条给归属与理由。**不建、不留桩。**

| 不建的东西 | 归属 / 理由 |
|---|---|
| Resource Scheduler 本体（§304 四问） | 第 4 层；P7B 只冻结具名端口，装配方实现；P7C 不重造（切分 §1.1 `:29-36`、§6.2 第 12 条 `:411`） |
| `Node 在哪里执行`／`用哪个模型`／`哪个工具`／`多少并行` | §304 `:1973-1982`；P7C 无 `place`／`rank`／`ModelProvider`／工具路径／`max_parallel`（J16） |
| P7B 的 `ResourceDirectivePort` 实现 | 装配方 `continuum-runtime`；P7C 只给结论（§7.2） |
| `ResourceDirective`／四槽载体 | P7B（设计 `:832-936`）；P7C 不定义、不填槽 |
| 对 `continuum-provider`／`continuum-model-registry`／`continuum-node`／`continuum-graph`／`continuum-budget` 的依赖 | §7.3 入度为零（`:452`、`:458`）；J1／J16 |
| 模型调用、提示、`ContextPackage`、`Message` | P3／P6A；§13 `:673` |
| 进程探测／进程名分类器／PID 读取 | 本机 Resource Arbiter 组件的本地探测器（装配方落地，不在零依赖决策核内）；P7C 只收聚合读数与已分类的 `WorkloadClass`（§2.4 第 4 条） |
| `throttle` 的量化执行 | P7D；P7C 只给动作名（§11 第 7 条） |
| checkpoint 的内容、存储、重试执行、重启恢复 | P7D；P7C 只拥有相位与失败结果（§11 第 8 条） |
| 后台任务来源（§299 四类）／`TaskSource`／`AuthorizedTask` | P7D；P7C 的 `WorkloadDeclaration` 不是 `TaskSource`（§2.6） |
| 产品状态／Milestone／Progress Gate／Shippable | P7A；P7C 不读不写 |
| Intent 调度／`IntentDecision`／`ResourceDirectivePort` 的 trait | P7B；P7C 不依赖 |
| §331 的 `Decision`／`decision.required` | P7A；`ResourceDecision` 不是 §331（§2.6） |
| 表、迁移、事件、审计 | P7C 零表（§10）；不占 `200–209` 档 |
| `kill`／终止进程／`Kill` 动作 | §14 `:730`；P7C 只做协作式挂起（J17） |
| `serde`／`Deserialize`／跨进程序列化 | 零外部依赖；公开类型不提供序列化面（§4.13） |
| 四档优先级的 AI／后台暴露 | 只对装配方（§6.3、§6.5）；J6／J7 |

---

# 10. 持久化：**本块零迁移**

## 10.1 结论：不建表、不建迁移、不写事件

| 本块的产物 | 为什么不落库 |
|---|---|
| `ResourceSummary`／`ResourceDecision` | 运行期结论；规范没有要求保存；AI 投影由装配方做 |
| `ResourceObservation` | 探测器的一次读数；不是持久对象 |
| `WorkloadDeclaration` | 调用方的一次输入 |
| `SuspensionState` | 每任务的内存状态；checkpoint 的持久化是 P7D 的（§11 第 8 条） |
| `ArbiterPolicy` | 装配期配置；不是表 |

## 10.2 迁移号段：**本块不占档**

- 切分 §四第 2 条 `:265-276`：P7 取 `200–209`；P7A 确定占 `200`；P7B／P7C 无表则不占号。
- 本块**零表**，故**不声明号、不占档**；`runtime_migrations()`（`crates/continuum-runtime/src/main.rs:114`）
  不加本块的行；`expected_migrations()`（`tests/migrations.rs:37`）与 `tests/startup.rs` 的三处计数
  （`:26`、`:81`、`:94`）不同批改。
- **「我没占它」不等于「它没人占」**：本块只说「P7C 零表、不占档」，不替 P7D 声明。

## 10.3 事件

本块**不写任何 `continuum-events` 的事件**。§313 的必录清单里没有「资源仲裁」这一类；
`ResourceDecision`／`SuspensionPhase` 是运行期控制状态，不是审计事件。若将来要记录「为什么挂起」，
那是 P7D 的 checkpoint/审计面或 §313 的扩展，不是本块。

---

# 11. 设计问题（逐条具名）

**每条给「缺的是哪一步」与收件人；不用「后续」「长期阶段」兜。共 14 条。** §2.4 的 **15 条**是
本设计自定判定，本节 **14 条**是跨块设计问题；两处编号不是同一张表（评审按 13 条看会漏项）。

1. **A14／OPEN-012：Resource Arbiter 的暴露面强制**。§300 `:1921` 是 SHOULD；§13 的「只传两项」
   不是规范段；P7C 只能交可观察的公开面（`ResourceSummary` 两项、`ResourceDecision` 无优先级、
   `PriorityTier` 默认不导出、原始信息不入类型）。**缺的是「规范级强制点」这一步**；P7C 不声称
   SHOULD 已变成 MUST。**收件人：规范维护者＋P9（边界）**（主树 `docs/superpowers/p8-p10-scope.md:63`、
   `:68` 已列 A14／OPEN-012 与「A14 还需要 P7C 的 Arbiter 暴露面」）。代价：一个能依赖本 crate 的
   消费方仍可启用 `assembly` 特性或把 `ResourceDecision` 转发给 AI；P7C 的 J6／J7 只是**默认构建**的
   源码面／公开面纪律，装配构建下不承诺跨 crate 强制（J7(e)）。

2. **§13 的 `INTERACTIVE` 笔误**：§13 `:708-715` 的四档是 `INTERACTIVE_GAME`／`COMPUTE_WORKLOAD`／
   `COMPILER`／`VIDEO_RENDER`；`:721` 的输出示例写 `workload_class = INTERACTIVE`，**不在四档里**。
   P7C 按切分 §2.4 `:175` 与 §6.2 第 4 条 `:403` 只实现四档，**不设 `INTERACTIVE` 变体**，
   `parse("INTERACTIVE")==None`。**缺的是「`INTERACTIVE` 是笔误还是汇总后的暴露类」这一步**；
   **收件人：规范维护者（§13）＋P9**。代价：若原意是汇总类，当前 `ResourceSummary` 暴露的是具体类
   （如 `INTERACTIVE_GAME`），比示例更细；要改的是暴露投影，不是给 `WorkloadClass` 加第五档。

3. **§300 四档优先级的分类规则**：§300 `:1909-1919` 只给顺序，没给「谁算 User workload／Foreground
   Runtime／Background Runtime／Maintenance」。本设计用 `WorkloadDeclaration` 的三个布尔＋
   `maintenance > user_process > foreground > 默认背景` 的规则（§5.3）。**缺的是「谁声明、如何验证、
   三个布尔冲突时谁优先」这一步**；**收件人：规范维护者（§300）＋协调者＋P7D**。代价：调用方可以
   自报 `user_process=true` 逃避限流，P7C 没有独立证据；P7D 的 `TaskSource`→三个布尔的映射也由 P7D
   自定（§7.1），两边可能不一致。

4. **`workload_class` 的判定来源**：§13 `:686-706` 允许本机读进程名／PID，`:708` 说进程名在本机
   Arbiter 分类，但**没有给进程名→四档的映射表，也没说分类器在哪个 crate**。本设计把分类放在
   **本机 Resource Arbiter 组件的本地探测器**（部署整体的一部分，**不是零依赖决策核
   `continuum-arbiter`**），决策核只接收 `Option<WorkloadClass>`；公开类型不携带进程名／PID
   （§2.4 第 4 条）。**缺的是「映射表与分类器归属」这一步**；**收件人：规范维护者＋协调者＋装配方**。
   代价：P7C 无法验证分类；不同探测器可能给出不同结果；未知进程只能 `None`。**待协调者确认的读法**：
   切分 §五 P7C 行的「负载类别」指类型／闭集与判定规则的所有权，判定器属于本机组件但不属于零依赖
   决策核；若协调者读作「判定器也必须在 `continuum-arbiter` 内」，本块要么加依赖（破坏
   `ALLOWED == []`），要么把分类器移进 crate（`src/` 读 OS，违反 §5.7 的无 I/O 与 J17），两条都
   与本设计的纯决策核边界相抵，必须升级裁定。

5. **§7.4 只覆盖判据 3，不覆盖 P7C 的其余交付**：§7.4 `:460-467` 的四条里只有 `:465` 落 P7C，
   它只测「模型层完全不可用仍能限制后台负载」；**压力分档、滞回、四档进程类、四档优先级、两阶段
   挂起、失败回退、最小暴露面、不 kill 都没有 §7.4 判据**。P7C 自报 J3–J17（§8.5），**不修改 §7.4**。
   **缺的是「是否为 P7C 增补正式判据」这一步**；**收件人：协调者＋《工程》维护者**。代价：P7C 的
   验收依赖本设计的自报判据；跨文档不一致时没有规范锚。

6. **§14 的转移图与失败类型**：§14 `:735-743` 列出 `Running → Throttled → Checkpointing → Suspended →
   Resume`，但**没有说箭头是否强制、`Checkpointing` 能否被压力打断、失败结果类型是什么**。本设计
   读作「相位集合＋受控转移」：`Suspend` 可从 `Running`／`Throttled` 直接进 `Checkpointing`，
   `Throttled` 不是必经点；失败／超时回 `Throttled`（§5.5）。**缺的是「§14 的箭头是否强制经过
   `Throttled`」这一步**；**收件人：规范维护者（§14）＋协调者＋P7D**。代价：若读作必经，P7C 的动作
   序列要改成先 `Throttle` 再 `Suspend`；当前设计在 `Running` 收到 `Suspend` 会直接进
   `Checkpointing`。**这是 P7C 计划开工前的具名前置**（§8.4 第 9 条）；J8 已标「当前读法」。

7. **`throttle` 的量化语义**：§14 只给相位，§300 只给顺序；**没有说 `throttle` 降多少并发／速率、
   如何观测、是否可跳过**。P7C 只发 `ResourceAction::Throttle`；P7D 决定执行。**缺的是「throttle 的
   量化与可观测口径」这一步**；**收件人：P7D＋规范维护者**。代价：不同 P7D 实现的 `Throttle` 效果
   不可比；P7C 无法证明「已限流」。

8. **checkpoint 的执行、存储与重启恢复**：P7C 只拥有 `Checkpointing`／`Suspended` 相位；checkpoint
   的内容、落点、重试执行、失败原因、重启后的恢复都在 P7D。本设计零持久化（§10）。**缺的是
   「checkpoint 的表示、落点、重试与恢复」这一步**；**收件人：P7D＋协调者**。代价：P7C 不能保证
   挂起任务可恢复；重启后 `SuspensionState` 丢失，P7D 必须重建；若 P7D 落库相位，它要自己决定
   编码（本 crate 不提供 `Deserialize`）。

9. **OS 探测器的归属、频率与输入面可信性**：P7C 是纯决策核；六项聚合读数与进程名分类由本机
   Resource Arbiter 组件的本地探测器填 `ResourceObservation`。**缺的是「探测器在哪个 crate、读取
   频率、失败/过期语义」这一步**；**收件人：装配方（`continuum-runtime`）＋协调者**。代价：探测器
   失效／过期时 P7C 只能保持上一压力（全 `None` 不升压）；探测器若接了模型路径，会间接破坏
   「独立于 AI」的可观察性；装配方必须保证探测器不依赖模型。**输入面可信性**：`ResourceObservation`
   的字段公开、`observe(&mut self)` 公开，任何持有 arbiter 的调用方都能伪造读数并改变**共享**压力与
   `ResourceSummary`；P7C 不验证真实性，可信性由装配方负责（§4.13、§8.4 第 4 条）。

10. **四档优先级的暴露机制不是硬边界**：本设计用私有 `priority` 模块＋`assembly` 特性＋
    `AssemblyView`；默认构建不能命名 `PriorityTier`，但 Cargo 特性在同一构建图里可能被统一启用，
    Rust 的 `pub` 无法按消费方区分。**缺的是「跨 crate 可见性强制」这一步**；**收件人：P9＋协调者**。
    代价：A14 仍未关闭；P7C 只能拍源码面（J7）；P7D 是否 import 优先级要靠 P7D 的 J 判据。

11. **`ResourcePressure` 的量纲/阈值/聚合**：§13 只给 `HIGH` 示例；本设计用四档、六项最大值、
    默认 `50/75/90`、`resume_at=Low`、cooldown 30s（§2.4 第 1、8 条）。**缺的是「六项聚合的量纲、
    权重、阈值、每资源分别设阈」这一步**；**收件人：规范维护者（§13）＋协调者**。代价：Disk IO 尖峰
    与 CPU 饱和同权；默认值可能不适合所有机器；本设计不提供按部署调动作阶梯的策略面（只调阈值）。

12. **P7B 的 `BackgroundSuitability` 投影**：P7B 的载体是 `String`，装配方从 P7C 的
    `resource_pressure`＋`workload_class` 做一次投影；P7C 不定义投影的取值域／语义（P7B 设计
    `:651-659`、`:1041`）。**缺的是「投影取值域与语义」这一步**；**收件人：P7B＋装配方＋规范维护者**。
    代价：P7C 无法验证投影；P7B 的基线策略不能使用它；装配方与模型策略必须自己约定字符串词汇。

13. **预算与资源仲裁的关系**：`continuum-budget` 是记账（§333 五维、账本、`check`），不是资源
    仲裁；P7C 不依赖、不读预算。**缺的是「预算余额/消耗是否应影响压力或动作」这一步**；
    **收件人：P4／budget 持有方＋协调者**。代价：P7C 可能对「预算内但资源重」的任务照常让路，
    也可能对「预算耗尽但资源轻」的任务不感知；两者不是同一个问题，若将来要合并，必须另立一条边，
    不能把 `continuum-budget` 塞进 `ALLOWED == []`。

14. **P6D 合并门与 J1(ii) 的跨块前置**：P7C 加 `("continuum-arbiter", &[])` 后，
    `p6_union_adds_only_its_own_rows` 的断言 8（新增 subject 恰是已落地的 P6 crate）必红：
    `continuum-arbiter` 不在 `P6_NAMES`。P7C 本地只能交 J1(i)（`cargo tree`／`ALLOWED` 精确行／
    `every_crate_depends_only_on_its_allowed_set`）；J1(ii) 的整树 `dependency_direction` 全绿
    **前置是 P7D 完成 P6D 合并门的 rebaseline／退役**（切分 §四第 1 条 `:253-264`）。**缺的是
    「合并门重基线或退役」这一步**；**收件人：P7D＋协调者**。代价：P7C 先落地时 J1(ii) 不能当
    绿灯；若计划把 P7C 排在 P7D 前，验收必须拆成「J1(i) 绿＋J1(ii) 待前置」，不能把整树
    `dependency_direction` 写成 P7C 的完成条件。

---

# 12. 实测对账（只记，不改）

1. **`continuum-arbiter` 名字未占用**：`grep -rn "continuum-arbiter\|continuum_arbiter" crates/ Cargo.toml`
   零命中（2026-10-19，本工作树）。切分 §1.2 `:57-59` 的建议名可用。
2. **切分 §五 P7C 行与 §2.3.2 第 2 条的行号**：P7C 行在 `:334`；结论面在 `:155-165`；
   零依赖硬断言在 §五判据段 `:356-365` 与 §八第 1 条 `:472-473`。本设计按这些行号引。
3. **P7B 的 `ResourceDirectivePort` 不引入 P7C 类型**：P7B 设计 `:832-936` 的 trait 只收
   `ExecutionRequest`；`:913-916` 明说结论由实现方持有、不是 trait 参数。本设计**不要求**
   `ResourceContext` 参数，P7B 的接口不变。
4. **`continuum-budget` 不是资源仲裁**：`lib.rs:1-68` 是预算树/账本/validator；`dimensions.rs:14`
   是 §333 五维；`validator.rs:73` 是 `check`。P7C 不依赖它（§3.6、J16）。
5. **router／placement 是第 4 层碎片**：`RoutingRequest` `router.rs:199`、`RankedExecutionCandidates`
   `:405`、`rank` `:523`；`PlacementRequest` `placement.rs:202`、`place` `:362`、零生产调用点
   `:357-358`。P7C 不依赖、不转调（§3.6、J16）。
6. **§7.4 只覆盖判据 3**：四条判据在 `docs/02-工程.md:462-467`；`:465` 是 P7C；其余三条落 P7A
   （切分 §七第 12 条 `:462`）。本设计不把 §7.4 写成 P7C 的完整判据（§8.5、§11 第 5 条）。
7. **本设计未改任何代码、切分、规范**：新增文件只有本设计；`ALLOWED`／`Cargo.toml`／迁移表
   的改动留给实现轮（本设计只写形状与判据）。
