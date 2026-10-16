//! `seeded()` 与 `UNREALIZED_BY_DESIGN`（设计 §4.5、§5、§7）。
//!
//! - [`MethodRegistry::seeded`]：§187 的四份目录**逐名**照录，四条目录共 18 条；
//!   `realized_by` 一律不预填（三个「空」的来源见函数注释）。
//! - [`UNREALIZED_BY_DESIGN`]：判据 6 的**条目级例外集**——「按设计留空」的两条
//!   （`software/TDD` 与 `software/debugging`）。

use crate::{MethodDomain, MethodEntry, MethodId, MethodRegistry};

impl MethodRegistry {
    /// §187 的四份目录，逐名照录（设计 §5；`docs/spec/04-method.md:118-141`）——
    /// 四条目录共 18 条。
    ///
    /// `realized_by` 初始为空：`3d/` 的三条**刻意**留空（域无对家，设计 §7 第 2 条）、
    /// `software/TDD` 与 `debugging` 两条**刻意**留空（条目无对家，设计 §4.5、判据 6，
    /// 由 [`UNREALIZED_BY_DESIGN`] 承载），其余 13 条待各自的算子块经 `bind` 填入。
    ///
    /// # 十八次 `register` 的返回值一律 `expect`
    ///
    /// 十八个 id 互不相同（§187 的清单无重名），撞名即代码写错。于是
    /// [`MethodError::Duplicate`] 在**生产路径上只在这条路径上可达**——`register`
    /// 在本 crate 的唯一生产调用者就是本函数（四个算子块被禁止调它，设计 §4.5）；
    /// 同一句话另两处在 `tests/registry.rs` 的 `register_rejects_…` 与 Task 5 Step 4
    /// 的判据 4 一行。
    ///
    /// [`MethodError::Duplicate`]: crate::MethodError::Duplicate
    pub fn seeded() -> Self {
        let mut registry = MethodRegistry::new();

        // §187 的四份目录**逐名**照录（`docs/spec/04-method.md:118-141`）：
        // 每 domain 一块、块内次序照原文；条目一律 `realized_by: Vec::new()`。
        let catalog: [(MethodDomain, &[&str]); 4] = [
            (
                MethodDomain::Software,
                &[
                    "planning",
                    "TDD",
                    "debugging",
                    "worktree",
                    "review",
                    "fuzz",
                    "verification",
                ],
            ),
            (
                MethodDomain::Video,
                &[
                    "shot-analysis",
                    "narrative-plan",
                    "timeline-compose",
                    "render-review",
                ],
            ),
            (
                MethodDomain::Research,
                &[
                    "retrieval",
                    "evidence-analysis",
                    "contradiction-check",
                    "citation-verification",
                ],
            ),
            (
                MethodDomain::ThreeD,
                &["reconstruction", "geometry-validation", "render-comparison"],
            ),
        ];
        for (domain, ids) in catalog {
            for id in ids {
                registry
                    .register(MethodEntry {
                        id: MethodId::new(*id),
                        domain,
                        realized_by: Vec::new(),
                    })
                    .expect("§187 的十八个 id 互不相同（清单无重名），撞名即代码写错");
            }
        }

        registry
    }
}

/// 「按设计留空」的条目，形状是 `(领域名, 方法 id)`：领域名与
/// [`MethodDomain::as_str`] 的取值对齐。
///
/// 今日**恰为**两条：§187 目录里的 `software/TDD` 与 `software/debugging`——
/// 两条方法名不是算子、本轮无算子对家（报出方是 P5c 设计 §7.2，协调者
/// 2026-10-10 裁定，理由写在本设计的第十节判据 6：`TDD` 的全部内容是**次序**、
/// `Vec<String>` 表达不了；`debugging` 在链上没有对应步骤）。
///
/// **这张表是判据 6 的例外集，是判据的输入，不是「注释里的一句豁免」**——
/// 域级有 [`MethodDomain::has_counterpart`] 兜住 `ThreeD`，条目级靠本表把
/// 「刻意留空」与「漏 bind」分开。表必须**恰为**这两条：多一条、少一条、
/// 或写错一个名，`tests/roster.rs` 的用例都红（**「多一条」「少一条」这两处
/// 变异各含类型长度**：类型里的 `;2` 与字面量都要改，否则是 `E0308` 编译失败
/// 而非红——纪律 1(c)）。
pub const UNREALIZED_BY_DESIGN: [(&str, &str); 2] =
    [("software", "TDD"), ("software", "debugging")];
