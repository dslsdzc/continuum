//! Git worktree 后端（§255）。
//!
//! Base 是 Git 仓库时，Task Workspace 是建在 `<base>/.ai/worktrees/<intent>` 的
//! worktree，分支名 `ai/<intent>`（§16）。创建、放弃都经 `git` 命令完成。
//!
//! 对 Base 的唯一写入是仓库本地的 `info/exclude`（追加一行 `.ai/`），
//! 见 [`ensure_ai_excluded`]；Base 中的跟踪文件一字不动。
//!
//! 用户的当前分支全程不被触碰：`git worktree add` 只写新建分支与主仓库的
//! worktree 登记（`.git/worktrees/`），`git worktree remove` 与 `git branch -D`
//! 各自只回收自己那一份。故 Base 的 HEAD 与工作树内容在创建/放弃前后逐字节不变。

use crate::backend::check_intent;
use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::ids::IntentId;
use crate::task::TaskWorkspace;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Base 内存放各 Intent worktree 的目录（§255）。
const WORKTREE_DIR: &str = ".ai/worktrees";

/// 仓库本地排除文件中标记 AI 私有目录的条目。
const IGNORE_ENTRY: &str = ".ai/";

/// worktree 分支名的前缀（§16）。
const BRANCH_PREFIX: &str = "ai/";

/// `<base>/.ai/worktrees/<intent>`。
pub(crate) fn worktree_path(base: &BaseWorkspace, intent: &IntentId) -> PathBuf {
    base.root().join(WORKTREE_DIR).join(intent.as_str())
}

/// `ai/<intent>`。分支名与 worktree 路径同源于 `intent`，故两者要么一起合法，
/// 要么在 [`crate::backend::check_intent`] 处一起被拒。
pub(crate) fn branch_name(intent: &IntentId) -> String {
    format!("{BRANCH_PREFIX}{intent}")
}

/// 创建一个 Task Workspace。
///
/// 次序：校验 intent → 确保 `.ai/` 被排除 → `git worktree add` →
/// 由 [`TaskWorkspace::new_outside`] 产出句柄。
///
/// **不做「先建目录再建 worktree」**：`git worktree add` 在目标目录已存在且非空时
/// 会失败（退出码 128），而在目录不存在时自行建出全部中间目录。让 git 建目录，
/// 失败路径上就没有需要回收的半成品。
pub(crate) fn create(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<TaskWorkspace, WorkspaceError> {
    check_intent(intent)?;
    ensure_ai_excluded(base.root())?;
    let path = worktree_path(base, intent);
    let path_arg = path_str(&path)?;
    let branch = branch_name(intent);
    git(base.root(), &["worktree", "add", "-b", &branch, path_arg])?;
    // worktree 已落盘，此处 `new_outside` 只做重叠判定与规范化。
    // 路径位于 Base 之内，重叠判定必然放行；保留该调用是为了让句柄的产出
    // 走唯一的公开入口，`TaskWorkspace` 的构造路径不因后端而分岔。
    TaskWorkspace::new_outside(base, &path, intent.clone())
}

/// 放弃一个 Task Workspace：移除 worktree 目录，再删除其分支。
///
/// 先移除 worktree 再删分支：分支被 worktree 检出时 `git branch -D` 会拒绝，
/// 反序会把「worktree 已移除但分支还在」变成常态。反过来的中间态（目录已移除、
/// 分支尚存）只在第二步失败时出现，此时返回 `Err` 而非静默。
pub(crate) fn discard(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<(), WorkspaceError> {
    let path_arg = path_str(task.root())?;
    git(base.root(), &["worktree", "remove", "--force", path_arg])?;
    git(base.root(), &["branch", "-D", &branch_name(task.intent_id())])?;
    Ok(())
}

/// 取出可传给 `git` 的路径参数。非 UTF-8 的路径无法作为命令行参数传递。
fn path_str(path: &Path) -> Result<&str, WorkspaceError> {
    path.to_str().ok_or_else(|| WorkspaceError::BackendUnavailable {
        reason: format!("路径不是合法 UTF-8，无法传给 git：{}", path.display()),
    })
}

/// Base 的仓库本地排除文件路径，即 git 真正会读的那一份。
///
/// 用 `git rev-parse --git-path info/exclude` 而非硬拼 `<base>/.git/info/exclude`：
/// Base 本身是一个 linked worktree 时 `.git` 是**文件**，硬拼会指向一个不存在的
/// 路径；该命令给出的是 git 实际读取的位置——linked worktree 下是公共仓库的
/// `<common>/.git/info/exclude`，这也正是 git 在那种情形下唯一读的一份。
///
/// 输出可能是相对路径（普通仓库为 `.git/info/exclude`）也可能是绝对路径，
/// 故一律以 `base.join` 归一：`PathBuf::join` 遇到绝对路径会直接采用它。
fn exclude_path(base: &Path) -> Result<PathBuf, WorkspaceError> {
    let out = git(base, &["rev-parse", "--git-path", "info/exclude"])?;
    if out.is_empty() {
        return Err(WorkspaceError::BackendUnavailable {
            reason: format!("git 未给出 info/exclude 的路径：{}", base.display()),
        });
    }
    Ok(base.join(out))
}

/// 确保 `.ai/` 被 Base 的仓库排除，缺失时**追加**一行。
///
/// `.ai/` 被 git 视为未跟踪目录，不排除则它会出现在用户的 `git status` 里——
/// 用户没做任何事，状态栏却脏了。设计上 `.ai/` 是 Runtime 的私有目录（§255），
/// 让它对用户不可见是后端的义务。
///
/// **写 `.git/info/exclude` 而非 `.gitignore`。** 后者是用户的跟踪文件：改它会让
/// 用户的工作树凭空多出一处并非用户所做的未提交修改，而 §256 要求 Base 只读。
/// `info/exclude` 正是 git 为这种用途提供的文件——仓库本地、不进工作树、不是
/// 跟踪文件、不影响其他 clone。本后端因此不触碰 Base 中的任何跟踪文件。
///
/// 该文件仍由 git 预置了注释，故只追加：先读原内容，**未列 `.ai` 才**在末尾补一行。
/// 原内容不以换行结束时先补一个换行，否则原末行与 `.ai/` 会粘成一条。
/// 读写成字节而非 `String`：非 UTF-8 的排除文件不该因本函数而损坏。
fn ensure_ai_excluded(base: &Path) -> Result<(), WorkspaceError> {
    let path = exclude_path(base)?;
    let existing = match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_error(&path, e)),
    };
    if let Some(bytes) = &existing {
        let text = String::from_utf8_lossy(bytes);
        if text
            .lines()
            .any(|line| matches!(line.trim(), ".ai" | ".ai/"))
        {
            return Ok(());
        }
    }
    let mut out = existing.unwrap_or_default();
    if !out.is_empty() && !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    out.extend_from_slice(IGNORE_ENTRY.as_bytes());
    out.push(b'\n');
    std::fs::write(&path, out).map_err(|e| io_error(&path, e))
}

/// 在 `dir` 内执行一条 git 命令，返回裁剪后的 stdout；非零退出即 `GitFailed`。
///
/// 以退出码判定成败，不看 stderr：`git worktree add` 成功时也会往 stderr
/// 打印 "Preparing worktree ..."。失败时把 code 与 stderr 原样带回，
/// 截断或改写都会让调用方丢掉 git 给出的具体原因。
fn git(dir: &Path, args: &[&str]) -> Result<String, WorkspaceError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| WorkspaceError::BackendUnavailable {
            reason: format!("无法执行 git：{e}"),
        })?;
    if !output.status.success() {
        return Err(WorkspaceError::GitFailed {
            // 被信号杀死时没有退出码；git 自己不会这样退出，故取 -1 仅作占位。
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim_end().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn io_error(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::IoFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}
