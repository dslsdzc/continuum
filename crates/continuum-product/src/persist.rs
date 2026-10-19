//! 迁移 200 与产品循环的落库／读回。
//!
//! C2=(b) 已批准：`product_drive` 是第四张表。本模块是内部 JSON 编码的唯一产生点；
//! 编码不是公开序列化契约，未知／缺失键与表外枚举值一律 `CorruptRow`，不取默认值。
//!
//! 所有写 API **不自己 `commit`**：`Err` 之后由调用方回滚。

use continuum_canonical::Opaque;
use continuum_persist::{Migration, Tx, Value};
use continuum_semantics::{
    Decision, DecisionId, IntentError, IntentKind, IntentStatus, ProductIdRef, create_decision,
    load_intent,
};
use serde_json::{Map, Value as JsonValue, json};

use crate::baseline::EngineeringBaseline;
use crate::drive::{ContinuousProductDrive, ProductDriveState, ProductPhase};
use crate::error::ProductError;
use crate::milestone::{Milestone, MilestoneId, MilestoneState};
use crate::predicate::{ShippableClause, ShippableClauseId, ShippablePredicate};
use crate::product::Product;
use crate::progress::{
    ProgressDelta, ProgressGateVerdict, ProgressPolicy, ProgressRecord, ReplanDraft,
    evaluate_progress,
};
use crate::readiness::{
    ProductReadiness, ReadinessDimension, ReadinessDimensionName, ReadinessValue,
};
use continuum_verify::Evidence;

/// 迁移 200，名称 `p7a_product`。版本恰 `[200]`。
///
/// 四张表：`product`／`milestone`／`product_progress`／`product_drive`。
/// **本 SQL 一经应用不得再改**；P7A 只占 200，不占 201–209。
pub fn p7a_product_migrations() -> Vec<Migration> {
    vec![Migration::new(
        200,
        "p7a_product",
        "CREATE TABLE product (
            id                   TEXT PRIMARY KEY NOT NULL,
            intent_id            TEXT NOT NULL UNIQUE,
            baseline             TEXT NOT NULL,
            readiness            TEXT NOT NULL,
            current_milestone    TEXT,
            shippable_predicate  TEXT NOT NULL,
            created_at           INTEGER NOT NULL,
            updated_at           INTEGER NOT NULL
        );
        CREATE TABLE milestone (
            id                   TEXT PRIMARY KEY NOT NULL,
            product_id           TEXT NOT NULL,
            seq                  INTEGER NOT NULL,
            objective            TEXT NOT NULL,
            dependencies         TEXT NOT NULL,
            entry_conditions     TEXT NOT NULL,
            exit_conditions      TEXT NOT NULL,
            state                TEXT NOT NULL,
            created_at           INTEGER NOT NULL,
            updated_at           INTEGER NOT NULL,
            UNIQUE (product_id, seq)
        );
        CREATE TABLE product_progress (
            product_id             TEXT NOT NULL,
            milestone_id           TEXT NOT NULL,
            previous_product_state TEXT NOT NULL,
            current_product_state  TEXT NOT NULL,
            progress_delta         TEXT NOT NULL,
            recorded_at            INTEGER NOT NULL,
            PRIMARY KEY (product_id, milestone_id)
        );
        CREATE TABLE product_drive (
            product_id                TEXT PRIMARY KEY NOT NULL,
            continuous_product_drive  INTEGER NOT NULL,
            phase                     TEXT NOT NULL,
            updated_at                INTEGER NOT NULL
        );
        CREATE INDEX idx_milestone_product_seq ON milestone(product_id, seq);",
    )]
}

// ===========================================================================
// 行读取与损坏行
// ===========================================================================

fn corrupt(table: &'static str, detail: impl Into<String>) -> ProductError {
    ProductError::CorruptRow {
        table,
        detail: detail.into(),
    }
}

fn text_at(row: &[Value], index: usize, table: &'static str) -> Result<String, ProductError> {
    match &row[index] {
        Value::Text(s) => Ok(s.clone()),
        other => Err(corrupt(
            table,
            format!("第 {index} 列应为文本，实际 {other:?}"),
        )),
    }
}

fn int_at(row: &[Value], index: usize, table: &'static str) -> Result<i64, ProductError> {
    match &row[index] {
        Value::Int(i) => Ok(*i),
        other => Err(corrupt(
            table,
            format!("第 {index} 列应为整数，实际 {other:?}"),
        )),
    }
}

fn optional_text_at(
    row: &[Value],
    index: usize,
    table: &'static str,
) -> Result<Option<String>, ProductError> {
    match &row[index] {
        Value::Null => Ok(None),
        Value::Text(s) => Ok(Some(s.clone())),
        other => Err(corrupt(
            table,
            format!("第 {index} 列应为文本或 NULL，实际 {other:?}"),
        )),
    }
}

// ===========================================================================
// 内部 JSON 编码（baseline／readiness／predicate）
// ===========================================================================

const BASELINE_KEYS: [&str; 10] = [
    "product_goal",
    "target_users",
    "required_capabilities",
    "hard_constraints",
    "architecture",
    "technology_choices",
    "minimum_complete_system",
    "known_unknowns",
    "engineering_risks",
    "acceptance_criteria",
];

const NINE: [ReadinessDimensionName; 9] = [
    ReadinessDimensionName::Functional,
    ReadinessDimensionName::Correctness,
    ReadinessDimensionName::Reliability,
    ReadinessDimensionName::Performance,
    ReadinessDimensionName::Security,
    ReadinessDimensionName::Usability,
    ReadinessDimensionName::Maintainability,
    ReadinessDimensionName::Deployability,
    ReadinessDimensionName::Documentation,
];

const PREDICATE_KEYS: [&str; 2] = ["required_dimensions", "clauses"];

fn parse_json(raw: &str, table: &'static str) -> Result<JsonValue, ProductError> {
    serde_json::from_str(raw)
        .map_err(|e| corrupt(table, format!("JSON 解析失败: {e}")))
}

fn object_exact<'a>(
    value: &'a JsonValue,
    keys: &[&str],
    table: &'static str,
    what: &str,
) -> Result<&'a Map<String, JsonValue>, ProductError> {
    let object = value
        .as_object()
        .ok_or_else(|| corrupt(table, format!("{what} 不是对象")))?;
    if object.len() != keys.len()
        || !object.keys().all(|key| keys.contains(&key.as_str()))
    {
        return Err(corrupt(
            table,
            format!("{what} 的键集不符：实际 {:?}", object.keys().collect::<Vec<_>>()),
        ));
    }
    Ok(object)
}

fn string_field(
    object: &Map<String, JsonValue>,
    key: &str,
    table: &'static str,
    what: &str,
) -> Result<String, ProductError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .map(str::to_owned)
        .ok_or_else(|| corrupt(table, format!("{what}.{key} 应为文本")))
}

fn string_array(
    value: &JsonValue,
    table: &'static str,
    what: &str,
) -> Result<Vec<String>, ProductError> {
    let items = value
        .as_array()
        .ok_or_else(|| corrupt(table, format!("{what} 不是数组")))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| corrupt(table, format!("{what} 的元素应为文本")))
        })
        .collect()
}

fn opaque_array(
    value: &JsonValue,
    table: &'static str,
    what: &str,
) -> Result<Vec<Opaque>, ProductError> {
    Ok(string_array(value, table, what)?
        .into_iter()
        .map(|s| Opaque::from_str(&s))
        .collect())
}

fn encode_baseline(baseline: &EngineeringBaseline) -> String {
    let strings = |items: &[Opaque]| -> Vec<JsonValue> {
        items
            .iter()
            .map(|item| JsonValue::String(item.as_str().to_owned()))
            .collect()
    };
    json!({
        "product_goal": baseline.product_goal.as_str(),
        "target_users": strings(&baseline.target_users),
        "required_capabilities": strings(&baseline.required_capabilities),
        "hard_constraints": strings(&baseline.hard_constraints),
        "architecture": baseline.architecture.as_str(),
        "technology_choices": strings(&baseline.technology_choices),
        "minimum_complete_system": strings(&baseline.minimum_complete_system),
        "known_unknowns": strings(&baseline.known_unknowns),
        "engineering_risks": strings(&baseline.engineering_risks),
        "acceptance_criteria": strings(&baseline.acceptance_criteria),
    })
    .to_string()
}

fn decode_baseline(raw: &str) -> Result<EngineeringBaseline, ProductError> {
    let value = parse_json(raw, "product")?;
    let object = object_exact(&value, &BASELINE_KEYS, "product", "baseline")?;
    let field = |key: &str| string_field(object, key, "product", "baseline");
    let array = |key: &str| opaque_array(&object[key], "product", &format!("baseline.{key}"));
    Ok(EngineeringBaseline {
        product_goal: Opaque::from_str(&field("product_goal")?),
        target_users: array("target_users")?,
        required_capabilities: array("required_capabilities")?,
        hard_constraints: array("hard_constraints")?,
        architecture: Opaque::from_str(&field("architecture")?),
        technology_choices: array("technology_choices")?,
        minimum_complete_system: array("minimum_complete_system")?,
        known_unknowns: array("known_unknowns")?,
        engineering_risks: array("engineering_risks")?,
        acceptance_criteria: array("acceptance_criteria")?,
    })
}

fn encode_readiness(readiness: &ProductReadiness) -> String {
    let mut object = Map::new();
    for (name, dimension) in readiness.iter() {
        let value = match dimension.value() {
            ReadinessValue::Unassessed => JsonValue::Null,
            ReadinessValue::Assessed(value) => {
                JsonValue::String(value.as_str().to_owned())
            }
        };
        let evidence: Vec<JsonValue> = dimension
            .evidence()
            .iter()
            .map(|id| JsonValue::String(id.as_str().to_owned()))
            .collect();
        let last_verified = match dimension.last_verified() {
            Some(at) => JsonValue::from(at),
            None => JsonValue::Null,
        };
        object.insert(
            name.as_str().to_owned(),
            json!({
                "value": value,
                "evidence": evidence,
                "last_verified": last_verified,
            }),
        );
    }
    JsonValue::Object(object).to_string()
}

fn decode_readiness(raw: &str) -> Result<ProductReadiness, ProductError> {
    let value = parse_json(raw, "product")?;
    let names: Vec<&str> = NINE.iter().map(ReadinessDimensionName::as_str).collect();
    let object = object_exact(&value, &names, "product", "readiness")?;
    let mut readiness = ProductReadiness::unassessed();
    for name in NINE {
        let what = format!("readiness.{}", name.as_str());
        let dimension = object_exact(
            &object[name.as_str()],
            &["value", "evidence", "last_verified"],
            "product",
            &what,
        )?;
        let evidence = string_array(&dimension["evidence"], "product", &format!("{what}.evidence"))?;
        let last_verified = &dimension["last_verified"];
        let decoded = match &dimension["value"] {
            JsonValue::Null => {
                if !evidence.is_empty() {
                    return Err(corrupt(
                        "product",
                        format!("{what} 无值却有 evidence"),
                    ));
                }
                if !last_verified.is_null() {
                    return Err(corrupt(
                        "product",
                        format!("{what} 无值却有 last_verified"),
                    ));
                }
                ReadinessDimension::unassessed()
            }
            JsonValue::String(value) => {
                if value.is_empty() {
                    return Err(corrupt("product", format!("{what} 的 value 为空")));
                }
                if evidence.is_empty() {
                    return Err(corrupt(
                        "product",
                        format!("{what} 有值却无 evidence"),
                    ));
                }
                let Some(last_verified) = last_verified.as_i64() else {
                    return Err(corrupt(
                        "product",
                        format!("{what} 有值却无 last_verified"),
                    ));
                };
                ReadinessDimension::assessed(
                    Opaque::from_str(value),
                    evidence
                        .into_iter()
                        .map(|id| continuum_verify::EvidenceId::from_str(&id))
                        .collect(),
                    last_verified,
                )
                .map_err(|e| corrupt("product", format!("{what} 的三元组不合法: {e}")))?
            }
            _ => {
                return Err(corrupt(
                    "product",
                    format!("{what} 的 value 应为文本或 null"),
                ))
            }
        };
        readiness = readiness.with_dimension(name, decoded);
    }
    Ok(readiness)
}

fn encode_predicate(predicate: &ShippablePredicate) -> String {
    let dimensions: Vec<JsonValue> = predicate
        .required_dimensions()
        .iter()
        .map(|name| JsonValue::String(name.as_str().to_owned()))
        .collect();
    let clauses: Vec<JsonValue> = predicate
        .clauses()
        .iter()
        .map(|clause| {
            json!({
                "id": clause.id().as_str(),
                "statement": clause.statement().as_str(),
            })
        })
        .collect();
    json!({"required_dimensions": dimensions, "clauses": clauses}).to_string()
}

fn decode_predicate(raw: &str) -> Result<ShippablePredicate, ProductError> {
    let value = parse_json(raw, "product")?;
    let object = object_exact(&value, &PREDICATE_KEYS, "product", "shippable_predicate")?;
    let dimensions = string_array(
        &object["required_dimensions"],
        "product",
        "shippable_predicate.required_dimensions",
    )?
    .into_iter()
    .map(|name| {
        ReadinessDimensionName::parse(&name)
            .ok_or_else(|| corrupt("product", format!("表外的 required dimension: {name}")))
    })
    .collect::<Result<Vec<_>, _>>()?;

    let clauses = object["clauses"]
        .as_array()
        .ok_or_else(|| corrupt("product", "shippable_predicate.clauses 不是数组"))?
        .iter()
        .map(|clause| {
            let clause = object_exact(
                clause,
                &["id", "statement"],
                "product",
                "shippable_predicate.clause",
            )?;
            let id = string_field(clause, "id", "product", "clause")?;
            let statement = string_field(clause, "statement", "product", "clause")?;
            ShippableClause::new(
                ShippableClauseId::from_str(&id),
                Opaque::from_str(&statement),
            )
            .map_err(|e| corrupt("product", format!("子句不合法: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    ShippablePredicate::new(dimensions, clauses)
        .map_err(|e| corrupt("product", format!("谓词不合法: {e}")))
}

// ===========================================================================
// Task 10：Product 落库与读回
// ===========================================================================

/// 校验锚 Intent 的四条写路径不变量后写入 `product`；同 id 二次写 ⇒ `ProductExists`。
///
/// C2=(b)：同一 `Tx` 写 `product_drive` 初始行（0／`pre_shippable`）。**不 commit。**
pub fn save_product(
    tx: &Tx<'_>,
    product: &Product,
    occurred_at: i64,
) -> Result<(), ProductError> {
    let anchor_id = product.intent_id().as_str().to_owned();
    let anchor = load_intent(tx, product.intent_id())?;
    let Some(anchor) = anchor else {
        return Err(ProductError::Intent(IntentError::UnknownIntent {
            id: anchor_id,
        }));
    };
    if anchor.kind != IntentKind::Product {
        return Err(ProductError::IntentNotProduct {
            id: anchor.id.as_str().to_owned(),
        });
    }
    match &anchor.product_id {
        Some(id) if id.as_str() == product.id().as_str() => {}
        _ => {
            return Err(ProductError::MissingProductId {
                intent: anchor.id.as_str().to_owned(),
            })
        }
    }
    if anchor.contract_id.id.as_str().is_empty()
        || anchor.contract_id.version < 1
    {
        return Err(ProductError::InvalidContract {
            intent: anchor.id.as_str().to_owned(),
        });
    }
    if anchor.status != IntentStatus::Ready {
        return Err(ProductError::AnchorIntentNotReady {
            id: anchor.id.as_str().to_owned(),
            status: crate::product::status_name(anchor.status),
        });
    }

    let existing = tx.query(
        "SELECT 1 FROM product WHERE id = ?1",
        &[Value::text(product.id().as_str())],
    )?;
    if !existing.is_empty() {
        return Err(ProductError::ProductExists {
            id: product.id().as_str().to_owned(),
        });
    }

    tx.execute(
        "INSERT INTO product
           (id, intent_id, baseline, readiness, current_milestone,
            shippable_predicate, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        &[
            Value::text(product.id().as_str()),
            Value::text(product.intent_id().as_str()),
            Value::text(encode_baseline(product.baseline())),
            Value::text(encode_readiness(product.readiness())),
            product
                .current_milestone()
                .map(|id| Value::text(id.as_str()))
                .unwrap_or(Value::Null),
            Value::text(encode_predicate(product.shippable_predicate())),
            Value::Int(occurred_at),
        ],
    )?;

    tx.execute(
        "INSERT INTO product_drive
           (product_id, continuous_product_drive, phase, updated_at)
         VALUES (?1, 0, 'pre_shippable', ?2)",
        &[
            Value::text(product.id().as_str()),
            Value::Int(occurred_at),
        ],
    )?;
    Ok(())
}

/// 读回 Product 并逐项复核（I6）：baseline 可用、predicate 重走构造校验、
/// readiness 三元组合法；任一违反 ⇒ `CorruptRow`。行不存在 ⇒ `Ok(None)`。
pub fn load_product(
    tx: &Tx<'_>,
    id: &ProductIdRef,
) -> Result<Option<Product>, ProductError> {
    let rows = tx.query(
        "SELECT id, intent_id, baseline, readiness, current_milestone, shippable_predicate
         FROM product WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };

    let stored_id = text_at(&row, 0, "product")?;
    let intent_id = text_at(&row, 1, "product")?;
    let baseline = decode_baseline(text_at(&row, 2, "product")?.as_str())?;
    let readiness = decode_readiness(text_at(&row, 3, "product")?.as_str())?;
    let predicate = decode_predicate(text_at(&row, 5, "product")?.as_str())?;
    let current = optional_text_at(&row, 4, "product")?;

    let history: Vec<MilestoneId> = tx
        .query(
            "SELECT id FROM milestone WHERE product_id = ?1 ORDER BY seq",
            &[Value::text(&stored_id)],
        )?
        .iter()
        .map(|row| Ok(MilestoneId::from_str(&text_at(row, 0, "product")?)))
        .collect::<Result<Vec<_>, ProductError>>()?;

    if let Some(current) = &current {
        let rows = tx.query(
            "SELECT product_id, state FROM milestone WHERE id = ?1",
            &[Value::text(current)],
        )?;
        let Some(row) = rows.into_iter().next() else {
            return Err(corrupt(
                "product",
                format!("current_milestone 指向不存在的 Milestone: {current}"),
            ));
        };
        let owner = text_at(&row, 0, "product")?;
        if owner != stored_id {
            return Err(corrupt(
                "product",
                format!("current_milestone 属于别的 Product: {current}"),
            ));
        }
        let state = text_at(&row, 1, "product")?;
        let state = MilestoneState::parse(&state)
            .ok_or_else(|| corrupt("product", format!("表外的 Milestone state: {state}")))?;
        if is_terminal(state) {
            return Err(corrupt(
                "product",
                format!("current_milestone 处于终态: {current}"),
            ));
        }
    }

    let product = Product::from_verified_parts(
        ProductIdRef::from_str(&stored_id),
        continuum_semantics::IntentId::from_str(&intent_id),
        baseline,
        readiness,
        current.map(|id| MilestoneId::from_str(&id)),
        history,
        predicate,
    )
    .map_err(|e| corrupt("product", format!("Product 复核失败: {e}")))?;
    Ok(Some(product))
}

/// 读 drive；行缺失 ⇒ `Ok(None)`；0/1 之外的开关值与表外 phase ⇒ `CorruptRow`。
pub fn load_product_drive(
    tx: &Tx<'_>,
    id: &ProductIdRef,
) -> Result<Option<ProductDriveState>, ProductError> {
    let rows = tx.query(
        "SELECT continuous_product_drive, phase FROM product_drive WHERE product_id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let enabled = match int_at(&row, 0, "product_drive")? {
        0 => false,
        1 => true,
        other => {
            return Err(corrupt(
                "product_drive",
                format!("continuous_product_drive 不是 0/1: {other}"),
            ))
        }
    };
    let phase_name = text_at(&row, 1, "product_drive")?;
    let phase = ProductPhase::parse(&phase_name)
        .ok_or_else(|| corrupt("product_drive", format!("表外的 phase: {phase_name}")))?;
    Ok(Some(ProductDriveState {
        continuous_product_drive: ContinuousProductDrive { enabled },
        phase,
    }))
}

// ===========================================================================
// Task 12：Progress 落库与读回
// ===========================================================================

fn delta_name(delta: ProgressDelta) -> &'static str {
    match delta {
        ProgressDelta::Improved => "improved",
        ProgressDelta::NoProgress => "no_progress",
        ProgressDelta::Regressed => "regressed",
        ProgressDelta::Undetermined => "undetermined",
    }
}

fn parse_delta(raw: &str) -> Result<ProgressDelta, ProductError> {
    match raw {
        "improved" => Ok(ProgressDelta::Improved),
        "no_progress" => Ok(ProgressDelta::NoProgress),
        "regressed" => Ok(ProgressDelta::Regressed),
        "undetermined" => Ok(ProgressDelta::Undetermined),
        other => Err(corrupt(
            "product_progress",
            format!("表外的 progress_delta: {other}"),
        )),
    }
}

fn decode_readiness_in(raw: &str, table: &'static str) -> Result<ProductReadiness, ProductError> {
    // 复用 product 面解码，再把错误表名替换成 progress 表。
    decode_readiness(raw).map_err(|error| match error {
        ProductError::CorruptRow { detail, .. } => ProductError::CorruptRow { table, detail },
        other => other,
    })
}

/// 写一条 Progress：完成态／跨 Product／重复／证据 id 全部在写入前核完；
/// verdict 从**写前**的 history 算出（本行不计入自己）；delta 只来自策略；
/// 写 readiness 与 `updated_at`，必要时推进游标。**不 commit。**
pub fn record_milestone_progress(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &Milestone,
    current: ProductReadiness,
    evidence: &[Evidence],
    policy: Option<&dyn ProgressPolicy>,
    occurred_at: i64,
) -> Result<(ProgressRecord, ProgressGateVerdict), ProductError> {
    if milestone.product_id().as_str() != product.id().as_str() {
        return Err(ProductError::MilestoneProductMismatch {
            milestone: milestone.id().as_str().to_owned(),
            product: product.id().as_str().to_owned(),
        });
    }
    if milestone.state() != MilestoneState::Completed {
        return Err(ProductError::MilestoneNotCompleted {
            id: milestone.id().as_str().to_owned(),
        });
    }

    let duplicate = tx.query(
        "SELECT 1 FROM product_progress WHERE product_id = ?1 AND milestone_id = ?2",
        &[
            Value::text(product.id().as_str()),
            Value::text(milestone.id().as_str()),
        ],
    )?;
    if !duplicate.is_empty() {
        return Err(ProductError::DuplicateProgress {
            product: product.id().as_str().to_owned(),
            milestone: milestone.id().as_str().to_owned(),
        });
    }

    let supplied: Vec<&str> = evidence.iter().map(|e| e.id().as_str()).collect();
    for (name, dimension) in current.iter() {
        let dimension_name = name.as_str();
        match dimension.value() {
            ReadinessValue::Unassessed => {
                if !dimension.evidence().is_empty() {
                    return Err(ProductError::ReadinessEvidenceWithoutValue {
                        dimension: dimension_name,
                    });
                }
                if dimension.last_verified().is_some() {
                    return Err(ProductError::ReadinessLastVerifiedMismatch {
                        dimension: dimension_name,
                    });
                }
            }
            ReadinessValue::Assessed(value) => {
                if value.as_str().is_empty() {
                    return Err(ProductError::EmptyReadinessValue {
                        dimension: dimension_name,
                    });
                }
                if dimension.evidence().is_empty() {
                    return Err(ProductError::ReadinessValueWithoutEvidence {
                        dimension: dimension_name,
                    });
                }
                for (i, id) in dimension.evidence().iter().enumerate() {
                    if dimension.evidence()[..i].iter().any(|prev| prev == id) {
                        return Err(ProductError::DuplicateEvidence {
                            id: id.as_str().to_owned(),
                        });
                    }
                    if !supplied.contains(&id.as_str()) {
                        return Err(ProductError::EvidenceNotSupplied {
                            id: id.as_str().to_owned(),
                        });
                    }
                }
                if dimension.last_verified().is_none() {
                    return Err(ProductError::ReadinessLastVerifiedMismatch {
                        dimension: dimension_name,
                    });
                }
            }
        }
    }

    let previous = product.readiness().clone();
    let history = load_progress(tx, product.id())?;
    let verdict = evaluate_progress(&previous, &current, &history, policy);
    let progress_delta = policy
        .map(|policy| policy.delta(&previous, &current))
        .unwrap_or(ProgressDelta::Undetermined);

    tx.execute(
        "INSERT INTO product_progress
           (product_id, milestone_id, previous_product_state, current_product_state,
            progress_delta, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        &[
            Value::text(product.id().as_str()),
            Value::text(milestone.id().as_str()),
            Value::text(encode_readiness(&previous)),
            Value::text(encode_readiness(&current)),
            Value::text(delta_name(progress_delta)),
            Value::Int(occurred_at),
        ],
    )?;

    tx.execute(
        "UPDATE product SET readiness = ?2, updated_at = ?3 WHERE id = ?1",
        &[
            Value::text(product.id().as_str()),
            Value::text(encode_readiness(&current)),
            Value::Int(occurred_at),
        ],
    )?;

    advance_cursor_if_current(tx, product.id(), milestone.id(), occurred_at)?;

    Ok((
        ProgressRecord {
            product_id: product.id().clone(),
            milestone_id: milestone.id().clone(),
            previous_product_state: previous,
            current_product_state: current,
            progress_delta,
            recorded_at: occurred_at,
        },
        verdict,
    ))
}

/// 按 `recorded_at`／插入序升序读一个 Product 的 Progress。
pub fn load_progress(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
) -> Result<Vec<ProgressRecord>, ProductError> {
    tx.query(
        "SELECT product_id, milestone_id, previous_product_state, current_product_state,
                progress_delta, recorded_at
         FROM product_progress WHERE product_id = ?1
         ORDER BY recorded_at, rowid",
        &[Value::text(product_id.as_str())],
    )?
    .iter()
    .map(|row| {
        Ok(ProgressRecord {
            product_id: ProductIdRef::from_str(&text_at(row, 0, "product_progress")?),
            milestone_id: MilestoneId::from_str(&text_at(row, 1, "product_progress")?),
            previous_product_state: decode_readiness_in(
                &text_at(row, 2, "product_progress")?,
                "product_progress",
            )?,
            current_product_state: decode_readiness_in(
                &text_at(row, 3, "product_progress")?,
                "product_progress",
            )?,
            progress_delta: parse_delta(&text_at(row, 4, "product_progress")?)?,
            recorded_at: int_at(row, 5, "product_progress")?,
        })
    })
    .collect()
}

// ===========================================================================
// Task 13：只出 replan Decision，不执行 replan
// ===========================================================================

/// 从**去重后的写前 history** 重算 Progress Gate；`Triggered` 时写一张 `Decision`
/// 与一条 `decision.required` 事件，其余两值零行零事件。
///
/// 签名里没有 `ProgressGateVerdict`：调用方无法把伪造的 `Triggered` 传进来。
pub fn emit_replan_decision(
    tx: &Tx<'_>,
    product: &Product,
    record: &ProgressRecord,
    history: &[ProgressRecord],
    policy: Option<&dyn ProgressPolicy>,
    draft: ReplanDraft,
    occurred_at: i64,
) -> Result<Option<Decision>, ProductError> {
    if record.product_id.as_str() != product.id().as_str() {
        return Err(ProductError::MilestoneProductMismatch {
            milestone: record.milestone_id.as_str().to_owned(),
            product: product.id().as_str().to_owned(),
        });
    }
    let Some(milestone) = load_milestone(tx, &record.milestone_id)? else {
        return Err(ProductError::UnknownMilestone {
            id: record.milestone_id.as_str().to_owned(),
        });
    };
    if milestone.product_id().as_str() != product.id().as_str() {
        return Err(ProductError::MilestoneProductMismatch {
            milestone: record.milestone_id.as_str().to_owned(),
            product: product.id().as_str().to_owned(),
        });
    }
    if draft.issue.as_str().is_empty() {
        return Err(ProductError::EmptyReplanField { field: "issue" });
    }
    if draft.options.is_empty() {
        return Err(ProductError::EmptyReplanField { field: "options" });
    }

    // 只做两件事：去掉本记录（它作为 previous/current 参数传入），其余重复
    // milestone 保留最后一条。record 不回到 history，否则本次 NoProgress 会数两次。
    let without_record: Vec<&ProgressRecord> = history
        .iter()
        .filter(|item| item.milestone_id != record.milestone_id)
        .collect();
    let deduped: Vec<ProgressRecord> = without_record
        .iter()
        .enumerate()
        .filter(|(index, item)| {
            !without_record[index + 1..]
                .iter()
                .any(|later| later.milestone_id == item.milestone_id)
        })
        .map(|(_, item)| (*item).clone())
        .collect();

    let verdict = evaluate_progress(
        &record.previous_product_state,
        &record.current_product_state,
        &deduped,
        policy,
    );
    if !matches!(verdict, ProgressGateVerdict::Triggered { .. }) {
        return Ok(None);
    }

    let decision = Decision {
        id: DecisionId::from_str(&format!(
            "decision/{}/replan/{}",
            product.id().as_str(),
            record.milestone_id.as_str()
        )),
        intent_id: product.intent_id().clone(),
        issue: draft.issue,
        options: draft.options,
        blocking: draft.blocking,
    };
    create_decision(tx, &decision, occurred_at)?;
    Ok(Some(decision))
}

// ===========================================================================
// Task 11：Milestone 落库、状态更新与游标
// ===========================================================================

const MILESTONE_COLUMNS: &str =
    "id, product_id, seq, objective, dependencies, entry_conditions, exit_conditions, state";

fn decode_id_array(raw: &str, what: &str) -> Result<Vec<MilestoneId>, ProductError> {
    Ok(string_array(
        &parse_json(raw, "milestone")?,
        "milestone",
        what,
    )?
    .into_iter()
    .map(|id| MilestoneId::from_str(&id))
    .collect())
}

fn decode_opaque_conditions(raw: &str, what: &str) -> Result<Vec<Opaque>, ProductError> {
    opaque_array(&parse_json(raw, "milestone")?, "milestone", what)
}

fn decode_milestone(row: &[Value]) -> Result<Milestone, ProductError> {
    let id = MilestoneId::from_str(&text_at(row, 0, "milestone")?);
    let product_id = ProductIdRef::from_str(&text_at(row, 1, "milestone")?);
    let _seq = int_at(row, 2, "milestone")?;
    let objective = Opaque::from_str(&text_at(row, 3, "milestone")?);
    let dependencies = decode_id_array(
        &text_at(row, 4, "milestone")?,
        "milestone.dependencies",
    )?;
    let entry_conditions = decode_opaque_conditions(
        &text_at(row, 5, "milestone")?,
        "milestone.entry_conditions",
    )?;
    let exit_conditions = decode_opaque_conditions(
        &text_at(row, 6, "milestone")?,
        "milestone.exit_conditions",
    )?;
    let state_name = text_at(row, 7, "milestone")?;
    let state = MilestoneState::parse(&state_name)
        .ok_or_else(|| corrupt("milestone", format!("表外的 state: {state_name}")))?;
    Milestone::from_verified_parts(
        id,
        product_id,
        objective,
        dependencies,
        entry_conditions,
        exit_conditions,
        state,
    )
    .map_err(|e| corrupt("milestone", format!("Milestone 复核失败: {e}")))
}

/// 保存一个批次：开头的 `load_product` 防孤儿；批内查重先于一切 INSERT；
/// 逐枚校验 product_id／state／objective／conditions／id 与全局 ordinal；
/// 评估门（已完成但无 progress）在任何 INSERT 之前。
pub fn save_milestones(
    tx: &Tx<'_>,
    product: &Product,
    milestones: &[Milestone],
    occurred_at: i64,
) -> Result<(), ProductError> {
    if load_product(tx, product.id())?.is_none() {
        return Err(ProductError::UnknownProduct {
            id: product.id().as_str().to_owned(),
        });
    }

    for (i, milestone) in milestones.iter().enumerate() {
        if milestones[..i]
            .iter()
            .any(|previous| previous.id() == milestone.id())
        {
            return Err(ProductError::MilestoneIdCollision {
                id: milestone.id().as_str().to_owned(),
            });
        }
    }

    let base = match tx
        .query(
            "SELECT COALESCE(MAX(seq), -1) + 1 FROM milestone WHERE product_id = ?1",
            &[Value::text(product.id().as_str())],
        )?
        .first()
        .and_then(|row| row.first())
    {
        Some(Value::Int(seq)) => *seq,
        other => {
            return Err(corrupt(
                "milestone",
                format!("MAX(seq) 不是整数: {other:?}"),
            ))
        }
    };

    for (offset, milestone) in milestones.iter().enumerate() {
        let seq = base + offset as i64;
        if milestone.product_id().as_str() != product.id().as_str() {
            return Err(ProductError::MilestoneProductMismatch {
                milestone: milestone.id().as_str().to_owned(),
                product: product.id().as_str().to_owned(),
            });
        }
        if milestone.state() != MilestoneState::Proposed {
            return Err(ProductError::MilestoneStateMismatch {
                id: milestone.id().as_str().to_owned(),
                expected: MilestoneState::Proposed.as_str().to_owned(),
                actual: milestone.state().as_str().to_owned(),
            });
        }
        if milestone.objective().as_str().is_empty() {
            return Err(ProductError::EmptyMilestoneObjective);
        }
        for (field, conditions) in [
            ("entry_conditions", milestone.entry_conditions()),
            ("exit_conditions", milestone.exit_conditions()),
        ] {
            for (index, condition) in conditions.iter().enumerate() {
                if condition.as_str().is_empty() {
                    return Err(ProductError::EmptyMilestoneCondition { field, index });
                }
            }
        }
        let exists = tx.query(
            "SELECT 1 FROM milestone WHERE id = ?1",
            &[Value::text(milestone.id().as_str())],
        )?;
        if !exists.is_empty() {
            return Err(ProductError::MilestoneIdCollision {
                id: milestone.id().as_str().to_owned(),
            });
        }
        let suffix = milestone
            .id()
            .as_str()
            .rsplit('/')
            .next()
            .unwrap_or_default();
        let actual = suffix.parse::<u64>().unwrap_or(u64::MAX);
        if actual != seq as u64 {
            return Err(ProductError::MilestoneOrdinalMismatch {
                id: milestone.id().as_str().to_owned(),
                expected: seq as u64,
                actual,
            });
        }
    }

    let missing = tx.query(
        "SELECT m.id FROM milestone m
         LEFT JOIN product_progress p
           ON p.product_id = m.product_id AND p.milestone_id = m.id
         WHERE m.product_id = ?1 AND m.state = 'completed' AND p.milestone_id IS NULL
         LIMIT 1",
        &[Value::text(product.id().as_str())],
    )?;
    if let Some(row) = missing.first() {
        return Err(ProductError::MissingProgressForCompleted {
            milestone: text_at(row, 0, "milestone")?,
        });
    }

    for (offset, milestone) in milestones.iter().enumerate() {
        let seq = base + offset as i64;
        tx.execute(
            "INSERT INTO milestone
               (id, product_id, seq, objective, dependencies, entry_conditions,
                exit_conditions, state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            &[
                Value::text(milestone.id().as_str()),
                Value::text(product.id().as_str()),
                Value::Int(seq),
                Value::text(milestone.objective().as_str()),
                Value::text(
                    serde_json::to_string(
                        &milestone
                            .dependencies()
                            .iter()
                            .map(|id| id.as_str())
                            .collect::<Vec<_>>(),
                    )
                    .expect("字符串数组可序列化"),
                ),
                Value::text(
                    serde_json::to_string(
                        &milestone
                            .entry_conditions()
                            .iter()
                            .map(Opaque::as_str)
                            .collect::<Vec<_>>(),
                    )
                    .expect("字符串数组可序列化"),
                ),
                Value::text(
                    serde_json::to_string(
                        &milestone
                            .exit_conditions()
                            .iter()
                            .map(Opaque::as_str)
                            .collect::<Vec<_>>(),
                    )
                    .expect("字符串数组可序列化"),
                ),
                Value::text(milestone.state().as_str()),
                Value::Int(occurred_at),
            ],
        )?;
    }

    let current = load_product(tx, product.id())?.and_then(|stored| {
        stored
            .current_milestone()
            .map(|id| id.as_str().to_owned())
    });
    if current.is_none() {
        if let Some(first) = milestones.first() {
            tx.execute(
                "UPDATE product SET current_milestone = ?2, updated_at = ?3 WHERE id = ?1",
                &[
                    Value::text(product.id().as_str()),
                    Value::text(first.id().as_str()),
                    Value::Int(occurred_at),
                ],
            )?;
            return Ok(());
        }
    }
    tx.execute(
        "UPDATE product SET updated_at = ?2 WHERE id = ?1",
        &[
            Value::text(product.id().as_str()),
            Value::Int(occurred_at),
        ],
    )?;
    Ok(())
}

/// 按 id 读一枚 Milestone；不存在 ⇒ `Ok(None)`。
pub fn load_milestone(
    tx: &Tx<'_>,
    id: &MilestoneId,
) -> Result<Option<Milestone>, ProductError> {
    let rows = tx.query(
        &format!("SELECT {MILESTONE_COLUMNS} FROM milestone WHERE id = ?1"),
        &[Value::text(id.as_str())],
    )?;
    rows.first().map(|row| decode_milestone(row)).transpose()
}

/// 按 `seq` 升序读一个 Product 的 Milestone。
pub fn load_milestones(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
) -> Result<Vec<Milestone>, ProductError> {
    tx.query(
        &format!(
            "SELECT {MILESTONE_COLUMNS} FROM milestone WHERE product_id = ?1 ORDER BY seq"
        ),
        &[Value::text(product_id.as_str())],
    )?
    .iter()
    .map(|row| decode_milestone(row))
    .collect()
}

/// 乐观并发：`expected` 必须等于当前状态；写入终态时若该枚是游标，游标推进到
/// 按 `seq` 的第一枚非终态（没有则置 NULL）。
pub fn record_milestone_state(
    tx: &Tx<'_>,
    id: &MilestoneId,
    expected: MilestoneState,
    to: MilestoneState,
    occurred_at: i64,
) -> Result<(), ProductError> {
    let Some(milestone) = load_milestone(tx, id)? else {
        return Err(ProductError::UnknownMilestone {
            id: id.as_str().to_owned(),
        });
    };
    if milestone.state() != expected {
        return Err(ProductError::MilestoneStateMismatch {
            id: id.as_str().to_owned(),
            expected: expected.as_str().to_owned(),
            actual: milestone.state().as_str().to_owned(),
        });
    }

    tx.execute(
        "UPDATE milestone SET state = ?2, updated_at = ?3 WHERE id = ?1",
        &[
            Value::text(id.as_str()),
            Value::text(to.as_str()),
            Value::Int(occurred_at),
        ],
    )?;

    if is_terminal(to) {
        advance_cursor_if_current(tx, &milestone.product_id(), id, occurred_at)?;
    }
    Ok(())
}

/// 若 `milestone_id` 是某个 Product 的当前游标，把它推进到按 `seq` 的第一枚
/// 非终态；没有则置 NULL。
fn advance_cursor_if_current(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
    milestone_id: &MilestoneId,
    occurred_at: i64,
) -> Result<(), ProductError> {
    let is_current = tx.query(
        "SELECT 1 FROM product WHERE id = ?1 AND current_milestone = ?2",
        &[
            Value::text(product_id.as_str()),
            Value::text(milestone_id.as_str()),
        ],
    )?;
    if is_current.is_empty() {
        return Ok(());
    }
    let next = tx.query(
        "SELECT id FROM milestone
         WHERE product_id = ?1
           AND state NOT IN ('completed', 'failed', 'superseded')
         ORDER BY seq LIMIT 1",
        &[Value::text(product_id.as_str())],
    )?;
    let next = next
        .first()
        .map(|row| text_at(row, 0, "milestone"))
        .transpose()?;
    tx.execute(
        "UPDATE product SET current_milestone = ?2, updated_at = ?3 WHERE id = ?1",
        &[
            Value::text(product_id.as_str()),
            next.map(Value::text).unwrap_or(Value::Null),
            Value::Int(occurred_at),
        ],
    )?;
    Ok(())
}

/// 写游标。`None` 只把 current 置 NULL，仍要求 Product 行存在。
pub fn set_current_milestone(
    tx: &Tx<'_>,
    product_id: &ProductIdRef,
    milestone: Option<&MilestoneId>,
    occurred_at: i64,
) -> Result<(), ProductError> {
    if load_product(tx, product_id)?.is_none() {
        return Err(ProductError::UnknownProduct {
            id: product_id.as_str().to_owned(),
        });
    }
    if let Some(id) = milestone {
        let Some(found) = load_milestone(tx, id)? else {
            return Err(ProductError::UnknownMilestone {
                id: id.as_str().to_owned(),
            });
        };
        if found.product_id().as_str() != product_id.as_str() {
            return Err(ProductError::MilestoneProductMismatch {
                milestone: id.as_str().to_owned(),
                product: product_id.as_str().to_owned(),
            });
        }
        if is_terminal(found.state()) {
            return Err(ProductError::TerminalMilestone {
                id: id.as_str().to_owned(),
            });
        }
    }
    tx.execute(
        "UPDATE product SET current_milestone = ?2, updated_at = ?3 WHERE id = ?1",
        &[
            Value::text(product_id.as_str()),
            milestone
                .map(|id| Value::text(id.as_str()))
                .unwrap_or(Value::Null),
            Value::Int(occurred_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn is_terminal(state: MilestoneState) -> bool {
    matches!(
        state,
        MilestoneState::Completed | MilestoneState::Failed | MilestoneState::Superseded
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_canonical::Opaque;
    use continuum_persist::{Db, builtin_migrations};

    use crate::baseline::EngineeringBaseline;
    use crate::predicate::{ShippableClause, ShippableClauseId, ShippablePredicate};
    use crate::readiness::{
        ProductReadiness, ReadinessDimension, ReadinessDimensionName, ReadinessValue,
    };
    use continuum_verify::EvidenceId;

    fn baseline() -> EngineeringBaseline {
        EngineeringBaseline {
            product_goal: Opaque::from_str("ship"),
            target_users: vec![Opaque::from_str("devs")],
            required_capabilities: vec![Opaque::from_str("build")],
            hard_constraints: vec![Opaque::from_str("offline")],
            architecture: Opaque::from_str("library"),
            technology_choices: vec![Opaque::from_str("rust")],
            minimum_complete_system: vec![Opaque::from_str("cli")],
            known_unknowns: vec![Opaque::from_str("demand")],
            engineering_risks: vec![Opaque::from_str("scope")],
            acceptance_criteria: vec![Opaque::from_str("tests")],
        }
    }

    fn predicate() -> ShippablePredicate {
        ShippablePredicate::new(
            vec![ReadinessDimensionName::Functional],
            vec![
                ShippableClause::new(
                    ShippableClauseId::from_str("c1"),
                    Opaque::from_str("works"),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn setup() -> (tempfile::TempDir, Db, Product) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("product.db");
        let mut migrations = builtin_migrations();
        migrations.extend(p7a_product_migrations());
        migrations.extend(continuum_semantics::p4_semantics_migrations());
        let db = Db::open_with(&path, migrations).unwrap();
        db.migrate().unwrap();

        let product = Product::from_verified_parts(
            ProductIdRef::from_str("p"),
            continuum_semantics::IntentId::from_str("intent/p"),
            baseline(),
            ProductReadiness::unassessed(),
            None,
            vec![],
            predicate(),
        )
        .unwrap();
        let tx = db.begin().unwrap();
        tx.execute(
            "INSERT INTO product
               (id, intent_id, baseline, readiness, current_milestone,
                shippable_predicate, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, 1, 1)",
            &[
                Value::text(product.id().as_str()),
                Value::text(product.intent_id().as_str()),
                Value::text(encode_baseline(product.baseline())),
                Value::text(encode_readiness(product.readiness())),
                Value::text(encode_predicate(product.shippable_predicate())),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
        (dir, db, product)
    }

    fn forged(
        id: &str,
        objective: &str,
        entry: Vec<&str>,
        exit: Vec<&str>,
    ) -> Milestone {
        Milestone::for_test(
            MilestoneId::from_str(id),
            ProductIdRef::from_str("p"),
            Opaque::from_str(objective),
            vec![],
            entry.into_iter().map(Opaque::from_str).collect(),
            exit.into_iter().map(Opaque::from_str).collect(),
            MilestoneState::Proposed,
        )
    }

    fn count(tx: &Tx<'_>) -> i64 {
        match tx
            .query("SELECT COUNT(*) FROM milestone", &[])
            .unwrap()[0][0]
        {
            Value::Int(n) => n,
            ref other => panic!("COUNT 不是整数: {other:?}"),
        }
    }

    #[test]
    fn save_milestones_rejects_a_forged_empty_milestone() {
        let (_dir, db, product) = setup();
        let tx = db.begin().unwrap();
        let cases = [
            (
                forged("milestone/p/0", "", vec![], vec![]),
                "objective",
            ),
            (
                forged("milestone/p/0", "m", vec![""], vec![]),
                "entry",
            ),
            (
                forged("milestone/p/0", "m", vec![], vec![""]),
                "exit",
            ),
        ];
        for (milestone, label) in cases {
            let err = save_milestones(&tx, &product, &[milestone], 1).unwrap_err();
            match label {
                "objective" => assert!(matches!(
                    err,
                    ProductError::EmptyMilestoneObjective
                )),
                "entry" => assert!(matches!(
                    err,
                    ProductError::EmptyMilestoneCondition {
                        field: "entry_conditions",
                        ..
                    }
                )),
                _ => assert!(matches!(
                    err,
                    ProductError::EmptyMilestoneCondition {
                        field: "exit_conditions",
                        ..
                    }
                )),
            }
            assert_eq!(count(&tx), 0, "坏 {label} 写入了行");
        }
    }

    #[test]
    fn save_milestones_rejects_a_forged_ordinal() {
        let (_dir, db, product) = setup();
        let tx = db.begin().unwrap();
        let milestone = forged("milestone/p/99", "m", vec![], vec![]);
        let err = save_milestones(&tx, &product, &[milestone], 1).unwrap_err();
        assert!(matches!(
            err,
            ProductError::MilestoneOrdinalMismatch {
                expected: 0,
                actual: 99,
                ..
            }
        ));
        assert_eq!(count(&tx), 0);
    }

    #[test]
    fn record_milestone_progress_rejects_assessed_without_evidence() {
        let (_dir, db, product) = setup();
        let tx = db.begin().unwrap();
        let milestone = forged("milestone/p/0", "m", vec![], vec![]);
        save_milestones(&tx, &product, &[milestone], 1).unwrap();
        record_milestone_state(
            &tx,
            &MilestoneId::from_str("milestone/p/0"),
            MilestoneState::Proposed,
            MilestoneState::Completed,
            2,
        )
        .unwrap();
        let milestone = load_milestone(&tx, &MilestoneId::from_str("milestone/p/0"))
            .unwrap()
            .unwrap();
        let current = ProductReadiness::unassessed().with_dimension(
            ReadinessDimensionName::Documentation,
            ReadinessDimension::for_test(
                ReadinessValue::Assessed(Opaque::from_str("0.8")),
                vec![],
                Some(100),
            ),
        );
        let err = record_milestone_progress(
            &tx,
            &product,
            &milestone,
            current,
            &[],
            None,
            3,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ProductError::ReadinessValueWithoutEvidence {
                dimension: "documentation"
            }
        ));
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM product_progress", &[]).unwrap()[0][0],
            Value::Int(0)
        );
        let stored = load_product(&tx, product.id()).unwrap().unwrap();
        assert!(
            stored
                .readiness()
                .iter()
                .into_iter()
                .all(|(_, dimension)| dimension.value().is_unassessed())
        );
    }

    #[test]
    fn save_milestones_rejects_duplicate_ids_within_a_batch() {
        let (_dir, db, product) = setup();
        let tx = db.begin().unwrap();
        let batch = [
            forged("milestone/p/0", "m0", vec![], vec![]),
            forged("milestone/p/0", "m1", vec![], vec![]),
        ];
        let err = save_milestones(&tx, &product, &batch, 1).unwrap_err();
        assert!(matches!(
            err,
            ProductError::MilestoneIdCollision { id } if id == "milestone/p/0"
        ));
        assert_eq!(count(&tx), 0);
    }
}
