# P5b（执行方法库）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新建一个 crate `continuum-method`，落《工程》§8.1 第 10 行的 **Execution Method Library**
（`docs/02-工程.md:494`，规范依据 §187）：方法的登记形态（`MethodEntry { id, domain, realized_by }`）、
领域（`MethodDomain`，§187 的四个目录名）、方法标识（`MethodId`，开放）、两枚错误
（`MethodError { NotFound, Duplicate }`）与登记／选取入口
（`MethodRegistry::{new, seeded, register, resolve, bind, select}`），并按 §187 逐名建四份目录。

**Architecture:** 本 crate 对 workspace 内其它 crate 的**依赖集合为空集**——依据是《工程》§8.3 该行的
字面取值「`Execution Method Library ← 无`」（`docs/02-工程.md:526`）。落地形态有且只有两处：
`Cargo.toml` 里零条 workspace 内依赖，以及 `ALLOWED` 里的 `("continuum-method", &[])`，
后者由既有的逐对断言（`crates/continuum-runtime/tests/dependency_direction.rs:307-325`）守卫。
方法到算子的引用是**文本**（`realized_by: Vec<String>`），本 crate **不解析**它——这正是「依赖集为空集」的兑现方式
（设计 §3.2）：写错的 `realized_by` 照样编得过，核对它的责任在消费块。

**本块是索引，不是知识与执行。** `MethodEntry` 不带 `input_schema` / `output_schema` / `determinism` /
`side_effect_class` / `backend_candidates`（那些是 `Operator` 的字段，`crates/continuum-operator/src/definition.rs:79`）；
`MethodEntry` 也**没有**「怎样做得可靠」的文本字段（设计 §4.3 三条理由）。方法**不可执行**；
`Queued → Running` 的迁移与 `OperatorRegistry::resolve` 的调用点都在第 3 层（切分文档 §四第 4 条）。

**Tech Stack:** Rust 1.95.0 / edition 2024；外部依赖只有 `thiserror`（`MethodError` 的 `Display`）；
**workspace 内依赖零条**。**不使用 `trybuild`**（本块无可写的不可构造性断言，理由见
「关于本计划的代码块」）。

**设计依据：** `docs/superpowers/specs/2026-10-08-p5b-execution-method-library-design.md`（**唯一事实来源**）。
切分与归属：`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`。
**本计划不改设计、不改规范、不改代码以外的任何计划。**

---

## 跨计划前置（本节依赖谁已交付什么、把什么交给谁）

### 一、依赖的交付物与它们的**实际状态**（2026-10-10 实读工作树）

| 依赖 | 状态 | 本块用在哪 |
|---|---|---|
| **`crates/continuum-method`** | **不存在**（实测 `ls crates/`：无此目录） | 本计划的全部交付物 |
| `continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`）、`workspace_crates()`（`:234`）、逐对 `assert_eq!`（`:307-325`）与「每个成员都在表里」那一组断言（`:295-298`） | **已交付** | Task 1 加一行 `("continuum-method", &[])`；**那一行不能省**（`:295-300` 漏了即红） |
| workspace `Cargo.toml` 的 `[workspace] members` | **已交付** | Task 1 加 `"crates/continuum-method"` |
| `continuum-operator` 的 `OperatorError`（`registry.rs:7-12`）、`OperatorRegistry`（`:15`）与它的 `register` / `resolve`（`:24`、`:36`） | **已交付** | **只作形状参照**（设计 §4.4「同形」）；**不登记依赖边**，本 crate 一处都不 `use` 它 |
| `continuum-artifact` 的 `ArtifactType::{ALL, as_str, parse}`（`artifact.rs:28`、`:47`、`:68`） | **已交付** | **只作形状参照**（判据 2 的两处穷尽 `match`）；**不登记依赖边** |
| `continuum-graph` 的 `OperatorRef`（`node.rs:42`，经 `src/lib.rs:24` 再导出） | **已交付** | **刻意不用**（设计 §3.2 第 1、2 条：用它的直接边是 `continuum-graph`，即 EML 被要求从中分出来的那一侧） |
| P5c／P5d／P5e／P5f 的四份设计 | **已定稿**（`docs/superpowers/specs/2026-10-09-p5{c,d,e,f}-*-design.md`） | 它们是 `bind` 的消费方；P5c 已在 §7.1 给出具名入口 `bind_code_methods` |
| P4 的三个 crate（`continuum-canonical` / `continuum-semantics` / `continuum-budget`） | **不存在**（实测 `ls crates/`） | **与本块无接口**（设计 §11：P4 无接口面） |

**一句话结论**：本块**依赖面为空集**——不因任何其它块未合入而编不过，也不为任何其它块新开一条边。
这是《工程》§8.3 那一行的直接兑现，不是巧合。

### 二、四个算子块与本块的接口（**本计划只作提供方写，一处都不实施**）

- **`bind` 是四块填内容的唯一入口**（设计 §4.5、第六节）。**P5c 的具名入口已由其设计给出**：
  `bind_code_methods(&mut MethodRegistry) -> Result<(), MethodError>`
  （`docs/superpowers/specs/2026-10-09-p5c-code-domain-operators-design.md` §7.1）。
  本计划**不代它写、不预演它的内容**，只在 Task 3 把 `bind` 的签名与「重复 bind 覆盖旧值、只碰 `realized_by`」的语义钉住。
- **P5f 没有 `bind` 目标**：§187 里没有 `image/` 目录（设计 §8 R3 的实测：
  `grep -n "image/\|images/" docs/spec/ docs/01-总纲.md docs/02-工程.md` 零命中），
  `MethodDomain` 无 image 取值。**本计划不发明它。**
- **判据 6 的「有对家域的条目除名册外逐条非空」这一半，本 crate 闭不了**：`seeded()` 之后四块的 `bind`
  尚未发生，故在本 crate 里「空」与「漏 bind」不可区分（设计 §4.5 末、§10 判据 6 第 2 条明写）。
  本计划把**能闭的那一半**（名册恰两条、两侧都钉、`3d/` 条目为空且域谓词为假）写进 Task 4，
  把**闭不了的那一半**写进 `## 交付给谁`，**不假称它已覆盖**。

### 三、本节把什么交给谁（逐条见 `## 交付给谁`）

- **交给 P5c／P5d／P5e**：判据 6 的「有对家域逐条非空」那一半，以及 `realized_by` 文本内容的核对
  （本 crate 看不到 `OperatorRegistry`，设计 §3.2 末三条）。
- **交给 P5f**：无 `bind` 目标这一事实（设计 §8 R3）。
- **交给协调者**：设计 §8 的 R2／R3／R9 三条待裁项。
- **交给规范维护者**：设计 §8 的 R8（《工程》§9.2 的入度为零表独缺层 8 那一行）。
- **交给 P4 的设计者与协调者**：设计 §8 的 R4（「领域」从何而来无规范来源）、R5（`select` 的消费点未定）。
- **交给复审者**：设计 §8 的 R1（`realized_by` 取文本的读法）与 R8。

**本计划一处都不替它们补写。**

---

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- **依赖方向：本计划新增的 workspace 内边只有登记的那一条「无」**——`ALLOWED` 加 `("continuum-method", &[])`。
  **空数组是硬断言**：任何**直接的**反向边（含指向 `continuum-graph` 者）都会让
  `every_crate_depends_only_on_its_allowed_set` 变红（设计 §9）。
  **这条规则只钉直接边，这就是它的完整粒度，本计划不另设闭包断言**：`cargo_tree_direct` 带 `--depth 1`
  （`dependency_direction.rs:270-285`），它看不到传递可达——而 Rust 的 crate 可见性要求**直接声明**才能 `use`，
  未写进 `Cargo.toml` 的传递依赖写不出 `use continuum_graph::…`。**不把这句读成「任何指向 graph 的边都会变红」**——
  那会是一条假保证。
- **`Cargo.toml` 只声明本 crate 实际用到的外部依赖**：`thiserror`（`MethodError` 上派生 `Error`）。
  **不声明 `serde`**（本 crate 不序列化任何东西）、**不声明任何 workspace 内 crate**。
  外部依赖不进 `ALLOWED`——该表逐对断言的是 workspace 成员之间的边。
- **枚举的字符串编码取小写、多词以 `_` 连接**，写在**枚举自己身上的显式函数**（`as_str` / `parse` 与类型同址），
  不依赖 serde、不用 `Debug`。表外取值读回得 `None`，**不取默认臂**。
  本块要编码的是 `MethodDomain` 的四个目录名，四个串**逐字**取自 §187（`docs/spec/04-method.md:117`、`:126`、`:132`、`:138`）。
- **代码注释、错误信息、测试断言信息用中文。** 标识符用英文。
- `cargo test --workspace --no-fail-fast` 必须全绿、**0 warning**；`cargo build --workspace --all-targets` 同样 0 warning。
- **门读数的判据是覆盖，不是「没报错」**：全量命令**不收 `--keep-going`**，要用 **`--no-fail-fast`**——
  否则在第一个失败目标处停住、后面的目标一行不跑。**读数时数两样**：日志里 `Running` 的行数
  与 `test result:` 的行数（**Doc-tests 算一条**），并核它们与 `cargo test --workspace --no-fail-fast -- --list`
  给出的目标数一致。`Compiling` 字样挡不住射程与截断。
- **不修改用户目录的权限位。** 不在仓库中写入任何凭据。
- **不要用 `git add -A`，不要 `git commit --amend`。** 只 `git add <显式路径>`。
  **新增 crate 时 `Cargo.lock` 会随之变化，须一并提交锁文件**——各 task 的显式路径清单只列源码与清单，锁文件按本行办。
- **范围与设计一致，不多做不少做。** 明确不做的事，逐条列出，免得被读成漏项：
  - **不定义算子、不注册方法进 `OperatorRegistry`、不 `use` `continuum-operator`**（设计 §二第 1 条、§3.2）。
  - **不建执行路径**：不调 `OperatorRegistry::resolve`、不碰 `Queued → Running`（切分文档 §四第 4 条）。
  - **不碰 `Node.execution_policy` / `Node.verification_policy`**（切分文档 §四第 5 条）。
  - **不定义 `Evidence` / `VerificationProfile` / Requirement Coverage / Completion Predicate**（P5a）。
  - **不扩展 `ArtifactType`、不建领域 Operator 集**（P5e／P5c–P5f）。
  - **不定义「方法的领域如何判定」**（设计 §8 R4：无规范判据）。
  - **不为「同一 id 被两块写」加检测**（设计 §5.1、R9：一对一成立时它无处可触发）。
  - **不给 `MethodEntry` 加非算子承载、不加次序维度、不恢复 `purpose`**（设计 §4.3、§10 判据 6 的否掉的两案）。
  - **不发明 `image/` 目录、不认领 `Scene`、不替 `3d/` 发明对家**（设计 §7、§8 R2／R3）。
  - **不使用 `trybuild`**（本块无不可构造性断言）。
  - **不改 `docs/spec/**`、`docs/02-工程.md`、`docs/01-总纲.md`、任何设计文件、任何其它计划。**

### 本计划特有的「签名即判据」，照抄前须对源

- **`OperatorError` 的形状**（`crates/continuum-operator/src/registry.rs:7-12`）：
  `#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]`，两枚变体各带 `#[error("…")]` 格式串。
  `MethodError` 照此形状（设计 §4.4「同形」），**但载荷只有 `MethodId` 一项**（`OperatorError` 是 `id` + `version` 两项）。
- **`OperatorRegistry` 的判定形状**（同文件 `:15`、`:24`、`:36`）：`entries` 私有字段；`register` 先 `contains_key`
  判重、命中返回 `Duplicate` 且**不插入**；`resolve` 用 `get(...).ok_or_else(|| NotFound { … })`。
  `MethodRegistry` 的三处判定照此形状。
- **`ArtifactType::as_str` / `parse` 的形状**（`crates/continuum-artifact/src/artifact.rs:47`、`:68`）：
  `as_str` 是**穷尽 `match`、无通配臂**；`parse` 是 `Some(match s { …, _ => return None })`。
  `MethodDomain` 的两处照此形状。
- **`dependency_direction.rs` 的登记形状**（`:25` 起的表、`:289-325` 的两条断言）：
  条目形如 `("crate-name", &[…])`，**数组按字母序**；`:295-300` 要求 `ALLOWED` 与 `workspace_crates()` **互为覆盖**。
  新条目的数组是**空数组**，故字母序无约束。
- **`MethodId` 的 `Display` 是必需的**：`MethodError` 的格式串以 `{id}` 引用该字段，理由与写法同
  `crates/continuum-operator/src/definition.rs:37-42`（那段注释与 `impl Display` 本体）。

### 三条已付过代价的纪律（照抄 P4 那份，按本块实情改写）

1. **变异必须在全量 `cargo test --workspace --no-fail-fast` 下得出否定结论**（「不变红」）；
   正向的「变红」跑全量是加分。**变异分四档，每一处「预期谁红」都要标档位**：
   **取反**（把判定反过来）／**放宽**（少判一半条件）／**收紧**（多判一半条件）／**移除**（删掉整条守卫）。
   **三条失效形态都要防**：
   (a) **锚点不唯一** → 变异没落到实现体却报 GREEN；
   (b) **等价变异体**——判据是「**这两版在哪个入参上会给出不同结果**」，举不出即是等价，
   处理是**换真变异体而非补用例**；
   (c) **变异导致编译失败**——那不是「变红」（判据用 `could not compile` 或 `error[E….`；
   cargo 在**用例失败**时也打印 `error: test failed, to rerun pass …`，不能拿它当判据）。
   **`..` 豁免会让变异成为等价变异体**：凡靠穷尽解构钉的清单，变异时若在被改处留了 `..`，它照过。
   **本块已预先识别的等价变异体有一处**：`MethodDomain::ALL` 的**元素顺序**——若用例只断「集合相等」，
   「把 `ALL` 的两项对调」是等价的。故 Task 2 的往返用例**逐值断言**而不是对 `ALL` 做集合比较，
  并**另有一条**断言 `ALL` 的次序。
   **本块的编译期照片有两处**（**照实写成编译期照片，不假称它们会跑红**）：
   `MethodDomain::as_str` / `parse` 的穷尽 `match`（加第五个目录时编译失败）、
   `MethodError` 的两枚变体（加第三枚时穷尽解构处编译失败）。这两处**没有运行期红**。
2. **变异脚本必须带还原护栏**（本仓出过一次「变异留在源码里」的事故）：
   每次变异**用 `trap` 装还原**、**变异前与还原后各核一次 `sha256sum`**、**每轮用独立日志路径**，
   报告里逐轮附「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径」。模板（`MUT` 是改的文件、`BAK` 是备份）：
   ```bash
   cd /home/DslsDZC/Continuum
   before=$(sha256sum "$MUT" | cut -d' ' -f1)
   cp "$MUT" "$BAK"
   trap 'cp "$BAK" "$MUT"; echo "已还原"' EXIT INT TERM
   # …施加变异…
   TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast 2>&1 | tee "$LOG"
   # …读 $LOG 判红…
   ```
   **`trap` 那一行不许省。**
3. **凡注释写绝对措辞，必须有对应用例；且要带时点**——「唯一／一律／只有／全部／没有任何」这类词，
   要么有可写出的照片，要么**明写为什么没有**。**断言的作用域要与事实同宽**：
   「本 crate 的源码文本里零命中」≠「全仓零命中」；「在本 task 结束时」≠「永远」。
   **枚举式绝对断言须逐项有照片**——「A/B/C/D 都…」的每条臂各要一条照片，不抽代表。
   **本块的绝对措辞落点**（逐处见各 task 的用例）：
   - 「§187 的四个目录**逐名**照录」→ Task 4 的四份目录逐名断言（十八个名各是照片的一条）；
   - 「`UNREALIZED_BY_DESIGN` **恰为**两条」→ Task 4 的 roster 用例（字面期望，两侧都钉）；
   - 「`has_counterpart` 对 Software／Video／Research **各**为真、对 ThreeD 为假」→ Task 2 的**逐臂**断言；
   - 「`parse` 对四值之外的输入**一律** `None`」→ Task 2 的逐串断言（含 `"image"`）。
   **自指计数不写数值**：本计划不写「本计划共 N 条用例」「本计划共 N 个 task」这类句子。
4. **失败路径的测试要断言是哪一种 `Err`**，不只「返回了 `Err`」；**并断言没有半写的副作用**。
   本块的失败面只有两枚：`MethodError::Duplicate`（`register` 撞同名）与 `MethodError::NotFound`
   （`resolve` 未注册、`bind` 未注册）——逐变体至少一条用例；副作用面是「撞名时不覆盖既有条目」与
   「`bind` 未注册时不新建条目」（Task 3）。
5. **「守卫」要两侧都钉；缺的那侧往往是 fail-open 的那侧。** 本块逐处标出「另一侧」是哪一个用例：
   `register` 的「不覆盖」、`bind` 未注册时的「不新建条目」（两处都在 Task 3，且都是 fail-open 侧）、
   名册的「多一条也红」（Task 4）。
6. **「删掉 X 即红」要先问「删掉之后行为真的变了吗」**——`ALL` 的次序、`HashMap` 的迭代序、
   可推断的字面量都会让它成为**等价变异体**（见第 1 条）。凡本计划标了「移除档」的地方，都已先答过这一问。

**另两条运行纪律**：跑测试加 `timeout`（本机 `TMPDIR` 在 FUSE 类挂载上，I/O 曾挂起），
**命令的管道结尾不要接 `tail`**（退出码会被 `tail` 吃掉）；若报「在等后台任务」，先核进程与日志。

**临时目录的用法**：`TMPDIR` 取**仓库内的 `.tmp/`**（`TMPDIR="$PWD/.tmp"`），不要用系统默认的那个。
收工前用 `chmod -R u+rwX .tmp && rm -rf .tmp`。**`.tmp/` 不入库**，但只按显式路径 `git add` 就不会误提交。

**变异日志是证据，必须活到复审结束**：**实现者保留 `.tmp/`，由协调者在复审结束后清理**。
报告里**不要**引用 `.superpowers/` 之类 gitignore 的路径作为任何东西的唯一来历。

**变异窗口与验证窗口互斥**：实现者与协调者**共用同一个工作区**，而变异是「改源码 → 跑全量 → 还原」。
**实现者报告完成之前，协调者不得在该工作区里跑 cargo**。

**变异口径分层**：变异**条数**按「有多少条**互不相同**的守卫」定，不按分支数定，且分两档、不许混成一句「通过」：
- **承重守卫 → 全量套件**：两侧对钉的守卫、**fail-open 的那一侧**、失败路径**判别哪一种 `Err`**、
  **跨 crate 才可见的效果**。**本块只有一条跨 crate 观察点**：`ALLOWED` 与实际依赖一致——
  它的红**只在 `continuum-runtime` 的 `dependency_direction` 里可见**，故变异
  「给 `crates/continuum-method/Cargo.toml` 加一条指向 `continuum-graph` 的依赖」必须跑全量
  （或至少 `cargo test -p continuum-runtime --test dependency_direction`）。
- **其余分支 → `continuum-method` 的包级套件**（`cargo test -p continuum-method --no-fail-fast`），
  报告里须**标明证据强度较低**并列出「这一条可能漏掉的跨 crate 观察点」。

---

## 关于本计划的代码块

本项目的既有事实：**手写的计划代码块错误率很高**，且已确立「**代码块是示意，正文的措辞才是约束**」。
因此本计划只给**类型签名、常量取值与关键判定**，**不给整段可粘贴实现**；
凡与既有 crate 交互的形状（`OperatorError` 的派生与格式串、`ArtifactType::as_str` / `parse` 的穷尽 `match`、
`dependency_direction.rs` 的条目形状），实现前须先读该处源码确认，不符时以源码为准并回报。

**本计划不使用 `trybuild`，理由写明**：本块的两处编译期性质（`MethodDomain` 的穷尽 `match`、
`MethodError` 的变体数）都是**穷尽性**断言，不是**不可构造性**断言——`MethodEntry` 的三个字段全 `pub`，
`MethodRegistry::entries` 私有但设计没有对它作任何「crate 外不能构造」的承诺，
故没有可写的 `compile_fail` 样例。**凡本计划写「加一个目录／加一枚变体会编译失败」处，那是编译期照片**，
各 task 的用例里已逐处标明，**不假称它会跑红**。

**Step 2「运行，确认失败」在本块的形态是编译期失败**：各 task 的首跑都是
「该包尚不存在、或新符号尚未定义 ⇒ `could not compile`」。
**照实记为编译期失败，不假称它是运行期红。** 运行期的红由每条用例的「红条件」行给出，
那是**变异判据**，不是首跑判据。**唯一的例外是 Task 1 Step 3b 的依赖门**——它的首跑是真的**运行期断言红**。

---

# 文件结构

```
crates/continuum-method/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/id.rs           MethodId（开放 newtype）
  src/error.rs        MethodError（两枚变体）
  src/domain.rs       MethodDomain、ALL、as_str、parse、has_counterpart
  src/entry.rs        MethodEntry
  src/registry.rs     MethodRegistry::{new, register, resolve, bind, select}
  src/catalog.rs      MethodRegistry::seeded、四份目录、UNREALIZED_BY_DESIGN
  tests/identity.rs   MethodId 与 MethodError 的形状
  tests/domain.rs     判据 2 与判据 6 的域级侧
  tests/registry.rs   判据 4、5 与 bind 的覆盖语义、两处 fail-open 侧
  tests/catalog.rs    判据 3：四份目录逐名与条数
  tests/roster.rs     判据 6 的条目级侧：名册恰两条、两侧都钉
```

**本计划要改的既有文件**（只有 Task 1 碰它们）

```
Cargo.toml                                              [workspace] members 加一行
crates/continuum-runtime/tests/dependency_direction.rs  ALLOWED 加 ("continuum-method", &[])
```

**本计划不碰的文件**：`crates/continuum-operator/**`、`crates/continuum-artifact/**`、
`crates/continuum-graph/**`、`crates/continuum-port/**`、`crates/continuum-core/**`、
`crates/continuum-runtime/src/**`、`crates/continuum-runtime/tests/**`（除 `dependency_direction.rs` 那一行）、
`docs/**`（本计划自身除外）。

---

### Task 1: `continuum-method` 骨架、`MethodId`、`MethodError` 与两处登记

**Files:**
- Create: `crates/continuum-method/Cargo.toml`
- Create: `crates/continuum-method/src/{lib.rs,id.rs,error.rs}`
- Create: `crates/continuum-method/tests/identity.rs`
- Modify: `Cargo.toml`（`[workspace] members` 加一行）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 加一行）

**Interfaces:**
- Produces: `continuum_method::{MethodId, MethodError}`
- Consumes: 无（本 crate 的 workspace 内依赖集合为空集）

- [ ] **Step 1: 写用例**

`tests/identity.rs`：

- `method_id_returns_the_string_it_was_built_from`：`MethodId::new("TDD").as_str() == "TDD"`；
  **另取一个含非 ASCII 与大小写混合的串**（如 `"planning/规划"`）断言逐字往返。
  红条件：`as_str` 返回常量、或对串做任何折叠／截断（**取反档**）——含非 ASCII 那一枚即红，
  **故夹具必须含非 ASCII**，否则这条与索引序一样是等价的。
- `method_error_renders_its_id_in_the_message`：`MethodError::NotFound { id: MethodId::new("nope") }.to_string()`
  **含子串 `"nope"`**；`MethodError::Duplicate { id: MethodId::new("dup") }` 同形。
  红条件：格式串改写成不带 `{id}` 的固定文案（**移除档**）——两枚变体照旧编译得过，只有本用例红。
  **本条钉的是 `MethodId: Display` 的必需性**，来历同 `crates/continuum-operator/src/definition.rs:37` 那段注释。
- `method_error_has_exactly_two_variants`：**穷尽解构、不带 `..`**（对一枚 `&MethodError` 写两臂 `match`）。
  红条件：**加第三枚变体** ⇒ 本处**编译失败**。**照实标注为编译期照片**（纪律 1(c)：编译不过不算变红）；
  **本条没有运行期红**。
- `method_error_variants_are_distinguishable`：两枚变体各构造一枚、`assert_ne!` 断言两者不相等。
  红条件：把两枚折叠成同一个值（**取反档**）。**本条的作用是给上一条补一个运行期可观察面**——
  上一条只有编译期照片，本条的相等性判定在运行期成立。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-method --test identity
```

预期：**`could not compile`**（`continuum-method` 这个包尚不存在）。**这是编译期失败，不是运行期红**，照实记录。

- [ ] **Step 3: 建骨架**

`crates/continuum-method/Cargo.toml` 的依赖：**只声明 `thiserror`**（workspace 依赖）。两处落点：
- `MethodId` 是包 `String` 的 newtype，内部串私有，构造入口只有 `new`。
  **派生 `Debug, Clone, PartialEq, Eq, Hash`**——设计 §4.2 的签名列里没有列派生，这四枚是由**两处必需**倒推的：
  `MethodRegistry` 的 `HashMap<MethodId, MethodEntry>` 要 `Eq + Hash`，`MethodError` 的
  `PartialEq / Eq / Clone` 派生要 `MethodId` 有同派生（**本计划自定**，记在 `## 遗留` 第三节）。
  **不加 `Ord` / `PartialOrd`**（`select` 的排序键取 `as_str`，见 Task 3）、**不加 serde 派生**（本 crate 不序列化）。
- `MethodError` 照 `OperatorError`（`registry.rs:7-12`）的形状：`#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]`，
  两枚变体各带 `#[error("…")]` 格式串，格式串以 `{id}` 引用 `MethodId`（故须有 `Display`）。

`crates/continuum-method/src/lib.rs` 导出本 task 的两个类型与 crate 文档；`mod id; mod error;` 逐 task 增量加。

- [ ] **Step 3b: 两处登记（这一步有真的运行期红）**

**次序不许颠倒**：

1. 先只改 `Cargo.toml` 的 `[workspace] members`（加 `"crates/continuum-method"`），**不动 `ALLOWED`**，跑：

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：`every_crate_depends_only_on_its_allowed_set` **运行期断言红**，文案是
「workspace 成员 continuum-method 未列入 ALLOWED，它的依赖方向不会被检查」（`dependency_direction.rs:295-298`）。
**这是本计划唯一一处「首跑即运行期红」**——设计 §9 明写这个次序是刻意的，不要靠「先不加 member」绕过。

2. 再在 `ALLOWED` 里、`("continuum-node", &["continuum-artifact"])` 那一条**之后**加：

```rust
    // P5b 执行方法库（设计 §9；《工程》§8.3 `docs/02-工程.md:526`「Execution Method Library ← 无」）。
    // **空数组是硬断言**：任何直接的反向边（含指向 continuum-graph 者）都会让本门变红——
    // 这正是「realized_by 取文本而非 OperatorRef」那条读法的**编译期照片**（设计 §3.2、§9）。
    // 该照片只覆盖依赖边，**不覆盖 realized_by 的文本内容**（那在 Vec<String> 里，编译期无从核）。
    // 本门查的是直接边（--depth 1），故对 continuum-operator 的传递依赖不出现在这张照片里。
    ("continuum-method", &[]),
```

3. 重跑同一条命令，预期全绿。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
git add crates/continuum-method Cargo.toml Cargo.lock crates/continuum-runtime/tests/dependency_direction.rs
git commit -m "feat(method): continuum-method 骨架、MethodId 与 MethodError"
```

---

### Task 2: `MethodDomain`——§187 的四个目录名（封闭枚举）

**Files:**
- Create: `crates/continuum-method/src/domain.rs`
- Create: `crates/continuum-method/tests/domain.rs`
- Modify: `crates/continuum-method/src/lib.rs`（导出）

**Interfaces:**
- Consumes: 无
- Produces: `continuum_method::MethodDomain` 与它的 `ALL` / `as_str` / `parse` / `has_counterpart`

- [ ] **Step 1: 写用例**

`tests/domain.rs`（判据 2 与判据 6 的域级侧）：

- `every_domain_round_trips_through_its_encoding`：**逐值**（不抽代表）对 `ALL` 的每一枚断言
  `parse(d.as_str()) == Some(d)`；并断言 `ALL.len() == 4`（字面量），
  以及 `as_str` 的四个串**逐字**是 `"software"` / `"video"` / `"research"` / `"3d"`。
  红条件（解码侧，**取反档**）：把 `parse` 的 `"3d"` 臂删掉或指向别的目录 ⇒ **该值的往返红**（运行期）。
  红条件（编码侧，**取反档**）：把 `as_str` 的两臂返回同一个串 ⇒ 其中一个值的往返红。
  **`"3d"` 单列一条逐字断言**：它是四条里唯一可能被顺手写成 `"3D"` 的一条。
  **不写的断言**：对 `ALL` 做集合比较——那与元素次序无关，是纪律 1 的等价变异体。
- `all_is_in_the_order_of_the_specs_directories`：断言 `ALL` 的次序**逐位**是
  `[Software, Video, Research, ThreeD]`（§187 原文的目录次序：`docs/spec/04-method.md:117`、`:126`、`:132`、`:138`）。
  红条件：对调 `ALL` 的任意两项（**取反档**）⇒ 红。**本条是上一条「集合比较缺口」的补丁**。
- `parse_rejects_every_string_outside_the_catalog`：**逐串**断言 `None`——含 `""`、`"Software"`（大小写）、
  `"3D"`、`"3d "`（尾空格）、`"software/"`（带斜杠的目录写法）、`"image"`、`"images"`。
  红条件（**放宽档**）：给 `parse` 加一条通配臂 `_ => Some(MethodDomain::Software)` ⇒ 逐串红。
  红条件（**收紧档**）：把 `"image"` 加成一个新臂（**本块未裁决，不得发明**）⇒ 该串那一条红。
  **`"image"` 的来历写在断言的注释里**：它是设计 §8 R3 点名的缺口（§187 没有 image 目录而 P5f 需要它），
  不是为了凑数加的样例串。
- `has_counterpart_is_true_for_each_paired_domain_and_false_for_three_d`：**逐臂四条断言**——
  `Software` / `Video` / `Research` **各一条**为 `true`，`ThreeD` 一条为 `false`。
  红条件（**取反档**）：`ThreeD => true` ⇒ 第四条红；把三枚真值里的任一枚改成 `false` ⇒ 对应那条红。
  **本用例是判据 6 的域级侧**（配对：`software/`↔P5c、`research/`↔P5d、`video/`↔P5e，`3d/` 无对家，设计 §5.1）。
  **注释里写明它是本设计的决定而不是规范断言**（设计 §4.1 末：来源是当前分块）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-method --test domain
```

预期：**`could not compile`**（`MethodDomain` 尚未定义）。编译期失败，照实记录。

- [ ] **Step 3: 实现**

四处落点：
- `MethodDomain` 四臂（`Software` / `Video` / `Research` / `ThreeD`），派生 `Debug, Clone, Copy, PartialEq, Eq, Hash`。
- `pub const ALL: [MethodDomain; 4]`，次序照 §187 原文。
- `as_str`：**穷尽 `match`、无通配臂**。
- `parse`：`Some(match s { …, _ => return None })`，四种外一律 `None`，**不取默认臂**。
- `has_counterpart`：**穷尽 `match`**，`Software` / `Video` / `Research` 为 `true`，`ThreeD` 为 `false`。

**类型的文档注释要写明封闭的来历**：§187 在列目录前写了「例如：」（`docs/spec/04-method.md:114`），
《总纲》§8.1 又以四行界定（`docs/01-总纲.md:1331-1336`）——**没有规范说这四个是穷尽的**；
取封闭是为了给四个算子块一个共同的词汇表（设计 §4.1），**这是设计决定，不是规范断言**。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-method
git commit -m "feat(method): MethodDomain 的四个目录名与 has_counterpart"
```

---

### Task 3: `MethodEntry` 与 `MethodRegistry` 的登记／解析／绑定／选取

**Files:**
- Create: `crates/continuum-method/src/{entry.rs,registry.rs}`
- Create: `crates/continuum-method/tests/registry.rs`
- Modify: `crates/continuum-method/src/lib.rs`（导出）

**Interfaces:**
- Consumes: Task 1 的 `MethodId` / `MethodError`；Task 2 的 `MethodDomain`
- Produces: `continuum_method::{MethodEntry, MethodRegistry}` 与
  `MethodRegistry::{new, register, resolve, bind, select}`（`seeded()` 在 Task 4 加）

- [ ] **Step 1: 写用例**

`tests/registry.rs`。夹具用**手建**的 `MethodEntry`——本 task 尚无目录，故不依赖 `seeded()`：

- `register_rejects_a_second_entry_with_the_same_id`：先 `register(A)`，再 `register(A')`（同 `id`、不同 `domain` 与
  `realized_by`）⇒ `Err(MethodError::Duplicate { id })`；**并断言第一次登记的那条未被覆盖**
  （`resolve(&id)` 读回的是 A 而不是 A'）。
  红条件（**移除档**，fail-open 侧）：把 `contains_key` 判定删掉、直接 `insert` ⇒ 第一次 `register` 变 `Ok`
  且 `resolve` 读回 A' ⇒ **两条断言都红**。
  **「另一侧」是不覆盖**：只断「返回了 `Duplicate`」会漏掉「同时也把旧值覆盖了」。
- `resolve_returns_not_found_for_an_unregistered_id`：`resolve(&MethodId::new("nope"))` ⇒ `Err(MethodError::NotFound { id })`。
  红条件（**取反档**）：改成返回 `Ok`（panic 不是本用例要的错型）⇒ 红。
- `bind_returns_not_found_for_an_unregistered_id_and_creates_nothing`：
  `bind(&MethodId::new("nope"), &["op"])` ⇒ `Err(NotFound { id })`；
  **并断言这次调用之后 `resolve(&MethodId::new("nope"))` 仍是 `NotFound`**（没有半写：`bind` 不新建条目）。
  红条件（**放宽档**，fail-open 侧）：让 `bind` 对未注册 id 走 `entry(id).or_default()` 一类的插入路径 ⇒ 后一条断言红。
  **这是本 task 的第二处 fail-open 侧**：只断「返回了 `Err`」会漏掉「顺手把条目建出来了」。
- `binding_twice_overwrites_the_previous_value`：同一 id 先 `bind(&["a"])` 再 `bind(&["b", "c"])`，
  断言 `realized_by` **等于** `["b", "c"]`（不是 `["a", "b", "c"]`）；**并断言 `id` / `domain` 两个字段未变**。
  红条件（**取反档**）：把覆盖改成追加 ⇒ 第一条断言红。
  **本条钉的是设计 §5.1 的覆盖语义**：一对一配对下覆盖不丢别人的贡献（设计 §4.5「重复 bind 覆盖旧值」）；
  改成追加会让「同一块重复调它」变成只增不减。**「`bind` 只碰 `realized_by`」由第二条断言钉住。**
- `select_returns_only_the_entries_of_the_requested_domain`：建三条——`(software, a)`、`(software, b)`、`(video, c)`——
  `select(Software)` 的 id 集合**恰为** `{a, b}`（逐名，不是「非空」）。
  红条件（**放宽档**）：把 domain 过滤删掉 ⇒ 红。
- `select_is_ordered_by_id_ascending`：以**逆字母序**插入 `(software, "gamma")`、`(software, "beta")`、`(software, "alpha")`，
  断言 `select(Software)` 的 id **逐位**是 `["alpha", "beta", "gamma"]`。
  红条件（**取反档**）：把排序键取反 ⇒ 逐位断言红。
  **并写明「移除排序」是弱变异体**：`HashMap` 的迭代序不是插入序，删掉排序后本用例**极可能**红但**不保证**
  （小容量下偶然升序即等价变异体）——**故取反档才是本条的判据**。
  **「同一 registry 两次 `select` 给出同一序列」不单列断言**：同一进程内不排序的 `HashMap` 迭代序也稳定，
  那条断言抓不到任何东西（等价变异体，纪律 6）。**设计 §10 判据 5 的「按 id 升序」这一半由本用例兑现，
  「两次同序」那一半据实记为无判据的加强句**（写进 `## 遗留`）。
- `select_of_a_domain_without_entries_is_empty`：对一个只含 software 条目的 registry 调 `select(ThreeD)` ⇒ 空 `Vec`
  （不 panic、不造条目）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-method --test registry
```

预期：**`could not compile`**（`MethodEntry` / `MethodRegistry` 尚未定义）。编译期失败，照实记录。

- [ ] **Step 3: 实现**

- `MethodEntry { pub id: MethodId, pub domain: MethodDomain, pub realized_by: Vec<String> }`——三字段全 `pub`
  （设计 §4.3），派生 `Debug, Clone, PartialEq, Eq`。
  **不带** `input_schema` / `output_schema` / `determinism` / `side_effect_class` / `backend_candidates`
  （那些是 `Operator` 的字段，`crates/continuum-operator/src/definition.rs:79`）；
  **不带** `purpose`（设计 §4.3 三条理由；本设计近期删过这个字段，来历见 `## 遗留` 第四节）。
- `MethodRegistry`：`#[derive(Debug, Default)]`，私有字段 `entries: HashMap<MethodId, MethodEntry>`；
  `new()` 取 `Self::default()`（同 `OperatorRegistry` 的形状，`registry.rs:14-21`）。
- `register` / `resolve` 照 `OperatorRegistry` 的判定形状（`registry.rs:24`、`:36`）。
- `bind(&mut self, id: &MethodId, realized_by: &[&str]) -> Result<(), MethodError>`：先 `resolve` 式的存在性判定，
  未注册返回 `NotFound`；命中则**只改 `realized_by`**，逐串 `to_string()` 覆盖旧值。
- `select(&self, domain: MethodDomain) -> Vec<&MethodEntry>`：过滤该 domain 的条目，
  **按 `id.as_str()` 升序**排序后返回。**排序键取 `as_str` 而不是给 `MethodId` 派生 `Ord`**——
  设计 §4.2 的签名列里没有 `Ord`，加派生是超出设计的一步（**本计划自定**，记在 `## 遗留` 第三节）。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-method
git commit -m "feat(method): MethodEntry 与 MethodRegistry 的登记、解析、绑定与选取"
```

---

### Task 4: 四份目录 `seeded()`、`UNREALIZED_BY_DESIGN` 与判据 3／6 的条目级侧

**Files:**
- Create: `crates/continuum-method/src/catalog.rs`
- Create: `crates/continuum-method/tests/catalog.rs`
- Create: `crates/continuum-method/tests/roster.rs`
- Modify: `crates/continuum-method/src/lib.rs`（`mod catalog;` 与导出）

**Interfaces:**
- Consumes: Task 3 的 `MethodRegistry`（同 crate 的 `impl` 块）/ `MethodEntry`；Task 2 的 `MethodDomain`
- Produces: `continuum_method::{MethodRegistry::seeded, UNREALIZED_BY_DESIGN}`

- [ ] **Step 1: 写用例（判据 3 与判据 6 的条目级侧）**

`tests/catalog.rs`：

- `seeded_builds_each_domain_catalog_with_exactly_the_named_methods`：**四条目录逐名**断言——
  对每个 domain，`select(domain)` 的 id **逐名**等于字面清单（不谈条数，直接列名；比较前对两边排序）：
  - `Software`：`planning` `TDD` `debugging` `worktree` `review` `fuzz` `verification`
  - `Video`：`shot-analysis` `narrative-plan` `timeline-compose` `render-review`
  - `Research`：`retrieval` `evidence-analysis` `contradiction-check` `citation-verification`
  - `ThreeD`：`reconstruction` `geometry-validation` `render-comparison`

  红条件：**漏一名即红、多一名即红**（逐名比较，不是计数比较）。
  **并逐 domain 断言条数**（`software=7` / `video=4` / `research=4` / `3d=3`，字面量）。
  **两条断言不是同一件事，两条都写**：逐名比较的是**排序后的两个集合**，
  条数断言是**字面量**——前者抓不到「同一次改动里既加一名又把清单也补上」，后者抓得到。
  十八个名的出处**逐字**是 `docs/spec/04-method.md:118-141`。
- `seeded_entries_all_have_an_empty_realized_by`：`seeded()` 之后**逐条**断言 `realized_by.is_empty()`。
  红条件（**取反档**）：在 `seeded()` 里给某一条预填一个算子 id ⇒ 红。
  作用：钉住「`seeded()` 不预填任何绑定」。

`tests/roster.rs`（判据 6 的条目级侧）：

- `the_unrealized_roster_is_exactly_the_two_named_entries`：把 `UNREALIZED_BY_DESIGN` **逐项**与**字面**值比较，
  期望写成 `[("software", "TDD"), ("software", "debugging")]`——**期望值不从该常量自身取**，
  否则断言恒真、是自证的正控制。
  红条件**三条各写一处**：**少一条**（表只剩一项）⇒ 红；**多一条**（表变三项）⇒ 红；
  **改一个名或一个域** ⇒ 红。不合并成「表不对即红」。
- `every_roster_name_exists_in_the_seeded_catalog_and_is_empty_today`：对表内每一项 `(domain, id)`——
  `MethodDomain::parse(domain)` 是 `Some`；`select(d)` 里**含**该 id（故名没写错）；
  `resolve(&id)` 的 `realized_by` **此刻确为空**。
  红条件（**取反档**）：把表里的 `"TDD"` 写成 `"tdd"` ⇒ 第二条断言红（目录里没有这个名字）。
  红条件（**取反档**）：让 `seeded()` 预先把 `software/TDD` bind 上 ⇒ 第三条断言红。
- `three_d_entries_are_empty_because_the_domain_has_no_counterpart`：
  断言 `has_counterpart(ThreeD) == false`，**且** `select(ThreeD)` 的**逐条** `realized_by` 为空，
  **且** `has_counterpart` 对其余三个域为真——第三半不能省：没有那个谓词时「空是刻意的」与「漏 bind」不可区分
  （设计 §7 第 2 条）。
  红条件（**取反档**）：`has_counterpart(ThreeD)` 给 `true` ⇒ 红。
  红条件（**取反档**）：`seeded()` 给 `3d/` 的任一条预填绑定 ⇒ 第二条断言红。
- `no_counterpart_entry_outside_the_roster_stays_empty`——**「名册外的空仍红」在本 crate 内的可闭形态**：
  取 `seeded()`，对 `has_counterpart == true` 的三个域（`Software` / `Video` / `Research`）里
  **每一条不在名册里的条目**调 `bind(&id, &["placeholder"])`；然后收集这三个域里**仍为空**的条目，
  断言它**恰等于**字面名册 `[("software", "TDD"), ("software", "debugging")]`（排序后逐项）。
  红条件**三条各写一处**：**（a）名册多一条** ⇒ 该条被跳过不绑 ⇒ 空集多一项 ⇒ 红；
  **（b）名册少一条** ⇒ 另一条被绑上 ⇒ 空集少一项 ⇒ 红；
  **（c）名册写错名**（`"TDD"` → `"tdd"`）⇒ 真 `TDD` 被绑上、假名不在目录里 ⇒ 空集里没有 `"TDD"` ⇒ 红。
  **并照实写明射程**：本用例**不**覆盖「生产里某块漏 bind」——它在用例内部自己把非名册条目全绑上了。
  那一侧 `continuum-method` 看不到（四块在别的 crate、在其后运行，设计 §4.5 末），
  交给 P5c／P5d／P5e 各自的计划并由整合作收口（`## 交付给谁`）。

- [ ] **Step 2: 运行，确认失败**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-method --test catalog
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-method --test roster
```

预期：**`could not compile`**（`seeded` / `UNREALIZED_BY_DESIGN` 尚未定义）。编译期失败，照实记录。

- [ ] **Step 3: 实现**

`crates/continuum-method/src/catalog.rs` 两处落点：

- `impl MethodRegistry { pub fn seeded() -> Self }`：以 §187 的四目录**逐名**建立条目，
  `domain` 与 `id` 照录，`realized_by` 为 `Vec::new()`（**`3d/` 的三条与 `software/TDD`／`debugging` 是刻意留空**，
  其余十三条待各自的算子块经 `bind` 填入）。**十八次 `register` 的返回值一律 `expect`**：
  十八个 id 互不相同（§187 的清单无重名），撞名即代码写错；
  `MethodError::Duplicate` 在生产里只在这条路径上可达（设计 §4.5）。
- `pub const UNREALIZED_BY_DESIGN: [(&str, &str); 2] = [("software", "TDD"), ("software", "debugging")];`
  常量文档注释照设计 §4.5 写清三件事：形状是 `(领域名, 方法 id)`、领域名与 `MethodDomain::as_str` 的取值对齐；
  今日**恰为**这两条（`TDD` 的全部内容是次序、`Vec<String>` 表达不了；`debugging` 在链上没有对应步骤——
  报出方是 P5c 设计 §7.2，协调者 2026-10-10 裁定）；**这张表是判据 6 的例外集，是判据的输入，
  不是「注释里的一句豁免」**——表必须恰为两条，多一条、少一条、写错一个名，Task 4 的 roster 用例都红。

- [ ] **Step 4: 运行全部测试并提交**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
git add crates/continuum-method
git commit -m "feat(method): 四份方法目录、seeded() 与按设计留空的名册"
```

---

### Task 5: 收尾与复核

**Files:**
- Modify: `crates/continuum-method/**`（**仅在复核发现缺口时**）

- [ ] **Step 1: 全量验证**

```bash
TMPDIR="$PWD/.tmp" timeout 1500 cargo test --workspace --no-fail-fast
TMPDIR="$PWD/.tmp" timeout 900 cargo build --workspace --all-targets
```

预期：全绿、0 warning。**读数按 Global Constraints 那条办**：数日志里 `Running` 的行数与 `test result:` 的行数
（Doc-tests 算一条），核它们与 `-- --list` 的目标数一致；`Compiling` 字样不作判据。

- [ ] **Step 2: 依赖门复核**

```bash
TMPDIR="$PWD/.tmp" timeout 600 cargo test -p continuum-runtime --test dependency_direction
```

预期：`every_crate_depends_only_on_its_allowed_set` 全绿。**逐条核三件事**：
（a）`("continuum-method", &[])` 在表里，且 `continuum-method` 对每个其它 workspace 成员的断言均为「未出现」；
（b）`crates/continuum-method/Cargo.toml` 的 `[dependencies]` 里**零条 workspace 内 crate**——
本表是「允许集合」的转录，而设计 §9 的口径是**精确一致**（逐对 `assert_eq!` 已覆盖两个方向）；
（c）粒度写清：`--depth 1` 只钉**直接边**；传递可达在 Rust 下不可 `use`，**故直接边即这条规则的完整粒度**。
**不把这句读成「任何指向 graph 的边都会变红」**——空数组挡的是「本 crate 自己写下 `use continuum_graph`」这一类。

- [ ] **Step 3: 源码面复核（按词根取候选、再人工通读）**

```bash
grep -rn "continuum_operator\|continuum_graph\|continuum_artifact\|continuum_port\|continuum_core" \
  crates/continuum-method/src
grep -rn "OperatorRef\|OperatorId\|OperatorRegistry\|OperatorError" crates/continuum-method/src
grep -rn "serde\|Serialize\|Deserialize" crates/continuum-method/src
grep -rn "todo!\|unimplemented!\|unreachable!\|panic!(" crates/continuum-method/src
grep -rn "SystemTime::now\|Instant::now" crates/continuum-method/src
```

**五条的预期与作用域**（**零命中不是充分证据**——grep 按**行**匹配，跨行折行会漏；
**故做法是 grep 出候选行后对 `src/` 逐处通读**，结论以通读为准、grep 只用来定位）：
（a）五个 workspace 内 crate 的标识符零 `use`；（b）四个 `Operator*` 名零命中；
（c）serde 零命中（本 crate 不序列化）；（d）`todo!` 一类零命中（**不留桩**）；
（e）系统时钟零命中。**作用域写死**：这五条 grep 只扫 `src/`，**不含测试目录**
（测试夹具允许构造外部值、允许用 `"placeholder"` 一类的占位串，见 Task 4）。
**并写明限度**：`realized_by` 是 `Vec<String>`，**写错的算子 id 照样编得过**——
本 crate 无从核它的内容（设计 §3.2 末三条），故上面五条的结论**不覆盖文本内容的正确性**。

- [ ] **Step 4: 逐条核对设计 §10 的六条判据**

| 设计 §10 的判据 | 证据 |
|---|---|
| 1. 依赖门：`ALLOWED` 含 `("continuum-method", &[])`，且逐对断言为「未出现」 | Task 1 Step 3b（首跑运行期红）＋ Task 5 Step 2 |
| 2. 目录封闭而可往返：`ALL.len() == 4`、四值逐一往返、四值之外逐项 `None`（含 `"image"`） | Task 2 的三条用例 |
| 3. 名册逐项有照片：逐 domain 计数 ＋ 四条目录的 id 集合逐名等于 §187 | Task 4 的 `seeded_builds_each_domain_catalog_with_exactly_the_named_methods` |
| 4. 两条错误路径各一枚：`register` 撞名 → `Duplicate`；`resolve` 未注册 → `NotFound`；`bind` 未注册 → `NotFound` | Task 3 的前三条用例（`Duplicate` 那条另带「不覆盖」断言） |
| 5. 选取确定性：同一 domain 两次 `select` 同序，且按 id 升序 | **一半**：升序由 Task 3 的 `select_is_ordered_by_id_ascending` 兑现；**「两次同序」那一半据实记为无判据的加强句**（同进程内不排序的 `HashMap` 迭代序也稳定） |
| 6. 空与漏由谓词与名册分开断 | **域级侧**：Task 2 的 `has_counterpart_…` ＋ Task 4 的 `three_d_entries_…`；**条目级名册侧**：Task 4 的两条 roster 用例（字面期望、两侧都钉）。**「有对家域的条目除名册外逐条非空」那一半本 crate 闭不了**，据实标注为**未完整成立**，收件人见 `## 交付给谁` |

**若某条找不到对应证据，不得标注为覆盖**。**判据 6 的第 2 条即此情形**，已按未完整成立记录。

- [ ] **Step 5: 变异复核**

按纪律 1／2 跑**承重守卫**，每轮：`trap` 装还原 → 变异前 `sha256sum` → 跑全量 `--no-fail-fast` → 读红位 →
还原后 `sha256sum` → 独立日志路径。**逐轮在报告里附
「变异前的 sha256 / 还原后的 sha256 / 红的位置 / 日志路径 / 档位（取反／放宽／收紧／移除）」**。

**必跑的变异体（逐条标档位与预期谁红）**：

| 变异 | 档位 | 预期红在 |
|---|---|---|
| `crates/continuum-method/Cargo.toml` 加一条 `continuum-graph` 依赖 | 收紧 | **跨 crate**：`continuum-runtime` 的 `dependency_direction`（**必须跑全量**） |
| `register` 删掉 `contains_key` 判定、直接 `insert` | 移除 | Task 3 的 `register_rejects_…`（两条断言都红） |
| `bind` 对未注册 id 走插入路径 | 放宽 | Task 3 的 `bind_returns_not_found_…` 的「不新建」断言 |
| 覆盖改追加（`bind` 用 `extend`） | 取反 | Task 3 的 `binding_twice_overwrites_…` |
| `select` 的排序键取反 | 取反 | Task 3 的 `select_is_ordered_by_id_ascending` |
| `has_counterpart(ThreeD)` 给 `true` | 取反 | Task 2 的 `has_counterpart_…` 第四条 |
| `parse` 加通配臂 `_ => Some(Software)` | 放宽 | Task 2 的 `parse_rejects_…` |
| `seeded()` 给 `software/TDD` 预填绑定 | 取反 | Task 4 的 `every_roster_name_…` 第三条 |
| `UNREALIZED_BY_DESIGN` 删一条（只剩 `TDD`） | 移除 | Task 4 的 `the_unrealized_roster_…` 与 `no_counterpart_entry_outside_…`（b） |
| `UNREALIZED_BY_DESIGN` 加第三条 | 收紧 | Task 4 的 `the_unrealized_roster_…` 与 `no_counterpart_entry_outside_…`（a） |
| `seeded()` 少 register 一条（如 `3d/render-comparison`） | 移除 | Task 4 的逐名与条数两条断言 |
| `as_str` 把 `ThreeD` 返回 `"3D"` | 取反 | Task 2 的往返用例 |

**不列入的变异体及理由**：删掉 `select` 的排序（**弱变异体**，`HashMap` 迭代序可能偶然升序，见 Task 3）；
给 `MethodError` 加第三枚变体（**编译不过不是变红**，纪律 1(c)；它的照片是 Task 1 的穷尽解构）。

- [ ] **Step 6: 提交**

```bash
git add <本 task 改动的显式路径>
git commit -m "feat(method): P5b 执行方法库的收尾与复核"
```

---

## 遗留

**凡设计未给判据的，下标「规范未给判据」；凡本计划自己定了的，下标「本计划自定」。**

### 一、设计 §8 的九条待对账（**本计划一条都不发明**，收件人照原文）

**打 ⚠️ 的六条是缺口**（设计 §8 末自陈：找不到规范判据、或两处来源相抵）：R2、R3、R4、R5、R8、R9。
**R6 与 R7 是设计取向，不是缺口**。

- **R1（待对账）**：§8.3「← 无」的读法决定 `realized_by` 取文本还是 `OperatorRef`。本计划**已在 Task 1 落成
  文本一侧的两件事**（`realized_by: Vec<String>`、`ALLOWED` 的空数组）——**这正是设计所取的读法**。
  **本计划不重开这条**；若重裁，落在 `Vec<OperatorRef>` 上时 `ALLOWED` 的空数组与 `Vec<String>` 一并改。
  **收件人：复审者与协调者。**
- ⚠️ **R2（实测，缺口）`3d/` 无算子块。** 本计划按设计建 `MethodDomain::ThreeD` 与其三条条目、
  `realized_by` 留空并由 `has_counterpart(ThreeD) == false` 承载「无对家」。
  **规范与切分文档均未指定对家，据实留成缺口，不发明对家。收件人：协调者。**
- ⚠️ **R3（实测，缺口）image 域无目录。** `MethodDomain` 无 image 取值，P5f 无 `bind` 目标。
  **本计划不发明 `image/` 目录**（理由与设计一致：§187 的清单以「例如：」开头、**非穷尽**，
  故「不发明」的判据是「无规范判据，据实记缺口」，不是「与 §187 冲突」）。**收件人：协调者。**
- ⚠️ **R4（缺口，规范未给判据）任务的「领域」如何判定无来源。** 没有任何字段把 `Intent` / `Contract` /
  `Plan` / `Node` 关联到 `MethodDomain`，故 `select` 的**输入从何而来**规范未定。
  **本计划只冻 `select(domain)` 这个入口，不发明领域判定器。** **收件人：P4 的设计者与协调者。**
- ⚠️ **R5（缺口，部分有来源）`select` 的消费点：位置钉得住，调用者未定。**
  §218 把 Execution Method 列在闭环步 `ADFIR ↓ Execution Method ↓ Model / Tool / Compute Routing` 上
  （`docs/spec/04-method.md:1373-1377`），§216 的 Runtime Learning Loop 消费「哪些 Execution Method 更可靠」
  （`:1177`、`:1185`）。**仍然未定的是「哪个组件在什么时候调 `select`」。**
  **本计划不指定消费点**（切分文档 §四第 4 条亦禁止 P5 自建执行路径）。**收件人：P1 执行器与 P4 Planner 的设计者、协调者。**
- **R6（待对账）**：§186 的流程名（如 `spec-review`、`quality-review`）**不入** `software/` 目录——
  §187 的 `software/` 清单里没有它们。**本计划照此办**（Task 4 的逐名断言即它的照片）。**收件人：P5c。**
- **R7（全称措辞降级）**：本计划全文只写「`seeded()` 依 §187 建十八条」，**不写「方法共十八种」**、
  **不写「四个领域是全部领域」**。**Task 4 的条数断言是字面量，不是自指计数。**
- ⚠️ **R8（工程文档内部不一致）**：§8.3 给层 8 的 EML 写了 `← 无`（`docs/02-工程.md:526`），
  而 §9.2「入度为零的组件」那张表（`:594-601`）按层列了六项、**独缺「跨领域(8)」一行**。
  **设计取 §8.3，本计划照此办**（`ALLOWED` 的空数组即它的兑现）。
  **「若改以 §9.2 为准，R1 的读法随之重开」这一句仍成立。收件人：规范维护者。**
- ⚠️ **R9（据实留的形状）「同一 id 被两块写」今天无处触发。** 目录与算子块一对一（设计 §5.1），
  **本计划不加检测**。**收件人：协调者。**

### 二、本计划闭不了的那一半（**据实记录，不假称已覆盖**）

- **判据 6 的「有对家域的条目除名册外逐条非空」**：`continuum-method` 看不到四块的 `bind`
  （它们在别的 crate、在其后运行）。本计划在 Task 4 用 `no_counterpart_entry_outside_the_roster_stays_empty`
  钉住**名册本身的完整性**（名册多一条／少一条／写错名各是一条红），
  但**「生产里某块漏 bind」这一侧本 crate 一条断言都覆盖不到**。**收件人：P5c／P5d／P5e 各自的计划与整合作。**
- **`realized_by` 的文本内容**：写错的算子 id 照样编得过，本 crate 不解析它（设计 §3.2 末三条）。
  Task 5 Step 3 的五条 grep 结论**不覆盖**它。**收件人：消费块（P5c 的设计 §7.1 已给出运行期核对的一条用例，那是本条的正解）。**
- **判据 5 的「同一 domain 两次 `select` 给出同一序列」**：**无判据**（同进程内不排序的 `HashMap`
  迭代序也稳定，这条加不到任何东西）。**据实记为无判据的加强句**，不写用例。

### 三、本计划自定的形状与取值（**申报**，逐条说清代价）

- **`MethodId` 的派生集**（Task 1）：设计 §4.2 只给了 `new` / `as_str` / `Display`，**派生由两处必需倒推**
  （`HashMap` 的键要 `Eq + Hash`、`MethodError` 的派生要 `PartialEq / Eq / Clone`）。
  **不加 `Ord` / `PartialOrd`、不加 serde 派生。代价**：派生集是本计划定的；**收益**：不引入设计未要求的接口面。
- **`select` 的排序键取 `id.as_str()`**（Task 3）：设计 §4.5 只写「按 id 升序」，没给排序依据。
  **取 `as_str` 而非给 `MethodId` 派生 `Ord`**——后者是超出设计的一步。
  **代价**：`select` 的实现里多一次 `as_str` 调用；**收益**：`MethodId` 的形状与设计逐字一致。
- **`seeded()` 里十八次 `register` 的返回值一律 `expect`**（Task 4）：设计 §4.5 明写 `Duplicate`
  只在这条路径上可达。**代价**：写成 `expect` 会让「§187 的清单哪天出现重名」在运行期 panic 而不是静默覆盖；
  **收益**：撞名是代码写错，静默覆盖会藏起它。**这不是「留桩」**（`expect` 是终态写法，不是待补的占位）。
- **`MethodEntry` 的派生集**（Task 3）：`Debug, Clone, PartialEq, Eq`（照 `Operator`
  在 `crates/continuum-operator/src/definition.rs:78` 的派生集去掉 serde）。

### 四、设计近期改过的两处：**本计划照改后的形状写**（来历留此，免得复审照旧版核）

- **`MethodEntry` 的 `purpose` 字段已删除**（设计 §4.3）：它曾是一个**没有写入路径的死字段**
  （唯一的填充入口 `bind` 只碰 `realized_by`、`register` 被四块禁止，故 `purpose` 恒等于 `id`）。
  设计取「去掉字段」而不是「拓宽 `bind`」，因为后者等于让四块各自发明「这条方法怎样才可靠」。
  **本计划照删后的形状写**（Task 3 的 `MethodEntry` 只有三个字段）。
- **判据 6 已按裁定收窄**（设计 §10 判据 6）：`software/` 的 `TDD` 与 `debugging` 两条方法名**不是算子**，
  故「逐条非空」收窄为「除 `UNREALIZED_BY_DESIGN` 外逐条非空」，豁免集**钉在常量上、两侧都钉**、
  **期望值写成字面**。**本计划照收窄后的形状写**（Task 4 的两条 roster 用例）。
  两处的报出方是 P5c 的设计 §7.2，裁定日期 2026-10-10。

---

## 交付给谁

- **交给 P5c／P5d／P5e（各自的实现计划）**：
  - 经 `bind(id, &[…])` 填各自领域目录里**每一条已 seed 的 id**（设计第六节的表：
    P5c 填 `software/` 的 `planning` `worktree` `review` `fuzz` `verification` 五条，
    `TDD`／`debugging` 两条**本轮不 bind**；P5d 填 `research/` 四条；P5e 填 `video/` 四条）。
  - **判据 6 的「有对家域除名册外逐条非空」这一半**（本 crate 闭不了，见 `## 遗留` 第二节）。
  - **`realized_by` 文本内容的核对**——本 crate 看不到 `OperatorRegistry`（设计 §3.2 末）。
- **交给 P5f**：**无 `bind` 目标**这一事实（§187 没有 `image/` 目录；设计 §8 R3）。本计划不发明目录。
- **交给协调者**：设计 §8 的 **R2**（`3d/` 无对家）、**R3**（image 无目录）、**R9**（同一 id 被两块写今天无处触发）。
- **交给规范维护者**（本项目无此角色）：设计 §8 的 **R8**（《工程》§9.2 的入度为零表独缺层 8 那一行；
  `docs/02-工程.md:594-601`）。
- **交给 P4 的设计者与协调者**：设计 §8 的 **R4**（「领域」从何而来无规范来源）、**R5**（`select` 的消费点未定）。
- **交给复审者**：设计 §8 的 **R1**（`realized_by` 取文本的读法，其编译期照片是 `ALLOWED` 的空数组）、
  **R8**（两表不一致以哪一张为准的残留）。
- **交给本块的复审者（本计划自身的三处）**：`## 遗留` 第二节的三条「闭不了／无判据」、
  第三节的四处「本计划自定」、以及 Task 5 Step 4 里**判据 6 第 2 条标注的「未完整成立」**。
