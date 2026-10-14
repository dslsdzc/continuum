//! §224 的 Contract（Requirement 四 class、`verification_requirement` 禁读）与
//! §226 的版本化（MUST NOT 原地修改、新版本写 `ContractChanges` 审计）的用例集
//! （设计 §7.1／§7.2／§7.3／§5.4）。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `every_requirement_class_has_a_photo` | §224 的四个 class **四值各一条**：写入并读回、断言是**哪一枚**；落库字面量逐枚（小写、多词接 `_`，raw 读列）；class 表外取值读回**具体** `Err` | 四值合并／编码改成大写或去掉 `_`／表外取值取默认值即红 |
//! | `verification_requirement_exposes_no_judging_method` | §7.2 的**结构性禁读**：`VerificationRequirement` 的公开方法集只有 `as_str` / `from_str`。照片 = `tests/compile_fail/verification_requirement_has_no_{is_satisfied,check,evaluate}.rs` 三份反例（`.stderr` 各自实跑取值）＋ `tests/compile_pass/verification_requirement_as_str_is_usable.rs` 正控制（`tests/type_level.rs` 驱动）；本条钉正面的两个方法确实可用 | 补任一枚判定方法名即反例变（编得过 ⇒ `.stderr` 不符） |
//! | `the_layer_has_no_function_that_takes_a_verification_requirement_as_a_judging_input` | **否定式照片**：`src/` 下该类型的**代码出现点逐条登记**（剥注释后扫）——没有以它为输入做判定的函数 | 新增一个以它为参数或返回类型的函数即出现点清单多一行、红；删／改既有出现点同样红（词法扫描的限度写在用例里） |
//! | `the_contract_requirement_table_has_exactly_the_expected_columns` | `PRAGMA table_info(contract_requirement)` 实际列 vs 手写期望**逐列**（Requirement 的五个字段名照 §224） | 加／少／改一列（含改类型／`NOT NULL`／主键位次）即红 |
//! | `the_contract_table_has_exactly_the_expected_columns` | 同上，`contract` 表（TaskContract 的九个落库列） | 同上 |
//! | `a_second_write_of_the_same_id_and_version_hits_the_primary_key` | §226 的 MUST NOT 原地修改：同 `(id, version)` 二次写得**撞主键的 `Err`**，**且原行逐列不变** | `INSERT OR REPLACE`／先删后插（**移除档**）——第二次写变 `Ok` 或行被覆写 |
//! | `a_duplicate_requirement_id_in_one_version_hits_the_key` | 同一个 save 里的重复 requirement id 撞 `contract_requirement` 的复合主键、得 `Err`，且回滚后两表无半写行 | 改成 `INSERT OR IGNORE`／`OR REPLACE`（**放宽档**）——save 变 `Ok` 或只剩一行 |
//! | `a_missing_contract_reads_back_as_none` | `load_contract` 的「不存在」处置：从没写过的 id、写过 v1 但没写过的 v2，都读回 `Ok(None)`（不是 `Err`、不是空记录） | 变成一枚 `Err`（**收紧档**）或 `Ok(Some(空记录))` 即红 |
//! | `a_new_version_writes_exactly_one_contract_changes_audit_row` | §5.4：断言 `AuditKind` **是 `ContractChanges`**、每个新版本**恰一条**、且与 INSERT **同一事务**（失败路径：审计写失败 ⇒ `contract` 表**无半写行**） | kind 写错（**取反档**）／写两条或不写／审计与 INSERT 拆成两个事务（**移除档**，(c) 红） |
//! | `the_other_fields_of_224_are_opaque` | `TaskContract` 的其余字段（六个名单）字段清单**逐项**（穷尽解构，**不带 `..`**——加／少／改名即编译不过）＋每个字段的表外取值逐字节往返 | 任一字段被裁剪／改写／与邻字段写串即红；字段加／少即**编译不过** |
//! | `requirement_ids_are_not_rewritten_trimmed_or_rejected` | `RequirementId` 的「不改写、不裁剪、无校验」：带空白／特殊字符／非 ASCII 的 id（含空串）经 `contract_requirement` 表**逐字节**往返 | 任一 `from_str` 裁剪／改写输入即红 |
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/spec.rs` 的夹具同形（P0 内建迁移 ＋ 本 crate 迁移 110，
//! 建库后必须 `migrate()`）；`new_db_without_audit()` 的造法照 `tests/intent_state.rs`
//! 的 `new_db_without_events`（迁移集里去掉那张表，于是写审计必然失败）。
//! `sample_contract` / `sample_requirement` 都是**每个字段取一个非默认值**的基准值：
//! 「哪一列没落库／与邻列写串」这类错在读回时的逐项比对里现形。
//!
//! `save_contract` 的第三个参数是 `occurred_at`——见 `src/persist.rs` 的
//! [`continuum_semantics::save_contract`] 文档：本层不取时钟，审计时刻由调用方给
//! （签名比任务简报的速写多这一个参数，先例是 Task 9 的 `apply_intent_transition`）。

use std::fs;
use std::path::{Path, PathBuf};

use continuum_canonical::Opaque;
use continuum_persist::{AuditKind, Db, Migration, PersistError, Tx, Value, builtin_migrations};
use continuum_semantics::{
    ContractError, ContractId, IntentId, Requirement, RequirementClass, RequirementId,
    TaskContract, VerificationRequirement, load_contract, p4_semantics_migrations, save_contract,
};

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移的临时库（与 `tests/spec.rs` 的夹具同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 只装本 crate 迁移的库：`contract` / `contract_requirement` 表在，
/// **`audit_log` 表不存在**（内建迁移 2 建它）。
///
/// 失败路径的造法照 `tests/intent_state.rs` 的 `new_db_without_events`（迁移集里去掉
/// 那张表，于是写审计必然失败），比伪造一个失败的 `Tx` 更省事，也仍是真实的数据库错误。
fn new_db_without_audit() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_with(&dir.path().join("t.db"), p4_semantics_migrations()).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条基准 Requirement：五个字段各取一个非默认值。
fn sample_requirement(id: &str, class: RequirementClass) -> Requirement {
    Requirement {
        id: RequirementId::from_str(id),
        class,
        expression: Opaque::from_str(&format!("expr-{id}")),
        source: Opaque::from_str(&format!("source-{id}")),
        verification_requirement: VerificationRequirement::from_str(&format!("vr-{id}")),
    }
}

/// 一条基准 TaskContract：每个字段都取一个非默认值（六个 Opaque 字段各不同）。
///
/// `requirements` 两条，**写入顺序与 id 升序相反**（先 `r-2` 后 `r-1`）——
/// 这样「读回按写入顺序（rowid）」这句话才有判别力（改成 `ORDER BY id` 即红）。
fn sample_contract(id: &str, version: u32) -> TaskContract {
    TaskContract {
        id: ContractId::from_str(id),
        version,
        intent_id: IntentId::from_str(&format!("i-for-{id}")),
        requirements: vec![
            sample_requirement("r-2", RequirementClass::Required),
            sample_requirement("r-1", RequirementClass::Preferred),
        ],
        budget: Opaque::from_str("budget-a"),
        authority_limit: Opaque::from_str("authority-a"),
        data_policy: Opaque::from_str("data-policy-a"),
        model_policy: Opaque::from_str("model-policy-a"),
        node_policy: Opaque::from_str("node-policy-a"),
        acceptance_predicate: Opaque::from_str("accept-a"),
    }
}

/// 读回一条 TaskContract；不存在直接炸（用例里它必须存在）。
fn load(tx: &Tx<'_>, id: &str, version: u32) -> TaskContract {
    load_contract(tx, &ContractId::from_str(id), version)
        .unwrap()
        .unwrap_or_else(|| panic!("库里应有 {id:?} 的 v{version} contract"))
}

/// `contract` 表的行数。
fn contract_row_count(tx: &Tx<'_>) -> i64 {
    match &tx.query("SELECT COUNT(*) FROM contract", &[]).unwrap()[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// `contract_requirement` 表的行数。
fn requirement_row_count(tx: &Tx<'_>) -> i64 {
    match &tx.query("SELECT COUNT(*) FROM contract_requirement", &[]).unwrap()[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// `contract` 表一行的**全部列**（raw `SELECT *`，列序 = 建表序）——
/// 「撞键后原行逐列不变」的判据（经公开读函数比不出被改掉的列）。
fn raw_contract_row(tx: &Tx<'_>, id: &str, version: u32) -> Vec<Value> {
    let rows = tx
        .query(
            "SELECT * FROM contract WHERE id = ?1 AND version = ?2",
            &[Value::text(id), Value::Int(i64::from(version))],
        )
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "contract 表里应有恰一行 ({id:?}, v{version})，实得 {} 行",
        rows.len()
    );
    rows.into_iter().next().unwrap()
}

/// `contract_requirement` 表某个版本的**全部行**（raw `SELECT *`，按 rowid）。
fn raw_requirement_rows(tx: &Tx<'_>, id: &str, version: u32) -> Vec<Vec<Value>> {
    tx.query(
        "SELECT * FROM contract_requirement WHERE contract_id = ?1 AND contract_version = ?2 \
         ORDER BY rowid",
        &[Value::text(id), Value::Int(i64::from(version))],
    )
    .unwrap()
}

/// `contract_requirement` 表某一行的某一列**原始文本**（绕开全部公开读函数）。
///
/// 这是编码纪律（小写、多词接 `_`）的唯一照片：经 `load_contract` 读回来的是已经过
/// `parse` 的值，`as_str` 与 `parse` 一起错（例如把 `required` 也编成 `preferred`）时
/// 它是看不出来的。`column` 只取本文件里写死的字面量，不是外部输入。
fn raw_requirement_text(
    tx: &Tx<'_>,
    id: &str,
    version: u32,
    requirement_id: &str,
    column: &str,
) -> String {
    let rows = tx
        .query(
            &format!(
                "SELECT {column} FROM contract_requirement \
                 WHERE contract_id = ?1 AND contract_version = ?2 AND id = ?3"
            ),
            &[
                Value::text(id),
                Value::Int(i64::from(version)),
                Value::text(requirement_id),
            ],
        )
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "contract_requirement 表里应有恰一行 ({id:?}, v{version}, {requirement_id:?})，\
         实得 {} 行",
        rows.len()
    );
    match &rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("{column} 列应是文本，实得 {other:?}"),
    }
}

/// `audit_log` 表里 `seq` 那一行的 `kind` 列**原始文本**。
fn raw_audit_kind(tx: &Tx<'_>, seq: i64) -> String {
    let rows = tx
        .query("SELECT kind FROM audit_log WHERE seq = ?1", &[Value::Int(seq)])
        .unwrap();
    assert_eq!(rows.len(), 1, "audit_log 里应有恰一行 seq={seq}，实得 {} 行", rows.len());
    match &rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("kind 列应是文本，实得 {other:?}"),
    }
}

/// 运行期从库里读出的列清单：`(列名, 声明类型, NOT NULL, 主键位次)`。
///
/// `table` 只取本文件里写死的字面量（`contract` / `contract_requirement`），不是外部输入。
/// 形态与 `tests/intent.rs` 的同名辅助函数同。
fn columns(tx: &Tx<'_>, table: &str) -> Vec<(String, String, i64, i64)> {
    tx.query(&format!("PRAGMA table_info({table})"), &[])
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

/// §224 的四个 class，**四值各一条**：写入并读回，断言读回的是**哪一枚**。
///
/// 外加两件不是「顺带」的事：
///
/// - **落库字面量逐枚**（raw 读列）——只断言往返一致是不够的：`as_str` 与 `parse`
///   互为逆函数时，一张把两枚都编成同一个串的表照样往返得通（同 `tests/intent.rs`
///   的 `enum_columns_…` 的理由）；
/// - **表外取值读回得具体 `Err`**（`ContractError::UnknownClass`，带库里那一个原始串、
///   不取默认值）——枚举列纪律的读侧（Task 10 的 fix 后形态）。
///
/// 读回顺序按**写入顺序**（`load_contract` 的文档）：夹具的写入序与 id 升序相反，
/// 故下面的逐项断言同时钉住「顺序没有被换成 id 序」。
///
/// **红条件**：四值合并／编码改成大写或去掉 `_`／表外取值取默认值／顺序改成 id 序即红。
#[test]
fn every_requirement_class_has_a_photo() {
    let (_d, db) = new_db();

    let classes = [
        (RequirementClass::Required, "required"),
        (RequirementClass::Preferred, "preferred"),
        (RequirementClass::Flexible, "flexible"),
        (RequirementClass::Unspecified, "unspecified"),
    ];

    // 四值各写一次——同一条 contract 里四条 requirement，id 序与写入序相反。
    let tx = db.begin().unwrap();
    let mut contract = sample_contract("c-class", 1);
    contract.requirements = vec![
        sample_requirement("r-4", RequirementClass::Required),
        sample_requirement("r-3", RequirementClass::Preferred),
        sample_requirement("r-2", RequirementClass::Flexible),
        sample_requirement("r-1", RequirementClass::Unspecified),
    ];
    save_contract(&tx, &contract, 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load(&tx, "c-class", 1);
    assert_eq!(back.requirements.len(), 4, "四条 requirement 应全部读回");
    let read_ids: Vec<&str> = back.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        read_ids,
        vec!["r-4", "r-3", "r-2", "r-1"],
        "requirements 应按写入顺序读回（§224 未给顺序语义；读法是行写入顺序，不是 id 升序）"
    );
    for (requirement, (class, literal)) in back.requirements.iter().zip(classes.iter()) {
        assert_eq!(
            requirement.class, *class,
            "写的是 {class:?}，读回的应是那一枚（四值逐枚，不抽代表）"
        );
        assert_eq!(
            raw_requirement_text(&tx, "c-class", 1, requirement.id.as_str(), "class"),
            *literal,
            "`{class:?}` 的落库编码应为小写、多词接 `_`"
        );
    }

    // 表外取值：绕开公开写路径直接把一列改坏（模拟历史行／被外部改过的行）。
    tx.execute(
        "UPDATE contract_requirement SET class = ?1 \
         WHERE contract_id = ?2 AND contract_version = ?3 AND id = ?4",
        &[
            Value::text("nonsense"),
            Value::text("c-class"),
            Value::Int(1),
            Value::text("r-4"),
        ],
    )
    .unwrap();
    assert_eq!(
        load_contract(&tx, &ContractId::from_str("c-class"), 1).unwrap_err(),
        ContractError::UnknownClass {
            value: "nonsense".to_owned()
        },
        "class 的表外取值应读回**具体**错误、并带库里那一个原始串（不取默认值）"
    );
    tx.commit().unwrap();
}

/// §7.2 的**结构性禁读**：`VerificationRequirement` 的公开方法集**只有** `as_str` /
/// `from_str`——没有 `is_satisfied` / `check` / `evaluate` 这类方法。
///
/// # 为什么（写在这里，因为这条禁令的理由是判据的一部分）
///
/// 《工程》§2.5 明写：该字段**可以存储**，但在 OPEN-001 解决前**无法判定其是否被满足**
/// （「这不阻断本层建设，阻断的是第 8 层的验证收口」）。OPEN-001 的三个可选方向
/// （a/b/c）**一个都不选**——补任何一个满足判据都是发明判据。故本层的落点是
/// 「只存不判」：字段完整保存，判定等第 8 层。
///
/// # 照片机制（三个半边，合起来才是「方法集只有这两个」的可判形态）
///
/// 1. **反例三份**（`tests/compile_fail/`，`tests/type_level.rs` 的 `t.compile_fail` 驱动）：
///    `verification_requirement_has_no_is_satisfied.rs` / `…_no_check.rs` /
///    `…_no_evaluate.rs`——`vr.is_satisfied()` / `vr.check(..)` / `vr.evaluate(..)`
///    各**编不过**，`.stderr` 实跑取值、逐字比对（「因拼错函数名而编译失败」不会变绿）。
/// 2. **正控制一份**（`tests/compile_pass/verification_requirement_as_str_is_usable.rs`）：
///    `from_str` / `as_str` 编得过——没有它，三份反例的绿可以来自「这个名字整个不存在」
///    这类与判据无关的原因（恒绿的守卫比没有更坏）。
/// 3. **运行期（本条）**：两个方法各用一次、逐字节往返——正面的公开面确实可用。
///
/// # 限度（与三份反例同宽）
///
/// 反例钉的是**那三个名字**：方法名是开放集，`trybuild` 枚举不完。更宽的那半句
/// （「本层没有任何判定方法／以它为输入做判定的函数」）的可判形态在**下一条用例**
/// （`src/` 的出现点逐条登记）与 Task 21 的复核里。
///
/// **红条件**：补任一枚判定方法（那三份反例的 `.stderr` 随之不符、即红）；
/// `as_str` 被裁剪／改写（本条的往返断言红）。
#[test]
fn verification_requirement_exposes_no_judging_method() {
    let raw = "任意串都存下来：空白 与 unicode ✓ 与 \"引号\" 与 换行\n也原样";
    let vr = VerificationRequirement::from_str(raw);
    assert_eq!(
        vr.as_str(),
        raw,
        "`from_str` / `as_str` 应逐字节往返（公开面只有这两个方法，见用例文档）"
    );
}

/// **否定式照片**（与上一条合起来是本组件的两侧）：`src/` 里**没有**以
/// `VerificationRequirement` 为输入做判定的函数。
///
/// # 机制与它的限度（写在用例里）
///
/// 设计 §7.2 写「本层**没有任何函数**以它为输入做判定（……它只被读写与透传）」。
/// 「没有任何函数」是**开放集**——枚举不完（同 `continuum-canonical/tests/risk.rs`
/// 的 `from_risk` 守卫的自述）。本用例的可判形态是**出现点逐条登记**：
/// 扫本 crate 的 `src/`（剥注释的代码行），把含 `VerificationRequirement` 的每一行
/// 归一化空白后与**写死的允许清单**逐条比较：
///
/// - 新增一个**以它为参数或返回类型**的函数（例如
///   `fn judge(vr: &VerificationRequirement) -> Verdict`），其行必然含该类型名、
///   不在清单里 ⇒ **红**；
/// - 删掉／改写既有出现点（例如把字段类型换成裸 `String`）⇒ 清单对不上 ⇒ **红**。
///
/// **它不钉什么**：(a) `impl VerificationRequirement` 块里的 `&self` 方法
/// （方法定义行不含类型名，扫不到）——那半边由上一条的三份反例（三个具体名字）承担；
/// (b) 「判定」与否不在本扫描的词汇表里——它保证的是**每个出现点都被人工复核并登记**，
/// 而不是自动识别「判定」。两者合起来 + Task 21 的复核，才覆盖 §7.2 那句话。
///
/// **红条件**：见上（出现点集合 ≠ 清单即红）。正常修改（如文档措辞）不红——
/// 只比代码行。
#[test]
fn the_layer_has_no_function_that_takes_a_verification_requirement_as_a_judging_input() {
    let sources = crate_sources();
    // 先证明扫描真的读到了这棵树（否则下面的清单断言在空语料上恒真、恒绿）。
    assert!(
        sources.len() >= 5,
        "只扫到 {} 个 `src/` 下的 `.rs`：源码树没被读到",
        sources.len()
    );
    let joined: String = sources
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("pub struct VerificationRequirement"),
        "读到了 {} 个文件，但拼起来找不到 `pub struct VerificationRequirement`——\
         读到的不是这棵树",
        sources.len()
    );

    // 出现点：含 `VerificationRequirement` 的代码行，归一化空白。
    let mut occurrences: Vec<String> = Vec::new();
    for (path, body) in &sources {
        let code = strip_comments(body);
        for line in code.lines() {
            if line.contains("VerificationRequirement") {
                let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
                let normalized = normalized.trim().to_owned();
                if !normalized.is_empty() {
                    occurrences.push(format!("{}: {normalized}", path.display()));
                }
            }
        }
    }
    occurrences.sort();
    occurrences.dedup();

    // 允许清单（逐条，**按字符串序**——与上面 sort 后的实得集合逐项比对）——每条都写明
    // 用途；新增一行意味着多了一个**需要人工复核**的出现点：确认它不是「以它为输入做判定」。
    let expected = [
        // `contract.rs`：`from_str` 的构造行。
        "src/contract.rs: VerificationRequirement(s.to_owned())",
        // `contract.rs`：类型定义与 impl 块本体。
        "src/contract.rs: impl VerificationRequirement {",
        "src/contract.rs: pub struct VerificationRequirement(String);",
        // `contract.rs`：`Requirement` 的字段声明（类型名出现在这里）。
        "src/contract.rs: pub verification_requirement: VerificationRequirement,",
        // `decision.rs`：`RequirementUpdate` 的字段声明（`Option` 承载「不改」——
        // 它是 patch 的**值**，不是判定输入）。
        "src/decision.rs: pub verification_requirement: Option<VerificationRequirement>,",
        // `decision.rs`：import（Task 15 的 `RequirementUpdate` 要用这个类型）。
        "src/decision.rs: use crate::contract::{RequirementClass, VerificationRequirement};",
        // `lib.rs`：re-export（crate 外可命名这个类型）。
        "src/lib.rs: pub use contract::{Requirement, RequirementClass, TaskContract, VerificationRequirement};",
        // `persist.rs`：import。
        "src/persist.rs: use crate::contract::{Requirement, RequirementClass, TaskContract, VerificationRequirement};",
        // `persist.rs`：读侧解码。写侧用的是字段名 `verification_requirement`（小写）——
        // 不匹配类型名，行内没有类型名（它是 `insert_requirement` 的列绑定）。
        "src/persist.rs: verification_requirement: VerificationRequirement::from_str(&text_at(row, 4)?),",
    ];
    let expected: Vec<String> = expected.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        occurrences, expected,
        "`src/` 里含 `VerificationRequirement` 的代码出现点应逐条等于登记清单：\
         新增出现点（如一个以它为参数/返回类型的函数）要先人工复核用途，再把它加进清单；\
         移除/改写了既有出现点则要更新对应行"
    );
}

/// `contract_requirement` 表的列清单**逐列**：实际 vs 手写期望。
///
/// Requirement 的五个字段名照 §224：`id` / `class` / `expression` / `source` /
/// `verification_requirement`；另两列 `contract_id` / `contract_version` 是**归属键**
/// （把 requirement 钉在一条 contract 的具体版本上，同 `specification` 表的
/// `intent_id` ＋ `version` 的处置）——它们连同 `id` 构成复合主键。
///
/// 手写期望与迁移 110 的 SQL 是**两份**（这正是本用例的判别力所在：改了一处、
/// 忘了另一处即红）。**红条件**：加／少／改一列（含改类型、改 `NOT NULL`、
/// 改主键位次）即红。
#[test]
fn the_contract_requirement_table_has_exactly_the_expected_columns() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let expected: Vec<(&str, &str, i64, i64)> = vec![
        ("contract_id", "TEXT", 1, 1),
        ("contract_version", "INTEGER", 1, 2),
        ("id", "TEXT", 1, 3),
        ("class", "TEXT", 1, 0),
        ("expression", "TEXT", 1, 0),
        ("source", "TEXT", 1, 0),
        ("verification_requirement", "TEXT", 1, 0),
    ];
    let expected: Vec<(String, String, i64, i64)> = expected
        .into_iter()
        .map(|(n, t, nn, pk)| (n.to_owned(), t.to_owned(), nn, pk))
        .collect();

    assert_eq!(
        columns(&tx, "contract_requirement"),
        expected,
        "contract_requirement 表的列清单与手写期望不符\
         （列名／类型／NOT NULL／主键位次任一不同即红）"
    );

    tx.commit().unwrap();
}

/// `contract` 表的列清单**逐列**（TaskContract 的九个落库列）——判据与上一条逐字相同
/// （`requirements[]` 不落本表，落 `contract_requirement`；`preferences[]` /
/// `prohibitions[]` 未建，见 `src/contract.rs` 的 `TaskContract` 文档）。
#[test]
fn the_contract_table_has_exactly_the_expected_columns() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let expected: Vec<(&str, &str, i64, i64)> = vec![
        ("id", "TEXT", 1, 1),
        ("version", "INTEGER", 1, 2),
        ("intent_id", "TEXT", 1, 0),
        ("budget", "TEXT", 1, 0),
        ("authority_limit", "TEXT", 1, 0),
        ("data_policy", "TEXT", 1, 0),
        ("model_policy", "TEXT", 1, 0),
        ("node_policy", "TEXT", 1, 0),
        ("acceptance_predicate", "TEXT", 1, 0),
    ];
    let expected: Vec<(String, String, i64, i64)> = expected
        .into_iter()
        .map(|(n, t, nn, pk)| (n.to_owned(), t.to_owned(), nn, pk))
        .collect();

    assert_eq!(
        columns(&tx, "contract"),
        expected,
        "contract 表的列清单与手写期望不符（列名／类型／NOT NULL／主键位次任一不同即红）"
    );

    tx.commit().unwrap();
}

/// §226 的「MUST NOT 原地修改已有版本」：同一 `(id, version)` 的第二次写入
/// **被复合主键拒绝**，且**原行逐列不变**。
///
/// # 判据两道
///
/// 1. 第二次 `save_contract` 得**撞主键的 `Err`**——实现里是裸 `INSERT`，不是
///    `INSERT OR REPLACE`。后者会让第二次写静默成功并把 v1 覆写掉，那正是 §226 禁的
///    「原地修改」（**红条件：移除档**）；
/// 2. **原行逐列不变**——只断言 `Err` 是不够的：一次「先删后插」的变体可能返回错误
///    却已把行改掉。`contract` 行与 `contract_requirement` 行都按 raw `SELECT *` 取，
///    逐列比对。
///
/// 第二次写用的是**内容不同**的同版本记录（预算与 requirement 都换过），这样
/// 「行没变」这句话才有内容上的判别力。**新版本（version + 1）是另一条正路**——
/// 它的照片在下一条用例（v1 → v2 各写一条审计）。
#[test]
fn a_second_write_of_the_same_id_and_version_hits_the_primary_key() {
    let (_d, db) = new_db();

    let tx = db.begin().unwrap();
    save_contract(&tx, &sample_contract("c-1", 1), 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let before_contract = raw_contract_row(&tx, "c-1", 1);
    let before_requirements = raw_requirement_rows(&tx, "c-1", 1);
    assert_eq!(
        before_requirements.len(),
        2,
        "起点应是两条 requirement 行（后面「逐列不变」才有判别力）"
    );
    tx.commit().unwrap();

    // 同 (c-1, v1) 的第二次写入：内容不同、版本号相同。
    let tx = db.begin().unwrap();
    let mut second = sample_contract("c-1", 1);
    second.budget = Opaque::from_str("SHOULD-NOT-LAND");
    second.requirements = vec![sample_requirement("r-9", RequirementClass::Flexible)];
    let err = save_contract(&tx, &second, 2_000).unwrap_err();
    assert!(
        matches!(&err, ContractError::Persist(PersistError::Database(m))
            if m.contains("UNIQUE constraint failed") && m.contains("contract")),
        "期望撞主键的 Persist（UNIQUE constraint failed: contract.…），实得 {err:?}"
    );
    // 调用方在 `Err` 后回滚（丢弃 Tx）——「不得原地修改」的落点：这次写入
    // 要么整体不存在、要么撞键被拒。
    drop(tx);

    let tx = db.begin().unwrap();
    assert_eq!(
        raw_contract_row(&tx, "c-1", 1),
        before_contract,
        "撞键之后原 contract 行应逐列不变（不是「报了个错、行也被改了」）"
    );
    assert_eq!(
        raw_requirement_rows(&tx, "c-1", 1),
        before_requirements,
        "撞键之后原 contract_requirement 行集应逐列不变"
    );
    tx.commit().unwrap();
}

/// 同一个 `save_contract` 里的**重复 requirement id** 撞 `contract_requirement` 的
/// 复合主键、得 `Err`——不会静默产生两行。
///
/// # 为什么这也是一条「裸 INSERT 不 REPLACE」的照片
///
/// 写入次序上它与上一条同族：`insert_requirement` 是裸 `INSERT`。§224 的
/// `requirements[]` 里 `id` 是键——**两条同 id 是坏数据，不是「两个元素」**
/// （同一条契约的同一版本上两条 same-id 的 requirement 没有可分辨的语义）。
/// 拒绝在**第二个 requirement 的 INSERT** 上发生：`contract` 行与第一条 requirement 行
/// 已在事务里，调用方回滚后两表都没有半写行。
///
/// **红条件**：改成 `INSERT OR IGNORE`（**放宽档**——第二行被静默丢掉、`save` 得 `Ok`）
/// 或 `OR REPLACE`（第一行被第二行覆盖、`save` 得 `Ok` 且库里只剩一行）即红。
#[test]
fn a_duplicate_requirement_id_in_one_version_hits_the_key() {
    let (_d, db) = new_db();

    let tx = db.begin().unwrap();
    let mut doubled = sample_contract("c-dup", 1);
    doubled.requirements = vec![
        sample_requirement("r-x", RequirementClass::Required),
        sample_requirement("r-x", RequirementClass::Preferred),
    ];
    let err = save_contract(&tx, &doubled, 1_000).unwrap_err();
    assert!(
        matches!(&err, ContractError::Persist(PersistError::Database(m))
            if m.contains("UNIQUE constraint failed") && m.contains("contract_requirement")),
        "同一版本里两条同 id 应撞 contract_requirement 的复合主键，实得 {err:?}"
    );
    // 调用方在 `Err` 后回滚（丢弃 Tx）。
    drop(tx);

    let tx = db.begin().unwrap();
    assert_eq!(contract_row_count(&tx), 0, "回滚后 contract 表应无半写行");
    assert_eq!(
        requirement_row_count(&tx),
        0,
        "回滚后 contract_requirement 表应无半写行"
    );
    tx.commit().unwrap();
}

/// `load_contract` 对「不存在」的处置：**`Ok(None)`，不是错误**——同 [`load_intent`] /
/// [`load_specification`]（读一个不存在的记录是「没有」，不是「出错」；后者——枚举列坏、
/// 容器坏——才得 `Err`，见 `every_requirement_class_has_a_photo`）。
///
/// 两个面都钉：**从没写过**的 id 得 `None`；**写过 v1 但没写过 v2** 也按版本得 `None`
/// ——版本化是「哪个版本读哪个版本」，「最近一版」由调用方自己先算（写在 `load_contract`
/// 的文档里，不做回退）。
///
/// **红条件**：把「不存在」变成一枚 `Err`（**收紧档**）即红；把它换成
/// `Ok(Some(空记录))`（静默造一条没写过的契约）也红。
#[test]
fn a_missing_contract_reads_back_as_none() {
    let (_d, db) = new_db();

    let tx = db.begin().unwrap();
    assert!(
        load_contract(&tx, &ContractId::from_str("c-none"), 1)
            .unwrap()
            .is_none(),
        "从没写过的 id 应读回 None（不是 Err、不是空记录）"
    );

    save_contract(&tx, &sample_contract("c-none", 1), 1_000).unwrap();
    assert!(
        load_contract(&tx, &ContractId::from_str("c-none"), 1)
            .unwrap()
            .is_some(),
        "写过的 v1 应读得到"
    );
    assert!(
        load_contract(&tx, &ContractId::from_str("c-none"), 2)
            .unwrap()
            .is_none(),
        "没写过的 v2 应读回 None——版本化不做「最近一版」回退"
    );
    tx.commit().unwrap();
}

/// §5.4 的写入点之一：Contract 新版本创建 → 恰一条 `AuditKind::ContractChanges` 审计，
/// **与 INSERT 同一事务**（设计 §7.3＋§5.4）。
///
/// # 断言三件
///
/// (a) kind **是 `ContractChanges`**（不是「写了审计」）——经 `Tx::audit_records()`
///     读回 `AuditKind` 那一枚，并断言 `audit_log.kind` 的**原始文本**是
///     `"contract changes"`（§313 的归类字面量；写侧 `as_str` 与读侧 serde 表示
///     是同一份的两条路径）；
/// (b) **每个新版本恰一条**——写 v1、v2，共恰两条，payload 指向各自的
///     `(contract_id, version)`；
/// (c) **同一事务**——失败路径：`audit_log` 表不存在时 `save_contract` 得具体 `Err`，
///     回滚后 `contract` 表与 `contract_requirement` 表**都无半写行**（行是在
///     INSERT 之后才失败的，撤得掉才算同一事务）。
///
/// # 并写明（§5.4）
///
/// `EventType` / `AuditKind` 的**变体一个不改、一张表不加**——本层只使用既有的六个
/// 写入点（这里是 `AuditKind::ContractChanges`），不新增枚举变体、不新增表。
///
/// **红条件**：kind 写错（**取反档**）／写两条或不写／审计与 INSERT 拆成两个事务
/// （**移除档**，(c) 红）。
#[test]
fn a_new_version_writes_exactly_one_contract_changes_audit_row() {
    // (a) + (b)
    let (_d, db) = new_db();

    let tx = db.begin().unwrap();
    save_contract(&tx, &sample_contract("c-1", 1), 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    save_contract(&tx, &sample_contract("c-1", 2), 2_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let records = tx.audit_records().unwrap();
    assert_eq!(records.len(), 2, "两个新版本应恰各写一条审计（不是「写了审计」）");
    for (i, record) in records.iter().enumerate() {
        assert_eq!(
            record.kind,
            AuditKind::ContractChanges,
            "第 {i} 条审计的 kind 应是 `ContractChanges` 那一枚"
        );
    }
    assert_eq!(
        raw_audit_kind(&tx, records[0].seq),
        "contract changes",
        "落库字面量应是 §313 的归类字面量（写侧 `as_str` 与读侧 serde 表示同源）"
    );
    // payload 指向各自的 (contract_id, version)。
    assert_eq!(
        records[0].payload.get("contract_id").and_then(|v| v.as_str()),
        Some("c-1")
    );
    assert_eq!(records[0].payload.get("version").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        records[1].payload.get("contract_id").and_then(|v| v.as_str()),
        Some("c-1")
    );
    assert_eq!(records[1].payload.get("version").and_then(|v| v.as_i64()), Some(2));
    tx.commit().unwrap();

    // (c) 失败路径：审计写失败 ⇒ 回滚后两张表都没有半写的行。
    let (_d2, db) = new_db_without_audit();
    let tx = db.begin().unwrap();
    let err = save_contract(&tx, &sample_contract("c-1", 1), 1_000).unwrap_err();
    assert!(
        matches!(&err, ContractError::Persist(PersistError::Database(m)) if m.contains("audit_log")),
        "(c) 期望具体 Err 是「audit_log 表缺失」的 Persist，实得 {err:?}"
    );
    // 调用方在 `Err` 后回滚（丢弃 Tx）。
    drop(tx);

    let tx = db.begin().unwrap();
    assert_eq!(contract_row_count(&tx), 0, "(c) 审计写失败后回滚，contract 表应无半写行");
    assert_eq!(
        requirement_row_count(&tx),
        0,
        "(c) contract_requirement 表同（INSERT 在审计之前，撤得掉才算同一事务）"
    );
    tx.commit().unwrap();
}

/// §224 的 `TaskContract` 的其余字段**规范只给名字**，本层一律取不透明值
/// （[`Opaque`]，定义收在 `continuum-canonical`）：字段清单**逐项**（穷尽解构、
/// **不带 `..`**——加／少／改名即编译不过），且每个字段的表外取值**逐字节往返**。
///
/// # 字段清单（§224 的 12 个规范字段 → 本结构体的 10 个成员）
///
/// `id` / `version` / `intent_id` / `requirements[]` 有明确类型（`ContractId` / `u32` /
/// `IntentId` / `Vec<Requirement>`，后者的五个字段在别的用例逐项钉）；
/// `budget` / `authority_limit` / `data_policy` / `model_policy` / `node_policy` /
/// `acceptance_predicate` 六个取 `Opaque`。**差额申报**：§224 的 `preferences[]` /
/// `prohibitions[]` 两个列表字段**未建**——设计 §7.1（与任务简报）对它们的元素结构、
/// 落库形态均无处置、下游也无消费者，本 task 不发明（完整理由写在 `src/contract.rs`
/// 的 `TaskContract` 文档里）。
///
/// # 「表外取值」在本用例里
///
/// 六个不透明字段没有词表（「规范只给名字」），故「表外」= 任何本层不解释的串
/// （带空白／特殊字符／非 ASCII）。每个字段各给一个**不同的**串，逐字节往返——
/// 写串列（把两个字段的值调换）或裁剪／改写输入即红。
///
/// # 并写明：§224 的 `budget` 与 §12 的预算**不是同一个东西**
///
/// 前者是 Contract 里的**声明字段**（本层只保存），§333 的 `Budget` 是**记账对象**；
/// 两者的关系规范没给（设计 §16 第 11 条：缺的是「Contract 的 `budget` 怎么变成
/// 树上的一次分配」这一步）。**本层不建那个桥**——本 crate 连 `continuum-budget`
/// 的依赖边都没有（§11.5 的第 1 条守卫）。
///
/// **红条件**：任一字段被裁剪／改写／与邻字段写串即红；字段加／少即**编译不过**
/// （穷尽解构不带 `..`）。
#[test]
fn the_other_fields_of_224_are_opaque() {
    let (_d, db) = new_db();

    // 六个字段各一个互不相同的「表外」串。
    let budget_text = "预算：含空格 与 unicode ✓ 与 \"引号\"";
    let authority_text = "authority-limit/表外 取值";
    let data_policy_text = "data-policy\t带制表符";
    let model_policy_text = "model-policy\n带换行";
    let node_policy_text = "node-policy:任意串";
    let acceptance_text = "acceptance-predicate，结束";

    let tx = db.begin().unwrap();
    save_contract(
        &tx,
        &TaskContract {
            id: ContractId::from_str("c-opaque"),
            version: 1,
            intent_id: IntentId::from_str("i-opaque"),
            requirements: vec![
                sample_requirement("r-2", RequirementClass::Required),
                sample_requirement("r-1", RequirementClass::Preferred),
            ],
            budget: Opaque::from_str(budget_text),
            authority_limit: Opaque::from_str(authority_text),
            data_policy: Opaque::from_str(data_policy_text),
            model_policy: Opaque::from_str(model_policy_text),
            node_policy: Opaque::from_str(node_policy_text),
            acceptance_predicate: Opaque::from_str(acceptance_text),
        },
        1_000,
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load(&tx, "c-opaque", 1);

    // 字段清单逐项 = 穷尽解构、**不带 `..`**：加／少／改名即编译不过。
    let TaskContract {
        id,
        version,
        intent_id,
        requirements,
        budget,
        authority_limit,
        data_policy,
        model_policy,
        node_policy,
        acceptance_predicate,
    } = &back;
    assert_eq!(id.as_str(), "c-opaque");
    assert_eq!(*version, 1);
    assert_eq!(intent_id.as_str(), "i-opaque");
    assert_eq!(requirements.len(), 2, "两条 requirement 也要读回（顺序按写入序）");
    assert_eq!(
        requirements.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec!["r-2", "r-1"]
    );
    assert_eq!(budget.as_str(), budget_text, "budget 应逐字节往返（不裁剪、不改写）");
    assert_eq!(
        authority_limit.as_str(),
        authority_text,
        "authority_limit 应逐字节往返（各字段一个不同的串——写串列即红）"
    );
    assert_eq!(data_policy.as_str(), data_policy_text, "data_policy 应逐字节往返");
    assert_eq!(model_policy.as_str(), model_policy_text, "model_policy 应逐字节往返");
    assert_eq!(node_policy.as_str(), node_policy_text, "node_policy 应逐字节往返");
    assert_eq!(
        acceptance_predicate.as_str(),
        acceptance_text,
        "acceptance_predicate 应逐字节往返"
    );

    tx.commit().unwrap();
}

/// `RequirementId` 的「不改写、不裁剪、无校验」（`src/ids.rs` 的绝对措辞）：
/// 带空白／特殊字符／非 ASCII 的 id 经 `contract_requirement` 表**逐字节**往返，
/// 空串也被接受。
///
/// 同 `tests/intent.rs` 的 `opaque_strings_are_not_rewritten_trimmed_or_rejected`
/// 对另三个 id 类型的处置——照片要**各自直接**，不靠普通串间接经过。
///
/// **红条件**：`from_str` 或列绑定裁剪／改写输入即红。
#[test]
fn requirement_ids_are_not_rewritten_trimmed_or_rejected() {
    let (_d, db) = new_db();

    let weird_ids = [" 前后有空白 ", "requirement/带 特殊\"引号\"与 ✓", ""];

    let tx = db.begin().unwrap();
    let mut contract = sample_contract("c-ids", 1);
    contract.requirements = weird_ids
        .iter()
        .map(|id| sample_requirement(id, RequirementClass::Flexible))
        .collect();
    save_contract(&tx, &contract, 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load(&tx, "c-ids", 1);
    let read: Vec<&str> = back.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        read,
        weird_ids.to_vec(),
        "id 应逐字节往返（不改写、不裁剪；空串也被接受）"
    );
    tx.commit().unwrap();
}

/// 本 crate `src/` 下的全部 `.rs`，路径**相对 `src/`**（如 `src/contract.rs`——
/// 出现点清单按它登记，绝对路径会把机器相关的前缀带进断言）。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**：按相对路径扫的守卫在 crate 根下跑是绿的，
/// 换个目录跑就**读不到东西**——而「什么都没读到」正是上面那条正控制要逮的形状
/// （同 `continuum-capability/tests/single_writer.rs` 的 `repo_root`）。
fn crate_sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let rel = Path::new("src")
                .join(path.strip_prefix(&src).expect("收集自 `src/` 下的路径应以它开头"));
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读不到源文件 {}：{e}", path.display()));
            (rel, source)
        })
        .collect()
}

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**——静默跳过只会让守卫变窄，
/// 而变窄之后它仍然是绿的（同 `tests/intent.rs` 的同名函数）。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("列不出目录 {}：{e}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("列目录 {} 时出错：{e}", dir.display()))
            .path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 剥掉源码里的行注释与块注释（字符串字面量内不算注释），只留代码。
///
/// 它是一个**极简词法扫描**，认四种状态：代码 / 行注释 / 块注释 / 字符串字面量
/// （字符串里出现 `//` 不算注释）。**它不认识**：原始串 `r"…"`、字符字面量 `'/'`、
/// 字节串、以及嵌套块注释。扫的语料是本 crate 自己的 `src/`（今天没有那几种形态），
/// 且调用方有一条**正控制**（`pub struct VerificationRequirement` 必须被找到）兜住
/// 「扫描把整个文件剥没了」这一类坏法。它**不**主张「扫描结果等于编译器眼里的代码」。
/// （形态与 `continuum-canonical/tests/risk.rs` 的同名函数同。）
fn strip_comments(src: &str) -> String {
    // 四种状态：0 = 代码，1 = 行注释，2 = 块注释，3 = 字符串字面量。
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut state = 0u8;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match state {
            0 if c == '/' && next == Some('/') => {
                state = 1;
                i += 2;
            }
            0 if c == '/' && next == Some('*') => {
                state = 2;
                i += 2;
            }
            0 => {
                if c == '"' {
                    state = 3;
                }
                out.push(c);
                i += 1;
            }
            1 => {
                if c == '\n' {
                    state = 0;
                    out.push(c);
                }
                i += 1;
            }
            2 if c == '*' && next == Some('/') => {
                state = 0;
                i += 2;
            }
            2 => i += 1,
            _ => {
                // 字符串字面量：整段照抄，含转义（`\"` 不算结束）。
                out.push(c);
                if c == '\\' {
                    if let Some(escaped) = next {
                        out.push(escaped);
                        i += 2;
                        continue;
                    }
                } else if c == '"' {
                    state = 0;
                }
                i += 1;
            }
        }
    }
    out
}
