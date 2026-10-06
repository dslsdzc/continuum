//! 「crate 外**写不出** `ComputeNode` 的字段字面量」这条**不可表达性**命题的唯一直接证据
//! （设计 §3.5、§9 的那一格）。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明「我没这么构造过」，
//! 证明不了「做不到」——`ComputeNode` 的字段若真是 `pub`，一份运行期用例照样绿。
//! 故用 `trybuild` 的编译失败用例：`tests/compile_fail/*.rs` 必须**编不过**，
//! 且必须**因为预期的那条错**编不过。每个样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**
//! （只有路径与行号会被规范化，**错误码不会**）——否则「因为拼错函数名而编译失败」
//! 也会让用例变绿，那样例钉的就是拼写，不是字段私有。
//!
//! # 本目录只有一份样例，它钉的措辞是「写不出字段字面量」
//!
//! 本目录现有**一份**，下面**点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）：
//! **`compute_node_fields_are_private`**——`ComputeNode` 的**全字段结构体字面量**
//! （预期是「字段私有」那一类，常见码 `E0451`；**这一处的判据是实跑产出的那份 `.stderr` 本身**，
//! 不是这个码——本项目为「凭记忆写错误码」付过代价）。
//!
//! **它不是「构造不出来」**：`ComputeNode::new` 是 `pub fn`，crate 外经它就能拿到一枚节点
//! （设计 §3.5 自己就这么写，§4 的注册表与 §6 的放置都要求拿得到）。
//! 本样例钉的是**另一条通道**：绕过 `new` 自己拼一个字段字面量。这条通道走不通，
//! 才使「`new` 是唯一构造入口」这句话成立；若字段可写，调用方可以自己拼一枚节点，
//! 那么「闸门读到的 `class`」与「构造时给的 `class`」之间就没有任何一处可挂不变量——
//! 而这正是 §3.5 取私有字段的**全部理由**。
//!
//! （**来历**：设计 §9 那一格的**标签**一度写「`ComputeNode` 不可外部构造」，
//! 与同格的**正文**「crate 外写不出字段字面量」及 §3.5 相抵；该标签已在 `3a7ddad` 收到正文的口径。
//! 本文件与样例文件名从一开始就按正文取——**不沿用那个已被订正的标签**。）
//!
//! # 「通配自动收」不等于「清单自动更新」
//!
//! 下面 `type_level_guarantees_hold` 里的 `t.compile_fail("tests/compile_fail/*.rs")` 是**通配**，
//! 新增一份样例**不需要改那三行驱动代码**；但**本文件的 `LISTED_SAMPLES` 与上面这段点名
//! 是手写的**，加样例的人必须同时加上。**这一处有守卫**：`LISTED_SAMPLES`、本文件头的点名、
//! `tests/compile_fail/` 目录三者由 `the_sample_list_matches_the_directory` 对钉——
//! 加了样例忘了改清单，会有一条用例红，它的消息就写着要改哪里。
//! 新增样例后**要重跑本文件**来接收它生成的 `.stderr`（首跑落 `wip/*.stderr`，
//! 搬进 `tests/compile_fail/` 才算接受）。

/// 清单里点到的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**现有一份**。
///
/// 它与文件头那段点名是**同一份清单的两处表示**，两处由下面
/// `the_sample_list_matches_the_directory` 对钉。加样例时**两处都要加**。
const LISTED_SAMPLES: [&str; 1] = ["compute_node_fields_are_private"];

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
