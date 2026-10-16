// 应编译失败：`Evidence` 的八个字段**全部私有**，故**全字段结构体字面量**这条路在
// crate 外走不通（预期是「字段私有」那一类，常见码 E0451；**判据是实跑产出的那份
// `.stderr` 本身，不是这个码**）。
// **文件名即这条通道——本样例只钉结构体字面量这一条。**
//
// 它钉的是 §4.4 的「唯一产生点」是**结构事实**：`Evidence` 的构造函数只有
// `Evidence::from_tool_result`（§89），字面量构造被私有字段挡住——若字段可写，
// 调用方可以绕过那个产生点自己拼一条证据，「唯一」就不成立。
//
// **它不声称「拿不到一枚 `Evidence`」**：`from_tool_result` 是 `pub fn`
// （`src/evidence.rs`），crate 外经它就能拿到证据；被挡住的只是「绕过它自己拼一个」。
use continuum_verify::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, EvidenceType,
};

fn build() -> Evidence {
    Evidence {
        id: EvidenceId::from_str("e-1"),
        evidence_type: EvidenceType::Test,
        subject: EvidenceSubject::Unattached,
        claim: Claim::from_str("测试通过"),
        producer: EvidenceProducer::Human,
        artifact_refs: Vec::new(),
        strength: EvidenceStrength::from_value(serde_json::Value::Null),
        scope: EvidenceScope::from_value(serde_json::Value::Null),
    }
}

fn main() {
    // 取一次函数项，免得 `build` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次字面量仍是本样例要的那次解析。
    let _ = build;
}
