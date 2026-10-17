// 应编译失败：提案不能表达终态；`validity` 不是 `ProposedClaim` 的字段。
// 样例：proposed_claim_cannot_express_validity（C4a）。
use continuum_memory::{Confidence, MemoryClaimId, MemorySourceId, ProposedClaim, Validity};

fn main() {
    let _ = ProposedClaim {
        id: MemoryClaimId::new("c"),
        statement: "陈述".to_string(),
        sources: vec![MemorySourceId::new("s")],
        confidence: Confidence::from_value(serde_json::json!(null)),
        supersedes: vec![],
        validity: Validity::Active,
    };
}
