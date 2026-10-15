# P4（语义层）残余台账

**本文件的执行判据是「在版控里 `grep` 得到吗」**，不是「我写过没有」——本仓吃过一次亏：
一张 gitignored 的台账（`.superpowers/sdd/p3f-minors.md`）里早有那条阻断项，而终审时
`grep -rn 'M-6-1' docs/` **零命中**，等于没人读过它。

编号约定：`M-<task>-<序号>`（Minor）；`D-<task>-<序号>`（错或缺陷）；
`P-<task>-<序号>`（**已核实的无照片**，只许记账、不许谎称有照片）。
**终审轮**（2026-10-15，全分支终审，HEAD `f5557ef9`）另用三套编号：
`I-<序号>`（终审 Important）、`Declined <序号>`（终审搁置不判）、终审 Minor `<序号>`（随手修清单）。

---

## Task 2（规范化产物两支互斥、Temporal Resolver、Ambiguity Manager）

实现 `5d6aa61`（父 `80d76b9`）；评审后订正 `1185ed9`；复评判词 **Approved**。

### 已闭合（在版控里，供终审不必重做）

- **M-2-1「恒真断言被注释说成是有照片的那一半」** —— `tests/canonical.rs` 原 `:202-211` 的
  `matches!(r, Canonical(_)) ^ matches!(r, NeedsResolution(_))` 对 `finish` 能返回的任何值都为真。
  订正 `1185ed9` **删除**了整段（不是换形，全文 grep 零命中），头注释改为指向实际会红的那两条
  `arms.contains(..)`。**判据已立**：一条恒真的守卫比没有守卫更坏——后来者会以为那里有照片。
- **M-2-2「`RELATIVE_DAYS` 的英文半边零照片」** —— 实测：删掉 7 条英文条目后全量门仍 0 红。
  订正 `1185ed9` 补了 7 条用例（`tests/canonical.rs:398-439`），**期望值是本文件手写的字面量，
  不是循环读 `RELATIVE_DAYS`**（读表就是自证的正控制）；M8/M9 各只红新用例。
- **⚠️ 门读数**：由实现者的证据承担（`1185ed9`：`Running` 109 ＋ Doc-tests 19 ＝ 128 条 `test result`，
  828 passed / 0 failed / 0 warning）。评审未复核，**这是刻意的**（不许让评审重跑整套顶替实现者那一份）。

### 已转交（不留在本台账里悬着）

- **M-2-3「`UnresolvedMention` 在类型层可装 `Resolved`」** —— `crates/continuum-canonical/src/canonical.rs:109-125`，
  不变量只由构造面保证（`:104-108` 的注释照实写了）。**已写进计划 Task 4 的 Files**
  （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md` 的 Task 4 一节），与本 task 的另一笔改动同处。
- **D-2-1「计划 Task 2 与 Task 4 次序相抵」** —— Task 2 的 `finish` 签名要用 Task 4 的 `ResolutionResult`，
  而载荷属 Task 4。**已写进计划 Task 4 的 Files**（两条 `Modify`）。

### 仍未闭合（终审要看的）

- **P-2-1「`CanonicalRepresentation` 的 crate 外不可构造性今天无照片」** —— 它的那一半要 trybuild，
  而 `tests/type_level.rs` 与 `trybuild` 这个 dev-dep 是 **Task 3** 的（`Task 2` 不动 `Cargo.toml`）。
  **已由 Task 3 拍上**（`tests/compile_fail/canonical_representation_is_not_constructible_outside.rs`
  ＋ 它那份实跑取值的 `.stderr`），闭合记录见下面「Task 3」一节的第一条。
  那之前这里记的是「无照片」，不是「已保证」。
- **台账记账缺口（2026-10-09 复审查出，本轮不修）** —— 订正轮报告 §9.4 自称「对评审三条的逐条判定」，
  而上一轮评审是 **1 Important ＋ 3 Minor**，它只答了 2 条 Minor，**答漏一条且误标了自己答复的条数**。
  被答漏的那条即 **M-2-3**（已如上转交）。**这不是代码缺陷，是记账缺口**——
  记在此处，因为「漏答一条」与「静默丢弃一条」在读者眼里没有区别。
- **过程层事实（供后来者不再重踩）** —— 首次评审报告里 `src/` 那几处的行号**是复评包的行号、不是源文件行号**
  （评审自己在复评时订正）。实测应为：`finish` `src/ambiguity.rs:92-116`、`ResolutionResult` `src/ambiguity.rs:59-88`、
  `Canonicalized` `src/canonical.rs:46-52`、`CanonicalRepresentation` `src/canonical.rs:71-85`、
  `UnresolvedMention` `src/canonical.rs:109-125`。**失效方向**：diff 包的行号与源文件行号长得一样，
  引用时若不写明「这是哪个坐标系的行号」，就没人发现。

---

## Task 3（候选面：`EntityId`、`Candidate`、`CandidateSource`、`CandidateSources`）

### 已闭合（在版控里，供终审不必重做）

- **P-2-1（Task 2 转来）「`CanonicalRepresentation` 的 crate 外不可构造性无照片」** ——
  已拍上：`tests/compile_fail/canonical_representation_is_not_constructible_outside.rs`
  （实跑得 `E0451: field \`mentions\` of struct \`CanonicalRepresentation\` is private`）
  ＋ 正控制 `tests/compile_pass/` 里那两份。**限度照实记**：它钉的是「crate 外拼不出**结构体字面量**」，
  **不钉**「拿不到这个值」——经 `finish` 拿得到，被挡住的只是绕过 `finish` 自己拼一个。

### 本 task 自定的判据（本轮新记）

- **`CandidateSources` 的拼接口径是本层自定的** —— 设计 §4.3（`design:700-729`）给这个类型的全部内容
  只有「由装配方给出」一句，**方法集与拼接口径规范未给**。本层取
  `new` / `retrieve_all`（另有 `Default`，与 `new` 同义），口径三条：**按装配顺序拼接、不去重、不排序**。
  三条都写在 `src/candidate.rs` 的 `retrieve_all` 文档里，各有一条运行期照片
  （`tests/candidate.rs::a_candidate_source_can_only_return_a_set`；「不去重」与「不塌成一枚」
  两枚变异体 M7／M7b 实测各红在该用例的不同行上）。
  **`CandidateSources` 今天没有消费者**：Task 5 的 `Interfaces. Consumes` 里只列了 Task 3 的 `Candidate`。
  **收件人：规范维护者**（§4.3 的 `CandidateSources` 只给了「由装配方给出」一句）。
  **终审亦搁置**（Declined 10——理由见终审报告）。

### 已闭合（评审与订正之后）

- **评审判词 Approved**（实现 `f5431aa`；订正 `3a5a69c`／`65ca48a`／`4d8ff95`／`4b87dab`）。
- **M-3-1「注释里的绝对承诺超出照片射程」（Important，plan-mandated）** —— 四处写的
  「本层不为单实体定义方法」只被**一个名字**的照片支撑（反例红在 `E0599: no method named pick_one`）；
  给 trait 加一个**别的名字**的单实体方法不会让任何样例变红。已收窄为
  「**本层没有 `pick_one` 这个口子**」，并把限度写成两条（名字是开放集、`trybuild` 枚举不完；
  这半边由「不内置来源、不提供默认实现」对冲）。
  **复评验到了底**：照收窄后的说法**再也构造不出不红的反例**，只剩一个「无 `self` 接收者的关联函数」
  ——而那**不是「来源上的口子」**，与收窄后的句子不冲突。**判据：改完要回头试着再构造一次反例，不能只看改了字。**
- **M-3-2「那条「红条件」实为编译失败」（Important，plan-mandated）** —— 简报给
  `a_score_is_optional_and_absent_is_not_zero` 的红条件是「把 `Option` 换成 `0.0`」，
  实测产出 `could not compile`，**按本仓纪律「变异导致编译失败不算红」**。
  ⇒ 该性质**只有编译期照片**，力量来自类型、不来自断言。**改的是计划那一格**（`1399a4f`），
  **没有为它发明运行期行为**（加一条「读回再判一次」的 helper 只会造出等价变异体）。
- **M-3-3／M-3-4（Minor）** —— 一条被前一句完全蕴含的断言已标明「不是独立断言」并改掉相抵的「红条件」；
  一处 `str::contains` 的子串检查已按实际射程改述，**并明写「本守卫自身没有照片」**（没把没照片的说成有照片）。
- **M-3-5／M-3-6（Minor，收尾时改）** —— ①「`continuum-model-registry` 没有一条可写成「编得过」的正面样例」
  **比实际宽**：可写的正面样例是有的、只是不必要（`TaskSkillRequirement::try_new` 公开可用，
  `tests/router.rs` 已在集成测试里用过）⇒ 改为「没有一条**需要**写成」；
  ②「`tests/persist.rs` 的**头注释**写明这是唯一通道」**位置指错**：那句在**中段一节的 `///`**（`:827-828`），
  该文件头是 `//!`、不讲这件事。
- **D-3-1「一处为假的跨 crate 全称」（Important，复评查出、本轮改出）** —— 新增的那节写
  「那两个 crate 的正面那半在 crate 外的运行期用例里就有照片」，**对 `continuum-model-registry` 为假**
  （`ModelProfile` 在 crate 外无构造入口：`try_new` 是 `pub(crate)`、字段全私有；需要画像的用例住在 crate 内）。
  已按 crate 分述、**不保留任何未核过的全称**。**判据：句子比材料宽是同一个毛病，材料换成别人的文件也一样。**

### 门读数与它的工件

`4d8ff95` 那一轮：`Running` **111**、`test result:` **130**（含 Doc-tests 19）、passed **834** / failed **0**、`warning` **0**。
**读数留有工件**：`.tmp/p4-t3-final-gate.log` 与 `.tmp/p4-t3-final-build.log`（两者 mtime 均晚于订正提交）。
**据实记两条限度**：① build 日志全文只有 `Finished \`dev\` profile … in 0.12s`——**无活可干，故它本身不构成证明**，
其射程来自 gate 日志里 111 条 `Running` 全部跑起来；② 更早那一轮（`3a5a69c`／`65ca48a` 之后）曾报过一组同样的读数，
**但盘上没有对应的日志**（`.tmp/p4-t3-gate.log` 的 mtime 早于那两个提交、且仍含旧样例名）——
**一条只写在报告里的读数，下一个人除了重跑无从核验**；**「读数可核」与「读数正确」是两件事**。

---

## Task 4（显式约束与「满足」判据、Reference Resolver 四态、两个数不同轴）

实现 `6a74078`（父 `c490263`）；**评审后订正 `e28dcfd`**（本轮，裁决一／二／四／五改代码与注释，
裁决三／六只在本节记账）。**复评判词 Approved。**

### 本 task 闭合的几条（供终审不必重做）

- **一、`ResolutionResult` 四支载荷字段转私有** —— 简报自相矛盾：Step 1 要求 crate 外逐支穷尽解构（要字段可命名），
  Step 3 的代码块要求字段私有（禁止它）。**裁定取「两条都满足」的第三条路**：字段私有 ＋ 三条字段清单用例搬进
  `src/resolver.rs` 的 `#[cfg(test)]`（**照片不丢**：字段增删仍撞 `E0027`／`E0063`）＋ **补十枚只读访问器**
  （**读的口子必须留在 crate 外**——计划的 Task 5 节逐字要求读 `Resolved.entity` / `candidate_count` /
  `Ambiguous.surviving` / `Unknown.reason`）＋ 新增 `compile_fail` 样例钉「crate 外构造不出」（**四支逐支各一段 `E0451`**）。
  **两半都钉住了**：把某字段改回 `pub` 会让新反例的 `.stderr` 不符；加字段会让解构红。
- **二、两处注释把照片指到了不存在的地方** —— `tests/explicit.rs` 原说「`exhaustive` 那个 `match` 穷尽」，
  而 `exhaustive` 里**没有 `match`**；真照片在 `src/alias.rs:96-101`。已改指，并把误名 `exhaustive` 收窄为 `mapped`。
- **三、`CanonicalRepresentation` 的接缝** —— 记 **P-4-1**（见上）：判为**正当偏离**（它是 Task 2 自己留的**推测性**承诺、
  设计没给字段清单、今天没有任何用例读它），但**它只留在一处文件头注释里会被读成「已经能用了」**——
  `Canonical` 支的载荷**说不出「这一枚 mention 解到了哪个实体」**。归属：**第一个真实消费者**（在 P4 之后）。
- **四、一句自指的可核事实写假了** —— `tests/resolver.rs` 原写「全文 `grep 'unwrap'` 零命中（这一句是本文件的可核事实）」，
  实测 6 处命中全在注释里、**包括它自己**。已改为「去掉注释之后零命中」并**不写数值**。
- **五、一条用例名偏宽** —— **用例名也是一种断言**（Task 3 上已按这条改过一次文件名）；已收窄。
- **六、本 task 自补的公开面无竞争** —— Task 6 的 Produces 是 `{ResolutionRisk, ExplicitBinding, BindingSource}` ＋
  `binding.apply`，`RequiredStrength` 出现在它的 **Consumes** 行；**已记入本节**，免得下一轮重问。

### 一处**前向**不一致（复评查出，本轮已在计划里补上）

订正后载荷字段私有，而**计划的 Task 5／Task 6 仍用「按名解构」写载荷断言**
（`Unknown { reason: AllCandidatesRejected }`），两 task 的用例都住在 `tests/`（crate 外）——
**那种 pattern 字段私有后在 crate 外写不出**。访问器已在故判据不丢，缺的是口径没写在那两节里。
**已在计划 Task 5 与 Task 6 各补一条**（写明「读走只读访问器，按名解构／结构体字面量在 crate 外写不出」），
判据同本计划对 `Remaining` 的既有处置（`:2135-2136`）。**否则按 task 切出的 brief 会把 pattern 原样带给实现者。**

### 本 task 自补的公开面（本轮新记，收件人：评审与 Task 6）

下面这几处是本 task 在简报逐字给出的 Produces **之外**自补的公开面，各自填一个规范空洞。
记在这里，免得下一轮读计划的人重新问一遍「这是谁定的、Task 6 会不会另立一个竞争的入口」。

- **`CandidateSet`**（`crates/continuum-canonical/src/explicit.rs`）—— 设计 §4.1.1 逐字给了
  `qualifying` 的签名，而里面的 `CandidateSet` 只出现为**类型名**，规范**没给它的定义**。
  本 task 取「候选与它的判定事实配对的值集合」（`&[(Candidate, CandidateFacts)]` 的口径）。
  **Task 6 消费而不另立**：Task 6 的 Produces 是
  `{ResolutionRisk, ExplicitBinding, BindingSource}` ＋ `binding::apply`（计划 `:887`）。
- **`ResolutionConfidence::new`**（`crates/continuum-canonical/src/resolver.rs`）与
  **`RequiredStrength::new`**（同文件）—— §4.4／§4.5 写死了「门槛必填、没有 `Default`、
  不能由分数构造、没有把置信变成数的读者」，而**两枚值的公开构造入口规范没给**。
  本 task 各给一个 `new(Opaque)`（里子是不透明值，本层不解释、不归一、不换算）。
  **Task 6 消费而不另立**：`RequiredStrength` 出现在 Task 5 与 Task 6 的 **Consumes** 行
  （计划 `:804` 与 `:886`）。
- **四个载荷的只读访问器**（`Resolved::entity` / `satisfied` / `candidate_count`、
  `ConfirmRequired::first` / `surviving` / `candidate_count`、`Ambiguous::surviving` /
  `candidate_count`、`Unknown::reason` / `candidate_count`，全在 `src/resolver.rs`）——
  **订正轮（裁决一）补的**：四个载荷的字段转私有、不给公开构造函数之后，crate 外**读不到**
  判定结果；而 Task 5 的四条断言（§4.2，写在 `tests/resolver.rs`，是 crate 外）逐条要读
  `Resolved.entity` / `Resolved.candidate_count` / `Ambiguous.surviving` / `Unknown.reason`
  （计划 Task 5 一节逐字点了它们）。故补这一组**只读**访问器——**读得出去、构造取不回来**；
  每一枚的读法都有照片（`tests/resolver.rs` 的 `each_of_the_four_states…` 与
  `unknown_reason…` 两条）。**Task 6 消费而不另立**：Task 6 不新增读载荷的入口。

### 仍未闭合（终审要看的）

- **P-4-1「`CanonicalRepresentation` 的 mention→实体 接缝：无消费者、无照片」** ——
  Task 2 在 `crates/continuum-canonical/src/canonical.rs:55-74` 留了一处**推测性**承诺：
  「Task 4 给四态补上载荷后，本类型应把那一份照录进来」。**Task 4 复判后不改**（三条理由写在原处）。
  它**今天只留在一处文件头注释里**，而这句话的读者会以为「这份规范化结果已经能用了」
  ——实际不是：`Canonical` 支的载荷**说不出「这一枚 mention 解到了哪个实体」**。
  **代价照实写**：`CanonicalRepresentation` 只是一个「哪些 mention 已经解出」的记录，
  **不是一个可直接使用的规范化结果**（要拿到映射就得改 `mentions()` 的形状，而换形状本身没有判据）。
  **归属：第一个真实消费者**——它在 P4 之后的 **Intent Compiler** 那一侧；
  补它的那个 task **应当带一条读它的用例**来（那时才有判据）。**当前没有任何 P4 task 会读它**：
  Task 5／6／7 的 Produces 里都没有「读 `CanonicalRepresentation` 的实体映射」这一项。

---

## Task 5（§4.2 的四条断言 A／B／C／D 与两侧守卫）

实现 `4d46778`；订正 `956bb67`；评审判 **Approved**（无 Critical／Important）。

- **计划排期错（已折回计划 `:799`／`:805`）**：计划把 `src/resolver.rs` 写成「续，**四条断言的实现体**」，
  而**实现体在 Task 4 就已落地**——本 task 改那个文件的**全是注释**（判定路径一字未改）
  ⇒ **Step 2「确认失败」空转**，七条新用例第一次跑就绿，**红证据全部来自变异轮**
  （八枚，逐枚真跑，**全为运行期红**：逐份日志 `grep -cE '^error\[E'` = 0）。
  **判据**：**一个 task 的 Files 若写「某某的实现体」，先核那个实现体是不是已经存在**——
  排错了会同时废掉「先跑红」这一步与该 task 的实付内容。（同族：Task 2→4 的次序相抵、Task 7 的 `alias.rs`。）
- **一处「今天没有红」的红条件**：简报的「把 A 实现成工作集（放宽档）」依赖 Task 6 的注入点；
  今天工作集 ≡ 检索集，故**它是等价变异体**。它换了等价真变异体（自造一枚塞进判定）并实测红。
  **判据**：**等价变异体要换真变异体，而不是补用例。**
- **一处「大概率红」**：D2 的 `HashSet` 顺序变异体 **4 跑 3 红**（两元素哈希序每次进程不同）。
  照实记「大概率红」、不补用例掩盖。评审核实：那条用例确实钉住**序列相等**（顺序承重），换确定性变异必红。

## Task 6（风险门槛 §276、`ExplicitBinding` 的最高优先序 §277、集合外注入 §4.6）

实现 `4f85a45`（门 114 `Running` ／ 133 `test result:`）；复评后订正 `e7547c8`；评审判 **Approved**。

- **那条前向风险解掉了**：注入形态**实测是「调用方传参」**（设计 §4.6 `:634-638` 的 `if let Some(b) = binding { … }`），
  故 Task 5 的 A 注入守卫**有效、未重写**，只给既有调用点**机械补了 `None`**（断言一字未改）。
  另一支（由 `BoundTo` 约束自动注入）会让那条守卫**把正确实现判红**——**该分支未取**。
- **一条要留档的残余**：`crates/continuum-canonical/src/candidate.rs:170` 的「解析器（Task 4/5）还没建」**今日亦假**，
  但那是**别的 task 的工件、非本提交引入**，订正轮未动它。
  **收件人：下一个碰那个文件的 task**（本台账按体例记下，不让它静默留在那里）。
- **同族的一处订正**：`tests/resolver.rs` 里「注入点是 Task 6／今天没有第二个来源」那一类**实为七处**
  （订正时点名的四处之外另有三处同类）；其中一处原文把「分岔条件」写成「注入点落地」——
  **注入点落地本身不使两者分岔**，是「某条用例真传一枚注入」才分岔。

---

## Task 7（alias 表（迁移 100）、四作用域与显式提升）

实现 `e2b11d7`（门 115 `Running` ／ 134 `test result:` ／ 876 passed ／ 0 failed ／ 0 warning）；评审判 **Approved**。

- **P-7-1（已核实的无照片，据实记）**：设计 §4.6 第 2 条那句「**没有任何函数会顺带调用 `promote_alias`**」
  **只被一半照片支撑**——设计要求的 (a)(b) 两张都拍了（三条用例逐条对上），
  但**整句话**还差一个**扫 `src/` 的源码扫描**（照 `continuum-capability/tests/single_writer.rs` 的体例）。
  **处置**：实现者**没有把这句绝对措辞写进任何注释**，缺口写在 `crates/continuum-canonical/src/persist.rs:75-86`
  与 `crates/continuum-canonical/tests/alias.rs:29-35`。**收件人：设计维护者（补扫描，或把那句收窄）。**
  > **判据**：**「不写」也是一种处置，但要写在能被 grep 到的地方**——本台账的执行判据正是「在版控里 `grep` 得到吗」；
  > 该缺口在两个被跟踪文件的注释里，故**已满足**。本条补的只是把它**再登记一次在这个按条可查的位置**。
- **本 task 的落点决定（申报）**：`Alias { alias, target, scope }` 与那三列（`alias` 为主键）——
  §278 只给四个作用域名、设计 §4.6 只给「一张表 ＋ `scope` 封闭枚举」，**字段与列都没给**。
  `target` 取 `EntityId`（与 `Candidate`/`Resolved` 同类型）的理由写在 `src/alias.rs`：
  免得「别名指向什么」与「候选集里装什么」成为两个各有一个串的类型。
- **一处偏离（已写明）**：四条 I/O（`save_alias`／`load_alias`／`list_aliases`／`promote_alias`）同落在 `src/persist.rs`
  （计划的文件结构原把 `promote_alias` 列在 `alias.rs`）——**裁定：以代码为准，计划那一行已改**（`611c251`）。
  理由：四条都要 `Tx`，而 `alias.rs` 是不碰持久化的类型与映射那一层。
- **一处「本 task 的改动把别处的句子变假」**：本轮加的 `continuum-canonical ← continuum-persist` 那条边，
  把 `src/candidate.rs` 与 `tests/candidate.rs` 里「本 crate 的 `ALLOWED` 是**空条目**」改假了（行号由 `:156` 漂到 `:162`）。
  实现在 Files 之外改了那两处注释（**只改注释**），按体例留了旧说法。**裁定：正当**——
  **那句话在自己 crate 的文件里，而本 task 的改动使它为假。**

---

## 设计面未闭合（计划 `## 遗留` 第一节，`:2476-2513`；终审 I-2／I-3）

**口径**：正文**见计划原文**（重述即第二份转录）；本节只留条目名、行号与收件人，供「一处说法可查」。
计划的「已闭合的八条」（`:2448-2474`）**不并入本台账**——设计已闭合，留格只为订正可查。

- **①`Remaining` 的 crate 归属相抵**（计划 `:2478-2487`）——**收件人：设计作者＋协调者**。
- **②§12.2 的 `I2` 与 ENG-005 原句的措辞差别**（计划 `:2488-2492`）——**收件人：协调者**。
- **③§12.1 的私有构造与 §12.6.1 的按名解构相抵**（计划 `:2493-2498`）——**收件人：设计作者＋复审者**。
- **④`let Remaining(dims) = r;` 的作用域未写明**（计划 `:2499-2504`）——**收件人：设计作者**。
- **⑤§16 第 34 条的「编译器保证」在本仓验证命令下不成立**（计划 `:2505-2513`）——**收件人：设计作者＋复审者**。

**I-2（终审收口）「设计 §5.4 的六项 → 实现五项」**：设计 §5.4 的表写「本层写六项」（设计 `:904-915`，
其中第六项是 `EventType::PlanReviewRequired`），而实现落了**五项**（`IntentCreated`／`IntentCompleted`／
`DecisionRequired` ＋ `ContractChanges`／`UserApprovals`——清单与来历见
`crates/continuum-semantics/src/lib.rs` 模块文档的「尚未做的」一节）。**第六项全仓零生产方**：
`PlanReviewRequired` 的命中只在 `continuum-events` 的变体／编码表与上面那节文档注释里
——**无归属 task、落库写路径不存在**。**收件人：协调者（建后续 task 时）＋设计维护者**
（设计要么给它一个 task，要么把「六项」的措辞与实现对齐）。**不返工代码。**
同处一并订正：设计 §5.4 表内引用的 **`§10.3` 是悬空引用**（设计无此节；`:915`，其语境是
`DecisionResult` 落库）——交**设计维护者**。

**I-3（终审收口）「设计 §4.6 的正文与它自己的照片 (a) 相抵」**：`:779-781` 把「最高优先级」写成
「先把候选集**缩到 `entity` 这一个**」，而照片 (a)（`:801-803`）要求「`entity` 不在检索集里」时
**追加**（`candidate_count` 比检索集**多 1**）——「缩」不能外推到那一支。**实现按照片取：情形 2 缩、
情形 3 追加**，三情形的完整论证写在 `crates/continuum-canonical/src/binding.rs:120-189`
（该处注释已把这一条指向本台账）。**收件人：设计维护者／规范维护者**（正文那一句要么限定在
「已在检索集里」的情形 2，要么照实现改述）。

**终审搁置的相邻三条**（理由见终审报告 Declined，`.superpowers/sdd/p4-final-review.md`，**gitignored**）：
TURN→conversation 的放宽方向与重判条件（**收件人：设计维护者**）、BudgetOwner「五个 owner 变体」
标题与正文四枚相抵（设计层计数措辞）、会话边界清退（§4.6 第 3 条已明写未实现）。

---

## 跨计划请求（计划 `## 遗留` 第二节，`:2515-2526`）

**本计划一处都不实施**；正文见计划原文，三处只点名：

- **`adfir_graph` 加 `contract_version`**（新增迁移 21、`DEFAULT 0` 哨兵、不改迁移 20 的 SQL；
  本层是消费方、不读该列）——**收件人：执行层（P1）＋协调者**。
- **`ContractIdRef` 与 `ContractId` 的合一与否**（生产点三处；「已有库不受影响」不成立；
  本层只定义 `ContractId`）——**收件人：执行层（P1）＋协调者**。
- **`ExecutionProfile.cost_budget` 收成 `Allocation`**（对 ENG-005 §五 的一处改判；类型本层交、
  收紧动作归执行层的接线 task）——**收件人：执行层的接线 task＋协调者**。

**终审亦搁置**（判给执行层＋协调者；理由见终审报告 Declined 1）。

---

## 照片缺口（计划 `## 遗留` 第三节，`:2528-2536`）

- **设计 §15.2 已列十条**：逐条见设计原文——本台账**不重述**。
- **本计划补三条**（`:2530-2536`）：⑪`Remaining` 五维 `None` 的**消费端**（收件人：驱动的投影
  实现与子项目 G）；⑫`Mention.kind` 的**取值域**（收件人：规范维护者）；⑬三个 crate 对下层的
  **零依赖**（传递可达不可拍；**不需要那条断言**——设计 §1.2.2 规则 1 末段）。
- **终审亦搁置**：Declined 3（§15.2 十条——设计已列）、Declined 11（`Remaining` 消费端＝上⑪）、
  Declined 13（`resolution_confidence` 校准——§15.2 已登记）。

---

## 申报接受（计划 `## 遗留` 第四节，`:2538-2566`）

**八条申报**（正文见计划原文，只点名）：迁移号 100／110／120 取用前现场核对空号；迁移 110 的
SQL 逐表追加（Task 20 之后不得再改）；`dangling_reservations` 的取名与落点；`PlanChange` 分类的
阈值入参；三个角色的公开读法 `dims()`；「四个入口 × 负值」两侧各补；`allocate` 的命名（已改名
对齐）；`#![deny(clippy::rest_pat_in_fully_bound_structs)]` 写进测试文件头（**今天不生效**）。同节
另有一条**已不再是「自定」**的留格可查：`ResolutionResult` 的载荷形状与 `resolve` 不含 `Result`
（设计 §4.1 已采纳——留格只为「曾由计划先取」可查）。

**其中两条带未闭合收件人、单列**：⑧生产投影仍缺席（**收件人：接线的那个 task**）；
⑨clippy 未入门禁（**收件人：设计作者＋复审者**）——见下节。

---

## 未闭合项

- **⑧生产投影仍缺席**：形状乙裁定把 `Remaining → BudgetView` 的**生产投影**判给驱动，而驱动今天
  没有装配那一步；本 plan 只交付 `crates/continuum-runtime/tests/budget_projection.rs` 的测试内投影
  与两条断言。**收件人：接线的那个 task**（计划 `:2563-2566`）。
- **⑨clippy 未入门禁**：那道 `#![deny(clippy::rest_pat_in_fully_bound_structs)]` 已写进测试文件头，
  而本仓验证命令（`cargo test` ＋ `cargo build`）不含 clippy，故**今天不生效**；要把它变成真保证
  缺「把 clippy 纳入门禁」这一步（仓库级决定）。**收件人：设计作者＋复审者**（计划 `:2560-2562`）。
- **分配额不可修改**：设计沉默，等第一个消费者带判据（终审亦搁置；理由见终审报告 Declined 7）。
- **Task 21 自查**：**无新增未闭合项**（该复核的五条设计面问题已按交接清单归位；自查记录在
  `.superpowers/sdd/p4-task-21-report.md` 的「自查发现」，该报告 **gitignored**，此处留格）。

---

## 设计 §16 的 34 条：收件人归类（指引）

34 条的**内容**不重述（指向设计 §16 原文）；**收件人归类**的全文见计划 `:2568-2599`（逐条点名与
五组归类，含第 24／25 条＝上「跨计划请求」三处、第 31 条 `Decision` 同名等）。

**终审亦搁置**：Declined 2（34 条整体——收件人多为本层之外的角色／后续层）、Declined 9（第 31 条）、
Declined 12（不透明串取值域——§16 第 3／8 条已登记）。理由见终审报告。

---

## 终审的两处 Ruling 结论（照终审报告原文）

- **Task 10 §6.2 留口 → 不收窄**：§223 的流程顺序要求**合法无 spec 态**；§6.2 管的是**配对写**的
  原子性。实现维持现状（与 `tests/spec.rs:316-320` 的注释一并看——终审另建议把那里改述为阶段语）。
- **Task 18 变异留档缺 sha256 → 接受协调者裁定，不补跑**：红位**非等价**（把一处显式拒绝改成
  静默默认值）＋变异后全量门全绿闭环，证据链够用。

---

## 终审「带文档」条目（51 条 triage 的归位）

**口径**：终审报告里判「带文档」的条目在此逐条落格（一句话）；判「顺手修」的见修复提交；
判「丢弃／已闭合」的见本文件末尾附录。

- **Task 9**：「31 次起库」是**已知的测试成本**（每用例建库，不为速度改夹具形状）；「30 条非条件对
  仅 Task 侧照片」——**建议在对应断言处补注释**（限度照实记）。
- **Task 10**：`tests/spec.rs:316-320` 的「v2 无 spec」**建议改注释为阶段语**（「今天的合法态」
  不是「永远没有」），**与上节 §6.2 Ruling 一并看**。
- **Task 11**：(c) 次序半句——**收窄建议**（照表录收正确）；`a_missing_contract_reads_back_as_none`
  **无变异**——证据限度照实记；`m.contains("contract")` 是**弱断言**——力量来自别处，不再加码。
- **Task 12**：重复 id 是那条用例的**前提**——**建议补一句前提注释**。
- **Task 13**：transition 两对读法——**销账**（照表录收正确）；`reaches_ready` 的**终态语义限度
  已知**（不再追）。
- **Task 14**：`apply_plan_change` 的**接缝收件人**已记（见计划原文）。
- **Task 16**：`execution_policy` 词表的 **fail-open 后果**（两张清单之外的条目本层不拦）——
  **收件人：规范维护者**（见本文件末尾「自定判据」节）。
- **Task 18**：`check_exploration` 的**负值**——**终审裁决：维持不收窄**（设计定格签名；负值不由
  它拒，门控偏松的后果照实记）——**收件人：规范维护者**。
- **Task 19**：断言三条红条件**只有一半实测**——同族限度照实记（不为它造用例）。

---

## 附录：终审「丢弃／已闭合」的条目（列名，不展开理由）

- **丢弃**（均为报告层／已申报项，无代码动作）：Task 8（§8.4 括注不一致、§5 旧变异表）、
  Task 9（`EventCodecChain` 零消费者）、Task 10（报告 §5.4 措辞、`SpecError::Intent` 两层包装、
  七个 newtype 样板、`decode_unknowns` 的 none_found 不读内容）、Task 11（出现点清单排版耦合、
  `occurred_at` 偏离）、Task 13（COMMAND-EXIT 字面）、Task 14（MINOR 五行同构、报告引用小疵、
  编码清单无表）、Task 15（失败点 1 叙述）、Task 16（「裁决五条」第 5 条、`ConstraintError` 落点
  张力）、Task 17（`record_root` 无全零侧、BudgetView 行号引用）、Task 19（等价变异措辞张力）、
  Task 21（行号误引、M3a 缺档位词）。
- **已闭合**：Task 13 的「plan 表表外取值」（Task 15 已做）、Task 16 的 `error.rs`「前五枚」
  （当前文本正确）、Task 17 的「构建门日志弱」（Task 21 已补）。
- **搁置后无动作**：Declined 15「`apply_result` 重复调用」——**不可达**（主键挡第二次），
  无静默路径。

---

## 本层已自定的判据（发明判据，收件人：规范维护者）

设计文档里有一条统一落点：`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md` §16
（遗留与未决项），**发明判据必须显式标出、写明代价与边界、具名收件人**。已登记的：

- **§16 第 5b 条**：§196 的十类输入没有任何一类有词表 ⇒ 本层自定 `PRONOUNS` 18 条与 `CONTEXT_REFERENCES` 12 条。
- **§16 第 5c 条**：时态解析的三处自定判据（`RELATIVE_DAYS` 18 条含 7 条英文、恒定 86400000 毫秒/日的日历假设、
  `TemporalBinding` 的两支形状）。**无照片处照实记**：英文半边删掉后全量门 0 红——该处已在 `1185ed9` 补上照片。
- **终审再核过两条**：**§196 词表自定判据**（终审亦搁置——§16 第 5b／5c 条已登记其明细；
  理由见终审报告 Declined 4）；**`execution_policy` 的词表规范缺口**（§225 两张清单之外的条目
  本层不拦＝fail-open 的后果——**收件人：规范维护者**，终审亦搁置，理由见终审报告 Declined 14，
  本地后果见「终审『带文档』条目」的 Task 16 一条）。
