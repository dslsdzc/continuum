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
