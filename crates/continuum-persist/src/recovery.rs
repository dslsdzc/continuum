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
    /// 该阶段是否执行了本子项目内置的工作（仅第一阶段为 true）。
    /// 与 `hooks_run` 分开：第一阶段的内建工作不受注册表控制。
    pub builtin_work: bool,
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
/// 第一阶段的内建工作由本函数执行，不受注册表控制：
/// 应用迁移 → 建链并校验缺口 → 解码事件日志 → 校验审计链。
/// 迁移放在此处而非交给调用方：本函数可以被直接调用，
/// 不能依赖调用方已经迁移过。
pub fn run_recovery(db: &Db, registry: &RecoveryRegistry) -> Result<RecoveryReport, PersistError> {
    let skipped_records = {
        // migrate 幂等：重复调用返回 0，故与 main.rs 的调用不冲突。
        db.migrate()?;

        let chain = continuum_events::default_chain();
        chain
            .validate_contiguous()
            .map_err(|e| PersistError::Database(e.to_string()))?;

        let tx = db.begin()?;
        let scan = tx.scan_event_log(&chain)?;
        tx.verify_audit_chain()?;
        tx.commit()?;
        scan.skipped_records
    };

    let builtin = |phase: RecoveryPhase| phase == RecoveryPhase::LoadDurableState;

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
                builtin_work: builtin(phase),
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
            builtin_work: builtin(phase),
        });
    }
    Ok(RecoveryReport {
        phases,
        skipped_records,
    })
}
