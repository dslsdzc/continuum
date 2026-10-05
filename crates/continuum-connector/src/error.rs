//! 连接器边界上的错误类型（设计 §6.1）。
//!
//! **没有产生方的变体不建**：本 task 只建注册期双向覆盖的那两条，它们各自在
//! [`crate::ConnectorRegistry::register`] 里有真实产生方。其余八条由后续 task
//! 在**用到它的那个 task**里增量加——`DuplicateKindBinding` / `OperationServiceMismatch`
//! 在 Task 3，`UnknownConnector` / `UndeclaredOperation` / `AuthorizationMismatch` 在
//! Task 4，`EffectAuthorizationRequired` 在 Task 5，转出型的 `Credentials` / `Provider`
//! 在 Task 4（设计 §6.1 的表共十行，本 task 落地其中两行）。

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
}
