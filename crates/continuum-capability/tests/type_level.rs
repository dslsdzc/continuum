//! 类型层只读保证的唯一直接证据（设计 §2.3、§8）。
//!
//! 「`full_access` 写不出来」「字符串还原不出能力」是**不可表达性**命题：运行期
//! 测试无论怎么写都只能证明「我没这么做」，证明不了「做不到」。故用 `trybuild`
//! 的编译失败用例。每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在
//! 预期的报错上——否则「因为拼错函数名而编译失败」也会让用例变绿。

/// 名字描述的是**这组样例共同守住的东西**（类型层保证），不是其中某一条的来历：
/// 样例里既有「能力造不出来」，也有 `full_access` 写不出来、非法组合写不出来、
/// 字符串互换不可用，以及 `AuthorizedTool` 造不出来。旧名
/// `capability_cannot_be_built_outside_its_issuing_point` 只概括得动第一条，本 task
/// 加入 `AuthorizedTool` 后更不合适，故改名；通配与 `.stderr` 一律未动。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
