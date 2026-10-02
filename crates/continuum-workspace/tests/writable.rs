//! `WritablePath` 的行为用例。
//!
//! 类型层保证（「Base 路径构造不出 WritablePath」）由 `type_level.rs` 的编译失败用例
//! 承担；本文件的用例只覆盖构造成功之后的读写与越界拒绝。

use continuum_workspace::{BaseWorkspace, IntentId, TaskWorkspace, WorkspaceError};

#[test]
fn writable_path_writes_inside_the_task_root() {
    let dir = tempfile::tempdir().unwrap();
    let task = TaskWorkspace::new_at(dir.path(), IntentId::new("i1")).unwrap();
    let w = task.writable_root();
    w.write("a/b.txt", b"hello").unwrap();
    assert_eq!(w.read("a/b.txt").unwrap(), b"hello");
}

#[test]
fn writable_path_rejects_parent_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let task = TaskWorkspace::new_at(dir.path(), IntentId::new("i1")).unwrap();
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
    let task = TaskWorkspace::new_at(dir.path(), IntentId::new("i1")).unwrap();
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
