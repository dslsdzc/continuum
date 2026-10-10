//! `alias` 表（迁移 100）的迁移与行级读写（设计 §4.6；§278 的四作用域落库）。
//!
//! # 本文件与 `alias.rs` 的分工（一处与计划正文不同的落点，理由在此）
//!
//! `alias.rs` 是 **§278 的词汇与记录体**（`AliasScope` 及其编码、`Alias`），
//! 本文件是**表与 I/O**（迁移、`save_alias` / `load_alias` / `list_aliases` / `promote_alias`）。
//! 计划「文件结构」一节**现在**这么分：`src/alias.rs` 记 `AliasScope、Alias、children 式的读函数`，
//! `src/persist.rs` 记「alias 表（迁移 100）、save_alias / load_alias / list_aliases / promote_alias」
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:280-283`）。
//! （**订正 2026-10-10**：原引的旧文把 `promote_alias` 列在 `alias.rs`——**那一行已由 `611c251` 改掉**，
//! 故这里的引文随之陈旧。**引用一处正在被改的地方，改完要回头扫引用**。）
//! **本文件把 `promote_alias` 与另三个 I/O 函数放在一起**（那里把它列在 `alias.rs`）：
//! 它是四条写／读路径里的一条，与 `save_alias` 共享同一列清单与同一张表，
//! 拆开会让「提升改的是哪一列」这件事在文件层面看不出来。
//! **这一处偏离按计划正文选的，不是按本节自己的偏好**——计划那一行是文件清单的速写，
//! 而 Task 7 的 Interfaces 只列函数名、不给家。
//!
//! # 表外取值一律 `Err`，不取默认值
//!
//! `scope` 列是**封闭枚举**（§278 的四值），解码走 [`AliasScope::parse`]；
//! 认不出的串转成 [`AliasError::UnknownScope`] 并**带原始取值**。
//! 这一条是 §4.6 第 1 条「四值封闭（表外取值读回得具体 `Err`，照 P3D 的枚举列纪律）」，
//! **不是**「读侧不重判」那一类例外——它与 `continuum-capability` 的 `tool` 表同一纪律
//! （那里管的是**取值合法性**，登记期不变量是另一件事）。
//!
//! # 编码本体不在本文件
//!
//! `scope` 的字面量取自 [`AliasScope::as_str`] / [`AliasScope::parse`]（与类型同址），
//! `target` 取自 [`EntityId::as_str`] / [`EntityId::from_str`]。**本文件不自建第二份编码**。

use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};

use crate::alias::{Alias, AliasScope};
use crate::candidate::EntityId;
use crate::error::AliasError;

/// 列清单只写一份：两条读路径（按名字、全量）共用同一列序，`row_to_alias` 按它取列。
const COLUMNS: &str = "alias, target, scope";

/// 本 crate 在 `alias` 表上注册的迁移（§4.6 的 ENG-009 闭合点）。
///
/// 编号 **100**：1、2 是 `continuum-persist` 的内建（events、audit_log），10 是 P1 的
/// artifact，20 是 P1 的 graph，30 是 P2 的 workspace，40 是 P2 的 effect，41 是 P2 的
/// policy，50 是 P3 的 capability，80 是 P3D 的模型表。
/// （**订正 2026-10-10**：原写「——八条」，而上面列的是**九个**号；**自指计数不写数值**。）
/// **100 在驱动装配的那个集合里未被占用**（本 task 现场核对：`grep -rn "Migration::new("
/// crates/*/src/` 除本文件那一处外命中的 version 为 1 / 2 / 10 / 20 / 30 / 40 / 41 / 50 / 80，
/// **另加 `continuum-persist/src/bin/crash-writer.rs:17` 的 `60`**——
/// 那是工具 bin，不进装配链，见下一段的干扰项说明。**订正**：原只列了那九个，
/// 与命令的输出不符；**引一条命令就要让它复现得出你列的输出**）。
///
/// **现场还逐条核过两处干扰项**（计划「全局约束」点名的那两处，本 task 复量）：
/// `60` 出现在 `crates/continuum-persist/src/bin/crash-writer.rs:17` 与
/// `crates/continuum-persist/tests/crash_atomicity.rs:9`（表 `crash_fixture`）；
/// `100` 出现在 `crates/continuum-persist/tests/migrations.rs:45`（表 `layer_table`）。
/// **四处都在各自测试／工具的 `Db` 里**（`Db::open_with` 的迁移集由调用方给），
/// **不进生产链**，故它们**不占用**号段。
///
/// **「未被占用」是按库说的，不是全仓**（同 `continuum-capability/src/persist.rs` 的
/// `p3_capability_migrations` 那段订正）：`crates/continuum-persist/tests/migrations.rs:45`
/// 那个 100 就活在全仓里，只是它那个库与本 crate 的库**永不同库**。
///
/// # 三列与它们的约束
///
/// - `alias TEXT PRIMARY KEY NOT NULL`——**键只是名字，不含 scope**。
///   若键取 `(alias, scope)`，「提升」就变成「插入一行、旧行还在」，
///   而 [`promote_alias`] 的一条 `UPDATE` 也就不成立了。
///   `NOT NULL` 是显式写的：SQLite 里 `TEXT PRIMARY KEY` **不**蕴含 `NOT NULL`
///   （只有 `INTEGER PRIMARY KEY` 是 rowid 别名），省掉它就能插入若干条 `alias` 为
///   `NULL` 的行（`NULL` 在唯一索引里两两不相等）。
/// - `target TEXT NOT NULL`——[`EntityId`] 的落库形态（就是它那个串，无额外编码）。
/// - `scope TEXT NOT NULL`——§278 四值的落库形态，封闭（表外取值读回得 `Err`）。
///
/// **没有过期列，也没有 `created_at` 一类的时间列**：§4.6 第 3 条——`TURN`／`SESSION`
/// 的**过期时点规范未给**（「会话结束」没有定义，§16 第 7 条），故本层只实现
/// 「不自动永久化」，**不实现会话边界的清退**。加一列时间戳而没有按它判定的规则，
/// 会让下一个读者以为清退已经实现了。
///
/// **编号 100 的登记（进驱动装配）归 Task 20**：本 task 只交出 `p4_canonical_migrations()`
/// 与 `ALLOWED` 的那条边（见本 crate 的 `Cargo.toml` 与
/// `crates/continuum-runtime/tests/dependency_direction.rs`）。
pub fn p4_canonical_migrations() -> Vec<Migration> {
    vec![Migration::new(
        100,
        "p4_alias",
        "CREATE TABLE alias (
            alias   TEXT PRIMARY KEY NOT NULL,
            target  TEXT NOT NULL,
            scope   TEXT NOT NULL
        );",
    )]
}

/// 插入一条 alias。**不 `commit`**——事务由调用方持有并提交（同 P1 的 `apply_transition`
/// 形状；§318 要求状态更新与对应事件在同一事务内提交，故写函数不自己收口）。
///
/// 写入的 `scope` **就是入参 `a.scope` 那一个**：本函数不替调用方升级作用域
/// （§278 的 MUST NOT 默认把 Session Alias 永久化）。可观察事实由
/// `tests/alias.rs` 的 `save_alias_writes_exactly_the_scope_it_was_given` 钉住：
/// 同一条名字在 `SESSION` 库里 `user_persistent` 行数为 0、在 `USER_PERSISTENT` 库里为 1。
///
/// # 为什么是裸 `INSERT`，不是 `INSERT OR REPLACE`
///
/// 同 id 的第二次写入由**主键**拒绝，判据在库层而非调用方自查。
/// **`INSERT OR REPLACE` 会多出一条本层不许有的路径**：它让第二次写入**静默覆盖**第一次，
/// 于是「改一条 alias 的作用域」可以不经 [`promote_alias`] 发生——
/// 而那正是 §4.6 第 2 条要关掉的那个口子（提升必须是一个独立的显式操作）。
/// 照片：`tests/alias.rs` 的 `saving_the_same_alias_twice_is_rejected`，
/// 它同时断言**被拒之后原行不变**。
pub fn save_alias(tx: &Tx<'_>, a: &Alias) -> Result<(), AliasError> {
    tx.execute(
        "INSERT INTO alias (alias, target, scope) VALUES (?1, ?2, ?3)",
        &[
            Value::text(a.alias.as_str()),
            Value::text(a.target.as_str()),
            Value::text(a.scope.as_str()),
        ],
    )?;
    Ok(())
}

/// 按名字读一条 alias。**不存在返回 `Ok(None)`**（那不是错误，同 `load_tool`）。
///
/// `scope` 是表外取值时返回 [`AliasError::UnknownScope`]，**不取默认值**（见文件头）。
pub fn load_alias(tx: &Tx<'_>, alias: &str) -> Result<Option<Alias>, AliasError> {
    let rows = tx.query(
        &format!("SELECT {COLUMNS} FROM alias WHERE alias = ?1"),
        &[Value::text(alias)],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(row_to_alias(&row)?))
}

/// 全部 alias，按名字升序（顺序固定，调用方才不必自己排）。
///
/// 与 [`load_alias`] 走同一列清单与同一个解码函数；**两条路径各自有表外取值的照片**
/// （`tests/alias.rs` 的 `an_out_of_table_scope_reads_back_as_a_specific_err` 与
/// `list_aliases_rejects_an_out_of_table_scope_too`）——它们共用解码是实现的选择，
/// 不是判据。
pub fn list_aliases(tx: &Tx<'_>) -> Result<Vec<Alias>, AliasError> {
    tx.query(&format!("SELECT {COLUMNS} FROM alias ORDER BY alias"), &[])?
        .iter()
        .map(|row| row_to_alias(row))
        .collect()
}

/// **唯一的提升路径**（§278：MUST NOT 默认把 Session Alias 永久化）。
///
/// 提升 = 把一行的 `scope` 改成 `to`，**其余两个字段不动**（那正是「提升」与
/// 「换一条 alias」的区别）。调用方必须先 [`save_alias`] 过一条记录；
/// 提升一个不存在的名字返回 [`AliasError::NotFound`]——
/// `UPDATE` 匹配不到行时 SQLite 返回 `Ok(0)`，只看「有没有报错」会把拼错的名字
/// 读成一次成功的提升。照片：`tests/alias.rs` 的
/// `promoting_an_alias_that_does_not_exist_is_rejected`。
///
/// # 「没有任何别的函数会顺带调用它」这句话，本处只钉到能钉的那一半
///
/// 设计 §4.6 第 2 条的原话是「且**没有任何函数会顺带调用它**」。本层能拍到的观察是
/// **写侧不升级**（[`save_alias`] 的文档与它的用例）。**整句话还差一个源码扫描**
/// ——照 `crates/continuum-capability/tests/single_writer.rs` 的形态扫本 crate 的 `src/`，
/// 逮住「将来某个函数顺手调一次 `promote_alias`」那一类。**本 task 未落那个扫描**，
/// 故此处照实写明缺口，不把半句话写成整句。
///
/// （**为什么不让 `save_alias` 在 scope 变化时内部调它**：那会让「永久化」在一次普通写入里
/// 静默发生，调用方读自己的代码看不到那一步——§278 要的正是它必须显式。
/// 这也是上面「本文件把 `promote_alias` 放在 `save_alias` 旁边」的理由：紧邻着看，
/// 两者不是同一条路径这件事在源码里就摆着。）
pub fn promote_alias(tx: &Tx<'_>, alias: &str, to: AliasScope) -> Result<(), AliasError> {
    let changed = tx.execute(
        "UPDATE alias SET scope = ?1 WHERE alias = ?2",
        &[Value::text(to.as_str()), Value::text(alias)],
    )?;
    if changed == 0 {
        return Err(AliasError::NotFound {
            alias: alias.to_owned(),
        });
    }
    Ok(())
}

/// 一行 → 一条 alias。列序与 [`COLUMNS`] 一一对应。
///
/// `scope` 的表外取值在此转成具体的 [`AliasError::UnknownScope`]，
/// **带库里那一个原始串**——把认不出的串猜成某一枚作用域，正是 §4.6 第 1 条要拦的。
fn row_to_alias(row: &[Value]) -> Result<Alias, AliasError> {
    let alias = text_at(row, 0)?;
    let target = EntityId::from_str(&text_at(row, 1)?);
    let raw_scope = text_at(row, 2)?;
    let scope = AliasScope::parse(&raw_scope)
        .ok_or_else(|| AliasError::UnknownScope { value: raw_scope })?;
    Ok(Alias {
        alias,
        target,
        scope,
    })
}

/// 按 `index` 取一列文本。**`PersistError::ColumnType` 只在本函数里构造一处**
/// （形态与 `continuum-capability/src/persist.rs` 的 `text_at` 同）。
fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    match row.get(index) {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}
