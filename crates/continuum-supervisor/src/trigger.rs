//! §171 的合成触发点：端口与三个转发（设计 §4.8）。

use continuum_memory::{
    MemoryError, SynthesisJobId, SynthesisProposal, SynthesisReport, SynthesisRequest,
};

use crate::error::SupervisorError;

/// §171 三个记忆合成调用的端口。P7D 定义 trait，装配方实现并持有事务句柄。
pub trait SynthesisIo {
    fn begin_synthesis(&self, request: &SynthesisRequest) -> Result<(), MemoryError>;

    fn apply_synthesis(
        &self,
        job: &SynthesisJobId,
        proposal: &SynthesisProposal,
        occurred_at: i64,
    ) -> Result<SynthesisReport, MemoryError>;

    fn fail_synthesis(
        &self,
        job: &SynthesisJobId,
        reason: &str,
        occurred_at: i64,
    ) -> Result<(), MemoryError>;
}

/// §171 的触发面。无状态；不持阈值、不取时钟、不做空闲判定、不重试。
#[derive(Debug, Default, Clone, Copy)]
pub struct SynthesisTriggerPoint;

impl SynthesisTriggerPoint {
    pub fn begin_synthesis(
        &self,
        io: &dyn SynthesisIo,
        request: &SynthesisRequest,
    ) -> Result<(), SupervisorError> {
        io.begin_synthesis(request).map_err(SupervisorError::Memory)
    }

    pub fn apply_synthesis(
        &self,
        io: &dyn SynthesisIo,
        job: &SynthesisJobId,
        proposal: &SynthesisProposal,
        occurred_at: i64,
    ) -> Result<SynthesisReport, SupervisorError> {
        io.apply_synthesis(job, proposal, occurred_at)
            .map_err(SupervisorError::Memory)
    }

    pub fn fail_synthesis(
        &self,
        io: &dyn SynthesisIo,
        job: &SynthesisJobId,
        reason: &str,
        occurred_at: i64,
    ) -> Result<(), SupervisorError> {
        io.fail_synthesis(job, reason, occurred_at)
            .map_err(SupervisorError::Memory)
    }
}
