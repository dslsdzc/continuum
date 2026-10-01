use continuum_artifact::{BlobError, BlobStore, ContentHash};

#[test]
fn same_content_is_stored_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());

    let a = store.put(b"hello").unwrap();
    let b = store.put(b"hello").unwrap();

    assert_eq!(a, b, "相同内容必须得到相同哈希");
    assert_eq!(a, ContentHash::of(b"hello"));
    // 磁盘上只有一份：目录下的文件总数不随重复写入增长
    assert_eq!(count_files(dir.path()), 1);
}

#[test]
fn different_content_lands_on_different_paths() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    store.put(b"one").unwrap();
    store.put(b"two").unwrap();
    assert_eq!(count_files(dir.path()), 2);
}

#[test]
fn content_round_trips_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let hash = store.put(b"\x00\xff binary").unwrap();
    assert_eq!(store.get(&hash).unwrap(), b"\x00\xff binary");
}

#[test]
fn tampered_content_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let hash = store.put(b"original").unwrap();

    // 找到落盘文件并改写它，模拟存储被篡改
    let path = find_file(dir.path());
    std::fs::write(&path, b"tampered").unwrap();

    let err = store.get(&hash).unwrap_err();
    assert!(matches!(err, BlobError::Corrupt { .. }), "实际 {err:?}");
}

#[test]
fn truncated_content_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let hash = store.put(b"original payload").unwrap();

    // 模拟写盘中途崩溃留下的截断文件：内容是原文前缀，长度不符
    let path = find_file(dir.path());
    std::fs::write(&path, b"orig").unwrap();

    let err = store.get(&hash).unwrap_err();
    assert!(matches!(err, BlobError::Corrupt { .. }), "实际 {err:?}");
}

#[test]
fn missing_content_reports_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let err = store.get(&ContentHash::of(b"never written")).unwrap_err();
    assert!(matches!(err, BlobError::Missing { .. }), "实际 {err:?}");
}

#[test]
fn malformed_hash_is_rejected_instead_of_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());

    // ContentHash 派生 Deserialize，可绕过 ContentHash::parse 的 64 位十六进制校验。
    // 空串会让 &s[..2] 越界，非 ASCII 会切在字符边界上，含 .. 的则会让路径逃出 root。
    for raw in ["\"\"", "\"日本語\"", "\"../../etc/passwd\"", "\"ZZZZ\""] {
        let hash: ContentHash = serde_json::from_str(raw).unwrap();
        let err = store.get(&hash).unwrap_err();
        assert!(
            matches!(err, BlobError::MalformedHash { .. }),
            "哈希 {raw} 应被拒，实际 {err:?}"
        );
        assert!(!store.contains(&hash), "非法哈希不可能是已落盘的内容");
    }
}

fn count_files(root: &std::path::Path) -> usize {
    walk(root).len()
}

fn find_file(root: &std::path::Path) -> std::path::PathBuf {
    walk(root).into_iter().next().expect("至少有一个文件")
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
