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
/// 次序：校验 intent → 确保 `.ai/` 被排除 → 建分支 → 检出 worktree →
/// 由 [`TaskWorkspace::new_outside`] 产出句柄。
///
/// **建分支与检出 worktree 分作两步，不写成单条 `git worktree add -b`。**
/// 那条命令实际是「先建分支 ref，再建 worktree」两个动作，第二阶段失败时
/// 分支会留在用户仓库里：实测目标目录已存在且非空时报 `already exists`（退出码
/// 128），intent 为 `@` 时报 `could not find created worktree '@'`（退出码 255），
/// 两种情形都留下一条 `ai/<intent>` ref。留下的 ref 有两个后果：它是本 task
/// 唯一一处「失败后在用户仓库里留下痕迹」的路径（痕迹是仓库元数据的写入），
/// 且会让**同一 intent 永远无法再创建**（重试报 `a branch named … already exists`）。
/// 拆开之后，第一阶段失败时仓库里什么都没多出来——`git branch` 是单个动作。
///
/// 第二阶段失败时回收刚建的分支再返回 git 的原始错误。不预建目标目录：
/// `git worktree add` 在目录不存在时自建全部中间目录，让 git 建目录，
/// 失败路径上就没有需要回收的目录。
pub(crate) fn create(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<TaskWorkspace, WorkspaceError> {
    check_intent(intent)?;
    let path = worktree_path(base, intent);
    let path_arg = path_str(&path)?;
    let branch = branch_name(intent);
    // `.ai/` 在本次之前是否已存在。失败时只清自己建出来的那一份目录，
    // 别的 Intent 的 worktree 可能就住在里面。
    let ai_dir_preexisting = base.root().join(WORKTREE_DIR).exists();

    git(base.root(), &["branch", &branch])?;

    // 不带 `-b`：分支已经建好，这里只把工作树检出到它上面。
    if let Err(err) = git(base.root(), &["worktree", "add", path_arg, &branch]) {
        // `git worktree add` 会先把中间目录建出来再失败（intent 为 `@` 时实测
        // 留下空的 `.ai/worktrees/`），故这一步的失败也要把目录清掉
        if !ai_dir_preexisting {
            remove_empty_ai_dirs(base.root());
        }
        let rollback = git(base.root(), &["branch", "-D", &branch]);
        return Err(match rollback {
            Ok(_) => err,
            Err(rb) => with_context(err, &format!("回收分支 {branch} 亦失败：{rb}")),
        });
    }

    // worktree 已落盘，此处 `new_outside` 只做重叠判定与规范化。
    // 路径位于 Base 之内，重叠判定必然放行；保留该调用是为了让句柄的产出
    // 走唯一的公开入口，`TaskWorkspace` 的构造路径不因后端而分岔。
    let task = TaskWorkspace::new_outside(base, &path, intent.clone())?;

    // 排除放在**创建成功之后**。`.ai/` 这时才存在，之前没有要排除的东西；
    // 而失败路径上（`git branch` 或 `git worktree add` 任一步失败）连 `.ai/`
    // 都没有。提前做只会让「一次失败的创建留下了什么」多出一个答案——
    // `info/exclude` 里凭空多一行。被 `check_intent` 放行却被 git 拒绝的
    // intent（`@`、`a b` 等）正是这种情况。
    //
    // **次序不可调回「先排除后创建」。** 唯一的约束是排除必须在任何外部观察者
    // 可能看到 `.ai/` 之前完成。本层是单线程调用，`git` 与 `TaskWorkspace::new_outside`
    // 之间没有让出点（无 await、无线程、无回调把控制权交回调用方），故「先创建后排除」
    // 不构成窗口。将来若有人在这两步之间插入让出点，这条约束才需要重新审。
    if let Err(err) = ensure_ai_excluded(base.root()) {
        // 排除失败同样要回收：否则留下一个没人认得的 worktree 与分支，
        // 且同一 intent 的重试会因分支已存在而永久失败——与建分支那一步同理。
        let rollback = discard(base, &task);
        if !ai_dir_preexisting {
            remove_empty_ai_dirs(base.root());
        }
        return Err(match rollback {
            Ok(()) => err,
            Err(rb) => with_context(
                err,
                &format!("回收 worktree {path_arg}（分支 {branch}）亦失败：{rb}"),
            ),
        });
    }

    Ok(task)
}

/// 清掉本次尝试建出来的、现在已空的两个目录：`<base>/.ai/worktrees` 与 `<base>/.ai`。
///
/// `git worktree add` 与 `git worktree remove` 都会留下空的中间目录（前者失败前
/// 已把目录建好，后者只删叶子）。空目录对 git 不可见（`git status` 不报空目录），
/// 但它仍是磁盘上的残留，与本层「失败不留痕」的语义相悖，故一并清掉。
///
/// 只删**空**目录：`std::fs::remove_dir` 不递归，目录里还有别的东西（别的 Intent
/// 的 worktree）时必然失败，故不会误删。调用方须先确认这两个目录在本次之前不存在，
/// 否则删的是别人本来就有的东西。
fn remove_empty_ai_dirs(base: &Path) {
    let worktrees = base.join(WORKTREE_DIR);
    let _ = std::fs::remove_dir(&worktrees);
    if let Some(ai) = worktrees.parent() {
        let _ = std::fs::remove_dir(ai);
    }
}

/// 放弃一个 Task Workspace：移除 worktree 目录，再删除其分支。
///
/// 先移除 worktree 再删分支：分支被 worktree 检出时 `git branch -D` 会拒绝，
/// 反序会把「worktree 已移除但分支还在」变成常态。反过来的中间态（目录已移除、
/// 分支尚存）只在第二步失败时出现，此时返回 `Err` 而非静默。
///
/// 两步的错误都带上分支名与路径：该中间态**无法经本函数回收**——重试会在第一步
/// 就失败（工作树已不在），故调用方只能手工收拾，错误里必须给得出收拾所需的两样。
pub(crate) fn discard(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<(), WorkspaceError> {
    let path_arg = path_str(task.root())?;
    let branch = branch_name(task.intent_id());
    let root = task.root().display();
    git(base.root(), &["worktree", "remove", "--force", path_arg])
        .map_err(|e| with_context(e, &format!("移除 worktree {root}（分支 {branch}）")))?;
    git(base.root(), &["branch", "-D", &branch])
        .map_err(|e| with_context(e, &format!("删除分支 {branch}（Task 根 {root}）")))?;
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

/// 给 `GitFailed` 的 `stderr` 前面添一行本层写的上下文，其余变体原样返回。
///
/// 用在「失败之后用户仓库里可能仍有残留」的两处：`create` 的分支回收失败，
/// 与 `discard` 的任一步失败。这两处的调用方拿到的不能只是 git 的报文——
/// `discard` 的残留分支无法再经 `discard` 回收（重试会在第一步就失败），
/// 分支名与路径必须出现在错误里，人工才有办法收拾。
///
/// 只动 `stderr` 字段的文本，`code` 仍是 git 的退出码：`WorkspaceError` 的
/// 公开形状不变。追加的那行以固定前缀标出，见该变体的文档注释。
fn with_context(err: WorkspaceError, context: &str) -> WorkspaceError {
    match err {
        WorkspaceError::GitFailed { code, stderr } => WorkspaceError::GitFailed {
            code,
            stderr: format!("（continuum-workspace：{context}）\n{stderr}"),
        },
        other => other,
    }
}

fn io_error(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::IoFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}
