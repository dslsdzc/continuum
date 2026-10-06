# P3 子项目 D（Model Registry 与 Router）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建一个新 crate `continuum-model-registry`（资源层），落《工程》§4.1 模型侧的五项——ModelProfile 与 SkillVector 存储（§247 §248）、Model Registry 生命周期（§249）、Router 与候选排序（§250 §84）、升级与降级的数据形状（§251 §85 §86）、成本输入的接口投影（ENG-005）。

**Architecture:** 一个 crate，内部按职责分文件（画像 / 生命周期 / 持久化 / 路由 / 预算 / 阶梯 / 错误）。**本子项目的交付物是一份判断，不是一次执行**：`rank` 是同步纯函数，收 `&[RoutableModel]` 与 `&RoutingRequest`，不接 `Tx`、不持有 `CallId`、不消费 `ModelStream`。`ModelProvider::invoke` / `stream` 的调用方是**子项目 G（模型调用路径）**，本 crate 里那两个名字一次都不出现（设计 §1.2）。

两条结构性的保证由类型承载，而不是靠调用方自觉：**可路由闸门**（未画像 / 被隔离 / 被停用 / 已漂移的模型在 `rank` 的入参里无处安放，§4.2）与**画像必须来自库**（`ModelProfile` 在 crate 外没有构造入口，§2.1）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`continuum-persist`（`Tx`、`Migration`、`Value`、`value::kind_name`、`PersistError`）；`continuum-core`（`ModelId`、`ToolId`、`ProviderHealth`）；`continuum-capability`（`Cost`、`Latency` 两个**类型**）；`serde_json`（三个列表列的容器编码）；`thiserror`；`trybuild`（类型层不可构造性）。

**设计依据：** `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md`（**唯一事实来源**）。本计划不改设计、不改代码、不改规范。

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向禁止反向。** 本计划新增的边只有三条：
  `continuum-model-registry → continuum-core`（`ModelId` / `ToolId` / `ProviderHealth`）、
  `→ continuum-capability`（`Cost` / `Latency`）、`→ continuum-persist`（`Tx` / `Migration` / `Value` / `kind_name` / `PersistError`）；
  以及装配处一条 `continuum-runtime → continuum-model-registry`。
  `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 与各 `Cargo.toml` 必须**精确一致**（断言是逐对 `assert_eq!`，且覆盖 dev 边；`workspace_crates()` 要求**每个** workspace 成员都在 `ALLOWED` 里，故新 crate 的空条目也要登记）。
- **本计划登记自己那几条 `ALLOWED` 与 `Cargo.toml` 边**（协调者裁决 §六.7：不设集中登记 task）。边**由需要它的那个 task 增量加**，不在 Task 1 一次声明齐——叶子 crate 的条目记的是**实际依赖**，一次声明齐会让中间若干 task 的条目在说谎（P3A 计划第 116 行）。
- **本计划不改 `continuum-capability` 的源码，也不改它的依赖边。** 本层只**消费**它的 `Cost` / `Latency` 两个类型（设计 §2.5）；**不取能力凭据**——`Capability` / `AuthorizedTool` / `authorize` 在本 crate 里零出现（设计 §8.4：`Router ← …` 那条边取的是两个画像容器，不是 Capability Token）。
- `continuum-core` 不含 I/O。数据库连接对象在 `continuum-persist` 内私有，本 crate 只能经 `Tx` 访问。
- **枚举列的落库编码一律小写、多词以 `_` 连接，经显式辅助函数读写，不依赖 serde、不用 `Debug`。** 编码挂在其类型上（`as_str` / `parse` 与类型同址），不在 `persist.rs` 里另建表。本 crate 的 `SkillDimension` 与 `LifecycleState` 各要一对。
- **迁移编号须在 task 里现场核实该库的空号**（不假定）：协调者已定号段（A 50、B 60、C 70、D 80、E 90），本设计取 80、**只取一个**，但**取用前最后核一次**；「未占用」的判据是**按库**说的，不是按全仓说的（设计 §3.3）。**不预留 81**——三张表全在迁移 80 里，本子项目没有第二个建表点（设计 §3.3 已把第一版那句「预留 81」删掉并留了来历）。
- **迁移的注册由本计划自己完成，注册在 `main.rs`**（协调者裁决 §六.4「谁的表谁注册」，与 P3A 的 Task 4 同一做法）。**不许把它记到「用它的那个 task」（＝ 子项目 G，本轮不存在）名下。**
- `continuum-runtime` 不直接对枚举列写 SQL 字面量。
- 代码注释、错误信息、测试断言信息用**中文**。标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增或变更 crate 依赖时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不调 `ModelProvider::invoke` / `stream`，不定义 `usage()` 的语义，不消费 `ModelStream`**——那是子项目 G（设计 §1.2）。
  - **不建路由探索**（§27 被 OPEN-014 与 OPEN-008 双阻断，§7.2）。
  - **不建升级与降级的触发**（失败检测、预算前提检查、降级调度都不在本层，§7.1）。
  - **不建预算记账**（预扣 / 结算 / 父子分配 / Budget Validator 归语义层，§6.2）。
  - **不建 `overall_score`，也不建任何把九维折成一个数的函数**（§2.3）。
  - **不发明 `effect_class` / `trust` / 工具侧 `cost` / `latency` 的处置**——对 `effect_class` 与 `trust` 的两次退件，在实现里就是「不实现」：**本计划不为它们建 task**（见 `## 遗留`）。
  - **不收紧 `ExecutionProfile.cost_budget`**（设计 §6.3 明写不做，理由两条）。

### 本计划的前置与交接（`docs/superpowers/p3bcdf-followups.md` §七 每份计划都要有的那一段）

- **依赖已交付（不必再造）**：P3A 的 `continuum-capability`（本层取 `Cost` / `Latency` 两个单位结构体）、
  `continuum-core` 的 `ModelId`（`crates/continuum-core/src/model.rs:9`）、`ToolId`（`src/tool.rs:7`）、
  `ProviderHealth`（`src/model.rs:83`）、`continuum-persist` 的 `Tx` / `Migration` / `Value` / `kind_name` / `PersistError`。
  **不依赖 B、C、F 的任何产物**；执行序 B → **D** → C → F，D 与 B 无编译期关系。
- **交给子项目 G（模型调用路径）**：`RankedExecutionCandidates`、`ExecutionCandidate`、`RoutingReason`、
  `RoutableModel`、`RoutingRequest`、`TaskSkillRequirement`、`FamilyPreference`、`BudgetView`，
  以及「取 `ProviderHealth` 快照」这一步（设计 §11 第 20 条；接缝裁决的权威转录是
  `docs/superpowers/p3bcdf-followups.md` §七——**不引那份 gitignore 的接缝分析 scratch**）。
- **交给协调者**：`effect_class` 两轴之问的退件、`trust` 退件、`ToolProfile` 上工具侧 `cost` / `latency`
  的 `Some` 无人认领（设计 §11 第 15 / 22 / 23 条，均见 `## 遗留`）。
- **必须自己声明的未决**：迁移号 80 的现场复核；以及一处**计划自定的取名**——落库版的迁移函数
  取名 `transition_in_tx`（与内存版 `lifecycle::transition` 在 crate 根同名会冲突，见 `## 遗留`）。

## 三条已付过代价的纪律

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；正向的「变红」跑全量是加分。**变异的三条失效形态都要防**：(a) 锚点不唯一 → 变异没落到实现体却报 GREEN；(b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，处理是**换真变异体而非补用例**；(c) **变异导致编译失败**——那不是「变红」。每次变异用**独立日志路径**，读前确认是本轮写的。判「编译失败」要用 `could not compile` 或 `error[E….`（cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
2. **凡注释写绝对措辞，必须有对应用例**；写不出的就改成名副其实的说法，或**明写它为什么没有照片**。**枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条分支各要照片；手写分支能各自漂移，故要逐项钉。判据：用例里的值字面量是**手工写的**还是**被测函数返回的**——前者钉格式，后者钉路径。
3. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 Err」。本计划的所有错误枚举（`ProfileError` / `LifecycleError` / `RequirementError` / `RoutingError`）逐变体至少一条用例。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。

**变异日志是证据，必须活到复审结束**：**实现者保留 `.tmp/`，由协调者在复审结束后清理**。报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。危险的是**变异**与任何别的东西并行；两份跑在同一份终稿字节上的验证并行是无害的。

**变异口径分层**：变异**条数**按「有多少条**互不相同**的守卫」定，不按分支数定，且分两档、**不许混成一句「通过」**：
- **承重守卫 → 全量套件**：两侧对钉的守卫、**fail-open 的那一侧**、失败路径**判别哪一种 `Err`**、以及**跨 crate 才可见**的效果（迁移编号与计数、`ALLOWED` 与实际依赖一致）。
- **其余分支 → 受影响 crate 的包级套件**，报告里须**标明证据强度较低**并列出「这一条可能漏掉的跨 crate 观察点」。

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；凡与既有 crate 交互的签名（`Tx::query` / `Tx::execute` / `Value` / `Migration::new`），实现前须先读该 crate 的源码确认，不符时以源码为准并回报。

**本计划特有的两处「签名即判据」，照抄前须对源**：

- `Tx::execute(&self, sql: &str, params: &[Value])`、`Tx::query(...) -> Result<Vec<Vec<Value>>, PersistError>`、`Migration::new(version, name, sql)`（`crates/continuum-persist/src/{tx.rs,db.rs}`）——`Migration.name` 与 `sql` 是 `&'static str`。
- `ModelProfile` 的构造是 **crate 内**的（`pub(crate) fn try_new`）。故**集成测试（`tests/`）构造不出画像**，只能经 `load_profile` 从库拿——这决定了本计划的测试夹具形态（见 Task 3、Task 11），不是实现细节。

**`tests/compile_fail/*.stderr` 的内容与错误码一律取自实跑，不许凭记忆写；而且逐份各不相同，不许拿一个值套三处。**
本项目已为此付过一次代价：Task 3 的 brief 给三份样例共用一个错误码 `E0603`，**实测 rustc 1.95 下三份各不相同**
（本节三份的实跑值：字面量构造 `E0451`、方法不存在 `E0599`、私有关联函数 `E0624`）。
**错误码是记忆最容易失真的一类事实**——它与本节开头那条「手写代码块错误率高」同源，只是更隐蔽：
正文措辞可以被复核，凭印象敲下的错误码看起来同样像事实。故本计划给出的任何错误码都只是**预期值**，
落地时以 `trybuild` 实际产出的 `.stderr` 为准。

---

# 文件结构

```
crates/continuum-model-registry/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/profile.rs      Ratio、SkillScore、SkillDimension、SkillObservation、SkillVector、ModelProfile
  src/lifecycle.rs    LifecycleState、RoutableState、RoutableModel、迁移表
  src/persist.rs      三张表的迁移与行级读写（与表定义同址）
  src/router.rs       RoutingRequest、TaskSkillRequirement、FamilyPreference、
                      RankedExecutionCandidates、ExecutionCandidate、RoutingReason、
                      CandidateScore、RankingPolicy、BaselineRankingPolicy、rank
  src/budget.rs       BudgetView（§333 的只读投影）
  src/escalation.rs   EscalationStep、EscalationLadder、next_step
  src/error.rs        ProfileError、LifecycleError、RequirementError、RoutingError
  tests/profile.rs       取值类型、九维向量、画像（crate 外能观察到的部分）
  tests/lifecycle.rs     十态、迁移表、闸门
  tests/persist.rs       三张表的落库与读写
  tests/router.rs        请求面、输出面、rank、基线策略
  tests/budget.rs        预算视图
  tests/escalation.rs    阶梯五档
  tests/type_level.rs    trybuild 驱动
  tests/compile_fail/*.rs   不可构造性样例（各配同名 .stderr）
```

**与设计 §8.1 一致**：本计划的模块划分与设计 §8.1 的清单逐项相同，共七个模块
（含 `src/escalation.rs`，设计 §8.1 已列入）。测试文件的划分亦同。

**既有的、本计划要改的文件**

```
Cargo.toml（workspace）                                   members 加 continuum-model-registry
crates/continuum-runtime/Cargo.toml                       新依赖边（装配迁移）
crates/continuum-runtime/src/main.rs                      runtime_migrations() 注册 p3d_model_migrations()
crates/continuum-runtime/tests/dependency_direction.rs    ALLOWED：新 crate 条目 + runtime 条目
crates/continuum-runtime/tests/migrations.rs              expected_migrations() 同步
crates/continuum-runtime/tests/startup.rs                 三处迁移计数断言与行内注释
```

**本计划不碰的文件**：`continuum-capability` 的任何文件、`continuum-core` 的任何文件、
`continuum-provider` 的任何文件（本 crate 对它的引用为零）、`docs/02-工程.md`。

---

### Task 1: `continuum-model-registry` 骨架与两个取值类型

**Files:**
- Create: `crates/continuum-model-registry/Cargo.toml`
- Create: `crates/continuum-model-registry/src/{lib.rs,profile.rs,error.rs}`
- Create: `crates/continuum-model-registry/tests/profile.rs`
- Modify: `Cargo.toml`（members）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 登记）

**Interfaces:**
- Produces: `continuum_model_registry::{Ratio, SkillScore, ProfileError}`

- [ ] **Step 1: 建 crate 骨架并登记**

`Cargo.toml` 的依赖：**只声明本 task 用得到的**——`thiserror`（`ProfileError` 上派生 `Error`，去掉即 `E0433`）。
`continuum-core` / `continuum-capability` / `continuum-persist` / `serde_json` 与 dev-dep `tempfile` / `trybuild`
**由需要它们的 task 增量加**（`ALLOWED` 对叶子 crate 记的是**实际依赖**，一次声明齐会让条目在中间若干 task 里说谎）。

`ALLOWED` 加（**只列本 task 实际有的边**：本 task 对内部 crate 的引用为零，故是空条目；
它不能省——`every_crate_depends_only_on_its_allowed_set` 要求每个 workspace 成员都在表里）：

```rust
    // P3 资源层（子项目 D）：模型画像、Model Registry 与 Router。本子项目自 Task 1 起
    // 只依赖 core（ModelId / ToolId / ProviderHealth）、capability（Cost / Latency 两个
    // **类型**，不取能力凭据）、persist（三张表经 Tx）。三条边由需要它们的 task 增量加。
    ("continuum-model-registry", &[]),
```

- [ ] **Step 2: 写用例**

`tests/profile.rs`（本 task 只放两个取值类型的用例）：

- `ratio_round_trips_through_its_text_encoding`：对 `0.0` / `0.5` / `0.94` / `1.0` / 极小与极大的**有限**值，
  断言 `Ratio::parse(&r.as_str()) == Some(r)`。**红的条件**：`as_str` 用**截断格式**（如 `{:.2}`）时变红
  ——**但那一半承重的是「极小 / 极大有限值」两档**：`{:.2}` 对 `0.0` / `0.5` / `0.94` / `1.0` 恰好仍能往返
  （`"0.94"` → `0.94`），故**若夹具把这两档换成普通值，这个变异就不红了**。**实现者实跑时按此核一遍**。
  **`Debug` 之所以不变红，要写进用例注释**：Rust 的 `f64` 的 `Debug` 与 `Display` 是**同一套最短精确往返算法**
  （实现者用探针实测：各取值丢精度 0 格；把 `as_str` 换成 `{:?}` 跑全量 `exit=0`、不变红）。
  这**不是等价变异体**——两版输出确实不同（`1e300`：`Display` 三百多个字符 vs `Debug` 的 `1e300`），
  只是**用例没有观察长度**；而设计 §2.4 只声称**往返**、不声称**最短长度**，故**不补用例**是对的。
  这是设计 §2.4「浮点存文本会丢精度」那句最容易被想当然的地方，故被钉住的是往返而非长度。
- `ratio_rejects_non_finite_and_out_of_range`：**逐格三格**——`NaN` → `Err(ProfileError::NotFinite)`；
  `1.5` 与 `-0.1` → `Err(ProfileError::OutOfRange { value })`（`value` 是给的那个数）。
  **协调者已裁：`NaN` 归 `NotFinite`，`OutOfRange` 只收「有限但落在 [0,1] 之外」**——
  判据是**跨类型一致性**：同一个 `NaN` 在 `Ratio` 上判 `OutOfRange`、在 `SkillScore` 上判 `NotFinite` 说不通，
  且 NaN 不是「落在某区间之外」的点；这样两枚变体的文档各自成立。
  红的条件：去掉 `is_finite()` 检查（`NaN` 那格红）、或把上界写成 `< 1.0`
  （`1.0` 是合法值，`ratio_round_trips…` 与它**两侧对钉**）。
- `skill_score_round_trips_through_its_text_encoding`：对 `9.2`（**唯一出处是 `docs/spec/02-positioning.md:773`，
  §83 Model Failure Modes** 的「不是只有：`coding = 9.2` 还应该有……」；设计 §2.4 把它记成
  `docs/spec/01-concepts.md:1068-1104`——那里是 §23 的技能树、全文无 `9.2`，该处已由协调者派回设计作者订正）、
  `0.0`、`-3.5`、极大有限值逐个断言往返。**同一对函数，另一侧的守卫**。
- `skill_score_rejects_nan_and_infinities`：`NaN` / `f64::INFINITY` / `f64::NEG_INFINITY` 各断言
  `Err(ProfileError::NotFinite)`（**不是有限实数**这一枚）。

- [ ] **Step 3: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test profile
```

- [ ] **Step 4: 实现**

```rust
/// §84 / §247 / §24 的 [0,1] 实数（confidence、compatibility）。构造期拒 NaN / ±∞ 与越界。
pub struct Ratio(f64);

impl Ratio {
    /// 非有限（NaN / ±∞）→ `NotFinite`；**有限**但不在 `0.0..=1.0` → `OutOfRange { value }`。
    /// 两者分开：NaN 不是「落在某区间之外」的点（协调者裁决，见 Step 2 的用例）。
    pub fn try_new(v: f64) -> Result<Self, ProfileError>;
    pub fn get(&self) -> f64;
    /// 十进制串。契约是 `parse(as_str(x)) == Some(x)`——只声称**往返**，不声称最短长度
    /// （设计 §2.4：Rust 的 `Display` 对 `f64` 保证可精确往返，`1e300` 会打出三百多个字符）。
    pub fn as_str(&self) -> String;
    /// `as_str` 的严格逆：非数值、非有限、越界一律 `None`，**不取默认值**。
    pub fn parse(s: &str) -> Option<Self>;
}

/// §23 的能力评分。**取值域与单位规范未定义**（§23 的示例是 9.2，§24 未给范围与方向）。
/// 本类型只保证一件事——**可比较**（全序）：本设计只使用它的序，从不使用它的量，
/// 故任何跨维度的求和、加权、归一化一处都不做（设计 §2.4）。
pub struct SkillScore(f64);
```

**为什么 `Ratio` 要拒 NaN**（实现时写在类型的文档注释里，不是可选说明）：NaN 与任何值的比较都是 false，
`sort_by` 在含 NaN 的列表上不是全序——排序结果随实现细节漂移，而 §84 的输出要被比对与记录。

`ProfileError` **与它的变体名逐字取自设计 §2.4**（`NotFinite` / `OutOfRange { value: f64 }` /
`BadTimeRange { start: i64, end: i64 }`；设计第一版只写「各返回具体 `Err`」而没给类型名，
这一处是计划作者报出后由设计补齐的）。本 task 定义前两枚——第三枚 `BadTimeRange` 的产生方是
`SkillObservation` 的构造，在 Task 2 落地（**没有产生方的变体不先铺开**）。
`ProfileError` **不进 `RoutingError`**：`rank` 收到的是构造好的值，构造失败在构造期就被拒（设计 §2.4、§5.4）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry Cargo.toml crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(model-registry): crate 骨架与 Ratio / SkillScore"
```

---

### Task 2: 九维、观测与「缺席不是 0」

**Files:**
- Modify: `crates/continuum-model-registry/src/profile.rs`
- Modify: `crates/continuum-model-registry/{src/error.rs,src/lib.rs}`（`ProfileError::BadTimeRange` 与导出）
- Modify: `crates/continuum-model-registry/tests/profile.rs`

**Interfaces:**
- Consumes: Task 1 的 `Ratio` / `SkillScore` / `ProfileError`
- Produces: `continuum_model_registry::{SkillDimension, SkillObservation, SkillVector, current_observation}`
  与 `ProfileError::BadTimeRange { start, end }`

- [ ] **Step 1: 写用例**

- `every_dimension_is_absent_before_any_observation`：九维**逐项**断言 `vector.get(d).is_none()`——不抽代表。
  红的条件：把 `Option` 换成默认值（如 `SkillScore(0.0)`）即红。这条钉的是 §2.2 的「**缺席不是 0**」：
  未画像的模型不该看起来「所有维度都很差」，那在 §23 的语义下与「很强」一样是假的。
- `one_observation_leaves_the_other_eight_absent`：写入一维后，其余八维**逐项**仍为 `None`。
  红的条件：数组下标算错（把 `dimension as usize` 当成 1-based）时只有这条能抓。
- `the_current_observation_is_the_highest_version_not_the_latest_time_range`：同一维度两条观测，
  `version` 大的那条 `time_range` **更早**；断言当前值取 `version` 大的那条。
  红的条件：改按 `time_range_end` 取最大即红。设计 §2.2 明写这个选择（`version` 是 §24 列出的字段里
  唯一由产生方显式递增的量），记在 §11 第 17 条。
- `an_empty_series_has_no_current_observation`：空切片 → `None`（不是 panic、不是默认值）。
- `skill_dimension_encoding_is_lowercase_with_underscores`：九个 `as_str` **逐项**断言字面量
  （`reasoning` / `coding` / `vision` / `planning` / `tool_use` / `constraint_following` / `verification` /
  `spatial` / `media`），并在同一用例里反向 `parse` 回各自的变体。
  红的条件：多词项写成 `toolUse` 或 `tool-use` 即红——**手册写字面量**，故钉的是格式。
- `an_unknown_dimension_name_is_rejected`：`SkillDimension::parse("visual")` → `None`。
- `an_observation_whose_time_range_is_reversed_is_rejected`：`time_range = (200, 100)` →
  `Err(ProfileError::BadTimeRange { start: 200, end: 100 })`，**两个端点值都断言**（设计 §2.4：
  「外加时间窗反序一条」）。**两侧对钉**：`(100, 200)` 与**退化区间** `(100, 100)` 各返回 `Ok`
  ——`time_range` 是**闭区间**，`start == end` 是自洽的。红的条件：去掉 `end < start` 判定即红；
  把判定写成 `end <= start` 则退化区间那一条红。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test profile
```

- [ ] **Step 3: 实现**

```rust
/// §248 的九个维度。半封闭枚举：非法维度名不可表达（落库小写、多词 `_` 连接）。
pub enum SkillDimension { Reasoning, Coding, Vision, Planning, ToolUse,
                          ConstraintFollowing, Verification, Spatial, Media }

/// §24 的一次观测：score / confidence / sample_count / version / time_range。
/// **构造期拒不自洽的时间窗**（`end < start`）——错误收在 `ProfileError` 里（设计 §2.4）。
pub struct SkillObservation { /* 五个字段私有，访问器按消费方需要增补 */ }

impl SkillObservation {
    pub fn try_new(score: SkillScore, confidence: Ratio, sample_count: u64, version: u32,
                   time_range: (i64, i64)) -> Result<Self, ProfileError>;   // end < start → BadTimeRange
}

/// §248 的向量：九维各自**当前**的一次观测。`None` = 该维度尚无观测——**缺席不是 0**。
pub struct SkillVector { dimensions: [Option<SkillObservation>; 9] }

impl SkillVector {
    /// 由「维度 → 该维当前的那次观测」构造。同一维度给两次即覆盖前一次，**不合并**。
    pub fn from_current(current: Vec<(SkillDimension, SkillObservation)>) -> Self;
    pub fn get(&self, dimension: SkillDimension) -> Option<&SkillObservation>;
}

/// 「当前值」的判据：`version` 最大的那次观测（**不是** `time_range` 最晚的那次）。
pub fn current_observation(series: &[SkillObservation]) -> Option<&SkillObservation>;
```

**分层（§23 的子技能树）不建**：§248 说的是 MAY，而规范没有给子维度的词表或聚合规则，建它就要发明一套（设计 §2.2）。这句话写进 `SkillVector` 的文档注释。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): §248 九维与 §24 的时间序列观测"
```

---

### Task 3: `ModelProfile`（§247 十二字段）与两处不可表达性

**Files:**
- Modify: `crates/continuum-model-registry/src/profile.rs`
- Modify: `crates/continuum-model-registry/{Cargo.toml,src/lib.rs}`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加 `continuum-capability`、`continuum-core`）
- Create: `crates/continuum-model-registry/tests/type_level.rs`
- Create: `crates/continuum-model-registry/tests/compile_fail/{model_profile_cannot_be_built,
  model_profile_has_no_constructor,overall_score_cannot_be_read}.rs`（各配同名 `.stderr`）

**Interfaces:**
- Consumes: Task 1 的 `Ratio`、Task 2 的 `SkillVector`；**既有的** `continuum_core::model::ModelId`（§315）、
  `continuum_core::tool::ToolId`（§252/§316）、`continuum_capability::{Cost, Latency}`（**类型**，不取凭据）
- Produces: `continuum_model_registry::ModelProfile`

- [ ] **Step 1: 写用例**

`ModelProfile` 的字段级用例写在 `src/profile.rs` 的 `#[cfg(test)] mod tests`（**crate 内**，能调 `pub(crate) try_new`）：
本仓既有此形态（`crates/continuum-capability/src/persist.rs:271`）。集成测试构造不出画像，这不是实现细节，
是设计 §2.1 那条保证的直接后果。

- `a_profile_carries_the_twelve_fields_of_247`（crate 内）：构造一个十二字段**全取非默认值**的画像，
  逐字段比对（十二项各断言一次）。红色的条件：某字段与邻字段写串即红（全零/默认值会让这类缺陷静默）。
  **十二是「画像对象」的字段数，不是 `model_profile` 表的列数——后者是十一**（`skill_vector` 落在
  `model_skill_score`），见 Task 5 的列清单用例与 Task 8。
- `the_confidence_is_a_ratio_and_nothing_else`（crate 内）：`confidence` 是 `Ratio`；`Ratio` 的越界输入在构造期即被拒。
- `a_profile_has_no_total_score`（crate 内）：**这条没有运行期形态**——它由 `tests/compile_fail/overall_score_cannot_be_read.rs`
  钉住（读 `profile.overall_score()`，判据是**编译失败**）。此条在用例清单里留名，以免被当成漏项（设计 §2.3 第 3 条 (a)）。
- `tests/compile_fail/model_profile_cannot_be_built.rs`：**结构体字面量**构造被拒（字段私有）。
- `tests/compile_fail/model_profile_has_no_constructor.rs`：**关联函数** `ModelProfile::try_new(…)` 在 crate 外
  被拒——实测 rustc 1.95 报 **`E0624`**（`associated function … is private`；**三份样例的错误码各不相同**，
  逐份的值见下面的表）。
  **这一份必须与上一份并存，判据是「每条构造通道各占一份样例」**：私有字段与私有构造函数是**两条不同的通道**，
  只钉「字面量构造」那一份时，把 `try_new` 改成 `pub` **不会有任何用例变红**——**缺的正是 fail-open 的那一侧**
  （P3A 的 `capability_fields_are_private.rs` 与 `capability_has_no_constructor.rs` 是一对，同一判据）。
- `tests/compile_fail/overall_score_cannot_be_read.rs`：同法，钉 §248 禁令的形状（§4.4 完成判据的「不依赖单一总分」）。
- **三份样例的判据都是「编译失败且失败原因正确」**——每份 `.stderr` 钉住预期报错，否则
  「因为拼错函数名而编译失败」也会让用例变绿。**三份的错误码逐份不同，必须对着实跑值写**：

  | 样例 | 实测错误码 | 诊断 |
  |---|---|---|
  | `model_profile_cannot_be_built.rs` | `E0451` | 字段私有（结构体字面量构造被拒） |
  | `overall_score_cannot_be_read.rs` | `E0599` | 该方法不存在 |
  | `model_profile_has_no_constructor.rs` | `E0624` | 关联函数私有 |

  ——**`.stderr` 不许凭记忆写，也不许拿一个值套三处**，见「关于本计划的代码块」一节。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test type_level
```

- [ ] **Step 3: 实现**

```rust
/// §247 的模型画像。字段私有，**crate 外无公开构造函数**。
///
/// **措辞与保证等强**：被挡住的是 **crate 外构造**；crate 内 `try_new` 是公开的（`persist::load_profile` 要用它）。
/// 若写成「唯一产生点」就过头了——**它是「crate 外无入口」，不是「全仓只有一个构造点」**（设计 §2.1）。
///
/// 这条保证与 `RoutableModel` 的闸门**同为结构性**：若画像能在内存里自由构造，
/// 一个从未落库、从未过画像流水线的模型就能被送进 `rank`，§21 的「新增模型不能直接进入自动 Router」随之失效。
pub struct ModelProfile {
    id: ModelId, version: String, provider: String, model_revision: String,
    modalities: Vec<String>, tools: Vec<ToolId>, skill_vector: SkillVector,
    failure_modes: Vec<String>, latency_profile: Option<Latency>, cost_profile: Option<Cost>,
    evidence_count: u64, confidence: Ratio,
}

impl ModelProfile {
    pub(crate) fn try_new(/* 十二个字段，顺序同上 */) -> Self;
    pub fn id(&self) -> &ModelId;
    pub fn skill_vector(&self) -> &SkillVector;
    pub fn confidence(&self) -> Ratio;
    pub fn evidence_count(&self) -> u64;
    pub fn modalities(&self) -> &[String];
    pub fn tools(&self) -> &[ToolId];
    pub fn failure_modes(&self) -> &[String];
    // 其余字段的访问器按消费方需要增补，不预先铺开（设计 §2.1）。
}
```

三点写在实现里，不写成注释就算完：

- **`cost_profile` / `latency_profile` 复用 `continuum_capability::{Cost, Latency}`**，以 `Option<…>` 出现
  （`None` = 尚未登记画像，与 P3A 的 `tool.cost` 列同一语义），**不另建一对自己的单位结构体**——
  否则「一个成本画像」就有两套词汇表（设计 §2.5）。**本层不给它们取值域**。
- **§81 与 §247 不一致处按 §247**：不擅自补 `deployment` 与 `capability fingerprint`（设计 §2.1、§11 第 7 条）。
- **`trust` 不在此处**：§247 的十二个字段里没有它，本设计不擅自加（据实退件见 `## 遗留`）。

`Cargo.toml` 与 `ALLOWED` **两处一起改**（那张表是逐对 `assert_eq!`，只改一处会红）：
`continuum-capability`（`Cost` / `Latency`）与 `continuum-core`（`ModelId` / `ToolId`）；dev-dep 加 `trybuild`。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(model-registry): §247 的 ModelProfile 与两处不可表达性"
```

---

### Task 4: 生命周期十态、可路由闸门与迁移表

**Files:**
- Create: `crates/continuum-model-registry/src/lifecycle.rs`
- Modify: `crates/continuum-model-registry/{src/lib.rs,src/error.rs}`
- Create: `crates/continuum-model-registry/tests/lifecycle.rs`
- Create: `crates/continuum-model-registry/tests/compile_fail/routable_model_fields_are_private.rs`（配 `.stderr`）
  （执行期改名：原名 `routable_model_cannot_be_built.rs`。原名会被读成「crate 外造不出 `RoutableModel`」，
  而 `RoutableModel::try_new` 按设计 §4.2 **是 `pub` 的**、闸门在它函数体内，故那个说法不成立。
  样例钉的一直只是**结构体字面量**这条通道，新名如实。）

**Interfaces:**
- Consumes: Task 3 的 `ModelProfile`
- Produces: `continuum_model_registry::{LifecycleState, RoutableState, RoutableModel, LifecycleError, RoutingError}`
  （`RoutingError` 本 task 只含 `NotRoutable`，其余变体由 Task 10 增补）

- [ ] **Step 1: 写用例**

- `the_ten_lifecycle_states_are_recorded_verbatim`：十态逐项（§249 照录），并逐项钉 `as_str` 的
  小写 `_` 连接字面量 + `parse` 往返。红的条件：多词或大写即红（手册写字面量，钉格式）。
- `only_the_six_routable_states_pass_the_gate`：**十态逐项喂进 `RoutableModel::try_new`**，六态 `Ok`、
  四态 `Err(RoutingError::NotRoutable { state })` 且**断言是哪一枚**（不是「返回了 Err」）。这是枚举式绝对断言，逐项钉。
- `a_stale_model_is_not_routable_while_an_active_one_is`：**两侧对钉的具名用例**——`Stale` → `Err(NotRoutable{state: Stale})`，
  同一份画像 + `Active` → `Ok`。**只钉一侧是 fail-open 的那一侧**：删掉 `stale` 这一臂时，
  「正常态确实会被路由」的那侧照样绿，只有这条会红。依据是已裁的 §4.2（`set-decisions` 第一节第 4 条）。
- `a_degraded_model_still_passes_the_gate`：`Degraded` → `Ok`，并在候选的 `reason` 里可见（§4.2 末段：可见但不禁）。
  红的条件：把 `Degraded` 一并挡下即红（撞名的是 `ProviderHealth::Degraded`，两个轴上的两个东西）。
- `the_transition_table_accepts_every_listed_pair`：§4.1 的合法对**逐条** `Ok`（正常阶梯六态线性、
  `verified→active`、`active→stale`、`stale→researched`、`active↔degraded`、`*→quarantined`、`*→disabled`、
  `quarantined→disabled`、`disabled→unprofiled`），每条的返回值与断言各写一次。
- `an_unlisted_transition_pair_is_rejected_with_both_ends`：若干条未列出的对（含 `stale→active`、
  `disabled→active`、`quarantined→active`）各断言 `Err(Illegal { from, to })` 且两端**正是给的那一对**。
  红的条件：把 `Illegal` 的两个字段写反、或把未列出的对放行即红。
- `quarantined_and_disabled_are_reachable_from_every_state`：十态**逐项**×两目标共 20 条，
  每条断言 `Ok`（§4.1：事故与人工下线不挑时机，与 P1 的 `INVALIDATED`/`CANCELLED` 同判据）。
  **这 20 条含两个自环**（`Quarantined → Quarantined`、`Disabled → Disabled`），见下面那条自环专案。
- `a_self_transition_follows_the_any_state_rule`：**自环的专案照片，不靠上面的矩阵顺带**。
  十态**逐项** `x → x`：目标是 `Quarantined` 或 `Disabled` 的两条 `Ok`，其余八条
  `Err(Illegal { from, to })` 且两端**正是给的那一对**。**两侧都钉**——只写「自环一律非法」会与矩阵那 20 条
  直接打架（至少一条必红），只写「自环一律合法」则会把 `Active → Active` 这种未列出的对放行。
- `disabling_returns_a_model_to_unprofiled_not_active`：`disabled → unprofiled` 为 `Ok`、
  `disabled → active` 为 `Err(Illegal{..})`。**两侧对钉**：这条钉住 §21 的「重新启用须重走画像流水线」那个方向，
  只写「`→unprofiled` 可以」的话，把出口改成 `active` 不会有任何用例变红。
- `tests/compile_fail/routable_model_fields_are_private.rs`：crate 外**用结构体字面量**构造被拒（E0451，字段私有）。
  与 P3A 的 `AuthorizedTool` 同形。
  （原写「无公开构造函数」——**这句与设计 §4.2 自己的 `pub fn try_new` 相抵**，更正为「结构体字面量」；
  文件原名 `routable_model_cannot_be_built.rs`。）

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test lifecycle
```

- [ ] **Step 3: 实现**

```rust
/// §249 的十个状态（照录）。
pub enum LifecycleState { Discovered, Unprofiled, Researched, Probed, Verified, Active,
                          Stale, Degraded, Quarantined, Disabled }

/// 可被**自动路由**的那六个状态。`unprofiled` / `quarantined` / `disabled` 是 §249 禁的三态；
/// `stale` 是裁决额外挡下的第四态（§82：漂移中的模型正在退出服役，放行即 fail-open）。
/// 做法与 P3A 的 `full_access` 同：**不是「禁止作为默认」，是没有这个成员**。
///
/// 措辞与保证等强：这四态在 `LifecycleState` 上**是表达得出来的**（画像流水线、漂移标记、
/// 隔离与停用都要用它），被挡住的是**路由路径**——**不声称「四态不可表达」**。
pub enum RoutableState { Discovered, Researched, Probed, Verified, Active, Degraded }

impl TryFrom<LifecycleState> for RoutableState { type Error = RoutingError; }   // NotRoutable { state }

/// 一个**可交给 Router** 的模型：画像 + 已过闸门的状态。字段私有、无公开构造函数。
///
/// 收 `LifecycleState` 而不是 `RoutableState`：§4.2 的十态逐项用例要求「把任意状态喂进来」，
/// 闸门若在构造之外，那十态的照片就得各写一遍 `try_from`（设计 §2.1 的示例写的是 `RoutableState`，
/// 与 §4.2 的用例要求不一致；本计划取 §4.2，见 `## 遗留`）。
pub struct RoutableModel { profile: ModelProfile, state: RoutableState }

impl RoutableModel {
    pub fn try_new(profile: ModelProfile, state: LifecycleState) -> Result<Self, RoutingError>;
    pub fn profile(&self) -> &ModelProfile;   // `RankingPolicy::evaluate` 要读它（§5.3 的签名即判据）
    pub fn state(&self) -> RoutableState;
}

/// §249 未给迁移关系，本表由本设计定（与 §237 落在 P1 的情形相同）。
/// `from` / `to` 相同不是特例：**自环的合法性由那张表决定**（见上面的执行期裁定），
/// 故实现里**不加**「`from == to` 一律拒绝」这一臂。
pub fn transition(from: LifecycleState, to: LifecycleState) -> Result<LifecycleState, LifecycleError>;
```

`transition` 的返回值为**迁移前的状态**（信息量非零的那一侧），与设计 §3.2 的定稿一致。

> **执行期裁定（协调者，2026-10-05）：自环取自「可 Ok」的那一侧。**
> 设计 §4.1 有两句相抵的话：「`* → quarantined` / `* → disabled` 可由**任意**状态进入」与
> 「自环（`x → x`）非法」。裁决取前者——即**自环的合法性由那张表决定**：目标是
> `Quarantined` / `Disabled` 的自环合法，其余自环是未列出的对、一律 `Illegal`。
> 两条判据：**一条与自己那张表相抵的「一律」正是本项目要消灭的形状**；且 P1 的既有实现
> `crates/continuum-graph/src/state.rs:17-19`（`if matches!(to, Invalidated | Cancelled) { return Ok(to); }`）
> **恰恰放行自环**——设计借的正是它那条判据。**设计那处的相抵由设计作者改，本计划不改设计**；
> 落在这里的是裁决本身。

**`LifecycleError` 不叫 `RegistryError`**：C 的设计在 `continuum-provider` 里已有一个同名不同物的
`RegistryError`（`NotFound` / `Duplicate`），两件事一个名字会让调用方与后来者混淆（设计 §4.1）。
本 task 先定义 `Illegal { from, to }`；`UnknownModel` 与 `Persist` 两个变体在 Task 6、`ProfileBeforeVerified`
在 Task 7 跟随各自的产生方落地（**没有产生方的变体不先铺开**，本仓对这类变体的处置是删或写明理由）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): §249 十态、可路由闸门与迁移表"
```

---

### Task 5: 三张表、迁移编号的现场核实与注册

**Files:**
- Create: `crates/continuum-model-registry/src/persist.rs`
- Modify: `crates/continuum-model-registry/{Cargo.toml,src/lib.rs}`
- Modify: `crates/continuum-runtime/{Cargo.toml,src/main.rs}`
- Modify: `crates/continuum-runtime/tests/{dependency_direction.rs,migrations.rs,startup.rs}`
- Create: `crates/continuum-model-registry/tests/persist.rs`

**Interfaces:**
- Produces: `continuum_model_registry::p3d_model_migrations`

**本 task 起登记 `continuum-persist` 这条边**（Step 4 的 `p3d_model_migrations()` 返回 `Vec<Migration>`，
Step 2 的夹具用 `Db::open_with` / `builtin_migrations()`）：`crates/continuum-model-registry/Cargo.toml`
加 `continuum-persist`、dev-dep 加 `tempfile`，`ALLOWED` 的 `continuum-model-registry` 条目**由 Task 3 登记的
两元素改为三元素**：`["continuum-capability", "continuum-core"]` → `["continuum-capability", "continuum-core",
"continuum-persist"]`（三枚与 Global Constraints 的边表相同；数组按本仓既有的字母序写，同 C 的 provider 条目）
——**两处一起改**（那张表是逐对 `assert_eq!`，
只改一处会红）。**边由用它的那个 task 登记，而本 task 就是它的使用者**（Global Constraints 的口径）；
Task 6 不再新增这条边，`continuum-core` 与 `continuum-capability` 两条边在 Task 3 已登记、本 task 不动它们。

- [ ] **Step 1: 现场核实该库的空号（**不许假定**）**

读 `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()` 与
`crates/continuum-runtime/tests/migrations.rs` 的 `expected_migrations()`，把两处的已占编号逐条列出来
（本轮动手前是 `1`、`2`、`10`、`20`、`30`、`40`、`41`、`50`——**这是任务书写下来的值，读源文件之前不得当事实**）。
确认 `80` 在该集合里未被占用；**若已被前序子项目占去，改用未占用的空号并回报**。
「未占用」的判据是**按库**说的：`crates/continuum-persist/src/bin/crash-writer.rs` 的 `60` 与
`crates/continuum-persist/tests/recovery.rs` 的探针表 `50` 都在**别的库**里，无害，但也不得据此认为
「60 / 50 已占」（设计 §3.3）。

**只取 `80`、不预留 `81`**：三张表全在迁移 80 里（设计 §3.3 明写「三张表在一条迁移里」），
本子项目没有第二个建表点，而预留一个没有表要建的编号就是留一条死迁移（设计 §3.3 已删第一版那句并留了来历）。

- [ ] **Step 2: 写用例**

`tests/persist.rs` 的夹具：`Db::open_with(path, builtin_migrations() + p3d_model_migrations())` 后必须 `migrate()`
——漏掉那一行时各用例会一起挂在 `no such table`，而报错位置指向被测函数（同 P3A 的 `tests/persist.rs` 首注）。

- `the_three_tables_exist`：`model_registry` / `model_profile` / `model_skill_score` 三张表各查得到。
- `the_model_profile_columns_are_exactly_the_eleven_columns`：`model_profile` 的列清单**逐列**断言
  （加一列即红）。这是 §2.3 第 2 条的结构性事实在库侧的落点，**不是**「顺便查一下表结构」。
  **「§247 的十二个字段」与「`model_profile` 的十一列」是两个数，不许混用**：第十二个字段 `skill_vector`
  **不在这张表里**，它落在 `model_skill_score`（每维度每版本一行）——故本用例的期望列数是 **11**，
  照 12 去建表会多出一列。（本计划原稿把这条用例写成「十二字段」，brief 里也如此——**那是错的，已订正**；
  实现者按设计 §3.1 落的表，用例名取的是 `…_the_eleven_columns`。）
  第十二个字段的落点由 `a_skill_observation_round_trips_field_by_field`（Task 8）承重。
- `the_model_profile_has_no_total_score_column`：`SELECT overall_score FROM model_profile` 得到**具体** `Err`
  （§2.3 第 3 条 (b)）；红的条件：谁给表加了这一列即红。

- [ ] **Step 3: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test persist
```

- [ ] **Step 4: 实现**

迁移（**编号取 Step 1 核实后的空号**；三张表在一条迁移里，`Db::migrate` 用 `execute_batch`）：

```sql
CREATE TABLE model_registry (          -- §21 §249：登记与生命周期
    id               TEXT PRIMARY KEY, -- §81／§315 的模型 id
    lifecycle_state  TEXT NOT NULL     -- §249 十态，小写 _ 连接
);

CREATE TABLE model_profile (           -- §247：画像（probed → verified 时才产生，见 §4.3）
    id               TEXT PRIMARY KEY REFERENCES model_registry(id),
    version          TEXT NOT NULL,
    provider         TEXT NOT NULL,
    model_revision   TEXT NOT NULL,
    modalities       TEXT NOT NULL,    -- JSON 字符串数组
    tools            TEXT NOT NULL,    -- JSON 字符串数组，元素是 ToolId
    failure_modes    TEXT NOT NULL,    -- JSON 字符串数组
    cost_profile     TEXT,             -- NULL = 尚未登记；非 NULL = 已登记（P3A 的存在性编码）
    latency_profile  TEXT,             -- 同上
    evidence_count   INTEGER NOT NULL,
    confidence       TEXT NOT NULL     -- Ratio 的十进制串
);

CREATE TABLE model_skill_score (       -- §248 §24：时间序列，不是常数
    model_id       TEXT NOT NULL REFERENCES model_profile(id),
    dimension      TEXT NOT NULL,      -- §248 九维，小写 _ 连接
    score_version  INTEGER NOT NULL,   -- §24 的 version
    score          TEXT NOT NULL,
    confidence     TEXT NOT NULL,
    sample_count   INTEGER NOT NULL,
    time_range_start INTEGER NOT NULL, -- Unix 毫秒
    time_range_end   INTEGER NOT NULL,
    PRIMARY KEY (model_id, dimension, score_version)
);
```

三处判据写在文件头，不当注释就算完：

- `model_skill_score` 挂 **`model_profile`** 而不是 `model_registry`：观测是画像的一部分，没有画像就没有观测
  （故 `load_skill_vector` 不可能是「对未画像的模型返回空向量」）。
- 主键 `(model_id, dimension, score_version)` 是「同一维度的同一版本只有一次观测」的落点；
  `model_skill_score` 与 `model_registry` 的写入都是**裸 `INSERT`**，不 `OR REPLACE`——否则「补记一次观测」
  会静默覆盖历史，而 §24 要的正是历史。
- 三个列表列取 **JSON 数组容器**，与 P3A 的 `tool.required_capabilities` 同形；**不建子表**是因为它们
  **没有逐元素属性**（与 `model_skill_score` 的分界是判据，不是巧合）。

- [ ] **Step 5: 注册迁移并更新连带断言**

`main.rs` 的 `runtime_migrations()` 加 `continuum_model_registry::p3d_model_migrations();`，
并在该函数的文档注释里加一句：本子项目的三张表**由 D 自己注册**（「谁的表谁注册」，协调者裁决），
**它的消费方（子项目 G）本轮尚不存在**——这句是给后来者的，不要删。
`runtime/Cargo.toml` 加依赖边；`ALLOWED` 的 `continuum-runtime` 条目加 `continuum-model-registry`。

连带断言**三处要改、一处不要改**（`crates/continuum-runtime/tests/startup.rs`）：两处「迁移应用 8 项」→ `9 项`、
`:79` 的「6 项」→ `7 项`（那个用例的 `Db::open` 只带 P0 内置迁移，故启动时补的是 P1+P2+P3A+P3D）；
`second_startup_applies_no_migration` 的 `0` **不要动**——它断的是「第二次启动不再应用」，与总数无关。
三处的行内注释（「P0 两条 + P1 两条 + P2 三条…」那类）**一并更新**，否则注释会与断言互相打脸。
`tests/migrations.rs` 的 `expected_migrations()` 同步 extend（**这是第二份转录，不是重复**，该文件的注释说明了理由）。

**具体行号与数值以 Step 1 读到的源文件为准**，上面的数字是任务书写下的预期值。

- [ ] **Step 6: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast     # 迁移编号与计数是跨 crate 才可见的效果，必须全量跑
git add crates/continuum-model-registry crates/continuum-runtime Cargo.toml Cargo.lock
git commit -m "feat(model-registry): 三张表与迁移注册"
```

---

### Task 6: 登记项与生命周期的落库读写

**Files:**
- Modify: `crates/continuum-model-registry/src/persist.rs`
- Modify: `crates/continuum-model-registry/{src/error.rs,src/lib.rs}`
- Modify: `crates/continuum-model-registry/tests/persist.rs`

**Interfaces:**
- Consumes: Task 4 的 `LifecycleState` / `transition`、Task 5 的 `p3d_model_migrations`
- Produces: `continuum_model_registry::{register_model, load_lifecycle, transition_in_tx}` 与
  `LifecycleError::{UnknownModel, Persist}`

- [ ] **Step 1: 写用例**

- `registering_a_model_leaves_it_discovered`：`register_model` 后 `load_lifecycle` 得 `Some(Discovered)`（§21 发现即登记）。
- `registering_the_same_id_twice_is_rejected_and_the_row_is_unchanged`：同 id 再登记返回具体 `Err`，
  **且原行状态不变**（裸 `INSERT`，不是 `OR REPLACE`）。
- `every_lifecycle_state_round_trips`：十态**逐项**：写进库再读回，得同一个变体。
- `the_lifecycle_state_column_is_lowercase_with_underscores`：直接查表，断言列值是小写串
  （`discovered` / `quarantined` / …），**不是** `Debug` 表示——手册写字面量，钉格式。
- `an_unknown_lifecycle_state_in_the_column_is_rejected`：裸 SQL 写入表外取值，`load_lifecycle` 返回**具体** `Err`，
  **不取默认值**（把串猜成另一枚状态，正是设计要拦的）。
- `an_unknown_model_has_no_lifecycle`：库里没有的 id → `Ok(None)`（不是 `Err`；`UnknownModel` 用在**转移**上）。
- `a_transition_writes_the_new_state_and_returns_the_previous`：`transition(tx, id, to)` 落库后读回的是 `to`，
  返回的是**迁移前**的那个状态（Task 4 定下的语义）。两侧都断言。
- `a_transition_on_an_unknown_model_is_rejected_with_the_id`：`Err(LifecycleError::UnknownModel { id })`，
  且**断言 id 是给的那一个**。
- `an_illegal_transition_leaves_the_row_unchanged`：非法对返回 `Illegal{from,to}`，**且库里的状态没变**
  （失败路径要断言是哪一种 `Err`，还要断言没有半写的副作用）。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test persist
```

- [ ] **Step 3: 实现**

```rust
/// §21 的「发现即登记」：新模型进 Registry，初始状态 `discovered`。裸 `INSERT`。
pub fn register_model(tx: &Tx<'_>, id: &ModelId) -> Result<(), PersistError>;
/// 不存在返回 `Ok(None)`；表外取值返回具体 `Err`（不取默认值）。
pub fn load_lifecycle(tx: &Tx<'_>, id: &ModelId) -> Result<Option<LifecycleState>, PersistError>;

/// §4.1 迁移表的落库版：读当前状态 → 内存 `transition` → 写回。
/// 非法对**在写之前**返回 `Err`，故失败不留下半写的行。
pub fn transition_in_tx(tx: &Tx<'_>, id: &ModelId, to: LifecycleState)
    -> Result<LifecycleState, LifecycleError>;   // 返回迁移前的状态
```

**本层只提供迁移表与写入口**：`discovered→unprofiled` 的触发方是登记方，`unprofiled→…→verified` 归 Model
Onboarding Task（尚未建），`active→stale` 归行为指纹的周期 probe（尚未建）——**本阶段这些产生方一个都不存在**，
故「谁真的推进了生命周期」只能由测试直接调 `transition_in_tx` 来演（设计 §4.3、§10 第 3 条）。
`LifecycleError` 本 task 增补 `UnknownModel { id }` 与 `Persist(#[from] PersistError)`。
**本 task 不新增任何依赖边**：`continuum-persist`（与 dev `tempfile`）的 `Cargo.toml` 与 `ALLOWED` 登记
已在 Task 5 随它的使用者一起落地。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(model-registry): 登记项与生命周期的落库读写"
```

---

### Task 7: 画像的落库与「画像早于 verified 被拒」

**Files:**
- Modify: `crates/continuum-model-registry/src/persist.rs`
- Modify: `crates/continuum-model-registry/{src/error.rs,src/lib.rs,Cargo.toml}`
- Modify: `crates/continuum-model-registry/src/profile.rs`（订正三处把 `load_profile` 记成 Task 5 的错标，见下）
- Modify: `crates/continuum-model-registry/tests/persist.rs`

**Interfaces:**
- Consumes: Task 3 的 `ModelProfile`、Task 6 的 `load_lifecycle`
- Produces: `continuum_model_registry::{save_profile, load_profile}` 与 `LifecycleError::ProfileBeforeVerified`

**连带面：`profile.rs` 里把 `load_profile` 记成 Task 5 产物的地方要一并订正。** 已知**三处**
（`crates/continuum-model-registry/src/profile.rs` 的 `:379`、`:413`、`:418`；**行号以本 task 开工时重读的为准**）——
`:379` 是「消费方随 `load_profile`（Task 5）」那一句。**`load_profile` 实际由本 task（Task 7）交付**，
Task 5 只建表。
**这一步不许靠上面这三个行号做完就收工**：Task 5 的实现者只报出两处，第三处（`:379`）是评审补出来的——
**按短清单逐行改，正是这类错标的典型遗留形态**（改完还剩一句自称「Task 5」的残句，而它看起来像已订正过）。
故落地时的判据是：**在 `src/` 里 grep 出所有含 `Task 5` 的行，逐处连同它的上一行与下一行一起读**，
确认它讲的是不是 `load_profile`（是则改），改完再 grep 一遍为零。
**为什么不是「grep 两者的共现行」**：同一句话被折成两行时同线匹配会漏——本 task 的第三处错标正是这样
（`load_profile` 在 `profile.rs:414`，而同一句的 `Task 5` 在**上一行** `:413`），只查共现行只找到 2 处、漏掉 1 处。
**这类判据的通病是「按行匹配、按句断言」**：凡用 grep 当判据，都要按**类别**取候选、再**人工读邻行**。
**收件人：Task 7 的实现者。**

- [ ] **Step 1: 写用例**

- `a_profile_round_trips_field_by_field`：先 `register_model` + 迁到 `verified`，存一条**十二个字段**全非默认的
  画像（**画像有十二个字段，而 `model_profile` 只有十一列**——第十二个 `skill_vector` 落在
  `model_skill_score`，本用例不走它），经 `load_profile` 读回，**逐字段**比对（含 `tools` 的元素是 `ToolId`、
  两个 `Option<Cost/Latency>` 的 `Some` / `None` 两侧）。红的条件：某列漏写或与邻列写串即红。
  **`skill_vector` 不在比对之列，且这不是遗漏**：`save_profile` 不写观测（唯一写者是
  `save_skill_observation`），故 `save∘load` 在那个字段上不是恒等——该不对称写在 `save_profile`
  的函数文档里（见 Step 3）。本用例比的是**十一列**对应的那些字段。
  **本用例的入参画像怎么来，是这一步第一件要解决的事**：`save_profile` 收一枚 `ModelProfile`，
  而它 crate 外造不出、`load_profile` 又要求库里先有行——**故「先写后读」不能自举**。
  本 task 的处置（Task 7 实现者已按此落地）：**先用裸 SQL 播下一行**（`INSERT INTO model_registry` ＋
  `INSERT INTO model_profile`），再走类型化接口读回、改、写。**两个方向都要**：裸 SQL 只用来造第一枚，
  余下的断言仍走 `save_profile` / `load_profile`，否则「往返」这件事就没有被测到。
- `saving_a_profile_before_verified_is_rejected_with_the_state`：**十态逐项**——六态（`verified` / `active` /
  `stale` / `degraded` / `quarantined` / `disabled`）`Ok`，四态（`discovered` / `unprofiled` / `researched` /
  `probed`）`Err(LifecycleError::ProfileBeforeVerified { state })` 且**断言是哪一枚**。
  这是十项枚举，逐项钉，不只钉 `researched` 一条。红的条件：把允许集只留 `verified` 即红四态那条；
  把拒绝集改宽即红六态那条。
- `load_profile_of_an_unknown_id_is_none`：库里不存在的 id → `Ok(None)`。
  这是「画像必须来自库」这条路的**反例照片**（设计 §2.1）。
- `a_profile_for_an_unregistered_model_is_rejected`：`model_profile.id REFERENCES model_registry(id)`，
  对未登记的模型 `save_profile` 返回具体 `Err`（外键在 `Db::open_with` 的 `PRAGMA foreign_keys=ON` 下生效）。
- `the_three_list_columns_round_trip_as_json_arrays`：三个列表列**各**一条（含**空数组**与非空数组两侧）
  ——空数组与 `NULL` 不是一回事，而三个列都是 `NOT NULL`。
- `an_out_of_range_confidence_in_the_column_is_rejected`：裸 SQL 写 `confidence = '1.5'` / `'abc'`
  → `load_profile` 返回具体 `Err`（`Ratio::parse` 给 `None`，`persist` 转成 `Err`，**不取默认值**）。
- `the_cost_and_latency_columns_distinguish_absent_from_registered`：`NULL` 读回 `None`、
  非 NULL 读回 `Some`——两侧都钉（P3A 的存在性编码，空串只表示「已登记」）。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test persist
```

- [ ] **Step 3: 实现**

```rust
/// 画像的写入点。**读登记项的当前状态**，不在允许集内即 `Err(ProfileBeforeVerified { state })`。
/// **返回 `LifecycleError` 而不是 `PersistError`**：闸门的那枚变体装不进后者（见 `## 遗留`
/// 的「save_profile 的返回类型」一条）——设计 §3.2 与 §4.1/§4.3 两处相抵，此处取后者的判据。
///
/// **它写十一列，不写观测**（`skill_vector` 不在 `model_profile` 表里；观测的唯一写者是
/// `save_skill_observation`）。故 **`save∘load` 在 `skill_vector` 这个字段上不是恒等**——
/// 这条不对称要写在函数文档里，**免得有人拿它做往返断言**：`save_profile` 之后 `load_profile`
/// 读回的向量来自技能表，与传进去的那一枚没有关系（除非调用方自己先写过观测）。
/// 与它对称的另一半在 `load_profile` 的文档里（**读**十二个字段、**组合** `load_skill_vector`）。
pub fn save_profile(tx: &Tx<'_>, profile: &ModelProfile) -> Result<(), LifecycleError>;
pub fn load_profile(tx: &Tx<'_>, id: &ModelId) -> Result<Option<ModelProfile>, PersistError>;
```

**画像在 `probed → verified` 时才产生**：允许集是 `{verified, active, stale, degraded, quarantined, disabled}`
（画像已产出，异常态只是改了它的可用性），拒绝集是 `{discovered, unprofiled, researched, probed}`。
依据是 §22 的流水线顺序——「生成初步画像 → 执行 Active Probe → Verifier → **生成正式 Profile**」。

**§22 的「初步画像」是一个被刻意丢弃的中间物，本设计不落它**——这一句要写进 `persist.rs` 的文件注释，
否则读了 §22 的人会以为漏了一环：初步画像先于 probe，而 `model_profile` 行只在 `probed → verified` 时产生；
它作为一个可路由的画像存下来，就是给从未实测的模型一个与实测画像同形的身份。
**它的效果由 §247 的两个字段承载**：`evidence_count` 与 `confidence`（初步阶段是「证据条数」与「低置信」，
正式画像生成时一并写入）。

**编码委托**：`Ratio` / `SkillScore` 用各自那一对 `as_str` / `parse`；三个列表列只做**容器**
（JSON 字符串数组，`tools` 的元素取 `ToolId::as_str`），编码本体不在 `persist.rs` 里另建表。
`Cargo.toml` 加 `serde_json`（列表列的容器）。`LifecycleError` 本 task 增补 `ProfileBeforeVerified { state }`。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): 画像的落库与「画像早于 verified 被拒」"
```

---

### Task 8: 技能观测的落库与「当前值」

**Files:**
- Modify: `crates/continuum-model-registry/src/persist.rs`
- Modify: `crates/continuum-model-registry/{src/lib.rs}`
- Modify: `crates/continuum-model-registry/tests/persist.rs`

**Interfaces:**
- Consumes: Task 2 的 `SkillVector` / `current_observation`、Task 7 的 `save_profile`
- Produces: `continuum_model_registry::{save_skill_observation, load_skill_vector, load_skill_series}`

**夹具**：观测的外键挂在 `model_profile` 上，故每个用例都要先有一份**已画像**的模型。
**第一枚画像同样只能由裸 SQL 播下**（理由与做法见 Task 7 的 `a_profile_round_trips_field_by_field`
与 Task 11 的夹具段——`save_profile` 收的画像 crate 外造不出，链条不能自举）。

- [ ] **Step 1: 写用例**

- `a_skill_observation_round_trips_field_by_field`：五个字段（score / confidence / sample_count / version /
  `time_range` 的闭区间两端）各写非默认值并逐项比对。
- `the_same_dimension_and_version_cannot_be_written_twice`：同 `(id, dim, version)` 第二次写入被主键拒，
  **且原行内容不变**（裸 `INSERT`）。红的条件：改成 `OR REPLACE` 即红——那会让「补记一次观测」静默覆盖历史，
  而 §24 要的正是历史。
- `a_series_comes_back_in_ascending_score_version`：同维度存三个版本，`load_skill_series` 返回**三条且升序**
  （顺序固定，调用方才不必自己排）。**红的条件：把 `ORDER BY` 的排序方向改成 `DESC`，或漏掉一条即红。**
  **原稿写的是「去掉 `ORDER BY` 即红」——实测不成立**（Task 8 实现者的变异实测）：复合主键
  `(model_id, dimension, score_version)` 的索引序**恰好与升序 `ORDER BY` 一致**，故删掉那一句时
  输出顺序不变、**假绿**。这是本项目那条「**等价变异体不算变红**」的实例——判据是「这两版在哪个入参上
  会给出不同结果」，举不出即是等价；处理是**换真变异体**（改 `DESC`），**不是补用例**。
- `the_vector_takes_one_observation_per_dimension_by_highest_version`：两版本、**`version` 大的 `time_range` 更早**
  → 向量里的当前值是 `version` 大的那条（与 Task 2 的纯函数用例同一条判据，此处钉的是**落库路径**也用它）。
  **红的条件：把「取 `version` 最大」改成「按 `time_range` 最晚取」——原稿若写「取最后一条即红」，那句同样不成立**
  （同一索引序，末条恰是 `version` 最大的那条），处理同上：换真变异体。
- `every_dimension_without_an_observation_is_absent_not_zero`：九维**逐项**断言 `None`；
  另一侧：写入两维后，这两维 `Some`、其余七维仍 `None`（**两侧都钉**）。
- `the_dimension_column_is_lowercase_with_underscores`：直接查表，断言 `tool_use` 与 `constraint_following`
  两个多词项的字面量（**逐项**），不是 `Debug` 表示。
- `an_unknown_dimension_in_the_column_is_rejected`：裸 SQL 写表外维度名 → `load_skill_vector` 返回具体 `Err`。
- `an_out_of_range_score_in_the_column_is_rejected`：`score = 'NaN'` 与非数值各一条，各断言具体 `Err`。
- `an_observation_for_an_unprofiled_model_is_rejected`：外键挂 `model_profile`，对没有画像的模型写入被拒
  （「没有画像就没有观测」的库侧落点）。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test persist
```

- [ ] **Step 3: 实现**

```rust
pub fn save_skill_observation(tx: &Tx<'_>, id: &ModelId, dim: SkillDimension, obs: &SkillObservation)
    -> Result<(), PersistError>;
/// 该画像的向量（九维各取当前一次观测）。画像不存在 → `Ok(None)`。
pub fn load_skill_vector(tx: &Tx<'_>, id: &ModelId) -> Result<Option<SkillVector>, PersistError>;
/// 按 `score_version` 升序返回该维度的全部观测。
pub fn load_skill_series(tx: &Tx<'_>, id: &ModelId, dim: SkillDimension)
    -> Result<Vec<SkillObservation>, PersistError>;
```

`load_skill_series` 的消费方是 §82 的行为指纹（「如果表现突然变化」）——判「变化」至少要看两次观测，
故它现在就必须有；只存当前值会让 §82 的判据无法成立。**它的产生方（周期性 probe）本阶段不存在**，
这一点按设计 §10 第 1 条写明，**不靠一句将来时糊过去**。

**本 task 还交付一步（协调者裁决，2026-10-06，见 `## 遗留` 的同名条）**：交付 `load_skill_vector` 的同时，
**让 `load_profile` 调它来填画像的 `skill_vector`**——`load_profile` 现在读回的画像十二个字段齐全。
**两条一起写进文档注释，不许含糊**：
- **`load_profile` 填 `skill_vector` 时，那个字段的唯一来源是 `load_skill_vector`**；`load_profile` **组合**它，
  **不自己再查 `model_skill_score`**（那就是同一字段的第二个来源，等同一次重复产生点）。
  **口径必须说准（订正注记）**：协调者裁决 A 的原话是「`load_skill_vector` 是 `model_skill_score` 表的
  唯一装载者」——**那句是关于整张表的，而它是假的**：同一个文件里 `load_skill_series` 也读那张表。
  本意是「`load_profile` 不另查那张表」，**故收窄为关于 `skill_vector` 这个字段的说法**。
  判据：**关于整张表的断言 ≠ 关于一个字段的断言**——前者要穷举该表的所有读函数，后者只要钉住一条路径。
  **该表另有一个读函数 `load_skill_series`**（§82 行为指纹用，按 `score_version` 升序返回整条序列），
  它与本处不冲突：它读的是**序列**，不是「当前值」；
- 故 `load_profile` 的返回值变为 `Result<Option<ModelProfile>, PersistError>` **不变**，
  但**它现在可能因技能表里的表外取值而失败**（`PersistError`，与画像列的表外取值同一条通路）——
  这条要补进它的文档注释，并补一条用例：**画像列全合法、技能列有表外取值 → `load_profile` 返回具体 `Err`**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): 技能观测的落库与「当前值」"
```

---

### Task 9: `BudgetView`（§333 的只读投影）

**Files:**
- Create: `crates/continuum-model-registry/src/budget.rs`
- Modify: `crates/continuum-model-registry/src/lib.rs`
- Create: `crates/continuum-model-registry/tests/budget.rs`

**Interfaces:**
- Produces: `continuum_model_registry::BudgetView`

- [ ] **Step 1: 写用例**

- `the_view_carries_the_five_dimensions_of_333`：五个字段**逐项**给值并读回（`money` / `wall_time` / `token` /
  `gpu_time` / `network_transfer`），断言字段名与类型（`Option<i64>`）。
- `none_is_not_the_same_as_zero`：`None` = 该量纲当前不构成约束；`Some(0)` = 额度为零。
  两侧各一条断言，红的条件：把 `None` 折成 `0`（或反之）即红——这两件事混同会让「不约束」变成「一分钱都不能花」。
- `the_view_is_a_plain_data_projection`：字段是 `pub` 的（驱动直接构造，不经任何 trait、不经任何校验）。
  **不定义 trait 是刻意的**：一个没有实现者的 trait 是**假接口**（设计 §6.1）。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test budget
```

- [ ] **Step 3: 实现**

```rust
/// §333 Budget 的**只读投影**：该任务当前可用的剩余额度。五个字段**照录 §333**
/// （`docs/spec/05-normative.md:2502-2516`）。**单位与取值域规范未定义**——本层不解释、
/// 不换算、不记账，只把这个投影交给排序策略。`None` = 该量纲当前不构成约束。
pub struct BudgetView {
    pub money: Option<i64>,
    pub wall_time: Option<i64>,
    pub token: Option<i64>,
    pub gpu_time: Option<i64>,
    pub network_transfer: Option<i64>,
}
```

**投影由驱动做，不由语义层做**（设计 §6.1，已订正）：语义层产出它自己的 `Budget`（§333 的分配对象），
驱动把它投影成 `BudgetView` 传进来——驱动本来就同时依赖两侧，故这条投影不新增任何边，
`语义层 → 资源层` 这条反向边在任何时刻都不出现。**本层不为 `BudgetView` 建记账**（预扣 / 结算 /
父子分配 / Budget Validator 全归语义层，设计的 §6.2 表逐条列出），这句话写进文件头。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): §333 的只读预算视图"
```

---

### Task 10: 请求面（§250 第 1、5 项与「预算必填」）

**Files:**
- Create: `crates/continuum-model-registry/src/router.rs`
- Modify: `crates/continuum-model-registry/{src/lib.rs,src/error.rs}`
- Create: `crates/continuum-model-registry/tests/router.rs`
- Create: `crates/continuum-model-registry/tests/compile_fail/*.rs`（各配 `.stderr`）

**Interfaces:**
- Consumes: Task 2 的 `SkillDimension`、Task 9 的 `BudgetView`；**既有的** `continuum_core::model::{ModelId, ProviderHealth}`
- Produces: `continuum_model_registry::{TaskSkillRequirement, RequirementError, FamilyPreference, RoutingRequest}`

- [ ] **Step 1: 写用例**

- `an_empty_requirement_is_rejected`：`TaskSkillRequirement::try_new(vec![])` → `Err(RequirementError::RequirementEmpty)`，
  **断言是哪一枚**。理由写在用例注释里：`compatibility = matched / required` 在 `required` 为空时是 `0/0`，
  而 `CandidateScore.compatibility: Ratio` 拒 NaN 且 `evaluate` 不返回 `Result`——实现者只剩 panic 或编一个值两条路，
  **两条都能过其余全套用例**。故做成非空类型（设计 §5.1 末段）。
- `a_non_empty_requirement_keeps_its_dimensions_in_order`：顺序与内容逐一相同；`dimensions()` 给出切片。
- `the_five_family_preferences_are_recorded_verbatim`：`Auto` / `OpenAiPreferred` / `ClaudePreferred` /
  `LocalPreferred` / `Custom` **逐项**（§19 的封闭清单，`docs/spec/01-concepts.md:933-960`），
  并逐项钉落库编码（小写 `_` 连接；本阶段它不落库，但那份编码与类型同址）。
  `Custom` 在 §19 里不带载荷，本类型**也不给它加**。
- `a_request_carries_availability_as_values`：`Vec<(ModelId, ProviderHealth)>`；含 `Degraded` 与 `Unavailable`
  两种取值。**这两个取值怎么用不在本 task**：过滤与失误路径在 Task 11 的候选集构造里（设计 §5.3）。
  **用例注释里写明撞名**：这里的 `ProviderHealth::Degraded` 是**供应商侧的可用性**，
  与 `LifecycleState::Degraded`（§249 的模型生命周期异常态）是两个轴上的两个东西，只是名字撞了（设计 §4.2）。
- `tests/compile_fail/a_requirement_cannot_be_built_from_a_bare_vec.rs`：字段私有、无 `From<Vec<_>>`。
- `tests/compile_fail/a_routing_request_without_a_budget.rs`：漏掉 `budget` 字段的构造**编译不过**。
  这就是「预算必填」的照片形态——**签名层面，无运行期用例**（设计 §9 的对应行、§10 第 2 条）：
  语义层未建，`BudgetView` 的每个 `Some` 都只能由测试构造，「剩余额度算得对不对」在本阶段不可观察。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test router
```

- [ ] **Step 3: 实现**

```rust
/// 一次自动路由的请求。**纯数据，无 I/O**。
pub struct RoutingRequest {
    pub requirements: TaskSkillRequirement,            // §250 #1
    pub family: FamilyPreference,                      // §250 #5
    pub availability: Vec<(ModelId, ProviderHealth)>,  // §250 的「当前可用性」
    pub budget: BudgetView,                            // ENG-005：**必填，不是 Option**
}

/// **非空**的 §248 维度集合。字段私有，空集合被构造期拒绝。
/// **不带权重、不带阈值**——两者都是规范没有的数，加了就是发明（设计 §5.1）。
pub struct TaskSkillRequirement { dimensions: Vec<SkillDimension> }

impl TaskSkillRequirement {
    pub fn try_new(dims: Vec<SkillDimension>) -> Result<Self, RequirementError>;  // 空 → Err
    pub fn dimensions(&self) -> &[SkillDimension];
}
```

**请求与策略的分界是刻意的**：请求是纯数据（进得去表驱动用例），策略是可替换实现（可以换一份重跑同一组用例）。
`CostPolicy` / `LatencyPolicy` / `FailureHistory` 三个输入**不定义形状**，落在排序策略接口之后
（规范只给名字，且 `FailureHistory` 的产生方也没有指定，设计 §5.1）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): §250 的请求面与非空需求"
```

---

### Task 11: 输出面与 `rank`（全序、判重、无候选）

**Files:**
- Modify: `crates/continuum-model-registry/src/router.rs`
- Modify: `crates/continuum-model-registry/{src/error.rs,src/lib.rs}`
- Modify: `crates/continuum-model-registry/tests/router.rs`

**Interfaces:**
- Consumes: Task 4 的 `RoutableModel` / `RoutableState`、Task 10 的 `RoutingRequest`、Task 7 的 `load_profile`
- Produces: `continuum_model_registry::{rank, RankingPolicy, CandidateScore, RankedExecutionCandidates,
  ExecutionCandidate, RoutingReason}` 与 `RoutingError::{NoEligibleCandidate, DuplicateModelCandidate,
  UnknownAvailability}`

- [ ] **Step 1: 写测试夹具（这是本 task 的第一件事）**

`rank` 是纯函数，但它的输入 `RoutableModel` 只能由 `load_profile` 产出（`ModelProfile` crate 外不可构造，Task 3）。
**而这个链条不能自举，写成 `save_profile → load_profile` 是自指的**：`save_profile` 要一枚 `ModelProfile` 做入参，
画像又只能由 `load_profile` 读回——**第一枚画像必须由裸 SQL 播下**（`INSERT INTO model_registry` 一行、
`INSERT INTO model_profile` 一行），再走类型化接口。Task 7 的实现者在自己的夹具里遇到的是同一件事，处置相同。
夹具链因此是：**裸 SQL 播两行 → `load_profile` → `RoutableModel::try_new`**。
**夹具的这一步不写出来，本 task 的所有用例都无从下笔**——它不是测试技巧，是设计 §2.1 那条保证的直接后果。

**关于画像里的 `skill_vector`（原稿在此点名的缺口，已由协调者裁定，2026-10-06）**：`load_profile`
现在会调 `load_skill_vector` 把观测填进画像（`skill_vector` 这个字段的唯一来源是后者，`load_profile`
只是组合它，且不另查 `model_skill_score`；见 `## 遗留` 的同名条），
故**夹具在播下画像之后还要按用例需要播 0 / 1 / N 维观测**（`save_skill_observation`，Task 8）。
**夹具还必须为每个候选给出一条 `availability` 条目**（`Healthy` 或缺省的那一档）：`rank` 对
「列表里没有条目」的候选返回 `UnknownAvailability`，故漏给会以 `Err` 的形式而不是断言的形式失败，
报错位置还会指向被测函数。**这条不是可选项**，用 `unavailable` / `degraded` 两个具名参数让需要它的用例显式覆盖。

- [ ] **Step 2: 写用例**

- `the_result_is_ordered_and_selected_is_its_head`：三个候选、`compatibility` 递减，断言 `selected()` 是表头。
- `alternatives_is_the_tail_of_the_same_list`：`candidates()[1..] == alternatives()`——§84 的
  `alternatives` **不是第二份数据**，是同一份有序列表的表尾（设计 §5.2）。
- `every_candidate_accessor_has_a_photo`：五个访问器**逐项**——`model()`（给出的 id 正是输入那个模型的 id）、
  `state()`、`compatibility()`、`confidence()`、`reason()`。五个各一条，不抽代表。
- `the_reason_carries_family_matched_missing_and_notes`：四个字段逐项（`family()` / `matched()` / `missing()` /
  `notes()`）。`missing` 是 `Vec<SkillDimension>`，**不是一个数值**——本设计不把「缺一个维度」折算成任何扣分，
  因为那需要一个规范没有的权重（设计 §5.2）。
- `shuffling_the_input_does_not_change_the_output`：打乱输入顺序，输出**逐项**相同（全序且确定的照片）。
  红的条件：`compare` 的兜底一档不按 `ModelId` 升序即红。
- `tied_candidates_are_ordered_by_model_id`：两条 `compatibility` / `confidence` 全同、`ModelId` 不同 →
  输出按 `ModelId` 升序。红的条件：去掉兜底那一档即红——**但这条有个前提，实现者须实跑核**：
  Rust 的 `sort_by` 是**稳定**排序，故**夹具必须把两条同分候选按与 id 升序相反的顺序喂入**，
  否则去掉兜底档后输出照样是 id 升序、**假绿**（同一现象见 Task 8 的 `ORDER BY` 那条：
  「删掉某成分」型预测要先问**它有没有别的途径产生同样效果**）。
- `the_same_model_twice_is_rejected_with_the_id`：同一个 `ModelId` 的两个候选 →
  `Err(RoutingError::DuplicateModelCandidate { id })`，**断言是哪一枚、id 是哪一个**。
  它是 `compare` 的「全序」这条断言的守门人：两条 `ModelId` 相同的候选无从定序，兜底档也兜不住。
- `no_eligible_candidate_is_its_own_error`：三条——**空输入**、**全部被闸门挡下**（`RoutableModel` 构造失败，
  故候选集为空）、以及**唯一候选不可用**（被下面的可用性过滤清空）各得 `Err(RoutingError::NoEligibleCandidate)`。
  **`NoEligibleCandidate` 不合并进 `NotRoutable`**：前者是「没有可用的」，后者是「有一枚被点名挡下了」，
  调用方（§110 的流程）对两者的处置不同。
- **「当前可用性」：过滤，且只过滤 `Unavailable`**（设计 §5.3 写死，三条照片）：

  - `an_unavailable_model_is_not_a_candidate`：两个候选，其一 `ProviderHealth::Unavailable` →
    输出里**没有它**，`selected()` 是另一个。红的条件：去掉这一档过滤即红。
    **两侧对钉**：把同一个候选由 `Unavailable` 改回 `Healthy` → 它**回到**输出里。
  - `healthy_and_degraded_both_stay_and_the_order_does_not_change`：两条候选，其一的 `ProviderHealth`
    在 `Healthy` 与 `Degraded` 之间来回改 → **两次输出逐项相同**。
    **这一条是「不发明降权判据」的照片**：设计写死只过滤 `Unavailable`，`Degraded` 该不该降权
    **规范未给判据**（§250 只说「考虑」、§84 没有给这一维的算法），故基线不为它改排序；
    它原样带进 `reason`（设计 §5.3、§11 第 24 条）。
  - `a_candidate_missing_from_availability_is_rejected`：候选集里的某个 `ModelId` 在
    `RoutingRequest::availability` 里**没有条目** → `Err(RoutingError::UnknownAvailability { id })`，
    **断言是哪一枚、id 是哪一个**。**不当作可用**——按未知放行是 fail-open 的形状（设计 §5.3）。
    红的条件：把「列表里没有」当成「可用」而放行即红。
- `a_routable_model_does_come_out_as_the_selected_candidate`：**与闸门那一侧配对的正面照片**——
  一个 `Active` 的模型经 `rank` **确实成为 `selected()`**。只钉「`stale` 被挡」而不钉这一侧，
  整条路由路径可以在「永远返回 `NoEligibleCandidate`」的情况下全绿，而那正是 fail-open 的反面。
- `tests/compile_fail/ranked_candidates_cannot_be_built.rs`：`RankedExecutionCandidates` 字段私有、
  唯一构造点在 `rank` 内（构造前先判空，故 `selected()` 不必返回 `Option`）。

- [ ] **Step 3: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test router
```

- [ ] **Step 4: 实现**

```rust
/// 排序的机制与接口在本层；**具体打分函数的数值明确推迟**（设计 §5.3 的三条依据）。
pub trait RankingPolicy {
    fn evaluate(&self, request: &RoutingRequest, model: &RoutableModel) -> CandidateScore;
    /// 全序比较：`Ordering::Less` 表示 `a` 排在 `b` 前面。
    /// 缺省实现按 (compatibility, confidence) 降序，再按 family、model id 兜底。
    /// 操作数就是 `ExecutionCandidate`——**不另立一个 `Scored` 类型**（同一件事的第二个落点）。
    fn compare(&self, a: &ExecutionCandidate, b: &ExecutionCandidate) -> Ordering { /* 缺省 */ }
}

/// 一个候选的打分。**没有总分字段**——§248 的禁令在接口上也看得见。
pub struct CandidateScore { pub compatibility: Ratio, pub confidence: Ratio, pub reason: RoutingReason }

/// §250 的输出。字段私有：无候选时不返回空列表，而是 `Err(NoEligibleCandidate)`。
pub struct RankedExecutionCandidates { candidates: Vec<ExecutionCandidate> }
impl RankedExecutionCandidates {
    pub fn selected(&self) -> &ExecutionCandidate;       // §84 的 selected_model
    pub fn alternatives(&self) -> &[ExecutionCandidate]; // §84 的 alternatives（表尾）
    pub fn candidates(&self) -> &[ExecutionCandidate];
}

pub struct ExecutionCandidate { model: ModelId, state: RoutableState,
                                compatibility: Ratio, confidence: Ratio, reason: RoutingReason }

/// §84 的 reason。结构化而非一行文本：字段要供后续对账取用。
pub struct RoutingReason { family: FamilyRelation, matched: Vec<SkillDimension>,
                           missing: Vec<SkillDimension>, notes: Vec<String> }

/// 骨架：**建候选集**（判重 → 按 `request.availability` 查可用性 → 只滤掉 `Unavailable`）
/// → 逐个 `evaluate` → 装成 `ExecutionCandidate` → `sort_by(policy.compare)` → 取头。
pub fn rank(request: &RoutingRequest, models: &[RoutableModel], policy: &dyn RankingPolicy)
    -> Result<RankedExecutionCandidates, RoutingError>;
```

**建候选集这一步里有三件事，逐条写死**（设计 §5.3）：

- **`ProviderHealth::Unavailable` 的模型不进候选集**。这不是发明阈值——§250 的输出是
  `RankedExecutionCandidates`，即**可执行的**候选；一个供应商侧已不可用的模型不是执行候选，
  把它排进去，「拿它跑」那一步必然失败。**故这一条是「不可用即不是候选」，不是「可用性低就降权」。**
- **`Healthy` 与 `Degraded` 都进候选集，两者之间没有判据**：`Degraded` 该不该降权、降到什么程度，
  规范未给判据，故本设计不发明——健康度原样带进 `reason`，让策略自己决定（§11 第 24 条）。
- **候选在 `availability` 里没有条目**（含候选集里有、列表里无）→
  `Err(RoutingError::UnknownAvailability { id })`，**不当作可用**：按未知放行是 fail-open 的形状。

**三条 `Err` 的判定次序设计未定，本计划取「判重 → 可用性 → 空判定」（`NotRoutable` 由
`RoutableModel::try_new` 产出，到不了 `rank`）**，且所有用例都只让一个条件成立——不构造两条 `Err`
同时可能的输入，故实现换一个次序也不会让哪条用例变绿或变红。这是刻意的：次序没有判据，用例就不该依赖它。

**「全序」须逐条落实**（实现时写进注释，理由是这条断言由三件事合起来成立）：`Ratio` 拒 NaN 保证前两档可比；
`ModelId` 升序保证兜底档是**全序的最后兜底**（两两可比且无相等）；若两条候选连 `ModelId` 都相同，
它们本就是同一个模型的两次打分——该情形由建候选集时判重排除。

**`profile.rs` 里那句「NaN 会让 `sort_by` 不是全序」的绝对措辞，照片归本 task，且要写成「为什么没有」**：
那句理由是对的，但**它在 `Ratio` 上拍不到**——NaN 在构造期就被拒，**进不到 `compare` 的入参里**，
故本处**不为它造用例**（造不出：候选的 `compatibility` / `confidence` 都是已构造的 `Ratio`）。
它真正的落点是**构造期**：Task 1 的 `ratio_rejects_non_finite_and_out_of_range` 就是它的照片——
`compare` 的前两档因此**可以假定**操作数是合法 `Ratio`。**这句话要写进 `compare` 的文档注释**，
否则后来者会以为这里少了一条排序用例。

`ExecutionCandidate` 的五个访问器**是接口冻结处的必需品**：`selected()` 要交给消费者，而消费者下一步
就是拿着 `model` 去（在它自己那一层）调 provider（**子项目 G**）。`RoutingReason` 的四个访问器
**今天没有具名消费方**，只是 §84 要输出的内容的读口——**这句话刻意写得比上面弱**（设计 §5.2）。

`RoutingError` 本 task 增补 `NoEligibleCandidate`、`DuplicateModelCandidate { id }` 与
`UnknownAvailability { id }`，**共四枚，到此为止**。**只有 `rank` 真的会产出的变体才收**——
设计 §5.4 明写三个「没有产生方」的一律不收：**`Persist` 已删**（`rank` 是纯函数、签名里根本没有 `Tx`，
故它产不出读库失败；留着它会让人以为本层会写库，且它是一个死物）；`Requirement(#[from] RequirementError)`
不收（空需求在 `try_new` 就被拒，`RoutingRequest` 装的是**已构造的** `TaskSkillRequirement`，到不了 `rank`）；
`ProfileBeforeVerified` 不收（它是 `LifecycleError` 的变体，产生方是 `save_profile`）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): §250 的输出面与 rank 的全序"
```

---

### Task 12: `BaselineRankingPolicy`（只用具名输入）

**Files:**
- Modify: `crates/continuum-model-registry/src/router.rs`
- Modify: `crates/continuum-model-registry/tests/router.rs`

**Interfaces:**
- Consumes: Task 11 的 `RankingPolicy` / `rank`
- Produces: `continuum_model_registry::BaselineRankingPolicy`

**夹具**：沿用 Task 11 的 `tests/router.rs` 夹具（含「**第一枚画像由裸 SQL 播下**」那一步）。
本 task 多一项前置，**已由协调者裁掉（2026-10-06，取 A，见 `## 遗留` 的同名条）**：`load_profile`
现在会调 `load_skill_vector` 把观测填进画像，故**夹具只要在播下画像之后再播几行观测**
（`save_skill_observation`，Task 8），经 `load_profile` 造出的候选就带上了「某维有观测」——
**下面那两条兼容度用例现在写得出来了**，不必等任何裁决。夹具要多做的一步是：
**按用例需要，为候选播 0 / 1 / N 维观测**（`the_baseline_counts_…` 要 1 维、`adding_an_observation_changes_the_order` 要两侧对比）。

- [ ] **Step 1: 写用例**

- `the_baseline_counts_the_dimensions_that_have_an_observation`：`compatibility = matched / required`，
  只读「有没有观测」；分数本身不参与。
- `changing_a_score_value_does_not_change_the_order`：把某维的 `SkillScore` **数值**改掉（保持有无不变），
  排序**不变**。红的条件：改用 `score` 的数值加权即红——这正是 §2.4「只用序、不用量」的落点，
  也是本层唯一能拍的「不依赖单一总分」的行为面照片。**这条的前提要由夹具保证、并由实现者实跑核**：
  **被改写的那几个 `SkillScore` 数值，必须使「按数值加权」算出的次序与「按有无」算出的次序相反**，
  否则加权版会给出同一个次序、变异不红（同 Task 8 那两处的形态）。
- `adding_an_observation_changes_the_order`：改「有无」（多一维有观测）→ 排序**变**。
  **与上一条两侧对钉**：只有上一条时，一个「什么都不读、永远返回同一个顺序」的策略照样全绿。
- `same_family_candidates_rank_before_cross_family_ones`：§19 的家族偏好排在数值之前；`Auto` 时全部视为
  `SameFamily`。两侧都钉（给 `OpenAiPreferred` 时同族优先；给 `Auto` 时不产生跨族差别）。
- `the_state_is_visible_in_the_reason`：`Degraded` 的候选仍在表里、`state()` 读出 `Degraded`，
  `reason` 里看得到——**可见但不禁**（§4.2 末段，让策略可以据此降权）。
- `the_baseline_does_not_read_the_budget`：两次请求只差 `BudgetView` → 排序不变。
  **用例名与注释写明这是「已写明未实现」而不是「忘了读」**：§250 的 MUST 考虑成本在本设计里是
  **结构性地不可省略**（`budget` 是必填参数，忽略它是一次看得见的选择），不是已实现——
  §333 的五个量纲没有单位，量值算不出来（设计 §5.3 末段、§10 第 2 条）。
**「当前可用性」在本 task 里没有用例，这是对的**：它落在 **`rank` 的候选集构造**（Task 11 的三条照片），
不是策略的输入——`RankingPolicy::evaluate` 收到的是已过可用性过滤的候选，
故一个策略想「按可用性排序」也无从下手。**基线不读它**（§5.3 列的基线输入里没有它），
`Degraded` 与 `Healthy` 的排序不变就是 Task 11 那条照片立的事实。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test router
```

- [ ] **Step 3: 实现**

```rust
/// 只用**已定义**的输入：§248 维度的**有无**（不读分数）、§247 的 confidence、
/// §19 的家族偏好、§249 的状态（设计 §5.3）。
pub struct BaselineRankingPolicy;

/// §19 的两族关系，落在候选的 `reason` 里。
pub enum FamilyRelation { SameFamily, CrossFamily }
```

**基线是具名的、可替换的，不是对规范的声称。** 被否掉的两个候选打分法写进文档注释（不是可选说明）：
**(a) 分数加权求和**——需要一个规范没有的尺度与方向；**(b) 阈值匹配**——需要一个规范没有的阈值。
两者都撞在 §2.4 的同一条判据上。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): 具名基线策略"
```

---

### Task 13: 升级阶梯的数据形状（§251 §86）

**Files:**
- Create: `crates/continuum-model-registry/src/escalation.rs`
- Modify: `crates/continuum-model-registry/src/lib.rs`
- Create: `crates/continuum-model-registry/tests/escalation.rs`
- Create: `crates/continuum-model-registry/tests/compile_fail/tier_one_low_is_not_a_step.rs`（配 `.stderr`）

**Interfaces:**
- Produces: `continuum_model_registry::{EscalationStep, EscalationLadder, next_step}`

- [ ] **Step 1: 写用例**

- `the_ladder_has_exactly_the_five_steps_of_251`：`Tier1` / `Tier2` / `Tier2High` / `CrossFamily` / `Specialized`
  **逐档**断言，并逐档钉落库编码（小写 `_` 连接）。
- `the_ladder_is_ordered_as_251_states`：顺序断言（不是集合断言）——`Tier1 → Tier2 → Tier2High → CrossFamily → Specialized`。
- `next_step_walks_the_ladder_in_order`：逐档一条，给出下一档；末档 → `None`。**逐项有照片**，不抽代表。
- `tier_one_low_is_not_a_step`（trybuild）：`EscalationStep::Tier1Low` **写不出来**。
  这是 §11 第 21 条的落点：§86 的降级例写「Tier 1 Low」，而 §251 的阶梯五档里没有它——
  「Low」是 §18 的**推理强度**取值，不是 §17 的 Tier。本设计不为它加第六个成员
  （加一个不在阶梯里的成员会让「照录 §251」这句话变假）。

- [ ] **Step 2: 运行，确认失败**

```bash
cargo test -p continuum-model-registry --test escalation
```

- [ ] **Step 3: 实现**

```rust
/// §251 的阶梯五档，**逐档照录**（`docs/spec/05-normative.md:922-944`）。
/// `Tier2High` 是 §251 的原词：它对应的是「Tier2 ＋ 高推理强度」这个**组合**，不是第六档能力。
/// **不另建** `Tier` / `ReasoningEffort` 两个枚举：本层没有消费者需要按轴拆分，拆了就是给
/// 两个没有产生方的类型建形状（「不预先发明」，设计 §7.1）。
pub enum EscalationStep { Tier1, Tier2, Tier2High, CrossFamily, Specialized }

/// §251 的阶梯（照录其**顺序**）。
pub struct EscalationLadder { steps: Vec<EscalationStep> }

/// 「下一档」。**纯函数**——本层建阶梯的**有序步骤**，触发不在本层。
pub fn next_step(current: EscalationStep) -> Option<EscalationStep>;
```

**本层明确不建**（写进文件头，逐条给理由，免得被读成漏项）：失败检测与升级触发（要读一次执行的失败，
是**子项目 G** 的活，不是 F——F 是工具调用路径）；§251 的前提检查「仍满足 Contract 和 Budget」（按 §110 归
Budget Validator，语义层）；§86 的降级调度（「400 个机械文件检查降回 Tier 1 Low」是**任务级**的模型再选择，
即用一个新的 `TaskSkillRequirement` **再调一次 `rank`**——本设计不为它建任何新机构）。

**它的消费者本阶段不存在**，故 `next_step` 的照片只能是对纯函数的直接调用，钉不了「驱动真的在失败后升级了」
（设计 §10 第 3 条）。这一点写在测试文件的注释里。

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace --no-fail-fast
git add crates/continuum-model-registry
git commit -m "feat(model-registry): §251 阶梯的数据形状"
```

---

### Task 14: 收尾与复核

**Files:**
- Modify: `crates/continuum-model-registry/**`（仅在复核发现缺口时）

- [ ] **Step 1: 全量验证**

```bash
timeout 1500 cargo test --workspace --no-fail-fast
timeout 900 cargo build --workspace --all-targets
```

预期：全绿、0 warning。

- [ ] **Step 2: 复核「本层是判断而不是执行」**

```bash
grep -rn "invoke\|stream\|ModelProvider\|continuum_provider\|continuum-provider" crates/continuum-model-registry
grep -rn "continuum_capability::\|Capability\|AuthorizedTool" crates/continuum-model-registry
```

第一条**预期零命中**（注释里也不该出现 `ModelProvider` 的调用面）；第二条**只允许出现 `Cost` / `Latency`**。
任何一条不符即据实报告，不自行改设计。

**这两条是「按词根取候选、再人工读」的判据，不是「命中数即结论」的判据**：grep 按**行**匹配，
故**跨行折行会漏**（`ModelProvider::` 与 `invoke` 被折到两行时，第一条的零命中是假的）。
**故零命中不是充分证据**——本轮判据的实际做法是：grep 出候选行后，**对 `src/` 的调用面逐处通读**
（本 crate 只有 `profile / lifecycle / persist / router / budget / escalation / error` 七个文件，通读代价很小），
**结论以通读为准、grep 只用来定位**。同一通病见 Task 7 的连带面判据。

- [ ] **Step 3: 逐条核对完成判据**

| 判据 | 证据 |
|---|---|
| §4.4「Router 输出带 confidence 的候选排序」 | Task 11 的五个访问器用例（`confidence()` 在其列） |
| §4.4「不依赖单一总分」 | Task 3 的 `overall_score` trybuild + Task 5 的列清单 + Task 11 的 `CandidateScore` 无总分子段 |
| §4.4「未完成画像的模型不会进入自动路由」 | Task 4 的十态闸门 + Task 7 的 `ProfileBeforeVerified`（两件事的合取） |
| §249 三态 + `stale` 不入自动路由 | Task 4 的 `only_the_six_routable_states_pass_the_gate` 与 `a_stale_model_is_not_routable_while_an_active_one_is` |
| §250「当前可用性」 | Task 11 的三条照片（`Unavailable` 被排除、`Healthy`/`Degraded` 都在且排序不变、缺条目得 `UnknownAvailability`） |
| 三张表与迁移计数 | Task 5 的全量跑（跨 crate 才可见） |

**若某条找不到对应证据，不得标注为覆盖**，据实报告缺口。

- [ ] **Step 4: 残余落到有版本的文档**

`## 遗留` 一节在本计划内（`docs/superpowers/plans/` 是有版本的位置）。**本计划不新建、也不修改第二份文档**：
若要并进 `docs/superpowers/p3bcdf-followups.md`，由**协调者**做。

- [ ] **Step 5: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "docs(model-registry): P3 子项目 D 的收尾与复核"
```

---

## 遗留

**设计定稿时已闭的七条**（本计划初稿曾把它们报为未决，来历留此，免得后来者按旧报告去找）：

- `RoutingError::Persist` —— 设计 §5.4 已裁**删**该变体（零产生方的死物，且会让人以为本层会写库）。
- 迁移 `81` —— 设计 §3.3 已删「预留 `81`」（三张表全在 80，只取一个）。
- `transition` 的返回值 —— 设计 §3.2 已写明返回**迁移前的旧态**。
- `ProfileError` 的名字与三枚变体 —— 设计 §2.4 已给出（`NotFinite` / `OutOfRange` / `BadTimeRange`）。
- `RoutableModel::try_new` 的入参 —— 设计 §2.1 已改收 `LifecycleState` 并留了来历。
- `EscalationStep` 的落点 —— 设计 §8.1 已列入 `src/escalation.rs`（七个模块）。
- **C↔D 接缝（设计 §11 第 13 条）—— 已闭，且它那条「未闭」系误读**（据接缝分析 §五.8）：
  设计把它记为「未闭（收件人：C 的设计）」，要求 C 订正 §4「调用面」与关于 `ALLOWED`「只应含
  `continuum-provider`」的那句。**判据两条**：(a) **C 的 §6 调用面早已订正**——C 现文写「Router 不调用
  适配器，调用方是子项目 G」；(b) D 引的 `C:274` 实为 C §4.2 形态 3
  （**现 C 设计 `:314-315`**，即「而它的 `ALLOWED` 条目只应含 `continuum-provider`」那一句），讲的是**未来的
  `continuum-adapter-*` crate 的 `ALLOWED`**「只应含 `continuum-provider`」——**不是 C 自己的条目**
  （C 自己的 provider 条目已是三元素）。
  **故本计划不给它写 task，实现时也不得去「修」C 设计 `:314-315`**——那会改掉一句正确的、
  关于未来适配器 crate 的约束。
  （条目标题里的「接缝分析 §五.8」是那次只读复核的编号；**它的判据已逐条抄在上面两句里**，
  故本条不依赖那份不随仓库版本走的文件。）

```
effect_class 两轴之问   D 退件（设计 §11 第 22 条）：本 crate 对 `effect_class` 的引用为零，
                      问句涉及的两侧（Tool Registry 与 Effect Journal）都在工具调用路径上。
                      **规范未给判据**。收件人：子项目 F ＋ 规范维护者（本项目无此角色）。
trust                   D 退件（设计 §11 第 23 条）：§247 的十二个字段里没有 trust，§250 的
                      八个 MUST 考虑项里也没有，路由侧没有消费点。**规范未给判据**。
                      收件人：子项目 F ＋ 规范维护者。
复用 Cost / Latency      **已落**（Task 3：本层取 `continuum_capability` 的两个**类型**，不取能力凭据）。
的一对类型              残余是设计 §11 第 6 条的那半句：若后续判定模型的 `cost_profile` 与工具的 `cost`
                      **不是同一轴**，此处要拆、并把 `Cost` / `Latency` 上移到 `continuum-core`
                      （那会动 P3A 已落地的类型与它的落库编码，超出本子项目范围）。
                      **规范未给「是不是同一轴」的判据**。收件人：协调者。
ProviderError →          D **有条件签收**：签收的是**已分类的结果**（`FailureHistory`，§250 的第六个输入，
FailureClass 的映射     落在策略一侧、今天形状未定）；**映射本身退件**——`ProviderError` 是
（§11 第 18 条）        `continuum-provider` 的类型，而本设计对那个 crate 的引用为零，搬进来就得登记
                      一条被裁掉的边。分类发生在**持有 `ProviderError` 的一侧＝子项目 G**。
                      **`max_attempts >= 2` 与退避参数也不是本层的**：按 §246 它们属
                      `ExecutionProfile.retry_policy`（归执行层），本层的 `EscalationLadder` 是
                      「换哪个模型」，与「重试几次」是两个轴——本计划**不接**
                      `docs/superpowers/p1-followups.md:62-64` 那句「P3 的 Router 必须…」里的
                      「Router」二字，这是一处与本仓既有 followups 的措辞分歧，明写在此。
                      **收件人：C 的设计（改指名真正的分类方）＋ 协调者。**
工具侧 cost / latency   其 `Some` 的「可达路径」在 B/C/D/F 四份设计里无人认领（设计 §11 第 15 条）：
的 Some                 D 复用的是两个**类型**（落在模型画像上），不读 `ToolProfile`；
                      而 §4.1 里**没有「工具选择」这个组件**。**规范未给判据**。
                      收件人：协调者（在四份之间指派）＋ 规范维护者。
#[allow(dead_code)]     **已闭（Task 7）**：两处豁免都已从源码里删掉，上面的判据照办——Task 7 的
的两处豁免             `save_profile` 要用满十二个字段，故五个字段各补了**真访问器**，`try_new`
（已闭）               也由 `load_profile` 在普通构建下调用；两处都不再需要豁免，**没有留下
                      「这个字段真的没人读」这类待复核项**。下面为原稿（照留，作这条决定的来历）：
                      **本仓首次出现这个豁免，共两处，都在 `crates/continuum-model-registry/src/profile.rs`**：
                      (a) `ModelProfile::try_new` 在非测试构建下无调用方（`load_profile` 要等 Task 7）；
                      (b) `ModelProfile` 的五个字段无访问器（读数方是 Router，在 Task 11 及之后）。
                      **接受的理由**：0 warning 是硬约束，而替代方案「提前铺五个访问器」＝**建了没人用的 API**，
                      那比这条豁免更坏。**但它会永久掩盖「这个字段真的没人读」**——故：
                      **Task 11 与其后的 router task 把读数接上之后，这两处豁免必须复核并从源码里去掉**；
                      若到那时仍有字段无人读，那是一个**要处置的发现**（删字段或写明为什么留），不是可以继续压着的事。
                      **收件人：Task 11 及之后接上读数的那些 task 的实现者。**
LifecycleError::Persist  **已闭（commit `531c548`）——原稿记的是「有产生方、无照片」**，订正如下，
的处置（已闭）            **下面那段叙述留作这条决定的前史**（本仓惯例：原话照留、加订正注记）。
                      **判据与出处**：Task 6 的修正提交 `531c548` 新增了
                      `a_persist_failure_keeps_the_inner_persist_error`
                      （`crates/continuum-model-registry/tests/persist.rs:644`）——断言外层
                      `LifecycleError::Persist` **与**内层 `PersistError::Database` 两层，并点出
                      `no such table: model_registry`；自证方式是变异 M10「吞掉 `?` 的升格」**只红这一条**、
                      红位在 `:659`。故那枚变体现有照片，**本条目不再是未决项**。
                      以下为原稿（照留，只把「本轮未补」那半句订正为已补）：位置是
                      `crates/continuum-model-registry/src/error.rs` 的
                      `LifecycleError::Persist(#[from] PersistError)`；原稿记「正常路径不产生它、
                      造一次**真实的写失败**超出该 task 的夹具能力」——**该判断已被 `531c548` 证伪**：
                      造法就是下面这三行，Task 6 的修正者照它补上了照片。
                      **若要让它有照片：本仓已有可直接照抄的造法，代价是三行**——
                      `crates/continuum-workspace/src/gate.rs:2514-2536` 的
                      `a_failed_audit_write_reports_persist_and_leaves_the_change_in_the_base`：
                      **建一个不带迁移的库**（`Db::open_with(&path, Vec::new())`），目标表因此不存在，
                      写入必然抛真实的数据库错误（比伪造一个失败的 `Tx` 省事，也仍是真实的库错误），
                      再断言 `matches!(err, LifecycleError::Persist(_))`。
                      本仓对这类 `#[from]` 转发变体的两种处置都有先例（`continuum-graph` 的
                      `ApplyError::Persist` **无**照片、`continuum-workspace` 的 `GateError::Persist` **有**），
                      故这不是「仓内已一致的做法」，而是一个**待定的选择**。
                      **本轮已补**（原稿写「本轮未补、记此待补」——**那句已随 `531c548` 作废**，
                      来历留在上面：它曾是「计划记了一件事、实现侧随后做了却没回来改计划」的那一类，
                      判据是「凡是『本轮未补』记进遗留的，补上之后必须回来改那一条」——
                      否则它就成了**已闭记成未闭**，后来者会照它去做一件已经做过的事）。
load_profile 的           **已裁（协调者，2026-10-06）：取 A——`load_profile` 也把 `skill_vector` 填上**，
skill_vector 与 rank      **由 Task 8 交付 `load_skill_vector` 时一并让 `load_profile` 调它来填**。
的输入面                  **判据三条**：(1) 设计 §247 说 `ModelProfile` 有十二个字段（§2.1 的表里
                          `skill_vector` 是其中之一）——**存储分成两张表是存储的事**，不该让对象少一个字段；
                          (2) **`rank` 的签名不变**（仍收 `&[RoutableModel]`），故**不必动 `RoutableModel` 的形状**，
                          改动面最小；(3) **`skill_vector` 这个字段只有一个来源**（`load_skill_vector`），`load_profile` 只是
                          **组合**它——若 `load_profile` 自己再查一遍 `model_skill_score`，就成了同一数据的
                          两个来源，而这正是本设计掐别的重复产生点时的同一判据。
                          **口径订正（2026-10-06）**：本条初版写的是「`load_skill_vector` 是 `model_skill_score`
                          **表的唯一装载者**」——**那是关于整张表的断言，而它是假的**（同文件的
                          `load_skill_series` 也读那张表）。**关于整张表的断言 ≠ 关于一个字段的断言**：
                          前者要穷举该表的所有读函数，后者只要钉住一条路径。原话与订正一并留此。
                          **原稿把它记为「缺口，未闭」，三条候选出路照留作前史**：
                          (i) `load_profile` 一并装载（**取的就是它**；当时担心的「多一个装载点」由判据 (3) 化解：
                          来源仍是 `load_skill_vector`，这里是组合）；
                          (ii) `RoutableModel` 改装 `(ModelProfile, SkillVector)`——**未取**，判据 (2)：动形状没有必要；
                          (iii) 由调用方（G）在构造候选集前合并——**未取**，它需要一条新的 crate 内入口。
                          **遗留里不再有未决项**；Task 8 与 Task 11/12 的落点见各自正文。
Degraded 的降权判据      §250 的八项 MUST 考虑里，可用性**只写死了「过滤 Unavailable」这一档**
                      （设计 §5.3）：`Healthy` 与 `Degraded` 之间**没有判据**——§250 只说「考虑」、
                      §84 没给这一维的算法。本设计不发明，`Degraded` 原样带进 `reason`。
                      **收件人：规范维护者。**
§4.1 自环的两句相抵      「`* → quarantined` / `* → disabled` 可由**任意**状态进入」与「自环（`x → x`）非法」
                      不能同真。**已裁：取自环可 `Ok` 的那一侧**（目标是这两态的自环合法，其余自环
                      是未列出的对，`Illegal`）——裁决与判据见 Task 4 的执行期裁定。
                      **设计那处由设计作者改，本计划不改设计。** 收件人：设计作者 ＋ 协调者。
transition_in_tx 的取名  落库版迁移函数在本计划里叫 `transition_in_tx`，设计 §3.2 叫 `transition`。
                      改名理由：内存版 `lifecycle::transition` 已在 crate 根导出，两个同名函数
                      在同一个导出面里冲突。**属计划自定，记此申报**；若复审要把两处收敛成一个名字，
                      须同时决定导出面的形状（根上只导内存版、落库版经 `persist::` 调用）。
save_profile 的返回类型   **设计自相抵**：§3.2 把 `save_profile` 记成 `-> Result<(), PersistError>`，
                      §4.1 / §4.3 却要求它返回 `Err(LifecycleError::ProfileBeforeVerified { state })`
                      ——`PersistError` 里没有装得下那枚变体的臂（它是 `LifecycleError` 的成员）。
                      **Task 7 的落地**：取 §4.1 / §4.3 那一侧，签名是
                      `Result<(), LifecycleError>`（落库失败经 `#[from]` 升格成 `LifecycleError::Persist`，
                      故没有丢信息）。本计划的 Task 7 Step 3 代码块会给成 `-> Result<(), PersistError>`，
                      **那处是照抄设计 §3.2、已按本条订正为 `LifecycleError`**。
                      **收件人：设计作者（§3.2 的签名要改）＋ 复审者。**
register_model 的落态     设计 §3.2 的注释写「§21 发现即登记」，§4.3 的表却把「`discovered → unprofiled`」
                      的触发方记为登记方（`register_model`）。本计划按 §3.2 取「登记后落在 `discovered`」，
                      两处的措辞差别记此，**不自行挑一边改设计**。收件人：设计作者 ＋ 复审者。
§2.4 的 9.2 出处         设计 §2.4 把 `SkillScore` 的示例值 `9.2` 记在 `docs/spec/01-concepts.md:1068-1104`
                      （那里是 §23 的技能树，全文无 `9.2`）；唯一出处实为 `docs/spec/02-positioning.md:773`
                      （§83 Model Failure Modes）。本计划已改用正确出处；
                      **设计那处已由协调者派回设计作者**。
规范级未决的其余各项     设计 §11 的第 2、3、4、5、7、8、9、10、11、12、16、17、19、21、24 条
                      （第 6、13、18 条已分别单列于上，不在此列）
                      （探索、Population Feedback、§84 的语义、§333 的单位、三处画像清单不一致、
                      子维度分层、failure_modes 词表、§19 阈值、§249 迁移关系、cost_budget 收紧、
                      初步画像、version vs time_range、上下文长度、Tier 1 Low、Degraded 降权）。
                      收件人多为规范维护者；本计划**一条都不发明**。
```
