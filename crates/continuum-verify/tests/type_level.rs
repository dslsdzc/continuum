//! 「初判输入装不下 Worker 的完成声明 / 没有盲判结论写不出 `PostBlindVerifierInput` /
//! `CompletionVerdict` 没有『完成』臂 / `Evidence` 的全字段字面量与 `Evidence::default()`
//! 两条路都走不通」这五条**不可表达性**命题的唯一直接证据（设计 §4.4、§7.4、§8.2、§12.3）。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明「我没这么构造过」，
//! 证明不了「做不到」——`InitialVerifierInput` 若真多一个 `worker_declaration` 字段、
//! `CompletionVerdict` 若真有 `Complete` 臂，一份运行期用例照样绿。**故本 task 的观测方式是
//! 「样例从编译失败变成编译通过」或「编译不过」，不是「用例变红」**：`.rs` 写的是
//! **被挡住的那次构造**，**必须编不过**，且必须**因为预期的那条错**编不过。故用 `trybuild`
//! 的编译失败用例：每份样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**
//! （只有路径与行号会被规范化，**错误码不会**）——否则「因为拼错函数名而编译失败」
//! 也会让用例变绿，那样例钉的就是拼写，不是通道。
//!
//! # 本目录现有五份样例，逐份点名
//!
//! 本目录现有**五份**，下面**点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）：
//!
//! 1. **`initial_verifier_input_has_no_place_for_a_worker_declaration`**——
//!    `InitialVerifierInput { contract, artifacts, evidence, worker_declaration }` 多出第四项
//!    （预期是「没有这个字段」那一类，常见码 `E0560`）。钉的是 **§7.4 第 1 条**：
//!    初判输入的字段清单里**没有那个位置**。
//! 2. **`post_blind_verifier_input_cannot_be_built_without_a_blind_verdict`**——
//!    `PostBlindVerifierInput::new(declaration)` 缺第一个参数（预期常见码 `E0061`）。
//!    钉的是 **§7.4 第 2 条 / §29**：没有盲判结论，这一类输入写不出来。
//! 3. **`completion_verdict_has_no_complete_arm`**——`CompletionVerdict::Complete { failing: .. }`
//!    （预期常见码 `E0599`）。钉的是 **§8.2**：「完成」这一臂不存在——
//!    **这是「完成不可表达」那句构造性论证的实测补件**。
//! 4. **`evidence_fields_are_private`**——`Evidence { .. }` 全字段结构体字面量
//!    （预期常见码 `E0451`）。钉的是 **§4.4** 的「唯一产生点」是结构事实。
//! 5. **`evidence_has_no_default`**——`Evidence::default()`（预期常见码 `E0599`）。
//!    钉的是 §4.4 同一条结构事实的另一条通道；**与第 4 份是两条不同的编译失败通道，
//!    不许并成一条**（两条通道的失效条件不同，见各份样例的文件头）。
//!
//! 各份的判据都是**实跑产出的那份 `.stderr` 本身**，不是上面这些「常见码」——
//! 本项目为「凭记忆写错误码」付过代价。
//!
//! # 与设计 §12.3 的条数对齐（设计列四条、本目录落五份）
//!
//! 设计 §12.3 现写「本块用**四条**」并逐条列出（`:1003-1006`），**其第 4 条把
//! `Evidence { .. }` 与 `Evidence::default()` 并作一条**；本 task 把它拆成**两份**样例
//! （上面第 4、5 份），理由是它们是**两条不同的编译失败通道**（字段私有 vs 无 `Default`
//! 实现）——两条通道的失效条件不同，合并写一份会漏掉另一条的失效。故设计列四条、
//! 本目录落五份，两者不矛盾。
//!
//! # 「通配自动收」不等于「清单自动更新」
//!
//! 下面 `type_level_guarantees_hold` 里的 `t.compile_fail("tests/compile_fail/*.rs")` 是**通配**，
//! 新增一份样例**不需要改那三行驱动代码**；但**本文件的 `LISTED_SAMPLES` 与上面这段点名
//! 是手写的**，加样例的人必须同时加上。**这一处有守卫**：`LISTED_SAMPLES`、本文件头的点名、
//! `tests/compile_fail/` 目录三者由 `the_sample_list_matches_the_directory` 对钉——
//! 加了样例忘了改清单，会有一条用例红，它的消息就写着要改哪里。
//! 新增样例后**要重跑本文件**来接收它生成的 `.stderr`（首跑落 `wip/*.stderr`，
//! 搬进 `tests/compile_fail/` 才算接受）；**`.stderr` 一旦接受，就不再改那份 `.rs` 的注释**
//! （`trybuild` 按文件主干配对且**不规范化行号**，改文件头会推移坐标——要改就重跑并一起提交新 `.stderr`）。

/// 清单里点到的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**现有五份**。
///
/// 它与文件头那段点名是**同一份清单的两处表示**，两处由下面
/// `the_sample_list_matches_the_directory` 对钉。加样例时**两处都要加**。
const LISTED_SAMPLES: [&str; 5] = [
    "completion_verdict_has_no_complete_arm",
    "evidence_fields_are_private",
    "evidence_has_no_default",
    "initial_verifier_input_has_no_place_for_a_worker_declaration",
    "post_blind_verifier_input_cannot_be_built_without_a_blind_verdict",
];

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}

/// 清单与目录对钉——**加样例时此清单要同步，本用例会告诉你**。
///
/// # 它守的是哪一个坏法
///
/// 加一份样例要对齐三处：`tests/compile_fail/` 下的 `.rs`、同名 `.stderr`、**手写的清单**。
/// 前两处本来就有守卫（上面那条的**通配**收走 `.rs`，`.stderr` 由 `trybuild` 逐字比对，
/// 缺了会红）；**只有清单原先没人守**，而实际会发生的坏法只有一个方向：
/// **加了样例、忘了改清单**。本用例补的就是那一处。
///
/// # 它**不**查什么（免得被读成覆盖得更广）
///
/// **只查名字集合**：不解析分组、不解析错误码、不比对 `.stderr` 的内容——那些由
/// `type_level_guarantees_hold` 与各份样例自身负责。也**不查 `.stderr` 是否配齐**
/// （那是 `trybuild` 的事）。
#[test]
fn the_sample_list_matches_the_directory() {
    let mut in_dir: Vec<String> = std::fs::read_dir("tests/compile_fail")
        .expect("tests/compile_fail/ 应可读（用例的工作目录是 crate 根，同 trybuild 的相对路径）")
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
    let mut listed: Vec<String> = LISTED_SAMPLES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    in_dir.sort();
    listed.sort();
    assert_eq!(
        listed, in_dir,
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 LISTED_SAMPLES **和**文件头那段点名各加一行"
    );

    // 第二条：清单里的名字**还必须在文件头的散文清单里点名**——否则「改了常量、忘了散文」
    // 一样静默。**只截到 `const LISTED_SAMPLES` 之前**：否则常量自己那行会让本断言恒真
    // （这是本守卫自身的假绿形状，故把截断的理由写在这里）。
    let source = include_str!("type_level.rs");
    let header = source
        .split("const LISTED_SAMPLES")
        .next()
        .expect("本文件里应有 LISTED_SAMPLES 的定义");
    for name in LISTED_SAMPLES {
        assert!(
            header.contains(name),
            "文件头那段点名里没有 `{name}`：加样例时那两处都要同步（本用例就是那个守卫）"
        );
    }
}
