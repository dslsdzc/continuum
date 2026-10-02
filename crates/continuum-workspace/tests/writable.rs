//! `WritablePath` 的行为用例。
//!
//! 类型层保证（「Base 路径构造不出 WritablePath」）由 `type_level.rs` 的编译失败用例
//! 承担；本文件的用例只覆盖构造成功之后的读写、越界拒绝，以及 Task 根与 Base 的重叠守卫。

use continuum_workspace::{BaseWorkspace, IntentId, TaskWorkspace, WorkspaceError};

/// 在 `base` 之内建一个 Task Workspace，根为 `<base>/task`。
fn task_inside(base: &BaseWorkspace, dir: &std::path::Path) -> TaskWorkspace {
    TaskWorkspace::new_outside(base, dir.join("task"), IntentId::new("i1")).unwrap()
}

#[test]
fn writable_path_writes_inside_the_task_root() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let task = task_inside(&base, dir.path());
    let w = task.writable_root();
    w.write("a/b.txt", b"hello").unwrap();
    assert_eq!(w.read("a/b.txt").unwrap(), b"hello");
}

#[test]
fn writable_path_rejects_parent_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let task = task_inside(&base, dir.path());
    let w = task.writable_root();
    let err = w.join("../escape").unwrap_err();
    assert!(
        matches!(err, WorkspaceError::EscapesRoot { .. }),
        "实际 {err:?}"
    );
}

#[test]
fn writable_path_rejects_absolute_paths() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let task = task_inside(&base, dir.path());
    let w = task.writable_root();
    let err = w.join("/etc/passwd").unwrap_err();
    assert!(
        matches!(err, WorkspaceError::EscapesRoot { .. }),
        "实际 {err:?}"
    );
}

#[test]
fn base_workspace_rejects_a_missing_directory() {
    let err = BaseWorkspace::new("/definitely/not/here").unwrap_err();
    assert!(
        matches!(err, WorkspaceError::MissingBase { .. }),
        "实际 {err:?}"
    );
}

#[test]
fn task_root_may_not_be_the_base_itself() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let err = TaskWorkspace::new_outside(&base, base.root(), IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::Overlaps { .. }),
        "实际 {err:?}"
    );
}

#[test]
fn task_root_may_not_be_an_ancestor_of_the_base() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let ancestor = dir.path().parent().expect("tempdir 必有父目录");
    let err = TaskWorkspace::new_outside(&base, ancestor, IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::Overlaps { .. }),
        "实际 {err:?}"
    );
}

/// 被拒绝的 root 不得在磁盘上留下任何目录：判定先于落盘。
///
/// `<tmp>/zzz/..` 词法上解析为 `<tmp>`，即 Base `<tmp>/base` 的祖先，
/// 而 `zzz` 并不存在。若判定排在 `create_dir_all` 之后，`<tmp>/zzz` 会先被建出来。
#[test]
fn rejected_root_leaves_no_directory_behind() {
    let dir = tempfile::tempdir().unwrap();
    let base_dir = dir.path().join("base");
    std::fs::create_dir(&base_dir).unwrap();
    let base = BaseWorkspace::new(&base_dir).unwrap();
    let root = dir.path().join("zzz").join("..");
    let err = TaskWorkspace::new_outside(&base, &root, IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::Overlaps { .. }),
        "实际 {err:?}"
    );
    assert!(
        !dir.path().join("zzz").exists(),
        "被拒绝的 root 留下了目录：{}",
        dir.path().join("zzz").display()
    );
}

/// 被接受的 root 若在「不存在的尾部」含 `..`，只应建出候选路径本身。
///
/// `<tmp>/zzz/../task` 的候选是 `<tmp>/task`；按字面建会连带建出 `<tmp>/zzz`，
/// 即创建了什么与句柄持有什么不一致。
#[test]
fn accepted_root_creates_only_the_candidate_path() {
    let dir = tempfile::tempdir().unwrap();
    let base_dir = dir.path().join("base");
    std::fs::create_dir(&base_dir).unwrap();
    let base = BaseWorkspace::new(&base_dir).unwrap();
    let root = dir.path().join("zzz").join("..").join("task");
    let task = TaskWorkspace::new_outside(&base, &root, IntentId::new("i1")).unwrap();
    let expected = std::fs::canonicalize(dir.path()).unwrap().join("task");
    assert_eq!(task.root(), expected, "句柄根应为候选路径");
    assert!(
        !dir.path().join("zzz").exists(),
        "候选路径之外被建了出来：{}",
        dir.path().join("zzz").display()
    );
}

/// 指向 Base 的 symlink：逐字符比较必然漏掉，只有规范化后才判得出重叠。
#[cfg(unix)]
#[test]
fn task_root_may_not_be_a_symlink_to_the_base() {
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    let link = dir.path().join("link-to-base");
    std::os::unix::fs::symlink(base.root(), &link).unwrap();
    let err = TaskWorkspace::new_outside(&base, &link, IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::Overlaps { .. }),
        "实际 {err:?}"
    );
}
