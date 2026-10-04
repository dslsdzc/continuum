//! 本 crate 的错误类型。

/// Capability 层的错误。
///
/// 变体只列**有产生方**的失败路径，随各自的产生方一起加（本 crate 对「声明了没有
/// 消费方的东西」一贯要么删、要么写明理由）：
///
/// - [`crate::mint`] 的两个拒绝原因：`PolicyDenied` 与 `ApprovalRequired`；
/// - [`crate::Capability::is_valid_at`]：`Expired`。
///
/// **尚无产生方**的是设计第 3.4 节强制点 (1) 的两个拒绝方向（工具声明了而调用方
/// 没给、调用方给了而工具没声明）：产生它们的是 Task 4 的
/// `ToolRegistry::authorize`，变体到那时再加。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    /// 签发被策略禁止。
    ///
    /// 显式确认（§5.5 的 `--approve`）**越不过它**：本仓既有的裁决接法在 `Deny`
    /// 上无论确认给没给都不铸造（`continuum-runtime` 的 `mints`，照片是那里的
    /// `the_mapping_from_a_decision_to_minting_has_six_cells` 与
    /// `a_system_safety_deny_is_not_overridden_by_the_flag`）。
    #[error("策略禁止这一项能力，不予签发")]
    PolicyDenied,

    /// 策略裁决为「要求批准」，而本次调用未带显式确认。
    #[error("策略要求显式确认，而本次未给出")]
    ApprovalRequired,

    /// 能力已失效。
    ///
    /// `expiry` 是**失效时刻**而非「最后有效的时刻」：`now == expiry` 即已失效。
    /// 照片：`tests/capability.rs` 的 `expired_at_and_after_the_expiry_instant`
    /// 逐格钉住 `expiry` 与 `expiry + 1` 两格。
    #[error("能力已于 {expiry} 失效（Unix 毫秒），当前 {now}")]
    Expired { expiry: i64, now: i64 },
}
