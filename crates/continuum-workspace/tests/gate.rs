//! Integration Gate 的只读操作在两种后端上的往返测试（§257）。
//!
//! 每个用例都在两个后端各跑一遍，后端写在用例名上：不带后缀的是 Git worktree 后端，
//! 带 `_on_the_overlay_backend` 的是 OverlayFS 后端。两个后端的 `view_diff` 是两套
//! 实现（走 git 与逐文件比较），只有真的各跑一遍才能说两边都成立。
//!
//! OverlayFS 的挂载只在其所在的用户与挂载命名空间内可见，故那几条用例要把自身重新
//! 执行进 `unshare -Urm`，由子进程完成断言；环境不具备时**显式跳过并打标记**，不静默
//! 通过（见 [`skip`]）。这一段与 `tests/backend_overlay.rs` 的写法同源——两个测试二进制
//! 是两个 crate，取不到对方的东西。
//!
//! 路径一律取 `canonicalize` 之后的值：临时目录可能落在符号链接之下，而两个后端的
//! `TaskWorkspace` 持有的根是规范化结果，两侧不规范化会让断言指向别处。

use continuum_workspace::{
    BaseWorkspace, IntentId, IntegrationGate, WorkspaceBackend, create_task_workspace,
    discard_task_workspace,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

// ===== 两种后端共用的夹具 =====

/// `dir` 内跑一条 git 命令，返回裁剪后的 stdout；失败即断言失败。
fn run_git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
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

/// `dir` 当前检出的分支名。
fn current_branch(dir: &Path) -> String {
    run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// `dir` 的全部本地分支名。
fn local_branches(dir: &Path) -> Vec<String> {
    run_git(dir, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

/// `root` 之下全部条目的相对路径与内容（按路径排序）。
///
/// 用内容**原文**而不是内容哈希：本文件的仓库只有几个字节级文件，直接比内容比哈希更强，
/// 也少一层「哈希算得对不对」的疑问。符号链接记其目标路径，目录只用于遍历。
/// 与 `docs` 的措辞一致（「路径加内容」的集合）。
fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                walk(root, &path, out);
            } else if meta.file_type().is_symlink() {
                let target = std::fs::read_link(&path).unwrap_or_default();
                out.push((rel, target.to_string_lossy().into_owned().into_bytes()));
            } else if let Ok(bytes) = std::fs::read(&path) {
                out.push((rel, bytes));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

// ===== Git worktree 后端 =====

/// 建一个真实 Git 仓库，返回（保活用的 `TempDir`，规范化的仓库根）。
///
/// 文件名用中文：`view_diff` 走的是 `git ... -z`，而 git 不加 `-z` 时会把非 ASCII
/// 路径转义成 `"\346\226\207"` 一类带引号的形式——中文名让这条路径真的被走到。
fn git_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    // 固定初始分支名，否则取 `init.defaultBranch` 配置，各机器不同。
    run_git(&root, &["init", "-b", "main"]);
    // 缺这两项 `git commit` 会直接失败，失败点会指向夹具而不是实现。
    run_git(&root, &["config", "user.email", "test@example.invalid"]);
    run_git(&root, &["config", "user.name", "Continuum 测试"]);
    std::fs::write(root.join("要改的.txt"), "原内容\n").unwrap();
    std::fs::write(root.join("要删的.txt"), "将被删除\n").unwrap();
    run_git(&root, &["add", "-A"]);
    run_git(&root, &["commit", "-m", "初始提交"]);
    (dir, root)
}

#[test]
fn view_diff_reports_the_task_changes() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    assert_eq!(backend, WorkspaceBackend::Worktree);

    // 在 Task 内改一个、删一个、加一个
    std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
    std::fs::remove_file(task.root().join("要删的.txt")).unwrap();
    std::fs::write(task.root().join("新文件.txt"), "Task 新增\n").unwrap();

    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert_eq!(
        diff.modified(),
        &[PathBuf::from("要改的.txt")][..],
        "修改类不对：{diff:?}"
    );
    assert_eq!(
        diff.deleted(),
        &[PathBuf::from("要删的.txt")][..],
        "删除类不对：{diff:?}"
    );
    assert_eq!(
        diff.added(),
        &[PathBuf::from("新文件.txt")][..],
        "新增类不对：{diff:?}"
    );
}

#[test]
fn view_diff_does_not_modify_the_base() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();

    // 让 Task 侧有真改动：这正是会让 git 刷新 stat 缓存的情形
    std::fs::write(task.root().join("要改的.txt"), "Task 改过\n").unwrap();
    std::fs::write(task.root().join("新文件.txt"), "Task 新增\n").unwrap();
    let head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

    // 快照含 `.git/`：worktree 后端的索引就在 Base 的目录树之内，而 git 在 stat 缓存
    // 过期时会刷新并写回索引——本用例要看住的正是「view_diff 没有触发这件事」。
    let before = tree_snapshot(&base_path);
    IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    let after = tree_snapshot(&base_path);

    assert_eq!(before, after, "view_diff 改动了 Base 的目录树");
    assert_eq!(
        run_git(&base_path, &["rev-parse", "HEAD"]),
        head_before,
        "Base 的 HEAD 变了"
    );
    assert_eq!(current_branch(&base_path), "main", "Base 的当前分支变了");
    assert_eq!(
        run_git(&base_path, &["status", "--porcelain"]),
        "",
        "Base 的工作树变脏了"
    );
}

/// stat 缓存过期时，`view_diff` 也不写 Base。
///
/// 这是 [`IndexCopy`] 存在的唯一理由：`git diff <分歧点>` 会把**内容没变**的条目的
/// 新 stat 写回索引，而 Task worktree 的索引在 `<base>/.git/worktrees/<intent>/` 之内。
/// 上一条用例让 Task 真的有改动，那种情形下 git 不会更新索引（条目仍是脏的，没什么
/// 可写），故碰不到这条路径——非要造出「stat 过期而内容不变」才走得到。
///
/// 对照臂是必要的：没有它，本用例在「git 将来不再刷新索引」时会**静静地**变成空话。
/// 故在金标准断言之后照原样再跑一次同样的 git 命令（这一次用真实索引），要求它
/// **确实**写回索引。
#[test]
fn view_diff_does_not_write_the_base_when_the_stat_cache_is_stale() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    let task_root = task.root().to_path_buf();

    // 造出「stat 过期、内容不变」：同样的内容重写一遍，mtime 变新。mtime 的分辨率
    // 可能是秒，先睡过一秒，免得与索引里记的同一个时刻。
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(task_root.join("要改的.txt"), "原内容\n").unwrap();

    let before = tree_snapshot(&base_path);
    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert!(diff.is_empty(), "内容没变，Diff 不该有改动：{diff:?}");
    assert_eq!(
        tree_snapshot(&base_path),
        before,
        "view_diff 在 stat 缓存过期时写回了 Base 里的索引"
    );

    // 对照臂：同一次 git diff，但用真实索引。它必须写回索引，否则本用例没造出要防的
    // 那条路径，上面那条断言也就无从谈起。
    run_git(
        &task_root,
        &["diff", "--no-renames", "--name-status", "-z", "HEAD"],
    );
    assert_ne!(
        tree_snapshot(&base_path),
        before,
        "对照臂（用真实索引的 git diff）没有写回索引：本用例没有造出 IndexCopy 要防的那条路径"
    );
}

/// Diff 的基准是**分歧点**，不是 Base 的当前 HEAD。
///
/// 用户在建好工作区之后又提交时，那些提交里的新文件在 Task 侧并不存在。以 Base 的
/// 当前 HEAD 为基准会把它们反向算进本 Task（看起来像 Task 删了它们）——本用例钉住
/// 这一条。两个基准在 Base 没有前进时结果相同，故非有本用例不可。
#[test]
fn view_diff_is_measured_from_the_point_the_task_branched_off() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();

    // 用户自己的提交：与 Task 无关，不该出现在 Task 的 Diff 里
    std::fs::write(base_path.join("用户后来又加的.txt"), "用户的东西\n").unwrap();
    run_git(&base_path, &["add", "用户后来又加的.txt"]);
    run_git(&base_path, &["commit", "-m", "用户自己的提交"]);

    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert!(
        diff.is_empty(),
        "Base 在创建之后新进的提交被算进了本 Task 的改动：{diff:?}"
    );
}

/// 重命名经 `--no-renames` 拆成一「删除」一「新增」。
///
/// 不拆的话 git 给的是 `R100\0旧名\0新名\0`——一个状态带**两个**路径，而三类的模型
/// 是「一条记录一个路径」，本层的解析只认一对一（见 `gate.rs` 里那条 `R100` 的用例）。
/// overlay 后端本来就没有重命名这个概念，拆开也是两端唯一对得上的说法。
#[test]
fn a_rename_is_reported_as_a_deletion_and_an_addition() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    run_git(task.root(), &["mv", "要改的.txt", "改了名的.txt"]);

    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert_eq!(
        diff.deleted(),
        &[PathBuf::from("要改的.txt")][..],
        "重命名的旧名不在删除类里：{diff:?}"
    );
    assert_eq!(
        diff.added(),
        &[PathBuf::from("改了名的.txt")][..],
        "重命名的新名不在新增类里：{diff:?}"
    );
    assert!(diff.modified().is_empty(), "重命名被算成了修改：{diff:?}");
}

/// 普通文件改成符号链接：git 给的是类型变更（`T`），本层归入「修改」。
///
/// 三类里没有「类型」这一类，而它确实是一处要被带进 Base 的改动——丢了它就等于
/// 漏报。本用例同时钉住真实 git 在这种改动上给的那个状态确实是本层认识的那一个。
#[test]
fn a_file_replaced_by_a_symlink_is_reported_as_modified() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    std::fs::remove_file(task.root().join("要改的.txt")).unwrap();
    std::os::unix::fs::symlink("要删的.txt", task.root().join("要改的.txt")).unwrap();

    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert_eq!(
        diff.modified(),
        &[PathBuf::from("要改的.txt")][..],
        "类型变更被漏掉或归错了类：{diff:?}"
    );
    assert!(diff.deleted().is_empty(), "{diff:?}");
}

/// 被 `.gitignore` 忽略的文件不计入改动（`--exclude-standard`）。
///
/// 用户明确不跟踪的东西，本层也不当作本 Task 的改动：三种写入操作都经 git 落到
/// Base 上，而 git 本来就不会带上被忽略的文件——列出来只会让 Diff 与实际会发生的
/// 集成对不上。
#[test]
fn ignored_files_are_not_reported_as_added() {
    let (_d, base_path) = git_repo();
    std::fs::write(base_path.join(".gitignore"), "被忽略的/\n").unwrap();
    run_git(&base_path, &["add", ".gitignore"]);
    run_git(&base_path, &["commit", "-m", "加一条忽略规则"]);

    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    std::fs::create_dir_all(task.root().join("被忽略的")).unwrap();
    std::fs::write(task.root().join("被忽略的/东西.txt"), "忽略我\n").unwrap();
    std::fs::write(task.root().join("没被忽略.txt"), "看着我\n").unwrap();

    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert_eq!(
        diff.added(),
        &[PathBuf::from("没被忽略.txt")][..],
        "被忽略的文件混进了新增类：{diff:?}"
    );
}

#[test]
fn discard_removes_the_task_and_leaves_the_base_unchanged() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    let task_root = task.root().to_path_buf();

    // Task 内的改动，并且提交——这正是 Agent 在 worktree 里的工作方式
    std::fs::write(task_root.join("Task 的东西.txt"), "Task 侧内容\n").unwrap();
    run_git(&task_root, &["add", "-A"]);
    run_git(&task_root, &["commit", "-m", "Task 内提交"]);
    let head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

    IntegrationGate::new(&base).discard(&task, backend).unwrap();

    assert!(!task_root.exists(), "Task 根仍在：{}", task_root.display());
    assert!(
        !base_path.join("Task 的东西.txt").exists(),
        "Task 的文件出现在 Base 中"
    );
    assert!(
        !local_branches(&base_path).contains(&"ai/i1".to_owned()),
        "分支 ai/i1 仍在：{:?}",
        local_branches(&base_path)
    );
    // 用户那一侧：HEAD、当前分支、工作树都没有被动过
    assert_eq!(run_git(&base_path, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(current_branch(&base_path), "main");
    assert_eq!(
        run_git(&base_path, &["status", "--porcelain"]),
        "",
        "Base 的工作树变脏了"
    );
    assert_eq!(
        std::fs::read_to_string(base_path.join("要改的.txt")).unwrap(),
        "原内容\n"
    );
}

// ===== OverlayFS 后端（需要用户与挂载命名空间）=====

/// 标识「本进程是被重新执行出来的子进程」，防止再次 re-exec 造成无限递归。
const CHILD_ENV: &str = "CONTINUUM_GATE_CHILD";

/// overlay 根：由父进程指到一个只属于本次用例的临时目录，子进程据此建立工作区。
const OVERLAY_ROOT_ENV: &str = "CONTINUUM_OVERLAY_ROOT";

fn is_child() -> bool {
    std::env::var_os(CHILD_ENV).is_some()
}

/// 环境不具备本用例所需条件时的**显式**跳过标记。
///
/// 不静默通过：跳过原因打到 stderr。cargo 在用例通过时不回显这段输出，用 `--nocapture`
/// 运行即可见——这是稳定工具链上既不与「全量全绿」冲突、又留下痕迹的处置。改判为
/// `panic!` 会把「环境不具备」误报成「实现有缺陷」，反而掩盖真正的问题。
fn skip(test: &str, reason: &str) {
    eprintln!("【跳过】{test}：{reason}。本用例未执行断言。");
}

/// 环境是否具备建立用户与挂载命名空间的能力。
///
/// 探测方式是把**本测试二进制**以 `--list` 重新执行进 `unshare -Urm`：能起来即说明
/// 命名空间可建立。用自身二进制而非 `/bin/true` 一类的替身，是因为要探的正是
/// 「本测试进程能否被这样执行」——替身探不出动态加载等本进程的实际依赖。
fn namespace_available(exe: &str) -> bool {
    Command::new("unshare")
        .args(["-Urm", exe, "--list"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// 把本用例重新执行进 `unshare -Urm` 的子进程，并核对子进程的退出码。
///
/// 返回 `true` 表示「当前就是子进程，继续执行断言」；`false` 表示「父进程已跑完子进程
/// 并核对过结果，或环境不具备已显式跳过」。
fn enter_namespace(test_name: &str) -> bool {
    if is_child() {
        return true;
    }
    let exe = std::env::current_exe().expect("无法取到测试可执行文件的路径");
    let exe = exe
        .to_str()
        .expect("测试可执行文件的路径不是合法 UTF-8")
        .to_owned();
    if !namespace_available(&exe) {
        skip(test_name, "unshare -Urm 无法建立用户与挂载命名空间");
        return false;
    }
    // overlay 根指到一个只属于本次用例的临时目录——绝不能落到默认的
    // `~/.local/share/continuum/overlays`，否则用例会污染用户的 home。
    let holder = tempfile::tempdir().unwrap();
    let root: OsString = holder.path().join("overlays").into_os_string();
    let out = Command::new("unshare")
        .args(["-Urm", &exe, test_name, "--exact"])
        .env(CHILD_ENV, "1")
        .env(OVERLAY_ROOT_ENV, &root)
        .output()
        .unwrap_or_else(|e| panic!("无法重新执行 {exe}：{e}"));
    assert!(
        out.status.success(),
        "子进程 {test_name} 失败（退出码 {:?}）：\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // 子进程已退出，本次用例的 overlay 存储随临时目录一并清掉
    drop(holder);
    false
}

/// 建一个带 lower 内容的 Base，返回（保活用的 `TempDir`，规范化的 Base 根）。
///
/// 只在子进程里调用。
fn base_with_lower() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("要改的.txt"), "lower").unwrap();
    std::fs::write(root.join("要删的.txt"), "只在 lower\n").unwrap();
    (dir, root)
}

#[test]
fn view_diff_reports_the_task_changes_on_the_overlay_backend() {
    if !enter_namespace("view_diff_reports_the_task_changes_on_the_overlay_backend") {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    assert_eq!(backend, WorkspaceBackend::Overlay);

    // 在 Task 内改一个、删一个、加一个。删除在覆盖层里落成一个白障（upper 层里
    // 一个 0/0 的字符设备），合并视图上就看不见了——本用例顺带钉住这条。
    task.writable_root().write("要改的.txt", b"upper").unwrap();
    std::fs::remove_file(task.root().join("要删的.txt")).unwrap();
    task.writable_root()
        .write("新文件.txt", "Task 新增\n".as_bytes())
        .unwrap();

    let diff = IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert_eq!(
        diff.modified(),
        &[PathBuf::from("要改的.txt")][..],
        "修改类不对：{diff:?}"
    );
    assert_eq!(
        diff.deleted(),
        &[PathBuf::from("要删的.txt")][..],
        "删除类不对：{diff:?}"
    );
    assert_eq!(
        diff.added(),
        &[PathBuf::from("新文件.txt")][..],
        "新增类不对：{diff:?}"
    );
}

#[test]
fn view_diff_does_not_modify_the_base_on_the_overlay_backend() {
    if !enter_namespace("view_diff_does_not_modify_the_base_on_the_overlay_backend") {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    task.writable_root().write("要改的.txt", b"upper").unwrap();
    std::fs::remove_file(task.root().join("要删的.txt")).unwrap();
    task.writable_root()
        .write("新文件.txt", "Task 新增\n".as_bytes())
        .unwrap();

    let before = tree_snapshot(&base_path);
    IntegrationGate::new(&base)
        .view_diff(&task, backend)
        .unwrap();
    assert_eq!(
        tree_snapshot(&base_path),
        before,
        "view_diff 改动了 Base 的目录树"
    );
}

#[test]
fn discard_removes_the_task_and_leaves_the_base_unchanged_on_the_overlay_backend() {
    if !enter_namespace("discard_removes_the_task_and_leaves_the_base_unchanged_on_the_overlay_backend")
    {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    let task_root = task.root().to_path_buf();
    task.writable_root().write("要改的.txt", b"upper").unwrap();
    task.writable_root()
        .write("Task 的东西.txt", "Task 侧内容\n".as_bytes())
        .unwrap();

    IntegrationGate::new(&base).discard(&task, backend).unwrap();

    assert!(!task_root.exists(), "Task 根仍在：{}", task_root.display());
    assert!(
        !base_path.join("Task 的东西.txt").exists(),
        "Task 的文件出现在 Base 中"
    );
    // Base 一字未动：Task 侧改过的文件仍是 lower 的内容
    assert_eq!(std::fs::read(base_path.join("要改的.txt")).unwrap(), b"lower");
    assert_eq!(
        std::fs::read_to_string(base_path.join("要删的.txt")).unwrap(),
        "只在 lower\n"
    );
    // 两个后端的 discard 是同一个实现，这里再经 Gate 走一遍，顺带钉住 Gate 的封装
    // 没有绕开 overlay 的「先 umount 再删目录」。
    let (again, _) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    discard_task_workspace(&base, &again, backend).unwrap();
}

/// Gate 的 `discard` 与 [`discard_task_workspace`] 是同一条实现：后者对「不是本后端
/// 布局的根」的拒绝必须原样透出 Gate 这一层，不被折成一个笼统的错误。
#[test]
fn discarding_a_foreign_root_through_the_gate_reports_the_backend_error() {
    let holder = tempfile::tempdir().unwrap();
    let base_path = holder.path().join("base");
    std::fs::create_dir(&base_path).unwrap();
    let task_root = holder.path().join("task");
    std::fs::create_dir_all(&task_root).unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    let task = continuum_workspace::TaskWorkspace::new_outside(
        &base,
        &task_root,
        IntentId::new("i1"),
    )
    .unwrap();

    // overlay 后端：形态校验排在 umount 之前，故不需要命名空间也走得到（与
    // `tests/backend_overlay.rs` 里那条同源）。
    let err = IntegrationGate::new(&base)
        .discard(&task, WorkspaceBackend::Overlay)
        .unwrap_err();
    match &err {
        continuum_workspace::GateError::Workspace(
            continuum_workspace::WorkspaceError::BackendUnavailable { reason },
        ) => assert!(
            reason.contains("overlay 布局"),
            "错误未说明这不是本后端的布局：{reason}"
        ),
        other => panic!("期望透出后端的 BackendUnavailable，得到 {other:?}"),
    }
    assert!(task_root.exists(), "误用的路径被删了");
}
