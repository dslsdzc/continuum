//! §234 的 `Milestone` 身份。
//!
//! `Milestone` 本体、七状态与惰性生成在 Task 7 补齐；Task 4 的 `Product` 需要
//! `current_milestone`／`milestone_history` 两个字段先能命名本类型。

/// P7A 的 Milestone 身份。不透明串。
///
/// `Milestone` 的 id 由 `propose_next_batch` 赋；`from_str` 只供读回与测试构造，
/// 不是生产入口。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MilestoneId(String);

impl MilestoneId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 只供存储解码与测试构造。生产路径由 `propose_next_batch` 派生
    /// （`milestone/{product_id}/{ordinal}`）。
    pub fn from_str(s: &str) -> Self {
        Self(s.to_owned())
    }
}
