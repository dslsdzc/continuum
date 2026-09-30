use continuum_events::audit::{verify_chain, AuditKind, GENESIS_HASH};
use continuum_events::{Event, EventType};
use continuum_persist::{Db, Value};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

#[test]
fn committed_event_is_visible() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::IntentCreated, "e1", 1, json!({"a": 1})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT event_id FROM events", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn dropped_transaction_rolls_back() {
    let (_d, db) = db();
    {
        let tx = db.begin().unwrap();
        tx.append_event(&Event::new(EventType::NodeStarted, "e1", 1, json!({})))
            .unwrap();
        // 未 commit，离开作用域时回滚
    }
    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT event_id FROM events", &[]).unwrap();
    assert!(rows.is_empty(), "未提交的事务必须回滚");
}

#[test]
fn event_and_audit_written_in_one_transaction_are_both_visible() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(
        EventType::EffectCommitted,
        "e1",
        1,
        json!({"effect": "push_branch"}),
    ))
    .unwrap();
    let rec = tx
        .append_audit(
            AuditKind::ExternalEffects,
            1,
            Value::text("push_branch"),
        )
        .unwrap();
    assert_eq!(rec.seq, 1);
    assert_eq!(rec.prev_hash, GENESIS_HASH);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(tx.query("SELECT event_id FROM events", &[]).unwrap().len(), 1);
    assert_eq!(
        tx.query("SELECT seq FROM audit_log", &[]).unwrap().len(),
        1
    );
}

#[test]
fn audit_chain_continues_across_transactions() {
    let (_d, db) = db();
    for i in 0..3 {
        let tx = db.begin().unwrap();
        tx.append_audit(
            AuditKind::ALL[i],
            100 + i as i64,
            Value::text(format!("p{i}")),
        )
        .unwrap();
        tx.commit().unwrap();
    }
    let tx = db.begin().unwrap();
    let recs = tx.audit_records().unwrap();
    assert_eq!(recs.len(), 3);
    assert_eq!(
        recs.iter().map(|r| r.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    verify_chain(&recs).expect("跨事务链应连续");
    tx.verify_audit_chain().unwrap();
}

#[test]
fn tampered_audit_row_is_detected_on_read() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_audit(AuditKind::DeviceJoin, 1, Value::text("dev-a"))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    // 篡改 occurred_at 而非 payload：payload 列在磁盘上是 JSON 文本，
    // 直接写裸字符串会让 audit_records() 在解析阶段就失败，走不到哈希校验。
    // 改一个整数列既保持该行可解析，又同样验证「改任意一列即被检出」。
    tx.execute(
        "UPDATE audit_log SET occurred_at = ?1 WHERE seq = 1",
        &[Value::Int(999)],
    )
    .unwrap();
    let err = tx.verify_audit_chain().expect_err("改写的记录必须被检出");
    assert!(err.to_string().contains("seq=1"), "实际: {err}");
}

#[test]
fn last_audit_hash_is_genesis_on_empty_chain() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    assert_eq!(tx.last_audit_hash().unwrap(), GENESIS_HASH);
    tx.commit().unwrap();
}

#[test]
fn ignorable_flag_survives_the_write_path() {
    // 该标志曾在首轮被静默丢弃（INSERT 漏列）。直接查列，锁死写路径。
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::NodeStarted, "e1", 1, json!({})).with_ignorable(true))
        .unwrap();
    tx.append_event(&Event::new(EventType::NodeStarted, "e2", 2, json!({})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT event_id, ignorable FROM events ORDER BY event_id", &[])
        .unwrap();
    let flags: Vec<(String, i64)> = rows
        .iter()
        .map(|r| {
            let id = match &r[0] {
                Value::Text(s) => s.clone(),
                other => panic!("event_id 应为文本，实际 {other:?}"),
            };
            let flag = match &r[1] {
                Value::Int(i) => *i,
                other => panic!("ignorable 应为整数，实际 {other:?}"),
            };
            (id, flag)
        })
        .collect();
    assert_eq!(flags, vec![("e1".to_owned(), 1), ("e2".to_owned(), 0)]);
}

/// 直接插一行，绕过 append_event，用于构造非法记录。
fn insert_raw_event(
    tx: &continuum_persist::Tx<'_>,
    event_id: &str,
    event_type: &str,
    schema_version: Value,
    ignorable: i64,
) {
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6)",
        &[
            Value::text(event_id),
            Value::text(event_type),
            schema_version,
            Value::Int(1),
            Value::Int(ignorable),
            Value::text("{}"),
        ],
    )
    .unwrap();
}

#[test]
fn scan_counts_skippable_records() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::NodeStarted, "e1", 1, json!({})))
        .unwrap();
    // schema_version 写成文本，解码失败但不致命
    insert_raw_event(&tx, "e2", "node.started", Value::text("one"), 0);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx.scan_event_log(&continuum_events::default_chain()).unwrap();
    assert_eq!(scan.decoded, 1);
    assert_eq!(scan.skipped_records, 1);
}

#[test]
fn scan_is_clean_when_all_records_decode() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::IntentCreated, "e1", 1, json!({})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx.scan_event_log(&continuum_events::default_chain()).unwrap();
    assert_eq!(scan.decoded, 1);
    assert_eq!(scan.skipped_records, 0);
}

#[test]
fn unknown_event_type_is_fatal_on_scan() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    insert_raw_event(&tx, "e1", "future.thing", Value::Int(1), 0);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect_err("未知且非 ignorable 的类型必须致命");
    assert!(err.to_string().contains("future.thing"), "实际: {err}");
}

#[test]
fn skippable_record_before_intent_completed_is_fatal() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    // 一条解码失败的记录
    insert_raw_event(&tx, "e1", "node.started", Value::text("one"), 0);
    // 其后存在收口事件
    tx.append_event(&Event::new(EventType::IntentCompleted, "e2", 2, json!({})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect_err("收口前的记录不可读时必须致命");
    assert!(err.to_string().contains("intent.completed"), "实际: {err}");
}

#[test]
fn skippable_record_after_intent_completed_is_only_counted() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::IntentCompleted, "e1", 1, json!({})))
        .unwrap();
    insert_raw_event(&tx, "e2", "node.started", Value::text("one"), 0);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect("收口之后的坏记录不升级为致命");
    assert_eq!(scan.skipped_records, 1);
}

#[test]
fn unreadable_payload_is_skippable() {
    // payload 列不是合法 JSON：应为可跳过，而不是被计为已解码
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, 0, ?5)",
        &[
            Value::text("e1"),
            Value::text("node.started"),
            Value::Int(1),
            Value::Int(1),
            Value::text("not json at all"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect("坏 payload 不应致命");
    assert_eq!(scan.decoded, 0, "坏 payload 不得被计为已解码");
    assert_eq!(scan.skipped_records, 1);
}

#[test]
fn unreadable_payload_on_intent_completed_is_fatal() {
    // 收口行本身不可读：跳过它会使收口依据不成立，必须致命
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, 0, ?5)",
        &[
            Value::text("e1"),
            Value::text("intent.completed"),
            Value::Int(1),
            Value::Int(1),
            Value::text("not json at all"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect_err("不可读的收口行必须致命");
    assert!(err.to_string().contains("intent.completed"), "实际: {err}");
}

#[test]
fn unknown_type_with_broken_payload_is_still_fatal() {
    // 未知类型与坏 payload 同时出现时，不得因为 payload 先被判为
    // 可跳过而绕过 §5.2 的 fail-closed 规则。
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, 0, ?5)",
        &[
            Value::text("e1"),
            Value::text("future.thing"),
            Value::Int(1),
            Value::Int(1),
            Value::text("not json at all"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect_err("未知类型 + 坏 payload 必须致命");
    assert!(err.to_string().contains("future.thing"), "实际: {err}");
}

#[test]
fn invalid_schema_version_is_fatal() {
    // 0 与 99 同属「可读但不在链中」，判定应一致为致命；
    // 只有「列不是整数」才归可跳过。
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, 0, ?5)",
        &[
            Value::text("e1"),
            Value::text("node.started"),
            Value::Int(0),
            Value::Int(1),
            Value::text("{}"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log(&continuum_events::default_chain())
        .expect_err("schema_version = 0 必须致命");
    assert!(err.to_string().contains("schema_version"), "实际: {err}");
}
