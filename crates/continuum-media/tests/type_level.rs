//! 媒体块（P5e）的两条**结构层照片**（设计 §12.4）与一份正控制：
//! `tests/compile_fail/*.rs`（**应编不过**，各配同名 `.stderr`）＋
//! `tests/compile_pass/*.rs`（**应编得过**，正控制）。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明
//! 「我没这么构造过」，证明不了「做不到」——`Evidence` 的字段若真是 `pub`、
//! `EvidenceSubject::Requirement` 若真能构造，一份运行期用例照样绿。故用
//! `trybuild`：样例必须**编不过**，且必须**因为预期的那条错**编不过。每份样例配一份
//! 同名 `.stderr`，由 `trybuild` **逐字比对**（只有路径与行号会被规范化，**错误码不会**）
//! ——否则「因为拼错函数名而编译失败」也会让用例变绿。
//!
//! # 本目录的样例**逐份点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）
//!
//! `tests/compile_fail/`（**三份**，应编不过）：
//!
//! 1. **`evidence_fields_are_private`** —— 在 `continuum-media` 里 `Evidence { .. }`
//!    （字段私有）编不过：**在本 crate 里造不出 `Evidence`**（字面量通道）。
//! 2. **`evidence_has_no_default`** —— `Evidence::default()`（无实现）编不过：同一条
//!    「造不出」的另一条通道（无参构造通道）。**（1、2 拆两份是实跑修出来的，不是计划
//!    原形）**：计划原写「同一条样例里两处都写」，而实跑里两条通道**互相遮蔽**——`default`
//!    那处的错误一出现，字面量那处的 E0451 就不报；这与 P5a 自己的处置（它把这两条拆成
//!    两份同名的样例，「不许并成一条」）一致。理由与日志见两份样例的文件头与 task 报告。
//! 3. **`evidence_subject_requires_a_named_requirement`** —— 在本 crate 里
//!    「指名一条 Requirement」所需的名字（语义层的 `RequirementId`）**写不出来**，
//!    故构造 `EvidenceSubject::Requirement(..)` 过不了：**归属的本块侧不可表达**。
//!    它的成立条件经过换形态（原假设「P5a 不 re-export `RequirementId`」实测已失效），
//!    逐条对账写在样例自己的文件头。
//!
//! `tests/compile_pass/`（**一份**，应编得过）：
//!
//! - **`evidence_type_is_nameable`** —— `Evidence` 这个**类型名**在 crate 外可命名。
//!   没有它，第 1 份反例的绿可以来自「本 crate 里根本写不出这个名字」这类与判据无关的
//!   坏法（恒绿的守卫比没有更坏）；反过来，只钉正控制，「字段开了 `pub`」照样绿。
//!   **本条的语料不从被测清单里取**：它说的是**名字本身**，不是任何一份反例的片段。
//!
//! # 两条坐标（改样例前必读）
//!
//! （i）**配对按文件主干**——`<stem>.rs` 与 `<stem>.stderr` 同名配对；改份名要连
//! `.stderr` 一起改名。
//! （ii）**样例里的注释也是照片坐标**——`.stderr` 记的是 `文件:行:列`，样例里
//! 增删一行注释就会让行号漂；动了样例（哪怕只改注释）就要重跑并一起提交新 `.stderr`。
//!
//! **三份 `.stderr` 一律取自实跑**（`TRYBUILD=overwrite` 生成之后逐份读，确认失败在
//! **预期的那一条**上——否则「因为拼错函数名而编译失败」也会让用例变绿）。**不许凭记忆写，
//! 不许拿一个值套两处。**
//!
//! # 「通配自动收」不再设第二份清单
//!
//! 下面驱动里的两个通配自动收走两个目录（新增样例不需要改驱动）；本文件**不另设**
//! 常量清单与「清单对目录」的对钉守卫（`continuum-budget` / `continuum-verify` 的
//! `the_sample_list_matches_the_directory` 那种）——本块样例少，上面这段点名就是
//! 第二处表示，**加样例时两处（目录与这段点名）都要同步**，由人读着守。

/// 只驱动 `trybuild` 并叙述坐标；被钉的话都由 `tests/compile_fail/` 与
/// `tests/compile_pass/` 下的样例去说。
#[test]
fn the_type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    t.pass("tests/compile_pass/*.rs");
}
