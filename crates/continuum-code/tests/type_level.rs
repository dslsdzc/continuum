//! 「`Evidence` 只能经唯一产生点 `Evidence::from_tool_result` 得到」这条**不可表达性**
//! 命题在**使用侧**（本 crate）的唯一直接证据（设计 §6.2；P5a 设计 §4.4；计划 §12.3
//! 第 4 条）。
//!
//! # 为什么在「使用侧」重拍
//!
//! P5a 已在 `continuum-verify` 自己的 `tests/compile_fail/` 里拍过同一条命题
//! （`evidence_fields_are_private` / `evidence_has_no_default`）——那两张照片证明的是
//! 「在制造方的测试环境里，crate 外拼不出 `Evidence`」。本块的产出面要**用** `Evidence`
//! （设计 §5.2 表），故本文件在**使用方的编译环境**里重拍一次：`continuum-verify` 这条
//! dev 边若没接上、或可见性在本 crate 侧不同，这一张照片会当场失效。
//!
//! # 不可表达性只有编译失败样例能钉
//!
//! 运行期用例无论怎么写，都只能证明「我没这么构造过」，证明不了「做不到」——
//! `Evidence` 的字段若真是 `pub`（或它真有了 `Default` 实现），一份运行期用例照样绿。
//! 故用 `trybuild` 的编译失败用例：`tests/compile_fail/*.rs` 必须**编不过**，且必须
//! **因为预期的那条错**编不过。每份样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**
//! （只有路径与行号会被规范化，**错误码不会**）——否则「因为拼错函数名而编译失败」
//! 也会让用例变绿，那样例钉的就是拼写，不是通道。
//!
//! # 本目录现有两份样例＋一份正控制，逐份点名
//!
//! `tests/compile_fail/` 现有**两份**、`tests/ui_pass/` 现有**一份**，下面**点名**
//! （便于对数：这段点名的项数必须等于两个目录里 `.rs` 的份数）：
//!
//! 1. **`evidence_literal_is_private`**——`Evidence` 的**全字段结构体字面量**
//!    （预期是「字段私有」那一类，常见码 `E0451`；**判据是实跑产出的那份 `.stderr`
//!    本身**，不是这个码——本项目为「凭记忆写错误码」付过代价）。钉的是 §4.4 的
//!    「唯一产生点」是**结构事实**：字面量构造被私有字段挡住，调用方不能绕过
//!    `from_tool_result` 自己拼一条证据。
//! 2. **`evidence_default_is_absent`**——`Evidence::default()`（预期是「没有这个关联
//!    函数」那一类，常见码 `E0599`）。钉的是同一条结构事实的**另一条通道**：连无参
//!    构造的捷径也不存在。**与第 1 份是两条不同的编译失败通道，不许并成一条**——
//!    两条通道的失效条件不同：字段若被开成 `pub`，第 1 份失效；别处若给 `Evidence`
//!    加了 `Default`，第 2 份失效（设计 §12.3 第 4 条把两条并作一条列；本块与 P5a
//!    同拆两份，理由同 P5a 文件头那段）。
//! 3. **`ui_pass/evidence_is_reachable`（正控制）**——只取构造点的**函数项**（不构造值、
//!    不传八个实参），证明「这个 crate 在本 crate 的编译环境里够得着」。**这条正控制的
//!    用途只有一个**：把「因为依赖没接上而编译失败」与「因为字段私有而编译失败」分开
//!    ——**没有它，两份样例的红可能是任何一种红**。实跑（`t6-03d`：临时删 dev 边后
//!    三份样例同红——两份 `compile_fail` 报 `mismatch`（实际输出是 `E0432` 找不到
//!    `continuum_verify`，与照片不符）、正控制报 `[should pass] ... error`；日志见
//!    `.tmp/t6-03d-no-devdep.log`）——正控制把「依赖没接上」这一因**单独点名**出来。
//!
//! # 红条件照实写成编译期照片
//!
//! P5a 若给出 `Default` 实现、或把 `Evidence` 的字段开成 `pub`，对应样例转为
//! **编译通过**，`trybuild` 把那一条报成 `error`，原文逐字：
//! `Expected test case to fail to compile, but it succeeded.`
//! （两处实跑各开一条通道：字段开 `pub` ⇒ 只有 `evidence_literal_is_private` 转
//! `error`；加 `Default` ⇒ 只有 `evidence_default_is_absent` 转 `error`——另一份样例
//! 与正控制都仍绿，两条通道互不牵连；日志见 `.tmp/t6-03a-mut-pub.log`、
//! `.tmp/t6-03b-mut-default.log`）。**这不是「用例变红」**（纪律 8）：本块**没有**运行期
//! 照片能拍到「绕过构造点造出了 `Evidence`」——那条通道一旦打开，唯一可拍的形态就是
//! 「样例从编译失败变成编译通过」，由 `trybuild` 的这一行报告承载。
//!
//! # 这条证不到什么（照实写）
//!
//! 它**证不到「产出只能经本块的某枚函数」**——`Evidence::from_tool_result` 是 `pub`，
//! **调用点在装配方**，本块不建第二枚包装（设计 §6.2：不建 `evidence_from_code_output`）。
//! 本块**没有**那类照片，也不该有。
//!
//! # 样例里的注释也是照片坐标
//!
//! `trybuild` 把样例源码逐行打印进 `.stderr` 的输出里，故**改一句注释就会改 `.stderr`**
//! ——注释与代码一样是这张照片的一部分，**改任一处都要重跑取新值**。两份 `.stderr`
//! **逐份取自实跑**、**各不相同**（两条通道、两份照片），不许拿一个值套两处。
//!
//! # 「通配自动收」不等于「清单自动更新」
//!
//! 下面 `evidence_cannot_be_constructed_bypassing_its_only_constructor` 里的
//! `t.compile_fail("tests/compile_fail/*.rs")` 与 `t.pass("tests/ui_pass/*.rs")` 都是
//! **通配**，新增一份样例**不需要改那几行驱动代码**；但**本文件的清单常量与上面这段
//! 点名是手写的**，加样例的人必须同时加上。**这一处有守卫**：两份清单、本文件头的
//! 点名、两个样例目录三者由 `the_sample_lists_match_the_directories` 对钉——加了样例
//! 忘了改清单，会有一条用例红，它的消息就写着要改哪里。新增样例后**要重跑本文件**来
//! 接收它生成的 `.stderr`（首跑落 `wip/*.stderr`，搬进 `tests/compile_fail/` 才算接受）；
//! **`.stderr` 一旦接受，就不再改那份 `.rs` 的注释**（`trybuild` 按文件主干配对且
//! **不规范化行号**，改文件头会推移坐标——要改就重跑并一起提交新 `.stderr`）。

/// 清单里点到的编译失败样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**现有两份**。
///
/// 它与文件头那段点名是**同一份清单的两处表示**，两处由下面
/// `the_sample_lists_match_the_directories` 对钉。加样例时**两处都要加**。
const LISTED_FAIL_SAMPLES: [&str; 2] = [
    "evidence_default_is_absent",
    "evidence_literal_is_private",
];

/// 清单里点到的正控制样例名（`tests/ui_pass/` 下 `.rs` 的文件名主干）——**现有一份**。
///
/// 它与文件头那段点名是**同一份清单的两处表示**，两处由下面
/// `the_sample_lists_match_the_directories` 对钉。加样例时**两处都要加**。
const LISTED_PASS_SAMPLES: [&str; 1] = ["evidence_is_reachable"];

/// 设计 §6.2 的「使用侧」照片：两份编译失败样例＋一份正控制，见文件头逐份点名。
#[test]
fn evidence_cannot_be_constructed_bypassing_its_only_constructor() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    // 正控制与失败样例**同处一次运行**：分开写成两个用例，会让「正控制没跑而失败
    // 样例照红」的形态可乘——两半合起来才算钉住。
    t.pass("tests/ui_pass/*.rs");
}

/// 读出目录下样例的文件名主干（去 `.rs`），排序。
///
/// 判据与 `trybuild` 的相对路径口径一致：用例的工作目录是 crate 根
/// （`cargo test` 起测试二进制时的 CWD），故 `tests/…` 两处都可直达。
fn rs_stems(dir: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("样例目录应可读（用例的工作目录是 crate 根，同 trybuild 的相对路径）")
        .map(|entry| {
            entry
                .expect("目录项应可读")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".rs"))
        .map(|name| name.trim_end_matches(".rs").to_string())
        .collect();
    names.sort();
    names
}

/// 清单与两个样例目录对钉——**加样例时此清单要同步，本用例会告诉你**。
///
/// # 它守的是哪一个坏法
///
/// 加一份样例要对齐三处：目录下的 `.rs`、同名 `.stderr`（`ui_pass` 侧没有 `.stderr`）、
/// **手写的清单**。前两处本来就有守卫（上面那条用例的**通配**收走 `.rs`，`.stderr` 由
/// `trybuild` 逐字比对，缺了会红）；**只有清单原先没人守**，而实际会发生的坏法只有
/// 一个方向：**加了样例、忘了改清单**。本用例补的就是那一处。
///
/// # 它**不**查什么（免得被读成覆盖得更广）
///
/// **只查名字集合**：不解析错误码、不比对 `.stderr` 的内容——那些由
/// `evidence_cannot_be_constructed_bypassing_its_only_constructor` 与各份样例自身负责。
/// 也**不查 `.stderr` 是否配齐**（那是 `trybuild` 的事）。
#[test]
fn the_sample_lists_match_the_directories() {
    let mut listed_fail: Vec<String> = LISTED_FAIL_SAMPLES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    listed_fail.sort();
    assert_eq!(
        listed_fail,
        rs_stems("tests/compile_fail"),
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 LISTED_FAIL_SAMPLES **和**文件头那段点名各加一行"
    );

    let mut listed_pass: Vec<String> = LISTED_PASS_SAMPLES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    listed_pass.sort();
    assert_eq!(
        listed_pass,
        rs_stems("tests/ui_pass"),
        "清单与 tests/ui_pass/*.rs 的名字集合必须相等：加了样例，就往 LISTED_PASS_SAMPLES **和**文件头那段点名各加一行"
    );

    // 第二条：清单里的名字**还必须在文件头的散文清单里点名**——否则「改了常量、忘了散文」
    // 一样静默。**只截到 `const LISTED_FAIL_SAMPLES` 之前**：否则常量自己那行会让本断言
    // 恒真（这是本守卫自身的假绿形状，故把截断的理由写在这里）。
    let source = include_str!("type_level.rs");
    let header = source
        .split("const LISTED_FAIL_SAMPLES")
        .next()
        .expect("本文件里应有 LISTED_FAIL_SAMPLES 的定义");
    for name in LISTED_FAIL_SAMPLES.iter().chain(LISTED_PASS_SAMPLES.iter()) {
        assert!(
            header.contains(name),
            "文件头那段点名里没有 `{name}`：加样例时那两处都要同步（本用例就是那个守卫）"
        );
    }
}
