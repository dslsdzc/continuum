//! Task 17 的**编译期**用例：crate 外「写不出来」的不可表达性命题，
//! 各配一份 `tests/compile_fail/*.rs` ＋ 同名 `.stderr`；另配
//! `tests/compile_pass/*.rs`（**应编得过**的正控制）。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明
//! 「我没这么构造过」，证明不了「做不到」——`Allocation` 的字段若真是 `pub`，
//! 一份运行期用例照样绿。故用 `trybuild`：样例必须**编不过**，且必须**因为
//! 预期的那条错**编不过。每份样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**
//! （只有路径与行号会被规范化，**错误码不会**）——否则「因为拼错函数名而编译失败」
//! 也会让用例变绿。
//!
//! # 本目录的样例**逐份点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）
//!
//! `tests/compile_fail/`（**六份**，应编不过）：
//!
//! - **`allocation_has_no_public_constructor`** —— `Allocation` 的字段私有、
//!   无公开构造函数，crate 外元组构造编不过。**逐角色各一份**（三份不合并）——
//!   三合一（或抽一个代表）时，其中一个角色漂了照样绿。判据是**失败必须落在
//!   私有性上**（形态照 P3A 对 `AuthorizedTool` 的样例）。预期错误码以实跑为准。
//! - **`reservation_has_no_public_constructor`** —— 同上；`Reservation` 的里子是
//!   `owner` ＋ `dims` 两项，结构体字面量两字段全列齐仍编不过。
//! - **`remaining_has_no_public_constructor`** —— 同上；最要紧的那半：若 `Remaining`
//!   后来开了公开构造函数而 `Allocation` 没有，一条合并的样例仍会过。
//! - **`allocation_is_not_the_other_two_roles`** —— 三角色三类型（§12.1）：
//!   `Allocation` 放进 `Reservation` 与 `Remaining` 的位置**各一条**（共两条），
//!   都是**类型不匹配**。逐角色各一份，不抽代表。
//! - **`reservation_is_not_the_other_two_roles`** —— 同上，`Reservation` 为主语。
//! - **`remaining_is_not_the_other_two_roles`** —— 同上，`Remaining` 为主语。
//!
//! 前三条与后三条合起来是 plan 点名的两个命题：
//! `each_role_has_exactly_one_constructor_and_no_second_public_path`（三反例 ＋
//! 三正控制共**六份**）与 `the_three_roles_are_three_types`（三份逐角色）。
//! **「构造」与「读」要分清**：三个角色的读法（`dims()`、逐维读者）是公开的、
//! **不构成**第二条构造路径（`dims()` 返回 `&Dimensions`，不是角色）——
//! 判据是**类型**。**并写明作用域与时点**：「除设计指定的那个入口外无第二个
//! 公开构造路径」是**在本 task 结束时**由这六份样例钉住的；「三个角色私有、
//! 无公开构造函数」这一句在 P4 落地前只是**设计写死的承诺**，本 task 是它
//! 第一次有代码层的照片。
//!
//! # `tests/compile_pass/`（**三份**，**应编得过**，各是它那份反例的正控制）
//!
//! - **`an_allocation_comes_from_allocate`** —— `Allocation` 由分配方给出
//!   （`allocate` 的签名可用、返回 `Allocation`）。**没有它**，三份反例的绿可以
//!   来自「把入口删光」这类与判据无关的坏法（恒绿的守卫比没有更坏）。
//! - **`a_reservation_comes_from_reserve`** —— `Reservation` 由 `reserve` 给出。
//! - **`a_remaining_comes_from_remaining`** —— `Remaining` 由 `remaining()` 给出。
//!
//! 六条（三反例 ＋ 三正控制）合起来才算钉住：只钉反例，「入口被删光」照样绿；
//! 只钉正控制，「crate 外随手能造一个」照样绿。
//!
//! # 「通配自动收」不等于「清单自动更新」
//!
//! 下面 `type_level_guarantees_hold` 里的 `t.compile_fail` / `t.pass` 是**通配**，
//! 新增一份样例**不需要改那两行驱动代码**；但**本文件的 `COMPILE_FAIL_SAMPLES` /
//! `COMPILE_PASS_SAMPLES` 与上面这段点名是手写的**，加样例的人必须同时加上。
//! 这一处有守卫：两个常量、本文件头的点名、两个目录三者由
//! `the_sample_lists_match_the_directories` 对钉——加了样例忘了改清单，
//! 会有一条用例红，它的消息就写着要改哪里。新增反例样例后**要重跑本文件**来接收
//! 它生成的 `.stderr`（首跑落 `wip/*.stderr`，搬进 `tests/compile_fail/` 才算接受）。

// 本文件**不 `use continuum_budget`**：它只驱动 `trybuild` 并核对清单，
// 那些「编得过 / 编不过」的话都由 `tests/compile_fail/` 与 `tests/compile_pass/`
// 下的样例去说。
// **公开 API 从 crate 外真的拿得到**那一侧的证据因此不止一处：`tests/tree.rs` /
// `tests/persist.rs`（整份都是 crate 外代码）与 `tests/compile_pass/` 的正控制。

/// 清单里点到的、**应编不过**的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**六份**。
const COMPILE_FAIL_SAMPLES: [&str; 6] = [
    "allocation_has_no_public_constructor",
    "allocation_is_not_the_other_two_roles",
    "remaining_has_no_public_constructor",
    "remaining_is_not_the_other_two_roles",
    "reservation_has_no_public_constructor",
    "reservation_is_not_the_other_two_roles",
];

/// 清单里点到的、**应编得过**的样例名（`tests/compile_pass/` 下 `.rs` 的文件名主干）——**三份**。
const COMPILE_PASS_SAMPLES: [&str; 3] = [
    "a_remaining_comes_from_remaining",
    "a_reservation_comes_from_reserve",
    "an_allocation_comes_from_allocate",
];

/// 两条通配收走两个目录——**清单的同步由下一条用例对钉**。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    t.pass("tests/compile_pass/*.rs");
}

/// 两份清单与它们各自的目录对钉，且清单还得在文件头的散文里点名。
///
/// # 它守的是哪一个坏法
///
/// 加一份样例要对齐三处：目录下的 `.rs`、同名 `.stderr`（由 `trybuild` 逐字比对，
/// 缺了会红；`compile_pass` 那侧没有 `.stderr`）、**手写的清单**。前两处本来就有
/// 守卫（上面那条的**通配**收走 `.rs`）；**只有清单原先没人守**，而实际会发生的
/// 坏法只有一个方向：**加了样例、忘了改清单**。本用例补的就是那一处。
///
/// # 它**不**查什么（免得被读成覆盖得更广）
///
/// **只查名字集合**：不解析分组、不解析错误码、不比对 `.stderr` 的内容——那些由
/// `type_level_guarantees_hold` 与各份样例自身负责。也**不查 `.stderr` 是否配齐**
/// （那是 `trybuild` 的事）。
#[test]
fn the_sample_lists_match_the_directories() {
    assert_eq!(
        listed(&COMPILE_FAIL_SAMPLES),
        samples_in("tests/compile_fail"),
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 \
         COMPILE_FAIL_SAMPLES **和**文件头那段点名各加一行"
    );
    assert_eq!(
        listed(&COMPILE_PASS_SAMPLES),
        samples_in("tests/compile_pass"),
        "清单与 tests/compile_pass/*.rs 的名字集合必须相等：加了样例，就往 \
         COMPILE_PASS_SAMPLES **和**文件头那段点名各加一行"
    );

    // 第二条：两份清单里的名字**还都必须在文件头的散文清单里点名**——否则「改了常量、
    // 忘了散文」一样静默。**只截到第一个 `const` 之前**：否则常量自己那几行会让本断言
    // 恒真（这是本守卫自身的假绿形状，故把截断的理由写在这里）。
    //
    // **它实际钉的比上面那句话弱**：判据是 `header.contains(name)`，即**子串**——名字只要在
    // 该段散文的任何位置出现一次即满足（出现在别处的行文里也算数），它**不校验**名字落在那条
    // 点名清单的行上，也不校验那行写了什么（同 canonical 同名守卫的自述）。
    let source = include_str!("type_level.rs");
    let header = source
        .split("const COMPILE_FAIL_SAMPLES")
        .next()
        .expect("本文件里应有 COMPILE_FAIL_SAMPLES 的定义");
    for name in COMPILE_FAIL_SAMPLES.iter().chain(COMPILE_PASS_SAMPLES.iter()) {
        assert!(
            header.contains(name),
            "文件头那段点名里没有 `{name}`：加样例时那两处都要同步（本用例就是那个守卫）"
        );
    }
}

/// 清单常量 → 排序后的 `Vec<String>`。
fn listed(names: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = names.iter().map(|n| (*n).to_string()).collect();
    out.sort();
    out
}

/// 目录下的 `.rs` 文件名主干，排序后。
fn samples_in(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
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
    out.sort();
    out
}
