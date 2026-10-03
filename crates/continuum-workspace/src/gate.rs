//! Integration Gate（§257）：Task Workspace 到 Base Workspace 的唯一通道。
//!
//! Base 的工作文件永久只读（§256），Task 的改动只能经本模块进入 Base。`§257` 的
//! 五种操作都在本模块：只读的 `view_diff`、`discard`（Task 8），与写入的
//! `apply_patch`、`cherry_pick`、`merge`（Task 9）。
//!
//! **只读操作不收批准值。** 它们不动 Base，按 `§15` 的影响级是 L0，收一个
//! [`GateApproval`] 只会让「这个值代表什么」变得含糊。写入操作一律收
//! `&GateApproval`，使「集成修改需要授权」有落点（§16、设计 6.2）；该类型在本模块内
//! 定义、字段私有且没有公开构造函数，唯一的产生点是 [`approve_integration`]。
//!
//! # 批准值的绑定（设计 §7）
//!
//! 上篇的保证是「[`GateApproval`] 类型层不可构造」；设计 §7.1 把它松到「**只有一个
//! 具名的产生点**」，维持手段是评审与 grep。松的是构造路径，不是绑定：§7.2 要求产出的
//! 那一枚**只对具体一次集成有效**，故 [`approve_integration`] 收下这次集成的标识，
//! 值里携带它的摘要（算法、代价与边界都写在那个函数的文档里）。
//!
//! 三个写入操作在**动第一个字节之前**重算摘要并比对，不匹配即拒
//! （[`GateError::ApprovalMismatch`]）。`discard` 不在其列：它不收批准值
//! （[`IntegrationGate::discard`]），也就不做这道比对。
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
//! # 审计的归类（`§313` 与设计 6.4）
//!
//! 四项操作各记一条审计（设计 6.4：「五种操作中三种写入 Base 的，以及 `discard`，各写
//! 一条审计记录」）。`kind` 取 [`AuditKind`]，归类如下——**连同「为什么不新增变体」与
//! 那处已知的落差一并写在这里，免得后来者把查证重走一遍**（查证的路径是：`§313` 的八项
//! 原文 → 设计 6.4 那句「类推」→《总纲》的影响等级）。
//!
//! - `apply_patch` / `cherry_pick` / `merge` → [`AuditKind::UserApprovals`]。这三项恰是收
//!   [`GateApproval`] 的那些，`§16` 要求的「集成修改需要授权」（L3）落点正在其中，记的
//!   就是**用户批准过的那次集成**。
//! - `discard` → [`AuditKind::ExternalEffects`]。它不收批准值、也不动 Base，但确实在 Runtime
//!   的库之外留下持久改动——worktree 后端删掉用户仓库里的 `ai/<intent>` 分支，overlay 后端
//!   卸载覆盖层并删除工作目录。
//!
//! **已知的、刻意的落差：变体的名义等级与 `§15` 的影响等级对不上。** `§313` 的「外部副作用」
//! 在《总纲》里对应 **L4**（`docs/01-总纲.md:815` 的效果等级分层），而 Gate 的三种写入是
//! **L3**（同文件 `:806`：「从 AI worktree 向用户 branch 集成修改时，影响域升级为 L3」），
//! `discard` 更是 **L0**。`§313` 的八项里没有任何一项是 L3 或 L0 的语义，**换任何变体同样
//! 对不上**，故这里保留该映射而不把落差消掉——可用的消法只有两个，一个是改用一个同样不齐的
//! 变体，一个是为一次语义类比新增变体，两者都不比保留更对。**落差要可见。**
//!
//! **也不新增变体**：`AuditKind::ALL` 是长度写死的 `[AuditKind; 8]`
//! （`crates/continuum-events/src/audit.rs:38`），扩它会牵动 P0 的黄金向量与 serde rename
//! 一致性用例——为一个语义类比去动 P0 既有的断言，是拿稳定的东西换不稳定的东西。
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
use crate::task::{AI_DIR, TaskWorkspace};
use crate::worktree;
use continuum_events::audit::AuditKind;
use continuum_persist::{PersistError, Tx, Value};
use sha2::{Digest, Sha256};
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
            // 归类与那处刻意的 L 级落差见模块文档「审计的归类」。
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
        // 批准摘要的比对排在**任何**后端动作之前：不符即拒，Base 一字不动。
        self.verify_approval(task, backend, approval)?;
        match backend {
            WorkspaceBackend::Worktree => apply_patch_worktree(self.base, task)?,
            WorkspaceBackend::Overlay => {
                // 形态校验排在集成之前，理由见 `overlay::require_layout` 与
                // `integrate_overlay` 的文档：`backend` 由调用方给出，与 Task 的实际
                // 形态不符时 overlay 路径会拿 Base 与一棵**不是它的合并视图**的树对差。
                crate::overlay::require_layout(task.root())?;
                integrate_overlay(self.base.root(), task.root())?;
            }
        }
        self.audit(
            tx,
            // 归类与那处刻意的 L 级落差见模块文档「审计的归类」。
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
    /// **空列表在两个后端上都由本层拒绝**（[`WorkspaceError::GateRefused`]）。它的语义是
    /// 「摘这些提交」，空列表是**调用方错误**（忘了点名），不是「请求全部」——要全部有
    /// [`IntegrationGate::merge`]，那才是它的用途；让两个同名方法做同一件事，会让调用方
    /// 以为自己在做有粒度的集成。worktree 侧不落到 git 的另一个理由：git 在零参数下打印
    /// 用法并以 129 退出，调用方拿到的会是「git 命令失败」而不是「你没给提交」。
    ///
    /// **overlay 后端上没有提交粒度，按提交摘取不成立**（设计 6.3）：**一律拒绝**，空列表
    /// 与非空列表都是。**不退化**为「并入全部改动」——那会静默地集成得**比调用方要求的
    /// 更多**（点名一个提交，得到整棵树的改动），方向是危险的；而「同一调用在两种后端上
    /// 给出相反处置」比单侧的静默更隐蔽（调用方按后端分派时会以为空列表处处等价）。
    /// 调用方要按提交粒度集成只能用 worktree 后端。
    ///
    /// 拒绝排在**任何改动之前**，故被拒绝的调用在磁盘与库上都不留痕迹（用例：
    /// `cherry_pick_refuses_an_empty_commit_list`（worktree 侧）、
    /// `cherry_pick_is_refused_on_the_overlay_backend`、
    /// `cherry_pick_with_an_empty_list_is_refused_on_the_overlay_backend`——三条合起来钉住
    /// 「空列表在两个后端上同样被拒」）。
    ///
    /// **批准摘要的比对写在空列表判定之前**（见 [`IntegrationGate::verify_approval`]）。
    /// 两者都在任何改动之前，故各自单独出现时拿到的都是拒。**两个同时成立时报哪一个**由
    /// 这处次序决定（摘要在前），而**这个组合没有用例**——两个方向都是拒且都不动 Base，
    /// 故本层不据此作任何承诺。
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
        // 批准摘要的比对排在**任何**后端动作之前：不符即拒，Base 一字不动。
        self.verify_approval(task, backend, approval)?;
        match backend {
            WorkspaceBackend::Worktree => {
                if commits.is_empty() {
                    return Err(WorkspaceError::GateRefused {
                        reason: "cherry_pick 未给出任何提交。要把 Task 的全部改动并入 Base，\
                                 用 apply_patch 或 merge"
                            .to_owned(),
                    }
                    .into());
                }
                cherry_pick_worktree(self.base, commits)?;
            }
            WorkspaceBackend::Overlay => {
                // 见本函数的文档：一律拒绝，空列表与非空列表都是（不退化）。
                return Err(WorkspaceError::GateRefused {
                    reason: format!(
                        "overlay 后端没有提交粒度，无法按提交摘取（请求了 {} 个提交）；\
                         退化并入全部改动会静默地集成得比要求的多。要按提交粒度集成只能\
                         用 worktree 后端，要并入全部改动用 apply_patch 或 merge",
                        commits.len()
                    ),
                }
                .into());
            }
        }
        self.audit(
            tx,
            // 归类与那处刻意的 L 级落差见模块文档「审计的归类」。
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
        // 批准摘要的比对排在**任何**后端动作之前：不符即拒，Base 一字不动。
        self.verify_approval(task, backend, approval)?;
        match backend {
            WorkspaceBackend::Worktree => merge_worktree(self.base, task)?,
            WorkspaceBackend::Overlay => {
                // 同 `apply_patch`：形态校验排在集成之前。
                crate::overlay::require_layout(task.root())?;
                integrate_overlay(self.base.root(), task.root())?;
            }
        }
        self.audit(
            tx,
            // 归类与那处刻意的 L 级落差见模块文档「审计的归类」。
            AuditKind::UserApprovals,
            occurred_at,
            "merge",
            task,
            backend,
            None,
        )
    }

    /// 写入操作动手之前的那道闸：重算本次集成的摘要，与批准值携带的那一枚比对。
    ///
    /// 排在**所有后端动作之前**（补丁、`git cherry-pick`、覆盖层集成、形态校验都在它
    /// 之后），故被拒的调用在磁盘与库上都不留痕迹。用例
    /// `every_write_operation_refuses_a_stale_approval`（三个操作各一条链）与
    /// `changing_the_base_between_approval_and_application_invalidates_it` 一并钉住
    /// 「Base 逐字节不变、审计链上一条不多」。
    ///
    /// **本条只声明这一半，不声明与别的拒绝的先后**：过期的批准值叠上一个本来就会被拒的
    /// 请求（形态不符、空提交列表）时，先报哪一个由代码的次序决定（摘要在前），而**这个
    /// 组合没有用例**。两个方向都是 fail-closed、都不动 Base，故没有可观察的差别需要钉。
    ///
    /// **摘要重算会失败**（读不到某个条目），此时报的是那个 I/O 错误而不是拒绝：读不出
    /// 「现在是什么」与「现在不是批准时的那一份」是两件事，把它们折叠成同一个拒绝，会让
    /// 「重新铸一枚就能过」这种误导性的处置看起来可行。
    fn verify_approval(
        &self,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
        approval: &GateApproval,
    ) -> Result<(), GateError> {
        let expected = integration_digest(self.base, task, backend)?;
        if expected == approval.0 {
            return Ok(());
        }
        Err(GateError::ApprovalMismatch {
            reason: format!(
                "这次集成（intent={}、backend={}、base={}）的摘要与批准值携带的不符：\
                 这一枚是为另一次集成铸的，或者铸造之后 Base 或 Task 的内容被改动过。\
                 要批准**当前**这份集成，重新调用 approve_integration",
                task.intent_id().as_str(),
                backend_str(backend),
                self.base.root().display(),
            ),
        })
    }

    /// 把一次写入操作记入审计（设计 6.4）。**归类及其已知落差见模块文档「审计的归类」。**
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

/// 集成授权的唯一产生点（设计 §7.2）。
///
/// 收下这次集成的标识（Base、Task、后端），铸出一枚只对它有效的 [`GateApproval`]。
/// 调用方须先取得授权（设计 §5.7：驱动在集成前问策略）；Capability 与 Authority 就位
/// 后，产生点移交给那一处。
///
/// # 摘要由两棵树的内容派生
///
/// 算法见 [`integration_digest`]：把**后端编码、Base 根、Task 根、Intent 标识**与
/// **两棵树各自的内容摘要**按序喂进一次 SHA-256。三个要说清的选择：
///
/// - **判据是内容，不是「路径 + `view_diff` 的路径清单」。** 后者在 §7.2 的刻意后果上
///   不成立：用户改一个 Task 没碰过的文件时，那个路径不在 `view_diff` 的清单里、两个根
///   也没变，摘要因而**失配不了**——而「集成前改动 Base 会使摘要失配」正是设计点名的
///   后果（用例 `changing_the_base_between_approval_and_application_invalidates_it`）。
/// - **也不取元数据（mtime、大小）。** 元数据回答的是「文件看起来变了吗」，不是「内容
///   是什么」：同样长度、同一时刻改写的文件元数据逐项相同（`IndexCopy` 的文档里那条
///   git 的 racy 规则，正是这个坑的另一种形态）。
/// - **位置与 Intent 也进摘要**：内容相同的两棵树分属不同 Intent 时得到两枚不同的值。
///   批准值绑定的是**这一次**集成，不只是这一份字节（用例
///   `an_approval_for_one_integration_does_not_authorize_another`）。**后端也进**：
///   [`IntegrationGate::view_diff`] 的结果按后端而异（分派到 `diff_worktree` 与
///   `diff_trees`），换后端就是换这一次集成的含义。
///
/// # 代价
///
/// 每次调用走遍两棵树，把每个条目的字节各过一次 SHA-256，量级是 **O(Base 的字节数 +
/// Task 的字节数)**，与仓库大小成正比。一次完整的「铸造 + 写入」因而要走四棵树：铸造两
/// 棵、写入前的比对两棵。文件内容经 `io::copy` 流式喂入，内存里不留副本，但**磁盘读取
/// 是实打实的**——Base 是一个大仓库时，这一步可能比补丁本身贵。这是「判据是内容」的
/// 直接代价：省掉内容，就回到上面那个**失配不了**的算法。
///
/// # 不进摘要的两处，与一处不绑定
///
/// - **`.git` 不进**：仓库的内部账本不是内容。判据与理由同 [`collect_files`]（Diff 一侧
///   也排除它）。
/// - **`.ai/` 不进**（**任意层级**上名为 `.ai` 的目录，连同它的子树）：Runtime 在 Base
///   之内的私有子树（§5、[`AI_DIR`]）。两条理由各自都足够：其一，**worktree 后端的 Task
///   根就在 `<base>/.ai/worktrees/<intent>`**，不排除则「Base 树」的遍历会把 Task 连同
///   内容一起走进去，Base 的摘要随之变成 Task 内容的函数，而**别的 Intent 的 worktree
///   也在里面**——另一个 Intent 的改动会让本枚批准值失配（跨 Intent 耦合）；其二，那是
///   本层自己写出来的东西（worktree 目录、排除项、分支登记），不是用户的内容。
///   **代价明说**：`.ai/` 之下用户自己的内容同样不算内容——改动它**不进摘要**。但**在
///   worktree 后端上它仍可能被集成**（两侧的规则并不相同，见下面那一段），那个方向是
///   fail-open。
///
/// **两个后端的「两侧是否同一套规则」不同，逐一写明。**
///
/// - **overlay 后端：两侧逐项相同。** `view_diff` 经 [`collect_files`]，与 [`tree_digest`]
///   跑的是**同一个名字测试**（`name == ".git" || name == AI_DIR`）：任意层级、不分条目
///   种类（目录、常规文件、符号链接一视同仁）。
/// - **worktree 后端：两侧不是同一套规则。** git 那一侧由 `.git/info/exclude` 里那一行
///   `.ai/` 决定（[`crate::worktree`] 的 `ensure_ai_excluded`，**不带前导斜杠**，故按
///   gitignore 的规则可匹配任意层级——实测 git 2.56.0：只写这一行时 `git check-ignore`
///   对 `.ai/h.txt`、`sub/deep/.ai/f.txt`、`x/.ai/g.txt` 都判忽略，未列名的 `top.txt`
///   不忽略）。但它与名字测试有**两处**差别：
///   1. **只管目录**：`.ai/` 带尾斜杠，gitignore 的规则是「以斜杠结尾的模式只与目录匹配」。
///      故**名为 `.ai` 的常规文件或符号链接不被忽略**：`git ls-files --others
///      --exclude-standard` 会把它报成新增、untracked 那一步会把它复制过去，而
///      [`tree_digest`] 与 [`collect_files`] 按名字跳过它（不分种类）。
///   2. **只管未跟踪路径**：`--exclude-standard` 与 gitignore 都**压不住已跟踪路径**。
///      故 Base 里**已提交**的 `sub/.ai/f.txt` 在 Task 里被改动时，`git diff --name-status`
///      会把它报成修改、`git apply` 会把它应用到 Base，而它的内容**不在摘要里**。
///
/// **第 2 条的方向是 fail-open，本层今天没有关掉它。** 用户可以在 [`approve_integration`]
/// 与写入操作之间改掉那个已跟踪的 `sub/.ai/f.txt`：摘要不变、[`IntegrationGate::verify_approval`]
/// 照过、改动照落进 Base——为一份内容铸的批准值被应用到另一份上。**实测（worktree 后端）**：
/// `view_diff.modified()` 给出 `["sub/.ai/f.txt"]`，而拿着铸造前那份内容铸出的批准值
/// `apply_patch` 返回 `Ok(())`，Base 拿到的是铸造**之后**才写的那一份。第 1 条同样成立，
/// 只是要取**嵌套**形态（`sub/.ai` 是一个常规文件）：根层那个名为 `.ai` 的文件会撞上
/// `<base>/.ai/` 本是目录，被 [`ensure_not_a_directory`] 拒掉——那一处是偶然，不是保障。
///
/// 要关掉它须改排除规则本身，而那是**第二次设计裁定**，不是实现修复：「收窄为根层」只是
/// 把同一处缝挪到别的路径上（嵌套的、已跟踪的内容仍会被应用而摘要不收），而给两个后端
/// 分别定规则又会在 overlay 侧重新引入一处摘要/Diff 分歧。现象、两处差别的出处、实测输出、
/// 可达性的现实前提、那条否定结论与**两条候选规则**都记在 `docs/superpowers/p2-followups.md`
/// 的第八节（**有版本、随分支合并**，故这里指得动；本层各函数的文档不重复那份清单）。
/// - **空的目录不绑定**（与 [`Diff`] 对目录的看法一致：git 不跟踪目录，三类改动以文件与
///   符号链接为单位）。`mkdir <base>/空目录` 不会使批准值失配，往里放文件才会。
///
/// # 不绑定的东西
///
/// 摘要绑定的是**两棵树的内容**，输入里没有 Base 的 HEAD、当前分支与提交历史。可观察的
/// 后果是：在 Base 上做一个不改内容的提交、或切到内容相同的一条分支之后，同一枚批准值
/// 仍然有效（用例 `a_content_preserving_commit_does_not_invalidate_the_approval`）。这是
/// 算法选定的边界，不是疏漏：§7.2 要绑的是「把当前这份差异应用过去」，而内容相同时
/// Diff 是同一份、补丁仍打得上去（`merge` 会少一次快进，结果仍是「Task 的改动进入
/// Base」，只是多一个合并提交）。要连历史一并绑定，须把 HEAD 也喂进哈希——那会让「用户
/// 在 Base 上提交」也变成失配，而对补丁的应用性没有影响。
pub fn approve_integration(
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<GateApproval, GateError> {
    Ok(GateApproval(integration_digest(base, task, backend)?))
}

/// 集成修改的批准值。
///
/// **无公开构造函数，只有一个具名的产生点**：[`approve_integration`]。字段私有，也没有
/// `new` 一类的关联函数，故「**无名**构造路径」不存在——`tests/compile_fail/
/// gate_approval_*.rs` 的两条编译失败样例钉的就是这一点（它们钉的不是「Crate 外拿不到
/// 批准值」，那条保证按设计 §7.1 已经放宽）。
///
/// **一枚只对一次集成有效。** 值里携带的是铸造那一刻这次集成的摘要
/// （[`integration_digest`]）：两棵树的内容、后端、两棵树的位置与 Intent 标识。三个写入
/// 操作在动第一个字节之前重算摘要并比对，不匹配即拒
/// （[`GateError::ApprovalMismatch`]）——故「拿另一枚为另一次集成背书」不成立，铸造之后
/// 改动了任一棵树也不成立。
///
/// 写入操作一律收 `&GateApproval`，使「从隔离工作空间集成回用户分支需要授权」有落点
/// （`§16`、设计 6.2）——缺少它时 Gate 退化为「谁调用谁生效」。本子项目内由驱动的显式
/// 确认参数产生（[`approve_integration`]）；Capability 与 Authority 就位后，其产生点
/// 移交给那一处。只读操作（[`IntegrationGate::view_diff`]、[`IntegrationGate::discard`]）
/// 不收本值：它们不动 Base。
#[derive(Debug)]
pub struct GateApproval([u8; 32]);

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
    /// 批准值与本次集成不对应（设计 §7.2）：写入操作在动第一个字节之前重算摘要，
    /// 与批准值携带的那一枚比对，不符即拒。
    ///
    /// 三种来源，都**不是**「调用方拿错了对象」这么窄：
    /// - 这一枚是为另一次集成铸的（别的 Task、别的 Base、别的后端）；
    /// - 铸造之后 Base 或 Task 的内容被改动过——**包括 Task 没碰过的文件**，那正是
    ///   §7.2 点名的刻意后果；
    /// - 手工拼出来的值（字段私有，crate 外拼不出，但库内别的代码可以）。
    ///
    /// 与 [`Self::Workspace`] 里那个 [`WorkspaceError::GateRefused`] **分开**：那一个的语义是
    /// 「这次请求本身不成立或这个后端做不到」（空提交列表、覆盖层没有提交粒度），与批准值
    /// 无关。两者混为一谈，会让「拒绝是因为没授权」这一支凭空多出一类与授权无关的来源。
    ///
    /// **`reason` 里不出现摘要本身**：那是一枚 SHA-256 的十六进制，对读错误的人没有信息，
    /// 而「换一枚重新铸造」才是可行动的那一步——要与别处的日志对账时，把两边各自重算一次
    /// 更有意义。
    #[error("批准值与本次集成不符：{reason}")]
    ApprovalMismatch { reason: String },
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
    // 空补丁不送给 git：实测（git 2.56）`git apply` 在空输入下以退出码 128 与
    // 「No valid patches in input (allow with "--allow-empty")」失败，而「Task 只改了
    // 未跟踪文件」是正常情形，不是错误。
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

/// overlay 后端的 `apply_patch` 与 `merge`：把 Task 相对 Base 的改动（[`Diff`]）落进 Base。
///
/// 两个操作在这个后端上是同一件事（设计 6.3）：`merge` 是「把 upper 层的内容合并进
/// Base」，而 `apply_patch` 也只能是同一件事——覆盖层里既没有提交，也没有补丁的基准点。
/// 二者共用本函数。
///
/// **`cherry_pick` 不到这里**：它按定义要提交粒度，而覆盖层没有，故那个后端上它一律被拒
/// （空列表也拒，见 [`IntegrationGate::cherry_pick`] 的文档与
/// `cherry_pick_is_refused_on_the_overlay_backend` 用例）。本函数只服务
/// [`IntegrationGate::apply_patch`] 与 [`IntegrationGate::merge`] 两项。
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
///
/// **两阶段：先算计划并全量校验，再执行。** 见 [`plan_overlay_integration`]——本函数
/// 不做任何判定，只按计划走。执行阶段仍可能失败（I/O 错误、两侧树在计划之后被并发
/// 改动），那部分不在计划能判定的范围内，也没有事务可回滚；计划覆盖的是**能从计划本身
/// 判定**的那一类失败，而早先的实现正是在这一类上先动刀再报错。
fn integrate_overlay(base_root: &Path, task_root: &Path) -> Result<(), GateError> {
    for step in plan_overlay_integration(base_root, task_root)? {
        match step {
            IntegrationStep::Remove(rel) => remove_entry(&base_root.join(&rel))?,
            IntegrationStep::Copy(rel) => copy_entry(task_root, base_root, &rel)?,
        }
    }
    Ok(())
}

/// 集成计划的一步。删除一律排在复制之前（次序的理由见 [`integrate_overlay`] 的文档）。
#[derive(Debug)]
enum IntegrationStep {
    /// 删掉 Base 里 `rel` 处的条目。
    Remove(PathBuf),
    /// 把 Task 里 `rel` 处的条目复制进 Base。
    Copy(PathBuf),
}

/// 算出一次 overlay 集成要做的事，并**在动第一个字节之前**把能从计划本身判定的失败
/// 全部报出。返回 `Err` 时 Base 一字未动。
///
/// **这是本函数存在的全部理由。** 早先的写法是「边删边发现」：删除那一趟先把
/// `base - task` 的每个文件移掉，复制那一趟才可能在某一级撞上「目标是目录」而拒绝——
/// 于是**返回 `Err` 之前 Base 已经被删掉一部分**。这个窗口很容易撞上，因为两个既有
/// 判据合起来正好把它敞开：`collect_files` 只收文件与符号链接（目录根本不进 Diff），
/// 而 `remove_entry` 只对**目录**拒绝。最要紧的一例：Task 是 worktree 形态（根里
/// `.git` 是**文件**）而 Base 是仓库（`.git` 是**目录**）时，删除那一趟会把
/// `base/.git/**` 整个移掉，复制那一趟才拒——用户的仓库在报错之前就没了。
///
/// 故凡是「只看计划就能判定」的条件都在这里定：源是不是条目、目标是不是目录。任何一项
/// 不成立就整体拒绝，Base 上不留半个字节。这与 [`IntegrationGate::cherry_pick`] 在
/// overlay 分支上的处置同一条道理——**拒绝排在任何改动之前**。
fn plan_overlay_integration(
    base_root: &Path,
    task_root: &Path,
) -> Result<Vec<IntegrationStep>, GateError> {
    let diff = diff_trees(base_root, task_root)?;
    let mut plan = Vec::with_capacity(diff.added().len() + diff.modified().len() + diff.deleted().len());
    for rel in diff.deleted() {
        ensure_not_a_directory(&base_root.join(rel))?;
        plan.push(IntegrationStep::Remove(rel.clone()));
    }
    for rel in diff.added().iter().chain(diff.modified()) {
        ensure_copyable(task_root, base_root, rel)?;
        plan.push(IntegrationStep::Copy(rel.clone()));
    }
    Ok(plan)
}

/// 一步复制能否成功，取只看计划就能判定的那部分：源必须是条目（常规文件或符号链接，
/// [`collect_files`] 收的正是这两类），目标若已存在则不得是目录（[`remove_entry`]
/// 只对目录拒绝）。
///
/// 目录级的父路径不在此列：[`create_parent_dirs`] 遇到链接或普通文件会删掉它改建目录，
/// 那正是「Task 侧的目录遮蔽 Base 的同名条目」应有的结果，不是失败条件。
fn ensure_copyable(from_root: &Path, to_root: &Path, rel: &Path) -> Result<(), GateError> {
    let src = from_root.join(rel);
    let meta = std::fs::symlink_metadata(&src).map_err(|e| io_error(&src, e))?;
    if !(meta.is_file() || meta.file_type().is_symlink()) {
        return Err(WorkspaceError::GateRefused {
            reason: format!(
                "{} 既不是常规文件也不是符号链接，本层不复制目录",
                src.display()
            ),
        }
        .into());
    }
    ensure_not_a_directory(&to_root.join(rel))
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
    ensure_not_a_directory(path)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(path, e)),
    }
}

/// `path` 处的条目不得是目录。
///
/// 抽出来是因为它有两个调用时机：集成前算计划时（[`plan_overlay_integration`]，为了在
/// 动第一个字节之前拒绝）与执行时（[`remove_entry`] 自己，`copy_entry` 与
/// [`create_parent_dirs`] 也经它）。判定必须一致——计划说能删、执行时却拒绝，两阶段就
/// 白拆了。
fn ensure_not_a_directory(path: &Path) -> Result<(), GateError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Err(WorkspaceError::GateRefused {
            reason: format!(
                "{} 是目录，而 Task 侧对应的是文件：本层不删除目录（会连带删掉里面本层\
                 没有看过的条目）",
                path.display()
            ),
        }
        .into()),
        Ok(_) => Ok(()),
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
/// **`.git` 不进结果**（任何层级上名为 `.git` 的条目，连同它的子树）：仓库的内部状态
/// 不是内容。理由与后果见函数体内那处注释。
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
            // **`.git` 与 `.ai/` 不进差异集，也不递归进去**（按名字，任意层级）。
            //
            // `.git`：仓库的内部状态不是「内容」——它是 git 自己的账本（HEAD、config、
            // objects、refs……），集成它没有意义，而把两侧的 `.git` 当内容比较是有害的
            // ——Task 根是 worktree 时 `.git` 是**文件**、Base 的 `.git` 是**目录**，两者
            // 按内容一比就是「Base 侧一整棵 .git 被删」加「多了一个 .git」。`.git` 这个
            // 名字在 git 仓库里不可能是有意义的被跟踪内容（git 自己拒绝跟踪它），故按
            // 名字排除不会丢掉任何真实改动。
            //
            // `.ai/`：Runtime 在 Base 之内的私有子树（§5、[`AI_DIR`]），不是用户的内容。
            // 不排除则 overlay 后端上 Task 侧 `.ai/…` 的改动会被报成新增、并被
            // [`integrate_overlay`] 复制进 Base——而摘要按名字跳过 `.ai`
            // （[`tree_digest`]），两侧对「什么算内容」的看法于是分叉。worktree 后端上
            // git 侧本来就挡住了它（`.git/info/exclude` 里那一行，见 [`crate::worktree`]
            // 的 `ensure_ai_excluded`），这一条让 overlay 侧也看不见它。
            // **但两侧的规则并不逐项相同**：git 那条只管目录（`.ai/` 带尾斜杠）、只管
            // 未跟踪路径，故名为 `.ai` 的常规文件与已跟踪的 `.ai/…` 仍会被报出并集成，
            // 而本函数与 [`tree_digest`] 都按名字跳过它们——那一处 fail-open 的实测与
            // 待裁定项见 [`approve_integration`] 的文档。
            //
            // 排除在这里而不是在调用方：本函数是 Diff 的唯一来源（[`diff_trees`]），
            // view_diff 报出的与集成落进 Base 的必须是同一个集合，两边各排一次早晚分叉。
            let name = entry.file_name();
            if name == ".git" || name == AI_DIR {
                continue;
            }
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

/// 批准摘要的域分隔标签。
///
/// 喂进哈希的第一个字段：把本层的摘要与别处用同一份输入算出的摘要分开（同一对树在
/// 本 crate 之外也可能被哈希，用途不同就该得到不同的值）。**末段的版本号是这份输入
/// 定义的版本**：输入一改就要改它，否则新旧两种算法算出的值会在同一个字段里混着比。
const APPROVAL_DOMAIN: &str = "continuum-workspace/approval/v1";

/// [`tree_digest`] 里条目类别的标记字节。有了它，一个内容恰为某个链接目标的常规文件
/// 才不会与那个链接撞成同一串。
const ENTRY_FILE: u8 = 1;
const ENTRY_SYMLINK: u8 = 2;

/// 一次集成的摘要：写入操作据此判断手里的批准值是不是为**这一次**集成铸的。
///
/// 输入按序是：域分隔标签、后端编码、Base 根、Task 根、Intent 标识、Base 树的内容摘要、
/// Task 树的内容摘要。**为什么是这些、代价是什么、哪两处不进、什么不绑定**，都写在
/// [`approve_integration`] 的文档里——那是这份算法唯一的规范来源，这里不复述。
///
/// 每一段都按「长度 + 字节」喂（[`field`]）：只喂字节的话，两个相邻字段的切分点就不可
/// 判定（`"ab"+"c"` 与 `"a"+"bc"` 会喂出同一串字节）。
fn integration_digest(
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<[u8; 32], GateError> {
    let mut hasher = Sha256::new();
    field(&mut hasher, APPROVAL_DOMAIN.as_bytes());
    field(&mut hasher, backend_str(backend).as_bytes());
    field(&mut hasher, base.root().as_os_str().as_encoded_bytes());
    field(&mut hasher, task.root().as_os_str().as_encoded_bytes());
    field(&mut hasher, task.intent_id().as_str().as_bytes());
    field(&mut hasher, &tree_digest(base.root())?);
    field(&mut hasher, &tree_digest(task.root())?);
    Ok(finish(hasher))
}

/// 把一个字段按「长度 + 字节」喂进哈希器（切分点可判定的理由见 [`integration_digest`]）。
fn field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// 收尾：SHA-256 的定长结果转成数组（`finalize` 给的是一个泛型数组）。
fn finish(hasher: Sha256) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// 一棵树的**内容**摘要：逐条目把「类别 + 相对路径 + 内容」喂进哈希。
///
/// 与 [`collect_files`] 逐项相同的判据：不收目录、符号链接按目标路径、非常规条目不进、
/// 读失败即 `Err`，以及**按名字排除 `.git` 与任意层级的 `.ai/`**（依据与实测见
/// [`approve_integration`] 的文档那一节）。差别只有一处：本函数把内容**流式**喂进哈希，
/// 不把整棵树收进内存，故大仓库下的内存占用是常数。
///
/// **次序是确定的**：每层目录的条目先按文件名的**原始字节**排序再递归。`read_dir` 给的
/// 次序由文件系统决定，不排序的话同一棵树两次遍历可能给出两个摘要——而本摘要的全部用处
/// 就是「同一棵树必得同一枚值」。
///
/// 排除 `.git` 与 `.ai/`（理由见 [`approve_integration`] 的文档），符号链接不跟随
/// （跟随会走进环，见 [`collect_files`] 的文档）。
fn tree_digest(root: &Path) -> Result<[u8; 32], GateError> {
    let mut hasher = Sha256::new();
    digest_dir(root, Path::new(""), &mut hasher)?;
    Ok(finish(hasher))
}

/// [`tree_digest`] 的递归主体。`rel` 是 `root` 之下的相对路径。
///
/// **目录自身不喂任何字节**（与 [`Diff`] 对目录的看法一致）：一个空目录在哈希里不留
/// 痕迹，有内容的目录经其子条目的相对路径体现。故 `mkdir <base>/空目录` 不会使摘要变化
/// ——这是 [`approve_integration`] 的文档明说的那一条边界。
fn digest_dir(root: &Path, rel: &Path, hasher: &mut Sha256) -> Result<(), GateError> {
    let entries = std::fs::read_dir(root).map_err(|e| io_error(root, e))?;
    // 名字与种类一起收下：种类取自 `read_dir` 且**不跟随**符号链接（与 `collect_files`
    // 同一条），排完序再遍历，次序因而与文件系统给出的次序无关。
    let mut items = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_error(root, e))?;
        let file_type = entry.file_type().map_err(|e| io_error(&entry.path(), e))?;
        items.push((entry.file_name(), file_type));
    }
    items.sort_by(|(a, _), (b, _)| a.as_encoded_bytes().cmp(b.as_encoded_bytes()));

    for (name, file_type) in items {
        // `.git` 与 `.ai/` 不进摘要：判据与理由见 `approve_integration` 的文档。
        if name == OsStr::new(".git") || name == OsStr::new(AI_DIR) {
            continue;
        }
        let path = root.join(&name);
        let rel = rel.join(&name);
        if file_type.is_dir() {
            digest_dir(&path, &rel, hasher)?;
        } else if file_type.is_symlink() {
            field(hasher, &[ENTRY_SYMLINK]);
            field(hasher, rel.as_os_str().as_encoded_bytes());
            let target = std::fs::read_link(&path).map_err(|e| io_error(&path, e))?;
            field(hasher, target.as_os_str().as_encoded_bytes());
        } else if file_type.is_file() {
            field(hasher, &[ENTRY_FILE]);
            field(hasher, rel.as_os_str().as_encoded_bytes());
            digest_file(&path, hasher)?;
        }
        // 非常规条目（FIFO、套接字、设备）**不进**：与 `collect_files` 同一条判据
        // （覆盖层 upper 层里的白障正是这类条目，故这一条同时让白障不成问题）。
    }
    Ok(())
}

/// 把一个常规文件的内容流式喂进哈希。
///
/// 先喂长度（分帧），内容再经 `std::io::copy` 送进哈希器——哈希器实现了
/// `std::io::Write`，故不必先把整个文件读进内存。
///
/// **本层保证的只有「同一棵静止的树必得同一枚值」。** 文件在读取期间被并发改动时，长度
/// （取自读完 `File::open` 之后**另一次** `metadata` 调用）与实际复制的字节数可以不一致，
/// 那一次读取因而得到一个既非「错」、也非「可预测」的值。这里不声称并发交错下会发生什么，
/// 只声明不并发时的确定性——而铸造与写入前的比对正是两次这样的读取，故这条确定性是那份
/// 保障的全部依据。
fn digest_file(path: &Path, hasher: &mut Sha256) -> Result<(), GateError> {
    let mut file = std::fs::File::open(path).map_err(|e| io_error(path, e))?;
    let len = std::fs::metadata(path).map_err(|e| io_error(path, e))?.len();
    field(hasher, &len.to_le_bytes());
    std::io::copy(&mut file, hasher).map_err(|e| io_error(path, e))?;
    Ok(())
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

    /// 内容相同、**创建次序相反**的两棵树得到同一枚摘要。
    ///
    /// 这条钉的是 [`tree_digest`] 的次序确定性：`read_dir` 给的次序由文件系统决定，不排序
    /// 的话两份副本可能给出两枚不同的值。而「同一棵树必得同一枚值」不是锦上添花的要求——
    /// 铸造与写入前的比对是**两次独立的遍历**，次序不定就等于批准值有一半概率失配。
    ///
    /// 「创建次序相反」是造出次序差异的手段，不是判据的根据：本用例断言的是两份**内容
    /// 相同**的树摘要相等，故它在任何文件系统上都成立；它能否在去掉排序的实现上变红，
    /// 取决于该文件系统是否按创建次序返回条目（tmpfs 是，按名哈希的目录不是）。
    #[test]
    fn two_copies_of_the_same_tree_get_the_same_digest() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        let names = ["一.txt", "二.txt", "三.txt", "子目录/四.txt", "子目录/五.txt"];
        for name in names {
            let path = a.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("{name} 的内容")).unwrap();
        }
        for name in names.iter().rev() {
            let path = b.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("{name} 的内容")).unwrap();
        }
        assert_eq!(
            tree_digest(&a).unwrap(),
            tree_digest(&b).unwrap(),
            "两份内容相同的树给出了不同的摘要（遍历次序没定？）"
        );
    }

    /// 空目录与非常规条目都不进摘要——两者都不是「内容」。
    ///
    /// 空目录那一半钉的是「目录自身不喂任何字节」：[`approve_integration`] 的文档明说
    /// `mkdir <base>/空目录` 不会使批准值失配，本用例是那句话的用例。FIFO 那一半同时钉
    /// 「不会被读」：若实现改成「不是目录就读内容」，读一个没有写者的 FIFO 会**阻塞**，
    /// 本用例会挂住而不是失败。`mkfifo` 取不到时跳过 FIFO 一段并打标记（与
    /// `the_tree_diff_compares_symlinks_by_their_target_and_skips_odd_entries` 同法）。
    #[test]
    fn the_tree_digest_ignores_empty_directories_and_odd_entries() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        for root in [&a, &b] {
            std::fs::create_dir_all(root.join("子目录")).unwrap();
            std::fs::write(root.join("一样.txt"), "一样").unwrap();
        }
        // 只在 a 里多出：一个空目录，以及（若本机有 `mkfifo`）一个 FIFO
        std::fs::create_dir_all(a.join("空目录")).unwrap();
        let fifo_made = std::process::Command::new("mkfifo")
            .arg(a.join("管道"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        // 断言消息随「FIFO 那一段这次跑没跑」而变：跳过时，失败文本里也要看得出本轮
        // 只验了空目录这一半——否则一条被跳过的分支会让这条用例看起来什么都验过了。
        let scope = if fifo_made {
            "空目录或非常规条目（FIFO）进了摘要"
        } else {
            "空目录进了摘要（FIFO 一段本次跳过：mkfifo 不可用，本轮只执行了空目录这一半）"
        };
        if !fifo_made {
            eprintln!(
                "【跳过】the_tree_digest_ignores_empty_directories_and_odd_entries 的 FIFO 一段：\
                 mkfifo 不可用。空目录那一段仍然执行。"
            );
        }
        assert_eq!(tree_digest(&a).unwrap(), tree_digest(&b).unwrap(), "{scope}");
    }

    /// 拒绝的报文里不出现摘要本身——连它的 `Debug` 形式也不出现。
    ///
    /// 要在 crate 内才做得到：这里能同时拿到**重算出来的那一枚**与批准值携带的那一枚，
    /// 而 `tests/gate.rs` 那边（`an_approval_for_one_integration_does_not_authorize_another`）
    /// 只拿得到后者。按十六进制扫是不够的——`[u8; 32]` 的 `Debug` 是一串十进制数字，
    /// 那种写法整串都能溜过去，故这里直接拿 `Debug` 的渲染结果去查。
    ///
    /// 报文里该有的是**这次**集成的标识（`verify_approval` 那句 `reason`），那才是读错误
    /// 的人能用上的东西。
    #[test]
    fn the_mismatch_reason_does_not_leak_the_digest() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
        let approval = approve_integration(&base, &task, backend).unwrap();
        // 铸造之后改 Base：写入时重算出来的那一枚与批准值里的那一枚**都**不该出现在报文里
        std::fs::write(base_path.join("要删的.txt"), "用户在 Base 上改的\n").unwrap();
        let recomputed = integration_digest(&base, &task, backend).unwrap();

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &approval)
            .unwrap_err();
        let GateError::ApprovalMismatch { reason } = &err else {
            panic!("期望 ApprovalMismatch，得到 {err:?}");
        };
        assert!(
            !reason.contains(&format!("{recomputed:?}")),
            "报文里出现了重算出的摘要：{reason}"
        );
        assert!(
            !reason.contains(&format!("{approval:?}")),
            "报文里出现了批准值携带的摘要：{reason}"
        );
        assert!(
            reason.contains("intent=i1") && reason.contains("backend=worktree"),
            "报文里没有这次集成的标识：{reason}"
        );
    }

    /// 摘要重算遇到读不到的条目时，报的是**那个 I/O 错误**，不是「批准值不符」。
    ///
    /// 两件事的区别是实质的：I/O 失败连「现在是什么」都读不出，而「不符」说的是「现在
    /// 不是批准时的那一份」。把它们折叠成同一个拒绝，会让「重新铸一枚就能过」这种误导性
    /// 的处置看起来可行（见 [`IntegrationGate::verify_approval`] 的文档）。
    ///
    /// 权限位对 root 不起作用，故 root 下显式跳过并打标记（与本模块既有的那条同法）。
    #[test]
    fn an_unreadable_tree_reports_the_io_error_instead_of_a_mismatch() {
        if running_as_root() {
            eprintln!(
                "【跳过】an_unreadable_tree_reports_the_io_error_instead_of_a_mismatch：\
                 本进程以 root 运行，权限位挡不住读取。本用例未执行断言。"
            );
            return;
        }
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();

        // 铸造时读得通，之后 Base 里一个条目读不出来
        let approval = approve_integration(&base, &task, backend).unwrap();
        let unreadable = base_path.join("要删的.txt");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &approval)
            .unwrap_err();
        match &err {
            GateError::Workspace(WorkspaceError::IoFailed { path, .. }) => {
                assert_eq!(path, &unreadable, "错误里的路径不是读不到的那一个：{err}");
            }
            other => panic!("期望读失败原样报出（而不是 ApprovalMismatch），得到 {other:?}"),
        }
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// 同一份集成铸两次得到同一枚摘要。
    ///
    /// 钉的是「同一输入必得同一摘要」（设计 §7.2 的摘要要求）里最容易被破坏的那一半：
    /// 摘要里掺进任何随调用而变的东西（时刻、随机量、进程内的计数器），这条就红。
    /// **不去动两棵树**——两次调用之间只有一个 `Tx` 的事实无关，摘要不碰数据库。
    #[test]
    fn minting_twice_on_an_unchanged_integration_gives_the_same_approval() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();

        let first = approve_integration(&base, &task, backend).unwrap();
        let second = approve_integration(&base, &task, backend).unwrap();
        assert_eq!(
            first.0, second.0,
            "同一份集成铸出的两枚批准值不同：摘要里掺进了随调用而变的东西"
        );
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

    /// 为**当前**这两棵树铸一枚批准值。
    ///
    /// 用法是在调用点上内联（`&mint(&base, &task, backend)`）：摘要因而在写入操作之前
    /// 的那一刻算出，与实现里「写入前重算」的时序一致。先铸、中间再改树的用例自己
    /// 把值接住。
    fn mint(base: &BaseWorkspace, task: &TaskWorkspace, backend: WorkspaceBackend) -> GateApproval {
        approve_integration(base, task, backend).unwrap()
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
            .apply_patch(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
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
            .cherry_pick(&tx, &task, backend, &[&first], 1_000, &mint(&base, &task, backend))
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
            .merge(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
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
                    .apply_patch(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
                    .unwrap(),
                "cherry_pick" => gate
                    .cherry_pick(&tx, &task, backend, &[&commit], 1_000, &mint(&base, &task, backend))
                    .unwrap(),
                "merge" => gate
                    .merge(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
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

    /// overlay 后端上 `cherry_pick` 一律拒绝，**不退化为「并入全部改动」**（设计 6.3）。
    ///
    /// 退化会静默地集成得**比调用方要求的更多**：点名一个提交，落进 Base 的却是整棵树
    /// 的改动。方向危险，故宁可报错。
    ///
    /// 拒绝排在**任何后端操作之前**，故本条不需要命名空间也不需要有覆盖层——工作区是
    /// 一个普通目录即可（与 `tests/gate.rs` 的
    /// `discarding_a_foreign_root_through_the_gate_reports_the_backend_error` 同一路数）。
    #[test]
    fn cherry_pick_is_refused_on_the_overlay_backend() {
        let holder = tempfile::tempdir().unwrap();
        let base_path = holder.path().join("base");
        std::fs::create_dir(&base_path).unwrap();
        let task_root = holder.path().join("task");
        std::fs::create_dir_all(&task_root).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task = crate::TaskWorkspace::new_outside(
            &base,
            &task_root,
            crate::ids::IntentId::new("i1"),
        )
        .unwrap();
        // Task 里有真改动：若那一支退化了，这处改动就会落进 Base
        task.writable_root().write("Task 的改动.txt", b"x\n").unwrap();

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .cherry_pick(
                &tx,
                &task,
                WorkspaceBackend::Overlay,
                &["HEAD~1"],
                1_000,
                &mint(&base, &task, WorkspaceBackend::Overlay),
            )
            .unwrap_err();

        match &err {
            GateError::Workspace(WorkspaceError::GateRefused { reason }) => assert!(
                reason.contains("提交粒度"),
                "拒绝的理由没说清是覆盖层没有提交粒度：{reason}"
            ),
            other => panic!("期望 GateRefused，得到 {other:?}"),
        }
        assert!(
            !base_path.join("Task 的改动.txt").exists(),
            "被拒绝的调用把 Task 的改动写进了 Base（退化了）"
        );
        assert!(audit_rows(&tx).is_empty(), "被拒绝的调用写了审计记录");
    }

    /// overlay 后端上给**空**列表时同样拒绝——**两个后端对「空列表」的处置一致**。
    ///
    /// 空列表的语义是「摘这些提交」中的「没有提交」，是**调用方错误**（忘了点名），不是
    /// 「请求全部」：要全部有 [`IntegrationGate::merge`]。让 overlay 侧把它退化成「并入
    /// 全部」，会让同一个调用在两种后端上给出相反处置（worktree 拒、overlay 并），而
    /// 调用方按后端分派时正会以为空列表处处等价——那比单侧的静默多集成更隐蔽。
    #[test]
    fn cherry_pick_with_an_empty_list_is_refused_on_the_overlay_backend() {
        let holder = tempfile::tempdir().unwrap();
        let base_path = holder.path().join("base");
        std::fs::create_dir(&base_path).unwrap();
        std::fs::write(base_path.join("要改的.txt"), "lower\n").unwrap();
        let task_root = holder.path().join("task");
        std::fs::create_dir_all(&task_root).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task = crate::TaskWorkspace::new_outside(
            &base,
            &task_root,
            crate::ids::IntentId::new("i1"),
        )
        .unwrap();
        task.writable_root().write("要改的.txt", b"upper\n").unwrap();
        task.writable_root().write("新增.txt", "新\n".as_bytes()).unwrap();

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .cherry_pick(
                &tx,
                &task,
                WorkspaceBackend::Overlay,
                &[],
                1_000,
                &mint(&base, &task, WorkspaceBackend::Overlay),
            )
            .unwrap_err();

        // 与 worktree 侧（`cherry_pick_refuses_an_empty_commit_list`）**同一种错误**：
        // 「空列表」在两个后端上是同一件事——调用方错误，两边都拒。
        match &err {
            GateError::Workspace(WorkspaceError::GateRefused { reason }) => assert!(
                reason.contains("提交粒度"),
                "拒绝的理由没说清是覆盖层没有提交粒度：{reason}"
            ),
            other => panic!("期望 GateRefused，得到 {other:?}"),
        }
        assert_eq!(
            std::fs::read(base_path.join("要改的.txt")).unwrap(),
            b"lower\n",
            "被拒绝的调用把 Task 的改动并进了 Base（退化了）"
        );
        assert!(
            !base_path.join("新增.txt").exists(),
            "被拒绝的调用把 Task 的新增文件写进了 Base（退化了）"
        );
        assert!(audit_rows(&tx).is_empty(), "被拒绝的调用写了审计记录");
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
            .cherry_pick(&tx, &task, backend, &[], 1_000, &mint(&base, &task, backend))
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
            .apply_patch(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
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

    /// Task 只改了未跟踪文件时，`apply_patch` 照样把它带进 Base（空补丁不送给 git）。
    ///
    /// `git apply` 在空输入下以退出码 128 失败（实测 git 2.56：
    /// 「No valid patches in input (allow with "--allow-empty")」），故本层必须先判空。
    /// 少了这一步，「Task 只加了一个新文件」这种最常见的集成会被报成 git 故障。
    #[test]
    fn apply_patch_with_only_untracked_files_still_integrates_them() {
        let (_d, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        // 一个新文件，且**不**提交、不 git add：补丁里因此什么也没有
        std::fs::write(task.root().join("只有一个新文件.txt"), "内容\n").unwrap();
        let head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
            .unwrap();
        tx.commit().unwrap();

        assert_eq!(
            std::fs::read_to_string(base_path.join("只有一个新文件.txt")).unwrap(),
            "内容\n",
            "只改未跟踪文件时那处改动没有进 Base"
        );
        assert_eq!(
            run_git(&base_path, &["rev-parse", "HEAD"]),
            head_before,
            "apply_patch 不该替用户提交"
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
            .apply_patch(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
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

    /// Base 全树的递归快照：每个条目的相对路径 →（种类, 内容）。
    ///
    /// **含 `.git/`，且按整棵树取。** 守卫用例要钉的是「返回 `Err` 之前 Base 有没有被
    /// 动过」，只查几个已知文件名不够——被删掉的恰恰是那些不在预期清单里的东西。目录也
    /// 记一条（空目录同样是一处改动），符号链接记目标而非目标的内容。
    fn tree_snapshot(root: &Path) -> BTreeMap<PathBuf, (char, Vec<u8>)> {
        fn walk(root: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, (char, Vec<u8>)>) {
            let entries = std::fs::read_dir(root.join(rel)).expect("快照：读目录失败");
            for entry in entries {
                let entry = entry.expect("快照：读条目失败");
                let rel = rel.join(entry.file_name());
                let full = root.join(&rel);
                let file_type = entry.file_type().expect("快照：取条目种类失败");
                if file_type.is_dir() {
                    out.insert(rel.clone(), ('d', Vec::new()));
                    walk(root, &rel, out);
                } else if file_type.is_symlink() {
                    let target = std::fs::read_link(&full).expect("快照：读链接失败");
                    out.insert(rel, ('l', target.into_os_string().into_vec()));
                } else {
                    out.insert(rel, ('f', std::fs::read(&full).expect("快照：读文件失败")));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, Path::new(""), &mut out);
        out
    }

    /// 守卫用例的夹具：一个真的 Git 仓库作 Base，外加一个由 **worktree 后端**建出的
    /// 真 worktree 作 Task（根里 `.git` 是**文件**，且根落在 `<base>/.ai/worktrees/<intent>`）。
    ///
    /// 返回保活用的 `TempDir`、Base 路径与 Task。用真的后端建 Task 而不手工摆目录：
    /// 要拦的正是「调用方把一个 worktree 建出的 Task 交给 overlay 路径」，
    /// 手工摆出来的形状可能漏掉真实现里的某一处。
    fn worktree_task_fixture() -> (tempfile::TempDir, PathBuf, TaskWorkspace) {
        let (dir, base_path) = git_repo();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        assert_eq!(
            backend,
            WorkspaceBackend::Worktree,
            "夹具本身不是 worktree 后端建的"
        );
        // 让 Task 有一处**真实的**改动：被拒绝时它是「本来会落进 Base 的改动」，
        // 用例才好把「拒绝」与「无事可做」分开。
        std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
        (dir, base_path, task)
    }

    /// **Critical 的守卫：`apply_patch` 在形态不匹配时必须于动手之前拒绝。**
    ///
    /// 处境：Base 是真的 Git 仓库（`.git` 是**目录**，里头有 HEAD、config、objects、
    /// refs），Task 是 worktree 后端建出的真 worktree（根里 `.git` 是**文件**），而
    /// `backend` 被给成 `Overlay`。
    ///
    /// 早先的 `integrate_overlay` 是「边删边发现」：删除那一趟按 `base - task` 把
    /// `base/.git/**` 逐个 `remove_file`（`collect_files` 只收文件，而 `remove_entry`
    /// 只对**目录**拒绝，故这些文件一路放行），直到复制那一趟走到 `task/.git` 才因目标是
    /// 目录而拒绝——**返回 `Err` 之前，用户的仓库已经被删掉一大片**。
    ///
    /// 现须在集成之前整体拒绝，且**Base 全树逐字节未变**（含 `.git/`，按整棵树比对）。
    #[test]
    fn a_worktree_task_with_the_overlay_backend_is_refused_before_any_change() {
        let (_d, base_path, task) = worktree_task_fixture();
        let base = BaseWorkspace::new(&base_path).unwrap();
        assert!(base_path.join(".git/HEAD").exists(), "夹具的 .git 不成形");
        let before = tree_snapshot(&base_path);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .apply_patch(&tx, &task, WorkspaceBackend::Overlay, 1_000, &mint(&base, &task, WorkspaceBackend::Overlay))
            .unwrap_err();

        assert!(
            matches!(
                &err,
                GateError::Workspace(WorkspaceError::BackendUnavailable { .. })
            ),
            "期望按「不是本后端的布局」拒绝，实际 {err:?}"
        );
        assert_eq!(
            tree_snapshot(&base_path),
            before,
            "被拒绝的集成改动了 Base（含 .git/）"
        );
        assert!(
            audit_rows(&tx).is_empty(),
            "被拒绝的操作写了审计记录（拒绝应当排在任何变更之前）"
        );
    }

    /// 同上，走 `merge`。两个操作在 overlay 分支上是同一个动作（设计 6.3），
    /// 故守卫必须两条路都在——只钉一条的话，另一条漏掉这道校验不会被发现。
    #[test]
    fn a_worktree_task_with_the_overlay_backend_is_refused_by_merge_too() {
        let (_d, base_path, task) = worktree_task_fixture();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let before = tree_snapshot(&base_path);

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        let err = IntegrationGate::new(&base)
            .merge(&tx, &task, WorkspaceBackend::Overlay, 1_000, &mint(&base, &task, WorkspaceBackend::Overlay))
            .unwrap_err();

        assert!(
            matches!(
                &err,
                GateError::Workspace(WorkspaceError::BackendUnavailable { .. })
            ),
            "期望按「不是本后端的布局」拒绝，实际 {err:?}"
        );
        assert_eq!(
            tree_snapshot(&base_path),
            before,
            "被拒绝的集成改动了 Base（含 .git/）"
        );
    }

    /// **两阶段的守卫：能从计划本身判定的失败，必须在动第一个字节之前报出。**
    ///
    /// 本条**绕过** [`IntegrationGate`] 直接调 `integrate_overlay`：形态校验在 Gate 那一层
    /// （见上面两条），而本条要钉的是两阶段本身，故不能让它先把误用拦掉。
    ///
    /// 处境取 `.git` 那一例的形状而不含 git：Base 在 `sub` 那一级是**目录**、Task 在那一级
    /// 是**文件**。一步式实现会把 `sub/x.txt` 先移掉，复制那一趟才因目标是目录而拒绝——
    /// 于是 Base 少了一个文件。两阶段则整体拒绝，Base 一字不动。
    #[test]
    fn a_refused_overlay_plan_leaves_the_base_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        std::fs::create_dir_all(base.join("sub")).unwrap();
        std::fs::write(base.join("sub/x.txt"), "只在 Base\n").unwrap();
        std::fs::write(base.join("留着.txt"), "也是 Base 的\n").unwrap();
        std::fs::create_dir_all(&task).unwrap();
        std::fs::write(task.join("sub"), "Task 侧这一级是文件\n").unwrap();

        let before = tree_snapshot(&base);
        let err = integrate_overlay(&base, &task).unwrap_err();

        assert!(
            matches!(&err, GateError::Workspace(WorkspaceError::GateRefused { .. })),
            "期望「目标是目录」被拒，实际 {err:?}"
        );
        assert_eq!(tree_snapshot(&base), before, "被拒绝的计划改动了 Base");
    }

    /// **`.git` 不进差异集，真正的改动照旧落进 Base。**
    ///
    /// 两侧的 `.git` 取**不同形态**（Base 是目录、Task 是文件）——这正是 worktree 那一例的
    /// 形状。若把 `.git` 当内容，Diff 会报出「Base 侧一整棵 `.git` 被删」加上「多了一个
    /// `.git`」，而后者正是把整棵删除合法化的那一步。
    ///
    /// 这条同时钉住排除的**边界**：排除只对 `.git` 生效，`.gitignore`、`.gitmodules`
    /// 一类同前缀的名字仍是内容，该报的要报。
    #[test]
    fn dot_git_is_never_part_of_the_diff_and_the_real_changes_still_land() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        std::fs::create_dir_all(base.join(".git/objects/ab")).unwrap();
        std::fs::write(base.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(base.join(".git/objects/ab/cdef"), b"\x00\x01").unwrap();
        std::fs::write(base.join("要改的.txt"), "base\n").unwrap();
        std::fs::write(base.join("要删的.txt"), "只在 Base\n").unwrap();
        std::fs::write(base.join(".gitignore"), "target/\n").unwrap();
        std::fs::create_dir_all(&task).unwrap();
        std::fs::write(task.join(".git"), "gitdir: /x/.git/worktrees/i1\n").unwrap();
        std::fs::write(task.join("要改的.txt"), "task\n").unwrap();
        std::fs::write(task.join(".gitignore"), "target/\n").unwrap();

        let diff = diff_trees(&base, &task).unwrap();
        for group in [diff.added(), diff.modified(), diff.deleted()] {
            assert!(
                !group.iter().any(|p| p.starts_with(".git")),
                ".git 进了差异集：{group:?}"
            );
        }
        assert_eq!(
            diff.modified(),
            &[PathBuf::from("要改的.txt")][..],
            "真正的改动没有报出：{diff:?}"
        );
        assert_eq!(
            diff.deleted(),
            &[PathBuf::from("要删的.txt")][..],
            "真正的删除没有报出：{diff:?}"
        );
        assert!(diff.added().is_empty(), "多出了不该有的新增：{diff:?}");

        integrate_overlay(&base, &task).unwrap();
        assert_eq!(
            std::fs::read_to_string(base.join("要改的.txt")).unwrap(),
            "task\n"
        );
        assert!(!base.join("要删的.txt").exists(), "删除没有落进 Base");
        assert!(base.join(".git/HEAD").exists(), "集成动了 .git/HEAD");
        assert!(
            base.join(".git/objects/ab/cdef").exists(),
            "集成动了 .git/objects"
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

    // ===== 真挂载的覆盖层上的集成（需要用户与挂载命名空间）=====
    //
    // 上面几条 overlay 用例都在**普通目录**上跑 [`integrate_overlay`]，验的是「比较两棵树
    // + 把差别写过去」这一段。而它在设计里是 overlay 后端的**唯一通道**（§257），故还要在
    // **真挂载**的覆盖层上走一次整条路：Task 根是覆盖层的挂载点，读它要经内核的合并视图，
    // 写入要落回 Base。
    //
    // 脚手架与 `tests/gate.rs`、`tests/backend_overlay.rs` 同源（那两处也是各留一份：单测
    // 与集成测试是两个 crate，取不到对方的东西）。要点有三：把自身重新执行进 `unshare -Urm`
    // 由子进程断言；overlay 根指到只属于本次用例的临时目录（**绝不能**落到用户 home 下的
    // 默认位置）；环境不具备时**显式跳过并打标记**，不静默通过。

    /// 标识「本进程是被重新执行出来的子进程」，防止再次 re-exec 造成无限递归。
    const CHILD_ENV: &str = "CONTINUUM_GATE_LIB_CHILD";

    /// overlay 根：由父进程指到一个只属于本次用例的临时目录。
    const OVERLAY_ROOT_ENV: &str = "CONTINUUM_OVERLAY_ROOT";

    fn is_child() -> bool {
        std::env::var_os(CHILD_ENV).is_some()
    }

    /// 环境不具备本用例所需条件时的**显式**跳过标记。
    ///
    /// 不静默通过：跳过原因打到 stderr（cargo 在用例通过时不回显，`--nocapture` 可见）。
    /// **并给出本条用例的执行/跳过条数**——按 Task 7 的教训（`continuum-sandbox` 的
    /// `tests/isolation.rs`：逐条打标记、`ran > 0` 兜底），标记要能让人一眼看出
    /// 「这条路径这次到底验没验」，而不是只留一句「跳过了」。
    fn skip(test: &str, reason: &str) {
        eprintln!(
            "【跳过】{test}：{reason}。本条：执行 0、跳过 1——**本次运行没有在挂载态的覆盖层上\
             验证集成**。用 `--nocapture` 可见本行。"
        );
    }

    /// 把本用例重新执行进 `unshare -Urm` 的子进程，并核对子进程的退出码。
    ///
    /// 返回 `true` 表示「当前就是子进程，继续执行断言」；`false` 表示父进程已跑完或已跳过。
    ///
    /// **本模块只有这一条用例需要命名空间**（其余 overlay 用例都在普通目录上跑
    /// [`integrate_overlay`]），故没有 `isolation.rs` 那种「全部不可用」的处境：命名空间取不到
    /// 时，覆盖层在本机根本建不出来（`create_task_workspace` 会以 `NotInNamespace` 拒绝），
    /// 整个 overlay 后端都无从验证——那与 `tests/gate.rs`、`tests/backend_overlay.rs` 的处置
    /// 一致，都是跳过并打标记，而不是把本机能力不足报成实现缺陷。
    fn enter_namespace(test_name: &str) -> bool {
        if is_child() {
            return true;
        }
        let exe = std::env::current_exe().expect("无法取到测试可执行文件的路径");
        let exe = exe
            .to_str()
            .expect("测试可执行文件的路径不是合法 UTF-8")
            .to_owned();
        // 探测方式是把**本测试二进制**以 `--list` 重新执行进 `unshare -Urm`：用自身而非
        // `/bin/true` 一类的替身，因为要探的正是「本进程能否被这样执行」。
        let available = std::process::Command::new("unshare")
            .args(["-Urm", &exe, "--list"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if !available {
            skip(test_name, "unshare -Urm 无法建立用户与挂载命名空间");
            return false;
        }
        let holder = tempfile::tempdir().unwrap();
        let root: OsString = holder.path().join("overlays").into_os_string();
        let out = std::process::Command::new("unshare")
            .args(["-Urm", &exe, test_name, "--exact"])
            .env(CHILD_ENV, "1")
            .env(OVERLAY_ROOT_ENV, &root)
            .output()
            .unwrap_or_else(|e| panic!("无法重新执行 {exe}：{e}"));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "子进程 {test_name} 失败（退出码 {:?}）：\n--- stdout ---\n{stdout}\n--- stderr ---\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        // **子进程必须真的跑过那一条用例。** libtest 的过滤器对不上任何名字时以「0 tests
        // run」退出 0——只看退出码的话，这条用例会**静默变绿**：什么都没验，却报通过。
        // （实测过一次：单测二进制里的用例全名带模块路径 `gate::tests::…`，只传函数名时
        // `--exact` 匹配不上，于是断言一次也没执行。）
        //
        // 与 `tests/common/mod.rs` 的 `assert_child_ran_one`、以及 `tests/gate.rs`、
        // `tests/backend_overlay.rs` 里对它的调用**同形**：三处都在同一条断定上（退出码
        // 为 0 还不够）。单测是另一个 target，取不到 `tests/common/` 那份模块，故这一处
        // 只能自己写一遍——改判定条件时三处要一起改。
        assert!(
            stdout.contains(" 1 passed"),
            "子进程没有执行 {test_name}（过滤器对不上时 libtest 以 0 tests 退出 0，本用例\
             会静默变绿）。子进程输出：\n{stdout}"
        );
        // 与 `【跳过】` 那条对称：本条真的在挂载态上跑过了。两行合起来是本模块挂载态
        // 覆盖的**条数**凭据（执行 1、跳过 0；环境不具备时反过来），`--nocapture` 可见。
        eprintln!(
            "【运行】{test_name}：本条：执行 1、跳过 0——集成在真挂载的覆盖层上验过了。"
        );
        drop(holder);
        false
    }

    /// 建一个带 lower 内容的 Base，返回（保活用的 `TempDir`，规范化的 Base 根）。
    ///
    /// 只在子进程里调用。
    fn base_with_lower() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(root.join("要改的.txt"), "lower\n").unwrap();
        std::fs::write(root.join("要删的.txt"), "只在 lower\n").unwrap();
        (dir, root)
    }

    /// 在**真挂载**的覆盖层上经 `apply_patch` 集成：改、删（白障）、增都要落进 Base。
    ///
    /// 与 `integrating_into_the_base_materializes_the_task_changes` 的分工：那条验的是
    /// 集成这一段本身（普通目录），本条验的是**它在真覆盖层上跑得通**——Task 根是挂载点，
    /// 读它经内核的合并视图、写它落进 upper 层，而集成要在这之上把差别正确读出并写回 Base。
    /// 少了这一条，「唯一通道」在 overlay 后端上就只有合成测试。
    #[test]
    fn applying_a_patch_on_a_mounted_overlay_reaches_the_base() {
        // 名字要带模块路径：单测二进制里的用例全名是 `gate::tests::<函数名>`，而
        // `--exact` 是按全名匹配的（传函数名会让子进程跑 0 条用例并以 0 退出）。
        if !enter_namespace(
            "gate::tests::applying_a_patch_on_a_mounted_overlay_reaches_the_base",
        ) {
            return;
        }
        let (_d, base_path) = base_with_lower();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let (task, backend) =
            crate::backend::create_task_workspace(&base, &crate::ids::IntentId::new("i1")).unwrap();
        assert_eq!(backend, WorkspaceBackend::Overlay);

        // Task 侧：改一个（落进 upper）、删一个（落成白障）、加一个（含深一层的路径）
        task.writable_root().write("要改的.txt", b"upper\n").unwrap();
        std::fs::remove_file(task.root().join("要删的.txt")).unwrap();
        task.writable_root()
            .write("新目录/深层.txt", "深层新增\n".as_bytes())
            .unwrap();
        // 集成之前 Base 一字未动：挂载点读到的是合并视图，而 Base 本身还是 lower 的内容
        assert_eq!(
            std::fs::read(base_path.join("要改的.txt")).unwrap(),
            b"lower\n",
            "集成之前 Base 就已被动过"
        );

        let (_db_dir, db) = db();
        let tx = db.begin().unwrap();
        IntegrationGate::new(&base)
            .apply_patch(&tx, &task, backend, 1_000, &mint(&base, &task, backend))
            .unwrap();
        let rows = audit_rows(&tx);
        tx.commit().unwrap();

        assert_eq!(
            std::fs::read(base_path.join("要改的.txt")).unwrap(),
            b"upper\n",
            "改动没有经挂载的覆盖层落进 Base"
        );
        assert!(
            !base_path.join("要删的.txt").exists(),
            "Task 在覆盖层里删掉的文件仍在 Base 里（白障没有被读成删除）"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("新目录/深层.txt")).unwrap(),
            "深层新增\n",
            "新增的深层文件没有进 Base"
        );
        assert_eq!(rows.len(), 1, "集成应记一条审计");
        assert!(rows[0].1.contains("backend=overlay"), "{}", rows[0].1);
    }
}
