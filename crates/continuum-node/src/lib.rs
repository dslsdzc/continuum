//! 计算节点注册与节点放置（P3 子项目 E；设计
//! `docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`）。
//!
//! 本 crate 是 P3 的**资源层**，落《工程》§4.1 组件表的两行——Compute Node 注册
//! （`§287`–`§291`）与节点放置（`§291` `§243` `§93`）。
//!
//! # 走到哪为止
//!
//! Task 1 建骨架：crate 文档与 workspace 登记。
//! Task 2 补 §287 的 [`ComputeNode`] 与它的三个取值类型 [`ComputeNodeId`] / [`NodeClass`] /
//! [`NodeTrust`]（[`node`] 模块），并以 `tests/type_level.rs` ＋
//! `tests/compile_fail/compute_node_fields_are_private.rs` 钉住「crate 外写不出 `ComputeNode`
//! 的字段字面量」这条不可表达性。
//! Task 3 补 §4 的进程内注册表 [`NodeRegistry`] / [`NodeRegistryError`]（[`registry`] 模块），
//! 并以 `tests/registry.rs` 的两条**源码文本守卫**钉住「这个模块里不出现某三个名字」——
//! 守卫的判据与证明力边界写在那份测试的文件头。
//! Task 4 补 §5.4 (b) 的表 [`PlacementRules`] × [`TransferRule`]（[`placement`] 模块）
//! 与它的构造期错误 [`RulesError`]（[`error`] 模块），并以 `tests/rules.rs` 钉住
//! 「覆盖全档／缺档即拒／重复即拒」三条判据。**本 task 是 `continuum-artifact`
//! 这条边的第一个使用点**：Task 1 只在 `ALLOWED` 里登记了声明，
//! `dependency_direction.rs` 的双向断言从本 task 起才对得上号。
//! Task 5 补 §5.2–§5.3 的请求面与硬闸门 [`PlacementRequest`] / [`PlacementPolicy`] /
//! [`BaselinePlacementPolicy`] / [`place`] 与运行期错误 [`PlacementError`]，
//! 并以 `tests/placement.rs` 钉住「判据 §4.4 的正反面、放行侧、闸门不可被策略放宽、
//! 四种 class × trust 组合、两条失败路径、空集两例、请求面反侧照片」。
//! **`place` 今天只落步骤 1–3**（判重 → 过闸门 → 判空）；
//! **步骤 4（排序）与兜底档由 Task 6 落**，故 `place` 今天**不读**
//! [`PlacementPolicy::compare`]——那是刻意的拆分，不是终态。
//!
//! 本段是**逐 task 更新的进度注记**，不是对代码性质的可跑断言，故没有用例钉它
//! ——与 `continuum-model-registry` 的 `lib.rs` 同一体例：它的「走到哪为止」也由各 task 改写。
//!
//! # 边
//!
//! 设计 §7.2 给本 crate 定死的只有一条 crate 边：`continuum-artifact`（见 `Cargo.toml`）。
//! 不登记 `continuum-model-registry` 的理由（《工程》§4.3 的「节点放置 ← Router 输出」
//! 已记为阻断）写在设计 §7.2 与 `crates/continuum-runtime/tests/dependency_direction.rs`
//! 的 `ALLOWED` 条目里——**不是漏登记**。
//!
//! `trybuild` 是**外部 crate**，不进那张 `ALLOWED` 表——该表只断言 workspace 成员之间的边
//! （设计 §7.3）；它在 `Cargo.toml` 的 `[dev-dependencies]` 里，由 Task 2 登记。

pub mod error;
pub mod node;
pub mod placement;
pub mod registry;

pub use error::{PlacementError, RulesError};
pub use node::{ComputeNode, ComputeNodeId, NodeClass, NodeTrust};
pub use placement::{
    place, BaselinePlacementPolicy, PlacementPolicy, PlacementRequest, PlacementRules,
    TransferRule,
};
pub use registry::{NodeRegistry, NodeRegistryError};
