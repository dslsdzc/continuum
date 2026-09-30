//! 崩溃原子性测试夹具。
//!
//! 在一个事务内写两张表，打印 READY 表示写入完成但未提交。
//! 无 `--commit` 时停住等待外部终止，测试用它验证未提交事务在进程被杀后全部回滚。
//! 带 `--commit` 时提交并退出，用作对照组。

use continuum_persist::{Db, Migration, Value};
use std::io::Write;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("用法: crash-writer <db 路径> [--commit]");
    let commit = args.any(|a| a == "--commit");
    let mut ms = continuum_persist::builtin_migrations();
    ms.push(Migration::new(
        60,
        "crash_fixture",
        "CREATE TABLE IF NOT EXISTS fx_artifact (id TEXT PRIMARY KEY);
         CREATE TABLE IF NOT EXISTS fx_node_state (node_id TEXT PRIMARY KEY, state TEXT NOT NULL);",
    ));
    let db = Db::open_with(Path::new(&path), ms).expect("打开数据库失败");
    db.migrate().expect("迁移失败");

    let tx = db.begin().expect("开启事务失败");
    tx.execute(
        "INSERT INTO fx_artifact (id) VALUES (?1)",
        &[Value::text("a1")],
    )
    .expect("写 fx_artifact 失败");
    tx.execute(
        "INSERT INTO fx_node_state (node_id, state) VALUES (?1, ?2)",
        &[Value::text("n1"), Value::text("RUNNING")],
    )
    .expect("写 fx_node_state 失败");

    println!("READY");
    std::io::stdout().flush().expect("flush 失败");

    if commit {
        tx.commit().expect("提交失败");
        println!("COMMITTED");
        return;
    }

    // 不提交，等待外部终止。进程被杀时 drop 不执行，ROLLBACK 由存储层在恢复时完成。
    std::thread::sleep(std::time::Duration::from_secs(60));
}
