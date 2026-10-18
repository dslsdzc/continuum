//! P7B Intent Scheduler：§302 的决策面、§303 的零逻辑转调与 §304 的具名资源端口。
//! 本 crate 不持久化、不迁移 Intent 状态、不实现 Resource Scheduler。

pub mod decision;
pub mod graph;
pub mod inputs;

pub use decision::{
    BaselineIntentPolicy, DecisionBasis, IntentDecision, IntentDecisionError,
    IntentSelectionPolicy, IntentScheduler,
};
pub use graph::runnable_nodes;
pub use inputs::{
    BackgroundSuitability, Blocking, Deadline, PrioritySignal, SchedulerInputs, UserFocus,
};
