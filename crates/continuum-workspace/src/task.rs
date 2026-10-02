//! Task Workspace 与 [`WritablePath`]。

use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::ids::IntentId;
use std::path::{Component, Path, PathBuf};

/// Task Workspace：本 Intent 的可写位置。
#[derive(Debug, Clone)]
pub struct TaskWorkspace {
    root: PathBuf,
    intent_id: IntentId,
}

impl TaskWorkspace {
    /// 唯一的公开构造入口。
    ///
    /// **不是测试专用。** 三种后端（worktree、overlay、以及测试夹具）都要经它产出句柄，
    /// 故不能置于 `#[cfg(test)]`。`root` 由后端决定，但必须与 `base` 无重叠。
    ///
    /// `root` 不得与 `base` 重叠：既不能等于 Base，也不能是 Base 的祖先。
    /// 前者使「Task 就是 Base」，后者使「Task 是包含 Base 的一层」——两者都会让
    /// [`crate::WritablePath`] 指向 Base（或 Base 之外），而 `Sandbox::spawn` 收
    /// `&TaskWorkspace`，内核层隔离会照着这个根反过来给 Base 开写权限，
    /// 只读强制的三层里最外一层被从内部绕过。
    ///
    /// **允许 root 位于 base 之内**（worktree 后端就是这种形态：Task 根在
    /// `<base>/.ai/worktrees/<intent>`），因为此时可写范围是 Base 的一个子目录，
    /// 而非 Base 本身。
    ///
    /// 判据是 `base_canonical.starts_with(root_canonical)`——为真即拒绝。
    /// 两侧都取 `std::fs::canonicalize` 的结果，故 symlink 与 `..` 均无法绕过。
    /// root 可能尚不存在，故**先 `create_dir_all` 再规范化**；句柄持有的也是规范化后的
    /// 路径，后续 `join` 与写操作都从它出发。
    pub fn new_outside(
        base: &BaseWorkspace,
        root: impl Into<PathBuf>,
        intent_id: IntentId,
    ) -> Result<Self, WorkspaceError> {
        let root = root.into();
        if root.exists() && !root.is_dir() {
            return Err(WorkspaceError::NotADirectory { path: root });
        }
        std::fs::create_dir_all(&root).map_err(|e| WorkspaceError::BackendUnavailable {
            reason: format!("无法建立 Task Workspace 目录 {}：{e}", root.display()),
        })?;
        let root = canonicalize(&root)?;
        let base_root = canonicalize(base.root())?;
        if base_root.starts_with(&root) {
            return Err(WorkspaceError::Overlaps {
                root,
                base: base_root,
            });
        }
        Ok(Self { root, intent_id })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 本 crate 内唯一能产出 [`WritablePath`] 的入口。
    pub fn writable_root(&self) -> WritablePath {
        WritablePath(self.root.clone())
    }
}

/// 已被证明位于 Task Workspace 内的路径。
///
/// **无公开构造函数。** 唯一来源是 [`TaskWorkspace::writable_root`]。
/// 本 crate 内所有写操作收 `&WritablePath` 而非 `&Path`，故「对 Base 写」不可表达。
///
/// 字段私有且不派生 `Serialize` / `Deserialize`：反序列化会给出第二条构造路径，
/// 使本类型的保证依赖于外部输入的正当性。
#[derive(Debug, Clone)]
pub struct WritablePath(PathBuf);

impl WritablePath {
    /// 绝对路径，位于 Task 根之内。
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// 拼接相对路径；拒绝 `..` 与绝对路径，越出 Task 根时返回 `Err`。
    ///
    /// 逐段判定而非事后规范化：`Path::components` 已经把 `.` 折叠掉，
    /// 而任何 `..`（无论是否仍在根内）与根前缀一律拒绝。缺了这一步，
    /// 本类型只保证**起点**在 Task 内，不保证**终点**在 Task 内，
    /// 类型层的保证会被一个 `join("../../etc/passwd")` 绕过。
    pub fn join(&self, rel: &str) -> Result<WritablePath, WorkspaceError> {
        let rel_path = Path::new(rel);
        let mut out = self.0.clone();
        for comp in rel_path.components() {
            match comp {
                Component::Normal(part) => out.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(WorkspaceError::EscapesRoot {
                        path: rel_path.to_path_buf(),
                    });
                }
            }
        }
        Ok(WritablePath(out))
    }

    /// 在 Task 内写入文件，必要时建立其父目录。
    pub fn write(&self, rel: &str, bytes: &[u8]) -> Result<(), WorkspaceError> {
        let path = self.join(rel)?;
        if let Some(parent) = path.0.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Self::io_error(parent, e))?;
        }
        std::fs::write(&path.0, bytes).map_err(|e| Self::io_error(&path.0, e))
    }

    /// 在 Task 内建立目录（含全部中间目录）。
    pub fn create_dir_all(&self, rel: &str) -> Result<(), WorkspaceError> {
        let path = self.join(rel)?;
        std::fs::create_dir_all(&path.0).map_err(|e| Self::io_error(&path.0, e))
    }

    /// 读回 Task 内的文件。
    pub fn read(&self, rel: &str) -> Result<Vec<u8>, WorkspaceError> {
        let path = self.join(rel)?;
        std::fs::read(&path.0).map_err(|e| Self::io_error(&path.0, e))
    }

    fn io_error(path: &Path, e: std::io::Error) -> WorkspaceError {
        WorkspaceError::IoFailed {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }
    }
}

/// 规范化路径：解析 symlink 并折叠 `..`。
///
/// 只有经此规范化的路径才可用于重叠判定——逐字符比较会让
/// `base/../base`、指向 Base 的 symlink 这类写法绕过 `new_outside` 的守卫。
fn canonicalize(path: &Path) -> Result<PathBuf, WorkspaceError> {
    std::fs::canonicalize(path).map_err(|e| WorkspaceError::BackendUnavailable {
        reason: format!("无法规范化路径 {}：{e}", path.display()),
    })
}
