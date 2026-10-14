//! 预算账本：行的词汇（[`LedgerKind`]）、余额的现算（`balances`）与悬空预扣的
//! 检出（[`dangling_reservations`]）。
//!
//! # 账本表（设计 §12.2）
//!
//! `budget_ledger(owner, dimension, kind, amount, at)`，**只追加、不 UPDATE**——
//! 余额是派生值。这样恢复时可重放，且「半写的副作用」在账本上表现为「有一条
//! `reserve` 而没有对应的 `settle` 或 `release`」，**是可检出的**。
//!
//! # 行语义与「配对」（本 task 自定的一处形状，申报）
//!
//! 设计**没给**每一行 `amount` 的语义，也没给「一条 `reserve` 怎样算『有对应的
//! `settle` 或 `release`』」的配对规则（账本五列写死，**没有 `reservation_id`**）。
//! 本实现取（理由与限度逐条写在 `balances` 与 `dangling_reservations` 的文档里）：
//!
//! - **`amount` 是名义量**：`allocate` 行 = 该次分给该维的额度；`reserve` 行 =
//!   该次预扣额；`settle` 行 = 该次结算的**实际用量**（无读数的维写预扣额，
//!   §12.3 按预扣结算）；`release` 行 = 释放掉的预扣额。
//! - **配对按行数、FIFO**：每 `(owner, dimension)` 上，与「`settle` ＋ `release`
//!   行的条数」等量的**最早** `reserve` 行视为已消化；其余 `reserve` 行的额之和
//!   就是 `reserved`（在飞的预扣）。

use continuum_persist::Tx;

use crate::dimensions::{ALL, DimensionKind, rank};
use crate::error::BudgetError;
use crate::persist::{self, load_ledger_rows};
use crate::tree::BudgetOwner;

/// 账本行的四种（设计 §12.2 写死的值域）。
///
/// # `Release` **今天没有产生方**（据实交代，不留位）
///
/// 本层没有释放预扣的入口——§12.3 判「释放余量是 fail-open」：无读数的执行
/// **按预扣结算、不释放**（恢复钩子同此）。故今天没有任何路径写 `release` 行。
/// **但本枚不能删**：账本的四个 `kind` 是设计写死的值域；它的**读法**（配对消化）
/// 已就位——将来谁写 `release` 行（若有），读法不需要改。**照片**：
/// `tests/persist.rs` 的 `a_dangling_reservation_is_detectable` 的 (c) 半边
/// 用裸插一行模拟 `release` 的账本形态，钉配对规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerKind {
    Allocate,
    Reserve,
    Release,
    Settle,
}

impl LedgerKind {
    /// 落库编码：**小写加下划线**（本仓枚举列纪律）。**无通配臂**——加变体即编译不过。
    pub fn as_str(&self) -> &'static str {
        match self {
            LedgerKind::Allocate => "allocate",
            LedgerKind::Reserve => "reserve",
            LedgerKind::Release => "release",
            LedgerKind::Settle => "settle",
        }
    }

    /// 落库解码。**认不出的串返回 `None`**（调用点转具体 `Err`，**不取默认值**）。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allocate" => Some(LedgerKind::Allocate),
            "reserve" => Some(LedgerKind::Reserve),
            "release" => Some(LedgerKind::Release),
            "settle" => Some(LedgerKind::Settle),
            _ => None,
        }
    }
}

/// 一条**悬空**的预扣：某 `(owner, dimension)` 上有 `reserve` 行没有被对应的
/// `settle` / `release` 消化。`amount` 是**未消化的预扣额之和**。
///
/// 字段是 `pub` 的：它是一份读出的纯数据（没有需要靠私有性守的不变量），
/// 且 [`crate::settle`] 的消费者（Task 18 的恢复钩子）要按它「按预扣结算」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanglingReservation {
    /// 悬空预扣的主人。
    pub owner: BudgetOwner,
    /// 悬空发生在哪一维。
    pub dimension: DimensionKind,
    /// 未消化的预扣额之和（同 [`balances`] 的 `reserved` 口径）。
    pub amount: i64,
}

/// 一维上的账本行（已解码），按插入序。
type Row = (DimensionKind, LedgerKind, i64);

/// 从账本行算 `(settled, reserved)` 两个五维数组——**余额的现算**（账本行是
/// 唯一的真值，没有 `balance` 列）。
///
/// # 算法（写死）
///
/// - `settled[i]` = 该维全部 `settle` 行 `amount` 之和（**实际用量**的名义量，
///   含超支——Over 的结算照记实际数，超支的裁决不改变账本行的写法）。
/// - `reserved[i]`：把该维的 `reserve` 行按插入序收集为 `r₁..r_R`，
///   设 `k = min(settle 行条数 + release 行条数, R)`（**FIFO 配对**）——
///   `reserved[i] = r_{k+1} + … + r_R`（未消化的那些）。
/// - `allocate` 行不参与这两个量（分配额住在 `budget_node`，见
///   [`crate::remaining`] 的组合公式）。
///
/// # 限度（申报）
///
/// 账本没有 `reservation_id` 列（五列写死），配对付不出「哪条 `reserve` 配哪条
/// `settle`」的精确答案：**乱序结算并发预扣时，未消化金额可能错位**（「有没有
/// 悬空」的条数判断不受顺序影响）。恢复钩子场景（进程死掉后悬空的是最后一条
/// `reserve`）在 FIFO 下恰好正确——那是 `reserved` 最要紧的读法。
pub(crate) fn balances(rows: &[Row]) -> ([i64; 5], [i64; 5]) {
    let mut settled = [0i64; 5];
    let mut reserved = [0i64; 5];
    for (i, kind_of_dimension) in ALL.iter().enumerate() {
        let mut reserves: Vec<i64> = Vec::new();
        let mut digested = 0usize;
        for (dimension, kind, amount) in rows {
            if dimension != kind_of_dimension {
                continue;
            }
            match kind {
                LedgerKind::Reserve => reserves.push(*amount),
                LedgerKind::Settle => {
                    settled[i] += amount;
                    digested += 1;
                }
                LedgerKind::Release => digested += 1,
                LedgerKind::Allocate => {}
            }
        }
        let digested = digested.min(reserves.len());
        reserved[i] = reserves[digested..].iter().sum();
    }
    (settled, reserved)
}

/// §12.2 的「有一条 `reserve` 而没有对应的 `settle` 或 `release`」——可检出。
///
/// 返回**全部**悬空 `(owner, dimension, 未消化额)`，按 `(owner 键, 维度枚举序)`
/// 排序（顺序固定，调用方才不必自己排）。**一条都没有时返回空 `Vec`**（那是
/// 「没有悬空」，不是错误）。
///
/// # 本函数是设计 §12.3 第 2 处的输入
///
/// 恢复钩子（`RecoveryPhase::MarkLostExecutions`）**按预扣结算**悬空的预留，
/// 不释放——它的输入就是本函数（§12.3：「一次执行如果没有实际读数，就按预扣额
/// 结算；不退余量」）。
///
/// # 表外取值
///
/// `owner` / `dimension` / `kind` 三列任一读不回来时返回**具体 `Err`**、带原始串
/// （[`crate::BudgetError::Persist`] 的 `Database` 形状——理由见
/// [`crate::BudgetError`] 的申报段）。**不取默认值、不跳过那一行**。
pub fn dangling_reservations(tx: &Tx<'_>) -> Result<Vec<DanglingReservation>, BudgetError> {
    let rows = load_ledger_rows(tx)?;

    // 分组（保序）：每个 (owner, dimension) 一组——reserve 的额按序收集、
    // settle/release 的行数计数。
    struct Group {
        owner: BudgetOwner,
        dimension: DimensionKind,
        reserves: Vec<i64>,
        digested: usize,
    }
    let mut groups: Vec<Group> = Vec::new();

    for row in &rows {
        let owner = persist::decode_owner(&row.owner)?;
        let dimension = persist::decode_dimension(&row.dimension)?;
        let kind = persist::decode_kind(&row.kind)?;
        // 先按不可变借用找位置（借用在 `position` 处结束），再取可变引用——
        // 免得 `None` 分支里的 `push` 与 `iter_mut` 的借用冲突。
        let index = groups
            .iter()
            .position(|g| g.owner == owner && g.dimension == dimension);
        let group = match index {
            Some(i) => &mut groups[i],
            None => {
                groups.push(Group {
                    owner,
                    dimension,
                    reserves: Vec::new(),
                    digested: 0,
                });
                groups.last_mut().expect("刚 push 过")
            }
        };
        match kind {
            LedgerKind::Reserve => group.reserves.push(row.amount),
            LedgerKind::Settle | LedgerKind::Release => group.digested += 1,
            LedgerKind::Allocate => {}
        }
    }

    let mut dangling: Vec<DanglingReservation> = Vec::new();
    for group in groups {
        let digested = group.digested.min(group.reserves.len());
        let undigested = group.reserves.len() - digested;
        if undigested > 0 {
            // **判据用条数而不是金额**：未消化的条目可以全是 0 额
            // （额度为零的预扣是合法的）——「有没有悬空」是条数问题。
            let amount: i64 = group.reserves[digested..].iter().sum();
            dangling.push(DanglingReservation {
                owner: group.owner,
                dimension: group.dimension,
                amount,
            });
        }
    }

    dangling.sort_by_key(|d| (d.owner.as_key(), rank(d.dimension)));
    Ok(dangling)
}
