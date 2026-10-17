# P6D Runtime Learning 与 Protected Core — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p6-scope-and-split.md` §五 P6D 行的原话，逐字照用）：

> 建 §321／§322 那条「Learning 不能自行取消规则」的边界，并把 P0 已定的 crate 方向在本阶段加固（不新增反向边）

**本块拥有 / 产出的接口**（同行的第二栏，逐字）：新建的 Protected Core／Learning crate（名由本块设计定）；
**`ALLOWED` 表的并集复核**（§四 第 1 条）；`Cargo.toml`。

**规范依据**：§320 §321 §322 §216 §217；ENG-001（`docs/02-工程.md:647`）、ENG-008（`:697`）、
《工程》§6.1（`:367`、`:368`）、§6.4 判据 4（`:405`）；P0 设计 `:56`／`:288`；
《总纲》§6.3（`docs/01-总纲.md:1097-1123`）。

**本文件只写 P6D。** 其余三块（P6A 上下文装配、P6B 信任标签、P6C Memory）各有其设计，本文件不替它们定
任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §10.2 写成一条**具名的对账条目**，不在正文里
就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被依赖（B 依赖 A）」
（判据与订正原文见 `docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

**行号口径**：本文件出现的每一处「文件:行号」都是在本工作树（分支 `p6-plan`，HEAD `03e79c7`）里当场
`grep -n` / `sed -n` 实测的，未照抄任何一份别的文档（含切分文档与 P6A／P6B／P6C 三份设计）的行号。
切分文档的若干行号已随 P5 合入而漂移，§11 一次列出替读者核对过的几处。

---

# 1. 范围

## 1.1 建什么

《工程》§6.1（`docs/02-工程.md:367-368`）的两行归本块，逐行落到本设计的一节：

| §6.1 的组件 | 规范依据 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| Protected Core | §321 §322 | §3、§4 | 全量：两枚 crate、`ProtectedCore` 五字段、五个槽位的上游归属表、ENG-001 的方向选择、封存与只读 |
| Runtime Learning | §320 §216 | §5 | **产物规则的全量**：六类目标、五类观察、记录与提案、提案与核心指纹的绑定；**学习算法与执行器不建**（§9） |
| 既有 crate 依赖方向的加固 | 切分 §五「P6D 的加固面」 | §7 | 全量：三点判据 (a)(b)(c) ＋ 并集复核的可执行形状（一条测试、两张对表） |
| `ALLOWED` 表的并集复核 | 切分 §四 第 1 条 | §7.3、§7.4 | 全量：P6 前 27 行基线（含 P5 六行）＋ P6 五行对表 |
| 两枚入度为零源头之一 | §9.2 `docs/02-工程.md:600` | §3.4 | `continuum-protected` 的 workspace 内依赖为空数组（硬断言） |

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| §322 五组规则的类型（`AuthorityRules` / `ContractRules` 一类） | **本块不得自造**（切分 §八.2 第 5 条）。五组规则分属语义层、资源层、边界层；本块只造它们的**槽位**（§4.2） |
| Authority 的规则集 | 上游不存在（§292 的 Authority Host 属长期层，未建）；`TaskContract.authority_limit` 是 opaque 字段，不是规则集（§4.3） |
| Contract 的规则集 | 上游类型在**语义层**（`continuum-semantics`），引用它要新增 `语义层 (2) → 认知层 (6)` 的边，而 §9.1 的图里没有这条边（§4.3） |
| `Evidence` 的类型、证据充分性判定 | P5a（切分 §四 第 1 条）。本块的学习记录只存 Evidence 的**不透明标识**（§5.3） |
| `MemoryClaim` / `MemorySource` / `MemoryScope` / 记忆检索面 | P6C（切分 §五）。本块零表、不写 claim（§6） |
| `ContextPackage` / `ContextTrust` / 七个信任标签 | P6A／P6B。本块不装配上下文、不给上下文片段打标签（§3.2、§10.2） |
| 策略引擎、能力签发、审计链、效应状态机 | 边界层／资源层／执行层已落地（`continuum-policy` / `continuum-capability` / `continuum-events` / `continuum-effect`）。本块**不重写**它们，也不给它们加写入口（§4.3） |
| §322 的「独立验证」与「用户批准」的实现 | 规范只给三条 MUST，没有给验证者接口与批准值类型；本块**不发明**（§4.6、§10.1 第 2 条） |
| ENG-001 的方向 (b)：签名配置、载入时校验 | 今天没有信任根（§292 Authority Host 未建），且**攻击者是 runtime 自身**——它能载入就能重签。本块只保留摘要重算（§4.5） |
| ENG-001 的方向 (c)：编译期固化 | 五组规则的值不在本 crate，`const` 化就要把规则类型搬进来（§八.2 第 5 条禁止），且每次规则变更都要重编认知层（§4.5） |
| 学习算法：去重、矛盾检测、评分、由记录生成提案 | 规范未给（§216 只给「记录什么、优化什么」）；本块只给类型与边界（§5.5、§9） |
| 执行器、图节点、调度、后台作业 | 第 3 层（P1）与第 7 层（P7）。§216 的「执行任务 → 收集 Evidence」两步不在本块（§9） |
| 模型调用、路由选择 | 第 4 层（P3）。本块不调 Provider、不读模型画像（§3.2） |
| 提案的应用（改 routing policy / method / verify / context selection / scheduling） | 各自的持有方：`continuum-model-registry`、`continuum-method`、`continuum-verify`、`continuum-context`、第 7 层。本块只产出提案（§5.3、§9） |
| §217 的自我改进流程（Runtime Improvement Intent → 隔离工作区 → benchmark → 升级） | 第 7 层＋边界层。本块只落 §217 的**最高原则**在类型上的那一半（§2.3、§5.5、§9；§5.1 只承载可学习面） |
| 持久化 / 迁移 / 事件 / 审计 | 本块**零表、零事件、零审计**（§3.3）。学习记录只在内存 |
| UI、报告、指标面板 | 前端／长期循环；规范未给 |

---

# 2. 规范依据与挂靠

## 2.1 逐条：规范给了什么、落在哪、没给什么

| 依据 | 原文给的 | 本设计的落点 | 规范**没给**、由本设计定的 |
|---|---|---|---|
| §320（`docs/spec/05-normative.md:2260-2270`） | `PIER MAY 根据历史执行更新` 五类：routing policy／planning heuristics／execution method／verification strategy／context selection | §5.1 | 五类的类型、产物形状、更新机制、与 §216 第六类的关系（§2.2） |
| §321（`:2274-2290`） | `Runtime MUST NOT 自动修改` 五类：Authority／Contract／Permission／User approval／Core security；它们属于 `Protected Runtime Core` | §4.4、§5.1 | 执行这条分界线的机制（OPEN-013）；五类与 §322 五组的对应（§4.4） |
| §322（`:2294-2312`） | `ProtectedCore` 五个字段名；更新 MUST：`显式版本化`／`独立验证`／`用户批准` | §4 | 五个字段的元素类型；五组规则各自的归属方；三条 MUST 的实现（§4.6） |
| §216（`docs/spec/04-method.md:1177-1212`） | 记录五类执行结果；优化六类；`执行任务 → 收集 Evidence → 学习任务执行方式 → 改善未来任务` | §5.2、§5.1 | 记录的数据形状；学习算法；提案与应用的分界 |
| §217（`:1214-1276`） | Runtime 自身可成为 Product Drive 对象；七类自优化；五类不能自动取消；最高原则「可以学习怎样更好地使用规则，但不能自行取消规则」 | §2.3、§5.5、§9（§5.1 只承载可学习面） | 自我改进流程的落点（不在本块）；七类与 §216 六类的差（§2.2） |
| 《工程》§6.1（`:367-368`） | 两行组件：Runtime Learning、Protected Core | 表头、§1.1 | 两行各自的机制（本文件） |
| 《工程》§6.2（`:382`） | Protected Core 是本层唯一必须先于其他组件存在的部分；学习需要一条不可被自身修改的边界 | §4.1、§3.1 | 「先于」的可执行形态（本设计落成 crate 边界与空依赖行） |
| 《工程》§6.3（`:393-397`） | `Protected Core ← 无`；`Runtime Learning ← 执行历史（第 3、4 层）+ Protected Core`；单向：Learning 读，不能写 | §3.2、§3.4 | 执行历史怎样到达 Learning（§10.1 第 9 条） |
| 《工程》§6.4 判据 4（`:405`） | `Runtime Learning 的产物无法修改 §321 列出的五类语义` | §5.5、§8 | 「产物」的定义（本设计定义为 `LearningProposal`） |
| 《工程》§10.1 ENG-001（`:647`） | 三个候选方向 a／b／c | §4.5 | 选哪一个（本设计取 a） |
| 《工程》§10.2 ENG-008（`:697`） | Context Compiler 是否纳入 Protected Core，来源 C4／OPEN-016 | §10.2 第 4 条 | 与 P6A 同口径：判定在外、不纳入；**不关闭该未决项** |
| 《工程》§6.5（`:410`、`:412`） | OPEN-013 阻断本层；C4／OPEN-016 影响 Compiler 定位 | §4.5、§10.2 | 本设计的选择要回写到哪（§10.1 第 1 条） |
| 《工程》§9.2（`:600`） | 认知层的入度为零组件：Memory Source Graph、Protected Core | §3.4 | 「外部依赖算不算入度」（§10.1 第 10 条） |
| P0 设计 `:56`／`:288` | crate 边界是 ENG-001 的候选机制；P0 定下的方向在 P6 只加固，不新增反向边 | §7 | 加固的可观察形状（§7.3） |
| 《总纲》§6.3（`docs/01-总纲.md:1097-1123`） | 可学习六类＋不可自动修改五类＋ProtectedCore 五字段＋三条更新 MUST＋最高原则 | §2.2、§5.1 | 同 §320／§322 |

## 2.2 三张可学习表（＋ 总纲的转录）、两张受保护表、一张五组表

**这是本设计最先要解决的一处词汇问题**：规范里与「可学／不可学」有关的**五元、六元、七元表不止一张**，
它们不是同一张表的详略版本。逐张照录（行号为实测）：

**（一）可学习对象——三张表**

| 出处 | 逐字 | 数目 |
|---|---|---|
| §320（`05-normative.md:2265-2269`） | routing policy／planning heuristics／execution method／verification strategy／context selection | **5** |
| §216（`04-method.md:1192-1197`） | Planning Policy／Routing Policy／Execution Method／Verification Strategy／Context Selection／Scheduling Heuristic | **6** |
| §217（`04-method.md:1254-1260`） | Planner policy／Routing policy／Skill library／Execution method／Verification strategy／Context strategy／Scheduling strategy | **7** |
| 《总纲》§6.3（`01-总纲.md:1099`，转录） | routing policy／planning heuristics／execution method／verification strategy／context selection／scheduling strategy | **6** |

**本设计取六类**，理由：①《总纲》§6.3 是本层自己的章，它明写「（§320、§217）」并给出六类；②§216 是
本层的规范正文，它的第六类 `Scheduling Heuristic` 在 §320 里没有——若只取 §320 的五类，§216 的第六类
在本层**无主**；③§217 的第七类 `Skill library` **不取**：它属于 §217 的自我改进流程（对 Runtime 自身的
Product Drive），不是 §216 的 Runtime Learning Loop 的优化对象（§217 的两张表与 §216 的两张表不是同
一组，见下）。

**词形不统一处，本设计只保留一个词**（逐条给取舍）：

| 概念 | 规范里的几种写法 | 本设计取的词（`as_str` 的返回值） |
|---|---|---|
| 规划策略 | `planning heuristics`（§320、总纲）／`Planning Policy`（§216）／`Planner policy`（§217） | `planning heuristics` |
| 路由策略 | `routing policy`（§320）／`Routing Policy`（§216、§217） | `routing policy` |
| 上下文策略 | `context selection`（§320、§216、总纲）／`Context strategy`（§217） | `context selection` |
| 调度策略 | `Scheduling Heuristic`（§216）／`Scheduling strategy`（§217）／`scheduling strategy`（总纲） | `scheduling heuristic` |

**大小写不是编码**：`LearningTarget::as_str` 一律返回**小写原词**（上表「本设计取的词」那一列），不返回
规范里的标题大小写。理由是这些串不落库、不传网络，只用于同一 crate 内的 `as_str`／`parse` 互逆与照片；
另造一套大小写写法会让「同一件事两个词汇表」在字符串层重现。

**（二）受保护对象——两张表，同一组概念**

| §321（`05-normative.md:2279-2284`） | §217（`04-method.md:1266-1270`） |
|---|---|
| Authority semantics | Authority |
| Contract semantics | Task Contract |
| Permission invariants | Permission Boundary |
| User approval requirements | User Review Requirement |
| Core security policy | Security Invariant |

两张表**逐项同义、措辞不同**。本设计以 §321 为准（§6.4 判据 4 逐字引用的是「§321 列出的五类语义」），
把 §217 的五个词登记为同义词，**不另造第三个枚举**。照片见 §8.1 的 R7。

**这张表的左右不对序**：左列按 §321 原文顺序；右列是按左列逐项对齐后的重排，**不是 §217 的原文顺序**。
§217 原文的顺序是 `Task Contract`／`Authority`／`Security Invariant`／`Permission Boundary`／
`User Review Requirement`（`04-method.md:1266-1270`）。故本表是「同义表」，不是「逐字照录」。

**（三）§322 的五组——不是上面任何一张表的同构物**

`ProtectedCore { authority_rules, capability_rules, contract_rules, audit_rules, effect_rules }`。
它与 §321／§217 的两张五元表**不是一一对应**：`User approval requirements` 在 §322 里**没有专属字段**，
`Core security policy` 跨 `effect_rules`／`audit_rules`／边界层的策略层级。对应关系是本设计定的，写在
§4.4，并**据实报出覆盖缺口**（§10.1 第 3 条）。

## 2.3 §6.4 判据 4 的读法

原文（`docs/02-工程.md:405`，逐字）：

```
Runtime Learning 的产物无法修改 §321 列出的五类语义
```

**这句话里的三个词要先定死，否则判据不可验收**：

1. **「产物」**＝`LearningProposal`（§5.3）。`LearningRecord` **不是产物**：它是一枚观察记录，没有
   `target` 字段，在类型上指不到任何语义，也就没有「修改」可言。
2. **「修改」**＝对**权威的** `ProtectedCore` 值或其五组规则持有方的写。**不是**「提出一条与规则冲突的
   建议」——建议可以存在，能不能生效是应用侧（执行器／各策略持有方）的事，不在本块。
3. **「§321 列出的五类语义」**＝§321 的五项，**不是** §322 的五个字段名。两者不是同一张表（§2.2 (三)），
   判据引的是前者。

**本设计对判据的兑现方式是三条独立的结构事实**（缺任一条都不是全称）：

- **类型事实**：`LearningTarget` 的六个臂里没有一个是 §321 的五类；`LearningProposal` 的唯一构造点只收
  `LearningTarget`（§5.3）。故一条学习产物**在类型上无法指向**受保护语义。
- **crate 事实**：`continuum-learning` 的 `ALLOWED` 行只有 `continuum-protected` 一项（§3.2）。它连
  `continuum-policy` / `continuum-semantics` / `continuum-capability` / `continuum-events` /
  `continuum-effect` 的类型都**命名不到**，故没有写它们的入口。**这一条就是 ENG-001 的方向 (a)**（§4.5）。
- **值事实**：`ProtectedCore` 没有 `&mut self` 方法、没有 setter、没有 `Default`；唯一的「写」是
  `RuleCommitment::committed`（造一个新值）与 `ProtectedCore::seal`（造一枚新核心），**都不改变任何既有
  核心**。学习侧拿到的是 `&ProtectedCore`（§5.3），没有 `&mut`。

**这三条各自拦不住什么**（据实写，不写成全覆盖）：类型事实拦不住「学习代码把 `ProtectedSemantic::Authority`
写进一个 `String` 字段」——那只是数据，不是修改；crate 事实拦不住「将来有人给 `continuum-learning` 加一条边」
——那正是 §7 的并集复核要抓的；值事实拦不住「学习代码自己 `seal` 一枚假核心」——但那枚值没有消费者，
也替换不了将来装配方持有的权威核心（本轮不存在），且本块不落库（§3.3），落库形态是 §10.1 第 15 条。

## 2.4 本块发明的、影响规范解释的判定（逐条：代价、边界、落点、收件人）

**这一节是本块的「规范解释」总账。** 规格没给、而本设计不得不定的每一处**影响规范解释**的判定，都在
下表里有一条；**没进这张表的规范解释判定，本块一处也没有定**。表的每一行给四件：代价、边界（这条判定
拦不住什么）、落点、收件人。**凡本节的行都不得只留在代码块或注释里**——代码块只是示意，正文措辞才是
约束。

**API 的纯形状选择不在这张表里**（`RuleVersion::new(0)` 的 `None`、`seal` 的输入类型、`to_hex`、派生、
`ObservationKind` 的 `parse` 等）：它们不改变规范的读法，另列 §3.6。两张表合起来覆盖「本块定了什么」的
**规范解释判定 ＋ 主要接口形状**；**只读这张表会漏掉接口形状，只读 §3.6 会漏掉规范解释**。§3.6 不逐项列
getter、派生、`ALL` 常量与字段可见性——它们直接落在 §4、§5 的代码块里。

| # | 发明的判定 | 规范给到哪一步为止 | 代价 | 边界（拦不住什么） | 落点 | 收件人 |
|---|---|---|---|---|---|---|
| 1 | `ProtectedCore` 五字段的元素类型取 `RuleCommitment`（承诺），不取规则本体 | §322 只给五个字段名，不给类型；§八.2 第 5 条禁止自造规则类型 | 核心不能检查规则语义，只能比对摘要；三组只能承诺词表 | 拦不住「摘要没变而语义被改」——那在规则持有方 | §4.2 | 规范维护者 |
| 2 | §321 五语义 ↔ §322 五组 的映射 | 两张五元表都给了，**对应关系**没给 | **非双射**：`approval` 无专属字段、`security` 跨两组＋边界层 | 拦不住「某个语义在两处被同时改、而五组承诺都没变」 | §4.4 | 规范维护者 |
| 3 | `RuleCommitment::Unavailable` 这一状态 | §322 假设五组都存在；今天两组都没有可引用的规则集上游 | 核心可以合法地「部分为空」 | 拦不住「只剩一组 Committed 也算核心」 | §4.2、§4.3 | 规范维护者 ＋ 协调者 |
| 4 | 核心级指纹 `CoreFingerprint` | §322 只给「显式版本化」，没给核心级标识 | 多一层摘要；核心一变，全部旧提案过期 | 不证明「规则字节没被换」，只证明「承诺变了」 | §4.2、§5.4 | 规范维护者 |
| 5 | `LearningTarget` 取六类（§320 五 ＋ §216 第六） | §320 五类、§216 六类，**两张表** | 把 `scheduling heuristic` 纳入可学习面 | 不覆盖 §217 的 `Skill library` | §5.1 | 规范维护者 |
| 6 | `ObservationKind` 取五类 | §216 只给自然语言列举，没给类型 | 五类可能漏掉将来的观察面 | 不判「一条观察是否真实」 | §5.2 | 规范维护者 |
| 7 | `LearningProposal` 必须携带 core 指纹 | 规范未要求「学习产物与核心版本绑定」 | 核心一变，全部旧提案过期（fail-closed） | 不证明「提案与目标对象对得上」（弱引用，第 10 行） | §5.3、§5.4 | 规范维护者 |
| 8 | 本块建**两个** crate，不是一个 | 切分 §五只写「crate（名由本块设计定）」，**未说死枚数** | 多一行 `ALLOWED`、多一个 workspace 成员 | 拦不住将来有人给 `continuum-learning` 加边——并集复核抓（§7.3） | §3.1 | 协调者 |
| 9 | 并集复核用「P6 前 27 行基线快照」 | 切分只要求「复核」，没给形状 | 基线要随有意的既有行改动维护；它放在 P6D 自己的数据文件里（§7.3.1），不放在共写文件 | 只查表，不查设计——语义面由 §7.4 的对表补 | §7.3 | 协调者 |
| 10 | `subject` / `evidence_refs` 取不透明字符串 | 规范未给学习记录与目标对象／Evidence 的连接 | 弱引用；类型层查不到错配 | 不判「这个 subject 是否存在」，也不判它是否指向受保护语义（应用侧义务见 §10.2 第 12 条） | §5.3 | 规范维护者 ＋ P5a |
| 11 | `committed` 拒绝空字节；`seal` 拒绝五组全空 | 规范没有「空规则」的概念 | 空组必须显式 `Unavailable` | 拦不住「部分为空」（第 3 行） | §4.2 | 规范维护者 |
| 12 | `ObservationKind::as_str` 返回中文原句 | §216 只有中文，没给英文名 | 臂名与字符串分离，多一层命名 | 不覆盖规范将来给出的英文名 | §5.2 | 规范维护者 |

**报出而不是发明**的几处（它们不在本块，但本块报了需求）：§322 的「独立验证」「用户批准」两枚类型
（§4.6、§10.1 第 2 条）、五组规则的规则集类型（§4.3、§10.1 第 4–6 条）、学习记录与 `Evidence` 的强引用
（§10.1 第 7 条）。**报出，是 §八.2 第 5 条与切分 §四 第 7 条在本块的兑现。**

---

# 3. 归属、crate 与依赖边

## 3.1 两个 crate：`continuum-protected` 与 `continuum-learning`

本块新建 **两个** crate：

| crate | 装什么 | 名字的理由 |
|---|---|---|
| `continuum-protected` | §322 的 `ProtectedCore` 五字段、五组的承诺、§321 的五类语义、封存入口 | 逐字取 §322 的标题 `Protected Runtime Core` 的后半；它不叫 `continuum-policy`（边界层已占）、不叫 `continuum-capability`（资源层已占）、不叫 `continuum-core`（P0 已占）——这三个名字都会让读者以为它持有规则本体或通用安全策略 |
| `continuum-learning` | §320／§216 的学习目标、观察类别、记录与提案 | 逐字取《工程》§6.1 与 §320 的标题 `Runtime Learning`；不叫 `continuum-runtime-learning`（与装配方 `continuum-runtime` 只差一个词，会被读成 runtime 的一部分），不叫 `continuum-self-improvement`（§217 的自我改进流程不在本块，见 §1.2） |

**为什么是两个 crate，不是一个**——切分 §五只写「crate（名由本块设计定）」，**没有说死一枚还是两枚**；
本设计读作两枚，这是对未定处的**填补**，不是偏离。理由只有一条：

> ENG-001 的方向 (a) 逐字是「**独立 crate，Learning 模块无写权限**」（`docs/02-工程.md:647`）。

若两半同处一个 crate，「学习模块」对核心的写权限只能靠**模块纪律**（不写 `pub(crate)` 写入口）——那是
一条注释与一次评审，不是编译器能拦的东西；且 `pub(crate)` 本身就是一个合法的写入口。**两个 crate 时，
「无写权限」由 Rust 的可见性直接给出**：`continuum-learning` 看不到 `continuum-protected` 的私有字段，
也调不到任何 `&mut` 方法（§4.2、§8.2）。**「写」指对既有的权威核心与五组规则持有方的修改；`seal` 是构造
一枚新值，不是写**——新核心没有消费者，替换不了将来装配方持有的权威核心（本轮不存在，§4.5 末、§2.3 的边界段）。
切分 §1.2 的禁令是「四块之间不得出现两枚**同义** crate」；这两枚不是同义：一枚是核心，一枚是学习，
§6.3 也把两者写成上下游（`:393-394`）。

**名字占用实测**：`grep -rn "continuum-protected\|continuum_protected\|continuum-learning\|continuum_learning" crates/ Cargo.toml`
在**代码语料**里**零命中**（2026-10-17 实测；**本设计文档自身含这些串，全仓 `grep` 会命中它**，故语料限定
在 `crates/` 与根 `Cargo.toml`）；`grep -rn "ProtectedCore\|LearningProposal\|LearningTarget\|RuleCommitment\|ProtectedSemantic" crates/ Cargo.toml`
**零命中**。两个名字都是自由名。

**与其余三块不同义**：P6A 已定 `continuum-context`、P6B 已定 `continuum-trust`、P6C 已定 `continuum-memory`
（各自设计 §2.1／§3.1）。本块不占 `context`／`trust`／`memory` 三个词，也不占 `policy`／`capability`／
`core` 三个既有词。

## 3.2 依赖边

**依赖边表**（**只登记实际用到的**，切分 §四 第 5 条）：

| crate | 被依赖 | 用到什么（逐项实测） | 边别 |
|---|---|---|---|
| `continuum-protected` | **workspace 内：无** | —— | —— |
| `continuum-learning` | `continuum-protected` | `ProtectedCore`（`fingerprint()`，§5.3 的唯一构造点收 `&ProtectedCore`）、`CoreFingerprint`（`LearningProposal` 的字段类型） | 普通 |
| `continuum-runtime`（**既有行**，新增一项） | `continuum-protected` | 装配照片：`seal` ＋ 五组承诺 ＋ 三组词表的规范字节（`tests/p6d_protected_core.rs`） | **dev 边** |

**`continuum-protected` 的 workspace 内依赖为空数组，是硬断言**：`ALLOWED` 行 `("continuum-protected", &[])`，
任何指向 workspace 内别的 crate 的直接边都会让 `every_crate_depends_only_on_its_allowed_set`
（`crates/continuum-runtime/tests/dependency_direction.rs:478`）变红。同形先例是 P5b 的
`("continuum-method", &[])`（同文件 `:157`）。它**只依赖外部 crate**：`sha2`（摘要）与 `thiserror`
（`ProtectedError` 的两条 `Display`）。外部 crate 不是 §9.1 八层图里的组件，故不破坏「入度为零」
（§3.4、§10.1 第 10 条）。

**`continuum-learning` 只有一条 workspace 边**。**不登记**的边逐条给理由（零使用的边即假边）：

- **不登记 `continuum-memory`**：本块零表、不写 claim（§6）。将来若学习记录落进 P6C 的 `RUNTIME` scope，
  那时才加这条边（§10.1 第 15 条）。
- **不登记 `continuum-graph` / `continuum-artifact` / 任何执行层 crate**：§6.3 的 `Runtime Learning ←
  执行历史（第 3、4 层）` 以**值**由装配方传入；`执行层 (3) → 认知层 (6)` 在 §9.1 的图里**没有**这条边
  （§八.2 第 1 条的同族，§10.1 第 9 条）。
- **不登记 `continuum-capability` / `continuum-events` / `continuum-effect` / `continuum-policy` /
  `continuum-semantics`**：**这一条不是遗漏，是判据 4 的强制面本身**（§2.3 的 crate 事实）。学习侧
  命名不到这五枚 crate 的类型，就写不了 §321 的五类语义。
- **不登记 `continuum-context` / `continuum-method` / `continuum-verify` / `continuum-model-registry`**：
  六类学习目标是**类别**，不是那四枚 crate 的目标对象类型。`subject` 是目标对象在它自己那层的标识，
  以不透明字符串承载（§5.3、§10.1 第 7 条）。加这四条边会把「学习产物」变成「对四个具体类型的写入」，
  正是本块要避免的。
- **不登记 `continuum-trust`**：本块不给上下文片段打标签，也不读标签（§10.2 第 2 条）。

**`continuum-runtime` 那一项为什么是 dev 边**：本轮 `continuum-protected` 的唯一使用点是
`tests/p6d_protected_core.rs`（装配照片），生产代码对它的调用要等执行器（§8.4、§10.1 第 11 条）。
按本仓「边别按谁在用判」的口径，它是 dev 边（同 `continuum-provider` 那一行的 persist、P5c 的
`continuum-graph` 与 `continuum-workspace`）。`cargo tree --edges all` 把 dev 边算进依赖，故与普通边
一样登记。生产装配落位后，**边别升为普通边，`ALLOWED` 数组不变**，只改行内注记。

## 3.3 共写文件、迁移与事件

**共写面与本块自己的新文件**（本块涉及的共写是 workspace `Cargo.toml`、`ALLOWED`、runtime `Cargo.toml`；
切分 §一 1.3 的第三处 `runtime_migrations()` 本块不碰；runtime `Cargo.toml` 是事实上的第四处，§10.2 第 11 条。
标「不共写」的是本块自己的路径）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members`（`:3-31`） | 加两行：`crates/continuum-protected`、`crates/continuum-learning` | 每块只加自己的行；两行的理由见 §3.1 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加两行：`("continuum-protected", &[])`、`("continuum-learning", &["continuum-protected"])`；并在 `continuum-runtime` 那一行（`:391-412`）加一项 `continuum-protected`（dev 边） | 数组按字母序；既有行只有 runtime 这一行增枚（§7.3） |
| 同一文件的**文件末尾** | 加用例 `p6_union_adds_only_its_own_rows`（**只加用例，不加基线常量**；基线在下一行的数据文件里） | **本块对该文件的第二处改动**；放在末尾以缩小与其余三块的合并冲突面。基线不在这个文件里（§7.3.1） |
| `crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`（**新建，P6D 自己的数据文件**） | 27 行基线，逐行等于 `03e79c7` 的 `ALLOWED`；文件头写明「改它是 P6D 的改动」与「P6 合并门，P7 重新基线或退役」 | **不共写**：不在切分 §一 1.3 的三处里，也不在 `dependency_direction.rs` 里；改动要在合并评审的 diff 里单独核对（§7.3.1、§7.3.5） |
| `crates/continuum-runtime/tests/p6d_protected_core.rs`（**新建，P6D 自己的用例**） | 装配照片 R9 | 不在共写表里；用 `continuum-protected` 的 dev 边（下一行） |
| `crates/continuum-runtime/Cargo.toml` 的 `[dev-dependencies]` | 加 `continuum-protected = { path = "../continuum-protected" }` | 装配照片的 dev 边；与上表的边别一致。**这个文件是事实上的第四共写面**（切分 §一 1.3 未列），四块各加自己的一行，见 §10.2 第 11 条 |

**本块不碰** `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:110`）：本块**零表**，
不注册任何迁移。P6 的档 `190`–`199` 由 P6C 写死 `190`（其设计 §3.3）；`191`–`199` 本块不预分（切分 §四 第 2 条）。

**零事件、零审计**：§310 的九类事件（`crates/continuum-events/src/event.rs:35` 的 `EventType::ALL`）与
§313 的八类审计（`crates/continuum-events/src/audit.rs:38` 的 `AuditKind::ALL`）都是封闭集合；本块不加
第十类／第九类。`ProtectedCore` 的封存与学习产物的产生**都不写事件**——它们是装配期/内存事实，不是
运行期的状态迁移。

## 3.4 层间位置与「入度为零」

§9.1（`docs/02-工程.md:561-588`）里与本块有关的三条，按「被依赖者 → 依赖者」读：

- **`资源层 (4) → 认知层 (6)`** 与 **`边界层 (5) → 认知层 (6)`**：本块**不登记**指向这两层的边
  （§3.2）。不登记不是遗漏，是零使用：五组规则的持有方一个都不在本块的依赖里。
- **层内边**：`continuum-protected → continuum-learning`（二者同属第 6 层）。本仓对层内边有同形处置
  ——`continuum-provider → continuum-capability`、`continuum-code → continuum-verify` 都登记并在行内
  标明。本条是单向的：`continuum-protected` 的允许集是空数组，不可能反向。
- **`认知层 (6) → 长期循环 (7)`**：本块不接。学习提案的「应用」若落到调度，属第 7 层（§10.2 第 7 条）。

**「入度为零」的兑现**：§9.2（`:590-601`）把 Protected Core 列为依赖图的源点。本设计把它落成
`continuum-protected` 的**空允许集**——这是比「没有反向边」更强的一条断言：它没有上游可指。
**一处申报**：它依赖 `sha2` / `thiserror` 两枚外部 crate。按 §9.2 的字面读法，这已经是两条上游；本设计
按「§9.1 的八层组件」读，外部 crate 不在那八层里，故层内入度仍为零。**这处读法有先例**：P6C 的
`continuum-memory` 依赖 `continuum-persist`（P0 基础设施，也不在八层里），其 §3.4 做了同形申报。
**本设计不自称解决它**，记入 §10.1 第 10 条。

**「入度为零」的是强制形态，不是规则本体**：五组规则仍在各层（§4.3），本 crate 持有的是它们的
**槽位与承诺**。说「Protected Core 入度为零」指的是这一枚强制组件不依赖规则的持有方，不是指规则本身
凭空存在。这句话要单独写出，否则会被读成「本 crate 已经持有五组规则」。

## 3.5 冻结清单：本块持有的类型

**`continuum-protected`**（八枚）：

```
ProtectedCore          —— §322 的五个字段，字段私有，唯一构造点 seal（§4.1、§4.2）
ProtectedCoreParts     —— seal 的输入，五个字段名与 §322 逐字相同（§4.2）
RuleCommitment         —— 一组规则在某一版本上的承诺：Unavailable 或 Committed{version, digest}（§4.2）
RuleVersion            —— 版本号；0 不合法（§4.2）
RuleDigest             —— 一组规则字节的 SHA-256；字段私有，唯一产生点是 committed（§4.2）
CoreFingerprint        —— 五组承诺的规范编码的 SHA-256；不是第六个字段（§4.2、§5.4）
ProtectedSemantic      —— §321 的五类语义，逐字（§4.4）
ProtectedError         —— NothingCommitted / EmptyRules 两臂（§4.2）
```

**`continuum-learning`**（四枚）：

```
LearningTarget         —— 六类可学习目标（§5.1）
ObservationKind        —— 五类观察（§5.2）
LearningRecord         —— 一条观察记录；不是产物（§5.3）
LearningProposal       —— 一条学习产物；唯一构造点收 &ProtectedCore（§5.3）
```

**派生清单**（照片 R3／R4／R5／R10 与日志需要；**不派生 serde**）：

| 类型 | 派生 |
|---|---|
| `ProtectedCore` | `Debug, Clone, PartialEq, Eq` |
| `ProtectedCoreParts` | `Debug, Clone, PartialEq, Eq` |
| `RuleCommitment` | `Debug, Clone, PartialEq, Eq` |
| `RuleVersion` | `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash` |
| `RuleDigest` | `Debug, Clone, Copy, PartialEq, Eq, Hash` |
| `CoreFingerprint` | `Debug, Clone, Copy, PartialEq, Eq, Hash` |
| `ProtectedSemantic` | `Debug, Clone, Copy, PartialEq, Eq, Hash` |
| `ProtectedError` | `Debug, Clone, PartialEq, Eq`（`thiserror` 另给 `Display` / `Error`） |
| `LearningTarget` | `Debug, Clone, Copy, PartialEq, Eq, Hash` |
| `ObservationKind` | `Debug, Clone, Copy, PartialEq, Eq, Hash` |
| `LearningRecord` | `Debug, Clone, PartialEq, Eq` |
| `LearningProposal` | `Debug, Clone, PartialEq, Eq` |

**为什么不派生 serde**：本块零持久化（§3.3）；多一条编码路径就是多一处会与 `as_str` / `to_hex` 分叉的
地方（同 §5.1、§5.2 对 `EffectType` 的引用）。**`RuleVersion` / `RuleDigest` / `CoreFingerprint` 的构造与
展示接口在 §3.6**（`new` / `get` / `to_hex`），不在这张清单里重复。

**不新增的类型**：不定义 `AuthorityRules` / `CapabilityRules` / `ContractRules` / `AuditRules` /
`EffectRules`（§八.2 第 5 条）；不定义 `Evidence`（P5a）、`MemoryClaim` / `MemoryScope`（P6C）、
`ContextPackage`（P6A）、`ContextTrust` / `Provenance`（P6B）、`Policy` / `Decision`（`continuum-policy`）、
`Capability` / `CapabilityKind`（`continuum-capability`）、`AuditKind`（`continuum-events`）、
`EffectType` / `EffectState`（`continuum-effect`）、`TaskContract` / `Requirement`（`continuum-semantics`）。

**错误类型只有一枚**：`ProtectedError`。`continuum-learning` **没有错误类型**——它的两个构造点都是
不可失败的（`LearningRecord` 是结构体字面量，`LearningProposal::new` 只读核心）。不建第二枚错误类型，
也不包装 `ProtectedError`（判定是同一件事则合并，是两件事则分开；同 P5c §3.2 的判据）。

## 3.6 API 形状（不改变规范解释；与 §2.4 合起来覆盖规范解释判定 ＋ 主要接口形状）

**这一节列的是本块为类型选定的接口形状**，它们不改变规范的读法，故不混进 §2.4 的规范解释总账。
每一项给「取值」与「边界」；照片编号指向 §8.1。**本节不逐项列 getter、派生、`ALL` 常量与字段可见性**：
`ProtectedCore` 的五个访问器、`LearningProposal` 的四个 getter、`RuleCommitment::{version,digest}`、三个
`ALL` 常量、`fingerprint()` 与 `LearningRecord` 的 pub 字段，形状在 §4、§5 的代码块里。

| # | 形状 | 取值 | 边界 / 理由 | 落点 |
|---|---|---|---|---|
| 1 | `RuleVersion` 构造与取值 | `new(u64) -> Option<Self>`（0 → `None`）；`get(&self) -> u64` | 0 不是合法版本；不引入第二枚错误类型 | §4.2、R3b |
| 2 | `RuleDigest` 展示 | `to_hex(&self) -> String`（64 hex） | 只给照片与日志；**没有** `FromStr`（唯一产生点是 `committed`） | §4.2、R1 |
| 3 | `CoreFingerprint` 展示与稳定 | `to_hex(&self) -> String`；**不承诺跨版本稳定** | 只在同一版内比较；编码一改，旧提案全过期（fail-closed） | §4.2、§5.4 |
| 4 | `seal` 的输入 | `ProtectedCoreParts`（五个 pub 字段） | 防五个同型位置参数传反；字段名与 §322 逐字 | §4.1、R4 |
| 5 | `seal` 的错误面 | `Result<_, ProtectedError>`；**不会**返回 `EmptyRules` | 不造第二枚错误类型；调用方不得据 `EmptyRules` 分支（§4.1 末） | §4.1、§4.2 |
| 6 | `RuleCommitment::matches` | `Unavailable` 恒 `false` | 「无承诺」不等于「匹配空字节」 | §4.2、R2 |
| 7 | `ProtectedCore` 的 trait | 无 `Default` / `Deref` / `DerefMut` | 无空核心、无隐式写 | §4.1、C4 |
| 8 | `LearningProposal` 的 target | 单值 `LearningTarget` | 一条提案一件事；原子性／回滚／审批单位有落点 | §5.1、§5.3 |
| 9 | `LearningProposal::rationale` | 单个 `String`（不是 `Vec`） | 规范没给理由列表；多理由由调用方拼接 | §5.3 |
| 10 | `is_current_for` | 比较 `core_fingerprint` 与 `core.fingerprint()`；核心一变全部过期 | fail-closed；不判「提案是否该被应用」 | §5.3、§5.4、R8 |
| 11 | `as_str` 的大小写 | `LearningTarget` 小写原词；`ObservationKind` 中文原句；`ProtectedSemantic` §321 原文 | 三处不同，按各自规范写法（§2.2（二）、§5.2） | §2.2、§4.4、§5.2 |
| 12 | `ObservationKind` 的 `parse` | 有，`as_str` 的严格逆；表外串 `None` | 与 `as_str` 成对（§5.2、R11） | §5.2、R11 |
| 13 | 派生 | 见 §3.5 的表 | 照片需要；不派生 serde | §3.5 |

**`RuleVersion` / `RuleDigest` / `CoreFingerprint` 的最小接口**（不重复 §4.2 的 `RuleCommitment` 块）：

```rust
impl RuleVersion {
    pub fn new(value: u64) -> Option<Self>;
    pub fn get(&self) -> u64;
}
impl RuleDigest {
    pub fn to_hex(&self) -> String;
}
impl CoreFingerprint {
    pub fn to_hex(&self) -> String;
}
```

---

# 4. `ProtectedCore` 的形状（§322 逐字）

## 4.1 五个字段，逐字

§322（`docs/spec/05-normative.md:2297-2303`）原文，逐字：

```
ProtectedCore {
    authority_rules
    capability_rules
    contract_rules
    audit_rules
    effect_rules
}
```

Rust 侧（**字段名、次序、五个——一个字不改**；变的只是元素类型，规范没给）：

```rust
pub struct ProtectedCore {
    authority_rules: RuleCommitment,
    capability_rules: RuleCommitment,
    contract_rules: RuleCommitment,
    audit_rules: RuleCommitment,
    effect_rules: RuleCommitment,
}

/// [`ProtectedCore::seal`] 的输入。五个字段名与 §322 逐字相同。
/// 字段 pub：它是**封存前**的零件，封存后由 `ProtectedCore` 私有持有。
pub struct ProtectedCoreParts {
    pub authority_rules: RuleCommitment,
    pub capability_rules: RuleCommitment,
    pub contract_rules: RuleCommitment,
    pub audit_rules: RuleCommitment,
    pub effect_rules: RuleCommitment,
}
```

**`ProtectedCore` 的字段私有**，五个访问器逐一同名（`authority_rules()` / `capability_rules()` / `contract_rules()` /
`audit_rules()` / `effect_rules()`），返回 `&RuleCommitment`。**没有 `&mut self` 方法，没有 setter，
没有 `Default`，没有 `DerefMut`**。唯一构造点是：

```rust
pub fn seal(parts: ProtectedCoreParts) -> Result<ProtectedCore, ProtectedError>;

#[derive(Debug, thiserror::Error)]
pub enum ProtectedError {
    /// 五组全部 Unavailable：这样的核心不保护任何东西，拒绝封存。
    #[error("五组规则全部缺失：ProtectedCore 不能封存一个空壳")]
    NothingCommitted,
    /// committed 收到空字节：一份「空规则」不是一份规则。
    #[error("规则字节为空：空规则不是规则")]
    EmptyRules,
}
```

**为什么构造参数是 `ProtectedCoreParts` 而不是五个位置参数**：五个参数同型（都是 `RuleCommitment`），
位置传参时把 `authority_rules` 与 `contract_rules` 传反**编译得过、运行期也看不出来**；`ProtectedCoreParts`
用结构体字面量的字段名把这一处钉住（照片 R4）。

**`seal` 不会返回 `EmptyRules`**：它不接收字节，`EmptyRules` 只从 `RuleCommitment::committed` 出。
返回类型仍是 `ProtectedError`，是为了不造第二枚错误类型（§3.5 末）；**调用方不得据 `EmptyRules` 分支**
——那是一条 `seal` 永远进不去的臂（§3.6 第 5 行）。

§322 的更新三 MUST（`:2308-2312`）逐字：

```
显式版本化
独立验证
用户批准
```

**本轮只落「显式版本化」的那一半**：`RuleCommitment::Committed` 带 `RuleVersion`。另两条的落点与缺口
见 §4.6。

## 4.2 为什么元素类型是「承诺」而不是规则

**这是本设计最核心的一处裁定**，依据两条：

1. **§八.2 第 5 条**（切分文档 `:263-266`）：`ProtectedCore` 的五组规则类型**分属多层**
   （`authority_rules`／`contract_rules` 在语义层与边界层、`capability_rules` 在资源层、`audit_rules`／
   `effect_rules` 在边界层）。**P6D 不得为它们另造类型，只能引用各层已冻结的那几枚。**
2. **引用会破坏「入度为零」**：若 `ProtectedCore` 直接装 `TaskContract` 或 `CapabilityKind`，
   `continuum-protected` 就依赖那些 crate：
   - `continuum-semantics` 属**语义层**，§9.1 里**没有** `语义层 (2) → 认知层 (6)` 这条边；引用它会
     让认知层反过来成为语义层的下游，这是一条新的反向边；
   - `continuum-capability`（资源层）与 `continuum-events` / `continuum-effect`（边界层）虽有
     `→ 认知层` 的层间边，但依赖它们会让 `continuum-protected` 不再是 §9.2 的源点。

**故本 crate 只持有「承诺」**：一组规则的**版本 ＋ 内容摘要**。它是「引用」的一种，不是「规则类型」的
另一种写法——它不含任何一条规则的字段，也不解释规则的语义。规则本体仍在各层（§4.3）。

**那为什么还要有这枚 crate**：没有它，五组规则只是散落在各层的一堆类型，「哪五组、哪一版、有没有变」
没有一个可传、可比、可绑定的值；有了它，§322 的五个槽位、§321 的五类语义、以及「学习产物是在哪一版上
学的」才有一枚共同的落点。**它不替规则做判定，它只把「保护了哪五组、保护的是哪一版」变成值。**

```rust
pub struct RuleCommitment {
    state: CommitmentState,   // 私有
}

enum CommitmentState {
    Unavailable,
    Committed { version: RuleVersion, digest: RuleDigest },
}

impl RuleCommitment {
    /// 该组规则的上游归属方今天不存在／未冻结。显式记「无」，不是「空规则」。
    pub fn unavailable() -> Self;

    /// 唯一的提交入口：摘要由本函数从字节算出，调用方给不出一个伪造的摘要。
    /// 空字节是 EmptyRules：一份「空规则」不是一份规则。
    pub fn committed(version: RuleVersion, rules: &[u8]) -> Result<Self, ProtectedError>;

    pub fn version(&self) -> Option<RuleVersion>;
    pub fn digest(&self) -> Option<&RuleDigest>;

    /// 以同一份字节重算摘要并比对；Unavailable 恒为 false。
    pub fn matches(&self, rules: &[u8]) -> bool;
}
```

**`Unavailable` 为什么必须在类型里**：今天五组里有**两组都没有可引用的规则集上游**（`authority_rules`／
`contract_rules`，§4.3）。若用「空摘要」表示，核心看起来是完整的，而实际上有两组什么都没保护——
**失配方向是静默的**。`Unavailable` 让「本轮装配保护不了这一组」成为可断言的事实（照片 R9）。

**`seal` 的失败面只有 `NothingCommitted` 一条**（`EmptyRules` 属 `committed`，不属 `seal`）。它拦的是
「封存一个空壳」；它**拦不住**「五组里只有一组是 Committed」——那是一个合法的部分核心，照片 R9 的装配状态就是
三组 Committed、两组 Unavailable。**这一条要写清，否则会被当成全覆盖的守卫。**

**核心指纹**：

```rust
impl ProtectedCore {
    /// 五组承诺的规范编码的 SHA-256。**不是第六个字段**：§322 只给五个字段，
    /// 故它每次按五个字段现算，不存储。
    pub fn fingerprint(&self) -> CoreFingerprint;
}
```

**规范编码是本设计定的**：固定按 `authority → capability → contract → audit → effect` 的次序，逐组编入
组名、状态、版本与摘要；**定长字段与长度前缀，不靠分隔符猜边界**（组名是已知的五个字面量，版本 8 字节，
状态 1 字节，摘要是 32 字节）。编码写在 crate 的文档注释里，**不被任何别的 crate 解释**。它只用于两件事：
①`LearningProposal` 的绑定（§5.4）；②照片 R5 的「同输入同指纹、改一组即变」。**它不是 §322 的
「版本」**——版本在每组上，指纹是五组当前状态的快照。

**`CoreFingerprint` 不承诺跨版本稳定**：编码只在本版本内解释；改编码会让所有旧提案过期（fail-closed，
这是可接受的方向）。R5 只证「同输入同指纹」与「五组逐组敏感」，**不证跨进程／跨版本稳定**。若要跨进程或
跨版本解释它，须另立版本号与 golden vector——**本设计不做**（§3.6 第 3 行）。

## 4.3 五组规则的位置：上游归属与照片 R9 的装配状态

**「位置的落地」的读法**：本块不搬动五组规则（它们仍在各层），落的是它们在 `ProtectedCore` 里的
**槽位**（五个字段）与每个槽位的**上游归属**。逐组：

| §322 字段 | 规则本体的归属 | 上游类型今天在哪（实测） | 照片 R9 中的状态 |
|---|---|---|---|
| `authority_rules` | 长期层 Authority Host（§292），未建 | **无**。`continuum-semantics` 只有 `TaskContract.authority_limit: Opaque`（`contract.rs:218-219`）与 `ViolatedProhibition::Authority`（`constraint.rs:219`）——前者是每任务契约的不透明字段，后者是违规类别，**都不是权威规则集** | `Unavailable` |
| `capability_rules` | 资源层 `continuum-capability` | `CapabilityKind`（`capability.rs:19`，12 臂）、`Capability`（`:317`，§253 五要素）、`Issuer`（`:294`）、`mint`（`:427`）。**没有 `CapabilityKind::ALL`**（`:120` 的注记逐字），规则集要装配方**穷举 12 臂** | `Committed`（词表摘要，版本 1） |
| `contract_rules` | 语义层 `continuum-semantics` | `TaskContract`（`contract.rs:207`）/ `Requirement`（`:154`）。**两重阻塞**：①它是每任务契约，不是「契约语义」的规则集；②引用它要新增 `语义层 (2) → 认知层 (6)` 的边（§9.1 无此边） | `Unavailable` |
| `audit_rules` | 边界层 `continuum-events` | `AuditKind::ALL`（`audit.rs:38`，8 臂）与 §313／§114 的哈希链；`AuditKind::AuthorityChanges` / `ContractChanges` / `UserApprovals` 是审计**记录**的类别，不是被审计的规则 | `Committed`（8 个类别名，版本 1） |
| `effect_rules` | 边界层 `continuum-effect` | `EffectType::ALL`（`effect.rs:31`，6 臂）与 `EffectState::ALL`（`:112`，7 臂）；策略裁决在 `continuum-policy` | `Committed`（6 类型 ＋ 7 状态，版本 1） |

**这张表要读出三件事**：

1. **五个槽位都建了，三组有内容、两组是显式的 `Unavailable`**。两组缺失不是本设计的疏漏，是上游不存在
   （§八.2 第 5 条要求「登记为上游缺口而不是在 P6 造一份」）。
2. **三组「已提交」只是词表摘要**：`capability` / `audit` / `effect` 的承诺覆盖的是**枚举名**，不是规则
   的不变量。`mint` 的非空 `scope` 检查（`capability.rs:432`）、审计链的 `prev_hash` 规则（`audit.rs`）、
   策略六级（`policy/src/rule.rs:18-25`）都是**代码**，没有可提交的数据形。**故承诺能发现「词表变了」，
   发现不了「不变量被改了」**。这是本设计最需要被读到的一句限制（§8.4 第 1 条、§10.1 第 6 条）。
3. **没有一组来自语义层或长期层**：`authority_rules`（长期层）与 `contract_rules`（语义层）恰好是
   §9.1 里没有「→ 认知层」边的两层。这不是巧合，是 §4.2 那条结构的直接后果。

**两组 `Unavailable` 不使判据 4 失效**：判据 4 防的是「学习产物修改五类语义」，而学习侧对五组规则的
持有方**连命名都做不到**（§2.3 的 crate 事实）。`Unavailable` 说的是「照片 R9 的装配核心没有这一组的承诺」，
不是「学习可以改这一组」。**两件事要分开读**：前者是**核心完整性**的缺口（§10.1 第 4、5 条），后者是
**边界**的缺口（本设计没有）；把前者误读成后者，会去补一个并不存在的边界洞。

## 4.4 §321 五语义 ↔ §322 五组：本设计的映射

规范没有给这张对应表。**本设计给一张，并逐格说明它为什么不是双射**。先把 §321 的五项落成类型
（**这是 §321 的词，不是 §322 的字段**）：

```rust
/// §321 的五类语义，逐字。`as_str` 返回规范原文（含空格与大小写）。
pub enum ProtectedSemantic {
    Authority,      // "Authority semantics"
    Contract,       // "Contract semantics"
    Permission,     // "Permission invariants"
    Approval,       // "User approval requirements"
    SecurityPolicy, // "Core security policy"
}
```

`ALL` 五枚；`as_str` / `parse` 严格互逆；`match` 穷尽且无通配臂。它**不是**五组规则的类型：它只是
§321 那张名单的编码，没有一条规则的字段（§3.5 的「不新增的类型」）。

| §321 的语义（逐字） | 承载它的 §322 组 | 映射的方式 | 覆盖的残余 |
|---|---|---|---|
| `Authority semantics` | `authority_rules` | 一对一的槽位 | 上游不存在（§4.3），今天只承诺「无」 |
| `Contract semantics` | `contract_rules` | 一对一的槽位 | 上游不可引用（§4.3） |
| `Permission invariants` | `capability_rules` | 权限词汇表 ＋ 签发不变量 | 不变量在 `mint` 的代码里，摘要只覆盖词表 |
| `User approval requirements` | **无专属组**：`capability_rules`（`Issuer::PolicyWithExplicitApproval`）＋ `audit_rules`（`AuditKind::UserApprovals`）＋ 边界层 `continuum-policy` 的 `Decision::RequireApproval`（`rule.rs:91`） | 多对一：三处共同承载 | §322 没有一枚字段叫 approval；批准**判定**在 `continuum-policy`，不在核心 |
| `Core security policy` | `effect_rules`（效应类型封闭）＋ `audit_rules`（审计不可关）＋ 边界层 `continuum-policy` 的 `Level::SystemSafety`（`rule.rs:19`，内建、无 setter） | 一对多：两组＋边界层策略层级共同承载 | 「核心安全策略」没有单一类型；六级层级是代码常量 |

**这张表的用法**：它证明 §321 的每一项**至少被一个槽位承载**，同时暴露两处非双射——
`User approval requirements` **没有专属字段**，`Core security policy` **跨两组＋边界层策略层级**。**本设计不为此加第六个
字段**（§322 是五字段，加字段是规范级改动），也不把 `approval` 塞进某一个字段假装是双射。报出：§10.1
第 3 条、第 13 条。

## 4.5 ENG-001 的方向选择：取 (a)

ENG-001（`docs/02-工程.md:647-653`）逐字：

```
ENG-001  Protected Core 的强制形态
  来源      OPEN-013
  问题      如何保证 Runtime Learning 无法修改 §321 列出的五类语义
  影响      认知层全部；边界层全部
  可选方向  a) 独立 crate，Learning 模块无写权限
            b) 签名配置，载入时校验
            c) 编译期固化，不提供运行时修改接口
```

**本设计取 (a)**。三条理由：

1. **它是三条里唯一能在不搬运规则类型、不引入信任根的前提下由编译器强制的**。
   (b) 的「载入时校验」在运行期做，而**被防的对象（Runtime）就是运行期的那个进程**：它能载入配置，
   就能重签配置；没有信任根时，签名只是自己给自己盖章。(c) 的
   「编译期固化」要求规则值以 `const` 存在于某处——而五组规则**不在本 crate**，`const` 化就要把规则类型
   搬进来，直接撞上 §八.2 第 5 条；且每次规则变更都要重编认知层。(a) 的「独立 crate」是 Rust 的可见性
   边界，**不需要信任根、不需要重编、不需要运行期判定**。
2. **它同时兑现 §6.2 的「先于」**（`docs/02-工程.md:382`）：crate 边界在编译期存在，学习 crate 要引用
   `ProtectedCore` 才能编译；故边界**先于**学习机制存在，不靠实施顺序。
3. **它与 OPEN-013 的第三案同义**：OPEN-013（`docs/01-总纲.md:1886-1889`）的三案是「分界线硬编码且不可
   学习／学习产出物必须经用户批准才生效／分界线由 Protected Core 持有，Learning 仅具只读访问权限」。
   (a) ＝第三案。**本设计不取第二案**（学习产出物须用户批准才生效）：那是**应用**学习产物的条件，应用
   不在本块（§9）；本块只保证产物**指不到**受保护语义。

**「无写权限」要读准**：学习 crate 仍可调用 `ProtectedCore::seal` 造一枚**新**核心——那是构造，不是写；
它没有 `&mut`、没有 setter、也没有对将来装配方持有的权威核心（本轮不存在）的替换路径。**判据 4 要的是后者。** 若把「无写
权限」读成「学习 crate 连 `seal` 都调不到」，那是把构造与写混为一谈，且会逼出一个没必要的私有构造点
（装配方也用不了）。

**(b) 里有一半被保留**：`RuleCommitment::committed` 从字节**重算**摘要，不接收调用方给的摘要——这是
「载入时校验」的**完整性**那一半，不是签名那一半。它拦的是「摘要与字节不符」，拦不住「有人重签一份
新的」；**不要把这一半说成 (b) 已被采纳**。

**(c) 里有一半被保留**：`ProtectedCore` 没有任何运行时修改接口（无 setter、无 `&mut self`）。这不是
因为选了 (c)，而是因为**根本不需要写接口**：更新路径（§4.6）本轮不建。**「没有接口」与「编译期固化」
是两件事**：前者是缺席，后者是保证。

**选择要回写到哪**：ENG-001 是《工程》§10.1 的未决项，本设计的选择**不自动关闭它**；缺的是「把这条选择
记入 ENG-001 的状态」这一步。收件人：协调者 ＋《工程》维护者（§10.1 第 1 条）。

## 4.6 §322 的三条更新 MUST：今天的位置

| MUST | 今天在哪 | 缺什么 |
|---|---|---|
| `显式版本化` | `RuleCommitment::Committed { version }`；`RuleVersion::new(u64) -> Option<Self>`，0 不合法 | 没有**更新**入口，故「新版本必须大于旧版本」这条**无处可判**。今天只有封存时的初始版本 1 |
| `独立验证` | **无** | 验证者的接口与产物类型；它属哪一层（P5a 的 `Verifier`？边界层的独立审查？）规范未给 |
| `用户批准` | **无专属类型**。边界层有 `continuum-policy::ExplicitApproval`（`context.rs:12`，一个无字段的标记）与 `Decision::RequireApproval`（`rule.rs:91`），`continuum-capability` 有 `Issuer::PolicyWithExplicitApproval`（`capability.rs:295`），`continuum-events` 有 `AuditKind::UserApprovals`（`audit.rs:32`） | 这四枚都不是「对 Protected Core 一次更新的批准值」；**本设计不拿它们冒充**——批准值的语义、铸造点与作用域都未给（§10.1 第 2 条） |

**为什么本轮不建更新路径**：建它就要同时建「独立验证」与「用户批准」两枚类型，而这两枚的类型归属
不在认知层（P5a 与边界层）；在 P6D 造一份就是第二套词汇。**§6.4 判据 4 只要求「学习产物无法修改」，
不要求「更新路径存在」**——本设计只做前者，把后者据实登记为缺口。

---

# 5. Runtime Learning 的形状（§320 §216 §217）

## 5.1 可学习目标：六类

§320 的五类 ＋ §216 的第六类（`Scheduling Heuristic`），理由与词形取舍见 §2.2：

```rust
/// §320 的五类 ＋ §216 的 Scheduling Heuristic。闭枚举，`ALL` 六枚。
pub enum LearningTarget {
    RoutingPolicy,          // "routing policy"
    PlanningHeuristics,     // "planning heuristics"
    ExecutionMethod,        // "execution method"
    VerificationStrategy,   // "verification strategy"
    ContextSelection,       // "context selection"
    SchedulingHeuristic,    // "scheduling heuristic"
}
```

`as_str` / `parse` 严格互逆（表外串 `None`，不取默认），`ALL` 六枚；`match` 穷尽且无通配臂——加第七类
时编译不过。**不派生 serde**：本块不落库，多一条编码路径就是多一处会与 `as_str` 分叉的地方（同
`continuum-effect::EffectType` 的口径，`effect.rs:8-11`）。

**这六类为什么不是「六个布尔位」而是六个枚举臂**：`LearningProposal` 的 `target` 是**单值**。一条提案
改一个目标；改两个目标要两条提案。若用位掩码/集合，一条提案可以同时指向六个目标，而「应用」时的
原子性、失败回滚与审批单位都无从谈起。**单值不是省事，是把「一条提案一件事」写进类型。**

## 5.2 观察类别：五类

§216（`04-method.md:1181-1186`）的五类：

```rust
/// §216 的五个观察类。
pub enum ObservationKind {
    PlanRework,                 // "哪些计划经常返工"
    ModelSkillFailure,          // "哪些模型在哪些 Skill 上失败"
    VerificationMiss,           // "哪些验证方式经常漏错"
    ExecutionMethodReliability, // "哪些 Execution Method 更可靠"
    ContextSelectionNoise,      // "哪些 Context 选择导致噪声"
}
```

`as_str` 返回规范里的**中文原句**（不是英文短语）——这是本块**两处 `as_str` 例外**之一（另一处是
`ProtectedSemantic`，它用 §321 的原文，§4.4）。理由：§216 只给了中文，英文名是本设计取的**臂名**，
不是规范词；把英文塞进 `as_str` 会让读者以为规范给过英文。**臂名与 `as_str` 的分离是刻意的**：臂名给
Rust 用，`as_str` 给照片与日志用。

`as_str` / `parse` 严格互逆（表外串 `None`）；`ALL` 五枚；`match` 穷尽无通配臂。**不派生 serde**（同上）。

## 5.3 记录与提案：两枚类型，两种角色

```rust
/// 一条观察记录。**不是产物**：它没有 target，指不到任何语义（§2.3）。
/// 字段 pub：纯数据，没有不变量可挂（同 P6B 的 ContextProvenance、P1 的 Artifact）。
pub struct LearningRecord {
    pub kind: ObservationKind,
    /// 被观察对象在它自己那层的标识（不透明串）。弱引用，§10.1 第 7 条。
    pub subject: String,
    /// P5a 的 Evidence 的不透明标识。本 crate 不依赖 continuum-verify（§3.2）。
    pub evidence_refs: Vec<String>,
}

/// 一条学习产物。**唯一构造点**收 `&ProtectedCore`：指纹由它读出（§5.4）。
/// 字段私有：`core_fingerprint` 与 `target` 的搭配是构造点维持的不变量。
pub struct LearningProposal {
    target: LearningTarget,
    subject: String,
    rationale: String,
    core_fingerprint: CoreFingerprint,
}

impl LearningProposal {
    /// **唯一构造点**。`core` 只读：本函数不改变它，也不保存它。
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

    /// 这条提案是不是在**当前**这枚核心上学的。
    pub fn is_current_for(&self, core: &ProtectedCore) -> bool;
}
```

**`LearningRecord` 的字段为什么是 `pub`**：它是一份纯观察记录，规范没给任何字段间的**不变量**（§216 只给
「记录什么」）。给它私有字段 + 构造函数只会造一个不判任何东西的入口（同 P5b 对死字段的处置）。

**`LearningProposal` 的字段为什么是私有**：`core_fingerprint`
必须由构造点从**调用方给的那一枚** `&ProtectedCore` 读出；若字段 `pub`，调用方可以把它改成**另一枚
核心**的指纹，绑定当场失效。

**`subject` / `evidence_refs` 是弱引用，据实记**：`String` 不是 `MethodId` / `VerificationPolicy` /
`EvidenceId` 中任何一枚。理由：引用它们要加四条边（§3.2 末），而本块今天**不应用**提案，强类型带不来
任何一处可执行的判定；P5b 的 `realized_by: Vec<String>` 有同形的先例与同形的弱引用申报。**收件人：
规范维护者 ＋ P5a**（§10.1 第 7 条）。

## 5.4 提案为什么带 core 指纹

§322 的「显式版本化」只说**更新**要版本化；规范没有要求「学习产物与核心版本绑定」。本设计加这一条，
理由是**它把「学习不能自行取消规则」从一句禁令变成一条可检的关系**：

- 学习产物是在某一版规则下学的。规则换了版本，旧提案的「更优」判断可能建立在旧语义上。
- 绑定指纹后，**核心一变，旧提案自动「过期」**（`is_current_for` 返回 false），不需要学习侧知道
  「哪一组变了、变成了什么」——它只拿得到指纹，拿不到规则本体。这正是「只读访问」的形状。
- **代价照实记**：每一次核心变更都会让**全部**旧提案过期，哪怕变的是一组与提案目标无关的规则。
  这是 fail-closed 的一侧：宁可让提案重学，不让它跨版本静默生效（§2.4 第 7 行）。

**指纹不是版本**：`RuleVersion` 在每组上，`CoreFingerprint` 是五组当前状态的快照（§4.2）。一条提案
绑定的是快照，不是某一组的版本——因为它要回答的是「整枚核心还是不是那一枚」，不是「某一组升了几版」。

## 5.5 判据 4 的落点：三条结构事实 ＋ 一组照片

§2.3 的三条事实，逐条给可执行落点：

| 事实 | 落点 | 照片 |
|---|---|---|
| 类型：六类目标里没有受保护语义 | `LearningTarget` 的六个臂；`LearningProposal::new` 只收它 | R6、R7、C3 |
| crate：学习侧命名不到五组规则的持有方 | `ALLOWED` 的 `continuum-learning` 行只有 `continuum-protected` | 既有 `every_crate_depends_only_on_its_allowed_set`（`:478`）＋ §7.3 的并集复核 |
| 值：核心没有写入口，提案拿 `&ProtectedCore` | `ProtectedCore` 无 `&mut self`／setter／`Default`；`LearningProposal::new(&ProtectedCore, ...)` | C1、C2、S1、S2 |

**这三条是「与」不是「或」**：没有类型事实时，学习代码可以把 `ProtectedSemantic` 当目标（字符串或枚举
都能编译，只是不能当 `LearningTarget`）；没有 crate 事实时，学习代码可以直接依赖 `continuum-policy` 改
规则；没有值事实时，学习代码可以对核心取 `&mut`。

**三条合起来覆盖的是「类型与依赖面」**：产物不能在类型上把受保护语义作为 `target`，不能命名其持有方，
也不能对权威核心取 `&mut`。**它不覆盖 `subject: String` 携带一个受保护语义名**——那在类型上可编译，
是否被应用是应用侧的检查（§10.2 第 12 条的具名义务）。**这就是为什么 §7 的并集复核是本设计的一半，
而不是附录**：它补的是「将来有人给学习 crate 加一条边」那一半，不是 `subject` 那一半。

---

# 6. 与 P6C 的接口（一页）

**这一页只写接缝，不造第二份词汇。**

1. **P6C 的 `RUNTIME` scope 不是本块的边界**。P6C 的 `MemoryScope::RUNTIME` 是「模型表现、工具行为、
   调度经验」的存储面（其设计 §12.2 第 3 条），它**明写自己不施加 §321**。本设计**不**把
   `MemoryScope::RUNTIME` 当作 Protected Core 的替身，也不把 §321 的五类语义塞进 claim 的 `scope` 字段。
   边界只有一处：`continuum-protected`。
2. **本块今天零持久化**。`LearningRecord` / `LearningProposal` 只在内存里；P6C 的八张表（其设计 §9.2）
   没有一张由本块写。故本块**不登记** `continuum-learning → continuum-memory`（§3.2），也**不占迁移号**。
3. **若将来落库，走 P6C 的唯一产生点**。P6C 的 claim 只由 `apply_synthesis` 产生（其设计 §5.5）；学习
   记录若要成为 `RUNTIME` claim，只能作为一次 synthesis 的 proposal 进去，**不另开写入口**。那时加一条
   `continuum-learning → continuum-memory` 的普通边（同层边），并重跑 §7.3 的并集复核。
4. **词汇表只有一份**：本块不定义 `Memory*` 任何类型；若将来写 claim，用 P6C 的 `MemoryClaim` /
   `MemoryScope` / `MemoryClaimId`，不另造 `LearningClaim` 一类同义类型。反过来，P6C 不 import
   `ProtectedCore`，也不把 `RUNTIME` scope 当作保护边界。
5. **一个已知的不对称，据实记**：P6C 的 `RUNTIME` claim 可以被 `apply_synthesis` 写、被 `correct` 撤；
   本块的学习记录**没有撤回通道**（它只在内存，进程结束即消失）。这不是本块偷懒：撤回要一条持久化与
   一个事务边界，两者都在 P6C／P0；若将来落到 `RUNTIME`，撤回随 claim 的 `correct` 走，本块不另建。

---

# 7. 依赖方向加固面

## 7.1 加固的是哪条边

切分 §五「P6D 的加固面」逐字：

> **加固的边**＝既有 workspace 成员之间的依赖方向，即 `crates/continuum-runtime/tests/dependency_direction.rs`
> 的 `ALLOWED` 逐对断言所钉住的那张表（`:25`；用例 `:288`，名单派生处 `:234`）。

**（实测：用例 `:478`、`workspace_crates()` `:424`；`ALLOWED` 的 `:25` 未漂。切分引的 `:288`／`:234` 是
写它时的行号，§11 第 2 条已记。）**

**本设计对这句话的落实是：不修改既有 crate 之间的边；只给装配方 `continuum-runtime` 增一条指向新 crate
的 dev 边（§3.2、§3.3），加固的是这张表的可观察性。** 具体三件：

1. **两个新 crate 的行是「最窄」的**：`continuum-protected` 空数组（ENG-001 (a) 的强制面），
   `continuum-learning` 一项。
2. **本块让既有行只有 `continuum-runtime` 增一枚**（dev 边），其余 26 行一字不动。其余三块若也要装配，
   只允许在 runtime 这一行增（§7.3 断言 5）；**增在任何别的既有行上都是反向边**。
3. **加一条整表复核**：从「每行对 Cargo.toml」升级为「整表对 P6 前基线（含 P5 六行）＋ P6 五行对表」。
   **这一条是本块对「加固」二字的实质贡献**——它抓的是现有用例抓不到的那一类（§7.5）。

## 7.2 三条判据的可观察形态

切分 §五给了 (a)(b)(c) 三条，逐条落到本设计：

| 判据 | 原文 | 本设计的可观察形态 | 落点 |
|---|---|---|---|
| **(a)** | `cargo test -p continuum-runtime --test dependency_direction` 在**含 P6 全部新 crate 的树**上绿 | 32 个 workspace 成员（27 ＋ P6 的 5）的整表用例绿；三个用例都跑（两个原有 ＋ 并集复核） | §7.6 命令 1 |
| **(b)** | P6 新建的每枚 crate 在 `ALLOWED` 里各占一行，行内只列它的**实际直接边** | 既有 `every_crate_depends_only_on_its_allowed_set`（`:478`）逐对 `assert_eq!` ＋ `workspace_crates()`（`:424`）的互为覆盖；本块再加「每行只等于声明的那一行」 | §7.3 断言 6、8 |
| **(c)** | 既有行不因 P6 增枚，唯一例外是 `continuum-runtime`（`:206`） | 基线比对：26 行逐行相等；runtime 行只增 `P6_NAMES` 的子集、不减 | §7.3 断言 4、5、7 |

**本设计另加一条 (d)**（不是切分给的，是 §四 第 1 条「并集复核具名给 P6D」的可执行形状）：

> **(d) 整表复核**：`ALLOWED` 的并集 ＝ P6 前基线（27 行，逐行不变）＋ P6 的 5 行（逐行等于声明）＋
> runtime 行只增 P6 的 5 个名字中的子集。**增在任何其他既有行上、或增在任何基线行上，即为新增反向边。**

(d) 与 (b)(c) 的分工：(b)(c) 各查一行，(d) 查**合并后的整张表**——这是「三份计划共写一张表」那类
事故的落点（切分 §四 第 1 条引 `docs/superpowers/p3f-followups.md:56`）。

## 7.3 并集复核的形状（测试）

**位置（两处，分开放——基线的自保护修法）**：

- **用例**：`crates/continuum-runtime/tests/dependency_direction.rs` 的**文件末尾**（P6D 的第二处共写；
  放末尾以缩小与 P6A／P6B／P6C 的冲突面），名为 `p6_union_adds_only_its_own_rows`。
- **基线**：`crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt`（**新建，P6D 自己的数据文件**）。
  **基线不在共写文件里**：改 `ALLOWED` 与改基线必须动**两个文件**，合并评审的 diff 里能看见（§7.3.5）。
- **`ALLOWED` 的读取**：用例用 `include_str!("dependency_direction.rs")` 读入**当前**的共写文件，用一个
  **严格的表解析器**取出 `ALLOWED`。解析器自带自检：取出的行数必须等于 `workspace_crates().len()`、
  subject 无重复、每个元素都是 member。**格式一变，解析器报错而不是静默漏行**——这是它不写成
  「手抄一份当前表」的理由（手抄的第二份表会与真表分叉，且分叉时两边都自洽）。**`include_str!` 嵌入的是
  文件的源码文本（含本用例自身），不是递归展开**；解析器只取 `const ALLOWED` 到它的 `];` 这一段。

### 7.3.1 基线：`tests/fixtures/p6_pre_allowed.txt`（27 行，P6D 自己的数据文件）

**格式**（一行一枚 crate；`#` 开头为注释；`crate:` 表示空允许集，`crate:dep1,dep2` 表示非空）：

```
# P6D 自己的基线：03e79c7 的 ALLOWED（27 行）。
# 改这个文件是 P6D 的改动，不是 P6 任一执行块的改动。
# 这是 P6 合并门，不是永久门。见设计 §7.3.5 / §7.6。
continuum-core:
continuum-events:continuum-core
continuum-persist:continuum-events
...
continuum-runtime:continuum-artifact,continuum-budget,...
```

**来源**：`03e79c7` 的 `ALLOWED` 表逐行抄录（**含注释行所记的 dev 边**；基线取的是**实际表**，不是任何
一份设计的边表）。**27 行**，与 `03e79c7` 时的 27 个 workspace 成员一一对应。为便于评审逐行核对，下面给出
**等价的 Rust 形式**（实际存储是上面的文本格式；两者必须逐行相等）：

```rust
// 等价形式，供核对；不是要写进 .rs 的常量。
const PRE_P6_ALLOWED: &[(&str, &[&str])] = &[
    ("continuum-core", &[]),
    ("continuum-events", &["continuum-core"]),
    ("continuum-persist", &["continuum-events"]),
    ("continuum-provider", &["continuum-capability", "continuum-core", "continuum-persist"]),
    ("continuum-artifact", &["continuum-core", "continuum-events", "continuum-persist"]),
    ("continuum-port", &["continuum-artifact"]),
    ("continuum-operator", &["continuum-artifact"]),
    ("continuum-graph", &["continuum-artifact", "continuum-events", "continuum-port", "continuum-operator", "continuum-persist"]),
    ("continuum-workspace", &["continuum-events", "continuum-persist"]),
    ("continuum-sandbox", &["continuum-workspace"]),
    ("continuum-effect", &["continuum-events", "continuum-persist"]),
    ("continuum-policy", &["continuum-artifact", "continuum-effect", "continuum-persist"]),
    ("continuum-capability", &["continuum-core", "continuum-effect", "continuum-events", "continuum-persist"]),
    ("continuum-secrets", &["continuum-capability"]),
    ("continuum-connector", &["continuum-capability", "continuum-core", "continuum-effect", "continuum-persist", "continuum-provider", "continuum-secrets"]),
    ("continuum-model-registry", &["continuum-capability", "continuum-core", "continuum-persist"]),
    ("continuum-node", &["continuum-artifact"]),
    ("continuum-method", &[]),
    ("continuum-canonical", &["continuum-persist"]),
    ("continuum-semantics", &["continuum-canonical", "continuum-persist"]),
    ("continuum-budget", &["continuum-persist", "continuum-semantics"]),
    ("continuum-media", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify"]),
    ("continuum-verify", &["continuum-artifact", "continuum-events", "continuum-graph", "continuum-operator", "continuum-persist"]),
    ("continuum-research", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify"]),
    ("continuum-code", &["continuum-artifact", "continuum-graph", "continuum-method", "continuum-operator", "continuum-verify", "continuum-workspace"]),
    ("continuum-image", &["continuum-artifact", "continuum-graph", "continuum-operator", "continuum-persist", "continuum-verify"]),
    ("continuum-runtime", &["continuum-artifact", "continuum-budget", "continuum-canonical", "continuum-capability", "continuum-core", "continuum-effect", "continuum-events", "continuum-graph", "continuum-model-registry", "continuum-persist", "continuum-policy", "continuum-provider", "continuum-sandbox", "continuum-secrets", "continuum-semantics", "continuum-verify", "continuum-workspace"]),
];
```

**P5 六行**（上表里的 `continuum-method` / `continuum-verify` / `continuum-media` / `continuum-research` /
`continuum-code` / `continuum-image`）**逐行冻结**，不因 P6 增枚。**一处要单独说明**：`continuum-image`
那行的 `continuum-persist` 是 P5f 的计划自定项（其设计 §2.1 未列，计划 `## 遗留` 第一节第 1 条记了取舍）。
**并集复核不重判它**——基线冻结的是**已合并的实际表**，不是设计的边表。谁要改它，是 P5f 的持有方另起
一次改动，不是 P6D 在复核里顺手改。

**fixture 文件头的两条声明**（逐字写进 `p6_pre_allowed.txt`，不是只写在本设计里）：

1. 「改这个文件是 P6D 的改动，不是 P6 任一执行块的改动；合并评审必须单独核这一处 diff。」
2. 「这是 P6 合并门，不是永久门；P7 起按 §7.6 重新基线化或退役，不得在此追加行。」

**用例上方也写同样两行注释**（读者先看到用例，不一定先看到 fixture）；两处注释是同一件事的两个入口，
不是两套规则。

**它挡不住什么**：有写权限的人仍可改 fixture；它挡的是「在同一个提交里顺手把 `ALLOWED` 与基线一起改宽
而不被看见」——现在要动**两个文件**，两处 diff 都在合并评审的眼前（§7.3.5）。**这不是防篡改，是防
静默**。

### 7.3.2 `P6_EXPECTED_ROWS`：P6 的 5 行

**来源**：P6A／P6B／P6C 三份设计（2026-10-17 工作树版）＋ 本设计 §3.2。**若三块定稿时改了自己的边表，
先改它们的 §2.1／§3.2，再由 P6D 改这张表**——P6D 不在复核里替它们改设计。

```rust
const P6_EXPECTED_ROWS: &[(&str, &[&str])] = &[
    // P6A（其设计 §2.1，四项；`<p6b-crate>` = continuum-trust）
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

**`P6_NAMES` 由 `P6_EXPECTED_ROWS` 派生**（`map(|(pkg, _)| *pkg)`），**不另写第三张名单**——两处名单会分叉，
而分叉的那一天没有任何断言会红。同理，`P6_EXPECTED_ROWS` 的每一项与各块设计的边表逐项对应，见 §7.4。

**`P6_EXPECTED_ROWS` 的所有权**：它是 P6D 的常量，但每一项来自各块的设计（§7.4）；改它要同时改被改块的
设计与 §7.4 的对表，**合并评审核这一处 diff**。它**不放进 fixture**——fixture 只装 P6 前的基线，P6 的
期望行是本轮的判断，混进历史基线会让「基线」与「期望」两件事在同一张表里分不清。

### 7.3.3 断言（八条，一条测试内）

1. **fixture 可解析且是 27 行**：`pre_p6_allowed()`（读 `include_str!("fixtures/p6_pre_allowed.txt")`）
   解析出 **27 行**，每行格式合法（`crate:` 或 `crate:dep1,dep2`），subjects 无重复；行数与
   `03e79c7` 的 `ALLOWED` 行数一致（行级核对见 §7.3.1 的等价 Rust 形式；P5／P6 行另见 §7.4）。
2. `P6_EXPECTED_ROWS.len() == 5`，且 subjects 无重复。
3. **解析器自检 + 行内元素检查**：解析器从 `dependency_direction.rs` 取出的行数必须等于
   `workspace_crates().len()`（**漏行即红**）；`ALLOWED` 的 subjects 无重复；每一行的每个**元素**都是
   `workspace_crates()` 的成员。**这条的后半是现有用例没有的**：现有用例查 subject 与「表 ↔ 树」，
   不查行内元素——`ALLOWED` 里列一个不存在的 crate，不会被它抓到。
4. **基线行逐行相等**：对每个 `(pkg, allowed)` in `pre_p6_allowed()`，若 `pkg != "continuum-runtime"`，
   则 `ALLOWED[pkg] == allowed`（**集合相等，不是包含**）。
5. **runtime 行只增不减**：`(pre_p6_allowed()["continuum-runtime"]) ⊆ ALLOWED["continuum-runtime"]`，
   且 `ALLOWED["continuum-runtime"] \ pre_p6_allowed()["continuum-runtime"] ⊆ P6_NAMES`。
6. **P6 行逐行相等**：对每个 `(pkg, allowed)` in `P6_EXPECTED_ROWS`，若 `pkg` 在 `workspace_crates()` 里，
   则 `ALLOWED[pkg] == allowed`。
7. **既有 crate 不指向 P6 crate**：对每个基线 `pkg`（`pkg != "continuum-runtime"`），
   `ALLOWED[pkg] ∩ P6_NAMES == ∅`。
8. **新增 subject 恰是已落地的 P6 crate**：
   `ALLOWED.subjects \ pre_p6_allowed().subjects == { p in P6_NAMES | p ∈ workspace_crates() }`。

**第 8 条是合并顺序容忍的**：P6B／P6C 先合入时，`continuum-trust`／`continuum-memory` 已经落地（P6B 只
加自己一行，P6C 只加自己一行）；P6A 必须在它们之后（它的边表引用两者，见 §7.6）；P6D 与三者无共享
crate，可任意先后。**等五枚全部落地，第 8 条退化成「恰好五行」**。这样每一块可以独立合入，而最终树上
的复核是严格的。**切分 §六 要求的「在真含新 crate 的树上跑」就是最后那一次**（§7.6）。

### 7.3.4 变异预告（谁红）

| 变异 | 现有用例（`:478`） | 并集复核 | 说明 |
|---|---|---|---|
| 把 `continuum-core` 的 `&[]` 改成 `&["continuum-events"]`，**同时**在 `Cargo.toml` 加真边 | **绿**（表与树一致） | **红**（断言 4） | **这一行就是整条复核存在的理由**：现有用例只查「表 ↔ 树」，查不到「表本身被改宽」 |
| 同上，只改表不改 `Cargo.toml` | 红（树里没有） | 红（断言 4） | 两侧都抓 |
| 给 `continuum-policy` 的允许集加 `continuum-protected`（真边＋表） | 绿 | **红**（断言 4、7） | P6 crate 成了既有 crate 的上游 |
| 给 `continuum-context` 的允许集加 `continuum-policy`（真边＋表） | 绿 | **红**（断言 6） | P6 行被加宽 |
| 漏掉 `continuum-learning` 那一行 | 红（覆盖） | 红（断言 8） | 两个方向 |
| 加第六枚 P6 crate 但不在 `P6_EXPECTED_ROWS` 里 | 绿（若表与树一致） | **红**（断言 8） | 未声明的成员 |
| 把 `continuum-image` 行的 `continuum-persist` 删掉（真边也删） | 绿 | **红**（断言 4） | P5 的取舍被 P6 重判 |

**最后一行要单独读出**：并集复核不只防 P6 的反向边，也防 P6 **顺手改 P5**。P5 六行是冻结的基线，
不是「可以再讨论的草稿」。

### 7.3.5 基线的所有权、自保护与生命周期

**所有权**：`tests/fixtures/p6_pre_allowed.txt` 是 **P6D 自己的文件**，不在切分 §一 1.3 的共写表里，
也不在 `dependency_direction.rs` 里。**改它是 P6D 的改动**，不是 P6A／P6B／P6C 的改动；合并评审必须
单独核这一处 diff（文件头的第 1 条声明即此）。

**自保护到什么程度（写清边界）**：

- 挡得住：**顺手**在同一次编辑里把 `ALLOWED` 与基线一起改宽——现在要动**两个文件**，两处 diff。
- 挡得住：**解析器漏行**——断言 3 的行数自检；格式一变，解析器报错而不是静默少一行。
- 挡不住：有写权限的人**故意**同时改两个文件并让测试通过；也挡不住有人改测试代码本身。**这不是防篡改，
  是防静默**；最终靠合并评审的 diff 与具名所有者（P6D）。
- **为什么不再加哈希**：哈希常量也在同一个测试文件里，故意改的人可以重算；它只增加「改基线必须同时改
  哈希」的仪式，不增加真正的保护。真正独立的是**两个文件**与**评审的 diff**。若将来要强保护，走
  `git show 03e79c7:...` 生成基线（需要完整历史），或把基线移到仓库外的签名制品——两者都超出本块。

**生命周期**：本测试是 **P6 期间的合并门**，不是永久门。

- P7 新增 crate 时，断言 1（27 行）与断言 8（新增 subject 恰是 P6 crate）二选一必红——**这是设计预期的
  到期日**，不是回归。
- P7 起二选一：①以 `PRE_P7_ALLOWED` 新 fixture 替换（连同 `include_str!` 的路径一起改；新常量、同一八条
  断言）——**保留同一八条断言时，`P6_EXPECTED_ROWS`／`P6_NAMES` 与断言 2 的 `len() == 5` 必须随 P7 重建**
  （P7 新增 crate 后计数不再是 5；只换 fixture 会让断言 2 与断言 6 对新树失效）；②**退役**本测试，改为从
  `Cargo.toml` / `cargo metadata` 生成基线。**不允许**在 P6 的 fixture 上追加行——那会同时破坏断言 1 与
  「基线是 P6 前」的语义。
- 所有者：P6D（本轮）→ 协调者（P7 起）。判定与排期写在 §10.2 第 9 条。

## 7.4 语义面：五行的对表

机械面（§7.3）保证「表没被改宽、行没被漏、P6 crate 不是既有 crate 的上游」。**它保证不了「P6 五行是否
等于各块设计的边表」**——那是语义面，P6D 在合并评审时逐块对表。

**P5 六行（冻结；逐行与 `03e79c7` 相等）**：

| crate | 期望的允许集（`03e79c7`） |
|---|---|
| `continuum-method` | `[]` |
| `continuum-verify` | artifact／events／graph／operator／persist |
| `continuum-media` | artifact／graph／method／operator／verify |
| `continuum-research` | artifact／graph／method／operator／verify |
| `continuum-code` | artifact／graph／method／operator／verify／workspace |
| `continuum-image` | artifact／graph／operator／persist／verify |

**P6 五行（四块；P6D 占两行。逐行等于各块设计的边表）**：

| crate | 设计边表 | `ALLOWED` 行 | 对表结果 |
|---|---|---|---|
| `continuum-context` | P6A §2.1 四行：artifact／core／memory／trust | 四项 | 按 2026-10-17 工作树版对齐，待合并评审 |
| `continuum-trust` | P6B §2.1：零项 | `&[]` | 按 2026-10-17 工作树版对齐，待合并评审 |
| `continuum-memory` | P6C §3.2：persist 一项 | 一项 | 按 2026-10-17 工作树版对齐，待合并评审 |
| `continuum-protected` | 本设计 §3.2：零项 | `&[]` | 本设计 |
| `continuum-learning` | 本设计 §3.2：protected 一项 | 一项 | 本设计 |

**对表的口径**：只核「每一条边在不在、边别（普通／dev）与设计一致」；不重判设计。若某块的边表在评审中
改了，改动方先改自己的设计，P6D 再改 §7.3.2 的 `P6_EXPECTED_ROWS`——**一次改动两处，不是两套表**。

## 7.5 现有用例抓不到的那一类

**这不是对现有用例的批评，是它的射程**。`every_crate_depends_only_on_its_allowed_set`（`:478`）做的是
**表 ↔ 实际依赖**的逐对相等：

- 若一条反向边**没登记**，`cargo tree` 里出现、`ALLOWED` 里没有 ⇒ 红。
- 若一条边**登记了但没用**，`ALLOWED` 里有、`cargo tree` 里没有 ⇒ 红。

**它抓不到的是**：一条反向边**既在 `Cargo.toml` 里、又在 `ALLOWED` 里**——两侧一致，用例全绿。这正是
切分 §四 第 1 条点名的历史事故形态：「`ALLOWED` 曾由三份计划共写，而『合并后的并集对不对』没有任何
task 认领」（`docs/superpowers/p3f-followups.md:56`）。三个人各自把行改对，合并后多出一条谁都不认领的边，
**每一行都自洽，整张表错了**。

**并集复核的 27 行基线就是为这一类准备的**：它不读设计、不读 Cargo，只读「P6 前的表长什么样」。表被
改宽，无论 Cargo 跟不跟，断言 4 都红。**这就是「加固」与「再加一条逐行断言」的区别**：前者钉住**变化
的许可集**，后者只钉住**当前的一致**。

## 7.6 运行手册与验收时点

**验收时点**：四块全部落地后，由 P6D 在**合并树**上跑（切分 §六：三条判据要真跑，不逐字推演）。
**这是 P6 期间的合并门，不是永久门**：P7 起按 §7.3.5 的生命周期重新基线化或退役。

```
# 1. 整表（三个用例：两个原有 ＋ 并集复核）
cargo test -p continuum-runtime --test dependency_direction

# 2. 只跑并集复核
cargo test -p continuum-runtime --test dependency_direction p6_union_adds_only_its_own_rows -- --exact

# 3. 受保护 crate 的入度（人工核对：除本包自身外无 continuum-* 行）
cargo tree -p continuum-protected --edges all --depth 1

# 4. 学习 crate 的唯一一条 workspace 边
cargo tree -p continuum-learning --edges all --depth 1
```

**期望**：1 与 2 绿；3 的输出里只有 `continuum-protected v0.1.0 (...)` 与 `sha2` / `thiserror` 的**直接
依赖**（`--depth 1` 只给直接依赖；不再声称「及其传递依赖」），**没有第二枚 `continuum-*`**；4 的输出里
`continuum-protected` 恰一行，没有第二枚 `continuum-*`。

**中间态与合并顺序**：**P6B 与 P6C 可以先合入**（它们的行不引用别的 P6 crate）；**P6A 必须在 P6B／P6C
之后**——它的边表引用 `continuum-trust` 与 `continuum-memory`，Cargo 的 path 依赖不存在时它连编都编不过；
**P6D 的两枚与三者无共享 crate，可任意先后**（两枚一起合入即可）。任一中间态只要求「现有用例 ＋ 并集复核
的已落地部分」绿；**不要求**五枚全在。第 8 条断言的合并顺序容忍见 §7.3.3。

---

# 8. 判据与照片

## 8.1 运行期（正例 ＋ 反例成对；缺一条即不算钉住）

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| R1 | `RuleCommitment::committed(v, b"abc")` 对 `b"abc"` 为 `matches == true`、对 `b"abd"` 为 `false`；`digest().to_hex()` 是 64 个十六进制字符 | 承诺的正例与反例 |
| R1b | `RuleCommitment::committed(v, b"")` ⇒ `Err(EmptyRules)`；`committed(v, b"\0")` ⇒ `Ok`（**空字节与一个零字节的区别**） | `EmptyRules` 的正例与反例（§4.2、§3.6 第 5 行） |
| R2 | `RuleCommitment::unavailable()` 的 `version()`／`digest()` 都是 `None`，`matches(任意)` 恒 `false` | `Unavailable` 不是「空摘要」 |
| R3 | 五组全 `unavailable()` 的 `seal` ⇒ `Err(NothingCommitted)`；一组 `committed` ＋ 四组 `unavailable` ⇒ `Ok` | 空壳的拒绝与部分核心的放行（两侧） |
| R3b | `RuleVersion::new(0)` ⇒ `None`；`RuleVersion::new(1).map(|v| v.get())` ⇒ `Some(1)`；`RuleVersion::new(u64::MAX)` 可构造 | 版本号的合法域（§4.2、§3.6 第 1 行） |
| R4 | 用**字段名**构造 `ProtectedCoreParts`，五组给五个可区分的承诺；`seal` 后逐组断言访问器返回的正是该组（把 authority 与 contract 对调，断言 R4 红） | 五个字段没有传反（§4.1） |
| R5 | **五组逐组表驱动**：同一组 parts 两次 `seal`，`fingerprint` 相等（稳定性）；基准 core 的 `fingerprint` 为 F0；对每组 i（authority／capability／contract／audit／effect 各一行），只改该组的 `version`（或 `digest`），seal 得 Fi；断言 `Fi != F0`，且 `{F0, F1..F5}` 六个值**两两不等**。**不是只改 `authority_rules` 一组**——只改一组测不出其余四组是否进了编码 | 指纹的稳定性与**五组逐组敏感性**（§4.2） |
| R6 | `LearningTarget::ALL` 恰六枚；手写期望的六个 `as_str` 逐枚相等 | 六类的清单（**手写**，不从 `ALL` 派生期望值） |
| R7 | `ProtectedSemantic::ALL` 的 `as_str` 逐枚等于手写的 §321 五个英文串；`LearningTarget::ALL` 的 `as_str` 与那五个串**交集为空** | §2.2（二）的同义表 ＋ 判据 4 的类型面 |
| R8 | 对六个 target 各造一条 `LearningProposal`（同一个 core），逐条 `is_current_for(&core) == true`；`seal` 一枚改过一组承诺的核心，六条全部 `is_current_for(&new_core) == false` | 提案与核心的绑定（正反两侧） |
| R9 | 装配照片（`crates/continuum-runtime/tests/p6d_protected_core.rs`）：`capability_rules` / `audit_rules` / `effect_rules` 是 `Committed`，`authority_rules` / `contract_rules` 是 `Unavailable`；三组的摘要由**手写的规范字节**算出 | **照片 R9 的装配状态**（§4.3；不是生产状态——本块没有生产装配，§8.4 第 4 条）；它是 tripwire：上游落地后必须改这条用例，不许静默 |
| R10 | `ProtectedCore` 在一批提案构造前后逐字段相等（`PartialEq`）；`fingerprint()` 不变 | 「学习不改变核心」的值面 |
| R11 | `ProtectedSemantic` / `LearningTarget` / `ObservationKind` 的 `as_str` / `parse` 各用**无通配臂**的穷尽 `match`；`LearningTarget` 加第七类、`ProtectedSemantic` 加第六类、`ObservationKind` 加第六类时编译不过（这是一条编译期性质，照片即代码本身，不靠 `trybuild`） | 加臂时的编译期可见性 |

**R9 的规范字节怎么来（写清，否则照片是假的）**：

- `audit`：`AuditKind::ALL`（8 枚）逐个 `as_str`，以固定分隔符拼接；
- `effect`：`EffectType::ALL`（6 枚）＋ `EffectState::ALL`（7 枚），同样拼接；
- `capability`：**没有 `ALL`**，用手写的 12 臂 `match`（每臂给 `CapabilityKind::as_str` 的串），
  match **不带 `..`**——加第 13 臂时用例编译不过；
- `authority` / `contract`：不上字节，直接 `unavailable()`。
- **字节的生成只在用例里**（本块不做生产装配，§8.4）；生产装配落位时**必须复用同一段生成逻辑**，
  不许另写一份——两份字节会给出两个摘要，而两个摘要都「看起来对」。

## 8.2 编译期（`trybuild`）

本仓已有 `trybuild` 的既有做法（P5c 设计 §10.4 实测八处）。本块用**五份：四份 `compile_fail` ＋ 一份正控制**：

| # | 样例 | 期望 | 钉的是 |
|---|---|---|---|
| C1 | 在 crate 外写字面量 `ProtectedCore { authority_rules: ..., ... }` | **编不过**（字段私有） | 唯一构造点 `seal` |
| C2 | 在 crate 外写 `core.authority_rules = ...` | **编不过**（字段私有） | 无写入口 |
| C3 | `LearningProposal::new(&core, ProtectedSemantic::Authority, "x", "y")` | **编不过**（类型不匹配） | 学习产物指不到受保护语义 |
| C4 | `ProtectedCore::default()` | **编不过**（无 `Default`） | 没有「空核心」的默认值 |
| C5 | 正控制：`core.authority_rules()`、`core.fingerprint()`、`LearningProposal::new(&core, LearningTarget::RoutingPolicy, "x", "y")` | **编得过** | 只读面可用（否则 C1–C4 可由「一律私有」满足） |

**C5 是必需的正控制**：只有 `compile_fail` 时，一个「把所有东西都设成私有」的 crate 也能让四份样例全绿，
而它同时把只读面也封死了。**正控制钉的是「该开的开着」。**

## 8.3 源码面（`grep` / 文件级断言）

**语料限定**：`crates/continuum-protected/src/`、`crates/continuum-learning/src/`、两个 crate 的
`Cargo.toml`。**本设计文档自身含这些串，全仓 `grep` 会命中它**——故一律限定 `crates/` 下的源码。

**零命中的语料是 `src/` 的「代码标识符」，注释不算**：凡注释里解释「不调用 `seal`」或「不依赖
`continuum-policy`」都会自己命中。实现时用 `grep` 排除以 `//` / `///` 开头的行，或把模式限定为
`::` / `(` / `use ` 形式。S4 已按这一口径取「标识符全名」；S2／S3 同此（下表给的是标识符形式，不是裸词）。

| # | 断言 | 命令（示意） | 期望 |
|---|---|---|---|
| S1 | 核心 crate 没有写入口 | `grep -rn "&mut self\|fn set_\|fn update\|fn replace" crates/continuum-protected/src/` | 零命中 |
| S2 | 学习 crate 不造承诺、不封存 | `grep -rn "\.seal(\|::seal(\|RuleCommitment::\|\.committed(" crates/continuum-learning/src/`（排除注释行） | 零命中（它只用 `ProtectedCore` 的读方法） |
| S3 | 学习 crate 不命名五组规则的持有方 | `grep -rn "use continuum_capability\|use continuum_events\|use continuum_effect\|use continuum_policy\|use continuum_semantics\|continuum_capability::\|continuum_events::\|continuum_effect::\|continuum_policy::\|continuum_semantics::" crates/continuum-learning/src/`（排除注释行） | 零命中 |
| S4 | 学习 crate 不引用别的块类型 | `grep -rn "continuum_verify\|MemoryClaim\|ContextPackage\|ContextTrust" crates/continuum-learning/src/` | 零命中（**模式取标识符全名，不取裸 `Evidence`**——本 crate 的文档注释里出现「P5a 的 Evidence」是说明文字，不是引用；这是 P5c 「零命中类判据先问哪条规矩会往语料里加字」那条教训的本处用法） |
| S5 | 核心 crate 的 workspace 依赖为空 | `grep -n "continuum-" crates/continuum-protected/Cargo.toml` | 只命中 `[package] name = "continuum-protected"`，无别的 `continuum-*` 依赖 |
| S6 | 学习 crate 只有一条 workspace 边 | `grep -n "continuum-" crates/continuum-learning/Cargo.toml` | 只命中本包名与 `continuum-protected` |
| S7 | 两个 crate 零迁移、零事件、零审计 | `grep -rn "Migration::new\|EventType\|AuditKind" crates/continuum-protected crates/continuum-learning` | 零命中 |
| S8 | `ALLOWED` 行逐字 | 读 `dependency_direction.rs` 的三行（§3.3） | 与 §7.3.2 一致 |

**S3 是判据 4 的 crate 面**：它不看「有没有写函数」，只看「能不能命名那些 crate」——依赖边不在，
`use` 就写不出来。**S3 比一条「没有写入口」的 grep 更强：它对那五枚 crate 连读都禁止**（对
`continuum-protected` 的读是允许的，那正是 §6.3 的单向依赖）。

## 8.4 拍不出照片的地方

1. **五组规则的内容真的被保护**：三组是词表摘要、两组是 `Unavailable`（§4.3）。**能拍到的是「承诺与
   状态」，拍不到「规则的语义没被改」**——后者要等上游给出规则的数据形（§10.1 第 6 条）。
2. **§322 的「独立验证」与「用户批准」**：没有类型、没有入口（§4.6）。**拍不到，因为不存在。**
3. **学习真的改善了未来任务**：没有算法、没有应用、没有执行器（§1.2、§9）。本块拍到的是「产物指不到
   受保护语义」，不是「产物有用」。
4. **权威核心的单一实例**：本轮**没有生产装配**——`ProtectedCore` 只在装配用例里被 `seal`，不落库、没有
   全局单例。**若将来有生产装配**，装配方持有唯一一枚；「学习拿不到别的一枚」要靠这个唯一性，本轮拍不到
   （§10.1 第 11 条）。**「学习拿不到 `&mut`」是类型事实**，与生产装配无关。
5. **§9.4 那条边的读法**：「认知层 → 执行层」按 §9.1 读作「执行层依赖认知层」，与层间图相抵
   （切分 §八.2 第 1 条）。本块不裁定，两种读法下都不加执行层边。
6. **C4／OPEN-016 的裁定**：本块与 P6A 同口径，但那是**口径**，不是裁定（§10.2 第 4 条）。

---

# 9. 明确不做

| 不做的东西 | 归属 / 理由 |
|---|---|
| 执行器、图节点、节点调度、后台作业 | 第 3 层（P1）与第 7 层（P7）。§216 的「执行任务 → 收集 Evidence」不在本块 |
| 模型调用、路由选择、模型画像读取 | 第 4 层（P3）。本块不依赖 `continuum-provider` / `continuum-model-registry` |
| 学习算法（去重、矛盾检测、评分、生成提案） | 规范未给（§216 只给「记录什么、优化什么」）。本块只给类型与边界 |
| 提案的应用（改 routing / planning / method / verify / context / scheduling） | 各自的持有方（§1.2 末）。本块只产出提案 |
| §217 的自我改进流程 | 第 7 层＋边界层。本块只落「不能自行取消规则」这一条原则 |
| §322 的更新路径（独立验证 ＋ 用户批准） | 见 §4.6；缺类型与归属，不在本块造 |
| 签名、密钥、信任根 | ENG-001 (b) 被否（§4.5）；Authority Host（§292）未建 |
| 五组规则的类型 | §八.2 第 5 条 |
| 持久化、迁移、事件、审计 | 零表、零事件、零审计（§3.3） |
| `Evidence` / `MemoryClaim` / `ContextPackage` / 标签类型 | P5a / P6C / P6A / P6B |
| capability 判定 | C4／OPEN-016；与 P6A 同口径：判定在外（§10.2 第 4 条） |
| UI、报告、指标 | 前端／长期循环 |

---

# 10. 遗留与对账

## 10.1 遗留（逐条具名收件人；「缺的是哪一步」写到步骤）

1. **ENG-001 的选择要回写**：本设计取 (a)（＝OPEN-013 第三案，§4.5）。**缺的是「把这条选择记入《工程》
   §10.1 的 ENG-001 状态」这一步。收件人：协调者 ＋《工程》维护者。** 本设计不自称关闭该未决项。
2. **§322 的「独立验证」与「用户批准」没有实现**（§4.6）。**缺的是「独立验证的产物类型与用户批准值的
   类型由谁给、放在哪一层」这一步。收件人：协调者 ＋ 规范维护者 ＋ 边界层（P2／policy）＋ 资源层（P3）。** 今天边界层有
   四枚相邻但不等的东西（一个类型 ＋ 三个枚举变体：`ExplicitApproval` / `Decision::RequireApproval` /
   `Issuer::PolicyWithExplicitApproval` / `AuditKind::UserApprovals`），**没有一枚是「对核心一次更新的
   批准值」**；本设计不拿它们冒充。
3. **§321 五语义与 §322 五组不是双射**（§4.4）：`User approval requirements` 无专属字段，
   `Core security policy` 跨两组＋边界层策略层级。**缺的是「两张五元表的对应关系」这一条规范。收件人：规范维护者。**
4. **`authority_rules` 的上游不存在**（§4.3）：没有任何 Authority 规则类型；`authority_limit` 是 opaque
   的契约字段，§292 的 Authority Host 属长期层。**缺的是「Authority 规则的冻结类型」这一步。收件人：
   规范维护者 ＋ 长期层（§292）。**
5. **`contract_rules` 的上游不可引用**（§4.3）：`TaskContract` 在语义层，引用它要新增
   `语义层 (2) → 认知层 (6)` 的边（§9.1 无此边）；且它是每任务契约，不是「契约语义」的规则集。
   **缺的是「契约语义的规则类型 ＋ 它到认知层的边」这两步。收件人：《工程》维护者 ＋ 协调者 ＋ P4。**
6. **三组「已提交」只是词表摘要**（§4.3）：`capability` / `audit` / `effect` 的承诺覆盖枚举名，不覆盖
   `mint` 的非空 scope、审计链的哈希规则、策略六级。**缺的是「规则的字节形」这一步。收件人：规范维护者
   ＋ 各层持有方。** 在它给出之前，承诺能发现「词表变了」，发现不了「不变量被改了」。
7. **学习记录与 Evidence／目标对象的连接是弱引用**（§5.3）：`subject` / `evidence_refs` 是 `String`。
   **缺的是「学习记录怎样引用 Evidence 与目标对象」这一步。收件人：P5a ＋ 规范维护者。**
8. **三张可学习表不一致**（§2.2）：§320 五、§216 六、§217 七；`Skill library` 只在 §217；
   `Planning Policy` / `planner policy` / `planning heuristics` 三种写法。**缺的是「哪一张是规范表」
   这一步。收件人：规范维护者。**
9. **执行历史没有代码边**（§3.2）：§6.3 要求 `Runtime Learning ← 执行历史（第 3、4 层）`，但 §9.1 没有
   `执行层 (3) → 认知层 (6)` 的边。本设计以值传入。**缺的是「这条边从哪一侧登记」的裁定。收件人：
   《工程》维护者 ＋ 协调者**（与切分 §八.2 第 1 条同处）。
10. **§9.2 的「入度为零」与外部依赖**：`continuum-protected` 依赖 `sha2` / `thiserror`。本设计按「§9.1
    的八层组件」读，外部 crate 不算组件。**缺的是「外部依赖算不算入度」的读法确认。收件人：协调者**
    （与 P6C §3.4 的同形申报并列）。
11. **本块没有生产调用方**：两个 crate 的入口只有用例；生产装配（`seal` 与 `LearningProposal::new` 的
    调用点）要等执行器。**缺的是「谁在何时封存核心、谁在何时造提案」这一步。收件人：P1 ＋ 第 3 层
    执行器 ＋ 协调者。**
12. **P2 的拒绝点需要 P6B 的标签类型，但 P2 的 `ALLOWED` 行不得增枚**：这与本块的并集复核相抵——
    并集复核会把「P2 行加 `continuum-trust`」判红。**缺的是「P2 怎样拿到标签类型」这一步**（P6A 设计
    §11 第 10 条、P6B 设计已各自报出）。收件人：P6B ＋ P2 ＋ 协调者。**本设计只登记：这条例外不能靠
    改 `ALLOWED` 行解决；若要认真做，得先有一次被裁定的例外，而不是一次静默的加行。**
13. **§322 的五字段容纳不了 Context Compiler**：若 C4 裁定「capability 判定在 Compiler 内部」，Compiler
    须成为安全边界；但 §322 的五个字段**没有一枚**能承载 Context 规则。**缺的是「加第六组，还是把
    context 规则归入某一组」这一条规范级裁定。收件人：规范维护者 ＋ 协调者。** 本设计不预设。
14. **迁移表**：本块零表，不占档；P6 的 `190` 由 P6C 写死（其设计 §3.3）。**缺的是「P6 档的合并表」这
    一步。收件人：协调者 ＋ P6C。**
15. **学习记录／提案没有落库**（§6）：今天只在内存。**缺的是「落到哪（P6C 的 `RUNTIME` claim？独立表？）
    与谁写」这一步。收件人：P6C ＋ 协调者。** 若落 `RUNTIME`，必须走 P6C 的 `apply_synthesis`，不另开
    写入口。

## 10.2 与各块的对账

1. **P6A（`continuum-context`）**：无共享类型。**同一口径**：capability 判定在外、Compiler 不纳入
   Protected Core（P6A 设计 §5.5／§11 第 9 条；本设计 §10.1 第 13 条）。P6A 的 `context selection` 是
   本块六类目标之一，但两者**没有类型边**（§3.2：不登记 `continuum-context`）。若 P6A 将来把
   「context selection 的规则」做成可提交的数据形，它是 `ProtectedCore` 的**第六组**候选——那要改 §322
   （§10.1 第 13 条），不是本块现在能加的字段。收件人：P6A 的设计者 ＋ 协调者。
2. **P6B（`continuum-trust`）**：无共享类型。本块不读标签、不打标签；`continuum-learning` 不登记
   `continuum-trust`（§3.2）。P6B 的 `&[]` 行在并集复核的 `P6_EXPECTED_ROWS` 里（§7.3.2）。**P6B 已改**：
   其 `:784` 写「P6D 设计 §10 第 12 条」、`:955` 写「P6D §10 第 12 条」，不再误指本设计 §11；
   **建议精确到 §10.1 第 12 条**（本设计 §11 只有四条）。**P6D 不改 P6B 的文件**，合并评审时口头提醒即可。
   收件人：P6B 的设计者。
3. **P6C（`continuum-memory`）**：接口见 §6，一页。`RUNTIME` scope **不是**本块的保护边界；本块不写
   claim、零表、不占档。若将来落库，走 `apply_synthesis`。收件人：P6C 的设计者 ＋ 协调者。
4. **P6A／P6D 与 C4／OPEN-016、ENG-008**：**同一口径——capability 判定在 Compiler 外，Compiler 不纳入
   Protected Core**。本设计**不关闭** ENG-008（《工程》§10.2 把它列为可推迟），只登记两处设计同口径；
   正式裁定属协调者／规范维护者。**若裁定翻案**，改动不是「给核心加一枚字段」——§322 是五字段，
   加第六组是规范级改动（§10.1 第 13 条）。收件人：协调者 ＋ 规范维护者。
5. **P5a（`continuum-verify`）**：`Evidence` 不进本块（无类型边）；`evidence_refs` 是它的不透明标识
   （§5.3）。**缺的是「学习记录怎样引用 Evidence」**（§10.1 第 7 条）。收件人：P5a。
6. **P5b（`continuum-method`）／P5c（`continuum-code`）**：`execution method` 是六类目标之一，但本块
   **不依赖** `continuum-method`（§3.2）。提案的应用归方法库的持有方；本块不 `bind`、不注册算子。
   收件人：P5b／P5c 的持有方。
7. **P7（长期循环 / Background Supervisor）**：`scheduling heuristic` 是本块的目标之一；**调度本身**
   （何时学、何时应用、后台作业）在第 7 层。**缺的是「调度提案的应用点」这一步。收件人：协调者**
   （P7 搁下期间，这一项只有类型、没有应用者）。
8. **P1／第 3 层执行器**：§6.3 的「执行历史」以值传入；本块不登记执行层边（§3.2、§10.1 第 9 条）。
   **缺的是「执行历史的值形状与传入点」这一步。收件人：P1 ＋ 第 3 层。**
9. **协调者（并集复核的验收）**：本块交付的是**测试形状 ＋ 两张对表**；真正的 (a)(b)(c) 要在四块落地后
   的树上跑（§7.6）。**缺的是「谁在合并评审时跑、把输出记在哪」这一步的排期。收件人：协调者。**
   **这是 P6 合并门**；P7 起的生命周期见 §7.3.5（新基线或退役），不在本条的排期里。
10. **规范维护者（规则的数据形）**：五组规则里只有三组有可提交的词表；另两组无上游（§4.3、§10.1 第 4、
    5、6 条）。**缺的是「每组规则的字节形与版本推进规则」这一步。收件人：规范维护者。**
11. **`crates/continuum-runtime/Cargo.toml` 是事实上的第四共写面**：四块都要在这个文件里各加自己的一行
    （P6D：`[dev-dependencies] continuum-protected`；P6C：`[dependencies] continuum-memory`，因为
    `main.rs` 的 `runtime_migrations()` 要调 `p6c_memory_migrations()`；P6A／P6B 若装配同理）。切分
    §一 1.3 的共写表**只列了三处**（`runtime_migrations()`、`ALLOWED`、workspace `Cargo.toml`），没有
    这一处。**缺的是「把第四处补进切分表，并写明四块各加自己那一行、彼此不删」这一步。收件人：协调者。**
    合并评审时由 P6D 核对：每块只加自己的行，不加别人的；删行要说明。
12. **应用侧对 `subject` 的检查义务**（答复 §5.5 的残余）：`LearningProposal.subject` 是 `String`，
    一条 `target: ExecutionMethod`、`subject: "Authority semantics"` 的提案**编译得过**。本块只保证
    「类型上不能把受保护语义当 target、crate 上不能命名其持有方」；**收到 `subject` 指向 §321 受保护
    语义的提案时，应用侧必须拒绝或升级，不得静默应用**。**缺的是「这条判定的落点与谁持有」这一步。
    收件人：P1 ＋ 第 3 层执行器 ＋ 各策略持有方 ＋ 协调者。** 若将来要把 `subject` 换成受约束的类型，
    那是另一条边（§3.2 末），本设计不预支。

---

# 11. 在切分文档里发现的错或缺口（**只记，不改**）

逐条给出实测。**本设计不改切分文档**（切分 §七 与协调纪律）。

1. **⚠️ §五 P6D 行的「新建的 Protected Core／Learning crate」是单数还是两枚，没有说死。** 本设计读作
   **两枚**（填补未定，不是偏离；§3.1），理由是 ENG-001 (a) 的「独立 crate，Learning 模块无写权限」
   （`docs/02-工程.md:647`）——同一 crate 里 `pub(crate)` 就是写权限（§3.1）。**后果**：并集复核的
   `P6_EXPECTED_ROWS` 是**五行**，不是四行；若协调者裁定只建一枚，除这一张表与 §3.1 外，**断言 2 的
   `len() == 5` 要同步改成 4**（`P6_NAMES` 由表派生，自动跟随），不是 §7.3 的断言形态整体作废。收件人：协调者。
2. **⚠️ §五「P6D 的加固面」引的三个行号已漂。** 实测：用例 `every_crate_depends_only_on_its_allowed_set`
   在 `:478`（切分写 `:288`）；`workspace_crates()` 在 `:424`（写 `:234`）；`continuum-runtime` 的
   `ALLOWED` 行在 `:391-412`（写 `:206`）。`ALLOWED` 的 `:25` 仍准确。**处置**：本设计按内容引，
   行号用实测值（§7.1 的引文保留切分原文的旧行号，并在括号里给实测值）。收件人：协调者。
3. **⚠️ §八.2 第 5 条把 `authority_rules` 归到「语义层与边界层」，措辞过宽。** 实测：全仓没有
   Authority 规则类型；`continuum-semantics` 只有 `TaskContract.authority_limit: Opaque`
   （`contract.rs:218-219`）与 `ViolatedProhibition::Authority`（`constraint.rs:219`），两者都不是规则集；
   §292 的 Authority Host 属长期层、未建。**准确的写法是「无上游」**（§4.3）。这不是给 P6D 减负，是
   把缺口记对：若照「在语义层」去找，会找到一枚不存在的类型。收件人：协调者 ＋ 规范维护者。
4. **✔ 已复测为正确的两条**（记此以免复审重做）：
   (i) `docs/02-工程.md:367-368` 的两行（Runtime Learning／Protected Core）连续、准确；
   (ii) `docs/02-工程.md:405` 的判据 4 逐字引 §321，而不是 §322 的字段名——本设计 §2.3 的读法据此。

---

# 12. 收口

- **本轮的交付是设计，不是代码**（切分 §七）。实现计划另起。
- **本块对 §6.4 的贡献只有判据 4**（§5.5）；判据 1、2、3 分别属 P6A、P6B、P6C。
- **本块对 §9.2 的贡献是两枚入度为零源头之一**（Protected Core）：空允许集见 §3.4，规范级的「外部依赖
  算不算入度」申报见 §10.1 第 10 条。
- **本块对「加固」的贡献不是新边，是一张能抓住『表被改宽』的测试**（§7.3、§7.5）：现有用例抓的是
  「表 ↔ 树」；本块加的是「表 ↔ P6 前基线」与「P6 五行 ↔ 四块的设计」。**基线在 P6D 自己的 fixture 里，
  不在共写文件**；它是 P6 合并门，P7 起重新基线化或退役（§7.3.5）。
- **本块不关闭任何规范未决项**：ENG-001 的选择、ENG-008 的口径、C4／OPEN-016 的判定，分别在 §10.1 第 1
  条、§10.2 第 4 条、§10.1 第 13 条报出，收件人具名。
