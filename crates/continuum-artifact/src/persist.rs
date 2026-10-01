//! Artifact 元数据的落库（§317）。

use crate::artifact::{Artifact, ArtifactId, ArtifactType, PrivacyClass};
use crate::content::ContentHash;
use continuum_persist::{Migration, PersistError, Tx, Value};

pub fn p1_artifact_migrations() -> Vec<Migration> {
    vec![
        Migration::new(10, "p1_artifact", "CREATE TABLE artifact (
            id TEXT PRIMARY KEY,
            artifact_type TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            producer_node TEXT,
            privacy_class TEXT NOT NULL,
            version INTEGER NOT NULL,
            metadata TEXT NOT NULL,
            provenance TEXT NOT NULL
        );
        CREATE INDEX idx_artifact_hash ON artifact(content_hash);
        CREATE TABLE artifact_input (
            artifact_id TEXT NOT NULL,
            input_artifact_id TEXT NOT NULL,
            PRIMARY KEY (artifact_id, input_artifact_id)
        );"),
    ]
}

/// 写入一个 Artifact 的元数据，并在同一事务内写入 `ArtifactCreated`（§16）。
///
/// 同 id 已存在时返回数据库错误，不覆盖（§241）。
///
/// 「入库」与「写事件」不可分离：本函数没有只写元数据的旁路，二者要么一起
/// 由调用方提交，要么一起回滚。这与 `continuum_graph::apply_transition` 是同
/// 一条规矩——状态/元数据的写入与它对应的事件必须在同一事务内（`§318`）。
///
/// `event_id` 的唯一性由调用方保证（`events.event_id` 是主键）。
pub fn save_artifact(
    tx: &Tx<'_>,
    artifact: &Artifact,
    event_id: &str,
    occurred_at: i64,
) -> Result<(), PersistError> {
    tx.execute(
        "INSERT INTO artifact
           (id, artifact_type, content_hash, size, producer_node, privacy_class,
            version, metadata, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        &[
            Value::text(artifact.id.as_str()),
            Value::text(type_str(artifact.artifact_type)),
            Value::text(artifact.content_hash.as_str()),
            Value::Int(artifact.size as i64),
            match &artifact.producer_node {
                Some(n) => Value::text(n.clone()),
                None => Value::Null,
            },
            Value::text(privacy_str(artifact.privacy_class)),
            Value::Int(i64::from(artifact.version)),
            Value::text(serde_json::to_string(&artifact.metadata).expect("metadata 可序列化")),
            Value::text(serde_json::to_string(&artifact.provenance).expect("provenance 可序列化")),
        ],
    )?;
    for input in &artifact.input_artifacts {
        tx.execute(
            "INSERT INTO artifact_input (artifact_id, input_artifact_id) VALUES (?1, ?2)",
            &[Value::text(artifact.id.as_str()), Value::text(input.as_str())],
        )?;
    }

    // 事件写在元数据之后：元数据写入失败时（如同 id 已存在）直接返回错误，
    // 不产生 ArtifactCreated——否则库里会留下一个指向不存在 Artifact 的事件。
    let event = continuum_events::Event::new(
        continuum_events::EventType::ArtifactCreated,
        event_id.to_owned(),
        occurred_at,
        serde_json::json!({ "artifact_id": artifact.id.as_str() }),
    );
    tx.append_event(&event)?;
    Ok(())
}

pub fn load_artifact(
    tx: &Tx<'_>,
    id: &ArtifactId,
) -> Result<Option<Artifact>, PersistError> {
    let rows = tx.query(
        "SELECT id, artifact_type, content_hash, size, producer_node, privacy_class,
                version, metadata, provenance
         FROM artifact WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };

    let inputs = tx.query(
        "SELECT input_artifact_id FROM artifact_input WHERE artifact_id = ?1
         ORDER BY input_artifact_id",
        &[Value::text(id.as_str())],
    )?;
    let input_artifacts = inputs
        .into_iter()
        .map(|r| match &r[0] {
            Value::Text(s) => Ok(ArtifactId::new(s.clone())),
            other => Err(PersistError::Database(format!(
                "input_artifact_id 应为文本，实际 {other:?}"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(Artifact {
        id: ArtifactId::new(text(&row[0])?),
        artifact_type: parse_type(&text(&row[1])?)?,
        content_hash: ContentHash::parse(&text(&row[2])?)
            .ok_or_else(|| PersistError::Database("content_hash 不是合法的十六进制摘要".to_owned()))?,
        size: int(&row[3])? as u64,
        producer_node: match &row[4] {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        },
        input_artifacts,
        metadata: serde_json::from_str(&text(&row[7])?)
            .map_err(|e| PersistError::Database(e.to_string()))?,
        provenance: serde_json::from_str(&text(&row[8])?)
            .map_err(|e| PersistError::Database(e.to_string()))?,
        privacy_class: parse_privacy(&text(&row[5])?)?,
        version: int(&row[6])? as u32,
    }))
}

fn text(v: &Value) -> Result<String, PersistError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(PersistError::Database(format!("列应为文本，实际 {other:?}"))),
    }
}

fn int(v: &Value) -> Result<i64, PersistError> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(PersistError::Database(format!("列应为整数，实际 {other:?}"))),
    }
}

fn type_str(t: ArtifactType) -> &'static str {
    match t {
        ArtifactType::SourceTree => "source_tree",
        ArtifactType::Patch => "patch",
        ArtifactType::TestResult => "test_result",
        ArtifactType::Text => "text",
        ArtifactType::Json => "json",
        ArtifactType::Blob => "blob",
    }
}

fn parse_type(s: &str) -> Result<ArtifactType, PersistError> {
    Ok(match s {
        "source_tree" => ArtifactType::SourceTree,
        "patch" => ArtifactType::Patch,
        "test_result" => ArtifactType::TestResult,
        "text" => ArtifactType::Text,
        "json" => ArtifactType::Json,
        "blob" => ArtifactType::Blob,
        other => {
            return Err(PersistError::Database(format!(
                "未知 ArtifactType: {other}"
            )))
        }
    })
}

fn privacy_str(p: PrivacyClass) -> &'static str {
    match p {
        PrivacyClass::Public => "public",
        PrivacyClass::Personal => "personal",
        PrivacyClass::Private => "private",
        PrivacyClass::Secret => "secret",
        PrivacyClass::LocalOnly => "local_only",
    }
}

fn parse_privacy(s: &str) -> Result<PrivacyClass, PersistError> {
    Ok(match s {
        "public" => PrivacyClass::Public,
        "personal" => PrivacyClass::Personal,
        "private" => PrivacyClass::Private,
        "secret" => PrivacyClass::Secret,
        "local_only" => PrivacyClass::LocalOnly,
        other => {
            return Err(PersistError::Database(format!(
                "未知 PrivacyClass: {other}"
            )))
        }
    })
}
