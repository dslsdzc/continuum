//! 结构层的照片（设计 §9.4 第 1 条）：**本 crate 里造不出 `Evidence`**——
//! 字段全部私有、且没有 `Default`（P5a 设计 §4.4 的「唯一产生点」是结构事实）。
//! 这是那条结构事实在本块的**使用侧**照片，由两份 `compile_fail` 样例钉住：
//! `Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现）各自编不过。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明「我没这么
//! 构造过」，证明不了「做不到」——字段若真被开成 `pub`、或真出现一个 `Default` 实现，
//! 一份运行期用例照样绿。故本 task 的观测方式是「样例从编译失败变成编译通过」或
//! 「编译不过」，不是「用例变红」：`.rs` 写的是**被挡住的那次构造**，**必须编不过**，
//! 且必须**因为预期的那条错**编不过。故用 `trybuild` 的编译失败用例：每份样例配一份
//! 同名 `.stderr`（按文件主干配对），由 `trybuild` **逐字比对**——否则「因为拼错
//! 函数名而编译失败」也会让用例变绿，那样例钉的就是拼写，不是结构。
//!
//! **两份的 `.stderr` 内容与错误码一律取自实跑**——逐份各不相同，不许拿一个值套两处。
//! **样例里的注释也是照片坐标**：注释写着「应编译失败：…」时，那句说明与样例实际钉的
//! 东西必须一致；`.stderr` 一旦接受，就不再改那份 `.rs` 的注释（`trybuild` 按文件主干
//! 配对且**不规范化行号**，改文件头会推移坐标——要改就重跑并一起提交新 `.stderr`）。
//!
//! # 红条件（两条，**都是移除档**；红色形态均取自实测）
//!
//! 1. **锚点在 P5a 的 crate 里**（`crates/continuum-verify/src/evidence.rs` 的
//!    `Evidence` 定义与它的派生）。两种触发形态各有一条实测的红色报错：
//!    - **字段开成可写**（实测：第 0 个字段单独改 `pub`）⇒ 样例的编译失败清单随之
//!      改变（少一项私有字段的报错）、与 `.stderr` 不再相符 ⇒ `trybuild` 报
//!      **mismatch**（EXPECTED/ACTUAL 对比；实测仅对应那份样例红）；
//!    - **给 `Evidence` 加一个 `Default` 实现**（实测）⇒ `evidence_has_no_default.rs`
//!      编译通过 ⇒ `trybuild` 报 **「Expected test case to fail to compile, but it
//!      succeeded.」**（实测文案）⇒ 本用例红。
//!    **两种都不是「变异导致编译失败」**（纪律 1(c) 说的是两回事）：那里说的是一种
//!    **无法判定**的形态——变异把被测 crate 本身弄到编译不过，于是用例连跑都跑不起来；
//!    **这里红的是本用例自己的比较结果**，锚点 crate 正常编译（实测日志里
//!    `Compiling continuum-verify … Finished` 与失败报告同在）。
//! 2. **锚点在本 crate 侧**：删掉 `Cargo.toml` 的 dev 依赖 `continuum-verify` ⇒
//!    样例报「未解析导入」（实测 `E0432: unresolved import`）、与旧的 `.stderr`
//!    不符 ⇒ **mismatch、本用例红**（实测：两份样例各报一处，`2 of 2 tests failed`；
//!    依赖 `trybuild` 也把 dev 依赖算进样例——实测 trybuild-1.0.121/src/run.rs:239）。
//!
//! # 本条的限度（设计要求，设计 §9.4 第 1 条末）
//!
//! 本条**证不到**「产出只能经 `Evidence::from_tool_result`」——那个构造点是 `pub`，
//! 构造由**宿主**的执行代码直接调它完成，**调用点不在本 crate 里**。故本条是本块在
//! `continuum-verify` 上的**唯一使用点**，也是那条边为 **dev 边**的理由。
//!
//! # 正控制（`tests/ui_pass/evidence_is_reachable.rs`）
//!
//! 只取构造点 `Evidence::from_tool_result` 的**函数项**（不构造值、不传八个实参），
//! 证明这条 dev 边在本 crate 的编译环境里**确实接上了**。**用途只有一个**：把
//! 「因为依赖没接上而编译失败」与「因为字段私有而编译失败」分开——**没有它，两份
//! `compile_fail` 样例的红可能是任何一种红**；它也是「用 `TRYBUILD=overwrite` 接受
//! 一份被偷换的 `.stderr`」那条假绿通道的锁（`.stderr` 若被换成「找不到 crate」的
//! 版本、样例转绿，正控会先红）。**正控与失败样例同处一次运行**（见下面
//! `type_level_guarantees_hold`）：分开写成两个用例，会让「正控制没跑而失败样例照红」
//! 的形态可乘——两半合起来才算钉住（评审 I-1；P5c 同形先例）。
//!
//! **判别力实测（评审 I-1 的 fix 轮，一轮；日志 `.tmp/task6-fix1-no-devdep.log`）**：
//! 临时删掉 `Cargo.toml` 的 `continuum-verify` dev 依赖 ⇒ **三份样例同红**——两份
//! `compile_fail` 报 `mismatch`（实际输出是 `E0432` 找不到 `continuum_verify`，
//! 与照片不符）、**正控制报 `[should pass] ... error`**——「依赖没接上」这一因被
//! 单独点名出来（形态与 P5c 的 `t6-03d` 同款；`3 of 3 tests failed`）。
//!
//! # 坐标
//!
//! **样例不引用 `continuum_research`**——它们钉的是 P5a 那一层的结构事实，经本 crate
//! 的测试面（即本文件的 `trybuild` 驱动）拍下来。下面的通配 `*.rs` 自动收走
//! `tests/compile_fail/` 里的两份样例；缺 `.stderr` 时 `trybuild` 报「wip」并把实际输出
//! 落进 crate 根的 `wip/` 目录——搬进 `tests/compile_fail/` 才算接受（首跑应看到这一形态）。
//! `tests/ui_pass/` 侧没有 `.stderr`（应绿样例由 `trybuild` 直接编译通过即可）。

/// 本 crate 里**造不出** `Evidence`：字段私有、无 `Default`（P5a 设计 §4.4 的「唯一产生点」）。
/// 这是那条结构事实在本块的**使用侧**照片。正控制（`tests/ui_pass/`）与它同处一次运行，
/// 把「依赖没接上」与「结构挡住了」两种红分开（评审 I-1；P5c 同形先例）。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    // 正控与失败样例**同处一次运行**：分开写成两个用例，会让「正控制没跑而失败
    // 样例照红」的形态可乘——两半合起来才算钉住（评审 I-1；P5c 同形先例）。
    t.pass("tests/ui_pass/*.rs");
}
