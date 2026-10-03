//! 重启恢复：`EXECUTING` 的效应在重启后转 `UNKNOWN`（设计上篇第 7.4 节、
//! §7.6 的「不猜」）。
//!
//! 夹具 `db()` 与 `tests/persist.rs` 的同形：只依赖 `p2_effect_migrations` 与
//! `Db`，建库后必须 `migrate()`。恢复走 `continuum_persist::run_recovery` 这条
//! 真实路径（而不是直接调 `hook.run`），这样钩子被注册进注册表后能否在阶段事务
//! 内跑通也一并被覆盖。

use continuum_effect::{
    advance, load_effect, p2_effect_migrations, record_planned, Effect, EffectId, EffectState,
    EffectType, MarkExecutingAsUnknown,
};
use continuum_persist::{
    builtin_migrations, run_recovery, Db, Migration, RecoveryHook, RecoveryPhase,
    RecoveryRegistry, Tx, Value,
};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p2_effect_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条 `PLANNED` 记录，字段取非默认值（默认值会让列写串之类的缺陷静默）。
fn sample(id: &str, key: &str) -> Effect {
    Effect {
        id: EffectId::new(id),
        effect_type: EffectType::PushBranch,
        target: "refs/heads/feature".to_owned(),
        parameters: json!({"branch": "feature", "force": false}),
        authorization: "approved-by-test".to_owned(),
        idempotency_key: key.to_owned(),
        state: EffectState::Planned,
        planned_at: 1_700_000_000_000,
        updated_at: 1_700_000_000_001,
    }
}

fn state_of(tx: &Tx<'_>, id: &str) -> EffectState {
    load_effect(tx, &EffectId::new(id))
        .unwrap()
        .unwrap_or_else(|| panic!("{id} 应存在"))
        .state
}

fn updated_at_of(tx: &Tx<'_>, id: &str) -> i64 {
    load_effect(tx, &EffectId::new(id))
        .unwrap()
        .unwrap()
        .updated_at
}

fn audit_count(tx: &Tx<'_>) -> i64 {
    match tx.query("SELECT COUNT(*) FROM audit_log", &[]).unwrap()[0][0] {
        Value::Int(i) => i,
        ref other => panic!("COUNT 应为整数，实际 {other:?}"),
    }
}

/// 建三条记录：`e1` 停在 `EXECUTING`、`e2` 停在 `PLANNED`、`e3` 停在 `COMMITTED`。
fn three_records() -> (tempfile::TempDir, Db) {
    let (dir, db) = db();
    let tx = db.begin().unwrap();

    record_planned(&tx, &sample("e1", "k1")).unwrap();
    let e1 = EffectId::new("e1");
    advance(&tx, &e1, EffectState::Authorized, 10).unwrap();
    advance(&tx, &e1, EffectState::Executing, 11).unwrap();

    record_planned(&tx, &sample("e2", "k2")).unwrap();

    record_planned(&tx, &sample("e3", "k3")).unwrap();
    let e3 = EffectId::new("e3");
    advance(&tx, &e3, EffectState::Authorized, 20).unwrap();
    advance(&tx, &e3, EffectState::Executing, 21).unwrap();
    advance(&tx, &e3, EffectState::Committed, 22).unwrap();

    tx.commit().unwrap();
    (dir, db)
}

/// 造一个只注册了本钩子的注册表，连同该钩子的计数字柄
/// （`counter()` 给的是同一个 `AtomicUsize` 的另一个引用，注册后仍读得到）。
fn registry_with_hook() -> (RecoveryRegistry, Arc<AtomicUsize>) {
    let hook = MarkExecutingAsUnknown::new();
    let counter = hook.counter();
    let mut registry = RecoveryRegistry::new();
    registry.register(Box::new(hook));
    (registry, counter)
}

#[test]
fn executing_becomes_unknown_after_recovery() {
    let (_dir, db) = three_records();
    // 恢复前：e1 确实停在 EXECUTING（否则下面的断言可能因夹具写错而假过）
    {
        let tx = db.begin().unwrap();
        assert_eq!(state_of(&tx, "e1"), EffectState::Executing);
    }

    let (registry, _counter) = registry_with_hook();
    run_recovery(&db, &registry).unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(
        state_of(&tx, "e1"),
        EffectState::Unknown,
        "停在 EXECUTING 的记录在恢复后应为 UNKNOWN"
    );
    assert_eq!(
        state_of(&tx, "e2"),
        EffectState::Planned,
        "停在 PLANNED 的记录尚未触碰外部，恢复不应改动它"
    );
    assert_eq!(
        state_of(&tx, "e3"),
        EffectState::Committed,
        "已是终态的记录，恢复不应改动它"
    );
    tx.commit().unwrap();
}

#[test]
fn recovery_never_guesses() {
    let (_dir, db) = three_records();
    let (registry, _counter) = registry_with_hook();
    run_recovery(&db, &registry).unwrap();

    let tx = db.begin().unwrap();
    let after = state_of(&tx, "e1");
    // 设计 §7.6 的「不猜」：外部操作究竟做没做成无从得知，复原成 COMMITTED
    // 会谎报「做成了」，复原成 FAILED 会谎报「没做成」。
    assert_ne!(
        after,
        EffectState::Committed,
        "恢复不得把 EXECUTING 猜成 COMMITTED"
    );
    assert_ne!(
        after,
        EffectState::Failed,
        "恢复不得把 EXECUTING 猜成 FAILED"
    );
    assert_eq!(after, EffectState::Unknown, "只能是 UNKNOWN");
    tx.commit().unwrap();
}

#[test]
fn the_hook_reports_how_many_it_marked() {
    let (_dir, db) = three_records();
    let (registry, counter) = registry_with_hook();
    assert_eq!(counter.load(Ordering::Relaxed), 0, "未跑之前计数应为 0");

    // 恢复前审计行数：e1（登记 + 2 次推进）+ e2（登记）+ e3（登记 + 3 次推进）= 8
    let before = {
        let tx = db.begin().unwrap();
        let n = audit_count(&tx);
        tx.commit().unwrap();
        n
    };
    assert_eq!(before, 8, "夹具的审计行数应与推算式相符");

    run_recovery(&db, &registry).unwrap();

    assert_eq!(counter.load(Ordering::Relaxed), 1, "本次只标记了 e1 一条");

    let tx = db.begin().unwrap();
    // 标记走的是 advance：每次成功的推进在同事务内各追加一条审计。
    // 若改为直接 UPDATE 状态列，这里会少一条。
    assert_eq!(
        audit_count(&tx),
        before + 1,
        "标记一条应恰好追加一条审计（说明走的是 advance 而非直接 UPDATE）"
    );
    // now 取的是该记录既有的 updated_at，故恢复本身不改动时间戳
    assert_eq!(
        updated_at_of(&tx, "e1"),
        11,
        "恢复取记录既有的 updated_at 作为 now，时间戳应保持原值"
    );
    tx.commit().unwrap();

    // 再跑一次：计数被覆写为本次的数目（0），而不是在上次的 1 上累加。
    // 恢复会被重复调用（每次启动一次），累加的计数会把「本次没标记任何记录」
    // 报成「标记了 2 条」。
    run_recovery(&db, &registry).unwrap();
    assert_eq!(
        counter.load(Ordering::Relaxed),
        0,
        "第二次恢复没有 EXECUTING 的记录可标记，计数应覆写为 0 而非累加成 2"
    );
    let tx = db.begin().unwrap();
    assert_eq!(audit_count(&tx), before + 1, "第二次恢复不应再落审计");
    tx.commit().unwrap();
}

#[test]
fn only_executing_records_are_touched() {
    // 模块文档写「只处理 EXECUTING」。三种非 EXECUTING 的记录在
    // `executing_becomes_unknown_after_recovery` 里已各有一条（PLANNED、
    // COMMITTED）；这里补齐其余四个状态，使「只处理 EXECUTING」这句有对应用例，
    // 而不是只有实现里的筛选条件。
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    record_planned(&tx, &sample("e1", "k1")).unwrap();
    let e1 = EffectId::new("e1");
    advance(&tx, &e1, EffectState::Authorized, 10).unwrap();
    advance(&tx, &e1, EffectState::Executing, 11).unwrap();

    // AUTHORIZED：停在执行之前
    record_planned(&tx, &sample("e2", "k2")).unwrap();
    advance(&tx, &EffectId::new("e2"), EffectState::Authorized, 20).unwrap();
    // FAILED 与 ROLLED_BACK：从 PLANNED 直接可到的两条终态
    record_planned(&tx, &sample("e3", "k3")).unwrap();
    advance(&tx, &EffectId::new("e3"), EffectState::Failed, 30).unwrap();
    record_planned(&tx, &sample("e4", "k4")).unwrap();
    advance(&tx, &EffectId::new("e4"), EffectState::RolledBack, 40).unwrap();
    // UNKNOWN：已经处在「不知道」，恢复不该再动它（也不该重复计数）
    record_planned(&tx, &sample("e5", "k5")).unwrap();
    let e5 = EffectId::new("e5");
    advance(&tx, &e5, EffectState::Authorized, 50).unwrap();
    advance(&tx, &e5, EffectState::Executing, 51).unwrap();
    advance(&tx, &e5, EffectState::Unknown, 52).unwrap();
    tx.commit().unwrap();

    let (registry, counter) = registry_with_hook();
    run_recovery(&db, &registry).unwrap();

    let tx = db.begin().unwrap();
    for (id, expected) in [
        ("e1", EffectState::Unknown), // 唯一被标记的那条
        ("e2", EffectState::Authorized),
        ("e3", EffectState::Failed),
        ("e4", EffectState::RolledBack),
        ("e5", EffectState::Unknown),
    ] {
        assert_eq!(state_of(&tx, id), expected, "{id} 的状态不符合预期");
    }
    assert_eq!(
        counter.load(Ordering::Relaxed),
        1,
        "五条记录中只有 e1 处于 EXECUTING，计数应为 1"
    );
    tx.commit().unwrap();
}

#[test]
fn the_hook_runs_in_the_incomplete_effects_phase() {
    // Task 12 按 phase 把钩子分派到 §319 的对应阶段；取错阶段会让它在该跑的
    // 时候不跑（且 report 里看不出异常）。
    let hook = MarkExecutingAsUnknown::new();
    assert_eq!(
        hook.phase(),
        RecoveryPhase::ReconcileIncompleteEffects,
        "未完成的外部操作对应 ReconcileIncompleteEffects"
    );
}
