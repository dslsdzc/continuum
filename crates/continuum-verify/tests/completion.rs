//! Completion Predicate 的十项合取判定（设计 §8.1–§8.4；计划 Task 7 的 Step 1）。
//!
//! # 一处对任务材料的最小修复（申报；汇报见 `task-7-report.md`）
//!
//! 设计 §8.3 末的第十一项——`Intent.completion_predicate` 的声明值，「**不是合取项**但同等效力」——
//! 在 §8.2 的类型 `Undetermined { undecidable: Vec<Conjunct> }` 里**无处安放**（元素类型是十臂的
//! [`Conjunct`]）；而 §8.4 与 §12.1 (r) 都要它「**追加一项**不可判」、且 (r) 的红条件要求
//! 「有内容 vs 无内容」在返回值上**可观察**。「只可能动列表或变体」的穷举验证见
//! `.tmp/task7-route-decision.md`。本实现把 `undecidable` 的元素类型取为 [`UndecidableItem`]
//! （两臂：合取项、第十一项），使「追加一项」在类型与返回值上都字面成立。合取项断言经
//! [`item`] 只多一层构造，**语义不变**。
//!
//! 本文件里凡提到「编译失败」的地方都照纪律 1(c) 记账：**编译失败不是「变红」**，
//! 不假称它们会跑红。

use continuum_verify::{
    CompletionInput, CompletionVerdict, Conjunct, CoverageVerdict, IncompleteReason,
    IndependenceVerdict, RequirementCoverage, RequirementId, UndecidableItem, VerdictAggregate,
    judge_completion,
};
use serde_json::{Value, json};

// ---------- 夹具 ----------

/// 一条 `Incomplete { NoValidEvidence }` 的 coverage（第 2 项的输入；判定不读它的内容）。
fn incomplete_coverage() -> Vec<RequirementCoverage> {
    vec![RequirementCoverage {
        requirement: RequirementId::from_str("r-1"),
        verdict: CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence,
        },
    }]
}

/// 「其余各项全为『真』」的输入：`artifact_exists = true`、coverage 一条 `Incomplete` 的 REQUIRED、
/// 六项 `Option` 全 `Some(true)`、聚合 `Passed`、独立性 `Independent`、
/// `declared_completion_predicate = None`。各用例只改自己那一个字段。
fn all_true_input() -> CompletionInput {
    CompletionInput {
        artifact_exists: true,
        coverage: incomplete_coverage(),
        contract_satisfied: Some(true),
        mandatory_effects_completed: Some(true),
        verifier_verdicts: VerdictAggregate::Passed,
        independence: IndependenceVerdict::Independent,
        mandatory_tests_pass: Some(true),
        applicable_properties_hold: Some(true),
        applicable_fuzz_budget_complete: Some(true),
        applicable_mutation_quality_sufficient: Some(true),
        declared_completion_predicate: None,
    }
}

/// 判定并将 `Err` 排除：`judge_completion` 今天没有失败路径（签名照设计 §8.4 照录）。
fn output(input: &CompletionInput) -> CompletionVerdict {
    judge_completion(input).expect("judge_completion 今天没有失败路径（签名照设计 §8.4 照录）")
}

/// 期望的不可判列表：一枚合取项。
fn item(conjunct: Conjunct) -> UndecidableItem {
    UndecidableItem::Conjunct(conjunct)
}

/// 期望的 `Blocked`。
fn blocked(failing: Vec<Conjunct>) -> CompletionVerdict {
    CompletionVerdict::Blocked { failing }
}

/// 期望的 `Undetermined`。
fn undetermined(undecidable: Vec<UndecidableItem>) -> CompletionVerdict {
    CompletionVerdict::Undetermined { undecidable }
}

/// 取出 `Blocked.failing`（其余形态即测试失败）。
fn failing_of(input: &CompletionInput) -> Vec<Conjunct> {
    match output(input) {
        CompletionVerdict::Blocked { failing } => failing,
        other => panic!("应为 Blocked，得到 {other:?}"),
    }
}

// ---------- 用例 ----------

/// §8.1 的并集：`Conjunct::ALL` 恰好十臂，**逐臂点名**；前五项出 §266 的第 1–5 项、
/// 后五项出 §195。
///
/// **两条去重**（设计 §8.1）：
/// - `AllRequiredRequirementsVerified` ≈ §195 的 `requirement_coverage_sufficient`——
///   合并**只在方向上安全**：合并后的臂恒 `Undetermined`（§3.4 第 3 条），故即便两者本非同义
///   也不会放行任何一项；这是 fail-closed 的方向，**不是同义性论证**；
/// - `ContractSatisfied` ≈ §195 的 `task_contract_satisfied`——判据同一：§225 的
///   Constraint Validator 的结论。
///
/// 红条件：删掉 `Conjunct::ALL` 里的一臂（**移除档**）⇒ 数目断言（锚点只取 `ALL`，
/// 纪律 1(a)）与逐臂断言都红。
#[test]
fn the_ten_conjuncts_come_from_the_union_of_266_and_195() {
    assert_eq!(
        Conjunct::ALL.len(),
        10,
        "§266 与 §195 的并集（含两条去重）恰十项"
    );
    assert_eq!(
        Conjunct::ALL,
        [
            Conjunct::ArtifactExists,
            Conjunct::AllRequiredRequirementsVerified,
            Conjunct::ContractSatisfied,
            Conjunct::MandatoryEffectsCompleted,
            Conjunct::NoBlockingVerificationFailure,
            Conjunct::MandatoryTestsPass,
            Conjunct::ApplicablePropertiesHold,
            Conjunct::ApplicableFuzzBudgetComplete,
            Conjunct::ApplicableMutationQualitySufficient,
            Conjunct::IndependentVerificationPass,
        ],
        "逐臂点名：前五项 §266 第 1–5 项、后五项 §195"
    );
}

/// (q) 六项 `Option<bool>` 的**两侧**：`None` ⇒ 该项列入 `Undetermined.undecidable`；
/// `Some(false)` ⇒ 该项列入 `Blocked.failing`。**逐项手写，不抽代表**（计划纪律第 2 条）。
///
/// 设计 §12.1 (q) 写的是「四项」，而 §8.4 的 `Option<bool>` 字段**实有六项**
/// （第 3、4、6、7、8、9 项各一）——本计划按 §8.4 的字段定义取**六项**
/// （正文优先于它的用例表）。
///
/// 两侧都钉：缺哪一侧都会让「没判」与「判为假」互写——而压缩的方向是把不放行说成失败，
/// 会让调用方去查一个不存在的失败。
///
/// 红条件：把 `Option<bool>` 换成 `bool`（`None` 与 `false` 合流，**取反档**）⇒
/// 十二条里的第一条红。
#[test]
fn a_none_option_is_undetermined_and_a_false_option_is_blocked() {
    // 第 3 项 contract_satisfied
    let mut input = all_true_input();
    input.contract_satisfied = None;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::ContractSatisfied),
        ]),
        "contract_satisfied = None ⇒ 列入 undecidable"
    );
    input.contract_satisfied = Some(false);
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::ContractSatisfied]),
        "contract_satisfied = Some(false) ⇒ 列入 failing"
    );

    // 第 4 项 mandatory_effects_completed
    let mut input = all_true_input();
    input.mandatory_effects_completed = None;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::MandatoryEffectsCompleted),
        ]),
        "mandatory_effects_completed = None ⇒ 列入 undecidable"
    );
    input.mandatory_effects_completed = Some(false);
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::MandatoryEffectsCompleted]),
        "mandatory_effects_completed = Some(false) ⇒ 列入 failing"
    );

    // 第 6 项 mandatory_tests_pass
    let mut input = all_true_input();
    input.mandatory_tests_pass = None;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::MandatoryTestsPass),
        ]),
        "mandatory_tests_pass = None ⇒ 列入 undecidable"
    );
    input.mandatory_tests_pass = Some(false);
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::MandatoryTestsPass]),
        "mandatory_tests_pass = Some(false) ⇒ 列入 failing"
    );

    // 第 7 项 applicable_properties_hold
    let mut input = all_true_input();
    input.applicable_properties_hold = None;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::ApplicablePropertiesHold),
        ]),
        "applicable_properties_hold = None ⇒ 列入 undecidable"
    );
    input.applicable_properties_hold = Some(false);
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::ApplicablePropertiesHold]),
        "applicable_properties_hold = Some(false) ⇒ 列入 failing"
    );

    // 第 8 项 applicable_fuzz_budget_complete
    let mut input = all_true_input();
    input.applicable_fuzz_budget_complete = None;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::ApplicableFuzzBudgetComplete),
        ]),
        "applicable_fuzz_budget_complete = None ⇒ 列入 undecidable"
    );
    input.applicable_fuzz_budget_complete = Some(false);
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::ApplicableFuzzBudgetComplete]),
        "applicable_fuzz_budget_complete = Some(false) ⇒ 列入 failing"
    );

    // 第 9 项 applicable_mutation_quality_sufficient
    let mut input = all_true_input();
    input.applicable_mutation_quality_sufficient = None;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::ApplicableMutationQualitySufficient),
        ]),
        "applicable_mutation_quality_sufficient = None ⇒ 列入 undecidable"
    );
    input.applicable_mutation_quality_sufficient = Some(false);
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::ApplicableMutationQualitySufficient]),
        "applicable_mutation_quality_sufficient = Some(false) ⇒ 列入 failing"
    );
}

/// (r) 有内容的 `declared_completion_predicate` ⇒ **追加一项不可判**；空/缺 ⇒ 不追加；
/// **两个不同的非空值给出同一种结果**（本块**不解析**它的内容——解析就是发明语法）。
///
/// **「有内容」的取一**（本 task 定）：无内容 ⟺ `None` 或 `Value::Null`；其余任何值
/// （含 `{}` / `[]` / `""`）都算有内容。理由：另一方向是把用户声明的追加谓词静默丢掉，
/// 触 §342 第 1 条的方向；代价：空容器被当成有内容。
///
/// 红条件：把「有内容」判成「无内容」（**放宽档**）⇒ 第一条红；把「一律追加」写死
/// （**取反档**）⇒ `None` 那一条红。
#[test]
fn a_declared_predicate_with_content_adds_one_undecidable_item() {
    // 有内容 ⇒ 多一项（恰含：第 2 项恒在 + 第十一项）
    let mut input = all_true_input();
    input.declared_completion_predicate = Some(json!({ "x": 1 }));
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            UndecidableItem::DeclaredCompletionPredicate,
        ]),
        "有内容 ⇒ 结果里多一项不可判"
    );

    // 两个不同的非空值给出同一种结果（不解析内容）
    let mut other = all_true_input();
    other.declared_completion_predicate = Some(json!({ "y": [1, 2, true] }));
    assert_eq!(
        output(&input),
        output(&other),
        "两个不同的非空值应给出同一种结果（本块不解析它的内容）"
    );

    // None 与 Value::Null ⇒ 不追加
    let none = all_true_input();
    assert_eq!(
        output(&none),
        undetermined(vec![item(Conjunct::AllRequiredRequirementsVerified)]),
        "None ⇒ 不追加"
    );
    let mut null = all_true_input();
    null.declared_completion_predicate = Some(Value::Null);
    assert_eq!(
        output(&null),
        output(&none),
        "Value::Null ⇒ 不追加（与 None 同结果）"
    );

    // 取一的边界：空容器也算有内容（本 task 的代价，逐值点名）
    for empty_container in [json!({}), json!([]), json!("")] {
        let mut input = all_true_input();
        input.declared_completion_predicate = Some(empty_container.clone());
        assert_eq!(
            output(&input),
            undetermined(vec![
                item(Conjunct::AllRequiredRequirementsVerified),
                UndecidableItem::DeclaredCompletionPredicate,
            ]),
            "取一：{empty_container} 也算有内容 ⇒ 追加"
        );
    }
}

/// **本设计最重要的一条**：第 2 项恒不可判——`coverage` 给的是一条 `Incomplete` 的 REQUIRED，
/// 其余各项都给「为真」的值 ⇒ 结果是 `Undetermined { undecidable }`，**且 `undecidable` 恰含
/// `AllRequiredRequirementsVerified`**（**不是 `Blocked`，也不是空表**）。
///
/// 红条件：把第 2 项改成读 `CoverageVerdict`（写成「有 `Incomplete` 即 `false`」⇒ `Blocked`；
/// 或写成「全 `NotEstablished` 即 `true`」⇒ 空表）（**取反档**）⇒ 红。
///
/// 这条是 §342 第 10 条与 §3.2「覆盖无正臂」的连接点，也是「完成不可表达」的运行期一侧。
#[test]
fn the_second_conjunct_is_always_undetermined() {
    let input = all_true_input();
    assert_eq!(
        output(&input),
        undetermined(vec![item(Conjunct::AllRequiredRequirementsVerified)]),
        "第 2 项恒不可判：恰含它、不是 Blocked、不是空表"
    );
}

/// **假优先于不可判**：`artifact_exists = false` ＋ 第 3、4、6–9 项全 `None` ⇒
/// `Blocked { failing: [ArtifactExists] }`（`failing` 恰是那一项）。
///
/// 红条件：把优先级反过来先取 `Undetermined`（**取反档**）⇒ 红。
#[test]
fn a_false_conjunct_blocks_even_when_others_are_undecided() {
    let mut input = all_true_input();
    input.artifact_exists = false;
    input.contract_satisfied = None;
    input.mandatory_effects_completed = None;
    input.mandatory_tests_pass = None;
    input.applicable_properties_hold = None;
    input.applicable_fuzz_budget_complete = None;
    input.applicable_mutation_quality_sufficient = None;
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::ArtifactExists]),
        "有假项 ⇒ Blocked，且 failing 恰是那一项（假优先于不可判）"
    );
}

/// (s) 第一子例：`independence = NotEstablished` ⇒ 第 10 项**列入 `undecidable`**，
/// **不是** `Blocked`。
///
/// 红条件：写成 `NotIndependent`，即「没判」写成「判为假」（**取反档**）⇒ 红。
/// 这正是 §8.4 警告过的「让调用方去查一个不存在的失败」。
///
/// **与 Task 6 的 `an_unknown_identity_is_not_established` 是同一处变异**（跨 task 的同一处判据）。
#[test]
fn independence_not_established_makes_the_tenth_item_undecidable() {
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotEstablished;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::IndependentVerificationPass),
        ]),
        "NotEstablished ⇒ 第 10 项列入 undecidable，不是 Blocked"
    );
}

/// (s) 第二子例（**另一侧**）：`NotIndependent` ⇒
/// `Blocked { failing: [IndependentVerificationPass] }`。
///
/// 红条件：与上一条同一次变异反向（**取反档**）⇒ 红。
#[test]
fn not_independent_blocks_the_tenth_item() {
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotIndependent;
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::IndependentVerificationPass]),
        "NotIndependent ⇒ 第 10 项为假，列入 failing"
    );
}

/// 第 10 项的正例：`Independent` ＋ `VerdictAggregate::Passed` ⇒ 第 10 项**既不在 `failing`
/// 也不在 `undecidable`**；`PassedWithUnknown` 同样为真。
///
/// 红条件：把它写成恒不可判（**收紧档**）⇒ 红（`undecidable` 里多一项）。
#[test]
fn an_independent_pass_makes_the_tenth_item_true() {
    // Independent + Passed（all_true 的默认值）
    let input = all_true_input();
    assert_eq!(
        output(&input),
        undetermined(vec![item(Conjunct::AllRequiredRequirementsVerified)]),
        "Independent + Passed ⇒ 第 10 项为真（既不在 failing 也不在 undecidable）"
    );

    // Independent + PassedWithUnknown 同样为真
    let mut input = all_true_input();
    input.verifier_verdicts = VerdictAggregate::PassedWithUnknown;
    assert_eq!(
        output(&input),
        undetermined(vec![item(Conjunct::AllRequiredRequirementsVerified)]),
        "Independent + PassedWithUnknown ⇒ 第 10 项为真"
    );
}

/// 第 10 项判据表的**第四格，三臂逐条**：`independence = Independent` 而聚合取
/// `Failed`、`AdjudicationRequired`、`Unknown` 各一条 ⇒ 第 10 项**为假**（列入 `failing`）。
///
/// **判据表**（§8.3 第 10 项只给了前三行，第四行由本计划补；本表写进实现的文档注释）：
/// `NotIndependent` ⇒ 假；`NotEstablished` ⇒ 不可判；
/// `Independent` ∧ 聚合 ∈ {`Passed`, `PassedWithUnknown`} ⇒ 真；
/// `Independent` 而聚合非通过 ⇒ 假。**本条只拍第四格**——前三格的照片是 (s) 的第一／二子例
/// 与 `an_independent_pass_makes_the_tenth_item_true`。
///
/// 红条件：把「`Independent` 而聚合非通过」写成不可判（**收紧档**）⇒ 三条全红。
#[test]
fn a_non_passing_aggregate_makes_the_tenth_item_false() {
    // Failed：第 5 项（阻断失败）与第 10 项同时为假
    let mut input = all_true_input();
    input.verifier_verdicts = VerdictAggregate::Failed;
    let failing = failing_of(&input);
    assert_eq!(failing.len(), 2, "Failed ⇒ 第 5、10 项都为假");
    assert!(
        failing.contains(&Conjunct::NoBlockingVerificationFailure),
        "Failed ⇒ 第 5 项也命中"
    );
    assert!(
        failing.contains(&Conjunct::IndependentVerificationPass),
        "Independent + Failed ⇒ 第 10 项为假"
    );

    // AdjudicationRequired：同上两项都为假
    let mut input = all_true_input();
    input.verifier_verdicts = VerdictAggregate::AdjudicationRequired;
    let failing = failing_of(&input);
    assert_eq!(failing.len(), 2, "AdjudicationRequired ⇒ 第 5、10 项都为假");
    assert!(
        failing.contains(&Conjunct::NoBlockingVerificationFailure),
        "AdjudicationRequired ⇒ 第 5 项也命中"
    );
    assert!(
        failing.contains(&Conjunct::IndependentVerificationPass),
        "Independent + AdjudicationRequired ⇒ 第 10 项为假"
    );

    // Unknown：第 5 项为真（不阻断），第 10 项为假
    let mut input = all_true_input();
    input.verifier_verdicts = VerdictAggregate::Unknown;
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::IndependentVerificationPass]),
        "Independent + Unknown ⇒ 第 10 项为假（第 5 项对 Unknown 为真）"
    );
}

/// 第 5 项随聚合的五臂逐条——`Failed` ⇒ 为假；`AdjudicationRequired` ⇒ 为假；
/// `Passed` / `PassedWithUnknown` / `Unknown` ⇒ 为真（判据是「不出现 `Failed`，
/// 且无 `AdjudicationRequired`」）。
///
/// **逐臂手写，不抽代表**；`independence` 取 `NotEstablished` 以隔离第 5 项
/// （第 10 项落 `undecidable`，不干扰 `failing` 的断言）。
///
/// 红条件：把 `Unknown` 也算作阻断（**收紧档**）⇒ 该臂红。
#[test]
fn the_fifth_conjunct_follows_the_aggregate_arm_by_arm() {
    // Failed ⇒ 第 5 项为假
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotEstablished;
    input.verifier_verdicts = VerdictAggregate::Failed;
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::NoBlockingVerificationFailure]),
        "Failed ⇒ 第 5 项为假"
    );

    // AdjudicationRequired ⇒ 第 5 项为假
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotEstablished;
    input.verifier_verdicts = VerdictAggregate::AdjudicationRequired;
    assert_eq!(
        output(&input),
        blocked(vec![Conjunct::NoBlockingVerificationFailure]),
        "AdjudicationRequired ⇒ 第 5 项为假"
    );

    // Passed ⇒ 第 5 项为真
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotEstablished;
    input.verifier_verdicts = VerdictAggregate::Passed;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::IndependentVerificationPass),
        ]),
        "Passed ⇒ 第 5 项为真（failing 为空，第 10 项改落 undecidable）"
    );

    // PassedWithUnknown ⇒ 第 5 项为真
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotEstablished;
    input.verifier_verdicts = VerdictAggregate::PassedWithUnknown;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::IndependentVerificationPass),
        ]),
        "PassedWithUnknown ⇒ 第 5 项为真"
    );

    // Unknown ⇒ 第 5 项为真（不阻断）
    let mut input = all_true_input();
    input.independence = IndependenceVerdict::NotEstablished;
    input.verifier_verdicts = VerdictAggregate::Unknown;
    assert_eq!(
        output(&input),
        undetermined(vec![
            item(Conjunct::AllRequiredRequirementsVerified),
            item(Conjunct::IndependentVerificationPass),
        ]),
        "Unknown ⇒ 第 5 项为真（判据是「不出现 Failed，且无 AdjudicationRequired」）"
    );
}

/// 两枚枚举的臂数与逐臂点名：`CompletionVerdict`（2 臂，§8.2）与
/// `UndecidableItem`（2 臂——本 task 对任务材料的最小修复，见文件头）。
///
/// **没有「完成」这一臂**：`AllRequiredRequirementsVerified` 的输入是 `CoverageVerdict`，
/// 而那个枚举没有正臂，故合取**不可能为真**——**这是构造性论证，不是实测**；
/// 它的实测补件是 Task 10 的 `CompletionVerdict::Complete` 编译失败样例。
///
/// 红条件：删掉 `CompletionVerdict::ALL`（或 `UndecidableItem::ALL`）里的一臂
/// （**移除档**）⇒ 数目断言红。
#[test]
fn the_two_verdict_arms_are_named() {
    assert_eq!(CompletionVerdict::ALL.len(), 2, "§8.2 恰两臂");
    assert_eq!(
        CompletionVerdict::ALL,
        [
            CompletionVerdict::Blocked { failing: vec![] },
            CompletionVerdict::Undetermined {
                undecidable: vec![]
            },
        ],
        "逐臂点名：Blocked / Undetermined；没有「完成」这一臂"
    );

    assert_eq!(UndecidableItem::ALL.len(), 2, "不可判项恰两臂");
    assert_eq!(
        UndecidableItem::ALL,
        [
            UndecidableItem::Conjunct(Conjunct::AllRequiredRequirementsVerified),
            UndecidableItem::DeclaredCompletionPredicate,
        ],
        "逐臂点名：合取项 / 第十一项（不是合取项但同等效力）"
    );
}
