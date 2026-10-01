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
    #[error("哈希 {hash} 不是 64 位小写十六进制，无法作为落盘路径")]
    MalformedHash { hash: ContentHash },
}

pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 内容的落盘路径。用哈希前两位分目录，避免单目录下文件过多。
    ///
    /// 合法性判据复用 [`ContentHash::parse`]（64 位小写十六进制），不在此另写一份：
    /// 本 crate 曾因同一约定分散两处而静默失配。该校验是必要的——`ContentHash`
    /// 派生 `Deserialize`，可绕过 `parse` 造出空串、非 ASCII 或含 `/` 的值，
    /// 前者会让 `&s[..2]` 越界，后者会切在非字符边界上，而含 `..` 的值更会
    /// 让落盘路径逃出 `root`。校验通过后 `s` 必为 64 字节 ASCII，前两位安全。
    fn path_of(&self, hash: &ContentHash) -> Result<PathBuf, BlobError> {
        let s = hash.as_str();
        if ContentHash::parse(s).is_none() {
            return Err(BlobError::MalformedHash { hash: hash.clone() });
        }
        Ok(self.root.join(&s[..2]).join(s))
    }

    /// 临时文件名后缀：进程 id + 纳秒时间 + 进程内自增序号，足以避免同目录内碰撞。
    fn temp_suffix() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{}-{}-{}", std::process::id(), nanos, seq)
    }

    /// 写入内容并返回其哈希。相同内容重复写入不产生第二份。
    ///
    /// 写盘经「临时文件 + `fs::rename`」两步：`fs::write` 不是原子的，中途崩溃
    /// 会留下截断文件，而上面的 exists 短路会让后续同内容的 `put` 直接成功、
    /// `get` 却报 `Corrupt`，且无自愈路径。同目录内的 `rename` 是原子的，
    /// 本代码因此不再产生截断文件。已有文件被外部手段损坏时仍由 `get` 报
    /// `Corrupt`——那是诚实行为，不自愈。
    pub fn put(&self, bytes: &[u8]) -> Result<ContentHash, BlobError> {
        let hash = ContentHash::of(bytes);
        let path = self.path_of(&hash)?;
        if path.exists() {
            return Ok(hash);
        }
        let dir = path.parent().ok_or_else(|| BlobError::Io {
            hash: hash.clone(),
            source: std::io::Error::other("落盘路径没有父目录"),
        })?;
        fs::create_dir_all(dir).map_err(|source| BlobError::Io {
            hash: hash.clone(),
            source,
        })?;

        let tmp = dir.join(format!(".tmp-{}", Self::temp_suffix()));
        let write = fs::write(&tmp, bytes).and_then(|()| fs::rename(&tmp, &path));
        if let Err(source) = write {
            // 失败路径上清掉半成品，避免把临时文件当成已落盘的内容
            let _ = fs::remove_file(&tmp);
            return Err(BlobError::Io {
                hash: hash.clone(),
                source,
            });
        }
        Ok(hash)
    }

    /// 读回内容并校验哈希。校验失败说明存储被改动，返回 `Corrupt` 而不是坏数据。
    pub fn get(&self, hash: &ContentHash) -> Result<Vec<u8>, BlobError> {
        let path = self.path_of(hash)?;
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

    /// 该哈希是否已有内容落盘。
    ///
    /// 保持返回 `bool` 而非 `Result`：非法哈希（见 `path_of` 的合法性判据）按定义
    /// 不可能有内容落盘——`put` 的哈希由字节算出，必合法——故 `false` 是**真话**
    /// 而不是掩盖错误。改成 `Result` 会让每个调用方为一件答案已知的事处理错误。
    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.path_of(hash).is_ok_and(|p| p.exists())
    }
}
