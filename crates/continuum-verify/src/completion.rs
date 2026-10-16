//! §8 的 Completion Predicate 判定（设计 §8.2–§8.4）。

use crate::conjunct::{Conjunct, UndecidableItem};
use crate::coverage::RequirementCoverage;
use crate::error::VerifyError;
use crate::selection::IndependenceVerdict;
use crate::verdict::VerdictAggregate;

/// **没有「完成」这一臂**（§8.2）。
///
/// 「完成」为什么不是第三臂：`AllRequiredRequirementsVerified` 的输入是
/// [`CoverageVerdict`](crate::coverage::CoverageVerdict)，而那个枚举**没有正臂**（§6.2），
/// 故合取**不可能为真**。**这是构造性论证，不是实测**：断的是「本类型里不存在一个可达的
/// 『完成』值」，判据是 §6.2 的类型定义 + 本函数的合取结构。**它的实测补件**是 Task 10 的
/// `CompletionVerdict::Complete` 编译失败样例（§12.3 第 3 条）。
///
/// **两条臂与 Intent 状态的对接不在本块**（切分 §四第 4 条）：本块给出的两个返回值都是
/// 「不得完成」，如何路由（`Blocked` → 节点 `FAILED` 或重试；`Undetermined` → `Decision`）
/// 是执行器与 P4 的 Decision 面的事。
///
/// **`Undetermined.undecidable` 的元素类型是 [`UndecidableItem`]**（设计 §8.2 写作
/// `Vec<Conjunct>`；对任务材料的最小修复见该枚举的文档）。
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionVerdict {
    /// 至少一项为假。列出为假的项。
    Blocked { failing: Vec<Conjunct> },
    /// 无一项为假，但至少一项**无判据或判不出**。列出这些项。
    Undetermined { undecidable: Vec<UndecidableItem> },
}

impl CompletionVerdict {
    /// §8.2 的两臂，逐臂照录。**本常量带数目字面量**，与 [`Conjunct::ALL`] 同形。
    ///
    /// 载荷取空清单（样例），不承诺代表性；数目与逐臂由 `tests/completion.rs` 的
    /// `the_two_verdict_arms_are_named` 把关。
    pub const ALL: [CompletionVerdict; 2] = [
        CompletionVerdict::Blocked {
            failing: Vec::new(),
        },
        CompletionVerdict::Undetermined {
            undecidable: Vec::new(),
        },
    ];
}

/// §8.4 的输入面。凡本块不判的，一律**以值传入**（P4 的约束结论 / P2 的效应终态 /
/// 装配方从制品与预算面读出的四项）。
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionInput {
    /// 第 1 项：由受验图与制品表算出（§8.3 第 1 项的判据：受验图的终端节点**各自**至少
    /// 有一个产出制品）。
    pub artifact_exists: bool,
    /// 第 2 项：§6 的逐项覆盖判定，数目与 REQUIRED 项相同。
    pub coverage: Vec<RequirementCoverage>,
    /// 第 3、4 项：**以值传入**（P4 的约束结论 / P2 的效应终态）。`None` = 未给出 ⇒ 不可判。
    pub contract_satisfied: Option<bool>,
    pub mandatory_effects_completed: Option<bool>,
    /// 第 5、10 项：§7 的判定产出。
    /// `independence == NotEstablished` ⇒ 第 10 项**不可判**（列入 `undecidable`），
    /// **不是为假**——与下面四项 `Option` 的 `None` 同一条纪律（见 §8.4 末「`Option<bool>`
    /// 而不是 `bool`」）。
    pub verifier_verdicts: VerdictAggregate,
    pub independence: IndependenceVerdict,
    /// 第 6–9 项：以值传入（装配方从制品与预算面读出）。`None` = 未给出 ⇒ 不可判。
    pub mandatory_tests_pass: Option<bool>,
    pub applicable_properties_hold: Option<bool>,
    pub applicable_fuzz_budget_complete: Option<bool>,
    pub applicable_mutation_quality_sufficient: Option<bool>,
    /// §222 的 `Intent.completion_predicate`——**用户声明的追加谓词**。
    /// **本块只判「它有没有内容」**：有内容 ⇒ 追加一项不可判（fail-closed，§342 第 1 条）；
    /// 空/缺 ⇒ 不追加。**不解析它**——解析就是发明语法。
    ///
    /// **「有内容」的取一（本 task 定）**：无内容 ⟺ `None` 或 `Value::Null`；其余任何值
    /// （含 `{}` / `[]` / `""`）都算有内容。**理由**：另一方向是把用户声明的追加谓词
    /// 静默丢掉，触 §342 第 1 条的方向。**代价**：`{}` 这类空容器会被当成有内容。
    pub declared_completion_predicate: Option<serde_json::Value>,
}

/// `Blocked` / `Undetermined` 都**不是** §344 的 `Result` 本身；那个 `Result` 由调用方据
/// 本返回值组装（含逐项状态）。**优先级**：任一项为假 ⇒ `Blocked`；否则 ⇒ `Undetermined`
/// （第 2 项恒不可判，故本轮两个返回值都是「不得完成」）。
///
/// # 逐项判据（§8.3 的表格 + 第 10 项判据表的第四行由本计划补）
///
/// - 第 1 项 `ArtifactExists`：`artifact_exists` 为假 ⇒ 假；
/// - 第 2 项 `AllRequiredRequirementsVerified`：**恒不可判**（§3.4 第 3 条；`coverage`
///   的内容**不读**——读了就要发明 OPEN-001 缺的「充分」判据）；
/// - 第 3、4 项（`contract_satisfied` / `mandatory_effects_completed`）与第 6–9 项：
///   `None` ⇒ 不可判；`Some(false)` ⇒ 假；`Some(true)` ⇒ 真；
/// - 第 5 项 `NoBlockingVerificationFailure`：判据是「**不出现 `Failed`，且无
///   `AdjudicationRequired`**」——五臂里 `Passed` / `PassedWithUnknown` / `Unknown` 都为真；
/// - 第 10 项 `IndependentVerificationPass`（判据表）：`NotIndependent` ⇒ 假；
///   `NotEstablished` ⇒ 不可判；`Independent` ∧ 聚合 ∈ {`Passed`, `PassedWithUnknown`} ⇒ 真；
///   **`Independent` 而聚合非通过（`Failed` / `AdjudicationRequired` / `Unknown`）⇒ 假**
///   （§8.3 只给前三行，第四行由本计划补——不补它，「`Independent` ＋ `Failed`」这一格
///   无判据；方向取假，fail-closed）；
/// - 第十一项（**不是合取项但同等效力**，§8.3 末）：`declared_completion_predicate`
///   有内容 ⇒ 追加一项不可判；空/缺 ⇒ 不追加。**不解析它的内容**——解析就是发明语法。
///
/// # `Result` 今天的形态
///
/// 签名照设计 §8.4 照录；**本实现今天没有失败路径**（恒 `Ok`）——`Err` 的消费者未定
/// （§344 的 `Result` 由调用方组装），本块不发明它的产生条件。
pub fn judge_completion(input: &CompletionInput) -> Result<CompletionVerdict, VerifyError> {
    let mut failing: Vec<Conjunct> = Vec::new();
    let mut undecidable: Vec<UndecidableItem> = Vec::new();

    // 第 1 项：artifact_exists 为假 ⇒ 假。
    if !input.artifact_exists {
        failing.push(Conjunct::ArtifactExists);
    }

    // 第 2 项：恒不可判（§3.4 第 3 条）。**不读 coverage 的内容**——读了就是发明判据。
    undecidable.push(UndecidableItem::Conjunct(
        Conjunct::AllRequiredRequirementsVerified,
    ));

    // 第 3、4 项：`None` / `Some(false)` / `Some(true)` 三分。
    push_option(
        &mut failing,
        &mut undecidable,
        Conjunct::ContractSatisfied,
        input.contract_satisfied,
    );
    push_option(
        &mut failing,
        &mut undecidable,
        Conjunct::MandatoryEffectsCompleted,
        input.mandatory_effects_completed,
    );

    // 第 5 项：不出现 `Failed`，且无 `AdjudicationRequired`（§8.3 第 5 项的判据）。
    match input.verifier_verdicts {
        VerdictAggregate::Failed | VerdictAggregate::AdjudicationRequired => {
            failing.push(Conjunct::NoBlockingVerificationFailure);
        }
        VerdictAggregate::Passed
        | VerdictAggregate::PassedWithUnknown
        | VerdictAggregate::Unknown => {}
    }

    // 第 6–9 项：同第 3、4 项的三分。
    push_option(
        &mut failing,
        &mut undecidable,
        Conjunct::MandatoryTestsPass,
        input.mandatory_tests_pass,
    );
    push_option(
        &mut failing,
        &mut undecidable,
        Conjunct::ApplicablePropertiesHold,
        input.applicable_properties_hold,
    );
    push_option(
        &mut failing,
        &mut undecidable,
        Conjunct::ApplicableFuzzBudgetComplete,
        input.applicable_fuzz_budget_complete,
    );
    push_option(
        &mut failing,
        &mut undecidable,
        Conjunct::ApplicableMutationQualitySufficient,
        input.applicable_mutation_quality_sufficient,
    );

    // 第 10 项：判据表四格（§8.3 前三行 + 本计划补的第四行）。
    match input.independence {
        IndependenceVerdict::NotIndependent => failing.push(Conjunct::IndependentVerificationPass),
        IndependenceVerdict::NotEstablished => undecidable.push(UndecidableItem::Conjunct(
            Conjunct::IndependentVerificationPass,
        )),
        IndependenceVerdict::Independent => match input.verifier_verdicts {
            VerdictAggregate::Passed | VerdictAggregate::PassedWithUnknown => {}
            VerdictAggregate::Failed
            | VerdictAggregate::AdjudicationRequired
            | VerdictAggregate::Unknown => {
                failing.push(Conjunct::IndependentVerificationPass);
            }
        },
    }

    // 第十一项（§8.3 末）：有内容 ⇒ 追加一项不可判（fail-closed）；空/缺 ⇒ 不追加。
    if has_content(input.declared_completion_predicate.as_ref()) {
        undecidable.push(UndecidableItem::DeclaredCompletionPredicate);
    }

    // 优先级：假优先于不可判。
    if failing.is_empty() {
        Ok(CompletionVerdict::Undetermined { undecidable })
    } else {
        Ok(CompletionVerdict::Blocked { failing })
    }
}

/// 六项 `Option<bool>` 判据的同一条纪律（§8.4）：`None` ⇒ 不可判；`Some(false)` ⇒ 假；
/// `Some(true)` ⇒ 真。**`None` 与 `false` 是两件事**——压缩它们是把「没判」写成「判为假」，
/// 会让调用方去查一个不存在的失败。
fn push_option(
    failing: &mut Vec<Conjunct>,
    undecidable: &mut Vec<UndecidableItem>,
    conjunct: Conjunct,
    value: Option<bool>,
) {
    match value {
        None => undecidable.push(UndecidableItem::Conjunct(conjunct)),
        Some(true) => {}
        Some(false) => failing.push(conjunct),
    }
}

/// 「有内容」的取一（本 task 定；理由与代价见 [`CompletionInput::declared_completion_predicate`]）：
/// 无内容 ⟺ `None` 或 `Value::Null`；其余任何值都算有内容。
fn has_content(value: Option<&serde_json::Value>) -> bool {
    match value {
        None => false,
        Some(serde_json::Value::Null) => false,
        Some(_) => true,
    }
}
