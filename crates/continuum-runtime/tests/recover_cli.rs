//! `recover` 子命令的端到端用例（Task 12）：**装配点真的把 effect 的恢复钩子注册进了
//! `recover` 这条路**，跑完之后停在 `EXECUTING` 的效应转 `UNKNOWN`，其余状态一字不动。
//!
//! 与 `task_cli.rs` 的 `a_killed_command_leaves_executing_and_recovery_turns_it_unknown`
//! 的分工：那条在**测试进程内**直接调 `run_recovery` + `MarkExecutingAsUnknown`，验的是
//! 钩子本身；本文件驱动**真实二进制**，验的是 `main.rs` / `recover_cmd.rs` 那条装配——
//! 钩子写得再对，没注册到 `recover` 上，崩溃重启后记录仍停在 `EXECUTING`。两者都要有：
//! 前者的照片盖不住后者（未接线时前者照绿）。
//!
//! 不经沙箱、不跑命令，故本文件的用例**不受任何能力门控**。

use continuum_effect::{Effect, EffectId, EffectState, EffectType, advance, record_planned};
use continuum_persist::{Db, Value};
use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_continuum-runtime");

/// 跑一次 `recover`，返回输出；非零退出即断言失败。
fn recover_ok(db: &Path) -> String {
    let out: Output = Command::new(BIN)
        .args(["recover", "--db"])
        .arg(db)
        .output()
        .expect("continuum-runtime 无法执行");
    assert!(
        out.status.success(),
        "recover 退出码 {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("输出不是 UTF-8")
}

/// 在 `effect` 表里种一条记录：先写 `PLANNED`，再按 `steps` 逐级推进。
///
/// 走 [`record_planned`] 与 [`advance`] 而不是手写 SQL：本用例要观察的是钩子**经状态机**
/// 做出的推进，手写 SQL 会让「钩子绕开状态机直接改列」这一路不受检。
fn seed(tx: &continuum_persist::Tx<'_>, target: &str, steps: &[EffectState]) {
    let id = EffectId::new(format!("key:{target}"));
    record_planned(
        tx,
        &Effect {
            id: id.clone(),
            effect_type: EffectType::Publish,
            target: target.to_owned(),
            parameters: serde_json::json!({}),
            authorization: "种下的记录".to_owned(),
            idempotency_key: format!("key:{target}"),
            state: EffectState::Planned,
            planned_at: 1,
            updated_at: 1,
        },
    )
    .expect("种下记录失败");
    for (i, to) in steps.iter().enumerate() {
        advance(tx, &id, *to, 2 + i as i64).expect("推进状态失败");
    }
}

/// 库里 `effect` 表的全部 `(target, state)`，按 target 排序。
///
/// 读的是**落库的文本**而不是 `load_effect` 的类型化结果：本用例要钉的正是状态列的编码
/// ——「unknown」这个串由钩子写下，经类型化读回会把编码那一层盖掉。
fn effect_rows(db: &Path) -> Vec<(String, String)> {
    let handle = Db::open(db).expect("打开数据库失败");
    let tx = handle.begin().unwrap();
    let rows = tx
        .query("SELECT target, state FROM effect ORDER BY target", &[])
        .expect("读 effect 表失败");
    let out = rows
        .iter()
        .map(|r| {
            let text = |v: &Value| match v {
                Value::Text(s) => s.clone(),
                other => panic!("列应为文本，实际 {other:?}"),
            };
            (text(&r[0]), text(&r[1]))
        })
        .collect();
    tx.commit().unwrap();
    out
}

/// 跑完 `recover` 后 `EXECUTING` 转 `UNKNOWN`，其余状态不变。
#[test]
fn recover_turns_executing_effects_unknown_and_leaves_the_rest_alone() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");

    // 第一次跑：建库、应用迁移（`effect` 表由此建出）。顺带钉住 P1 的钩子仍装配在
    // 它自己的阶段上——本 task 往注册表里加了一个钩子，不得把原有的挤掉或挪位。
    let first = recover_ok(&db);
    assert!(
        first.contains("reconcile running nodes: 1 钩子"),
        "「reconcile running nodes」阶段应恰有 1 个钩子（P1 的节点钩子）\n--- stdout ---\n{first}"
    );
    // 「reconcile incomplete effects」那一栏的钩子数**故意留到末尾才断**：它是装配在
    // 哪个阶段的旁证，而本用例要钉的主要事实是「记录真的被推进了」。若把它排在这里，
    // 「钩子根本没注册」这条变异会先撞上计数断言，状态断言就被挡住了、没有照片。

    // 种四条，覆盖「要转的」「两个终态」「尚未触碰外部的」三类里各一个。
    {
        let handle = Db::open(&db).expect("打开数据库失败");
        let tx = handle.begin().unwrap();
        seed(&tx, "exec", &[EffectState::Authorized, EffectState::Executing]);
        seed(
            &tx,
            "committed",
            &[
                EffectState::Authorized,
                EffectState::Executing,
                EffectState::Committed,
            ],
        );
        seed(&tx, "planned", &[]);
        seed(
            &tx,
            "already_unknown",
            &[
                EffectState::Authorized,
                EffectState::Executing,
                EffectState::Unknown,
            ],
        );
        tx.commit().unwrap();
    }
    // 前置事实：种完之后恰有一条 EXECUTING。少了这一句，若 `advance` 没把状态推上去，
    // 下面的「EXECUTING 转 UNKNOWN」会因为入库时就不是 EXECUTING 而假绿。
    let before = effect_rows(&db);
    assert_eq!(
        before
            .iter()
            .filter(|(_, s)| s == "executing")
            .count(),
        1,
        "种完之后应恰有一条停在 EXECUTING，实际 {before:?}"
    );
    assert_eq!(
        before.iter().filter(|(_, s)| s == "unknown").count(),
        1,
        "种完之后应恰有一条已 UNKNOWN（它不该被再次推进）"
    );

    let second = recover_ok(&db);

    // 逐项对照。期望值**手写**在这里（钉格式），不调实现去生成。
    // 本条排在钩子数断言之前：钩子没注册时，这一条是**先**变红的那一条——它才是本用例
    // 要钉的事实（记录真的被推进了），钩子数只是「装配在哪个阶段」的旁证。
    assert_eq!(
        effect_rows(&db),
        vec![
            ("already_unknown".to_owned(), "unknown".to_owned()),
            ("committed".to_owned(), "committed".to_owned()),
            ("exec".to_owned(), "unknown".to_owned()),
            ("planned".to_owned(), "planned".to_owned()),
        ],
        "EXECUTING 应转 UNKNOWN，PLANNED 与两个终态应原样不动"
    );

    // 「不猜」的两个方向逐条各断言一次（设计第 6.4 节）：转成 UNKNOWN 不得被读成
    // 「确定成功」或「确定失败」。
    let exec = effect_rows(&db)
        .into_iter()
        .find(|(t, _)| t == "exec")
        .map(|(_, s)| s)
        .expect("exec 记录不见了");
    assert_ne!(exec, "committed", "不得把 EXECUTING 猜成 COMMITTED");
    assert_ne!(exec, "failed", "不得把 EXECUTING 猜成 FAILED");

    assert!(
        second.contains("reconcile incomplete effects: 1 钩子"),
        "第二次运行的钩子数应与首次一致\n--- stdout ---\n{second}"
    );
}

/// 没有 `EXECUTING` 记录时恢复照跑，不误伤其它状态。
///
/// 上一条用例里「其余不变」与「有东西要转」同时存在，二者若被写成同一段逻辑（例如
/// 「无匹配即整表改写」）仍可能通过。本条把「无可转」这一支单独钉一次。
#[test]
fn recover_without_executing_effects_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    recover_ok(&db);

    {
        let handle = Db::open(&db).expect("打开数据库失败");
        let tx = handle.begin().unwrap();
        seed(&tx, "planned", &[]);
        seed(&tx, "failed", &[EffectState::Authorized, EffectState::Failed]);
        tx.commit().unwrap();
    }

    recover_ok(&db);

    assert_eq!(
        effect_rows(&db),
        vec![
            ("failed".to_owned(), "failed".to_owned()),
            ("planned".to_owned(), "planned".to_owned()),
        ],
        "无 EXECUTING 时不得改动任何记录"
    );
}
