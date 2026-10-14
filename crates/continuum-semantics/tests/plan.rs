//! §228 的 Plan、§229／§338 的审查状态机、§339／§340 的审查记录与 §341 的八项视图
//! 的用例集（设计 §9.1／§9.2／§9.3／§9.5）。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `plan_carries_exactly_the_ten_fields_of_228` | §228 的十字段清单**逐项**（穷尽解构，**不带 `..`**） | 加／少／改名一个字段即**编译不过**（**编译期照片**，运行期无照片——纯数据声明） |
//! | `plan_version_is_two_parts` | §9.1 的两段式：MATERIAL ⇒ `major` +1（`minor` 归零）；MINOR ⇒ `minor` +1（**两侧都钉**） | 只 +1 一个数（**收紧档**——(a) 或 (b) 必有一侧红） |
//! | `every_listed_review_pair_is_accepted` | §9.2 迁移表的**九对逐条** `Ok`（不抽代表） | 少列一对即红 |
//! | `unlisted_review_pairs_are_rejected` | 未列出的对（终态出边、跳步、回退、**六态自环全数**）各得 `Err(IllegalReview { from, to })` 且**原样带出那一对** | 把某条未列出的边放行（**放宽档**——那一条红） |
//! | `a_strategic_plan_cannot_be_executed_without_user_approval` | §229 的执行门**一侧**：`STRATEGIC` 未经 `USER_APPROVED` 得 `Err(UserApprovalRequired)`；**反面**：经 `USER_APPROVED` 得 `Ok` | 门不再拦（**移除档**）；或对一切态恒拦（反面那半边红） |
//! | `an_execution_plan_reaches_ready_without_user_approval` | **另一侧，不能省**：`EXECUTION` 同路径得 `Ok`（§229 的 MUST 只对 Strategic Plan，§341 把用户审查限定在战略层） | 把 MUST 套到所有 Plan（**收紧档**——本用例红） |
//! | `a_revision_is_a_new_version_and_the_old_one_becomes_superseded` | §9.2：「修订」**不是第六个态**——**产生新版本**（`DRAFT`、major +1）**＋旧版本转 `SUPERSEDED`**（一条用例断言两件事都成立） | 把修订实现成原地改状态（**移除档**——两件里至少一件红） |
//! | `every_review_finding_severity_has_a_photo` | §339 的四 severity **四值各一条**：各构造一枚 `ReviewFinding` 并断言是**哪一枚**（判别式读出经穷尽 `match`、不带 `..`） | 四值合并／少一枚即**编译不过**（判别式）；构造路径错即红 |
//! | `a_review_finding_record_carries_the_independence_facts` | §340：`ReviewFindingRecord` **必带** `ReviewIndependence`；两者的字段清单**逐项**（穷尽解构，**不带 `..`**） | 少一个字段即**编译不过** |
//! | `a_same_model_same_context_review_is_observable_as_such` | §340 的落点：一次「同一模型 + 同一上下文」的审查**可观察**——`separate_context == false` **且**两个 model ref 相同（**两条断言都写，不靠一条顺带**），且两个串**逐字节**读回（`from_str` 的「不改写、不裁剪」由此有直接照片） | 把事实抹掉（字段被改写）／`from_str` 裁剪即红；**本层不禁止这种审查**（它是 SHOULD） |
//! | `the_user_review_view_has_exactly_the_eight_lines_of_341` | §341 的八项**逐项**（穷尽解构，**不带 `..`**）——**多一项即红、少一项即红、`成本` 与 `权限` 合并成一项即红** | 加／少／改名／合并即**编译不过**（负向守卫，本仓接受否定式照片） |
//! | `the_user_review_view_has_no_field_pointing_at_internal_steps` | **否定式照片**：`src/review_view.rs` 里 `UserReviewView` 的字段集合**恰为八项**（源码面运行期对钉——编译期那半边在上一条） | 源码里多／少／改名／合并一个字段而清单未同步即红（词法扫描的限度写在用例里） |
//! | `the_plan_table_has_exactly_the_expected_columns` | `PRAGMA table_info(plan)` 实际列 vs 手写期望，**逐列** | 加／少／改一列即红 |
//! | `enum_codes_use_lowercase_underscore_and_reject_out_of_table_values` | `PlanType` 两枚、`ReviewState` 六枚、`Severity` 四枚的落库字面量**逐枚**（小写、多词接 `_`）＋ `parse` 的定义域（表外取值得 `None`） | 编码改成大写／去掉 `_`／与邻枚编成同一个串／表外取值被猜成某一枚即红 |
//!
//! # 两处读法申报（写在用例里，收件人：复审者）
//!
//! - **两段式版本是设计的决定**（§9.1）：§228 只给 `version` 一个字段名；`major` 在
//!   MATERIAL 时 +1、`minor` 在 MINOR 时 +1。**「major +1 时 minor 归零」是本层从 §213
//!   的例子读出来的**（「重大变化：`Plan v2`」——从 `v1.x` 出发的 major 版写作 `v2`
//!   而不是 `v2.x`），§228／§9.1 只写了「+1」。
//! - **修订两件事的断言形态**（§9.2）：修订是**跨版本**的动作（产生新版本 + 旧版本进
//!   `SUPERSEDED`），不是同版本迁移；同版本迁移表里那对 `AI_REVIEWED → DRAFT`（设计的
//!   批注「修订，见下」）照表录收（`every_listed_review_pair_is_accepted` 里各条），
//!   跨版本的那一半落在 `revise`（本文件的第七条用例）。
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/contract.rs` 的夹具同形（P0 内建迁移 ＋ 本 crate 迁移 110，
//! 建库后必须 `migrate()`）；`sample_plan` **每个字段都取一个非默认值**，好让
//! 「哪个字段没入表／与邻字段写串」这类错在读回比对里现形（本条 task 只建表，
//! 行级读写是后续 task 的——**本文件里库只用于表列清单**）。
//!
//! # §340 的判定没有照片（照实写在此处）
//!
//! §340 的三条独立性是 **SHOULD**、「分离到什么程度算够」**规范没给**（§16 第 14 条）——
//! 故本层**不禁止**「同模型 + 同上下文」的审查，只**使事实可观察**；那一条判定
//! **没有照片**，本文件也不为它发明一条。

use std::fs;
use std::path::{Path, PathBuf};

use continuum_canonical::Opaque;
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};
use continuum_semantics::{
    IntentId, ModelRefRef, Plan, PlanError, PlanId, PlanType, PlanVersion, ReviewFinding,
    ReviewFindingRecord, ReviewState, ReviewIndependence, Severity, UserReviewView,
    p4_semantics_migrations, revise,
};

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移的临时库（与 `tests/contract.rs` 的夹具同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条基准 Plan：**每个字段都取一个非默认值**（五个列表都与彼此不同）。
fn sample_plan(id: &str) -> Plan {
    Plan {
        id: PlanId::from_str(id),
        version: PlanVersion { major: 1, minor: 0 },
        intent_id: IntentId::from_str(&format!("i-for-{id}")),
        r#type: PlanType::Strategic,
        objectives: vec![Opaque::from_str("obj-1"), Opaque::from_str("obj-2")],
        milestones: vec![Opaque::from_str("ms-1")],
        assumptions: vec![Opaque::from_str("asm-1")],
        risks: vec![Opaque::from_str("risk-1")],
        acceptance_conditions: vec![Opaque::from_str("ac-1")],
        review_state: ReviewState::Draft,
    }
}

/// 一条基准 ReviewFinding：五个字段各取一个非默认值。
fn sample_finding(severity: Severity) -> ReviewFinding {
    ReviewFinding {
        severity,
        subject: Opaque::from_str("subject-1"),
        problem: Opaque::from_str("problem-1"),
        evidence: Opaque::from_str("evidence-1"),
        recommendation: Opaque::from_str("recommendation-1"),
    }
}

/// `PRAGMA table_info(table)` → `(列名, 类型, NOT NULL, 主键位次)`，按建表序。
/// 与 `tests/contract.rs` 的同名夹具逐字相同（列清单用例的判据形态）。
fn columns(tx: &Tx<'_>, table: &str) -> Vec<(String, String, i64, i64)> {
    tx.query(&format!("PRAGMA table_info({table})"), &[])
        .unwrap()
        .iter()
        .map(|row| {
            let text = |at: usize| match &row[at] {
                Value::Text(s) => s.clone(),
                other => panic!("table_info 第 {at} 列应是文本，实得 {other:?}"),
            };
            let int = |at: usize| match &row[at] {
                Value::Int(i) => *i,
                other => panic!("table_info 第 {at} 列应是整数，实得 {other:?}"),
            };
            (text(1), text(2), int(3), int(5))
        })
        .collect()
}

/// §228 的十字段清单**逐项**：机制 = 穷尽解构、**不带 `..`**。
///
/// 加／少／改名一个字段即**编译不过**（按本仓纪律，编译不过不算「变红」，
/// 故这里是**编译期照片**：运行期断言为空，承重在编译器）。同一机制在本仓的
/// 先例：`tests/intent.rs` 的 `intent_carries_exactly_the_fifteen_fields_of_222`。
///
/// §228 的字段名逐字照录（`id` / `version` / `intent_id` / `type` / `objectives[]` /
/// `milestones[]` / `assumptions[]` / `risks[]` / `acceptance_conditions[]` /
/// `review_state`）；`type` 是 Rust 关键字，用 raw identifier `r#type` 承载——
/// 字段名**没有**改成 `plan_type`（那会让「照 §228」这句话变假）。
#[test]
fn plan_carries_exactly_the_ten_fields_of_228() {
    let plan = sample_plan("p-fields");
    let Plan {
        id,
        version,
        intent_id,
        r#type,
        objectives,
        milestones,
        assumptions,
        risks,
        acceptance_conditions,
        review_state,
    } = plan;
    // `let _ = (…)` 只是把绑定消费掉，免得 unused-variable 警告。
    let _ = (
        id,
        version,
        intent_id,
        r#type,
        objectives,
        milestones,
        assumptions,
        risks,
        acceptance_conditions,
        review_state,
    );
}

/// §9.1 的两段式版本（**本设计的决定**，不是规范转录——§228 只给 `version` 一个
/// 字段名；收件人：复审者，「`## 遗留` 照抄」见任务简报）：
///
/// - (a) `MATERIAL` 变更 ⇒ `major` +1（`minor` 归零——**读法申报**：§213 的
///   「重大变化：`Plan v2`」是从 `v1.x` 链上写出来的 major 版，若 `minor` 不归零，
///   它应写作 `v2.x`；§228／§9.1 只写了「+1」，归零这一半是本层从 §213 的例子读出来的）；
/// - (b) `MINOR` 变更 ⇒ `minor` +1（`major` 不动）。
///
/// 红条件：只 +1 一个数（**收紧档**——(a) 或 (b) 必有一侧红）。
#[test]
fn plan_version_is_two_parts() {
    let v = PlanVersion { major: 1, minor: 2 };

    assert_eq!(
        v.next_material(),
        PlanVersion { major: 2, minor: 0 },
        "MATERIAL 变更应 major +1（minor 归零）——只 +1 一个数的实现（如改 minor）在这里红"
    );
    assert_eq!(
        v.next_minor(),
        PlanVersion { major: 1, minor: 3 },
        "MINOR 变更应 minor +1（major 不动）——只 +1 一个数的实现在另一侧红"
    );
}

/// §9.2 的迁移表**逐条** `Ok`（九对，不抽代表）：
///
/// | 从 | 到 |
/// |---|---|
/// | `DRAFT` | `SELF_REVIEWED`、`REJECTED` |
/// | `SELF_REVIEWED` | `AI_REVIEWED`、`DRAFT`、`REJECTED` |
/// | `AI_REVIEWED` | `USER_APPROVED`、`DRAFT`、`REJECTED` |
/// | `USER_APPROVED` | `SUPERSEDED` |
///
/// （`REJECTED` / `SUPERSEDED` 是终态，没有出边——它们的照片在
/// `unlisted_review_pairs_are_rejected` 里。）
///
/// **表里两对与「修订」的关系照实申报**：`AI_REVIEWED → DRAFT` 带设计批注
/// 「（修订，见下）」而「见下」段说的是**跨版本**的修订（新版本 + 旧版本
/// `SUPERSEDED`）；本层把表**照录**（这九对逐条 `Ok`），跨版本的那一半落在
/// `a_revision_is_a_new_version_and_the_old_one_becomes_superseded` 的 `revise`。
/// 两者不互相顶替：表里那一对是「一个版本内」的判定，`revise` 是「产生新版本」。
#[test]
fn every_listed_review_pair_is_accepted() {
    use ReviewState::*;

    let listed = [
        (Draft, SelfReviewed),
        (Draft, Rejected),
        (SelfReviewed, AiReviewed),
        (SelfReviewed, Draft),
        (SelfReviewed, Rejected),
        (AiReviewed, UserApproved),
        (AiReviewed, Draft),
        (AiReviewed, Rejected),
        (UserApproved, Superseded),
    ];
    assert_eq!(
        listed.len(),
        9,
        "§9.2 的表是九对（逐条清单与表一一对应——多／少一条都会让本断言或下面逐条断言红）"
    );
    for (from, to) in listed {
        assert_eq!(
            ReviewState::transition(from, to),
            Ok(()),
            "§9.2 列出的对 {from:?} → {to:?} 应被接受"
        );
    }
}

/// 未列出的对得 `Err(IllegalReview { from, to })`，**原样带出那一对**。
///
/// 覆盖四类：终态的出边、跳步、回退、**六态自环全数**（本表没有自环——
/// 与 §223 的 intent 表不同，那里 `RUNNING` / `WAITING` 两条自环是 `Ok` 的）。
///
/// **`(AI_REVIEWED, SUPERSEDED)` 在列、且预期 `Err`——这是刻意的**：
/// §9.2 的「旧版本转 `SUPERSEDED`」发生在**修订**（跨版本动作）里，
/// 不是同版本迁移表的一条边；本对因此是「未列出的对」。
#[test]
fn unlisted_review_pairs_are_rejected() {
    use ReviewState::*;

    let unlisted = [
        // 终态没有出边：
        (Rejected, Draft),
        (Rejected, SelfReviewed),
        (Rejected, Superseded),
        (Superseded, Draft),
        (Superseded, Rejected),
        // 跳步（不在表里）：
        (Draft, AiReviewed),
        (Draft, UserApproved),
        (SelfReviewed, UserApproved),
        // 回退只剩表里列的两处（SELF_REVIEWED → DRAFT、AI_REVIEWED → DRAFT 在上一条里打过了）：
        (UserApproved, Draft),
        (UserApproved, Rejected),
        // 修订的旧版本转 SUPERSEDED 不走同版本迁移（见下注），所以它是未列出的对：
        (AiReviewed, Superseded),
        (Draft, Superseded),
        // 六态自环全部非法（本表没有自环）：
        (Draft, Draft),
        (SelfReviewed, SelfReviewed),
        (AiReviewed, AiReviewed),
        (UserApproved, UserApproved),
        (Rejected, Rejected),
        (Superseded, Superseded),
    ];
    for (from, to) in unlisted {
        assert_eq!(
            ReviewState::transition(from, to),
            Err(PlanError::IllegalReview { from, to }),
            "未列出的对 {from:?} → {to:?} 应得 IllegalReview（from／to 原样带出）"
        );
    }
}

/// §229 的执行门**一侧**：`STRATEGIC` 未经 `USER_APPROVED` 不得执行
/// ——「去 READY」得 `Err(UserApprovalRequired)`。
///
/// **反面（守卫的另一半）也在本用例里**：已经 `USER_APPROVED` 的 `STRATEGIC`
/// 得 `Ok`——只钉拒绝那一侧会让「永远拒绝」的实现照样绿。
///
/// §229 原文的「在需要**重大用户承诺**时」这一条件**规范没给判据**；
/// 设计把它读作「Strategic Plan 一律要求 `USER_APPROVED`」（§9.2），本层照录。
#[test]
fn a_strategic_plan_cannot_be_executed_without_user_approval() {
    assert_eq!(
        ReviewState::Draft.reaches_ready(PlanType::Strategic),
        Err(PlanError::UserApprovalRequired {
            plan_type: PlanType::Strategic,
            state: ReviewState::Draft,
        }),
        "STRATEGIC 未经 USER_APPROVED 去 READY 应得 UserApprovalRequired（带出那一枚 plan_type 与 state）"
    );
    assert_eq!(
        ReviewState::AiReviewed.reaches_ready(PlanType::Strategic),
        Err(PlanError::UserApprovalRequired {
            plan_type: PlanType::Strategic,
            state: ReviewState::AiReviewed,
        }),
        "审查后（AI_REVIEWED）但未经用户批准，同样不得执行"
    );
    assert_eq!(
        ReviewState::UserApproved.reaches_ready(PlanType::Strategic),
        Ok(()),
        "经 USER_APPROVED 的 STRATEGIC 应可执行（只钉拒绝那侧会让「永远拒绝」漂过去）"
    );
}

/// **另一侧，不能省**：`EXECUTION` 同路径得 `Ok`。
///
/// §229 的 MUST 只对 Strategic Plan（「Strategic Plan MUST 在需要重大用户承诺时
/// 进入 USER_APPROVED 才可以执行」），而 §341 把用户审查的范围限定在战略层——
/// **`type = EXECUTION` 的 Plan 不进 `USER_APPROVED`**，故这道门不拦它。
///
/// 红条件：把 MUST 套到所有 Plan（**收紧档**——本用例红）。
/// **限度照实写**：本门只落实 §229 这一条；「终态（`REJECTED` / `SUPERSEDED`）不可
/// 执行」之类的更严判定**没有规范来源**，本层不为它发明判据（若将来要收紧，那是一处
/// 要带判据来的改）。
#[test]
fn an_execution_plan_reaches_ready_without_user_approval() {
    assert_eq!(
        ReviewState::Draft.reaches_ready(PlanType::Execution),
        Ok(()),
        "EXECUTION 同路径不应被 USER_APPROVED 门拦下"
    );
}

/// §9.2：「修订」**不是第六个态**——**产生新版本（`DRAFT`）＋旧版本转 `SUPERSEDED`**。
/// 一条用例，断言**两件事都成立**（不靠一件顺带）。
///
/// §338 的阶梯是 `草稿 → 自审 → 独立强 AI 审查 → 修订 → 用户审查 → 批准`；
/// 「修订」是**一次新版本**（§213 的版本化），不是原地改状态。修订属 MATERIAL 档——
/// 推导链：修订后重新从 `DRAFT` 走审查阶梯 ⇒ 重开审查门，而 §230 的 MATERIAL
/// **MUST** 重触发审查门、§213 把「重大变化」写作 `Plan v2`（`REVIEW_REQUIRED`）。
///
/// 红条件：把修订实现成原地改状态（**移除档**——若实现只把旧版本的状态改成别的、
/// 不产生 `major` +1 的新版本，第一件红；若新版本没把旧版本标 `SUPERSEDED`，第二件红）。
#[test]
fn a_revision_is_a_new_version_and_the_old_one_becomes_superseded() {
    let mut old = sample_plan("p-revise");
    old.version = PlanVersion { major: 5, minor: 3 };
    old.review_state = ReviewState::AiReviewed;

    let (revised, superseded) = revise(&old);

    // 第一件：产生新版本（`DRAFT`）——同一个 Plan id 的新版本，不是把旧版本原地改态。
    assert_eq!(
        revised.id.as_str(),
        old.id.as_str(),
        "修订产生的是同一个 Plan id 的新版本"
    );
    assert_eq!(
        revised.version,
        PlanVersion { major: 6, minor: 0 },
        "修订是 MATERIAL 档：major +1、minor 归零（读法见用例文档）"
    );
    assert_eq!(
        revised.review_state,
        ReviewState::Draft,
        "新版本从 DRAFT 开始（§338 的循环）"
    );
    assert_eq!(
        revised.objectives.len(),
        old.objectives.len(),
        "修订产出的版本带着原内容（内容的修改由调用方随后进行——本函数不发明内容）"
    );

    // 第二件：旧版本转 SUPERSEDED（终态）。版本号不变——被取代的是原来那一版。
    assert_eq!(
        superseded.review_state,
        ReviewState::Superseded,
        "旧版本应进 SUPERSEDED"
    );
    assert_eq!(superseded.version, old.version, "被取代的版本号不应被改写");
}

/// §339 的四 severity **四值各一条**：各构造一枚 `ReviewFinding`、断言是**哪一枚**。
///
/// 「哪一枚」经一个**穷尽 `match`、不带 `..`** 的判别式读出——加第五枚即**编译不过**
/// （编译期那一半）；运行期这一半是四值各自的构造与断言。
#[test]
fn every_review_finding_severity_has_a_photo() {
    /// 判别式读出。**无通配臂**：加第五枚 severity 即编译不过。
    fn severity_name(finding: &ReviewFinding) -> &'static str {
        match finding.severity {
            Severity::Blocker => "Blocker",
            Severity::Major => "Major",
            Severity::Minor => "Minor",
            Severity::Suggestion => "Suggestion",
        }
    }

    assert_eq!(
        severity_name(&sample_finding(Severity::Blocker)),
        "Blocker",
        "BLOCKER 应可构造、且是 Blocker 那一枚"
    );
    assert_eq!(
        severity_name(&sample_finding(Severity::Major)),
        "Major",
        "MAJOR 应可构造、且是 Major 那一枚"
    );
    assert_eq!(
        severity_name(&sample_finding(Severity::Minor)),
        "Minor",
        "MINOR 应可构造、且是 Minor 那一枚"
    );
    assert_eq!(
        severity_name(&sample_finding(Severity::Suggestion)),
        "Suggestion",
        "SUGGESTION 应可构造、且是 Suggestion 那一枚"
    );
}

/// §340：`ReviewFindingRecord` **必带** `ReviewIndependence`；两者的字段清单**逐项**
/// （穷尽解构，**不带 `..`**——少一个字段即**编译不过**）。
///
/// 它不是「布尔标志位」，是**三条各带来源的记录**（§9.3）：
/// `separate_context` / `reviewer_model` / `author_model` / `cross_family`。
#[test]
fn a_review_finding_record_carries_the_independence_facts() {
    let record = ReviewFindingRecord {
        finding: sample_finding(Severity::Major),
        independence: ReviewIndependence {
            separate_context: true,
            reviewer_model: ModelRefRef::from_str("reviewer-a"),
            author_model: ModelRefRef::from_str("author-b"),
            cross_family: true,
        },
    };

    let ReviewIndependence {
        separate_context,
        reviewer_model,
        author_model,
        cross_family,
    } = &record.independence;
    let _ = (separate_context, reviewer_model, author_model, cross_family);

    // `ReviewFindingRecord` 的字段清单**逐项**：`independence` 没有可省路径
    //（构造时必填、解构时必列）。
    let ReviewFindingRecord {
        finding,
        independence,
    } = &record;
    let _ = (finding, independence);
}

/// §340 的落点：一次「**同一模型 + 同一上下文**」的审查**可观察**——
/// 读回的 `separate_context == false` **且**两个 model ref 相同（**两条断言都写，
/// 不靠一条顺带**）。
///
/// **本层不禁止这种审查**：§340 的三条独立性是 SHOULD（自审 MAY 同一模型；
/// 独立审查 SHOULD 独立上下文／更强模型／必要时跨 family），
/// **「分离到什么程度算够」规范没给判据**（§16 第 14 条）——故判定不落本层，
/// 本层只**使事实可观察**。那一条判定**没有照片**（本用例不冒充它）。
#[test]
fn a_same_model_same_context_review_is_observable_as_such() {
    // 两个 model ref 用**带空白与全角字符**的同一个串构造：读回逐字节比对时，
    // 任何裁剪／改写都会在下面两处之一显形（`ModelRefRef` 的「不改写、不裁剪」
    // 由此有直接照片）。
    let raw = " model-x 模型（含空白） ";
    let record = ReviewFindingRecord {
        finding: sample_finding(Severity::Minor),
        independence: ReviewIndependence {
            separate_context: false,
            reviewer_model: ModelRefRef::from_str(raw),
            author_model: ModelRefRef::from_str(raw),
            cross_family: false,
        },
    };

    assert!(
        !record.independence.separate_context,
        "同一上下文：读回的 separate_context 应是 false（事实可观察）"
    );
    assert_eq!(
        record.independence.reviewer_model.as_str(),
        record.independence.author_model.as_str(),
        "同一模型：两个 model ref 应读回相同的串（事实可观察）"
    );
    assert_eq!(
        record.independence.reviewer_model.as_str(),
        raw,
        "两个 model ref 应逐字节读回构造时给的串（`from_str` 不改写、不裁剪）"
    );
}

/// §341 的用户审查视图：字段清单**逐项八条**——**多一项即红、少一项即红、
/// `成本` 与 `权限` 合并成一项即红**（三侧）。机制 = 穷尽解构**不带 `..`**：
/// 多一项／改名／合并都**编译不过**。
///
/// # 这一条的来历（写进注释，见任务简报）
///
/// 设计初稿把 `成本` 与 `权限` 合并成一个字段 `cost_and_authority`，并写
/// 「七项（原文如此，第七项与第八项合并计为七类）」——**那句话是假的**：
/// §341 写的是**八行**；**出错的恰是「少一项」那一侧**（初稿只写「多一项即红」时，
/// 它守不住自己的错）。§9.5 的订正段与 §14 的同一条守卫（三侧写全）留了记载；
/// 本条的三侧断言就是那次教训的落点。
///
/// **这是负向守卫，本仓接受否定式照片**（`docs/superpowers/p3bcdf-followups.md` §四.4）。
/// 运行期的第二半（源码面字段集合对钉）在
/// `the_user_review_view_has_no_field_pointing_at_internal_steps`。
///
/// **本组件不是 UI**：§2.1 的组件表里没有「用户界面」组件，本层只产出这个视图；
/// 它的呈现方不在本层（本用例只钉类型，不钉任何呈现）。
#[test]
fn the_user_review_view_has_exactly_the_eight_lines_of_341() {
    let view = UserReviewView {
        objectives: Opaque::from_str("目标一"),
        major_designs: Opaque::from_str("重大设计一"),
        key_constraints: Opaque::from_str("关键约束一"),
        milestones: Opaque::from_str("Milestone 一"),
        risks: Opaque::from_str("风险一"),
        cost: Opaque::from_str("成本一"),
        authority: Opaque::from_str("权限一"),
        acceptance_conditions: Opaque::from_str("验收条件一"),
    };

    let UserReviewView {
        objectives,
        major_designs,
        key_constraints,
        milestones,
        risks,
        cost,
        authority,
        acceptance_conditions,
    } = view;
    let _ = (
        objectives,
        major_designs,
        key_constraints,
        milestones,
        risks,
        cost,
        authority,
        acceptance_conditions,
    );
}

/// **否定式照片**：八项里**没有任何一项**承载 Plan 的内部执行步骤——
/// §341 的清单是这个视图的**全部**判据。
///
/// # 机制与它的限度（写在用例里）
///
/// 「没有指向内部执行步骤的字段」是一个**否定命题**，而「内部执行步骤」没有固定的
/// 名字清单（开放集：`steps` / `internal_steps` / `nodes` / `schedule` …枚举不完）。
/// 本用例的可判形态是**源码面字段集合对钉**：扫本 crate 的 `src/review_view.rs`，
/// 取 `UserReviewView` 结构体的字段名集合，与**手写八项清单**比较——
///
/// - 新增任何字段（无论叫什么，包括承载内部步骤的那一类）⇒ 集合多一项 ⇒ **红**；
/// - 少一项／改名／把两项合并成一项 ⇒ 集合对不上 ⇒ **红**。
///
/// **它不钉什么**：字段**类型**里出现「步骤」概念（例如某个字段的类型改成引用了
/// 带步骤的类型）不在本扫描的射程——那半边由编译期那张照片（上一条的穷尽解构，
/// 运行时字段的类型一旦露出 Plan 内部结构会改变构造点）与 Task 21 的复核承担。
#[test]
fn the_user_review_view_has_no_field_pointing_at_internal_steps() {
    let sources = crate_sources();
    // 先证明扫描真的读到了这棵树（否则下面的清单断言在空语料上恒真、恒绿）。
    assert!(
        sources.len() >= 5,
        "只扫到 {} 个 `src/` 下的 `.rs`：源码树没被读到",
        sources.len()
    );
    let view_source = sources
        .iter()
        .find(|(path, _)| path.ends_with("review_view.rs"))
        .map(|(_, body)| body)
        .unwrap_or_else(|| {
            panic!(
                "`src/` 下应有 `review_view.rs`（扫到 {} 个文件）",
                sources.len()
            )
        });

    let mut actual = struct_field_names(view_source, "UserReviewView");
    actual.sort();

    let mut expected: Vec<&str> = vec![
        "objectives",
        "major_designs",
        "key_constraints",
        "milestones",
        "risks",
        "cost",
        "authority",
        "acceptance_conditions",
    ];
    expected.sort();

    assert_eq!(
        actual,
        expected,
        "`UserReviewView` 的字段集合应**恰为** §341 的八项（源码面）：\
         多一项（如承载内部执行步骤的字段）／少一项／改名／合并即红——\
         §341 的清单是这个视图的全部判据"
    );
}

/// 本 crate `src/` 下的全部 `.rs`（路径排序，消息里的次序因此确定）。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**（同 `tests/contract.rs` 的
/// `crate_sources`：按相对路径扫的守卫在 crate 根下跑是绿的，换个目录跑就**读不到东西**）。
fn crate_sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读不到源文件 {}：{e}", path.display()));
            (path, source)
        })
        .collect()
}

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**（同 `tests/contract.rs`）。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("列不出目录 {}：{e}", dir.display()))
    {
        let path = entry.unwrap_or_else(|e| panic!("列目录 {} 时出错：{e}", dir.display())).path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 从源码里取 `pub struct {name} { … }` 的字段声明名（按源码顺序）。
///
/// **词法扫描的限度**：只认「`pub struct {name} {` 起始、到第一个整行 `}` 结束」的块，
/// 且只取每个字段声明行 `:` 之前的部分——折行声明的字段会漏（本仓体例里结构体字段
/// 逐行声明，且对钉的清单与源码是两份、漏了就红，不会静默通过）。
fn struct_field_names(source: &str, name: &str) -> Vec<String> {
    let header = format!("pub struct {name} {{");
    let mut in_struct = false;
    let mut fields = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if !in_struct {
            if trimmed.starts_with(&header) {
                in_struct = true;
            }
            continue;
        }
        if trimmed.starts_with('}') {
            break;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if let Some((field, _)) = trimmed.split_once(':') {
            let field = field.trim();
            let field = field.strip_prefix("pub ").unwrap_or(field);
            if !field.is_empty() {
                fields.push(field.to_owned());
            }
        }
    }
    fields
}

/// `plan` 表的列清单**逐列**（§228 的十字段 → 11 个落库列的映射）：
///
/// - `id`（主键第一段）；`version` 拆成 `major` ＋ `minor` 两列（两段式版本的落库形态，
///   §9.1）——它们连同 `id` 构成复合主键，同 `(id, major, minor)` 的二次写由主键拒绝
///   （与 `contract` 的 `(id, version)` 同一条处置，行级读写在后续 task）；
/// - `type` / `review_state` 是两个**枚举列**（编码小写、多词接 `_`——`PlanType` /
///   `ReviewState` 的 `as_str`／`parse` 与类型同址，表外取值读回具体 `Err` 的处置
///   随行级读写落地）；
/// - 五个列表列（`objectives` / `milestones` / `assumptions` / `risks` /
///   `acceptance_conditions`）是 JSON 字符串数组（同 `specification` 的五个列表列）。
///
/// 手写期望与迁移 110 的 SQL 是**两份**（这正是本用例的判别力所在：改了一处、
/// 忘了另一处即红）。**红条件**：加／少／改一列（含改类型、改 `NOT NULL`、
/// 改主键位次）即红。
#[test]
fn the_plan_table_has_exactly_the_expected_columns() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let expected: Vec<(&str, &str, i64, i64)> = vec![
        ("id", "TEXT", 1, 1),
        ("major", "INTEGER", 1, 2),
        ("minor", "INTEGER", 1, 3),
        ("intent_id", "TEXT", 1, 0),
        ("type", "TEXT", 1, 0),
        ("objectives", "TEXT", 1, 0),
        ("milestones", "TEXT", 1, 0),
        ("assumptions", "TEXT", 1, 0),
        ("risks", "TEXT", 1, 0),
        ("acceptance_conditions", "TEXT", 1, 0),
        ("review_state", "TEXT", 1, 0),
    ];
    let expected: Vec<(String, String, i64, i64)> = expected
        .into_iter()
        .map(|(n, t, nn, pk)| (n.to_owned(), t.to_owned(), nn, pk))
        .collect();

    assert_eq!(
        columns(&tx, "plan"),
        expected,
        "plan 表的列清单与手写期望不符（列名／类型／NOT NULL／主键位次任一不同即红）"
    );

    tx.commit().unwrap();
}

/// 三对枚举的落库编码：**逐枚**字面量（钉「小写、多词接 `_`」——枚举列纪律）、
/// `parse` 的往返、表外取值得 `None`（**不取默认值**；「读回具体 `Err`」的处置随
/// `plan` 的行级 I/O 落地，见 `src/plan.rs` 的模块文档）。
///
/// 这一对编码为 `pub` 的加宽申报写在 `src/plan.rs` 的 `PlanType::as_str` 文档里
/// （两条理由：本 task 的 `plan` 表无行级读写，照片只能落在集成测试；0 warning 下
/// `pub(crate)` 无消费者会触发 `dead_code`）。**收窄回 `pub(crate)` 的时点是行级 I/O
/// 落地之后**。
///
/// 红条件：把某一枚编成大写／去掉 `_`／与邻枚编成同一个串即红；`parse` 对表外
/// 取值给出某一枚（而不是 `None`）即红。
#[test]
fn enum_codes_use_lowercase_underscore_and_reject_out_of_table_values() {
    // PlanType 两枚。
    assert_eq!(PlanType::Strategic.as_str(), "strategic");
    assert_eq!(PlanType::Execution.as_str(), "execution");
    assert_eq!(PlanType::parse("strategic"), Some(PlanType::Strategic));
    assert_eq!(PlanType::parse("execution"), Some(PlanType::Execution));
    assert_eq!(PlanType::parse("nonsense"), None);

    // ReviewState 六枚。
    assert_eq!(ReviewState::Draft.as_str(), "draft");
    assert_eq!(ReviewState::SelfReviewed.as_str(), "self_reviewed");
    assert_eq!(ReviewState::AiReviewed.as_str(), "ai_reviewed");
    assert_eq!(ReviewState::UserApproved.as_str(), "user_approved");
    assert_eq!(ReviewState::Rejected.as_str(), "rejected");
    assert_eq!(ReviewState::Superseded.as_str(), "superseded");
    assert_eq!(ReviewState::parse("draft"), Some(ReviewState::Draft));
    assert_eq!(
        ReviewState::parse("self_reviewed"),
        Some(ReviewState::SelfReviewed)
    );
    assert_eq!(
        ReviewState::parse("ai_reviewed"),
        Some(ReviewState::AiReviewed)
    );
    assert_eq!(
        ReviewState::parse("user_approved"),
        Some(ReviewState::UserApproved)
    );
    assert_eq!(ReviewState::parse("rejected"), Some(ReviewState::Rejected));
    assert_eq!(
        ReviewState::parse("superseded"),
        Some(ReviewState::Superseded)
    );
    assert_eq!(ReviewState::parse("nonsense"), None);

    // Severity 四枚。
    assert_eq!(Severity::Blocker.as_str(), "blocker");
    assert_eq!(Severity::Major.as_str(), "major");
    assert_eq!(Severity::Minor.as_str(), "minor");
    assert_eq!(Severity::Suggestion.as_str(), "suggestion");
    assert_eq!(Severity::parse("blocker"), Some(Severity::Blocker));
    assert_eq!(Severity::parse("major"), Some(Severity::Major));
    assert_eq!(Severity::parse("minor"), Some(Severity::Minor));
    assert_eq!(Severity::parse("suggestion"), Some(Severity::Suggestion));
    assert_eq!(Severity::parse("nonsense"), None);
}
