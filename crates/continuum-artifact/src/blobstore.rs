//! 内容寻址的二进制存储（设计 §15、§242）。
//!
//! 内容不存库：按 `content_hash` 落磁盘，路径为 `<root>/<哈希前两位>/<哈希>`。
//! 该布局使得相同内容必然落在同一路径，磁盘上的去重由路径本身保证，
//! 不需要额外的索引或引用计数。

use crate::content::ContentHash;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("内容 {hash} 的读写失败：{source}")]
    Io {
        hash: ContentHash,
        #[source]
        source: std::io::Error,
    },
    #[error("内容 {hash} 的字节与其哈希不符（存储已损坏或被篡改）")]
    Corrupt { hash: ContentHash },
    #[error("内容 {hash} 不存在")]
    Missing { hash: ContentHash },
}

pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 内容的落盘路径。用哈希前两位分目录，避免单目录下文件过多。
    fn path_of(&self, hash: &ContentHash) -> PathBuf {
        let s = hash.as_str();
        self.root.join(&s[..2]).join(s)
    }

    /// 写入内容并返回其哈希。相同内容重复写入不产生第二份。
    pub fn put(&self, bytes: &[u8]) -> Result<ContentHash, BlobError> {
        let hash = ContentHash::of(bytes);
        let path = self.path_of(&hash);
        if path.exists() {
            return Ok(hash);
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|source| BlobError::Io {
                hash: hash.clone(),
                source,
            })?;
        }
        fs::write(&path, bytes).map_err(|source| BlobError::Io {
            hash: hash.clone(),
            source,
        })?;
        Ok(hash)
    }

    /// 读回内容并校验哈希。校验失败说明存储被改动，返回 `Corrupt` 而不是坏数据。
    pub fn get(&self, hash: &ContentHash) -> Result<Vec<u8>, BlobError> {
        let path = self.path_of(hash);
        if !path.exists() {
            return Err(BlobError::Missing { hash: hash.clone() });
        }
        let bytes = fs::read(&path).map_err(|source| BlobError::Io {
            hash: hash.clone(),
            source,
        })?;
        if &ContentHash::of(&bytes) != hash {
            return Err(BlobError::Corrupt { hash: hash.clone() });
        }
        Ok(bytes)
    }

    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.path_of(hash).exists()
    }
}
