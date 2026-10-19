//! P7C Resource Arbiter：独立于模型的资源仲裁决策核。
//! workspace 内零依赖；不读 OS、不取时钟、不调模型。

mod action;
mod arbiter;
mod declaration;
mod policy;
mod pressure;
mod priority;
mod state;
mod summary;
mod workload;

#[cfg(feature = "assembly")]
pub mod assembly;

pub use action::ResourceAction;
pub use arbiter::{ResourceArbiter, ResourceDecision};
pub use declaration::WorkloadDeclaration;
pub use policy::{ArbiterPolicy, PolicyError};
pub use pressure::{ResourceObservation, ResourcePressure};
pub use state::{SuspensionError, SuspensionPhase, SuspensionState};
pub use summary::ResourceSummary;
pub use workload::WorkloadClass;
