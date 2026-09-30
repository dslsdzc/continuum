use continuum_persist::{Db, Migration, Value};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

/// 夹具迁移。两张表模拟 §318 的跨实体一致性要求。
fn fixture_db(path: &std::path::Path) -> Db {
    let mut ms = continuum_persist::builtin_migrations();
    ms.push(Migration::new(
        60,
        "crash_fixture",
        "CREATE TABLE fx_artifact (id TEXT PRIMARY KEY);
         CREATE TABLE fx_node_state (node_id TEXT PRIMARY KEY, state TEXT NOT NULL);",
    ));
    let db = Db::open_with(path, ms).unwrap();
    db.migrate().unwrap();
    db
}

/// `commit` 为真时夹具在 READY 后提交并退出，用作对照组。
fn spawn_writer(path: &std::path::Path, commit: bool) -> std::process::Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_crash-writer"));
    cmd.arg(path).stdout(Stdio::piped()).stderr(Stdio::inherit());
    if commit {
        cmd.arg("--commit");
    }
    cmd.spawn().expect("crash-writer 无法启动")
}

/// 读到 READY 表示两行已写入事务但未提交。
///
/// 用 `as_mut` 借用而非 `take` 取走：`take` 会把读端移进本函数的
/// `BufReader`，函数返回即析构、读端关闭，夹具提交后那句
/// `println!("COMMITTED")` 会撞上断管而 panic，退出码变成 101，
/// 对照组的 `status.success()` 断言会以错误的理由失败。
/// 借用则读端仍由 `child.stdout` 持有，活到用例结束，
/// 也不依赖调用方记得绑定返回值。
fn wait_ready(child: &mut Child) {
    let out = child.stdout.as_mut().expect("stdout 已管道化");
    let mut reader = BufReader::new(out);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).expect("读 stdout 失败");
        assert_ne!(n, 0, "crash-writer 提前退出，未见 READY");
        if line.trim() == "READY" {
            break;
        }
    }
}

fn count(db: &Db, table: &str) -> usize {
    let tx = db.begin().unwrap();
    tx.query(&format!("SELECT COUNT(*) FROM {table}"), &[])
        .unwrap()
        .into_iter()
        .next()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            Value::Int(i) => Some(i as usize),
            _ => None,
        })
        .unwrap_or(0)
}

#[test]
fn killed_mid_transaction_leaves_no_partial_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.db");
    fixture_db(&path);

    let mut child = spawn_writer(&path, false);
    wait_ready(&mut child);
    child.kill().expect("SIGKILL 失败");
    child.wait().expect("回收子进程失败");

    let db = fixture_db(&path);
    let a = count(&db, "fx_artifact");
    let n = count(&db, "fx_node_state");
    assert_eq!(
        (a, n),
        (0, 0),
        "事务未提交即被终止，两张表必须同时不可见，实际 artifact={a} node_state={n}"
    );
}

#[test]
fn committed_writer_leaves_both_rows_visible() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ok.db");
    fixture_db(&path);

    let mut child = spawn_writer(&path, true);
    wait_ready(&mut child);
    let status = child.wait().expect("回收子进程失败");
    assert!(status.success(), "夹具应正常退出，实际 {status:?}");

    let db = fixture_db(&path);
    assert_eq!(count(&db, "fx_artifact"), 1, "提交后 fx_artifact 应可见");
    assert_eq!(count(&db, "fx_node_state"), 1, "提交后 fx_node_state 应可见");
}
