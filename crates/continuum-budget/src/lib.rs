//! `continuum-budget`：P4 语义层的预算半边（设计 §12）——预算树、三个角色
//! （[`Allocation`] / [`Reservation`] / [`Remaining`]）、I1／I2／I3、账本、
//! Budget Validator 与探索预算。
//! 规划 §2.1：「预算树、`Allocation` / `Reservation` / `Remaining`、预算账本、
//! Budget Validator、探索预算」（§333 §334 §110；ENG-005）。
//!
//! **Task 17 落了前四件**（树与三角色、三条不变量、账本）；
//! **Task 18 落**：无读数即按预扣（一条规则、两处应用——[`settle_without_a_reading`]
//! 与 [`BudgetReservationRecovery`]）、[`check`]（§110）、[`check_exploration`]（§334）。
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
//! - `src/ledger.rs`：[`LedgerKind`]、余额的现算、[`dangling_reservations`] 与
//!   逐条的未消化读法（crate 内——钩子的输入）。
//! - `src/persist.rs`：[`p4_budget_migrations`]（迁移 **120**）与两张表的行级
//!   读写原语（crate 内）。
//! - `src/error.rs`：[`BudgetError`]（§12.2.1 的三枚语义失败 ＋ 一枚 `Persist`
//!   转出——申报见那个类型）。
//! - `src/settle_policy.rs`（Task 18）：§12.3 的**第一处**——
//!   [`settle_without_a_reading`]（流式调用没有读数）。
//! - `src/recovery.rs`（Task 18）：§12.3 的**第二处**——
//!   [`BudgetReservationRecovery`]（悬空预留，挂 `MarkLostExecutions`）。
//! - `src/validator.rs`（Task 18）：§110 的 [`BudgetVerdict`] 与 [`check`]。
//! - `src/exploration.rs`（Task 18）：§334 的 [`ExplorationThresholds`] /
//!   [`ExplorationVerdict`] 与 [`check_exploration`]。
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
//! # 吃 `Dimensions` 的入口 × 负值（§12.2.1）
//!
//! `allocate` / `reserve` / `settle` / `check` **在同一口径下**：
//! 含负值的 `Dimensions` 一律拒收（[`BudgetError::NegativeAmount`]）、**不做豁免**。
//! 四个入口各有一对用例（拒收侧 ＋ 全零反例侧；`check` 那对在
//! `tests/validator.rs`）；同刻落地的根入账函数 [`record_root`] 也在同一口径下。
//! **[`check_exploration`] 不在 §12.2.1 的四行表里**——它的负值处置有一处申报
//! （见 `src/exploration.rs` 的申报 2）。
//!
//! # 本层不取时钟
//!
//! 所有写入口的 `at` 由**调用方**给出（账本 `at` 列）——同 `Intent.created_at`
//! 的纪律；`src/` 里没有 `SystemTime::now` 一类（Task 21 的源码面复核对这一条
//! 与「对下层的零依赖」有第二侧核查）。恢复钩子（[`BudgetReservationRecovery`]）
//! 不收时钟：它写的 `settle` 行沿用被消化那条 `reserve` 行自己的 `at`。

mod account;
mod dimensions;
mod error;
mod exploration;
mod ledger;
mod persist;
mod recovery;
mod settle_policy;
mod tree;
mod validator;

pub use account::{Allocation, Remaining, Reservation};
pub use dimensions::{DimensionKind, Dimensions};
pub use error::BudgetError;
pub use exploration::{ExplorationThresholds, ExplorationVerdict, check_exploration};
pub use ledger::{DanglingReservation, LedgerKind, dangling_reservations};
pub use persist::p4_budget_migrations;
pub use recovery::BudgetReservationRecovery;
pub use settle_policy::settle_without_a_reading;
pub use tree::{BudgetOwner, SettleOutcome, allocate, record_root, remaining, reserve, settle};
pub use validator::{BudgetVerdict, check};
