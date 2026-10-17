//! §321 的五类语义，逐字（设计 §4.4）。

/// §321 的五类语义。`as_str` 返回规范原文（含空格与大小写）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtectedSemantic {
    Authority,
    Contract,
    Permission,
    Approval,
    SecurityPolicy,
}

impl ProtectedSemantic {
    /// 全部五枚。
    pub const ALL: [ProtectedSemantic; 5] = [
        ProtectedSemantic::Authority,
        ProtectedSemantic::Contract,
        ProtectedSemantic::Permission,
        ProtectedSemantic::Approval,
        ProtectedSemantic::SecurityPolicy,
    ];

    /// §321 原文，逐字（不得改写大小写或空格）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authority => "Authority semantics",
            Self::Contract => "Contract semantics",
            Self::Permission => "Permission invariants",
            Self::Approval => "User approval requirements",
            Self::SecurityPolicy => "Core security policy",
        }
    }

    /// [`ProtectedSemantic::as_str`] 的严格逆：表外串一律 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|x| x.as_str() == s)
    }
}
