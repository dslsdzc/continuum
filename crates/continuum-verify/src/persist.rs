//! §10 的持久化：迁移 130 的五张表、写行路径与 `VerificationFailed` 的第一个
//! 产生点（设计 §10.1／§10.2／§10.3）。
//!
//! # 本文件的写函数一律不 `commit`
//!
//! 事务由调用方持有并提交（照 P1 的 `apply_transition` 形状，
//! `crates/continuum-graph/src/transition_tx.rs`）。**返回 `Err` 之后调用方必须回滚
//! 该事务，不得提交**：写入是先写行、后写事件（[`record_verification_round`]）或
//! 先写行、后写关联行（[`insert_evidence`]），`Err` 时前一步的写入可能已经留在事务
//! 里。本文件**不代劳回滚**——`Tx` 没有保存点接口，回滚只能由调用方丢弃句柄。
//! **本块不写「事务已回滚」的运行期照片**（计划纪律第 4 条：那条约定没有可拍的
//! 手段），失败路径的照片只到「没有半写的行」为止。
//!
//! # 枚举列的编码不在本文件
//!
//! 七个枚举列的串一律取自各类型自己的 `as_str`（与类型同址）：`EvidenceType` /
//! `Validity` / `TestValidityCheck` / `CheckVerdict` / `VerdictAggregate` /
//! `IndependenceVerdict` / `AdversaryOutcome`。本文件**一个字面量都不内联**——
//! 它只把类型给的串放进参数。`subject_kind` 列同法（`EvidenceSubject::as_str`）。
//!
//! # 两处本 task 定的形状（申报）
//!
//! - **`producer_node` 列可空**：`EvidenceProducer::Human` 一支写 `NULL`，且 §10.2
//!   第 3 条的边核对**只在 `Node` 一支执行**——不变量「一行 evidence 对应一枚
//!   `producer_node → verified_node` 的 `EdgeKind::Evidence` 边」与「Human 的证据没有
//!   产生者节点」（§4.3.3）不能同真。代价与申报见 task 简报与计划的 `## 遗留`
//!   第一节第 7 条（收件人：设计作者）。
//! - **`verification_round.strategy_verdict` 列存 `AdversaryOutcome` 的编码**：
//!   设计 §10.2 只给列名、§2.2 的冻结清单里没有它的承载类型；**不为它新造枚举**
//!   （已知的一枚 `verification_strategy` 取值不构成完整取值域，规范未给其余取值）。
//!   申报见计划 `## 遗留` 第二节。

use continuum_events::{Event, EventType};
use continuum_graph::{AdfirGraph, EdgeKind, GraphId, NodeId};
use continuum_persist::{Migration, PersistError, Tx, Value};

use crate::adversary::{AdversaryOutcome, adversary_permits_passing};
use crate::error::VerifyError;
use crate::evidence::{Evidence, EvidenceProducer, EvidenceSubject};
use crate::input::InitialVerifierInput;
use crate::selection::{IndependenceVerdict, VerifierSelection};
use crate::test_validity::TestValidity;
use crate::verdict::{VerdictAggregate, VerifierVerdict, clamp_by_readability};

/// 迁移 130（§10.1 已实测该档为空）。五张表一次写齐（设计 §10.2）：
///   evidence              evidence_id PK, graph_id, verified_node, producer_node(可空),
///                         subject_kind, subject_requirement, evidence_type,
///                         claim, strength(TEXT/JSON), scope(TEXT/JSON), occurred_at
///   evidence_artifact     (evidence_id, artifact_id) PK
///   test_validity         evidence_id PK, verdict
///   test_validity_check   (evidence_id, check_name) PK, verdict
///   verification_round    round_id PK, graph_id, verified_node, selection(JSON),
///                         aggregate, independence, strategy_verdict, occurred_at
/// 不变量逐条见设计 §10.2；**本函数不 `commit`**（照 P1 的 apply_transition 形状）。
///
/// # 一次写齐五张表（申报）
///
/// 设计与 P4 的先例允许逐表追加，本块五个表同属一次判定链，一次写齐比五次追加少
/// 四次「改同一条 SQL」的机会；**Task 11 之后不得再改它的 SQL**（`Db::migrate()`
/// 以 `schema_migrations.version` 判「已应用」，对已建库的改号或改 SQL 都无效）。
/// 照片：`tests/persist.rs` 的 `the_migration_is_the_only_producer_of_version_130`
/// （版本恰 `[130]`、五张表逐表名断言、迁移后五张表在库里）。
pub fn p5a_verify_migrations() -> Vec<Migration> {
    vec![Migration::new(
        130,
        "p5a_verify",
        "CREATE TABLE evidence (
            evidence_id         TEXT PRIMARY KEY NOT NULL,
            graph_id            TEXT NOT NULL,
            verified_node       TEXT NOT NULL,
            producer_node       TEXT,
            subject_kind        TEXT NOT NULL,
            subject_requirement TEXT,
            evidence_type       TEXT NOT NULL,
            claim               TEXT NOT NULL,
            strength            TEXT NOT NULL,
            scope               TEXT NOT NULL,
            occurred_at         INTEGER NOT NULL
        );
        CREATE TABLE evidence_artifact (
            evidence_id TEXT NOT NULL,
            artifact_id TEXT NOT NULL,
            PRIMARY KEY (evidence_id, artifact_id)
        );
        CREATE TABLE test_validity (
            evidence_id TEXT PRIMARY KEY NOT NULL,
            verdict     TEXT NOT NULL
        );
        CREATE TABLE test_validity_check (
            evidence_id TEXT NOT NULL,
            check_name  TEXT NOT NULL,
            verdict     TEXT NOT NULL,
            PRIMARY KEY (evidence_id, check_name)
        );
        CREATE TABLE verification_round (
            round_id         TEXT PRIMARY KEY NOT NULL,
            graph_id         TEXT NOT NULL,
            verified_node    TEXT NOT NULL,
            selection        TEXT NOT NULL,
            aggregate        TEXT NOT NULL,
            independence     TEXT NOT NULL,
            strategy_verdict TEXT NOT NULL,
            occurred_at      INTEGER NOT NULL
        );",
    )]
}

/// 只 INSERT，无 UPDATE（§4.1）。写行时同事务核对图上存在 `producer_node → verified_node`
/// 的 `EdgeKind::Evidence` 边（`Human` 一支不核对，`producer_node` 写 NULL）。
///
/// **本函数不 `commit`**（文件头）；`Err` 之后调用方必须回滚。核对排在写行之前：
/// 缺边时不留半成品（照片：`tests/persist.rs` 的
/// `the_producer_node_of_human_evidence_is_null_and_is_not_edge_checked`）。
///
/// `graph_id` 取 `graph.id`（§10.2 的 `graph_id` 列与「受验图」是同一枚身份）；
/// `verified_node` 由调用方给（证据被写来验证哪一枚节点）。`occurred_at` 由调用方给：
/// 本层不取系统时钟（同 P4 的 `save_contract` 与 P1 的 `apply_transition`）。
pub fn insert_evidence(
    tx: &Tx<'_>,
    graph: &AdfirGraph,
    evidence: &Evidence,
    verified_node: &NodeId,
    occurred_at: i64,
) -> Result<(), VerifyError> {
    // §10.2 第 3 条的边核对**按支**执行：`Node` 一支核对 `producer_node →
    // verified_node` 的 `EdgeKind::Evidence` 边（§238 的语义面由 P1 设计 §8.2 定为
    // 「验证关系。指向被验证节点」），缺则拒；`Human` 一支没有产生者节点（§4.3.3），
    // 列写 NULL、不核对。不变量「一行 evidence 对应一枚该边」与 Human 一支不能同真，
    // 本 task 的处置（可空列 ＋ 按支核对）与其申报见文件头。
    let producer_node = match evidence.producer() {
        EvidenceProducer::Node { node, .. } => {
            let has_edge = graph.edges().iter().any(|edge| {
                edge.from_node == *node
                    && edge.to_node == *verified_node
                    && edge.kind == EdgeKind::Evidence
            });
            if !has_edge {
                return Err(VerifyError::MissingEvidenceEdge {
                    evidence_id: evidence.id().clone(),
                    graph_id: graph.id.clone(),
                    producer_node: node.clone(),
                    verified_node: verified_node.clone(),
                });
            }
            Some(node.as_str())
        }
        // Human 一支：没有产生者节点（§4.3.3），列写 NULL。
        EvidenceProducer::Human => None,
    };

    // 只 INSERT，无 UPDATE（§4.1「证据记录只追加」）。二次写同一 `evidence_id` 撞
    // 主键得 `Err`、原行不变（照片：`tests/persist.rs` 的
    // `an_evidence_row_is_insert_only`——有效性判定与证据必须是两条记录，否则
    // 「验证者改了被验之物」）。
    tx.execute(
        "INSERT INTO evidence (evidence_id, graph_id, verified_node, producer_node,
                               subject_kind, subject_requirement, evidence_type, claim,
                               strength, scope, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        &[
            Value::text(evidence.id().as_str()),
            Value::text(graph.id.as_str()),
            Value::text(verified_node.as_str()),
            match producer_node {
                Some(node) => Value::text(node),
                None => Value::Null,
            },
            // `subject_kind` 是判别串（两臂的编码）；`subject_requirement` 是
            // Requirement 载荷的串，Unattached 写作 NULL（§10.2 的两列）。
            Value::text(evidence.subject().as_str()),
            match evidence.subject() {
                EvidenceSubject::Requirement(requirement) => Value::text(requirement.as_str()),
                EvidenceSubject::Unattached => Value::Null,
            },
            Value::text(evidence.evidence_type().as_str()),
            Value::text(evidence.claim().as_str()),
            // strength / scope 是 TEXT/JSON（§10.2）：值照存，不判（§4.3.4：
            // 无取值域、无比较规则，本块只做结构性保存）。
            Value::text(
                serde_json::to_string(evidence.strength().as_value())
                    .expect("serde_json::Value 必可序列化"),
            ),
            Value::text(
                serde_json::to_string(evidence.scope().as_value())
                    .expect("serde_json::Value 必可序列化"),
            ),
            Value::Int(occurred_at),
        ],
    )?;

    // 制品引用逐条落 `evidence_artifact`（复合主键拒同一 (evidence_id, artifact_id)
    // 的第二行）。空引用（`HumanConfirmation`）时本循环不跑。
    for artifact_id in evidence.artifact_refs() {
        tx.execute(
            "INSERT INTO evidence_artifact (evidence_id, artifact_id) VALUES (?1, ?2)",
            &[
                Value::text(evidence.id().as_str()),
                Value::text(artifact_id.as_str()),
            ],
        )?;
    }
    Ok(())
}

/// 写一条有效性记录（五臂齐已在 Task 3 的构造点把关）。
///
/// **本函数不 `commit`**（文件头）；`Err` 之后调用方必须回滚。两处写入：`test_validity`
/// 一行（聚合判定）＋ `test_validity_check` 逐检查一行（§10.2 的两张表）。
///
/// 本函数**不重复校验**「五臂齐」：那是 [`TestValidity::new`] 的职责（字段开 `pub`
/// 是设计 §6.4 原文，字面量构造绕过 `new` 的残余由 Task 3 的文档照实记录）——在这里
/// 加第二条守卫就是发明一处规范没给的检查，且与「把关在构造点」的申报相抵。
pub fn insert_test_validity(tx: &Tx<'_>, validity: &TestValidity) -> Result<(), VerifyError> {
    tx.execute(
        "INSERT INTO test_validity (evidence_id, verdict) VALUES (?1, ?2)",
        &[
            Value::text(validity.evidence.as_str()),
            Value::text(validity.verdict.as_str()),
        ],
    )?;
    for (check, verdict) in &validity.checks {
        tx.execute(
            "INSERT INTO test_validity_check (evidence_id, check_name, verdict)
             VALUES (?1, ?2, ?3)",
            &[
                Value::text(validity.evidence.as_str()),
                Value::text(check.as_str()),
                Value::text(verdict.as_str()),
            ],
        )?;
    }
    Ok(())
}

/// 记一次判定：**逐条 clamp（§7.5）→ 聚合（§7.5 的五臂）→ 核对消费规则（§9.1）→ 写行**；
/// 聚合落 `Failed` / `AdjudicationRequired` 时**同事务**写一条 `EventType::VerificationFailed`
/// （payload 含逐项状态）。**不写 `IntentCompleted`**（本块不产生「完成」）。
/// **不写审计**：§313 的必录清单里没有验证类事件，故本块不新增 `AuditKind`。
///
/// # 次序与拒绝点
///
/// 前三步都不落库：`clamp` 逐条改判（无物可读时 `Pass` 变 `Unknown`）、聚合五臂
/// （声明序即优先序）、§9.1 第 3 条的消费规则核对（`CounterexampleFound` 时聚合
/// 不得取两个通过臂——**只拒绝那一对组合、不改写聚合值**）。核对不过即 `Err`，
/// **不写行、不写事件**。写入次序：先 `verification_round` 行、后事件（同
/// `apply_transition` / `save_artifact` 的次序）。
///
/// # 参数面
///
/// `selection` 落 `selection(JSON)` 列（三枚各取自己的编码串）；`independence` 落
/// `independence` 列——两列都由本函数写，调用方通常传同源的两枚（`selection.independence`），
/// 本函数不替调用方断言两者一致。`initial_input` 只读一个值：
/// [`InitialVerifierInput::has_readable_artifact`]（clamp 的入参来源）；其余字段
/// （`contract` / `evidence` / `artifacts` 的内容）不参与本函数。
///
/// **`event_id` 的唯一性由调用方保证**（`events.event_id` 是主键；事件只在两个失败聚合
/// 臂上写，故复用的 id 会在第二次撞键——见 P1 `apply_transition` 对同一处的说明）。
/// `occurred_at` 由调用方给（本层不取时钟）。
///
/// **本函数不 `commit`**（文件头）；`Err` 之后调用方必须回滚，不得提交。
pub fn record_verification_round(
    tx: &Tx<'_>,
    round_id: &str,
    graph_id: &GraphId,
    verified_node: &NodeId,
    selection: &VerifierSelection,
    initial_input: &InitialVerifierInput,
    verdicts: &[VerifierVerdict],
    independence: IndependenceVerdict,
    outcome: &AdversaryOutcome,
    event_id: &str,
    occurred_at: i64,
) -> Result<VerdictAggregate, VerifyError> {
    // (1) 逐条 clamp（§7.5：无物可读 ⇒ Unknown，不得 Pass）。
    let readable = initial_input.has_readable_artifact();
    let clamped: Vec<VerifierVerdict> = verdicts
        .iter()
        .map(|verdict| clamp_by_readability(readable, verdict.clone()))
        .collect();

    // (2) 聚合（§7.5 的五臂，声明序即优先序）。
    let aggregate = VerdictAggregate::from_verdicts(&clamped);

    // (3) §9.1 第 3 条的消费规则：`CounterexampleFound` 时聚合不得取 `Passed` /
    // `PassedWithUnknown`（§215 的「必须增强验证体系」落在这一条上）。核对排在写行
    // 之前：被拒的一轮不留半成品（照片：`tests/persist.rs` 的
    // `a_counterexample_with_a_passing_aggregate_cannot_be_recorded`）。
    if !adversary_permits_passing(&aggregate, outcome) {
        return Err(VerifyError::CounterexampleWithPassingAggregate {
            round_id: round_id.to_owned(),
            aggregate,
        });
    }

    // (4a) 写判定行。`selection` 列是 JSON：三枚各取自己的编码串（不依赖 serde 的
    // 表示）；`aggregate` / `independence` / `strategy_verdict` 三列各取各的 `as_str`。
    tx.execute(
        "INSERT INTO verification_round (round_id, graph_id, verified_node, selection,
                                         aggregate, independence, strategy_verdict, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        &[
            Value::text(round_id),
            Value::text(graph_id.as_str()),
            Value::text(verified_node.as_str()),
            Value::text(
                serde_json::to_string(&serde_json::json!({
                    "level": selection.level.as_str(),
                    "backend": selection.backend.as_str(),
                    "independence": selection.independence.as_str(),
                }))
                .expect("selection 的 JSON 由字面构造，必可序列化"),
            ),
            Value::text(aggregate.as_str()),
            Value::text(independence.as_str()),
            Value::text(outcome.as_str()),
            Value::Int(occurred_at),
        ],
    )?;

    // (4b) §10.3：聚合落 `Failed` / `AdjudicationRequired` 时**同事务**写一条
    // `VerificationFailed`，payload 含逐项状态（clamp 后的逐条判定，判别串取自
    // `VerifierVerdict::as_str`）。九类既有事件不调 `with_ignorable`（`event.rs`
    // 的原话）；**不写审计**（§313 的必录清单里没有验证类事件）。
    if matches!(
        aggregate,
        VerdictAggregate::Failed | VerdictAggregate::AdjudicationRequired
    ) {
        let event = Event::new(
            EventType::VerificationFailed,
            event_id.to_owned(),
            occurred_at,
            serde_json::json!({
                "round_id": round_id,
                "graph_id": graph_id.as_str(),
                "verified_node": verified_node.as_str(),
                "aggregate": aggregate.as_str(),
                "independence": independence.as_str(),
                "verdicts": clamped.iter().map(VerifierVerdict::as_str).collect::<Vec<_>>(),
            }),
        )
        .with_node(verified_node.as_str().to_owned());
        tx.append_event(&event)?;
    }

    Ok(aggregate)
}

/// §266 第 1 项：受验图的**每个终端节点**至少有一个产出制品（读 `artifact` 表的 `producer_node`）。
/// **不读 `Artifact.metadata`**（其 schema 规范未给，§15 第 9 条）。
///
/// # 空终端集 ⇒ `true`（照实钉住的 fail-open 形状）
///
/// 空集时「各自至少有一个」空洞为真。**上游没有守卫**：实测
/// `AdfirGraph::validate()`（`crates/continuum-graph/src/graph.rs`）不要求
/// `terminal_nodes` 非空，故空终端集是一条可达输入。本块**照实钉住这个取值、不改它**
/// ——改它就要替 §266 第 1 项发明一条「空图不算存在」的判据；此形状已报给设计作者
/// （计划 `## 遗留` 第一节第 9 条）。照片：`tests/persist.rs` 的
/// `artifact_exists_over_an_empty_terminal_set_is_true_and_that_is_a_fail_open_shape`。
///
/// **本函数不 `commit`**（文件头）：它只读，但经调用方的 `Tx` 读。
pub fn artifact_exists(tx: &Tx<'_>, graph: &AdfirGraph) -> Result<bool, VerifyError> {
    for terminal in graph.terminal_nodes() {
        let produced = one_int(
            tx,
            "SELECT EXISTS(SELECT 1 FROM artifact WHERE producer_node = ?1)",
            &[Value::text(terminal.as_str())],
        )?;
        if produced == 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// 一条 SQL 的结果里取恰一个整数（`artifact_exists` 的 `EXISTS` 查询）。
fn one_int(tx: &Tx<'_>, sql: &str, params: &[Value]) -> Result<i64, VerifyError> {
    let rows = tx.query(sql, params)?;
    match rows.into_iter().next().and_then(|row| row.into_iter().next()) {
        Some(Value::Int(i)) => Ok(i),
        other => Err(VerifyError::Persist(PersistError::ColumnType {
            index: 0,
            actual: other
                .as_ref()
                .map(continuum_persist::value::kind_name)
                .unwrap_or("missing"),
        })),
    }
}
