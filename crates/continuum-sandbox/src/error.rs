//! 内核层隔离的错误类型。

/// 沙箱操作失败的原因。
///
/// 变体按失败发生的位置划分：
///
/// - [`SandboxError::MechanismUnavailable`]：机制在本机不存在或不可用——例如 bubblewrap
///   需要的 `bwrap` 可执行文件不在 PATH 中，或内核的 Landlock ABI 为 0（Landlock 整个
///   不存在）。判定发生在启动子进程之前。
/// - [`SandboxError::IsolationFailed`]：本层在**父进程侧**为施加隔离而失败
///   （例如构建规则集）。
/// - [`SandboxError::SpawnFailed`]：子进程启动失败。
///
/// **`IsolationFailed` 与 `SpawnFailed` 的分界在调用方一侧不总是可判。** 设计第 4.3 节
/// 要求 Landlock 在 `pre_exec`（fork 之后、exec 之前）内施加，那里失败时 std 把它报成
/// 普通的 spawn `io::Error`，与 exec 失败同形，本层无从分辨。故调用方：
///
/// - 不得据「拿到了 [`SandboxError::IsolationFailed`]」推断「子进程未存在」；
/// - 不得据「拿到了 [`SandboxError::SpawnFailed`]」推断「隔离已施加」。
///
/// 两个变体都发生在子进程**交付给调用方之前**，故调用方在任何一种情形下都拿不到
/// 可用的 `Child`——这是可以依赖的那一半。
///
/// 本枚举标 `#[non_exhaustive]`——两种机制（Task 6、Task 7）落地时各自会暴露新的
/// 失败形态，外部消费者不应因增补变体而改动 `match`。新增变体时仍应优先考虑既有的
/// 三个能否表达。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SandboxError {
    /// 机制在本机不可用。
    ///
    /// `mechanism` 是机制名（与 [`crate::SandboxCapabilities::mechanism`] 同源），
    /// `reason` 给出不可用的原因，供调用方与报告使用。
    ///
    /// **内核 ABI 的两种「不足」在这里分界，处置相反**（设计第 4.3 节）：
    ///
    /// - **ABI < 1（Landlock 整个不可用）属本变体**：该机制在本机不存在，须拒绝启动。
    ///   放行等于让子进程在零文件系统隔离下运行，而「[`crate::SandboxCapabilities`]
    ///   会报假」不足以弥补——调用方可以不查那个报告。
    /// - **ABI ≥ 1 但某个访问类别不被支持不属本变体**：降级该类别、启动照常，
    ///   实际隔离面由 [`crate::SandboxCapabilities`] 如实反映。
    #[error("沙箱机制 {mechanism} 不可用：{reason}")]
    MechanismUnavailable {
        mechanism: &'static str,
        reason: String,
    },
    /// 本层在父进程侧为施加隔离而失败（构建规则集等）。
    #[error("沙箱机制 {mechanism} 施加隔离失败：{reason}")]
    IsolationFailed {
        mechanism: &'static str,
        reason: String,
    },
    /// 子进程启动失败。
    ///
    /// exec 失败与 `pre_exec` 内施加隔离失败在此合流，调用方据本变体区分不出是哪一条
    /// （见枚举文档）。
    #[error("子进程启动失败：{reason}")]
    SpawnFailed { reason: String },
}
