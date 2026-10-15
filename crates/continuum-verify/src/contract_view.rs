//! §6.1 的 `ContractView` 一侧的类型。**本 task 只落 [`RequirementId`] 一枚**——
//! 它是 `EvidenceSubject::Requirement` 的载荷；`RequirementClass` /
//! `ContractRequirement` / `ContractView` 由 Task 5 追加到同一文件。

/// §224 的 `Requirement.id`（本块的承载；设计 §4.3.1 的订正段）。
///
/// # 与 P4 的 `continuum_semantics::RequirementId` 是两个类型
///
/// 同名、不同型，不共享 `From` / `Deref`，`rustc` 不会把一枚当另一枚。本块**不登记**
/// 指向 `continuum-semantics` 的边（设计 §6.1：以值承载是无条件的），故这枚第二份是
/// 该处置的代价；**两者之间的对应由装配方维持**——装配方从 P4 的 Contract 数据里取出
/// `Requirement.id`，构造本块这一枚，再填进 `EvidenceSubject::Requirement`。
///
/// # 派生 `PartialEq`
///
/// §6.3 第 1 条的判据是 `e.subject == EvidenceSubject::Requirement(r)`，归属判定按它
/// 比较；这与 P4 那枚（不派生 `PartialEq`、断言一律经 `as_str` 比较）的处置不同——
/// 那边没有按它比较的判定，本边有。`PartialEq` 不是构造路径（本类型的构造只有
/// [`RequirementId::from_str`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct RequirementId(String);

impl RequirementId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——§224 只给了字段名，没给取值域。
    ///
    /// 名字用 `from_str` 而没有实现 `std::str::FromStr`：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**（同 P4 的 id 类类型与本 crate 的 `EvidenceId`）。
    pub fn from_str(s: &str) -> Self {
        RequirementId(s.to_owned())
    }
}
