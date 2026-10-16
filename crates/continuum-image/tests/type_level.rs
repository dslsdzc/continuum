//! 结构层的编译期照片（设计 §11.4 的两张；计划 Task 8）。
//!
//! 三份**命题**的不可表达性，运行期用例证明不了：
//! - 「`Point` 那一支没有 `detector` 就调不动」（§4.4 第一条）——
//!   `a_point_region_without_a_detector_does_not_compile.rs`；
//! - 「`Box` 那一支没有源图尺寸就投不出掩码」（同上）——
//!   `a_box_region_without_the_source_dims_does_not_compile.rs`；
//! - 「`Evidence` 在 crate 外造不出」（P5a 设计 §4.4 的「唯一产生点」）——
//!   `evidence_fields_are_private.rs` 与 `evidence_has_no_default.rs` **两份**。
//!
//! # 与设计 §11.4 的条数对账（设计两条、本目录落四份）
//!
//! 设计 §11.4 说「本块用**两条**」：第 1 条是「`resolve_edit_region` 少传实参」，
//! 第 2 条把 `Evidence { .. }` 与 `Evidence::default()` **并作一条**。
//! 本 task 按**一份样例只能钉一个**的纪律把它展开成**四份**：
//! - 第 1 条拆两份（两份分开的理由写在各样例的注释里：它们是**两个不同的实参**缺了，
//!   且两份的 `.stderr` 逐份不同，不许拿一个错误码套两处——计划「关于本计划的代码块」
//!   的纪律）；
//! - 第 2 条拆两份（`evidence_fields_are_private` / `evidence_has_no_default`），
//!   照 P5a 任务 10 的成例（`crates/continuum-verify/tests/compile_fail/` 的 4、5 号）
//!   与它逐字的「**不许并成一份**——两条通道的失效条件不同」；且**实跑**证明并一份
//!   必丢一条：`Evidence::default()` 的 E0599 会把 E0451 整个吞掉（任何别的错误都会
//!   把 E0451 吞掉），字面量那份连错误都不出。**故设计列两条、本目录落四份，两者
//!   不矛盾**（同 P5a 的「设计列四条、本目录落五份」）。
//!
//! **第 2 条在本块的分量比 P5e 小**（设计 §11.4 第 2 条）：本块不写域包装（§7.2），
//! 故它是本块侧**唯一**的一条证据面照片。
//!
//! # 正控制
//!
//! **不另建目录**（计划「关于本计划的代码块」一节）：本块不新建 `ui_pass/` 目录、
//! 不加 `t.pass`——正控制由普通运行期用例承担：Task 5 的四支用例
//! （`tests/resolve_region.rs`：`Box` / `Mask` / `Point` / `SemanticObject`）本身
//! 就是正控制（它们能编译即已说明正常调用路径成立）。
//! 〔订正〕计划写时该节的实测是「本仓没有一处 `t.pass`」；本 task 落地时那句已过时
//! ——`t.pass` 在 `continuum-budget` / `continuum-canonical` / `continuum-semantics`
//! 的 `tests/type_level.rs` 各有先例（`compile_pass/` 目录）。本块的决定照计划不变：
//! **不新建（`ui_pass/` 类）目录、不加 `t.pass`**。
//!
//! # 坐标
//!
//! 每份 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在预期的报错上——否则
//! 「因为拼错函数名而编译失败」也会让用例变绿。下面的通配 `*.rs` 自动收走
//! `tests/compile_fail/` 里的四份样例；**缺 `.stderr` 时 `trybuild` 报「wip」并把实际
//! 输出落进 crate 根的 `wip/` 目录**——搬进 `tests/compile_fail/` 才算接受。
//! **样例里的注释也是照片坐标**：注释改了，`.stderr` 里引的行号与列位置随之漂，
//! 两份文件一起重取。

/// 名字描述的是这组样例共同守住的东西（结构层的不可表达性），不是其中某一条的来历。
/// 与 `continuum-connector` / `continuum-capability` / `continuum-secrets` 的
/// `tests/type_level.rs` 同形。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
