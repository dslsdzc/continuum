//! 迁移 **120**（`budget_node` / `budget_ledger`，设计 §2.4 的 120 段）与两张表的
//! 行级读写原语。
//!
//! # 迁移 120 的来历（本 task 现场核对）
//!
//! 编号 **120**：1、2 是 `continuum-persist` 的内建（events、audit_log），10 是 P1 的
//! artifact，20 是 P1 的 graph，30 是 P2 的 workspace，40 是 P2 的 effect，41 是 P2 的
//! policy，50 是 P3 的 capability，80 是 P3D 的模型表，100 是 P4 Task 7 的 alias，
//! 110 是 P4 Task 8–15 的语义层七表。**120 在驱动装配的那个集合里未被占用**
//! （本 task 现场核对：`grep -rn "Migration::new(" crates/` 除本文件那一处外命中的
//! version 为 1 / 2 / 10 / 20 / 30 / 40 / 41 / 50 / 80 / 100 / 110，**另加
//! `continuum-persist/src/bin/crash-writer.rs` 的 60 与 `tests/migrations.rs` 的
//! 100 / 101 / 102**——后几处活在各自的测试/工具 `Db` 里，不进装配链）。
//! **「未被占用」是按库说的，不是全仓**（同 `continuum-canonical/src/persist.rs`
//! 的 `p4_canonical_migrations` 那段）：它们各自的库与本 crate 的库**永不同库**。
//!
//! **迁移 120 的登记（进驱动装配）归 Task 20**：本 task 只交出
//! [`p4_budget_migrations`] 与 `ALLOWED` 的那两条边。
//!
//! # 两张表的形状
//!
//! - `budget_ledger` 的五列 `(owner, dimension, kind, amount, at)` 是**设计 §12.2
//!   写死**的。`dimension` / `kind` 两列是**封闭枚举列**（编码 / 解码与类型同址：
//!   [`crate::DimensionKind`] / [`crate::LedgerKind`]），表外取值读回**具体 `Err`**、
//!   带原始串、不取默认值（落点：本文件的三个 `decode_*`）。
//!   **行语义（本 task 自定，申报）**：`amount` 是**名义量**——`allocate` 行 = 该次
//!   分给该维的额度、`reserve` 行 = 该次预扣额、`settle` 行 = 该次结算的实际用量
//!   （无读数的维写预扣额，§12.3 按预扣结算）；`reserved`（在飞的预扣）由行数配对
//!   得出（见 [`crate::ledger::balances`] 的算法与限度申报）。**只追加、不 UPDATE**
//!   ——余额是派生值（[`crate::remaining`] 现算），表里没有 `balance` 一列。
//! - `budget_node` 的列设计**没给**——本实现取：`owner`（主键，复合键的编码，见
//!   [`crate::BudgetOwner::as_key`]）＋ `parent`（父的编码；`NULL` = 根）＋ 五维
//!   （`NULL` = `None`）。`parent` 列是 I2 的判据来源：`Σ 子节点分配额` 就是
//!   「`parent` 列等于该节点的那些行」的和。
//! - 两张表都**不建外键**（同 `continuum-semantics` 的处置：跨记录对应关系由写路径
//!   的调用方保证，本层不在库里登记引用约束）。
//!
//! # 写函数不自己 `commit`
//!
//! 本文件与 [`crate::tree`] 的写路径都不收口事务——事务由调用方持有并提交
//! （「写函数不自己 commit」；§318 要求状态更新与对应事件在同一事务内提交，
//! 这里同理：任一步失败，调用方必须回滚，不得提交）。

use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};

use crate::dimensions::{ALL, DimensionKind, Dimensions};
use crate::ledger::{LedgerKind, balances};
use crate::tree::BudgetOwner;

/// 本 crate 在两张表上注册的迁移（设计 §2.4 的 120 段：`budget_node` / `budget_ledger`）。
///
/// # 120 是**一个**迁移项
///
/// **不要往返回向量里加第二个 `version = 120` 的项**——`Db::migrate()` 以
/// `schema_migrations.version`（**主键**）判「已应用」，同号的第二个迁移项会被
/// **静默跳过**（第二个表永不建）。照片：`tests/persist.rs` 的
/// `the_migration_120_is_a_single_item_with_both_tables`（它断言 `len() == 1`，
/// 并逐表断言 SQL 里两张表都在——那条 `len() == 1` 正是这件事的守卫）。
pub fn p4_budget_migrations() -> Vec<Migration> {
    vec![Migration::new(
        120,
        "p4_budget",
        "CREATE TABLE budget_node (
            owner            TEXT PRIMARY KEY NOT NULL,
            parent           TEXT,
            money            INTEGER,
            wall_time        INTEGER,
            token            INTEGER,
            gpu_time         INTEGER,
            network_transfer INTEGER
        );
        CREATE TABLE budget_ledger (
            owner     TEXT NOT NULL,
            dimension TEXT NOT NULL,
            kind      TEXT NOT NULL,
            amount    INTEGER NOT NULL,
            at        INTEGER NOT NULL
        );",
    )]
}

/// 一行 `budget_node`（读回侧）——今天只读五维（`owner` 是查询参数、`parent` 供
/// 「子树求和」的查询自用：`WHERE parent = ?`，故本结构不携带它们）。
pub(crate) struct NodeRow {
    /// 五个量纲的分配额（行上的原值；`None` = 该维不构成约束）。
    pub(crate) dims: [Option<i64>; 5],
}

/// 按 `owner` 读一行 `budget_node`。**不存在返回 `Ok(None)`**（调用点转
/// [`crate::BudgetError::UnknownOwner`]——那不是「没有」，是「树上查不到」）。
pub(crate) fn load_node(tx: &Tx<'_>, owner: &BudgetOwner) -> Result<Option<NodeRow>, PersistError> {
    let rows = tx.query(
        "SELECT money, wall_time, token, gpu_time, network_transfer \
         FROM budget_node WHERE owner = ?1",
        &[Value::text(owner.as_key())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let mut dims = [None; 5];
    for (i, slot) in dims.iter_mut().enumerate() {
        *slot = opt_int_at(&row, i)?;
    }
    Ok(Some(NodeRow { dims }))
}

/// 写一行 `budget_node`。**裸 `INSERT`**（不是 `INSERT OR REPLACE`）：同一个
/// `owner` 的第二次写入由**主键**拒绝（fail-closed——重复的根／重复的分配都撞键
/// 得 `Err`，不静默覆写）。
pub(crate) fn insert_node(
    tx: &Tx<'_>,
    owner: &BudgetOwner,
    parent: Option<&BudgetOwner>,
    dims: &Dimensions,
) -> Result<(), PersistError> {
    let values = dims.to_values();
    tx.execute(
        "INSERT INTO budget_node (owner, parent, money, wall_time, token, gpu_time, network_transfer) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        &[
            Value::text(owner.as_key()),
            match parent {
                Some(p) => Value::text(p.as_key()),
                None => Value::Null,
            },
            opt_int(values[0]),
            opt_int(values[1]),
            opt_int(values[2]),
            opt_int(values[3]),
            opt_int(values[4]),
        ],
    )?;
    Ok(())
}

/// 写账本行：`dims` 里**每一维 `Some` 的地方各写一行**（`None` 的维不涉及本次
/// 操作——「不构成约束」）。
///
/// 账本**只追加**：本函数只有 `INSERT`，没有 `UPDATE` 与 `DELETE`。
pub(crate) fn append_ledger(
    tx: &Tx<'_>,
    owner: &BudgetOwner,
    kind: LedgerKind,
    dims: &Dimensions,
    at: i64,
) -> Result<(), PersistError> {
    let values = dims.to_values();
    for (i, dimension) in ALL.iter().enumerate() {
        let Some(amount) = values[i] else {
            continue;
        };
        tx.execute(
            "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                Value::text(owner.as_key()),
                Value::text(dimension.as_str()),
                Value::text(kind.as_str()),
                Value::Int(amount),
                Value::Int(at),
            ],
        )?;
    }
    Ok(())
}

/// 该 owner 的全部账本行（**已解码**、按插入序）——`balances` 的输入。
pub(crate) fn load_owner_ledger(
    tx: &Tx<'_>,
    owner: &BudgetOwner,
) -> Result<Vec<(DimensionKind, LedgerKind, i64)>, PersistError> {
    tx.query(
        "SELECT dimension, kind, amount FROM budget_ledger WHERE owner = ?1 ORDER BY rowid",
        &[Value::text(owner.as_key())],
    )?
    .iter()
    .map(|row| {
        Ok((
            decode_dimension(&text_at(row, 0)?)?,
            decode_kind(&text_at(row, 1)?)?,
            int_at(row, 2)?,
        ))
    })
    .collect()
}

/// 该 owner 的**可用额**逐维：`allocation − settled − reserved`。
///
/// **`None` 传染**：行的该维是 `None`（不构成约束）时，可用额也 **`None`**——
/// 「无限减多少还是无限」的最保守读法，且它与 `Remaining` 的 `None` 语义
/// （该维不构成约束）自洽（照片：`tests/tree.rs` 的 `dimension_none_is_not_some_zero`）。
pub(crate) fn available_of(
    tx: &Tx<'_>,
    owner: &BudgetOwner,
    node: &NodeRow,
) -> Result<[Option<i64>; 5], PersistError> {
    let ledger = load_owner_ledger(tx, owner)?;
    let (settled, reserved) = balances(&ledger);
    let mut available = [None; 5];
    for (i, slot) in available.iter_mut().enumerate() {
        *slot = node.dims[i].map(|allocation| allocation - settled[i] - reserved[i]);
    }
    Ok(available)
}

/// 该 parent 的**全部子节点**在五维上的分配额之和（`parent` 列等于它的行）。
///
/// **子行该维 `None` 贡献 0**（该维上这个孩子不构成约束）；返回 `[i64; 5]`，
/// 每维都是从 0 起的和（哪怕一个孩子都没有）。
pub(crate) fn children_sums(tx: &Tx<'_>, parent: &BudgetOwner) -> Result<[i64; 5], PersistError> {
    let rows = tx.query(
        "SELECT money, wall_time, token, gpu_time, network_transfer \
         FROM budget_node WHERE parent = ?1",
        &[Value::text(parent.as_key())],
    )?;
    let mut sums = [0i64; 5];
    for row in &rows {
        for (i, sum) in sums.iter_mut().enumerate() {
            if let Some(v) = opt_int_at(row, i)? {
                *sum += v;
            }
        }
    }
    Ok(sums)
}

/// 账本**全部**行（**原始串**，按插入序）——`undigested_reserve_rows` 的输入：
/// 它要读全表（跨 owner），且解码失败要报**具体是哪一列、哪个原始串**。
/// `at` 一并读出：恢复钩子按预扣结算时，`settle` 行用**被消化那条 `reserve` 行
/// 自己的 `at`**（恢复钩子不收时钟——本层不取时钟，同 effect 的
/// `MarkExecutingAsUnknown` 对「记录既有时刻」的处置）。
pub(crate) struct RawLedgerRow {
    pub(crate) owner: String,
    pub(crate) dimension: String,
    pub(crate) kind: String,
    pub(crate) amount: i64,
    pub(crate) at: i64,
}

/// 读全表账本行。
pub(crate) fn load_ledger_rows(tx: &Tx<'_>) -> Result<Vec<RawLedgerRow>, PersistError> {
    tx.query(
        "SELECT owner, dimension, kind, amount, at FROM budget_ledger ORDER BY rowid",
        &[],
    )?
    .iter()
    .map(|row| {
        Ok(RawLedgerRow {
            owner: text_at(row, 0)?,
            dimension: text_at(row, 1)?,
            kind: text_at(row, 2)?,
            amount: int_at(row, 3)?,
            at: int_at(row, 4)?,
        })
    })
    .collect()
}

/// 解码 `budget_ledger.owner` 列（复合键）。**表外/坏格式读回具体 `Err`**、
/// 带原始串——**不取默认值、不猜**（枚举/复合键列的读回侧；`PersistError` 的
/// `Database` 形状是本 crate 在「`BudgetError` 三枚语义枚之外」的落点，
/// 理由见 [`crate::BudgetError`] 的申报段）。
pub(crate) fn decode_owner(raw: &str) -> Result<BudgetOwner, PersistError> {
    BudgetOwner::parse(raw)
        .ok_or_else(|| PersistError::Database(format!("表外的 budget_ledger.owner 取值: {raw}")))
}

/// 解码 `budget_ledger.dimension` 列（同 [`decode_owner`] 的处置）。
pub(crate) fn decode_dimension(raw: &str) -> Result<DimensionKind, PersistError> {
    DimensionKind::parse(raw)
        .ok_or_else(|| PersistError::Database(format!("表外的 budget_ledger.dimension 取值: {raw}")))
}

/// 解码 `budget_ledger.kind` 列（同 [`decode_owner`] 的处置）。
pub(crate) fn decode_kind(raw: &str) -> Result<LedgerKind, PersistError> {
    LedgerKind::parse(raw)
        .ok_or_else(|| PersistError::Database(format!("表外的 budget_ledger.kind 取值: {raw}")))
}

/// `Option<i64>` → 可绑定的列值（`None` 落 `NULL`）。
fn opt_int(value: Option<i64>) -> Value {
    match value {
        Some(v) => Value::Int(v),
        None => Value::Null,
    }
}

/// 按 `index` 取一列**可空整数**。`NULL` → `None`（同 `continuum-semantics` 的
/// `opt_text_at` 的处置：`NULL` 与「表外取值」是两件事——前者是「这一列本来就
/// 可以为空」，后者是「这一列的值不合法」；本函数只处理前者）。
fn opt_int_at(row: &[Value], index: usize) -> Result<Option<i64>, PersistError> {
    match row.get(index) {
        Some(Value::Null) => Ok(None),
        Some(Value::Int(i)) => Ok(Some(*i)),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// 按 `index` 取一列文本。**`PersistError::ColumnType` 只在本函数里构造一处**
/// （形态与 `continuum-canonical/src/persist.rs` 的 `text_at` 同）。
fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    match row.get(index) {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// 按 `index` 取一列整数。
fn int_at(row: &[Value], index: usize) -> Result<i64, PersistError> {
    match row.get(index) {
        Some(Value::Int(i)) => Ok(*i),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}
