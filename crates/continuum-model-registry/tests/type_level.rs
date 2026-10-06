//! 类型层只读保证的唯一直接证据（设计 §2.1「唯一产生点」、§2.3 第 3 条 (a)、§4.4 完成判据）。
//!
//! 「画像在 crate 外构造不出来」「读不到总分」都是**不可表达性**命题：运行期用例无论怎么写
//! 都只能证明「我没这么构造过 / 我没这么读过」，证明不了「做不到」。故用 `trybuild`
//! 的编译失败用例。每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在
//! 预期的报错上——否则「因为拼错函数名而编译失败」也会让用例变绿。
//!
//! **一份样例只钉一条通道，不合并**——本目录现有**九份**，下面**逐份点名**（便于对数：
//! 这份清单的项数必须等于目录里 `.rs` 的份数）：
//!
//! - **E0451 字段私有**（**四份**）：`model_profile_cannot_be_built`（`ModelProfile` 字面量）、
//!   `a_requirement_cannot_be_built_from_a_bare_vec`（`TaskSkillRequirement` 字面量）、
//!   `routable_model_fields_are_private`（`RoutableModel` 字面量）、
//!   `ranked_candidates_cannot_be_built`（排序输出的字面量）；
//! - **E0624 非 `pub`**（一份）：`model_profile_has_no_constructor`（`ModelProfile::try_new`）；
//! - **E0599 没有这个方法 / 变体**（**两份**）：`overall_score_cannot_be_read`（总分）、
//!   `tier_one_low_is_not_a_step`（Task 13：`EscalationStep::Tier1Low` 写不出来）；
//! - **E0277 trait 未实现**（一份）：`a_requirement_cannot_be_built_by_conversion`（`From<Vec<_>>`）；
//! - **E0063 缺字段**（一份）：`a_routing_request_without_a_budget`（`RoutingRequest.budget`）。
//!
//! 合并成一份会让「改掉其中一条通道」只在 `.stderr` 的整体比对里漂移，读不出红在哪条上；
//! 且实测（rustc 1.95）同一函数体里 rustc **只报第一条错**，合并会让没被报出来的那条**没有照片**。
//!
//! **本目录由下面 `type_level_guarantees_hold` 里的 `tests/compile_fail/*.rs` 通配收走**：
//! 新增一份样例**不需要改本文件**，但**要重跑本用例**来接收它生成的 `.stderr`
//! ——首次运行会落 `wip/*.stderr`，把它搬进 `tests/compile_fail/` 才算接受。

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
