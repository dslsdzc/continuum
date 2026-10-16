//! §308 的第一枚 `Checkpointable` 实现：`generate-broll` 与 `render`（设计 §4.5）。
//!
//! - **两枚算子**：`generate-broll`（§308 `:2048` 点名的第一项长任务「video generation」）
//!   与 `render`（§5.2 第 11 行）。§308 的「尤其」清单是**举例**（`:2045` 逐字「尤其：」），
//!   故 `render` 按同一条 SHOULD 一并实现，**不是**把它读成 §308 的穷尽清单。
//! - **与 `all_operators()` 的关系**：两枚类型各带 [`GenerateBroll::operator_id`]，其返回值
//!   **必须**与 [`crate::all_operators()`] 里那两枚 `Operator` 值的 `id` 相同。
//!   这条对应是**纯数据关系**（`Operator` 是七个字段的值），编译期核不了，
//!   由 `tests/checkpoint.rs` 的 `the_two_implementations_are_the_two_named_operators` 逐枚核；
//!   实现侧的 id 是硬编码的（`new()` 各一处——若从清单取，那条用例测的就是「它等于它自己」）。
//! - **调用点不在本块**（据实写明）：谁在什么时候调 `checkpoint()` / `restore()`，规范未给，
//!   属第 3 层的恢复路径（§311 / §312 那一组）。故 P5e 落地之后是
//!   「**有实现、有测试、无生产调用点**」（设计 §4.5 与 §13 第 1 条）。
//! - **判定顺序**（两枚实现共用私有 helper，是同一件事的单一事实源）：
//!   `checkpoint()` 先判「尚无进度」（片段与 backend 选择**都**为空）⇒ `NothingToCheckpoint`，
//!   再判「选中的 backend 是否在该算子清单枚的 `backend_candidates` 内」⇒ 不在则
//!   `UnsupportedByBackend`，否则快照即当前进度。「支不支持」的判定**不另立第二份名单**
//!   （设计 §4.5 现文）：`§5.4` 的九枚名单判的是 `determinism`，不是 checkpoint 能力。

use std::sync::Mutex;

use continuum_operator::definition::CheckpointOwner;
use continuum_operator::{
    BackendId, CheckpointError, Checkpointable, Operator, OperatorId, OperatorVersion,
};

/// §308 点名的第一项长任务（video generation）的检查点实现。
/// 尚无进度、尚未选定 backend 的初值是 [`GenerateBroll::new`]（**没有 `Default`**：
/// 零进度初值只有 `new` 一个产生点）。
#[derive(Debug)]
pub struct GenerateBroll {
    progress: Progress,
}

/// 同一条 SHOULD 下的另一枚长算子（`render`；设计 §4.5）。
/// 构造与读法同 [`GenerateBroll`]：`new` / `operator_id` / `record_completed_segment` /
/// `select_backend` / `progress`。
#[derive(Debug)]
pub struct Render {
    progress: Progress,
}

/// 检查点的负载：已完成的片段区间 ＋ 选中的 backend ＋ 身份（`CheckpointOwner`）。
/// 字段全 `pub`：判据 1／2 的「逐字段相同」要能写成穷尽解构（不带 `..`）。
///
/// **片段区间的表示**：设计措辞是「已完成的片段区间」；本实现取「已完成片段序号的序列」
/// 这一最小形态——每次 `record_completed_segment(index)` 追加一个 `index`，
/// **不去重、不排序**（区间合并/排序/去重均无判据要求；设计写明「`type Checkpoint` 的
/// 字段清单不在本设计的约束范围内」、计划写明「其余实现细节随实现走」）。
///
/// **字段为什么包在 `Mutex` 里（偏离最显然写法处）**：最显然的写法是裸 `Vec<u32>` /
/// `Option<BackendId>`；**这里不是那样，理由是一条链**：
/// （i）`Checkpointable::restore` 的接收者是 `&self`——这是 P1 冻结的形状
///（P5e Task 2 只把两处 `String` 换成 `CheckpointError`），本块不动它；
/// （ii）设计 §4.5 判据 1／2 要求「`restore` 该检查点后算子的进度与存前一致」、
///「恢复失败后算子状态与调用前逐字段相同（**半恢复比不恢复更坏**）」——即**恢复要真的能写回状态**；
/// （iii）安全 Rust 里从 `&self` 写回共享状态只有内部可变性一条路，而 `type Checkpoint`
/// 还带 `Send + Sync` 约束（`trait Checkpointable` 的关联类型），`RefCell` 不是 `Sync`，故取 `Mutex`。
/// **代价**：快照的字面是「锁住的容器」而不是纯值；**收益**：判据 1／2 与「把写回删掉 ⇒ 红」
/// 都有可观察意义，且**零 `unsafe`**。**另一条路**（把 `restore` 改成 `&mut self`）要动
/// P1 冻结的 trait 形状，不在本块范围。
/// **`owner` 不包容器**：身份不参与写回（只参与校验），且它是构造时固定的值。
#[derive(Debug)]
pub struct Progress {
    pub owner: CheckpointOwner,
    pub completed_segments: Mutex<Vec<u32>>,
    pub selected_backend: Mutex<Option<BackendId>>,
}

// 手写 `Clone` 而不 derive：`CheckpointOwner`（`continuum-operator` 的既有类型，处于第 3 层）
// 只有 `Debug`，本块不动它；两个 `Mutex` 字段取锁内值的克隆（`Mutex<T>: Clone where T: Clone` 同效）。
impl Clone for Progress {
    fn clone(&self) -> Self {
        Progress {
            owner: clone_owner(&self.owner),
            completed_segments: Mutex::new(
                self.completed_segments
                    .lock()
                    .expect("Progress 的锁不应中毒")
                    .clone(),
            ),
            selected_backend: Mutex::new(
                self.selected_backend
                    .lock()
                    .expect("Progress 的锁不应中毒")
                    .clone(),
            ),
        }
    }
}

/// `CheckpointOwner` 的逐字段克隆（它没有 `Clone`——`continuum-operator` 的既有类型，本块不动它）。
fn clone_owner(owner: &CheckpointOwner) -> CheckpointOwner {
    CheckpointOwner {
        id: owner.id.clone(),
        version: owner.version,
    }
}

impl Progress {
    /// 零进度、未选 backend 的初值。**身份在这里成形**：
    /// id 是 `new()` 里硬编码的那两个串（与清单的对应由用例逐枚核）；
    /// version 取 §5.1 的不变量「一律为 1」（清单侧由 `all_operators()` 的既有用例钉，
    /// 本侧由 `tests/checkpoint.rs` 的 (p) 与 (q) 断言钉）。
    fn empty_of(id: &str) -> Self {
        Progress {
            owner: CheckpointOwner {
                id: OperatorId::new(id),
                version: OperatorVersion::new(1),
            },
            completed_segments: Mutex::new(Vec::new()),
            selected_backend: Mutex::new(None),
        }
    }
}

impl GenerateBroll {
    /// 尚无进度、尚未选定 backend 的初值。
    pub fn new() -> Self {
        Self {
            progress: Progress::empty_of("generate-broll"),
        }
    }

    /// 与该类型对应的那一枚算子的 id（与 `all_operators()` 的关系见模块文档）。
    pub fn operator_id(&self) -> OperatorId {
        self.progress.owner.id.clone()
    }

    /// 推进进度：记下第 `index` 个片段已完成（判据 1 的「有进度」由它造出）。
    /// 每个 `index` 一次记录；**不去重、不排序**（见 [`Progress`] 的说明）。
    pub fn record_completed_segment(&mut self, index: u32) {
        self.progress
            .completed_segments
            .get_mut()
            .expect("Progress 的锁不应中毒（&mut 下无并发持有者）")
            .push(index);
    }

    /// 指定本次要用的 backend（判据 3 的两侧由它造出）。覆盖上一次的选择；
    /// `new()` 起的初值是「尚未选定」（`None`）。
    pub fn select_backend(&mut self, backend: BackendId) {
        *self
            .progress
            .selected_backend
            .get_mut()
            .expect("Progress 的锁不应中毒（&mut 下无并发持有者）") = Some(backend);
    }

    /// 读回当前进度（判据 1／2 的「逐字段相同」以它取存前的值）。
    pub fn progress(&self) -> &Progress {
        &self.progress
    }
}

impl Render {
    /// 尚无进度、尚未选定 backend 的初值。
    pub fn new() -> Self {
        Self {
            progress: Progress::empty_of("render"),
        }
    }

    /// 与该类型对应的那一枚算子的 id（与 `all_operators()` 的关系见模块文档）。
    pub fn operator_id(&self) -> OperatorId {
        self.progress.owner.id.clone()
    }

    /// 推进进度：记下第 `index` 个片段已完成（判据 1 的「有进度」由它造出）。
    /// 每个 `index` 一次记录；**不去重、不排序**（见 [`Progress`] 的说明）。
    pub fn record_completed_segment(&mut self, index: u32) {
        self.progress
            .completed_segments
            .get_mut()
            .expect("Progress 的锁不应中毒（&mut 下无并发持有者）")
            .push(index);
    }

    /// 指定本次要用的 backend（判据 3 的两侧由它造出）。覆盖上一次的选择；
    /// `new()` 起的初值是「尚未选定」（`None`）。
    pub fn select_backend(&mut self, backend: BackendId) {
        *self
            .progress
            .selected_backend
            .get_mut()
            .expect("Progress 的锁不应中毒（&mut 下无并发持有者）") = Some(backend);
    }

    /// 读回当前进度（判据 1／2 的「逐字段相同」以它取存前的值）。
    pub fn progress(&self) -> &Progress {
        &self.progress
    }
}

/// 两枚实现共用的 `checkpoint()` 判定——**同一件事的单一事实源**。
/// （两枚 impl 各写一份的话，「变异只落在未被 (q) 用到的那一枚上」的失效形态就有了空间；
/// 共享 helper 让它**在结构上不可能**。）
fn take_checkpoint(state: &Progress) -> Result<Progress, CheckpointError> {
    {
        let completed = state
            .completed_segments
            .lock()
            .expect("Progress 的锁不应中毒");
        let selected = state
            .selected_backend
            .lock()
            .expect("Progress 的锁不应中毒");

        // 「尚无进度」＝片段与 backend 选择**都**为空（`new()` 的初值形态）。
        if completed.is_empty() && selected.is_none() {
            return Err(CheckpointError::NothingToCheckpoint {
                owner: clone_owner(&state.owner),
            });
        }

        // 「选中的 backend 支不支持」＝它是否在该算子清单枚的 `backend_candidates` 内；
        // 未选定（`None`）不是错误——判据 1 的正例就不选 backend。
        if let Some(backend) = selected.as_ref() {
            let declared = declared_operator(state.owner.id.as_str());
            if !declared.backend_candidates.contains(backend) {
                return Err(CheckpointError::UnsupportedByBackend {
                    backend: backend.clone(),
                });
            }
        }
    }
    // guards 已在本块末尾释放，`clone` 不会重入同一把锁。
    Ok(state.clone())
}

/// 两枚实现共用的 `restore()` 判定——**校验先行、失败零写入**。
/// 「半恢复比不恢复更坏」（设计 §4.5 判据 2）：身份不符时在**任何写入之前**返回 `Err`；
/// 身份通过后把检查点的进度写回（`completed_segments` 与 `selected_backend` 两个字段；
/// 恢复的真实用途在 §311 / §312 的恢复路径——本块无生产调用点，见模块文档）。
fn apply_restore(state: &Progress, checkpoint: &Progress) -> Result<(), CheckpointError> {
    if checkpoint.owner.id != state.owner.id || checkpoint.owner.version != state.owner.version {
        return Err(CheckpointError::RestoreRejected {
            expected: clone_owner(&state.owner),
            found: clone_owner(&checkpoint.owner),
        });
    }

    // 先取快照（锁在语句末释放），再写回——同一线程不会重入同一把锁。
    let completed = checkpoint
        .completed_segments
        .lock()
        .expect("Progress 的锁不应中毒")
        .clone();
    let selected = checkpoint
        .selected_backend
        .lock()
        .expect("Progress 的锁不应中毒")
        .clone();

    *state
        .completed_segments
        .lock()
        .expect("Progress 的锁不应中毒") = completed;
    *state
        .selected_backend
        .lock()
        .expect("Progress 的锁不应中毒") = selected;
    Ok(())
}

/// 从清单取一枚算子（检查点判定要读它的 `backend_candidates`）。
/// **取不到是编程错误**（实现的 `operator_id` 与清单的对应被用例钉住），
/// 故以 `panic!` 作哨兵——三臂里没有「清单少了一枚算子」这种运行期判定的位置。
fn declared_operator(id: &str) -> Operator {
    crate::all_operators()
        .into_iter()
        .find(|op| op.id.as_str() == id)
        .unwrap_or_else(|| {
            panic!(
                "清单 all_operators() 里应有算子 {id}；\
                 每枚实现的 operator_id 与清单的对应由 tests/checkpoint.rs 逐枚核"
            )
        })
}

impl Checkpointable for GenerateBroll {
    type Checkpoint = Progress;

    fn checkpoint(&self) -> Result<Progress, CheckpointError> {
        take_checkpoint(&self.progress)
    }

    fn restore(&self, checkpoint: &Progress) -> Result<(), CheckpointError> {
        apply_restore(&self.progress, checkpoint)
    }
}

impl Checkpointable for Render {
    type Checkpoint = Progress;

    fn checkpoint(&self) -> Result<Progress, CheckpointError> {
        take_checkpoint(&self.progress)
    }

    fn restore(&self, checkpoint: &Progress) -> Result<(), CheckpointError> {
        apply_restore(&self.progress, checkpoint)
    }
}
