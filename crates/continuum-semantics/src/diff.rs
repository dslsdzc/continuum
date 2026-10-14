//! §226 的 ContractDiff（设计 §8.1／§8.2）：四类差异的判定与并列的 `unjudged`。
//!
//! 计划「文件结构」一节把 `DiffClass、DiffEntry、UnjudgedChange、UnjudgedReason、
//! ContractDiff、diff` 归在 `src/diff.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:307`）。
//!
//! # 纯函数：不碰库、不碰图（§8.2）
//!
//! [`diff`] 收两个 [`ContractRef`]（比较基准的版本）与两侧的 [`Requirement`] 集合，
//! 返回 [`ContractDiff`]——**不收 `&Tx`、不读写任何表、不引用 `continuum-graph`**。
//! §226 的「受影响的 ADFIR 节点 MUST 被重新判定有效性」是一个**跨层义务**：
//! 本层产出判定所需的最小信息，**映射到节点并调 `propagate_invalidation` 在执行层**
//! （持有图的一侧），本层不调（§8.2）。今天这条边没有消费者（§16 第 13 条）。
//!
//! # 四类怎么判（本设计的判定，不是规范转录）
//!
//! §226 只给了四个名字：`added / removed / strengthened / weakened`。
//! **四类各自怎么判，规范没有给**。本设计把判据落在 §224 给的字段上
//! （`class` 与 `expression`）：
//!
//! | 差异类 | 判定 |
//! |---|---|
//! | [`DiffClass::Added`] | `(requirement id)` 在 vN 无、vN+1 有 |
//! | [`DiffClass::Removed`] | 在 vN 有、vN+1 无 |
//! | [`DiffClass::Strengthened`] | 同 id 两边都有，且 `class` 的**序**上升 |
//! | [`DiffClass::Weakened`] | 同 id 两边都有，且 `class` 的序下降 |
//!
//! `class` 相同时 `expression` 变了**不下判**（见下）。`verification_requirement`
//! **不做判据**：它是结构性禁读的（OPEN-001 未决期间只存不判，§7.2／Task 11），
//! 拿它做判定正是那条禁令要挡的东西。
//!
//! # 序的来历（§8.1 的订正段）
//!
//! [`class_strength`] 的序是 `UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`。
//! **这是本设计从四类含义的强弱推出来的、不是原文给的**：总纲只给四类的含义、
//! 未给序，且其列举顺序与此**相反**。**若复审判定此序不成立**，`Strengthened` /
//! `Weakened` 两类**无判据**——两类的判定会整个落进 `unjudged`（§16 第 12 条）。
//!
//! # `unjudged`：未判定是「记录」，不是差异类（§8.1 的订正段）
//!
//! `class` 相同时的 `expression` 变化判不了：§224 未给表达式的语法（§7.1），
//! 本层不发明一套表达式语言——「`duration >= 4K` 改成 `duration >= 2K`」这种
//! 语义上的削弱无法判定。这是**刻意留下的未判定**，走 [`ContractDiff::unjudged`]
//! 这个**并列字段**：不进 `entries`、更不进 [`DiffClass`]。初稿曾把它写成 §226
//! 四类的「第五类」，那与「它不是四类之一」自相矛盾——判据：只要那一项不在规范的
//! 枚举位置上，它就是「记录」；一旦成为 diff 类型的第五个变体，它就是「差异类」，
//! 措辞声明救不回来。
//!
//! # 照片
//!
//! `tests/diff.rs` 的七条用例：四枚各一条（枚名经穷尽 `match` 读出——加第五枚即
//! 编译不过）、id 集判增删、序升/降、**三对逐对**的序、同 class 异 expression 进
//! `unjudged` 且不在 `entries`、字段清单逐项（穷尽解构、不带 `..`）、签名只有
//! 两个 ref ＋ 两侧集合（`fn` 指针类型标注钉死形状）。

use crate::contract::{Requirement, RequirementClass};
use crate::ids::RequirementId;
use crate::intent::ContractRef;

/// §226 的四类差异。**恰好四枚**，与 §226 的枚举一一对应
/// （`added` / `removed` / `strengthened` / `weakened`）。
///
/// # 恰好四枚的机制与它的两半
///
/// - **枚名侧**：`tests/diff.rs` 的 `class_name` 是穷尽 `match`、不带 `..`——
///   加第五枚即**编译不过**；
/// - **边界侧**：未判定**不进本枚举**，走 [`ContractDiff::unjudged`] 的并列字段
///   （§8.1 的订正段）——「同 class 异 expression」的照片
///   （`a_same_class_with_a_changed_expression_is_unjudged_and_not_in_the_entries`）
///   把「记录 vs 差异类」的分界钉死。
///
/// 派生里 `PartialEq` / `Eq` 是判据要的（断言「判成了哪一枚」就是等值比较）；
/// `Debug` 是断言失败时打得出来；`Clone` / `Copy` 是它要随 [`DiffEntry`] 传递。
/// **没有 `Ord` / `PartialOrd`**：四类之间没有序（序在 `class` 上，见
/// [`class_strength`]），加它是今天没有用例的表达力。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffClass {
    /// §226 的 `added`：`(requirement id)` 在 vN 无、vN+1 有。
    Added,
    /// §226 的 `removed`：在 vN 有、vN+1 无。
    Removed,
    /// §226 的 `strengthened`：同 id 两边都有，且 `class` 的**序**上升
    /// （序见 [`class_strength`]）。
    Strengthened,
    /// §226 的 `weakened`：同 id 两边都有，且 `class` 的序下降。
    Weakened,
}

/// 一条**已判定**的差异（§8.1）：requirement id ＋ 四类中的一枚。
///
/// 四枚各一条照片在 `tests/diff.rs` 的 `each_diff_class_has_a_photo`。
/// 字段是 `pub` 的：本类型是判定的记录体，没有比它更强的约束可挂
/// （同 [`crate::Requirement`] / [`crate::TaskContract`] 的处置）。
///
/// 派生里**没有 `PartialEq` / `Eq`**：装的是 [`RequirementId`]，而它按本层惯例
/// 没有等值比较（断言经 `as_str()` 比，调用点看得到自己在比什么，同
/// `continuum-canonical` 的 `EntityId`）。
#[derive(Debug, Clone)]
pub struct DiffEntry {
    /// 差异所属的 requirement id（原样带出，不改写、不裁剪）。
    pub requirement_id: RequirementId,
    /// 四类中的一枚。
    pub class: DiffClass,
}

/// 一条**未判定**的变更（§8.1）：本层判不了，所以只记录。
///
/// **它不是差异类**——它是「记录」，不是「判定」：它在 [`ContractDiff::unjudged`]
/// 这个与 `entries` **并列**的字段里，不进 [`DiffClass`]（§8.1 的订正段）。
#[derive(Debug, Clone)]
pub struct UnjudgedChange {
    /// 未判定变更所属的 requirement id（原样带出，不改写、不裁剪）。
    pub requirement_id: RequirementId,
    /// 判不了的原因。
    pub reason: UnjudgedReason,
}

/// 本层判不了的**原因**（[`UnjudgedChange::reason`]）。
///
/// 今天只有一枚：`class` 相同时的 `expression` 变化——「Requirement 表达式的
/// 语义比较」这一步本层没有（§16 第 12 条），故这一变化无法判强弱。
/// 枚举不预先建「将来可能」的变体：判不了的情形出现一枚才记一枚
/// （同 [`crate::ContractError`] 只收真实产生方的处置）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnjudgedReason {
    /// 同一 `class` 下 `expression` 变了：§224 未给表达式的语法（§7.1），
    /// 本层不发明表达式语言去判它。**刻意留下的未判定**，不是遗漏。
    ExpressionChanged,
}

/// §226 的 ContractDiff：两个版本与两张集合。
///
/// # 字段清单（逐项）
///
/// `from` / `to` / `entries` / `unjudged`。字段清单**逐项**的照片是 `tests/diff.rs`
/// 的 `contract_diff_carries_the_two_versions_and_two_collections`（穷尽解构、
/// **不带 `..`**——加／少／改名即编译不过）。**不建第五个字段**：未判定已由
/// [`UnjudgedChange`] 承载（§8.1 的订正段），再加一枚「别的集合」是发明。
///
/// # `unjudged` 是并列字段，不是第四个差异类
///
/// [`ContractDiff::entries`] 里恰好是 §226 的四类（[`DiffClass`] 唯一决定）；
/// [`ContractDiff::unjudged`] 与它平级、有自己的元素类型 [`UnjudgedChange`]——
/// 「判不了」与「判成了哪一类」是两件事，合并会把「记录」偷渡成「判定」。
///
/// # 顺序
///
/// `entries` / `unjudged` 内部的**位次没有规范语义**（§226 未给顺序）：
/// 照片按 `requirement_id` 查找，不按位次。
#[derive(Debug, Clone)]
pub struct ContractDiff {
    /// 基准版本的 ref（哪个版本是「旧」）。
    pub from: ContractRef,
    /// 新版本的 ref（哪个版本是「新」）。
    pub to: ContractRef,
    /// 恰好 §226 四类的判定结果。
    pub entries: Vec<DiffEntry>,
    /// 本层判不了、只记录的变更。**并列字段，不并入 `entries`**。
    pub unjudged: Vec<UnjudgedChange>,
}

/// 计算 vN → vN+1 的 [`ContractDiff`]（§226）。
///
/// # 输入（§8.2 的签名形状）
///
/// 两个 [`ContractRef`]（比较基准：原样带进结果）＋ 两侧的 [`Requirement`] 集合。
/// **不收 `&Tx`、不读写库、不碰图**——本层产出判定所需的最小信息，
/// 映射到节点并调 `propagate_invalidation` 在执行层（`fn` 指针类型标注的照片在
/// `tests/diff.rs` 的 `the_diff_is_computed_from_two_contract_refs_only`）。
///
/// # 判定（逐条照模块文档的表格）
///
/// - vN+1 侧每个 id：vN 无 ⇒ `Added`；
/// - vN 侧每个 id：vN+1 无 ⇒ `Removed`；
/// - 两侧都有的 id：`class` 的序上升 ⇒ `Strengthened`、下降 ⇒ `Weakened`；
///   序相同而 `expression` 变了 ⇒ 进 `unjudged`（[`UnjudgedReason::ExpressionChanged`]）；
///   序相同且 `expression` 也相同 ⇒ 不是差异（两张集合里都不出现）。
///
/// # 顺序（不作承诺）
///
/// 当前实现先扫 vN+1 侧（`Added` / `Strengthened` / `Weakened` / 未判定）再扫
/// vN 侧（`Removed`），但这是实现细节：`entries` 的位次**没有规范语义**，
/// 调用方不应依赖。
pub fn diff(
    from: ContractRef,
    to: ContractRef,
    from_requirements: &[Requirement],
    to_requirements: &[Requirement],
) -> ContractDiff {
    let mut entries: Vec<DiffEntry> = Vec::new();
    let mut unjudged: Vec<UnjudgedChange> = Vec::new();

    // vN+1 侧：新出现的 id 是 Added；同 id 的比 class 的序（升降），
    // 序相同再看 expression（变了 → 未判定；没变 → 不是差异）。
    for new in to_requirements {
        let old = from_requirements
            .iter()
            .find(|old| old.id.as_str() == new.id.as_str());
        let Some(old) = old else {
            entries.push(DiffEntry {
                requirement_id: new.id.clone(),
                class: DiffClass::Added,
            });
            continue;
        };

        let old_strength = class_strength(old.class);
        let new_strength = class_strength(new.class);
        if new_strength > old_strength {
            entries.push(DiffEntry {
                requirement_id: new.id.clone(),
                class: DiffClass::Strengthened,
            });
        } else if new_strength < old_strength {
            entries.push(DiffEntry {
                requirement_id: new.id.clone(),
                class: DiffClass::Weakened,
            });
        } else if old.expression.as_str() != new.expression.as_str() {
            // 同 class 下的 expression 变化：判不了强弱，记录原因（§8.1）。
            unjudged.push(UnjudgedChange {
                requirement_id: new.id.clone(),
                reason: UnjudgedReason::ExpressionChanged,
            });
        }
    }

    // vN 侧：vN+1 已不含的 id 是 Removed。
    for old in from_requirements {
        if to_requirements
            .iter()
            .any(|new| new.id.as_str() == old.id.as_str())
        {
            continue;
        }
        entries.push(DiffEntry {
            requirement_id: old.id.clone(),
            class: DiffClass::Removed,
        });
    }

    ContractDiff {
        from,
        to,
        entries,
        unjudged,
    }
}

/// `class` 四值序的秩（§8.1）：`UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`。
///
/// # 序是本设计推出来的，不是原文给的（§8.1 的订正段）
///
/// 序的依据是四类含义的强弱（「必须满足」＞「尽量满足」＞「可调整」＞「未指定」）；
/// 总纲的列举顺序与此**相反**。**若复审判定此序不成立**，`Strengthened` /
/// `Weakened` 两类无判据——判定整个落进 `unjudged`（§16 第 12 条）。
///
/// # 无通配臂
///
/// [`RequirementClass`] 是封闭四值：加第五枚即本函数**编译不过**，逼着序一起更新
/// （同 [`RequirementClass::as_str`] 的纪律）。序属于 **diff 的判定**，不是
/// `class` 自身的属性，故不写在 `contract.rs`（那里不为此加 `Ord` 派生）。
fn class_strength(class: RequirementClass) -> u8 {
    match class {
        RequirementClass::Unspecified => 0,
        RequirementClass::Flexible => 1,
        RequirementClass::Preferred => 2,
        RequirementClass::Required => 3,
    }
}
