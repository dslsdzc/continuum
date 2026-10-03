//! Effect Journal：记录体与状态机（设计上篇第 7 节）。
//!
//! 本 crate 目前只有类型与纯函数的状态机；落库（表定义、编码辅助函数、
//! 恢复阶段）不在本 task。

pub mod effect;

pub use effect::{Effect, EffectId, EffectState, EffectType, StateError, transition};
