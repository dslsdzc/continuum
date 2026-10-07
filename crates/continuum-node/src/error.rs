//! 本 crate 的错误类型。
//!
//! **本模块按类型分段落，不合并同族的错误**：[`RulesError`]（Task 4，
//! [`PlacementRules::try_new`](crate::PlacementRules::try_new) 拒绝一张表的原因）与
//! [`PlacementError`]（Task 5，[`place`](crate::place) 的失败路径）各写一遍。
//! 两者的派生口径相同（`Debug` ＋ `thiserror::Error`，**不派生 `PartialEq`**）但**不合并**：
//! 合并成一张大枚举会让「哪条路径能产生哪种错」在类型上消失——
//! `RulesError` 的产生点是**构造期**（表还进不了 `place`），`PlacementError` 的产生点是**运行期**，
//! 两者的操作数、消费方与判据面都不同。

use continuum_artifact::PrivacyClass;

use crate::node::ComputeNodeId;

/// [`PlacementRules::try_new`](crate::PlacementRules::try_new) 拒绝一张表的原因。
///
/// **只派生 `Debug` 与 `thiserror::Error`，不派生 `PartialEq`**——与 Task 3 的
/// `NodeRegistryError` 同口径：用例一律 `match` 取出 `level` 再断言是哪一档；
/// 派生 `PartialEq` 会把「两枚错误相等」变成一条**本层没有判据**的命题。
///
/// **`Debug` 是必需的、不是装饰**——但**理由不是 `unwrap_err()`**（那半句曾写在这里，
/// **是错的，订正于 2026-10-07 的 P3-E 终审修复轮，旧的错话留在此段末尾**）：
///
/// - **`unwrap_err()` / `expect_err()` 的界在 `T` 上，不在 `E` 上**
///   （`impl<T: Debug, E> Result<T, E>` 的 `unwrap_err`，实测 rustc 1.95.0：
///   `T` 缺 `Debug` 报 E0277「required by a bound in `Result::<T, E>::unwrap_err`」，
///   `E` 缺 `Debug` 一字不提）。这里 `T` 是 **`PlacementRules`**，
///   而它**一个 trait 都不派生**，故这两个方法**在本层根本写不出来**——
///   它们举不出 `Debug` 的必要性，缺的正是 `T` 那一侧。
/// - **真正要 `E: Debug` 的是另外两处**：`try_new(...).expect(…)`（`Result::expect`
///   的界在 `E` 上）与断言红时的 `{err:?}`；`thiserror::Error` 本身也要求
///   `Self: Debug + Display`。
///
/// **旧话照留（它错在哪）**：本段此前写「`unwrap_err()` / `expect_err()` 的签名要求 `E: Debug`」
/// ——把界记到了 `E` 上，与本 crate 的 `node.rs`（`ComputeNode` 的 `Debug` 一段，
/// 那里写的是 `T: Debug`）相抵；两处矛盾时**那一处是对的**。
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

/// [`place`](crate::place) 的失败路径。**只有它真的会产出的两枚**（设计 §5.8）。
///
/// **只派生 `Debug` 与 `thiserror::Error`，不派生 `PartialEq`**——与 [`RulesError`] 同口径：
/// 用例一律 `matches!`／`match` 取出载荷再断言。
///
/// **`Debug` 是必需的，但承重的那一侧是 `T`、不是 `E`**（这一句曾把两侧记反，
/// **订正于 2026-10-07 的 P3-E 终审修复轮，旧话照留在本段末尾**）：
/// **每一条失败路径的用例**都写 `let err = place(…).unwrap_err();`，
/// 而 `Result::unwrap_err` 的签名是 `impl<T: Debug, E> Result<T, E>` 上的方法
/// ——**界在 `T` 上**。这里 `T` 是 `&ComputeNode`，它的 `Debug` 由
/// [`ComputeNode`](crate::ComputeNode) 的派生提供（那也是 `ComputeNode` 派生 `Debug` 的理由，
/// 见 `node.rs` 的 `ComputeNode` 文档），**与本类型的 `E` 无关**
/// （实测 rustc 1.95.0：`E` 那一侧完全不派生 `Debug` 时，`unwrap_err()` 照样编得过）。
/// **本类型这一侧的 `Debug` 由另两处要**：断言红时的 `{err:?}`，
/// 以及 `thiserror::Error`（它要求 `Self: Debug + Display`）。
///
/// **旧话照留（它错在哪）**：本段此前先逐字引出 `impl<T: Debug, E> Result<T, E>`，
/// 紧接着却结论成「故 `E: Debug` 是**硬约束**」——**一句话内自相矛盾**：
/// 它自己引的那句签名就写着界在 `T` 上。
///
/// **三处「看起来该收但没有产生方」的不收，逐条**（设计 §5.8）：
///
/// - **不收「未归类的隐私等级」**：它在 [`PlacementRules::try_new`](crate::PlacementRules::try_new)
///   就被拒成 [`RulesError::MissingLevel`]，而
///   [`PlacementPolicy::rules`](crate::PlacementPolicy::rules) 交出的是一枚**已构造的**
///   [`PlacementRules`](crate::PlacementRules)，故「缺档」这条路径**到不了 `place`**。
/// - **不收 `Persist(...)`**：[`place`](crate::place) 是纯函数、签名里没有 `Tx`（设计 §5.2），
///   产不出读库失败。
/// - **不收「无候选模型」**：本设计的请求面里没有候选模型（§5.2 按裁定删去了那一格），
///   故这一类失败在本层**没有操作数**。
///
/// [`NoPlaceableNode`](PlacementError::NoPlaceableNode) **不区分「因为隐私被滤掉」与
/// 「本来就没节点」**：区分它需要把「哪几个节点因哪一条被滤掉」记成一个输出，
/// 而 §243／§291 **没有要求放置输出一个理由**（设计 §5.8）。
#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    /// 闸门过滤之后一个节点都不剩（**含 `nodes` 为空的情形**）。
    ///
    /// 两件事**刻意合并**成一枚：判据 §4.4 的照片因此必须写成**一对**
    /// （`a_local_only_artifact_lands_only_on_a_trusted_personal_node` 与
    /// `a_local_only_artifact_with_no_trusted_personal_node_is_unplaceable`）——
    /// 单看任一条分不清是两种失败里的哪一种。
    #[error("闸门过滤之后没有可放置的节点")]
    NoPlaceableNode,

    /// `nodes` 里同一个 [`ComputeNodeId`] 出现了两次。
    ///
    /// **它是「确定」这条断言的守门人**（设计 §5.9）：两条同 id 的节点无从定序，
    /// `ComputeNodeId` 兜底档也就兜不住。故判它的一步落在 [`place`](crate::place) 的**最前面**，
    /// 不是结尾的卫生检查。
    #[error("节点集里 id {id:?} 出现了两次；两条同 id 的节点无从定序")]
    DuplicateNode { id: ComputeNodeId },
}
