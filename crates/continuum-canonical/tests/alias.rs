//! `alias` 表（迁移 100）、§278 的四作用域与「不自动永久化」（设计 §4.6）。
//!
//! 本文件的夹具 `db()` 单独成立：只依赖 `p4_canonical_migrations` 与 `Db`，
//! 建库后必须 `migrate()`——漏掉那一行时各用例会一起挂在 `no such table: alias`，
//! 而报错位置指向被测函数，容易误判成实现的问题（同 `continuum-capability/tests/persist.rs`）。
//!
//! # 本文件钉的四件事，以及每一件为什么这样钉
//!
//! 1. **四作用域逐枚往返、且落库编码逐枚断言**（`every_alias_scope_round_trips`）。
//!    只断言「往返一致」是不够的：`as_str` 与 `parse` 互为逆函数时，一张把
//!    `TURN` 与 `SESSION` 都编成 `"session"` 的表照样往返得通（读回来永远是后一枚）。
//!    故**另断一枚原始列文本**——编码纪律（小写、多词接 `_`）只能从库里读出来。
//! 2. **表外取值读回得具体 `Err`**，两条读路径各一条（`load_alias` / `list_aliases`）。
//! 3. **不自动永久化**（§278 的 MUST NOT）——两侧：
//!    写 `SESSION` 之后 `scope='user_persistent'` 的行数为 0（否定式照片）；
//!    而显式 `promote_alias` 得 `Ok` 且读回 scope 变了（**只钉前一侧会让提升路径
//!    整个不存在也照样绿**，设计 §4.6 第 2 条两端逐字点名）。
//! 4. **列清单恰好是三列、且没有过期列**（§4.6 第 3 条）。
//!
//! # 本文件刻意不写的话
//!
//! **不写「没有任何函数会顺带调用 `promote_alias`」这句绝对措辞**（设计 §4.6 第 2 条有它）。
//! 本文件能拍到的是**写侧的可观察行为**——`save_alias_writes_exactly_the_scope_it_was_given`
//! 钉住 `save_alias` 不替调用方升级作用域。**整句话还差一个源码扫描**（照
//! `crates/continuum-capability/tests/single_writer.rs` 的形态扫本 crate 的 `src/`），
//! 本 task 未落，故那半句不在任何地方被断言——这处缺口照实记在此，不靠措辞含糊过去。

use continuum_canonical::{
    Alias, AliasError, AliasScope, EntityId, list_aliases, load_alias, p4_canonical_migrations,
    promote_alias, save_alias,
};
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_canonical_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条待写的 alias。`target` 取非空串：某列若不落库或与邻列写串，
/// 读回时的逐字段比对会变红。
fn alias(name: &str, target: &str, scope: AliasScope) -> Alias {
    Alias {
        alias: name.to_owned(),
        target: EntityId::from_str(target),
        scope,
    }
}

/// 绕开全部公开读写函数，直读一行的 `scope` **原始列文本**。
///
/// 这是编码纪律（小写、多词接 `_`）的唯一照片：经 `load_alias` 读回来的是已经过
/// `parse` 的值，`as_str` 与 `parse` 一起错（例如把 `TURN` 也编成 `"session"`）
/// 时它是看不出来的。
fn raw_scope(tx: &Tx<'_>, name: &str) -> String {
    let rows = tx
        .query("SELECT scope FROM alias WHERE alias = ?1", &[Value::text(name)])
        .unwrap();
    assert_eq!(rows.len(), 1, "alias 表里应有恰一行 {name:?}，实得 {} 行", rows.len());
    match &rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("scope 列应是文本，实得 {other:?}"),
    }
}

/// `scope` 列等于 `scope` 的行数。
fn count_scope(tx: &Tx<'_>, scope: &str) -> i64 {
    let rows = tx
        .query("SELECT COUNT(*) FROM alias WHERE scope = ?1", &[Value::text(scope)])
        .unwrap();
    match &rows[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// 运行期从库里读出的 `alias` 表列清单：`(列名, 声明类型, NOT NULL, 主键位次)`。
fn columns(tx: &Tx<'_>) -> Vec<(String, String, i64, i64)> {
    tx.query("PRAGMA table_info(alias)", &[])
        .unwrap()
        .iter()
        .map(|row| {
            let text = |at: usize| match &row[at] {
                Value::Text(s) => s.clone(),
                other => panic!("table_info 第 {at} 列应是文本，实得 {other:?}"),
            };
            let int = |at: usize| match &row[at] {
                Value::Int(i) => *i,
                other => panic!("table_info 第 {at} 列应是整数，实得 {other:?}"),
            };
            (text(1), text(2), int(3), int(5))
        })
        .collect()
}

/// §278 的四值逐枚往返，且**落库编码逐枚断言**。
///
/// 四值各取一条、**不抽代表**：手写分支能各自漂移，抽一条样本时其余三枚的编码
/// 错了也看不见（本仓「枚举断言须逐项有照片」）。
///
/// 红条件两枚，档位不同：
/// - `as_str` 少一档（两枚变体编成同一个串）——**放宽档**：编码的定义域被收窄，
///   直接往返看不出来，靠下面的原始列文本那一段。
/// - 多词不接 `_`（`"userpersistent"`）——**取反档**：编码纪律本身被违反。
#[test]
fn every_alias_scope_round_trips() {
    // (行名, 值, 落库字面量)
    let cases = [
        ("turn-alias", AliasScope::Turn, "turn"),
        ("session-alias", AliasScope::Session, "session"),
        ("project-alias", AliasScope::Project, "project"),
        ("user-alias", AliasScope::UserPersistent, "user_persistent"),
    ];

    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    for &(name, scope, _) in cases.iter() {
        save_alias(&tx, &alias(name, "e-1", scope)).unwrap();
    }

    // 先钉编码：库里的原始文本逐枚等于上表右列。
    for &(name, _, literal) in cases.iter() {
        assert_eq!(
            raw_scope(&tx, name),
            literal,
            "{name} 落库的 scope 不是 {literal:?}——四值的编码纪律（小写、多词接 `_`）在此处断"
        );
    }

    // 再钉往返：读回来的是**哪一枚**（不是「有个值」）。
    for &(name, scope, _) in cases.iter() {
        let got = load_alias(&tx, name)
            .unwrap()
            .unwrap_or_else(|| panic!("刚写进去的 {name} 应读得回来"));
        assert_eq!(got.scope, scope, "{name} 读回来的是另一枚作用域");
    }

    tx.commit().unwrap();
}

/// 表外取值经 `load_alias` 读回得**具体** `Err`，且错误里带的是**那一个**值。
///
/// 红条件：`parse` 对认不出的串返回一个默认态（**放宽档**）——那时读回得
/// `Ok(某枚作用域)`，本用例红。把认不出的串猜成另一枚作用域，正是 §4.6 第 1 条
/// 与 P3D 的枚举列纪律要拦的：一个拼错的 scope 会静默变成一条有作用域的记录。
#[test]
fn an_out_of_table_scope_reads_back_as_a_specific_err() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    // 裸 INSERT：`save_alias` 收的是 `AliasScope`，从类型上写不出表外取值，
    // 故这一行只能绕过它造出来——它是「库里的历史行」这一路的语料。
    tx.execute(
        "INSERT INTO alias (alias, target, scope) VALUES (?1, ?2, ?3)",
        &[
            Value::text("bad"),
            Value::text("e-1"),
            Value::text("global"),
        ],
    )
    .unwrap();

    let err = load_alias(&tx, "bad").unwrap_err();
    assert_eq!(
        err,
        AliasError::UnknownScope {
            value: "global".to_owned(),
        },
        "表外 scope 应读回带原始取值的 UnknownScope，实得 {err:?}"
    );

    tx.commit().unwrap();
}

/// `list_aliases` 是**第二条读路径**，它也要拒表外取值。
///
/// 与上一条分开写：两条读路径共用一个解码函数是**实现**的选择，不是判据——
/// 哪天真把它们分开（例如 list 走一条批量解码），缺了这条照片的那条路径会静默放宽。
#[test]
fn list_aliases_rejects_an_out_of_table_scope_too() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO alias (alias, target, scope) VALUES (?1, ?2, ?3)",
        &[
            Value::text("bad"),
            Value::text("e-1"),
            Value::text("global"),
        ],
    )
    .unwrap();

    let err = list_aliases(&tx).unwrap_err();
    assert_eq!(
        err,
        AliasError::UnknownScope {
            value: "global".to_owned(),
        },
        "list_aliases 也应报带原始取值的 UnknownScope，实得 {err:?}"
    );

    tx.commit().unwrap();
}

/// §278 的正面禁令（MUST NOT 默认把 Session Alias 永久化）——**否定式照片**。
///
/// 红条件：`save_alias` 把 scope 硬编码成 `USER_PERSISTENT`（**移除档**——
/// 把「写调用方给的那个 scope」这一步去掉，依赖它的断言随之红）。
#[test]
fn a_session_alias_is_never_written_as_user_persistent() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_alias(&tx, &alias("s", "e-1", AliasScope::Session)).unwrap();

    assert_eq!(
        count_scope(&tx, "user_persistent"),
        0,
        "写一条 SESSION alias 之后，库里不该有任何 user_persistent 行"
    );
    assert_eq!(raw_scope(&tx, "s"), "session");

    tx.commit().unwrap();
}

/// 显式提升那条路径——**上一条的另一侧，不能省**。
///
/// 只钉「不提升」那侧，一个 `promote_alias` 根本不存在（或恒 `Ok` 而无写入）的实现
/// 也照样全绿。故本条先断提升前是 `SESSION`、再断提升后**变了**。
#[test]
fn an_explicit_promotion_changes_the_scope() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_alias(&tx, &alias("s", "e-1", AliasScope::Session)).unwrap();
    assert_eq!(
        load_alias(&tx, "s").unwrap().unwrap().scope,
        AliasScope::Session,
        "提升之前应是 SESSION"
    );

    promote_alias(&tx, "s", AliasScope::UserPersistent).unwrap();

    assert_eq!(
        load_alias(&tx, "s").unwrap().unwrap().scope,
        AliasScope::UserPersistent,
        "显式提升之后 scope 应已改变"
    );
    assert_eq!(raw_scope(&tx, "s"), "user_persistent");

    tx.commit().unwrap();
}

/// 提升一个不存在的 alias 是 `Err`，不是静默的 `Ok`。
///
/// 红条件：`promote_alias` 只看 `execute` 的返回值是否 `Ok`（**移除档**——
/// 去掉「影响行数为 0 即报错」那一步）。`UPDATE` 匹配不到行时 SQLite 返回 `Ok(0)`，
/// 于是「提升了一个拼错的 alias」会得到一次成功的假象，而调用方无从分辨。
#[test]
fn promoting_an_alias_that_does_not_exist_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let err = promote_alias(&tx, "nope", AliasScope::UserPersistent).unwrap_err();
    assert_eq!(
        err,
        AliasError::NotFound {
            alias: "nope".to_owned(),
        },
        "提升一个不存在的 alias 应报 NotFound，实得 {err:?}"
    );

    tx.commit().unwrap();
}

/// `save_alias` 只写调用方给的那个作用域——同一条 alias、两个库、两种 scope。
///
/// 这一对计数（0 与 1）是「**写侧不替调用方升级作用域**」的可观察事实：
/// 同一个名字在 `SESSION` 库里 `user_persistent` 行数为 0、在 `USER_PERSISTENT` 库里为 1，
/// 说明落库的 scope 是**入参决定**的，与名字无关、也不由 `save_alias` 自己抬高。
///
/// **它拍不到的范围照实写**：这条断的是 `save_alias` 这一个函数的行为，
/// **不是**「本 crate 里没有任何函数会顺带调用 `promote_alias`」那句更宽的话
/// （见本文件头「本文件刻意不写的话」）。
#[test]
fn save_alias_writes_exactly_the_scope_it_was_given() {
    let (_dir_a, db_a) = db();
    let tx_a = db_a.begin().unwrap();
    save_alias(&tx_a, &alias("same", "e-1", AliasScope::Session)).unwrap();
    assert_eq!(
        count_scope(&tx_a, "user_persistent"),
        0,
        "以 SESSION 写入的库里不该有 user_persistent 行"
    );
    tx_a.commit().unwrap();

    let (_dir_b, db_b) = db();
    let tx_b = db_b.begin().unwrap();
    save_alias(&tx_b, &alias("same", "e-1", AliasScope::UserPersistent)).unwrap();
    assert_eq!(
        count_scope(&tx_b, "user_persistent"),
        1,
        "以 USER_PERSISTENT 写入的库里应有恰一行 user_persistent"
    );
    tx_b.commit().unwrap();
}

/// 同一条 alias 写第二次是 `Err`，且库里仍是第一行。
///
/// 判据在库层（主键）而不在调用方自查：`INSERT OR REPLACE` 会让第二次写入
/// **覆盖**第一次——那正是一条不经过 `promote_alias` 的作用域改写路径，
/// 与 §4.6 第 2 条「提升是一个独立的显式操作」相抵。
#[test]
fn saving_the_same_alias_twice_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_alias(&tx, &alias("dup", "e-1", AliasScope::Session)).unwrap();

    assert!(
        save_alias(&tx, &alias("dup", "e-2", AliasScope::UserPersistent)).is_err(),
        "同一条 alias 的第二次写入应由主键拒掉"
    );
    // 被拒之后原行不变：变更的既不是 target 也不是 scope。
    let got = load_alias(&tx, "dup").unwrap().unwrap();
    assert_eq!(got.target.as_str(), "e-1");
    assert_eq!(got.scope, AliasScope::Session);

    tx.commit().unwrap();
}

/// `list_aliases` 交出全部行，按 alias 升序。
///
/// 顺序固定是**公开契约**（调用方不必自己排）：乱序时本用例红。
#[test]
fn list_aliases_returns_every_row_in_alias_order() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    // 写入顺序刻意与字典序相反。
    save_alias(&tx, &alias("c", "e-3", AliasScope::Project)).unwrap();
    save_alias(&tx, &alias("a", "e-1", AliasScope::Turn)).unwrap();
    save_alias(&tx, &alias("b", "e-2", AliasScope::UserPersistent)).unwrap();

    let all = list_aliases(&tx).unwrap();
    let names: Vec<&str> = all.iter().map(|a| a.alias.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
    let scopes: Vec<AliasScope> = all.iter().map(|a| a.scope).collect();
    assert_eq!(
        scopes,
        vec![
            AliasScope::Turn,
            AliasScope::UserPersistent,
            AliasScope::Project
        ]
    );

    tx.commit().unwrap();
}

/// `alias` 表的列清单**恰好**是手写期望的那三列，逐列比对（多一列即红）。
///
/// # 这条断言**不恒真**（与「手抄名单恒真」那一类不同）
///
/// 一侧是**运行期从库里读出的实际列**（`PRAGMA table_info`），另一侧是**手写的期望**——
/// 两者相等是运行期才知道的事实，不是同一份文本的两份抄写。改表加一列、
/// 或把某个 `NOT NULL` 去掉，这里就红。
///
/// 逐列比的是 `(列名, 声明类型, NOT NULL, 主键位次)` 四元组（不是只比列名）：
/// 只比名字的话，「把 `scope` 的 `NOT NULL` 去掉」（那一列可以落成 `NULL`，
/// 而 `NULL` 经 `parse` 会变成一条没有作用域的记录）不会被发现。
#[test]
fn the_alias_table_has_exactly_the_expected_columns() {
    /// 手写的期望清单：`(列名, 类型, NOT NULL, 主键位次)`。
    ///
    /// `alias` 是**主键**：提升（改 scope）是「改一行的属性」，若键是 `(alias, scope)`，
    /// 提升就变成插入一行、旧行还在——那不是提升。这也是 `promote_alias` 只 `UPDATE`
    /// 一行的前提。
    const EXPECTED: &[(&str, &str, i64, i64)] = &[
        ("alias", "TEXT", 1, 1),
        ("target", "TEXT", 1, 0),
        ("scope", "TEXT", 1, 0),
    ];

    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    let actual = columns(&tx);
    let expected: Vec<(String, String, i64, i64)> = EXPECTED
        .iter()
        .map(|&(name, ty, notnull, pk)| (name.to_owned(), ty.to_owned(), notnull, pk))
        .collect();

    assert_eq!(
        actual, expected,
        "alias 表的实际列与手写期望不符（列名/类型/NOT NULL/主键位次四元组逐列比）"
    );

    tx.commit().unwrap();
}

/// §4.6 第 3 条：`TURN` / `SESSION` 的**过期时点规范未给**，故本层只实现
/// 「不自动永久化」，**不实现会话边界的清退**。
///
/// 两条断言合起来说的是这一句：
/// - **否定式照片**：列清单里没有 `expires_at` 一类的列——没有过期列，
///   就**没有**任何一处能按它清退；
/// - **正面**：刚写的 `TURN` 与 `SESSION` 都立刻读得回来（它们照常持久化，
///   只是不自动升格成 `USER_PERSISTENT`）。
///
/// **为什么没有过期列**：**没有定义「会话结束」是什么**（设计 §16 第 7 条；
/// §274 的 `session` / `conversation` 两枚作用域也没有定义）。一条按未定义的时点
/// 清退的规则会删掉它不该删的记录，而本层无从知道该不该删。
///
/// `contains("expire")` 的**射程**照实写：它逮「任何列名里带 expire 的列」，
/// 逮不住一个叫 `deadline` 的同义列——本层的判据是「有没有过期**这一概念**的落点」，
/// 而列名是开放集，枚举不完。
#[test]
fn a_turn_alias_and_a_session_alias_both_persist_without_an_expiry_column() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let names: Vec<String> = columns(&tx).into_iter().map(|(name, ..)| name).collect();
    assert!(
        !names.iter().any(|name| name.contains("expire")),
        "alias 表不该有过期列（本层不实现会话边界的清退），实得列：{names:?}"
    );

    for &(name, scope) in [("t", AliasScope::Turn), ("s", AliasScope::Session)].iter() {
        save_alias(&tx, &alias(name, "e-1", scope)).unwrap();
        let got = load_alias(&tx, name)
            .unwrap()
            .unwrap_or_else(|| panic!("刚写进去的 {name} 应立刻读得回来（本层不做清退）"));
        assert_eq!(got.scope, scope);
    }

    tx.commit().unwrap();
}
