//! Task 4 的四态用例（§4.1）与「两个数不同轴」（§4.4）。
//!
//! 每条用例的头注释写明**它钉的是什么**与**红条件是什么**；红条件在本 task 的变异轮里
//! 逐条真的取过（真写变异体、真跑到红），取到的结果记在
//! `.superpowers/sdd/p4-task-4-report.md`。
//!
//! # 这些用例钉的是**分支机制**，不是解析质量
//!
//! 真实候选来源今天**零个**（设计 §16 第 5 条），`ExplicitBinding` 的注入点还没建
//! （Task 6），§4.2 的四条断言由 Task 5 钉。故下面的候选集是**夹具**给的，
//! 本文件钉的是「四态各自在什么条件下产出」这件事本身
//! （与设计 §15.2 第 3 条同一体：能拍的只有机制）。
//!
//! # 为什么没有一条用例写 `unwrap()`
//!
//! [`resolve`] 的返回类型是 [`ResolutionResult`]，**不是 `Result`**（§4.1 写死）——
//! §206 的 `abstain` 已把「没有可靠解析」收进 `Unknown`，而本层没有第二种失败。
//! 这份文件里每一个 `resolve(...)` 的结果都**直接**被断言（`let r: ResolutionResult = …`
//! 或 `match`），故若某天给它套上 `Result`，本文件**全部编不过**
//! （那是编译期照片；本仓纪律：编译不过不算红——本条要求的可观察者是「没有一条写 `unwrap`」，
//! 而它由这份文件的源码形态兑现）。

use continuum_canonical::{
    Ambiguous, Candidate, CandidateFacts, ConfirmRequired, EntityId, EntityKind,
    ExplicitConstraint, Opaque, RequiredStrength, ResolutionConfidence, ResolutionResult,
    ResolutionScope, Resolved, Unknown, UnknownReason, resolve,
};

/// 夹具：一枚候选。
fn cand(id: &str, score: Option<f64>) -> Candidate {
    Candidate {
        entity_id: EntityId::from_str(id),
        evidence: Vec::new(),
        retrieval_score: score,
    }
}

/// 夹具：一枚候选的判定事实。
fn facts(scope: ResolutionScope, kind: EntityKind) -> CandidateFacts {
    CandidateFacts { scope, kind }
}

/// 夹具：一枚「已达」的置信值（§275；本层不解释它的取值域，见 `src/resolver.rs`）。
fn conf(level: &str) -> ResolutionConfidence {
    ResolutionConfidence::new(Opaque::from_str(level))
}

/// 夹具：一枚「要求」的强度（§276 的 `required_resolution_strength`）。
fn req(level: &str) -> RequiredStrength {
    RequiredStrength::new(Opaque::from_str(level))
}

/// 把一条约束摊成一个串——**不带 `..` 的穷尽解构**。
///
/// 用途：`Resolved.satisfied` 是「照录判定时用的那一组约束」，而
/// `ExplicitConstraint` **没有** `PartialEq`（`BoundTo` 里是 `EntityId`，
/// 而它没有相等判据——见 `src/explicit.rs`），故这里逐枚摊开再比。
/// 顺带它自己也是一处「三枚穷尽、加一枚即编译不过」的解构。
fn describe(k: &ExplicitConstraint) -> String {
    match k {
        ExplicitConstraint::Scope(s) => format!("scope:{s:?}"),
        ExplicitConstraint::EntityKind(t) => format!("kind:{t:?}"),
        ExplicitConstraint::BoundTo(e) => format!("bound_to:{}", e.as_str()),
    }
}

/// 把一批候选的实体串按**输入次序**取出来。
fn ids(cs: &[Candidate]) -> Vec<&str> {
    cs.iter().map(|c| c.entity_id.as_str()).collect()
}

/// **四态各一条**，且每条都**断言是哪一枚**（不是 `is_ok`、不是 `!= Unknown`）。
///
/// | 态 | 本条的夹具 |
/// |---|---|
/// | [`Resolved`] | 两个候选里**恰一个**合格，且门槛已过 |
/// | [`ConfirmRequired`] | 同一份输入，**门槛未过** |
/// | [`Ambiguous`] | **两个**候选都合格 |
/// | [`Unknown`] | **零个**（候选集为空） |
///
/// # 四格为什么写在一条用例里（以及它的代价）
///
/// 简报把这一条点成一条用例（「四态各一条」）——四格是**同一个判定的四个出口**，
/// 拆成四条会让「四态的定义是同一张表」这件事在文件结构上看不出来。
/// **代价照实写**：`#[test]` 是顺序执行的，任一格 `match` 落空即 panic、
/// 后面的格不再跑，全量门只能给出「这条红了」、给不出「哪一格落空」。
/// 故本条在每一格的 `match` 里都写了**带格名的 panic 消息**
/// （「…应落 `ConfirmRequired`，实得 …」），红输出里因此看得到是哪一格。
/// **它钉不住「四格都跑到」**——那件事在本仓是 Task 1/2 用「一条臂一个 `#[test]`」
/// 换来的；本条按简报被点成**一条**用例（四格是同一个判定的四个出口），
/// 故这一处代价照实记在这里，不假装已消除。
///
/// # 红条件（**移除档**）
///
/// 把 `ConfirmRequired` 与 `Ambiguous` 合并成一支（例如让「唯一合格但门槛未过」也返回
/// `Ambiguous`）⟹ **只有 `ConfirmRequired` 那一格红**，其余三格不受影响——
/// 这正是「两支的判据不同源」那条边界要的可观察后果：
/// **门槛唯一能改变结果的地方就是这一格**（「多个合格」那一格的判据与门槛无关）。
/// 实测见报告 §变异轮。
#[test]
fn each_of_the_four_states_has_a_photo_and_the_state_is_asserted() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let met = req("high");
    let unmet = req("unreachable");

    // —— 一：唯一合格 ＋ 门槛已过 ⟹ `Resolved` ——
    let one_qualified = vec![(
        cand("e-only", Some(0.9)),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];
    let resolved_case = match resolve(&one_qualified, &k, &conf("high"), &met) {
        ResolutionResult::Resolved(r) => r,
        other => panic!("唯一合格且门槛已过应落 `Resolved`，实得 {other:?}"),
    };

    // —— 二：同一份输入、门槛未过 ⟹ `ConfirmRequired` ——
    let confirm_case = match resolve(&one_qualified, &k, &conf("high"), &unmet) {
        ResolutionResult::ConfirmRequired(r) => r,
        other => panic!("占优候选存在但门槛未过应落 `ConfirmRequired`，实得 {other:?}"),
    };

    // —— 三：两个都合格 ⟹ `Ambiguous` ——
    let two_qualified = vec![
        (
            cand("e-a", Some(0.9)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-b", Some(0.5)),
            facts(ResolutionScope::Project, EntityKind::Skill),
        ),
    ];
    let ambiguous_case = match resolve(&two_qualified, &k, &conf("high"), &met) {
        ResolutionResult::Ambiguous(r) => r,
        other => panic!("两个合格候选应落 `Ambiguous`，实得 {other:?}"),
    };
    // **门槛在这一格不可观察**：换成「门槛未过」仍是 `Ambiguous`
    // （`Ambiguous` 的判据是「多个合格」，与门槛无关——见 `resolve` 的文档第 4 格）。
    let ambiguous_with_unmet_gate = match resolve(&two_qualified, &k, &conf("high"), &unmet) {
        ResolutionResult::Ambiguous(r) => r,
        other => panic!("门槛变不变，多个合格都该是 `Ambiguous`，实得 {other:?}"),
    };

    // —— 四：零个合格（候选集为空）⟹ `Unknown` ——
    let empty: Vec<(Candidate, CandidateFacts)> = Vec::new();
    let unknown_case = match resolve(&empty, &k, &conf("high"), &met) {
        ResolutionResult::Unknown(r) => r,
        other => panic!("零个合格候选应落 `Unknown`，实得 {other:?}"),
    };

    // —— 逐格断言（四格都算过了，才不会「第一格红就看不到后面三格」）——
    assert_eq!(
        resolved_case.entity.as_str(),
        "e-only",
        "`Resolved` 的解出实体是那一枚合格候选",
    );
    assert_eq!(
        resolved_case.satisfied.iter().map(describe).collect::<Vec<_>>(),
        k.iter().map(describe).collect::<Vec<_>>(),
        "`Resolved.satisfied` 是**照录**判定时用的那一组约束",
    );
    assert_eq!(resolved_case.candidate_count, 1, "判定输入的规模是 1");

    assert_eq!(
        confirm_case.first.as_str(),
        "e-only",
        "`ConfirmRequired.first` 是那个**占优**候选",
    );
    assert_eq!(
        ids(&confirm_case.surviving),
        vec!["e-only"],
        "`surviving` 是满足全部显式约束的候选全体（这一格里恰是那一枚）",
    );
    assert_eq!(
        confirm_case.candidate_count, 1,
        "判定输入的规模仍是 1（门槛不改变输入）",
    );

    assert_eq!(
        ids(&ambiguous_case.surviving),
        vec!["e-a", "e-b"],
        "`Ambiguous.surviving` 是合格的**全体**，按输入次序、不去重、不排序",
    );
    assert_eq!(
        ambiguous_case.candidate_count, 2,
        "`candidate_count` 是判定**输入**的规模，不是合格数",
    );
    assert_eq!(
        ids(&ambiguous_with_unmet_gate.surviving),
        vec!["e-a", "e-b"],
        "门槛未过时 `Ambiguous` 的载荷一模一样——门槛在这一格没有可观察后果",
    );

    assert_eq!(
        unknown_case.reason,
        UnknownReason::NoCandidate,
        "空候选集的那一枚 `reason` 是 `NoCandidate`",
    );
    assert_eq!(
        unknown_case.candidate_count, 0,
        "空集的规模是 0（它是把两枚 `reason` 分开的第二个读数）",
    );
}

/// [`UnknownReason`] 的两枚**各一条**，且**混成一枚即红**。
///
/// - [`UnknownReason::NoCandidate`]：候选集**为空**（§4.2 的断言 C）；
/// - [`UnknownReason::AllCandidatesRejected`]：候选集**非空但全不合格**（§4.2 的断言 B）。
///
/// # 为什么必须两枚（混成一枚会丢什么）
///
/// 「本来就没有」与「全不合格」**处置方向相反**：前者要**扩大检索**
/// （§201 的 `0 candidate → NIL / Expand Retrieval`，`docs/spec/04-method.md:691-692`），
/// 后者要**问用户**（检索给了东西，是约束把它们全否掉了——再检索一次不会改变这件事）。
///
/// # 两枚的读数不只 `reason`：`candidate_count` 是第二个
///
/// 空那一枚必伴 **0**、非空那一枚必伴**一个非零的数**。故本用例两侧都断
/// （`reason` ＋ `candidate_count`）——只断 `reason` 的话，一个理由是常量、
/// 计数乱填的实现也能绿。
///
/// 红条件（**移除档**）：把两枚合成一枚（例如让非空但全不合格的那一格也返回
/// `NoCandidate`）——**判据上**不可能（编译期：`match` 少一臂即编译不过）；
/// 能取到的运行期红是把那一格的 `reason` 写成 `NoCandidate`（**取反档**），
/// 那时本用例第二条断言红。实测见报告 §变异轮。
#[test]
fn unknown_reason_distinguishes_an_empty_set_from_all_rejected() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let met = req("high");

    // 一：候选集为空 ⟹ `NoCandidate`，规模 0。
    let empty: Vec<(Candidate, CandidateFacts)> = Vec::new();
    let empty_reason = match resolve(&empty, &k, &conf("high"), &met) {
        ResolutionResult::Unknown(u) => u,
        other => panic!("空候选集应落 `Unknown`，实得 {other:?}"),
    };
    assert_eq!(
        empty_reason.reason,
        UnknownReason::NoCandidate,
        "候选集**为空**那一格：`NoCandidate`",
    );
    assert_eq!(empty_reason.candidate_count, 0, "空集的规模必为 0");

    // 二：候选集非空、但全部违反那一条约束 ⟹ `AllCandidatesRejected`，规模非零。
    let all_rejected = vec![
        (
            cand("e-x", Some(0.9)),
            facts(ResolutionScope::Global, EntityKind::Tool),
        ),
        (
            cand("e-y", Some(0.4)),
            facts(ResolutionScope::Session, EntityKind::Skill),
        ),
    ];
    let rejected_reason = match resolve(&all_rejected, &k, &conf("high"), &met) {
        ResolutionResult::Unknown(u) => u,
        other => panic!("全不合格应落 `Unknown`，实得 {other:?}"),
    };
    assert_eq!(
        rejected_reason.reason,
        UnknownReason::AllCandidatesRejected,
        "候选集**非空但全不合格**那一格：`AllCandidatesRejected`",
    );
    assert!(
        rejected_reason.candidate_count > 0,
        "这一格的规模必须非零（它把「本来就没有」与「全否掉了」分开）",
    );

    // 两枚**可区分**——这一句是本用例的名字所在。
    assert_ne!(
        empty_reason.reason, rejected_reason.reason,
        "两枚 `UnknownReason` 必须可区分：混成一枚，两条处置方向就再没有分支点",
    );
}

/// [`resolve`] 的返回类型**不是 `Result`**（§4.1 写死）。
///
/// # 照片机制
///
/// 1. `let r: ResolutionResult = resolve(...)` ——类型注解就是断言：若返回类型变成
///    `Result<…>`，这一行**编译不过**；
/// 2. 下面的 `match` **穷尽四支、不带 `..`、也没有 `Err` 臂**——`Err` 若存在，
///    这个 `match` 立刻不再穷尽（`E0004`）。
///
/// **两处都是编译期照片**，按本仓纪律（编译不过不算红）故**运行期无红**。
/// 本条能取到的运行期红是四态用例那四条（它们也直接对 `ResolutionResult` 断言）。
///
/// # 为什么本层没有第二种失败
///
/// §206 的 `abstain` 已把「没有可靠解析」收进 `Unknown`；输入全是值（候选集与门槛
/// 都由调用方给）、不做 I/O、不落库。故一个 `Result` 会在本层编出一个**没有产处的
/// `Err` 支**，而它的存在会让每个调用方都写一段永远不会走的分支。
///
/// # 本文件里**没有一条** `unwrap()`
///
/// 这是本条要求的可观察者：`unwrap()` 是关于 `Result` / `Option` 的语法，
/// 本层的结果类型两者都不是。全文 `grep -n 'unwrap' tests/resolver.rs` 零命中
/// （这一句是本文件的可核事实，不是转述）。
#[test]
fn resolve_returns_the_result_type_with_no_err_arm() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        cand("e-ok", None),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];

    // 类型注解即断言（返回类型不是 `Result`）。
    let r: ResolutionResult = resolve(&cs, &k, &conf("high"), &req("high"));

    // 穷尽四支、**不带 `..`**、没有 `Err` 臂。
    let which = match r {
        ResolutionResult::Resolved(_) => "resolved",
        ResolutionResult::ConfirmRequired(_) => "confirm_required",
        ResolutionResult::Ambiguous(_) => "ambiguous",
        ResolutionResult::Unknown(_) => "unknown",
    };
    assert_eq!(which, "resolved", "这一份输入落 `Resolved`");
}

/// 四支的**字段清单逐项**：机制 = 对四支**逐支穷尽解构、不带 `..`**。
///
/// 解构的是四个载荷类型**自己的字段**（§4.1 的字段清单）：
/// `Resolved { entity, satisfied, candidate_count }` /
/// `ConfirmRequired { first, surviving, candidate_count }` /
/// `Ambiguous { surviving, candidate_count }` /
/// `Unknown { reason, candidate_count }`。
///
/// 任一支加字段、少字段、改名 ⟹ **编译不过**（加字段 `E0027`、写不存在的字段 `E0026`、
/// 改名 `E0026`／`E0609`）。
///
/// **明写**：这是**编译期照片，运行期无照片**——四个载荷是纯数据声明，
/// 运行期能断言的只有「字段装得进、读得回」，而那与「字段清单**恰好**是这几个」不是一回事。
/// 加字段这件事**没有变红的观测**，只有一条编译错误，而按本仓纪律「编译不过」不算红。
///
/// # `candidate_count` **不是分数**（免得这一条被读成与 §4.4 第 1 条相抵）
///
/// 它是**规模**（`usize`，§4.2 的断言 A／D 拿它核「集合里有几个」）；
/// §4.4 第 1 条禁的是 `retrieval_score` 一类**排序用的数**。两者的区别由
/// `resolution_result_carries_no_score` 与 `resolution_confidence_never_appears_in_a_candidate_or_a_resolved`
/// 各自断言（那里逐枚断言语义，这里只断字段清单）。
#[test]
fn every_arm_carries_exactly_its_declared_fields() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let one = vec![(
        cand("e-one", None),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];
    let two = vec![
        (
            cand("e-1", None),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-2", None),
            facts(ResolutionScope::Project, EntityKind::Skill),
        ),
    ];
    let empty: Vec<(Candidate, CandidateFacts)> = Vec::new();

    // —— `Resolved`：**不带 `..`** ——
    match resolve(&one, &k, &conf("l"), &req("l")) {
        ResolutionResult::Resolved(Resolved {
            entity,
            satisfied,
            candidate_count,
        }) => {
            assert_eq!(entity.as_str(), "e-one");
            assert_eq!(satisfied.len(), 1);
            assert_eq!(candidate_count, 1);
        }
        other => panic!("夹具前提：这一份输入该落 `Resolved`，实得 {other:?}"),
    }

    // —— `ConfirmRequired`：**不带 `..`** ——
    match resolve(&one, &k, &conf("l"), &req("other")) {
        ResolutionResult::ConfirmRequired(ConfirmRequired {
            first,
            surviving,
            candidate_count,
        }) => {
            assert_eq!(first.as_str(), "e-one");
            assert_eq!(ids(&surviving), vec!["e-one"]);
            assert_eq!(candidate_count, 1);
        }
        other => panic!("夹具前提：这一份输入该落 `ConfirmRequired`，实得 {other:?}"),
    }

    // —— `Ambiguous`：**不带 `..`** ——
    match resolve(&two, &k, &conf("l"), &req("l")) {
        ResolutionResult::Ambiguous(Ambiguous {
            surviving,
            candidate_count,
        }) => {
            assert_eq!(ids(&surviving), vec!["e-1", "e-2"]);
            assert_eq!(candidate_count, 2);
        }
        other => panic!("夹具前提：这一份输入该落 `Ambiguous`，实得 {other:?}"),
    }

    // —— `Unknown`：**不带 `..`** ——
    match resolve(&empty, &k, &conf("l"), &req("l")) {
        ResolutionResult::Unknown(Unknown {
            reason,
            candidate_count,
        }) => {
            assert_eq!(reason, UnknownReason::NoCandidate);
            assert_eq!(candidate_count, 0);
        }
        other => panic!("夹具前提：这一份输入该落 `Unknown`，实得 {other:?}"),
    }
}

/// §4.4 第 1 条：**已解出的结果携带不了分数**。
///
/// # 机制（与上一条共用，为什么还要单独写一条）
///
/// 判据仍然是对 `Resolved` 做**不带 `..` 的穷尽解构**——它的字段清单里**没有任何
/// 分数字段**（`retrieval_score` 只出现在 [`Candidate`] 上，见 §272 与 §4.4）。
/// 与上一条分开写，是因为它是**另一条要求**（§14 的表把它与「字段清单逐项」并列记为两条），
/// 且这里还多断一件事：`candidate_count` 是**规模**、不是分数——它是 `usize`，
/// 数的是「集合里有几个」。**这一句要写出来**，否则它会与「结果携带不了分数」读起来相抵。
///
/// # 分数的去处：它是**判定输入**的一部分，不进判定输出
///
/// 下面那份输入里的候选**带着分数**（`Some(0.97)`），而结果的三个字段里一个数都没有——
/// 实体、约束、规模。**本条不主张「分数不参与任何判定」**（§272 说它 MAY 用于排序，
/// 那是消费者的事；本层今天也没有排序），它主张的是**结果的形状**：
/// 拿到一枚 `Resolved` 的调用方**在类型上取不到**那个分数。
///
/// 红条件（**移除档**）：给 `Resolved` 加一个 `confidence` 或 `retrieval_score` 字段
/// ⟹ 上一条与本条的解构都**编译不过**（`E0027`）；按本仓纪律那是**编译期照片**，
/// 故这一枚变异体的性质照实记：**没有运行期红**。见报告 §变异轮。
#[test]
fn resolution_result_carries_no_score() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        cand("e-scored", Some(0.97)),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];

    match resolve(&cs, &k, &conf("high"), &req("high")) {
        // **不带 `..`**：这一行就是本用例的机制（清单里没有第四个字段）。
        ResolutionResult::Resolved(Resolved {
            entity,
            satisfied,
            candidate_count,
        }) => {
            assert_eq!(entity.as_str(), "e-scored", "解出的还是那一枚带分数的候选");
            assert_eq!(satisfied.len(), 1, "照录的是那一条约束");
            // `candidate_count` 是**规模**：它是「集合里有几个」，不是任何分数。
            assert_eq!(candidate_count, 1usize, "规模是 usize，数的是候选个数");
        }
        other => panic!("该落 `Resolved`，实得 {other:?}"),
    }
}

/// §4.4 第 2 条：`ResolutionConfidence` **没有读者能把它变成一个数**。
///
/// # 照片在这条用例的文件外（**跨类型不可转换**只有编译期能钉）
///
/// - 反例：`tests/compile_fail/resolution_confidence_has_no_numeric_reader.rs`
///   ——`c.to_f64()` / `c.as_probability()` 编不过（`.stderr` 实跑取值）；
/// - 反例：`tests/compile_fail/resolution_confidence_cannot_be_built_from_a_score.rs`
///   ——`ResolutionConfidence::from_score(0.9)` / `::from(0.9f64)` 编不过；
/// - **正控制**：`tests/compile_pass/resolution_confidence_meets_is_usable.rs`
///   ——`c.meets(&required)` 编得过。没有它，上面两份反例的绿可以来自
///   「`ResolutionConfidence` 这个名字整个不存在」这类与判据无关的原因
///   （本仓「正控制不得自证」的同一形状）。
///
/// # 运行期这一半钉什么（弱，但写清它弱在哪）
///
/// 只能钉「唯一那个读者返回的是 `bool`，且它**两边都读**」：
/// 同档 ⟹ `true`、异档 ⟹ `false`。**它钉不住**「没有别的读者」——
/// 方法名是开放集，枚举不完（与 `CandidateSource` 的 `pick_one` 守卫同一限度）。
/// 那半张照片由上面两份反例提供（它们钉的也只是一个名字，照实记）。
///
/// # 判据为什么是**等值**
///
/// §275 的取值域与 §276 的 `f` 规范都没给，故本层**没有这两枚值之间的序**；
/// 造一个序会把「高门槛」判成「低门槛」（比没有更坏），取等值落在 fail-closed 那侧
/// （更严 ⟹ 更容易落到 `ConfirmRequired`，而它不会自动绑定）。
/// 与 §4.1.1 的 `Scope` 取等值同一条理由——判据的实现见 `ResolutionConfidence::meets`。
///
/// 红条件（**取反档**）：加 `impl From<ResolutionConfidence> for f64`
/// ⟹ 上面第二份反例的 `.stderr` 变红（那份样例里 `::from(0.9f64)` 与 `to_f64()`
/// 会有一枚变成合法调用）。实测见报告 §变异轮。
#[test]
fn resolution_confidence_has_no_reader_that_yields_a_number() {
    // 唯一的读者是 `meets`，它返回 `bool`——**类型注解即断言**。
    let met: bool = conf("high").meets(&req("high"));
    let unmet: bool = conf("high").meets(&req("unreachable"));
    assert!(met, "同档 ⟹ 达到要求");
    assert!(!unmet, "异档 ⟹ 未达到要求（它两边都读，不是恒真／恒假）");

    // 它**不出现在**已解出的结果里（判据的另一半在下面那条用例里）。
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        cand("e-gate", None),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];
    let r = resolve(&cs, &k, &conf("high"), &req("high"));
    assert!(
        matches!(&r, ResolutionResult::Resolved(_)),
        "门槛过了才落 `Resolved`（门槛唯一能改变结果的地方是「唯一合格」那一格）：{r:?}",
    );
}

/// §4.4 第 2 条的后半：`ResolutionConfidence` **不出现在** `Candidate` 与 `Resolved`
/// 的字段里（**两侧都断**）。
///
/// # 机制
///
/// 两处**不带 `..` 的穷尽解构**：
/// - [`Candidate`]（Task 3 的三字段，§272 照录）；
/// - [`Resolved`]（本 task 的三字段，§4.1）。
///
/// 把 `ResolutionConfidence` 塞进任何一侧 ⟹ 那一侧的解构**编译不过**（`E0027`）。
/// **照实记**：这是编译期照片，运行期无红。
///
/// 红条件（**取反档**）：给 `Candidate` 加 `pub confidence: ResolutionConfidence`
/// ⟹ 本用例第一段编不过；**同一枚变异体还会让 Task 3 的**
/// `tests/candidate.rs::the_candidate_records_exactly_the_three_fields_of_272` **先红**
/// （两处钉同一张清单，那是设计有意的重复：一处钉 §272 的形、一处钉 §4.4 的位置）。
#[test]
fn resolution_confidence_never_appears_in_a_candidate_or_a_resolved() {
    // —— `Candidate` 那一侧：**不带 `..`** ——
    let c = cand("e-272", Some(0.5));
    let Candidate {
        entity_id,
        evidence,
        retrieval_score,
    } = c;
    assert_eq!(entity_id.as_str(), "e-272");
    assert!(evidence.is_empty());
    assert_eq!(retrieval_score, Some(0.5));

    // —— `Resolved` 那一侧：**不带 `..`** ——
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        cand("e-res", None),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];
    match resolve(&cs, &k, &conf("high"), &req("high")) {
        ResolutionResult::Resolved(Resolved {
            entity,
            satisfied,
            candidate_count,
        }) => {
            assert_eq!(entity.as_str(), "e-res");
            assert_eq!(described(&satisfied), vec!["scope:Project".to_owned()]);
            assert_eq!(candidate_count, 1);
        }
        other => panic!("该落 `Resolved`，实得 {other:?}"),
    }
}

/// `describe` 对一批约束的投影（夹具函数，见它的文档）。
fn described(ks: &[ExplicitConstraint]) -> Vec<String> {
    ks.iter().map(describe).collect()
}

/// §4.4 第 3 条：`retrieval_score` 与 `ResolutionConfidence` **两个数不同轴**。
///
/// # 照片机制（公开面清单逐条）
///
/// 1. **`ResolutionConfidence` 的构造函数不接受分数**：公开入口是
///    `ResolutionConfidence::new(Opaque)`（本层不解释那个不透明值），
///    **没有** `from_score` 一类；下面第一段用的就是那个入口——
///    若有人把它改成收 `f64`，`conf("high")` 这一族夹具**全部编不过**。
/// 2. **没有 `impl From<f64> for ResolutionConfidence`**：反例
///    `tests/compile_fail/resolution_confidence_cannot_be_built_from_a_score.rs`（`.stderr` 实跑取值）。
/// 3. **两个数落在两个类型上**：`retrieval_score` 是 [`Candidate`] 上的 `Option<f64>`
///    （§272，排序用、无取值域），`ResolutionConfidence` 只被 [`resolve`] 的门槛判定读，
///    且只读成一个 `bool`。
///
/// # 本层**没有**判据的那一半（照实写，免得被读成已覆盖）
///
/// 「`ResolutionConfidence` 必须经过**校准**」（§275，`docs/spec/05-normative.md:1428-1448`）
/// **在本层没有判据、也没有照片**——校准是一个过程，本层不取系统时钟、不做 I/O、
/// 也没有任何校准所需的事实（设计 §15.2 第 2 条已把这一类记为「不是没有照片，
/// 是没有判据」，`design:2262-2263`）。故本层在类型上只做到
/// **「它不能被分数构造」**这一条；「它确实是校准出来的」这件事本层**说不出**。
///
/// 红条件（**取反档**）：给 `ResolutionConfidence` 加 `impl From<f64>`
/// ⟹ 上面第 2 条那份反例红。实测见报告 §变异轮。
#[test]
fn the_two_numbers_are_not_on_the_same_axis() {
    // 一：置信值从**不透明值**来（不是从分数来）。
    let c = ResolutionConfidence::new(Opaque::from_str("calibrated"));
    assert!(
        c.meets(&RequiredStrength::new(Opaque::from_str("calibrated"))),
        "同一档 ⟹ 放行（判据是等值，见 `ResolutionConfidence::meets`）",
    );
    assert!(
        !c.meets(&RequiredStrength::new(Opaque::from_str("higher-than-calibrated"))),
        "不同档 ⟹ 不放行——门槛不是恒真",
    );

    // 二：排序用的那个数长在**候选**上，且本层不解释它（`Option<f64>`，§272）。
    let scored = cand("e-rank", Some(0.42));
    let unscored = cand("e-rank", None);
    assert_eq!(
        scored.retrieval_score, Some(0.42),
        "候选带分数（排序用；本层不归一、不换算成概率）",
    );
    assert_ne!(
        scored.retrieval_score.is_none(),
        unscored.retrieval_score.is_none(),
        "「没给分数」与「给了分数」可区分（§272 的 `retrieval_score?`）",
    );

    // 三：两个数**各自进各自的类型**——结果的字段里没有任何数（上一条已逐项解构）。
    //     这里只把「它们不是一个东西」写成一条可读的断言：门槛判定读的是
    //     `ResolutionConfidence` 与 `RequiredStrength`，两个都不在 `Candidate` 上。
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        scored.clone(),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];
    let r = resolve(&cs, &k, &conf("calibrated"), &req("calibrated"));
    assert!(
        matches!(&r, ResolutionResult::Resolved(_)),
        "门槛那一格读的是 `confidence.meets(required)` 这两个不透明值：{r:?}",
    );
}
