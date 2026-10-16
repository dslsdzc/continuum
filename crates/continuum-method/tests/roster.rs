//! Task 4 的照片（判据 6 的条目级侧）：`UNREALIZED_BY_DESIGN` 恰为两条、两侧都钉。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `the_unrealized_roster_is_exactly_the_two_named_entries` | 名册逐项等于字面 `[("software", "TDD"), ("software", "debugging")]`（设计 §10 判据 6 末） | **移除档**：少一条 ⇒ 红；**加反例档**：多一条 ⇒ 红；**加反例档**：改一个名或一个域 ⇒ 红。三条**各写一处**（见用例内注释）；「少一条」「多一条」两条含类型长度 |
//! | `every_roster_name_exists_in_the_seeded_catalog_and_is_empty_today` | 名字没写错（`parse` 给 `Some`、目录里含该 id）＋「此刻确为空」 | **取反档**：把 `"TDD"` 写成 `"tdd"` ⇒ 「目录里含」那条红；**加反例档**：`seeded()` 预先把 `software/TDD` bind 上 ⇒ 「此刻确为空」那条红 |
//! | `three_d_entries_are_empty_because_the_domain_has_no_counterpart` | 设计 §10 判据 6 第 1 条：`has_counterpart(ThreeD) == false` ＋ `select(ThreeD)` 逐条为空 ＋ 其余三个域为真 | **取反档**：`has_counterpart(ThreeD)` 给 `true` ⇒ 第一半红；**加反例档**：`seeded()` 给 `3d/` 的任一条预填绑定 ⇒ 第二半红 |
//! | `no_counterpart_entry_outside_the_roster_stays_empty` | 判据 6 第 2 条的**可闭形态**：非名册条目全绑上后，仍空的**恰等于**字面名册（排序后逐项） | **加反例档**（a）：名册多一条 ⇒ 该条被跳过不绑 ⇒ 空集多一项 ⇒ 红；**移除档**（b）：名册少一条 ⇒ 另一条被绑上 ⇒ 空集少一项 ⇒ 红；**加反例档**（c）：名册写错名（`"TDD"` → `"tdd"`）⇒ 真 `TDD` 被绑上、假名不在目录里 ⇒ 空集里没有 `"TDD"` ⇒ 红。（a）（b）各含类型长度（见用例内注释） |
//!
//! # 射程与归属
//!
//! - **`no_counterpart_entry_outside_the_roster_stays_empty` 不覆盖「生产里某块漏 bind」**——
//!   它在用例内部自己把非名册条目全绑上了。那一侧 `continuum-method` 看不到
//!   （四块在别的 crate、在其后运行，设计 §4.5 末），交给 P5c／P5d／P5e 各自的计划
//!   并由整合作收口（实现计划 `## 交付给谁`）。
//! - 名册的**领域名**与 `MethodDomain::as_str` 的取值对齐——由
//!   `every_roster_name_exists_in_the_seeded_catalog_and_is_empty_today` 的
//!   `parse` 断言与实现里的字面值两侧钉住。

use continuum_method::{MethodDomain, MethodId, MethodRegistry, UNREALIZED_BY_DESIGN};

#[test]
fn the_unrealized_roster_is_exactly_the_two_named_entries() {
    // 期望值**写成字面**（下），**不从该常量自身取**——否则断言恒真，
    // 是自证的正控制（设计 §10 判据 6 末）。
    let expected = [("software", "TDD"), ("software", "debugging")];

    // 红条件**三条各写一处**：
    // - **移除档**（少一条）：表只剩一项 ⇒ 红。**变异含类型长度**：常量类型
    //   `[(&str, &str); 2]` 的数组长度是类型的一部分——只把字面量改成一项而不改
    //   类型里的 `;2`，得到 `error[E0308]`（expected an array with a fixed size
    //   of 2 elements），按纪律 1(c) **那是编译失败，不是红**。
    //   完整变异体 = 类型改 `;1` **且**字面量改一项。
    // - **加反例档**（多一条）：表变三项 ⇒ 红。**同样含类型长度**：类型改 `;3`
    //   **且**字面量改三项；只改一处是 `E0308` 编译失败（同上）。
    // - **加反例档**（改一个名或一个域，如 `"TDD"` → `"tdd"`、或
    //   `("software", "TDD")` → `("video", "TDD")`）：长度不变、照旧编译得过，
    //   逐项比较即红。
    assert_eq!(
        UNREALIZED_BY_DESIGN.len(),
        expected.len(),
        "名册恰为两条——少一条（1）、多一条（3）都红"
    );
    for (index, (got, want)) in UNREALIZED_BY_DESIGN.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, want,
            "名册第 {index} 项必须逐字等于字面值（改一个名或一个域即红）"
        );
    }
}

#[test]
fn every_roster_name_exists_in_the_seeded_catalog_and_is_empty_today() {
    let registry = MethodRegistry::seeded();

    for (domain, id) in UNREALIZED_BY_DESIGN {
        // 第一断言：领域名可解析（名册的领域名与 MethodDomain::as_str 的取值对齐）。
        let parsed = MethodDomain::parse(domain).unwrap_or_else(|| {
            panic!(
                "名册里的领域名 {domain:?} 必须与 MethodDomain::as_str 的取值对齐（parse 给 Some）"
            )
        });

        // 第二断言：目录里**含**该 id（故名没写错）。
        // 红条件（**取反档**）：把表里的 "TDD" 写成 "tdd" ⇒ 本断言红（目录里没有这个名字）。
        let listed = registry
            .select(parsed)
            .iter()
            .any(|entry| entry.id.as_str() == id);
        assert!(
            listed,
            "名册里的 {domain}/{id} 必须出现在 seeded() 的目录里（名字没写错）"
        );

        // 第三断言：该条此刻确为空。
        // 红条件（**加反例档**）：让 seeded() 预先把 software/TDD bind 上 ⇒ 本断言红。
        let entry = registry
            .resolve(&MethodId::new(id))
            .expect("名册里的条目必须已登记（第二断言已钉住）");
        assert!(
            entry.realized_by.is_empty(),
            "名册里的 {domain}/{id} 此刻必须确为空——名册是「按设计留空」的例外集"
        );
    }
}

#[test]
fn three_d_entries_are_empty_because_the_domain_has_no_counterpart() {
    let registry = MethodRegistry::seeded();

    // 第一半：域级谓词——3d/ 本轮无对家。
    // 红条件（**取反档**）：has_counterpart(ThreeD) 给 true ⇒ 本断言红。
    assert!(
        !MethodDomain::ThreeD.has_counterpart(),
        "3d/ 本轮无对家（设计 §7 第 2 条；R2 被重新裁决时按裁决改）"
    );

    // 第二半：该域**逐条**为空——「空是刻意的，不是漏填」。
    // 红条件（**加反例档**）：seeded() 给 3d/ 的任一条预填绑定 ⇒ 本断言红。
    for entry in registry.select(MethodDomain::ThreeD) {
        assert!(
            entry.realized_by.is_empty(),
            "3d/{} 的 realized_by 必须为空（无对家，不是漏填）",
            entry.id.as_str()
        );
    }

    // 第三半：其余三个域为真——**不能省**：没有那个谓词时「空是刻意的」与
    // 「漏 bind」不可区分（设计 §7 第 2 条）。逐臂写，不抽代表。
    assert!(
        MethodDomain::Software.has_counterpart(),
        "software/ 有对家（P5c）"
    );
    assert!(
        MethodDomain::Video.has_counterpart(),
        "video/ 有对家（P5e）"
    );
    assert!(
        MethodDomain::Research.has_counterpart(),
        "research/ 有对家（P5d）"
    );
}

#[test]
fn no_counterpart_entry_outside_the_roster_stays_empty() {
    // 本用例是判据 6 第 2 条（「有对家域的条目除名册所列者外逐条非空」）
    // **在本 crate 内的可闭形态**：取 seeded()，对 has_counterpart == true 的三个域
    // （Software / Video / Research）里**每一条不在名册里的条目**调
    // bind(&id, &["placeholder"])；然后收集这三个域里**仍为空**的条目，
    // 断言它**恰等于**字面名册（排序后逐项）。
    //
    // 红条件**三条各写一处**：
    // - **加反例档**（a）名册多一条 ⇒ 该条被跳过不绑 ⇒ 空集多一项 ⇒ 红。
    //   **含类型长度**：常量类型改 `;3` **且**字面量改三项；只改一处是 `E0308`
    //   编译失败，不是红（纪律 1(c)，同前一条用例的说明）。
    // - **移除档**（b）名册少一条 ⇒ 另一条被绑上 ⇒ 空集少一项 ⇒ 红。
    //   同样含类型长度：类型改 `;1` **且**字面量改一项。
    // - **加反例档**（c）名册写错名（"TDD" → "tdd"）⇒ 真 TDD 被绑上、假名不在
    //   目录里 ⇒ 空集里没有 "TDD" ⇒ 红（长度不变，照旧编译得过）。
    //
    // **射程照实写明**：本用例**不**覆盖「生产里某块漏 bind」——它在用例内部自己把
    // 非名册条目全绑上了。那一侧 continuum-method 看不到（四块在别的 crate、
    // 在其后运行，设计 §4.5 末），交给 P5c／P5d／P5e 各自的计划并由整合作收口
    // （实现计划 `## 交付给谁`）。
    let mut registry = MethodRegistry::seeded();

    // 第一步：把三个有对家域里每一条**不在名册里**的条目绑上一个占位算子 id。
    let mut to_bind: Vec<MethodId> = Vec::new();
    for domain in MethodDomain::ALL {
        if !domain.has_counterpart() {
            continue;
        }
        for entry in registry.select(domain) {
            let in_roster = UNREALIZED_BY_DESIGN
                .iter()
                .any(|&(d, name)| d == domain.as_str() && name == entry.id.as_str());
            if !in_roster {
                to_bind.push(entry.id.clone());
            }
        }
    }
    for id in &to_bind {
        registry
            .bind(id, &["placeholder"])
            .expect("非名册条目必须可绑定");
    }

    // 第二步：收集这三个域里**仍为空**的条目。
    let mut still_empty: Vec<(&str, &str)> = Vec::new();
    for domain in MethodDomain::ALL {
        if !domain.has_counterpart() {
            continue;
        }
        for entry in registry.select(domain) {
            if entry.realized_by.is_empty() {
                still_empty.push((domain.as_str(), entry.id.as_str()));
            }
        }
    }

    // 第三步：断言仍为空的恰等于**字面**名册（排序后逐项；期望值不从常量自身取）。
    still_empty.sort();
    let mut expected = vec![("software", "TDD"), ("software", "debugging")];
    expected.sort();
    assert_eq!(
        still_empty, expected,
        "名册外全绑上之后，仍为空的必须恰等于字面名册 [(\"software\", \"TDD\"), (\"software\", \"debugging\")]（名册多一条、少一条、写错名，都红）"
    );
}
