//! P7C Resource Arbiter：独立于模型的资源仲裁决策核。
//! workspace 内零依赖；不读 OS、不取时钟、不调模型。

mod arbiter;
mod policy;
mod pressure;
mod summary;
mod workload;

pub use arbiter::ResourceArbiter;
pub use policy::{ArbiterPolicy, PolicyError};
pub use pressure::{ResourceObservation, ResourcePressure};
pub use summary::ResourceSummary;
pub use workload::WorkloadClass;
