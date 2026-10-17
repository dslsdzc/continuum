// 正控制：只读面必须开着——否则 C1–C4 可以由「一律私有」满足。
// `core.authority_rules()`、`core.fingerprint()` 与 `LearningProposal::new` 都必须编得过。
use continuum_learning::{LearningProposal, LearningTarget};
use continuum_protected::{ProtectedCoreParts, RuleCommitment, RuleVersion, seal};

fn main() {
    let core = seal(ProtectedCoreParts {
        authority_rules: RuleCommitment::committed(RuleVersion::new(1).unwrap(), b"rules")
            .unwrap(),
        capability_rules: RuleCommitment::unavailable(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    })
    .unwrap();

    let _commitment = core.authority_rules();
    let _fingerprint = core.fingerprint();
    let _proposal = LearningProposal::new(&core, LearningTarget::RoutingPolicy, "x", "y");
}
