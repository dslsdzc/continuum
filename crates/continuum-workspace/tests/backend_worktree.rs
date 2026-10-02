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
    // 预置一份非空 `.gitignore`：用例要验证「追加」而非「覆盖」，
    // 空文件或缺失文件都测不出覆盖。
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
    // Base 的 git status 只因 .gitignore 的追加而变（`.ai/` 已被忽略），
    // 写入 Task 的文件不在其中（run_git 已裁掉 porcelain 的前导状态位空格）
    assert_eq!(
        run_git(&base_path, &["status", "--porcelain"]),
        "M .gitignore",
        "Base 的 git status 与预期不符"
    );
}

#[test]
fn creating_a_task_workspace_gitignores_the_ai_directory() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();
    create_task_workspace(&base, &IntentId::new("i4")).unwrap();

    let ignore = std::fs::read_to_string(base_path.join(".gitignore")).unwrap();
    // 原有内容保留（追加而非覆盖）
    assert!(ignore.contains("target/"), "原有条目丢失：{ignore:?}");
    assert!(
        ignore.lines().any(|l| l.trim() == ".ai/"),
        "未追加 .ai/：{ignore:?}"
    );
    // `.ai/` 不出现在 Base 的 git status 中
    let status = run_git(&base_path, &["status", "--porcelain"]);
    assert!(!status.contains(".ai"), "git status 里出现了 .ai：{status:?}");
}

#[test]
fn a_gitignore_is_appended_verbatim_when_it_lacks_a_trailing_newline() {
    let (_d, base_path) = git_repo();
    std::fs::write(base_path.join(".gitignore"), "/build").unwrap();
    run_git(&base_path, &["add", "-A"]);
    run_git(&base_path, &["commit", "-m", "改写 .gitignore"]);

    let base = BaseWorkspace::new(&base_path).unwrap();
    create_task_workspace(&base, &IntentId::new("i5")).unwrap();

    // 缺结尾换行时须先补一个，否则原末行与 `.ai/` 会粘成 `/build.ai/`
    assert_eq!(
        std::fs::read_to_string(base_path.join(".gitignore")).unwrap(),
        "/build\n.ai/\n"
    );
}

#[test]
fn a_gitignore_already_listing_ai_is_left_alone() {
    let (_d, base_path) = git_repo();
    std::fs::write(base_path.join(".gitignore"), ".ai\ntarget/\n").unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    create_task_workspace(&base, &IntentId::new("i6")).unwrap();

    assert_eq!(
        std::fs::read_to_string(base_path.join(".gitignore")).unwrap(),
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
    // 失败的创建不留下半成品
    assert!(!base_path.join(".ai/worktrees/i7").exists());
    assert_eq!(current_branch(&base_path), "main");
}

#[test]
fn an_intent_id_that_is_not_a_single_path_component_is_rejected() {
    let (_d, base_path) = git_repo();
    let base = BaseWorkspace::new(&base_path).unwrap();

    for bad in ["../逃逸", "a/b", "", ".", ".."] {
        let err = create_task_workspace(&base, &IntentId::new(bad)).unwrap_err();
        assert!(
            matches!(
                err,
                continuum_workspace::WorkspaceError::InvalidIntent { .. }
            ),
            "Intent 标识 {bad:?} 未被拒绝：{err:?}"
        );
    }
    // 拒绝发生在 git 之前：没有分支、没有目录被建出来
    assert_eq!(local_branches(&base_path), vec!["main".to_owned()]);
    assert!(!base_path.join(".ai").exists());
}
