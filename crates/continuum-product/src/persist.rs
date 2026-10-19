//! 迁移 200 与产品循环的落库／读回。
//!
//! C2=(b) 已批准：`product_drive` 是第四张表。本模块是内部 JSON 编码的唯一产生点；
//! 编码不是公开序列化契约，未知／缺失键与表外枚举值一律 `CorruptRow`，不取默认值。
//!
//! 所有写 API **不自己 `commit`**：`Err` 之后由调用方回滚。

use continuum_canonical::Opaque;
use continuum_persist::{Migration, Tx, Value};
use continuum_semantics::{
    IntentError, IntentKind, IntentStatus, ProductIdRef, load_intent,
};
use serde_json::{Map, Value as JsonValue, json};

use crate::baseline::EngineeringBaseline;
use crate::drive::{ContinuousProductDrive, ProductDriveState, ProductPhase};
use crate::error::ProductError;
use crate::milestone::{MilestoneId, MilestoneState};
use crate::predicate::{ShippableClause, ShippableClauseId, ShippablePredicate};
use crate::product::Product;
use crate::readiness::{
    ProductReadiness, ReadinessDimension, ReadinessDimensionName, ReadinessValue,
};

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

pub(crate) fn is_terminal(state: MilestoneState) -> bool {
    matches!(
        state,
        MilestoneState::Completed | MilestoneState::Failed | MilestoneState::Superseded
    )
}
