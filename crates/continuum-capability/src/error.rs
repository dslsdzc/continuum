//! 本 crate 的错误类型。

use continuum_core::tool::ToolId;
use continuum_persist::PersistError;

use crate::capability::CapabilityKind;

/// Capability 层的错误。
///
/// 每个变体都有一个**真实产生方**（Task 1 留下的空枚举等的就是这些线）：
///
/// - [`crate::mint`]：`EmptyScope`；
/// - [`crate::Capability::is_valid_at`]：`Expired`；
/// - [`crate::authorize`]：`UnknownTool` / `Persist` / `MissingCapability` /
///   `UndeclaredCapability`，并转出上面两条。
///
/// **不含「裁决为 `Deny`」这类变体**：那是**驱动**的判断（`continuum-runtime` 的
/// `mints` 六格表），本 crate 不复核调用方已经决定过的事——同一个判断有两个产生点，
/// 正是本项目一贯判为缺陷的那一类。设计第 3.4 节强制点 (1) 的两个拒绝方向自 Task 5
/// 起由 [`crate::authorize`] 产出，故在此落地：它们不是「裁决」的重判，而是**登记项与
/// 出示集之间的集合关系**（声明的没给、给的没声明）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    /// 作用域为空。
    ///
    /// §253 的 `scope` 是自由文本，而**空作用域的能力没有任何下游能判断它指的是
    /// 什么**：凭据的签发（设计 §5.2）、执行点与对账都要读这个字段，而它既不能被执行，
    /// 也不能被安全地忽略。这与 P2b 的「效应目标为空在解析期即拒」是同一条判据
    /// （`continuum-runtime/src/cli.rs:140-155` 的 `EffectWithEmptyTarget`），本处把该
    /// 判据落在签发点上——能力比 Journal 记录更早，故拒得更早。
    ///
    /// 判据是 `scope.is_empty()`，与那条先例逐字相同：**首尾空白不算空**（本处不做
    /// trim，也不做别的规范化——那些是「静默退化」，先例同样没做）。
    #[error("能力的作用域不能为空")]
    EmptyScope,

    /// 能力已失效。
    ///
    /// `expiry` 是**失效时刻**而非「最后有效的时刻」：`now == expiry` 即已失效。
    /// 照片：`tests/capability.rs` 的 `expired_at_and_after_the_expiry_instant`
    /// 逐格钉住 `expiry` 与 `expiry + 1` 两格。
    ///
    /// [`crate::authorize`] **转出**本变体而不另造一个：出示集中已声明却失效的那一枚
    /// 报的就是它，时钟比较仍只有 [`crate::Capability::is_valid_at`] 一个产生点。
    /// 照片：`tests/authorize.rs` 的 `an_expired_presented_capability_is_rejected`。
    #[error("能力已于 {expiry} 失效（Unix 毫秒），当前 {now}")]
    Expired { expiry: i64, now: i64 },

    /// 要授权的工具不在登记表里。
    ///
    /// `load_tool` 对不存在的 id 返回 `Ok(None)`（那是**正常**结果，不是错误），
    /// 而 [`crate::authorize`] 不能把「工具没找到」静默当成「这个工具不需要任何能力」
    /// ——那会让一次拼错 id 的授权变成一次无校验的放行。故本变体在此落地。
    /// 照片：`tests/authorize.rs` 的 `an_unknown_tool_is_rejected`。
    #[error("工具 {} 不在登记表中", id.as_str())]
    UnknownTool { id: ToolId },

    /// [`crate::authorize`] 的两次数据库往返之一失败。**两个产生方**：
    ///
    /// - **[`load_tool`](crate::load_tool) 读登记项**（列里是表外取值、列类型不符等）：
    ///   不吞成 [`CapabilityError::UnknownTool`]、也不吞成「无需能力」——数据库层的失败
    ///   若被静默降级，`authorize` 的签名仍会返回 `Ok`，而调用方拿到的是一次**没查过
    ///   声明表**的授权。照片：`tests/authorize.rs` 的
    ///   `a_persist_failure_while_loading_the_tool_is_not_swallowed`；
    /// - **`Tx::append_audit` 写审计**（表缺失等）：同样不许吞——吞掉它会让这次授权
    ///   有授权之实而无审计之据。照片：同文件的
    ///   `an_audit_write_failure_is_reported_as_persist`。
    ///
    /// **消息刻意只说「持久化失败」**：本变体由两个产生方共用一个 `#[from]`，若消息写成
    /// 「读取…失败」就会在写审计那条路径上告诉操作者一件假事。具体是哪一次往返失败，
    /// 由内层 [`PersistError`] 的消息给出（它带表名/取值）。
    #[error("持久化失败: {0}")]
    Persist(#[from] PersistError),

    /// 工具声明了这枚能力，调用方没有出示（§252：`Tool` MUST NOT 接收未声明的
    /// capability；反过来，缺了声明的那一枚也不许调用）。
    ///
    /// 多枚同时缺时报的是**声明表里最靠前的那一枚**（[`crate::authorize`] 的次序）。
    /// 照片：`tests/authorize.rs` 的 `a_required_capability_that_is_missing_is_rejected`。
    #[error("工具声明了能力 {}，调用方未出示", kind.as_str())]
    MissingCapability { kind: CapabilityKind },

    /// 调用方出示了工具没有声明的能力：**超范围同样不许**（设计 §3.4 的另一半）。
    /// 照片：`tests/authorize.rs` 的
    /// `a_presented_capability_that_the_tool_did_not_declare_is_rejected`。
    #[error("调用方出示了工具未声明的能力 {}", kind.as_str())]
    UndeclaredCapability { kind: CapabilityKind },
}
