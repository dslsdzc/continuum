use continuum_artifact::{
    Artifact, ArtifactError, ArtifactId, ArtifactStore, ArtifactType, ContentHash, PrivacyClass,
};
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
