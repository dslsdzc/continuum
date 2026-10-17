// 应编译失败：`LearningProposal::new` 的第二参数只收 `LearningTarget`；
// `ProtectedSemantic` 不是可学习目标，学习产物指不到 §321 的受保护语义。
use continuum_learning::LearningProposal;
use continuum_protected::{
    ProtectedCoreParts, ProtectedSemantic, RuleCommitment, RuleVersion, seal,
};

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

    let _proposal = LearningProposal::new(&core, ProtectedSemantic::Authority, "x", "y");
}
