//! `tool` 表的落库与读写（设计 §3.3、§7）。
//!
//! 本文件的夹具 `db()` 单独成立：它只依赖 `p3_capability_migrations` 与 `Db`，
//! 建库后必须 `migrate()` —— 漏掉那一行时各用例会一起挂在 `no such table`，
//! 而报错位置指向被测函数，容易误判成实现的问题（同 `continuum-effect/tests/persist.rs`）。

use continuum_capability::{
    CapabilityKind, Cost, EmailAction, EnvAction, FsAction, GitAction, GithubAction, Latency,
    PaymentAction, RegistryAction, Tool, ToolId, ToolProfile, Trust, load_tool, load_tools,
    p3_capability_migrations, save_tool,
};
use continuum_effect::EffectType;
use continuum_persist::{Db, Migration, PersistError, Tx, Value, builtin_migrations};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p3_capability_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条工具定义。字段全部取非默认值：若某列不落库或与邻列写串，
/// 读回时的整结构比对会变红（默认值/零值会让这类缺陷静默）。
///
/// `required_capabilities` 同时含单字动作与多词动作（`Git(WorktreeWrite)`），
/// 后者在落库编码里必须写成 `git_worktree_write`——**不是** `action()` 的
/// `worktree.write`（点号不满足落库约定）。
fn tool(id: &str, required: Vec<CapabilityKind>) -> Tool {
    Tool::new(
        ToolId::new(id),
        "1.2.3".to_owned(),
        json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        json!({"type": "boolean"}),
        required,
        Some(EffectType::DeleteRemote),
        true,
    )
}

fn sample(id: &str) -> Tool {
    tool(
        id,
        vec![
            CapabilityKind::Filesystem(FsAction::Read),
            CapabilityKind::Git(GitAction::WorktreeWrite),
        ],
    )
}

/// 画像已登记（`cost` / `latency` 为 `Some`）。
fn registered(id: &str) -> ToolProfile {
    ToolProfile::new(sample(id), Some(Cost), Some(Latency), Trust)
}

/// 尚未登记画像（`cost` / `latency` 为 `None`）。
fn unregistered(id: &str) -> ToolProfile {
    ToolProfile::new(sample(id), None, None, Trust)
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

fn count_tools(tx: &Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM tool", &[]).unwrap();
    int_of(&rows[0][0])
}

/// 绕过编码辅助函数直接写行：模拟旧版本写入的、或外部工具写坏的表外取值。
/// 七个参数里 `None` 表示该列为 `NULL`。
fn insert_raw(
    tx: &Tx<'_>,
    id: &str,
    required_capabilities: &str,
    effect_class: Option<&str>,
    cost: Option<&str>,
    latency: Option<&str>,
    trust: &str,
    deterministic: i64,
) {
    tx.execute(
        "INSERT INTO tool
           (id, version, input_schema, output_schema, required_capabilities,
            effect_class, deterministic, cost, latency, trust)
         VALUES (?1, '1', '{}', '{}', ?2, ?3, ?4, ?5, ?6, ?7)",
        &[
            Value::text(id),
            Value::text(required_capabilities),
            effect_class.map_or(Value::Null, Value::text),
            Value::Int(deterministic),
            cost.map_or(Value::Null, Value::text),
            latency.map_or(Value::Null, Value::text),
            Value::text(trust),
        ],
    )
    .unwrap();
}

#[test]
fn tool_round_trips() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // 空表读出空表，不是错误
    assert!(load_tools(&tx).unwrap().is_empty(), "空表应读出空列表");

    // 两种画像状态各存一条：`Some`（画像已登记）与 `None`（尚未登记）。
    // 两者都必须往返——设计 §10 第 9 条禁止让 `Some` 一侧不可观察。
    for profile in [registered("a"), unregistered("b")] {
        save_tool(&tx, &profile).unwrap();
        let back = load_tool(&tx, profile.tool().id())
            .unwrap()
            .expect("刚存入的工具应能读回");
        // 整结构比对覆盖定义面七字段与画像面三字段
        assert_eq!(back, profile, "读回的登记项应与写入逐字段相同");
    }

    // 逐格钉住两侧画像状态，别让上面的整结构比对独自承担
    let a = load_tool(&tx, &ToolId::new("a")).unwrap().unwrap();
    let b = load_tool(&tx, &ToolId::new("b")).unwrap().unwrap();
    assert_eq!(a.cost(), Some(Cost), "已登记画像应读回 Some(Cost)");
    assert_eq!(a.latency(), Some(Latency), "已登记画像应读回 Some(Latency)");
    assert_eq!(b.cost(), None, "尚未登记画像应读回 None");
    assert_eq!(b.latency(), None, "尚未登记画像应读回 None");
    assert_eq!((a.trust(), b.trust()), (Trust, Trust));

    // 不存在的 id 是 `None`，不是错误
    assert!(
        load_tool(&tx, &ToolId::new("nope")).unwrap().is_none(),
        "不存在的 id 应返回 None"
    );

    // 全量读按 id 升序
    let all = load_tools(&tx).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].tool().id().as_str(), "a");
    assert_eq!(all[1].tool().id().as_str(), "b");

    tx.commit().unwrap();
}

#[test]
fn enum_columns_use_the_lowercase_encoding() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(&tx, &registered("a")).unwrap();

    let rows = tx
        .query(
            "SELECT required_capabilities, effect_class, cost, latency, trust, deterministic
             FROM tool",
            &[],
        )
        .unwrap();

    // `required_capabilities` 的**容器**是 JSON 字符串数组（同 `continuum-graph` 的
    // `adfir_node.capabilities` 列）；**元素**取自 `CapabilityKind::as_str`。
    assert_eq!(
        text_of(&rows[0][0]),
        r#"["filesystem_read","git_worktree_write"]"#,
        "元素编码应为小写、多词以 _ 连接"
    );
    // effect_class 复用 continuum-effect 既有的 EffectType 编码，不自建第二份
    assert_eq!(text_of(&rows[0][1]), "delete_remote");
    // cost / latency 的 `Some` 以非 NULL 编码；空串是那个个体的名字，不是「未知」
    assert_eq!(text_of(&rows[0][2]), "");
    assert_eq!(text_of(&rows[0][3]), "");
    // trust 恒非 NULL
    assert_eq!(text_of(&rows[0][4]), "");
    assert_eq!(int_of(&rows[0][5]), 1, "deterministic 的 true 应落成 1");

    // 形态：元素是小写、多词以 _ 连接，且**不是** Debug 表示（Debug 含大写）
    for element in ["filesystem_read", "git_worktree_write"] {
        assert!(
            !element.is_empty()
                && element.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {element}"
        );
    }
    assert!(
        !text_of(&rows[0][0]).contains("WorktreeWrite"),
        "落库编码不得是 Debug 表示"
    );

    // deterministic 的 false 一侧同样有照片
    save_tool(
        &tx,
        &ToolProfile::new(
            Tool::new(
                ToolId::new("b"),
                "1".to_owned(),
                json!({}),
                json!({}),
                vec![],
                None,
                false,
            ),
            None,
            None,
            Trust,
        ),
    )
    .unwrap();
    let rows = tx
        .query("SELECT deterministic, effect_class FROM tool WHERE id = 'b'", &[])
        .unwrap();
    assert_eq!(int_of(&rows[0][0]), 0, "deterministic 的 false 应落成 0");
    // effect_class 的 `None` 一侧也落成 NULL
    assert_eq!(rows[0][1], Value::Null, "无外部副作用应落成 NULL");

    tx.commit().unwrap();
}

/// 存在性编码的**两侧都钉**：存在的那一侧只接受那一个字面量，表外取值一律拒绝。
///
/// 三处编码（`Cost` / `Latency` / `Trust`）各一行，故三条解码路径都有对照片。
#[test]
fn the_presence_encoding_has_exactly_one_literal() {
    assert_eq!(Cost::as_str(&Cost), "");
    assert_eq!(Latency::as_str(&Latency), "");
    assert_eq!(Trust::as_str(&Trust), "");

    assert_eq!(Cost::parse(""), Some(Cost));
    assert_eq!(Latency::parse(""), Some(Latency));
    assert_eq!(Trust::parse(""), Some(Trust));

    for bogus in ["0", "high", "low", "null", "unknown", " "] {
        assert_eq!(Cost::parse(bogus), None, "表外取值 {bogus} 不应解出 Cost");
        assert_eq!(
            Latency::parse(bogus),
            None,
            "表外取值 {bogus} 不应解出 Latency"
        );
        assert_eq!(Trust::parse(bogus), None, "表外取值 {bogus} 不应解出 Trust");
    }
}

#[test]
fn required_capabilities_round_trip() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // 顺序与重复项都照存：这条列是列表，不是集合
    let kinds = vec![
        CapabilityKind::Git(GitAction::Push),
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Payment(PaymentAction::Charge),
        CapabilityKind::Git(GitAction::Push),
    ];
    let profile = ToolProfile::new(tool("a", kinds.clone()), None, None, Trust);
    save_tool(&tx, &profile).unwrap();
    let back = load_tool(&tx, &ToolId::new("a")).unwrap().unwrap();
    assert_eq!(
        back.tool().required_capabilities(),
        kinds.as_slice(),
        "顺序与内容都应一致"
    );

    // 容器是 JSON 数组，元素取自 as_str，不多不少
    let raw = tx
        .query("SELECT required_capabilities FROM tool WHERE id = 'a'", &[])
        .unwrap();
    assert_eq!(
        text_of(&raw[0][0]),
        r#"["git_push","filesystem_read","payment_charge","git_push"]"#
    );

    // 空表是另一侧：空列表往返成空列表，不是 NULL，也不是一条空串元素
    let empty = ToolProfile::new(tool("b", vec![]), None, None, Trust);
    save_tool(&tx, &empty).unwrap();
    let back = load_tool(&tx, &ToolId::new("b")).unwrap().unwrap();
    assert!(
        back.tool().required_capabilities().is_empty(),
        "空列表应往返成空列表"
    );
    let raw = tx
        .query("SELECT required_capabilities FROM tool WHERE id = 'b'", &[])
        .unwrap();
    assert_eq!(text_of(&raw[0][0]), "[]");

    tx.commit().unwrap();
}

#[test]
fn saving_the_same_id_twice_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    let original = registered("a");
    save_tool(&tx, &original).unwrap();

    // 同 id、内容不同。裸 INSERT，不是 OR REPLACE：判据在库层
    let err = save_tool(&tx, &unregistered("a")).unwrap_err();
    // 断言是哪一种 Err：`Tx::execute` 把 rusqlite 的错误码抹成字符串，
    // 故只能在 `Database` 变体内按消息断言冲突来自主键。
    match &err {
        PersistError::Database(m) => assert!(
            m.contains("UNIQUE constraint failed: tool.id"),
            "应为 tool.id 上的主键冲突，实际 {m}"
        ),
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }

    // 原行不得被改动
    assert_eq!(
        load_tool(&tx, &ToolId::new("a")).unwrap().unwrap(),
        original,
        "被拒的写入不得改动原行"
    );
    assert_eq!(count_tools(&tx), 1, "被拒的写入不得新增行");

    // 挡住的是同 id，不是全部写入：换个 id 仍能写
    save_tool(&tx, &registered("c")).unwrap();
    assert_eq!(count_tools(&tx), 2);

    tx.commit().unwrap();
}

#[test]
fn an_unknown_enum_column_value_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // 每一行只写坏一列，使每条解码路径都有对照片；其余列取合法值。
    // 最后一行是全合法值，作为对照臂。
    let rows: &[(&str, &str, Option<&str>, Option<&str>, Option<&str>, &str, i64, &str)] = &[
        (
            "bad_cap",
            r#"["git_push","not_a_kind"]"#,
            None,
            None,
            None,
            "",
            0,
            "not_a_kind",
        ),
        ("bad_json", "not json", None, None, None, "", 0, "not json"),
        (
            "bad_effect",
            "[]",
            Some("not_a_type"),
            None,
            None,
            "",
            0,
            "not_a_type",
        ),
        ("bad_cost", "[]", None, Some("high"), None, "", 0, "high"),
        ("bad_latency", "[]", None, None, Some("slow"), "", 0, "slow"),
        ("bad_trust", "[]", None, None, None, "trusted", 0, "trusted"),
        ("bad_det", "[]", None, None, None, "", 2, "2"),
    ];
    for (id, caps, effect, cost, latency, trust, det, _) in rows {
        insert_raw(&tx, id, caps, *effect, *cost, *latency, trust, *det);
    }
    // 对照臂：全合法值的一行必须读得回来
    insert_raw(&tx, "ok", "[]", None, None, None, "", 0);

    // 表外取值必须报错，不得取默认值悄悄读回
    for (id, _, _, _, _, _, _, bogus) in rows {
        let err = load_tool(&tx, &ToolId::new(*id)).unwrap_err();
        match &err {
            PersistError::Database(m) => assert!(
                m.contains(bogus),
                "错误信息应指出表外取值 {bogus}，实际 {m}"
            ),
            other => panic!("{id}: 应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }
    assert!(
        load_tool(&tx, &ToolId::new("ok")).unwrap().is_some(),
        "合法行应读得回来（对照臂）"
    );

    tx.commit().unwrap();
}

/// `CapabilityKind` 落库编码的往返，覆盖**全部十二个** kind。
///
/// **这条用例守的是解码侧**：`parse` 从 `&str` 出发，编译器点不出漏掉的臂，只有遍历
/// 能发现「某个**已在名单里**的 kind 解不回去」。编码侧不靠本用例——`as_str` 的 match
/// 穷尽且无通配臂，加 kind 时编译失败。
///
/// **数目断言比它看起来窄**（据实写明，同 `EffectState::ALL` 的说明）：它只在「名单
/// 长度与这里的字面量不一致」时红，**发现不了**「给枚举加了 kind 却没加进本名单」——
/// 那种情形下长度不变，遍历也走不到新 kind，本用例照过。那一种由 `as_str` 的编译失败
/// 拦下：报错点就在 `CapabilityKind` 的定义处。
///
/// **末两条断言钉住「落库编码不是 `action()`」**：给例里的展示词汇含点号
/// （`worktree.write`），不满足落库约定，两者取值**刻意不同**。后来者若「统一」它们，
/// 这里会红。
#[test]
fn capability_kind_storage_encoding_round_trips_every_arm() {
    let all = [
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Filesystem(FsAction::Write),
        CapabilityKind::Git(GitAction::Read),
        CapabilityKind::Git(GitAction::WorktreeWrite),
        CapabilityKind::Git(GitAction::CommitLocal),
        CapabilityKind::Git(GitAction::Push),
        CapabilityKind::Git(GitAction::DeleteRemote),
        CapabilityKind::Github(GithubAction::CreatePr),
        CapabilityKind::Email(EmailAction::Send),
        CapabilityKind::Registry(RegistryAction::Publish),
        CapabilityKind::Payment(PaymentAction::Charge),
        CapabilityKind::Environment(EnvAction::Deploy),
    ];
    assert_eq!(all.len(), 12, "CapabilityKind 的名单与 kind 数不符");

    for kind in all {
        let encoded = kind.as_str();
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
        assert_eq!(
            CapabilityKind::parse(encoded),
            Some(kind),
            "{encoded} 应解回 {kind:?}"
        );
    }

    // 表外取值一律 None：不取默认 kind——会把伪造串当成另一枚能力，正是 §253 要拦的
    for bogus in [
        "",
        "git",
        "git.",
        "Git_Push",
        "git_push ",
        "filesystem",
        "worktree.write",
        "create_pr",
        "delete_remote",
    ] {
        assert_eq!(
            CapabilityKind::parse(bogus),
            None,
            "{bogus} 不应解出任何 kind"
        );
    }

    // 落库编码 ≠ action()：这一对刻意不同，不得被「统一」
    assert_eq!(
        CapabilityKind::Git(GitAction::WorktreeWrite).action(),
        "worktree.write",
        "action() 复现 §88 给例的点号形状"
    );
    assert_eq!(
        CapabilityKind::Git(GitAction::WorktreeWrite).as_str(),
        "git_worktree_write",
        "落库编码用小写 + _，点号不满足落库约定"
    );
    assert_eq!(
        CapabilityKind::Github(GithubAction::CreatePr).action(),
        "create_pr"
    );
    assert_eq!(
        CapabilityKind::Github(GithubAction::CreatePr).as_str(),
        "github_create_pr"
    );
}
