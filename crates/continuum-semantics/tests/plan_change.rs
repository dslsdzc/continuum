//! §230 的 Plan Change 分类的用例集（设计 §9.4）。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `both_plan_change_classes_have_a_photo` | `MINOR` / `MATERIAL` **各一条**照片；判别式读出经穷尽 `match`、**不带 `..`** | 加第三枚即**编译不过**（判别式）；两张照片的任一构造走错路径即红 |
//! | `each_of_the_ten_examples_of_230_classifies_as_the_spec_says` | §230 的十个例**逐项一条**（五个 MINOR ＋ 五个 MATERIAL，**不抽代表**）；表长恰 10 | 把 MATERIAL 的某一例判成 MINOR（**放宽档**——那一行红） |
//! | `a_model_swap_is_a_minor_change` | **直接调用**（不经表）：MINOR **首例**「模型替换」——两 Plan 与两 footprint 逐字段相等 | 同路径误判即红 |
//! | `a_major_architecture_change_is_material` | **直接调用**（不经表）：MATERIAL **首例**「重大架构变化」——计划内容变化 | 同路径误判即红 |
//! | `a_risks_only_change_is_material` | **直接调用**（不经表）：`risks` **单独**变化（其余不动）→ `Material`——钉五列表析取的 **risks 那一臂**（fix 轮 1 的评审 Important） | 删掉 risks 那一支（风险单独变化漏判 MINOR）即红 |
//! | `an_assumptions_only_change_is_material` | **直接调用**（不经表）：`assumptions` **单独**变化 → `Material`——钉 **assumptions 那一臂** | 删掉 assumptions 那一支即红 |
//! | `a_list_length_change_is_material` | **直接调用**（不经表）：某列表**追加一项**（长度变化）→ `Material`——钉 `same_list` 的 **`len` 比较那一臂**（`zip` 只比到短侧） | `len` 比较恒真（追加项被漏判、套件其余全绿）即红 |
//! | `a_material_change_re_triggers_the_review_gate` | §230 的 MUST：MATERIAL 产出**新版本**（major +1）且 `review_state == DRAFT`；**反面**：MINOR 不重触发（minor +1、审查态保持）。**两侧都钉** | MATERIAL 不重置审查门（**移除档**）；或 MINOR 也重置（反面那半边红） |
//! | `affected_scope_is_opaque` | §230 未给 `affected_scope` 的取值域 → 不透明值：**表外取值**逐字节往返；字段清单逐项（穷尽解构、不带 `..`） | 把取值域写死（拒表外取值）／值被改写即红 |
//! | `the_classification_reads_both_plans_and_both_footprints` | **输入面签名**：`classify` 的 `fn` 指针标注（两份 `Plan` ＋ 两份 `PlanFootprint` ＋ 阈值入参——R-B 的第五参）；两个阈值判据**真的被调用**（本层不自己比较） | 去掉阈值入参（回到四参）即**编译不过**；阈值判据被忽略（恒真判据不生效）即红 |
//!
//! # 用例数写死：十个例 → 十条行
//!
//! **数的来历**（设计 §9.4 的订正段，2026-10-06 评审查出，原句照抄）：
//!
//! > 初稿写「**十一个**例（五个 MINOR、五个 MATERIAL，原文实为五个 MATERIAL）」。
//! > **「十一」与括号里自己写的「五个＋五个」相抵**，括号那半句本身也读不通
//! > （原文本来就是两个五）。错因是把数数错了又用一句解释去圆它。**原句留在此处。**
//!
//! §230 的原文（`docs/spec/05-normative.md:403-417`）就是 **MINOR 五个 ＋ MATERIAL 五个**：
//! 「模型替换 / 重试 / 并行调整 / 工具替换 / 内部步骤重排」与「重大架构变化 /
//! Requirement 改变 / 权限扩大 / 成本显著扩大 / Milestone 目标变化」。
//!
//! # 表驱动的限度与五条直接调用（照实写）
//!
//! **「表驱动用例也可能覆盖错路径」**（本仓已付过代价）：十条行共用一种构造形态时，
//! 一条走对不必保证另一条也走对——**故另加五条「直接调用分类函数」的用例**（不经表）：
//!
//! - 两个 class 的**第一条**例各一条：`a_model_swap_is_a_minor_change`（「模型替换」）、
//!   `a_major_architecture_change_is_material`（「重大架构变化」）；
//! - **三条臂级照片**（fix 轮 1 的评审 Important 补，2026-10-14）：十行表只覆盖了
//!   objectives / acceptance_conditions / milestones 三个列表的**等长替换**；
//!   `a_risks_only_change_is_material` 与 `an_assumptions_only_change_is_material`
//!   钉五列表析取里 risks / assumptions 两支，`a_list_length_change_is_material`
//!   钉 `same_list` 的**长度那一臂**——没有它们，删掉任一支或让 `len` 比较恒真
//!   都有漏判而套件其余全绿的路径。
//!
//! # 读法申报（收件人：复审者）
//!
//! - **判定的量**：§9.4 把 MATERIAL 的判定量写作「权限、成本、架构」。权限与成本在
//!   `PlanFootprint` 的 `authority` / `estimated_cost` 两个字段上，**比较由调用方给的
//!   [`MaterialThresholds`] 两个判据承载**（「显著扩大」的阈值规范未给——§16 第 15 条；
//!   本层不发明数值、也不发明比较）；架构在计划本身上，本层取「**计划内容（§228 的
//!   五个列表：objectives / milestones / assumptions / risks / acceptance_conditions）
//!   有任一变化**」为它的判定（不发明「哪一字段算重大」的分层——`PlanFootprint` 的
//!   另两个字段 `privacy_class` / `external_effects` 不参与，它们的名字对应 §225 的
//!   另两条禁令，那是 Task 16 的 Constraint Validator 的判定面）。
//! - **MINOR 五个例的构造**：换模型、重试、并行调整、工具替换、内部步骤重排都是
//!   **执行 HOW** 的变更（§225 的 MAY 清单与这五例重叠——「换模型 / 换工具 /
//!   改变并行度 / 改变内部执行顺序」），在 `classify` 的输入面（计划与它声明的影响面）
//!   上**不可见**——故这五行的构造是两 Plan 与两 footprint **逐字段相等**
//!   （行与行之间以不同的计划内容值区分）。这是本用例集的读法，不是 §230 的原文。
//! - **MATERIAL 五行的构造各只变一个维度**（判因唯一）：objectives 变（重大架构变化）、
//!   `acceptance_conditions` 变（Requirement 改变——Plan 面上与「要求」最近的字段；
//!   Contract 不在 `classify` 的输入面）、authority 变（权限扩大）、`estimated_cost` 变
//!   （成本显著扩大）、milestones 变（Milestone 目标变化）。
//! - **阈值判据的替身**：本文件的调用方判据取「值不同即视为扩大」（见 `differs`）——
//!   判据的语义由调用方定，用例只是把「扩大」构造成可观察（不是本层的判据）。
//!
//! # 夹具的来历
//!
//! 本文件**不碰库**（`classify` / `apply_plan_change` 都是纯函数：不收 `&Tx`、
//! 不读写任何表）——故没有 `new_db` 一类夹具。`plan_tagged` 的五个列表各取一项、
//! 且带 `tag` 记号：每行数据可区分，同时让「五个列表都参与比较」这件事可被观察
//! （MATERIAL 五行各自改动落在不同字段上）。
//!
//! # 「Apply」一侧的界限（照实写）
//!
//! `apply_plan_change` 是**值层面**的函数（同 `plan.rs` 的 `revise`）：本 task 的
//! `plan` 表没有行级读写，落库版的变更应用随行级 I/O 落地。它**不处置旧版本**
//! （§230 只说 MATERIAL MUST 重触发审查门；「新版本产生时旧版本进 `SUPERSEDED`」
//! 是 §9.2 给「修订」的规则，落在 `revise`）——本层不把两件事混成一件。

use std::cell::Cell;

use continuum_canonical::Opaque;
use continuum_semantics::{
    IntentId, MaterialThresholds, Plan, PlanChange, PlanChangeClass, PlanFootprint, PlanId,
    PlanType, PlanVersion, ReviewState, apply_plan_change, classify,
};

/// 一条基准 Plan：五个列表各取一项、带 `tag` 记号；`version = 1.3`、
/// `review_state = AiReviewed`（**非 DRAFT**——好让「保持 vs 重置」在用例里有区分度）。
fn plan_tagged(tag: &str) -> Plan {
    Plan {
        id: PlanId::from_str(&format!("p-{tag}")),
        version: PlanVersion { major: 1, minor: 3 },
        intent_id: IntentId::from_str("i-1"),
        r#type: PlanType::Execution,
        objectives: vec![Opaque::from_str(&format!("obj-{tag}"))],
        milestones: vec![Opaque::from_str(&format!("ms-{tag}"))],
        assumptions: vec![Opaque::from_str(&format!("asm-{tag}"))],
        risks: vec![Opaque::from_str(&format!("risk-{tag}"))],
        acceptance_conditions: vec![Opaque::from_str(&format!("ac-{tag}"))],
        review_state: ReviewState::AiReviewed,
    }
}

/// 一份基准 `PlanFootprint`：`privacy_class` / `external_effects` 取固定值
/// （本 task 的判定不读它们），authority / cost 由调用点给。
fn footprint(authority: &str, cost: &str) -> PlanFootprint {
    PlanFootprint {
        authority: Opaque::from_str(authority),
        privacy_class: Opaque::from_str("privacy-class-1"),
        external_effects: vec![Opaque::from_str("effect-1")],
        estimated_cost: Opaque::from_str(cost),
    }
}

/// 阈值判据的替身：**值不同即视为扩大**。判据的语义由调用方定——本层的用例
/// 只是把「扩大」构造成可观察（不是本层的判据，见文件头的读法申报）。
fn differs(before: &Opaque, after: &Opaque) -> bool {
    before.as_str() != after.as_str()
}

/// 用例用的 `MaterialThresholds`：两个判据都取 [`differs`]。
fn thresholds() -> MaterialThresholds<'static> {
    MaterialThresholds {
        authority_expanded: &differs,
        cost_expanded: &differs,
    }
}

/// `PlanChangeClass` 两枚的判别式：**穷尽 `match`、不带 `..`**——加第三枚即编译不过。
fn class_name(class: PlanChangeClass) -> &'static str {
    match class {
        PlanChangeClass::Minor => "minor",
        PlanChangeClass::Material => "material",
    }
}

/// 两份 `Opaque` 列表是否逐项相等（长度 ＋ 逐位置 `as_str`）。
fn same_list(a: &[Opaque], b: &[Opaque]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.as_str() == y.as_str())
}

/// 两个 class 各一条照片：`MINOR`（两 Plan 与两 footprint 全同）与 `MATERIAL`
/// （计划内容变化）。**判别式读出经穷尽 `match`**（见 `class_name`）。
#[test]
fn both_plan_change_classes_have_a_photo() {
    let plan = plan_tagged("photo-minor");
    let fp = footprint("authority-1", "cost-1");
    let minor = classify(&plan, &plan, &fp, &fp, &thresholds());
    assert_eq!(class_name(minor), "minor", "零差异的变更应判 MINOR");

    let before = plan_tagged("photo-material");
    let mut after = before.clone();
    after.milestones = vec![Opaque::from_str("ms-photo-material-2")];
    let material = classify(&before, &after, &fp, &fp, &thresholds());
    assert_eq!(class_name(material), "material", "计划内容变化的变更应判 MATERIAL");
}

/// §230 的**十个例逐项一条**（五个 MINOR ＋ 五个 MATERIAL，不抽代表）。
///
/// 红条件：把 MATERIAL 的某一例判成 MINOR（**放宽档**——那一行红）。
/// 表驱动的限度与两条直接调用见文件头。
#[test]
fn each_of_the_ten_examples_of_230_classifies_as_the_spec_says() {
    let thresholds = thresholds();
    let mut rows: Vec<(&str, Plan, Plan, PlanFootprint, PlanFootprint, PlanChangeClass)> =
        Vec::new();

    // ---- MINOR 五个例：执行 HOW 的变更——两 Plan 与两 footprint 逐字段相等 ----
    let minor_examples = [
        ("模型替换", "swap"),
        ("重试", "retry"),
        ("并行调整", "parallel"),
        ("工具替换", "tool"),
        ("内部步骤重排", "reorder"),
    ];
    for (name, tag) in minor_examples {
        let plan = plan_tagged(tag);
        let fp = footprint("authority-1", "cost-1");
        rows.push((name, plan.clone(), plan, fp.clone(), fp, PlanChangeClass::Minor));
    }

    // ---- MATERIAL 五个例：每个只变一个维度（判因唯一） ----

    // 重大架构变化：objectives 变化（计划内容之一）。
    let before = plan_tagged("arch");
    let mut after = before.clone();
    after.objectives = vec![Opaque::from_str("obj-arch-2")];
    rows.push((
        "重大架构变化",
        before,
        after,
        footprint("authority-1", "cost-1"),
        footprint("authority-1", "cost-1"),
        PlanChangeClass::Material,
    ));

    // Requirement 改变：acceptance_conditions 变化（Plan 面上与「要求」最近的字段；
    // Contract 不在 classify 的输入面——见文件头的读法申报）。
    let before = plan_tagged("req");
    let mut after = before.clone();
    after.acceptance_conditions = vec![Opaque::from_str("ac-req-2")];
    rows.push((
        "Requirement 改变",
        before,
        after,
        footprint("authority-1", "cost-1"),
        footprint("authority-1", "cost-1"),
        PlanChangeClass::Material,
    ));

    // Milestone 目标变化：milestones 变化。
    let before = plan_tagged("ms");
    let mut after = before.clone();
    after.milestones = vec![Opaque::from_str("ms-ms-2")];
    rows.push((
        "Milestone 目标变化",
        before,
        after,
        footprint("authority-1", "cost-1"),
        footprint("authority-1", "cost-1"),
        PlanChangeClass::Material,
    ));

    // 权限扩大：authority 变化（两 Plan 与 footprint 的其余字段不变）。
    let plan = plan_tagged("auth");
    rows.push((
        "权限扩大",
        plan.clone(),
        plan,
        footprint("authority-1", "cost-1"),
        footprint("authority-2", "cost-1"),
        PlanChangeClass::Material,
    ));

    // 成本显著扩大：estimated_cost 变化。
    let plan = plan_tagged("cost");
    rows.push((
        "成本显著扩大",
        plan.clone(),
        plan,
        footprint("authority-1", "cost-1"),
        footprint("authority-1", "cost-2"),
        PlanChangeClass::Material,
    ));

    assert_eq!(
        rows.len(),
        10,
        "§230 的例是十个（五个 MINOR ＋ 五个 MATERIAL）——两个五见文件头的「数的来历」"
    );
    for (name, before, after, before_fp, after_fp, expected) in rows {
        let got = classify(&before, &after, &before_fp, &after_fp, &thresholds);
        assert_eq!(
            got, expected,
            "§230 的例「{name}」：期望 {expected:?}，实得 {got:?}"
        );
    }
}

/// **直接调用**（不经表）：MINOR **首例**「模型替换」——两 Plan 与两 footprint
/// 逐字段相等（执行 HOW 的变化在 `classify` 的输入面上不可见）。
///
/// 这条用例与 `a_major_architecture_change_is_material` 一起，是表驱动用例的对照
/// （「表驱动用例也可能覆盖错路径」——见文件头）；两个 class 各自的**第一条**例
/// 也在实现注释（`src/plan_change.rs`）里点名。
#[test]
fn a_model_swap_is_a_minor_change() {
    let plan = plan_tagged("direct-swap");
    let fp = footprint("authority-1", "cost-1");
    let got = classify(&plan, &plan, &fp, &fp, &thresholds());
    assert_eq!(
        got,
        PlanChangeClass::Minor,
        "模型替换（MINOR 首例）：应判 minor，实得 {got:?}"
    );
}

/// **直接调用**（不经表）：MATERIAL **首例**「重大架构变化」——计划内容变化。
#[test]
fn a_major_architecture_change_is_material() {
    let before = plan_tagged("direct-arch");
    let mut after = before.clone();
    after.objectives = vec![Opaque::from_str("obj-direct-arch-2")];
    let fp = footprint("authority-1", "cost-1");
    let got = classify(&before, &after, &fp, &fp, &thresholds());
    assert_eq!(
        got,
        PlanChangeClass::Material,
        "重大架构变化（MATERIAL 首例）：应判 material，实得 {got:?}"
    );
}

/// **直接调用**（不经表）：`risks` **单独**变化（其余不动）→ MATERIAL。
///
/// 钉 `plan_content_changed` 的 **risks 那一臂**（五列表析取中的一支）：十行表只用
/// objectives / acceptance_conditions / milestones 三个列表构造，删掉 risks 那一支
/// 会漏判「风险单独变化」而套件其余全绿（fix 轮 1 的评审 Important）。
#[test]
fn a_risks_only_change_is_material() {
    let before = plan_tagged("direct-risks");
    let mut after = before.clone();
    after.risks = vec![Opaque::from_str("risk-direct-risks-2")];
    let fp = footprint("authority-1", "cost-1");
    let got = classify(&before, &after, &fp, &fp, &thresholds());
    assert_eq!(
        got,
        PlanChangeClass::Material,
        "risks 单独变化：应判 material，实得 {got:?}"
    );
}

/// **直接调用**（不经表）：`assumptions` **单独**变化 → MATERIAL。
///
/// 钉 **assumptions 那一臂**（照片理由同 [`a_risks_only_change_is_material`]：
/// 十行表没有 assumptions 的构造）。
#[test]
fn an_assumptions_only_change_is_material() {
    let before = plan_tagged("direct-assumptions");
    let mut after = before.clone();
    after.assumptions = vec![Opaque::from_str("asm-direct-assumptions-2")];
    let fp = footprint("authority-1", "cost-1");
    let got = classify(&before, &after, &fp, &fp, &thresholds());
    assert_eq!(
        got,
        PlanChangeClass::Material,
        "assumptions 单独变化：应判 material，实得 {got:?}"
    );
}

/// **直接调用**（不经表）：某列表**长度变化**（追加一项、前项不变）→ MATERIAL。
///
/// 钉 `same_list` 的 **`len` 比较那一臂**：没有这条照片，`a.len() == b.len()` 恒真时
/// `zip` 只比到短侧——「追加一项」会被漏判 MINOR 而套件其余全绿（fix 轮 1 的
/// 评审 Important 点名了这一格）。
#[test]
fn a_list_length_change_is_material() {
    let before = plan_tagged("direct-append");
    let mut after = before.clone();
    after.objectives.push(Opaque::from_str("obj-direct-append-2"));
    let fp = footprint("authority-1", "cost-1");
    let got = classify(&before, &after, &fp, &fp, &thresholds());
    assert_eq!(
        got,
        PlanChangeClass::Material,
        "列表追加一项（长度变化）：应判 material，实得 {got:?}"
    );
}

/// §230 的 MUST：MATERIAL 重触发审查门。**两侧都钉**——MATERIAL 产出新版本
/// （major +1、`review_state == DRAFT`）；MINOR 不重触发（minor +1、审查态保持）。
///
/// 输入取 `review_state = AiReviewed`（非 DRAFT）：这样「保持 vs 重置」有区分度。
/// 红条件：MATERIAL 不重置审查门（**移除档**）；或 MINOR 也重置（反面那半边红）。
#[test]
fn a_material_change_re_triggers_the_review_gate() {
    let plan = plan_tagged("gate"); // version {1, 3}、review_state = AiReviewed

    // MATERIAL：重触发审查门——新版本从 DRAFT 起（§338 的阶梯重走）。
    let material = apply_plan_change(&plan, PlanChangeClass::Material);
    assert_eq!(
        material.version,
        PlanVersion { major: 2, minor: 0 },
        "MATERIAL 应 major +1（minor 归零），实得 {:?}",
        material.version
    );
    assert_eq!(
        material.review_state,
        ReviewState::Draft,
        "MATERIAL 应重触发审查门（新版本为 DRAFT），实得 {:?}",
        material.review_state
    );

    // MINOR：不重触发——版本推进但审查态保持。
    let minor = apply_plan_change(&plan, PlanChangeClass::Minor);
    assert_eq!(
        minor.version,
        PlanVersion { major: 1, minor: 4 },
        "MINOR 应 minor +1（major 不动），实得 {:?}",
        minor.version
    );
    assert_eq!(
        minor.review_state,
        ReviewState::AiReviewed,
        "MINOR 不重触发（审查态保持），实得 {:?}",
        minor.review_state
    );

    // 两者都是**同一计划的下一版**：id 不变、内容（五个列表）逐项原样。
    for (what, next) in [("MATERIAL", &material), ("MINOR", &minor)] {
        assert_eq!(next.id.as_str(), plan.id.as_str(), "{what} 新版本应保持同一个 id");
        assert!(
            same_list(&next.objectives, &plan.objectives)
                && same_list(&next.milestones, &plan.milestones)
                && same_list(&next.assumptions, &plan.assumptions)
                && same_list(&next.risks, &plan.risks)
                && same_list(&next.acceptance_conditions, &plan.acceptance_conditions),
            "{what} 新版本的内容应逐项原样（本函数只推版本与审查态）"
        );
    }

    // 原计划未被改动（纯函数）。
    assert_eq!(plan.version, PlanVersion { major: 1, minor: 3 });
    assert_eq!(plan.review_state, ReviewState::AiReviewed);
}

/// §230 未给 `affected_scope` 的取值域 → 不透明值（§16 第 15 条）：**表外取值**
/// 逐字节往返；字段清单逐项（穷尽解构、不带 `..`）。
///
/// **本层不发明取值域**——一个明显不在任何词表上的串原样进出。
#[test]
fn affected_scope_is_opaque() {
    let raw = "范围：步骤 2–4（表外取值，本层不解释）";
    let change = PlanChange {
        class: PlanChangeClass::Material,
        affected_scope: Opaque::from_str(raw),
    };
    assert_eq!(
        change.affected_scope.as_str(),
        raw,
        "affected_scope 应按原样逐字节读回（不改写、不裁剪）"
    );

    let PlanChange { class, affected_scope } = change;
    assert_eq!(class_name(class), "material");
    assert_eq!(affected_scope.as_str(), raw);
}

/// **输入面签名**：`classify` 收两份 `Plan` ＋ 两份 `PlanFootprint` ＋ 一个显式的
/// 阈值入参（`&MaterialThresholds`——R-B 的第五参）。
///
/// 依据：`权限扩大` 与 `成本显著扩大` 需要计划侧声明的影响面（§11.2 的
/// `PlanFootprint`）；而「显著扩大」的**阈值规范未给**（§16 第 15 条）——本层
/// **不发明阈值**，比较由调用方的判据承载（`MaterialThresholds` 的两个引用）。
///
/// 红条件：去掉阈值入参即**编译不过**（`fn` 指针标注钉死五参）；阈值判据被忽略
/// （恒真判据不生效）即红。
#[test]
fn the_classification_reads_both_plans_and_both_footprints() {
    // 签名层面：`fn` 指针标注钉死五参与返回类型（短签名或换形状即编译不过）。
    let f: fn(&Plan, &Plan, &PlanFootprint, &PlanFootprint, &MaterialThresholds<'_>) -> PlanChangeClass =
        classify;
    let _ = f;

    // 两个阈值判据**真的被调用**（本层不自己比较——比较是调用方的）。
    let authority_called = Cell::new(false);
    let cost_called = Cell::new(false);
    let counting = MaterialThresholds {
        authority_expanded: &|before: &Opaque, after: &Opaque| {
            authority_called.set(true);
            before.as_str() != after.as_str()
        },
        cost_expanded: &|before: &Opaque, after: &Opaque| {
            cost_called.set(true);
            before.as_str() != after.as_str()
        },
    };
    let plan = plan_tagged("reads");
    let fp = footprint("authority-1", "cost-1");
    let got = classify(&plan, &plan, &fp, &fp, &counting);
    assert_eq!(got, PlanChangeClass::Minor);
    assert!(authority_called.get(), "authority 判据应被调用（两 Plan 与两 footprint 都进入判定）");
    assert!(cost_called.get(), "cost 判据应被调用（两 Plan 与两 footprint 都进入判定）");

    // 恒真判据：即便两 Plan 与其余都相同，`authority_expanded == true` 也判 MATERIAL
    // （若实现忽略 footprint，这里会是 Minor——红）。
    let always = MaterialThresholds {
        authority_expanded: &|_: &Opaque, _: &Opaque| true,
        cost_expanded: &|_: &Opaque, _: &Opaque| false,
    };
    assert_eq!(
        classify(&plan, &plan, &fp, &fp, &always),
        PlanChangeClass::Material,
        "authority 判据判真时应判 MATERIAL"
    );

    let always = MaterialThresholds {
        authority_expanded: &|_: &Opaque, _: &Opaque| false,
        cost_expanded: &|_: &Opaque, _: &Opaque| true,
    };
    assert_eq!(
        classify(&plan, &plan, &fp, &fp, &always),
        PlanChangeClass::Material,
        "cost 判据判真时应判 MATERIAL"
    );
}
