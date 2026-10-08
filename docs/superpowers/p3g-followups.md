# P3 子项目 G：实现期的遗留台账（**本文件是版控里的正本**）

本文件是 G（模型调用路径）**实现期**逐 task 评审攒下的 Minor 与未结项的**正本**。
体例照 `p3e-followups.md`（E）与 `p3f-followups.md`（F）。

**为什么现在就有这一本**：G 还剩 5 个 task（8–12），本文件**随各 task 的评审逐条追加**。
**之所以不在收口时才建**，是因为 F 那边付过代价：它的遗留台账当时只活在 gitignored 的
`.superpowers/sdd/p3f-minors.md` 里，**全分支终审 `grep docs/` 零命中、读不到，据此判了一条阻断项**。
**判据**：**一张 gitignored 台账的全部价值，等于它被搬进版控的那一刻**；
**收口时的执行判据不是「我写过没有」，而是「在版控里 `grep` 得到吗」。**

---

## 一、未结项

| # | 位置 | 内容 | 处置 |
|---|---|---|---|
| **M-7-1** | `p3g-task-7-review.md` §5 | 报告 §5 表里「红落在哪一行」**整体偏小 9 行**——它取自 `.tmp/t7-impl-tests-model_call.rs`（**1453 行**，11:19 前那份），而提交版是 **1462 行**，两者**只差文件头的模块文档（＋9）**。**红集与「红在哪条断言」不受影响**（评审用实测行号逐条复现一致）；报告 §1 的落点行号是对的（176/211/226/239/251/297 实测全中），**只有 §5 那批是手数的** | **订正该表并把来历留在原地**（手数的行号是本仓点过名的形状） |
| **M-8-1** ✅ | Task 8 的 probe 函数 | **`call_stream` 的第三枚 future 不在那条 `assert_send` 用例里**（Task 9 才落地）。今天断言的是 `select` 与 `call` 两枚 | **已闭合（2026-10-08）**：派单侧——Task 9 的 `Files:` 与 `git add` **都已加上 `tests/model_call_face.rs`**，并在计划里点明交接的两处所在（该文件头第四节 ＋ 本表这条）。**交付侧（Task 9 已办）**：第三枚 future 已**加进同一个 `probe_the_async_segments_are_send`**（加一行，未另立用例），该文件头第四节同步改写为「三枚都在」并留下原句的来历。**实测该行是承重的**：给 `call_stream` 加一枚跨 `await` 活着的 `&Tx<'_>` → `model_call_face.rs` 的 `assert_send` 处报 `E0277`（`RefCell<…> cannot be shared between threads safely`）。**评审当时的判据照留**：**跨 task 的交接若只写在被交出去那个文件里，派单时就会丢——派 Task 9 的 brief 是按那一节切的，切不到别的文件里** |
| **M-8-2** ✅ | Task 8 的变异红集 | **射程偏离「逐枚在全量门下」**：14 枚里只有 `M1` 跑了全量门，其余跑受影响 crate 的完整套件。实现者自陈「残余风险没被证伪、只被压低」 | **评审裁定：射程够，不必逐枚补跑**——**理由比实现者给的更强、且是构造性的**：**整个 workspace 没有任何 crate 依赖 `continuum-runtime`（实测零命中）**，根目录也无 `[[test]]`／根级 `tests/`。**故 M1–M10 不可能改变别的 crate 的测试输出**，`M1` 与 `M1-full` 红集相同与这条一致 ⇒ **实现者自陈的残余风险被证伪**。（评审另实测：跑的是 **14 个目标 ＋ 1 Doc-tests**，报告与台账原写「12 个」偏低） |
| **M-8-3** ✅ | `the_deadline_wraps_one_call_only` | 那枚变异体**在交付签名上无处安放** | **档位标签改准**（评审实测）：**不是层②**，而是「**只能在改签名的前提下表达**」——最直白的一次性编码（`&mut Option<Duration>` ＋ `.take()`）**src 编得过、tests 五处 `E0308` 编不过**，故**唯一可观察者是调用侧编译失败，而按纪律那不算红**。**该用例今天钉的是「两次调用各成功 ＋ 适配器被调两次」，是回归护栏不是判别式。** **另note**：**计划 Task 12 Step 4 的证据表把「只包住一次调用」整格记到它头上（计划那一处），而那格的强度低于它读起来的样子**——派 Task 12 时要先想清楚 |
| **M-8-4** ✅ | `select` 的签名 | **界不是最小的**：交付与设计都写 `&(dyn RankingPolicy + Send + Sync)`，而评审实测 **`&(dyn RankingPolicy + Sync)` 就够**（整 crate 14 个目标编过；`+ Send` 单独不够、且多余） | **已收口（2026-10-08，Task 9）**：签名收窄为 `+ Sync`（`src/model_call.rs` 的 `select`），**设计 §3.1 的代码块同步订正**，两处调用侧（`tests/model_call.rs` 的 `a_plan_over`、`tests/model_call_face.rs` 的 probe）一并收窄。**`+ Send` 单独不够这一半由 Task 9 实测复核**：三处一起换成 `+ Send` → `E0277: dyn RankingPolicy + Send cannot be shared between threads safely`（落点 `model_call_face.rs` 的 `assert_send`） |
| **M-8-5** ✅ | 设计提交 `476368a` | **它自引的 `:368`/`:905` 因它自己在上方插了 10 行而漂到 `:378`/`:915`**（实测）——**与它正在修的托管订正 #1 是同一失效模式**（改一个文件，引它行号的另一处就漂） | **已收口（2026-10-08，Task 9）**：那两处改成**按内容引**（「§3.4 末段那句 `fn assert_send<T: Send>(_: &T) {}`」与「§11 表里『`Tx` 不跨 `await`』那一行」），并写明**本设计内不再写自引行号** |

| **M-9-1** | 计划 Task 9 的 `dropping_a_stream_does_not_cancel` 红条件（计划 `:1189`，实测） | **它写的是「在 `ModelStream` 的 `Drop` 里调 `cancel` → 红」，而那枚变异体在本 crate 里写不出来**：`ModelStream` 是 C 的类型，`impl Drop for ModelStream` 实测报 `E0117`（孤儿规则）＋ `E0120`（`Drop` 只能对本地类型实现）；**绕开它要 G 包一层自己的 `Stream` 实现，而 `Stream` 是 `futures_core` 的 trait、在本 crate 里只挂 dev 依赖**（lib 里实现不了，而本 task 不动 `Cargo.toml`） | **用例本身照留**（它是那张否定式照片的落点），**实测取它的可写邻形**：「在 `call_stream` 里建立流之后顺手取消一次」→ 红四条（该条 ＋ 两条 `abort` 计数条 ＋ `a_stream_call_...` 的 `stream_calls() == 1`）。**计划那句的收件人：协调者**——它与 M-8-3 同族（红条件写在计划里、而那一枚写不出来），**处置照 M-8-3：只在本表记实测，不代改计划** |
| **M-9-2** | `a_stream_call_hands_the_candidate_s_model_id_to_the_adapter` 的红条件 | 它写的「把 `adapter.stream(..)` 换成 `adapter.invoke(..)`」**也写不出来**：两者返回类型不同（`InvokeResponse` / `ModelStream`），要成型得把响应包成一枚 `Stream`，**而那要 `futures-core` 进 lib 依赖** | **实测取可写邻形**：多插一次 `adapter.invoke(..)`（不动返回类型）→ **只红该条**，落点 `invoke_calls() == 0`。**这与 M-8-3 的第三类相邻但不同**：不是「改签名才能表达」，是「**要动本 crate 的依赖表才能表达**」 |
| **M-9-3** | `aborting_a_finished_stream_is_not_an_error` 的定位 | 它的红条件（「对已完成的 `CallId` 返 `Err`」）**只能写成「`Ok` 路径上返 `Err`」**——G 手里没有任何「这条流已完成」的读数（`ModelStream` 只有 `call` 与 `chunks`，而 `chunks` 要 `Pin<&mut>` 才推得动，`abort` 收的是 `&ModelStream`；`cancel` 的返回值也不带状态） | **据实记**：它是「G 不在 `Ok` 路径上自行合成 `Err`」的**回归护栏**，不是能把「已完成／未完成」分开的判别式。实测红两条（该条 ＋ `aborting_a_stream_calls_cancel_with_the_streams_own_call_id`）。**不是空转用例**（那一枚挡得住） |
| **M-9-4** | `into_call_error` 的「三处共用」 | **行为照片三处都有了**（`call` / `call_stream` / `abort` 各一条，Task 9 补齐后两条），而「三处**都**不再自己映射一遍」这句话要一条**源码文本**判据才钉得住（`ModelCallError::Provider` 在本 crate 的 `src/` 里只有一处构造点） | **仍缺，收件人未指派**（`tests/model_call_discipline.rs` 的三条守卫不覆盖它）。**据实记为具名缺口**——`into_call_error` 的文档里已写明这一条 |
| **M-9-5** | Task 9 的一处自审 | 交付时写下的 `call_stream` 文档把失败面照片指到 `abort` 那条用例上，**而那一条走的是别的入口**——`call_stream` 的失败路径当时**一条照片都没有**（`StreamOutcome::Fail` 这一可配面也没被任何用例用上） | **已闭合（2026-10-08，同 task 内）**：补 `each_provider_failure_on_a_stream_call_keeps_its_class`（五枚变体逐项），实测红集只有它一枚。**它不在简报列的六条用例内，是本 task 加写的第七条**——理由与来历写在用例文档与 task-9 报告里 |

| **M-9-6** | `rustfmt` | **本仓不是 fmt 干净的、且无配置无 CI**（`p0-followups.md` 已记）。**实测**（`rustfmt --edition 2021 --check`）：`tests/model_call.rs` 在 Task 9 之前已有 **23** 处、之后 **32** 处（新增 9 处，与既有 23 处**同形**——那座文件的 `assert_eq!(class, FailureClass::X, "…")` 单行写法）；`tests/model_call_face.rs` 由 0 变 **1**（Task 9 加宽了那行 `use`）；`src/model_call.rs` 两版都是 **0** | **不整文件重排**：那样会顺手改掉 Task 8 的 22 处（不是本 task 的行），而 fmt 不是本仓的门。**中途一度整文件重排过、随后撤回**，撤回后工作树与提交逐字节相同。**判据（量法本身的一处坑）**：**带 `mod` 声明的文件不能挪出它所在目录去测**——把旧版复制到 `.tmp/` 再跑 `rustfmt --check` 会因解析不到 `mod common;` 而报「0 处」，**那是测具的假象** |

## 二、据实记为无照片（本轮不补）

| # | 属性 | 现状 |
|---|---|---|
| G-∅-1 | `adapter_for` 的 `expect` | **构造性不可达、无照片**（Task 7 评审记） |
| G-∅-2 | 空候选集下「`rank` 一次都没被调用」 | **在 Task 6 上类型上做不到**（`plan_candidates` 的参数表没有 `&dyn RankingPolicy`）。**协调者已裁定落到 Task 11**（标题正是「可达性与射程边界」，硬依赖 Task 7），**Task 7 不做**（`select` 永远收不到空 `Vec`） |
| G-∅-3 | Task 2／3／6 的隔离复跑 | Task 7 评审**走的是全量门 ＋ 逐行 diff**，未逐枚做隔离复跑 |
| G-∅-4 | M5 的写法与实现者不同 | sha 不同、语义同，**未做交叉验证** |

## 三、合入时的两笔（与 F 的那本同源，记此免得漏）

1. **并集收口**：`Cargo.toml` 的 `members` 与 `crates/continuum-runtime/tests/dependency_direction.rs`
   的 `ALLOWED` 由 **E / F / G / P4 共写**，**「合并后的并集对不对」没有任何 task 认领**。
   详见 `p3bcdf-followups.md` 的 `## 遗留` 第 1 条与 `p3f-followups.md` 的 §三。
2. **`async-trait` 的重复边**：这条 **dev 边被加过两次**——F 的 Task 3 在 `p3f` 上、G 的 Task 2 在 `p3g` 上。
   合入时**同一张表会落成重复键或文本冲突**，按「**同一条边、一个键**」收口。
