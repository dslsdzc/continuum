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
#[derive(Debug, thiserror::Error)]
pub enum CanonicalError {}
