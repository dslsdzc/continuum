//! 本块的错误类型（设计 §3.2）。

use continuum_operator::{BackendId, OperatorError, OperatorId};

/// 本块的错误类型（设计 §3.2）。
///
/// **两臂，不是三臂**：P5e 的 `MediaError` 有第三臂 `NotPermitted`（§34 的生成授权），
/// 而代码域**没有对应的授权面**——§34 管的是「生成内容替换原始素材」，本块九枚算子
/// 没有任何一枚产「生成内容」（它们产 `Plan` / `Patch` / `TestResult`）。
/// **不为对称补一臂：一条永远进不去的臂与一个死字段同类**（设计 §3.2 末段）。
#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    /// §3.5 的注册期规则不成立：声明 `Deterministic` 的算子混进了名单外的 backend。
    /// 判定的持有者在本块（§3.5）。
    ///
    /// **格式化参数取 `as_str()` 而不直接用 `{backend}`**：`BackendId` 今天没有实现
    /// `Display`（`crates/continuum-operator/src/definition.rs`），直接写 `{backend}`
    /// 编译不过（`E0277`）。显式取 `as_str()` 使渲染文案与设计 §3.2 的格式串逐字一致
    /// （不用 `{backend:?}`——那会渲染成 `BackendId("…")`）。**这是计划代码块与
    /// 源码的一处不符，已在报告中回报**；P5e 的分支给 `BackendId` 补了 `Display`
    /// （其提交 `26b9fb4`），合入后本处可简化，本块不预改。
    #[error("算子 {operator} 的候选 backend {backend} 不在逐位可复现名单内（§3.5）", backend = backend.as_str())]
    UnverifiableDeterminism {
        operator: OperatorId,
        backend: BackendId,
    },

    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1。
    /// **这一臂是包装不是同名臂**：撞车的判定已经有一枚臂（`OperatorError::Duplicate`，
    /// `crates/continuum-operator/src/registry.rs:11`），本块再立一枚
    /// `CodeError::Duplicate` 就是「注册表里有没有这个算子」的第二套词汇表
    /// （设计 §3.2）。
    #[error(transparent)]
    Registry(#[from] OperatorError),
}
