# P7C（Resource Arbiter）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**任务单：** 切分 §五 P7C 行（`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md:336`，逐字）：

> 建独立于模型的资源仲裁：压力／负载类别、四档优先级、协作式挂起（含 `Checkpointing` 两阶段与失败回退）与最小暴露面

**本计划是该行的实现分解。设计与规范的唯一事实来源是**
`docs/superpowers/specs/2026-10-19-p7c-resource-arbiter-design.md`（1535 行，下称「设计」）。
**本计划不改设计、不改规范、不改切分文档、不改代码以外的任何计划。** 引用一律用**设计节号**；
规范依据由设计 §3 逐条给出，本计划不复制规范原文。

**Goal:** 新建零 workspace 依赖的 crate `continuum-arbiter`，交付：①`ResourcePressure`／`WorkloadClass`
与六项聚合读数输入面；②`ResourceArbiter::observe` 的压力计算与滞回；③§300 四档优先级
（仅 `assembly` 特性面）与压力×优先级的 4×4 动作阶梯；④四档动作 `run`／`throttle`／`suspend`／`resume`
与 `SuspensionPhase` 四相位、`Checkpointing` 两阶段、失败／超时回 `Throttled`、重试上限；
⑤对 AI／后台只暴露 `ResourceSummary` 两项。**零表、零迁移、零事件、零外部依赖**
（设计 §1.1、§2.1、§4、§5、§9、§10）。

**Architecture:** 本块是第 7 层的**纯决策核**，在 workspace 内**入度为零**：
`ALLOWED` 行必须是 `("continuum-arbiter", &[])`（设计 §2.1、§2.3；切分 §五判据段）。
输入只有 `ResourceObservation`、`WorkloadDeclaration`、`SuspensionState` 与调用方给的 `now_millis`；
不读 OS、不取时钟、不调模型、不依赖 router／placement／budget（设计 §5.7、§7.4）。
`ResourceSummary` 恰 `resource_pressure`＋`workload_class` 两项；`ResourceDecision` 恰
`summary`＋`action`＋`phase` 三项；§300 的四档 `PriorityTier` 定义在私有模块，只有
`#[cfg(feature = "assembly")]` 的 `assembly` 模块 re-export，`AssemblyView` 只在装配构建可见
（设计 §4.4、§4.6、§4.10、§4.12、§6.5）。`Suspend` 只推进到 `Checkpointing`；
`Suspended` 只在 `checkpoint_succeeded` 截止前成功时可达；失败／超时**返回错误并回 `Throttled`**，
没有任何 `Kill` 动作（设计 §4.7–§4.9、§5.5、§5.6）。

**Tech Stack:** Rust 1.95.0 / edition 2024（workspace 的 `rust-version = "1.95"`）。
**普通依赖：零。外部依赖：零。dev 依赖：零。** 只用 `std`；错误类型手写 `Display`／`Error`；
不引入 `serde`／`thiserror`／`trybuild`／`sysinfo`／`rand`／`tokio`（设计 §2.1）。
测试也只用 `std`：编译失败夹具由 `std::process::Command` 调 `cargo check`，不用 `trybuild`
（设计 §8.2）。**任务书里的「dev 只有 trybuild」与设计 §2.1 的「没有 dev-dependencies」相抵；
按设计执行，见「遗留/待协调」第 5 条。**

**Spec:** 设计全文（**唯一事实来源**）；切分
`docs/superpowers/specs/2026-10-18-p7-scope-and-split.md` §五 P7C 行（`:336`）、§2.2、§2.3.2 第 2 条
（`:155-167`）、§2.4、§三、§四第 1／2／3／5／6 条、§五判据段（`:358-367`）、§六.2 第 1／4／12 条；
规范 §300（`docs/spec/05-normative.md:1907-1921`）、§13（`docs/spec/01-concepts.md:669-724`）、
§14（`:728-754`）；《工程》§7.1（`docs/02-工程.md:429`）、§7.2（`:441`）、§7.3（`:452`、`:458`）、
§7.4 判据 3（`:465`）；A14／OPEN-012（`docs/02-工程.md:345`；
`docs/01-总纲.md:1698-1702`、`:1881-1884`）。**§7.4 只覆盖判据 3，不覆盖 P7C 的其余交付**
（设计 §3.4、§8.5）。

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。`cargo test -p continuum-arbiter`（默认与
  `--features assembly` 两套）全绿、**0 warning**；`cargo build --workspace --all-targets` 0 warning。
  测试命令统一带 `TMPDIR="$PWD/.tmp"`。
- **`ALLOWED == []` 是硬断言**：`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`
  只加 `("continuum-arbiter", &[])` 一行；指向任何 workspace 内 crate 的直接边都会让逐对断言变红
  （设计 §2.1；“同 `continuum-protected` 先例”，`:322`）。不碰 `continuum-runtime` 的 `ALLOWED` 行。
- **零表、零迁移、零事件**：不改 `runtime_migrations()`（`crates/continuum-runtime/src/main.rs:114`）、
  `expected_migrations()`（`crates/continuum-runtime/tests/migrations.rs:37`）、`tests/startup.rs` 的三处计数
  （`:26`／`:81`／`:94`）；不登记 `continuum-persist`／`continuum-events`；不占 `200–209` 档
  （设计 §10；切分 §四第 2 条）。
- **四档动作恰四变体，没有 `Kill`**：`ResourceAction` 只有 `Run`／`Throttle`／`Suspend`／`Resume`；
  `action: Option<ResourceAction>` 的 `None` 表示“无转移”，不是第五动作（设计 §4.7、§2.4 第 15 条）。
- **`Checkpointing` 失败不得报 `Suspended`、不得停在 `Checkpointing`**：`checkpoint_failed` 与
  `checkpoint_timed_out` 直接返回 `SuspensionError`（不是 `Result`），相位回 `Throttled`；
  `checkpoint_succeeded` 在截止后调用返回 `Err(CheckpointTimedOut)` 并回 `Throttled`；
  `Suspended` 只在截止前成功时可达（设计 §4.9、§5.5、§5.6）。
- **四档优先级只对装配方可见**：`PriorityTier` 在私有 `priority` 模块；默认构建的公开面没有
  `PriorityTier`／`AssemblyView`；`ResourceSummary`／`ResourceDecision` 不含优先级；只有
  `#[cfg(feature = "assembly")]` 的 `assembly` 模块 re-export（设计 §4.6、§4.12、§6.5）。
- **`max_checkpoint_attempts = 0` 只 throttle**：`Running` 下收到 `Suspend` 退化为 `Throttle` 并进
  `Throttled`（不是 `None`、也不停在 `Running`）；`Throttled` 下收到 `Suspend` 返回 `None` 并保持
  `Throttled`（设计 §5.6）。
- **`Resume` 的 cooldown 只用 `SuspensionState.last_transition_millis`**（状态时钟），不用
  `ResourceArbiter.last_pressure_change_millis`（压力时钟）；`Run`（从 `Throttled`）与 `Resume`
  （从 `Suspended`）都把 `attempts` 重置为 0；`checkpoint_succeeded` **不**重置（设计 §4.8、§5.5、§5.6）。
- **`ResourceSummary` 恰两项、字段私有**：只有 `resource_pressure`／`workload_class` 两个公开访问器；
  唯一公开构造点是 `ResourceArbiter::observe`／`summary`；无 `Default`／`Deserialize`／`Display`／
  `Serialize`（设计 §4.4、§4.13）。
- **`ResourceDecision` 恰三项、无优先级**：`summary`／`action`／`phase` 三个访问器；唯一公开构造点是
  `direct`（设计 §4.10）。
- **原始进程信息不进公开类型**：没有 `process_name`／`process_id`／`pid`／`cpu_usage`／`gpu_usage`／
  `command_line`／`working_directory`／`window_title`；聚合 CPU／GPU 允许，逐进程 CPU／GPU 禁止
  （设计 §6.1、§4.13、J14）。
- **不建 Resource Scheduler／不碰 budget**：`src/` 不出现 `continuum_provider`／`continuum_model_registry`／
  `continuum_node`／`continuum_graph`／`continuum_budget`／`RoutingRequest`／`PlacementRequest`／
  `ModelProvider`／`select_runnable`；`ResourceArbiter` 没有 `place`／`rank`／`model`／`tool`／`parallelism`
  的决策方法（设计 §3.7、J16）。**不宣称 §304 已闭合**。
- **不取时钟、不随机、无 I/O**：`src/` 不出现 `SystemTime::now`／`Instant::now`／`UNIX_EPOCH`／
  `thread_rng`／`rand::`／`std::process`／`Command::new`／`std::fs`／`std::net`／`std::env`／`unsafe`；
  所有时间由调用方的 `now_millis` 驱动（设计 §5.7、J17）。
- **测试一律放 `crates/continuum-arbiter/tests/`**，不放 `src/` 的 `#[cfg(test)]`——J6／J7／J13／J14／
  J15／J16／J17 的源码面语料限定 `src/`，测试构造不得混入（设计 §4.1、§8.2）。
- **`tests/assembly.rs` 必须由 `[[test]] name = "assembly" required-features = ["assembly"]` 门控**；
  默认 `cargo test -p continuum-arbiter` 不得编译它（设计 §2.1、§4.1、J15）。
- 代码注释、错误信息、测试断言信息用中文；标识符用英文。
- **不要用 `git add -A`，不要 `git commit --amend`**；只 `git add <显式路径>`。新 crate 会改
  `Cargo.lock`，Task 1 按显式路径一并提交。
- 本块**不碰** `crates/continuum-runtime/src/**`、`crates/continuum-runtime/tests/migrations.rs`、
  `crates/continuum-runtime/tests/startup.rs`、`crates/continuum-runtime/tests/fixtures/**`、
  `docs/02-工程.md`、`docs/spec/**`、`docs/superpowers/specs/**`。

---

## Review Focus

复审时**逐条按下面的用例名核**；每条都归到拥有该代码的 task 的测试，不留在“设计说了”层面。

| # | 要试着推翻的断言 | 归属 task 与用例 |
|---|---|---|
| 1 | **模型层不可用时无法限流**：把 `continuum-arbiter` 接上任何模型／router／placement crate，或让高压力下的 `BackgroundRuntime`／`Maintenance` 不再得到 `Throttle`／`Suspend` | **Task 1** `the_crate_has_no_dependencies`＋`every_crate_depends_only_on_its_allowed_set`；**Task 4** `high_pressure_limits_background_and_maintenance_without_any_model_path`（每个声明用独立 `SuspensionState`） |
| 2 | **`Checkpointing` 永久停留**：`direct`／`checkpoint_succeeded` 不检查截止，或晚到成功仍报 `Suspended` | **Task 5** `direct_at_the_deadline_returns_to_throttled`／`late_success_is_rejected_after_the_deadline`／`the_timer_hook_returns_to_throttled` |
| 3 | **四档优先级泄漏到默认公开面**：默认构建能命名 `PriorityTier`／`AssemblyView`，或 `ResourceSummary`／`ResourceDecision` 加了 `priority` 字段／访问器 | **Task 3** 的 `classification_follows_maintenance_user_foreground_default`（装配构建）；**Task 6** `priority_tier_is_private_by_default_and_only_assembly_re_exports_it`／`summary_has_exactly_two_public_accessors`／`decision_has_exactly_three_public_accessors_and_no_priority`／`default_surface_rejects_hidden_priority_and_private_literals` |
| 4 | **`SuspensionState` 复用导致第二次 `direct` 错位**：P7D 把同一个 `SuspensionState` 给两个任务用，第二个声明从上一任务的 `Checkpointing`／`Suspended` 出发，而不是从 `Running` | **Task 4** `per_task_state_is_required_for_independent_decisions`（同一状态第二次 `direct` 返回 `None`／`Checkpointing`；新状态才得到 `Suspend`） |
| 5 | **`Resume` 后尝试计数未重置**：`Suspended → Running` 没有把 `attempts` 清 0，下一轮 `Critical` 被 `max_checkpoint_attempts` 永久抑制 | **Task 5** `resume_resets_the_checkpoint_attempt_budget`（`max=1`、`cooldown=0`；`Resume` 后再 `Critical` 必须仍得到 `Suspend`） |

---

## 跨计划前置（依赖的实际状态、J1 两段与共写文件）

### 一、依赖的交付物与它们的**实际状态**（2026-10-19 实读本工作树）

| 依赖 | 实际状态 | 本块用在哪 |
|---|---|---|
| **workspace 内 crate** | **零条**（设计 §2.1 的依赖边表只有“无”一行） | 本块全部类型都在 `continuum-arbiter` 内定义，不引用任何 workspace 类型 |
| **外部 crate** | **零条**：只用 `std`；不引入 `serde`／`thiserror`／`sysinfo`／`rand`／`tokio` | 错误类型手写 `Display`／`Error`；测试只用 `std` |
| **dev-dependencies** | **零条**（设计 §2.1） | 编译失败夹具用 `std::process::Command` 调 `cargo check`（设计 §8.2）；**不引入 `trybuild`** |
| **`assembly` 特性** | 本块新建：`[features] default = []`／`assembly = []` | `PriorityTier`／`AssemblyView`／`ResourceArbiter::assembly_view` 只在 `--features assembly` 下存在 |
| **`continuum-arbiter`** | **不存在**：`grep -rn "continuum-arbiter\|continuum_arbiter" crates/ Cargo.toml` 零命中 | 本计划的全部交付物 |
| **`continuum-protected` 先例** | **已交付**：`ALLOWED` 的 `("continuum-protected", &[])` 在 `dependency_direction.rs:322`，空数组是硬断言 | `continuum-arbiter` 照同一形状 |
| **依赖门** | **已交付**：`ALLOWED` 在 `crates/continuum-runtime/tests/dependency_direction.rs:26`；逐对断言 `every_crate_depends_only_on_its_allowed_set` 在 `:494`；`cargo_tree_direct` 用 `--edges all`（`:476-491`） | J1(i)；新行必须精确为空 |
| **P6D 合并门** | **未 rebaseline／退役**：`p6_union_adds_only_its_own_rows` 在 `:784`；断言 1 的 `baseline.len() == 27` 在 `:790`；解析器的 `rows.len() == 27` 在 `:615` | J1(ii) 的最终树前置；本块**不碰** `p6_pre_allowed.txt`／`P6_EXPECTED_ROWS`／27 字面量 |

**结论：本块没有硬前置。** 零条内部边、零条外部边、零 dev 边；Task 1–8 只依赖本计划先前 task 的接口。
唯一不能在本块交付点变绿的是 **J1(ii) 整树 `dependency_direction`**，前置是 P7D 完成 P6D 合并门的
rebaseline／退役（设计 §2.1、§11 第 14 条；切分 §四第 1 条）。

### 二、J1 拆两段（**不要混成一条**）

| 段 | 内容 | 本计划落点 | 何时可绿 |
|---|---|---|---|
| **J1(i) P7C 本地** | `cargo tree -p continuum-arbiter --depth 1 --edges all --prefix none` 只有本 crate 一行；`Cargo.toml` 的 `[dependencies]`／`[dev-dependencies]` 为空；`ALLOWED` 的 subject 行精确是 `("continuum-arbiter", &[])`；`every_crate_depends_only_on_its_allowed_set` 绿 | Task 1 的 `tests/zero_deps.rs`＋依赖门命令；Task 8 复核 | **P7C 本地可达** |
| **J1(ii) 最终树** | `cargo test -p continuum-runtime --test dependency_direction` 全绿（含 `p6_union_adds_only_its_own_rows`） | Task 8 只记录**预期红**；不把整树绿写成 P7C 完成条件 | **P7D 完成 P6D 合并门 rebaseline／退役之后**；在 P7D 完成前，断言 8（新增 subject 不在 `P6_NAMES`）会红，不算 J1(ii) 绿灯 |

**J1(ii) 的红不是回归**：`continuum-arbiter` 是新增 workspace 成员、新增 `ALLOWED` subject，P6 的
`P6_NAMES` 不认识它；本块**不得**在 `p6_pre_allowed.txt` 上追加行，也**不得**改
`P6_EXPECTED_ROWS`／`rows.len() == 27`／`baseline.len() == 27`。这些是 P7D 的合并门工作。

### 三、共写文件（**只有两行**）

| 文件 | 本块的改动 | 落在哪个 task |
|---|---|---|
| 根 `Cargo.toml` 的 `[workspace] members`（`:3`） | 加一行 `"crates/continuum-arbiter"` | Task 1 Step 4 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:26`） | 加 `("continuum-arbiter", &[]),`；**空数组是硬断言** | Task 1 Step 4 |

**不碰**：`crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`、`tests/migrations.rs` 的
`expected_migrations()`、`tests/startup.rs` 的三处计数、`crates/continuum-runtime` 的 `ALLOWED` 行
（P7C 不被 runtime 依赖；装配方落地时才加它的行）、`p6_pre_allowed.txt`、`P6_EXPECTED_ROWS`
（设计 §2.1、§10）。`Cargo.lock` 随新 crate 变化，Task 1 一并提交。

### 四、各 task 的“运行，确认失败”一步量到的是哪一种失败

| task | Step 2 量到的是 | 为什么 |
|---|---|---|
| Task 1 | **package 未找到**（不是编译失败） | Step 2 时 `Cargo.toml` 尚未创建；cargo 报 “package ID specification did not match any packages” |
| Task 2 | **编译失败 `E0432`** | `ArbiterPolicy`／`ResourceArbiter`／`ResourceSummary` 由 Step 3 才建出 |
| Task 3 | **编译失败 `E0432`**（Step 2）；Step 3 后 `--test assembly` 无特性时是 **cargo 目标错误** | `continuum_arbiter::assembly` 由 Step 4 才建出；Step 3 专门验证 `required-features` 门控 |
| Task 4 | **编译失败 `E0432`／`E0599`** | `ResourceAction`／`SuspensionState`／`ResourceDecision`／`direct` 由 Step 3 才建出 |
| Task 5 | **运行期红** | `checkpoint_timed_out` 与状态机在 Task 4 已存在；超时检查、重试上限、`max=0` 分支尚未实现 |
| Task 6 | **首次即绿**（源码面，Step 3）；编译失败夹具可能首轮报路径／错误码不符 | 本 task 只新建断言与夹具；实现已在 Task 1–5 交付。若源码面红，说明实现漂了，回查拥有该代码的 task |
| Task 7 | **首次即绿**（determinism）；manifest 若有门控缺口会红 | 本 task 只新建断言；manifest 缺口回 Task 3 修门控 |
| Task 8 | 无“确认失败”步 | 它是全量门、J1(ii) 记录与变异复核 |

**“首次即绿”不是缺陷**：Task 6／7 的交付物就是断言本身，它们的红由 Task 8 的变异复核取。

---

## File Structure

设计 §4.1 的模块表**是唯一依据**。任务书列举的 `suspension.rs`／`error.rs` 不在设计表里：
`suspension` 的内容按设计落在 `state.rs`（`SuspensionPhase`／`SuspensionState`／`SuspensionError`），
`error` 没有独立文件（`PolicyError` 在 `policy.rs`，`SuspensionError` 在 `state.rs`）。**不建空文件充数。**

```
crates/continuum-arbiter/
  Cargo.toml                零普通依赖、零 dev 依赖；features: default=[] / assembly=[]
                            Task 3 追加 [[test]] name="assembly" required-features=["assembly"]
  src/lib.rs                只做 mod／pub use／#[cfg(feature = "assembly")] pub mod assembly；
                            不定义类型（设计 §4.1）
  src/pressure.rs           ResourcePressure / ResourceObservation / pub(crate) raw_pressure（设计 §4.1、§4.2、§5.1）
  src/workload.rs           WorkloadClass（四档闭集、ALL、as_str／parse）（设计 §4.3）
  src/summary.rs            ResourceSummary（两项；字段私有；pub(crate) from_parts）（设计 §4.4）
  src/declaration.rs        WorkloadDeclaration（三个布尔）（设计 §4.5）
  src/priority.rs           PriorityTier（私有模块）＋ pub(crate) classify（设计 §4.6、§5.3）
  src/assembly.rs           #[cfg(feature="assembly")]：re-export PriorityTier、AssemblyView、
                            ResourceArbiter::assembly_view（设计 §4.12）
  src/action.rs             ResourceAction（设计 §4.7）
  src/state.rs              SuspensionPhase / SuspensionState / SuspensionError（设计 §4.8、§4.9）
  src/policy.rs             ArbiterPolicy / PolicyError（设计 §4.11）
  src/arbiter.rs            ResourceArbiter / ResourceDecision / 动作阶梯 / direct（设计 §4.10、§4.11）
  tests/support/mod.rs      源码语料 helper：src_files（原文）／code_files（去注释与字符串）／
                            code_only／struct_block／impl_block／field_names／public_fn_names／
                            contains_token（Task 6 起）
  tests/zero_deps.rs        J1(i)（Task 1）
  tests/workload.rs         J5（Task 1）
  tests/pressure.rs         J3 的压力分档／summary 运行期（Task 1 的 parse＋Task 2 的分档）
  tests/policy.rs           默认值、非法策略（Task 2）
  tests/hysteresis.rs       J4（Task 2）
  tests/assembly.rs         J7 的分类／as_str／rank／AssemblyView（Task 3；仅 assembly 特性）
  tests/action.rs           ResourceAction／SuspensionPhase 的文本往返与穷尽 match（Task 4）
  tests/ladder.rs           J12 的 16 格（Task 4）
  tests/without_model.rs    J2＋Review Focus 4 的状态复用对照（Task 4）
  tests/suspension.rs       J8／J9（Task 4）
  tests/timeout.rs          J10（Task 5）
  tests/retry.rs            J11（Task 5）
  tests/source_surface.rs   J6／J7／J13／J14／J16／J17 的源码面（Task 6）
  tests/compile_fail.rs     默认公开面的五份编译失败夹具驱动（Task 6）
  tests/fixtures/default_surface/
    Cargo.toml              空 [workspace]；path 依赖 continuum-arbiter；五个独立 bin
    .gitignore              忽略 target/ 与 Cargo.lock
    src/bin/priority.rs            use continuum_arbiter::PriorityTier;
    src/bin/assembly_view.rs       use continuum_arbiter::AssemblyView;
    src/bin/summary_priority.rs    summary.priority()
    src/bin/summary_literal.rs     ResourceSummary { .. }
    src/bin/decision_priority.rs   decision.priority()
  tests/determinism.rs      J17 的确定性、文本往返、穷尽 match（Task 7）
  tests/manifest.rs         J15 的清单／零表／默认测试面（Task 7）
```

**`ResourceSummary` 的构造点（实现决定，设计 §4.4 的“唯一构造点是 observe／summary”按公开面读）**：
`summary.rs` 定义字段私有的 `ResourceSummary`，并提供一个 `pub(crate) fn from_parts(...)` 供
`arbiter.rs` 的 `observe`／`summary` 使用。`pub(crate)` 不是公开构造点；J6 的
`pub fn` 计数只数 `pub fn`，不会把它当成第三个公开访问器。**不提供** `pub fn new`／`Default`／
`Deserialize`。

**本计划要改的既有文件**只有两处手写文件（都在共写表里），外加生成的 `Cargo.lock`：

```
Cargo.toml                                             members 加 crates/continuum-arbiter（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs  ALLOWED 加 continuum-arbiter 行（Task 1）
Cargo.lock                                             随新 crate 生成（Task 1 一并提交）
```

**不建** `src/error.rs`／`src/suspension.rs`／`src/builder.rs`／`src/clock.rs`：设计 §4.1 没有这些文件；
本块没有第二个错误面、没有构造器、没有时钟。

---

### Task 1: crate 骨架、零依赖门与两张基础枚举

**Files:**
- Create: `crates/continuum-arbiter/Cargo.toml`
- Create: `crates/continuum-arbiter/src/lib.rs`
- Create: `crates/continuum-arbiter/src/pressure.rs`
- Create: `crates/continuum-arbiter/src/workload.rs`
- Create: `crates/continuum-arbiter/tests/zero_deps.rs`
- Create: `crates/continuum-arbiter/tests/workload.rs`
- Create: `crates/continuum-arbiter/tests/pressure.rs`（本 task 只放 `ResourcePressure` 的 `parse`／`as_str`；
  Task 2 追加分档用例）
- Modify: `Cargo.toml`（`[workspace] members`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED`）

**Interfaces:**
- Consumes: 无
- Produces:
  - `continuum_arbiter::ResourcePressure`：`Low`／`Moderate`／`High`／`Critical`；
    `as_str(&self) -> &'static str`、`parse(&str) -> Option<Self>`
  - `continuum_arbiter::ResourceObservation`：六个 `Option<u8>`（`cpu`／`ram`／`gpu`／`vram`／
    `disk_io`／`network`）＋`workload_class: Option<WorkloadClass>`；派生
    `Debug, Clone, Copy, PartialEq, Eq, Default`
  - `continuum_arbiter::WorkloadClass`：`InteractiveGame`／`ComputeWorkload`／`Compiler`／`VideoRender`；
    `ALL: [WorkloadClass; 4]`、`as_str`、`parse`

- [ ] **Step 1: 写三份失败用例**

`tests/zero_deps.rs` 两条：

```rust
use std::process::Command;

#[test]
fn the_crate_has_no_dependencies() {
    let output = Command::new(env!("CARGO"))
        .args([
            "tree", "-p", "continuum-arbiter", "--depth", "1",
            "--edges", "all", "--prefix", "none",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree 运行失败");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let tree = String::from_utf8(output.stdout).expect("cargo tree 输出不是 UTF-8");
    let lines: Vec<&str> = tree.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "依赖树必须只有本 crate 一行：\n{tree}");
    assert!(lines[0].starts_with("continuum-arbiter v"), "第一行不是本 crate：{tree}");
}

#[test]
fn the_manifest_has_empty_dependency_sections_and_the_allowed_row_is_empty() {
    let manifest = include_str!("../Cargo.toml");
    for section in ["dependencies", "dev-dependencies"] {
        let body = manifest
            .split(&format!("[{section}]"))
            .nth(1)
            .unwrap_or_else(|| panic!("Cargo.toml 缺少 [{section}]"))
            .split("\n[")
            .next()
            .expect("section 体为空");
        let entries: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        assert!(entries.is_empty(), "[{section}] 必须为空：{entries:?}");
    }
    let allowed = include_str!("../../continuum-runtime/tests/dependency_direction.rs");
    let row = allowed
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("(\"continuum-arbiter\","))
        .expect("ALLOWED 缺少 continuum-arbiter 行");
    assert_eq!(row, "(\"continuum-arbiter\", &[]),");
}
```

`tests/workload.rs` 三条：

```rust
use continuum_arbiter::WorkloadClass;

#[test]
fn the_class_set_is_exactly_four_and_round_trips() {
    assert_eq!(WorkloadClass::ALL.len(), 4);
    for class in WorkloadClass::ALL {
        assert_eq!(WorkloadClass::parse(class.as_str()), Some(class));
    }
}

#[test]
fn interactive_is_not_a_fifth_class() {
    assert_eq!(WorkloadClass::parse("INTERACTIVE"), None);
    assert_eq!(WorkloadClass::parse("interactive_game"), None);
}

#[test]
fn the_exhaustive_match_has_no_fifth_arm() {
    fn name(class: WorkloadClass) -> &'static str {
        match class {
            WorkloadClass::InteractiveGame => "INTERACTIVE_GAME",
            WorkloadClass::ComputeWorkload => "COMPUTE_WORKLOAD",
            WorkloadClass::Compiler => "COMPILER",
            WorkloadClass::VideoRender => "VIDEO_RENDER",
        }
    }
    assert_eq!(name(WorkloadClass::Compiler), "COMPILER");
}
```

`tests/pressure.rs` 本 task 只写两条：

```rust
use continuum_arbiter::ResourcePressure;

#[test]
fn pressure_text_is_exact_uppercase() {
    assert_eq!(ResourcePressure::parse("HIGH"), Some(ResourcePressure::High));
    assert_eq!(ResourcePressure::parse("HIGHER"), None);
    assert_eq!(ResourcePressure::parse("high"), None);
    for pressure in [
        ResourcePressure::Low, ResourcePressure::Moderate,
        ResourcePressure::High, ResourcePressure::Critical,
    ] {
        assert_eq!(ResourcePressure::parse(pressure.as_str()), Some(pressure));
    }
}

#[test]
fn pressure_has_four_ordered_bands() {
    assert!(ResourcePressure::Low < ResourcePressure::Moderate);
    assert!(ResourcePressure::Moderate < ResourcePressure::High);
    assert!(ResourcePressure::High < ResourcePressure::Critical);
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test zero_deps --test workload --test pressure
```

**结果：** cargo 报 “package ID specification did not match any packages”（**package 未找到**，不是
`E0432`／`E0433`）。按 P5c 纪律 1(c) 这**不算红**；Step 3 才建 crate。

- [ ] **Step 3: 建 crate 骨架与两张枚举**

`Cargo.toml` 照设计 §2.1 的形状，但 `[[test]]` 留到 Task 3：

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
```

`src/lib.rs` 只做 `mod`／`pub use`，不定义类型：

```rust
//! P7C Resource Arbiter：独立于模型的资源仲裁决策核。
//! workspace 内零依赖；不读 OS、不取时钟、不调模型。

mod pressure;
mod workload;

pub use pressure::{ResourceObservation, ResourcePressure};
pub use workload::WorkloadClass;
```

`src/pressure.rs` 定义 `ResourcePressure` 与 `ResourceObservation`；`src/workload.rs` 定义
`WorkloadClass`、`ALL`、`as_str`／`parse`。**本 task 不实现 `ResourceArbiter`**——`observe`／`summary`
在 Task 2 才出现。

- [ ] **Step 4: 加 workspace member 与 `ALLOWED` 行**

在根 `Cargo.toml` 的 `members` 末尾加 `"crates/continuum-arbiter",`；在
`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 里加：

```rust
    // P7C Resource Arbiter（设计 §2.1）。**空数组是硬断言**：
    // 任何指向 workspace 内别的 crate 的直接边都会让逐对断言变红。
    ("continuum-arbiter", &[]),
```

- [ ] **Step 5: 跑本 task 的测试与依赖门**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test zero_deps --test workload --test pressure
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction every_crate_depends_only_on_its_allowed_set
```

**结果：** 前一条全绿（7 条用例）；后一条只跑 `every_crate_depends_only_on_its_allowed_set`，绿。
**不要跑整份 `dependency_direction`**：`p6_union_adds_only_its_own_rows` 此刻会红，那是 J1(ii) 的预期前置，
不是本 task 的回归。

- [ ] **Step 6: 提交**

```bash
git add Cargo.toml Cargo.lock \
  crates/continuum-arbiter/Cargo.toml \
  crates/continuum-arbiter/src/lib.rs \
  crates/continuum-arbiter/src/pressure.rs \
  crates/continuum-arbiter/src/workload.rs \
  crates/continuum-arbiter/tests/zero_deps.rs \
  crates/continuum-arbiter/tests/workload.rs \
  crates/continuum-arbiter/tests/pressure.rs \
  crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(p7c): 建立 continuum-arbiter 与零依赖门"
```

---

### Task 2: 策略、压力聚合、滞回与 `ResourceSummary`

**Files:**
- Create: `crates/continuum-arbiter/src/policy.rs`
- Create: `crates/continuum-arbiter/src/summary.rs`
- Create: `crates/continuum-arbiter/src/arbiter.rs`
- Modify: `crates/continuum-arbiter/src/pressure.rs`（追加 `pub(crate) raw_pressure`）
- Modify: `crates/continuum-arbiter/src/lib.rs`（导出三个新模块的类型）
- Modify: `crates/continuum-arbiter/tests/pressure.rs`（追加 J3 分档／钳位／全 `None`）
- Create: `crates/continuum-arbiter/tests/policy.rs`
- Create: `crates/continuum-arbiter/tests/hysteresis.rs`

**Interfaces:**
- Consumes: Task 1 的 `ResourcePressure`、`ResourceObservation`、`WorkloadClass`
- Produces:
  - `continuum_arbiter::ArbiterPolicy`：`moderate_at: u8`（默认 `50`）、`high_at: u8`（`75`）、
    `critical_at: u8`（`90`）、`resume_at: ResourcePressure`（`Low`）、`cooldown_millis: u64`（`30_000`）、
    `checkpoint_timeout_millis: u64`（`60_000`）、`max_checkpoint_attempts: u32`（`3`）；
    `impl Default`
  - `continuum_arbiter::PolicyError`：`ThresholdsNotIncreasing`／`ResumeAtTooHigh`；
    手写 `Display`／`Error`
  - `continuum_arbiter::ResourceSummary`：字段私有；`resource_pressure()`／`workload_class()`；
    `pub(crate) from_parts(pressure, workload_class)`
  - `continuum_arbiter::{ResourceArbiter}`：`new(policy) -> Result<Self, PolicyError>`；
    `observe(&mut self, observation: &ResourceObservation, now_millis: u64) -> ResourceSummary`；
    `summary(&self) -> ResourceSummary`
  - `crate::pressure::raw_pressure(&ResourceObservation, &ArbiterPolicy) -> Option<ResourcePressure>`
    （`pub(crate)`；六项最大值、钳位、全 `None` 返回 `None`，§5.1 第 1–3 步）

- [ ] **Step 1: 写分档、策略与滞回用例**

`tests/pressure.rs` 追加（每档用**新 arbiter**，避免上一档的滞回状态污染）：

```rust
use continuum_arbiter::{ArbiterPolicy, ResourceArbiter, ResourceObservation, ResourcePressure,
    ResourceSummary, WorkloadClass};

fn arbiter() -> ResourceArbiter {
    ResourceArbiter::new(ArbiterPolicy::default()).expect("默认策略必须合法")
}

fn pressure_of(cpu: u8) -> ResourcePressure {
    arbiter()
        .observe(&ResourceObservation { cpu: Some(cpu), ..Default::default() }, 0)
        .resource_pressure()
}

#[test]
fn pressure_bands_use_the_max_of_non_none_readings() {
    assert_eq!(pressure_of(0), ResourcePressure::Low);
    assert_eq!(pressure_of(49), ResourcePressure::Low);
    assert_eq!(pressure_of(50), ResourcePressure::Moderate);
    assert_eq!(pressure_of(75), ResourcePressure::High);
    assert_eq!(pressure_of(90), ResourcePressure::Critical);
    assert_eq!(pressure_of(101), ResourcePressure::Critical);
    let observation = ResourceObservation { cpu: Some(10), gpu: Some(100), ..Default::default() };
    let summary = arbiter().observe(&observation, 0);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical);
}

#[test]
fn all_none_keeps_pressure_and_clears_the_class() {
    let mut arbiter = arbiter();
    let first = ResourceObservation {
        cpu: Some(95), workload_class: Some(WorkloadClass::Compiler), ..Default::default()
    };
    let summary = arbiter.observe(&first, 0);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical);
    assert_eq!(summary.workload_class(), Some(WorkloadClass::Compiler));
    let summary = arbiter.observe(&ResourceObservation::default(), 1);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical);
    assert_eq!(summary.workload_class(), None);
}

#[test]
fn summary_has_exactly_two_read_only_accessors() {
    fn reads(summary: &ResourceSummary)
        -> (ResourcePressure, Option<WorkloadClass>) {
        (summary.resource_pressure(), summary.workload_class())
    }
    let _: fn(&ResourceSummary) -> (ResourcePressure, Option<WorkloadClass>) = reads;
}
```

`tests/policy.rs`：

```rust
use continuum_arbiter::{ArbiterPolicy, PolicyError, ResourceArbiter, ResourcePressure};

#[test]
fn defaults_are_the_documented_values() {
    let policy = ArbiterPolicy::default();
    assert_eq!(policy.moderate_at, 50);
    assert_eq!(policy.high_at, 75);
    assert_eq!(policy.critical_at, 90);
    assert_eq!(policy.resume_at, ResourcePressure::Low);
    assert_eq!(policy.cooldown_millis, 30_000);
    assert_eq!(policy.checkpoint_timeout_millis, 60_000);
    assert_eq!(policy.max_checkpoint_attempts, 3);
}

#[test]
fn non_increasing_thresholds_are_rejected() {
    for (moderate_at, high_at, critical_at) in
        [(0, 75, 90), (80, 70, 90), (80, 90, 90), (80, 90, 101)]
    {
        let policy = ArbiterPolicy {
            moderate_at, high_at, critical_at, ..ArbiterPolicy::default()
        };
        assert_eq!(
            ResourceArbiter::new(policy).unwrap_err(),
            PolicyError::ThresholdsNotIncreasing
        );
    }
}

#[test]
fn resume_at_must_be_below_high() {
    let high = ArbiterPolicy { resume_at: ResourcePressure::High, ..ArbiterPolicy::default() };
    assert_eq!(ResourceArbiter::new(high).unwrap_err(), PolicyError::ResumeAtTooHigh);
    let moderate = ArbiterPolicy { resume_at: ResourcePressure::Moderate, ..ArbiterPolicy::default() };
    assert!(ResourceArbiter::new(moderate).is_ok());
}
```

`tests/hysteresis.rs` 照设计 J4 的序列：

```rust
use continuum_arbiter::{ArbiterPolicy, ResourceArbiter, ResourceObservation, ResourcePressure};

fn observe(arbiter: &mut ResourceArbiter, cpu: u8, now: u64) -> ResourcePressure {
    arbiter
        .observe(&ResourceObservation { cpu: Some(cpu), ..Default::default() }, now)
        .resource_pressure()
}

#[test]
fn pressure_rises_immediately_and_falls_only_below_resume_at_after_cooldown() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    assert_eq!(observe(&mut arbiter, 95, 0), ResourcePressure::Critical);
    assert_eq!(observe(&mut arbiter, 10, 10_000), ResourcePressure::Critical);
    assert_eq!(observe(&mut arbiter, 10, 30_000), ResourcePressure::Low);
    assert_eq!(observe(&mut arbiter, 80, 30_001), ResourcePressure::High);
    assert_eq!(observe(&mut arbiter, 80, 31_000), ResourcePressure::High);
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test pressure --test policy --test hysteresis
```

**结果：** `E0432`（`ArbiterPolicy`／`ResourceArbiter`／`ResourceSummary` 尚不存在）。这是编译失败，
按 P5c 纪律 1(c) **不算红**；Step 3 才建生产代码。

- [ ] **Step 3: 实现策略、summary 与 `observe`／`summary`**

`src/policy.rs`：按 §4.11 定义 `ArbiterPolicy`（字段公开）与 `PolicyError`；`new` 的校验是
`1 <= moderate_at < high_at < critical_at <= 100` 与 `resume_at < ResourcePressure::High`。
`src/pressure.rs`：追加 `pub(crate) fn raw_pressure(&ResourceObservation, &ArbiterPolicy) ->
Option<ResourcePressure>`——六项非 `None` 读数逐项钳到 `0..=100` 取最大值，按
`critical_at`／`high_at`／`moderate_at` 分档；全 `None` 返回 `None`（§5.1 第 1–3 步）。
`src/summary.rs`：字段私有，两个公开访问器＋`pub(crate) fn from_parts`。
`src/arbiter.rs`：`ResourceArbiter` 字段私有（`policy`／`pressure`／`last_pressure_change_millis`／
`workload_class`）；`new` 令 `pressure = Low`、`last_pressure_change_millis = 0`、`workload_class = None`。
`observe` 按 §5.1：

1. 调 `pressure::raw_pressure(observation, &self.policy)` 取原始档位；
2. 若为 `None`：不改变 `pressure`，但仍用本次 observation 的 `workload_class` 覆盖（`None` 就清空）；
3. 否则把原始档位交给 §5.2 的滞回：升压立即；降压只在 `raw <= resume_at` 且
   `now.saturating_sub(last_pressure_change_millis) >= cooldown_millis` 时生效；
   **仅在实际变化时**更新 `last_pressure_change_millis`；
4. 返回 `summary()`。

`src/lib.rs` 追加三个模块与导出。**本 task 不实现 `direct`／`ResourceAction`／`SuspensionState`**。

- [ ] **Step 4: 跑本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test pressure --test policy --test hysteresis
```

**结果：** 全绿（`pressure` 5 条、`policy` 3 条、`hysteresis` 1 条），0 warning。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-arbiter/Cargo.toml \
  crates/continuum-arbiter/src/lib.rs \
  crates/continuum-arbiter/src/pressure.rs \
  crates/continuum-arbiter/src/policy.rs \
  crates/continuum-arbiter/src/summary.rs \
  crates/continuum-arbiter/src/arbiter.rs \
  crates/continuum-arbiter/tests/pressure.rs \
  crates/continuum-arbiter/tests/policy.rs \
  crates/continuum-arbiter/tests/hysteresis.rs
git commit -m "feat(p7c): 压力分档、滞回与最小结论面"
```

---

### Task 3: §300 四档优先级、`assembly` 特性与装配面

**Files:**
- Create: `crates/continuum-arbiter/src/declaration.rs`
- Create: `crates/continuum-arbiter/src/priority.rs`
- Create: `crates/continuum-arbiter/src/assembly.rs`
- Modify: `crates/continuum-arbiter/src/lib.rs`（`mod priority;`、`#[cfg(feature = "assembly")] pub mod assembly;`、
  导出 `WorkloadDeclaration`）
- Modify: `crates/continuum-arbiter/Cargo.toml`（追加 `[[test]]` 门控）
- Create: `crates/continuum-arbiter/tests/assembly.rs`

**Interfaces:**
- Consumes: Task 2 的 `ResourceArbiter`／`ResourceSummary`
- Produces:
  - `continuum_arbiter::WorkloadDeclaration { pub user_process: bool, pub foreground: bool,
    pub maintenance: bool }`；派生 `Debug, Clone, Copy, PartialEq, Eq`，**无 `Default`**
  - `crate::priority::PriorityTier`：`UserWorkload`／`ForegroundRuntime`／`BackgroundRuntime`／
    `Maintenance`；`pub(crate) fn classify(WorkloadDeclaration) -> PriorityTier`
  - `continuum_arbiter::assembly::{PriorityTier, AssemblyView}`；
    `AssemblyView::summary()`／`priority()`；
    `ResourceArbiter::assembly_view(&self, declaration: WorkloadDeclaration) -> AssemblyView`
  - `#[cfg(feature = "assembly")]` 下 `PriorityTier::as_str()`（逐字 §300 名字，含空格）与
    `rank()`（3／2／1／0）

- [ ] **Step 1: 写 `tests/assembly.rs`**

```rust
use continuum_arbiter::assembly::{AssemblyView, PriorityTier};
use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourcePressure, WorkloadDeclaration,
};

fn arbiter() -> ResourceArbiter {
    ResourceArbiter::new(ArbiterPolicy::default()).unwrap()
}

const USER: WorkloadDeclaration =
    WorkloadDeclaration { user_process: true, foreground: false, maintenance: false };
const FOREGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: true, maintenance: false };
const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };
const MAINTENANCE: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: true };

#[test]
fn classification_follows_maintenance_user_foreground_default() {
    assert_eq!(arbiter().assembly_view(MAINTENANCE).priority(), PriorityTier::Maintenance);
    assert_eq!(arbiter().assembly_view(USER).priority(), PriorityTier::UserWorkload);
    assert_eq!(arbiter().assembly_view(FOREGROUND).priority(), PriorityTier::ForegroundRuntime);
    assert_eq!(arbiter().assembly_view(BACKGROUND).priority(), PriorityTier::BackgroundRuntime);
    let both = WorkloadDeclaration {
        user_process: true, foreground: true, maintenance: true,
    };
    assert_eq!(arbiter().assembly_view(both).priority(), PriorityTier::Maintenance);
    let user_and_foreground = WorkloadDeclaration {
        user_process: true, foreground: true, maintenance: false,
    };
    assert_eq!(
        arbiter().assembly_view(user_and_foreground).priority(),
        PriorityTier::UserWorkload
    );
}

#[test]
fn priority_tier_text_and_rank_are_verbatim() {
    assert_eq!(PriorityTier::UserWorkload.as_str(), "User workload");
    assert_eq!(PriorityTier::ForegroundRuntime.as_str(), "Foreground Runtime");
    assert_eq!(PriorityTier::BackgroundRuntime.as_str(), "Background Runtime");
    assert_eq!(PriorityTier::Maintenance.as_str(), "Maintenance");
    assert_eq!(PriorityTier::UserWorkload.rank(), 3);
    assert_eq!(PriorityTier::ForegroundRuntime.rank(), 2);
    assert_eq!(PriorityTier::BackgroundRuntime.rank(), 1);
    assert_eq!(PriorityTier::Maintenance.rank(), 0);
}

#[test]
fn assembly_view_exposes_summary_and_priority_only() {
    let arbiter = arbiter();
    let view = arbiter.assembly_view(BACKGROUND);
    fn reads(view: &AssemblyView) -> (continuum_arbiter::ResourceSummary, PriorityTier) {
        (view.summary(), view.priority())
    }
    let _: fn(&AssemblyView) -> (continuum_arbiter::ResourceSummary, PriorityTier) = reads;
    assert_eq!(view.summary().resource_pressure(), ResourcePressure::Low);
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test assembly
```

**结果：** `E0432`（`continuum_arbiter::assembly` 不存在）。这是编译失败，**不算红**。**注意**：此刻
`tests/assembly.rs` 会被默认测试面自动发现——这正是 Step 3 要装门控的原因。

- [ ] **Step 3: 装 `[[test]]` 门控（单独一步）**

在 `crates/continuum-arbiter/Cargo.toml` 末尾加：

```toml
# `tests/assembly.rs` 只在启用 assembly 特性时构建；默认 `cargo test -p continuum-arbiter`
# 不得编译它（否则 `continuum_arbiter::assembly` 不存在，默认全套测试红）。
[[test]]
name = "assembly"
required-features = ["assembly"]
```

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test assembly
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --no-run
```

**结果：** 第一条以 cargo 的“target `assembly` requires the features: `assembly`”结束（不是编译错误）；
第二条成功，且**没有编译** `tests/assembly.rs`。

- [ ] **Step 4: 实现 `WorkloadDeclaration`、`PriorityTier` 与 `assembly` 模块**

`src/declaration.rs` 按 §4.5；`src/priority.rs` 定义私有模块内的 `PriorityTier` 与唯一的
`pub(crate) fn classify`（顺序：`maintenance` → `user_process` → `foreground` → 默认
`BackgroundRuntime`；§5.3）。`PriorityTier::as_str`／`rank` 带 `#[cfg(feature = "assembly")]`。
`src/assembly.rs` 只做：

```rust
pub use crate::priority::PriorityTier;

pub struct AssemblyView { summary: ResourceSummary, priority: PriorityTier }
impl AssemblyView { pub fn summary(&self) -> ResourceSummary; pub fn priority(&self) -> PriorityTier; }

impl ResourceArbiter {
    pub fn assembly_view(&self, declaration: WorkloadDeclaration) -> AssemblyView;
}
```

`src/lib.rs` 必须是 `mod priority;`（**不是** `pub mod`），并加
`#[cfg(feature = "assembly")] pub mod assembly;`。`assembly_view` 用 `crate::priority::classify`，
**不**在装配方或 `assembly.rs` 里写第二份 `match`。

- [ ] **Step 5: 跑装配测试与默认测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --features assembly --test assembly
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter
```

**结果：** 第一条全绿（3 条）；第二条全绿且 0 warning，`assembly` 目标不在默认测试面。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-arbiter/Cargo.toml \
  crates/continuum-arbiter/src/lib.rs \
  crates/continuum-arbiter/src/declaration.rs \
  crates/continuum-arbiter/src/priority.rs \
  crates/continuum-arbiter/src/assembly.rs \
  crates/continuum-arbiter/tests/assembly.rs
git commit -m "feat(p7c): 四档优先级与装配方特性面"
```

---

### Task 4: 四档动作、两阶段挂起、动作阶梯与结论值

**Files:**
- Create: `crates/continuum-arbiter/src/action.rs`
- Create: `crates/continuum-arbiter/src/state.rs`
- Modify: `crates/continuum-arbiter/src/arbiter.rs`（`ResourceDecision`、`direct`、4×4 阶梯）
- Modify: `crates/continuum-arbiter/src/lib.rs`（导出新类型）
- Create: `crates/continuum-arbiter/tests/action.rs`
- Create: `crates/continuum-arbiter/tests/ladder.rs`
- Create: `crates/continuum-arbiter/tests/without_model.rs`
- Create: `crates/continuum-arbiter/tests/suspension.rs`

**Interfaces:**
- Consumes: Task 1–3 的全部类型
- Produces:
  - `continuum_arbiter::ResourceAction`：`Run`／`Throttle`／`Suspend`／`Resume`；`as_str`／`parse`
  - `continuum_arbiter::SuspensionPhase`：`Running`／`Throttled`／`Checkpointing`／`Suspended`；
    `as_str`／`parse`（**没有 `Resume` 相位**）
  - `continuum_arbiter::SuspensionState`：`new()`、`phase()`、`checkpoint_deadline_millis()`、
    `checkpoint_succeeded(&mut self, now) -> Result<SuspensionPhase, SuspensionError>`、
    `checkpoint_failed(&mut self, now) -> SuspensionError`、
    `checkpoint_timed_out(&mut self, now) -> SuspensionError`
  - `continuum_arbiter::SuspensionError`：`CheckpointFailed`／`CheckpointTimedOut`／
    `NotCheckpointing { phase: SuspensionPhase }`；手写 `Display`／`Error`
  - `continuum_arbiter::ResourceDecision`：`summary()`／`action()`／`phase()`；字段私有
  - `ResourceArbiter::direct(&self, declaration: WorkloadDeclaration,
    state: &mut SuspensionState, now_millis: u64) -> Result<ResourceDecision, SuspensionError>`

- [ ] **Step 1: 写动作、16 格阶梯、J2 与 J8／J9 用例**

`tests/action.rs`：

```rust
use continuum_arbiter::{ResourceAction, SuspensionPhase};

#[test]
fn action_text_round_trips_and_has_exactly_four_variants() {
    for action in [ResourceAction::Run, ResourceAction::Throttle,
                   ResourceAction::Suspend, ResourceAction::Resume] {
        assert_eq!(ResourceAction::parse(action.as_str()), Some(action));
    }
    fn name(action: ResourceAction) -> &'static str {
        match action {
            ResourceAction::Run => "run",
            ResourceAction::Throttle => "throttle",
            ResourceAction::Suspend => "suspend",
            ResourceAction::Resume => "resume",
        }
    }
    assert_eq!(name(ResourceAction::Suspend), "suspend");
}

#[test]
fn phase_text_round_trips_and_has_exactly_four_variants() {
    for phase in [SuspensionPhase::Running, SuspensionPhase::Throttled,
                  SuspensionPhase::Checkpointing, SuspensionPhase::Suspended] {
        assert_eq!(SuspensionPhase::parse(phase.as_str()), Some(phase));
    }
    fn name(phase: SuspensionPhase) -> &'static str {
        match phase {
            SuspensionPhase::Running => "Running",
            SuspensionPhase::Throttled => "Throttled",
            SuspensionPhase::Checkpointing => "Checkpointing",
            SuspensionPhase::Suspended => "Suspended",
        }
    }
    assert_eq!(name(SuspensionPhase::Checkpointing), "Checkpointing");
}
```

`tests/ladder.rs` 逐格列 16 行（**每格新建 arbiter 与 state**；先断言压力等于目标档，再 `direct`）：

```rust
use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, ResourcePressure,
    SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

struct Cell {
    declaration: WorkloadDeclaration,
    cpu: u8,
    pressure: ResourcePressure,
    action: Option<ResourceAction>,
    phase: SuspensionPhase,
}

const USER: WorkloadDeclaration =
    WorkloadDeclaration { user_process: true, foreground: false, maintenance: false };
const FOREGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: true, maintenance: false };
const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };
const MAINTENANCE: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: true };

const CELLS: [Cell; 16] = [
    Cell { declaration: USER, cpu: 10, pressure: ResourcePressure::Low,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: USER, cpu: 60, pressure: ResourcePressure::Moderate,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: USER, cpu: 80, pressure: ResourcePressure::High,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: USER, cpu: 95, pressure: ResourcePressure::Critical,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: FOREGROUND, cpu: 10, pressure: ResourcePressure::Low,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: FOREGROUND, cpu: 60, pressure: ResourcePressure::Moderate,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: FOREGROUND, cpu: 80, pressure: ResourcePressure::High,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: FOREGROUND, cpu: 95, pressure: ResourcePressure::Critical,
           action: Some(ResourceAction::Throttle), phase: SuspensionPhase::Throttled },
    Cell { declaration: BACKGROUND, cpu: 10, pressure: ResourcePressure::Low,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: BACKGROUND, cpu: 60, pressure: ResourcePressure::Moderate,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: BACKGROUND, cpu: 80, pressure: ResourcePressure::High,
           action: Some(ResourceAction::Throttle), phase: SuspensionPhase::Throttled },
    Cell { declaration: BACKGROUND, cpu: 95, pressure: ResourcePressure::Critical,
           action: Some(ResourceAction::Suspend), phase: SuspensionPhase::Checkpointing },
    Cell { declaration: MAINTENANCE, cpu: 10, pressure: ResourcePressure::Low,
           action: None, phase: SuspensionPhase::Running },
    Cell { declaration: MAINTENANCE, cpu: 60, pressure: ResourcePressure::Moderate,
           action: Some(ResourceAction::Throttle), phase: SuspensionPhase::Throttled },
    Cell { declaration: MAINTENANCE, cpu: 80, pressure: ResourcePressure::High,
           action: Some(ResourceAction::Suspend), phase: SuspensionPhase::Checkpointing },
    Cell { declaration: MAINTENANCE, cpu: 95, pressure: ResourcePressure::Critical,
           action: Some(ResourceAction::Suspend), phase: SuspensionPhase::Checkpointing },
];

#[test]
fn every_cell_of_the_four_by_four_ladder_is_explicit() {
    for cell in CELLS {
        let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
        let mut state = SuspensionState::new();
        let observation = ResourceObservation { cpu: Some(cell.cpu), ..Default::default() };
        assert_eq!(arbiter.observe(&observation, 0).resource_pressure(), cell.pressure);
        let decision = arbiter.direct(cell.declaration, &mut state, 0).unwrap();
        assert_eq!(decision.action(), cell.action, "cpu={}", cell.cpu);
        assert_eq!(decision.phase(), cell.phase, "cpu={}", cell.cpu);
        assert_eq!(state.phase(), cell.phase, "cpu={}", cell.cpu);
    }
}
```

`tests/without_model.rs`：

```rust
use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, SuspensionPhase,
    SuspensionState, WorkloadDeclaration,
};

fn critical() -> ResourceArbiter {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    arbiter.observe(&ResourceObservation { cpu: Some(95), ..Default::default() }, 0);
    arbiter
}

#[test]
fn high_pressure_limits_background_and_maintenance_without_any_model_path() {
    let arbiter = critical();
    let mut background = SuspensionState::new();
    let decision = arbiter
        .direct(WorkloadDeclaration { user_process: false, foreground: false, maintenance: false },
                &mut background, 0)
        .unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);

    let mut maintenance = SuspensionState::new();
    let decision = arbiter
        .direct(WorkloadDeclaration { user_process: false, foreground: false, maintenance: true },
                &mut maintenance, 0)
        .unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);

    let mut user = SuspensionState::new();
    let decision = arbiter
        .direct(WorkloadDeclaration { user_process: true, foreground: false, maintenance: false },
                &mut user, 0)
        .unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}

#[test]
fn per_task_state_is_required_for_independent_decisions() {
    let arbiter = critical();
    let mut reused = SuspensionState::new();
    let background = WorkloadDeclaration {
        user_process: false, foreground: false, maintenance: false
    };
    assert_eq!(
        arbiter.direct(background, &mut reused, 0).unwrap().phase(),
        SuspensionPhase::Checkpointing
    );
    let maintenance = WorkloadDeclaration {
        user_process: false, foreground: false, maintenance: true
    };
    assert_eq!(arbiter.direct(maintenance, &mut reused, 0).unwrap().action(), None);
    assert_eq!(reused.phase(), SuspensionPhase::Checkpointing);

    let mut fresh = SuspensionState::new();
    assert_eq!(
        arbiter.direct(maintenance, &mut fresh, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(fresh.phase(), SuspensionPhase::Checkpointing);
}
```

`tests/suspension.rs`：

```rust
use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, ResourcePressure,
    SuspensionError, SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };

fn critical() -> ResourceArbiter {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    arbiter.observe(&ResourceObservation { cpu: Some(95), ..Default::default() }, 0);
    arbiter
}

#[test]
fn suspend_only_reaches_checkpointing_and_success_reaches_suspended() {
    let arbiter = critical();
    let mut state = SuspensionState::new();
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    assert!(state.checkpoint_deadline_millis().is_some());
    assert_eq!(state.checkpoint_succeeded(0), Ok(SuspensionPhase::Suspended));
    assert_eq!(state.phase(), SuspensionPhase::Suspended);
    assert_eq!(state.checkpoint_deadline_millis(), None);
}

#[test]
fn resume_uses_the_suspension_state_clock_not_the_pressure_clock() {
    let mut arbiter = critical();
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(state.checkpoint_succeeded(0), Ok(SuspensionPhase::Suspended));
    arbiter.observe(&ResourceObservation { cpu: Some(10), ..Default::default() }, 30_000);
    assert_eq!(arbiter.summary().resource_pressure(), ResourcePressure::Low);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Resume));
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}

#[test]
fn resume_waits_for_the_state_cooldown_even_when_pressure_is_low() {
    let mut arbiter = critical();
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(state.checkpoint_succeeded(20_000), Ok(SuspensionPhase::Suspended));
    arbiter.observe(&ResourceObservation { cpu: Some(10), ..Default::default() }, 30_000);
    assert_eq!(arbiter.summary().resource_pressure(), ResourcePressure::Low);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Suspended);
    let decision = arbiter.direct(BACKGROUND, &mut state, 50_000).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Resume));
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}

#[test]
fn checkpoint_failure_returns_to_throttled_and_never_suspends() {
    let arbiter = critical();
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(state.checkpoint_failed(1), SuspensionError::CheckpointFailed);
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    assert_eq!(
        state.checkpoint_succeeded(2),
        Err(SuspensionError::NotCheckpointing { phase: SuspensionPhase::Throttled })
    );
    assert_ne!(state.phase(), SuspensionPhase::Suspended);
}

#[test]
fn checkpoint_calls_outside_checkpointing_report_the_current_phase() {
    let mut state = SuspensionState::new();
    assert_eq!(
        state.checkpoint_failed(3),
        SuspensionError::NotCheckpointing { phase: SuspensionPhase::Running }
    );
    assert_eq!(state.phase(), SuspensionPhase::Running);
    assert_eq!(
        state.checkpoint_timed_out(4),
        SuspensionError::NotCheckpointing { phase: SuspensionPhase::Running }
    );
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test action --test ladder --test without_model --test suspension
```

**结果：** `E0432`／`E0599`（动作、状态机、`direct` 尚不存在）。编译失败，**不算红**。

- [ ] **Step 3: 实现动作、状态机与 `direct`（不含超时／重试上限）**

`src/action.rs`／`src/state.rs` 按 §4.7–§4.9。`SuspensionState::new()` 为
`Running`／`attempts=0`／`last_transition_millis=0`／`deadline=None`。`checkpoint_succeeded` 只允许
从 `Checkpointing`：成功 → `Suspended`、清空截止、`last_transition=now`；**本 task 不检查截止**（J10 在
Task 5）。`checkpoint_failed`／`checkpoint_timed_out` 只允许从 `Checkpointing`：回 `Throttled`、
清空截止、`last_transition=now`；错相位返回 `NotCheckpointing { phase }`，相位不变。
`checkpoint_timed_out` 本 task 就实现（无条件从 `Checkpointing` 回 `Throttled`）。
`src/arbiter.rs` 加 `ResourceDecision`（字段私有，三个访问器）与 `direct`：

1. `Checkpointing` 相位：**Task 5 前不检查截止**，返回 `Ok(action=None, phase=Checkpointing)`；
2. `Suspended` 相位：若 `pressure <= policy.resume_at` 且
   `now.saturating_sub(state.last_transition_millis) >= policy.cooldown_millis` → `Resume`／`Running`，
   清空截止、`last_transition=now`；否则 `None`／`Suspended`。**本 task 不重置 `attempts`**（Task 5 补）；
3. 其余相位按 §5.4 的 4×4 表选动作，再按 §5.5 转移：
   `Run` 从 `Throttled` → `Running`；`Throttle` 从 `Running` → `Throttled`；
   `Suspend` 从 `Running`／`Throttled` → `Checkpointing`，`attempts += 1`、设置
   `deadline = now.saturating_add(policy.checkpoint_timeout_millis)`、`last_transition=now`；
   **每一条实际转移都把 `last_transition_millis` 设为 `now_millis`**；`action=None` 的 no-op 不更新
   `last_transition`。**本 task 不实现 `attempts >= max` 的退化、不实现重试 cooldown、不重置
   `attempts`**（J11 与 `Run`／`Resume` 重置都在 Task 5）。

- [ ] **Step 4: 跑本 task 的测试**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test action --test ladder --test without_model --test suspension
```

**结果：** 全绿（`action` 2 条、`ladder` 1 条 16 格、`without_model` 2 条、`suspension` 5 条），0 warning。
**J10／J11 此时应红**（尚未写用例）；Task 5 补。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-arbiter/src/lib.rs \
  crates/continuum-arbiter/src/action.rs \
  crates/continuum-arbiter/src/state.rs \
  crates/continuum-arbiter/src/arbiter.rs \
  crates/continuum-arbiter/tests/action.rs \
  crates/continuum-arbiter/tests/ladder.rs \
  crates/continuum-arbiter/tests/without_model.rs \
  crates/continuum-arbiter/tests/suspension.rs
git commit -m "feat(p7c): 四档动作、动作阶梯与两阶段挂起"
```

---

### Task 5: 超时、重试上限、`max=0` 与 `Resume` 重置

**Files:**
- Modify: `crates/continuum-arbiter/src/state.rs`（`checkpoint_succeeded` 的截止检查）
- Modify: `crates/continuum-arbiter/src/arbiter.rs`（`direct` 的截止检查、重试上限、`max=0` 退化）
- Create: `crates/continuum-arbiter/tests/timeout.rs`
- Create: `crates/continuum-arbiter/tests/retry.rs`

**Interfaces:**
- Consumes: Task 4 的 `SuspensionState`／`SuspensionError`／`direct`
- Produces: **行为补全**，不新增公开类型；`SuspensionError::CheckpointTimedOut` 的出口从 Task 4 的
  `checkpoint_timed_out` 扩展到 `direct`／`checkpoint_succeeded` 的截止检查

- [ ] **Step 1: 写 J10／J11 用例**

`tests/timeout.rs`（`checkpoint_timeout_millis = 100`）：

```rust
use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourceObservation, SuspensionError,
    SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };

fn critical_with_timeout(timeout: u64) -> ResourceArbiter {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        checkpoint_timeout_millis: timeout, ..ArbiterPolicy::default()
    }).unwrap();
    arbiter.observe(&ResourceObservation { cpu: Some(95), ..Default::default() }, 0);
    arbiter
}

#[test]
fn direct_at_the_deadline_returns_to_throttled() {
    let arbiter = critical_with_timeout(100);
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().phase(),
        SuspensionPhase::Checkpointing
    );
    assert_eq!(state.checkpoint_deadline_millis(), Some(100));
    let decision = arbiter.direct(BACKGROUND, &mut state, 99).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 100),
        Err(SuspensionError::CheckpointTimedOut)
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    assert_eq!(state.checkpoint_deadline_millis(), None);
}

#[test]
fn late_success_is_rejected_after_the_deadline() {
    let arbiter = critical_with_timeout(100);
    let mut state = SuspensionState::new();
    arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(
        state.checkpoint_succeeded(100),
        Err(SuspensionError::CheckpointTimedOut)
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
}

#[test]
fn the_timer_hook_returns_to_throttled() {
    let arbiter = critical_with_timeout(100);
    let mut state = SuspensionState::new();
    arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(state.checkpoint_timed_out(50), SuspensionError::CheckpointTimedOut);
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    assert_eq!(state.checkpoint_deadline_millis(), None);
}

#[test]
fn timer_hook_outside_checkpointing_reports_the_phase() {
    let mut state = SuspensionState::new();
    assert_eq!(
        state.checkpoint_timed_out(50),
        SuspensionError::NotCheckpointing { phase: SuspensionPhase::Running }
    );
}
```

`tests/retry.rs` 五条（**每条独立 arbiter／state**）：

```rust
use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, SuspensionError,
    SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };

fn observe(arbiter: &mut ResourceArbiter, cpu: u8, now: u64) {
    arbiter.observe(&ResourceObservation { cpu: Some(cpu), ..Default::default() }, now);
}

#[test]
fn attempts_are_capped_and_run_resets_the_budget() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 2, cooldown_millis: 0, ..ArbiterPolicy::default()
    }).unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
               Some(ResourceAction::Suspend));
    assert_eq!(state.checkpoint_failed(0), SuspensionError::CheckpointFailed);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
               Some(ResourceAction::Suspend));
    assert_eq!(state.checkpoint_failed(0), SuspensionError::CheckpointFailed);
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Throttled);

    observe(&mut arbiter, 10, 1);
    let decision = arbiter.direct(BACKGROUND, &mut state, 1).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Run));
    assert_eq!(decision.phase(), SuspensionPhase::Running);

    observe(&mut arbiter, 95, 2);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 2).unwrap().action(),
               Some(ResourceAction::Suspend));
}

#[test]
fn resume_resets_the_checkpoint_attempt_budget() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 1, cooldown_millis: 0, ..ArbiterPolicy::default()
    }).unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
               Some(ResourceAction::Suspend));
    assert_eq!(state.checkpoint_succeeded(0), Ok(SuspensionPhase::Suspended));
    observe(&mut arbiter, 10, 30_000);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Resume));
    assert_eq!(decision.phase(), SuspensionPhase::Running);
    observe(&mut arbiter, 95, 30_001);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_001).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
}

#[test]
fn retry_waits_for_the_state_clock_cooldown() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 3, cooldown_millis: 1_000, ..ArbiterPolicy::default()
    }).unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
               Some(ResourceAction::Suspend));
    assert_eq!(state.checkpoint_failed(1), SuspensionError::CheckpointFailed);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 500).unwrap().action(), None);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 500).unwrap().phase(),
               SuspensionPhase::Throttled);
    assert_eq!(arbiter.direct(BACKGROUND, &mut state, 1_001).unwrap().action(),
               Some(ResourceAction::Suspend));
}

#[test]
fn the_checkpointing_window_is_not_interrupted_while_retrying() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 3, ..ArbiterPolicy::default()
    }).unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    let decision = arbiter.direct(BACKGROUND, &mut state, 49).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    assert_eq!(state.checkpoint_succeeded(50), Ok(SuspensionPhase::Suspended));
}

#[test]
fn zero_attempts_only_throttles() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 0, ..ArbiterPolicy::default()
    }).unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Throttle));
    assert_eq!(decision.phase(), SuspensionPhase::Throttled);
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Throttled);
}
```

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test timeout --test retry
```

**结果：** 运行期红：
- `direct_at_the_deadline_returns_to_throttled` 得到 `Ok(Checkpointing)` 而不是 `Err(CheckpointTimedOut)`；
- `late_success_is_rejected_after_the_deadline` 得到 `Ok(Suspended)`；
- `attempts_are_capped...` 第三次仍得到 `Suspend`；
- `retry_waits...` 在 500 就得到 `Suspend`；
- `zero_attempts_only_throttles` 得到 `Suspend`／`Checkpointing`。
- **`resume_resets_the_checkpoint_attempt_budget` 与 `the_checkpointing_window_is_not_interrupted_while_retrying`
  在这一步先绿**：Task 4 还没有上限抑制（所以 `Resume` 后总会 `Suspend`），也没有打断
  `Checkpointing`。它们不是“先红后绿”的例外，而是 Task 5 实现“上限 + 重置”组合的守卫；
  去掉 `Resume` 重置后 `resume_resets...` 会红，见 Task 8 的变异复核。

- [ ] **Step 3: 实现截止检查、重试上限与 `max=0`**

`src/arbiter.rs` 的 `direct` 开头加：若 `Checkpointing` 且 `now >= deadline`，相位回 `Throttled`、
清空截止、`last_transition=now`，返回 `Err(CheckpointTimedOut)`。
`src/state.rs` 的 `checkpoint_succeeded` 加同一截止检查（截止后回 `Throttled`、清空截止、
`last_transition=now`，并返回 `Err(CheckpointTimedOut)`；晚到成功不报 `Suspended`）。
`direct` 的 `Suspend` 分支按 §5.6：

- `state.attempts >= policy.max_checkpoint_attempts` 时：`Running` → 发 `Throttle`、相位 `Throttled`
  （**不增加 `attempts`**）；`Throttled` → `action=None`、相位保持 `Throttled`；
- 否则从 `Throttled` 重试（`attempts > 0`）需
  `now.saturating_sub(state.last_transition_millis) >= policy.cooldown_millis`；不满足 → `action=None`、
  保持 `Throttled`；首次（`attempts == 0`）不要求 cooldown；
- 通过后 `attempts += 1`、设置截止、`last_transition=now`、相位 `Checkpointing`。
`Run`（从 `Throttled`）与 `Resume`（从 `Suspended`）重置 `attempts=0`；`checkpoint_succeeded` **不**重置。

- [ ] **Step 4: 跑本 task 的测试与 T4 回归**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter --test timeout --test retry
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-arbiter
```

**结果：** 全绿（`timeout` 4 条、`retry` 5 条；默认全套绿），0 warning。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-arbiter/src/state.rs \
  crates/continuum-arbiter/src/arbiter.rs \
  crates/continuum-arbiter/tests/timeout.rs \
  crates/continuum-arbiter/tests/retry.rs
git commit -m "feat(p7c): checkpoint 超时与重试上限"
```

---

### Task 6: 默认公开面、源码纪律与编译失败夹具

**Files:**
- Create: `crates/continuum-arbiter/tests/support/mod.rs`
- Create: `crates/continuum-arbiter/tests/source_surface.rs`
- Create: `crates/continuum-arbiter/tests/compile_fail.rs`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/Cargo.toml`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/.gitignore`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/priority.rs`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/assembly_view.rs`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/summary_priority.rs`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/summary_literal.rs`
- Create: `crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/decision_priority.rs`

**Interfaces:**
- Consumes: Task 1–5 的全部公开面
- Produces: 无生产类型；源码面 helper：
  - `support::src_files() -> Vec<(&'static str, String)>`（**原文**，用于精确行匹配）
  - `support::code_files() -> Vec<(&'static str, String)>`（对每个文件应用 `code_only`）
  - `support::code_only(&str) -> String`（去 `//`／`///`／`/* */` 注释与字符串字面量，保留换行）
  - `support::struct_block(&str, &str) -> &str`／`support::impl_block(&str, &str) -> &str`
    （花括号配对；返回 `{` 与配对 `}` **之间**的体）
  - `support::field_names(&str) -> Vec<&str>`／`support::public_fn_names(&str) -> Vec<&str>`
  - `support::contains_token(&str, &str) -> bool`（按标识符边界匹配）

- [ ] **Step 1: 写 helper 与 `tests/source_surface.rs`**

`tests/support/mod.rs` 先实现并自测 helper（`code_only` 至少覆盖 `//`、`///`、`/* */`、普通字符串与
`r#"…"#` 原始字符串；`struct_block`／`impl_block` 用花括号计数，遇到字符串中的花括号先由 `code_only`
抹掉）。`tests/source_surface.rs` 至少包含下面这些用例：

```rust
mod support;

use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourceDecision, ResourceObservation, ResourceSummary,
    SuspensionState, WorkloadDeclaration,
};

#[test]
fn the_source_helper_strips_comments_and_strings() {
    let source = "// priority\nlet x = \"PriorityTier\"; /* pid */ let y = 1;";
    let code = support::code_only(source);
    assert!(!code.contains("priority"));
    assert!(!code.contains("PriorityTier"));
    assert!(!code.contains("pid"));
    assert!(code.contains("let y = 1;"));
}

#[test]
fn summary_has_exactly_two_public_accessors() {
    let files = support::code_files();
    let hits: Vec<_> = files.iter().filter(|(_, text)| text.contains("impl ResourceSummary")).collect();
    assert_eq!(hits.len(), 1, "impl ResourceSummary 必须恰一处");
    let block = support::impl_block(&hits[0].1, "impl ResourceSummary");
    assert_eq!(support::public_fn_names(block), vec!["resource_pressure", "workload_class"]);
    let summary = files.iter().find(|(name, _)| *name == "summary.rs").unwrap().1.clone();
    let fields = support::struct_block(&summary, "pub struct ResourceSummary");
    assert_eq!(support::field_names(fields),
               vec!["resource_pressure", "workload_class"]);
    assert!(!fields.contains("pub "), "ResourceSummary 字段必须私有");
}

#[test]
fn summary_source_has_no_priority_or_process_terms() {
    let files = support::code_files();
    let summary = files.iter().find(|(name, _)| *name == "summary.rs").unwrap().1.clone();
    let fields = support::struct_block(&summary, "pub struct ResourceSummary");
    let block = support::impl_block(&summary, "impl ResourceSummary");
    for token in ["priority", "PriorityTier", "process_name", "process_id", "pid",
                  "cpu_usage", "gpu_usage"] {
        assert!(!support::contains_token(fields, token), "summary 字段出现 {token}");
        assert!(!support::contains_token(block, token), "summary impl 出现 {token}");
    }
}

#[test]
fn decision_has_exactly_three_public_accessors_and_no_priority() {
    let files = support::code_files();
    let hits: Vec<_> = files.iter().filter(|(_, text)| text.contains("impl ResourceDecision")).collect();
    assert_eq!(hits.len(), 1, "impl ResourceDecision 必须恰一处");
    let block = support::impl_block(&hits[0].1, "impl ResourceDecision");
    assert_eq!(support::public_fn_names(block), vec!["summary", "action", "phase"]);
    assert!(!block.contains("PriorityTier"));
    let arbiter = files.iter().find(|(name, _)| *name == "arbiter.rs").unwrap().1.clone();
    let fields = support::struct_block(&arbiter, "pub struct ResourceDecision");
    assert_eq!(support::field_names(fields), vec!["summary", "action", "phase"]);
    assert!(!fields.contains("pub "), "ResourceDecision 字段必须私有");
    assert!(!fields.contains("priority") && !fields.contains("PriorityTier"));
}

#[test]
fn priority_tier_is_private_by_default_and_only_assembly_re_exports_it() {
    let files = support::src_files();
    let lib = files.iter().find(|(name, _)| *name == "lib.rs").unwrap().1.clone();
    assert!(lib.lines().any(|line| line.trim_start().starts_with("mod priority;")));
    assert!(!lib.lines().any(|line| line.trim_start().starts_with("pub mod priority;")));
    assert!(lib.contains("#[cfg(feature = \"assembly\")] pub mod assembly;"));
    let reexports: Vec<&str> = files.iter()
        .filter(|(_, text)| text.contains("pub use crate::priority::PriorityTier;"))
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(reexports, vec!["assembly.rs"]);
}

#[test]
fn priority_tier_is_confined_to_three_source_files() {
    let files = support::code_files();
    let hits: Vec<&str> = files.iter()
        .filter(|(_, text)| text.contains("PriorityTier"))
        .map(|(name, _)| *name)
        .collect();
    assert!(hits.iter().all(|name| matches!(*name, "priority.rs" | "assembly.rs" | "arbiter.rs")),
            "PriorityTier 出现在白名单外：{hits:?}");
    let priority = files.iter().find(|(name, _)| *name == "priority.rs").unwrap().1.clone();
    assert_eq!(priority.matches("fn classify(").count(), 1,
               "四档分类函数必须只有一个");
}

#[test]
fn no_raw_process_information_in_public_types() {
    let files = support::code_files();
    let all: String = files.iter().map(|(_, text)| text.as_str()).collect();
    for token in ["process_name", "process_id", "cpu_usage", "gpu_usage",
                  "command_line", "working_directory", "window_title"] {
        assert!(!all.contains(token), "公开语料出现 {token}");
    }
    assert!(!support::contains_token(&all, "pid"), "公开语料出现 pid");
    assert_eq!(
        support::field_names(support::struct_block(
            &files.iter().find(|(name, _)| *name == "pressure.rs").unwrap().1,
            "pub struct ResourceObservation")),
        vec!["cpu", "ram", "gpu", "vram", "disk_io", "network", "workload_class"]
    );
    assert_eq!(
        support::field_names(support::struct_block(
            &files.iter().find(|(name, _)| *name == "arbiter.rs").unwrap().1,
            "pub struct ResourceDecision")),
        vec!["summary", "action", "phase"]
    );
    assert_eq!(
        support::field_names(support::struct_block(
            &files.iter().find(|(name, _)| *name == "state.rs").unwrap().1,
            "pub struct SuspensionState")),
        vec!["phase", "attempts", "last_transition_millis", "checkpoint_deadline_millis"]
    );
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    let summary = arbiter.observe(&ResourceObservation::default(), 0);
    assert!(!format!("{summary:?}").contains("Priority"));
    assert!(!format!("{summary:?}").contains("pid"));
    assert!(!format!("{summary:?}").contains("process"));
    let mut state = SuspensionState::new();
    let decision: ResourceDecision = arbiter
        .direct(WorkloadDeclaration { user_process: false, foreground: false, maintenance: false },
                &mut state, 0)
        .unwrap();
    assert!(!format!("{decision:?}").contains("Priority"));
    assert!(!format!("{state:?}").contains("process"));
}

#[test]
fn no_resource_scheduler_budget_or_persistence_surface() {
    let files = support::code_files();
    let all: String = files.iter().map(|(_, text)| text.as_str()).collect();
    for token in ["continuum_provider", "continuum_model_registry", "continuum_node",
                  "continuum_graph", "continuum_budget", "RoutingRequest", "PlacementRequest",
                  "ModelProvider", "select_runnable", "Migration", "Tx", "persist"] {
        assert!(!all.contains(token), "src/ 出现禁用符号 {token}");
    }
    let arbiter_blocks: Vec<&str> = files.iter()
        .filter(|(_, text)| text.contains("impl ResourceArbiter"))
        .map(|(_, text)| support::impl_block(text, "impl ResourceArbiter"))
        .collect();
    for block in arbiter_blocks {
        for method in ["place", "rank", "model", "tool", "parallelism"] {
            assert!(!support::public_fn_names(block).contains(&method),
                    "ResourceArbiter 出现决策方法 {method}");
        }
    }
}

#[test]
fn no_clock_random_process_filesystem_network_environment_or_unsafe() {
    let all: String = support::code_files().iter().map(|(_, text)| text.as_str()).collect();
    for token in ["SystemTime::now", "Instant::now", "UNIX_EPOCH", "thread_rng", "rand::",
                  "std::process", "Command::new", "kill(", "std::fs", "std::net", "std::env",
                  "unsafe", "Kill"] {
        assert!(!all.contains(token), "src/ 出现禁用符号 {token}");
    }
}

#[test]
fn serialization_display_and_default_are_absent_from_conclusion_types() {
    let all: String = support::code_files().iter().map(|(_, text)| text.as_str()).collect();
    for token in ["Serialize", "Deserialize", "serde"] {
        assert!(!all.contains(token), "src/ 出现序列化面 {token}");
    }
    for ty in ["ResourceSummary", "ResourceDecision", "SuspensionState", "ResourceArbiter"] {
        assert!(!all.contains(&format!("impl Display for {ty}")), "{ty} 不应实现 Display");
    }
    assert!(!all.contains("impl Default for ResourceSummary"));
    assert!(!all.contains("impl Default for ResourceDecision"));
    assert!(!all.contains("impl Default for SuspensionState"));
    assert!(!all.contains("impl Default for ResourceArbiter"));
}
```

- [ ] **Step 2: 写五份夹具与 `tests/compile_fail.rs`**

`tests/fixtures/default_surface/Cargo.toml`：

```toml
[package]
name = "default_surface"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
continuum-arbiter = { path = "../../.." }
```

`tests/fixtures/default_surface/.gitignore`：`/target/` 与 `/Cargo.lock`。
五份 bin 的内容分别是：

- `priority.rs`：`use continuum_arbiter::PriorityTier; fn main() {}` → `E0432`
- `assembly_view.rs`：`use continuum_arbiter::AssemblyView; fn main() {}` → `E0432`
- `summary_priority.rs`：构造 `ResourceArbiter::new(Default::default()).unwrap()`，`summary.priority()`
  → `E0599`
- `summary_literal.rs`：`let _ = continuum_arbiter::ResourceSummary { ... };` → `E0451`
- `decision_priority.rs`：`direct(..).unwrap().priority()` → `E0599`

`tests/compile_fail.rs`：

```rust
use std::path::Path;
use std::process::Command;

fn run_bin(name: &str) -> String {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/default_surface");
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui/default_surface");
    let output = Command::new(env!("CARGO"))
        .args(["check", "--quiet", "--bin", name])
        .current_dir(fixture)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("夹具 cargo check 运行失败");
    assert!(!output.status.success(), "{name} 不应在默认构建下通过");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn default_surface_rejects_hidden_priority_and_private_literals() {
    assert!(run_bin("priority").contains("E0432"));
    assert!(run_bin("assembly_view").contains("E0432"));
    assert!(run_bin("summary_priority").contains("E0599"));
    assert!(run_bin("summary_literal").contains("E0451"));
    assert!(run_bin("decision_priority").contains("E0599"));
}
```

- [ ] **Step 3: 运行，确认失败／首次即绿**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter --test source_surface --test compile_fail
```

**结果：** 源码面若全绿，说明 Task 1–5 的实现没有漂；若红，按命中行回查拥有该代码的 task。
编译失败夹具首轮可能因路径或错误码不符而红——按实际 rustc 输出修正夹具或断言，**不改生产代码语义**。

- [ ] **Step 4: 跑默认全套与装配全套**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter --features assembly
```

**结果：** 两套全绿、0 warning。默认套件里 `assembly` 目标不出现。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-arbiter/tests/support/mod.rs \
  crates/continuum-arbiter/tests/source_surface.rs \
  crates/continuum-arbiter/tests/compile_fail.rs \
  crates/continuum-arbiter/tests/fixtures/default_surface/Cargo.toml \
  crates/continuum-arbiter/tests/fixtures/default_surface/.gitignore \
  crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/priority.rs \
  crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/assembly_view.rs \
  crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/summary_priority.rs \
  crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/summary_literal.rs \
  crates/continuum-arbiter/tests/fixtures/default_surface/src/bin/decision_priority.rs
git commit -m "test(p7c): 默认公开面与源码纪律照片"
```

---

### Task 7: 确定性、零表与默认测试面清单

**Files:**
- Create: `crates/continuum-arbiter/tests/determinism.rs`
- Create: `crates/continuum-arbiter/tests/manifest.rs`

**Interfaces:**
- Consumes: Task 1–6 的全部交付物
- Produces: 无生产类型；J17 的确定性照片与 J15 的清单门

- [ ] **Step 1: 写确定性、文本往返与清单用例**

`tests/determinism.rs`：

```rust
use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceDecision, ResourceObservation,
    ResourcePressure, SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };

fn run_once() -> (Vec<ResourcePressure>, Vec<ResourceDecision>, Vec<SuspensionPhase>) {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    let mut state = SuspensionState::new();
    let mut pressures = Vec::new();
    let mut decisions = Vec::new();
    let mut phases = Vec::new();
    for (cpu, now) in [(95, 0), (10, 10_000), (10, 30_000), (95, 30_001)] {
        pressures.push(
            arbiter.observe(&ResourceObservation { cpu: Some(cpu), ..Default::default() }, now)
                .resource_pressure(),
        );
        decisions.push(arbiter.direct(BACKGROUND, &mut state, now).unwrap());
        phases.push(state.phase());
    }
    (pressures, decisions, phases)
}

#[test]
fn the_same_sequence_yields_equal_values() {
    assert_eq!(run_once(), run_once());
}

#[test]
fn action_and_phase_text_round_trip() {
    for action in [ResourceAction::Run, ResourceAction::Throttle,
                   ResourceAction::Suspend, ResourceAction::Resume] {
        assert_eq!(ResourceAction::parse(action.as_str()), Some(action));
    }
    for phase in [SuspensionPhase::Running, SuspensionPhase::Throttled,
                  SuspensionPhase::Checkpointing, SuspensionPhase::Suspended] {
        assert_eq!(SuspensionPhase::parse(phase.as_str()), Some(phase));
    }
}
```

`tests/manifest.rs`：

```rust
use std::path::Path;

const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn the_manifest_is_zero_table_and_gates_the_assembly_target() {
    for forbidden in ["continuum-persist", "continuum-events", "continuum-runtime",
                      "continuum-model-registry", "continuum-node", "continuum-provider",
                      "continuum-graph", "continuum-budget", "serde", "thiserror"] {
        assert!(!MANIFEST.contains(forbidden), "Cargo.toml 出现禁用依赖 {forbidden}");
    }
    assert!(MANIFEST.contains("name = \"assembly\""));
    assert!(MANIFEST.contains("required-features = [\"assembly\"]"));
    let all: String = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    for token in ["Migration", "Tx", "persist"] {
        assert!(!all.contains(token), "src/ 出现持久化符号 {token}");
    }
}

#[test]
fn runtime_migrations_and_startup_do_not_mention_the_arbiter() {
    for source in [
        include_str!("../../continuum-runtime/src/main.rs"),
        include_str!("../../continuum-runtime/tests/migrations.rs"),
        include_str!("../../continuum-runtime/tests/startup.rs"),
    ] {
        assert!(!source.contains("continuum_arbiter"));
        assert!(!source.contains("continuum-arbiter"));
    }
}
```

**`--no-run` 的 artifact 检查放在 Step 3 的 shell 里**，不再由测试进程嵌套调用 cargo：
嵌套 `cargo test --no-run` 可能与外层 cargo 的 target 锁互相等待；J15 的照片是命令输出，不是必须写成
Rust 测试。

- [ ] **Step 2: 运行，确认失败／首次即绿**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter --test determinism --test manifest
```

**结果：** `determinism` 首次即绿（它断言 Task 4／5 的确定性）；`manifest` 若有门控缺口会红——
回 Task 3 修 `[[test]]`，不在此处改语义。

- [ ] **Step 3: 跑默认与装配两套，并核对 artifact**

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter --features assembly
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-arbiter --no-run --message-format=json \
  2>/dev/null | grep -E '"name"[[:space:]]*:[[:space:]]*"assembly"' > .tmp/p7c-assembly-artifacts.txt || true
test ! -s .tmp/p7c-assembly-artifacts.txt
```

**结果：** 两套全绿、0 warning；`p7c-assembly-artifacts.txt` 为空——默认构建没有编译 `assembly` 目标
（J15）。若文件非空，回 Task 3 检查 `required-features`。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-arbiter/tests/determinism.rs \
  crates/continuum-arbiter/tests/manifest.rs
git commit -m "test(p7c): 确定性、零表与默认测试面"
```

---

### Task 8: 全量门、J1(ii) 记录与变异复核

**Files:**
- Modify: `crates/continuum-arbiter/**`（**仅在复核发现缺口时**）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**仅在对账不齐时**）

- [ ] **Step 1: 全量验证**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test -p continuum-arbiter --no-fail-fast 2>&1 | tee .tmp/p7c-default-test.log
TMPDIR="$PWD/.tmp" timeout 1500 cargo test -p continuum-arbiter --features assembly --no-fail-fast 2>&1 | tee .tmp/p7c-assembly-test.log
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets 2>&1 | tee .tmp/p7c-build.log
```

**结果：** 两套 crate 测试全绿、0 warning；workspace 构建 0 warning。判据不是退出码：逐条读日志里的
`test result:` 行，确认默认套件没有 `assembly` 目标、装配套件有 `tests/assembly.rs` 的 3 条。

- [ ] **Step 2: J1(i) 复核与 J1(ii) 记录**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo tree -p continuum-arbiter --depth 1 --edges all --prefix none
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction every_crate_depends_only_on_its_allowed_set
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction 2>&1 | tee .tmp/p7c-dependency-direction.log
```

**结果：**
- `cargo tree` 只有 `continuum-arbiter v0.1.0 (...)` 一行，无子节点；
- `every_crate_depends_only_on_its_allowed_set` 绿；
- 整份 `dependency_direction` **预期红在 `p6_union_adds_only_its_own_rows` 的断言 8**（新增 subject
  `continuum-arbiter` 不在 `P6_NAMES`），**这不是 J1(ii) 绿灯**；在报告里抄下红的断言号与
  `p6_pre_allowed.txt` 未被修改的事实。J1(ii) 的最终绿等 P7D 完成合并门 rebaseline／退役。

- [ ] **Step 3: 零表／零迁移与 runtime 三处核对**

```bash
git diff -- crates/continuum-runtime/src/main.rs \
  crates/continuum-runtime/tests/migrations.rs \
  crates/continuum-runtime/tests/startup.rs \
  crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test migrations
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test startup
```

**结果：** `git diff` 为空；`migrations`／`startup` 全绿（现有 14 项计数不变）；`Cargo.toml` 的
`[dependencies]`／`[dev-dependencies]` 仍为空，没有 `continuum-persist`／`continuum-events`。

- [ ] **Step 4: 逐条核对设计 §2.4 的 15 条自定判定与 §8.1 的 J1–J17**

| 设计 §2.4 的行 | 本计划的用例 |
|---|---|
| 第 1 条（压力四档／最大值／默认阈值） | Task 2 的 `pressure_bands_use_the_max_of_non_none_readings`、`hysteresis` |
| 第 2 条（`Option<u8>`、`None` 忽略、全 `None` 保持、`>100` 钳位） | Task 2 的 `all_none_keeps_pressure_and_clears_the_class`、`pressure_bands...` 的 101 |
| 第 3 条（`WorkloadClass` 四档闭集、`INTERACTIVE` 不设变体） | Task 1 的 `the_class_set_is_exactly_four_and_round_trips`、`interactive_is_not_a_fifth_class` |
| 第 4 条（分类器在本机探测器、进程名不进公开类型） | Task 6 的 `no_raw_process_information_in_public_types`；归属待协调见「遗留」第 2 条 |
| 第 5 条（`workload_class` 可为 `None`） | Task 2 的 `all_none_keeps_pressure_and_clears_the_class`；Task 4 的 summary 断言 |
| 第 6 条（四档分类规则） | Task 3 的 `classification_follows_maintenance_user_foreground_default` |
| 第 7 条（4×4 动作阶梯） | Task 4 的 `every_cell_of_the_four_by_four_ladder_is_explicit` |
| 第 8 条（滞回默认值与升降方向） | Task 2 的 `pressure_rises_immediately_and_falls_only_below_resume_at_after_cooldown` |
| 第 9 条（相位转移图、`Throttled` 非必经） | Task 4 的 `suspend_only_reaches_checkpointing_and_success_reaches_suspended`；越步裁定见「遗留」第 1 条 |
| 第 10 条（失败结果类型） | Task 4 的 `checkpoint_failure_returns_to_throttled_and_never_suspends` |
| 第 11 条（超时／重试／状态时钟） | Task 5 的 `timeout`／`retry` 全部用例；`resume_resets_the_checkpoint_attempt_budget` |
| 第 12 条（四档优先级暴露机制） | Task 3 的装配测试＋Task 6 的 `priority_tier_...`／`default_surface_...` |
| 第 13 条（纯决策核＋本地探测器） | Task 1 的 `zero_deps`＋Task 6 的 `no_clock...`；探测器归属见「遗留」第 2 条 |
| 第 14 条（零持久化） | Task 7 的 `the_manifest_is_zero_table_and_gates_the_assembly_target` |
| 第 15 条（`action: Option`） | Task 4 的 16 格；Task 6 的 `decision_has_exactly_three_public_accessors_and_no_priority` |

**照片归位（J1–J17）**见「自审」第一节的 J1–J17 表；本步逐条勾对，不重述设计原文。任何一条没有真实用例的，
按设计 §8.4 记为“拍不出”，不得写成已覆盖。

- [ ] **Step 5: 变异复核**

每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑对应测试 → 读红位 → 还原后 `sha256sum` → 独立日志路径。
**逐轮在报告里附“变异前 sha256／还原后 sha256／红的位置／日志路径”**。按设计 §8.3 的预告表跑六条承重变异：

| 变异 | 该红 | 不该红 |
|---|---|---|
| 给 `Cargo.toml` 加 `continuum-budget = { path = "../continuum-budget" }` | Task 1 的 `the_crate_has_no_dependencies`、`every_crate_depends_only_on_its_allowed_set` | Task 2 的纯行为用例 |
| 把 `WorkloadClass::parse("INTERACTIVE")` 映射到 `Some(InteractiveGame)` | Task 1 的 `interactive_is_not_a_fifth_class` | Task 2／4 的行为用例 |
| `direct` 在 `Running` 收到 `Suspend` 时直接设 `Suspended` | Task 4 的 `suspend_only_reaches_checkpointing...`、`every_cell...`、`high_pressure...` | Task 2 的压力用例 |
| `checkpoint_failed` 停在 `Checkpointing` 或报 `Suspended` | Task 4 的 `checkpoint_failure_returns_to_throttled_and_never_suspends` | Task 2／Task 5 的重试用例（若未触发失败路径） |
| 删掉 `direct` 的截止检查 | Task 5 的 `direct_at_the_deadline_returns_to_throttled`、`late_success_is_rejected_after_the_deadline` | Task 4 的成功路径 |
| `max_checkpoint_attempts == 0` 分支返回 `None`／`Running` | Task 5 的 `zero_attempts_only_throttles` | Task 4 的 16 格（默认 max=3） |

**结果：** 六轮都红在该红的位置、还原后 `sha256sum` 与变异前一致、`git status` 干净。

- [ ] **Step 6: 收尾**

**结果：** `git diff --stat` 只包含本计划交付的文件；`.tmp/` 未入库；若复核发现缺口，修复提交回到拥有
该代码的 task，不在本 task 顺手改语义。**J1(ii) 仍标“待 P7D 合并门”，不写成全绿。**

---

## 自审

### 一、Spec coverage（设计节 → task）

| 设计节 | 内容 | 落在哪个 task |
|---|---|---|
| §1.1／§1.2 | 建什么／不建什么 | 全部 task 的 Files 与 Global Constraints |
| §2.1 | crate 名、零依赖边、共写文件、J1 两段 | Task 1（`Cargo.toml`＋`ALLOWED`）＋Task 8 Step 2 |
| §2.2 | 15 枚类型、7 条判定、不持有的判定 | Task 1–7；类型清单见「三、Type consistency」 |
| §2.3 | 层间位置、入度为零、不接第 4／5／6 层边 | Task 1（零依赖）＋Task 6（源码面） |
| §2.4 | 15 条自定判定 | Task 8 Step 4 的逐条表 |
| §2.5 | 切分 §四六条横切约束 | Task 1（ALLOWED／零迁移）＋Task 6（词汇表／源码面）＋Task 8（复核） |
| §2.6 | 同名异物（`Suspended`／`priority`／`workload_class`／`TaskSource`／`Decision`／`ResourceDirective`／`Budget`） | Task 6 的源码面；Task 3／4 的类型名与测试注释 |
| §3.1–§3.3 | §300／§13／§14 的规范依据 | Task 2（压力）／Task 1（负载类）／Task 3（优先级）／Task 4／5（挂起） |
| §3.4 | §7.1–§7.4；判据 3 只覆盖 J1＋J2 | Task 1／Task 4；「遗留」第 4 条 |
| §3.5 | A14／OPEN-012 的暴露面 | Task 3／6；「遗留」第 4 条 |
| §3.6 | budget／router／placement 边界 | Task 6 的 `no_resource_scheduler_budget_or_persistence_surface` |
| §3.7 | P7C 不是 Resource Scheduler | Task 6 同上；Global Constraints 末段 |
| §4.1 | 11 个源文件与测试位置 | File Structure；Task 1／2／3／4 的 Files |
| §4.2 | `ResourcePressure`／`ResourceObservation` | Task 1（类型）＋Task 2（observe） |
| §4.3 | `WorkloadClass` | Task 1 |
| §4.4 | `ResourceSummary` | Task 2 |
| §4.5 | `WorkloadDeclaration` | Task 3 |
| §4.6 | `PriorityTier`／分类 | Task 3 |
| §4.7 | `ResourceAction` | Task 4 |
| §4.8 | `SuspensionPhase`／`SuspensionState` | Task 4 |
| §4.9 | `SuspensionError` | Task 4 |
| §4.10 | `ResourceDecision` | Task 4 |
| §4.11 | `ArbiterPolicy`／`PolicyError`／`ResourceArbiter` | Task 2（new／observe／summary）＋Task 4／5（direct） |
| §4.12 | `AssemblyView`／`assembly` 特性 | Task 3 |
| §4.13 | 可见性、构造点、不变量 | Task 2（summary）＋Task 6（源码面） |
| §5.1 | 压力计算 | Task 2 |
| §5.2 | 滞回 | Task 2 |
| §5.3 | 四档分类 | Task 3 |
| §5.4 | 4×4 动作阶梯 | Task 4 |
| §5.5 | 相位转移、Resume 状态时钟 | Task 4 |
| §5.6 | 失败／超时／重试／`max=0` | Task 4（失败）＋Task 5（超时／重试） |
| §5.7 | 纯函数、确定性、无时钟 | Task 6（源码面）＋Task 7（determinism） |
| §6.1 | AI／后台结论面 | Task 2（summary）＋Task 6（两项访问器） |
| §6.2 | P7D 控制面 | Task 4（decision／state）＋Task 5（失败出口）；P7D 义务不在本块代码 |
| §6.3 | 装配方面 | Task 3 |
| §6.4 | P7B 端口不引入 P7C 类型 | **不在本块代码内**；Type consistency 明写无 `ResourceDirective`／`ResourceContext` |
| §6.5 | 暴露面强度与限制 | Task 3／6；「遗留」第 3 条 |
| §7.1／§7.2 | P7D／装配方的消费清单 | Task 4 的 Interfaces＋「遗留」第 2 条 |
| §7.3／§7.4／§7.5 | 不参与 product drive／budget／router；不碰 P7A／B／D 内部 | Global Constraints＋Task 6 |
| §8.1 | J1–J17 | 「二、照片归位」 |
| §8.2 | 照片的语料与读法 | Task 6 的 `tests/support/mod.rs`＋Task 6 Step 1 |
| §8.3 | 变异预告 | Task 8 Step 5（六条承重变异） |
| §8.4 | 拍不出照片的地方 | 「遗留/待协调」逐条列 |
| §8.5 | §7.4 判据 3 与自报判据的关系 | Task 8 Step 2／Step 4；Global Constraints |
| §9 | 明确不做 | Global Constraints 末段；Task 6 源码面 |
| §10 | 零持久化／不占迁移档 | Task 7 的 manifest；Task 8 Step 3 |
| §11 | 14 条设计问题 | 「遗留/待协调」列与本块直接相关的四条＋两条接缝 |
| §12 | 实测对账 | 本计划引用其结论，不重述 |

**照片归位（J1–J17）**：照片按设计 §5 的判定分节归位——J12→§5.4、J8→§5.5、J11→§5.6、J2→§5.4、
J7→§4.6／§6.5、J13→§4.10；其余按 §8.1 的 J 编号直接归位。

| 判据 | 照片（用例／命令） | task |
|---|---|---|
| **J1(i)** | `tests/zero_deps.rs` 两条＋`every_crate_depends_only_on_its_allowed_set` | Task 1 Step 5／Task 8 Step 2 |
| **J1(ii)** | 整份 `dependency_direction` 的最终树；**P7D 完成合并门前预期红在断言 8** | Task 8 Step 2（记录，不算绿灯） |
| **J2** | `high_pressure_limits_background_and_maintenance_without_any_model_path`（独立状态） | Task 4 |
| **J3** | `pressure_bands_use_the_max_of_non_none_readings`、`all_none_...`、`policy` 的非法策略、`pressure_text_is_exact_uppercase` | Task 1／Task 2 |
| **J4** | `pressure_rises_immediately_and_falls_only_below_resume_at_after_cooldown` | Task 2 |
| **J5** | `the_class_set_is_exactly_four_and_round_trips`、`interactive_is_not_a_fifth_class`、穷尽 match | Task 1 |
| **J6** | `summary_has_exactly_two_public_accessors`、`summary_source_has_no_priority_or_process_terms`、`serialization_display_and_default_are_absent...`、`summary_priority`／`summary_literal` 夹具 | Task 6 |
| **J7** | 装配测试三条＋`priority_tier_is_private_by_default...`、`priority_tier_is_confined...`、`priority`／`assembly_view` 夹具 | Task 3／Task 6 |
| **J8** | `suspend_only_reaches_checkpointing_and_success_reaches_suspended`、`resume_uses_the_suspension_state_clock_not_the_pressure_clock`、`resume_waits_for_the_state_cooldown...` | Task 4 |
| **J9** | `checkpoint_failure_returns_to_throttled_and_never_suspends`、`checkpoint_calls_outside_checkpointing_report_the_current_phase` | Task 4 |
| **J10** | `direct_at_the_deadline_returns_to_throttled`、`late_success_is_rejected_after_the_deadline`、`the_timer_hook_returns_to_throttled`、`timer_hook_outside_checkpointing...` | Task 5 |
| **J11** | `attempts_are_capped_and_run_resets_the_budget`、`resume_resets_the_checkpoint_attempt_budget`、`retry_waits_for_the_state_clock_cooldown`、`the_checkpointing_window_is_not_interrupted_while_retrying`、`zero_attempts_only_throttles` | Task 5 |
| **J12** | `every_cell_of_the_four_by_four_ladder_is_explicit`（16 格逐格） | Task 4 |
| **J13** | `decision_has_exactly_three_public_accessors_and_no_priority`、`decision_priority` 夹具 | Task 6 |
| **J14** | `no_raw_process_information_in_public_types`＋`format!("{:?}", …)` 断言 | Task 6 |
| **J15** | `the_manifest_is_zero_table_and_gates_the_assembly_target`、`default_test_build_does_not_compile_the_assembly_target`、Task 3 Step 3 | Task 3／Task 7 |
| **J16** | `no_resource_scheduler_budget_or_persistence_surface`＋manifest 的禁用依赖 | Task 6／Task 7 |
| **J17** | `no_clock_random_process_filesystem_network_environment_or_unsafe`＋`the_same_sequence_yields_equal_values`＋`action`／`phase` 的穷尽 match | Task 4／Task 6／Task 7 |

### 二、Step scan

- 每个 Step 是**一个动作 + 一个可核对结果**：写用例／跑失败形态／建骨架或实现／跑测试／提交；
  Task 3 的 `[[test]]` 门控单独一步；Task 5 的超时与重试合为一个行为补全 task（同一状态机出口）；
  Task 8 只有验证与复核，不新增语义。
- **“运行，确认失败”逐 task 标注量到什么**：Task 1 是 package 未找到（不是编译失败）；Task 2／3／4 是
  `E0432`／`E0599`（不算红）；Task 5 主体是运行期红（`resume_resets...` 与
  `the_checkpointing_window...` 两条是 Task 5 实现的守卫，先绿；见 Task 5 Step 2）；Task 6／7
  首次即绿（或夹具首轮红），其红由 Task 8 的
  变异复核取；Task 8 无“确认失败”步。
- **测试命令精确到 `--test` 目标**：`zero_deps`／`workload`／`pressure`／`policy`／`hysteresis`／
  `assembly`／`action`／`ladder`／`without_model`／`suspension`／`timeout`／`retry`／`source_surface`／
  `compile_fail`／`determinism`／`manifest`。
- **没有一步同时改两个关注点**：Task 1 只做骨架＋基础枚举＋依赖门；Task 2 只做策略／压力／summary；
  Task 3 只做优先级与装配面；Task 4 只做动作／相位／阶梯；Task 5 只做超时／重试；Task 6 只做源码面与
  夹具；Task 7 只做确定性与清单；Task 8 只做全量门与变异。
- **提交只用显式路径**；`Cargo.lock` 只在 Task 1 随新 crate 提交。

### 三、Type consistency

- `ResourcePressure`：四变体；`as_str` 返回 `"LOW"`／`"MODERATE"`／`"HIGH"`／`"CRITICAL"`；`parse` 精确大写。
- `ResourceObservation`：`cpu`／`ram`／`gpu`／`vram`／`disk_io`／`network: Option<u8>`＋
  `workload_class: Option<WorkloadClass>`；派生 `Debug, Clone, Copy, PartialEq, Eq, Default`。
- `WorkloadClass`：四变体；`ALL: [WorkloadClass; 4]`；`as_str`／`parse`；无 `Default`。
- `ResourceSummary`：字段私有；`resource_pressure()`／`workload_class()`；`pub(crate) from_parts`；
  派生 `Debug, Clone, Copy, PartialEq, Eq`；无 `Default`／`Serialize`／`Deserialize`／`Display`。
- `WorkloadDeclaration`：三个 `pub bool`；派生 `Debug, Clone, Copy, PartialEq, Eq`；无 `Default`。
- `PriorityTier`：四变体；私有模块；`#[cfg(feature = "assembly")] as_str`／`rank`；**不派生 `Ord`**。
- `AssemblyView`：字段私有；`summary()`／`priority()`；只由 `assembly_view` 构造。
- `ResourceAction`：四变体；`as_str`／`parse`；无 `Kill`。
- `SuspensionPhase`：四相位；`as_str`／`parse`；**没有 `Resume` 相位**。
- `SuspensionState`：字段私有；`new`／`phase`／`checkpoint_deadline_millis`／`checkpoint_succeeded`／
  `checkpoint_failed`／`checkpoint_timed_out`；`checkpoint_failed`／`checkpoint_timed_out` 返回
  `SuspensionError`，不是 `Result`。
- `SuspensionError`：三臂；手写 `Display`／`Error`。
- `ResourceDecision`：字段私有；`summary()`／`action()`／`phase()`；`action: Option<ResourceAction>`。
- `ArbiterPolicy`：七个公开字段与默认值；`impl Default`。
- `PolicyError`：两臂；手写 `Display`／`Error`。
- `ResourceArbiter`：`new`／`observe`／`summary`／`direct`／`assembly_view`（后者仅特性面）；
  `direct(&self, declaration, &mut state, now_millis) -> Result<ResourceDecision, SuspensionError>`。
- **没有自造** `NodeState`／`IntentStatus`／`Intent`／`Decision`／`TaskSource`／`AuthorizedTask`／
  `ResourceDirective`／`ResourceContext`／`RoutingRequest`／`PlacementRequest`／`Budget`。

### 四、Review Focus 归位

| Review Focus | 归属 task | 具体用例 |
|---|---|---|
| 1. 模型不可用时无法限流 | Task 1／Task 4 | `the_crate_has_no_dependencies`、`every_crate_depends_only_on_its_allowed_set`、`high_pressure_limits_background_and_maintenance_without_any_model_path` |
| 2. `Checkpointing` 永久停留 | Task 5 | `direct_at_the_deadline_returns_to_throttled`、`late_success_is_rejected_after_the_deadline`、`the_timer_hook_returns_to_throttled` |
| 3. 四档优先级泄漏 | Task 3／Task 6 | `classification_follows_maintenance_user_foreground_default`、`priority_tier_is_private_by_default_and_only_assembly_re_exports_it`、`summary_has_exactly_two_public_accessors`、`decision_has_exactly_three_public_accessors_and_no_priority`、`default_surface_rejects_hidden_priority_and_private_literals` |
| 4. `SuspensionState` 复用导致第二次 `direct` 错位 | Task 4 | `per_task_state_is_required_for_independent_decisions` |
| 5. `Resume` 后尝试计数未重置 | Task 5 | `resume_resets_the_checkpoint_attempt_budget` |

### 五、比例

- 设计 1535 行；本计划 **2160 行**（`wc -l` 实测）。任务 **8 个**。
- 比例约 1.4×设计，与 P6A（1828／1283）／P7B（1752／1451）同量级；文档里没有实现函数体，
  只有签名、测试断言、转移表、4×4 阶梯与源码面规则。测试量大于生产量是刻意的——本块的判据几乎全是
  “形状＋状态边界＋否定面＋源码面”。
- J1–J17 每条至少有一张照片；**J1(ii) 明确标“待 P7D 合并门”，不是全绿**；J6／J7／J13 的
  `impl` 块扫描覆盖全量 `src/`，不按文件白名单偷懒；J12 的 16 格逐格列出；J11 的 `max=0` 与
  `Resume` 重置各有一条独立用例。
- 没有把设计 §11 的 14 条全部搬成 task：与本块实现直接相关的第 1／4／6／14 条进「遗留」，
  第 7／8／9／10／11／12／13 条留在设计里（重述就是第二份转录）。

---

## 遗留/待协调

**凡设计未给判据的，下标“规范未给判据”；凡本计划自己定了的，下标“本计划自定”。**

1. **§14 越步裁定（开工前具名前置）**：设计 §11 第 6 条与 §8.4 第 9 条要求先确认 §14 的
   `Running → Throttled → Checkpointing → Suspended → Resume` 是否**强制经过 `Throttled`**。本计划按
   设计当前读法（`Suspend` 可从 `Running` 直接进 `Checkpointing`）写 Task 4／5 与 J8／J12；若规范维护者
   读作必经，Task 4 的阶梯与 Task 5 的转移必须先改（先 `Throttle` 再 `Suspend`），J8／J12 整组重写。
   **收件人：规范维护者＋协调者。未确认前不得把本计划当“设计已裁定”。**

2. **`workload_class` 判定器归属**：设计 §11 第 4 条把分类器放在本机 Resource Arbiter 组件的本地探测器
   （装配方落地），`continuum-arbiter` 只收已分类的 `Option<WorkloadClass>`。若协调者读作“判定器也必须在
   `continuum-arbiter` 内”，本块要么加 workspace 依赖（破坏 `ALLOWED == []`），要么在 `src/` 读 OS
   （破坏 §5.7 无 I/O 与 J17），两条都与设计边界相抵。**收件人：规范维护者＋协调者＋装配方。**

3. **P6D 合并门与 J1(ii)**：`continuum-arbiter` 进 `ALLOWED` 后，`p6_union_adds_only_its_own_rows` 的
   断言 8（新增 subject 不在 `P6_NAMES`）会红；P7C 只新增 subject，先撞的是断言 8（切分 §四第 1 条另
   指出 P7D 的 runtime 行增量还会撞断言 5）。本块**不碰** `p6_pre_allowed.txt`、`P6_EXPECTED_ROWS`、
   解析器与断言 1 的 `27` 字面量。J1(ii) 的最终绿前置是 P7D 完成合并门的 rebaseline／退役。
   **收件人：P7D＋协调者。** 本计划 Task 8 只记录该红，不把它算作 P7C 的完成条件。

4. **A14／OPEN-012 的规范级强制归 P9**：P7C 只交可观察的公开面（`ResourceSummary` 两项、
   `ResourceDecision` 无优先级、`PriorityTier` 默认不导出、原始信息不入类型）；`assembly` 特性不是安全
   边界，同一构建图里启用特性的消费方仍可命名 `PriorityTier`。**P7C 不声称 SHOULD 已变成 MUST**。
   **收件人：规范维护者＋P9**（主树 `docs/superpowers/p8-p10-scope.md:63`、`:68`）。

5. **dev-dependencies 口径（本计划自定，按设计执行）**：任务书写“dev 只有 `trybuild`”，但设计 §2.1
   逐字写“测试也只使用 `std`，**没有 dev-dependencies**”，且 J1(a) 的 `cargo tree --edges all` 要求
   只有本 crate 一行；加 `trybuild` 会引入外部子节点并破坏 J1(a)。本计划因此**零 dev 依赖**，编译失败
   夹具用 `std::process::Command` 调 `cargo check`（设计 §8.2）。若协调者坚持 `trybuild`，必须先改设计
   §2.1 与 J1(a)，不能在实现期偷偷加。**收件人：协调者。**

6. **预算与资源仲裁的关系**：设计 §11 第 13 条明确 `continuum-budget` 是记账，不是资源仲裁；P7C 不依赖、
   不读预算。若将来要让预算读数影响压力／动作，必须另立一条边并升级裁定，**不能把 `continuum-budget`
   塞进 `ALLOWED == []`**。**收件人：P4／budget 持有方＋协调者。**

7. **P7B 的 `BackgroundSuitability` 投影**：设计 §11 第 12 条把投影取值域／语义留给 P7B＋装配方；
   P7C 只给 `resource_pressure`＋`workload_class` 两个访问器，不定义投影。本块不实现、不写第二套词汇。
   **收件人：P7B＋装配方＋规范维护者。**

**不列入本计划的**：`TaskSource`→`WorkloadDeclaration` 的映射（设计 §7.1 明确是 P7D 的设计决定）、
`throttle` 的量化语义（P7D）、checkpoint 的内容／存储／重启恢复（P7D）、OS 探测器的频率与失败语义
（装配方）。这些是消费方义务，P7C 只给接口与状态机。
