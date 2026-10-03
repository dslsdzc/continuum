//! Integration Gate（§257）：Task Workspace 到 Base Workspace 的唯一通道。
//!
//! Base 的工作文件永久只读（§256），Task 的改动只能经本模块进入 Base。`§257` 的
//! 五种操作里，本模块实现只读的两种：`view_diff` 与 `discard`；三种写入操作及其
//! 批准值在 Task 9。
//!
//! **只读操作不收批准值。** 它们不动 Base，按 `§15` 的影响级是 L0，收一个
//! [`GateApproval`] 只会让「这个值代表什么」变得含糊。写入操作一律收
//! `&GateApproval`，使「集成修改需要授权」有落点（§16、设计 6.2）；该类型在本模块内
//! 定义但没有公开构造函数，其产生点在下篇交给驱动。
//!
//! # 两种后端的 view_diff
//!
//! 「Task 相对 Base 改了什么」在两个后端上不是同一件事，各按自己的形态取：
//!
//! - **worktree 后端**：Task 是**某一个提交**（建分支时 Base 的 HEAD）的检出，
//!   故以「Task 分支与 Base 的分歧点」为基准走 git。Base 在创建之后新进的提交
//!   不是本 Task 的改动，不会反向出现在结果里（否则那些新文件看起来就像 Task
//!   删了它们）。
//! - **overlay 后端**：Task 的 lower 是**活的 Base**——Task 没碰过的文件在两边
//!   逐字节相同，故逐文件比较两棵当前树即可。Base 在创建之后被用户改动的文件同样
//!   不会出现在结果里，理由不同而结论一致：Task 读到的是**当前**的 Base。
//!
//! 两端因此在本层「不感知后端差异」的接口下给出各自正确的答案；差异写在上面，
//! 调用方按 `backend` 传参（创建时随句柄一同返回的那个），本层不重新探测。
//!
//! # 只读的含义
//!
//! `view_diff` 不在 Base 上留下任何改动，包括 `.git/`。这一条不是理所当然的：
//! `git diff <分歧点>` 会把刷新后的 stat 缓存**写回** Task worktree 的索引，而那个
//! 索引位于 `<base>/.git/worktrees/<intent>/index`，在 Base 的目录树之内。故差异经
//! [`IndexCopy`] 计算——把索引指到副本上，让这次写回落进临时文件（触发条件与实测
//! 证据见该类型的文档）。

use crate::backend::{WorkspaceBackend, discard_task_workspace};
use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::task::TaskWorkspace;
use crate::worktree;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 让 git 把索引写到别处的环境变量（见 [`IndexCopy`]）。
const GIT_INDEX_FILE: &str = "GIT_INDEX_FILE";

/// Integration Gate：Task Workspace 与 Base Workspace 之间的唯一通道（§257）。
///
/// 只持有一个 Base：`§257` 的通道是「Task → 它的 Base」，不是任意两者之间。
#[derive(Debug)]
pub struct IntegrationGate<'a> {
    base: &'a BaseWorkspace,
}

impl<'a> IntegrationGate<'a> {
    /// 为本 Base 建一个 Gate。
    pub fn new(base: &'a BaseWorkspace) -> Self {
        Self { base }
    }

    /// 比较 Task 与 Base，列出 Task 相对 Base 的改动。只读，不收批准值。
    ///
    /// 结果的语义按后端而异，见模块文档「两种后端的 view_diff」。
    ///
    /// `backend` 由调用方给出而不是本层重新探测：判据（Base 是否为 Git 仓库）在创建
    /// 之后可能已变（用户后来才 `git init`），按新判据取 Diff 会与磁盘上实际存在的
    /// 那个工作区对不上。这与 [`discard`](Self::discard) 收 `backend` 是同一条理由。
    pub fn view_diff(
        &self,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
    ) -> Result<Diff, GateError> {
        match backend {
            WorkspaceBackend::Worktree => diff_worktree(self.base, task),
            WorkspaceBackend::Overlay => diff_trees(self.base.root(), task.root()),
        }
    }

    /// 丢弃 Task Workspace。Base 不变。只读级（L0），不收批准值。
    ///
    /// 实现交给 [`discard_task_workspace`]，本层不复制它的逻辑：overlay 后端的
    /// 「`umount` 未成功就绝不往下走」与 worktree 后端的回收次序都在那里，
    /// 从这里另走一条路等于把那几条守卫旁路掉。
    ///
    /// **与落库记录的关系**（次序，而非原子性）：本函数成功返回之后再删记录
    /// （[`crate::remove_workspace`]），返回 `Err` 时不删。要求的实质是次序而非事务
    /// ——本层的变更全在文件系统与 git 上，本来就不可回滚；调用方的事务里若还有别的
    /// 写（放弃事件、审计记录），与那些写放进同一事务才有意义。两个方向的不一致
    /// 都要避免，其中最隐蔽的一向是「有工作区无记录」：它要到同一 Intent 再次创建、
    /// 撞上仍然存在的分支或 Intent 目录时才显形。
    pub fn discard(
        &self,
        task: &TaskWorkspace,
        backend: WorkspaceBackend,
    ) -> Result<(), GateError> {
        discard_task_workspace(self.base, task, backend)?;
        Ok(())
    }
}

/// 集成修改的批准值。
///
/// **无公开构造函数**：字段私有，也没有 `new` 一类的关联函数。写入操作（Task 9）
/// 一律收 `&GateApproval`，使「从隔离工作空间集成回用户分支需要授权」有落点
/// （`§16`、设计 6.2）——缺少它时 Gate 退化为「谁调用谁生效」。
///
/// 本子项目内由驱动的显式确认参数产生；Capability 与 Authority 就位后，其产生点
/// 移交给那一处。只读操作（[`IntegrationGate::view_diff`]、[`IntegrationGate::discard`]）
/// 不收本值：它们不动 Base。
#[derive(Debug)]
pub struct GateApproval(());

/// Task 相对 Base 的改动（`§257` 的 view diff）。
///
/// 三类，各自一组**相对路径**——基准是 Workspace 根（Task 根与 Base 根在布局上
/// 同构，两个后端的 `root()` 就是同一层的目录）。构造即规范化：组内按路径排序，
/// 且不含重复项。
///
/// **三类即全部。** 更细的分类在两种后端上并不都对得上：overlay 后端只有逐文件
/// 比较，没有 git 的重命名概念。故统一归入这三类——git 侧的重命名经 `--no-renames`
/// 拆成一「删除」一「新增」，类型变更（普通文件与符号链接互改）归入「修改」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    added: Vec<PathBuf>,
    modified: Vec<PathBuf>,
    deleted: Vec<PathBuf>,
}

impl Diff {
    /// 只在 Task 里有的文件。
    pub fn added(&self) -> &[PathBuf] {
        &self.added
    }

    /// 两侧都有、但内容不同的文件。
    pub fn modified(&self) -> &[PathBuf] {
        &self.modified
    }

    /// 只在 Base 里有的文件：在 Task 侧被删掉了。
    pub fn deleted(&self) -> &[PathBuf] {
        &self.deleted
    }

    /// 三类都为空：Task 相对 Base 没有任何改动。
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// 由三个集合构造。用 `BTreeSet` 而非 `Vec` 是让「排序且无重复」由类型保证，
    /// 不必在构造处再检查一遍。
    fn from_sets(
        added: BTreeSet<PathBuf>,
        modified: BTreeSet<PathBuf>,
        deleted: BTreeSet<PathBuf>,
    ) -> Self {
        Self {
            added: added.into_iter().collect(),
            modified: modified.into_iter().collect(),
            deleted: deleted.into_iter().collect(),
        }
    }
}

/// Integration Gate 的错误。
///
/// 与 [`WorkspaceError`] 分开而不是取别名：后者的变体描述**后端**（worktree / overlay）
/// 为何失败，本类型的变体描述**Gate 这一层**拿到了什么。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GateError {
    /// 底层 Workspace 操作失败：后端不可用、文件系统读写失败、git 命令失败等。
    ///
    /// 原样透传，调用方仍按 [`WorkspaceError`] 的变体分派。把后端的具体原因折叠成
    /// 本层的一个字符串，会让「git 究竟因何失败」（退出码、stderr）无从读取。
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    /// 后端给出的结果无法解释。
    ///
    /// 目前只有一处来源：git 在 `--name-status` 里给出了本层不认识的状态码。
    /// 这种情况下**不能丢弃那一条**——Diff 是三种写入操作的依据，少列一条意味着
    /// 有一处改动调用方看不见，而它可能是要在 Base 上生效的那个。
    #[error("Integration Gate 无法解释后端给出的结果：{reason}")]
    Malformed { reason: String },
}

/// worktree 后端的 view_diff：以分歧点为基准走 git。
///
/// 三段合起来是「分歧点 → Task 工作树」的全部改动：
/// 1. `git diff <分歧点>`：已提交在 Task 分支上的、已 `git add` 的、以及工作树里
///    未 `git add` 的改动，git 自己合成为一份逐路径的状态；
/// 2. `git ls-files --others --exclude-standard`：未跟踪文件——它们不在 `git diff`
///    的输出里（那条命令只看已跟踪的路径），一律计为「新增」。被 `.gitignore` 忽略
///    的不计（`--exclude-standard`）：三种写入操作都经 git 落到 Base 上，而 git
///    本来就不会带上被忽略的文件；
/// 3. 两者都是相对**同一个分歧点**的，故同名路径不会在两处各出现一次。
///
/// **不用 `git status` 拼**：status 的基准是 Task 分支的 HEAD（而不是分歧点），
/// 两者要另行合成，而合成在「某路径在提交里新增、又在工作树里被删掉」这类组合上
/// 有分歧点，凭空多出一张需要逐项验证的对照表。`git diff <分歧点>` 直接给出
/// git 自己的合成结果。
///
/// **基准取分歧点而非 Base 的当前 HEAD。** 用户在建好工作区之后又提交过时，那些
/// 提交里的新文件在 Task 侧并不存在；以 Base 当前 HEAD 为基准会把它们**反向**算进
/// 本 Task（看起来像 Task 删了它们）。已知限制：用户把 Base 切到创建点**之前**的
/// 提交或另一条分支上时，`merge-base` 会落到更早处，Diff 因而多算——本后端不记录
/// 创建点（句柄与落库记录里都没有它），无从取到更准的那个提交。
fn diff_worktree(base: &BaseWorkspace, task: &TaskWorkspace) -> Result<Diff, GateError> {
    let branch = worktree::branch_name(task.intent_id());
    let divergence = worktree::git(base.root(), &["merge-base", &branch, "HEAD"])?;
    if divergence.is_empty() {
        // 理论上不可达：`merge-base` 成功返回时必有输出（失败时是 `GitFailed`）。
        // 留着是因为空值传给 git 会以「未知的修订」失败，那个原因与本层的真实处境
        // （没拿到分歧点）对不上，报出来只会误导。
        return Err(GateError::Malformed {
            reason: format!("git 未给出 Task 分支 {branch} 与 Base 的分歧点"),
        });
    }

    // 索引指到副本：见 IndexCopy 的文档（git 会刷新并写回索引，而索引在 Base 之内）。
    let index = IndexCopy::for_task(task.root())?;
    let envs: [(&str, &OsStr); 1] = [(GIT_INDEX_FILE, index.path().as_os_str())];
    let changed = worktree::git_raw(
        task.root(),
        &["diff", "--no-renames", "--name-status", "-z", &divergence],
        &envs,
    )?;
    let untracked = worktree::git_raw(
        task.root(),
        &["ls-files", "--others", "--exclude-standard", "-z"],
        &envs,
    )?;

    let mut added = BTreeSet::new();
    let mut modified = BTreeSet::new();
    let mut deleted = BTreeSet::new();
    for (kind, path) in parse_name_status(&changed)? {
        let set = match kind {
            ChangeKind::Added => &mut added,
            ChangeKind::Modified => &mut modified,
            ChangeKind::Deleted => &mut deleted,
        };
        set.insert(path);
    }
    for path in parse_paths(&untracked) {
        added.insert(path);
    }
    Ok(Diff::from_sets(added, modified, deleted))
}

/// overlay 后端的 view_diff：逐文件比较两棵树。
///
/// Task 根是覆盖层的挂载点，读到的是 lower（Base）与 upper 的合并视图，故这里不必
/// 感知 overlay，两边都当普通目录遍历：同名改动（upper 遮蔽 lower）体现为「内容
/// 不同」，删除（upper 里的白障）体现为「Task 侧读不到」，与三类改动正好对上。
fn diff_trees(base_root: &Path, task_root: &Path) -> Result<Diff, GateError> {
    let base_files = collect_files(base_root)?;
    let task_files = collect_files(task_root)?;

    let mut added = BTreeSet::new();
    let mut modified = BTreeSet::new();
    let mut deleted = BTreeSet::new();
    for (path, task_bytes) in &task_files {
        match base_files.get(path) {
            None => {
                added.insert(path.clone());
            }
            Some(base_bytes) if base_bytes != task_bytes => {
                modified.insert(path.clone());
            }
            Some(_) => {}
        }
    }
    for path in base_files.keys() {
        if !task_files.contains_key(path) {
            deleted.insert(path.clone());
        }
    }
    Ok(Diff::from_sets(added, modified, deleted))
}

/// `root` 之下全部**普通文件**的相对路径 → 内容。
///
/// 只收文件，不收目录：三类改动以文件为单位。目录在两端会平白分岔——`git` 根本不
/// 跟踪目录（`mkdir` 出来的空目录在 `git diff` 里不出现），把目录计入会让 worktree
/// 与 overlay 两个后端对同一件事给出不同答案。
///
/// 符号链接经 [`std::fs::metadata`] **跟随**：指向目录的按目录递归，指向文件的按其
/// 目标内容比较。断链（`NotFound`）按「该路径上什么都没有」处理，不进结果。其余
/// 读取失败（权限等）返回 `Err` 而不跳过：跳过会让 Diff 少列一条而调用方无从得知，
/// 而 Diff 是写入操作的依据。
fn collect_files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, GateError> {
    fn walk(
        root: &Path,
        rel: &Path,
        out: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), GateError> {
        let entries = std::fs::read_dir(root).map_err(|e| io_error(root, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| io_error(root, e))?;
            let path = entry.path();
            // 相对路径由父级的相对路径拼出，不靠事后 strip_prefix：遍历的起点就是
            // 相对路径的基准，拼比减少一层「前缀对不上怎么办」。
            let rel = rel.join(entry.file_name());
            let meta = match std::fs::metadata(&path) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_error(&path, e)),
            };
            if meta.is_dir() {
                walk(&path, &rel, out)?;
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => {
                    out.insert(rel, bytes);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_error(&path, e)),
            }
        }
        Ok(())
    }

    let mut out = BTreeMap::new();
    walk(root, Path::new(""), &mut out)?;
    Ok(out)
}

/// 一条改动的类别。`Diff` 的三类各对应一个集合，本类型只在解析中途用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

/// `git diff --name-status -z` 的原始输出 → 逐条改动。
///
/// 输出是 `状态\0路径\0` 的重复。**用 `-z` 而不是逐行读**：不加 `-z` 时 git 会把
/// 非 ASCII 与含特殊字符的路径转义成 `"\346\226\207"` 一类带引号的形式，本层就得
/// 自己实现一遍反转义；`-z` 给的是原始字节。同理这里收 `&[u8]` 而非 `&str`：
/// 非 UTF-8 的文件名（Linux 上合法）经 `from_utf8_lossy` 会被替换字符顶掉，
/// 还原出来的就是另一个路径。
///
/// `--no-renames` 已让 git 把重命名拆成一删一增，故状态只有四种。别的一律返回
/// `Err`（见 [`GateError::Malformed`]）：丢弃会让 Diff 少列一条。
fn parse_name_status(bytes: &[u8]) -> Result<Vec<(ChangeKind, PathBuf)>, GateError> {
    let mut fields = bytes.split(|b| *b == 0);
    let mut out = Vec::new();
    while let Some(status) = fields.next() {
        // 末尾的 NUL 之后是一个空字段（输入为空时也只有这一个），至此结束。
        if status.is_empty() {
            break;
        }
        let kind = match status {
            b"A" => ChangeKind::Added,
            b"M" => ChangeKind::Modified,
            // 类型变更：普通文件与符号链接（或子模块）互改。同一个路径上内容不同，
            // 归入「修改」。三类里没有「类型」这一类，而它确实是一处要带进 Base 的改动。
            b"T" => ChangeKind::Modified,
            b"D" => ChangeKind::Deleted,
            other => {
                return Err(GateError::Malformed {
                    reason: format!(
                        "git diff --name-status 给出了本层不认识的状态：{:?}",
                        String::from_utf8_lossy(other)
                    ),
                });
            }
        };
        // 路径字段可能是「缺字段」也可能是「空字段」：git 的路径都相对仓库根，至少有
        // 一个分量，两者都说明这条记录不完整——不接受的代价是多出一条空路径的改动。
        let path = match fields.next() {
            Some(path) if !path.is_empty() => path,
            _ => {
                return Err(GateError::Malformed {
                    reason: "git diff --name-status 的记录里状态之后没有路径".to_owned(),
                });
            }
        };
        out.push((kind, PathBuf::from(OsString::from_vec(path.to_vec()))));
    }
    Ok(out)
}

/// `git ls-files -z` 一类的输出 → 路径列表（NUL 分隔，末尾有一个 NUL 或没有）。
fn parse_paths(bytes: &[u8]) -> Vec<PathBuf> {
    bytes
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .map(|field| PathBuf::from(OsString::from_vec(field.to_vec())))
        .collect()
}

/// `view_diff` 期间，让 git 的索引写入落到副本上。
///
/// `git diff <分歧点>` 在需要刷新索引时会**写回**索引。Task worktree 的索引位于
/// `<base>/.git/worktrees/<intent>/index`，在 Base 的目录树之内——而 `view_diff`
/// 是只读操作，不该在 Base 上留下任何改动（`§256`）。实测本机 git 2.56：
/// `GIT_OPTIONAL_LOCKS=0` 也照写（那个变量挡住的是 `git status`，不是 `git diff`）。
///
/// 触发条件**不是**「工作树里有改动」，恰恰相反：内容真的变了的条目刷新之后仍是脏的，
/// 没有可写的东西；只有**stat 缓存过期而重新哈希后内容未变**的条目会被写回新的 stat。
/// 实测四种情形：改 `a.txt` 的内容——不写；把 `b.txt` 按原内容重写或 `touch` 它——
/// 写；新增未跟踪的 `c.txt`——不写。故这条路径要靠专门的用例才走得到
/// （`tests/gate.rs` 的 `view_diff_does_not_write_the_base_when_the_stat_cache_is_stale`，
/// 它带一条用真实索引的对照臂），而它会走得到：工作区里的文件被工具重写、被检出、
/// 被 `touch`，stat 缓存过期是常态。
///
/// 处置是把真实索引复制到临时文件，再用 [`GIT_INDEX_FILE`] 把它指给 git：git 读到的
/// 索引内容与用真实索引时逐字节相同，故差异的算法与结果都不变，而刷新后的写回落进
/// 这个副本，随本值一并删除。
///
/// 副本建成后即与真实索引无关——本层是单线程调用，两次 git 之间没有让出点
/// （无 await、无线程、无回调把控制权交回调用方），真实索引不会在此期间被改动。
struct IndexCopy {
    path: PathBuf,
}

impl IndexCopy {
    /// 为 `task_root` 这个 worktree 建索引副本。
    fn for_task(task_root: &Path) -> Result<Self, GateError> {
        // 索引路径问 git 要（`--git-path` 会把 per-worktree 的路径算对），不硬拼
        // `<base>/.git/...`：Base 本身是 linked worktree 时 `.git` 是**文件**。
        // 输出可能是相对路径，故一律以 `task_root.join` 归一（`join` 遇绝对路径会
        // 直接采用它）——与 worktree 后端取 `info/exclude` 时同一条理由。
        let reported = worktree::git(task_root, &["rev-parse", "--git-path", "index"])?;
        Self::from_index(&task_root.join(reported))
    }

    /// 复制 `real_index`。索引不存在时建一个**空**副本：git 把「索引不存在」与
    /// 「索引为空」视作同一件事，本层照同一个看法处理。
    fn from_index(real_index: &Path) -> Result<Self, GateError> {
        let bytes = match std::fs::read(real_index) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(io_error(real_index, e)),
        };
        // 名字撞了就换一个。排他由 `create_new` 保证（`O_CREAT|O_EXCL`：路径已存在
        // ——包括它是一个符号链接——即失败），故不会覆盖临时目录里别人的文件。
        for _ in 0..8 {
            let path = temp_index_path();
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                // 索引里有用户仓库的路径清单，副本按属主可读可写建，不留在临时目录
                // 里给别人看。
                .mode(0o600)
                .open(&path)
            {
                Ok(mut file) => {
                    file.write_all(&bytes).map_err(|e| io_error(&path, e))?;
                    return Ok(Self { path });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io_error(&path, e)),
            }
        }
        Err(GateError::Workspace(WorkspaceError::BackendUnavailable {
            reason: "临时目录里取不到不冲突的索引副本名（连试 8 次都已被占用）".to_owned(),
        }))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for IndexCopy {
    /// 用完即删。删除失败不报错（析构里没有可返回错误的地方，且残留只在临时目录里，
    /// 对 Base 与后续操作都没有影响）。
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 临时目录里的一个候选路径。
///
/// 进程号 + 纳秒使不同进程、同一进程内的不同次调用都不会撞名；真正的排他交给
/// `create_new`（见 [`IndexCopy::from_index`]），本函数只管给出一个尽量不冲突的名字。
fn temp_index_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "continuum-gate-index-{}-{nanos}",
        std::process::id()
    ))
}

fn io_error(path: &Path, e: std::io::Error) -> GateError {
    GateError::Workspace(WorkspaceError::IoFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 拼一条 `--name-status -z` 的记录：`状态\0路径\0`。
    fn record(status: &str, path: &str) -> Vec<u8> {
        let mut out = status.as_bytes().to_vec();
        out.push(0);
        out.extend_from_slice(path.as_bytes());
        out.push(0);
        out
    }

    /// `--name-status -z` 的四种状态各归到哪一类；字节序列按 git 的实际输出形状摆。
    ///
    /// 这是 [`parse_name_status`] 分类表的唯一直接证据：真实仓库里造出「类型变更」
    /// 要额外造符号链接，而状态码到类别的映射本身就是一张表，用字面输入钉住更直接。
    /// 实测（git 2.56）`git diff --no-renames --name-status -z` 对普通文件改成符号链接
    /// 给的正是一个 `T` 记录，故这条不是凭空设想的状态。
    #[test]
    fn the_name_status_parser_maps_every_status_git_can_give() {
        let cases: [(&str, ChangeKind, &str); 4] = [
            ("A", ChangeKind::Added, "新文件.txt"),
            ("M", ChangeKind::Modified, "改过的.txt"),
            ("D", ChangeKind::Deleted, "删掉的.txt"),
            ("T", ChangeKind::Modified, "换成链接的.txt"),
        ];
        for (status, expected, path) in cases {
            let input = record(status, path);
            let parsed = parse_name_status(&input).unwrap();
            assert_eq!(parsed.len(), 1, "输入 {input:?} 应解析出一条");
            assert_eq!(parsed[0].1, PathBuf::from(path), "路径解析错：{input:?}");
            assert_eq!(parsed[0].0, expected, "输入 {input:?} 的分类不对");
        }
    }

    /// 多条记录、末尾缺 NUL、以及空输入都要解析得对。
    #[test]
    fn the_name_status_parser_reads_records_until_the_end() {
        let input = [
            record("M", "a"),
            record("D", "b"),
            record("A", "c"),
        ]
        .concat();
        let parsed = parse_name_status(&input).unwrap();
        let paths: Vec<PathBuf> = parsed.into_iter().map(|(_, p)| p).collect();
        assert_eq!(
            paths,
            vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]
        );

        // 末尾缺 NUL：git 每条记录之后（含最后一条）都会给 NUL，故这一条不会出现；
        // 它在这里只用来钉住「字段耗尽即结束」，不把最后一条读丢。
        let parsed = parse_name_status(b"M\0a").unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1, PathBuf::from("a"));

        assert!(parse_name_status(b"").unwrap().is_empty());
    }

    /// 不认识的状态一律报错，**不丢弃**：丢弃会让 Diff 少列一条改动。
    #[test]
    fn the_name_status_parser_refuses_a_status_it_does_not_know() {
        // `U` 是合并冲突留下的未合并条目，`R`/`C` 在 `--no-renames` 下不该出现，
        // `X` 是 git 自己都不认的未知状态。三者都不在本层的分类表里。
        for input in [
            record("U", "冲突的.txt"),
            record("R100", "旧名.txt"),
            record("X", "怪的.txt"),
        ] {
            let err = parse_name_status(&input).unwrap_err();
            match &err {
                GateError::Malformed { reason } => assert!(
                    reason.contains("状态"),
                    "错误未说明是状态不认识：{reason}"
                ),
                other => panic!("期望 Malformed，得到 {other:?}"),
            }
        }
        // 状态之后没有路径（缺字段与空字段两种形态）同样是 Malformed，
        // 而不是「解析成一条空路径的改动」
        for input in [&b"A"[..], b"A\0", b"A\0\0"] {
            let err = parse_name_status(input).unwrap_err();
            assert!(
                matches!(err, GateError::Malformed { .. }),
                "输入 {input:?} 得到 {err:?}"
            );
        }
    }

    /// 非 UTF-8 的文件名按原始字节还原，不经替换字符。
    #[test]
    fn the_name_status_parser_keeps_non_utf8_paths_intact() {
        let raw = b"A\0\xff\xfe.txt\0";
        let parsed = parse_name_status(raw).unwrap();
        assert_eq!(
            parsed[0].1.as_os_str().as_encoded_bytes(),
            b"\xff\xfe.txt",
            "非 UTF-8 的路径被改写"
        );
    }

    /// 逐文件比较的三类判定：只在 Task 里、两边都有而内容不同、只在 Base 里。
    #[test]
    fn the_tree_diff_classifies_each_file() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root.join("子目录")).unwrap();
            std::fs::write(root.join("相同.txt"), "一样".as_bytes()).unwrap();
            std::fs::write(root.join("子目录/改过的.txt"), "原".as_bytes()).unwrap();
        }
        std::fs::write(base.join("只在_base_里.txt"), "旧".as_bytes()).unwrap();
        std::fs::write(task.join("新文件.txt"), "新".as_bytes()).unwrap();
        std::fs::write(task.join("子目录/改过的.txt"), "改".as_bytes()).unwrap();

        let diff = diff_trees(&base, &task).unwrap();
        assert_eq!(diff.added(), &[PathBuf::from("新文件.txt")][..], "{diff:?}");
        assert_eq!(
            diff.modified(),
            &[PathBuf::from("子目录/改过的.txt")][..],
            "嵌套路径的比较不对：{diff:?}"
        );
        assert_eq!(
            diff.deleted(),
            &[PathBuf::from("只在_base_里.txt")][..],
            "{diff:?}"
        );
        assert!(!diff.is_empty());
    }

    /// 两侧完全一致时三类都空——空目录与只在 Task 里存在的目录都不算改动。
    #[test]
    fn the_tree_diff_ignores_directories_and_equal_files() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let task = dir.path().join("task");
        for root in [&base, &task] {
            std::fs::create_dir_all(root.join("子目录")).unwrap();
            std::fs::write(root.join("一样.txt"), "一样".as_bytes()).unwrap();
        }
        // 只在 Task 里建一个**空**目录：目录不进结果，故 Diff 仍为空
        std::fs::create_dir_all(task.join("空目录")).unwrap();

        let diff = diff_trees(&base, &task).unwrap();
        assert!(diff.is_empty(), "空目录被算成了改动：{diff:?}");
    }

    /// 索引副本：内容与真实索引逐字节相同，且随本值一并删除。
    #[test]
    fn the_index_copy_mirrors_the_index_and_cleans_up_after_itself() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("index");
        std::fs::write(&real, "假的索引内容".as_bytes()).unwrap();

        let copy = IndexCopy::from_index(&real).unwrap();
        let path = copy.path().to_path_buf();
        assert_ne!(path, real, "副本不该就是真实索引本身");
        assert_eq!(std::fs::read(&path).unwrap(), "假的索引内容".as_bytes());

        drop(copy);
        assert!(!path.exists(), "副本未被删除：{}", path.display());
        assert!(real.exists(), "真实索引被删了");
    }

    /// 索引不存在时建一个空副本——git 眼中「没有索引」与「索引为空」是同一件事。
    #[test]
    fn the_index_copy_of_a_missing_index_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let copy = IndexCopy::from_index(&dir.path().join("并不存在的索引")).unwrap();
        assert_eq!(std::fs::read(copy.path()).unwrap(), Vec::<u8>::new());
    }

    /// 每次都是新的临时文件，不会撞上上一次留下的名字。
    #[test]
    fn each_index_copy_gets_its_own_path() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("index");
        std::fs::write(&real, b"x").unwrap();
        let a = IndexCopy::from_index(&real).unwrap();
        let b = IndexCopy::from_index(&real).unwrap();
        assert_ne!(a.path(), b.path());
    }
}
