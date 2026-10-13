//! Task 10 的**编译期**用例：三条「crate 外写不出来」的不可表达性命题，
//! 各配一份 `tests/compile_fail/*.rs` ＋ 同名 `.stderr`。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明「我没这么构造过」，
//! 证明不了「做不到」——`Specification` 的字段若真是 `pub`，一份运行期用例照样绿。
//! 故用 `trybuild`：样例必须**编不过**，且必须**因为预期的那条错**编不过。
//! 每份样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**（只有路径与行号会被规范化，
//! **错误码不会**）——否则「因为拼错函数名而编译失败」也会让用例变绿。
//!
//! # 本目录的样例**逐份点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）
//!
//! `tests/compile_fail/`（**三份**，应编不过）：
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
//!
//! # 本 crate 为何**没有** `tests/compile_pass/`
//!
//! `continuum-canonical` 建了一份 `compile_pass/`，理由是它有一条**只有编译期能表达**
//! 的正命题（「只实现 `retrieve` 的实现编得过」⇔ 该 trait 的必需方法集恰好一个），
//! 以及给「反例的绿可能来自『这个名字整个不存在』」当正控制。**本 task 不建**，
//! 论证两条：
//!
//! 1. **逐字 `.stderr` 本身就是「因为预期的那条错」的判据**：这里三份样例的绿
//!    （= 编不过且输出与 `.stderr` 逐字相同）要求 `Specification` / `SpecDraft` /
//!    `finalize` / `Unknowns` 都真实存在且签名/可见性与判据一致——「API 整体不可用」
//!    那类坏法给出的是别的错误码与别的指向，比对即红（canonical 文件头对 `.stderr`
//!    机制的论证同）。
//! 2. **正控制在运行期用例里**：`tests/spec.rs` 整份都在正常路径上使用这几个 API
//!    （构造 `SpecDraft`、`finalize`、组合写、读回）——若 API 整体不可用，那一份
//!    全红，不存在「三份反例绿而 API 坏」的静默状态。这与 `continuum-node` /
//!    `continuum-model-registry` 的处置相同（它们的样例也钉「某条通道写不出来」，
//!    正面那半各有各的运行期落点）。
//!
//! # 「通配自动收」不等于「清单自动更新」
//!
//! 下面 `type_level_guarantees_hold` 里的 `t.compile_fail` 是**通配**，
//! 新增一份样例**不需要改那行驱动代码**；但**本文件的 `COMPILE_FAIL_SAMPLES`
//! 与上面这段点名是手写的**，加样例的人必须同时加上。这一处有守卫：常量、本文件头的
//! 点名、目录三者由 `the_sample_list_matches_the_directory` 对钉——加了样例忘了改清单，
//! 会有一条用例红，它的消息就写着要改哪里。新增样例后**要重跑本文件**来接收它生成的
//! `.stderr`（首跑落 `wip/*.stderr`，搬进 `tests/compile_fail/` 才算接受）。

// 本文件**不 `use continuum_semantics`**：它只驱动 `trybuild` 并核对清单，
// 那些「编得过 / 编不过」的话都由 `tests/compile_fail/` 下的样例去说。
// **公开 API 从 crate 外真的拿得到**那一侧的证据在 `tests/spec.rs`（它整份都是 crate 外代码）。

/// 清单里点到的、**应编不过**的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**三份**。
const COMPILE_FAIL_SAMPLES: [&str; 3] = [
    "a_draft_without_unknowns_cannot_be_finalized",
    "a_specification_cannot_be_constructed_outside_the_crate",
    "conflict_and_undefined_behaviour_are_types_only",
];

/// 通配收走 `tests/compile_fail/*.rs`——**清单的同步由下一条用例对钉**。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}

/// 清单与目录对钉，且清单还得在文件头的散文里点名。
///
/// # 它守的是哪一个坏法
///
/// 加一份样例要对齐三处：目录下的 `.rs`、同名 `.stderr`（由 `trybuild` 逐字比对，
/// 缺了会红）、**手写的清单**。前两处本来就有守卫（上面那条的**通配**收走 `.rs`，
/// `.stderr` 由 `trybuild` 管）；**只有清单原先没人守**，而实际会发生的坏法只有一个
/// 方向：**加了样例、忘了改清单**。本用例补的就是那一处。
///
/// # 它**不**查什么（免得被读成覆盖得更广）
///
/// **只查名字集合**：不解析分组、不解析错误码、不比对 `.stderr` 的内容——那些由
/// `type_level_guarantees_hold` 与各份样例自身负责。也**不查 `.stderr` 是否配齐**
/// （那是 `trybuild` 的事）。
#[test]
fn the_sample_list_matches_the_directory() {
    assert_eq!(
        listed(&COMPILE_FAIL_SAMPLES),
        samples_in("tests/compile_fail"),
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 COMPILE_FAIL_SAMPLES **和**文件头那段点名各加一行"
    );

    // 第二条：清单里的名字**还都必须在文件头的散文清单里点名**——否则「改了常量、忘了散文」
    // 一样静默。**只截到第一个 `const` 之前**：否则常量自己那几行会让本断言恒真
    // （这是本守卫自身的假绿形状，故把截断的理由写在这里）。
    //
    // **它实际钉的比上面那句话弱**：判据是 `header.contains(name)`，即**子串**——名字只要在
    // 该段散文的任何位置出现一次即满足（出现在别处的行文里也算数），它**不校验**名字落在那条
    // 点名清单的行上，也不校验那行写了什么（同 canonical 同名守卫的自述）。
    let source = include_str!("type_level.rs");
    let header = source
        .split("const COMPILE_FAIL_SAMPLES")
        .next()
        .expect("本文件里应有 COMPILE_FAIL_SAMPLES 的定义");
    for name in COMPILE_FAIL_SAMPLES {
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
