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

/// 审计表里的 (kind, payload) 各一行，按 seq 排序。
fn audit_rows(tx: &continuum_persist::Tx<'_>) -> Vec<(String, serde_json::Value)> {
    tx.query("SELECT kind, payload FROM audit_log ORDER BY seq", &[])
        .unwrap()
        .iter()
        .map(|row| {
            let kind = text_of(&row[0]);
            let raw = text_of(&row[1]);
            let payload = serde_json::from_str(&raw)
                .unwrap_or_else(|e| panic!("审计 payload 应为 JSON，实际 {raw}: {e}"));
            (kind, payload)
        })
        .collect()
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
    // 被拒的写入不落审计：否则审计里会有一条「登记过」而 effect 表里没有的效应
    assert_eq!(audit_rows(&tx).len(), 1, "被拒的登记不应写审计");

    // 挡住的是键，不是全部写入：换个键仍能写
    record_planned(&tx, &sample("e3", "k2")).unwrap();
    assert_eq!(count_effects(&tx), 2);
    assert_eq!(audit_rows(&tx).len(), 2);

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

    // 落库编码的形态：遍历**全部**单元格，不逐索引列举——逐索引列举会漏格子
    // （本用例早先的版本就漏了 `rows[0][1]`）。上面四条精确断言逐格钉住取值，
    // 这条另管「形态合法且非 Debug 表示」，将来往本用例加行时也不会漏检。
    for cell in rows.iter().flatten() {
        let raw = text_of(cell);
        assert!(
            !raw.is_empty() && raw.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "枚举列的落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {raw}"
        );
    }

    tx.commit().unwrap();
}

/// `EffectType` 的编码往返：遍历 [`EffectType::ALL`] 的每个变体，并断言名单的数目。
///
/// **本用例守的是解码侧**：`parse` 从 `&str` 出发，编译器点不出漏掉的臂，只有遍历
/// 能发现「某个**已在名单里**的变体解不回去」。编码侧不靠本用例——`as_str` 的 match
/// 穷尽且无通配臂，加变体时编译失败。
///
/// **数目断言比它看起来窄**，据实写明：它只在「名单的长度与这里的字面量不一致」时红
/// （有人增删了 `ALL` 的项而没同步本行）。它**发现不了**「给枚举加了变体却没加进
/// `ALL`」——那种情形下长度不变，往返也走不到新变体，本用例与它一起照过。那一种由
/// `as_str` 的编译失败拦下：报错点就在 `EffectType` 的定义处，而 `ALL` 与 `as_str`
/// 同在一个 `impl` 块里紧邻。
///
/// 编码现在只在 `EffectType::as_str` / `parse` 上（库列、审计 payload、策略条件三处
/// 共用），故本用例同时是那三处的前提：`parse` 漏掉某个变体，条件里的对应取值就会
/// 在解析期报错——这里先红。
#[test]
fn every_effect_type_round_trips_through_its_encoding() {
    assert_eq!(EffectType::ALL.len(), 6, "EffectType 的名单与变体数不符");

    for effect_type in EffectType::ALL {
        let encoded = effect_type.as_str();
        assert_eq!(
            EffectType::parse(encoded),
            Some(effect_type),
            "{encoded} 应解回 {effect_type:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }

    // 表外取值一律 None：不取默认类型——取默认会让策略表漏判。
    for bogus in ["", "charge ", "Charge", "send email", "not_a_type", "deploy_"] {
        assert_eq!(EffectType::parse(bogus), None, "{bogus} 不应解出任何类型");
    }
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
    // 被拒的迁移也不入审计：4 = 1 次登记 + 3 次成功的推进
    assert_eq!(audit_rows(&tx).len(), 4, "被拒的迁移不应写审计");

    tx.commit().unwrap();
}

#[test]
fn every_state_change_appends_one_audit_record() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    record_planned(&tx, &sample("e1", "k1")).unwrap();
    assert_eq!(audit_rows(&tx).len(), 1, "登记一条效应应写一条审计");

    let id = EffectId::new("e1");
    for (i, to) in [EffectState::Authorized, EffectState::Executing, EffectState::Committed]
        .into_iter()
        .enumerate()
    {
        let before = audit_rows(&tx).len();
        advance(&tx, &id, to, 1_000 + i as i64).unwrap();
        assert_eq!(audit_rows(&tx).len(), before + 1, "每次推进应各加一条审计");
    }

    let rows = audit_rows(&tx);
    assert_eq!(rows.len(), 4);
    // 归类取 ExternalEffects：上篇把不可回滚的外部操作归它
    for (kind, _) in &rows {
        assert_eq!(kind, "external effects", "审计归类应为 external effects");
    }
    // 登记那条记的是被登记的效应本身
    assert_eq!(rows[0].1["effect_id"], "e1");
    assert_eq!(rows[0].1["effect_type"], "push_branch");
    assert_eq!(rows[0].1["idempotency_key"], "k1");
    assert_eq!(rows[0].1["state"], "planned");
    // 推进那条记的是两端状态，取值与 effect 表的列编码同源
    assert_eq!(rows[3].1["effect_id"], "e1");
    assert_eq!(rows[3].1["from"], "executing");
    assert_eq!(rows[3].1["to"], "committed");

    tx.commit().unwrap();
}

#[test]
fn the_audit_and_the_state_change_are_rolled_back_together() {
    let (_dir, db) = db();
    {
        // 只开事务不提交：`Tx` 被丢弃时整笔回滚
        let tx = db.begin().unwrap();
        record_planned(&tx, &sample("e1", "k1")).unwrap();
        advance(&tx, &EffectId::new("e1"), EffectState::Authorized, 11).unwrap();
        assert_eq!(count_effects(&tx), 1);
        assert_eq!(audit_rows(&tx).len(), 2);
    }

    let tx = db.begin().unwrap();
    assert_eq!(count_effects(&tx), 0, "回滚后 effect 表不应留下行");
    assert_eq!(
        audit_rows(&tx).len(),
        0,
        "回滚后审计不应留下行——留下即说明它没和状态变更走同一个事务"
    );
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
    assert_eq!(audit_rows(&tx).len(), 0, "报错后不得留下审计");

    tx.commit().unwrap();
}
