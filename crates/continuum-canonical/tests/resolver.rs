//! Task 4 的四态用例（§4.1）与「两个数不同轴」（§4.4）。
//!
//! 每条用例的头注释写明**它钉的是什么**与**红条件是什么**；红条件在本 task 的变异轮里
//! 逐条真的取过（真写变异体、真跑到红），取到的结果记在
//! `.superpowers/sdd/p4-task-4-report.md`。
//!
//! # 三条用例住在 `src/resolver.rs` 的 `#[cfg(test)]` 里（本轮裁决一搬的）
//!
//! `every_arm_carries_exactly_its_declared_fields`（字段清单逐项）、
//! `resolution_result_carries_no_score`（结果携带不了分数）、
//! `resolution_confidence_never_appears_in_a_candidate_or_a_resolved`（它两侧的解构）
//! ——这三条的机制都是**对载荷做不带 `..` 的穷尽解构**。四个载荷的字段本轮**转私有**
//! （`src/resolver.rs` 的文件头写了理由：crate 外构造不出 `ResolutionResult`），
//! 穷尽解构于是只有 crate 内写得出。**用例名、断言、红条件一字未改**，只是换了文件。
//! 本文件里**读载荷走只读访问器**（`resolved_case.entity()` 一类）。
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

// **四个载荷类型（`Resolved` / `ConfirmRequired` / `Ambiguous` / `Unknown`）不在这里 import**：
// 它们的字段私有（本轮裁决一），故「不带 `..` 的穷尽解构」只有 crate 内写得出——
// 那三条用例搬进了 `src/resolver.rs` 的 `#[cfg(test)]`。本文件读载荷**走只读访问器**
// （`resolved_case.entity()` 一类），故四个载荷类型名一个都不必 import。
use continuum_canonical::{
    Candidate, CandidateFacts, EntityId, EntityKind, ExplicitConstraint, Opaque, RequiredStrength,
    ResolutionConfidence, ResolutionResult, ResolutionScope, UnknownReason, resolve, satisfies,
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
    let resolved_case = match resolve(&one_qualified, None, &k, &conf("high"), &met) {
        ResolutionResult::Resolved(r) => r,
        other => panic!("唯一合格且门槛已过应落 `Resolved`，实得 {other:?}"),
    };

    // —— 二：同一份输入、门槛未过 ⟹ `ConfirmRequired` ——
    let confirm_case = match resolve(&one_qualified, None, &k, &conf("high"), &unmet) {
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
    let ambiguous_case = match resolve(&two_qualified, None, &k, &conf("high"), &met) {
        ResolutionResult::Ambiguous(r) => r,
        other => panic!("两个合格候选应落 `Ambiguous`，实得 {other:?}"),
    };
    // **门槛在这一格不可观察**：换成「门槛未过」仍是 `Ambiguous`
    // （`Ambiguous` 的判据是「多个合格」，与门槛无关——见 `resolve` 的文档第 4 格）。
    let ambiguous_with_unmet_gate = match resolve(&two_qualified, None, &k, &conf("high"), &unmet) {
        ResolutionResult::Ambiguous(r) => r,
        other => panic!("门槛变不变，多个合格都该是 `Ambiguous`，实得 {other:?}"),
    };

    // —— 四：零个合格（候选集为空）⟹ `Unknown` ——
    let empty: Vec<(Candidate, CandidateFacts)> = Vec::new();
    let unknown_case = match resolve(&empty, None, &k, &conf("high"), &met) {
        ResolutionResult::Unknown(r) => r,
        other => panic!("零个合格候选应落 `Unknown`，实得 {other:?}"),
    };

    // —— 逐格断言（四格都算过了，才不会「第一格红就看不到后面三格」）——
    // 载荷的字段私有（本轮裁决一），读它们走**只读访问器**——这也是 Task 5 的读法。
    assert_eq!(
        resolved_case.entity().as_str(),
        "e-only",
        "`Resolved` 的解出实体是那一枚合格候选",
    );
    assert_eq!(
        resolved_case.satisfied().iter().map(describe).collect::<Vec<_>>(),
        k.iter().map(describe).collect::<Vec<_>>(),
        "`Resolved.satisfied` 是**照录**判定时用的那一组约束",
    );
    assert_eq!(resolved_case.candidate_count(), 1, "判定输入的规模是 1");

    assert_eq!(
        confirm_case.first().as_str(),
        "e-only",
        "`ConfirmRequired.first` 是那个**占优**候选",
    );
    assert_eq!(
        ids(confirm_case.surviving()),
        vec!["e-only"],
        "`surviving` 是满足全部显式约束的候选全体（这一格里恰是那一枚）",
    );
    assert_eq!(
        confirm_case.candidate_count(), 1,
        "判定输入的规模仍是 1（门槛不改变输入）",
    );

    assert_eq!(
        ids(ambiguous_case.surviving()),
        vec!["e-a", "e-b"],
        "`Ambiguous.surviving` 是合格的**全体**，按输入次序、不去重、不排序",
    );
    assert_eq!(
        ambiguous_case.candidate_count(), 2,
        "`candidate_count` 是判定**输入**的规模，不是合格数",
    );
    assert_eq!(
        ids(ambiguous_with_unmet_gate.surviving()),
        vec!["e-a", "e-b"],
        "门槛未过时 `Ambiguous` 的载荷一模一样——门槛在这一格没有可观察后果",
    );

    assert_eq!(
        unknown_case.reason(),
        &UnknownReason::NoCandidate,
        "空候选集的那一枚 `reason` 是 `NoCandidate`",
    );
    assert_eq!(
        unknown_case.candidate_count(), 0,
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
    let empty_reason = match resolve(&empty, None, &k, &conf("high"), &met) {
        ResolutionResult::Unknown(u) => u,
        other => panic!("空候选集应落 `Unknown`，实得 {other:?}"),
    };
    assert_eq!(
        empty_reason.reason(),
        &UnknownReason::NoCandidate,
        "候选集**为空**那一格：`NoCandidate`",
    );
    assert_eq!(empty_reason.candidate_count(), 0, "空集的规模必为 0");

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
    let rejected_reason = match resolve(&all_rejected, None, &k, &conf("high"), &met) {
        ResolutionResult::Unknown(u) => u,
        other => panic!("全不合格应落 `Unknown`，实得 {other:?}"),
    };
    assert_eq!(
        rejected_reason.reason(),
        &UnknownReason::AllCandidatesRejected,
        "候选集**非空但全不合格**那一格：`AllCandidatesRejected`",
    );
    assert!(
        rejected_reason.candidate_count() > 0,
        "这一格的规模必须非零（它把「本来就没有」与「全否掉了」分开）",
    );

    // 两枚**可区分**——这一句是本用例的名字所在。
    assert_ne!(
        empty_reason.reason(),
        rejected_reason.reason(),
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
/// 本条能取到的运行期红是**四态用例那几条**（它们也直接对 `ResolutionResult` 断言；
/// 其中三条本轮搬进了 `src/resolver.rs` 的 `#[cfg(test)]`，见那个模块的说明）。
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
/// 本层的结果类型两者都不是。
/// **判据是「去掉注释之后，`grep -n 'unwrap' tests/resolver.rs` 零命中」**——
/// 数的是**代码行**；注释里出现的那些（含写这条判据的**本句自己**）不算。
/// **故此处不写命中数**：任何一个数值都会被写它的这一句自身推翻。
#[test]
fn resolve_returns_the_result_type_with_no_err_arm() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        cand("e-ok", None),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];

    // 类型注解即断言（返回类型不是 `Result`）。
    let r: ResolutionResult = resolve(&cs, None, &k, &conf("high"), &req("high"));

    // 穷尽四支、**不带 `..`**、没有 `Err` 臂。
    let which = match r {
        ResolutionResult::Resolved(_) => "resolved",
        ResolutionResult::ConfirmRequired(_) => "confirm_required",
        ResolutionResult::Ambiguous(_) => "ambiguous",
        ResolutionResult::Unknown(_) => "unknown",
    };
    assert_eq!(which, "resolved", "这一份输入落 `Resolved`");
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
    let r = resolve(&cs, None, &k, &conf("high"), &req("high"));
    assert!(
        matches!(&r, ResolutionResult::Resolved(_)),
        "门槛过了才落 `Resolved`（门槛唯一能改变结果的地方是「唯一合格」那一格）：{r:?}",
    );
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
    let r = resolve(&cs, None, &k, &conf("calibrated"), &req("calibrated"));
    assert!(
        matches!(&r, ResolutionResult::Resolved(_)),
        "门槛那一格读的是 `confidence.meets(required)` 这两个不透明值：{r:?}",
    );
}

// ============================================================================
// Task 5：§4.2 的「不存在正确候选时不强行绑定」——四条断言 A／B／C／D 与两侧守卫。
//
// 每条用例的头注释写明**它钉的是什么**与**红条件是什么**；红条件在 Task 5 的变异轮里
// 逐条真的取过（真写变异体、真跑到红），取到的结果记在
// `.superpowers/sdd/p4-task-5-report.md`。
//
// # 「不存在正确候选时不强行绑定」这句绝对措辞的照片就是下面这组
// （**不写数值**：这组里每加一条，那个数就旧——本仓已有「自指计数不写数值」这条判据）
//
// A（在来源里）、A 的注入守卫（除两个声明的来源外没有第三个）、B（全不合格不产出）、
// C（空集不产出）、D 第 1 条（合格者被选中）、D 第 2 条（多合格者得 `Ambiguous` 且
// `surviving` 恰是合格的那两枚）、外加「守卫的另一侧」（唯一合格**要**产出）。
// **少任何一条，这条绝对措辞就有一格没有照片**：A／B／D 三条都在 fail-open 侧
// （少了它们，系统仍会返回一个看起来正常的 `Resolved`），C 在 fail-closed 侧，一条即可
// （设计 §4.2 末段，`design:682-686`）。
//
// # 每条夹具都含至少一条**非空**显式约束 `K`
//
// 约束集为空时每个候选都平凡地合格（`qualifying` 的合取在空集上取中性元），用例会退化成
// 「两个候选都在集合里」——**恒真**的用例，一个把「合格」判成恒真的实现照样绿。
// 故下面每一条都先断言夹具的**前提**（哪一枚满足 `K`、哪一枚违反），把「合格／不合格」
// 变成被证明的，而不是被假定的（本仓「正控制不得自证」的同一条）。
//
// # 载荷一律经只读访问器读
//
// 四个载荷的字段私有（见 `src/resolver.rs` 的文件头），本文件在 crate 外，
// 故按名解构（`Unknown { reason, .. }`）与结构体字面量都写不出来——
// 下面全部走 `entity()` / `surviving()` / `reason()` / `candidate_count()`。
// ============================================================================

/// §4.2 断言 A 的判据，**可执行形态**：`e ∈ 检索集 ∪ {由 binding 注入的那一枚}`。
///
/// # 这里不是「`e` ∈ 工作集」：因为那个版本会恒真
///
/// A 的初稿写「`e` 是**输入候选集**里的成员」，而「输入候选集」两读都通
/// （设计 §4.2 的订正，`design:616-628`）：
///
/// - 读成「**检索器交出的那一集**」——§4.6 的 binding 注入会让 A 变假，故 A 必须
///   把注入的那一枚**显式列成第二个来源**（本函数的 `injected` 参数就是它）；
/// - 读成「**解析器实际工作的那一集**」——注入之后 A **恒真**，解析器可以自造任何实体
///   而 A 永不响，而 A 本来就是防「解析器自造实体」的。
///
/// 故 A 拿的是**两个声明的来源**的并，不是「解析器手里那一集」。
/// 证据在设计自己的照片里：`Resolved.candidate_count` 比检索集多 1（§4.6 的照片 (a)）
/// ⟹ 「检索集」与「工作集」是两个集合。
///
/// # 今天 `injected` 一律传 `None`
///
/// `ExplicitBinding` 的注入点是 Task 6（`binding::apply`），今天**没有第二个来源**。
/// 参数留在签名里，是为了让注入落地之后这条判据**不必重写**且**仍不恒真**——
/// 判据写在「来源」上而不是写在「工作集」上，`assertion_a_injection_guard` 钉的就是这个差别。
fn a_holds(
    e: &EntityId,
    retrieved: &[(Candidate, CandidateFacts)],
    injected: Option<&EntityId>,
) -> bool {
    retrieved
        .iter()
        .any(|(c, _)| c.entity_id.as_str() == e.as_str())
        || injected.is_some_and(|i| i.as_str() == e.as_str())
}

/// A 里「**至多一次**注入」那一侧的规模上界：`|检索集| + [有注入]`。
///
/// 它与 [`a_holds`] 是同一条判据的两个读数（成员、规模），
/// 对应设计给的两条观测形式（`design:637-640`）。
fn a_count_bound(retrieved: &[(Candidate, CandidateFacts)], injected: Option<&EntityId>) -> usize {
    retrieved.len() + usize::from(injected.is_some())
}

/// **断言 A**（§4.2）：`Resolved(e)` 蕴含 `e ∈ 检索集 ∪ {注入的那一枚}`。
///
/// 不是「`e` 与某个候选相似」，而是**同一个 `EntityId` 值**（经 `as_str` 比）。
/// 它排除的失败形态：解析器自造一个实体、或把检索分数最高的候选的**名字拼出来**。
///
/// 夹具：`K = [Scope(Project)]`（非空）；`{e-c1 满足 K, e-c2 违反 K}`。
///
/// 红条件（**取反档**）：让解析器不问候选、自己拼一个实体出来
/// （例如把 `only.entity_id` 换成 `EntityId::from_str("e-invented")`，或把最高分候选的
/// 名字按格式拼出来）⟹ 本用例第一条断言红。实测见报告 §变异轮。
#[test]
fn assertion_a_a_resolved_entity_is_a_member_of_the_retrieved_set_or_the_injected_one() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![
        (
            cand("e-c1", Some(0.50)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-c2", Some(0.99)),
            facts(ResolutionScope::Global, EntityKind::Tool),
        ),
    ];
    // 夹具前提：这两枚**确实**一个合格、一个不合格（`K` 非空，故这不是恒真的一格）。
    assert!(
        satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：e-c1 满足 K",
    );
    assert!(
        !satisfies(&retrieved[1].0, &retrieved[1].1, &k[0]),
        "夹具前提：e-c2 违反 K（它分数更高，故它不是被分数选中的那一枚）",
    );

    // 今天没有第二个来源（注入点是 Task 6）。
    let injected: Option<EntityId> = None;

    let r = resolve(&retrieved, None, &k, &conf("high"), &req("high"));
    let x = match &r {
        ResolutionResult::Resolved(x) => x,
        other => panic!("恰一个合格候选、门槛已过应落 `Resolved`，实得 {other:?}"),
    };

    assert!(
        a_holds(x.entity(), &retrieved, injected.as_ref()),
        "断言 A：解出的实体必须落在「检索集 ∪ 注入的那一枚」里，实得 {} \
         （不在两个来源的并里 ⟹ 解析器自造了实体，或把某个名字拼了出来）",
        x.entity().as_str(),
    );
    // 同一枚 `EntityId` 值（经 `as_str` 比）——不是「像某个候选」。
    assert_eq!(
        x.entity().as_str(),
        retrieved[0].0.entity_id.as_str(),
        "解出的 id 与检索集里那一枚是**同一个值**（不是相似、不是拼出来的）",
    );
    assert_eq!(
        x.candidate_count(),
        retrieved.len(),
        "`candidate_count` 是判定**输入**的规模：它与夹具给的检索集一致 \
         （今天没有注入，故上界 `|检索集| + 1` 取等号）",
    );
    assert!(
        x.candidate_count() <= a_count_bound(&retrieved, injected.as_ref()),
        "规模不得超过「检索集 ＋ 至多一次注入」",
    );
}

/// **断言 A 的配套守卫**（§4.2 点名的那一条）：**注入只能来自 `ExplicitBinding`**。
///
/// # 没有这条用例，A 只是同义反复
///
/// 若 A 被实现成「`e` ∈ 工作集」（解析器手里那一集），那么注入之后它**恒真**：
/// 解析器自造一个实体、把它塞进自己那一集，A 照样成立。
/// 本用例把判据**钉在两个声明的来源上**，故自造实体在这里红。
///
/// # 夹具刻意选成「自造实体最像是对的那一格」
///
/// `K = [Scope(Project), BoundTo(e-pointed)]`，检索集里那一枚**违反 `Scope`**。
/// 「用户点名的实体就是"正确候选"」是一个很自然的错误实现——那时解析器会返回
/// `Resolved(e-pointed)`，而 **`e-pointed` 既不在检索集里、今天也没有 binding 给它背书**：
/// 它不在两个来源的并里。正确结果是 `Unknown { AllCandidatesRejected }`
/// （`binding` 不豁免约束；那一侧的正式用例是 Task 6 的照片 (b)，这里钉的是
/// 「**在今天这个没有注入点的实现里**，除检索集外没有第二个来源」）。
///
/// # 红条件（这条用例本身的红）
///
/// 在 `resolve` 里加一条「没有合格候选时，把 `BoundTo` 点名的那个实体作为候选产出」
/// （**取反档**：它正是「解析器自造实体」的形态）⟹ 结果落 `Resolved(e-pointed)`，
/// 本用例 `Resolved` 那一支的**第一条**断言红（`a_holds` 为假）。
/// 把 A 实现成「`e` ∈ 工作集」（**放宽档**）在**今天**与正确实现不可分
/// ——注入点落地（Task 6）之后两者才分岔，而这条用例正是为那一刻准备的：
/// 它读的是「两个声明的来源」，不是「解析器手里那一集」。
/// 实测见报告 §变异轮。
#[test]
fn assertion_a_injection_guard() {
    let k = [
        ExplicitConstraint::Scope(ResolutionScope::Project),
        ExplicitConstraint::BoundTo(EntityId::from_str("e-pointed")),
    ];
    // 检索集里的那一枚**违反 `Scope`**：合格集因此为空。它的 id 与 `BoundTo` 的那个也不同。
    let retrieved = vec![(
        cand("e-retrieved", Some(0.97)),
        facts(ResolutionScope::Global, EntityKind::Tool),
    )];
    assert!(
        !satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：这一枚违反 `Scope`，故合格集为空",
    );

    // 今天没有第二个来源：`ExplicitBinding` 的注入点（`binding::apply`）是 Task 6。
    let injected: Option<EntityId> = None;

    let r = resolve(&retrieved, None, &k, &conf("high"), &req("high"));
    match &r {
        // —— 这一支就是守卫本身 ——
        ResolutionResult::Resolved(x) => {
            assert!(
                a_holds(x.entity(), &retrieved, injected.as_ref()),
                "断言 A 的注入守卫：解出的实体 {} 不在「检索集 ∪ 注入的那一枚」里 \
                 ——自造实体只能来自 `ExplicitBinding` 这唯一一个注入点，今天是零个",
                x.entity().as_str(),
            );
            assert!(
                x.candidate_count() <= a_count_bound(&retrieved, injected.as_ref()),
                "观测形式之二：`candidate_count` 不得大于「检索集 ＋ 至多一次注入」的应有大小",
            );
        }
        // —— 正确实现走这一支 ——
        ResolutionResult::Unknown(u) => {
            assert_eq!(
                u.reason(),
                &UnknownReason::AllCandidatesRejected,
                "检索集非空、但全部违反约束：`AllCandidatesRejected`（不是 `NoCandidate`）",
            );
        }
        other => panic!(
            "合格集为空且候选集非空应落 `Unknown {{ AllCandidatesRejected }}`，实得 {other:?}",
        ),
    }
}

/// **断言 B**（§4.2，拒绑）：候选集非空 ∧ 全部候选都违反某个显式约束 ⟹ **不是 `Resolved`**。
///
/// # 「全部候选都得判到」——故**逐候选一条断言，不抽代表**
///
/// 夹具：三条非空约束 `K = [Scope(Project), EntityKind(Skill), BoundTo(e-x)]`，
/// 三枚候选各**恰好违反一条不同的**约束（`c1` 违反 `Scope`、`c2` 违反 `EntityKind`、
/// `c3` 违反 `BoundTo`）。每一枚各喂一次 [`resolve`]（它在该次里是唯一候选），
/// **每一次**都断言不是 `Resolved`，并断言该次的具体态。
///
/// 「恰好违反一条」不是假定的：本用例对每一枚**逐条**核
/// 「它违反的正是第 `i` 条、其余两条都满足」，故一个写歪的夹具会让本用例红，
/// 而不是静默地少覆盖一格（本仓「枚举断言须逐项有照片」）。
///
/// # 逐次断言的是哪一态（`ConfirmRequired` 在本条取不到照片，照实说）
///
/// 本条的每一格里，候选集非空而**合格数为零**，按 [`resolve`] 写死的判定顺序落
/// `Unknown { AllCandidatesRejected }`（`NoCandidate` 那一枚的界线是**候选集为空**，
/// 见 `resolve` 的文档）。**`ConfirmRequired` 这一格在本条到不了**：它的条件是
/// 「恰有**一个合格**候选而门槛未过」，与本条「全部候选都不合格」相抵——
/// 故这里逐次断言的是 `AllCandidatesRejected`，不是「`Unknown` 或 `ConfirmRequired` 二选一」。
///
/// 红条件（**放宽档**）：把「全部候选都不合格」也判成 `Resolved`
/// （例如让 `qualifying` 在结果为空时退回全体候选）⟹ 本用例逐枚红。实测见报告 §变异轮。
#[test]
fn assertion_b_each_candidate_violating_a_different_constraint_is_not_resolved() {
    let k = [
        ExplicitConstraint::Scope(ResolutionScope::Project),
        ExplicitConstraint::EntityKind(EntityKind::Skill),
        ExplicitConstraint::BoundTo(EntityId::from_str("e-x")),
    ];
    // 每一枚**恰好违反一条**：c1 违反 k0（其 id 与 kind 都对），
    // c2 违反 k1，c3 违反 k2。c1 与 c2 同 id 是故意的——`BoundTo` 要它们都得是 `e-x`。
    let candidates = vec![
        (
            cand("e-x", Some(0.9)),
            facts(ResolutionScope::Global, EntityKind::Skill),
        ),
        (
            cand("e-x", Some(0.8)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-y", Some(0.7)),
            facts(ResolutionScope::Project, EntityKind::Skill),
        ),
    ];

    for (i, (c, f)) in candidates.iter().enumerate() {
        // —— 夹具前提：这一枚违反的**正是第 `i` 条**，其余两条都满足 ——
        let violated: Vec<usize> = (0..k.len())
            .filter(|&j| !satisfies(c, f, &k[j]))
            .collect();
        assert_eq!(
            violated,
            vec![i],
            "夹具前提：第 {i} 枚候选必须**恰好**违反第 {i} 条约束（实得违反 {violated:?}）",
        );

        // —— 这一枚单独喂进去：它是该次唯一的候选，且不合格 ——
        let r = resolve(
            std::slice::from_ref(&candidates[i]),
            None,
            &k,
            &conf("high"),
            &req("high"),
        );
        assert!(
            !matches!(&r, ResolutionResult::Resolved(_)),
            "断言 B：第 {i} 枚候选违反了第 {i} 条约束，它**唯一**且不合格，\
             结果不许是 `Resolved`——实得 {r:?}",
        );
        match &r {
            ResolutionResult::Unknown(u) => {
                assert_eq!(
                    u.reason(),
                    &UnknownReason::AllCandidatesRejected,
                    "第 {i} 枚：候选集非空（规模 1）而合格数为零 ⟹ `AllCandidatesRejected`",
                );
                assert_eq!(u.candidate_count(), 1, "第 {i} 枚：判定输入的规模是 1");
            }
            other => panic!(
                "第 {i} 枚应落 `Unknown {{ AllCandidatesRejected }}`，实得 {other:?}",
            ),
        }
    }

    // —— 三枚一起喂：仍然全部不合格 ⟹ 仍不是 `Resolved`，且规模是 3 ——
    let all = resolve(&candidates, None, &k, &conf("high"), &req("high"));
    assert!(
        !matches!(&all, ResolutionResult::Resolved(_)),
        "断言 B（全集）：三枚各违反一条，没有一枚合格，结果不许是 `Resolved`——实得 {all:?}",
    );
    match &all {
        ResolutionResult::Unknown(u) => {
            assert_eq!(
                u.reason(),
                &UnknownReason::AllCandidatesRejected,
                "全集：候选集非空（规模 3）而合格数为零 ⟹ `AllCandidatesRejected`",
            );
            assert_eq!(
                u.candidate_count(),
                3,
                "全集：判定输入的规模是 3（`candidate_count` 数的是**输入**，不是合格数）",
            );
        }
        other => panic!("全集应落 `Unknown {{ AllCandidatesRejected }}`，实得 {other:?}"),
    }
}

/// **断言 C**（§4.2，空集）：候选集为空 ⟹ `Unknown { NoCandidate }`。
///
/// fail-closed 侧，一条即可（设计 §4.2 末段）——**但这一条必须逐项断到两个读数**：
/// `reason` **与** `candidate_count`。只断 `reason` 的话，一个理由写死成常量、
/// 计数乱填的实现也能绿（本仓「守卫须两侧都钉」的同一条）。
///
/// 红条件（**取反档**，两枚 `UnknownReason` 混用正是它要防的）：
/// 把空集那一格的 `reason` 给成 `AllCandidatesRejected` ⟹ 本用例红；
/// 空集返回 `ConfirmRequired` ⟹ 本用例红。实测见报告 §变异轮。
#[test]
fn assertion_c_an_empty_candidate_set_is_unknown_with_no_candidate() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let empty: Vec<(Candidate, CandidateFacts)> = Vec::new();

    let r = resolve(&empty, None, &k, &conf("high"), &req("high"));
    let u = match &r {
        ResolutionResult::Unknown(u) => u,
        other => panic!(
            "断言 C：候选集为空，没有可绑定的来源，结果不许是 `Resolved` / \
             `ConfirmRequired` / `Ambiguous`——应落 `Unknown {{ NoCandidate }}`，实得 {other:?}",
        ),
    };
    assert_eq!(
        u.reason(),
        &UnknownReason::NoCandidate,
        "断言 C：候选集**为空**那一枚 `reason` 是 `NoCandidate`（不是 `AllCandidatesRejected`）",
    );
    assert_eq!(
        u.candidate_count(),
        0,
        "断言 C 的第二个读数：空集的规模必为 0",
    );
}

/// **断言 D 第 1 条**（§4.2）：`Resolved(e)` 蕴含 `e` **满足全部显式约束**。
///
/// 夹具：`K = [Scope(Project)]`（非空）；`{e-qualified 满足 K, e-unqualified 违反 K}`，
/// 且**不合格的那一枚检索分数更高**（`0.99` vs `0.10`）。
///
/// # 为什么钉住 A 不足以替代 D
///
/// `e-unqualified` **也在检索集里**——若解析器绑定到它，**A 仍然成立**。
/// 下面第三条断言就是这件事的证据：`a_holds` 对「绑到不合格的那一枚」**不响**。
/// 故 D 是独立的一条，不能由 A 推出。
///
/// 红条件（**移除档**）：删掉约束求值（让候选全体都算合格、再取检索分数最高的那一枚）
/// ⟹ 会绑定到 `e-unqualified`，本用例第一条断言红。实测见报告 §变异轮。
#[test]
fn assertion_d_1_the_qualified_candidate_is_selected_even_when_an_unqualified_one_is_present() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![
        (
            cand("e-qualified", Some(0.10)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-unqualified", Some(0.99)),
            facts(ResolutionScope::Global, EntityKind::Tool),
        ),
    ];
    assert!(
        satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：e-qualified 满足 K",
    );
    assert!(
        !satisfies(&retrieved[1].0, &retrieved[1].1, &k[0]),
        "夹具前提：e-unqualified 违反 K",
    );

    let r = resolve(&retrieved, None, &k, &conf("high"), &req("high"));
    let x = match &r {
        ResolutionResult::Resolved(x) => x,
        other => panic!("恰一个合格候选、门槛已过应落 `Resolved`，实得 {other:?}"),
    };
    assert_eq!(
        x.entity().as_str(),
        "e-qualified",
        "断言 D 前半：解出的是**合格**的那一枚（合格与否由非空约束 `K` 判定）",
    );
    assert_eq!(
        x.satisfied().len(),
        k.len(),
        "`satisfied` **照录**判定时用的那一组约束（D 的判据要能由调用方自己核）",
    );
    // —— A 不足以替代 D 的证据 ——
    assert!(
        a_holds(&EntityId::from_str("e-unqualified"), &retrieved, None),
        "这一行断言 A 对「绑到不合格的那一枚」**不响**（它也在检索集里）\
         —— 故 D 是独立的一条",
    );
}

/// **断言 D 第 2 条**（§4.2）：多个合格候选 ⟹ **不是 `Resolved`**。
///
/// 夹具：**同一条非空约束 `K = [Scope(Project)]`**；候选 `{c1 满足 K, c2 满足 K, c3 违反 K}`。
/// 两条断言：
///
/// 1. 结果是 [`ResolutionResult::Ambiguous`] 而**不是** `Resolved(c1)`；
/// 2. `Ambiguous.surviving` **恰是 `{c1, c2}`**——逐项：长度为 2、两枚都在、`c3` **不在**。
///
/// # 第 2 条才是要紧的那一条（`c3` 靠它承重）
///
/// 只断言「结果是 `Ambiguous`」时 **`c3` 不承重**：一个把「合格」判成**恒真**的实现
/// 会让 `{c1, c2, c3}` 全合格，结果**仍是 `Ambiguous`**，用例照样绿。
/// `c3` 是对照组，靠第 2 条兑现——这也是 `Ambiguous` 必须带 `surviving` 的原因。
///
/// # 为什么是**逐项相等**而不是集合相等
///
/// 集合相等会让「把 `surviving` 的顺序交给 `HashSet` 迭代」成为**等价变异体**
/// （顺序变了而集合没变——那正是本仓纪律 1(b) 点名的假绿）。
/// 逐项相等同时钉住「按输入次序」：夹具的 id 就取作 `c1`／`c2`／`c3`，
/// 故输入次序与 **id 升序**在这一份夹具里是同一个次序。
///
/// 红条件（两枚真变异体）：
/// - **移除档**：把「合格」判成恒真（`qualifying` 不过滤）⟹ `surviving` 多出 `c3`，第 2 条红；
/// - **取反档**：把 `surviving` 的顺序交给无序容器迭代 ⟹ 第 2 条的逐项相等红。
///   实测见报告 §变异轮（含它对两枚元素是否稳定变红的**照实记录**）。
#[test]
fn assertion_d_2_multiple_qualified_candidates_give_ambiguous() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![
        (
            cand("c1", Some(0.90)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("c2", Some(0.80)),
            facts(ResolutionScope::Project, EntityKind::Skill),
        ),
        (
            cand("c3", Some(0.99)),
            facts(ResolutionScope::Global, EntityKind::Tool),
        ),
    ];
    // 夹具前提：前两枚满足 K、第三枚违反 K（`c3` 是**对照组**，它承重靠下面第 2 条）。
    assert!(
        satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：c1 满足 K",
    );
    assert!(
        satisfies(&retrieved[1].0, &retrieved[1].1, &k[0]),
        "夹具前提：c2 满足 K",
    );
    assert!(
        !satisfies(&retrieved[2].0, &retrieved[2].1, &k[0]),
        "夹具前提：c3 **违反** K（它分数最高，故它正是「按分数选」会选中的那一枚）",
    );

    let r = resolve(&retrieved, None, &k, &conf("high"), &req("high"));

    // —— 第 1 条 ——
    assert!(
        !matches!(&r, ResolutionResult::Resolved(_)),
        "断言 D 后半：两个合格候选不许产出 `Resolved`，实得 {r:?}",
    );
    let amb = match &r {
        ResolutionResult::Ambiguous(a) => a,
        other => panic!("两个合格候选应落 `Ambiguous`，实得 {other:?}"),
    };

    // —— 第 2 条（要紧的那一条）——
    let surviving = amb.surviving();
    assert_eq!(
        surviving.len(),
        2,
        "`surviving` 恰是合格的那**两枚**（`c3` 不在其中）",
    );
    assert_eq!(
        ids(surviving),
        vec!["c1", "c2"],
        "逐项相等（不是集合相等）：长度 2、两枚都在、**按输入次序**\
         ——夹具的 id 取作 c1／c2／c3，故它同时也是 **id 升序**",
    );
    assert!(
        !surviving
            .iter()
            .any(|c| c.entity_id.as_str() == "c3"),
        "`c3` **不在** `surviving` 里——它违反 K，是本用例的对照组",
    );
    assert_eq!(
        amb.candidate_count(),
        retrieved.len(),
        "`candidate_count` 是判定**输入**的规模（3），不是合格数（2）",
    );
}

/// **守卫的另一侧**：候选集非空且**唯一**合格 ⟹ `Resolved`（不许落 `Unknown`）。
///
/// # 为什么这条不能省
///
/// 只钉「拒绑」那一侧（A／B／D）会让一个**永远返回 `Unknown`** 的解析器**照样全绿**
/// ——它是 fail-open 的对侧，正是本仓「守卫须两侧都钉」的缺的那一侧。
///
/// 夹具：`K = [Scope(Project)]`（非空），三枚候选里**恰一枚**满足它——
/// 另两枚的 scope 各是别的值（`Global` / `Session`）。门槛已过，故结论是
/// [`ResolutionResult::Resolved`]，且解出的正是那一枚。
///
/// # 另两枚为什么不是「各违反种不同的约束」
///
/// 这一份 `K` 只含 `Scope` 一条，故「违反」只有一种形态（scope 不等于 `Project`）——
/// 两枚违反者靠**各自的 scope 取值**相区分。**不为此再加一条约束**：本用例钉的是
/// 「恰一个合格要产出」这一侧，`K` 再宽只会让「恰一个」更难核对。
///
/// 红条件（**移除档**）：让「没有候选合格」与「恰一个候选合格」走同一支
/// （例如把 `qualifying` 的结果长度恒判成 0）⟹ 本用例红。实测见报告 §变异轮。
#[test]
fn the_other_side_a_nonempty_set_with_exactly_one_qualified_candidate_is_resolved() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![
        (
            cand("e-only", Some(0.40)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-global", Some(0.95)),
            facts(ResolutionScope::Global, EntityKind::Tool),
        ),
        (
            cand("e-session", Some(0.20)),
            facts(ResolutionScope::Session, EntityKind::Skill),
        ),
    ];
    // 夹具前提：**恰一枚**合格——这一条由 `satisfies` 现算，不是假定的。
    let qualified: Vec<&str> = retrieved
        .iter()
        .filter(|(c, f)| satisfies(c, f, &k[0]))
        .map(|(c, _)| c.entity_id.as_str())
        .collect();
    assert_eq!(
        qualified,
        vec!["e-only"],
        "夹具前提：三枚里**恰一枚**（e-only）满足 K",
    );

    let r = resolve(&retrieved, None, &k, &conf("high"), &req("high"));
    let x = match &r {
        ResolutionResult::Resolved(x) => x,
        other => panic!(
            "守卫的另一侧：候选集非空且唯一合格、门槛已过 ⟹ 必须落 `Resolved`，\
             实得 {other:?}（一个永远返回 `Unknown` 的解析器会在这里红）",
        ),
    };
    assert_eq!(
        x.entity().as_str(),
        "e-only",
        "解出的正是唯一合格的那一枚",
    );
    assert_eq!(
        x.candidate_count(),
        retrieved.len(),
        "`candidate_count` 是判定**输入**的规模（3），不是合格数（1）",
    );
}
