//! §341 的用户审查视图（设计 §9.5）。
//!
//! 计划「文件结构」一节把 `UserReviewView` 归在 `src/review_view.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:310`：「`UserReviewView`
//! （§341 的八项投影）」）。本文件只有这一枚类型：**它不变量、不判定、不投影**——
//! 投影的**产生方**（谁把 Plan 与其它输入填进这八项）不在本层，本文件只给形状。
//!
//! # 八项逐行照 §341，不合并任何两行
//!
//! §341（`docs/spec/05-normative.md:2661-2678`）的清单**八行**，逐行照录：
//! 目标 / 重大设计 / 关键约束 / Milestone / 风险 / 成本 / 权限 / 验收条件。
//! **`成本` 与 `权限` 是分列的两行**，不是一行。
//!
//! > **这一条的来历（设计 §9.5 的订正段）**：设计初稿把这两行合并成一个字段
//! > `cost_and_authority`，并写「七项（原文如此，第七项与第八项合并计为七类）」——
//! > **那句话是假的**：§341 写的是八行；**出错的恰是「少一项」那一侧**。
//! > 初稿的守卫只写「多一项即红」，守不住它自己的错；§14 的同一条守卫（三侧写全：
//! > 多／少／合并）是修后复核查出的。本文件按八项分列落，两侧守卫的照片见
//! > `tests/plan.rs` 的同名条目。
//!
//! # 字段类型取统一的 [`Opaque`]（申报）
//!
//! §341 只给八行的名字，**未给任何一行的结构**（哪些是列表、哪些是单值都没有判据）。
//! 故八项一律取 [`Opaque`]：本层保存事实、不发明结构——若把「目标」建成
//! `Vec<Opaque>` 而「成本」建成 `Opaque`，那个「哪些是列表」的分法就是本层的发明
//! （§341 没有依据）。将来某一行先有判据时，单独加宽它是一个小改。
//!
//! # 本组件不是 UI
//!
//! §2.1 的组件表里没有「用户界面」组件——本层只产出这个视图的**形状**，
//! 它的呈现方不在本层。「八项里任何两项都不得合并」说的是**事实**（用户看得到什么），
//! 不是排版：若将来判定「成本与权限在界面上应并排显示」，那是呈现方的事，不是这个
//! 投影类型的事——本设计不为它留字段、也不做这个合并（§9.5 末段）。

use continuum_canonical::Opaque;

/// §341 的用户审查视图。**字段恰好八项**，与 §341 的八行一一对应，**不合并任何两行**。
///
/// - 八项的字段名：`objectives` / `major_designs` / `key_constraints` / `milestones` /
///   `risks` / `cost` / `authority` / `acceptance_conditions`；
/// - 字段清单**逐项**的照片（**三侧负向守卫**：多一项即红、少一项即红、`成本` 与
///   `权限` 合并成一项即红）在 `tests/plan.rs` 的两条：
///   `the_user_review_view_has_exactly_the_eight_lines_of_341`（编译期：穷尽解构、
///   **不带 `..`**）与 `the_user_review_view_has_no_field_pointing_at_internal_steps`
///   （运行期：源码面字段集合对钉——**八项里没有任何一项承载 Plan 的内部执行步骤**，
///   §341 的清单是这个视图的**全部**判据）。
///
/// 字段是 `pub` 的（同 [`crate::Plan`]：本类型是 §341 逐字给出的清单，
/// 本层没有更强的约束可挂）。
#[derive(Debug, Clone)]
pub struct UserReviewView {
    /// §341 的「目标」。
    pub objectives: Opaque,
    /// §341 的「重大设计」。
    pub major_designs: Opaque,
    /// §341 的「关键约束」。
    pub key_constraints: Opaque,
    /// §341 的「Milestone」。
    pub milestones: Opaque,
    /// §341 的「风险」。
    pub risks: Opaque,
    /// §341 的「成本」。**与 `权限` 分列**（见模块文档的来历段）。
    pub cost: Opaque,
    /// §341 的「权限」。**与 `成本` 分列**。
    pub authority: Opaque,
    /// §341 的「验收条件」。
    pub acceptance_conditions: Opaque,
}
