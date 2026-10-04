//! 本 crate 的错误类型。

/// Capability 层的错误。
///
/// 两个变体各有一个**真实产生方**（Task 1 留下的空枚举等的就是这两条线）：
///
/// - [`crate::mint`]：`EmptyScope`；
/// - [`crate::Capability::is_valid_at`]：`Expired`。
///
/// **不含「裁决为 `Deny`」这类变体**：那是**驱动**的判断（`continuum-runtime` 的
/// `mints` 六格表），本 crate 不复核调用方已经决定过的事——同一个判断有两个产生点，
/// 正是本项目一贯判为缺陷的那一类。同一条理由，设计第 3.4 节强制点 (1) 的两个拒绝
/// 方向也留到计划 Task 5 的 `authorize`——那时它们才有产生方。
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
    #[error("能力已于 {expiry} 失效（Unix 毫秒），当前 {now}")]
    Expired { expiry: i64, now: i64 },
}
