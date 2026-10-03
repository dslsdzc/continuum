//! 裁决（设计下篇第 5.3 节）。

use crate::context::PolicyContext;
use crate::rule::{Decision, Level, Policy};

/// 六级裁决。
///
/// 取所有条件成立的规则中最高层；同层内取更严的一条（`Deny > RequireApproval >
/// Allow`）；**无任何规则匹配时返回 `Deny`**。层级高低按 [`Level`] 的判别值：
/// 数字即顺序，越大越弱，故「最高层」是判别值最小的那一级。
///
/// 本函数是纯函数：只读入参，不改入参，对同一对入参恒给出同一结果——规则集的
/// 排列次序不影响结果。
///
/// **前置条件：涉及 `--approve` 的那条规则由调用方放进 `policies`。** 第 2 级
/// `ExplicitCurrent` 的 `Allow` 在本层没有内建来源（设计下篇第 5.2 节：第 2 级在
/// 下篇由驱动的显式确认占位），故本函数不自行合成它，也不对规则集做任何增删。
/// 一条不带它的规则集的裁决结果里不会有 `--approve` 的效果——这是刻意的：
/// 「第 1 级不可越」那条性质需要一个不被 `--approve` 干扰的落点。
///
/// 调用方给出的那条第 2 级规则**必须带条件**（`explicit_current` 成立才匹配）。
/// 一条无条件的第 2 级 `Allow` 会在 `--approve` 未给出时照样获胜，把第 3–5 级的
/// `Deny` 一律推翻，即 fail-open（设计下篇第 5.5 节的取舍只在 `--approve` 给出时
/// 成立）。
///
/// 本函数**不**做「`--approve` 越过第 3–5 级」那一步的映射（设计下篇第 5.7 节的
/// 表）。**那张表不能逐行套到本函数的返回值上。** 本接法把第 2 级也当作表里的一条
/// 规则一次性裁决（见上面的前置条件），故 `--approve` 已给出时返回 `Deny`，其含义
/// 只有两种：
///
/// - **第 1 级为 `Deny`**——它高于第 2 级，此时**不铸造批准值**；
/// - **调用方未按前置条件放入第 2 级规则**——此时本函数无从知道 `--approve` 的存在，
///   `Deny` 只是「没有更高的规则放行」，是否铸造按第 5.7 节的接法处理。
///
/// 混用两种接法恰好在「第 1 级不可越」这条性质上 fail-open：把第一种含义的 `Deny`
/// 按第 5.7 节表格里 `Deny` 那一行的字面读法（「有 `--approve` 才铸造」）处理，
/// 就会给第 1 级 `Deny` 铸造出批准值。
///
/// 两种含义都由 `tests/arbitration.rs` 的
/// `with_an_explicit_approval_a_deny_comes_only_from_the_first_level` 钉住。第二种
/// 含义同时是「漏放第 2 级规则」的失误面：`--approve` 失效、拒绝照旧，即
/// fail-closed——比相反方向（fail-open）安全。
///
/// [`crate::rule::Scope`] 不影响裁决结果：它只记规则的来源，供存储与审计用
/// （第 5.3 节的裁决规则里只有层级与决策）。
///
/// 用例见 `tests/arbitration.rs`。
pub fn decide(policies: &[Policy], ctx: &PolicyContext) -> Decision {
    let mut winner: Option<(Level, Decision)> = None;
    for policy in policies {
        if !policy.condition.matches(ctx) {
            // 条件不成立，或所引用的事实不在上下文中（设计下篇第 5.3 节）。
            // 两者都不放行：放行必须由某条明确的 `Allow` 给出。
            continue;
        }
        let replace = match winner {
            // 更高层（判别值更小）获胜；同层则取更严的一条。
            Some((level, decision)) => {
                policy.level < level
                    || (policy.level == level && severity(policy.decision) > severity(decision))
            }
            None => true,
        };
        if replace {
            winner = Some((policy.level, policy.decision));
        }
    }
    // 无匹配即 `Deny`：本设计是 fail-closed 的（设计下篇第 5.3 节）。
    winner.map_or(Decision::Deny, |(_, decision)| decision)
}

/// 同层内的严苛次序：`Deny` 最严，`Allow` 最松。
///
/// 只用于同层比较，故不涉及层级。数值本身无意义，只提供全序。
fn severity(decision: Decision) -> u8 {
    match decision {
        Decision::Allow => 0,
        Decision::RequireApproval => 1,
        Decision::Deny => 2,
    }
}
