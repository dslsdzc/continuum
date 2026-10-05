//! 本 crate 的错误类型。
//!
//! **没有一条消息会带出凭据材料**：可能提到材料的地方只有「主题是谁」（源名与作用域），
//! 内容一律不进消息。`SourceFormat` 的 `reason` 取 `&'static str` 而不是拼串，正是让
//! 「把出错的那一行原样带出去」在**类型上写不出来**。照片：`tests/issue.rs` 的
//! `no_error_message_embeds_the_material`，与 `tests/source.rs` 的
//! `a_malformed_entry_is_rejected_without_echoing_the_line`。

use continuum_capability::CapabilityError;

/// 密钥运行时的错误。
///
/// 每个变体都有一个**真实产生方**：
///
/// - [`Capability`](SecretsError::Capability)：`runtime.rs` 的 `issue` 在
///   `Capability::is_valid_at` 上转出（能力的时钟比较只有那一个产生点）；
/// - `ScopeNotCovered`：文件源与环境变量源对未列出的作用域；
/// - `SourceFormat`：文件源的解析（第三字段不是整数、字段数不对、作用域或材料为空、
///   作用域重复）；`line` 只在文件源里有意义，环境变量源不产出本变体；
/// - `SourceIo`：文件源的读（文件不存在、权限、非 UTF-8 内容）；
/// - `EmptyMaterial`：环境变量源取到空值——空材料不能当成一枚凭据发出去；
/// - `CredentialExpired`：`Credential::is_valid_at`；
/// - `Superseded`：`SecretsRuntime::material` 的代号比对。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretsError {
    /// 所据能力在签发时不可用（已失效，或作用域为空这类签发点错误）。
    ///
    /// **转出** [`CapabilityError`]，不另造一个「能力已过期」的变体：本项目里
    /// 「能力还有效吗」只有 [`continuum_capability::Capability::is_valid_at`] 一个
    /// 产生点，本 crate 复制一份比较（`now < cap.expiry()`）就会与它漂移。
    /// 照片：`tests/issue.rs` 的 `an_expired_capability_cannot_issue`（拒签那一侧
    /// 断言的是哪一种 `Err`，并带一个到期前一毫秒的对照臂）。
    #[error("能力不可用: {0}")]
    Capability(#[from] CapabilityError),

    /// 凭据源不覆盖该作用域：源里没有这个作用域的材料，故发不出凭据。
    ///
    /// 这是运行期**唯一**能让一次签发失败的作用域原因——「拿窄能力去要宽凭据」不是
    /// 走到这里被拒的，而是**要不到**（`issue` 的签名里没有指定另一个作用域的位置，
    /// 见 `tests/compile_fail/issue_has_no_scope_parameter.rs`）。
    /// 照片：`tests/issue.rs` 的 `a_scope_the_source_does_not_cover_is_rejected`、
    /// `tests/source.rs` 与 `tests/source_env.rs` 各一条。
    #[error("凭据源 {origin} 不覆盖作用域 {scope}")]
    ScopeNotCovered { origin: String, scope: String },

    /// 文件源的第 `line` 行不是合法条目。
    ///
    /// `reason` 是静态文本：出错的那一行**不许**被原样拼进消息（那会把材料带进日志）。
    #[error("凭据源 {origin} 的第 {line} 行不是合法条目: {reason}")]
    SourceFormat {
        origin: String,
        line: usize,
        reason: &'static str,
    },

    /// 文件源读不出来。
    #[error("凭据源 {origin} 读取失败: {message}")]
    SourceIo { origin: String, message: String },

    /// 环境变量存在但取值为空。
    ///
    /// 与文件源的空材料同样是拒签：空值不是一枚可用的凭据。
    /// 照片：`tests/source_env.rs` 的 `an_empty_variable_value_is_rejected`。
    #[error("凭据源 {origin} 的作用域 {scope} 取值为空")]
    EmptyMaterial { origin: String, scope: String },

    /// 凭据已过自己的到期时刻。
    ///
    /// 判的是**凭据自己的到期时刻**（可能因源声称更短而早于能力的），不是能力的到期
    /// 时刻——凭据不含能力（设计 §5.2），故也无从用那枚能力的判据。能力的有效性判定
    /// 在本 crate 里只有一处：`issue` 里对 `Capability::is_valid_at` 的调用。
    /// 产生点唯一：`Credential::is_valid_at`。照片：`tests/issue.rs` 的
    /// `the_material_is_readable_before_the_credential_expires_and_not_at_it`（到期
    /// 前一毫秒的对照臂 + `expiry` 与 `expiry + 1` 两格）。
    #[error("凭据已于 {expiry} 失效（Unix 毫秒），当前 {now}")]
    CredentialExpired { expiry: i64, now: i64 },

    /// 凭据已被轮换取代：签出它的那个代号不再是当前代号。
    ///
    /// `current` 是取代它的当前代号。消息只说代号，不带材料。
    /// 照片：`tests/issue.rs` 的 `every_rotation_event_class_invalidates_old_credentials`
    /// （§103 的四类事件各一行，逐项）。
    #[error("凭据已被取代（当前代号 {current}）")]
    Superseded { current: u64 },
}
