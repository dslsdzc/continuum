//! 本 crate 的错误类型。计划「文件结构」一节把 `IntentError、SpecError、ContractError、
//! PlanError、DecisionError、ConstraintError` 都列在 `src/error.rs`；本 task 只落第一个。

use continuum_persist::PersistError;

/// Intent 落库与读回的错误。
///
/// # 三枚各有真实产生方，一条不漏
///
/// - [`IntentError::UnknownKind`]：`intent` 表的 `kind` 列是**封闭六值**
///   （[`crate::IntentKind`]），库里读到表外取值（历史行、被外部改过的行）时由
///   [`crate::load_intent`] / [`crate::list_children`] 的解码处产出，
///   **带库里那一个原始串**。
/// - [`IntentError::UnknownStatus`]：同上，`status` 列是闭集十三枚
///   （[`crate::IntentStatus`]）。
/// - [`IntentError::Persist`]：`Tx` 的往返失败，`#[from]` 转出——**不吞成上面两枚**：
///   数据库层的失败若被降级成 `UnknownKind` / `UnknownStatus`，调用方会以为
///   「数据就是这样」，而真相是这次压根没读到（同 `continuum_canonical::AliasError`
///   的 `Persist` 那条理由）。
///
/// **两枚「表外取值」是分开的，不合并成一枚 `UnknownValue { column, value }`**：
/// 合并之后「kind 表外」与「status 表外」在类型上不再可分辨，而两者的处置方向可能不同
/// （一个要扩枚举、一个要修这一行）。分开是**一字之省**的代价换来可分辨的裁决。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntentError {
    /// `kind` 列是表外取值。`value` 是**库里那一个**原始取值，不是被映射过的任何东西。
    #[error("表外的 intent kind 取值: {value}")]
    UnknownKind { value: String },

    /// `status` 列是表外取值。`value` 同上。
    #[error("表外的 intent status 取值: {value}")]
    UnknownStatus { value: String },

    /// `Tx` 的往返失败。
    #[error(transparent)]
    Persist(#[from] PersistError),
}
