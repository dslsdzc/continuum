//! Base Workspace：用户的原始工作区。

use crate::error::WorkspaceError;
use std::path::{Path, PathBuf};

/// Base Workspace：用户的工作区，永久只读。
///
/// 本类型**不提供任何可写句柄**——没有 `writable_root`，没有 [`crate::WritablePath`]
/// 的构造路径。写 Base 的私有路径仅被 `gate` 模块的三种写入操作调用。
/// 该不可构造性是 §256 在类型层的落点。
///
/// 本类型亦不提供任何原地修改 Base 的方法（改权限、删除、重命名）。
/// `§256` 的隔离由文件系统或等价机制强制，而非由本类型修改用户目录的属性达成。
#[derive(Debug, Clone)]
pub struct BaseWorkspace {
    root: PathBuf,
}

impl BaseWorkspace {
    /// `root` 必须存在且为目录。
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, WorkspaceError> {
        let root = root.into();
        let meta = std::fs::metadata(&root).map_err(|_| WorkspaceError::MissingBase {
            path: root.clone(),
        })?;
        if !meta.is_dir() {
            return Err(WorkspaceError::NotADirectory { path: root });
        }
        Ok(Self { root })
    }

    /// Base 的根路径。只读视图：调用方拿到 `&Path`，无法由此构造可写句柄。
    pub fn root(&self) -> &Path {
        &self.root
    }
}
