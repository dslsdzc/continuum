//! §10 的持久化（迁移 130）与 `VerificationFailed` 的第一个产生点
//! （设计 §10.1／§10.2／§10.3；计划 Task 9 的 Step 1）。
//!
//! # 用例与红条件（逐条）
//!
//! | # | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|---|
//! | 1 | `an_evidence_row_is_insert_only` | §4.1／§10.2 第 1 条：只 INSERT，无 UPDATE；二次写撞主键**且原行逐列不变** | 写成 `INSERT OR REPLACE`（**放宽档**）⇒ 第二次变 `Ok`、原行被覆写 |
//! | 2 | `the_producer_node_of_human_evidence_is_null_and_is_not_edge_checked` | §10.2 第 3 条的按支核对：`Human` 写 `NULL` 且不核对；`Node` 缺边 ⇒ `Err(MissingEvidenceEdge)`、有边 ⇒ `Ok` | 删掉边核对（**移除档**）⇒ 缺边侧变 `Ok` 而红 |
//! | 3 | `every_enum_column_round_trips_through_its_helper` | 七列逐列经各自 `as_str` / `parse` 读写 | 某列直接用 `Debug` / serde 写（**取反档**）⇒ 该列读回红 |
//! | 4 | `an_out_of_table_value_in_a_column_is_an_error` | 表外取值读回 `Err`、**不取默认型**（七列逐列） | `parse` 末支返回默认（**放宽档**）⇒ 该列红 |
//! | 5 | `a_failed_round_writes_a_verification_failed_event_in_the_same_transaction` | §10.3：`Failed` 同事务写 `verification.failed`（`node_id` 已置、payload 含逐项状态）；`Passed` **不写** | 删事件写入（**移除档**）⇒ 第一条红；写成「一律写」（**取反档**）⇒ 第二条红 |
//! | 6 | `the_event_is_written_only_for_failed_and_adjudication_required` | 五臂逐条：`Failed`／`AdjudicationRequired` 写，其余三臂不写 | 漏掉 `AdjudicationRequired`（**移除档**）⇒ 该臂红 |
//! | 7 | `a_counterexample_with_a_passing_aggregate_cannot_be_recorded` | §9.1 第 3 条的消费规则落在写行处：`(CounterexampleFound, Passed)` ⇒ `Err`；`(…, Failed)`／`(NoCounterexample, Passed)` ⇒ `Ok` | 删掉核对（**移除档**）⇒ 第一条红 |
//! | 8 | `the_written_aggregate_is_the_clamped_one` | §7.5 的 clamp 在写入路径上可观察：`[Pass]` ＋ 无可读制品 ⇒ 写下 `Unknown`（**不是 `Passed`**） | 跳过 `clamp_by_readability`（**移除档**）⇒ 红 |
//! | 9 | `artifact_exists_needs_every_terminal_node_to_have_produced_something` | §8.3 第 1 项：每个终端节点**各自**至少有一个产出制品 | 写成「任一终端节点有制品」（**放宽档**）⇒ 第一条红 |
//! | 10 | `artifact_exists_over_an_empty_terminal_set_is_true_and_that_is_a_fail_open_shape` | 空终端集 ⇒ `true`：**照实钉住的 fail-open 形状**（上游无守卫，见用例注释） | 写成「空终端集 ⇒ 假」（**取反档**）⇒ 红（这正是「钉住」的意义） |
//! | 11 | `the_migration_is_the_only_producer_of_version_130` | 迁移 130：版本恰 `[130]`、名字、五张表的 SQL 与建表 | 改成 131（**取反档**）⇒ 本文件红；与 Task 11 的 `runtime_migrations()`／`expected_migrations()` 对不上是跨 task 的观察点 |
//!
//! # 夹具的来历
//!
//! - `new_db()`：P0 内建迁移 ＋ P1 的 `p1_artifact_migrations()` /
//!   `p1_graph_migrations()` ＋ 本 task 的 `p5a_verify_migrations()`——照 P4 对
//!   `contract` 表的做法（`Db::open_with` ＋ `migrate` 起真库）。
//! - 含 `Evidence` 边的图**经库读回**（[`load_graph`]）而不是就地构造：
//!   `AdfirGraph::connect` 要 `continuum-port` 的 `Port` 值，而本 crate 不登记指向
//!   `continuum-port` 的边（计划 Global Constraints 的边表没有它），测试侧写不出
//!   `use continuum_port::…`。故 P1 四张表的行**直接写库**（列取值字面量即 P1 的
//!   落库编码），再由 `load_graph` 经 `connect` 重建——那条 `Evidence` 边与生产路径
//!   的图同形。
//! - 本 task 的证据表**只写不读**（没有 `load_*`，消费者未建），故「读回」统一走
//!   原始列 ＋ 与将来读回路径同形的本地 [`read_back`]（表外取值 ⇒ `Err`，不取默认型；
//!   这也是「编码对的严格逆」在今天的可拍形态）。

use continuum_artifact::{
    Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass, p1_artifact_migrations,
    save_artifact,
};
use continuum_events::EventType;
use continuum_graph::{
    AdfirGraph, ContractIdRef, EdgeKind, GraphId, Node, NodeId, load_graph, p1_graph_migrations,
};
use continuum_operator::{BackendId, OperatorId, OperatorVersion};
use continuum_persist::{Db, Migration, PersistError, Tx, Value, builtin_migrations};
use continuum_verify::{
    AdversaryOutcome, CheckVerdict, Claim, ContractView, Evidence, EvidenceId, EvidenceProducer,
    EvidenceScope, EvidenceStrength, EvidenceSubject, EvidenceType, IndependenceVerdict,
    InitialVerifierInput, RequirementId, TestValidity, TestValidityCheck, Validity,
    VerdictAggregate, VerifierLevel, VerifierSelection, VerifierVerdict, VerifyError,
    artifact_exists, insert_evidence, insert_test_validity, p5a_verify_migrations,
    record_verification_round,
};
use serde_json::json;

// ---------- 库与读写夹具 ----------

/// 建一个装了 P0 内建迁移 ＋ P1 两套迁移 ＋ 本 task 迁移 130 的临时库。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p1_artifact_migrations());
    migrations.extend(p1_graph_migrations());
    migrations.extend(p5a_verify_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 取一个文本列值。行数不对（0 行或多行）即打回——那是夹具自己的错。
fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实得 {other:?}"),
    }
}

/// 一条 SQL 的结果里取恰一行一列，且断言它是文本。
fn one_text(tx: &Tx<'_>, sql: &str, params: &[Value]) -> String {
    let rows = tx.query(sql, params).unwrap();
    assert_eq!(rows.len(), 1, "查询应恰返回一行：{sql}");
    text_of(&rows[0][0])
}

/// 落库列读回的统一形状：`parse` 给 `None` 即 `Err`——**不取默认型**。
///
/// 本 task 没有 `load_*` 读回函数（消费者未建），故这里给出一枚与**将来读回路径
/// 同形**的本地 helper：读原始列 → 经枚举自己的 `parse` → 表外取值转 `Err`。
/// 「放宽档：`parse` 末支返回默认」在这里表现为 `Ok(默认)` ⇒ 断 `Err` 的用例红。
fn read_back<T>(raw: &str, parse: fn(&str) -> Option<T>, column: &str) -> Result<T, String> {
    parse(raw).ok_or_else(|| format!("{column} 列出现表外取值 {raw:?}（不取默认型）"))
}

/// `evidence` 表的整一行（`SELECT *`）。
fn raw_evidence_row(tx: &Tx<'_>, evidence_id: &str) -> Vec<Value> {
    let rows = tx
        .query(
            "SELECT * FROM evidence WHERE evidence_id = ?1",
            &[Value::text(evidence_id)],
        )
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "evidence 表里应有恰一行 {evidence_id:?}，实得 {} 行",
        rows.len()
    );
    rows.into_iter().next().unwrap()
}

/// `evidence` 表里有没有这个 id 的行（不带断言）。
fn evidence_row_exists(tx: &Tx<'_>, evidence_id: &str) -> bool {
    !tx.query(
        "SELECT 1 FROM evidence WHERE evidence_id = ?1",
        &[Value::text(evidence_id)],
    )
    .unwrap()
    .is_empty()
}

/// `evidence` 表某行的某列原始文本（列名只取本文件里写死的字面量）。
fn raw_evidence_text(tx: &Tx<'_>, evidence_id: &str, column: &str) -> String {
    one_text(
        tx,
        &format!("SELECT {column} FROM evidence WHERE evidence_id = ?1"),
        &[Value::text(evidence_id)],
    )
}

/// `evidence_artifact` 表某个证据的全部关联行。
fn raw_evidence_artifact_rows(tx: &Tx<'_>, evidence_id: &str) -> Vec<Vec<Value>> {
    tx.query(
        "SELECT evidence_id, artifact_id FROM evidence_artifact \
         WHERE evidence_id = ?1 ORDER BY artifact_id",
        &[Value::text(evidence_id)],
    )
    .unwrap()
}

/// `test_validity` 表某行的某列原始文本。
fn raw_validity_text(tx: &Tx<'_>, evidence_id: &str, column: &str) -> String {
    one_text(
        tx,
        &format!("SELECT {column} FROM test_validity WHERE evidence_id = ?1"),
        &[Value::text(evidence_id)],
    )
}

/// `test_validity_check` 表的全部行（按 `check_name` 排序）。
fn raw_check_rows(tx: &Tx<'_>, evidence_id: &str) -> Vec<Vec<Value>> {
    tx.query(
        "SELECT check_name, verdict FROM test_validity_check \
         WHERE evidence_id = ?1 ORDER BY check_name",
        &[Value::text(evidence_id)],
    )
    .unwrap()
}

/// `test_validity_check` 表某行的某列原始文本。
fn raw_check_text(tx: &Tx<'_>, evidence_id: &str, check_name: &str, column: &str) -> String {
    one_text(
        tx,
        &format!(
            "SELECT {column} FROM test_validity_check \
             WHERE evidence_id = ?1 AND check_name = ?2"
        ),
        &[Value::text(evidence_id), Value::text(check_name)],
    )
}

/// `verification_round` 表某行的某列原始文本。
fn raw_round_text(tx: &Tx<'_>, round_id: &str, column: &str) -> String {
    one_text(
        tx,
        &format!("SELECT {column} FROM verification_round WHERE round_id = ?1"),
        &[Value::text(round_id)],
    )
}

/// `verification_round` 表的行数。
fn round_count(tx: &Tx<'_>) -> i64 {
    match &tx.query("SELECT COUNT(*) FROM verification_round", &[]).unwrap()[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// 某类事件的全部行（`event_id` / `node_id` / `payload`，按写入序）。
fn event_rows(tx: &Tx<'_>, event_type: &str) -> Vec<Vec<Value>> {
    tx.query(
        "SELECT event_id, node_id, payload FROM events WHERE event_type = ?1 ORDER BY rowid",
        &[Value::text(event_type)],
    )
    .unwrap()
}

/// `verification.failed`（`VerificationFailed`）的行数。
fn verification_failed_count(tx: &Tx<'_>) -> usize {
    event_rows(tx, EventType::VerificationFailed.as_str()).len()
}

// ---------- 值夹具 ----------

/// 一张含 `n-producer → n-verified` 的 `EdgeKind::Evidence` 边的图，外加第三个节点
/// `n-other`（用来拍「缺边」那一侧）。
///
/// 经库读回（[`load_graph`]）而不是就地构造：`AdfirGraph::connect` 要 `continuum-port`
/// 的 `Port` 值，而本 crate 不登记指向 `continuum-port` 的边（计划 Global Constraints），
/// 测试侧命名不了那些类型。故 P1 四张表的行直接写库——**列取值字面量即 P1 的落库编码**
/// （`state` 用 `pending`、`artifact_type` 用 `text`、边 `kind` 用 `evidence`、
/// 两个方向 `output` / `input`），与 `crates/continuum-graph/src/persist.rs` 逐处一致。
fn a_graph_with_an_evidence_edge(tx: &Tx<'_>) -> AdfirGraph {
    tx.execute(
        "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
         VALUES ('g-1', 1, 'c-1', '[]', '[]')",
        &[],
    )
    .unwrap();
    for node in ["n-producer", "n-verified", "n-other"] {
        tx.execute(
            "INSERT INTO adfir_node (graph_id, node_id, operator_id, operator_version, state,
                                     execution_policy, verification_policy, constraints, capabilities)
             VALUES ('g-1', ?1, 'op-1', 1, 'pending', 'null', 'null', '[]', '[]')",
            &[Value::text(node)],
        )
        .unwrap();
    }
    tx.execute(
        "INSERT INTO adfir_port (graph_id, node_id, port_id, direction, name, artifact_type)
         VALUES ('g-1', 'n-producer', 'p-out', 'output', 'out', 'text')",
        &[],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO adfir_port (graph_id, node_id, port_id, direction, name, artifact_type)
         VALUES ('g-1', 'n-verified', 'p-in', 'input', 'in', 'text')",
        &[],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO adfir_edge (graph_id, from_node, from_port, to_node, to_port, kind)
         VALUES ('g-1', 'n-producer', 'p-out', 'n-verified', 'p-in', 'evidence')",
        &[],
    )
    .unwrap();

    let graph = load_graph(tx, &GraphId::new("g-1"))
        .unwrap()
        .expect("夹具的图应已入库");
    assert!(
        graph.edges().iter().any(|e| e.from_node == NodeId::new("n-producer")
            && e.to_node == NodeId::new("n-verified")
            && e.kind == EdgeKind::Evidence),
        "夹具的图应含 n-producer → n-verified 的 Evidence 边"
    );
    graph
}

/// 一张两个终端节点（`n-1` / `n-2`）的图，用于 `artifact_exists`。
fn a_two_terminal_graph() -> AdfirGraph {
    let mut graph = AdfirGraph::new(GraphId::new("g-2"), ContractIdRef::new("c-1"));
    for node in ["n-1", "n-2"] {
        graph
            .add_node(Node::new(
                NodeId::new(node),
                OperatorId::new("op-1"),
                OperatorVersion::new(1),
            ))
            .unwrap();
    }
    graph.set_terminal_nodes(vec![NodeId::new("n-1"), NodeId::new("n-2")]);
    graph
}

/// 一件制品。`producer` 是它的产出节点（`artifact.producer_node`）。
fn an_artifact(id: &str, producer: Option<&str>) -> Artifact {
    Artifact {
        id: ArtifactId::new(id),
        artifact_type: ArtifactType::Text,
        content_hash: ContentHash::of(b"artifact-content"),
        size: 16,
        producer_node: producer.map(str::to_owned),
        input_artifacts: Vec::new(),
        metadata: json!({}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Public,
        version: 1,
    }
}

/// 往 `artifact` 表写一件有产出节点的制品（经 P1 的 `save_artifact`，同事务带
/// `ArtifactCreated` 事件——`new_db()` 的库里有 `events` 表）。
fn save_a_produced_artifact(tx: &Tx<'_>, artifact_id: &str, producer: &str) {
    save_artifact(
        tx,
        &an_artifact(artifact_id, Some(producer)),
        &format!("event-{artifact_id}"),
        1_000,
    )
    .unwrap();
}

/// 一条节点产的 `Test` 证据（`Test` 在「要读制品」的十三型内，故带一件制品引用）。
///
/// 产出节点固定为 `n-producer`——`a_graph_with_an_evidence_edge` 的图上有
/// `n-producer → n-verified` 的 `Evidence` 边，故它对 `n-verified` 能过边核对。
fn node_evidence(id: &str, claim_text: &str) -> Evidence {
    Evidence::from_tool_result(
        EvidenceId::from_str(id),
        EvidenceType::Test,
        EvidenceSubject::Unattached,
        Claim::from_str(claim_text),
        EvidenceProducer::Node {
            node: NodeId::new("n-producer"),
            backend: BackendId::new("backend-1"),
        },
        vec![ArtifactId::new("art-1")],
        EvidenceStrength::from_value(json!({ "level": 3 })),
        EvidenceScope::from_value(json!({ "covers": "全部" })),
    )
    .expect("夹具证据应能构造（Test 型带一件制品引用）")
}

/// 一条人确认的证据（`HumanConfirmation` ＋ `Human`，无制品引用）。
fn human_evidence(id: &str) -> Evidence {
    Evidence::from_tool_result(
        EvidenceId::from_str(id),
        EvidenceType::HumanConfirmation,
        EvidenceSubject::Requirement(RequirementId::from_str("req-1")),
        Claim::from_str("人确认（本 crate 不解析）"),
        EvidenceProducer::Human,
        Vec::new(),
        EvidenceStrength::from_value(json!({ "level": 1 })),
        EvidenceScope::from_value(json!({ "covers": "全部" })),
    )
    .expect("HumanConfirmation ＋ Human：空 artifact_refs 应可构造")
}

/// 一条五臂齐的有效性记录：三条 `Pass` ＋ 一条 `Fail` ＋ 一条 `Unknown` ⇒ `Invalid`。
///
/// 五条检查各取一个不同判定，「逐臂点名」的判别力最大（`Fail` 在
/// `AssertionStrength`、`Unknown` 在 `ErrorImplWouldPass`）。
fn a_validity(id: &str) -> TestValidity {
    TestValidity::new(
        EvidenceId::from_str(id),
        vec![
            (TestValidityCheck::RequirementMapping, CheckVerdict::Pass),
            (TestValidityCheck::AssertionStrength, CheckVerdict::Fail),
            (
                TestValidityCheck::ErrorImplWouldPass,
                CheckVerdict::Unknown,
            ),
            (
                TestValidityCheck::MockBypassesRealPath,
                CheckVerdict::Pass,
            ),
            (TestValidityCheck::MissingBoundary, CheckVerdict::Pass),
        ],
    )
    .expect("五臂齐的记录应构造成功")
}

/// 初判输入面：`artifacts` 给不给决定 `has_readable_artifact()`（§7.5 的 clamp 输入）。
fn an_input(readable: bool) -> InitialVerifierInput {
    InitialVerifierInput {
        contract: ContractView {
            requirements: Vec::new(),
        },
        artifacts: if readable {
            vec![an_artifact("art-1", None)]
        } else {
            Vec::new()
        },
        evidence: Vec::new(),
    }
}

/// 一轮选择结果（三项取值都不参与本文件的判定，只作落库值）。
fn a_selection() -> VerifierSelection {
    VerifierSelection {
        level: VerifierLevel::DeterministicChecker,
        backend: BackendId::new("backend-1"),
        independence: IndependenceVerdict::NotEstablished,
    }
}

/// 一条 `Fail` 判定。
fn fail_verdict() -> VerifierVerdict {
    VerifierVerdict::Fail {
        reason: Claim::from_str("判定理由（本 crate 不解析）"),
    }
}

/// 一条 `Unknown` 判定。
fn unknown_verdict() -> VerifierVerdict {
    VerifierVerdict::Unknown {
        reason: Claim::from_str("判不出（本 crate 不解析）"),
    }
}

// ---------- 1. 证据记录只追加 ----------

/// §4.1／§10.2 第 1 条：`evidence` 只 INSERT、无 UPDATE。
///
/// 第二次写同一个 `evidence_id` 撞主键得 `Err`（带原始串的 `Persist`），**且原行
/// 逐字段不变**——第二次的内容与第一次**全不同**（`claim` 与 `occurred_at` 都变），
/// 故「原行不变」有判别力。这一条钉的理由是具体的：有效性判定与证据必须是两条记录，
/// 否则「验证者改了被验之物」。
///
/// 红条件（放宽档）：把 INSERT 写成 `INSERT OR REPLACE` ⇒ 第二次变 `Ok`、原行被覆写，
/// 本条两处断言都红。
#[test]
fn an_evidence_row_is_insert_only() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let graph = a_graph_with_an_evidence_edge(&tx);

    insert_evidence(
        &tx,
        &graph,
        &node_evidence("ev-1", "第一次的断言文本"),
        &NodeId::new("n-verified"),
        1_000,
    )
    .unwrap();

    let before_row = raw_evidence_row(&tx, "ev-1");
    let before_refs = raw_evidence_artifact_rows(&tx, "ev-1");

    // 第二次写同一 id：内容全不同（claim 与 occurred_at 都变）——撞键被拒后
    // 原行应仍是第一份。
    let err = insert_evidence(
        &tx,
        &graph,
        &node_evidence("ev-1", "SHOULD-NOT-LAND"),
        &NodeId::new("n-verified"),
        2_000,
    )
    .unwrap_err();
    assert!(
        matches!(&err, VerifyError::Persist(PersistError::Database(m))
            if m.contains("UNIQUE constraint failed") && m.contains("evidence")),
        "期望撞主键的 Persist（UNIQUE constraint failed: evidence.…），实得 {err:?}"
    );

    assert_eq!(
        raw_evidence_row(&tx, "ev-1"),
        before_row,
        "撞键之后原 evidence 行应逐列不变（不是「报了个错、行也被改了」）"
    );
    assert_eq!(
        raw_evidence_artifact_rows(&tx, "ev-1"),
        before_refs,
        "证据-制品关联行同样不变（只追加的记录没有第二条写入路径）"
    );
    tx.commit().unwrap();
}

// ---------- 2. producer_node 可空与按支核对 ----------

/// §10.2 第 3 条的边核对**按支**执行（本 task 定的形状，理由与申报见 task 简报）：
///
/// - `Human` 一支：**没有产生者节点**（§4.3.3），故 `producer_node` 写 `NULL`
///   （列可空）、**不核对**——不变量「一行 evidence 对应一枚 producer_node →
///   verified_node 的 Evidence 边」与这一支不能同真；
/// - `Node` 一支：图上**没有**该边 ⇒ `Err(VerifyError::MissingEvidenceEdge { .. })`，
///   **且不写行**（核对在写入之前）；图上**有**该边 ⇒ `Ok`——这是对照正例：删掉
///   核对后缺边侧漏过去（变 `Ok` 即红），有边侧不因那次变异改变。
///
/// 红条件（移除档）：删掉边核对 ⇒ 缺边侧变 `Ok` 而红。
#[test]
fn the_producer_node_of_human_evidence_is_null_and_is_not_edge_checked() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let graph = a_graph_with_an_evidence_edge(&tx);

    // Human 一支：写入成功、producer_node 读回为空。
    insert_evidence(
        &tx,
        &graph,
        &human_evidence("ev-human"),
        &NodeId::new("n-verified"),
        1_000,
    )
    .expect("Human 证据没有产生者节点，其写入不核对 Evidence 边");
    let row = raw_evidence_row(&tx, "ev-human");
    assert_eq!(
        row[3],
        Value::Null,
        "Human 证据的 producer_node 应为 NULL（列可空——本 task 定的形状）"
    );
    // 顺带：subject 两列的形状（Requirement 一支：kind 有值、requirement 有值）。
    assert_eq!(row[4], Value::text("requirement"), "subject_kind 列");
    assert_eq!(row[5], Value::text("req-1"), "subject_requirement 列");

    // Node 一支，图上没有 producer → verified 的 Evidence 边 ⇒ Err，且不写行。
    let err = insert_evidence(
        &tx,
        &graph,
        &node_evidence("ev-no-edge", "缺边的证据"),
        &NodeId::new("n-other"),
        1_000,
    )
    .unwrap_err();
    assert_eq!(
        err,
        VerifyError::MissingEvidenceEdge {
            evidence_id: EvidenceId::from_str("ev-no-edge"),
            graph_id: GraphId::new("g-1"),
            producer_node: NodeId::new("n-producer"),
            verified_node: NodeId::new("n-other"),
        },
        "缺边应报 MissingEvidenceEdge，且载荷能定位（证据、图、两端）"
    );
    assert!(
        !evidence_row_exists(&tx, "ev-no-edge"),
        "核对失败时不得留下半写的行"
    );

    // 另一侧：图上**有**该边 ⇒ Ok。
    insert_evidence(
        &tx,
        &graph,
        &node_evidence("ev-with-edge", "有边的证据"),
        &NodeId::new("n-verified"),
        1_000,
    )
    .expect("图上有 producer → verified 的 Evidence 边时 Node 一支应放行");
    assert!(evidence_row_exists(&tx, "ev-with-edge"));
    tx.commit().unwrap();
}

// ---------- 3. 枚举列经 helper 往返 ----------

/// 七列**逐列一次**：`evidence.evidence_type`、`test_validity.verdict`、
/// `test_validity_check.check_name`、`test_validity_check.verdict`、
/// `verification_round.aggregate`、`verification_round.independence`、
/// `verification_round.strategy_verdict` 各写各读。
///
/// 每列两重断言：**手写字面串**（防 `as_str` / `parse` 一起被改）＋ **经 `parse` 读回**
/// （防某列直接用 `Debug` / serde 写——`NodeState` 的 serde 是 `SCREAMING_SNAKE_CASE`，
/// 直接反序列化会失败，P1 设计 §15 的同一条纪律）。
///
/// 红条件（取反档）：某列直接用 `Debug` 或 serde 写 ⇒ 该列读回红。
#[test]
fn every_enum_column_round_trips_through_its_helper() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let graph = a_graph_with_an_evidence_edge(&tx);

    // (1) evidence.evidence_type
    insert_evidence(
        &tx,
        &graph,
        &node_evidence("ev-type", "类型往返"),
        &NodeId::new("n-verified"),
        1_000,
    )
    .unwrap();
    let raw = raw_evidence_text(&tx, "ev-type", "evidence_type");
    assert_eq!(raw, "test", "evidence_type 列的落库编码（手写期望）");
    assert_eq!(
        read_back(&raw, EvidenceType::parse, "evidence.evidence_type"),
        Ok(EvidenceType::Test),
        "evidence_type 列应经 parse 读回"
    );

    // (2)(3) test_validity.verdict 与五条检查行
    let validity = a_validity("ev-tv");
    assert_eq!(
        validity.verdict,
        Validity::Invalid,
        "夹具起点：有 Fail ⇒ Invalid（后面 verdict 列的断言不变）"
    );
    insert_test_validity(&tx, &validity).unwrap();
    let raw = raw_validity_text(&tx, "ev-tv", "verdict");
    assert_eq!(raw, "invalid", "test_validity.verdict 列的落库编码（手写期望）");
    assert_eq!(
        read_back(&raw, Validity::parse, "test_validity.verdict"),
        Ok(Validity::Invalid),
        "test_validity.verdict 列应经 parse 读回"
    );

    let rows = raw_check_rows(&tx, "ev-tv");
    assert_eq!(rows.len(), 5, "五臂齐的记录应有五条检查行");
    // 逐行：手写期望（串 ＋ 枚），不从实现取。
    let expected: [(&str, TestValidityCheck, &str, CheckVerdict); 5] = [
        (
            "assertion_strength",
            TestValidityCheck::AssertionStrength,
            "fail",
            CheckVerdict::Fail,
        ),
        (
            "error_impl_would_pass",
            TestValidityCheck::ErrorImplWouldPass,
            "unknown",
            CheckVerdict::Unknown,
        ),
        (
            "missing_boundary",
            TestValidityCheck::MissingBoundary,
            "pass",
            CheckVerdict::Pass,
        ),
        (
            "mock_bypasses_real_path",
            TestValidityCheck::MockBypassesRealPath,
            "pass",
            CheckVerdict::Pass,
        ),
        (
            "requirement_mapping",
            TestValidityCheck::RequirementMapping,
            "pass",
            CheckVerdict::Pass,
        ),
    ];
    for (row, (name, check, verdict_str, verdict)) in rows.iter().zip(expected) {
        assert_eq!(
            text_of(&row[0]),
            name,
            "test_validity_check.check_name 列的落库编码（手写期望）"
        );
        assert_eq!(
            read_back(
                &text_of(&row[0]),
                TestValidityCheck::parse,
                "test_validity_check.check_name"
            ),
            Ok(check),
            "check_name 列应经 parse 读回该检查"
        );
        assert_eq!(
            text_of(&row[1]),
            verdict_str,
            "test_validity_check.verdict 列的落库编码（手写期望）"
        );
        assert_eq!(
            read_back(
                &text_of(&row[1]),
                CheckVerdict::parse,
                "test_validity_check.verdict"
            ),
            Ok(verdict),
            "verdict 列应经 parse 读回该判定"
        );
    }

    // (4)(5)(6) verification_round 的三列（写一轮 Passed——不写事件，见 §10.3）。
    let aggregate = record_verification_round(
        &tx,
        "round-enums",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[VerifierVerdict::Pass],
        IndependenceVerdict::NotEstablished,
        &AdversaryOutcome::NoCounterexample,
        "event-unused",
        1_000,
    )
    .unwrap();
    assert_eq!(aggregate, VerdictAggregate::Passed, "夹具起点：Passed");

    let raw = raw_round_text(&tx, "round-enums", "aggregate");
    assert_eq!(raw, "passed", "aggregate 列的落库编码（手写期望）");
    assert_eq!(
        read_back(&raw, VerdictAggregate::parse, "verification_round.aggregate"),
        Ok(VerdictAggregate::Passed),
        "aggregate 列应经 parse 读回"
    );

    let raw = raw_round_text(&tx, "round-enums", "independence");
    assert_eq!(
        raw, "not_established",
        "independence 列的落库编码（手写期望）"
    );
    assert_eq!(
        read_back(
            &raw,
            IndependenceVerdict::parse,
            "verification_round.independence"
        ),
        Ok(IndependenceVerdict::NotEstablished),
        "independence 列应经 parse 读回"
    );

    let raw = raw_round_text(&tx, "round-enums", "strategy_verdict");
    assert_eq!(
        raw, "no_counterexample",
        "strategy_verdict 列的落库编码（手写期望）"
    );
    assert_eq!(
        read_back(
            &raw,
            AdversaryOutcome::parse,
            "verification_round.strategy_verdict"
        ),
        Ok(AdversaryOutcome::NoCounterexample),
        "strategy_verdict 列应经 parse 读回"
    );

    // selection 列是 JSON（不是枚举列，但同属这一轮写入的可读面）：三枚都写着编码串。
    let selection_text = raw_round_text(&tx, "round-enums", "selection");
    let selection_json: serde_json::Value =
        serde_json::from_str(&selection_text).expect("selection 列应是合法 JSON");
    assert_eq!(
        selection_json,
        json!({
            "level": "deterministic_checker",
            "backend": "backend-1",
            "independence": "not_established",
        }),
        "selection 列应写着 VerifierSelection 三枚的编码"
    );
    tx.commit().unwrap();
}

// ---------- 4. 表外取值读回 Err ----------

/// 绕过 writer 直接 SQL 把某列改成表外取值 ⇒ 读回 `Err`、**不取默认型**（七列逐列）。
///
/// 「读回」走与将来读回路径同形的 [`read_back`]：`parse` 给 `None` 即 `Err`。
/// 「不取默认型」的语义 = `parse` 的末支不得返回某个默认臂——若它返回默认，
/// 这里会拿到 `Ok(默认)` 而红。
///
/// 红条件（放宽档）：`parse` 末支返回默认 ⇒ 逐列红。
#[test]
fn an_out_of_table_value_in_a_column_is_an_error() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let graph = a_graph_with_an_evidence_edge(&tx);

    // 先经 writer 写合法行，再把要考的列改成表外取值（绕过 writer 的编码产生点）。
    insert_evidence(
        &tx,
        &graph,
        &node_evidence("ev-x", "表外取值"),
        &NodeId::new("n-verified"),
        1_000,
    )
    .unwrap();
    insert_test_validity(&tx, &a_validity("ev-tv-x")).unwrap();
    record_verification_round(
        &tx,
        "round-x",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[VerifierVerdict::Pass],
        IndependenceVerdict::NotEstablished,
        &AdversaryOutcome::NoCounterexample,
        "event-unused",
        1_000,
    )
    .unwrap();

    // (1) evidence.evidence_type
    tx.execute(
        "UPDATE evidence SET evidence_type = 'NOT-A-TYPE' WHERE evidence_id = 'ev-x'",
        &[],
    )
    .unwrap();
    let raw = raw_evidence_text(&tx, "ev-x", "evidence_type");
    assert!(
        read_back(&raw, EvidenceType::parse, "evidence.evidence_type").is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );

    // (2) test_validity.verdict
    tx.execute(
        "UPDATE test_validity SET verdict = 'NOT-A-VERDICT' WHERE evidence_id = 'ev-tv-x'",
        &[],
    )
    .unwrap();
    let raw = raw_validity_text(&tx, "ev-tv-x", "verdict");
    assert!(
        read_back(&raw, Validity::parse, "test_validity.verdict").is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );

    // (3) test_validity_check.check_name
    tx.execute(
        "UPDATE test_validity_check SET check_name = 'NOT-A-CHECK' \
         WHERE evidence_id = 'ev-tv-x' AND check_name = 'requirement_mapping'",
        &[],
    )
    .unwrap();
    let raw = raw_check_text(&tx, "ev-tv-x", "NOT-A-CHECK", "check_name");
    assert!(
        read_back(&raw, TestValidityCheck::parse, "test_validity_check.check_name").is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );

    // (4) test_validity_check.verdict
    tx.execute(
        "UPDATE test_validity_check SET verdict = 'NOT-A-CV' \
         WHERE evidence_id = 'ev-tv-x' AND check_name = 'missing_boundary'",
        &[],
    )
    .unwrap();
    let raw = raw_check_text(&tx, "ev-tv-x", "missing_boundary", "verdict");
    assert!(
        read_back(&raw, CheckVerdict::parse, "test_validity_check.verdict").is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );

    // (5) verification_round.aggregate
    tx.execute(
        "UPDATE verification_round SET aggregate = 'NOT-AN-AGGREGATE' WHERE round_id = 'round-x'",
        &[],
    )
    .unwrap();
    let raw = raw_round_text(&tx, "round-x", "aggregate");
    assert!(
        read_back(&raw, VerdictAggregate::parse, "verification_round.aggregate").is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );

    // (6) verification_round.independence
    tx.execute(
        "UPDATE verification_round SET independence = 'NOT-AN-INDEPENDENCE' WHERE round_id = 'round-x'",
        &[],
    )
    .unwrap();
    let raw = raw_round_text(&tx, "round-x", "independence");
    assert!(
        read_back(
            &raw,
            IndependenceVerdict::parse,
            "verification_round.independence"
        )
        .is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );

    // (7) verification_round.strategy_verdict
    tx.execute(
        "UPDATE verification_round SET strategy_verdict = 'NOT-A-STRATEGY' WHERE round_id = 'round-x'",
        &[],
    )
    .unwrap();
    let raw = raw_round_text(&tx, "round-x", "strategy_verdict");
    assert!(
        read_back(
            &raw,
            AdversaryOutcome::parse,
            "verification_round.strategy_verdict"
        )
        .is_err(),
        "表外取值必须读回 Err、不取默认型（实得 {raw:?}）"
    );
}

// ---------- 5. VerificationFailed 的第一个产生点 ----------

/// §10.3：`VerdictAggregate ∈ {Failed, AdjudicationRequired}` 时写一条
/// `EventType::VerificationFailed`，**与行写入同一事务**（同一条 `Tx` 里未提交即
/// 读到，是「同一事务」的正面照片）；`node_id` 置为受验节点，payload 含逐项状态。
///
/// **另一侧**：`Passed` ⇒ **不写**事件。
///
/// 红条件：删掉事件写入（**移除档**）⇒ 第一条红；写成「一律写」（**取反档**）⇒
/// 第二条红。
#[test]
fn a_failed_round_writes_a_verification_failed_event_in_the_same_transaction() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    // Failed：一条 Fail ⇒ 不加 clamp（Fail 不受可读性影响）⇒ Failed ⇒ 写事件。
    let aggregate = record_verification_round(
        &tx,
        "round-failed",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[fail_verdict()],
        IndependenceVerdict::NotEstablished,
        &AdversaryOutcome::NoCounterexample,
        "event-1",
        1_000,
    )
    .unwrap();
    assert_eq!(aggregate, VerdictAggregate::Failed);

    // 同一条 Tx 里读得到 ⇒ 行与事件在同一事务（测试不写「已回滚」的运行期照片，
    // 纪律第 4 条：回滚只能由调用方丢弃句柄）。
    let rows = event_rows(&tx, EventType::VerificationFailed.as_str());
    assert_eq!(
        rows.len(),
        1,
        "Failed 应在同一事务里写恰一条 verification.failed"
    );
    assert_eq!(rows[0][0], Value::text("event-1"), "事件 id 由调用方给");
    assert_eq!(
        rows[0][1],
        Value::text("n-verified"),
        "node_id 应置为受验节点"
    );
    let payload: serde_json::Value =
        serde_json::from_str(&text_of(&rows[0][2])).expect("payload 应是合法 JSON");
    assert_eq!(
        payload,
        json!({
            "round_id": "round-failed",
            "graph_id": "g-1",
            "verified_node": "n-verified",
            "aggregate": "failed",
            "independence": "not_established",
            "verdicts": ["fail"],
        }),
        "payload 应含逐项状态（§10.3）"
    );
    tx.commit().unwrap();

    // 另一侧：Passed ⇒ 不写（事件数仍只有第一条）。
    let tx = db.begin().unwrap();
    let aggregate = record_verification_round(
        &tx,
        "round-passed",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[VerifierVerdict::Pass],
        IndependenceVerdict::NotEstablished,
        &AdversaryOutcome::NoCounterexample,
        "event-2",
        2_000,
    )
    .unwrap();
    assert_eq!(aggregate, VerdictAggregate::Passed);
    assert_eq!(
        verification_failed_count(&tx),
        1,
        "Passed 不得写 verification.failed（仍只有 round-failed 那一条）"
    );
    tx.commit().unwrap();
}

// ---------- 6. 事件只在两个聚合臂上写 ----------

/// §10.3 的五臂**逐条**：`Failed` 写、`AdjudicationRequired` 写、`Passed` /
/// `PassedWithUnknown` / `Unknown` 不写。每一轮的聚合身份先断言（第 5 条用例
/// 的对照），再断言事件数的增量。
///
/// 红条件（移除档）：把 `AdjudicationRequired` 漏掉 ⇒ 该臂红；把某一「不写」臂
/// 写出去 ⇒ 该臂红。
#[test]
fn the_event_is_written_only_for_failed_and_adjudication_required() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let cases: [(&str, VerdictAggregate, Vec<VerifierVerdict>, bool); 5] = [
        ("failed", VerdictAggregate::Failed, vec![fail_verdict()], true),
        (
            "adjudication_required",
            VerdictAggregate::AdjudicationRequired,
            vec![VerifierVerdict::Pass, fail_verdict()],
            true,
        ),
        ("passed", VerdictAggregate::Passed, vec![VerifierVerdict::Pass], false),
        (
            "passed_with_unknown",
            VerdictAggregate::PassedWithUnknown,
            vec![VerifierVerdict::Pass, unknown_verdict()],
            false,
        ),
        ("unknown", VerdictAggregate::Unknown, vec![unknown_verdict()], false),
    ];

    let mut seen = 0usize;
    for (tag, expected, verdicts, writes_event) in cases {
        let before = verification_failed_count(&tx);
        let aggregate = record_verification_round(
            &tx,
            &format!("round-{tag}"),
            &GraphId::new("g-1"),
            &NodeId::new("n-verified"),
            &a_selection(),
            &an_input(true),
            &verdicts,
            IndependenceVerdict::NotEstablished,
            &AdversaryOutcome::NoCounterexample,
            &format!("event-{tag}"),
            1_000,
        )
        .unwrap();
        assert_eq!(aggregate, expected, "第 {tag} 臂的聚合身份（夹具起点）");
        assert_eq!(
            verification_failed_count(&tx),
            before + usize::from(writes_event),
            "第 {tag} 臂：写事件与否与 §10.3 不符"
        );
        seen += 1;
    }
    assert_eq!(seen, 5, "五臂应逐条跑到（不抽代表）");
    tx.commit().unwrap();
}

// ---------- 7. 消费规则落在写行处 ----------

/// §9.1 第 3 条的消费规则落在写行处：`CounterexampleFound` 时该轮聚合**不得**取
/// `Passed` / `PassedWithUnknown`。
///
/// `(CounterexampleFound, Passed)` ⇒ `Err`，**且不写行、不写事件**（核对在写入之前）；
/// `(CounterexampleFound, Failed)` ⇒ `Ok`（只拒绝那一对组合，不是「反例否决一切」）；
/// `(NoCounterexample, Passed)` ⇒ `Ok`（谓词不得恒假）。
///
/// 红条件（移除档）：删掉这道核对 ⇒ 第一条红。
#[test]
fn a_counterexample_with_a_passing_aggregate_cannot_be_recorded() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let counterexample = AdversaryOutcome::CounterexampleFound {
        passed_verifier: BackendId::new("backend-1"),
        counterexample: Claim::from_str("反例（本 crate 不解析）"),
    };

    // (CounterexampleFound, Passed) ⇒ Err。
    let err = record_verification_round(
        &tx,
        "round-x",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[VerifierVerdict::Pass],
        IndependenceVerdict::NotEstablished,
        &counterexample,
        "event-x",
        1_000,
    )
    .unwrap_err();
    assert_eq!(
        err,
        VerifyError::CounterexampleWithPassingAggregate {
            round_id: "round-x".to_owned(),
            aggregate: VerdictAggregate::Passed,
        },
        "反例在场而聚合为 Passed 应在写行前被拒（§9.1 第 3 条）"
    );
    assert_eq!(round_count(&tx), 0, "被拒的一轮不得写行");
    assert_eq!(verification_failed_count(&tx), 0, "被拒的一轮不得写事件");

    // (CounterexampleFound, Failed) ⇒ Ok：反例与失败聚合可共存（finding 记录）。
    let aggregate = record_verification_round(
        &tx,
        "round-failed",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[fail_verdict()],
        IndependenceVerdict::NotEstablished,
        &counterexample,
        "event-a",
        2_000,
    )
    .unwrap();
    assert_eq!(aggregate, VerdictAggregate::Failed);
    assert_eq!(
        raw_round_text(&tx, "round-failed", "strategy_verdict"),
        "counterexample_found",
        "finding 的可见形态之一：strategy_verdict 的标注（§9.1 第 4 条）"
    );

    // (NoCounterexample, Passed) ⇒ Ok：谓词不得恒假。
    let aggregate = record_verification_round(
        &tx,
        "round-ok",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(true),
        &[VerifierVerdict::Pass],
        IndependenceVerdict::NotEstablished,
        &AdversaryOutcome::NoCounterexample,
        "event-b",
        3_000,
    )
    .unwrap();
    assert_eq!(aggregate, VerdictAggregate::Passed);
    assert_eq!(round_count(&tx), 2, "两轮放行的判定都应写行");
    tx.commit().unwrap();
}

// ---------- 8. 落库的是 clamp 后的聚合 ----------

/// §7.5 的 clamp 在写入路径上可观察（把 Task 6 的 (o) 与落库连起来）：
/// 给 `[Pass]` 而 `initial_input` 无可读制品 ⇒ 写下的 `aggregate` 是 `Unknown`
/// （**不是 `Passed`**）——「无物可读 ⇒ `Unknown`，不得 `Pass`」。
///
/// 红条件（移除档）：跳过 `clamp_by_readability` ⇒ 写下的列变 `passed`、返回值变
/// `Passed`，本条红。
#[test]
fn the_written_aggregate_is_the_clamped_one() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let aggregate = record_verification_round(
        &tx,
        "round-clamped",
        &GraphId::new("g-1"),
        &NodeId::new("n-verified"),
        &a_selection(),
        &an_input(false),
        &[VerifierVerdict::Pass],
        IndependenceVerdict::NotEstablished,
        &AdversaryOutcome::NoCounterexample,
        "event-1",
        1_000,
    )
    .unwrap();

    assert_eq!(
        aggregate,
        VerdictAggregate::Unknown,
        "无可读制品时 Pass 被 clamp 成 Unknown（不是 Passed）"
    );
    assert_eq!(
        raw_round_text(&tx, "round-clamped", "aggregate"),
        "unknown",
        "写下的 aggregate 应是 clamp 后的值"
    );
    // clamp 后的 Unknown 不是 {Failed, AdjudicationRequired}，故不写事件。
    assert_eq!(
        verification_failed_count(&tx),
        0,
        "clamp 成 Unknown 不写 verification.failed"
    );
    tx.commit().unwrap();
}

// ---------- 9. artifact_exists：每个终端节点各自有产出 ----------

/// §8.3 第 1 项：受验图的**每个终端节点**至少有一个产出制品（读 `artifact` 表的
/// `producer_node`）。
///
/// 图上两个终端节点、制品表里只有其一 ⇒ `false`；两个都有 ⇒ `true`。
///
/// 红条件（放宽档）：把它写成「任一终端节点有制品」⇒ 第一条红。
#[test]
fn artifact_exists_needs_every_terminal_node_to_have_produced_something() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let graph = a_two_terminal_graph();

    save_a_produced_artifact(&tx, "art-n1", "n-1");
    assert!(
        !artifact_exists(&tx, &graph).unwrap(),
        "「每个终端节点各自至少有一个」：n-2 还没有产出制品 ⇒ false"
    );

    save_a_produced_artifact(&tx, "art-n2", "n-2");
    assert!(
        artifact_exists(&tx, &graph).unwrap(),
        "两个终端节点都有产出制品 ⇒ true"
    );
    tx.commit().unwrap();
}

// ---------- 10. 空终端集：照实钉住的 fail-open 形状 ----------

/// 终端集为空 ⇒ `true`（「各自至少有一个」空洞为真）。
///
/// **这是一处 fail-open 的形状**：上游没有守卫——实测 `AdfirGraph::validate()`
/// （`crates/continuum-graph/src/graph.rs`）不要求 `terminal_nodes` 非空，故空终端集
/// 是一条**可达**输入。**本计划照实钉住它、不改它的取值**（改它就要替 §266 第 1 项
/// 发明一条「空图不算存在」的判据），并把它报给设计作者（计划 `## 遗留` 第一节第 9 条）。
///
/// 红条件（取反档）：把它写成「终端集为空 ⇒ 假」⇒ 本条红（**这正是「钉住」的意义**：
/// 值一旦被改，有人会知道）。
#[test]
fn artifact_exists_over_an_empty_terminal_set_is_true_and_that_is_a_fail_open_shape() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let graph = AdfirGraph::new(GraphId::new("g-3"), ContractIdRef::new("c-1"));
    assert!(
        graph.terminal_nodes().is_empty(),
        "夹具起点：终端集为空（可照实钉住这个取值的前提）"
    );
    assert!(
        artifact_exists(&tx, &graph).unwrap(),
        "空终端集 ⇒ true（空洞为真）——这是照实钉住的 fail-open 形状，见本条文档"
    );
}

// ---------- 11. 迁移 130 ----------

/// `p5a_verify_migrations()` 返回的 version 恰是 `[130]`、名字是 `p5a_verify` 一类；
/// 五张表都在它的 SQL 里（逐表名断言）——并且经 `migrate()` 真的建出来了。
///
/// **「恰是 [130]」的承重面有两处**：本文件的等值断言（改号即红）与 Task 11 的
/// `runtime_migrations()` / `expected_migrations()`（跨 task 的观察点，本文件拍不到）。
///
/// 红条件（取反档）：改成 131 ⇒ 本文件的版本断言红；与 Task 11 的装配对不上时是
/// 下一层的观察点。
#[test]
fn the_migration_is_the_only_producer_of_version_130() {
    let migrations = p5a_verify_migrations();

    assert_eq!(
        migrations.iter().map(|m| m.version).collect::<Vec<_>>(),
        vec![130],
        "版本号应恰是 [130]（§10.1 的号段）"
    );
    assert!(
        migrations[0].name.contains("p5a_verify"),
        "迁移名应是 p5a_verify 一类，实得 {:?}",
        migrations[0].name
    );

    let tables = [
        "evidence",
        "evidence_artifact",
        "test_validity",
        "test_validity_check",
        "verification_round",
    ];
    let sql = migrations[0].sql;
    for table in tables {
        assert!(
            sql.contains(&format!("CREATE TABLE {table} (")),
            "迁移 130 的 SQL 应逐表建出 {table}（§10.2 的五张表一次写齐）"
        );
    }

    // 五张表经 `migrate()` 真的在库里（不只是 SQL 文本里有）。
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    let existing: Vec<String> = tx
        .query(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| text_of(&row[0]))
        .collect();
    for table in tables {
        assert!(
            existing.iter().any(|t| t == table),
            "库里应有 {table} 表，实得 {existing:?}"
        );
    }
}
