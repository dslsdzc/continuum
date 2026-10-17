# P6D（Runtime Learning 与 Protected Core）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development（推荐）或
> superpowers:executing-plans。步骤用 `- [ ]` 复选框跟踪。

> **任务单**（`docs/superpowers/specs/2026-10-08-p6-scope-and-split.md` §五 P6D 行，逐字）：
> 建 §321／§322 那条「Learning 不能自行取消规则」的边界，并把 P0 已定的 crate 方向在本阶段加固
> （不新增反向边）。

**Goal:** 新建两枚 crate——`continuum-protected`（§322 的 `ProtectedCore` 五字段、五组承诺、§321 五类语义、
封存入口）与 `continuum-learning`（§320／§216 的六类学习目标、五类观察、记录与提案）；让学习产物在
**类型、crate、值**三个面上都指不到 §321 的五类语义；并在 `dependency_direction.rs` 上加一条**并集复核**，
把「合并后的 `ALLOWED` 并集」变成可执行的门。本块**不建**学习算法、执行器、提案应用、持久化、迁移、
事件与审计（设计 §1.2、§9）。

**Architecture:** 两枚 crate 是**上下游**，不是同义拆分：

- `continuum-protected`：workspace 内依赖**零条**（`ALLOWED` 行 `("continuum-protected", &[])`，硬断言）。
  装 `ProtectedCore` 五个字段（`authority_rules` / `capability_rules` / `contract_rules` / `audit_rules` /
  `effect_rules`，字段名、次序与 §322 逐字相同、字段私有）、`RuleCommitment`（`Unavailable` 或
  `Committed { version, digest }`）、`RuleVersion`、`RuleDigest`、`CoreFingerprint`、`ProtectedSemantic`、
  `ProtectedError`，唯一构造点 `seal(ProtectedCoreParts)`。核心**没有** `&mut` 方法、setter、`Default`、
  `Deref` / `DerefMut`；`fingerprint()` 按五个字段现算，不是第六个字段。
- `continuum-learning`：只依赖 `continuum-protected` 一条 workspace 边。装 `LearningTarget`（六类）、
  `ObservationKind`（五类）、`LearningRecord`（观察记录，不是产物）、`LearningProposal`（产物，唯一构造点
  收 `&ProtectedCore`，字段私有，绑定 `CoreFingerprint`）。没有错误类型，不派生 serde。
- **并集复核**：`crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` 装 **27 行** P6 前基线
  （逐行等于 `03e79c7` 的 `ALLOWED`）；`P6_EXPECTED_ROWS` 装 P6 的 **5 行**（P6A 四项、P6B 零项、
  P6C 一项、P6D 两项）；`dependency_direction.rs` 末尾的 `p6_union_adds_only_its_own_rows` 用**八条断言**
  查整张表。它是 **P6 合并门**，P7 重新基线化或退役（设计 §7.3.5）。

**Tech Stack:** Rust 1.95.0 / edition 2024；`sha2`（`RuleDigest` / `CoreFingerprint` 的 SHA-256）；
`thiserror`（`ProtectedError` 的 `Display` / `Error`）；dev：`trybuild`（C1–C5）。
**不引入** serde、`once_cell`、`parking_lot` 或任何第二枚错误类型；`sha2` / `thiserror` 都是
`[workspace.dependencies]` 已有的外部 crate，**不进 `ALLOWED`**。

**Spec:** `docs/superpowers/specs/2026-10-17-p6d-runtime-learning-and-protected-core-design.md`
（**唯一事实来源**，1382 行，工作树 HEAD `03e79c7` 的实测版）。
切片与冻结口径：`docs/superpowers/specs/2026-10-08-p6-scope-and-split.md` §四第 1 条、§五 P6D 行。
**本计划不改设计、不改规范、不改切分文档。**

---

## Global Constraints

- **§322 的五组，逐字**：`authority_rules`、`capability_rules`、`contract_rules`、`audit_rules`、
  `effect_rules`。`ProtectedCore` 的形状逐字：

  ```
  ProtectedCore {
      authority_rules
      capability_rules
      contract_rules
      audit_rules
      effect_rules
  }
  ```

  Rust 侧只变元素类型（`RuleCommitment`），**字段名、次序、五个——一个字不改**（设计 §4.1）。
- **§321 的五类语义，逐字**（`ProtectedSemantic` 的 `as_str` 返回值，不得改写大小写或空格）：
  `Authority semantics`、`Contract semantics`、`Permission invariants`、
  `User approval requirements`、`Core security policy`（设计 §4.4）。
- **`ProtectedCore` 的五个字段私有**；五个访问器同名返回 `&RuleCommitment`；唯一构造点是
  `seal(ProtectedCoreParts)`。**没有** `&mut` 方法、setter、`Default`、`Deref`、`DerefMut`（设计 §4.1、§3.6）。
  **源码注释里不要写出 `&mut self`、`impl Deref`、`impl DerefMut` 这些字面串**——S1 的 grep 不排除
  注释，写了就命中（设计 §8.3）。
- **`RuleCommitment::committed` 拒绝空字节**（`rules.is_empty()` → `Err(EmptyRules)`），但 `b"\0"` 是
  合法的一字节规则；`RuleCommitment::unavailable().matches(任意)` **恒为 `false`**；`RuleVersion::new(0)`
  返回 `None`（设计 §4.2、§3.6）。
- **`seal` 只拒绝「五组全 `Unavailable`」**（`NothingCommitted`）；一组 `Committed` + 四组 `Unavailable`
  是合法的部分核心；`seal` **不会**返回 `EmptyRules`（设计 §4.1 末、§3.6 第 5 行）。
- **`RUNTIME` 不是保护边界**：P6C 的 `MemoryScope::RUNTIME` 只是记忆存储面，本块不把它当 `ProtectedCore`
  的替身，也不把 §321 五类语义塞进 claim 的 `scope`（设计 §6.1）。保护边界只有 `continuum-protected`
  这一处。
- **零表、零迁移、零事件、零审计**：不碰 `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`，
  不新增 `Migration::new`，不新增 `EventType` / `AuditKind`，不写 `continuum-events` / `continuum-persist`
  （设计 §3.3）。`continuum-learning` 只在内存里存记录与提案。
- **`ALLOWED` 的既有行不因 P6D 增枚**：26 行一字不动；唯一例外是装配方 `continuum-runtime` 那一行，
  **只加 `continuum-protected` 一项**（dev 边，唯一使用点是 `tests/p6d_protected_core.rs`）。
  其它既有行（尤其 `("continuum-core", &[])`）增枚即为反向边（设计 §7.1、切分 §四第 1 条）。
- **并集复核是 P6 合并门，不是永久门**：`p6_pre_allowed.txt` 是 P6D 自己的数据文件；改它必须与
  `ALLOWED` 分成两个文件的 diff 被评审看见。P7 新增 crate 时**实际到期的是断言 8**（新增 subject 不在
  `P6_NAMES` 里）；断言 1 的 27 行**不会红**——P7 不得往 P6 的 fixture 追加行（设计 §7.3.5）。P7 起换新
  基线或退役。`P6_EXPECTED_ROWS` 是 **5 行**，不是 4 行（设计 §11.1：两枚 crate 的读法）。
- **只新建设计 §3.5 冻结的类型**：protected 八枚（`ProtectedCore`、`ProtectedCoreParts`、`RuleCommitment`、
  `RuleVersion`、`RuleDigest`、`CoreFingerprint`、`ProtectedSemantic`、`ProtectedError`），learning 四枚
  （`LearningTarget`、`ObservationKind`、`LearningRecord`、`LearningProposal`）。不定义 `AuthorityRules` /
  `CapabilityRules` / `ContractRules` / `AuditRules` / `EffectRules`，不定义 `Evidence`、`MemoryClaim`、
  `ContextPackage`、`ContextTrust`（设计 §3.5 末、切分 §八.2 第 5 条）。
- **错误类型只有一枚** `ProtectedError`；`continuum-learning` **没有**错误类型，不包装 `ProtectedError`
  （设计 §3.5 末）。
- **不派生 serde**（两枚 crate 的 12 枚类型全部）；派生清单照设计 §3.5 的表，不增不减。
- **`ALLOWED` 只记 workspace 成员之间的边**；`sha2` / `thiserror` / `trybuild` 是外部 crate，不进表。
  dev 边也登记（`cargo tree --edges all` 把 dev 边算进依赖）。
- **`continuum-learning` 的 `ALLOWED` 行只有 `continuum-protected` 一项**；它命名不到
  `continuum-policy` / `continuum-semantics` / `continuum-capability` / `continuum-events` /
  `continuum-effect` 的类型——这就是 ENG-001 方向 (a) 的强制面（设计 §2.3、§4.5）。
- **代码注释、错误信息、测试断言信息用中文**；标识符用英文。
- **`cargo test --workspace --no-fail-fast` 全绿、0 warning**；`cargo build --workspace --all-targets`
  同样 0 warning。测试命令统一带 `TMPDIR="$PWD/.tmp"` 与 `timeout`；管道结尾不接 `tail`。
- **不要用 `git add -A`，不要 `git commit --amend`**。只 `git add <显式路径>`。新增 path 依赖会改
  `Cargo.lock`，一并提交锁文件。
- **本块不做的事**（照设计 §1.2、§9，逐条防漏读）：不建学习算法（去重、矛盾检测、评分、由记录生成提案）、
  不建执行器 / 图节点 / 调度 / 后台作业、不建提案应用、不建 §217 的自我改进流程、不建 §322 的
  「独立验证」与「用户批准」、不建签名 / 密钥 / 信任根、不建五组规则的规则本体、不建 UI / 报告 / 指标。
- **不做模型调用、路由选择、模型画像读取**（设计 §9，第 4 层 P3）：本块不依赖 `continuum-provider` /
  `continuum-model-registry`；六类目标里的 routing / scheduling 只是**目标枚举**，不是调用面。
- **不做 capability 判定**（设计 §9、§10.2 第 4 条）：C4／OPEN-016 与 P6A 同口径——**判定在外**；
  本块不关闭它，也不把判定搬进 `ProtectedCore`。
- **设计 §2.4 的 12 条「规范解释判定」不在实现里重判**：逐条按设计落点执行；本计划只在 Task 8 核对
  「落点栏是否有一条真实用例」，不另造解释。

---

## Review Focus

1. **并集基线被同一提交改宽**（设计 §7.3.5）：`p6_pre_allowed.txt` 与 `ALLOWED` 必须分成两处 diff；
   断言 4 是「集合相等」，不是「包含」。**拥有该代码的 task：Task 7**（Task 8 的变异复核再验一次）。
2. **P7 加 crate 时断言必红**（设计 §7.3.5）：P7 新增 crate 时**实际到期的是断言 8**（新增 subject
   不在 `P6_NAMES` 里）；断言 1 的 27 行**不会红**，因为 P7 不得往 P6 的 fixture 追加行。这是有到期日
   的门，不是永久门。P7 起二选一：新基线 + 重建 `P6_EXPECTED_ROWS` / `P6_NAMES` 与 `len() == 5`，
   或退役改为从 `cargo metadata` 生成。**拥有该代码的 task：Task 7**（注释里写明，Task 8 复核）。
3. **`subject: String` 携带受保护语义**（设计 §5.5、§10.2 第 12 条）：一条
   `target: ExecutionMethod, subject: "Authority semantics"` 的提案**编译得过**；本块**不**在 learning 里加
   `subject` 校验（那会造第二套词汇），只把「应用侧必须拒绝或升级」报给 P1 / 执行器。**拥有该代码的
   task：Task 4**（字段类型与构造点）；Task 5 的 R7 只证 `target` 的交集为空，**不证** `subject`。
4. **`RuleVersion(0)` / 空字节未拒**（设计 §3.6 第 1 行、§4.2）：`new(0)` 必须是 `None`；
   `committed(v, b"")` 必须是 `Err(EmptyRules)`；`committed(v, b"\0")` 必须 `Ok`。**拥有该代码的
   task：Task 2**（实现）＋ **Task 3**（R1b / R3b 照片）。
5. **规则编码漏组**（设计 §4.2）：`fingerprint()` 的规范编码必须按
   `authority → capability → contract → audit → effect` **五组逐组**编入；漏一组时 R5 的那一行必须红。
   **拥有该代码的 task：Task 2**（编码）＋ **Task 3**（五组表驱动照片，版本与摘要两种变异都覆盖）。

---

## 跨计划前置

### 一、本块的内部依赖与实际状态（2026-10-17 实读工作树）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| `continuum-protected`（本块新建） | 不存在；`grep -rn "continuum-protected\|continuum_protected" crates/ Cargo.toml` 零命中 | Task 1–3、Task 6 的 R9 |
| `continuum-learning`（本块新建） | 不存在；同上零命中 | Task 1、Task 4、Task 5 |
| `continuum-runtime`（既有装配方）的测试面 | 已存在；`tests/dependency_direction.rs` 的 `ALLOWED` 在 `:25`，用例在 `:478`，`workspace_crates()` 在 `:424` | Task 6（dev 边 + R9）、Task 7（并集复核） |
| `continuum-capability` 的 `CapabilityKind` 12 个具体值 | 已交付；`as_str` 是 12 臂穷尽 `match`（`crates/continuum-capability/src/capability.rs:236`），**没有 `ALL`** | Task 6 的 R9 手写字节 |
| `continuum-events` 的 `AuditKind::ALL`（8 枚） | 已交付；`crates/continuum-events/src/audit.rs:38` | Task 6 的 R9 |
| `continuum-effect` 的 `EffectType::ALL`（6 枚）与 `EffectState::ALL`（7 枚） | 已交付；`crates/continuum-effect/src/effect.rs:31`、`:112`。`EffectType` 有 `as_str`；**`EffectState` 没有 `as_str`**——编码在私有的 `persist::state_str`（`crates/continuum-effect/src/persist.rs:50`，crate 外不可见） | Task 6 的 R9（`EffectState` 的七个串由用例内 7 臂 `match` 给出） |
| `sha2` / `thiserror` | `[workspace.dependencies]` 已有（`sha2 = "0.10"`、`thiserror = "2"`） | Task 2 |
| `trybuild` | `[workspace.dependencies]` 已有 | Task 2（C1/C2/C4）、Task 4（C3/C5） |
| P6A `continuum-context`、P6B `continuum-trust`、P6C `continuum-memory` | **未落地**（本工作树 `crates/` 无这三个目录） | **只影响 Task 7 的最终验收**，不是本块任何 task 的编译依赖 |

**一句话结论**：本块的 Task 1–6 只依赖既有 crate 与两枚新建 crate，**今天就可以开工**；Task 7 的代码今天
也可以写，但它的**验收执行**等 P6A／P6B／P6C 的 crate 落地（见第五节）。

### 二、`P6_EXPECTED_ROWS` 依赖 P6A／P6B／P6C 的 `ALLOWED` 行（实读三份设计确认）

| crate | 设计边表（逐字来源） | `ALLOWED` 行 | 本计划的 `P6_EXPECTED_ROWS` |
|---|---|---|---|
| `continuum-context` | P6A 设计 §2.1：`continuum-artifact`、`continuum-core`、`continuum-memory`、`continuum-trust` 四项（其 `:124` 逐字给出整行） | 四项 | `("continuum-context", &["continuum-artifact", "continuum-core", "continuum-memory", "continuum-trust"])` |
| `continuum-trust` | P6B 设计 §2.1：workspace 内**零依赖**（其 `:77-80` 逐字给出 `("continuum-trust", &[])`） | `&[]` | `("continuum-trust", &[])` |
| `continuum-memory` | P6C 设计 §3.2：`continuum-persist` 一项（其 `:177` 逐字给出整行） | 一项 | `("continuum-memory", &["continuum-persist"])` |
| `continuum-protected` | 本设计 §3.2：workspace 内零依赖 | `&[]` | `("continuum-protected", &[])` |
| `continuum-learning` | 本设计 §3.2：`continuum-protected` 一项 | 一项 | `("continuum-learning", &["continuum-protected"])` |

**这张表的所有权**：`P6_EXPECTED_ROWS` 是 P6D 的常量，但前三行来自 P6A／P6B／P6C 的设计；若它们定稿时
改了自己的边表，**先改其 §2.1／§3.2，再由 P6D 改这张表**。P6D 不在复核里替它们改设计（设计 §7.3.2、§7.4）。
**合并顺序**：P6B／P6C 可先合入（行不引用别的 P6 crate）；P6A 必须在 P6B／P6C 之后（其 path 依赖不存在时
编不过）；P6D 与三者无共享 crate，可任意先后。断言 8 对中间态容忍，五枚全落地后退化成「恰好五行」
（设计 §7.3.3、§7.6）。

### 三、共写面（本块只动这些）

| 文件 | 本块的改动 | 所属 task | 冲突纪律 |
|---|---|---|---|
| `Cargo.toml`（workspace）的 `[workspace] members` | 加两行：`crates/continuum-protected`、`crates/continuum-learning` | Task 1 | 四块各加自己的行；本块的两行一起加 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` | 加两行：`("continuum-protected", &[])`、`("continuum-learning", &["continuum-protected"])` | Task 1 | 数组按字母序；不碰任何既有行的数组 |
| 同一文件的 `continuum-runtime` 行 | 加一项 `continuum-protected`（dev 边） | Task 6 | 唯一允许增枚的既有行；只加这一项 |
| 同一文件的**文件末尾** | 加 `p6_union_adds_only_its_own_rows`、`P6_EXPECTED_ROWS`、两个解析器 | Task 7 | 放末尾，缩小与 P6A／P6B／P6C 的冲突；不把基线常量放这里 |
| `crates/continuum-runtime/Cargo.toml` 的 `[dev-dependencies]` | 加 `continuum-protected = { path = "../continuum-protected" }` | Task 6 | 事实上的第四共写面（设计 §10.2 第 11 条）；四块各加自己的一行 |
| `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt` | **新建**：27 行 P6 前基线 + 三行文件头 | Task 7 | **P6D 自己的数据文件**，不在切分 §一 1.3 的三处共写里；改动单独核 diff |
| `crates/continuum-runtime/tests/p6d_protected_core.rs` | **新建**：R9 装配照片 | Task 6 | P6D 自己的用例 |
| `crates/continuum-runtime/src/main.rs` / `runtime_migrations()` | **不碰** | — | 本块零表、零迁移 |
| `crates/continuum-runtime/src/**` 其余 | **不碰** | — | 本块没有生产装配面 |
| 两份设计、规范、切分文档 | **不碰** | — | — |

### 四、哪些 task 可立即开工、哪些等前三块

- **Task 1–6：立即开工**。它们只依赖既有 crate、`sha2` / `thiserror` / `trybuild` 与两枚新 crate；
  与 P6A／P6B／P6C 无共享类型。
- **Task 7：代码可立即写，验收执行等 P6A／P6B／P6C 的 crate 与它们的 `ALLOWED` 行落地**。在只有 P6D
  两枚 crate 的树上，八条断言是**部分树绿**（断言 2、6、8 只覆盖已落地的 P6 crate）；这不是合并门。
- **Task 8：在 Task 7 的合并树验收之后**，再跑全量与源码面复核。
- **块内硬序**：Task 1 → Task 2 → Task 3；Task 1 → Task 4 → Task 5；Task 2 → Task 6；Task 1 → Task 7。
  Task 3 与 Task 4 可并行（不同 crate），Task 5 与 Task 6 可并行。

### 五、并集复核的执行时点（写死）

**验收时点 = 四块全部落地后的合并树**（设计 §7.6；切分 §六）。执行者是 P6D，记录 `git rev-parse HEAD`
与三条命令的输出。**中间态**（只落了 P6B／P6C，或只落了 P6D）只要求「现有用例 + 并集复核的已落地部分」
绿；**不要求五枚全在**。最终树上，`cargo test -p continuum-runtime --test dependency_direction` 必须跑
**三个用例**（两个原有 + `p6_union_adds_only_its_own_rows`）。

---

# 文件结构

```
crates/continuum-protected/
  Cargo.toml                    sha2 + thiserror；workspace 内零依赖
  src/lib.rs                    crate 文档：两 crate 的理由、空允许集、规范编码（设计 §4.2）、re-export
  src/error.rs                  ProtectedError（NothingCommitted / EmptyRules）
  src/version.rs                RuleVersion（0 非法）、RuleDigest（to_hex；pub(crate) from_bytes/as_bytes）
  src/commitment.rs             RuleCommitment + CommitmentState（Unavailable / Committed）
  src/core.rs                   ProtectedCore、ProtectedCoreParts、seal、CoreFingerprint、fingerprint、编码
  src/semantic.rs               ProtectedSemantic（§321 五类，ALL / as_str / parse）
  tests/core.rs                 R2 / R3 / R4
  tests/photos.rs               R1 / R1b / R3b / R5（五组表驱动）
  tests/type_level.rs           trybuild 驱动：C1 / C2 / C4
  tests/compile_fail/*.rs       三份编译失败样例（各配同名 .stderr）
crates/continuum-learning/
  Cargo.toml                    [dependencies] 只 continuum-protected；[dev-dependencies] trybuild
  src/lib.rs                    crate 文档：六类/五类/两枚记录类型、无错误类型、零持久化、re-export
  src/target.rs                 LearningTarget（六类，ALL / as_str / parse）
  src/observation.rs            ObservationKind（五类，中文 as_str / parse）
  src/record.rs                 LearningRecord（三个 pub 字段）
  src/proposal.rs               LearningProposal（四个私有字段，唯一构造点收 &ProtectedCore）
  tests/types.rs                未编号的形状测试（构造、getter、指纹读取、记录字面量）
  tests/photos.rs               R6 / R7 / R8 / R10 / R11
  tests/type_level.rs           trybuild 驱动：C3 / C5
  tests/compile_fail/*.rs       C3（各配同名 .stderr）
  tests/ui_pass/*.rs            C5（正控制）
crates/continuum-runtime/
  tests/p6d_protected_core.rs   R9 装配照片（dev 边 continuum-protected）
  tests/fixtures/p6_pre_allowed.txt   27 行 P6 前基线（P6D 自己的数据文件）
  tests/dependency_direction.rs        ALLOWED 两行 + runtime 一项 + 文件末尾的并集复核
  Cargo.toml                          [dev-dependencies] 加 continuum-protected
```

**本计划要改的既有文件**（只有这些）：

```
Cargo.toml                                             members 加两行（Task 1）
crates/continuum-runtime/tests/dependency_direction.rs  ALLOWED 两行（Task 1）＋ runtime 一项（Task 6）＋ 末尾用例（Task 7）
crates/continuum-runtime/Cargo.toml                    [dev-dependencies] 加 continuum-protected（Task 6）
```

**本计划不碰的文件**：`crates/continuum-runtime/src/**`（尤其 `main.rs` 的 `runtime_migrations()`）、
`crates/continuum-capability/**`、`crates/continuum-events/**`、`crates/continuum-effect/**`、
`crates/continuum-policy/**`、`crates/continuum-semantics/**`、`crates/continuum-persist/**`、
`crates/continuum-verify/**`、`crates/continuum-memory/**`（尚不存在）、`docs/02-工程.md`、
`docs/spec/**`、`docs/superpowers/specs/**`（四份设计一份都不改）。

---

## 各 task 的「运行，确认失败」一步量到的是哪一种失败（逐 task 扫过）

**这一步的名字只在「本 task 有新生产代码」时才成立。** 按本仓纪律：编译失败不算运行期红；测试-only 的
task「首次即绿」不是缺陷，不许改写成「先红后绿」——它们的红由 Task 8 的变异复核取。

| task | 那一步量到的是 | 为什么 |
|---|---|---|
| Task 1 | **真实测试失败**（`workspace 成员 ... 未列入 ALLOWED`） | 新成员已进 workspace，`ALLOWED` 尚未补行；这是刻意的门 |
| Task 2 | **编译失败**（`E0432` / `E0433`） | 类型由 Step 3 才建出；R2/R3/R4 的首次运行是空转的编译失败 |
| Task 3 | **首次即绿** | 本 task 只新建照片，测的是 Task 2 已交付的实现；若红，是 Task 2 的实现缺陷 |
| Task 4 | **编译失败**（`E0432` / `E0433`） | 类型由 Step 3 才建出 |
| Task 5 | **首次即绿** | 本 task 只新建照片，测的是 Task 4 已交付的实现 |
| Task 6 | **首次即绿** | dev 边与 R9 测试同时落地；R9 是对既有 API 的装配照片，没有新生产代码 |
| Task 7 | **首次即绿（部分树）** | 八条断言对未落地的 P6 crate 是容忍的；真实红由 Step 5 的变异自检与合并树验收取 |
| Task 8 | 无「确认失败」步 | 收尾与复核 |

---

### Task 1: 两枚 crate 的骨架与 `ALLOWED` 登记

**Files:**
- Create: `crates/continuum-protected/Cargo.toml`
- Create: `crates/continuum-protected/src/lib.rs`
- Create: `crates/continuum-learning/Cargo.toml`
- Create: `crates/continuum-learning/src/lib.rs`
- Modify: `Cargo.toml`（`[workspace] members` 末尾加两行）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加两行）

**Interfaces:**
- Produces: 两个可编译的空 crate；`continuum-learning` 对 `continuum-protected` 的 path 依赖。
- 不产生任何公开类型（类型在 Task 2／Task 4）。

- [ ] **Step 1: 建 `continuum-protected` 骨架**

`Cargo.toml` 只写 `[package]` 四行（`name = "continuum-protected"`、version/edition/rust-version 取
workspace 值），**暂时没有 `[dependencies]`**（`sha2` / `thiserror` 由 Task 2 增量加；一次声明齐会让
「只登记实际用到的边」在 Task 1 说谎）。`src/lib.rs` 只写 crate 文档，**不声明任何 `mod`**（模块由 Task 2 增量加），文档覆盖：
两枚 crate 而不是一枚的理由（ENG-001 方向 (a)，设计 §3.1）、workspace 内依赖为空是硬断言、
字段私有与唯一构造点（设计 §4.1）、规范编码只在本版本内解释（设计 §4.2）。
**文档里不要写出 `&mut self`、`Migration::new`、`EventType`、`AuditKind` 这些会被 S1／S7 命中的字面串。**

- [ ] **Step 2: 建 `continuum-learning` 骨架**

`Cargo.toml`：

```toml
[dependencies]
continuum-protected = { path = "../continuum-protected" }
```

**只这一条 workspace 边。**`src/lib.rs` 只写 crate 文档：六类目标/五类观察的出处（设计 §5.1、§5.2）、
`LearningRecord` 不是产物、`LearningProposal` 绑核心指纹（设计 §5.3、§5.4）、**没有错误类型**、
零持久化（设计 §3.5 末、§6）。文档里不要写出 `.seal(` / `::seal(` / `RuleCommitment::` / `.committed(`
（S2 会命中），也不要写出 `continuum_capability::` / `continuum_policy::` 一类的路径（S3 会命中）。

- [ ] **Step 3: 加 workspace members**

在 `Cargo.toml` 的 `members` **末尾**（`crates/continuum-image` 之后）加两行：

```toml
    "crates/continuum-protected",
    "crates/continuum-learning",
```

- [ ] **Step 4: 运行，确认失败（真实测试失败）**

```bash
mkdir -p .tmp
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：**用例失败**，信息里出现第一个未列入 `ALLOWED` 的成员（`workspace_crates()` 排序在前，通常是
`continuum-learning`；修完第一处后才可能看到 `continuum-protected`），断言信息是
`workspace 成员 ... 未列入 ALLOWED，它的依赖方向不会被检查`。这是切分 §一 1.3 说的「刻意红」，
**不要靠先不加 member 绕过**。

- [ ] **Step 5: 补 `ALLOWED` 两行**

在 `continuum-image` 行之后、`continuum-runtime` 行之前插入（内层数组按字母序）：

```rust
    // P6D Protected Core（设计 §3.2）。**空数组是硬断言**：ENG-001 方向 (a) 的强制面，
    // 任何指向 workspace 内别的 crate 的直接边都会让本表逐对断言变红。
    // workspace 内零依赖；外部依赖只有 sha2 / thiserror，不进本表。
    ("continuum-protected", &[]),
    // P6D Runtime Learning（设计 §3.2）。**只有一条 workspace 边**：
    // 学习侧读 ProtectedCore 的 fingerprint（LearningProposal 的唯一构造点收 &ProtectedCore）。
    // 不登记 memory / graph / artifact / capability / events / effect / policy / semantics /
    // context / method / verify / model-registry / trust——逐条理由见设计 §3.2。
    ("continuum-learning", &["continuum-protected"]),
```

- [ ] **Step 6: 运行，确认绿**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected -p continuum-learning
```

预期：第一条两个既有测试全绿（新增两行与 `cargo tree` 一致）；第二条两个空 crate 编译通过、0 warning。

- [ ] **Step 7: 提交**

```bash
git add Cargo.toml Cargo.lock crates/continuum-protected crates/continuum-learning crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(p6d): 新建 continuum-protected 与 continuum-learning 骨架并登记 ALLOWED"
```

---

### Task 2: `continuum-protected` 的类型、`seal` 与规范编码

**Files:**
- Modify: `crates/continuum-protected/Cargo.toml`（加 `sha2` / `thiserror` 与 dev 的 `trybuild`）
- Create: `crates/continuum-protected/src/error.rs`
- Create: `crates/continuum-protected/src/version.rs`
- Create: `crates/continuum-protected/src/commitment.rs`
- Create: `crates/continuum-protected/src/core.rs`
- Create: `crates/continuum-protected/src/semantic.rs`
- Modify: `crates/continuum-protected/src/lib.rs`
- Create: `crates/continuum-protected/tests/core.rs`（R2 / R3 / R4 ＋ `ProtectedSemantic` 基本测试）
- Create: `crates/continuum-protected/tests/type_level.rs`（C1 / C2 / C4）
- Create: `crates/continuum-protected/tests/compile_fail/*.rs` 与同名 `.stderr`

**Interfaces（逐字照设计 §3.5／§3.6／§4.1／§4.2）：**

```rust
pub struct ProtectedCore { /* 五个私有字段，字段名与 §322 逐字相同 */ }
pub struct ProtectedCoreParts {
    pub authority_rules: RuleCommitment,
    pub capability_rules: RuleCommitment,
    pub contract_rules: RuleCommitment,
    pub audit_rules: RuleCommitment,
    pub effect_rules: RuleCommitment,
}
pub fn seal(parts: ProtectedCoreParts) -> Result<ProtectedCore, ProtectedError>;

pub struct RuleCommitment { /* 私有 */ }
impl RuleCommitment {
    pub fn unavailable() -> Self;
    pub fn committed(version: RuleVersion, rules: &[u8]) -> Result<Self, ProtectedError>;
    pub fn version(&self) -> Option<RuleVersion>;
    pub fn digest(&self) -> Option<&RuleDigest>;
    pub fn matches(&self, rules: &[u8]) -> bool;
}

impl RuleVersion {
    pub fn new(value: u64) -> Option<Self>;   // 0 → None
    pub fn get(&self) -> u64;
}
impl RuleDigest {
    pub fn to_hex(&self) -> String;
    // crate 内构造：`committed` 造摘要、`core.rs` 的规范编码写摘要时用。
    // 公开构造仍只有 `RuleCommitment::committed`；crate 外不可见。
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self;
    // crate 内读 32 字节；规范编码用它，**不**用 `to_hex()` 的 64 个 ASCII 字节。
    pub(crate) fn as_bytes(&self) -> &[u8; 32];
}
impl CoreFingerprint { pub fn to_hex(&self) -> String; }
impl ProtectedCore {
    pub fn authority_rules(&self) -> &RuleCommitment;
    pub fn capability_rules(&self) -> &RuleCommitment;
    pub fn contract_rules(&self) -> &RuleCommitment;
    pub fn audit_rules(&self) -> &RuleCommitment;
    pub fn effect_rules(&self) -> &RuleCommitment;
    pub fn fingerprint(&self) -> CoreFingerprint;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtectedError {
    #[error("五组规则全部缺失：ProtectedCore 不能封存一个空壳")]
    NothingCommitted,
    #[error("规则字节为空：空规则不是规则")]
    EmptyRules,
}
```

- `ProtectedCore` / `ProtectedCoreParts` / `RuleCommitment` / `RuleVersion` / `RuleDigest` /
  `CoreFingerprint` / `ProtectedSemantic` 的派生**逐条照设计 §3.5 的表**；私有 `CommitmentState` 同样
  派生 `Debug, Clone, PartialEq, Eq`（`RuleCommitment` 的派生要用到）；`ProtectedError` 只派生
  `Debug, Clone, PartialEq, Eq`（`thiserror` 另给 `Display` / `Error`）。**都不派生 serde。**
- `ProtectedSemantic`：`Authority` / `Contract` / `Permission` / `Approval` / `SecurityPolicy` 五臂；
  `ALL` 五枚；`as_str` 返回 §321 原文（五个串逐字见 Global Constraints）；`parse` 是严格逆，表外 `None`。
- **规范编码**（`core.rs` 的私有函数，文档写在 `lib.rs`）：固定按
  `authority → capability → contract → audit → effect` 的次序；每组建入
  `u8` 组名长度 + 组名 ASCII 字节、`u8` 状态（`0` = `Unavailable`，`1` = `Committed`）；`Committed` 再编入
  `u64` 大端版本（8 字节）与 32 字节摘要（取 `RuleDigest::as_bytes()`，**不**取 `to_hex()`）。
  **不靠分隔符猜边界**；`Unavailable` 不带版本与摘要。
  `CoreFingerprint` 是这串编码的 SHA-256，`fingerprint()` **每次现算**，不是第六个字段。
  编码只在本版本内解释；**不承诺跨版本稳定**（设计 §4.2、§3.6 第 3 行）。
- `RuleDigest` 的字段私有；`from_bytes` / `as_bytes` 是 `pub(crate)`，**crate 外唯一的产生路径仍是
  `RuleCommitment::committed`**（`commitment.rs` 与 `core.rs` 是兄弟模块，没有这两个辅助函数就只能把字段
  改成 `pub(crate)` 或去读 64 个 ASCII 字节，两者都偏离设计）。
- `RuleDigest` / `CoreFingerprint` 的 `to_hex` 共用一个私有 hex 辅助函数（`src/version.rs` 或
  `src/core.rs` 的 `pub(crate) fn`），**不写第二份十六进制编码**。
- `committed`：`rules.is_empty()` → `Err(EmptyRules)`；否则 `Sha256::digest(rules)` 存 32 字节
  （经 `RuleDigest::from_bytes`）。
- `matches`：`Unavailable` 恒 `false`；`Committed` 重算 SHA-256 并与存储摘要比较。
- `seal`：五个字段全是 `Unavailable` → `Err(NothingCommitted)`；否则 `Ok`。**不返回 `EmptyRules`。**
- **源码注释里不要写出 `&mut self`、`fn set_`、`fn update`、`fn replace`、`impl Deref`、`impl DerefMut`**
  （S1 零命中）；也不要写出 `Migration::new` / `EventType` / `AuditKind`（S7 零命中）。

- [ ] **Step 1: 写 R2 / R3 / R4 的用例（先红）**

`tests/core.rs`：

- **R2**：`RuleCommitment::unavailable()` 的 `version()` / `digest()` 都是 `None`；
  `matches(b"任意")` 与 `matches(b"")` 都 `false`。
- **R3**：五组全 `unavailable()` 的 `seal` ⇒ `Err(ProtectedError::NothingCommitted)`（用 `matches!` 判**哪一臂**，
  不只判 `is_err()`）；`authority` 一组 `committed` + 其余四组 `unavailable` ⇒ `Ok`，并逐组断言访问器状态。
- **R4**：用**字段名**构造 `ProtectedCoreParts`，五组给五个可区分的承诺（版本 1–5、字节各不同）；
  `seal` 后逐组断言 `authority_rules().version() == Some(1)` 等五条，并逐组断言
  `matches(该组的原始字节)`。注释里写红条件：把 authority 与 contract 的字段**对调**，R4 必红。
- **`ProtectedSemantic` 的基本测试**（§4.4；R7 的完整 §321 五串对表在 Task 5）：`ALL` 五枚；五个 `as_str` 逐字等于
  Global Constraints 的 §321 原文；`parse(as_str(x)) == Some(x)`，表外串 `None`。

- [ ] **Step 2: 运行，确认失败（编译失败）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected --test core
```

预期：`error[E0432]` / `error[E0433]`（`continuum_protected::…` 尚未定义）。**按纪律这不是运行期红**，
照实记为编译期失败，继续 Step 3。

- [ ] **Step 3: 实现**

按上面的 Interfaces 建 `error.rs`、`version.rs`、`commitment.rs`、`core.rs`、`semantic.rs`，在 `lib.rs`
声明模块并 `pub use` 八枚类型。`Cargo.toml` 加：

```toml
[dependencies]
sha2 = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
trybuild = { workspace = true }
```

实现时逐条核：字段名与 §322 逐字；五个访问器同名；**无 `Default` / `Deref` / `DerefMut`**
（`Default` 由 C4 拍；`Deref` / `DerefMut` 由 Task 8 的 S1 加查 `impl Deref` / `impl DerefMut` 兜住）；
`RuleVersion::new(0)` → `None`；`committed` 空字节拒绝；`seal` 只拒全空；编码五组齐全。
**不要为了「将来好扩展」加第六个字段或 `Option<RuleCommitment>`。**

- [ ] **Step 4: 运行，确认绿**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected
```

预期：`core` 测试全绿；0 warning。

- [ ] **Step 5: 加 C1 / C2 / C4 的编译期照片**

`tests/type_level.rs` 用 `trybuild::TestCases::new()` 跑三份 `compile_fail`：

- C1：在 crate 外写字面量 `ProtectedCore { authority_rules: …, capability_rules: …, … }` ⇒ **编不过**
  （字段私有）。
- C2：在 crate 外写 `core.authority_rules = …` ⇒ **编不过**（字段私有；用一个
  `fn mutate(core: &mut ProtectedCore)` 参数，不需要构造核心）。
- C4：`ProtectedCore::default()` ⇒ **编不过**（无 `Default`）。

先跑：

```bash
TRYBUILD=overwrite TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected --test type_level
```

`.stderr` 由**实跑**生成，逐份人工核对（C1/C2 是私有字段错误、C4 是「找不到关联项」，具体错误码以
rustc 1.95 实际输出为准，**不许凭记忆写**）。再跑一次不带 `TRYBUILD=overwrite` 的命令确认绿，最后跑
`TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected` 确认整个 crate 绿。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-protected Cargo.lock
git commit -m "feat(protected): ProtectedCore 五字段、RuleCommitment、seal 与 CoreFingerprint"
```

---

### Task 3: R1 / R1b / R3b / R5 的照片（五组表驱动）

**Files:**
- Create: `crates/continuum-protected/tests/photos.rs`
- Modify: 仅当照片抓到 Task 2 的实现缺陷时，改 `crates/continuum-protected/src/**`

**Interfaces:** 无新增公开类型；只消费 Task 2 的 API。

- [ ] **Step 1: 写照片**

`tests/photos.rs` 逐条：

- **R1**：`RuleCommitment::committed(v, b"abc")` 对 `b"abc"` 为 `matches == true`、对 `b"abd"` 为
  `false`；`digest().unwrap().to_hex()` 是 **64 个十六进制字符**（`len() == 64` 且
  `chars().all(|c| c.is_ascii_hexdigit())`）。
- **R1b**：`committed(v, b"")` ⇒ `Err(EmptyRules)`；`committed(v, b"\0")` ⇒ `Ok`（**空字节与一个零字节的区别**）。
- **R3b**：`RuleVersion::new(0)` ⇒ `None`；`new(1).map(|v| v.get())` ⇒ `Some(1)`；
  `new(u64::MAX)` 可构造。
- **R5（五组逐组表驱动）**：基准 `ProtectedCoreParts` 五组给五个可区分的承诺（版本 1–5、字节各不同）；
  同一份 parts 两次 `seal`（`ProtectedCoreParts: Clone`，两次各传一份克隆），`fingerprint()` 相等
  （稳定性），记 F0。然后逐组一行，**只改该组**：

  | 行 | 组 | 变异 |
  |---|---|---|
  | 1 | `authority_rules` | 版本改为 9（摘要不变） |
  | 2 | `capability_rules` | 字节改为另一串（版本不变，摘要变） |
  | 3 | `contract_rules` | 版本改为 9 |
  | 4 | `audit_rules` | 字节改为另一串 |
  | 5 | `effect_rules` | 版本改为 9 |

  逐行 `seal` 得 Fi，断言 `Fi != F0`；把 `{F0, F1..F5}` 六个 `to_hex()` 放进 `BTreeSet`，断言
  `len() == 6`（两两不等）。**版本与摘要两种变异都覆盖**——只改版本测不出摘要是否进编码，只改摘要
  测不出版本是否进编码；这是 R5 的「只改该组的 version（或 digest）」在计划里的具体取法。
  注释写明：**编码漏掉任何一组时，该组那一行会 `Fi == F0`，本测试红**。

- [ ] **Step 2: 运行，确认（首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected --test photos
```

预期：**首次即绿**（它审计 Task 2 的实现）。若红：**先判断是照片写错还是实现漏组**——编码漏组、
空字节未拒、版本 0 未拒都是 Task 2 的实现缺陷，回 Task 2 修；**不许改宽照片**来迁就实现。
把红的位置与修法记进报告。

- [ ] **Step 3: 跑全 crate 并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-protected
git add crates/continuum-protected/tests/photos.rs
git commit -m "test(protected): 承诺、版本域与五组指纹敏感性照片"
```

若 Step 2 改了 `src/`，`git add` 显式加上对应源文件。

---

### Task 4: `continuum-learning` 的四枚类型

**Files:**
- Create: `crates/continuum-learning/src/target.rs`
- Create: `crates/continuum-learning/src/observation.rs`
- Create: `crates/continuum-learning/src/record.rs`
- Create: `crates/continuum-learning/src/proposal.rs`
- Modify: `crates/continuum-learning/src/lib.rs`
- Modify: `crates/continuum-learning/Cargo.toml`（加 `[dev-dependencies] trybuild`）
- Create: `crates/continuum-learning/tests/types.rs`
- Create: `crates/continuum-learning/tests/type_level.rs`（C3 / C5）
- Create: `crates/continuum-learning/tests/compile_fail/*.rs` 与同名 `.stderr`
- Create: `crates/continuum-learning/tests/ui_pass/*.rs`

**Interfaces（逐字照设计 §5.1／§5.2／§5.3）：**

```rust
pub enum LearningTarget {
    RoutingPolicy, PlanningHeuristics, ExecutionMethod,
    VerificationStrategy, ContextSelection, SchedulingHeuristic,
}
impl LearningTarget {
    pub const ALL: [LearningTarget; 6];
    pub fn as_str(&self) -> &'static str; // 小写原词
    pub fn parse(s: &str) -> Option<Self>;
}

pub enum ObservationKind {
    PlanRework, ModelSkillFailure, VerificationMiss,
    ExecutionMethodReliability, ContextSelectionNoise,
}
impl ObservationKind {
    pub const ALL: [ObservationKind; 5];
    pub fn as_str(&self) -> &'static str; // §216 中文原句
    pub fn parse(s: &str) -> Option<Self>;
}

pub struct LearningRecord {
    pub kind: ObservationKind,
    pub subject: String,
    pub evidence_refs: Vec<String>,
}

pub struct LearningProposal { /* 四个私有字段 */ }
impl LearningProposal {
    pub fn new(
        core: &ProtectedCore,
        target: LearningTarget,
        subject: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Self;
    pub fn target(&self) -> LearningTarget;
    pub fn subject(&self) -> &str;
    pub fn rationale(&self) -> &str;
    pub fn core_fingerprint(&self) -> &CoreFingerprint;
    pub fn is_current_for(&self, core: &ProtectedCore) -> bool;
}
```

- `LearningTarget::as_str` 的六个返回值：`routing policy` / `planning heuristics` / `execution method` /
  `verification strategy` / `context selection` / `scheduling heuristic`（**小写原词**）。
- `ObservationKind::as_str` 的五个返回值（§216 中文原句，空格与大小写照抄）：
  `哪些计划经常返工` / `哪些模型在哪些 Skill 上失败` / `哪些验证方式经常漏错` /
  `哪些 Execution Method 更可靠` / `哪些 Context 选择导致噪声`。
- `parse` **严格逆**：表外串 `None`，不取默认。实现用 `ALL.iter().find(|x| x.as_str() == s)`，
  **不写第二张字符串表**；`as_str` 的 `match` 穷尽且无通配臂。
- `LearningProposal` 字段私有：`target`、`subject`、`rationale`、`core_fingerprint`。`new` 从**调用方给的
  那一枚** `&ProtectedCore` 读 `fingerprint()`；**不保存 `core`**。`is_current_for` 比较
  `core_fingerprint` 与 `core.fingerprint()`。
- **没有错误类型**；不派生 serde；`LearningRecord` 三个字段 `pub`。
- 源码注释里不要写出 S2／S3／S4 的标识符（`.seal(`、`RuleCommitment::`、`.committed(`、
  `continuum_capability::`、`continuum_policy::`、`continuum_verify`、`MemoryClaim`、`ContextPackage`、
  `ContextTrust`）。

- [ ] **Step 1: 写未编号的形状测试**

`tests/types.rs`：

- `LearningRecord` 用结构体字面量构造（三个 pub 字段），断言字段值。
- 对 `LearningTarget::ALL` 的六枚各造一条 `LearningProposal`（同一个 core），逐条断言
  `target()` 等于输入、`subject()` / `rationale()` 等于输入、`core_fingerprint() == &core.fingerprint()`。
- 断言 `is_current_for(&core) == true`（这是形状测试；R8 的完整正反两侧在 Task 5）。
- 注释写明：**`subject` 是 `String`，本 test 不校验它是否指向受保护语义**——那是应用侧义务
  （设计 §10.2 第 12 条）。

- [ ] **Step 2: 运行，确认失败（编译失败）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-learning --test types
```

预期：`E0432` / `E0433`（类型尚未定义）。记为编译期失败。

- [ ] **Step 3: 实现**

按 Interfaces 建四个模块并在 `lib.rs` 重导出。`LearningProposal::new` 的 `subject` / `rationale` 用
`impl Into<String>`；`is_current_for` 用 `self.core_fingerprint == core.fingerprint()`（`CoreFingerprint: Copy`）。
`Cargo.toml` 的 `[dev-dependencies]` 加：

```toml
trybuild = { workspace = true }
```

- [ ] **Step 4: 加 C3 / C5 的编译期照片**

`tests/type_level.rs`：

- C3（`compile_fail`）：`LearningProposal::new(&core, ProtectedSemantic::Authority, "x", "y")` ⇒ **编不过**
  （类型不匹配）。
- C5（`ui_pass` 正控制）：`core.authority_rules()`、`core.fingerprint()`、
  `LearningProposal::new(&core, LearningTarget::RoutingPolicy, "x", "y")` ⇒ **编得过**。
  `core` 用 `ProtectedCoreParts` + `seal` 构造（一组 `committed`、四组 `unavailable`）。

先跑（生成 C3 的 `.stderr`）：

```bash
TRYBUILD=overwrite TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-learning --test type_level
```

`.stderr` 由**实跑**生成，逐份人工核对（C3 是**第二参数类型不匹配**：expected `LearningTarget`,
found `ProtectedSemantic`；`"x"` / `"y"` 本身满足 `Into<String>`。具体错误码以 rustc 1.95
实际输出为准，**不许凭记忆写**）。再跑一次不带 `TRYBUILD=overwrite` 的命令确认绿：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-learning --test type_level
```

**C5 必须有**——只有 `compile_fail` 时，一个把只读面也封死的 crate 会让 C1–C4 全绿。

- [ ] **Step 5: 运行，确认绿并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-learning
git add crates/continuum-learning Cargo.lock
git commit -m "feat(learning): 六类目标、五类观察、学习记录与绑定核心指纹的提案"
```

---

### Task 5: R6 / R7 / R8 / R10 / R11 的照片

**R9 不在这里**：它是 `crates/continuum-runtime/tests/p6d_protected_core.rs` 的装配照片，需要 runtime 的
dev 边，单独放在 Task 6（设计 §8.1 对 R9 指定了文件路径）。

**Files:**
- Create: `crates/continuum-learning/tests/photos.rs`
- Modify: 仅当照片抓到 Task 4（或 Task 2 的 `ProtectedSemantic`）的实现缺陷时，改对应 `src/**`

**Interfaces:** 无新增公开类型；只消费 Task 2／Task 4 的 API。

- [ ] **Step 1: 写照片**

`tests/photos.rs` 逐条：

- **R6**：`LearningTarget::ALL` 长度恰 6；**手写**六个期望 `as_str` 逐枚相等（**不从 `ALL` 派生期望值**）。
- **R7**：`ProtectedSemantic::ALL` 的 `as_str` 逐枚等于手写的 §321 五个英文串（Global Constraints 的五个）；
  `LearningTarget::ALL` 的 `as_str` 集合与那五个串**交集为空**（用 `BTreeSet` 求交）。
- **R8**：对六个 target 各造一条 `LearningProposal`（同一个 core），逐条 `is_current_for(&core) == true`；
  另 `seal` 一枚改过一组承诺（例如 authority 版本 +1）的 new_core，六条全部 `is_current_for(&new_core) == false`；
  并断言 `new_core.fingerprint() != core.fingerprint()`。
- **R10**：`ProtectedCore` 在一批提案构造前后逐字段相等（`PartialEq`）；`fingerprint()` 不变。
  用 `let before = core.clone(); …; assert_eq!(core, before);`。
- **R11**：`ProtectedSemantic` / `LearningTarget` / `ObservationKind` 各写一个**无通配臂**的穷尽
  `match` 辅助函数（每个臂返回该值的 `as_str()`），对 `ALL` 逐枚走该函数；再对每枚断言
  `parse(as_str(x)) == Some(x)`，对表外串断言 `None`。`ObservationKind` 另加一条**手写五个中文串**的逐枚断言
  （R11 只保证互逆，不保证原文；原文由这一条钉住）。
  R11 的编译期性质只有一半：`as_str` 的穷尽 `match` 保证**加第七类/第六类时 `as_str` 编译不过**；
  `ALL` 是手写清单，**固定数组长度挡不住「补了 `as_str` 却漏加 `ALL`」**——那时编译照过，R11 的遍历
  也走不到新臂（与 `continuum-effect` 的 `EffectState::ALL` 同形；`variant_count` 在 rustc 1.95 仍
  unstable）。这是设计未覆盖的缺口，本块**不引入宏、不补 `variant_count`**，只把限制写在这里。

- [ ] **Step 2: 运行，确认（首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-learning --test photos
```

预期：首次即绿。若红：按错误归位——`ProtectedSemantic` 的串错回 Task 2，六类/五类的串错回 Task 4；
**不许改宽手写语料**来迁就实现。

- [ ] **Step 3: 跑全 crate 并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-learning
git add crates/continuum-learning/tests/photos.rs
git commit -m "test(learning): 六类/五类清单、核心指纹绑定与穷尽互逆照片"
```

若改了 `src/`，显式加上对应路径。

---

### Task 6: R9 装配照片与 runtime 的 dev 边

**Files:**
- Modify: `crates/continuum-runtime/Cargo.toml`（`[dev-dependencies]` 加 `continuum-protected`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（runtime 行加 `continuum-protected`）
- Create: `crates/continuum-runtime/tests/p6d_protected_core.rs`

**Interfaces:**
- Consumes: `continuum_protected::{ProtectedCoreParts, RuleCommitment, RuleVersion, seal}`；
  `continuum_capability::{CapabilityKind, FsAction, GitAction, GithubAction, EmailAction, RegistryAction,
  PaymentAction, EnvAction}`；`continuum_events::AuditKind`；`continuum_effect::{EffectType, EffectState}`。
- Produces: 无生产接口；R9 照片。

- [ ] **Step 1: 登记 dev 边**

`crates/continuum-runtime/Cargo.toml` 的 `[dev-dependencies]` 末尾加：

```toml
# P6D 的 R9 装配照片（设计 §3.2、§8.1）：唯一使用点是 tests/p6d_protected_core.rs。
# 生产装配落位后边别升为普通边，ALLOWED 数组不变，只改行内注记。
continuum-protected = { path = "../continuum-protected" }
```

`ALLOWED` 的 `continuum-runtime` 行按字母序在 `continuum-policy` 与 `continuum-provider` 之间加
`"continuum-protected"`。**只加这一项，不动该行其余项**（P6D 单独落地时该行是 17 项；若 P6C 已先合入，
`continuum-memory` 也在该行里，**不要删它**）。

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：绿（`cargo tree` 已能看到 dev 边，表与树一致）。

- [ ] **Step 2: 写 R9**

`tests/p6d_protected_core.rs` 的规范字节**只在用例里生成**，固定分隔符取 `\n`
（**不带尾随换行**）：

- `capability`：**手写 12 个具体值**（`Filesystem(Read)`、`Filesystem(Write)`、`Git(Read)`、
  `Git(WorktreeWrite)`、`Git(CommitLocal)`、`Git(Push)`、`Git(DeleteRemote)`、`Github(CreatePr)`、
  `Email(Send)`、`Registry(Publish)`、`Payment(Charge)`、`Environment(Deploy)`）；
  写一个 12 臂、**不带 `..`** 的 `match` 辅助函数拿到各臂的 `as_str()`，再按该顺序 `join("\n")`。
- `audit`：`AuditKind::ALL`（8 枚）逐个 `as_str()`，按 `ALL` 顺序 `join("\n")`。
- `effect`：`EffectType::ALL`（6 枚）的 `as_str()` 后接 `EffectState::ALL`（7 枚）的**本用例固定串**，
  同一分隔符 `join("\n")`。**`EffectState` 没有 `as_str`**（编码是 `continuum-effect` 的私有
  `persist::state_str`，crate 外读不到）：在用例里写一个 **7 臂、无通配臂**的 `match` 取本用例固定串
  （`planned` / `authorized` / `executing` / `committed` / `failed` / `rolled_back` / `unknown`，与
  `persist.rs` 的落库编码同形；新增第 8 个状态时这条 `match` 会编译不过，必须回到用例；
  上游编码若变，这条 tripwire 也必须同步改）。两段都收进 `Vec<&'static str>` 再 `join`。
- `authority` / `contract`：不上字节，直接 `RuleCommitment::unavailable()`。
- 三组 `committed` 的版本都取 `RuleVersion::new(1).unwrap()`；`seal` 后逐组断言：
  `capability_rules` / `audit_rules` / `effect_rules` 的 `version() == Some(1)`、`digest().is_some()`、
  `matches(同一段字节) == true`；`authority_rules` / `contract_rules` 的 `version()` / `digest()` 都是 `None`。
- 注释写明：**这是 tripwire**——上游规则集落地后必须改这条用例，不许静默；
  **生产装配落位时必须复用同一段字节生成逻辑**，不许另写一份（两份字节会给出两个摘要，而两个都「看起来对」）。

- [ ] **Step 3: 运行，确认（首次即绿）**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test p6d_protected_core
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：两条都绿。R9 是对 Task 2 的 API 做的装配照片，没有新生产代码，**首次即绿**；
若红，先核字节顺序/分隔符，再核 API 是否与设计不符。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-runtime/Cargo.toml crates/continuum-runtime/tests/dependency_direction.rs crates/continuum-runtime/tests/p6d_protected_core.rs Cargo.lock
git commit -m "test(runtime): P6D 装配照片与 Protected Core 的 dev 边"
```

---

### Task 7: `ALLOWED` 并集复核（fixture + `P6_EXPECTED_ROWS` + 八条断言）

**Files:**
- Create: `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（**文件末尾**加常量、解析器、用例）

**Interfaces:**
- `const P6_EXPECTED_ROWS: &[(&str, &[&str])]`（5 行，逐字见 §7.3.2）；
- `fn p6_names() -> Vec<&'static str>`（由 `P6_EXPECTED_ROWS` 派生：
  `P6_EXPECTED_ROWS.iter().map(|(pkg, _)| *pkg).collect()`；设计里的 `P6_NAMES` 在本文件落成这个
  函数，**不写第三张名单**）；
- `fn pre_p6_allowed() -> BTreeMap<&'static str, Vec<&'static str>>`（读
  `include_str!("fixtures/p6_pre_allowed.txt")`）；
- `fn p6d_parse_allowed_source(source: &str) -> BTreeMap<String, Vec<String>>`（严格解析
  `const ALLOWED` 到它的 `];`）；
- `#[test] fn p6_union_adds_only_its_own_rows()`（八条断言）。

**硬前置（验收执行）**：**P6A／P6B／P6C 的 crate 与它们的 `ALLOWED` 行已落地**（设计 §7.6）。
代码可以现在写，但 Step 4 的合并树验收必须等前三块；中间态只算部分树绿。

- [ ] **Step 1: 建 fixture（27 行）**

`p6_pre_allowed.txt` 的文件头三行（前两行是设计 §7.3.1 要求逐字写进文件的声明）：

```
# P6D 自己的基线：03e79c7 的 ALLOWED（27 行）。
# 改这个文件是 P6D 的改动，不是 P6 任一执行块的改动；合并评审必须单独核这一处 diff。
# 这是 P6 合并门，不是永久门；P7 起按 §7.6 重新基线化或退役，不得在此追加行。
```

正文 27 行（**逐行等于 `03e79c7` 的 `ALLOWED`**；顺序与那张表一致）：

```
continuum-core:
continuum-events:continuum-core
continuum-persist:continuum-events
continuum-provider:continuum-capability,continuum-core,continuum-persist
continuum-artifact:continuum-core,continuum-events,continuum-persist
continuum-port:continuum-artifact
continuum-operator:continuum-artifact
continuum-graph:continuum-artifact,continuum-events,continuum-port,continuum-operator,continuum-persist
continuum-workspace:continuum-events,continuum-persist
continuum-sandbox:continuum-workspace
continuum-effect:continuum-events,continuum-persist
continuum-policy:continuum-artifact,continuum-effect,continuum-persist
continuum-capability:continuum-core,continuum-effect,continuum-events,continuum-persist
continuum-secrets:continuum-capability
continuum-connector:continuum-capability,continuum-core,continuum-effect,continuum-persist,continuum-provider,continuum-secrets
continuum-model-registry:continuum-capability,continuum-core,continuum-persist
continuum-node:continuum-artifact
continuum-method:
continuum-canonical:continuum-persist
continuum-semantics:continuum-canonical,continuum-persist
continuum-budget:continuum-persist,continuum-semantics
continuum-media:continuum-artifact,continuum-graph,continuum-method,continuum-operator,continuum-verify
continuum-verify:continuum-artifact,continuum-events,continuum-graph,continuum-operator,continuum-persist
continuum-research:continuum-artifact,continuum-graph,continuum-method,continuum-operator,continuum-verify
continuum-code:continuum-artifact,continuum-graph,continuum-method,continuum-operator,continuum-verify,continuum-workspace
continuum-image:continuum-artifact,continuum-graph,continuum-operator,continuum-persist,continuum-verify
continuum-runtime:continuum-artifact,continuum-budget,continuum-canonical,continuum-capability,continuum-core,continuum-effect,continuum-events,continuum-graph,continuum-model-registry,continuum-persist,continuum-policy,continuum-provider,continuum-sandbox,continuum-secrets,continuum-semantics,continuum-verify,continuum-workspace
```

核对：`grep -v '^#' crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt | grep -c .` 输出 `27`。
**末尾不要留空行。**

- [ ] **Step 2: 在 `dependency_direction.rs` 末尾加常量与解析器**

`P6_EXPECTED_ROWS` 逐字（5 行）：

```rust
const P6_EXPECTED_ROWS: &[(&str, &[&str])] = &[
    // P6A（其设计 §2.1，四项）
    ("continuum-context", &["continuum-artifact", "continuum-core", "continuum-memory", "continuum-trust"]),
    // P6B（其设计 §2.1，零项）
    ("continuum-trust", &[]),
    // P6C（其设计 §3.2，一项）
    ("continuum-memory", &["continuum-persist"]),
    // P6D：Protected Core（本设计 §3.2，零项）
    ("continuum-protected", &[]),
    // P6D：Runtime Learning（本设计 §3.2，一项）
    ("continuum-learning", &["continuum-protected"]),
];
```

**解析器必须做三件事**：

1. 用 `include_str!("dependency_direction.rs")` 读入当前文件，**只取**
   `const ALLOWED: &[(&str, &[&str])] = &[` 到它的 `];` 这一段；
2. **先剥掉 `//` 到行尾的注释**再解析——现有表在 `continuum-runtime` 条目的注释里有一处
   `` `("continuum-runtime", &[…])` `` 这样的字面，不剥会把注释当表项；
3. 严格按 `(` `"crate"` `,` `&` `[` 若干 `"dep"` `]` `)` 的语法解析；**任何残余字符、重复 subject、
   空元素都 `panic!`**，不许静默跳过。

`pre_p6_allowed` 解析 fixture：跳过 `#` 注释与空行；`split_once(':')`；`crate:` 为空 Vec；
`crate:dep1,dep2` 按逗号切；**空 subject / 空 dep / 重复 subject 都 `panic!`**；最后断言 27 行。

- [ ] **Step 3: 写八条断言（一条测试内）**

按设计 §7.3.3 逐条，**顺序与编号照原文**：

1. fixture 可解析且是 27 行；subjects 无重复。
2. `P6_EXPECTED_ROWS.len() == 5`；subjects 无重复。
3. 解析器自检：取出的行数 `== workspace_crates().len()`；`ALLOWED` subjects 无重复；
   **每一行的每个元素都是 `workspace_crates()` 的成员**（这一条是现有用例没有的）。
4. 对每个 `(pkg, allowed)` in `pre_p6_allowed()`，若 `pkg != "continuum-runtime"`，则
   `ALLOWED[pkg] == allowed`（**集合相等**，不是包含；**行缺失即红**）。**P5 六行逐行冻结**；尤其
   `continuum-image` 行的 `continuum-persist` 是 P5f 的计划自定项，并集复核**不重判它**——谁要改是
   P5f 的持有方另起一次改动（设计 §7.3.1）。
5. runtime 行只增不减：`pre["continuum-runtime"] ⊆ ALLOWED["continuum-runtime"]`，且差集
   `⊆ p6_names()`。
6. 对每个 `(pkg, allowed)` in `P6_EXPECTED_ROWS`，若 `pkg` 在 `workspace_crates()` 里，
   则 `ALLOWED[pkg] == allowed`（集合相等；**行缺失即红，不得跳过**）。
7. 对每个基线 `pkg`（`pkg != "continuum-runtime"`），`ALLOWED[pkg] ∩ p6_names() == ∅`。
8. `ALLOWED.subjects \ pre_p6_allowed().subjects == { p in p6_names() | p ∈ workspace_crates() }`。

**用例上方的注释**写设计 §7.3.1 的两条声明（与 fixture 文件头同一件事的两个入口）：
「改这个文件是 P6D 的改动，不是 P6 任一执行块的改动；合并评审必须单独核这一处 diff。」
「这是 P6 合并门，不是永久门；P7 起按 §7.6 重新基线化或退役，不得在此追加行。」

- [ ] **Step 4: 运行（当前树 = 部分树绿；合并树 = 验收）**

当前树（若 P6A／P6B／P6C 尚未落地，只有 P6D 两枚 crate）：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction p6_union_adds_only_its_own_rows -- --exact
```

预期：**绿**。这是部分树绿：断言 2、6、8 只覆盖已落地的 `continuum-protected` / `continuum-learning`；
**不是 P6 合并门**。记录 `git rev-parse HEAD` 与输出。

**验收执行（等 P6A／P6B／P6C 落地）**：在四块全部落地的合并树上重跑：

```bash
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction p6_union_adds_only_its_own_rows -- --exact
```

预期：第一条跑**三个用例**（两个原有 + 并集复核）全绿；第二条绿。把 `git rev-parse HEAD`、两条命令的
`test result:` 行与五行的实际值记进合并门报告。

**若红**：先分清是哪一类——是 P6A／B／C 的 `ALLOWED` 行与它们的设计不符（断言 6），还是有人在既有行上
增了枚（断言 4／7），还是漏了行（断言 8）。**不许为了让测试变绿而改宽 `P6_EXPECTED_ROWS` 或基线**；
要改先改对应块的设计与 §7.4 的对表，再单独提交本表。

- [ ] **Step 5: 变异自检（基线防静默）**

用 `trap` 装还原、前后各核一次 `sha256sum`、独立日志路径：

```bash
set -o pipefail   # 预期红时让管道退出码非零；本段不 set -e，后面仍要还原
FIX=crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt
BAK=.tmp/p6_pre_allowed.txt.bak
before=$(sha256sum "$FIX" | cut -d' ' -f1)
cp "$FIX" "$BAK"
trap 'cp "$BAK" "$FIX"; echo "已还原"' EXIT INT TERM
# 把 continuum-core: 改成 continuum-core:continuum-events（只改 fixture，不改 ALLOWED）
sed -i 's/^continuum-core:$/continuum-core:continuum-events/' "$FIX"
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction p6_union_adds_only_its_own_rows -- --exact 2>&1 | tee .tmp/p6d-union-mutation.log
# 预期：断言 4 红（fixture 与 ALLOWED 不再逐行相等）
cp "$BAK" "$FIX"
after=$(sha256sum "$FIX" | cut -d' ' -f1)
test "$before" = "$after" && echo "sha256 一致"
```

**这一条证明的是「表被改宽时断言 4 会红」**，不是防篡改；报告里附变异前后的 sha256 与日志路径。
若变异后仍绿，说明解析器或断言 4 写错了，**先修测试**。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "test(runtime): P6D ALLOWED 并集复核——P6 前 27 行基线 + P6 五行对表"
```

**这次提交只含 P6D 的两个文件**；合并树的验收结果另记在合并门报告里，不改代码。

---

### Task 8: 收尾与复核

**Files:**
- Modify: `crates/continuum-protected/**`、`crates/continuum-learning/**`、`crates/continuum-runtime/tests/**`
  （**仅在复核发现缺口时**）
- 不改设计、规范、切分文档。

**Interfaces:** 无新增公开接口；只复核 Task 1–7 已交付的 API、依赖边与门。

- [ ] **Step 1: 全量验证（门读数按 `Running` / `test result:` 行数读）**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee .tmp/p6d-final-test.log
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets 2>&1 | tee .tmp/p6d-final-build.log
```

预期：全绿、0 warning。**判据不是退出码**：逐条读 `Running` 与 `test result:` 的行数，确认射程覆盖全部
测试目标（Doc-tests 算一条）；`--no-fail-fast` 必须带；管道结尾不接 `tail`。

- [ ] **Step 2: 依赖面终态核对**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test migrations
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test startup
TMPDIR="$PWD/.tmp" timeout 600 cargo tree -p continuum-protected --edges all --depth 1
TMPDIR="$PWD/.tmp" timeout 600 cargo tree -p continuum-learning --edges all --depth 1
```

**逐条核**：

- `continuum-protected` 的输出 = 自身 + `sha2` / `thiserror`（直接依赖）+ `[dev-dependencies]` 的
  `trybuild`（预期，`--edges all` 会打印它）；判据是**没有第二枚 `continuum-*`**（设计 §7.6 的同一句
  过窄，按本条读）；
- `continuum-learning` 的输出里 `continuum-protected` 恰一行，**没有第二枚 `continuum-*`**（dev 的
  `trybuild` 同样预期）；
- `ALLOWED` 的 protected 行是 `&[]`、learning 行只有 `continuum-protected`；runtime 行的**增量** =
  `P6_NAMES ∩ 实际落地的 P6 项`（并集复核断言 5），**不是**固定的「恰多一项」。按块列出的期望集合：

  | 块 | runtime 行可能新增的项 | 何时出现 |
  |---|---|---|
  | P6C | `continuum-memory` | 其设计 §3.3 要求 `main.rs` 注册 `p6c_memory_migrations()`，**预期** |
  | P6D | `continuum-protected` | 本块 dev 边，**必有** |
  | P6A | `continuum-context` | 仅当 P6A 装配进 runtime（P6D 设计 §10.2 第 11 条「若装配同理」） |
  | P6B | `continuum-trust` | 仅当 P6B 装配进 runtime |

  即 `ALLOWED["continuum-runtime"] = 17 项基线 ∪ 上表中实际落地的项`；**P6D 自己的增量只有
  `continuum-protected`**，`continuum-memory` 是 P6C 的预期项，不是污染。其余 26 行与 fixture 逐行
  相等（并集复核的断言 4）；
- 本块没有动 `runtime_migrations()`，`migrations` 与 `startup` 两个测试目标仍绿（这两个文件都存在）。

- [ ] **Step 3: 源码面 S1–S8**

```bash
grep -rn "&mut self\|fn set_\|fn update\|fn replace\|impl Deref\|impl DerefMut" crates/continuum-protected/src/
grep -rn "\.seal(\|::seal(\|RuleCommitment::\|\.committed(" crates/continuum-learning/src/
grep -rn "use continuum_capability\|use continuum_events\|use continuum_effect\|use continuum_policy\|use continuum_semantics\|continuum_capability::\|continuum_events::\|continuum_effect::\|continuum_policy::\|continuum_semantics::" crates/continuum-learning/src/
grep -rn "continuum_verify\|MemoryClaim\|ContextPackage\|ContextTrust" crates/continuum-learning/src/
grep -n "continuum-" crates/continuum-protected/Cargo.toml
grep -n "continuum-" crates/continuum-learning/Cargo.toml
grep -rn "Migration::new\|EventType\|AuditKind" crates/continuum-protected crates/continuum-learning
grep -n -C3 '"continuum-protected"\|"continuum-learning"' crates/continuum-runtime/tests/dependency_direction.rs
```

**预期与作用域**：

- S1 零命中（**注释也算**——所以源码里不许写出 `&mut self`、`impl Deref`、`impl DerefMut` 字面串）。
  S1 同时是设计 §3.6 第 7 行 `Deref` / `DerefMut` 的照片；`Default` 由 C4 拍。
- S2 零命中（**排除注释行**；判据是学习 crate 不造承诺、不封存）；
- S3 零命中（**排除注释行**；这是判据 4 的 crate 面）；
- S4 零命中（模式取标识符全名，不取裸 `Evidence`；文档里「P5a 的证据」这类说明文字不算）；
- S5 只命中 `name = "continuum-protected"`；
- S6 只命中本包名与 `continuum-protected`；
- S7 零命中（两个 crate 都零迁移、零事件、零审计）；
- S8：**不是断言，是人工逐字比对**。读 `ALLOWED` 的三处并逐字核：protected 行 `("continuum-protected", &[])`、
  learning 行 `("continuum-learning", &["continuum-protected"])`、runtime 行新加的 `"continuum-protected"`
  一项；三处都必须与设计 §3.3／§7.3.2 逐字一致。上面那条 `grep -C3` 只用来给候选行上下文，**不靠 `head`
  截断，也不把 grep 命中当通过**。

**零命中不是充分证据**：grep 按行匹配，跨行折行会漏；做法是 grep 出候选行后对 `src/` 逐处通读，
结论以通读为准。

- [ ] **Step 4: 设计 §2.4 的 12 条逐条核「落点是否有真实用例」，并标出无照片的两行**

不重述表，只核落点：

| §2.4 的行 | 落点 | 本计划的用例 |
|---|---|---|
| 1（元素类型取 `RuleCommitment`） | §4.2 | Task 2 的 R2/R3/R4；Task 3 的 R5 |
| 2（§321 ↔ §322 的非双射映射） | §4.4 | **设计正文表，无代码照片**；R7 只证 §321 五串与六类目标交集为空，不证这张表 |
| 3（`Unavailable` 状态） | §4.2、§4.3 | Task 2 的 R2/R3；Task 6 的 R9 |
| 4（`CoreFingerprint`） | §4.2、§5.4 | Task 3 的 R5；Task 5 的 R8/R10 |
| 5（`LearningTarget` 六类） | §5.1 | Task 5 的 R6/R7 |
| 6（`ObservationKind` 五类） | §5.2 | Task 5 的 R11（含手写中文串） |
| 7（提案绑 core 指纹） | §5.3、§5.4 | Task 5 的 R8 |
| 8（两个 crate） | §3.1 | Task 1 的 `ALLOWED` 两行 + Task 8 的 `cargo tree` |
| 9（并集复核的 27 行基线） | §7.3 | Task 7 的八条断言 + 变异自检 |
| 10（`subject` / `evidence_refs` 不透明串） | §5.3 | Task 4 的字段类型；**应用侧义务无照片**（设计 §10.2 第 12 条） |
| 11（空字节 / 全空核心拒绝） | §4.2 | Task 3 的 R1b + Task 2 的 R3 |
| 12（`ObservationKind` 中文 `as_str`） | §5.2 | Task 5 的 R11 手写五串 |

**第 2 行的落点不是用例**：设计 §4.4 只给了一张正文映射表，没有映射 API；R7 只证 §321 五串与
`LearningTarget` 六类的交集为空，**不证** `Authority → authority_rules` 等五行。本计划**不自造新的
映射类型或 API**，这一行标为「设计正文表，无代码照片」。

**第 10 行的落点不是用例**：`subject` 指向受保护语义在类型上可编译，本块只报出应用侧义务
（Review Focus 3、`## 遗留` 第 1 条），**不标注为覆盖**。

- [ ] **Step 5: 变异复核（承重守卫）**

按本仓纪律：每轮 `trap` 装还原、变异前后 `sha256sum`、独立日志、报告逐轮附「档位（取反／放宽／收紧／移除）」。
**至少四轮**：

1. **基线被同一提交改宽（真边＋表）**：在 `crates/continuum-policy/Cargo.toml` 加
   `continuum-protected = { path = "../continuum-protected" }`，并在 `ALLOWED` 的 `continuum-policy`
   行加 `"continuum-protected"`。**现有用例 `every_crate_depends_only_on_its_allowed_set` 会绿**（表与树
   一致），**并集复核的断言 4／7 红**（**取反**；改的是被断言的允许集合）。还原两个文件；这一轮是
   Review Focus 1 的正面证据，**不许提交**。模板：

   ```bash
   POL=crates/continuum-policy/Cargo.toml
   DD=crates/continuum-runtime/tests/dependency_direction.rs
   cp "$POL" .tmp/policy.Cargo.toml.bak
   cp "$DD" .tmp/dependency_direction.rs.bak
   trap 'cp .tmp/policy.Cargo.toml.bak "$POL"; cp .tmp/dependency_direction.rs.bak "$DD"; echo "已还原"' EXIT INT TERM
   # …加真边与 ALLOWED 项…
   TMPDIR="$PWD/.tmp" timeout 900 cargo test -p continuum-runtime --test dependency_direction 2>&1 | tee .tmp/p6d-mutation-policy-row.log
   # 预期：every_crate_depends_only_on_its_allowed_set 绿；p6_union_adds_only_its_own_rows 红（断言 4、7）
   cp .tmp/policy.Cargo.toml.bak "$POL"
   cp .tmp/dependency_direction.rs.bak "$DD"
   ```

   报告里附两个文件变异前后的 `sha256sum`、红的位置与日志路径。
2. **规则编码漏组**：在 `core.rs` 的编码函数里删掉 `audit_rules` 那一段（**保持编译**，例如从有序表里
   删掉一项或注释掉该段的调用）（**移除**）⇒ R5 的 audit 行红（`Fi == F0`）；还原后 R5 绿。
3. **`RuleVersion(0)` 放行**：把 `new` 的 `value == 0` 判断改成恒 `Some`（**放宽**）⇒ R3b 红；还原。
4. **`committed` 空字节放行**：删掉 `rules.is_empty()` 的 `Err(EmptyRules)`（**移除**）⇒ R1b 红；还原。

**等价变异体不算守卫**：把 R5 的版本变异改成「把两组同时改」不是 R5 的形状（它会红，但红的原因不是
逐组敏感性）——不要用它冒充「五组逐组」的证据。

- [ ] **Step 6: 提交（仅当有改动）**

```bash
git add crates/continuum-protected crates/continuum-learning crates/continuum-runtime/tests
git commit -m "chore(p6d): 收尾复核——S1–S8、全量测试与变异复核"
```

---

## 自审

### 一、Spec coverage（设计节 → task）

| 设计节 | 内容 | 落点 |
|---|---|---|
| §3.1–§3.4 | 两枚 crate、依赖边、共写面、入度为零 | Task 1、Task 8 Step 2 |
| §3.5–§3.6 | 12 枚类型与派生、API 形状 | Task 2、Task 4 |
| §4.1–§4.2 | `ProtectedCore` 五字段、`seal`、承诺、指纹与编码 | Task 2、Task 3、Task 6 |
| §4.3 | 五组规则的上游归属与 R9 装配状态 | Task 6 |
| §4.4 | §321 五语义与 `ProtectedSemantic` | Task 2、Task 5（R7）；**§321↔§322 映射表无代码照片**（遗留第 12 条） |
| §4.5–§4.6 | ENG-001 方向 (a)、三条更新 MUST 的缺口 | Task 1（crate 边界）、`## 遗留` |
| §5.1–§5.2 | 六类目标、五类观察 | Task 4、Task 5（R6/R11） |
| §5.3–§5.4 | 记录、提案、指纹绑定 | Task 4、Task 5（R8/R10） |
| §5.5 | 判据 4 的三条结构事实 | Task 1（crate）、Task 2（值）、Task 5（类型/照片） |
| §6 | 与 P6C 的接缝（`RUNTIME` 不是边界、零表） | Global Constraints、`## 遗留` |
| §7.3 | 并集复核的形状、八条断言、生命周期 | Task 7、Task 8 |
| §7.4 | 五行对表 | Task 7 的 `P6_EXPECTED_ROWS`、`## 遗留` |
| §7.6 | 运行手册与验收时点 | Task 7 Step 4、Task 8 Step 1 |
| §8.1 R1–R11 | 13 张运行期照片 | Task 2（R2/R3/R4）＋ Task 3（R1/R1b/R3b/R5）＋ Task 5（R6/R7/R8/R10/R11）＋ Task 6（R9） |
| §8.2 C1–C5 | 5 张编译期照片 | Task 2（C1/C2/C4）、Task 4（C3/C5） |
| §8.3 S1–S8 | 8 条源码面断言 | Task 8 Step 3 |
| §8.4 | 拍不到照片的地方 | `## 遗留` 第 2 节 |
| §9 | 明确不做 | Global Constraints 末段 |
| §10.1–§10.2 | 遗留与对账 | `## 遗留` 第一节（逐条承接 + 15 行对表；#12 落跨块第 4 条）、第四节（跨块对账） |
| §11 | 切分文档的错/缺口 | Task 7（`P6_EXPECTED_ROWS` 五行）、`## 跨计划前置` 第二节（行号按实测）、`## 遗留` 第一节第 3/4 条（上游归属） |

**设计节都有落点，但没有全部落成可执行证据**：§1、§2 是范围与依据，落在 Global Constraints 与
Task 8 Step 4；§12 是收口，落在本节的「不关闭任何规范未决项」。**明确无照片/无实现的三处**：
§4.4 的 §321↔§322 映射表（Task 8 Step 4 第 2 行、遗留第 12 条）、§3.6 第 7 行的 `Deref`/`DerefMut`
（由 S1 加查 `impl Deref` / `impl DerefMut`，不是独立照片）、§4.6 的更新入口与版本单调性（遗留第 2 条）。
`§10.1` 的 15 条在遗留第一节逐条承接（#12 落跨块第 4 条，对表里注明）；`§10.2` 的 12 条在遗留第四节、
Global Constraints 与各 task 中分列（其中 #7 的调度应用点补在跨块第 6 条）。

### 二、Step scan（每步一个动作 + 可核对结果）

- 每个 Step 都以一个动词开头（建、写、运行、加、提交、核），**没有「视情况实现」**。
- 每条测试命令都带 `TMPDIR="$PWD/.tmp"` 与 `timeout`；每条 `git add` 都是显式路径。
- 「运行，确认失败」的形态在 `## 各 task 的「运行，确认失败」一步量到的是哪一种失败` 表里逐 task 写明；
  编译失败照实标注，不冒充运行期红。
- 首次即绿的 task（Task 3／5／6／7）都写明了「若红，缺陷归谁、不许改宽照片」；Task 3／5 的真实红
  由 Task 8 的变异复核取，Task 6／7 的红由装配照片与并集变异取。
- 每个 task 的最后一个 Step 都是提交或明确的「仅当有改动」提交。

### 三、Type consistency（与设计逐条对名）

| 名字 | 设计节 | 本计划的形状 | 检查 |
|---|---|---|---|
| `ProtectedCore` | §3.5／§4.1 | 五私有字段 + 五个同名访问器 + `fingerprint()` | Task 2 |
| `ProtectedCoreParts` | §4.1 | 五 pub 字段，`seal` 的唯一输入 | Task 2 |
| `RuleCommitment` | §4.2 | `unavailable` / `committed` / `version` / `digest` / `matches` | Task 2 |
| `RuleVersion` | §3.6 | `new(u64) -> Option<Self>`、`get`；0 → `None` | Task 2/3 |
| `RuleDigest` | §3.6 | `to_hex`；`pub(crate) from_bytes` / `as_bytes`；公开构造仍只有 `committed` | Task 2/3 |
| `CoreFingerprint` | §4.2 | `to_hex`；不是第六个字段；不承诺跨版本稳定 | Task 2/3 |
| `ProtectedSemantic` | §4.4 | 五臂、`ALL`、`as_str`/`parse`、§321 原文 | Task 2/5 |
| `ProtectedError` | §4.1 | 恰好 `NothingCommitted` / `EmptyRules` | Task 2/3 |
| `LearningTarget` | §5.1 | 六臂、`ALL`、小写 `as_str`/`parse` | Task 4/5 |
| `ObservationKind` | §5.2 | 五臂、`ALL`、中文 `as_str`/`parse` | Task 4/5 |
| `LearningRecord` | §5.3 | 三 pub 字段 | Task 4 |
| `LearningProposal` | §5.3 | 四私有字段、`new(&ProtectedCore, …)`、四个 getter、`is_current_for` | Task 4/5 |

**没有第二套名字**：不出现 `AuthorityRules` / `ContractRules` / `LearningError` / `LearningClaim` /
`ProtectedCoreFingerprint` 一类；`continuum-learning` 没有错误类型。

### 四、Review Focus 归位

| Review Focus | 拥有代码的 task | 验证它的照片/步骤 |
|---|---|---|
| 1. 并集基线被同一提交改宽 | Task 7 | 八条断言的断言 4；Task 7 Step 5 与 Task 8 Step 5 的变异 1 |
| 2. P7 加 crate 时断言必红 | Task 7 | 用例注释与 fixture 文件头的生命周期声明；**实际到期的是断言 8**，断言 1 不会红；P7 的生命周期不在本计划 |
| 3. `subject: String` 携带受保护语义 | Task 4 | 字段类型与构造点；R7 **只证 target 交集为空**；应用侧义务见 `## 遗留` 第 1 条 |
| 4. `RuleVersion(0)` / 空字节未拒 | Task 2（实现）＋ Task 3（R1b/R3b） | R1b/R3b；Task 8 Step 5 的变异 3/4 |
| 5. 规则编码漏组 | Task 2（编码）＋ Task 3（R5） | R5 的五组表驱动；Task 8 Step 5 的变异 2 |

### 五、比例

- **8 个 task**：1 个骨架、2 个实现（Task 2／Task 4）、2 个照片（Task 3／Task 5）、1 个装配照片（Task 6）、
  1 个并集复核（Task 7）、1 个收尾（Task 8）。实现与照片分开是任务单的要求，不是重复：
  Task 3／Task 5 的照片是 Task 2／Task 4 的**审计**，红时回实现修，不另写第二份实现。
- **12 枚类型**（protected 8 + learning 4），与设计 §3.5 的冻结清单一一对应，不多不少。
- **18 张照片**：R1、R1b、R2、R3、R3b、R4、R5、R6、R7、R8、R9、R10、R11（13 张运行期）＋ C1–C5
  （5 张编译期）；S1–S8 是 8 条源码面断言，不是照片。设计 §8.1／§8.2 的每一张都有落点。
- **不建的东西**与设计 §1.2／§9 逐条对齐；没有「顺手加」的第六个字段、第二个错误类型、serde 派生、
  迁移或事件。

---

## 遗留/待协调

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。本计划不改设计，
以下各条只报出、不就地补。**

### 一、设计已自陈、本计划只承接的缺口（收件人照设计原文）

1. **`subject: String` 的应用侧检查义务**（设计 §5.5、§10.2 第 12 条）：一条
   `target: ExecutionMethod`、`subject: "Authority semantics"` 的提案编译得过；本块只保证「类型上不能把
   受保护语义当 target、crate 上不能命名其持有方」。**收到这种提案时，应用侧必须拒绝或升级，不得静默应用。**
   收件人：P1 ＋ 第 3 层执行器 ＋ 各策略持有方 ＋ 协调者。**规范未给判据。**
2. **§322 的「独立验证」与「用户批准」没有实现**（设计 §4.6、§10.1 第 2 条）：本块只落「显式版本化」的
   一半（`RuleVersion`）；两枚类型与归属未给。**本块也没有更新入口，故「新版本必须大于旧版本」这条
   单调性判定无处可判**（设计 §4.6）；今天只有封存时的初始版本 1。缺的是「更新入口及其单调性判定」
   这一步。收件人：协调者 ＋ 规范维护者 ＋ 边界层 ＋ 资源层。
3. **`authority_rules` 上游不存在**（设计 §4.3、§10.1 第 4 条）：`authority_limit` 是 opaque 契约字段，
   §292 的 Authority Host 属长期层、未建；本块的 R9 里它是 `Unavailable`。收件人：规范维护者 ＋ 长期层。
4. **`contract_rules` 上游不可引用**（设计 §4.3、§10.1 第 5 条）：`TaskContract` 在语义层，引用它要新增
   `语义层 (2) → 认知层 (6)` 的边（§9.1 无此边）；本块的 R9 里它也是 `Unavailable`。收件人：《工程》维护者 ＋
   协调者 ＋ P4。
5. **三组「已提交」只是词表摘要**（设计 §4.3、§10.1 第 6 条）：`capability` / `audit` / `effect` 的承诺
   覆盖枚举名，不覆盖 `mint` 的非空 scope、审计链哈希规则、策略六级；承诺能发现「词表变了」，发现不了
   「不变量被改了」。收件人：规范维护者 ＋ 各层持有方。
6. **ENG-001 的选择不自动关闭**（设计 §4.5、§10.1 第 1 条）：本设计取方向 (a)，缺的是「把这条选择记入
   《工程》§10.1 的 ENG-001 状态」这一步。收件人：协调者 ＋《工程》维护者。
7. **学习记录／提案没有落库**（设计 §6、§10.1 第 15 条）：`RUNTIME` 不是保护边界（设计 §6.1），
   P6C 的 `MemoryScope::RUNTIME` 不施加 §321；本块零表、不写 claim，学习记录与提案今天只在内存。
   **缺的是「落到哪（P6C 的 `RUNTIME` claim？独立表？）与谁写」这一步**——本计划不预设 `RUNTIME`。
   收件人：P6C ＋ 协调者。若最终落 `RUNTIME`，必须走 P6C 的 `apply_synthesis`，不另开写入口；那时才加
   `continuum-learning → continuum-memory` 的边并重跑并集复核。**规范未给判据。**
8. **本块没有生产调用方**（设计 §10.1 第 11 条）：`seal` 与 `LearningProposal::new` 的调用点要等执行器；
   本轮两个 crate 的入口只有用例。收件人：P1 ＋ 第 3 层执行器 ＋ 协调者。
9. **学习记录与 Evidence 的连接是弱引用**（设计 §5.3、§10.1 第 7 条）：`evidence_refs: Vec<String>` 与
   `subject: String` 都不是 P5a 的 `EvidenceId` / 目标对象的强类型。**缺的是「学习记录怎样引用 Evidence
   与目标对象」这一步。收件人：P5a ＋ 规范维护者。**
10. **执行历史没有代码边**（设计 §3.2、§10.1 第 9 条）：§6.3 要求
    `Runtime Learning ← 执行历史（第 3、4 层）`，但 §9.1 没有 `执行层 (3) → 认知层 (6)` 的边；本块以**值**
    传入，不登记执行层边。**缺的是「这条边从哪一侧登记」的裁定。收件人：《工程》维护者 ＋ 协调者。**
11. **§9.2 的「入度为零」与外部依赖**（设计 §3.4、§10.1 第 10 条）：`continuum-protected` 依赖
    `sha2` / `thiserror`；本设计按「§9.1 的八层组件」读，外部 crate 不算组件。**缺的是「外部依赖算不算
    入度」的读法确认。收件人：协调者**（与 P6C 的同形申报并列）。
12. **§321 五语义与 §322 五组不是双射**（设计 §4.4、§10.1 第 3 条）：`User approval requirements` 无专属
    字段，`Core security policy` 跨两组＋边界层策略层级。**缺的是「两张五元表的对应关系」这一条规范。
    收件人：规范维护者。规范未给判据。** 本计划不自造映射类型，也不把映射写进 API（Task 8 Step 4
    第 2 行标为「设计正文表，无代码照片」）。
13. **三张可学习表不一致**（设计 §2.2、§10.1 第 8 条）：§320 五、§216 六、§217 七；`Skill library` 只在
    §217；`Planning Policy` / `planner policy` / `planning heuristics` 三种写法。**缺的是「哪一张是规范表」
    这一步。收件人：规范维护者。规范未给判据。** 本块只按 §5.1 的六类落类型，不裁定这张表。
14. **§322 的五字段容纳不了 Context Compiler**（设计 §10.1 第 13 条）：若 C4 裁定「capability 判定在
    Compiler 内部」，Compiler 须成为安全边界；但 §322 的五个字段**没有一枚**能承载 Context 规则。
    **缺的是「加第六组，还是把 context 规则归入某一组」这一条规范级裁定。收件人：规范维护者 ＋ 协调者。
    规范未给判据。** 本设计不预设；本块也不加第六个字段。
15. **P6 的迁移表**（设计 §10.1 第 14 条）：本块零表、不占档；P6 的 `190` 由 P6C 写死（其设计 §3.3）。
    **缺的是「P6 档的合并表」这一步。收件人：协调者 ＋ P6C。**

**§10.1 十五条对表**（设计条号 → 本计划条目）：

| 设计 §10.1 | 本计划条目 |
|---|---|
| #1 ENG-001 回写 | 6 |
| #2 独立验证／用户批准 | 2 |
| #3 §321／§322 非双射 | 12 |
| #4 `authority_rules` 上游 | 3 |
| #5 `contract_rules` 上游 | 4 |
| #6 三组只是词表摘要 | 5 |
| #7 学习记录弱引用 | 9 |
| #8 三张可学习表 | 13 |
| #9 执行历史无代码边 | 10 |
| #10 外部依赖与入度 | 11 |
| #11 无生产调用方 | 8 |
| #12 P2 的例外 | 跨块 4 |
| #13 五字段容纳不了 Compiler | 14 |
| #14 P6 迁移表 | 15 |
| #15 学习记录没有落库 | 7 |

**一条不属 §10.1 的 Minor**：**学习记录没有撤回通道**（设计 §6.5）。本块零持久化，记录只在内存；
撤回要等 P6C／P0 的事务边界。收件人：P6C ＋ P0 ＋ 协调者。**不阻塞本块。**

### 二、拍不到照片的地方（设计 §8.4，逐条照原文，不另造）

- 五组规则的**内容**真的被保护：三组是词表摘要、两组是 `Unavailable`；能拍到「承诺与状态」，拍不到
  「规则的语义没被改」。
- §322 的「独立验证」与「用户批准」：没有类型、没有入口，拍不到因为不存在。
- 学习真的改善了未来任务：没有算法、没有应用、没有执行器；本块拍到的是「产物指不到受保护语义」。
- 权威核心的单一实例：本轮没有生产装配；`ProtectedCore` 只在 R9 里被 `seal`，不落库、没有全局单例。
- §9.4 那条边的读法：本块不裁定，两种读法下都不加执行层边。
- C4／OPEN-016 的裁定：本块与 P6A 同口径，但那是口径，不是裁定。

### 三、本计划自定的形状（申报，逐条说清代价）

1. **`P6_NAMES` 落成 `fn p6_names()`**（Task 7）：设计只要求「由 `P6_EXPECTED_ROWS` 派生，不写第三张名单」；
   常量无法在 `const` 里做 `map`，故用函数。**代价**：名字从常量变成函数；**收益**：单源不变，避免第三张名单。
2. **trybuild 的落点**（Task 2／Task 4）：设计 §8.2 没指定哪枚 crate 承载。本计划把 C1/C2/C4（只碰
   protected）放 `continuum-protected/tests/type_level.rs`，C3/C5（要同时碰 learning 与 protected）放
   `continuum-learning/tests/type_level.rs`。**代价**：两份 `trybuild` dev 依赖；**收益**：每份样例在
   它要证明的 crate 旁边。
3. **R9 的固定分隔符取 `\n`、不带尾随换行**（Task 6）：设计只说「固定分隔符拼接」，没给具体字节。
   `EffectState` 没有 `as_str`，七个串由用例内的 7 臂 `match` 给出（与私有 `persist::state_str` 同形）。
   **代价**：字节序列与七个状态串是本计划定的；**收益**：R9 的 `matches` 有一个可复现的输入；生产装配
   落位时**必须复用同一段生成逻辑**，不许另写一份。
4. **R5 的变异在五组上混用版本与摘要**（Task 3）：设计写「只改该组的 `version`（或 `digest`）」，两者二选一；
   本计划让第 1/3/5 行改版本、第 2/4 行改摘要。**代价**：五条变异的字段选择由本计划定；**收益**：覆盖
   版本与摘要两种编码字段，编码漏掉任一种都会红，而不是只覆盖一半。
5. **并集复核的解析器先剥 `//` 注释**（Task 7）：设计要求严格解析，但现有 `ALLOWED` 的注释里有一处
   `("continuum-runtime", &[…])` 字面；不剥注释会把注释当表项。**代价**：解析器多一步；**收益**：不误报，
   也不放宽语法（残余字符仍 `panic!`）。
6. **fixture 文件头三行**（Task 7）：设计 §7.3.1 给了两条声明与一个示例头；本计划把示例头的第一行也保留，
   共三行注释。**代价**：多一行注释；**收益**：27 行数据与「来源 `03e79c7`」在同一处可读。

### 四、跨块待协调（本计划只登记，不替它们改）

1. **`P6_EXPECTED_ROWS` 依赖 P6A／P6B／P6C 的 `ALLOWED` 行**：若三块定稿时改边表，先改其设计，再由 P6D
   改本表；**一次改动两处，不是两套表**（设计 §7.3.2、§7.4）。收件人：P6A／P6B／P6C 的设计者 ＋ 协调者。
2. **并集复核的合并门排期**：本块交付测试形状与两张对表；四块全部落地后由 P6D 在合并树上跑，记录
   `HEAD` 与输出。**缺的是「谁在合并评审时跑、把输出记在哪」这一步的排期**（设计 §10.2 第 9 条）。
   收件人：协调者。
3. **`crates/continuum-runtime/Cargo.toml` 是事实上的第四共写面**：四块各加自己的一行，彼此不删
   （设计 §10.2 第 11 条）。本块只加 `continuum-protected` 的 dev 依赖。收件人：协调者 ＋ P6A／P6B／P6C。
4. **P2 的拒绝点需要 P6B 的标签类型，但 P2 的 `ALLOWED` 行不得增枚**（设计 §10.1 第 12 条）：并集复核会把
   「P2 行加 `continuum-trust`」判红；这条例外不能靠改 `ALLOWED` 行解决。收件人：P6B ＋ P2 ＋ 协调者。
5. **P7 的生命周期**：P7 新增 crate 时**实际到期的是断言 8**（新增 subject 不在 `P6_NAMES` 里）；
   断言 1（27 行）**不会红**，因为 P7 不得往 P6 的 fixture 追加行。P7 起换新基线（连同
   `P6_EXPECTED_ROWS` / `P6_NAMES` 与 `len() == 5` 一起重建）或退役（设计 §7.3.5）。
   收件人：协调者（P7 起）。
6. **P7 的调度应用点**（设计 §10.2 第 7 条）：`scheduling heuristic` 是六类目标之一，但「何时学、何时应用、
   后台作业」在第 7 层；本块只有类型、没有应用者。**缺的是「调度提案的应用点」这一步。
   收件人：协调者**（P7 搁下期间，这一项只有类型、没有应用者）。

### 五、设计措辞的一处读法（不改设计，只报出）

- 设计 §8.1 的 R11 写「`as_str` / `parse` 各用**无通配臂**的穷尽 `match`」。`parse` 收 `&str`，表外串
  必须有一个分支，按字面无法成立。**本计划的实现是**：`as_str` 用无通配臂的穷尽 `match`；`parse` 用
  `ALL.iter().find(|x| x.as_str() == s)` 反查，不写第二张字符串表。R11 的编译期性质**只保证 `as_str`
  穷尽**；`ALL` 的完整性仍靠手写清单，**不是**「加臂必编译不过」——补了 `as_str` 却漏加 `ALL` 时编译
  照过（与 `continuum-effect` 的 `EffectState::ALL` 同形）。这是设计缺口，本块不引入宏 / `variant_count`。
  **收件人：设计作者 ＋ 复审者。**
