//! §5.4 (b) 的「表覆盖」通道（设计 `docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`）
//! 的运行期照片：`PlacementRules` 的构造期判据与它的查询面。
//!
//! # 夹具为什么是「两档对三档」而不是「五档同值」
//!
//! [`fixture_rule`] 把 [`PrivacyClass::Public`] 与 [`PrivacyClass::Private`] 判给
//! [`TransferRule::TrustedPersonalOnly`]，其余三档判给 [`TransferRule::AnyNode`]（2 对 3）。
//!
//! **换值的做法是承重的，不是随手抽的**：`TransferRule` 只有两枚而档有五枚，
//! 若五档写同一个值，那么本文件里**任何**关于 `rule_for` 的断言都只有一种取值可分辨，
//! 「把某两档的值写反」这类变异**不可观察**（等价变异体）。
//!
//! **但这一条要按实写、不许写成「写反即红」**：互换两个**同值**的档
//! （例如两条都是 `AnyNode`）仍然是**等价变异体**——那不是用例不够，
//! 是两版在**任何入参上都不会给出不同结果**。故本文件的判据是：
//! **任何改变「某档的返回值」的变异，都能被它那一档的断言抓到**；
//! `a_table_covering_every_level_is_accepted` 逐档各一条正是为了这一点。
//!
//! # 逐档断言为什么用「累积后一次断言」而不是五条 `assert_eq!` 顺序排下去
//!
//! 五条顺序排下去时，第一条不符就 panic，**「五条里红了几条」在产物里读不出来**。
//! 本文件把逐档比对的结果累积进一个 `Vec`，末尾一次断言并把它打印出来——
//! 于是「红几条」是**可读的实测值**，而不是推断：
//! `rule_for` 被改成返回常量时，
//! 常量取 `AnyNode` 则列表里恒有 **2** 条（`Public` / `Private`），
//! 取 `TrustedPersonalOnly` 则恒有 **3** 条（`Personal` / `Secret` / `LocalOnly`）。
//!
//! **按行比对，不做集合比较**（纪律 6）：两侧都是有序的 `Vec`，
//! 逐位比对才算「每一项都对上号」。

use continuum_artifact::PrivacyClass;
use continuum_node::{PlacementRules, RulesError, TransferRule};

/// 夹具：本文件要分辨的**唯一**一维是「哪一档对哪一枚 `TransferRule`」。
///
/// `Public` / `Private` → `TrustedPersonalOnly`，其余三档 → `AnyNode`（2 对 3）。
/// 取值理由见文件头。
fn fixture_rule(level: PrivacyClass) -> TransferRule {
    match level {
        PrivacyClass::Public | PrivacyClass::Private => TransferRule::TrustedPersonalOnly,
        PrivacyClass::Personal | PrivacyClass::Secret | PrivacyClass::LocalOnly => {
            TransferRule::AnyNode
        }
    }
}

/// 由 [`PrivacyClass::ALL`] 逐档生成的**全覆盖**表（设计 §5.4 (b) 的合法输入）。
fn full_table() -> Vec<(PrivacyClass, TransferRule)> {
    PrivacyClass::ALL
        .iter()
        .map(|&level| (level, fixture_rule(level)))
        .collect()
}

/// 全覆盖的表被接受，且 `rule_for` 对**每一档**返回的就是给那一档的值。
///
/// **红在末尾那一条 `assert!(mismatches.is_empty(), …)`**（本文件所有逐档比对共用这一句），
/// 报错信息里列出不符的档位／期望值／实际值，故「红几条」可读。
///
/// **红的条件（档位：取反）**：把 `rule_for` 的查表改成返回一个**常量** →
/// 红的条数等于「与该常量不同的档数」：常量取 `AnyNode` 时 **2** 条
/// （`Public` / `Private` 两档）、取 `TrustedPersonalOnly` 时 **3** 条
/// （`Personal` / `Secret` / `LocalOnly` 三档）。**总之不会五条全红，最少 2 条**
/// ——单一常量最多与 3 档对上（夹具是 2 对 3）。
/// **这正是逐档写五条的作用**：换成「只断一档」，则当那一档的期望值恰好等于常量时，
/// 常量变异体**漏掉**——`AnyNode` 常量对上三档（`Personal` / `Secret` / `LocalOnly`），
/// `TrustedPersonalOnly` 常量对上两档（`Public` / `Private`），
/// 即漏掉的概率是 **3/5 或 2/5**，取决于抽中哪一档当代表。
#[test]
fn a_table_covering_every_level_is_accepted() {
    let expected = full_table();
    // 夹具自证：两侧都非空，否则下面「红 2 条 / 红 3 条」无从谈起。
    assert_eq!(expected.len(), 5, "夹具应当由 ALL 逐档生成五条");
    assert_eq!(
        expected
            .iter()
            .filter(|(_, r)| *r == TransferRule::TrustedPersonalOnly)
            .count(),
        2,
        "夹具应当是 2 档 TrustedPersonalOnly 对 3 档 AnyNode"
    );

    let rules =
        PlacementRules::try_new(expected.clone()).expect("覆盖 PrivacyClass::ALL 每一档的表必须被接受");

    let mut mismatches: Vec<(PrivacyClass, TransferRule, TransferRule)> = Vec::new();
    for &(level, want) in expected.iter() {
        let got = rules.rule_for(level);
        if got != want {
            mismatches.push((level, want, got));
        }
    }
    assert!(
        mismatches.is_empty(),
        "rule_for 逐档比对不符（{} 档，分别为 档位/期望/实际）：{mismatches:?}",
        mismatches.len()
    );
}

/// 缺**任何**一档的表都被拒，且报出的就是缺的那一档。
///
/// **五档各一条**，不是一个代表：一条按档分支的手写检查可以各自漂移，
/// 抽代表会漏掉漂移掉的那几档。
///
/// **红的条件（档位：移除）**：删掉 `try_new` 里覆盖率检查那一整段 →
/// 五条全部落进 `failures`（缺档的表会被判 `Ok`）→ 红在末尾那一条断言上。
/// **本用例的唯一红点是「五条都红」**，故实现后必须确认**五条都跑了**
/// （`failures` 的长度在报错信息里，实测见报告）。
#[test]
fn a_table_missing_any_level_is_rejected() {
    let mut failures: Vec<(PrivacyClass, String)> = Vec::new();
    for &missing in PrivacyClass::ALL.iter() {
        let entries: Vec<(PrivacyClass, TransferRule)> = PrivacyClass::ALL
            .iter()
            .filter(|&&level| level != missing)
            .map(|&level| (level, fixture_rule(level)))
            .collect();
        assert_eq!(entries.len(), 4, "夹具应当只有四条（去掉 {missing:?} 之后）");

        match PlacementRules::try_new(entries) {
            Err(RulesError::MissingLevel { level }) if level == missing => {}
            Err(RulesError::MissingLevel { level }) => failures.push((
                missing,
                format!("MissingLevel {{ level: {level:?} }}（报的不是缺的那一档）"),
            )),
            Err(RulesError::DuplicateLevel { level }) => failures.push((
                missing,
                format!("DuplicateLevel {{ level: {level:?} }}（表不重复，判错了变体）"),
            )),
            // `Ok(_)` 故意不打印内部值：`PlacementRules` 不派生 `Debug`（设计 §5.4 末段）。
            Ok(_) => failures.push((missing, "Ok（缺档却未被拒）".to_string())),
        }
    }
    assert!(
        failures.is_empty(),
        "以下档位缺档时未被判 MissingLevel{{level}}（档位/实际结果）：{failures:?}"
    );
}

/// 覆盖齐但某档重复一次的表被拒，且报出的就是**重复**的那一档。
///
/// **与上一条的分工**：重复一条**不违反覆盖率**（六条覆盖齐），
/// 故只钉覆盖率是抓不到它的——两条用例各钉一处。
///
/// **逐档各一条**（brief 写的是一张六条表；这里把「重复的是哪一档」在**每一档上**都拍一次）：
/// 只重复某一档时，若实现报的是**第一次出现的那一档**而不是重复的那一档，
/// 则当那句「重复档」不是首条时才看得出来——五档各拍一次，
/// 就把「报哪一档」这个问题在每个位置上都问了一遍。
///
/// **红的条件（档位：移除）**：删掉 `try_new` 里的重复检查 →
/// 这五张表（六条、覆盖齐）全被判 `Ok` → 五条落进 `failures` → 红在末尾那一条断言上。
#[test]
fn a_table_repeating_a_level_is_rejected() {
    let mut failures: Vec<(PrivacyClass, String)> = Vec::new();
    for &duplicated in PrivacyClass::ALL.iter() {
        let mut entries = full_table();
        entries.push((duplicated, fixture_rule(duplicated)));
        assert_eq!(entries.len(), 6, "夹具应当是六条（五档齐 ＋ {duplicated:?} 重复一次）");

        match PlacementRules::try_new(entries) {
            Err(RulesError::DuplicateLevel { level }) if level == duplicated => {}
            Err(RulesError::DuplicateLevel { level }) => failures.push((
                duplicated,
                format!("DuplicateLevel {{ level: {level:?} }}（报的不是重复的那一档）"),
            )),
            Err(RulesError::MissingLevel { level }) => failures.push((
                duplicated,
                format!("MissingLevel {{ level: {level:?} }}（表覆盖齐，判错了变体）"),
            )),
            Ok(_) => failures.push((duplicated, "Ok（重复档却未被拒）".to_string())),
        }
    }
    assert!(
        failures.is_empty(),
        "以下档位被重复时未被判 DuplicateLevel{{level}}（档位/实际结果）：{failures:?}"
    );
}

/// 覆盖率判据的输入侧事实：`PrivacyClass::ALL` 是五枚、且五枚**两两不同**。
///
/// 用**计数**断言（每档在 `ALL` 里恰好出现 1 次），不用「看起来没有重复」。
///
/// # 射程（照实写）
///
/// **这一条钉的是输入侧的事实**，不是实现的取法。
/// **它拍不到「实现用的是 `ALL` 还是手抄的同一份五档清单」**——
/// 那两版在今天**是等价变异体**（手抄清单与 `ALL` 逐项相同，只有加第六档才能区分；
/// 见设计 §5.4「覆盖判据的来源是 `PrivacyClass::ALL`」与 §10 第 17 条）。
/// 故本文件**不写一条声称「来源是 `ALL`」的运行期断言**：那会是一条没有照片的断言。
///
/// 末尾的机制自证（合成一份带重复的五元数组，计数法必须报 2）是**正控制**：
/// 没有它，一个恒返回 1 的计数写法会永远绿——这正是「守卫须两侧都钉」里缺的那一侧。
#[test]
fn the_coverage_criterion_reads_the_five_levels_from_the_type() {
    assert_eq!(
        PrivacyClass::ALL.len(),
        5,
        "ALL 应当是五档（设计 §5.4 (b) 的判据面按它取）"
    );

    let mut bad: Vec<(PrivacyClass, usize)> = Vec::new();
    for &level in PrivacyClass::ALL.iter() {
        let count = PrivacyClass::ALL.iter().filter(|&&l| l == level).count();
        if count != 1 {
            bad.push((level, count));
        }
    }
    assert!(
        bad.is_empty(),
        "以下档位在 ALL 里的出现次数不是 1（档位/次数）：{bad:?}"
    );

    // 机制自证：同一套计数写法在**真有重复**的数组上必须报 2，
    // 否则上面那条「每档恰好 1 次」在恒返回 1 的写法下也会绿。
    let with_duplicate = [
        PrivacyClass::Public,
        PrivacyClass::Public,
        PrivacyClass::Private,
        PrivacyClass::Secret,
        PrivacyClass::LocalOnly,
    ];
    let public_count = with_duplicate
        .iter()
        .filter(|&&l| l == PrivacyClass::Public)
        .count();
    assert_eq!(
        public_count, 2,
        "机制自证失败：计数法在真出现重复的数组上没报出 2"
    );
}
