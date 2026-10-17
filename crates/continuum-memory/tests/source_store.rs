//! Task 3：Source 的插入与读回（设计 §4.2／§4.4）。
//!
//! 照片：R2（往返）、R31（重复 id 不覆盖）。软删除的墓碑读回只钉读侧；
//! 删除路径在 Task 9。

use continuum_memory::{
    MemoryError, MemorySourceId, SourceKind, insert_source, load_source, load_sources,
};
use continuum_persist::Value;

mod common;

use common::{new_db, source};

#[test]
fn a_source_round_trips() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let original = source("s1", SourceKind::Files, "file:///a", 100);
    insert_source(&tx, &original).unwrap();
    let read = load_source(&tx, &MemorySourceId::new("s1")).unwrap();
    assert_eq!(read, Some(original));
    assert_eq!(read.unwrap().removed_at(), None);
}

#[test]
fn a_duplicate_source_id_is_rejected_and_the_original_is_untouched() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let original = source("s1", SourceKind::Files, "file:///a", 100);
    insert_source(&tx, &original).unwrap();

    let second = source("s1", SourceKind::Conversations, "sess-2", 200);
    assert_eq!(
        insert_source(&tx, &second),
        Err(MemoryError::DuplicateSourceId)
    );
    let read = load_source(&tx, &MemorySourceId::new("s1")).unwrap();
    assert_eq!(read, Some(original), "重复插入不得覆盖原行");
}

#[test]
fn load_sources_returns_only_existing_ids_in_id_order_without_duplicates() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for id in ["s3", "s1", "s2"] {
        insert_source(&tx, &source(id, SourceKind::Files, id, 1)).unwrap();
    }
    let requested = [
        MemorySourceId::new("s2"),
        MemorySourceId::new("s1"),
        MemorySourceId::new("s1"),
        MemorySourceId::new("missing"),
    ];
    let read = load_sources(&tx, &requested).unwrap();
    let ids: Vec<&str> = read.iter().map(|s| s.id().as_str()).collect();
    assert_eq!(ids, ["s1", "s2"], "应升序、去重、静默省略缺失 id");
}

#[test]
fn an_unknown_source_reads_as_none() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        load_source(&tx, &MemorySourceId::new("nope")).unwrap(),
        None
    );
}

#[test]
fn removed_at_reads_back_as_a_timestamp() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", SourceKind::Files, "file:///a", 100)).unwrap();
    tx.execute(
        "UPDATE memory_source SET removed_at = ? WHERE source_id = ?",
        &[Value::Int(77), Value::text("s1")],
    )
    .unwrap();
    let read = load_source(&tx, &MemorySourceId::new("s1")).unwrap().unwrap();
    assert_eq!(read.removed_at(), Some(77));
}
