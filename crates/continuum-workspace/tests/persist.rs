//! `workspace` 表的往返与列编码。
//!
//! 表的读写在 `src/persist.rs`；本文件只经 `Tx` 访问数据库，不直接构造 `Db` 的
//! 连接对象（连接是 `continuum-persist` 的私有字段）。

use continuum_persist::{builtin_migrations, Db, Migration, PersistError, Value};
use continuum_workspace::{
    BaseWorkspace, IntentId, TaskWorkspace, WorkspaceBackend, WorkspaceRecord, load_workspace,
    p2_workspace_migrations, remove_workspace, save_workspace,
};
use std::path::PathBuf;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p2_workspace_migrations());
    let db = Db::open_with(&dir.path().join("t.db"), migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn record(intent: &str, backend: WorkspaceBackend) -> WorkspaceRecord {
    WorkspaceRecord {
        intent_id: IntentId::new(intent),
        backend,
        path: PathBuf::from(format!("/tmp/base/.ai/worktrees/{intent}")),
        base_path: PathBuf::from("/tmp/base"),
        created_at: 1_700_000_000_000,
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

#[test]
fn workspace_record_round_trips() {
    let (_d, db) = db();

    // 两个后端各存一条：`backend` 列的两种取值都要能原样取回。
    for (intent, backend) in [
        ("i1", WorkspaceBackend::Worktree),
        ("i2", WorkspaceBackend::Overlay),
    ] {
        let rec = record(intent, backend);
        let tx = db.begin().unwrap();
        save_workspace(&tx, &rec).unwrap();
        tx.commit().unwrap();

        let tx = db.begin().unwrap();
        let got = load_workspace(&tx, &rec.intent_id).unwrap();
        assert_eq!(
            got,
            Some(rec.clone()),
            "Intent {intent} 的记录未原样取回"
        );
    }
}

#[test]
fn loading_an_unrecorded_intent_yields_none() {
    // 「没有记录」与「记录为空值」必须分开：前者是 `Ok(None)`，后者是错误。
    // 少了这条，`load_workspace` 的 `None` 分支无人过问。
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    let got = load_workspace(&tx, &IntentId::new("未记录")).unwrap();
    assert_eq!(got, None, "未记录的 Intent 应得到 None");
}

#[test]
fn backend_column_uses_the_lowercase_encoding() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    save_workspace(&tx, &record("i1", WorkspaceBackend::Worktree)).unwrap();
    save_workspace(&tx, &record("i2", WorkspaceBackend::Overlay)).unwrap();
    tx.commit().unwrap();

    // 直接查表，不经过 `load_workspace` 的还原：断言的是**列里躺着的字面取值**。
    //
    // 本用例钉的是契约——该列的写法由 `backend_str` 一处给出，规则是小写、
    // 多词以 `_` 连接，不得由类型的 `Debug` 或 serde 表示推出。
    // 就当前两个变体而言，`Debug` 小写与 serde 的 `snake_case` 恰好给出同样的
    // 字符串，故本用例不能在某一种实现与另一种之间判别；有判别力的是
    // `an_unknown_backend_value_is_rejected`（未知取值必须报错，而非落回默认后端）。
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT intent_id, backend FROM workspace ORDER BY intent_id",
            &[],
        )
        .unwrap();
    let got: Vec<(String, String)> = rows.iter().map(|r| (text(&r[0]), text(&r[1]))).collect();
    assert_eq!(
        got,
        vec![
            ("i1".to_owned(), "worktree".to_owned()),
            ("i2".to_owned(), "overlay".to_owned()),
        ]
    );
}

#[test]
fn an_unknown_backend_value_is_rejected() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    save_workspace(&tx, &record("i1", WorkspaceBackend::Worktree)).unwrap();
    // 把一个不属于本层编码的值直接写进列里——例如别的实现可能采用的
    // `Debug` 形态。读回时必须报错并指出该取值，而不是落回某个默认后端：
    // 后端决定放弃时走哪条回收路径，猜错会留下残留而不报错。
    tx.execute(
        "UPDATE workspace SET backend = ?2 WHERE intent_id = ?1",
        &[Value::text("i1"), Value::text("Worktree")],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = load_workspace(&tx, &IntentId::new("i1")).unwrap_err();
    match err {
        PersistError::Database(msg) => assert!(
            msg.contains("Worktree"),
            "错误未指出出错的取值：{msg}"
        ),
        other => panic!("期望 Database，得到 {other:?}"),
    }
}

#[test]
fn saving_the_same_intent_twice_is_rejected_without_overwriting() {
    let (_d, db) = db();
    let first = record("i1", WorkspaceBackend::Worktree);
    let tx = db.begin().unwrap();
    save_workspace(&tx, &first).unwrap();
    tx.commit().unwrap();

    // 第二条除了路径都相同——若实现改成覆盖写，这一条会把 first 的 path 改掉
    let mut second = record("i1", WorkspaceBackend::Worktree);
    second.path = PathBuf::from("/tmp/别处");
    let tx = db.begin().unwrap();
    let err = save_workspace(&tx, &second).unwrap_err();
    assert!(
        matches!(err, PersistError::Database(_)),
        "期望 Database，得到 {err:?}"
    );
    drop(tx);

    let tx = db.begin().unwrap();
    let got = load_workspace(&tx, &IntentId::new("i1")).unwrap();
    assert_eq!(got, Some(first), "重复写入改动了既有的记录");
}

#[test]
fn a_record_built_from_the_handles_carries_their_paths() {
    let base_dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(base_dir.path()).unwrap();
    let task =
        TaskWorkspace::new_outside(&base, base_dir.path().join("task"), IntentId::new("i1"))
            .unwrap();

    let rec = WorkspaceRecord::from_workspaces(&base, &task, WorkspaceBackend::Overlay, 42);

    assert_eq!(rec.intent_id, IntentId::new("i1"));
    assert_eq!(rec.backend, WorkspaceBackend::Overlay);
    assert_eq!(rec.created_at, 42);
    // path 取 Task 句柄持有的**规范化**根；base_path 取 Base 句柄持有的原文。
    // 两者都比对各自 accessor 的返回值：若实现改从别处取值（例如把创建时传入的
    // 字面 root 存进去），此处会与句柄不一致。
    assert_eq!(rec.path, task.root());
    assert_eq!(rec.base_path, base.root());

    // 经落库往返一次，确认这套字段在库里也自洽
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    save_workspace(&tx, &rec).unwrap();
    tx.commit().unwrap();
    let tx = db.begin().unwrap();
    assert_eq!(load_workspace(&tx, &IntentId::new("i1")).unwrap(), Some(rec));
}

#[test]
fn removing_a_record_makes_it_load_as_none() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    save_workspace(&tx, &record("i1", WorkspaceBackend::Worktree)).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    remove_workspace(&tx, &IntentId::new("i1")).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(
        load_workspace(&tx, &IntentId::new("i1")).unwrap(),
        None,
        "删除之后仍能读回记录"
    );
}

/// 「记录的生命周期与工作区一致」这条裁定的直接守卫：放弃后删记录，同一 Intent
/// 重建时 `save_workspace` 必须成功。`remove_workspace` 变成空操作时，这一条会
/// 撞上主键而变红。
#[test]
fn a_workspace_can_be_recorded_again_after_its_record_is_removed() {
    let (_d, db) = db();
    let first = record("i1", WorkspaceBackend::Overlay);
    let tx = db.begin().unwrap();
    save_workspace(&tx, &first).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    remove_workspace(&tx, &IntentId::new("i1")).unwrap();
    tx.commit().unwrap();

    let mut second = record("i1", WorkspaceBackend::Overlay);
    second.path = PathBuf::from("/tmp/重建");
    let tx = db.begin().unwrap();
    save_workspace(&tx, &second).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(
        load_workspace(&tx, &IntentId::new("i1")).unwrap(),
        Some(second),
        "重建后的记录不是新写的那条"
    );
}

/// 删除不存在的记录不是错误——放弃路径上「记录本就不存在」不该让清理失败。
#[test]
fn removing_an_unrecorded_intent_is_not_an_error() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    remove_workspace(&tx, &IntentId::new("未曾记录")).unwrap();
    // 幂等：再删一次也还是 Ok
    remove_workspace(&tx, &IntentId::new("未曾记录")).unwrap();
    tx.commit().unwrap();
}

/// 非 UTF-8 的路径分量在落库时被替换字符悄悄改写，读回就指向另一个路径，
/// 而记录与磁盘实物分叉后无从察觉。故此处是失败路径，不是静默降级。
#[cfg(unix)]
#[test]
fn a_non_utf8_path_is_rejected_instead_of_silently_replaced() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let (_d, db) = db();
    let mut rec = record("i1", WorkspaceBackend::Overlay);
    // 0x80 不是合法 UTF-8 起始字节
    rec.path = PathBuf::from(OsString::from_vec(b"/tmp/\x80".to_vec()));

    let tx = db.begin().unwrap();
    let err = save_workspace(&tx, &rec).unwrap_err();
    match err {
        PersistError::Database(msg) => assert!(
            msg.contains("UTF-8"),
            "错误未说明原因：{msg}"
        ),
        other => panic!("期望 Database，得到 {other:?}"),
    }
}
