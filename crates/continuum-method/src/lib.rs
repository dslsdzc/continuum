//! `continuum-method`：P5b 的执行方法库（设计 §六）——「这一类任务怎样做得
//! 可靠」的目录索引：四个领域各一份方法目录，条目是**索引**（id ＋ 实现它的
//! 算子 id 文本，设计 §4.3）。规划 §一：「把『这一类任务怎样做得可靠』从
//! ADFIR 的图结构里分出来」。
//!
//! **Task 1 落骨架**：本 task 导出 [`MethodId`] 与 [`MethodError`]；
//! `mod id; mod error;` 按规划的文件结构逐 task 增量追加。
//!
//! # 依赖面：workspace 内为空集（设计 §3.2／§9）
//!
//! 本 crate **不 `use` 任何其它 workspace 成员**：《工程》§8.3
//! （`docs/02-工程.md:526`）把本组件的依赖面写死为「无」
//! （`Execution Method Library ← 无`）。兑现是两处登记——根 `Cargo.toml` 的
//! `[workspace] members` 加一行、`crates/continuum-runtime/tests/dependency_direction.rs`
//! 的 `ALLOWED` 加 `("continuum-method", &[])`：**空数组是硬断言**，任何**直接的**
//! 反向边都会让 `every_crate_depends_only_on_its_allowed_set` 变红（那张表逐对
//! `assert_eq!`）。
//!
//! **这张照片只覆盖依赖边**：`MethodEntry.realized_by` 的文本内容编译期无从核
//! （那在 `Vec<String>` 里，设计 §3.2 末三条）——本 crate 也看不到
//! `operator` 注册表，文本的核对只能落在消费块。外部依赖只有 `thiserror`
//! （[`MethodError`] 上派生 `std::error::Error`）。

mod error;
mod id;

pub use error::MethodError;
pub use id::MethodId;
