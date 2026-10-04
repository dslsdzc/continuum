//! 强制点 (1)：`authorize` 与 `AuthorizedTool`（设计 §3.4、§8）。
//!
//! 五个失败方向各有照片，且**每条都断言是哪一种 `Err`**（纪律 3）：
//! `UnknownTool` / `Persist` / `UndeclaredCapability` / `Expired` / `MissingCapability`；
//! 成功侧另有对照臂。审计的两侧都钉：成功**恰一条**、拒绝**零条**。
//!
//! 夹具与 `tests/persist.rs` 同形（同一个 `p3_capability_migrations()`，建库后必须
//! `migrate()`）。

use continuum_capability::{
    Capability, CapabilityError, CapabilityKind, EmailAction, FsAction, GitAction, Tool, ToolId,
    ToolProfile, Trust, authorize, mint, p3_capability_migrations, save_tool,
};
use continuum_persist::{Db, Migration, PersistError, Tx, Value, builtin_migrations};
use serde_json::{Value as JsonValue, json};

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p3_capability_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条只关心 `required_capabilities` 的登记项：其余字段取合法但不变的值
/// （本文件的用例都不读它们）。
fn profile(id: &str, required: Vec<CapabilityKind>) -> ToolProfile {
    ToolProfile::new(
        Tool::new(
            ToolId::new(id),
            "1".to_owned(),
            json!({}),
            json!({}),
            required,
            None,
            true,
        ),
        None,
        None,
        Trust,
    )
}

fn cap(kind: CapabilityKind, scope: &str, expiry: i64) -> Capability {
    mint(kind, scope.to_owned(), expiry).expect("用例里的作用域恒非空")
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        other => panic!("列应为整数，实际 {other:?}"),
    }
}

fn audit_count(tx: &Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM audit_log", &[]).unwrap();
    int_of(&rows[0][0])
}

/// 绕过编码辅助函数直接写行：模拟被写坏的表外取值。只写坏
/// `required_capabilities` 一列——`load_tool` 在解码它时必然失败。
fn insert_bad_capabilities(tx: &Tx<'_>, id: &str, raw: &str) {
    tx.execute(
        "INSERT INTO tool
           (id, version, input_schema, output_schema, required_capabilities,
            effect_class, deterministic, cost, latency, trust)
         VALUES (?1, '1', '{}', '{}', ?2, NULL, 0, NULL, NULL, '')",
        &[Value::text(id), Value::text(raw)],
    )
    .unwrap();
}

/// 成功授权：恰一条审计；`kind` **列**是手写字面量 `"capability grants"`；
/// payload 含工具 id 与已获准能力的 kind / scope。
///
/// 判据刻意**不是**把 `AuditRecord.kind` 与 `AuditKind::CapabilityGrants` 比一圈——
/// 那样只证明枚举等于自己，钉不住落库编码。这里读的是列里的原始文本。
#[test]
fn a_successful_authorization_writes_one_capability_grants_audit_row() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile(
            "t1",
            vec![
                CapabilityKind::Filesystem(FsAction::Read),
                CapabilityKind::Git(GitAction::Push),
            ],
        ),
    )
    .unwrap();

    let now = 1_000;
    let presented = [
        cap(CapabilityKind::Filesystem(FsAction::Read), "/project", now + 10),
        cap(CapabilityKind::Git(GitAction::Push), "origin/main", now + 20),
    ];

    let authorized = authorize(&tx, &ToolId::new("t1"), &presented, now).unwrap();

    // 带出的是工具 id 与刚获准的那几枚能力（顺序与内容都照出示的样子）
    assert_eq!(authorized.tool_id(), &ToolId::new("t1"));
    assert_eq!(
        authorized.granted(),
        presented.as_slice(),
        "已获准的能力应与出示的逐枚相同"
    );

    // 恰一条审计
    assert_eq!(audit_count(&tx), 1, "成功授权恰写一条审计");
    let rows = tx
        .query("SELECT kind, occurred_at, payload FROM audit_log", &[])
        .unwrap();
    // kind **列**是手写字面量，不是枚举转一圈
    assert_eq!(
        text_of(&rows[0][0]),
        "capability grants",
        "审计 kind 列的落库文本应为手写字面量"
    );
    assert_eq!(int_of(&rows[0][1]), now, "审计时刻应取调用方给的 now");

    // payload 是 JSON 对象：工具 id + 每枚已获准能力的 kind 与 scope
    let payload: JsonValue = serde_json::from_str(&text_of(&rows[0][2])).unwrap();
    assert_eq!(payload["tool_id"], json!("t1"));
    let capabilities = payload["capabilities"].as_array().expect("capabilities 应为数组");
    assert_eq!(capabilities.len(), 2);
    assert_eq!(capabilities[0]["kind"], json!("filesystem_read"));
    assert_eq!(capabilities[0]["scope"], json!("/project"));
    assert_eq!(capabilities[1]["kind"], json!("git_push"));
    assert_eq!(capabilities[1]["scope"], json!("origin/main"));

    // 这条记录进的是同一条哈希链
    tx.verify_audit_chain().unwrap();

    tx.commit().unwrap();
}

/// 声明集为空、出示集也为空：这是一次**成功**授权，不是拒绝，且仍写一条审计
/// （payload 的能力列表为空）。
#[test]
fn a_tool_that_declares_nothing_authorizes_an_empty_presentation() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(&tx, &profile("t1", vec![])).unwrap();

    let authorized = authorize(&tx, &ToolId::new("t1"), &[], 7).unwrap();
    assert!(authorized.granted().is_empty());
    assert_eq!(audit_count(&tx), 1, "空声明的成功授权也应写一条审计");
    let rows = tx.query("SELECT payload FROM audit_log", &[]).unwrap();
    let payload: JsonValue = serde_json::from_str(&text_of(&rows[0][0])).unwrap();
    assert_eq!(payload["tool_id"], json!("t1"));
    assert_eq!(payload["capabilities"], json!([]));

    tx.commit().unwrap();
}

/// 已获准的能力**按出示顺序照录**：同 kind 的两枚都在，不重排、不去重。
///
/// 出示顺序刻意既非字母序也非声明序，且含重复 kind——去重或排序的实现在本用例上会红。
#[test]
fn granted_capabilities_keep_the_presentation_order_and_duplicates() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile(
            "t1",
            vec![
                CapabilityKind::Filesystem(FsAction::Read),
                CapabilityKind::Git(GitAction::Push),
            ],
        ),
    )
    .unwrap();

    let now = 1_000;
    let presented = [
        cap(CapabilityKind::Git(GitAction::Push), "origin/dev", now + 1),
        cap(CapabilityKind::Git(GitAction::Push), "origin/main", now + 2),
        cap(CapabilityKind::Filesystem(FsAction::Read), "/project", now + 3),
    ];
    let authorized = authorize(&tx, &ToolId::new("t1"), &presented, now).unwrap();
    assert_eq!(authorized.granted(), presented.as_slice());

    // 审计里同样是三枚、同样顺序
    let rows = tx.query("SELECT payload FROM audit_log", &[]).unwrap();
    let payload: JsonValue = serde_json::from_str(&text_of(&rows[0][0])).unwrap();
    let scopes: Vec<&str> = payload["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["scope"].as_str().unwrap())
        .collect();
    assert_eq!(scopes, vec!["origin/dev", "origin/main", "/project"]);

    tx.commit().unwrap();
}

/// 声明了没给：两个方向里的第一个。报的是**声明表里第一条**缺的能力。
#[test]
fn a_required_capability_that_is_missing_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    let declared = vec![
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Git(GitAction::Push),
    ];
    save_tool(&tx, &profile("t1", declared.clone())).unwrap();
    let now = 1_000;

    // 枚不出示：两枚都缺，报第一条
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &[], now).unwrap_err(),
        CapabilityError::MissingCapability {
            kind: CapabilityKind::Filesystem(FsAction::Read),
        },
        "两枚都缺时报声明表里第一条"
    );

    // 只给一半：报缺的那一枚
    let only_push = [cap(CapabilityKind::Git(GitAction::Push), "origin/main", now + 1)];
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &only_push, now).unwrap_err(),
        CapabilityError::MissingCapability {
            kind: CapabilityKind::Filesystem(FsAction::Read),
        }
    );

    // 对照臂：给全了就过；上面两次被拒不得留下审计
    let both = [
        cap(CapabilityKind::Filesystem(FsAction::Read), "/p", now + 1),
        cap(CapabilityKind::Git(GitAction::Push), "origin/main", now + 1),
    ];
    assert!(authorize(&tx, &ToolId::new("t1"), &both, now).is_ok());
    assert_eq!(audit_count(&tx), 1, "只有成功的那次写了审计");

    tx.commit().unwrap();
}

/// 给了没声明：两个方向里的第二个——超范围的能力同样不许。
#[test]
fn a_presented_capability_that_the_tool_did_not_declare_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile("t1", vec![CapabilityKind::Filesystem(FsAction::Read)]),
    )
    .unwrap();
    let now = 1_000;

    let over = [
        cap(CapabilityKind::Filesystem(FsAction::Read), "/p", now + 1),
        cap(CapabilityKind::Git(GitAction::Push), "origin/main", now + 1),
    ];
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &over, now).unwrap_err(),
        CapabilityError::UndeclaredCapability {
            kind: CapabilityKind::Git(GitAction::Push),
        }
    );

    // 对照臂：只出示声明过的那一枚即通过
    assert!(authorize(&tx, &ToolId::new("t1"), &over[..1], now).is_ok());
    assert_eq!(audit_count(&tx), 1, "只有成功的那次写了审计");

    tx.commit().unwrap();
}

/// 检查次序：**先查出示集，再查声明集**。
///
/// 本用例里工具声明的两枚一枚都没给，同时出示了一枚没声明的：先查出示集则报
/// `UndeclaredCapability`，先查声明集则报 `MissingCapability`——两版只有一版能过。
#[test]
fn the_presented_set_is_checked_before_the_declared_set() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile(
            "t1",
            vec![
                CapabilityKind::Git(GitAction::Push),
                CapabilityKind::Filesystem(FsAction::Read),
            ],
        ),
    )
    .unwrap();

    let stray = [cap(CapabilityKind::Email(EmailAction::Send), "a@b", 10_000)];
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &stray, 1_000).unwrap_err(),
        CapabilityError::UndeclaredCapability {
            kind: CapabilityKind::Email(EmailAction::Send),
        },
        "缺声明与超范围同时出现时，报的是出示侧那一种"
    );
    assert_eq!(audit_count(&tx), 0);

    tx.commit().unwrap();
}

/// 出示集里**同一枚**能力既没声明又已过期：报 `UndeclaredCapability`。
///
/// 单枚能力内部的次序也钉住：声明判定在看它是否失效之前。
#[test]
fn an_undeclared_capability_is_reported_even_when_it_has_expired() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(&tx, &profile("t1", vec![])).unwrap();

    let stray = [cap(CapabilityKind::Git(GitAction::Push), "origin/main", 500)];
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &stray, 501).unwrap_err(),
        CapabilityError::UndeclaredCapability {
            kind: CapabilityKind::Git(GitAction::Push),
        }
    );

    tx.commit().unwrap();
}

/// 到期边界三格：未到期通过、**恰好到期**被拒、已过期被拒。
///
/// 判据直接复用 `Capability::is_valid_at`——本用例不同时断言 `is_valid_at` 的行为，
/// 那条路径的照片在 `tests/capability.rs`；本用例钉的是 `authorize` 走到了它，
/// 且过期的那枚**不得**被当成「缺」（否则报的是 `MissingCapability`）。
#[test]
fn an_expired_presented_capability_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile("t1", vec![CapabilityKind::Git(GitAction::Push)]),
    )
    .unwrap();
    let expiry = 500;
    let pushed = [cap(CapabilityKind::Git(GitAction::Push), "origin/main", expiry)];

    // 未到期：通过
    assert!(authorize(&tx, &ToolId::new("t1"), &pushed, expiry - 1).is_ok());

    // 恰好到期：已失效
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &pushed, expiry).unwrap_err(),
        CapabilityError::Expired {
            expiry,
            now: expiry,
        }
    );
    // 已过期
    assert_eq!(
        authorize(&tx, &ToolId::new("t1"), &pushed, expiry + 1).unwrap_err(),
        CapabilityError::Expired {
            expiry,
            now: expiry + 1,
        }
    );

    assert_eq!(audit_count(&tx), 1, "只有未到期那次写了审计");

    tx.commit().unwrap();
}

/// 不存在的 id 是 `UnknownTool`，**不是**「没有声明能力」——不得被悄悄放行。
#[test]
fn an_unknown_tool_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert_eq!(
        authorize(&tx, &ToolId::new("nope"), &[], 0).unwrap_err(),
        CapabilityError::UnknownTool {
            id: ToolId::new("nope"),
        }
    );
    assert_eq!(audit_count(&tx), 0);

    tx.commit().unwrap();
}

/// 读取登记项本身失败（列里是表外取值）时，错误经 `CapabilityError::Persist` 透出，
/// 不被吞成 `UnknownTool` 或「无需能力」。
#[test]
fn a_persist_failure_while_loading_the_tool_is_not_swallowed() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    insert_bad_capabilities(&tx, "broken", r#"["git_push","not_a_kind"]"#);

    match authorize(&tx, &ToolId::new("broken"), &[], 0).unwrap_err() {
        CapabilityError::Persist(PersistError::Database(m)) => assert!(
            m.contains("not_a_kind"),
            "错误信息应指出表外取值，实际 {m}"
        ),
        other => panic!("应为 CapabilityError::Persist(PersistError::Database(_))，实际 {other:?}"),
    }
    assert_eq!(audit_count(&tx), 0, "读取失败不得留下审计");

    tx.commit().unwrap();
}

/// `Persist` 的**第二个产生方**：写审计那次往返失败，同样不许吞。
///
/// 只能在同一事务里丢掉审计表来确定性制造——表缺失时 `append_audit` 必然失败。
/// 此时工具、出示集、声明集都合法，失败只可能来自那次写。
#[test]
fn an_audit_write_failure_is_reported_as_persist() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(&tx, &profile("t1", vec![])).unwrap();
    tx.execute("DROP TABLE audit_log", &[]).unwrap();

    match authorize(&tx, &ToolId::new("t1"), &[], 7).unwrap_err() {
        // 报的是 Persist，且内层消息点出**审计**这一次往返（不是「工具不存在」）
        CapabilityError::Persist(PersistError::Database(m)) => assert!(
            m.contains("audit_log"),
            "错误信息应指出审计写入失败，实际 {m}"
        ),
        other => panic!("应为 CapabilityError::Persist(PersistError::Database(_))，实际 {other:?}"),
    }

    tx.commit().unwrap();
}

/// **方向相反的对照臂**：拒绝一律不写审计（五种失败各走一遍）。
#[test]
fn a_rejected_authorization_writes_no_audit_row() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile("t1", vec![CapabilityKind::Git(GitAction::Push)]),
    )
    .unwrap();
    insert_bad_capabilities(&tx, "broken", "not json");

    // 缺
    assert!(matches!(
        authorize(&tx, &ToolId::new("t1"), &[], 1).unwrap_err(),
        CapabilityError::MissingCapability { .. }
    ));
    assert_eq!(audit_count(&tx), 0, "MissingCapability 不得写审计");

    // 超范围
    let stray = [cap(CapabilityKind::Email(EmailAction::Send), "a@b", 10)];
    assert!(matches!(
        authorize(&tx, &ToolId::new("t1"), &stray, 1).unwrap_err(),
        CapabilityError::UndeclaredCapability { .. }
    ));
    assert_eq!(audit_count(&tx), 0, "UndeclaredCapability 不得写审计");

    // 过期
    let expired = [cap(CapabilityKind::Git(GitAction::Push), "origin/main", 5)];
    assert!(matches!(
        authorize(&tx, &ToolId::new("t1"), &expired, 5).unwrap_err(),
        CapabilityError::Expired { .. }
    ));
    assert_eq!(audit_count(&tx), 0, "Expired 不得写审计");

    // 未知工具
    assert!(matches!(
        authorize(&tx, &ToolId::new("nope"), &[], 1).unwrap_err(),
        CapabilityError::UnknownTool { .. }
    ));
    assert_eq!(audit_count(&tx), 0, "UnknownTool 不得写审计");

    // 读取失败
    assert!(matches!(
        authorize(&tx, &ToolId::new("broken"), &[], 1).unwrap_err(),
        CapabilityError::Persist(_)
    ));
    assert_eq!(audit_count(&tx), 0, "Persist 不得写审计");

    tx.commit().unwrap();
}
