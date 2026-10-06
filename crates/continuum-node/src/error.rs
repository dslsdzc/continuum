//! 本 crate 的错误类型。
//!
//! **本模块按类型分段落，不合并同族的错误**：本文件今天只有
//! [`RulesError`]（Task 4），放置本身的错误（`PlacementError`）由 Task 5 追加入本模块。
//! 两者的派生口径相同（`Debug` ＋ `thiserror::Error`，**不派生 `PartialEq`**）但各写一遍
//! ——合并成一张大枚举会让「哪条路径能产生哪种错」在类型上消失，
//! 而 `place` 的判据面（设计 §5.4 通道 (b) 之后）只应看到 `RulesError`。

use continuum_artifact::PrivacyClass;

/// [`PlacementRules::try_new`](crate::PlacementRules::try_new) 拒绝一张表的原因。
///
/// **只派生 `Debug` 与 `thiserror::Error`，不派生 `PartialEq`**——与 Task 3 的
/// `NodeRegistryError` 同口径：用例一律 `match` 取出 `level` 再断言是哪一档；
/// 派生 `PartialEq` 会把「两枚错误相等」变成一条**本层没有判据**的命题。
///
/// **`Debug` 是必需的、不是装饰**：`try_new` 返回 `Result<Self, RulesError>`，
/// 而 `unwrap_err()` / `expect_err()` 的签名要求 `E: Debug`；它也是断言红时
/// 能读出实际值的前提。
#[derive(Debug, thiserror::Error)]
pub enum RulesError {
    /// 表里没有 `level` 这一档。
    ///
    /// 这是 §5.4 通道 (b) 的**主要**产物：一张不覆盖全档的表正是「未归类」的形状，
    /// 而 fail-closed 要的恰恰是「必须逐档表态」（设计 §5.4 末段，
    /// 那里记着否掉「只收其余四档」这一替代方案的理由）。
    #[error("放置表缺 {level:?} 这一档；必须覆盖 PrivacyClass::ALL 的每一档")]
    MissingLevel { level: PrivacyClass },

    /// 表里 `level` 出现了不止一次。
    ///
    /// **它与 [`RulesError::MissingLevel`] 的分工**：重复一条**不违反覆盖率**
    /// （六条可以覆盖齐五档），故只钉覆盖率抓不到它；
    /// 反过来，只钉重复也抓不到缺档。两条判据都要有。
    ///
    /// [`PrivacyClass`] 只有五枚，故键的取值域只有五个，「多一条」没有别的表现形式：
    /// **一张条数超过五的表必然含重复**，由本变体拒。于是
    /// 「条数不对」被两条判据穷尽：多则由本变体拒，少则由
    /// [`RulesError::MissingLevel`] 拒——**没有第三种**。
    #[error("放置表里 {level:?} 重复出现；每一档只允许一条")]
    DuplicateLevel { level: PrivacyClass },
}
