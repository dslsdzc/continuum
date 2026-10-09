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

---

## 本层已自定的判据（发明判据，收件人：规范维护者）

设计文档里有一条统一落点：`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md` §16
（遗留与未决项），**发明判据必须显式标出、写明代价与边界、具名收件人**。已登记的：

- **§16 第 5b 条**：§196 的十类输入没有任何一类有词表 ⇒ 本层自定 `PRONOUNS` 18 条与 `CONTEXT_REFERENCES` 12 条。
- **§16 第 5c 条**：时态解析的三处自定判据（`RELATIVE_DAYS` 18 条含 7 条英文、恒定 86400000 毫秒/日的日历假设、
  `TemporalBinding` 的两支形状）。**无照片处照实记**：英文半边删掉后全量门 0 红——该处已在 `1185ed9` 补上照片。
