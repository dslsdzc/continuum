// 应编译失败：`Evidence` 的八个字段**全部私有**，故**全字段结构体字面量**这条路在
// crate 外走不通（预期是「字段私有」那一类，常见码 E0451；**判据是实跑产出的那份
// `.stderr` 本身，不是这个码**）。
// **文件名即这条通道——本样例只钉结构体字面量这一条。**
//
// 它钉的是 P5a 设计 §4.4 的「唯一产生点」是**结构事实**：`Evidence` 的构造函数只有
// `Evidence::from_tool_result`（`crates/continuum-verify/src/evidence.rs`），字面量构造被私有
// 字段挡住——若字段可写，调用方可以绕过那个产生点自己拼一条证据，「唯一」就不成立。
// **这是本块（P5d）在那条结构事实上的使用侧照片**：证据在本 crate 里造不出来。
//
// 八个字段的值取自函数形参（与 `Evidence::from_tool_result` 的八参数同形）——
// **零件全在手里，也拼不出这个字面量**：值怎么来不影响「字段私有」这条事实。
//
// **它不声称「拿不到一枚 `Evidence`」**：`from_tool_result` 是 `pub fn`，
// crate 外经它就能拿到证据；被挡住的只是「绕过它自己拼一个」。
//
// **失效条件（移除档）**：把 `Evidence` 的字段开成可写 ⇒ 本样例的编译失败输出随之
// 改变、与 `.stderr` 不再相符 ⇒ `trybuild` 报 mismatch ⇒ `type_level.rs` 的用例红
// （实测：第 0 个字段单独改 `pub` 时少一项字段报错）。兄弟样例 `evidence_has_no_default.rs`
// 钉的是另一条通道（无 `Default`），**不许并成一份**——两条通道的失效条件不同：
// 字段若被开成可写，本样例变；别处若给 `Evidence` 加了 `Default` 实现，那份样例变。
use continuum_artifact::ArtifactId;
use continuum_verify::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, EvidenceType,
};

fn build(
    id: EvidenceId,
    evidence_type: EvidenceType,
    subject: EvidenceSubject,
    claim: Claim,
    producer: EvidenceProducer,
    artifact_refs: Vec<ArtifactId>,
    strength: EvidenceStrength,
    scope: EvidenceScope,
) -> Evidence {
    // P5a §4.4：八个私有字段，字面量构造被挡住——这一次构造编译不过。
    Evidence {
        id,
        evidence_type,
        subject,
        claim,
        producer,
        artifact_refs,
        strength,
        scope,
    }
}

fn main() {
    // 取一次函数项，免得 `build` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次字面量仍是本样例要的那次解析。
    let _ = build;
}
