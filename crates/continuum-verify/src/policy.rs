//! §236 的节点级 `VerificationPolicy`（设计 §5.1）。

use serde_json::Value;

use crate::error::VerifyError;
use crate::evidence_type::EvidenceType;
use crate::verifier_level::VerifierLevel;

/// §236 的 `verification_policy`。**不是** §261 的 `VerificationProfile`：
/// 后者按**任务**，本型按**节点**（§236 的字段挂在 Node 上）。
///
/// # 派生清单（本 task 定的形状）
///
/// `Default`（设计 §5.1 明写）：两个字段都取「不限」——`min_verifier_level = None`、
/// `required_evidence_types = []`；`Node::new` 的 `Value::Null` 解析到它。
/// **它是「无限定」不是「无要求」**：任务级 `VerificationProfile` 仍然适用。
/// `Debug` / `Clone` / `PartialEq` 同 `TestValidity`：不产生构造路径，在用例的
/// `Result` 断言上承压。**不派生** `Serialize` / `Deserialize`：本型的解析点只有
/// [`VerificationPolicy::from_value`] 一个（§5.1 的「唯一解析点」），让 serde 再开一条
/// 读路径就会绕过它的两侧守卫。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VerificationPolicy {
    /// §262 的五级序里的**最低可接受级**。`None` = 不限，按 §262 的顺序取最高可用级。
    pub min_verifier_level: Option<VerifierLevel>,
    /// 本节点**额外**要求产出的证据类型（任务级 `VerificationProfile` 之外的要求）。
    pub required_evidence_types: Vec<EvidenceType>,
}

impl VerificationPolicy {
    /// §5.1 的唯一解析点。两侧：`Null` ⇒ `Default`（无限定，**不是「无要求」**）；
    /// **非空但不认识 ⇒ `Err`**（一个写着内容的策略被当成空策略，正是「静默放宽」；
    /// 与 `ArtifactType::parse` 对未知串返回 `None` 同形）。
    ///
    /// 「认识」的判据（§5.1 第 2 条的四类输入逐类拒绝）：值必须是**恰好**含
    /// [`min_verifier_level`](Self::min_verifier_level) 与
    /// [`required_evidence_types`](Self::required_evidence_types) 两个字段的对象——
    /// 缺字段、多出未定义字段都算不认识；`min_verifier_level` 取 `null`（不限）或
    /// 五级序内的串（[`VerifierLevel::parse`]），其余串拒绝；`required_evidence_types`
    /// 取数组且每项是 [`EvidenceType::parse`] 认识的串，其余项拒绝。
    /// **非对象、非 `Null` 的值同样拒绝**：既不是 `Null` 也不是本型的形态。
    pub fn from_value(v: &Value) -> Result<VerificationPolicy, VerifyError> {
        // 第一侧：`Null`（`Node::new` 的初值）⇒ `Default`。
        if v.is_null() {
            return Ok(VerificationPolicy::default());
        }

        // 第二侧：其余一切不是「恰好两个已知字段」的输入都拒绝。
        let Some(obj) = v.as_object() else {
            return Err(VerifyError::UnknownVerificationPolicy);
        };
        let Some(level_value) = obj.get("min_verifier_level") else {
            return Err(VerifyError::UnknownVerificationPolicy);
        };
        let Some(types_value) = obj.get("required_evidence_types") else {
            return Err(VerifyError::UnknownVerificationPolicy);
        };
        // 两个已知字段都在场后，任何第三个键都是「多出未定义字段」。
        if obj.len() != 2 {
            return Err(VerifyError::UnknownVerificationPolicy);
        }

        // `min_verifier_level`：`null` = 不限；其余按五级序的串解析。
        let min_verifier_level = if level_value.is_null() {
            None
        } else {
            let Some(level_str) = level_value.as_str() else {
                return Err(VerifyError::UnknownVerificationPolicy);
            };
            let Some(level) = VerifierLevel::parse(level_str) else {
                return Err(VerifyError::UnknownVerificationPolicy);
            };
            Some(level)
        };

        // `required_evidence_types`：数组，每项必须是认识的证据类型名。
        let Some(items) = types_value.as_array() else {
            return Err(VerifyError::UnknownVerificationPolicy);
        };
        let mut required_evidence_types = Vec::with_capacity(items.len());
        for item in items {
            let Some(type_str) = item.as_str() else {
                return Err(VerifyError::UnknownVerificationPolicy);
            };
            let Some(evidence_type) = EvidenceType::parse(type_str) else {
                return Err(VerifyError::UnknownVerificationPolicy);
            };
            required_evidence_types.push(evidence_type);
        }

        Ok(VerificationPolicy {
            min_verifier_level,
            required_evidence_types,
        })
    }
}
