//! P7C Resource Arbiter：独立于模型的资源仲裁决策核。
//! workspace 内零依赖；不读 OS、不取时钟、不调模型。

mod arbiter;
mod declaration;
mod policy;
mod pressure;
// Task 3 提交点的 dead_code 窗口：默认构建里 classify／PriorityTier 还没有调用点
// （assembly_view 只在 assembly 特性下编译，direct 到 Task 4 才接入）。
// Task 4 的 direct 接入后删除本豁免。
#[allow(dead_code)]
mod priority;
mod summary;
mod workload;

#[cfg(feature = "assembly")]
pub mod assembly;

pub use arbiter::ResourceArbiter;
pub use declaration::WorkloadDeclaration;
pub use policy::{ArbiterPolicy, PolicyError};
pub use pressure::{ResourceObservation, ResourcePressure};
pub use summary::ResourceSummary;
pub use workload::WorkloadClass;
