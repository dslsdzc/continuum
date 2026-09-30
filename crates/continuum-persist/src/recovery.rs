//! §319 启动恢复的五个阶段与钩子注册表。
//!
//! 每个阶段在独立事务内执行，阶段之间不共享事务。
//! 某阶段失败时该阶段的写入回滚，其后的阶段不执行。

use crate::{Db, PersistError, Tx};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryPhase {
    LoadDurableState,
    ReconcileIncompleteEffects,
    ReconcileRunningNodes,
    MarkLostExecutions,
    ResumeEligibleTasks,
}

impl RecoveryPhase {
    /// 顺序即 §319 的执行顺序，不得改动。
    pub const ALL: [RecoveryPhase; 5] = [
        RecoveryPhase::LoadDurableState,
        RecoveryPhase::ReconcileIncompleteEffects,
        RecoveryPhase::ReconcileRunningNodes,
        RecoveryPhase::MarkLostExecutions,
        RecoveryPhase::ResumeEligibleTasks,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryPhase::LoadDurableState => "load durable state",
            RecoveryPhase::ReconcileIncompleteEffects => "reconcile incomplete effects",
            RecoveryPhase::ReconcileRunningNodes => "reconcile running nodes",
            RecoveryPhase::MarkLostExecutions => "mark lost executions",
            RecoveryPhase::ResumeEligibleTasks => "resume eligible tasks",
        }
    }
}

/// 各层注册的恢复步骤。`run` 在一个阶段事务内被调用。
pub trait RecoveryHook: Send + Sync {
    fn phase(&self) -> RecoveryPhase;
    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError>;
}

#[derive(Default)]
pub struct RecoveryRegistry {
    hooks: Vec<Box<dyn RecoveryHook>>,
}

impl RecoveryRegistry {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Box<dyn RecoveryHook>) {
        self.hooks.push(hook);
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseReport {
    pub phase: RecoveryPhase,
    pub hooks_run: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub phases: Vec<PhaseReport>,
    /// 事件日志解码时被跳过的记录数（P0 设计第 4.1 节）。
    pub skipped_records: usize,
}

/// 按 §319 顺序执行五个阶段。报告覆盖全部五个阶段，
/// 无钩子的阶段也出现在报告中，`hooks_run` 为 0。
///
/// 第一阶段的内建工作由本函数执行，不受注册表控制：解码事件日志、
/// 校验审计链。任一项致命时返回错误，不进入后续四个阶段。
pub fn run_recovery(db: &Db, registry: &RecoveryRegistry) -> Result<RecoveryReport, PersistError> {
    let skipped_records = {
        let tx = db.begin()?;
        let scan = tx.scan_event_log()?;
        tx.verify_audit_chain()?;
        tx.commit()?;
        scan.skipped_records
    };

    let mut phases = Vec::with_capacity(RecoveryPhase::ALL.len());
    for phase in RecoveryPhase::ALL {
        let matched: Vec<&Box<dyn RecoveryHook>> = registry
            .hooks
            .iter()
            .filter(|h| h.phase() == phase)
            .collect();
        if matched.is_empty() {
            phases.push(PhaseReport {
                phase,
                hooks_run: 0,
            });
            continue;
        }
        let tx = db.begin()?;
        for hook in &matched {
            hook.run(&tx)?;
        }
        tx.commit()?;
        phases.push(PhaseReport {
            phase,
            hooks_run: matched.len(),
        });
    }
    Ok(RecoveryReport {
        phases,
        skipped_records,
    })
}
