// 应编译失败：`ProtectedCore` 的五个字段全部私有，crate 外写不出全字段结构体
// 字面量。唯一构造点是 `seal`。
use continuum_protected::{ProtectedCore, RuleCommitment};

fn main() {
    let _core = ProtectedCore {
        authority_rules: RuleCommitment::unavailable(),
        capability_rules: RuleCommitment::unavailable(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    };
}
