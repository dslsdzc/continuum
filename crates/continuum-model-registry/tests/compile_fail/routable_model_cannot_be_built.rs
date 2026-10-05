// 应编译失败：`RoutableModel` 的字段私有，故**结构体字面量**这条路在 crate 外走不通
// （E0451）。
//
// 这是设计 §4.2 那条结构性保证的载体：唯一能把一份画像与一个状态凑成候选的入口是
// `RoutableModel::try_new`，而它**在函数体内过闸门**（`RoutableState::try_from`）。
// 若字段可写，调用方就能自己拼一个「已过闸门」的值出来——`stale` / `unprofiled` /
// `quarantined` / `disabled` 四态随之进入 `rank` 的入参，§249 的 MUST NOT 与
// `set-decisions` 第一节第 4 条一起落空。
//
// **本样例不构造画像**（crate 外构造不出来，见 `model_profile_cannot_be_built.rs`），
// 故只声明一个收 `ModelProfile` 的函数再写字面量——这足以让编译器在结构体字面量上报字段私有。
use continuum_model_registry::{ModelProfile, RoutableModel, RoutableState};

fn build_with(profile: ModelProfile) -> RoutableModel {
    RoutableModel {
        profile,
        state: RoutableState::Active,
    }
}

fn main() {
    // 取一次函数项，免得 `build_with` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次字面量仍是本样例要的那次解析。
    let _ = build_with;
}
