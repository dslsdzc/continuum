//! §6.1–§6.3 的 Requirement Coverage（§260 §190 §261）：迭代域（只判 `REQUIRED`）、
//! 两臂判定（[`CoverageVerdict`]）与计数规则（[`counts`]）。

use crate::contract_view::{ContractView, RequirementClass, RequirementId};
use crate::error::VerifyError;
use crate::evidence::{Evidence, EvidenceSubject};
use crate::evidence_type::EvidenceType;
use crate::profile::VerificationProfile;
use crate::test_validity::{TestValidity, Validity};

/// 一条 requirement 的判定结果（设计 §6.2）。
///
/// 字段为 `pub`：设计 §6.2 的代码块逐字如此（照录）。
#[derive(Debug, Clone, PartialEq)]
pub struct RequirementCoverage {
    /// 被判定的 requirement——从 [`ContractView`] 读出、本块以值承载的那一枚 id。
    pub requirement: RequirementId,
    /// 该 requirement 的判定。
    pub verdict: CoverageVerdict,
}

/// **只有两臂，没有正臂**（§6.2、§3.2）。`NotEstablished` 这个名字是刻意的：
/// 它不叫 `Unknown`——它说的是**本层没有判据、因而不放行**。
///
/// **不给 `Ord`、也不写该序的用例**：设计 §6.2 原写「两臂的严重度有序」，设计已自行订正为
/// 「**前半那一条今天没有载体**」（`:496-499`）——本层没有一处代码依赖该序。**本块与之同处置**：
/// 给一个规范没给的 `Ord` 就是发明（与 §4.3.4 对 `EvidenceStrength` 的处置同向）。
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageVerdict {
    /// **确定的欠缺**。§260 的 `verification = INCOMPLETE` 落在这一臂。
    Incomplete {
        /// 确定的欠缺是哪一种（§190 的无有效证据 / §261 的缺必需类型）。
        reason: IncompleteReason,
    },
    /// **没有确定的欠缺，但「充分」无判据**（OPEN-001）。它**不是**「满足」，也不是「不确定的满足」。
    NotEstablished,
}

impl CoverageVerdict {
    /// §6.2 的两臂。**本常量带数目字面量**，与
    /// [`EvidenceType::ALL`](crate::EvidenceType::ALL) 同形。
    ///
    /// `ALL` 只用于数臂与逐臂点名（`Incomplete` 的载荷取
    /// [`IncompleteReason::NoValidEvidence`] 作样例），不承诺载荷的代表性。
    /// 数目与逐项由 `tests/coverage.rs` 的 `the_two_verdict_enums_each_name_their_arms`
    /// 把关。
    pub const ALL: [CoverageVerdict; 2] = [
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence,
        },
        CoverageVerdict::NotEstablished,
    ];
}

/// [`CoverageVerdict::Incomplete`] 的两条理由（设计 §6.2）。
#[derive(Debug, Clone, PartialEq)]
pub enum IncompleteReason {
    /// §190：该 Requirement 没有有效证据（集合为空，或全部被判 `Invalid`）。
    NoValidEvidence,
    /// §261：`required_evidence_types` 里有类型在本 Requirement 的证据集合里缺席。
    MissingRequiredTypes {
        /// 缺席的类型——按 `required_evidence_types` 的传入序（不排序、不去重）。
        missing: Vec<EvidenceType>,
    },
}

impl IncompleteReason {
    /// §6.2 的两条理由。**本常量带数目字面量**，与
    /// [`EvidenceType::ALL`](crate::EvidenceType::ALL) 同形。
    ///
    /// `ALL` 只用于数臂与逐臂点名（`MissingRequiredTypes` 的载荷取空清单作样例），
    /// 不承诺载荷的代表性。数目与逐项由 `tests/coverage.rs` 的
    /// `the_two_verdict_enums_each_name_their_arms` 把关。
    pub const ALL: [IncompleteReason; 2] = [
        IncompleteReason::NoValidEvidence,
        IncompleteReason::MissingRequiredTypes {
            missing: Vec::new(),
        },
    ];
}

/// §6.3 的五条计数规则。**第 1 条是前提，2–4 条互斥且穷尽 `TestValidity` 的三种取值形态，
/// 第 5 条覆盖非测试证据**。
///
/// 逐条兑现：
/// 1. 归属不是 `Requirement(r)` 的证据一律不计数（`Unattached` 的证据不属于任何
///    Requirement）——先于其余各条判定，故不通过的证据直接出局；
/// 2. `Test` 且无对应 [`TestValidity`] 记录 ⇒ 不计数（§191 的原文：「测试不能因为
///    存在于 `test/` 目录里就自动被认为有效」）；
/// 3. `Test` 且记录判 [`Validity::Invalid`] ⇒ 不计数（「有效」由 §191 给）；
/// 4. `Test` 且记录判 `Valid`／`Unknown` ⇒ 计数（§123 的正式未知是「有判据但判不出」，
///    与「判为无效」是两件事）；
/// 5. 非 `Test` ⇒ 计数（§191 的判据只覆盖测试证据，规范未给非测试证据的「不计数」判据，
///    本设计不发明）。
///
/// **翻转条件**：这套规则今天只决定 `Incomplete` 与 `NotEstablished` 的分界；一旦
/// `CoverageVerdict` 加了正臂（§3.4），**第 2 与第 4 条立即变成载荷规则**（它们决定
/// 「哪些证据算数」，而那是正臂的直接输入），必须重审。
pub fn counts(e: &Evidence, r: &RequirementId, validity: &[TestValidity]) -> bool {
    // 第 1 条（前提）：归属判定。`Unattached` 与别的 requirement 都在这里出局。
    match e.subject() {
        EvidenceSubject::Requirement(subject) if subject == r => {}
        _ => return false,
    }
    // 第 5 条：非测试证据没有「不计数」的判据，直接计数。
    if e.evidence_type() != EvidenceType::Test {
        return true;
    }
    // 第 2 条：测试证据没有对应的有效性记录 ⇒ 不计数。记录按 `Evidence.id` 配对。
    let Some(record) = validity.iter().find(|v| &v.evidence == e.id()) else {
        return false;
    };
    // 第 3、4 条：`Invalid` 不计数；`Valid` / `Unknown` 计数。
    record.verdict != Validity::Invalid
}

/// 逐项判定 Contract 里 `class = REQUIRED` 的**全部**项（§6.1）。
///
/// 返回**逐项一条**，与 REQUIRED 项的**数目相同**（§342 第 1 条的唯一可执行守卫）：
/// 漏掉一项即数目不符。顺序即 `contract.requirements` 的传入序。
///
/// # 迭代域（§6.1）
///
/// 只有 [`RequirementClass::Required`] 的项进判定——§224 的四值里只有 `REQUIRED` 受
/// §190 的证据约束；本块只做 `match`，不判 `class` 的语义。
///
/// # 判定（§6.3 的两条 ⟺）
///
/// 计数集为空 ⇒ `Incomplete { NoValidEvidence }`；否则 `required_evidence_types` 的
/// 某一型在计数集里不出现 ⇒ `Incomplete { MissingRequiredTypes }`；两者皆不成立 ⇒
/// `NotEstablished`（**不是**「满足」：本层只判确定的欠缺，§3.2）。
///
/// # `Result` 今天的形态
///
/// 签名照设计 §6.1 照录；**本实现今天没有失败路径**（恒 `Ok`）——档案缺失已在取得点
/// [`profile_for`](crate::profile_for) 拒掉，故这里收的是已有档案的引用。
pub fn coverage(
    contract: &ContractView,
    evidence: &[Evidence],
    validity: &[TestValidity],
    profile: &VerificationProfile,
) -> Result<Vec<RequirementCoverage>, VerifyError> {
    let mut rows = Vec::new();
    for requirement in &contract.requirements {
        // §6.1 的迭代域：只有 REQUIRED 进判定。
        if requirement.class != RequirementClass::Required {
            continue;
        }

        let counted: Vec<&Evidence> = evidence
            .iter()
            .filter(|e| counts(e, &requirement.id, validity))
            .collect();

        let verdict = if counted.is_empty() {
            // §190：没有有效证据（集合为空，或全部被判 Invalid）。
            CoverageVerdict::Incomplete {
                reason: IncompleteReason::NoValidEvidence,
            }
        } else {
            // §261：必需类型在计数集里缺席的逐型列表，按 profile 的传入序。
            let mut missing = Vec::new();
            for required_type in &profile.required_evidence_types {
                if !counted.iter().any(|e| e.evidence_type() == *required_type) {
                    missing.push(*required_type);
                }
            }
            if missing.is_empty() {
                // 没有确定的欠缺——但「充分」无判据（OPEN-001），故不放行。
                CoverageVerdict::NotEstablished
            } else {
                CoverageVerdict::Incomplete {
                    reason: IncompleteReason::MissingRequiredTypes { missing },
                }
            }
        };

        rows.push(RequirementCoverage {
            requirement: requirement.id.clone(),
            verdict,
        });
    }
    Ok(rows)
}
