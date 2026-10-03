//! `effect` 表的落库、状态推进与幂等键（设计下篇第 8 节、上篇第 7.3、7.6 节）。
//!
//! 本文件的夹具 `db()` 单独成立：它只依赖 `p2_effect_migrations` 与 `Db`，
//! 建库后必须 `migrate()` —— 漏掉那一行时各用例会一起挂在 `no such table`，
//! 而报错位置指向被测函数，容易误判成实现的问题。

use continuum_effect::{
    advance, find_by_idempotency_key, load_effect, p2_effect_migrations, record_planned, Effect,
    EffectId, EffectState, EffectType, StateError,
};
use continuum_persist::{builtin_migrations, Db, Migration, PersistError, Value};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p2_effect_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条 `PLANNED` 记录。字段全部取非默认值：若某列不落库或与邻列写串，
/// 读回时的整结构比对会变红（默认值/零值会让这类缺陷静默）。
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

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

fn count_effects(tx: &continuum_persist::Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM effect", &[]).unwrap();
    match rows[0][0] {
        Value::Int(i) => i,
        ref other => panic!("COUNT 应为整数，实际 {other:?}"),
    }
}

#[test]
fn record_planned_then_load_round_trips() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    let effect = sample("e1", "k1");
    record_planned(&tx, &effect).unwrap();

    let back = load_effect(&tx, &EffectId::new("e1"))
        .unwrap()
        .expect("刚写入的记录应能读回");
    // 整结构比对覆盖九个字段：任何一列写错、写串或读错都会在此变红
    assert_eq!(back, effect, "读回的各字段应与写入值相同");
    assert_eq!(back.effect_type, EffectType::PushBranch);
    assert_eq!(back.state, EffectState::Planned);

    // 幂等键查询读到同一条
    let by_key = find_by_idempotency_key(&tx, "k1")
        .unwrap()
        .expect("键 k1 应能查到记录");
    assert_eq!(by_key, effect);

    // 写入按记录携带的状态落库，不硬编码成 PLANNED：写入函数不做状态校验，
    // 状态机的把关在 advance（若此处被硬编码，这条会红）
    let mut authorized = sample("e4", "k4");
    authorized.state = EffectState::Authorized;
    record_planned(&tx, &authorized).unwrap();
    assert_eq!(
        load_effect(&tx, &EffectId::new("e4")).unwrap().unwrap(),
        authorized
    );

    // 不存在的 id 与不存在的键都是 `None`，不是错误
    assert!(
        load_effect(&tx, &EffectId::new("nope")).unwrap().is_none(),
        "不存在的 id 应返回 None"
    );
    assert!(
        find_by_idempotency_key(&tx, "nope").unwrap().is_none(),
        "不存在的键应返回 None"
    );

    tx.commit().unwrap();
}

#[test]
fn the_same_idempotency_key_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    record_planned(&tx, &sample("e1", "k1")).unwrap();

    let err = record_planned(&tx, &sample("e2", "k1")).unwrap_err();
    // 断言是哪一种 Err，不是「返回了 Err」：`PersistError` 没有独立的冲突变体，
    // 而 `Tx::execute` 把 rusqlite 的错误码抹成了字符串，故只能在 `Database`
    // 变体内按消息断言冲突来自 `idempotency_key` 上的那条唯一索引。
    match &err {
        PersistError::Database(m) => assert!(
            m.contains("UNIQUE constraint failed: effect.idempotency_key"),
            "应为 idempotency_key 上的唯一键冲突，实际 {m}"
        ),
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }

    assert_eq!(count_effects(&tx), 1, "同键被拒后库里应仍只有一条记录");

    // 挡住的是键，不是全部写入：换个键仍能写
    record_planned(&tx, &sample("e3", "k2")).unwrap();
    assert_eq!(count_effects(&tx), 2);

    tx.commit().unwrap();
}

#[test]
fn enum_columns_use_the_lowercase_encoding() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    record_planned(&tx, &sample("e1", "k1")).unwrap();

    // 第二个效应：类型与状态都是多词，用来钉住 `_` 连接
    let mut multi = sample("e2", "k2");
    multi.effect_type = EffectType::DeleteRemote;
    record_planned(&tx, &multi).unwrap();
    advance(&tx, &EffectId::new("e2"), EffectState::RolledBack, 5).unwrap();

    let rows = tx
        .query(
            "SELECT effect_type, state FROM effect ORDER BY id",
            &[],
        )
        .unwrap();
    assert_eq!(text_of(&rows[0][0]), "push_branch");
    assert_eq!(text_of(&rows[0][1]), "planned");
    assert_eq!(text_of(&rows[1][0]), "delete_remote");
    assert_eq!(text_of(&rows[1][1]), "rolled_back");

    // 落库的不是 Rust 枚举的 Debug 表示
    for raw in [text_of(&rows[0][0]), text_of(&rows[1][0]), text_of(&rows[1][1])] {
        assert!(
            !raw.contains("PushBranch") && !raw.contains("DeleteRemote") && !raw.contains("RolledBack"),
            "列里是枚举的 Debug 表示而非小写编码：{raw}"
        );
    }

    tx.commit().unwrap();
}

#[test]
fn advance_follows_the_state_machine() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    record_planned(&tx, &sample("e1", "k1")).unwrap();
    let id = EffectId::new("e1");

    for (i, to) in [EffectState::Authorized, EffectState::Executing, EffectState::Committed]
        .into_iter()
        .enumerate()
    {
        advance(&tx, &id, to, 1_000 + i as i64).unwrap();
        let back = load_effect(&tx, &id).unwrap().unwrap();
        assert_eq!(back.state, to, "第 {} 步推进后状态应为 {to:?}", i + 1);
    }

    // COMMITTED 是终态，表外迁移必须被拒
    let err = advance(&tx, &id, EffectState::Executing, 2_000).unwrap_err();
    assert_eq!(
        err,
        PersistError::Database(
            StateError::Illegal {
                from: EffectState::Committed,
                to: EffectState::Executing,
            }
            .to_string()
        ),
        "表外迁移应报出迁移被拒，而不是别的数据库错误"
    );

    let back = load_effect(&tx, &id).unwrap().unwrap();
    assert_eq!(back.state, EffectState::Committed, "被拒的迁移不得改动库里的状态");
    assert_eq!(back.updated_at, 1_002, "被拒的迁移不得改动时间戳");

    tx.commit().unwrap();
}

#[test]
fn advance_updates_updated_at() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    let effect = sample("e1", "k1");
    let planned_at = effect.planned_at;
    record_planned(&tx, &effect).unwrap();
    let id = EffectId::new("e1");

    advance(&tx, &id, EffectState::Authorized, 4_242).unwrap();

    let back = load_effect(&tx, &id).unwrap().unwrap();
    assert_eq!(back.updated_at, 4_242, "推进后 updated_at 应为传入值");
    assert_eq!(back.state, EffectState::Authorized);
    assert_eq!(back.planned_at, planned_at, "推进不得改动 planned_at");
}

#[test]
fn an_unknown_enum_column_value_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    // 绕过编码辅助函数直接写行：模拟旧版本写入的、或外部工具写坏的表外取值。
    // 两个枚举列各写一行，使两条解码路径都有对照片。
    for (id, effect_type, state) in [
        ("bad_state", "send_email", "not_a_state"),
        ("bad_type", "not_a_type", "planned"),
    ] {
        tx.execute(
            "INSERT INTO effect
               (id, effect_type, target, parameters, authorization,
                idempotency_key, state, planned_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            &[
                Value::text(id),
                Value::text(effect_type),
                Value::text("target"),
                Value::text("{}"),
                Value::text("auth"),
                Value::text(format!("key-{id}")),
                Value::text(state),
                Value::Int(0),
                Value::Int(0),
            ],
        )
        .unwrap();
    }

    // 表外取值必须报错，不得取默认值悄悄读回：把状态猜成 COMMITTED/FAILED
    // 正是设计上篇第 7.4 节「不猜」所禁止的
    for (id, bogus) in [("bad_state", "not_a_state"), ("bad_type", "not_a_type")] {
        let err = load_effect(&tx, &EffectId::new(id)).unwrap_err();
        match &err {
            PersistError::Database(m) => assert!(
                m.contains(bogus),
                "错误信息应指出表外取值 {bogus}，实际 {m}"
            ),
            other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }

    tx.commit().unwrap();
}

#[test]
fn advance_on_a_missing_effect_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    // 无记录时推进必须报错，不得静默成功——静默成功会让调用方以为记录已推进
    let err = advance(&tx, &EffectId::new("nope"), EffectState::Authorized, 1).unwrap_err();
    match &err {
        PersistError::Database(m) => assert!(
            m.contains("nope"),
            "错误信息应指出是哪个 id 不存在，实际 {m}"
        ),
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }
    assert_eq!(count_effects(&tx), 0, "报错后不得留下任何行");

    tx.commit().unwrap();
}
