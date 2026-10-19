use std::fmt;

/// §14 的四相位（`:735-743`）。**不复用** `NodeState`／`IntentStatus`。
/// `Resume` 不是第五相位：它是 `Suspended → Running` 的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionPhase {
    Running,
    Throttled,
    Checkpointing,
    Suspended,
}

impl SuspensionPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Throttled => "Throttled",
            Self::Checkpointing => "Checkpointing",
            Self::Suspended => "Suspended",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Running" => Some(Self::Running),
            "Throttled" => Some(Self::Throttled),
            "Checkpointing" => Some(Self::Checkpointing),
            "Suspended" => Some(Self::Suspended),
            _ => None,
        }
    }
}

/// 每个后台／维护任务的挂起状态机。P7D 持有；P7C 不集中管理任务表。
/// 字段私有；唯一公开构造点 `new()`；不实现 `Default`／`Deserialize`。
/// `last_transition_millis` 在每一次**被接受的相位转移**时更新（含进入 `Suspended`）；
/// `Resume` 的 cooldown 从它起算，不从压力时钟起算。
#[derive(Debug, PartialEq, Eq)]
pub struct SuspensionState {
    phase: SuspensionPhase,
    attempts: u32,
    last_transition_millis: u64,
    checkpoint_deadline_millis: Option<u64>,
}

impl SuspensionState {
    pub fn new() -> Self {
        Self {
            phase: SuspensionPhase::Running,
            attempts: 0,
            last_transition_millis: 0,
            checkpoint_deadline_millis: None,
        }
    }

    pub fn phase(&self) -> SuspensionPhase {
        self.phase
    }

    /// checkpoint 的截止时刻；`Some` 只在 `Checkpointing`。P7D 用它安排定时器。
    pub fn checkpoint_deadline_millis(&self) -> Option<u64> {
        self.checkpoint_deadline_millis
    }

    /// 只允许从 `Checkpointing` 调用。截止前成功 → `Suspended`（`Ok`）；
    /// 错相位返回 `NotCheckpointing { phase }`，相位不变。
    pub fn checkpoint_succeeded(
        &mut self,
        now_millis: u64,
    ) -> Result<SuspensionPhase, SuspensionError> {
        if self.phase != SuspensionPhase::Checkpointing {
            return Err(SuspensionError::NotCheckpointing { phase: self.phase });
        }
        self.phase = SuspensionPhase::Suspended;
        self.checkpoint_deadline_millis = None;
        self.last_transition_millis = now_millis;
        Ok(SuspensionPhase::Suspended)
    }

    /// 只允许从 `Checkpointing` 调用。相位回 `Throttled`，返回 `CheckpointFailed`；
    /// 错相位返回 `NotCheckpointing { phase }`。**总是返回错误（没有 Ok 路径）**。
    pub fn checkpoint_failed(&mut self, now_millis: u64) -> SuspensionError {
        if self.phase != SuspensionPhase::Checkpointing {
            return SuspensionError::NotCheckpointing { phase: self.phase };
        }
        self.phase = SuspensionPhase::Throttled;
        self.checkpoint_deadline_millis = None;
        self.last_transition_millis = now_millis;
        SuspensionError::CheckpointFailed
    }

    /// 定时器钩子；只允许从 `Checkpointing` 调用。相位回 `Throttled`，
    /// 返回 `CheckpointTimedOut`；错相位返回 `NotCheckpointing { phase }`。
    /// **总是返回错误（没有 Ok 路径）**。
    pub fn checkpoint_timed_out(&mut self, now_millis: u64) -> SuspensionError {
        if self.phase != SuspensionPhase::Checkpointing {
            return SuspensionError::NotCheckpointing { phase: self.phase };
        }
        self.phase = SuspensionPhase::Throttled;
        self.checkpoint_deadline_millis = None;
        self.last_transition_millis = now_millis;
        SuspensionError::CheckpointTimedOut
    }

    /// 重试预算；Task 5 起由 `direct` 的上限判断读取。不公开。
    #[allow(dead_code)] // Task 5 接入上限判断后删除本豁免。
    pub(crate) fn attempts(&self) -> u32 {
        self.attempts
    }

    /// `Resume` 的 cooldown 起算点。不公开。
    pub(crate) fn last_transition_millis(&self) -> u64 {
        self.last_transition_millis
    }

    /// 仅从 `Running`／`Throttled` 调用；进入两阶段挂起的第一阶段。
    pub(crate) fn enter_checkpointing(&mut self, now_millis: u64, timeout_millis: u64) {
        self.phase = SuspensionPhase::Checkpointing;
        self.attempts += 1;
        self.checkpoint_deadline_millis = Some(now_millis.saturating_add(timeout_millis));
        self.last_transition_millis = now_millis;
    }

    /// 仅从 `Running` 调用。
    pub(crate) fn throttle(&mut self, now_millis: u64) {
        self.phase = SuspensionPhase::Throttled;
        self.checkpoint_deadline_millis = None;
        self.last_transition_millis = now_millis;
    }

    /// 仅从 `Throttled` 调用。
    pub(crate) fn run(&mut self, now_millis: u64) {
        self.phase = SuspensionPhase::Running;
        self.checkpoint_deadline_millis = None;
        self.last_transition_millis = now_millis;
    }

    /// 仅从 `Suspended` 调用。
    pub(crate) fn resume(&mut self, now_millis: u64) {
        self.phase = SuspensionPhase::Running;
        self.checkpoint_deadline_millis = None;
        self.last_transition_millis = now_millis;
    }
}

/// checkpoint 失败／超时／错相位的错误。**不是** `PolicyError`；
/// 也不是 P7D 的 checkpoint 错误。`CheckpointFailed`／`CheckpointTimedOut`
/// 返回时，相位已经回到 `Throttled`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionError {
    CheckpointFailed,
    CheckpointTimedOut,
    NotCheckpointing { phase: SuspensionPhase },
}

impl fmt::Display for SuspensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheckpointFailed => f.write_str("checkpoint 失败，相位已回到 Throttled"),
            Self::CheckpointTimedOut => f.write_str("checkpoint 超时，相位已回到 Throttled"),
            Self::NotCheckpointing { phase } => {
                write!(f, "相位不是 Checkpointing，而是 {}", phase.as_str())
            }
        }
    }
}

impl std::error::Error for SuspensionError {}
