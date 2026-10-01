use continuum_artifact::{BlobStore, ContentHash};

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
    assert!(matches!(err, continuum_artifact::BlobError::Corrupt { .. }), "实际 {err:?}");
}

#[test]
fn missing_content_reports_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let err = store.get(&ContentHash::of(b"never written")).unwrap_err();
    assert!(matches!(err, continuum_artifact::BlobError::Missing { .. }), "实际 {err:?}");
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
