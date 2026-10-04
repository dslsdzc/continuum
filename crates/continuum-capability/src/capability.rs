//! 能力的半封闭词汇表（设计第 2.1、2.2 节）。
//!
//! `resource` 取封闭枚举 [`CapabilityKind`]，每个 resource 携带它自己的动作枚举
//! （[`FsAction`] 等），故「`Filesystem.Push`」这类组合在类型层面写不出来
//! （设计第 2.1 节）。
//!
//! 动作的取值有两个来源：§88 / §253 给例**照录**的，与按 [`EffectType`] **推导**
//! 补齐的。后者的理由与对照表见 [`CapabilityKind::for_effect`]。

use continuum_effect::EffectType;

/// §88 与 §253 的例子给到的三个 resource，加四个由 `EffectType` 反推的（设计第 2.2 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    Filesystem(FsAction),
    Git(GitAction),
    Github(GithubAction),
    Email(EmailAction),
    Registry(RegistryAction),
    Payment(PaymentAction),
    Environment(EnvAction),
}

/// §88 给例 `filesystem.read`（§253 的给例 `filesystem.read:/project` 同族）；
/// 动作集取设计第 2.1 节列出的 Read | Write。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FsAction {
    Read,
    Write,
}

/// §88 给例：`git.read` / `git.worktree.write` / `git.commit.local` / `git.push`。
///
/// `DeleteRemote` 在 §88 与 §253 里都没有给例，由 [`EffectType::DeleteRemote`]
/// 反推补齐（设计第 2.2 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitAction {
    Read,
    WorktreeWrite,
    CommitLocal,
    Push,
    DeleteRemote,
}

/// §253 给例：`github.create_pr:repo/X`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GithubAction {
    CreatePr,
}

/// 推导项（设计第 2.2 节）：对应 [`EffectType::SendEmail`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmailAction {
    Send,
}

/// 推导项（设计第 2.2 节）：对应 [`EffectType::Publish`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryAction {
    Publish,
}

/// 推导项（设计第 2.2 节）：对应 [`EffectType::Charge`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentAction {
    Charge,
}

/// 推导项（设计第 2.2 节）：对应 [`EffectType::Deploy`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvAction {
    Deploy,
}

impl CapabilityKind {
    /// [`EffectType`] 与 [`CapabilityKind`] 的对应（设计第 2.2 节的对照表）。
    ///
    /// 六个臂逐个手写、穷尽且**无通配臂**：`EffectType` 加变体时本函数编译不过，
    /// 对应不会漏分支。
    ///
    /// 本条对应是**单射**：六个 `EffectType` 给出六个互不相同的 `CapabilityKind`
    /// （`tests/vocabulary.rs` 的 `no_capability_kind_maps_to_two_effect_types`）。
    /// 它不是满射——[`CapabilityKind::Filesystem`]、[`GithubAction::CreatePr`]、
    /// [`GitAction::Read`] / [`GitAction::WorktreeWrite`] / [`GitAction::CommitLocal`]
    /// 在 `EffectType` 里没有对应项，它们是 §88 / §253 的照录项而非外部效应类型；
    /// 六条对应的精确取值由同文件的 `every_effect_type_has_exactly_one_capability`
    /// 逐项钉住。
    ///
    /// 方向写在本类型上而不是 `EffectType` 上：后者会让 `continuum-effect` 反向
    /// 依赖本 crate，而依赖方向在本项目是逐对断言的硬约束
    /// （`continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`）。
    pub fn for_effect(effect: EffectType) -> Self {
        match effect {
            EffectType::SendEmail => Self::Email(EmailAction::Send),
            EffectType::PushBranch => Self::Git(GitAction::Push),
            EffectType::Publish => Self::Registry(RegistryAction::Publish),
            EffectType::DeleteRemote => Self::Git(GitAction::DeleteRemote),
            EffectType::Charge => Self::Payment(PaymentAction::Charge),
            EffectType::Deploy => Self::Environment(EnvAction::Deploy),
        }
    }
}
