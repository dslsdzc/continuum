//! Task 4 的照片（判据 3 的条目级侧）：`seeded()` 的四份目录。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `seeded_builds_each_domain_catalog_with_exactly_the_named_methods` | 判据 3：四条目录的 id **逐名**等于 §187 的清单（`docs/spec/04-method.md:118-141`）＋ **逐 domain 条数**（字面量） | **移除档**（逐名侧）：`seeded()` 少 register 一条（如 `3d/render-comparison`）⇒ 逐名与该 domain 的条数两条断言都红；**加反例档**（逐名侧）：`seeded()` 多登记一条目录外的名 ⇒ 逐名断言红；**加反例档**（逐名侧）：把某条登记 id 写错（如 `"timeline-compose"` 写成 `"timeline_composer"`）⇒ 该 domain 的逐名断言红 |
//! | `seeded_entries_all_have_an_empty_realized_by` | 「`seeded()` 不预填任何绑定」 | **加反例档**：在 `seeded()` 里给某一条预填一个算子 id ⇒ 红 |
//!
//! # 两条断言为什么都写（逐名 ＋ 条数）
//!
//! **逐名相等蕴含条数相等（反之不成立）**，故条数断言是逐名断言的**弱化式**：
//! 逐名断言抓得到的它不一定抓得到，它抓得到的逐名断言全抓得到（例：同域内
//! 「少一名且多一名」净零变化——条数不动、逐名红）。两条都写的理由**不是**
//! 逻辑上互补，而是**设计 §10 判据 3 同时要求计数与逐名**
//! （`docs/superpowers/specs/2026-10-08-p5b-execution-method-library-design.md:567-568`）。
//!
//! **该计数不违纪律 3 的「自指计数不写数值」**：它数的是 §187 的清单（外部事实），
//! 不是本计划的用例数／task 数（自指）。
//!
//! # 射程
//!
//! - 十八条方法名的出处**逐字**是 §187 的四目录（`docs/spec/04-method.md:118-141`）。
//! - 逐名比较前对两边排序——元素次序不是本用例要钉的东西（`select` 的「按 id 升序」
//!   由 Task 3 的 `select_is_ordered_by_id_ascending` 钉）。
//! - 本文件**不**钉 `realized_by` 的内容（那是四块的 `bind` 与 `tests/roster.rs` 的事）；
//!   它只钉「目录里有什么（id 与归属）」与「此刻全空」。

use continuum_method::{MethodDomain, MethodRegistry};

#[test]
fn seeded_builds_each_domain_catalog_with_exactly_the_named_methods() {
    // 红条件：**漏一名即红、多一名即红**（逐名比较，不是计数比较）。
    // - **移除档**：seeded() 少 register 一条（如 `3d/render-comparison`）⇒ 逐名断言
    //   与该 domain 的条数断言**都红**；
    // - **加反例档**：seeded() 多登记一条目录外的名 ⇒ 逐名断言红（若同时在同域里
    //   少一名，条数断言会照过——这正是逐名比计数强的那一处）；
    // - **加反例档**：把某条登记 id 写错（如 "timeline-compose" 写成 "timeline_composer"）
    //   ⇒ 该 domain 的逐名断言红。
    let registry = MethodRegistry::seeded();

    // 四条目录**逐名**列在下面：每 domain 一块，块内次序照 §187 原文。
    let cases: [(MethodDomain, &[&str]); 4] = [
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
    for (domain, named) in cases {
        // 比较前对两边排序——逐名比较只要「名单相同」，不绑元素次序。
        let mut actual: Vec<&str> = registry
            .select(domain)
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        actual.sort();
        let mut expected = named.to_vec();
        expected.sort();
        assert_eq!(
            actual, expected,
            "{domain:?} 的目录必须逐名等于 §187 的清单（排序后比较；漏一名、多一名都红）"
        );
    }

    // 逐 domain 条数——**字面量**（不是从上面的清单推的）。**这是逐名断言的弱化式**：
    // 两条都写是因为设计 §10 判据 3 同时要求计数与逐名；**该计数不违纪律 3**：
    // 它数的是 §187 的清单（外部事实），不是本计划的用例数／task 数（自指）。
    assert_eq!(
        registry.select(MethodDomain::Software).len(),
        7,
        "software/ 恰 7 条（§187 清单的字面量）"
    );
    assert_eq!(
        registry.select(MethodDomain::Video).len(),
        4,
        "video/ 恰 4 条（§187 清单的字面量）"
    );
    assert_eq!(
        registry.select(MethodDomain::Research).len(),
        4,
        "research/ 恰 4 条（§187 清单的字面量）"
    );
    assert_eq!(
        registry.select(MethodDomain::ThreeD).len(),
        3,
        "3d/ 恰 3 条（§187 清单的字面量）"
    );
}

#[test]
fn seeded_entries_all_have_an_empty_realized_by() {
    // 作用：钉住「`seeded()` 不预填任何绑定」——四条目录的**每一条**（不抽代表）
    // 此刻 `realized_by` 为空。
    // 红条件（**加反例档**）：在 seeded() 里给某一条预填一个算子 id
    // （如 planning 预填 vec!["op-x"]）⇒ 本用例红。
    // **空的三个来源**（设计 §4.5）：域无对家（`3d/` 三条）、条目无对家
    // （`software/TDD`、`debugging`）、四块还没 bind（其余十三条）。本条只钉
    // 「此刻全空」这一件事；三者的区分由 `tests/roster.rs` 与 Task 2 的域谓词负责。
    let registry = MethodRegistry::seeded();
    for domain in MethodDomain::ALL {
        for entry in registry.select(domain) {
            assert!(
                entry.realized_by.is_empty(),
                "seeded() 不预填任何绑定：{domain:?}/{} 的 realized_by 必须为空",
                entry.id.as_str()
            );
        }
    }
}
