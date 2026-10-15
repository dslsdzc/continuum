// 应编译失败：`ExplorationThresholds` **没有** `Default`——不存在「全默认」的
// 无参构造路径。
//
// # 判据
//
// `network_transfer` / `gpu_time` 两个阈值**原文只写 `threshold`、没有数值**
// （§7 末两行；§16 第 21 条）——故它们是**必填入参**，本层**不发明这两个数**。
// 一条无参的 `Default`（或任何全默认构造）等于替调用方发明了这两个数。
//
// # 形态
//
// `ExplorationThresholds::default()` 同时覆盖两条路：trait 的 `Default` 与
// inherent 的 `default`——加了任何一条，本样例都会**编得过**（那时该回来重判
// 「必填入参」这个决定，而不是改样例）。错误以实跑取值为准
// （`tests/type_level.rs` 的 `.stderr` 逐字比对）。
//
// # 正控制
//
// `tests/compile_pass/exploration_thresholds_needs_all_four_thresholds.rs`——
// 四个阈值全给时构造得出、`check_exploration` 的签名可用。**两处合起来才算钉住**：
// 只钉反例，「把这个类型删光」这类与判据无关的坏法照样绿。
use continuum_budget::ExplorationThresholds;

fn main() {
    // 无参构造：`default` 这个入口不存在。
    let _ = ExplorationThresholds::default();
}
