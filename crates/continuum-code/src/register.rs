//! §3.5 的注册期规则（先核后写）与它读的「逐位可复现的 backend」封闭名单。

use continuum_operator::{Determinism, Operator, OperatorRegistry};

use crate::error::CodeError;

/// §3.5 的「逐位可复现的 backend」封闭名单。**内容是设计 §3.5 定的，规范没有一处给 backend 的确定性。**
/// 名单里为什么有 `overlay`：§255 给非 Git 项目列的五种形态（`OverlayFS` / `Btrfs snapshot` /
/// `ZFS clone` / `CoW workspace` / `container layer`），而 P2 已落地的
/// `WorkspaceBackend`（`crates/continuum-workspace/src/backend.rs:22`）只有两臂；本块取那两臂，
/// 不按 §255 的五种各造一个名字——那会让候选里出现三个没有实现的取值。
pub const REPRODUCIBLE_BACKENDS: [&str; 5] = [
    "worktree",
    "overlay",
    "builtin",
    "cargo-test",
    "local-test-runner",
];

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核 §3.5 的声明，全部通过后再逐枚注册；任一枚不成即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚代码域算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§3.5 那条规则要能对**任意** `Operator` 触发，
/// 否则 §10.2 第 (d) 条写不出来。
///
/// **这条规则拦不住哪一类**（否则会被当成一条全覆盖的守卫）：它只查「声明 `Deterministic`
/// 的算子，其候选是否逐位可复现」，**不查「这个算子的真实输入是否在端口上」**。
/// `code-workspace` 正是后者：它的两个候选（`worktree` / `overlay`）都在名单内，故把它
/// 声明成 `Deterministic` **能通过**本规则，而 §10.2 第 (g) 条（`tests/operators.rs` 的
/// `the_determinism_of_every_row`）判它错。**两者是两条独立的守卫，缺任一条都留下一类
/// 静默错误**：前者抓「候选不可复现」，后者抓「输入不可见」（设计 §3.5 末）。
pub fn register_code_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), CodeError> {
    // 先核：逐枚、逐候选查 §3.5 的封闭名单；遇到第一枚不成即返回——此处起注册表一字未动。
    for operator in operators {
        if operator.determinism != Determinism::Deterministic {
            continue;
        }
        for candidate in &operator.backend_candidates {
            if !REPRODUCIBLE_BACKENDS.contains(&candidate.as_str()) {
                return Err(CodeError::UnverifiableDeterminism {
                    operator: operator.id.clone(),
                    backend: candidate.clone(),
                });
            }
        }
    }

    // 后写：全部核过才逐枚注册。同 (id, version) 撞车由 P1 的 `OperatorError::Duplicate` 判定，
    // 经这个 `?` 转进 `CodeError::Registry`（`tests/register.rs` 的 (b) 条的唯一锚点）。
    for operator in operators {
        registry.register(operator.clone())?;
    }

    Ok(())
}
