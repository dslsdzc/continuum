// 应编译失败：提案不能表达时间；`last_confirmed` 不是 `ProposedClaim` 的字段。
// 样例：proposed_claim_cannot_express_last_confirmed（C4b）。
use continuum_memory::{Confidence, MemoryClaimId, MemorySourceId, ProposedClaim};

fn main() {
    let _ = ProposedClaim {
        id: MemoryClaimId::new("c"),
        statement: "陈述".to_string(),
        sources: vec![MemorySourceId::new("s")],
        confidence: Confidence::from_value(serde_json::json!(null)),
        supersedes: vec![],
        last_confirmed: 0,
    };
}
