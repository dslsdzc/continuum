//! `continuum-budget`：P4 语义层的预算半边（设计 §12）——预算树、三个角色
//! （[`Allocation`] / [`Reservation`] / [`Remaining`]）、I1／I2／I3 与账本。
//! 规划 §2.1：「预算树、`Allocation` / `Reservation` / `Remaining`、预算账本、
//! Budget Validator、探索预算」（§333 §334 §110；ENG-005）。
//!
//! **本 task（Task 17）落前四件**：树与三角色、三条不变量、账本；
//! **Budget Validator（`check`）与探索预算归 Task 18**。
//!
//! # 本 crate 的边界（设计 §2.1／§2.2）
//!
//! 只登记两条 workspace 内的边：`← continuum-semantics`（owner 要 `IntentId`）
//! 与 `← continuum-persist`（树与账本经 `Tx`）。**没有指向执行层／资源层／
//! 边界层的边**；**反向也没有边**（`continuum-semantics` 的允许集里没有本 crate
//! ——§11.5 的两侧守卫之一，`crates/continuum-runtime/tests/dependency_direction.rs`
//! 的 `ALLOWED` 逐对断言）。`serde_json` 是**外部** crate（owner 复合键的编码；
//! 计划 Global Constraints 的「外部 crate 按需加，不进 `ALLOWED`」）。
//!
//! # 模块分工
//!
//! - [`dimensions`]（`src/dimensions.rs`）：§333 的五维名字（`DimensionKind`）与
//!   纯数据载体（`Dimensions`）。
//! - [`account`]（`src/account.rs`）：三个角色三个类型（内部字段私有、无公开
//!   构造函数）。
//! - `src/tree.rs`：[`BudgetOwner`] 与四个写入口 / 一个读入口
//!   （`record_root` / `allocate` / `reserve` / `settle` / `remaining`）。
//! - `src/ledger.rs`：[`LedgerKind`]、余额的现算与 [`dangling_reservations`]。
//! - `src/persist.rs`：[`p4_budget_migrations`]（迁移 **120**）与两张表的行级
//!   读写原语（crate 内）。
//! - `src/error.rs`：[`BudgetError`]（§12.2.1 的三枚语义失败 ＋ 一枚 `Persist`
//!   转出——申报见那个类型）。
//!
//! # 三条不变量（§12.2）
//!
//! ```text
//! available(n, d)  =  allocation(n, d) − settled(n, d) − reserved(n, d)
//! I1: available(n, d) ≥ 0                     （reserve 不满足则拒，fail-closed）
//! I2: Σ_{c ∈ children(n)} allocation(c, d) ≤ available(n, d)
//! I3: settled(n, d) + reserved(n, d) ≤ allocation(n, d)
//! ```
//!
//! I2 的强制点是 [`allocate`]（超分配在分配那一刻就被拒）；I1 的判定点是
//! [`reserve`]；I3 由两者共用同一个不等式（`available ≥ 0` 的代换）。
//!
//! # 四个吃 `Dimensions` 的入口 × 负值（§12.2.1）
//!
//! `allocate` / `reserve` / `settle` / `check`（Task 18）**在同一口径下**：
//! 含负值的 `Dimensions` 一律拒收（[`BudgetError::NegativeAmount`]）、**不做豁免**。
//! 本 task 落地的三个各有一对用例（拒收侧 ＋ 全零反例侧），`check` 那对归 Task 18。
//!
//! # 本层不取时钟
//!
//! 所有写入口的 `at` 由**调用方**给出（账本 `at` 列）——同 `Intent.created_at`
//! 的纪律；`src/` 里没有 `SystemTime::now` 一类（Task 21 的源码面复核对这一条
//! 与「对下层的零依赖」有第二侧核查）。

mod account;
mod dimensions;
mod error;
mod ledger;
mod persist;
mod tree;

pub use account::{Allocation, Remaining, Reservation};
pub use dimensions::{DimensionKind, Dimensions};
pub use error::BudgetError;
pub use ledger::{DanglingReservation, LedgerKind, dangling_reservations};
pub use persist::p4_budget_migrations;
pub use tree::{BudgetOwner, SettleOutcome, allocate, record_root, remaining, reserve, settle};
