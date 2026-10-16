//! Operator 定义（§244）。

use continuum_artifact::ArtifactType;
use serde::{Deserialize, Serialize};

/// ENG-002 的判定依据：只有 `Deterministic` 的 Operator 参与按输入哈希复用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Determinism {
    Deterministic,
    NonDeterministic,
}

/// §307 的判定依据：`NonIdempotent` 不进入自动重试。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    Pure,
    Idempotent,
    NonIdempotent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorId(String);

impl OperatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `OperatorError` 的格式串以 `{id}` 引用 OperatorId，
// thiserror 要求该字段实现 Display。
impl std::fmt::Display for OperatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorVersion(u32);

impl OperatorVersion {
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

// 同 OperatorId：`OperatorError` 的格式串以 `{version}` 引用该字段，
// 缺 Display 时错误文案会渲染成 `OperatorVersion(1)`。
impl std::fmt::Display for OperatorVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendId(String);

impl BackendId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// 同 OperatorId：§4.3 与 §5.1 两个错误类型的格式串都以 `{backend}` 引用本字段；
// 缺 Display 时错误文案会渲染成 `BackendId("nvenc")`。
impl std::fmt::Display for BackendId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operator {
    pub id: OperatorId,
    pub version: OperatorVersion,
    pub input_schema: Vec<ArtifactType>,
    pub output_schema: Vec<ArtifactType>,
    pub determinism: Determinism,
    pub side_effect_class: SideEffectClass,
    /// §245：具体后端由 Router 决定，本层只保存候选。
    pub backend_candidates: Vec<BackendId>,
}

/// §308 要求长任务 Operator 支持 checkpoint。接口在 P1，错误类型与第一枚实现在 `continuum-media`。
/// 未实现该接口的 Operator 不参与检查点。
pub trait Checkpointable {
    type Checkpoint: Send + Sync;

    fn checkpoint(&self) -> Result<Self::Checkpoint, CheckpointError>;
    fn restore(&self, checkpoint: &Self::Checkpoint) -> Result<(), CheckpointError>;
}

/// 检查点对产出它的算子实例的引用（设计 §4.3）：`id` 与 `version` 两个字段。
///
/// **不是 `OperatorRef` 的副本，也不与之合并**：两者字段逐字相同（`OperatorRef` 在
/// `continuum-graph`），合并要么让本 crate 依赖 `continuum-graph`，即 2-环；要么把
/// `OperatorRef` 下移到本 crate，那要动 P1 的图 API 与它的 `pub use`。语义也不同：
/// `OperatorRef` 是**图上节点**对算子的引用，本型是**检查点**对产出它的算子实例的引用。
#[derive(Debug)]
pub struct CheckpointOwner {
    pub id: OperatorId,
    pub version: OperatorVersion,
}

// `CheckpointError` 的三个格式串以 `{owner}` / `{found}` / `{expected}` 引用本型，
// thiserror 要求 Display；该枚举自身 `derive(Debug)` 也要求本型实现 Debug。
impl std::fmt::Display for CheckpointOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

/// §308 的检查点接口的错误类型。
///
/// 它**不是**第二份 `OperatorError`：那两枚讲的是「注册表里有没有这个算子」，
/// 本型讲的是「这一次检查点/恢复为什么不成」。两者的调用方、恢复策略都不同
///（§309 的 failure type 分类要能分开它们）。
///
/// **这里不是 `continuum-media`：因为那会让第 3 层依赖第 8 层，即 2-环。**
///
/// 为什么是这三臂、不是两臂、不是四臂：
/// - `UnsupportedByBackend` 的存在理由是 §245 的多 backend。若把它删掉，则「后端不支持」
///   只能表现为 `Ok(())`——即**假装检查点成功**，而这在崩溃恢复时会把「没存」读成「存了」。
///   这是三臂里唯一一条 fail-open 反面的臂。
/// - `NothingToCheckpoint` 与 `RestoreRejected` 是**两个不同的处置**：前者是「现在没有东西
///   可存，稍后再来」（可重试），后者是「这份检查点不是我的，重试无用」（不可重试）。
///   把它们合成一臂，§309 的 failure type 分类就没有落点。
/// - **不给 `operator.determinism` 之类的取值留臂**：那属 `OperatorError` 与注册期检查
///   （§5.4），不属于一次检查点调用。**不给 IO 臂**：`checkpoint()` 的返回类型是
///   `Self::Checkpoint`，落盘是调用方的事；§308 只规定了「SHOULD 支持 checkpoint」，
///   没给存储。
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    /// §308 是 **SHOULD**，而 §245（`docs/spec/05-normative.md:756`）允同一个 Operator 有多个 backend
    /// ——故「不支持」是一个合法答案，且它的主语是**选中的那个 backend**，不是这个算子。
    #[error("backend {backend} 不支持检查点（§308）")]
    UnsupportedByBackend { backend: BackendId },

    /// 尚无状态可保存：算子还没产出任何可检查点的中间状态。
    #[error("算子 {owner} 尚无可检查点的状态")]
    NothingToCheckpoint { owner: CheckpointOwner },

    /// 恢复被拒：该检查点不属于本算子实例。
    #[error("检查点属于 {found}，本实例是 {expected}，恢复被拒")]
    RestoreRejected {
        expected: CheckpointOwner,
        found: CheckpointOwner,
    },
}
