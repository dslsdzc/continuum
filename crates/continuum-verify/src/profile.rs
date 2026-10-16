//! §261 的任务级 `VerificationProfile` 与它的取得点 `profile_for`（设计 §5.2、§5.3）。

use crate::error::VerifyError;
use crate::evidence_type::EvidenceType;

/// §261 的六个字段。`required_evidence_types` 与 `independent_verifier_required` **进判定**；
/// 其余四项**只存不判**（§261 未给它们的用途或量纲）。
/// **不取 `Vec<String>`**：集合包含判定要编译期穷尽（设计 §5.2）。
///
/// # 派生清单（本 task 定的形状）
///
/// `Debug` / `Clone` / `PartialEq`（同 [`TestValidity`](crate::test_validity::TestValidity)）：
/// 不产生构造路径；`Clone` 是 [`profile_for`] 从引用返回逐值所需的复制，`Debug` 与
/// `PartialEq` 在用例的 `Result` 断言上承压。**不派生** `Default`：它会凭空给出
/// 「无要求」的档案，而 §5.3 第 3 条说无档案是 `Err(ProfileMissing)`——`Default`
/// 正是那条 fail-open 路径的类型面承载。也不派生 `Serialize` / `Deserialize`：
/// 本型的形态与取得点由 §5.3 钉，不取 serde 表示。
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationProfile {
    pub required_evidence_types: Vec<EvidenceType>,
    pub optional_evidence_types: Vec<EvidenceType>,
    pub fuzz_budget: Option<serde_json::Value>,
    pub mutation_threshold: Option<serde_json::Value>,
    pub property_requirements: Vec<serde_json::Value>,
    pub independent_verifier_required: bool,
}

/// 取得该任务验证档案的**来源**。**它的形态本轮不决定**（来自 Contract 的哪一字段？
/// 还是另一个输入？取决于 `OPEN-009` 怎么解，设计 §5.3、§14 第 1 条）。
/// 本计划只取「有档案」与「没有档案」两臂——**这两臂是 §5.3 第 3 条要钉的东西**。
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationProfileSource {
    /// 有档案：由装配方以**值**传入（生成者未定，见 §5.3 第 1 条）。
    Provided(VerificationProfile),
    /// 没有档案：`profile_for` 对它的答复是 `Err(ProfileMissing)`。
    Missing,
}

/// **没有档案时返回 `Err(ProfileMissing)`，不返回「无要求」的档案**——
/// 后者会让每项 REQUIRED 的 required 集合为空、覆盖判定无缺项，即
/// 「无档案 ⇒ 无要求 ⇒ 不阻断」，正是 fail-open 的那一侧（设计 §5.3 第 3 条）。
pub fn profile_for(source: &VerificationProfileSource) -> Result<VerificationProfile, VerifyError> {
    match source {
        VerificationProfileSource::Provided(profile) => Ok(profile.clone()),
        VerificationProfileSource::Missing => Err(VerifyError::ProfileMissing),
    }
}
