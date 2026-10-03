//! Integration Gate（§257）：Task Workspace 到 Base Workspace 的唯一通道。
//!
//! Base 的工作文件永久只读（§256），Task 的改动只能经本模块进入 Base。`§257` 的
//! 五种操作都在本模块：只读的 `view_diff`、`discard`（Task 8），与写入的
//! `apply_patch`、`cherry_pick`、`merge`（Task 9）。
//!
//! **只读操作不收批准值。** 它们不动 Base，按 `§15` 的影响级是 L0，收一个
//! [`GateApproval`] 只会让「这个值代表什么」变得含糊。写入操作一律收
//! `&GateApproval`，使「集成修改需要授权」有落点（§16、设计 6.2）；该类型在本模块内
//! 定义但没有公开构造函数，其产生点在下篇交给驱动。
//!
//! # 写入操作与事务
//!
//! 三项写入操作**各收一个 `tx: &Tx<'_>`**，与 `continuum-graph` 的 `apply_transition`、
//! `continuum-artifact` 的 `save_artifact` 同形：它们要经 `Tx::append_audit` 写一条审计
//! 记录（设计 6.4），而事务由调用方持有——本层不自行开事务，也不提交。
//!
//! **`§318` 的「同一事务」在本层只覆盖审计行。** git / 文件系统的改动不在事务内，
//! 也无法回滚：`git apply` 改的是 Base 的工作树，`git merge` 甚至可能留下一个
//! 未完成的合并。故本层的次序与 P1 一致——**先做变更、后写审计行**——契约也一样：
//! 返回 `Err` 之后调用方必须回滚该事务，不得提交。审计行写失败时变更已经生效而库里
//! 没有记录，这一处窗口无法用事务合上，见各写入操作的文档。
//!
//! # 两种后端的 view_diff
//!
//! 「Task 相对 Base 改了什么」在两个后端上不是同一件事，各按自己的形态取：
//!
//! - **worktree 后端**：Task 是**某一个提交**（建分支时 Base 的 HEAD）的检出，
//!   故以「Task 分支与 Base 的分歧点」为基准走 git。Base 在创建之后新进的提交
//!   不是本 Task 的改动，不会反向出现在结果里（否则那些新文件看起来就像 Task
//!   删了它们）。
//! - **overlay 后端**：Task 的 lower 是**活的 Base**——Task 没碰过的文件在两边
//!   逐字节相同，故逐文件比较两棵当前树即可。Base 在创建之后被用户改动的文件同样
//!   不会出现在结果里，理由不同而结论一致：Task 读到的是**当前**的 Base。
//!
//! 两端因此在本层「不感知后端差异」的接口下给出各自正确的答案；差异写在上面，
//! 调用方按 `backend` 传参（创建时随句柄一同返回的那个），本层不重新探测。
//!
//! # 只读的含义
//!
//! `view_diff` 不在 Base 上留下任何改动，包括 `.git/`。这一条不是理所当然的：
//! `git diff <分歧点>` 会把刷新后的 stat 缓存**写回** Task worktree 的索引，而那个
//! 索引位于 `<base>/.git/worktrees/<intent>/index`，在 Base 的目录树之内。故差异经
//! [`IndexCopy`] 计算——把索引指到副本上，让这次写回落进临时文件（触发条件与实测
//! 证据见该类型的文档）。

use crate::backend::{WorkspaceBackend, discard_task_workspace};
use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::persist::backend_str;
use crate::task::TaskWorkspace;
use crate::worktree;
use continuum_events::audit::AuditKind;
use continuum_persist::{PersistError, Tx, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 让 git 把索引写到别处的环境变量（见 [`IndexCopy`]）。
const GIT_INDEX_FILE: &str = "GIT_INDEX_FILE";

/// Integration Gate：Task Workspace 与 Base Workspace 之间的唯一通道（§257）。
///
/// 只持有一个 Base：`§257` 的通道是「Task → 它的 Base」，不是任意两者之间。
#[derive(Debug)]
pub struct IntegrationGate<'a> {
    base: &'a BaseWorkspace,
}

impl<'a> IntegrationGate<'a> {
    /// 为本 Base 建一个 Gate。
    pub fn new(base: &'a BaseWorkspace) -> Self {
        Self { base }
    }

    /// 比较 Task 与 Base，列出 Task 相对 Base 的改动。只读，不收批准值。
    ///
    /// 结果的语义按后端而异，见模块文档「两种后端的 view_diff」。
    ///
    /// `backend` 由调用方给出而不是本层重新探测：判据（Base 是否为 Git 仓库）在创建
    /// 之后可能已变（用户后来才 `git init`），按新判据取 Diff 会与磁盘上实际存在的
    /// 那个工作区对不上。这与 [`discard`](Self::discard) 收 `backend` 是同一条理由。
    pub fn view_diff(
        &self,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
    ) -> Result<Diff, GateError> {
        match backend {
            WorkspaceBackend::Worktree => diff_worktree(self.base, task),
            WorkspaceBackend::Overlay => diff_trees(self.base.root(), task.root()),
        }
    }

    /// 丢弃 Task Workspace。Base 不变。只读级（L0），不收批准值。
    ///
    /// 实现交给 [`discard_task_workspace`]，本层不复制它的逻辑：overlay 后端的
    /// 「`umount` 未成功就绝不往下走」与 worktree 后端的回收次序都在那里，
    /// 从这里另走一条路等于把那几条守卫旁路掉。
    ///
    /// **与落库记录的关系**（次序，而非原子性）：本函数成功返回之后再删记录
    /// （[`crate::remove_workspace`]），返回 `Err` 时不删。要求的实质是次序而非事务
    /// ——本层的变更全在文件系统与 git 上，本来就不可回滚；调用方的事务里若还有别的
    /// 写（放弃事件、审计记录），与那些写放进同一事务才有意义。两个方向的不一致
    /// 都要避免，其中最隐蔽的一向是「有工作区无记录」：它要到同一 Intent 再次创建、
    /// 撞上仍然存在的分支或 Intent 目录时才显形。
    ///
    /// **收 `tx` 是为了审计**（设计 6.4：`discard` 也要记一条）。底层
    /// [`discard_task_workspace`] 仍然不收 `Tx`（Task 4 裁定：它只动文件系统与 git）
    /// ——层在 Gate：收下事务、在回收成功之后补记审计行的都是本函数。
    ///
    /// **审计行的落点。** 与 P1 的先变更后写事件的次序一致：回收成功之后再写审计行。
    /// 返回 `Err` 时调用方必须回滚该事务；回收本身已发生的部分（worktree 已移除而
    /// 分支未删，或 overlay 已卸载而目录未删）不在事务内，回滚不会撤销它——那些残留
    /// 按 [`discard_task_workspace`] 的文档只能手工收拾。
    pub fn discard(
        &self,
        tx: &Tx<'_>,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
        occurred_at: i64,
    ) -> Result<(), GateError> {
        discard_task_workspace(self.base, task, backend)?;
        self.audit(
            tx,
            // 「外部副作用」：`discard` 不收批准值，它不改 Base，但确实在 Runtime 的
            // 库之外留下持久改动——worktree 后端删掉用户仓库里的 `ai/<intent>` 分支，
            // overlay 后端卸载覆盖层并删除工作目录。设计 6.4 点的就是这一类。
            AuditKind::ExternalEffects,
            occurred_at,
            "discard",
            task,
            backend,
            None,
        )
    }

    /// 把 Task 的改动作为补丁应用到 Base（§257；L3，须批准）。
    ///
    /// worktree 后端经 `git -C <base> apply` 打补丁，overlay 后端把 Task 相对 Base 的
    /// 改动逐条落进 Base——两者的语义见 [`integrate_overlay`]。
    ///
    /// **补丁的基准是分歧点**，与 [`IntegrationGate::view_diff`] 同（见 [`diff_worktree`]）：
    /// Task 的改动是「相对它分出去的那个点」的，不是相对 Base 的当前 HEAD。因而用户
    /// 在创建之后又提交过时，那些提交不会被卷进来。
    ///
    /// **`git apply` 只改 Base 的工作树**（不加 `--index`）：改动落成未暂存状态，由用户
    /// 自己决定怎么提交。`apply_patch` 不替用户提交，Base 的 HEAD 与分支一概不动。
    ///
    /// **补丁对不上 Base 时以 git 的失败报出**（`GitFailed`，带退出码与 stderr）：
    /// 补丁带上下文，而 Base 的当前内容可能与分歧点已不同（用户改过那些行）。
    /// 这与 [`IntegrationGate::merge`] 的冲突是同一类处境，本层不做三方合并。
    ///
    /// **未跟踪的文件不在补丁里**（`git diff` 只看已跟踪的路径），本层另行按字节复制
    /// 过去，符号链接按目标重建。不复制的话，[`IntegrationGate::view_diff`] 报出的
    /// 「新增」会有一部分应用不上——Diff 与实际生效的集成对不上。
    ///
    /// 返回 `Err` 之后调用方必须回滚事务；但补丁若已部分应用，那部分不在事务内，
    /// 回滚不会撤销它（见模块文档「写入操作与事务」）。
    pub fn apply_patch(
        &self,
        tx: &Tx<'_>,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
        occurred_at: i64,
        approval: &GateApproval,
    ) -> Result<(), GateError> {
        let _ = approval;
        match backend {
            WorkspaceBackend::Worktree => apply_patch_worktree(self.base, task)?,
            WorkspaceBackend::Overlay => integrate_overlay(self.base.root(), task.root())?,
        }
        self.audit(
            tx,
            AuditKind::UserApprovals,
            occurred_at,
            "apply_patch",
            task,
            backend,
            None,
        )
    }

    /// 把 Task 分支上的指定提交摘到 Base 分支（§257；L3，须批准）。
    ///
    /// `commits` 是交给 `git cherry-pick` 的修订名（完整的或短的前缀哈希、`HEAD~2`
    /// 一类都行，`git rev-parse` 认得的写法即可）。它们必须能从 Base 的仓库解析出来
    /// ——Task 分支与 Base 在同一个仓库里，故 `ai/<intent>` 上的提交名都可用。
    ///
    /// **空列表由本层拒绝**（[`WorkspaceError::GateRefused`]），不落到 git：git 在这种
    /// 情形下打印用法并以 129 退出，调用方拿到的会是「git 命令失败」而不是「你没给
    /// 提交」。要把 Task 的全部改动并入 Base，用 [`IntegrationGate::apply_patch`] 或
    /// [`IntegrationGate::merge`]。
    ///
    /// **overlay 后端上本操作退化为与 [`IntegrationGate::merge`] 同一个动作**
    /// （设计 6.3）：那个后端没有提交历史，`commits` 无从兑现，并入的是 Task 的**全部**
    /// 改动。此时传进来的提交名不会让本层少并或多并任何东西——调用方若要按提交粒度
    /// 集成，只能用 worktree 后端。
    ///
    /// **摘取中途失败会留下 git 的未完成状态。** `cherry-pick` 冲突时 git 停在冲突处
    /// （`CHERRY_PICK_HEAD` 与工作树里的冲突标记都在），本层不代 git 收尾：自动
    /// `--abort` 会把用户可能已经开始的冲突解决一并丢掉。错误原样报出，收拾由用户
    /// 用 `git cherry-pick --continue` / `--abort` 完成。
    pub fn cherry_pick(
        &self,
        tx: &Tx<'_>,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
        commits: &[&str],
        occurred_at: i64,
        approval: &GateApproval,
    ) -> Result<(), GateError> {
        let _ = approval;
        if commits.is_empty() {
            return Err(WorkspaceError::GateRefused {
                reason: "cherry_pick 未给出任何提交。要把 Task 的全部改动并入 Base，\
                         用 apply_patch 或 merge"
                    .to_owned(),
            }
            .into());
        }
        match backend {
            WorkspaceBackend::Worktree => cherry_pick_worktree(self.base, commits)?,
            WorkspaceBackend::Overlay => integrate_overlay(self.base.root(), task.root())?,
        }
        self.audit(
            tx,
            AuditKind::UserApprovals,
            occurred_at,
            "cherry_pick",
            task,
            backend,
            Some(&format!("commits={}", commits.join(","))),
        )
    }

    /// 把 Task 分支并入 Base 的当前分支（§257；L3，须批准）。
    ///
    /// worktree 后端经 `git -C <base> merge --no-edit ai/<intent>`；overlay 后端把 Task
    /// 相对 Base 的改动逐条落进 Base（与 [`IntegrationGate::apply_patch`] 同一个动作，
    /// 见 [`integrate_overlay`]）。
    ///
    /// `--no-edit`：合并提交的报文取 git 的默认值，不为此打开编辑器——本层在
    /// 非交互环境里运行，编辑器打不开时合并会停在那里。
    ///
    /// **允许快进。** Task 分支是 Base 当前 HEAD 的后代时（用户建好工作区后没再提交），
    /// git 直接把分支指针前移，不产生合并提交。要求是「Task 的改动进入 Base」，
    /// 快进满足它；强制 `--no-ff` 只会凭空造出一个空合并提交。
    ///
    /// **冲突时不代 git 收尾**，同 [`IntegrationGate::cherry_pick`]：git 停在冲突处，
    /// 错误原样报出，由用户 `git merge --continue` / `--abort` 收拾。
    pub fn merge(
        &self,
        tx: &Tx<'_>,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
        occurred_at: i64,
        approval: &GateApproval,
    ) -> Result<(), GateError> {
        let _ = approval;
        match backend {
            WorkspaceBackend::Worktree => merge_worktree(self.base, task)?,
            WorkspaceBackend::Overlay => integrate_overlay(self.base.root(), task.root())?,
        }
        self.audit(
            tx,
            AuditKind::UserApprovals,
            occurred_at,
            "merge",
            task,
            backend,
            None,
        )
    }

    /// 把一次写入操作记入审计（设计 6.4）。
    ///
    /// 只写一条记录，不做别的：`kind` 由调用方给出（各操作的归类写在各调用点），
    /// `occurred_at` 也由调用方给出——本层不取时钟，与 [`crate::WorkspaceRecord`]
    /// 的 `created_at` 同一条理由（落库时刻与业务时刻是两件事，隐式取时间会让调用方
    /// 无从控制，也无从在测试里固定）。
    ///
    /// payload 是**给人读的一行文本**（`Value::text` 不是合法 JSON 时 `append_audit`
    /// 会把它当字符串存下）：操作名、Intent、后端、Base 根，必要时加一项细节。
    /// 不写成 JSON 对象是为了不为此引入 `serde_json` ——本 crate 至今没有它，而这一行
    /// 的内容不带任何需要转义的嵌套结构。
    fn audit(
        &self,
        tx: &Tx<'_>,
        kind: AuditKind,
        occurred_at: i64,
        operation: &str,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
        detail: Option<&str>,
    ) -> Result<(), GateError> {
        let payload = format!(
            "{operation} intent={} backend={} base={}{}",
            task.intent_id().as_str(),
            backend_str(backend),
            self.base.root().display(),
            match detail {
                Some(detail) => format!(" {detail}"),
                None => String::new(),
            }
        );
        tx.append_audit(kind, occurred_at, Value::text(payload))?;
        Ok(())
    }
}

/// 集成修改的批准值。
///
/// **无公开构造函数**：字段私有，也没有 `new` 一类的关联函数。写入操作（Task 9）
/// 一律收 `&GateApproval`，使「从隔离工作空间集成回用户分支需要授权」有落点
/// （`§16`、设计 6.2）——缺少它时 Gate 退化为「谁调用谁生效」。
///
/// 本子项目内由驱动的显式确认参数产生；Capability 与 Authority 就位后，其产生点
/// 移交给那一处。只读操作（[`IntegrationGate::view_diff`]、[`IntegrationGate::discard`]）
/// 不收本值：它们不动 Base。
#[derive(Debug)]
pub struct GateApproval(());

/// Task 相对 Base 的改动（`§257` 的 view diff）。
///
/// 三类，各自一组**相对路径**——基准是 Workspace 根（Task 根与 Base 根在布局上
/// 同构，两个后端的 `root()` 就是同一层的目录）。构造即规范化：组内按路径排序，
/// 且不含重复项。
///
/// **三类即全部。** 更细的分类在两种后端上并不都对得上：overlay 后端只有逐文件
/// 比较，没有 git 的重命名概念。故统一归入这三类——git 侧的重命名经 `--no-renames`
/// 拆成一「删除」一「新增」，类型变更（普通文件与符号链接互改）归入「修改」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    added: Vec<PathBuf>,
    modified: Vec<PathBuf>,
    deleted: Vec<PathBuf>,
}

impl Diff {
    /// 只在 Task 里有的文件。
    pub fn added(&self) -> &[PathBuf] {
        &self.added
    }

    /// 两侧都有、但内容不同的文件。
    pub fn modified(&self) -> &[PathBuf] {
        &self.modified
    }

    /// 只在 Base 里有的文件：在 Task 侧被删掉了。
    pub fn deleted(&self) -> &[PathBuf] {
        &self.deleted
    }

    /// 三类都为空：Task 相对 Base 没有任何改动。
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// 由三个集合构造。用 `BTreeSet` 而非 `Vec` 是让「排序且无重复」由类型保证，
    /// 不必在构造处再检查一遍。
    fn from_sets(
        added: BTreeSet<PathBuf>,
        modified: BTreeSet<PathBuf>,
        deleted: BTreeSet<PathBuf>,
    ) -> Self {
        Self {
            added: added.into_iter().collect(),
            modified: modified.into_iter().collect(),
            deleted: deleted.into_iter().collect(),
        }
    }
}

/// Integration Gate 的错误。
///
/// 与 [`WorkspaceError`] 分开而不是取别名：后者的变体描述**后端**（worktree / overlay）
/// 为何失败，本类型的变体描述**Gate 这一层**拿到了什么。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GateError {
    /// 底层 Workspace 操作失败：后端不可用、文件系统读写失败、git 命令失败等。
    ///
    /// 原样透传，调用方仍按 [`WorkspaceError`] 的变体分派。把后端的具体原因折叠成
    /// 本层的一个字符串，会让「git 究竟因何失败」（退出码、stderr）无从读取。
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    /// 审计记录的落库失败（设计 6.4：三项写入操作与 `discard` 各写一条）。
    ///
    /// 与 [`GateError::Workspace`] **分开**：那个变体的语义是「后端（worktree /
    /// overlay）为何失败」，本变体来自数据库，与后端无关。调用方按变体分派时，
    /// 两者混为一谈会让「后端失败了」这一分支凭空多出一类数据库来源。
    ///
    /// 本变体报出时**Base 的改动已经生效**（本层先变更、后写审计），调用方回滚事务
    /// 不会撤销它，见模块文档「写入操作与事务」。
    #[error(transparent)]
    Persist(#[from] PersistError),
    /// 后端给出的结果无法解释，或后端以本层读不出的方式失败。
    ///
    /// 两处来源：
    /// - git 在 `--name-status` 里给出了本层不认识的状态码。这种情况下**不能丢弃那
    ///   一条**——Diff 是三种写入操作的依据，少列一条意味着一处改动调用方看不见，
    ///   而它可能是要在 Base 上生效的那个；
    /// - `git merge-base` 以退出码 1、空 stderr 失败（两棵树没有共同祖先）。那个失败
    ///   经 [`WorkspaceError::GitFailed`] 原样透出时是一句没有内容的报文，看不出发生了什么。
    #[error("Integration Gate 无法解释后端给出的结果：{reason}")]
    Malformed { reason: String },
}

/// worktree 后端的 view_diff：以分歧点为基准走 git。
///
/// 三段合起来是「分歧点 → Task 工作树」的全部改动：
/// 1. `git diff <分歧点>`：已提交在 Task 分支上的、已 `git add` 的、以及工作树里
///    未 `git add` 的改动，git 自己合成为一份逐路径的状态；
/// 2. `git ls-files --others --exclude-standard`：未跟踪文件——它们不在 `git diff`
///    的输出里（那条命令只看已跟踪的路径），一律计为「新增」。被 `.gitignore` 忽略
///    的不计（`--exclude-standard`）：三种写入操作都经 git 落到 Base 上，而 git
///    本来就不会带上被忽略的文件；
/// 3. 两者都是相对**同一个分歧点**的，故同名路径不会在两处各出现一次。
///
/// **不用 `git status` 拼**：status 的基准是 Task 分支的 HEAD（而不是分歧点），
/// 两者要另行合成，而合成在「某路径在提交里新增、又在工作树里被删掉」这类组合上
/// 有分歧点，凭空多出一张需要逐项验证的对照表。`git diff <分歧点>` 直接给出
/// git 自己的合成结果。
///
/// **基准取分歧点而非 Base 的当前 HEAD。** 用户在建好工作区之后又提交过时，那些
/// 提交里的新文件在 Task 侧并不存在；以 Base 当前 HEAD 为基准会把它们**反向**算进
/// 本 Task（看起来像 Task 删了它们）。
///
/// 已知限制两种形态，都源于「本后端不记录创建点」（句柄与落库记录里都没有它）：
/// - 用户把 Base 切到创建点**之前**的提交或另一条分支上时，`merge-base` 会落到更早处，
///   Diff 因而多算；
/// - Base 的当前 HEAD 与 Task 分支**没有共同祖先**时（用户另起了一条无关历史，
///   `git checkout --orphan` 一类），`merge-base` 不是给出别的提交而是**直接失败**，
///   本函数以 [`WorkspaceError::GitFailed`] 报出，Diff 取不到。
///
/// 两种都只能靠记下创建时的那个提交来根治，属落库记录的范围。
fn diff_worktree(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<Diff, GateError> {
    let divergence = divergence_point(base, task)?;

    // 索引指到副本：见 IndexCopy 的文档（git 会刷新并写回索引，而索引在 Base 之内）。
    let index = IndexCopy::for_task(task.root())?;
    let envs: [(&str, &OsStr); 1] = [(GIT_INDEX_FILE, index.path().as_os_str())];
    let changed = worktree::git_raw(
        task.root(),
        &["diff", "--no-renames", "--name-status", "-z", &divergence],
        &envs,
    )?;
    let untracked = worktree::git_raw(
        task.root(),
        &["ls-files", "--others", "--exclude-standard", "-z"],
        &envs,
    )?;

    let mut added = BTreeSet::new();
    let mut modified = BTreeSet::new();
    let mut deleted = BTreeSet::new();
    for (kind, path) in parse_name_status(&changed)? {
        let set = match kind {
            ChangeKind::Added => &mut added,
            ChangeKind::Modified => &mut modified,
            ChangeKind::Deleted => &mut deleted,
        };
        set.insert(path);
    }
    for path in parse_paths(&untracked) {
        added.insert(path);
    }
    Ok(Diff::from_sets(added, modified, deleted))
}

/// Task 分支与 Base **当前 HEAD** 的分歧点（共同祖先）。
///
/// view_diff 与 apply_patch 都以它为基准（见 [`diff_worktree`] 的文档），故取法只有
/// 这一处：两处各取一次会让「基准是什么」有两个出处，改一处忘一处时 Diff 与实际补丁
/// 的基准会静默分叉。
///
/// 失败有两种形态，都在 [`WorkspaceError::GitFailed`] 之外另行说明：
/// - 两棵树没有共同祖先时（用户另起了一条无关历史），`git merge-base` 以退出码 1
///   且 **stderr 为空**失败（实测 git 2.56）——原样透出的话，调用方拿到的是一句
///   「git 命令失败（退出码 1）：」后面什么都没有，看不出发生了什么。这里报
///   [`GateError::Malformed`]；
/// - 别的失败都带 stderr（分支不存在是 128 + "Not a valid object name"），原样透传。
fn divergence_point(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<String, GateError> {
    let branch = worktree::branch_name(task.intent_id());
    let divergence = match worktree::git(base.root(), &["merge-base", &branch, "HEAD"]) {
        Ok(hash) => hash,
        Err(WorkspaceError::GitFailed { code: 1, stderr }) if stderr.is_empty() => {
            return Err(GateError::Malformed {
                reason: format!(
                    "Task 分支 {branch} 与 Base 的当前 HEAD 没有共同祖先，取不到分歧点\
                     （用户另起了一条无关历史？）"
                ),
            });
        }
        Err(e) => return Err(e.into()),
    };
    if divergence.is_empty() {
        // 理论上不可达：`merge-base` 成功返回时必有输出（失败时是 `GitFailed`）。
        // 留着是因为空值传给 git 会以「未知的修订」失败，那个原因与本层的真实处境
        // （没拿到分歧点）对不上，报出来只会误导。
        return Err(GateError::Malformed {
            reason: format!("git 未给出 Task 分支 {branch} 与 Base 的分歧点"),
        });
    }
    Ok(divergence)
}

/// worktree 后端的 `apply_patch`：补丁 + 未跟踪文件，两点次序见下。
///
/// **补丁部分**：`git -C <task> diff --binary --no-renames <分歧点>` 给出「分歧点 →
/// Task 工作树」的全部**已跟踪**改动（已提交的、已 `git add` 的、工作树里未 add 的，
/// git 自己合成），经 stdin 送进 `git -C <base> apply`。
///
/// - `--binary`：不加的话二进制文件的改动在补丁里只留一句「Binary files differ」，
///   `git apply` 无从应用；
/// - `--no-renames`：与 view_diff 取同一形态（重命名拆成一删一增）。补丁里带
///   `rename from/to` 也能应用，但那会让本层与 overlay 后端对同一件事给出不同的
///   分节方式，而两个后端的 Diff 已经统一成三类；
/// - 索引经 [`IndexCopy`] 指到副本：`git diff` 会刷新 stat 缓存并写回索引，而 Task
///   worktree 的索引在 Base 的目录树之内。写入操作本来就动 Base，但**动的应当是
///   Base 的内容，而不是 Task 的索引**——那是另一回事，且调用方看不见。
///
/// **未跟踪文件部分**：`git diff` 不看未跟踪的路径，故另取
/// `git ls-files --others --exclude-standard -z`（与 view_diff 同一判据：被忽略的
/// 不算数，三种写入操作都经 git 落到 Base 上），逐个从 Task 根复制到 Base 根。
/// 不做这一步的话，view_diff 报出的「新增」里有一部分应用不上。
///
/// **次序**：先打补丁、后复制未跟踪文件。反过来的话，一个在 Task 侧是未跟踪文件、
/// 在补丁里又出现同名条目是不可能的（补丁只含已跟踪路径），故两者不相交，次序只
/// 影响失败时留下了什么：先补丁意味着失败时 Base 上多出的是「已跟踪文件的改动」，
/// 那更容易被 `git status` 看见、也更容易被用户回退。
fn apply_patch_worktree(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<(), GateError> {
    let divergence = divergence_point(base, task)?;
    let index = IndexCopy::for_task(task.root())?;
    let envs: [(&str, &OsStr); 1] = [(GIT_INDEX_FILE, index.path().as_os_str())];

    let patch = worktree::git_raw(
        task.root(),
        &["diff", "--binary", "--no-renames", &divergence],
        &envs,
    )?;
    // 空补丁不送给 git：`git apply` 在空输入下以「unrecognized input」失败，而
    // 「Task 只改了未跟踪文件」是正常情形，不是错误。
    if !patch.is_empty() {
        worktree::git_with_stdin(base.root(), &["apply"], &patch)?;
    }

    let untracked = worktree::git_raw(
        task.root(),
        &["ls-files", "--others", "--exclude-standard", "-z"],
        &envs,
    )?;
    for rel in parse_paths(&untracked) {
        copy_entry(task.root(), base.root(), &rel)?;
    }
    Ok(())
}

/// worktree 后端的 `cherry_pick`：把修订名交给 `git cherry-pick`，逐个按序摘取。
///
/// 不自己解析修订、也不逐个提交地循环调用 git 一次一个：`git cherry-pick A B` 本身就是
/// 「按序摘取」的语义，且串联失败时报出的是**那一个**提交的冲突。逐个循环反而要在本层
/// 维护「摘到第几个失败」的状态，而 git 的 sequencer 已经在维护它。
///
/// 空列表不会到这里（[`IntegrationGate::cherry_pick`] 在此之前拒绝）。
fn cherry_pick_worktree(base: &BaseWorkspace, commits: &[&str]) -> Result<(), GateError> {
    let mut args = Vec::with_capacity(commits.len() + 1);
    args.push("cherry-pick");
    args.extend_from_slice(commits);
    worktree::git(base.root(), &args)?;
    Ok(())
}

/// worktree 后端的 `merge`：把 `ai/<intent>` 并入 Base 的当前分支。
///
/// `--no-edit`：合并提交的报文取默认值，不打开编辑器（见 [`IntegrationGate::merge`]）。
fn merge_worktree(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<(), GateError> {
    let branch = worktree::branch_name(task.intent_id());
    worktree::git(base.root(), &["merge", "--no-edit", &branch])?;
    Ok(())
}

/// overlay 后端的三项写入操作：把 Task 相对 Base 的改动（[`Diff`]）落进 Base。
///
/// 设计 6.3 把这个后端上的 `cherry_pick` 与 `merge` 定为同一个动作——「把 upper 层的
/// 内容合并进 Base」；`apply_patch` 在这个后端上也只能是同一件事（没有提交，也没有
/// 补丁的基准点）。三项共用本函数。
///
/// **改动集合取自 [`diff_trees`]，而不是直接读 upper 层。** 两者在「内容」上等价
/// （upper 遮蔽 lower，故合并视图里就是 Task 的最终内容），差别在删除：upper 层表达
/// 删除用的是白障（0/0 的字符设备），直接读 upper 就得自己识别白障；而逐文件比较
/// Base 与 Task 的**当前**视图，Task 侧读不到的条目自然就是删除。这样还有一个附带
/// 的好处：**集成的内容与 [`IntegrationGate::view_diff`] 报出的 Diff 是同一个集合**
/// ——用户看到的差异就是会落进 Base 的东西，两者不会各说各话。
///
/// 先删后增：两侧不会对同一路径既报删除又报新增（`diff_trees` 按「该路径在两侧是否
/// 存在」分三类），故次序只影响中途失败时留下了什么。先删的话，中途失败留下的是一棵
/// 「少了一些东西」的 Base；先增的话，是一个「多了一些东西」的 Base——后者更容易让
/// 用户误以为改动已经全部生效，故先删。
fn integrate_overlay(base_root: &Path, task_root: &Path) -> Result<(), GateError> {
    let diff = diff_trees(base_root, task_root)?;
    for rel in diff.deleted() {
        remove_entry(&base_root.join(rel))?;
    }
    for rel in diff.added().iter().chain(diff.modified()) {
        copy_entry(task_root, base_root, rel)?;
    }
    Ok(())
}

/// 把 `rel` 处的条目从 `from_root` 复制到 `to_root`：常规文件按字节，符号链接按目标
/// 重建（与 [`collect_files`] 对符号链接的看法一致——它本身是一个条目，不是通往目标的
/// 门）。非常规条目（FIFO、套接字、设备）跳过：`collect_files` 不把它们计入 Diff，
/// 故正常情况下到不了这里；真到了（两侧树在此期间被改动）也不该去读它——读一个没有
/// 写者的 FIFO 会阻塞。
///
/// 目标处已有条目时**先删再写**，不就地覆盖：就地写会**穿过**那里的符号链接落到 Base
/// 之外（一个指向 `/etc` 的链接会把文件写进 `/etc`），而删掉的是链接本身。
fn copy_entry(from_root: &Path, to_root: &Path, rel: &Path) -> Result<(), GateError> {
    let src = from_root.join(rel);
    let dst = to_root.join(rel);
    let meta = std::fs::symlink_metadata(&src).map_err(|e| io_error(&src, e))?;

    create_parent_dirs(to_root, rel)?;
    remove_entry(&dst)?;

    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(&src).map_err(|e| io_error(&src, e))?;
        std::os::unix::fs::symlink(&target, &dst).map_err(|e| io_error(&dst, e))?;
    } else if meta.is_file() {
        let bytes = std::fs::read(&src).map_err(|e| io_error(&src, e))?;
        std::fs::write(&dst, &bytes).map_err(|e| io_error(&dst, e))?;
    }
    Ok(())
}

/// 保证 `rel` 的各级父目录在 `to_root` 之下都是**真正的目录**。
///
/// 这一条是边界层的事，不是形式检查：Base 在某一级是符号链接（`sub -> /etc`）而 Task
/// 侧那一级是目录时（overlay 的 upper 层里的目录遮蔽了 lower 的链接），直接往
/// `to_root/rel` 写会**穿过**那个链接落到 Base 之外。故遇到链接或普通文件就删掉它、
/// 改建成目录——那正是「Task 侧的目录遮蔽 Base 的同名条目」应有的结果。
///
/// 反过来，若只在最后一级 `remove_entry`，中间几级的链接就把写操作引到了别处。
fn create_parent_dirs(to_root: &Path, rel: &Path) -> Result<(), GateError> {
    let Some(parent) = rel.parent() else {
        return Ok(());
    };
    let mut current = to_root.to_path_buf();
    for component in parent.components() {
        current.push(component);
        // `symlink_metadata` 不跟随链接：指向目录的链接在这里是「链接」而不是「目录」，
        // 正是要判的那件事。
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                std::fs::remove_file(&current).map_err(|e| io_error(&current, e))?;
                std::fs::create_dir(&current).map_err(|e| io_error(&current, e))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current).map_err(|e| io_error(&current, e))?;
            }
            Err(e) => return Err(io_error(&current, e)),
        }
    }
    Ok(())
}

/// 删掉 `path` 处的条目；它不存在时什么也不做（幂等）。
///
/// **是目录就拒绝**，不递归删除：Diff 只以文件与符号链接为单位（见 [`collect_files`]），
/// 一个目录出现在这里说明 Base 与 Task 在那一级的**种类**不同（Task 侧是文件、Base 侧
/// 是目录）。递归删掉这个目录会连带删掉里面本层没看过的东西；报出来让调用方处置。
fn remove_entry(path: &Path) -> Result<(), GateError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Err(WorkspaceError::GateRefused {
            reason: format!(
                "{} 是目录，而 Task 侧对应的是文件：本层不删除目录（会连带删掉里面本层\
                 没有看过的条目）",
                path.display()
            ),
        }
        .into()),
        Ok(_) => std::fs::remove_file(path).map_err(|e| io_error(path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(path, e)),
    }
}

/// overlay 后端的 view_diff：逐文件比较两棵树。
///
/// Task 根是覆盖层的挂载点，读到的是 lower（Base）与 upper 的合并视图，故这里不必
/// 感知 overlay，两边都当普通目录遍历：同名改动（upper 遮蔽 lower）体现为「内容
/// 不同」，删除（upper 里的白障）体现为「Task 侧读不到」，与三类改动正好对上。
fn diff_trees(base_root: &Path, task_root: &Path) -> Result<Diff, GateError> {
    let base_files = collect_files(base_root)?;
    let task_files = collect_files(task_root)?;

    let mut added = BTreeSet::new();
    let mut modified = BTreeSet::new();
    let mut deleted = BTreeSet::new();
    for (path, task_bytes) in &task_files {
        match base_files.get(path) {
            None => {
                added.insert(path.clone());
            }
            Some(base_bytes) if base_bytes != task_bytes => {
                modified.insert(path.clone());
            }
            Some(_) => {}
        }
    }
    for path in base_files.keys() {
        if !task_files.contains_key(path) {
            deleted.insert(path.clone());
        }
    }
    Ok(Diff::from_sets(added, modified, deleted))
}

/// `root` 之下全部条目的相对路径 → 内容。内容按条目种类取：常规文件是它的字节，
/// 符号链接是它的目标路径（见下）。
///
/// **不收目录**：三类改动以文件为单位。目录在两端会平白分岔——`git` 根本不跟踪目录
/// （`mkdir` 出来的空目录在 `git diff` 里不出现），把目录计入会让 worktree 与 overlay
/// 两个后端对同一件事给出不同答案。
///
/// **符号链接不跟随**，按目标路径本身比较（`read_link`）。这与 worktree 后端的 git
/// 一致：git 把符号链接当作一个条目，内容即其目标，从不跟进目标里头。跟随会走进两个
/// 坑：一是指向目录的链接被递归进去，链接指向树内某处时同一批文件在 Diff 里出现两次；
/// 二是**自指或互指的链接构成环**，跟随会一路递归到内核的 `ELOOP`（Linux 上 40 层）
/// 才停下——实测一个指向自己所在目录的链接正是如此，报出来的错误里带着几十段重复路径。
/// 不跟随让环根本无从进入：环上的链接就是一个普通条目，读它的目标不触碰文件系统，
/// 故这里不需要另设深度上限。
///
/// **非常规条目（FIFO、套接字、设备文件）不进结果**：git 也不跟踪它们，而读一个 FIFO
/// 会阻塞。覆盖层 upper 层里的白障正是这类条目，故这一条同时让白障天然不成问题。
///
/// 读取失败返回 `Err` 而不跳过：跳过会让 Diff 少列一条而调用方无从得知，而 Diff 是
/// 写入操作的依据。
fn collect_files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, GateError> {
    fn walk(
        root: &Path,
        rel: &Path,
        out: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), GateError> {
        let entries = std::fs::read_dir(root).map_err(|e| io_error(root, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| io_error(root, e))?;
            let path = entry.path();
            // 相对路径由父级的相对路径拼出，不靠事后 strip_prefix：遍历的起点就是
            // 相对路径的基准，拼比减少一层「前缀对不上怎么办」。
            let rel = rel.join(entry.file_name());
            // `DirEntry::file_type` 取自 `read_dir` 本身且**不跟随**符号链接，
            // 故「这是个链接」与「链接指向目录」在这里是两件事。
            let file_type = entry.file_type().map_err(|e| io_error(&path, e))?;
            if file_type.is_dir() {
                walk(&path, &rel, out)?;
            } else if file_type.is_symlink() {
                let target = std::fs::read_link(&path).map_err(|e| io_error(&path, e))?;
                // 目标按原始字节存：非 UTF-8 的目标路径经 lossy 转换后，两个不同的
                // 目标可能比成相同，Diff 会漏报一处改动。
                out.insert(rel, target.into_os_string().into_vec());
            } else if file_type.is_file() {
                let bytes = std::fs::read(&path).map_err(|e| io_error(&path, e))?;
                out.insert(rel, bytes);
            }
        }
        Ok(())
    }

    let mut out = BTreeMap::new();
    walk(root, Path::new(""), &mut out)?;
    Ok(out)
}

/// 一条改动的类别。`Diff` 的三类各对应一个集合，本类型只在解析中途用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

/// `git diff --name-status -z` 的原始输出 → 逐条改动。
///
/// 输出是 `状态\0路径\0` 的重复。**用 `-z` 而不是逐行读**：不加 `-z` 时 git 会把
/// 非 ASCII 与含特殊字符的路径转义成 `"\346\226\207"` 一类带引号的形式，本层就得
/// 自己实现一遍反转义；`-z` 给的是原始字节。同理这里收 `&[u8]` 而非 `&str`：
/// 非 UTF-8 的文件名（Linux 上合法）经 `from_utf8_lossy` 会被替换字符顶掉，
/// 还原出来的就是另一个路径。
///
/// `--no-renames` 已让 git 把重命名拆成一删一增，故状态只有四种。别的一律返回
/// `Err`（见 [`GateError::Malformed`]）：丢弃会让 Diff 少列一条。
fn parse_name_status(bytes: &[u8]) -> Result<Vec<(ChangeKind, PathBuf)>, GateError> {
    let mut fields = bytes.split(|b| *b == 0);
    let mut out = Vec::new();
    while let Some(status) = fields.next() {
        // 末尾的 NUL 之后是一个空字段（输入为空时也只有这一个），至此结束。
        if status.is_empty() {
            break;
        }
        let kind = match status {
            b"A" => ChangeKind::Added,
            b"M" => ChangeKind::Modified,
            // 类型变更：普通文件与符号链接（或子模块）互改。同一个路径上内容不同，
            // 归入「修改」。三类里没有「类型」这一类，而它确实是一处要带进 Base 的改动。
            b"T" => ChangeKind::Modified,
            b"D" => ChangeKind::Deleted,
            other => {
                return Err(GateError::Malformed {
                    reason: format!(
                        "git diff --name-status 给出了本层不认识的状态：{:?}",
                        String::from_utf8_lossy(other)
                    ),
                });
            }
        };
        // 路径字段可能是「缺字段」也可能是「空字段」：git 的路径都相对仓库根，至少有
        // 一个分量，两者都说明这条记录不完整——不接受的代价是多出一条空路径的改动。
        let path = match fields.next() {
            Some(path) if !path.is_empty() => path,
            _ => {
                return Err(GateError::Malformed {
                    reason: "git diff --name-status 的记录里状态之后没有路径".to_owned(),
                });
            }
        };
        out.push((kind, PathBuf::from(OsString::from_vec(path.to_vec()))));
    }
    Ok(out)
}

/// `git ls-files -z` 一类的输出 → 路径列表（NUL 分隔，末尾有一个 NUL 或没有）。
fn parse_paths(bytes: &[u8]) -> Vec<PathBuf> {
    bytes
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .map(|field| PathBuf::from(OsString::from_vec(field.to_vec())))
        .collect()
}

/// `view_diff` 期间，让 git 的索引写入落到副本上。
///
/// `git diff <分歧点>` 在需要刷新索引时会**写回**索引。Task worktree 的索引位于
/// `<base>/.git/worktrees/<intent>/index`，在 Base 的目录树之内——而 `view_diff`
/// 是只读操作，不该在 Base 上留下任何改动（`§256`）。实测本机 git 2.56：
/// `GIT_OPTIONAL_LOCKS=0` 也照写（那个变量挡住的是 `git status`，不是 `git diff`）。
///
/// 触发条件**不是**「工作树里有改动」，恰恰相反：内容真的变了的条目刷新之后仍是脏的，
/// 没有可写的东西；只有**stat 缓存过期而重新哈希后内容未变**的条目会被写回新的 stat。
/// 实测四种情形：改 `a.txt` 的内容——不写；把 `b.txt` 按原内容重写或 `touch` 它——
/// 写；新增未跟踪的 `c.txt`——不写。故这条路径要靠专门的用例才走得到
/// （`tests/gate.rs` 的 `view_diff_does_not_write_the_base_when_the_stat_cache_is_stale`，
/// 它带一条用真实索引的对照臂），而它会走得到：工作区里的文件被工具重写、被检出、
/// 被 `touch`，stat 缓存过期是常态。
///
/// 处置是把真实索引复制到临时文件，再用 [`GIT_INDEX_FILE`] 把它指给 git：git 读到的
/// 索引内容与用真实索引时逐字节相同，故差异的算法与结果都不变，而刷新后的写回落进
/// 这个副本，随本值一并删除。
///
/// # 副本的时间戳必须与真实索引一致
///
/// **只复制内容是不够的。** git 判定一个条目是否改动，先比 stat（mtime、ctime、大小、
/// inode…），比中了就用索引里记的那个 blob，**根本不读文件**。挡住这条捷径的是它自己的
/// 「racy」规则：条目 mtime 不早于**索引文件自身**的 mtime 时，stat 不可信，git 才去读
/// 内容。故索引文件的 mtime 是这套判定的一个输入。
///
/// 副本是新建的文件，mtime 是此刻——比它里面记的每个条目都新，于是**每个条目都不 racy**，
/// 那条捷径对全树生效。这与真实索引的行为不同，而且差在要害上：**同一时刻（临时目录
/// 的时钟粒度约十几毫秒）按同样长度改写内容**的文件，stat 与索引里的记录逐项相同，
/// git 会认为它没变。实测（git 2.56，`tests/gate.rs` 的
/// `the_index_copy_does_not_hide_a_same_size_rewrite`）：真实索引报得出来，副本索引漏报。
/// 漏报对 `view_diff` 是少列一处改动，对 `apply_patch` 是**那处改动不会被应用**。
///
/// 故副本建成后把 mtime 拨回真实索引的那一刻：`is_racy_timestamp` 的输入由此与用真实
/// 索引时逐项相同，判定也就逐项相同。拨回的是**真实索引的** mtime（不是某个条目或
/// 什么常量）——要的是「与真实索引等价」，不是「更保守」。
///
/// 副本建成后即与真实索引无关——本层是单线程调用，两次 git 之间没有让出点
/// （无 await、无线程、无回调把控制权交回调用方），真实索引不会在此期间被改动。
struct IndexCopy {
    path: PathBuf,
}

impl IndexCopy {
    /// 为 `task_root` 这个 worktree 建索引副本。
    fn for_task(task_root: &Path) -> Result<Self, GateError> {
        // 索引路径问 git 要（`--git-path` 会把 per-worktree 的路径算对），不硬拼
        // `<base>/.git/...`：Base 本身是 linked worktree 时 `.git` 是**文件**。
        // 输出可能是相对路径，故一律以 `task_root.join` 归一（`join` 遇绝对路径会
        // 直接采用它）——与 worktree 后端取 `info/exclude` 时同一条理由。
        let reported = worktree::git(task_root, &["rev-parse", "--git-path", "index"])?;
        Self::from_index(&task_root.join(reported))
    }

    /// 复制 `real_index`。索引不存在时建一个**空**副本：git 把「索引不存在」与
    /// 「索引为空」视作同一件事，本层照同一个看法处理。
    ///
    /// 副本的 mtime 拨回真实索引的那一刻（见本类型的文档「副本的时间戳必须与真实索引
    /// 一致」）。读不到真实索引的 mtime 时仍然照常复制：那种情形下副本的 mtime 是此刻，
    /// 即回到「每个条目都不 racy」的老样子——差异可能漏报一处（同一时刻按同样长度
    /// 改写的文件），但操作本身照常进行，不该因为一次 `stat` 失败而整个失败。
    fn from_index(real_index: &Path) -> Result<Self, GateError> {
        let bytes = match std::fs::read(real_index) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(io_error(real_index, e)),
        };
        let mtime = std::fs::metadata(real_index)
            .and_then(|meta| meta.modified())
            .ok();
        // 名字撞了就换一个。排他由 `create_new` 保证（`O_CREAT|O_EXCL`：路径已存在
        // ——包括它是一个符号链接——即失败），故不会覆盖临时目录里别人的文件。
        for _ in 0..8 {
            let path = temp_index_path();
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                // 索引里有用户仓库的路径清单，副本按属主可读可写建，不留在临时目录
                // 里给别人看。
                .mode(0o600)
                .open(&path)
            {
                Ok(mut file) => {
                    file.write_all(&bytes).map_err(|e| io_error(&path, e))?;
                    // 写完再拨 mtime：先拨的话内容还没写完，写入本身会把 mtime 顶到此刻。
                    if let Some(mtime) = mtime
                        && let Err(e) = file.set_modified(mtime)
                    {
                        // 拨不动（临时目录的文件系统不支持？）不是复制失败：副本的内容
                        // 是对的，只是差异可能漏报一处。报出来比静默好。
                        return Err(io_error(&path, e));
                    }
                    return Ok(Self { path });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io_error(&path, e)),
            }
        }
        Err(GateError::Workspace(WorkspaceError::BackendUnavailable {
            reason: "临时目录里取不到不冲突的索引副本名（连试 8 次都已被占用）".to_owned(),
        }))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for IndexCopy {
    /// 用完即删。删除失败不报错（析构里没有可返回错误的地方，且残留只在临时目录里，
    /// 对 Base 与后续操作都没有影响）。
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 临时目录里的一个候选路径。
///
/// 进程号 + 纳秒使不同进程、同一进程内的不同次调用都不会撞名；真正的排他交给
/// `create_new`（见 [`IndexCopy::from_index`]），本函数只管给出一个尽量不冲突的名字。
fn temp_index_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "continuum-gate-index-{}-{nanos}",
        std::process::id()
    ))
}

fn io_error(path: &Path, e: std::io::Error) -> GateError {
    GateError::Workspace(WorkspaceError::IoFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 本进程的有效 uid 是否为 0。root 无视文件权限位，故权限类用例要先问一句。
    ///
    /// 与 [`crate::overlay`] 里读 `/proc/self/status` 的写法同源（同一份判定在
    /// `tests/backend_overlay.rs` 里还有一份，那是另一个 crate）。
    fn running_as_root() -> bool {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| {
                status.lines().find_map(|line| {
                    let rest = line.strip_prefix("Uid:")?;
                    rest.split_whitespace().nth(1)?.parse::<u32>().ok()
                })
            })
            == Some(0)
    }

    /// 拼一条 `--name-status -z` 的记录：`状态\0路径\0`。
    fn record(status: &str, path: &str) -> Vec<u8> {
        let mut out = status.as_bytes().to_vec();
        out.push(0);
        out.extend_from_slice(path.as_bytes());
        out.push(0);
        out
    }

    /// `--name-status -z` 的四种状态各归到哪一类；字节序列按 git 的实际输出形状摆。
    ///
    /// 这是 [`parse_name_status`] 分类表的唯一直接证据：真实仓库里造出「类型变更」
    /// 要额外造符号链接，而状态码到类别的映射本身就是一张表，用字面输入钉住更直接。
    /// 实测（git 2.56）`git diff --no-renames --name-status -z` 对普通文件改成符号链接
    /// 给的正是一个 `T` 记录，故这条不是凭空设想的状态。
    #[test]
    fn the_name_status_parser_maps_every_status_git_can_give() {
        let cases: [(&str, ChangeKind, &str); 4] = [
            ("A", ChangeKind::Added, "新文件.txt"),
            ("M", ChangeKind::Modified, "改过的.txt"),
            ("D", ChangeKind::Deleted, "删掉的.txt"),
            ("T", ChangeKind::Modified, "换成链接的.txt"),
        ];
        for (status, expected, path) in cases {
            let input = record(status, path);
            let parsed = parse_name_status(&input).unwrap();
            assert_eq!(parsed.len(), 1, "输入 {input:?} 应解析出一条");
            assert_eq!(parsed[0].1, PathBuf::from(path), "路径解析错：{input:?}");
            assert_eq!(parsed[0].0, expected, "输入 {input:?} 的分类不对");
        }
    }

    /// 多条记录、末尾缺 NUL、以及空输入都要解析得对。
    #[test]
    fn the_name_status_parser_reads_records_until_the_end() {
        let input = [
            record("M", "a"),
            record("D", "b"),
            record("A", "c"),
        ]
        .concat();
        let parsed = parse_name_status(&input).unwrap();
        let paths: Vec<PathBuf> = parsed.into_iter().map(|(_, p)| p).collect();
        assert_eq!(
            paths,
            vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]
        );

        // 末尾缺 NUL：git 每条记录之后（含最后一条）都会给 NUL，故这一条不会出现；
        // 它在这里只用来钉住「字段耗尽即结束」，不把最后一条读丢。
        let parsed = parse_name_status(b"M\0a").unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1, PathBuf::from("a"));

        assert!(parse_name_status(b"").unwrap().is_empty());
    }

    /// 不认识的状态一律报错，**不丢弃**：丢弃会让 Diff 少列一条改动。
    #[test]
    fn the_name_status_parser_refuses_a_status_it_does_not_know() {
        // `U` 是合并冲突留下的未合并条目，`R`/`C` 在 `--no-renames` 下不该出现，
        // `X` 是 git 自己都不认的未知状态。三者都不在本层的分类表里。
        for input in [
            record("U", "冲突的.txt"),
            record("R100", "旧名.txt"),
            record("X", "怪的.txt"),
        ] {
            let err = parse_name_status(&input).unwrap_err();
            match &err {
                GateError::Malformed { reason } => assert!(
                    reason.contains("状态"),
                    "错误未说明是状态不认识：{reason}"
                ),
                other => panic!("期望 Malformed，得到 {other:?}"),
            }
        }
        // 状态之后没有路径（缺字段与空字段两种形态）同样是 Malformed，
        // 而不是「解析成一条空路径的改动」
        for input in [&b"A"[..], b"A\0", b"A\0\0"] {
            let err = parse_name_status(input).unwrap_err();
            assert!(
                matches!(err, GateError::Malformed { .. }),
                "输入 {input:?} 得到 {err:?}"
            );
        }
    }

    /// 非 UTF-8 的文件名按原始字节还原，不经替换字符。
    #[test]
    fn the_name_status_parser_keeps_non_utf8_paths_intact() {
        let raw = b"A\0\xff\xfe.txt\0";
        let parsed = parse_name_status(raw).unwrap();
        assert_eq!(
            parsed[0].1.as_os_str().as_encoded_bytes(),
            b"\xff\xfe.txt",
            "非 UTF-8 的路径被改写"
        );
    }

    /// 逐文件比较的三类判定：只在 Task 里、两边都有而内容不同、只在 Base 里。
    #[test]
    fn the_tree_diff_classifies_each_file() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root.join("子目录")).unwrap();
            std::fs::write(root.join("相同.txt"), "一样".as_bytes()).unwrap();
            std::fs::write(root.join("子目录/改过的.txt"), "原".as_bytes()).unwrap();
        }
        std::fs::write(base.join("只在_base_里.txt"), "旧".as_bytes()).unwrap();
        std::fs::write(task.join("新文件.txt"), "新".as_bytes()).unwrap();
        std::fs::write(task.join("子目录/改过的.txt"), "改".as_bytes()).unwrap();

        let diff = diff_trees(&base, &task).unwrap();
        assert_eq!(diff.added(), &[PathBuf::from("新文件.txt")][..], "{diff:?}");
        assert_eq!(
            diff.modified(),
            &[PathBuf::from("子目录/改过的.txt")][..],
            "嵌套路径的比较不对：{diff:?}"
        );
        assert_eq!(
            diff.deleted(),
            &[PathBuf::from("只在_base_里.txt")][..],
            "{diff:?}"
        );
        assert!(!diff.is_empty());
    }

    /// 两侧完全一致时三类都空——空目录与只在 Task 里存在的目录都不算改动。
    #[test]
    fn the_tree_diff_ignores_directories_and_equal_files() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root.join("子目录")).unwrap();
            std::fs::write(root.join("一样.txt"), "一样".as_bytes()).unwrap();
        }
        // 只在 Task 里建一个**空**目录：目录不进结果，故 Diff 仍为空
        std::fs::create_dir_all(task.join("空目录")).unwrap();

        let diff = diff_trees(&base, &task).unwrap();
        assert!(diff.is_empty(), "空目录被算成了改动：{diff:?}");
    }

    /// 符号链接按**目标路径**比较，不跟随目标；非常规条目（FIFO）不进结果。
    ///
    /// 「不跟随」是 `collect_files` 文档里的一条：跟随的话，悬空的链接会以 `NotFound`
    /// 被当成不存在，指向目录的链接会被递归进去（见另一条环的用例）。FIFO 那一半钉的是
    /// 「只登记 `is_file()` 的条目」——若实现改成「不是目录就读内容」，读一个没有写者的
    /// FIFO 会**阻塞**，本用例会挂住而不是失败。
    #[test]
    fn the_tree_diff_compares_symlinks_by_their_target_and_skips_odd_entries() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root).unwrap();
            std::fs::write(root.join("目标.txt"), "内容").unwrap();
            std::os::unix::fs::symlink("目标.txt", root.join("链接.txt")).unwrap();
            std::os::unix::fs::symlink("并不存在", root.join("悬空链接.txt")).unwrap();
        }
        // Task 侧：一个链接改了目标，另有一个指向自身所在目录的**环**
        std::fs::remove_file(task.join("链接.txt")).unwrap();
        std::os::unix::fs::symlink("别处.txt", task.join("链接.txt")).unwrap();
        std::os::unix::fs::symlink(".", task.join("环")).unwrap();
        // Task 侧另有一个 FIFO（非常规条目）。`mkfifo` 是外部命令，取不到就跳过这一段，
        // 不把「环境里没有 mkfifo」误报成实现缺陷。
        let fifo = task.join("管道");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !made {
            eprintln!(
                "【跳过】the_tree_diff_compares_symlinks_by_their_target_and_skips_odd_entries\
                 的 FIFO 一段：mkfifo 不可用。"
            );
        }

        let diff = diff_trees(&base, &task).unwrap();
        assert_eq!(
            diff.modified(),
            &[PathBuf::from("链接.txt")][..],
            "改目标的链接没被算成修改：{diff:?}"
        );
        assert_eq!(
            diff.added(),
            &[PathBuf::from("环")][..],
            "指向自身所在目录的链接应是一个普通条目（内容即其目标），而不是递归源头；\
             两侧相同的东西与 FIFO 都不该出现：{diff:?}"
        );
        assert!(diff.deleted().is_empty(), "悬空链接被当成了不存在：{diff:?}");
    }

    /// 读不到的条目返回 `Err`（`IoFailed` 带该路径），**不**当作「不存在」跳过。
    ///
    /// 跳过会让 Diff 少列一条而调用方无从得知，而 Diff 是三种写入操作的依据。
    /// 权限位对 root 不起作用，故 root 下显式跳过并打标记。
    #[test]
    fn the_tree_diff_reports_an_unreadable_file_as_an_error() {
        if running_as_root() {
            eprintln!(
                "【跳过】the_tree_diff_reports_an_unreadable_file_as_an_error：\
                 本进程以 root 运行，权限位挡不住读取。本用例未执行断言。"
            );
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&task).unwrap();
        let unreadable = task.join("读不到.txt");
        std::fs::write(&unreadable, "秘密").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

        let err = diff_trees(&base, &task).unwrap_err();
        match &err {
            GateError::Workspace(WorkspaceError::IoFailed { path, .. }) => {
                assert_eq!(path, &unreadable, "错误里的路径不对：{err}");
            }
            other => panic!("期望读失败原样报出，得到 {other:?}"),
        }

        // 目录读不到同样是 Err（`read_dir` 失败），理由相同
        let closed = task.join("关着的目录");
        std::fs::create_dir_all(&closed).unwrap();
        std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000)).unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o600)).unwrap();
        let err = diff_trees(&base, &task).unwrap_err();
        assert!(
            matches!(
                &err,
                GateError::Workspace(WorkspaceError::IoFailed { path, .. }) if path == &closed
            ),
            "目录读不到未原样报出：{err:?}"
        );
        std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    /// 指向自身所在目录的链接不会让遍历陷进去：跟随的话会一路递归到内核的 `ELOOP`
    /// （Linux 40 层），报出来的错误里带着几十段重复路径。
    ///
    /// 与上一条的区别是这里两侧都有环，故 Diff 为空——要钉的是「不报错、正常返回」，
    /// 而错误形态（40 段路径）在 `Ok` 的前提下自然无从谈起。
    #[test]
    fn a_symlink_cycle_does_not_send_the_walk_into_the_kernel() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root).unwrap();
            std::fs::write(root.join("普通.txt"), "x").unwrap();
            std::os::unix::fs::symlink(".", root.join("自指")).unwrap();
            std::fs::create_dir_all(root.join("子")).unwrap();
            std::os::unix::fs::symlink("..", root.join("子/回指")).unwrap();
        }
        let diff = diff_trees(&base, &task).unwrap();
        assert!(diff.is_empty(), "两侧相同的环不该出现在 Diff 里：{diff:?}");
    }

    /// 索引副本：内容与真实索引逐字节相同，且随本值一并删除。
    #[test]
    fn the_index_copy_mirrors_the_index_and_cleans_up_after_itself() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("index");
        std::fs::write(&real, "假的索引内容".as_bytes()).unwrap();

        let copy = IndexCopy::from_index(&real).unwrap();
        let path = copy.path().to_path_buf();
        assert_ne!(path, real, "副本不该就是真实索引本身");
        assert_eq!(std::fs::read(&path).unwrap(), "假的索引内容".as_bytes());

        drop(copy);
        assert!(!path.exists(), "副本未被删除：{}", path.display());
        assert!(real.exists(), "真实索引被删了");
    }

    /// 索引不存在时建一个空副本——git 眼中「没有索引」与「索引为空」是同一件事。
    #[test]
    fn the_index_copy_of_a_missing_index_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let copy = IndexCopy::from_index(&dir.path().join("并不存在的索引")).unwrap();
        assert_eq!(std::fs::read(copy.path()).unwrap(), Vec::<u8>::new());
    }

    /// 每次都是新的临时文件，不会撞上上一次留下的名字。
    #[test]
    fn each_index_copy_gets_its_own_path() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("index");
        std::fs::write(&real, b"x").unwrap();
        let a = IndexCopy::from_index(&real).unwrap();
        let b = IndexCopy::from_index(&real).unwrap();
        assert_ne!(a.path(), b.path());
    }

    // ===== 写入操作（Task 9）=====
    //
    // **这三项操作的用例写在本模块内，不写在 `tests/gate.rs`。** 它们都收
    // `&GateApproval`，而该类型在 crate 之外没有构造路径——`tests/gate.rs` 是另一个
    // crate，连一个批准值都拿不出来，从那里根本调用不到这些函数。这不是省事的权宜：
    // 它正是「无公开构造函数」那条保证的直接后果（设计 6.2 把该值的产生点留给了下篇
    // 的驱动），`tests/compile_fail/gate_approval_*.rs` 钉的就是这一点。`discard` 不收
    // 批准值，故它的用例仍在外面的集成测试里。
    //
    // 夹具与 `tests/gate.rs` 的那几个同源，但无法共用（单测与集成测试是两个 crate），
    // 故各留一份，这里只保留本模块用得上的那几个。

    /// `dir` 内跑一条 git 命令，返回裁剪后的 stdout；失败即断言失败。
    fn run_git(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} 失败（退出码 {:?}）：{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// `dir` 内跑一条 git 命令，返回是否以退出码 0 结束（不断言）。
    fn git_succeeds(dir: &Path, args: &[&str]) -> bool {
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    }

    /// `dir` 内跑一条给出路径清单的 git 命令（`-z` 输出），按 NUL 切分。
    ///
    /// 用 `-z` 而不是逐行读：不加它时 git 会把含中文的路径转义成 `"\346\226\207"`
    /// 一类带引号的形式，断言就得跟着写转义串。与 `view_diff` 取同一形态。
    fn git_paths(dir: &Path, args: &[&str]) -> Vec<String> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} 失败（退出码 {:?}）：{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
            .split(|b| *b == 0)
            .filter(|field| !field.is_empty())
            .map(|field| String::from_utf8_lossy(field).into_owned())
            .collect()
    }

    /// 建一个真实 Git 仓库，返回（保活用的 `TempDir`，规范化的仓库根）。
    fn git_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        run_git(&root, &["init", "-b", "main"]);
        // 缺这两项 `git commit` / `cherry-pick` 会直接失败，失败点会指向夹具而非实现。
        run_git(&root, &["config", "user.email", "test@example.invalid"]);
        run_git(&root, &["config", "user.name", "Continuum 测试"]);
        std::fs::write(root.join("要改的.txt"), "原内容\n").unwrap();
        std::fs::write(root.join("要删的.txt"), "将被删除\n").unwrap();
        // 一个二进制文件（内容含 NUL）：`git diff` 不带 `--binary` 时只给一句
        // 「Binary files differ」，`git apply` 因而应用不上——补丁里必须有它的实际
        // 内容（`apply_patch_integrates_the_task_changes_into_the_base` 用它钉住这一点）。
        std::fs::write(root.join("二进制.bin"), [0u8, 1, 2, 255, 254]).unwrap();
        run_git(&root, &["add", "-A"]);
        run_git(&root, &["commit", "-m", "初始提交"]);
        (dir, root)
    }

    /// 建一个带审计表的库，返回（保活用的 `TempDir`，库）。
    fn db() -> (tempfile::TempDir, continuum_persist::Db) {
        let dir = tempfile::tempdir().unwrap();
        let db = continuum_persist::Db::open(&dir.path().join("t.db")).unwrap();
        db.migrate().unwrap();
        (dir, db)
    }

    /// `audit_log` 的全部（kind, payload）。
    fn audit_rows(tx: &Tx<'_>) -> Vec<(String, String)> {
        tx.query("SELECT kind, payload FROM audit_log ORDER BY seq", &[])
            .unwrap()
            .into_iter()
            .map(|row| {
                let text = |v: &Value| match v {
                    Value::Text(s) => s.clone(),
                    other => panic!("审计列应为文本，实际 {other:?}"),
                };
                (text(&row[0]), text(&row[1]))
            })
            .collect()
    }

    /// `apply_patch` 把 Task 的改动打进 Base：已跟踪文件的改与删、未跟踪文件的新增。
    ///
    /// 断言里含有「Base 的提交点与索引都没动」——`git apply` 不加 `--index`，改动落成
    /// 未暂存的工作树内容，由用户自己决定怎么提交。这一条与「Task 仍在」一起，把
    /// 「集成是把改动搬过去，不是把 Task 搬过去」钉住。
    #[test]
    fn apply_patch_integrates_the_task_changes_into_the_base() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) = crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1"))
            .unwrap();
        assert_eq!(backend, WorkspaceBackend::Worktree);

        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
        std::fs::remove_file(task.root().join("要删的.txt")).unwrap();
        std::fs::write(task.root().join("新文件.txt"), "Task 新增\n").unwrap();
        std::fs::create_dir_all(task.root().join("新目录")).unwrap();
        std::fs::write(task.root().join("新目录/深层.txt"), "深层新增\n").unwrap();
        // 二进制文件的改动也在补丁里：`--binary` 少一个，这一处就到不了 Base
        std::fs::write(task.root().join("二进制.bin"), [9u8, 8, 7, 0, 6]).unwrap();
        let head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &GateApproval(()))
            .unwrap();
        tx.commit().unwrap();

        assert_eq!(
            std::fs::read_to_string(base_path.join("要改的.txt")).unwrap(),
            "Task 改过\n",
            "已跟踪文件的改动没有进 Base"
        );
        assert!(
            !base_path.join("要删的.txt").exists(),
            "Task 删掉的文件仍在 Base 里"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("新文件.txt")).unwrap(),
            "Task 新增\n",
            "Task 的未跟踪文件没有进 Base"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("新目录/深层.txt")).unwrap(),
            "深层新增\n",
            "Task 的深层未跟踪文件没有进 Base（中间目录未建出？）"
        );
        assert_eq!(
            std::fs::read(base_path.join("二进制.bin")).unwrap(),
            vec![9u8, 8, 7, 0, 6],
            "二进制文件的改动没有进 Base（补丁缺了 --binary？）"
        );

        // 改动是未暂存的：`git diff` 看得见，`git diff --cached` 是空的。
        // 用具名值接住再排一次序：git 按路径的字节序列出，而本用例关心的是**集合**
        // ——少了哪个、多了哪个，不关心它在输出里排第几。
        let mut unstaged = git_paths(&base_path, &["diff", "--name-only", "-z", "--no-renames"]);
        unstaged.sort();
        assert_eq!(
            unstaged,
            vec![
                "二进制.bin".to_owned(),
                "要删的.txt".to_owned(),
                "要改的.txt".to_owned()
            ],
            "未暂存的改动集合不对"
        );
        assert!(
            git_paths(&base_path, &["diff", "--cached", "--name-only", "-z"]).is_empty(),
            "apply_patch 替用户把改动暂存了"
        );
        let mut untracked =
            git_paths(&base_path, &["ls-files", "--others", "--exclude-standard", "-z"]);
        untracked.sort();
        assert_eq!(
            untracked,
            vec!["新文件.txt".to_owned(), "新目录/深层.txt".to_owned()],
            "新增的文件不该被暂存，也不该少"
        );
        assert_eq!(
            run_git(&base_path, &["rev-parse", "HEAD"]),
            head_before,
            "apply_patch 不该替用户提交"
        );

        // Task 仍在：集成不是把工作区搬走
        assert!(task.root().exists(), "Task 根不见了");
        assert!(
            run_git(&base_path, &["branch", "--format=%(refname:short)"])
                .lines()
                .any(|l| l.trim() == "ai/i1"),
            "Task 分支不见了"
        );
    }

    /// `cherry_pick` 只摘**点名的**提交：Task 上两个提交，只摘第一个。
    ///
    /// 这条与 `merge_brings_the_whole_task_branch` 是一对：同一个 Task、同样的两个提交，
    /// 一个只带第一个，一个把两个都带上。缺了任一条，「按提交摘取」与「并入整个分支」
    /// 的差别就无从分辨——两边都只剩「改动进了 Base」这一个共同结果。
    #[test]
    fn cherry_pick_brings_only_the_named_commits() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) = crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1"))
            .unwrap();
        let task_root = task.root().to_path_buf();

        std::fs::write(task_root.join("第一个提交.txt"), "一\n").unwrap();
        run_git(&task_root, &["add", "-A"]);
        run_git(&task_root, &["commit", "-m", "第一个"]);
        let first = run_git(&task_root, &["rev-parse", "HEAD"]);
        std::fs::write(task_root.join("第二个提交.txt"), "二\n").unwrap();
        run_git(&task_root, &["add", "-A"]);
        run_git(&task_root, &["commit", "-m", "第二个"]);
        let head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        IntegrationGate::new(&base)
            .cherry_pick(&tx, &task, backend, &[&first], 1_000, &GateApproval(()))
            .unwrap();
        tx.commit().unwrap();

        assert!(
            base_path.join("第一个提交.txt").exists(),
            "点名的提交没有进 Base"
        );
        assert!(
            !base_path.join("第二个提交.txt").exists(),
            "只摘了一个提交，第二个提交的改动也进了 Base"
        );
        assert_ne!(
            run_git(&base_path, &["rev-parse", "HEAD"]),
            head_before,
            "cherry-pick 应当在 Base 上产生一个提交"
        );
        // 拿提交报文认，不拿哈希：**原提交对象不一定是新提交的祖先**。摘取产生的是
        // 新提交，只有它的父、树、作者、提交者与时刻与原件全同，两者的哈希才会相同
        // （实测同一秒内做这件事就相同）——那样的断言会随「跨没跨过一秒」时绿时红。
        let subjects = run_git(&base_path, &["log", "--format=%s"]);
        assert!(
            subjects.lines().any(|line| line == "第一个"),
            "摘下来的改动没有成为 Base 历史里的一个提交：{subjects}"
        );
        assert!(
            !subjects.lines().any(|line| line == "第二个"),
            "没点名的那个提交也进了 Base 的历史：{subjects}"
        );
        // 与 apply_patch 的对照：那边改动落成未暂存的工作树内容，这边落成一个提交
        assert_eq!(
            run_git(&base_path, &["status", "--porcelain"]),
            "",
            "cherry-pick 之后 Base 的工作树应当是干净的"
        );
    }

    /// `merge` 把 Task 分支整个并进 Base：两个提交的改动都在。
    #[test]
    fn merge_brings_the_whole_task_branch() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) = crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1"))
            .unwrap();
        let task_root = task.root().to_path_buf();

        std::fs::write(task_root.join("第一个提交.txt"), "一\n").unwrap();
        run_git(&task_root, &["add", "-A"]);
        run_git(&task_root, &["commit", "-m", "第一个"]);
        std::fs::write(task_root.join("第二个提交.txt"), "二\n").unwrap();
        run_git(&task_root, &["add", "-A"]);
        run_git(&task_root, &["commit", "-m", "第二个"]);
        let task_head = run_git(&task_root, &["rev-parse", "HEAD"]);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        IntegrationGate::new(&base)
            .merge(&tx, &task, backend, 1_000, &GateApproval(()))
            .unwrap();
        tx.commit().unwrap();

        assert_eq!(
            std::fs::read_to_string(base_path.join("第一个提交.txt")).unwrap(),
            "一\n"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("第二个提交.txt")).unwrap(),
            "二\n"
        );
        // 「并入」的实质是提交进了 Base 的历史：Base 的 HEAD 含 Task 分支的 HEAD。
        // 用户建好工作区后没再提交时 git 会快进，故这里不断言产生了合并提交。
        assert!(
            git_succeeds(&base_path, &["merge-base", "--is-ancestor", &task_head, "HEAD"]),
            "Task 分支的提交不在 Base 当前 HEAD 的历史里"
        );
    }

    /// 三项写入操作各写一条审计记录，`kind` 与操作对应（设计 6.4）。
    ///
    /// **逐项各建一个仓库**：同一条链上跑三次的话，一条记录的 `kind` 写错会被另一条
    /// 记录的存在掩盖。每次从空链开始，故「行数加一」「kind 是哪一个」都直接可读。
    #[test]
    fn the_write_operations_write_an_audit_record() {
        // 一个操作、它该记的 kind、以及审计 payload 里该出现的字样。
        let cases: [(&str, AuditKind, &str); 3] = [
            ("apply_patch", AuditKind::UserApprovals, "apply_patch"),
            ("cherry_pick", AuditKind::UserApprovals, "commits="),
            ("merge", AuditKind::UserApprovals, "merge"),
        ];
        for (operation, expected_kind, expected_payload) in cases {
            let (_d, base_path) = git_repo();
            let base = BaseWorkspace::new(&base_path).unwrap();
            let (task, backend) =
                crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
            let task_root = task.root().to_path_buf();
            std::fs::write(task_root.join("要改的.txt"), format!("{operation} 改过\n")).unwrap();
            run_git(&task_root, &["add", "-A"]);
            run_git(&task_root, &["commit", "-m", "Task 的一个提交"]);
            let commit = run_git(&task_root, &["rev-parse", "HEAD"]);

            let (_db_dir, db) = db();
            let tx = db.begin().unwrap();
            let gate = IntegrationGate::new(&base);
            assert!(
                audit_rows(&tx).is_empty(),
                "{operation}：操作之前审计链不是空的"
            );
            match operation {
                "apply_patch" => gate
                    .apply_patch(&tx, &task, backend, 1_000, &GateApproval(()))
                    .unwrap(),
                "cherry_pick" => gate
                    .cherry_pick(&tx, &task, backend, &[&commit], 1_000, &GateApproval(()))
                    .unwrap(),
                "merge" => gate
                    .merge(&tx, &task, backend, 1_000, &GateApproval(()))
                    .unwrap(),
                other => panic!("未覆盖的操作：{other}"),
            }
            let rows = audit_rows(&tx);
            tx.commit().unwrap();

            assert_eq!(rows.len(), 1, "{operation}：应恰好多出一条审计记录");
            assert_eq!(
                rows[0].0,
                expected_kind.as_str(),
                "{operation}：审计记录的 kind 不对"
            );
            assert!(
                rows[0].1.contains(operation),
                "{operation}：payload 里没有操作名：{}",
                rows[0].1
            );
            assert!(
                rows[0].1.contains(expected_payload),
                "{operation}：payload 里缺少 {expected_payload}：{}",
                rows[0].1
            );
            assert!(
                rows[0].1.contains("intent=i1") && rows[0].1.contains("backend=worktree"),
                "{operation}：payload 里缺少 Intent 或后端：{}",
                rows[0].1
            );
        }
    }

    /// `cherry_pick` 不给提交即拒绝，报出的是 `GateRefused` 而不是 git 的用法报错。
    ///
    /// 两种错法的差别对调用方是实质的：`git cherry-pick` 在零参数下打印用法并以 129
    /// 退出，经本层原样透出就是一句「git 命令失败（退出码 129）」，看不出是**调用方
    /// 没给提交**。Base 一并断言未被动过——拒绝必须发生在任何改动之前。
    #[test]
    fn cherry_pick_refuses_an_empty_commit_list() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) = crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1"))
            .unwrap();
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
        let head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .cherry_pick(&tx, &task, backend, &[], 1_000, &GateApproval(()))
            .unwrap_err();

        match &err {
            GateError::Workspace(WorkspaceError::GateRefused { reason }) => assert!(
                reason.contains("提交"),
                "拒绝的理由没说清是缺提交：{reason}"
            ),
            other => panic!("期望 GateRefused，得到 {other:?}"),
        }
        assert_eq!(
            run_git(&base_path, &["rev-parse", "HEAD"]),
            head_before,
            "被拒绝的调用动了 Base"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("要改的.txt")).unwrap(),
            "原内容\n",
            "被拒绝的调用改了 Base 的工作树"
        );
        assert!(
            audit_rows(&tx).is_empty(),
            "被拒绝的调用写了审计记录"
        );
    }

    /// 审计行写不进去时，报出的是 [`GateError::Persist`]，而**Base 的改动已经生效**。
    ///
    /// 这条钉的是本层事务边界的确切形状：变更在库之外（不可回滚），审计行在事务里。
    /// 本层先变更、后记录（与 P1 的先状态后事件同一次序），故记录失败时那处改动留在
    /// Base 上，调用方回滚事务也撤不回来。契约要求调用方见到 `Err` 就回滚，但这条窗口
    /// 是回滚合不上的——写在这里是为了让它可验证，而不是让它隐形。
    ///
    /// 造法：建一个**不带任何迁移**的库，`audit_log` 表因此不存在，`append_audit` 必然
    /// 失败。比伪造一个失败的 `Tx` 更省事，也仍是真实的数据库错误。
    #[test]
    fn a_failed_audit_write_reports_persist_and_leaves_the_change_in_the_base() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();

        let dir = tempfile::tempdir().unwrap();
        let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), Vec::new()).unwrap();
        db.migrate().unwrap();
        let tx = db.begin().unwrap();

        let err = IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &GateApproval(()))
            .unwrap_err();
        assert!(
            matches!(err, GateError::Persist(_)),
            "期望 Persist，得到 {err:?}"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("要改的.txt")).unwrap(),
            "Task 改过\n",
            "审计失败时改动没有落到 Base：本层的次序变了（应当是先变更、后记录）"
        );
    }

    /// 补丁对不上 Base 时，报出的是 git 的失败（带退出码与 stderr），Base 不动。
    ///
    /// 处境是真实的：补丁带上下文，而 Base 的当前内容可能与分歧点已不同——用户改了
    /// 同一行。本层不做三方合并（`merge` 才有那个语义），故这是失败而非自动处理。
    #[test]
    fn apply_patch_reports_a_patch_that_does_not_fit_the_base() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) = crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1"))
            .unwrap();
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
        // 用户在 Base 上把同一处改成了别的内容：补丁的上下文对不上
        std::fs::write(base_path.join("要改的.txt"), "用户在 Base 上的改动\n").unwrap();

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &GateApproval(()))
            .unwrap_err();

        match &err {
            GateError::Workspace(WorkspaceError::GitFailed { code, stderr }) => {
                assert_ne!(*code, 0, "GitFailed 的退出码不该是 0");
                assert!(
                    !stderr.is_empty(),
                    "git 的报文丢了，调用方看不出为什么打不上"
                );
            }
            other => panic!("期望透出 git 的失败，得到 {other:?}"),
        }
        assert_eq!(
            std::fs::read_to_string(base_path.join("要改的.txt")).unwrap(),
            "用户在 Base 上的改动\n",
            "打不上补丁时 Base 被改动了"
        );
        assert!(
            audit_rows(&tx).is_empty(),
            "失败的操作写了审计记录（先变更后记录的次序在这里体现）"
        );
    }

    /// overlay 后端的三项写入操作：Task 的改动逐条落进 Base。
    ///
    /// 直接调 [`integrate_overlay`]，不经挂载——本函数只做「比较两棵树」与「把差别写
    /// 过去」，与覆盖层本身无关（Task 根是挂载点时也走同一条路）。覆盖层的挂载/卸载
    /// 由 `tests/backend_overlay.rs` 覆盖，本用例钉的是集成这一段的语义。
    #[test]
    fn integrating_into_the_base_materializes_the_task_changes() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root.join("子目录")).unwrap();
            std::fs::write(root.join("要改的.txt"), "原内容\n").unwrap();
            std::fs::write(root.join("子目录/改过的.txt"), "原\n").unwrap();
        }
        std::fs::write(base.join("要删的.txt"), "只在 Base\n").unwrap();

        // Task 侧：改一个、删一个（`要删的.txt` 只在 Base 里有，Task 侧本就没有——覆盖
        // 层里这正是一个白障）、加一个（含深一层的路径）、加一个符号链接
        std::fs::write(task.join("要改的.txt"), "Task 改过\n").unwrap();
        std::fs::write(task.join("子目录/改过的.txt"), "改\n").unwrap();
        std::fs::write(task.join("子目录/新增.txt"), "新\n").unwrap();
        std::os::unix::fs::symlink("要改的.txt", task.join("新链接.txt")).unwrap();

        integrate_overlay(&base, &task).unwrap();

        assert_eq!(
            std::fs::read_to_string(base.join("要改的.txt")).unwrap(),
            "Task 改过\n"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("子目录/改过的.txt")).unwrap(),
            "改\n"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("子目录/新增.txt")).unwrap(),
            "新\n"
        );
        assert!(
            !base.join("要删的.txt").exists(),
            "Task 侧删掉的条目仍在 Base 里（覆盖层里的删除是白障，本层按「Task 侧看不到」\
             判它是删除）"
        );
        assert_eq!(
            std::fs::read_link(base.join("新链接.txt")).unwrap(),
            PathBuf::from("要改的.txt"),
            "新增的符号链接没有按目标重建"
        );
    }

    /// 复制一个条目时，目标处的符号链接被**替换**而不是被穿过。
    ///
    /// 这条盯的是边界本身：Base 在某一级是符号链接（`sub -> 别处`）而 Task 侧那一级
    /// 是目录时，直接往 `to_root/sub/x` 写会落到 `别处/x` ——写入越出了 Base，而
    /// `§256` 说的正是 Base 只读、改动只能落在 Base 之内。用 `copy_entry` 直接构造这一
    /// 处境（真实路径下 `integrate_overlay` 会先把那个链接当删除项移掉，故那里碰不到）。
    #[test]
    fn copying_replaces_a_symlinked_directory_instead_of_writing_through_it() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("别处");
        let to = dir.path().join("base");
        let from = dir.path().join("task");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::create_dir_all(from.join("sub")).unwrap();
        std::fs::write(from.join("sub/文件.txt"), "内容\n").unwrap();
        // Base 里的 `sub` 是指向 Base 之外的链接
        std::os::unix::fs::symlink(&outside, to.join("sub")).unwrap();

        copy_entry(&from, &to, Path::new("sub/文件.txt")).unwrap();

        assert!(
            !outside.join("文件.txt").exists(),
            "写入穿过了 Base 里的符号链接，落到 Base 之外：{}",
            outside.display()
        );
        assert_eq!(
            std::fs::read_to_string(to.join("sub/文件.txt")).unwrap(),
            "内容\n",
            "文件没有落在 Base 之内"
        );
        let meta = std::fs::symlink_metadata(to.join("sub")).unwrap();
        assert!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "Base 里那一级仍是链接，遮蔽没有生效"
        );
    }
}
