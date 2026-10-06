# P4 语义层设计：Intent、Specification、Contract、Plan、Decision 与预算

**范围**：《工程》§2.1 的建设内容全表——**十二个组件**（`docs/02-工程.md:76-89`）：
Input Canonicalization Frontend（§270 §196 §197）、Reference Resolver（§271–§278）、
Intent 存储与状态机（§222 §223）、Specification 编译（§227 §214）、Contract 编译与版本化（§224 §226）、
ContractDiff 计算（§226）、Plan 与审查状态机（§228 §229 §338–§341）、Plan Change 分类（§230）、
Decision 对象与恢复（§331 §332）、Constraint Validator（§225 §6）、Budget 与探索预算（§333 §334）、
Budget Validator（§110 §333 §334）。

**判据的唯一来源是上表每个组件后面的那一串规范节**（`docs/02-工程.md:78-89`）。规范没给判据的，
本设计**不发明**，一律落到 §16 的遗留项并具名收件人。

**本文的第一条主线是 §2 开头那一句**——「本层不依赖任何下层，是纯语义处理」（`docs/02-工程.md:72`）。
它与 §9.1 的 `语义层 (2) → 执行层 (3)`／`→ 资源层 (4)` 并存，**两种读法都自洽**（逐条见 §1.3），
而这条读法决定了本层每一个 crate 的依赖边能不能登记。§1.2 处理它。

**第二条主线是 ENG-005**：本层是「Budget 与探索预算」与「Budget Validator」的主人，
而 ENG-005 的三层预算裁决把「预扣—结算、父子分配、预计超支走 §110、运行期超支归
`FailureClass::Constraint` + `EscalationPolicy::Decision`」逐条判给了本层
（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md` §三、§五）。§12 把它们落到组件上。

---

# 1. 范围

## 1.1 建什么、不建什么

| 组件 | 规范依据 | 本设计 |
|---|---|---|
| Input Canonicalization Frontend | §270 §196 §197 | 建（七个组件，§3） |
| Reference Resolver | §271–§278 | 建（四态 + 约束解析 + Alias，§4） |
| Intent 存储与状态机 | §222 §223 | 建（§5） |
| Specification 编译 | §227 §214 | 建（§6） |
| Contract 编译与版本化 | §224 §226 | 建（§7） |
| ContractDiff 计算 | §226 | **只建差异的判定与类型**；对图的施加不在本层（§8） |
| Plan 与审查状态机 | §228 §229 §338–§341 | 建（§9） |
| Plan Change 分类 | §230 | 建（§9.4） |
| Decision 对象与恢复 | §331 §332 | 建（§10） |
| Constraint Validator | §225 §6 | 建（§11） |
| Budget 与探索预算 | §333 §334 | 建（§12） |
| Budget Validator | §110 §333 §334 | 建（§12.4） |

**不建，且不留桩**：

| 不建的东西 | 归谁 | 依据 |
|---|---|---|
| ADFIR 图与失效传播 | 执行层（P1 已建） | §235 §306；本层只产出 ContractDiff（§8） |
| 规划本身（receding-horizon Planner、Operator 选择、并行度） | 执行层（§75 §214 的 Planner） | §2.1 的组件表里没有 Planner |
| 实体注册表（工具／模型／连接器／Skill 的登记） | P3（A/B/C/D） | 本层只**消费候选**，见 §4.3 |
| 验证与证据（Verifier、Evidence、Completion Predicate） | 跨领域层（第 8 层） | §2.5 明写本层只**存储** `verification_requirement` |
| Policy Engine 与安全规则 | 边界层（P2 已建） | §8.2 的第 2 级由本层**传入**，见 §13.3 |
| 产品状态与长期循环（Product/Milestone 的推进） | 长期循环层（第 7 层） | §231 §232 §233 不在本层组件表 |
| 维护性预算**根**的创建方 | 第 7 层 | ENG-005 §三末行判给长期循环；本层只提供该 owner 变体（§12.2） |

## 1.2 「本层不依赖任何下层」这条线怎么划

### 1.2.1 本设计的箭头约定（**全篇唯一一种**）

> **订正（2026-10-06，评审查出）**：本文初稿**两义混用**了 `→`——
> 引 §9.1 时按「被依赖者 → 依赖者」，而在几处描写「谁依赖谁」时又按「依赖者 → 被依赖者」
> （§1.2.3 的规则 2、§1.3、§2.2 的边表、§12.6、§13.2、§13.4、§16 第 22/24 条的箭头都受此影响）。
> **两义混用的后果是具体的**：`执行层 → 语义层` 一半读作「语义层可被执行层依赖」（合法），
> 一半读作「执行层可被语义层依赖」（反向边）——**同一串字符在同一个文件里指向两条相反的边**。
> 错因是本文在引文与自述之间切换时没有重定约定。

以下是本文的**唯一**约定，与 §9.1 一致，此后全篇遵守：

| 写法 | 读作 | 例 |
|---|---|---|
| `A → B` | **A 被 B 依赖**（B 依赖 A） | `语义层 → 执行层` = 执行层依赖语义层 |
| `A ← B` | **A 依赖 B**（B 被 A 依赖） | `continuum-semantics ← continuum-persist` = semantics 依赖 persist |
| 「X 依赖 Y」「X 的边指向 Y」 | 用文字，不用 `→` | —— |

**引用他文时**：若引文里的 `→` 用的是相反约定，**原样照录并在引文后标注**
（照本项目「原话照留、加订正注记」的惯例），不改引文里的箭头。

### 1.2.2 判据：两处并存，**两种读法都写出**

`docs/02-工程.md:72` 说「本层不依赖任何下层，是纯语义处理」；
`docs/02-工程.md:570-571` 画了 `语义层 (2) → 执行层 (3)` 与 `语义层 (2) → 资源层 (4)`，
而 §9.1 的箭头读**被依赖者 → 依赖者**（`docs/02-工程.md:563-567` 的订正段，
判据是 §9.2「入度为零的组件……不依赖任何其他组件」）。

> **订正（2026-10-06，修后复核查出）**：本节的标题与下面那句初稿都写「**只有一种读法**」／
> 「**唯一**读法」。**「唯一」是越界的**——`02-工程.md:72` 与 §9.1 的图并存，至少有两种自洽读法
> （逐条写在 §1.3），而初稿只给一种并称其唯一。
> **错在把一句本设计的取舍写成了逻辑上的唯一解。** 原句留在此处；
> §1.3 把两种读法与各自的代价一并写出。

本设计采纳的读法是（**另一种见 §1.3**）：

> **「不依赖下层」说的是语义不依赖——本层的每一条判定，其真值来源都在本层与规范里，
> 不在执行层／资源层／边界层的运行状态里。**
> **它不承诺本层不落库、不写事件、不产出被下层消费的东西。**

**这一读法有一处例外，据实列出**：`Node.constraints` 的词汇表属执行层（§1.2.4 末段），
本层对它的判定是在下层给定的语义下做的。

### 1.2.3 由此得到一条**可判定的操作规则**

「语义上不依赖」如果只写在文档里，它就是一句无法证伪的话。本设计把它折成一条**结构**规则：

> **本层不登记任何指向执行层／资源层／边界层的 `Cargo.toml` 依赖边。**
> **凡跨层的输入，一律以「值」由装配方传入；凡跨层的输出，一律以「类型」被下层消费。**

三条推论，全部有既有守卫可钉：

1. **依赖边在 `ALLOWED` 表里可见**。`crates/continuum-runtime/tests/dependency_direction.rs:22-160`
   的 `ALLOWED` 逐对断言（`:265-283` 用 `cargo tree --depth 1 --edges all` 逐对比对），
   且 `:252-257` 强制每个 workspace 成员都必须在表里出现
   （「workspace 成员 {name} 未列入 ALLOWED，它的依赖方向不会被检查」）——
   **故本层新增的 crate 不可能漏登记，漏了即红**。
   本层三个 crate 的允许集只含 `continuum-persist`（＋层内前驱）。
   **任何一条指向 graph／capability／provider／connector／model-registry／policy 的边都会当场变红**。

   **这条判据的粒度是「直接边」，不是传递闭包**（评审查出应写明）：
   `cargo tree --depth 1` 看不到传递可达，故「经一个中间 crate 间接够到 P1」不会被这条断言抓住。
   **但本仓不因此留缺口**：Rust 的 crate 可见性要求**直接声明**才能 `use`，
   未写进 `Cargo.toml` 的传递依赖写不出 `use continuum_graph::…`。
   **故「直接边」恰是这条规则的完整粒度，不必另设闭包断言**——
   把这一点写出来，是为了不让上面那句读起来像在说传递闭包（那会是一条假保证）。
2. **§9.1 的箭头因此保持单向**：按 §1.2.1 的约定，§9.1 里以本层为被依赖者的边
   **有两条**——`语义层 → 执行层` 与 `语义层 → 资源层`（`docs/02-工程.md:570-571`），
   即**执行层与资源层都可以依赖本层**，而本层不依赖它们中任何一个。
   **两条今天都没有消费者**（执行器未建；`BudgetView` 的投影由驱动做，§12.6），
   故本层落地时跨层边**一条都不登记**（§2.3）。

   > **订正（2026-10-06，评审查出）**：本条的初稿写「`执行层 → 语义层` 是**唯一**允许的跨层方向」，
   > **两处错**：(i) 箭头方向按的是相反约定，按 §1.2.1 应写作 `语义层 → 执行层`；
   > (ii) 「**唯一**」是假的——§9.1 里有**两条**以本层为被依赖者的边（执行层与资源层各一条），
   > 且本层还依赖 P0 基础设施。错因是把「本层作为被依赖者的边」写成了「跨层方向只有一个」。
   > 原句留在此处。
3. **跨层输入不引入类型耦合**：候选集、`Node.execution_policy`、外部副作用标志、
   失败读数都由装配方以值交出（§13 逐条列出），本层不 `use` 它们的定义 crate。

**这条规则的代价写明**：跨层 id 在本层只能是**不透明 newtype**（`EntityId`，§4.3），
于是「mention 解析出的实体」与「注册表里的 `ToolId`」之间的对应由装配方维持。
代价是**可能静默失配**；本设计的对冲是 §4.3 的**闭集不变量**（`Resolved` 的实体必属于输入候选集）。

### 1.2.4 三处必须**碰下层**的地方，逐条说清怎么碰

| 地方 | 规范要求 | 本设计的碰法 | 是否登记边 |
|---|---|---|---|
| **存储** | §222 §223 §224 §228 §331 都要持久化 | 经 P0 的 `Tx` / `Migration`（`crates/continuum-persist/src/tx.rs:12`、`db.rs:14`）；状态迁移与对应事件**同一事务**（照 P1 的 `apply_transition` 形状） | 是（`continuum-persist`，属 P0 基础设施，不在八层之内） |
| **ContractDiff 被第 3 层消费** | §226：「受影响的 ADFIR 节点 MUST 被重新判定有效性」 | 本层**产出** diff（四类差异，§7.3）；**受影响节点的判定与施加在执行层**（§8） | 否。**今天没有消费者**；将来由执行器取 `语义层 → 执行层` 这条边（§1.2.1 的约定：执行层依赖本层） |
| **Constraint Validator 判「候选计划」** | §6 §225 | 「候选计划」是**值**：§228 的 `Plan` ＋ 声明的影响面（§11.2）。**图的 `Node.execution_policy` 与 `Node.constraints` 以 `&str` / `serde_json::Value` 原样传入**，本层不 `use continuum_graph` | 否 |

第 3 行是本层最容易做错的一处：`crates/continuum-graph/src/node.rs:17` 写着
`/// 结构保存，判定属 P4。`（`execution_policy` 字段），
`docs/superpowers/plans/2026-10-01-p1-execution-layer.md:1387` 是同一句话的来历。
**「判定属 P4」说的是判定的归属，不是依赖边的归属**——P1 交出的是结构，本层交出的是判定，
两侧都不需要 `use` 对方。本设计按这条办。

**但第 3 行有一处「以值传进来」的边界要说清**（评审查出）：
`Node.constraints: Vec<String>`（`node.rs:15`）的**词汇表属执行层**——§236 只给字段名，
§2.1 也没把它的语义判给本层。本层按 §11.2 的读法把它当作「Constraint 的引用键」，
那**是在下层给定的语义下**做的判定。故：

> **本层保证的是「同一个串的行为一致」，不保证该语义正确。**
> §1.2.2 那句「本层的每一条判定，其真值来源都在本层与规范里」**在这一处不成立**——
> 这里有一份语义是从下层随值带进来的。**这是 §1.2.2 的一个例外，据实列出，不掩盖。**

## 1.3 本层的入度：§9.2 与 §2.3 那个短语的**两种读法**

§9.2 的表把**Input Canonicalization** 与 **Intent 存储** 都列为「不依赖任何其他组件」的源点
（`docs/02-工程.md:596`），而 §2.3 写的是 `Specification 编译 ← Intent 存储（经 Canonicalization）`
（`docs/02-工程.md:116`）。

**先纠正本节的定性**：`:596` 讲的是「入度为零的**源点**」，`:116` 讲的是「**层内依赖边**」——
**两个问题**，不是一处并列的矛盾。相抵**只在**把「（经 Canonicalization）」读成
「Intent 存储 ← Canonicalization」时才出现。

**两种读法都写出**（免得下一个人重新发现同一处）：

- **读法甲（本设计采纳）**：把它读成**类型依赖**——Intent 存的 `goal` 是规范化后的表示
  （§270 的 `Canonical Representation` 在 `Intent Compiler` 之前），
  故 Intent 表的外键面依赖 `EntityId` 这一个类型，而 Intent 的**任何判定**都不查 Canonicalization。
  于是 `continuum-semantics ← continuum-canonical` 是一条「类型边」（§1.2.1 的约定），
  §9.2 与 §2.3 同时成立。
- **读法乙**：把它读成**次序说明**——「经 Canonicalization」只是说 Intent 的**内容来自**那条前置流程，
  不是一条组件间的依赖边。于是 `continuum-semantics` 不必依赖 `continuum-canonical`，
  两处的字面也能同时成立（代价是 `Intent.goal` 的类型要与 `continuum-canonical` 的产物**重复定义**，
  那是「同一概念两个类型」）。

**本设计采纳读法甲**，理由是读法乙的代价落在类型重复上。**两种读法都是本设计的判断，不是规范给的**
——故 §9.2 那一格与 §2.3 的相抵据实记在 §16 第 1 条，**收件人：规范维护者**
（两处须择一改，本设计不改文档）。

> **订正（2026-10-06，评审查出）**：本节初稿写「两处同时成立的**唯一**读法」，并把相抵定性为
> 「一处相抵」。**「唯一」越界了**——`§9.2` 的源点表与 `§2.3` 的依赖边表回答的是两个问题，
> 而在「经 Canonicalization」这一短语上至少有两种自洽读法（上面列出）。
> 初稿只给出一种并称其唯一，会让后来者以为另一读法已被排除。**原句与错因留在此处。**

---

# 2. crate 划分与依赖边

## 2.1 三个 crate

| crate | 装什么 | 依据 |
|---|---|---|
| `continuum-canonical` | Input Canonicalization Frontend 的七个组件 ＋ Reference Resolver ＋ Alias 存储 | §270 §196 §197 §271–§278 |
| `continuum-semantics` | Intent、Specification、Contract、ContractDiff、Plan、Decision、Constraint Validator | §222 §223 §227 §214 §224 §226 §228 §229 §230 §338–§341 §331 §332 §225 §6 |
| `continuum-budget` | 预算树、`Allocation` / `Reservation` / `Remaining`、预算账本、Budget Validator、探索预算 | §333 §334 §110；ENG-005 |

**为什么 Constraint Validator 与 Budget Validator 分在两个 crate**：§2.2 的原文是
「Constraint Validator 与 Budget Validator **必须分离**（§6 §7）」（`docs/02-工程.md:109`）。
「必须分离」如果只写在文档里，它与「两个函数放在同一个文件里，注释说它们分离」不可区分。
**分成两个 crate 是这句话唯一可断言的形式**：`continuum-semantics` 的允许集里**没有** `continuum-budget`，
反向亦然（§11.5 的两侧守卫）。

**为什么不合成一个 crate**：ENG-005 要的是**单一预算树**（一次全局判断），
而 §6 要的是两道独立的门。两者不冲突——**树是预算层的内部结构，门是两层的对外面**。
合一会让「分离」退化成约定。

## 2.2 依赖边

按 §1.2.1 的约定（`A ← B` 读作「A 依赖 B」）：

```
continuum-canonical  ← continuum-persist
continuum-semantics  ← continuum-canonical, continuum-persist
continuum-budget     ← continuum-semantics, continuum-persist
```

| 边（左依赖右） | 使用点 | 何时登记 |
|---|---|---|
| `canonical ← persist` | `alias` 表（§4.6）经 `Tx` 读写 | 用它的那个 task |
| `semantics ← canonical` | `Intent.goal` 存规范化表示，取 `EntityId` 这一个类型（§1.3） | 同上 |
| `semantics ← persist` | 五张表经 `Tx`；状态迁移与 `Event` 同事务（§5.3） | 同上 |
| `budget ← semantics` | 预算树的 owner 取 `IntentId`（§12.2） | 同上 |
| `budget ← persist` | 账本表经 `Tx`；`RecoveryHook`（`recovery.rs:39`）的读写在恢复路径上（挂 `MarkLostExecutions` 态，§13.1） | 同上 |
| ~~`model-registry ← semantics`~~ | **不登记**——协调者已于 2026-10-06 裁定取形状乙（§12.6.1）：两类型 ＋ 驱动做恒等转换，零跨层边。此行留作前史，说明「跨层边可以登记，但这一条被裁掉了」 | 无 |

**上表因此没有任何跨层边**：本层落地时**一条跨层边都不登记**。
**本设计对两条出路都不改动 `Remaining` 的定义**（五个 `Option<i64>`、逐维现算），
故 §12.1–§12.5 一字不改——**读到本表时不要以为 §2.2 已经选了边**（选边的是 §12.6.1 的裁定）。

> **订正（2026-10-06，评审查出并已裁定）**：初稿的 §13.4 里出现过「允许 `graph → semantics`」这个说法，
> 而本表里没有对应行，§1.2.3 规则 2 又说「跨层边一条都不登记」——**三处各说一套**。
> 收敛结果：跨层边**可以**登记，前提是方向从本层出发（§1.2.1）且由那次改动的 task 自己登记；
> 而**这一条具体的边没有被采用**（裁定见 §12.6.1）。初稿那句 `graph → semantics` 的箭头也是反的，
> 已随本节重写一并改掉。

> **订正（2026-10-06，评审查出）**：本表初稿五行写的是 `canonical → persist` 这种形式，
> 与 §1.2.1 的约定相反。**含义没变**（左依赖右），改的是箭头的朝向与标题的注明，
> 以免同一份文件里 `→` 指向两条相反的边。

**按「叶子 crate 的条目记实际依赖」的既有口径**（`crates/continuum-runtime/tests/dependency_direction.rs:104-108`）：
上表的边**逐条由用它的 task 登记**，不预先声明齐。**没有一条指向 `continuum-core`**——
本层的 crate **不 `use`** `ModelId` / `ToolId` / `ConnectorId` / `Usage` 这四个类型，理由见 §1.2.3 的规则 3。

> **措辞订正（2026-10-06，评审查出）**：初稿写「本设计对 `continuum-core` 的公开面……**一次都不引用**」。
> **作用域过宽**：同篇 §12.3 与 §13.4 两次点名 `continuum_core::model::Usage`
> （`crates/continuum-core/src/model.rs:51`）并以它立论。**事实对（不 `use`），措辞要说准**——
> 「不 `use`」讲的是依赖边为零，「不引用」讲的是整篇不出现，两者不是一件事。
> 原句留在此处。

**事件与审计不新增 crate**：`EventType` 与 `AuditKind` 是 `continuum-events` 的类型，
本层经 `continuum-persist` 的 `Tx::append_event` / `Tx::append_audit` 写它们
（`crates/continuum-persist/src/tx.rs:49`、`tx.rs:79`），故不新增边。

## 2.3 本层的三个 crate 之外，**没有任何 crate 依赖本层**

今天零消费者。这不是缺口，是**次序**：ContractDiff 的消费者是执行器、
`Remaining` 的消费者是驱动（投影成 D 的 `BudgetView`）、预算结算的调用方是执行器。
三者都未建。**三处接缝逐个记在 §13，收件人分别是执行器、驱动、执行器**——
「不预先发明 API」这条纪律在这里的兑现方式是：**本层不定义一个没有实现者的 trait**
（与 D §6.1 拒绝为 `BudgetView` 抽 trait 是同一条判据）。

## 2.4 迁移号段

**已占用的号（逐 crate 实读，2026-10-06）**：

| 段 | 已占用 | 出处 |
|---|---|---|
| P0 | 1、2 | `crates/continuum-persist/src/db.rs:30`、`:47` |
| P1 | 10、20 | `crates/continuum-artifact/src/persist.rs:9`、`crates/continuum-graph/src/persist.rs:15` |
| P2 | 30、40、41 | `crates/continuum-workspace/src/persist.rs:22`、`crates/continuum-effect/src/persist.rs:28`、`crates/continuum-policy/src/persist.rs:31` |
| P3 | **50、80** | `crates/continuum-capability/src/persist.rs:42`、`crates/continuum-model-registry/src/persist.rs:107` |

> **订正（2026-10-06）**：本节初稿写「P3 取 50、60、70、80、90……实测」。
> **「实测」是假的**：P3 实际只占了 **50 与 80** 两个号——
> 60（B 的连接器）与 70（C 的 Provider）**至今没有迁移**（`continuum-connector/src` 与
> `continuum-provider/src` 里没有 `persist.rs`），90（E）也没有。
> 50/60/70/80/90 那个列表是**协调者的号段裁定**
> （`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:456`：
> 「一个子项目一个十位档——A 50、B 60、C 70、D 80、E 90」），
> 不是实际占用——**分配与占用是两件事**，初稿把它们写成了一句「实测」。
> 原句留在此处。

**两处「号相同但不是同一条链」的干扰项**（现场核对时会被误判为空号被占）：

- `60` 出现在 `crates/continuum-persist/src/bin/crash-writer.rs:17` 与
  `crates/continuum-persist/tests/crash_atomicity.rs:9`；
- `100` 出现在 `crates/continuum-persist/tests/migrations.rs:45`（表名 `layer_table`）。

**这四处都在各自的测试/工具 `Db` 里，不进生产链**（`Db::open_with` 的迁移集由调用方给，
`run_recovery`/`migrate` 只作用在那一个实例上）。故它们**不占用**号段，
但**读到它们时不要以为 P4 的 100 已被占**。

**本层取 100 段**：

| 号 | 表 | crate |
|---|---|---|
| 100 | `alias` | `continuum-canonical` |
| 110 | `intent` / `specification` / `contract` / `contract_requirement` / `plan` / `decision` / `decision_result` | `continuum-semantics` |
| 120 | `budget_node` / `budget_ledger` | `continuum-budget` |

**每一档取用前现场核对空号**（P3 的既有规矩，`docs/superpowers/p3bcdf-followups.md` §七.4），
`migrate()` 按 version 升序应用、号段可以不连续（`crates/continuum-persist/src/db.rs:96-101`），
故 100 段与 P3 的 90 段之间留空是合法的。

---

# 3. Input Canonicalization Frontend（§270 §196 §197）

## 3.1 §270 的七个组件逐项处置

§270（`docs/spec/05-normative.md:1344-1363`）给的是七个组件名与它们在 Intent Compiler 之前的次序，
**未给任何组件的输入输出类型**。§196（`docs/spec/04-method.md:471-511`）给了流程
（`Raw Input → Mention Detection → Candidate Generation → Context Resolution → Canonical Representation`），
§197（同文件 `:514-550`）给了候选生成可用的手段清单。

| §270 组件 | 本设计的处置 |
|---|---|
| Surface Normalizer | 建：`normalize(&str) -> Surface`。**只做字面处理**：Unicode 归一、宽度归一、空白折叠、大小写折叠。**不做同义词、不做纠错**——纠错是候选生成（§197）的活 |
| Mention Detector | 建：`detect(&Surface) -> Vec<Mention>`。`Mention { span, text, kind }`，`kind` 的取值域**§196/§197 未给**（只给了「代词／上下文引用」两个例），故 `kind` 取 `Opaque(String)`，不做封闭枚举（§16 第 3 条） |
| Candidate Retriever | 建：**接口在本层，来源在调用方**。`retrieve(&Mention, &CandidateSources) -> CandidateSet`，其中 `CandidateSources` 是调用方装配好的若干来源（§4.3） |
| Reference Resolver | 建（§4） |
| Alias Resolver | 建；作为 `CandidateRetriever` 的一个来源接入，不是独立的解析路径（§4.6） |
| Temporal Resolver | 建：`resolve_temporal(&Mention, &Now) -> TemporalBinding`。**「Now」是入参**（本层不取系统时钟，与 P2 的入口同一条纪律） |
| Ambiguity Manager | 建：聚合同一次输入里所有 mention 的 `ResolutionResult`，产出 `CanonicalRepresentation` 或挂起（§3.3） |

**「联合解析」（§198）没有落点**：§198（`docs/spec/04-method.md:552-593`）要求多个模糊引用
**联合**解析、解出「最一致的 Entity Graph」，而 §2.1 的组件表里**没有**承担联合约束传播的组件，
§2.1 的依据栏也没有列 §198。本设计**不发明**联合约束算法，故同一次输入里的多个 mention
**各自独立解析**。**这是本设计已知未实现的一条规范要求**，记在 §16 第 4 条。

## 3.2 规范化产物与「无法规范化」的表示

§2.4 的第一条完成判据是「含错别字、简称、中英混写、代词的输入被规范化，**或被明确标记为
Unknown / Ambiguous**」。故产物是一个和类型：

```rust
pub enum Canonicalized {
    Canonical(CanonicalRepresentation),
    NeedsResolution(Vec<UnresolvedMention>),
}
```

**「明确标记」这句话在这里的结构形态**：`Canonical` 这一支要求**每个 mention 都是 `Resolved`**
（§4.2 的闭集判定），故「规范化成功」与「有 mention 未解」不可能同时成立——
一支是另一支的补集，不是两个可以同时为真也可以同时为假的标志位。

---

# 4. Reference Resolver（§271–§278）

## 4.1 四态：定义从 §206 来，§271 只给名字

§271 只列了四个名字（`docs/spec/05-normative.md:1366-1375`）。**它们的含义在 §206**
（`docs/spec/04-method.md:865-899`），逐条照录：

| 态 | §206 的定义 | 本层的判定读法 |
|---|---|---|
| `Resolved` | 「允许自动绑定」 | 约束解出**恰好一个**候选，且该候选满足**全部显式约束**，且风险门槛已过（§4.5） |
| `ConfirmRequired` | 「第一候选明显，但不足以安全执行」 | 存在一个占优候选，但**要么**风险门槛未过，**要么**占优性未达判据（§4.4 的 `ConfirmationThreshold` 是**必填入参**） |
| `Ambiguous` | 「多个候选仍然成立」 | 满足全部显式约束的候选**多于一个** |
| `Unknown` | 「当前没有可靠解析」 | 满足全部显式约束的候选**为零** |

**§206 的 `abstain` 是这四个态的公共性质**：四态里没有「必须给一个答案」这一支，
故四个态都是**返回值**，不是错误。

**四态的类型与字段清单**（2026-10-06 补，计划作者查出本节原先只说「字段清单逐项断言」
却从未给出字段——**那个要求因此不可断言**）：

```rust
pub enum ResolutionResult {
    Resolved(Resolved),
    ConfirmRequired(ConfirmRequired),
    Ambiguous(Ambiguous),
    Unknown(Unknown),
}

/// 解出了**恰好一个**合格候选。
pub struct Resolved {
    entity: EntityId,
    /// 该候选满足的那一组显式约束 —— **照录判定时用的那一组**，不是重新推的（§4.2 的断言 D 靠它可查）。
    satisfied: Vec<ExplicitConstraint>,
    /// 判定用的候选集规模，供调用方核对「唯一合格」这件事（§4.2 的断言 A/D）。
    candidate_count: usize,
}

/// 有一个占优候选，但不足以安全执行（§206）。
pub struct ConfirmRequired {
    first: EntityId,
    /// 与 `first` 一同满足全部显式约束的候选**全体**（含 `first`），**多于一**。
    surviving: Vec<Candidate>,
    /// 判定用的候选集规模。
    candidate_count: usize,
}

/// 多个候选仍然成立（§206）。
pub struct Ambiguous {
    /// 满足全部显式约束的候选**全体**，**至少两个**。
    surviving: Vec<Candidate>,
    candidate_count: usize,
}

/// 当前没有可靠解析（§206）。
pub struct Unknown {
    reason: UnknownReason,
    candidate_count: usize,
}

/// 「为什么是 `Unknown`」。**两枚都必须是可区分的**——
/// §4.2 的断言 B 与断言 C 各钉一枚，混成一枚会让「全不合格」与「本来就没有」不可分。
pub enum UnknownReason {
    /// 候选集为空（§4.2 的断言 C）。
    NoCandidate,
    /// 候选集非空，但全部违反某条显式约束（§4.2 的断言 B）。
    AllCandidatesRejected,
}
```

**`ResolutionResult` 的每一支都带 `candidate_count`**：断言 A（`e` ∈ 候选集）与断言 D（唯一合格）
都要拿「集合里有几个」去核，而**判定用的那个集合在结果返回后就没了**——
不带这个数，两条断言就只能各自再构造一次候选集，那种做法会让用例**用自己的构造去核自己的结果**
（本仓「表驱动用例覆盖错路径」的同一形状）。带一个数而不是整个集合，是为了不把
「判定输入」混进「判定输出」（`surviving` 那几个是**输出**：合格的那一部分）。

**返回类型不是 `Result`，故没有 `Err` 支**：§206 的 `abstain` 已经把「没有可靠解析」
收进 `Unknown` 这一支，而**本层没有第二种失败**
——输入是值（候选集、`ResolutionRisk` 都由调用方给），解析不做 I/O、不落库。
故 `resolve` **返回 `ResolutionResult`，无 `Err` 支**
（2026-10-06 写死；计划作者取同一结论，理由一致）。

> **订正（2026-10-06，计划作者查出）**：本节初稿写「**四态一律 `Ok`**」，
> 而全节**没有任何 `Err` 支**——那句话隐含「返回类型是 `Result`」，与实际的
> 「四态本身即返回类型」相抵。已改为上面那句「返回 `ResolutionResult`，无 `Err` 支」。
> 原句留在此处。
**四态之间的两条边界没有判据，记在 §16**：
(a) `ConfirmRequired` 与 `Ambiguous` 的界是「第一候选明显」中的「明显」——
§206 未给阈值，§271–§278 也未给；
(b) `Unknown` 与「扩大检索」的界是 §201 的「0 candidate → NIL / **Expand Retrieval**」
（`docs/spec/04-method.md:691-692`）——**重试检索的条件未给**，本设计不做二次检索。
两条都**收件人：规范维护者**。

## 4.1.1 「显式约束」是什么：类型与「满足」的判据（**本设计自定**）

§4.2 的四条断言、§14 的用例、§4.6 的 `ExplicitBinding` **全部建立在「显式约束」这个类型与
「满足」这个判据上**，而本节先前**没有给出它们**——那会让上面那些断言**无法落地**
（2026-10-06，计划作者查出；这是 Task 5 的开工前置）。

**规范给到哪一步，先说清**：

| 规范给的 | 出处 |
|---|---|
| 「约束解析」的**流程名**：`Candidate Set → Explicit Constraints → … → Constraint Propagation` | §201（`docs/spec/04-method.md:662-704`） |
| 一个**值域**：`ResolutionScope` 的九个作用域 | §274（`docs/spec/05-normative.md:1408-1427`）／§199 |
| 一个**形状**：`ExplicitBinding { mention, entity, scope, source = USER }` | §277（同文件 `:1464-1484`） |
| 一句**性质**：「显式 scope 优先级最高」 | §274 |

**规范没给的**：约束的**类型**、**可以用哪几种**、以及**「满足」怎么判**。
故下面这两件是**本设计的决定**（不是转录），理由逐条给出。

```rust
/// 一条显式约束。**取值域是闭集，三枚**——每一枚都能追到规范里已有的一个概念，
/// **不引入开放谓词语言**（那会变成一套本层发明的语法）。
pub enum ExplicitConstraint {
    /// 候选必须落在该作用域内。出处：§274 的九值 ＋「显式 scope 优先级最高」。
    Scope(ResolutionScope),
    /// 候选的实体种类必须是这一种。出处：§198 的三类候选（`ToolCandidate` /
    /// `SkillCandidate` / `Relation`）。
    EntityKind(EntityKind),
    /// mention 必须绑到该实体。出处：§277 的 `ExplicitBinding`。
    BoundTo(EntityId),
}

/// 候选的**判定事实**，由调用方与候选一并给出。
///
/// **本层查不到它们**：注册表不在本层（§1.2.3），故 scope 与 kind 只能随值进来。
/// **§272 的 `Candidate` 三字段不动**（`entity_id` / `evidence[]` / `retrieval_score?`）
/// ——往那里加字段会改掉张照录的形状。
pub struct CandidateFacts {
    pub scope: ResolutionScope,
    pub kind: EntityKind,
}

/// 判据：**合取**。候选 `c` 满足约束集 `K` ⟺ `K` 中**每一条**都由 `c` 与 `f` 满足。
pub fn satisfies(c: &Candidate, f: &CandidateFacts, k: &ExplicitConstraint) -> bool {
    match k {
        ExplicitConstraint::Scope(s)      => f.scope == *s,
        ExplicitConstraint::EntityKind(t) => f.kind == *t,
        ExplicitConstraint::BoundTo(e)    => c.entity_id() == e,
    }
}

/// 「合格」＝候选集里满足全部约束的那些。
pub fn qualifying(cs: &[(Candidate, CandidateFacts)], k: &[ExplicitConstraint]) -> Vec<Candidate>
```

**「合取」这一条是本设计的决定**（§201 只给了流程的名字）：把每一条约束都当作**必要条件**
是唯一不引入优先级的读法；若将来要加权或分级，那是**新增判据**，须有规范出处。
**这一条正好是 §4.2 断言 B 与 D 的定义基础**（「全部违反」＝合取下的空集，「唯一合格」＝合取下恰一个）。

**`EntityKind` 的取值域照 §198 的三类**（`ToolCandidate` / `SkillCandidate` / `Relation`）——
**只是那三类，不扩**；§197 的七种**检索手段**不是种类（它们是 `CandidateSource`，§4.3）。

> **为何不把这些做成 trait／闭包**：一个「任意谓词」的开放接口会让**四条断言全部失去判据**
> （「满足」变成调用方定义的，本层就无从断言 B/D）。闭集是让断言可写的前提。
> **若复审判定需要开放谓词，则四条断言要一并重写**——这不是一处局部改动。

## 4.2 「不存在正确候选时不强行绑定」写成可断言的东西

这是本层最容易做错的一处，来源有两处规范原文：

- §273：「每次解析 MUST 保留 `UNKNOWN / NIL` 可能性。系统 MUST NOT 因 Registry 中存在候选就强行绑定。」
- §200（`docs/spec/04-method.md:634-659`）：「禁止：没有正确候选 → 强行选择最相似候选，
  否则系统会产生**稳定的错误确定性**。」

本设计把它折成**四条断言**：三条管「什么时候**可以**返回 `Resolved`」，一条管「什么时候**不可以**」。

**（断言 A，闭集）`Resolved(e)` 蕴含 `e` 是输入候选集里的成员。**
不是「e 与某个候选相似」，而是**同一个 `EntityId` 值**。
它排除的失败形态：解析器自造一个实体、或把检索分数最高的候选的名字拼出来。

**（断言 B，拒绑）候选集非空 ∧ 全部候选都违反某个显式约束 ⟹ 结果不是 `Resolved`。**
这一条是 §273 的正面形态。它的难点是「全部候选`都得判到`」——
故用例必须是**逐个候选各一条**（与「枚举断言须逐项有照片」同一条纪律）：
构造 N 个候选、每个恰好违反一条不同的显式约束，断言 N 次结果里**没有一次**是 `Resolved`，
且返回的具体是哪一态（`Unknown` 还是 `ConfirmRequired`）逐次断言。

**（断言 C，空集）候选集为空 ⟹ `Unknown`。**

**（断言 D，唯一合格）`Resolved(e)` 蕴含 `e` 满足**全部**显式约束，且候选集里**没有第二个**满足者。**

> **订正（2026-10-06，评审查出）**：本节的初稿只写了 A / B / C 三条。
> **三条不穷尽**——它们钉住的是「`e` 在集合里」（A）与「全不合格时不许绑定」（B），
> **漏掉了两种同样会产出错误 `Resolved` 的情形**：
> (i) `Resolved` 指向的候选**在集合里但在约束下不合格**（例如集合里既有合格者又有不合格者，
> 解析器绑定到了不合格的那个——A 判不出来，因为它在集合里）；
> (ii) **多个合格候选却返回 `Resolved`**（正确结果是 `Ambiguous`——B 也判不出来，
> 因为不是「全部不合格」）。**两种都是 §206 的「多个候选仍然成立」被违反**，
> 而两者都产出 `Resolved`，是不可从返回值上看出的错。

**D 的两条照片**（评审查出后补，逐条对应上面两种情形）：

**两条照片的夹具都必须含一条非空显式约束**（评审查出后补，见下）：

1. **`Resolved` 指向合格者、且集合里另有不合格者**：
   夹具：显式约束集**至少一条且非空**（记作 `K`），候选集 `{c₁ 满足 K, c₂ 违反 K}`。
   断言结果 `Resolved(c₁)`——**正例**，钉住「合格的那一个能被选出来」，
   同时钉住 A 不足以替代 D（`c₂` 也在集合里，若绑定到它，A 仍成立）。
2. **多个合格候选 ⟹ 不是 `Resolved`**：
   夹具：**同一条非空约束 `K`**（`K` 非空由 §4.1.1 的 `ExplicitConstraint` 闭集保证可构造）；
   候选集 `{c₁ 满足 K, c₂ 满足 K, c₃ 违反 K}`。
   断言两条，**`c₃` 靠第二条承重**：

   - 结果是 `Ambiguous` 而**不是** `Resolved(c₁)`；
   - **`Ambiguous.surviving` 恰是 `{c₁, c₂}`，且不含 `c₃`**（逐项：长度为 2、两枚都在、`c₃` 不在）。

   **第二条是要紧的那一条**（2026-10-06，计划作者查出后加强）：
   若「合格」被实现成**恒真**，则 `c₃` 也会合格、`surviving` 会变成三枚——
   但只断言「结果是 `Ambiguous`」的话**它仍然绿**。
   **故「`c₃` 是对照组」这句话要靠 `surviving` 的逐项断言去兑现，否则 `c₃` 不承重、等于没设**。
   这也是为什么 `Ambiguous` 必须带 `surviving`（§4.1 的字段清单）——不带就写不出这条断言。

> **订正（2026-10-06，修后复核查出）**：本条初稿把照片 2 的夹具写成
> 「构造 `{c₁ 合格, c₂ 合格}`（两者满足**同一组显式约束**）」，而**「同一组约束」没有要求它非空**。
> **约束集为空时每个候选都平凡地合格**，该用例退化成「两个候选都在集合里」，
> **没有经过任何约束求值**，却照样返回 `Ambiguous`——**这样的用例恒真**：
> 一个把「合格」判成恒真的实现也会绿。故夹具写死**至少一条非空约束**，
> 并加第三个候选 `c₃` 违反同一条作对照——**有 `c₃` 在，「合格」才是被证明的，而不是被假定的**。
> 照片 1 同理（那两个候选的合格／不合格必须由**同一条非空约束**区分，
> 否则「`c₂` 违反一条」里的「违反」也无从判定）。原句留在此处。

四条一起钉住两侧：空集不产出（C），全不合格不产出（B），
产出时必须是集合成员（A）、**必须合格（D 前半）、且必须唯一（D 后半）**。
**A、B、D 三条都在 fail-open 侧**（少了任何一条，系统仍会返回一个 `Resolved`，看不出错），故三条都必须有照片；
C 在 fail-closed 侧，一条即可。

## 4.3 候选集合以**值**进入（§201 的约束解析，§1.2 的边界）

§201（`docs/spec/04-method.md:662-701`）定核心语义是
`Candidate Set → Explicit Constraints → Registry Relations → Scope → Conversation Relations → Constraint Propagation`，
并明写「概率和相似度主要用于 candidate retrieval / ranking，**而不是系统语义本身**」。

据此本层分两个面：

```rust
/// 候选。§272 的三个字段照录：
/// entity_id / evidence[] / retrieval_score?。
pub struct Candidate {
    entity_id: EntityId,      // 不透明 newtype，见下
    evidence: Vec<String>,
    retrieval_score: Option<f64>,   // §272：MAY 用于排序
}
```

**`EntityId` 是不透明 newtype，不是 `ToolId` / `ModelId` / `ConnectorId` 中的任何一个。**
理由不是偏好，是 §1.2.3 的规则 3：引用那三个类型就要登记指向资源层／边界层的边。
代价与对冲见 §1.2.3 末段与 §4.2 的断言 A。

**`CandidateSources` 由装配方给出**：§197 列的七种手段（edit distance / phonetic similarity /
ASR hypotheses / pinyin / embedding retrieval / alias registry / entity registry）里，
**只有 alias registry 在本层**（§4.6），其余六种是外部来源，以 `dyn CandidateSource` 的形式由调用方装配。
**本层不为它们定义 trait 的默认实现**，也不内置任何一个（§16 第 5 条：真实来源今天零个）。

**§197 的分工照录**：候选生成负责「**高召回候选生成**」，「而不是直接决定最终语义」
（`docs/spec/04-method.md:542-548`）。故 `CandidateSource` 的返回值只能是 `Vec<Candidate>`，
**没有任何返回单一实体的方法**——这一条是可断言的（trait 的方法集逐条列，见 §14）。

## 4.4 `retrieval_score` 与 `ResolutionConfidence` 的禁令落在类型上（§272 §275）

§272：「`retrieval_score` MAY 用于排序。MUST NOT 自动被当成真实概率。」
§275：「该值只决定**是否允许自动执行**，不定义**实体为真的概率**。」
而 §2.2 复述为：「候选检索分数仅用于排序，不得当作真实概率」。

两句禁令若只写在注释里，就没有照片。本设计把落点放在**类型的位置**上：

1. **`retrieval_score` 只出现在 `Candidate` 上，不出现在 `ResolutionResult` 上。**
   故一个已解出的结果**携带不了**分数——想要「用分数再判一次」的调用方拿不到那个数。
   照片：`ResolutionResult` 的字段清单逐项断言（加一个分数字段即红）。
2. **`ResolutionConfidence` 是独立的 newtype，且只有一处消费者**：风险门槛（§4.5）。
   它**没有** `as_probability` / `to_f64` 这类读者，只有 `meets(&RequiredStrength) -> bool`。
   照片：trybuild 样例（跨类型不可转换）+ 「`ResolutionConfidence` 不出现在 `Candidate` 与
   `Resolved` 的字段里」的字段清单断言。
3. **两个数不同轴**：`retrieval_score` 是排序用的（无取值域给定），
   `ResolutionConfidence` 是「是否自动执行」用的（§275 要求**经过校准**）。
   故 `ResolutionConfidence` 的取值只能由**门槛判定**产出，不能由 `retrieval_score` 转换而来。

## 4.5 风险自适应门槛（§276）：输入照录，`f` 不发明

§276 的原文是 `required_resolution_strength = f(effect, cost, reversibility, privacy)`，
「风险越高，自动解析要求越严格」。**`f` 的形态规范没有给**，§2.2 也没有给。

本设计的处置与 D 对 `BudgetView` 的处置同形（把没给判据的东西做成**不可省略的输入**）：

```rust
/// §276 的四个风险事实。字段名逐字照录 §276，取值域规范未给（§16 第 6 条）。
pub struct ResolutionRisk {
    pub effect: Opaque,        // 待执行动作的效应
    pub cost: Opaque,          // 成本
    pub reversibility: Opaque, // 可逆性
    pub privacy: Opaque,       // 隐私等级
}
```

**`required_resolution_strength` 是必填入参，没有默认值**：
本层**不提供**任何把 `ResolutionRisk` 映射成门槛的基线实现。
理由：「风险越高要求越严格」是一条**单调性**要求，而单调性需要一个**序**——
`cost` 的高、`reversibility` 的低、`privacy` 的高，四者之间的**序与量纲规范都没有给**，
故一个「保守的」基线也无从写起（无中生有的序会把「高成本」判成「低风险」）。
**签名层面可断言**：`Resolver::resolve` 不收 `Option<RequiredStrength>`；
`RequiredStrength` 没有 `Default`。**运行期无照片**，与 D 的「预算是必填输入」同一处置（§15 第 2 条）。

## 4.6 Manual Binding 与 Alias 作用域（§277 §278）

**§277**：`ExplicitBinding { mention, entity, scope, source = USER }`，
「该 Binding SHOULD 在当前 scope 拥有最高优先级」。本设计照录该结构，
并把「最高优先级」落在**约束解析的取值顺序**上：`ExplicitBinding` 在
`CandidateSet` 过滤之前**先把候选集缩到 `entity` 这一个**（不是给它加权）。
这一条可断言：给一个与 `ExplicitBinding` 冲突的高分候选，断言结果仍是 `Resolved(binding.entity)`。

**`entity` 不在候选集里时给哪一态**（2026-10-06 补，计划作者查出本节原先没写）：
**不新增分支，走同一条路径**——把该实体**作为一条候选加入候选集**
（`evidence` 标 `user_explicit`），再照常过滤。理由三条：

1. **§277 的语义是「用户**选择候选**」**，而 binding 给的是**用户直接指定的实体**，
   其权威高于检索（「SHOULD 在当前 scope 拥有最高优先级」）。
   **若因为注册表没检索到就把它拒在集合外，等于让检索结果否决用户**——那是 §277 的反面。
2. **断言 A 因此不受影响**：`Resolved(e)` 的 `e` 仍**是候选集里的成员**
   （它就是被加入的那一条），A 的闭集性质保持。
3. **与它冲突的约束仍照常判**：若该实体同时违反某条显式约束（例如 `Scope` 不符），
   它与别的候选同罪，按 §4.2 的断言 B 走（全不合格 ⇒ `Unknown { AllCandidatesRejected }`）。
   **binding 不是「绕过约束」的口子**——它只是提高**检索**这一侧，不豁免 `ExplicitConstraint`。

**照片两条**：(a) `entity` 不在检索给的候选集里 → 结果 `Resolved(that entity)`，
且 `Resolved.candidate_count` 比检索集**多 1**（证明它确实被加入了）；
(b) **反例**：`entity` 不在集里**且**违反一条 `Scope` 约束 → `Unknown { AllCandidatesRejected }`
（证明 binding 不豁免约束）——**两侧都钉**。

**§278**：`TURN / SESSION / PROJECT / USER_PERSISTENT` 四个作用域，
「系统 **MUST NOT 默认把 Session Alias 永久化**」。落点：

1. `alias` 表有 `scope` 列，四值封闭（表外取值读回得具体 `Err`，照 P3D 的枚举列纪律）。
2. **没有从 `SESSION` 到 `USER_PERSISTENT` 的写入路径**：提升是一个**独立的显式操作**
   （`promote_alias(alias, USER_PERSISTENT)`），且**没有任何函数会顺带调用它**。
   照片两条：(a) 写一条 `SESSION` alias，断言 `alias` 表里 `scope='user_persistent'` 的行数为 0；
   (b) **两侧守卫**——显式提升的那条路径断言 `Ok` 且 scope 变了（只钉「不提升」那侧会让
   提升路径整个不存在也照样绿）。
3. **「会话结束」没有定义**：`TURN` 与 `SESSION` 的**过期时点**规范没给
   （§274 的 `session` / `conversation` 也没有定义）。故本设计只实现「不自动永久化」，
   **不实现会话边界的清退**。§16 第 7 条。

**ENG-009 由此闭合**（`docs/02-工程.md:698`：「Resolver 的 Alias 持久化存储形态」列为可推迟项）：
本层定为**一张 `alias` 表（migration 100），`scope` 列为封闭枚举**。
《工程》§10.2 的那一行需订正为已决，**收件人：《工程》文档维护者**。

---

# 5. Intent 存储与状态机（§222 §223）

## 5.1 数据模型

§222（`docs/spec/05-normative.md:130-169`）的**十五个字段**逐项处置
（下表十三行，`created_at`/`updated_at` 与 `parent_intent?`/`child_intents[]` 各一行两字段）：

| §222 字段 | 本设计的类型 | 说明 |
|---|---|---|
| `id` | `IntentId` | 本层定义（本仓尚无此类型） |
| `version` | `u32` | 见 §5.2 |
| `kind` | `IntentKind`，六值封闭 | §222 的六个：`TASK PRODUCT MAINTENANCE RESEARCH BACKGROUND RUNTIME_IMPROVEMENT` |
| `goal` | 规范化后的表示（§3.2） | §1.3 的类型边 |
| `status` | `IntentStatus`，见 §5.3 | §223 |
| `priority` | `Priority` | **§222 未给取值域**（§16 第 8 条） |
| `created_at` / `updated_at` | `i64`（毫秒） | 照 P0 的 `now_millis()` |
| `parent_intent?` / `child_intents[]` | `Option<IntentId>` / 由 `parent_intent` 反查 | **子列表不落列**：只存父指针，子列表由查询派生——两份真值会漂移（本仓一贯判为缺陷的那一类） |
| `contract_id` | `ContractRef { id, version }` | **规范只给 `contract_id`，本设计补版本**，理由见 §7.3 |
| `active_plan_id?` | `Option<PlanId>` | §222 |
| `graph_id?` | `Option<GraphIdRef>` | **不透明 `String`**（§1.2.3 的规则 3；取 `continuum_graph::GraphId` 会写出一条指向执行层的边） |
| `product_id?` | `Option<ProductIdRef>` | 同上，不透明 `String` |
| `completion_predicate` | `Opaque` | **§222 只给字段名**；判定属第 8 层（Completion Predicate），本层只存（§16 第 9 条） |

## 5.2 「版本」在这一层指什么

§64（`docs/总纲 §2.1`）要求「Intent 和 Contract 必须有版本」，§222 有 `version` 字段，
**但规范没有说 Intent 的版本在什么时候递增**。本设计的判定（**这是一个设计决定，不是规范转录**）：

- **`version` 在 Intent 的语义内容变化时递增**：`kind` / `goal` / `parent_intent` /
  `completion_predicate` / `contract_id` 五者之一改变时 +1。
- **`status` 迁移不递增**。理由：§223 的状态机在一次任务里会走十几个迁移，
  若每次迁移都 +1，「版本」就退化成事件计数，而 §64 要的是「**用户要求一定会变**」这件事的版本。
- **`updated_at` 与 `version` 是两件事**：状态迁移改前者，语义变化改两者。

落库形态：`intent` 表**一行一 Intent**（`id` 主键），`version` 是列；
**历史不进本表**——历史的权威是 P0 的 `events` 表（§310 的 `intent.created` / `intent.completed`）
与审计表。这与 Contract 的处理**刻意不同**（§7.3：§226 明写 MUST NOT 原地修改），
两者的差别来自规范原文的差别，不是不一致。

## 5.3 状态机（§223）

§223（`docs/spec/05-normative.md:172-206`）给了两组：一组是进入运行的五步链
（`PROPOSED → SPECIFYING → PLANNING → REVIEW → READY → RUNNING`），
一组是运行后的六个态（`WAITING SUSPENDED VERIFYING COMPLETED FAILED CANCELLED`，
另 `EVOLVING` 给 Product Intent）。

**§223 未给迁移关系**（给了态与顺序箭头，没给完整的合法对）。本设计的迁移表是**本设计的决定**，
与 D 对 §249 的处置同形（D 设计 §4.1，收件人：复审者）：

| 从 | 到 |
|---|---|
| `PROPOSED` | `SPECIFYING`、`CANCELLED` |
| `SPECIFYING` | `PLANNING`、`PROPOSED`（规格化失败回退）、`CANCELLED` |
| `PLANNING` | `REVIEW`、`SUSPENDED`、`CANCELLED` |
| `REVIEW` | `READY`、`PLANNING`（审查驳回）、`CANCELLED` |
| `READY` | `RUNNING`、`CANCELLED` |
| `RUNNING` | `WAITING`、`SUSPENDED`、`VERIFYING`、`FAILED`、`CANCELLED` |
| `WAITING` | `RUNNING`、`SUSPENDED`、`CANCELLED` |
| `SUSPENDED` | `RUNNING`、`CANCELLED` |
| `VERIFYING` | `COMPLETED`、`FAILED`、`RUNNING`（验证失败返工） |
| `COMPLETED` | `EVOLVING`（**仅 `kind = PRODUCT`**）、终态 |
| `EVOLVING` | `RUNNING`、`CANCELLED` |
| `FAILED` / `CANCELLED` | 终态 |

**自环单列**（照 P3D 的纪律：自环不靠矩阵顺带）：`RUNNING → RUNNING`、`WAITING → WAITING`
两条断言 `Ok`；`PROPOSED → PROPOSED`、`COMPLETED → COMPLETED` 两条断言
`Err(Illegal { from, to })`。**两侧都钉**——只钉放行那侧会让「没列进表的自环」漂过去。

**`COMPLETED → EVOLVING` 只在 `kind = PRODUCT` 时合法**（§223 原文「Product Intent MAY……」）。
照片：两条，`PRODUCT` 得 `Ok`、`TASK` 得 `Err(NotProduct { kind })`，断言是哪一枚。

## 5.4 落库、事件与审计

**写路径的形状照 P1 的 `apply_transition`**：状态迁移与对应 `Event` **在同一事务**内写入
（`crates/continuum-persist/src/tx.rs:41` 的 `commit` 由调用方给；P1 的同一约定见
`crates/continuum-graph/src/transition_tx.rs`）。**失败路径的纪律照 P1**
（`docs/superpowers/p1-followups.md` §一.1）：写入函数返回 `Err` 之后调用方必须回滚、不得提交——
故本层的写函数**都不自己 `commit`**，与 P1 同形。

**本层写六项**，其中**五项是首个生产方、一项不是**（`AuditKind::UserApprovals`）——逐项实测如下
（判据：`grep -rn "EventType::<X>" --include="*.rs" crates/`，排除 `continuum-events`
自身与测试目录后的命中数）：

| 类型 | 本层的写入点 | 依据 | 今天有无生产方（实测） |
|---|---|---|---|
| `EventType::IntentCreated` | `Intent` 首次落库 | §310 | **无**（仅 `continuum-persist/tests/transaction.rs` 出现） |
| `EventType::IntentCompleted` | 迁移到 `COMPLETED` | §310 | **无生产方**；但 `crates/continuum-persist/src/tx.rs:369` 是它的**消费者**，见下 |
| `EventType::PlanReviewRequired` | Plan 进入 `REVIEW` 前的审查挂起点 | §310 §229 | **无**（全仓零命中，连测试也没有） |
| `EventType::DecisionRequired` | `Decision` 创建（§10） | §310 §331 | **无**（全仓零命中） |
| `AuditKind::ContractChanges` | Contract 新版本创建（§7.3） | §313「contract changes」 §226 | **无**（除 `continuum-events` 外零命中） |
| `AuditKind::UserApprovals` | `DecisionResult` 落库、Plan 进 `USER_APPROVED`（§10.3、§9.3） | §313「user approvals」 §332 §229 | **已有**：`crates/continuum-workspace/src/gate.rs:222`、`:303`、`:348`（Integration Gate） |

> **订正（2026-10-06，评审查出）**：本节初稿写「本层是四个 `EventType` 与**两个** `AuditKind` 的
> 首个生产方（实测：全仓除 `continuum-events` 与测试外零命中）」。**`AuditKind::UserApprovals`
> 那一半是假的**——`continuum-workspace` 的 Integration Gate 早已在三处写它
> （`apply_patch` / `cherry_pick` / `merge`，同文件 `:42` 的模块文档也写着这三项归它）。
> **更坏的是那句括注「实测……零命中」**：本设计查过那三个文件（`grep` 的输出里就有它们），
> 仍写出了「零命中」——**这是一句没做过的实测被写成做过**，与本项目反复栽的同一形状。
> 错因：把「`IntentCreated` 只在测试里出现」这半边的结论顺手扩到了整行括注。
> **原句与错因留在此处。**

两处由此改写的细节：

1. **`AuditKind::UserApprovals` 的载荷形状因此不止一种**：Gate 写的是操作名
   （`"apply_patch"` 等，`crates/continuum-workspace/src/gate.rs:224`），本层写的是决策单与
   被选项。`append_audit` 的 `payload` 是自由 JSON（`crates/continuum-persist/src/tx.rs:79-84`），
   **同一个 kind 下不保证载荷同形**——读 `audit_log` 的一方不得假定「一个 kind 一种载荷」。
   这是本层引入的第二种形状，**据实记在此**（不改 Gate，也不新增 kind）。
2. **`EventType::IntentCompleted` 已经有一个消费者，它带一条 MUST**：
   `crates/continuum-persist/src/tx.rs:369-373` 在事件日志扫描时断言
   「**可跳过事件之后存在 `intent.completed`**」即报 `Err`。
   故本层**写 `IntentCompleted` 的位置必须满足**：它之前不得有可跳过（不可解码／版本未知）的事件。
   本层无法控制别层在此之前写了什么，**但可以保证自己的写入点在链尾**——
   这条是既有的消费者强加给本层写路径的约束，记在此以免实现时踩上。

**`EventType` / `AuditKind` 的变体一个不改、一张表不加**（两个枚举已含所需项）。
**`EventType::ALL` 是第三份清单**（`crates/continuum-events/src/event.rs:35-45`），
本设计不动它；本层**只使用**上表六项，故不需要新变体。

---

# 6. Specification 编译（§227 §214）

## 6.1 六元组与 `unknowns` 的强制形态

§227（`docs/spec/05-normative.md:313-331`）给六元组，并明写两句 MUST：
「`unknowns` MUST 被正式保存」与「系统 MUST NOT 把未知项自动转换成假设」。
§214（`docs/spec/04-method.md:1105-1138`）给的是过程：
`User Intent → Specification Mining → 发现隐含要求 → 检测冲突/未定义行为 → 形成 Executable Specification`，
末句「**只有关键歧义才询问用户**」。

**MUST 的落点是一个类型，不是一个约定**：

```rust
/// §227 的六元组。**无公开构造函数**——唯一构造入口是 `SpecDraft::finalize`。
pub struct Specification {
    goals: Vec<Goal>,
    invariants: Vec<Invariant>,
    acceptance_predicates: Vec<AcceptancePredicate>,
    forbidden_states: Vec<ForbiddenState>,
    preferences: Vec<Preference>,
    unknowns: Unknowns,          // 非 Option，非 Vec 的裸形态
}

pub enum Unknowns {
    /// 显式枚举出的未知项。
    Enumerated(Vec<Unknown>),
    /// 「已查过、没有未知项」。
    NoneFound,
}
```

**为什么不是 `Vec<Unknown>`**：`vec![]` 同时表示「已查过、没有」与「没查」。
§227 禁的是后者被当成前者（「把未知项自动转换成假设」）。
`Unknowns` 的两个变体让「没查」**不可表达**：`SpecDraft` 里没有 `unknowns` 字段，
`finalize` 要求调用方**显式给出** `Unknowns` 之一。
**照片**：trybuild 断言 `Specification` crate 外不可构造；`SpecDraft::finalize` 的签名里
`Unknowns` 不是 `Option`；另加一条「`Specification` 的字段清单**逐项**」断言（加一个字段即红）。

**§214 的「检测冲突/未定义行为」未给判据**（「什么算冲突」规范没给），
故本设计只建**类型**（`Conflict` / `UndefinedBehavior` 作为 `Unknowns::Enumerated` 的成员），
**不建检测器**。§16 第 10 条。

## 6.2 与 Intent 的关系

Specification 挂在 Intent 上、有版本（§64 的 Intent 版本化连带）。
**Specification 的版本与 Intent 的版本同号**：一次语义变化产出一个新的 Intent version 与
一个对应的 Specification；**两者同事务写**。这一条排除了「Intent 已经是 v3、Specification 还是 v1」
这种可漂移的状态。照片：写 Intent v3 时不写 Specification 断言事务回滚且两张表都不变。

---

# 7. Contract 编译与版本化（§224 §226）

## 7.1 Requirement 的两个字段（§224）

§224（`docs/spec/05-normative.md:209-254`）给 `Requirement { id, class, expression, source, verification_requirement }`
与四个 `class` 值。逐项：

| 字段 | 本设计的类型 | 说明 |
|---|---|---|
| `id` | `RequirementId` | |
| `class` | `RequirementClass`，四值封闭 | `REQUIRED PREFERRED FLEXIBLE UNSPECIFIED`。**逐项照片**：四值各一条用例 |
| `expression` | `Opaque`（`String`） | **§224 未给表达式的语法**，本层**不解析**它。§225 的判定（不得修改 REQUIRED）判的是 `class`，不是表达式（§11.3） |
| `source` | `Opaque` | 未给取值域 |
| `verification_requirement` | `VerificationRequirement`，**只存不判** | 见 §7.2 |

`TaskContract` 的其余字段（`budget` / `authority_limit` / `data_policy` / `model_policy` /
`node_policy` / `acceptance_predicate`）**规范只给名字**，本设计一律取 `Opaque`
（不发明取值域）。其中 `budget` 与 §12 的预算**不是同一个东西**：
§224 的 `budget` 是 Contract 里的**声明字段**，§333 的 `Budget` 是**记账对象**。
两者的关系**规范没给**（§16 第 11 条）。

## 7.2 `verification_requirement`：OPEN-001 未决期间只存不判

《工程》§2.5 明写（`docs/02-工程.md:138-140`）：该字段**可以存储**，
但在 OPEN-001 解决前**无法判定其是否被满足**——「这不阻断本层建设，阻断的是第 8 层的验证收口」。

故本设计的落点是**结构性禁读**：

- `VerificationRequirement` 是一个 newtype，**只有** `as_str` 与 `from_str`，**没有**
  `is_satisfied` / `check` / `evaluate` 这类方法。
- 本层**没有任何函数**以它为输入做判定（第 8 层完成验证收口之前，它只被读写与透传）。
- **照片**：`VerificationRequirement` 的公开方法集**逐条**断言；另加一条「`contract_requirement`
  表的列清单**逐列**」（加一列即红）。

**这是「不许发明规范没给判据的东西」在本层最直接的一次兑现**：
OPEN-001 的三个可选方向（a/b/c）一个都不选，本层只把字段**完整保存下来**。

## 7.3 版本化（§226）与 `ContractRef`

§226（`docs/spec/05-normative.md:288-310`）：用户改要求时 MUST 创建 `Contract vN+1`，
**MUST NOT 原地修改**；MUST 计算 `ContractDiff`；受影响的 ADFIR 节点 MUST 被重新判定有效性。

落库：`contract` 表主键 `(id, version)`，**无 UPDATE 路径**（`save_contract` 只 INSERT，
二次写同一 `(id, version)` 撞主键得 `Err`——照片：裸 `INSERT` 断言主键拒且原行不变）。
每个新版本在同一事务内写一条 `AuditKind::ContractChanges`（§5.4）。

**`ContractRef { id, version }`**：§222 的 `Intent.contract_id` 与 §235 的 `ADFIRGraph.contract_id`
**都只有 id、没有版本**，而 §226 的 ContractDiff 消费**必须知道基准版本**
（否则「哪些节点受影响」没有比较基准）。故：

- **本层的 `Intent.contract_id` 取 `ContractRef { id, version }`**（§5.1 已记）。
- **执行层的 `adfir_graph` 也需要版本**——那是对已建之物的一处请求，见 §13.2。

**§226 的「受影响的 ADFIR 节点」判定的归属**：diff 的四类差异由本层算（§8.1），
**节点侧的判定与施加在执行层**（§8.2）。本层的产出是判定所需的最小信息，不是节点集合。

---

# 8. ContractDiff（§226）

## 8.1 四类差异的判定

§226 只给了四个名字：`added / removed / strengthened / weakened`。
**四类各自怎么判，规范没有给**。本设计给出的是**本设计的判定**（不是规范转录），
判据只能落在 §224 给的字段上（`class` 与 `expression` / `verification_requirement`）：

| 差异类 | 判定 | 依据 |
|---|---|---|
| `added` | `(requirement id)` 在 vN 无、vN+1 有 | §226 的名字 |
| `removed` | 在 vN 有、vN+1 无 | 同上 |
| `strengthened` | 同 id 两边都有，且 `class` 的**序**上升 | 序是 `UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`；`class` 相同时 `expression` 变了则**不下判**（见下） |
| `weakened` | 同 id 两边都有，且 `class` 的序下降 | 同上 |

**那个序是本设计定的，不是原文给的**：

> **订正（2026-10-06，评审查出）**：初稿把它写成「**这个序是 `class` 四值的自然序**」并引
> `docs/总纲 §2.3`。**该处（`docs/01-总纲.md:153-160`）只给四类的含义，未给序**，
> 且总纲的列举顺序与上表的序**相反**。这恰是 §16 第 12 条自认「本设计的判定」的那一条，
> 而初稿的措辞读起来像转录。**原句留在此处。**
> **序的依据是四类含义的强弱**（「必须满足」＞「尽量满足」＞「可调整」＞「未指定」），
> 那是本设计从含义推出来的，**「自然」二字不成立**。
> **后果**：若复审判定此序不成立，则 `strengthened` / `weakened` **无判据**——
> 两类的判定会整个落进下面的 `unjudged`（§16 第 12 条）。

**`class` 相同时的 `expression` 变化不下判**——这是本设计**刻意留下的未判定**：
§224 没有给表达式的语法（§7.1），故「`duration >= 4K` 改成 `duration >= 2K`」这种
**语义上的削弱无法判定**。本设计**不发明一套表达式语言**去判它。

**未判定另立并列字段，不进差异类**：

```rust
/// §226 的四类差异。**恰好四枚**，与 §226 的枚举一一对应。
pub enum DiffClass { Added, Removed, Strengthened, Weakened }

pub struct DiffEntry { requirement_id: RequirementId, class: DiffClass }

/// 本层判不了的变更。**不是差异类**——它是「记录」，不是「判定」。
pub struct UnjudgedChange { requirement_id: RequirementId, reason: UnjudgedReason }

pub struct ContractDiff {
    from: ContractRef,
    to: ContractRef,
    entries: Vec<DiffEntry>,          // 恰好 §226 的四类
    unjudged: Vec<UnjudgedChange>,    // 并列字段，**不并入 entries**
}
```

> **订正（2026-10-06，评审查出）**：初稿写「`ContractDiff` 增加一个**第五类** `ExpressionChanged { id }`，
> 它不是 §226 的四类之一」。**「不是四类之一」与「是第五类」是矛盾的**：
> §226 给的是一个**规范枚举**，把它放进**同一个类别域**——无论措辞怎么声明——
> 类型上就是第五种差异，下游（用例名、将来执行层的 `match`）也必然这么读。
> **判据**：只要那一项不在规范的枚举位置上，它就是「记录」；一旦成为 diff 类型的第五个变体，
> 它就是「差异类」。故改为上面那个形状：四类留在 `DiffClass`，未判定走并列字段 `unjudged`。
> 原句留在此处。**这一改同时收敛了 §16 第 32 条的三条路**（见该条）。

**照片**：`DiffClass` 的四枚各一条用例；`unjudged` 一条用例，
且**断言它不出现在 `entries` 里**——这条断言把「记录 vs 差异类」的分界钉死。

## 8.2 输出被第 3 层消费：接口形状与「谁施加」

§226 的「受影响的 ADFIR 节点 MUST 被重新判定有效性」是一个**跨层义务**。
分工按 §1.2.3：

- **本层产出**：`ContractDiff { from, to, entries: Vec<DiffEntry>, unjudged: Vec<UnjudgedChange> }`（§8.1）。
  `DiffEntry` 带 `requirement_id` 与 `DiffClass`；`UnjudgedChange` 是**并列字段**，不是差异类。
- **执行层施加**：由**持有图的一侧**把 `DiffEntry` 映射到节点（§235 的 `ADFIRGraph.contract_id`、
  §236 的 `Node.constraints` / `Node.execution_policy`）并调 `propagate_invalidation`
  （`crates/continuum-graph/src/invalidation.rs:15`）。
- **本层不调 `propagate_invalidation`**，也不引用 `continuum-graph`。

**今天这条边没有消费者**（执行器未建）→ §16 第 13 条。

---

# 9. Plan 与审查状态机（§228 §229 §338–§341）＋ Plan Change 分类（§230）

## 9.1 §228 的数据模型

§228（`docs/spec/05-normative.md:334-360`）：`Plan { id, version, intent_id, type, objectives[], milestones[],
assumptions[], risks[], acceptance_conditions[], review_state }`，`type ∈ {STRATEGIC, EXECUTION}`。

`version` 取**两段** `(major: u32, minor: u32)`——**依据是 §230**：MINOR 变更（换模型、重试、
并行调整、工具替换、内部步骤重排）与 MATERIAL 变更（重大架构变化、Requirement 改变、权限扩大、
成本显著扩大、Milestone 目标变化）是两类，而 §213（《总纲》§2.6）把它们对应到「v1.1 可自动」与
「v2 需重审」。故 `major` 在 MATERIAL 时 +1、`minor` 在 MINOR 时 +1。
**§228 只给 `version` 一个字段名**，两段式是本设计的决定（收件人：复审者）。

## 9.2 审查状态机（§229）与三层阶梯（§338）

§229 六态：`DRAFT → SELF_REVIEWED → AI_REVIEWED → USER_APPROVED`，另 `REJECTED` / `SUPERSEDED`。
`Strategic Plan` 在需要重大用户承诺时 MUST 进 `USER_APPROVED` 才可执行。

**迁移关系 §229 未给**（给了顺序与两个旁支态）。本设计的迁移表（本设计的决定）：

| 从 | 到 |
|---|---|
| `DRAFT` | `SELF_REVIEWED`、`REJECTED` |
| `SELF_REVIEWED` | `AI_REVIEWED`、`DRAFT`（自审发现内部不一致，§208 允许自动修正）、`REJECTED` |
| `AI_REVIEWED` | `USER_APPROVED`、`DRAFT`（修订，见下）、`REJECTED` |
| `USER_APPROVED` | `SUPERSEDED`（新版本产生时） |
| `REJECTED` / `SUPERSEDED` | 终态 |

**「修订」不是 §229 的第六个态**，而是**产生新版本 + 旧版本进 `SUPERSEDED`**：
§338 的阶梯是 `草稿 → 自审 → 独立强 AI 审查 → **修订** → 用户审查 → 批准`，
「修订」是**一次新版本**（§213 的版本化），不是原地改状态。故 `AI_REVIEWED --修订--> ` 的
目标态是**新版本的 `DRAFT`**，旧版本转 `SUPERSEDED`。这一条说的是规划与审查之间的**循环**，
而 §229 的链是**一个版本内**的链——两者不冲突。

**`type = EXECUTION` 的 Plan 不进 `USER_APPROVED`**：§229 的 MUST 只对 Strategic Plan
（「Strategic Plan MUST 在需要重大用户承诺时进入 USER_APPROVED 才可以执行」），
而 §341 把用户审查的范围限定在战略层。照片：两条——`STRATEGIC` 未经 `USER_APPROVED`
去 `READY` 得 `Err(UserApprovalRequired)`；`EXECUTION` 同路径得 `Ok`。**两侧都钉**。

## 9.3 `ReviewFinding`（§339）与自审／独立审查的分离（§340）

§339：`ReviewFinding { severity, subject, problem, evidence, recommendation }`，
`severity ∈ {BLOCKER, MAJOR, MINOR, SUGGESTION}`。**照录**，四个 severity 逐项照片。

§340：自审 MAY 同一模型；独立审查 SHOULD 用独立上下文、优先更强模型、必要时跨 family。
**这三条是 SHOULD，不是 MUST**，故本设计**不**在状态机上禁止「同一模型 + 同一上下文」的审查
（那会把 SHOULD 抬成 MUST = 发明判据）。**本设计的落点是「使事实可观察」**：

```rust
/// §340 的三条独立性事实。**必填**——不是布尔标志位，是三条各带来源的记录。
pub struct ReviewIndependence {
    pub separate_context: bool,
    pub reviewer_model: ModelRefRef,   // 不透明 String（§1.2.3 规则 3）
    pub author_model: ModelRefRef,
    pub cross_family: bool,
}
```

**可断言的两条**：(a) `ReviewFinding` 记录必带 `ReviewIndependence`（字段清单逐项）；
(b) 「同一模型 + 同一上下文」的审查**在记录上可分辨**——用例构造一次这样的审查，
断言读回来的 `separate_context = false` 且两个 model ref 相同。
**「分离到什么程度算够」的判定规范没给** → §16 第 14 条。

## 9.4 Plan Change 分类（§230）

§230：`PlanChange { class, affected_scope }`，`class ∈ {MINOR, MATERIAL}`，MATERIAL MUST 重触发审查门。

- 两个 class 逐项照片。
- `affected_scope` **§230 未给取值域** → `Opaque`（§16 第 15 条）。
- §230 的十个例（**五个 MINOR ＋ 五个 MATERIAL**，`docs/spec/05-normative.md:403-417`）
  **逐条给一条分类用例**——「枚举断言须逐项有照片」。

  > **订正（2026-10-06，评审查出）**：初稿写「**十一个**例（五个 MINOR、五个 MATERIAL，原文实为五个 MATERIAL）」。
  > **「十一」与括号里自己写的「五个＋五个」相抵**，括号那半句本身也读不通（原文本来就是两个五）。
  > 错因是把数数错了又用一句解释去圆它。**原句留在此处。**
- **MATERIAL 的判定需要看的量（权限、成本、架构）从哪来**：`权限扩大` 与 `成本显著扩大`
  需要计划侧声明的影响面（§11.2 的 `PlanFootprint`）。故 `PlanChange` 的分类函数输入是
  `(&Plan, &Plan, &PlanFootprint, &PlanFootprint)`。**「显著扩大」的阈值规范未给** → §16 第 15 条。

## 9.5 用户审查视图（§341）

§341（`docs/spec/05-normative.md:2661-2678`）列的清单是**八项**，逐行照录：

```
目标
重大设计
关键约束
Milestone
风险
成本
权限
验收条件
```

**`成本` 与 `权限` 是分列的两行**，不是一行。

> **订正（2026-10-06，评审查出）**：本节初稿把这两行合并成一个字段
> `cost_and_authority`，并写「**七项**（原文如此，第七项与第八项合并计为七类）」。
> **那句话把一个本层的合并说成了规范原文**——「原文如此」四个字是假的：§341 写的是八行。
> 这是「把设计里说了的写成规范里有的」的同一形状。**原句与错因留在此处**，
> 以免后来者以为八项是后人加的。

**落点是一个投影类型**，字段**恰好**这八项，且**没有**指向内部执行步骤的字段：

```rust
/// §341 的用户审查视图。**字段恰好八项**，逐项断言。
/// 字段名与 §341 的八行**一一对应**，不合并任何两行。
pub struct UserReviewView {
    pub objectives: ...,            // 目标
    pub major_designs: ...,         // 重大设计
    pub key_constraints: ...,       // 关键约束
    pub milestones: ...,            // Milestone
    pub risks: ...,                 // 风险
    pub cost: ...,                  // 成本
    pub authority: ...,             // 权限
    pub acceptance_conditions: ..., // 验收条件
}
```

**八项里任何两项都不得合并**：合并会改变「用户看得到什么」这一事实，
而 §341 的八行是这个视图的**全部**判据。若将来判定「成本与权限在界面上应并排显示」，
那是**呈现方**的事，不是这个投影类型的事（本组件不是 UI，见末段）——
**本设计不做这个合并，也不为它留字段**。

**照片**：字段清单逐项断言——**多一项即红、少一项即红、`成本` 与 `权限` 合并成一项即红**。
这是负向的守卫，本仓接受否定式照片（见 `docs/superpowers/p3bcdf-followups.md` §四.4）。

> **订正（2026-10-06，修后复核查出）**：本句初稿只写「**多一项即红**」，
> **而这次出错的恰恰是「少」那一侧**——初稿合并成七个字段，是**少**了一项；
> 「多一项即红」守不住它。§14 的同一条守卫（`:1557`）当时已写全（多／少／合并三侧），
> **§9.5 没写全，两处对同一个守卫的表述不等宽**。这是「守卫须两侧都钉」
> 在**同一个文件内两处**的形态。原句留在此处。

**本组件不是 UI**：§2.1 的组件表里没有「用户界面」组件，本层只产出这个视图；它的呈现方不在本层。

---

# 10. Decision 对象与恢复（§331 §332）

## 10.1 对象与触发

§331：`Decision { issue, options[], consequences[], blocking }`，「而不是只发一句自由文本问题」。
§55（《总纲》§2.7）列了六条典型触发条件：违反 REQUIRED、高成本探索、收费计算、高风险副作用、
权限升级、Trust 变化；并明写「Agent 内部的局部执行问题不应默认升级为用户决策」。

**本层的触发点是两个**（其余触发器在别的层）：
1. **Constraint Validator 判违反 REQUIRED**（§6：「违反 REQUIRED → 挂起 → 请求用户决定」）；
2. **Budget Validator 判预计超支且找不到合法低成本方案**（§110）。

两条都产出一个 `Decision`，`blocking = true`。**`blocking` 的取值域 §331 未给**（一个布尔，
`false` 的语义「不阻塞」在规范里没有用武之地），本设计保留该字段并**只在本层的两个触发点写 `true`**，
**不发明 `false` 的触发条件**（§16 第 16 条）。

**命名冲突一处，须写明**：`continuum-policy` 已有 `Decision`（`Allow / Deny / RequireApproval`，
`crates/continuum-policy/src/rule.rs:88`）。**本层的 §331 对象与它不是同一个概念**
（一个是策略裁决的三值，一个是给用户的决策单），但**同名**。
本设计的处置：本层的类型名为 `Decision`（规范的名字不换），
**在同时引用两者的模块里，`continuum_policy` 的那个一律以 `PolicyDecision` 别名导入**。
照片：`continuum-semantics` 的公开面清单里 `Decision` 恰好一个（不是两个）。

**`Decision` 的落库与读回**（§10.2 的恢复要用）：`options[]` 与 `consequences[]` 是
**等长的配对**（§106 的例子：每个选项带后果）。**「每个选项带后果」的强制落在构造期**：
`DecisionOption::new(option, consequence)` 不接受缺后果的选项，
故「等长」由类型保证而不是由检查保证。照片：`options.len() == consequences.len()` 对每个读回的
`Decision` 成立（枚举式的属性断言），以及「不存在只带选项的构造路径」（trybuild）。

## 10.2 恢复链（§332）

§332：`DecisionResult → Contract / Plan update → Graph invalidation → Resume`。
另 §107：「用户选择必须作为结构化 Decision Result 处理，**不得将其视为普通会话消息并重新解释整个任务**」。

四步的前三步的归属：

| 步 | 谁做 | 依据 |
|---|---|---|
| `DecisionResult` 落库 | 本层（`decision_result` 表 + `AuditKind::UserApprovals`） | §313 §332 |
| `Contract` / `Plan` update | 本层：**产生新版本**（§7.3 的 MUST NOT 原地修改，§9.1 的版本） | §226 §213 §332 |
| `Graph invalidation` | **执行层**（本层不碰图，§8.2） | §306 §332 |
| `Resume` | 本层：Intent 从 `SUSPENDED` 回 `RUNNING`（§5.3） | §223 |

**「不得重新解释整个任务」的落点**：`DecisionResult` 是一个**结构化的、指向某个 `Decision` 与某个
`DecisionOption` 的值**，`apply_result` 的输入只有它（不是自由文本、不是重跑 Intent Compiler）。
照片：两条——(a) 一个 `DecisionResult` 只改被它指名的 Requirement / Plan 字段，
断言**其余字段逐项不变**；(b) 不存在「把 `DecisionResult` 序列化成文本再重新编译」的路径
（公开面清单断言）。

**恢复的原子性**：`DecisionResult` 落库 + Contract/Plan 新版本 + Intent 状态回迁
**在同一事务**。失败路径照片：任一步失败断言四张表（`decision_result` / `contract` / `plan` / `intent`）
**都没有半写的行**（不是只断言返回哪一种 `Err`）。

---

# 11. Constraint Validator（§225 §6）

## 11.1 两道门必须独立

§6 的图：`候选计划 → Constraint Validator → 满足 Contract → 执行 / 违反 REQUIRED → 挂起 → Ask User Decision`。
§2.2 明写「Constraint Validator 与 Budget Validator **必须分离**（§6 §7）。前者判定合法性，
后者判定成本可接受性」。**分离的落点是两个 crate**（§2.1）＋ §11.5 的两侧守卫。

## 11.2 输入：候选计划是**值**

§2.2：「Constraint Validator 输入候选计划与当前 Contract，输出通过或违反」。
「候选计划」的载体在本设计里由两部分组成，**都由调用方以值给出**：

```rust
/// 候选计划 = §228 的 Plan ＋ 声明的影响面。
pub struct CandidatePlan<'a> {
    plan: &'a Plan,
    footprint: &'a PlanFootprint,
}

/// 计划的声明影响面。**§228 的 Plan 没有这几个字段**——
/// 本类型是本设计的决定，理由是 §225 的四条 MUST NOT 需要它们才有判据（见下）。
pub struct PlanFootprint {
    pub authority: Opaque,            // 计划要用的权限上限
    pub privacy_class: Opaque,        // 计划触及的最高隐私等级
    pub external_effects: Vec<Opaque>, // 计划要产生的**外部副作用**（类型 + 目标）
    pub estimated_cost: Opaque,       // 计划的成本估计（§110 的输入，§12.4）
}
```

**为什么必须有 `PlanFootprint`**：§225 的四条 MUST NOT 是
「修改 REQUIRED / **扩大权限** / **降低隐私约束** / **擅自增加高风险外部副作用**」——
后三条比的是**计划的声明值**与 **Contract 的 `authority_limit` / `data_policy`**（§224 有这两个字段）。
没有计划侧的对偶声明，后三条**无可判**。故本类型是「让 §225 有判据」的载体，
**不是本层新造的判据**：判据是 §225 给的，本类型只是它的第二个操作数。

**图侧的输入以原样类型进入**：`Node.constraints: Vec<String>`（`crates/continuum-graph/src/node.rs:15`）
与 `Node.execution_policy: serde_json::Value`（同文件 `:18`，其 `/// 结构保存，判定属 P4。` 在 `:17`）
**不经过任何本层的类型转换**就被读：

- `Node.constraints` 的**词汇表没有规范来源**（§236 只给字段名，`docs/spec/05-normative.md:558-580`）
  → 本层把每个串当作**一个 REQUIRED 约束的引用键**（按字符串比对 Contract 的 Requirement），
  **不发明约束语言的语法**。§16 第 17 条。
  **按本设计 §11.2 的这个读法**，一个匹配不到任何 Requirement 的键是**悬空引用**，
  它的处置是 §11.3 的 `Err(UnknownConstraintKey { key })`（拦下，不是放行）。

  > **订正（2026-10-06，评审查出）**：初稿把这一处的后果写成「按**今天的**读法，一个拼错的约束键
  > 会静默不匹配（fail-open）」。**「今天的读法」不存在**——全仓 `constraints` 只有两处用法
  > （`crates/continuum-graph/src/persist.rs:103` 的序列化与 `:253` 的反序列化），
  > **零逻辑读者**，故今天没有任何「读法」。那个读法是**本设计 §11.2 定的**。
  > 并且那个 fail-open **原先只写在遗留项里**（§16 第 17 条），
  > **裁决类型上看不出来**——现已落进 §11.3 的 `ConstraintError`。原句留在此处。
- `Node.execution_policy` 的判定按 §225 的 MAY/MUST NOT 两条清单：
  `换模型 / 换工具 / 换节点 / 换算法 / 重新规划 / 改变并行度 / 改变内部执行顺序` 属 MAY（放行）；
  `扩大权限 / 降低隐私约束 / 增加高风险外部副作用` 属 MUST NOT（拦下）。
  **用例数：7（MAY 逐项放行）＋ 3（这三条禁令逐项拦下）＝ 10 条**；
  §225 的**第四条** MUST NOT（「修改 REQUIRED」）不是本表的行，它是 §11.3 的 `ViolatesRequired` 那一条。

  > **订正（2026-10-06，评审查出）**：初稿写「**（14 项）**」。§225
  > （`docs/spec/05-normative.md:257-285`）实为 **MAY 七条 ＋ MUST NOT 四条**，
  > 故 7＋3＝10、7＋4＝11，**都不是 14**。原句留在此处。

## 11.3 判定：**两条**拦下路径，加一条「判不了」

§6 的图给的是一条拦下路径：「**违反 REQUIRED**」。而 §225
（`docs/spec/05-normative.md:257-285`）另给一组**不依赖任何 Requirement 的 `class`** 就能判的禁令。

> **订正（2026-10-06，评审查出）**：本节初稿标题写「**REQUIRED 是唯一的拦下条件**」，
> **与同节的 `ViolatesProhibition` 那一臂自相矛盾**——那条禁令不依赖任何 Requirement 的 class
> 就能拦下。初稿又把 §225 的 MUST NOT 说成「三条」，**实为四条**
> （修改 REQUIRED／扩大权限／降低隐私约束／擅自增加高风险外部副作用）；
> 其中「修改 REQUIRED」即 `ViolatesRequired` 那一臂，故禁令臂是其余**三条**。
> 原本「三条 MUST NOT」这一作用域比事实窄。**原句与错因留在此处。**

**§225 的四条 MUST NOT 与两臂的对应**（逐条，不许漏）：

| §225 的 MUST NOT | 落在哪一臂 |
|---|---|
| 修改 REQUIRED | `ViolatesRequired { requirement }` |
| 扩大权限 | `ViolatesProhibition { Authority }` |
| 降低隐私约束 | `ViolatesProhibition { Privacy }` |
| 擅自增加高风险外部副作用 | `ViolatesProhibition { Effect }` |

判定函数**不是**「通过／违反／不确定」——「不确定」在 §6 里没有位置：

```rust
pub enum ConstraintVerdict {
    Satisfied,
    ViolatesRequired { requirement: RequirementId },
    ViolatesProhibition { what: ViolatedProhibition },
}
```

**「判不了」走 `Err`，不走第三个裁决值**（评审查出，本节新增）：

```rust
pub enum ConstraintError {
    /// `Node.constraints` 里的一个串**不在当前 Contract 的 Requirement id 集合里**。
    ///
    /// **判据与 `class` 无关**——命中的那条 Requirement 是 `REQUIRED` 还是 `PREFERRED`
    /// 都不改变本变体的触发条件（见下第三条判据）。
    UnknownConstraintKey { key: String },
}
pub fn validate(...) -> Result<ConstraintVerdict, ConstraintError>
```

判据三条：

1. **一个悬空的约束键不是「满足」**：它意味着**写它的那一方与当前 Contract 对不上**
   （计划是在旧 Contract 下写的、或键拼错了），`Satisfied` 会让它以「已判过、通过」的形式流下去
   ——**这正是 fail-open**。**故它必须被拦下**，而「拦下」在 `Result` 上的形态是 `Err`。
2. **它也不该变成一个 §6 的裁决值**：§6 只给「满足 Contract／违反 REQUIRED」两种结果，
   往 `ConstraintVerdict` 里加第三支等于把「本层判不了」说成一个 §6 的结论。
   `Result` 的 `Err` **不可被忽略地当成通过**——调用方必须显式处置它。
3. **触发条件是「不在 id 集合里」，不是「不指向 REQUIRED」**——这一格是本节的要害：

   > **订正（2026-10-06，评审查出）**：本节初稿把判据写成「**匹配不到当前 Contract 的任何
   > Requirement**」，而 §11.2 写的是「每个串当作**一个 REQUIRED 约束的引用键**」——
   > 两句**不等价**，差的那一格会出事：若节点写了一个**存在、但 `class` 是
   > `PREFERRED`/`FLEXIBLE`/`UNSPECIFIED`** 的 Requirement id，
   > 按「不指向 REQUIRED」判就成 `Err`，**于是 §11.3 的「三类违反不拦下」在同一条路径上被绕过**
   > ——**同一件事两条路给出相反结果**。原句留在此处。
   >
   > 故 `Err` 的判据明确定为「**`key` 不在当前 Contract 的 Requirement id 集合里**」，
   > **与 `class` 无关**。§11.2 那句「当作 REQUIRED 约束的引用键」说的是
   > 「命中之后**按 REQUIRED 那一路去判**」，**不是**「只认 REQUIRED 的键」——
   > 命中一条非 REQUIRED 的键时，裁决由该 Requirement 的 `class` 决定
   > （`PREFERRED`/`FLEXIBLE`/`UNSPECIFIED` 一律**不拦下**）。

**照片三条，三侧都钉**：

1. `unknown_constraint_key_is_an_error`——`constraints` 里放一个 Contract 里不存在的键，
   断言 `Err(UnknownConstraintKey { key })`，且 `key` 是**那一个**。
2. `empty_constraints_is_not_an_error`——`constraints` 为空**不是** `Err`（空集合法，
   `Node::new` 就置空，`crates/continuum-graph/src/node.rs:31`）。
3. **`a_key_pointing_at_a_preferred_requirement_is_not_an_error`**（评审查出后补的**第三条**）——
   键命中一条 `class = PREFERRED` 的 Requirement：断言**不是 `Err`、也不拦下**
   （该 Requirement 的违反按 §11.3 末段走「不拦下」）。
   这条照片钉的正是第三条判据：**没有它，「不指向 REQUIRED 就报错」的实现照样全绿**，
   而那样会绕掉 §11.3。

**三侧合起来才算钉住**：悬空键 `Err`（1）／空表不是 `Err`（2）／**命中非 REQUIRED 键不是 `Err`**（3）。

**`PREFERRED` / `FLEXIBLE` / `UNSPECIFIED` 的违反不拦下**——§224 的四类含义
（`docs/总纲 §2.3`）明写 PREFERRED「有充分理由时可调整」、FLEXIBLE「明确允许 Runtime 自动调整」。
照片：三条 class 各一条「违反但不拦下」（`Ok(Satisfied)` 或带记录的放行），
`class` 为 `REQUIRED` 时一条 `ViolatesRequired`，三条禁令各一条 `ViolatesProhibition`。
**§11.2 的用例数与本节分工**：§11.2 的 10 条管 `execution_policy`，本节的 5 条管裁决。

## 11.4 违反之后的挂起与 Decision

§6：「违反 REQUIRED → Suspend → Ask User Decision」。落点：
`ConstraintVerdict::Violates*` 由调用方转成 `Decision`（§10.1 第 1 条触发点）与 Intent 的
`SUSPENDED`（§5.3）。**本组件自己不挂起、不写 Intent 状态**——
§2.3 的层内依赖把它写成 `Constraint Validator ← Contract + Plan`，
它**不依赖 Intent 存储**，故它只返回裁决。**挂起与建 Decision 是调用方（`continuum-semantics` 的
Intent 路径）的下一步**，这也是为什么三者在同一个 crate 里。

## 11.5 两侧守卫：分离必须是双向的

§2.2 的「必须分离」若只钉一侧（例如「Constraint Validator 不读预算」），
**反方向那侧是 fail-open 的那侧**——「预算判定偷偷把不合法的计划放行」不会有任何断言变红。
故两侧都钉：

1. `continuum-semantics` 的 `ALLOWED` 条目里**没有** `continuum-budget`（依赖边不存在）；
2. `continuum-budget` 的公开面里**没有**任何以 `CandidatePlan` 为参数的判定函数
   （方法集逐条断言）；
3. 反向：`continuum-budget` 的 `ALLOWED` 条目里**有** `continuum-semantics`
   （它要 `IntentId`），而**没有任何以 `ConstraintVerdict` 为输入的函数**。

---

# 12. Budget、探索预算与 Budget Validator（§333 §334 §110，ENG-005）

**本节是本层对 ENG-005 的兑现**。ENG-005 §五判给本层的是**两行**：

- `docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md:112`——
  `Budget` 类型与分配关系（含预扣／结算）；
- 同文件 `:113`——Budget Validator、`Decision{blocking}` 的触发与恢复。

同表 `:111` 的「预算视图」判给**当下**（子项目 D），`:114` 的「维护性预算根」判给**第 7 层**。
**探索预算的记账不在 §五**——它的依据是 §334（本设计 §12.5）。

> **订正（2026-10-06，评审查出）**：初稿写「§五 的表把**四件事**判给本层……以及探索预算的记账」。
> **两处错**：(i) §五 与探索预算有关的判给项不在那里（依据是 §334）；
> (ii) 「四件事」这个数是把 §五的两行与它自己的列举混在了一起。
> **原句留在此处。**

## 12.1 预算树（ENG-005 §三.1 的「单一预算树」）

ENG-005 取方向 (a)：**节点预算与探索预算都是树上节点的分配**，§110 的「先找合法低成本方案」
由此成为**一次全局判断**。

```rust
/// 树上的一个节点。五个 owner 变体齐了才叫「单一树」。
pub enum BudgetOwner {
    SystemMaintenance,        // §22 onboarding 的归属；**创建方是第 7 层**（§16 第 18 条）
    Intent(IntentId),
    NodeRun { intent: IntentId, node: String },   // NodeId 是不透明 String（§1.2.3 规则 3）
    Exploration { intent: IntentId, of_node: String },
}

/// §333 的五个量纲。**单位与取值域规范未给**（§16 第 19 条）——
/// 本层不解释、不换算、不做跨量纲比较。
///
/// **字段是 `pub` 的**，与 D 的 `BudgetView` 同一处置
/// （`crates/continuum-model-registry/src/budget.rs:36-37` 的判据）：
/// 本类型**没有需要靠私有性守的不变量**（五个 `Option<i64>`、无取值域、无跨维关系），
/// 故 `pub` 不削弱任何保证。**三个角色的保证不在本类型上，在下面三个 newtype 上**
/// ——它们各自私有、无公开构造函数，故「把余量填进上限的位置」仍不可写（见下）。
pub struct Dimensions {
    pub money: Option<i64>,
    pub wall_time: Option<i64>,
    pub token: Option<i64>,
    pub gpu_time: Option<i64>,
    pub network_transfer: Option<i64>,
}
```

**两个角色两个类型**（照 D §6.3 的同一判据：上限与余量不能共用一个类型）：

| 类型 | 角色 | 谁写 |
|---|---|---|
| `Allocation` | 树上节点的**上限**（声明式约束） | 分配方（父节点 / 计划） |
| `Reservation` | 一次在飞的**预扣** | `reserve` |
| `Remaining` | **计算值**：`Allocation − settled − reserved`（逐维） | 无（`remaining()` 现算） |

**`Dimensions` 三处共用一个结构体**是这个设计的取舍，不是疏忽：三个角色的**五个量纲**是同一组，
差别在**操作**（`Allocation` 只被写、`Reservation` 被 reserve/settle、`Remaining` 只被读）。
**类型不共用**（三个 newtype 各包一个 `Dimensions`），故「把余量填进上限的位置」在**类型上不可写**——
这一条正好回答了 D §11 第 12 条对 `ExecutionProfile.cost_budget` 的角色之问（§13.2）。

**`pub` 字段会不会削弱那个 newtype 的保证？——判过，不会，理由两条**（2026-10-06，计划作者查出后补）：

1. **被 `pub` 的是 `Dimensions`，不是三个角色**。三个 newtype（`Allocation` / `Reservation` /
   `Remaining`）**各自私有、无公开构造函数**（只能经分配／`reserve`／`remaining()` 得到），
   故「把一个 `Dimensions` 当 `Allocation` 用」**依然写不出来**。
   `pub` 只允许调用方**自己拼一个 `Dimensions`**，而 `Dimensions` 不是角色，没有可伪造的语义。
2. **`Dimensions` 没有不变量可守**：§333 未给单位、未给取值域、未给量纲间的任何关系
   （§16 第 19 条），故「私有」在它身上**守不住任何东西**——本仓对「公开字段＝无闸门」
   的判据是「**有闸门要守时才有闸门**」（D 的 `BudgetView` 同判）。

**故 §12.6.1 的解构是两步**（这一条在这里写死，免得两处不能同真）：

```rust
let Remaining(dims) = r;                       // newtype：无公开构造函数，此处由 remaining() 得来
let Dimensions { money, wall_time, token, gpu_time, network_transfer } = dims;  // pub 字段
```

> **订正（2026-10-06，计划作者查出）**：本节先前只写「三个 newtype 各包一个 `Dimensions`」，
> 而 §12.6.1 写的是**一步按名解构** `let Remaining { money, … }`——
> **两步解构与一步解构不能同真**（前者要求 `Remaining` 是元组 newtype，后者要求它自己有五个字段）。
> 已二选一写死为**两步解构**，并补了上面那条 `pub` 之判。原句留在此处。

## 12.2 预扣—结算与父子分配（ENG-005 §三.1）

**逐维的不变量**（每一条都有照片）：

```
available(n, d)  =  allocation(n, d) − settled(n, d) − reserved(n, d)
I1: available(n, d) ≥ 0                     （reserve 不满足则拒，fail-closed）
I2: Σ_{c ∈ children(n)} allocation(c, d) ≤ available(n, d)
I3: settled(n, d) + reserved(n, d) ≤ allocation(n, d)
```

**I1 是预扣的判定点**：`reserve(owner, dims) -> Result<Reservation, BudgetError::Insufficient { dimension }>`
返回**具体哪一维不足**（不是笼统的 `Exceeds`）。ENG-005 的「没有预留，§110 这一步就没有判据」
在这一行上成立。

**I2 是 ENG-005「兄弟节点的活跃预留之和 ≤ 父的剩余」的形态**。原文说的是「活跃预留之和」，
本设计判的是**兄弟的分配额之和 ≤ 父的可用额**——即**更强的一条**（分配额 ≥ 该子树的任何预留之和，
由 I3 保证）。**这是本设计与 ENG-005 原句的一处措辞差别，据实记在此**：
**原句未述「分配是上限还是转移」**（全文没有「上限」「转移」二词，是**未述**，不是「一处两读」），
本设计取**「分配是上限、不转移」**
（否则 I2 恒真、那条约束不成立，前一句「否则树不成其为树，额度会被嵌套凭空放大」
要防的形态也就防不住）。**收件人：协调者**（若判「分配即转移」，I2 换成
`Σ_{执行中的子节点} reserved(child, d) ≤ allocation(n, d) − settled(n, d)`，其余不变）。

**结算**：`settle(reservation, actual: Dimensions) -> SettleOutcome`，
`SettleOutcome ∈ { Within { refunded: Dimensions }, Over { dimension, by: i64 } }`。
`Over` 不写第二行账——**运行期实际超支的裁决不在本层**：ENG-005 §三.2 把它判给
`FailureClass::Constraint` ＋ 该节点的 `EscalationPolicy::Decision`，那两项都是执行层的词汇
（`crates/continuum-graph/src/failure.rs:8`、`:32`）。
**本层只回答「超了多少、哪一维」**，由调用方翻译成失败的类别与升级策略。

**账本落库**：`budget_ledger(owner, dimension, kind, amount, at)`，`kind ∈ {allocate, reserve, release, settle}`，
**只追加、不 UPDATE**——余额是派生值。这样恢复时可重放，且「半写的副作用」在账本上表现为
「有一条 `reserve` 而没有对应的 `settle` 或 `release`」，是可检出的。

## 12.3 无读数即按预扣结算（一条贯穿两处的规则）

`settle` 需要一次执行的**实际用量**。本仓今天**只有一个用量读数**：
`continuum_core::model::Usage { input_tokens, output_tokens }`
（`crates/continuum-core/src/model.rs:51`）——它**只覆盖 §333 五维中的 `token` 一维**，
且形状与 §333 的 `token` 不同（拆成输入/输出）。其余四维（`money` / `wall_time` /
`gpu_time` / `network_transfer`）**没有任何产生方**：实测全仓命中只有两处，**两处都不是读数**——
`crates/continuum-model-registry/src/budget.rs:38-43` 是 `BudgetView` 的**类型字段**，
`crates/continuum-model-registry/tests/budget.rs:46-62` 是它的**测试夹具**。
流式调用连 `Usage` 都没有（`StreamChunk` 只有 `delta` / `done`，同文件 `:64`；
G 的设计 §14 第 8 条已报此条）。

> **措辞订正（2026-10-06）**：初稿写「其余四维**全仓零读数**」。**「零读数」成立，
> 但「全仓」这个作用域太宽**——四维在仓里有命中（上面两处），只是都不是读数。
> 本仓已两次栽在「断言的作用域与事实不同宽」，故改写成「没有任何产生方 ＋ 逐条列出命中处」。
> 原句的来历留在此处。

**本设计的规则（一条，两处应用）**：

> **一次执行如果没有实际读数，就按预扣额结算；不退余量。**

两处应用：

1. **流式调用**（G §14 第 8 条）：按预扣结算。
2. **悬空的预留**（进程在预留与结算之间死掉）：**恢复钩子按预扣结算**，不释放。
   钩子挂在 `RecoveryPhase::MarkLostExecutions`（§13.1 有挂该态的两条判据）。

**理由一致**：无读数时，「实际 ≤ 预扣」没有依据；**按预扣结算永不低记**（fail-closed 在预算上），
而释放余量是 fail-open（可能永久漏记一次真实消耗）。
**代价写明**：连续多次无读数的执行会把额度用光，而实际可能没花那么多——
这是一个**已知的保守偏差**，不是缺陷。
**照片三条**：(a) 流式路径结算后 `settled == reserved` 且 `reserved == 0`；
(b) 恢复路径同；(c) **两侧**——有读数时不按预扣（`settled == actual`，余量退回）。

## 12.4 Budget Validator（§110）

§110（`docs/spec/02-positioning.md:1474-1493`）：
`Current Plan → Budget Validator →（预计超预算）→ 先寻找合法低成本方案 → 找不到 → Decision Required`。

**本组件的判定只覆盖中间那一格**：

```rust
pub enum BudgetVerdict {
    Within,
    Exceeds { dimension: DimensionKind, by: i64 },
}
pub fn check(estimate: &Dimensions, owner: BudgetOwner, ...) -> Result<BudgetVerdict, BudgetError>
```

**`Err` 的判据（2026-10-06 补，计划作者查出本节原先没给）**：`BudgetError` 只收**一种**——

```rust
pub enum BudgetError {
    /// 树上没有该 owner 的分配记录（`budget_node` 里查不到它）。
    UnknownOwner { owner: BudgetOwner },
}
```

三条写清楚：

1. **超支不是 `Err`**，是 `BudgetVerdict::Exceeds`——§110 的流程要把「预计超支」当**一个可处置的结论**
   往下走（找替代方案 → 找不到则 Decision），做成 `Err` 会让调用方把它当失败而中止。
2. **量纲的单位／取值域不是 `Err`**：§333 未给单位（§16 第 19 条），本层不解释，
   故「值看起来不合理」在本层**没有判据**，不做校验。
3. **`UnknownOwner` 是 fail-closed 的**：查不到分配记录时**不给 `Within`**
   （给 `Within` 等于对一棵不存在的子树放行）。这条与 §12.2 的 I1 同侧。

**「先寻找合法低成本方案」不在本层**：那是一次**重新规划**（换模型/换节点/分块/延长时限），
是执行层 Planner 的动作（§2.1 的组件表里本层没有 Planner）。
故分工是：`check` 返回 `Exceeds` → **调用方去找替代方案** → 找不到时调本层的
`Decision` 路径（§10.1 第 2 条触发点）。**「找不到」由谁判定、试几次算「找不到」规范未给**
→ §16 第 20 条，收件人：规范维护者。

**`Within` 的判据是逐维比较 `estimate` 与 `remaining`（§12.1 的 I1）**，
**不是**与 `allocation` 比较：`remaining` 已经把兄弟的占用算进去了，
按 `allocation` 比会把 ENG-005 要的「一次全局判断」拆回「各层各自裁决」。

## 12.5 探索预算（§334）

§334：「寻找替代方案也属于预算。默认可自动执行：低成本 / 可逆 / 短时间 / 无外部副作用。
高成本探索必须进入 Decision Gate。」

**ENG-005 §三.1 末行已把阈值钉住**：取 §7 的示例数值作为**规范级默认**
（`docs/spec/01-concepts.md:362-402`）：`wall_time ≤ 60s`、`money_cost = 0`、
`external_side_effect = none`；策略可在此基础上**收紧**。

**四个判据与 §333 的五个量纲对不齐——照实记**：

| §334 的判据 | 落在哪 | 说明 |
|---|---|---|
| `wall_time ≤ 60s` | `Dimensions.wall_time` | 直接可比 |
| `money_cost = 0` | `Dimensions.money` | 直接可比（"= 0" 就是 `≤ 0`） |
| `external_side_effect = none` | **不在 `Dimensions` 里** | 它是**一个种类，不是一个量**：外部副作用的判定属边界层（Effect Journal / `EffectType`）。故它是**必填入参** `has_external_effect: bool`，由调用方以值给出 |
| `网络传输 / 算力在阈值内`（§7 末两行） | `network_transfer` / `gpu_time` | **「阈值」的数值 §7 没给**（原文只有 `<= threshold` 这两个词）→ §16 第 21 条 |
| §334 的「可逆」 | **不在 `Dimensions` 里** | 未给量纲，故是必填入参 `reversible: bool` |

**落点**：

```rust
pub enum ExplorationVerdict {
    AutoAllowed,
    DecisionRequired { dimension: Option<DimensionKind> },   // None = 因「不可逆 / 有外部副作用」而门控
}
```

**逐条照片**：四个默认判据**各一条放行用例与各一条门控用例**（八条）；
外加「策略收紧」一条（把 `wall_time` 阈值调小，同一动作从放行变门控）。
`has_external_effect = true` 那一格 `dimension` 为 `None`——**枚举断言逐项**，
不靠「四格里三格顺带覆盖」。

## 12.6 与 D 的 `BudgetView` 的投影（D §11 第 5 条）

D 的设计 §6.1 已划好：**语义层产出自己的 `Budget`，驱动把它投影成 `BudgetView` 传进 `RoutingRequest`**，
故任何时刻都不出现「语义层造一个资源层的类型」这件事。本设计**照此办，并补齐它缺的那一半**：

> **引用说明（2026-10-06）**：D 的原文在这一句里写的是「`语义层 → 资源层` 这条反向边在任何时刻都不出现」。
> **按 §1.2.1 的约定，`语义层 → 资源层` 读作「资源层依赖语义层」，正是 §9.1 已声明的合法方向**，
> 不是反向边。D 那句话里的箭头用的是相反约定（它与 D 自己 §6.3 的订正段不一致）。
> **本设计照录 D 的结论（「过渡期不出现反向边」），但不照抄它的箭头**——
> 引文原样留在此处，读到它时按本节 §1.2.1 换算。

- **D 侧的 `BudgetView` 已经落地为代码**：`crates/continuum-model-registry/src/budget.rs:34-44`
  （五个 `pub Option<i64>`，无派生、无构造闸门，模块文档 `:20-22` 写着「不是一个 trait」的判据）。
  **D 的设计 `:917-923` 与这份代码一致**，本设计按代码读。
  它的文档还明写了一条与本层同源的区别（`:33-36`）：`None` = 该量纲不构成约束，
  `Some(0)` = 额度为零，**两者不是同一件事**——本层的 `Remaining` 必须守同一条（§12.1 的 `Dimensions`）。
- **本层产出的是 `Remaining`**（§12.1），五个 `Option<i64>`。
- **投影是恒等映射，逐维**：`BudgetView { money: r.money(), wall_time: r.wall_time(), ... }`。
  照片：五个量纲**各一条**用例，断言投影后的值与 `remaining` 逐维相等（含 `None` 那一侧：
  `None` 投影成 `None`，**不是 `Some(0)`**——D 的计划 `## 遗留` 里那一条「`None` ≠ `Some(0)` 谁守」
  在本层这侧由「`Remaining` 的 `None` 只表示『该维不构成约束』」守住，
  两侧的用例合起来才算钉住）。

**对 D §11 第 5 条那句「若复审判定二者应合一，那是把只读投影塞进记账对象，本设计不同意」的答复**：

**D 的理由在本设计上不成立**：`Remaining` **不是记账对象**（记账对象是 `Allocation` + 账本），
它本身就是一条**现算的只读读法**。故「把只读投影塞进记账对象」这个形状**不会发生**——
把 `Remaining` 与 `BudgetView` 合成一个类型，得到的是**一个只读读法**，不是把余量塞进上限。

**而本设计初稿给的另一条理由（方向）是错的**，据实订正：

> **订正（2026-10-06，评审查出）**：初稿写「若复审要合一，代价是接受一条
> `资源层 → 语义层` 的边（与 §9.1 相反），本设计不同意」。**这个方向判断反了**：
> 合并后的类型**住在语义层**时，资源层依赖它，那条边是 `语义层 → 资源层`——
> **正是 §9.1 已画的那一条，不是反向边**。
> 反向边只在**类型住在资源层**时出现（那时本层要依赖资源层）。
> 故**合并在方向上没有代价**。原句与错因留在此处。

## 12.6.1 裁定：**取形状乙**（2026-10-06，协调者）

**裁定：两个类型 ＋ 一道恒等转换，零跨层边。**
**`Remaining` 在 `continuum-budget`**（记账方定义并产出；**crate 归属见下**），`BudgetView` 留在
`continuum-model-registry`（资源层的排序输入面），**转换由驱动做**。
**D 与本层都不动**；§2.2 那条条件性跨层边 `model-registry ← semantics` **不登记**。

**裁定全文的出处**：`docs/superpowers/2026-10-06-p4-decisions.md`
（含日期／背景／甲乙两形状与代价／裁定与三条判据／两条断言／双向翻转条件）。
**本节是转述，以那个文件为准。**

> **订正（2026-10-06，计划作者查出）**：本节初稿写「**`Remaining` 在 `continuum-semantics`**」，
> **与 §2.1（`:204`）、§2.2 的边表、§12.1 的迁移表（`:308`）三处相抵**——
> 那三处都把它归 **`continuum-budget`**（记账对象住在预算层）。**正解是 `continuum-budget`**，
> 已改。**错因**：裁定文件 `p4-decisions.md` 的初版也抄了这一句（协调者已订正，`0e2e73b`），
> 本设计跟着抄了它——**这是「以另一份文档为源而不回核本层自己的划分」**。原句留在此处。

> **补记（2026-10-06）**：本节初稿写「**本节当前没有仓库里的原始记录，这是一处已知缺口**」，
> 并把「落成那个文件」列给协调者。**那一句曾经成立**——裁定最初只经消息下达，
> 而「仓库里查不到」与「没发生过」在文档层面不可区分（这条判据本身是对的，
> 且同一份设计刚在 §5.4 真栽过「把没做过的事写成做过」）。
> **现该文件已落地**（`docs/superpowers/2026-10-06-p4-decisions.md`），**缺口已闭**，
> 故本节改为引它当出处。原句留在此处。

**协调者的三条判据（照录）**：

1. **两条路都可逆，选现在更便宜的那条**：形状甲的代价里含「**删掉 D 已落地并过了评审的
   `BudgetView`**」，连同 D 的设计 §6.1、计划里那个类型／构造／五个用例、`RoutingRequest`
   的字段类型一起改；而 **D 正跑到 Task 10，`BudgetView` 是上一个 task 刚交付的**。
   为一个**今天看不见的**收益去重开那个 task，不划算。
2. **对齐所有权取向**：`BudgetView` 是**资源层自己的排序输入面**，而本项目一贯
   「**谁用谁定义它的输入面**」（D 的 `RankedExecutionCandidates` 就是这么来的）。
   甲会把这个面挪到语义层。
3. **乙的风险是可钉的**：那道转换是恒等的、今天没有独立判据，**但一条「两侧字段集逐项相同」
   的断言就把「漂移」变成「漂移即红」**——**有断言的恒等转换与没有断言的恒等转换是两回事**，
   故乙的那条代价由这条断言抵掉。

### 这条裁定要的两条断言（**两处都要有照片**）

```rust
// 断言一：两侧字段集逐项相同。
#[test] fn remaining_and_budget_view_have_the_same_five_dimensions() { … }
// 断言二：None ≠ Some(0) 在两侧一致。
#[test] fn none_is_not_some_zero_on_both_sides() { … }
```

1. **`remaining_and_budget_view_have_the_same_five_dimensions`**——
   **照片**：两侧字段集**逐项相等**（不是「数量相等」：数量相等会让「`money` 换成 `cost`」漂过去）。
   **任一侧加一个字段、或改一个维度名即红。**
   这条断言是**形状乙的全部防线**：没有它，那道恒等转换的恒等性**无人守**。

   **实现机制写死为「穷尽解构／结构体字面量」，不用 serde**（2026-10-06 定）：

   ```rust
   // 两行解构都**不得带 `..`**（见下「谁保证」）。两个方向都写。
   let BudgetView { money, wall_time, token, gpu_time, network_transfer } = project(&r);
   let Remaining  { money, wall_time, token, gpu_time, network_transfer } = unproject(&v);
   // 逐个绑定比，不比较整个结构体——故**不需要 `PartialEq`**（见下）。
   assert_eq!(money, r.money());          // 五维各一行，逐维断言（运行期的照片）
   assert_eq!(wall_time, r.wall_time());
   assert_eq!(token, r.token());
   assert_eq!(gpu_time, r.gpu_time());
   assert_eq!(network_transfer, r.network_transfer());
   ```

   **「谁保证」逐条写清**（评审查出后补——这句话本身是一句断言，不能由「编译器」三个字背书）：

   | 保证 | 由谁 | 强度 |
   |---|---|---|
   | 全绑 ＋ `..` | **编译器**（需 crate 级 `#![deny(clippy::rest_pat_in_fully_bound_structs)]`） | 编译不过 |
   | **部分绑 ＋ `..`**（删掉一个绑定再加 `..`） | **不由编译器保证**——该 lint 只管「全绑了还写 `..`」 | **仅由站点注释 ＋ 复审守** |
   | 任一侧加字段（在两行都不带 `..` 的前提下） | **编译器**：模式不再穷尽 | 编译不过 |

   **故「由编译器保证」这句只覆盖上表第 1、3 行；第 2 行是**约定**，不是结构。**
   落地时把这张表连同「那两行不得带 `..`」写进代码注释；**若将来要连第 2 行一起钉**，
   缺的是一条**自定义检查**（例如在测试里读那两行源码、断言不含 `..`）——
   **本设计不建它**（今天没有产生方，且它会引入一处读源码的测试）；**收件人：实现者**（见 §16）。

   **断言的落点**：它同时要 `continuum_semantics::Remaining` 与
   `continuum_model_registry::BudgetView`，**故住在驱动**（`continuum-runtime`，
   它已经依赖 model-registry；再加 `continuum-semantics` 是 §1.2.1 约定的合法方向）。

   选择的两条判据：

   - **两行解构都不得带 `..`**。带 `..` 的模式是**穷尽性豁免**——任一侧加字段它就照过，
     那时这条断言与手抄名单**没有区别**。故「不得带 `..`」比断言本身更要紧。
   - **不选 serde**：那条路要给 `BudgetView` 加 `#[derive(Serialize)]`，
     而它的模块文档（`crates/continuum-model-registry/src/budget.rs:36-37`）明写
     「**没有当场消费方的派生（`Debug` / `PartialEq` 等）在这里一律不加**」。
     **为一条测试去改被断言类型的公开形状，是把断言的成本转嫁给它**；
     而穷尽解构**不需要任何派生**，且把红的时点从**跑测试**提前到**编译**。
   - **逐维比而不比整个结构体**（2026-10-06 改）：`assert_eq!(project(&unproject(&view)), view)`
     需要 `BudgetView: PartialEq`——**而上一句刚说过该类型不加派生**，两处**自相矛盾**。
     改成逐维比对**不需要任何派生**，顺带把五个绑定都用上（否则 `unused_variables` 会报）；
     代价是五行而非一行，且**五维逐行正是本仓「枚举断言逐项有照片」要的形状**。

   > **订正（2026-10-06，修后复核查出）**：本条初稿只写「把两侧字段名**各列一遍**并逐项相等」。
   > **Rust 没有反射**——若照字面理解成手写两份名单，**那条断言恒真**
   > （名单是手抄的，类型改了它不会红），它钉的只是「抄的时候两边一样」。
   > **而这条断言的全部意义在「加字段即红」**，落进本仓「表驱动用例覆盖错路径」同一个坑
   > （手工构造的清单在变异下照过）。原句留在此处。
   > 裁定文件 `docs/superpowers/2026-10-06-p4-decisions.md:37-42` 已把这条记为它的初版漏项，
   > 并写明「具体选哪一个、为什么，落在设计 §12.6.1」——**本节即那一处的答复**。
2. **`none_is_not_some_zero_on_both_sides`**——**照片两条，两侧各一条**：
   本层 `Remaining` 的 `None` 表示「该维不构成约束」；
   `BudgetView` 的 `None` 同义（D 已在 `crates/continuum-model-registry/src/budget.rs:33-36`
   写明）。**两条合起来才算钉住**——只钉本层那侧，D 那侧把 `None` 当 `Some(0)` 仍会绿。
   转换侧另加一条：**`None` 投影成 `None`，不是 `Some(0)`**（§12.6 的五个量纲各一条里含这一侧）。

### 翻转条件（**这条裁定有失效条件，不是「它永远对」**）

- **若有一天断言一红了**（两侧真的分叉）：**那一刻就是「两个概念确实不同」的证据**——
  形状乙坐实，那道转换不再是恒等的，必须按真实映射改写，并**重新判定是否要那道转换**。
- **反之，若长期保持恒等、且资源层始终不需要自己的一维**：**形状甲会更简单**，
  那时**可以再改甲**（改起来也便宜——两侧字段始终相同，合并是一次机械搬移）。
  判据是「甲今天看不见的收益」在那一刻**看得见了**（资源层确实要为自己加一维）
  **或**（更可能）「乙的转换始终是恒等的、因而始终是纯负担」。
- **两条方向都可逆**，故本裁定的效力是**有条件的**：它的依据是「当下甲的代价 > 当下甲的收益」，
  而不是「甲错」。

### 这一处**撞上本仓一条既有规则**，据实交代

本仓对「**同一个概念两个类型**」一贯判为 Critical，§13.2 正用这条要求
`ContractIdRef` 与 `ContractId` 合一。**而形状乙的结果恰好是 `Remaining` 与 `BudgetView`
五个字段逐字相同、零跨层边**——**同一形状，只隔三节**。故必须交代它为何不落在同一条判据下：

- **这两者是「同一条已声明的装配关系的两侧」**：驱动**同时依赖两层**
  （它装配迁移、也组装 `RoutingRequest`），转换就发生在它手里；
  而 `ContractIdRef` 与 `ContractId` **之间没有这样一个装配方**——两张 id 各自产生、各自消费，
  沒有任何一方同时持有两者并做转换。
- **而且乙有守、那一对没有**：`remaining_and_budget_view_have_the_same_five_dimensions`
  （上面写死的穷尽解构）**把「漂移」变成「编译不过」**；`ContractIdRef` / `ContractId`
  今日**没有任何断言守它们是否同形**——它们连字段都没有，是两个各自漂移的 newtype。

**判据（本节的口径）**：**「两个同形类型」本身不是罪，
罪在「同形而又无人守它们是否一直同形」。** 乙有守（穷尽解构 ＋ 逐维比对），那一对没有。

**这一段的用途**：免得下次复审把这一处按 Critical 提出来。

**失效条件两支（对称写全）**：本口径的判据是「**罪在无人守**」，故失效条件必须**同时覆盖
『有人守了』**这一支——只问「有没有装配方」会漏掉它。两条**并列**，任一条成立即重判：

- **支一：出现装配方**——有人能指出一个「同时持有 `ContractIdRef` 与 `ContractId`
  并做转换」的装配方；
- **支二：出现守它们的断言**——`ContractIdRef` / `ContractId` 之间补上一条**性质相同的断言**
  （守「两者一直同形」），于是它们也成了「**有人守的同形对**」。

**任一支成立时，本口径就区分不开它们与乙**，届时应**两条一起重判**：
按 §13.2 的同一条判据处置那一对，**且乙也要按同一判据重判**（不能只处理那一对、留着乙）。
**换句话说：这一支不是「别人的事」，它是乙的失效条件之一**——
与「断言一变红」（§12.6.1 的翻转条件）**并列**。

> **订正（2026-10-06，修后复核查出）**：本段初稿的反证口**只有支一**，
> 而口径说的是「罪在**无人守**」——**只问「有没有装配方」没问「有没有人守」，
> 于是支二发生时（那一对补了断言）口径区分不开、反证口却不响**；
> 且它响的后果只是重申 §13.2 已有的要求，**对乙没有杀伤**，故那时它不算乙的失效条件。
> **判据**：**口径说「罪在无人守」，失效条件就必须包括「有人守了」——
> 否则这条口径不是判据，是一个方向。** 原句留在此处。

### 形状甲的代价与前史（照留）

- **甲的形状**：删 `BudgetView`，`.model-registry` 直接收 `continuum_semantics::Remaining`
  （登记 `continuum-model-registry ← continuum-semantics`；§1.2.1 的约定下读作
  「model-registry 依赖 semantics」），驱动不再做投影。
- **甲的代价**（协调者第 1、2 条的展开）：D 的 `BudgetView` 是**已落地并过审的代码**
  （`crates/continuum-model-registry/src/budget.rs:34-44`），删它要连带改 D 的设计 §6.1、
  D 的计划里那个类型／构造／五个用例、`RoutingRequest` 的字段类型；
  且资源层的排序输入面会**归语义层所有**。
- **甲在方向上合法**——这一点是本设计前一版订正出来的：合并后的类型住在语义层时，
  那条边正是 §9.1 已画的 `语义层 → 资源层`，**不是反向边**（订正见上）。
  故甲的否决理由**不是方向**，是上面那两条代价。

**本设计对两种落地都不改动 `Remaining` 的定义**（五个 `Option<i64>`、逐维现算），
故无论裁哪一边，§12.1–§12.5 一字不改。
**裁定与翻转条件记在 §16 第 22 条，收件人：复审者（若给反证则再议）。**

---

# 13. 与已建之物的接缝（逐条）

## 13.1 P0（`continuum-persist` / `continuum-events`）

| 用到的既有面 | 本层怎么用 | 是否要 P0 改 |
|---|---|---|
| `Tx::query` / `execute` / `commit`（`tx.rs:26-47`） | 本层全部读写经它；**写函数不自己 commit** | 不改 |
| `Tx::append_event` / `append_audit`（`tx.rs:49`、`tx.rs:79`） | §5.4 的六个写入点 | 不改 |
| `Migration` / `Db::open_with` / `migrate`（`db.rs:14`、`:72`、`:96`） | 100 / 110 / 120 三个迁移；由 `continuum-runtime` 的 `runtime_migrations()` 追加 | 加三行注册（由用它的 task 登记） |
| `RecoveryHook`（`crates/continuum-persist/src/recovery.rs:39`）与 `RecoveryPhase`（同文件 `:9-15`） | **本层注册一个钩子**：起库时把悬空的 `reserve` 按预扣结算（§12.3 第 2 处） | 不改；注册点在 `crates/continuum-runtime/src/recover_cmd.rs:38-41`（`RecoveryRegistry::new()` ＋ 两次 `register`），**不是 `main.rs`** |

**本层的钩子挂在 `RecoveryPhase::MarkLostExecutions` 这一态，不新增 phase。**
四步判据（**三个备选逐条排除，不是只挑一个**）：

1. **`RecoveryPhase` 的态集不得扩充**：`crates/continuum-persist/src/recovery.rs:18` 的注释写着
   「**顺序即 §319 的执行顺序，不得改动**」（它在 `impl RecoveryPhase` 内、`ALL` 之上），五态是 §319（`docs/spec/05-normative.md:2242-2258`）的转录。
   本层**没有** §319 的判据去加第六态。
2. **不是 `ReconcileRunningNodes`**：该阶段已有的钩子是 `MarkRunningNodesLost`
   （`crates/continuum-runtime/src/recovery.rs:27-40`，`fn phase` 返回 `ReconcileRunningNodes`），
   它**正是产出「哪些运行已 LOST」这个事实的那一步**；本层的结算要**消费**那个事实。
   两者挂在**同一阶段**会让正确性依赖**阶段内的注册顺序**，而那不是一份被文档化的契约。
   挂到**下一阶段**（`MarkLostExecutions`）就把这个依赖消掉：那时 LOST 已经落了库。
3. **不是 `ReconcileIncompleteEffects`**（评审判建议挂此态，**本设计不采纳，理由如下**）：
   该阶段的钩子 `MarkExecutingAsUnknown`（`crates/continuum-effect/src/recovery.rs:47-49`）
   处理的是**效应**——把停在 `EXECUTING` 的外部效应标为 `UNKNOWN`（同文件 `:1-9`）。
   而悬空的**预算预留**不是一次效应：§12.3 的触发是「**这次执行没有读数**」，
   与「外部做没做成不知道」是两件事。把预算结算挂进效应阶段，会让一处「效应语义」的改动
   波及预算记账（反之亦然）。
4. **不是 `ResumeEligibleTasks`**：那时额度已被后续任务读走，结算太晚。

**实测（比只引 §319 更硬的一条，评审者补）**：今天注册的钩子只有两个
（`recover_cmd.rs:38-40`），分别落在 `ReconcileRunningNodes` 与 `ReconcileIncompleteEffects`。
而 `MarkRunningNodesLost` 的**钩子文档自称覆盖两个阶段**——
`crates/continuum-runtime/src/recovery.rs:8-9` 写着「§319 的 `reconcile running nodes`
**与** `mark lost executions`」——**却只注册在 `ReconcileRunningNodes` 上**
（同文件 `:27-29` 的 `fn phase`）。
**故 `MarkLostExecutions` 是「§319 点了名、却没有任何钩子注册」的那一态**，
本层的预算钩子挂上去正是补上它。

**本层的钩子不重复 `MarkRunningNodesLost` 已经做掉的那一半**：
那一步做的是**把节点状态标成 LOST**（`continuum_graph::mark_running_nodes_lost`，
同文件 `:33`）；本层做的是**读那个已经成立的事实、把悬空的 `reserve` 按预扣结算**（§12.3）。
两者**一个写状态、一个记账**，不重不漏——**挂到下一阶段正是为了让后者读得到前者**（第 2 条）。

> **订正（2026-10-06，评审查出）**：本行与随后的段落初稿有三处事实错：
> (i) 说 `RecoveryHook` 定义在 `continuum-runtime` 的同名文件——**它在 `continuum-persist`**；
> (ii) 说注册在 `main.rs`——**今天唯一注册点是 `recover_cmd.rs:38-41`**
> （`main.rs` 里零命中；该函数是从 `main.rs` 迁过来的，这一句写在 `recover_cmd.rs:44-46` 的注释里，
> 我把它读成了「今天还在 main.rs」）；
> (iii) 说「加一个 `phase`（照 `RecoveryPhase` 的既有取值）」——**自相矛盾**：
> 既有取值是五态封闭集，「加一个」就不是「照既有取值」；
> 而 `:24` 的「不得改动」明写它不能扩。
> (iv) 初稿把「定义」「注册」两句话的出处都指到了 `crates/continuum-runtime/src/recovery.rs:39`——
> 那一行是 `impl RecoveryHook for MarkRunningNodesLost` 的**末行**，不是 trait 的定义处。
> **原句与四处错因留在此处。**

**这条由本层的实现 task 做，不推给别处**（改的是 `recover_cmd.rs` 的注册块，
不是 `main.rs`；**五态一个不动**）。

## 13.2 P1（`continuum-graph`）——**本设计要在这里改两处**

| 处 | 现状（实读） | 本设计的请求 | 为什么现在提 |
|---|---|---|---|
| `adfir_graph` 缺 `contract_version` | `crates/continuum-graph/src/persist.rs:17-23` 的 `adfir_graph` 只有 `contract_id TEXT NOT NULL`（`:20`）；`AdfirGraph.contract_id: ContractIdRef`（`crates/continuum-graph/src/graph.rs:49`）也只带 id | **新增一个迁移 21**：`ALTER TABLE adfir_graph ADD COLUMN contract_version INTEGER NOT NULL DEFAULT 0`。三条落地要求见下 | §226 的 MUST 落在执行层，而这条信息**今天在图上不可表达**。图上补一列是一次迁移的成本；等执行器落地后再补，就要同时改执行器、恢复路径与既有行 |

| `ContractIdRef` 与 `ContractId` 是同概念两类型 | `crates/continuum-graph/src/ids.rs:41` 的 `ContractIdRef` 是一个 `String` newtype | **裁定唯一的 `ContractId`**：本层定义真类型，`ContractIdRef` 改为复用它（那需要登记 `语义层 → 执行层` 这条**合法**方向的边，§1.2.1 的约定：执行层依赖本层），或明写 `ContractIdRef` 是**不透明字符串引用**、其权威在本层 | 本仓对「同一个概念两个类型」一贯判为 Critical（`ToolId` / `ModelId` 各有前例）。**生产点是三处**（实读）：`AdfirGraph::new`（`crates/continuum-graph/src/graph.rs:53`）、**`load_graph` 的读回路径**（`crates/continuum-graph/src/persist.rs:233`），以及各测试 |

**迁移 21 的三条落地要求**（评审查出，逐条都是会直接报错或静默失效的坑）：

1. **必须带 `DEFAULT`**。SQLite 在**非空表**上执行 `ALTER TABLE … ADD COLUMN … NOT NULL`
   **不带默认值时直接报错**（"Cannot add a NOT NULL column with default value NULL"）。
   `adfir_graph` 在生产库里非空，故 `NOT NULL` 与 `DEFAULT` 必须同时给。
   （替代路线：先加**可空**列、回填后再约束——SQLite 不支持后加 `NOT NULL` 约束以外的改列，
   故这条替代要重建表，成本高得多。**取前者**。）
2. **不得改迁移 20 的 SQL**。`migrate()` 按 `schema_migrations` 里已记录的 `version` 跳过
   （`crates/continuum-persist/src/db.rs:96-113`），故**改 20 的 SQL 对已建库完全无效**——
   那是一处「改了但没生效」的静默失效。**加列只能走新号 21**。
3. **`DEFAULT 0` 表示「图建立时 Contract 版本未知」**，且**读取侧必须把 0 判为「未绑定版本」**，
   **不得当成真实的 v0**（`load_graph` 读回时 `0` 要能与任何真版本区分开）。取 `0` 而不是 `1`
   的理由：`1` 会被读成一个**看似有意义的**真版本（迁移之前的行并没有按 v1 建过任何东西），
   而 `0` 在 `TaskContract.version` 的取值域里不是一个合法版本，故可作哨兵。
   这个值**没有规范依据**（§226 未规定既有图该按哪一版读），故它**必须与那一列同批写进订正注记**。
   **这一条记在 §16 第 24 条，收件人：执行层 ＋ 协调者**。

**「早裁便宜」这句话在第二条上要收窄**（评审查出）：`ContractIdRef` 的生产点里有
**`load_graph` 的读回路径**（`crates/continuum-graph/src/persist.rs:233`），
故「今天没有既有库受影响」不成立——**已有库里的行会被读回**。
这不改变「现在裁比以后裁便宜」的结论（今天只有一个读回点，将来执行器落地后还会多几处），
但**「生产方只有一个（测试）」那句已经删掉**，别再拿它当论据。

**不改的**：`Node.constraints` / `Node.execution_policy` 的类型不动（§11.2 按原样读）；
`propagate_invalidation` 的签名不动（§8.2 由执行层调）。

## 13.3 P2（`continuum-policy`）——**一处文档与代码相抵**

P2 的设计 §8.2（`docs/superpowers/specs/2026-10-02-p2-boundary-layer-design.md:534-536`）列的
六级优先级的第 2 级是：

```
2  Explicit Current Task Contract   由语义层（P4）在运行期传入
```

而实现里占第 2 级的是 `PolicyContext.explicit_current: Option<ExplicitApproval>`
（`crates/continuum-policy/src/context.rs:25-31`）——一个 `--approve` 的是否给出标志。

**两者不是同一件事**：一个是「当前 Task Contract」，一个是「本次显式批准」。
占位对（`context.rs:9-10` 的注释自述「它占第 2 级 `ExplicitCurrent`，第 1 级之外的规则都可被它越过」），
**内容不是一个概念**。

**相抵的后果「落成后可见」，今天拍不出来**：按 §8.2 的原文，一条 `Deny` **用户持久策略**（第 3 级）
在运行期应当被第 2 级的**当前 Contract** 越过。而今天「当前 Contract 允许」这个事实
**没有任何承载方**——`PolicyContext` 里没有它的位置，本层也还没有契约可传，
故「当前 Contract 允许 ∧ 用户持久策略 Deny ∧ 无 `--approve`」这一情形**今天就不可构造**，
「会被拒」是一句**反事实推演**。

> **订正（2026-10-06，评审查出）**：本段的初稿把上面那句写成「相抵的后果是**可观察的**……**会被拒**」。
> 本层未建、`PolicyContext` 里也没有该事实的位置，故**它今天没有照片**；
> 按本仓纪律，写不出照片的要把「为什么没有」写出来，而不是把它说成可观察的。
> 原句留在此处。**该情形的照片要等两件事同时到位**：本层落地 ＋ P2 给第 2 级一个承载位置。

**本设计不擅自改 P2 的类型**（那是边界层的面）。**本层承诺的是**：
本层的 `TaskContract` 是那个「当前 Task Contract」的**唯一来源**，
故本层可以在需要时把它的若干字段以**值**交给策略求值的上下文
（与 §1.2.3 的规则一致：以值进入，不登记边）。
**处置与收件人记在 §16 第 23 条**（收件人：P2 的设计 / 协调者）。

## 13.4 P3（`continuum-model-registry` / `continuum-provider`）

| 处 | 本层的处置 |
|---|---|
| D 的 `BudgetView` | §12.6：本层产出 `Remaining`，投影恒等；不合一 |
| D §11 第 12 条：`ExecutionProfile.cost_budget` 的类型收紧 | **本层交付了那个类型**：`cost_budget` 收成 **`Allocation`**（上限，不是余量）——D §6.3 的角色判据（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:986-990`）与本设计一致。**收紧的动作归执行层**（那列在 `crates/continuum-graph/src/persist.rs:60`），不是本层；那一步需要执行层依赖本层（`语义层 → 执行层`，§1.2.1 的约定），是 §9.1 已声明的方向。**这是对 ENG-005 §五的一处改判**（见下）。**收件人：执行层（接线时）＋ 协调者** |

> **这是一处改判，须写明（2026-10-06，评审查出）**：ENG-005 §五（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md:111`）
> 把「`execution_profile.cost_budget` 的类型收紧」判给「**D 落地时**」。
> **本设计把它改判给执行层的接线 task**，判据两条：D 的设计 §11 第 12 条**自陈未做**
> （并写明「解除条件是语义层落地」），而那一列属**执行层**的表（`crates/continuum-graph`，P1）。
> **改判要写在这里，否则 ENG-005 §五 与本文两处对不上**——这正是本项目反复栽的「两处各说一套」。
> **收件人：协调者**（若判「仍由 D 做」，则执行层不必接这一步，而 `cost_budget` 的列在 D 手里改不动）。
| G §14 第 5 条：`usage()` 的语义（累计？会话？） | **本层作为对账方给出答案：不需要那个定义。** 预扣—结算要的是**逐次**读数（一次 `invoke` / 一次 `stream` 的增量），累计是**账本**的职责，不是 `usage()` 的。故 §315 的 `usage()` 按「单次」实现即可，**本层不消费它**（本层消费的是调用方交来的 `Dimensions`）。**这一条把 C §12 第 3 条与 G §14 第 5 条从「定不了」降为「不必定」** |
| G §14 第 8 条：流式无用量读数 | §12.3：按预扣结算。**本层给出的是保守处置，不是要求 G 补读数**——补读数要先有 §315 的承载位置（G §14 第 4 条同一条缝） |
| C §6（`docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md:487-489`）：「预算视图来自语义层，不是 provider 的 `usage()`」 | **确认，并按 §12.6 落实**。附一条实读：`Usage { input_tokens, output_tokens }`（`crates/continuum-core/src/model.rs:51-54`）**只覆盖 `token` 一维**且拆成输入/输出，故它**不可能作唯一的成本输入**（五个量纲只对上一维，且那一维的形状还要合成）——**但 `token` 那一维仍可由它喂**，本设计不否认这一点 |

## 13.5 引擎室之外：一条接口面的请求

§10.3 的冻结清单（`docs/02-工程.md:704-710`）里 `ModelProvider` / `ToolProvider` / `Connector`
三项与本层无关（本层不调用任何 provider）。**本设计对 §10.3 不提请求。**

---

# 14. 测试策略

| 验什么 | 怎么验 |
|---|---|
| 规范化的两条互补结果（§2.4 第 1 条） | `Canonicalized` 的两个变体各一条；含错别字／简称／中英混写／代词四类输入**各一条**；有未解 mention 时断言得 `NeedsResolution`（不是 `Canonical`） |
| 四态**逐项**（§271 §206） | 四态各至少一条；每条断言是**哪一枚**（不是 `is_ok`）；`ResolutionResult` 的四个变体**字段清单逐项**（加/少/改名即红）；`UnknownReason` 两枚**各一条**（`NoCandidate` / `AllCandidatesRejected`——混成一枚即红） |
| 显式约束与「满足」（§4.1.1，**本设计自定**） | `ExplicitConstraint` 三枚**各一条** `satisfies` 用例（含各一条反面）；**合取**一条（`K` 有两枚、候选只满足其一时**不合格**）；`CandidateFacts` 的 `scope` / `kind` 各一条；**`Candidate` 三字段不动**（字段清单逐项，加字段即红） |
| 不强行绑定（§273 §200 §206） | §4.2 的断言 A / B / C / D：A 一条（`Resolved` 的 id ∈ 候选集，用 `candidate_count` 核）；B **逐候选 N 条**（每候选一条 `Err` 形态的 `Unknown{AllCandidatesRejected}`）；C 一条（`Unknown{NoCandidate}`）；**D 两条照片**（(1) 合格者被选中；(2) 多合格者 ⇒ `Ambiguous` **且 `surviving` 恰是 `{c₁,c₂}`、不含 `c₃`**——**第二条才让 `c₃` 承重**）。**A、B、D 是 fail-open 侧，必须有** |
| `ExplicitBinding` 指向候选集外（§4.6） | 两条：**(a)** 实体不在检索集里 ⇒ `Resolved(它)` 且 `candidate_count` 比检索集**多 1**；**(b)** 反例——不在集里**且**违反一条 `Scope` 约束 ⇒ `Unknown{AllCandidatesRejected}`（证明 binding 不豁免约束） |
| 单侧守卫的反面 | 「候选集非空且唯一合格 → `Resolved`」也有一条（只钉拒绑那侧会让解析器永远返回 `Unknown` 照样绿） |
| `retrieval_score` 与 `ResolutionConfidence`（§272 §275） | `ResolutionResult` 字段清单逐项（加分数即红）；`ResolutionConfidence` 无可转 f64 的读者（trybuild）；两者不在同一类型的字段里 |
| `ResolutionRisk` 是必填（§276） | 签名层面（无 `Option`、无 `Default`）；**运行期无照片**（§15 第 2 条） |
| Alias 四作用域与不永久化（§278） | 四值各一条写入 + 表外取值读回得具体 `Err`；`SESSION` 写入后 `user_persistent` 行数 0；**显式提升那条路径断言 `Ok` 且 scope 变**（两侧） |
| Intent 状态机（§223） | 合法对逐条 `Ok`；未列出的对若干条 `Err(Illegal{from,to})`；**自环四条**（两条 `Ok`、两条 `Err`）；`COMPLETED → EVOLVING` 两条（`PRODUCT` / `TASK`） |
| Intent 版本语义（§5.2） | 改 `goal` → version +1；只做状态迁移 → version 不变、`updated_at` 变。两条 |
| Specification 的 `unknowns`（§227） | `Specification` crate 外不可构造（trybuild）；`Unknowns` 两变体各一条；**没有 `unknowns` 的 `SpecDraft` 不能 finalize**（签名 + 一条 trybuild） |
| Requirement 四 class（§224） | 四值各一条；`verification_requirement` 的方法集逐条（无判定方法） |
| Contract 不可原地修改（§226） | 二次写同一 `(id, version)` 撞主键、原行不变（裸 `INSERT`） |
| ContractDiff 四类 + `unjudged`（§8.1） | `DiffClass` 四枚各一条；`class` 升/降各一条；**`class` 相同而 `expression` 变 → 进 `unjudged`，且断言它不出现在 `entries` 里** |
| Plan 审查状态机（§229 §338） | 合法对逐条；`STRATEGIC` 未批不得执行 + `EXECUTION` 可（**两侧**）；「修订 = 新版本 + 旧版本 `SUPERSEDED`」一条 |
| `ReviewFinding` 四 severity（§339） | 四值各一条 |
| §340 的三条事实可观察 | 一次「同模型同上下文」的审查读回 `separate_context = false` 且两个 model ref 相同 |
| Plan Change **十例**（§230，五 MINOR ＋ 五 MATERIAL） | 逐例一条，两条 class 各覆盖 |
| `UserReviewView`（§341） | 字段清单**逐项八条**（多一项即红、少一项即红；**成本与权限各占一条**，合并即红） |
| `Decision`（§331） | `options.len() == consequences.len()` 逐条读回；无「只带选项」的构造路径（trybuild）；`Decision` 公开面恰好一个类型（与 `PolicyDecision` 不混） |
| 恢复链（§332 §107） | 只改指名字段、**其余字段逐项不变**；失败路径断言**四张表无半写行**（不只断言 `Err` 哪一种） |
| Constraint Validator（§225 §6） | §225 的 MAY 七条逐项放行 ＋ MUST NOT 三条逐项拦下（第四条见 `ViolatesRequired`）；`REQUIRED` 违反一条；`PREFERRED`/`FLEXIBLE`/`UNSPECIFIED` 违反三条各不拦下；**三侧**：悬空键 `Err(UnknownConstraintKey)` 一条 ＋ `constraints` 为空不是 `Err` 一条 ＋ **命中 `PREFERRED` 键不是 `Err`、也不拦下一条**（第三条钉住「判据与 `class` 无关」） |
| 两道门分离（§2.2） | §11.5 的三条（两条 `ALLOWED` 条目 + 方法集断言） |
| 预算不变量（§12.2） | I1 / I2 / I3 **各一条**违规断言；`reserve` 返回**具体哪一维**（逐维五条）；`settle` 的 `Over` 断言 `dimension` 与 `by` |
| 无读数按预扣（§12.3） | 三条（流式 / 恢复 / **有读数时不按预扣**） |
| Budget Validator（§110） | `Within` 一条；`Exceeds` **逐维五条**；判据是 `remaining` 不是 `allocation`（构造一个「按 allocation 判会放行、按 remaining 判超支」的用例） |
| 探索预算（§334） | 四个默认判据**各两条**（放行/门控，共八条）；策略收紧一条；`has_external_effect` 那一格 `dimension` 为 `None` |
| 投影到 `BudgetView`（§12.6，裁定 §12.6.1） | 五维各一条（含 `None → None`，**不是 `Some(0)`**） |
| 乙的两条防线（§12.6.1） | `remaining_and_budget_view_have_the_same_five_dimensions`：两侧字段名**各列一遍并逐项相等**（不是数量相等）——任一侧加字段或改维度名即红；`none_is_not_some_zero_on_both_sides`：**两侧各一条**（只钉本层那侧，D 侧把 `None` 当 `Some(0)` 仍会绿） |
| 迁移与表形状 | `migrations.rs` 的集合比对（**须含 P1 的新号 21**）＋ `startup.rs` 的计数；三张新表的列清单**逐列** |
| 事件与审计的六个写入点（§5.4） | **逐项六条**，每条断言写出的 `EventType` / `AuditKind` **是哪一枚**（不是「写了事件」）；每条断言事件与状态迁移**同一事务**（任一步失败 ⇒ **两张表都没有半写的行**，不只断言 `Err` 哪一种） |
| `IntentCompleted` 的既有 MUST（§5.4） | 一条：在链上先写一条可跳过（不可解码／版本未知）的事件，再写 `IntentCompleted`，断言得具体 `Err`（`crates/continuum-persist/src/tx.rs:369-373`）；**反例一条**：无可跳过事件时写 `IntentCompleted` 得 `Ok` —— 两侧都钉 |
| 编码纪律 | `scope` / `status` / `class` / `dimension` 等枚举列小写、多词 `_` 连接；表外取值读回得具体 `Err` |

**本项目既有的六条纪律一并适用**：变异须在**全量 `--no-fail-fast`** 下得出否定结论，
且要确认**红的位置**；注释里的绝对措辞须有对应用例（枚举式断言**逐项**有照片）；
失败路径不但断言哪一种 `Err`，还要断言**没有半写的副作用**；守卫**两侧都钉**；
「表驱动用例也可能覆盖错路径」——凡表驱动的，另加至少一条**直接调用**的用例；
纯数据声明／纯投影的类型**没有运行期照片**，只钉类型与签名（编译期）。

---

# 15. 完成判据（§2.4 六条）与**拍不到的照片**

## 15.1 六条逐条落点

| §2.4 的判据（`docs/02-工程.md:130-135`） | 落点 |
|---|---|
| 含错别字、简称、中英混写、代词的输入被规范化，**或被明确标记为 Unknown / Ambiguous** | §3.2 的 `Canonicalized` 和类型（两支互斥）＋ §14 的四类输入各一条 |
| **不存在正确候选时系统不强行绑定**（§273） | §4.2 的断言 A / B / C **＋ D**（B 逐候选；**D 两条照片**：合格者被选中／多合格者得 `Ambiguous`） |
| 带版本的 Specification，且 `unknowns` 显式保留 | §6.1 的 `Unknowns` 两变体 + `Specification` 无公开构造 + §5.2 的版本规则 |
| 带版本的 Contract，每条 Requirement 带 `class` 与 `verification_requirement` | §7.1 / §7.3；`class` 四值逐项照片，`verification_requirement` 方法集逐条 |
| **ContractDiff 能正确判定哪些已有节点受影响** | **本层只做到一半**：§226 四类的判定在 §8.1（判不了的另立 `unjudged` 字段，**不进差异类**）；「哪些节点受影响」的判定在执行层，**今天无消费者**（§16 第 13 条）。**这一条在 P4 单独落地时无法完整成立**，据实写明 |
| 违反 REQUIRED 的候选计划被拦截并生成 Decision | §11.3 / §11.4 / §10.1；拦截与建 Decision 在同一 crate 的两步（不是两个 crate） |

## 15.2 拍不到的照片（逐条写「为什么没有」）

1. **端到端的 Intent → Contract → Plan → Decision 全链**：本层没有调用方（§2.3），
   也没有 Planner 与执行器。故所有照片都只能是对纯函数与库函数的直接调用，
   **钉不了「驱动真的按这条链走」**。
2. **`ResolutionRisk → RequiredStrength` 的单调性**（§276）：门槛是必填入参、`f` 未给（§4.5），
   故本层**没有任何函数**能拍出「风险越高要求越严」。这一条**不是「没有照片」，是「没有判据」**。
3. **规范化的真实质量**：「克拉的code → Claude Code」是否修得对，
   §197 只给了手段清单、没给召回率或正确率判据；且**真实候选来源零个**（§16 第 5 条）。
   能拍的只有机制：四个来源接口、闭集不变量、四态分支。
4. **§198 的联合解析**：未实现（§3.1），故无照片。
5. **§227 的「检测冲突/未定义行为」**：检测器未建（§6.1），只有类型。
6. **表达式之间的强弱比较**（§8.1 的 `UnjudgedChange`）：本层判不了，
   故「`duration >= 4K` → `>= 2K` 算削弱」这件事**没有照片**，它有的是一条**显式标记**。
7. **§340 的「独立性是否足够」**：三条事实可观察（有照片），但**判定没有判据**，故无照片。
8. **真实用量读数**：除 `token` 一维外，其余四维**没有任何产生方**（§12.3，命中处逐条列出），
   故「结算算得对不对」在 `money` / `wall_time` / `gpu_time` / `network_transfer` 四维上
   **不可观察**；`token` 那一维也需把输入/输出两数合成一个（换算规则规范未给，§16 第 19 条）。
9. **§110 的「先寻找合法低成本方案」**：不在本层（§12.4），
   故「找不到时才 Decision」这个次序**没有照片**。
10. **§334 的收紧策略**：四个阈值可由策略收紧，而**策略的来源（谁配、配在哪）规范未给**，
    故「策略真的收紧了」在本层不可观察，只有「给定阈值时的判定」可拍。

---

# 16. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **§9.2 的「Intent 存储」与 §2.3 的 `Specification ← Intent 存储（经 Canonicalization）`**（§1.3）：
   两处回答的是**不同问题**（源点表 vs 层内依赖边），相抵只在「（经 Canonicalization）」
   这一个短语的读法上出现；本设计采纳**读法甲（类型边）**，另一种读法及其代价写在 §1.3。
   **收件人：规范维护者**（两处须择一改，或把那个短语写清）。
2. **「判定属 P4」只覆盖 §236 的两个字段之一**：`crates/continuum-graph/src/node.rs:17`
   的 `/// 结构保存，判定属 P4。` 只管 `:18` 的 `execution_policy`，
   **`constraints`（`:15`）没有对应注记**（`:19` 的 `/// 结构保存，判定属 P5。` 管 `verification_policy`）。
   §236（`docs/spec/05-normative.md:558-580`）两个字段都只有字段名、无词汇表。
   本设计按 §225 的清单给 `execution_policy` 的判据（§11.2 的 10 条用例）、
   按「约束引用键」给 `constraints` 的读法（§11.2、§11.3）。
   **收件人：执行层（补注记）＋ 规范维护者（§236 的词汇表）**。

   > **订正（2026-10-06，评审查出）**：初稿在 §1.2.3 与本节都把这处的后果写成
   > 「按**今天的**读法……静默不匹配（fail-open）」。**「今天的读法」不存在**——
   > 全仓 `constraints` 只有序列化（`persist.rs:103`）与反序列化（`:253`）两处，**零逻辑读者**。
   > 那个读法是**本设计 §11.2 定的**；且该 fail-open 原先**只写在遗留项里**，
   > **裁决类型上看不出来**——现已落进 §11.3 的 `Err(UnknownConstraintKey { key })`。
3. **`Mention.kind` 的取值域 §196/§197 未给**（§3.1）。本层取不透明串。
   **收件人：规范维护者**。
4. **§198 的联合解析没有落点**（§3.1）：§2.1 的组件表里没有承担联合约束传播的组件，
   依据栏也没有列 §198。本层不发明算法，故同一次输入里的多个 mention **各自独立解析**。
   要补的话，缺的是**一个组件**（不是一段算法）与它在 §2.1 表里的位置。
   **收件人：规范维护者**。
5. **§197 的七种候选来源今天零个**（§4.3）：本层只定接口（`CandidateSource`），
   **不内置任何实现**；`alias registry` 是唯一在本层的来源。
   **收件人：实体注册表的持有方（P3 A/B/C/D）＋《工程》§2.1 的组件表**
   （「候选来源」这个跨层的产生点在表里没有一格）。
6. **§276 的 `f(effect, cost, reversibility, privacy)` 未给**（§4.5）：四维的**序与量纲**都没有，
   故本层把 `RequiredStrength` 做成必填、不提供基线。**缺的是「风险四维之间的序」这一步**。
   **收件人：规范维护者**。
7. **会话边界的定义缺失**（§4.6）：`TURN` / `SESSION` alias 的**过期时点**未定义
   （§274 的 `session` / `conversation` 也未定义），故本层只实现「不自动永久化」。
   **收件人：规范维护者**。
8. **§222 的 `priority` 未给取值域**（§5.1）。本层取不透明表示、不排序。
   **收件人：规范维护者**。
9. **§222 的 `completion_predicate` 只有字段名**（§5.1）：判定属第 8 层（§266 Completion Predicate），
   本层只存。**收件人：跨领域层（P5 或后续）**。
10. **§214 的「检测冲突 / 未定义行为」未给判据**（§6.1）。本层只建类型，不建检测器。
    **缺的是「什么算冲突」这一步。收件人：规范维护者**。
11. **§224 的 `budget` 与 §333 的 `Budget` 是什么关系未给**（§7.1）：
    一个在 Contract 里声明、一个是记账对象。本设计取「前者是声明字段（`Opaque`）」，
    两者的桥**没有**。**缺的是「Contract 的 `budget` 怎么变成树上的一次分配」这一步。
    收件人：规范维护者**。
12. **§226 的 `strengthened` / `weakened` 未给判据**（§8.1）：本设计按 `class` 的四值序判，
    而 `class` 相同时的 `expression` 变化判不了，故改记为**并列字段** `unjudged`（§8.1，
    它**不是** §226 的四类之一，也不进 `entries`）。**缺的是「Requirement 表达式的语义比较」这一步。
    **另一处后果一并记此**：本设计用来判 `strengthened`/`weakened` 的 `class` 四值序
    **是推出来的、不是原文给的**（§8.1 的订正段）；**若此序不成立，则这两类也整个落进 `unjudged`**。
    收件人：规范维护者**。
13. **ContractDiff 今天没有消费者**（§8.2 §15.1 第 5 条）：§226 的「受影响的 ADFIR 节点 MUST 被重新判定」
    落在执行层，而**执行器未建**。故 §2.4 的第 5 条完成判据在本层单独落地时只能成立一半。
    **缺的是「映射 `DiffEntry` 到节点并调 `propagate_invalidation`」那一步的执行方**。
    **收件人：执行器（执行层，未建）＋ 协调者（排进哪一轮）**。
14. **§340 的三条独立性是 SHOULD，判定门槛未给**（§9.3）：本层使三条事实**可观察**，
    不判「够不够」。**收件人：规范维护者**。
15. **§230 的 `affected_scope` 未给取值域、「成本显著扩大」未给阈值**（§9.4）。
    **收件人：规范维护者**。
16. **§331 的 `blocking = false` 无触发条件**（§10.1）：本层只在本层两个触发点写 `true`，
    不发明 `false` 的场景。**收件人：规范维护者**。
17. **`Node.constraints` 的词汇表没有规范来源**（§11.2）：本层按「约束引用键」读
    （**这是本设计定的读法，不是今天已有的**——全仓零逻辑读者）。一个拼错的键按 §11.3
    得 `Err(UnknownConstraintKey { key })`（**拦下，不是静默放行**）。
    **缺的是「`constraints[]` 的元素是什么」这一步。收件人：规范维护者 ＋ 复审者**
    （「悬空键一律拦下」是本设计的 fail-closed 选择，规范没给判据；若判「未知键按通过处理」，
    须在 §11.3 正文写明这是有意的 fail-open 并说明代价）。
18. **维护性预算根（`BudgetOwner::SystemMaintenance`）的创建方是第 7 层**（§12.1）：
    本层提供该 owner 变体与它的入账函数，但**没有任何一方会创建它**
    （ENG-005 §三末行把这一步判给长期循环层，该层未建）。
    **收件人：长期循环层（第 7 层）**。
19. **§333 的五个量纲没有单位、没有取值域**（§12.1）：与 D 的 `Cost` / `Latency` 同一处境
    （D §11 第 5 条）。本层不解释、不换算。**代价写明**：`token` 那一维即使有读数
    （`Usage` 的输入/输出两数），**合成规则也未给**，故结算在 `token` 上也不可算
    （§15.2 第 8 条）。**收件人：规范维护者**。
20. **§110 的「先寻找合法低成本方案」的判定方与次数未给**（§12.4）：本层只判「超没超」，
    搜索与「找不到」的判定归 Planner。**缺的是「试几次 / 什么算找不到」这一步。
    收件人：规范维护者 ＋ 执行层 Planner（未建）**。
21. **§7 的 `network_transfer <= threshold` 与 `compute_cost <= threshold` 里的 threshold 没有数值**
    （§12.5）：ENG-005 只把 §7 的**前三条**数钉住，后两条原文就写作 `threshold`。
    **缺的是这两个数。收件人：规范维护者**。
22. ~~`Remaining` 与 D 的 `BudgetView` 是否应合一~~ —— **已裁（2026-10-06，协调者）：取形状乙**
    （两类型 ＋ 驱动恒等转换，零跨层边；裁定全文与三条判据见 §12.6.1）。
    本设计先前反对合一的唯一理由（「合一会写出反向边」）**是错的、已撤回**（订正见 §12.6）——
    类型住在语义层时那条边正是 §9.1 已画的 `语义层 → 资源层`。
    故否决甲的理由**不是方向**，是「删 D 已落地并过审的 `BudgetView`（它刚在上一个 task 交付，
    D 正跑到 Task 10）」与「资源层的排序输入面会归语义层所有」这两条代价。
    **这条裁定带失效条件**（§12.6.1）：
    - **若 `remaining_and_budget_view_have_the_same_five_dimensions` 变红**（两侧真的分叉），
      那一刻就是「两个概念确实不同」的证据，**乙坐实**，转换须按真实映射改写；
    - **若长期保持恒等、且资源层始终不需要自己的一维**，**甲会更简单**，那时可以再改甲
      （改起来也便宜）。
    **两条路都可逆，故本裁定的效力是有条件的**：依据是「当下甲的代价 > 当下甲的收益」，不是「甲错」。
    **另有两支并列的失效条件**（§12.6.1「失效条件两支」）：
    **支一**「出现一个同时持有 `ContractIdRef` 与 `ContractId` 并做转换的装配方」；
    **支二**「那一对之间补上一条守『两者一直同形』的断言」——
    **任一支成立时本口径同时失效，且乙也要按同一判据重判**（不能只处理那一对）。
    **残留三条收件人**：(a) **复审者**（对上面两条代价、翻转条件或两支失效条件给反证即再议）；
    (b) 支一／支二的提出者（可能是任何一方）；
    (c) **实现者**——见第 34 条（`..` 那一半的守卫未建）。
    **裁定全文的出处已落地**：`docs/superpowers/2026-10-06-p4-decisions.md`
    （本设计 §12.6.1 是转述，以那份为准）。
    **本裁定未牵连出别的暂定值**：`Remaining` 的五个 `Option<i64>` 直接取 §333 的字段名，不是暂定。
    **全篇的暂定值共两处，并列如下（本层自己标的，不是规范给的）**：
    ① §13.2 的 `DEFAULT 0`（哨兵，见第 24 条）；
    ② §8.1 判 `strengthened`/`weakened` 用的 `class` 四值序（由四类含义推出，见第 12 条）。
23. **P2 的 `PolicyContext.explicit_current` 与 §8.2 的第 2 级不是同一件事**（§13.3）：
    文档写「Explicit Current Task Contract 由 P4 运行期传入」，代码里是 `Option<ExplicitApproval>`。
    **本层不擅自改 P2 的类型**；若要落 §8.2 的原文，缺的是
    「第 2 级用一个承载当前 Contract 的字段（或其投影）」这一步，且**改的是 P2 的类型与它的用例**。
    **收件人：P2 的设计 ＋ 协调者**。
24. **`adfir_graph` 缺 `contract_version`，`ContractIdRef` 与 `ContractId` 是同概念两类型**（§13.2）：
    **这两条是本设计对已建之物的请求**（一列迁移 + 一次类型裁定）。
    加列那一条附带一个**无规范依据的暂定值**：
    `ALTER TABLE … ADD COLUMN contract_version INTEGER NOT NULL DEFAULT 0` 里的 **`0`**，
    它声明「图建立时 Contract 版本未知」（哨兵，**不是一个真版本**，读取侧须能与之区分）。
    §226 **没有规定**既有图该按哪一版读——**那个 `0` 是本设计的暂定，不是规范给的**，
    落地时须重新判定（§13.2 第 3 条）。
    **全篇的暂定值共两处，都已标成「本设计的暂定、无规范依据」**：
    这一处的 `DEFAULT 0`，以及 §8.1 判 `strengthened`/`weakened` 用的 `class` 四值序
    （它由四类含义推出，不是原文给的；若此序不成立，两类整个落进 `unjudged`，见第 12 条）。
    **收件人：执行层（P1）＋ 协调者**（裁定 `ContractId` 归谁、以及 `DEFAULT` 取什么）。
25. **`ExecutionProfile.cost_budget` 收成 `Allocation`**（§13.4）：类型本层已给，
    **收紧的动作归执行层**（列在 `crates/continuum-graph/src/persist.rs:60`）。
    **收件人：执行层的接线 task**（不是本层、也不是 D）。
26. **`p3bcdf-followups.md` §八.19（＝B 的设计 §11 第 12 条）「服务与 resource 的对应」：
    本层**接一半、退一半**。**
    - **接的一半**：「连接器服务名」（`GitHub` / `Email` / …）作为**一个 mention** 的解析目标，
      其解析属 §271–§278 的射程——本层的 Reference Resolver 对它的处理与其它实体**同形**
      （候选集以值进来、四态输出、不强行绑定）。
    - **退的一半**：「一条操作该绑哪一枚 `CapabilityKind`」与「服务名与 `resource` 的对应表」
      **不在本层**：§2.1 的十二个组件里没有服务目录这一类组件，
      而 §271–§278 是**解析**的规范、不是**绑定**的规范。
      那两件事的两侧都在 B（§124/§125 的注册期声明）与 P3A（`CapabilityKind` 的闭集）之间，
      本层对 `CapabilityKind` 的引用为零（§2.2 的边表即证据）。
      **退件的判据是「本层没有可挂的触发条件」**——不预先发明一个「将来 P4 会读服务名」的钩子，
      那正是「不预先发明」禁的形状。
    **收件人：规范维护者（§125 的服务清单与 `CapabilityKind` 的 resource 集之间要不要一张对应表）＋
    子项目 B 的实现**（照原文；遇到第二个服务时回报）。
27. **`Secret` / `Connector` / `CapabilityKind` 一侧的四条**（`p3bcdf-followups.md` §三 第 1–4 条）：
    **本层全部不经手**，理由逐条：
    - §三.1（非效应臂的强制点落在哪里）：强制点是**工具／连接器调用路径**上的事（F 与 B），
      本层不持有任何强制点；
    - §三.2（一条操作该绑哪一枚 kind 无判据）：同第 26 条的退件理由；
    - §三.3（`effect_class` 是否与 `EffectType` 两个轴）：两侧都在工具调用路径上，
      本层对 `effect_class` 与 `EffectType` 的引用**均为零**；
    - §三.4（`Trust` 与工具侧 `cost` / `latency` 无收件人）：那是**工具选择**，
      而 §2.1 的组件表里没有「工具选择」这一格（该缺口的判据见 `p3bcdf-followups.md` §七.6）；
      本层不选工具、不读 `ToolProfile`。
    **收件人**（见下）：`§三.3`／`§三.4` **照原文**（执行侧工具调用路径 ＋ 规范维护者／后续子项目 ＋ 规范）；
    **`§三.1` 与 `§三.2` 原文没有收件人**，故这两条是本设计**补的**——
    `§三.1`：规范维护者（§125 的读/草稿臂归属）＋ 执行侧（工具调用路径）；
    `§三.2`：同第 26 条的退件理由，规范维护者 ＋ 子项目 B。
    **本层一条都不认领**——不为凑数把工具侧的词汇揽进语义层，那正是 §1.2.3 那条边规则要防的形状。

    > **订正（2026-10-06，评审查出）**：初稿写「**四条收件人照原文**」。**这是假引用**：
    > `p3bcdf-followups.md` §三 里**只有**第 3 条（`:68-69`）与第 4 条（`:73`）有收件人，
    > 第 1 条（`:63`）与第 2 条（`:64-66`）原文**没有收件人**。原句留在此处。

    **`§三.1` 另需一句边界**（评审查出，初稿只给了「本层不持有强制点」一句，太弱）：
    §11.2 判的是 **Contract 的 Requirement**（计划的约束声明），
    §三.1 问的是**能力/效应侧的强制点**（§125 的读/草稿类操作该由谁强制）——**两者不同轴**；
    本层对 `EffectType` 与能力凭据的引用**为零**（§2.2 的边表即证据）。故无交叠。
28. **`p3bcdf-followups.md` §八 的其余 24 条**：**据实核过，收件人无一条是语义层**
    （分别是协调者、规范维护者、B 的实现的后续轮、`continuum-core`、「无（已闭）」）。
    **§八.19 是唯一提到语义层的一条，已在第 26 条逐条处置；§八.25（§125 的五个操作没有语义相配的
    `CapabilityKind`）的收件人是「B 的实现的后续一轮」，本层不接。**
    留这一条是为了让「核过」这件事可查（照 G 的 §13.2 末行的做法）。
29. **G 的设计 §14 第 5 条（`usage()` 的语义）与本设计第 8 条（流式无读数）的答复**（§13.4）：
    本层的答复是「对账只需逐次增量，故『会话／累计』这个定义**不需要**」，
    流式无读数则**按预扣结算**。两条的**规范面**（§315 的 `usage()` 该不该有累计语义、
    `StreamChunk` 该不该带 usage）仍开，**收件人：规范的持有方（C 的设计）＋ 规范维护者**。
30. **ENG-009 已由本层闭合**（§4.6）：Alias 的持久化形态定为一张带 `scope` 枚举列的表。
    **《工程》§10.2 的那一行仍写着「可推迟」**——**收件人：《工程》文档维护者**（订正为已决，指向本文）。
31. **`Decision` 与 `continuum_policy::Decision` 同名**（§10.1）：本设计的处置是
    「本层用 `Decision`、同时引用两者的模块把策略的那个别名成 `PolicyDecision`」。
    **这是命名约定、不是类型守卫**——没有任何编译期机制阻止后来者写 `use continuum_policy::Decision;`
    与 `use continuum_semantics::Decision;` 并列，故它**没有照片**。
    更强的做法（重命名本层的类型）会偏离 §331 的名字。**收件人：复审者**（要不要更强）。
32. ~~本设计在 §8.1 给 ContractDiff 加了第五类 `ExpressionChanged`~~ —— **已闭（2026-10-06，评审查出）**。
    初稿加的那一项被写成了 §226 四类的**第五个变体**，于是「它不是第五种差异」这句声明与类型自相矛盾。
    已改为**并列字段** `unjudged: Vec<UnjudgedChange>`，`DiffClass` 保持**恰好四枚**（§8.1）。
    **初稿自陈的「三条路都不满意」随之收敛**：三条（改写四类／归并／忽略）**一条都不用走**——
    记录另立字段，既不动 §226 的枚举，也不漏判。附一条照片要求：
    「`unjudged` 的条不出现在 `entries` 里」，这条断言把「记录 vs 差异类」的分界钉死。
    **残留一条收件人：规范维护者**——§226 未给 `strengthened`/`weakened` 的判据，
    本设计用的 `class` 四值序是推出来的（§8.1 的订正段）；若此序不成立，两类也落进 `unjudged`。
33. **`AuditKind::UserApprovals` 在本层落地后有两个生产方、两种载荷形状**（§5.4）：
    `continuum-workspace` 的 Integration Gate 写的是操作名（`gate.rs:224` 的 `"apply_patch"` 等），
    本层写的是决策单与被选项。`append_audit` 的 `payload` 是自由 JSON（`tx.rs:79-84`），
    **同一个 kind 下没有形状约束**，故读 `audit_log` 的一方**不得假定「一个 kind 一种载荷」**。
    本设计**不改 Gate、也不为两种形状新增一个 kind**（新增 kind 会偏离 §313 的八行清单）。
    若要收窄，缺的是「§313 的每个 kind 是否允许多种载荷」这一步——
    **收件人：规范维护者**。
34. **断言一有一半不由编译器保证**（§12.6.1 的「谁保证」表）：
    `remaining_and_budget_view_have_the_same_five_dimensions` 的力量全在
    「那两行解构都不得带 `..`」，而**编译器只保证「全绑 ＋ `..`」那一格**
    （需 crate 级 `#![deny(clippy::rest_pat_in_fully_bound_structs)]`，且该 lint **不管部分绑 ＋ `..`**）；
    **「删掉一个绑定再加 `..`」这一格只由站点注释与复审守**。
    **缺的是哪一步**：一条**自定义检查**（例如在测试里读那两行源码、断言不含 `..`）——
    本设计**不建它**（它要引入一处读源码的测试，且今天没有产生方）。
    **本设计已做的是把「谁保证」逐格写明**（§12.6.1 的表），不再由「编译器」三个字替一个约定背书。
    **收件人：实现者**（落地时决定是否补那条检查；若补，须同时说明它读的是哪个文件的哪两行）。
