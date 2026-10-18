//! R9 装配照片（设计 §8.1、§4.3；计划 Task 6）。
//!
//! 本照片把「五组槽位的装配状态」拍成值：`capability_rules` / `audit_rules` /
//! `effect_rules` 是 `Committed`（版本 1），`authority_rules` / `contract_rules` 是
//! 显式的 `Unavailable`。三组的摘要由**本用例手写的规范字节**算出。
//!
//! **这是 tripwire**：上游规则集落地后必须改这条用例，不许静默；**生产装配落位时
//! 必须复用同一段字节生成逻辑**，不许另写一份——两份字节会给出两个摘要，而两个都
//! 「看起来对」。
//!
//! 字节的固定分隔符取 `\n`，**不带尾随换行**。`EffectState` 没有 `as_str`（编码在
//! `continuum-effect` 的私有 `persist::state_str`），故七个状态串由下面的 7 臂、无通配
//! 臂的 `match` 给出；新增第 8 个状态时本文件编译不过，必须回到用例同步。

use continuum_capability::{
    CapabilityKind, EmailAction, EnvAction, FsAction, GitAction, GithubAction, PaymentAction,
    RegistryAction,
};
use continuum_effect::{EffectState, EffectType};
use continuum_events::AuditKind;
use continuum_protected::{ProtectedCoreParts, RuleCommitment, RuleVersion, seal};

/// R9 的固定分隔符：`\n`，不带尾随换行。
const SEP: &str = "\n";

/// 12 个具体能力值，按 §4.3 的装配次序；match 不带 `..`，加第 13 臂时编译不过。
fn capability_str(kind: CapabilityKind) -> &'static str {
    match kind {
        CapabilityKind::Filesystem(FsAction::Read) => kind.as_str(),
        CapabilityKind::Filesystem(FsAction::Write) => kind.as_str(),
        CapabilityKind::Git(GitAction::Read) => kind.as_str(),
        CapabilityKind::Git(GitAction::WorktreeWrite) => kind.as_str(),
        CapabilityKind::Git(GitAction::CommitLocal) => kind.as_str(),
        CapabilityKind::Git(GitAction::Push) => kind.as_str(),
        CapabilityKind::Git(GitAction::DeleteRemote) => kind.as_str(),
        CapabilityKind::Github(GithubAction::CreatePr) => kind.as_str(),
        CapabilityKind::Email(EmailAction::Send) => kind.as_str(),
        CapabilityKind::Registry(RegistryAction::Publish) => kind.as_str(),
        CapabilityKind::Payment(PaymentAction::Charge) => kind.as_str(),
        CapabilityKind::Environment(EnvAction::Deploy) => kind.as_str(),
    }
}

/// `EffectState` 在本用例里的固定串，与 `persist::state_str` 的落库编码同形。
/// 7 臂、无通配臂：新增状态时编译不过。
fn effect_state_str(state: EffectState) -> &'static str {
    match state {
        EffectState::Planned => "planned",
        EffectState::Authorized => "authorized",
        EffectState::Executing => "executing",
        EffectState::Committed => "committed",
        EffectState::Failed => "failed",
        EffectState::RolledBack => "rolled_back",
        EffectState::Unknown => "unknown",
    }
}

fn capability_bytes() -> String {
    let kinds = [
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Filesystem(FsAction::Write),
        CapabilityKind::Git(GitAction::Read),
        CapabilityKind::Git(GitAction::WorktreeWrite),
        CapabilityKind::Git(GitAction::CommitLocal),
        CapabilityKind::Git(GitAction::Push),
        CapabilityKind::Git(GitAction::DeleteRemote),
        CapabilityKind::Github(GithubAction::CreatePr),
        CapabilityKind::Email(EmailAction::Send),
        CapabilityKind::Registry(RegistryAction::Publish),
        CapabilityKind::Payment(PaymentAction::Charge),
        CapabilityKind::Environment(EnvAction::Deploy),
    ];
    kinds
        .into_iter()
        .map(capability_str)
        .collect::<Vec<_>>()
        .join(SEP)
}

fn audit_bytes() -> String {
    AuditKind::ALL
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join(SEP)
}

fn effect_bytes() -> String {
    let mut parts: Vec<&'static str> = EffectType::ALL.iter().map(|kind| kind.as_str()).collect();
    parts.extend(EffectState::ALL.iter().map(|state| effect_state_str(*state)));
    parts.join(SEP)
}

#[test]
fn r9_assembled_core_commits_three_groups_and_leaves_two_unavailable() {
    let capability = capability_bytes();
    let audit = audit_bytes();
    let effect = effect_bytes();

    let version = RuleVersion::new(1).expect("版本 1 合法");
    let core = seal(ProtectedCoreParts {
        authority_rules: RuleCommitment::unavailable(),
        capability_rules: RuleCommitment::committed(version, capability.as_bytes())
            .expect("capability 词表非空"),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::committed(version, audit.as_bytes())
            .expect("audit 词表非空"),
        effect_rules: RuleCommitment::committed(version, effect.as_bytes())
            .expect("effect 词表非空"),
    })
    .expect("三组已提交、两组缺失是合法的部分核心");

    for (name, commitment, bytes) in [
        ("capability_rules", core.capability_rules(), capability.as_str()),
        ("audit_rules", core.audit_rules(), audit.as_str()),
        ("effect_rules", core.effect_rules(), effect.as_str()),
    ] {
        assert_eq!(commitment.version(), Some(version), "{name} 的版本应为 1");
        assert!(commitment.digest().is_some(), "{name} 应有摘要");
        assert!(
            commitment.matches(bytes.as_bytes()),
            "{name} 应匹配本用例的规范字节"
        );
    }

    for (name, commitment) in [
        ("authority_rules", core.authority_rules()),
        ("contract_rules", core.contract_rules()),
    ] {
        assert_eq!(commitment.version(), None, "{name} 必须显式 Unavailable");
        assert_eq!(commitment.digest(), None, "{name} 必须显式 Unavailable");
    }
}
