//! P7C Resource Arbiter：独立于模型的资源仲裁决策核。
//! workspace 内零依赖；不读 OS、不取时钟、不调模型。

mod pressure;
mod workload;

pub use pressure::{ResourceObservation, ResourcePressure};
pub use workload::WorkloadClass;
