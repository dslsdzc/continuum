//! `MethodError`：方法库的两条失败路径（设计 §4.4）。

use crate::MethodId;

/// 方法库的失败：**恰好两枚**变体（设计 §4.4）。
///
/// 与 `OperatorError`（`crates/continuum-operator/src/registry.rs:7-12`）同形，
/// 只两枚变体、载荷只有 [`MethodId`] 一项。**不新增第三个错误类型**
/// （设计 §4.4：切分文档 §二第 4 条对「第二份错误类型」的禁令本处同理）。
///
/// # 两条路径与格式串
///
/// - [`MethodError::Duplicate`]：登记时撞同一 `MethodId`（`register` 的判重侧）；
/// - [`MethodError::NotFound`]：`resolve` 未注册、`bind` 对未注册 id。
///
/// 格式串以 `{id}` 引用 [`MethodId`]——故 [`MethodId`] 须有 `Display`
/// （理由同 `crates/continuum-operator/src/definition.rs:36-37` 那段注释）；
/// `tests/identity.rs` 有照片（`method_error_renders_its_id_in_the_message`）。
/// 变体数本身的照片是同一文件里那道不带 `..` 的两臂穷尽解构（编译期）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MethodError {
    /// 要解析／绑定的方法不存在。
    #[error("方法 {id} 未注册")]
    NotFound { id: MethodId },
    /// 同一 `MethodId` 已登记。
    #[error("方法 {id} 已注册")]
    Duplicate { id: MethodId },
}
