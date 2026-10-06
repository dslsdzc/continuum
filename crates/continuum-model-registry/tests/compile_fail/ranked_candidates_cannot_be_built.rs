//! `RankedExecutionCandidates` 的字段私有，**唯一构造点在 `rank` 内**（设计 §5.2）。
//!
//! 这条不可表达性命题撑起的是 `selected()` 的签名：`rank` 在构造之前**先判空**，
//! 无候选时返回 `Err(RoutingError::NoEligibleCandidate)` 而不返回空列表，故
//! `selected()` 不必返回 `Option`（§84 的 `selected_model` 总是存在）。
//!
//! 若这个结构体能在 crate 外构造，一个空列表就能被造出来，`selected()` 的
//! 「总有一个表头」随之失效——**这条保证与 `selected()` 不返回 `Option` 是同一件事的两面**。
//!
//! 判据是编译失败（E0451：字段私有），配同名 `.stderr` 把失败钉在预期的报错上——
//! 否则「因为拼错类型名而编译失败」也会让用例变绿。

use continuum_model_registry::RankedExecutionCandidates;

fn main() {
    let _ = RankedExecutionCandidates {
        candidates: Vec::new(),
    };
}
