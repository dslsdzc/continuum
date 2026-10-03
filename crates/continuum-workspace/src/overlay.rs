//! OverlayFS 后端（§255）。
//!
//! Base 不是 Git 仓库时，Task Workspace 是一个 OverlayFS 覆盖层：lowerdir 为 Base，
//! upperdir 为该 Intent 的私有目录，`discard` 即丢弃 upper 层（§255）。
//!
//! 挂载与卸载经外部命令 `mount` / `umount`，失败时把命令、退出码与原样 stderr 放进
//! [`WorkspaceError::CommandFailed`]。
//!
//! 目录布局为 `<overlay 根>/<Base 标识>/<intent>/{upper,work,mnt}`，其中 overlay 根
//! 默认 `~/.local/share/continuum/overlays`，可由 [`OVERLAY_ROOT_ENV`] 覆盖。

use crate::backend::check_intent;
use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::ids::IntentId;
use crate::task::{TaskWorkspace, canonical_candidate};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 覆盖 overlay 根的环境变量（测试用）。
pub(crate) const OVERLAY_ROOT_ENV: &str = "CONTINUUM_OVERLAY_ROOT";

/// 未设 [`OVERLAY_ROOT_ENV`] 时，overlay 根在 `$HOME` 之下的相对路径。
const DEFAULT_OVERLAY_DIR: &str = ".local/share/continuum/overlays";

/// Base 标识取 sha256 的前多少位十六进制字符。
const ID_HEX_LEN: usize = 16;

/// OverlayFS 后端。
///
/// **命名空间约束。** 无特权 OverlayFS 的挂载**只在其所在的用户与挂载命名空间内可见**，
/// 命名空间退出即消失。故本后端要求调用进程已处在一个用户与挂载命名空间内，且该命名空间
/// 须存活至子进程用完工作区为止；不在时 [`crate::create_task_workspace`] 返回
/// [`WorkspaceError::NotInNamespace`]，**不静默建出一个对子进程不可见的工作区**。
/// 驱动的处置是：需要本后端时把自身 re-exec 进 `unshare -Urm`，此后其子进程继承同一
/// 命名空间。以 root 运行是唯一例外——初始命名空间下 root 有 `CAP_SYS_ADMIN`，无需
/// 用户命名空间即可挂载，故同样放行。
///
/// **工作目录在 Base 之外。** 若把 upper 放在 `<base>/.ai/` 下，则 lower 为整个 Base 时，
/// Intent A 的工作区能读到 `<base>/.ai/overlays/B/upper` 里 Intent B 的未提交工作——这是
/// 跨 Intent 的**读取**泄漏，而规范只约束了写入。故 overlay 根默认取
/// `~/.local/share/continuum/overlays`，Base 标识取其规范路径的 sha256 前 [`ID_HEX_LEN`]
/// 位十六进制。overlay 根落在 Base 之内时 [`create`] 直接拒绝（见该函数的注释）。
///
/// **与 worktree 后端的差异。** 两者的工作区存储位置不同，这是上一条的直接结果：worktree
/// 后端把工作区放在 `<base>/.ai/worktrees/`（`.ai/` 被 gitignore，而 gitignore 的目录不会
/// 出现在 git worktree 的检出里，故 Task 内看不到它）；overlay 后端必须放在 Base 之外。
///
/// **`.ai/` 的可见性。** lowerdir 是整个 Base 根，故 Base 内的 `.ai/` 也会出现在 overlay
/// 的 lower 中，Task 内因此能看到自己的元数据目录。本后端**不做排除**：OverlayFS 的
/// `lowerdir` 只支持以 `:` 分隔的多层，没有「排除某项」的语法，而真正的排除需要改用多层
/// lower（父目录 + 逐项），代价与收益不相称。功能上无害——upper 层的改动仍与 Base 隔离，
/// 只是 Task 内多看到一个只读的 `.ai/`。worktree 后端没有这个可见性，这是两端在
/// 「Task 内能看到什么」上的又一处分歧。
///
/// **可见不等于算改动。** 集成那一侧按名字排除任意层级的 `.ai/`（`gate` 模块的
/// `collect_files`），故 Task 内能看到的 `.ai/` 既不出现在 `view_diff` 里，也不会被
/// `integrate_overlay` 复制进 Base——「`view_diff` 报出的 == 集成落进去的」那条不变式
/// 因此照旧成立。**本后端两侧是同一套判据**（都跑 `collect_files` 那个名字测试）；
/// worktree 后端不是（那一侧是 git 的规则，只管目录、只管未跟踪路径），差别见
/// `gate` 模块 `approve_integration` 的文档。
pub struct OverlayBackend;

/// 本进程是否处于一个用户命名空间内。
///
/// 判定读 `/proc/self/uid_map`：初始用户命名空间是恒等映射（`0 0 4294967295`），
/// 非初始命名空间下映射序列与宿主不同（`unshare -Urm` 之后形如 `0 <真实 uid> 1`），
/// 多行映射同样只可能出现在非初始命名空间。
///
/// **读不到时返回 `false`**：非 Linux 或受限环境无从判定，一律按「不在」处理，
/// 于是 [`create`] 报 [`WorkspaceError::NotInNamespace`] 而非在无保证的情况下挂载。
/// 这是 fail-closed 的方向，与「不静默建出不可见的工作区」一致。
///
/// **只判用户命名空间，不判挂载命名空间。** `unshare -U`（不带 `-m`）之后本函数返回真，
/// 而该进程并没有自己的挂载命名空间，`mount` 未必成功。这个范围是刻意的：判据只回答
/// 「`uid_map` 是否非恒等映射」这一个问题，overlay 挂不挂得上由 `mount` 自己回答。误判的
/// 方向是**响的**——挂载失败会经 [`WorkspaceError::CommandFailed`] 带 stderr 报出，不会
/// 静默建出一个坏掉的工作区。要一个真能用的工作区，调用方应 re-exec 进 `unshare -Urm`
/// （用户与挂载命名空间一并建立），见 [`OverlayBackend`] 的文档。
pub fn in_user_namespace() -> bool {
    match std::fs::read_to_string("/proc/self/uid_map") {
        Ok(text) => !is_initial_uid_map(&text),
        Err(_) => false,
    }
}

/// `/proc/self/uid_map` 的内容是否为初始用户命名空间的恒等映射。
fn is_initial_uid_map(text: &str) -> bool {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let Some(first) = lines.next() else {
        // 空文件：读得出但没有任何映射，不是本进程应有的形态，按「不在」处理。
        return false;
    };
    if lines.next().is_some() {
        // 多行映射：初始命名空间只有一行恒等映射。
        return false;
    }
    let fields: Vec<u64> = first
        .split_whitespace()
        .filter_map(|f| f.parse().ok())
        .collect();
    fields.len() == 3 && fields == [0, 0, u64::from(u32::MAX)]
}

/// 本进程能否建立无特权 OverlayFS 挂载：已在用户命名空间内，或以 root 运行。
fn can_mount_overlay() -> bool {
    in_user_namespace() || is_root()
}

/// 本进程的有效 uid 是否为 0。
///
/// 读 `/proc/self/status` 的 `Uid:` 行（真实、有效、保存、文件系统四项）而非
/// 链接 libc——本 crate 至今没有 C 依赖，为一次判定引入它不划算。
fn is_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .as_deref()
        .and_then(effective_uid)
        == Some(0)
}

/// 从 `/proc/self/status` 的内容里取有效 uid（`Uid:` 行的第二项）。
fn effective_uid(status: &str) -> Option<u32> {
    status.lines().find_map(|line| {
        let rest = line.strip_prefix("Uid:")?;
        rest.split_whitespace().nth(1)?.parse().ok()
    })
}

/// 一个 Intent 的 overlay 目录布局：`<overlay 根>/<Base 标识>/<intent>/{upper,work,mnt}`。
struct Layout {
    /// `<overlay 根>/<Base 标识>`：同一 Base 的全部 Intent 共用，清空后一并移除。
    base_dir: PathBuf,
    /// 本 Intent 的私有目录，`discard` 删的就是它，也是 `create` 拒绝复用的对象。
    intent_dir: PathBuf,
    /// `lowerdir`：Base 的**规范**路径。
    lower: PathBuf,
    /// `upperdir`：本 Intent 的写入层。
    upper: PathBuf,
    /// `workdir`：OverlayFS 的工作目录，必须与 `upperdir` 同文件系统且为空。
    work: PathBuf,
    /// 挂载点，即返回的 Task 根。
    mnt: PathBuf,
}

impl Layout {
    fn new(base: &BaseWorkspace, intent: &IntentId) -> Result<Self, WorkspaceError> {
        let lower = canonical_base(base)?;
        let base_dir = overlay_root()?.join(base_id(&lower));
        let intent_dir = base_dir.join(intent.as_str());
        Ok(Self {
            upper: intent_dir.join("upper"),
            work: intent_dir.join("work"),
            mnt: intent_dir.join("mnt"),
            base_dir,
            intent_dir,
            lower,
        })
    }
}

/// overlay 根：优先 [`OVERLAY_ROOT_ENV`]，否则 `$HOME/.local/share/continuum/overlays`。
fn overlay_root() -> Result<PathBuf, WorkspaceError> {
    if let Some(value) = std::env::var_os(OVERLAY_ROOT_ENV) {
        if value.is_empty() {
            return Err(WorkspaceError::BackendUnavailable {
                reason: format!("{OVERLAY_ROOT_ENV} 为空：overlay 根不能是空路径"),
            });
        }
        return Ok(PathBuf::from(value));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| WorkspaceError::BackendUnavailable {
        reason: format!("HOME 未设置，无法确定默认 overlay 根（可由 {OVERLAY_ROOT_ENV} 指定）"),
    })?;
    Ok(PathBuf::from(home).join(DEFAULT_OVERLAY_DIR))
}

/// Base 的**规范**路径。重叠判定、标识哈希与 `lowerdir` 都用它。
///
/// 用规范路径而非字面路径：同一 Base 经不同写法（符号链接、`..`）到来时必须落到同一份
/// overlay 存储，否则同一个 Base 会有多份互不相见的 upper 层。
fn canonical_base(base: &BaseWorkspace) -> Result<PathBuf, WorkspaceError> {
    std::fs::canonicalize(base.root()).map_err(|e| WorkspaceError::BackendUnavailable {
        reason: format!("无法规范化 Base 路径 {}：{e}", base.root().display()),
    })
}

/// Base 标识：其规范路径的 sha256 前 [`ID_HEX_LEN`] 位十六进制。
///
/// 入参须已是规范路径（见 [`canonical_base`]）；本函数不再规范化，以免同一事实有
/// 两处来源。
fn base_id(canonical_lower: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_lower.as_os_str().as_encoded_bytes());
    let digest = hasher.finalize();
    hex(&digest[..ID_HEX_LEN / 2])
}

/// 字节序列的十六进制表示（小写）。
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

/// 组装 `mount -o` 的选项串：`lowerdir=<base>,upperdir=<upper>,workdir=<work>`。
///
/// 三个路径含 `,` 或 `:` 时拒绝：选项串以 `,` 分隔各选项，`,` 会让内核的解析错位；
/// `lowerdir` 内以 `:` 分隔多层，Base 路径里的 `:` 会被当作分层。两者都只会得到一个
/// 与真实原因对不上的挂载错误，故在进入外部命令之前挡下，给出能对照的说明。
///
/// `upperdir` 与 `workdir` 是单层路径，`:` 在其中并无特殊含义，仍一并拒绝是刻意的：
/// 三处用同一条规则，省得日后有人按「只有 `lowerdir` 需要」放宽而连带漏掉 `,`，
/// 也让错误信息不必按选项分岔。
fn mount_options(lower: &Path, upper: &Path, work: &Path) -> Result<String, WorkspaceError> {
    let mut parts = Vec::with_capacity(3);
    for (name, path) in [("lowerdir", lower), ("upperdir", upper), ("workdir", work)] {
        let text = path
            .to_str()
            .ok_or_else(|| WorkspaceError::BackendUnavailable {
                reason: format!("{name} 不是合法 UTF-8，无法传给 mount：{}", path.display()),
            })?;
        if let Some(bad) = text.chars().find(|c| *c == ',' || *c == ':') {
            return Err(WorkspaceError::BackendUnavailable {
                reason: format!(
                    "{name} 的路径含 {bad:?}，overlay 的挂载选项以 , 分隔、以 : 分隔多层 lower，\
                     该路径无法表达：{}",
                    path.display()
                ),
            });
        }
        parts.push(format!("{name}={text}"));
    }
    Ok(parts.join(","))
}

/// 创建一个 Task Workspace：建目录、挂载覆盖层、产出句柄。
///
/// 次序：校验 intent → 算布局 → 判定工作目录在 Base 之外 → 判定在命名空间内 →
/// 建 `{upper,work,mnt}` → 产出句柄 → 挂载。
///
/// **两类拒绝（intent、工作目录位置）排在命名空间判定之前**：它们都是「输入或配置不合法」
/// 的答复，任一成立时都不该落到「先建目录再挂载」那一段，故顺序只影响报出哪一个错误，
/// 不影响磁盘。这样排也让这两类判定在无命名空间的测试进程里可达——否则覆盖它们的用例
/// 必须先 re-exec 进命名空间。**命名空间判定排在 `create_dir_all` 之前**：不在命名空间时
/// 一个目录都不建，否则会在 overlay 根下留下对谁都不可见（也不能用）的空壳。
///
/// **句柄在挂载之前产出。** 挂载是最后一步可能失败的动作，故失败回收只需处理「目录已建、
/// 覆盖层未挂」这一种处境，不必再有一条「已挂载后失败」的回收路径。挂载前后
/// [`TaskWorkspace::new_outside`] 拿到的都是同一个路径（挂载点自身不经由符号链接解析），
/// 句柄持有的根与随后的挂载点是同一个。
pub(crate) fn create(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<TaskWorkspace, WorkspaceError> {
    check_intent(intent)?;
    let layout = Layout::new(base, intent)?;

    // 工作目录必须在 Base 之外。判定用 canonical_candidate（不产生副作用），
    // 故被拒绝的配置不会在磁盘上留下任何目录。
    let lower = layout.lower.clone();
    let candidate = canonical_candidate(&layout.intent_dir)?;
    if candidate.starts_with(&lower) {
        return Err(WorkspaceError::BackendUnavailable {
            reason: format!(
                "overlay 工作目录 {} 位于 Base {} 之内：lower 为整个 Base 时，\
                 别的 Intent 能读到它的未提交工作（跨 Intent 读取泄漏），\
                 故 overlay 根不得落在 Base 之内",
                candidate.display(),
                lower.display()
            ),
        });
    }

    if !can_mount_overlay() {
        return Err(WorkspaceError::NotInNamespace);
    }

    // 不复用已存在的 Intent 目录：里头的 upper 层是上一次留下的未提交工作，
    // 静默接续会让本次 Intent 看到不属于它的改动。宁可见地失败。
    if layout.intent_dir.exists() {
        return Err(WorkspaceError::BackendUnavailable {
            reason: format!(
                "该 Intent 的 overlay 工作目录已存在：{}（上一次创建的残留，须先清理）",
                layout.intent_dir.display()
            ),
        });
    }

    // 选项串在落盘之前算好：它只依赖路径，失败时磁盘上还没有任何东西要回收。
    let options = mount_options(&layout.lower, &layout.upper, &layout.work)?;

    for dir in [&layout.upper, &layout.work, &layout.mnt] {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return Err(with_cleanup(io_error(dir, e), cleanup_failed_create(&layout)));
        }
    }

    let task = match TaskWorkspace::new_outside(base, &layout.mnt, intent.clone()) {
        Ok(task) => task,
        Err(e) => return Err(with_cleanup(e, cleanup_failed_create(&layout))),
    };

    if let Err(e) = mount(&options, &layout.mnt) {
        return Err(with_cleanup(e, cleanup_failed_create(&layout)));
    }
    Ok(task)
}

/// 确认 `mnt` 确实是本后端建出的 overlay 布局：它是 `<intent>` 目录下的 `mnt`，
/// 且同级目录里有 `upper` 与 `work`。
///
/// 判定的用途是**把「别的后端建出的 Task 根」报成「不是本后端的布局」**，而不是让它
/// 走到后面某一步以别的名义失败（`discard` 那边会变成「umount 失败」，把调用方的误用
/// 说成外部命令故障）。故本函数**必须排在调用方动第一个字节之前**：`discard` 排在
/// `umount` 之前，Gate 的写入操作排在集成之前。
///
/// worktree 后端建出的 Task 根（`<base>/.ai/worktrees/<intent>`）在这里必然失败——它的
/// 同级目录是别的 Intent，没有 `upper` / `work`。这正是本判定要拦住的那一类误用。
pub(crate) fn require_layout(mnt: &Path) -> Result<(), WorkspaceError> {
    let Some(intent_dir) = mnt.parent() else {
        return Err(WorkspaceError::BackendUnavailable {
            reason: format!(
                "Task 根 {} 没有父目录，不是本后端建出的 overlay 布局",
                mnt.display()
            ),
        });
    };
    for sibling in ["upper", "work"] {
        if !intent_dir.join(sibling).is_dir() {
            return Err(WorkspaceError::BackendUnavailable {
                reason: format!(
                    "Task 根 {} 不是本后端建出的 overlay 布局：其同级目录中没有 {sibling} \
                     （布局为 <Base 标识>/<intent>/{{upper,work,mnt}}）",
                    mnt.display()
                ),
            });
        }
    }
    Ok(())
}

/// 放弃一个 Task Workspace：卸载覆盖层，再删除其工作目录。
///
/// 先卸载再删除：挂载点被挂载时 `remove_dir_all` 会以 `EBUSY` 失败，反序一步也做不成。
///
/// **`umount` 未成功就绝不往下走。** 卸载失败仍继续删目录，只会有两种结局：路径确实仍被
/// 挂载时，删以 `EBUSY` 失败，错误里 umount 给出的具体原因被后一个失败盖掉；路径并非挂载
/// 点时（误用或中间态），删会**成功**，把一个不属于本次放弃的目录整删掉。故 `umount` 的
/// 错误原样返回（带退出码与 stderr），不尝试继续。
///
/// **形态校验排在 `umount` 之前**，判定见 [`require_layout`]。少了这一步，误用会走到
/// `umount`，那里的答复是「umount 失败」——把调用方的误用说成外部命令故障；而若该路径下
/// 当真挂了别的东西，`umount` 会成功，随后 `remove_dir_all` 整删一个不属于本后端的目录
/// （例如 `<base>/.ai/worktrees/<intent>`）。
///
/// 该判定不止此处要：Gate 的写入操作**同样**收一个由调用方给出的 `backend`，误用在那里
/// 的后果是删改 Base（见 [`crate::gate`]），故它排在任何改动之前，共用本函数。
///
/// `<overlay 根>/<Base 标识>` 只在其**已空**时移除（`remove_dir` 不递归），故住在同一
/// Base 下的别的 Intent 不受影响；移除失败不报错——非空正是「还有别的 Intent」这一正常
/// 情形的表现。overlay 根自身不动：它是 Runtime 的公共目录，可由多个 Base 共用。
///
/// **中间态没有重试路径。** `umount` 成功而 `remove_dir_all` 失败时，该 Intent 会卡死：
/// 再调 `discard` 会在 `umount` 处失败（那时已不是挂载点），而 `create` 又因 `intent_dir`
/// 已存在（见该函数里对既有目录的拒绝）不肯重建。返回的 [`WorkspaceError::IoFailed`]
/// 带 `path`，人工据此删除该目录即可；除手工收拾外没有自动回收的路子。
pub(crate) fn discard(task: &TaskWorkspace) -> Result<(), WorkspaceError> {
    // 形态校验（见本函数的文档）：在 umount 之前确认这确实是本后端的布局。
    // 经它之后 `parent()` 必定是 `Some`（`require_layout` 已判过）。
    require_layout(task.root())?;
    let mnt = task.root();
    let intent_dir = mnt.parent().expect("require_layout 已确认有父目录");
    umount(mnt)?;
    std::fs::remove_dir_all(intent_dir).map_err(|e| io_error(intent_dir, e))?;
    if let Some(base_dir) = intent_dir.parent() {
        let _ = std::fs::remove_dir(base_dir);
    }
    Ok(())
}

/// 挂载覆盖层：`mount -t overlay overlay -o <options> <target>`。
fn mount(options: &str, target: &Path) -> Result<(), WorkspaceError> {
    run(
        "mount",
        &["-t", "overlay", "overlay", "-o", options, path_str(target)?],
    )
}

/// 卸载覆盖层。
fn umount(target: &Path) -> Result<(), WorkspaceError> {
    run("umount", &[path_str(target)?])
}

/// 执行一条外部命令；非零退出即 [`WorkspaceError::CommandFailed`]（带退出码与 stderr）。
///
/// 以退出码判定成败，不看 stderr：`mount` 成功时也可能往 stderr 写警告。失败时把
/// 退出码与 stderr 原样带回，截断或改写都会让调用方丢掉命令给出的具体原因。
fn run(program: &str, args: &[&str]) -> Result<(), WorkspaceError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| WorkspaceError::BackendUnavailable {
            reason: format!("无法执行 {program}：{e}"),
        })?;
    if !output.status.success() {
        return Err(WorkspaceError::CommandFailed {
            command: format!("{program} {}", args.join(" ")),
            // 被信号杀死时没有退出码；取 -1 仅作占位。
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim_end().to_owned(),
        });
    }
    Ok(())
}

/// 一次失败的创建之后，清掉本次建出来的东西。
///
/// 只做两件事：删掉本 Intent 的目录（`intent_dir` 在 [`create`] 里已确认本次之前不存在，
/// 故整体删除不会误伤别人），以及在其已空时移除 `<Base 标识>` 目录。overlay 根不动——
/// 它可由多个 Base 共用，且空目录对谁都没有妨碍。
///
/// 删除失败时返回该错误：调用方拿到的错误里要能看出仓库外还剩什么。
/// `<Base 标识>` 那一步的失败被忽略——非空表示还有别的 Intent 住在里面，这是正常情形
/// 而非残留。
fn cleanup_failed_create(layout: &Layout) -> Result<(), WorkspaceError> {
    std::fs::remove_dir_all(&layout.intent_dir).map_err(|e| io_error(&layout.intent_dir, e))?;
    let _ = std::fs::remove_dir(&layout.base_dir);
    Ok(())
}

/// 把「创建为什么失败」与「回收本次创建的结果」合成最终错误。
///
/// 回收成功时主错误就是全部事实；回收失败时两个都要给调用方——主错误说明创建为什么
/// 失败，回收错误说明磁盘上可能还剩什么。
fn with_cleanup(err: WorkspaceError, cleanup: Result<(), WorkspaceError>) -> WorkspaceError {
    match cleanup {
        Ok(()) => err,
        Err(c) => WorkspaceError::Context {
            context: format!("回收本次创建亦失败：{c}"),
            source: Box::new(err),
        },
    }
}

/// 取出可传给外部命令的路径参数。非 UTF-8 的路径无法作为命令行参数传递。
fn path_str(path: &Path) -> Result<&str, WorkspaceError> {
    path.to_str().ok_or_else(|| WorkspaceError::BackendUnavailable {
        reason: format!("路径不是合法 UTF-8，无法传给外部命令：{}", path.display()),
    })
}

fn io_error(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::IoFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 初始用户命名空间的 uid_map 是单行恒等映射；其余形态都表示已在某个用户命名空间内。
    #[test]
    fn the_initial_uid_map_is_recognized_only_in_its_identity_form() {
        // 本机初始命名空间的原文（含内核的对齐空格）
        assert!(is_initial_uid_map("         0          0 4294967295\n"));
        // `unshare -Urm` 之后
        assert!(!is_initial_uid_map("         0       1000          1\n"));
        // 多行映射只可能出现在非初始命名空间
        assert!(!is_initial_uid_map("0 1000 1\n1 1001 1\n"));
        // 读得出但为空：不是本进程应有的形态，按「不在」处理
        assert!(!is_initial_uid_map(""));
        assert!(!is_initial_uid_map("   \n"));
    }

    /// 有效 uid 取 `Uid:` 行的第二项，不是第一项（真实 uid）。
    #[test]
    fn the_effective_uid_is_read_from_the_second_field() {
        let status = "Name:\tcat\nUid:\t1000\t1000\t1000\t1000\nGid:\t1000\t1000\t1000\t1000\n";
        assert_eq!(effective_uid(status), Some(1000));
        let root = "Uid:\t0\t0\t0\t0\n";
        assert_eq!(effective_uid(root), Some(0));
        // 没有 Uid 行时无从判定
        assert_eq!(effective_uid("Name:\tcat\n"), None);
    }

    /// Base 标识必须跟着**规范**路径走：同一目录的两种写法要落到同一份 overlay 存储。
    #[test]
    fn the_base_id_follows_the_canonical_path_not_the_literal_one() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let via_real = base_id(&canonical_base(&BaseWorkspace::new(&real).unwrap()).unwrap());
        let via_link = base_id(&canonical_base(&BaseWorkspace::new(&link).unwrap()).unwrap());
        assert_eq!(
            via_real, via_link,
            "同一目录的符号链接写法给出了不同的 Base 标识"
        );
        assert_eq!(via_real.len(), ID_HEX_LEN);
        assert!(
            via_real.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "Base 标识应为小写十六进制：{via_real}"
        );
    }

    /// 选项串的形状；含 `,` 或 `:` 的路径在进入 `mount` 之前就被拒。
    #[test]
    fn the_mount_options_are_rejected_when_a_path_contains_a_separator() {
        let opts = mount_options(Path::new("/b"), Path::new("/o/upper"), Path::new("/o/work"))
            .unwrap();
        assert_eq!(opts, "lowerdir=/b,upperdir=/o/upper,workdir=/o/work");

        // `,` 会让内核的选项解析错位，`:` 在 lowerdir 里是分层分隔符
        for bad in ["/b,ar", "/b:ar"] {
            let err = mount_options(Path::new(bad), Path::new("/o/upper"), Path::new("/o/work"))
                .unwrap_err();
            assert!(
                matches!(err, WorkspaceError::BackendUnavailable { .. }),
                "含 {bad:?} 的路径未被拒绝：{err:?}"
            );
            assert!(
                err.to_string().contains(bad),
                "错误里未给出出问题的路径：{err}"
            );
        }
    }

    /// 失败的创建清掉本次建出的 Intent 目录，并在 `<Base 标识>` 目录已空时一并移除。
    #[test]
    fn a_failed_create_removes_the_intent_directory() {
        let root = tempfile::tempdir().unwrap();
        let layout = Layout {
            base_dir: root.path().join("id"),
            intent_dir: root.path().join("id").join("i1"),
            lower: PathBuf::from("/base"),
            upper: root.path().join("id").join("i1").join("upper"),
            work: root.path().join("id").join("i1").join("work"),
            mnt: root.path().join("id").join("i1").join("mnt"),
        };
        for d in [&layout.upper, &layout.work, &layout.mnt] {
            std::fs::create_dir_all(d).unwrap();
        }
        cleanup_failed_create(&layout).unwrap();
        assert!(!layout.intent_dir.exists(), "Intent 目录未清理");
        assert!(!layout.base_dir.exists(), "已空的 Base 标识目录未清理");
        assert!(root.path().exists(), "overlay 根本身不该被删");
    }

    /// 别的 Intent 住在同一 Base 下时，`<Base 标识>` 目录不是空的，不得被移除。
    #[test]
    fn a_failed_create_keeps_a_base_directory_that_still_holds_another_intent() {
        let root = tempfile::tempdir().unwrap();
        let layout = Layout {
            base_dir: root.path().join("id"),
            intent_dir: root.path().join("id").join("i1"),
            lower: PathBuf::from("/base"),
            upper: root.path().join("id").join("i1").join("upper"),
            work: root.path().join("id").join("i1").join("work"),
            mnt: root.path().join("id").join("i1").join("mnt"),
        };
        std::fs::create_dir_all(&layout.mnt).unwrap();
        let sibling = layout.base_dir.join("i2");
        std::fs::create_dir_all(&sibling).unwrap();

        cleanup_failed_create(&layout).unwrap();
        assert!(!layout.intent_dir.exists());
        assert!(sibling.exists(), "误删了别的 Intent 的目录");
    }
}
