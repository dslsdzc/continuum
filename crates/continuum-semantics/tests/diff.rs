//! §226 的 ContractDiff（设计 §8.1／§8.2）：四类差异（`added` / `removed` /
//! `strengthened` / `weakened`）与**并列**的 `unjudged`。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `each_diff_class_has_a_photo` | `DiffClass` **四枚各一条**：四个场景各判成对应的一枚；枚名经**穷尽 `match`、不带 `..`** 读出（加第五枚即编译不过——「记录 vs 差异类」的分界钉在类型上） | 任一场景判错枚即红；把未判定做成第五枚即**编译不过** |
//! | `added_and_removed_are_decided_by_the_id_set` | 增删**只由 id 集决定**：`(id)` 在 vN 无、vN+1 有 → `Added`；反向 → `Removed`；两边都有的 id 不出现在 `entries` | 把「两边都有但没变」的 id 也算成差异即红 |
//! | `a_rising_class_is_strengthened_and_a_falling_one_is_weakened` | 同 id 两边都有：`class` 序上升 → `Strengthened`；下降 → `Weakened`（各一条） | 升/降互换（**取反档**）即红 |
//! | `the_class_order_is_the_one_the_design_declared` | `UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED` **三对逐对**（双向）：每对低 → 高是 `Strengthened`、高 → 低是 `Weakened` | 任一对次序反了／合并了即红（**这是设计从四类含义推出来的序，不是原文给的**——§8.1 的订正段；若复审判定此序不成立，两类整个落进 `unjudged`，§16 第 12 条） |
//! | `a_same_class_with_a_changed_expression_is_unjudged_and_not_in_the_entries` | 同 id、`class` 相同、`expression` 不同 → 进 `unjudged`，**且逐条断言它不在 `entries` 里** | 把未判定项并进 `entries`（**移除档**）即红 |
//! | `contract_diff_carries_the_two_versions_and_two_collections` | 字段清单**逐项**（穷尽解构，**不带 `..`**）：`from` / `to` / `entries` / `unjudged`；`unjudged` 是**并列字段**，不是第四个差异类 | 加第五字段即**编译不过**；`unjudged` 并进 `entries` 的模型即红 |
//! | `the_diff_is_computed_from_two_contract_refs_only` | **签名层面**：两个 `ContractRef` ＋ 两侧 `Requirement` 集合（`fn` 指针类型标注钉死形状——没有 `&Tx`、没有任何图类型） | 加一个参数（如 `&Tx`）／换返回类型即**编译不过** |
//!
//! # 夹具的来历
//!
//! 本文件**不建库、不碰 `persist.rs`**：ContractDiff 是纯函数（§8.2——本层不调
//! `propagate_invalidation`、不引用 `continuum-graph`）。`contract_ref` 与
//! `requirement` 都是**每个字段取一个非默认值**的基准值（同 `tests/contract.rs`
//! 的 `sample_requirement` 惯例）。
//!
//! 条目在 `entries` 里的**位次没有规范语义**（§226 未给顺序）：下面的断言一律按
//! `requirement_id` 查找条目，不按位次。

use continuum_canonical::Opaque;
use continuum_semantics::{
    ContractDiff, ContractId, ContractRef, DiffClass, DiffEntry, Requirement, RequirementClass,
    RequirementId, UnjudgedChange, UnjudgedReason, VerificationRequirement, diff,
};

/// 夹具：同一条 contract 的某个版本（id 固定，版本由调用方给）。
fn contract_ref(version: u32) -> ContractRef {
    ContractRef {
        id: ContractId::from_str("c-1"),
        version,
    }
}

/// 夹具：一条 §224 的 Requirement，五个字段各取一个值（expression 由调用方给）。
fn requirement(id: &str, class: RequirementClass, expression: &str) -> Requirement {
    Requirement {
        id: RequirementId::from_str(id),
        class,
        expression: Opaque::from_str(expression),
        source: Opaque::from_str(&format!("source-{id}")),
        verification_requirement: VerificationRequirement::from_str(&format!("vr-{id}")),
    }
}

/// 从 `entries` 里取 `requirement_id` 的那一条；找不到直接炸出全部条目。
fn entry_of<'a>(entries: &'a [DiffEntry], requirement_id: &str) -> &'a DiffEntry {
    entries
        .iter()
        .find(|e| e.requirement_id.as_str() == requirement_id)
        .unwrap_or_else(|| {
            panic!("entries 里应有 id 为 {requirement_id:?} 的条目，实得 {entries:?}")
        })
}

/// 从 `unjudged` 里取 `requirement_id` 的那一条；找不到直接炸出全部未判定项。
fn unjudged_of<'a>(unjudged: &'a [UnjudgedChange], requirement_id: &str) -> &'a UnjudgedChange {
    unjudged
        .iter()
        .find(|u| u.requirement_id.as_str() == requirement_id)
        .unwrap_or_else(|| {
            panic!("unjudged 里应有 id 为 {requirement_id:?} 的项，实得 {unjudged:?}")
        })
}

/// `DiffClass` 四枚的符号名（断言消息里报「实得的是哪一枚」）。
///
/// **穷尽 `match`、不带 `..`**：`DiffClass` 一旦有第五枚，本函数即**编译不过**——
/// 这正是「四类留在 `DiffClass`、未判定走并列字段 `unjudged`」的机制（§8.1 的
/// 订正段：一旦未判定成为 diff 的第五个变体，它就是差异类，措辞声明救不回来）。
fn class_name(class: DiffClass) -> &'static str {
    match class {
        DiffClass::Added => "added（新增）",
        DiffClass::Removed => "removed（移除）",
        DiffClass::Strengthened => "strengthened（收紧）",
        DiffClass::Weakened => "weakened（放松）",
    }
}

/// `DiffClass` **四枚各一条**：四个场景各判成对应的一枚。
///
/// 枚名经 [`class_name`] 的**穷尽 `match`** 读出——加第五枚即编译不过。
/// 「记录 vs 差异类」的分界另有 `a_same_class_with_a_changed_expression_…`
/// 一条用例（未判定不进 `entries`），两条合起来才是判据的完整形态。
#[test]
fn each_diff_class_has_a_photo() {
    // 第 1 枚 `Added`：vN 无、vN+1 有（`(id)` 判定，见下一条用例）。
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[],
        &[requirement("r-added", RequirementClass::Required, "x >= 1")],
    );
    assert_eq!(d.entries.len(), 1, "第 1 个场景应恰一条差异");
    let e = entry_of(&d.entries, "r-added");
    assert_eq!(
        e.class,
        DiffClass::Added,
        "vN 无、vN+1 有的 id 应判第一枚；实得 {}",
        class_name(e.class)
    );

    // 第 2 枚 `Removed`：vN 有、vN+1 无。
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[requirement("r-removed", RequirementClass::Required, "x >= 1")],
        &[],
    );
    assert_eq!(d.entries.len(), 1, "第 2 个场景应恰一条差异");
    let e = entry_of(&d.entries, "r-removed");
    assert_eq!(
        e.class,
        DiffClass::Removed,
        "vN 有、vN+1 无的 id 应判第二枚；实得 {}",
        class_name(e.class)
    );

    // 第 3 枚 `Strengthened`：同 id、class 的序上升（FLEXIBLE → REQUIRED）。
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[requirement("r-up", RequirementClass::Flexible, "x >= 1")],
        &[requirement("r-up", RequirementClass::Required, "x >= 1")],
    );
    assert_eq!(d.entries.len(), 1, "第 3 个场景应恰一条差异");
    let e = entry_of(&d.entries, "r-up");
    assert_eq!(
        e.class,
        DiffClass::Strengthened,
        "class 序上升应判第三枚；实得 {}",
        class_name(e.class)
    );

    // 第 4 枚 `Weakened`：同 id、class 的序下降（REQUIRED → FLEXIBLE）。
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[requirement("r-down", RequirementClass::Required, "x >= 1")],
        &[requirement("r-down", RequirementClass::Flexible, "x >= 1")],
    );
    assert_eq!(d.entries.len(), 1, "第 4 个场景应恰一条差异");
    let e = entry_of(&d.entries, "r-down");
    assert_eq!(
        e.class,
        DiffClass::Weakened,
        "class 序下降应判第四枚；实得 {}",
        class_name(e.class)
    );
}

/// 增删**只由 id 集决定**：`(id)` 在 vN 无、vN+1 有 → `Added`；反向 → `Removed`。
///
/// 夹具两侧都含 `r-2`（class / expression 都相同）——它**两边都在、没有变化**，
/// 既不在 `entries` 里也不在 `unjudged` 里。红条件：把「两边都有但没变」的 id
/// 也算成差异即红。
#[test]
fn added_and_removed_are_decided_by_the_id_set() {
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[
            requirement("r-1", RequirementClass::Required, "expr-1"),
            requirement("r-2", RequirementClass::Flexible, "expr-2"),
        ],
        &[
            requirement("r-2", RequirementClass::Flexible, "expr-2"),
            requirement("r-3", RequirementClass::Required, "expr-3"),
        ],
    );

    assert_eq!(d.entries.len(), 2, "应恰两条差异（一增一删，r-2 不动）");
    assert_eq!(
        entry_of(&d.entries, "r-3").class,
        DiffClass::Added,
        "r-3 在 vN 无、vN+1 有，应判 Added（判定只看 id 集）"
    );
    assert_eq!(
        entry_of(&d.entries, "r-1").class,
        DiffClass::Removed,
        "r-1 在 vN 有、vN+1 无，应判 Removed（判定只看 id 集）"
    );
    assert!(
        d.entries.iter().all(|e| e.requirement_id.as_str() != "r-2"),
        "r-2 两边都在、class 与 expression 都相同——不应出现在 entries 里；实得 {:?}",
        d.entries
    );
    assert!(
        d.unjudged.is_empty(),
        "本夹具没有未判定项；实得 {:?}",
        d.unjudged
    );
}

/// 同 id 两边都有：`class` 的序上升 → `Strengthened`、下降 → `Weakened`（各一条）。
///
/// 两个场景的 `expression` 都不动——变化的只有 `class` 的序。
#[test]
fn a_rising_class_is_strengthened_and_a_falling_one_is_weakened() {
    // 上升：FLEXIBLE → REQUIRED。
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[requirement("r-up", RequirementClass::Flexible, "x >= 1")],
        &[requirement("r-up", RequirementClass::Required, "x >= 1")],
    );
    assert_eq!(d.entries.len(), 1, "上升场景应恰一条差异");
    assert_eq!(
        entry_of(&d.entries, "r-up").class,
        DiffClass::Strengthened,
        "FLEXIBLE → REQUIRED 是序上升，应判 Strengthened"
    );
    assert!(d.unjudged.is_empty(), "class 序能判强弱，不应有未判定");

    // 下降：REQUIRED → PREFERRED。
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[requirement("r-down", RequirementClass::Required, "x >= 1")],
        &[requirement("r-down", RequirementClass::Preferred, "x >= 1")],
    );
    assert_eq!(d.entries.len(), 1, "下降场景应恰一条差异");
    assert_eq!(
        entry_of(&d.entries, "r-down").class,
        DiffClass::Weakened,
        "REQUIRED → PREFERRED 是序下降，应判 Weakened"
    );
    assert!(d.unjudged.is_empty(), "class 序能判强弱，不应有未判定");
}

/// `UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`：**三对逐对**（每对双向）。
///
/// # 这个序是本设计推出来的，不是原文给的（§8.1 的订正段）
///
/// 总纲的列举顺序与此**相反**；这个序的依据是四类含义的强弱（「必须满足」＞
/// 「尽量满足」＞「可调整」＞「未指定」）。**若复审判定此序不成立**，
/// `Strengthened` / `Weakened` 两类**整个落进 `unjudged`**（§16 第 12 条）——
/// 那时本用例的期望整个改写成「进 unjudged」，而不是修修补补。
///
/// 三对逐对、不抽代表：只测 `UNSPECIFIED < REQUIRED` 之类的一对，
/// 中间两枚的位次反了看不出来。
#[test]
fn the_class_order_is_the_one_the_design_declared() {
    let pairs = [
        (RequirementClass::Unspecified, RequirementClass::Flexible),
        (RequirementClass::Flexible, RequirementClass::Preferred),
        (RequirementClass::Preferred, RequirementClass::Required),
    ];

    for (low, high) in pairs {
        // 升：低 → 高 判 Strengthened。
        let d = diff(
            contract_ref(1),
            contract_ref(2),
            &[requirement("r-order", low, "x >= 1")],
            &[requirement("r-order", high, "x >= 1")],
        );
        assert_eq!(d.entries.len(), 1, "{low:?} → {high:?} 应恰一条差异");
        assert_eq!(
            entry_of(&d.entries, "r-order").class,
            DiffClass::Strengthened,
            "{low:?} → {high:?} 是序上升，应判 Strengthened（三对逐对）"
        );

        // 降：高 → 低 判 Weakened。
        let d = diff(
            contract_ref(1),
            contract_ref(2),
            &[requirement("r-order", high, "x >= 1")],
            &[requirement("r-order", low, "x >= 1")],
        );
        assert_eq!(d.entries.len(), 1, "{high:?} → {low:?} 应恰一条差异");
        assert_eq!(
            entry_of(&d.entries, "r-order").class,
            DiffClass::Weakened,
            "{high:?} → {low:?} 是序下降，应判 Weakened（三对逐对）"
        );
    }
}

/// **本 task 的要害**：同 id、`class` 相同、`expression` 不同 → 进 `unjudged`，
/// **且不在 `entries` 里**（逐条比对 `entries` 的 `requirement_id`）。
///
/// # 判不了的来历（写在用例里，因为它是判据的一部分）
///
/// §224 未给表达式的语法（§7.1），本层没有「Requirement 表达式的语义比较」
/// 这一步（§16 第 12 条）——「`duration >= 4K` 改成 `duration >= 2K`」这种
/// 语义上的削弱**判不了**，本设计不发明一套表达式语言去判它。这是**刻意留下的
/// 未判定**：它进并列字段 `unjudged`。§8.1 的订正段：一旦把它放进 `DiffClass`
/// （第五个变体）或并进 `entries`，它就是「差异类」，措辞声明救不回来。
///
/// 红条件：把未判定项并进 `entries`（**移除档**）即红。
#[test]
fn a_same_class_with_a_changed_expression_is_unjudged_and_not_in_the_entries() {
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[requirement(
            "r-1",
            RequirementClass::Required,
            "duration >= 4K",
        )],
        &[requirement(
            "r-1",
            RequirementClass::Required,
            "duration >= 2K",
        )],
    );

    // 出现在 `unjudged` 里：恰一条、id 对、理由是「表达式变了但判不了强弱」。
    assert_eq!(d.unjudged.len(), 1, "未判定应恰一条；实得 {:?}", d.unjudged);
    let u = unjudged_of(&d.unjudged, "r-1");
    assert_eq!(
        u.reason,
        UnjudgedReason::ExpressionChanged,
        "未判定的理由应是「同一 class 下 expression 变了」"
    );

    // **且它不在 `entries` 里**——逐条比对 `entries` 的 `requirement_id`。
    for entry in &d.entries {
        assert_ne!(
            entry.requirement_id.as_str(),
            "r-1",
            "未判定项不得并进 entries（§8.1 的订正段：并进 entries 或成为枚举变体 \
             就是差异类，措辞声明救不回来）；实得 {:?}",
            d.entries
        );
    }
    assert!(
        d.entries.is_empty(),
        "本夹具下 entries 应恰为空：r-1 是未判定（记录），不是差异（判定）；实得 {:?}",
        d.entries
    );
}

/// 字段清单**逐项**：`from` / `to` / `entries` / `unjudged`。
///
/// 穷尽解构、**不带 `..`**：加第五个字段即**编译不过**（这条把「`unjudged` 是
/// 并列字段、不是第四个差异类」的模型钉在类型上——它和 `entries` 平级、
/// 有自己的元素类型 [`UnjudgedChange`]、不进 [`DiffClass`]，§8.1 的订正段）。
#[test]
fn contract_diff_carries_the_two_versions_and_two_collections() {
    let d = diff(
        contract_ref(1),
        contract_ref(2),
        &[
            requirement("r-unjudged", RequirementClass::Required, "duration >= 4K"),
            requirement("r-removed", RequirementClass::Required, "expr-r"),
        ],
        &[
            requirement("r-unjudged", RequirementClass::Required, "duration >= 2K"),
            requirement("r-added", RequirementClass::Required, "expr-a"),
        ],
    );

    // 字段清单逐项（穷尽解构，不带 `..`）。
    let ContractDiff {
        from,
        to,
        entries,
        unjudged,
    } = d;

    assert_eq!(from.id.as_str(), "c-1", "`from` 应是基准那一版的 ref");
    assert_eq!(from.version, 1, "`from` 应是基准那一版的 ref（版本逐字段）");
    assert_eq!(to.id.as_str(), "c-1", "`to` 应是新版的 ref（同一条 contract）");
    assert_eq!(to.version, 2, "`to` 应是新版的 ref");
    assert_eq!(entries.len(), 2, "本夹具应恰两条差异（一增一删）");
    assert_eq!(
        entry_of(&entries, "r-added").class,
        DiffClass::Added,
        "r-added 是 vN+1 新出现的 id"
    );
    assert_eq!(
        entry_of(&entries, "r-removed").class,
        DiffClass::Removed,
        "r-removed 是 vN+1 不含的 id"
    );
    // `unjudged` 是**并列字段，不是第四个差异类**：它和 `entries` 平级。
    assert_eq!(unjudged.len(), 1, "本夹具应恰一条未判定");
    assert_eq!(
        unjudged_of(&unjudged, "r-unjudged").requirement_id.as_str(),
        "r-unjudged",
        "未判定项按 id 出现在并列字段 `unjudged` 里"
    );
}

/// **签名层面**：`diff` 收的是**两个 [`ContractRef`] ＋ 两侧的 [`Requirement`] 集合**。
///
/// `fn` 指针的类型标注把形状钉死：加一个参数（如 `&Tx`）、换返回类型（如带
/// `Result`）、或让某一侧变成所有权，都**编译不过**。§8.2：本层**不调
/// `propagate_invalidation`**、不引用 `continuum-graph`——「不收 `&Tx`、不碰图」
/// 就是这条类型标注；grep 的第二侧归 Task 21。
#[test]
fn the_diff_is_computed_from_two_contract_refs_only() {
    let compute: fn(ContractRef, ContractRef, &[Requirement], &[Requirement]) -> ContractDiff = diff;

    let d = compute(
        contract_ref(1),
        contract_ref(2),
        &[],
        &[requirement("r-1", RequirementClass::Required, "x >= 1")],
    );

    // 结果直接携带调用方给的两个基准 ref——不去库里找、不读别的输入。
    assert_eq!(d.from.id.as_str(), "c-1", "`from` 应是调用方给的基准 ref");
    assert_eq!(d.from.version, 1, "`from` 应是调用方给的基准 ref（版本原样）");
    assert_eq!(d.to.version, 2, "`to` 应是调用方给的新版 ref（版本原样）");
    assert_eq!(
        entry_of(&d.entries, "r-1").class,
        DiffClass::Added,
        "两侧集合直接作为比较输入（r-1 在 vN+1 侧才出现）"
    );
}
