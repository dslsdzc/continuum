use continuum_artifact::{
    Artifact, ArtifactError, ArtifactId, ArtifactStore, ArtifactType, BlobStore, ContentHash,
    PrivacyClass, p1_artifact_migrations,
};
use continuum_persist::Value;
use serde_json::json;

fn artifact(id: &str, bytes: &[u8], inputs: Vec<ArtifactId>) -> Artifact {
    Artifact {
        id: ArtifactId::new(id),
        artifact_type: ArtifactType::Text,
        content_hash: ContentHash::of(bytes),
        size: bytes.len() as u64,
        producer_node: Some("n1".to_owned()),
        input_artifacts: inputs,
        metadata: json!({}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Personal,
        version: 1,
    }
}

#[test]
fn identical_content_produces_identical_hash() {
    assert_eq!(ContentHash::of(b"hello"), ContentHash::of(b"hello"));
    assert_ne!(ContentHash::of(b"hello"), ContentHash::of(b"world"));
}

#[test]
fn hash_is_lowercase_hex_sha256() {
    // 已知向量：b"abc" 的 SHA-256
    assert_eq!(
        ContentHash::of(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn commit_persists_and_is_readable() {
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"one", vec![])).unwrap();

    let got = store.get(&ArtifactId::new("a1")).expect("应能读回");
    assert_eq!(got.artifact_type, ArtifactType::Text);
    assert_eq!(got.size, 3);
}

#[test]
fn recommitting_the_same_id_is_rejected() {
    // §241：已提交的 Artifact 不可修改，修改必须产生新版本
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"one", vec![])).unwrap();

    match store.commit(artifact("a1", b"two", vec![])) {
        Err(ArtifactError::AlreadyCommitted { id }) => assert_eq!(id.as_str(), "a1"),
        other => panic!("同 id 重复提交必须被拒绝，实际 {other:?}"),
    }
    // 原内容未被覆盖
    assert_eq!(
        store.get(&ArtifactId::new("a1")).unwrap().content_hash,
        ContentHash::of(b"one")
    );
}

#[test]
fn same_content_under_two_ids_keeps_one_storage_entry() {
    // ENG-003：内容寻址，相同内容只存一份
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"same", vec![])).unwrap();
    store.commit(artifact("a2", b"same", vec![])).unwrap();

    assert_eq!(store.stored_blob_count(), 1, "相同内容不得存两份");
    assert_eq!(
        store.find_by_hash(&ContentHash::of(b"same")).map(|a| a.id.as_str()),
        // 先提交者胜出，顺序稳定
        Some("a1")
    );
}

#[test]
fn lineage_is_traceable_back_to_the_roots() {
    // §242：任意结果可反向追溯到原始输入
    let mut store = ArtifactStore::new();
    store.commit(artifact("root", b"r", vec![])).unwrap();
    store
        .commit(artifact("mid", b"m", vec![ArtifactId::new("root")]))
        .unwrap();
    store
        .commit(artifact("leaf", b"l", vec![ArtifactId::new("mid")]))
        .unwrap();

    let mut traced = store.lineage(&ArtifactId::new("leaf"));
    traced.sort();
    assert_eq!(traced, vec![ArtifactId::new("mid"), ArtifactId::new("root")]);
}

#[test]
fn committing_with_an_unknown_input_is_rejected() {
    let mut store = ArtifactStore::new();
    match store.commit(artifact("a1", b"x", vec![ArtifactId::new("nope")])) {
        Err(ArtifactError::UnresolvedInput { id }) => assert_eq!(id.as_str(), "nope"),
        other => panic!("输入 Artifact 不存在时必须拒绝，实际 {other:?}"),
    }
}

#[test]
fn store_round_trips_through_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"root", vec![])).unwrap();
    store.commit(artifact("a2", b"derived", vec![ArtifactId::new("a1")])).unwrap();
    store.persist(&tx, 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    assert_eq!(restored.get(&ArtifactId::new("a1")), store.get(&ArtifactId::new("a1")));
    assert_eq!(restored.get(&ArtifactId::new("a2")), store.get(&ArtifactId::new("a2")));
    assert_eq!(restored.lineage(&ArtifactId::new("a2")), vec![ArtifactId::new("a1")]);
    assert_eq!(restored.stored_blob_count(), 2);
}

#[test]
fn deduplication_holds_on_the_persisted_path() {
    // 两个 id 不同的 Artifact 携带相同内容：落库后仍只算一份内容
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    let a1 = artifact("a1", b"same", vec![]);
    let a2 = artifact("a2", b"same", vec![]); // 与 a1 内容相同、id 不同
    let hash = a1.content_hash.clone();
    assert_eq!(hash, a2.content_hash, "夹具前提：两者内容相同");
    store.commit(a1).unwrap();
    store.commit(a2).unwrap();
    store.persist(&tx, 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    assert_eq!(restored.stored_blob_count(), 1, "相同内容只存一份");
    assert!(restored.find_by_hash(&hash).is_some());
}

#[test]
fn restore_follows_dependency_order_not_id_order() {
    // "a1" 的输入是 "z1"：按 id 字典序重建会先撞上输入缺失，按依赖序不会
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    store.commit(artifact("z1", b"root", vec![])).unwrap();
    store.commit(artifact("a1", b"derived", vec![ArtifactId::new("z1")])).unwrap();
    store.persist(&tx, 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).expect("按依赖序重建应当成功");
    assert_eq!(restored.lineage(&ArtifactId::new("a1")), vec![ArtifactId::new("z1")]);
}

#[test]
fn restore_reports_dangling_inputs_instead_of_dropping_them() {
    // 库中存在一个输入不存在的 Artifact：重建必须报错，不能静默少一个。
    //
    // 命中的是 `restore` 循环末尾那条分支（一轮之内无进展 → 点名 deferred
    // 集合）；`store.commit(...)` 的 map_err 分支在此不可达，见其上方注释。
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"root", vec![])).unwrap();
    store.persist(&tx, 1_000).unwrap();
    // 绕过内存路径直接写一条输入悬空的记录
    tx.execute(
        "INSERT INTO artifact_input (artifact_id, input_artifact_id) VALUES ('a1', 'ghost')",
        &[],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = ArtifactStore::restore(&tx).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("a1"), "错误应点名出问题的 Artifact，实际：{msg}");
}

#[test]
fn commit_with_content_puts_the_bytes_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    let a = artifact("a1", b"payload", vec![]);
    let hash = a.content_hash.clone();
    store.commit_with_content(&blob, a, b"payload").unwrap();

    assert!(blob.contains(&hash), "字节必须已按哈希落盘");
    assert_eq!(blob.get(&hash).unwrap(), b"payload");
    assert_eq!(store.stored_blob_count(), 1);
}

#[test]
fn commit_with_content_rejects_bytes_that_do_not_match_the_hash() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    // artifact 声明的哈希由 b"declared" 算出，实际传入 b"tampered"。
    // 两者必须**等长**（均为 8 字节）：否则 size 校验会先于哈希校验命中，
    // 本用例就变成在测 SizeMismatch，而钉不住哈希校验这一条。
    let a = artifact("a1", b"declared", vec![]);
    let hash = a.content_hash.clone();
    let err = store.commit_with_content(&blob, a, b"tampered").unwrap_err();

    assert!(
        matches!(err, ArtifactError::ContentMismatch { .. }),
        "实际 {err:?}"
    );
    assert!(!blob.contains(&hash), "被拒的提交不得留下字节");
    // 上面那条钉的是「声明的哈希」不在盘上；若哈希校验被挪到 blob.put 之后，
    // 落盘的是 b"tampered" 的内容（挂在另一个哈希下），contains(&hash) 仍为假、
    // 断言照样通过。所以必须直接数文件：拒绝的提交不得留下任何字节。
    assert_eq!(count_files(dir.path()), 0, "被拒的提交不得留下任何字节");
    assert!(
        store.get(&ArtifactId::new("a1")).is_none(),
        "被拒的提交不得登记元数据"
    );
}

#[test]
fn commit_with_content_rejects_a_size_that_does_not_match_the_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    // 哈希由 b"payload" 算出（相符），size 声明为 999（不符）
    let mut a = artifact("a1", b"payload", vec![]);
    a.size = 999;
    let err = store.commit_with_content(&blob, a, b"payload").unwrap_err();

    assert!(
        matches!(
            err,
            ArtifactError::SizeMismatch {
                declared: 999,
                actual: 7,
                ..
            }
        ),
        "实际 {err:?}"
    );
    assert_eq!(count_files(dir.path()), 0, "被拒的提交不得留下任何字节");
    assert!(
        store.get(&ArtifactId::new("a1")).is_none(),
        "被拒的提交不得登记元数据"
    );
}

#[test]
fn identical_content_under_two_ids_shares_one_blob_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    store
        .commit_with_content(&blob, artifact("a1", b"same", vec![]), b"same")
        .unwrap();
    store
        .commit_with_content(&blob, artifact("a2", b"same", vec![]), b"same")
        .unwrap();

    assert_eq!(store.stored_blob_count(), 1, "相同内容只登记一份");
    // 磁盘上也只应有一份：BlobStore 按哈希寻址，相同内容落在同一路径
    assert_eq!(count_files(dir.path()), 1, "相同内容在盘上只占一份");
}

#[test]
fn content_survives_commit_persist_restore_and_read_back() {
    // 设计 §15 的完整回路：字节落盘、元数据落库、从库重建后仍能按哈希取回字节
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path().join("blobs"));
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();

    let tx = db.begin().unwrap();
    let mut store = ArtifactStore::new();
    store
        .commit_with_content(&blob, artifact("z1", b"root", vec![]), b"root")
        .unwrap();
    store
        .commit_with_content(
            &blob,
            artifact("a1", b"derived", vec![ArtifactId::new("z1")]),
            b"derived",
        )
        .unwrap();
    store.persist(&tx, 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    let hash = restored
        .get(&ArtifactId::new("z1"))
        .unwrap()
        .content_hash
        .clone();
    assert_eq!(blob.get(&hash).unwrap(), b"root", "重建后仍能按哈希取回原始字节");
    assert_eq!(
        restored.lineage(&ArtifactId::new("a1")),
        vec![ArtifactId::new("z1")]
    );
}

#[test]
fn persist_writes_artifact_created_with_the_time_it_was_given() {
    // 唯一一条走**真实生产路径**（`ArtifactStore::persist`）覆盖 `ArtifactCreated`
    // 的用例。`save_artifact` 由 graph 侧的用例直接调，走不到 persist 里的事件 id
    // 派生与时间戳传递。
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();

    let tx = db.begin().unwrap();
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"one", vec![])).unwrap();
    store.commit(artifact("a2", b"two", vec![])).unwrap();
    store.persist(&tx, 1_724_000_000_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT event_id, event_type, occurred_at FROM events ORDER BY event_id",
            &[],
        )
        .unwrap();
    tx.commit().unwrap();

    let got: Vec<(String, String, i64)> = rows
        .iter()
        .map(|r| match (&r[0], &r[1], &r[2]) {
            (Value::Text(id), Value::Text(t), Value::Int(at)) => (id.clone(), t.clone(), *at),
            other => panic!("events 行的列类型不符：{other:?}"),
        })
        .collect();

    assert_eq!(
        got,
        vec![
            (
                "artifact/a1".to_owned(),
                "artifact.created".to_owned(),
                1_724_000_000_000
            ),
            (
                "artifact/a2".to_owned(),
                "artifact.created".to_owned(),
                1_724_000_000_000
            ),
        ],
        "每个 Artifact 一条事件，id 由 artifact id 派生，时刻为调用方传入值"
    );
}

fn count_files(root: &std::path::Path) -> usize {
    walk(root).len()
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

fn migrations() -> Vec<continuum_persist::Migration> {
    let mut m = continuum_persist::builtin_migrations();
    m.extend(p1_artifact_migrations());
    m
}
