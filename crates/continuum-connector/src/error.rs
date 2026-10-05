//! 连接器边界上的错误类型（设计 §6.1）。
//!
//! **没有产生方的变体不建**：已建的四条注册期变体各自在
//! [`crate::ConnectorRegistry::register`] 里有真实产生方。其余六条由后续 task
//! 在**用到它的那个 task**里增量加——`UnknownConnector` / `UndeclaredOperation` /
//! `AuthorizationMismatch` 在 Task 4，`EffectAuthorizationRequired` 在 Task 5，
//! 转出型的 `Credentials` / `Provider` 在 Task 4（设计 §6.1 的表共十行，
//! 截至本 task 落地其中四行）。

use continuum_capability::CapabilityKind;
use continuum_core::connector::{ConnectorId, ConnectorOp};

/// 连接器注册与调用边界上的错误。
///
/// 注册期的核对都以**变体**区分失败种类，而不是一句笼统的话——调用方要能判出
/// 是「声明了没绑」还是「绑了没声明」，这两侧互为反面（设计 §6.1 的取材：
/// 只建一侧不算钉住）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectorError {
    /// 声明的操作里有一条没有对应的绑定——绑定不完备。
    #[error("连接器 {} 声明的操作 {} 没有绑定", connector.as_str(), op.as_str())]
    UnboundOperation {
        connector: ConnectorId,
        op: ConnectorOp,
    },
    /// 绑定里有一条对应的操作不在声明集内——双向覆盖的另一侧。
    #[error(
        "连接器 {} 绑定了未声明的操作 {}",
        connector.as_str(),
        op.as_str()
    )]
    UndeclaredBoundOperation {
        connector: ConnectorId,
        op: ConnectorOp,
    },
    /// 同一个连接器内，一枚 `CapabilityKind` 被两条操作绑定——「一一」被破坏
    /// （设计 §3.2 第 3 条）。
    ///
    /// 理由（设计 §3.3）：绑定的粒度存在，正是为了让 §125 的每条操作**能单独不授**
    /// （「`GitHub.merge` 能单独不授」）。两条操作绑同一枚 kind，则授权其一即授权另一，
    /// 粒度退回服务级——那正是 §125 禁止的形态。
    ///
    /// 照片：`tests/register.rs` 的 `two_operations_bound_to_the_same_kind_are_rejected`。
    #[error("连接器 {} 的两条操作绑定了同一枚能力 {}", connector.as_str(), kind.as_str())]
    DuplicateKindBinding {
        connector: ConnectorId,
        kind: CapabilityKind,
    },
    /// 操作串的服务半边不等于本连接器的 id（设计 §3.2 第 5 条）。
    ///
    /// `ConnectorOp` 只要求串里有一个 `.`，不保证操作属于哪个服务。服务半边对不上的
    /// 操作**永不可达**——入口第 1 步按服务半边解析连接器，永远解析不到它——故这条
    /// 核对把它挡在注册期。
    ///
    /// 照片：`tests/register.rs` 的
    /// `an_operation_whose_service_half_is_not_this_connector_is_rejected` 与
    /// `the_service_half_is_compared_case_sensitively`（后者钉「逐字比较、不折叠大小写」）。
    #[error(
        "连接器 {} 不能声明服务半边不是自己的操作 {}",
        connector.as_str(),
        op.as_str()
    )]
    OperationServiceMismatch {
        connector: ConnectorId,
        op: ConnectorOp,
    },
}
