// 应编译失败：`ModelProfile` 的构造函数 `try_new` 是 crate 可见的（`pub(crate)`），
// 故**构造函数**这条路在 crate 外也走不通（E0624）。
//
// 与 `model_profile_cannot_be_built.rs` 是**两条独立的通道**（构造函数 / 结构体字面量），
// 各需一张照片。只钉字面量那一份是不够的：把 `pub(crate) fn try_new` 改成
// `pub fn try_new` 时，字面量样例**仍然**因字段私有而编译失败，它不会变红——
// 缺的正是这一侧（fail-open 的那一侧）。
//
// **本样例钉不住什么**：它钉的是 `try_new` **这个名字**在 crate 外不可用，
// 钉不住「今后不会有人加一个别的名字的公开构造函数」——名字无从枚举。
// 这条限制写在这里而不是假装覆盖到了。
use continuum_capability::{Cost, Latency};
use continuum_core::model::ModelId;
use continuum_core::tool::ToolId;
use continuum_model_registry::{ModelProfile, Ratio, SkillVector};

fn main() {
    let _ = ModelProfile::try_new(
        ModelId::new("model-alpha"),
        String::from("version-beta"),
        String::from("provider-gamma"),
        String::from("revision-delta"),
        vec![String::from("text")],
        vec![ToolId::new("tool-epsilon")],
        SkillVector::from_current(Vec::new()),
        vec![String::from("loses constraints in very long tasks")],
        Some(Latency),
        Some(Cost),
        42,
        Ratio::try_new(0.94).unwrap(),
    );
}
