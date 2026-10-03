//! Git worktree 后端（§255）。
//!
//! Base 是 Git 仓库时，Task Workspace 是建在 `<base>/.ai/worktrees/<intent>` 的
//! worktree，分支名 `ai/<intent>`（§16）。创建、放弃都经 `git` 命令完成。
//!
//! **Base 中被跟踪的内容零改动。** 写进 Base 目录树的只有两处，都属仓库或 Runtime
//! 自身：`.git/` 里的账本（仓库本地的 `info/exclude` 追加一行 `.ai/`，见
//! [`ensure_ai_excluded`]；新建分支 `ai/<intent>`；worktree 登记
//! `.git/worktrees/<intent>/**`），以及 `.ai/` 之下本次建出的 worktree 目录本身。
//!
//! 用户的当前分支全程不被触碰：`git worktree add` 只写新建分支与主仓库的
//! worktree 登记（`.git/worktrees/`），`git worktree remove` 与 `git branch -D`
//! 各自只回收自己那一份。故 Base 的 HEAD 与工作树内容在创建/放弃前后逐字节不变。

use crate::backend::check_intent;
use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::ids::IntentId;
use crate::task::{AI_DIR, TaskWorkspace};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 各 Intent 的 worktree 所在的子目录名，位于 [`AI_DIR`] 之下（§255）。
const WORKTREE_SUBDIR: &str = "worktrees";

/// 仓库本地排除文件中标记 AI 私有目录的条目是否为 `line`。
///
/// **`.ai` 这个名字只有 [`AI_DIR`] 一处写法**：本条与追加时写的那一行都由它派生，
/// 改 `AI_DIR` 不会漏掉这里（`IGNORE_ENTRY = ".ai/"` 曾是与它并存的第二个字面量）。
///
/// 两种拼法都认（`.ai` 与 `.ai/`）：gitignore 对二者等价，用户可能自己写过任一种，
/// 认出来就不必重复追加。故判定是「去掉一个尾斜杠之后等于 [`AI_DIR`]」。
fn is_ignore_entry(line: &str) -> bool {
    let line = line.trim();
    line.strip_suffix('/').unwrap_or(line) == AI_DIR
}

/// worktree 分支名的前缀（§16）。
const BRANCH_PREFIX: &str = "ai/";

/// `<base>/.ai`。
///
/// 与 [`worktrees_dir`] 分开取而非从后者 `parent()`：失败回收时两者的
/// 「本次之前是否存在」是两个独立的事实，须分别判定（Base 里可能本来就有
/// 用户预建的空 `.ai/`，而 `.ai/worktrees` 是本次才建出来的）。
fn ai_dir(base: &Path) -> PathBuf {
    base.join(AI_DIR)
}

/// `<base>/.ai/worktrees`。
fn worktrees_dir(base: &Path) -> PathBuf {
    ai_dir(base).join(WORKTREE_SUBDIR)
}

/// `<base>/.ai/worktrees/<intent>`。
pub(crate) fn worktree_path(base: &BaseWorkspace, intent: &IntentId) -> PathBuf {
    worktrees_dir(base.root()).join(intent.as_str())
}

/// `ai/<intent>`。分支名与 worktree 路径同源于 `intent`，故两者要么一起合法，
/// 要么在 [`crate::backend::check_intent`] 处一起被拒。
pub(crate) fn branch_name(intent: &IntentId) -> String {
    format!("{BRANCH_PREFIX}{intent}")
}

/// 创建一个 Task Workspace。
///
/// 次序：校验 intent → 建分支 → 检出 worktree → 由
/// [`TaskWorkspace::new_outside`] 产出句柄 → 确保 `.ai/` 被排除。
///
/// **排除排在最后**（见文件末尾那段注释）：`.ai/` 这时才存在，之前没有要排除的
/// 东西；创建失败时 `info/exclude` 因此一字不动。
///
/// **建分支与检出 worktree 分作两步，不写成单条 `git worktree add -b`。**
/// 那条命令实际是「先建分支 ref，再建 worktree」两个动作，第二阶段失败时
/// 分支会留在用户仓库里：实测目标目录已存在且非空时报 `already exists`（退出码
/// 128），intent 为 `@` 时报 `could not find created worktree '@'`（退出码 255），
/// 两种情形都留下一条 `ai/<intent>` ref。留下的 ref 有两个后果：它是失败路径上
/// 对用户仓库元数据的写入，且会让**同一 intent 永远无法再创建**（重试报
/// `a branch named … already exists`）。拆开之后，第一阶段失败时仓库里什么都没
/// 多出来——`git branch` 是单个动作。
///
/// 拆开还挡住另一种误伤：单条命令下，**已存在的同名分支**会让第二阶段失败，
/// 而回收会顺手 `git branch -D` 把那条**不是本次创建**的分支删掉。真实代码里
/// 第一步 `git branch` 已在该情形失败并裸 `?` 返回，根本走不到回收。
///
/// **每一个可能留下东西的步骤失败时都走同一个 [`Reclaim`]**，故「任何一步失败
/// 都不留工作树与分支的痕迹」由结构保证，而不是靠逐个论证「那一步不可达」。
/// 第一步（`git branch`）是例外：它是单个动作，失败时仓库里什么都没多出来，
/// 没有可回收的东西。
///
/// 失败路径上唯一不由本函数清掉的是 `git worktree add` 可能写了一半的
/// `.git/worktrees/<name>` 登记条目——它没有对应目录，只有 `git worktree prune`
/// 够得着，由 [`Reclaim::run`] 负责。不预建目标目录：`git worktree add` 在目录
/// 不存在时自建全部中间目录。
pub(crate) fn create(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<TaskWorkspace, WorkspaceError> {
    check_intent(intent)?;
    let path = worktree_path(base, intent);
    let path_arg = path_str(&path)?;
    let branch = branch_name(intent);
    let reclaim = Reclaim {
        path: &path,
        branch: &branch,
        // `git branch` 之后、`git worktree add` 之前：worktree 尚未登记
        worktree_registered: false,
        // 两个目录在本次之前是否存在。只清本次建出来的那一份——用户预建的
        // 空 `.ai/` 不是我们的东西，别的 Intent 的 worktree 也可能住在里面。
        worktrees_preexisting: worktrees_dir(base.root()).exists(),
        ai_preexisting: ai_dir(base.root()).exists(),
    };

    // 第一步是单个动作，失败即干净返回，仓库里什么都没多出来
    git(base.root(), &["branch", &branch])?;

    // 第二步：不带 `-b`，分支已经建好，这里只把工作树检出到它上面。
    if let Err(err) = git(base.root(), &["worktree", "add", path_arg, &branch]) {
        // 这一步失败时目录可能已被 git 建出（intent 为 `@` 时实测留下空的
        // `.ai/worktrees/`），但 worktree 未登记，故回收不必移除 worktree
        return Err(with_reclaim(err, reclaim.run(base.root())));
    }

    // 从这里起 worktree 已登记，回收时要连它一起移除
    let reclaim = Reclaim {
        worktree_registered: true,
        ..reclaim
    };

    // worktree 已落盘，此处 `new_outside` 只做重叠判定与规范化。
    // 路径位于 Base 之内，重叠判定必然放行；保留该调用是为了让句柄的产出
    // 走唯一的公开入口，`TaskWorkspace` 的构造路径不因后端而分岔。
    let task = TaskWorkspace::new_outside(base, &path, intent.clone())
        .map_err(|e| with_reclaim(e, reclaim.run(base.root())))?;

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
    // 排除失败同样要回收：否则留下一个没人认得的 worktree 与分支，
    // 且同一 intent 的重试会因分支已存在而永久失败——与建分支那一步同理。
    ensure_ai_excluded(base.root())
        .map_err(|e| with_reclaim(e, reclaim.run(base.root())))?;

    Ok(task)
}

/// 一次失败的创建之后要回收的东西。
///
/// 抽成类型而非一串参数：其中三个是布尔事实，位置传参极易传错，而传错的后果
/// 是删掉不属于本次尝试的目录、或漏删本该删的。按**路径与分支名**而非
/// `TaskWorkspace` 回收，是因为 [`TaskWorkspace::new_outside`] 那一步失败时句柄
/// 尚未产出——而那一步同样要回收。
struct Reclaim<'a> {
    /// worktree 的目标路径。
    path: &'a Path,
    /// 该 Intent 的分支名。
    branch: &'a str,
    /// `git worktree add` 是否已成功登记过这个 worktree。
    /// 未登记时不能也不必执行 `git worktree remove`（那会以「不是工作树」失败）。
    worktree_registered: bool,
    /// `.ai/worktrees` 在本次尝试之前是否已存在（存在即不是本次建出的，不能删）。
    worktrees_preexisting: bool,
    /// `.ai` 在本次尝试之前是否已存在。
    ai_preexisting: bool,
}

impl Reclaim<'_> {
    /// 尽力回收，返回**第一个**失败。
    ///
    /// 三件事（移除 worktree、删分支、清空目录）互相独立，一件失败不跳过其余：
    /// 目的正是把残留清到最少，中途放弃只会留下更多。第一个失败会带上路径与
    /// 分支名返回，调用方据此判断仓库里还剩什么。
    fn run(&self, base: &Path) -> Result<(), WorkspaceError> {
        let mut failure: Option<WorkspaceError> = None;

        if self.worktree_registered {
            let path = self.path.to_string_lossy().into_owned();
            if let Err(e) = git(base, &["worktree", "remove", "--force", &path]) {
                failure = Some(with_context(
                    e,
                    &format!("回收 worktree {path}（分支 {}）", self.branch),
                ));
            }
        }

        if let Err(e) = git(base, &["branch", "-D", self.branch]) {
            failure.get_or_insert(with_context(
                e,
                &format!("回收分支 {}（Task 根 {}）", self.branch, self.path.display()),
            ));
        }

        remove_empty_ai_dirs(base, self.worktrees_preexisting, self.ai_preexisting);

        // `git worktree add` 未登记成功也能留下东西：它在失败前已往
        // `.git/worktrees/` 写过 admin 目录（intent 为 `@` 时实测留下
        // `.git/worktrees/-`）。这类条目没有对应的目录，`git worktree remove`
        // 够不着——它以路径为参数，而那个路径并不存在——只有 `prune` 清得掉。
        //
        // 只在 worktree 未登记时做：那时才可能有写了一半的条目；已登记的情形
        // 由上面的 `worktree remove` 收尾，无事可 prune。
        //
        // `prune` 只删「登记还在、目录已不存在」的条目，故别的 Intent 的有效
        // worktree 不受影响——它们的目录真实存在。代价是它也可能顺手清掉用户
        // 自己遗留的同类僵尸条目；那正是 git 对这类无目录登记的定义与处置，
        // 不涉及任何存活的工作树，故不触碰有效数据。
        if !self.worktree_registered
            && let Err(e) = git(base, &["worktree", "prune"])
        {
            failure.get_or_insert(with_context(e, "清理 worktree 登记（git worktree prune）"));
        }

        failure.map_or(Ok(()), Err)
    }
}

/// 清掉本次尝试建出来的、现在已空的两个目录：`<base>/.ai/worktrees` 与 `<base>/.ai`。
///
/// `git worktree add` 与 `git worktree remove` 都会留下空的中间目录（前者失败前
/// 已把目录建好，后者只删叶子）。空目录对 git 不可见（`git status` 不报空目录），
/// 但它仍是磁盘上的残留，与本层「失败不留痕」的语义相悖，故一并清掉。
///
/// 只删**空**目录：`std::fs::remove_dir` 不递归，目录里还有别的东西（别的 Intent
/// 的 worktree）时必然失败，故不会误删。
///
/// `preexisting` 为真表示该目录在本次尝试之前就存在，即**不是本次建出来的**，
/// 一律不碰——用户可以在 Base 里预建一个空的 `.ai/`，那不是我们的东西。
/// 两个标志分开传的理由正在于此：`.ai/` 可能是用户预建的，而 `.ai/worktrees`
/// 是本次才建出来的，两者的去留不同。次序不能颠倒：先删子目录，父目录才可能空。
fn remove_empty_ai_dirs(base: &Path, worktrees_preexisting: bool, ai_preexisting: bool) {
    if !worktrees_preexisting {
        let _ = std::fs::remove_dir(worktrees_dir(base));
    }
    if !ai_preexisting {
        let _ = std::fs::remove_dir(ai_dir(base));
    }
}

/// 把「创建为什么失败」与「回收的结果」合成最终错误。
///
/// 回收成功时主错误就是全部事实；回收失败时两个错误都要给调用方——主错误说明
/// 创建为什么失败，回收错误说明仓库里可能留下了什么（分支名与路径）。
fn with_reclaim(err: WorkspaceError, reclaim: Result<(), WorkspaceError>) -> WorkspaceError {
    match reclaim {
        Ok(()) => err,
        Err(rb) => with_context(err, &format!("回收本次创建亦失败：{rb}")),
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
        if text.lines().any(is_ignore_entry) {
            return Ok(());
        }
    }
    let mut out = existing.unwrap_or_default();
    if !out.is_empty() && !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    // 追加 `AI_DIR` + `/`：gitignore 里带尾斜杠表示「目录」，不带会连同名的普通文件
    // 一并忽略——本层要排除的是那个目录。
    out.extend_from_slice(AI_DIR.as_bytes());
    out.push(b'/');
    out.push(b'\n');
    std::fs::write(&path, out).map_err(|e| io_error(&path, e))
}

/// 在 `dir` 内执行一条 git 命令，返回裁剪后的 stdout；非零退出即 `GitFailed`。
///
/// 以退出码判定成败，不看 stderr：`git worktree add` 成功时也会往 stderr
/// 打印 "Preparing worktree ..."。失败时把 code 与 stderr 原样带回，
/// 截断或改写都会让调用方丢掉 git 给出的具体原因。
///
/// 需要原始字节或需要给 git 传环境变量时用 [`git_raw`]：本函数经
/// `from_utf8_lossy` 之后非 UTF-8 的字节已被替换字符顶掉（`-z` 输出里的路径会因此
/// 失真），而 `trim` 是给「本来就是一行的文本」用的标量输出（修订号、路径）准备的。
pub(crate) fn git(dir: &Path, args: &[&str]) -> Result<String, WorkspaceError> {
    let raw = git_raw(dir, args, &[])?;
    Ok(String::from_utf8_lossy(&raw).trim().to_owned())
}

/// 在 `dir` 内执行一条 git 命令，返回**原始** stdout 字节；非零退出即 `GitFailed`。
///
/// `envs` 是额外传给 git 的环境变量，用于 `GIT_INDEX_FILE` 这类「让 git 把某个文件
/// 写到别处」的开关（见 [`crate::gate`]）。其余环境原样继承。
///
/// 失败判定与错误内容与 [`git`] 同：只看退出码，失败时把 code 与 stderr 原样带回。
pub(crate) fn git_raw(
    dir: &Path,
    args: &[&str],
    envs: &[(&str, &std::ffi::OsStr)],
) -> Result<Vec<u8>, WorkspaceError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .envs(envs.iter().map(|(key, value)| (*key, *value)))
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
    Ok(output.stdout)
}

/// 在 `dir` 内执行一条 git 命令，把 `input` 从标准输入送入，返回原始 stdout。
///
/// 失败判定与错误内容与 [`git`] 同：只看退出码，失败时把 code 与 stderr 原样带回。
///
/// **用途是 `git apply`**（见 [`crate::gate`] 的 `apply_patch`）：补丁也可以经
/// 命令行参数传，但那要先把用户代码写到磁盘上再让 git 去读（临时文件），
/// stdin 不必落盘。
///
/// **输入在另一个线程里写。** 本进程若先把整份输入写完再收结果，输入大于管道缓冲区
/// 且 git 同时往 stderr 写（例如补丁有问题时逐条报错）就会互相等待：本进程等 git 读，
/// git 等本进程读它已经写满的 stderr。子线程写、主线程 `wait_with_output` 收，
/// 两边的缓冲都在被消费，故不会互等。
///
/// 退出码为 0 时若输入**没写完**，返回 `IoFailed` 而不当作成功：那种情形下 git 读到
/// 的是残缺输入（例如它在开头就判定无需处理而提前退出），本层无从判断它是否用上了
/// 整份输入。
pub(crate) fn git_with_stdin(
    dir: &Path,
    args: &[&str],
    input: &[u8],
) -> Result<Vec<u8>, WorkspaceError> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| WorkspaceError::BackendUnavailable {
            reason: format!("无法执行 git：{e}"),
        })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| WorkspaceError::BackendUnavailable {
            reason: "git 的标准输入未按请求建立".to_owned(),
        })?;
    // `scope` 而非 `thread::spawn`：要借 `input`（它是调用方的切片），不需要把它
    // 复制成拥有所有权的数据。
    let (output, written) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            let result = stdin.write_all(input);
            // 关掉管道让 git 读到 EOF：不关的话 git 会一直等输入，双方都停在这里。
            drop(stdin);
            result
        });
        let output = child.wait_with_output().map_err(|e| {
            WorkspaceError::BackendUnavailable {
                reason: format!("无法执行 git：{e}"),
            }
        });
        (output, writer.join())
    });
    let output = output?;
    // 子线程只做一次 `write_all`，没有可 panic 之处；真出了 panic 也只说明输入没送完，
    // 那是下面那条 `IoFailed` 要报的情形，故把 `join` 的失败折进同一个答复。
    let written = written
        .unwrap_or_else(|_| Err(std::io::Error::other("向 git 写入标准输入的子线程 panic")));
    if !output.status.success() {
        return Err(WorkspaceError::GitFailed {
            // 被信号杀死时没有退出码；取 -1 仅作占位。
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim_end().to_owned(),
        });
    }
    written.map_err(|e| WorkspaceError::IoFailed {
        path: dir.to_path_buf(),
        reason: format!("向 git 送出输入时失败：{e}"),
    })?;
    Ok(output.stdout)
}

/// 给错误附加一层上下文（哪一步、哪个分支、哪个路径）。
///
/// 用在「失败之后用户仓库里可能仍有残留」的路径：`create` 的回收失败，
/// 与 `discard` 的任一步失败。这些路径的调用方拿到的不能只是 git 的报文——
/// `discard` 的残留分支无法再经 `discard` 回收（重试会在第一步就失败），
/// 分支名与路径必须出现在错误里，人工才有办法收拾。
///
/// **对所有错误种类都生效**，不只是 `GitFailed`：`create` 那一步的主错误是
/// 排除失败，实际产生的是 `IoFailed`。附加方式按变体各取其能承载文本之处：
///
/// - `GitFailed` 追加到 `stderr`（它是 git 报文的自由文本字段），`code` 不动，
///   故按该变体分派、按退出码判断的既有调用方不受影响；
/// - 其余变体没有这样的自由文本字段（`IoFailed` 的 `reason` 是 io 错误本身，
///   `Overlaps`/`NotInNamespace` 之类连文本字段都没有），故包一层
///   [`WorkspaceError::Context`]。
fn with_context(err: WorkspaceError, context: &str) -> WorkspaceError {
    match err {
        WorkspaceError::GitFailed { code, stderr } => WorkspaceError::GitFailed {
            code,
            stderr: format!("（continuum-workspace：{context}）\n{stderr}"),
        },
        other => WorkspaceError::Context {
            context: context.to_owned(),
            source: Box::new(other),
        },
    }
}

fn io_error(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::IoFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `with_context` 必须对**非** `GitFailed` 的错误同样附加。
    ///
    /// 集成用例覆盖不到这条：唯一会产生非 `GitFailed` 主错误的路径是
    /// 「排除失败且回收也失败」，而它在公开 API 下构造不出来——`git worktree add`
    /// 刚成功时，紧随其后的 `remove --force` 与 `branch -D` 都会成功。故用单测
    /// 直接钉住。这条一旦失效，调用方拿到的错误里就没有分支名与路径，无从判断
    /// 仓库里是否留下了残留物，而 `create` 与 `Reclaim::run` 的注释承诺了这一点。
    #[test]
    fn with_context_carries_context_for_non_git_failures() {
        let err = WorkspaceError::IoFailed {
            path: PathBuf::from("/base/.git/info/exclude"),
            reason: "Is a directory (os error 21)".to_owned(),
        };
        let out = with_context(err, "回收 worktree /base/.ai/worktrees/i1（分支 ai/i1）");
        let text = out.to_string();
        assert!(
            text.contains("/base/.ai/worktrees/i1"),
            "上下文里的路径丢失：{text}"
        );
        assert!(text.contains("ai/i1"), "上下文里的分支名丢失：{text}");
        assert!(
            text.contains("Is a directory"),
            "原始错误丢失：{text}"
        );
    }

    /// `GitFailed` 走的是另一条附加方式（追加到 `stderr`），同样要带上上下文。
    #[test]
    fn with_context_carries_context_for_git_failures() {
        let err = WorkspaceError::GitFailed {
            code: 1,
            stderr: "error: boom".to_owned(),
        };
        let out = with_context(err, "回收 worktree /base/.ai/worktrees/i1（分支 ai/i1）");
        let text = out.to_string();
        assert!(text.contains("/base/.ai/worktrees/i1"), "路径丢失：{text}");
        assert!(text.contains("ai/i1"), "分支名丢失：{text}");
        assert!(text.contains("error: boom"), "git 报文丢失：{text}");
        assert!(
            matches!(out, WorkspaceError::GitFailed { code: 1, .. }),
            "GitFailed 的变体与退出码不应被改动"
        );
    }

    /// 两个私有目录的父子关系由构造函数保证，不靠常量字面量对齐。
    #[test]
    fn the_worktrees_directory_sits_under_the_ai_directory() {
        let base = Path::new("/tmp/base");
        assert_eq!(worktrees_dir(base), ai_dir(base).join(WORKTREE_SUBDIR));
        assert_eq!(ai_dir(base), Path::new("/tmp/base/.ai"));
    }
}
