// 应编译失败：`ModelProfile` 的字段私有，故**结构体字面量**这条路在 crate 外走不通（E0451）。
//
// 这是设计 §2.1 / §4.2 那条结构性保证的一条通道：画像只能从库里来（`load_profile`），
// 否则一个从未落库、从未过画像流水线的模型就能被送进 `rank`，§21 的「新增模型不能直接
// 进入自动 Router」随之失效。
//
// **本样例只钉「字段私有」这一条通道**；「没有公开构造函数」是另一条通道，由
// `model_profile_has_no_constructor.rs` 单独钉。两条通道必须分开：把十二个字段全改 `pub`
// 只让本样例变红，把 `try_new` 改成 `pub` 只让那一份变红——反之亦然。
use continuum_capability::{Cost, Latency};
use continuum_core::model::ModelId;
use continuum_core::tool::ToolId;
use continuum_model_registry::{ModelProfile, Ratio, SkillVector};

fn main() {
    let _ = ModelProfile {
        id: ModelId::new("model-alpha"),
        version: String::from("version-beta"),
        provider: String::from("provider-gamma"),
        model_revision: String::from("revision-delta"),
        modalities: vec![String::from("text")],
        tools: vec![ToolId::new("tool-epsilon")],
        skill_vector: SkillVector::from_current(Vec::new()),
        failure_modes: vec![String::from("loses constraints in very long tasks")],
        latency_profile: Some(Latency),
        cost_profile: Some(Cost),
        evidence_count: 42,
        confidence: Ratio::try_new(0.94).unwrap(),
    };
}
