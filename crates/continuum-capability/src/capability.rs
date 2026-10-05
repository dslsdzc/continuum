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

    /// [`CapabilityKind::for_effect`] 的逆（设计 §3.2 第 4 条）。
    ///
    /// 对应关系由 `for_effect` 拥有，**逆也必须只有一个产生点**，故它与 `for_effect`
    /// 同址（同一 impl 块、紧邻）。`for_effect` 是单射而**不是满射**，故返回 `Option`。
    ///
    /// 十二个臂**逐个手写、穷尽且无通配臂**：加 kind 时本函数编译不过，逆不会漏分支。
    /// 注意臂要写到**动作枚举那一层**（`Self::Filesystem(FsAction::Read)` 这样），
    /// 不能写成 `Self::Filesystem(_)`——后者在 `FsAction` 加变体时照样编译通过，
    /// 「加 kind 编译不过」这条保证就没了。
    ///
    /// 「该给 `Some` 的恰是像那六枚」由 `tests/vocabulary.rs` 的三条用例钉（前两条逐项、
    /// 第三条钉两份清单互斥）；**编译器看不出把 `Some` 写成 `None` 的臂**。
    /// **而「kind 一共十二枚」这一半是编译期保证，不是运行期用例**——本 crate 没有
    /// `CapabilityKind::ALL`，且 `as_str` 是实例方法，运行期遍历不出全集（见该用例的说明）。
    pub fn effect(&self) -> Option<EffectType> {
        match self {
            Self::Email(EmailAction::Send) => Some(EffectType::SendEmail),
            Self::Git(GitAction::Push) => Some(EffectType::PushBranch),
            Self::Registry(RegistryAction::Publish) => Some(EffectType::Publish),
            Self::Git(GitAction::DeleteRemote) => Some(EffectType::DeleteRemote),
            Self::Payment(PaymentAction::Charge) => Some(EffectType::Charge),
            Self::Environment(EnvAction::Deploy) => Some(EffectType::Deploy),
            Self::Filesystem(FsAction::Read) => None,
            Self::Filesystem(FsAction::Write) => None,
            Self::Git(GitAction::Read) => None,
            Self::Git(GitAction::WorktreeWrite) => None,
            Self::Git(GitAction::CommitLocal) => None,
            Self::Github(GithubAction::CreatePr) => None,
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
    /// - **§88 照录**（`docs/spec/02-positioning.md:909-910`）：`read`、`worktree.write`、
    ///   `commit.local`、`push`。多词用**点号**，因为给例就是这么写的；
    /// - **§253 照录**（`docs/spec/05-normative.md:984-986`）：`read`、`write`
    ///   （给例 `git.write:/task-worktree` 里的动作串）、`create_pr`——多词用
    ///   **下划线**，给例就是这么写的；
    /// - **推导项**（设计第 2.2 节按 [`EffectType`] 补齐、规范无给例）：
    ///   `delete_remote`、`send`、`publish`、`charge`、`deploy`，
    ///   按本仓编码约定小写、多词以 `_` 连接。
    ///
    /// # 这处不对称是刻意的，不是笔误
    ///
    /// `worktree.write` / `commit.local` 用点号而 `create_pr` / `delete_remote` 用
    /// 下划线，**来源是规范自己不统一**（§88 给例与 §253 给例的两种形状，见上表），
    /// 不是本 crate 漏了统一。**动手「修」它之前请回去读那两处原文**：
    ///
    /// - 若把前两条改成 `worktree_write` / `commit_local`，`Display` 就再也产不出
    ///   §88 的字面串，而 `git.worktree_write` 这个串**在规范里不存在**；
    /// - 若把 `create_pr` 改成 `create.pr`，`Display` 就产不出 §253 的字面串
    ///   `github.create_pr:repo/X`。
    ///
    /// 两种改法都是**把规范的给例改掉**去迁就一条本仓的拼写习惯。这条注释留在此处
    /// 而不是只留在报告里，正是本项目「订正时把错误说法的来历留在原地」的手法：
    /// 后来者看到不对称会想改，得先看到这段。
    ///
    /// 另：这里的串**不是落库编码**。它们是 §253 的展示词汇，只经
    /// [`Display`](std::fmt::Display) 出；`Capability` 本体不落库（设计 §2.3）。
    /// **落库的是 [`CapabilityKind::as_str`]**——本 crate 落库的枚举元素是
    /// `Tool.required_capabilities` 里的 kind（Task 4 起），它用的是那一对编码函数，
    /// 与此处逐项取值不同（见 [`CapabilityKind::as_str`]）。
    ///
    /// match 穷尽且无通配臂：加动作时本函数编译不过，串不会漏分支。逐项取值由
    /// `tests/capability.rs` 的 `resource_and_action_strings_follow_the_spec_examples`
    /// 钉住（手工字面量，钉格式）。
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

    /// 落库编码（设计 §3.3）：小写，多词以 `_` 连接，`resource` 与 `action` 之间也以
    /// `_` 连接。**本函数是该编码唯一的产生点**：`tool.required_capabilities` 列的元素
    /// 写入取用它，解码侧取 [`CapabilityKind::parse`]。
    ///
    /// # 与 [`CapabilityKind::action`] **不是同一张串表**
    ///
    /// `action()` 复现 §88 / §253 的**展示词汇**，按给例照录，故
    /// [`GitAction::WorktreeWrite`] 是 `worktree.write`（**点号**）——
    /// 那个串里有 `.`，不满足落库编码「小写 + `_`」的约定，**不可**直接落库。
    /// 本函数给出的是 `git_worktree_write`。
    ///
    /// 两者的照片：`tests/persist.rs` 的
    /// `capability_kind_storage_encoding_round_trips_every_arm` 同时断言
    /// `action() == "worktree.write"` 与 `as_str() == "git_worktree_write"`，
    /// 并遍历全部十二个 kind。
    ///
    /// **不要「统一」这两个函数**：把 `action()` 改成下划线会产不出 §88 的字面串
    /// `git.worktree.write`；把本函数改成点号会违反落库约定。不对称的来历同样记在
    /// [`CapabilityKind::action`] 的文档里。
    ///
    /// match 穷尽且无通配臂：加 kind 时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Filesystem(FsAction::Read) => "filesystem_read",
            Self::Filesystem(FsAction::Write) => "filesystem_write",
            Self::Git(GitAction::Read) => "git_read",
            Self::Git(GitAction::WorktreeWrite) => "git_worktree_write",
            Self::Git(GitAction::CommitLocal) => "git_commit_local",
            Self::Git(GitAction::Push) => "git_push",
            Self::Git(GitAction::DeleteRemote) => "git_delete_remote",
            Self::Github(GithubAction::CreatePr) => "github_create_pr",
            Self::Email(EmailAction::Send) => "email_send",
            Self::Registry(RegistryAction::Publish) => "registry_publish",
            Self::Payment(PaymentAction::Charge) => "payment_charge",
            Self::Environment(EnvAction::Deploy) => "environment_deploy",
        }
    }

    /// [`CapabilityKind::as_str`] 的**严格逆**：每个 kind 都有
    /// `parse(k.as_str()) == Some(k)`，表外字符串一律 `None`。
    ///
    /// `None` 而非某个默认 kind：把表外串猜成另一枚能力，正是 §253 要拦的伪造路径
    /// （与 [`EffectType::parse`] 同一条理由）。解码侧没有穷尽 match 的保护——来源是
    /// `&str` 而非枚举，编译器点不出漏掉的臂，故由 `tests/persist.rs` 的
    /// `capability_kind_storage_encoding_round_trips_every_arm` 遍历全部十二个 kind 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "filesystem_read" => Self::Filesystem(FsAction::Read),
            "filesystem_write" => Self::Filesystem(FsAction::Write),
            "git_read" => Self::Git(GitAction::Read),
            "git_worktree_write" => Self::Git(GitAction::WorktreeWrite),
            "git_commit_local" => Self::Git(GitAction::CommitLocal),
            "git_push" => Self::Git(GitAction::Push),
            "git_delete_remote" => Self::Git(GitAction::DeleteRemote),
            "github_create_pr" => Self::Github(GithubAction::CreatePr),
            "email_send" => Self::Email(EmailAction::Send),
            "registry_publish" => Self::Registry(RegistryAction::Publish),
            "payment_charge" => Self::Payment(PaymentAction::Charge),
            "environment_deploy" => Self::Environment(EnvAction::Deploy),
            _ => return None,
        })
    }
}

/// 设计 §2.4：签发来源。
///
/// 规范里的终极来源是 Authority Host（§292，长期阶段），本阶段尚无该宿主。
/// [`Issuer::AuthorityHost`] 在**本 crate 中不产出**（唯一签发点 [`mint`] 只记另一个
/// 变体；照片是 `tests/capability.rs` 的
/// `every_minted_capability_records_the_only_issuer_of_this_stage`，十二个 kind 全在）。
/// 它标出移交的去向（设计第 10 节第 2 条）——凡可承载于类型的移交不留在文字里。
/// 它是公开枚举的变体，故下游**可以**构造它；本阶段没有那样做的签发方。
///
/// 本阶段签发的每一枚能力都记 [`Issuer::PolicyWithExplicitApproval`]：那是**本阶段
/// 唯一签发路径**的名字（策略裁决 + 显式确认）。名字记的是**路径**，不是「本次附带
/// 了一次显式确认」——那一位信息属驱动（它随 [`mint`] 的签名一起不入本类型，理由见
/// `mint` 的文档）。照片：`tests/capability.rs` 的
/// `every_minted_capability_records_the_only_issuer_of_this_stage`（逐项断言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issuer {
    PolicyWithExplicitApproval,
    AuthorityHost,
}

/// §253 的五要素。字段私有、无公开构造函数：本 crate 里唯一的产出路径是 [`mint`]。
/// **crate 外**由类型保证——两条构造通道各有一份编译失败样例（无公开构造函数、
/// 结构体字面量被拒）；**crate 内**的第二个产出点由评审与 grep 维持，理由与那张
/// 同形的既有判据见 [`mint`] 的文档。
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
/// # 判定不在本函数里：唯一落点是驱动的 `mints`
///
/// 「这次授权铸不铸得出」的判定（裁决值 × `--approve` 给没给，六格）**只**在驱动：
/// `continuum-runtime` 的 `mints`（`crates/continuum-runtime/src/task_cmd.rs:649-654`，
/// 设计下篇第 5.7 节的表在本仓的接法），其照片是那里的
/// `the_mapping_from_a_decision_to_minting_has_six_cells` 与
/// `a_system_safety_deny_is_not_overridden_by_the_flag`。
///
/// 若本函数也按那六格算一遍，同一个判断就有了**两个产生点**——改一处不会让另一处
/// 失败，而本项目一贯把这类形状判为缺陷（P2 为此出过 Critical）。故本函数的签名里
/// **没有**裁决值、也没有「是否附了显式确认」这类入参：不是「本处复核后放行」，而是
/// **本处无从复核**。
///
/// 由此，本 crate 里曾经有过的 `Grant` 与 `Verdict` 两个类型**已删除**——去掉重判后
/// 它们没有任何消费方（`issuer` 恒为本阶段那一条路径，一位信息，[`Issuer`] 已有那个
/// 变体）。本 crate 对「声明了没有消费方的东西」一贯要么删、要么写明理由，这次是删；
/// 来历见设计 §2.4 的「签发点不重判，故 `Grant` / `Verdict` 已删」一段——**执行期裁定**，
/// 不是漏实现——与计划 Task 2 Step 3 的同名订正。
///
/// 「只有一个具名的签发点」这条保证由此**不再由类型独家承担**：类型层保证的是
/// 「**crate 外除 `mint` 外没有第二条产出能力的公开路径**」（结构体字面量与公开构造函数两条通道
/// 各有一份 `tests/compile_fail/` 样例；另有 `FromStr` / `From<&str>` 两份钉住字符串还原）。
/// 注意**不是**「crate 外造不出来」——`mint` 本身公开且被再导出，crate 外经它产出一枚能力是
/// **设计承认的**（设计 §4.2：那是 `mint` 公开本身的性质，与本类型无关）。
/// crate **内**的第二处产出点由评审与 grep 维持——与 P2 对 `GateApproval` 的既有判据同形
/// （`crates/continuum-workspace/src/gate.rs:12-17`：「只有一个具名的产生点，维持手段是
/// 评审与 grep，不再是类型」）：字段的可见性是**crate 内**，故本 crate 的任何模块都**可以**
/// 写字面量；今天只有本模块内的一处，本 crate 内的第二处由评审把关。
///
/// # 失败路径：空 `scope`
///
/// §253 的 `scope` 是自由文本，而空作用域的能力**没有任何下游能判断它指的是什么**
/// （凭据签发、执行点与对账都要读它）。与 P2b 的「效应目标为空在解析期即拒」是同一条
/// 判据，见 [`CapabilityError::EmptyScope`]。照片：`tests/capability.rs` 的
/// `mint_rejects_an_empty_scope`（两侧都有）。
///
/// **本函数不读时钟**（与 [`Capability::is_valid_at`] 同一条约定），故 `expiry` 是否
/// 已经过去不由它判断：铸得出的能力可以一铸即失效，那由 `is_valid_at` 在调用点拒。
/// 照片：`mint_does_not_read_the_clock`。
pub fn mint(
    kind: CapabilityKind,
    scope: String,
    expiry: i64,
) -> Result<Capability, CapabilityError> {
    if scope.is_empty() {
        return Err(CapabilityError::EmptyScope);
    }

    Ok(Capability {
        kind,
        scope,
        expiry,
        // 本阶段的签发路径只有一条（设计 §2.4）；Authority Host 就位后由它接手。
        issuer: Issuer::PolicyWithExplicitApproval,
    })
}
