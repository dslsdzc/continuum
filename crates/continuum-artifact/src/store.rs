//! Artifact 的提交与寻址（§241、§242）。

use crate::artifact::{Artifact, ArtifactId};
use crate::blobstore::{BlobError, BlobStore};
use crate::content::ContentHash;
use crate::persist::{load_artifact, save_artifact};
use continuum_persist::{PersistError, Tx, Value};
use std::collections::HashMap;

// 不派生 Clone/PartialEq/Eq：`Blob` 变体携带的 `BlobError::Io` 内含
// `std::io::Error`，既不实现 `Clone` 也不实现 `PartialEq`，本枚举因此无法诚实
// 地实现这两者；把 `BlobError` 压成字符串可以绕过，但会丢掉 `#[from]` 建立的
// source 链，而那正是「落盘为何失败」的唯一线索，故不取。
// 全仓无调用方依赖 ArtifactError 的克隆或相等比较（消费方一律模式匹配）。
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("Artifact {id} 已提交，修改必须产生新版本（§241）")]
    AlreadyCommitted { id: ArtifactId },
    #[error("Artifact {id} 不存在")]
    NotFound { id: ArtifactId },
    #[error("输入 Artifact {id} 不存在")]
    UnresolvedInput { id: ArtifactId },
    #[error("Artifact {id} 声明的 content_hash 与其字节不符")]
    ContentMismatch { id: ArtifactId },
    #[error("内容落盘失败：{0}")]
    Blob(#[from] BlobError),
}

/// 内容寻址的 Artifact 集合。相同内容只保留一份。
///
/// 本结构是工作副本，始终保存在内存中；元数据经 [`ArtifactStore::persist`] 落库、
/// 经 [`ArtifactStore::restore`] 重建。二进制的落盘存储见 [`crate::blobstore`]。
#[derive(Debug, Default)]
pub struct ArtifactStore {
    by_id: HashMap<ArtifactId, Artifact>,
    /// content_hash → 首次提交该内容的 artifact_id。
    /// 此映射是「相同内容只存一份」的载体。
    by_hash: HashMap<ContentHash, ArtifactId>,
    /// 内容份数：by_hash 的大小。
    blobs: usize,
}

impl ArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 提交一个 Artifact。
    ///
    /// 同 id 已存在时返回 `AlreadyCommitted`，不覆盖（§241）。
    /// 输入 Artifact 不存在时返回 `UnresolvedInput`。
    pub fn commit(&mut self, artifact: Artifact) -> Result<(), ArtifactError> {
        for input in &artifact.input_artifacts {
            if !self.by_id.contains_key(input) {
                return Err(ArtifactError::UnresolvedInput { id: input.clone() });
            }
        }
        if self.by_id.contains_key(&artifact.id) {
            return Err(ArtifactError::AlreadyCommitted {
                id: artifact.id.clone(),
            });
        }
        match self.by_hash.entry(artifact.content_hash.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {}
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(artifact.id.clone());
                self.blobs += 1;
            }
        }
        self.by_id.insert(artifact.id.clone(), artifact);
        Ok(())
    }

    /// 提交一个 Artifact，并把它的字节按 `content_hash` 落盘。
    ///
    /// 与 [`ArtifactStore::commit`] 的分工：`commit` 只登记元数据，用于
    /// [`ArtifactStore::restore`] 这类「字节不在手上」的场景；本函数是**产生**
    /// Artifact 的路径，它保证字节在任何元数据写入之前已按哈希落盘，并校验
    /// `artifact.content_hash` 与实际字节相符——设计 §15 的「内容按哈希寻址
    /// 落磁盘、元数据入库」由它闭合。
    ///
    /// 落盘先于登记：若登记阶段失败（如 id 重复），已写入的字节留在盘上。
    /// 内容寻址存储是幂等的，孤儿内容可被其他 Artifact 复用，故不回收。
    pub fn commit_with_content(
        &mut self,
        blob: &BlobStore,
        artifact: Artifact,
        bytes: &[u8],
    ) -> Result<(), ArtifactError> {
        if ContentHash::of(bytes) != artifact.content_hash {
            return Err(ArtifactError::ContentMismatch { id: artifact.id });
        }
        blob.put(bytes)?;
        self.commit(artifact)
    }

    pub fn get(&self, id: &ArtifactId) -> Option<&Artifact> {
        self.by_id.get(id)
    }

    /// 返回首次提交该内容的 Artifact。
    pub fn find_by_hash(&self, hash: &ContentHash) -> Option<&Artifact> {
        let id = self.by_hash.get(hash)?;
        self.by_id.get(id)
    }

    /// 去重后的内容份数。
    pub fn stored_blob_count(&self) -> usize {
        self.blobs
    }

    /// 反向追溯至原始输入。返回传递闭包，不含自身。
    /// 不存在的 id 返回空向量。
    pub fn lineage(&self, id: &ArtifactId) -> Vec<ArtifactId> {
        let mut seen: Vec<ArtifactId> = Vec::new();
        let mut stack: Vec<ArtifactId> = match self.by_id.get(id) {
            Some(a) => a.input_artifacts.clone(),
            None => return seen,
        };
        while let Some(current) = stack.pop() {
            if seen.contains(&current) {
                continue;
            }
            if let Some(a) = self.by_id.get(&current) {
                stack.extend(a.input_artifacts.iter().cloned());
            }
            seen.push(current);
        }
        seen
    }

    /// 把内存中的全部 Artifact 元数据落入 `artifact` 与 `artifact_input`。
    ///
    /// 只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责，
    /// 以便与同一事务内的其他写入（如事件）一起原子生效。
    pub fn persist(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        for artifact in self.by_id.values() {
            save_artifact(tx, artifact)?;
        }
        Ok(())
    }

    /// 从库中重建。
    ///
    /// 按**依赖序**重建而非 id 序：`commit` 要求输入 Artifact 已存在，而 id
    /// 的字典序不保证输入在前（id 为 `z` 的 Artifact 可以依赖 `a`）。每轮提交
    /// 所有输入已就位的 Artifact，直到一轮之内没有进展；此时若仍有剩余，说明
    /// 存在环或输入的 id 缺失，返回错误而不是静默丢弃。
    ///
    /// `by_hash` 的「首次提交者胜出」随之由该顺序决定。
    pub fn restore(tx: &Tx<'_>) -> Result<Self, PersistError> {
        let rows = tx.query("SELECT id FROM artifact ORDER BY id", &[])?;
        let mut pending: Vec<Artifact> = Vec::with_capacity(rows.len());
        for row in rows {
            let id = match &row[0] {
                Value::Text(s) => ArtifactId::new(s.clone()),
                other => {
                    return Err(PersistError::Database(format!(
                        "artifact.id 应为文本，实际 {other:?}"
                    )))
                }
            };
            let artifact = load_artifact(tx, &id)?
                .ok_or_else(|| PersistError::Database(format!("artifact {id} 在枚举后读不回")))?;
            pending.push(artifact);
        }

        let mut store = Self::new();
        while !pending.is_empty() {
            let mut deferred = Vec::new();
            let mut progressed = false;
            for artifact in pending {
                if artifact
                    .input_artifacts
                    .iter()
                    .all(|input| store.get(input).is_some())
                {
                    // 走 commit 而不是直接插表：恢复到与内存路径同一套不变量。
                    //
                    // 当前不可达：进入本分支的前提是输入全在 store 中、id 尚未
                    // 提交，这恰好是 commit 通过的两项检查，故 commit 必成功。
                    // 保留该分支作为防御——commit 日后新增失败模式时在此报错，
                    // 而不是静默丢 Artifact。现有用例覆盖的是循环末尾那条
                    // 「输入不存在或依赖成环」（见 restore_reports_dangling_...）。
                    let id = artifact.id.clone();
                    store.commit(artifact).map_err(|e| {
                        PersistError::Database(format!("恢复 Artifact {id} 被拒：{e}"))
                    })?;
                    progressed = true;
                } else {
                    deferred.push(artifact);
                }
            }
            if !progressed {
                let blocked: Vec<&str> = deferred.iter().map(|a| a.id.as_str()).collect();
                return Err(PersistError::Database(format!(
                    "以下 Artifact 的输入在库中不存在或依赖成环：{blocked:?}"
                )));
            }
            pending = deferred;
        }
        Ok(store)
    }
}
