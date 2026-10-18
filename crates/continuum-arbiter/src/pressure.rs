use crate::policy::ArbiterPolicy;
use crate::workload::WorkloadClass;

/// §13 的压力等级。**规范只给示例 HIGH**；四档是本设计自定（设计 §2.4 第 1 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePressure {
    Low,
    Moderate,
    High,
    Critical,
}

impl ResourcePressure {
    /// 逐字的大写文本；解析按同一张表精确匹配。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Moderate => "MODERATE",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }

    /// 精确大写解析；未知文本（含小写与 `INTERACTIVE` 一类笔误）返回 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "LOW" => Some(Self::Low),
            "MODERATE" => Some(Self::Moderate),
            "HIGH" => Some(Self::High),
            "CRITICAL" => Some(Self::Critical),
            _ => None,
        }
    }
}

/// 六项**聚合**读数＋已分类的负载类别。**不是**每进程读数；没有进程名／PID。
/// **聚合 CPU／GPU 允许；逐进程 CPU／GPU 禁止**（设计 §6.1、J14）。
/// 六项单位是百分比（0–100）；`None` 表示该项不可读；`>100` 在 observe 里钳到 100。
/// `workload_class` 由本机 Resource Arbiter 组件的本地探测器分类后填入；
/// P7C 不读进程名（设计 §2.4 第 4 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceObservation {
    pub cpu: Option<u8>,
    pub ram: Option<u8>,
    pub gpu: Option<u8>,
    pub vram: Option<u8>,
    pub disk_io: Option<u8>,
    pub network: Option<u8>,
    pub workload_class: Option<WorkloadClass>,
}

/// 设计 §5.1 第 1–3 步：六项非 `None` 读数先钳到 `0..=100`，取最大值后按阈值分档。
/// 六项全 `None` 返回 `None`（调用方保持上一压力）。
pub(crate) fn raw_pressure(
    observation: &ResourceObservation,
    policy: &ArbiterPolicy,
) -> Option<ResourcePressure> {
    let max = [
        observation.cpu,
        observation.ram,
        observation.gpu,
        observation.vram,
        observation.disk_io,
        observation.network,
    ]
    .into_iter()
    .flatten()
    .map(|value| value.min(100))
    .max()?;

    Some(if max >= policy.critical_at {
        ResourcePressure::Critical
    } else if max >= policy.high_at {
        ResourcePressure::High
    } else if max >= policy.moderate_at {
        ResourcePressure::Moderate
    } else {
        ResourcePressure::Low
    })
}
