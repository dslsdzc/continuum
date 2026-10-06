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
///
/// **`effect_class` 恒取 `Some(DeleteRemote)`，故凡用它构造的登记项，能力表里必须含
/// `for_effect(DeleteRemote) = Git(DeleteRemote)`**——登记期不变量（Task 1）就长在
/// `save_tool` 上。经 [`sample`] 的三处用例因此都带上那一枚；直接调 `tool()` 的两处
/// 各自在调用点写明处置（`required_capabilities_round_trip` 的空表支改用 `None`）。
/// **不把 `effect_class` 改成 `None` 来回避**：`Some` 那一侧的往返照片
/// （`tool_round_trips` 的注释明写，设计 §10 第 9 条）不许被弄没。
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

/// 承 [`tool`] 的 `Some(DeleteRemote)`，故第三枚能力是它对应的 `Git(DeleteRemote)`：
/// 少了它，凡经本夹具（`registered` / `unregistered`）的 `save_tool` 都会被登记期
/// 不变量拒掉。前两枚是编码面的样本（单字动作 + 多词动作），第三枚是覆盖面的样本。
fn sample(id: &str) -> Tool {
    tool(
        id,
        vec![
            CapabilityKind::Filesystem(FsAction::Read),
            CapabilityKind::Git(GitAction::WorktreeWrite),
            CapabilityKind::Git(GitAction::DeleteRemote),
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

/// 无外部副作用（`effect_class` 为 `None`）、非确定、且尚未登记画像。
///
/// 这条画像专门给**解码侧**用：`effect_class = None` 与 `deterministic = false`
/// 是「fail-open 的那一侧」——把它们默认成某个 `EffectType` / `true`，工具会被当成
/// 有副作用的确定性工具。故这条必须经 `load_tool` 读回并逐字段断言，只在写侧查原始
/// 列值不够（复审 Important）。
fn pure(id: &str) -> ToolProfile {
    ToolProfile::new(
        Tool::new(
            ToolId::new(id),
            "0.1".to_owned(),
            json!({}),
            json!({}),
            vec![],
            None,
            false,
        ),
        None,
        None,
        Trust,
    )
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

    // 三种画像状态各存一条，且都**经 `load_tool` 读回**：
    // `Some`（画像已登记）、`None`（尚未登记），以及 `pure`（无外部副作用、非确定）。
    // 三者都必须往返——设计 §10 第 9 条禁止让 `Some` 一侧不可观察；`pure` 一侧是
    // `effect_class` / `deterministic` 两个解码分支的 fail-open 侧（复审 Important）。
    for profile in [registered("a"), unregistered("b"), pure("c")] {
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
    let c = load_tool(&tx, &ToolId::new("c")).unwrap().unwrap();
    assert_eq!(a.cost(), Some(Cost), "已登记画像应读回 Some(Cost)");
    assert_eq!(a.latency(), Some(Latency), "已登记画像应读回 Some(Latency)");
    assert_eq!(b.cost(), None, "尚未登记画像应读回 None");
    assert_eq!(b.latency(), None, "尚未登记画像应读回 None");
    assert_eq!((a.trust(), b.trust()), (Trust, Trust));
    // `effect_class` 的两个方向都经 `load_tool` 钉住：`Some` 侧不许丢，`None` 侧
    // 不许被默认成任何 `EffectType`（纯计算/只读工具被当成有外部副作用是 fail-open）
    assert_eq!(
        a.tool().effect_class(),
        Some(EffectType::DeleteRemote),
        "有外部副作用的工具应读回其类型"
    );
    assert_eq!(
        c.tool().effect_class(),
        None,
        "无外部副作用必须读回 None，不得默认成某个 EffectType"
    );
    // `deterministic` 的两个方向同理
    assert!(a.tool().deterministic(), "deterministic=true 应读回 true");
    assert!(
        !c.tool().deterministic(),
        "deterministic=false 必须读回 false，不得默认成 true"
    );

    // 不存在的 id 是 `None`，不是错误
    assert!(
        load_tool(&tx, &ToolId::new("nope")).unwrap().is_none(),
        "不存在的 id 应返回 None"
    );

    // 全量读按 id 升序
    let all = load_tools(&tx).unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].tool().id().as_str(), "a");
    assert_eq!(all[1].tool().id().as_str(), "b");
    assert_eq!(all[2].tool().id().as_str(), "c");

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
    // 三枚：前两枚是编码面的样本，第三枚是 `sample()` 为满足登记期不变量补的
    // `Git(DeleteRemote)`（见 `sample()`），故这里的字面量必须同步。
    assert_eq!(
        text_of(&rows[0][0]),
        r#"["filesystem_read","git_worktree_write","git_delete_remote"]"#,
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

    // 形态：对被写进去的**实际 kind 值**断言（不是对上面手写的字面量再数一遍——
    // 那样只有人改了字面量才会红，读起来却像覆盖了编码性质）。`as_str()` 必须是小写、
    // 多词以 `_` 连接，且不得含 `action()` 的点号（Debug 表示含大写，同样被排除）。
    for kind in [
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Git(GitAction::WorktreeWrite),
        CapabilityKind::Git(GitAction::DeleteRemote),
    ] {
        let encoded = kind.as_str();
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），{kind:?} 给出 {encoded}"
        );
        assert!(
            !encoded.contains('.'),
            "落库编码不得含 action() 的点号，{kind:?} 给出 {encoded}"
        );
    }
    assert!(
        !text_of(&rows[0][0]).contains("WorktreeWrite"),
        "落库编码不得是 Debug 表示"
    );

    // deterministic 的 false 与 effect_class 的 NULL 一侧同样有照片（写侧）
    save_tool(&tx, &pure("b")).unwrap();
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

    // 顺序与重复项都照存：这条列是列表，不是集合。
    // 末一枚 `Git(DeleteRemote)` 是 `tool()` 的 `Some(DeleteRemote)` 所要求的覆盖项
    // （登记期不变量），放在**末尾**以免打乱前四枚那条「顺序与重复项照存」的样本。
    let kinds = vec![
        CapabilityKind::Git(GitAction::Push),
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Payment(PaymentAction::Charge),
        CapabilityKind::Git(GitAction::Push),
        CapabilityKind::Git(GitAction::DeleteRemote),
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
        r#"["git_push","filesystem_read","payment_charge","git_push","git_delete_remote"]"#
    );

    // 空表是另一侧：空列表往返成空列表，不是 NULL，也不是一条空串元素。
    //
    // **这一支**不用 `tool()`（它是 `Some(DeleteRemote)`，空能力表会被登记期不变量
    // 拒掉），也不能「补一枚能力」——那一补恰好把「空列表往返」这个样本弄没。故在此
    // **直接构造** `effect_class: None` 的工具（与 `pure()` 同法）：本支的对象是空
    // 容器的编码，与效应无关，前提不成立时不变量无结论。
    let empty = ToolProfile::new(
        Tool::new(
            ToolId::new("b"),
            "1.2.3".to_owned(),
            json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            json!({"type": "boolean"}),
            vec![],
            None,
            true,
        ),
        None,
        None,
        Trust,
    );
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

/// 一条自定义 `effect_class` 与 `required_capabilities` 的登记项。
///
/// 三条不变量用例共用它：`tool()` 夹具恒取 `Some(DeleteRemote)`，钉不出别的组合。
fn with_effect(
    id: &str,
    required: Vec<CapabilityKind>,
    effect: Option<EffectType>,
) -> ToolProfile {
    ToolProfile::new(
        Tool::new(
            ToolId::new(id),
            "1".to_owned(),
            json!({}),
            json!({}),
            required,
            effect,
            true,
        ),
        None,
        None,
        Trust,
    )
}

/// 登记期不变量的**被拒那一侧**：声明了 `Some(Charge)` 而能力表里没有
/// `for_effect(Charge) = Payment(Charge)`，`save_tool` 必须返回 `Err`。
///
/// **且一行都不写**（「写都不写」，不是「写了再删」）：表里既没有那一行，行数也不增。
/// 只断 `Err` 会放过「先 INSERT 再回滚/再 DELETE」的实现——那在本用例里虽然等价，
/// 但它依赖调用方的事务边界，而不变量要的是**本函数自己**不写。
#[test]
fn saving_a_tool_whose_effect_class_is_not_covered_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let uncovered = with_effect(
        "uncovered",
        vec![CapabilityKind::Filesystem(FsAction::Read)],
        Some(EffectType::Charge),
    );

    // 断言**是哪一种** Err（变体 + 消息指出缺的那枚能力），不是「返回了 Err」
    let err = save_tool(&tx, &uncovered).unwrap_err();
    match &err {
        PersistError::Database(m) => assert!(
            m.contains(CapabilityKind::for_effect(EffectType::Charge).as_str()),
            "错误信息应指出缺的那枚能力，实际 {m}"
        ),
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }

    assert_eq!(count_tools(&tx), 0, "被拒的登记不得写入任何行");
    // **上面那一句之后，本句不是本 task 主题的独立守卫**（Task 1 评审 Minor 5，据实订正）。
    // 评审的原始说法是「本句被上一句**逻辑蕴含**」——**只在 `load_tool` 正确时成立**。
    // 按守卫判据机械地看，它是守卫：让 `load_tool` 为不存在的 id 臆造一行
    // （已实测的变异体 F′，见 Task 1 报告 §修复轮 1），此处全绿的前几句照过、本句红。故准确的处置是
    // 「**保留 + 写明它守的是什么**」，而不是「删」：
    //   - 它守的是 `load_tool` 的取数行为（不存在 ⇒ `None`），**不是**「不变量被拒后不落行」；
    //   - 那份取数行为已由 `tool_round_trips` 的「不存在的 id 是 `None`」断言钉住
    //     （同一变异体下那条也红）——故本句是**重复覆盖**，对本 task 不新增判别力。
    // 保留的理由：它是**用户可见的最终结果**（按 id 查回那一行拿不到），而不只是一个计数；
    // 计数正确而按 id 查得到，是读者会先怀疑的错法。
    assert!(
        load_tool(&tx, &ToolId::new("uncovered")).unwrap().is_none(),
        "被拒的登记不得留下那一行"
    );

    tx.commit().unwrap();
}

/// **方向相反的那一半**：`Some(Charge)` + `[Payment(Charge)]` 必须被接受，且读得回来。
///
/// 缺了这一条，一个「恒拒绝」的实现也能让上面那条用例全绿。
#[test]
fn a_declared_effect_class_covered_by_the_capabilities_is_accepted() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let covered = with_effect(
        "covered",
        vec![
            CapabilityKind::Filesystem(FsAction::Read),
            CapabilityKind::Payment(PaymentAction::Charge),
        ],
        Some(EffectType::Charge),
    );
    save_tool(&tx, &covered).unwrap();

    assert_eq!(count_tools(&tx), 1, "被接受的登记应恰写一行");
    let back = load_tool(&tx, &ToolId::new("covered"))
        .unwrap()
        .expect("被接受的登记应能读回");
    assert_eq!(back, covered, "读回的登记项应与写入逐字段相同");

    tx.commit().unwrap();
}

/// 不变量的**射程**：它形如 `effect_class == Some(t) ⇒ required 含 for_effect(t)`，
/// **前提不成立（`None`）时无结论**——`None` 的工具不受本不变量约束。
///
/// 这一条钉的是射程本身：把判定写成「`effect_class` 为空也必须出示点什么」或
/// 「`None` 也要比一轮」的实现，在本用例上红。
#[test]
fn a_tool_without_an_effect_class_may_declare_anything() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // 空表：`None` + `[]`
    let pure = with_effect("pure", vec![], None);
    save_tool(&tx, &pure).unwrap();
    assert_eq!(count_tools(&tx), 1, "`None` 的空表应被接受");

    // 与任何效应都无关的能力表：`None` + `[Filesystem(Read)]`
    // （这一枚的 `effect()` 是 `Option::None`，故它连「哪条效应」都指不出）
    let unrelated = with_effect(
        "unrelated",
        vec![CapabilityKind::Filesystem(FsAction::Read)],
        None,
    );
    save_tool(&tx, &unrelated).unwrap();
    assert_eq!(count_tools(&tx), 2, "`None` 的任意能力表都应被接受");

    let back = load_tool(&tx, &ToolId::new("pure")).unwrap().unwrap();
    assert_eq!(back.tool().effect_class(), None);
    let back = load_tool(&tx, &ToolId::new("unrelated")).unwrap().unwrap();
    assert_eq!(back, unrelated, "`None` 侧的登记应逐字段读回");

    tx.commit().unwrap();
}

/// 不变量的**读侧不重判**：本不变量生效**之前**登记的历史行（`Some(t)` 而能力表不覆盖）
/// 必须仍读得回来。
///
/// 这是本不变量**可部署**的安全性质：不变量只在新行**登记时**把关，不在读取时重判。
/// 若读侧也重判，升级本 crate 之后旧库里那些行会整个读不出来——`load_tools` 会连带失败，
/// 于是**已有的库**因为一条新增的登记规则而不可用。故 `src/persist.rs` 的模块文档写了
/// 「读侧不重判」，本条是那句话的照片。
///
/// **为什么单列一条**：`an_unknown_enum_column_value_is_rejected` 造的是**表外取值**
/// （`not_a_type` 之类，解不出来），那条对照臂 `ok` 的 `effect_class` 是 `None`——
/// 两者都不覆盖「**能解出来、但能力表不覆盖**」这一形态。缺了本条，「历史行」这个说法
/// 没有照片（绝对措辞须有用例）。
#[test]
fn a_legacy_row_that_violates_the_invariant_is_still_readable() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // 绕过 `save_tool` 直接写，模拟不变量生效之前登记的、或外部工具写进来的行：
    // `delete_remote` 解得出来（`EffectType::parse`），但能力表里没有 `git_delete_remote`。
    insert_raw(
        &tx,
        "legacy",
        r#"["filesystem_read"]"#,
        Some("delete_remote"),
        None,
        None,
        "",
        1,
    );

    let back = load_tool(&tx, &ToolId::new("legacy"))
        .unwrap()
        .expect("历史行必须仍读得回来——读侧不重判");
    assert_eq!(
        back.tool().effect_class(),
        Some(EffectType::DeleteRemote),
        "历史行的 effect_class 应原样读回"
    );
    let expected = vec![CapabilityKind::Filesystem(FsAction::Read)];
    assert_eq!(
        back.tool().required_capabilities(),
        expected.as_slice(),
        "能力表应原样读回，**不补**成覆盖形态——补了就从「读得回来」变成「读到的是另一样东西」"
    );

    // 全量读同样不得因这一行而失败
    assert_eq!(load_tools(&tx).unwrap().len(), 1);

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
    // 对照臂不只看「读得回来」，还逐项断言两个可空/布尔列的原值——否则
    // 「合法行」这一侧也是只钉了「没报错」，`effect_class` / `deterministic` 的
    // fail-open 默认仍能活过本用例
    let ok = load_tool(&tx, &ToolId::new("ok"))
        .unwrap()
        .expect("合法行应读得回来（对照臂）");
    assert_eq!(ok.tool().effect_class(), None, "对照臂的 effect_class 应为 None");
    assert!(!ok.tool().deterministic(), "对照臂的 deterministic 应为 false");

    tx.commit().unwrap();
}

/// 列类型不符（[`PersistError::ColumnType`]）与列缺失两条错误臂各有照片。
///
/// SQLite 的 TEXT 亲和列会把整数转成文本，故「类型不符」用 **BLOB** 制造：blob 不被
/// 亲和转换，读回是 [`Value::Blob`]，`text_at` / `int_at` 都收窄不了。
/// 列缺失那一臂库侧没有产生方（`SELECT` 的列清单固定），照片在 `src/persist.rs` 的
/// 单元用例 `a_short_row_reports_the_missing_column`。
#[test]
fn a_column_of_the_wrong_type_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    // version（第 1 列）与 deterministic（第 6 列）各写一个 BLOB，其余列合法
    tx.execute(
        "INSERT INTO tool
           (id, version, input_schema, output_schema, required_capabilities,
            effect_class, deterministic, cost, latency, trust)
         VALUES ('bad_version', ?1, '{}', '{}', '[]', NULL, 0, NULL, NULL, '')",
        &[Value::Blob(vec![1, 2, 3])],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO tool
           (id, version, input_schema, output_schema, required_capabilities,
            effect_class, deterministic, cost, latency, trust)
         VALUES ('bad_det', '1', '{}', '{}', '[]', NULL, ?1, NULL, NULL, '')",
        &[Value::Blob(vec![1, 2, 3])],
    )
    .unwrap();

    // 断言是**哪一种** Err（哪个下标、实际类型），不是「返回了 Err」
    assert_eq!(
        load_tool(&tx, &ToolId::new("bad_version")).unwrap_err(),
        PersistError::ColumnType {
            index: 1,
            actual: "blob",
        },
        "version 列是 BLOB，应报第 1 列类型不符"
    );
    assert_eq!(
        load_tool(&tx, &ToolId::new("bad_det")).unwrap_err(),
        PersistError::ColumnType {
            index: 6,
            actual: "blob",
        },
        "deterministic 列是 BLOB，应报第 6 列类型不符"
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
