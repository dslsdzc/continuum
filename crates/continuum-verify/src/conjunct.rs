//! §8.1 的十项合取与「不可判项」的承载（设计 §8.1、§8.2、§8.3 末）。

/// §266 的五项与 §195 的八项的**并集**（两者不是同一张表的两个抄本：§195 有五项在 §266 里
/// 没有对应物；只取 §266 会把它静默丢掉，而 §195 的标题正是「Completion Predicate **强化**」，
/// 丢掉它是**放宽**，触 §342 第 1 条的方向）。**两条去重的理由见用例注释**
/// （`tests/completion.rs` 的 `the_ten_conjuncts_come_from_the_union_of_266_and_195`）。
///
/// **本型今天没有 `as_str` / `parse`**（同 [`VerifierVerdict`](crate::verdict::VerifierVerdict)
/// 的处置）：它没有落库列（设计 §10.2 未给承载列；Task 9 才处置落库与否），
/// 没有第二处编码的需要，也不造一个。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conjunct {
    /// §266 第 1 项：受验图的终端节点各自至少有一个产出制品。
    ArtifactExists,
    /// §266 第 2 项（≈§195 的 `requirement_coverage_sufficient`；合并只在**方向**上安全，
    /// 见用例注释）。**判定恒 `Undetermined`**（§3.4 第 3 条）。
    AllRequiredRequirementsVerified,
    /// §266 第 3 项（≈§195 的 `task_contract_satisfied`：判据同一——§225 的
    /// Constraint Validator 的结论）。
    ContractSatisfied,
    /// §266 第 4 项。
    MandatoryEffectsCompleted,
    /// §266 第 5 项。
    NoBlockingVerificationFailure,
    /// §195。
    MandatoryTestsPass,
    /// §195。
    ApplicablePropertiesHold,
    /// §195。
    ApplicableFuzzBudgetComplete,
    /// §195。
    ApplicableMutationQualitySufficient,
    /// §195。
    IndependentVerificationPass,
}

impl Conjunct {
    /// §8.1 的十项，逐项照录。**本常量带数目字面量**，与
    /// [`EvidenceType::ALL`](crate::evidence_type::EvidenceType::ALL) 同形。
    ///
    /// 顺序即 §8.1 的声明序：前五项出 §266 第 1–5 项、后五项出 §195。完整性与数目由
    /// `tests/completion.rs` 的 `the_ten_conjuncts_come_from_the_union_of_266_and_195`
    /// 把关（逐臂点名，不抽代表）。
    pub const ALL: [Conjunct; 10] = [
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
    ];
}

/// [`CompletionVerdict::Undetermined`](crate::completion::CompletionVerdict::Undetermined)
/// 的 `undecidable` 列表的一项。
///
/// # 本型是对任务材料的一处最小修复（申报见 `tests/completion.rs` 文件头与报告）
///
/// 设计 §8.2 把该列表的元素类型写作 [`Conjunct`]，而 §8.3 末的**第十一项**
/// （`Intent.completion_predicate` 的声明值：「**不是合取项**但同等效力」）在十臂的
/// [`Conjunct`] 里无处安放；§8.4 与 §12.1 (r) 又要它「**追加一项**不可判」、且红条件要求
/// 「有内容 vs 无内容」在返回值上可观察。本枚举两臂使「追加一项」在类型与返回值上都成立：
/// 合取项、或第十一项。**判定语义不变**（十臂、两臂、「假优先于不可判」都不动）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndecidableItem {
    /// 十项合取中的一项无判据或判不出。
    Conjunct(Conjunct),
    /// §8.3 末的第十一项：`Intent.completion_predicate` **有内容**（本块只判有无内容，
    /// **不解析它**——解析就是发明语法）而**无判据** ⇒ 不可判（fail-closed，§342 第 1 条）。
    DeclaredCompletionPredicate,
}

impl UndecidableItem {
    /// 两臂，逐臂照录。**本常量带数目字面量**，与 [`Conjunct::ALL`] 同形。
    ///
    /// 载荷取样例（`Conjunct` 臂取恒不可判的 `AllRequiredRequirementsVerified`），
    /// 不承诺代表性；数目与逐臂由 `tests/completion.rs` 的 `the_two_verdict_arms_are_named`
    /// 把关。
    pub const ALL: [UndecidableItem; 2] = [
        UndecidableItem::Conjunct(Conjunct::AllRequiredRequirementsVerified),
        UndecidableItem::DeclaredCompletionPredicate,
    ];
}
