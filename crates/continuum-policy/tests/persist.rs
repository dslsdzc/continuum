//! `policy` 表的落库与读回（设计下篇第 8 节）。
//!
//! 本文件的夹具 `db()` 单独成立：它只依赖 `p2_policy_migrations` 与 `Db`，
//! 建库后必须 `migrate()` —— 漏掉那一行时各用例会一起挂在 `no such table`，
//! 而报错位置指向被测函数，容易误判成实现的问题。

use continuum_policy::{
    load_policies, p2_policy_migrations, save_policy, Condition, Decision, Level, Policy, Scope,
};
use continuum_persist::{builtin_migrations, Db, Migration, PersistError, Tx, Value};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p2_policy_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn policy(level: Level, condition: Condition, decision: Decision, scope: Scope) -> Policy {
    Policy {
        level,
        condition,
        decision,
        scope,
    }
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

fn count_policies(tx: &Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM policy", &[]).unwrap();
    match rows[0][0] {
        Value::Int(i) => i,
        ref other => panic!("COUNT 应为整数，实际 {other:?}"),
    }
}

/// 一条规则的整结构往返。两条规则取不同层级、不同决策、不同作用域，
/// 条件一条是裸谓词、一条是两项合取（且含 `gte`）。
///
/// 断言「条件相同」比的是 [`Condition`] 的相等，**不是 JSON 字节**：
/// `parse` 会把裸谓词归一化成一元合取，写回时也是合取形式，两者字面不同而语义相同
/// （`tests/condition.rs` 的 `a_bare_predicate_equals_a_one_term_conjunction` 钉住这一等价）。
#[test]
fn policy_round_trips() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let bare = Condition::parse(&json!({"fact": "effect_type", "eq": "charge"}))
        .expect("裸谓词应可解析");
    let conjunction = Condition::parse(&json!({"all": [
        {"fact": "task_class", "eq": "build"},
        {"fact": "duration_ms", "gte": 1000},
    ]}))
    .expect("两项合取应可解析");

    let user = policy(
        Level::UserPersistent,
        bare.clone(),
        Decision::RequireApproval,
        Scope::User,
    );
    let project = policy(
        Level::Project,
        conjunction.clone(),
        Decision::Deny,
        Scope::Project,
    );
    save_policy(&tx, "p3", &user).unwrap();
    save_policy(&tx, "p4", &project).unwrap();

    let back = load_policies(&tx).unwrap();
    assert_eq!(back.len(), 2, "存了两条规则，读回应是两条");
    assert_eq!(back[0], user, "读回的四个字段应与写入值相同");
    assert_eq!(back[1], project);
    // 逐字段再钉一次：整结构比对在「四个字段一起写串」这类缺陷下仍会变红，
    // 但单列名指认不出是哪个字段错，故两处都留
    assert_eq!(back[0].level, Level::UserPersistent);
    assert_eq!(back[0].decision, Decision::RequireApproval);
    assert_eq!(back[0].scope, Scope::User);
    assert_eq!(back[0].condition, bare, "条件应按 Condition 的相等比，不比 JSON 字节");
    assert_eq!(back[1].condition, conjunction);

    // 空合取也要走通往返：它是「对所有情形都适用」的表达，与空表读回无从区分，
    // 故必须与「没存进去」分开断言
    let all_cases = Condition::parse(&json!({"all": []})).expect("空合取应可解析");
    let runtime_default = policy(
        Level::RuntimeDefault,
        all_cases.clone(),
        Decision::Allow,
        Scope::User,
    );
    save_policy(&tx, "p5", &runtime_default).unwrap();
    let back = load_policies(&tx).unwrap();
    assert_eq!(back.len(), 3);
    assert_eq!(back[2].condition, all_cases);

    tx.commit().unwrap();
}

/// 三个枚举列的落库编码：小写、多词以 `_` 连接，**不是 `Debug` 表示**。
///
/// `level` 是带数字序的枚举（`SystemSafety = 1` … `ModelSuggestion = 6`），
/// 落库仍取名字而非数字：数字会让「加一级」变成破坏性变更——在中间插入一层时，
/// 既有行的数字全部要改写，而名字不会。
#[test]
fn enum_columns_use_the_lowercase_encoding() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let any = Condition::parse(&json!({"all": []})).unwrap();
    for (id, level, decision, scope) in [
        (
            "p1",
            Level::UserPersistent,
            Decision::RequireApproval,
            Scope::Project,
        ),
        (
            "p2",
            Level::RuntimeDefault,
            Decision::Allow,
            Scope::User,
        ),
        ("p3", Level::SystemSafety, Decision::Deny, Scope::User),
    ] {
        save_policy(&tx, id, &policy(level, any.clone(), decision, scope)).unwrap();
    }

    let rows = tx
        .query(
            "SELECT level, decision, scope FROM policy ORDER BY id",
            &[],
        )
        .unwrap();
    assert_eq!(text_of(&rows[0][0]), "user_persistent");
    assert_eq!(text_of(&rows[0][1]), "require_approval");
    assert_eq!(text_of(&rows[0][2]), "project");
    assert_eq!(text_of(&rows[1][0]), "runtime_default");
    assert_eq!(text_of(&rows[1][1]), "allow");
    assert_eq!(text_of(&rows[1][2]), "user");
    assert_eq!(text_of(&rows[2][0]), "system_safety");
    assert_eq!(text_of(&rows[2][1]), "deny");

    // 落库编码的形态：遍历**全部**单元格，不逐索引列举——逐索引列举会漏格子。
    // 上面八条精确断言逐格钉住取值（`level` 列是数字时这里也会红），这条另管
    // 「形态合法且非 Debug 表示」，将来往本用例加行时也不会漏检。
    for cell in rows.iter().flatten() {
        let raw = text_of(cell);
        assert!(
            !raw.is_empty() && raw.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "枚举列的落库编码应为小写、多词以 _ 连接（Debug 表示含大写、数字序是十进制），实际 {raw}"
        );
    }

    tx.commit().unwrap();
}

/// `Level` 的编码往返：遍历 [`Level::ALL`] 的每个变体，并断言名单的数目。
///
/// **本用例守的是解码侧**：`parse` 从 `&str` 出发，编译器点不出漏掉的臂，只有遍历
/// 能发现「某个**已在名单里**的变体解不回去」。编码侧不靠本用例——`as_str` 的 match
/// 穷尽且无通配臂，加变体时编译失败。
///
/// 数目断言比它看起来窄，据实写明：它只在「名单长度与这里的字面量不一致」时红。
/// 「给枚举加了变体却没加进 `ALL`」由 `as_str` 的编译失败拦下，报错点就在 `Level`
/// 的定义处，而 `ALL` 与 `as_str` 同在一个 `impl` 块里紧邻。
#[test]
fn every_level_round_trips_through_its_encoding() {
    assert_eq!(Level::ALL.len(), 6, "Level 的名单与变体数不符");

    for level in Level::ALL {
        let encoded = level.as_str();
        assert_eq!(Level::parse(encoded), Some(level), "{encoded} 应解回 {level:?}");
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }

    // 表外取值一律 None：不取默认层级——取默认会让优先级判错。
    for bogus in ["", "user persistent", "UserPersistent", "3", "system_safety ", "nope"] {
        assert_eq!(Level::parse(bogus), None, "{bogus} 不应解出任何层级");
    }
}

/// `Decision` 的编码往返，形态与 [`every_level_round_trips_through_its_encoding`] 同。
#[test]
fn every_decision_round_trips_through_its_encoding() {
    assert_eq!(Decision::ALL.len(), 3, "Decision 的名单与变体数不符");

    for decision in Decision::ALL {
        let encoded = decision.as_str();
        assert_eq!(
            Decision::parse(encoded),
            Some(decision),
            "{encoded} 应解回 {decision:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }

    for bogus in ["", "require approval", "RequireApproval", "nope"] {
        assert_eq!(Decision::parse(bogus), None, "{bogus} 不应解出任何决策");
    }
}

/// `Scope` 的编码往返，形态同上。
#[test]
fn every_scope_round_trips_through_its_encoding() {
    assert_eq!(Scope::ALL.len(), 2, "Scope 的名单与变体数不符");

    for scope in Scope::ALL {
        let encoded = scope.as_str();
        assert_eq!(Scope::parse(encoded), Some(scope), "{encoded} 应解回 {scope:?}");
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }

    for bogus in ["", "User", "project policy", "nope"] {
        assert_eq!(Scope::parse(bogus), None, "{bogus} 不应解出任何作用域");
    }
}

/// 读回侧的条件要**重新经 `Condition::parse`**：库里被手工改坏的条件下不该静默
/// 变成「永不匹配」——那会让人以为策略生效了而实际没有（设计下篇第 5.1 节）。
///
/// 绕过写入函数直接插行，模拟外部工具写坏、或旧版本写下的表内容。
/// 断言到**具体的错误来源**（纪律 3）：`PersistError` 没有独立的条件解析变体，
/// 故按 `effect` 表那条解码用例的成法，在 `Database` 变体内按消息指认是
/// `PolicyError` 的哪一个变体、指名了哪个事实。
#[test]
fn a_condition_that_is_not_a_valid_predicate_cannot_be_loaded() {
    let (_dir, db) = db();

    // 三种写错各占一个作用域：**不能同时开两个 `Tx`**——连接由 `Db` 内的互斥量把守，
    // 第二个 `begin` 会一直等第一个释放（`Tx` 无超时）。每个作用域结束时不提交，
    // `Tx` 被丢弃即回滚，故下一个作用域看到的是空表，各案的坏行不会互相遮蔽。

    // 未知事实名：`Condition::parse` 报 `UnknownFact { name: "nope" }`
    {
        let tx = db.begin().unwrap();
        insert_raw(&tx, "bad_fact", "user_persistent", "user", r#"{"fact":"nope","eq":"x"}"#, "allow");

        let err = load_policies(&tx).unwrap_err();
        match &err {
            PersistError::Database(m) => assert!(
                m.contains("未知事实名：nope"),
                "应报出条件解析的具体原因（未知事实名 nope），实际 {m}"
            ),
            other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }

    // 结构合法但取值不在封闭集合内：另一类「写错」，同样是读回期的 `Err` 而非
    // 「永不匹配」。
    {
        let tx = db.begin().unwrap();
        insert_raw(&tx, "bad_value", "project", "project", r#"{"fact":"effect_type","eq":"charg"}"#, "deny");

        let err = load_policies(&tx).unwrap_err();
        match &err {
            PersistError::Database(m) => assert!(
                m.contains("取值不在封闭集合内：charg"),
                "应报出取值不在封闭集合内，实际 {m}"
            ),
            other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }

    // 连 JSON 都不是：在 `Condition::parse` 之前就断。这条路径与上面两条不同，
    // 若不单独钉住，一个「先 `unwrap_or(Value::Null)` 再解析」的实现会把它降级成
    // 「结构不符」而仍然报错——报错位置对了，来源却错，调用方无从分辨。
    {
        let tx = db.begin().unwrap();
        insert_raw(&tx, "not_json", "project", "project", "{ 这不是 JSON", "deny");

        let err = load_policies(&tx).unwrap_err();
        match &err {
            PersistError::Database(m) => assert!(
                m.contains("condition") && m.contains("not_json"),
                "应指出是哪个 id 的 condition 列不可读，实际 {m}"
            ),
            other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }
}

/// 同 id 再存一次必须被拒，**且库里那一行仍是原来的内容**。
///
/// 裸 `INSERT`（不 `OR REPLACE`）是兄弟表 `effect` 已确立的成法：判据在库层，
/// 不靠调用方自觉。这里比 `effect` 更重——`OR REPLACE` 会**静默覆盖用户自己定的
/// 持久策略**，而策略是被安全语义依赖的输入。
#[test]
fn saving_the_same_id_twice_is_rejected_and_leaves_the_first_row_intact() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let strict = Condition::parse(&json!({"fact": "effect_type", "eq": "charge"})).unwrap();
    let loose = Condition::parse(&json!({"all": []})).unwrap();
    let first = policy(Level::UserPersistent, strict, Decision::Deny, Scope::User);
    save_policy(&tx, "p1", &first).unwrap();

    let second = policy(Level::Project, loose, Decision::Allow, Scope::Project);
    let err = save_policy(&tx, "p1", &second).unwrap_err();
    match &err {
        PersistError::Database(m) => assert!(
            m.contains("UNIQUE constraint failed: policy.id"),
            "应为 id 上的唯一键冲突，实际 {m}"
        ),
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }

    assert_eq!(count_policies(&tx), 1, "同 id 被拒后库里应仍只有一行");
    // 「仍是原来的内容」必须直接查列，不能只比 `load_policies` 的结果：
    // 那会同时经过解码，把「写覆盖了」与「读错了」两种缺陷混在一起
    let rows = tx
        .query("SELECT level, decision, scope FROM policy WHERE id = 'p1'", &[])
        .unwrap();
    assert_eq!(text_of(&rows[0][0]), "user_persistent", "被拒的写入不得改动 level");
    assert_eq!(text_of(&rows[0][1]), "deny", "被拒的写入不得改动 decision");
    assert_eq!(text_of(&rows[0][2]), "user", "被拒的写入不得改动 scope");
    let back = load_policies(&tx).unwrap();
    assert_eq!(back, vec![first], "读回的那条应与第一次写入的相同");

    // 挡住的是 id，不是全部写入：换个 id 仍能写
    save_policy(&tx, "p2", &second).unwrap();
    assert_eq!(count_policies(&tx), 2);

    tx.commit().unwrap();
}

/// 绕过 `save_policy` 直接插行，模拟外部工具写坏、或旧版本写下的表内容。
fn insert_raw(tx: &Tx<'_>, id: &str, level: &str, scope: &str, condition: &str, decision: &str) {
    tx.execute(
        "INSERT INTO policy (id, level, scope, condition, decision)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(id),
            Value::text(level),
            Value::text(scope),
            Value::text(condition),
            Value::text(decision),
        ],
    )
    .unwrap();
}

/// 表外的枚举列取值必须报错，不取默认——把未知层级猜成第 5 级会让优先级判错，
/// 与 `effect` 表那条解码用例同一条理由（设计上篇第 7.4 节「不猜」）。
#[test]
fn an_unknown_enum_column_value_is_rejected() {
    let (_dir, db) = db();

    // 三个枚举列各占一个作用域，理由同 `a_condition_that_is_not_a_valid_predicate_cannot_be_loaded`：
    // 不能同时开两个 `Tx`；不提交即回滚，下一个作用域看到空表。
    for (id, level, scope, decision, bogus) in [
        ("bad_level", "not_a_level", "user", "allow", "not_a_level"),
        ("bad_scope", "project", "not_a_scope", "allow", "not_a_scope"),
        ("bad_decision", "project", "user", "not_a_decision", "not_a_decision"),
    ] {
        let tx = db.begin().unwrap();
        insert_raw(&tx, id, level, scope, r#"{"all":[]}"#, decision);
        match load_policies(&tx).unwrap_err() {
            PersistError::Database(m) => assert!(
                m.contains(bogus),
                "错误信息应指出表外取值 {bogus}，实际 {m}"
            ),
            other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }
}

/// 读回次序按 id 升序，不依赖表的物理顺序：**插入次序与 id 次序故意相反**，
/// 两种「稳定次序」在这里才会给出不同结果。
///
/// 这条钉的是 `load_policies` 文档里那句「按 id 升序」——注释里的承诺没有对应用例
/// 就是说大了的说法。次序本身也是调用方要的：裁决要遍历全部规则，输出次序不稳
/// 会让同类输入的裁决结果在两次运行间不同。
#[test]
fn load_policies_returns_rows_in_id_order() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let any = Condition::parse(&json!({"all": []})).unwrap();
    save_policy(&tx, "p9", &policy(Level::SystemSafety, any.clone(), Decision::Deny, Scope::User))
        .unwrap();
    save_policy(&tx, "p1", &policy(Level::UserPersistent, any, Decision::Allow, Scope::User))
        .unwrap();

    let back = load_policies(&tx).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].level, Level::UserPersistent, "id 小的应排在前");
    assert_eq!(back[1].level, Level::SystemSafety);

    tx.commit().unwrap();
}

/// 空表读回空列表，不报错。
#[test]
fn an_empty_table_loads_as_an_empty_list() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    assert!(load_policies(&tx).unwrap().is_empty());
    tx.commit().unwrap();
}
