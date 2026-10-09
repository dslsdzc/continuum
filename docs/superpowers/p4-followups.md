# P4（语义层）残余台账

**本文件的执行判据是「在版控里 `grep` 得到吗」**，不是「我写过没有」——本仓吃过一次亏：
一张 gitignored 的台账（`.superpowers/sdd/p3f-minors.md`）里早有那条阻断项，而终审时
`grep -rn 'M-6-1' docs/` **零命中**，等于没人读过它。

编号约定：`M-<task>-<序号>`（Minor）；`D-<task>-<序号>`（错或缺陷）；
`P-<task>-<序号>`（**已核实的无照片**，只许记账、不许谎称有照片）。

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
裁决三／六只在本节记账）。

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

## 本层已自定的判据（发明判据，收件人：规范维护者）

设计文档里有一条统一落点：`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md` §16
（遗留与未决项），**发明判据必须显式标出、写明代价与边界、具名收件人**。已登记的：

- **§16 第 5b 条**：§196 的十类输入没有任何一类有词表 ⇒ 本层自定 `PRONOUNS` 18 条与 `CONTEXT_REFERENCES` 12 条。
- **§16 第 5c 条**：时态解析的三处自定判据（`RELATIVE_DAYS` 18 条含 7 条英文、恒定 86400000 毫秒/日的日历假设、
  `TemporalBinding` 的两支形状）。**无照片处照实记**：英文半边删掉后全量门 0 红——该处已在 `1185ed9` 补上照片。
