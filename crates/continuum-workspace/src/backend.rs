//! 后端的判别与分派。
//!
//! 后端由 **Base 是否为 Git 仓库**决定，上层不感知差异：调用方只经本模块的
//! 三个函数取用 Task Workspace，不直接引用具体后端。后端在创建时确定并随
//! 句柄一同返回，使后续的放弃不必重新探测——Base 中途变成/不再是仓库时，
//! 按创建时记录的判据回收才对得上当时建出来的东西。

use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::ids::IntentId;
use crate::overlay;
use crate::task::TaskWorkspace;
use crate::worktree;
use serde::{Deserialize, Serialize};

/// Task Workspace 的实现形态。
///
/// 两个后端对 Intent 标识的合法性要求是同一条：**单个路径分量**。判定在
/// `check_intent` 一处实现，两个后端共用，不在各自的创建路径里另立一套。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceBackend {
    /// Base 是 Git 仓库：每个 Intent 一个 worktree 与一条独立分支（§255、§16）。
    ///
    /// 合法 Intent 标识：**单个路径分量**——非空、不为 `.` 或 `..`、不含 `/` 与 `\`。
    Worktree,
    /// Base 不是 Git 仓库：可写覆盖层（§255）。
    ///
    /// 合法 Intent 标识：**单个路径分量**——非空、不为 `.` 或 `..`、不含 `/` 与 `\`。
    /// 与 [`WorkspaceBackend::Worktree`] 同一条规则，由同一处判定。
    Overlay,
}

/// Intent 标识必须能作为**单个**路径分量。
///
/// 两个后端都把 Intent 嵌进路径，故这条规则是它们共用的（见 [`WorkspaceBackend`]）。
/// 缺了这一步，`IntentId::new("../../x")` 会让后端算出的路径越出各自的私有目录
/// ——worktree 后端下它与分支名一并越出 `.ai/worktrees`，而
/// `TaskWorkspace::new_outside` 拦不住：折叠之后它落在 Base 之内，反而通过重叠判定。
/// 本层是边界层，边界由本层的判定给出，不靠 git 的 refname 校验兜底。
///
/// `.` 与空串会让路径退化成私有目录自身（`.ai/worktrees`、overlay 的 Intent 目录），
/// 同样拒绝。含 `\` 的标识在 Windows 上是分隔符，一并拒绝以免两端行为分岔。
///
/// **本函数的职责边界是「路径逃逸」**，不覆盖 git 的全部 refname 规则（空格、`@{`、
/// 结尾 `.lock` 等）。分工是：这里管路径，refname 合法性由 git 在创建的第一步兜底，
/// 而那一步失败时仓库里不会多出任何东西（见 [`crate::worktree::create`] 的两步次序）。
/// 实测 `@` 一类标识能过 git 的 `branch` 却在检出 worktree 时失败——那条路径由
/// `create` 的回滚负责，同样不留痕迹。故这里不必再列一份 refname 黑名单：
/// 黑名单会与 git 的实际规则漂移，而漂移的后果是漏放或误拒。
pub(crate) fn check_intent(intent: &IntentId) -> Result<(), WorkspaceError> {
    let raw = intent.as_str();
    let is_single_component = !raw.is_empty()
        && raw != "."
        && raw != ".."
        && !raw.contains('/')
        && !raw.contains('\\')
        && std::path::Path::new(raw).components().count() == 1;
    if is_single_component {
        return Ok(());
    }
    Err(WorkspaceError::InvalidIntent {
        intent: raw.to_owned(),
    })
}

/// Base 是否为 Git 仓库决定后端。
///
/// 判据是 `<base>/.git` 存在（目录或文件——worktree 中它是文件）。**不使用
/// `git rev-parse`**：它会把父目录的仓库也算进来，而本函数的语义是「Base 自身
/// 是不是一个仓库」。Base 位于某个更大仓库的子目录时，用 worktree 后端会在
/// 那个外层仓库里建分支，任务的可写范围随之扩到 Base 之外。
pub fn detect_backend(base: &BaseWorkspace) -> WorkspaceBackend {
    if base.root().join(".git").exists() {
        WorkspaceBackend::Worktree
    } else {
        WorkspaceBackend::Overlay
    }
}

/// 依后端创建 Task Workspace，并返回本次采用的后端。
///
/// 返回后端是让调用方在放弃时有据可依：判据只在创建时成立，事后 Base 的形态
/// 可能已变（例如用户在建好之后才 `git init`）。
///
/// 两个后端都在各自的创建路径的开头调用 [`check_intent`]，故 `IntentId` 的路径逃逸
/// 判定与后端无关，且**先于**后端自身的环境判定（overlay 的命名空间要求）报出。
pub fn create_task_workspace(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<(TaskWorkspace, WorkspaceBackend), WorkspaceError> {
    let backend = detect_backend(base);
    let task = match backend {
        WorkspaceBackend::Worktree => worktree::create(base, intent)?,
        WorkspaceBackend::Overlay => overlay::create(base, intent)?,
    };
    Ok((task, backend))
}

/// 依创建时记录的后端放弃一个 Task Workspace。
///
/// 后端由调用方传入而非重新探测：重新探测会在 Base 形态已变时按**另一个**后端
/// 去回收，留下原后端的残留（worktree 目录或分支）而不报错。
///
/// # 与落库记录的关系
///
/// 若该工作区曾由 `save_workspace` 落库，**放弃之后必须一并删除其记录**
/// （`continuum_workspace::remove_workspace`），且两件事应放进**同一个事务**。
/// 本函数不收 `Tx`、也不碰数据库——它只动文件系统与 git，故这件事由调用方
/// 促成。只放弃工作区而不删记录，库里就留下一条指向已删工作区的记录：
/// 此后 `load_workspace` 给出的后端与路径都不再对应任何实物，按它去回收只会
/// 失败（worktree 目录或分支已不在），而那条记录本身看不出已经作废。
pub fn discard_task_workspace(
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<(), WorkspaceError> {
    match backend {
        WorkspaceBackend::Worktree => worktree::discard(base, task),
        // overlay 的布局全部由 Task 根推出（上层 `upper`/`work` 是它的兄弟），
        // 故不需要 Base——这一点与 worktree 后端不同，后者要经 `-C <base>` 调 git。
        WorkspaceBackend::Overlay => overlay::discard(task),
    }
}
