//! 本 crate 的错误类型。

/// Capability 层的错误。
///
/// **本 task 尚无变体，也无产生方**：本 task 的内容是词汇表，其上的操作
/// [`crate::CapabilityKind::for_effect`] 不可失败，故此处没有任何 `Err` 路径
/// 可写。变体由后续 task 的强制点定义——设计第 3.4 节的两个拒绝方向
/// （工具声明了而调用方没给、调用方给了而工具没声明）各是一种 `Err`，
/// 设计第 8 节的用例要求「断言是哪一种 `Err`」。
///
/// 先声明类型名，是为让下游 task 的接口在编译期即可命名它；变体随各自的
/// 产生方一起加，不在此处预造。
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {}
