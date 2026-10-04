//! 能力的半封闭词汇表与能力本身（设计第 2.1–2.4 节）。
//!
//! `resource` 取封闭枚举 [`CapabilityKind`]，每个 resource 携带它自己的动作枚举
//! （[`FsAction`] 等），故「`Filesystem.Push`」这类组合在类型层面写不出来
//! （设计第 2.1 节）。
//!
//! 动作的取值有两个来源：§88 / §253 给例**照录**的，与按 [`EffectType`] **推导**
//! 补齐的。后者的理由与对照表见 [`CapabilityKind::for_effect`]。
//!
//! [`Capability`] 是 §253 的五要素本体：字段私有、无公开构造函数，唯一产出路径是
//! 本模块的 [`mint`]。

use continuum_effect::EffectType;

use crate::error::CapabilityError;

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

    /// §253 二字段形状里的 `resource` 串，即本枚举的七个 resource。
    ///
    /// 取值取自给例：`filesystem` 与 `git`（§88 `docs/spec/02-positioning.md:909`、
    /// §253 `docs/spec/05-normative.md:984-986` 两处都有），`github`（§253）。
    /// 余下四个 resource（`email` / `registry` / `payment` / `environment`）是设计
    /// 第 2.2 节的推导项，规范未给串，取小写单数。
    ///
    /// match 穷尽且无通配臂：加 resource 时本函数编译不过，串不会漏分支。
    /// 逐项取值由 `tests/capability.rs` 的 `resource_and_action_strings_follow_the_spec_examples`
    /// 钉住（用例里的字面量是手工写的，钉的是格式；`for_effect` 那条路径另有用例钉）。
    pub fn resource(&self) -> &'static str {
        match self {
            Self::Filesystem(_) => "filesystem",
            Self::Git(_) => "git",
            Self::Github(_) => "github",
            Self::Email(_) => "email",
            Self::Registry(_) => "registry",
            Self::Payment(_) => "payment",
            Self::Environment(_) => "environment",
        }
    }

    /// §253 二字段形状里的 `action` 串。
    ///
    /// **取值不按一条统一规则拼**，而是逐项有来历——规范自己就是两种形状：
    ///
    /// - **§88 照录**（`docs/spec/02-positioning.md:909`）：`read`、`worktree.write`、
    ///   `commit.local`、`push`。多词用**点号**，因为给例就是这么写的；
    /// - **§253 照录**（`docs/spec/05-normative.md:984-986`）：`read`、`write`
    ///   （给例 `git.write:/task-worktree` 里的动作串）、`create_pr`——多词用
    ///   **下划线**，给例就是这么写的；
    /// - **推导项**（设计第 2.2 节按 [`EffectType`] 补齐、规范无给例）：
    ///   `delete_remote`、`send`、`publish`、`charge`、`deploy`，
    ///   按本仓编码约定小写、多词以 `_` 连接。
    ///
    /// 若一律按下划线拼，`git.worktree_write` 这个串在规范里**不存在**——那条不是
    /// 「编码约定」而是**照录**，本函数的取值以给例为准。
    ///
    /// match 穷尽且无通配臂：加动作时本函数编译不过，串不会漏分支。逐项取值由
    /// `tests/capability.rs` 的 `resource_and_action_strings_follow_the_spec_examples`
    /// 钉住。
    pub fn action(&self) -> &'static str {
        match self {
            Self::Filesystem(FsAction::Read) => "read",
            Self::Filesystem(FsAction::Write) => "write",
            Self::Git(GitAction::Read) => "read",
            Self::Git(GitAction::WorktreeWrite) => "worktree.write",
            Self::Git(GitAction::CommitLocal) => "commit.local",
            Self::Git(GitAction::Push) => "push",
            Self::Git(GitAction::DeleteRemote) => "delete_remote",
            Self::Github(GithubAction::CreatePr) => "create_pr",
            Self::Email(EmailAction::Send) => "send",
            Self::Registry(RegistryAction::Publish) => "publish",
            Self::Payment(PaymentAction::Charge) => "charge",
            Self::Environment(EnvAction::Deploy) => "deploy",
        }
    }
}

/// 设计 §2.4：签发来源。
///
/// 规范里的终极来源是 Authority Host（§292，长期阶段），本阶段尚无该宿主。
/// [`Issuer::AuthorityHost`] 在**本 crate 中不产出**（唯一签发点 [`mint`] 只记另一个
/// 变体；照片是 `tests/capability.rs` 的
/// `every_minted_capability_records_the_only_issuer_of_this_stage`，三格成功全在）。
/// 它标出移交的去向（设计第 10 节第 2 条）——凡可承载于类型的移交不留在文字里。
/// 它是公开枚举的变体，故下游**可以**构造它；本阶段没有那样做的签发方。
///
/// 本阶段签发的每一枚能力都记 [`Issuer::PolicyWithExplicitApproval`]：那是**本阶段
/// 唯一签发路径**的名字（策略裁决 + 显式确认），与「本次是否附带了一次显式确认」
/// **不是同一个问题**。`Grant::Policy(Verdict::Allow)`（未给 `--approve`）铸出的能力
/// 同样记它——本阶段没有第二个来源可记。照片：`tests/capability.rs` 的
/// `every_minted_capability_records_the_only_issuer_of_this_stage`（三格成功逐个断言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issuer {
    PolicyWithExplicitApproval,
    AuthorityHost,
}

/// §253 的五要素。字段私有、无公开构造函数：本 crate 里唯一的产出路径是 [`mint`]
/// （两条构造通道各有一份编译失败样例：无公开构造函数、结构体字面量被拒）。
///
/// 三条结构性保证（设计 §2.3）：
///
/// 1. **`full_access` 不存在**——不是「禁止作默认」，是没有这个成员、也没有这个字段。
///    两张照片：词汇表按成员钉（`tests/compile_fail/capability_has_no_full_access.rs`）、
///    §253 原文的字面形状 `full_access = true` 按字段钉
///    （`tests/compile_fail/full_access_field_does_not_exist.rs`）——两者是两条通道，
///    只钉一条会漏掉另一条；
/// 2. **`expiry` 必填**，无「不过期」的表示（§51 的 short-lived 是结构而非约定）。
///    **本类型不读时钟**：校验收 `now`，边界见 [`Capability::is_valid_at`]；
/// 3. **不可与裸字符串互换**——无 `From<&str>`、无 `FromStr`，只有单向的 `Display`
///    （供审计与日志）。这一条的每条通道各有一份 `tests/compile_fail/` 样例
///    （`FromStr`、`From<&str>`，加上上面那两条构造通道）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    kind: CapabilityKind,
    scope: String,
    expiry: i64,
    issuer: Issuer,
}

impl Capability {
    pub fn kind(&self) -> CapabilityKind {
        self.kind
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// Unix 毫秒的**失效时刻**（见 [`Capability::is_valid_at`]）。
    pub fn expiry(&self) -> i64 {
        self.expiry
    }

    pub fn issuer(&self) -> Issuer {
        self.issuer
    }

    /// §253 二字段形状的 `resource`。存储与比较都用 [`Capability::kind`]，本访问器
    /// 只把形状呈现出来。
    pub fn resource(&self) -> &'static str {
        self.kind.resource()
    }

    /// §253 二字段形状的 `action`。取值与逐项来历见 [`CapabilityKind::action`]。
    pub fn action(&self) -> &'static str {
        self.kind.action()
    }

    /// 本类型不读时钟：`now` 由调用方给（本项目的既有约定：核不收时钟）。
    ///
    /// `expiry` 是**失效时刻**而非「最后有效的时刻」，故判据是 `now < expiry`：
    /// `now == expiry` 已经失效。两个方向都有照片（`tests/capability.rs`）：
    /// `expiry - 1` → `Ok`，`expiry` 与 `expiry + 1` → `Err(Expired { .. })`。
    pub fn is_valid_at(&self, now: i64) -> Result<(), CapabilityError> {
        if now < self.expiry {
            Ok(())
        } else {
            Err(CapabilityError::Expired {
                expiry: self.expiry,
                now,
            })
        }
    }
}

/// §253 的展示形状：`resource.action:scope`（给例 `git.push:origin/main`、
/// `github.create_pr:repo/X`）。
///
/// **只出不进**（设计 §2.3 第三条）：本 crate 没有 `From<&str>`、没有 `FromStr`，
/// 两份 `tests/compile_fail/` 样例分别钉住这两条通道。若字符串能还原出能力，则凡能
/// 写字符串之处即可伪造能力——那正是 §253 要拦的。
impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}:{}", self.resource(), self.action(), self.scope)
    }
}

/// **唯一的签发点**（设计 §2.4）。今天的临时签发方是「策略裁决 + 显式确认」，
/// 也就是 `continuum-workspace` 的 `gate.rs` 已经点名的那个位置。
///
/// 本 crate 里没有第二个产出 `Capability` 的公开路径：另外两条构造通道（公开构造
/// 函数、结构体字面量）各有一份 `tests/compile_fail/` 样例。
///
/// `granted` 是「什么授权了这一次」的表示，由调用方（驱动）从策略裁决构造——
/// 本 crate 不依赖 `continuum-policy`，故用本 crate 自己的类型（依赖方向在本项目
/// 是逐对断言的硬约束）。
///
/// **可否签发按本仓既有的六格接法**（`continuum-runtime` 的 `mints`，
/// `crates/continuum-runtime/src/task_cmd.rs:649-654`，设计下篇第 5.7 节的表在本仓
/// 的接法）：`Allow` 无论有无显式确认皆铸；`RequireApproval` 须有显式确认；`Deny`
/// 一律不铸。不另发明判定——`Grant` 的两个变体正好承载这张表的两个入参（裁决值与
/// `--approve` 给没给）。
///
/// **本函数不读时钟**（与 [`Capability::is_valid_at`] 同一条约定），故 `expiry` 是否
/// 已经过去不由它判断：铸得出的能力可以一铸即失效，那由 `is_valid_at` 在调用点拒。
/// 照片：`tests/capability.rs` 的 `mint_does_not_read_the_clock`。
///
/// 六个格子逐个手写、**穷尽且无通配臂**：`Verdict` 加变体时本函数编译不过。
/// 六格逐格有照片：`tests/capability.rs` 的 `minting_follows_the_six_cell_table`。
pub fn mint(
    kind: CapabilityKind,
    scope: String,
    expiry: i64,
    granted: Grant,
) -> Result<Capability, CapabilityError> {
    match granted {
        Grant::Policy(Verdict::Allow) => {}
        Grant::Policy(Verdict::RequireApproval) => return Err(CapabilityError::ApprovalRequired),
        Grant::Policy(Verdict::Deny) => return Err(CapabilityError::PolicyDenied),
        Grant::PolicyWithExplicitApproval(Verdict::Allow) => {}
        Grant::PolicyWithExplicitApproval(Verdict::RequireApproval) => {}
        Grant::PolicyWithExplicitApproval(Verdict::Deny) => return Err(CapabilityError::PolicyDenied),
    }

    Ok(Capability {
        kind,
        scope,
        expiry,
        issuer: Issuer::PolicyWithExplicitApproval,
    })
}

/// 「什么授权了这一次」。`PolicyWithExplicitApproval` 对应 §5.5 的 `--approve`：
/// 显式确认**已给出**时用后者，未给出时用前者。两者的差别只有这一处，而它正是
/// [`mint`] 六格表里 `RequireApproval` 那一行的分岔。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grant {
    Policy(Verdict),
    PolicyWithExplicitApproval(Verdict),
}

/// 策略裁决的结果。与 `continuum_policy::Decision` 的三个取值一一对应
/// （`Allow` / `Deny` / `RequireApproval`，见 `crates/continuum-policy/src/rule.rs:88-92`）；
/// 由驱动转换过来（本 crate 不依赖 `continuum-policy`，依赖方向是逐对断言的硬约束）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    RequireApproval,
    Deny,
}
