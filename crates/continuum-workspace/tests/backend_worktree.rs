//! Git worktree 后端的往返测试。
//!
//! 每个用例都在真实 Git 仓库上跑：「用户的当前分支未被改动」「worktree 目录已移除」
//! 这类事实由 `git` 的行为决定，替身测不出来。
//!
//! 仓库与 worktree 的路径一律取 `canonicalize` 之后的值。临时目录可能落在符号链接
//! 之下，而 `TaskWorkspace` 持有的根是规范化结果——两侧不规范化，`starts_with`
//! 的比较必然为假，失败点会指向断言而不是实现。

use continuum_workspace::{
    BaseWorkspace, IntentId, WorkspaceBackend, create_task_workspace, detect_backend,
    discard_task_workspace,
};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 建一个真实 Git 仓库，返回（保活用的 `TempDir`，规范化的仓库根）。
fn git_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    // 固定初始分支名，否则取 `init.defaultBranch` 配置，各机器不同。
    run_git(&root, &["init", "-b", "main"]);
    // 缺这两项 `git commit` 会直接失败，失败点会指向夹具而不是实现。
    run_git(&root, &["config", "user.email", "test@example.invalid"]);
    run_git(&root, &["config", "user.name", "Continuum 测试"]);
    // 预置一份非空 `.gitignore`：它是用户的跟踪文件，用例要验证它**不被动**，
    // 空文件或缺失文件都测不出「被改写」。
    std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
    std::fs::write(root.join("README.md"), "初始内容\n").unwrap();
    run_git(&root, &["add", "-A"]);
    run_git(&root, &["commit", "-m", "初始提交"]);
    (dir, root)
}

/// 在 `dir` 内跑一条 git 命令，返回裁剪后的 stdout；失败即断言失败。
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

/// `dir` 下的一级条目名（排序后），用于断言「没有多出任何东西」。
fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `dir` 的全部本地分支名。
fn local_branches(dir: &Path) -> Vec<String> {
    run_git(dir, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn worktree_backend_creates_an_isolated_branch() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    assert_eq!(detect_backend(&base), WorkspaceBackend::Worktree);

    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    assert_eq!(backend, WorkspaceBackend::Worktree);

    // §16：每个 Intent 一个独立分支，名为 ai/<intent-id>
    assert_eq!(current_branch(task.root()), "ai/i1");
    // §255：worktree 位于 <base>/.ai/worktrees/ 之下
    let worktrees = base_path.join(".ai/worktrees");
    assert!(
        task.root().starts_with(&worktrees),
        "Task 根 {} 不在 {} 之下",
        task.root().display(),
        worktrees.display()
    );
    // 用户的当前分支未被改动
    assert_eq!(current_branch(&base_path), "main");
    // 分支真实存在，且不是用户当前所在的那个
    let branches = local_branches(&base_path);
    assert!(branches.contains(&"ai/i1".to_owned()), "分支缺失：{branches:?}");
    assert!(branches.contains(&"main".to_owned()), "分支缺失：{branches:?}");
    // worktree 是一次真实检出：Base 已提交的内容在其中可见
    assert_eq!(
        std::fs::read_to_string(task.root().join("README.md")).unwrap(),
        "初始内容\n"
    );
}

#[test]
fn discarding_a_worktree_leaves_the_base_untouched() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i2")).unwrap();
    let base_head_before = run_git(&base_path, &["rev-parse", "HEAD"]);

    // 在 Task 内写文件并提交——这正是 Agent 在 worktree 里的工作方式
    std::fs::write(task.root().join("新文件.txt"), "Task 侧内容\n").unwrap();
    run_git(task.root(), &["add", "-A"]);
    run_git(task.root(), &["commit", "-m", "Task 内提交"]);
    let task_root = task.root().to_path_buf();

    discard_task_workspace(&base, &task, backend).unwrap();

    // 文件没有到达 Base
    assert!(!base_path.join("新文件.txt").exists());
    // worktree 目录已移除
    assert!(!task_root.exists(), "worktree 目录仍在：{}", task_root.display());
    // 分支已删除
    assert!(
        !local_branches(&base_path).contains(&"ai/i2".to_owned()),
        "分支 ai/i2 仍在：{:?}",
        local_branches(&base_path)
    );
    // git 侧也不再登记该 worktree（只剩 Base 自身）
    let listed = run_git(&base_path, &["worktree", "list"]);
    assert_eq!(
        listed.lines().filter(|l| !l.trim().is_empty()).count(),
        1,
        "worktree 登记未清干净：{listed:?}"
    );
    // 用户的分支与提交都没有被改动
    assert_eq!(current_branch(&base_path), "main");
    assert_eq!(run_git(&base_path, &["rev-parse", "HEAD"]), base_head_before);

    // 再放弃一次会失败（工作树已不在），该中间态无法经本函数自救，
    // 故错误里必须给得出人工收拾所需的分支名与路径
    let err = discard_task_workspace(&base, &task, backend).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("ai/i2"), "错误未给出分支名：{text}");
    assert!(
        text.contains(&task_root.to_string_lossy().to_string()),
        "错误未给出 Task 根路径：{text}"
    );
    assert_eq!(
        std::fs::read_to_string(base_path.join("README.md")).unwrap(),
        "初始内容\n"
    );
}

#[test]
fn worktree_writes_do_not_reach_the_base() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, _backend) = create_task_workspace(&base, &IntentId::new("i3")).unwrap();

    let writable = task.writable_root();
    let content = "// Task 侧\n";
    writable.write("src/新模块.rs", content.as_bytes()).unwrap();
    assert_eq!(
        writable.read("src/新模块.rs").unwrap(),
        content.as_bytes()
    );

    assert!(!base_path.join("src/新模块.rs").exists());
    assert!(!base_path.join("src").exists(), "Base 下多出了 src/");
    // Base 的工作树里什么都看不出来：Task 侧写的文件不在其中，
    // `.ai/` 也被仓库本地排除挡住
    assert_eq!(
        run_git(&base_path, &["status", "--porcelain"]),
        "",
        "Base 的 git status 与预期不符"
    );
}

#[test]
fn creating_a_task_workspace_excludes_ai_without_touching_the_users_files() {
    let (_d, base_path) = git_repo();
    // 记下排除文件的原文：git init 会预置一段注释，验证「追加」必须对着它来
    let exclude_path = base_path.join(".git/info/exclude");
    let exclude_before = std::fs::read_to_string(&exclude_path).unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();
    create_task_workspace(&base, &IntentId::new("i4")).unwrap();

    // 用户的跟踪文件一字未动——§256 要求 Base 只读，改 `.gitignore` 会凭空
    // 造出一处并非用户所做的未提交修改
    assert_eq!(
        std::fs::read_to_string(base_path.join(".gitignore")).unwrap(),
        "target/\n",
        "用户的 .gitignore 被改动了"
    );
    // 排除项落在仓库本地的 info/exclude 上：不进工作树、不是跟踪文件
    let exclude_after = std::fs::read_to_string(&exclude_path).unwrap();
    assert!(
        exclude_after.starts_with(&exclude_before),
        "info/exclude 的原有内容丢失：{exclude_after:?}"
    );
    assert!(
        exclude_after.lines().any(|l| l.trim() == ".ai/"),
        "info/exclude 未追加 .ai/：{exclude_after:?}"
    );
    // 因此用户在 Base 里看不到任何变化
    assert_eq!(run_git(&base_path, &["status", "--porcelain"]), "");
}

#[test]
fn the_exclude_file_is_appended_verbatim_when_it_lacks_a_trailing_newline() {
    let (_d, base_path) = git_repo();
    std::fs::write(base_path.join(".git/info/exclude"), "/build").unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    create_task_workspace(&base, &IntentId::new("i5")).unwrap();

    // 缺结尾换行时须先补一个，否则原末行与 `.ai/` 会粘成 `/build.ai/`
    assert_eq!(
        std::fs::read_to_string(base_path.join(".git/info/exclude")).unwrap(),
        "/build\n.ai/\n"
    );
}

#[test]
fn an_exclude_file_already_listing_ai_is_left_alone() {
    let (_d, base_path) = git_repo();
    std::fs::write(base_path.join(".git/info/exclude"), ".ai\ntarget/\n").unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    create_task_workspace(&base, &IntentId::new("i6")).unwrap();

    assert_eq!(
        std::fs::read_to_string(base_path.join(".git/info/exclude")).unwrap(),
        ".ai\ntarget/\n",
        "已列出 .ai 时不应重复追加"
    );
}

#[test]
fn a_non_git_base_is_detected_as_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    assert_eq!(detect_backend(&base), WorkspaceBackend::Overlay);
}

#[test]
fn a_failing_git_command_is_reported_with_code_and_stderr() {
    let (_d, base_path) = git_repo();
    // 预先占掉分支名：worktree 建不出来，但失败原因必须原样带回
    run_git(&base_path, &["branch", "ai/i7"]);
    let base = BaseWorkspace::new(&base_path).unwrap();

    let err = create_task_workspace(&base, &IntentId::new("i7")).unwrap_err();
    match err {
        continuum_workspace::WorkspaceError::GitFailed { code, stderr } => {
            assert_ne!(code, 0, "退出码应非零");
            assert!(
                stderr.contains("ai/i7"),
                "stderr 未说明失败原因：{stderr:?}"
            );
        }
        other => panic!("期望 GitFailed，得到 {other:?}"),
    }
    // 第一步（建分支）就失败：没有目录，也没有多出任何分支
    assert!(!base_path.join(".ai/worktrees/i7").exists());
    assert_eq!(local_branches(&base_path), vec!["ai/i7".to_owned(), "main".to_owned()]);
    assert_eq!(current_branch(&base_path), "main");
}

#[test]
fn a_failed_creation_leaves_no_dangling_branch() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();

    // 情形一：目标目录已存在且非空。`git worktree add` 会**先建分支**再在此失败，
    // 若不分两步，留下的 `ai/i9` 会让同一 intent 永远无法再创建。
    let blocker = base_path.join(".ai/worktrees/i9");
    std::fs::create_dir_all(&blocker).unwrap();
    std::fs::write(blocker.join("挡路.txt"), "占用\n").unwrap();

    let err = create_task_workspace(&base, &IntentId::new("i9")).unwrap_err();
    assert!(
        matches!(err, continuum_workspace::WorkspaceError::GitFailed { .. }),
        "期望 GitFailed，得到 {err:?}"
    );
    assert_eq!(
        local_branches(&base_path),
        vec!["main".to_owned()],
        "失败的创建留下了游离分支"
    );

    // 清掉挡路的目录后，同一 intent 必须能创建成功——游离分支会让它永远失败
    std::fs::remove_dir_all(&blocker).unwrap();
    let (task, _) = create_task_workspace(&base, &IntentId::new("i9")).unwrap();
    assert_eq!(current_branch(task.root()), "ai/i9");

    // 情形二：intent 为 `@`。git 能建出 `ai/@`，却在检出工作树时报
    // `could not find created worktree '@'`（退出码 255）——同样留下游离分支
    let err = create_task_workspace(&base, &IntentId::new("@")).unwrap_err();
    assert!(
        matches!(err, continuum_workspace::WorkspaceError::GitFailed { .. }),
        "期望 GitFailed，得到 {err:?}"
    );
    assert_eq!(
        local_branches(&base_path),
        vec!["ai/i9".to_owned(), "main".to_owned()],
        "失败的创建留下了游离分支"
    );
    // 失败没有动到别人：i9 的 worktree 还在原地（清理只删自己建出来的空目录，
    // 不会误删住在 `.ai/worktrees` 里的别的 Intent）
    assert!(
        base_path.join(".ai/worktrees/i9").exists(),
        "误删了别的 Intent 的 worktree"
    );
    assert!(!base_path.join(".ai/worktrees/@").exists());
}

#[test]
fn a_failed_creation_does_not_touch_the_exclude_file() {
    let (_d, base_path) = git_repo();
    let exclude_path = base_path.join(".git/info/exclude");
    let before = std::fs::read_to_string(&exclude_path).unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();

    // `@` 能过 `check_intent`，却被 git 拒绝——创建失败时 `.ai/` 根本没被建出来，
    // 也就没有要排除的东西，排除文件不该被追加一行
    let err = create_task_workspace(&base, &IntentId::new("@")).unwrap_err();
    assert!(
        matches!(err, continuum_workspace::WorkspaceError::GitFailed { .. }),
        "期望 GitFailed，得到 {err:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&exclude_path).unwrap(),
        before,
        "失败的创建改动了 info/exclude"
    );
    assert!(!base_path.join(".ai").exists());
}

#[test]
fn a_failure_while_excluding_rolls_the_worktree_back() {
    let (_d, base_path) = git_repo();
    // 注入排除文件的故障：把它换成一个目录，读它必失败
    let exclude_path = base_path.join(".git/info/exclude");
    std::fs::remove_file(&exclude_path).unwrap();
    std::fs::create_dir(&exclude_path).unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    let err = create_task_workspace(&base, &IntentId::new("i10")).unwrap_err();
    assert!(
        matches!(err, continuum_workspace::WorkspaceError::IoFailed { .. }),
        "期望 IoFailed，得到 {err:?}"
    );

    // 排除这一步失败时，已经建出来的 worktree 与分支同样要回收：
    // 否则同一 intent 的重试会因分支已存在而永久失败
    assert!(
        !base_path.join(".ai/worktrees/i10").exists(),
        "失败后 worktree 未回收"
    );
    assert_eq!(
        local_branches(&base_path),
        vec!["main".to_owned()],
        "失败后分支未回收"
    );
    assert!(!base_path.join(".ai").exists(), "失败后留下了 .ai/");

    // 修好排除文件后，同一 intent 必须能创建成功
    std::fs::remove_dir(&exclude_path).unwrap();
    std::fs::write(&exclude_path, "").unwrap();
    let (task, _) = create_task_workspace(&base, &IntentId::new("i10")).unwrap();
    assert_eq!(current_branch(task.root()), "ai/i10");
}

#[test]
fn an_intent_id_that_is_not_a_single_path_component_is_rejected() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let entries_before = dir_entries(&base_path);

    // `../../x` 是这条判定的由来：不挡它，worktree 路径与分支名会一并越出
    // `.ai/worktrees`，而 `new_outside` 拦不住（折叠后落在 Base 之内，反而通过）
    for bad in ["../../x", "a/b", "../逃逸", "", ".", ".."] {
        let err = create_task_workspace(&base, &IntentId::new(bad)).unwrap_err();
        assert!(
            matches!(
                err,
                continuum_workspace::WorkspaceError::InvalidIntent { .. }
            ),
            "Intent 标识 {bad:?} 未被拒绝：{err:?}"
        );
    }
    // 拒绝发生在 git 之前：没有分支、Base 下没有任何新目录
    assert_eq!(local_branches(&base_path), vec!["main".to_owned()]);
    assert!(!base_path.join(".ai").exists(), "`.ai/` 被建出来了");
    assert_eq!(
        dir_entries(&base_path),
        entries_before,
        "Base 下多出了东西"
    );
}

#[test]
fn a_base_that_is_itself_a_linked_worktree_works() {
    let (_d, main_path) = git_repo();
    // Base 本身是一个 linked worktree：此时 `<base>/.git` 是**文件**而非目录，
    // 排除文件在公共仓库那边。这正是本仓库自己跑起来时的形态。
    let linked = main_path.join("linked");
    run_git(
        &main_path,
        &["worktree", "add", "-b", "w1", linked.to_str().unwrap()],
    );
    let linked = linked.canonicalize().unwrap();
    let base = BaseWorkspace::new(&linked).unwrap();
    assert_eq!(detect_backend(&base), WorkspaceBackend::Worktree);

    let (task, backend) = create_task_workspace(&base, &IntentId::new("i8")).unwrap();
    assert_eq!(backend, WorkspaceBackend::Worktree);
    assert_eq!(current_branch(task.root()), "ai/i8");

    // 排除项落在公共仓库的 info/exclude 上——linked worktree 下 git 只读那一份
    let exclude = std::fs::read_to_string(main_path.join(".git/info/exclude")).unwrap();
    assert!(
        exclude.lines().any(|l| l.trim() == ".ai/"),
        "公共仓库的 info/exclude 未追加 .ai/：{exclude:?}"
    );
    // linked worktree 自身的工作树里看不到变化
    assert_eq!(run_git(&linked, &["status", "--porcelain"]), "");
}
