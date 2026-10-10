use continuum_persist::PersistError;

/// 本 crate 的错误类型。
///
/// # 零变体，不是占位
///
/// 本 task 的四个公开函数（[`crate::normalize`]、[`crate::detect`]、
/// [`crate::Opaque::as_str`] / [`crate::Opaque::from_str`]，
/// 以及 [`crate::MentionKind`] 的两个同名方法）**都是全函数**：任何输入都有输出，
/// 规范（`§270` §196 §197）也没有给本层任何一条「无法规范化」的判据
/// ——那条判据落在 §3.2 的 `Canonicalized` 和类型上，不在本 task。
///
/// 于是今天本层**没有失败路径**。把这件事写成类型的形态就是**零变体**：
/// 不可构造函数 ⇒ 不可能存在 `Err`。这是可表达的（与 `std::convert::Infallible` 同形），
/// 不是「先留个空壳、以后填」——一个带 `Unimplemented` 变体的枚举才叫空壳。
///
/// **本 task 未给它任何变体**（可读本文件自证）。第一个真的产生失败路径的 task
/// （例如 `alias` 表经 `Tx` 读写那一条，§4.6）在这里加变体，
/// 并**同时**给出该变体的照片。
///
/// **〔订正（2026-10-10，P4 Task 7）〕上面那句预告**（「第一个真的产生失败路径的 task
/// 在**这里**加变体」）**落空了一半**：那条失败路径确实到了（`alias` 表经 `Tx` 读写），
/// 但它的错误是**另一个类型** [`AliasError`]，不是 `CanonicalError` 的变体——
/// 计划「文件结构」一节逐字写的就是两个类型（`src/error.rs  CanonicalError、AliasError`，
/// `docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:282`），而 Task 7 的 Interfaces
/// 也把 `AliasError` 与 `CanonicalError` 并列产出。旧预告留在此处，免得下一轮照它
/// 去 `CanonicalError` 里找一个不存在的变体。
/// **`CanonicalError` 至此仍是零变体**——它的产生方要等一个本层自己判定的失败
/// （`canonical`／`ambiguity` 那两模块今天仍无失败路径），不是 `alias` 那条 I/O 失败。
#[derive(Debug, thiserror::Error)]
pub enum CanonicalError {}

/// Alias 存储的错误（§4.6、§278）。
///
/// 三个变体各有真实产生方，一条不漏：
///
/// - [`AliasError::UnknownScope`]：`alias` 表的 `scope` 列是**封闭枚举**（§278 的四值），
///   库里读到表外取值（历史行、被外部改过的行）时由 [`crate::load_alias`] /
///   [`crate::list_aliases`] 的解码处产出。照片：`tests/alias.rs` 的
///   `an_out_of_table_scope_reads_back_as_a_specific_err` 与
///   `list_aliases_rejects_an_out_of_table_scope_too`（**两条读路径各一条**——
///   它们共用一个解码函数是实现的选择，不是判据）。
/// - [`AliasError::NotFound`]：**提升一个不存在的 alias**。[`crate::promote_alias`]
///   是一条 `UPDATE`，而 SQLite 对匹配不到行的 `UPDATE` 返回 `Ok(0)`——
///   只看「`execute` 有没有报错」会把「拼错了名字」读成一次成功的提升。
///   照片：`tests/alias.rs` 的 `promoting_an_alias_that_does_not_exist_is_rejected`。
///   **`load_alias` 对不存在的 alias 返回 `Ok(None)` 而不是本变体**：那不是错误
///   （同 `load_tool`），只有「本该改到一行却一行都没改到」才是。
/// - [`AliasError::Persist`]：`Tx` 的两次往返（读、写）失败，`#[from]` 转出——
///   **不吞成上面两枚**：数据库层的失败若被降级成 `NotFound` / `UnknownScope`，
///   调用方会以为「数据就是这样」，而真相是这次压根没读到。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AliasError {
    /// `scope` 列是表外取值。`value` 是**库里那一个**原始取值，不是被映射过的任何东西。
    #[error("表外的 alias 作用域取值: {value}")]
    UnknownScope { value: String },

    /// 要提升的 alias 不在表里。
    #[error("alias 不存在: {alias}")]
    NotFound { alias: String },

    /// 读或写失败。具体是哪一次往返，由内层 [`PersistError`] 的消息给出。
    #[error("持久化失败: {0}")]
    Persist(#[from] PersistError),
}
