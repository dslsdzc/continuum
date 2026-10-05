// 应编译失败：`RoutableModel` 的字段私有，故**结构体字面量**这条路在 crate 外走不通
// （E0451）。**文件名即这条通道——本样例只钉结构体字面量这一条**。
//
// **它不声称「crate 外造不出 `RoutableModel`」**：`RoutableModel::try_new` 是 `pub fn`
// （`src/lifecycle.rs`，设计 §4.2 自己就写的 `pub`），crate 外经它就能构造出一个候选；
// 闸门（`RoutableState::try_from`）在**它函数体内**，这正是要被保护的那道口。
// 本样例钉的是另一条通道：**绕过 `try_new` 自己拼一个「已过闸门」的值**。若字段可写，
// 调用方就能直接拼出 `state: RoutableState::Active`，闸门形同虚设——`stale` /
// `unprofiled` / `quarantined` / `disabled` 四态随之进入 `rank` 的入参，
// §249 的 MUST NOT 与 `set-decisions` 第一节第 4 条一起落空。
//
// （本文件原名 `routable_model_cannot_be_built.rs`：那个名字会被读成上一段所否认的命题，
// 与「构造入口是公开的、闸门在其内」相抵，故改名。）
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
