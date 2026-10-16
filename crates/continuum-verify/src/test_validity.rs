//! §191 的 `TestValidity`：五个检查、三档判定与聚合（设计 §6.4）。
//!
//! 本文件装设计 §6.4 的四处形状：检查名单（[`TestValidityCheck`]）、单条判定
//! （[`CheckVerdict`]）、聚合判定（[`Validity`]）与记录本体（[`TestValidity`]）。

use crate::error::VerifyError;
use crate::evidence::EvidenceId;

/// §191 的五个检查，逐条照录成闭合枚举（设计 §6.4）。
///
/// 检查名对应 §191 的列表（`docs/spec/04-method.md:298`）：测试对应哪条 Requirement /
/// 断言是否足够强 / 错误实现是否可能仍通过 / Mock 是否绕过真实路径 / 边界条件是否缺失。
/// **封闭枚举**：开放字符串会把「五臂齐了吗」退化成字符串比较，而漏判的方向是
/// 「以为齐了」——处置同 §4.2 对证据类型的理由。
///
/// 字符串名在 [`TestValidityCheck::as_str`] / [`TestValidityCheck::parse`] 上，是该编码
/// 唯一的产生点（同 [`EvidenceType`](crate::evidence_type::EvidenceType)）：本 crate 的
/// 落库列（`test_validity_check.check_name`，Task 9）取用它，不取 serde 表示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestValidityCheck {
    /// §191 第 1 条：测试对应哪条 Requirement。
    RequirementMapping,
    /// §191 第 2 条：断言是否足够强。
    AssertionStrength,
    /// §191 第 3 条：错误实现是否可能仍通过。
    ErrorImplWouldPass,
    /// §191 第 4 条：Mock 是否绕过真实路径。
    MockBypassesRealPath,
    /// §191 第 5 条：边界条件是否缺失。
    MissingBoundary,
}

impl TestValidityCheck {
    /// §191 的五个检查。**本常量带数目字面量**，与 `EvidenceType::ALL` 同形。
    ///
    /// 完整性与数目由 `tests/test_validity.rs` 的 `the_three_enums_each_name_their_arms`
    /// 把关。
    pub const ALL: [TestValidityCheck; 5] = [
        TestValidityCheck::RequirementMapping,
        TestValidityCheck::AssertionStrength,
        TestValidityCheck::ErrorImplWouldPass,
        TestValidityCheck::MockBypassesRealPath,
        TestValidityCheck::MissingBoundary,
    ];

    /// 各检查的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RequirementMapping => "requirement_mapping",
            Self::AssertionStrength => "assertion_strength",
            Self::ErrorImplWouldPass => "error_impl_would_pass",
            Self::MockBypassesRealPath => "mock_bypasses_real_path",
            Self::MissingBoundary => "missing_boundary",
        }
    }

    /// [`TestValidityCheck::as_str`] 的**严格逆**：对 [`TestValidityCheck::ALL`] 里的每臂
    /// 都有 `TestValidityCheck::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认臂：落库列读出不认识的串说明库里有本版本不认得的检查名，取默认
    /// 会把一份别的东西当成本检查处理（理由同 `EvidenceType::parse`）。
    ///
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由
    /// `tests/test_validity.rs` 的 `the_three_enums_each_name_their_arms` 遍历
    /// [`TestValidityCheck::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "requirement_mapping" => Self::RequirementMapping,
            "assertion_strength" => Self::AssertionStrength,
            "error_impl_would_pass" => Self::ErrorImplWouldPass,
            "mock_bypasses_real_path" => Self::MockBypassesRealPath,
            "missing_boundary" => Self::MissingBoundary,
            _ => return None,
        })
    }
}

/// 单条检查的三档判定（设计 §6.4）。
///
/// `Unknown` 是**正式档**，不是待补的空位：§6.4 的聚合规则按 §123 的「正式未知」对待它，
/// 不把它折进 `Fail`——那会把「判不出」说成「判为否」。
///
/// 字符串名在 [`CheckVerdict::as_str`] / [`CheckVerdict::parse`] 上，是该编码唯一的
/// 产生点：本 crate 的落库列（`test_validity_check.verdict`，Task 9）取用它。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckVerdict {
    /// 该条检查通过。
    Pass,
    /// 该条检查不通过。
    Fail,
    /// 该条检查判不出（§123 的正式未知）。
    Unknown,
}

impl CheckVerdict {
    /// 三档判定。**本常量带数目字面量**，与 `EvidenceType::ALL` 同形。
    pub const ALL: [CheckVerdict; 3] = [
        CheckVerdict::Pass,
        CheckVerdict::Fail,
        CheckVerdict::Unknown,
    ];

    /// 三档的字符串名：小写。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unknown => "unknown",
        }
    }

    /// [`CheckVerdict::as_str`] 的**严格逆**：对 [`CheckVerdict::ALL`] 里的每臂都有
    /// `CheckVerdict::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`（理由同
    /// [`TestValidityCheck::parse`]）。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pass" => Self::Pass,
            "fail" => Self::Fail,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }
}

/// 测试有效性记录的聚合判定（设计 §6.4 的三档）。
///
/// `Unknown` 与 [`CheckVerdict::Unknown`] 同源（§123 的正式未知）：聚合规则把「无 `Fail`
/// 但有 `Unknown`」判为它，而**不是**降格为 `Invalid`。
///
/// 字符串名在 [`Validity::as_str`] / [`Validity::parse`] 上，是该编码唯一的产生点：
/// 本 crate 的落库列（`test_validity.verdict`，Task 9）取用它。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Validity {
    /// 五条检查全 `Pass`。
    Valid,
    /// 至少一条检查 `Fail`。
    Invalid,
    /// 无 `Fail`，但至少一条判不出。
    Unknown,
}

impl Validity {
    /// 三档判定。**本常量带数目字面量**，与 `EvidenceType::ALL` 同形。
    pub const ALL: [Validity; 3] = [Validity::Valid, Validity::Invalid, Validity::Unknown];

    /// 三档的字符串名：小写。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
            Self::Unknown => "unknown",
        }
    }

    /// [`Validity::as_str`] 的**严格逆**：对 [`Validity::ALL`] 里的每臂都有
    /// `Validity::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`——读出不认识的串
    /// 说明库里有本版本不认得的判定（理由同 [`TestValidityCheck::parse`]；落库列读回
    /// 由 Task 9 报错）。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "valid" => Self::Valid,
            "invalid" => Self::Invalid,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }
}

/// §191 的判定记录（设计 §6.4）。**有效性是独立的记录，不是 `Evidence` 的一个字段**——
/// 否则「验证者改了被验之物」（设计 §4.1、§6.4）。
///
/// # 字段为 `pub`（照设计 §6.4 原文）
///
/// 与 [`Evidence`](crate::evidence::Evidence) 的「八字段全私有＋唯一产生点」是**两种处置**，
/// 据实分清：`Evidence` 的唯一产生点要挡的是「第二条构造路径」（§4.4），而本类型的字段照
/// 设计原文是 `pub`，故「五臂齐」与「`verdict` 由构造点算」是**构造路径上的保证**，不是
/// 类型级保证——绕过 [`TestValidity::new`] 的字面量构造仍可写出不齐的 `checks` 或不一致的
/// `verdict`。本 task 照设计原文照录字段可见性，不擅改。
///
/// # 派生清单（本 task 定的形状）
///
/// `Debug` / `Clone` / `PartialEq`（同 [`Evidence`](crate::evidence::Evidence)）：不产生构造
/// 路径；`Debug` 与 `PartialEq` 在用例的 `Result` 断言上承压。**不派生** `Default` / `From` /
/// `FromStr` / `Serialize` / `Deserialize`：不为 [`TestValidity::new`] 之外再造解码／默认
/// 构造路径（`Deserialize` 与 `Default` 各能凭空给出不经形状校验的记录；证据记录的落库
/// 不取 serde 表示，编码列经 `as_str` / `parse`）。
#[derive(Debug, Clone, PartialEq)]
pub struct TestValidity {
    /// 该记录判定的证据（§258 的 `Evidence.id`）。
    pub evidence: EvidenceId,
    /// 五个检查各一条（§6.4：五臂齐且不重复；由 [`TestValidity::new`] 校验）。
    pub checks: Vec<(TestValidityCheck, CheckVerdict)>,
    /// 聚合判定：由 [`TestValidity::new`] 按聚合规则算出，不由调用方给。
    pub verdict: Validity,
}

impl TestValidity {
    /// §191 判定记录的构造点。**要求 `checks` 五臂齐且不重复**（缺臂或重复臂返回
    /// [`VerifyError::IncompleteTestValidityChecks`]，两个名单随错误带回）：这些检查里任何
    /// 一条都可能是唯一发现「测试无效」的那条，静默缺一条等于静默放宽；「重复补齐五条」
    /// 是同一件事的另一条路——重复臂把缺席的臂静默填平。
    ///
    /// **`verdict` 由本构造点按聚合规则算出**，不由调用方给——`new` 的入参里没有这一项，
    /// 故经由 `new` 构造的记录里「`checks` 与 `verdict` 不一致」写不出来（字面量构造绕过
    /// `new` 的残余见 [`TestValidity`] 的文档）。
    ///
    /// **聚合规则（本设计定，规范未给）**：任一 `Fail` ⇒ `Invalid`；五条全 `Pass` ⇒ `Valid`；
    /// 其余（无 `Fail` 但有 `Unknown`）⇒ `Unknown`。取「任一 `Fail` 即整条失效」的理由是它是
    /// 三档里**更严的一档**：代价是可能多判为无效，而不是漏判。
    ///
    /// **本构造点只做形状校验与聚合，不产生检查结论**：五个检查的判定由调用方（将来的
    /// 产生者）给出；「谁跑这五个检查、输入取自哪里」规范未给，见设计 §6.5 的缺口。
    pub fn new(
        evidence: EvidenceId,
        checks: Vec<(TestValidityCheck, CheckVerdict)>,
    ) -> Result<TestValidity, VerifyError> {
        // 五臂齐且不重复：对 ALL 逐臂数出现次数。missing / repeated 都按 ALL 的顺序生成，
        // 错误载荷的顺序因此是确定的。
        let mut missing = Vec::new();
        let mut repeated = Vec::new();
        for check in TestValidityCheck::ALL {
            let count = checks.iter().filter(|(arm, _)| *arm == check).count();
            if count == 0 {
                missing.push(check);
            } else if count > 1 {
                repeated.push(check);
            }
        }
        if !missing.is_empty() || !repeated.is_empty() {
            return Err(VerifyError::IncompleteTestValidityChecks { missing, repeated });
        }

        // 聚合规则（§6.4）：任一 Fail ⇒ Invalid；五条全 Pass ⇒ Valid；其余 ⇒ Unknown。
        let verdict = if checks.iter().any(|pair| pair.1 == CheckVerdict::Fail) {
            Validity::Invalid
        } else if checks.iter().all(|pair| pair.1 == CheckVerdict::Pass) {
            Validity::Valid
        } else {
            Validity::Unknown
        };

        Ok(TestValidity {
            evidence,
            checks,
            verdict,
        })
    }
}
