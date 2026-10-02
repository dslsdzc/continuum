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
    /// `root` 必须是**绝对路径**，且存在、为目录。
    ///
    /// 绝对路径是硬要求，不是规范化的前置。`root()` 是 Base 的稳定标识：worktree
    /// 后端拿它当 `git -C` 的工作目录，overlay 后端拿它算存储标识，而 `WorkspaceRecord`
    /// 落库后再跨 cwd 读回还会解析一次。相对路径的含义随调用方的当前目录而变，
    /// 这三种用法都会因而指向别处。
    ///
    /// 本函数**不代为 `canonicalize`**：那会改变路径标识（本机 `TMPDIR` 经符号链接
    /// 时尤甚），而 Base 的标识应当就是调用方给出的那个路径——worktree 后端据此调
    /// git，overlay 后端另行规范化后再算标识（见 `overlay::canonical_base`）。
    ///
    /// 判定先于存在性判定：相对路径无论是否存在都是无效输入。
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, WorkspaceError> {
        let root = root.into();
        if !root.is_absolute() {
            return Err(WorkspaceError::NotAbsolute { path: root });
        }
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
