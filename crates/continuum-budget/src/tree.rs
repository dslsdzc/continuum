//! 预算树：`BudgetOwner`（树上的节点）与四个写入口 ＋ 一个读入口
//! （[`record_root`] / [`allocate`] / [`reserve`] / [`settle`] / [`remaining`]；
//! 账本侧还有 [`crate::dangling_reservations`]）。
//!
//! # 单一预算树（ENG-005 §三.1）
//!
//! 节点预算与探索预算都是**树上节点的分配**——「树」的边由 `allocate` 的
//! `parent → child` 写入 `budget_node.parent` 列；I2（`Σ 子节点分配额 ≤ 父的可用额`）
//! 就是「`parent` 列等于它的那些行的和」。**分配是上限、不转移**（协调者裁定，
//! 2026-10-06）：父的可用额不因一次分配而减少，减少它的是自己的 `settle` 与
//! 在飞的 `reserve`。
//!
//! # 写函数不自己 `commit`
//!
//! 事务由调用方持有并提交（同本 crate 的 `persist.rs`；任一步失败，调用方必须
//! 回滚，不得提交）。

use continuum_persist::Tx;
use continuum_semantics::IntentId;

use crate::account::{Allocation, Remaining, Reservation};
use crate::dimensions::{ALL, DimensionKind, Dimensions};
use crate::error::BudgetError;
use crate::ledger::LedgerKind;
use crate::persist;

/// 树上的一个节点（设计 §12.1）。
///
/// # 四枚变体（用例名里的「五」照实交代）
///
/// 设计 §12.1 的代码块标题写「**五个** owner 变体齐了才叫『单一树』」，而其正文
/// 与 §12.2.1、§16 第 18 条一致列**四枚**（本枚举）。用例名
/// `the_tree_is_one_tree_with_five_owner_variants` 沿用计划的措辞，钉的是这四枚
/// ——**本 task 不发明第五枚**。
///
/// # `NodeRun.node` / `Exploration.of_node` 是不透明 `String`
///
/// `NodeId` 在本仓是不透明 `String`（§1.2.3 规则 3）——本层不解释它的取值。
#[derive(Debug, Clone)]
pub enum BudgetOwner {
    /// 系统级（维护）预算根。**创建方是第 7 层**（§16 第 18 条）——本层提供该变体
    /// 与它的**入账函数**（[`record_root`]，owner 由调用方传入），但**本层内部
    /// 没有任何流程**创建它（否定式照片见 `tests/tree.rs`）。
    SystemMaintenance,
    /// 某个 Intent 的预算节点。
    Intent(IntentId),
    /// 某个 Intent 的某次节点运行。
    NodeRun { intent: IntentId, node: String },
    /// 某个节点运行之上的探索预算（§334 的落点之一）。
    Exploration { intent: IntentId, of_node: String },
}

impl BudgetOwner {
    /// 落库键：复合键的**无歧义**编码（`budget_node.owner` 与 `budget_ledger.owner`
    /// 两列的形态）。
    ///
    /// # 为什么是 JSON（而不是拼分隔符）
    ///
    /// 四个变体的字段都是**任意串**（`IntentId` 无校验、`node` / `of_node` 是不透明
    /// `String`），拼接分隔符在有取值本身含分隔符时不可逆——JSON 的转义由
    /// serde_json 保证，任何串都无歧义往返。`kind` 的取值**小写加下划线**
    /// （本仓枚举列纪律的复合键同位处置）。四枚：
    ///
    /// - `{"kind":"system_maintenance"}`
    /// - `{"kind":"intent","id":…}`
    /// - `{"kind":"node_run","intent":…,"node":…}`
    /// - `{"kind":"exploration","intent":…,"of_node":…}`
    ///
    /// 照片：`tests/persist.rs` 的
    /// `the_enum_codes_use_lowercase_underscore_and_parse_back` 的四变体往返。
    pub fn as_key(&self) -> String {
        match self {
            // 无字段变体：固定串（不走 `json!`——没有要转义的字段，且这一行的形态
            // 是 `SystemMaintenance` 否定式照片的白名单条目之一）。
            BudgetOwner::SystemMaintenance => "{\"kind\":\"system_maintenance\"}".to_owned(),
            BudgetOwner::Intent(id) => {
                serde_json::json!({"kind": "intent", "id": id.as_str()}).to_string()
            }
            BudgetOwner::NodeRun { intent, node } => serde_json::json!({
                "kind": "node_run",
                "intent": intent.as_str(),
                "node": node,
            })
            .to_string(),
            BudgetOwner::Exploration { intent, of_node } => serde_json::json!({
                "kind": "exploration",
                "intent": intent.as_str(),
                "of_node": of_node,
            })
            .to_string(),
        }
    }

    /// [`BudgetOwner::as_key`] 的逆。**认不出的键返回 `None`**（不是 JSON、缺字段、
    /// kind 表外、字段类型不对，一律 `None`）——调用点转**具体 `Err`**（带原始串）、
    /// **不取默认值**（照片：`tests/persist.rs` 的
    /// `out_of_table_values_read_back_as_specific_errors` 的 (c)）。
    pub fn parse(key: &str) -> Option<BudgetOwner> {
        let value: serde_json::Value = serde_json::from_str(key).ok()?;
        match value.get("kind")?.as_str()? {
            "system_maintenance" => Some(BudgetOwner::SystemMaintenance),
            "intent" => Some(BudgetOwner::Intent(IntentId::from_str(
                value.get("id")?.as_str()?,
            ))),
            "node_run" => Some(BudgetOwner::NodeRun {
                intent: IntentId::from_str(value.get("intent")?.as_str()?),
                node: value.get("node")?.as_str()?.to_owned(),
            }),
            "exploration" => Some(BudgetOwner::Exploration {
                intent: IntentId::from_str(value.get("intent")?.as_str()?),
                of_node: value.get("of_node")?.as_str()?.to_owned(),
            }),
            _ => None,
        }
    }
}

/// 两个 owner 相等 ⟺ 同变体且各字段的串相等（**手写而不是派生**）。
///
/// # 判据
///
/// `IntentId` 没有 `PartialEq`（`ids.rs` 的模块文档：「本层没有给『两个 id 是同
/// 一个』以外的任何判据」），而树键的相等判据就是「**编码后是同一个节点**」——
/// 手写按 `as_str` 比较比派生更准确地表达这一点，且 [`BudgetError::UnknownOwner`]
/// 的 `owner` 要能逐值比较（错误类型的断言形状）。
impl PartialEq for BudgetOwner {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BudgetOwner::SystemMaintenance, BudgetOwner::SystemMaintenance) => true,
            (BudgetOwner::Intent(a), BudgetOwner::Intent(b)) => a.as_str() == b.as_str(),
            (
                BudgetOwner::NodeRun { intent: a, node: n },
                BudgetOwner::NodeRun { intent: b, node: m },
            ) => a.as_str() == b.as_str() && n == m,
            (
                BudgetOwner::Exploration {
                    intent: a,
                    of_node: o,
                },
                BudgetOwner::Exploration {
                    intent: b,
                    of_node: p,
                },
            ) => a.as_str() == b.as_str() && o == p,
            _ => false,
        }
    }
}

impl Eq for BudgetOwner {}

/// §12.2 结算的结局。
///
/// # `Over` **不是** `Err`
///
/// 实际超过预扣是 §110 的**运行期超支**——本层只回答「超了多少、哪一维」，
/// 判给执行层（`FailureClass::Constraint` ＋ 该节点的 `EscalationPolicy::Decision`）。
/// 做成 `Err` 会让调用方把它当失败而中止。
///
/// # 只派生 `Debug`（同 `Dimensions` 的判据）
///
/// 今天没有 `PartialEq` 的消费方（测试逐字段/逐变体断言）；`Debug` 是断言失败时
/// 必须打得出来的那个（它反过来要求 [`Dimensions`] 派生 `Debug`——那是它的
/// 当场消费方）。
#[derive(Debug)]
pub enum SettleOutcome {
    /// 实际用量没有超出预扣：`refunded` **逐维**是退回去的差额
    /// （`refunded` 与本次预扣同定义域：预扣没取的维为 `None`；无读数的维退 0
    /// ——「按预扣结算，余量不退」）。
    Within { refunded: Dimensions },
    /// 实际用量超出了预扣：**哪一维**、**超了多少**。
    /// （一次结算里多维多超时，本实现报**字段序第一处**违反者——设计未给这种
    /// 情形下的选择规则；单维超时的照片在 `tests/tree.rs`。）
    Over { dimension: DimensionKind, by: i64 },
}

/// 拒收含负值的 `Dimensions`（§12.2.1 的同一口径——本 crate 里**所有**吃
/// `Dimensions` 的入口共用本检查、不做豁免：`allocate` / `reserve` / `settle` /
/// `record_root` 与 Task 18 的 `check`）。
pub(crate) fn reject_negative(dims: &Dimensions) -> Result<(), BudgetError> {
    match dims.first_negative() {
        Some((dimension, value)) => Err(BudgetError::NegativeAmount { dimension, value }),
        None => Ok(()),
    }
}

/// **根入账函数**：把一枚 owner 记为一棵（子）树的根——`budget_node` 的一行
/// （`parent = NULL`）＋ 账本 `allocate` 行。**不 `commit`**。
///
/// # 这个函数是什么（一处申报）
///
/// 设计 §16 第 18 条写「本层提供该 owner 变体与**它的入账函数**，但**没有任何
/// 一方会创建它**」——本函数就是那个入账函数：owner 由**调用方**传入
/// （第 7 层将来创建维护性预算根时、今天由测试建树时），**本层内部没有任何流程
/// 调用它**，也**不在 `src/` 里构造 `SystemMaintenance` 的值**（唯一构造它的
/// 位置是 `parse`——读路径；否定式照片与白名单在 `tests/tree.rs` 的
/// `the_tree_is_one_tree_with_five_owner_variants`）。它**通用**（任何 owner 都可
/// 为根）——本层不发明「谁能当根」的拓扑判据。
///
/// # 它也是「吃 `Dimensions` 的入口」（同一口径）
///
/// §12.2.1 的表列四个入口（`allocate` / `reserve` / `settle` / `check`）；本函数
/// 同刻落地、同样吃一份 `Dimensions`——**拒负值（`NegativeAmount`）、不做豁免**。
/// 否则「同一个口径管辖所有吃 `Dimensions` 的入口」在这里破例，而 `dims` 的产生
/// 方与 `allocate` 的相同（规划方），负值同样是它的 bug。
///
/// # 失败路径
///
/// 负值在任何库写入之前被拒（账本天然未变）；重复入账（同一个 owner 已是节点）
/// 撞主键得 [`BudgetError::Persist`]——**不静默覆写**。
pub fn record_root(
    tx: &Tx<'_>,
    owner: BudgetOwner,
    dims: Dimensions,
    at: i64,
) -> Result<Allocation, BudgetError> {
    reject_negative(&dims)?;
    persist::insert_node(tx, &owner, None, &dims)?;
    persist::append_ledger(tx, &owner, LedgerKind::Allocate, &dims, at)?;
    Ok(Allocation(dims))
}

/// 父给子的分配（§12.2 的 **I2 的强制点**）。**超分配在分配那一刻就被拒**：
/// `Σ 子节点分配额 ≤ 父的可用额`（逐维），不满足 ⇒
/// [`BudgetError::Insufficient`]，**是超的那一维**。
///
/// # 判定的口径（写死）
///
/// - 只对**本次 `dims` 里 `Some` 的维**判定（该维 `None` = 本次不构成约束）；
/// - 父该维可用额为 `None`（不构成约束）时**跳过**该维；
/// - `Σ 子节点分配额` 的求和里，孩子该维 `None` **贡献 0**；
/// - 负分配额拒（[`BudgetError::NegativeAmount`]）——「负分配让 I2 更易满足」
///   不是豁免的理由（§12.2.1）。
///
/// # 失败路径
///
/// 负值 / 父不在树上 / I2 越界都在**任何库写入之前**返回——被拒的分配**不进树、
/// 不落账**（照片：`tests/tree.rs` 的 `i2_is_enforced_at_allocation_time`）。
pub fn allocate(
    tx: &Tx<'_>,
    parent: BudgetOwner,
    child: BudgetOwner,
    dims: Dimensions,
    at: i64,
) -> Result<Allocation, BudgetError> {
    reject_negative(&dims)?;
    let parent_node = persist::load_node(tx, &parent)?.ok_or_else(|| BudgetError::UnknownOwner {
        owner: parent.clone(),
    })?;
    let parent_available = persist::available_of(tx, &parent, &parent_node)?;
    let children = persist::children_sums(tx, &parent)?;

    let values = dims.to_values();
    for (i, dimension) in ALL.iter().enumerate() {
        let Some(amount) = values[i] else {
            continue;
        };
        if let Some(available) = parent_available[i] {
            if children[i] + amount > available {
                return Err(BudgetError::Insufficient {
                    dimension: *dimension,
                });
            }
        }
    }

    persist::insert_node(tx, &child, Some(&parent), &dims)?;
    persist::append_ledger(tx, &child, LedgerKind::Allocate, &dims, at)?;
    Ok(Allocation(dims))
}

/// **I1 的判定点**：预扣。`available(n, d) ≥ 0` 不满足则拒（fail-closed），
/// **返回具体哪一维不足**（不是笼统的 `Exceeds`）。
///
/// # 判定的口径（写死）
///
/// 与 [`allocate`] 同型的逐维口径：对本次 `dims` 里 `Some` 的维，`available` 是
/// `Some` 时要求 `预扣 ≤ 可用`；`available` 为 `None`（不构成约束）时放行。
///
/// # 失败路径
///
/// 负值 / owner 不在树上 / 越界都在**任何库写入之前**返回——成功的预扣写账本
/// `reserve` 行（每 `Some` 维一条）后才交出 [`Reservation`]。
pub fn reserve(
    tx: &Tx<'_>,
    owner: BudgetOwner,
    dims: Dimensions,
    at: i64,
) -> Result<Reservation, BudgetError> {
    reject_negative(&dims)?;
    let node = persist::load_node(tx, &owner)?.ok_or_else(|| BudgetError::UnknownOwner {
        owner: owner.clone(),
    })?;
    let available = persist::available_of(tx, &owner, &node)?;

    let values = dims.to_values();
    for (i, dimension) in ALL.iter().enumerate() {
        let Some(amount) = values[i] else {
            continue;
        };
        if let Some(remaining) = available[i] {
            if amount > remaining {
                return Err(BudgetError::Insufficient {
                    dimension: *dimension,
                });
            }
        }
    }

    persist::append_ledger(tx, &owner, LedgerKind::Reserve, &dims, at)?;
    Ok(Reservation { owner, dims })
}

/// 结算：把一次预扣转成实际用量，**退回差额**（`Within`）或**报哪一维超多少**
/// （`Over`）。**不 `commit`**。
///
/// # 口径（写死）
///
/// - `actual` 的某一维 `None` = **该维没有读数** ⇒ 按预扣结算该维、不退余量
///   （§12.3 的规则：无读数没有「实际 ≤ 预扣」的依据；按预扣结算永不低记）。
/// - `refunded` 与本次预扣同定义域：预扣没取的维为 `None`；无读数的维退 0。
/// - 负 `actual` ⇒ [`BudgetError::NegativeAmount`]——**拒收，不截断**（负数是
///   fail-open 的入口：`settled` 会因此变小、`available` 反而变大，而 I1 只约束
///   `available`、不约束 `actual` 的符号）。
/// - `Over` **不写第二行账**：结算照写它的 `settle` 行（每预扣 `Some` 维一条）、
///   只回答「超了多少、哪一维」——运行期超支的裁决不在本层（ENG-005 §三.2）。
/// - 一次结算里多维多超时：报**字段序第一处**（设计未给选择规则，见
///   [`SettleOutcome::Over`] 的文档）。
///
/// # 失败路径
///
/// 负值 / owner 不在树上都在**任何库写入之前**返回（照片：`tests/tree.rs` 的
/// `a_negative_actual_is_rejected_and_the_ledger_is_untouched`——失败路径不但断言
/// 哪一种 `Err`，还断言**没有半写的副作用**）。
pub fn settle(
    tx: &Tx<'_>,
    reservation: Reservation,
    actual: Dimensions,
    at: i64,
) -> Result<SettleOutcome, BudgetError> {
    reject_negative(&actual)?;
    let owner = reservation.owner;
    persist::load_node(tx, &owner)?.ok_or_else(|| BudgetError::UnknownOwner {
        owner: owner.clone(),
    })?;

    let reserved = reservation.dims.to_values();
    let measured = actual.to_values();
    let mut effective = [None; 5];
    let mut refunded = [None; 5];
    let mut over: Option<(DimensionKind, i64)> = None;

    for (i, dimension) in ALL.iter().enumerate() {
        let Some(reserved_amount) = reserved[i] else {
            continue;
        };
        // 无读数（`None`）⇒ 按预扣结算该维（§12.3）；有读数就用读数。
        let used = measured[i].unwrap_or(reserved_amount);
        if let Some(measured_amount) = measured[i] {
            if measured_amount > reserved_amount && over.is_none() {
                over = Some((*dimension, measured_amount - reserved_amount));
            }
        }
        refunded[i] = Some(reserved_amount - used);
        effective[i] = Some(used);
    }

    persist::append_ledger(
        tx,
        &owner,
        LedgerKind::Settle,
        &Dimensions::from_values(effective),
        at,
    )?;

    match over {
        Some((dimension, by)) => Ok(SettleOutcome::Over { dimension, by }),
        None => Ok(SettleOutcome::Within {
            refunded: Dimensions::from_values(refunded),
        }),
    }
}

/// **`Remaining` 的现算**：`allocation − settled − reserved`（逐维）——不是落库
/// 的列（账本只追加、余额是派生值）。
///
/// # `None` 传染
///
/// 分配行该维为 `None`（不构成约束）时余量也 `None`——同 [`crate::Dimensions`]
/// 的 `None` 语义（该维不构成约束），两者**都不被折成 `Some(0)`**。
///
/// # 失败路径
///
/// owner 不在树上 ⇒ [`BudgetError::UnknownOwner`]（fail-closed——对一棵不存在的
/// 子树不给任何余量读数）。
pub fn remaining(tx: &Tx<'_>, owner: &BudgetOwner) -> Result<Remaining, BudgetError> {
    let node = persist::load_node(tx, owner)?.ok_or_else(|| BudgetError::UnknownOwner {
        owner: owner.clone(),
    })?;
    let available = persist::available_of(tx, owner, &node)?;
    Ok(Remaining(Dimensions::from_values(available)))
}
