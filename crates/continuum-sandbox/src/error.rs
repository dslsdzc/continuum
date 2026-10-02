//! 内核层隔离的错误类型。

/// 沙箱操作失败的原因。
///
/// 变体按失败发生的位置划分，这是调用方的分派依据：
///
/// - [`SandboxError::MechanismUnavailable`]：机制本身不可用（可执行文件不在 PATH 中、
///   内核缺少该机制所需的支持）。判定发生在启动任何进程之前。
/// - [`SandboxError::IsolationFailed`]：隔离规则集的构建或施加失败。此时子进程尚未
///   被启动，调用方可以改用别的机制重试。
/// - [`SandboxError::SpawnFailed`]：隔离之外的原因使进程未能启动（可执行文件不存在、
///   权限不足等）。
///
/// 三者的分界意图是：「机制不可用」与「隔离失败」都发生在子进程存在之前，
/// 调用方据此决定改用另一机制还是拒绝运行。
///
/// 本枚举标 `#[non_exhaustive]`——两种机制（Task 6、Task 7）落地时各自会暴露新的
/// 失败形态，外部消费者不应因增补变体而改动 `match`。新增变体时仍应优先考虑既有的
/// 三个能否表达。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SandboxError {
    /// 机制不可用。
    ///
    /// `mechanism` 是机制名（与 [`crate::SandboxCapabilities::mechanism`] 同源），
    /// `reason` 给出不可用的原因，供调用方与报告使用。
    #[error("沙箱机制 {mechanism} 不可用：{reason}")]
    MechanismUnavailable {
        mechanism: &'static str,
        reason: String,
    },
    /// 隔离规则集未能建立或施加。
    #[error("沙箱机制 {mechanism} 施加隔离失败：{reason}")]
    IsolationFailed {
        mechanism: &'static str,
        reason: String,
    },
    /// 子进程启动失败。
    #[error("子进程启动失败：{reason}")]
    SpawnFailed { reason: String },
}
