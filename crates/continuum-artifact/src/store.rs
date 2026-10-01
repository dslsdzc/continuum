//! Artifact 的提交与寻址（§241、§242）。

use crate::artifact::{Artifact, ArtifactId};
use crate::content::ContentHash;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactError {
    #[error("Artifact {id} 已提交，修改必须产生新版本（§241）")]
    AlreadyCommitted { id: ArtifactId },
    #[error("Artifact {id} 不存在")]
    NotFound { id: ArtifactId },
    #[error("输入 Artifact {id} 不存在")]
    UnresolvedInput { id: ArtifactId },
}

/// 内容寻址的 Artifact 集合。相同内容只保留一份。
///
/// P1 的实现保存在内存中；落库在 Task 11。
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
}
