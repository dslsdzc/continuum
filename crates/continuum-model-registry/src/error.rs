//! 画像侧取值类型的构造错误（设计 §2.4）。

/// 画像侧取值类型的构造错误。
///
/// **与 `LifecycleError` / `RoutingError` 分开**：它标的是「这个值根本不是合法取值」，
/// 不是「这次操作不合法」。
///
/// **它不进 `RoutingError`**：`rank` 收到的是构造好的值，构造失败在构造期就被拒
/// （设计 §2.4、§5.4）。
///
/// 设计 §2.4 给了三枚变体（`NotFinite` / `OutOfRange` / `BadTimeRange`），
/// 本 task 只定义前两枚——第三枚 `BadTimeRange { start, end }` 的产生方是
/// `SkillObservation` 的构造，在 Task 2 落地（**没有产生方的变体不先铺开**）。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileError {
    /// 不是有限实数（NaN 或 ±∞）。
    ///
    /// 两处产生方，判据同一条：`SkillScore` 的取值域在规范里**未定义**
    /// （§23 的示例是 9.2，§24 既未给范围也未给方向），故那边除了「必须是个有限实数」
    /// 之外无界可判；`Ratio` 的 `NaN` / `±∞` 也归这一枚——**不是有限实数**与
    /// **落在区间之外**是两回事，前者不是后者的一种。
    #[error("取值不是有限实数（NaN 或 ±∞）")]
    NotFinite,

    /// 有限、但落在 `[0,1]` 之外（`Ratio`）。
    ///
    /// `value` 原样带回入参，且**总是个有意义的数**：非有限的入参走
    /// [`ProfileError::NotFinite`]，不会到这里来。
    #[error("取值 {value} 落在 [0,1] 之外")]
    OutOfRange { value: f64 },
}
