//! Task 10／11 的**编译期**用例：crate 外「写不出来」的不可表达性命题，
//! 各配一份 `tests/compile_fail/*.rs` ＋ 同名 `.stderr`；Task 11 起另配
//! `tests/compile_pass/*.rs`（**应编得过**的正控制）。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明「我没这么构造过」，
//! 证明不了「做不到」——`Specification` 的字段若真是 `pub`，一份运行期用例照样绿。
//! 故用 `trybuild`：样例必须**编不过**，且必须**因为预期的那条错**编不过。
//! 每份样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**（只有路径与行号会被规范化，
//! **错误码不会**）——否则「因为拼错函数名而编译失败」也会让用例变绿。
//!
//! # 本目录的样例**逐份点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）
//!
//! `tests/compile_fail/`（**六份**，应编不过）：
//!
//! - **`a_specification_cannot_be_constructed_outside_the_crate`** —— `Specification` 的
//!   六个字段私有、无公开构造函数，故 crate 外拼不出结构体字面量（哪怕六个字段全列齐）。
//!   它钉 §227 两句 MUST 的类型落点里「唯一构造入口是 `SpecDraft::finalize`」那一半；
//!   预期是 E0451 那一类（判据是实跑产出的那份 `.stderr` 本身，不是某个码）。
//!   **本样例不声称「crate 外拿不到 `Specification`」**：经 `load_specification`
//!   从库里读得到；被挡住的只是「绕过 `finalize` 自己拼一个」。
//! - **`a_draft_without_unknowns_cannot_be_finalized`** —— `finalize` 的入参是必填的
//!   `Unknowns`：少给它编不过（E0061 那一类）、把它传成 `None` 也编不过（E0308 那一类）。
//!   两个面在同一份里，因为它们钉的是**同一个签名**（不能省、也不能是 `Option`）。
//!   它钉「`SpecDraft` 里没有 `unknowns` 字段」这句话的可判形态：`unknowns` 只在
//!   `finalize` 的入参上出现——「没查」不可表达（设计 §6.1）。
//! - **`conflict_and_undefined_behaviour_are_types_only`** —— `Specification` 上没有
//!   `detect_conflicts` / `find_undefined_behaviour`（E0599 那一类，逐名比对）。
//!   它钉的**只是那两个名字**，不是「没有任何别的检测函数」那句更宽的话——函数名是
//!   **开放集**，`trybuild` 枚举不完；限度之外的那一半由 Task 21 的 `src/` grep 复核。
//!   本文件的前置两行（`SpecDraft` → `finalize`）编得过：若那条正路走不通，`.stderr`
//!   会出现别处的错误、逐字比对即红。
//! - **`verification_requirement_has_no_is_satisfied`** —— §7.2 的**结构性禁读**：
//!   `VerificationRequirement` 没有 `is_satisfied`（该字段只存不判——OPEN-001 未决期间
//!   补任何满足判据都是发明判据，《工程》§2.5）。预期 E0599 那一类，逐字比对。
//!   （Task 11 追加。）
//! - **`verification_requirement_has_no_check`** —— 同上，`check(..)` 取不到。
//!   逐枚列出而不是只钉一个名字：方法名是开放集，钉一枚漏其余。（Task 11 追加。）
//! - **`verification_requirement_has_no_evaluate`** —— 同上，`evaluate(..)` 取不到。
//!   三份合起来是「公开方法集只有 `as_str` / `from_str`」的可判形态。
//!   （Task 11 追加。）
//!
//! `tests/compile_pass/`（**一份**，**应编得过**，是上面那三份反例的正控制）：
//!
//! - **`verification_requirement_as_str_is_usable`** —— `VerificationRequirement::from_str` /
//!   `as_str` 编得过：§7.2 的禁读是「没有判定方法」，**不是**「没有公开面」。
//!   **没有它**，三份 `verification_requirement_has_no_*` 的绿可以来自「把公开面删光」
//!   这类与判据无关的坏法（恒绿的守卫比没有更坏）。（Task 11 追加。）
//!
//! # 本 crate 为何在 Task 11 起**有** `tests/compile_pass/`
//!
//! Task 10 建本文件时**没有**建 `compile_pass/`，当时的论证有两条（留档）：
//! 逐字 `.stderr` 本身就是「因为预期的那条错」的判据（API 整体不可用会给出别的
//! 错误码与别的指向，比对即红）；且正控制在运行期用例里（`tests/spec.rs` 整份都在
//! 正常路径上使用那几个 API，API 整体不可用则它全红）。**那两条对 Task 10 的三份
//! 仍然成立**，Task 11 仍补建它的理由是一条两条论证都盖不住的坏法：
//! **反例只钉「那三个名字不存在」，没有东西钉「`as_str` / `from_str` 存在」**——
//! 若实现者把公开面删光（例如把 `as_str` 也拿掉），三份反例照样绿，
//! 而 §7.2 要的是**两个方法可用、判定方法不可有**（「只存不判」不是「不存」）。
//! 正控制按任务简报落地（「正控制（`vr.as_str()` 编译通过）」），
//! 机制与 `continuum-canonical/tests/type_level.rs` 的同名形状对齐
//! （那边的先例：`resolution_confidence_meets_is_usable` 与
//! `the_risk_and_binding_surface_is_usable` 都是给反例兜底的正控制）。
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

// 本文件**不 `use continuum_semantics`**：它只驱动 `trybuild` 并核对清单，
// 那些「编得过 / 编不过」的话都由 `tests/compile_fail/` 与 `tests/compile_pass/`
// 下的样例去说。
// **公开 API 从 crate 外真的拿得到**那一侧的证据因此不止一处：`tests/spec.rs` /
// `tests/contract.rs`（整份都是 crate 外代码）与 `tests/compile_pass/` 的正控制。

/// 清单里点到的、**应编不过**的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**六份**。
const COMPILE_FAIL_SAMPLES: [&str; 6] = [
    "a_draft_without_unknowns_cannot_be_finalized",
    "a_specification_cannot_be_constructed_outside_the_crate",
    "conflict_and_undefined_behaviour_are_types_only",
    "verification_requirement_has_no_check",
    "verification_requirement_has_no_evaluate",
    "verification_requirement_has_no_is_satisfied",
];

/// 清单里点到的、**应编得过**的样例名（`tests/compile_pass/` 下 `.rs` 的文件名主干）——**一份**。
const COMPILE_PASS_SAMPLES: [&str; 1] = ["verification_requirement_as_str_is_usable"];

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
/// 缺了会红；`compile_pass` 那侧没有 `.stderr`）、**手写的清单**。前两处本来就有守卫
/// （上面那条的**通配**收走 `.rs`）；**只有清单原先没人守**，而实际会发生的坏法只有
/// 一个方向：**加了样例、忘了改清单**。本用例补的就是那一处。
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
