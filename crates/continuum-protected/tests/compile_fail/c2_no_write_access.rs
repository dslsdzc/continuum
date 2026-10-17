// 应编译失败：`ProtectedCore` 的字段私有，crate 外没有写入口。
use continuum_protected::{ProtectedCore, RuleCommitment};

fn mutate(core: &mut ProtectedCore) {
    core.authority_rules = RuleCommitment::unavailable();
}

fn main() {
    let _ = mutate;
}
