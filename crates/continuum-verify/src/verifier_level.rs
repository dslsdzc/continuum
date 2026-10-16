//! §262 的五级序：`VerifierLevel`（设计 §7.1）。

/// §262 的五臂，**声明顺序即优先序**（`Ord` 由声明序派生）。
/// §262 的「可确定性验证 SHOULD 优先于模型判断」是 SHOULD；本块实现成**严格取最高可用级**
/// （把 SHOULD 当 MUST 用）——这是**收紧**，不触 §342 第 1 条，据实记在 `## 遗留`。
///
/// # 派生清单（本 task 定的形状）
///
/// `Ord`（连同 `PartialOrd`）由声明序派生——**声明序即 §262 的优先序**，改一次声明序
/// 就改一次选择结果；四条相邻序的守卫是 `tests/policy.rs` 的
/// `the_five_levels_are_declared_in_the_262_priority_order`（不抽代表）。
/// `Copy` / `Eq` 等照 [`EvidenceType`](crate::evidence_type::EvidenceType) 的形状；
/// **不派生** `Default`（五级里没有哪一级是默认）与 `Serialize` / `Deserialize`
/// （本型不进落库列；从值解析走 [`VerifierLevel::parse`]，即 §5.1 的唯一解析点的一支）。
///
/// 字符串名在 [`VerifierLevel::as_str`] / [`VerifierLevel::parse`] 上，是该编码唯一的
/// 产生点（同 `EvidenceType`）：`VerificationPolicy` 的 `min_verifier_level` 从值解析时
/// 取用它，不取 serde 表示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerifierLevel {
    /// §262 第 1 级（deterministic checker）。
    DeterministicChecker,
    /// §262 第 2 级（formal/static checker）。
    FormalStaticChecker,
    /// §262 第 3 级（independent model verifier）。
    IndependentModelVerifier,
    /// §262 第 4 级（cross-family verifier）。
    CrossFamilyVerifier,
    /// §262 第 5 级（human review）。
    HumanReview,
}

impl VerifierLevel {
    /// §262 的五级，逐级照录。**本常量带数目字面量**，与 `EvidenceType::ALL`
    /// （`crate::evidence_type::EvidenceType`）同形。
    ///
    /// **不据 §262 题头的「默认顺序」推别处还有级**：本枚举只照录这五级，不声明
    /// 规范只承认它们。数目与顺序由 `tests/policy.rs` 的
    /// `the_five_levels_are_declared_in_the_262_priority_order` 把关。
    pub const ALL: [VerifierLevel; 5] = [
        VerifierLevel::DeterministicChecker,
        VerifierLevel::FormalStaticChecker,
        VerifierLevel::IndependentModelVerifier,
        VerifierLevel::CrossFamilyVerifier,
        VerifierLevel::HumanReview,
    ];

    /// 五级的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// 串名没有规范逐字出处（§262 只给级别名）：五枚串名是本块的命名，照 §4.2 的口径。
    /// `VerificationPolicy::from_value` 的 `min_verifier_level` 与它互为严格逆
    /// （[`VerifierLevel::parse`]）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeterministicChecker => "deterministic_checker",
            Self::FormalStaticChecker => "formal_static_checker",
            Self::IndependentModelVerifier => "independent_model_verifier",
            Self::CrossFamilyVerifier => "cross_family_verifier",
            Self::HumanReview => "human_review",
        }
    }

    /// [`VerifierLevel::as_str`] 的**严格逆**：对 [`VerifierLevel::ALL`] 里的每级都有
    /// `VerifierLevel::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认级：从值里读出五级序之外的串说明写着本版本不认得的级别，
    /// 取默认会把一份别的东西当成本级处理（理由同 `EvidenceType::parse`）；调用方
    /// （[`VerificationPolicy::from_value`](crate::policy::VerificationPolicy::from_value)）
    /// 把 `None` 升为 `Err(UnknownVerificationPolicy)`——静默取默认正是「静默放宽」。
    ///
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由
    /// `tests/policy.rs` 的 `every_level_round_trips_through_its_encoding` 遍历
    /// [`VerifierLevel::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "deterministic_checker" => Self::DeterministicChecker,
            "formal_static_checker" => Self::FormalStaticChecker,
            "independent_model_verifier" => Self::IndependentModelVerifier,
            "cross_family_verifier" => Self::CrossFamilyVerifier,
            "human_review" => Self::HumanReview,
            _ => return None,
        })
    }
}
