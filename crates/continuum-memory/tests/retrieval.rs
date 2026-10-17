//! Task 7：确定性检索面（设计 §6.1／§6.2）。
//!
//! 照片：R20（终态／scope／anchor／顺序）、R21（两个边界）、R25（trait 与自由
//! 函数同一实现）、R27（构造侧 anchor 规则）。夹具直接 SQL 写 claim，不经
//! `apply_synthesis`（Task 8 才有）。

use continuum_memory::{
    MemoryClaim, MemoryError, MemoryRetrieval, MemoryScope, MemoryStore, RetrievalQuery,
    Validity, retrieve_claims,
};

mod common;

use common::{new_db, seed_claim};

fn query(
    scopes: Vec<MemoryScope>,
    anchor: Option<&str>,
    text: Option<&str>,
    limit: usize,
    include_terminal: bool,
) -> RetrievalQuery {
    RetrievalQuery::new(
        scopes,
        anchor.map(str::to_string),
        text.map(str::to_string),
        limit,
        include_terminal,
    )
    .unwrap()
}

fn ids(claims: &[MemoryClaim]) -> Vec<&str> {
    claims.iter().map(|c| c.id().as_str()).collect()
}

#[test]
fn query_construction_applies_the_anchor_rules() {
    assert_eq!(
        RetrievalQuery::new(vec![MemoryScope::Project], None, None, 10, false),
        Err(MemoryError::AnchorRequired)
    );
    assert_eq!(
        RetrievalQuery::new(
            vec![MemoryScope::Personal],
            Some("p".to_string()),
            None,
            10,
            false
        ),
        Err(MemoryError::AnchorNotAllowed)
    );
    assert_eq!(
        RetrievalQuery::new(
            vec![MemoryScope::Personal, MemoryScope::Project],
            Some("p".to_string()),
            None,
            10,
            false
        ),
        Err(MemoryError::AnchorNotAllowed)
    );
    assert_eq!(
        RetrievalQuery::new(
            vec![MemoryScope::Project, MemoryScope::Product],
            Some("p".to_string()),
            None,
            10,
            false
        ),
        Err(MemoryError::AnchorNotAllowed)
    );
    assert!(
        RetrievalQuery::new(
            vec![MemoryScope::Project],
            Some("p".to_string()),
            None,
            10,
            false
        )
        .is_ok()
    );
    assert!(RetrievalQuery::new(vec![MemoryScope::Personal], None, None, 10, false).is_ok());
}

#[test]
fn query_dedupes_scopes_without_error() {
    let q = query(
        vec![MemoryScope::Project, MemoryScope::Project],
        Some("p"),
        None,
        10,
        false,
    );
    assert_eq!(q.scopes(), [MemoryScope::Project]);

    let q = query(
        vec![MemoryScope::Personal, MemoryScope::Personal],
        None,
        None,
        10,
        false,
    );
    assert_eq!(q.scopes(), [MemoryScope::Personal]);
}

#[test]
fn query_normalizes_empty_text_to_none() {
    let q = RetrievalQuery::new(
        vec![MemoryScope::Personal],
        None,
        Some(String::new()),
        10,
        false,
    )
    .unwrap();
    assert_eq!(q.text(), None);

    let q = RetrievalQuery::new(
        vec![MemoryScope::Personal],
        None,
        Some("  ".to_string()),
        10,
        false,
    )
    .unwrap();
    assert_eq!(q.text(), Some("  "), "只归一 Some(\"\")");
}

#[test]
fn limit_zero_returns_empty_and_empty_scopes_is_an_error() {
    assert_eq!(
        RetrievalQuery::new(vec![], None, None, 10, false),
        Err(MemoryError::EmptyScopeSet)
    );

    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_claim(
        &tx,
        "c1",
        MemoryScope::Personal,
        "用户喜欢中文",
        Validity::Active,
        10,
        None,
        &["s1"],
    );
    let q = query(vec![MemoryScope::Personal], None, None, 0, false);
    assert_eq!(retrieve_claims(&tx, &q), Ok(vec![]));
}

#[test]
fn retrieve_returns_only_active_claims_by_default() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for (id, validity) in [
        ("c-active", Validity::Active),
        ("c-superseded", Validity::Superseded),
        ("c-invalidated", Validity::Invalidated),
    ] {
        seed_claim(
            &tx,
            id,
            MemoryScope::Personal,
            id,
            validity,
            10,
            None,
            &["s1"],
        );
    }
    let default = query(vec![MemoryScope::Personal], None, None, 10, false);
    assert_eq!(ids(&retrieve_claims(&tx, &default).unwrap()), ["c-active"]);

    let all = query(vec![MemoryScope::Personal], None, None, 10, true);
    assert_eq!(
        ids(&retrieve_claims(&tx, &all).unwrap()),
        ["c-active", "c-invalidated", "c-superseded"]
    );
}

#[test]
fn scope_and_anchor_filter() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_claim(
        &tx,
        "p1c",
        MemoryScope::Project,
        "p1",
        Validity::Active,
        10,
        Some("p1"),
        &["s1"],
    );
    seed_claim(
        &tx,
        "p2c",
        MemoryScope::Project,
        "p2",
        Validity::Active,
        10,
        Some("p2"),
        &["s2"],
    );
    seed_claim(
        &tx,
        "personal",
        MemoryScope::Personal,
        "me",
        Validity::Active,
        10,
        None,
        &["s3"],
    );

    let q = query(vec![MemoryScope::Project], Some("p1"), None, 10, false);
    assert_eq!(ids(&retrieve_claims(&tx, &q).unwrap()), ["p1c"]);

    let q = query(vec![MemoryScope::Personal], None, None, 10, false);
    assert_eq!(ids(&retrieve_claims(&tx, &q).unwrap()), ["personal"]);
}

#[test]
fn text_filter_is_case_sensitive_substring() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_claim(
        &tx,
        "c1",
        MemoryScope::Personal,
        "Core 不再使用 move",
        Validity::Active,
        10,
        None,
        &["s1"],
    );

    for (text, expected) in [
        ("move", true),
        ("MOVE", false),
        ("不再使用", true),
        ("core", false),
    ] {
        let q = query(vec![MemoryScope::Personal], None, Some(text), 10, false);
        let got = retrieve_claims(&tx, &q).unwrap();
        assert_eq!(!got.is_empty(), expected, "子串 {text:?} 的命中不符");
    }
}

#[test]
fn order_is_last_confirmed_desc_then_id_asc() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for (id, last_confirmed) in [("b", 100), ("a", 200), ("c", 200)] {
        seed_claim(
            &tx,
            id,
            MemoryScope::Personal,
            id,
            Validity::Active,
            last_confirmed,
            None,
            &["s1"],
        );
    }
    let q = query(vec![MemoryScope::Personal], None, None, 10, false);
    assert_eq!(ids(&retrieve_claims(&tx, &q).unwrap()), ["a", "c", "b"]);
}

#[test]
fn limit_caps_without_panicking() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for (id, last_confirmed) in [("a", 30), ("b", 20), ("c", 10)] {
        seed_claim(
            &tx,
            id,
            MemoryScope::Personal,
            id,
            Validity::Active,
            last_confirmed,
            None,
            &["s1"],
        );
    }
    let q = query(vec![MemoryScope::Personal], None, None, 2, false);
    assert_eq!(ids(&retrieve_claims(&tx, &q).unwrap()), ["a", "b"]);
    let q = query(vec![MemoryScope::Personal], None, None, 100, false);
    assert_eq!(ids(&retrieve_claims(&tx, &q).unwrap()), ["a", "b", "c"]);
}

#[test]
fn memory_store_and_free_function_agree() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_claim(
        &tx,
        "c1",
        MemoryScope::Personal,
        "用户喜欢中文",
        Validity::Active,
        10,
        None,
        &["s1"],
    );
    let q = query(vec![MemoryScope::Personal], None, None, 10, false);
    let store = MemoryStore::new(&tx);
    let via_trait = (&store as &dyn MemoryRetrieval).retrieve(&q).unwrap();
    let via_free = retrieve_claims(&tx, &q).unwrap();
    assert_eq!(via_trait, via_free);
    assert_eq!(via_trait.len(), 1);
}
