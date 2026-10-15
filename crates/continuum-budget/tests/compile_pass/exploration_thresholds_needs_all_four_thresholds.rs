// 应编得过（正控制）：四个阈值**全给**才构造得出 `ExplorationThresholds`，
// 且 `check_exploration` 的签名可用。
//
// # 它守的坏法
//
// 没有它，`exploration_thresholds_has_no_default` 的绿可以来自
// 「把 `ExplorationThresholds` 删光 / 把 `check_exploration` 删光」这类与判据
// 无关的坏法（恒绿的守卫比没有更坏）。
//
// # 四个字段逐项写出
//
// 前两个是 §7 的规范级默认（`wall_time ≤ 60s`、`money_cost = 0`——ENG-005 §三.1
// 采为规范级默认）；后两个（`network_transfer` / `gpu_time`）是 §7 里写作
// `threshold` 的那两个数，**由调用方以值给出**——本样例给 1000。
//
// 只在**编译期**证明签名与构造可用（trybuild 的 pass 样例会被编译并运行
// `main`）；本样例是纯函数调用，不建库、不落任何表。
use continuum_budget::{
    Dimensions, ExplorationThresholds, ExplorationVerdict, check_exploration,
};

fn main() {
    let thresholds = ExplorationThresholds {
        wall_time: 60,
        money: 0,
        network_transfer: 1000,
        gpu_time: 1000,
    };
    let dims = Dimensions {
        money: Some(0),
        wall_time: Some(60),
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    // 穷尽解构两个变体（不带 `..`）——签名与裁决值域都可用。
    match check_exploration(&dims, &thresholds, false, true) {
        ExplorationVerdict::AutoAllowed => {}
        ExplorationVerdict::DecisionRequired { dimension: _ } => {}
    }
}
